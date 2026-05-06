use super::{GuiAction, GuiState, SelectedEntity, SelectionMode, task_panel};
use crate::scene::ObjectId;
use cadkernel_sketch::WorkPlane;

/// Dim section header label for context menus — matches FreeCAD style.
fn menu_section(ui: &mut egui::Ui, label: &str) {
    ui.label(
        egui::RichText::new(label)
            .size(10.0)
            .color(super::theme::MENU_SECTION_COLOR)
            .strong(),
    );
}

/// Menu item with icon prefix and optional shortcut suffix.
fn menu_item(ui: &mut egui::Ui, icon: &str, label: &str, shortcut: &str) -> bool {
    let text = if shortcut.is_empty() {
        format!("{icon}  {label}")
    } else {
        format!("{icon}  {label}    {shortcut}")
    };
    ui.button(text).clicked()
}

/// Context menu for a scene object (right-click in model tree or 3D viewport).
pub(crate) fn object_context_menu(
    ui: &mut egui::Ui,
    gui: &mut GuiState,
    id: ObjectId,
    obj_name: &str,
    obj_visible: bool,
) {
    menu_section(ui, "Selection");
    if menu_item(ui, "\u{1F4CC}", "Select", "") {
        gui.actions.push(GuiAction::SelectObject(id));
        ui.close_menu();
    }

    ui.separator();

    menu_section(ui, "Edit");
    if menu_item(ui, "\u{2398}", "Duplicate", "Ctrl+D") {
        gui.actions.push(GuiAction::DuplicateObject(id));
        ui.close_menu();
    }

    if menu_item(ui, "\u{270F}", "Rename\u{2026}", "F2") {
        gui.rename_edit = Some((id, obj_name.to_string()));
        ui.close_menu();
    }

    ui.separator();

    menu_section(ui, "Appearance");
    let (vis_icon, vis_label) = if obj_visible {
        ("\u{1F441}", "Hide")
    } else {
        ("\u{25CB}", "Show")
    };
    if menu_item(ui, vis_icon, vis_label, "H") {
        gui.actions.push(GuiAction::ToggleVisibility(id));
        ui.close_menu();
    }

    // Color picker
    ui.menu_button("Set Color", |ui| {
        let presets: &[(&str, [f32; 4])] = &[
            ("Red", [0.9, 0.2, 0.2, 1.0]),
            ("Green", [0.2, 0.8, 0.2, 1.0]),
            ("Blue", [0.3, 0.5, 0.9, 1.0]),
            ("Yellow", [0.9, 0.9, 0.2, 1.0]),
            ("Orange", [0.9, 0.6, 0.2, 1.0]),
            ("Cyan", [0.2, 0.8, 0.8, 1.0]),
            ("Gray", [0.6, 0.6, 0.6, 1.0]),
            ("White", [0.95, 0.95, 0.95, 1.0]),
        ];
        for (name, color) in presets {
            if ui.button(*name).clicked() {
                gui.actions
                    .push(GuiAction::SetObjectColor { id, color: *color });
                ui.close_menu();
            }
        }
    });

    ui.separator();

    // Transform submenu
    ui.menu_button("Transform", |ui| {
        if ui.button("Move +X (10)").clicked() {
            gui.actions.push(GuiAction::MoveObject {
                id,
                dx: 10.0,
                dy: 0.0,
                dz: 0.0,
            });
            ui.close_menu();
        }
        if ui.button("Move +Y (10)").clicked() {
            gui.actions.push(GuiAction::MoveObject {
                id,
                dx: 0.0,
                dy: 10.0,
                dz: 0.0,
            });
            ui.close_menu();
        }
        if ui.button("Move +Z (10)").clicked() {
            gui.actions.push(GuiAction::MoveObject {
                id,
                dx: 0.0,
                dy: 0.0,
                dz: 10.0,
            });
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Rotate X 90").clicked() {
            gui.actions.push(GuiAction::RotateObject {
                id,
                axis: 0,
                angle_deg: 90.0,
            });
            ui.close_menu();
        }
        if ui.button("Rotate Y 90").clicked() {
            gui.actions.push(GuiAction::RotateObject {
                id,
                axis: 1,
                angle_deg: 90.0,
            });
            ui.close_menu();
        }
        if ui.button("Rotate Z 90").clicked() {
            gui.actions.push(GuiAction::RotateObject {
                id,
                axis: 2,
                angle_deg: 90.0,
            });
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Scale 2x").clicked() {
            gui.actions
                .push(GuiAction::ScaleObjectUniform { id, factor: 2.0 });
            ui.close_menu();
        }
        if ui.button("Scale 0.5x").clicked() {
            gui.actions
                .push(GuiAction::ScaleObjectUniform { id, factor: 0.5 });
            ui.close_menu();
        }
    });

    ui.separator();

    menu_section(ui, "Analysis");
    if menu_item(ui, "\u{1F4CF}", "Measure", "") {
        gui.actions.push(GuiAction::MeasureSolid);
        ui.close_menu();
    }
    if menu_item(ui, "\u{2714}", "Check Geometry", "") {
        gui.actions.push(GuiAction::CheckGeometry);
        ui.close_menu();
    }

    ui.separator();

    // Workbench-specific operations
    ui.menu_button("Operations", |ui| {
        if ui.button("Mirror (YZ)").clicked() {
            gui.actions.push(GuiAction::SelectObject(id));
            gui.actions
                .push(GuiAction::MirrorSolid(super::MirrorPlane::YZ));
            ui.close_menu();
        }
        if ui.button("Shell (1mm)").clicked() {
            gui.actions.push(GuiAction::SelectObject(id));
            gui.actions.push(GuiAction::ShellSolid { thickness: 1.0 });
            ui.close_menu();
        }
        if ui.button("Fillet Edges").clicked() {
            gui.active_task = Some(task_panel::ActiveTask::Fillet {
                radius: 1.0,
                preview_id: None,
            });
            ui.close_menu();
        }
        if ui.button("Chamfer Edges").clicked() {
            gui.active_task = Some(task_panel::ActiveTask::Chamfer {
                distance: 1.0,
                preview_id: None,
            });
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Linear Pattern").clicked() {
            gui.active_task = Some(task_panel::ActiveTask::Pattern {
                count: 3,
                spacing: 15.0,
                axis: 0,
                preview_id: None,
            });
            ui.close_menu();
        }
    });

    // Export
    ui.menu_button("Export As\u{2026}", |ui| {
        let formats: &[(&str, &str, &[&str])] = &[
            ("STL", "model.stl", &["stl"]),
            ("OBJ", "model.obj", &["obj"]),
            ("glTF", "model.gltf", &["gltf"]),
            ("STEP", "model.step", &["step", "stp"]),
            ("IGES", "model.iges", &["iges", "igs"]),
            ("PLY", "model.ply", &["ply"]),
            ("3MF", "model.3mf", &["3mf"]),
            ("DXF", "model.dxf", &["dxf"]),
            ("BREP", "model.brep", &["brep"]),
        ];
        for &(label, filename, exts) in formats {
            if ui.button(format!("{label}\u{2026}")).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter(label, exts)
                    .set_file_name(filename)
                    .save_file()
                {
                    let action = match label {
                        "STL" => GuiAction::ExportStl(path),
                        "OBJ" => GuiAction::ExportObj(path),
                        "glTF" => GuiAction::ExportGltf(path),
                        "STEP" => GuiAction::ExportStep(path),
                        "IGES" => GuiAction::ExportIges(path),
                        "PLY" => GuiAction::ExportPly(path),
                        "3MF" => GuiAction::Export3mf(path),
                        "DXF" => GuiAction::ExportDxf(path),
                        _ => GuiAction::ExportBrep(path),
                    };
                    gui.actions.push(action);
                }
                ui.close_menu();
            }
        }
    });

    ui.separator();

    // Delete
    if menu_item(ui, "\u{1F5D1}", "Delete", "Del") {
        gui.actions.push(GuiAction::RemoveObject(id));
        ui.close_menu();
    }
}

