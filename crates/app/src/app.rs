//! The UI-thread controller.
//!
//! CONTRACT (owned by the app agent). [`run`]:
//!
//! 1. Logs `MelCat <version> starting`. Initializes COM apartment-threaded
//!    (WIC decodes the tray icons) and uninitializes it on the way out.
//! 2. Single instance: named mutex `Local\MelCat.SingleInstance`. A second
//!    MelCat logs `another MelCat is already running; exiting` and returns 0.
//! 3. `InitCommonControlsEx` with `ICC_STANDARD_CLASSES | ICC_DATE_CLASSES`
//!    (the settings window's controls and time pickers).
//! 4. Creates `MelCat.Main` (hidden top-level, `WS_EX_TOOLWINDOW`) and
//!    `MelCat.Msg` (message-only), see `messages.rs` for what each receives.
//!    Registers `TaskbarCreated` (and lets it through UIPI with
//!    `ChangeWindowMessageFilterEx`), raw input (`jiggle::watch::register` on
//!    `MelCat.Msg`; a failure is logged and MelCat runs without it, relying
//!    on the cursor check), and the session monitor (`jiggle::session` on
//!    `MelCat.Main`).
//! 5. Loads `Settings` (shared with the settings window as
//!    `Rc<RefCell<Settings>>`) and notes the file's stamp (modification
//!    time and size, `Settings::stamp_of`), adds the tray icon (retrying on
//!    `TRAY_RETRY_TIMER` every 2 s, up to 30 times, if Explorer is not ready;
//!    logs `tray icon added` once it is there), starts `TICK_TIMER`, and runs
//!    one tick at once. If `settings.json` did not exist at start (first
//!    launch), it opens the settings window, like the macOS app.
//! 6. Runs the message loop, giving the settings window's messages to
//!    `IsDialogMessageW` first, until Quit. Returns the exit code.
//!
//! Messages:
//! - `TICK_TIMER`: first, if the stamp of `settings.json` differs from the
//!   one noted when MelCat last read or saved it (someone edited the file),
//!   reload it (`Settings::read_from`), log `settings reloaded: <settings>`
//!   and refresh an open settings window, like the macOS app reading its
//!   defaults through on every tick. A file that cannot be read or parsed is
//!   logged and the settings in force are kept; so are they when the file is
//!   removed. Then `jiggler.tick(&settings, || session.is_away())` (the
//!   jiggler asks only when a stroll could start); on a status change or a
//!   reload, update the tray: `tray.set_active(status.is_active())` and the
//!   tooltip `MelCat: <status line>` (`ui::text::status_line`). The tooltip
//!   is also refreshed once a minute, so "tomorrow" and "today" stay right.
//!   The same tick also runs after every settings change and before the tray
//!   menu opens, so the menu shows an edited file at once.
//! - `WALK_TIMER`: `jiggler.on_walk_timer()`.
//! - `WM_INPUT`: `if watch::is_personal_input(lparam) { jiggler.note_personal_input() }`,
//!   then `DefWindowProcW`.
//! - `WM_APP_TRAY` (`NOTIFYICON_VERSION_4`: event in `LOWORD(lParam)`, anchor
//!   in `wParam`): `WM_CONTEXTMENU`, `NIN_SELECT` and `NIN_KEYSELECT` open the
//!   tray menu (`ui::menu_model::build_menu` + `ui::menu::show_popup`) at the
//!   anchor. A click within 300 ms of the menu closing is the click that
//!   closed it and does not reopen it.
//! - Menu commands: `SetMode`, `SetIntensity`, `SetIdleInterval` change the
//!   settings, save them (`Settings::save`, then note the new stamp, so the
//!   save is not taken for an edit; a failure is a warning balloon),
//!   tick at once and refresh an open settings window. `OpenSettings` opens or
//!   activates the settings window. `ToggleLaunchAtLogin` calls
//!   `ui::autostart::set_enabled` (an error is a warning balloon).
//!   `OpenLog` opens `util::log::log_file()` with `ShellExecuteW` "open".
//!   `Quit` leaves the loop.
//! - `WM_APP_SETTINGS_CHANGED`: save (as above), tick at once, update the tray.
//!   `WM_APP_PREVIEW`: `jiggler.preview(settings.intensity)`.
//!   `WM_APP_SETTINGS_CLOSED`: drop the settings window handle.
//! - `WM_WTSSESSION_CHANGE`, `WM_POWERBROADCAST`: forward to the session
//!   monitor; when it reports a change, `jiggler.interrupt("session changed")`.
//! - `WM_DISPLAYCHANGE`: `jiggler.interrupt("display changed")`.
//! - `WM_SETTINGCHANGE` (theme, "ImmersiveColorSet"), `WM_DPICHANGED`:
//!   `tray.refresh()`. `TaskbarCreated`: `tray.readd()`.
//! - `WM_QUERYENDSESSION`: `jiggler.interrupt("session ending")`, return TRUE
//!   (never veto). `WM_ENDSESSION` with `wParam` TRUE: the same, then clean up.
//!
//! On quit: interrupt the jiggler, kill the timers, close the settings
//! window, remove the tray icon, unregister raw input and the session
//! monitor, destroy both windows, log `MelCat stopped`.
//!
//! Balloons are `Tray::balloon` with a title and a [`crate::messages::sentence`].
//!
//! How messages flow: both window procedures turn the messages they care
//! about into an [`Event`] and queue it. The queue is drained by whichever
//! call is not nested inside the controller: if the controller is busy (a
//! handler is waiting in a call that lets sent messages through, such as
//! `Shell_NotifyIconW`), the event waits and runs right after the current
//! handler, so the controller is never re-entered and its `RefCell` is never
//! borrowed twice. Calls that run a modal loop or may pump messages (the tray
//! menu, `ShellExecuteW`) run outside the controller as a [`Followup`], so
//! the ticks and the strolls go on while the menu is open, like the macOS
//! app's timers in the common run loop modes.
//!
//! Two things are read synchronously, inside the window procedure, because
//! their parameters are only valid while the message is being handled: raw
//! input (`GetRawInputData`) and `WM_POWERBROADCAST`, whose `lParam` points
//! at a `POWERBROADCAST_SETTING`. That is why the session monitor lives in
//! its own thread-local next to the controller: it is reachable even while
//! the controller is busy.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant as Clock, SystemTime};

