//! Face-from-wires and points-from-shape operations.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::Point3;
use cadkernel_topology::{BRepModel, EntityKind, FaceData, Handle, SolidData, Tag, VertexData};

/// Result of creating a face from wire points.
#[derive(Debug)]
pub struct FaceFromWiresResult {
    pub face: Handle<FaceData>,
    pub vertices: Vec<Handle<VertexData>>,
}

/// Result of extracting points from a shape.
#[derive(Debug)]
pub struct PointsFromShapeResult {
    pub points: Vec<Point3>,
}

/// Creates a planar face from a closed wire defined by ordered points.
///
/// The points must be coplanar (or nearly so). The face is created by
/// connecting them in order as a polygon.
pub fn face_from_wires(
    model: &mut BRepModel,
    wire_points: &[Point3],
) -> KernelResult<FaceFromWiresResult> {
    if wire_points.len() < 3 {
        return Err(KernelError::InvalidArgument(
            "face_from_wires needs at least 3 points".into(),
        ));
    }

    let op = model.history.next_operation("face_from_wires");

    let verts: Vec<Handle<VertexData>> = wire_points
        .iter()
        .enumerate()
        .map(|(i, &pt)| {
            let tag = Tag::generated(EntityKind::Vertex, op, i as u32);
            model.add_vertex_tagged(pt, tag)
        })
        .collect();

    let n = verts.len();
    let mut half_edges = Vec::with_capacity(n);
    for i in 0..n {
        let j = (i + 1) % n;
        let etag = Tag::generated(EntityKind::Edge, op, i as u32);
        let (_, he, _) = model.add_edge_tagged(verts[i], verts[j], etag);
        half_edges.push(he);
    }

    let loop_h = model.make_loop(&half_edges)?;
    let face_tag = Tag::generated(EntityKind::Face, op, 0);
    let face = model.make_face_tagged(loop_h, face_tag);

    Ok(FaceFromWiresResult {
        face,
        vertices: verts,
    })
}

/// Extracts all vertex positions from a solid as a point cloud.
///
/// Deduplicates by vertex handle to avoid returning the same
/// vertex position multiple times.
pub fn points_from_shape(
    model: &BRepModel,
    solid: Handle<SolidData>,
) -> KernelResult<PointsFromShapeResult> {
    let sd = model
        .solids
        .get(solid)
        .ok_or(KernelError::InvalidHandle("solid"))?;

    let mut seen = std::collections::HashSet::new();
    let mut points = Vec::new();

    for &shell_h in &sd.shells {
        let sh = model
            .shells
            .get(shell_h)
            .ok_or(KernelError::InvalidHandle("shell"))?;
        for &face_h in &sh.faces {
            let verts = model.vertices_of_face(face_h)?;
            for &vh in &verts {
                if seen.insert(vh.index()) {
                    let vd = model
                        .vertices
                        .get(vh)
                        .ok_or(KernelError::InvalidHandle("vertex"))?;
                    points.push(vd.point);
                }
            }
        }
    }

    Ok(PointsFromShapeResult { points })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_face_from_wires_triangle() {
        let mut model = BRepModel::new();
        let pts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.5, 1.0, 0.0),
        ];
        let result = face_from_wires(&mut model, &pts).unwrap();
        assert_eq!(result.vertices.len(), 3);
        assert!(model.faces.is_alive(result.face));
    }

    #[test]
    fn test_face_from_wires_quad() {
        let mut model = BRepModel::new();
        let pts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(2.0, 2.0, 0.0),
            Point3::new(0.0, 2.0, 0.0),
        ];
        let result = face_from_wires(&mut model, &pts).unwrap();
        assert_eq!(result.vertices.len(), 4);
    }

    #[test]
    fn test_face_from_wires_too_few_points() {
        let mut model = BRepModel::new();
        let pts = vec![Point3::ORIGIN, Point3::new(1.0, 0.0, 0.0)];
        assert!(face_from_wires(&mut model, &pts).is_err());
    }

    #[test]
    fn test_points_from_shape_box() {
        let mut model = BRepModel::new();
        let b = crate::primitives::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();
        let result = points_from_shape(&model, b.solid).unwrap();
        assert_eq!(result.points.len(), 8);
    }
}
