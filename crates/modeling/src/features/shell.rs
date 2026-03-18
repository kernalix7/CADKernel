//! Shell (hollowing) operation for solids.

use std::collections::{HashMap, HashSet};

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, EntityKind, FaceData, Handle, SolidData, Tag, VertexData};

use super::copy_utils::collect_solid_faces;

/// Result of a shell operation.
#[derive(Debug)]
pub struct ShellResult {
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Hollows out a solid by removing specified faces and offsetting the
/// remaining faces inward by `thickness`.
///
/// A **new** solid is created; the original is not modified.
///
/// Algorithm:
/// 1. Collect all faces of the solid.
/// 2. Build an outer copy of the remaining faces (original positions).
/// 3. Build an inner copy of the remaining faces (vertices offset inward by thickness).
/// 4. Connect outer and inner boundary edges (where removed faces were) with side quads.
pub fn shell_solid(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    faces_to_remove: &[Handle<FaceData>],
    thickness: f64,
) -> KernelResult<ShellResult> {
    if thickness.abs() < 1e-14 {
        return Err(KernelError::InvalidArgument(
            "shell thickness must be non-zero".into(),
        ));
    }

    let op = model.history.next_operation("shell_solid");
    let all_faces = collect_solid_faces(model, solid)?;

    let remove_set: HashSet<u32> = faces_to_remove.iter().map(|h| h.index()).collect();

    let remaining_faces: Vec<Handle<FaceData>> = all_faces
        .iter()
        .filter(|h| !remove_set.contains(&h.index()))
        .copied()
        .collect();

    if remaining_faces.is_empty() {
        return Err(KernelError::InvalidArgument(
            "cannot shell: all faces removed".into(),
        ));
    }

    // Compute per-vertex average normal from remaining faces.
    let mut vert_normals: HashMap<u32, Vec3> = HashMap::new();
    let mut vert_face_count: HashMap<u32, usize> = HashMap::new();

    for &fh in &remaining_faces {
        let verts = model.vertices_of_face(fh)?;
        if verts.len() >= 3 {
            // Compute face normal from first 3 vertices.
            let p0 = model.vertices.get(verts[0]).unwrap().point;
            let p1 = model.vertices.get(verts[1]).unwrap().point;
            let p2 = model.vertices.get(verts[2]).unwrap().point;
            let e1 = p1 - p0;
            let e2 = p2 - p0;
            let fn_vec = Vec3::new(
                e1.y * e2.z - e1.z * e2.y,
                e1.z * e2.x - e1.x * e2.z,
                e1.x * e2.y - e1.y * e2.x,
            );
            if let Some(n) = fn_vec.normalized() {
                for &vh in &verts {
                    let key = vh.index();
                    let entry = vert_normals.entry(key).or_insert(Vec3::ZERO);
                    *entry = Vec3::new(entry.x + n.x, entry.y + n.y, entry.z + n.z);
                    *vert_face_count.entry(key).or_insert(0) += 1;
                }
            }
        }
    }

    // Normalize averaged normals.
    for (&key, n) in &mut vert_normals {
        let count = vert_face_count[&key] as f64;
        *n = Vec3::new(n.x / count, n.y / count, n.z / count)
            .normalized()
            .unwrap_or(Vec3::Z);
    }

    // Create outer and inner vertex copies.
    let mut outer_map: HashMap<u32, Handle<VertexData>> = HashMap::new();
    let mut inner_map: HashMap<u32, Handle<VertexData>> = HashMap::new();
    let mut vert_idx = 0u32;

    for &fh in &remaining_faces {
        let verts = model.vertices_of_face(fh)?;
        for &vh in &verts {
            let key = vh.index();
            if outer_map.contains_key(&key) {
                continue;
            }
            let pt = model.vertices.get(vh).unwrap().point;
            let n = vert_normals.get(&key).copied().unwrap_or(Vec3::Z);

            // Outer vertex: same position as original
            let outer_tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
            outer_map.insert(key, model.add_vertex_tagged(pt, outer_tag));
            vert_idx += 1;

            // Inner vertex: offset inward by thickness
            let inner_pt = Point3::new(
                pt.x - thickness * n.x,
                pt.y - thickness * n.y,
                pt.z - thickness * n.z,
            );
            let inner_tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
            inner_map.insert(key, model.add_vertex_tagged(inner_pt, inner_tag));
            vert_idx += 1;
        }
    }

    let mut new_faces = Vec::new();
    let mut edge_idx = 0u32;
    let mut face_idx = 0u32;

    // Outer faces (same winding as original)
    for &fh in &remaining_faces {
        let verts = model.vertices_of_face(fh)?;
        let mapped: Vec<Handle<VertexData>> = verts.iter().map(|vh| outer_map[&vh.index()]).collect();
        let f = make_polygon_face(model, &mapped, op, &mut edge_idx, face_idx, false)?;
        new_faces.push(f);
        face_idx += 1;
    }

    // Inner faces (reversed winding for inward-facing normals)
    for &fh in &remaining_faces {
        let verts = model.vertices_of_face(fh)?;
        let mapped: Vec<Handle<VertexData>> = verts.iter().map(|vh| inner_map[&vh.index()]).collect();
        let f = make_polygon_face(model, &mapped, op, &mut edge_idx, face_idx, true)?;
        new_faces.push(f);
        face_idx += 1;
    }

    // Find boundary edges (edges on removed faces that border remaining faces).
    // For simplicity, connect outer→inner along edges of removed face boundaries.
    // Collect edges from removed faces that have at least one vertex in remaining faces.
    let remaining_vert_keys: HashSet<u32> = outer_map.keys().copied().collect();

    for &fh in faces_to_remove {
        let verts = model.vertices_of_face(fh)?;
        let n = verts.len();
        for i in 0..n {
            let j = (i + 1) % n;
            let ki = verts[i].index();
            let kj = verts[j].index();
            // Only create rim quads for edges where both vertices exist in remaining faces.
            if remaining_vert_keys.contains(&ki) && remaining_vert_keys.contains(&kj) {
                let quad = [
                    outer_map[&ki],
                    outer_map[&kj],
                    inner_map[&kj],
                    inner_map[&ki],
                ];
                let f = make_quad_face(model, &quad, op, &mut edge_idx, face_idx)?;
                new_faces.push(f);
                face_idx += 1;
            }
        }
    }

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&new_faces, shell_tag);
    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let new_solid = model.make_solid_tagged(&[shell], solid_tag);

