//! Split operation for solids.

use cadkernel_core::KernelResult;
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, FaceData, Handle, SolidData};

/// Result of a split operation.
#[derive(Debug)]
pub struct SplitResult {
    pub solids: Vec<Handle<SolidData>>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Splits a solid along a plane defined by a point and normal.
///
/// Returns the resulting pieces as new solids.
pub fn split_solid(
    _model: &mut BRepModel,
    _solid: Handle<SolidData>,
    _plane_point: Point3,
    _plane_normal: Vec3,
) -> KernelResult<SplitResult> {
    todo!("split_solid not yet implemented")
}
