use super::{GuiAction, GuiState, SelectionMode, SketchTool, Workbench};
use cadkernel_sketch::WorkPlane;

pub(crate) fn draw_toolbar(ctx: &egui::Context, gui: &mut GuiState) {
    egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;

            // -- File group --
            if ui.button("\u{1F4C4} New").on_hover_text("New model (Ctrl+N)").clicked() {
                gui.actions.push(GuiAction::NewModel);
            }
            if ui.button("\u{1F4C2} Open").on_hover_text("Open file (Ctrl+O)").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("CADKernel", &["cadk"])
                    .add_filter("STL", &["stl"])
                    .add_filter("OBJ", &["obj"])
                    .add_filter("All", &["*"])
                    .pick_file()
                {
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if ext == "cadk" || ext == "json" {
                        gui.actions.push(GuiAction::OpenFile(path));
                    } else {
                        gui.actions.push(GuiAction::ImportFile(path));
                    }
                }
            }
            if ui.button("\u{1F4BE} Save").on_hover_text("Save project (Ctrl+S)").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("CADKernel", &["cadk"])
                    .set_file_name("model.cadk")
                    .save_file()
                {
                    gui.actions.push(GuiAction::SaveFile(path));
                }
            }

            ui.separator();

            // -- Edit group --
            ui.separator();
            if ui.button("\u{21A9} Undo").on_hover_text("Undo (Ctrl+Z)").clicked() {
                gui.actions.push(GuiAction::Undo);
            }
            if ui.button("\u{21AA} Redo").on_hover_text("Redo (Ctrl+Y)").clicked() {
                gui.actions.push(GuiAction::Redo);
            }

            ui.separator();

            // -- View group --
            if ui.button("\u{1F50D} Fit All").on_hover_text("Fit all (F)").clicked() {
                gui.actions.push(GuiAction::FitAll);
            }
            if ui.button("\u{1F3E0} Reset").on_hover_text("Reset camera").clicked() {
                gui.actions.push(GuiAction::ResetCamera);
            }

            ui.separator();

            // -- Scene group --
            if ui.button("\u{1F441} Show All").on_hover_text("Show all objects").clicked() {
                gui.actions.push(GuiAction::ShowAll);
            }
            if ui.button("\u{1F6AB} Hide All").on_hover_text("Hide all objects").clicked() {
                gui.actions.push(GuiAction::HideAll);
            }

            ui.separator();

            // -- Selection mode --
            for &(mode, icon, tip) in &[
                (SelectionMode::Solid, "\u{25A3}", "Select solids"),
                (SelectionMode::Face, "\u{25A2}", "Select faces"),
                (SelectionMode::Edge, "\u{2500}", "Select edges"),
                (SelectionMode::Vertex, "\u{25CF}", "Select vertices"),
            ] {
                let sel = gui.selection_mode == mode;
                let btn = egui::Button::new(
                    egui::RichText::new(icon).size(14.0)
                ).selected(sel);
                if ui.add(btn).on_hover_text(tip).clicked() {
                    gui.selection_mode = mode;
                }
            }

            ui.separator();

            // Workbench dropdown selector
            let wb_label = gui.active_workbench.label();
            egui::ComboBox::from_id_salt("wb_selector")
                .selected_text(wb_label)
                .show_ui(ui, |ui| {
                    for &wb in Workbench::ALL {
                        ui.selectable_value(&mut gui.active_workbench, wb, wb.label());
                    }
                });
        });
    });
}

pub(crate) fn draw_workbench_tabs(ctx: &egui::Context, gui: &mut GuiState) {
    egui::TopBottomPanel::top("workbench_tabs").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for &wb in Workbench::ALL {
                let selected = gui.active_workbench == wb;
                let btn =
                    egui::Button::new(egui::RichText::new(wb.label()).strong()).selected(selected);
                if ui.add(btn).on_hover_text(format!("{} workbench", wb.label())).clicked() {
                    gui.active_workbench = wb;
                }
            }
        });
    });
}

