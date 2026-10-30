//! The words the tray uses for MelCat's state and schedule, ported from the
//! macOS app's `StatusText`. Times follow the user's locale (18:00 or
//! 6:00 PM); everything else is English.
//!
//! CONTRACT (owned by the tray agent). The pure functions take the clock
//! formatter as a parameter so they are unit tested on any locale; the
//! `locale_*` functions are the Win32 side. Exact strings, as on macOS:
//!
//! | Status                         | `status_line`                              |
//! |--------------------------------|--------------------------------------------|
//! | `Status::Off`                  | `MelCat is off`                            |
//! | `Status::Napping{until: Some}` | `Mel is napping · back <moment>`           |
//! | `Status::Napping{until: None}` | `Mel is napping · no days selected`        |
//! | `Status::Active{until: Some}`  | `Mel is playing · until <moment>`          |
//! | `Status::Active{until: None}`  | `Mel is playing`                           |
//!
//! `<moment>` ([`moment`]): `18:00` today, `tomorrow 09:00`, `Mon 09:00`
//! two to six days ahead (`RelativeDay`), otherwise a short date and time
//! (`locale_date_time`). The separator is U+00B7 MIDDLE DOT with spaces.
//!
//! [`schedule_summary`]: `Mon–Fri, 09:00–18:00`; `Weekends, all day` when
//! start equals end; just `No days` when no day is selected. Days come from
//! `Days::describe(first_weekday)`; the dash between times is U+2013.
//!
//! [`intensity_title`]: `Gentle · up to 5 pt` (title, middle dot, radius).

use melcat_core::{Calendar, Instant, Intensity, Schedule, TimeOfDay, Weekday};

use crate::jiggle::Status;

/// U+2026, used when a Win32 buffer is too small for the text.
pub(crate) const ELLIPSIS: char = '\u{2026}';

/// The status line at the top of the tray menu (also the tooltip, after "MelCat: ").
/// `clock` formats an instant's time of day (`locale_time`); `date_time`
/// formats a far-away instant (`locale_date_time`).
pub(crate) fn status_line(
    status: &Status,
    now: Instant,
    calendar: &impl Calendar,
    clock: &dyn Fn(Instant) -> String,
    date_time: &dyn Fn(Instant) -> String,
) -> String {
    todo!("CONTRACT: see the table in the module docs")
}

/// "18:00" today, "tomorrow 09:00", "Mon 09:00" this week, a date beyond.
pub(crate) fn moment(
    date: Instant,
    now: Instant,
    calendar: &impl Calendar,
    clock: &dyn Fn(Instant) -> String,
    date_time: &dyn Fn(Instant) -> String,
) -> String {
    todo!("CONTRACT: see the module docs")
}

/// "Mon–Fri, 09:00–18:00", "Weekends, all day", "No days".
/// `time` formats a wall-clock time (`locale_time_of_day`).
pub(crate) fn schedule_summary(schedule: &Schedule, first_weekday: Weekday, time: &dyn Fn(TimeOfDay) -> String) -> String {
    todo!("CONTRACT: see the module docs")
}

/// "Gentle · up to 5 pt".
pub(crate) fn intensity_title(intensity: Intensity) -> String {
    todo!("CONTRACT: see the module docs")
}

/// An instant's local time of day in the user's short time format, without
/// seconds (`GetTimeFormatEx` with `TIME_NOSECONDS`): "18:00" or "6:00 PM".
pub(crate) fn locale_time(instant: Instant) -> String {
    todo!("CONTRACT: Win32, user locale")
}

/// A wall-clock time in the user's short time format ("09:00", "9:00 AM").
pub(crate) fn locale_time_of_day(time: TimeOfDay) -> String {
    todo!("CONTRACT: Win32, user locale")
}

/// A short date and time in the user's format, for moments a week or more away.
pub(crate) fn locale_date_time(instant: Instant) -> String {
    todo!("CONTRACT: Win32, user locale")
}

/// The user's first day of the week (`LOCALE_IFIRSTDAYOFWEEK`: 0 = Monday ...
/// 6 = Sunday), Sunday if it cannot be read.
pub(crate) fn first_weekday() -> Weekday {
    todo!("CONTRACT: Win32, user locale")
}

/// A null-terminated UTF-16 copy of `s` for a fixed-size Win32 buffer,
/// truncated (with an ellipsis, never splitting a surrogate pair) if needed.
pub(crate) fn fixed_wide<const N: usize>(s: &str) -> [u16; N] {
    let mut out = [0u16; N];
    if N == 0 {
        return out;
    }
    let units: Vec<u16> = s.encode_utf16().collect();
    let capacity = N - 1;
    if units.len() <= capacity {
        out[..units.len()].copy_from_slice(&units);
        return out;
    }
    if capacity == 0 {
        return out;
    }
    let mut keep = capacity - 1;
    // Do not end on a high surrogate.
    if keep > 0 && (0xD800..0xDC00).contains(&units[keep - 1]) {
        keep -= 1;
    }
    out[..keep].copy_from_slice(&units[..keep]);
    out[keep] = ELLIPSIS as u16;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_wide_copies_and_terminates() {
        let buf: [u16; 8] = fixed_wide("MelCat");
        assert_eq!(&buf[..7], &[77, 101, 108, 67, 97, 116, 0]);
        let exact: [u16; 4] = fixed_wide("abc");
        assert_eq!(exact, [97, 98, 99, 0]);
    }

    #[test]
    fn fixed_wide_truncates_with_an_ellipsis() {
        let buf: [u16; 5] = fixed_wide("abcdefgh");
        assert_eq!(buf, [97, 98, 99, ELLIPSIS as u16, 0]);
        // "ab😀cd": the emoji is a surrogate pair at units 2-3; it must not be split.
        let emoji: [u16; 5] = fixed_wide("ab\u{1F600}cd");
        assert_eq!(emoji, [97, 98, ELLIPSIS as u16, 0, 0]);
        let tiny: [u16; 1] = fixed_wide("abc");
        assert_eq!(tiny, [0]);
    }
}
