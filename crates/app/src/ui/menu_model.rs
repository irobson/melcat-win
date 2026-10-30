//! The tray menu as plain data: what it shows for a given state, and what each
//! item does. `ui::menu` turns it into a Win32 popup menu.
//!
//! CONTRACT (owned by the tray agent). The menu mirrors the macOS app's
//! (`StatusBarController.swift`), minus the Accessibility item, plus Open
//! Log File:
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

use melcat_core::{Intensity, JiggleMode};

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

impl Command {
    /// The menu item id (never 0, which `TrackPopupMenu` uses for "nothing chosen").
    pub(crate) fn id(self) -> u32 {
        todo!("CONTRACT: stable, distinct, non-zero ids")
    }

    /// The command for a menu item id, if it is one.
    pub(crate) fn from_id(id: u32) -> Option<Command> {
        todo!("CONTRACT: inverse of `id`")
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

/// The entries of the tray menu for `state`, top to bottom.
pub(crate) fn build_menu(state: &MenuState) -> Vec<MenuEntry> {
    todo!("CONTRACT: the layout in the module docs")
}
