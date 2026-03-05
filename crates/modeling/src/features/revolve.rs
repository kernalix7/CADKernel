use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, EntityKind, FaceData, Handle, SolidData, Tag, VertexData};

/// Handles returned from [`revolve`].
pub struct RevolveResult {
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Revolves a planar profile around an axis to produce a B-Rep solid.
///
/// `profile`     – ordered points forming an open polyline (not closed).
/// `axis_origin` – a point on the rotation axis.
/// `axis_dir`    – direction of the rotation axis (will be normalised).
/// `angle`       – sweep angle in radians (use `2π` for a full revolution).
/// `segments`    – number of angular subdivisions.
pub fn revolve(
    model: &mut BRepModel,
    profile: &[Point3],
    axis_origin: Point3,
    axis_dir: Vec3,
    angle: f64,
    segments: usize,
) -> KernelResult<RevolveResult> {
    let n_pts = profile.len();
    if n_pts < 2 {
        return Err(KernelError::InvalidArgument(
            "revolve requires at least 2 profile points".into(),
        ));
    }
    if segments < 3 {
        return Err(KernelError::InvalidArgument(
            "revolve requires at least 3 segments".into(),
        ));
    }

    let op = model.history.next_operation("revolve");
    let axis = axis_dir.normalized().unwrap_or(Vec3::Y);
    let full_revolution = (angle - std::f64::consts::TAU).abs() < 1e-10;
    let n_rings = if full_revolution {
        segments
    } else {
        segments + 1
    };

    // --- Build vertex rings ---
    // rings[ring_idx][profile_idx]
    let mut rings: Vec<Vec<Handle<VertexData>>> = Vec::with_capacity(n_rings);

    for ring in 0..n_rings {
        let theta = angle * ring as f64 / segments as f64;
        let mut ring_verts = Vec::with_capacity(n_pts);
        for (pi, &pt) in profile.iter().enumerate() {
            let rotated = rotate_point_around_axis(pt, axis_origin, axis, theta);
            let vidx = (ring * n_pts + pi) as u32;
            let tag = Tag::generated(EntityKind::Vertex, op, vidx);
            ring_verts.push(model.add_vertex_tagged(rotated, tag));
        }
        rings.push(ring_verts);
    }

    // --- Build side faces ---
    let mut faces = Vec::new();
    let mut face_idx = 0u32;

    for seg in 0..segments {
        let r0 = seg;
        let r1 = if full_revolution {
            (seg + 1) % n_rings
        } else {
            seg + 1
        };

        for pi in 0..(n_pts - 1) {
            let quad = [
                rings[r0][pi],
                rings[r0][pi + 1],
                rings[r1][pi + 1],
                rings[r1][pi],
            ];
            let f = make_quad_face(model, &quad, op, face_idx)?;
            faces.push(f);
            face_idx += 1;
        }
    }

    // --- End caps (partial revolution only) ---
    if !full_revolution {
        let start_cap = make_ngon_face(model, &rings[0], op, face_idx, true)?;
        faces.push(start_cap);
        face_idx += 1;

        let end_cap = make_ngon_face(model, &rings[n_rings - 1], op, face_idx, false)?;
        faces.push(end_cap);
        face_idx += 1;
    }

    // --- Assemble ---
    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&faces, shell_tag);

    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let solid = model.make_solid_tagged(&[shell], solid_tag);

    let _ = face_idx;
    Ok(RevolveResult { solid, faces })
}

fn rotate_point_around_axis(point: Point3, axis_origin: Point3, axis: Vec3, angle: f64) -> Point3 {
    let v = point - axis_origin;
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    let dot = axis.dot(v);
    let cross = axis.cross(v);
    // Rodrigues' rotation formula
    let rotated = v * cos_a + cross * sin_a + axis * (dot * (1.0 - cos_a));
    axis_origin + rotated
}

