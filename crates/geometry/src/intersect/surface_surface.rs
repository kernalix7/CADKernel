//! General surface-surface intersection (SSI) via marching algorithm.
//!
//! - [`ssi_starting_points`]: finds seed points by tessellating both surfaces.
//! - [`intersect_surfaces`]: marches from seeds to produce intersection curves.

use cadkernel_math::Point3;

use crate::surface::Surface;

/// A seed point for SSI marching.
#[derive(Debug, Clone)]
pub struct SsiSeed {
    /// Parameters on surface 1.
    pub u1: f64,
    pub v1: f64,
    /// Parameters on surface 2.
    pub u2: f64,
    pub v2: f64,
    /// 3D point.
    pub point: Point3,
}

/// An intersection curve represented as a sequence of points with parameters.
#[derive(Debug, Clone)]
pub struct SsiCurve {
    pub points: Vec<Point3>,
    pub params_s1: Vec<(f64, f64)>,
    pub params_s2: Vec<(f64, f64)>,
}

/// Finds starting points for SSI by coarse sampling and proximity test.
///
/// Tessellates both surfaces on a grid and finds close point pairs.
pub fn ssi_starting_points(
    s1: &dyn Surface,
    s2: &dyn Surface,
    tolerance: f64,
) -> Vec<SsiSeed> {
    let grid = 20;
    let (u10, u11) = s1.domain_u();
    let (v10, v11) = s1.domain_v();
    let (u20, u21) = s2.domain_u();
    let (v20, v21) = s2.domain_v();

    let mut seeds = Vec::new();

    // Sample surface 1
    let mut pts1 = Vec::with_capacity((grid + 1) * (grid + 1));
    for i in 0..=grid {
        let u = u10 + (u11 - u10) * i as f64 / grid as f64;
        for j in 0..=grid {
            let v = v10 + (v11 - v10) * j as f64 / grid as f64;
            pts1.push((u, v, s1.point_at(u, v)));
        }
    }

    // For each sample on s1, project onto s2 and check distance
    for &(u1, v1, p1) in &pts1 {
        let (u2, v2, p2) = s2.project_point(p1);
        let dist = p1.distance_to(p2);
        if dist < tolerance * 10.0 {
            // Refine with Newton
            if let Some(seed) = refine_ssi_point(s1, s2, u1, v1, u2, v2, tolerance) {
                if !seeds.iter().any(|s: &SsiSeed| s.point.distance_to(seed.point) < tolerance * 5.0) {
                    seeds.push(seed);
                }
            }
        }
    }

    // Also check from s2 side
    for i in 0..=grid {
        let u = u20 + (u21 - u20) * i as f64 / grid as f64;
        for j in 0..=grid {
            let v = v20 + (v21 - v20) * j as f64 / grid as f64;
            let p2 = s2.point_at(u, v);
            let (u1, v1, p1) = s1.project_point(p2);
            let dist = p1.distance_to(p2);
            if dist < tolerance * 10.0 {
                if let Some(seed) = refine_ssi_point(s1, s2, u1, v1, u, v, tolerance) {
                    if !seeds.iter().any(|s: &SsiSeed| s.point.distance_to(seed.point) < tolerance * 5.0) {
                        seeds.push(seed);
                    }
                }
            }
        }
    }

    seeds
}

/// Marches from seed points to produce intersection curves.
pub fn intersect_surfaces(
    s1: &dyn Surface,
    s2: &dyn Surface,
    tolerance: f64,
) -> Vec<SsiCurve> {
    let seeds = ssi_starting_points(s1, s2, tolerance);
    let mut curves = Vec::new();
    let mut used = vec![false; seeds.len()];

    for (idx, seed) in seeds.iter().enumerate() {
        if used[idx] {
            continue;
        }
        used[idx] = true;

        // March in both directions from the seed
        let forward = march(s1, s2, seed, tolerance, 1.0);
        let backward = march(s1, s2, seed, tolerance, -1.0);

        // Combine: backward (reversed) + seed + forward
        let mut points = Vec::new();
        let mut params_s1 = Vec::new();
        let mut params_s2 = Vec::new();

        for i in (0..backward.len()).rev() {
            points.push(backward[i].point);
            params_s1.push((backward[i].u1, backward[i].v1));
            params_s2.push((backward[i].u2, backward[i].v2));
        }

        points.push(seed.point);
        params_s1.push((seed.u1, seed.v1));
        params_s2.push((seed.u2, seed.v2));

        for pt in &forward {
            points.push(pt.point);
            params_s1.push((pt.u1, pt.v1));
            params_s2.push((pt.u2, pt.v2));
        }

        // Mark nearby seeds as used
        for (j, other) in seeds.iter().enumerate() {
            if !used[j] && points.iter().any(|p| p.distance_to(other.point) < tolerance * 20.0) {
                used[j] = true;
            }
        }

        if points.len() >= 2 {
            curves.push(SsiCurve {
                points,
                params_s1,
                params_s2,
            });
        }
    }

    curves
}

