//! Fillet (rounding) operation for solid edges.

use cadkernel_core::KernelResult;
use cadkernel_topology::{BRepModel, FaceData, Handle, SolidData, VertexData};

/// Result of a fillet operation.
#[derive(Debug)]
pub struct FilletResult {
    pub solid: Handle<SolidData>,
    pub fillet_faces: Vec<Handle<FaceData>>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Fillets (rounds) a single edge of a solid by a uniform radius.
///
/// `edge_v1` and `edge_v2` identify the edge by its two endpoint vertex
/// handles.  A **new** solid is created; the original is not modified.
pub fn fillet_edge(
    _model: &mut BRepModel,
    _solid: Handle<SolidData>,
    _edge_v1: Handle<VertexData>,
    _edge_v2: Handle<VertexData>,
    _radius: f64,
) -> KernelResult<FilletResult> {
    todo!("fillet_edge not yet implemented")
}
