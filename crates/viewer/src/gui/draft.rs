//! Draft workbench actions.
//!
//! Extracted from `gui/mod.rs` as part of the viewer module split. Holds the
//! 32 Draft-prefixed top-level actions plus `ToggleDraftSnap` previously
//! inlined in `GuiAction`.
//!
//! Note: `DraftAction` derives only `Clone + Debug + PartialEq` because
//! `ToggleSnap(String)` carries an owned string (not `Copy`).

/// Actions specific to the Draft workbench, dispatched through
/// `GuiAction::Draft(DraftAction)`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DraftAction {
    // Drawing primitives
    Line,
    Wire,
    Circle,
    Arc,
    Ellipse,
    Rectangle,
    Polygon,
    BSpline,
    Bezier,
    Point,
    Facebinder,
    Hatch,

    // Modify operations
    Move,
    Rotate,
    Scale,
    Mirror,
    Offset,
    Trim,
    Stretch,
    Clone,

    // Array operations
    ArrayRect,
    ArrayPolar,
    ArrayPath,
    ArrayPoint,

    // Annotation
    Dimension,
    Label,
    Text,

    // Conversion
    Upgrade,
    Downgrade,
    WireToBSpline,
    ToSketch,

    // Settings
    ToggleSnap(String),
}
