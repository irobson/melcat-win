//! Physical-pixel geometry and monitor lookup.
//!
//! MelCat is per-monitor DPI aware (v2, from the manifest), so the pointer,
//! `SendInput` and the monitor rectangles all speak physical pixels of the
//! virtual desktop. The core crate's distances (2, 5, 12 pt radius, 4 pt edge
//! margin, walking speed) are logical points: convert with the monitor's
//! `scale` (effective DPI / 96) at the boundary, as [`MonitorInfo`] does.

use melcat_core::{Point, Rect};
use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, HMONITOR, MONITORINFO, MONITOR_DEFAULTTONEAREST};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PointPx {
    pub x: i32,
    pub y: i32,
}

impl PointPx {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Chebyshev distance in pixels: the larger of the horizontal and
    /// vertical gaps.
    pub fn max_gap(self, other: PointPx) -> i32 {
        (self.x - other.x).abs().max((self.y - other.y).abs())
    }
}

/// A rectangle in physical pixels: `left`/`top` inclusive, `right`/`bottom` exclusive.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RectPx {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl RectPx {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self { left, top, right, bottom }
    }
    pub fn width(&self) -> i32 {
        self.right - self.left
    }
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
    pub fn is_empty(&self) -> bool {
        self.width() <= 0 || self.height() <= 0
    }
    pub fn contains(&self, p: PointPx) -> bool {
        p.x >= self.left && p.x < self.right && p.y >= self.top && p.y < self.bottom
    }
    pub fn from_win(r: RECT) -> RectPx {
        RectPx::new(r.left, r.top, r.right, r.bottom)
    }
}

/// A display, in physical pixels of the virtual desktop.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MonitorInfo {
    /// Raw `HMONITOR` value.
    pub handle: isize,
    pub rect: RectPx,
    /// Effective DPI / 96: 1.0, 1.25, 1.5, 2.0, ...
    pub scale: f64,
}

impl MonitorInfo {
    /// The monitor containing (or nearest to) `point`.
    pub fn from_point(point: PointPx) -> MonitorInfo {
        // SAFETY: plain monitor queries into locals.
        unsafe {
            let monitor = MonitorFromPoint(POINT { x: point.x, y: point.y }, MONITOR_DEFAULTTONEAREST);
            Self::from_hmonitor(monitor)
        }
    }

    pub fn from_hmonitor(monitor: HMONITOR) -> MonitorInfo {
        // SAFETY: as above; failures leave the defaults (96 DPI).
        unsafe {
            let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
            let _ = GetMonitorInfoW(monitor, &mut info);
            let (mut dpi_x, mut dpi_y) = (96u32, 96u32);
            let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
            MonitorInfo { handle: monitor.0 as isize, rect: RectPx::from_win(info.rcMonitor), scale: dpi_x.max(96) as f64 / 96.0 }
        }
    }

    /// The monitor in logical points, relative to its own top-left corner:
    /// `(0, 0, width / scale, height / scale)`.
    pub fn logical_frame(self) -> Rect {
        Rect::new(0.0, 0.0, self.rect.width() as f64 / self.scale, self.rect.height() as f64 / self.scale)
    }

    /// A physical pixel as a logical point relative to the monitor.
    pub fn to_logical(self, point: PointPx) -> Point {
        to_logical(point, self.rect, self.scale)
    }

    /// A logical point relative to the monitor as the nearest physical pixel.
    pub fn to_physical(self, point: Point) -> PointPx {
        to_physical(point, self.rect, self.scale)
    }
}

/// See [`MonitorInfo::to_logical`]; separate so it can be tested without a display.
pub fn to_logical(point: PointPx, monitor: RectPx, scale: f64) -> Point {
    Point::new((point.x - monitor.left) as f64 / scale, (point.y - monitor.top) as f64 / scale)
}

/// See [`MonitorInfo::to_physical`].
pub fn to_physical(point: Point, monitor: RectPx, scale: f64) -> PointPx {
    PointPx::new(monitor.left + (point.x * scale).round() as i32, monitor.top + (point.y * scale).round() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_and_physical_round_trip_on_scaled_and_offset_monitors() {
        for (monitor, scale) in [
            (RectPx::new(0, 0, 1920, 1080), 1.0),
            (RectPx::new(-2560, -200, 0, 1240), 1.5),
            (RectPx::new(1920, 0, 5760, 2160), 2.0),
        ] {
            for point in [PointPx::new(monitor.left, monitor.top), PointPx::new(monitor.left + 77, monitor.top + 31)] {
                assert_eq!(to_physical(to_logical(point, monitor, scale), monitor, scale), point);
            }
        }
        assert_eq!(to_logical(PointPx::new(-2560 + 30, -200 + 15), RectPx::new(-2560, -200, 0, 1240), 1.5), Point::new(20.0, 10.0));
    }

    #[test]
    fn gaps_and_containment() {
        assert_eq!(PointPx::new(3, 4).max_gap(PointPx::new(0, 0)), 4);
        assert_eq!(PointPx::new(-3, 1).max_gap(PointPx::new(0, 0)), 3);
        let r = RectPx::new(0, 0, 10, 10);
        assert!(r.contains(PointPx::new(0, 0)) && !r.contains(PointPx::new(10, 5)));
    }
}
