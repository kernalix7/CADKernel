//! Split operation for solids.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, EntityKind, FaceData, Handle, SolidData, Tag, VertexData};
use std::collections::HashMap;

/// Result of a split operation.
#[derive(Debug)]
pub struct SplitResult {
    pub solids: Vec<Handle<SolidData>>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Splits a solid along a plane defined by a point and normal.
///
/// Returns the resulting pieces as new solids. Faces that straddle the
/// plane are split at the intersection, producing new edges and vertices.
pub fn split_solid(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    plane_point: Point3,
    plane_normal: Vec3,
) -> KernelResult<SplitResult> {
    let normal = plane_normal
        .normalized()
        .ok_or(KernelError::InvalidArgument(
            "plane_normal must be non-zero".into(),
        ))?;

    let d = Vec3::new(plane_point.x, plane_point.y, plane_point.z).dot(normal);

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

    let op = model.history.next_operation("split_solid");

    let mut face_vert_lists: Vec<(Handle<FaceData>, Vec<Handle<VertexData>>)> =
        Vec::with_capacity(all_faces.len());
    for &fh in &all_faces {
        let verts = model.vertices_of_face(fh)?;
        face_vert_lists.push((fh, verts));
    }

    // Classify vertices
    let mut vert_side: HashMap<u32, f64> = HashMap::new();
    for (_, verts) in &face_vert_lists {
        for &vh in verts {
            let key = vh.index();
            if vert_side.contains_key(&key) {
                continue;
            }
            let p = vertex_point(model, vh)?;
            let dist = Vec3::new(p.x, p.y, p.z).dot(normal) - d;
            vert_side.insert(key, dist);
        }
    }

    let mut vert_cache_pos: HashMap<VertKey, Handle<VertexData>> = HashMap::new();
    let mut vert_cache_neg: HashMap<VertKey, Handle<VertexData>> = HashMap::new();
    let mut vert_idx = 0u32;

    let get_or_create = |model: &mut BRepModel,
                         cache: &mut HashMap<VertKey, Handle<VertexData>>,
                         idx: &mut u32,
                         point: Point3,
                         op_id: cadkernel_topology::OperationId| {
        let key = VertKey::from_point(point);
        if let Some(&h) = cache.get(&key) {
            return h;
        }
        let tag = Tag::generated(EntityKind::Vertex, op_id, *idx);
        *idx += 1;
        let h = model.add_vertex_tagged(point, tag);
        cache.insert(key, h);
        h
    };

    let mut pos_faces: Vec<Handle<FaceData>> = Vec::new();
    let mut neg_faces: Vec<Handle<FaceData>> = Vec::new();
    let mut all_new_faces: Vec<Handle<FaceData>> = Vec::new();
    let mut face_idx = 0u32;
    let mut edge_idx = 0u32;

    let eps = 1e-10;

    for (_, orig_verts) in &face_vert_lists {
        let n = orig_verts.len();
        let mut pos_poly: Vec<Point3> = Vec::new();
        let mut neg_poly: Vec<Point3> = Vec::new();

        for i in 0..n {
            let vh_a = orig_verts[i];
            let vh_b = orig_verts[(i + 1) % n];
            let pa = vertex_point(model, vh_a)?;
            let pb = vertex_point(model, vh_b)?;
            let da = vert_side[&vh_a.index()];
            let db = vert_side[&vh_b.index()];

            if da >= -eps {
                pos_poly.push(pa);
            }
            if da <= eps {
                neg_poly.push(pa);
            }

            // Edge crosses plane
            if (da > eps && db < -eps) || (da < -eps && db > eps) {
                let t = da / (da - db);
                let ix = Point3::new(
                    pa.x + t * (pb.x - pa.x),
                    pa.y + t * (pb.y - pa.y),
                    pa.z + t * (pb.z - pa.z),
                );
                pos_poly.push(ix);
                neg_poly.push(ix);
            }
        }

        // Build faces for positive side
        if pos_poly.len() >= 3 {
            let new_verts: Vec<Handle<VertexData>> = pos_poly
                .iter()
                .map(|&pt| get_or_create(model, &mut vert_cache_pos, &mut vert_idx, pt, op))
                .collect();
            let fh = build_face_from_verts(model, &new_verts, op, face_idx, &mut edge_idx)?;
            pos_faces.push(fh);
            all_new_faces.push(fh);
            face_idx += 1;
        }

        // Build faces for negative side
        if neg_poly.len() >= 3 {
            let new_verts: Vec<Handle<VertexData>> = neg_poly
                .iter()
                .map(|&pt| get_or_create(model, &mut vert_cache_neg, &mut vert_idx, pt, op))
                .collect();
            let fh = build_face_from_verts(model, &new_verts, op, face_idx, &mut edge_idx)?;
            neg_faces.push(fh);
            all_new_faces.push(fh);
            face_idx += 1;
        }
    }

    // Add cap faces on the split plane
    // Collect intersection points along the split plane from the positive side
    let cap_verts_pos: Vec<Handle<VertexData>> = collect_plane_verts(
        model,
        &pos_faces,
        normal,
        d,
        eps,
    )?;
    if cap_verts_pos.len() >= 3 {
        let fh = build_face_from_verts(model, &cap_verts_pos, op, face_idx, &mut edge_idx)?;
        pos_faces.push(fh);
        all_new_faces.push(fh);
        face_idx += 1;
    }

    let cap_verts_neg: Vec<Handle<VertexData>> = collect_plane_verts(
        model,
        &neg_faces,
        normal,
        d,
        eps,
    )?;
    if cap_verts_neg.len() >= 3 {
        let mut reversed = cap_verts_neg;
        reversed.reverse();
        let fh = build_face_from_verts(model, &reversed, op, face_idx, &mut edge_idx)?;
        neg_faces.push(fh);
        all_new_faces.push(fh);
        face_idx += 1;
    }
    let _ = face_idx;

    let mut solids = Vec::new();

    if !pos_faces.is_empty() {
        let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
        let shell = model.make_shell_tagged(&pos_faces, shell_tag);
        let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
        solids.push(model.make_solid_tagged(&[shell], solid_tag));
    }

    if !neg_faces.is_empty() {
        let shell_tag = Tag::generated(EntityKind::Shell, op, 1);
        let shell = model.make_shell_tagged(&neg_faces, shell_tag);
        let solid_tag = Tag::generated(EntityKind::Solid, op, 1);
        solids.push(model.make_solid_tagged(&[shell], solid_tag));
    }

    Ok(SplitResult {
        solids,
        faces: all_new_faces,
    })
}

fn vertex_point(model: &BRepModel, v: Handle<VertexData>) -> KernelResult<Point3> {
    Ok(model
        .vertices
        .get(v)
        .ok_or(KernelError::InvalidHandle("vertex"))?
        .point)
}

fn collect_plane_verts(
    model: &BRepModel,
    faces: &[Handle<FaceData>],
    normal: Vec3,
    d: f64,
    eps: f64,
) -> KernelResult<Vec<Handle<VertexData>>> {
    let mut on_plane = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for &fh in faces {
        let verts = model.vertices_of_face(fh)?;
        for &vh in &verts {
            if seen.contains(&vh.index()) {
                continue;
            }
            let p = vertex_point(model, vh)?;
            let dist = (Vec3::new(p.x, p.y, p.z).dot(normal) - d).abs();
            if dist < eps * 100.0 {
                on_plane.push(vh);
                seen.insert(vh.index());
            }
        }
    }
    Ok(on_plane)
}

fn build_face_from_verts(
    model: &mut BRepModel,
    verts: &[Handle<VertexData>],
    op: cadkernel_topology::OperationId,
    face_idx: u32,
    edge_base: &mut u32,
) -> KernelResult<Handle<FaceData>> {
    let n = verts.len();
    let mut half_edges = Vec::with_capacity(n);
    for i in 0..n {
        let j = (i + 1) % n;
        let tag = Tag::generated(EntityKind::Edge, op, *edge_base);
        *edge_base += 1;
        let (_, he_a, _) = model.add_edge_tagged(verts[i], verts[j], tag);
        half_edges.push(he_a);
    }
    let loop_h = model.make_loop(&half_edges)?;
    let face_tag = Tag::generated(EntityKind::Face, op, face_idx);
    Ok(model.make_face_tagged(loop_h, face_tag))
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct VertKey {
    x: i64,
    y: i64,
    z: i64,
}

impl VertKey {
    fn from_point(p: Point3) -> Self {
        const SCALE: f64 = 1e9;
        Self {
            x: (p.x * SCALE).round() as i64,
            y: (p.y * SCALE).round() as i64,
            z: (p.z * SCALE).round() as i64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_box_at_midplane() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let result = split_solid(
            &mut model,
            b.solid,
            Point3::new(0.0, 0.0, 2.0),
            Vec3::Z,
        )
        .unwrap();

        assert_eq!(result.solids.len(), 2);
        for &sh in &result.solids {
            assert!(model.solids.is_alive(sh));
        }
    }

    #[test]
    fn test_split_plane_outside_solid() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let result = split_solid(
            &mut model,
            b.solid,
            Point3::new(0.0, 0.0, 10.0),
            Vec3::Z,
        )
        .unwrap();

        // All verts on negative side, positive side should be empty
        assert!(!result.solids.is_empty());
    }

    #[test]
    fn test_split_zero_normal() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let err = split_solid(
            &mut model,
            b.solid,
            Point3::ORIGIN,
            Vec3::ZERO,
        )
        .unwrap_err();
        assert!(matches!(err, KernelError::InvalidArgument(_)));
    }
}
