//! Compound operations: boolean fragments, slice to compound, compound filter.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, Handle, SolidData};

use crate::boolean::{BooleanOp, boolean_op};
use crate::compound::Compound;
use crate::features::split::split_solid;

/// Result of a boolean fragments operation.
#[derive(Debug)]
pub struct BooleanFragmentsResult {
    pub compound: Compound,
}

/// Result of a slice-to-compound operation.
#[derive(Debug)]
pub struct SliceToCompoundResult {
    pub compound: Compound,
}

/// Computes boolean fragments of two solids.
///
/// Fragments the two solids into up to three regions:
/// A only, B only, and A∩B (intersection).
/// Returns all non-empty fragments as a compound.
pub fn boolean_fragments(
    model_a: &BRepModel,
    solid_a: Handle<SolidData>,
    model_b: &BRepModel,
    solid_b: Handle<SolidData>,
) -> KernelResult<BooleanFragmentsResult> {
    let mut compound = Compound::new("boolean_fragments");

    // Region 1: A - B (A only)
    let a_only = boolean_op(model_a, solid_a, model_b, solid_b, BooleanOp::Difference)?;
    for (h, _) in a_only.solids.iter() {
        compound.add(h);
    }

    // Region 2: A ∩ B (intersection)
    let a_and_b = boolean_op(model_a, solid_a, model_b, solid_b, BooleanOp::Intersection)?;
    for (h, _) in a_and_b.solids.iter() {
        compound.add(h);
    }

    // Region 3: B - A (B only)
    let b_only = boolean_op(model_b, solid_b, model_a, solid_a, BooleanOp::Difference)?;
    for (h, _) in b_only.solids.iter() {
        compound.add(h);
    }

    Ok(BooleanFragmentsResult { compound })
}

/// Slices a solid along a plane and returns the pieces as a compound.
///
/// Unlike `split_solid` which returns raw solid handles, this wraps
/// the result into a `Compound` for downstream compound operations.
pub fn slice_to_compound(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    plane_point: Point3,
    plane_normal: Vec3,
) -> KernelResult<SliceToCompoundResult> {
    let split = split_solid(model, solid, plane_point, plane_normal)?;

    let mut compound = Compound::new("slice_to_compound");
    for sh in split.solids {
        compound.add(sh);
    }

    Ok(SliceToCompoundResult { compound })
}

/// Filters compound solids by face count threshold.
///
/// Returns a new compound containing only solids whose face count
/// is at least `min_faces`.
pub fn compound_filter(
    model: &BRepModel,
    compound: &Compound,
    min_faces: usize,
) -> KernelResult<Compound> {
    let mut result = Compound::new("filtered");

    for &solid_h in &compound.solids {
        let sd = model
            .solids
            .get(solid_h)
            .ok_or(KernelError::InvalidHandle("solid"))?;

        let mut face_count = 0;
        for &shell_h in &sd.shells {
            let sh = model
                .shells
                .get(shell_h)
                .ok_or(KernelError::InvalidHandle("shell"))?;
            face_count += sh.faces.len();
        }

        if face_count >= min_faces {
            result.add(solid_h);
        }
    }

    Ok(result)
}

/// Explodes a compound into its constituent solids.
///
/// This is a convenience wrapper around `Compound::explode()` that
/// returns the exploded handles.
pub fn explode_compound(compound: &Compound) -> Vec<Handle<SolidData>> {
    compound.explode()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::make_box;

    #[test]
    fn test_boolean_fragments_disjoint() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(10.0, 10.0, 10.0), 2.0, 2.0, 2.0).unwrap();

        let result = boolean_fragments(&a, ra.solid, &b, rb.solid).unwrap();
        // Disjoint: A-B=A, A∩B=∅, B-A=B → 2 fragments
        assert_eq!(result.compound.solids.len(), 2);
    }

    #[test]
    fn test_slice_to_compound() {
        let mut model = BRepModel::new();
        let b = make_box(&mut model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let result = slice_to_compound(
            &mut model,
            b.solid,
            Point3::new(0.0, 0.0, 2.0),
            Vec3::Z,
        )
        .unwrap();

        assert_eq!(result.compound.solids.len(), 2);
    }

    #[test]
    fn test_compound_filter() {
        let mut model = BRepModel::new();
        let r1 = make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();
        let r2 = make_box(&mut model, Point3::new(5.0, 0.0, 0.0), 2.0, 2.0, 2.0).unwrap();

        let mut compound = Compound::new("test");
        compound.add(r1.solid);
        compound.add(r2.solid);

        let filtered = compound_filter(&model, &compound, 6).unwrap();
        assert_eq!(filtered.solids.len(), 2);

        let filtered_strict = compound_filter(&model, &compound, 100).unwrap();
        assert_eq!(filtered_strict.solids.len(), 0);
    }

    #[test]
    fn test_explode_compound() {
        let mut model = BRepModel::new();
        let r1 = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let r2 = make_box(&mut model, Point3::new(5.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap();

        let mut compound = Compound::new("test");
        compound.add(r1.solid);
        compound.add(r2.solid);

        let exploded = explode_compound(&compound);
        assert_eq!(exploded.len(), 2);
    }
}
