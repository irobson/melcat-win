//! The words the tray uses for MelCat's state and schedule, ported from the
//! macOS app's `StatusText`. Times follow the user's locale (18:00 or
//! 6:00 PM); everything else is English.
//!
//! The pure functions take the clock formatter as a parameter so they are
//! unit tested on any locale; the `locale_*` functions are the Win32 side.
//! Exact strings, as on macOS:
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
//!
//! The locale functions read the user's own formats (`LOCALE_NAME_USER_DEFAULT`,
//! so the Region settings' overrides count). Times use the short time format
//! (`LOCALE_SSHORTTIME`, [`short_time_format`], the format the settings
//! window's pickers show), passed to `GetTimeFormatEx` without seconds; it is
//! read on every call, so a change in the Region settings applies at once.
//! Only when it cannot be read is the format left to Windows (a null format).
//! Instants are turned into a wall
//! clock with `ChronoCalendar<chrono::Local>`, the same calendar the jiggler
//! uses for the schedule, so "18:00" in the status line is the 18:00 the
//! schedule meant. If Windows cannot format a value, a 24-hour `HH:MM` (and
//! an ISO date) stands in: a plain time beats an empty menu line.

use melcat_core::{Calendar, ChronoCalendar, Instant, Intensity, LocalDate, RelativeDay, Schedule, TimeOfDay, Weekday};
use windows::core::PCWSTR;
use windows::Win32::Foundation::SYSTEMTIME;
use windows::Win32::Globalization::{
    GetDateFormatEx, GetLocaleInfoEx, GetTimeFormatEx, DATE_SHORTDATE, LOCALE_IFIRSTDAYOFWEEK, LOCALE_SSHORTTIME,
    TIME_NOSECONDS,
};

use crate::jiggle::Status;
use crate::util::wide::{from_wide, wide};

/// U+2026, used when a Win32 buffer is too small for the text.
pub(crate) const ELLIPSIS: char = '\u{2026}';

/// U+00B7 with a space on each side, between the parts of a menu line.
const DOT: &str = " \u{b7} ";

/// U+2013, between the start and end of a time range.
const EN_DASH: char = '\u{2013}';

/// `LOCALE_NAME_USER_DEFAULT` is `NULL` in the Windows headers; windows-rs
/// has no constant for it.
const LOCALE_NAME_USER_DEFAULT: PCWSTR = PCWSTR::null();

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
    match *status {
        Status::Off => "MelCat is off".to_owned(),
        Status::Napping { until: Some(until) } => {
            format!("Mel is napping{DOT}back {}", moment(until, now, calendar, clock, date_time))
        }
        Status::Napping { until: None } => format!("Mel is napping{DOT}no days selected"),
        Status::Active { until: Some(until) } => {
            format!("Mel is playing{DOT}until {}", moment(until, now, calendar, clock, date_time))
        }
        Status::Active { until: None } => "Mel is playing".to_owned(),
    }
}

/// "18:00" today, "tomorrow 09:00", "Mon 09:00" this week, a date beyond.
pub(crate) fn moment(
    date: Instant,
    now: Instant,
    calendar: &impl Calendar,
    clock: &dyn Fn(Instant) -> String,
    date_time: &dyn Fn(Instant) -> String,
) -> String {
    match RelativeDay::of(date, now, calendar) {
        RelativeDay::Today => clock(date),
        RelativeDay::Tomorrow => format!("tomorrow {}", clock(date)),
        RelativeDay::ThisWeek(day) => format!("{} {}", day.short_name(), clock(date)),
        RelativeDay::Later => date_time(date),
    }
}

/// "Mon–Fri, 09:00–18:00", "Weekends, all day", "No days".
/// `time` formats a wall-clock time (`locale_time_of_day`).
pub(crate) fn schedule_summary(schedule: &Schedule, first_weekday: Weekday, time: &dyn Fn(TimeOfDay) -> String) -> String {
    let days = schedule.days.describe(first_weekday);
    if schedule.days.is_empty() {
        return days;
    }
    if schedule.is_all_day() {
        return format!("{days}, all day");
    }
    format!("{days}, {}{EN_DASH}{}", time(schedule.start), time(schedule.end))
}

/// "Gentle · up to 5 pt".
pub(crate) fn intensity_title(intensity: Intensity) -> String {
    // Whole points, truncated like the macOS app's `Int(intensity.radius)`.
    format!("{}{DOT}up to {} pt", intensity.title(), intensity.radius() as i64)
}