pub(crate) fn viewport_context_menu(ui: &mut egui::Ui, gui: &mut GuiState) {
    use crate::render::{DisplayMode, StandardView};

    let has_objects = gui.tb_has_objects;

    menu_section(ui, "View");
    if menu_item(ui, "\u{1F50D}", "Fit All", "V") {
        gui.actions.push(GuiAction::FitAll);
        ui.close_menu();
    }
    if menu_item(ui, "\u{1F3E0}", "Reset Camera", "") {
        gui.actions.push(GuiAction::ResetCamera);
        ui.close_menu();
    }

    ui.separator();

    ui.menu_button("Standard Views", |ui| {
        for &(view, label) in &[
            (StandardView::Front, "Front"),
            (StandardView::Back, "Back"),
            (StandardView::Right, "Right"),
            (StandardView::Left, "Left"),
            (StandardView::Top, "Top"),
            (StandardView::Bottom, "Bottom"),
            (StandardView::Isometric, "Isometric"),
        ] {
            if ui.button(label).clicked() {
                gui.actions.push(GuiAction::SetStandardView(view));
                ui.close_menu();
            }
        }
    });

    ui.menu_button("Display Mode", |ui| {
        for &mode in DisplayMode::ALL {
            if ui.button(mode.label()).clicked() {
                gui.actions.push(GuiAction::SetDisplayMode(mode));
                ui.close_menu();
            }
        }
    });

    // -- Navigation Style submenu --
    ui.menu_button("Navigation Style", |ui| {
        for &style in &["CAD", "Blender", "Maya", "OpenInventor"] {
            if ui.button(style).clicked() {
                gui.actions.push(GuiAction::StatusMessage(format!(
                    "Navigation style: {style}"
                )));
                ui.close_menu();
            }
        }
    });

    ui.separator();

    menu_section(ui, "Display");
    if menu_item(ui, "\u{25A6}", "Toggle Grid", "G") {
        gui.actions.push(GuiAction::ToggleGrid);
        ui.close_menu();
    }
    if menu_item(ui, "\u{25A3}", "Toggle Projection", "5") {
        gui.actions.push(GuiAction::ToggleProjection);
        ui.close_menu();
    }

    ui.separator();

    menu_section(ui, "Overlays");
    if menu_item(ui, "\u{2725}", "Toggle Origin Axes", "") {
        gui.actions.push(GuiAction::ToggleOrigin);
        ui.close_menu();
    }
    if menu_item(ui, "\u{25A6}", "Toggle 3D Grid", "") {
        gui.actions.push(GuiAction::ToggleGrid3d);
        ui.close_menu();
    }
    if menu_item(ui, "\u{1F4CF}", "Toggle Measurement", "") {
        gui.actions.push(GuiAction::ToggleMeasurement);
        ui.close_menu();
    }

    // -- Clip Plane submenu --
    ui.menu_button("Clip Plane", |ui| {
        if ui.button("Enable").clicked() {
            gui.actions
                .push(GuiAction::StatusMessage("Clip plane enabled".into()));
            ui.close_menu();
        }
        if ui.button("Disable").clicked() {
            gui.actions
                .push(GuiAction::StatusMessage("Clip plane disabled".into()));
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Flip Direction").clicked() {
            gui.actions
                .push(GuiAction::StatusMessage("Clip plane flipped".into()));
            ui.close_menu();
        }
        ui.add_enabled_ui(gui.tb_has_selection, |ui| {
            if ui.button("Set to Selection").clicked() {
                gui.actions.push(GuiAction::StatusMessage(
                    "Clip plane set to selection".into(),
                ));
                ui.close_menu();
            }
        });
    });

    ui.separator();

    menu_section(ui, "Selection");
    ui.add_enabled_ui(has_objects, |ui| {
        if menu_item(ui, "\u{2610}", "Select All", "Ctrl+A") {
            gui.actions.push(GuiAction::SelectAll);
            ui.close_menu();
        }
    });
    if menu_item(ui, "\u{2612}", "Deselect All", "Esc") {
        gui.actions.push(GuiAction::DeselectAll);
        ui.close_menu();
    }

    ui.separator();

    menu_section(ui, "Create");
    // -- Quick create primitives --
    ui.menu_button("Create", |ui| {
        if ui.button("Box").clicked() {
            gui.active_task = Some(task_panel::ActiveTask::Box {
                width: 10.0,
                height: 10.0,
                depth: 10.0,
                preview_id: None,
            });
            ui.close_menu();
        }
        if ui.button("Cylinder").clicked() {
            gui.active_task = Some(task_panel::ActiveTask::Cylinder {
                radius: 5.0,
                height: 10.0,
                preview_id: None,
            });
            ui.close_menu();
        }
        if ui.button("Sphere").clicked() {
            gui.active_task = Some(task_panel::ActiveTask::Sphere {
                radius: 5.0,
                preview_id: None,
            });
            ui.close_menu();
        }
        if ui.button("Cone").clicked() {
            gui.active_task = Some(task_panel::ActiveTask::Cone {
                base_radius: 5.0,
                top_radius: 0.0,
                height: 10.0,
                preview_id: None,
            });
            ui.close_menu();
        }
        if ui.button("Torus").clicked() {
            gui.active_task = Some(task_panel::ActiveTask::Torus {
                major_radius: 5.0,
                minor_radius: 1.5,
                preview_id: None,
            });
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Sketch on XY").clicked() {
            gui.actions
                .push(GuiAction::Sketcher(super::SketcherAction::Enter(
                    WorkPlane::xy(),
                )));
            ui.close_menu();
        }
        if ui.button("Sketch on XZ").clicked() {
            gui.actions
                .push(GuiAction::Sketcher(super::SketcherAction::Enter(
                    WorkPlane::xz(),
                )));
            ui.close_menu();
        }
        if ui.button("Sketch on YZ").clicked() {
            gui.actions
                .push(GuiAction::Sketcher(super::SketcherAction::Enter(
                    WorkPlane::new(
                        cadkernel_math::Point3::ORIGIN,
                        cadkernel_math::Vec3::X,
                        cadkernel_math::Vec3::Y,
                    ),
                )));
            ui.close_menu();
        }
    });

    ui.separator();

    // -- Show/Hide All --
    ui.add_enabled_ui(has_objects, |ui| {
        if ui.button("Show All").clicked() {
            gui.actions.push(GuiAction::ShowAll);
            ui.close_menu();
        }
        if ui.button("Hide All").clicked() {
            gui.actions.push(GuiAction::HideAll);
            ui.close_menu();
        }
    });
}

