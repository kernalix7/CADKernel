//! Task panel — contextual side panel for creation/modification operations.
//!
//! Replaces popup dialogs with an inline panel that shows parameters
//! and updates the 3D preview in real-time. Mirrors FreeCAD's TaskView.

use super::{GuiAction, GuiState};

/// Active task state — one variant per operation type.
#[derive(Clone, Debug)]
pub(crate) enum ActiveTask {
    Box { width: f64, height: f64, depth: f64 },
    Cylinder { radius: f64, height: f64 },
    Sphere { radius: f64 },
    Cone { base_radius: f64, top_radius: f64, height: f64 },
    Torus { major_radius: f64, minor_radius: f64 },
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
}

/// Draw the task panel if a task is active.
/// Returns true if the task panel is shown (so properties panel can be hidden).
pub(crate) fn draw_task_panel(
    ctx: &egui::Context,
    gui: &mut GuiState,
) -> bool {
    let Some(task) = &gui.active_task else {
        return false;
    };
    let title = task.title().to_string();
    let mut task = task.clone();
    let mut commit = false;
    let mut cancel = false;

    egui::SidePanel::right("task_panel")
        .default_width(280.0)
        .show(ctx, |ui| {
            ui.heading(format!("\u{2699} {title}"));
            ui.separator();

            match &mut task {
                ActiveTask::Box { width, height, depth } => {
                    egui::Grid::new("task_box").num_columns(2).show(ui, |ui| {
                        ui.label("Width:");
                        ui.add(egui::DragValue::new(width).range(0.1..=1000.0).speed(0.5));
                        ui.end_row();
                        ui.label("Height:");
                        ui.add(egui::DragValue::new(height).range(0.1..=1000.0).speed(0.5));
                        ui.end_row();
                        ui.label("Depth:");
                        ui.add(egui::DragValue::new(depth).range(0.1..=1000.0).speed(0.5));
                        ui.end_row();
                    });
                }
                ActiveTask::Cylinder { radius, height } => {
                    egui::Grid::new("task_cyl").num_columns(2).show(ui, |ui| {
                        ui.label("Radius:");
                        ui.add(egui::DragValue::new(radius).range(0.1..=500.0).speed(0.5));
                        ui.end_row();
                        ui.label("Height:");
                        ui.add(egui::DragValue::new(height).range(0.1..=1000.0).speed(0.5));
                        ui.end_row();
                    });
                }
                ActiveTask::Sphere { radius } => {
                    egui::Grid::new("task_sph").num_columns(2).show(ui, |ui| {
                        ui.label("Radius:");
                        ui.add(egui::DragValue::new(radius).range(0.1..=500.0).speed(0.5));
                        ui.end_row();
                    });
                }
                ActiveTask::Cone { base_radius, top_radius, height } => {
                    egui::Grid::new("task_cone").num_columns(2).show(ui, |ui| {
                        ui.label("Base Radius:");
                        ui.add(egui::DragValue::new(base_radius).range(0.01..=500.0).speed(0.5));
                        ui.end_row();
                        ui.label("Top Radius:");
                        ui.add(egui::DragValue::new(top_radius).range(0.0..=500.0).speed(0.5));
                        ui.end_row();
                        ui.label("Height:");
                        ui.add(egui::DragValue::new(height).range(0.1..=1000.0).speed(0.5));
                        ui.end_row();
                    });
                }
                ActiveTask::Torus { major_radius, minor_radius } => {
                    egui::Grid::new("task_tor").num_columns(2).show(ui, |ui| {
                        ui.label("Major Radius:");
                        ui.add(egui::DragValue::new(major_radius).range(0.1..=500.0).speed(0.5));
                        ui.end_row();
                        ui.label("Minor Radius:");
                        ui.add(egui::DragValue::new(minor_radius).range(0.01..=200.0).speed(0.1));
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
            ui.weak("Adjust parameters above.\nClick OK to create the object.");
        });

    if commit {
        // Emit the create action from the task parameters
        match &task {
            ActiveTask::Box { width, height, depth } => {
                gui.actions.push(GuiAction::CreateBox {
                    width: *width, height: *height, depth: *depth,
                });
            }
            ActiveTask::Cylinder { radius, height } => {
                gui.actions.push(GuiAction::CreateCylinder {
                    radius: *radius, height: *height,
                });
            }
            ActiveTask::Sphere { radius } => {
                gui.actions.push(GuiAction::CreateSphere { radius: *radius });
            }
            ActiveTask::Cone { base_radius, top_radius, height } => {
                gui.actions.push(GuiAction::CreateCone {
                    base_radius: *base_radius, top_radius: *top_radius, height: *height,
                });
            }
            ActiveTask::Torus { major_radius, minor_radius } => {
                gui.actions.push(GuiAction::CreateTorus {
                    major_radius: *major_radius, minor_radius: *minor_radius,
                });
            }
        }
        gui.active_task = None;
    } else if cancel {
        gui.active_task = None;
    } else {
        gui.active_task = Some(task);
    }

    true
}
