//! Platform-independent logic for MelCat: a port of the macOS app's
//! `MelCatCore`, with the same rules and the same test cases, so both apps
//! behave alike.
//!
//! - [`weekday`]: days of the week and compact descriptions ("Mon–Fri").
//! - [`time`]: a wall-clock minute.
//! - [`calendar`]: instants, local dates, and the [`Calendar`] that maps
//!   between them (time zones and daylight saving time).
//! - [`schedule`]: whether a moment is inside the window, and when that changes.
//! - [`policy`]: modes, the move-or-wait decision and the idle threshold.
//! - [`path`]: the stroll planner.
//! - [`relative_day`]: today, tomorrow or a weekday, for status lines.
//! - [`rng`]: the random source the planner and the thresholds draw from.

pub mod calendar;
pub mod path;
pub mod policy;
pub mod relative_day;
pub mod rng;
pub mod schedule;
pub mod time;
pub mod weekday;

pub use calendar::{Calendar, ChronoCalendar, Instant, LocalDate, Repeated};
pub use path::{plan_walk, Intensity, JiggleWalk, Point, Rect};
pub use policy::{
    describe_interval, idle_threshold, verdict, Activity, JiggleConditions, JiggleMode, JiggleVerdict, WaitReason,
    DEFAULT_INTERVAL, INTERVAL_CHOICES, SHORTEST_THRESHOLD_FRACTION,
};
pub use relative_day::RelativeDay;
pub use rng::{RandomSource, SplitMix64};
pub use schedule::Schedule;
pub use time::TimeOfDay;
pub use weekday::{Days, Weekday};
