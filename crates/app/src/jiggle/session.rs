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
//! - Displays: `RegisterPowerSettingNotification(main, GUID_SESSION_DISPLAY_STATUS,
//!   DEVICE_NOTIFY_WINDOW_HANDLE)`: the display of this session, which is
//!   what an app in an interactive session should watch. A Remote Desktop or
//!   VM session stays "on" while the physical console's monitor sleeps
//!   (`GUID_CONSOLE_DISPLAY_STATE`, which MelCat does not use). `WM_POWERBROADCAST`
//!   / `PBT_POWERSETTINGCHANGE` carries 0 (off), 1 (on) or 2 (dimmed, counts
//!   as on). `PBT_APMSUSPEND` and `PBT_APMRESUMEAUTOMATIC` also report a
//!   change, so a stroll in progress is dropped around sleep.
//!
//! Both notifications go to `MelCat.Main` (a top-level window).
//!
//! `MelCat.Main` also receives other processes' messages, so the pointer in a
//! `PBT_POWERSETTINGCHANGE` is checked before it is read: it must point to
//! committed, readable memory of this process holding the display state's
//! GUID and a 4-byte value. Anything else is ignored. When the session
//! notification could not be registered, the lock state is read on demand
//! instead, so a failed registration never leaves MelCat thinking the screen
//! is unlocked forever.

use std::ffi::c_void;
use std::mem::{offset_of, size_of};

use windows::core::{BOOL, GUID, PWSTR};
use windows::Win32::Foundation::{HANDLE, HWND, LPARAM, WPARAM};
use windows::Win32::System::Memory::{
    VirtualQuery, MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE,
    PAGE_EXECUTE_WRITECOPY, PAGE_GUARD, PAGE_READONLY, PAGE_READWRITE, PAGE_WRITECOPY,
};
use windows::Win32::System::Power::{
    RegisterPowerSettingNotification, UnregisterPowerSettingNotification, HPOWERNOTIFY, POWERBROADCAST_SETTING,
};
use windows::Win32::System::RemoteDesktop::{
    WTSActive, WTSConnectState, WTSFreeMemory, WTSQuerySessionInformationW, WTSRegisterSessionNotification,
    WTSSessionInfoEx, WTSUnRegisterSessionNotification, NOTIFY_FOR_THIS_SESSION, WTSINFOEXW, WTS_CONNECTSTATE_CLASS,
    WTS_CURRENT_SESSION, WTS_INFO_CLASS, WTS_SESSIONSTATE_LOCK, WTS_SESSIONSTATE_UNLOCK,
};
use windows::Win32::System::SystemServices::GUID_SESSION_DISPLAY_STATUS;
use windows::Win32::UI::WindowsAndMessaging::{
    SystemParametersInfoW, DEVICE_NOTIFY_WINDOW_HANDLE, PBT_APMRESUMEAUTOMATIC, PBT_APMSUSPEND, PBT_POWERSETTINGCHANGE,
    SPI_GETSCREENSAVERRUNNING, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, WTS_SESSION_LOCK, WTS_SESSION_UNLOCK,
};

/// Bytes of a `POWERBROADCAST_SETTING` carrying one `DWORD`: the GUID, the
/// data length and the value.
const DISPLAY_SETTING_BYTES: usize = offset_of!(POWERBROADCAST_SETTING, Data) + size_of::<u32>();

/// The power setting MelCat watches for the displays: this session's.
const DISPLAY_SETTING: GUID = GUID_SESSION_DISPLAY_STATUS;

/// `GUID_SESSION_DISPLAY_STATUS` values (a `MONITOR_DISPLAY_STATE`).
const DISPLAY_OFF: u32 = 0;
const DISPLAY_ON: u32 = 1;
const DISPLAY_DIMMED: u32 = 2;

pub(crate) struct SessionMonitor {
    main: HWND,
    /// `WTSRegisterSessionNotification` succeeded and is still in force.
    session_registered: bool,
    power: Option<HPOWERNOTIFY>,
    /// From the lock and unlock notifications (and the initial query).
    locked: bool,
    /// From the session display state notifications.
    displays_off: bool,
}