pub(crate) fn draw_context_toolbar(ctx: &egui::Context, gui: &mut GuiState) {
    egui::TopBottomPanel::top("context_toolbar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            match gui.active_workbench {
                Workbench::Part => draw_part_toolbar(ui, gui),
                Workbench::PartDesign => draw_partdesign_toolbar(ui, gui),
                Workbench::Sketcher => draw_sketcher_toolbar(ui, gui),
                Workbench::Mesh => draw_mesh_toolbar(ui, gui),
                Workbench::TechDraw => draw_techdraw_toolbar(ui, gui),
                Workbench::Assembly => draw_assembly_toolbar(ui, gui),
                Workbench::Draft => draw_draft_toolbar(ui, gui),
                Workbench::Surface => draw_surface_toolbar(ui, gui),
                Workbench::Fem => draw_fem_toolbar(ui, gui),
            }
        });
    });
}

fn draw_part_toolbar(ui: &mut egui::Ui, gui: &mut GuiState) {
    use super::task_panel::ActiveTask;

    // -- Primitives (task panel for main 5, legacy dialogs for rest) --
    ui.weak("Primitives");
    if ui.button("\u{25A1} Box").on_hover_text("Create box (Task Panel)").clicked() {
        gui.active_task = Some(ActiveTask::Box { width: 10.0, height: 10.0, depth: 10.0, preview_id: None });
    }
    if ui.button("\u{25CB} Cylinder").on_hover_text("Create cylinder (Task Panel)").clicked() {
        gui.active_task = Some(ActiveTask::Cylinder { radius: 5.0, height: 10.0, preview_id: None });
    }
    if ui.button("\u{25CF} Sphere").on_hover_text("Create sphere (Task Panel)").clicked() {
        gui.active_task = Some(ActiveTask::Sphere { radius: 5.0, preview_id: None });
    }
    if ui.button("\u{25B3} Cone").on_hover_text("Create cone (Task Panel)").clicked() {
        gui.active_task = Some(ActiveTask::Cone { base_radius: 5.0, top_radius: 0.0, height: 10.0, preview_id: None });
    }
    if ui.button("\u{25CE} Torus").on_hover_text("Create torus (Task Panel)").clicked() {
        gui.active_task = Some(ActiveTask::Torus { major_radius: 10.0, minor_radius: 3.0, preview_id: None });
    }
    // Legacy dialog primitives
    for (label, show_flag, tip) in [
        ("Tube", &mut gui.show_create_tube as &mut bool, "Create a hollow tube"),
        ("Prism", &mut gui.show_create_prism, "Create a regular polygon prism"),
        ("Wedge", &mut gui.show_create_wedge, "Create a wedge or pyramid"),
        ("Ellipsoid", &mut gui.show_create_ellipsoid, "Create an ellipsoid"),
        ("Helix", &mut gui.show_create_helix, "Create a helical coil"),
    ] {
        if ui.button(label).on_hover_text(tip).clicked() {
            *show_flag = true;
        }
    }

    ui.separator();

    // -- Boolean --
    ui.weak("Boolean");
    if ui.button("Union…").on_hover_text("Boolean union with a second solid").clicked() {
        gui.show_boolean_union = true;
    }
    if ui.button("Subtract…").on_hover_text("Boolean subtraction with a second solid").clicked() {
        gui.show_boolean_subtract = true;
    }
    if ui.button("Intersect…").on_hover_text("Boolean intersection with a second solid").clicked() {
        gui.show_boolean_intersect = true;
    }

    ui.separator();

    // -- Transforms --
    ui.weak("Transform");
    if ui.button("Mirror…").on_hover_text("Mirror solid across a plane").clicked() {
        gui.show_mirror = true;
    }
    if ui.button("Scale…").on_hover_text("Scale solid uniformly").clicked() {
        gui.show_scale = true;
    }
    if ui.button("Shell…").on_hover_text("Hollow out a solid").clicked() {
        gui.show_shell = true;
    }
    if ui.button("Fillet…").on_hover_text("Round all edges").clicked() {
        gui.show_fillet = true;
    }
    if ui.button("Chamfer…").on_hover_text("Chamfer all edges").clicked() {
        gui.show_chamfer = true;
    }
    if ui.button("Pattern…").on_hover_text("Create a linear pattern").clicked() {
        gui.show_pattern = true;
    }

    ui.separator();

    // -- Analysis --
    ui.weak("Analysis");
    if ui.button("\u{1F4CF} Measure").on_hover_text("Compute mass properties").clicked() {
        gui.actions.push(GuiAction::MeasureSolid);
    }
    if ui.button("\u{2714} Check").on_hover_text("Check geometry validity").clicked() {
        gui.actions.push(GuiAction::CheckGeometry);
    }
}

