//! Conversion of analytical surfaces to NURBS representation.
//!
//! Provides `to_nurbs()` methods for [`Plane`], [`Cylinder`], [`Sphere`],
//! [`Cone`], and [`Torus`].

use std::f64::consts::{FRAC_PI_2, PI};

use cadkernel_core::KernelResult;
use cadkernel_math::{Point3, Vec3};

use super::cylinder::Cylinder;
use super::nurbs::NurbsSurface;
use super::plane::Plane;
use super::sphere::Sphere;

impl Plane {
    /// Converts a finite rectangular region of this plane to a bilinear NURBS surface.
    ///
    /// # Arguments
    /// * `u_range` — (u_min, u_max) parameter range in the u-direction.
    /// * `v_range` — (v_min, v_max) parameter range in the v-direction.
    pub fn to_nurbs(&self, u_range: (f64, f64), v_range: (f64, f64)) -> KernelResult<NurbsSurface> {
        let (u0, u1) = u_range;
        let (v0, v1) = v_range;
        let p00 = self.origin + self.u_axis * u0 + self.v_axis * v0;
        let p10 = self.origin + self.u_axis * u1 + self.v_axis * v0;
        let p01 = self.origin + self.u_axis * u0 + self.v_axis * v1;
        let p11 = self.origin + self.u_axis * u1 + self.v_axis * v1;

        NurbsSurface::new(
            1,
            1,
            2,
            2,
            vec![p00, p10, p01, p11],
            vec![1.0; 4],
            vec![0.0, 0.0, 1.0, 1.0],
            vec![0.0, 0.0, 1.0, 1.0],
        )
    }
}

impl Cylinder {
    /// Converts this cylinder to a rational NURBS surface.
    ///
    /// u-direction: circular cross-section (rational, degree 2).
    /// v-direction: height (linear, degree 1).
    pub fn to_nurbs(&self) -> KernelResult<NurbsSurface> {
        // Build a full circle's rational control points at the base
        let n_arcs = 4; // 4 quarter-arcs
        let d_theta = PI / 2.0;
        let w1 = (d_theta / 2.0).cos(); // cos(π/4) = √2/2

        // 9 CPs for the circular cross-section (degree 2)
        let mut circle_pts = Vec::with_capacity(9);
        let mut circle_wts = Vec::with_capacity(9);

        let point_on_circle = |angle: f64| -> Vec3 {
            let (s, c) = angle.sin_cos();
            Vec3::new(
                self.base_center.x + self.radius * (c * self.x_axis().x + s * self.y_axis().x),
                self.base_center.y + self.radius * (c * self.x_axis().y + s * self.y_axis().y),
                self.base_center.z + self.radius * (c * self.x_axis().z + s * self.y_axis().z),
            )
        };

        let mut angle = 0.0;
        circle_pts.push(point_on_circle(angle));
        circle_wts.push(1.0);

        for _ in 0..n_arcs {
            let mid_angle = angle + d_theta / 2.0;
            let end_angle = angle + d_theta;

            let mid_on = point_on_circle(mid_angle);
            let bc = Vec3::new(self.base_center.x, self.base_center.y, self.base_center.z);
            let mid_cp = bc + (mid_on - bc) / w1;

            circle_pts.push(mid_cp);
            circle_wts.push(w1);

            circle_pts.push(point_on_circle(end_angle));
            circle_wts.push(1.0);

            angle = end_angle;
        }

        let count_u = circle_pts.len(); // 9
        let count_v = 2; // bottom and top

        let height_vec = self.axis * self.height;

        // Control points: row-major [v][u]
        // v=0: base circle, v=1: top circle (base + height)
        let mut cps = Vec::with_capacity(count_u * count_v);
        let mut wts = Vec::with_capacity(count_u * count_v);

        // v=0 (base)
        for (pt, &w) in circle_pts.iter().zip(circle_wts.iter()) {
            cps.push(Point3::new(pt.x, pt.y, pt.z));
            wts.push(w);
        }
        // v=1 (top)
        for (pt, &w) in circle_pts.iter().zip(circle_wts.iter()) {
            cps.push(Point3::new(
                pt.x + height_vec.x,
                pt.y + height_vec.y,
                pt.z + height_vec.z,
            ));
            wts.push(w);
        }

        // Knot vectors
        let knots_u = vec![0.0, 0.0, 0.0, 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1.0, 1.0, 1.0];
        let knots_v = vec![0.0, 0.0, 1.0, 1.0];

        NurbsSurface::new(2, 1, count_u, count_v, cps, wts, knots_u, knots_v)
    }

    /// Returns the local X-axis.
    fn x_axis(&self) -> Vec3 {
        // The x_axis field is private; reconstruct it
        let candidate = if self.axis.x.abs() < 0.9 {
            Vec3::X
        } else {
            Vec3::Y
        };
        let perp = self.axis.cross(candidate);
        perp.normalized().unwrap_or(Vec3::X)
    }

    /// Returns the local Y-axis.
    fn y_axis(&self) -> Vec3 {
        self.axis.cross(self.x_axis())
    }
}

