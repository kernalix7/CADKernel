//! Part workbench actions.
//!
//! Extracted from `gui/mod.rs` as part of the viewer module split. Holds the
//! Part-prefixed Join / Compound / Convert top-level actions previously
//! inlined in `GuiAction`.

/// Actions specific to the Part workbench, dispatched through
/// `GuiAction::Part(PartAction)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum PartAction {
    // Join operations
    FaceFromWires,
    ConnectShapes,
    EmbedShapes,
    CutoutShapes,
    // Compound operations
    ExplodeCompound,
    CompoundFilter,
    BooleanFragments,
    SliceToCompound,
    // Convert operations
    PointsFromShape,
    ConvertToSolid,
    AutoDefeaturing { threshold: f64 },
    TransformedCopy { dx: f64, dy: f64, dz: f64 },
    ProjectCurvesOnSurface,
    CoonsPatch,
}
