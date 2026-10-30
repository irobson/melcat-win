//! Port of the macOS app's `TimeOfDayTests` and `ScheduleTests`.

mod common;

use common::Clock;
use melcat_core::{Calendar, Days, Instant, Schedule, TimeOfDay, Weekday};

fn hm(hour: i32, minute: i32) -> TimeOfDay {
    TimeOfDay::hm(hour, minute)
}

fn days(list: &[Weekday]) -> Days {
    list.iter().copied().collect()
}

// --- TimeOfDay -------------------------------------------------------------

#[test]
fn hours_and_minutes_round_trip_through_minutes_since_midnight() {
    let time = hm(9, 30);
    assert_eq!(time.minutes(), 570);
    assert_eq!(time.hour(), 9);
    assert_eq!(time.minute(), 30);
    assert_eq!(time.to_string(), "09:30");
}

#[test]
fn out_of_range_values_wrap_around_the_clock() {
    assert_eq!(TimeOfDay::new(-1), hm(23, 59));
    assert_eq!(TimeOfDay::new(1440), TimeOfDay::MIDNIGHT);
    assert_eq!(hm(25, 0), hm(1, 0));
}

#[test]
fn the_wall_clock_time_of_a_date_depends_on_the_calendars_time_zone() {
    let date = Clock::utc().date(2026, 10, 5, 12, 15);
    assert_eq!(Clock::utc().calendar.local(date).1, hm(12, 15));
    assert_eq!(Clock::new("America/Sao_Paulo").calendar.local(date).1, hm(9, 15));
}

// --- Schedule --------------------------------------------------------------
// 2026-10-05 is a Monday; 2026-10-10 a Saturday.

#[test]
fn office_hours_inside_on_weekdays_outside_at_night_and_on_weekends() {
    let clock = Clock::utc();
    let cal = &clock.calendar;
    let work = Schedule::WORK_HOURS;
    assert!(work.is_active(clock.date(2026, 10, 5, 9, 0), cal));
    assert!(work.is_active(clock.date(2026, 10, 5, 17, 59), cal));
    assert!(!work.is_active(clock.date(2026, 10, 5, 18, 0), cal), "the end is exclusive");
    assert!(!work.is_active(clock.date(2026, 10, 5, 8, 59), cal));
    assert!(!work.is_active(clock.date(2026, 10, 10, 12, 0), cal), "Saturday");
    assert!(work.is_active(clock.date(2026, 10, 9, 12, 0), cal), "Friday");
}

#[test]
fn a_window_across_midnight_belongs_to_the_day_it_starts_on() {
    let clock = Clock::utc();
    let cal = &clock.calendar;
    let nights = Schedule::new(days(&[Weekday::Friday]), hm(22, 0), hm(6, 0));
    assert!(nights.crosses_midnight());
    assert!(nights.is_active(clock.date(2026, 10, 9, 22, 0), cal), "Friday evening");
    assert!(nights.is_active(clock.date(2026, 10, 10, 5, 59), cal), "Saturday early morning");
    assert!(!nights.is_active(clock.date(2026, 10, 10, 6, 0), cal), "Saturday at the end");
    assert!(!nights.is_active(clock.date(2026, 10, 9, 5, 0), cal), "Friday early morning belongs to Thursday");
    assert!(!nights.is_active(clock.date(2026, 10, 10, 22, 30), cal), "Saturday evening");
    assert!(!nights.is_active(clock.date(2026, 10, 9, 12, 0), cal), "Friday noon");
}

#[test]
fn equal_start_and_end_mean_all_day_on_the_selected_days() {
    let clock = Clock::utc();
    let cal = &clock.calendar;
    let weekends = Schedule::new(Days::WEEKEND, hm(8, 0), hm(8, 0));
    assert!(weekends.is_all_day());
    assert!(weekends.is_active(clock.date(2026, 10, 10, 0, 0), cal));
    assert!(weekends.is_active(clock.date(2026, 10, 11, 23, 59), cal));
    assert!(!weekends.is_active(clock.date(2026, 10, 12, 0, 0), cal), "Monday");
}

#[test]
fn no_days_selected_never_active_never_changes() {
    let clock = Clock::utc();
    let empty = Schedule::new(Days::EMPTY, hm(9, 0), hm(18, 0));
    let now = clock.date(2026, 10, 5, 12, 0);
    assert!(!empty.is_active(now, &clock.calendar));
    assert_eq!(empty.next_change(now, &clock.calendar), None);
}

