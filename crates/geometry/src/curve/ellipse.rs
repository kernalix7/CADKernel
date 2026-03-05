use cadkernel_math::{Point3, Vec3};

use super::Curve;

/// A full ellipse in 3D space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ellipse {
    pub center: Point3,
    pub normal: Vec3,
    pub major_axis: Vec3,
    pub minor_axis: Vec3,
    pub semi_major: f64,
    pub semi_minor: f64,
}

impl Ellipse {
    /// Creates an ellipse from a center, plane normal, major-axis direction,
    /// and the two semi-axis lengths. The minor axis is derived as `normal x major_axis`.
    pub fn new(
        center: Point3,
        normal: Vec3,
        major_axis: Vec3,
        semi_major: f64,
        semi_minor: f64,
    ) -> Self {
        let n = normal.normalized().unwrap_or(Vec3::Z);
        let ma = major_axis.normalized().unwrap_or(Vec3::X);
        let mi = n.cross(ma);
        Self {
            center,
            normal: n,
            major_axis: ma,
            minor_axis: mi,
            semi_major,
            semi_minor,
        }
    }
}

impl Curve for Ellipse {
    fn point_at(&self, t: f64) -> Point3 {
        let c = t.cos();
        let s = t.sin();
        self.center
            + self.major_axis * (self.semi_major * c)
            + self.minor_axis * (self.semi_minor * s)
    }

    fn tangent_at(&self, t: f64) -> Vec3 {
        let c = t.cos();
        let s = t.sin();
        self.major_axis * (-self.semi_major * s) + self.minor_axis * (self.semi_minor * c)
    }

    fn domain(&self) -> (f64, f64) {
        (0.0, std::f64::consts::TAU)
    }

    fn length(&self) -> f64 {
        let a = self.semi_major;
        let b = self.semi_minor;
        let sum = a + b;
        if sum.abs() < 1e-14 {
            return 0.0;
        }
        let h = ((a - b) * (a - b)) / (sum * sum);
        std::f64::consts::PI * sum * (1.0 + 3.0 * h / (10.0 + (4.0 - 3.0 * h).sqrt()))
    }

    fn is_closed(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_math::EPSILON;
    use std::f64::consts::{FRAC_PI_2, TAU};

    #[test]
    fn test_ellipse_points() {
        let e = Ellipse::new(Point3::ORIGIN, Vec3::Z, Vec3::X, 3.0, 2.0);
        let p0 = e.point_at(0.0);
        assert!(p0.approx_eq(Point3::new(3.0, 0.0, 0.0)));
        let p90 = e.point_at(FRAC_PI_2);
        assert!((p90.x).abs() < EPSILON);
        assert!((p90.y - 2.0).abs() < EPSILON);
    }

    #[test]
    fn test_ellipse_is_closed() {
        let e = Ellipse::new(Point3::ORIGIN, Vec3::Z, Vec3::X, 5.0, 3.0);
        assert!(e.is_closed());
        let p_start = e.point_at(0.0);
        let p_end = e.point_at(TAU);
        assert!(p_start.approx_eq(p_end));
    }

    #[test]
    fn test_ellipse_circle_case() {
        let e = Ellipse::new(Point3::ORIGIN, Vec3::Z, Vec3::X, 1.0, 1.0);
        let len = e.length();
        let expected = TAU;
        assert!((len - expected).abs() < 0.01);
    }
}
