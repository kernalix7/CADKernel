use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};

use super::Surface;

/// A conical surface defined by an apex, axis, and half-angle.
///
/// Parameterisation: `u` = angle around axis, `v` = distance from apex along slant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cone {
    pub apex: Point3,
    pub axis: Vec3,
    pub half_angle: f64,
    pub x_axis: Vec3,
    pub y_axis: Vec3,
}

impl Cone {
    /// Creates a cone with the given apex, axis direction, and half-angle (radians).
    pub fn new(apex: Point3, axis: Vec3, half_angle: f64) -> KernelResult<Self> {
        if half_angle <= 0.0 || half_angle >= std::f64::consts::FRAC_PI_2 {
            return Err(KernelError::InvalidArgument(
                "cone half_angle must be in (0, π/2)".into(),
            ));
        }
        let a = axis.normalized().unwrap_or(Vec3::Z);
        let x = if a.cross(Vec3::X).length() > 1e-6 {
            a.cross(Vec3::X).normalized().unwrap_or(Vec3::Y)
        } else {
            a.cross(Vec3::Y).normalized().unwrap_or(Vec3::X)
        };
        let y = a.cross(x);
        Ok(Self {
            apex,
            axis: a,
            half_angle,
            x_axis: x,
            y_axis: y,
        })
    }
}

impl Surface for Cone {
    fn point_at(&self, u: f64, v: f64) -> Point3 {
        let r = v * self.half_angle.tan();
        let dir = self.x_axis * u.cos() + self.y_axis * u.sin();
        self.apex + self.axis * v + dir * r
    }

    fn normal_at(&self, u: f64, _v: f64) -> Vec3 {
        let dir = self.x_axis * u.cos() + self.y_axis * u.sin();
        let slant = self.axis * self.half_angle.sin() - dir * self.half_angle.cos();
        let n = dir.cross(slant);
        n.normalized().unwrap_or(Vec3::Z)
    }

    fn domain_u(&self) -> (f64, f64) {
        (0.0, std::f64::consts::TAU)
    }

    fn domain_v(&self) -> (f64, f64) {
        (0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_math::EPSILON;

    #[test]
    fn test_cone_apex() {
        let c = Cone::new(Point3::ORIGIN, Vec3::Z, std::f64::consts::FRAC_PI_4).unwrap();
        let p = c.point_at(0.0, 0.0);
        assert!(p.approx_eq(Point3::ORIGIN));
    }

    #[test]
    fn test_cone_radius_at_v1() {
        let c = Cone::new(Point3::ORIGIN, Vec3::Z, std::f64::consts::FRAC_PI_4).unwrap();
        let p = c.point_at(0.0, 1.0);
        // At v=1, z=1, radius = tan(45°) = 1
        assert!((p.z - 1.0).abs() < EPSILON);
        let r = (p.x * p.x + p.y * p.y).sqrt();
        assert!((r - 1.0).abs() < 1e-6);
    }
}
