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

use std::ffi::c_void;
use std::mem::size_of;

use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::UI::Input::{
    GetRawInputData, RegisterRawInputDevices, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE, RAWINPUTDEVICE_FLAGS,
    RAWINPUTHEADER, RAWMOUSE, RIDEV_INPUTSINK, RIDEV_REMOVE, RID_INPUT, RIM_TYPEKEYBOARD, RIM_TYPEMOUSE,
};

/// HID usage page "Generic Desktop Controls".
const USAGE_PAGE_GENERIC: u16 = 1;
/// HID usage "Mouse" on that page.
const USAGE_MOUSE: u16 = 2;
/// HID usage "Keyboard" on that page.
const USAGE_KEYBOARD: u16 = 6;

/// The kind of device a raw input came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RawKind {
    Keyboard,
    Mouse,
    Other,
}

/// Starts raw input for `target` (the `MelCat.Msg` window).
pub(crate) fn register(target: HWND) -> windows::core::Result<()> {
    // No RIDEV_NOLEGACY and no RIDEV_NOHOTKEYS: every window, MelCat's own
    // included, keeps getting its ordinary keyboard and mouse messages.
    let devices = devices(RIDEV_INPUTSINK, target);
    // SAFETY: a valid slice of fully initialized RAWINPUTDEVICE structures,
    // with the size of one; `target` is a window of this thread.
    unsafe { RegisterRawInputDevices(&devices, size_of::<RAWINPUTDEVICE>() as u32) }
}

/// Stops raw input (`RIDEV_REMOVE` for both devices).
pub(crate) fn unregister() {
    // RIDEV_REMOVE requires a null target.
    let devices = devices(RIDEV_REMOVE, HWND::default());
    // SAFETY: as in `register`.
    if let Err(error) = unsafe { RegisterRawInputDevices(&devices, size_of::<RAWINPUTDEVICE>() as u32) } {
        crate::log_warn!("could not stop raw input: {error}");
    }
}

/// The keyboard and the mouse, both with `flags` and `target`.
fn devices(flags: RAWINPUTDEVICE_FLAGS, target: HWND) -> [RAWINPUTDEVICE; 2] {
    [USAGE_KEYBOARD, USAGE_MOUSE].map(|usage| RAWINPUTDEVICE {
        usUsagePage: USAGE_PAGE_GENERIC,
        usUsage: usage,
        dwFlags: flags,
        hwndTarget: target,
    })
}

/// For a `WM_INPUT`'s `LPARAM`: true for input MelCat never produces, a key,
/// a mouse button or the wheel. Reads it with `GetRawInputData`; false if it
/// cannot be read.
pub(crate) fn is_personal_input(lparam: LPARAM) -> bool {
    let mut raw = RAWINPUT::default();
    let mut size = size_of::<RAWINPUT>() as u32;
    // SAFETY: `raw` is a writable buffer of `size` bytes. Only keyboards and
    // mice are registered, and their data always fits in a RAWINPUT; anything
    // bigger, or a handle that is not (or no longer) raw input, makes the call
    // fail with (UINT)-1 without writing past the buffer. Reading never
    // removes the input from anyone: `DefWindowProcW` still gets the message.
    let copied = unsafe {
        GetRawInputData(
            HRAWINPUT(lparam.0 as *mut c_void),
            RID_INPUT,
            Some(&mut raw as *mut RAWINPUT as *mut c_void),
            &mut size,
            size_of::<RAWINPUTHEADER>() as u32,
        )
    };
    if copied == u32::MAX || (copied as usize) < size_of::<RAWINPUTHEADER>() {
        return false;
    }
    let kind = kind_of(raw.header.dwType);
    let button_flags =
        if kind == RawKind::Mouse && copied as usize >= size_of::<RAWINPUTHEADER>() + size_of::<RAWMOUSE>() {
            // SAFETY: the header says mouse and a whole RAWMOUSE was written after
            // it, so the union holds one; both union views are plain integers.
            unsafe { raw.data.mouse.Anonymous.Anonymous.usButtonFlags }
        } else {
            0
        };
    classify(kind, button_flags)
}

/// The device kind a `RAWINPUTHEADER.dwType` names.
fn kind_of(raw_type: u32) -> RawKind {
    match raw_type {
        t if t == RIM_TYPEKEYBOARD.0 => RawKind::Keyboard,
        t if t == RIM_TYPEMOUSE.0 => RawKind::Mouse,
        _ => RawKind::Other,
    }
}

/// The decision behind [`is_personal_input`]: every keyboard event, and any
/// mouse event with button or wheel flags (`usButtonFlags != 0`). Pure.
pub(crate) fn classify(kind: RawKind, mouse_button_flags: u16) -> bool {
    match kind {
        RawKind::Keyboard => true,
        RawKind::Mouse => mouse_button_flags != 0,
        RawKind::Other => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // `usButtonFlags` bits (RI_MOUSE_*).
    const LEFT_DOWN: u16 = 0x0001;
    const RIGHT_UP: u16 = 0x0008;
    const X_BUTTON_2_DOWN: u16 = 0x0100;
    const WHEEL: u16 = 0x0400;
    const HORIZONTAL_WHEEL: u16 = 0x0800;

    #[test]
    fn keys_buttons_and_the_wheel_are_a_person() {
        assert!(classify(RawKind::Keyboard, 0));
        for flags in [LEFT_DOWN, RIGHT_UP, X_BUTTON_2_DOWN, WHEEL, HORIZONTAL_WHEEL, LEFT_DOWN | WHEEL] {
            assert!(classify(RawKind::Mouse, flags), "{flags:#06x}");
        }
    }

    #[test]
    fn plain_movement_and_other_devices_are_not() {
        // MelCat's own moves arrive as mouse input without any button flag.
        assert!(!classify(RawKind::Mouse, 0));
        assert!(!classify(RawKind::Other, 0));
        assert!(!classify(RawKind::Other, LEFT_DOWN));
    }

    #[test]
    fn header_types_map_to_kinds() {
        assert_eq!(kind_of(RIM_TYPEMOUSE.0), RawKind::Mouse);
        assert_eq!(kind_of(RIM_TYPEKEYBOARD.0), RawKind::Keyboard);
        assert_eq!(kind_of(2), RawKind::Other);
        assert_eq!(kind_of(u32::MAX), RawKind::Other);
    }

    #[test]
    fn both_devices_are_generic_desktop_keyboard_and_mouse() {
        let registered = devices(RIDEV_INPUTSINK, HWND::default());
        assert_eq!(registered.map(|d| (d.usUsagePage, d.usUsage)), [(1, 6), (1, 2)]);
        assert!(registered.iter().all(|d| d.dwFlags == RIDEV_INPUTSINK));
        assert!(devices(RIDEV_REMOVE, HWND::default()).iter().all(|d| d.dwFlags == RIDEV_REMOVE));
    }
}
