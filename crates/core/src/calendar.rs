//! Instants, local dates, and the calendar that maps between them.
//!
//! The schedule is defined in wall-clock terms ("Monday to Friday, 09:00 to
//! 18:00"), so every rule needs a [`Calendar`]: the time zone that turns an
//! instant into a local date and time and back, including the days the
//! clocks change. [`ChronoCalendar`] implements it for any `chrono` time
//! zone: the app uses `chrono::Local`, the tests `chrono_tz` zones.

use chrono::{DateTime, Datelike, LocalResult, NaiveDate, Offset, TimeZone, Timelike};

use crate::time::TimeOfDay;
use crate::weekday::Weekday;

/// A moment in time: whole seconds since 1970-01-01 00:00:00 UTC.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Instant(pub i64);

impl Instant {
    pub fn plus_seconds(self, seconds: i64) -> Instant {
        Instant(self.0.saturating_add(seconds))
    }

    /// Seconds from `earlier` to `self` (negative if `earlier` is later).
    pub fn seconds_since(self, earlier: Instant) -> i64 {
        self.0.saturating_sub(earlier.0)
    }
}

/// A day of the proleptic Gregorian calendar, in local time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LocalDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl LocalDate {
    pub const fn new(year: i32, month: u32, day: u32) -> LocalDate {
        LocalDate { year, month, day }
    }

    /// Days since 1970-01-01 (Howard Hinnant's `days_from_civil`).
    pub fn days_since_epoch(self) -> i64 {
        let year = self.year as i64 - if self.month <= 2 { 1 } else { 0 };
        let era = year.div_euclid(400);
        let year_of_era = year - era * 400;
        let month = self.month as i64;
        let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + self.day as i64 - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        era * 146_097 + day_of_era - 719_468
    }

    /// The date `days` after 1970-01-01 (Howard Hinnant's `civil_from_days`).
    pub fn from_days_since_epoch(days: i64) -> LocalDate {
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let day_of_era = z - era * 146_097;
        let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let mp = (5 * day_of_year + 2) / 153;
        let day = (day_of_year - (153 * mp + 2) / 5 + 1) as u32;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let year = (year_of_era + era * 400 + if month <= 2 { 1 } else { 0 }) as i32;
        LocalDate { year, month, day }
    }

    pub fn plus_days(self, days: i64) -> LocalDate {
        LocalDate::from_days_since_epoch(self.days_since_epoch() + days)
    }

    /// Calendar days from `self` to `other` (negative if `other` is earlier).
    pub fn days_until(self, other: LocalDate) -> i64 {
        other.days_since_epoch() - self.days_since_epoch()
    }

    pub fn weekday(self) -> Weekday {
        // 1970-01-01 was a Thursday (weekday number 5).
        let number = (self.days_since_epoch() + 4).rem_euclid(7) as u8 + 1;
        Weekday::from_number(number).unwrap_or(Weekday::Sunday)
    }
}

/// Which occurrence of a wall-clock time to take on the day the clocks fall
/// back and the same time happens twice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Repeated {
    First,
    Last,
}

/// A time zone, as the schedule needs it.
pub trait Calendar {
    /// The local date and wall-clock time (to the minute) at `instant`.
    fn local(&self, instant: Instant) -> (LocalDate, TimeOfDay);

    /// The instant at which the local wall clock reads `time` on `date`.
    /// When the clock skips that time (spring forward), the first instant
    /// after the gap. When it shows it twice (fall back), the occurrence
    /// `repeated` picks. `None` only for dates the zone cannot represent.
    fn resolve(&self, date: LocalDate, time: TimeOfDay, repeated: Repeated) -> Option<Instant>;

    /// The first instant in `(after, before)` at which the offset from UTC
    /// changes (a daylight saving transition), if any.
    fn next_transition(&self, after: Instant, before: Instant) -> Option<Instant>;

    /// The first instant of `date`.
    fn start_of_day(&self, date: LocalDate) -> Option<Instant> {
        self.resolve(date, TimeOfDay::MIDNIGHT, Repeated::First)
    }
}

