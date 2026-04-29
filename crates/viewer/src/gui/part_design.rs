//! PartDesign workbench actions.
//!
//! Extracted from `gui/mod.rs` as part of the viewer module split. Holds the
//! sketch-driven feature operations (Pad / Pocket / Groove / Hole / Loft /
//! Pipe / built-in gears + sprockets / shape-binder / feature reorder) that
//! were previously top-level variants of `GuiAction`.
//!
//! Note: `PartDesignAction` derives only `Clone + Debug` because
//! `CreateShaftDesign { segments }` carries a `Vec<(f64, f64)>`, which is
//! not `Copy`.

/// Actions specific to the PartDesign workbench, dispatched through
/// `GuiAction::PartDesign(PartDesignAction)`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PartDesignAction {
    // Sketch-driven primary features
    PadSketch { depth: f64, symmetric: bool },
    PocketSketch { depth: f64, through_all: bool },
    GrooveSketch { angle: f64 },
    HoleSketch { radius: f64, depth: f64 },
    CountersunkHoleSketch { radius: f64, depth: f64, countersink_angle: f64 },

    // Loft / pipe variants
    AdditiveLoft,
    AdditivePipe,
    SubtractiveLoft,
    SubtractivePipe,

    // Built-in mechanical generators
    CreateSprocket { teeth: u32, roller_diameter: f64, pitch: f64, bore: f64 },
    CreateShaftDesign { segments: Vec<(f64, f64)> },
    CreateInvoluteGear { teeth: u32, module_val: f64, pressure_angle: f64 },

    // Feature management
    ShapeBinder,
    SuppressFeature,
    SetTip,
    MoveFeatureUp,
    MoveFeatureDown,
}
