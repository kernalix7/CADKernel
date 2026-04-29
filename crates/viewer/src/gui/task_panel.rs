//! Task panel — contextual side panel for creation/modification operations.
//!
//! Replaces popup dialogs with an inline panel that shows parameters
//! and updates the 3D preview in real-time. Mirrors FreeCAD's TaskView.

use super::{GuiAction, GuiState, MirrorPlane};
use super::theme;

/// Active task state — one variant per operation type.
#[derive(Clone, Debug)]
pub(crate) enum ActiveTask {
    // -- Primitives --
    Box { width: f64, height: f64, depth: f64, preview_id: Option<crate::scene::ObjectId> },
    Cylinder { radius: f64, height: f64, preview_id: Option<crate::scene::ObjectId> },
    Sphere { radius: f64, preview_id: Option<crate::scene::ObjectId> },
    Cone { base_radius: f64, top_radius: f64, height: f64, preview_id: Option<crate::scene::ObjectId> },
    Torus { major_radius: f64, minor_radius: f64, preview_id: Option<crate::scene::ObjectId> },
    Tube { outer_radius: f64, inner_radius: f64, height: f64, preview_id: Option<crate::scene::ObjectId> },
    Prism { radius: f64, height: f64, sides: usize, preview_id: Option<crate::scene::ObjectId> },
    Wedge { dx: f64, dy: f64, dz: f64, dx2: f64, dy2: f64, preview_id: Option<crate::scene::ObjectId> },
    Ellipsoid { rx: f64, ry: f64, rz: f64, preview_id: Option<crate::scene::ObjectId> },
    Helix { radius: f64, pitch: f64, turns: f64, tube_radius: f64, preview_id: Option<crate::scene::ObjectId> },
    // -- PartDesign features --
    Pad { depth: f64, symmetric: bool, preview_id: Option<crate::scene::ObjectId> },
    Pocket { depth: f64, through_all: bool, preview_id: Option<crate::scene::ObjectId> },
    Hole { radius: f64, depth: f64, countersink: bool, countersink_angle: f64, preview_id: Option<crate::scene::ObjectId> },
    Groove { angle: f64, preview_id: Option<crate::scene::ObjectId> },
    Fillet { radius: f64, preview_id: Option<crate::scene::ObjectId> },
    Chamfer { distance: f64, preview_id: Option<crate::scene::ObjectId> },
    Shell { thickness: f64, preview_id: Option<crate::scene::ObjectId> },
    MirrorOp { plane: u8, preview_id: Option<crate::scene::ObjectId> },
    Pattern { count: usize, spacing: f64, axis: u8, preview_id: Option<crate::scene::ObjectId> },
    Sprocket { teeth: u32, roller_diameter: f64, pitch: f64, bore: f64, preview_id: Option<crate::scene::ObjectId> },
    InvoluteGear { teeth: u32, module_val: f64, pressure_angle: f64, preview_id: Option<crate::scene::ObjectId> },
    // -- Draft --
    DraftLine { length: f64, angle: f64, preview_id: Option<crate::scene::ObjectId> },
    DraftCircle { radius: f64, preview_id: Option<crate::scene::ObjectId> },
    DraftRectangle { width: f64, height: f64, preview_id: Option<crate::scene::ObjectId> },
    DraftPolygon { radius: f64, sides: usize, preview_id: Option<crate::scene::ObjectId> },
    DraftArc { radius: f64, start_angle: f64, end_angle: f64, preview_id: Option<crate::scene::ObjectId> },
    DraftEllipse { rx: f64, ry: f64, preview_id: Option<crate::scene::ObjectId> },
    // -- Surface --
    SurfacePipe { radius: f64, length: f64, preview_id: Option<crate::scene::ObjectId> },
    SurfaceRuled { width: f64, depth: f64, offset: f64, preview_id: Option<crate::scene::ObjectId> },
    // -- FEM --
    FemMesh { element_size: f64, preview_id: Option<crate::scene::ObjectId> },
    // -- Boolean --
    BooleanOp { op_type: u8, width: f64, height: f64, depth: f64, offset_x: f64, offset_y: f64, offset_z: f64, preview_id: Option<crate::scene::ObjectId> },
    // -- Scale --
    ScaleOp { factor: f64, preview_id: Option<crate::scene::ObjectId> },
}

