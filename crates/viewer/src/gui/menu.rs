use super::{
    AssemblyJointType, BcKind, FemConstraintType, GuiAction, GuiState, SketchTool, Workbench,
    task_panel,
};
use crate::render::{Camera, DisplayMode, Projection, StandardView};

pub(crate) fn draw_menu_bar(
    ctx: &egui::Context,
    gui: &mut GuiState,
    camera: &Camera,
    display_mode: DisplayMode,
) {
    egui::TopBottomPanel::top("menu_bar")
        .frame(egui::Frame {
            fill: egui::Color32::from_rgb(0x14, 0x17, 0x1D),
            inner_margin: egui::Margin::symmetric(6, 2),
            stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(0x0B, 0x0D, 0x12)),
            ..egui::Frame::NONE
        })
        .show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            // ---- File ----
            ui.menu_button("File", |ui| {
                menu_section(ui, "Project");
                if ui
                    .add(egui::Button::new("New").shortcut_text("Ctrl+N"))
                    .clicked()
                {
                    gui.actions.push(GuiAction::NewModel);
                    gui.status_message = "New model".into();
                    ui.close_menu();
                }

                if ui
                    .add(egui::Button::new("Open\u{2026}").shortcut_text("Ctrl+O"))
                    .clicked()
                {
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

                if ui
                    .add(egui::Button::new("Save As\u{2026}").shortcut_text("Ctrl+S"))
                    .clicked()
                {
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
                            let short =
                                path_str.rsplit('/').next().unwrap_or(&path_str).to_string();
                            if ui.button(&short).on_hover_text(&path_str).clicked() {
                                gui.actions
                                    .push(GuiAction::OpenFile(std::path::PathBuf::from(&path_str)));
                                ui.close_menu();
                            }
                        }
                    });
                }

                if ui
                    .add(egui::Button::new("Inspect Command File\u{2026}"))
                    .on_hover_text(
                        "Decode a .cadk Command-log container and show its header, \
                         flags, command count, and thumbnail status.",
                    )
                    .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("CADKernel Command Log", &["cadk"])
                        .pick_file()
                    {
                        let report = super::inspect_cadk_path(&path);
                        gui.status_message = format!("Inspected {}", path.display());
                        gui.cadk_inspector = Some(report);
                    }
                    ui.close_menu();
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
                    let exports: &[(
                        &str,
                        &str,
                        &[&str],
                        fn(std::path::PathBuf) -> GuiAction,
                    )] = &[
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
                    let cad_exports: &[(
                        &str,
                        &str,
                        &[&str],
                        fn(std::path::PathBuf) -> GuiAction,
                    )] = &[
                        ("STEP...", "model.step", &["step", "stp"], |p| {
                            GuiAction::ExportStep(p)
                        }),
                        ("IGES...", "model.iges", &["iges", "igs"], |p| {
                            GuiAction::ExportIges(p)
                        }),
                        ("DXF...", "model.dxf", &["dxf"], |p| GuiAction::ExportDxf(p)),
                        ("BREP...", "model.brep", &["brep", "brp"], |p| {
                            GuiAction::ExportBrep(p)
                        }),
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
                    let scene_exports: &[(
                        &str,
                        &str,
                        &[&str],
                        fn(std::path::PathBuf) -> GuiAction,
                    )] = &[
                        ("glTF...", "model.gltf", &["gltf"], |p| {
                            GuiAction::ExportGltf(p)
                        }),
                        ("3MF...", "model.3mf", &["3mf"], |p| GuiAction::Export3mf(p)),
                        ("SVG...", "model.svg", &["svg"], |p| GuiAction::ExportSvg(p)),
                        ("DAE (Collada)...", "model.dae", &["dae"], |p| {
                            GuiAction::ExportDae(p)
                        }),
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
                        ui.label(
                            egui::RichText::new("No recent files")
                                .size(11.0)
                                .color(super::theme::COLOR_DIM),
                        );
                    } else {
                        let files = gui.recent_files.clone();
                        for path_str in &files {
                            let display = std::path::Path::new(path_str)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or(path_str);
                            if ui
                                .button(display)
                                .on_hover_text(path_str.as_str())
                                .clicked()
                            {
                                gui.actions
                                    .push(GuiAction::OpenFile(std::path::PathBuf::from(path_str)));
                                ui.close_menu();
                            }
                        }
                        ui.separator();
                        if ui
                            .button(
                                egui::RichText::new("Clear Recent Files")
                                    .size(11.0)
                                    .color(super::theme::COLOR_DIM),
                            )
                            .clicked()
                        {
                            gui.actions.push(GuiAction::ClearRecentFiles);
                            ui.close_menu();
                        }
                    }
                });

                ui.separator();

                if ui
                    .add(egui::Button::new("Quit").shortcut_text("Ctrl+Q"))
                    .clicked()
                {
                    gui.request_quit = true;
                }
            });

            // ---- Edit ----
            ui.menu_button("Edit", |ui| {
                menu_section(ui, "History");
                menu_action_sc(ui, gui, "Undo", "Ctrl+Z", GuiAction::Undo);
                menu_action_sc(ui, gui, "Redo", "Ctrl+Y", GuiAction::Redo);
                ui.separator();
                menu_action_sc(
                    ui,
                    gui,
                    "Copy",
                    "Ctrl+C",
                    GuiAction::StatusMessage("Copy: select an object first".into()),
                );
                menu_action_sc(
                    ui,
                    gui,
                    "Paste",
                    "Ctrl+V",
                    GuiAction::StatusMessage("Paste: clipboard empty".into()),
                );
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
                        let enter =
                            resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        let create = ui
                            .add_enabled(
                                !gui.group_name_input.trim().is_empty(),
                                egui::Button::new("Create"),
                            )
                            .clicked();
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
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new(eye_icon)
                                                .color(eye_color)
                                                .size(14.0),
                                        )
                                        .frame(false),
                                    )
                                    .on_hover_text(if vis { "Hide group" } else { "Show group" })
                                    .clicked()
                                {
                                    gui.actions.push(GuiAction::ToggleGroupVisibility(gid));
                                }
                                // Group name + member count
                                let label = format!("{name} ({count})");
                                if ui
                                    .button(&label)
                                    .on_hover_text("Add selected objects to this group")
                                    .clicked()
                                {
                                    gui.actions.push(GuiAction::GroupSelected(gid));
                                    ui.close_menu();
                                }
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .small_button(
                                                egui::RichText::new("\u{2715}").weak().size(11.0),
                                            )
                                            .on_hover_text("Delete group (keeps objects)")
                                            .clicked()
                                        {
                                            gui.actions.push(GuiAction::DeleteGroup(gid));
                                            ui.close_menu();
                                        }
                                    },
                                );
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
                    gui.active_task = Some(task_panel::ActiveTask::Box {
                        width: 10.0,
                        height: 10.0,
                        depth: 10.0,
                        preview_id: None,
                    });
                    ui.close_menu();
                }
                if ui.button("Cylinder...").clicked() {
                    gui.active_task = Some(task_panel::ActiveTask::Cylinder {
                        radius: 5.0,
                        height: 10.0,
                        preview_id: None,
                    });
                    ui.close_menu();
                }
                if ui.button("Sphere...").clicked() {
                    gui.active_task = Some(task_panel::ActiveTask::Sphere {
                        radius: 5.0,
                        preview_id: None,
                    });
                    ui.close_menu();
                }
                if ui.button("Cone...").clicked() {
                    gui.active_task = Some(task_panel::ActiveTask::Cone {
                        base_radius: 5.0,
                        top_radius: 0.0,
                        height: 10.0,
                        preview_id: None,
                    });
                    ui.close_menu();
                }
                if ui.button("Torus...").clicked() {
                    gui.active_task = Some(task_panel::ActiveTask::Torus {
                        major_radius: 5.0,
                        minor_radius: 1.5,
                        preview_id: None,
                    });
                    ui.close_menu();
                }
                ui.separator();
                menu_section(ui, "Extended Primitives");
                if ui.button("Tube...").clicked() {
                    gui.active_task = Some(task_panel::ActiveTask::Tube {
                        outer_radius: 5.0,
                        inner_radius: 3.0,
                        height: 10.0,
                        preview_id: None,
                    });
                    ui.close_menu();
                }
                if ui.button("Prism...").clicked() {
                    gui.active_task = Some(task_panel::ActiveTask::Prism {
                        radius: 5.0,
                        height: 10.0,
                        sides: 6,
                        preview_id: None,
                    });
                    ui.close_menu();
                }
                if ui.button("Wedge...").clicked() {
                    gui.active_task = Some(task_panel::ActiveTask::Wedge {
                        dx: 10.0,
                        dy: 10.0,
                        dz: 10.0,
                        dx2: 5.0,
                        dy2: 5.0,
                        preview_id: None,
                    });
                    ui.close_menu();
                }
                if ui.button("Ellipsoid...").clicked() {
                    gui.active_task = Some(task_panel::ActiveTask::Ellipsoid {
                        rx: 5.0,
                        ry: 3.0,
                        rz: 2.0,
                        preview_id: None,
                    });
                    ui.close_menu();
                }
                if ui.button("Helix...").clicked() {
                    gui.active_task = Some(task_panel::ActiveTask::Helix {
                        radius: 5.0,
                        pitch: 3.0,
                        turns: 3.0,
                        tube_radius: 0.5,
                        preview_id: None,
                    });
                    ui.close_menu();
                }
            });

            // ---- Macro ----
            ui.menu_button("Macro", |ui| {
                menu_action(
                    ui,
                    gui,
                    "Run Script...",
                    GuiAction::StatusMessage("Python scripting: coming soon".into()),
                );
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
                if ui
                    .add(egui::Button::new(proj_label).shortcut_text("5"))
                    .clicked()
                {
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
                        if ui
                            .add(egui::Button::new(view.label()).shortcut_text(key))
                            .clicked()
                        {
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
                menu_action_sc(
                    ui,
                    gui,
                    "Section Plane",
                    "Shift+S",
                    GuiAction::ToggleSectionPlane,
                );

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
                        let enter =
                            resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        let save = ui
                            .add_enabled(
                                !gui.bookmark_name_input.trim().is_empty(),
                                egui::Button::new("Save"),
                            )
                            .clicked();
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
                        ui.weak(
                            "No bookmarks saved yet.\nSave a view to quickly return to it later.",
                        );
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
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .small_button(
                                                egui::RichText::new("\u{2715}").weak().size(11.0),
                                            )
                                            .on_hover_text("Delete bookmark")
                                            .clicked()
                                        {
                                            gui.actions.push(GuiAction::DeleteBookmark(idx));
                                            ui.close_menu();
                                        }
                                    },
                                );
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

fn menu_action_sc(
    ui: &mut egui::Ui,
    gui: &mut GuiState,
    label: &str,
    shortcut: &str,
    action: GuiAction,
) {
    if ui
        .add(egui::Button::new(label).shortcut_text(shortcut))
        .clicked()
    {
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
            if ui.button("Box...").clicked() {
                gui.active_task = Some(task_panel::ActiveTask::Box {
                    width: 10.0,
                    height: 10.0,
                    depth: 10.0,
                    preview_id: None,
                });
                ui.close_menu();
            }
            if ui.button("Cylinder...").clicked() {
                gui.active_task = Some(task_panel::ActiveTask::Cylinder {
                    radius: 5.0,
                    height: 10.0,
                    preview_id: None,
                });
                ui.close_menu();
            }
            if ui.button("Sphere...").clicked() {
                gui.active_task = Some(task_panel::ActiveTask::Sphere {
                    radius: 5.0,
                    preview_id: None,
                });
                ui.close_menu();
            }
            if ui.button("Cone...").clicked() {
                gui.active_task = Some(task_panel::ActiveTask::Cone {
                    base_radius: 5.0,
                    top_radius: 0.0,
                    height: 10.0,
                    preview_id: None,
                });
                ui.close_menu();
            }
            if ui.button("Torus...").clicked() {
                gui.active_task = Some(task_panel::ActiveTask::Torus {
                    major_radius: 5.0,
                    minor_radius: 1.5,
                    preview_id: None,
                });
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Tube...").clicked() {
                gui.active_task = Some(task_panel::ActiveTask::Tube {
                    outer_radius: 5.0,
                    inner_radius: 3.0,
                    height: 10.0,
                    preview_id: None,
                });
                ui.close_menu();
            }
            if ui.button("Prism...").clicked() {
                gui.active_task = Some(task_panel::ActiveTask::Prism {
                    radius: 5.0,
                    height: 10.0,
                    sides: 6,
                    preview_id: None,
                });
                ui.close_menu();
            }
            if ui.button("Wedge...").clicked() {
                gui.active_task = Some(task_panel::ActiveTask::Wedge {
                    dx: 10.0,
                    dy: 10.0,
                    dz: 10.0,
                    dx2: 5.0,
                    dy2: 5.0,
                    preview_id: None,
                });
                ui.close_menu();
            }
            if ui.button("Ellipsoid...").clicked() {
                gui.active_task = Some(task_panel::ActiveTask::Ellipsoid {
                    rx: 5.0,
                    ry: 3.0,
                    rz: 2.0,
                    preview_id: None,
                });
                ui.close_menu();
            }
            if ui.button("Helix...").clicked() {
                gui.active_task = Some(task_panel::ActiveTask::Helix {
                    radius: 5.0,
                    pitch: 3.0,
                    turns: 3.0,
                    tube_radius: 0.5,
                    preview_id: None,
                });
                ui.close_menu();
            }
        });
        ui.separator();
        use super::PartAction as Pa;
        ui.menu_button("Boolean", |ui| {
            menu_action(ui, gui, "Union", GuiAction::BooleanSceneUnion);
            menu_action(ui, gui, "Subtract", GuiAction::BooleanSceneSubtract);
            menu_action(ui, gui, "Intersect", GuiAction::BooleanSceneIntersect);
            ui.separator();
            menu_action(
                ui,
                gui,
                "Boolean Fragments",
                GuiAction::Part(Pa::BooleanFragments),
            );
            menu_action(
                ui,
                gui,
                "Slice to Compound",
                GuiAction::Part(Pa::SliceToCompound),
            );
        });
        ui.menu_button("Join", |ui| {
            menu_action(
                ui,
                gui,
                "Face from Wires",
                GuiAction::Part(Pa::FaceFromWires),
            );
            menu_action(
                ui,
                gui,
                "Connect Shapes",
                GuiAction::Part(Pa::ConnectShapes),
            );
            menu_action(ui, gui, "Embed Shapes", GuiAction::Part(Pa::EmbedShapes));
            menu_action(ui, gui, "Cutout Shapes", GuiAction::Part(Pa::CutoutShapes));
        });
        ui.menu_button("Compound", |ui| {
            menu_action(
                ui,
                gui,
                "Explode Compound",
                GuiAction::Part(Pa::ExplodeCompound),
            );
            menu_action(
                ui,
                gui,
                "Compound Filter",
                GuiAction::Part(Pa::CompoundFilter),
            );
        });
        ui.separator();
        ui.menu_button("Transform", |ui| {
            menu_action(
                ui,
                gui,
                "Mirror XY",
                GuiAction::MirrorSolid(super::MirrorPlane::XY),
            );
            menu_action(
                ui,
                gui,
                "Mirror XZ",
                GuiAction::MirrorSolid(super::MirrorPlane::XZ),
            );
            menu_action(
                ui,
                gui,
                "Mirror YZ",
                GuiAction::MirrorSolid(super::MirrorPlane::YZ),
            );
            ui.separator();
            menu_action(ui, gui, "Scale...", GuiAction::ScaleSolid { factor: 2.0 });
            menu_action(
                ui,
                gui,
                "Transformed Copy...",
                GuiAction::Part(Pa::TransformedCopy {
                    dx: 10.0,
                    dy: 0.0,
                    dz: 0.0,
                }),
            );
            ui.separator();
            menu_action(
                ui,
                gui,
                "Linear Pattern X",
                GuiAction::LinearPattern {
                    count: 3,
                    spacing: 10.0,
                    axis: 0,
                },
            );
            menu_action(
                ui,
                gui,
                "Linear Pattern Y",
                GuiAction::LinearPattern {
                    count: 3,
                    spacing: 10.0,
                    axis: 1,
                },
            );
            menu_action(
                ui,
                gui,
                "Linear Pattern Z",
                GuiAction::LinearPattern {
                    count: 3,
                    spacing: 10.0,
                    axis: 2,
                },
            );
        });
        ui.menu_button("Convert", |ui| {
            menu_action(
                ui,
                gui,
                "Points from Shape",
                GuiAction::Part(Pa::PointsFromShape),
            );
            menu_action(
                ui,
                gui,
                "Convert to Solid",
                GuiAction::Part(Pa::ConvertToSolid),
            );
            menu_action(
                ui,
                gui,
                "Auto Defeature...",
                GuiAction::Part(Pa::AutoDefeaturing { threshold: 1.0 }),
            );
            menu_action(
                ui,
                gui,
                "Project Curves on Surface",
                GuiAction::Part(Pa::ProjectCurvesOnSurface),
            );
            menu_action(ui, gui, "Coons Patch", GuiAction::Part(Pa::CoonsPatch));
        });
        ui.separator();
        ui.menu_button("Features", |ui| {
            menu_action(
                ui,
                gui,
                "Fillet All Edges...",
                GuiAction::FilletAllEdges { radius: 1.0 },
            );
            menu_action(
                ui,
                gui,
                "Chamfer All Edges...",
                GuiAction::ChamferAllEdges { distance: 1.0 },
            );
            menu_action(
                ui,
                gui,
                "Shell...",
                GuiAction::ShellSolid { thickness: 1.0 },
            );
        });
    });
}

