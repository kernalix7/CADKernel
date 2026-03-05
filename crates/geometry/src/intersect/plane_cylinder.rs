use cadkernel_math::{Vec3, tolerance};

use super::types::{IntersectionEllipse, SsiResult};
use crate::surface::cylinder::Cylinder;
use crate::surface::plane::Plane;

/// Computes the intersection of a plane and an (infinite) cylinder.
///
/// The cylinder is treated as infinitely long along its axis for the SSI computation.
/// Returns `Ellipse` (general case), `Line` pair degenerate cases, or `Empty`.
pub fn intersect_plane_cylinder(plane: &Plane, cyl: &Cylinder) -> SsiResult {
    let n = plane.normal();
    let axis = cyl.axis;
    let cos_angle = n.dot(axis).abs();

    // Plane perpendicular to cylinder axis → circle
    if tolerance::approx_eq_tol(cos_angle, 1.0, 1e-7) {
        let oc = cyl.base_center - plane.origin;
        let dist = n.dot(oc);
        if dist.abs() > cyl.height + cadkernel_math::EPSILON {
            return SsiResult::Empty;
        }
        let center = cyl.base_center + axis * (-dist / n.dot(axis));
        return SsiResult::Circle {
            center,
            normal: n,
            radius: cyl.radius,
        };
    }

    // Plane parallel to cylinder axis → two lines or empty
    if tolerance::is_zero(cos_angle) {
        let to_plane = plane.origin - cyl.base_center;
        let perp = to_plane - axis * to_plane.dot(axis);
        let dist = perp.length();
        if dist > cyl.radius + cadkernel_math::EPSILON {
            return SsiResult::Empty;
        }
        if tolerance::approx_eq_tol(dist, cyl.radius, cadkernel_math::EPSILON) {
            let direction = axis;
            let origin = cyl.base_center + perp;
            return SsiResult::Line { origin, direction };
        }
        // Two tangent lines -- return one as approximation for the primary intersection
        let offset = (cyl.radius * cyl.radius - dist * dist).sqrt();
        let perp_dir = n.cross(axis).normalized().unwrap_or(Vec3::X);
        let origin =
            cyl.base_center + perp.normalized().unwrap_or(Vec3::X) * dist + perp_dir * offset;
        return SsiResult::Line {
            origin,
            direction: axis,
        };
    }

    // General case: ellipse
    // The semi-major axis = radius / sin(angle between plane normal and cylinder axis)
    let sin_angle = (1.0 - cos_angle * cos_angle).sqrt();
    let semi_major = cyl.radius / sin_angle;
    let semi_minor = cyl.radius;

    // Project cylinder axis onto the plane to get major axis direction
    let axis_proj = axis - n * n.dot(axis);
    let major_axis = axis_proj.normalized().unwrap_or(Vec3::X);
    let minor_axis = n.cross(major_axis).normalized().unwrap_or(Vec3::Y);

    // Center: project cylinder base center onto the plane along the cylinder axis
    let oc = plane.origin - cyl.base_center;
    let t = n.dot(oc) / n.dot(axis);
    let center = cyl.base_center + axis * t;

    SsiResult::Ellipse(IntersectionEllipse {
        center,
        normal: n,
        major_axis,
        minor_axis,
        semi_major,
        semi_minor,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_math::{EPSILON, Point3};

    #[test]
    fn test_perpendicular_cut_gives_circle() {
        let plane = Plane::new(Point3::new(0.0, 0.0, 2.0), Vec3::X, Vec3::Y).unwrap();
        let cyl = Cylinder::z_axis(1.0, 5.0);
        match intersect_plane_cylinder(&plane, &cyl) {
            SsiResult::Circle { radius, .. } => {
                assert!((radius - 1.0).abs() < EPSILON);
            }
            _ => panic!("expected circle"),
        }
    }

    #[test]
    fn test_oblique_cut_gives_ellipse() {
        let plane = Plane::new(
            Point3::new(0.0, 0.0, 2.0),
            Vec3::X,
            Vec3::new(0.0, 1.0, 1.0).normalized().unwrap(),
        )
        .unwrap();
        let cyl = Cylinder::z_axis(1.0, 5.0);
        match intersect_plane_cylinder(&plane, &cyl) {
            SsiResult::Ellipse(e) => {
                assert!(e.semi_major >= e.semi_minor - EPSILON);
            }
            other => panic!("expected ellipse, got {other:?}"),
        }
    }
}
