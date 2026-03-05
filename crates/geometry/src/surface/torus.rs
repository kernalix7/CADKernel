use cadkernel_math::{Point3, Vec3};

use super::Surface;

/// A torus surface defined by center, axis, major radius, and minor radius.
///
/// Parameterisation: `u` = angle around the axis (major circle),
/// `v` = angle around the tube (minor circle).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Torus {
    pub center: Point3,
    pub axis: Vec3,
    pub major_radius: f64,
    pub minor_radius: f64,
    pub x_axis: Vec3,
    pub y_axis: Vec3,
}

impl Torus {
    /// Creates a torus with the given center, axis, major radius (ring), and minor radius (tube).
    pub fn new(center: Point3, axis: Vec3, major_radius: f64, minor_radius: f64) -> Self {
        let a = axis.normalized().unwrap_or(Vec3::Z);
        let x = if a.cross(Vec3::X).length() > 1e-6 {
            a.cross(Vec3::X).normalized().unwrap_or(Vec3::Y)
        } else {
            a.cross(Vec3::Y).normalized().unwrap_or(Vec3::X)
        };
        let y = a.cross(x);
        Self {
            center,
            axis: a,
            major_radius,
            minor_radius,
            x_axis: x,
            y_axis: y,
        }
    }
}

impl Surface for Torus {
    fn point_at(&self, u: f64, v: f64) -> Point3 {
        let ring_dir = self.x_axis * u.cos() + self.y_axis * u.sin();
        let ring_center = self.center + ring_dir * self.major_radius;
        let tube_dir = ring_dir * v.cos() + self.axis * v.sin();
        ring_center + tube_dir * self.minor_radius
    }

    fn normal_at(&self, u: f64, v: f64) -> Vec3 {
        let ring_dir = self.x_axis * u.cos() + self.y_axis * u.sin();
        let tube_dir = ring_dir * v.cos() + self.axis * v.sin();
        tube_dir.normalized().unwrap_or(Vec3::Z)
    }

    fn domain_u(&self) -> (f64, f64) {
        (0.0, std::f64::consts::TAU)
    }

    fn domain_v(&self) -> (f64, f64) {
        (0.0, std::f64::consts::TAU)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_math::EPSILON;
    use std::f64::consts::{FRAC_PI_2, TAU};

    #[test]
    fn test_torus_at_origin() {
        let t = Torus::new(Point3::ORIGIN, Vec3::Z, 3.0, 1.0);
        // u=0, v=0 → on the outer equator
        let p = t.point_at(0.0, 0.0);
        let r = (p.x * p.x + p.y * p.y).sqrt();
        assert!((r - 4.0).abs() < EPSILON); // major + minor
        assert!(p.z.abs() < EPSILON);
    }

    #[test]
    fn test_torus_inner_point() {
        let t = Torus::new(Point3::ORIGIN, Vec3::Z, 3.0, 1.0);
        // u=0, v=π → on the inner equator
        let p = t.point_at(0.0, std::f64::consts::PI);
        let r = (p.x * p.x + p.y * p.y).sqrt();
        assert!((r - 2.0).abs() < EPSILON); // major - minor
    }

    #[test]
    fn test_torus_top() {
        let t = Torus::new(Point3::ORIGIN, Vec3::Z, 3.0, 1.0);
        // u=0, v=π/2 → top of the tube
        let p = t.point_at(0.0, FRAC_PI_2);
        assert!((p.z - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_torus_closed() {
        let t = Torus::new(Point3::ORIGIN, Vec3::Z, 5.0, 2.0);
        let p0 = t.point_at(0.0, 0.0);
        let p1 = t.point_at(TAU, TAU);
        assert!(p0.approx_eq(p1));
    }
}