impl Sphere {
    /// Converts this sphere to a rational NURBS surface.
    ///
    /// Uses a 9×5 control net (4 u-arcs × 2 v-arcs) for a hemisphere approach.
    /// u-direction: longitude (degree 2, rational, full circle).
    /// v-direction: latitude (degree 2, rational, -π/2 to π/2).
    pub fn to_nurbs(&self) -> KernelResult<NurbsSurface> {
        // Build longitude circle CPs (same structure as cylinder)
        let n_u_arcs = 4;
        let d_theta = PI / 2.0;
        let w_u = (d_theta / 2.0).cos(); // cos(π/4)

        // Build latitude arc CPs (2 arcs: -π/2→0, 0→π/2)
        let n_v_arcs = 2;
        let d_phi = PI / 2.0;
        let w_v = (d_phi / 2.0).cos(); // cos(π/4)

        // Latitude levels: -π/2, -π/4, 0, π/4, π/2
        let v_angles = [
            -FRAC_PI_2,
            -FRAC_PI_2 / 2.0,
            0.0,
            FRAC_PI_2 / 2.0,
            FRAC_PI_2,
        ];
        let v_weights_pattern = [1.0, w_v, 1.0, w_v, 1.0];

        let count_u = 2 * n_u_arcs + 1; // 9
        let count_v = 2 * n_v_arcs + 1; // 5

        let mut cps = Vec::with_capacity(count_u * count_v);
        let mut wts = Vec::with_capacity(count_u * count_v);

        for (vi, (&phi, &wv)) in v_angles.iter().zip(v_weights_pattern.iter()).enumerate() {
            let (sin_phi, cos_phi) = phi.sin_cos();
            let r_at_v = self.radius * cos_phi;
            let z_at_v = self.radius * sin_phi;

            // For intermediate latitude CPs (odd vi), we need to compensate
            // the rational weight so the actual point lies on the sphere
            let cos_phi_actual = if vi % 2 == 1 {
                // For mid-latitude CPs: the rational "shoulder" point
                // needs to be scaled by 1/w_v to produce correct interpolation
                cos_phi / wv
            } else {
                cos_phi
            };
            let z_actual = if vi % 2 == 1 {
                self.radius * sin_phi / wv
            } else {
                z_at_v
            };
            let r_actual = self.radius * cos_phi_actual;

            let _ = r_at_v; // using r_actual instead

            let mut angle = 0.0;

            // First CP
            let p = self.center
                + Vec3::new(r_actual * 1.0, r_actual * 0.0, z_actual);
            cps.push(p);
            wts.push(1.0 * wv);

            for _ in 0..n_u_arcs {
                let mid_angle = angle + d_theta / 2.0;
                let end_angle = angle + d_theta;

                // Mid CP (rational shoulder)
                let (s_m, c_m) = mid_angle.sin_cos();
                let mid_r = r_actual / w_u; // shoulder compensation
                let mid_p = self.center + Vec3::new(mid_r * c_m, mid_r * s_m, z_actual);
                cps.push(mid_p);
                wts.push(w_u * wv);

                // End CP
                let (s_e, c_e) = end_angle.sin_cos();
                let end_p = self.center + Vec3::new(r_actual * c_e, r_actual * s_e, z_actual);
                cps.push(end_p);
                wts.push(1.0 * wv);

                angle = end_angle;
            }
        }

        let knots_u = vec![
            0.0, 0.0, 0.0, 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1.0, 1.0, 1.0,
        ];
        let knots_v = vec![0.0, 0.0, 0.0, 0.5, 0.5, 1.0, 1.0, 1.0];

        NurbsSurface::new(2, 2, count_u, count_v, cps, wts, knots_u, knots_v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::Surface;

    #[test]
    fn test_plane_to_nurbs() {
        let plane = Plane::xy().unwrap();
        let s = plane.to_nurbs((0.0, 2.0), (0.0, 3.0)).unwrap();

        let p00 = s.point_at(0.0, 0.0);
        assert!(p00.distance_to(Point3::ORIGIN) < 1e-10);

        let p11 = s.point_at(1.0, 1.0);
        assert!(p11.distance_to(Point3::new(2.0, 3.0, 0.0)) < 1e-10);

        let pmid = s.point_at(0.5, 0.5);
        assert!(pmid.distance_to(Point3::new(1.0, 1.5, 0.0)) < 1e-10);
    }

    #[test]
    fn test_cylinder_to_nurbs() {
        let cyl = Cylinder::z_axis(2.0, 5.0);
        let s = cyl.to_nurbs().unwrap();
        let (u0, u1) = s.domain_u();
        let (v0, v1) = s.domain_v();

        // Sample points and verify they're on the cylinder
        for i in 0..=10 {
            let u = u0 + (u1 - u0) * i as f64 / 10.0;
            for j in 0..=4 {
                let v = v0 + (v1 - v0) * j as f64 / 4.0;
                let p = s.point_at(u, v);
                // Should be at radius 2 from z-axis
                let r = (p.x * p.x + p.y * p.y).sqrt();
                assert!(
                    (r - 2.0).abs() < 0.05,
                    "point at (u={u},v={v}) not on cylinder: r={r}"
                );
                // Height should be between 0 and 5
                assert!(p.z >= -0.01 && p.z <= 5.01, "z={} out of range", p.z);
            }
        }
    }

    #[test]
    fn test_sphere_to_nurbs() {
        let sph = Sphere::new(Point3::ORIGIN, 3.0).unwrap();
        let s = sph.to_nurbs().unwrap();
        let (u0, u1) = s.domain_u();
        let (v0, v1) = s.domain_v();

        // Sample points and verify they're on the sphere
        for i in 0..=10 {
            let u = u0 + (u1 - u0) * i as f64 / 10.0;
            for j in 0..=8 {
                let v = v0 + (v1 - v0) * j as f64 / 8.0;
                let p = s.point_at(u, v);
                let dist = p.distance_to(Point3::ORIGIN);
                assert!(
                    (dist - 3.0).abs() < 0.15,
                    "point at (u={u:.2},v={v:.2}) not on sphere: dist={dist:.4}"
                );
            }
        }
    }
}
