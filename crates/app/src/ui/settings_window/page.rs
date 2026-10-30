//! What the settings page is made of and what it says: its parts in tab
//! order, their words, and the values the controls show for a given
//! `Settings`. No window is touched here, so all of it is unit tested.

use melcat_core::{JiggleMode, Schedule, TimeOfDay, Weekday, INTERVAL_CHOICES};
use windows::Win32::Foundation::SYSTEMTIME;
use windows::Win32::UI::WindowsAndMessaging::IDOK;

/// Under the Preview button, as on macOS.
pub(super) const PREVIEW_NOTE: &str = "Keep your hand off the mouse for a few seconds.";

/// Under the movement controls.
pub(super) const MOVEMENT_FOOTER: &str = "Mel moves the pointer after about this long without keyboard, mouse or \
     touch input (a little sooner, at random), then again each time it passes, and lets go when you use the mouse \
     or keyboard. Pick a time shorter than your screen saver and screen-off delays.";

/// Every footer [`mode_footer`] can return, so the layout can make room for the longest.
pub(super) const MODE_FOOTERS: [&str; 3] = [
    "Mel keeps your computer active on the days and hours below.",
    "Mel keeps your computer active at any time.",
    "Mel leaves the pointer alone.",
];

const HINT_NO_DAYS: &str = "Pick at least one day.";
const HINT_ALL_DAY: &str = "Start and end are the same: all day on the selected days.";

/// One control (or label) of the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Part {
    ModeLabel,
    /// A mode radio button, indexed in `JiggleMode::MENU_ORDER`.
    Mode(usize),
    ModeFooter,
    ScheduleHeader,
    DaysLabel,
    /// A day check box, indexed in the page's day order (see [`day_order`]).
    Day(usize),
    FromLabel,
    From,
    ToLabel,
    To,
    Hint,
    MovementHeader,
    DistanceLabel,
    Distance,
    IntervalLabel,
    Interval,
    Preview,
    PreviewNote,
    MovementFooter,
    GeneralHeader,
    Launch,
    Close,
}

/// What kind of Win32 control a [`Part`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    /// One line in the semibold message font.
    Header,
    /// One line naming the control on its right.
    Label,
    /// Secondary text in gray, wrapped to its width.
    Note,
    Radio,
    Check,
    /// A time picker.
    Picker,
    /// A drop-down list.
    Combo,
    Button,
    /// The button Enter presses.
    DefaultButton,
}

impl Part {
    /// Every part, in creation order, which is also the tab order and the
    /// order screen readers walk (a label right before the control it names).
    pub(super) const ORDER: [Part; 30] = [
        Part::ModeLabel,
        Part::Mode(0),
        Part::Mode(1),
        Part::Mode(2),
        Part::ModeFooter,
        Part::ScheduleHeader,
        Part::DaysLabel,
        Part::Day(0),
        Part::Day(1),
        Part::Day(2),
        Part::Day(3),
        Part::Day(4),
        Part::Day(5),
        Part::Day(6),
        Part::FromLabel,
        Part::From,
        Part::ToLabel,
        Part::To,
        Part::Hint,
        Part::MovementHeader,
        Part::DistanceLabel,
        Part::Distance,
        Part::IntervalLabel,
        Part::Interval,
        Part::Preview,
        Part::PreviewNote,
        Part::MovementFooter,
        Part::GeneralHeader,
        Part::Launch,
        Part::Close,
    ];

    /// The labels of the left column.
    pub(super) const LABELS: [Part; 5] =
        [Part::DaysLabel, Part::FromLabel, Part::ToLabel, Part::DistanceLabel, Part::IntervalLabel];

    /// The control ID. Close is `IDOK`, the ID `IsDialogMessageW` sends for
    /// Enter when no other button has the focus.
    pub(super) fn id(self) -> u16 {
        match self {
            Part::ModeLabel => 100,
            Part::Mode(index) => 101 + index as u16,
            Part::ModeFooter => 104,
            Part::ScheduleHeader => 110,
            Part::DaysLabel => 111,
            Part::Day(index) => 112 + index as u16,
            Part::FromLabel => 120,
            Part::From => 121,
            Part::ToLabel => 122,
            Part::To => 123,
            Part::Hint => 124,
            Part::MovementHeader => 130,
            Part::DistanceLabel => 131,
            Part::Distance => 132,
            Part::IntervalLabel => 133,
            Part::Interval => 134,
            Part::Preview => 135,
            Part::PreviewNote => 136,
            Part::MovementFooter => 137,
            Part::GeneralHeader => 140,
            Part::Launch => 141,
            Part::Close => IDOK.0 as u16,
        }
    }

