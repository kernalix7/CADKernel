//! Draft (taper) operation for solid faces.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, EntityKind, FaceData, Handle, SolidData, Tag, VertexData};
use std::collections::HashMap;

/// Result of a draft operation.
#[derive(Debug)]
pub struct DraftResult {
    pub solid: Handle<SolidData>,
    pub drafted_faces: Vec<Handle<FaceData>>,
}

/// Applies a draft (taper) angle to the specified faces of a solid.
///
/// Vertices are displaced perpendicular to `pull_direction` by an amount
/// proportional to their height along `pull_direction` and `tan(angle)`.
pub fn draft_faces(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    faces: &[Handle<FaceData>],
    pull_direction: Vec3,
    angle: f64,
) -> KernelResult<DraftResult> {
    if angle.abs() < 1e-14 {
        return Err(KernelError::InvalidArgument(
            "draft angle must be non-zero".into(),
        ));
    }
    if angle.abs() >= std::f64::consts::FRAC_PI_2 {
        return Err(KernelError::InvalidArgument(
            "draft angle must be less than 90 degrees".into(),
        ));
    }

    let pull = pull_direction
        .normalized()
        .ok_or(KernelError::InvalidArgument(
            "pull_direction must be non-zero".into(),
        ))?;

    let solid_data = model
        .solids
        .get(solid)
        .ok_or(KernelError::InvalidHandle("solid"))?;
    let shells = solid_data.shells.clone();

    let mut all_faces: Vec<Handle<FaceData>> = Vec::new();
    for &sh in &shells {
        let sd = model
            .shells
            .get(sh)
            .ok_or(KernelError::InvalidHandle("shell"))?;
        all_faces.extend(sd.faces.clone());
    }

    // Collect vertices of draft faces and compute base plane
    let draft_set: std::collections::HashSet<Handle<FaceData>> = faces.iter().copied().collect();

    // Find min height along pull for the neutral plane
    let mut min_height = f64::INFINITY;
    let mut draft_verts: std::collections::HashSet<u32> = std::collections::HashSet::new();

    for &fh in faces {
        let verts = model.vertices_of_face(fh)?;
        for &vh in &verts {
            draft_verts.insert(vh.index());
            let p = vertex_point(model, vh)?;
            let h = Vec3::new(p.x, p.y, p.z).dot(pull);
            if h < min_height {
                min_height = h;
            }
        }
    }

    let tan_angle = angle.tan();
    let op = model.history.next_operation("draft_faces");

    // Precompute face vertex lists
    let mut face_vert_lists: Vec<(Handle<FaceData>, Vec<Handle<VertexData>>)> =
        Vec::with_capacity(all_faces.len());
    for &fh in &all_faces {
        let verts = model.vertices_of_face(fh)?;
        face_vert_lists.push((fh, verts));
    }

    let mut vert_map: HashMap<u32, Handle<VertexData>> = HashMap::new();
    let mut vert_idx = 0u32;

    // Create new vertices (draft-modified or copied)
    for (_, verts) in &face_vert_lists {
        for &vh in verts {
            let key = vh.index();
            if vert_map.contains_key(&key) {
                continue;
            }
            let old_pt = vertex_point(model, vh)?;
            let new_pt = if draft_verts.contains(&key) {
                let h = Vec3::new(old_pt.x, old_pt.y, old_pt.z).dot(pull) - min_height;
                let displacement = h * tan_angle;
                // Displace perpendicular to pull in the face plane
                // Use the vertex's radial direction from the pull axis
                let on_axis = pull * Vec3::new(old_pt.x, old_pt.y, old_pt.z).dot(pull);
                let radial =
                    Vec3::new(old_pt.x, old_pt.y, old_pt.z) - on_axis;
                if let Some(rd) = radial.normalized() {
                    Point3::new(
                        old_pt.x + rd.x * displacement,
                        old_pt.y + rd.y * displacement,
                        old_pt.z + rd.z * displacement,
                    )
                } else {
                    old_pt
                }
            } else {
                old_pt
            };

            let tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
            vert_map.insert(key, model.add_vertex_tagged(new_pt, tag));
            vert_idx += 1;
        }
    }

    let mut new_faces = Vec::new();
    let mut drafted_faces_out = Vec::new();
    let mut edge_idx = 0u32;

    for (fi, &(orig_fh, ref orig_verts)) in face_vert_lists.iter().enumerate() {
        let new_verts: Vec<Handle<VertexData>> =
            orig_verts.iter().map(|vh| vert_map[&vh.index()]).collect();

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
        let fh = model.make_face_tagged(loop_h, face_tag);
        new_faces.push(fh);
        if draft_set.contains(&orig_fh) {
            drafted_faces_out.push(fh);
        }
    }

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&new_faces, shell_tag);
    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let new_solid = model.make_solid_tagged(&[shell], solid_tag);

    Ok(DraftResult {
        solid: new_solid,
        drafted_faces: drafted_faces_out,
    })
}

fn vertex_point(model: &BRepModel, v: Handle<VertexData>) -> KernelResult<Point3> {
    Ok(model
        .vertices
        .get(v)
        .ok_or(KernelError::InvalidHandle("vertex"))?
        .point)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draft_box_side_faces() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 4.0).unwrap();

        // Draft all side faces (not top/bottom) along Z
        // Side faces have vertices with both z=0 and z=4
        let mut side_faces = Vec::new();
        for &fh in &b.faces {
            let verts = model.vertices_of_face(fh).unwrap();
            let zs: Vec<f64> = verts
                .iter()
                .map(|&vh| model.vertices.get(vh).unwrap().point.z)
                .collect();
            let has_low = zs.iter().any(|&z| z < 0.1);
            let has_high = zs.iter().any(|&z| z > 3.9);
            if has_low && has_high {
                side_faces.push(fh);
            }
        }
        assert_eq!(side_faces.len(), 4);

        let result =
            draft_faces(&mut model, b.solid, &side_faces, Vec3::Z, 0.1).unwrap();
        assert!(model.solids.is_alive(result.solid));
        assert_eq!(result.drafted_faces.len(), 4);
    }

    #[test]
    fn test_draft_zero_angle() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let err = draft_faces(&mut model, b.solid, &b.faces, Vec3::Z, 0.0).unwrap_err();
        assert!(matches!(err, KernelError::InvalidArgument(_)));
    }

    #[test]
    fn test_draft_too_large_angle() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let err = draft_faces(
            &mut model,
            b.solid,
            &b.faces,
            Vec3::Z,
            std::f64::consts::FRAC_PI_2,
        )
        .unwrap_err();
        assert!(matches!(err, KernelError::InvalidArgument(_)));
    }
}
