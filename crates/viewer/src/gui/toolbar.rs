use super::{GuiAction, GuiState, SketchTool, Workbench};
use cadkernel_sketch::WorkPlane;

pub(crate) fn draw_toolbar(ctx: &egui::Context, gui: &mut GuiState) {
    egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;

            // -- File group --
            ui.weak("File");
            if ui.button("New").on_hover_text("Create a new empty model").clicked() {
                gui.actions.push(GuiAction::NewModel);
            }
            if ui.button("Open").on_hover_text("Open a project or mesh file").clicked() {
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
            if ui.button("Save").on_hover_text("Save the current project").clicked() {
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
            ui.weak("Edit");
            if ui.button("Undo").on_hover_text("Undo last operation (Ctrl+Z)").clicked() {
                gui.actions.push(GuiAction::Undo);
            }
            if ui.button("Redo").on_hover_text("Redo last undone operation (Ctrl+Y)").clicked() {
                gui.actions.push(GuiAction::Redo);
            }

            ui.separator();

            // -- View group --
            ui.weak("View");
            if ui.button("Fit All").on_hover_text("Fit all geometry in view (V)").clicked() {
                gui.actions.push(GuiAction::FitAll);
            }
            if ui.button("Reset View").on_hover_text("Reset camera to default position").clicked() {
                gui.actions.push(GuiAction::ResetCamera);
            }
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
    // -- Primitives --
    ui.weak("Primitives");
    for (label, show_flag, tip) in [
        ("Box", &mut gui.show_create_box as &mut bool, "Create a box primitive"),
        ("Cylinder", &mut gui.show_create_cylinder, "Create a cylinder primitive"),
        ("Sphere", &mut gui.show_create_sphere, "Create a sphere primitive"),
        ("Cone", &mut gui.show_create_cone, "Create a cone or frustum"),
        ("Torus", &mut gui.show_create_torus, "Create a torus primitive"),
        ("Tube", &mut gui.show_create_tube, "Create a hollow tube"),
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
    if ui.button("Measure").on_hover_text("Compute mass properties").clicked() {
        gui.actions.push(GuiAction::MeasureSolid);
    }
    if ui.button("Check").on_hover_text("Check geometry validity").clicked() {
        gui.actions.push(GuiAction::CheckGeometry);
    }
}

fn draw_partdesign_toolbar(ui: &mut egui::Ui, gui: &mut GuiState) {
    ui.weak("Dress-up");
    if ui.button("Fillet…").on_hover_text("Fillet all edges").clicked() {
        gui.show_fillet = true;
    }
    if ui.button("Chamfer…").on_hover_text("Chamfer all edges").clicked() {
        gui.show_chamfer = true;
    }
    if ui.button("Shell…").on_hover_text("Hollow out solid").clicked() {
        gui.show_shell = true;
    }
    ui.separator();
    ui.weak("Transform");
    if ui.button("Mirror…").on_hover_text("Mirror solid").clicked() {
        gui.show_mirror = true;
    }
    if ui.button("Scale…").on_hover_text("Scale solid").clicked() {
        gui.show_scale = true;
    }
    if ui.button("Pattern…").on_hover_text("Linear pattern").clicked() {
        gui.show_pattern = true;
    }
    ui.separator();
    ui.weak("Boolean");
    if ui.button("Union…").on_hover_text("Boolean union").clicked() {
        gui.show_boolean_union = true;
    }
    if ui.button("Subtract…").on_hover_text("Boolean subtraction").clicked() {
        gui.show_boolean_subtract = true;
    }
}

fn draw_sketcher_toolbar(ui: &mut egui::Ui, gui: &mut GuiState) {
    let in_sketch = gui.sketch_mode.is_some();
    if !in_sketch {
        ui.weak("New Sketch");
        if ui.button("XY Plane").on_hover_text("Start a sketch on the XY plane").clicked() {
            gui.actions.push(GuiAction::EnterSketch(WorkPlane::xy()));
        }
        if ui.button("XZ Plane").on_hover_text("Start a sketch on the XZ plane").clicked() {
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
            ("Select", SketchTool::Select, "Selection tool"),
            ("Line", SketchTool::Line, "Draw a line segment"),
            ("Rectangle", SketchTool::Rectangle, "Draw a rectangle"),
            ("Circle", SketchTool::Circle, "Draw a circle"),
            ("Arc", SketchTool::Arc, "Draw an arc"),
        ] {
            let btn = egui::Button::new(label);
            if ui.add(btn.selected(current_tool == tool)).on_hover_text(tip).clicked() {
                gui.actions.push(GuiAction::SetSketchTool(tool));
            }
        }
        ui.separator();
        ui.weak("Constrain");
        if ui.button("Horizontal").on_hover_text("Add horizontal constraint").clicked() {
            gui.actions.push(GuiAction::SketchConstrainHorizontal);
        }
        if ui.button("Vertical").on_hover_text("Add vertical constraint").clicked() {
            gui.actions.push(GuiAction::SketchConstrainVertical);
        }
        if ui.button("Length").on_hover_text("Add length constraint").clicked() {
            gui.actions
                .push(GuiAction::SketchConstrainLength(gui.constraint_length_value));
        }
        ui.add(
            egui::DragValue::new(&mut gui.constraint_length_value)
                .speed(0.1)
                .prefix("L: "),
        );
        ui.separator();
        if ui.button("Close Sketch").on_hover_text("Solve and extrude the sketch").clicked() {
            gui.actions.push(GuiAction::CloseSketch);
        }
        if ui.button("Cancel").on_hover_text("Discard sketch changes").clicked() {
            gui.actions.push(GuiAction::CancelSketch);
        }
    }
}

fn draw_mesh_toolbar(ui: &mut egui::Ui, gui: &mut GuiState) {
    ui.weak("I/O");
    if ui.button("Import STL").on_hover_text("Import an STL mesh file").clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("STL", &["stl"])
            .pick_file()
        {
            gui.actions.push(GuiAction::ImportFile(path));
        }
    }
    if ui.button("Export STL").on_hover_text("Export model as STL").clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("STL", &["stl"])
            .set_file_name("model.stl")
            .save_file()
        {
            gui.actions.push(GuiAction::ExportStl(path));
        }
    }
    if ui.button("Export OBJ").on_hover_text("Export model as OBJ").clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("OBJ", &["obj"])
            .set_file_name("model.obj")
            .save_file()
        {
            gui.actions.push(GuiAction::ExportObj(path));
        }
    }
    if ui.button("Export glTF").on_hover_text("Export model as glTF").clicked() {
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
    if ui.button("Decimate 50%").on_hover_text("Reduce mesh triangle count by half").clicked() {
        gui.actions.push(GuiAction::MeshDecimate(0.5));
    }
    if ui.button("Subdivide").on_hover_text("Subdivide mesh (Loop subdivision)").clicked() {
        gui.actions.push(GuiAction::MeshSubdivide);
    }
    if ui.button("Fill Holes").on_hover_text("Fill boundary holes in mesh").clicked() {
        gui.actions.push(GuiAction::MeshFillHoles);
    }
    if ui.button("Flip Normals").on_hover_text("Reverse all face normals").clicked() {
        gui.actions.push(GuiAction::MeshFlipNormals);
    }
    if ui.button("Smooth…").on_hover_text("Laplacian mesh smoothing").clicked() {
        gui.show_mesh_smooth = true;
    }
    if ui.button("Harmonize").on_hover_text("Make face normals consistent").clicked() {
        gui.actions.push(GuiAction::MeshHarmonizeNormals);
    }
    if ui.button("Watertight?").on_hover_text("Check if mesh is watertight").clicked() {
        gui.actions.push(GuiAction::MeshCheckWatertight);
    }
    if ui.button("Remesh…").on_hover_text("Remesh to target edge length").clicked() {
        gui.show_mesh_remesh = true;
    }
    if ui.button("Repair").on_hover_text("Auto-repair mesh issues").clicked() {
        gui.actions.push(GuiAction::MeshRepair);
    }
}

fn draw_techdraw_toolbar(ui: &mut egui::Ui, gui: &mut GuiState) {
    use cadkernel_io::ProjectionDir;

    ui.weak("Views");
    if ui.button("Front").on_hover_text("Add front projection view").clicked() {
        gui.actions
            .push(GuiAction::TechDrawAddView(ProjectionDir::Front));
    }
    if ui.button("Top").on_hover_text("Add top projection view").clicked() {
        gui.actions
            .push(GuiAction::TechDrawAddView(ProjectionDir::Top));
    }
    if ui.button("Right").on_hover_text("Add right projection view").clicked() {
        gui.actions
            .push(GuiAction::TechDrawAddView(ProjectionDir::Right));
    }
    if ui.button("Iso").on_hover_text("Add isometric projection view").clicked() {
        gui.actions
            .push(GuiAction::TechDrawAddView(ProjectionDir::Isometric));
    }
    ui.separator();
    if ui.button("3-View").on_hover_text("Generate standard 3-view drawing").clicked() {
        gui.actions.push(GuiAction::TechDrawThreeView);
    }
    ui.separator();
    ui.weak("Export");
    if ui.button("Export SVG").on_hover_text("Export drawing sheet to SVG").clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("SVG", &["svg"])
            .set_file_name("drawing.svg")
            .save_file()
        {
            gui.actions.push(GuiAction::TechDrawExportSvg(path));
        }
    }
    if ui.button("Clear").on_hover_text("Clear all drawing views").clicked() {
        gui.actions.push(GuiAction::TechDrawClear);
    }
}

