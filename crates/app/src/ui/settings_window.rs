//! The settings window: everything MelCat can be told, on one page, like the
//! macOS app's `SettingsView`.
//!
//! CONTRACT (owned by the settings-window agent). A plain Win32 window
//! (class `MelCat.Settings`, title "MelCat Settings", caption + close box,
//! not resizable) built from standard controls with common controls v6
//! (the manifest asks for them; `app::run` calls `InitCommonControlsEx`),
//! using the system message font (`SystemParametersInfoForDpi` /
//! `NONCLIENTMETRICSW.lfMessageFont`) at the window's DPI. Per-monitor DPI v2:
//! lay everything out in logical units times `GetDpiForWindow / 96`, and on
//! `WM_DPICHANGED` recreate the font, re-layout and move to the suggested rect.
//! After every re-layout (a DPI change, or a new message font or text size)
//! the window and all its controls are repainted, so no label keeps pixels
//! drawn in the old font.
//!
//! Layout, top to bottom (English labels, as on macOS):
//!
//! ```text
//! Mode            (•) On Schedule   ( ) Always On   ( ) Off
//!                 <footer: "Mel keeps your computer active on the days and hours below." /
//!                  "Mel keeps your computer active at any time." / "Mel leaves the pointer alone.">
//! Schedule
//!   Days          [Sun] [Mon] [Tue] [Wed] [Thu] [Fri] [Sat]   check boxes, ordered from ui::text::first_weekday()
//!   From          [09:00 ▴▾]       DateTimePicker, DTS_TIMEFORMAT | DTS_UPDOWN, ui::text::short_time_format
//!   To            [18:00 ▴▾]
//!   <hint>        "Pick at least one day." / "Start and end are the same: all day on the selected days." /
//!                 "Ends the next morning at 06:00." (ui::text::locale_time_of_day, the pickers' format;
//!                 one final period even after "6:00 a.m.") / nothing
//! Movement
//!   Distance      [Gentle · up to 5 pt ▾]    combo box (CBS_DROPDOWNLIST), ui::text::intensity_title
//!   Move after idle [1 minute ▾]             combo box: INTERVAL_CHOICES plus the current value if custom
//!   Try the movement now [Preview]
//!   "Keep your hand off the mouse for a few seconds."      (under the label, as on macOS)
//!   <footer: "Mel moves the pointer after about this long without keyboard, mouse or touch input
//!            (a little sooner, at random), then again each time it passes, and lets go when you use
//!            the mouse or keyboard. Pick a time shorter than your screen saver and screen-off delays.">
//! General
//!   [x] Launch at login                      ui::autostart
//!                                                                            [Close]
//! ```
//!
//! The schedule controls are disabled unless the mode is On Schedule.
//!
//! Every change is applied at once: the window updates the shared `Settings`
//! and posts `messages::WM_APP_SETTINGS_CHANGED` to `notify`; the controller
//! saves them (and reports a failure). The window never saves by itself. Preview posts
//! `messages::WM_APP_PREVIEW`. Launch at login calls `ui::autostart::set_enabled`
//! directly and shows its error sentence in a message box owned by the window.
//! Closing (close box, Close button, Escape) destroys the window and posts
//! `messages::WM_APP_SETTINGS_CLOSED`. The window never steals focus except
//! when [`SettingsWindow::show`] is called (it then comes to the foreground,
//! centered on the monitor under the pointer, or is just activated when
//! already open). Escape and Enter work through `IsDialogMessageW`, which the
//! message loop calls for [`SettingsWindow::hwnd`].
//!
//! While open, the window re-reads the shared settings when the controller
//! calls [`SettingsWindow::refresh`] (after the tray menu changed something),
//! so both always agree.
//!
//! Implementation notes:
//! - `page` names the parts and holds the words and derived values; `layout`
//!   places the parts. Both are pure and unit tested.
//! - The window procedure never holds a `RefCell` borrow (of its state or of
//!   the settings) while it talks to a control or opens a message box:
//!   controls answer with synchronous notifications (`WM_CTLCOLORSTATIC`,
//!   `WM_NOTIFY`) that re-enter it.
//! - Text is measured with the real font, so the page fits the user's message
//!   font and text size; the window is as big as its content, and the room
//!   for the mode footer and the hint is reserved, so it never changes size
//!   while open.
//! - Enter presses Close (`IDOK`, the default button) unless Preview has the
//!   focus; Escape sends `IDCANCEL`. Both close the window: every change is
//!   already applied, so there is nothing to confirm or cancel.
//! - Launch at login is re-read when the window is activated, since the tray
//!   menu can switch it while the window is in the background.

