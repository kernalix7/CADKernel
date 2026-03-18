//! General curve-surface intersection via subdivision + Newton refinement.

use cadkernel_math::Point3;

use crate::curve::Curve;
use crate::surface::Surface;

/// A hit point from a curve-surface intersection.
#[derive(Debug, Clone)]
pub struct CurveSurfaceHit {
    /// Parameter on the curve.
    pub t: f64,
    /// Parameters on the surface.
    pub u: f64,
    pub v: f64,
    /// 3D intersection point.
    pub point: Point3,
}

/// Intersects a curve with a surface.
///
/// Algorithm:
/// 1. Sample the curve at `n_samples` points.
/// 2. For each sample, project onto the surface.
/// 3. If the distance is small, use Newton refinement to find the exact intersection.
///
/// # Arguments
/// * `curve` — the 3D curve.
/// * `surface` — the 3D surface.
/// * `tolerance` — distance tolerance for accepting a hit.
///
/// # Returns
/// A list of intersection points (may be empty).
pub fn intersect_curve_surface(
    curve: &dyn Curve,
    surface: &dyn Surface,
    tolerance: f64,
) -> Vec<CurveSurfaceHit> {
    let (t0, t1) = curve.domain();
    let n_samples = 100;
    let dt = (t1 - t0) / n_samples as f64;

    let mut hits = Vec::new();

    // Sample the curve and look for sign changes in distance
    let mut prev_pt = curve.point_at(t0);
    let (_, _, prev_proj) = surface.project_point(prev_pt);
    let mut prev_dist = prev_pt.distance_to(prev_proj);

    for i in 1..=n_samples {
        let t = t0 + dt * i as f64;
        let pt = curve.point_at(t);
        let (u, v, proj) = surface.project_point(pt);
        let dist = pt.distance_to(proj);

        // If we're close to the surface, or there's a sign change in
        // "which side of the surface we're on"
        if dist < tolerance {
            // Direct hit — refine with Newton
            if let Some(hit) = newton_refine(curve, surface, t, u, v, tolerance) {
                if !is_duplicate(&hits, &hit, tolerance) {
                    hits.push(hit);
                }
            }
        } else if prev_dist > tolerance && dist > tolerance {
            // Check for a crossing: the projected-distance curve crossed zero
            // Use midpoint bisection
            let dot_prev = signed_distance_approx(curve, surface, t - dt);
            let dot_curr = signed_distance_approx(curve, surface, t);
            if dot_prev * dot_curr < 0.0 {
                // There's a sign change → bisect to find crossing
                let mut lo_t = t - dt;
                let mut hi_t = t;
                for _ in 0..50 {
                    let mid_t = (lo_t + hi_t) / 2.0;
                    let d = signed_distance_approx(curve, surface, mid_t);
                    if d * dot_prev < 0.0 {
                        hi_t = mid_t;
                    } else {
                        lo_t = mid_t;
                    }
                }
                let mid_t = (lo_t + hi_t) / 2.0;
                let mid_pt = curve.point_at(mid_t);
                let (mu, mv, _) = surface.project_point(mid_pt);
                if let Some(hit) = newton_refine(curve, surface, mid_t, mu, mv, tolerance) {
                    if !is_duplicate(&hits, &hit, tolerance) {
                        hits.push(hit);
                    }
                }
            }
        }

        prev_pt = pt;
        prev_dist = dist;
        let _ = prev_pt; // suppress unused warning
    }

    hits
}

/// Approximate signed distance: (C(t) - S_proj(C(t))) · n(u,v).
fn signed_distance_approx(curve: &dyn Curve, surface: &dyn Surface, t: f64) -> f64 {
    let pt = curve.point_at(t);
    let (u, v, proj) = surface.project_point(pt);
    let n = surface.normal_at(u, v);
    let diff = pt - proj;
    diff.x * n.x + diff.y * n.y + diff.z * n.z
}

