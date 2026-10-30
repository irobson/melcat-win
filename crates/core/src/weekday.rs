//! Days of the week, sets of them, and their compact English descriptions.

/// A day of the week, numbered like the macOS app (and `Calendar.weekday`):
/// Sunday is 1, Saturday is 7.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Weekday {
    Sunday = 1,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}

impl Weekday {
    /// Sunday to Saturday.
    pub const ALL: [Weekday; 7] = [
        Weekday::Sunday,
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
        Weekday::Saturday,
    ];

    /// The weekday numbered `number` (1 = Sunday ... 7 = Saturday).
    pub fn from_number(number: u8) -> Option<Weekday> {
        number.checked_sub(1).and_then(|index| Self::ALL.get(index as usize)).copied()
    }

    /// 1 = Sunday ... 7 = Saturday.
    pub fn number(self) -> u8 {
        self as u8
    }

    pub fn next(self) -> Weekday {
        Self::ALL[(self.number() % 7) as usize]
    }

    pub fn previous(self) -> Weekday {
        Self::ALL[((self.number() + 5) % 7) as usize]
    }

    /// Three-letter English abbreviation, as shown in the menu and settings.
    pub fn short_name(self) -> &'static str {
        match self {
            Weekday::Sunday => "Sun",
            Weekday::Monday => "Mon",
            Weekday::Tuesday => "Tue",
            Weekday::Wednesday => "Wed",
            Weekday::Thursday => "Thu",
            Weekday::Friday => "Fri",
            Weekday::Saturday => "Sat",
        }
    }

    /// All seven days, starting from `first` (the locale's first day of the week).
    pub fn week_starting_on(first: Weekday) -> [Weekday; 7] {
        let mut days = [first; 7];
        for index in 1..7 {
            days[index] = days[index - 1].next();
        }
        days
    }
}

/// A set of weekdays.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Days(u8);

impl Days {
    pub const EMPTY: Days = Days(0);
    /// Monday to Friday.
    pub const WORKDAYS: Days = Days(0b011_1110);
    /// Saturday and Sunday.
    pub const WEEKEND: Days = Days(0b100_0001);
    pub const EVERY_DAY: Days = Days(0b111_1111);

    fn bit(day: Weekday) -> u8 {
        1 << (day.number() - 1)
    }

    pub fn contains(self, day: Weekday) -> bool {
        self.0 & Self::bit(day) != 0
    }

    pub fn insert(&mut self, day: Weekday) {
        self.0 |= Self::bit(day);
    }

    pub fn remove(&mut self, day: Weekday) {
        self.0 &= !Self::bit(day);
    }

    pub fn len(self) -> usize {
        self.0.count_ones() as usize
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The days in the set, Sunday first.
    pub fn iter(self) -> impl Iterator<Item = Weekday> {
        Weekday::ALL.into_iter().filter(move |&day| self.contains(day))
    }

    /// A compact English description, for the menu.
    ///
    /// Runs of three or more consecutive days collapse into a range, so Monday
    /// to Friday reads "Mon–Fri". Days are listed in calendar order starting
    /// from `first_weekday`.
    ///
    /// ```text
    /// []                         "No days"
    /// all seven                  "Every day"
    /// Saturday and Sunday        "Weekends"
    /// Monday to Friday           "Mon–Fri"
    /// Monday, Wednesday, Friday  "Mon, Wed, Fri"
    /// ```
    pub fn describe(self, first_weekday: Weekday) -> String {
        if self.is_empty() {
            return "No days".to_owned();
        }
        if self.len() == 7 {
            return "Every day".to_owned();
        }
        if self == Days::WEEKEND {
            return "Weekends".to_owned();
        }

        let week = Weekday::week_starting_on(first_weekday);
        let position = |day: Weekday| week.iter().position(|&d| d == day).unwrap_or(0);
        // Start the walk on a day that is not selected, so a run that wraps
        // around the end of the week (Fri–Mon with a Monday start) stays whole.
        let offset = week.iter().position(|&day| !self.contains(day)).unwrap_or(0);
        let ordered = week[offset..].iter().chain(week[..offset].iter()).copied();

        let mut runs: Vec<Vec<Weekday>> = Vec::new();
        let mut current: Vec<Weekday> = Vec::new();
        for day in ordered {
            if self.contains(day) {
                current.push(day);
            } else if !current.is_empty() {
                runs.push(std::mem::take(&mut current));
            }
        }
        if !current.is_empty() {
            runs.push(current);
        }

        // Only runs of three or more become ranges; shorter ones are listed day
        // by day. Everything is then sorted by the position of its first day.
        let mut groups: Vec<Vec<Weekday>> = runs
            .into_iter()
            .flat_map(|run| if run.len() >= 3 { vec![run] } else { run.into_iter().map(|day| vec![day]).collect() })
            .collect();
        groups.sort_by_key(|group| position(group[0]));

        groups
            .iter()
            .map(|group| match group.as_slice() {
                [single] => single.short_name().to_owned(),
                [first, .., last] => format!("{}\u{2013}{}", first.short_name(), last.short_name()),
                [] => String::new(),
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl FromIterator<Weekday> for Days {
    fn from_iter<I: IntoIterator<Item = Weekday>>(days: I) -> Days {
        let mut set = Days::EMPTY;
        for day in days {
            set.insert(day);
        }
        set
    }
}