impl ActiveTask {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Box { .. } => "Create Box",
            Self::Cylinder { .. } => "Create Cylinder",
            Self::Sphere { .. } => "Create Sphere",
            Self::Cone { .. } => "Create Cone",
            Self::Torus { .. } => "Create Torus",
            Self::Tube { .. } => "Create Tube",
            Self::Prism { .. } => "Create Prism",
            Self::Wedge { .. } => "Create Wedge",
            Self::Ellipsoid { .. } => "Create Ellipsoid",
            Self::Helix { .. } => "Create Helix",
            Self::Pad { .. } => "Pad Sketch",
            Self::Pocket { .. } => "Pocket Sketch",
            Self::Hole { .. } => "Create Hole",
            Self::Groove { .. } => "Groove",
            Self::Fillet { .. } => "Fillet Edges",
            Self::Chamfer { .. } => "Chamfer Edges",
            Self::Shell { .. } => "Shell Solid",
            Self::MirrorOp { .. } => "Mirror Solid",
            Self::Pattern { .. } => "Linear Pattern",
            Self::Sprocket { .. } => "Create Sprocket",
            Self::InvoluteGear { .. } => "Involute Gear",
            Self::DraftLine { .. } => "Draft Line",
            Self::DraftCircle { .. } => "Draft Circle",
            Self::DraftRectangle { .. } => "Draft Rectangle",
            Self::DraftPolygon { .. } => "Draft Polygon",
            Self::DraftArc { .. } => "Draft Arc",
            Self::DraftEllipse { .. } => "Draft Ellipse",
            Self::SurfacePipe { .. } => "Surface Pipe",
            Self::SurfaceRuled { .. } => "Ruled Surface",
            Self::FemMesh { .. } => "FEM Tet Mesh",
            Self::BooleanOp { op_type, .. } => match op_type {
                0 => "Boolean Union",
                1 => "Boolean Subtract",
                _ => "Boolean Intersect",
            },
            Self::ScaleOp { .. } => "Scale Solid",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Box { .. } => "\u{25A3}",
            Self::Cylinder { .. } => "\u{25CD}",
            Self::Sphere { .. } => "\u{25CF}",
            Self::Cone { .. } => "\u{25B2}",
            Self::Torus { .. } => "\u{25CE}",
            Self::Tube { .. } => "\u{25C9}",
            Self::Prism { .. } => "\u{2B23}",
            Self::Wedge { .. } => "\u{25C7}",
            Self::Ellipsoid { .. } => "\u{2B2D}",
            Self::Helix { .. } => "\u{223F}",
            Self::Pad { .. } | Self::Pocket { .. } | Self::Groove { .. } => "\u{2B06}",
            Self::Hole { .. } => "\u{25CB}",
            Self::Fillet { .. } => "\u{25D5}",
            Self::Chamfer { .. } => "\u{25C8}",
            Self::Shell { .. } => "\u{25A2}",
            Self::MirrorOp { .. } => "\u{21C6}",
            Self::Pattern { .. } => "\u{2237}",
            Self::Sprocket { .. } | Self::InvoluteGear { .. } => "\u{2699}",
            Self::DraftLine { .. } => "\u{2571}",
            Self::DraftCircle { .. } => "\u{25CB}",
            Self::DraftRectangle { .. } => "\u{25A1}",
            Self::DraftPolygon { .. } => "\u{2B23}",
            Self::DraftArc { .. } => "\u{25DC}",
            Self::DraftEllipse { .. } => "\u{2B2D}",
            Self::SurfacePipe { .. } => "\u{2234}",
            Self::SurfaceRuled { .. } => "\u{2225}",
            Self::FemMesh { .. } => "\u{2206}",
            Self::BooleanOp { .. } => "\u{222A}",
            Self::ScaleOp { .. } => "\u{2922}",
        }
    }

    pub fn preview_id(&self) -> Option<crate::scene::ObjectId> {
        match self {
            Self::Box { preview_id, .. }
            | Self::Cylinder { preview_id, .. }
            | Self::Sphere { preview_id, .. }
            | Self::Cone { preview_id, .. }
            | Self::Torus { preview_id, .. }
            | Self::Tube { preview_id, .. }
            | Self::Prism { preview_id, .. }
            | Self::Wedge { preview_id, .. }
            | Self::Ellipsoid { preview_id, .. }
            | Self::Helix { preview_id, .. }
            | Self::Pad { preview_id, .. }
            | Self::Pocket { preview_id, .. }
            | Self::Hole { preview_id, .. }
            | Self::Groove { preview_id, .. }
            | Self::Fillet { preview_id, .. }
            | Self::Chamfer { preview_id, .. }
            | Self::Shell { preview_id, .. }
            | Self::MirrorOp { preview_id, .. }
            | Self::Pattern { preview_id, .. }
            | Self::Sprocket { preview_id, .. }
            | Self::InvoluteGear { preview_id, .. }
            | Self::DraftLine { preview_id, .. }
            | Self::DraftCircle { preview_id, .. }
            | Self::DraftRectangle { preview_id, .. }
            | Self::DraftPolygon { preview_id, .. }
            | Self::DraftArc { preview_id, .. }
            | Self::DraftEllipse { preview_id, .. }
            | Self::SurfacePipe { preview_id, .. }
            | Self::SurfaceRuled { preview_id, .. }
            | Self::FemMesh { preview_id, .. }
            | Self::BooleanOp { preview_id, .. }
            | Self::ScaleOp { preview_id, .. } => *preview_id,
        }
    }

    pub fn set_preview_id(&mut self, id: crate::scene::ObjectId) {
        match self {
            Self::Box { preview_id, .. }
            | Self::Cylinder { preview_id, .. }
            | Self::Sphere { preview_id, .. }
            | Self::Cone { preview_id, .. }
            | Self::Torus { preview_id, .. }
            | Self::Tube { preview_id, .. }
            | Self::Prism { preview_id, .. }
            | Self::Wedge { preview_id, .. }
            | Self::Ellipsoid { preview_id, .. }
            | Self::Helix { preview_id, .. }
            | Self::Pad { preview_id, .. }
            | Self::Pocket { preview_id, .. }
            | Self::Hole { preview_id, .. }
            | Self::Groove { preview_id, .. }
            | Self::Fillet { preview_id, .. }
            | Self::Chamfer { preview_id, .. }
            | Self::Shell { preview_id, .. }
            | Self::MirrorOp { preview_id, .. }
            | Self::Pattern { preview_id, .. }
            | Self::Sprocket { preview_id, .. }
            | Self::InvoluteGear { preview_id, .. }
            | Self::DraftLine { preview_id, .. }
            | Self::DraftCircle { preview_id, .. }
            | Self::DraftRectangle { preview_id, .. }
            | Self::DraftPolygon { preview_id, .. }
            | Self::DraftArc { preview_id, .. }
            | Self::DraftEllipse { preview_id, .. }
            | Self::SurfacePipe { preview_id, .. }
            | Self::SurfaceRuled { preview_id, .. }
            | Self::FemMesh { preview_id, .. }
            | Self::BooleanOp { preview_id, .. }
            | Self::ScaleOp { preview_id, .. } => *preview_id = Some(id),
        }
    }
}