/// Newton refinement for curve-surface intersection.
///
/// Solves `F(t, u, v) = C(t) - S(u, v) = 0` using Newton's method.
fn newton_refine(
    curve: &dyn Curve,
    surface: &dyn Surface,
    t0: f64,
    u0: f64,
    v0: f64,
    tolerance: f64,
) -> Option<CurveSurfaceHit> {
    let (ct0, ct1) = curve.domain();
    let (su0, su1) = surface.domain_u();
    let (sv0, sv1) = surface.domain_v();

    let mut t = t0;
    let mut u = u0;
    let mut v = v0;

    for _ in 0..30 {
        let c = curve.point_at(t);
        let s = surface.point_at(u, v);
        let diff = c - s;
        let dist = (diff.x * diff.x + diff.y * diff.y + diff.z * diff.z).sqrt();

        if dist < tolerance * 0.01 {
            return Some(CurveSurfaceHit {
                t,
                u,
                v,
                point: c,
            });
        }

        let ct = curve.tangent_at(t);
        let su = surface.du(u, v);
        let sv = surface.dv(u, v);

        // 3×3 system: [ct, -su, -sv] * [dt, du, dv]^T = diff
        // We solve via least-squares (J^T J) x = J^T r
        let a11 = ct.dot(ct);
        let a12 = -ct.dot(su);
        let a13 = -ct.dot(sv);
        let a22 = su.dot(su);
        let a23 = su.dot(sv);
        let a33 = sv.dot(sv);

        let diff_v = cadkernel_math::Vec3::new(diff.x, diff.y, diff.z);
        let b1 = diff_v.dot(ct);
        let b2 = -diff_v.dot(su);
        let b3 = -diff_v.dot(sv);

        // Solve 3x3 by Cramer's rule
        let mat = [
            [a11, a12, a13],
            [a12, a22, a23],
            [a13, a23, a33],
        ];
        let det = mat[0][0] * (mat[1][1] * mat[2][2] - mat[1][2] * mat[2][1])
            - mat[0][1] * (mat[1][0] * mat[2][2] - mat[1][2] * mat[2][0])
            + mat[0][2] * (mat[1][0] * mat[2][1] - mat[1][1] * mat[2][0]);

        if det.abs() < 1e-30 {
            break;
        }

        let dt = (b1 * (mat[1][1] * mat[2][2] - mat[1][2] * mat[2][1])
            - mat[0][1] * (b2 * mat[2][2] - mat[1][2] * b3)
            + mat[0][2] * (b2 * mat[2][1] - mat[1][1] * b3))
            / det;
        let du = (mat[0][0] * (b2 * mat[2][2] - mat[1][2] * b3)
            - b1 * (mat[1][0] * mat[2][2] - mat[1][2] * mat[2][0])
            + mat[0][2] * (mat[1][0] * b3 - b2 * mat[2][0]))
            / det;
        let dv = (mat[0][0] * (mat[1][1] * b3 - b2 * mat[2][1])
            - mat[0][1] * (mat[1][0] * b3 - b2 * mat[2][0])
            + b1 * (mat[1][0] * mat[2][1] - mat[1][1] * mat[2][0]))
            / det;

        t = (t + dt).clamp(ct0, ct1);
        u = (u + du).clamp(su0, su1);
        v = (v + dv).clamp(sv0, sv1);
    }

    // Final check
    let c = curve.point_at(t);
    let s = surface.point_at(u, v);
    if c.distance_to(s) < tolerance {
        Some(CurveSurfaceHit {
            t,
            u,
            v,
            point: c,
        })
    } else {
        None
    }
}

/// Check if a hit is a duplicate of an existing one.
fn is_duplicate(hits: &[CurveSurfaceHit], candidate: &CurveSurfaceHit, tol: f64) -> bool {
    hits.iter()
        .any(|h| h.point.distance_to(candidate.point) < tol * 10.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::line::LineSegment;
    use crate::surface::plane::Plane;
    use crate::surface::sphere::Sphere;
    use cadkernel_math::Point3;

    #[test]
    fn test_line_plane_intersection() {
        let line = LineSegment::new(
            Point3::new(0.0, 0.0, -1.0),
            Point3::new(0.0, 0.0, 1.0),
        );
        let plane = Plane::xy().unwrap();
        let hits = intersect_curve_surface(&line, &plane, 1e-6);
        assert_eq!(hits.len(), 1, "line should intersect plane once");
        assert!(
            hits[0].point.distance_to(Point3::ORIGIN) < 1e-4,
            "intersection should be at origin, got {:?}",
            hits[0].point
        );
    }

    #[test]
    fn test_line_sphere_two_hits() {
        let line = LineSegment::new(
            Point3::new(-5.0, 0.0, 0.0),
            Point3::new(5.0, 0.0, 0.0),
        );
        let sphere = Sphere::new(Point3::ORIGIN, 1.0).unwrap();
        let hits = intersect_curve_surface(&line, &sphere, 1e-4);
        assert!(
            hits.len() >= 2,
            "line through sphere should have 2 hits, got {}",
            hits.len()
        );
    }

    #[test]
    fn test_line_sphere_miss() {
        let line = LineSegment::new(
            Point3::new(-5.0, 5.0, 0.0),
            Point3::new(5.0, 5.0, 0.0),
        );
        let sphere = Sphere::new(Point3::ORIGIN, 1.0).unwrap();
        let hits = intersect_curve_surface(&line, &sphere, 1e-4);
        assert_eq!(hits.len(), 0, "line above sphere should miss");
    }
}
