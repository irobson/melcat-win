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
//!
//! What counts, as the macOS app's `keyDown`, `flagsChanged`, `scrollWheel`
//! and `*MouseDown`: a key press (no `RI_KEY_BREAK`), any change of a
//! modifier key (Shift, Ctrl, Alt, Windows, Caps Lock: press or release), a
//! mouse button press and the wheel. A key or button release alone does not:
//! its press came before the stroll, as when Enter presses Preview in the
//! settings window and is let go a moment later.

use std::ffi::c_void;
use std::mem::size_of;

use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VIRTUAL_KEY, VK_CAPITAL, VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_RCONTROL, VK_RMENU,
    VK_RSHIFT, VK_RWIN, VK_SHIFT,
};
use windows::Win32::UI::Input::{
    GetRawInputData, RegisterRawInputDevices, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE, RAWINPUTDEVICE_FLAGS,
    RAWINPUTHEADER, RAWKEYBOARD, RAWMOUSE, RIDEV_INPUTSINK, RIDEV_REMOVE, RID_INPUT, RIM_TYPEKEYBOARD, RIM_TYPEMOUSE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    RI_KEY_BREAK, RI_MOUSE_BUTTON_4_DOWN, RI_MOUSE_BUTTON_5_DOWN, RI_MOUSE_HWHEEL, RI_MOUSE_LEFT_BUTTON_DOWN,
    RI_MOUSE_MIDDLE_BUTTON_DOWN, RI_MOUSE_RIGHT_BUTTON_DOWN, RI_MOUSE_WHEEL,
};

/// HID usage page "Generic Desktop Controls".
const USAGE_PAGE_GENERIC: u16 = 1;
/// HID usage "Mouse" on that page.
const USAGE_MOUSE: u16 = 2;
/// HID usage "Keyboard" on that page.
const USAGE_KEYBOARD: u16 = 6;

/// The modifier keys, whose release counts too, like the macOS app's
/// `flagsChanged`. Raw input names Shift, Ctrl and Alt without their side.
const MODIFIERS: [VIRTUAL_KEY; 12] = [
    VK_SHIFT, VK_CONTROL, VK_MENU, VK_LSHIFT, VK_RSHIFT, VK_LCONTROL, VK_RCONTROL, VK_LMENU, VK_RMENU, VK_LWIN, VK_RWIN,
    VK_CAPITAL,
];

/// The `usButtonFlags` bits of a button press or a wheel turn.
const PRESS_OR_WHEEL: u16 = (RI_MOUSE_LEFT_BUTTON_DOWN
    | RI_MOUSE_RIGHT_BUTTON_DOWN
    | RI_MOUSE_MIDDLE_BUTTON_DOWN
    | RI_MOUSE_BUTTON_4_DOWN
    | RI_MOUSE_BUTTON_5_DOWN
    | RI_MOUSE_WHEEL
    | RI_MOUSE_HWHEEL) as u16;

/// The kind of device a raw input came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RawKind {
    Keyboard,
    Mouse,
    Other,
}

/// What a raw input record says, as far as telling a person goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RawEvent {
    /// A key: `RAWKEYBOARD.Flags` and `VKey`.
    Key { flags: u16, vkey: u16 },
    /// The mouse: `RAWMOUSE.usButtonFlags`.
    Mouse { button_flags: u16 },
    /// Any other device.
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

/// For a `WM_INPUT`'s `LPARAM`: true for input MelCat never produces and
/// that means a person, a key press, a modifier change, a button press or
/// the wheel ([`classify`]). Reads it with `GetRawInputData`; false if it
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
    let holds = |data: usize| copied as usize >= size_of::<RAWINPUTHEADER>() + data;
    let event = match kind_of(raw.header.dwType) {
        RawKind::Keyboard if holds(size_of::<RAWKEYBOARD>()) => {
            // SAFETY: the header says keyboard and a whole RAWKEYBOARD was
            // written after it, so the union holds one of plain integers.
            let keyboard = unsafe { raw.data.keyboard };
            RawEvent::Key { flags: keyboard.Flags, vkey: keyboard.VKey }
        }
        // A key whose details cannot be read counts as a press.
        RawKind::Keyboard => RawEvent::Key { flags: 0, vkey: 0 },
        RawKind::Mouse if holds(size_of::<RAWMOUSE>()) => {
            // SAFETY: the header says mouse and a whole RAWMOUSE was written after
            // it, so the union holds one; both union views are plain integers.
            RawEvent::Mouse { button_flags: unsafe { raw.data.mouse.Anonymous.Anonymous.usButtonFlags } }
        }
        RawKind::Mouse => RawEvent::Mouse { button_flags: 0 },
        RawKind::Other => RawEvent::Other,
    };
    classify(event)
}