/// Context menu for a tree item (right-click in model tree, extends object_context_menu).
pub(crate) fn tree_context_menu(
    ui: &mut egui::Ui,
    gui: &mut GuiState,
    id: ObjectId,
    obj_name: &str,
    obj_visible: bool,
) {
    // Include all object menu items
    object_context_menu(ui, gui, id, obj_name, obj_visible);

    ui.separator();

    // PartDesign feature tree operations
    use super::PartDesignAction as Pd;
    if ui.button("Suppress Feature").clicked() {
        gui.actions.push(GuiAction::PartDesign(Pd::SuppressFeature));
        ui.close_menu();
    }
    if ui.button("Set as Tip").clicked() {
        gui.actions.push(GuiAction::PartDesign(Pd::SetTip));
        ui.close_menu();
    }

    ui.separator();

    if ui.button("Move Up").clicked() {
        gui.actions.push(GuiAction::PartDesign(Pd::MoveFeatureUp));
        ui.close_menu();
    }
    if ui.button("Move Down").clicked() {
        gui.actions.push(GuiAction::PartDesign(Pd::MoveFeatureDown));
        ui.close_menu();
    }
}

/// Context menu for sub-element selection (face, edge, or vertex).
///
/// The menu adapts its contents based on the actual selected entity types,
/// not the selection mode — auto-pick means any element type can be selected.
pub(crate) fn face_edge_context_menu(ui: &mut egui::Ui, gui: &mut GuiState) {
    let has_sel = gui.tb_has_selection;

    // Determine context menu type from actually selected entities
    let has_face = gui
        .selected_entities
        .iter()
        .any(|e| matches!(e, SelectedEntity::Face(_)));
    let has_edge = gui
        .selected_entities
        .iter()
        .any(|e| matches!(e, SelectedEntity::Edge(_)));
    let has_vertex = gui
        .selected_entities
        .iter()
        .any(|e| matches!(e, SelectedEntity::Vertex(_)));

    if has_vertex {
        vertex_context_menu(ui, gui, has_sel);
    } else if has_edge {
        edge_context_menu(ui, gui, has_sel);
    } else if has_face {
        face_context_menu(ui, gui, has_sel);
    } else {
        // No sub-element selected — basic operations
        if ui.button("Measure").clicked() {
            gui.actions.push(GuiAction::ToggleMeasurement);
            ui.close_menu();
        }
        if ui.button("Check Geometry").clicked() {
            gui.actions.push(GuiAction::CheckGeometry);
            ui.close_menu();
        }
    }
}

