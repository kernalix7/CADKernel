use super::{AssemblyJointType, BcKind, FemConstraintType, GuiAction, GuiState, SketchTool, Workbench, task_panel};
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
                menu_section(ui, "Project");
                if ui.add(egui::Button::new("New").shortcut_text("Ctrl+N")).clicked() {
                    gui.actions.push(GuiAction::NewModel);
                    gui.status_message = "New model".into();
                    ui.close_menu();
                }

                if ui.add(egui::Button::new("Open\u{2026}").shortcut_text("Ctrl+O")).clicked() {
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

                if ui.add(egui::Button::new("Save As\u{2026}").shortcut_text("Ctrl+S")).clicked() {
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

                menu_section(ui, "Transfer");
                // Import submenu
                ui.menu_button("Import", |ui| {
                    // Mesh formats
                    for (label, exts) in &[
                        ("STL...", vec!["stl"]),
                        ("OBJ...", vec!["obj"]),
                        ("PLY...", vec!["ply"]),
                    ] {
                        if ui.button(*label).clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter(label.trim_end_matches("..."), exts)
                                .pick_file()
                            {
                                gui.status_message = format!("Importing {}", path.display());
                                gui.actions.push(GuiAction::ImportFile(path));
                            }
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    // CAD formats
                    for (label, exts) in &[
                        ("STEP...", vec!["step", "stp"]),
                        ("IGES...", vec!["iges", "igs"]),
                        ("BREP...", vec!["brep", "brp"]),
                        ("DXF...", vec!["dxf"]),
                    ] {
                        if ui.button(*label).clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter(label.trim_end_matches("..."), exts)
                                .pick_file()
                            {
                                gui.status_message = format!("Importing {}", path.display());
                                gui.actions.push(GuiAction::ImportFile(path));
                            }
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    // Scene/exchange formats
                    if ui.button("glTF...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("glTF", &["gltf", "glb"])
                            .pick_file()
                        {
                            gui.actions.push(GuiAction::ImportGltf(path));
                        }
                        ui.close_menu();
                    }
                    if ui.button("3MF...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("3MF", &["3mf"])
                            .pick_file()
                        {
                            gui.actions.push(GuiAction::Import3mf(path));
                        }
                        ui.close_menu();
                    }
                    if ui.button("SVG...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("SVG", &["svg"])
                            .pick_file()
                        {
                            gui.actions.push(GuiAction::ImportSvg(path));
                        }
                        ui.close_menu();
                    }
                    if ui.button("DAE (Collada)...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Collada", &["dae"])
                            .pick_file()
                        {
                            gui.actions.push(GuiAction::ImportDae(path));
                        }
                        ui.close_menu();
                    }
                });

                // Export submenu
                ui.menu_button("Export", |ui| {
                    // STL with options dialog
                    if ui.button("STL...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("STL", &["stl"])
                            .set_file_name("model.stl")
                            .save_file()
                        {
                            gui.export_path = Some(path);
                            gui.show_export_options = true;
                        }
                        ui.close_menu();
                    }
                    #[allow(clippy::type_complexity)]
                    let exports: &[(&str, &str, &[&str], fn(std::path::PathBuf) -> GuiAction)] = &[
                        ("OBJ...", "model.obj", &["obj"], |p| GuiAction::ExportObj(p)),
                        ("PLY...", "model.ply", &["ply"], |p| GuiAction::ExportPly(p)),
                    ];
                    for (label, filename, exts, make_action) in exports {
                        if ui.button(*label).clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter(label.trim_end_matches("..."), exts)
                                .set_file_name(*filename)
                                .save_file()
                            {
                                gui.actions.push(make_action(path));
                            }
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    #[allow(clippy::type_complexity)]
                    let cad_exports: &[(&str, &str, &[&str], fn(std::path::PathBuf) -> GuiAction)] = &[
                        ("STEP...", "model.step", &["step", "stp"], |p| GuiAction::ExportStep(p)),
                        ("IGES...", "model.iges", &["iges", "igs"], |p| GuiAction::ExportIges(p)),
                        ("DXF...", "model.dxf", &["dxf"], |p| GuiAction::ExportDxf(p)),
                        ("BREP...", "model.brep", &["brep", "brp"], |p| GuiAction::ExportBrep(p)),
                    ];
                    for (label, filename, exts, make_action) in cad_exports {
                        if ui.button(*label).clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter(label.trim_end_matches("..."), exts)
                                .set_file_name(*filename)
                                .save_file()
                            {
                                gui.actions.push(make_action(path));
                            }
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    #[allow(clippy::type_complexity)]
                    let scene_exports: &[(&str, &str, &[&str], fn(std::path::PathBuf) -> GuiAction)] = &[
                        ("glTF...", "model.gltf", &["gltf"], |p| GuiAction::ExportGltf(p)),
                        ("3MF...", "model.3mf", &["3mf"], |p| GuiAction::Export3mf(p)),
                        ("SVG...", "model.svg", &["svg"], |p| GuiAction::ExportSvg(p)),
                        ("DAE (Collada)...", "model.dae", &["dae"], |p| GuiAction::ExportDae(p)),
                    ];
                    for (label, filename, exts, make_action) in scene_exports {
                        if ui.button(*label).clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter(label.trim_end_matches("..."), exts)
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

                // Recent Files submenu
                ui.menu_button("Recent Files", |ui| {
                    ui.set_min_width(200.0);
                    if gui.recent_files.is_empty() {
                        ui.label(egui::RichText::new("No recent files").size(11.0).color(super::theme::COLOR_DIM));
                    } else {
                        let files = gui.recent_files.clone();
                        for path_str in &files {
                            let display = std::path::Path::new(path_str)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or(path_str);
                            if ui.button(display).on_hover_text(path_str.as_str()).clicked() {
                                gui.actions.push(GuiAction::OpenFile(std::path::PathBuf::from(path_str)));
                                ui.close_menu();
                            }
                        }
                        ui.separator();
                        if ui.button(egui::RichText::new("Clear Recent Files").size(11.0).color(super::theme::COLOR_DIM)).clicked() {
                            gui.actions.push(GuiAction::ClearRecentFiles);
                            ui.close_menu();
                        }
                    }
                });

                ui.separator();

                if ui.add(egui::Button::new("Quit").shortcut_text("Ctrl+Q")).clicked() {
                    gui.request_quit = true;
                }
            });

            // ---- Edit ----
            ui.menu_button("Edit", |ui| {
                menu_section(ui, "History");
                menu_action_sc(ui, gui, "Undo", "Ctrl+Z", GuiAction::Undo);
                menu_action_sc(ui, gui, "Redo", "Ctrl+Y", GuiAction::Redo);
                ui.separator();
                menu_action_sc(ui, gui, "Copy", "Ctrl+C", GuiAction::StatusMessage("Copy: select an object first".into()));
                menu_action_sc(ui, gui, "Paste", "Ctrl+V", GuiAction::StatusMessage("Paste: clipboard empty".into()));
                ui.separator();
                menu_action_sc(ui, gui, "Select All", "Ctrl+A", GuiAction::SelectAll);
                menu_action_sc(ui, gui, "Deselect All", "Esc", GuiAction::DeselectAll);
                ui.separator();
                menu_action_sc(ui, gui, "Delete", "Del", GuiAction::DeleteSelected);
                ui.separator();

                ui.menu_button("Groups", |ui| {
                    ui.set_min_width(240.0);
                    ui.weak("Group selected objects");
                    ui.horizontal(|ui| {
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut gui.group_name_input)
                                .hint_text("New group name...")
                                .desired_width(160.0),
                        );
                        let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        let create = ui.add_enabled(
                            !gui.group_name_input.trim().is_empty(),
                            egui::Button::new("Create"),
                        ).clicked();
                        if (create || enter) && !gui.group_name_input.trim().is_empty() {
                            let name = gui.group_name_input.trim().to_string();
                            gui.actions.push(GuiAction::CreateGroup(name));
                            gui.group_name_input.clear();
                            ui.close_menu();
                        }
                    });
                    ui.separator();
                    let groups: Vec<(u32, String, bool, usize)> = gui
                        .scene_groups
                        .iter()
                        .map(|(id, name, vis, count)| (*id, name.clone(), *vis, *count))
                        .collect();
                    if groups.is_empty() {
                        ui.add_space(4.0);
                        ui.weak("No groups. Select objects and\ncreate a group to manage them.");
                        ui.add_space(4.0);
                    } else {
                        for (gid, name, vis, count) in groups {
                            ui.horizontal(|ui| {
                                // Visibility eye toggle
                                let eye_icon = if vis { "\u{25C9}" } else { "\u{25CB}" };
                                let eye_color = if vis {
                                    egui::Color32::from_rgb(100, 200, 120)
                                } else {
                                    egui::Color32::from_gray(100)
                                };
                                if ui.add(egui::Button::new(
                                    egui::RichText::new(eye_icon).color(eye_color).size(14.0)
                                ).frame(false)).on_hover_text(
                                    if vis { "Hide group" } else { "Show group" }
                                ).clicked() {
                                    gui.actions.push(GuiAction::ToggleGroupVisibility(gid));
                                }
                                // Group name + member count
                                let label = format!("{name} ({count})");
                                if ui.button(&label)
                                    .on_hover_text("Add selected objects to this group")
                                    .clicked()
                                {
                                    gui.actions.push(GuiAction::GroupSelected(gid));
                                    ui.close_menu();
                                }
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.small_button(
                                        egui::RichText::new("\u{2715}").weak().size(11.0)
                                    ).on_hover_text("Delete group (keeps objects)").clicked() {
                                        gui.actions.push(GuiAction::DeleteGroup(gid));
                                        ui.close_menu();
                                    }
                                });
                            });
                        }
                    }
                });
                ui.separator();
                if ui.button("Settings...").clicked() {
                    gui.show_settings = true;
                    ui.close_menu();
                }
            });

            // ---- Create ----
            ui.menu_button("Create", |ui| {
                menu_section(ui, "Basic Primitives");
                if ui.button("Box...").clicked() {
                    gui.active_task = Some(task_panel::ActiveTask::Box { width: 10.0, height: 10.0, depth: 10.0, preview_id: None });
                    ui.close_menu();
                }
                if ui.button("Cylinder...").clicked() {
                    gui.active_task = Some(task_panel::ActiveTask::Cylinder { radius: 5.0, height: 10.0, preview_id: None });
                    ui.close_menu();
                }
                if ui.button("Sphere...").clicked() {
                    gui.active_task = Some(task_panel::ActiveTask::Sphere { radius: 5.0, preview_id: None });
                    ui.close_menu();
                }
                if ui.button("Cone...").clicked() {
                    gui.active_task = Some(task_panel::ActiveTask::Cone { base_radius: 5.0, top_radius: 0.0, height: 10.0, preview_id: None });
                    ui.close_menu();
                }
                if ui.button("Torus...").clicked() {
                    gui.active_task = Some(task_panel::ActiveTask::Torus { major_radius: 5.0, minor_radius: 1.5, preview_id: None });
                    ui.close_menu();
                }
                ui.separator();
                menu_section(ui, "Extended Primitives");
                if ui.button("Tube...").clicked() {
                    gui.active_task = Some(task_panel::ActiveTask::Tube { outer_radius: 5.0, inner_radius: 3.0, height: 10.0, preview_id: None });
                    ui.close_menu();
                }
                if ui.button("Prism...").clicked() {
                    gui.active_task = Some(task_panel::ActiveTask::Prism { radius: 5.0, height: 10.0, sides: 6, preview_id: None });
                    ui.close_menu();
                }
                if ui.button("Wedge...").clicked() {
                    gui.active_task = Some(task_panel::ActiveTask::Wedge { dx: 10.0, dy: 10.0, dz: 10.0, dx2: 5.0, dy2: 5.0, preview_id: None });
                    ui.close_menu();
                }
                if ui.button("Ellipsoid...").clicked() {
                    gui.active_task = Some(task_panel::ActiveTask::Ellipsoid { rx: 5.0, ry: 3.0, rz: 2.0, preview_id: None });
                    ui.close_menu();
                }
                if ui.button("Helix...").clicked() {
                    gui.active_task = Some(task_panel::ActiveTask::Helix { radius: 5.0, pitch: 3.0, turns: 3.0, tube_radius: 0.5, preview_id: None });
                    ui.close_menu();
                }
            });

            // ---- Macro ----
            ui.menu_button("Macro", |ui| {
                menu_action(ui, gui, "Run Script...", GuiAction::StatusMessage("Python scripting: coming soon".into()));
                ui.separator();
                ui.add_enabled_ui(false, |ui| {
                    let _ = ui.button("Macro Console");
                    let _ = ui.button("Start Recording");
                    let _ = ui.button("Stop Recording");
                });
            });

            // ---- View ----
            ui.menu_button("View", |ui| {
                menu_section(ui, "Layout");
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
                    Projection::Perspective => "Switch to Orthographic",
                    Projection::Orthographic => "Switch to Perspective",
                };
                if ui.add(egui::Button::new(proj_label).shortcut_text("5")).clicked() {
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
                        if ui.add(egui::Button::new(view.label()).shortcut_text(key)).clicked() {
                            gui.actions.push(GuiAction::SetStandardView(view));
                            ui.close_menu();
                        }
                    }
                });

                ui.separator();
                menu_section(ui, "Overlays");
                menu_action_sc(ui, gui, "Toggle Grid", "G", GuiAction::ToggleGrid);
                menu_action(ui, gui, "Toggle Origin", GuiAction::ToggleOrigin);
                menu_action(ui, gui, "Toggle 3D Grid", GuiAction::ToggleGrid3d);
                menu_action_sc(ui, gui, "Section Plane", "Shift+S", GuiAction::ToggleSectionPlane);

                ui.separator();

                // -- View Bookmarks --
                ui.menu_button("Bookmarks", |ui| {
                    ui.set_min_width(220.0);
                    ui.weak("Save current view");
                    ui.horizontal(|ui| {
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut gui.bookmark_name_input)
                                .hint_text("Bookmark name...")
                                .desired_width(150.0),
                        );
                        let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        let save = ui.add_enabled(
                            !gui.bookmark_name_input.trim().is_empty(),
                            egui::Button::new("Save"),
                        ).clicked();
                        if (save || enter) && !gui.bookmark_name_input.trim().is_empty() {
                            let name = gui.bookmark_name_input.trim().to_string();
                            gui.actions.push(GuiAction::SaveBookmark(name));
                            gui.bookmark_name_input.clear();
                            ui.close_menu();
                        }
                    });
                    ui.separator();
                    let bookmarks: Vec<(usize, String, f32, f32, f32)> = gui
                        .nav_bookmarks
                        .iter()
                        .enumerate()
                        .map(|(i, (name, yaw, pitch, dist))| (i, name.clone(), *yaw, *pitch, *dist))
                        .collect();
                    if bookmarks.is_empty() {
                        ui.add_space(4.0);
                        ui.weak("No bookmarks saved yet.\nSave a view to quickly return to it later.");
                        ui.add_space(4.0);
                    } else {
                        ui.weak(format!("{}/20 bookmarks", bookmarks.len()));
                        ui.add_space(2.0);
                        for (idx, name, yaw_d, pitch_d, dist) in bookmarks {
                            ui.horizontal(|ui| {
                                // Numbered label
                                ui.label(
                                    egui::RichText::new(format!("{}.", idx + 1))
                                        .weak()
                                        .size(11.0),
                                );
                                let tooltip = format!(
                                    "Yaw: {:.1}\u{00B0}  Pitch: {:.1}\u{00B0}  Dist: {:.1}",
                                    yaw_d, pitch_d, dist,
                                );
                                if ui.button(&name).on_hover_text(&tooltip).clicked() {
                                    gui.actions.push(GuiAction::RestoreBookmark(idx));
                                    ui.close_menu();
                                }
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.small_button(
                                        egui::RichText::new("\u{2715}").weak().size(11.0)
                                    ).on_hover_text("Delete bookmark").clicked() {
                                        gui.actions.push(GuiAction::DeleteBookmark(idx));
                                        ui.close_menu();
                                    }
                                });
                            });
                        }
                    }
                });

                ui.separator();
                menu_section(ui, "Camera");
                menu_action(ui, gui, "Reset Camera", GuiAction::ResetCamera);
                menu_action_sc(ui, gui, "Fit All", "V", GuiAction::FitAll);
            });

            // ---- Workbench ----
            ui.menu_button("Workbench", |ui| {
                for &wb in super::Workbench::ALL {
                    let selected = gui.active_workbench == wb;
                    if ui.selectable_label(selected, wb.label()).clicked() {
                        gui.active_workbench = wb;
                        ui.close_menu();
                    }
                }
            });

            // ---- Tools ----
            ui.menu_button("Tools", |ui| {
                menu_section(ui, "Analysis");
                menu_action(ui, gui, "Check Geometry", GuiAction::CheckGeometry);
                menu_action(ui, gui, "Mass Properties", GuiAction::MeasureSolid);
                ui.separator();
                menu_section(ui, "Measurement");
                menu_action(ui, gui, "Measure Distance", GuiAction::ToggleMeasurement);
                menu_action(ui, gui, "Clear Measurement", GuiAction::ClearMeasurement);
                ui.separator();

                // -- Scripting submenu --
                ui.menu_button("\u{1F4DC} Scripting", |ui| {
                    if ui.button("Run Lua Script\u{2026}").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Lua scripts", &["lua"])
                            .pick_file()
                        {
                            gui.actions.push(GuiAction::ExecuteLuaFile(
                                path.to_string_lossy().to_string(),
                            ));
                        }
                        ui.close_menu();
                    }
                    if ui.button("Clear Lua Console").clicked() {
                        gui.actions.push(GuiAction::ClearLuaConsole);
                        ui.close_menu();
                    }
                });

                // -- Plugins submenu --
                ui.menu_button("\u{1F9E9} Plugins", |ui| {
                    if ui.button("Plugin Manager\u{2026}").clicked() {
                        gui.actions.push(GuiAction::TogglePluginManager);
                        ui.close_menu();
                    }
                    if ui.button("Initialize Plugins").clicked() {
                        gui.actions.push(GuiAction::InitPlugins);
                        ui.close_menu();
                    }
                });

                // -- MCP submenu --
                ui.menu_button("\u{1F310} MCP", |ui| {
                    let label = if gui.mcp_running {
                        "Stop MCP Server"
                    } else {
                        "Start MCP Server"
                    };
                    if ui.button(label).clicked() {
                        if gui.mcp_running {
                            gui.actions.push(GuiAction::StopMcpServer);
                        } else {
                            gui.actions.push(GuiAction::StartMcpServer);
                        }
                        ui.close_menu();
                    }
                });
            });

            // ---- Workbench-specific menus ----
            match gui.active_workbench {
                Workbench::Part => draw_part_menu(ui, gui),
                Workbench::PartDesign => draw_part_design_menu(ui, gui),
                Workbench::Sketcher => draw_sketch_menu(ui, gui),
                Workbench::Mesh => draw_mesh_menu(ui, gui),
                Workbench::TechDraw => draw_techdraw_menu(ui, gui),
                Workbench::Assembly => draw_assembly_menu(ui, gui),
                Workbench::Draft => draw_draft_menu(ui, gui),
                Workbench::Surface => draw_surface_menu(ui, gui),
                Workbench::Fem => draw_fem_menu(ui, gui),
            }
            // Also show Sketch menu when in sketch mode (regardless of workbench)
            if gui.sketch_mode.is_some() && gui.active_workbench != Workbench::Sketcher {
                draw_sketch_menu(ui, gui);
            }

            // ---- Help ----
            ui.menu_button("Help", |ui| {
                if ui.button("About CADKernel").clicked() {
                    gui.show_about = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Keyboard Shortcuts  (?)").clicked() {
                    gui.show_shortcuts = true;
                    ui.close_menu();
                }
            });
        });
    });
}