#[test]
fn every_day_around_the_clock_always_active_never_changes() {
    let clock = Clock::utc();
    let always = Schedule::new(Days::EVERY_DAY, TimeOfDay::MIDNIGHT, TimeOfDay::MIDNIGHT);
    let now = clock.date(2026, 10, 5, 12, 0);
    assert!(always.is_active(now, &clock.calendar));
    assert_eq!(always.next_change(now, &clock.calendar), None);
}

#[test]
fn next_change_the_end_of_todays_window_or_the_start_of_the_next_one() {
    let clock = Clock::utc();
    let cal = &clock.calendar;
    let work = Schedule::WORK_HOURS;
    // Monday noon: ends at 18:00 today.
    assert_eq!(work.next_change(clock.date(2026, 10, 5, 12, 0), cal), Some(clock.date(2026, 10, 5, 18, 0)));
    // Monday 07:00: starts at 09:00 today.
    assert_eq!(work.next_change(clock.date(2026, 10, 5, 7, 0), cal), Some(clock.date(2026, 10, 5, 9, 0)));
    // Friday 18:30: back on Monday 09:00.
    assert_eq!(work.next_change(clock.date(2026, 10, 9, 18, 30), cal), Some(clock.date(2026, 10, 12, 9, 0)));
    // Exactly at the start: the next change is the end, not the start itself.
    assert_eq!(work.next_change(clock.date(2026, 10, 5, 9, 0), cal), Some(clock.date(2026, 10, 5, 18, 0)));
}

#[test]
fn next_change_across_midnight() {
    let clock = Clock::utc();
    let cal = &clock.calendar;
    let nights = Schedule::new(days(&[Weekday::Friday]), hm(22, 0), hm(6, 0));
    assert_eq!(nights.next_change(clock.date(2026, 10, 9, 23, 0), cal), Some(clock.date(2026, 10, 10, 6, 0)));
    assert_eq!(nights.next_change(clock.date(2026, 10, 10, 7, 0), cal), Some(clock.date(2026, 10, 16, 22, 0)));
}

#[test]
fn adjacent_overnight_windows_run_straight_through() {
    // Every night 22:00–06:00 on Monday and Tuesday: Tuesday 06:00 ends Monday's window.
    let clock = Clock::utc();
    let cal = &clock.calendar;
    let nights = Schedule::new(days(&[Weekday::Monday, Weekday::Tuesday]), hm(22, 0), hm(6, 0));
    assert_eq!(nights.next_change(clock.date(2026, 10, 5, 23, 0), cal), Some(clock.date(2026, 10, 6, 6, 0)));
    assert_eq!(nights.next_change(clock.date(2026, 10, 6, 7, 0), cal), Some(clock.date(2026, 10, 6, 22, 0)));
}

#[test]
fn all_day_schedules_change_at_midnight() {
    let clock = Clock::utc();
    let cal = &clock.calendar;
    let weekends = Schedule::new(Days::WEEKEND, TimeOfDay::MIDNIGHT, TimeOfDay::MIDNIGHT);
    assert_eq!(weekends.next_change(clock.date(2026, 10, 10, 15, 0), cal), Some(clock.date(2026, 10, 12, 0, 0)));
    assert_eq!(weekends.next_change(clock.date(2026, 10, 7, 15, 0), cal), Some(clock.date(2026, 10, 10, 0, 0)));
}

#[test]
fn daylight_saving_windows_follow_the_wall_clock() {
    // New York springs forward on Sunday 2026-03-08 at 02:00.
    let ny = Clock::new("America/New_York");
    let sundays = Schedule::new(days(&[Weekday::Sunday]), hm(1, 0), hm(9, 0));
    let early = ny.date(2026, 3, 8, 1, 30);
    assert!(sundays.is_active(early, &ny.calendar));
    let end = sundays.next_change(early, &ny.calendar);
    assert_eq!(end, Some(ny.date(2026, 3, 8, 9, 0)));
    // Only 6.5 real hours pass, because one hour was skipped.
    assert_eq!(end.map(|end| end.seconds_since(early)), Some(6 * 3600 + 1800));
}

#[test]
fn a_start_time_skipped_by_daylight_saving_still_opens_the_window() {
    let ny = Clock::new("America/New_York");
    let sundays = Schedule::new(days(&[Weekday::Sunday]), hm(2, 30), hm(9, 0));
    let start = sundays.next_change(ny.date(2026, 3, 8, 0, 0), &ny.calendar).expect("a start");
    assert!(sundays.is_active(start, &ny.calendar));
    assert!(start < ny.date(2026, 3, 8, 9, 0));
}