fn draw_partdesign_toolbar(ui: &mut egui::Ui, gui: &mut GuiState) {
    ui.weak("Dress-up");
    if ui.button("\u{25E0} Fillet…").on_hover_text("Round edges").clicked() {
        gui.show_fillet = true;
    }
    if ui.button("\u{2571} Chamfer…").on_hover_text("Bevel edges").clicked() {
        gui.show_chamfer = true;
    }
    if ui.button("\u{25A1} Shell…").on_hover_text("Hollow out solid").clicked() {
        gui.show_shell = true;
    }
    ui.separator();
    ui.weak("Transform");
    if ui.button("\u{2194} Mirror…").on_hover_text("Mirror solid").clicked() {
        gui.show_mirror = true;
    }
    if ui.button("\u{2922} Scale…").on_hover_text("Scale solid").clicked() {
        gui.show_scale = true;
    }
    if ui.button("\u{2261} Pattern…").on_hover_text("Linear pattern").clicked() {
        gui.show_pattern = true;
    }
    ui.separator();
    ui.weak("Boolean");
    if ui.button("\u{222A} Union…").on_hover_text("Boolean union").clicked() {
        gui.show_boolean_union = true;
    }
    if ui.button("\u{2212} Subtract…").on_hover_text("Boolean subtraction").clicked() {
        gui.show_boolean_subtract = true;
    }
}

fn draw_sketcher_toolbar(ui: &mut egui::Ui, gui: &mut GuiState) {
    let in_sketch = gui.sketch_mode.is_some();
    if !in_sketch {
        ui.weak("New Sketch");
        if ui.button("\u{25AD} XY Plane").on_hover_text("Sketch on XY plane").clicked() {
            gui.actions.push(GuiAction::EnterSketch(WorkPlane::xy()));
        }
        if ui.button("\u{25AF} XZ Plane").on_hover_text("Sketch on XZ plane").clicked() {
            gui.actions.push(GuiAction::EnterSketch(WorkPlane::xz()));
        }
    } else {
        ui.weak("Draw");
        let current_tool = gui
            .sketch_mode
            .as_ref()
            .map(|s| s.tool)
            .unwrap_or(SketchTool::Select);
        for (label, tool, tip) in [
            ("\u{25B9} Select", SketchTool::Select, "Selection tool"),
            ("\u{2500} Line", SketchTool::Line, "Draw line (L)"),
            ("\u{25A1} Rect", SketchTool::Rectangle, "Draw rectangle (R)"),
            ("\u{25CB} Circle", SketchTool::Circle, "Draw circle (C)"),
            ("\u{25E0} Arc", SketchTool::Arc, "Draw arc (A)"),
        ] {
            let btn = egui::Button::new(label);
            if ui.add(btn.selected(current_tool == tool)).on_hover_text(tip).clicked() {
                gui.actions.push(GuiAction::SetSketchTool(tool));
            }
        }
        ui.separator();
        ui.weak("Constrain");
        if ui.button("\u{2194} H").on_hover_text("Horizontal constraint").clicked() {
            gui.actions.push(GuiAction::SketchConstrainHorizontal);
        }
        if ui.button("\u{2195} V").on_hover_text("Vertical constraint").clicked() {
            gui.actions.push(GuiAction::SketchConstrainVertical);
        }
        if ui.button("\u{21A6} L").on_hover_text("Length constraint").clicked() {
            gui.actions
                .push(GuiAction::SketchConstrainLength(gui.constraint_length_value));
        }
        ui.add(
            egui::DragValue::new(&mut gui.constraint_length_value)
                .speed(0.1)
                .prefix("L: "),
        );
        ui.separator();
        if ui.button("\u{2714} Close").on_hover_text("Solve and extrude sketch").clicked() {
            gui.actions.push(GuiAction::CloseSketch);
        }
        if ui.button("\u{2716} Cancel").on_hover_text("Discard sketch").clicked() {
            gui.actions.push(GuiAction::CancelSketch);
        }
    }
}

