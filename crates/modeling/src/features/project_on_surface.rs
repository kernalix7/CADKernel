//! Project points onto a surface.

use cadkernel_geometry::Surface;
use cadkernel_math::Point3;

/// Projects points onto a surface using the surface's `project_point` method.
///
/// For each input point, the closest point on the surface is returned.
pub fn project_points_on_surface(
    surface: &dyn Surface,
    points: &[Point3],
) -> Vec<Point3> {
    points
        .iter()
        .map(|p| {
            let (_, _, proj) = surface.project_point(*p);
            proj
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_geometry::Plane;
    use cadkernel_math::Vec3;

    #[test]
    fn test_project_points_on_plane() {
        let plane = Plane::new(Point3::ORIGIN, Vec3::X, Vec3::Y).unwrap();
        let points = vec![
            Point3::new(1.0, 2.0, 5.0),
            Point3::new(-3.0, 4.0, -2.0),
            Point3::new(0.0, 0.0, 10.0),
        ];

        let projected = project_points_on_surface(&plane, &points);

        assert_eq!(projected.len(), 3);
        // All projected points should have z == 0 (projected onto XY plane)
        for (i, p) in projected.iter().enumerate() {
            assert!(
                p.z.abs() < 1e-10,
                "projected point {} z should be 0, got {}",
                i,
                p.z
            );
        }
        // X and Y should be preserved
        assert!((projected[0].x - 1.0).abs() < 1e-10);
        assert!((projected[0].y - 2.0).abs() < 1e-10);
        assert!((projected[1].x - (-3.0)).abs() < 1e-10);
        assert!((projected[1].y - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_project_empty_points() {
        let plane = Plane::new(Point3::ORIGIN, Vec3::X, Vec3::Y).unwrap();
        let projected = project_points_on_surface(&plane, &[]);
        assert!(projected.is_empty());
    }
}
