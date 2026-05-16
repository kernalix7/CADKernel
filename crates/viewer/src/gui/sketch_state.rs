//! Sketch editing state — extracted from `gui/mod.rs` as part of the
//! viewer module split (refactor #1 part B).
//!
//! Holds the active sketch, drawing tool, undo/redo snapshots, selection,
//! drag state, and validation results for the Sketcher workbench.
//! Pure relocation — no behaviour changes.

use cadkernel_sketch::{Constraint, Sketch, WorkPlane};

/// Degrees of freedom of a `Sketch`: `2 * points − Σ constraint_dofs`. This
/// is the formula the Sketcher status bar (`gui/status_bar.rs`) reads to
/// drive the "Fully constrained / Under-constrained / N DOF" readout.
///
/// Exposed here (instead of as a method on the `pub(crate)` `SketchMode`) so
/// integration tests can lock the readout in without poking at viewer
/// internals.
pub fn sketch_degrees_of_freedom(sketch: &Sketch) -> i32 {
    let pt_dofs = (sketch.points.len() * 2) as i32;
    let mut c_dofs = 0i32;
    for c in &sketch.constraints {
        c_dofs += match c {
            Constraint::Coincident(..) => 2,
            Constraint::Horizontal(_) | Constraint::Vertical(_) => 1,
            Constraint::Parallel(..) | Constraint::Perpendicular(..) => 1,
            Constraint::PointOnLine(..) => 1,
            Constraint::PointOnCircle(..) => 1,
            Constraint::Symmetric(..) => 2,
            Constraint::Distance(..) => 1,
            Constraint::Angle(..) => 1,
            Constraint::Radius(..) => 1,
            Constraint::Length(..) => 1,
            Constraint::Fixed(..) => 2,
            Constraint::Tangent(..) => 1,
            Constraint::EqualLength(..) => 1,
            Constraint::Midpoint(..) => 2,
            Constraint::Collinear(..) => 2,
            Constraint::EqualRadius(..) => 1,
            Constraint::Concentric(..) => 2,
            Constraint::Diameter(..) => 1,
            Constraint::Block(..) => 2,
            Constraint::HorizontalDistance(..) => 1,
            Constraint::VerticalDistance(..) => 1,
            Constraint::PointOnObject(..) => 1,
            Constraint::Refraction { .. } => 1,
        };
    }
    pt_dofs - c_dofs
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SketchTool {
    Select,
    Line,
    Rectangle,
    Circle,
    Arc,
    Point,
    Ellipse,
    Polyline,
    Slot,
    BSpline,
    Polygon { sides: u32 },
}

/// Kind of dimension constraint being edited in the popup.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum DimensionKind {
    Distance,
    Radius,
    Angle,
    Length,
    HDistance,
    VDistance,
    Diameter,
}

/// State for the dimension input popup shown after clicking a dimensional constraint button.
pub(crate) struct DimensionPopup {
    pub kind: DimensionKind,
    pub value: f64,
    pub just_opened: bool,
    /// If editing an existing constraint, its index.
    pub edit_constraint_index: Option<usize>,
}

/// Reference to a specific sketch entity for selection/editing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SketchEntityRef {
    Point(usize),
    Line(usize),
    Arc(usize),
    Circle(usize),
    Ellipse(usize),
    BSpline(usize),
}

/// Full sketch state snapshot for undo/redo.
#[derive(Clone)]
pub(crate) struct SketchSnapshot {
    pub sketch: Sketch,
}

pub(crate) struct SketchMode {
    pub sketch: Sketch,
    pub plane: WorkPlane,
    pub tool: SketchTool,
    pub pending_point: Option<(f64, f64)>,
    pub extrude_distance: f64,
    pub construction_mode: bool,
    pub show_grid: bool,
    pub snap_enabled: bool,
    pub show_constraints: bool,
    pub polyline_points: Vec<(f64, f64)>,
    pub undo_stack: Vec<SketchSnapshot>,
    pub redo_stack: Vec<SketchSnapshot>,
    pub selected_entities: Vec<SketchEntityRef>,
    pub hovered_entity: Option<SketchEntityRef>,
    pub drag_points: Vec<usize>,
    pub drag_origin: Option<(f64, f64)>,
    pub drag_started: bool,
    /// Box selection: start screen coordinates
    pub box_select_start: Option<(f64, f64)>,
    /// Box selection: current screen coordinates
    pub box_select_end: Option<(f64, f64)>,
    pub show_context_menu: bool,
    pub constraint_residuals: Vec<f64>,
    pub solver_converged: bool,
    pub grid_spacing: f64,
    /// Clipboard: copied points as (dx, dy) offsets relative to centroid.
    pub clipboard_points: Vec<(f64, f64)>,
    /// Clipboard: copied lines as (start_idx, end_idx) into clipboard_points.
    pub clipboard_lines: Vec<(usize, usize)>,
    /// Validation issues from last check.
    pub validation_issues: Vec<cadkernel_sketch::SketchValidationIssue>,
    /// Count of actionable constraint diagnostics beyond normal under-constrained DOF.
    pub constraint_warning_count: usize,
    /// Compact English constraint diagnostic status for banners/status bars.
    pub constraint_status: String,
    /// Count of projected external reference entities in the active sketch.
    pub external_reference_count: usize,
    /// Count of entities imported through sketch reuse / carbon copy.
    pub reused_geometry_count: usize,
    /// Compact English status for external references and sketch reuse.
    pub reference_status: String,
    /// True when regular sketch lines form exactly one closed feature profile.
    pub profile_ready: bool,
    /// Number of closed regular-line profile loops found.
    pub profile_loop_count: usize,
    /// Number of endpoints left open in regular profile chains.
    pub profile_open_endpoint_count: usize,
    /// Number of branch points where more than two regular profile lines meet.
    pub profile_branch_point_count: usize,
    /// Compact English status for the on-view Sketcher banner.
    pub profile_status: String,
}

