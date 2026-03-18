//! Mirror operation for solids.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, FaceData, Handle, SolidData};

use super::copy_utils::copy_solid_transformed;

/// Result of a mirror operation.
#[derive(Debug)]
pub struct MirrorResult {
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Mirrors a solid about a plane defined by a point and normal.
///
/// A **new** solid is created; the original is not modified.
/// The winding is reversed to maintain outward-facing normals.
pub fn mirror_solid(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    plane_point: Point3,
    plane_normal: Vec3,
) -> KernelResult<MirrorResult> {
    let n = plane_normal
        .normalized()
        .ok_or(KernelError::InvalidArgument(
            "mirror plane normal must be non-zero".into(),
        ))?;
    let op = model.history.next_operation("mirror_solid");

    let result = copy_solid_transformed(
        model,
        solid,
        op,
        |pt| {
            // Reflect point across the plane: p' = p - 2 * dot(p - plane_point, n) * n
            let d = (pt - plane_point).dot(n);
            Point3::new(
                pt.x - 2.0 * d * n.x,
                pt.y - 2.0 * d * n.y,
                pt.z - 2.0 * d * n.z,
            )
        },
        true, // reverse winding (reflection flips orientation)
    )?;

    Ok(MirrorResult {
        solid: result.solid,
        faces: result.faces,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::make_box;

    #[test]
    fn test_mirror_box_entity_counts() {
        let mut model = cadkernel_topology::BRepModel::new();
        let r = make_box(&mut model, Point3::new(1.0, 0.0, 0.0), 2.0, 2.0, 2.0).unwrap();

        let mr = mirror_solid(&mut model, r.solid, Point3::ORIGIN, Vec3::X).unwrap();

        // Mirror should create another 6 faces
        assert_eq!(mr.faces.len(), 6);
        assert_eq!(model.solids.len(), 2);
    }

    #[test]
    fn test_mirror_box_vertex_positions() {
        let mut model = cadkernel_topology::BRepModel::new();
        let r = make_box(&mut model, Point3::new(1.0, 0.0, 0.0), 2.0, 2.0, 2.0).unwrap();

        let _mr = mirror_solid(&mut model, r.solid, Point3::ORIGIN, Vec3::X).unwrap();

        // Original box: x in [1, 3]. Mirrored across x=0: x in [-3, -1].
        let mirrored_has_neg = model.vertices.iter().any(|(_, v)| v.point.x < -0.5);
        assert!(mirrored_has_neg, "expected mirrored vertices at negative x");
    }
}
