use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::Point3;
use cadkernel_topology::{BRepModel, EntityKind, FaceData, Handle, SolidData, Tag, VertexData};
use std::collections::HashMap;

#[derive(Debug)]
pub struct ChamferResult {
    pub solid: Handle<SolidData>,
    pub chamfer_face: Handle<FaceData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Chamfers (bevels) a single edge of a solid by a uniform distance.
///
/// `edge_v1` and `edge_v2` identify the edge by its two endpoint vertex
/// handles.  A **new** solid is created; the original is not modified.
pub fn chamfer_edge(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    edge_v1: Handle<VertexData>,
    edge_v2: Handle<VertexData>,
    distance: f64,
) -> KernelResult<ChamferResult> {
    if distance <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "chamfer distance must be positive".into(),
        ));
    }

    let p1 = vertex_point(model, edge_v1)?;
    let p2 = vertex_point(model, edge_v2)?;
    let edge_len = (p2 - p1).length();
    if distance >= edge_len * 0.5 {
        return Err(KernelError::InvalidArgument(
            "chamfer distance too large for this edge".into(),
        ));
    }

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

    let mut face_vert_lists: Vec<(Handle<FaceData>, Vec<Handle<VertexData>>)> =
        Vec::with_capacity(all_faces.len());
    for &fh in &all_faces {
        let verts = model.vertices_of_face(fh)?;
        face_vert_lists.push((fh, verts));
    }

    let mut adj_faces: Vec<Handle<FaceData>> = Vec::new();
    for &(fh, ref verts) in &face_vert_lists {
        if has_consecutive_pair(verts, edge_v1, edge_v2) {
            adj_faces.push(fh);
        }
    }
    if adj_faces.len() != 2 {
        return Err(KernelError::InvalidArgument(format!(
            "chamfer requires an edge shared by exactly 2 faces, found {}",
            adj_faces.len()
        )));
    }

    let op = model.history.next_operation("chamfer_edge");

    let mut offset_points: HashMap<(Handle<FaceData>, Handle<VertexData>), Point3> = HashMap::new();

    for &adj_fh in &adj_faces {
        let verts = face_vert_for(adj_fh, &face_vert_lists)?;

        for &target_v in &[edge_v1, edge_v2] {
            let pos = find_index(verts, target_v)?;
            let n = verts.len();
            let prev_v = verts[(pos + n - 1) % n];
            let next_v = verts[(pos + 1) % n];

            let other_v = if prev_v != edge_v1 && prev_v != edge_v2 {
                prev_v
            } else {
                next_v
            };

            let p_target = vertex_point(model, target_v)?;
            let p_other = vertex_point(model, other_v)?;
            let offset = compute_offset(p_target, p_other, distance);
            offset_points.insert((adj_fh, target_v), offset);
        }
    }

    let mut new_vert_cache: HashMap<VertKey, Handle<VertexData>> = HashMap::new();
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

    let mut new_faces: Vec<Handle<FaceData>> = Vec::new();
    let mut face_idx = 0u32;
    let mut edge_idx_base = 0u32;

    for &(orig_fh, ref orig_verts) in &face_vert_lists {
        let is_adj = adj_faces.contains(&orig_fh);

        let mut new_verts_for_face: Vec<Handle<VertexData>> = Vec::new();

        for &vh in orig_verts {
            if is_adj && (vh == edge_v1 || vh == edge_v2) {
                if let Some(&off_pt) = offset_points.get(&(orig_fh, vh)) {
                    let nvh = get_or_create(model, &mut new_vert_cache, &mut vert_idx, off_pt, op);
                    new_verts_for_face.push(nvh);
                } else {
                    let pt = vertex_point(model, vh)?;
                    let nvh = get_or_create(model, &mut new_vert_cache, &mut vert_idx, pt, op);
                    new_verts_for_face.push(nvh);
                }
            } else {
                let pt = vertex_point(model, vh)?;
                let nvh = get_or_create(model, &mut new_vert_cache, &mut vert_idx, pt, op);
                new_verts_for_face.push(nvh);
            }
        }

        let fh = build_face(model, &new_verts_for_face, op, face_idx, &mut edge_idx_base)?;
        new_faces.push(fh);
        face_idx += 1;
    }

    let f1 = adj_faces[0];
    let f2 = adj_faces[1];
    let chamfer_quad = [
        offset_points[&(f1, edge_v1)],
        offset_points[&(f2, edge_v1)],
        offset_points[&(f2, edge_v2)],
        offset_points[&(f1, edge_v2)],
    ];

    let mut cham_verts: Vec<Handle<VertexData>> = Vec::with_capacity(4);
    for &pt in &chamfer_quad {
        let nvh = get_or_create(model, &mut new_vert_cache, &mut vert_idx, pt, op);
        cham_verts.push(nvh);
    }

    let cham_fh = build_face(model, &cham_verts, op, face_idx, &mut edge_idx_base)?;
    new_faces.push(cham_fh);

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&new_faces, shell_tag);
    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let new_solid = model.make_solid_tagged(&[shell], solid_tag);

    Ok(ChamferResult {
        solid: new_solid,
        chamfer_face: cham_fh,
        faces: new_faces,
    })
}

