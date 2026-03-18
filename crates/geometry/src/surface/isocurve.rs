use std::sync::Arc;

use cadkernel_math::{Point3, Vec3};

use crate::curve::Curve;
use crate::surface::Surface;

/// A curve extracted from a surface at a constant u parameter.
/// The curve parameter `t` maps to the surface's `v` parameter.
pub struct IsocurveU {
    surface: Arc<dyn Surface>,
    u_val: f64,
    v_domain: (f64, f64),
}

impl IsocurveU {
    pub fn new(surface: Arc<dyn Surface>, u: f64) -> Self {
        let v_domain = surface.domain_v();
        Self {
            surface,
            u_val: u,
            v_domain,
        }
    }
}

impl Curve for IsocurveU {
    fn point_at(&self, t: f64) -> Point3 {
        self.surface.point_at(self.u_val, t)
    }

    fn tangent_at(&self, t: f64) -> Vec3 {
        self.surface.dv(self.u_val, t)
    }

    fn domain(&self) -> (f64, f64) {
        self.v_domain
    }

    fn length(&self) -> f64 {
        let (t0, t1) = self.domain();
        let n = 64;
        let dt = (t1 - t0) / n as f64;
        let mut len = 0.0;
        let mut prev = self.point_at(t0);
        for i in 1..=n {
            let t = t0 + dt * i as f64;
            let curr = self.point_at(t);
            len += prev.distance_to(curr);
            prev = curr;
        }
        len
    }

    fn is_closed(&self) -> bool {
        false
    }
}

/// A curve extracted from a surface at a constant v parameter.
/// The curve parameter `t` maps to the surface's `u` parameter.
pub struct IsocurveV {
    surface: Arc<dyn Surface>,
    v_val: f64,
    u_domain: (f64, f64),
}

impl IsocurveV {
    pub fn new(surface: Arc<dyn Surface>, v: f64) -> Self {
        let u_domain = surface.domain_u();
        Self {
            surface,
            v_val: v,
            u_domain,
        }
    }
}

impl Curve for IsocurveV {
    fn point_at(&self, t: f64) -> Point3 {
        self.surface.point_at(t, self.v_val)
    }

    fn tangent_at(&self, t: f64) -> Vec3 {
        self.surface.du(t, self.v_val)
    }

    fn domain(&self) -> (f64, f64) {
        self.u_domain
    }

    fn length(&self) -> f64 {
        let (t0, t1) = self.domain();
        let n = 64;
        let dt = (t1 - t0) / n as f64;
        let mut len = 0.0;
        let mut prev = self.point_at(t0);
        for i in 1..=n {
            let t = t0 + dt * i as f64;
            let curr = self.point_at(t);
            len += prev.distance_to(curr);
            prev = curr;
        }
        len
    }

    fn is_closed(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::plane::Plane;
    use cadkernel_math::EPSILON;

    #[test]
    fn test_isocurve_u_on_plane() {
        let plane = Arc::new(Plane::new(Point3::ORIGIN, Vec3::X, Vec3::Y).unwrap());
        let iso = IsocurveU::new(plane, 2.0);
        // At constant u=2, point_at(v) = origin + X*2 + Y*v
        let p = iso.point_at(3.0);
        assert!(p.approx_eq(Point3::new(2.0, 3.0, 0.0)));
    }

    #[test]
    fn test_isocurve_v_on_plane() {
        let plane = Arc::new(Plane::new(Point3::ORIGIN, Vec3::X, Vec3::Y).unwrap());
        let iso = IsocurveV::new(plane, 5.0);
        // At constant v=5, point_at(u) = origin + X*u + Y*5
        let p = iso.point_at(1.0);
        assert!(p.approx_eq(Point3::new(1.0, 5.0, 0.0)));
    }

    #[test]
    fn test_isocurve_u_tangent() {
        let plane = Arc::new(Plane::new(Point3::ORIGIN, Vec3::X, Vec3::Y).unwrap());
        let iso = IsocurveU::new(plane, 0.0);
        let tang = iso.tangent_at(0.0);
        // Tangent along v direction = Y
        assert!((tang.x - 0.0).abs() < EPSILON);
        assert!((tang.y - 1.0).abs() < EPSILON);
        assert!((tang.z - 0.0).abs() < EPSILON);
    }

    #[test]
    fn test_isocurve_v_tangent() {
        let plane = Arc::new(Plane::new(Point3::ORIGIN, Vec3::X, Vec3::Y).unwrap());
        let iso = IsocurveV::new(plane, 0.0);
        let tang = iso.tangent_at(0.0);
        // Tangent along u direction = X
        assert!((tang.x - 1.0).abs() < EPSILON);
        assert!((tang.y - 0.0).abs() < EPSILON);
        assert!((tang.z - 0.0).abs() < EPSILON);
    }
}
