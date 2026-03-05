use std::f64::consts::{PI, TAU};

use cadkernel_math::{Point3, Vec3};

use super::Surface;

/// A sphere parameterized by u = longitude [0, 2*PI], v = latitude [-PI/2, PI/2].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sphere {
    pub center: Point3,
    pub radius: f64,
}

impl Sphere {
    /// Creates a sphere with the given center and radius.
    pub fn new(center: Point3, radius: f64) -> Self {
        Self { center, radius }
    }
}

impl Surface for Sphere {
    fn point_at(&self, u: f64, v: f64) -> Point3 {
        let (sin_u, cos_u) = u.sin_cos();
        let (sin_v, cos_v) = v.sin_cos();
        self.center
            + Vec3::new(
                self.radius * cos_v * cos_u,
                self.radius * cos_v * sin_u,
                self.radius * sin_v,
            )
    }

    fn normal_at(&self, u: f64, v: f64) -> Vec3 {
        let (sin_u, cos_u) = u.sin_cos();
        let (sin_v, cos_v) = v.sin_cos();
        Vec3::new(cos_v * cos_u, cos_v * sin_u, sin_v)
    }

    fn domain_u(&self) -> (f64, f64) {
        (0.0, TAU)
    }

    fn domain_v(&self) -> (f64, f64) {
        (-PI / 2.0, PI / 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_math::EPSILON;
    use std::f64::consts::FRAC_PI_2;

    #[test]
    fn test_sphere_north_pole() {
        let s = Sphere::new(Point3::ORIGIN, 1.0);
        let p = s.point_at(0.0, FRAC_PI_2);
        assert!(p.approx_eq(Point3::new(0.0, 0.0, 1.0)));
    }

    #[test]
    fn test_sphere_equator() {
        let s = Sphere::new(Point3::ORIGIN, 2.0);
        let p = s.point_at(0.0, 0.0);
        assert!(p.approx_eq(Point3::new(2.0, 0.0, 0.0)));
    }

    #[test]
    fn test_sphere_normal_unit() {
        let s = Sphere::new(Point3::ORIGIN, 5.0);
        let n = s.normal_at(0.5, 0.3);
        assert!((n.length() - 1.0).abs() < EPSILON);
    }
}