fn draw_part_design_menu(ui: &mut egui::Ui, gui: &mut GuiState) {
    use super::PartDesignAction as Pd;
    ui.menu_button("PartDesign", |ui| {
        ui.menu_button("Additive", |ui| {
            menu_action_sc(
                ui,
                gui,
                "Pad",
                "P",
                GuiAction::PartDesign(Pd::PadSketch {
                    depth: 10.0,
                    symmetric: false,
                }),
            );
            menu_action(
                ui,
                gui,
                "Additive Loft",
                GuiAction::PartDesign(Pd::AdditiveLoft),
            );
            menu_action(
                ui,
                gui,
                "Additive Pipe",
                GuiAction::PartDesign(Pd::AdditivePipe),
            );
            ui.separator();
            if ui.button("Additive Box...").clicked() {
                gui.active_task = Some(task_panel::ActiveTask::Box {
                    width: 10.0,
                    height: 10.0,
                    depth: 10.0,
                    preview_id: None,
                });
                ui.close_menu();
            }
            if ui.button("Additive Cylinder...").clicked() {
                gui.active_task = Some(task_panel::ActiveTask::Cylinder {
                    radius: 5.0,
                    height: 10.0,
                    preview_id: None,
                });
                ui.close_menu();
            }
            if ui.button("Additive Sphere...").clicked() {
                gui.active_task = Some(task_panel::ActiveTask::Sphere {
                    radius: 5.0,
                    preview_id: None,
                });
                ui.close_menu();
            }
            if ui.button("Additive Cone...").clicked() {
                gui.active_task = Some(task_panel::ActiveTask::Cone {
                    base_radius: 5.0,
                    top_radius: 0.0,
                    height: 10.0,
                    preview_id: None,
                });
                ui.close_menu();
            }
            if ui.button("Additive Torus...").clicked() {
                gui.active_task = Some(task_panel::ActiveTask::Torus {
                    major_radius: 5.0,
                    minor_radius: 1.5,
                    preview_id: None,
                });
                ui.close_menu();
            }
        });
        ui.menu_button("Subtractive", |ui| {
            menu_action(
                ui,
                gui,
                "Pocket",
                GuiAction::PartDesign(Pd::PocketSketch {
                    depth: 10.0,
                    through_all: false,
                }),
            );
            menu_action(
                ui,
                gui,
                "Groove",
                GuiAction::PartDesign(Pd::GrooveSketch { angle: 360.0 }),
            );
            menu_action(
                ui,
                gui,
                "Subtractive Loft",
                GuiAction::PartDesign(Pd::SubtractiveLoft),
            );
            menu_action(
                ui,
                gui,
                "Subtractive Pipe",
                GuiAction::PartDesign(Pd::SubtractivePipe),
            );
            ui.separator();
            menu_action(
                ui,
                gui,
                "Hole...",
                GuiAction::PartDesign(Pd::HoleSketch {
                    radius: 5.0,
                    depth: 10.0,
                }),
            );
            menu_action(
                ui,
                gui,
                "Countersunk Hole...",
                GuiAction::PartDesign(Pd::CountersunkHoleSketch {
                    radius: 5.0,
                    depth: 10.0,
                    countersink_angle: 90.0,
                }),
            );
        });
        ui.separator();
        ui.menu_button("Features", |ui| {
            menu_action(
                ui,
                gui,
                "Fillet All...",
                GuiAction::FilletAllEdges { radius: 1.0 },
            );
            menu_action(
                ui,
                gui,
                "Chamfer All...",
                GuiAction::ChamferAllEdges { distance: 1.0 },
            );
            menu_action(
                ui,
                gui,
                "Shell...",
                GuiAction::ShellSolid { thickness: 1.0 },
            );
        });
        ui.menu_button("Pattern", |ui| {
            menu_action(
                ui,
                gui,
                "Linear Pattern X",
                GuiAction::LinearPattern {
                    count: 3,
                    spacing: 10.0,
                    axis: 0,
                },
            );
            menu_action(
                ui,
                gui,
                "Linear Pattern Y",
                GuiAction::LinearPattern {
                    count: 3,
                    spacing: 10.0,
                    axis: 1,
                },
            );
            menu_action(
                ui,
                gui,
                "Linear Pattern Z",
                GuiAction::LinearPattern {
                    count: 3,
                    spacing: 10.0,
                    axis: 2,
                },
            );
            ui.separator();
            menu_action(
                ui,
                gui,
                "Mirror XY",
                GuiAction::MirrorSolid(super::MirrorPlane::XY),
            );
            menu_action(
                ui,
                gui,
                "Mirror XZ",
                GuiAction::MirrorSolid(super::MirrorPlane::XZ),
            );
            menu_action(
                ui,
                gui,
                "Mirror YZ",
                GuiAction::MirrorSolid(super::MirrorPlane::YZ),
            );
        });
        ui.separator();
        ui.menu_button("Mechanical", |ui| {
            menu_action(
                ui,
                gui,
                "Involute Gear...",
                GuiAction::PartDesign(Pd::CreateInvoluteGear {
                    teeth: 20,
                    module_val: 2.0,
                    pressure_angle: 20.0,
                }),
            );
            menu_action(
                ui,
                gui,
                "Sprocket...",
                GuiAction::PartDesign(Pd::CreateSprocket {
                    teeth: 15,
                    roller_diameter: 8.0,
                    pitch: 12.7,
                    bore: 10.0,
                }),
            );
            menu_action(
                ui,
                gui,
                "Shaft Design...",
                GuiAction::PartDesign(Pd::CreateShaftDesign {
                    segments: vec![(10.0, 5.0), (20.0, 8.0), (10.0, 5.0)],
                }),
            );
        });
        ui.separator();
        ui.menu_button("Body", |ui| {
            menu_action(
                ui,
                gui,
                "Shape Binder",
                GuiAction::PartDesign(Pd::ShapeBinder),
            );
            menu_action(
                ui,
                gui,
                "Suppress Feature",
                GuiAction::PartDesign(Pd::SuppressFeature),
            );
            menu_action(ui, gui, "Set Tip", GuiAction::PartDesign(Pd::SetTip));
            menu_action(
                ui,
                gui,
                "Move Feature Up",
                GuiAction::PartDesign(Pd::MoveFeatureUp),
            );
            menu_action(
                ui,
                gui,
                "Move Feature Down",
                GuiAction::PartDesign(Pd::MoveFeatureDown),
            );
        });
    });
}

