//! Curve-curve intersection via subdivision + Newton-Raphson refinement.
//!
//! Works with any pair of [`Curve`](crate::curve::Curve) implementors,
//! including NURBS curves, by operating purely through the parametric interface.

use cadkernel_math::Point3;

use crate::curve::Curve;

/// Result of a curve-curve intersection: parameter pairs and 3D points.
#[derive(Debug, Clone)]
pub struct CurveCurveHit {
    /// Parameter on the first curve.
    pub t1: f64,
    /// Parameter on the second curve.
    pub t2: f64,
    /// Intersection point (midpoint of the two evaluations).
    pub point: Point3,
}

/// Finds all intersections between two curves within a distance tolerance.
///
/// Uses recursive bounding-box subdivision to find candidate pairs, then
/// Newton-Raphson refinement to converge to exact parameters.
///
/// # Arguments
///
/// * `c1`, `c2` — the two curves.
/// * `tolerance` — maximum distance between the curves to count as an
///   intersection.
///
/// # Returns
///
/// A vector of [`CurveCurveHit`] values. Duplicates (within tolerance) are
/// filtered out.
pub fn intersect_curves(
    c1: &dyn Curve,
    c2: &dyn Curve,
    tolerance: f64,
) -> Vec<CurveCurveHit> {
    let (t1_lo, t1_hi) = c1.domain();
    let (t2_lo, t2_hi) = c2.domain();

    let mut hits = Vec::new();
    intersect_recursive(
        c1,
        c2,
        t1_lo,
        t1_hi,
        t2_lo,
        t2_hi,
        tolerance,
        0,
        &mut hits,
    );

    // Deduplicate close hits.
    dedup_hits(&mut hits, tolerance);
    hits
}

/// Recursive subdivision: splits each curve interval in half and tests
/// bounding-box overlap.
#[allow(clippy::too_many_arguments)]
fn intersect_recursive(
    c1: &dyn Curve,
    c2: &dyn Curve,
    t1_lo: f64,
    t1_hi: f64,
    t2_lo: f64,
    t2_hi: f64,
    tolerance: f64,
    depth: usize,
    hits: &mut Vec<CurveCurveHit>,
) {
    const MAX_DEPTH: usize = 30;
    const SAMPLE_N: usize = 5;

    // Sample bounding boxes.
    let bb1 = sample_bbox(c1, t1_lo, t1_hi, SAMPLE_N);
    let bb2 = sample_bbox(c2, t2_lo, t2_hi, SAMPLE_N);

    if !bbox_overlap(&bb1, &bb2, tolerance) {
        return;
    }

    let span1 = t1_hi - t1_lo;
    let span2 = t2_hi - t2_lo;

    // If intervals are tiny, try Newton refinement.
    if depth >= MAX_DEPTH || (span1 < 1e-12 && span2 < 1e-12) {
        let t1_mid = 0.5 * (t1_lo + t1_hi);
        let t2_mid = 0.5 * (t2_lo + t2_hi);
        if let Some(hit) = newton_refine(c1, c2, t1_mid, t2_mid, tolerance) {
            hits.push(hit);
        }
        return;
    }

    let t1_mid = 0.5 * (t1_lo + t1_hi);
    let t2_mid = 0.5 * (t2_lo + t2_hi);

    // 4 sub-problems.
    intersect_recursive(c1, c2, t1_lo, t1_mid, t2_lo, t2_mid, tolerance, depth + 1, hits);
    intersect_recursive(c1, c2, t1_lo, t1_mid, t2_mid, t2_hi, tolerance, depth + 1, hits);
    intersect_recursive(c1, c2, t1_mid, t1_hi, t2_lo, t2_mid, tolerance, depth + 1, hits);
    intersect_recursive(c1, c2, t1_mid, t1_hi, t2_mid, t2_hi, tolerance, depth + 1, hits);
}

/// Axis-aligned bounding box: (min, max).
type BBox = (Point3, Point3);

fn sample_bbox(curve: &dyn Curve, t_lo: f64, t_hi: f64, n: usize) -> BBox {
    let mut min = Point3::new(f64::MAX, f64::MAX, f64::MAX);
    let mut max = Point3::new(f64::MIN, f64::MIN, f64::MIN);
    for i in 0..=n {
        let t = t_lo + (t_hi - t_lo) * i as f64 / n as f64;
        let p = curve.point_at(t);
        min.x = min.x.min(p.x);
        min.y = min.y.min(p.y);
        min.z = min.z.min(p.z);
        max.x = max.x.max(p.x);
        max.y = max.y.max(p.y);
        max.z = max.z.max(p.z);
    }
    (min, max)
}

fn bbox_overlap(a: &BBox, b: &BBox, tol: f64) -> bool {
    a.0.x - tol <= b.1.x
        && a.1.x + tol >= b.0.x
        && a.0.y - tol <= b.1.y
        && a.1.y + tol >= b.0.y
        && a.0.z - tol <= b.1.z
        && a.1.z + tol >= b.0.z
}

