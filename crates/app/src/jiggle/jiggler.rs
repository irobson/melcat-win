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

use std::time::Instant as Clock;

use melcat_core::{Instant, Intensity};
use windows::Win32::Foundation::HWND;

use crate::geom::PointPx;
use crate::settings::Settings;

/// A pointer farther than this (in pixels, on either axis) from where MelCat
/// put it was moved by a person.
pub(crate) const TAKEOVER_PX: i32 = 1;

/// Strolls stay this far inside the monitor's edges, in logical points.
pub(crate) const EDGE_MARGIN_PT: f64 = 4.0;

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

pub(crate) struct Jiggler {
    timer_window: HWND,
    status: Status,
    anchor: Option<PointPx>,
    started: Option<Clock>,
}

impl Jiggler {
    /// A jiggler that sets `WALK_TIMER` on `timer_window` (the `MelCat.Msg` window).
    pub(crate) fn new(timer_window: HWND) -> Jiggler {
        todo!("CONTRACT: see the module docs")
    }

    /// The once-a-second check. Returns the new status when it changed.
    pub(crate) fn tick(&mut self, settings: &Settings, session_away: bool) -> Option<Status> {
        todo!("CONTRACT: see the module docs")
    }

    /// `WM_TIMER` with `WALK_TIMER`: the next stroll step.
    pub(crate) fn on_walk_timer(&mut self) {
        todo!("CONTRACT: see the module docs")
    }

    /// A key, a mouse button or the wheel was used (from `watch::is_personal_input`).
    pub(crate) fn note_personal_input(&mut self) {
        todo!("CONTRACT: see the module docs")
    }

    /// One stroll now with `intensity` (the settings window's Preview).
    pub(crate) fn preview(&mut self, intensity: Intensity) {
        todo!("CONTRACT: see the module docs")
    }

    /// Stops any stroll now and forgets the anchor. `reason` goes into the log.
    pub(crate) fn interrupt(&mut self, reason: &str) {
        todo!("CONTRACT: see the module docs")
    }

    pub(crate) fn status(&self) -> Status {
        self.status
    }

    pub(crate) fn is_strolling(&self) -> bool {
        todo!("CONTRACT: see the module docs")
    }
}
