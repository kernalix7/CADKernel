use std::f64::consts::TAU;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};

use super::Curve;

/// A full circle in 3D space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Circle {
    pub center: Point3,
    pub normal: Vec3,
    pub radius: f64,
    x_axis: Vec3,
    y_axis: Vec3,
}

impl Circle {
    /// Creates a circle given center, normal, and radius.
    /// Constructs a local coordinate frame from the normal.
    pub fn new(center: Point3, normal: Vec3, radius: f64) -> KernelResult<Self> {
        let n = normal
            .normalized()
            .ok_or_else(|| KernelError::InvalidArgument("circle normal must be non-zero".into()))?;
        let x_axis = Self::arbitrary_perpendicular(n);
        let y_axis = n.cross(x_axis);
        Ok(Self {
            center,
            normal: n,
            radius,
            x_axis,
            y_axis,
        })
    }

    /// Constructs a circle on the XY-plane with X as the reference direction.
    pub fn xy(center: Point3, radius: f64) -> Self {
        Self {
            center,
            normal: Vec3::Z,
            radius,
            x_axis: Vec3::X,
            y_axis: Vec3::Y,
        }
    }

    /// Returns the local X-axis of the circle's coordinate frame.
    pub fn x_axis(&self) -> Vec3 {
        self.x_axis
    }

    /// Returns the local Y-axis of the circle's coordinate frame.
    pub fn y_axis(&self) -> Vec3 {
        self.y_axis
    }

    fn arbitrary_perpendicular(n: Vec3) -> Vec3 {
        let candidate = if n.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
        let perp = n.cross(candidate);
        perp.normalized().unwrap_or(Vec3::X)
    }
}

impl Curve for Circle {
    fn point_at(&self, t: f64) -> Point3 {
        let (sin, cos) = t.sin_cos();
        self.center + self.x_axis * (self.radius * cos) + self.y_axis * (self.radius * sin)
    }

    fn tangent_at(&self, t: f64) -> Vec3 {
        let (sin, cos) = t.sin_cos();
        self.x_axis * (-self.radius * sin) + self.y_axis * (self.radius * cos)
    }

    fn domain(&self) -> (f64, f64) {
        (0.0, TAU)
    }

    fn length(&self) -> f64 {
        TAU * self.radius
    }

    fn is_closed(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_math::EPSILON;
    use std::f64::consts::FRAC_PI_2;

    #[test]
    fn test_circle_circumference() {
        let c = Circle::xy(Point3::ORIGIN, 1.0);
        assert!((c.length() - TAU).abs() < EPSILON);
    }

    #[test]
    fn test_circle_points() {
        let c = Circle::xy(Point3::ORIGIN, 1.0);
        assert!(c.point_at(0.0).approx_eq(Point3::new(1.0, 0.0, 0.0)));
        assert!(c.point_at(FRAC_PI_2).approx_eq(Point3::new(0.0, 1.0, 0.0)));
    }
}
