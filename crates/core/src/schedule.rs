//! The days and hours when MelCat keeps the computer looking busy.

use crate::calendar::{Calendar, Instant, Repeated};
use crate::time::TimeOfDay;
use crate::weekday::Days;

/// The days and hours when MelCat keeps the computer looking busy.
///
/// The window is the same on every selected day. It can cross midnight: a
/// window from 22:00 to 06:00 that is set for Friday runs from Friday 22:00 to
/// Saturday 06:00, because a window belongs to the day it starts on. A window
/// whose start equals its end covers the selected days around the clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Schedule {
    pub days: Days,
    pub start: TimeOfDay,
    pub end: TimeOfDay,
}

impl Schedule {
    /// Monday to Friday, 09:00 to 18:00.
    pub const WORK_HOURS: Schedule =
        Schedule { days: Days::WORKDAYS, start: TimeOfDay::at(9, 0), end: TimeOfDay::at(18, 0) };

    pub fn new(days: Days, start: TimeOfDay, end: TimeOfDay) -> Schedule {
        Schedule { days, start, end }
    }

    /// The window covers the whole of each selected day.
    pub fn is_all_day(&self) -> bool {
        self.start == self.end
    }

    /// The window ends on the day after it starts.
    pub fn crosses_midnight(&self) -> bool {
        self.end < self.start
    }

    /// Whether `at` falls inside the window, judged by the wall clock of `calendar`.
    pub fn is_active(&self, at: Instant, calendar: &impl Calendar) -> bool {
        let (date, time) = calendar.local(at);
        let day = date.weekday();

        if self.is_all_day() {
            return self.days.contains(day);
        }
        if self.start < self.end {
            return self.days.contains(day) && time >= self.start && time < self.end;
        }
        // Crosses midnight: the evening part belongs to today's window, the
        // early-morning part to yesterday's.
        if time >= self.start {
            return self.days.contains(day);
        }
        if time < self.end {
            return self.days.contains(day.previous());
        }
        false
    }

    /// The first instant after `after` at which [`Schedule::is_active`]
    /// changes, or `None` if it never does: no days selected, or every day
    /// around the clock.
    ///
    /// `is_active` follows the wall clock, so the state can only flip at a
    /// wall-clock `start` or `end` (both occurrences of it on the day the
    /// clocks fall back), at midnight, or at a daylight saving transition,
    /// where the wall clock jumps. Those instants over the next nine days are
    /// the only candidates checked.
    pub fn next_change(&self, after: Instant, calendar: &impl Calendar) -> Option<Instant> {
        let current = self.is_active(after, calendar);
        let (today, _) = calendar.local(after);
        let horizon = calendar.start_of_day(today.plus_days(9)).unwrap_or(after.plus_seconds(9 * 86_400));

        let mut candidates: Vec<Instant> = Vec::new();
        for offset in 0..=8 {
            let day = today.plus_days(offset);
            candidates.extend(calendar.start_of_day(day));
            for time in [self.start, self.end] {
                for repeated in [Repeated::First, Repeated::Last] {
                    candidates.extend(calendar.resolve(day, time, repeated));
                }
            }
        }
        let mut cursor = after;
        while let Some(transition) = calendar.next_transition(cursor, horizon) {
            candidates.push(transition);
            cursor = transition;
        }

        candidates.retain(|&candidate| candidate > after);
        candidates.sort();
        candidates.dedup();
        candidates.into_iter().find(|&candidate| self.is_active(candidate, calendar) != current)
    }
}