impl SessionMonitor {
    /// Starts watching for `main`. Failures are logged; the monitor then
    /// reports only what it can read on demand.
    pub(crate) fn register(main: HWND) -> SessionMonitor {
        // SAFETY: `main` is a window of this thread; the registration ends in `unregister`.
        let session_registered = match unsafe { WTSRegisterSessionNotification(main, NOTIFY_FOR_THIS_SESSION) } {
            Ok(()) => true,
            Err(error) => {
                crate::log_warn!("could not watch the session lock: {error}; reading it on demand");
                false
            }
        };
        // SAFETY: as above; the GUID is a static. Windows answers at once with
        // a notification carrying the current display state.
        let power = match unsafe {
            RegisterPowerSettingNotification(HANDLE(main.0), &DISPLAY_SETTING, DEVICE_NOTIFY_WINDOW_HANDLE)
        } {
            Ok(handle) => Some(handle),
            Err(error) => {
                crate::log_warn!("could not watch the display state: {error}");
                None
            }
        };
        SessionMonitor { main, session_registered, power, locked: query_locked().unwrap_or(false), displays_off: false }
    }

    /// `WM_WTSSESSION_CHANGE`. Returns true when the away state may have changed.
    pub(crate) fn on_session_change(&mut self, wparam: WPARAM) -> bool {
        // Lock and unlock say it outright. Every other event (console or
        // remote connect and disconnect, logon, logoff...) can change the
        // connection state, which `is_away` reads on demand.
        if let Some(locked) = lock_from_event(wparam.0 as u32) {
            self.locked = locked;
        }
        true
    }

    /// `WM_POWERBROADCAST`. Returns true when the away state may have
    /// changed, or the system is about to sleep or just woke.
    pub(crate) fn on_power_broadcast(&mut self, wparam: WPARAM, lparam: LPARAM) -> bool {
        match wparam.0 as u32 {
            PBT_POWERSETTINGCHANGE => match read_display_setting(lparam) {
                Some(off) => {
                    let changed = off != self.displays_off;
                    self.displays_off = off;
                    changed
                }
                None => false,
            },
            // The displays report their own state when they come back, so it
            // is not reset here: an unattended wake (a scheduled task) keeps
            // them off, and a move then would keep the computer awake.
            PBT_APMSUSPEND | PBT_APMRESUMEAUTOMATIC => true,
            _ => false,
        }
    }

    /// True when nobody can be at this session's screen right now.
    pub(crate) fn is_away(&self) -> bool {
        let locked = if self.session_registered { self.locked } else { query_locked().unwrap_or(false) };
        is_away_from(locked, self.displays_off, is_screen_saver_running(), query_connect_state())
    }

    /// Stops both notifications (also on drop).
    pub(crate) fn unregister(&mut self) {
        if self.session_registered {
            self.session_registered = false;
            // SAFETY: registered by `register` for this same window.
            if let Err(error) = unsafe { WTSUnRegisterSessionNotification(self.main) } {
                crate::log_warn!("could not stop watching the session lock: {error}");
            }
        }
        if let Some(handle) = self.power.take() {
            // SAFETY: a handle returned by RegisterPowerSettingNotification, unregistered once.
            if let Err(error) = unsafe { UnregisterPowerSettingNotification(handle) } {
                crate::log_warn!("could not stop watching the display state: {error}");
            }
        }
    }
}

impl Drop for SessionMonitor {
    fn drop(&mut self) {
        self.unregister();
    }
}

/// The away decision: locked, displays off, screen saver running, or a
/// session that is not the active one. An unknown connection state (`None`,
/// the query failed) counts as present, like the macOS app when a session
/// key is missing.
fn is_away_from(locked: bool, displays_off: bool, screen_saver_running: bool, connect_state: Option<i32>) -> bool {
    locked || displays_off || screen_saver_running || connect_state.is_some_and(|state| state != WTSActive.0)
}

/// What a `WM_WTSSESSION_CHANGE` event says about the lock: `Some(true)`
/// locked, `Some(false)` unlocked, `None` nothing.
fn lock_from_event(event: u32) -> Option<bool> {
    match event {
        WTS_SESSION_LOCK => Some(true),
        WTS_SESSION_UNLOCK => Some(false),
        _ => None,
    }
}

/// `WTSINFOEX_LEVEL1_W.SessionFlags` as a lock state; `None` for
/// `WTS_SESSIONSTATE_UNKNOWN` or anything else. (Windows 7 swapped the two
/// values; MelCat runs on Windows 10 and 11, where they are as documented.)
fn lock_from_flags(flags: i32) -> Option<bool> {
    match flags as u32 {
        WTS_SESSIONSTATE_LOCK => Some(true),
        WTS_SESSIONSTATE_UNLOCK => Some(false),
        _ => None,
    }
}

