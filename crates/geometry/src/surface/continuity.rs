use crate::surface::Surface;

/// Continuity level between two surfaces along a shared boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContinuityLevel {
    Discontinuous,
    G0,
    G1,
    G2,
}

/// Check continuity between two surfaces along their shared boundary.
///
/// Evaluates `s1` at `(u, v1_end)` and `s2` at `(u, v2_start)` for
/// `n_samples` uniformly spaced `u` values across the common `u` domain.
///
/// Tolerances:
/// - `pos_tol`: maximum position deviation for G0
/// - `ang_tol`: maximum normal angle deviation (radians) for G1
/// - `curv_tol`: maximum curvature deviation for G2
pub fn check_surface_continuity(
    s1: &dyn Surface,
    s2: &dyn Surface,
    n_samples: usize,
    pos_tol: f64,
    ang_tol: f64,
    curv_tol: f64,
) -> ContinuityLevel {
    let (u0, u1) = s1.domain_u();
    let (_, v1_end) = s1.domain_v();
    let (v2_start, _) = s2.domain_v();

    let samples = n_samples.max(2);
    let mut is_g0 = true;
    let mut is_g1 = true;
    let mut is_g2 = true;

    for i in 0..samples {
        let u = u0 + (u1 - u0) * i as f64 / (samples - 1) as f64;

        // G0: position check
        let p1 = s1.point_at(u, v1_end);
        let p2 = s2.point_at(u, v2_start);
        if p1.distance_to(p2) > pos_tol {
            is_g0 = false;
            break;
        }

        // G1: normal angle check
        let n1 = s1.normal_at(u, v1_end);
        let n2 = s2.normal_at(u, v2_start);
        let n1_len = n1.length();
        let n2_len = n2.length();
        if n1_len > 1e-14 && n2_len > 1e-14 {
            let cos_angle = n1.dot(n2) / (n1_len * n2_len);
            let angle = cos_angle.clamp(-1.0, 1.0).acos();
            if angle > ang_tol {
                is_g1 = false;
            }
        }

        // G2: curvature check (compare second fundamental form approximation)
        if is_g1 {
            let c1 = crate::surface::curvature::surface_curvatures(s1, u, v1_end);
            let c2 = crate::surface::curvature::surface_curvatures(s2, u, v2_start);
            if (c1.gaussian - c2.gaussian).abs() > curv_tol
                || (c1.mean - c2.mean).abs() > curv_tol
            {
                is_g2 = false;
            }
        }
    }

    if !is_g0 {
        ContinuityLevel::Discontinuous
    } else if !is_g1 {
        ContinuityLevel::G0
    } else if !is_g2 {
        ContinuityLevel::G1
    } else {
        ContinuityLevel::G2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::extrusion::ExtrusionSurface;
    use crate::surface::sphere::Sphere;
    use crate::curve::line::LineSegment;
    use cadkernel_math::{Point3, Vec3};
    use std::sync::Arc;

    #[test]
    fn test_two_coplanar_extrusions_are_g2() {
        // s1 extrudes [0,1] in Y from y=0 to y=1
        // s2 extrudes [0,1] in Y from y=1 to y=2
        // They share a boundary at y=1 (s1 v_end=1.0, s2 v_start=0.0)
        let line = Arc::new(LineSegment::new(
            Point3::ORIGIN,
            Point3::new(1.0, 0.0, 0.0),
        ));
        // s1: point_at(u, v=1) = line(u) + Y*1
        let s1 = ExtrusionSurface::new(line.clone(), Vec3::Y, 1.0);
        // s2: starts at y=1, point_at(u, v=0) = line2(u)
        let line2 = Arc::new(LineSegment::new(
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
        ));
        let s2 = ExtrusionSurface::new(line2, Vec3::Y, 1.0);
        let level = check_surface_continuity(&s1, &s2, 10, 1e-6, 1e-4, 1e-4);
        assert_eq!(level, ContinuityLevel::G2);
    }

    #[test]
    fn test_extrusion_vs_sphere_is_not_g1() {
        // An extrusion surface and a sphere will not be tangent-continuous
        let line = Arc::new(LineSegment::new(
            Point3::ORIGIN,
            Point3::new(1.0, 0.0, 0.0),
        ));
        let ext = ExtrusionSurface::new(line, Vec3::Z, 1.0);
        let sphere = Sphere::new(Point3::new(0.5, 0.0, 2.0), 1.0).unwrap();
        let level = check_surface_continuity(&ext, &sphere, 10, 2.0, 0.01, 0.01);
        // The normals differ substantially
        assert!(
            level == ContinuityLevel::G0 || level == ContinuityLevel::Discontinuous,
            "got {:?}",
            level,
        );
    }

    #[test]
    fn test_disconnected_extrusions() {
        let line1 = Arc::new(LineSegment::new(
            Point3::ORIGIN,
            Point3::new(1.0, 0.0, 0.0),
        ));
        let line2 = Arc::new(LineSegment::new(
            Point3::new(0.0, 0.0, 100.0),
            Point3::new(1.0, 0.0, 100.0),
        ));
        let s1 = ExtrusionSurface::new(line1, Vec3::Y, 1.0);
        let s2 = ExtrusionSurface::new(line2, Vec3::Y, 1.0);
        let level = check_surface_continuity(&s1, &s2, 10, 1e-6, 1e-4, 1e-4);
        assert_eq!(level, ContinuityLevel::Discontinuous);
    }
}
