//! Everything MelCat reads from, and writes to, the pointer.
//!
//! CONTRACT (owned by the jiggle agent). Coordinates are physical pixels of
//! the virtual desktop (`geom`), as `GetCursorPos` and `SendInput` use them.
//!
//! - [`move_to`] sends one `INPUT_MOUSE` with `MOUSEEVENTF_MOVE |
//!   MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK`, `time` 0 (the system
//!   stamps it) and `dwExtraInfo` [`MARKER`], at coordinates normalized to the
//!   center of the target pixel over `SM_XVIRTUALSCREEN`/`SM_CXVIRTUALSCREEN`
//!   (and Y): [`normalize`]. That lands exactly on the pixel on any monitor
//!   layout, negative origins included. Injected input goes into the session
//!   input stream like hardware input, so it resets the idle time that
//!   `GetLastInputInfo` reports: that is what keeps the computer awake.
//!   UIPI silently drops injected input aimed at an elevated window; for a
//!   pointer move that is harmless (the pointer still moves) and is documented.
//! - [`seconds_since_last_input`] is `GetTickCount() - LASTINPUTINFO.dwTime`
//!   (wrapping arithmetic) in seconds: any input in the session, MelCat's own
//!   moves included.
//! - [`is_button_down`] checks the left, right, middle and both X buttons with
//!   `GetAsyncKeyState` (high bit).

use windows::Win32::Foundation::{GetLastError, POINT};
use windows::Win32::System::SystemInformation::GetTickCount;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetLastInputInfo, SendInput, INPUT, INPUT_0, INPUT_MOUSE, LASTINPUTINFO, MOUSEEVENTF_ABSOLUTE,
    MOUSEEVENTF_MOVE, MOUSEEVENTF_VIRTUALDESK, MOUSEINPUT, VIRTUAL_KEY, VK_LBUTTON, VK_MBUTTON, VK_RBUTTON,
    VK_XBUTTON1, VK_XBUTTON2,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

use crate::geom::PointPx;

/// `dwExtraInfo` of every move MelCat sends: "MCAT".
pub(crate) const MARKER: usize = 0x4D43_4154;

/// `SendInput`'s absolute coordinates run from 0 to 65535 across the desktop.
const ABSOLUTE_SPAN: i64 = 65_536;

/// The mouse buttons [`is_button_down`] looks at. `GetAsyncKeyState` reads the
/// physical buttons, so a swapped primary button changes nothing here.
const BUTTONS: [VIRTUAL_KEY; 5] = [VK_LBUTTON, VK_RBUTTON, VK_MBUTTON, VK_XBUTTON1, VK_XBUTTON2];

/// Where the pointer is now.
pub(crate) fn cursor_pos() -> Option<PointPx> {
    let mut point = POINT::default();
    // SAFETY: `point` is a valid out parameter. The call fails (for example
    // while another desktop has the input) without touching it.
    unsafe { GetCursorPos(&mut point) }.ok()?;
    Some(PointPx::new(point.x, point.y))
}

/// Moves the pointer to `point`. Returns false if `SendInput` inserted nothing.
pub(crate) fn move_to(point: PointPx) -> bool {
    // The layout can change at any time; read it for every move. Both calls
    // answer in physical pixels, as MelCat is per-monitor DPI aware.
    // SAFETY: GetSystemMetrics has no preconditions.
    let (left, top, width, height) = unsafe {
        (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        )
    };
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: normalize(point.x, left, width),
                dy: normalize(point.y, top, height),
                mouseData: 0,
                dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
                time: 0,
                dwExtraInfo: MARKER,
            },
        },
    };
    // SAFETY: a one-element slice of a fully initialized INPUT, with its size.
    let inserted = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
    if inserted == 0 {
        // Once per stroll at most: the jiggler stops the stroll on a failed move.
        // SAFETY: GetLastError has no preconditions.
        let error = unsafe { GetLastError() };
        crate::log_warn!("SendInput inserted no pointer move (error {})", error.0);
        return false;
    }
    true
}

/// `value` (a pixel on an axis of the virtual desktop that starts at
/// `origin` and is `extent` pixels long) as `SendInput`'s 0..=65535 absolute
/// coordinate aimed at the pixel's center: `((value - origin) * 65536 +
/// 32768) / extent`, with `value` clamped onto the desktop first. 0 when
/// `extent` is not positive.
pub(crate) fn normalize(value: i32, origin: i32, extent: i32) -> i32 {
    if extent <= 0 {
        return 0;
    }
    let extent = i64::from(extent);
    let offset = (i64::from(value) - i64::from(origin)).clamp(0, extent - 1);
    // Aiming at the center lands on this pixel when Windows truncates on the
    // way back, and at most one pixel off if it rounds over `extent - 1`.
    // The result never exceeds 65535 (it is at most 65536 - 32768 / extent,
    // rounded down); the clamp only states it.
    ((offset * ABSOLUTE_SPAN + ABSOLUTE_SPAN / 2) / extent).clamp(0, ABSOLUTE_SPAN - 1) as i32
}

/// The pixel that `normalize`d coordinate `coordinate` lands on: the inverse
/// of [`normalize`] (`origin + coordinate * extent / 65536`), for tests.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn denormalize(coordinate: i32, origin: i32, extent: i32) -> i32 {
    if extent <= 0 {
        return origin;
    }
    let offset = (i64::from(coordinate) * i64::from(extent)).div_euclid(ABSOLUTE_SPAN);
    (i64::from(origin) + offset) as i32
}

