//! Task panel — contextual side panel for creation/modification operations.
//!
//! Replaces popup dialogs with an inline panel that shows parameters
//! and updates the 3D preview in real-time. Mirrors FreeCAD's TaskView.

use super::{GuiAction, GuiState};

/// Active task state — one variant per operation type.
#[derive(Clone, Debug)]
pub(crate) enum ActiveTask {
    Box { width: f64, height: f64, depth: f64, preview_id: Option<crate::scene::ObjectId> },
    Cylinder { radius: f64, height: f64, preview_id: Option<crate::scene::ObjectId> },
    Sphere { radius: f64, preview_id: Option<crate::scene::ObjectId> },
    Cone { base_radius: f64, top_radius: f64, height: f64, preview_id: Option<crate::scene::ObjectId> },
    Torus { major_radius: f64, minor_radius: f64, preview_id: Option<crate::scene::ObjectId> },
}

impl ActiveTask {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Box { .. } => "Create Box",
            Self::Cylinder { .. } => "Create Cylinder",
            Self::Sphere { .. } => "Create Sphere",
            Self::Cone { .. } => "Create Cone",
            Self::Torus { .. } => "Create Torus",
        }
    }

    pub fn preview_id(&self) -> Option<crate::scene::ObjectId> {
        match self {
            Self::Box { preview_id, .. }
            | Self::Cylinder { preview_id, .. }
            | Self::Sphere { preview_id, .. }
            | Self::Cone { preview_id, .. }
            | Self::Torus { preview_id, .. } => *preview_id,
        }
    }

    pub fn set_preview_id(&mut self, id: crate::scene::ObjectId) {
        match self {
            Self::Box { preview_id, .. }
            | Self::Cylinder { preview_id, .. }
            | Self::Sphere { preview_id, .. }
            | Self::Cone { preview_id, .. }
            | Self::Torus { preview_id, .. } => *preview_id = Some(id),
        }
    }
}

/// Standalone panel version (deprecated — kept for compatibility).
#[allow(dead_code)]
pub(crate) fn draw_task_panel(
    _ctx: &egui::Context,
    _gui: &mut GuiState,
) -> bool {
    false // Now drawn inline inside ComboView
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
    let mut task = task.clone();
    let mut commit = false;
    let mut cancel = false;
    let mut changed = false;

    {
            ui.heading(format!("\u{2699} {title}"));
            ui.separator();

            match &mut task {
                ActiveTask::Box { width, height, depth, .. } => {
                    egui::Grid::new("task_box").num_columns(2).show(ui, |ui| {
                        ui.label("Width:");
                        changed |= ui.add(egui::DragValue::new(width).range(0.1..=1000.0).speed(0.5)).changed();
                        ui.end_row();
                        ui.label("Height:");
                        changed |= ui.add(egui::DragValue::new(height).range(0.1..=1000.0).speed(0.5)).changed();
                        ui.end_row();
                        ui.label("Depth:");
                        changed |= ui.add(egui::DragValue::new(depth).range(0.1..=1000.0).speed(0.5)).changed();
                        ui.end_row();
                    });
                }
                ActiveTask::Cylinder { radius, height, .. } => {
                    egui::Grid::new("task_cyl").num_columns(2).show(ui, |ui| {
                        ui.label("Radius:");
                        changed |= ui.add(egui::DragValue::new(radius).range(0.1..=500.0).speed(0.5)).changed();
                        ui.end_row();
                        ui.label("Height:");
                        changed |= ui.add(egui::DragValue::new(height).range(0.1..=1000.0).speed(0.5)).changed();
                        ui.end_row();
                    });
                }
                ActiveTask::Sphere { radius, .. } => {
                    egui::Grid::new("task_sph").num_columns(2).show(ui, |ui| {
                        ui.label("Radius:");
                        changed |= ui.add(egui::DragValue::new(radius).range(0.1..=500.0).speed(0.5)).changed();
                        ui.end_row();
                    });
                }
                ActiveTask::Cone { base_radius, top_radius, height, .. } => {
                    egui::Grid::new("task_cone").num_columns(2).show(ui, |ui| {
                        ui.label("Base Radius:");
                        changed |= ui.add(egui::DragValue::new(base_radius).range(0.01..=500.0).speed(0.5)).changed();
                        ui.end_row();
                        ui.label("Top Radius:");
                        changed |= ui.add(egui::DragValue::new(top_radius).range(0.0..=500.0).speed(0.5)).changed();
                        ui.end_row();
                        ui.label("Height:");
                        changed |= ui.add(egui::DragValue::new(height).range(0.1..=1000.0).speed(0.5)).changed();
                        ui.end_row();
                    });
                }
                ActiveTask::Torus { major_radius, minor_radius, .. } => {
                    egui::Grid::new("task_tor").num_columns(2).show(ui, |ui| {
                        ui.label("Major Radius:");
                        changed |= ui.add(egui::DragValue::new(major_radius).range(0.1..=500.0).speed(0.5)).changed();
                        ui.end_row();
                        ui.label("Minor Radius:");
                        changed |= ui.add(egui::DragValue::new(minor_radius).range(0.01..=200.0).speed(0.1)).changed();
                        ui.end_row();
                    });
                }
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("\u{2714} OK").clicked() {
                    commit = true;
                }
                if ui.button("\u{2716} Cancel").clicked() {
                    cancel = true;
                }
            });

            ui.separator();
            ui.weak("Parameters update the preview in real-time.");
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
    }
}
