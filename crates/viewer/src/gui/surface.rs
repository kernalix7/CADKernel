//! Surface workbench actions.
//!
//! Extracted from `gui/mod.rs` as part of the viewer module split. Holds the
//! seven Surface-prefixed top-level actions previously inlined in `GuiAction`.

/// Actions specific to the Surface workbench, dispatched through
/// `GuiAction::Surface(SurfaceAction)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SurfaceAction {
    Filling,
    Boundary,
    Sections,
    Extend,
    Blend,
    Pipe,
    Coons,
}
