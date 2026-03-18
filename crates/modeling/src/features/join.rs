//! Join operations: connect, embed, cutout.
//!
//! These operations merge solids while attempting to maintain
//! surface continuity where the solids meet.

use cadkernel_core::KernelResult;
use cadkernel_topology::{BRepModel, Handle, SolidData};

use crate::boolean::{BooleanOp, boolean_op};

/// Result of a join operation.
#[derive(Debug)]
pub struct JoinResult {
    pub solid: Handle<SolidData>,
}

/// Connects two solids by boolean union, producing a single solid.
///
/// This is the Part workbench "Connect Shapes" operation: it fuses
/// overlapping regions and keeps the union of both solids.
pub fn connect_shapes(
    model_a: &BRepModel,
    solid_a: Handle<SolidData>,
    model_b: &BRepModel,
    solid_b: Handle<SolidData>,
) -> KernelResult<BRepModel> {
    boolean_op(model_a, solid_a, model_b, solid_b, BooleanOp::Union)
}

/// Embeds solid B into solid A.
///
/// This is the Part workbench "Embed Shapes" operation: it performs
/// a union where B is placed inside or overlapping with A, keeping
/// all geometry from both solids.
pub fn embed_shapes(
    model_a: &BRepModel,
    solid_a: Handle<SolidData>,
    model_b: &BRepModel,
    solid_b: Handle<SolidData>,
) -> KernelResult<BRepModel> {
    boolean_op(model_a, solid_a, model_b, solid_b, BooleanOp::Union)
}

/// Cuts solid A with solid B, keeping A's volume minus B's intersection.
///
/// This is the Part workbench "Cutout Shape" operation: it performs
/// a boolean difference where B is subtracted from A.
pub fn cutout_shapes(
    model_a: &BRepModel,
    solid_a: Handle<SolidData>,
    model_b: &BRepModel,
    solid_b: Handle<SolidData>,
) -> KernelResult<BRepModel> {
    boolean_op(model_a, solid_a, model_b, solid_b, BooleanOp::Difference)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::make_box;
    use cadkernel_math::Point3;

    #[test]
    fn test_connect_shapes_disjoint() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(10.0, 10.0, 10.0), 2.0, 2.0, 2.0).unwrap();

        let result = connect_shapes(&a, ra.solid, &b, rb.solid).unwrap();
        assert!(result.faces.len() >= 12);
    }

    #[test]
    fn test_embed_shapes() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(10.0, 10.0, 10.0), 1.0, 1.0, 1.0).unwrap();

        let result = embed_shapes(&a, ra.solid, &b, rb.solid).unwrap();
        assert_eq!(result.solids.len(), 1);
    }

    #[test]
    fn test_cutout_shapes() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(10.0, 10.0, 10.0), 1.0, 1.0, 1.0).unwrap();

        let result = cutout_shapes(&a, ra.solid, &b, rb.solid).unwrap();
        assert_eq!(result.faces.len(), 6);
    }
}
