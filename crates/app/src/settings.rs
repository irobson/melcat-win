//! Persisted preferences, the same as the macOS app's.
//!
//! JSON at `%APPDATA%\MelCat\settings.json`, loaded at start (defaults on any
//! error, field by field) and saved on every change. Launch at login is not
//! stored here but in `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
//!
//! ```json
//! { "mode": "Scheduled", "days": [2, 3, 4, 5, 6], "start_minutes": 540,
//!   "end_minutes": 1080, "intensity": "Gentle", "idle_interval_secs": 60 }
//! ```
//!
//! Days are numbered from Sunday (1) to Saturday (7); times are minutes
//! since midnight. A hand-edited file is repaired on load: times wrap around
//! the clock, unknown day numbers are dropped, and the idle interval is
//! rounded to whole seconds and kept between 5 seconds and a day.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use melcat_core::{Days, Intensity, JiggleMode, Schedule, TimeOfDay, Weekday, DEFAULT_INTERVAL};
use serde::{Deserialize, Serialize};

/// Shortest and longest idle interval accepted, in seconds.
pub const IDLE_RANGE: (f64, f64) = (5.0, 24.0 * 60.0 * 60.0);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModeSetting {
    Off,
    #[default]
    Scheduled,
    Always,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntensitySetting {
    Tiny,
    #[default]
    Gentle,
    Playful,
}

/// The file's shape. Kept separate from [`Settings`] so the core types stay
/// free of serialization.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct SettingsFile {
    mode: ModeSetting,
    days: Vec<u8>,
    start_minutes: i32,
    end_minutes: i32,
    intensity: IntensitySetting,
    idle_interval_secs: f64,
}

impl Default for SettingsFile {
    fn default() -> Self {
        SettingsFile::from(&Settings::default())
    }
}

/// What the user chose.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    /// Off, on schedule, or always on. Defaults to on schedule.
    pub mode: JiggleMode,
    /// Days and hours for scheduled mode. Defaults to Monday to Friday, 09:00 to 18:00.
    pub schedule: Schedule,
    /// How far the pointer wanders. Defaults to gentle.
    pub intensity: Intensity,
    /// Seconds without any input before the pointer moves, and between moves
    /// while the computer stays idle. Defaults to one minute.
    pub idle_interval: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            mode: JiggleMode::Scheduled,
            schedule: Schedule::WORK_HOURS,
            intensity: Intensity::Gentle,
            idle_interval: DEFAULT_INTERVAL,
        }
    }
}

impl From<&Settings> for SettingsFile {
    fn from(settings: &Settings) -> Self {
        SettingsFile {
            mode: match settings.mode {
                JiggleMode::Off => ModeSetting::Off,
                JiggleMode::Scheduled => ModeSetting::Scheduled,
                JiggleMode::Always => ModeSetting::Always,
            },
            days: settings.schedule.days.iter().map(Weekday::number).collect(),
            start_minutes: settings.schedule.start.minutes(),
            end_minutes: settings.schedule.end.minutes(),
            intensity: match settings.intensity {
                Intensity::Tiny => IntensitySetting::Tiny,
                Intensity::Gentle => IntensitySetting::Gentle,
                Intensity::Playful => IntensitySetting::Playful,
            },
            idle_interval_secs: settings.idle_interval,
        }
    }
}

impl From<SettingsFile> for Settings {
    fn from(file: SettingsFile) -> Self {
        Settings {
            mode: match file.mode {
                ModeSetting::Off => JiggleMode::Off,
                ModeSetting::Scheduled => JiggleMode::Scheduled,
                ModeSetting::Always => JiggleMode::Always,
            },
            schedule: Schedule::new(
                file.days.iter().filter_map(|&n| Weekday::from_number(n)).collect::<Days>(),
                TimeOfDay::new(file.start_minutes),
                TimeOfDay::new(file.end_minutes),
            ),
            intensity: match file.intensity {
                IntensitySetting::Tiny => Intensity::Tiny,
                IntensitySetting::Gentle => Intensity::Gentle,
                IntensitySetting::Playful => Intensity::Playful,
            },
            idle_interval: sanitize_interval(file.idle_interval_secs),
        }
    }
}

/// Rounded to whole seconds and kept within [`IDLE_RANGE`]; anything that is
/// not a number gives the default.
pub fn sanitize_interval(seconds: f64) -> f64 {
    if seconds.is_finite() {
        seconds.round().clamp(IDLE_RANGE.0, IDLE_RANGE.1)
    } else {
        DEFAULT_INTERVAL
    }
}

impl Settings {
    /// `%APPDATA%\MelCat\settings.json`.
    pub fn path() -> PathBuf {
        crate::util::paths::app_data_dir().join("settings.json")
    }

    /// Loads the settings file, falling back to defaults on any error.
    pub fn load() -> Settings {
        Self::load_from(&Self::path())
    }

    /// Loads `path`, falling back to defaults when it is missing or unreadable.
    pub fn load_from(path: &Path) -> Settings {
        match fs::read_to_string(path) {
            Ok(text) => Self::from_json(&text),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Settings::default(),
            Err(error) => {
                crate::log_warn!("could not read {}: {error}; using defaults", path.display());
                Settings::default()
            }
        }
    }

