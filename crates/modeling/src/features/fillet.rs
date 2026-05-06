//! Fillet (rounding) operation for solid edges.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::Point3;
use cadkernel_topology::{BRepModel, EntityKind, FaceData, Handle, SolidData, Tag, VertexData};
use std::collections::HashMap;

/// Result of a fillet operation.
#[derive(Debug)]
pub struct FilletResult {
    pub solid: Handle<SolidData>,
    pub fillet_faces: Vec<Handle<FaceData>>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Fillets (rounds) a single edge of a solid by a uniform radius.
///
/// The fillet is approximated by 4 planar strips.
pub fn fillet_edge(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    edge_v1: Handle<VertexData>,
    edge_v2: Handle<VertexData>,
    radius: f64,
) -> KernelResult<FilletResult> {
    fillet_edge_segments(model, solid, edge_v1, edge_v2, radius, 4)
}

/// Fillets an edge with a configurable number of arc segments.
pub fn fillet_edge_segments(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    edge_v1: Handle<VertexData>,
    edge_v2: Handle<VertexData>,
    radius: f64,
    segments: usize,
) -> KernelResult<FilletResult> {
    if radius <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "fillet radius must be positive".into(),
        ));
    }
    if segments < 1 {
        return Err(KernelError::InvalidArgument("segments must be >= 1".into()));
    }

    let p1 = vertex_point(model, edge_v1)?;
    let p2 = vertex_point(model, edge_v2)?;
    let edge_len = (p2 - p1).length();
    if radius >= edge_len * 0.5 {
        return Err(KernelError::InvalidArgument(
            "fillet radius too large for this edge".into(),
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

    // Re-resolve the input vertices by position within the active solid.
    // Sequential feature ops rebuild topology (new vertex handles), so the
    // caller may pass stale handles whose coordinates still exist somewhere
    // in the model. Resolving by position makes operations composable.
    let (edge_v1, edge_v2) = resolve_edge_in_faces(model, edge_v1, edge_v2, &face_vert_lists)?;

    let mut adj_faces: Vec<Handle<FaceData>> = Vec::new();
    for &(fh, ref verts) in &face_vert_lists {
        if has_consecutive_pair(verts, edge_v1, edge_v2) {
            adj_faces.push(fh);
        }
    }
    if adj_faces.len() != 2 {
        return Err(KernelError::InvalidArgument(format!(
            "fillet requires an edge shared by exactly 2 faces, found {}",
            adj_faces.len()
        )));
    }

    let op = model.history.next_operation("fillet_edge");

    let inward_a = compute_inward(model, adj_faces[0], edge_v1, edge_v2, &face_vert_lists)?;
    let inward_b = compute_inward(model, adj_faces[1], edge_v1, edge_v2, &face_vert_lists)?;

    // Compute arc offset strips for each edge endpoint
    let mut offset_strips: Vec<HashMap<Handle<VertexData>, Point3>> =
        (0..=segments).map(|_| HashMap::new()).collect();

    for &target_v in &[edge_v1, edge_v2] {
        let p_target = vertex_point(model, target_v)?;
        let da = inward_a.normalized().unwrap_or(cadkernel_math::Vec3::X);
        let db = inward_b.normalized().unwrap_or(cadkernel_math::Vec3::Y);

        for (s, strip) in offset_strips.iter_mut().enumerate() {
            let t = s as f64 / segments as f64;
            let angle = t * std::f64::consts::FRAC_PI_2;
            let pt = p_target + da * (radius * angle.cos()) + db * (radius * angle.sin());
            strip.insert(target_v, pt);
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

    // Rebuild original faces with offset vertices
    for &(orig_fh, ref orig_verts) in &face_vert_lists {
        let strip_idx = if orig_fh == adj_faces[0] {
            Some(0)
        } else if orig_fh == adj_faces[1] {
            Some(segments)
        } else {
            None
        };

        let mut new_verts_for_face: Vec<Handle<VertexData>> = Vec::new();

        for &vh in orig_verts {
            if let Some(si) = strip_idx {
                if vh == edge_v1 || vh == edge_v2 {
                    if let Some(&off_pt) = offset_strips[si].get(&vh) {
                        let nvh =
                            get_or_create(model, &mut new_vert_cache, &mut vert_idx, off_pt, op);
                        new_verts_for_face.push(nvh);
                        continue;
                    }
                }
            }
            let pt = vertex_point(model, vh)?;
            let nvh = get_or_create(model, &mut new_vert_cache, &mut vert_idx, pt, op);
            new_verts_for_face.push(nvh);
        }

        let fh = build_face(model, &new_verts_for_face, op, face_idx, &mut edge_idx_base)?;
        new_faces.push(fh);
        face_idx += 1;
    }

    // Build fillet strip faces
    let mut fillet_faces = Vec::new();
    for s in 0..segments {
        let p1a = offset_strips[s][&edge_v1];
        let p2a = offset_strips[s + 1][&edge_v1];
        let p2b = offset_strips[s + 1][&edge_v2];
        let p1b = offset_strips[s][&edge_v2];

        let v1a = get_or_create(model, &mut new_vert_cache, &mut vert_idx, p1a, op);
        let v2a = get_or_create(model, &mut new_vert_cache, &mut vert_idx, p2a, op);
        let v2b = get_or_create(model, &mut new_vert_cache, &mut vert_idx, p2b, op);
        let v1b = get_or_create(model, &mut new_vert_cache, &mut vert_idx, p1b, op);

        let quad = [v1a, v2a, v2b, v1b];
        let fh = build_face(model, &quad, op, face_idx, &mut edge_idx_base)?;
        fillet_faces.push(fh);
        new_faces.push(fh);
        face_idx += 1;
    }

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&new_faces, shell_tag);
    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let new_solid = model.make_solid_tagged(&[shell], solid_tag);

    Ok(FilletResult {
        solid: new_solid,
        fillet_faces,
        faces: new_faces,
    })
}

/// Fillets multiple edges of a solid in a single rebuild pass.
///
/// Unlike calling `fillet_edge` in a loop — which rebuilds topology after
/// each call and makes subsequent edge handles stale — this batched API
/// computes all offsets against the *original* topology and rebuilds once.
/// Corners shared by multiple requested edges are split into one offset per
/// incident filleted edge on each adjacent face.
pub fn fillet_edges(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    edges: &[(Handle<VertexData>, Handle<VertexData>)],
    radius: f64,
) -> KernelResult<FilletResult> {
    fillet_edges_segments(model, solid, edges, radius, 4)
}

/// Like `fillet_edges` but with a configurable number of arc segments per edge.
pub fn fillet_edges_segments(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    edges: &[(Handle<VertexData>, Handle<VertexData>)],
    radius: f64,
    segments: usize,
) -> KernelResult<FilletResult> {
    if radius <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "fillet radius must be positive".into(),
        ));
    }
    if segments < 1 {
        return Err(KernelError::InvalidArgument("segments must be >= 1".into()));
    }
    if edges.is_empty() {
        return Err(KernelError::InvalidArgument(
            "fillet_edges requires at least one edge".into(),
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

    struct ResolvedFilletEdge {
        v1: Handle<VertexData>,
        v2: Handle<VertexData>,
        adj_faces: [Handle<FaceData>; 2],
    }
    let mut resolved: Vec<ResolvedFilletEdge> = Vec::with_capacity(edges.len());

    for &(v1_in, v2_in) in edges {
        let p1 = vertex_point(model, v1_in)?;
        let p2 = vertex_point(model, v2_in)?;
        let (v1, v2) =
            resolve_edge_by_position(&face_vert_lists, model, p1, p2).ok_or_else(|| {
                KernelError::InvalidArgument(format!(
                    "fillet_edges: edge ({:?},{:?}) not on any face of the solid",
                    p1, p2
                ))
            })?;
        let edge_len = (p2 - p1).length();
        if radius >= edge_len * 0.5 {
            return Err(KernelError::InvalidArgument(
                "fillet radius too large for this edge".into(),
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
                "fillet_edges: edge shared by {} faces (need exactly 2)",
                adj.len()
            )));
        }
        resolved.push(ResolvedFilletEdge {
            v1,
            v2,
            adj_faces: [adj[0], adj[1]],
        });
    }

    let op = model.history.next_operation("fillet_edges");

    // For each edge, compute inward vectors on each adjacent face, and arc
    // strip offsets at each endpoint. Strip layer 0 lies on adj_faces[0]
    // (offset into that face away from the edge), layer `segments` lies on
    // adj_faces[1].
    struct EdgeOffsets {
        // [face_idx(0 or 1)][endpoint(0=v1,1=v2)] -> offset point
        // face_idx 0 corresponds to strip layer 0 (on adj_faces[0])
        // face_idx 1 corresponds to strip layer `segments` (on adj_faces[1])
        layer_points: Vec<[Point3; 2]>, // indexed by strip layer 0..=segments
    }

    let mut all_edge_offsets: Vec<EdgeOffsets> = Vec::with_capacity(resolved.len());
    for re in &resolved {
        let inward_a = compute_inward(model, re.adj_faces[0], re.v1, re.v2, &face_vert_lists)?;
        let inward_b = compute_inward(model, re.adj_faces[1], re.v1, re.v2, &face_vert_lists)?;
        let da = inward_a.normalized().unwrap_or(cadkernel_math::Vec3::X);
        let db = inward_b.normalized().unwrap_or(cadkernel_math::Vec3::Y);

        let p1 = vertex_point(model, re.v1)?;
        let p2 = vertex_point(model, re.v2)?;

        let mut layers: Vec<[Point3; 2]> = Vec::with_capacity(segments + 1);
        for s in 0..=segments {
            let t = s as f64 / segments as f64;
            let angle = t * std::f64::consts::FRAC_PI_2;
            let off1 = p1 + da * (radius * angle.cos()) + db * (radius * angle.sin());
            let off2 = p2 + da * (radius * angle.cos()) + db * (radius * angle.sin());
            layers.push([off1, off2]);
        }
        all_edge_offsets.push(EdgeOffsets {
            layer_points: layers,
        });
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

    // Face rebuild: walk each original face's vertex loop and substitute
    // corner offsets where needed.
    let mut new_faces: Vec<Handle<FaceData>> = Vec::new();
    let mut fillet_faces: Vec<Handle<FaceData>> = Vec::new();
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

            let offset_for_edge = |edge_idx: usize, vertex: Handle<VertexData>| -> Point3 {
                let re = &resolved[edge_idx];
                let eo = &all_edge_offsets[edge_idx];
                let layer = if re.adj_faces[0] == orig_fh {
                    0
                } else {
                    segments
                };
                let endpoint = if vertex == re.v1 { 0 } else { 1 };
                eo.layer_points[layer][endpoint]
            };

            match (incoming, outgoing) {
                (None, None) => {
                    let pt = vertex_point(model, v)?;
                    let nvh = get_or_create(model, &mut new_vert_cache, &mut vert_idx, pt, op);
                    new_loop.push(nvh);
                }
                (Some(e), None) => {
                    let off = offset_for_edge(e, v);
                    let nvh = get_or_create(model, &mut new_vert_cache, &mut vert_idx, off, op);
                    new_loop.push(nvh);
                }
                (None, Some(e)) => {
                    let off = offset_for_edge(e, v);
                    let nvh = get_or_create(model, &mut new_vert_cache, &mut vert_idx, off, op);
                    new_loop.push(nvh);
                }
                (Some(e_in), Some(e_out)) => {
                    let off_in = offset_for_edge(e_in, v);
                    let off_out = offset_for_edge(e_out, v);
                    let nvh_in =
                        get_or_create(model, &mut new_vert_cache, &mut vert_idx, off_in, op);
                    new_loop.push(nvh_in);
                    if e_in != e_out {
                        let nvh_out =
                            get_or_create(model, &mut new_vert_cache, &mut vert_idx, off_out, op);
                        new_loop.push(nvh_out);
                    }
                }
            }
        }

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

    // For each requested edge, emit `segments` strip faces between layers s
    // and s+1.
    for eo in &all_edge_offsets {
        for s in 0..segments {
            let p1a = eo.layer_points[s][0];
            let p2a = eo.layer_points[s + 1][0];
            let p2b = eo.layer_points[s + 1][1];
            let p1b = eo.layer_points[s][1];

            let v1a = get_or_create(model, &mut new_vert_cache, &mut vert_idx, p1a, op);
            let v2a = get_or_create(model, &mut new_vert_cache, &mut vert_idx, p2a, op);
            let v2b = get_or_create(model, &mut new_vert_cache, &mut vert_idx, p2b, op);
            let v1b = get_or_create(model, &mut new_vert_cache, &mut vert_idx, p1b, op);

            let mut quad = vec![v1a, v2a, v2b, v1b];
            quad.dedup();
            if quad.len() < 3 {
                continue;
            }

            let fh = build_face(model, &quad, op, face_idx, &mut edge_idx_base)?;
            new_faces.push(fh);
            fillet_faces.push(fh);
            face_idx += 1;
        }
    }

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&new_faces, shell_tag);
    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let new_solid = model.make_solid_tagged(&[shell], solid_tag);

    Ok(FilletResult {
        solid: new_solid,
        fillet_faces,
        faces: new_faces,
    })
}

/// Resolves an edge to current-face vertex handles by position lookup.
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

/// Resolves two vertex handles to handles that actually appear consecutively
/// on some face of the active solid. If the exact handles are already valid,
/// returns them unchanged. Otherwise searches by position (1e-9 tolerance).
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
        "fillet edge endpoints ({:?}, {:?}) not found on any face after position-based lookup",
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

fn compute_inward(
    model: &BRepModel,
    fh: Handle<FaceData>,
    ev1: Handle<VertexData>,
    ev2: Handle<VertexData>,
    list: &[(Handle<FaceData>, Vec<Handle<VertexData>>)],
) -> KernelResult<cadkernel_math::Vec3> {
    let verts = list
        .iter()
        .find(|(f, _)| *f == fh)
        .map(|(_, v)| v.as_slice())
        .ok_or(KernelError::InvalidHandle("face not found"))?;

    let n = face_normal(model, verts)?;
    let pe1 = vertex_point(model, ev1)?;
    let pe2 = vertex_point(model, ev2)?;
    let edge_dir = (pe2 - pe1).normalized().unwrap_or(cadkernel_math::Vec3::X);

    let inward = n.cross(edge_dir);

    let centroid = face_centroid(model, verts)?;
    let mid_edge = Point3::new(
        (pe1.x + pe2.x) * 0.5,
        (pe1.y + pe2.y) * 0.5,
        (pe1.z + pe2.z) * 0.5,
    );
    let to_center = centroid - mid_edge;
    if to_center.dot(inward) < 0.0 {
        Ok((inward * (-1.0))
            .normalized()
            .unwrap_or(cadkernel_math::Vec3::Y))
    } else {
        Ok(inward.normalized().unwrap_or(cadkernel_math::Vec3::Y))
    }
}

fn face_normal(
    model: &BRepModel,
    verts: &[Handle<VertexData>],
) -> KernelResult<cadkernel_math::Vec3> {
    if verts.len() < 3 {
        return Ok(cadkernel_math::Vec3::Z);
    }
    let p0 = vertex_point(model, verts[0])?;
    let p1 = vertex_point(model, verts[1])?;
    let p2 = vertex_point(model, verts[2])?;
    Ok((p1 - p0)
        .cross(p2 - p0)
        .normalized()
        .unwrap_or(cadkernel_math::Vec3::Z))
}

fn face_centroid(model: &BRepModel, verts: &[Handle<VertexData>]) -> KernelResult<Point3> {
    let mut sum = cadkernel_math::Vec3::ZERO;
    for &vh in verts {
        let p = vertex_point(model, vh)?;
        sum += cadkernel_math::Vec3::new(p.x, p.y, p.z);
    }
    let n = verts.len() as f64;
    Ok(Point3::new(sum.x / n, sum.y / n, sum.z / n))
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
    fn test_fillet_box_edge() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();
        let v0 = b.vertices[0];
        let v1 = b.vertices[1];

        let result = fillet_edge(&mut model, b.solid, v0, v1, 0.5).unwrap();
        assert_eq!(result.faces.len(), 10); // 6 original + 4 fillet strips
        assert_eq!(result.fillet_faces.len(), 4);
        assert!(model.solids.is_alive(result.solid));
    }

    #[test]
    fn test_fillet_radius_too_large() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();
        let v0 = b.vertices[0];
        let v1 = b.vertices[1];

        let err = fillet_edge(&mut model, b.solid, v0, v1, 5.0).unwrap_err();
        assert!(matches!(err, KernelError::InvalidArgument(_)));
    }

    #[test]
    fn test_fillet_negative_radius() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();
        let v0 = b.vertices[0];
        let v1 = b.vertices[1];

        let err = fillet_edge(&mut model, b.solid, v0, v1, -1.0).unwrap_err();
        assert!(matches!(err, KernelError::InvalidArgument(_)));
    }

    #[test]
    fn test_fillet_custom_segments() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();
        let v0 = b.vertices[0];
        let v1 = b.vertices[1];

        let result = fillet_edge_segments(&mut model, b.solid, v0, v1, 0.5, 8).unwrap();
        assert_eq!(result.faces.len(), 14); // 6 + 8
        assert_eq!(result.fillet_faces.len(), 8);
    }
}