use melcat_core::{ChronoCalendar, Instant};
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ACCESS_DENIED, ERROR_ALREADY_EXISTS, HANDLE, HWND, LPARAM, LRESULT, WPARAM,
};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::Controls::{
    InitCommonControlsEx, ICC_DATE_CLASSES, ICC_STANDARD_CLASSES, INITCOMMONCONTROLSEX,
};
use windows::Win32::UI::Shell::{ShellExecuteW, NINF_KEY, NIN_SELECT};
use windows::Win32::UI::WindowsAndMessaging::{
    ChangeWindowMessageFilterEx, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    IsDialogMessageW, IsWindow, KillTimer, PostQuitMessage, RegisterWindowMessageW, SetTimer, TranslateMessage,
    HWND_MESSAGE, MSG, MSGFLT_ALLOW, SW_SHOWNORMAL, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLOSE, WM_CONTEXTMENU,
    WM_DISPLAYCHANGE, WM_DPICHANGED, WM_ENDSESSION, WM_INPUT, WM_POWERBROADCAST, WM_QUERYENDSESSION, WM_SETTINGCHANGE,
    WM_TIMER, WM_WTSSESSION_CHANGE, WNDPROC, WS_EX_TOOLWINDOW, WS_POPUP,
};

use crate::geom::PointPx;
use crate::jiggle::jiggler::Jiggler;
use crate::jiggle::session::SessionMonitor;
use crate::jiggle::{pointer, watch, Status};
use crate::messages::{
    io_error_text, sentence, TICK_MILLIS, TICK_TIMER, TRAY_RETRY_TIMER, WALK_TIMER, WM_APP_PREVIEW,
    WM_APP_SETTINGS_CHANGED, WM_APP_SETTINGS_CLOSED, WM_APP_TRAY,
};
use crate::settings::{sanitize_interval, Settings};
use crate::ui::menu_model::{build_menu, Command, MenuEntry, MenuState};
use crate::ui::settings_window::SettingsWindow;
use crate::ui::tray::{Balloon, Tray};
use crate::ui::{autostart, menu, text, window};
use crate::util::paths;

/// The hidden top-level window: tray callbacks, broadcasts and the end of the session.
const MAIN_CLASS: PCWSTR = w!("MelCat.Main");
/// The message-only window: timers, raw input and MelCat's own notifications.
const MSG_CLASS: PCWSTR = w!("MelCat.Msg");
const MUTEX_NAME: PCWSTR = w!("Local\\MelCat.SingleInstance");

/// How often, and how many times, adding the tray icon is retried while
/// Explorer is not ready (for example right after logging in).
const TRAY_RETRY_MILLIS: u32 = 2000;
const TRAY_RETRY_LIMIT: u32 = 30;
/// A tray click this soon after the menu closed is the click that closed it.
const MENU_REOPEN_GUARD: Duration = Duration::from_millis(300);
/// `NIN_KEYSELECT` (Enter or Space on the focused tray icon).
const NIN_KEYSELECT: u32 = NIN_SELECT | NINF_KEY;

/// The tray tooltip for a status line, as on macOS: "MelCat: Mel is playing · until 18:00".
fn tooltip(status_line: &str) -> String {
    format!("MelCat: {status_line}")
}

/// The wall-clock minute an instant falls in (Unix minutes; every time zone
/// in use today is a whole number of minutes from UTC, so local midnight is
/// a minute boundary too).
fn minute_of(now: Instant) -> i64 {
    now.0.div_euclid(60)
}

/// True when the tooltip was last written in another minute than `now`'s
/// (or never), so words like "today" and "tomorrow" may be stale.
fn tip_is_stale(shown_minute: Option<i64>, now: Instant) -> bool {
    shown_minute != Some(minute_of(now))
}

/// A `WM_APP_TRAY` with `NOTIFYICON_VERSION_4`: the event is `LOWORD(lParam)`
/// (the icon id is in the high word), the anchor is `wParam`'s signed X and Y.
fn decode_tray(wparam: WPARAM, lparam: LPARAM) -> (u32, PointPx) {
    let (x, y) = window::lparam_point(LPARAM(wparam.0 as isize));
    ((lparam.0 & 0xFFFF) as u32, PointPx::new(x, y))
}

/// The tray events that open the menu: a right click (`WM_CONTEXTMENU`), a
/// left click (`NIN_SELECT`) and Enter or Space on the focused icon (`NIN_KEYSELECT`).
fn opens_menu(event: u32) -> bool {
    matches!(event, WM_CONTEXTMENU | NIN_SELECT | NIN_KEYSELECT)
}

/// True when a tray click must not open the menu: one is open already, or it
/// closed less than [`MENU_REOPEN_GUARD`] ago (the click on the icon that
/// closed it arrives right after it closes).
fn menu_blocked(open: bool, closed_at: Option<Clock>, now: Clock) -> bool {
    open || closed_at.is_some_and(|closed| now.saturating_duration_since(closed) < MENU_REOPEN_GUARD)
}

