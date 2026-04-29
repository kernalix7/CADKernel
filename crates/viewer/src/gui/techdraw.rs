//! TechDraw workbench actions.
//!
//! Extracted from `gui/mod.rs` as part of the viewer module split. Holds the
//! TechDraw drawing-sheet operations (page management, view projection,
//! dimensions, annotations, centerlines, exports) previously inlined as
//! top-level `GuiAction` variants.
//!
//! Note: `TechDrawAction` derives only `Clone + Debug` because the
//! `ExportSvg` / `ExportDxf` / `ExportPdf` variants carry `PathBuf`, and
//! `cadkernel_io::ProjectionDir` lacks `PartialEq` derives in some configs.

use std::path::PathBuf;

/// Actions specific to the TechDraw workbench, dispatched through
/// `GuiAction::TechDraw(TechDrawAction)`.
#[derive(Clone, Debug)]
pub(crate) enum TechDrawAction {
    // Page management
    NewPage,
    FromTemplate,
    Redraw,
    Clear,

    // View projection
    AddView(cadkernel_io::ProjectionDir),
    ThreeView,
    SectionView,
    DetailView,
    BrokenView,

    // Dimensions
    DimLinear,
    DimRadius,
    DimDiameter,
    DimAngle,
    DimArcLen,
    DimArea,

    // Annotation
    Text,
    RichText,
    Balloon,
    Leader,
    Weld,
    SurfFinish,

    // Centerlines / bolt circles
    CenterFace,
    CenterLines,
    CenterPoints,
    BoltCircle,

    // Exports
    ExportSvg(PathBuf),
    ExportDxf(PathBuf),
    ExportPdf(PathBuf),
}
