//! Window messages, timers and user-facing sentences.
//!
//! MelCat has one thread. Its two windows (created in `app.rs`):
//! - `MelCat.Main`, a hidden top-level window: the tray icon callback
//!   ([`WM_APP_TRAY`]), `TaskbarCreated`, `WM_SETTINGCHANGE`,
//!   `WM_DISPLAYCHANGE`, `WM_WTSSESSION_CHANGE`, `WM_POWERBROADCAST` and the
//!   end of the session. Being top-level it also receives other processes'
//!   broadcasts, so it never trusts a message's parameters beyond its type.
//! - `MelCat.Msg`, a message-only window (`HWND_MESSAGE`, never reached by
//!   broadcasts): the timers ([`TICK_TIMER`], [`WALK_TIMER`]), raw input
//!   (`WM_INPUT`) and MelCat's own notifications ([`WM_APP_SETTINGS_CHANGED`],
//!   [`WM_APP_PREVIEW`]). None of them carries a pointer, so nothing a
//!   foreign process posts can be mistaken for memory MelCat owns.
//!
//! Text shown to the user (balloons, the status line) is made of whole
//! sentences. [`sentence`] is the one place that adds a missing final period,
//! so a message never ends in "..".

use windows::Win32::UI::WindowsAndMessaging::WM_APP;

/// Tray icon callback (`uCallbackMessage`); standard `Shell_NotifyIcon`
/// semantics with `NOTIFYICON_VERSION_4`. Sent to `MelCat.Main`.
pub const WM_APP_TRAY: u32 = WM_APP + 1;

/// The settings window changed `Settings` (already saved). No parameters.
/// Posted to `MelCat.Msg`; the controller re-reads the settings, refreshes
/// the jiggler and the tray.
pub const WM_APP_SETTINGS_CHANGED: u32 = WM_APP + 2;

/// The settings window's Preview button was pressed. No parameters. Posted
/// to `MelCat.Msg`; the controller asks the jiggler for one stroll now.
pub const WM_APP_PREVIEW: u32 = WM_APP + 3;

/// The settings window closed. No parameters. Posted to `MelCat.Msg`.
pub const WM_APP_SETTINGS_CLOSED: u32 = WM_APP + 4;

/// Timer on `MelCat.Msg`: the once-a-second check (`Jiggler::tick`).
pub const TICK_TIMER: usize = 1;
/// Milliseconds between two ticks.
pub const TICK_MILLIS: u32 = 1000;

/// Timer on `MelCat.Msg`: one stroll step (`Jiggler::on_walk_timer`), set
/// only while a stroll is under way.
pub const WALK_TIMER: usize = 2;
/// Milliseconds between two stroll steps. Windows rounds timers to its tick
/// (about 15.6 ms), so the jiggler picks each step by elapsed time, not by
/// counting timer messages.
pub const WALK_MILLIS: u32 = 16;

/// Timer on `MelCat.Main`: retries adding the tray icon while Explorer is not ready.
pub const TRAY_RETRY_TIMER: usize = 3;

/// `text` as a sentence for the user: trimmed, the first letter capitalized
/// and a final period added unless it already ends with `.`, `!`, `?` or `…`.
/// Idempotent, so passing a reason through it twice changes nothing.
pub fn sentence(text: &str) -> String {
    let text = text.trim();
    let mut chars = text.chars();
    let Some(first) = chars.next() else { return String::new() };
    let mut out: String = first.to_uppercase().chain(chars).collect();
    if !out.ends_with(['.', '!', '?', '\u{2026}']) {
        out.push('.');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentences_get_exactly_one_final_period() {
        assert_eq!(sentence("Could not write the Run key (error 5)"), "Could not write the Run key (error 5).");
        assert_eq!(sentence("  could not save the settings. "), "Could not save the settings.");
        assert_eq!(sentence("Really?"), "Really?");
        assert_eq!(sentence("\u{e9}t\u{e9}"), "\u{c9}t\u{e9}.");
        assert_eq!(sentence(""), "");
        for text in ["x", "Nothing moved.", "A (b)", "c\u{2026}"] {
            assert_eq!(sentence(&sentence(text)), sentence(text), "{text}");
            assert!(!sentence(text).ends_with(".."), "{text}");
        }
    }

    #[test]
    fn app_messages_are_distinct() {
        let ids = [WM_APP_TRAY, WM_APP_SETTINGS_CHANGED, WM_APP_PREVIEW, WM_APP_SETTINGS_CLOSED];
        for (i, a) in ids.iter().enumerate() {
            for b in &ids[i + 1..] {
                assert_ne!(a, b);
            }
        }
        assert_ne!(TICK_TIMER, WALK_TIMER);
    }
}