    /// The part with control ID `id`, if any.
    pub(super) fn from_id(id: u16) -> Option<Part> {
        Part::ORDER.into_iter().find(|part| part.id() == id)
    }

    pub(super) fn kind(self) -> Kind {
        match self {
            Part::ModeLabel | Part::ScheduleHeader | Part::MovementHeader | Part::GeneralHeader => Kind::Header,
            Part::DaysLabel | Part::FromLabel | Part::ToLabel | Part::DistanceLabel | Part::IntervalLabel => {
                Kind::Label
            }
            Part::ModeFooter | Part::Hint | Part::PreviewNote | Part::MovementFooter => Kind::Note,
            Part::Mode(_) => Kind::Radio,
            Part::Day(_) | Part::Launch => Kind::Check,
            Part::From | Part::To => Kind::Picker,
            Part::Distance | Part::Interval => Kind::Combo,
            Part::Preview => Kind::Button,
            Part::Close => Kind::DefaultButton,
        }
    }

    /// The fixed text of the part. Empty for the parts whose text depends on
    /// the settings or the locale (the mode footer, the hint, the days).
    pub(super) fn title(self) -> &'static str {
        match self {
            Part::ModeLabel => "Mode",
            Part::Mode(index) => JiggleMode::MENU_ORDER.get(index).map_or("", |mode| mode.title()),
            Part::ModeFooter | Part::Hint | Part::Day(_) => "",
            Part::ScheduleHeader => "Schedule",
            Part::DaysLabel => "Days",
            Part::FromLabel => "From",
            Part::From | Part::To | Part::Distance | Part::Interval => "",
            Part::ToLabel => "To",
            Part::MovementHeader => "Movement",
            Part::DistanceLabel => "Distance",
            Part::IntervalLabel => "Move after idle",
            Part::Preview => "Preview",
            Part::PreviewNote => PREVIEW_NOTE,
            Part::MovementFooter => MOVEMENT_FOOTER,
            Part::GeneralHeader => "General",
            Part::Launch => "Launch at login",
            Part::Close => "Close",
        }
    }

    /// Tab moves between these. A radio group has one stop, its first
    /// button; the dialog manager then lands on the checked one.
    pub(super) fn is_tab_stop(self) -> bool {
        match self.kind() {
            Kind::Header | Kind::Label | Kind::Note => false,
            Kind::Radio => self == Part::Mode(0),
            Kind::Check | Kind::Picker | Kind::Combo | Kind::Button | Kind::DefaultButton => true,
        }
    }

    /// `WS_GROUP`: every part starts its own group except the other mode
    /// radio buttons and day check boxes, so the arrow keys move within those two rows.
    pub(super) fn starts_group(self) -> bool {
        !matches!(self, Part::Mode(1..) | Part::Day(1..))
    }

    /// The schedule rows, disabled (controls) or grayed (labels) unless the
    /// mode is On Schedule.
    pub(super) fn is_schedule(self) -> bool {
        matches!(self, Part::DaysLabel | Part::Day(_) | Part::FromLabel | Part::From | Part::ToLabel | Part::To)
    }
}

/// The sentence under the mode buttons.
pub(super) fn mode_footer(mode: JiggleMode) -> &'static str {
    match mode {
        JiggleMode::Scheduled => MODE_FOOTERS[0],
        JiggleMode::Always => MODE_FOOTERS[1],
        JiggleMode::Off => MODE_FOOTERS[2],
    }
}

/// The hint under the time pickers, if the schedule needs one. `time`
/// formats a wall-clock time (`ui::text::locale_time_of_day`).
pub(super) fn schedule_hint(schedule: &Schedule, time: &dyn Fn(TimeOfDay) -> String) -> Option<String> {
    if schedule.days.is_empty() {
        return Some(HINT_NO_DAYS.to_owned());
    }
    if schedule.is_all_day() {
        return Some(HINT_ALL_DAY.to_owned());
    }
    if schedule.crosses_midnight() {
        return Some(night_hint(schedule.end, time));
    }
    None
}

fn night_hint(end: TimeOfDay, time: &dyn Fn(TimeOfDay) -> String) -> String {
    format!("Ends the next morning at {}.", time(end))
}

/// Every kind of hint, with sample times, so the layout keeps room for the
/// tallest and the window never changes size when the hint changes.
pub(super) fn hint_samples(time: &dyn Fn(TimeOfDay) -> String) -> Vec<String> {
    vec![
        HINT_NO_DAYS.to_owned(),
        HINT_ALL_DAY.to_owned(),
        night_hint(TimeOfDay::hm(10, 58), time),
        night_hint(TimeOfDay::hm(22, 58), time),
    ]
}