/// The balloon text when the settings file cannot be written. Whole
/// sentences: "Access is denied (error 5). The change applies until MelCat quits."
fn save_failure_text(error: &std::io::Error) -> String {
    format!("{} The change applies until MelCat quits.", sentence(&io_error_text(error)))
}

/// A Windows error as a sentence for the user: its message, or its code when
/// Windows has no text for it.
fn error_text(error: &windows::core::Error) -> String {
    let message = error.message();
    if message.trim().is_empty() {
        format!("Windows error 0x{:08X}.", error.code().0 as u32)
    } else {
        sentence(&message)
    }
}

/// Runs MelCat until the user quits. Returns the process exit code.
pub fn run() -> i32 {
    crate::log_info!("MelCat {} starting", env!("CARGO_PKG_VERSION"));
    // SAFETY: the first COM initialization on the UI thread, balanced below.
    let com = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
    if com.is_err() {
        crate::log_error!("CoInitializeEx failed: {}", windows::core::Error::from_hresult(com));
    }
    let code = run_app();
    if com.is_ok() {
        // SAFETY: balances the successful initialization above, on the same thread.
        unsafe { CoUninitialize() };
    }
    code
}

fn run_app() -> i32 {
    let Some(_instance) = SingleInstance::acquire() else {
        crate::log_info!("another MelCat is already running; exiting");
        return 0;
    };
    let controls = INITCOMMONCONTROLSEX {
        dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
        dwICC: ICC_STANDARD_CLASSES | ICC_DATE_CLASSES,
    };
    // SAFETY: registers the common control classes (buttons, combo boxes, time pickers) for this process.
    if !unsafe { InitCommonControlsEx(&controls) }.as_bool() {
        crate::log_warn!("InitCommonControlsEx failed");
    }

    let main = match create_main_window() {
        Ok(main) => main,
        Err(error) => {
            crate::log_error!("could not create the main window: {error}");
            return 1;
        }
    };
    let msg = match create_message_window() {
        Ok(msg) => msg,
        Err(error) => {
            crate::log_error!("could not create the message window: {error}");
            // SAFETY: destroys the window created above, on its own thread.
            unsafe {
                let _ = DestroyWindow(main);
            }
            return 1;
        }
    };
    // SAFETY: registers a well-known message name; letting it through UIPI
    // matters only when MelCat runs elevated and Explorer does not.
    unsafe {
        let taskbar_created = RegisterWindowMessageW(w!("TaskbarCreated"));
        TASKBAR_CREATED.set(taskbar_created);
        if taskbar_created == 0 {
            crate::log_warn!("could not register TaskbarCreated: {}", windows::core::Error::from_thread());
        } else if let Err(error) = ChangeWindowMessageFilterEx(main, taskbar_created, MSGFLT_ALLOW, None) {
            crate::log_warn!("could not let TaskbarCreated through UIPI: {error}");
        }
    }
    let raw_input = match watch::register(msg) {
        Ok(()) => true,
        Err(error) => {
            crate::log_warn!("raw input unavailable ({error}); relying on the cursor check");
            false
        }
    };
    SESSION.with(|slot| *slot.borrow_mut() = Some(SessionMonitor::register(main)));

    let first_launch = !Settings::path().exists();
    // Noted before reading, so an edit made in between is reloaded at the first tick.
    let settings_stamp = Settings::stamp_of(&Settings::path());
    let settings = Settings::load();
    crate::log_info!("settings: {settings:?}");
    let controller = Controller {
        main,
        msg,
        settings: Rc::new(RefCell::new(settings)),
        settings_stamp,
        jiggler: Jiggler::new(msg),
        tray: Tray::new(main),
        tray_retrying: false,
        tray_retries: 0,
        raw_input,
        settings_window: None,
        tip_minute: None,
    };
    CONTROLLER.with(|slot| *slot.borrow_mut() = Some(controller));
    // The rest of the start runs as a handler, so anything that arrives
    // meanwhile waits in the queue instead of finding no controller.
    dispatch(Event::Start { first_launch });

    let code = message_loop();

    // Already gone if the end of the session cleaned up.
    if let Some(controller) = take_controller() {
        controller.shutdown("quit");
    }
    QUEUE.with(|queue| queue.borrow_mut().clear());
    code
}

/// Holds the single-instance mutex for the life of the process.
struct SingleInstance(HANDLE);

