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

use crate::geom::PointPx;

/// `dwExtraInfo` of every move MelCat sends: "MCAT".
pub(crate) const MARKER: usize = 0x4D43_4154;

/// Where the pointer is now.
pub(crate) fn cursor_pos() -> Option<PointPx> {
    todo!("CONTRACT: GetCursorPos")
}

/// Moves the pointer to `point`. Returns false if `SendInput` inserted nothing.
pub(crate) fn move_to(point: PointPx) -> bool {
    todo!("CONTRACT: see the module docs")
}

/// `value` (a pixel on an axis of the virtual desktop that starts at
/// `origin` and is `extent` pixels long) as `SendInput`'s 0..=65535 absolute
/// coordinate aimed at the pixel's center: `((value - origin) * 65536 +
/// 32768) / extent`, with `value` clamped onto the desktop first. 0 when
/// `extent` is not positive.
pub(crate) fn normalize(value: i32, origin: i32, extent: i32) -> i32 {
    todo!("CONTRACT: pure, unit tested")
}

/// The pixel that `normalize`d coordinate `coordinate` lands on: the inverse
/// of [`normalize`] (`origin + coordinate * extent / 65536`), for tests.
pub(crate) fn denormalize(coordinate: i32, origin: i32, extent: i32) -> i32 {
    todo!("CONTRACT: pure, unit tested")
}

/// Whether any mouse button is held.
pub(crate) fn is_button_down() -> bool {
    todo!("CONTRACT: GetAsyncKeyState")
}

/// Seconds since the last keyboard, mouse or touch input in this session,
/// MelCat's own moves included.
pub(crate) fn seconds_since_last_input() -> f64 {
    todo!("CONTRACT: GetLastInputInfo")
}

/// [`seconds_since_last_input`] from raw tick counts, wrapping around after
/// 49.7 days like `GetTickCount`. Pure, for tests.
pub(crate) fn idle_seconds(now_ticks: u32, last_input_ticks: u32) -> f64 {
    todo!("CONTRACT: pure, unit tested")
}
