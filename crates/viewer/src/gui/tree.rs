use super::{GuiAction, GuiState};
use crate::scene::Scene;

pub(crate) fn draw_model_tree(
    ctx: &egui::Context,
    gui: &mut GuiState,
    scene: &Scene,
) {
    if !gui.show_model_tree {
        return;
    }
    egui::SidePanel::left("model_tree")
        .default_width(240.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Scene");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.weak(format!("{} object(s)", scene.len()));
                });
            });
            // Search filter
            ui.horizontal(|ui| {
                ui.label("\u{1F50D}");
                ui.text_edit_singleline(&mut gui.tree_filter);
            });
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                if scene.is_empty() {
                    ui.weak("(empty scene — create or import a model)");
                    return;
                }

                let filter = gui.tree_filter.to_lowercase();
                for obj in &scene.objects {
                    if !filter.is_empty() && !obj.name.to_lowercase().contains(&filter) {
                        continue;
                    }
                    draw_object_row(ui, gui, obj);
                }
            });
        });
}

fn draw_object_row(
    ui: &mut egui::Ui,
    gui: &mut GuiState,
    obj: &crate::scene::SceneObject,
) {
    let is_selected = obj.selected;
    let id = obj.id;

    ui.horizontal(|ui| {
        // Visibility toggle (eye icon)
        let eye = if obj.visible { "\u{25C9}" } else { "\u{25CB}" };
        let eye_btn = ui.button(
            egui::RichText::new(eye).size(14.0).color(
                if obj.visible {
                    egui::Color32::from_rgb(80, 180, 80)
                } else {
                    egui::Color32::from_rgb(120, 120, 120)
                },
            ),
        );
        if eye_btn.clicked() {
            gui.actions.push(GuiAction::ToggleVisibility(id));
        }

        // Color swatch
        let [r, g, b, _] = obj.color;
        let color = egui::Color32::from_rgb(
            (r * 255.0) as u8,
            (g * 255.0) as u8,
            (b * 255.0) as u8,
        );
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(14.0, 14.0),
            egui::Sense::click(),
        );
        ui.painter().rect_filled(rect, 2.0, color);

        // Object name (selectable)
        let label_color = if is_selected {
            egui::Color32::from_rgb(100, 200, 255)
        } else if !obj.visible {
            egui::Color32::from_rgb(130, 130, 130)
        } else {
            ui.visuals().text_color()
        };

        let name_resp = ui.selectable_label(
            is_selected,
            egui::RichText::new(&obj.name).color(label_color),
        );

        if name_resp.clicked() {
            gui.actions.push(GuiAction::SelectObject(id));
        }

        // Context menu
        name_resp.context_menu(|ui| {
            if ui.button("\u{1F441} Toggle Visibility").clicked() {
                gui.actions.push(GuiAction::ToggleVisibility(id));
                ui.close_menu();
            }
            if ui.button("\u{1F5D1} Delete").clicked() {
                gui.actions.push(GuiAction::RemoveObject(id));
                ui.close_menu();
            }
            if ui.button("\u{1F4CB} Duplicate").clicked() {
                gui.actions.push(GuiAction::DuplicateObject(id));
                ui.close_menu();
            }
            ui.separator();
            if ui.button("\u{1F4CF} Measure").clicked() {
                gui.actions.push(GuiAction::MeasureSolid);
                ui.close_menu();
            }
            if ui.button("\u{2714} Check Geometry").clicked() {
                gui.actions.push(GuiAction::CheckGeometry);
                ui.close_menu();
            }
        });
    });

    // Show topology details if selected
    if is_selected {
        ui.indent(format!("obj_detail_{id}"), |ui| {
            let m = &obj.model;
            if !m.solids.is_empty() {
                ui.weak(format!(
                    "  {} solid, {} faces, {} edges, {} verts",
                    m.solids.len(),
                    m.faces.len(),
                    m.edges.len(),
                    m.vertices.len(),
                ));
            }
            ui.weak(format!(
                "  {} triangles, {} vertices (mesh)",
                obj.mesh.triangle_count(),
                obj.mesh.vertices.len(),
            ));
            if let Some(params) = &obj.params {
                ui.weak(format!("  Type: {}", params_label(params)));
            }
        });
    }
}

fn params_label(p: &crate::scene::CreationParams) -> &'static str {
    use crate::scene::CreationParams;
    match p {
        CreationParams::Box { .. } => "Box",
        CreationParams::Cylinder { .. } => "Cylinder",
        CreationParams::Sphere { .. } => "Sphere",
        CreationParams::Cone { .. } => "Cone",
        CreationParams::Torus { .. } => "Torus",
        CreationParams::Tube { .. } => "Tube",
        CreationParams::Prism { .. } => "Prism",
        CreationParams::Wedge { .. } => "Wedge",
        CreationParams::Ellipsoid { .. } => "Ellipsoid",
        CreationParams::Helix { .. } => "Helix",
        CreationParams::Imported { .. } => "Imported",
        CreationParams::Extruded => "Extruded",
        CreationParams::Revolved => "Revolved",
        CreationParams::Boolean { .. } => "Boolean",
    }
}