/// Whether any mouse button is held.
pub(crate) fn is_button_down() -> bool {
    BUTTONS.iter().any(|button| {
        // SAFETY: GetAsyncKeyState has no preconditions; it answers 0 when the
        // input desktop is not this one, which reads as "not held".
        is_held(unsafe { GetAsyncKeyState(i32::from(button.0)) })
    })
}

/// A `GetAsyncKeyState` result says the key or button is down when its high
/// bit is set, which makes the `i16` negative. The low bit ("pressed since the
/// last call") is shared with every other caller and ignored.
fn is_held(state: i16) -> bool {
    state < 0
}

/// Seconds since the last keyboard, mouse or touch input in this session,
/// MelCat's own moves included.
pub(crate) fn seconds_since_last_input() -> f64 {
    let mut info = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
    // SAFETY: `info` is a valid LASTINPUTINFO with cbSize set.
    if !unsafe { GetLastInputInfo(&mut info) }.as_bool() {
        // Unknown reads as "input just now": MelCat then waits instead of
        // moving the pointer under someone's hand.
        return 0.0;
    }
    // Read the clock after the last input time, so the input can never look
    // later than "now" (which would wrap around to 49 days).
    // SAFETY: GetTickCount has no preconditions.
    let now = unsafe { GetTickCount() };
    idle_seconds(now, info.dwTime)
}

/// [`seconds_since_last_input`] from raw tick counts, wrapping around after
/// 49.7 days like `GetTickCount`. Pure, for tests.
pub(crate) fn idle_seconds(now_ticks: u32, last_input_ticks: u32) -> f64 {
    f64::from(now_ticks.wrapping_sub(last_input_ticks)) / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Virtual desktops as (origin, extent) on one axis: a single monitor,
    /// monitors left of and above the primary (negative origins), odd and
    /// prime extents, a one-pixel axis and the widest axis that still fits
    /// two coordinates per pixel.
    const LAYOUTS: [(i32, i32); 10] = [
        (0, 1920),
        (0, 1080),
        (-1920, 3840),
        (-2560, 6400),
        (-1080, 3000),
        (-1000, 1001),
        (-3, 7),
        (17, 1366),
        (0, 1),
        (-16_384, 32_768),
    ];

    #[test]
    fn every_pixel_round_trips_through_absolute_coordinates() {
        for (origin, extent) in LAYOUTS {
            for value in origin..origin + extent {
                let n = normalize(value, origin, extent);
                assert!((0..=65_535).contains(&n), "origin {origin} extent {extent} value {value}: {n}");
                assert_eq!(denormalize(n, origin, extent), value, "origin {origin} extent {extent}");
            }
        }
    }

    /// The other mapping Windows is commonly described to use, rounding over
    /// `extent - 1`, lands at most one pixel away: within `TAKEOVER_PX`, so the
    /// jiggler never mistakes its own landing for a hand on the mouse.
    #[test]
    fn the_rounding_mapping_lands_within_one_pixel() {
        for (origin, extent) in LAYOUTS {
            for value in origin..origin + extent {
                let n = i64::from(normalize(value, origin, extent));
                let landed = i64::from(origin) + ((n * i64::from(extent - 1)) as f64 / 65_535.0).round() as i64;
                assert!((landed - i64::from(value)).abs() <= 1, "origin {origin} extent {extent} value {value}");
            }
        }
    }

    #[test]
    fn coordinates_are_clamped_onto_the_desktop() {
        assert_eq!(normalize(-50, 0, 1920), normalize(0, 0, 1920));
        assert_eq!(normalize(5_000, 0, 1920), normalize(1919, 0, 1920));
        assert_eq!(normalize(-5_000, -1920, 3840), normalize(-1920, -1920, 3840));
        assert_eq!(normalize(i32::MIN, -1920, 3840), normalize(-1920, -1920, 3840));
        assert_eq!(normalize(i32::MAX, -1920, 3840), normalize(1919, -1920, 3840));
        assert_eq!(normalize(0, 0, 1920), 17);
        assert_eq!(normalize(1919, 0, 1920), 65_518);
        assert_eq!(normalize(0, 0, 1), 32_768);
    }

    #[test]
    fn an_empty_desktop_gives_zero() {
        assert_eq!(normalize(10, 0, 0), 0);
        assert_eq!(normalize(10, 0, -5), 0);
        assert_eq!(denormalize(100, 7, 0), 7);
    }

    #[test]
    fn idle_time_wraps_like_the_tick_count() {
        assert_eq!(idle_seconds(5_000, 5_000), 0.0);
        assert_eq!(idle_seconds(65_250, 5_000), 60.25);
        // GetTickCount wrapped 1 s after the last input, which came 0.995 s before the wrap.
        assert_eq!(idle_seconds(5, u32::MAX - 994), 1.0);
        assert_eq!(idle_seconds(0, 1), f64::from(u32::MAX) / 1000.0);
    }

    #[test]
    fn only_the_high_bit_means_held() {
        assert!(is_held(i16::MIN));
        assert!(is_held(i16::MIN | 1));
        assert!(!is_held(0));
        assert!(!is_held(1));
        assert!(!is_held(i16::MAX));
    }
}