/// Newton-Raphson refinement: find parameters (t1, t2) where c1(t1) ≈ c2(t2).
///
/// Minimises |c1(t1) - c2(t2)|² using the Jacobian of the distance vector.
fn newton_refine(
    c1: &dyn Curve,
    c2: &dyn Curve,
    t1_init: f64,
    t2_init: f64,
    tolerance: f64,
) -> Option<CurveCurveHit> {
    const MAX_ITER: usize = 20;
    let (lo1, hi1) = c1.domain();
    let (lo2, hi2) = c2.domain();

    let mut t1 = t1_init;
    let mut t2 = t2_init;

    for _ in 0..MAX_ITER {
        let p1 = c1.point_at(t1);
        let p2 = c2.point_at(t2);
        let diff = p1 - p2;
        let dist = (diff.x * diff.x + diff.y * diff.y + diff.z * diff.z).sqrt();

        if dist < tolerance {
            return Some(CurveCurveHit {
                t1,
                t2,
                point: Point3::new(
                    0.5 * (p1.x + p2.x),
                    0.5 * (p1.y + p2.y),
                    0.5 * (p1.z + p2.z),
                ),
            });
        }

        let d1 = c1.tangent_at(t1);
        let d2 = c2.tangent_at(t2);

        // Jacobian of F = c1(t1) - c2(t2):
        // dF/dt1 = d1, dF/dt2 = -d2
        // We want to solve J * [dt1, dt2]^T = -F using normal equations.
        let a11 = d1.x * d1.x + d1.y * d1.y + d1.z * d1.z;
        let a12 = -(d1.x * d2.x + d1.y * d2.y + d1.z * d2.z);
        let a22 = d2.x * d2.x + d2.y * d2.y + d2.z * d2.z;
        let b1 = -(diff.x * d1.x + diff.y * d1.y + diff.z * d1.z);
        let b2 = diff.x * d2.x + diff.y * d2.y + diff.z * d2.z;

        let det = a11 * a22 - a12 * a12;
        if det.abs() < 1e-20 {
            break;
        }

        let dt1 = (b1 * a22 - b2 * a12) / det;
        let dt2 = (a11 * b2 - a12 * b1) / det;

        t1 = (t1 + dt1).clamp(lo1, hi1);
        t2 = (t2 + dt2).clamp(lo2, hi2);
    }

    // Final distance check.
    let p1 = c1.point_at(t1);
    let p2 = c2.point_at(t2);
    let dist = p1.distance_to(p2);
    if dist < tolerance {
        Some(CurveCurveHit {
            t1,
            t2,
            point: Point3::new(
                0.5 * (p1.x + p2.x),
                0.5 * (p1.y + p2.y),
                0.5 * (p1.z + p2.z),
            ),
        })
    } else {
        None
    }
}

fn dedup_hits(hits: &mut Vec<CurveCurveHit>, tol: f64) {
    let mut i = 0;
    while i < hits.len() {
        let mut j = i + 1;
        while j < hits.len() {
            let dt1 = (hits[i].t1 - hits[j].t1).abs();
            let dt2 = (hits[i].t2 - hits[j].t2).abs();
            let dp = hits[i].point.distance_to(hits[j].point);
            if dt1 < tol * 10.0 && dt2 < tol * 10.0 && dp < tol * 10.0 {
                hits.remove(j);
            } else {
                j += 1;
            }
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::circle::Circle;
    use crate::curve::line::LineSegment;

    #[test]
    fn test_line_line_intersection() {
        let l1 = LineSegment::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(2.0, 2.0, 0.0),
        );
        let l2 = LineSegment::new(
            Point3::new(0.0, 2.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        );
        let hits = intersect_curves(&l1, &l2, 1e-8);
        assert_eq!(hits.len(), 1, "two crossing lines should intersect once");
        assert!((hits[0].point.x - 1.0).abs() < 1e-6);
        assert!((hits[0].point.y - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_parallel_lines_no_intersection() {
        let l1 = LineSegment::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
        );
        let l2 = LineSegment::new(
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
        );
        let hits = intersect_curves(&l1, &l2, 1e-8);
        assert!(hits.is_empty());
    }

    #[test]
    fn test_line_circle_intersection() {
        let line = LineSegment::new(
            Point3::new(-2.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        );
        let circle = Circle::new(Point3::ORIGIN, cadkernel_math::Vec3::Z, 1.0).unwrap();
        let hits = intersect_curves(&line, &circle, 1e-6);
        assert_eq!(hits.len(), 2, "line through circle center should hit twice");
        // The two hit points should be at x = ±1.
        let xs: Vec<f64> = hits.iter().map(|h| h.point.x).collect();
        assert!(xs.iter().any(|&x| (x - 1.0).abs() < 1e-4));
        assert!(xs.iter().any(|&x| (x + 1.0).abs() < 1e-4));
    }

    #[test]
    fn test_nurbs_intersection() {
        use crate::curve::nurbs::NurbsCurve;
        // Two quadratic Bezier curves that cross.
        let c1 = NurbsCurve::bezier(vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.5, 1.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
        ])
        .unwrap();
        let c2 = NurbsCurve::bezier(vec![
            Point3::new(0.0, 0.5, 0.0),
            Point3::new(0.5, -0.5, 0.0),
            Point3::new(1.0, 0.5, 0.0),
        ])
        .unwrap();
        let hits = intersect_curves(&c1, &c2, 1e-8);
        assert!(
            hits.len() == 2,
            "two crossing parabolas should intersect twice, got {}",
            hits.len()
        );
    }
}