// ---------------------------------------------------------------------------
// Workbench-specific menu functions
// ---------------------------------------------------------------------------

fn menu_action(ui: &mut egui::Ui, gui: &mut GuiState, label: &str, action: GuiAction) {
    if ui.button(label).clicked() {
        gui.actions.push(action);
        ui.close_menu();
    }
}

fn menu_action_sc(ui: &mut egui::Ui, gui: &mut GuiState, label: &str, shortcut: &str, action: GuiAction) {
    if ui.add(egui::Button::new(label).shortcut_text(shortcut)).clicked() {
        gui.actions.push(action);
        ui.close_menu();
    }
}

/// Dim section header label for menus (FreeCAD-style).
fn menu_section(ui: &mut egui::Ui, label: &str) {
    ui.label(
        egui::RichText::new(label)
            .size(10.0)
            .color(super::theme::MENU_SECTION_COLOR)
            .strong(),
    );
}

fn draw_part_menu(ui: &mut egui::Ui, gui: &mut GuiState) {
    ui.menu_button("Part", |ui| {
        ui.menu_button("Primitives", |ui| {
            if ui.button("Box...").clicked() { gui.active_task = Some(task_panel::ActiveTask::Box { width: 10.0, height: 10.0, depth: 10.0, preview_id: None }); ui.close_menu(); }
            if ui.button("Cylinder...").clicked() { gui.active_task = Some(task_panel::ActiveTask::Cylinder { radius: 5.0, height: 10.0, preview_id: None }); ui.close_menu(); }
            if ui.button("Sphere...").clicked() { gui.active_task = Some(task_panel::ActiveTask::Sphere { radius: 5.0, preview_id: None }); ui.close_menu(); }
            if ui.button("Cone...").clicked() { gui.active_task = Some(task_panel::ActiveTask::Cone { base_radius: 5.0, top_radius: 0.0, height: 10.0, preview_id: None }); ui.close_menu(); }
            if ui.button("Torus...").clicked() { gui.active_task = Some(task_panel::ActiveTask::Torus { major_radius: 5.0, minor_radius: 1.5, preview_id: None }); ui.close_menu(); }
            ui.separator();
            if ui.button("Tube...").clicked() { gui.active_task = Some(task_panel::ActiveTask::Tube { outer_radius: 5.0, inner_radius: 3.0, height: 10.0, preview_id: None }); ui.close_menu(); }
            if ui.button("Prism...").clicked() { gui.active_task = Some(task_panel::ActiveTask::Prism { radius: 5.0, height: 10.0, sides: 6, preview_id: None }); ui.close_menu(); }
            if ui.button("Wedge...").clicked() { gui.active_task = Some(task_panel::ActiveTask::Wedge { dx: 10.0, dy: 10.0, dz: 10.0, dx2: 5.0, dy2: 5.0, preview_id: None }); ui.close_menu(); }
            if ui.button("Ellipsoid...").clicked() { gui.active_task = Some(task_panel::ActiveTask::Ellipsoid { rx: 5.0, ry: 3.0, rz: 2.0, preview_id: None }); ui.close_menu(); }
            if ui.button("Helix...").clicked() { gui.active_task = Some(task_panel::ActiveTask::Helix { radius: 5.0, pitch: 3.0, turns: 3.0, tube_radius: 0.5, preview_id: None }); ui.close_menu(); }
        });
        ui.separator();
        ui.menu_button("Boolean", |ui| {
            menu_action(ui, gui, "Union", GuiAction::BooleanSceneUnion);
            menu_action(ui, gui, "Subtract", GuiAction::BooleanSceneSubtract);
            menu_action(ui, gui, "Intersect", GuiAction::BooleanSceneIntersect);
            ui.separator();
            menu_action(ui, gui, "Boolean Fragments", GuiAction::BooleanFragments);
            menu_action(ui, gui, "Slice to Compound", GuiAction::SliceToCompound);
        });
        ui.menu_button("Join", |ui| {
            menu_action(ui, gui, "Face from Wires", GuiAction::FaceFromWires);
            menu_action(ui, gui, "Connect Shapes", GuiAction::ConnectShapes);
            menu_action(ui, gui, "Embed Shapes", GuiAction::EmbedShapes);
            menu_action(ui, gui, "Cutout Shapes", GuiAction::CutoutShapes);
        });
        ui.menu_button("Compound", |ui| {
            menu_action(ui, gui, "Explode Compound", GuiAction::ExplodeCompound);
            menu_action(ui, gui, "Compound Filter", GuiAction::CompoundFilter);
        });
        ui.separator();
        ui.menu_button("Transform", |ui| {
            menu_action(ui, gui, "Mirror XY", GuiAction::MirrorSolid(super::MirrorPlane::XY));
            menu_action(ui, gui, "Mirror XZ", GuiAction::MirrorSolid(super::MirrorPlane::XZ));
            menu_action(ui, gui, "Mirror YZ", GuiAction::MirrorSolid(super::MirrorPlane::YZ));
            ui.separator();
            menu_action(ui, gui, "Scale...", GuiAction::ScaleSolid { factor: 2.0 });
            menu_action(ui, gui, "Transformed Copy...", GuiAction::TransformedCopy { dx: 10.0, dy: 0.0, dz: 0.0 });
            ui.separator();
            menu_action(ui, gui, "Linear Pattern X", GuiAction::LinearPattern { count: 3, spacing: 10.0, axis: 0 });
            menu_action(ui, gui, "Linear Pattern Y", GuiAction::LinearPattern { count: 3, spacing: 10.0, axis: 1 });
            menu_action(ui, gui, "Linear Pattern Z", GuiAction::LinearPattern { count: 3, spacing: 10.0, axis: 2 });
        });
        ui.menu_button("Convert", |ui| {
            menu_action(ui, gui, "Points from Shape", GuiAction::PointsFromShape);
            menu_action(ui, gui, "Convert to Solid", GuiAction::ConvertToSolid);
            menu_action(ui, gui, "Auto Defeature...", GuiAction::AutoDefeaturing { threshold: 1.0 });
            menu_action(ui, gui, "Project Curves on Surface", GuiAction::ProjectCurvesOnSurface);
            menu_action(ui, gui, "Coons Patch", GuiAction::CoonsPatch);
        });
        ui.separator();
        ui.menu_button("Features", |ui| {
            menu_action(ui, gui, "Fillet All Edges...", GuiAction::FilletAllEdges { radius: 1.0 });
            menu_action(ui, gui, "Chamfer All Edges...", GuiAction::ChamferAllEdges { distance: 1.0 });
            menu_action(ui, gui, "Shell...", GuiAction::ShellSolid { thickness: 1.0 });
        });
    });
}