/// The device kind a `RAWINPUTHEADER.dwType` names.
fn kind_of(raw_type: u32) -> RawKind {
    match raw_type {
        t if t == RIM_TYPEKEYBOARD.0 => RawKind::Keyboard,
        t if t == RIM_TYPEMOUSE.0 => RawKind::Mouse,
        _ => RawKind::Other,
    }
}

/// The decision behind [`is_personal_input`]: a key press (no
/// `RI_KEY_BREAK`), a modifier's press or release, and a mouse event with a
/// button press or the wheel among its flags. Releases alone are not. Pure.
pub(crate) fn classify(event: RawEvent) -> bool {
    match event {
        RawEvent::Key { flags, vkey } => {
            u32::from(flags) & RI_KEY_BREAK == 0 || MODIFIERS.iter().any(|modifier| modifier.0 == vkey)
        }
        RawEvent::Mouse { button_flags } => button_flags & PRESS_OR_WHEEL != 0,
        RawEvent::Other => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // `usButtonFlags` bits (RI_MOUSE_*).
    const LEFT_DOWN: u16 = 0x0001;
    const LEFT_UP: u16 = 0x0002;
    const RIGHT_DOWN: u16 = 0x0004;
    const RIGHT_UP: u16 = 0x0008;
    const MIDDLE_DOWN: u16 = 0x0010;
    const MIDDLE_UP: u16 = 0x0020;
    const X_BUTTON_1_DOWN: u16 = 0x0040;
    const X_BUTTON_1_UP: u16 = 0x0080;
    const X_BUTTON_2_DOWN: u16 = 0x0100;
    const X_BUTTON_2_UP: u16 = 0x0200;
    const WHEEL: u16 = 0x0400;
    const HORIZONTAL_WHEEL: u16 = 0x0800;

    // `RAWKEYBOARD.Flags` bits (RI_KEY_*).
    const MAKE: u16 = 0x00;
    const BREAK: u16 = 0x01;
    const E0: u16 = 0x02;

    // Virtual keys.
    const VK_RETURN: u16 = 0x0D;
    const VK_F13: u16 = 0x7C;
    const VK_A: u16 = 0x41;

    fn key(flags: u16, vkey: u16) -> bool {
        classify(RawEvent::Key { flags, vkey })
    }

    fn mouse(button_flags: u16) -> bool {
        classify(RawEvent::Mouse { button_flags })
    }

    #[test]
    fn key_presses_button_presses_and_the_wheel_are_a_person() {
        for vkey in [VK_RETURN, VK_F13, VK_A, 0] {
            assert!(key(MAKE, vkey), "{vkey:#04x}");
            assert!(key(MAKE | E0, vkey), "{vkey:#04x}");
        }
        for flags in [
            LEFT_DOWN,
            RIGHT_DOWN,
            MIDDLE_DOWN,
            X_BUTTON_1_DOWN,
            X_BUTTON_2_DOWN,
            WHEEL,
            HORIZONTAL_WHEEL,
            LEFT_DOWN | WHEEL,
            // A press and another button's release in one record.
            LEFT_UP | RIGHT_DOWN,
        ] {
            assert!(mouse(flags), "{flags:#06x}");
        }
    }

    #[test]
    fn a_modifier_counts_when_pressed_and_when_released() {
        for vkey in MODIFIERS.map(|modifier| modifier.0) {
            assert!(key(MAKE, vkey), "{vkey:#04x}");
            assert!(key(BREAK, vkey), "{vkey:#04x}");
            assert!(key(BREAK | E0, vkey), "{vkey:#04x}");
        }
        assert_eq!(MODIFIERS.map(|modifier| modifier.0), [
            0x10, 0x11, 0x12, 0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0x5B, 0x5C, 0x14
        ]);
    }

    #[test]
    fn releases_alone_are_not_a_person() {
        // Enter let go just after it pressed Preview, or any other key.
        for vkey in [VK_RETURN, VK_F13, VK_A] {
            assert!(!key(BREAK, vkey), "{vkey:#04x}");
            assert!(!key(BREAK | E0, vkey), "{vkey:#04x}");
        }
        for flags in [LEFT_UP, RIGHT_UP, MIDDLE_UP, X_BUTTON_1_UP, X_BUTTON_2_UP, LEFT_UP | RIGHT_UP] {
            assert!(!mouse(flags), "{flags:#06x}");
        }
        assert_eq!(PRESS_OR_WHEEL, 0x0D55);
    }

    #[test]
    fn plain_movement_and_other_devices_are_not() {
        // MelCat's own moves arrive as mouse input without any button flag.
        assert!(!mouse(0));
        assert!(!classify(RawEvent::Other));
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
