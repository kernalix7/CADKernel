use super::{GuiAction, GuiState, MirrorPlane};
use crate::nav::{NavConfig, NavStyle};
use crate::render::Projection;

pub(crate) fn draw_create_dialogs(ctx: &egui::Context, gui: &mut GuiState) {
    // --- Box ---
    let mut show_box = gui.show_create_box;
    if show_box {
        egui::Window::new("Create Box")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_box)
            .show(ctx, |ui| {
                egui::Grid::new("box_grid").show(ui, |ui| {
                    ui.label("Width:");
                    ui.add(egui::DragValue::new(&mut gui.create_box_size[0]).speed(0.1));
                    ui.end_row();
                    ui.label("Height:");
                    ui.add(egui::DragValue::new(&mut gui.create_box_size[1]).speed(0.1));
                    ui.end_row();
                    ui.label("Depth:");
                    ui.add(egui::DragValue::new(&mut gui.create_box_size[2]).speed(0.1));
                    ui.end_row();
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        let [w, h, d] = gui.create_box_size;
                        gui.actions.push(GuiAction::CreateBox {
                            width: w,
                            height: h,
                            depth: d,
                        });
                        gui.show_create_box = false;
                    }
                    if ui.button("Cancel").clicked() {
                        gui.show_create_box = false;
                    }
                });
            });
    }
    gui.show_create_box = show_box;

    // --- Cylinder ---
    let mut show_cyl = gui.show_create_cylinder;
    if show_cyl {
        egui::Window::new("Create Cylinder")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_cyl)
            .show(ctx, |ui| {
                egui::Grid::new("cyl_grid").show(ui, |ui| {
                    ui.label("Radius:");
                    ui.add(egui::DragValue::new(&mut gui.create_cylinder_radius).speed(0.1));
                    ui.end_row();
                    ui.label("Height:");
                    ui.add(egui::DragValue::new(&mut gui.create_cylinder_height).speed(0.1));
                    ui.end_row();
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        gui.actions.push(GuiAction::CreateCylinder {
                            radius: gui.create_cylinder_radius,
                            height: gui.create_cylinder_height,
                        });
                        gui.show_create_cylinder = false;
                    }
                    if ui.button("Cancel").clicked() {
                        gui.show_create_cylinder = false;
                    }
                });
            });
    }
    gui.show_create_cylinder = show_cyl;

    // --- Sphere ---
    let mut show_sph = gui.show_create_sphere;
    if show_sph {
        egui::Window::new("Create Sphere")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_sph)
            .show(ctx, |ui| {
                egui::Grid::new("sph_grid").show(ui, |ui| {
                    ui.label("Radius:");
                    ui.add(egui::DragValue::new(&mut gui.create_sphere_radius).speed(0.1));
                    ui.end_row();
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        gui.actions.push(GuiAction::CreateSphere {
                            radius: gui.create_sphere_radius,
                        });
                        gui.show_create_sphere = false;
                    }
                    if ui.button("Cancel").clicked() {
                        gui.show_create_sphere = false;
                    }
                });
            });
    }
    gui.show_create_sphere = show_sph;

    // --- Cone ---
    let mut show_cone = gui.show_create_cone;
    if show_cone {
        egui::Window::new("Create Cone")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_cone)
            .show(ctx, |ui| {
                egui::Grid::new("cone_grid").show(ui, |ui| {
                    ui.label("Base Radius:");
                    ui.add(egui::DragValue::new(&mut gui.create_cone_base_radius).speed(0.1));
                    ui.end_row();
                    ui.label("Top Radius:");
                    ui.add(
                        egui::DragValue::new(&mut gui.create_cone_top_radius)
                            .speed(0.1)
                            .range(0.0..=f64::MAX),
                    );
                    ui.end_row();
                    ui.label("Height:");
                    ui.add(egui::DragValue::new(&mut gui.create_cone_height).speed(0.1));
                    ui.end_row();
                });
                ui.weak("Top Radius = 0 for pointed cone");
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        gui.actions.push(GuiAction::CreateCone {
                            base_radius: gui.create_cone_base_radius,
                            top_radius: gui.create_cone_top_radius,
                            height: gui.create_cone_height,
                        });
                        gui.show_create_cone = false;
                    }
                    if ui.button("Cancel").clicked() {
                        gui.show_create_cone = false;
                    }
                });
            });
    }
    gui.show_create_cone = show_cone;

    // --- Torus ---
    let mut show_torus = gui.show_create_torus;
    if show_torus {
        egui::Window::new("Create Torus")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_torus)
            .show(ctx, |ui| {
                egui::Grid::new("torus_grid").show(ui, |ui| {
                    ui.label("Major Radius:");
                    ui.add(egui::DragValue::new(&mut gui.create_torus_major_radius).speed(0.1));
                    ui.end_row();
                    ui.label("Minor Radius:");
                    ui.add(egui::DragValue::new(&mut gui.create_torus_minor_radius).speed(0.1));
                    ui.end_row();
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        gui.actions.push(GuiAction::CreateTorus {
                            major_radius: gui.create_torus_major_radius,
                            minor_radius: gui.create_torus_minor_radius,
                        });
                        gui.show_create_torus = false;
                    }
                    if ui.button("Cancel").clicked() {
                        gui.show_create_torus = false;
                    }
                });
            });
    }
    gui.show_create_torus = show_torus;

    // --- Tube ---
    let mut show_tube = gui.show_create_tube;
    if show_tube {
        egui::Window::new("Create Tube")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_tube)
            .show(ctx, |ui| {
                egui::Grid::new("tube_grid").show(ui, |ui| {
                    ui.label("Outer Radius:");
                    ui.add(egui::DragValue::new(&mut gui.create_tube_outer_radius).speed(0.1));
                    ui.end_row();
                    ui.label("Inner Radius:");
                    ui.add(egui::DragValue::new(&mut gui.create_tube_inner_radius).speed(0.1));
                    ui.end_row();
                    ui.label("Height:");
                    ui.add(egui::DragValue::new(&mut gui.create_tube_height).speed(0.1));
                    ui.end_row();
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        gui.actions.push(GuiAction::CreateTube {
                            outer_radius: gui.create_tube_outer_radius,
                            inner_radius: gui.create_tube_inner_radius,
                            height: gui.create_tube_height,
                        });
                        gui.show_create_tube = false;
                    }
                    if ui.button("Cancel").clicked() {
                        gui.show_create_tube = false;
                    }
                });
            });
    }
    gui.show_create_tube = show_tube;

    // --- Prism ---
    let mut show_prism = gui.show_create_prism;
    if show_prism {
        egui::Window::new("Create Prism")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_prism)
            .show(ctx, |ui| {
                egui::Grid::new("prism_grid").show(ui, |ui| {
                    ui.label("Radius:");
                    ui.add(egui::DragValue::new(&mut gui.create_prism_radius).speed(0.1));
                    ui.end_row();
                    ui.label("Height:");
                    ui.add(egui::DragValue::new(&mut gui.create_prism_height).speed(0.1));
                    ui.end_row();
                    ui.label("Sides:");
                    ui.add(egui::DragValue::new(&mut gui.create_prism_sides).range(3..=64));
                    ui.end_row();
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        gui.actions.push(GuiAction::CreatePrism {
                            radius: gui.create_prism_radius,
                            height: gui.create_prism_height,
                            sides: gui.create_prism_sides,
                        });
                        gui.show_create_prism = false;
                    }
                    if ui.button("Cancel").clicked() {
                        gui.show_create_prism = false;
                    }
                });
            });
    }
    gui.show_create_prism = show_prism;

    // --- Wedge ---
    let mut show_wedge = gui.show_create_wedge;
    if show_wedge {
        egui::Window::new("Create Wedge")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_wedge)
            .show(ctx, |ui| {
                egui::Grid::new("wedge_grid").show(ui, |ui| {
                    ui.label("Width (DX):");
                    ui.add(egui::DragValue::new(&mut gui.create_wedge_dx).speed(0.1));
                    ui.end_row();
                    ui.label("Height (DY):");
                    ui.add(egui::DragValue::new(&mut gui.create_wedge_dy).speed(0.1));
                    ui.end_row();
                    ui.label("Depth (DZ):");
                    ui.add(egui::DragValue::new(&mut gui.create_wedge_dz).speed(0.1));
                    ui.end_row();
                    ui.label("Top Width:");
                    ui.add(
                        egui::DragValue::new(&mut gui.create_wedge_dx2)
                            .speed(0.1)
                            .range(0.0..=f64::MAX),
                    );
                    ui.end_row();
                    ui.label("Top Depth:");
                    ui.add(
                        egui::DragValue::new(&mut gui.create_wedge_dy2)
                            .speed(0.1)
                            .range(0.0..=f64::MAX),
                    );
                    ui.end_row();
                });
                ui.weak("Top = 0 for pyramid");
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        gui.actions.push(GuiAction::CreateWedge {
                            dx: gui.create_wedge_dx,
                            dy: gui.create_wedge_dy,
                            dz: gui.create_wedge_dz,
                            dx2: gui.create_wedge_dx2,
                            dy2: gui.create_wedge_dy2,
                        });
                        gui.show_create_wedge = false;
                    }
                    if ui.button("Cancel").clicked() {
                        gui.show_create_wedge = false;
                    }
                });
            });
    }
    gui.show_create_wedge = show_wedge;

    // --- Ellipsoid ---
    let mut show_ellipsoid = gui.show_create_ellipsoid;
    if show_ellipsoid {
        egui::Window::new("Create Ellipsoid")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_ellipsoid)
            .show(ctx, |ui| {
                egui::Grid::new("ellipsoid_grid").show(ui, |ui| {
                    ui.label("Radius X:");
                    ui.add(egui::DragValue::new(&mut gui.create_ellipsoid_rx).speed(0.1));
                    ui.end_row();
                    ui.label("Radius Y:");
                    ui.add(egui::DragValue::new(&mut gui.create_ellipsoid_ry).speed(0.1));
                    ui.end_row();
                    ui.label("Radius Z:");
                    ui.add(egui::DragValue::new(&mut gui.create_ellipsoid_rz).speed(0.1));
                    ui.end_row();
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        gui.actions.push(GuiAction::CreateEllipsoid {
                            rx: gui.create_ellipsoid_rx,
                            ry: gui.create_ellipsoid_ry,
                            rz: gui.create_ellipsoid_rz,
                        });
                        gui.show_create_ellipsoid = false;
                    }
                    if ui.button("Cancel").clicked() {
                        gui.show_create_ellipsoid = false;
                    }
                });
            });
    }
    gui.show_create_ellipsoid = show_ellipsoid;

    // --- Helix ---
    let mut show_helix = gui.show_create_helix;
    if show_helix {
        egui::Window::new("Create Helix")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_helix)
            .show(ctx, |ui| {
                egui::Grid::new("helix_grid").show(ui, |ui| {
                    ui.label("Radius:");
                    ui.add(egui::DragValue::new(&mut gui.create_helix_radius).speed(0.1));
                    ui.end_row();
                    ui.label("Pitch:");
                    ui.add(egui::DragValue::new(&mut gui.create_helix_pitch).speed(0.1));
                    ui.end_row();
                    ui.label("Turns:");
                    ui.add(egui::DragValue::new(&mut gui.create_helix_turns).speed(0.1));
                    ui.end_row();
                    ui.label("Tube Radius:");
                    ui.add(egui::DragValue::new(&mut gui.create_helix_tube_radius).speed(0.1));
                    ui.end_row();
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        gui.actions.push(GuiAction::CreateHelix {
                            radius: gui.create_helix_radius,
                            pitch: gui.create_helix_pitch,
                            turns: gui.create_helix_turns,
                            tube_radius: gui.create_helix_tube_radius,
                        });
                        gui.show_create_helix = false;
                    }
                    if ui.button("Cancel").clicked() {
                        gui.show_create_helix = false;
                    }
                });
            });
    }
    gui.show_create_helix = show_helix;

    // -----------------------------------------------------------------------
    // Boolean operation dialogs
    // -----------------------------------------------------------------------
    fn draw_boolean_dialog(
        ctx: &egui::Context,
        gui: &mut GuiState,
        title: &str,
        show: &mut bool,
        make_action: fn(f64, f64, f64, [f64; 3]) -> GuiAction,
    ) {
        let mut open = *show;
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("Second solid (Box):");
                egui::Grid::new(format!("{title}_grid")).show(ui, |ui| {
                    ui.label("Width:");
                    ui.add(egui::DragValue::new(&mut gui.bool_box_size[0]).speed(0.1));
                    ui.end_row();
                    ui.label("Height:");
                    ui.add(egui::DragValue::new(&mut gui.bool_box_size[1]).speed(0.1));
                    ui.end_row();
                    ui.label("Depth:");
                    ui.add(egui::DragValue::new(&mut gui.bool_box_size[2]).speed(0.1));
                    ui.end_row();
                    ui.label("Offset X:");
                    ui.add(egui::DragValue::new(&mut gui.bool_offset[0]).speed(0.1));
                    ui.end_row();
                    ui.label("Offset Y:");
                    ui.add(egui::DragValue::new(&mut gui.bool_offset[1]).speed(0.1));
                    ui.end_row();
                    ui.label("Offset Z:");
                    ui.add(egui::DragValue::new(&mut gui.bool_offset[2]).speed(0.1));
                    ui.end_row();
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Apply").clicked() {
                        let [w, h, d] = gui.bool_box_size;
                        let offset = gui.bool_offset;
                        gui.actions.push(make_action(w, h, d, offset));
                        *show = false;
                    }
                    if ui.button("Cancel").clicked() {
                        *show = false;
                    }
                });
            });
        if !open {
            *show = false;
        }
    }

    fn make_bool_union(w: f64, h: f64, d: f64, offset: [f64; 3]) -> GuiAction {
        GuiAction::BooleanUnionWith { width: w, height: h, depth: d, offset }
    }
    fn make_bool_subtract(w: f64, h: f64, d: f64, offset: [f64; 3]) -> GuiAction {
        GuiAction::BooleanSubtractWith { width: w, height: h, depth: d, offset }
    }
    fn make_bool_intersect(w: f64, h: f64, d: f64, offset: [f64; 3]) -> GuiAction {
        GuiAction::BooleanIntersectWith { width: w, height: h, depth: d, offset }
    }

    {
        let mut show = gui.show_boolean_union;
        if show {
            draw_boolean_dialog(ctx, gui, "Boolean Union", &mut show, make_bool_union);
        }
        gui.show_boolean_union = show;
    }
    {
        let mut show = gui.show_boolean_subtract;
        if show {
            draw_boolean_dialog(ctx, gui, "Boolean Subtract", &mut show, make_bool_subtract);
        }
        gui.show_boolean_subtract = show;
    }
    {
        let mut show = gui.show_boolean_intersect;
        if show {
            draw_boolean_dialog(ctx, gui, "Boolean Intersect", &mut show, make_bool_intersect);
        }
        gui.show_boolean_intersect = show;
    }

    // Mirror dialog
    let mut show_mirror = gui.show_mirror;
    if show_mirror {
        egui::Window::new("Mirror Solid")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_mirror)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Plane:");
                    ui.selectable_value(&mut gui.mirror_plane, MirrorPlane::YZ, "YZ");
                    ui.selectable_value(&mut gui.mirror_plane, MirrorPlane::XZ, "XZ");
                    ui.selectable_value(&mut gui.mirror_plane, MirrorPlane::XY, "XY");
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Apply").clicked() {
                        gui.actions.push(GuiAction::MirrorSolid(gui.mirror_plane));
                        gui.show_mirror = false;
                    }
                    if ui.button("Cancel").clicked() {
                        gui.show_mirror = false;
                    }
                });
            });
    }
    gui.show_mirror = show_mirror;

    // Scale dialog
    let mut show_scale = gui.show_scale;
    if show_scale {
        egui::Window::new("Scale Solid")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_scale)
            .show(ctx, |ui| {
                egui::Grid::new("scale_grid").show(ui, |ui| {
                    ui.label("Factor:");
                    ui.add(egui::DragValue::new(&mut gui.scale_factor).speed(0.01).range(0.01..=100.0));
                    ui.end_row();
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Apply").clicked() {
                        gui.actions.push(GuiAction::ScaleSolid {
                            factor: gui.scale_factor,
                        });
                        gui.show_scale = false;
                    }
                    if ui.button("Cancel").clicked() {
                        gui.show_scale = false;
                    }
                });
            });
    }
    gui.show_scale = show_scale;

    // Shell dialog
    let mut show_shell = gui.show_shell;
    if show_shell {
        egui::Window::new("Shell Solid")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_shell)
            .show(ctx, |ui| {
                egui::Grid::new("shell_grid").show(ui, |ui| {
                    ui.label("Thickness:");
                    ui.add(egui::DragValue::new(&mut gui.shell_thickness).speed(0.1).range(0.01..=50.0));
                    ui.end_row();
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Apply").clicked() {
                        gui.actions.push(GuiAction::ShellSolid {
                            thickness: gui.shell_thickness,
                        });
                        gui.show_shell = false;
                    }
                    if ui.button("Cancel").clicked() {
                        gui.show_shell = false;
                    }
                });
            });
    }
    gui.show_shell = show_shell;

    // Fillet dialog
    let mut show_fillet = gui.show_fillet;
    if show_fillet {
        egui::Window::new("Fillet All Edges")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_fillet)
            .show(ctx, |ui| {
                egui::Grid::new("fillet_grid").show(ui, |ui| {
                    ui.label("Radius:");
                    ui.add(egui::DragValue::new(&mut gui.fillet_radius).speed(0.1).range(0.01..=50.0));
                    ui.end_row();
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Apply").clicked() {
                        gui.actions.push(GuiAction::FilletAllEdges {
                            radius: gui.fillet_radius,
                        });
                        gui.show_fillet = false;
                    }
                    if ui.button("Cancel").clicked() {
                        gui.show_fillet = false;
                    }
                });
            });
    }
    gui.show_fillet = show_fillet;

    // Chamfer dialog
    let mut show_chamfer = gui.show_chamfer;
    if show_chamfer {
        egui::Window::new("Chamfer All Edges")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_chamfer)
            .show(ctx, |ui| {
                egui::Grid::new("chamfer_grid").show(ui, |ui| {
                    ui.label("Distance:");
                    ui.add(egui::DragValue::new(&mut gui.chamfer_distance).speed(0.1).range(0.01..=50.0));
                    ui.end_row();
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Apply").clicked() {
                        gui.actions.push(GuiAction::ChamferAllEdges {
                            distance: gui.chamfer_distance,
                        });
                        gui.show_chamfer = false;
                    }
                    if ui.button("Cancel").clicked() {
                        gui.show_chamfer = false;
                    }
                });
            });
    }
    gui.show_chamfer = show_chamfer;

    // Linear Pattern dialog
    let mut show_pattern = gui.show_pattern;
    if show_pattern {
        egui::Window::new("Linear Pattern")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_pattern)
            .show(ctx, |ui| {
                egui::Grid::new("pattern_grid").show(ui, |ui| {
                    ui.label("Count:");
                    let mut count_i32 = gui.pattern_count as i32;
                    ui.add(egui::DragValue::new(&mut count_i32).range(2..=20));
                    gui.pattern_count = count_i32.max(2) as usize;
                    ui.end_row();
                    ui.label("Spacing:");
                    ui.add(egui::DragValue::new(&mut gui.pattern_spacing).speed(0.1).range(0.1..=200.0));
                    ui.end_row();
                    ui.label("Axis:");
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut gui.pattern_axis, 0, "X");
                        ui.selectable_value(&mut gui.pattern_axis, 1, "Y");
                        ui.selectable_value(&mut gui.pattern_axis, 2, "Z");
                    });
                    ui.end_row();
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Apply").clicked() {
                        gui.actions.push(GuiAction::LinearPattern {
                            count: gui.pattern_count,
                            spacing: gui.pattern_spacing,
                            axis: gui.pattern_axis,
                        });
                        gui.show_pattern = false;
                    }
                    if ui.button("Cancel").clicked() {
                        gui.show_pattern = false;
                    }
                });
            });
    }
    gui.show_pattern = show_pattern;

    // Mesh Smooth dialog
    let mut show_smooth = gui.show_mesh_smooth;
    if show_smooth {
        egui::Window::new("Smooth Mesh")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_smooth)
            .show(ctx, |ui| {
                egui::Grid::new("smooth_grid").show(ui, |ui| {
                    ui.label("Iterations:");
                    let mut iters_i32 = gui.mesh_smooth_iters as i32;
                    ui.add(egui::DragValue::new(&mut iters_i32).range(1..=20));
                    gui.mesh_smooth_iters = iters_i32.max(1) as usize;
                    ui.end_row();
                    ui.label("Factor:");
                    ui.add(egui::DragValue::new(&mut gui.mesh_smooth_factor).speed(0.01).range(0.01..=1.0));
                    ui.end_row();
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Apply").clicked() {
                        gui.actions.push(GuiAction::MeshSmooth {
                            iterations: gui.mesh_smooth_iters,
                            factor: gui.mesh_smooth_factor,
                        });
                        gui.show_mesh_smooth = false;
                    }
                    if ui.button("Cancel").clicked() {
                        gui.show_mesh_smooth = false;
                    }
                });
            });
    }
    gui.show_mesh_smooth = show_smooth;

    // Remesh dialog
    let mut show_remesh = gui.show_mesh_remesh;
    if show_remesh {
        egui::Window::new("Remesh")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_remesh)
            .show(ctx, |ui| {
                egui::Grid::new("remesh_grid").show(ui, |ui| {
                    ui.label("Target edge length:");
                    ui.add(egui::DragValue::new(&mut gui.mesh_remesh_edge_len).speed(0.1).range(0.1..=100.0));
                    ui.end_row();
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Apply").clicked() {
                        gui.actions.push(GuiAction::MeshRemesh {
                            target_edge_len: gui.mesh_remesh_edge_len,
                        });
                        gui.show_mesh_remesh = false;
                    }
                    if ui.button("Cancel").clicked() {
                        gui.show_mesh_remesh = false;
                    }
                });
            });
    }
    gui.show_mesh_remesh = show_remesh;
}

