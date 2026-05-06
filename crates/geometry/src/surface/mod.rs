//! Parametric surfaces in 3D space.
//!
//! This module defines the [`Surface`] trait and provides analytic
//! ([`Plane`](plane::Plane), [`Cylinder`](cylinder::Cylinder),
//! [`Sphere`](sphere::Sphere), [`Cone`](cone::Cone),
//! [`Torus`](torus::Torus)) and free-form
//! ([`NurbsSurface`](nurbs::NurbsSurface)) implementations.

pub mod cone;
pub mod continuity;
pub mod curvature;
pub mod cylinder;
pub mod extrusion;
pub mod filling;
pub mod isocurve;
pub mod nurbs;
pub mod nurbs_fitting;
pub mod parametric_wire;
pub mod pipe;
pub mod plane;
pub mod revolution;
pub mod sphere;
pub mod to_nurbs;
pub mod torus;
pub mod trimmed;

use cadkernel_math::{BoundingBox, Point3, Vec3};

const FINITE_DIFF_H: f64 = 1e-7;
const NUMERIC_ZERO: f64 = 1e-14;
const GRID_SAMPLES: usize = 16;

/// A parametric surface in 3D space evaluated over parameters `(u, v)`.
///
/// All implementations must be `Send + Sync` to allow safe usage inside
/// `Arc<dyn Surface>` across threads.
///
/// Required methods: [`point_at`](Self::point_at),
/// [`normal_at`](Self::normal_at), [`domain_u`](Self::domain_u),
/// [`domain_v`](Self::domain_v).
///
/// Optional methods with default implementations: `du`, `dv`,
/// `project_point`, `bounding_box`.
pub trait Surface: Send + Sync {
    /// Evaluates the surface position at parameters `(u, v)`.
    fn point_at(&self, u: f64, v: f64) -> Point3;

    /// Evaluates the outward surface normal at parameters `(u, v)`.
    fn normal_at(&self, u: f64, v: f64) -> Vec3;

    /// Returns the valid `u` parameter range `(u_min, u_max)`.
    fn domain_u(&self) -> (f64, f64);

    /// Returns the valid `v` parameter range `(v_min, v_max)`.
    fn domain_v(&self) -> (f64, f64);

    /// Partial derivative with respect to `u` at `(u, v)`.
    fn du(&self, u: f64, v: f64) -> Vec3 {
        let (u0, u1) = self.domain_u();
        let ua = (u - FINITE_DIFF_H).max(u0);
        let ub = (u + FINITE_DIFF_H).min(u1);
        let dt = ub - ua;
        if dt.abs() < NUMERIC_ZERO {
            return Vec3::ZERO;
        }
        let pa = self.point_at(ua, v);
        let pb = self.point_at(ub, v);
        (pb - pa) / dt
    }

    /// Partial derivative with respect to `v` at `(u, v)`.
    fn dv(&self, u: f64, v: f64) -> Vec3 {
        let (v0, v1) = self.domain_v();
        let va = (v - FINITE_DIFF_H).max(v0);
        let vb = (v + FINITE_DIFF_H).min(v1);
        let dt = vb - va;
        if dt.abs() < NUMERIC_ZERO {
            return Vec3::ZERO;
        }
        let pa = self.point_at(u, va);
        let pb = self.point_at(u, vb);
        (pb - pa) / dt
    }

    /// Projects a 3D point onto the surface, returning `(u, v, closest_point)`.
    /// Default: grid-sampled brute force.
    fn project_point(&self, point: Point3) -> (f64, f64, Point3) {
        let (u0, u1) = self.domain_u();
        let (v0, v1) = self.domain_v();
        let mut best_u = u0;
        let mut best_v = v0;
        let mut best_dist = f64::MAX;
        for i in 0..=GRID_SAMPLES {
            let u = u0 + (u1 - u0) * i as f64 / GRID_SAMPLES as f64;
            for j in 0..=GRID_SAMPLES {
                let v = v0 + (v1 - v0) * j as f64 / GRID_SAMPLES as f64;
                let p = self.point_at(u, v);
                let d = point.distance_to(p);
                if d < best_dist {
                    best_dist = d;
                    best_u = u;
                    best_v = v;
                }
            }
        }
        (best_u, best_v, self.point_at(best_u, best_v))
    }

    /// Axis-aligned bounding box of the surface (sampled).
    fn bounding_box(&self) -> BoundingBox {
        let (u0, u1) = self.domain_u();
        let (v0, v1) = self.domain_v();
        let first = self.point_at(u0, v0);
        let mut bb = BoundingBox::new(first, first);
        for i in 0..=GRID_SAMPLES {
            let u = u0 + (u1 - u0) * i as f64 / GRID_SAMPLES as f64;
            for j in 0..=GRID_SAMPLES {
                let v = v0 + (v1 - v0) * j as f64 / GRID_SAMPLES as f64;
                bb.include_point(self.point_at(u, v));
            }
        }
        bb
    }
}
