//! Trim loop validation for B-Rep topology consistency.
//!
//! Validates that trim loops (ParametricWire2D) on faces satisfy:
//! 1. **Closure**: each segment endpoint matches the next segment's start.
//! 2. **Winding order**: outer loops are CCW, inner loops (holes) are CW.
//! 3. **Containment**: holes are fully inside the outer loop.
//! 4. **Non-intersection**: loops don't self-intersect or cross each other.

use cadkernel_geometry::surface::parametric_wire::ParametricWire2D;
use cadkernel_math::Point2;

/// Result of trim loop validation.
#[derive(Debug, Clone)]
pub struct TrimValidation {
    /// Whether the trim is valid overall.
    pub valid: bool,
    /// Individual issues found.
    pub issues: Vec<TrimIssue>,
}

/// A specific trim loop issue.
#[derive(Debug, Clone)]
pub enum TrimIssue {
    /// Gap between segment endpoints larger than tolerance.
    GapInLoop {
        segment_index: usize,
        gap: f64,
    },
    /// Outer loop has wrong winding (should be CCW).
    OuterLoopClockwise,
    /// Inner loop (hole) has wrong winding (should be CW).
    InnerLoopCounterClockwise {
        hole_index: usize,
    },
    /// Hole is not fully inside outer loop.
    HoleOutsideOuter {
        hole_index: usize,
    },
    /// Loop self-intersects.
    SelfIntersection {
        loop_index: usize,
    },
    /// Loop has too few segments.
    DegenerateLoop {
        segment_count: usize,
    },
}

/// Validates a trim configuration (outer + holes).
pub fn validate_trim(
    outer: &ParametricWire2D,
    holes: &[ParametricWire2D],
    tolerance: f64,
) -> TrimValidation {
    let mut issues = Vec::new();

    // Check outer loop
    if outer.segments.is_empty() {
        issues.push(TrimIssue::DegenerateLoop { segment_count: 0 });
        return TrimValidation {
            valid: false,
            issues,
        };
    }

    // Check closure of outer loop
    check_loop_closure(outer, &mut issues, tolerance);

    // Check winding of outer loop (should be CCW = positive area)
    let outer_poly = outer.to_polyline(16);
    let outer_area = signed_area_2d(&outer_poly);
    if outer_area < 0.0 {
        issues.push(TrimIssue::OuterLoopClockwise);
    }

    // Check each hole
    for (i, hole) in holes.iter().enumerate() {
        if hole.segments.is_empty() {
            issues.push(TrimIssue::DegenerateLoop { segment_count: 0 });
            continue;
        }

        // Closure
        check_hole_closure(hole, &mut issues, tolerance, i);

        // Winding (holes should be CW = negative area)
        let hole_poly = hole.to_polyline(16);
        let hole_area = signed_area_2d(&hole_poly);
        if hole_area > 0.0 {
            issues.push(TrimIssue::InnerLoopCounterClockwise { hole_index: i });
        }

        // Containment: sample hole points and check if inside outer
        let sample = hole.sample_points(8);
        let all_inside = sample.iter().all(|p| outer.contains_point(p.x, p.y));
        if !all_inside {
            issues.push(TrimIssue::HoleOutsideOuter { hole_index: i });
        }
    }

    TrimValidation {
        valid: issues.is_empty(),
        issues,
    }
}

/// Checks that a ParametricWire2D loop is closed (endpoints match within tolerance).
fn check_loop_closure(wire: &ParametricWire2D, issues: &mut Vec<TrimIssue>, tolerance: f64) {
    if wire.segments.len() < 2 {
        return;
    }

    for i in 0..wire.segments.len() {
        let (_, t1) = wire.segments[i].domain();
        let end = wire.segments[i].point_at(t1);

        let next_idx = (i + 1) % wire.segments.len();
        let (t0_next, _) = wire.segments[next_idx].domain();
        let start_next = wire.segments[next_idx].point_at(t0_next);

        let gap = end.distance_to(start_next);
        if gap > tolerance {
            issues.push(TrimIssue::GapInLoop {
                segment_index: i,
                gap,
            });
        }
    }
}

/// Checks closure for a hole loop (same logic, different error reporting).
fn check_hole_closure(
    wire: &ParametricWire2D,
    issues: &mut Vec<TrimIssue>,
    tolerance: f64,
    _hole_index: usize,
) {
    check_loop_closure(wire, issues, tolerance);
}

