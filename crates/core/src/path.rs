//! How far and how fast the pointer wanders, and the stroll planner.
//!
//! Distances are in logical points (pixels at 100 % scale). The app converts
//! to physical pixels with the monitor's scale.

use std::f64::consts::{PI, TAU};

use crate::rng::RandomSource;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const fn new(x: f64, y: f64) -> Point {
        Point { x, y }
    }

    pub fn distance_to(self, other: Point) -> f64 {
        (self.x - other.x).hypot(self.y - other.y)
    }

    /// This point, pulled straight towards `center` until it is no farther than `limit`.
    pub fn limited_to(self, limit: f64, center: Point) -> Point {
        let d = self.distance_to(center);
        if d <= limit || d <= 0.0 {
            return self;
        }
        let scale = limit / d;
        Point::new(center.x + (self.x - center.x) * scale, center.y + (self.y - center.y) * scale)
    }

    /// This point, moved inside `rect` (edges included).
    pub fn clamped_to(self, rect: Rect) -> Point {
        Point::new(self.x.clamp(rect.min_x, rect.max_x), self.y.clamp(rect.min_y, rect.max_y))
    }
}

/// An axis-aligned rectangle, edges included.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl Rect {
    /// The rectangle at `x`, `y` of `width` by `height`.
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Rect {
        Rect { min_x: x.min(x + width), min_y: y.min(y + height), max_x: x.max(x + width), max_y: y.max(y + height) }
    }

    /// `dx` and `dy` in from every side, or `None` if nothing is left.
    pub fn inset(self, dx: f64, dy: f64) -> Option<Rect> {
        let rect = Rect { min_x: self.min_x + dx, min_y: self.min_y + dy, max_x: self.max_x - dx, max_y: self.max_y - dy };
        (rect.min_x <= rect.max_x && rect.min_y <= rect.max_y).then_some(rect)
    }

    pub fn contains(self, point: Point) -> bool {
        (self.min_x..=self.max_x).contains(&point.x) && (self.min_y..=self.max_y).contains(&point.y)
    }
}

/// How far and how fast the pointer wanders.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Intensity {
    Tiny,
    #[default]
    Gentle,
    Playful,
}

impl Intensity {
    pub const ALL: [Intensity; 3] = [Intensity::Tiny, Intensity::Gentle, Intensity::Playful];

    /// Farthest the pointer ever gets from where it was left, in points.
    /// A pointer left on a screen edge is first moved inside the margin the
    /// walk keeps from the edges, and the radius counts from there.
    pub fn radius(self) -> f64 {
        match self {
            Intensity::Tiny => 2.0,
            Intensity::Gentle => 5.0,
            Intensity::Playful => 12.0,
        }
    }

    /// Walking speed, in points per second. Slow on purpose: a stroll, not a twitch.
    pub fn speed(self) -> f64 {
        match self {
            Intensity::Tiny => 2.0,
            Intensity::Gentle => 3.0,
            Intensity::Playful => 6.0,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Intensity::Tiny => "Tiny",
            Intensity::Gentle => "Gentle",
            Intensity::Playful => "Playful",
        }
    }
}

/// One stroll of the pointer: the positions to visit, one per step.
#[derive(Clone, Debug, PartialEq)]
pub struct JiggleWalk {
    pub start: Point,
    /// Positions after `start`, in order. The last one is the target.
    pub points: Vec<Point>,
}

impl JiggleWalk {
    /// Time between two positions: 60 per second, smooth on any display.
    pub const STEP_INTERVAL: f64 = 1.0 / 60.0;

    /// The walk never takes less or more than this, in seconds, whatever the distance.
    pub const DURATION_RANGE: (f64, f64) = (0.6, 3.0);

    pub fn target(&self) -> Point {
        self.points.last().copied().unwrap_or(self.start)
    }

    pub fn duration(&self) -> f64 {
        self.points.len() as f64 * Self::STEP_INTERVAL
    }
}

