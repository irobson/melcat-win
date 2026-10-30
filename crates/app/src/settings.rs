//! Persisted preferences, the same as the macOS app's.
//!
//! JSON at `%APPDATA%\MelCat\settings.json`, loaded at start (defaults on any
//! error, field by field) and saved on every change. While MelCat runs, an
//! edit made to the file by hand or by another program applies within a
//! second (the controller reloads it on its next tick; MelCat's own saves do
//! not count), like a `defaults write` on macOS. Launch at login is not
//! stored here but in `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
//!
//! ```json
//! { "mode": "Scheduled", "days": [2, 3, 4, 5, 6], "start_minutes": 540,
//!   "end_minutes": 1080, "intensity": "Gentle", "idle_interval_secs": 60 }
//! ```
//!
//! Days are numbered from Sunday (1) to Saturday (7); times are minutes
//! since midnight. A hand-edited file is repaired on load: times wrap around
//! the clock, each entry of `days` that is not a day number is dropped (the
//! others stay), and the idle interval is rounded to whole seconds and kept
//! between 5 seconds and a day. The file is UTF-8, with or without a byte
//! order mark; UTF-16 with a byte order mark (what Windows PowerShell's
//! `Out-File` and `>` write) is read too. MelCat writes UTF-8 without one.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

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
/// free of serialization. `days` keeps any JSON value, so one entry that is
/// not a day number drops only itself.
#[derive(Clone, Debug, PartialEq, Serialize)]
struct SettingsFile {
    mode: ModeSetting,
    days: Vec<serde_json::Value>,
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
            days: settings.schedule.days.iter().map(|day| day.number().into()).collect(),
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
                file.days.iter().filter_map(weekday_of).collect::<Days>(),
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

/// A day number from the file (1 = Sunday ... 7 = Saturday), or `None` for
/// anything else. A whole number written as a float (`2.0`) counts, as an
/// `NSNumber` does on macOS.
fn weekday_of(value: &serde_json::Value) -> Option<Weekday> {
    let number = value.as_f64().filter(|n| n.fract() == 0.0 && (1.0..=7.0).contains(n))?;
    Weekday::from_number(number as u8)
}

/// The text of a settings file: UTF-16 after a UTF-16 byte order mark,
/// otherwise UTF-8 (invalid bytes become U+FFFD; a UTF-8 byte order mark is
/// dropped by [`Settings::parse`]).
fn decode(bytes: &[u8]) -> String {
    fn utf16(bytes: &[u8], unit: fn([u8; 2]) -> u16) -> String {
        let units: Vec<u16> = bytes.as_chunks::<2>().0.iter().map(|&pair| unit(pair)).collect();
        String::from_utf16_lossy(&units)
    }
    match bytes {
        [0xFF, 0xFE, rest @ ..] => utf16(rest, u16::from_le_bytes),
        [0xFE, 0xFF, rest @ ..] => utf16(rest, u16::from_be_bytes),
        _ => String::from_utf8_lossy(bytes).into_owned(),
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

    /// Loads `path`, falling back to defaults when it is missing, unreadable
    /// or not a JSON object (the last two with a warning in the log).
    pub fn load_from(path: &Path) -> Settings {
        match Self::read_from(path) {
            Ok(settings) => settings.unwrap_or_default(),
            Err(reason) => {
                crate::log_warn!("{reason}; using the defaults");
                Settings::default()
            }
        }
    }

    /// Reads `path`: `Ok(None)` when it does not exist, `Err` with the reason
    /// (for the log) when it cannot be read or is not a JSON object.
    pub fn read_from(path: &Path) -> Result<Option<Settings>, String> {
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(format!("could not read {}: {error}", path.display())),
        };
        match Self::parse(&decode(&bytes)) {
            Some(settings) => Ok(Some(settings)),
            None => Err(format!("{} is not a JSON object", path.display())),
        }
    }

    /// The modification time and size of `path`, which change with each
    /// write; `None` when it does not exist or cannot be read.
    pub fn stamp_of(path: &Path) -> Option<(SystemTime, u64)> {
        let metadata = fs::metadata(path).ok()?;
        Some((metadata.modified().ok()?, metadata.len()))
    }

    /// [`Settings::parse`], with the defaults for anything that is not a JSON
    /// object. MelCat itself reads through [`Settings::read_from`].
    #[cfg(test)]
    pub fn from_json(text: &str) -> Settings {
        Self::parse(text).unwrap_or_default()
    }

    /// Parses settings JSON leniently: a leading byte order mark and unknown
    /// keys are ignored, and a missing or invalid value falls back to its
    /// default without discarding the others. `None` when `text` is not a
    /// JSON object.
    fn parse(text: &str) -> Option<Settings> {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(text) else {
            return None;
        };
        fn field<T: serde::de::DeserializeOwned>(
            map: &serde_json::Map<String, serde_json::Value>,
            key: &str,
            fallback: T,
        ) -> T {
            map.get(key).and_then(|value| serde_json::from_value(value.clone()).ok()).unwrap_or(fallback)
        }
        let defaults = SettingsFile::default();
        Some(
            SettingsFile {
                mode: field(&map, "mode", defaults.mode),
                days: field(&map, "days", defaults.days),
                start_minutes: field(&map, "start_minutes", defaults.start_minutes),
                end_minutes: field(&map, "end_minutes", defaults.end_minutes),
                intensity: field(&map, "intensity", defaults.intensity),
                idle_interval_secs: field(&map, "idle_interval_secs", defaults.idle_interval_secs),
            }
            .into(),
        )
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
    fn one_bad_day_number_drops_only_that_entry() {
        let days = |json: &str| Settings::from_json(&format!(r#"{{ "days": {json}, "idle_interval_secs": 30 }}"#));
        let set = |list: &[Weekday]| list.iter().copied().collect::<Days>();
        assert_eq!(days("[1, 7, 300]").schedule.days, set(&[Weekday::Sunday, Weekday::Saturday]));
        assert_eq!(days("[-1, 2]").schedule.days, set(&[Weekday::Monday]));
        assert_eq!(days("[2.0, 3]").schedule.days, set(&[Weekday::Monday, Weekday::Tuesday]));
        assert_eq!(days(r#"[2.5, "3", null, [4], 4, 1e300]"#).schedule.days, set(&[Weekday::Wednesday]));
        assert_eq!(days("[]").schedule.days, Days::EMPTY);
        // Not a list at all: the default days, the other fields kept.
        let settings = days(r#""Mon""#);
        assert_eq!(settings.schedule, Schedule::WORK_HOURS);
        assert_eq!(settings.idle_interval, 30.0);
    }

    const ALWAYS: &str = r#"{ "mode": "Always", "days": [1, 7], "intensity": "Playful", "idle_interval_secs": 5 }"#;

    fn always() -> Settings {
        Settings {
            mode: JiggleMode::Always,
            schedule: Schedule { days: Days::WEEKEND, ..Schedule::WORK_HOURS },
            intensity: Intensity::Playful,
            idle_interval: 5.0,
        }
    }

    fn utf16(text: &str, big_endian: bool) -> Vec<u8> {
        let bom: &[u8] = if big_endian { &[0xFE, 0xFF] } else { &[0xFF, 0xFE] };
        let bytes = |unit: u16| if big_endian { unit.to_be_bytes() } else { unit.to_le_bytes() };
        let units = text.encode_utf16().flat_map(bytes);
        bom.iter().copied().chain(units).collect()
    }

    #[test]
    fn a_byte_order_mark_is_not_part_of_the_json() {
        assert_eq!(Settings::from_json(&format!("\u{feff}{ALWAYS}")), always());
        let utf8_bom = [&[0xEF, 0xBB, 0xBF][..], ALWAYS.as_bytes()].concat();
        assert_eq!(Settings::from_json(&decode(&utf8_bom)), always());
        assert_eq!(decode(&utf16(ALWAYS, false)), ALWAYS);
        assert_eq!(decode(&utf16(ALWAYS, true)), ALWAYS);
        assert_eq!(decode(ALWAYS.as_bytes()), ALWAYS);
        // Only one leading mark is dropped; a second one is not JSON.
        assert_eq!(Settings::parse(&format!("\u{feff}\u{feff}{ALWAYS}")), None);
    }

    #[test]
    fn files_with_a_byte_order_mark_load_from_disk() {
        let dir = std::env::temp_dir().join(format!("melcat-settings-bom-test-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temporary folder");
        let cases = [
            ("utf8-bom.json", [&[0xEF, 0xBB, 0xBF][..], ALWAYS.as_bytes()].concat()),
            ("utf16le.json", utf16(ALWAYS, false)),
            ("utf16be.json", utf16(ALWAYS, true)),
        ];
        for (name, bytes) in cases {
            let path = dir.join(name);
            fs::write(&path, bytes).expect("written");
            assert_eq!(Settings::load_from(&path), always(), "{name}");
            assert_eq!(Settings::read_from(&path), Ok(Some(always())), "{name}");
        }
        let garbage = dir.join("garbage.json");
        fs::write(&garbage, "not json").expect("written");
        assert_eq!(Settings::load_from(&garbage), Settings::default());
        let reason = Settings::read_from(&garbage).expect_err("not a JSON object");
        assert!(reason.ends_with("garbage.json is not a JSON object"), "{reason}");
        assert_eq!(Settings::read_from(&dir.join("missing.json")), Ok(None));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_stamp_changes_with_each_write() {
        let dir = std::env::temp_dir().join(format!("melcat-settings-stamp-test-{}", std::process::id()));
        let path = dir.join("settings.json");
        assert_eq!(Settings::stamp_of(&path), None);
        Settings::default().save_to(&path).expect("saved");
        let first = Settings::stamp_of(&path).expect("a stamp");
        assert_eq!(Settings::stamp_of(&path), Some(first), "reading does not change it");
        always().save_to(&path).expect("saved");
        assert_ne!(Settings::stamp_of(&path), Some(first));
        let _ = fs::remove_dir_all(&dir);
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
