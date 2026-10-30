//! A wall-clock time, to the minute.

use std::fmt;

/// A wall-clock time, to the minute, independent of any date or time zone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TimeOfDay {
    minutes: u16,
}

impl TimeOfDay {
    pub const MINUTES_PER_DAY: i32 = 24 * 60;
    pub const MIDNIGHT: TimeOfDay = TimeOfDay { minutes: 0 };

    /// Out-of-range values wrap around the clock, so `-1` is 23:59.
    pub fn new(minutes: i32) -> TimeOfDay {
        TimeOfDay { minutes: minutes.rem_euclid(Self::MINUTES_PER_DAY) as u16 }
    }

    /// A time known at compile time; `hour` and `minute` must be in range.
    pub const fn at(hour: u16, minute: u16) -> TimeOfDay {
        TimeOfDay { minutes: (hour * 60 + minute) % 1440 }
    }

    pub fn hm(hour: i32, minute: i32) -> TimeOfDay {
        TimeOfDay::new(hour * 60 + minute)
    }

    /// Minutes since midnight, `0..1440`.
    pub fn minutes(self) -> i32 {
        self.minutes as i32
    }

    pub fn hour(self) -> u32 {
        (self.minutes / 60) as u32
    }

    pub fn minute(self) -> u32 {
        (self.minutes % 60) as u32
    }
}

impl fmt::Display for TimeOfDay {
    /// 24-hour `HH:MM`, for logs and tests. The UI formats with the user's locale.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}", self.hour(), self.minute())
    }
}
