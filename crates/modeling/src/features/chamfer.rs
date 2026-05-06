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

    let (edge_v1, edge_v2) = resolve_edge_in_faces(model, edge_v1, edge_v2, &face_vert_lists)?;

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

/// Chamfers multiple edges of a solid in a single rebuild pass.
///
/// Unlike calling `chamfer_edge` in a loop — which rebuilds topology after
/// each call and makes subsequent edge handles stale — this batched API
/// computes all offset positions against the *original* topology and rebuilds
/// only once. When a vertex is shared by several requested edges (e.g. a box
/// corner with 3 incident edges chosen for chamfer), the corner is split into
/// one offset point per (face, edge) pair and the chamfer quad is generated
/// for each edge.
pub fn chamfer_edges(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    edges: &[(Handle<VertexData>, Handle<VertexData>)],
    distance: f64,
) -> KernelResult<ChamferResult> {
    if distance <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "chamfer distance must be positive".into(),
        ));
    }
    if edges.is_empty() {
        return Err(KernelError::InvalidArgument(
            "chamfer_edges requires at least one edge".into(),
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

    struct ResolvedEdge {
        v1: Handle<VertexData>,
        v2: Handle<VertexData>,
        adj_faces: [Handle<FaceData>; 2],
    }
    let mut resolved: Vec<ResolvedEdge> = Vec::with_capacity(edges.len());

    for &(v1_in, v2_in) in edges {
        let p1 = vertex_point(model, v1_in)?;
        let p2 = vertex_point(model, v2_in)?;
        let (v1, v2) =
            resolve_edge_by_position(&face_vert_lists, model, p1, p2).ok_or_else(|| {
                KernelError::InvalidArgument(format!(
                    "chamfer_edges: edge ({:?},{:?}) not on any face of the solid",
                    p1, p2
                ))
            })?;

        let edge_len = (p2 - p1).length();
        if distance >= edge_len * 0.5 {
            return Err(KernelError::InvalidArgument(
                "chamfer distance too large for this edge".into(),
            ));
        }

        let mut adj: Vec<Handle<FaceData>> = Vec::new();
        for &(fh, ref verts) in &face_vert_lists {
            if has_consecutive_pair(verts, v1, v2) {
                adj.push(fh);
            }
        }
        if adj.len() != 2 {
            return Err(KernelError::InvalidArgument(format!(
                "chamfer_edges: edge shared by {} faces (need exactly 2)",
                adj.len()
            )));
        }
        resolved.push(ResolvedEdge {
            v1,
            v2,
            adj_faces: [adj[0], adj[1]],
        });
    }

    let op = model.history.next_operation("chamfer_edges");

    // For each (face, vertex) pair where vertex is an endpoint of one or more
    // requested edges on that face, compute the offset point toward the
    // neighbouring non-edge vertex by `distance`. For corners with multiple
    // requested edges incident on the face (e.g. a box corner with two edges
    // on the same bottom face selected), we emit one offset per edge.
    type OffsetKey = (Handle<FaceData>, Handle<VertexData>, usize);
    let mut offsets: HashMap<OffsetKey, Point3> = HashMap::new();

    for (edge_idx, re) in resolved.iter().enumerate() {
        for &adj_fh in &re.adj_faces {
            let verts = face_vert_for(adj_fh, &face_vert_lists)?;
            for &target_v in &[re.v1, re.v2] {
                let pos = find_index(verts, target_v)?;
                let n = verts.len();
                let prev_v = verts[(pos + n - 1) % n];
                let next_v = verts[(pos + 1) % n];
                let other_v = if prev_v == re.v1 || prev_v == re.v2 {
                    next_v
                } else {
                    prev_v
                };

                let p_target = vertex_point(model, target_v)?;
                let p_other = vertex_point(model, other_v)?;
                let offset = compute_offset(p_target, p_other, distance);
                offsets.insert((adj_fh, target_v, edge_idx), offset);
            }
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

    // For each face, walk its vertex loop. At each vertex, find which requested
    // edges are incident on this face at this vertex. Emit offset points in
    // the correct order (predecessor-side first, then successor-side).
    let mut new_faces: Vec<Handle<FaceData>> = Vec::new();
    let mut cham_faces: Vec<Handle<FaceData>> = Vec::new();
    let mut face_idx = 0u32;
    let mut edge_idx_base = 0u32;

    for &(orig_fh, ref orig_verts) in &face_vert_lists {
        let n = orig_verts.len();
        let mut new_loop: Vec<Handle<VertexData>> = Vec::with_capacity(n * 2);

        for i in 0..n {
            let prev_i = (i + n - 1) % n;
            let next_i = (i + 1) % n;
            let v = orig_verts[i];
            let vp = orig_verts[prev_i];
            let vn = orig_verts[next_i];

            // Find edges requested at this (face,vertex) that lie on the
            // incoming side (between vp and v) and outgoing side (between v and vn).
            let mut incoming: Option<usize> = None;
            let mut outgoing: Option<usize> = None;
            for (edge_idx, re) in resolved.iter().enumerate() {
                let on_face = re.adj_faces[0] == orig_fh || re.adj_faces[1] == orig_fh;
                if !on_face {
                    continue;
                }
                let endpoints = [re.v1, re.v2];
                if endpoints.contains(&v) && endpoints.contains(&vp) {
                    incoming = Some(edge_idx);
                }
                if endpoints.contains(&v) && endpoints.contains(&vn) {
                    outgoing = Some(edge_idx);
                }
            }

            match (incoming, outgoing) {
                (None, None) => {
                    let pt = vertex_point(model, v)?;
                    let nvh = get_or_create(model, &mut new_vert_cache, &mut vert_idx, pt, op);
                    new_loop.push(nvh);
                }
                (Some(in_idx), None) => {
                    let off = offsets[&(orig_fh, v, in_idx)];
                    let nvh = get_or_create(model, &mut new_vert_cache, &mut vert_idx, off, op);
                    new_loop.push(nvh);
                }
                (None, Some(out_idx)) => {
                    let off = offsets[&(orig_fh, v, out_idx)];
                    let nvh = get_or_create(model, &mut new_vert_cache, &mut vert_idx, off, op);
                    new_loop.push(nvh);
                }
                (Some(in_idx), Some(out_idx)) => {
                    // Corner shared by two filleted edges on this face — split
                    // the corner into two offsets (predecessor-side, then
                    // successor-side).
                    let off_in = offsets[&(orig_fh, v, in_idx)];
                    let off_out = offsets[&(orig_fh, v, out_idx)];
                    let nvh_in =
                        get_or_create(model, &mut new_vert_cache, &mut vert_idx, off_in, op);
                    new_loop.push(nvh_in);
                    if in_idx != out_idx {
                        let nvh_out =
                            get_or_create(model, &mut new_vert_cache, &mut vert_idx, off_out, op);
                        new_loop.push(nvh_out);
                    }
                }
            }
        }

        // Dedup consecutive identical vertices introduced above.
        new_loop.dedup();
        if new_loop.len() >= 2 && new_loop.first() == new_loop.last() {
            new_loop.pop();
        }
        if new_loop.len() < 3 {
            continue;
        }

        let fh = build_face(model, &new_loop, op, face_idx, &mut edge_idx_base)?;
        new_faces.push(fh);
        face_idx += 1;
    }

    // One chamfer quad per requested edge.
    for (edge_idx, re) in resolved.iter().enumerate() {
        let f0 = re.adj_faces[0];
        let f1 = re.adj_faces[1];
        let quad = [
            offsets[&(f0, re.v1, edge_idx)],
            offsets[&(f1, re.v1, edge_idx)],
            offsets[&(f1, re.v2, edge_idx)],
            offsets[&(f0, re.v2, edge_idx)],
        ];
        let mut quad_vs: Vec<Handle<VertexData>> = Vec::with_capacity(4);
        for &pt in &quad {
            quad_vs.push(get_or_create(
                model,
                &mut new_vert_cache,
                &mut vert_idx,
                pt,
                op,
            ));
        }
        quad_vs.dedup();
        if quad_vs.len() < 3 {
            continue;
        }
        let fh = build_face(model, &quad_vs, op, face_idx, &mut edge_idx_base)?;
        new_faces.push(fh);
        cham_faces.push(fh);
        face_idx += 1;
    }

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&new_faces, shell_tag);
    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let new_solid = model.make_solid_tagged(&[shell], solid_tag);

    let chamfer_face = cham_faces.first().copied().unwrap_or_else(|| new_faces[0]);
    Ok(ChamferResult {
        solid: new_solid,
        chamfer_face,
        faces: new_faces,
    })
}

/// Resolves an (approximate) 3D edge to current-face vertex handles by
/// position. Returns None if no face has both endpoints.
fn resolve_edge_by_position(
    face_vert_lists: &[(Handle<FaceData>, Vec<Handle<VertexData>>)],
    model: &BRepModel,
    p1: Point3,
    p2: Point3,
) -> Option<(Handle<VertexData>, Handle<VertexData>)> {
    let find_by_pos =
        |verts: &[Handle<VertexData>], target: Point3| -> Option<Handle<VertexData>> {
            verts.iter().copied().find(|&vh| {
                if let Some(vd) = model.vertices.get(vh) {
                    (vd.point - target).length() < 1e-9
                } else {
                    false
                }
            })
        };
    for (_, verts) in face_vert_lists {
        let r1 = find_by_pos(verts, p1);
        let r2 = find_by_pos(verts, p2);
        if let (Some(h1), Some(h2)) = (r1, r2) {
            if has_consecutive_pair(verts, h1, h2) {
                return Some((h1, h2));
            }
        }
    }
    None
}

fn resolve_edge_in_faces(
    model: &BRepModel,
    v1: Handle<VertexData>,
    v2: Handle<VertexData>,
    face_vert_lists: &[(Handle<FaceData>, Vec<Handle<VertexData>>)],
) -> KernelResult<(Handle<VertexData>, Handle<VertexData>)> {
    if face_vert_lists
        .iter()
        .any(|(_, verts)| has_consecutive_pair(verts, v1, v2))
    {
        return Ok((v1, v2));
    }

    let p1 = vertex_point(model, v1)?;
    let p2 = vertex_point(model, v2)?;

    let find_by_pos =
        |verts: &[Handle<VertexData>], target: Point3| -> Option<Handle<VertexData>> {
            verts.iter().copied().find(|&vh| {
                if let Some(vd) = model.vertices.get(vh) {
                    (vd.point - target).length() < 1e-9
                } else {
                    false
                }
            })
        };

    for (_, verts) in face_vert_lists {
        let r1 = find_by_pos(verts, p1);
        let r2 = find_by_pos(verts, p2);
        if let (Some(h1), Some(h2)) = (r1, r2) {
            if has_consecutive_pair(verts, h1, h2) {
                return Ok((h1, h2));
            }
        }
    }

    Err(KernelError::InvalidArgument(format!(
        "chamfer edge endpoints ({:?}, {:?}) not found on any face after position-based lookup",
        p1, p2
    )))
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