/// Inline version — draws task panel into existing Ui.
/// Returns true if a task is active.
pub(crate) fn draw_task_panel_inline(
    ui: &mut egui::Ui,
    gui: &mut GuiState,
) -> bool {
    let Some(task) = &gui.active_task else {
        return false;
    };
    let title = task.title().to_string();
    let icon = task.icon().to_string();
    let mut task = task.clone();
    let commit;
    let cancel;
    let mut changed = false;

    // Helper: styled label for grid rows
    macro_rules! plabel {
        ($ui:expr, $t:expr) => {
            $ui.label(egui::RichText::new($t).size(11.5).color(egui::Color32::from_gray(180)));
        };
    }
    // Helper: drag value with mm suffix
    macro_rules! pmm {
        ($ui:expr, $val:expr, $r:expr, $sp:expr, $ch:expr) => {{
            let r = $ui.add(egui::DragValue::new($val).range($r).speed($sp).suffix(" mm"));
            $ch |= r.changed();
        }};
    }
    // Helper: drag value with ° suffix
    macro_rules! pdeg {
        ($ui:expr, $val:expr, $r:expr, $sp:expr, $ch:expr) => {{
            let r = $ui.add(egui::DragValue::new($val).range($r).speed($sp).suffix("\u{00B0}"));
            $ch |= r.changed();
        }};
    }
    // Helper: plain drag value (no suffix)
    macro_rules! pval {
        ($ui:expr, $val:expr, $r:expr, $sp:expr, $ch:expr) => {{
            let r = $ui.add(egui::DragValue::new($val).range($r).speed($sp));
            $ch |= r.changed();
        }};
    }

    // Grid spacing for all task grids
    let grid_sp = [10.0, 4.0];

    {
            theme::draw_task_header(ui, &icon, &title);

            match &mut task {
                ActiveTask::Box { width, height, depth, .. } => {
                    theme::draw_task_section(ui, "Dimensions");
                    egui::Grid::new("task_box").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "Width"); pmm!(ui, width, 0.1..=1000.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "Height"); pmm!(ui, height, 0.1..=1000.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "Depth"); pmm!(ui, depth, 0.1..=1000.0, 0.5, changed); ui.end_row();
                    });
                }
                ActiveTask::Cylinder { radius, height, .. } => {
                    theme::draw_task_section(ui, "Parameters");
                    egui::Grid::new("task_cyl").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "Radius"); pmm!(ui, radius, 0.1..=500.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "Height"); pmm!(ui, height, 0.1..=1000.0, 0.5, changed); ui.end_row();
                    });
                }
                ActiveTask::Sphere { radius, .. } => {
                    theme::draw_task_section(ui, "Parameters");
                    egui::Grid::new("task_sph").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "Radius"); pmm!(ui, radius, 0.1..=500.0, 0.5, changed); ui.end_row();
                    });
                }
                ActiveTask::Cone { base_radius, top_radius, height, .. } => {
                    theme::draw_task_section(ui, "Parameters");
                    egui::Grid::new("task_cone").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "Base Radius"); pmm!(ui, base_radius, 0.01..=500.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "Top Radius"); pmm!(ui, top_radius, 0.0..=500.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "Height"); pmm!(ui, height, 0.1..=1000.0, 0.5, changed); ui.end_row();
                    });
                }
                ActiveTask::Torus { major_radius, minor_radius, .. } => {
                    theme::draw_task_section(ui, "Parameters");
                    egui::Grid::new("task_tor").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "Major Radius"); pmm!(ui, major_radius, 0.1..=500.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "Minor Radius"); pmm!(ui, minor_radius, 0.01..=200.0, 0.1, changed); ui.end_row();
                    });
                }
                ActiveTask::Tube { outer_radius, inner_radius, height, .. } => {
                    theme::draw_task_section(ui, "Parameters");
                    egui::Grid::new("task_tube").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "Outer Radius"); pmm!(ui, outer_radius, 0.1..=500.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "Inner Radius"); pmm!(ui, inner_radius, 0.01..=499.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "Height"); pmm!(ui, height, 0.1..=1000.0, 0.5, changed); ui.end_row();
                    });
                }
                ActiveTask::Prism { radius, height, sides, .. } => {
                    theme::draw_task_section(ui, "Parameters");
                    egui::Grid::new("task_prism").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "Radius"); pmm!(ui, radius, 0.1..=500.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "Height"); pmm!(ui, height, 0.1..=1000.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "Sides");
                        let mut sides_f = *sides as f64;
                        if ui.add(egui::DragValue::new(&mut sides_f).range(3.0..=100.0).speed(0.2)).changed() {
                            *sides = sides_f as usize;
                            changed = true;
                        }
                        ui.end_row();
                    });
                }
                ActiveTask::Wedge { dx, dy, dz, dx2, dy2, .. } => {
                    theme::draw_task_section(ui, "Base Dimensions");
                    egui::Grid::new("task_wedge").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "DX"); pmm!(ui, dx, 0.1..=1000.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "DY"); pmm!(ui, dy, 0.1..=1000.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "DZ"); pmm!(ui, dz, 0.1..=1000.0, 0.5, changed); ui.end_row();
                    });
                    theme::draw_task_section(ui, "Top Dimensions");
                    egui::Grid::new("task_wedge2").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "DX2"); pmm!(ui, dx2, 0.0..=1000.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "DY2"); pmm!(ui, dy2, 0.0..=1000.0, 0.5, changed); ui.end_row();
                    });
                }
                ActiveTask::Ellipsoid { rx, ry, rz, .. } => {
                    theme::draw_task_section(ui, "Radii");
                    egui::Grid::new("task_ellip").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "RX"); pmm!(ui, rx, 0.1..=500.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "RY"); pmm!(ui, ry, 0.1..=500.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "RZ"); pmm!(ui, rz, 0.1..=500.0, 0.5, changed); ui.end_row();
                    });
                }
                ActiveTask::Helix { radius, pitch, turns, tube_radius, .. } => {
                    theme::draw_task_section(ui, "Helix Parameters");
                    egui::Grid::new("task_helix").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "Radius"); pmm!(ui, radius, 0.1..=500.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "Pitch"); pmm!(ui, pitch, 0.1..=200.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "Turns"); pval!(ui, turns, 0.5..=50.0, 0.1, changed); ui.end_row();
                        plabel!(ui, "Tube Radius"); pmm!(ui, tube_radius, 0.01..=50.0, 0.1, changed); ui.end_row();
                    });
                }
                ActiveTask::Pad { depth, symmetric, .. } => {
                    theme::draw_task_section(ui, "Extrusion");
                    egui::Grid::new("task_pad").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "Depth"); pmm!(ui, depth, 0.1..=1000.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "Symmetric"); changed |= ui.checkbox(symmetric, "").changed(); ui.end_row();
                    });
                }
                ActiveTask::Pocket { depth, through_all, .. } => {
                    theme::draw_task_section(ui, "Pocket");
                    egui::Grid::new("task_pocket").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "Depth"); pmm!(ui, depth, 0.1..=1000.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "Through All"); changed |= ui.checkbox(through_all, "").changed(); ui.end_row();
                    });
                }
                ActiveTask::Hole { radius, depth, countersink, countersink_angle, .. } => {
                    theme::draw_task_section(ui, "Hole Parameters");
                    egui::Grid::new("task_hole").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "Radius"); pmm!(ui, radius, 0.01..=200.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "Depth"); pmm!(ui, depth, 0.1..=1000.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "Countersink"); changed |= ui.checkbox(countersink, "").changed(); ui.end_row();
                        if *countersink {
                            plabel!(ui, "CS Angle"); pdeg!(ui, countersink_angle, 0.0..=180.0, 0.5, changed); ui.end_row();
                        }
                    });
                }
                ActiveTask::Groove { angle, .. } => {
                    theme::draw_task_section(ui, "Revolve");
                    egui::Grid::new("task_groove").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "Angle"); pdeg!(ui, angle, 1.0..=360.0, 1.0, changed); ui.end_row();
                    });
                }
                ActiveTask::Fillet { radius, .. } => {
                    let edge_count = gui.selected_entities.iter()
                        .filter(|e| matches!(e, super::SelectedEntity::Edge(_)))
                        .count();
                    theme::draw_task_section(ui, "Fillet");
                    egui::Grid::new("task_fillet").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        if edge_count > 0 {
                            plabel!(ui, "Edges");
                            ui.label(egui::RichText::new(format!("{edge_count} selected")).size(11.0).color(theme::COLOR_INFO));
                            ui.end_row();
                        }
                        plabel!(ui, "Radius"); pmm!(ui, radius, 0.01..=100.0, 0.1, changed); ui.end_row();
                    });
                }
                ActiveTask::Chamfer { distance, .. } => {
                    let edge_count = gui.selected_entities.iter()
                        .filter(|e| matches!(e, super::SelectedEntity::Edge(_)))
                        .count();
                    theme::draw_task_section(ui, "Chamfer");
                    egui::Grid::new("task_chamfer").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        if edge_count > 0 {
                            plabel!(ui, "Edges");
                            ui.label(egui::RichText::new(format!("{edge_count} selected")).size(11.0).color(theme::COLOR_INFO));
                            ui.end_row();
                        }
                        plabel!(ui, "Distance"); pmm!(ui, distance, 0.01..=100.0, 0.1, changed); ui.end_row();
                    });
                }
                ActiveTask::Shell { thickness, .. } => {
                    theme::draw_task_section(ui, "Shell");
                    egui::Grid::new("task_shell").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "Thickness"); pmm!(ui, thickness, 0.01..=50.0, 0.1, changed); ui.end_row();
                    });
                }
                ActiveTask::MirrorOp { plane, .. } => {
                    theme::draw_task_section(ui, "Mirror");
                    egui::Grid::new("task_mirror").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "Plane");
                        let labels = ["XY", "XZ", "YZ"];
                        let mut plane_f = *plane as f64;
                        if ui.add(egui::DragValue::new(&mut plane_f).range(0.0..=2.0).speed(0.1).custom_formatter(|v, _| {
                            labels[v as usize % 3].to_string()
                        })).changed() {
                            *plane = plane_f as u8;
                            changed = true;
                        }
                        ui.end_row();
                    });
                }
                ActiveTask::Pattern { count, spacing, axis, .. } => {
                    theme::draw_task_section(ui, "Pattern");
                    egui::Grid::new("task_pattern").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "Count");
                        let mut count_f = *count as f64;
                        if ui.add(egui::DragValue::new(&mut count_f).range(2.0..=100.0).speed(0.2)).changed() {
                            *count = count_f as usize;
                            changed = true;
                        }
                        ui.end_row();
                        plabel!(ui, "Spacing"); pmm!(ui, spacing, 0.1..=500.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "Axis");
                        let labels = ["X", "Y", "Z"];
                        let mut axis_f = *axis as f64;
                        if ui.add(egui::DragValue::new(&mut axis_f).range(0.0..=2.0).speed(0.1).custom_formatter(|v, _| {
                            labels[v as usize % 3].to_string()
                        })).changed() {
                            *axis = axis_f as u8;
                            changed = true;
                        }
                        ui.end_row();
                    });
                }
                ActiveTask::Sprocket { teeth, roller_diameter, pitch, bore, .. } => {
                    theme::draw_task_section(ui, "Sprocket Parameters");
                    egui::Grid::new("task_sprocket").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "Teeth");
                        let mut teeth_f = *teeth as f64;
                        if ui.add(egui::DragValue::new(&mut teeth_f).range(4.0..=200.0).speed(0.5)).changed() {
                            *teeth = teeth_f as u32;
                            changed = true;
                        }
                        ui.end_row();
                        plabel!(ui, "Roller Dia"); pmm!(ui, roller_diameter, 0.1..=50.0, 0.1, changed); ui.end_row();
                        plabel!(ui, "Pitch"); pmm!(ui, pitch, 0.1..=100.0, 0.1, changed); ui.end_row();
                        plabel!(ui, "Bore"); pmm!(ui, bore, 0.1..=100.0, 0.1, changed); ui.end_row();
                    });
                }
                ActiveTask::InvoluteGear { teeth, module_val, pressure_angle, .. } => {
                    theme::draw_task_section(ui, "Gear Parameters");
                    egui::Grid::new("task_gear").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "Teeth");
                        let mut teeth_f = *teeth as f64;
                        if ui.add(egui::DragValue::new(&mut teeth_f).range(6.0..=200.0).speed(0.5)).changed() {
                            *teeth = teeth_f as u32;
                            changed = true;
                        }
                        ui.end_row();
                        plabel!(ui, "Module"); pval!(ui, module_val, 0.1..=50.0, 0.1, changed); ui.end_row();
                        plabel!(ui, "Pressure Angle"); pdeg!(ui, pressure_angle, 10.0..=30.0, 0.5, changed); ui.end_row();
                    });
                }
                ActiveTask::DraftLine { length, angle, .. } => {
                    theme::draw_task_section(ui, "Line");
                    egui::Grid::new("task_dline").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "Length"); pmm!(ui, length, 0.1..=1000.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "Angle"); pdeg!(ui, angle, 0.0..=360.0, 1.0, changed); ui.end_row();
                    });
                }
                ActiveTask::DraftCircle { radius, .. } => {
                    theme::draw_task_section(ui, "Circle");
                    egui::Grid::new("task_dcirc").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "Radius"); pmm!(ui, radius, 0.1..=500.0, 0.5, changed); ui.end_row();
                    });
                }
                ActiveTask::DraftRectangle { width, height, .. } => {
                    theme::draw_task_section(ui, "Rectangle");
                    egui::Grid::new("task_drect").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "Width"); pmm!(ui, width, 0.1..=1000.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "Height"); pmm!(ui, height, 0.1..=1000.0, 0.5, changed); ui.end_row();
                    });
                }
                ActiveTask::DraftPolygon { radius, sides, .. } => {
                    theme::draw_task_section(ui, "Polygon");
                    egui::Grid::new("task_dpoly").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "Radius"); pmm!(ui, radius, 0.1..=500.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "Sides");
                        let mut sides_f = *sides as f64;
                        if ui.add(egui::DragValue::new(&mut sides_f).range(3.0..=100.0).speed(0.2)).changed() {
                            *sides = sides_f as usize;
                            changed = true;
                        }
                        ui.end_row();
                    });
                }
                ActiveTask::DraftArc { radius, start_angle, end_angle, .. } => {
                    theme::draw_task_section(ui, "Arc");
                    egui::Grid::new("task_darc").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "Radius"); pmm!(ui, radius, 0.1..=500.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "Start"); pdeg!(ui, start_angle, 0.0..=360.0, 1.0, changed); ui.end_row();
                        plabel!(ui, "End"); pdeg!(ui, end_angle, 0.0..=360.0, 1.0, changed); ui.end_row();
                    });
                }
                ActiveTask::DraftEllipse { rx, ry, .. } => {
                    theme::draw_task_section(ui, "Ellipse");
                    egui::Grid::new("task_dellip").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "RX"); pmm!(ui, rx, 0.1..=500.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "RY"); pmm!(ui, ry, 0.1..=500.0, 0.5, changed); ui.end_row();
                    });
                }
                ActiveTask::SurfacePipe { radius, length, .. } => {
                    theme::draw_task_section(ui, "Pipe Surface");
                    egui::Grid::new("task_spipe").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "Radius"); pmm!(ui, radius, 0.1..=100.0, 0.1, changed); ui.end_row();
                        plabel!(ui, "Length"); pmm!(ui, length, 1.0..=500.0, 0.5, changed); ui.end_row();
                    });
                }
                ActiveTask::SurfaceRuled { width, depth, offset, .. } => {
                    theme::draw_task_section(ui, "Ruled Surface");
                    egui::Grid::new("task_sruled").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "Width"); pmm!(ui, width, 0.1..=500.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "Depth"); pmm!(ui, depth, 0.1..=500.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "Offset"); pmm!(ui, offset, -100.0..=100.0, 0.5, changed); ui.end_row();
                    });
                }
                ActiveTask::FemMesh { element_size, .. } => {
                    theme::draw_task_section(ui, "Mesh Generation");
                    egui::Grid::new("task_fem").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "Element Size"); pmm!(ui, element_size, 0.1..=50.0, 0.1, changed); ui.end_row();
                    });
                }
                ActiveTask::BooleanOp { op_type, width, height, depth, offset_x, offset_y, offset_z, .. } => {
                    theme::draw_task_section(ui, "Operation");
                    egui::Grid::new("task_bool_op").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "Type");
                        let labels = ["Union", "Subtract", "Intersect"];
                        let mut op_f = *op_type as f64;
                        if ui.add(egui::DragValue::new(&mut op_f).range(0.0..=2.0).speed(0.1).custom_formatter(|v, _| {
                            labels[v as usize % 3].to_string()
                        })).changed() {
                            *op_type = op_f as u8;
                            changed = true;
                        }
                        ui.end_row();
                    });
                    theme::draw_task_section(ui, "Tool Shape");
                    egui::Grid::new("task_bool_shape").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "Width"); pmm!(ui, width, 0.1..=500.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "Height"); pmm!(ui, height, 0.1..=500.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "Depth"); pmm!(ui, depth, 0.1..=500.0, 0.5, changed); ui.end_row();
                    });
                    theme::draw_task_section(ui, "Offset");
                    egui::Grid::new("task_bool_off").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "X"); pmm!(ui, offset_x, -500.0..=500.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "Y"); pmm!(ui, offset_y, -500.0..=500.0, 0.5, changed); ui.end_row();
                        plabel!(ui, "Z"); pmm!(ui, offset_z, -500.0..=500.0, 0.5, changed); ui.end_row();
                    });
                }
                ActiveTask::ScaleOp { factor, .. } => {
                    theme::draw_task_section(ui, "Scale");
                    egui::Grid::new("task_scale").num_columns(2).spacing(grid_sp).show(ui, |ui| {
                        plabel!(ui, "Factor"); pval!(ui, factor, 0.01..=100.0, 0.1, changed); ui.end_row();
                    });
                }
            }

            let (ok, can) = theme::draw_task_buttons(ui);
            commit = ok;
            cancel = can;

            // Status hint
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Parameters update preview in real-time")
                    .size(10.0)
                    .color(theme::COLOR_DIM)
                    .italics(),
            );
    }

    if commit {
        // On OK: if we have a preview, just keep it (it's already in the scene)
        // Otherwise create from scratch
        if task.preview_id().is_none() {
            emit_create_action(gui, &task);
        }
        gui.active_task = None;
    } else if cancel {
        // Remove preview object if it exists
        if let Some(pid) = task.preview_id() {
            gui.actions.push(GuiAction::RemoveObject(pid));
        }
        gui.active_task = None;
    } else {
        // Live preview: if params changed, emit preview update
        if changed {
            gui.actions.push(GuiAction::TaskPreviewUpdate(task.clone()));
        }
        gui.active_task = Some(task);
    }

    true
}

