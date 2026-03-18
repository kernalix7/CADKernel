//! Conversion of analytical curves to NURBS representation.
//!
//! Provides `to_nurbs()` methods for [`Line`], [`LineSegment`], [`Circle`],
//! [`Arc`], and [`Ellipse`].

use std::f64::consts::{FRAC_PI_2, PI};

use cadkernel_core::KernelResult;
use cadkernel_math::Point3;

use super::arc::Arc;
use super::circle::Circle;
use super::ellipse::Ellipse;
use super::line::{Line, LineSegment};
use super::nurbs::NurbsCurve;

impl LineSegment {
    /// Converts this line segment to a degree-1 NURBS curve.
    pub fn to_nurbs(&self) -> KernelResult<NurbsCurve> {
        NurbsCurve::new(
            1,
            vec![self.start, self.end],
            vec![1.0, 1.0],
            vec![0.0, 0.0, 1.0, 1.0],
        )
    }
}

impl Line {
    /// Converts a finite portion of this infinite line to a degree-1 NURBS curve.
    ///
    /// `t_start` and `t_end` define the parameter range to convert.
    pub fn to_nurbs(&self, t_start: f64, t_end: f64) -> KernelResult<NurbsCurve> {
        let p0 = self.origin + self.direction * t_start;
        let p1 = self.origin + self.direction * t_end;
        NurbsCurve::new(
            1,
            vec![p0, p1],
            vec![1.0, 1.0],
            vec![0.0, 0.0, 1.0, 1.0],
        )
    }
}

impl Circle {
    /// Converts this full circle to a rational NURBS curve.
    ///
    /// Uses 9 control points (4 quarter-arcs) with rational weights.
    /// The NURBS Book §7.3.
    pub fn to_nurbs(&self) -> KernelResult<NurbsCurve> {
        arc_to_nurbs(
            self.center,
            self.x_axis(),
            self.y_axis(),
            self.radius,
            0.0,
            2.0 * PI,
        )
    }
}

impl Arc {
    /// Converts this circular arc to a rational NURBS curve.
    pub fn to_nurbs(&self) -> KernelResult<NurbsCurve> {
        arc_to_nurbs(
            self.center,
            self.x_axis(),
            self.y_axis(),
            self.radius,
            self.start_angle,
            self.end_angle,
        )
    }
}

impl Ellipse {
    /// Converts this full ellipse to a rational NURBS curve.
    ///
    /// Uses the same 9-point rational structure as a circle,
    /// but with semi-major/semi-minor scaling on each axis.
    pub fn to_nurbs(&self) -> KernelResult<NurbsCurve> {
        ellipse_to_nurbs(
            self.center,
            self.major_axis,
            self.minor_axis,
            self.semi_major,
            self.semi_minor,
            0.0,
            2.0 * PI,
        )
    }
}