fn make_quad_face(
    model: &mut BRepModel,
    verts: &[Handle<VertexData>; 4],
    op: cadkernel_topology::OperationId,
    face_idx: u32,
) -> KernelResult<Handle<FaceData>> {
    let edge_base = face_idx * 10;
    let mut half_edges = Vec::with_capacity(4);
    for i in 0..4 {
        let j = (i + 1) % 4;
        let tag = Tag::generated(EntityKind::Edge, op, edge_base + i as u32);
        let (_, he_a, _) = model.add_edge_tagged(verts[i], verts[j], tag);
        half_edges.push(he_a);
    }
    let loop_h = model.make_loop(&half_edges)?;
    let face_tag = Tag::generated(EntityKind::Face, op, face_idx);
    Ok(model.make_face_tagged(loop_h, face_tag))
}

fn make_ngon_face(
    model: &mut BRepModel,
    verts: &[Handle<VertexData>],
    op: cadkernel_topology::OperationId,
    face_idx: u32,
    reverse: bool,
) -> KernelResult<Handle<FaceData>> {
    let n = verts.len();
    let ordered: Vec<Handle<VertexData>> = if reverse {
        verts.iter().rev().copied().collect()
    } else {
        verts.to_vec()
    };

    let edge_base = face_idx * 100;
    let mut half_edges = Vec::with_capacity(n);
    for i in 0..n {
        let j = (i + 1) % n;
        let tag = Tag::generated(EntityKind::Edge, op, edge_base + i as u32);
        let (_, he_a, _) = model.add_edge_tagged(ordered[i], ordered[j], tag);
        half_edges.push(he_a);
    }
    let loop_h = model.make_loop(&half_edges)?;
    let face_tag = Tag::generated(EntityKind::Face, op, face_idx);
    Ok(model.make_face_tagged(loop_h, face_tag))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::TAU;

    #[test]
    fn test_revolve_full_360() {
        let mut model = BRepModel::new();
        // Revolve a vertical line segment around Y axis → cylinder-like shape
        let profile = vec![Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)];

        let result = revolve(&mut model, &profile, Point3::ORIGIN, Vec3::Y, TAU, 8).unwrap();

        // 8 segments × 1 face per segment = 8 side faces, no caps
        assert_eq!(result.faces.len(), 8);
        assert_eq!(model.solids.len(), 1);
        assert_eq!(model.vertices.len(), 16); // 8 rings × 2 pts
    }

    #[test]
    fn test_revolve_partial_90deg() {
        let mut model = BRepModel::new();
        let profile = vec![Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)];

        let result = revolve(
            &mut model,
            &profile,
            Point3::ORIGIN,
            Vec3::Y,
            std::f64::consts::FRAC_PI_2,
            4,
        )
        .unwrap();

        // 4 segments × 1 + 2 caps = 6 faces
        assert_eq!(result.faces.len(), 6);
        assert_eq!(model.solids.len(), 1);
        // 5 rings × 2 pts = 10 vertices
        assert_eq!(model.vertices.len(), 10);
    }

    #[test]
    fn test_revolve_tags_present() {
        let mut model = BRepModel::new();
        let profile = vec![Point3::new(2.0, 0.0, 0.0), Point3::new(2.0, 3.0, 0.0)];

        let result = revolve(&mut model, &profile, Point3::ORIGIN, Vec3::Y, TAU, 6).unwrap();

        let op = model.history.records()[0].operation;
        let face_tag = Tag::generated(EntityKind::Face, op, 0);
        assert!(model.find_face_by_tag(&face_tag).is_some());
        assert!(model.solids.is_alive(result.solid));
    }

    #[test]
    fn test_revolve_rectangle_profile() {
        let mut model = BRepModel::new();
        // 3-point profile → 2 quads per segment
        let profile = vec![
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(2.0, 1.0, 0.0),
        ];

        let result = revolve(&mut model, &profile, Point3::ORIGIN, Vec3::Y, TAU, 6).unwrap();

        // 6 segments × 2 quads = 12 faces, no caps
        assert_eq!(result.faces.len(), 12);
    }
}