fn draw_sketch_menu(ui: &mut egui::Ui, gui: &mut GuiState) {
    use super::SketcherAction as S;
    let sk = GuiAction::Sketcher;
    ui.menu_button("Sketch", |ui| {
        let in_sketch = gui.sketch_mode.is_some();
        ui.menu_button("Geometry", |ui| {
            menu_action_sc(ui, gui, "Select", "S", sk(S::SetTool(SketchTool::Select)));
            ui.separator();
            menu_action(ui, gui, "Point", sk(S::SetTool(SketchTool::Point)));
            menu_action_sc(ui, gui, "Line", "L", sk(S::SetTool(SketchTool::Line)));
            menu_action_sc(
                ui,
                gui,
                "Rectangle",
                "R",
                sk(S::SetTool(SketchTool::Rectangle)),
            );
            menu_action_sc(ui, gui, "Circle", "C", sk(S::SetTool(SketchTool::Circle)));
            menu_action_sc(ui, gui, "Arc", "A", sk(S::SetTool(SketchTool::Arc)));
            menu_action_sc(ui, gui, "Ellipse", "E", sk(S::SetTool(SketchTool::Ellipse)));
            menu_action_sc(
                ui,
                gui,
                "Polyline",
                "W",
                sk(S::SetTool(SketchTool::Polyline)),
            );
            menu_action(ui, gui, "Slot", sk(S::SetTool(SketchTool::Slot)));
            menu_action_sc(
                ui,
                gui,
                "B-Spline",
                "B",
                sk(S::SetTool(SketchTool::BSpline)),
            );
            menu_action_sc(
                ui,
                gui,
                "Polygon",
                "N",
                sk(S::SetTool(SketchTool::Polygon { sides: 6 })),
            );
        });
        ui.menu_button("Constraints", |ui| {
            menu_section(ui, "Geometric");
            menu_action_sc(ui, gui, "Horizontal", "H", sk(S::ConstrainHorizontal));
            menu_action_sc(ui, gui, "Vertical", "V", sk(S::ConstrainVertical));
            menu_action(ui, gui, "Parallel", sk(S::ConstrainParallel));
            menu_action(ui, gui, "Perpendicular", sk(S::ConstrainPerpendicular));
            menu_action(ui, gui, "Coincident", sk(S::ConstrainCoincident));
            menu_action(ui, gui, "Tangent", sk(S::ConstrainTangent));
            menu_action(ui, gui, "Equal", sk(S::ConstrainEqual));
            menu_action(ui, gui, "Symmetric", sk(S::ConstrainSymmetric));
            menu_action_sc(ui, gui, "Fixed", "F", sk(S::ConstrainFixed));
            menu_action(ui, gui, "Block", sk(S::ConstrainBlock));
            ui.separator();
            menu_section(ui, "Dimensional");
            menu_action(
                ui,
                gui,
                "Length...",
                sk(S::ConstrainLength(gui.constraint_length_value)),
            );
            menu_action(
                ui,
                gui,
                "Distance...",
                sk(S::ConstrainDistance(gui.constraint_distance_value)),
            );
            menu_action(
                ui,
                gui,
                "Angle...",
                sk(S::ConstrainAngle(gui.constraint_angle_value)),
            );
            menu_action(
                ui,
                gui,
                "Radius...",
                sk(S::ConstrainRadius(gui.constraint_radius_value)),
            );
            menu_action(
                ui,
                gui,
                "Diameter...",
                sk(S::ConstrainDiameter(gui.constraint_radius_value * 2.0)),
            );
            menu_action(
                ui,
                gui,
                "H Distance...",
                sk(S::ConstrainHDistance(gui.constraint_distance_value)),
            );
            menu_action(
                ui,
                gui,
                "V Distance...",
                sk(S::ConstrainVDistance(gui.constraint_distance_value)),
            );
        });
        ui.menu_button("Tools", |ui| {
            menu_action(
                ui,
                gui,
                "Fillet Corner...",
                sk(S::FilletCorner {
                    radius: gui.sketch_fillet_radius,
                }),
            );
            menu_action(
                ui,
                gui,
                "Chamfer Corner...",
                sk(S::ChamferCorner {
                    distance: gui.sketch_chamfer_distance,
                }),
            );
            ui.separator();
            menu_action(ui, gui, "Trim Edge", sk(S::TrimEdge));
            menu_action(ui, gui, "Split Edge", sk(S::SplitEdge));
            menu_action(ui, gui, "Extend Edge", sk(S::ExtendEdge));
            ui.separator();
            menu_action(ui, gui, "Mirror Geometry", sk(S::MirrorGeometry));
            menu_action(ui, gui, "External Projection", sk(S::ExternalProjection));
            menu_action(ui, gui, "Select References", sk(S::SelectReferences));
            menu_action(ui, gui, "Promote References", sk(S::PromoteReferences));
            menu_action(ui, gui, "Carbon Copy", sk(S::CarbonCopy));
        });
        ui.menu_button("B-Spline", |ui| {
            menu_action_sc(
                ui,
                gui,
                "B-Spline Tool",
                "B",
                sk(S::SetTool(SketchTool::BSpline)),
            );
            ui.separator();
            menu_action(ui, gui, "Convert to B-Spline", sk(S::ConvertToBSpline));
            menu_action(ui, gui, "Increase Degree", sk(S::IncreaseDegree));
            menu_action(ui, gui, "Decrease Degree", sk(S::DecreaseDegree));
            menu_action(ui, gui, "Insert Knot", sk(S::InsertKnot));
        });
        ui.separator();
        ui.menu_button("Toggles", |ui| {
            menu_action_sc(ui, gui, "Construction Mode", "X", sk(S::ToggleConstruction));
            menu_action_sc(ui, gui, "Grid", "G", sk(S::ToggleGrid));
            menu_action_sc(ui, gui, "Snap", "Shift+S", sk(S::ToggleSnap));
            menu_action(ui, gui, "Show Constraints", sk(S::ToggleConstraintsVisible));
        });
        ui.separator();
        if in_sketch {
            menu_action_sc(ui, gui, "Close Sketch", "Enter", sk(S::Close));
            menu_action_sc(ui, gui, "Cancel Sketch", "Esc", sk(S::Cancel));
        } else {
            menu_action(
                ui,
                gui,
                "New Sketch (XY)",
                sk(S::Enter(cadkernel_sketch::WorkPlane::xy())),
            );
            menu_action(
                ui,
                gui,
                "New Sketch (XZ)",
                sk(S::Enter(cadkernel_sketch::WorkPlane::xz())),
            );
            menu_action(
                ui,
                gui,
                "New Sketch (YZ)",
                sk(S::Enter(cadkernel_sketch::WorkPlane::new(
                    cadkernel_math::Point3::ORIGIN,
                    cadkernel_math::Vec3::X,
                    cadkernel_math::Vec3::Y,
                ))),
            );
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
                    if let Some(path) = rfd::FileDialog::new().add_filter(*filter, exts).pick_file()
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
                (
                    "Export STL...",
                    "model.stl",
                    "STL",
                    &["stl"][..],
                    GuiAction::ExportStl as fn(std::path::PathBuf) -> GuiAction,
                ),
                (
                    "Export OBJ...",
                    "model.obj",
                    "OBJ",
                    &["obj"][..],
                    GuiAction::ExportObj,
                ),
                (
                    "Export PLY...",
                    "model.ply",
                    "PLY",
                    &["ply"][..],
                    GuiAction::ExportPly,
                ),
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
            use super::MeshAction as M;
            menu_action(ui, gui, "Decimate (50%)", GuiAction::Mesh(M::Decimate(0.5)));
            menu_action(ui, gui, "Subdivide", GuiAction::Mesh(M::Subdivide));
            menu_action(
                ui,
                gui,
                "Smooth",
                GuiAction::Mesh(M::Smooth {
                    iterations: 3,
                    factor: 0.5,
                }),
            );
            menu_action(ui, gui, "Fill Holes", GuiAction::Mesh(M::FillHoles));
            menu_action(
                ui,
                gui,
                "Remesh...",
                GuiAction::Mesh(M::Remesh {
                    target_edge_len: 1.0,
                }),
            );
            menu_action(ui, gui, "Repair", GuiAction::Mesh(M::Repair));
        });
        ui.menu_button("Analyze", |ui| {
            use super::MeshAction as M;
            menu_action(
                ui,
                gui,
                "Check Watertight",
                GuiAction::Mesh(M::CheckWatertight),
            );
            menu_action(
                ui,
                gui,
                "Harmonize Normals",
                GuiAction::Mesh(M::HarmonizeNormals),
            );
            menu_action(ui, gui, "Flip Normals", GuiAction::Mesh(M::FlipNormals));
            menu_action(ui, gui, "Check Geometry", GuiAction::CheckGeometry);
        });
    });
}