/// What a `PBT_POWERSETTINGCHANGE` payload (the `POWERBROADCAST_SETTING`
/// bytes) says about the displays: `Some(true)` off, `Some(false)` on or
/// dimmed, `None` for another setting, a malformed payload or an unknown value.
fn displays_off_from(payload: &[u8]) -> Option<bool> {
    let bytes = |at: usize, len: usize| payload.get(at..at + len);
    let dword = |at: usize| Some(u32::from_le_bytes(bytes(at, 4)?.try_into().ok()?));
    let word = |at: usize| Some(u16::from_le_bytes(bytes(at, 2)?.try_into().ok()?));
    // A GUID in memory: Data1, Data2 and Data3 little-endian, then 8 bytes.
    let guid = GUID::from_values(dword(0)?, word(4)?, word(6)?, bytes(8, 8)?.try_into().ok()?);
    let length = dword(offset_of!(POWERBROADCAST_SETTING, DataLength))?;
    if guid != DISPLAY_SETTING || length as usize != size_of::<u32>() {
        return None;
    }
    match dword(offset_of!(POWERBROADCAST_SETTING, Data))? {
        DISPLAY_OFF => Some(true),
        DISPLAY_ON | DISPLAY_DIMMED => Some(false),
        _ => None,
    }
}

/// Reads the display state from a `PBT_POWERSETTINGCHANGE`'s `lParam`, after
/// checking that it points to readable memory of this process.
fn read_display_setting(lparam: LPARAM) -> Option<bool> {
    let address = lparam.0 as *const u8;
    if !is_readable(address, DISPLAY_SETTING_BYTES) {
        return None;
    }
    // SAFETY: `DISPLAY_SETTING_BYTES` bytes at `address` are committed,
    // readable memory of this process (checked just above), and no other
    // thread exists that could release them in between. Only bytes are read,
    // so alignment does not matter.
    let payload = unsafe { std::slice::from_raw_parts(address, DISPLAY_SETTING_BYTES) };
    displays_off_from(payload)
}

/// Whether `len` bytes at `address` are committed, readable memory of this process.
fn is_readable(address: *const u8, len: usize) -> bool {
    if address.is_null() {
        return false;
    }
    let mut info = MEMORY_BASIC_INFORMATION::default();
    // SAFETY: VirtualQuery only describes the address space; it reads nothing
    // at `address` and writes only `info`, whose size is passed.
    let written =
        unsafe { VirtualQuery(Some(address as *const c_void), &mut info, size_of::<MEMORY_BASIC_INFORMATION>()) };
    written != 0
        && region_allows_read(
            info.State.0,
            info.Protect.0,
            info.BaseAddress as usize,
            info.RegionSize,
            address as usize,
            len,
        )
}

/// The decision behind [`is_readable`]: the region is committed, readable and
/// not a guard page, and `[address, address + len)` lies inside it.
fn region_allows_read(state: u32, protect: u32, base: usize, size: usize, address: usize, len: usize) -> bool {
    let readable = PAGE_READONLY.0
        | PAGE_READWRITE.0
        | PAGE_WRITECOPY.0
        | PAGE_EXECUTE_READ.0
        | PAGE_EXECUTE_READWRITE.0
        | PAGE_EXECUTE_WRITECOPY.0;
    let inside = match (address.checked_add(len), base.checked_add(size)) {
        (Some(end), Some(region_end)) => address >= base && end <= region_end,
        _ => false,
    };
    state == MEM_COMMIT.0 && protect & PAGE_GUARD.0 == 0 && protect & readable != 0 && inside
}