fn draw_part_design_menu(ui: &mut egui::Ui, gui: &mut GuiState) {
    ui.menu_button("PartDesign", |ui| {
        ui.menu_button("Additive", |ui| {
            menu_action_sc(ui, gui, "Pad", "P", GuiAction::PadSketch { depth: 10.0, symmetric: false });
            menu_action(ui, gui, "Additive Loft", GuiAction::AdditiveLoft);
            menu_action(ui, gui, "Additive Pipe", GuiAction::AdditivePipe);
            ui.separator();
            if ui.button("Additive Box...").clicked() { gui.active_task = Some(task_panel::ActiveTask::Box { width: 10.0, height: 10.0, depth: 10.0, preview_id: None }); ui.close_menu(); }
            if ui.button("Additive Cylinder...").clicked() { gui.active_task = Some(task_panel::ActiveTask::Cylinder { radius: 5.0, height: 10.0, preview_id: None }); ui.close_menu(); }
            if ui.button("Additive Sphere...").clicked() { gui.active_task = Some(task_panel::ActiveTask::Sphere { radius: 5.0, preview_id: None }); ui.close_menu(); }
            if ui.button("Additive Cone...").clicked() { gui.active_task = Some(task_panel::ActiveTask::Cone { base_radius: 5.0, top_radius: 0.0, height: 10.0, preview_id: None }); ui.close_menu(); }
            if ui.button("Additive Torus...").clicked() { gui.active_task = Some(task_panel::ActiveTask::Torus { major_radius: 5.0, minor_radius: 1.5, preview_id: None }); ui.close_menu(); }
        });
        ui.menu_button("Subtractive", |ui| {
            menu_action(ui, gui, "Pocket", GuiAction::PocketSketch { depth: 10.0, through_all: false });
            menu_action(ui, gui, "Groove", GuiAction::GrooveSketch { angle: 360.0 });
            menu_action(ui, gui, "Subtractive Loft", GuiAction::SubtractiveLoft);
            menu_action(ui, gui, "Subtractive Pipe", GuiAction::SubtractivePipe);
            ui.separator();
            menu_action(ui, gui, "Hole...", GuiAction::HoleSketch { radius: 5.0, depth: 10.0 });
            menu_action(ui, gui, "Countersunk Hole...", GuiAction::CountersunkHoleSketch {
                radius: 5.0, depth: 10.0, countersink_angle: 90.0,
            });
        });
        ui.separator();
        ui.menu_button("Features", |ui| {
            menu_action(ui, gui, "Fillet All...", GuiAction::FilletAllEdges { radius: 1.0 });
            menu_action(ui, gui, "Chamfer All...", GuiAction::ChamferAllEdges { distance: 1.0 });
            menu_action(ui, gui, "Shell...", GuiAction::ShellSolid { thickness: 1.0 });
        });
        ui.menu_button("Pattern", |ui| {
            menu_action(ui, gui, "Linear Pattern X", GuiAction::LinearPattern { count: 3, spacing: 10.0, axis: 0 });
            menu_action(ui, gui, "Linear Pattern Y", GuiAction::LinearPattern { count: 3, spacing: 10.0, axis: 1 });
            menu_action(ui, gui, "Linear Pattern Z", GuiAction::LinearPattern { count: 3, spacing: 10.0, axis: 2 });
            ui.separator();
            menu_action(ui, gui, "Mirror XY", GuiAction::MirrorSolid(super::MirrorPlane::XY));
            menu_action(ui, gui, "Mirror XZ", GuiAction::MirrorSolid(super::MirrorPlane::XZ));
            menu_action(ui, gui, "Mirror YZ", GuiAction::MirrorSolid(super::MirrorPlane::YZ));
        });
        ui.separator();
        ui.menu_button("Mechanical", |ui| {
            menu_action(ui, gui, "Involute Gear...", GuiAction::CreateInvoluteGear {
                teeth: 20, module_val: 2.0, pressure_angle: 20.0,
            });
            menu_action(ui, gui, "Sprocket...", GuiAction::CreateSprocket {
                teeth: 15, roller_diameter: 8.0, pitch: 12.7, bore: 10.0,
            });
            menu_action(ui, gui, "Shaft Design...", GuiAction::CreateShaftDesign {
                segments: vec![(10.0, 5.0), (20.0, 8.0), (10.0, 5.0)],
            });
        });
        ui.separator();
        ui.menu_button("Body", |ui| {
            menu_action(ui, gui, "Shape Binder", GuiAction::ShapeBinder);
            menu_action(ui, gui, "Suppress Feature", GuiAction::SuppressFeature);
            menu_action(ui, gui, "Set Tip", GuiAction::SetTip);
            menu_action(ui, gui, "Move Feature Up", GuiAction::MoveFeatureUp);
            menu_action(ui, gui, "Move Feature Down", GuiAction::MoveFeatureDown);
        });
    });
}