pub(crate) fn draw_about_dialog(ctx: &egui::Context, gui: &mut GuiState) {
    let mut show = gui.show_about;
    if show {
        egui::Window::new("About CADKernel")
            .collapsible(false)
            .resizable(false)
            .open(&mut show)
            .show(ctx, |ui| {
                ui.heading("\u{2B22} CADKernel");
                ui.label("Open-source CAD software built with Rust");
                ui.separator();
                egui::Grid::new("about_grid").num_columns(2).show(ui, |ui| {
                    ui.strong("Version:");
                    ui.label("0.1.0 (pre-alpha)");
                    ui.end_row();
                    ui.strong("License:");
                    ui.label("Apache-2.0");
                    ui.end_row();
                    ui.strong("Author:");
                    ui.label("Kim DaeHyun");
                    ui.end_row();
                    ui.strong("Renderer:");
                    ui.label("wgpu 24 + egui 0.31");
                    ui.end_row();
                    ui.strong("Crates:");
                    ui.label("core, math, geometry, topology, modeling, sketch, io, viewer, python");
                    ui.end_row();
                    ui.strong("Features:");
                    ui.label("9 workbenches, 15 I/O formats, NURBS kernel");
                    ui.end_row();
                });
                ui.separator();
                ui.hyperlink_to("\u{1F517} GitHub", "https://github.com/kernalix7/CADKernel");
            });
    }
    gui.show_about = show;
}