fn draw_assembly_toolbar(ui: &mut egui::Ui, gui: &mut GuiState) {
    ui.weak("Assembly");
    for (tool, tip) in [
        ("Insert Component", "Insert a component into the assembly"),
        ("Fixed", "Fix component position"),
        ("Coincident", "Add coincident constraint"),
        ("Concentric", "Add concentric constraint"),
        ("Distance", "Add distance constraint"),
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
    if ui.button("Wire").on_hover_text("Create a wire from points").clicked() {
        gui.actions.push(GuiAction::StatusMessage(
            "Draft Wire: select points to create a wire".into(),
        ));
    }
    if ui.button("BSpline Wire").on_hover_text("Create a B-spline wire").clicked() {
        gui.actions.push(GuiAction::StatusMessage(
            "Draft BSpline Wire: not yet implemented".into(),
        ));
    }
    ui.separator();
    ui.weak("Modify");
    if ui.button("Clone").on_hover_text("Clone the current solid").clicked() {
        gui.actions.push(GuiAction::StatusMessage(
            "Draft Clone: not yet implemented".into(),
        ));
    }
    ui.separator();
    ui.weak("Array");
    if ui.button("Rect Array").on_hover_text("Create a rectangular array").clicked() {
        gui.actions.push(GuiAction::StatusMessage(
            "Draft Rectangular Array: not yet implemented".into(),
        ));
    }
    if ui.button("Path Array").on_hover_text("Create a path array").clicked() {
        gui.actions.push(GuiAction::StatusMessage(
            "Draft Path Array: not yet implemented".into(),
        ));
    }
}

fn draw_surface_toolbar(ui: &mut egui::Ui, gui: &mut GuiState) {
    ui.weak("Surface");
    if ui.button("Ruled Surface").on_hover_text("Create a ruled surface between two curves").clicked() {
        gui.actions.push(GuiAction::StatusMessage(
            "Surface Ruled: not yet implemented".into(),
        ));
    }
    if ui.button("From Curves").on_hover_text("Create surface from curve network").clicked() {
        gui.actions.push(GuiAction::StatusMessage(
            "Surface From Curves: not yet implemented".into(),
        ));
    }
    if ui.button("Extend").on_hover_text("Extend a surface").clicked() {
        gui.actions.push(GuiAction::StatusMessage(
            "Surface Extend: not yet implemented".into(),
        ));
    }
    if ui.button("Pipe").on_hover_text("Create a pipe surface along a path").clicked() {
        gui.actions.push(GuiAction::StatusMessage(
            "Surface Pipe: not yet implemented".into(),
        ));
    }
}

fn draw_fem_toolbar(ui: &mut egui::Ui, gui: &mut GuiState) {
    ui.weak("Mesh");
    if ui.button("Generate Tet Mesh").on_hover_text("Generate a tetrahedral FEM mesh").clicked() {
        gui.actions.push(GuiAction::StatusMessage(
            "FEM: Tet mesh generation not yet wired to UI".into(),
        ));
    }
    ui.separator();
    ui.weak("Analysis");
    if ui.button("Static Analysis").on_hover_text("Run static structural analysis").clicked() {
        gui.actions.push(GuiAction::StatusMessage(
            "FEM: Static analysis not yet wired to UI".into(),
        ));
    }
    ui.separator();
    ui.weak("Material");
    if ui.button("Assign Material").on_hover_text("Assign material properties").clicked() {
        gui.actions.push(GuiAction::StatusMessage(
            "FEM: Material assignment not yet wired to UI".into(),
        ));
    }
}
