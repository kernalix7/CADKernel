//! Uniform scaling operation for solids.

use cadkernel_core::KernelResult;
use cadkernel_math::Point3;
use cadkernel_topology::{BRepModel, FaceData, Handle, SolidData};

/// Result of a scale operation.
#[derive(Debug)]
pub struct ScaleResult {
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Scales a solid uniformly about a center point.
///
/// A **new** solid is created; the original is not modified.
pub fn scale_solid(
    _model: &mut BRepModel,
    _solid: Handle<SolidData>,
    _center: Point3,
    _factor: f64,
) -> KernelResult<ScaleResult> {
    todo!("scale_solid not yet implemented")
}
