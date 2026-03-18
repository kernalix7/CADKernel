use std::sync::Arc;

use cadkernel_math::{Point3, Vec3};

use crate::curve::Curve;
use crate::surface::Surface;

/// A Coons-style bilinear blending patch bounded by 4 curves.
///
/// - `c_u0`: bottom edge (u-curve at v=0)
/// - `c_u1`: top edge (u-curve at v=1)
/// - `c_v0`: left edge (v-curve at u=0)
/// - `c_v1`: right edge (v-curve at u=1)
///
/// The patch interpolates all four boundary curves exactly via the
/// standard Coons formula:  `S(u,v) = Lc(u,v) + Ld(u,v) - B(u,v)`.
pub struct CoonsPatch {
    c_u0: Arc<dyn Curve>,
    c_u1: Arc<dyn Curve>,
    c_v0: Arc<dyn Curve>,
    c_v1: Arc<dyn Curve>,
    corners: [Point3; 4],
}

impl CoonsPatch {
    pub fn new(
        c_u0: Arc<dyn Curve>,
        c_u1: Arc<dyn Curve>,
        c_v0: Arc<dyn Curve>,
        c_v1: Arc<dyn Curve>,
    ) -> Self {
        let (u0s, u0e) = c_u0.domain();
        let (u1s, u1e) = c_u1.domain();
        let corners = [
            c_u0.point_at(u0s), // p00
            c_u0.point_at(u0e), // p10
            c_u1.point_at(u1s), // p01
            c_u1.point_at(u1e), // p11
        ];
        let _ = (u1s, u1e); // suppress unused
        Self {
            c_u0,
            c_u1,
            c_v0,
            c_v1,
            corners,
        }
    }
}

impl Surface for CoonsPatch {
    fn point_at(&self, u: f64, v: f64) -> Point3 {
        let (u0_s, u0_e) = self.c_u0.domain();
        let (u1_s, u1_e) = self.c_u1.domain();
        let (v0_s, v0_e) = self.c_v0.domain();
        let (v1_s, v1_e) = self.c_v1.domain();

        let t_u0 = u0_s + u * (u0_e - u0_s);
        let t_u1 = u1_s + u * (u1_e - u1_s);
        let t_v0 = v0_s + v * (v0_e - v0_s);
        let t_v1 = v1_s + v * (v1_e - v1_s);

        let cu0 = self.c_u0.point_at(t_u0);
        let cu1 = self.c_u1.point_at(t_u1);
        let cv0 = self.c_v0.point_at(t_v0);
        let cv1 = self.c_v1.point_at(t_v1);

        let [p00, p10, p01, p11] = self.corners;

        // Lc: ruled surface in v direction
        let lc = Point3::new(
            (1.0 - v) * cu0.x + v * cu1.x,
            (1.0 - v) * cu0.y + v * cu1.y,
            (1.0 - v) * cu0.z + v * cu1.z,
        );
        // Ld: ruled surface in u direction
        let ld = Point3::new(
            (1.0 - u) * cv0.x + u * cv1.x,
            (1.0 - u) * cv0.y + u * cv1.y,
            (1.0 - u) * cv0.z + u * cv1.z,
        );
        // B: bilinear interpolation of corners
        let b = Point3::new(
            (1.0 - u) * (1.0 - v) * p00.x + u * (1.0 - v) * p10.x
                + (1.0 - u) * v * p01.x
                + u * v * p11.x,
            (1.0 - u) * (1.0 - v) * p00.y + u * (1.0 - v) * p10.y
                + (1.0 - u) * v * p01.y
                + u * v * p11.y,
            (1.0 - u) * (1.0 - v) * p00.z + u * (1.0 - v) * p10.z
                + (1.0 - u) * v * p01.z
                + u * v * p11.z,
        );

        Point3::new(lc.x + ld.x - b.x, lc.y + ld.y - b.y, lc.z + ld.z - b.z)
    }

    fn normal_at(&self, u: f64, v: f64) -> Vec3 {
        let du_vec = self.du(u, v);
        let dv_vec = self.dv(u, v);
        let n = du_vec.cross(dv_vec);
        n.normalized().unwrap_or(Vec3::Z)
    }

    fn domain_u(&self) -> (f64, f64) {
        (0.0, 1.0)
    }

    fn domain_v(&self) -> (f64, f64) {
        (0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::line::LineSegment;

    fn make_unit_square_patch() -> CoonsPatch {
        let c_u0 = Arc::new(
            LineSegment::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)),
        ) as Arc<dyn Curve>;
        let c_u1 = Arc::new(
            LineSegment::new(Point3::new(0.0, 1.0, 0.0), Point3::new(1.0, 1.0, 0.0)),
        ) as Arc<dyn Curve>;
        let c_v0 = Arc::new(
            LineSegment::new(Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0)),
        ) as Arc<dyn Curve>;
        let c_v1 = Arc::new(
            LineSegment::new(Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)),
        ) as Arc<dyn Curve>;
        CoonsPatch::new(c_u0, c_u1, c_v0, c_v1)
    }

    #[test]
    fn test_coons_corner_interpolation() {
        let patch = make_unit_square_patch();

        let p00 = patch.point_at(0.0, 0.0);
        assert!(
            p00.distance_to(Point3::new(0.0, 0.0, 0.0)) < 1e-10,
            "p00 = {p00:?}"
        );

        let p10 = patch.point_at(1.0, 0.0);
        assert!(
            p10.distance_to(Point3::new(1.0, 0.0, 0.0)) < 1e-10,
            "p10 = {p10:?}"
        );

        let p01 = patch.point_at(0.0, 1.0);
        assert!(
            p01.distance_to(Point3::new(0.0, 1.0, 0.0)) < 1e-10,
            "p01 = {p01:?}"
        );

        let p11 = patch.point_at(1.0, 1.0);
        assert!(
            p11.distance_to(Point3::new(1.0, 1.0, 0.0)) < 1e-10,
            "p11 = {p11:?}"
        );
    }

    #[test]
    fn test_coons_midpoint() {
        let patch = make_unit_square_patch();
        let mid = patch.point_at(0.5, 0.5);
        assert!(
            mid.distance_to(Point3::new(0.5, 0.5, 0.0)) < 1e-10,
            "midpoint = {mid:?}"
        );
    }

    #[test]
    fn test_coons_boundary_u0() {
        let patch = make_unit_square_patch();
        // Along v=0, should match c_u0
        for i in 0..=10 {
            let u = i as f64 / 10.0;
            let p = patch.point_at(u, 0.0);
            let expected = Point3::new(u, 0.0, 0.0);
            assert!(
                p.distance_to(expected) < 1e-10,
                "boundary u at v=0, u={u}: {p:?}"
            );
        }
    }

    #[test]
    fn test_coons_boundary_v0() {
        let patch = make_unit_square_patch();
        // Along u=0, should match c_v0
        for j in 0..=10 {
            let v = j as f64 / 10.0;
            let p = patch.point_at(0.0, v);
            let expected = Point3::new(0.0, v, 0.0);
            assert!(
                p.distance_to(expected) < 1e-10,
                "boundary v at u=0, v={v}: {p:?}"
            );
        }
    }

    #[test]
    fn test_coons_flat_normal() {
        let patch = make_unit_square_patch();
        let n = patch.normal_at(0.5, 0.5);
        // Flat patch: normal should be +Z or -Z
        assert!(
            (n.z.abs() - 1.0).abs() < 1e-6,
            "normal = {n:?}, expected +-Z"
        );
    }
}