fn draw_mesh_toolbar(ui: &mut egui::Ui, gui: &mut GuiState) {
    ui.weak("I/O");
    if ui.button("\u{1F4C2} Import STL").on_hover_text("Import STL mesh").clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("STL", &["stl"])
            .pick_file()
        {
            gui.actions.push(GuiAction::ImportFile(path));
        }
    }
    if ui.button("\u{1F4BE} Export STL").on_hover_text("Export as STL").clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("STL", &["stl"])
            .set_file_name("model.stl")
            .save_file()
        {
            gui.actions.push(GuiAction::ExportStl(path));
        }
    }
    if ui.button("\u{1F4BE} Export OBJ").on_hover_text("Export as OBJ").clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("OBJ", &["obj"])
            .set_file_name("model.obj")
            .save_file()
        {
            gui.actions.push(GuiAction::ExportObj(path));
        }
    }
    if ui.button("\u{1F4BE} Export glTF").on_hover_text("Export as glTF").clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("glTF", &["gltf"])
            .set_file_name("model.gltf")
            .save_file()
        {
            gui.actions.push(GuiAction::ExportGltf(path));
        }
    }

    ui.separator();
    ui.weak("Repair");
    if ui.button("\u{25BC} Decimate").on_hover_text("Reduce triangles 50%").clicked() {
        gui.actions.push(GuiAction::MeshDecimate(0.5));
    }
    if ui.button("\u{25B2} Subdivide").on_hover_text("Subdivide mesh").clicked() {
        gui.actions.push(GuiAction::MeshSubdivide);
    }
    if ui.button("\u{25C7} Fill Holes").on_hover_text("Fill boundary holes").clicked() {
        gui.actions.push(GuiAction::MeshFillHoles);
    }
    if ui.button("\u{21C5} Flip Normals").on_hover_text("Reverse normals").clicked() {
        gui.actions.push(GuiAction::MeshFlipNormals);
    }
    if ui.button("\u{223F} Smooth…").on_hover_text("Laplacian smoothing").clicked() {
        gui.show_mesh_smooth = true;
    }
    if ui.button("\u{21BB} Harmonize").on_hover_text("Consistent normals").clicked() {
        gui.actions.push(GuiAction::MeshHarmonizeNormals);
    }
    if ui.button("\u{2714} Watertight?").on_hover_text("Check watertight").clicked() {
        gui.actions.push(GuiAction::MeshCheckWatertight);
    }
    if ui.button("\u{25A6} Remesh…").on_hover_text("Remesh to target edge").clicked() {
        gui.show_mesh_remesh = true;
    }
    if ui.button("\u{1F527} Repair").on_hover_text("Auto-repair mesh").clicked() {
        gui.actions.push(GuiAction::MeshRepair);
    }
}

fn draw_techdraw_toolbar(ui: &mut egui::Ui, gui: &mut GuiState) {
    use cadkernel_io::ProjectionDir;

    ui.weak("Views");
    if ui.button("\u{25A3} Front").on_hover_text("Front projection").clicked() {
        gui.actions
            .push(GuiAction::TechDrawAddView(ProjectionDir::Front));
    }
    if ui.button("\u{25A2} Top").on_hover_text("Top projection").clicked() {
        gui.actions
            .push(GuiAction::TechDrawAddView(ProjectionDir::Top));
    }
    if ui.button("\u{25A1} Right").on_hover_text("Right projection").clicked() {
        gui.actions
            .push(GuiAction::TechDrawAddView(ProjectionDir::Right));
    }
    if ui.button("\u{25C7} Iso").on_hover_text("Isometric projection").clicked() {
        gui.actions
            .push(GuiAction::TechDrawAddView(ProjectionDir::Isometric));
    }
    ui.separator();
    if ui.button("\u{25A8} 3-View").on_hover_text("Standard 3-view drawing").clicked() {
        gui.actions.push(GuiAction::TechDrawThreeView);
    }
    ui.separator();
    ui.weak("Export");
    if ui.button("\u{1F4BE} Export SVG").on_hover_text("Export to SVG").clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("SVG", &["svg"])
            .set_file_name("drawing.svg")
            .save_file()
        {
            gui.actions.push(GuiAction::TechDrawExportSvg(path));
        }
    }
    if ui.button("\u{1F5D1} Clear").on_hover_text("Clear all views").clicked() {
        gui.actions.push(GuiAction::TechDrawClear);
    }
}

