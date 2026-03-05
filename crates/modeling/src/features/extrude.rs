use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, EntityKind, FaceData, Handle, SolidData, Tag, VertexData};

/// Handles returned from [`extrude`].
pub struct ExtrudeResult {
    pub solid: Handle<SolidData>,
    pub bottom_face: Handle<FaceData>,
    pub top_face: Handle<FaceData>,
    pub side_faces: Vec<Handle<FaceData>>,
}

/// Extrudes a planar profile along a direction to produce a B-Rep solid.
///
/// `profile` is an ordered sequence of 3D points forming a closed polygon.
/// The extrusion direction is `direction * distance`.
///
/// Produces: N bottom vertices, N top vertices, 1 bottom face, 1 top face,
/// N side quad faces, 1 shell, 1 solid.  All entities carry persistent tags.
pub fn extrude(
    model: &mut BRepModel,
    profile: &[Point3],
    direction: Vec3,
    distance: f64,
) -> KernelResult<ExtrudeResult> {
    let n = profile.len();
    if n < 3 {
        return Err(KernelError::InvalidArgument(
            "extrude requires at least 3 profile points".into(),
        ));
    }

    let op = model.history.next_operation("extrude");
    let offset = direction.normalized().unwrap_or(Vec3::Z) * distance;

    // --- Vertices ---
    let mut bot_v: Vec<Handle<VertexData>> = Vec::with_capacity(n);
    let mut top_v: Vec<Handle<VertexData>> = Vec::with_capacity(n);

    for (i, &pt) in profile.iter().enumerate() {
        let tag = Tag::generated(EntityKind::Vertex, op, i as u32);
        bot_v.push(model.add_vertex_tagged(pt, tag));

        let tag_top = Tag::generated(EntityKind::Vertex, op, (n + i) as u32);
        top_v.push(model.add_vertex_tagged(pt + offset, tag_top));
    }

    // --- Bottom face (CCW from outside = CW from top = reversed winding) ---
    let bottom_face = make_polygon_face(model, &bot_v, op, 0, true)?;

    // --- Top face (CCW from outside) ---
    let top_face = make_polygon_face(model, &top_v, op, 1, false)?;

    // --- Side faces ---
    let mut side_faces = Vec::with_capacity(n);
    for i in 0..n {
        let j = (i + 1) % n;
        let quad = [bot_v[i], bot_v[j], top_v[j], top_v[i]];

        let face_idx = (2 + i) as u32;
        let sf = make_polygon_face(model, &quad, op, face_idx, false)?;
        side_faces.push(sf);
    }

    // --- Assemble ---
    let mut all_faces = vec![bottom_face, top_face];
    all_faces.extend_from_slice(&side_faces);

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&all_faces, shell_tag);

    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let solid = model.make_solid_tagged(&[shell], solid_tag);

    Ok(ExtrudeResult {
        solid,
        bottom_face,
        top_face,
        side_faces,
    })
}

fn make_polygon_face(
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

    let mut half_edges = Vec::with_capacity(n);
    let edge_base = face_idx * 100;
    for i in 0..n {
        let j = (i + 1) % n;
        let edge_tag = Tag::generated(EntityKind::Edge, op, edge_base + i as u32);
        let (_, he_a, _) = model.add_edge_tagged(ordered[i], ordered[j], edge_tag);
        half_edges.push(he_a);
    }

    let loop_h = model.make_loop(&half_edges)?;
    let face_tag = Tag::generated(EntityKind::Face, op, face_idx);
    Ok(model.make_face_tagged(loop_h, face_tag))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extrude_square_entity_counts() {
        let mut model = BRepModel::new();
        let profile = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];

        let result = extrude(&mut model, &profile, Vec3::Z, 2.0).unwrap();

        assert_eq!(model.vertices.len(), 8);
        assert_eq!(model.faces.len(), 6);
        assert_eq!(model.shells.len(), 1);
        assert_eq!(model.solids.len(), 1);
        assert_eq!(result.side_faces.len(), 4);
    }

    #[test]
    fn test_extrude_tags_present() {
        let mut model = BRepModel::new();
        let profile = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];

        let result = extrude(&mut model, &profile, Vec3::Z, 5.0).unwrap();

        let op = model.history.records()[0].operation;
        let bottom_tag = Tag::generated(EntityKind::Face, op, 0);
        let top_tag = Tag::generated(EntityKind::Face, op, 1);
        assert_eq!(
            model.find_face_by_tag(&bottom_tag),
            Some(result.bottom_face)
        );
        assert_eq!(model.find_face_by_tag(&top_tag), Some(result.top_face));
    }

    #[test]
    fn test_extrude_top_vertices_offset() {
        let mut model = BRepModel::new();
        let profile = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];

        let _result = extrude(&mut model, &profile, Vec3::Z, 3.0).unwrap();

        let op = model.history.records()[0].operation;
        for i in 0..4 {
            let top_tag = Tag::generated(EntityKind::Vertex, op, (4 + i) as u32);
            let vh = model.find_vertex_by_tag(&top_tag).unwrap();
            let v = model.vertices.get(vh).unwrap();
            assert!(
                (v.point.z - 3.0).abs() < 1e-8,
                "top vertex z = {}",
                v.point.z
            );
        }
    }
}
