use std::sync::Arc;

use cadkernel_math::{Point3, Vec3};

use crate::curve::Curve;
use crate::surface::Surface;

/// Surface created by revolving a profile curve around an axis.
///
/// Parameter mapping:
/// - `u` = profile curve parameter
/// - `v` = rotation angle
pub struct RevolutionSurface {
    profile: Arc<dyn Curve>,
    axis_origin: Point3,
    axis_dir: Vec3,
    angle_start: f64,
    angle_end: f64,
}

impl RevolutionSurface {
    pub fn new(
        profile: Arc<dyn Curve>,
        axis_origin: Point3,
        axis_dir: Vec3,
        angle_start: f64,
        angle_end: f64,
    ) -> Self {
        let len = axis_dir.length();
        let axis_dir = if len > 1e-14 {
            axis_dir / len
        } else {
            Vec3::Z
        };
        Self {
            profile,
            axis_origin,
            axis_dir,
            angle_start,
            angle_end,
        }
    }
}

/// Rotate a point around an axis using Rodrigues' rotation formula.
fn rotate_point_around_axis(p: Point3, origin: Point3, axis: Vec3, angle: f64) -> Point3 {
    let rel = Vec3::new(p.x - origin.x, p.y - origin.y, p.z - origin.z);
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    let dot = rel.dot(axis);
    let cross = axis.cross(rel);
    let rotated = Vec3::new(
        rel.x * cos_a + cross.x * sin_a + axis.x * dot * (1.0 - cos_a),
        rel.y * cos_a + cross.y * sin_a + axis.y * dot * (1.0 - cos_a),
        rel.z * cos_a + cross.z * sin_a + axis.z * dot * (1.0 - cos_a),
    );
    Point3::new(
        origin.x + rotated.x,
        origin.y + rotated.y,
        origin.z + rotated.z,
    )
}

impl Surface for RevolutionSurface {
    fn point_at(&self, u: f64, v: f64) -> Point3 {
        let p = self.profile.point_at(u);
        rotate_point_around_axis(p, self.axis_origin, self.axis_dir, v)
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
        (self.angle_start, self.angle_end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::line::LineSegment;
    use std::f64::consts::TAU;

    #[test]
    fn test_revolution_cylinder_shape() {
        // Revolving a vertical line around the Z axis should produce a cylinder
        let radius = 3.0;
        let line = Arc::new(LineSegment::new(
            Point3::new(radius, 0.0, 0.0),
            Point3::new(radius, 0.0, 5.0),
        ));
        let rev = RevolutionSurface::new(line, Point3::ORIGIN, Vec3::Z, 0.0, TAU);

        // At v=0 (angle 0), point should be on original line
        let p0 = rev.point_at(0.0, 0.0);
        assert!((p0.x - radius).abs() < 1e-10);
        assert!(p0.y.abs() < 1e-10);

        // At v=PI/2, point should be rotated 90 degrees
        let p90 = rev.point_at(0.0, std::f64::consts::FRAC_PI_2);
        assert!(p90.x.abs() < 1e-10);
        assert!((p90.y - radius).abs() < 1e-10);

        // All points should be at distance = radius from Z axis
        for i in 0..=8 {
            let angle = TAU * i as f64 / 8.0;
            let p = rev.point_at(0.5, angle);
            let dist = (p.x * p.x + p.y * p.y).sqrt();
            assert!(
                (dist - radius).abs() < 1e-10,
                "angle={}, dist={}, expected={}",
                angle,
                dist,
                radius
            );
        }
    }

    #[test]
    fn test_revolution_domain() {
        let line = Arc::new(LineSegment::new(
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 1.0),
        ));
        let rev = RevolutionSurface::new(line, Point3::ORIGIN, Vec3::Z, 0.0, TAU);
        assert_eq!(rev.domain_u(), (0.0, 1.0));
        assert_eq!(rev.domain_v(), (0.0, TAU));
    }
}
