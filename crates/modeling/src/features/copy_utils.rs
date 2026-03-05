//! Internal utilities for deep-copying B-Rep topology during feature operations.

use cadkernel_core::KernelResult;
use cadkernel_topology::{BRepModel, FaceData, Handle};

/// Deep-copies the faces of a solid into new topology entities.
///
/// Returns handles to the newly created faces.
#[allow(dead_code)]
pub(crate) fn deep_copy_faces(
    _model: &mut BRepModel,
    _faces: &[Handle<FaceData>],
) -> KernelResult<Vec<Handle<FaceData>>> {
    todo!("deep_copy_faces not yet implemented")
}