fn draw_techdraw_menu(ui: &mut egui::Ui, gui: &mut GuiState) {
    use super::TechDrawAction as T;
    use super::TechDrawAnnotationKind as Ak;
    use super::TechDrawCenterlineKind as Ck;
    use super::TechDrawDimensionKind as Dk;
    use super::TechDrawViewKind as Vk;
    ui.menu_button("TechDraw", |ui| {
        ui.menu_button("Page", |ui| {
            menu_action(ui, gui, "New Page", GuiAction::TechDraw(T::NewPage));
            menu_action(
                ui,
                gui,
                "From Template...",
                GuiAction::TechDraw(T::OpenPageSetup),
            );
            menu_action(ui, gui, "Redraw Page", GuiAction::TechDraw(T::Redraw));
        });
        ui.menu_button("Views", |ui| {
            menu_action(
                ui,
                gui,
                "Front View",
                GuiAction::TechDraw(T::OpenViewSetup(Vk::Front)),
            );
            menu_action(
                ui,
                gui,
                "Top View",
                GuiAction::TechDraw(T::OpenViewSetup(Vk::Top)),
            );
            menu_action(
                ui,
                gui,
                "Right View",
                GuiAction::TechDraw(T::OpenViewSetup(Vk::Right)),
            );
            menu_action(
                ui,
                gui,
                "Isometric View",
                GuiAction::TechDraw(T::OpenViewSetup(Vk::Isometric)),
            );
            ui.separator();
            menu_action(
                ui,
                gui,
                "3-View Layout",
                GuiAction::TechDraw(T::OpenViewSetup(Vk::ThreeView)),
            );
            menu_action(
                ui,
                gui,
                "Section View",
                GuiAction::TechDraw(T::OpenViewSetup(Vk::Section)),
            );
            menu_action(
                ui,
                gui,
                "Detail View",
                GuiAction::TechDraw(T::OpenViewSetup(Vk::Detail)),
            );
            menu_action(
                ui,
                gui,
                "Broken View",
                GuiAction::TechDraw(T::OpenViewSetup(Vk::Broken)),
            );
        });
        ui.separator();
        ui.menu_button("Dimensions", |ui| {
            menu_action(
                ui,
                gui,
                "Linear Dimension",
                GuiAction::TechDraw(T::OpenDimensionSetup(Dk::Linear)),
            );
            menu_action(
                ui,
                gui,
                "Radius Dimension",
                GuiAction::TechDraw(T::OpenDimensionSetup(Dk::Radius)),
            );
            menu_action(
                ui,
                gui,
                "Diameter Dimension",
                GuiAction::TechDraw(T::OpenDimensionSetup(Dk::Diameter)),
            );
            menu_action(
                ui,
                gui,
                "Angle Dimension",
                GuiAction::TechDraw(T::OpenDimensionSetup(Dk::Angle)),
            );
            menu_action(
                ui,
                gui,
                "Arc Length",
                GuiAction::TechDraw(T::OpenDimensionSetup(Dk::ArcLength)),
            );
            menu_action(
                ui,
                gui,
                "Area",
                GuiAction::TechDraw(T::OpenDimensionSetup(Dk::Area)),
            );
        });
        ui.menu_button("Annotations", |ui| {
            menu_action(
                ui,
                gui,
                "Text",
                GuiAction::TechDraw(T::OpenAnnotationSetup(Ak::Text)),
            );
            menu_action(
                ui,
                gui,
                "Rich Text",
                GuiAction::TechDraw(T::OpenAnnotationSetup(Ak::RichText)),
            );
            menu_action(
                ui,
                gui,
                "Balloon",
                GuiAction::TechDraw(T::OpenAnnotationSetup(Ak::Balloon)),
            );
            menu_action(
                ui,
                gui,
                "Leader Line",
                GuiAction::TechDraw(T::OpenAnnotationSetup(Ak::Leader)),
            );
            menu_action(
                ui,
                gui,
                "Weld Symbol",
                GuiAction::TechDraw(T::OpenAnnotationSetup(Ak::Weld)),
            );
            menu_action(
                ui,
                gui,
                "Surface Finish",
                GuiAction::TechDraw(T::OpenAnnotationSetup(Ak::SurfaceFinish)),
            );
        });
        ui.menu_button("Centerlines", |ui| {
            menu_action(
                ui,
                gui,
                "Center on Face",
                GuiAction::TechDraw(T::OpenCenterlineSetup(Ck::Face)),
            );
            menu_action(
                ui,
                gui,
                "Center Lines",
                GuiAction::TechDraw(T::OpenCenterlineSetup(Ck::BetweenLines)),
            );
            menu_action(
                ui,
                gui,
                "Center Points",
                GuiAction::TechDraw(T::OpenCenterlineSetup(Ck::CenterMark)),
            );
            menu_action(
                ui,
                gui,
                "Bolt Circle",
                GuiAction::TechDraw(T::OpenCenterlineSetup(Ck::BoltCircle)),
            );
        });
        ui.separator();
        ui.menu_button("Export", |ui| {
            if ui.button("Export SVG...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("SVG", &["svg"])
                    .set_file_name("drawing.svg")
                    .save_file()
                {
                    gui.actions.push(GuiAction::TechDraw(T::ExportSvg(path)));
                }
                ui.close_menu();
            }
            if ui.button("Export DXF...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("DXF", &["dxf"])
                    .set_file_name("drawing.dxf")
                    .save_file()
                {
                    gui.actions.push(GuiAction::TechDraw(T::ExportDxf(path)));
                }
                ui.close_menu();
            }
            if ui.button("Export PDF...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("PDF", &["pdf"])
                    .set_file_name("drawing.pdf")
                    .save_file()
                {
                    gui.actions.push(GuiAction::TechDraw(T::ExportPdf(path)));
                }
                ui.close_menu();
            }
        });
        ui.separator();
        menu_action(ui, gui, "Clear All", GuiAction::TechDraw(T::Clear));
    });
}

