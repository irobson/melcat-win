//! The notification-area icon and its balloons.
//!
//! The tooltip and the artwork are recorded as shown only once the shell has
//! accepted them, so a change Explorer missed (it was hung, for example) is
//! sent again the next time the status is shown (at least once a minute).

use windows::core::w;
use windows::Win32::Foundation::{ERROR_SUCCESS, HWND};
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use windows::Win32::UI::HiDpi::GetSystemMetricsForDpi;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIIF_WARNING, NIM_ADD, NIM_DELETE,
    NIM_MODIFY, NIM_SETVERSION, NOTIFYICONDATAW, NOTIFYICONDATAW_0, NOTIFY_ICON_MESSAGE, NOTIFYICON_VERSION_4,
};
use windows::Win32::UI::WindowsAndMessaging::SM_CXSMICON;

use crate::geom::{MonitorInfo, PointPx};
use crate::messages::WM_APP_TRAY;
use crate::ui::icons::{icon_from_png, tray_png, Icon};
use crate::ui::text::fixed_wide;

/// The one icon MelCat shows (`uID`).
pub(crate) const TRAY_ID: u32 = 1;
const DEFAULT_TOOLTIP: &str = "MelCat";

/// The kind of balloon, which picks the system icon shown in it. MelCat
/// only ever warns (a setting that could not be saved or applied).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Balloon {
    Warning,
}

/// Which artwork is current, so the icon is only rebuilt when it changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct IconKey {
    dark_taskbar: bool,
    active: bool,
    size: u32,
}

/// Sends one `Shell_NotifyIconW` message; true if the shell accepted it.
type Notify = fn(NOTIFY_ICON_MESSAGE, &NOTIFYICONDATAW) -> bool;

fn shell_notify(message: NOTIFY_ICON_MESSAGE, data: &NOTIFYICONDATAW) -> bool {
    // SAFETY: `data` is a fully initialized NOTIFYICONDATAW for our window.
    unsafe { Shell_NotifyIconW(message, data) }.as_bool()
}

/// The tray icon of the main window.
pub(crate) struct Tray {
    hwnd: HWND,
    added: bool,
    active: bool,
    /// The tooltip the shell shows (while added), or will get when added.
    tip: String,
    /// The artwork the shell shows (while added); `None` when it must be sent again.
    key: Option<IconKey>,
    icon: Option<Icon>,
    notify: Notify,
}

/// True when the taskbar uses the dark theme
/// (`HKCU\...\Themes\Personalize\SystemUsesLightTheme` is 0 or missing).
pub(crate) fn taskbar_is_dark() -> bool {
    let mut value: u32 = 1;
    let mut size = std::mem::size_of::<u32>() as u32;
    // SAFETY: reads a DWORD into a local of the right size.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            w!("SystemUsesLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut value as *mut u32 as *mut _),
            Some(&mut size),
        )
    };
    // Windows 10 before 1903 has no light taskbar and no such value.
    status != ERROR_SUCCESS || value == 0
}

/// The small-icon size for the primary monitor's DPI (where the notification area lives).
fn small_icon_size() -> u32 {
    let primary = MonitorInfo::from_point(PointPx::new(0, 0));
    let dpi = (primary.scale * 96.0).round() as u32;
    // SAFETY: a plain metrics query.
    let size = unsafe { GetSystemMetricsForDpi(SM_CXSMICON, dpi) };
    if size > 0 {
        size as u32
    } else {
        ((16.0 * primary.scale).round() as u32).max(16)
    }
}

impl Tray {
    /// A tray icon for `hwnd`, not yet added (call [`Tray::ensure_added`]).
    pub(crate) fn new(hwnd: HWND) -> Tray {
        Tray::with_notify(hwnd, shell_notify)
    }

    fn with_notify(hwnd: HWND, notify: Notify) -> Tray {
        Tray { hwnd, added: false, active: false, tip: DEFAULT_TOOLTIP.to_owned(), key: None, icon: None, notify }
    }