/// Whether a screen saver is running now; false if it cannot be read.
fn is_screen_saver_running() -> bool {
    let mut running = BOOL(0);
    // SAFETY: SPI_GETSCREENSAVERRUNNING writes one BOOL to `running`.
    let read = unsafe {
        SystemParametersInfoW(
            SPI_GETSCREENSAVERRUNNING,
            0,
            Some(&mut running as *mut BOOL as *mut c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    read.is_ok() && running.as_bool()
}

/// This session's lock state from `WTSSessionInfoEx`, if Windows knows it.
fn query_locked() -> Option<bool> {
    let read = |info: &WTSINFOEXW| {
        // SAFETY: level 1 is the only level, and it is checked before that
        // view of the union is read; its fields are plain integers.
        (info.Level == 1).then(|| lock_from_flags(unsafe { info.Data.WTSInfoExLevel1.SessionFlags })).flatten()
    };
    // SAFETY: WTSSessionInfoEx answers with a WTSINFOEXW.
    unsafe { query_session(WTSSessionInfoEx, read) }
}

/// This session's connection state (`WTS_CONNECTSTATE_CLASS`), if it can be read.
fn query_connect_state() -> Option<i32> {
    // SAFETY: WTSConnectState answers with a WTS_CONNECTSTATE_CLASS.
    unsafe { query_session(WTSConnectState, |state: &WTS_CONNECTSTATE_CLASS| Some(state.0)) }
}

/// Asks the Remote Desktop Services for `class` about this session, reads
/// the answer as a `T` with `read` and frees it. `None` when the call fails
/// or the answer is smaller than a `T`.
///
/// # Safety
///
/// `T` must be the plain-data structure Windows returns for `class`.
unsafe fn query_session<T, R>(class: WTS_INFO_CLASS, read: impl FnOnce(&T) -> Option<R>) -> Option<R> {
    let mut buffer = PWSTR::null();
    let mut bytes = 0u32;
    // SAFETY: valid out parameters; on success Windows allocates `buffer`,
    // which is freed below with WTSFreeMemory.
    unsafe { WTSQuerySessionInformationW(None, WTS_CURRENT_SESSION, class, &mut buffer, &mut bytes) }.ok()?;
    if buffer.is_null() {
        return None;
    }
    let answer = if bytes as usize >= size_of::<T>() {
        // SAFETY: Windows wrote at least `size_of::<T>()` bytes of the
        // structure `class` returns, which the caller promises is `T`, in a
        // buffer it allocated (aligned for any structure).
        read(unsafe { &*(buffer.0 as *const T) })
    } else {
        None
    };
    // SAFETY: `buffer` came from WTSQuerySessionInformationW and is freed once.
    unsafe { WTSFreeMemory(buffer.0 as *mut c_void) };
    answer
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `POWERBROADCAST_SETTING` for `guid` with one `DWORD`, as Windows lays it out.
    fn payload(guid: GUID, length: u32, value: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&guid.data1.to_le_bytes());
        bytes.extend_from_slice(&guid.data2.to_le_bytes());
        bytes.extend_from_slice(&guid.data3.to_le_bytes());
        bytes.extend_from_slice(&guid.data4);
        bytes.extend_from_slice(&length.to_le_bytes());
        bytes.extend_from_slice(&value.to_le_bytes());
        bytes
    }

    #[test]
    fn the_payload_layout_matches_the_struct() {
        assert_eq!(DISPLAY_SETTING_BYTES, 24);
        assert_eq!(payload(DISPLAY_SETTING, 4, 0).len(), DISPLAY_SETTING_BYTES);
        let setting = POWERBROADCAST_SETTING { PowerSetting: DISPLAY_SETTING, DataLength: 4, Data: [7] };
        // SAFETY: reading the bytes of a plain struct, within its size.
        let raw = unsafe {
            std::slice::from_raw_parts(
                &setting as *const POWERBROADCAST_SETTING as *const u8,
                offset_of!(POWERBROADCAST_SETTING, Data),
            )
        };
        assert_eq!(raw, &payload(DISPLAY_SETTING, 4, 0)[..20]);
    }

    #[test]
    fn display_states_decode() {
        assert_eq!(displays_off_from(&payload(DISPLAY_SETTING, 4, 0)), Some(true));
        assert_eq!(displays_off_from(&payload(DISPLAY_SETTING, 4, 1)), Some(false));
        assert_eq!(displays_off_from(&payload(DISPLAY_SETTING, 4, 2)), Some(false));
        assert_eq!(displays_off_from(&payload(DISPLAY_SETTING, 4, 3)), None);
    }

    /// The session's display, never the physical console's: a Remote
    /// Desktop or VM session must not stop when the console monitor sleeps.
    #[test]
    fn the_session_display_is_watched_not_the_console() {
        // GUID_SESSION_DISPLAY_STATUS, {2B84C20E-AD23-4DDF-93DB-05FFBD7EFCA5}.
        assert_eq!(DISPLAY_SETTING, GUID::from_u128(0x2b84c20e_ad23_4ddf_93db_05ffbd7efca5));
        let console = GUID::from_u128(0x6fe69556_704a_47a0_8f24_c28d936fda47); // GUID_CONSOLE_DISPLAY_STATE
        for value in [0, 1, 2] {
            assert_eq!(displays_off_from(&payload(console, 4, value)), None, "{value}");
        }
        assert_eq!(displays_off_from(&payload(DISPLAY_SETTING, 4, 0)), Some(true));
    }

    #[test]
    fn other_settings_and_malformed_payloads_are_ignored() {
        let other = GUID::from_u128(0x02731015_4510_4526_99e6_e5a17ebd1aea); // GUID_MONITOR_POWER_ON
        assert_eq!(displays_off_from(&payload(other, 4, 0)), None);
        assert_eq!(displays_off_from(&payload(DISPLAY_SETTING, 1, 0)), None);
        assert_eq!(displays_off_from(&payload(DISPLAY_SETTING, 8, 0)), None);
        assert_eq!(displays_off_from(&payload(DISPLAY_SETTING, 4, 0)[..23]), None);
        assert_eq!(displays_off_from(&[]), None);
    }

    #[test]
    fn away_when_anything_hides_the_screen() {
        let active = Some(WTSActive.0);
        assert!(!is_away_from(false, false, false, active));
        assert!(is_away_from(true, false, false, active));
        assert!(is_away_from(false, true, false, active));
        assert!(is_away_from(false, false, true, active));
        // WTSConnected, WTSDisconnected: switched away or disconnected.
        assert!(is_away_from(false, false, false, Some(1)));
        assert!(is_away_from(false, false, false, Some(4)));
        // Unknown connection state counts as present.
        assert!(!is_away_from(false, false, false, None));
    }

    #[test]
    fn lock_events_and_flags_decode() {
        assert_eq!(lock_from_event(WTS_SESSION_LOCK), Some(true));
        assert_eq!(lock_from_event(WTS_SESSION_UNLOCK), Some(false));
        for other in [1, 2, 3, 4, 5, 6, 9, 0xB, 0] {
            assert_eq!(lock_from_event(other), None, "{other}");
        }
        assert_eq!(lock_from_flags(0), Some(true));
        assert_eq!(lock_from_flags(1), Some(false));
        assert_eq!(lock_from_flags(-1), None); // WTS_SESSIONSTATE_UNKNOWN
    }

    #[test]
    fn readable_regions() {
        let (base, size) = (0x1_0000, 0x2000);
        let rw = PAGE_READWRITE.0;
        assert!(region_allows_read(MEM_COMMIT.0, rw, base, size, base, 24));
        assert!(region_allows_read(MEM_COMMIT.0, PAGE_READONLY.0, base, size, base + size - 24, 24));
        assert!(!region_allows_read(MEM_COMMIT.0, rw, base, size, base + size - 23, 24));
        assert!(!region_allows_read(MEM_COMMIT.0, rw, base, size, base - 1, 24));
        assert!(!region_allows_read(0x2000 /* MEM_RESERVE */, rw, base, size, base, 24));
        assert!(!region_allows_read(MEM_COMMIT.0, 0x01 /* PAGE_NOACCESS */, base, size, base, 24));
        assert!(!region_allows_read(MEM_COMMIT.0, rw | PAGE_GUARD.0, base, size, base, 24));
        assert!(!region_allows_read(MEM_COMMIT.0, 0x10 /* PAGE_EXECUTE */, base, size, base, 24));
        assert!(!region_allows_read(MEM_COMMIT.0, rw, usize::MAX - 8, 0x2000, usize::MAX - 4, 24));
    }

    #[test]
    fn this_process_memory_is_readable_and_null_is_not() {
        let bytes = payload(DISPLAY_SETTING, 4, 0);
        assert!(is_readable(bytes.as_ptr(), bytes.len()));
        assert!(!is_readable(std::ptr::null(), 24));
        assert_eq!(read_display_setting(LPARAM(bytes.as_ptr() as isize)), Some(true));
        assert_eq!(read_display_setting(LPARAM(0)), None);
        assert_eq!(read_display_setting(LPARAM(16)), None);
    }
}
