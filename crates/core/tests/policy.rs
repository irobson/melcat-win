//! Port of the macOS app's `ActivityTests`, `JigglePolicyTests` and `RelativeDayTests`.

mod common;

use common::{seeded, Clock};
use melcat_core::{
    describe_interval, idle_threshold, verdict, Activity, Days, JiggleConditions, JiggleMode, JiggleVerdict,
    RelativeDay, Schedule, TimeOfDay, WaitReason, Weekday, DEFAULT_INTERVAL, INTERVAL_CHOICES,
};

// --- Activity --------------------------------------------------------------

#[test]
fn off_is_off_whatever_the_schedule() {
    let clock = Clock::utc();
    let activity = Activity::at(clock.date(2026, 10, 5, 12, 0), JiggleMode::Off, &Schedule::WORK_HOURS, &clock.calendar);
    assert_eq!(activity, Activity::Off);
    assert!(!activity.is_active());
}

#[test]
fn always_on_ignores_the_schedule_and_never_ends() {
    let clock = Clock::utc();
    let saturday = clock.date(2026, 10, 10, 3, 0);
    assert_eq!(
        Activity::at(saturday, JiggleMode::Always, &Schedule::WORK_HOURS, &clock.calendar),
        Activity::Active { until: None }
    );
}

#[test]
fn scheduled_mode_reports_when_the_current_phase_ends() {
    let clock = Clock::utc();
    let cal = &clock.calendar;
    assert_eq!(
        Activity::at(clock.date(2026, 10, 5, 12, 0), JiggleMode::Scheduled, &Schedule::WORK_HOURS, cal),
        Activity::Active { until: Some(clock.date(2026, 10, 5, 18, 0)) }
    );
    assert_eq!(
        Activity::at(clock.date(2026, 10, 10, 12, 0), JiggleMode::Scheduled, &Schedule::WORK_HOURS, cal),
        Activity::Napping { until: Some(clock.date(2026, 10, 12, 9, 0)) }
    );
    let no_days = Schedule::new(Days::EMPTY, TimeOfDay::hm(9, 0), TimeOfDay::hm(18, 0));
    assert_eq!(
        Activity::at(clock.date(2026, 10, 5, 12, 0), JiggleMode::Scheduled, &no_days, cal),
        Activity::Napping { until: None }
    );
}

// --- JigglePolicy ----------------------------------------------------------

fn conditions() -> JiggleConditions {
    JiggleConditions {
        is_active: true,
        can_post_events: true,
        is_session_away: false,
        is_mouse_button_down: false,
        seconds_since_last_input: 120.0,
        idle_threshold: 60.0,
    }
}

#[test]
fn moves_once_everything_allows_it_and_the_computer_has_been_idle_long_enough() {
    assert_eq!(verdict(&conditions()), JiggleVerdict::Move);
    assert_eq!(
        verdict(&JiggleConditions { seconds_since_last_input: 60.0, idle_threshold: 60.0, ..conditions() }),
        JiggleVerdict::Move
    );
}

#[test]
fn waits_while_someone_is_using_the_computer() {
    assert_eq!(
        verdict(&JiggleConditions { seconds_since_last_input: 59.9, idle_threshold: 60.0, ..conditions() }),
        JiggleVerdict::Wait(WaitReason::RecentInput)
    );
    assert_eq!(
        verdict(&JiggleConditions { seconds_since_last_input: 0.0, ..conditions() }),
        JiggleVerdict::Wait(WaitReason::RecentInput)
    );
}

#[test]
fn each_blocker_is_reported_most_fundamental_first() {
    let c = conditions();
    assert_eq!(
        verdict(&JiggleConditions { is_active: false, can_post_events: false, ..c }),
        JiggleVerdict::Wait(WaitReason::Inactive)
    );
    assert_eq!(
        verdict(&JiggleConditions { can_post_events: false, is_session_away: true, ..c }),
        JiggleVerdict::Wait(WaitReason::NeedsPermission)
    );
    assert_eq!(
        verdict(&JiggleConditions { is_session_away: true, is_mouse_button_down: true, ..c }),
        JiggleVerdict::Wait(WaitReason::SessionAway)
    );
    assert_eq!(
        verdict(&JiggleConditions { is_mouse_button_down: true, ..c }),
        JiggleVerdict::Wait(WaitReason::MouseButtonDown)
    );
}

#[test]
fn idle_thresholds_vary_but_never_exceed_the_chosen_interval() {
    let mut rng = seeded(7);
    let mut seen = std::collections::HashSet::new();
    for _ in 0..1000 {
        let threshold = idle_threshold(60.0, &mut rng);
        assert!((45.0..=60.0).contains(&threshold), "{threshold}");
        seen.insert(threshold as i64);
    }
    assert!(seen.len() > 10, "thresholds should not fall into a fixed rhythm");
}

#[test]
fn intervals_read_in_plain_words() {
    let cases = [
        (30.0, "30 seconds"),
        (60.0, "1 minute"),
        (120.0, "2 minutes"),
        (240.0, "4 minutes"),
        (90.0, "90 seconds"),
        (1.0, "1 second"),
    ];
    for (seconds, expected) in cases {
        assert_eq!(describe_interval(seconds), expected);
    }
    // Absurd values never panic.
    assert_eq!(describe_interval(f64::NAN), "0 seconds");
    assert_eq!(describe_interval(1e300), "1000000000 seconds");
}

#[test]
fn the_default_interval_is_one_of_the_menus_choices() {
    assert!(INTERVAL_CHOICES.contains(&DEFAULT_INTERVAL));
}

// --- RelativeDay -----------------------------------------------------------

#[test]
fn today_tomorrow_this_week_later() {
    let clock = Clock::utc();
    let cal = &clock.calendar;
    let now = clock.date(2026, 10, 5, 23, 0); // Monday
    assert_eq!(RelativeDay::of(clock.date(2026, 10, 5, 23, 30), now, cal), RelativeDay::Today);
    assert_eq!(RelativeDay::of(clock.date(2026, 10, 6, 0, 30), now, cal), RelativeDay::Tomorrow);
    assert_eq!(RelativeDay::of(clock.date(2026, 10, 7, 9, 0), now, cal), RelativeDay::ThisWeek(Weekday::Wednesday));
    assert_eq!(RelativeDay::of(clock.date(2026, 10, 11, 9, 0), now, cal), RelativeDay::ThisWeek(Weekday::Sunday));
    assert_eq!(RelativeDay::of(clock.date(2026, 10, 12, 9, 0), now, cal), RelativeDay::Later);
}
