//! Draft (taper) operation for solid faces.

use cadkernel_core::KernelResult;
use cadkernel_math::Vec3;
use cadkernel_topology::{BRepModel, FaceData, Handle, SolidData};

/// Result of a draft operation.
#[derive(Debug)]
pub struct DraftResult {
    pub solid: Handle<SolidData>,
    pub drafted_faces: Vec<Handle<FaceData>>,
}

/// Applies a draft (taper) angle to the specified faces of a solid.
///
/// `faces` -- handles of faces to draft.
/// `pull_direction` -- the direction in which the draft taper is measured.
/// `angle` -- draft angle in radians.
pub fn draft_faces(
    _model: &mut BRepModel,
    _solid: Handle<SolidData>,
    _faces: &[Handle<FaceData>],
    _pull_direction: Vec3,
    _angle: f64,
) -> KernelResult<DraftResult> {
    todo!("draft_faces not yet implemented")
}