fn draw_sketch_menu(ui: &mut egui::Ui, gui: &mut GuiState) {
    ui.menu_button("Sketch", |ui| {
        let in_sketch = gui.sketch_mode.is_some();
        ui.menu_button("Geometry", |ui| {
            menu_action_sc(ui, gui, "Select", "S", GuiAction::SetSketchTool(SketchTool::Select));
            ui.separator();
            menu_action(ui, gui, "Point", GuiAction::SetSketchTool(SketchTool::Point));
            menu_action_sc(ui, gui, "Line", "L", GuiAction::SetSketchTool(SketchTool::Line));
            menu_action_sc(ui, gui, "Rectangle", "R", GuiAction::SetSketchTool(SketchTool::Rectangle));
            menu_action_sc(ui, gui, "Circle", "C", GuiAction::SetSketchTool(SketchTool::Circle));
            menu_action_sc(ui, gui, "Arc", "A", GuiAction::SetSketchTool(SketchTool::Arc));
            menu_action_sc(ui, gui, "Ellipse", "E", GuiAction::SetSketchTool(SketchTool::Ellipse));
            menu_action_sc(ui, gui, "Polyline", "W", GuiAction::SetSketchTool(SketchTool::Polyline));
            menu_action(ui, gui, "Slot", GuiAction::SetSketchTool(SketchTool::Slot));
            menu_action_sc(ui, gui, "B-Spline", "B", GuiAction::SetSketchTool(SketchTool::BSpline));
            menu_action_sc(ui, gui, "Polygon", "N", GuiAction::SetSketchTool(SketchTool::Polygon { sides: 6 }));
        });
        ui.menu_button("Constraints", |ui| {
            menu_section(ui, "Geometric");
            menu_action_sc(ui, gui, "Horizontal", "H", GuiAction::SketchConstrainHorizontal);
            menu_action_sc(ui, gui, "Vertical", "V", GuiAction::SketchConstrainVertical);
            menu_action(ui, gui, "Parallel", GuiAction::SketchConstrainParallel);
            menu_action(ui, gui, "Perpendicular", GuiAction::SketchConstrainPerpendicular);
            menu_action(ui, gui, "Coincident", GuiAction::SketchConstrainCoincident);
            menu_action(ui, gui, "Tangent", GuiAction::SketchConstrainTangent);
            menu_action(ui, gui, "Equal", GuiAction::SketchConstrainEqual);
            menu_action(ui, gui, "Symmetric", GuiAction::SketchConstrainSymmetric);
            menu_action_sc(ui, gui, "Fixed", "F", GuiAction::SketchConstrainFixed);
            menu_action(ui, gui, "Block", GuiAction::SketchConstrainBlock);
            ui.separator();
            menu_section(ui, "Dimensional");
            menu_action(ui, gui, "Length...", GuiAction::SketchConstrainLength(gui.constraint_length_value));
            menu_action(ui, gui, "Distance...", GuiAction::SketchConstrainDistance(gui.constraint_distance_value));
            menu_action(ui, gui, "Angle...", GuiAction::SketchConstrainAngle(gui.constraint_angle_value));
            menu_action(ui, gui, "Radius...", GuiAction::SketchConstrainRadius(gui.constraint_radius_value));
            menu_action(ui, gui, "Diameter...", GuiAction::SketchConstrainDiameter(gui.constraint_radius_value * 2.0));
            menu_action(ui, gui, "H Distance...", GuiAction::SketchConstrainHDistance(gui.constraint_distance_value));
            menu_action(ui, gui, "V Distance...", GuiAction::SketchConstrainVDistance(gui.constraint_distance_value));
        });
        ui.menu_button("Tools", |ui| {
            menu_action(ui, gui, "Fillet Corner...", GuiAction::SketchFilletCorner { radius: gui.sketch_fillet_radius });
            menu_action(ui, gui, "Chamfer Corner...", GuiAction::SketchChamferCorner { distance: gui.sketch_chamfer_distance });
            ui.separator();
            menu_action(ui, gui, "Trim Edge", GuiAction::SketchTrimEdge);
            menu_action(ui, gui, "Split Edge", GuiAction::SketchSplitEdge);
            menu_action(ui, gui, "Extend Edge", GuiAction::SketchExtendEdge);
            ui.separator();
            menu_action(ui, gui, "Mirror Geometry", GuiAction::SketchMirrorGeometry);
            menu_action(ui, gui, "External Projection", GuiAction::SketchExternalProjection);
            menu_action(ui, gui, "Carbon Copy", GuiAction::SketchCarbonCopy);
        });
        ui.menu_button("B-Spline", |ui| {
            menu_action_sc(ui, gui, "B-Spline Tool", "B", GuiAction::SetSketchTool(SketchTool::BSpline));
            ui.separator();
            menu_action(ui, gui, "Convert to B-Spline", GuiAction::SketchConvertToBSpline);
            menu_action(ui, gui, "Increase Degree", GuiAction::SketchIncreaseDegree);
            menu_action(ui, gui, "Decrease Degree", GuiAction::SketchDecreaseDegree);
            menu_action(ui, gui, "Insert Knot", GuiAction::SketchInsertKnot);
        });
        ui.separator();
        ui.menu_button("Toggles", |ui| {
            menu_action_sc(ui, gui, "Construction Mode", "X", GuiAction::ToggleSketchConstruction);
            menu_action_sc(ui, gui, "Grid", "G", GuiAction::ToggleSketchGrid);
            menu_action_sc(ui, gui, "Snap", "Shift+S", GuiAction::ToggleSketchSnap);
            menu_action(ui, gui, "Show Constraints", GuiAction::ToggleSketchConstraintsVisible);
        });
        ui.separator();
        if in_sketch {
            menu_action_sc(ui, gui, "Close Sketch", "Enter", GuiAction::CloseSketch);
            menu_action_sc(ui, gui, "Cancel Sketch", "Esc", GuiAction::CancelSketch);
        } else {
            menu_action(ui, gui, "New Sketch (XY)", GuiAction::EnterSketch(
                cadkernel_sketch::WorkPlane::xy()
            ));
            menu_action(ui, gui, "New Sketch (XZ)", GuiAction::EnterSketch(
                cadkernel_sketch::WorkPlane::xz()
            ));
            menu_action(ui, gui, "New Sketch (YZ)", GuiAction::EnterSketch(
                cadkernel_sketch::WorkPlane::new(
                    cadkernel_math::Point3::ORIGIN,
                    cadkernel_math::Vec3::X,
                    cadkernel_math::Vec3::Y,
                )
            ));
        }
    });
}

