use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::Point3;
use cadkernel_topology::{BRepModel, EntityKind, FaceData, HalfEdgeData, Handle, SolidData, Tag};

use super::broad_phase::collect_solid_faces;
use super::classify::{FacePosition, classify_face};
use super::csg::BooleanOp;

/// Performs a boolean operation between two solids.
///
/// This implementation works by classifying each face of both inputs as
/// INSIDE or OUTSIDE relative to the other solid, then selecting the
/// appropriate faces for the result based on the operation type.
///
/// Limitations: faces are not split along intersection curves. This works
/// correctly when the two solids share no partial face overlaps (e.g.,
/// one box fully penetrates another face). For general cases, the
/// split/trim pipeline would need to be invoked first.
pub fn boolean_op(
    model_a: &BRepModel,
    solid_a: Handle<SolidData>,
    model_b: &BRepModel,
    solid_b: Handle<SolidData>,
    op: BooleanOp,
) -> KernelResult<BRepModel> {
    let faces_a = collect_solid_faces(model_a, solid_a)?;
    let faces_b = collect_solid_faces(model_b, solid_b)?;

    let mut result = BRepModel::new();
    let result_op = result.history.next_operation(match op {
        BooleanOp::Union => "boolean_union",
        BooleanOp::Intersection => "boolean_intersection",
        BooleanOp::Difference => "boolean_difference",
    });

    let mut result_faces: Vec<Handle<FaceData>> = Vec::new();
    let mut face_counter = 0u32;

    // Classify and copy faces from A
    for &face_h in &faces_a {
        let pos = classify_face(model_a, face_h, model_b, solid_b)?;
        let keep = match op {
            BooleanOp::Union => pos == FacePosition::Outside,
            BooleanOp::Intersection => pos == FacePosition::Inside,
            BooleanOp::Difference => pos == FacePosition::Outside,
        };
        if keep {
            let new_face = copy_face(model_a, face_h, &mut result, result_op, face_counter)?;
            result_faces.push(new_face);
            face_counter += 1;
        }
    }

    // Classify and copy faces from B
    for &face_h in &faces_b {
        let pos = classify_face(model_b, face_h, model_a, solid_a)?;
        let keep = match op {
            BooleanOp::Union => pos == FacePosition::Outside,
            BooleanOp::Intersection => pos == FacePosition::Inside,
            BooleanOp::Difference => pos == FacePosition::Inside,
        };
        if keep {
            let new_face = copy_face(model_b, face_h, &mut result, result_op, face_counter)?;
            result_faces.push(new_face);
            face_counter += 1;
        }
    }

    if !result_faces.is_empty() {
        let shell_tag = Tag::generated(EntityKind::Shell, result_op, 0);
        let shell = result.make_shell_tagged(&result_faces, shell_tag);
        let solid_tag = Tag::generated(EntityKind::Solid, result_op, 0);
        result.make_solid_tagged(&[shell], solid_tag);
    }

    Ok(result)
}

/// Copies a face (and its boundary) from `src` model into `dst` model.
fn copy_face(
    src: &BRepModel,
    face_h: Handle<FaceData>,
    dst: &mut BRepModel,
    op: cadkernel_topology::OperationId,
    face_index: u32,
) -> KernelResult<Handle<FaceData>> {
    let face_data = src
        .faces
        .get(face_h)
        .ok_or(KernelError::InvalidHandle("face"))?;
    let loop_data = src
        .loops
        .get(face_data.outer_loop)
        .ok_or(KernelError::InvalidHandle("loop"))?;
    let src_hes = src.loop_half_edges(loop_data.half_edge);

    let mut new_verts: Vec<Handle<cadkernel_topology::VertexData>> = Vec::new();
    let mut src_positions: Vec<Point3> = Vec::new();

    for &he_h in &src_hes {
        let he = src
            .half_edges
            .get(he_h)
            .ok_or(KernelError::InvalidHandle("half_edge"))?;
        let v = src
            .vertices
            .get(he.origin)
            .ok_or(KernelError::InvalidHandle("vertex"))?;
        let existing = src_positions.iter().position(|p| p.approx_eq(v.point));
        if let Some(idx) = existing {
            new_verts.push(new_verts[idx]);
        } else {
            let new_v = dst.add_vertex(v.point);
            src_positions.push(v.point);
            new_verts.push(new_v);
        }
    }

    let n = new_verts.len();
    let mut new_hes: Vec<Handle<HalfEdgeData>> = Vec::new();
    for i in 0..n {
        let vs = new_verts[i];
        let ve = new_verts[(i + 1) % n];
        let (_, he, _) = dst.add_edge(vs, ve);
        new_hes.push(he);
    }

    let new_loop = dst.make_loop(&new_hes)?;

    let tag = src
        .faces
        .get(face_h)
        .and_then(|f| f.tag.clone())
        .unwrap_or_else(|| Tag::generated(EntityKind::Face, op, face_index));

    Ok(dst.make_face_tagged(new_loop, tag))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::make_box;

    #[test]
    fn test_union_disjoint_boxes() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(5.0, 5.0, 5.0), 1.0, 1.0, 1.0).unwrap();

        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Union).unwrap();
        // Disjoint: all 12 faces should be outside each other → all kept
        assert_eq!(result.faces.len(), 12);
        assert_eq!(result.solids.len(), 1);
    }

    #[test]
    fn test_intersection_disjoint_boxes() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(5.0, 5.0, 5.0), 1.0, 1.0, 1.0).unwrap();

        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Intersection).unwrap();
        // Disjoint: no faces inside each other → empty result
        assert_eq!(result.faces.len(), 0);
    }

    #[test]
    fn test_difference_disjoint_boxes() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(5.0, 5.0, 5.0), 1.0, 1.0, 1.0).unwrap();

        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Difference).unwrap();
        // Disjoint: A is entirely outside B, B is entirely outside A
        // Keep A's outside faces (6) + B's inside faces (0) = 6
        assert_eq!(result.faces.len(), 6);
    }
}
