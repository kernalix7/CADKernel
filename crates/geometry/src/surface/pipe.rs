use std::sync::Arc;

use cadkernel_math::{Point3, Vec3};

use crate::curve::Curve;
use crate::surface::Surface;

/// Surface created by sweeping a circular profile along a path curve.
///
/// Parameter `u` traces along the path curve; parameter `v` sweeps the
/// circle cross-section in `[0, 2pi]`.  The local frame is built from a
/// stable reference direction (avoiding the degenerate case when the
/// tangent is nearly aligned with the chosen up-vector).
pub struct PipeSurface {
    path: Arc<dyn Curve>,
    radius: f64,
}

impl PipeSurface {
    pub fn new(path: Arc<dyn Curve>, radius: f64) -> Self {
        Self { path, radius }
    }

    /// Builds a stable Frenet-like frame `(normal, binormal)` at a path parameter.
    fn frame_at(&self, u: f64) -> (Vec3, Vec3) {
        let tang = self.path.tangent_at(u);
        let t_len = tang.length();
        if t_len < 1e-14 {
            return (Vec3::X, Vec3::Y);
        }
        let t = tang / t_len;

        let up = if t.z.abs() < 0.9 { Vec3::Z } else { Vec3::X };
        let n = t.cross(up);
        let n_len = n.length();
        if n_len < 1e-14 {
            return (Vec3::X, Vec3::Y);
        }
        let n = n / n_len;
        let b = t.cross(n);
        (n, b)
    }
}

impl Surface for PipeSurface {
    fn point_at(&self, u: f64, v: f64) -> Point3 {
        let center = self.path.point_at(u);
        let (n, b) = self.frame_at(u);

        let cos_v = v.cos();
        let sin_v = v.sin();
        Point3::new(
            center.x + self.radius * (n.x * cos_v + b.x * sin_v),
            center.y + self.radius * (n.y * cos_v + b.y * sin_v),
            center.z + self.radius * (n.z * cos_v + b.z * sin_v),
        )
    }

    fn normal_at(&self, u: f64, v: f64) -> Vec3 {
        let center = self.path.point_at(u);
        let p = self.point_at(u, v);
        let n = Vec3::new(p.x - center.x, p.y - center.y, p.z - center.z);
        let n_len = n.length();
        if n_len < 1e-14 {
            Vec3::Z
        } else {
            n / n_len
        }
    }

    fn domain_u(&self) -> (f64, f64) {
        self.path.domain()
    }

    fn domain_v(&self) -> (f64, f64) {
        (0.0, std::f64::consts::TAU)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::line::LineSegment;

    #[test]
    fn test_pipe_along_line() {
        let path = Arc::new(
            LineSegment::new(Point3::ORIGIN, Point3::new(10.0, 0.0, 0.0)),
        );
        let pipe = PipeSurface::new(path, 1.0);

        // At midpoint of path (u=0.5), v=0 should be at radius distance from center
        let (u0, u1) = pipe.domain_u();
        let u_mid = (u0 + u1) * 0.5;
        let center = pipe.path.point_at(u_mid);
        let p = pipe.point_at(u_mid, 0.0);
        let dist = center.distance_to(p);
        assert!(
            (dist - 1.0).abs() < 1e-10,
            "distance from center = {dist}, expected 1.0"
        );
    }

    #[test]
    fn test_pipe_full_circle() {
        let path = Arc::new(
            LineSegment::new(Point3::ORIGIN, Point3::new(0.0, 0.0, 5.0)),
        );
        let pipe = PipeSurface::new(path, 2.0);

        let (u0, u1) = pipe.domain_u();
        let u = (u0 + u1) * 0.5;

        // Points at v=0 and v=pi should be diametrically opposite
        let p0 = pipe.point_at(u, 0.0);
        let p_pi = pipe.point_at(u, std::f64::consts::PI);
        let center = pipe.path.point_at(u);
        let d0 = center.distance_to(p0);
        let d_pi = center.distance_to(p_pi);
        assert!((d0 - 2.0).abs() < 1e-10);
        assert!((d_pi - 2.0).abs() < 1e-10);

        let diameter = p0.distance_to(p_pi);
        assert!(
            (diameter - 4.0).abs() < 1e-10,
            "diameter = {diameter}, expected 4.0"
        );
    }

    #[test]
    fn test_pipe_normal_outward() {
        let path = Arc::new(
            LineSegment::new(Point3::ORIGIN, Point3::new(5.0, 0.0, 0.0)),
        );
        let pipe = PipeSurface::new(path, 1.0);

        let (u0, u1) = pipe.domain_u();
        let u = (u0 + u1) * 0.5;
        let center = pipe.path.point_at(u);

        for i in 0..8 {
            let v = std::f64::consts::TAU * i as f64 / 8.0;
            let n = pipe.normal_at(u, v);
            let p = pipe.point_at(u, v);
            let radial = Vec3::new(p.x - center.x, p.y - center.y, p.z - center.z);
            let r_len = radial.length();
            if r_len > 1e-14 {
                let cos_angle = n.dot(radial / r_len);
                assert!(
                    cos_angle > 0.99,
                    "normal not outward at v={v}: cos_angle={cos_angle}"
                );
            }
        }
    }
}
