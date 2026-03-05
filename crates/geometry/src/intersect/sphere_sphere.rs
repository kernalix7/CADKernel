use cadkernel_math::tolerance;

use super::types::SsiResult;
use crate::surface::sphere::Sphere;

/// Computes the intersection of two spheres.
///
/// Returns `Circle` if they intersect in a ring, `Point` if tangent,
/// `Coincident` if same sphere, or `Empty`.
pub fn intersect_sphere_sphere(a: &Sphere, b: &Sphere) -> SsiResult {
    let ab = b.center - a.center;
    let d = ab.length();

    if tolerance::is_zero(d)
        && tolerance::approx_eq_tol(a.radius, b.radius, cadkernel_math::EPSILON)
    {
        return SsiResult::Coincident;
    }

    if d > a.radius + b.radius + cadkernel_math::EPSILON {
        return SsiResult::Empty;
    }

    if d < (a.radius - b.radius).abs() - cadkernel_math::EPSILON {
        return SsiResult::Empty;
    }

    let Some(normal) = ab.normalized() else {
        return SsiResult::Empty;
    };

    // Tangent cases
    if tolerance::approx_eq_tol(d, a.radius + b.radius, cadkernel_math::EPSILON) {
        let point = a.center + normal * a.radius;
        return SsiResult::Point(point);
    }
    if tolerance::approx_eq_tol(d, (a.radius - b.radius).abs(), cadkernel_math::EPSILON) {
        let point = a.center + normal * a.radius;
        return SsiResult::Point(point);
    }

    // General case: intersection circle
    // h = distance from center_a to the intersection plane along the line ab
    let h = (d * d + a.radius * a.radius - b.radius * b.radius) / (2.0 * d);
    let radius = (a.radius * a.radius - h * h).sqrt();
    let center = a.center + normal * h;

    SsiResult::Circle {
        center,
        normal,
        radius,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_math::{EPSILON, Point3};

    #[test]
    fn test_unit_spheres_offset() {
        let a = Sphere::new(Point3::ORIGIN, 1.0);
        let b = Sphere::new(Point3::new(1.0, 0.0, 0.0), 1.0);
        match intersect_sphere_sphere(&a, &b) {
            SsiResult::Circle { center, radius, .. } => {
                assert!((center.x - 0.5).abs() < EPSILON);
                assert!(radius > 0.0);
            }
            _ => panic!("expected circle"),
        }
    }

    #[test]
    fn test_tangent_spheres() {
        let a = Sphere::new(Point3::ORIGIN, 1.0);
        let b = Sphere::new(Point3::new(2.0, 0.0, 0.0), 1.0);
        assert!(matches!(
            intersect_sphere_sphere(&a, &b),
            SsiResult::Point(_)
        ));
    }

    #[test]
    fn test_no_intersection() {
        let a = Sphere::new(Point3::ORIGIN, 1.0);
        let b = Sphere::new(Point3::new(5.0, 0.0, 0.0), 1.0);
        assert!(matches!(intersect_sphere_sphere(&a, &b), SsiResult::Empty));
    }
}