fn draw_mesh_menu(ui: &mut egui::Ui, gui: &mut GuiState) {
    ui.menu_button("Mesh", |ui| {
        ui.menu_button("Import/Export", |ui| {
            for (label, filter, exts) in &[
                ("Import STL...", "STL", &["stl"][..]),
                ("Import OBJ...", "OBJ", &["obj"][..]),
                ("Import PLY...", "PLY", &["ply"][..]),
            ] {
                if ui.button(*label).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter(*filter, exts)
                        .pick_file()
                    {
                        gui.actions.push(GuiAction::ImportFile(path));
                    }
                    ui.close_menu();
                }
            }
            if ui.button("Import glTF...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("glTF", &["gltf", "glb"])
                    .pick_file()
                {
                    gui.actions.push(GuiAction::ImportGltf(path));
                }
                ui.close_menu();
            }
            ui.separator();
            for (label, name, filter, exts, make) in [
                ("Export STL...", "model.stl", "STL", &["stl"][..], GuiAction::ExportStl as fn(std::path::PathBuf) -> GuiAction),
                ("Export OBJ...", "model.obj", "OBJ", &["obj"][..], GuiAction::ExportObj),
                ("Export PLY...", "model.ply", "PLY", &["ply"][..], GuiAction::ExportPly),
            ] {
                if ui.button(label).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter(filter, exts)
                        .set_file_name(name)
                        .save_file()
                    {
                        gui.actions.push(make(path));
                    }
                    ui.close_menu();
                }
            }
            if ui.button("Export glTF...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("glTF", &["gltf"])
                    .set_file_name("model.gltf")
                    .save_file()
                {
                    gui.actions.push(GuiAction::ExportGltf(path));
                }
                ui.close_menu();
            }
        });
        ui.separator();
        ui.menu_button("Modify", |ui| {
            menu_action(ui, gui, "Decimate (50%)", GuiAction::MeshDecimate(0.5));
            menu_action(ui, gui, "Subdivide", GuiAction::MeshSubdivide);
            menu_action(ui, gui, "Smooth", GuiAction::MeshSmooth { iterations: 3, factor: 0.5 });
            menu_action(ui, gui, "Fill Holes", GuiAction::MeshFillHoles);
            menu_action(ui, gui, "Remesh...", GuiAction::MeshRemesh { target_edge_len: 1.0 });
            menu_action(ui, gui, "Repair", GuiAction::MeshRepair);
        });
        ui.menu_button("Analyze", |ui| {
            menu_action(ui, gui, "Check Watertight", GuiAction::MeshCheckWatertight);
            menu_action(ui, gui, "Harmonize Normals", GuiAction::MeshHarmonizeNormals);
            menu_action(ui, gui, "Flip Normals", GuiAction::MeshFlipNormals);
            menu_action(ui, gui, "Check Geometry", GuiAction::CheckGeometry);
        });
    });
}

