//! The jiggler: a port of the macOS app's `Jiggler.swift`, driven by the UI
//! thread's timers.
//!
//! CONTRACT (owned by the jiggle agent). The decision is
//! `melcat_core::verdict` and the path is `melcat_core::plan_walk`; this type
//! feeds them the live state and plays a stroll back. Behavior, exactly as on
//! macOS:
//!
//! **Tick** ([`Jiggler::tick`], once a second and after any settings change):
//! 1. `Activity::at(now, mode, schedule, ChronoCalendar<chrono::Local>)` gives
//!    the [`Status`]: `Off`, `Napping { until }` or `Active { until }`. When
//!    the status leaves `Active`, a stroll in progress stops at once.
//! 2. The idle threshold is drawn with `melcat_core::idle_threshold` whenever
//!    the interval setting changes and after every finished stroll, minus 1.2 s
//!    (the tick's period plus its slack), never below 1 s, so a move never
//!    comes later than the interval the user picked.
//! 3. While a stroll runs, nothing else happens.
//! 4. Otherwise `verdict` with: active from step 1; `can_post_events` true;
//!    `is_session_away` as passed in; `pointer::is_button_down`; and
//!    `min(pointer::seconds_since_last_input, seconds since the last stroll
//!    ended)` (MelCat's own moves may or may not reset the system idle time;
//!    the smaller value works either way). `Move` starts a stroll.
//!
//! **Stroll start**: the pointer's position is read. If there is no anchor,
//! or the pointer is more than [`TAKEOVER_PX`] from the last pixel MelCat put
//! it on, a person moved it: the anchor becomes the current position. The
//! monitor under the anchor (`geom::MonitorInfo`) gives the bounds: its
//! `logical_frame()` inset by [`EDGE_MARGIN_PT`] (4 pt) on every side.
//! `plan_walk` runs in that monitor's logical points (`to_logical` /
//! `to_physical`), with `settings.intensity` (or the preview's intensity).
//! `WALK_TIMER` (`messages::WALK_MILLIS`) is set on the timer window.
//!
//! **Steps** ([`Jiggler::on_walk_timer`]): the step to show is chosen from the
//! time elapsed since the stroll began (`elapsed / JiggleWalk::STEP_INTERVAL`),
//! not by counting timer messages, so a late timer skips points instead of
//! stretching the stroll. Before each move:
//! - if the pointer is more than [`TAKEOVER_PX`] from both of the last two
//!   pixels MelCat posted, a hand moved it: stop, and forget the anchor;
//! - if a mouse button is down, or [`Jiggler::note_personal_input`] was called
//!   since the stroll began (a key, a button or the wheel): stop, keep the anchor;
//! - otherwise move to the step's pixel (`pointer::move_to`), skipped when it
//!   is the pixel MelCat posted last.
//!   After the last point the stroll ends; the end time is remembered and a new
//!   threshold drawn.
//!
//! **Preview** ([`Jiggler::preview`]): one stroll now with the given
//! intensity, whatever the status and the idle time, unless one is running.
//!
//! **Interrupt** ([`Jiggler::interrupt`]): stop any stroll now and forget the
//! anchor and the last posted pixels (session lock/unlock, display change,
//! sleep, end of session, quit).
//!
//! **Log lines** (`util::log`; the smoke test relies on them, keep them exact):
//! - `status: off` / `status: napping` / `status: active`, once per change;
//! - `stroll started: <n> steps from <x>,<y> to <tx>,<ty>` (pixels);
//! - `stroll finished at <x>,<y>`;
//! - `stroll stopped: pointer moved`, `stroll stopped: input`,
//!   `stroll stopped: <interrupt reason>`.
//!
//! Cases the list above leaves open, decided here:
//! - The first tick always reports (and logs) its status, so the tray and
//!   the log start from a known state even when it is `Off`.
//! - A status that leaves `Active` stops a stroll as
//!   `stroll stopped: status changed`, keeping the anchor.
//! - A move that `SendInput` refuses stops the stroll
//!   (`stroll stopped: the pointer could not be moved`) and counts as its
//!   end, so MelCat tries again after a whole threshold, not every second.

