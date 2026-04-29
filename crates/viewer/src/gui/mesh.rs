//! Mesh workbench actions.
//!
//! Extracted from `gui/mod.rs` as part of the viewer module split. Operations
//! here transform an imported triangle mesh in place; they do not touch the
//! B-Rep model.

/// Actions specific to the Mesh workbench, dispatched through
/// `GuiAction::Mesh(MeshAction)`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum MeshAction {
    Decimate(f64),
    Subdivide,
    FlipNormals,
    FillHoles,
    Smooth { iterations: usize, factor: f64 },
    HarmonizeNormals,
    CheckWatertight,
    Remesh { target_edge_len: f64 },
    Repair,
}