    fn data(&self) -> NOTIFYICONDATAW {
        NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: TRAY_ID,
            ..Default::default()
        }
    }

    /// Rebuilds the icon if the theme, DPI or state changed. Returns true if it changed.
    fn update_icon(&mut self) -> bool {
        let key = IconKey { dark_taskbar: taskbar_is_dark(), active: self.active, size: small_icon_size() };
        if self.key == Some(key) && self.icon.is_some() {
            return false;
        }
        match icon_from_png(tray_png(key.dark_taskbar, key.active), key.size) {
            Ok(icon) => {
                self.icon = Some(icon);
                self.key = Some(key);
                true
            }
            Err(error) => {
                crate::log_error!("could not build the tray icon: {error}");
                false
            }
        }
    }

    /// Adds the icon if it is not in the notification area yet (also after
    /// Explorer restarted). Returns false if the shell refused; retry later.
    pub(crate) fn ensure_added(&mut self) -> bool {
        self.update_icon();
        let mut data = self.data();
        data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP;
        data.uCallbackMessage = WM_APP_TRAY;
        data.hIcon = self.icon.as_ref().map(Icon::handle).unwrap_or_default();
        data.szTip = fixed_wide(&self.tip);
        let send = self.notify;
        let notify = |message| send(message, &data);
        // An icon that is already there (for example after a NIM_ADD that timed
        // out but did succeed) refuses NIM_ADD and accepts NIM_MODIFY.
        let (first, second) = if self.added { (NIM_MODIFY, NIM_ADD) } else { (NIM_ADD, NIM_MODIFY) };
        let ok = notify(first) || notify(second);
        if !ok {
            crate::log_warn!("Shell_NotifyIcon could not add the tray icon (will retry)");
            self.added = false;
            return false;
        }
        data.Anonymous = NOTIFYICONDATAW_0 { uVersion: NOTIFYICON_VERSION_4 };
        if !(self.notify)(NIM_SETVERSION, &data) {
            crate::log_warn!("NIM_SETVERSION 4 was refused");
        }
        self.added = true;
        true
    }

    /// Explorer restarted (`TaskbarCreated`): the old icon is gone, add it again.
    pub(crate) fn readd(&mut self) -> bool {
        self.added = false;
        self.ensure_added()
    }

    /// Re-reads the theme and DPI and swaps the icon if needed.
    pub(crate) fn refresh(&mut self) {
        if self.update_icon() && self.added && !self.modify_icon() {
            // Not shown: build and send it again next time.
            self.key = None;
        }
    }

    /// Switches between the active and the paused (dimmed) artwork, and
    /// sends the artwork again if the shell missed it last time.
    pub(crate) fn set_active(&mut self, active: bool) {
        if self.active != active || self.key.is_none() {
            self.active = active;
            self.refresh();
        }
    }

    /// Sets the tooltip (at most 127 characters; longer text is cut with an
    /// ellipsis). If the shell refuses it, the same text is sent again on the next call.
    pub(crate) fn set_tip(&mut self, tip: &str) {
        if self.tip == tip {
            return;
        }
        if !self.added {
            // Sent by `ensure_added`.
            self.tip = tip.to_owned();
            return;
        }
        let mut data = self.data();
        data.uFlags = NIF_TIP | NIF_SHOWTIP;
        data.szTip = fixed_wide(tip);
        if (self.notify)(NIM_MODIFY, &data) {
            self.tip = tip.to_owned();
        } else {
            crate::log_warn!("could not update the tray tooltip (will retry)");
        }
    }

    /// Sends the current artwork; true if the shell accepted it.
    fn modify_icon(&self) -> bool {
        let mut data = self.data();
        data.uFlags = NIF_ICON;
        data.hIcon = self.icon.as_ref().map(Icon::handle).unwrap_or_default();
        let accepted = (self.notify)(NIM_MODIFY, &data);
        if !accepted {
            crate::log_warn!("could not update the tray icon (will retry)");
        }
        accepted
    }

    /// Shows a balloon (a toast on Windows 10 and 11) from the tray icon.
    pub(crate) fn balloon(&self, title: &str, text: &str, kind: Balloon) {
        crate::log_info!("notice: {title}: {text}");
        if !self.added {
            return;
        }
        let mut data = self.data();
        data.uFlags = NIF_INFO;
        data.szInfoTitle = fixed_wide(title);
        data.szInfo = fixed_wide(text);
        data.dwInfoFlags = match kind {
            Balloon::Warning => NIIF_WARNING,
        };
        if !(self.notify)(NIM_MODIFY, &data) {
            crate::log_warn!("could not show a balloon");
        }
    }

    /// Removes the icon from the notification area.
    pub(crate) fn remove(&mut self) {
        if self.added {
            let data = self.data();
            // Identifies our icon by window and id.
            let _ = (self.notify)(NIM_DELETE, &data);
            self.added = false;
        }
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        self.remove();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};

    use crate::util::from_wide;

    /// One message the fake shell got: the message, its flags and its tooltip.
    #[derive(Clone, Debug, PartialEq)]
    struct Sent {
        message: NOTIFY_ICON_MESSAGE,
        flags: u32,
        tip: String,
    }

    thread_local! {
        static SENT: RefCell<Vec<Sent>> = const { RefCell::new(Vec::new()) };
        /// The next message the fake shell refuses (like a send that timed out on a hung Explorer).
        static REFUSE_NEXT: Cell<bool> = const { Cell::new(false) };
    }

    fn fake_shell(message: NOTIFY_ICON_MESSAGE, data: &NOTIFYICONDATAW) -> bool {
        let tip = from_wide(&data.szTip);
        SENT.with(|sent| sent.borrow_mut().push(Sent { message, flags: data.uFlags.0, tip }));
        !REFUSE_NEXT.with(|refuse| refuse.replace(false))
    }

    /// What the fake shell got since the last call.
    fn sent() -> Vec<Sent> {
        SENT.with(|sent| std::mem::take(&mut *sent.borrow_mut()))
    }

    fn refuse_next() {
        REFUSE_NEXT.with(|refuse| refuse.set(true));
    }

    /// A tray icon on this test's thread, which needs COM for the artwork
    /// (WIC), as the UI thread has.
    fn tray() -> Tray {
        // SAFETY: initializes COM for this test thread, which ends with the test.
        let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        Tray::with_notify(HWND::default(), fake_shell)
    }

    /// A tray icon already in the notification area.
    fn added_tray() -> Tray {
        let mut tray = tray();
        assert!(tray.ensure_added());
        let added = sent();
        assert_eq!(added.first().map(|sent| sent.message), Some(NIM_ADD), "{added:?}");
        tray
    }

    #[test]
    fn a_refused_tooltip_is_sent_again() {
        let mut tray = added_tray();
        refuse_next();
        tray.set_tip("MelCat: Mel is playing");
        assert_eq!(sent().len(), 1);
        // The next status update (at least once a minute) sends it again.
        tray.set_tip("MelCat: Mel is playing");
        let again = sent();
        assert_eq!(
            again,
            [Sent {
                message: NIM_MODIFY,
                flags: (NIF_TIP | NIF_SHOWTIP).0,
                tip: "MelCat: Mel is playing".to_owned()
            }]
        );
        // Accepted now: nothing more to send.
        tray.set_tip("MelCat: Mel is playing");
        assert_eq!(sent(), []);
    }

    #[test]
    fn a_refused_icon_is_sent_again() {
        let mut tray = added_tray();
        refuse_next();
        tray.set_active(true);
        let first = sent();
        assert_eq!(first.len(), 1, "{first:?}");
        assert_eq!((first[0].message, first[0].flags), (NIM_MODIFY, NIF_ICON.0));
        // Same state, but the shell never showed it: sent again.
        tray.set_active(true);
        let again = sent();
        assert_eq!(again.len(), 1, "{again:?}");
        assert_eq!((again[0].message, again[0].flags), (NIM_MODIFY, NIF_ICON.0));
        // Accepted now: nothing more to send.
        tray.set_active(true);
        assert_eq!(sent(), []);
    }

    #[test]
    fn changes_before_the_icon_is_added_wait_for_it() {
        let mut tray = tray();
        tray.set_tip("MelCat: Mel is napping");
        tray.set_active(true);
        assert_eq!(sent(), []);
        assert!(tray.ensure_added());
        let added = sent();
        let first = added.first().map(|sent| (sent.message, sent.tip.as_str()));
        assert_eq!(first, Some((NIM_ADD, "MelCat: Mel is napping")), "{added:?}");
    }
}