fn draw_assembly_menu(ui: &mut egui::Ui, gui: &mut GuiState) {
    use super::AssemblyAction as A;
    ui.menu_button("Assembly", |ui| {
        menu_action(ui, gui, "Create Assembly", GuiAction::Assembly(A::Create));
        menu_action(
            ui,
            gui,
            "Insert Component",
            GuiAction::Assembly(A::InsertComponent),
        );
        ui.separator();
        menu_action(
            ui,
            gui,
            "Ground Component",
            GuiAction::Assembly(A::AddJoint(AssemblyJointType::Grounded)),
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
                menu_action(ui, gui, jt.label(), GuiAction::Assembly(A::AddJoint(jt)));
            }
        });
        ui.separator();
        menu_action(ui, gui, "Solve Assembly", GuiAction::Assembly(A::Solve));
        menu_action(
            ui,
            gui,
            "Exploded View",
            GuiAction::Assembly(A::Explode { factor: 2.0 }),
        );
        menu_action(
            ui,
            gui,
            "Bill of Materials",
            GuiAction::Assembly(A::BillOfMaterials),
        );
        menu_action(ui, gui, "DOF Analysis", GuiAction::Assembly(A::DofAnalysis));
    });
}

fn draw_draft_menu(ui: &mut egui::Ui, gui: &mut GuiState) {
    use super::DraftAction as D;
    ui.menu_button("Draft", |ui| {
        ui.menu_button("Drawing", |ui| {
            menu_action(ui, gui, "Line", GuiAction::Draft(D::Line));
            menu_action(ui, gui, "Wire", GuiAction::Draft(D::Wire));
            menu_action(ui, gui, "Circle", GuiAction::Draft(D::Circle));
            menu_action(ui, gui, "Arc", GuiAction::Draft(D::Arc));
            menu_action(ui, gui, "Ellipse", GuiAction::Draft(D::Ellipse));
            menu_action(ui, gui, "Rectangle", GuiAction::Draft(D::Rectangle));
            menu_action(ui, gui, "Polygon", GuiAction::Draft(D::Polygon));
            menu_action(ui, gui, "B-Spline", GuiAction::Draft(D::BSpline));
            menu_action(ui, gui, "Bezier", GuiAction::Draft(D::Bezier));
            menu_action(ui, gui, "Point", GuiAction::Draft(D::Point));
            menu_action(ui, gui, "Facebinder", GuiAction::Draft(D::Facebinder));
            menu_action(ui, gui, "Hatch", GuiAction::Draft(D::Hatch));
        });
        ui.menu_button("Modification", |ui| {
            menu_action(ui, gui, "Move", GuiAction::Draft(D::Move));
            menu_action(ui, gui, "Rotate", GuiAction::Draft(D::Rotate));
            menu_action(ui, gui, "Scale", GuiAction::Draft(D::Scale));
            menu_action(ui, gui, "Mirror", GuiAction::Draft(D::Mirror));
            menu_action(ui, gui, "Offset", GuiAction::Draft(D::Offset));
            menu_action(ui, gui, "Trim", GuiAction::Draft(D::Trim));
            menu_action(ui, gui, "Stretch", GuiAction::Draft(D::Stretch));
            menu_action(ui, gui, "Clone", GuiAction::Draft(D::Clone));
        });
        ui.menu_button("Arrays", |ui| {
            menu_action(ui, gui, "Rectangular Array", GuiAction::Draft(D::ArrayRect));
            menu_action(ui, gui, "Polar Array", GuiAction::Draft(D::ArrayPolar));
            menu_action(ui, gui, "Path Array", GuiAction::Draft(D::ArrayPath));
            menu_action(ui, gui, "Point Array", GuiAction::Draft(D::ArrayPoint));
        });
        ui.menu_button("Annotation", |ui| {
            menu_action(ui, gui, "Dimension", GuiAction::Draft(D::Dimension));
            menu_action(ui, gui, "Label", GuiAction::Draft(D::Label));
            menu_action(ui, gui, "Text", GuiAction::Draft(D::Text));
        });
        ui.menu_button("Conversion", |ui| {
            menu_action(ui, gui, "Upgrade", GuiAction::Draft(D::Upgrade));
            menu_action(ui, gui, "Downgrade", GuiAction::Draft(D::Downgrade));
            menu_action(
                ui,
                gui,
                "Wire to B-Spline",
                GuiAction::Draft(D::WireToBSpline),
            );
            menu_action(ui, gui, "Draft to Sketch", GuiAction::Draft(D::ToSketch));
        });
        ui.menu_button("Snap", |ui| {
            for snap in [
                "Midpoint",
                "Endpoint",
                "Center",
                "Perpendicular",
                "Grid",
                "Intersection",
                "Extension",
                "Nearest",
            ] {
                menu_action(
                    ui,
                    gui,
                    snap,
                    GuiAction::Draft(D::ToggleSnap(snap.to_lowercase())),
                );
            }
        });
    });
}

