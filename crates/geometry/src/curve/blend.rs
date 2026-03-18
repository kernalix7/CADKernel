use cadkernel_math::Point3;

use crate::curve::nurbs::NurbsCurve;
use crate::curve::Curve;

/// Creates a cubic Bezier blend curve connecting the end of `curve1` to the
/// start of `curve2`.
///
/// The `continuity` parameter controls matching:
/// - 0 = G0 (position only)
/// - 1+ = G1 (position + tangent direction)
pub fn blend_curve(
    curve1: &dyn Curve,
    curve2: &dyn Curve,
    continuity: usize,
) -> NurbsCurve {
    let (_, t1_end) = curve1.domain();
    let (t2_start, _) = curve2.domain();

    let p0 = curve1.point_at(t1_end);
    let p3 = curve2.point_at(t2_start);

    let dist = p0.distance_to(p3);
    let scale = dist / 3.0;

    let p1 = if continuity >= 1 {
        let tang1 = curve1.tangent_at(t1_end);
        let t1_len = tang1.length();
        if t1_len > 1e-14 {
            let t = tang1 / t1_len;
            Point3::new(p0.x + t.x * scale, p0.y + t.y * scale, p0.z + t.z * scale)
        } else {
            Point3::new(
                p0.x + (p3.x - p0.x) / 3.0,
                p0.y + (p3.y - p0.y) / 3.0,
                p0.z + (p3.z - p0.z) / 3.0,
            )
        }
    } else {
        Point3::new(
            p0.x + (p3.x - p0.x) / 3.0,
            p0.y + (p3.y - p0.y) / 3.0,
            p0.z + (p3.z - p0.z) / 3.0,
        )
    };

    let p2 = if continuity >= 1 {
        let tang2 = curve2.tangent_at(t2_start);
        let t2_len = tang2.length();
        if t2_len > 1e-14 {
            let t = tang2 / t2_len;
            Point3::new(p3.x - t.x * scale, p3.y - t.y * scale, p3.z - t.z * scale)
        } else {
            Point3::new(
                p0.x + 2.0 * (p3.x - p0.x) / 3.0,
                p0.y + 2.0 * (p3.y - p0.y) / 3.0,
                p0.z + 2.0 * (p3.z - p0.z) / 3.0,
            )
        }
    } else {
        Point3::new(
            p0.x + 2.0 * (p3.x - p0.x) / 3.0,
            p0.y + 2.0 * (p3.y - p0.y) / 3.0,
            p0.z + 2.0 * (p3.z - p0.z) / 3.0,
        )
    };

    // Cubic Bezier knot vector: [0,0,0,0, 1,1,1,1]
    let knots = vec![0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0];
    let weights = vec![1.0; 4];
    let control_points = vec![p0, p1, p2, p3];

    NurbsCurve::new(3, control_points, weights, knots)
        .expect("blend_curve: cubic Bezier construction must not fail")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::line::LineSegment;

    #[test]
    fn test_blend_g0_endpoints() {
        let c1 = LineSegment::new(Point3::ORIGIN, Point3::new(1.0, 0.0, 0.0));
        let c2 = LineSegment::new(Point3::new(2.0, 1.0, 0.0), Point3::new(3.0, 1.0, 0.0));

        let blend = blend_curve(&c1, &c2, 0);

        // Start should match end of c1
        let start = blend.point_at(0.0);
        assert!(start.approx_eq(Point3::new(1.0, 0.0, 0.0)));

        // End should match start of c2
        let end = blend.point_at(1.0);
        assert!(end.approx_eq(Point3::new(2.0, 1.0, 0.0)));
    }

    #[test]
    fn test_blend_g1_tangent_direction() {
        let c1 = LineSegment::new(Point3::ORIGIN, Point3::new(1.0, 0.0, 0.0));
        let c2 = LineSegment::new(Point3::new(2.0, 1.0, 0.0), Point3::new(3.0, 1.0, 0.0));

        let blend = blend_curve(&c1, &c2, 1);

        // Tangent at start should be in the same direction as c1's tangent (along X)
        let tang_start = blend.tangent_at(0.0);
        let tang_start_len = tang_start.length();
        assert!(tang_start_len > 1e-10);
        // Should point in +X direction (same as c1)
        let normalized = tang_start / tang_start_len;
        assert!(normalized.x > 0.9);
        assert!(normalized.y.abs() < 0.1);
    }

    #[test]
    fn test_blend_g0_is_cubic_bezier() {
        let c1 = LineSegment::new(Point3::ORIGIN, Point3::new(1.0, 0.0, 0.0));
        let c2 = LineSegment::new(Point3::new(4.0, 0.0, 0.0), Point3::new(5.0, 0.0, 0.0));

        let blend = blend_curve(&c1, &c2, 0);

        // Domain should be [0, 1]
        assert_eq!(blend.domain(), (0.0, 1.0));
    }
}
