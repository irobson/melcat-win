//! Whether anyone can be at this session's screen.
//!
//! CONTRACT (owned by the jiggle agent). MelCat stays still while the screen
//! is locked, the screen saver runs, the displays are off, or this session is
//! not the one in front of the user (switched away, disconnected), like the
//! macOS app. A pointer move then would only wake the displays or dismiss the
//! screen saver.
//!
//! - Lock: `WTSRegisterSessionNotification(main, NOTIFY_FOR_THIS_SESSION)`;
//!   `WM_WTSSESSION_CHANGE` with `WTS_SESSION_LOCK` / `WTS_SESSION_UNLOCK`.
//!   The initial state comes from `WTSQuerySessionInformationW(WTSSessionInfoEx)`
//!   (`WTS_SESSIONSTATE_LOCK`).
//! - Connection: `WTSQuerySessionInformationW(WTSConnectState)` must be
//!   `WTSActive` (read on demand in [`SessionMonitor::is_away`]).
//! - Screen saver: `SystemParametersInfoW(SPI_GETSCREENSAVERRUNNING)`, read on demand.
//! - Displays: `RegisterPowerSettingNotification(main, GUID_CONSOLE_DISPLAY_STATE,
//!   DEVICE_NOTIFY_WINDOW_HANDLE)`; `WM_POWERBROADCAST` / `PBT_POWERSETTINGCHANGE`
//!   carries 0 (off), 1 (on) or 2 (dimmed, counts as on). `PBT_APMSUSPEND`
//!   and `PBT_APMRESUMEAUTOMATIC` also report a change, so a stroll in
//!   progress is dropped around sleep.
//!
//! Both notifications go to `MelCat.Main` (a top-level window).

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};

pub(crate) struct SessionMonitor {
    main: HWND,
}

impl SessionMonitor {
    /// Starts watching for `main`. Failures are logged; the monitor then
    /// reports only what it can read on demand.
    pub(crate) fn register(main: HWND) -> SessionMonitor {
        todo!("CONTRACT: see the module docs")
    }

    /// `WM_WTSSESSION_CHANGE`. Returns true when the away state may have changed.
    pub(crate) fn on_session_change(&mut self, wparam: WPARAM) -> bool {
        todo!("CONTRACT: see the module docs")
    }

    /// `WM_POWERBROADCAST`. Returns true when the away state may have
    /// changed, or the system is about to sleep or just woke.
    pub(crate) fn on_power_broadcast(&mut self, wparam: WPARAM, lparam: LPARAM) -> bool {
        todo!("CONTRACT: see the module docs")
    }

    /// True when nobody can be at this session's screen right now.
    pub(crate) fn is_away(&self) -> bool {
        todo!("CONTRACT: see the module docs")
    }

    /// Stops both notifications (also on drop).
    pub(crate) fn unregister(&mut self) {
        todo!("CONTRACT: see the module docs")
    }
}