// ---------------------------------------------------------------------------
// Face selection context menu
// ---------------------------------------------------------------------------

fn face_context_menu(ui: &mut egui::Ui, gui: &mut GuiState, has_sel: bool) {
    let has_face = gui
        .selected_entities
        .iter()
        .any(|e| matches!(e, SelectedEntity::Face(_)));

    ui.add_enabled_ui(has_face, |ui| {
        if ui.button("Select Face Loop").clicked() {
            gui.actions.push(GuiAction::SelectFaceLoop);
            ui.close_menu();
        }
    });

    ui.separator();

    ui.add_enabled_ui(has_face, |ui| {
        if ui.button("Create Sketch on Face").clicked() {
            gui.actions.push(GuiAction::Sketcher(
                super::SketcherAction::EnterOnSelectedFace,
            ));
            ui.close_menu();
        }
    });

    ui.separator();

    ui.add_enabled_ui(has_sel, |ui| {
        if ui.button("Fillet Adjacent Edges").clicked() {
            gui.actions.push(GuiAction::FilletAllEdges { radius: 1.0 });
            ui.close_menu();
        }
        if ui.button("Chamfer Adjacent Edges").clicked() {
            gui.actions
                .push(GuiAction::ChamferAllEdges { distance: 1.0 });
            ui.close_menu();
        }
        if ui.button("Shell (remove this face)").clicked() {
            gui.active_task = Some(task_panel::ActiveTask::Shell {
                thickness: 1.0,
                preview_id: None,
            });
            ui.close_menu();
        }
    });

    ui.separator();

    ui.add_enabled_ui(has_sel, |ui| {
        if ui.button("Set Face Color").clicked() {
            gui.actions
                .push(GuiAction::StatusMessage("Set face color".into()));
            ui.close_menu();
        }
        if ui.button("Measure Face Area").clicked() {
            gui.actions.push(GuiAction::ToggleMeasurement);
            ui.close_menu();
        }
    });

    ui.separator();

    if ui.button("Select Parent Object").clicked() {
        gui.selection_mode = SelectionMode::Solid;
        gui.actions.push(GuiAction::StatusMessage(
            "Switched to solid selection".into(),
        ));
        ui.close_menu();
    }
}

