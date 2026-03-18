//! Cross-section operation: computes multiple parallel cross-sections of a solid.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, Handle, SolidData};

use super::section::{SectionResult, section_solid};
use super::copy_utils::collect_solid_faces;

/// Creates multiple cross-sections of a solid at regular intervals along a direction.
///
/// The solid's extent along `direction` is determined from its vertex positions.
/// `num_sections` equally spaced planes are created within that extent (excluding
/// the exact boundary planes). Each plane is passed to `section_solid` to compute
/// the intersection contour.
pub fn cross_sections(
    model: &BRepModel,
    solid: Handle<SolidData>,
    direction: Vec3,
    num_sections: usize,
) -> KernelResult<Vec<SectionResult>> {
    if num_sections == 0 {
        return Err(KernelError::InvalidArgument(
            "cross_sections requires at least 1 section".into(),
        ));
    }
    let dir = direction.normalized().ok_or(KernelError::InvalidArgument(
        "cross_sections direction must be non-zero".into(),
    ))?;

    // Compute bounding extent along direction from solid vertices
    let face_handles = collect_solid_faces(model, solid)?;
    let mut min_proj = f64::MAX;
    let mut max_proj = f64::MIN;

    for &face_h in &face_handles {
        if let Ok(verts) = model.vertices_of_face(face_h) {
            for &vh in &verts {
                if let Some(vd) = model.vertices.get(vh) {
                    let proj = vd.point.x * dir.x + vd.point.y * dir.y + vd.point.z * dir.z;
                    min_proj = min_proj.min(proj);
                    max_proj = max_proj.max(proj);
                }
            }
        }
    }

    if min_proj >= max_proj {
        return Ok(Vec::new());
    }

    let mut results = Vec::with_capacity(num_sections);
    let step = (max_proj - min_proj) / (num_sections + 1) as f64;

    for i in 1..=num_sections {
        let t = min_proj + step * i as f64;
        let plane_point = Point3::new(dir.x * t, dir.y * t, dir.z * t);
        let section = section_solid(model, solid, plane_point, direction)?;
        results.push(section);
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_sections_box_three_slices() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let sections = cross_sections(&model, b.solid, Vec3::Z, 3).unwrap();
        assert_eq!(sections.len(), 3);
        // Each section through a box should produce 4 edges (one per lateral face)
        for (i, s) in sections.iter().enumerate() {
            assert_eq!(
                s.edges.len(),
                4,
                "section {} should have 4 edges, got {}",
                i,
                s.edges.len()
            );
        }
    }

    #[test]
    fn test_cross_sections_single() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let sections = cross_sections(&model, b.solid, Vec3::Z, 1).unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].edges.len(), 4);
    }

    #[test]
    fn test_cross_sections_validation() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        assert!(cross_sections(&model, b.solid, Vec3::Z, 0).is_err());
        assert!(cross_sections(&model, b.solid, Vec3::ZERO, 1).is_err());
    }
}