use std::hash::{BuildHasher, RandomState};
use std::time::{Instant as Clock, SystemTime, UNIX_EPOCH};

use melcat_core::{
    idle_threshold, plan_walk, verdict, Activity, ChronoCalendar, Instant, Intensity, JiggleConditions, JiggleVerdict,
    JiggleWalk, RandomSource, SplitMix64,
};
use windows::Win32::Foundation::{GetLastError, HWND};
use windows::Win32::UI::WindowsAndMessaging::{KillTimer, SetTimer};

use crate::geom::{MonitorInfo, PointPx};
use crate::jiggle::pointer;
use crate::messages::{TICK_MILLIS, WALK_MILLIS, WALK_TIMER};
use crate::settings::Settings;
use crate::util::paths;

/// A pointer farther than this (in pixels, on either axis) from where MelCat
/// put it was moved by a person.
pub(crate) const TAKEOVER_PX: i32 = 1;

/// Strolls stay this far inside the monitor's edges, in logical points.
pub(crate) const EDGE_MARGIN_PT: f64 = 4.0;

/// How late a tick may come, in seconds, on top of its period (the macOS
/// app's timer tolerance; a Windows timer is late by a system tick or two).
const TICK_TOLERANCE: f64 = 0.2;

/// Taken off every idle threshold: the tick only looks once a second, give or
/// take its tolerance, so without it a move could come up to that much later
/// than the interval the user picked.
const THRESHOLD_SLACK: f64 = TICK_MILLIS as f64 / 1000.0 + TICK_TOLERANCE;

/// No threshold is ever shorter than this, in seconds.
const SHORTEST_THRESHOLD: f64 = 1.0;

/// What the tray shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Status {
    Off,
    /// Scheduled mode, outside the window; `until` is when it opens again.
    Napping { until: Option<Instant> },
    /// Moving the pointer when idle; `until` is when the window closes.
    Active { until: Option<Instant> },
}

impl Status {
    pub(crate) fn is_active(&self) -> bool {
        matches!(self, Status::Active { .. })
    }
}

/// A stroll under way.
struct Stroll {
    /// The planned positions, in physical pixels, one per `STEP_INTERVAL`.
    /// Never empty.
    pixels: Vec<PointPx>,
    started: Clock,
    /// A key, a button or the wheel was used since the stroll began.
    input_seen: bool,
}

pub(crate) struct Jiggler {
    timer_window: HWND,
    status: Status,
    /// Whether a tick has reported `status` yet.
    reported: bool,
    /// Where the user left the pointer. Strolls wander around it.
    anchor: Option<PointPx>,
    /// The last two pixels MelCat posted, newest first, to notice when a
    /// person takes over. At the start of a stroll, the start pixel.
    last_posted: Option<PointPx>,
    previous_posted: Option<PointPx>,
    stroll: Option<Stroll>,
    /// When the last stroll finished.
    last_stroll_ended: Option<Clock>,
    /// Idle time required before the next stroll, in seconds.
    threshold: f64,
    /// The interval `threshold` was drawn for; another one redraws it.
    threshold_interval: Option<f64>,
    rng: SplitMix64,
}

impl Jiggler {
    /// A jiggler that sets `WALK_TIMER` on `timer_window` (the `MelCat.Msg` window).
    pub(crate) fn new(timer_window: HWND) -> Jiggler {
        Jiggler {
            timer_window,
            status: Status::Off,
            reported: false,
            anchor: None,
            last_posted: None,
            previous_posted: None,
            stroll: None,
            last_stroll_ended: None,
            threshold: 0.0,
            threshold_interval: None,
            rng: SplitMix64::new(seed()),
        }
    }

