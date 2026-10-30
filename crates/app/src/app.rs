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
//!    `Rc<RefCell<Settings>>`), adds the tray icon (retrying on
//!    `TRAY_RETRY_TIMER` every 2 s, up to 30 times, if Explorer is not ready;
//!    logs `tray icon added` once it is there), starts `TICK_TIMER`, and runs
//!    one tick at once. If `settings.json` did not exist at start (first
//!    launch), it opens the settings window, like the macOS app.
//! 6. Runs the message loop, giving the settings window's messages to
//!    `IsDialogMessageW` first, until Quit. Returns the exit code.
//!
//! Messages:
//! - `TICK_TIMER`: `jiggler.tick(&settings, session.is_away())`; on a status
//!   change, update the tray: `tray.set_active(status.is_active())` and the
//!   tooltip `MelCat: <status line>` (`ui::text::status_line`). The tooltip
//!   is also refreshed once a minute, so "tomorrow" and "today" stay right.
//! - `WALK_TIMER`: `jiggler.on_walk_timer()`.
//! - `WM_INPUT`: `if watch::is_personal_input(lparam) { jiggler.note_personal_input() }`,
//!   then `DefWindowProcW`.
//! - `WM_APP_TRAY` (`NOTIFYICON_VERSION_4`: event in `LOWORD(lParam)`, anchor
//!   in `wParam`): `WM_CONTEXTMENU`, `NIN_SELECT` and `NIN_KEYSELECT` open the
//!   tray menu (`ui::menu_model::build_menu` + `ui::menu::show_popup`) at the
//!   anchor. A click within 300 ms of the menu closing is the click that
//!   closed it and does not reopen it.
//! - Menu commands: `SetMode`, `SetIntensity`, `SetIdleInterval` change the
//!   settings, save them (`Settings::save`; a failure is a warning balloon),
//!   tick at once and refresh an open settings window. `OpenSettings` opens or
//!   activates the settings window. `ToggleLaunchAtLogin` calls
//!   `ui::autostart::set_enabled` (an error is a warning balloon).
//!   `OpenLog` opens `util::log::log_file()` with `ShellExecuteW` "open".
//!   `Quit` leaves the loop.
//! - `WM_APP_SETTINGS_CHANGED`: save, tick at once, update the tray.
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

/// Runs MelCat until the user quits. Returns the process exit code.
pub fn run() -> i32 {
    todo!("CONTRACT: see the module docs")
}
