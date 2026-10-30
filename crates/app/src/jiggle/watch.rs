//! Raw input, to notice a person during a stroll.
//!
//! CONTRACT (owned by the jiggle agent). Purely passive: `RegisterRawInputDevices`
//! for the keyboard (usage page 1, usage 6) and the mouse (page 1, usage 2)
//! with `RIDEV_INPUTSINK` and the `MelCat.Msg` window as target, so
//! `WM_INPUT` arrives while MelCat is in the background. Nothing is ever held
//! or swallowed: every `WM_INPUT` still goes to `DefWindowProcW`, and other
//! apps see the input exactly as before. No hook is installed.
//!
//! Why only keys, buttons and the wheel: MelCat never sends any of those, so
//! every one of them is a person (or another program acting for one,
//! including the smoke test's `SendInput`). Plain movement is left to the
//! jiggler's cursor check, exactly like the macOS app, so MelCat never has to
//! tell its own moves apart from anyone else's.

use windows::Win32::Foundation::{HWND, LPARAM};

/// The kind of device a raw input came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RawKind {
    Keyboard,
    Mouse,
    Other,
}

/// Starts raw input for `target` (the `MelCat.Msg` window).
pub(crate) fn register(target: HWND) -> windows::core::Result<()> {
    todo!("CONTRACT: RegisterRawInputDevices, RIDEV_INPUTSINK")
}

/// Stops raw input (`RIDEV_REMOVE` for both devices).
pub(crate) fn unregister() {
    todo!("CONTRACT: see the module docs")
}

/// For a `WM_INPUT`'s `LPARAM`: true for input MelCat never produces, a key,
/// a mouse button or the wheel. Reads it with `GetRawInputData`; false if it
/// cannot be read.
pub(crate) fn is_personal_input(lparam: LPARAM) -> bool {
    todo!("CONTRACT: GetRawInputData, then `classify`")
}

/// The decision behind [`is_personal_input`]: every keyboard event, and any
/// mouse event with button or wheel flags (`usButtonFlags != 0`). Pure.
pub(crate) fn classify(kind: RawKind, mouse_button_flags: u16) -> bool {
    todo!("CONTRACT: pure, unit tested")
}
