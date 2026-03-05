//! Shell (hollowing) operation for solids.

use cadkernel_core::KernelResult;
use cadkernel_topology::{BRepModel, FaceData, Handle, SolidData};

/// Result of a shell operation.
#[derive(Debug)]
pub struct ShellResult {
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Hollows out a solid by removing specified faces and offsetting the
/// remaining faces inward by `thickness`.
///
/// A **new** solid is created; the original is not modified.
pub fn shell_solid(
    _model: &mut BRepModel,
    _solid: Handle<SolidData>,
    _faces_to_remove: &[Handle<FaceData>],
    _thickness: f64,
) -> KernelResult<ShellResult> {
    todo!("shell_solid not yet implemented")
}
