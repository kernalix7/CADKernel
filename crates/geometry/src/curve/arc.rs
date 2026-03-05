use cadkernel_math::{Point3, Vec3};

use super::Curve;

/// A circular arc in 3D space, parameterized from `start_angle` to `end_angle` (radians).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Arc {
    pub center: Point3,
    pub radius: f64,
    pub start_angle: f64,
    pub end_angle: f64,
    x_axis: Vec3,
    y_axis: Vec3,
}

impl Arc {
    /// Creates an arc on the XY-plane.
    pub fn xy(center: Point3, radius: f64, start_angle: f64, end_angle: f64) -> Self {
        Self {
            center,
            radius,
            start_angle,
            end_angle,
            x_axis: Vec3::X,
            y_axis: Vec3::Y,
        }
    }

    fn angle_span(&self) -> f64 {
        self.end_angle - self.start_angle
    }

    fn angle_at(&self, t: f64) -> f64 {
        self.start_angle + t * self.angle_span()
    }
}

impl Curve for Arc {
    fn point_at(&self, t: f64) -> Point3 {
        let a = self.angle_at(t);
        let (sin, cos) = a.sin_cos();
        self.center + self.x_axis * (self.radius * cos) + self.y_axis * (self.radius * sin)
    }

    fn tangent_at(&self, t: f64) -> Vec3 {
        let a = self.angle_at(t);
        let span = self.angle_span();
        let (sin, cos) = a.sin_cos();
        self.x_axis * (-self.radius * sin * span) + self.y_axis * (self.radius * cos * span)
    }

    fn domain(&self) -> (f64, f64) {
        (0.0, 1.0)
    }

    fn length(&self) -> f64 {
        self.radius * self.angle_span().abs()
    }

    fn is_closed(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_math::EPSILON;
    use std::f64::consts::{FRAC_PI_2, PI};

    #[test]
    fn test_arc_quarter_circle_length() {
        let arc = Arc::xy(Point3::ORIGIN, 1.0, 0.0, FRAC_PI_2);
        assert!((arc.length() - FRAC_PI_2).abs() < EPSILON);
    }

    #[test]
    fn test_arc_endpoints() {
        let arc = Arc::xy(Point3::ORIGIN, 1.0, 0.0, PI);
        assert!(arc.point_at(0.0).approx_eq(Point3::new(1.0, 0.0, 0.0)));
        assert!(arc.point_at(1.0).approx_eq(Point3::new(-1.0, 0.0, 0.0)));
    }
}