mod layout;
mod page;

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use melcat_core::{describe_interval, Intensity, JiggleMode, TimeOfDay, Weekday};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SYSTEMTIME, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateFontIndirectW, DeleteObject, DrawTextW, GetDC, GetMonitorInfoW, GetSysColor, GetSysColorBrush,
    InvalidateRect, MonitorFromPoint, RedrawWindow, ReleaseDC, SelectObject, SetBkColor, SetTextColor, COLOR_GRAYTEXT,
    COLOR_WINDOW, COLOR_WINDOWTEXT, DT_CALCRECT, DT_NOPREFIX, DT_SINGLELINE, DT_WORDBREAK, FW_SEMIBOLD, HBRUSH, HDC,
    HFONT, LOGFONTW, MONITORINFO, MONITOR_DEFAULTTONEAREST, RDW_ALLCHILDREN, RDW_ERASE, RDW_INVALIDATE,
};
use windows::Win32::System::SystemServices::{SS_CENTERIMAGE, SS_LEFT, SS_NOPREFIX};
use windows::Win32::UI::Controls::{
    BST_CHECKED, BST_UNCHECKED, DATETIMEPICK_CLASSW, DTM_SETFORMATW, DTM_SETSYSTEMTIME, DTN_DATETIMECHANGE,
    DTS_TIMEFORMAT, DTS_UPDOWN, GDT_VALID, NMDATETIMECHANGE, NMHDR, WC_BUTTONW, WC_COMBOBOXW, WC_STATICW,
};
use windows::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow, SystemParametersInfoForDpi};
use windows::Win32::UI::Input::KeyboardAndMouse::{EnableWindow, GetFocus, IsWindowEnabled, SetFocus};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetCursorPos, GetDlgCtrlID, GetWindowLongPtrW, GetWindowRect,
    IsChild, IsIconic, MessageBoxW, PostMessageW, SendMessageW, SetForegroundWindow, SetWindowLongPtrW, SetWindowPos,
    SetWindowTextW, ShowWindow, BM_GETCHECK, BM_SETCHECK, BN_CLICKED, BS_AUTOCHECKBOX, BS_AUTORADIOBUTTON,
    BS_DEFPUSHBUTTON, BS_PUSHBUTTON, CBN_CLOSEUP, CBN_SELCHANGE, CBS_DROPDOWNLIST, CB_ADDSTRING, CB_GETCURSEL,
    CB_RESETCONTENT, CB_SETCURSEL, DC_HASDEFID, DM_GETDEFID, GWL_STYLE, HMENU, IDCANCEL, MB_ICONWARNING, MB_OK,
    NONCLIENTMETRICSW, SPI_GETNONCLIENTMETRICS, SPI_SETNONCLIENTMETRICS, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER,
    SW_RESTORE, SW_SHOW, WA_INACTIVE, WINDOW_EX_STYLE, WINDOW_STYLE, WM_ACTIVATE, WM_COMMAND, WM_CTLCOLORSTATIC,
    WM_DESTROY, WM_DPICHANGED, WM_NCDESTROY, WM_NEXTDLGCTL, WM_NOTIFY, WM_SETFOCUS, WM_SETFONT, WM_SETTINGCHANGE,
    WS_CAPTION, WS_CHILD, WS_CLIPCHILDREN, WS_GROUP, WS_SYSMENU, WS_TABSTOP, WS_VISIBLE, WS_VSCROLL,
};

use self::layout::{Face, Layout, Texts};
use self::page::{Kind, Part};
use crate::geom::RectPx;
use crate::messages::{WM_APP_PREVIEW, WM_APP_SETTINGS_CHANGED, WM_APP_SETTINGS_CLOSED};
use crate::settings::Settings;
use crate::ui::{autostart, text, window};
use crate::util::wide;

const CLASS: PCWSTR = w!("MelCat.Settings");
const TITLE: PCWSTR = w!("MelCat Settings");
/// Caption and close box; no resizing, minimizing or maximizing.
const STYLE: WINDOW_STYLE = WINDOW_STYLE(WS_CAPTION.0 | WS_SYSMENU.0 | WS_CLIPCHILDREN.0);
const EX_STYLE: WINDOW_EX_STYLE = WINDOW_EX_STYLE(0);
/// Logical height of a combo box's drop-down list, below the box itself.
const DROPDOWN_LIST: i32 = 200;

thread_local! {
    /// The open window, so a second `show` brings it forward instead of opening another.
    static OPEN: RefCell<Weak<RefCell<State>>> = const { RefCell::new(Weak::new()) };
}

/// The open settings window, if any. Owned by the controller.
pub(crate) struct SettingsWindow {
    hwnd: HWND,
    /// Shared with the window procedure; tells whether the window still exists.
    state: Rc<RefCell<State>>,
}

impl SettingsWindow {
    /// Opens the window (or brings the existing one forward) showing
    /// `settings`. Changes are written back into `settings` and announced to
    /// `notify` (the `MelCat.Msg` window) as described in the module docs.
    pub(crate) fn show(settings: Rc<RefCell<Settings>>, notify: HWND) -> windows::core::Result<SettingsWindow> {
        if let Some(open) = Self::open_window() {
            open.activate();
            return Ok(open);
        }
        window::register_class(CLASS, Some(settings_proc), Some(window_brush()))?;

        // Created hidden on the monitor under the pointer, so it gets that
        // monitor's DPI from the start; sized and centered once laid out.
        let work = work_area_under_pointer();
        // SAFETY: standard window creation; the class was registered above.
        let hwnd = unsafe {
            CreateWindowExW(
                EX_STYLE,
                CLASS,
                TITLE,
                STYLE,
                work.left,
                work.top,
                1,
                1,
                None,
                None,
                Some(window::instance()),
                None,
            )
        }?;
        let days = page::day_order(text::first_weekday());
        let time_format = wide(&page::picker_format(&text::short_time_format()));
        let controls = match create_controls(hwnd, &days, &time_format) {
            Ok(controls) => controls,
            Err(error) => {
                crate::log_error!("could not create the settings window: {error}");
                // SAFETY: destroys the half-built window we just created (no state is attached yet).
                unsafe {
                    let _ = DestroyWindow(hwnd);
                }
                return Err(error);
            }
        };
        let state = Rc::new(RefCell::new(State {
            hwnd,
            notify,
            settings,
            open: true,
            dpi: window_dpi(hwnd),
            fonts: None,
            controls,
            days,
            intervals: Vec::new(),
            schedule_enabled: true,
            syncing: false,
            focus: None,
            _time_format: time_format,
        }));
        window::attach(hwnd, &state);
        OPEN.with(|open| *open.borrow_mut() = Rc::downgrade(&state));

        sync_all(&state);
        let (width, height) = relayout(&state, window_dpi(hwnd)).unwrap_or((480, 560));
        let (x, y) = layout::centered(work, width, height);
        // SAFETY: sizes and places our own, still hidden, window.
        unsafe {
            let _ = SetWindowPos(hwnd, None, x, y, width, height, SWP_NOZORDER | SWP_NOACTIVATE);
        }
        let window = SettingsWindow { hwnd, state };
        window.activate();
        crate::log_info!("settings window opened");
        Ok(window)
    }

