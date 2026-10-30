//! Tray icon and menu, settings window, launch at login, and the text they show.
//!
//! Everything here runs on the UI thread. Window class names start with
//! `MelCat.`.
//!
//! Module map:
//! - `tray`, `icons`, `menu`, `menu_model`: notification-area icon, balloons and the popup menu.
//! - `settings_window`: the settings page.
//! - `autostart`: launch at login (the Run key).
//! - `text`: status line, schedule summary and locale formatting.
//! - `window`: class registration and per-window state, shared by the windows above.

pub(crate) mod autostart;
pub(crate) mod icons;
pub(crate) mod menu;
pub(crate) mod menu_model;
pub(crate) mod settings_window;
pub(crate) mod text;
pub(crate) mod tray;
pub(crate) mod window;