/// Plans a slow, slightly curved walk from `start` to a random spot near `anchor`.
///
/// - The target lies between half the radius and the full radius from
///   `anchor`, at a random angle, and at least half the radius away from
///   `start`, so every walk visibly moves without ever going far. Because
///   every target is chosen around the same anchor, repeated walks wander
///   around the spot where the user left the pointer instead of drifting off.
/// - The path is a quadratic Bézier with a random sideways bend, eased in and
///   out, so the pointer drifts like a hand resting on the mouse.
/// - The target and the bend's control point are kept within `radius` of
///   `anchor` and inside `bounds`, when given. Pass the display's frame inset
///   by a few points so walks never reach a screen edge. An anchor outside
///   `bounds` is first moved inside.
/// - A Bézier curve never leaves the triangle of its three points. So when
///   `start` is inside both limits, the whole walk is; when it is not (a
///   pointer left on a screen edge), the walk glides inwards from it, never
///   farther out than `start`, instead of snapping inside.
pub fn plan_walk(
    start: Point,
    anchor: Point,
    intensity: Intensity,
    bounds: Option<Rect>,
    rng: &mut impl RandomSource,
) -> JiggleWalk {
    let radius = intensity.radius();
    // Clamping is a projection onto the rectangle, which never increases the
    // distance to a point inside it, so once the anchor is inside, every
    // constrained point is both in bounds and within the radius.
    let anchor = bounds.map_or(anchor, |bounds| anchor.clamped_to(bounds));
    let constrain = |point: Point| {
        let point = point.limited_to(radius, anchor);
        bounds.map_or(point, |bounds| point.clamped_to(bounds))
    };
    let candidate = |angle: f64, distance: f64| {
        constrain(Point::new(anchor.x + distance * angle.cos(), anchor.y + distance * angle.sin()))
    };

    // Pick a target: a few random tries to find one far enough from where we
    // are. Near a corner many directions are cut off; if every try falls
    // short, sweep 16 directions at the full radius and take the farthest.
    // The reachable area always holds a point at least half a radius from any
    // start inside it, and the sweep finds it.
    let mut target = anchor;
    let mut best = -1.0;
    for _ in 0..8 {
        let angle = rng.uniform(0.0, TAU);
        let point = candidate(angle, radius * rng.uniform(0.5, 1.0));
        let from_start = point.distance_to(start);
        if from_start > best {
            target = point;
            best = from_start;
        }
        if from_start >= radius / 2.0 {
            break;
        }
    }
    if best < radius / 2.0 {
        for index in 0..16 {
            let point = candidate(index as f64 * PI / 8.0, radius);
            let from_start = point.distance_to(start);
            if from_start > best {
                target = point;
                best = from_start;
            }
        }
    }

    // Bend the path sideways by up to a third of its length.
    let length = start.distance_to(target);
    let middle = Point::new((start.x + target.x) / 2.0, (start.y + target.y) / 2.0);
    let mut control = middle;
    if length > 0.0 {
        let normal = Point::new(-(target.y - start.y) / length, (target.x - start.x) / length);
        let bend = length * rng.uniform(-0.33, 0.33);
        control = constrain(Point::new(middle.x + normal.x * bend, middle.y + normal.y * bend));
    }

    let pace = length / intensity.speed() * rng.uniform(0.85, 1.15);
    let duration = pace.clamp(JiggleWalk::DURATION_RANGE.0, JiggleWalk::DURATION_RANGE.1);
    let steps = ((duration / JiggleWalk::STEP_INTERVAL).round() as usize).max(1);

    let mut points = Vec::with_capacity(steps);
    for step in 1..=steps {
        let t = step as f64 / steps as f64;
        let eased = t * t * (3.0 - 2.0 * t);
        let u = 1.0 - eased;
        points.push(Point::new(
            u * u * start.x + 2.0 * u * eased * control.x + eased * eased * target.x,
            u * u * start.y + 2.0 * u * eased * control.y + eased * eased * target.y,
        ));
    }
    // The curve already ends on the target; make it exact despite rounding.
    if let Some(last) = points.last_mut() {
        *last = target;
    }

    JiggleWalk { start, points }
}