    /// The once-a-second check. Returns the new status when it changed.
    pub(crate) fn tick(&mut self, settings: &Settings, session_away: bool) -> Option<Status> {
        let activity =
            Activity::at(paths::now(), settings.mode, &settings.schedule, &ChronoCalendar::new(chrono::Local));
        let changed = self.set_status(status_of(activity));

        if self.threshold_interval != Some(settings.idle_interval) {
            self.threshold_interval = Some(settings.idle_interval);
            self.draw_threshold();
        }

        // Outside the window the verdict is `Wait(Inactive)` whatever the
        // pointer says, so there is nothing to read.
        if self.stroll.is_none() && activity.is_active() {
            let since_stroll = self.last_stroll_ended.map(|ended| ended.elapsed().as_secs_f64());
            let conditions = JiggleConditions {
                is_active: true,
                can_post_events: true,
                is_session_away: session_away,
                is_mouse_button_down: pointer::is_button_down(),
                seconds_since_last_input: idle_for_decision(pointer::seconds_since_last_input(), since_stroll),
                idle_threshold: self.threshold,
            };
            if verdict(&conditions) == JiggleVerdict::Move {
                self.begin(settings.intensity);
            }
        }
        changed
    }

    /// `WM_TIMER` with `WALK_TIMER`: the next stroll step.
    pub(crate) fn on_walk_timer(&mut self) {
        // None: a WM_TIMER that was already queued when the stroll ended.
        let Some(stroll) = &self.stroll else { return };
        let index = step_index(stroll.started.elapsed().as_secs_f64(), stroll.pixels.len());
        let pixel = stroll.pixels[index];
        let is_last = index + 1 == stroll.pixels.len();
        let input_seen = stroll.input_seen;

        match check_step(
            pointer::cursor_pos(),
            self.last_posted,
            self.previous_posted,
            pointer::is_button_down(),
            input_seen,
        ) {
            StepCheck::PointerMoved => {
                // A hand took the mouse: let go, and wander around wherever
                // it leaves the pointer next time.
                self.stop("pointer moved");
                self.forget_position();
                return;
            }
            StepCheck::Input => {
                // The pointer is still on MelCat's path: keep the anchor.
                self.stop("input");
                return;
            }
            StepCheck::Go => {}
        }

        if self.last_posted != Some(pixel) {
            if !pointer::move_to(pixel) {
                self.stop("the pointer could not be moved");
                self.last_stroll_ended = Some(Clock::now());
                return;
            }
            self.previous_posted = self.last_posted;
            self.last_posted = Some(pixel);
        }

        if is_last {
            self.end_stroll();
            crate::log_info!("{}", finished_line(pixel));
            self.last_stroll_ended = Some(Clock::now());
            self.draw_threshold();
        }
    }

    /// A key, a mouse button or the wheel was used (from `watch::is_personal_input`).
    pub(crate) fn note_personal_input(&mut self) {
        if let Some(stroll) = &mut self.stroll {
            stroll.input_seen = true;
        }
    }

    /// One stroll now with `intensity` (the settings window's Preview).
    pub(crate) fn preview(&mut self, intensity: Intensity) {
        if self.stroll.is_none() {
            self.begin(intensity);
        }
    }

    /// Stops any stroll now and forgets the anchor. `reason` goes into the log.
    pub(crate) fn interrupt(&mut self, reason: &str) {
        // Locked, asleep, another layout: the pointer may be anywhere now.
        self.stop(reason);
        self.forget_position();
    }

    pub(crate) fn status(&self) -> Status {
        self.status
    }

    /// Records `status`; returns it if it differs from the last one reported.
    fn set_status(&mut self, status: Status) -> Option<Status> {
        if self.reported && status == self.status {
            return None;
        }
        let was_active = self.status.is_active();
        self.status = status;
        self.reported = true;
        if was_active && !status.is_active() {
            // Leave the pointer alone from this moment.
            self.stop("status changed");
        }
        crate::log_info!("{}", status_log_line(status));
        Some(status)
    }

    /// A fresh threshold for the next stroll.
    fn draw_threshold(&mut self) {
        if let Some(interval) = self.threshold_interval {
            self.threshold = threshold_after_slack(idle_threshold(interval, &mut self.rng));
        }
    }

