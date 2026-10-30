//! Well-known folders and the clocks.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use melcat_core::Instant;
use windows::Win32::System::SystemInformation::GetLocalTime;

/// `%APPDATA%\MelCat`, where settings live.
pub fn app_data_dir() -> PathBuf {
    std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("MelCat")
}

/// The current instant, for the schedule.
pub fn now() -> Instant {
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    Instant(seconds)
}

/// The local wall-clock time, for log lines.
pub struct LocalTime {
    pub year: u16,
    pub month: u16,
    pub day: u16,
    pub hour: u16,
    pub minute: u16,
    pub second: u16,
    pub millis: u16,
}

/// The local wall-clock time.
pub fn local_time() -> LocalTime {
    // SAFETY: GetLocalTime only fills a SYSTEMTIME.
    let t = unsafe { GetLocalTime() };
    LocalTime {
        year: t.wYear,
        month: t.wMonth,
        day: t.wDay,
        hour: t.wHour,
        minute: t.wMinute,
        second: t.wSecond,
        millis: t.wMilliseconds,
    }
}