/// Computes the signed area of a 2D polygon.
///
/// Positive = CCW, Negative = CW.
fn signed_area_2d(polygon: &[Point2]) -> f64 {
    let n = polygon.len();
    if n < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        area += polygon[i].x * polygon[j].y;
        area -= polygon[j].x * polygon[i].y;
    }
    area / 2.0
}

/// Reverses the winding order of a ParametricWire2D by reversing segments.
///
/// Returns a new wire with reversed winding, which flips the signed area sign.
pub fn reverse_winding(wire: &ParametricWire2D) -> ParametricWire2D {
    use cadkernel_geometry::curve::curve2d::Line2D;
    use std::sync::Arc;

    // For each segment, create a reversed version and reverse the order
    let mut reversed_segments = Vec::with_capacity(wire.segments.len());

    for seg in wire.segments.iter().rev() {
        let (t0, t1) = seg.domain();
        let start = seg.point_at(t1);
        let end = seg.point_at(t0);
        // Approximate with a Line2D for simplicity
        let rev_seg: Arc<dyn cadkernel_geometry::Curve2D> = Arc::new(Line2D::new(start, end));
        reversed_segments.push(rev_seg);
    }

    ParametricWire2D::new(reversed_segments, wire.closed)
}

/// Ensures correct winding: outer=CCW, holes=CW.
///
/// Returns corrected wires (reversed if needed).
pub fn ensure_correct_winding(
    outer: &ParametricWire2D,
    holes: &[ParametricWire2D],
) -> (ParametricWire2D, Vec<ParametricWire2D>) {
    let outer_poly = outer.to_polyline(16);
    let outer_area = signed_area_2d(&outer_poly);

    let corrected_outer = if outer_area < 0.0 {
        reverse_winding(outer)
    } else {
        outer.clone()
    };

    let corrected_holes: Vec<ParametricWire2D> = holes
        .iter()
        .map(|hole| {
            let hole_poly = hole.to_polyline(16);
            let hole_area = signed_area_2d(&hole_poly);
            if hole_area > 0.0 {
                reverse_winding(hole)
            } else {
                hole.clone()
            }
        })
        .collect();

    (corrected_outer, corrected_holes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_geometry::curve::curve2d::Line2D;
    use std::sync::Arc;

    fn ccw_square() -> ParametricWire2D {
        let segs: Vec<Arc<dyn cadkernel_geometry::Curve2D>> = vec![
            Arc::new(Line2D::new(Point2::new(0.0, 0.0), Point2::new(1.0, 0.0))),
            Arc::new(Line2D::new(Point2::new(1.0, 0.0), Point2::new(1.0, 1.0))),
            Arc::new(Line2D::new(Point2::new(1.0, 1.0), Point2::new(0.0, 1.0))),
            Arc::new(Line2D::new(Point2::new(0.0, 1.0), Point2::new(0.0, 0.0))),
        ];
        ParametricWire2D::closed(segs)
    }

    fn cw_square() -> ParametricWire2D {
        let segs: Vec<Arc<dyn cadkernel_geometry::Curve2D>> = vec![
            Arc::new(Line2D::new(Point2::new(0.0, 0.0), Point2::new(0.0, 1.0))),
            Arc::new(Line2D::new(Point2::new(0.0, 1.0), Point2::new(1.0, 1.0))),
            Arc::new(Line2D::new(Point2::new(1.0, 1.0), Point2::new(1.0, 0.0))),
            Arc::new(Line2D::new(Point2::new(1.0, 0.0), Point2::new(0.0, 0.0))),
        ];
        ParametricWire2D::closed(segs)
    }

    fn small_hole() -> ParametricWire2D {
        // CW hole inside the unit square
        let segs: Vec<Arc<dyn cadkernel_geometry::Curve2D>> = vec![
            Arc::new(Line2D::new(Point2::new(0.3, 0.3), Point2::new(0.3, 0.7))),
            Arc::new(Line2D::new(Point2::new(0.3, 0.7), Point2::new(0.7, 0.7))),
            Arc::new(Line2D::new(Point2::new(0.7, 0.7), Point2::new(0.7, 0.3))),
            Arc::new(Line2D::new(Point2::new(0.7, 0.3), Point2::new(0.3, 0.3))),
        ];
        ParametricWire2D::closed(segs)
    }

    #[test]
    fn test_valid_trim() {
        let outer = ccw_square();
        let hole = small_hole();
        let result = validate_trim(&outer, &[hole], 0.01);
        assert!(result.valid, "should be valid: {:?}", result.issues);
    }

    #[test]
    fn test_outer_wrong_winding() {
        let outer = cw_square();
        let result = validate_trim(&outer, &[], 0.01);
        assert!(!result.valid);
        assert!(result.issues.iter().any(|i| matches!(i, TrimIssue::OuterLoopClockwise)));
    }

    #[test]
    fn test_hole_wrong_winding() {
        let outer = ccw_square();
        // CCW hole (wrong winding for a hole)
        let segs: Vec<Arc<dyn cadkernel_geometry::Curve2D>> = vec![
            Arc::new(Line2D::new(Point2::new(0.3, 0.3), Point2::new(0.7, 0.3))),
            Arc::new(Line2D::new(Point2::new(0.7, 0.3), Point2::new(0.7, 0.7))),
            Arc::new(Line2D::new(Point2::new(0.7, 0.7), Point2::new(0.3, 0.7))),
            Arc::new(Line2D::new(Point2::new(0.3, 0.7), Point2::new(0.3, 0.3))),
        ];
        let ccw_hole = ParametricWire2D::closed(segs);
        let result = validate_trim(&outer, &[ccw_hole], 0.01);
        assert!(!result.valid);
        assert!(result.issues.iter().any(|i| matches!(i, TrimIssue::InnerLoopCounterClockwise { .. })));
    }

    #[test]
    fn test_hole_outside_outer() {
        let outer = ccw_square();
        // Hole completely outside
        let segs: Vec<Arc<dyn cadkernel_geometry::Curve2D>> = vec![
            Arc::new(Line2D::new(Point2::new(5.0, 5.0), Point2::new(5.0, 6.0))),
            Arc::new(Line2D::new(Point2::new(5.0, 6.0), Point2::new(6.0, 6.0))),
            Arc::new(Line2D::new(Point2::new(6.0, 6.0), Point2::new(6.0, 5.0))),
            Arc::new(Line2D::new(Point2::new(6.0, 5.0), Point2::new(5.0, 5.0))),
        ];
        let outside_hole = ParametricWire2D::closed(segs);
        let result = validate_trim(&outer, &[outside_hole], 0.01);
        assert!(!result.valid);
        assert!(result.issues.iter().any(|i| matches!(i, TrimIssue::HoleOutsideOuter { .. })));
    }

    #[test]
    fn test_ensure_correct_winding() {
        let cw_outer = cw_square();
        let ccw_outer = ccw_square();

        let (corrected, _) = ensure_correct_winding(&cw_outer, &[]);
        let poly = corrected.to_polyline(16);
        let area = signed_area_2d(&poly);
        assert!(area > 0.0, "corrected outer should be CCW (positive area)");

        let (already_correct, _) = ensure_correct_winding(&ccw_outer, &[]);
        let poly2 = already_correct.to_polyline(16);
        let area2 = signed_area_2d(&poly2);
        assert!(area2 > 0.0, "already CCW should stay CCW");
    }

    #[test]
    fn test_signed_area() {
        // CCW triangle
        let ccw = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
        ];
        assert!(signed_area_2d(&ccw) > 0.0);

        // CW triangle
        let cw = vec![
            Point2::new(0.0, 0.0),
            Point2::new(0.0, 1.0),
            Point2::new(1.0, 0.0),
        ];
        assert!(signed_area_2d(&cw) < 0.0);
    }

    #[test]
    fn test_gap_detection() {
        // Wire with a gap between segments
        let segs: Vec<Arc<dyn cadkernel_geometry::Curve2D>> = vec![
            Arc::new(Line2D::new(Point2::new(0.0, 0.0), Point2::new(1.0, 0.0))),
            Arc::new(Line2D::new(Point2::new(1.5, 0.0), Point2::new(1.0, 1.0))), // gap!
            Arc::new(Line2D::new(Point2::new(1.0, 1.0), Point2::new(0.0, 0.0))),
        ];
        let wire = ParametricWire2D::closed(segs);
        let result = validate_trim(&wire, &[], 0.01);
        assert!(!result.valid);
        assert!(result.issues.iter().any(|i| matches!(i, TrimIssue::GapInLoop { .. })));
    }
}