pub(crate) fn draw_settings(ctx: &egui::Context, gui: &mut GuiState, nav: &mut NavConfig) {
    let mut show = gui.show_settings;
    if !show {
        return;
    }
    egui::Window::new("Settings")
        .collapsible(false)
        .resizable(true)
        .default_width(440.0)
        .open(&mut show)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // 3D View
                ui.heading("3D View");
                ui.separator();

                ui.label("General");
                ui.indent("general_indent", |ui| {
                    ui.checkbox(
                        &mut nav.show_axes_indicator,
                        "Show coordinate system in corner",
                    );
                    ui.checkbox(&mut nav.show_fps, "Show FPS counter");
                });

                ui.add_space(8.0);
                ui.label("Camera Type");
                ui.indent("cam_type_indent", |ui| {
                    ui.horizontal(|ui| {
                        ui.radio_value(
                            &mut nav.default_projection,
                            Projection::Perspective,
                            "Perspective",
                        );
                        ui.radio_value(
                            &mut nav.default_projection,
                            Projection::Orthographic,
                            "Orthographic",
                        );
                    });
                });

                ui.add_space(12.0);

                // Navigation
                ui.heading("Navigation");
                ui.separator();

                ui.label("View Cube");
                ui.indent("cube_indent", |ui| {
                    ui.checkbox(&mut nav.show_view_cube, "Show View Cube");
                    if nav.show_view_cube {
                        egui::Grid::new("cube_settings")
                            .num_columns(2)
                            .spacing([8.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Orbit steps:");
                                ui.add(
                                    egui::DragValue::new(&mut nav.orbit_steps)
                                        .range(2..=24)
                                        .suffix(" steps"),
                                );
                                ui.end_row();

                                ui.label("Cube size:");
                                ui.add(
                                    egui::DragValue::new(&mut nav.cube_size)
                                        .range(60..=200)
                                        .suffix(" px"),
                                );
                                ui.end_row();

                                ui.label("Inactive opacity:");
                                ui.add(
                                    egui::Slider::new(&mut nav.cube_opacity, 0.1..=1.0)
                                        .show_value(true),
                                );
                                ui.end_row();
                            });
                        ui.checkbox(&mut nav.snap_to_nearest, "Snap to nearest view");
                        egui::Grid::new("cube_corner_grid").num_columns(2).show(ui, |ui| {
                            ui.label("Corner:");
                            egui::ComboBox::from_id_salt("cube_corner")
                                .selected_text(match nav.cube_corner {
                                    1 => "Top-Left",
                                    2 => "Bottom-Left",
                                    3 => "Bottom-Right",
                                    _ => "Top-Right",
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut nav.cube_corner, 0, "Top-Right");
                                    ui.selectable_value(&mut nav.cube_corner, 1, "Top-Left");
                                    ui.selectable_value(&mut nav.cube_corner, 2, "Bottom-Left");
                                    ui.selectable_value(&mut nav.cube_corner, 3, "Bottom-Right");
                                });
                            ui.end_row();
                        });
                    }
                });

                ui.add_space(8.0);
                ui.label("Orbit Style");
                ui.indent("orbit_style_indent", |ui| {
                    egui::ComboBox::from_label("Mouse style")
                        .selected_text(nav.style.label())
                        .show_ui(ui, |ui| {
                            for &style in NavStyle::ALL {
                                ui.selectable_value(&mut nav.style, style, style.label());
                            }
                        });
                    ui.add_space(2.0);
                    ui.weak(nav.style.description());
                });

                ui.add_space(8.0);
                ui.label("Sensitivity");
                ui.indent("sens_indent", |ui| {
                    egui::Grid::new("sens_grid")
                        .num_columns(2)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Orbit:");
                            ui.add(
                                egui::Slider::new(&mut nav.orbit_sensitivity, 0.001..=0.02)
                                    .logarithmic(true),
                            );
                            ui.end_row();
                            ui.label("Pan:");
                            ui.add(
                                egui::Slider::new(&mut nav.pan_sensitivity, 0.0005..=0.01)
                                    .logarithmic(true),
                            );
                            ui.end_row();
                            ui.label("Zoom:");
                            ui.add(
                                egui::Slider::new(&mut nav.zoom_sensitivity, 0.02..=0.5)
                                    .logarithmic(true),
                            );
                            ui.end_row();
                        });
                    ui.checkbox(&mut nav.invert_zoom, "Invert zoom direction");
                });

                ui.add_space(8.0);
                ui.label("Animation");
                ui.indent("anim_indent", |ui| {
                    ui.checkbox(&mut nav.enable_view_animation, "Animate view transitions");
                    if nav.enable_view_animation {
                        egui::Grid::new("anim_grid")
                            .num_columns(2)
                            .spacing([8.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Duration:");
                                ui.add(
                                    egui::Slider::new(&mut nav.view_animation_duration, 0.1..=1.0)
                                        .suffix(" s")
                                        .step_by(0.05),
                                );
                                ui.end_row();
                            });
                    }
                });

                ui.add_space(12.0);

                // Lighting
                ui.heading("Lighting");
                ui.separator();

                ui.indent("light_indent", |ui| {
                    ui.checkbox(&mut nav.enable_lighting, "Enable lighting");
                    if nav.enable_lighting {
                        egui::Grid::new("light_grid")
                            .num_columns(2)
                            .spacing([8.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Intensity:");
                                ui.add(egui::Slider::new(&mut nav.light_intensity, 0.0..=2.0));
                                ui.end_row();
                                ui.label("Direction X:");
                                ui.add(
                                    egui::DragValue::new(&mut nav.light_dir[0])
                                        .speed(0.01)
                                        .range(-1.0..=1.0),
                                );
                                ui.end_row();
                                ui.label("Direction Y:");
                                ui.add(
                                    egui::DragValue::new(&mut nav.light_dir[1])
                                        .speed(0.01)
                                        .range(-1.0..=1.0),
                                );
                                ui.end_row();
                                ui.label("Direction Z:");
                                ui.add(
                                    egui::DragValue::new(&mut nav.light_dir[2])
                                        .speed(0.01)
                                        .range(-1.0..=1.0),
                                );
                                ui.end_row();
                            });
                    }
                });

                ui.add_space(16.0);
                if ui.button("Reset to defaults").clicked() {
                    *nav = NavConfig::new();
                    gui.status_message = "Settings reset to defaults".into();
                }
            });
        });
    gui.show_settings = show;
}