/// The idle intervals offered: the standard choices, plus `current` in its
/// sorted place when it is a custom value (from a hand-edited settings file).
pub(super) fn interval_choices(current: f64) -> Vec<f64> {
    let mut choices = INTERVAL_CHOICES.to_vec();
    if current.is_finite() && !choices.contains(&current) {
        choices.push(current);
        choices.sort_by(f64::total_cmp);
    }
    choices
}

/// The seven day check boxes, left to right, starting from the locale's first day of the week.
pub(super) fn day_order(first: Weekday) -> [Weekday; 7] {
    Weekday::week_starting_on(first)
}

/// `time` on a fixed day without daylight saving changes (Monday 1 January
/// 2001, as the macOS app uses), for the time pickers, which need a whole date.
pub(super) fn to_system_time(time: TimeOfDay) -> SYSTEMTIME {
    SYSTEMTIME {
        wYear: 2001,
        wMonth: 1,
        wDayOfWeek: 1,
        wDay: 1,
        wHour: time.hour() as u16,
        wMinute: time.minute() as u16,
        wSecond: 0,
        wMilliseconds: 0,
    }
}

/// The wall-clock time a picker shows; its date and seconds are ignored.
pub(super) fn from_system_time(time: &SYSTEMTIME) -> TimeOfDay {
    TimeOfDay::hm(i32::from(time.wHour), i32::from(time.wMinute))
}