    /// Plans a stroll from where the pointer is and starts its timer.
    fn begin(&mut self, intensity: Intensity) {
        let Some(current) = pointer::cursor_pos() else { return };
        // A pointer that is not where MelCat left it was moved by a person:
        // wander around its new spot from now on.
        if needs_new_anchor(current, self.anchor, self.last_posted) {
            self.anchor = Some(current);
        }
        let anchor = self.anchor.unwrap_or(current);
        let pixels = plan_pixels(current, anchor, MonitorInfo::from_point(anchor), intensity, &mut self.rng);
        let Some(&target) = pixels.last() else { return };

        // SAFETY: a timer on the timer window, which belongs to this thread;
        // no callback, so it arrives as WM_TIMER through the message loop.
        if unsafe { SetTimer(Some(self.timer_window), WALK_TIMER, WALK_MILLIS, None) } == 0 {
            // SAFETY: GetLastError has no preconditions.
            let error = unsafe { GetLastError() };
            crate::log_warn!("could not start the stroll timer (error {})", error.0);
            return;
        }
        crate::log_info!("{}", started_line(pixels.len(), current, target));
        self.last_posted = Some(current);
        self.previous_posted = None;
        self.stroll = Some(Stroll { pixels, started: Clock::now(), input_seen: false });
    }

    /// Ends the stroll in progress, if any, and logs why.
    fn stop(&mut self, reason: &str) {
        if self.stroll.is_some() {
            self.end_stroll();
            crate::log_info!("{}", stopped_line(reason));
        }
    }

    fn end_stroll(&mut self) {
        self.stroll = None;
        // SAFETY: killing this thread's own timer; failure (no such timer) is harmless.
        unsafe {
            let _ = KillTimer(Some(self.timer_window), WALK_TIMER);
        }
    }

    fn forget_position(&mut self) {
        self.anchor = None;
        self.last_posted = None;
        self.previous_posted = None;
    }
}

/// A seed nobody can predict: std's per-process random hash keys over the
/// time of day.
fn seed() -> u64 {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_nanos());
    RandomState::new().hash_one(nanos)
}

/// The status the tray shows for what the mode and schedule say.
fn status_of(activity: Activity) -> Status {
    match activity {
        Activity::Off => Status::Off,
        Activity::Napping { until } => Status::Napping { until },
        Activity::Active { until } => Status::Active { until },
    }
}

/// A drawn threshold minus the tick's period and slack, never below one second.
fn threshold_after_slack(drawn: f64) -> f64 {
    (drawn - THRESHOLD_SLACK).max(SHORTEST_THRESHOLD)
}

/// The idle time the decision uses: the system's, or the time since MelCat's
/// own last stroll if that is shorter. MelCat's moves may or may not reset
/// the system's idle time; the smaller value works either way.
fn idle_for_decision(system_idle: f64, since_last_stroll: Option<f64>) -> f64 {
    since_last_stroll.map_or(system_idle, |since| system_idle.min(since))
}

/// Whether `current` is more than [`TAKEOVER_PX`] from every one of `posted`.
fn far_from_all(current: PointPx, posted: impl IntoIterator<Item = PointPx>) -> bool {
    posted.into_iter().all(|pixel| current.max_gap(pixel) > TAKEOVER_PX)
}

/// At the start of a stroll: whether the anchor becomes the current position,
/// because there is none yet, or the pointer is not where MelCat last put it.
fn needs_new_anchor(current: PointPx, anchor: Option<PointPx>, last_posted: Option<PointPx>) -> bool {
    anchor.is_none() || last_posted.is_none_or(|last| far_from_all(current, [last]))
}

/// What a stroll step does before it moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StepCheck {
    /// The pointer is far from both of the last two posted pixels: a hand moved it.
    PointerMoved,
    /// A button is held, or a key, a button or the wheel was used.
    Input,
    /// Carry on.
    Go,
}