/// A [`Calendar`] for any `chrono` time zone.
#[derive(Clone, Debug)]
pub struct ChronoCalendar<Tz: TimeZone> {
    pub zone: Tz,
}

impl<Tz: TimeZone> ChronoCalendar<Tz> {
    pub fn new(zone: Tz) -> ChronoCalendar<Tz> {
        ChronoCalendar { zone }
    }

    fn offset_seconds(&self, instant: i64) -> i32 {
        match DateTime::from_timestamp(instant, 0) {
            Some(utc) => self.zone.offset_from_utc_datetime(&utc.naive_utc()).fix().local_minus_utc(),
            None => 0,
        }
    }
}

impl<Tz: TimeZone> Calendar for ChronoCalendar<Tz> {
    fn local(&self, instant: Instant) -> (LocalDate, TimeOfDay) {
        let Some(utc) = DateTime::from_timestamp(instant.0, 0) else {
            return (LocalDate::new(1970, 1, 1), TimeOfDay::MIDNIGHT);
        };
        let local = utc.with_timezone(&self.zone).naive_local();
        (
            LocalDate::new(local.year(), local.month(), local.day()),
            TimeOfDay::hm(local.hour() as i32, local.minute() as i32),
        )
    }

    fn resolve(&self, date: LocalDate, time: TimeOfDay, repeated: Repeated) -> Option<Instant> {
        let naive = NaiveDate::from_ymd_opt(date.year, date.month, date.day)?.and_hms_opt(time.hour(), time.minute(), 0)?;
        match self.zone.from_local_datetime(&naive) {
            LocalResult::Single(at) => Some(Instant(at.timestamp())),
            LocalResult::Ambiguous(a, b) => {
                let (a, b) = (a.timestamp(), b.timestamp());
                Some(Instant(match repeated {
                    Repeated::First => a.min(b),
                    Repeated::Last => a.max(b),
                }))
            }
            LocalResult::None => {
                // Skipped by a spring-forward: take the first wall-clock minute
                // that exists after it, which is where the clock jumped to.
                let mut probe = naive;
                for _ in 0..(24 * 60) {
                    probe += chrono::Duration::minutes(1);
                    match self.zone.from_local_datetime(&probe) {
                        LocalResult::Single(at) => return Some(Instant(at.timestamp())),
                        LocalResult::Ambiguous(a, b) => return Some(Instant(a.timestamp().min(b.timestamp()))),
                        LocalResult::None => continue,
                    }
                }
                None
            }
        }
    }

    fn next_transition(&self, after: Instant, before: Instant) -> Option<Instant> {
        // Offsets change at most a few times a year: sample hourly, then
        // bisect the hour in which the offset changed down to the second.
        let base = self.offset_seconds(after.0);
        let mut low = after.0;
        while low < before.0 {
            let high = low.saturating_add(3600).min(before.0);
            if self.offset_seconds(high) != base {
                let (mut lo, mut hi) = (low, high);
                while hi - lo > 1 {
                    let mid = lo + (hi - lo) / 2;
                    if self.offset_seconds(mid) == base {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                return Some(Instant(hi));
            }
            low = high;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates_round_trip() {
        for days in [-800_000_i64, -1, 0, 1, 59, 365, 10_957, 20_730, 2_932_896] {
            assert_eq!(LocalDate::from_days_since_epoch(days).days_since_epoch(), days, "{days}");
        }
        assert_eq!(LocalDate::new(1970, 1, 1).days_since_epoch(), 0);
        assert_eq!(LocalDate::new(2000, 3, 1).days_since_epoch(), 11_017);
        assert_eq!(LocalDate::new(2026, 10, 5).weekday(), Weekday::Monday);
        assert_eq!(LocalDate::new(2024, 2, 28).plus_days(1), LocalDate::new(2024, 2, 29));
        assert_eq!(LocalDate::new(2026, 12, 31).plus_days(1), LocalDate::new(2027, 1, 1));
        assert_eq!(LocalDate::new(2026, 10, 5).days_until(LocalDate::new(2026, 10, 12)), 7);
    }
}