impl SketchMode {
    pub fn new(plane: WorkPlane) -> Self {
        Self {
            sketch: Sketch::new(),
            plane,
            tool: SketchTool::Line,
            pending_point: None,
            extrude_distance: 10.0,
            construction_mode: false,
            show_grid: true,
            snap_enabled: true,
            show_constraints: true,
            polyline_points: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            selected_entities: Vec::new(),
            hovered_entity: None,
            drag_points: Vec::new(),
            drag_origin: None,
            drag_started: false,
            box_select_start: None,
            box_select_end: None,
            show_context_menu: false,
            constraint_residuals: Vec::new(),
            solver_converged: true,
            grid_spacing: 1.0,
            clipboard_points: Vec::new(),
            clipboard_lines: Vec::new(),
            validation_issues: Vec::new(),
            constraint_warning_count: 0,
            constraint_status: "Constraints: healthy".into(),
            external_reference_count: 0,
            reused_geometry_count: 0,
            reference_status: "Refs: none".into(),
            profile_ready: false,
            profile_loop_count: 0,
            profile_open_endpoint_count: 0,
            profile_branch_point_count: 0,
            profile_status: "Profile: no regular lines".into(),
        }
    }

    /// Save a full clone of the current sketch state before a sketch operation.
    pub fn save_snapshot(&mut self) {
        self.undo_stack.push(SketchSnapshot {
            sketch: self.sketch.clone(),
        });
        self.redo_stack.clear();
    }

    /// Undo: restore the previous sketch state.
    pub fn undo(&mut self) -> bool {
        if let Some(snap) = self.undo_stack.pop() {
            self.redo_stack.push(SketchSnapshot {
                sketch: self.sketch.clone(),
            });
            self.sketch = snap.sketch;
            self.pending_point = None;
            self.selected_entities.clear();
            true
        } else {
            false
        }
    }

    /// Redo: restore the next sketch state.
    pub fn redo(&mut self) -> bool {
        if let Some(snap) = self.redo_stack.pop() {
            self.undo_stack.push(SketchSnapshot {
                sketch: self.sketch.clone(),
            });
            self.sketch = snap.sketch;
            self.selected_entities.clear();
            true
        } else {
            false
        }
    }

    /// Compute degrees of freedom: 2 * points - constraint_dofs.
    pub fn degrees_of_freedom(&self) -> i32 {
        sketch_degrees_of_freedom(&self.sketch)
    }