/// An instant's local time of day in the user's short time format, without
/// seconds (`GetTimeFormatEx` with `TIME_NOSECONDS`): "18:00" or "6:00 PM".
pub(crate) fn locale_time(instant: Instant) -> String {
    let (date, time) = ChronoCalendar::new(chrono::Local).local(instant);
    format_time(&system_time(date, time)).unwrap_or_else(|| time.to_string())
}

/// A wall-clock time in the user's short time format ("09:00", "9:00 AM").
pub(crate) fn locale_time_of_day(time: TimeOfDay) -> String {
    // Any valid date will do: only the time fields are formatted. 2001-01-01,
    // like the macOS app's `StatusText.date(for:)`.
    format_time(&system_time(LocalDate::new(2001, 1, 1), time)).unwrap_or_else(|| time.to_string())
}

/// A short date and time in the user's format, for moments a week or more away.
pub(crate) fn locale_date_time(instant: Instant) -> String {
    let (date, time) = ChronoCalendar::new(chrono::Local).local(instant);
    let system = system_time(date, time);
    let day = format_date(&system).unwrap_or_else(|| iso_date(date));
    let clock = format_time(&system).unwrap_or_else(|| time.to_string());
    format!("{day} {clock}")
}

/// The user's first day of the week (`LOCALE_IFIRSTDAYOFWEEK`: 0 = Monday ...
/// 6 = Sunday), Sunday if it cannot be read.
pub(crate) fn first_weekday() -> Weekday {
    read_wide(|buffer| {
        // SAFETY: the buffer, if any, is a live slice whose length windows-rs
        // passes as the character count; `None` asks for the length.
        unsafe { GetLocaleInfoEx(LOCALE_NAME_USER_DEFAULT, LOCALE_IFIRSTDAYOFWEEK, buffer) }
    })
    .and_then(|value| weekday_from_locale(&value))
    .unwrap_or(Weekday::Sunday)
}

/// `time` on `date` as the `SYSTEMTIME` the formatting functions take.
/// Seconds are zero; the year is kept within what a `SYSTEMTIME` can hold.
fn system_time(date: LocalDate, time: TimeOfDay) -> SYSTEMTIME {
    SYSTEMTIME {
        wYear: date.year.clamp(1601, 30827) as u16,
        wMonth: date.month as u16,
        // 0 = Sunday, while `Weekday::number` counts Sunday as 1.
        wDayOfWeek: u16::from(date.weekday().number() - 1),
        wDay: date.day as u16,
        wHour: time.hour() as u16,
        wMinute: time.minute() as u16,
        wSecond: 0,
        wMilliseconds: 0,
    }
}

/// The user's short time format (`LOCALE_SSHORTTIME`: "HH:mm", "h:mm tt"),
/// empty if it cannot be read. The tray texts and the settings window's time
/// pickers both use it, so they always show times the same way.
pub(crate) fn short_time_format() -> String {
    read_wide(|buffer| {
        // SAFETY: the buffer, if any, is a live slice whose length windows-rs
        // passes as the character count; `None` asks for the length.
        unsafe { GetLocaleInfoEx(LOCALE_NAME_USER_DEFAULT, LOCALE_SSHORTTIME, buffer) }
    })
    .unwrap_or_default()
}

/// The time of `system` in the user's short format, without seconds.
fn format_time(system: &SYSTEMTIME) -> Option<String> {
    format_time_as(&short_time_format(), system)
}

/// The time of `system` in the picture `format` (a `LOCALE_SSHORTTIME`
/// value), without seconds.
fn format_time_as(format: &str, system: &SYSTEMTIME) -> Option<String> {
    let picture = time_picture(format);
    let format = picture.as_ref().map_or(PCWSTR::null(), |picture| PCWSTR(picture.as_ptr()));
    read_wide(|buffer| {
        // SAFETY: `system` and the null-terminated `picture` outlive the call
        // and are only read; the buffer, if any, is a live slice whose length
        // windows-rs passes along; a null locale means the user's default, a
        // null format the locale's own.
        unsafe {
            GetTimeFormatEx(LOCALE_NAME_USER_DEFAULT, TIME_NOSECONDS, Some(system as *const SYSTEMTIME), format, buffer)
        }
    })
}

/// The picture `GetTimeFormatEx` gets for the short time format `format`:
/// the format itself, or `None` (a null format: Windows picks the locale's
/// own) when it is blank.
fn time_picture(format: &str) -> Option<Vec<u16>> {
    (!format.trim().is_empty()).then(|| wide(format))
}

