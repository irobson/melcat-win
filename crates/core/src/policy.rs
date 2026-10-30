//! Modes, what they mean at a given moment, and the decision to move.

use crate::calendar::{Calendar, Instant};
use crate::rng::RandomSource;
use crate::schedule::Schedule;

/// When MelCat is allowed to move the pointer at all.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum JiggleMode {
    /// Never moves the pointer.
    Off,
    /// Moves the pointer only inside the schedule's window.
    #[default]
    Scheduled,
    /// Moves the pointer at any time.
    Always,
}

impl JiggleMode {
    /// Menu order: On Schedule, Always On, Off.
    pub const MENU_ORDER: [JiggleMode; 3] = [JiggleMode::Scheduled, JiggleMode::Always, JiggleMode::Off];

    pub fn title(self) -> &'static str {
        match self {
            JiggleMode::Scheduled => "On Schedule",
            JiggleMode::Always => "Always On",
            JiggleMode::Off => "Off",
        }
    }
}

/// What the mode and the schedule say about a given moment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    /// The mode is off.
    Off,
    /// Scheduled mode, outside the window. `until` is when the next window
    /// opens, or `None` if none ever will (no days selected).
    Napping { until: Option<Instant> },
    /// Inside the window, or always on. `until` is when the window closes, or
    /// `None` if it never does (always on, or every day around the clock).
    Active { until: Option<Instant> },
}

impl Activity {
    pub fn is_active(self) -> bool {
        matches!(self, Activity::Active { .. })
    }

    pub fn at(now: Instant, mode: JiggleMode, schedule: &Schedule, calendar: &impl Calendar) -> Activity {
        match mode {
            JiggleMode::Off => Activity::Off,
            JiggleMode::Always => Activity::Active { until: None },
            JiggleMode::Scheduled => {
                let until = schedule.next_change(now, calendar);
                if schedule.is_active(now, calendar) {
                    Activity::Active { until }
                } else {
                    Activity::Napping { until }
                }
            }
        }
    }
}

/// Everything the policy needs to know about the present moment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JiggleConditions {
    /// The mode and schedule allow moving now.
    pub is_active: bool,
    /// The system lets this process post input events. Always true on
    /// Windows; kept so the decision matches the macOS app's exactly.
    pub can_post_events: bool,
    /// Nobody is at this session's screen: it is locked, the screen saver is
    /// running, the displays are off, or the session is not the one on the
    /// console. Moving the pointer then would only wake things up.
    pub is_session_away: bool,
    /// A mouse button is held, for example in the middle of a drag.
    pub is_mouse_button_down: bool,
    /// Seconds since the last keyboard, mouse or touch input, MelCat's own included.
    pub seconds_since_last_input: f64,
    /// How long the computer must sit idle before the next move.
    pub idle_threshold: f64,
}

/// Why the policy waits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaitReason {
    Inactive,
    NeedsPermission,
    SessionAway,
    MouseButtonDown,
    /// Someone used the keyboard or mouse less than `idle_threshold` ago.
    RecentInput,
}

/// The decision taken once per tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JiggleVerdict {
    Move,
    Wait(WaitReason),
}

/// Move only when everything allows it and the computer has been idle long
/// enough. MelCat never competes with a person who is using the computer.
pub fn verdict(conditions: &JiggleConditions) -> JiggleVerdict {
    if !conditions.is_active {
        return JiggleVerdict::Wait(WaitReason::Inactive);
    }
    if !conditions.can_post_events {
        return JiggleVerdict::Wait(WaitReason::NeedsPermission);
    }
    if conditions.is_session_away {
        return JiggleVerdict::Wait(WaitReason::SessionAway);
    }
    if conditions.is_mouse_button_down {
        return JiggleVerdict::Wait(WaitReason::MouseButtonDown);
    }
    if conditions.seconds_since_last_input < conditions.idle_threshold {
        return JiggleVerdict::Wait(WaitReason::RecentInput);
    }
    JiggleVerdict::Move
}

/// Fraction of the chosen interval below which a threshold is never drawn.
pub const SHORTEST_THRESHOLD_FRACTION: f64 = 0.75;

/// A fresh idle threshold for the next move: somewhere between 75 % and
/// 100 % of `interval`, so moves never fall into a fixed rhythm and never
/// come later than the interval the user picked.
pub fn idle_threshold(interval: f64, rng: &mut impl RandomSource) -> f64 {
    interval * rng.uniform(SHORTEST_THRESHOLD_FRACTION, 1.0)
}

/// The idle intervals offered in the menu and settings, in seconds.
pub const INTERVAL_CHOICES: [f64; 4] = [30.0, 60.0, 120.0, 240.0];

/// The interval used until the user picks one.
pub const DEFAULT_INTERVAL: f64 = 60.0;

/// "30 seconds", "1 minute", "2 minutes"...
pub fn describe_interval(interval: f64) -> String {
    // Clamp before converting, like the macOS app: absurd values stay printable.
    let seconds = if interval.is_finite() { interval.round().clamp(0.0, 1e9) as i64 } else { 0 };
    if seconds < 60 || seconds % 60 != 0 {
        return if seconds == 1 { "1 second".to_owned() } else { format!("{seconds} seconds") };
    }
    let minutes = seconds / 60;
    if minutes == 1 {
        "1 minute".to_owned()
    } else {
        format!("{minutes} minutes")
    }
}