/// Converts a circular arc defined by center, local axes, radius, and angle range
/// to a rational NURBS curve.
///
/// Splits the arc into segments of at most 90° each, using 3 control points
/// per segment (degree 2).
fn arc_to_nurbs(
    center: Point3,
    x_axis: cadkernel_math::Vec3,
    y_axis: cadkernel_math::Vec3,
    radius: f64,
    start_angle: f64,
    end_angle: f64,
) -> KernelResult<NurbsCurve> {
    let mut theta = end_angle - start_angle;
    if theta < 0.0 {
        theta += 2.0 * PI;
    }
    if theta.abs() < 1e-14 {
        theta = 2.0 * PI;
    }

    // Number of 90° segments
    let n_arcs = if theta <= FRAC_PI_2 + 1e-10 {
        1
    } else if theta <= PI + 1e-10 {
        2
    } else if theta <= 3.0 * FRAC_PI_2 + 1e-10 {
        3
    } else {
        4
    };

    let d_theta = theta / n_arcs as f64;
    let w1 = (d_theta / 2.0).cos(); // weight for intermediate CPs

    let mut control_points = Vec::with_capacity(2 * n_arcs + 1);
    let mut weights = Vec::with_capacity(2 * n_arcs + 1);

    let point_on_circle = |angle: f64| -> Point3 {
        let c = angle.cos();
        let s = angle.sin();
        Point3::new(
            center.x + radius * (c * x_axis.x + s * y_axis.x),
            center.y + radius * (c * x_axis.y + s * y_axis.y),
            center.z + radius * (c * x_axis.z + s * y_axis.z),
        )
    };

    let mut angle = start_angle;
    control_points.push(point_on_circle(angle));
    weights.push(1.0);

    for _ in 0..n_arcs {
        let angle_mid = angle + d_theta / 2.0;
        let angle_end = angle + d_theta;

        // Intermediate CP: intersection of tangent lines (scaled by 1/w1)
        let mid_on_circle = point_on_circle(angle_mid);
        let mid_cp = Point3::new(
            center.x + (mid_on_circle.x - center.x) / w1,
            center.y + (mid_on_circle.y - center.y) / w1,
            center.z + (mid_on_circle.z - center.z) / w1,
        );

        control_points.push(mid_cp);
        weights.push(w1);

        control_points.push(point_on_circle(angle_end));
        weights.push(1.0);

        angle = angle_end;
    }

    // Knot vector: degree 2, each segment needs 2 knot spans
    let n_cps = control_points.len();
    let mut knots = Vec::with_capacity(n_cps + 3);
    knots.extend_from_slice(&[0.0, 0.0, 0.0]);
    for i in 1..n_arcs {
        let k = i as f64 / n_arcs as f64;
        knots.push(k);
        knots.push(k);
    }
    knots.extend_from_slice(&[1.0, 1.0, 1.0]);

    NurbsCurve::new(2, control_points, weights, knots)
}

