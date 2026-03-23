use super::{GuiAction, GuiState};
use crate::render::{Camera, DisplayMode, Projection, StandardView};

pub(crate) fn draw_menu_bar(
    ctx: &egui::Context,
    gui: &mut GuiState,
    camera: &Camera,
    display_mode: DisplayMode,
) {
    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            // ---- File ----
            ui.menu_button("File", |ui| {
                if ui.button("New").clicked() {
                    gui.actions.push(GuiAction::NewModel);
                    gui.status_message = "New model".into();
                    ui.close_menu();
                }

                if ui.button("Open…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("CADKernel Project", &["cadk"])
                        .add_filter("Mesh files", &["stl", "obj"])
                        .add_filter("All supported", &["cadk", "stl", "obj"])
                        .pick_file()
                    {
                        gui.status_message = format!("Opening {}", path.display());
                        gui.actions.push(GuiAction::OpenFile(path));
                    }
                    ui.close_menu();
                }

                if ui.button("Save As…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("CADKernel Project", &["cadk"])
                        .set_file_name("project.cadk")
                        .save_file()
                    {
                        gui.status_message = format!("Saving {}", path.display());
                        gui.actions.push(GuiAction::SaveFile(path));
                    }
                    ui.close_menu();
                }

                // Recent files
                if !gui.recent_files.is_empty() {
                    ui.menu_button("Recent Files", |ui| {
                        for path_str in gui.recent_files.clone() {
                            let short = path_str.rsplit('/').next().unwrap_or(&path_str).to_string();
                            if ui.button(&short).on_hover_text(&path_str).clicked() {
                                gui.actions.push(GuiAction::OpenFile(std::path::PathBuf::from(&path_str)));
                                ui.close_menu();
                            }
                        }
                    });
                }

                ui.separator();

                // Import submenu
                ui.menu_button("Import", |ui| {
                    for (label, exts) in &[
                        ("STL…", vec!["stl"]),
                        ("OBJ…", vec!["obj"]),
                        ("STEP…", vec!["step", "stp"]),
                        ("IGES…", vec!["iges", "igs"]),
                        ("DXF…", vec!["dxf"]),
                        ("PLY…", vec!["ply"]),
                        ("BREP…", vec!["brep", "brp"]),
                    ] {
                        if ui.button(*label).clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter(label.trim_end_matches('…'), exts)
                                .pick_file()
                            {
                                gui.status_message = format!("Importing {}", path.display());
                                gui.actions.push(GuiAction::ImportFile(path));
                            }
                            ui.close_menu();
                        }
                    }
                });

                // Export submenu
                ui.menu_button("Export", |ui| {
                    #[allow(clippy::type_complexity)]
                    let exports: &[(&str, &str, &[&str], fn(std::path::PathBuf) -> GuiAction)] = &[
                        ("STL…", "model.stl", &["stl"], |p| GuiAction::ExportStl(p)),
                        ("OBJ…", "model.obj", &["obj"], |p| GuiAction::ExportObj(p)),
                        ("glTF…", "model.gltf", &["gltf"], |p| GuiAction::ExportGltf(p)),
                        ("STEP…", "model.step", &["step", "stp"], |p| GuiAction::ExportStep(p)),
                        ("IGES…", "model.iges", &["iges", "igs"], |p| GuiAction::ExportIges(p)),
                        ("DXF…", "model.dxf", &["dxf"], |p| GuiAction::ExportDxf(p)),
                        ("PLY…", "model.ply", &["ply"], |p| GuiAction::ExportPly(p)),
                        ("3MF…", "model.3mf", &["3mf"], |p| GuiAction::Export3mf(p)),
                        ("BREP…", "model.brep", &["brep", "brp"], |p| GuiAction::ExportBrep(p)),
                    ];
                    for (label, filename, exts, make_action) in exports {
                        if ui.button(*label).clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter(label.trim_end_matches('…'), exts)
                                .set_file_name(*filename)
                                .save_file()
                            {
                                gui.actions.push(make_action(path));
                            }
                            ui.close_menu();
                        }
                    }
                });

                ui.separator();

                if ui.button("Quit").clicked() {
                    gui.request_quit = true;
                }
            });

            // ---- Edit ----
            ui.menu_button("Edit", |ui| {
                if ui.button("Undo  (Ctrl+Z)").clicked() {
                    gui.actions.push(GuiAction::Undo);
                    ui.close_menu();
                }
                if ui.button("Redo  (Ctrl+Y)").clicked() {
                    gui.actions.push(GuiAction::Redo);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Select All  (Ctrl+A)").clicked() {
                    gui.actions.push(GuiAction::SelectAll);
                    ui.close_menu();
                }
                if ui.button("Deselect All").clicked() {
                    gui.actions.push(GuiAction::DeselectAll);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Delete  (Del)").clicked() {
                    gui.actions.push(GuiAction::DeleteSelected);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Settings…").clicked() {
                    gui.show_settings = true;
                    ui.close_menu();
                }
            });

            // ---- Create ----
            ui.menu_button("Create", |ui| {
                if ui.button("Box…").clicked() {
                    gui.show_create_box = true;
                    ui.close_menu();
                }
                if ui.button("Cylinder…").clicked() {
                    gui.show_create_cylinder = true;
                    ui.close_menu();
                }
                if ui.button("Sphere…").clicked() {
                    gui.show_create_sphere = true;
                    ui.close_menu();
                }
                if ui.button("Cone…").clicked() {
                    gui.show_create_cone = true;
                    ui.close_menu();
                }
                if ui.button("Torus…").clicked() {
                    gui.show_create_torus = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Tube…").clicked() {
                    gui.show_create_tube = true;
                    ui.close_menu();
                }
                if ui.button("Prism…").clicked() {
                    gui.show_create_prism = true;
                    ui.close_menu();
                }
                if ui.button("Wedge…").clicked() {
                    gui.show_create_wedge = true;
                    ui.close_menu();
                }
                if ui.button("Ellipsoid…").clicked() {
                    gui.show_create_ellipsoid = true;
                    ui.close_menu();
                }
                if ui.button("Helix…").clicked() {
                    gui.show_create_helix = true;
                    ui.close_menu();
                }
            });

            // ---- Macro ----
            ui.menu_button("Macro", |ui| {
                if ui.button("Macro Console…").clicked() {
                    gui.status_message = "Macro console (not yet implemented)".into();
                    ui.close_menu();
                }
                if ui.button("Start Recording").clicked() {
                    gui.status_message = "Macro recording (not yet implemented)".into();
                    ui.close_menu();
                }
                if ui.button("Stop Recording").clicked() {
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Execute Macro…").clicked() {
                    gui.status_message = "Macro execute (not yet implemented)".into();
                    ui.close_menu();
                }
            });

            // ---- View ----
            ui.menu_button("View", |ui| {
                // Panels
                ui.menu_button("Panels", |ui| {
                    ui.checkbox(&mut gui.show_model_tree, "Model Tree");
                    ui.checkbox(&mut gui.show_properties, "Properties");
                    ui.checkbox(&mut gui.show_report_panel, "Report View");
                });
                ui.separator();

                // Display modes
                ui.menu_button("Display Mode", |ui| {
                    for &mode in DisplayMode::ALL {
                        let selected = mode == display_mode;
                        let text = format!("{}    {}", mode.label(), mode.shortcut());
                        if ui.selectable_label(selected, text).clicked() {
                            gui.actions.push(GuiAction::SetDisplayMode(mode));
                            ui.close_menu();
                        }
                    }
                });
                ui.separator();

                let proj_label = match camera.projection {
                    Projection::Perspective => "Switch to Orthographic  (5)",
                    Projection::Orthographic => "Switch to Perspective  (5)",
                };
                if ui.button(proj_label).clicked() {
                    gui.actions.push(GuiAction::ToggleProjection);
                    ui.close_menu();
                }
                ui.separator();

                ui.menu_button("Standard Views", |ui| {
                    for &(view, key) in &[
                        (StandardView::Front, "1"),
                        (StandardView::Back, "Ctrl+1"),
                        (StandardView::Right, "3"),
                        (StandardView::Left, "Ctrl+3"),
                        (StandardView::Top, "7"),
                        (StandardView::Bottom, "Ctrl+7"),
                        (StandardView::Isometric, "0"),
                    ] {
                        if ui.button(format!("{}  ({})", view.label(), key)).clicked() {
                            gui.actions.push(GuiAction::SetStandardView(view));
                            ui.close_menu();
                        }
                    }
                });

                ui.separator();
                if ui.button("Toggle Grid  (G)").clicked() {
                    gui.actions.push(GuiAction::ToggleGrid);
                    ui.close_menu();
                }

                ui.separator();
                if ui.button("Reset Camera").clicked() {
                    gui.actions.push(GuiAction::ResetCamera);
                    ui.close_menu();
                }
                if ui.button("Fit All  (V)").clicked() {
                    gui.actions.push(GuiAction::FitAll);
                    ui.close_menu();
                }
            });

            // ---- Tools ----
            ui.menu_button("Tools", |ui| {
                if ui.button("Check Geometry").clicked() {
                    gui.actions.push(GuiAction::CheckGeometry);
                    ui.close_menu();
                }
                if ui.button("Mass Properties").clicked() {
                    gui.actions.push(GuiAction::MeasureSolid);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Measure Distance…").clicked() {
                    gui.status_message = "Measure distance (not yet implemented)".into();
                    ui.close_menu();
                }
            });

            // ---- Help ----
            ui.menu_button("Help", |ui| {
                if ui.button("About CADKernel").clicked() {
                    gui.show_about = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Keyboard Shortcuts").clicked() {
                    gui.status_message = "See View menu for shortcuts".into();
                    ui.close_menu();
                }
            });
        });
    });
}
