//! Mirror operation for solids.

use cadkernel_core::KernelResult;
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, FaceData, Handle, SolidData};

/// Result of a mirror operation.
#[derive(Debug)]
pub struct MirrorResult {
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Mirrors a solid about a plane defined by a point and normal.
///
/// A **new** solid is created; the original is not modified.
pub fn mirror_solid(
    _model: &mut BRepModel,
    _solid: Handle<SolidData>,
    _plane_point: Point3,
    _plane_normal: Vec3,
) -> KernelResult<MirrorResult> {
    todo!("mirror_solid not yet implemented")
}