    Ok(ShellResult {
        solid: new_solid,
        faces: new_faces,
    })
}

fn make_polygon_face(
    model: &mut BRepModel,
    verts: &[Handle<VertexData>],
    op: cadkernel_topology::OperationId,
    edge_idx: &mut u32,
    face_idx: u32,
    reverse: bool,
) -> KernelResult<Handle<FaceData>> {
    let n = verts.len();
    let ordered: Vec<Handle<VertexData>> = if reverse {
        verts.iter().rev().copied().collect()
    } else {
        verts.to_vec()
    };
    let mut hes = Vec::with_capacity(n);
    for i in 0..n {
        let j = (i + 1) % n;
        let etag = Tag::generated(EntityKind::Edge, op, *edge_idx);
        let (_, he, _) = model.add_edge_tagged(ordered[i], ordered[j], etag);
        hes.push(he);
        *edge_idx += 1;
    }
    let lp = model.make_loop(&hes)?;
    let ft = Tag::generated(EntityKind::Face, op, face_idx);
    Ok(model.make_face_tagged(lp, ft))
}

fn make_quad_face(
    model: &mut BRepModel,
    verts: &[Handle<VertexData>; 4],
    op: cadkernel_topology::OperationId,
    edge_idx: &mut u32,
    face_idx: u32,
) -> KernelResult<Handle<FaceData>> {
    let mut hes = Vec::with_capacity(4);
    for i in 0..4 {
        let j = (i + 1) % 4;
        let etag = Tag::generated(EntityKind::Edge, op, *edge_idx);
        let (_, he, _) = model.add_edge_tagged(verts[i], verts[j], etag);
        hes.push(he);
        *edge_idx += 1;
    }
    let lp = model.make_loop(&hes)?;
    let ft = Tag::generated(EntityKind::Face, op, face_idx);
    Ok(model.make_face_tagged(lp, ft))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::make_box;

    #[test]
    fn test_shell_box_removes_one_face() {
        let mut model = cadkernel_topology::BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 10.0, 10.0, 10.0).unwrap();

        // Remove the first face (e.g. top)
        let face_to_remove = r.faces[0];
        let sr = shell_solid(&mut model, r.solid, &[face_to_remove], 1.0).unwrap();

        // Should have: 5 outer + 5 inner + 4 rim quads = 14 faces
        assert_eq!(sr.faces.len(), 14);
        assert_eq!(model.solids.len(), 2); // original + shelled
    }

    #[test]
    fn test_shell_zero_thickness_rejected() {
        let mut model = cadkernel_topology::BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 5.0, 5.0, 5.0).unwrap();
        assert!(shell_solid(&mut model, r.solid, &[r.faces[0]], 0.0).is_err());
    }
}
