use super::{GuiAction, GuiState};
use crate::scene::Scene;

/// Standalone panel version (deprecated — kept for compatibility).
#[allow(dead_code)]
pub(crate) fn draw_model_tree(
    _ctx: &egui::Context,
    _gui: &mut GuiState,
    _scene: &Scene,
) {
    // Now drawn inline inside ComboView
}

/// Inline version — draws tree content into an existing Ui.
pub(crate) fn draw_model_tree_inline(
    ui: &mut egui::Ui,
    gui: &mut GuiState,
    scene: &Scene,
) {
    ui.horizontal(|ui| {
        ui.strong("\u{1F4C1} Model");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.weak(format!("{}", scene.len()));
        });
    });
    // Search filter
    ui.horizontal(|ui| {
        ui.label("\u{1F50D}");
        ui.text_edit_singleline(&mut gui.tree_filter);
    });

    if scene.is_empty() {
        ui.weak("(empty scene)");
        return;
    }

    let filter = gui.tree_filter.to_lowercase();
    for obj in &scene.objects {
        if !filter.is_empty() && !obj.name.to_lowercase().contains(&filter) {
            continue;
        }
        draw_object_row(ui, gui, obj);
    }
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

        // Object name (selectable, double-click to rename)
        let is_renaming = gui.rename_edit.as_ref().is_some_and(|(rid, _)| *rid == id);

        if is_renaming {
            let (_, text) = gui.rename_edit.as_mut().unwrap();
            let resp = ui.text_edit_singleline(text);
            if resp.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let new_name = gui.rename_edit.take().unwrap().1;
                if !new_name.is_empty() {
                    gui.actions.push(GuiAction::RenameObject(id, new_name));
                }
            }
        } else {
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
                if ui.input(|i| i.modifiers.ctrl) {
                    gui.actions.push(GuiAction::ToggleSelect(id));
                } else {
                    gui.actions.push(GuiAction::SelectObject(id));
                }
            }
            if name_resp.double_clicked() {
                gui.rename_edit = Some((id, obj.name.clone()));
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
            ui.menu_button("\u{2B06} Transform", |ui| {
                if ui.button("Move +X (10)").clicked() {
                    gui.actions.push(GuiAction::MoveObject { id, dx: 10.0, dy: 0.0, dz: 0.0 });
                    ui.close_menu();
                }
                if ui.button("Move +Y (10)").clicked() {
                    gui.actions.push(GuiAction::MoveObject { id, dx: 0.0, dy: 10.0, dz: 0.0 });
                    ui.close_menu();
                }
                if ui.button("Move +Z (10)").clicked() {
                    gui.actions.push(GuiAction::MoveObject { id, dx: 0.0, dy: 0.0, dz: 10.0 });
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Rotate X 90\u{00B0}").clicked() {
                    gui.actions.push(GuiAction::RotateObject { id, axis: 0, angle_deg: 90.0 });
                    ui.close_menu();
                }
                if ui.button("Rotate Y 90\u{00B0}").clicked() {
                    gui.actions.push(GuiAction::RotateObject { id, axis: 1, angle_deg: 90.0 });
                    ui.close_menu();
                }
                if ui.button("Rotate Z 90\u{00B0}").clicked() {
                    gui.actions.push(GuiAction::RotateObject { id, axis: 2, angle_deg: 90.0 });
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Scale 2\u{00D7}").clicked() {
                    gui.actions.push(GuiAction::ScaleObjectUniform { id, factor: 2.0 });
                    ui.close_menu();
                }
                if ui.button("Scale 0.5\u{00D7}").clicked() {
                    gui.actions.push(GuiAction::ScaleObjectUniform { id, factor: 0.5 });
                    ui.close_menu();
                }
            });
            ui.separator();
            if ui.button("\u{1F4CF} Measure").clicked() {
                gui.actions.push(GuiAction::MeasureSolid);
                ui.close_menu();
            }
            if ui.button("\u{2714} Check Geometry").clicked() {
                gui.actions.push(GuiAction::CheckGeometry);
                ui.close_menu();
            }
            ui.separator();
            ui.menu_button("\u{222A} Boolean", |ui| {
                if ui.button("\u{222A} Union (2 selected)").clicked() {
                    gui.actions.push(GuiAction::BooleanSceneUnion);
                    ui.close_menu();
                }
                if ui.button("\u{2212} Subtract (2 selected)").clicked() {
                    gui.actions.push(GuiAction::BooleanSceneSubtract);
                    ui.close_menu();
                }
                if ui.button("\u{2229} Intersect (2 selected)").clicked() {
                    gui.actions.push(GuiAction::BooleanSceneIntersect);
                    ui.close_menu();
                }
            });
            if ui.button("\u{270F} Rename").clicked() {
                gui.rename_edit = Some((id, obj.name.clone()));
                ui.close_menu();
            }
        });
        } // close else block for non-rename mode
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

            // Construction history
            let records = obj.model.history.records();
            if !records.is_empty() {
                egui::CollapsingHeader::new(format!("\u{1F4DC} History ({})", records.len()))
                    .id_salt(format!("hist_{id}"))
                    .default_open(false)
                    .show(ui, |ui| {
                        for (i, record) in records.iter().enumerate() {
                            ui.weak(format!("  {}. {}", i + 1, record.label));
                        }
                    });
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
