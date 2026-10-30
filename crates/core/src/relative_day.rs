//! How a future instant relates to today, for status lines such as
//! "until 18:00", "back tomorrow 09:00" or "back Mon 09:00".

use crate::calendar::{Calendar, Instant};
use crate::weekday::Weekday;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelativeDay {
    Today,
    Tomorrow,
    /// Two to six days ahead: the weekday name is unambiguous.
    ThisWeek(Weekday),
    /// A week or more ahead, or in the past.
    Later,
}

impl RelativeDay {
    pub fn of(date: Instant, now: Instant, calendar: &impl Calendar) -> RelativeDay {
        let (today, _) = calendar.local(now);
        let (day, _) = calendar.local(date);
        match today.days_until(day) {
            0 => RelativeDay::Today,
            1 => RelativeDay::Tomorrow,
            2..=6 => RelativeDay::ThisWeek(day.weekday()),
            _ => RelativeDay::Later,
        }
    }
}
