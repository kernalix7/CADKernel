//! Defeaturing operations: remove or simplify faces from a solid.

use std::collections::{HashMap, HashSet};

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_topology::{BRepModel, EntityKind, FaceData, Handle, SolidData, Tag, VertexData};

use super::copy_utils::collect_solid_faces;

/// Removes a face from a solid and rebuilds the solid without it.
///
/// A new solid is created containing all faces except the specified one.
/// The original solid is not modified. This is a simplified defeaturing
/// that does not attempt to patch the resulting hole.
pub fn remove_face(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    face: Handle<FaceData>,
) -> KernelResult<Handle<SolidData>> {
    let op = model.history.next_operation("remove_face");
    let all_faces = collect_solid_faces(model, solid)?;

    let remove_idx = face.index();
    let remaining: Vec<Handle<FaceData>> = all_faces
        .iter()
        .filter(|h| h.index() != remove_idx)
        .copied()
        .collect();

    if remaining.is_empty() {
        return Err(KernelError::InvalidArgument(
            "cannot remove the only face from a solid".into(),
        ));
    }
    if remaining.len() == all_faces.len() {
        return Err(KernelError::InvalidArgument(
            "face not found in solid".into(),
        ));
    }

    rebuild_solid_from_faces(model, &remaining, op)
}

/// Removes all faces whose area is below `min_face_area` and rebuilds the solid.
///
/// Face area is estimated via tessellation (sum of triangle areas).
/// If all faces are below the threshold the operation fails.
pub fn simplify_solid(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    min_face_area: f64,
) -> KernelResult<Handle<SolidData>> {
    if min_face_area < 0.0 {
        return Err(KernelError::InvalidArgument(
            "min_face_area must be non-negative".into(),
        ));
    }

    let op = model.history.next_operation("simplify_solid");
    let all_faces = collect_solid_faces(model, solid)?;

    let mut small_faces: HashSet<u32> = HashSet::new();
    for &fh in &all_faces {
        let area = face_area(model, fh);
        if area < min_face_area {
            small_faces.insert(fh.index());
        }
    }

    let remaining: Vec<Handle<FaceData>> = all_faces
        .iter()
        .filter(|h| !small_faces.contains(&h.index()))
        .copied()
        .collect();

    if remaining.is_empty() {
        return Err(KernelError::InvalidArgument(
            "all faces are below the area threshold".into(),
        ));
    }

    rebuild_solid_from_faces(model, &remaining, op)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn face_area(model: &BRepModel, face: Handle<FaceData>) -> f64 {
    let triangles = cadkernel_io::tessellate_face(model, face);
    let mut area = 0.0_f64;
    for tri in &triangles {
        let a = tri.vertices[0];
        let b = tri.vertices[1];
        let c = tri.vertices[2];
        let ab = b - a;
        let ac = c - a;
        let cross_x = ab.y * ac.z - ab.z * ac.y;
        let cross_y = ab.z * ac.x - ab.x * ac.z;
        let cross_z = ab.x * ac.y - ab.y * ac.x;
        area += 0.5 * (cross_x * cross_x + cross_y * cross_y + cross_z * cross_z).sqrt();
    }
    area
}

/// Rebuilds a solid by deep-copying the given faces (preserving vertex positions).
fn rebuild_solid_from_faces(
    model: &mut BRepModel,
    faces: &[Handle<FaceData>],
    op: cadkernel_topology::OperationId,
) -> KernelResult<Handle<SolidData>> {
    let mut vert_map: HashMap<u32, Handle<VertexData>> = HashMap::new();
    let mut vert_idx = 0u32;

    // Map old vertices to new vertices
    for &fh in faces {
        let verts = model.vertices_of_face(fh)?;
        for &vh in &verts {
            let key = vh.index();
            if vert_map.contains_key(&key) {
                continue;
            }
            let pt = model
                .vertices
                .get(vh)
                .ok_or(KernelError::InvalidHandle("vertex"))?
                .point;
            let tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
            vert_map.insert(key, model.add_vertex_tagged(pt, tag));
            vert_idx += 1;
        }
    }

    // Recreate faces with new vertices
    let mut new_faces = Vec::with_capacity(faces.len());
    let mut edge_idx = 0u32;

    for (fi, &fh) in faces.iter().enumerate() {
        let verts = model.vertices_of_face(fh)?;
        let new_verts: Vec<Handle<VertexData>> =
            verts.iter().map(|vh| vert_map[&vh.index()]).collect();

        let n = new_verts.len();
        let mut half_edges = Vec::with_capacity(n);

        for j in 0..n {
            let v_start = new_verts[j];
            let v_end = new_verts[(j + 1) % n];
            let tag_e = Tag::generated(EntityKind::Edge, op, edge_idx);
            edge_idx += 1;
            let (_edge_h, he_a, _he_b) = model.add_edge_tagged(v_start, v_end, tag_e);
            half_edges.push(he_a);
        }

        let loop_h = model.make_loop(&half_edges)?;
        let tag_f = Tag::generated(EntityKind::Face, op, fi as u32);
        new_faces.push(model.make_face_tagged(loop_h, tag_f));
    }

    let tag_sh = Tag::generated(EntityKind::Shell, op, 0);
    let shell_h = model.make_shell_tagged(&new_faces, tag_sh);
    let tag_so = Tag::generated(EntityKind::Solid, op, 0);
    Ok(model.make_solid_tagged(&[shell_h], tag_so))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_math::Point3;

    #[test]
    fn test_remove_face_from_box() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 3.0, 4.0).unwrap();

        let face_to_remove = b.faces[0];
        let new_solid = remove_face(&mut model, b.solid, face_to_remove).unwrap();

        // Original box has 6 faces, new solid should have 5
        let sd = model.solids.get(new_solid).unwrap();
        let shell = model.shells.get(sd.shells[0]).unwrap();
        assert_eq!(shell.faces.len(), 5);
    }

    #[test]
    fn test_simplify_solid_no_removal() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 3.0, 4.0).unwrap();

        // All faces of a 2×3×4 box have area >= 6, so threshold 0.1 removes nothing
        let new_solid = simplify_solid(&mut model, b.solid, 0.1).unwrap();
        let sd = model.solids.get(new_solid).unwrap();
        let shell = model.shells.get(sd.shells[0]).unwrap();
        assert_eq!(shell.faces.len(), 6);
    }
}