fn emit_create_action(gui: &mut GuiState, task: &ActiveTask) {
    match task {
        ActiveTask::Box { width, height, depth, .. } => {
            gui.actions.push(GuiAction::CreateBox {
                width: *width, height: *height, depth: *depth,
            });
        }
        ActiveTask::Cylinder { radius, height, .. } => {
            gui.actions.push(GuiAction::CreateCylinder {
                radius: *radius, height: *height,
            });
        }
        ActiveTask::Sphere { radius, .. } => {
            gui.actions.push(GuiAction::CreateSphere { radius: *radius });
        }
        ActiveTask::Cone { base_radius, top_radius, height, .. } => {
            gui.actions.push(GuiAction::CreateCone {
                base_radius: *base_radius, top_radius: *top_radius, height: *height,
            });
        }
        ActiveTask::Torus { major_radius, minor_radius, .. } => {
            gui.actions.push(GuiAction::CreateTorus {
                major_radius: *major_radius, minor_radius: *minor_radius,
            });
        }
        ActiveTask::Tube { outer_radius, inner_radius, height, .. } => {
            gui.actions.push(GuiAction::CreateTube {
                outer_radius: *outer_radius, inner_radius: *inner_radius, height: *height,
            });
        }
        ActiveTask::Prism { radius, height, sides, .. } => {
            gui.actions.push(GuiAction::CreatePrism {
                radius: *radius, height: *height, sides: *sides,
            });
        }
        ActiveTask::Wedge { dx, dy, dz, dx2, dy2, .. } => {
            gui.actions.push(GuiAction::CreateWedge {
                dx: *dx, dy: *dy, dz: *dz, dx2: *dx2, dy2: *dy2,
            });
        }
        ActiveTask::Ellipsoid { rx, ry, rz, .. } => {
            gui.actions.push(GuiAction::CreateEllipsoid {
                rx: *rx, ry: *ry, rz: *rz,
            });
        }
        ActiveTask::Helix { radius, pitch, turns, tube_radius, .. } => {
            gui.actions.push(GuiAction::CreateHelix {
                radius: *radius, pitch: *pitch, turns: *turns, tube_radius: *tube_radius,
            });
        }
        ActiveTask::Pad { depth, symmetric, .. } => {
            gui.actions.push(GuiAction::PartDesign(super::PartDesignAction::PadSketch {
                depth: *depth, symmetric: *symmetric,
            }));
        }
        ActiveTask::Pocket { depth, through_all, .. } => {
            gui.actions.push(GuiAction::PartDesign(super::PartDesignAction::PocketSketch {
                depth: *depth, through_all: *through_all,
            }));
        }
        ActiveTask::Hole { radius, depth, countersink, countersink_angle, .. } => {
            if *countersink {
                gui.actions.push(GuiAction::PartDesign(super::PartDesignAction::CountersunkHoleSketch {
                    radius: *radius, depth: *depth, countersink_angle: *countersink_angle,
                }));
            } else {
                gui.actions.push(GuiAction::PartDesign(super::PartDesignAction::HoleSketch {
                    radius: *radius, depth: *depth,
                }));
            }
        }
        ActiveTask::Groove { angle, .. } => {
            gui.actions.push(GuiAction::PartDesign(super::PartDesignAction::GrooveSketch { angle: *angle }));
        }
        ActiveTask::Fillet { radius, .. } => {
            gui.actions.push(GuiAction::FilletAllEdges { radius: *radius });
        }
        ActiveTask::Chamfer { distance, .. } => {
            gui.actions.push(GuiAction::ChamferAllEdges { distance: *distance });
        }
        ActiveTask::Shell { thickness, .. } => {
            gui.actions.push(GuiAction::ShellSolid { thickness: *thickness });
        }
        ActiveTask::MirrorOp { plane, .. } => {
            let mp = match plane {
                0 => MirrorPlane::XY,
                1 => MirrorPlane::XZ,
                _ => MirrorPlane::YZ,
            };
            gui.actions.push(GuiAction::MirrorSolid(mp));
        }
        ActiveTask::Pattern { count, spacing, axis, .. } => {
            gui.actions.push(GuiAction::LinearPattern {
                count: *count, spacing: *spacing, axis: *axis,
            });
        }
        ActiveTask::Sprocket { teeth, roller_diameter, pitch, bore, .. } => {
            gui.actions.push(GuiAction::PartDesign(super::PartDesignAction::CreateSprocket {
                teeth: *teeth, roller_diameter: *roller_diameter, pitch: *pitch, bore: *bore,
            }));
        }
        ActiveTask::InvoluteGear { teeth, module_val, pressure_angle, .. } => {
            gui.actions.push(GuiAction::PartDesign(super::PartDesignAction::CreateInvoluteGear {
                teeth: *teeth, module_val: *module_val, pressure_angle: *pressure_angle,
            }));
        }
        ActiveTask::DraftLine { .. } => {
            gui.actions.push(GuiAction::DraftLine);
        }
        ActiveTask::DraftCircle { .. } => {
            gui.actions.push(GuiAction::DraftCircle);
        }
        ActiveTask::DraftRectangle { .. } => {
            gui.actions.push(GuiAction::DraftRectangle);
        }
        ActiveTask::DraftPolygon { .. } => {
            gui.actions.push(GuiAction::DraftPolygon);
        }
        ActiveTask::DraftArc { .. } => {
            gui.actions.push(GuiAction::DraftArc);
        }
        ActiveTask::DraftEllipse { .. } => {
            gui.actions.push(GuiAction::DraftEllipse);
        }
        ActiveTask::SurfacePipe { .. } => {
            gui.actions.push(GuiAction::Surface(super::SurfaceAction::Pipe));
        }
        ActiveTask::SurfaceRuled { .. } => {
            gui.actions.push(GuiAction::StatusMessage("Ruled surface created".into()));
        }
        ActiveTask::FemMesh { element_size, .. } => {
            gui.actions.push(GuiAction::Fem(super::FemAction::GenTetMesh { element_size: *element_size }));
        }
        ActiveTask::BooleanOp { op_type, width, height, depth, offset_x, offset_y, offset_z, .. } => {
            let offset = [*offset_x, *offset_y, *offset_z];
            match op_type {
                0 => gui.actions.push(GuiAction::BooleanUnionWith {
                    width: *width, height: *height, depth: *depth, offset,
                }),
                1 => gui.actions.push(GuiAction::BooleanSubtractWith {
                    width: *width, height: *height, depth: *depth, offset,
                }),
                _ => gui.actions.push(GuiAction::BooleanIntersectWith {
                    width: *width, height: *height, depth: *depth, offset,
                }),
            }
        }
        ActiveTask::ScaleOp { factor, .. } => {
            gui.actions.push(GuiAction::ScaleSolid { factor: *factor });
        }
    }
}