fn draw_techdraw_menu(ui: &mut egui::Ui, gui: &mut GuiState) {
    ui.menu_button("TechDraw", |ui| {
        ui.menu_button("Page", |ui| {
            menu_action(ui, gui, "New Page", GuiAction::TechDrawNewPage);
            menu_action(ui, gui, "From Template...", GuiAction::TechDrawFromTemplate);
            menu_action(ui, gui, "Redraw Page", GuiAction::TechDrawRedraw);
        });
        ui.menu_button("Views", |ui| {
            menu_action(ui, gui, "Front View", GuiAction::TechDrawAddView(cadkernel_io::ProjectionDir::Front));
            menu_action(ui, gui, "Top View", GuiAction::TechDrawAddView(cadkernel_io::ProjectionDir::Top));
            menu_action(ui, gui, "Right View", GuiAction::TechDrawAddView(cadkernel_io::ProjectionDir::Right));
            menu_action(ui, gui, "Isometric View", GuiAction::TechDrawAddView(cadkernel_io::ProjectionDir::Isometric));
            ui.separator();
            menu_action(ui, gui, "3-View Layout", GuiAction::TechDrawThreeView);
            menu_action(ui, gui, "Section View", GuiAction::TechDrawSectionView);
            menu_action(ui, gui, "Detail View", GuiAction::TechDrawDetailView);
            menu_action(ui, gui, "Broken View", GuiAction::TechDrawBrokenView);
        });
        ui.separator();
        ui.menu_button("Dimensions", |ui| {
            menu_action(ui, gui, "Linear Dimension", GuiAction::TechDrawDimLinear);
            menu_action(ui, gui, "Radius Dimension", GuiAction::TechDrawDimRadius);
            menu_action(ui, gui, "Diameter Dimension", GuiAction::TechDrawDimDiameter);
            menu_action(ui, gui, "Angle Dimension", GuiAction::TechDrawDimAngle);
            menu_action(ui, gui, "Arc Length", GuiAction::TechDrawDimArcLen);
            menu_action(ui, gui, "Area", GuiAction::TechDrawDimArea);
        });
        ui.menu_button("Annotations", |ui| {
            menu_action(ui, gui, "Text", GuiAction::TechDrawText);
            menu_action(ui, gui, "Rich Text", GuiAction::TechDrawRichText);
            menu_action(ui, gui, "Balloon", GuiAction::TechDrawBalloon);
            menu_action(ui, gui, "Leader Line", GuiAction::TechDrawLeader);
            menu_action(ui, gui, "Weld Symbol", GuiAction::TechDrawWeld);
            menu_action(ui, gui, "Surface Finish", GuiAction::TechDrawSurfFinish);
        });
        ui.menu_button("Centerlines", |ui| {
            menu_action(ui, gui, "Center on Face", GuiAction::TechDrawCenterFace);
            menu_action(ui, gui, "Center Lines", GuiAction::TechDrawCenterLines);
            menu_action(ui, gui, "Center Points", GuiAction::TechDrawCenterPoints);
            menu_action(ui, gui, "Bolt Circle", GuiAction::TechDrawBoltCircle);
        });
        ui.separator();
        ui.menu_button("Export", |ui| {
            if ui.button("Export SVG...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("SVG", &["svg"])
                    .set_file_name("drawing.svg")
                    .save_file()
                {
                    gui.actions.push(GuiAction::TechDrawExportSvg(path));
                }
                ui.close_menu();
            }
            if ui.button("Export DXF...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("DXF", &["dxf"])
                    .set_file_name("drawing.dxf")
                    .save_file()
                {
                    gui.actions.push(GuiAction::TechDrawExportDxf(path));
                }
                ui.close_menu();
            }
            if ui.button("Export PDF...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("PDF", &["pdf"])
                    .set_file_name("drawing.pdf")
                    .save_file()
                {
                    gui.actions.push(GuiAction::TechDrawExportPdf(path));
                }
                ui.close_menu();
            }
        });
        ui.separator();
        menu_action(ui, gui, "Clear All", GuiAction::TechDrawClear);
    });
}