/// The pickers' display format: the user's short time format
/// (`LOCALE_SSHORTTIME`, hours and minutes, "HH:mm" or "h:mm tt"), whose
/// picture codes the picker understands. 24-hour if the locale gives none.
pub(super) fn picker_format(short_time: &str) -> String {
    let short_time = short_time.trim();
    if short_time.is_empty() {
        "HH:mm".to_owned()
    } else {
        short_time.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use melcat_core::Days;

    fn clock(time: TimeOfDay) -> String {
        time.to_string()
    }

    #[test]
    fn mode_footers_match_the_macos_app() {
        assert_eq!(mode_footer(JiggleMode::Scheduled), "Mel keeps your computer active on the days and hours below.");
        assert_eq!(mode_footer(JiggleMode::Always), "Mel keeps your computer active at any time.");
        assert_eq!(mode_footer(JiggleMode::Off), "Mel leaves the pointer alone.");
    }

    #[test]
    fn hints_for_every_kind_of_schedule() {
        let work = Schedule::WORK_HOURS;
        assert_eq!(schedule_hint(&work, &clock), None);
        let nobody = Schedule::new(Days::EMPTY, work.start, work.end);
        assert_eq!(schedule_hint(&nobody, &clock).as_deref(), Some("Pick at least one day."));
        let all_day = Schedule::new(Days::WEEKEND, TimeOfDay::hm(8, 0), TimeOfDay::hm(8, 0));
        assert_eq!(
            schedule_hint(&all_day, &clock).as_deref(),
            Some("Start and end are the same: all day on the selected days.")
        );
        let night = Schedule::new(Days::WORKDAYS, TimeOfDay::hm(22, 0), TimeOfDay::hm(6, 0));
        assert_eq!(schedule_hint(&night, &clock).as_deref(), Some("Ends the next morning at 06:00."));
        let twelve_hour = |time: TimeOfDay| format!("{}:{:02} AM", time.hour(), time.minute());
        assert_eq!(schedule_hint(&night, &twelve_hour).as_deref(), Some("Ends the next morning at 6:00 AM."));
        // No days wins over everything else, as on macOS.
        let empty_night = Schedule::new(Days::EMPTY, TimeOfDay::hm(22, 0), TimeOfDay::hm(6, 0));
        assert_eq!(schedule_hint(&empty_night, &clock).as_deref(), Some("Pick at least one day."));
    }

    #[test]
    fn hint_samples_cover_every_hint() {
        let samples = hint_samples(&clock);
        assert!(samples.iter().any(|sample| sample == "Pick at least one day."));
        assert!(samples.iter().any(|sample| sample.starts_with("Start and end are the same")));
        assert!(samples.iter().any(|sample| sample == "Ends the next morning at 22:58."));
    }

    #[test]
    fn interval_choices_insert_a_custom_value_in_order() {
        assert_eq!(interval_choices(60.0), vec![30.0, 60.0, 120.0, 240.0]);
        assert_eq!(interval_choices(240.0), vec![30.0, 60.0, 120.0, 240.0]);
        assert_eq!(interval_choices(90.0), vec![30.0, 60.0, 90.0, 120.0, 240.0]);
        assert_eq!(interval_choices(5.0), vec![5.0, 30.0, 60.0, 120.0, 240.0]);
        assert_eq!(interval_choices(86_400.0), vec![30.0, 60.0, 120.0, 240.0, 86_400.0]);
        assert_eq!(interval_choices(f64::NAN), vec![30.0, 60.0, 120.0, 240.0]);
    }

    #[test]
    fn days_start_on_the_locale_first_weekday() {
        use Weekday::*;
        assert_eq!(day_order(Sunday), [Sunday, Monday, Tuesday, Wednesday, Thursday, Friday, Saturday]);
        assert_eq!(day_order(Monday), [Monday, Tuesday, Wednesday, Thursday, Friday, Saturday, Sunday]);
        assert_eq!(day_order(Saturday), [Saturday, Sunday, Monday, Tuesday, Wednesday, Thursday, Friday]);
    }

    #[test]
    fn times_round_trip_through_the_pickers() {
        for minutes in 0..TimeOfDay::MINUTES_PER_DAY {
            let time = TimeOfDay::new(minutes);
            assert_eq!(from_system_time(&to_system_time(time)), time);
        }
        let nine = to_system_time(TimeOfDay::hm(9, 30));
        assert_eq!((nine.wYear, nine.wMonth, nine.wDay, nine.wDayOfWeek), (2001, 1, 1, 1));
        assert_eq!((nine.wHour, nine.wMinute, nine.wSecond), (9, 30, 0));
        // The picker's date and seconds do not matter.
        let picked = SYSTEMTIME { wYear: 2026, wMonth: 10, wDay: 30, wHour: 18, wMinute: 5, wSecond: 42, ..nine };
        assert_eq!(from_system_time(&picked), TimeOfDay::hm(18, 5));
    }

    #[test]
    fn the_picker_format_falls_back_to_24_hours() {
        assert_eq!(picker_format("h:mm tt"), "h:mm tt");
        assert_eq!(picker_format("HH:mm"), "HH:mm");
        assert_eq!(picker_format(""), "HH:mm");
        assert_eq!(picker_format("  "), "HH:mm");
    }

    #[test]
    fn control_ids_are_unique_and_close_is_idok() {
        for (index, part) in Part::ORDER.iter().enumerate() {
            assert_eq!(Part::from_id(part.id()), Some(*part), "{part:?}");
            for other in &Part::ORDER[index + 1..] {
                assert_ne!(part.id(), other.id(), "{part:?} and {other:?}");
            }
        }
        assert_eq!(Part::Close.id(), 1);
        assert_eq!(Part::from_id(2), None, "IDCANCEL is not a control");
    }

    #[test]
    fn titles_follow_the_macos_page() {
        let modes: Vec<_> = (0..3).map(|index| Part::Mode(index).title()).collect();
        assert_eq!(modes, ["On Schedule", "Always On", "Off"]);
        assert_eq!(Part::IntervalLabel.title(), "Move after idle");
        assert_eq!(Part::Launch.title(), "Launch at login");
        assert!(
            MOVEMENT_FOOTER.contains("without keyboard, mouse or touch input (a little sooner"),
            "{MOVEMENT_FOOTER}"
        );
        assert!(MOVEMENT_FOOTER.ends_with("screen saver and screen-off delays."));
        assert!(!MOVEMENT_FOOTER.contains("  "), "no double spaces from the line breaks");
    }

    #[test]
    fn tab_order_and_groups() {
        let stops: Vec<Part> = Part::ORDER.into_iter().filter(|part| part.is_tab_stop()).collect();
        assert_eq!(stops.first(), Some(&Part::Mode(0)));
        assert_eq!(stops.last(), Some(&Part::Close));
        assert_eq!(stops.len(), 1 + 7 + 2 + 2 + 1 + 1 + 1);
        assert!(!Part::Mode(1).starts_group() && !Part::Day(6).starts_group());
        assert!(Part::Mode(0).starts_group() && Part::Day(0).starts_group() && Part::ModeFooter.starts_group());
        let schedule: Vec<Part> = Part::ORDER.into_iter().filter(|part| part.is_schedule()).collect();
        assert_eq!(schedule.len(), 3 + 7 + 2);
        assert!(!Part::Hint.is_schedule() && !Part::Mode(0).is_schedule());
    }
}