fn draw_surface_menu(ui: &mut egui::Ui, gui: &mut GuiState) {
    ui.menu_button("Surface", |ui| {
        use super::SurfaceAction as S;
        menu_action(ui, gui, "Filling", GuiAction::Surface(S::Filling));
        menu_action(ui, gui, "Boundary", GuiAction::Surface(S::Boundary));
        menu_action(ui, gui, "Sections", GuiAction::Surface(S::Sections));
        menu_action(ui, gui, "Extend", GuiAction::Surface(S::Extend));
        menu_action(ui, gui, "Blend", GuiAction::Surface(S::Blend));
        menu_action(ui, gui, "Pipe", GuiAction::Surface(S::Pipe));
        menu_action(ui, gui, "Coons Patch", GuiAction::Surface(S::Coons));
    });
}

fn draw_fem_menu(ui: &mut egui::Ui, gui: &mut GuiState) {
    use super::FemAction as F;
    ui.menu_button("FEM", |ui| {
        ui.menu_button("Analysis", |ui| {
            menu_action(ui, gui, "New Analysis", GuiAction::Fem(F::CreateAnalysis));
            menu_action(ui, gui, "Summary", GuiAction::Fem(F::Summary));
            menu_action(ui, gui, "Report", GuiAction::Fem(F::Report));
        });
        ui.menu_button("Material", |ui| {
            menu_action(
                ui,
                gui,
                "Pick Material...",
                GuiAction::Fem(F::OpenMaterialPicker),
            );
            ui.separator();
            menu_action(
                ui,
                gui,
                "Steel",
                GuiAction::Fem(F::SetMaterial("steel".into())),
            );
            menu_action(
                ui,
                gui,
                "Aluminum",
                GuiAction::Fem(F::SetMaterial("aluminum".into())),
            );
        });
        ui.menu_button("Boundary Conditions", |ui| {
            ui.menu_button("Loads", |ui| {
                menu_action(
                    ui,
                    gui,
                    "Force...",
                    GuiAction::Fem(F::OpenBcEditor(BcKind::Force)),
                );
                menu_action(
                    ui,
                    gui,
                    "Pressure...",
                    GuiAction::Fem(F::OpenBcEditor(BcKind::Pressure)),
                );
                menu_action(
                    ui,
                    gui,
                    "Gravity...",
                    GuiAction::Fem(F::OpenBcEditor(BcKind::Gravity)),
                );
                menu_action(
                    ui,
                    gui,
                    "Distributed Load...",
                    GuiAction::Fem(F::OpenBcEditor(BcKind::DistributedLoad)),
                );
                menu_action(
                    ui,
                    gui,
                    "Centrifugal Load...",
                    GuiAction::Fem(F::OpenBcEditor(BcKind::CentrifugalLoad)),
                );
                menu_action(
                    ui,
                    gui,
                    "Self Weight...",
                    GuiAction::Fem(F::OpenBcEditor(BcKind::SelfWeight)),
                );
                menu_action(
                    ui,
                    gui,
                    "Body Load...",
                    GuiAction::Fem(F::OpenBcEditor(BcKind::BodyLoad)),
                );
            });
            ui.menu_button("Constraints", |ui| {
                menu_action(
                    ui,
                    gui,
                    "Fixed Node...",
                    GuiAction::Fem(F::OpenBcEditor(BcKind::FixedNode)),
                );
                menu_action(
                    ui,
                    gui,
                    "Displacement...",
                    GuiAction::Fem(F::OpenBcEditor(BcKind::Displacement)),
                );
                menu_action(
                    ui,
                    gui,
                    "Spring...",
                    GuiAction::Fem(F::OpenBcEditor(BcKind::Spring)),
                );
                menu_action(
                    ui,
                    gui,
                    "Spring Constraint...",
                    GuiAction::Fem(F::OpenBcEditor(BcKind::SpringConstraint)),
                );
                menu_action(
                    ui,
                    gui,
                    "Section Print...",
                    GuiAction::Fem(F::OpenBcEditor(BcKind::SectionPrint)),
                );
                menu_action(
                    ui,
                    gui,
                    "Tie Constraint...",
                    GuiAction::Fem(F::OpenBcEditor(BcKind::TieConstraint)),
                );
                menu_action(
                    ui,
                    gui,
                    "Rigid Body...",
                    GuiAction::Fem(F::OpenBcEditor(BcKind::RigidBody)),
                );
                menu_action(
                    ui,
                    gui,
                    "Contact Constraint...",
                    GuiAction::Fem(F::OpenBcEditor(BcKind::ContactConstraint)),
                );
            });
            ui.menu_button("Thermal", |ui| {
                menu_action(
                    ui,
                    gui,
                    "Initial Temperature...",
                    GuiAction::Fem(F::OpenBcEditor(BcKind::InitialTemperature)),
                );
            });
        });
        ui.menu_button("Mesh", |ui| {
            menu_action(
                ui,
                gui,
                "Generate Tet Mesh",
                GuiAction::Fem(F::GenTetMesh { element_size: 1.0 }),
            );
            menu_action(
                ui,
                gui,
                "Generate Hex Mesh",
                GuiAction::Fem(F::GenHexMesh {
                    nx: 10,
                    ny: 10,
                    nz: 10,
                }),
            );
        });
        ui.menu_button("Constraints", |ui| {
            menu_action(
                ui,
                gui,
                "Fixed",
                GuiAction::Fem(F::AddConstraint(FemConstraintType::Fixed)),
            );
            menu_action(
                ui,
                gui,
                "Force",
                GuiAction::Fem(F::AddConstraint(FemConstraintType::Force)),
            );
            menu_action(
                ui,
                gui,
                "Pressure",
                GuiAction::Fem(F::AddConstraint(FemConstraintType::Pressure)),
            );
            menu_action(
                ui,
                gui,
                "Displacement",
                GuiAction::Fem(F::AddConstraint(FemConstraintType::Displacement)),
            );
            menu_action(
                ui,
                gui,
                "Gravity",
                GuiAction::Fem(F::AddConstraint(FemConstraintType::Gravity)),
            );
            menu_action(
                ui,
                gui,
                "Spring",
                GuiAction::Fem(F::AddConstraint(FemConstraintType::Spring)),
            );
        });
        ui.separator();
        ui.menu_button("Solve", |ui| {
            menu_action(ui, gui, "Static Analysis", GuiAction::Fem(F::SolveStatic));
            menu_action(
                ui,
                gui,
                "Modal Analysis",
                GuiAction::Fem(F::SolveModal { modes: 6 }),
            );
            menu_action(ui, gui, "Thermal Analysis", GuiAction::Fem(F::SolveThermal));
            menu_action(
                ui,
                gui,
                "Buckling Analysis",
                GuiAction::Fem(F::SolveBuckling { modes: 3 }),
            );
            menu_action(
                ui,
                gui,
                "Nonlinear Analysis",
                GuiAction::Fem(F::SolveNonlinear),
            );
        });
        ui.menu_button("Results", |ui| {
            menu_action(ui, gui, "Show Stress", GuiAction::Fem(F::ShowStress));
            menu_action(
                ui,
                gui,
                "Show Displacement",
                GuiAction::Fem(F::ShowDisplacement),
            );
            menu_action(ui, gui, "Show Von Mises", GuiAction::Fem(F::ShowVonMises));
            ui.separator();
            menu_action(ui, gui, "Probe Node...", GuiAction::Fem(F::OpenResultProbe));
            menu_action(
                ui,
                gui,
                "Result Table...",
                GuiAction::Fem(F::OpenResultTable),
            );
        });
    });
}