    /// Brings an open window to the foreground.
    pub(crate) fn activate(&self) {
        let Some(hwnd) = self.live() else { return };
        // SAFETY: plain show and activation calls on a window this thread owns.
        unsafe {
            let command = if IsIconic(hwnd).as_bool() { SW_RESTORE } else { SW_SHOW };
            let _ = ShowWindow(hwnd, command);
            let _ = SetForegroundWindow(hwnd);
        }
    }

    /// Re-reads the shared settings into the controls (after the tray menu changed them).
    pub(crate) fn refresh(&self) {
        if self.live().is_some() {
            sync_all(&self.state);
        }
    }

    /// The window handle, for `IsDialogMessageW` in the message loop.
    pub(crate) fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// Closes the window if it is still open (on quit).
    pub(crate) fn close(self) {
        if let Some(hwnd) = self.live() {
            // SAFETY: destroys the window this thread created; WM_NCDESTROY releases its state.
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
        }
    }

    /// The window handle while the window exists. The controller can hold a
    /// `SettingsWindow` for a moment after the user closed it (until it
    /// handles `WM_APP_SETTINGS_CLOSED`); it then does nothing.
    fn live(&self) -> Option<HWND> {
        self.state.try_borrow().ok().filter(|st| st.open).map(|_| self.hwnd)
    }

    fn open_window() -> Option<SettingsWindow> {
        let state = OPEN.with(|open| open.borrow().upgrade())?;
        let hwnd = state.try_borrow().ok().filter(|st| st.open).map(|st| st.hwnd)?;
        Some(SettingsWindow { hwnd, state })
    }
}

/// Per-window state, attached to the window with `ui::window::attach`.
struct State {
    hwnd: HWND,
    notify: HWND,
    settings: Rc<RefCell<Settings>>,
    /// False once the window is destroyed.
    open: bool,
    dpi: u32,
    /// The fonts the controls use; replaced on a DPI or font change, dropped at `WM_NCDESTROY`.
    fonts: Option<Fonts>,
    controls: Controls,
    /// The weekday of each day check box, left to right.
    days: [Weekday; 7],
    /// The idle intervals in the drop-down list, in order.
    intervals: Vec<f64>,
    /// Whether the schedule rows are enabled (the mode is On Schedule).
    schedule_enabled: bool,
    /// True while the controls are being filled from the settings, so their
    /// notifications are not taken for the user's changes.
    syncing: bool,
    /// The control that had the focus when the window was last deactivated.
    focus: Option<HWND>,
    /// The pickers' format string (`DTM_SETFORMATW`), kept for as long as the
    /// pickers in case they keep the pointer. Never read.
    _time_format: Vec<u16>,
}

/// The window handle of every part.
#[derive(Clone, Default)]
struct Controls(Vec<(Part, HWND)>);

impl Controls {
    fn get(&self, part: Part) -> HWND {
        self.0.iter().find(|(p, _)| *p == part).map(|&(_, hwnd)| hwnd).unwrap_or_default()
    }

    fn contains(&self, hwnd: HWND) -> bool {
        !hwnd.is_invalid() && self.0.iter().any(|&(_, control)| control == hwnd)
    }

    fn iter(&self) -> impl Iterator<Item = (Part, HWND)> + '_ {
        self.0.iter().copied()
    }

    /// The mode radio button that is checked (the first one if none is).
    fn checked_mode(&self) -> HWND {
        (0..JiggleMode::MENU_ORDER.len())
            .map(|index| self.get(Part::Mode(index)))
            .find(|&radio| is_checked(radio))
            .unwrap_or_else(|| self.get(Part::Mode(0)))
    }
}

/// The page's two fonts at one DPI.
struct Fonts {
    body: HFONT,
    header: HFONT,
}

impl Fonts {
    /// The system message font at `dpi`, and its semibold variant.
    fn new(dpi: u32) -> Fonts {
        let mut metrics =
            NONCLIENTMETRICSW { cbSize: std::mem::size_of::<NONCLIENTMETRICSW>() as u32, ..Default::default() };
        // SAFETY: the buffer is a NONCLIENTMETRICSW with cbSize set, as the call requires.
        let found = unsafe {
            SystemParametersInfoForDpi(
                SPI_GETNONCLIENTMETRICS.0,
                metrics.cbSize,
                Some(&mut metrics as *mut NONCLIENTMETRICSW as *mut _),
                0,
                dpi,
            )
        }
        .is_ok();
        let mut logfont = if found && metrics.lfMessageFont.lfFaceName[0] != 0 {
            metrics.lfMessageFont
        } else {
            crate::log_warn!("could not read the message font; using Segoe UI 9 pt");
            LOGFONTW {
                lfHeight: -((9 * dpi as i32 + 36) / 72),
                lfWeight: 400,
                lfFaceName: text::fixed_wide::<32>("Segoe UI"),
                ..Default::default()
            }
        };
        // SAFETY: the LOGFONTW is fully initialized.
        let body = unsafe { CreateFontIndirectW(&logfont) };
        logfont.lfWeight = logfont.lfWeight.max(FW_SEMIBOLD.0 as i32);
        // SAFETY: as above.
        let header = unsafe { CreateFontIndirectW(&logfont) };
        if body.is_invalid() || header.is_invalid() {
            crate::log_warn!("CreateFontIndirectW failed; the settings window uses the system font");
        }
        Fonts { body, header }
    }

    fn get(&self, face: Face) -> HFONT {
        match face {
            Face::Body => self.body,
            Face::Header => self.header,
        }
    }
}