#[test]
fn fall_back_the_repeated_hour_reopens_a_window_that_already_ended() {
    // New York falls back on Sunday 2026-11-01: 02:00 EDT becomes 01:00 EST.
    // 01:30 EDT is 05:30 UTC; 01:00 EST is 06:00 UTC; 01:30 EST is 06:30 UTC.
    let ny = Clock::new("America/New_York");
    let utc = Clock::utc();
    let cal = &ny.calendar;
    let nights = Schedule::new(days(&[Weekday::Saturday]), hm(22, 0), hm(1, 30));

    assert_eq!(nights.next_change(utc.date(2026, 11, 1, 5, 0), cal), Some(utc.date(2026, 11, 1, 5, 30)));
    // Between the two 01:30s the wall clock says inactive, until it jumps back to 01:00.
    assert!(!nights.is_active(utc.date(2026, 11, 1, 5, 45), cal));
    assert_eq!(nights.next_change(utc.date(2026, 11, 1, 5, 45), cal), Some(utc.date(2026, 11, 1, 6, 0)));
    assert!(nights.is_active(utc.date(2026, 11, 1, 6, 10), cal));
    assert_eq!(nights.next_change(utc.date(2026, 11, 1, 6, 10), cal), Some(utc.date(2026, 11, 1, 6, 30)));
}

#[test]
fn fall_back_a_start_in_the_repeated_hour_is_found_twice() {
    let ny = Clock::new("America/New_York");
    let utc = Clock::utc();
    let cal = &ny.calendar;
    let sundays = Schedule::new(days(&[Weekday::Sunday]), hm(1, 30), hm(9, 0));

    // Active from 01:30 EDT; the jump back to 01:00 EST pauses it until 01:30 EST.
    assert_eq!(sundays.next_change(utc.date(2026, 11, 1, 5, 40), cal), Some(utc.date(2026, 11, 1, 6, 0)));
    assert_eq!(sundays.next_change(utc.date(2026, 11, 1, 6, 5), cal), Some(utc.date(2026, 11, 1, 6, 30)));
}

/// The first minute boundary after `after` where `is_active` differs, scanning up to 9 days.
fn first_flip(schedule: &Schedule, after: Instant, cal: &impl Calendar) -> Option<Instant> {
    let current = schedule.is_active(after, cal);
    let mut t = Instant(after.0.div_euclid(60) * 60 + 60);
    let limit = after.plus_seconds(9 * 86_400);
    while t < limit {
        if schedule.is_active(t, cal) != current {
            return Some(t);
        }
        t = t.plus_seconds(60);
    }
    None
}

#[test]
fn next_change_agrees_with_a_minute_by_minute_scan_across_daylight_saving_changes() {
    let schedules = [
        Schedule::new(days(&[Weekday::Saturday, Weekday::Sunday]), hm(22, 0), hm(1, 30)),
        Schedule::new(days(&[Weekday::Sunday]), hm(1, 30), hm(9, 0)),
        Schedule::new(Days::WORKDAYS, hm(2, 15), hm(2, 45)),
        Schedule::new(days(&[Weekday::Sunday]), TimeOfDay::MIDNIGHT, TimeOfDay::MIDNIGHT),
    ];
    let utc = Clock::utc();
    for zone in ["America/New_York", "Europe/Berlin", "America/Santiago", "Australia/Lord_Howe", "Asia/Beirut"] {
        let clock = Clock::new(zone);
        let cal = &clock.calendar;
        // Three days either side of each of the zone's 2026 transitions.
        let mut windows = Vec::new();
        let mut cursor = utc.day(2026, 1, 1);
        let end_of_year = utc.day(2027, 1, 1);
        while let Some(transition) = cal.next_transition(cursor, end_of_year) {
            windows.push((transition.plus_seconds(-3 * 86_400), transition.plus_seconds(3 * 86_400)));
            cursor = transition;
        }
        assert_eq!(windows.len(), 2, "{zone} should change twice in 2026");
        for schedule in &schedules {
            for &(from, to) in &windows {
                let mut minute = from;
                while minute < to {
                    let expected = first_flip(schedule, minute, cal);
                    assert_eq!(
                        schedule.next_change(minute, cal),
                        expected,
                        "{zone} {}–{} after {}",
                        schedule.start,
                        schedule.end,
                        minute.0
                    );
                    minute = minute.plus_seconds(37 * 60); // an odd stride covers every minute offset over time
                }
            }
        }
    }
}
