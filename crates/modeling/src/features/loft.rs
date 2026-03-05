//! Loft operation: blends between two or more cross-section profiles.

use cadkernel_core::KernelResult;
use cadkernel_math::Point3;
use cadkernel_topology::{BRepModel, FaceData, Handle, SolidData};

/// Result of a loft operation.
#[derive(Debug)]
pub struct LoftResult {
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Lofts between two or more cross-section profiles to produce a solid.
///
/// Each profile is a closed polyline given as a slice of 3D points.
/// Profiles must all have the same number of points.
pub fn loft(_model: &mut BRepModel, _profiles: &[&[Point3]]) -> KernelResult<LoftResult> {
    todo!("loft not yet implemented")
}