    /// Parses settings JSON leniently: unknown keys are ignored, a missing or
    /// invalid value falls back to its default without discarding the others,
    /// and anything that is not a JSON object gives the defaults.
    pub fn from_json(text: &str) -> Settings {
        if let Ok(file) = serde_json::from_str::<SettingsFile>(text) {
            return file.into();
        }
        let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(text) else {
            return Settings::default();
        };
        fn field<T: serde::de::DeserializeOwned>(
            map: &serde_json::Map<String, serde_json::Value>,
            key: &str,
            fallback: T,
        ) -> T {
            map.get(key).and_then(|value| serde_json::from_value(value.clone()).ok()).unwrap_or(fallback)
        }
        let defaults = SettingsFile::default();
        SettingsFile {
            mode: field(&map, "mode", defaults.mode),
            days: field(&map, "days", defaults.days),
            start_minutes: field(&map, "start_minutes", defaults.start_minutes),
            end_minutes: field(&map, "end_minutes", defaults.end_minutes),
            intensity: field(&map, "intensity", defaults.intensity),
            idle_interval_secs: field(&map, "idle_interval_secs", defaults.idle_interval_secs),
        }
        .into()
    }

    /// Pretty-printed JSON for the settings file.
    pub fn to_json(self) -> String {
        serde_json::to_string_pretty(&SettingsFile::from(&self)).unwrap_or_else(|_| "{}".to_owned())
    }

    /// Writes the settings file (via a temporary file, so a crash never leaves half a file).
    pub fn save(&self) -> io::Result<()> {
        self.save_to(&Self::path())
    }

    /// Writes the settings to `path`, creating its folder if needed.
    pub fn save_to(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let temporary = path.with_extension("json.tmp");
        fs::write(&temporary, self.to_json())?;
        fs::rename(&temporary, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_macos_app() {
        let settings = Settings::default();
        assert_eq!(settings.mode, JiggleMode::Scheduled);
        assert_eq!(settings.schedule, Schedule::WORK_HOURS);
        assert_eq!(settings.intensity, Intensity::Gentle);
        assert_eq!(settings.idle_interval, 60.0);
    }

    #[test]
    fn json_round_trip() {
        let settings = Settings {
            mode: JiggleMode::Always,
            schedule: Schedule::new(Days::WEEKEND, TimeOfDay::hm(22, 0), TimeOfDay::hm(6, 30)),
            intensity: Intensity::Playful,
            idle_interval: 120.0,
        };
        assert_eq!(Settings::from_json(&settings.to_json()), settings);
    }

    #[test]
    fn the_documented_shape_parses() {
        let text = r#"{ "mode": "Scheduled", "days": [2, 3, 4, 5, 6], "start_minutes": 540,
                        "end_minutes": 1080, "intensity": "Gentle", "idle_interval_secs": 60 }"#;
        assert_eq!(Settings::from_json(text), Settings::default());
    }

    #[test]
    fn malformed_json_gives_defaults() {
        assert_eq!(Settings::from_json("not json"), Settings::default());
        assert_eq!(Settings::from_json("[1, 2]"), Settings::default());
        assert_eq!(Settings::from_json(""), Settings::default());
    }

    #[test]
    fn one_bad_field_does_not_discard_the_others() {
        let settings = Settings::from_json(r#"{ "mode": "Always", "intensity": "Enormous", "idle_interval_secs": 30 }"#);
        assert_eq!(settings.mode, JiggleMode::Always);
        assert_eq!(settings.intensity, Intensity::Gentle);
        assert_eq!(settings.idle_interval, 30.0);
        assert_eq!(settings.schedule, Schedule::WORK_HOURS);
    }

    #[test]
    fn out_of_range_values_are_repaired() {
        let settings = Settings::from_json(
            r#"{ "days": [0, 1, 7, 9], "start_minutes": -60, "end_minutes": 1500, "idle_interval_secs": 1e19 }"#,
        );
        assert_eq!(settings.schedule.days, [Weekday::Sunday, Weekday::Saturday].into_iter().collect());
        assert_eq!(settings.schedule.start, TimeOfDay::hm(23, 0));
        assert_eq!(settings.schedule.end, TimeOfDay::hm(1, 0));
        assert_eq!(settings.idle_interval, IDLE_RANGE.1);
        assert_eq!(sanitize_interval(2.0), 5.0);
        assert_eq!(sanitize_interval(60.4), 60.0);
        assert_eq!(sanitize_interval(f64::NAN), DEFAULT_INTERVAL);
    }

    #[test]
    fn save_and_load_from_disk() {
        let dir = std::env::temp_dir().join(format!("melcat-settings-test-{}", std::process::id()));
        let path = dir.join("settings.json");
        let settings = Settings { idle_interval: 240.0, ..Settings::default() };
        settings.save_to(&path).expect("saved");
        assert_eq!(Settings::load_from(&path), settings);
        assert_eq!(Settings::load_from(&dir.join("missing.json")), Settings::default());
        let _ = fs::remove_dir_all(&dir);
    }
}
