//! MelCat for Windows: a gentle, scheduled mouse jiggler in the system tray.
//!
//! On the days and hours you choose (or always), after a stretch without any
//! keyboard or mouse input, MelCat walks the pointer a few points along a
//! slow, slightly curved path around where you left it, and lets go the
//! moment you use the mouse or keyboard. The same behavior as the macOS app.
//!
//! Module map (see docs/ARCHITECTURE.md):
//! - `app`      UI-thread controller: windows, timers, tray, wiring.
//! - `jiggle`   the jiggler: idle check, strolls, input and session watching.
//! - `ui`       tray icon and menu, settings window, launch at login, text.
//! - `settings` persisted preferences.
//! - `geom`, `messages`, `util`  shared plumbing.
//!
//! Everything runs on one thread, the UI thread, like the macOS app's main
//! actor. No other thread exists.

#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod app;
#[cfg(windows)]
mod geom;
#[cfg(windows)]
mod jiggle;
#[cfg(windows)]
mod messages;
#[cfg(windows)]
mod settings;
#[cfg(windows)]
mod ui;
#[cfg(windows)]
mod util;

#[cfg(windows)]
fn main() {
    std::process::exit(app::run());
}

#[cfg(not(windows))]
fn main() {
    eprintln!("MelCat for Windows only runs on Windows. Build with --target x86_64-pc-windows-gnu or on Windows.");
    std::process::exit(1);
}
