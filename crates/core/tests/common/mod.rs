//! Helpers shared by the ported test suites.
#![allow(dead_code)]

use chrono::TimeZone;
use chrono_tz::Tz;
use melcat_core::{ChronoCalendar, Instant, RandomSource, SplitMix64};

/// A calendar in a fixed IANA time zone, with instants built from wall-clock parts.
pub struct Clock {
    pub calendar: ChronoCalendar<Tz>,
}

impl Clock {
    pub fn new(zone: &str) -> Clock {
        Clock { calendar: ChronoCalendar::new(zone.parse::<Tz>().expect("known zone")) }
    }

    pub fn utc() -> Clock {
        Clock::new("UTC")
    }

    /// 2026-10-05 is a Monday. Ambiguous times take the first occurrence.
    pub fn date(&self, year: i32, month: u32, day: u32, hour: u32, minute: u32) -> Instant {
        let at = self.calendar.zone.with_ymd_and_hms(year, month, day, hour, minute, 0).earliest().expect("valid local time");
        Instant(at.timestamp())
    }

    pub fn day(&self, year: i32, month: u32, day: u32) -> Instant {
        self.date(year, month, day, 0, 0)
    }
}

/// Deterministic generator, so random walks repeat exactly.
pub fn seeded(seed: u64) -> SplitMix64 {
    SplitMix64::new(seed)
}

/// Keeps the trait in scope for callers that draw numbers directly.
pub fn draw(rng: &mut impl RandomSource) -> f64 {
    rng.unit()
}