// ---------------------------------------------------------------------------
// Edge selection context menu
// ---------------------------------------------------------------------------

fn edge_context_menu(ui: &mut egui::Ui, gui: &mut GuiState, has_sel: bool) {
    let has_edge = gui
        .selected_entities
        .iter()
        .any(|e| matches!(e, SelectedEntity::Edge(_)));

    ui.add_enabled_ui(has_edge, |ui| {
        if ui.button("Select Edge Loop").clicked() {
            gui.actions.push(GuiAction::SelectEdgeLoop);
            ui.close_menu();
        }
        if ui.button("Select Edge Ring").clicked() {
            gui.actions.push(GuiAction::SelectEdgeRing);
            ui.close_menu();
        }
    });

    ui.separator();

    ui.add_enabled_ui(has_edge, |ui| {
        if ui.button("Fillet Selected Edges").clicked() {
            gui.active_task = Some(task_panel::ActiveTask::Fillet {
                radius: 1.0,
                preview_id: None,
            });
            ui.close_menu();
        }
        if ui.button("Chamfer Selected Edges").clicked() {
            gui.active_task = Some(task_panel::ActiveTask::Chamfer {
                distance: 1.0,
                preview_id: None,
            });
            ui.close_menu();
        }
    });

    ui.separator();

    ui.add_enabled_ui(has_sel, |ui| {
        if ui.button("Measure Edge Length").clicked() {
            gui.actions.push(GuiAction::ToggleMeasurement);
            ui.close_menu();
        }
    });

    ui.separator();

    if ui.button("Select Parent Object").clicked() {
        gui.selection_mode = SelectionMode::Solid;
        gui.actions.push(GuiAction::StatusMessage(
            "Switched to solid selection".into(),
        ));
        ui.close_menu();
    }
}

// ---------------------------------------------------------------------------
// Vertex selection context menu
// ---------------------------------------------------------------------------

fn vertex_context_menu(ui: &mut egui::Ui, gui: &mut GuiState, has_sel: bool) {
    ui.add_enabled_ui(has_sel, |ui| {
        if ui.button("Fillet at Vertex").clicked() {
            gui.actions.push(GuiAction::FilletAllEdges { radius: 1.0 });
            ui.close_menu();
        }
        if ui.button("Measure Point Coordinates").clicked() {
            gui.actions.push(GuiAction::ToggleMeasurement);
            ui.close_menu();
        }
    });

    ui.separator();

    if ui.button("Select Parent Object").clicked() {
        gui.selection_mode = SelectionMode::Solid;
        gui.actions.push(GuiAction::StatusMessage(
            "Switched to solid selection".into(),
        ));
        ui.close_menu();
    }
}
