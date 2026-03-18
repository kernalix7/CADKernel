use cadkernel_math::{Point3, Vec3, tolerance};

use super::types::RayHit;
use crate::surface::cylinder::Cylinder;
use crate::surface::plane::Plane;
use crate::surface::sphere::Sphere;

/// Intersects a ray/line with a plane.
pub fn intersect_line_plane(origin: Point3, direction: Vec3, plane: &Plane) -> Option<RayHit> {
    let n = plane.normal();
    let denom = n.dot(direction);
    if tolerance::is_zero(denom) {
        return None;
    }
    let oc = plane.origin - origin;
    let t = n.dot(oc) / denom;
    let point = origin + direction * t;
    Some(RayHit { t, point })
}

/// Intersects a ray/line with a sphere. Returns 0, 1, or 2 hits.
pub fn intersect_line_sphere(origin: Point3, direction: Vec3, sphere: &Sphere) -> Vec<RayHit> {
    let oc = origin - sphere.center;
    let a = direction.dot(direction);
    let b = 2.0 * oc.dot(direction);
    let c = oc.dot(oc) - sphere.radius * sphere.radius;
    let discriminant = b * b - 4.0 * a * c;

    if discriminant < -cadkernel_math::EPSILON {
        return vec![];
    }

    if discriminant.abs() < cadkernel_math::EPSILON {
        let t = -b / (2.0 * a);
        return vec![RayHit {
            t,
            point: origin + direction * t,
        }];
    }

    let sqrt_d = discriminant.sqrt();
    let t1 = (-b - sqrt_d) / (2.0 * a);
    let t2 = (-b + sqrt_d) / (2.0 * a);
    vec![
        RayHit {
            t: t1,
            point: origin + direction * t1,
        },
        RayHit {
            t: t2,
            point: origin + direction * t2,
        },
    ]
}

/// Intersects a ray/line with an infinite cylinder. Returns 0, 1, or 2 hits.
pub fn intersect_line_cylinder(origin: Point3, direction: Vec3, cyl: &Cylinder) -> Vec<RayHit> {
    let axis = cyl.axis;
    let delta = origin - cyl.base_center;

    // Project out the axis component
    let d_perp = direction - axis * direction.dot(axis);
    let delta_perp = delta - axis * delta.dot(axis);

    let a = d_perp.dot(d_perp);
    let b = 2.0 * d_perp.dot(delta_perp);
    let c = delta_perp.dot(delta_perp) - cyl.radius * cyl.radius;

    let discriminant = b * b - 4.0 * a * c;

    if discriminant < -cadkernel_math::EPSILON || tolerance::is_zero(a) {
        return vec![];
    }

    if discriminant.abs() < cadkernel_math::EPSILON {
        let t = -b / (2.0 * a);
        return vec![RayHit {
            t,
            point: origin + direction * t,
        }];
    }

    let sqrt_d = discriminant.sqrt();
    let t1 = (-b - sqrt_d) / (2.0 * a);
    let t2 = (-b + sqrt_d) / (2.0 * a);
    vec![
        RayHit {
            t: t1,
            point: origin + direction * t1,
        },
        RayHit {
            t: t2,
            point: origin + direction * t2,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_math::EPSILON;

    #[test]
    fn test_line_plane_hit() {
        let plane = Plane::xy().unwrap();
        let hit = intersect_line_plane(
            Point3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, -1.0),
            &plane,
        )
        .unwrap();
        assert!((hit.t - 1.0).abs() < EPSILON);
        assert!(hit.point.approx_eq(Point3::ORIGIN));
    }

    #[test]
    fn test_line_plane_parallel() {
        let plane = Plane::xy().unwrap();
        let hit = intersect_line_plane(Point3::new(0.0, 0.0, 1.0), Vec3::X, &plane);
        assert!(hit.is_none());
    }

    #[test]
    fn test_line_sphere_two_hits() {
        let sphere = Sphere::new(Point3::ORIGIN, 1.0).unwrap();
        let hits = intersect_line_sphere(Point3::new(-2.0, 0.0, 0.0), Vec3::X, &sphere);
        assert_eq!(hits.len(), 2);
        assert!((hits[0].point.x - (-1.0)).abs() < EPSILON);
        assert!((hits[1].point.x - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_line_sphere_miss() {
        let sphere = Sphere::new(Point3::ORIGIN, 1.0).unwrap();
        let hits = intersect_line_sphere(Point3::new(0.0, 5.0, 0.0), Vec3::X, &sphere);
        assert!(hits.is_empty());
    }

    #[test]
    fn test_line_cylinder_two_hits() {
        let cyl = Cylinder::z_axis(1.0, 5.0);
        let hits = intersect_line_cylinder(Point3::new(-2.0, 0.0, 2.5), Vec3::X, &cyl);
        assert_eq!(hits.len(), 2);
        assert!((hits[0].point.x - (-1.0)).abs() < EPSILON);
        assert!((hits[1].point.x - 1.0).abs() < EPSILON);
    }
}