fn has_consecutive_pair(
    verts: &[Handle<VertexData>],
    a: Handle<VertexData>,
    b: Handle<VertexData>,
) -> bool {
    let n = verts.len();
    for i in 0..n {
        let j = (i + 1) % n;
        if (verts[i] == a && verts[j] == b) || (verts[i] == b && verts[j] == a) {
            return true;
        }
    }
    false
}

fn vertex_point(model: &BRepModel, v: Handle<VertexData>) -> KernelResult<Point3> {
    Ok(model
        .vertices
        .get(v)
        .ok_or(KernelError::InvalidHandle("vertex"))?
        .point)
}

fn face_vert_for(
    fh: Handle<FaceData>,
    list: &[(Handle<FaceData>, Vec<Handle<VertexData>>)],
) -> KernelResult<&[Handle<VertexData>]> {
    list.iter()
        .find(|(f, _)| *f == fh)
        .map(|(_, v)| v.as_slice())
        .ok_or(KernelError::InvalidHandle("face not found in list"))
}

fn find_index(verts: &[Handle<VertexData>], target: Handle<VertexData>) -> KernelResult<usize> {
    verts
        .iter()
        .position(|&v| v == target)
        .ok_or(KernelError::TopologyError(
            "vertex not found in face loop".into(),
        ))
}

fn compute_offset(from: Point3, toward: Point3, dist: f64) -> Point3 {
    let dir = toward - from;
    let len = dir.length();
    if len < 1e-14 {
        return from;
    }
    from + dir * (dist / len)
}

fn build_face(
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
    fn test_chamfer_box_edge() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let v0 = b.vertices[0]; // (0,0,0)
        let v1 = b.vertices[1]; // (4,0,0)

        let result = chamfer_edge(&mut model, b.solid, v0, v1, 0.5).unwrap();
        assert_eq!(result.faces.len(), 7);
        assert!(model.solids.is_alive(result.solid));
        assert!(model.faces.is_alive(result.chamfer_face));
    }

    #[test]
    fn test_chamfer_distance_too_large() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let v0 = b.vertices[0];
        let v1 = b.vertices[1];

        let err = chamfer_edge(&mut model, b.solid, v0, v1, 5.0).unwrap_err();
        assert!(matches!(err, KernelError::InvalidArgument(_)));
    }

    #[test]
    fn test_chamfer_negative_distance() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let v0 = b.vertices[0];
        let v1 = b.vertices[1];

        let err = chamfer_edge(&mut model, b.solid, v0, v1, -1.0).unwrap_err();
        assert!(matches!(err, KernelError::InvalidArgument(_)));
    }

    #[test]
    fn test_chamfer_tags_present() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let v0 = b.vertices[0];
        let v1 = b.vertices[1];

        let result = chamfer_edge(&mut model, b.solid, v0, v1, 0.5).unwrap();

        let records = model.history.records();
        let chamfer_op = records.last().unwrap().operation;
        let face_tag = Tag::generated(EntityKind::Face, chamfer_op, 0);
        assert!(model.find_face_by_tag(&face_tag).is_some());
        assert!(model.solids.is_alive(result.solid));
    }
}