/// The date of `system` in the user's short format.
fn format_date(system: &SYSTEMTIME) -> Option<String> {
    read_wide(|buffer| {
        // SAFETY: as in `format_time`; a null calendar means the locale's own.
        unsafe {
            GetDateFormatEx(
                LOCALE_NAME_USER_DEFAULT,
                DATE_SHORTDATE,
                Some(system as *const SYSTEMTIME),
                PCWSTR::null(),
                buffer,
                PCWSTR::null(),
            )
        }
    })
}

/// Runs a Win32 call that fills a string buffer the documented two-step way:
/// first without a buffer, which returns the length needed (terminator
/// included), then with a buffer of that length. `None` when either step
/// fails (returns 0 or less).
fn read_wide(mut call: impl FnMut(Option<&mut [u16]>) -> i32) -> Option<String> {
    let needed = call(None);
    if needed <= 0 {
        return None;
    }
    let mut buffer = vec![0u16; needed as usize];
    if call(Some(&mut buffer)) <= 0 {
        return None;
    }
    Some(from_wide(&buffer))
}

/// `LOCALE_IFIRSTDAYOFWEEK`'s text ("0" = Monday ... "6" = Sunday) as a weekday.
fn weekday_from_locale(value: &str) -> Option<Weekday> {
    let index: u8 = value.trim().parse().ok()?;
    if index > 6 {
        return None;
    }
    // Monday is weekday number 2, Sunday number 1.
    Weekday::from_number((index + 1) % 7 + 1)
}

