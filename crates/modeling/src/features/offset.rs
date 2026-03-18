//! Offset solid — creates a new solid with all faces offset by a distance.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, FaceData, Handle, SolidData};

use super::copy_utils::{collect_solid_faces, copy_solid_transformed};

/// Result of an offset operation.
pub struct OffsetResult {
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Creates a new solid by offsetting all vertices along their averaged face normals.
///
/// `distance > 0` expands the solid outward, `distance < 0` shrinks inward.
/// This is a vertex-based offset: each vertex is moved along the average
/// normal of its adjacent faces.
pub fn offset_solid(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    distance: f64,
) -> KernelResult<OffsetResult> {
    if distance.abs() < 1e-14 {
        return Err(KernelError::InvalidArgument(
            "offset distance must be non-zero".into(),
        ));
    }

    let face_handles = collect_solid_faces(model, solid)?;

    // Compute per-vertex normal by averaging adjacent face normals
    let mut vertex_normals: std::collections::HashMap<u32, Vec3> = std::collections::HashMap::new();

    for &face_h in &face_handles {
        let verts = model.vertices_of_face(face_h)?;
        if verts.len() < 3 {
            continue;
        }

        let positions: Vec<Point3> = verts
            .iter()
            .map(|&vh| {
                model
                    .vertices
                    .get(vh)
                    .map(|v| v.point)
                    .unwrap_or(Point3::ORIGIN)
            })
            .collect();

        // Compute face normal from first 3 vertices
        let v01 = positions[1] - positions[0];
        let v02 = positions[2] - positions[0];
        let face_normal = v01.cross(v02);

        for &vh in &verts {
            vertex_normals
                .entry(vh.index())
                .and_modify(|n| *n += face_normal)
                .or_insert(face_normal);
        }
    }

    // Normalize all vertex normals
    for n in vertex_normals.values_mut() {
        if let Some(nn) = n.normalized() {
            *n = nn;
        }
    }

    let op = model.history.next_operation("offset_solid");

    let result = copy_solid_transformed(model, solid, op, |pt| {
        // Find this vertex's normal — we need to search by position matching
        // Since copy_solid_transformed gives us the original point, look up by position
        pt
    }, false)?;

    // Now offset the new vertices using the computed normals
    // We need to re-map: collect all vertices of the new solid and offset them
    let new_face_handles = collect_solid_faces(model, result.solid)?;
    let orig_face_handles = &face_handles;

    // Build position → normal map from original vertices
    let mut pos_to_normal: std::collections::HashMap<u64, Vec3> = std::collections::HashMap::new();
    for &face_h in orig_face_handles {
        let verts = model.vertices_of_face(face_h)?;
        for &vh in &verts {
            if let Some(vd) = model.vertices.get(vh) {
                let key = point_key(vd.point);
                if let Some(&n) = vertex_normals.get(&vh.index()) {
                    pos_to_normal.insert(key, n);
                }
            }
        }
    }

    // Offset new vertices
    let mut moved = std::collections::HashSet::new();
    for &face_h in &new_face_handles {
        let verts = model.vertices_of_face(face_h)?;
        for &vh in &verts {
            if moved.contains(&vh.index()) {
                continue;
            }
            moved.insert(vh.index());
            if let Some(vd) = model.vertices.get_mut(vh) {
                let key = point_key(vd.point);
                if let Some(&n) = pos_to_normal.get(&key) {
                    vd.point = Point3::new(
                        vd.point.x + n.x * distance,
                        vd.point.y + n.y * distance,
                        vd.point.z + n.z * distance,
                    );
                }
            }
        }
    }

    Ok(OffsetResult {
        solid: result.solid,
        faces: result.faces,
    })
}

fn point_key(p: Point3) -> u64 {
    let xb = p.x.to_bits();
    let yb = p.y.to_bits();
    let zb = p.z.to_bits();
    xb ^ yb.rotate_left(21) ^ zb.rotate_left(42)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offset_box_outward() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let result = offset_solid(&mut model, b.solid, 1.0).unwrap();
        assert!(model.solids.is_alive(result.solid));
    }

    #[test]
    fn test_offset_box_inward() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let result = offset_solid(&mut model, b.solid, -0.5).unwrap();
        assert!(model.solids.is_alive(result.solid));
    }

    #[test]
    fn test_offset_zero_distance() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        assert!(offset_solid(&mut model, b.solid, 0.0).is_err());
    }
}