/// The checks before each step, in order. The position check needs both the
/// pointer (unknown when another desktop has the input) and a posted pixel;
/// comparing with the last two absorbs one step of lag in `SendInput`.
fn check_step(
    current: Option<PointPx>,
    last_posted: Option<PointPx>,
    previous_posted: Option<PointPx>,
    button_down: bool,
    input_seen: bool,
) -> StepCheck {
    if let (Some(current), Some(last)) = (current, last_posted) {
        if far_from_all(current, std::iter::once(last).chain(previous_posted)) {
            return StepCheck::PointerMoved;
        }
    }
    if button_down || input_seen {
        return StepCheck::Input;
    }
    StepCheck::Go
}

/// The step to show `elapsed` seconds into a stroll of `steps` positions:
/// one per `STEP_INTERVAL`, so a late timer skips positions instead of
/// stretching the stroll; the last one once the time is up. Negative or NaN
/// times give the first (`as` saturates and maps NaN to 0).
fn step_index(elapsed: f64, steps: usize) -> usize {
    ((elapsed / JiggleWalk::STEP_INTERVAL) as usize).min(steps.saturating_sub(1))
}

/// A stroll from `current` around `anchor` on `monitor`, as physical pixels.
/// Planned in the monitor's logical points, inside its frame inset by
/// [`EDGE_MARGIN_PT`], so distances and the speed look the same at any scale.
fn plan_pixels(
    current: PointPx,
    anchor: PointPx,
    monitor: MonitorInfo,
    intensity: Intensity,
    rng: &mut impl RandomSource,
) -> Vec<PointPx> {
    let bounds = monitor.logical_frame().inset(EDGE_MARGIN_PT, EDGE_MARGIN_PT);
    let walk = plan_walk(monitor.to_logical(current), monitor.to_logical(anchor), intensity, bounds, rng);
    walk.points.iter().map(|&point| monitor.to_physical(point)).collect()
}

fn status_log_line(status: Status) -> String {
    let name = match status {
        Status::Off => "off",
        Status::Napping { .. } => "napping",
        Status::Active { .. } => "active",
    };
    format!("status: {name}")
}

fn started_line(steps: usize, from: PointPx, to: PointPx) -> String {
    format!("stroll started: {steps} steps from {},{} to {},{}", from.x, from.y, to.x, to.y)
}

fn finished_line(at: PointPx) -> String {
    format!("stroll finished at {},{}", at.x, at.y)
}

fn stopped_line(reason: &str) -> String {
    format!("stroll stopped: {reason}")
}

#[cfg(test)]
mod tests {
    use melcat_core::{idle_threshold, SplitMix64};

    use super::*;
    use crate::geom::RectPx;

    #[test]
    fn statuses_follow_the_activity() {
        let at = Some(Instant(1_800_000_000));
        assert_eq!(status_of(Activity::Off), Status::Off);
        assert_eq!(status_of(Activity::Napping { until: at }), Status::Napping { until: at });
        assert_eq!(status_of(Activity::Napping { until: None }), Status::Napping { until: None });
        assert_eq!(status_of(Activity::Active { until: at }), Status::Active { until: at });
        assert_eq!(status_of(Activity::Active { until: None }), Status::Active { until: None });
        assert!(status_of(Activity::Active { until: None }).is_active());
        assert!(!status_of(Activity::Napping { until: at }).is_active());
    }

    #[test]
    fn thresholds_lose_the_tick_slack_but_never_drop_below_a_second() {
        assert!((THRESHOLD_SLACK - 1.2).abs() < 1e-12);
        assert!((threshold_after_slack(60.0) - 58.8).abs() < 1e-9);
        assert!((threshold_after_slack(2.5) - 1.3).abs() < 1e-9);
        assert_eq!(threshold_after_slack(2.0), 1.0);
        assert_eq!(threshold_after_slack(0.0), 1.0);
        let mut rng = SplitMix64::new(7);
        for interval in [5.0, 30.0, 60.0, 240.0] {
            for _ in 0..1000 {
                let threshold = threshold_after_slack(idle_threshold(interval, &mut rng));
                assert!(threshold <= interval - THRESHOLD_SLACK, "{interval}: {threshold}");
                assert!(threshold >= interval * 0.75 - THRESHOLD_SLACK, "{interval}: {threshold}");
            }
        }
    }

