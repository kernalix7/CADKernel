use super::{GuiState, SelectedEntity};
use cadkernel_topology::BRepModel;

pub(crate) fn draw_model_tree(ctx: &egui::Context, gui: &mut GuiState, model: &BRepModel) {
    if !gui.show_model_tree {
        return;
    }
    egui::SidePanel::left("model_tree")
        .default_width(220.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Model");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.weak(format!(
                        "S:{} F:{} E:{} V:{}",
                        model.solids.len(),
                        model.faces.len(),
                        model.edges.len(),
                        model.vertices.len(),
                    ));
                });
            });
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                // Construction history
                let records = model.history.records();
                if !records.is_empty() {
                    egui::CollapsingHeader::new("Construction History")
                        .default_open(false)
                        .show(ui, |ui| {
                            for (i, record) in records.iter().enumerate() {
                                ui.label(format!("{}. {}", i + 1, record.label));
                            }
                        });
                    ui.separator();
                }

                // Solids hierarchy
                if model.solids.is_empty() {
                    ui.weak("(no geometry)");
                } else {
                    for (solid_handle, solid_data) in model.solids.iter() {
                        let solid_id = format!("solid_{:?}", solid_handle);
                        let solid_label = if let Some(tag) = &solid_data.tag {
                            format!("Solid [{}]", tag)
                        } else {
                            format!("Solid #{:?}", solid_handle)
                        };

                        let is_solid_selected = gui.selected_entity
                            == Some(SelectedEntity::Solid(solid_handle));

                        let header = egui::CollapsingHeader::new(
                            egui::RichText::new(&solid_label)
                                .strong()
                                .color(if is_solid_selected {
                                    egui::Color32::from_rgb(100, 180, 255)
                                } else {
                                    ui.visuals().text_color()
                                }),
                        )
                        .id_salt(&solid_id)
                        .default_open(true);

                        header.show(ui, |ui| {
                            // Click to select solid
                            let resp = ui.selectable_label(
                                is_solid_selected,
                                format!(
                                    "  {} shell(s)",
                                    solid_data.shells.len()
                                ),
                            );
                            if resp.clicked() {
                                gui.selected_entity =
                                    Some(SelectedEntity::Solid(solid_handle));
                            }
                            resp.context_menu(|ui| {
                                super::context_menu::solid_context_menu(ui, gui, solid_handle);
                            });

                            // Shells
                            for (si, &shell_handle) in solid_data.shells.iter().enumerate() {
                                if let Some(shell_data) = model.shells.get(shell_handle) {
                                    let shell_id = format!("{solid_id}_shell_{si}");
                                    let shell_label = format!("Shell #{si}");
                                    let is_shell_selected = gui.selected_entity
                                        == Some(SelectedEntity::Shell(shell_handle));

                                    egui::CollapsingHeader::new(
                                        egui::RichText::new(&shell_label).color(
                                            if is_shell_selected {
                                                egui::Color32::from_rgb(100, 180, 255)
                                            } else {
                                                ui.visuals().text_color()
                                            },
                                        ),
                                    )
                                    .id_salt(&shell_id)
                                    .default_open(false)
                                    .show(ui, |ui| {
                                        let resp = ui.selectable_label(
                                            is_shell_selected,
                                            format!(
                                                "  {} face(s)",
                                                shell_data.faces.len()
                                            ),
                                        );
                                        if resp.clicked() {
                                            gui.selected_entity =
                                                Some(SelectedEntity::Shell(shell_handle));
                                        }

                                        // Faces
                                        for (fi, &face_handle) in
                                            shell_data.faces.iter().enumerate()
                                        {
                                            let is_face_selected = gui.selected_entity
                                                == Some(SelectedEntity::Face(face_handle));
                                            let face_label = if let Some(face) =
                                                model.faces.get(face_handle)
                                            {
                                                if let Some(tag) = &face.tag {
                                                    format!("Face [{tag}]")
                                                } else {
                                                    format!("Face #{fi}")
                                                }
                                            } else {
                                                format!("Face #{fi}")
                                            };
                                            let resp = ui.selectable_label(
                                                is_face_selected,
                                                format!("    {face_label}"),
                                            );
                                            if resp.clicked() {
                                                gui.selected_entity =
                                                    Some(SelectedEntity::Face(face_handle));
                                            }
                                        }
                                    });
                                }
                            }
                        });
                    }
                }

                // Standalone faces (not in any solid)
                let standalone_faces: Vec<_> = model
                    .faces
                    .iter()
                    .filter(|(_, f)| f.shell.is_none())
                    .collect();
                if !standalone_faces.is_empty() {
                    ui.separator();
                    egui::CollapsingHeader::new("Standalone Faces")
                        .default_open(false)
                        .show(ui, |ui| {
                            for (handle, _) in &standalone_faces {
                                let is_selected = gui.selected_entity
                                    == Some(SelectedEntity::Face(*handle));
                                let resp = ui.selectable_label(
                                    is_selected,
                                    format!("Face #{:?}", handle),
                                );
                                if resp.clicked() {
                                    gui.selected_entity =
                                        Some(SelectedEntity::Face(*handle));
                                }
                            }
                        });
                }

                // Edges summary
                if !model.edges.is_empty() {
                    egui::CollapsingHeader::new(format!("Edges ({})", model.edges.len()))
                        .default_open(false)
                        .show(ui, |ui| {
                            for (handle, _) in model.edges.iter() {
                                let is_selected = gui.selected_entity
                                    == Some(SelectedEntity::Edge(handle));
                                let resp = ui.selectable_label(
                                    is_selected,
                                    format!("  Edge #{:?}", handle),
                                );
                                if resp.clicked() {
                                    gui.selected_entity =
                                        Some(SelectedEntity::Edge(handle));
                                }
                            }
                        });
                }

                // Vertices summary
                if !model.vertices.is_empty() {
                    egui::CollapsingHeader::new(format!("Vertices ({})", model.vertices.len()))
                        .default_open(false)
                        .show(ui, |ui| {
                            for (handle, v) in model.vertices.iter() {
                                let is_selected = gui.selected_entity
                                    == Some(SelectedEntity::Vertex(handle));
                                let resp = ui.selectable_label(
                                    is_selected,
                                    format!(
                                        "  ({:.2}, {:.2}, {:.2})",
                                        v.point.x, v.point.y, v.point.z
                                    ),
                                );
                                if resp.clicked() {
                                    gui.selected_entity =
                                        Some(SelectedEntity::Vertex(handle));
                                }
                            }
                        });
                }
            });
        });
}
