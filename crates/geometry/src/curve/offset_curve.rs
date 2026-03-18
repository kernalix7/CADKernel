use std::sync::Arc;

use cadkernel_math::{Point3, Vec3};

use crate::curve::Curve;

/// A curve offset by a fixed distance from a base curve.
///
/// The offset direction at each point is computed as the cross product of
/// `ref_normal` and the tangent, providing a consistent offset within a
/// reference plane.
pub struct OffsetCurve {
    base: Arc<dyn Curve>,
    distance: f64,
    ref_normal: Vec3,
}

impl OffsetCurve {
    pub fn new(base: Arc<dyn Curve>, distance: f64, ref_normal: Vec3) -> Self {
        Self {
            base,
            distance,
            ref_normal,
        }
    }
}

impl Curve for OffsetCurve {
    fn point_at(&self, t: f64) -> Point3 {
        let p = self.base.point_at(t);
        let tang = self.base.tangent_at(t);
        let offset_dir = self.ref_normal.cross(tang);
        let len = offset_dir.length();
        if len < 1e-14 {
            return p;
        }
        let offset_dir = offset_dir / len;
        Point3::new(
            p.x + offset_dir.x * self.distance,
            p.y + offset_dir.y * self.distance,
            p.z + offset_dir.z * self.distance,
        )
    }

    fn tangent_at(&self, t: f64) -> Vec3 {
        let (t0, t1) = self.domain();
        let h = 1e-6;
        let ta = (t - h).max(t0);
        let tb = (t + h).min(t1);
        let dt = tb - ta;
        if dt.abs() < 1e-14 {
            return Vec3::ZERO;
        }
        let pa = self.point_at(ta);
        let pb = self.point_at(tb);
        (pb - pa) / dt
    }

    fn domain(&self) -> (f64, f64) {
        self.base.domain()
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
        self.base.is_closed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::line::LineSegment;

    #[test]
    fn test_offset_line_distance() {
        // Line along X, offset in Z-plane → should shift in Y
        let line = Arc::new(LineSegment::new(
            Point3::ORIGIN,
            Point3::new(10.0, 0.0, 0.0),
        ));
        let offset = OffsetCurve::new(line, 3.0, Vec3::Z);
        // ref_normal=Z, tangent along X → Z×X = Y → offset by 3 in Y
        let p = offset.point_at(0.5);
        assert!((p.x - 5.0).abs() < 1e-6);
        assert!((p.y - 3.0).abs() < 1e-6);
        assert!(p.z.abs() < 1e-6);
    }

    #[test]
    fn test_offset_preserves_domain() {
        let line = Arc::new(LineSegment::new(
            Point3::ORIGIN,
            Point3::new(1.0, 0.0, 0.0),
        ));
        let offset = OffsetCurve::new(line.clone(), 1.0, Vec3::Z);
        assert_eq!(offset.domain(), line.domain());
    }

    #[test]
    fn test_offset_distance_verified() {
        let line = Arc::new(LineSegment::new(
            Point3::ORIGIN,
            Point3::new(5.0, 0.0, 0.0),
        ));
        let dist = 2.5;
        let offset = OffsetCurve::new(line.clone(), dist, Vec3::Z);
        // Check distance between base and offset at several points
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            let p_base = line.point_at(t);
            let p_off = offset.point_at(t);
            let d = p_base.distance_to(p_off);
            assert!(
                (d - dist).abs() < 1e-6,
                "at t={}, distance={}, expected={}",
                t,
                d,
                dist
            );
        }
    }
}
