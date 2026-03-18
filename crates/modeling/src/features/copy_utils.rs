//! Internal utilities for deep-copying B-Rep topology during feature operations.

use std::collections::HashMap;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::Point3;
use cadkernel_topology::{
    BRepModel, EntityKind, FaceData, Handle, OperationId, ShellData, SolidData, Tag, VertexData,
};

/// Deep-copies the faces of a solid, applying a point transform to all vertices.
///
/// Returns handles to the newly created faces, shell, and solid.
pub(crate) fn copy_solid_transformed(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    op: OperationId,
    transform_point: impl Fn(Point3) -> Point3,
    reverse_winding: bool,
) -> KernelResult<CopyResult> {
    // Collect all faces from the solid's shells.
    let face_handles = collect_solid_faces(model, solid)?;

    // Map old vertex handles → new vertex handles (transformed).
    let mut vert_map: HashMap<u32, Handle<VertexData>> = HashMap::new();
    let mut vert_idx = 0u32;

    for &face_h in &face_handles {
        let verts = model.vertices_of_face(face_h)?;
        for &vh in &verts {
            let key = vh.index();
            if vert_map.contains_key(&key) {
                continue;
            }
            let old_pt = model
                .vertices
                .get(vh)
                .ok_or(KernelError::InvalidHandle("vertex"))?
                .point;
            let new_pt = transform_point(old_pt);
            let tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
            vert_map.insert(key, model.add_vertex_tagged(new_pt, tag));
            vert_idx += 1;
        }
    }

    // Recreate faces with new vertices.
    let mut new_faces = Vec::with_capacity(face_handles.len());
    let mut edge_idx = 0u32;

    for (fi, &face_h) in face_handles.iter().enumerate() {
        let verts = model.vertices_of_face(face_h)?;
        let mut new_verts: Vec<Handle<VertexData>> =
            verts.iter().map(|vh| vert_map[&vh.index()]).collect();

        if reverse_winding {
            new_verts.reverse();
        }

        let n = new_verts.len();
        let mut half_edges = Vec::with_capacity(n);
        for i in 0..n {
            let j = (i + 1) % n;
            let etag = Tag::generated(EntityKind::Edge, op, edge_idx);
            let (_, he_a, _) = model.add_edge_tagged(new_verts[i], new_verts[j], etag);
            half_edges.push(he_a);
            edge_idx += 1;
        }

        let loop_h = model.make_loop(&half_edges)?;
        let face_tag = Tag::generated(EntityKind::Face, op, fi as u32);
        new_faces.push(model.make_face_tagged(loop_h, face_tag));
    }

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&new_faces, shell_tag);

    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let new_solid = model.make_solid_tagged(&[shell], solid_tag);

    Ok(CopyResult {
        solid: new_solid,
        shell,
        faces: new_faces,
    })
}

/// Result of a solid copy operation.
pub(crate) struct CopyResult {
    pub solid: Handle<SolidData>,
    #[allow(dead_code)]
    pub shell: Handle<ShellData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Collects all face handles from a solid (across all shells).
pub(crate) fn collect_solid_faces(
    model: &BRepModel,
    solid: Handle<SolidData>,
) -> KernelResult<Vec<Handle<FaceData>>> {
    let sd = model
        .solids
        .get(solid)
        .ok_or(KernelError::InvalidHandle("solid"))?;
    let mut faces = Vec::new();
    for &shell_h in &sd.shells {
        let sh = model
            .shells
            .get(shell_h)
            .ok_or(KernelError::InvalidHandle("shell"))?;
        faces.extend_from_slice(&sh.faces);
    }
    Ok(faces)
}
