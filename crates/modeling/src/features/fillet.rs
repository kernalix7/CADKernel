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
        return Err(KernelError::InvalidArgument(
            "segments must be >= 1".into(),
        ));
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

fn face_normal(model: &BRepModel, verts: &[Handle<VertexData>]) -> KernelResult<cadkernel_math::Vec3> {
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