fn draw_assembly_toolbar(ui: &mut egui::Ui, gui: &mut GuiState) {
    ui.weak("Assembly");
    for (tool, tip) in [
        ("\u{2795} Insert", "Insert component"),
        ("\u{1F4CC} Fixed", "Fix position"),
        ("\u{25CE} Coincident", "Coincident constraint"),
        ("\u{25CB} Concentric", "Concentric constraint"),
        ("\u{21A6} Distance", "Distance constraint"),
    ] {
        if ui.button(tool).on_hover_text(tip).clicked() {
            gui.actions.push(GuiAction::StatusMessage(format!(
                "Assembly {tool}: not yet implemented"
            )));
        }
    }
}

fn draw_draft_toolbar(ui: &mut egui::Ui, gui: &mut GuiState) {
    ui.weak("Wire");
    if ui.button("\u{2500} Wire").on_hover_text("Create wire from points").clicked() {
        gui.actions.push(GuiAction::StatusMessage(
            "Draft Wire: select points to create a wire".into(),
        ));
    }
    if ui.button("\u{223F} BSpline").on_hover_text("B-spline wire").clicked() {
        gui.actions.push(GuiAction::StatusMessage(
            "Draft BSpline Wire: not yet implemented".into(),
        ));
    }
    ui.separator();
    ui.weak("Modify");
    if ui.button("\u{1F4CB} Clone").on_hover_text("Clone solid").clicked() {
        gui.actions.push(GuiAction::StatusMessage(
            "Draft Clone: not yet implemented".into(),
        ));
    }
    ui.separator();
    ui.weak("Array");
    if ui.button("\u{25A6} Rect Array").on_hover_text("Rectangular array").clicked() {
        gui.actions.push(GuiAction::StatusMessage(
            "Draft Rectangular Array: not yet implemented".into(),
        ));
    }
    if ui.button("\u{21DD} Path Array").on_hover_text("Path array").clicked() {
        gui.actions.push(GuiAction::StatusMessage(
            "Draft Path Array: not yet implemented".into(),
        ));
    }
}

fn draw_surface_toolbar(ui: &mut egui::Ui, gui: &mut GuiState) {
    ui.weak("Surface");
    if ui.button("\u{25B1} Ruled").on_hover_text("Ruled surface between curves").clicked() {
        gui.actions.push(GuiAction::StatusMessage(
            "Surface Ruled: not yet implemented".into(),
        ));
    }
    if ui.button("\u{25A8} From Curves").on_hover_text("Surface from curves").clicked() {
        gui.actions.push(GuiAction::StatusMessage(
            "Surface From Curves: not yet implemented".into(),
        ));
    }
    if ui.button("\u{21A6} Extend").on_hover_text("Extend surface").clicked() {
        gui.actions.push(GuiAction::StatusMessage(
            "Surface Extend: not yet implemented".into(),
        ));
    }
    if ui.button("\u{25CB} Pipe").on_hover_text("Pipe surface along path").clicked() {
        gui.actions.push(GuiAction::StatusMessage(
            "Surface Pipe: not yet implemented".into(),
        ));
    }
}

fn draw_fem_toolbar(ui: &mut egui::Ui, gui: &mut GuiState) {
    ui.weak("Mesh");
    if ui.button("\u{25A6} Gen Tet Mesh").on_hover_text("Generate tet FEM mesh").clicked() {
        gui.actions.push(GuiAction::StatusMessage(
            "FEM: Tet mesh generation not yet wired to UI".into(),
        ));
    }
    ui.separator();
    ui.weak("Analysis");
    if ui.button("\u{2206} Static").on_hover_text("Static structural analysis").clicked() {
        gui.actions.push(GuiAction::StatusMessage(
            "FEM: Static analysis not yet wired to UI".into(),
        ));
    }
    ui.separator();
    ui.weak("Material");
    if ui.button("\u{1F3A8} Material").on_hover_text("Assign material").clicked() {
        gui.actions.push(GuiAction::StatusMessage(
            "FEM: Material assignment not yet wired to UI".into(),
        ));
    }
}
