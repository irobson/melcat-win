//! The tray menu as plain data: what it shows for a given state, and what each
//! item does. `ui::menu` turns it into a Win32 popup menu.
//!
//! The menu mirrors the macOS app's (`StatusBarController.swift`), minus the
//! Accessibility item, plus Open Log File:
//!
//! ```text
//! <status line>                      disabled, e.g. "Mel is playing · until 18:00"
//! <schedule line>                    disabled, only when mode is On Schedule, e.g. "Mon–Fri, 09:00–18:00"
//! ─────────
//! On Schedule                        radio items, the current mode checked
//! Always On
//! Off
//! ─────────
//! Movement           ▸ Tiny · up to 2 pt / Gentle · up to 5 pt / Playful · up to 12 pt   (radio)
//! Move After Idle    ▸ 30 seconds / 1 minute / 2 minutes / 4 minutes                      (radio)
//! ─────────
//! Settings…
//! Launch at Login                    check box
//! ─────────
//! Open Log File
//! Quit MelCat
//! ```
//!
//! "Move After Idle" lists `INTERVAL_CHOICES`, plus the current interval in
//! its sorted place when it is not one of them (a value from a hand-edited
//! settings file), checked. Labels come from `melcat_core::describe_interval`
//! and `ui::text::intensity_title`. Menu ids are never 0 and every
//! [`Command`] survives `Command::from_id(command.id())`; the informational
//! lines' ids map to no command.
//!
//! Ids stay within 16 bits: `MENUITEMINFOW::wID` documents menu ids as 16-bit
//! values, and a larger one could come back from `TrackPopupMenuEx` cut short,
//! that is as a different command. An interval's id carries its seconds
//! (`1000 + seconds`), so the ones that fit, up to 64 535 s (almost 18 hours),
//! round-trip. A longer current interval, only possible from a hand-edited
//! file, gets an id that maps to no command: choosing it does nothing, which
//! is what choosing the interval already in force means anyway.

use melcat_core::{describe_interval, Intensity, JiggleMode, INTERVAL_CHOICES};

use crate::ui::text::intensity_title;

/// A check mark style.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Check {
    None,
    /// A check box item, checked or not.
    Toggle(bool),
    /// A radio item (bullet), selected or not.
    Radio(bool),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MenuItem {
    pub id: u32,
    pub text: String,
    pub enabled: bool,
    pub check: Check,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MenuEntry {
    Item(MenuItem),
    Separator,
    Submenu { text: String, entries: Vec<MenuEntry> },
}

/// What a menu item does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Command {
    SetMode(JiggleMode),
    SetIntensity(Intensity),
    /// Whole seconds.
    SetIdleInterval(u32),
    OpenSettings,
    ToggleLaunchAtLogin,
    OpenLog,
    Quit,
}

/// Ids of the informational (disabled) lines; never returned as commands.
const ID_STATUS: u32 = 1;
const ID_SCHEDULE: u32 = 2;
/// The id of an interval too long for its seconds to fit in an id (see the module docs).
const ID_UNLISTED_INTERVAL: u32 = 3;
/// `JiggleMode::MENU_ORDER` from here.
const ID_MODE_BASE: u32 = 10;
/// `Intensity::ALL` from here.
const ID_INTENSITY_BASE: u32 = 20;
const ID_OPEN_SETTINGS: u32 = 30;
const ID_LAUNCH_AT_LOGIN: u32 = 31;
const ID_OPEN_LOG: u32 = 40;
const ID_QUIT: u32 = 41;
/// `SetIdleInterval(seconds)` is `ID_INTERVAL_BASE + seconds`, up to [`ID_MAX`].
const ID_INTERVAL_BASE: u32 = 1000;
/// Menu ids are 16-bit values.
const ID_MAX: u32 = 0xFFFF;
/// The longest interval whose seconds fit in an id: 64 535 s.
const MAX_INTERVAL_IN_ID: u32 = ID_MAX - ID_INTERVAL_BASE;