fn draw_assembly_menu(ui: &mut egui::Ui, gui: &mut GuiState) {
    ui.menu_button("Assembly", |ui| {
        menu_action(ui, gui, "Create Assembly", GuiAction::CreateAssembly);
        menu_action(ui, gui, "Insert Component", GuiAction::InsertComponent);
        ui.separator();
        menu_action(
            ui,
            gui,
            "Ground Component",
            GuiAction::AddAssemblyJoint(AssemblyJointType::Grounded),
        );
        ui.menu_button("Joints", |ui| {
            for jt in [
                AssemblyJointType::Fixed,
                AssemblyJointType::Revolute,
                AssemblyJointType::Cylindrical,
                AssemblyJointType::Slider,
                AssemblyJointType::Ball,
                AssemblyJointType::Distance,
                AssemblyJointType::Angle,
                AssemblyJointType::Parallel,
                AssemblyJointType::Perpendicular,
                AssemblyJointType::Gear,
                AssemblyJointType::Rack,
                AssemblyJointType::Screw,
                AssemblyJointType::Belt,
            ] {
                menu_action(ui, gui, jt.label(), GuiAction::AddAssemblyJoint(jt));
            }
        });
        ui.separator();
        menu_action(ui, gui, "Solve Assembly", GuiAction::SolveAssembly);
        menu_action(ui, gui, "Exploded View", GuiAction::ExplodedView { factor: 2.0 });
        menu_action(ui, gui, "Bill of Materials", GuiAction::BillOfMaterials);
        menu_action(ui, gui, "DOF Analysis", GuiAction::DOFAnalysis);
    });
}

