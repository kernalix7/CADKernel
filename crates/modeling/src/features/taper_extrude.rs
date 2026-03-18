//! Taper extrude: extrudes a profile with a taper angle.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, EntityKind, FaceData, Handle, SolidData, Tag, VertexData};

/// Result of a taper extrude operation.
#[derive(Debug)]
pub struct TaperExtrudeResult {
    pub solid: Handle<SolidData>,
    pub bottom_face: Handle<FaceData>,
    pub top_face: Handle<FaceData>,
    pub side_faces: Vec<Handle<FaceData>>,
}

/// Extrudes a profile with a taper angle.
///
/// `profile` is an ordered sequence of 3D points forming a closed polygon.
/// `direction` is the extrusion direction (will be normalized).
/// `height` is the extrusion distance along the direction.
/// `taper_angle` is the taper angle in radians — positive tapers inward,
/// negative tapers outward.
///
/// The top profile is computed by offsetting each bottom vertex inward
/// (perpendicular to the direction) by `height * tan(taper_angle)`.
pub fn taper_extrude(
    model: &mut BRepModel,
    profile: &[Point3],
    direction: Vec3,
    height: f64,
    taper_angle: f64,
) -> KernelResult<TaperExtrudeResult> {
    let n = profile.len();
    if n < 3 {
        return Err(KernelError::InvalidArgument(
            "taper_extrude requires at least 3 profile points".into(),
        ));
    }
    if height <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "taper_extrude height must be positive".into(),
        ));
    }

    let op = model.history.next_operation("taper_extrude");
    let dir = direction.normalized().unwrap_or(Vec3::Z);
    let offset = dir * height;
    let inward_dist = height * taper_angle.tan();

    // Compute profile centroid
    let cx = profile.iter().map(|p| p.x).sum::<f64>() / n as f64;
    let cy = profile.iter().map(|p| p.y).sum::<f64>() / n as f64;
    let cz = profile.iter().map(|p| p.z).sum::<f64>() / n as f64;
    let centroid = Point3::new(cx, cy, cz);

    // Create bottom vertices
    let mut bot_v: Vec<Handle<VertexData>> = Vec::with_capacity(n);
    let mut top_v: Vec<Handle<VertexData>> = Vec::with_capacity(n);

    for (i, &pt) in profile.iter().enumerate() {
        let tag = Tag::generated(EntityKind::Vertex, op, i as u32);
        bot_v.push(model.add_vertex_tagged(pt, tag));

        // Compute inward direction for this vertex (toward centroid, projected onto
        // the plane perpendicular to the extrusion direction)
        let to_center = Point3::new(
            centroid.x - pt.x,
            centroid.y - pt.y,
            centroid.z - pt.z,
        );
        // Remove the component along the extrusion direction
        let dot = to_center.x * dir.x + to_center.y * dir.y + to_center.z * dir.z;
        let planar = Vec3::new(
            to_center.x - dot * dir.x,
            to_center.y - dot * dir.y,
            to_center.z - dot * dir.z,
        );
        let planar_len = planar.length();
        let inward = if planar_len > 1e-12 {
            Vec3::new(planar.x / planar_len, planar.y / planar_len, planar.z / planar_len)
        } else {
            Vec3::ZERO
        };

        let top_pt = Point3::new(
            pt.x + offset.x + inward.x * inward_dist,
            pt.y + offset.y + inward.y * inward_dist,
            pt.z + offset.z + inward.z * inward_dist,
        );
        let tag_top = Tag::generated(EntityKind::Vertex, op, (n + i) as u32);
        top_v.push(model.add_vertex_tagged(top_pt, tag_top));
    }

    // Bottom face (reversed winding for outward normal)
    let bottom_face = make_polygon_face(model, &bot_v, op, 0, true)?;

    // Top face (normal winding)
    let top_face = make_polygon_face(model, &top_v, op, 1, false)?;

    // Side faces
    let mut side_faces = Vec::with_capacity(n);
    for i in 0..n {
        let j = (i + 1) % n;
        let quad = [bot_v[i], bot_v[j], top_v[j], top_v[i]];
        let face_idx = (2 + i) as u32;
        let sf = make_polygon_face(model, &quad, op, face_idx, false)?;
        side_faces.push(sf);
    }

    // Assemble shell and solid
    let mut all_faces = vec![bottom_face, top_face];
    all_faces.extend_from_slice(&side_faces);

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&all_faces, shell_tag);

    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let solid = model.make_solid_tagged(&[shell], solid_tag);

    Ok(TaperExtrudeResult {
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
    fn test_taper_extrude_square_inward() {
        let mut model = BRepModel::new();
        let profile = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(2.0, 2.0, 0.0),
            Point3::new(0.0, 2.0, 0.0),
        ];

        let taper_angle = 10.0_f64.to_radians();
        let height = 5.0;
        let result = taper_extrude(&mut model, &profile, Vec3::Z, height, taper_angle).unwrap();

        assert_eq!(model.vertices.len(), 8);
        assert_eq!(model.faces.len(), 6);
        assert_eq!(model.solids.len(), 1);
        assert_eq!(result.side_faces.len(), 4);

        // Top profile should be smaller (tapered inward)
        // The inward offset is height * tan(10°) ≈ 0.882
        let op = model.history.records()[0].operation;
        let top_tag_0 = Tag::generated(EntityKind::Vertex, op, 4);
        let top_vh = model.find_vertex_by_tag(&top_tag_0).unwrap();
        let top_v = model.vertices.get(top_vh).unwrap();
        // Bottom vertex 0 is at (0,0,0), centroid is (1,1,0)
        // Inward direction is toward (1,1), so top_x > 0
        assert!(top_v.point.x > 0.1, "top vertex should be inward from origin");
        assert!((top_v.point.z - height).abs() < 1e-8, "top z should equal height");
    }

    #[test]
    fn test_taper_extrude_zero_angle_matches_extrude() {
        let mut model = BRepModel::new();
        let profile = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];

        let result = taper_extrude(&mut model, &profile, Vec3::Z, 3.0, 0.0).unwrap();
        assert_eq!(model.vertices.len(), 8);
        assert_eq!(model.faces.len(), 6);

        // With zero taper, top vertices should be directly above bottom
        let op = model.history.records()[0].operation;
        for i in 0..4 {
            let bot_tag = Tag::generated(EntityKind::Vertex, op, i);
            let top_tag = Tag::generated(EntityKind::Vertex, op, 4 + i);
            let bot = model.vertices.get(model.find_vertex_by_tag(&bot_tag).unwrap()).unwrap();
            let top = model.vertices.get(model.find_vertex_by_tag(&top_tag).unwrap()).unwrap();
            assert!((top.point.x - bot.point.x).abs() < 1e-10);
            assert!((top.point.y - bot.point.y).abs() < 1e-10);
            assert!((top.point.z - 3.0).abs() < 1e-10);
        }

        // Verify solid handle
        assert!(model.solids.get(result.solid).is_some());
    }

    #[test]
    fn test_taper_extrude_validation() {
        let mut model = BRepModel::new();
        let too_few = vec![Point3::ORIGIN, Point3::new(1.0, 0.0, 0.0)];
        assert!(taper_extrude(&mut model, &too_few, Vec3::Z, 1.0, 0.1).is_err());

        let profile = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.5, 1.0, 0.0),
        ];
        assert!(taper_extrude(&mut model, &profile, Vec3::Z, -1.0, 0.1).is_err());
    }
}
