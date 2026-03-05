//! Sweep operation: sweeps a profile along a path curve.

use cadkernel_core::KernelResult;
use cadkernel_math::Point3;
use cadkernel_topology::{BRepModel, FaceData, Handle, SolidData};

/// Result of a sweep operation.
#[derive(Debug)]
pub struct SweepResult {
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Sweeps a planar profile along a 3D path to produce a solid.
///
/// `profile` -- a closed polyline (the cross section).
/// `path`    -- an open polyline defining the sweep trajectory.
pub fn sweep(
    _model: &mut BRepModel,
    _profile: &[Point3],
    _path: &[Point3],
) -> KernelResult<SweepResult> {
    todo!("sweep not yet implemented")
}
