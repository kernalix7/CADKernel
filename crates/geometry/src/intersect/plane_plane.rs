use cadkernel_math::{Point3, Vec3, tolerance};

use super::types::SsiResult;
use crate::surface::plane::Plane;

/// Computes the intersection of two planes.
///
/// Returns a `Line` if the planes are not parallel, `Coincident` if they
/// are the same plane, or `Empty` if they are parallel but distinct.
pub fn intersect_plane_plane(a: &Plane, b: &Plane) -> SsiResult {
    let na = a.normal();
    let nb = b.normal();
    let dir = na.cross(nb);

    if tolerance::is_zero(dir.length()) {
        let d = b.origin - a.origin;
        if tolerance::is_zero(na.dot(d)) {
            return SsiResult::Coincident;
        }
        return SsiResult::Empty;
    }

    let Some(direction) = dir.normalized() else {
        return SsiResult::Empty;
    };

    // Find a point on the intersection line.
    // Solve: na . (p - a.origin) = 0  and  nb . (p - b.origin) = 0
    // We pick the point closest to the origin by solving the 2x3 system.
    let d1 = na.dot(Vec3::new(a.origin.x, a.origin.y, a.origin.z));
    let d2 = nb.dot(Vec3::new(b.origin.x, b.origin.y, b.origin.z));

    let n1n2 = na.dot(nb);
    let n1n1 = na.dot(na);
    let n2n2 = nb.dot(nb);
    let det = n1n1 * n2n2 - n1n2 * n1n2;

    let c1 = (d1 * n2n2 - d2 * n1n2) / det;
    let c2 = (d2 * n1n1 - d1 * n1n2) / det;

    let origin = Point3::new(
        c1 * na.x + c2 * nb.x,
        c1 * na.y + c2 * nb.y,
        c1 * na.z + c2 * nb.z,
    );

    SsiResult::Line { origin, direction }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_math::EPSILON;

    #[test]
    fn test_xy_xz_intersection() {
        let xy = Plane::xy().unwrap();
        let xz = Plane::xz().unwrap();
        match intersect_plane_plane(&xy, &xz) {
            SsiResult::Line { origin, direction } => {
                assert!(tolerance::is_zero(origin.y));
                assert!(tolerance::is_zero(origin.z));
                assert!((direction.cross(Vec3::X).length()) < EPSILON);
            }
            _ => panic!("expected line intersection"),
        }
    }

    #[test]
    fn test_parallel_planes() {
        let a = Plane::xy().unwrap();
        let b = Plane::new(Point3::new(0.0, 0.0, 5.0), Vec3::X, Vec3::Y).unwrap();
        assert!(matches!(intersect_plane_plane(&a, &b), SsiResult::Empty));
    }

    #[test]
    fn test_coincident_planes() {
        let a = Plane::xy().unwrap();
        let b = Plane::xy().unwrap();
        assert!(matches!(
            intersect_plane_plane(&a, &b),
            SsiResult::Coincident
        ));
    }
}
