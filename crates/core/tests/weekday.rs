//! Port of the macOS app's `WeekdayTests`.

mod common;

use common::Clock;
use melcat_core::{Calendar, Days, Weekday};

#[test]
fn weekday_of_a_date_follows_the_calendars_time_zone() {
    let utc = Clock::utc();
    // 2026-10-05 00:30 UTC is a Monday in UTC, still Sunday in São Paulo (UTC−3).
    let date = utc.date(2026, 10, 5, 0, 30);
    assert_eq!(utc.calendar.local(date).0.weekday(), Weekday::Monday);
    assert_eq!(Clock::new("America/Sao_Paulo").calendar.local(date).0.weekday(), Weekday::Sunday);
}

#[test]
fn next_and_previous_wrap_around_the_week() {
    assert_eq!(Weekday::Saturday.next(), Weekday::Sunday);
    assert_eq!(Weekday::Sunday.previous(), Weekday::Saturday);
    assert_eq!(Weekday::Wednesday.next(), Weekday::Thursday);
    assert_eq!(Weekday::Wednesday.previous(), Weekday::Tuesday);
    for day in Weekday::ALL {
        assert_eq!(day.next().previous(), day);
    }
}

#[test]
fn a_week_starts_on_the_calendars_first_weekday() {
    use Weekday::*;
    assert_eq!(Weekday::week_starting_on(Sunday), [Sunday, Monday, Tuesday, Wednesday, Thursday, Friday, Saturday]);
    assert_eq!(Weekday::week_starting_on(Monday)[0], Monday);
    assert_eq!(Weekday::week_starting_on(Monday)[6], Sunday);
}

#[test]
fn day_sets_read_naturally() {
    use Weekday::*;
    let cases: [(Days, &str); 9] = [
        (Days::EMPTY, "No days"),
        (Days::EVERY_DAY, "Every day"),
        (Days::WEEKEND, "Weekends"),
        (Days::WORKDAYS, "Mon\u{2013}Fri"),
        ([Monday, Wednesday, Friday].into_iter().collect(), "Mon, Wed, Fri"),
        ([Monday, Tuesday].into_iter().collect(), "Mon, Tue"),
        ([Monday, Tuesday, Wednesday, Friday].into_iter().collect(), "Mon\u{2013}Wed, Fri"),
        ([Tuesday].into_iter().collect(), "Tue"),
        ([Sunday, Monday, Tuesday, Wednesday, Thursday, Friday].into_iter().collect(), "Sun\u{2013}Fri"),
    ];
    for (days, expected) in cases {
        assert_eq!(days.describe(Sunday), expected);
    }
}

#[test]
fn runs_that_wrap_the_end_of_the_week_stay_whole() {
    use Weekday::*;
    let set = |days: &[Weekday]| days.iter().copied().collect::<Days>();
    // Monday-first week: Sunday is the last day, so Fri, Sat, Sun, Mon is one run.
    assert_eq!(set(&[Friday, Saturday, Sunday, Monday]).describe(Monday), "Fri\u{2013}Mon");
    // Sunday-first week: the same days read from Sunday.
    assert_eq!(set(&[Friday, Saturday, Sunday, Monday]).describe(Sunday), "Fri\u{2013}Mon");
    assert_eq!(set(&[Sunday, Monday, Tuesday]).describe(Monday), "Sun\u{2013}Tue");
    // Short runs across the wrap are listed day by day, in calendar order.
    assert_eq!(set(&[Monday, Wednesday, Sunday]).describe(Monday), "Mon, Wed, Sun");
    assert_eq!(set(&[Wednesday, Friday, Saturday, Sunday, Monday]).describe(Monday), "Wed, Fri\u{2013}Mon");
}

#[test]
fn days_round_trip_through_numbers() {
    for day in Weekday::ALL {
        assert_eq!(Weekday::from_number(day.number()), Some(day));
    }
    assert_eq!(Weekday::from_number(0), None);
    assert_eq!(Weekday::from_number(8), None);
    let mut days = Days::EMPTY;
    days.insert(Weekday::Monday);
    days.insert(Weekday::Friday);
    assert_eq!(days.len(), 2);
    assert!(days.contains(Weekday::Monday) && !days.contains(Weekday::Tuesday));
    days.remove(Weekday::Monday);
    assert_eq!(days.iter().collect::<Vec<_>>(), vec![Weekday::Friday]);
}
