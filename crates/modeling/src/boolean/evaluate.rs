use rayon::prelude::*;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::Point3;
use cadkernel_topology::{BRepModel, EntityKind, FaceData, HalfEdgeData, Handle, SolidData, Tag};

use super::broad_phase::collect_solid_faces;
use super::classify::{FacePosition, classify_face};
use super::csg::BooleanOp;

/// Performs a boolean operation between two solids.
///
/// Automatically detects overlapping faces and applies face-splitting
/// when needed for correct results. Falls back to simple classification
/// when solids are disjoint or fully contained.
///
/// Returns `Err` for degenerate inputs (empty solids, invalid handles)
/// instead of panicking.
pub fn boolean_op(
    model_a: &BRepModel,
    solid_a: Handle<SolidData>,
    model_b: &BRepModel,
    solid_b: Handle<SolidData>,
    op: BooleanOp,
) -> KernelResult<BRepModel> {
    // Validate inputs — never panic on bad handles
    if model_a.solids.get(solid_a).is_none() {
        return Err(KernelError::InvalidHandle("solid_a"));
    }
    if model_b.solids.get(solid_b).is_none() {
        return Err(KernelError::InvalidHandle("solid_b"));
    }

    // Check for degenerate solids (no shells or no faces)
    let has_faces_a = model_a
        .solids
        .get(solid_a)
        .is_some_and(|s| s.shells.iter().any(|&sh| {
            model_a.shells.get(sh).is_some_and(|shell| !shell.faces.is_empty())
        }));
    let has_faces_b = model_b
        .solids
        .get(solid_b)
        .is_some_and(|s| s.shells.iter().any(|&sh| {
            model_b.shells.get(sh).is_some_and(|shell| !shell.faces.is_empty())
        }));

    if !has_faces_a && !has_faces_b {
        return Ok(BRepModel::new());
    }
    // For union with one empty operand, return the non-empty one
    if !has_faces_a {
        return match op {
            BooleanOp::Union => copy_solid_to_new_model(model_b, solid_b),
            BooleanOp::Intersection => Ok(BRepModel::new()),
            BooleanOp::Difference => Ok(BRepModel::new()),
        };
    }
    if !has_faces_b {
        return match op {
            BooleanOp::Union | BooleanOp::Difference => copy_solid_to_new_model(model_a, solid_a),
            BooleanOp::Intersection => Ok(BRepModel::new()),
        };
    }

    // Try split path first — if faces overlap, split them for precision
    let split_result = super::face_split::split_solids_at_intersection(
        model_a, solid_a, model_b, solid_b, 1e-6,
    );

    // Use split models if splitting succeeded and produced splits
    let (eff_a, eff_solid_a, eff_b, eff_solid_b) = match &split_result {
        Ok(sr) if sr.had_splits => (&sr.model_a, sr.solid_a, &sr.model_b, sr.solid_b),
        _ => (model_a, solid_a, model_b, solid_b),
    };

    let faces_a = collect_solid_faces(eff_a, eff_solid_a)?;
    let faces_b = collect_solid_faces(eff_b, eff_solid_b)?;

    let mut result = BRepModel::new();
    let result_op = result.history.next_operation(match op {
        BooleanOp::Union => "boolean_union",
        BooleanOp::Intersection => "boolean_intersection",
        BooleanOp::Difference => "boolean_difference",
    });

    // Classify faces from A in parallel
    let classified_a: Vec<(Handle<FaceData>, FacePosition)> = faces_a
        .par_iter()
        .map(|&face_h| {
            classify_face(eff_a, face_h, eff_b, eff_solid_b).map(|pos| (face_h, pos))
        })
        .collect::<KernelResult<Vec<_>>>()?;

    // Classify faces from B in parallel
    let classified_b: Vec<(Handle<FaceData>, FacePosition)> = faces_b
        .par_iter()
        .map(|&face_h| {
            classify_face(eff_b, face_h, eff_a, eff_solid_a).map(|pos| (face_h, pos))
        })
        .collect::<KernelResult<Vec<_>>>()?;

    // Copy kept faces sequentially (requires mutable result model)
    let mut result_faces: Vec<Handle<FaceData>> = Vec::new();
    let mut face_counter = 0u32;

    for (face_h, pos) in &classified_a {
        let keep = match op {
            BooleanOp::Union => *pos == FacePosition::Outside,
            BooleanOp::Intersection => *pos == FacePosition::Inside,
            BooleanOp::Difference => *pos == FacePosition::Outside,
        };
        if keep {
            let new_face = copy_face(eff_a, *face_h, &mut result, result_op, face_counter)?;
            result_faces.push(new_face);
            face_counter += 1;
        }
    }

    for (face_h, pos) in &classified_b {
        let keep = match op {
            BooleanOp::Union => *pos == FacePosition::Outside,
            BooleanOp::Intersection => *pos == FacePosition::Inside,
            BooleanOp::Difference => *pos == FacePosition::Inside,
        };
        if keep {
            let new_face = copy_face(eff_b, *face_h, &mut result, result_op, face_counter)?;
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

/// Copies a solid into a fresh BRepModel (used for degenerate-operand cases).
fn copy_solid_to_new_model(
    src: &BRepModel,
    solid: Handle<SolidData>,
) -> KernelResult<BRepModel> {
    let mut dst = BRepModel::new();
    let op = dst.history.next_operation("boolean_copy");

    let src_solid = src.solids.get(solid).ok_or(KernelError::InvalidHandle("solid"))?;
    let mut all_faces = Vec::new();

    for (i, &shell_h) in src_solid.shells.iter().enumerate() {
        let shell_data = src.shells.get(shell_h).ok_or(KernelError::InvalidHandle("shell"))?;
        let mut shell_faces = Vec::new();
        for (j, &face_h) in shell_data.faces.iter().enumerate() {
            let idx = (i * 1000 + j) as u32;
            let new_face = copy_face(src, face_h, &mut dst, op, idx)?;
            shell_faces.push(new_face);
        }
        all_faces.extend_from_slice(&shell_faces);
    }

    if !all_faces.is_empty() {
        let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
        let shell = dst.make_shell_tagged(&all_faces, shell_tag);
        let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
        dst.make_solid_tagged(&[shell], solid_tag);
    }

    Ok(dst)
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

    #[test]
    fn test_parallel_boolean_many_boxes() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(20.0, 20.0, 20.0), 3.0, 3.0, 3.0).unwrap();

        let union = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Union).unwrap();
        assert_eq!(union.faces.len(), 12);
        assert_eq!(union.solids.len(), 1);

        let inter = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Intersection).unwrap();
        assert_eq!(inter.faces.len(), 0);

        let diff = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Difference).unwrap();
        assert_eq!(diff.faces.len(), 6);
    }

    // -----------------------------------------------------------------------
    // Edge cases and robustness tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_boolean_invalid_handle_a() {
        let a = BRepModel::new();
        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        // Invalid solid handle for A
        let bad_handle = cadkernel_topology::Handle::from_raw_parts(999, 0);
        let result = boolean_op(&a, bad_handle, &b, rb.solid, BooleanOp::Union);
        assert!(result.is_err(), "should fail with invalid handle A");
    }

    #[test]
    fn test_boolean_invalid_handle_b() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let b = BRepModel::new();

        let bad_handle = cadkernel_topology::Handle::from_raw_parts(999, 0);
        let result = boolean_op(&a, ra.solid, &b, bad_handle, BooleanOp::Union);
        assert!(result.is_err(), "should fail with invalid handle B");
    }

    #[test]
    fn test_boolean_coplanar_faces() {
        // Two boxes sharing a face (touching at z=1)
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(0.0, 0.0, 1.0), 1.0, 1.0, 1.0).unwrap();

        // Union of touching boxes should not panic
        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Union);
        assert!(result.is_ok(), "coplanar face union should not panic");

        // Difference should not panic
        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Difference);
        assert!(result.is_ok(), "coplanar face difference should not panic");
    }

    #[test]
    fn test_boolean_identical_solids() {
        // Same box at same position — fully overlapping
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        // Union of identical solids should not panic
        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Union);
        assert!(result.is_ok(), "identical solid union should not panic");

        // Intersection should produce a result
        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Intersection);
        assert!(result.is_ok(), "identical solid intersection should not panic");
    }

    #[test]
    fn test_boolean_contained_solid() {
        // Small box fully inside larger box
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 10.0, 10.0, 10.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(3.0, 3.0, 3.0), 2.0, 2.0, 2.0).unwrap();

        // Difference: should keep A faces (outer), add B faces (inner)
        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Difference);
        assert!(result.is_ok(), "contained solid difference should not panic");

        // Intersection: should produce the inner box
        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Intersection);
        assert!(result.is_ok(), "contained solid intersection should not panic");
        let model = result.unwrap();
        // Inner box faces should be classified as Inside, so kept
        assert!(!model.faces.is_empty(), "intersection of containment should produce faces");
    }

    #[test]
    fn test_boolean_thin_wall() {
        // Very thin box (almost 2D) — tests near-degenerate geometry
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 10.0, 10.0, 0.001).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(5.0, 5.0, -0.5), 10.0, 10.0, 1.0).unwrap();

        // Should not panic even with very thin geometry
        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Union);
        assert!(result.is_ok(), "thin wall union should not panic");

        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Difference);
        assert!(result.is_ok(), "thin wall difference should not panic");
    }

    #[test]
    fn test_boolean_tangent_contact() {
        // Two boxes touching at a single edge
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(1.0, 1.0, 0.0), 1.0, 1.0, 1.0).unwrap();

        // Touching at edge — should not panic
        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Union);
        assert!(result.is_ok(), "edge-touching union should not panic");
    }

    #[test]
    fn test_boolean_vertex_contact() {
        // Two boxes touching at a single vertex
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(1.0, 1.0, 1.0), 1.0, 1.0, 1.0).unwrap();

        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Union);
        assert!(result.is_ok(), "vertex-touching union should not panic");
        let model = result.unwrap();
        assert_eq!(model.faces.len(), 12, "disjoint-touching union should keep all 12 faces");
    }

    #[test]
    fn test_boolean_very_small_solid() {
        // Extremely small solid — near machine epsilon
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 1e-10, 1e-10, 1e-10).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Union);
        assert!(result.is_ok(), "very small solid union should not panic");
    }

    #[test]
    fn test_boolean_empty_model_union() {
        // Model with solid but no faces
        let mut a = BRepModel::new();
        let shell = a.make_shell(&[]);
        let solid_a = a.make_solid(&[shell]);

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        // Union with empty solid should return the non-empty one
        let result = boolean_op(&a, solid_a, &b, rb.solid, BooleanOp::Union);
        assert!(result.is_ok());
        let model = result.unwrap();
        assert_eq!(model.faces.len(), 6, "union with empty should return other solid's faces");
    }

    #[test]
    fn test_boolean_empty_model_intersection() {
        let mut a = BRepModel::new();
        let shell = a.make_shell(&[]);
        let solid_a = a.make_solid(&[shell]);

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        // Intersection with empty solid should be empty
        let result = boolean_op(&a, solid_a, &b, rb.solid, BooleanOp::Intersection);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().faces.len(), 0);
    }

    #[test]
    fn test_boolean_all_ops_stress() {
        // Run all 3 operations on overlapping boxes — ensure none panic
        let offsets = [
            Point3::new(0.5, 0.0, 0.0),
            Point3::new(0.0, 0.5, 0.0),
            Point3::new(0.0, 0.0, 0.5),
            Point3::new(0.5, 0.5, 0.5),
            Point3::new(-0.5, -0.5, -0.5),
        ];

        for offset in &offsets {
            let mut a = BRepModel::new();
            let ra = make_box(&mut a, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

            let mut b = BRepModel::new();
            let rb = make_box(&mut b, *offset, 1.0, 1.0, 1.0).unwrap();

            for op in [BooleanOp::Union, BooleanOp::Intersection, BooleanOp::Difference] {
                let result = boolean_op(&a, ra.solid, &b, rb.solid, op);
                assert!(
                    result.is_ok(),
                    "boolean {:?} at offset {:?} should not panic/fail",
                    op, offset
                );
            }
        }
    }
}
