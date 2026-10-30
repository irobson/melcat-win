//! Port of the macOS app's `JigglePlannerTests`.

mod common;

use common::seeded;
use melcat_core::{plan_walk, Intensity, JiggleWalk, Point, Rect};

const ANCHOR: Point = Point::new(500.0, 400.0);
const EPSILON: f64 = 1e-9;

fn screen() -> Rect {
    Rect::new(0.0, 0.0, 1440.0, 900.0).inset(4.0, 4.0).expect("a screen")
}

/// Edges included, with room for floating-point rounding along the curve.
fn is_inside(point: Point, rect: Rect) -> bool {
    (rect.min_x - EPSILON..=rect.max_x + EPSILON).contains(&point.x)
        && (rect.min_y - EPSILON..=rect.max_y + EPSILON).contains(&point.y)
}

#[test]
fn every_point_stays_within_the_radius_of_the_anchor() {
    for intensity in Intensity::ALL {
        let mut rng = seeded(1);
        let mut start = ANCHOR;
        for _ in 0..200 {
            let walk = plan_walk(start, ANCHOR, intensity, None, &mut rng);
            for point in &walk.points {
                assert!(point.distance_to(ANCHOR) <= intensity.radius() + EPSILON);
            }
            start = walk.target();
        }
    }
}

#[test]
fn each_walk_visibly_moves_the_target_is_at_least_half_the_radius_away() {
    for intensity in Intensity::ALL {
        let mut rng = seeded(2);
        let mut start = ANCHOR;
        for _ in 0..200 {
            let walk = plan_walk(start, ANCHOR, intensity, None, &mut rng);
            assert!(walk.target().distance_to(ANCHOR) >= intensity.radius() / 2.0 - EPSILON);
            assert!(walk.target().distance_to(start) >= intensity.radius() / 2.0 - EPSILON);
            start = walk.target();
        }
    }
}

#[test]
fn walks_are_slow_and_smooth_bounded_duration_tiny_steps_no_jump_at_the_start() {
    for intensity in Intensity::ALL {
        let mut rng = seeded(3);
        let mut start = ANCHOR;
        for _ in 0..100 {
            let walk = plan_walk(start, ANCHOR, intensity, None, &mut rng);
            let duration = (walk.duration() * 1e6).round() / 1e6;
            assert!((JiggleWalk::DURATION_RANGE.0..=JiggleWalk::DURATION_RANGE.1).contains(&duration), "{duration}");

            let mut previous = start;
            let mut longest: f64 = 0.0;
            for &point in &walk.points {
                longest = longest.max(point.distance_to(previous));
                previous = point;
            }
            // Eased motion peaks at 1.5× the average speed. With the bend and
            // the ±15 % pace jitter, a step never exceeds 3× the nominal pace.
            assert!(longest <= intensity.speed() * 3.0 * JiggleWalk::STEP_INTERVAL, "{longest}");
            start = walk.target();
        }
    }
}

#[test]
fn walks_wander_around_the_anchor_instead_of_drifting_away() {
    let mut rng = seeded(4);
    let mut start = ANCHOR;
    for _ in 0..1000 {
        start = plan_walk(start, ANCHOR, Intensity::Playful, None, &mut rng).target();
    }
    assert!(start.distance_to(ANCHOR) <= Intensity::Playful.radius() + EPSILON);
}

#[test]
fn bounds_keep_walks_off_the_screen_edges() {
    let screen = screen();
    for intensity in Intensity::ALL {
        let mut rng = seeded(5);
        for _ in 0..50 {
            // Pointer left in the top-left corner, right on the hot corner.
            let corner = Point::new(0.0, 0.0);
            let inside = Point::new(4.0, 4.0); // the anchor, moved inside the bounds
            let mut start = corner;
            for walk_index in 0..20 {
                let walk = plan_walk(start, corner, intensity, Some(screen), &mut rng);
                for &point in &walk.points {
                    if walk_index == 0 {
                        // The first walk glides in from the corner: never farther out than its start.
                        assert!(point.x >= start.x - EPSILON && point.y >= start.y - EPSILON);
                        assert!(point.x <= screen.max_x && point.y <= screen.max_y);
                    } else {
                        assert!(is_inside(point, screen));
                        assert!(point.distance_to(inside) <= intensity.radius() + EPSILON);
                    }
                }
                assert!(is_inside(walk.target(), screen));
                assert!(walk.target().distance_to(inside) <= intensity.radius() + EPSILON);
                start = walk.target();
            }
        }
    }
}

#[test]
fn every_walk_moves_at_least_half_a_radius_even_when_chained_in_a_corner() {
    let screen = screen();
    for intensity in Intensity::ALL {
        let mut rng = seeded(9);
        for left in [Point::new(0.0, 0.0), Point::new(1439.0, 899.0), Point::new(700.0, 0.0)] {
            for _ in 0..1000 {
                let mut start = left;
                for _ in 0..20 {
                    let walk = plan_walk(start, left, intensity, Some(screen), &mut rng);
                    assert!(walk.target().distance_to(start) >= intensity.radius() / 2.0 - EPSILON);
                    start = walk.target();
                }
            }
        }
    }
}

#[test]
fn a_pointer_left_on_an_edge_drifts_away_from_it_smoothly() {
    let screen = screen();
    for intensity in Intensity::ALL {
        let mut rng = seeded(8);
        for start in [Point::new(0.0, 0.0), Point::new(700.0, 0.0), Point::new(1439.0, 899.0), Point::new(0.0, 450.0)] {
            for _ in 0..200 {
                let walk = plan_walk(start, start, intensity, Some(screen), &mut rng);
                let mut previous = start;
                for &point in &walk.points {
                    assert!(point.distance_to(previous) <= intensity.speed() * 3.0 * JiggleWalk::STEP_INTERVAL);
                    previous = point;
                }
                // Even from a corner, where many directions are cut off, the pointer moves.
                assert!(walk.target().distance_to(start) > 0.5);
            }
        }
    }
}

#[test]
fn same_seed_same_walk() {
    let (mut a, mut b) = (seeded(42), seeded(42));
    let first = plan_walk(ANCHOR, ANCHOR, Intensity::Gentle, None, &mut a);
    let second = plan_walk(ANCHOR, ANCHOR, Intensity::Gentle, None, &mut b);
    assert_eq!(first, second);
    assert_eq!(first.start, ANCHOR);
    assert_eq!(first.points.last().copied(), Some(first.target()));
}

#[test]
fn walks_differ_from_one_another() {
    let mut rng = seeded(6);
    let targets: std::collections::HashSet<String> = (0..20)
        .map(|_| {
            let target = plan_walk(ANCHOR, ANCHOR, Intensity::Gentle, None, &mut rng).target();
            format!("{},{}", target.x, target.y)
        })
        .collect();
    assert_eq!(targets.len(), 20);
}
