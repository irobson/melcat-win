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
//!   [`WM_APP_PREVIEW`], [`WM_APP_SETTINGS_CLOSED`]). None of them carries a
//!   pointer, so nothing a foreign process posts can be mistaken for memory
//!   MelCat owns.
//!
//! Messages shown to the user (balloons, message boxes) are whole sentences,
//! each with one final period (or `!`, `?`, `…`, or a CJK full stop when the
//! text comes from Windows); the status line and the menu items are labels
//! without one, as on macOS. [`sentence`] is the one place that adds a
//! missing final period, so a message never ends in "..", and
//! [`io_error_text`] turns an I/O error into one clause for it.

use windows::Win32::UI::WindowsAndMessaging::WM_APP;

/// Tray icon callback (`uCallbackMessage`); standard `Shell_NotifyIcon`
/// semantics with `NOTIFYICON_VERSION_4`. Sent to `MelCat.Main`.
pub const WM_APP_TRAY: u32 = WM_APP + 1;

/// The settings window changed the shared `Settings` (not yet saved). No
/// parameters. Posted to `MelCat.Msg`; the controller saves them (a failure
/// is a warning balloon), ticks at once and updates the tray.
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

/// What ends a sentence: `.`, `!`, `?`, `…`, and the CJK full stops and
/// marks that Windows' own messages end with in Chinese or Japanese.
const FINAL_PUNCTUATION: [char; 9] =
    ['.', '!', '?', '\u{2026}', '\u{3002}', '\u{FF01}', '\u{FF1F}', '\u{FF0E}', '\u{FF61}'];

/// Full stops only (not `!`, `?` or `…`, which carry meaning).
const FULL_STOPS: [char; 4] = ['.', '\u{3002}', '\u{FF0E}', '\u{FF61}'];

/// `text` as a sentence for the user: trimmed, the first letter capitalized
/// and a final period added unless it already ends with one of
/// [`FINAL_PUNCTUATION`]. Idempotent, so passing a reason through it twice
/// changes nothing.
pub fn sentence(text: &str) -> String {
    let text = text.trim();
    let mut chars = text.chars();
    let Some(first) = chars.next() else { return String::new() };
    let mut out: String = first.to_uppercase().chain(chars).collect();
    if !out.ends_with(FINAL_PUNCTUATION) {
        out.push('.');
    }
    out
}

/// An I/O error as one clause for [`sentence`]: for an OS error, Windows'
/// text for it without its final full stop, then its code, as in "Access is
/// denied (error 5)". (`io::Error`'s own text, "Access is denied. (os error
/// 5)", has a period in the middle.) Any other error is its own text.
pub fn io_error_text(error: &std::io::Error) -> String {
    match error.raw_os_error() {
        Some(code) => with_code(&windows::core::HRESULT::from_win32(code as u32).message(), code),
        None => error.to_string(),
    }
}

/// `message` without its final full stop, followed by `(error {code})`, or
/// `Windows error {code}` when there is no message.
fn with_code(message: &str, code: i32) -> String {
    let message = message.trim().trim_end_matches(FULL_STOPS).trim_end();
    if message.is_empty() {
        format!("Windows error {code}")
    } else {
        format!("{message} (error {code})")
    }
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
        // Windows' messages in Chinese or Japanese end with a CJK full stop.
        assert_eq!(sentence("\u{62d2}\u{5426}\u{3002}"), "\u{62d2}\u{5426}\u{3002}");
        for mark in FINAL_PUNCTUATION {
            let text = format!("\u{62d2}\u{5426}{mark}");
            assert_eq!(sentence(&text), text, "{mark}");
        }
    }

    #[test]
    fn os_errors_read_as_one_clause() {
        assert_eq!(with_code("Access is denied.\r\n", 5), "Access is denied (error 5)");
        assert_eq!(with_code("\u{62d2}\u{5426}\u{3002}", 5), "\u{62d2}\u{5426} (error 5)");
        assert_eq!(with_code("Done!", 7), "Done! (error 7)");
        assert_eq!(with_code("  ", 1234), "Windows error 1234");
        assert_eq!(sentence(&with_code("Access is denied.", 5)), "Access is denied (error 5).");

        let denied = io_error_text(&std::io::Error::from_raw_os_error(5));
        assert!(denied.ends_with(" (error 5)") && !denied.contains(". ("), "{denied}");
        assert_eq!(io_error_text(&std::io::Error::other("disk full")), "disk full");
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