impl Drop for Fonts {
    fn drop(&mut self) {
        for font in [self.body, self.header] {
            if !font.is_invalid() {
                // SAFETY: we created the font, and no control uses it any more: the
                // controls are switched to new fonts first, or already destroyed.
                unsafe {
                    let _ = DeleteObject(font.into());
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Window procedure

extern "system" fn settings_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if message == WM_NCDESTROY {
        if let Some(state) = window::state::<State>(hwnd) {
            closed(&state);
        }
        window::detach::<State>(hwnd);
    } else if let Some(state) = window::state::<State>(hwnd) {
        if let Some(result) = handle(&state, hwnd, message, wparam, lparam) {
            return result;
        }
    }
    // SAFETY: default handling for everything else.
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

fn handle(state: &Rc<RefCell<State>>, hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    match message {
        WM_COMMAND => on_command(state, loword(wparam.0), hiword(wparam.0)),
        WM_NOTIFY => on_notify(state, lparam),
        WM_CTLCOLORSTATIC => return Some(static_colors(state, HDC(wparam.0 as *mut _), HWND(lparam.0 as *mut _))),
        DM_GETDEFID => return Some(default_button(state)),
        WM_NEXTDLGCTL => {
            // IsDialogMessageW asks for this when an arrow key moves along the
            // day check boxes; DefWindowProcW would ignore it.
            let target = HWND(wparam.0 as *mut _);
            // SAFETY: focus moves only to one of our own children.
            unsafe {
                if loword(lparam.0 as usize) != 0 && IsChild(hwnd, target).as_bool() {
                    let _ = SetFocus(Some(target));
                }
            }
        }
        WM_ACTIVATE => {
            on_activate(state, wparam);
            // DefWindowProcW then gives the window the focus; WM_SETFOCUS passes it on.
            return None;
        }
        WM_SETFOCUS => restore_focus(state),
        WM_DPICHANGED => on_dpi_changed(state, hwnd, wparam, lparam),
        WM_SETTINGCHANGE if wparam.0 == SPI_SETNONCLIENTMETRICS.0 as usize => {
            // The message font or text size changed: same DPI, new fonts.
            let dpi = state.try_borrow().map(|st| st.dpi).unwrap_or(96);
            resize(state, hwnd, dpi, None);
            return None;
        }
        WM_DESTROY => {
            if let Ok(notify) = state.try_borrow().map(|st| st.notify) {
                post(notify, WM_APP_SETTINGS_CLOSED);
            }
            crate::log_info!("settings window closed");
        }
        _ => return None,
    }
    Some(LRESULT(0))
}

fn loword(value: usize) -> u16 {
    (value & 0xFFFF) as u16
}

fn hiword(value: usize) -> u32 {
    ((value >> 16) & 0xFFFF) as u32
}

/// `WM_NCDESTROY`: the controls are gone; release the fonts and forget the window.
fn closed(state: &Rc<RefCell<State>>) {
    if let Ok(mut st) = state.try_borrow_mut() {
        st.open = false;
        st.fonts = None;
        st.focus = None;
    }
    OPEN.with(|open| {
        if let Ok(mut open) = open.try_borrow_mut() {
            if open.ptr_eq(&Rc::downgrade(state)) {
                *open = Weak::new();
            }
        }
    });
}

fn on_command(state: &Rc<RefCell<State>>, id: u16, code: u32) {
    let Ok((controls, days, intervals, notify, syncing)) =
        state.try_borrow().map(|st| (st.controls.clone(), st.days, st.intervals.clone(), st.notify, st.syncing))
    else {
        return;
    };
    if syncing {
        return;
    }
    if id == IDCANCEL.0 as u16 {
        close_window(state);
        return;
    }
    match (Part::from_id(id), code) {
        (Some(Part::Mode(index)), BN_CLICKED) => {
            if let Some(&mode) = JiggleMode::MENU_ORDER.get(index) {
                edit(state, |settings| settings.mode = mode);
            }
        }
        (Some(Part::Day(index)), BN_CLICKED) => {
            if let Some(&day) = days.get(index) {
                let on = is_checked(controls.get(Part::Day(index)));
                edit(state, |settings| {
                    if on {
                        settings.schedule.days.insert(day);
                    } else {
                        settings.schedule.days.remove(day);
                    }
                });
            }
        }
        // A list closed with Escape goes back to its previous entry, so the
        // selection is read again when the list closes.
        (Some(Part::Distance), CBN_SELCHANGE | CBN_CLOSEUP) => {
            if let Some(&intensity) = combo_selection(controls.get(Part::Distance)).and_then(|i| Intensity::ALL.get(i))
            {
                edit(state, |settings| settings.intensity = intensity);
            }
        }
        (Some(Part::Interval), CBN_SELCHANGE | CBN_CLOSEUP) => {
            if let Some(&interval) = combo_selection(controls.get(Part::Interval)).and_then(|i| intervals.get(i)) {
                edit(state, |settings| settings.idle_interval = interval);
            }
        }
        (Some(Part::Preview), BN_CLICKED) => post(notify, WM_APP_PREVIEW),
        (Some(Part::Launch), BN_CLICKED) => set_launch_at_login(state, is_checked(controls.get(Part::Launch))),
        (Some(Part::Close), BN_CLICKED) => close_window(state),
        _ => {}
    }
}

fn on_notify(state: &Rc<RefCell<State>>, lparam: LPARAM) {
    if lparam.0 == 0 {
        return;
    }
    // SAFETY: WM_NOTIFY's lParam points to the sender's NMHDR for the duration of the call.
    let header = unsafe { &*(lparam.0 as *const NMHDR) };
    if header.code != DTN_DATETIMECHANGE {
        return;
    }
    let Ok((from, to, syncing)) =
        state.try_borrow().map(|st| (st.controls.get(Part::From), st.controls.get(Part::To), st.syncing))
    else {
        return;
    };
    if syncing || header.hwndFrom.is_invalid() || (header.hwndFrom != from && header.hwndFrom != to) {
        return;
    }
    // SAFETY: the sender is one of our date and time pickers, whose
    // DTN_DATETIMECHANGE carries an NMDATETIMECHANGE.
    let change = unsafe { &*(lparam.0 as *const NMDATETIMECHANGE) };
    if change.dwFlags != GDT_VALID {
        return;
    }
    let time = page::from_system_time(&change.st);
    if header.hwndFrom == from {
        edit(state, |settings| settings.schedule.start = time);
    } else {
        edit(state, |settings| settings.schedule.end = time);
    }
}

/// Labels and notes sit on the window color; notes, and the schedule labels
/// while the schedule is off, are gray.
fn static_colors(state: &Rc<RefCell<State>>, hdc: HDC, control: HWND) -> LRESULT {
    // SAFETY: GetDlgCtrlID accepts any handle.
    let part = u16::try_from(unsafe { GetDlgCtrlID(control) }).ok().and_then(Part::from_id);
    let schedule_enabled = state.try_borrow().map_or(true, |st| st.schedule_enabled);
    let gray = match part {
        Some(part) if part.kind() == Kind::Note => true,
        Some(part) if part.kind() == Kind::Label && part.is_schedule() => !schedule_enabled,
        _ => false,
    };
    // SAFETY: sets colors on the DC the control is painting with; system color brushes are never deleted.
    unsafe {
        SetTextColor(hdc, COLORREF(GetSysColor(if gray { COLOR_GRAYTEXT } else { COLOR_WINDOWTEXT })));
        SetBkColor(hdc, COLORREF(GetSysColor(COLOR_WINDOW)));
        LRESULT(GetSysColorBrush(COLOR_WINDOW).0 as isize)
    }
}

/// `DM_GETDEFID`, asked by `IsDialogMessageW` for Enter: Preview when it has
/// the focus, Close otherwise. (A dialog procedure would track this by
/// itself; a plain window answers.)
fn default_button(state: &Rc<RefCell<State>>) -> LRESULT {
    let preview = state.try_borrow().map(|st| st.controls.get(Part::Preview)).unwrap_or_default();
    // SAFETY: plain focus query on this thread.
    let focus = unsafe { GetFocus() };
    let id = if !preview.is_invalid() && focus == preview { Part::Preview.id() } else { Part::Close.id() };
    LRESULT(((DC_HASDEFID as isize) << 16) | id as isize)
}

fn on_activate(state: &Rc<RefCell<State>>, wparam: WPARAM) {
    if u32::from(loword(wparam.0)) == WA_INACTIVE {
        // SAFETY: plain focus query on this thread.
        let focus = unsafe { GetFocus() };
        if let Ok(mut st) = state.try_borrow_mut() {
            if st.controls.contains(focus) {
                st.focus = Some(focus);
            }
        }
    } else if let Ok(launch) = state.try_borrow().map(|st| st.controls.get(Part::Launch)) {
        set_check(launch, autostart::is_enabled());
    }
}

/// Gives the focus back to the control that had it, or to the checked mode
/// button. A mode button that is no longer checked is skipped: focusing it
/// with the keyboard would check it, and change the mode.
fn restore_focus(state: &Rc<RefCell<State>>) {
    let Ok((saved, controls)) = state.try_borrow().map(|st| (st.focus, st.controls.clone())) else { return };
    let checked = controls.checked_mode();
    let is_mode = |hwnd: HWND| (0..JiggleMode::MENU_ORDER.len()).any(|index| controls.get(Part::Mode(index)) == hwnd);
    let target = saved
        .filter(|&hwnd| controls.contains(hwnd))
        // SAFETY: plain query of our own control.
        .filter(|&hwnd| unsafe { IsWindowEnabled(hwnd) }.as_bool())
        .filter(|&hwnd| !is_mode(hwnd) || hwnd == checked)
        .unwrap_or(checked);
    // SAFETY: moves the focus within our own, active window.
    unsafe {
        let _ = SetFocus(Some(target));
    }
}

fn on_dpi_changed(state: &Rc<RefCell<State>>, hwnd: HWND, wparam: WPARAM, lparam: LPARAM) {
    let dpi = match u32::from(loword(wparam.0)) {
        0 => 96,
        dpi => dpi,
    };
    let origin = (lparam.0 != 0).then(|| {
        // SAFETY: for WM_DPICHANGED, lParam points to the suggested window rectangle.
        let suggested = unsafe { *(lparam.0 as *const RECT) };
        (suggested.left, suggested.top)
    });
    resize(state, hwnd, dpi, origin);
}

/// Lays the page out again at `dpi` and resizes the window to fit, its
/// top-left corner at `origin` (or where it is).
///
/// On a DPI change Windows suggests a rectangle scaled from the old one; the
/// window keeps its corner but takes the size its content needs at the new
/// DPI, which can differ by a few pixels because fonts do not scale linearly.
fn resize(state: &Rc<RefCell<State>>, hwnd: HWND, dpi: u32, origin: Option<(i32, i32)>) {
    let Some((width, height)) = relayout(state, dpi) else { return };
    let (x, y) = origin.unwrap_or_else(|| {
        let mut rect = RECT::default();
        // SAFETY: writes into a local RECT.
        let _ = unsafe { GetWindowRect(hwnd, &mut rect) };
        (rect.left, rect.top)
    });
    // SAFETY: moves and sizes our own window.
    unsafe {
        let _ = SetWindowPos(hwnd, None, x, y, width, height, SWP_NOZORDER | SWP_NOACTIVATE);
    }
}

// ---------------------------------------------------------------------------
// Changes

/// Applies a change made in the window to the shared settings and tells the
/// controller, if anything changed. Then updates what depends on it.
fn edit(state: &Rc<RefCell<State>>, change: impl FnOnce(&mut Settings)) {
    let Ok((settings, notify)) = state.try_borrow().map(|st| (Rc::clone(&st.settings), st.notify)) else { return };
    let (changed, now) = {
        let Ok(mut current) = settings.try_borrow_mut() else {
            crate::log_warn!("settings window: the settings are busy; the change was not applied");
            return;
        };
        let before = *current;
        change(&mut current);
        (*current != before, *current)
    };
    if changed {
        post(notify, WM_APP_SETTINGS_CHANGED);
    }
    sync_dependents(state, &now);
}

fn set_launch_at_login(state: &Rc<RefCell<State>>, enabled: bool) {
    let Ok((hwnd, launch)) = state.try_borrow().map(|st| (st.hwnd, st.controls.get(Part::Launch))) else { return };
    match autostart::set_enabled(enabled) {
        Ok(()) => crate::log_info!("launch at login turned {}", if enabled { "on" } else { "off" }),
        Err(message) => {
            crate::log_warn!("launch at login: {message}");
            let text = wide(&message);
            // SAFETY: a modal message box owned by our window; no borrow is held while it runs.
            unsafe {
                MessageBoxW(Some(hwnd), PCWSTR(text.as_ptr()), w!("MelCat"), MB_OK | MB_ICONWARNING);
            }
        }
    }
    set_check(launch, autostart::is_enabled());
}

fn close_window(state: &Rc<RefCell<State>>) {
    let Ok(hwnd) = state.try_borrow().map(|st| st.hwnd) else { return };
    // SAFETY: destroys our own window; WM_DESTROY tells the controller.
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
}

fn post(target: HWND, message: u32) {
    // SAFETY: posts a message without parameters; a stale handle only makes the call fail.
    if let Err(error) = unsafe { PostMessageW(Some(target), message, WPARAM(0), LPARAM(0)) } {
        crate::log_warn!("settings window: could not notify the controller ({message:#x}): {error}");
    }
}

// ---------------------------------------------------------------------------
// Settings to controls

/// Fills every control from the shared settings.
fn sync_all(state: &Rc<RefCell<State>>) {
    let Ok((controls, days, settings, shown_intervals)) = state.try_borrow_mut().map(|mut st| {
        st.syncing = true;
        (st.controls.clone(), st.days, Rc::clone(&st.settings), st.intervals.clone())
    }) else {
        return;
    };
    let settings = settings.try_borrow().map(|settings| *settings);
    if let Ok(settings) = settings {
        for (index, day) in days.iter().enumerate() {
            set_check(controls.get(Part::Day(index)), settings.schedule.days.contains(*day));
        }
        set_picker(controls.get(Part::From), settings.schedule.start);
        set_picker(controls.get(Part::To), settings.schedule.end);
        select_combo(controls.get(Part::Distance), Intensity::ALL.iter().position(|&i| i == settings.intensity));
        let intervals = page::interval_choices(settings.idle_interval);
        let interval = controls.get(Part::Interval);
        if intervals != shown_intervals {
            fill_combo(interval, &intervals.iter().map(|&seconds| describe_interval(seconds)).collect::<Vec<_>>());
        }
        select_combo(interval, intervals.iter().position(|&seconds| seconds == settings.idle_interval));
        set_check(controls.get(Part::Launch), autostart::is_enabled());
        if let Ok(mut st) = state.try_borrow_mut() {
            st.intervals = intervals;
        }
    } else {
        crate::log_warn!("settings window: the settings are busy; the window was not refreshed");
    }
    if let Ok(mut st) = state.try_borrow_mut() {
        st.syncing = false;
    }
    if let Ok(settings) = settings {
        sync_dependents(state, &settings);
    }
}

/// Updates what follows from the mode and the schedule: the mode buttons,
/// the footer, the hint, and whether the schedule rows are enabled.
fn sync_dependents(state: &Rc<RefCell<State>>, settings: &Settings) {
    let Ok((controls, was_enabled)) = state.try_borrow().map(|st| (st.controls.clone(), st.schedule_enabled)) else {
        return;
    };
    for (index, mode) in JiggleMode::MENU_ORDER.iter().enumerate() {
        let radio = controls.get(Part::Mode(index));
        set_check(radio, settings.mode == *mode);
        // Tab lands on the checked button of the group.
        set_tab_stop(radio, settings.mode == *mode);
    }
    set_text(controls.get(Part::ModeFooter), page::mode_footer(settings.mode));
    let hint = page::schedule_hint(&settings.schedule, &text::locale_time_of_day);
    set_text(controls.get(Part::Hint), hint.as_deref().unwrap_or(""));

    let enabled = settings.mode == JiggleMode::Scheduled;
    if enabled == was_enabled {
        return;
    }
    if let Ok(mut st) = state.try_borrow_mut() {
        st.schedule_enabled = enabled;
    }
    // SAFETY: plain calls on our own controls. The focus only moves when it
    // is on a control being disabled, which means our window is the active one.
    unsafe {
        let focus = GetFocus();
        for (part, control) in controls.iter().filter(|(part, _)| part.is_schedule()) {
            if part.kind() == Kind::Label {
                let _ = InvalidateRect(Some(control), None, true);
                continue;
            }
            if !enabled && focus == control {
                let _ = SetFocus(Some(controls.checked_mode()));
            }
            let _ = EnableWindow(control, enabled);
        }
    }
}

// ---------------------------------------------------------------------------
// Building and laying out

fn create_controls(parent: HWND, days: &[Weekday; 7], time_format: &[u16]) -> windows::core::Result<Controls> {
    let mut controls = Vec::with_capacity(Part::ORDER.len());
    for part in Part::ORDER {
        let title = match part {
            Part::Day(index) => days.get(index).map_or("", |day| day.short_name()),
            _ => part.title(),
        };
        let control = create_control(parent, part, title)?;
        if part.kind() == Kind::Picker {
            // SAFETY: the format string is null-terminated and outlives the picker (it is kept in the state).
            unsafe {
                SendMessageW(control, DTM_SETFORMATW, None, Some(LPARAM(time_format.as_ptr() as isize)));
            }
        }
        controls.push((part, control));
    }
    let controls = Controls(controls);
    let intensities: Vec<String> = Intensity::ALL.iter().map(|&intensity| text::intensity_title(intensity)).collect();
    fill_combo(controls.get(Part::Distance), &intensities);
    Ok(controls)
}

fn create_control(parent: HWND, part: Part, title: &str) -> windows::core::Result<HWND> {
    let (class, kind_style) = match part.kind() {
        Kind::Header | Kind::Label => (WC_STATICW, SS_LEFT.0 | SS_NOPREFIX.0 | SS_CENTERIMAGE.0),
        Kind::Note => (WC_STATICW, SS_LEFT.0 | SS_NOPREFIX.0),
        Kind::Radio => (WC_BUTTONW, BS_AUTORADIOBUTTON as u32),
        Kind::Check => (WC_BUTTONW, BS_AUTOCHECKBOX as u32),
        Kind::Picker => (DATETIMEPICK_CLASSW, DTS_TIMEFORMAT | DTS_UPDOWN),
        Kind::Combo => (WC_COMBOBOXW, CBS_DROPDOWNLIST as u32 | WS_VSCROLL.0),
        Kind::Button => (WC_BUTTONW, BS_PUSHBUTTON as u32),
        Kind::DefaultButton => (WC_BUTTONW, BS_DEFPUSHBUTTON as u32),
    };
    let mut style = WS_CHILD | WS_VISIBLE | WINDOW_STYLE(kind_style);
    if part.starts_group() {
        style |= WS_GROUP;
    }
    if part.is_tab_stop() {
        style |= WS_TABSTOP;
    }
    let title = wide(title);
    // SAFETY: a standard control as a child of our window; the ID travels in
    // the menu parameter, as for every child window. Placed by `relayout`.
    unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            PCWSTR(title.as_ptr()),
            style,
            0,
            0,
            0,
            0,
            Some(parent),
            Some(HMENU(std::ptr::without_provenance_mut(usize::from(part.id())))),
            Some(window::instance()),
            None,
        )
    }
}

/// Recreates the fonts at `dpi`, lays the page out, places the controls and
/// repaints them all. Returns the window size that fits the page.
fn relayout(state: &Rc<RefCell<State>>, dpi: u32) -> Option<(i32, i32)> {
    let (hwnd, controls, texts) = {
        let st = state.try_borrow().ok()?;
        (st.hwnd, st.controls.clone(), layout_texts(&st.intervals))
    };
    let fonts = Fonts::new(dpi);
    let layout = measure_and_layout(hwnd, &fonts, dpi, &texts);
    for (part, control) in controls.iter() {
        let font = fonts.get(if part.kind() == Kind::Header { Face::Header } else { Face::Body });
        // SAFETY: WM_SETFONT on our own control; the font lives in the state until the control is gone.
        unsafe {
            SendMessageW(control, WM_SETFONT, Some(WPARAM(font.0 as usize)), Some(LPARAM(0)));
        }
    }
    for &(part, rect) in &layout.parts {
        place(controls.get(part), part, rect, dpi);
    }
    match state.try_borrow_mut() {
        Ok(mut st) => {
            st.dpi = dpi;
            // The old fonts are deleted here, now that no control uses them.
            st.fonts = Some(fonts);
        }
        Err(_) => {
            // Never happens (no borrow is held across control calls); leaking two
            // fonts is better than deleting fonts the controls now use.
            crate::log_warn!("settings window: could not keep the new fonts");
            std::mem::forget(fonts);
        }
    }
    // The controls got their new fonts without redrawing, and a moved static
    // control keeps its old pixels; the parent clips its children, so
    // invalidating it alone would not reach them. Repaint all of it.
    // SAFETY: repaints our own window and its controls.
    unsafe {
        let _ = RedrawWindow(Some(hwnd), None, None, RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN);
    }
    Some(window_size(layout.width, layout.height, dpi))
}

/// The texts the layout measures that depend on the locale and the settings.
fn layout_texts(intervals: &[f64]) -> Texts {
    let mut choices: Vec<String> = Intensity::ALL.iter().map(|&intensity| text::intensity_title(intensity)).collect();
    choices.extend(intervals.iter().map(|&seconds| describe_interval(seconds)));
    Texts {
        choices,
        times: [TimeOfDay::hm(10, 58), TimeOfDay::hm(22, 58)].into_iter().map(text::locale_time_of_day).collect(),
        hints: page::hint_samples(&text::locale_time_of_day),
    }
}

fn measure_and_layout(hwnd: HWND, fonts: &Fonts, dpi: u32, texts: &Texts) -> Layout {
    let scale = f64::from(dpi) / 96.0;
    // SAFETY: a DC of our window, only for measuring, released below.
    let hdc = unsafe { GetDC(Some(hwnd)) };
    let measure = |face: Face, text: &str, wrap: Option<i32>| measure_text(hdc, fonts.get(face), text, wrap, scale);
    let layout = layout::layout(scale, &measure, texts);
    // SAFETY: releases the DC taken above.
    unsafe {
        ReleaseDC(Some(hwnd), hdc);
    }
    layout
}

/// The size of `text` in `font`: one line, or word-wrapped to `wrap` pixels
/// the way a static control wraps it.
fn measure_text(hdc: HDC, font: HFONT, text: &str, wrap: Option<i32>, scale: f64) -> (i32, i32) {
    let mut units: Vec<u16> = text.encode_utf16().collect();
    let mut rect = RECT { left: 0, top: 0, right: wrap.unwrap_or(0).max(1), bottom: 0 };
    let format = DT_CALCRECT | DT_NOPREFIX | if wrap.is_some() { DT_WORDBREAK } else { DT_SINGLELINE };
    // SAFETY: the DC and the font are valid for the call, and the DC's
    // previous font is put back; DT_CALCRECT only writes the rectangle.
    let height = unsafe {
        let previous = SelectObject(hdc, font.into());
        let height = DrawTextW(hdc, &mut units, &mut rect, format);
        SelectObject(hdc, previous);
        height
    };
    if height == 0 && !units.is_empty() {
        // No DC: an estimate keeps the page usable.
        let line = (16.0 * scale).round() as i32;
        let width = (units.len() as f64 * 7.0 * scale).round() as i32;
        return match wrap {
            None => (width, line),
            Some(wrap) => (width.min(wrap), line * ((width + wrap - 1) / wrap.max(1)).max(1)),
        };
    }
    (rect.right - rect.left, rect.bottom - rect.top)
}

fn place(control: HWND, part: Part, rect: RectPx, dpi: u32) {
    let combo = part.kind() == Kind::Combo;
    // A combo box's height includes its drop-down list.
    let list = if combo { DROPDOWN_LIST * dpi as i32 / 96 } else { 0 };
    // SAFETY: moves our own control within our window.
    unsafe {
        let _ = SetWindowPos(
            control,
            None,
            rect.left,
            rect.top,
            rect.width(),
            rect.height() + list,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
        if combo {
            // The closed box takes the height its font needs; center it on the row.
            let mut actual = RECT::default();
            if GetWindowRect(control, &mut actual).is_ok() {
                let height = actual.bottom - actual.top;
                if height > 0 && height < rect.height() {
                    let top = rect.top + (rect.height() - height) / 2;
                    let _ =
                        SetWindowPos(control, None, rect.left, top, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
                }
            }
        }
    }
}

/// The window size for a client area of `width` x `height` at `dpi`.
fn window_size(width: i32, height: i32, dpi: u32) -> (i32, i32) {
    let mut rect = RECT { left: 0, top: 0, right: width, bottom: height };
    // SAFETY: adjusts a local RECT.
    match unsafe { AdjustWindowRectExForDpi(&mut rect, STYLE, false, EX_STYLE, dpi) } {
        Ok(()) => (rect.right - rect.left, rect.bottom - rect.top),
        Err(_) => (width, height),
    }
}

fn window_dpi(hwnd: HWND) -> u32 {
    // SAFETY: plain query of our own window.
    match unsafe { GetDpiForWindow(hwnd) } {
        0 => 96,
        dpi => dpi,
    }
}

/// The class background: the window color, which follows the system colors.
fn window_brush() -> HBRUSH {
    HBRUSH(std::ptr::without_provenance_mut((COLOR_WINDOW.0 + 1) as usize))
}

/// The work area of the monitor under the pointer (the primary one if the
/// pointer cannot be read).
fn work_area_under_pointer() -> RectPx {
    let mut point = POINT::default();
    // SAFETY: plain cursor and monitor queries into locals.
    unsafe {
        let _ = GetCursorPos(&mut point);
        let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        if GetMonitorInfoW(monitor, &mut info).as_bool() {
            RectPx::from_win(info.rcWork)
        } else {
            RectPx::new(0, 0, 1024, 768)
        }
    }
}

// ---------------------------------------------------------------------------
// Control helpers

fn set_check(button: HWND, on: bool) {
    let check = if on { BST_CHECKED } else { BST_UNCHECKED };
    // SAFETY: plain message to our own control.
    unsafe {
        SendMessageW(button, BM_SETCHECK, Some(WPARAM(check.0 as usize)), None);
    }
}

fn is_checked(button: HWND) -> bool {
    // SAFETY: plain query of our own control.
    unsafe { SendMessageW(button, BM_GETCHECK, None, None) }.0 == BST_CHECKED.0 as isize
}

fn set_tab_stop(control: HWND, on: bool) {
    // SAFETY: reads and writes the style of our own control.
    unsafe {
        let style = GetWindowLongPtrW(control, GWL_STYLE);
        let tab_stop = WS_TABSTOP.0 as isize;
        let updated = if on { style | tab_stop } else { style & !tab_stop };
        if updated != style {
            SetWindowLongPtrW(control, GWL_STYLE, updated);
        }
    }
}

fn set_text(control: HWND, text: &str) {
    let text = wide(text);
    // SAFETY: SetWindowTextW copies the null-terminated string.
    unsafe {
        let _ = SetWindowTextW(control, PCWSTR(text.as_ptr()));
    }
}

fn set_picker(picker: HWND, time: TimeOfDay) {
    let value: SYSTEMTIME = page::to_system_time(time);
    // SAFETY: DTM_SETSYSTEMTIME reads the SYSTEMTIME during the call.
    unsafe {
        SendMessageW(
            picker,
            DTM_SETSYSTEMTIME,
            Some(WPARAM(GDT_VALID.0 as usize)),
            Some(LPARAM(&value as *const SYSTEMTIME as isize)),
        );
    }
}

fn fill_combo(combo: HWND, items: &[String]) {
    // SAFETY: CB_ADDSTRING copies each null-terminated string during the call.
    unsafe {
        SendMessageW(combo, CB_RESETCONTENT, None, None);
        for item in items {
            let item = wide(item);
            SendMessageW(combo, CB_ADDSTRING, None, Some(LPARAM(item.as_ptr() as isize)));
        }
    }
}

fn select_combo(combo: HWND, index: Option<usize>) {
    // SAFETY: plain message to our own control; -1 (usize::MAX) clears the selection.
    unsafe {
        SendMessageW(combo, CB_SETCURSEL, Some(WPARAM(index.unwrap_or(usize::MAX))), None);
    }
}

fn combo_selection(combo: HWND) -> Option<usize> {
    // SAFETY: plain query of our own control.
    let index = unsafe { SendMessageW(combo, CB_GETCURSEL, None, None) }.0;
    usize::try_from(index).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_and_title() {
        // SAFETY: CLASS and TITLE are static, null-terminated strings.
        let (class, title) = unsafe { (CLASS.to_string(), TITLE.to_string()) };
        let class = class.unwrap_or_default();
        assert!(class.starts_with(window::CLASS_PREFIX), "{class}");
        assert_eq!(class, "MelCat.Settings");
        assert_eq!(title.unwrap_or_default(), "MelCat Settings");
    }

    #[test]
    fn the_window_cannot_be_resized() {
        use windows::Win32::UI::WindowsAndMessaging::{WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_THICKFRAME};
        assert_eq!(STYLE.0 & (WS_THICKFRAME.0 | WS_MAXIMIZEBOX.0 | WS_MINIMIZEBOX.0), 0);
        assert_eq!(STYLE.0 & WS_SYSMENU.0, WS_SYSMENU.0, "close box");
    }

    #[test]
    fn words_split_into_halves() {
        assert_eq!(loword(0x0002_0078), 0x78);
        assert_eq!(hiword(0x0002_0078), 2);
        assert_eq!(hiword(usize::from(Part::Distance.id()) | ((CBN_SELCHANGE as usize) << 16)), CBN_SELCHANGE);
    }
}