    #[test]
    fn the_decision_uses_the_shorter_idle_time() {
        assert_eq!(idle_for_decision(30.0, None), 30.0);
        assert_eq!(idle_for_decision(30.0, Some(10.0)), 10.0);
        assert_eq!(idle_for_decision(5.0, Some(10.0)), 5.0);
    }

    #[test]
    fn a_stroll_keeps_its_anchor_unless_the_pointer_moved() {
        let anchor = Some(PointPx::new(100, 100));
        let last = PointPx::new(-40, 12);
        assert!(needs_new_anchor(last, None, Some(last)));
        assert!(needs_new_anchor(last, anchor, None));
        assert!(!needs_new_anchor(last, anchor, Some(last)));
        assert!(!needs_new_anchor(PointPx::new(-41, 13), anchor, Some(last)));
        assert!(needs_new_anchor(PointPx::new(-42, 12), anchor, Some(last)));
        assert!(needs_new_anchor(PointPx::new(-40, 14), anchor, Some(last)));
    }

    #[test]
    fn a_hand_on_the_mouse_is_noticed_against_the_last_two_pixels() {
        let last = Some(PointPx::new(10, 10));
        let previous = Some(PointPx::new(13, 10));
        let at = |x, y| Some(PointPx::new(x, y));
        assert_eq!(check_step(at(10, 10), last, previous, false, false), StepCheck::Go);
        assert_eq!(check_step(at(11, 9), last, previous, false, false), StepCheck::Go);
        // Still on the previous pixel: the last move has not landed yet.
        assert_eq!(check_step(at(13, 10), last, previous, false, false), StepCheck::Go);
        assert_eq!(check_step(at(10, 12), last, previous, false, false), StepCheck::PointerMoved);
        assert_eq!(check_step(at(16, 10), last, previous, false, false), StepCheck::PointerMoved);
        assert_eq!(check_step(at(12, 10), last, None, false, false), StepCheck::PointerMoved);
        // The position check comes first, so a drag forgets the anchor.
        assert_eq!(check_step(at(40, 40), last, previous, true, true), StepCheck::PointerMoved);
    }

    #[test]
    fn keys_buttons_and_the_wheel_stop_a_stroll_on_its_path() {
        let last = Some(PointPx::new(10, 10));
        assert_eq!(check_step(last, last, None, true, false), StepCheck::Input);
        assert_eq!(check_step(last, last, None, false, true), StepCheck::Input);
        // An unknown pointer or nothing posted yet skips only the position check.
        assert_eq!(check_step(None, last, None, false, false), StepCheck::Go);
        assert_eq!(check_step(None, last, None, false, true), StepCheck::Input);
        assert_eq!(check_step(Some(PointPx::new(90, 90)), None, None, false, false), StepCheck::Go);
    }

    #[test]
    fn steps_follow_the_clock() {
        let step = JiggleWalk::STEP_INTERVAL;
        assert_eq!(step_index(0.0, 60), 0);
        assert_eq!(step_index(step * 0.9, 60), 0);
        assert_eq!(step_index(step * 1.5, 60), 1);
        // A timer 100 ms late skips ahead instead of stretching the stroll.
        assert_eq!(step_index(0.1 + step * 0.5, 60), 6);
        assert_eq!(step_index(step * 59.5, 60), 59);
        assert_eq!(step_index(100.0, 60), 59);
        assert_eq!(step_index(-1.0, 60), 0);
        assert_eq!(step_index(f64::NAN, 60), 0);
        assert_eq!(step_index(f64::INFINITY, 60), 59);
        assert_eq!(step_index(5.0, 1), 0);
    }

