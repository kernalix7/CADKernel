use std::sync::Arc;

use cadkernel_math::{Point3, Vec3};

use crate::curve::Curve;
use crate::surface::Surface;

/// Surface created by translating a profile curve along a direction vector.
///
/// Parameter mapping:
/// - `u` = profile curve parameter
/// - `v` in `[0, 1]` = extrusion parameter (0 = profile, 1 = fully extruded)
pub struct ExtrusionSurface {
    profile: Arc<dyn Curve>,
    direction: Vec3,
    length: f64,
}

impl ExtrusionSurface {
    pub fn new(profile: Arc<dyn Curve>, direction: Vec3, length: f64) -> Self {
        Self {
            profile,
            direction,
            length,
        }
    }
}

impl Surface for ExtrusionSurface {
    fn point_at(&self, u: f64, v: f64) -> Point3 {
        let p = self.profile.point_at(u);
        let offset = self.direction * (v * self.length);
        Point3::new(p.x + offset.x, p.y + offset.y, p.z + offset.z)
    }

    fn normal_at(&self, u: f64, v: f64) -> Vec3 {
        let du = self.du(u, v);
        let dv = self.dv(u, v);
        let n = du.cross(dv);
        let len = n.length();
        if len < 1e-14 {
            Vec3::Z
        } else {
            n / len
        }
    }

    fn domain_u(&self) -> (f64, f64) {
        self.profile.domain()
    }

    fn domain_v(&self) -> (f64, f64) {
        (0.0, 1.0)
    }

    fn du(&self, u: f64, _v: f64) -> Vec3 {
        self.profile.tangent_at(u)
    }

    fn dv(&self, _u: f64, _v: f64) -> Vec3 {
        self.direction * self.length
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::line::LineSegment;

    #[test]
    fn test_extrusion_of_line_produces_plane() {
        // Extrude a line along X in the Z direction
        let line = Arc::new(LineSegment::new(
            Point3::ORIGIN,
            Point3::new(4.0, 0.0, 0.0),
        ));
        let ext = ExtrusionSurface::new(line, Vec3::Z, 3.0);

        // At v=0, should be on the original line
        let p0 = ext.point_at(0.5, 0.0);
        assert!(p0.approx_eq(Point3::new(2.0, 0.0, 0.0)));

        // At v=1, should be offset by Z*3
        let p1 = ext.point_at(0.5, 1.0);
        assert!(p1.approx_eq(Point3::new(2.0, 0.0, 3.0)));

        // At v=0.5, should be midway
        let pm = ext.point_at(0.0, 0.5);
        assert!(pm.approx_eq(Point3::new(0.0, 0.0, 1.5)));
    }

    #[test]
    fn test_extrusion_domain() {
        let line = Arc::new(LineSegment::new(
            Point3::ORIGIN,
            Point3::new(1.0, 0.0, 0.0),
        ));
        let ext = ExtrusionSurface::new(line, Vec3::Y, 5.0);
        assert_eq!(ext.domain_u(), (0.0, 1.0));
        assert_eq!(ext.domain_v(), (0.0, 1.0));
    }

    #[test]
    fn test_extrusion_normal() {
        let line = Arc::new(LineSegment::new(
            Point3::ORIGIN,
            Point3::new(1.0, 0.0, 0.0),
        ));
        let ext = ExtrusionSurface::new(line, Vec3::Z, 1.0);
        // du = X direction, dv = Z direction, normal = X × Z = -Y
        let n = ext.normal_at(0.5, 0.5);
        assert!((n.x).abs() < 1e-6);
        assert!((n.y - (-1.0)).abs() < 1e-6);
        assert!((n.z).abs() < 1e-6);
    }
}
