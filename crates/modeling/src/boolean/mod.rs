pub mod broad_phase;
pub mod classify;
pub mod csg;
pub mod evaluate;
pub mod face_split;
pub mod trim_validate;

pub use csg::BooleanOp;
pub use evaluate::boolean_op;
pub use face_split::{BooleanSplitResult, fit_ssi_to_nurbs, fit_ssi_to_pcurve, split_solids_at_intersection};
pub use trim_validate::{TrimIssue, TrimValidation, ensure_correct_winding, validate_trim};

use cadkernel_core::KernelResult;
use cadkernel_topology::{BRepModel, Handle, SolidData};

/// Boolean operation with face splitting preprocessing.
///
/// Unlike `boolean_op` which classifies whole faces, this version first splits
/// faces along intersection curves so that partially overlapping faces are
/// handled correctly.
pub fn boolean_op_exact(
    model_a: &BRepModel,
    solid_a: Handle<SolidData>,
    model_b: &BRepModel,
    solid_b: Handle<SolidData>,
    op: BooleanOp,
    tolerance: f64,
) -> KernelResult<BRepModel> {
    // Step 1: Split faces along intersection curves
    let split = split_solids_at_intersection(model_a, solid_a, model_b, solid_b, tolerance)?;

    // Step 2: Run standard boolean on the split models
    boolean_op(
        &split.model_a,
        split.solid_a,
        &split.model_b,
        split.solid_b,
        op,
    )
}

/// Boolean XOR (exclusive-or): union minus intersection.
///
/// XOR = (A + B) - (A * B). Computes union, then intersection, then
/// subtracts the intersection from the union.
pub fn boolean_xor(
    model_a: &BRepModel,
    solid_a: Handle<SolidData>,
    model_b: &BRepModel,
    solid_b: Handle<SolidData>,
) -> KernelResult<BRepModel> {
    let union_model = boolean_op(model_a, solid_a, model_b, solid_b, BooleanOp::Union)?;
    let intersect_model =
        boolean_op(model_a, solid_a, model_b, solid_b, BooleanOp::Intersection)?;

    // If intersection is empty, XOR = union
    if intersect_model.solids.is_empty() {
        return Ok(union_model);
    }

    // Get solid handles from the result models
    let union_solid = union_model
        .solids
        .iter()
        .next()
        .map(|(h, _)| h)
        .ok_or_else(|| {
            cadkernel_core::KernelError::InvalidArgument("union produced no solid".into())
        })?;
    let intersect_solid = intersect_model
        .solids
        .iter()
        .next()
        .map(|(h, _)| h)
        .ok_or_else(|| {
            cadkernel_core::KernelError::InvalidArgument("intersection produced no solid".into())
        })?;

    boolean_op(
        &union_model,
        union_solid,
        &intersect_model,
        intersect_solid,
        BooleanOp::Difference,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::make_box;
    use cadkernel_math::Point3;
    use cadkernel_topology::BRepModel;

    #[test]
    fn test_union_produces_combined_faces() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(10.0, 10.0, 10.0), 2.0, 2.0, 2.0).unwrap();

        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Union).unwrap();
        assert!(
            result.faces.len() >= 12,
            "union of two disjoint boxes should keep all 12 faces, got {}",
            result.faces.len()
        );
        assert_eq!(result.solids.len(), 1);
    }

    #[test]
    fn test_subtract_removes_intersection() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(10.0, 10.0, 10.0), 2.0, 2.0, 2.0).unwrap();

        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Difference).unwrap();
        assert_eq!(
            result.faces.len(),
            6,
            "difference with disjoint box should keep A's 6 faces"
        );
    }

    #[test]
    fn test_intersect_keeps_only_overlap() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(10.0, 10.0, 10.0), 2.0, 2.0, 2.0).unwrap();

        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Intersection).unwrap();
        assert_eq!(
            result.faces.len(),
            0,
            "intersection of disjoint boxes should produce no faces"
        );
    }

    #[test]
    fn test_xor_disjoint_boxes() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(10.0, 10.0, 10.0), 2.0, 2.0, 2.0).unwrap();

        let result = boolean_xor(&a, ra.solid, &b, rb.solid).unwrap();
        // Disjoint: XOR = union (no intersection to subtract)
        assert!(
            result.faces.len() >= 12,
            "XOR of disjoint boxes should keep all 12 faces, got {}",
            result.faces.len()
        );
    }

    #[test]
    fn test_exact_boolean_disjoint() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(5.0, 5.0, 5.0), 1.0, 1.0, 1.0).unwrap();

        let result =
            boolean_op_exact(&a, ra.solid, &b, rb.solid, BooleanOp::Union, 0.001).unwrap();
        assert!(
            result.faces.len() >= 12,
            "exact union of disjoint boxes should keep all 12 faces"
        );
    }

    #[test]
    fn test_exact_boolean_overlapping() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(1.0, 1.0, 1.0), 2.0, 2.0, 2.0).unwrap();

        // This should at least not panic and produce a valid model
        let result =
            boolean_op_exact(&a, ra.solid, &b, rb.solid, BooleanOp::Union, 0.001).unwrap();
        assert!(
            !result.faces.is_empty(),
            "exact union of overlapping boxes should produce faces"
        );
        assert_eq!(result.solids.len(), 1);
    }
}