    /// Recompute per-constraint residuals, solver status, and validation issues.
    pub fn update_constraint_status(&mut self) {
        if self.sketch.constraints.is_empty() {
            self.constraint_residuals.clear();
            self.solver_converged = true;
        } else {
            self.constraint_residuals = cadkernel_sketch::constraint_residuals(&self.sketch);
            self.solver_converged = self.constraint_residuals.iter().all(|r| *r < 1e-6);
        }
        let validation = cadkernel_sketch::validate_sketch(&self.sketch, 0.01);
        self.constraint_warning_count = validation.diagnostic_issue_count();
        self.constraint_status = validation.status_label();
        self.validation_issues = validation.issues;
        let construction_reference_count =
            self.sketch.construction_points.len() + self.sketch.construction_lines.len();
        self.external_reference_count = self
            .sketch
            .external_reference_count()
            .max(construction_reference_count);
        self.reference_status = match (self.external_reference_count, self.reused_geometry_count) {
            (0, 0) => "Refs: none".into(),
            (external, 0) => format!("Refs: {external} external"),
            (0, reused) => format!("Reuse: {reused} copied"),
            (external, reused) => format!("Refs: {external} external, {reused} copied"),
        };
        let profile = cadkernel_sketch::analyze_profiles(&self.sketch);
        self.profile_ready = profile.is_single_closed_profile();
        self.profile_loop_count = profile.loops.len();
        self.profile_open_endpoint_count = profile.open_endpoints.len();
        self.profile_branch_point_count = profile.branch_points.len();
        self.profile_status = profile.status_label();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sketch_mode_profile_status_detects_open_and_closed_profiles() {
        let mut sm = SketchMode::new(WorkPlane::xy());
        let p0 = sm.sketch.add_point(0.0, 0.0);
        let p1 = sm.sketch.add_point(1.0, 0.0);
        let p2 = sm.sketch.add_point(1.0, 1.0);
        let p3 = sm.sketch.add_point(0.0, 1.0);
        sm.sketch.add_line(p0, p1);
        sm.sketch.add_line(p1, p2);
        sm.sketch.add_line(p2, p3);
        sm.update_constraint_status();
        assert!(!sm.profile_ready);
        assert_eq!(sm.profile_open_endpoint_count, 2);
        assert!(sm.profile_status.contains("open"));

        sm.sketch.add_line(p3, p0);
        sm.update_constraint_status();
        assert!(sm.profile_ready);
        assert_eq!(sm.profile_loop_count, 1);
        assert!(sm.profile_status.contains("ready"));
    }

    #[test]
    fn sketch_mode_constraint_status_reports_duplicate_constraints() {
        let mut sm = SketchMode::new(WorkPlane::xy());
        let p0 = sm.sketch.add_point(0.0, 0.0);
        let p1 = sm.sketch.add_point(1.0, 0.0);
        let line = sm.sketch.add_line(p0, p1);
        sm.sketch.add_constraint(Constraint::Horizontal(line));
        sm.sketch.add_constraint(Constraint::Horizontal(line));

        sm.update_constraint_status();

        assert_eq!(sm.constraint_warning_count, 1);
        assert!(sm.constraint_status.contains("duplicate"));
    }

    #[test]
    fn sketch_mode_reference_status_counts_construction_refs() {
        let mut sm = SketchMode::new(WorkPlane::xy());
        let p0 = sm.sketch.add_point(0.0, 0.0);
        let p1 = sm.sketch.add_point(1.0, 0.0);
        let line = sm.sketch.add_line(p0, p1);
        sm.sketch.mark_construction_point(p0);
        sm.sketch.mark_construction_point(p1);
        sm.sketch.mark_construction_line(line);

        sm.update_constraint_status();

        assert_eq!(sm.external_reference_count, 3);
        assert!(sm.reference_status.contains("external"));
    }
}

/// Actions specific to the Sketcher workbench, dispatched through
/// `GuiAction::Sketcher(SketcherAction)`. Workbench prefix dropped where the
/// wrapper variant already provides scope.
///
/// Note: `PartialEq` is omitted because `WorkPlane` (in `Enter(_)`) does not
/// implement it. The parent `GuiAction` enum has no derive either, so this
/// diverges intentionally from `AssemblyAction` / `FemAction`.
#[derive(Clone, Debug)]
pub(crate) enum SketcherAction {
    // Lifecycle
    Enter(WorkPlane),
    EnterOnSelectedFace,
    Edit,
    Close,
    Cancel,

    // Pointer / tool
    Click(f64, f64),
    SetTool(SketchTool),

    // Geometric constraints
    ConstrainHorizontal,
    ConstrainVertical,
    ConstrainLength(f64),
    ConstrainParallel,
    ConstrainPerpendicular,
    ConstrainCoincident,
    ConstrainTangent,
    ConstrainEqual,
    ConstrainSymmetric,
    ConstrainFixed,
    ConstrainBlock,
    ConstrainDistance(f64),
    ConstrainAngle(f64),
    ConstrainRadius(f64),
    ConstrainDiameter(f64),
    ConstrainHDistance(f64),
    ConstrainVDistance(f64),

    // Tools
    FilletCorner { radius: f64 },
    ChamferCorner { distance: f64 },
    TrimEdge,
    SplitEdge,
    ExtendEdge,
    MirrorGeometry,
    ExternalProjection,
    SelectReferences,
    PromoteReferences,
    CarbonCopy,
    CopySelection,
    PasteSelection(f64, f64),
    MergePoints,

    // B-spline
    ConvertToBSpline,
    IncreaseDegree,
    DecreaseDegree,
    InsertKnot,

    // Toggles
    ToggleConstruction,
    ToggleGrid,
    ToggleSnap,
    ToggleConstraintsVisible,
}