impl SingleInstance {
    /// `None` when another MelCat already holds the mutex.
    fn acquire() -> Option<SingleInstance> {
        // SAFETY: creates or opens a named mutex; GetLastError is read right after.
        unsafe {
            match CreateMutexW(None, false, MUTEX_NAME) {
                Ok(handle) => {
                    if GetLastError() == ERROR_ALREADY_EXISTS {
                        let _ = CloseHandle(handle);
                        None
                    } else {
                        Some(SingleInstance(handle))
                    }
                }
                // Held by an instance we may not open (an elevated one).
                Err(error) if error.code() == ERROR_ACCESS_DENIED.to_hresult() => None,
                Err(error) => {
                    crate::log_warn!("single-instance mutex unavailable ({error}); continuing");
                    Some(SingleInstance(HANDLE::default()))
                }
            }
        }
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            // SAFETY: closes the mutex handle we own.
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
}

fn create_window(
    class: PCWSTR,
    proc: WNDPROC,
    ex_style: WINDOW_EX_STYLE,
    style: WINDOW_STYLE,
    parent: Option<HWND>,
) -> windows::core::Result<HWND> {
    window::register_class(class, proc, None)?;
    // SAFETY: standard window creation with a class registered just above; the window is never shown.
    unsafe {
        CreateWindowExW(ex_style, class, w!("MelCat"), style, 0, 0, 0, 0, parent, None, Some(window::instance()), None)
    }
}

/// The hidden main window: a real top-level window (not message-only), so it
/// receives broadcasts such as `TaskbarCreated` and `WM_SETTINGCHANGE`, the
/// session and power notifications, and `WM_QUERYENDSESSION` /
/// `WM_ENDSESSION`. It also owns the tray icon and its menu.
fn create_main_window() -> windows::core::Result<HWND> {
    create_window(MAIN_CLASS, Some(main_proc), WS_EX_TOOLWINDOW, WS_POPUP, None)
}

/// The message-only window (parent `HWND_MESSAGE`): never reached by broadcasts.
fn create_message_window() -> windows::core::Result<HWND> {
    create_window(MSG_CLASS, Some(msg_proc), WINDOW_EX_STYLE(0), WINDOW_STYLE(0), Some(HWND_MESSAGE))
}

fn message_loop() -> i32 {
    let mut message = MSG::default();
    loop {
        // SAFETY: the standard message loop on the UI thread.
        let result = unsafe { GetMessageW(&mut message, None, 0, 0) };
        match result.0 {
            0 => return message.wParam.0 as i32,
            -1 => {
                crate::log_error!("GetMessageW failed: {}", windows::core::Error::from_thread());
                return 1;
            }
            _ => {
                let dialog = DIALOG.get();
                // SAFETY: IsDialogMessageW only takes messages for `dialog` and
                // its controls (Tab, Enter, Escape) and dispatches them itself;
                // for a window that is already gone it just returns FALSE.
                if !dialog.is_invalid() && unsafe { IsDialogMessageW(dialog, &message) }.as_bool() {
                    continue;
                }
                // SAFETY: as above.
                unsafe {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Message plumbing

/// A message for the controller, already decoded.
enum Event {
    /// The rest of [`run`]'s step 5: tray icon, tick timer, first tick, first launch.
    Start {
        first_launch: bool,
    },
    Tick,
    WalkStep,
    /// A key, a mouse button or the wheel (raw input).
    PersonalInput,
    Tray {
        event: u32,
        anchor: PointPx,
    },
    MenuCommand(u32),
    TrayRetry,
    TaskbarCreated,
    /// Theme or DPI change: the tray artwork may need swapping.
    RefreshTray,
    DisplayChanged,
    /// The session monitor reported a possible change of the away state.
    SessionChanged,
    SettingsChanged,
    Preview,
    SettingsClosed,
    /// A warning balloon from work done outside the controller.
    Notice {
        title: &'static str,
        text: String,
    },
    /// `WM_CLOSE` on the main window (for example `taskkill` without `/f`).
    Close,
}

/// Work that runs a modal loop or may pump messages, so it runs outside the controller.
enum Followup {
    Menu {
        owner: HWND,
        entries: Vec<MenuEntry>,
        at: PointPx,
    },
    /// `ShellExecuteW` may pump messages (DDE, COM) while it starts the viewer.
    Open {
        path: PathBuf,
        failure_title: &'static str,
    },
}

thread_local! {
    static CONTROLLER: RefCell<Option<Controller>> = const { RefCell::new(None) };
    static QUEUE: RefCell<VecDeque<Event>> = const { RefCell::new(VecDeque::new()) };
    /// Outside the controller: `WM_POWERBROADCAST` must be read while it is
    /// being handled, even when the controller is busy (see the module docs).
    static SESSION: RefCell<Option<SessionMonitor>> = const { RefCell::new(None) };
    /// True while the tray menu runs its modal loop.
    static MENU_OPEN: Cell<bool> = const { Cell::new(false) };
    static MENU_CLOSED_AT: Cell<Option<Clock>> = const { Cell::new(None) };
    static TASKBAR_CREATED: Cell<u32> = const { Cell::new(0) };
    /// The open settings window, for `IsDialogMessageW` in the message loop.
    static DIALOG: Cell<HWND> = const { Cell::new(HWND(std::ptr::null_mut())) };
}

fn enqueue(event: Event) {
    QUEUE.with(|queue| queue.borrow_mut().push_back(event));
}

fn dispatch(event: Event) {
    enqueue(event);
    drain();
}

/// Handles queued events until the queue is empty. Returns at once when
/// called from inside a controller handler; that outer drain goes on.
fn drain() {
    loop {
        // `None`: the controller is busy or the queue is empty.
        let handled = CONTROLLER.with(|slot| {
            let mut slot = slot.try_borrow_mut().ok()?;
            let event = QUEUE.with(|queue| queue.borrow_mut().pop_front())?;
            // Without a controller (MelCat is quitting) nothing handles it any more.
            Some(slot.as_mut().and_then(|controller| controller.handle(event)))
        });
        match handled {
            None => return,
            Some(Some(followup)) => run_followup(followup),
            Some(None) => {}
        }
    }
}

/// Takes the controller out of its slot, unless a handler further up the stack has it.
fn take_controller() -> Option<Controller> {
    CONTROLLER.with(|slot| slot.try_borrow_mut().ok().and_then(|mut slot| slot.take()))
}

/// Runs `f` on the controller when it exists and is not borrowed further up the stack.
fn with_free_controller<R>(f: impl FnOnce(&mut Controller) -> R) -> Option<R> {
    CONTROLLER.with(|slot| {
        let mut slot = slot.try_borrow_mut().ok()?;
        slot.as_mut().map(f)
    })
}

fn run_followup(followup: Followup) {
    match followup {
        Followup::Menu { owner, entries, at } => {
            if MENU_OPEN.replace(true) {
                return;
            }
            let chosen = menu::show_popup(owner, &entries, at);
            MENU_OPEN.set(false);
            MENU_CLOSED_AT.set(Some(Clock::now()));
            if let Some(id) = chosen {
                // The drain that ran this followup handles it next.
                enqueue(Event::MenuCommand(id));
            }
        }
        Followup::Open { path, failure_title } => {
            if let Err(text) = shell_open(&path) {
                crate::log_warn!("{text}");
                enqueue(Event::Notice { title: failure_title, text });
            }
        }
    }
}

/// Opens `path` with its default app. The error is a sentence for the user.
fn shell_open(path: &Path) -> Result<(), String> {
    let target = HSTRING::from(path.as_os_str());
    // SAFETY: plain ShellExecuteW with null-terminated strings that outlive the call.
    let result = unsafe { ShellExecuteW(None, w!("open"), &target, PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL) };
    // ShellExecuteW returns a value greater than 32 on success.
    if result.0 as isize > 32 {
        Ok(())
    } else {
        Err(sentence(&format!("Windows could not open {} (error {})", path.display(), result.0 as isize)))
    }
}

/// Feeds a notification to the session monitor. True when the away state
/// may have changed (also when the monitor cannot be reached, so a stroll
/// in progress stops rather than goes on).
fn session_changed(forward: impl FnOnce(&mut SessionMonitor) -> bool) -> bool {
    SESSION.with(|slot| match slot.try_borrow_mut() {
        Ok(mut slot) => slot.as_mut().is_some_and(forward),
        Err(_) => true,
    })
}

/// True when nobody can be at this session's screen. When the monitor is
/// busy (never expected), MelCat stays still for this tick.
fn session_is_away() -> bool {
    SESSION.with(|slot| match slot.try_borrow() {
        Ok(slot) => slot.as_ref().is_some_and(SessionMonitor::is_away),
        Err(_) => true,
    })
}

/// `WM_QUERYENDSESSION`: MelCat never vetoes; it only lets go of the pointer.
fn query_end_session() {
    let _ = with_free_controller(|controller| controller.jiggler.interrupt("session ending"));
}

/// `WM_ENDSESSION` (ending): Windows may end the process as soon as this
/// returns, so MelCat cleans up now.
fn end_session() {
    crate::log_info!("session ending");
    // A handler further up the stack may have the controller (never expected);
    // the end of `run_app` then cleans up, if Windows lets it get there.
    if let Some(controller) = take_controller() {
        controller.shutdown("session ending");
    }
    // SAFETY: ends the message loop of this (the UI) thread.
    unsafe { PostQuitMessage(0) };
}

/// The top-level window: tray callbacks, broadcasts, power and session
/// notifications, the end of the session. Other processes' broadcasts reach
/// it too, so it never trusts a message's parameters beyond its type (it
/// reads no pointer from them; `WM_POWERBROADCAST` goes to the session monitor).
extern "system" fn main_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let event = match message {
        WM_APP_TRAY => {
            let (event, anchor) = decode_tray(wparam, lparam);
            Event::Tray { event, anchor }
        }
        WM_TIMER if wparam.0 == TRAY_RETRY_TIMER => Event::TrayRetry,
        // Any setting change may be the theme; a refresh only swaps the icon
        // when the artwork really changed, and needs nothing from `lParam`.
        WM_SETTINGCHANGE | WM_DPICHANGED => Event::RefreshTray,
        WM_DISPLAYCHANGE => Event::DisplayChanged,
        WM_WTSSESSION_CHANGE => {
            if session_changed(|session| session.on_session_change(wparam)) {
                dispatch(Event::SessionChanged);
            }
            return LRESULT(0);
        }
        WM_POWERBROADCAST => {
            if session_changed(|session| session.on_power_broadcast(wparam, lparam)) {
                dispatch(Event::SessionChanged);
            }
            // TRUE: grants any (legacy) request.
            return LRESULT(1);
        }
        WM_QUERYENDSESSION => {
            query_end_session();
            // MelCat never vetoes the end of the session.
            return LRESULT(1);
        }
        WM_ENDSESSION => {
            if wparam.0 != 0 {
                end_session();
            }
            return LRESULT(0);
        }
        WM_CLOSE => Event::Close,
        _ if message != 0 && message == TASKBAR_CREATED.get() => Event::TaskbarCreated,
        // SAFETY: default handling for everything else.
        _ => return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    };
    dispatch(event);
    LRESULT(0)
}

/// The message-only window: the timers, raw input and MelCat's own notifications.
extern "system" fn msg_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let event = match message {
        WM_TIMER if wparam.0 == TICK_TIMER => Event::Tick,
        WM_TIMER if wparam.0 == WALK_TIMER => Event::WalkStep,
        WM_INPUT => {
            // Read now: the raw input handle is only valid while this message is handled.
            if watch::is_personal_input(lparam) {
                dispatch(Event::PersonalInput);
            }
            // SAFETY: lets the system clean up the raw input; nothing is held or swallowed.
            return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
        }
        WM_APP_SETTINGS_CHANGED => Event::SettingsChanged,
        WM_APP_PREVIEW => Event::Preview,
        WM_APP_SETTINGS_CLOSED => Event::SettingsClosed,
        // SAFETY: default handling for everything else.
        _ => return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    };
    dispatch(event);
    LRESULT(0)
}

// ---------------------------------------------------------------------------
// The controller

struct Controller {
    /// The hidden top-level window (tray icon and menu owner, session notifications).
    main: HWND,
    /// The message-only window (timers, raw input, settings window notifications).
    msg: HWND,
    /// Shared with the settings window, which writes changes into it. Never
    /// borrowed across a call into another module.
    settings: Rc<RefCell<Settings>>,
    /// `settings.json`'s stamp when MelCat last read or saved it; another one
    /// means someone else wrote the file.
    settings_stamp: Option<(SystemTime, u64)>,
    jiggler: Jiggler,
    tray: Tray,
    /// True while `TRAY_RETRY_TIMER` runs.
    tray_retrying: bool,
    tray_retries: u32,
    /// Raw input was registered (and must be unregistered on quit).
    raw_input: bool,
    settings_window: Option<SettingsWindow>,
    /// The minute the tooltip was last written for (see [`tip_is_stale`]).
    tip_minute: Option<i64>,
}

impl Controller {
    fn handle(&mut self, event: Event) -> Option<Followup> {
        match event {
            Event::Start { first_launch } => self.start(first_launch),
            Event::Tick => self.tick(),
            Event::WalkStep => self.jiggler.on_walk_timer(),
            Event::PersonalInput => self.jiggler.note_personal_input(),
            Event::Tray { event, anchor } => return self.on_tray(event, anchor),
            Event::MenuCommand(id) => match Command::from_id(id) {
                Some(command) => return self.on_command(command),
                None => crate::log_warn!("unknown menu id {id}"),
            },
            Event::TrayRetry => self.on_tray_retry(),
            Event::TaskbarCreated => {
                crate::log_info!("taskbar created; adding the tray icon again");
                if self.tray.readd() {
                    self.tray_added();
                } else {
                    self.schedule_tray_retry();
                }
            }
            Event::RefreshTray => self.tray.refresh(),
            Event::DisplayChanged => {
                self.jiggler.interrupt("display changed");
                // The primary monitor, and so the tray icon's size, may have changed.
                self.tray.refresh();
            }
            Event::SessionChanged => self.jiggler.interrupt("session changed"),
            Event::SettingsChanged => {
                self.save_settings();
                self.tick();
                self.show_status(paths::now());
            }
            Event::Preview => {
                let intensity = self.current().intensity;
                self.jiggler.preview(intensity);
            }
            Event::SettingsClosed => self.forget_settings_window(),
            Event::Notice { title, text } => self.notice(title, &text),
            Event::Close => {
                crate::log_info!("asked to close; quitting");
                // SAFETY: ends the message loop of this (the UI) thread.
                unsafe { PostQuitMessage(0) };
            }
        }
        None
    }

    /// A copy of the current settings (the borrow ends here).
    fn current(&self) -> Settings {
        *self.settings.borrow()
    }

    fn start(&mut self, first_launch: bool) {
        if self.tray.ensure_added() {
            self.tray_added();
        } else {
            self.schedule_tray_retry();
        }
        // SAFETY: a timer on our own window.
        if unsafe { SetTimer(Some(self.msg), TICK_TIMER, TICK_MILLIS, None) } == 0 {
            crate::log_error!("could not start the tick timer: {}", windows::core::Error::from_thread());
        }
        self.tick();
        if first_launch {
            // Written now so the next start is not a first launch any more.
            self.save_settings();
            self.open_settings();
        }
    }

    // --- status -----------------------------------------------------------

    /// The once-a-second check, also run after any settings change.
    fn tick(&mut self) {
        let reloaded = self.reload_if_edited();
        let settings = self.current();
        let changed = self.jiggler.tick(&settings, session_is_away).is_some();
        let now = paths::now();
        if changed || reloaded || tip_is_stale(self.tip_minute, now) {
            self.show_status(now);
        }
    }

    /// Reloads `settings.json` when someone else wrote it since MelCat last
    /// read or saved it. True when the settings were replaced.
    fn reload_if_edited(&mut self) -> bool {
        let path = Settings::path();
        let stamp = Settings::stamp_of(&path);
        if stamp == self.settings_stamp {
            return false;
        }
        // Noted before reading, so a write that lands meanwhile is read next time.
        self.settings_stamp = stamp;
        match Settings::read_from(&path) {
            Ok(Some(settings)) => {
                crate::log_info!("settings reloaded: {settings:?}");
                *self.settings.borrow_mut() = settings;
                self.refresh_settings_window();
                true
            }
            // Removed: the settings in force stay, and the next change writes the file again.
            Ok(None) => false,
            Err(reason) => {
                crate::log_warn!("{reason}; keeping the current settings");
                false
            }
        }
    }

    fn status_line(&self, status: &Status, now: Instant) -> String {
        text::status_line(status, now, &ChronoCalendar::new(chrono::Local), &text::locale_time, &text::locale_date_time)
    }

    /// Shows the jiggler's status in the tray: the artwork and the tooltip.
    fn show_status(&mut self, now: Instant) {
        let status = self.jiggler.status();
        self.tray.set_active(status.is_active());
        self.tray.set_tip(&tooltip(&self.status_line(&status, now)));
        self.tip_minute = Some(minute_of(now));
    }

    fn notice(&self, title: &str, text: &str) {
        self.tray.balloon(title, &sentence(text), Balloon::Warning);
    }

    fn save_settings(&mut self) {
        match self.current().save() {
            // MelCat's own write is not an edit to reload.
            Ok(()) => self.settings_stamp = Settings::stamp_of(&Settings::path()),
            Err(error) => {
                crate::log_error!("could not save the settings: {error}");
                self.notice("Could not save the settings", &save_failure_text(&error));
            }
        }
    }

    // --- tray icon --------------------------------------------------------

    fn tray_added(&mut self) {
        crate::log_info!("tray icon added");
        self.stop_tray_retry();
    }

    fn schedule_tray_retry(&mut self) {
        self.tray_retries = 0;
        self.tray_retrying = true;
        // SAFETY: a timer on our own window; setting it again restarts it.
        if unsafe { SetTimer(Some(self.main), TRAY_RETRY_TIMER, TRAY_RETRY_MILLIS, None) } == 0 {
            crate::log_error!("could not start the tray retry timer: {}", windows::core::Error::from_thread());
        }
    }

    fn stop_tray_retry(&mut self) {
        if self.tray_retrying {
            self.tray_retrying = false;
            // SAFETY: our own timer on our own window.
            unsafe {
                let _ = KillTimer(Some(self.main), TRAY_RETRY_TIMER);
            }
        }
    }

    fn on_tray_retry(&mut self) {
        if !self.tray_retrying {
            return;
        }
        self.tray_retries += 1;
        if self.tray.ensure_added() {
            self.tray_added();
        } else if self.tray_retries >= TRAY_RETRY_LIMIT {
            crate::log_error!("the tray icon could not be added after {TRAY_RETRY_LIMIT} tries; waiting for Explorer");
            self.stop_tray_retry();
        }
    }

    // --- tray menu --------------------------------------------------------

    fn menu_state(&self) -> MenuState {
        let settings = self.current();
        MenuState {
            status_line: self.status_line(&self.jiggler.status(), paths::now()),
            schedule_line: text::schedule_summary(&settings.schedule, text::first_weekday(), &text::locale_time_of_day),
            mode: settings.mode,
            intensity: settings.intensity,
            idle_interval: settings.idle_interval,
            launch_at_login: autostart::is_enabled(),
        }
    }

    fn on_tray(&mut self, event: u32, anchor: PointPx) -> Option<Followup> {
        if !opens_menu(event) || menu_blocked(MENU_OPEN.get(), MENU_CLOSED_AT.get(), Clock::now()) {
            return None;
        }
        // Like the macOS menu (`menuNeedsUpdate`): re-evaluate first, so the
        // status line is current.
        self.tick();
        let at = if anchor == PointPx::default() { pointer::cursor_pos().unwrap_or(anchor) } else { anchor };
        Some(Followup::Menu { owner: self.main, entries: build_menu(&self.menu_state()), at })
    }

    fn on_command(&mut self, command: Command) -> Option<Followup> {
        crate::log_info!("menu: {command:?}");
        match command {
            Command::SetMode(mode) => self.change_settings(|settings| settings.mode = mode),
            Command::SetIntensity(intensity) => self.change_settings(|settings| settings.intensity = intensity),
            Command::SetIdleInterval(seconds) => {
                self.change_settings(|settings| settings.idle_interval = sanitize_interval(f64::from(seconds)))
            }
            Command::OpenSettings => self.open_settings(),
            Command::ToggleLaunchAtLogin => {
                if let Err(error) = autostart::set_enabled(!autostart::is_enabled()) {
                    crate::log_error!("launch at login: {error}");
                    self.notice("Could not change Launch at Login", &error);
                }
                self.refresh_settings_window();
            }
            Command::OpenLog => match crate::util::log::log_file() {
                Some(path) => {
                    crate::log_info!("opening the log file");
                    return Some(Followup::Open { path, failure_title: "Could not open the log file" });
                }
                None => self.notice("Could not open the log file", "LOCALAPPDATA is not set, so there is no log file"),
            },
            // SAFETY: ends the message loop of this (the UI) thread.
            Command::Quit => unsafe { PostQuitMessage(0) },
        }
        None
    }

    /// A change from the tray menu: applied, saved, ticked at once and shown
    /// in an open settings window.
    fn change_settings(&mut self, change: impl FnOnce(&mut Settings)) {
        change(&mut self.settings.borrow_mut());
        self.save_settings();
        self.tick();
        self.refresh_settings_window();
    }

    // --- settings window --------------------------------------------------

    fn open_settings(&mut self) {
        if let Some(window) = &self.settings_window {
            // SAFETY: only asks whether the handle still names a window.
            if unsafe { IsWindow(Some(window.hwnd())) }.as_bool() {
                window.activate();
                return;
            }
            // It closed but its WM_APP_SETTINGS_CLOSED never arrived: open a new one.
            self.forget_settings_window();
        }
        match SettingsWindow::show(Rc::clone(&self.settings), self.msg) {
            Ok(window) => {
                DIALOG.set(window.hwnd());
                self.settings_window = Some(window);
            }
            Err(error) => {
                crate::log_error!("could not open the settings window: {error}");
                self.notice("Could not open the settings", &error_text(&error));
            }
        }
    }

    fn refresh_settings_window(&self) {
        if let Some(window) = &self.settings_window {
            window.refresh();
        }
    }

    /// The window closed itself (`WM_APP_SETTINGS_CLOSED`).
    fn forget_settings_window(&mut self) {
        DIALOG.set(HWND::default());
        self.settings_window = None;
    }

    // --- quitting ---------------------------------------------------------

    /// Lets go of the pointer and removes everything MelCat set up. Used by
    /// Quit (after the message loop) and by `WM_ENDSESSION`.
    fn shutdown(mut self, reason: &str) {
        self.jiggler.interrupt(reason);
        // SAFETY: our own timers on our own windows.
        unsafe {
            let _ = KillTimer(Some(self.msg), TICK_TIMER);
            let _ = KillTimer(Some(self.msg), WALK_TIMER);
            let _ = KillTimer(Some(self.main), TRAY_RETRY_TIMER);
        }
        DIALOG.set(HWND::default());
        if let Some(window) = self.settings_window.take() {
            window.close();
        }
        self.tray.remove();
        if self.raw_input {
            watch::unregister();
        }
        if let Some(mut session) = SESSION.with(|slot| slot.try_borrow_mut().ok().and_then(|mut slot| slot.take())) {
            session.unregister();
        }
        // SAFETY: destroys our own windows on the thread that created them.
        unsafe {
            let _ = DestroyWindow(self.msg);
            let _ = DestroyWindow(self.main);
        }
        crate::log_info!("MelCat stopped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::UI::Shell::{NIN_BALLOONUSERCLICK, NIN_POPUPOPEN};
    use windows::Win32::UI::WindowsAndMessaging::{WM_LBUTTONUP, WM_MOUSEMOVE, WM_RBUTTONUP};

    fn wide_name(name: PCWSTR) -> String {
        // SAFETY: the names are static, null-terminated strings.
        unsafe { name.to_string() }.unwrap_or_default()
    }

    #[test]
    fn window_classes_use_the_melcat_prefix() {
        let (main, msg) = (wide_name(MAIN_CLASS), wide_name(MSG_CLASS));
        assert_eq!(main, "MelCat.Main");
        assert_eq!(msg, "MelCat.Msg");
        assert!(main.starts_with(window::CLASS_PREFIX) && msg.starts_with(window::CLASS_PREFIX));
        assert_eq!(wide_name(MUTEX_NAME), r"Local\MelCat.SingleInstance");
    }

    #[test]
    fn key_select_is_select_plus_the_key_flag() {
        assert_eq!(NIN_SELECT, 0x400);
        assert_eq!(NIN_KEYSELECT, 0x401);
    }

    #[test]
    fn tray_events_decode_the_event_and_a_signed_anchor() {
        // Icon id 1 in the high word of lParam, the event in the low word.
        assert_eq!(decode_tray(WPARAM(0x0400_0780), LPARAM(0x0001_007B)), (WM_CONTEXTMENU, PointPx::new(1920, 1024)));
        assert_eq!(decode_tray(WPARAM(0x0010_FF00), LPARAM(0x0001_0400)), (NIN_SELECT, PointPx::new(-256, 16)));
        // A monitor left of and above the primary one.
        assert_eq!(decode_tray(WPARAM(0xFFF6_F830), LPARAM(0x0001_0401)), (NIN_KEYSELECT, PointPx::new(-2000, -10)));
        // Whatever is above the low word is not part of the event.
        assert_eq!(decode_tray(WPARAM(0), LPARAM(-0x0001_0000 | 0x0400)).0, NIN_SELECT);
    }

    #[test]
    fn only_clicks_and_key_selects_open_the_menu() {
        for event in [WM_CONTEXTMENU, NIN_SELECT, NIN_KEYSELECT] {
            assert!(opens_menu(event), "{event:#x}");
        }
        for event in [WM_MOUSEMOVE, WM_LBUTTONUP, WM_RBUTTONUP, NIN_POPUPOPEN, NIN_BALLOONUSERCLICK, 0] {
            assert!(!opens_menu(event), "{event:#x}");
        }
    }

    #[test]
    fn the_click_that_closed_the_menu_does_not_reopen_it() {
        let closed = Clock::now();
        assert!(!menu_blocked(false, None, closed));
        assert!(menu_blocked(true, None, closed), "never two menus at once");
        assert!(menu_blocked(false, Some(closed), closed + Duration::from_millis(299)));
        assert!(!menu_blocked(false, Some(closed), closed + MENU_REOPEN_GUARD));
        assert!(!menu_blocked(false, Some(closed), closed + Duration::from_secs(5)));
        // A clock read before the close time counts as "just closed".
        assert!(menu_blocked(false, Some(closed + Duration::from_millis(10)), closed));
    }

    #[test]
    fn the_tooltip_names_melcat_like_the_macos_app() {
        assert_eq!(tooltip("Mel is playing \u{b7} until 18:00"), "MelCat: Mel is playing \u{b7} until 18:00");
        assert_eq!(tooltip("MelCat is off"), "MelCat: MelCat is off");
        // The longest status line still fits a tooltip (127 characters).
        let long = tooltip("Mel is napping \u{b7} back Wednesday, September 30, 2026 12:00 PM");
        assert!(long.encode_utf16().count() <= 127, "{long}");
    }

    #[test]
    fn the_tooltip_goes_stale_when_the_minute_changes() {
        assert_eq!(minute_of(Instant(0)), 0);
        assert_eq!(minute_of(Instant(59)), 0);
        assert_eq!(minute_of(Instant(60)), 1);
        assert_eq!(minute_of(Instant(-1)), -1);
        assert!(tip_is_stale(None, Instant(1_000)));
        assert!(!tip_is_stale(Some(minute_of(Instant(1_020))), Instant(1_079)));
        assert!(tip_is_stale(Some(minute_of(Instant(1_020))), Instant(1_080)));
    }

    #[test]
    fn notices_are_whole_sentences() {
        // io::Error's own text is "Access is denied. (os error 5)".
        let denied = save_failure_text(&std::io::Error::from_raw_os_error(5));
        assert!(denied.ends_with(" (error 5). The change applies until MelCat quits."), "{denied}");
        let bare = save_failure_text(&std::io::Error::other("disk full"));
        assert_eq!(bare, "Disk full. The change applies until MelCat quits.");
        for text in [denied, bare] {
            assert!(!text.contains(".."), "{text}");
            assert!(!text.contains(". ("), "{text}");
            // A balloon holds 255 characters.
            assert!(text.chars().count() <= 255, "{text}");
        }
    }

    #[test]
    fn windows_errors_read_as_sentences() {
        let described = windows::core::Error::new(windows::Win32::Foundation::E_FAIL, "The window class is missing.");
        assert_eq!(error_text(&described), "The window class is missing.");
        let bare = windows::core::Error::new(windows::core::HRESULT(0x8899_0001_u32 as i32), "");
        let text = error_text(&bare);
        assert!(text.ends_with('.') && !text.contains(".."), "{text}");
    }
}