impl Command {
    /// The menu item id (never 0, which `TrackPopupMenu` uses for "nothing chosen").
    pub(crate) fn id(self) -> u32 {
        match self {
            Command::SetMode(mode) => {
                // Every mode is in the menu order; the fallback is never used.
                ID_MODE_BASE + JiggleMode::MENU_ORDER.iter().position(|&m| m == mode).unwrap_or(0) as u32
            }
            Command::SetIntensity(intensity) => {
                ID_INTENSITY_BASE + Intensity::ALL.iter().position(|&i| i == intensity).unwrap_or(0) as u32
            }
            Command::SetIdleInterval(seconds) if seconds <= MAX_INTERVAL_IN_ID => ID_INTERVAL_BASE + seconds,
            Command::SetIdleInterval(_) => ID_UNLISTED_INTERVAL,
            Command::OpenSettings => ID_OPEN_SETTINGS,
            Command::ToggleLaunchAtLogin => ID_LAUNCH_AT_LOGIN,
            Command::OpenLog => ID_OPEN_LOG,
            Command::Quit => ID_QUIT,
        }
    }

    /// The command for a menu item id, if it is one.
    pub(crate) fn from_id(id: u32) -> Option<Command> {
        let fixed = [Command::OpenSettings, Command::ToggleLaunchAtLogin, Command::OpenLog, Command::Quit];
        if let Some(command) = fixed.into_iter().find(|command| command.id() == id) {
            return Some(command);
        }
        if let Some(&mode) = id.checked_sub(ID_MODE_BASE).and_then(|i| JiggleMode::MENU_ORDER.get(i as usize)) {
            return Some(Command::SetMode(mode));
        }
        if let Some(&intensity) = id.checked_sub(ID_INTENSITY_BASE).and_then(|i| Intensity::ALL.get(i as usize)) {
            return Some(Command::SetIntensity(intensity));
        }
        if (ID_INTERVAL_BASE..=ID_MAX).contains(&id) {
            return Some(Command::SetIdleInterval(id - ID_INTERVAL_BASE));
        }
        None
    }
}

/// Everything the menu depends on.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MenuState {
    /// `ui::text::status_line`, already formatted.
    pub status_line: String,
    /// `ui::text::schedule_summary`, shown only in scheduled mode.
    pub schedule_line: String,
    pub mode: JiggleMode,
    pub intensity: Intensity,
    /// Seconds, as in the settings.
    pub idle_interval: f64,
    pub launch_at_login: bool,
}

/// A disabled line that only informs.
fn info(id: u32, text: &str) -> MenuEntry {
    MenuEntry::Item(MenuItem { id, text: text.to_owned(), enabled: false, check: Check::None })
}

/// An item that runs `command`.
fn action(command: Command, text: impl Into<String>, check: Check) -> MenuEntry {
    MenuEntry::Item(MenuItem { id: command.id(), text: text.into(), enabled: true, check })
}

/// The intervals "Move After Idle" lists, in whole seconds: `INTERVAL_CHOICES`,
/// plus `current` in its sorted place when it is not one of them.
fn interval_choices(current: u32) -> Vec<u32> {
    let mut choices: Vec<u32> = INTERVAL_CHOICES.iter().map(|&seconds| seconds as u32).collect();
    if !choices.contains(&current) {
        choices.push(current);
        choices.sort_unstable();
    }
    choices
}

