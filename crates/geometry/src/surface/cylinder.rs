use std::f64::consts::TAU;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};

use super::Surface;

/// A cylinder with given center axis, radius, and height.
/// Parameterized as u = angle [0, 2*PI], v = height [0, height].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cylinder {
    pub base_center: Point3,
    pub axis: Vec3,
    pub radius: f64,
    pub height: f64,
    x_axis: Vec3,
    y_axis: Vec3,
}

impl Cylinder {
    /// Creates a cylinder along the given axis direction.
    pub fn new(base_center: Point3, axis: Vec3, radius: f64, height: f64) -> KernelResult<Self> {
        let a = axis
            .normalized()
            .ok_or_else(|| KernelError::InvalidArgument("cylinder axis must be non-zero".into()))?;
        let x_axis = arbitrary_perpendicular(a);
        let y_axis = a.cross(x_axis);
        Ok(Self {
            base_center,
            axis: a,
            radius,
            height,
            x_axis,
            y_axis,
        })
    }

    /// A Z-axis aligned cylinder at the origin with X as the reference direction.
    pub fn z_axis(radius: f64, height: f64) -> Self {
        Self {
            base_center: Point3::ORIGIN,
            axis: Vec3::Z,
            radius,
            height,
            x_axis: Vec3::X,
            y_axis: Vec3::Y,
        }
    }
}

fn arbitrary_perpendicular(n: Vec3) -> Vec3 {
    let candidate = if n.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
    n.cross(candidate).normalized().unwrap_or(Vec3::X)
}

impl Surface for Cylinder {
    fn point_at(&self, u: f64, v: f64) -> Point3 {
        let (sin, cos) = u.sin_cos();
        self.base_center
            + self.x_axis * (self.radius * cos)
            + self.y_axis * (self.radius * sin)
            + self.axis * v
    }

    fn normal_at(&self, u: f64, _v: f64) -> Vec3 {
        let (sin, cos) = u.sin_cos();
        self.x_axis * cos + self.y_axis * sin
    }

    fn domain_u(&self) -> (f64, f64) {
        (0.0, TAU)
    }

    fn domain_v(&self) -> (f64, f64) {
        (0.0, self.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_math::EPSILON;
    use std::f64::consts::FRAC_PI_2;

    #[test]
    fn test_cylinder_point_at_base() {
        let c = Cylinder::z_axis(1.0, 5.0);
        let p = c.point_at(0.0, 0.0);
        assert!((p.distance_to(Point3::new(1.0, 0.0, 0.0))).abs() < EPSILON);
    }

    #[test]
    fn test_cylinder_point_at_top() {
        let c = Cylinder::z_axis(1.0, 5.0);
        let p = c.point_at(FRAC_PI_2, 5.0);
        assert!(p.approx_eq(Point3::new(0.0, 1.0, 5.0)));
    }

    #[test]
    fn test_cylinder_normal_outward() {
        let c = Cylinder::z_axis(1.0, 5.0);
        let n = c.normal_at(0.0, 0.0);
        assert!((n.length() - 1.0).abs() < EPSILON);
    }
}