fn draw_draft_menu(ui: &mut egui::Ui, gui: &mut GuiState) {
    ui.menu_button("Draft", |ui| {
        ui.menu_button("Drawing", |ui| {
            menu_action(ui, gui, "Line", GuiAction::DraftLine);
            menu_action(ui, gui, "Wire", GuiAction::DraftWire);
            menu_action(ui, gui, "Circle", GuiAction::DraftCircle);
            menu_action(ui, gui, "Arc", GuiAction::DraftArc);
            menu_action(ui, gui, "Ellipse", GuiAction::DraftEllipse);
            menu_action(ui, gui, "Rectangle", GuiAction::DraftRectangle);
            menu_action(ui, gui, "Polygon", GuiAction::DraftPolygon);
            menu_action(ui, gui, "B-Spline", GuiAction::DraftBSpline);
            menu_action(ui, gui, "Bezier", GuiAction::DraftBezier);
            menu_action(ui, gui, "Point", GuiAction::DraftPoint);
            menu_action(ui, gui, "Facebinder", GuiAction::DraftFacebinder);
            menu_action(ui, gui, "Hatch", GuiAction::DraftHatch);
        });
        ui.menu_button("Modification", |ui| {
            menu_action(ui, gui, "Move", GuiAction::DraftMove);
            menu_action(ui, gui, "Rotate", GuiAction::DraftRotate);
            menu_action(ui, gui, "Scale", GuiAction::DraftScale);
            menu_action(ui, gui, "Mirror", GuiAction::DraftMirror);
            menu_action(ui, gui, "Offset", GuiAction::DraftOffset);
            menu_action(ui, gui, "Trim", GuiAction::DraftTrim);
            menu_action(ui, gui, "Stretch", GuiAction::DraftStretch);
            menu_action(ui, gui, "Clone", GuiAction::DraftClone);
        });
        ui.menu_button("Arrays", |ui| {
            menu_action(ui, gui, "Rectangular Array", GuiAction::DraftArrayRect);
            menu_action(ui, gui, "Polar Array", GuiAction::DraftArrayPolar);
            menu_action(ui, gui, "Path Array", GuiAction::DraftArrayPath);
            menu_action(ui, gui, "Point Array", GuiAction::DraftArrayPoint);
        });
        ui.menu_button("Annotation", |ui| {
            menu_action(ui, gui, "Dimension", GuiAction::DraftDimension);
            menu_action(ui, gui, "Label", GuiAction::DraftLabel);
            menu_action(ui, gui, "Text", GuiAction::DraftText);
        });
        ui.menu_button("Conversion", |ui| {
            menu_action(ui, gui, "Upgrade", GuiAction::DraftUpgrade);
            menu_action(ui, gui, "Downgrade", GuiAction::DraftDowngrade);
            menu_action(ui, gui, "Wire to B-Spline", GuiAction::DraftWireToBSpline);
            menu_action(ui, gui, "Draft to Sketch", GuiAction::DraftToSketch);
        });
        ui.menu_button("Snap", |ui| {
            for snap in ["Midpoint", "Endpoint", "Center", "Perpendicular", "Grid", "Intersection", "Extension", "Nearest"] {
                menu_action(ui, gui, snap, GuiAction::ToggleDraftSnap(snap.to_lowercase()));
            }
        });
    });
}

fn draw_surface_menu(ui: &mut egui::Ui, gui: &mut GuiState) {
    ui.menu_button("Surface", |ui| {
        menu_action(ui, gui, "Filling", GuiAction::SurfaceFilling);
        menu_action(ui, gui, "Boundary", GuiAction::SurfaceBoundary);
        menu_action(ui, gui, "Sections", GuiAction::SurfaceSections);
        menu_action(ui, gui, "Extend", GuiAction::SurfaceExtend);
        menu_action(ui, gui, "Blend", GuiAction::SurfaceBlend);
        menu_action(ui, gui, "Pipe", GuiAction::SurfacePipe);
        menu_action(ui, gui, "Coons Patch", GuiAction::SurfaceCoons);
    });
}

fn draw_fem_menu(ui: &mut egui::Ui, gui: &mut GuiState) {
    ui.menu_button("FEM", |ui| {
        ui.menu_button("Analysis", |ui| {
            menu_action(ui, gui, "New Analysis", GuiAction::CreateFemAnalysis);
            menu_action(ui, gui, "Summary", GuiAction::FemSummary);
            menu_action(ui, gui, "Report", GuiAction::FemReport);
        });
        ui.menu_button("Material", |ui| {
            menu_action(ui, gui, "Pick Material...", GuiAction::OpenMaterialPicker);
            ui.separator();
            menu_action(ui, gui, "Steel", GuiAction::SetFemMaterial("steel".into()));
            menu_action(ui, gui, "Aluminum", GuiAction::SetFemMaterial("aluminum".into()));
        });
        ui.menu_button("Boundary Conditions", |ui| {
            ui.menu_button("Loads", |ui| {
                menu_action(ui, gui, "Force...", GuiAction::OpenBcEditor(BcKind::Force));
                menu_action(ui, gui, "Pressure...", GuiAction::OpenBcEditor(BcKind::Pressure));
                menu_action(ui, gui, "Gravity...", GuiAction::OpenBcEditor(BcKind::Gravity));
                menu_action(ui, gui, "Distributed Load...", GuiAction::OpenBcEditor(BcKind::DistributedLoad));
                menu_action(ui, gui, "Centrifugal Load...", GuiAction::OpenBcEditor(BcKind::CentrifugalLoad));
                menu_action(ui, gui, "Self Weight...", GuiAction::OpenBcEditor(BcKind::SelfWeight));
                menu_action(ui, gui, "Body Load...", GuiAction::OpenBcEditor(BcKind::BodyLoad));
            });
            ui.menu_button("Constraints", |ui| {
                menu_action(ui, gui, "Fixed Node...", GuiAction::OpenBcEditor(BcKind::FixedNode));
                menu_action(ui, gui, "Displacement...", GuiAction::OpenBcEditor(BcKind::Displacement));
                menu_action(ui, gui, "Spring...", GuiAction::OpenBcEditor(BcKind::Spring));
                menu_action(ui, gui, "Spring Constraint...", GuiAction::OpenBcEditor(BcKind::SpringConstraint));
            });
            ui.menu_button("Thermal", |ui| {
                menu_action(ui, gui, "Initial Temperature...", GuiAction::OpenBcEditor(BcKind::InitialTemperature));
            });
        });
        ui.menu_button("Mesh", |ui| {
            menu_action(ui, gui, "Generate Tet Mesh", GuiAction::GenTetMesh { element_size: 1.0 });
            menu_action(ui, gui, "Generate Hex Mesh", GuiAction::GenHexMesh { nx: 10, ny: 10, nz: 10 });
        });
        ui.menu_button("Constraints", |ui| {
            menu_action(ui, gui, "Fixed", GuiAction::AddFemConstraint(FemConstraintType::Fixed));
            menu_action(ui, gui, "Force", GuiAction::AddFemConstraint(FemConstraintType::Force));
            menu_action(ui, gui, "Pressure", GuiAction::AddFemConstraint(FemConstraintType::Pressure));
            menu_action(ui, gui, "Displacement", GuiAction::AddFemConstraint(FemConstraintType::Displacement));
            menu_action(ui, gui, "Gravity", GuiAction::AddFemConstraint(FemConstraintType::Gravity));
            menu_action(ui, gui, "Spring", GuiAction::AddFemConstraint(FemConstraintType::Spring));
        });
        ui.separator();
        ui.menu_button("Solve", |ui| {
            menu_action(ui, gui, "Static Analysis", GuiAction::SolveStatic);
            menu_action(ui, gui, "Modal Analysis", GuiAction::SolveModal { modes: 6 });
            menu_action(ui, gui, "Thermal Analysis", GuiAction::SolveThermal);
            menu_action(ui, gui, "Buckling Analysis", GuiAction::SolveBuckling { modes: 3 });
            menu_action(ui, gui, "Nonlinear Analysis", GuiAction::SolveNonlinear);
        });
        ui.menu_button("Results", |ui| {
            menu_action(ui, gui, "Show Stress", GuiAction::ShowStress);
            menu_action(ui, gui, "Show Displacement", GuiAction::ShowDisplacement);
            menu_action(ui, gui, "Show Von Mises", GuiAction::ShowVonMises);
        });
    });
}