/// The entries of the tray menu for `state`, top to bottom.
pub(crate) fn build_menu(state: &MenuState) -> Vec<MenuEntry> {
    let mut entries = vec![info(ID_STATUS, &state.status_line)];
    if state.mode == JiggleMode::Scheduled {
        entries.push(info(ID_SCHEDULE, &state.schedule_line));
    }
    entries.push(MenuEntry::Separator);

    for mode in JiggleMode::MENU_ORDER {
        entries.push(action(Command::SetMode(mode), mode.title(), Check::Radio(mode == state.mode)));
    }
    entries.push(MenuEntry::Separator);

    entries.push(MenuEntry::Submenu {
        text: "Movement".to_owned(),
        entries: Intensity::ALL
            .iter()
            .map(|&intensity| {
                action(
                    Command::SetIntensity(intensity),
                    intensity_title(intensity),
                    Check::Radio(intensity == state.intensity),
                )
            })
            .collect(),
    });
    // The settings keep whole seconds; `as` saturates, so even a value that
    // is not a number gives some item rather than a panic.
    let current = state.idle_interval.round() as u32;
    entries.push(MenuEntry::Submenu {
        text: "Move After Idle".to_owned(),
        entries: interval_choices(current)
            .into_iter()
            .map(|seconds| {
                action(
                    Command::SetIdleInterval(seconds),
                    describe_interval(f64::from(seconds)),
                    Check::Radio(seconds == current),
                )
            })
            .collect(),
    });
    entries.push(MenuEntry::Separator);

    entries.push(action(Command::OpenSettings, "Settings\u{2026}", Check::None));
    entries.push(action(Command::ToggleLaunchAtLogin, "Launch at Login", Check::Toggle(state.launch_at_login)));
    entries.push(MenuEntry::Separator);

    entries.push(action(Command::OpenLog, "Open Log File", Check::None));
    entries.push(action(Command::Quit, "Quit MelCat", Check::None));
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(mode: JiggleMode) -> MenuState {
        MenuState {
            status_line: "Mel is playing \u{b7} until 18:00".to_owned(),
            schedule_line: "Mon\u{2013}Fri, 09:00\u{2013}18:00".to_owned(),
            mode,
            intensity: Intensity::Gentle,
            idle_interval: 60.0,
            launch_at_login: false,
        }
    }

    /// The menu as text, one line per entry: `~` disabled, `(*)`/`( )` radio,
    /// `[x]`/`[ ]` check box, `---` separator, `>` submenu, children indented.
    fn outline(entries: &[MenuEntry]) -> Vec<String> {
        let mut lines = Vec::new();
        for entry in entries {
            match entry {
                MenuEntry::Separator => lines.push("---".to_owned()),
                MenuEntry::Submenu { text, entries } => {
                    lines.push(format!("{text} >"));
                    lines.extend(outline(entries).into_iter().map(|line| format!("  {line}")));
                }
                MenuEntry::Item(item) => {
                    let mark = match item.check {
                        Check::None => "",
                        Check::Radio(true) => "(*) ",
                        Check::Radio(false) => "( ) ",
                        Check::Toggle(true) => "[x] ",
                        Check::Toggle(false) => "[ ] ",
                    };
                    let disabled = if item.enabled { "" } else { "~ " };
                    lines.push(format!("{disabled}{mark}{}", item.text));
                }
            }
        }
        lines
    }

    fn items(entries: &[MenuEntry]) -> Vec<MenuItem> {
        let mut out = Vec::new();
        for entry in entries {
            match entry {
                MenuEntry::Item(item) => out.push(item.clone()),
                MenuEntry::Submenu { entries, .. } => out.extend(items(entries)),
                MenuEntry::Separator => {}
            }
        }
        out
    }

    fn submenu<'a>(entries: &'a [MenuEntry], name: &str) -> &'a [MenuEntry] {
        entries
            .iter()
            .find_map(|entry| match entry {
                MenuEntry::Submenu { text, entries } if text == name => Some(entries.as_slice()),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no submenu {name}"))
    }

    fn interval_lines(state: &MenuState) -> Vec<String> {
        outline(submenu(&build_menu(state), "Move After Idle"))
    }

    #[test]
    fn scheduled_menu_layout() {
        let expected = [
            "~ Mel is playing \u{b7} until 18:00",
            "~ Mon\u{2013}Fri, 09:00\u{2013}18:00",
            "---",
            "(*) On Schedule",
            "( ) Always On",
            "( ) Off",
            "---",
            "Movement >",
            "  ( ) Tiny \u{b7} up to 2 pt",
            "  (*) Gentle \u{b7} up to 5 pt",
            "  ( ) Playful \u{b7} up to 12 pt",
            "Move After Idle >",
            "  ( ) 30 seconds",
            "  (*) 1 minute",
            "  ( ) 2 minutes",
            "  ( ) 4 minutes",
            "---",
            "Settings\u{2026}",
            "[ ] Launch at Login",
            "---",
            "Open Log File",
            "Quit MelCat",
        ];
        assert_eq!(outline(&build_menu(&state(JiggleMode::Scheduled))), expected);
    }

    #[test]
    fn schedule_line_only_in_scheduled_mode() {
        for mode in [JiggleMode::Always, JiggleMode::Off] {
            let menu = build_menu(&state(mode));
            let lines = outline(&menu);
            assert_eq!(lines[0], "~ Mel is playing \u{b7} until 18:00");
            assert_eq!(lines[1], "---", "{mode:?}");
            assert!(!lines.iter().any(|line| line.contains("Mon\u{2013}Fri")), "{mode:?}");
            assert!(items(&menu).iter().all(|item| item.id != ID_SCHEDULE));
        }
    }

    #[test]
    fn the_current_mode_intensity_and_login_are_checked() {
        let mut s = state(JiggleMode::Always);
        s.intensity = Intensity::Playful;
        s.idle_interval = 240.0;
        s.launch_at_login = true;
        let lines = outline(&build_menu(&s));
        for line in [
            "( ) On Schedule",
            "(*) Always On",
            "( ) Off",
            "  ( ) Tiny \u{b7} up to 2 pt",
            "  ( ) Gentle \u{b7} up to 5 pt",
            "  (*) Playful \u{b7} up to 12 pt",
            "  ( ) 1 minute",
            "  (*) 4 minutes",
            "[x] Launch at Login",
        ] {
            assert!(lines.iter().any(|l| l == line), "missing {line:?} in {lines:#?}");
        }
        let off = outline(&build_menu(&state(JiggleMode::Off)));
        assert!(off.iter().any(|l| l == "(*) Off") && off.iter().any(|l| l == "( ) On Schedule"));
    }

    #[test]
    fn exactly_one_radio_per_group_is_selected() {
        let menu = build_menu(&state(JiggleMode::Scheduled));
        let selected = |entries: &[MenuEntry]| {
            entries.iter().filter(|e| matches!(e, MenuEntry::Item(MenuItem { check: Check::Radio(true), .. }))).count()
        };
        assert_eq!(selected(&menu), 1, "modes");
        assert_eq!(selected(submenu(&menu, "Movement")), 1);
        assert_eq!(selected(submenu(&menu, "Move After Idle")), 1);
    }

    #[test]
    fn a_custom_interval_takes_its_sorted_place() {
        let mut s = state(JiggleMode::Scheduled);
        s.idle_interval = 90.0;
        assert_eq!(
            interval_lines(&s),
            ["( ) 30 seconds", "( ) 1 minute", "(*) 90 seconds", "( ) 2 minutes", "( ) 4 minutes"]
        );
        s.idle_interval = 5.0;
        assert_eq!(interval_lines(&s)[0], "(*) 5 seconds");
        assert_eq!(interval_lines(&s).len(), 5);
        s.idle_interval = 600.0;
        assert_eq!(interval_lines(&s).last().map(String::as_str), Some("(*) 10 minutes"));
        // A listed value adds nothing; a fraction is rounded first, like the settings.
        s.idle_interval = 59.6;
        assert_eq!(interval_lines(&s), ["( ) 30 seconds", "(*) 1 minute", "( ) 2 minutes", "( ) 4 minutes"]);
    }

    #[test]
    fn interval_choices_are_sorted_and_unique() {
        assert_eq!(interval_choices(60), [30, 60, 120, 240]);
        assert_eq!(interval_choices(45), [30, 45, 60, 120, 240]);
        assert_eq!(interval_choices(86_400), [30, 60, 120, 240, 86_400]);
        assert_eq!(interval_choices(0), [0, 30, 60, 120, 240]);
    }

    #[test]
    fn every_command_id_round_trips() {
        let mut all = vec![Command::OpenSettings, Command::ToggleLaunchAtLogin, Command::OpenLog, Command::Quit];
        all.extend(JiggleMode::MENU_ORDER.map(Command::SetMode));
        all.extend(Intensity::ALL.map(Command::SetIntensity));
        all.extend([0, 5, 30, 60, 90, 120, 240, 600, 3600, 36_000, MAX_INTERVAL_IN_ID].map(Command::SetIdleInterval));
        for command in &all {
            assert_eq!(Command::from_id(command.id()), Some(*command), "{command:?}");
            assert!(command.id() != 0 && command.id() <= ID_MAX, "{command:?}");
        }
        let mut ids: Vec<u32> = all.iter().map(|command| command.id()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), all.len(), "ids are unique");
        for info_id in [0, ID_STATUS, ID_SCHEDULE, ID_UNLISTED_INTERVAL, 13, 23, 999, ID_MAX + 1, u32::MAX] {
            assert_eq!(Command::from_id(info_id), None, "{info_id}");
        }
    }

    #[test]
    fn an_interval_too_long_for_an_id_maps_to_nothing() {
        for seconds in [MAX_INTERVAL_IN_ID + 1, 86_400, u32::MAX] {
            assert_eq!(Command::SetIdleInterval(seconds).id(), ID_UNLISTED_INTERVAL);
        }
        assert_eq!(Command::from_id(ID_UNLISTED_INTERVAL), None);
        // It is still listed and checked: the menu shows the truth.
        let mut s = state(JiggleMode::Scheduled);
        s.idle_interval = 86_400.0;
        assert_eq!(interval_lines(&s).last().map(String::as_str), Some("(*) 1440 minutes"));
    }

    #[test]
    fn menu_items_map_to_their_commands() {
        for mode in JiggleMode::MENU_ORDER {
            for item in items(&build_menu(&state(mode))) {
                assert_ne!(item.id, 0, "{}", item.text);
                assert_eq!(Command::from_id(item.id).is_some(), item.enabled, "{}", item.text);
            }
        }
        let menu = build_menu(&state(JiggleMode::Scheduled));
        let id_of = |text: &str| items(&menu).into_iter().find(|item| item.text == text).map(|item| item.id);
        assert_eq!(id_of("Quit MelCat").and_then(Command::from_id), Some(Command::Quit));
        assert_eq!(id_of("Open Log File").and_then(Command::from_id), Some(Command::OpenLog));
        assert_eq!(id_of("Settings\u{2026}").and_then(Command::from_id), Some(Command::OpenSettings));
        assert_eq!(id_of("Launch at Login").and_then(Command::from_id), Some(Command::ToggleLaunchAtLogin));
        assert_eq!(id_of("Off").and_then(Command::from_id), Some(Command::SetMode(JiggleMode::Off)));
        assert_eq!(id_of("2 minutes").and_then(Command::from_id), Some(Command::SetIdleInterval(120)));
        assert_eq!(
            id_of("Tiny \u{b7} up to 2 pt").and_then(Command::from_id),
            Some(Command::SetIntensity(Intensity::Tiny))
        );
    }

    #[test]
    fn menu_starts_with_the_status_ends_with_quit_and_has_no_double_separators() {
        for mode in JiggleMode::MENU_ORDER {
            let menu = build_menu(&state(mode));
            assert!(matches!(&menu[0], MenuEntry::Item(MenuItem { id: ID_STATUS, enabled: false, .. })));
            assert_eq!(items(&menu).last().map(|item| item.text.clone()), Some("Quit MelCat".to_owned()));
            assert!(menu.windows(2).all(|w| !(w[0] == MenuEntry::Separator && w[1] == MenuEntry::Separator)));
            assert_ne!(menu.last(), Some(&MenuEntry::Separator));
        }
    }
}