    /// Monitors with negative origins and fractional scales, the anchor in
    /// the middle and next to each corner.
    #[test]
    fn strolls_stay_on_their_monitor_inside_the_margin() {
        let monitors = [
            MonitorInfo { handle: 0, rect: RectPx::new(0, 0, 1920, 1080), scale: 1.0 },
            MonitorInfo { handle: 0, rect: RectPx::new(-2560, -200, 0, 1240), scale: 1.5 },
            MonitorInfo { handle: 0, rect: RectPx::new(1920, -1080, 3840, 0), scale: 1.25 },
            MonitorInfo { handle: 0, rect: RectPx::new(-3840, 0, 0, 2160), scale: 2.0 },
        ];
        let mut rng = SplitMix64::new(42);
        for monitor in monitors {
            let r = monitor.rect;
            // The margin in pixels, less half a pixel of rounding.
            let margin = (EDGE_MARGIN_PT * monitor.scale - 0.5).floor() as i32;
            let anchors = [
                PointPx::new(r.left + r.width() / 2, r.top + r.height() / 2),
                PointPx::new(r.left + margin + 1, r.top + margin + 1),
                PointPx::new(r.right - margin - 2, r.bottom - margin - 2),
            ];
            for anchor in anchors {
                for intensity in Intensity::ALL {
                    for _ in 0..50 {
                        let pixels = plan_pixels(anchor, anchor, monitor, intensity, &mut rng);
                        assert!(pixels.len() >= 36, "at least 0.6 s at 60 steps a second");
                        let reach = (intensity.radius() * monitor.scale).ceil() as i32 + 1;
                        for pixel in &pixels {
                            assert!(pixel.x >= r.left + margin && pixel.x < r.right - margin, "{monitor:?} {pixel:?}");
                            assert!(pixel.y >= r.top + margin && pixel.y < r.bottom - margin, "{monitor:?} {pixel:?}");
                            assert!(pixel.max_gap(anchor) <= reach, "{monitor:?} {anchor:?} {pixel:?}");
                        }
                        // Consecutive positions are at most a pixel apart, so a
                        // move that lands a step late never looks like a hand.
                        for pair in pixels.windows(2) {
                            assert!(pair[0].max_gap(pair[1]) <= TAKEOVER_PX, "{monitor:?} {pair:?}");
                        }
                    }
                }
            }
        }
    }

    /// A pointer left on the very edge glides inwards and never leaves the monitor.
    #[test]
    fn a_stroll_from_the_edge_stays_on_the_monitor() {
        let monitor = MonitorInfo { handle: 0, rect: RectPx::new(-2560, -200, 0, 1240), scale: 1.5 };
        let r = monitor.rect;
        let mut rng = SplitMix64::new(3);
        for corner in [PointPx::new(r.left, r.top), PointPx::new(r.right - 1, r.bottom - 1)] {
            for intensity in Intensity::ALL {
                let pixels = plan_pixels(corner, corner, monitor, intensity, &mut rng);
                assert!(pixels.iter().all(|&p| r.contains(p)), "{corner:?}");
                let target = *pixels.last().expect("never empty");
                let margin = (EDGE_MARGIN_PT * monitor.scale) as i32;
                assert!(target.max_gap(corner) >= margin - 1, "the target lies inside the margin: {target:?}");
            }
        }
    }

    #[test]
    fn log_lines_are_exact() {
        assert_eq!(status_log_line(Status::Off), "status: off");
        assert_eq!(status_log_line(Status::Napping { until: None }), "status: napping");
        assert_eq!(status_log_line(Status::Active { until: Some(Instant(0)) }), "status: active");
        assert_eq!(
            started_line(84, PointPx::new(-1200, 340), PointPx::new(-1195, 336)),
            "stroll started: 84 steps from -1200,340 to -1195,336"
        );
        assert_eq!(finished_line(PointPx::new(7, -3)), "stroll finished at 7,-3");
        assert_eq!(stopped_line("pointer moved"), "stroll stopped: pointer moved");
        assert_eq!(stopped_line("session changed"), "stroll stopped: session changed");
    }

    #[test]
    fn seeds_differ() {
        assert_ne!(seed(), seed());
    }
}
