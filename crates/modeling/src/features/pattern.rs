//! Linear and circular pattern (array) operations.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Quaternion, Vec3};
use cadkernel_topology::{BRepModel, FaceData, Handle, SolidData};

use super::copy_utils::copy_solid_transformed;

/// Result of a pattern operation.
#[derive(Debug)]
pub struct PatternResult {
    pub solids: Vec<Handle<SolidData>>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Creates a linear pattern of a solid along a direction.
///
/// `count` copies are placed at `spacing` intervals along `direction`.
/// The original solid is included as the first entry.
pub fn linear_pattern(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    direction: Vec3,
    spacing: f64,
    count: usize,
) -> KernelResult<PatternResult> {
    if count < 2 {
        return Err(KernelError::InvalidArgument(
            "pattern count must be at least 2".into(),
        ));
    }
    let dir = direction
        .normalized()
        .ok_or(KernelError::InvalidArgument(
            "pattern direction must be non-zero".into(),
        ))?;

    let mut solids = vec![solid];
    let mut faces = Vec::new();

    for i in 1..count {
        let offset_x = dir.x * spacing * i as f64;
        let offset_y = dir.y * spacing * i as f64;
        let offset_z = dir.z * spacing * i as f64;

        let op = model.history.next_operation("linear_pattern");
        let result = copy_solid_transformed(
            model,
            solid,
            op,
            |pt| Point3::new(pt.x + offset_x, pt.y + offset_y, pt.z + offset_z),
            false,
        )?;
        solids.push(result.solid);
        faces.extend(result.faces);
    }

    Ok(PatternResult { solids, faces })
}

/// Creates a circular pattern of a solid around an axis.
///
/// `count` copies are placed at equal angular intervals (full 360°) around
/// the axis defined by `axis_origin` and `axis_dir`.
/// The original solid is included as the first entry.
pub fn circular_pattern(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    axis_origin: Point3,
    axis_dir: Vec3,
    count: usize,
) -> KernelResult<PatternResult> {
    if count < 2 {
        return Err(KernelError::InvalidArgument(
            "pattern count must be at least 2".into(),
        ));
    }
    let axis = axis_dir
        .normalized()
        .ok_or(KernelError::InvalidArgument(
            "pattern axis must be non-zero".into(),
        ))?;

    let angle_step = std::f64::consts::TAU / count as f64;

    let mut solids = vec![solid];
    let mut faces = Vec::new();

    for i in 1..count {
        let angle = angle_step * i as f64;
        let q = Quaternion::from_axis_angle(axis, angle);

        let origin = axis_origin;
        let op = model.history.next_operation("circular_pattern");
        let result = copy_solid_transformed(
            model,
            solid,
            op,
            move |pt| {
                // Translate to origin, rotate, translate back.
                let rel = Vec3::new(pt.x - origin.x, pt.y - origin.y, pt.z - origin.z);
                let rotated = q.rotate_vec(rel);
                Point3::new(
                    origin.x + rotated.x,
                    origin.y + rotated.y,
                    origin.z + rotated.z,
                )
            },
            false,
        )?;
        solids.push(result.solid);
        faces.extend(result.faces);
    }

    Ok(PatternResult { solids, faces })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::make_box;

    #[test]
    fn test_linear_pattern_3_copies() {
        let mut model = cadkernel_topology::BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let pr = linear_pattern(&mut model, r.solid, Vec3::X, 3.0, 3).unwrap();
        // 3 solids total (original + 2 copies)
        assert_eq!(pr.solids.len(), 3);
        // 2 copies * 6 faces = 12 new faces
        assert_eq!(pr.faces.len(), 12);
        assert_eq!(model.solids.len(), 3);
    }

    #[test]
    fn test_circular_pattern_4_copies() {
        let mut model = cadkernel_topology::BRepModel::new();
        let r = make_box(&mut model, Point3::new(3.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap();

        let pr = circular_pattern(&mut model, r.solid, Point3::ORIGIN, Vec3::Z, 4).unwrap();
        assert_eq!(pr.solids.len(), 4);
        // 3 copies * 6 faces = 18 new faces
        assert_eq!(pr.faces.len(), 18);

        // Check that a copy exists roughly at (0, 3, 0) — 90° rotation around Z
        let has_rotated = model
            .vertices
            .iter()
            .any(|(_, v)| (v.point.y - 3.0).abs() < 0.5 && v.point.x.abs() < 0.5);
        assert!(has_rotated, "expected rotated vertex near (0, 3, 0)");
    }

    #[test]
    fn test_pattern_validation() {
        let mut model = cadkernel_topology::BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        assert!(linear_pattern(&mut model, r.solid, Vec3::X, 2.0, 1).is_err());
        assert!(circular_pattern(&mut model, r.solid, Point3::ORIGIN, Vec3::Z, 1).is_err());
    }
}
