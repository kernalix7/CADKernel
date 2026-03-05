use cadkernel_math::tolerance;

use super::types::SsiResult;
use crate::surface::plane::Plane;
use crate::surface::sphere::Sphere;

/// Computes the intersection of a plane and a sphere.
///
/// Returns `Circle` if the plane slices the sphere, `Point` if tangent,
/// or `Empty` if they don't intersect.
pub fn intersect_plane_sphere(plane: &Plane, sphere: &Sphere) -> SsiResult {
    let n = plane.normal();
    let oc = sphere.center - plane.origin;
    let dist = n.dot(oc);

    let abs_dist = dist.abs();

    if abs_dist > sphere.radius + cadkernel_math::EPSILON {
        return SsiResult::Empty;
    }

    if tolerance::approx_eq_tol(abs_dist, sphere.radius, cadkernel_math::EPSILON) {
        let point = sphere.center + n * (-dist);
        return SsiResult::Point(point);
    }

    let center = sphere.center + n * (-dist);
    let radius = (sphere.radius * sphere.radius - dist * dist).sqrt();

    SsiResult::Circle {
        center,
        normal: n,
        radius,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_math::{EPSILON, Point3, Vec3};

    #[test]
    fn test_equator_intersection() {
        let plane = Plane::xy().unwrap();
        let sphere = Sphere::new(Point3::ORIGIN, 1.0);
        match intersect_plane_sphere(&plane, &sphere) {
            SsiResult::Circle {
                center,
                normal: _,
                radius,
            } => {
                assert!(center.approx_eq(Point3::ORIGIN));
                assert!((radius - 1.0).abs() < EPSILON);
            }
            _ => panic!("expected circle"),
        }
    }

    #[test]
    fn test_tangent() {
        let plane = Plane::new(Point3::new(0.0, 0.0, 1.0), Vec3::X, Vec3::Y).unwrap();
        let sphere = Sphere::new(Point3::ORIGIN, 1.0);
        assert!(matches!(
            intersect_plane_sphere(&plane, &sphere),
            SsiResult::Point(_)
        ));
    }

    #[test]
    fn test_no_intersection() {
        let plane = Plane::new(Point3::new(0.0, 0.0, 5.0), Vec3::X, Vec3::Y).unwrap();
        let sphere = Sphere::new(Point3::ORIGIN, 1.0);
        assert!(matches!(
            intersect_plane_sphere(&plane, &sphere),
            SsiResult::Empty
        ));
    }
}