/// Converts an elliptical arc to a rational NURBS curve.
fn ellipse_to_nurbs(
    center: Point3,
    major_axis: cadkernel_math::Vec3,
    minor_axis: cadkernel_math::Vec3,
    semi_major: f64,
    semi_minor: f64,
    start_angle: f64,
    end_angle: f64,
) -> KernelResult<NurbsCurve> {
    let mut theta = end_angle - start_angle;
    if theta < 0.0 {
        theta += 2.0 * PI;
    }
    if theta.abs() < 1e-14 {
        theta = 2.0 * PI;
    }

    let n_arcs = if theta <= FRAC_PI_2 + 1e-10 {
        1
    } else if theta <= PI + 1e-10 {
        2
    } else if theta <= 3.0 * FRAC_PI_2 + 1e-10 {
        3
    } else {
        4
    };

    let d_theta = theta / n_arcs as f64;
    let w1 = (d_theta / 2.0).cos();

    let mut control_points = Vec::with_capacity(2 * n_arcs + 1);
    let mut weights = Vec::with_capacity(2 * n_arcs + 1);

    let point_on_ellipse = |angle: f64| -> Point3 {
        let c = angle.cos();
        let s = angle.sin();
        Point3::new(
            center.x + semi_major * c * major_axis.x + semi_minor * s * minor_axis.x,
            center.y + semi_major * c * major_axis.y + semi_minor * s * minor_axis.y,
            center.z + semi_major * c * major_axis.z + semi_minor * s * minor_axis.z,
        )
    };

    let mut angle = start_angle;
    control_points.push(point_on_ellipse(angle));
    weights.push(1.0);

    for _ in 0..n_arcs {
        let angle_mid = angle + d_theta / 2.0;
        let angle_end = angle + d_theta;

        let mid_on_ellipse = point_on_ellipse(angle_mid);
        let mid_cp = Point3::new(
            center.x + (mid_on_ellipse.x - center.x) / w1,
            center.y + (mid_on_ellipse.y - center.y) / w1,
            center.z + (mid_on_ellipse.z - center.z) / w1,
        );

        control_points.push(mid_cp);
        weights.push(w1);

        control_points.push(point_on_ellipse(angle_end));
        weights.push(1.0);

        angle = angle_end;
    }

    let n_cps = control_points.len();
    let mut knots = Vec::with_capacity(n_cps + 3);
    knots.extend_from_slice(&[0.0, 0.0, 0.0]);
    for i in 1..n_arcs {
        let k = i as f64 / n_arcs as f64;
        knots.push(k);
        knots.push(k);
    }
    knots.extend_from_slice(&[1.0, 1.0, 1.0]);

    NurbsCurve::new(2, control_points, weights, knots)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::Curve;

    #[test]
    fn test_line_segment_to_nurbs() {
        let seg = LineSegment::new(Point3::new(0.0, 0.0, 0.0), Point3::new(3.0, 4.0, 0.0));
        let nurbs = seg.to_nurbs().unwrap();
        let (lo, hi) = nurbs.domain();
        assert!(nurbs.point_at(lo).distance_to(seg.start) < 1e-10);
        assert!(nurbs.point_at(hi).distance_to(seg.end) < 1e-10);
        // Midpoint
        let mid = nurbs.point_at(0.5);
        assert!(mid.distance_to(Point3::new(1.5, 2.0, 0.0)) < 1e-10);
    }

    #[test]
    fn test_circle_to_nurbs_roundtrip() {
        let circle = Circle::xy(Point3::new(1.0, 2.0, 0.0), 3.0);
        let nurbs = circle.to_nurbs().unwrap();
        let (lo, hi) = nurbs.domain();

        // Sample many points and verify they lie on the circle
        for i in 0..=20 {
            let t_nurbs = lo + (hi - lo) * i as f64 / 20.0;
            let p = nurbs.point_at(t_nurbs);
            let dist = p.distance_to(circle.center);
            assert!(
                (dist - 3.0).abs() < 1e-6,
                "point {i} not on circle: dist={dist}, expected 3.0"
            );
        }

        // Start point should be at angle=0
        let start = nurbs.point_at(lo);
        assert!(start.distance_to(Point3::new(4.0, 2.0, 0.0)) < 1e-6);
    }

    #[test]
    fn test_arc_to_nurbs() {
        let arc = Arc::xy(Point3::ORIGIN, 1.0, 0.0, FRAC_PI_2);
        let nurbs = arc.to_nurbs().unwrap();
        let (lo, hi) = nurbs.domain();

        // Start at (1,0,0)
        let start = nurbs.point_at(lo);
        assert!(start.distance_to(Point3::new(1.0, 0.0, 0.0)) < 1e-6);

        // End at (0,1,0)
        let end = nurbs.point_at(hi);
        assert!(end.distance_to(Point3::new(0.0, 1.0, 0.0)) < 1e-6);

        // All points should be on unit circle
        for i in 0..=10 {
            let t = lo + (hi - lo) * i as f64 / 10.0;
            let p = nurbs.point_at(t);
            let dist = (p.x * p.x + p.y * p.y).sqrt();
            assert!(
                (dist - 1.0).abs() < 1e-6,
                "point not on circle: dist={dist}"
            );
        }
    }

    #[test]
    fn test_ellipse_to_nurbs() {
        let ellipse = Ellipse::new(
            Point3::ORIGIN,
            cadkernel_math::Vec3::Z,
            cadkernel_math::Vec3::X,
            3.0, // semi-major
            1.0, // semi-minor
        );
        let nurbs = ellipse.to_nurbs().unwrap();
        let (lo, hi) = nurbs.domain();

        // Sample points and verify they satisfy x²/9 + y² = 1
        for i in 0..=20 {
            let t = lo + (hi - lo) * i as f64 / 20.0;
            let p = nurbs.point_at(t);
            let val = (p.x * p.x) / 9.0 + p.y * p.y;
            assert!(
                (val - 1.0).abs() < 1e-5,
                "point {i} not on ellipse: x²/9+y²={val}"
            );
        }
    }

    #[test]
    fn test_line_to_nurbs() {
        let line = Line::new(Point3::ORIGIN, cadkernel_math::Vec3::X);
        let nurbs = line.to_nurbs(-2.0, 5.0).unwrap();
        let (lo, hi) = nurbs.domain();
        assert!(nurbs.point_at(lo).distance_to(Point3::new(-2.0, 0.0, 0.0)) < 1e-10);
        assert!(nurbs.point_at(hi).distance_to(Point3::new(5.0, 0.0, 0.0)) < 1e-10);
    }
}