/// "2026-10-05", for when Windows cannot format the date.
fn iso_date(date: LocalDate) -> String {
    format!("{:04}-{:02}-{:02}", date.year, date.month, date.day)
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
    use melcat_core::Days;

    /// The calendar of the pure tests: UTC, so instants are plain arithmetic.
    fn utc() -> ChronoCalendar<chrono::Utc> {
        ChronoCalendar::new(chrono::Utc)
    }

    /// `hour:minute` UTC on `date`.
    fn at(date: LocalDate, hour: i64, minute: i64) -> Instant {
        Instant(date.days_since_epoch() * 86_400 + hour * 3600 + minute * 60)
    }

    /// A Monday.
    const MONDAY: LocalDate = LocalDate::new(2026, 10, 5);

    /// 24-hour clock, as a fixed stand-in for `locale_time`.
    fn clock(instant: Instant) -> String {
        utc().local(instant).1.to_string()
    }

    /// A recognizable stand-in for `locale_date_time`.
    fn date_time(instant: Instant) -> String {
        let (date, time) = utc().local(instant);
        format!("{} {time}", iso_date(date))
    }

    fn line(status: Status, now: Instant) -> String {
        status_line(&status, now, &utc(), &clock, &date_time)
    }

    fn when(date: Instant, now: Instant) -> String {
        moment(date, now, &utc(), &clock, &date_time)
    }

    /// 24-hour `HH:MM`, as a fixed stand-in for `locale_time_of_day`.
    fn hhmm(time: TimeOfDay) -> String {
        time.to_string()
    }

    /// A 12-hour stand-in, to show the formatter is the one used.
    fn twelve_hour(time: TimeOfDay) -> String {
        let hour = match time.hour() % 12 {
            0 => 12,
            hour => hour,
        };
        format!("{hour}:{:02} {}", time.minute(), if time.hour() < 12 { "AM" } else { "PM" })
    }

    #[test]
    fn every_status_line_form() {
        let now = at(MONDAY, 10, 0);
        assert_eq!(line(Status::Off, now), "MelCat is off");
        assert_eq!(
            line(Status::Napping { until: Some(at(MONDAY.plus_days(1), 9, 0)) }, now),
            "Mel is napping \u{b7} back tomorrow 09:00"
        );
        assert_eq!(line(Status::Napping { until: None }, now), "Mel is napping \u{b7} no days selected");
        assert_eq!(line(Status::Active { until: Some(at(MONDAY, 18, 0)) }, now), "Mel is playing \u{b7} until 18:00");
        assert_eq!(line(Status::Active { until: None }, now), "Mel is playing");
        // Friday evening, napping until Monday morning.
        let friday = MONDAY.plus_days(4);
        assert_eq!(
            line(Status::Napping { until: Some(at(friday.plus_days(3), 9, 0)) }, at(friday, 18, 0)),
            "Mel is napping \u{b7} back Mon 09:00"
        );
    }

    #[test]
    fn moments_today_tomorrow_this_week_and_later() {
        let now = at(MONDAY, 10, 0);
        assert_eq!(when(at(MONDAY, 18, 0), now), "18:00");
        assert_eq!(when(at(MONDAY, 23, 59), now), "23:59");
        assert_eq!(when(at(MONDAY.plus_days(1), 0, 0), now), "tomorrow 00:00");
        assert_eq!(when(at(MONDAY.plus_days(1), 9, 0), now), "tomorrow 09:00");
        assert_eq!(when(at(MONDAY.plus_days(2), 9, 0), now), "Wed 09:00");
        assert_eq!(when(at(MONDAY.plus_days(6), 22, 30), now), "Sun 22:30");
        assert_eq!(when(at(MONDAY.plus_days(7), 9, 0), now), "2026-10-12 09:00");
        assert_eq!(when(at(MONDAY.plus_days(-1), 9, 0), now), "2026-10-04 09:00");
    }

    #[test]
    fn moments_count_calendar_days_not_hours() {
        // Half an hour later, but already the next day.
        assert_eq!(when(at(MONDAY.plus_days(1), 0, 30), at(MONDAY, 23, 30)), "tomorrow 00:30");
        // Almost two days later, but only one calendar day.
        assert_eq!(when(at(MONDAY.plus_days(1), 23, 59), at(MONDAY, 0, 0)), "tomorrow 23:59");
    }

    #[test]
    fn schedule_summaries() {
        assert_eq!(
            schedule_summary(&Schedule::WORK_HOURS, Weekday::Sunday, &hhmm),
            "Mon\u{2013}Fri, 09:00\u{2013}18:00"
        );
        assert_eq!(
            schedule_summary(&Schedule::WORK_HOURS, Weekday::Sunday, &twelve_hour),
            "Mon\u{2013}Fri, 9:00 AM\u{2013}6:00 PM"
        );
        let weekends = Schedule::new(Days::WEEKEND, TimeOfDay::hm(8, 0), TimeOfDay::hm(8, 0));
        assert_eq!(schedule_summary(&weekends, Weekday::Sunday, &hhmm), "Weekends, all day");
        let always = Schedule::new(Days::EVERY_DAY, TimeOfDay::MIDNIGHT, TimeOfDay::MIDNIGHT);
        assert_eq!(schedule_summary(&always, Weekday::Monday, &hhmm), "Every day, all day");
        let none = Schedule::new(Days::EMPTY, TimeOfDay::hm(9, 0), TimeOfDay::hm(18, 0));
        assert_eq!(schedule_summary(&none, Weekday::Sunday, &hhmm), "No days");
        let none_all_day = Schedule::new(Days::EMPTY, TimeOfDay::hm(9, 0), TimeOfDay::hm(9, 0));
        assert_eq!(schedule_summary(&none_all_day, Weekday::Sunday, &hhmm), "No days");
    }

    #[test]
    fn schedule_summary_crossing_midnight_keeps_start_then_end() {
        let nights = Schedule::new([Weekday::Friday].into_iter().collect(), TimeOfDay::hm(22, 0), TimeOfDay::hm(6, 30));
        assert_eq!(schedule_summary(&nights, Weekday::Sunday, &hhmm), "Fri, 22:00\u{2013}06:30");
    }

    #[test]
    fn schedule_summary_follows_the_first_weekday() {
        let days: Days = [Weekday::Sunday, Weekday::Monday, Weekday::Wednesday].into_iter().collect();
        let schedule = Schedule::new(days, TimeOfDay::hm(9, 0), TimeOfDay::hm(17, 0));
        assert_eq!(schedule_summary(&schedule, Weekday::Sunday, &hhmm), "Sun, Mon, Wed, 09:00\u{2013}17:00");
        assert_eq!(schedule_summary(&schedule, Weekday::Monday, &hhmm), "Mon, Wed, Sun, 09:00\u{2013}17:00");
    }

    #[test]
    fn intensity_titles() {
        assert_eq!(intensity_title(Intensity::Tiny), "Tiny \u{b7} up to 2 pt");
        assert_eq!(intensity_title(Intensity::Gentle), "Gentle \u{b7} up to 5 pt");
        assert_eq!(intensity_title(Intensity::Playful), "Playful \u{b7} up to 12 pt");
    }

    #[test]
    fn system_times_carry_the_wall_clock() {
        let system = system_time(MONDAY, TimeOfDay::hm(18, 5));
        assert_eq!((system.wYear, system.wMonth, system.wDay), (2026, 10, 5));
        assert_eq!((system.wHour, system.wMinute, system.wSecond, system.wMilliseconds), (18, 5, 0, 0));
        assert_eq!(system.wDayOfWeek, 1, "Monday");
        assert_eq!(system_time(MONDAY.plus_days(6), TimeOfDay::MIDNIGHT).wDayOfWeek, 0, "Sunday");
        assert_eq!(system_time(MONDAY.plus_days(5), TimeOfDay::MIDNIGHT).wDayOfWeek, 6, "Saturday");
        assert_eq!(system_time(LocalDate::new(1200, 1, 1), TimeOfDay::MIDNIGHT).wYear, 1601);
    }

    #[test]
    fn first_weekday_index_maps_monday_to_sunday() {
        let expected = [
            Weekday::Monday,
            Weekday::Tuesday,
            Weekday::Wednesday,
            Weekday::Thursday,
            Weekday::Friday,
            Weekday::Saturday,
            Weekday::Sunday,
        ];
        for (index, day) in expected.into_iter().enumerate() {
            assert_eq!(weekday_from_locale(&index.to_string()), Some(day), "{index}");
        }
        assert_eq!(weekday_from_locale(" 6 "), Some(Weekday::Sunday));
        for junk in ["", "7", "-1", "Mon", "300"] {
            assert_eq!(weekday_from_locale(junk), None, "{junk:?}");
        }
    }

    #[test]
    fn read_wide_asks_for_the_length_then_fills() {
        let text: Vec<u16> = "18:00\0".encode_utf16().collect();
        let mut calls = Vec::new();
        let result = read_wide(|buffer| match buffer {
            None => {
                calls.push(0);
                text.len() as i32
            }
            Some(buffer) => {
                calls.push(buffer.len());
                buffer.copy_from_slice(&text);
                text.len() as i32
            }
        });
        assert_eq!(result.as_deref(), Some("18:00"));
        assert_eq!(calls, [0, 6]);
    }

    #[test]
    fn read_wide_gives_up_when_a_step_fails() {
        assert_eq!(read_wide(|_| 0), None);
        assert_eq!(read_wide(|buffer| if buffer.is_none() { 6 } else { 0 }), None);
        assert_eq!(read_wide(|_| -1), None);
    }

    #[test]
    fn the_short_time_format_is_the_picture_unless_blank() {
        assert_eq!(time_picture("HH:mm"), Some(wide("HH:mm")));
        assert_eq!(time_picture("h:mm tt"), Some(wide("h:mm tt")));
        assert_eq!(time_picture(""), None);
        assert_eq!(time_picture("  "), None);
    }

    /// Times follow the short format they are given, not the locale's long
    /// one, and never show seconds.
    #[test]
    fn times_are_formatted_with_the_given_picture() {
        let evening = system_time(MONDAY, TimeOfDay::hm(18, 5));
        assert_eq!(format_time_as("HH:mm", &evening).as_deref(), Some("18:05"));
        assert_eq!(format_time_as("H.mm", &evening).as_deref(), Some("18.05"));
        assert_eq!(format_time_as("HH:mm:ss", &evening).as_deref(), Some("18:05"));
        let morning = system_time(MONDAY, TimeOfDay::hm(6, 0));
        assert_eq!(format_time_as("HH:mm", &morning).as_deref(), Some("06:00"));
        assert_eq!(format_time_as("H:mm", &morning).as_deref(), Some("6:00"));
        // Blank: the locale's own time format still gives a time.
        let fallback = format_time_as("", &evening).unwrap_or_default();
        assert!(fallback.contains("05"), "{fallback:?}");
    }

    #[test]
    fn iso_dates_are_zero_padded() {
        assert_eq!(iso_date(LocalDate::new(2026, 3, 7)), "2026-03-07");
    }

    /// The live locale (whatever the machine uses): the text is there, without
    /// stray terminators, and reading the first weekday does not panic.
    #[test]
    fn locale_functions_give_text() {
        let now = Instant(chrono::Utc::now().timestamp());
        for text in [locale_time(now), locale_time_of_day(TimeOfDay::hm(18, 0)), locale_date_time(now)] {
            assert!(!text.trim().is_empty(), "{text:?}");
            assert!(!text.contains('\0'), "{text:?}");
        }
        assert!(!short_time_format().contains('\0'));
        let _ = first_weekday();
    }

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