/// Marches along the intersection curve in one direction.
fn march(
    s1: &dyn Surface,
    s2: &dyn Surface,
    seed: &SsiSeed,
    tolerance: f64,
    direction: f64,
) -> Vec<SsiSeed> {
    let max_steps = 200;
    let step_size = tolerance * 5.0; // adaptive would be better

    let mut result = Vec::new();
    let mut u1 = seed.u1;
    let mut v1 = seed.v1;
    let mut u2 = seed.u2;
    let mut v2 = seed.v2;

    let (u10, u11) = s1.domain_u();
    let (v10, v11) = s1.domain_v();
    let (u20, u21) = s2.domain_u();
    let (v20, v21) = s2.domain_v();

    for _ in 0..max_steps {
        // Tangent direction: n1 × n2
        let n1 = s1.normal_at(u1, v1);
        let n2 = s2.normal_at(u2, v2);
        let tangent = n1.cross(n2);
        let len = tangent.length();
        if len < 1e-14 {
            break; // surfaces are tangent — can't march
        }
        let tangent = tangent / len * direction;

        // Predictor: step along tangent
        let p = s1.point_at(u1, v1);
        let predicted = p + tangent * step_size;

        // Corrector: project back onto both surfaces
        let (nu1, nv1, _) = s1.project_point(predicted);
        let p1 = s1.point_at(nu1, nv1);
        let (nu2, nv2, _) = s2.project_point(p1);
        let p2 = s2.point_at(nu2, nv2);

        let dist = p1.distance_to(p2);
        if dist > tolerance * 100.0 {
            break; // diverged
        }

        // Refine
        if let Some(refined) = refine_ssi_point(s1, s2, nu1, nv1, nu2, nv2, tolerance) {
            // Check domain bounds
            if refined.u1 < u10 || refined.u1 > u11 || refined.v1 < v10 || refined.v1 > v11
                || refined.u2 < u20 || refined.u2 > u21 || refined.v2 < v20 || refined.v2 > v21
            {
                break;
            }

            u1 = refined.u1;
            v1 = refined.v1;
            u2 = refined.u2;
            v2 = refined.v2;
            result.push(refined);
        } else {
            break;
        }
    }

    result
}

/// Refines an SSI point using Newton iteration on both surfaces.
fn refine_ssi_point(
    s1: &dyn Surface,
    s2: &dyn Surface,
    u1: f64,
    v1: f64,
    u2: f64,
    v2: f64,
    tolerance: f64,
) -> Option<SsiSeed> {
    let mut u1 = u1;
    let mut v1 = v1;
    let mut u2 = u2;
    let mut v2 = v2;

    for _ in 0..20 {
        let p1 = s1.point_at(u1, v1);
        let p2 = s2.point_at(u2, v2);
        let diff = p1 - p2;
        let dist = (diff.x * diff.x + diff.y * diff.y + diff.z * diff.z).sqrt();

        if dist < tolerance * 0.01 {
            let mid = Point3::new(
                (p1.x + p2.x) / 2.0,
                (p1.y + p2.y) / 2.0,
                (p1.z + p2.z) / 2.0,
            );
            return Some(SsiSeed {
                u1,
                v1,
                u2,
                v2,
                point: mid,
            });
        }

        // Project p2 onto s1 and p1 onto s2 alternately
        let (nu1, nv1, _) = s1.project_point(p2);
        let (nu2, nv2, _) = s2.project_point(p1);
        u1 = nu1;
        v1 = nv1;
        u2 = nu2;
        v2 = nv2;
    }

    let p1 = s1.point_at(u1, v1);
    let p2 = s2.point_at(u2, v2);
    if p1.distance_to(p2) < tolerance {
        Some(SsiSeed {
            u1,
            v1,
            u2,
            v2,
            point: Point3::new(
                (p1.x + p2.x) / 2.0,
                (p1.y + p2.y) / 2.0,
                (p1.z + p2.z) / 2.0,
            ),
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::plane::Plane;
    use crate::surface::sphere::Sphere;

    #[test]
    fn test_ssi_seeds_plane_sphere() {
        let plane = Plane::xy().unwrap();
        let sphere = Sphere::new(Point3::ORIGIN, 1.0).unwrap();
        let seeds = ssi_starting_points(&plane, &sphere, 0.01);
        // Plane z=0 intersects unit sphere at a circle of radius 1 on z=0
        assert!(
            !seeds.is_empty(),
            "plane-sphere intersection should have seeds"
        );
        // All seed points should be near z=0 and at radius ~1
        for seed in &seeds {
            assert!(
                seed.point.z.abs() < 0.1,
                "seed z should be ~0: {:?}",
                seed.point
            );
            let r = (seed.point.x * seed.point.x + seed.point.y * seed.point.y).sqrt();
            assert!(
                (r - 1.0).abs() < 0.1,
                "seed radius should be ~1: r={r}"
            );
        }
    }

    #[test]
    fn test_ssi_two_spheres() {
        let s1 = Sphere::new(Point3::new(-0.5, 0.0, 0.0), 1.0).unwrap();
        let s2 = Sphere::new(Point3::new(0.5, 0.0, 0.0), 1.0).unwrap();
        // Use generous tolerance — sphere project_point uses coarse 16×16 grid
        let seeds = ssi_starting_points(&s1, &s2, 0.15);
        assert!(
            !seeds.is_empty(),
            "two overlapping spheres should have SSI seeds"
        );
    }
}
