use super::theme;
use super::{
    AssemblyJointType, BcKind, GuiAction, GuiState, MaterialPreset, MirrorPlane, SelectedEntity,
    TechDrawAnnotationKind, TechDrawCenterlineKind, TechDrawDimensionKind, TechDrawTemplatePreset,
    TechDrawViewKind,
};
use crate::nav::{BgPreset, NavConfig, NavStyle, OrbitStyle, RotationMode, UnitSystem};
use crate::render::Projection;

// ---------------------------------------------------------------------------
// Shared dialog helpers
// ---------------------------------------------------------------------------

/// Section header for dialog groups — FreeCAD-style accent bar.
fn dialog_section(ui: &mut egui::Ui, label: &str) {
    ui.add_space(6.0);
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 20.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter()
            .rect_filled(rect, 2.0, super::theme::COLOR_ACCENT.gamma_multiply(0.16));
        // Left accent bar
        ui.painter().rect_filled(
            egui::Rect::from_min_size(rect.left_top(), egui::vec2(3.0, rect.height())),
            1.0,
            theme::COLOR_ACCENT,
        );
        ui.painter().text(
            egui::pos2(rect.left() + 8.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(11.5),
            theme::COLOR_ACCENT,
        );
    }
    ui.add_space(4.0);
}

/// Render a parameter row: Label | DragValue with " mm" suffix + optional help text.
fn param_field(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    speed: f64,
    help: &str,
) -> bool {
    ui.label(
        egui::RichText::new(format!("{label}:"))
            .size(11.0)
            .color(theme::COLOR_DIM),
    );
    let changed = ui
        .add(
            egui::DragValue::new(value)
                .range(range)
                .speed(speed)
                .suffix(" mm"),
        )
        .changed();
    ui.end_row();
    if !help.is_empty() {
        ui.label("");
        ui.label(
            egui::RichText::new(help)
                .size(10.0)
                .color(theme::COLOR_DIM)
                .italics(),
        );
        ui.end_row();
    }
    changed
}

/// Render a parameter row without unit suffix.
fn param_field_no_unit(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    speed: f64,
) -> bool {
    ui.label(
        egui::RichText::new(format!("{label}:"))
            .size(11.0)
            .color(theme::COLOR_DIM),
    );
    let changed = ui
        .add(egui::DragValue::new(value).range(range).speed(speed))
        .changed();
    ui.end_row();
    changed
}

/// Validation message (red text shown below fields).
fn validation_error(ui: &mut egui::Ui, msg: &str) {
    ui.label(
        egui::RichText::new(msg)
            .size(10.0)
            .color(egui::Color32::from_rgb(230, 80, 70)),
    );
}

/// OK / Cancel / Reset button bar — FreeCAD-style accent primary button.
fn button_bar(ui: &mut egui::Ui, ok_label: &str) -> (bool, bool, bool) {
    let mut ok = false;
    let mut cancel = false;
    let mut reset = false;
    ui.add_space(4.0);
    // Thin separator line
    let (sep_rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(sep_rect, 0.0, egui::Color32::from_gray(55));
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        // Primary button with accent color
        let btn = ui.add(
            egui::Button::new(
                egui::RichText::new(ok_label)
                    .strong()
                    .color(egui::Color32::WHITE),
            )
            .fill(egui::Color32::from_rgb(0, 100, 180))
            .min_size(egui::vec2(70.0, 24.0)),
        );
        ok = btn.clicked();
        cancel = ui
            .add(egui::Button::new("Cancel").min_size(egui::vec2(60.0, 24.0)))
            .clicked();
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            reset = ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("Defaults")
                            .size(11.0)
                            .color(theme::COLOR_DIM),
                    )
                    .frame(false),
                )
                .clicked();
        });
    });
    (ok, cancel, reset)
}

fn draw_use_as_sketch_reference_dialog(ctx: &egui::Context, gui: &mut GuiState) {
    if gui.sketch_mode.is_none() {
        return;
    }
    let face_count = gui
        .selected_entities
        .iter()
        .filter(|entity| matches!(entity, SelectedEntity::Face(_)))
        .count();
    let edge_count = gui
        .selected_entities
        .iter()
        .filter(|entity| matches!(entity, SelectedEntity::Edge(_)))
        .count();
    if face_count + edge_count == 0 {
        return;
    }

    let mut confirm = false;
    let mut cancel = false;
    egui::Window::new("Use as Sketch Reference")
        .collapsible(false)
        .resizable(false)
        .fixed_size([260.0, 0.0])
        .show(ctx, |ui| {
            dialog_section(ui, "External Reference");
            ui.label(
                egui::RichText::new(format!("Selected: {face_count} face, {edge_count} edge"))
                    .size(11.0)
                    .color(egui::Color32::WHITE),
            );
            ui.label(
                egui::RichText::new("Projected geometry is read-only construction geometry.")
                    .size(10.0)
                    .color(theme::COLOR_DIM),
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                confirm = ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Use Reference")
                                .strong()
                                .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(0, 100, 180))
                        .min_size(egui::vec2(105.0, 24.0)),
                    )
                    .clicked();
                cancel = ui
                    .add(egui::Button::new("Cancel").min_size(egui::vec2(70.0, 24.0)))
                    .clicked();
            });
        });

    if confirm {
        gui.actions.push(GuiAction::Sketcher(
            super::SketcherAction::ExternalProjection,
        ));
        gui.selected_entities.clear();
        gui.status_message = "Sketch reference projection queued".into();
    } else if cancel {
        gui.selected_entities.clear();
        gui.status_message = "Sketch reference selection cleared".into();
    }
}

// ---------------------------------------------------------------------------
// Create primitive dialogs
// ---------------------------------------------------------------------------

pub(crate) fn draw_create_dialogs(ctx: &egui::Context, gui: &mut GuiState) {
    draw_use_as_sketch_reference_dialog(ctx, gui);

    // --- Box ---
    let mut show_box = gui.show_create_box;
    if show_box {
        egui::Window::new("Create Box")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_box)
            .show(ctx, |ui| {
                dialog_section(ui, "Dimensions");
                let mut valid = true;
                egui::Grid::new("box_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        param_field(
                            ui,
                            "Width",
                            &mut gui.create_box_size[0],
                            0.1..=1000.0,
                            0.1,
                            "",
                        );
                        param_field(
                            ui,
                            "Height",
                            &mut gui.create_box_size[1],
                            0.1..=1000.0,
                            0.1,
                            "",
                        );
                        param_field(
                            ui,
                            "Depth",
                            &mut gui.create_box_size[2],
                            0.1..=1000.0,
                            0.1,
                            "",
                        );
                    });
                for v in &gui.create_box_size {
                    if *v <= 0.0 {
                        valid = false;
                        validation_error(ui, "All dimensions must be > 0");
                        break;
                    }
                }
                let (ok, cancel, reset) = button_bar(ui, "Create");
                if ok && valid {
                    let [w, h, d] = gui.create_box_size;
                    gui.actions.push(GuiAction::CreateBox {
                        width: w,
                        height: h,
                        depth: d,
                    });
                    gui.show_create_box = false;
                }
                if cancel {
                    gui.show_create_box = false;
                }
                if reset {
                    gui.create_box_size = [10.0, 10.0, 10.0];
                }
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
                dialog_section(ui, "Parameters");
                egui::Grid::new("cyl_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        param_field(
                            ui,
                            "Radius",
                            &mut gui.create_cylinder_radius,
                            0.1..=500.0,
                            0.1,
                            "",
                        );
                        param_field(
                            ui,
                            "Height",
                            &mut gui.create_cylinder_height,
                            0.1..=1000.0,
                            0.1,
                            "",
                        );
                    });
                let valid = gui.create_cylinder_radius > 0.0 && gui.create_cylinder_height > 0.0;
                if !valid {
                    validation_error(ui, "Radius and Height must be > 0");
                }
                let (ok, cancel, reset) = button_bar(ui, "Create");
                if ok && valid {
                    gui.actions.push(GuiAction::CreateCylinder {
                        radius: gui.create_cylinder_radius,
                        height: gui.create_cylinder_height,
                    });
                    gui.show_create_cylinder = false;
                }
                if cancel {
                    gui.show_create_cylinder = false;
                }
                if reset {
                    gui.create_cylinder_radius = 5.0;
                    gui.create_cylinder_height = 10.0;
                }
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
                dialog_section(ui, "Parameters");
                egui::Grid::new("sph_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        param_field(
                            ui,
                            "Radius",
                            &mut gui.create_sphere_radius,
                            0.1..=500.0,
                            0.1,
                            "",
                        );
                    });
                let valid = gui.create_sphere_radius > 0.0;
                if !valid {
                    validation_error(ui, "Radius must be > 0");
                }
                let (ok, cancel, reset) = button_bar(ui, "Create");
                if ok && valid {
                    gui.actions.push(GuiAction::CreateSphere {
                        radius: gui.create_sphere_radius,
                    });
                    gui.show_create_sphere = false;
                }
                if cancel {
                    gui.show_create_sphere = false;
                }
                if reset {
                    gui.create_sphere_radius = 5.0;
                }
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
                dialog_section(ui, "Parameters");
                egui::Grid::new("cone_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        param_field(
                            ui,
                            "Base Radius",
                            &mut gui.create_cone_base_radius,
                            0.01..=500.0,
                            0.1,
                            "",
                        );
                        param_field(
                            ui,
                            "Top Radius",
                            &mut gui.create_cone_top_radius,
                            0.0..=500.0,
                            0.1,
                            "Set to 0 for pointed cone",
                        );
                        param_field(
                            ui,
                            "Height",
                            &mut gui.create_cone_height,
                            0.1..=1000.0,
                            0.1,
                            "",
                        );
                    });
                let valid = gui.create_cone_base_radius > 0.0 && gui.create_cone_height > 0.0;
                if !valid {
                    validation_error(ui, "Base Radius and Height must be > 0");
                }
                let (ok, cancel, reset) = button_bar(ui, "Create");
                if ok && valid {
                    gui.actions.push(GuiAction::CreateCone {
                        base_radius: gui.create_cone_base_radius,
                        top_radius: gui.create_cone_top_radius,
                        height: gui.create_cone_height,
                    });
                    gui.show_create_cone = false;
                }
                if cancel {
                    gui.show_create_cone = false;
                }
                if reset {
                    gui.create_cone_base_radius = 5.0;
                    gui.create_cone_top_radius = 0.0;
                    gui.create_cone_height = 10.0;
                }
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
                dialog_section(ui, "Parameters");
                egui::Grid::new("torus_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        param_field(
                            ui,
                            "Major Radius",
                            &mut gui.create_torus_major_radius,
                            0.1..=500.0,
                            0.1,
                            "Distance from center to tube center",
                        );
                        param_field(
                            ui,
                            "Minor Radius",
                            &mut gui.create_torus_minor_radius,
                            0.01..=200.0,
                            0.05,
                            "Tube cross-section radius",
                        );
                    });
                let valid = gui.create_torus_major_radius > gui.create_torus_minor_radius;
                if !valid {
                    validation_error(ui, "Major Radius must be > Minor Radius");
                }
                let (ok, cancel, reset) = button_bar(ui, "Create");
                if ok && valid {
                    gui.actions.push(GuiAction::CreateTorus {
                        major_radius: gui.create_torus_major_radius,
                        minor_radius: gui.create_torus_minor_radius,
                    });
                    gui.show_create_torus = false;
                }
                if cancel {
                    gui.show_create_torus = false;
                }
                if reset {
                    gui.create_torus_major_radius = 5.0;
                    gui.create_torus_minor_radius = 1.5;
                }
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
                dialog_section(ui, "Parameters");
                egui::Grid::new("tube_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        param_field(
                            ui,
                            "Outer Radius",
                            &mut gui.create_tube_outer_radius,
                            0.1..=500.0,
                            0.1,
                            "",
                        );
                        param_field(
                            ui,
                            "Inner Radius",
                            &mut gui.create_tube_inner_radius,
                            0.01..=499.0,
                            0.1,
                            "Must be less than Outer Radius",
                        );
                        param_field(
                            ui,
                            "Height",
                            &mut gui.create_tube_height,
                            0.1..=1000.0,
                            0.1,
                            "",
                        );
                    });
                let valid = gui.create_tube_outer_radius > gui.create_tube_inner_radius;
                if !valid {
                    validation_error(ui, "Outer Radius must be > Inner Radius");
                }
                let (ok, cancel, reset) = button_bar(ui, "Create");
                if ok && valid {
                    gui.actions.push(GuiAction::CreateTube {
                        outer_radius: gui.create_tube_outer_radius,
                        inner_radius: gui.create_tube_inner_radius,
                        height: gui.create_tube_height,
                    });
                    gui.show_create_tube = false;
                }
                if cancel {
                    gui.show_create_tube = false;
                }
                if reset {
                    gui.create_tube_outer_radius = 5.0;
                    gui.create_tube_inner_radius = 3.0;
                    gui.create_tube_height = 10.0;
                }
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
                dialog_section(ui, "Parameters");
                egui::Grid::new("prism_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        param_field(
                            ui,
                            "Radius",
                            &mut gui.create_prism_radius,
                            0.1..=500.0,
                            0.1,
                            "Circumscribed circle radius",
                        );
                        param_field(
                            ui,
                            "Height",
                            &mut gui.create_prism_height,
                            0.1..=1000.0,
                            0.1,
                            "",
                        );
                        ui.label(
                            egui::RichText::new("Sides:")
                                .size(11.0)
                                .color(theme::COLOR_DIM),
                        );
                        ui.add(egui::DragValue::new(&mut gui.create_prism_sides).range(3..=64));
                        ui.end_row();
                    });
                let (ok, cancel, reset) = button_bar(ui, "Create");
                if ok {
                    gui.actions.push(GuiAction::CreatePrism {
                        radius: gui.create_prism_radius,
                        height: gui.create_prism_height,
                        sides: gui.create_prism_sides,
                    });
                    gui.show_create_prism = false;
                }
                if cancel {
                    gui.show_create_prism = false;
                }
                if reset {
                    gui.create_prism_radius = 5.0;
                    gui.create_prism_height = 10.0;
                    gui.create_prism_sides = 6;
                }
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
                dialog_section(ui, "Base Dimensions");
                egui::Grid::new("wedge_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        param_field(
                            ui,
                            "Width (DX)",
                            &mut gui.create_wedge_dx,
                            0.1..=1000.0,
                            0.1,
                            "",
                        );
                        param_field(
                            ui,
                            "Height (DY)",
                            &mut gui.create_wedge_dy,
                            0.1..=1000.0,
                            0.1,
                            "",
                        );
                        param_field(
                            ui,
                            "Depth (DZ)",
                            &mut gui.create_wedge_dz,
                            0.1..=1000.0,
                            0.1,
                            "",
                        );
                    });
                dialog_section(ui, "Top Dimensions");
                egui::Grid::new("wedge_top_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        param_field(
                            ui,
                            "Top Width",
                            &mut gui.create_wedge_dx2,
                            0.0..=1000.0,
                            0.1,
                            "Set to 0 for pyramid",
                        );
                        param_field(
                            ui,
                            "Top Depth",
                            &mut gui.create_wedge_dy2,
                            0.0..=1000.0,
                            0.1,
                            "",
                        );
                    });
                let (ok, cancel, reset) = button_bar(ui, "Create");
                if ok {
                    gui.actions.push(GuiAction::CreateWedge {
                        dx: gui.create_wedge_dx,
                        dy: gui.create_wedge_dy,
                        dz: gui.create_wedge_dz,
                        dx2: gui.create_wedge_dx2,
                        dy2: gui.create_wedge_dy2,
                    });
                    gui.show_create_wedge = false;
                }
                if cancel {
                    gui.show_create_wedge = false;
                }
                if reset {
                    gui.create_wedge_dx = 10.0;
                    gui.create_wedge_dy = 10.0;
                    gui.create_wedge_dz = 10.0;
                    gui.create_wedge_dx2 = 5.0;
                    gui.create_wedge_dy2 = 5.0;
                }
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
                dialog_section(ui, "Semi-Axes");
                egui::Grid::new("ellipsoid_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        param_field(
                            ui,
                            "Radius X",
                            &mut gui.create_ellipsoid_rx,
                            0.1..=500.0,
                            0.1,
                            "",
                        );
                        param_field(
                            ui,
                            "Radius Y",
                            &mut gui.create_ellipsoid_ry,
                            0.1..=500.0,
                            0.1,
                            "",
                        );
                        param_field(
                            ui,
                            "Radius Z",
                            &mut gui.create_ellipsoid_rz,
                            0.1..=500.0,
                            0.1,
                            "",
                        );
                    });
                let (ok, cancel, reset) = button_bar(ui, "Create");
                if ok {
                    gui.actions.push(GuiAction::CreateEllipsoid {
                        rx: gui.create_ellipsoid_rx,
                        ry: gui.create_ellipsoid_ry,
                        rz: gui.create_ellipsoid_rz,
                    });
                    gui.show_create_ellipsoid = false;
                }
                if cancel {
                    gui.show_create_ellipsoid = false;
                }
                if reset {
                    gui.create_ellipsoid_rx = 5.0;
                    gui.create_ellipsoid_ry = 3.0;
                    gui.create_ellipsoid_rz = 2.0;
                }
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
                dialog_section(ui, "Parameters");
                egui::Grid::new("helix_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        param_field(
                            ui,
                            "Radius",
                            &mut gui.create_helix_radius,
                            0.1..=500.0,
                            0.1,
                            "Helix center radius",
                        );
                        param_field(
                            ui,
                            "Pitch",
                            &mut gui.create_helix_pitch,
                            0.1..=100.0,
                            0.1,
                            "Distance per turn",
                        );
                        param_field_no_unit(
                            ui,
                            "Turns",
                            &mut gui.create_helix_turns,
                            0.5..=100.0,
                            0.1,
                        );
                        param_field(
                            ui,
                            "Tube Radius",
                            &mut gui.create_helix_tube_radius,
                            0.01..=100.0,
                            0.05,
                            "Wire cross-section radius",
                        );
                    });
                let (ok, cancel, reset) = button_bar(ui, "Create");
                if ok {
                    gui.actions.push(GuiAction::CreateHelix {
                        radius: gui.create_helix_radius,
                        pitch: gui.create_helix_pitch,
                        turns: gui.create_helix_turns,
                        tube_radius: gui.create_helix_tube_radius,
                    });
                    gui.show_create_helix = false;
                }
                if cancel {
                    gui.show_create_helix = false;
                }
                if reset {
                    gui.create_helix_radius = 5.0;
                    gui.create_helix_pitch = 3.0;
                    gui.create_helix_turns = 3.0;
                    gui.create_helix_tube_radius = 0.5;
                }
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
                dialog_section(ui, "Tool Solid (Box)");
                egui::Grid::new(format!("{title}_grid"))
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        param_field(ui, "Width", &mut gui.bool_box_size[0], 0.1..=500.0, 0.1, "");
                        param_field(
                            ui,
                            "Height",
                            &mut gui.bool_box_size[1],
                            0.1..=500.0,
                            0.1,
                            "",
                        );
                        param_field(ui, "Depth", &mut gui.bool_box_size[2], 0.1..=500.0, 0.1, "");
                    });
                dialog_section(ui, "Offset");
                egui::Grid::new(format!("{title}_off_grid"))
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        param_field(
                            ui,
                            "Offset X",
                            &mut gui.bool_offset[0],
                            -500.0..=500.0,
                            0.1,
                            "",
                        );
                        param_field(
                            ui,
                            "Offset Y",
                            &mut gui.bool_offset[1],
                            -500.0..=500.0,
                            0.1,
                            "",
                        );
                        param_field(
                            ui,
                            "Offset Z",
                            &mut gui.bool_offset[2],
                            -500.0..=500.0,
                            0.1,
                            "",
                        );
                    });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .add(egui::Button::new(egui::RichText::new("Apply").strong()))
                        .clicked()
                    {
                        let [w, h, d] = gui.bool_box_size;
                        gui.actions.push(make_action(w, h, d, gui.bool_offset));
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
        GuiAction::BooleanUnionWith {
            width: w,
            height: h,
            depth: d,
            offset,
        }
    }
    fn make_bool_subtract(w: f64, h: f64, d: f64, offset: [f64; 3]) -> GuiAction {
        GuiAction::BooleanSubtractWith {
            width: w,
            height: h,
            depth: d,
            offset,
        }
    }
    fn make_bool_intersect(w: f64, h: f64, d: f64, offset: [f64; 3]) -> GuiAction {
        GuiAction::BooleanIntersectWith {
            width: w,
            height: h,
            depth: d,
            offset,
        }
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
            draw_boolean_dialog(
                ctx,
                gui,
                "Boolean Intersect",
                &mut show,
                make_bool_intersect,
            );
        }
        gui.show_boolean_intersect = show;
    }

    // -----------------------------------------------------------------------
    // Part operation dialogs
    // -----------------------------------------------------------------------

    // Mirror
    let mut show_mirror = gui.show_mirror;
    if show_mirror {
        egui::Window::new("Mirror Solid")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_mirror)
            .show(ctx, |ui| {
                dialog_section(ui, "Mirror Plane");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut gui.mirror_plane, MirrorPlane::YZ, "YZ");
                    ui.selectable_value(&mut gui.mirror_plane, MirrorPlane::XZ, "XZ");
                    ui.selectable_value(&mut gui.mirror_plane, MirrorPlane::XY, "XY");
                });
                let (ok, cancel, _) = button_bar(ui, "Apply");
                if ok {
                    gui.actions.push(GuiAction::MirrorSolid(gui.mirror_plane));
                    gui.show_mirror = false;
                }
                if cancel {
                    gui.show_mirror = false;
                }
            });
    }
    gui.show_mirror = show_mirror;

    // Scale
    let mut show_scale = gui.show_scale;
    if show_scale {
        egui::Window::new("Scale Solid")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_scale)
            .show(ctx, |ui| {
                dialog_section(ui, "Scale Factor");
                egui::Grid::new("scale_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        param_field_no_unit(
                            ui,
                            "Factor",
                            &mut gui.scale_factor,
                            0.01..=100.0,
                            0.01,
                        );
                    });
                if gui.scale_factor <= 0.0 {
                    validation_error(ui, "Factor must be > 0");
                }
                let (ok, cancel, reset) = button_bar(ui, "Apply");
                if ok && gui.scale_factor > 0.0 {
                    gui.actions.push(GuiAction::ScaleSolid {
                        factor: gui.scale_factor,
                    });
                    gui.show_scale = false;
                }
                if cancel {
                    gui.show_scale = false;
                }
                if reset {
                    gui.scale_factor = 2.0;
                }
            });
    }
    gui.show_scale = show_scale;

    // Shell
    let mut show_shell = gui.show_shell;
    if show_shell {
        egui::Window::new("Shell Solid")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_shell)
            .show(ctx, |ui| {
                dialog_section(ui, "Parameters");
                egui::Grid::new("shell_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        param_field(
                            ui,
                            "Thickness",
                            &mut gui.shell_thickness,
                            0.01..=50.0,
                            0.1,
                            "Wall thickness",
                        );
                    });
                let (ok, cancel, reset) = button_bar(ui, "Apply");
                if ok {
                    gui.actions.push(GuiAction::ShellSolid {
                        thickness: gui.shell_thickness,
                    });
                    gui.show_shell = false;
                }
                if cancel {
                    gui.show_shell = false;
                }
                if reset {
                    gui.shell_thickness = 1.0;
                }
            });
    }
    gui.show_shell = show_shell;

    // Fillet
    let mut show_fillet = gui.show_fillet;
    if show_fillet {
        egui::Window::new("Fillet All Edges")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_fillet)
            .show(ctx, |ui| {
                dialog_section(ui, "Parameters");
                egui::Grid::new("fillet_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        param_field(
                            ui,
                            "Radius",
                            &mut gui.fillet_radius,
                            0.01..=50.0,
                            0.1,
                            "Fillet radius",
                        );
                    });
                let (ok, cancel, reset) = button_bar(ui, "Apply");
                if ok {
                    gui.actions.push(GuiAction::FilletAllEdges {
                        radius: gui.fillet_radius,
                    });
                    gui.show_fillet = false;
                }
                if cancel {
                    gui.show_fillet = false;
                }
                if reset {
                    gui.fillet_radius = 1.0;
                }
            });
    }
    gui.show_fillet = show_fillet;

    // Chamfer
    let mut show_chamfer = gui.show_chamfer;
    if show_chamfer {
        egui::Window::new("Chamfer All Edges")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_chamfer)
            .show(ctx, |ui| {
                dialog_section(ui, "Parameters");
                egui::Grid::new("chamfer_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        param_field(
                            ui,
                            "Distance",
                            &mut gui.chamfer_distance,
                            0.01..=50.0,
                            0.1,
                            "Chamfer distance",
                        );
                    });
                let (ok, cancel, reset) = button_bar(ui, "Apply");
                if ok {
                    gui.actions.push(GuiAction::ChamferAllEdges {
                        distance: gui.chamfer_distance,
                    });
                    gui.show_chamfer = false;
                }
                if cancel {
                    gui.show_chamfer = false;
                }
                if reset {
                    gui.chamfer_distance = 1.0;
                }
            });
    }
    gui.show_chamfer = show_chamfer;

    // Linear Pattern
    let mut show_pattern = gui.show_pattern;
    if show_pattern {
        egui::Window::new("Linear Pattern")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_pattern)
            .show(ctx, |ui| {
                dialog_section(ui, "Pattern Parameters");
                egui::Grid::new("pattern_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("Count:")
                                .size(11.0)
                                .color(theme::COLOR_DIM),
                        );
                        let mut count_i32 = gui.pattern_count as i32;
                        ui.add(egui::DragValue::new(&mut count_i32).range(2..=20));
                        gui.pattern_count = count_i32.max(2) as usize;
                        ui.end_row();
                        param_field(
                            ui,
                            "Spacing",
                            &mut gui.pattern_spacing,
                            0.1..=200.0,
                            0.1,
                            "Distance between copies",
                        );
                        ui.label(
                            egui::RichText::new("Axis:")
                                .size(11.0)
                                .color(theme::COLOR_DIM),
                        );
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut gui.pattern_axis, 0, "X");
                            ui.selectable_value(&mut gui.pattern_axis, 1, "Y");
                            ui.selectable_value(&mut gui.pattern_axis, 2, "Z");
                        });
                        ui.end_row();
                    });
                let (ok, cancel, reset) = button_bar(ui, "Apply");
                if ok {
                    gui.actions.push(GuiAction::LinearPattern {
                        count: gui.pattern_count,
                        spacing: gui.pattern_spacing,
                        axis: gui.pattern_axis,
                    });
                    gui.show_pattern = false;
                }
                if cancel {
                    gui.show_pattern = false;
                }
                if reset {
                    gui.pattern_count = 3;
                    gui.pattern_spacing = 15.0;
                    gui.pattern_axis = 0;
                }
            });
    }
    gui.show_pattern = show_pattern;

    // -----------------------------------------------------------------------
    // Mesh dialogs
    // -----------------------------------------------------------------------

    // Mesh Smooth
    let mut show_smooth = gui.show_mesh_smooth;
    if show_smooth {
        egui::Window::new("Smooth Mesh")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_smooth)
            .show(ctx, |ui| {
                dialog_section(ui, "Smoothing Parameters");
                egui::Grid::new("smooth_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("Iterations:")
                                .size(11.0)
                                .color(theme::COLOR_DIM),
                        );
                        let mut iters_i32 = gui.mesh_smooth_iters as i32;
                        ui.add(egui::DragValue::new(&mut iters_i32).range(1..=20));
                        gui.mesh_smooth_iters = iters_i32.max(1) as usize;
                        ui.end_row();
                        param_field_no_unit(
                            ui,
                            "Factor",
                            &mut gui.mesh_smooth_factor,
                            0.01..=1.0,
                            0.01,
                        );
                    });
                let (ok, cancel, reset) = button_bar(ui, "Apply");
                if ok {
                    gui.actions.push(GuiAction::Mesh(super::MeshAction::Smooth {
                        iterations: gui.mesh_smooth_iters,
                        factor: gui.mesh_smooth_factor,
                    }));
                    gui.show_mesh_smooth = false;
                }
                if cancel {
                    gui.show_mesh_smooth = false;
                }
                if reset {
                    gui.mesh_smooth_iters = 3;
                    gui.mesh_smooth_factor = 0.5;
                }
            });
    }
    gui.show_mesh_smooth = show_smooth;

    // Remesh
    let mut show_remesh = gui.show_mesh_remesh;
    if show_remesh {
        egui::Window::new("Remesh")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_remesh)
            .show(ctx, |ui| {
                dialog_section(ui, "Parameters");
                egui::Grid::new("remesh_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        param_field(
                            ui,
                            "Target Edge Length",
                            &mut gui.mesh_remesh_edge_len,
                            0.1..=100.0,
                            0.1,
                            "Smaller = more detail",
                        );
                    });
                let (ok, cancel, reset) = button_bar(ui, "Apply");
                if ok {
                    gui.actions.push(GuiAction::Mesh(super::MeshAction::Remesh {
                        target_edge_len: gui.mesh_remesh_edge_len,
                    }));
                    gui.show_mesh_remesh = false;
                }
                if cancel {
                    gui.show_mesh_remesh = false;
                }
                if reset {
                    gui.mesh_remesh_edge_len = 1.0;
                }
            });
    }
    gui.show_mesh_remesh = show_remesh;

    // -----------------------------------------------------------------------
    // PartDesign dialogs
    // -----------------------------------------------------------------------

    // Pad
    let mut show_pad = gui.show_pad;
    if show_pad {
        egui::Window::new("Pad (Additive Extrude)")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_pad)
            .show(ctx, |ui| {
                dialog_section(ui, "Parameters");
                egui::Grid::new("pad_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        param_field(
                            ui,
                            "Depth",
                            &mut gui.pad_depth,
                            0.01..=500.0,
                            0.1,
                            "Extrusion depth",
                        );
                    });
                ui.checkbox(&mut gui.pad_symmetric, "Symmetric (both sides)");
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new("Requires active sketch with closed profile")
                        .size(10.0)
                        .color(theme::COLOR_DIM)
                        .italics(),
                );
                let (ok, cancel, reset) = button_bar(ui, "Apply");
                if ok {
                    gui.actions
                        .push(GuiAction::PartDesign(super::PartDesignAction::PadSketch {
                            depth: gui.pad_depth,
                            symmetric: gui.pad_symmetric,
                        }));
                    gui.show_pad = false;
                }
                if cancel {
                    gui.show_pad = false;
                }
                if reset {
                    gui.pad_depth = 10.0;
                    gui.pad_symmetric = false;
                }
            });
    }
    gui.show_pad = show_pad;

    // Pocket
    let mut show_pocket = gui.show_pocket;
    if show_pocket {
        egui::Window::new("Pocket (Subtractive Extrude)")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_pocket)
            .show(ctx, |ui| {
                dialog_section(ui, "Parameters");
                egui::Grid::new("pocket_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        param_field(
                            ui,
                            "Depth",
                            &mut gui.pocket_depth,
                            0.01..=500.0,
                            0.1,
                            "Cut depth",
                        );
                    });
                ui.checkbox(&mut gui.pocket_through_all, "Through all");
                ui.label(
                    egui::RichText::new("Requires active sketch with closed profile")
                        .size(10.0)
                        .color(theme::COLOR_DIM)
                        .italics(),
                );
                let (ok, cancel, reset) = button_bar(ui, "Apply");
                if ok {
                    gui.actions.push(GuiAction::PartDesign(
                        super::PartDesignAction::PocketSketch {
                            depth: gui.pocket_depth,
                            through_all: gui.pocket_through_all,
                        },
                    ));
                    gui.show_pocket = false;
                }
                if cancel {
                    gui.show_pocket = false;
                }
                if reset {
                    gui.pocket_depth = 5.0;
                    gui.pocket_through_all = false;
                }
            });
    }
    gui.show_pocket = show_pocket;

    // Groove
    let mut show_groove = gui.show_groove;
    if show_groove {
        egui::Window::new("Groove (Subtractive Revolve)")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_groove)
            .show(ctx, |ui| {
                dialog_section(ui, "Parameters");
                egui::Grid::new("groove_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("Angle:")
                                .size(11.0)
                                .color(theme::COLOR_DIM),
                        );
                        ui.add(
                            egui::DragValue::new(&mut gui.groove_angle)
                                .speed(1.0)
                                .range(1.0..=360.0)
                                .suffix("°"),
                        );
                        ui.end_row();
                    });
                ui.label(
                    egui::RichText::new("Requires active sketch with profile")
                        .size(10.0)
                        .color(theme::COLOR_DIM)
                        .italics(),
                );
                let (ok, cancel, reset) = button_bar(ui, "Apply");
                if ok {
                    gui.actions.push(GuiAction::PartDesign(
                        super::PartDesignAction::GrooveSketch {
                            angle: gui.groove_angle,
                        },
                    ));
                    gui.show_groove = false;
                }
                if cancel {
                    gui.show_groove = false;
                }
                if reset {
                    gui.groove_angle = 360.0;
                }
            });
    }
    gui.show_groove = show_groove;

    // Hole
    let mut show_hole = gui.show_hole;
    if show_hole {
        egui::Window::new("Hole")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_hole)
            .show(ctx, |ui| {
                dialog_section(ui, "Hole Parameters");
                egui::Grid::new("hole_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        param_field(ui, "Radius", &mut gui.hole_radius, 0.1..=200.0, 0.1, "");
                        param_field(ui, "Depth", &mut gui.hole_depth, 0.1..=500.0, 0.1, "");
                    });
                ui.checkbox(&mut gui.hole_countersink, "Countersink");
                if gui.hole_countersink {
                    egui::Grid::new("csink_grid")
                        .num_columns(2)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("Countersink Angle:")
                                    .size(11.0)
                                    .color(theme::COLOR_DIM),
                            );
                            ui.add(
                                egui::DragValue::new(&mut gui.hole_countersink_angle)
                                    .speed(1.0)
                                    .range(30.0..=120.0)
                                    .suffix("°"),
                            );
                            ui.end_row();
                        });
                }
                let (ok, cancel, reset) = button_bar(ui, "Apply");
                if ok {
                    if gui.hole_countersink {
                        gui.actions.push(GuiAction::PartDesign(
                            super::PartDesignAction::CountersunkHoleSketch {
                                radius: gui.hole_radius,
                                depth: gui.hole_depth,
                                countersink_angle: gui.hole_countersink_angle,
                            },
                        ));
                    } else {
                        gui.actions.push(GuiAction::PartDesign(
                            super::PartDesignAction::HoleSketch {
                                radius: gui.hole_radius,
                                depth: gui.hole_depth,
                            },
                        ));
                    }
                    gui.show_hole = false;
                }
                if cancel {
                    gui.show_hole = false;
                }
                if reset {
                    gui.hole_radius = 2.0;
                    gui.hole_depth = 10.0;
                    gui.hole_countersink = false;
                    gui.hole_countersink_angle = 90.0;
                }
            });
    }
    gui.show_hole = show_hole;

    // Sprocket
    let mut show_sprocket = gui.show_sprocket;
    if show_sprocket {
        egui::Window::new("Create Sprocket")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_sprocket)
            .show(ctx, |ui| {
                dialog_section(ui, "Sprocket Parameters");
                egui::Grid::new("sprocket_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("Teeth:")
                                .size(11.0)
                                .color(theme::COLOR_DIM),
                        );
                        let mut t_i32 = gui.sprocket_teeth as i32;
                        ui.add(egui::DragValue::new(&mut t_i32).range(4..=200));
                        gui.sprocket_teeth = t_i32.max(4) as u32;
                        ui.end_row();
                        param_field(
                            ui,
                            "Roller Diameter",
                            &mut gui.sprocket_roller_diameter,
                            0.1..=100.0,
                            0.1,
                            "",
                        );
                        param_field(
                            ui,
                            "Pitch",
                            &mut gui.sprocket_pitch,
                            0.1..=200.0,
                            0.1,
                            "Chain pitch distance",
                        );
                        param_field(
                            ui,
                            "Bore Radius",
                            &mut gui.sprocket_bore,
                            0.1..=100.0,
                            0.1,
                            "Center bore radius",
                        );
                    });
                let (ok, cancel, reset) = button_bar(ui, "Create");
                if ok {
                    gui.actions.push(GuiAction::PartDesign(
                        super::PartDesignAction::CreateSprocket {
                            teeth: gui.sprocket_teeth,
                            roller_diameter: gui.sprocket_roller_diameter,
                            pitch: gui.sprocket_pitch,
                            bore: gui.sprocket_bore,
                        },
                    ));
                    gui.show_sprocket = false;
                }
                if cancel {
                    gui.show_sprocket = false;
                }
                if reset {
                    gui.sprocket_teeth = 18;
                    gui.sprocket_roller_diameter = 10.16;
                    gui.sprocket_pitch = 15.875;
                    gui.sprocket_bore = 5.0;
                }
            });
    }
    gui.show_sprocket = show_sprocket;

    // Shaft
    let mut show_shaft = gui.show_shaft;
    if show_shaft {
        egui::Window::new("Shaft Design")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_shaft)
            .show(ctx, |ui| {
                dialog_section(ui, "Segments (Length, Diameter)");
                let mut remove_idx = None;
                for (i, seg) in gui.shaft_segments.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("{}:", i + 1)).size(11.0));
                        ui.add(
                            egui::DragValue::new(&mut seg.0)
                                .speed(0.5)
                                .prefix("L=")
                                .range(0.1..=500.0)
                                .suffix(" mm"),
                        );
                        ui.add(
                            egui::DragValue::new(&mut seg.1)
                                .speed(0.5)
                                .prefix("D=")
                                .range(0.1..=500.0)
                                .suffix(" mm"),
                        );
                        if ui
                            .small_button("\u{2716}")
                            .on_hover_text("Remove segment")
                            .clicked()
                        {
                            remove_idx = Some(i);
                        }
                    });
                }
                if let Some(idx) = remove_idx {
                    if gui.shaft_segments.len() > 1 {
                        gui.shaft_segments.remove(idx);
                    }
                }
                if ui.button("+ Add Segment").clicked() {
                    gui.shaft_segments.push((20.0, 10.0));
                }
                let (ok, cancel, reset) = button_bar(ui, "Create");
                if ok {
                    gui.actions.push(GuiAction::PartDesign(
                        super::PartDesignAction::CreateShaftDesign {
                            segments: gui.shaft_segments.clone(),
                        },
                    ));
                    gui.show_shaft = false;
                }
                if cancel {
                    gui.show_shaft = false;
                }
                if reset {
                    gui.shaft_segments = vec![(20.0, 10.0), (30.0, 15.0), (20.0, 10.0)];
                }
            });
    }
    gui.show_shaft = show_shaft;

    // Involute Gear
    let mut show_gear = gui.show_gear;
    if show_gear {
        egui::Window::new("Involute Gear")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_gear)
            .show(ctx, |ui| {
                dialog_section(ui, "Gear Parameters");
                egui::Grid::new("gear_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("Teeth:")
                                .size(11.0)
                                .color(theme::COLOR_DIM),
                        );
                        let mut t_i32 = gui.gear_teeth as i32;
                        ui.add(egui::DragValue::new(&mut t_i32).range(6..=200));
                        gui.gear_teeth = t_i32.max(6) as u32;
                        ui.end_row();
                        param_field(
                            ui,
                            "Module",
                            &mut gui.gear_module,
                            0.1..=50.0,
                            0.1,
                            "Tooth size parameter",
                        );
                        ui.label(
                            egui::RichText::new("Pressure Angle:")
                                .size(11.0)
                                .color(theme::COLOR_DIM),
                        );
                        ui.add(
                            egui::DragValue::new(&mut gui.gear_pressure_angle)
                                .speed(0.5)
                                .range(10.0..=30.0)
                                .suffix("°"),
                        );
                        ui.end_row();
                    });
                let (ok, cancel, reset) = button_bar(ui, "Create");
                if ok {
                    gui.actions.push(GuiAction::PartDesign(
                        super::PartDesignAction::CreateInvoluteGear {
                            teeth: gui.gear_teeth,
                            module_val: gui.gear_module,
                            pressure_angle: gui.gear_pressure_angle,
                        },
                    ));
                    gui.show_gear = false;
                }
                if cancel {
                    gui.show_gear = false;
                }
                if reset {
                    gui.gear_teeth = 20;
                    gui.gear_module = 2.0;
                    gui.gear_pressure_angle = 20.0;
                }
            });
    }
    gui.show_gear = show_gear;

    // Defeaturing
    let mut show_def = gui.show_defeaturing;
    if show_def {
        egui::Window::new("Auto Defeaturing")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_def)
            .show(ctx, |ui| {
                dialog_section(ui, "Parameters");
                egui::Grid::new("defeature_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        param_field(
                            ui,
                            "Size Threshold",
                            &mut gui.defeaturing_threshold,
                            0.01..=100.0,
                            0.1,
                            "Removes features smaller than threshold",
                        );
                    });
                let (ok, cancel, reset) = button_bar(ui, "Apply");
                if ok {
                    gui.actions
                        .push(GuiAction::Part(super::PartAction::AutoDefeaturing {
                            threshold: gui.defeaturing_threshold,
                        }));
                    gui.show_defeaturing = false;
                }
                if cancel {
                    gui.show_defeaturing = false;
                }
                if reset {
                    gui.defeaturing_threshold = 1.0;
                }
            });
    }
    gui.show_defeaturing = show_def;

    // -----------------------------------------------------------------------
    // Export Options dialog
    // -----------------------------------------------------------------------
    let mut show_export = gui.show_export_options;
    if show_export {
        egui::Window::new("Export Options")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_export)
            .default_width(280.0)
            .show(ctx, |ui| {
                if let Some(path) = &gui.export_path {
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
                    dialog_section(ui, &format!("Export: {filename}"));
                    ui.add_space(4.0);

                    egui::Grid::new("export_opts_grid")
                        .num_columns(2)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            if ext == "stl" {
                                ui.label(
                                    egui::RichText::new("Format:")
                                        .size(11.0)
                                        .color(theme::COLOR_DIM),
                                );
                                ui.horizontal(|ui| {
                                    ui.radio_value(&mut gui.export_stl_binary, true, "Binary");
                                    ui.radio_value(&mut gui.export_stl_binary, false, "ASCII");
                                });
                                ui.end_row();
                            }

                            ui.label(
                                egui::RichText::new("Scale:")
                                    .size(11.0)
                                    .color(theme::COLOR_DIM),
                            );
                            ui.add(
                                egui::DragValue::new(&mut gui.export_scale)
                                    .range(0.001..=1000.0)
                                    .speed(0.1)
                                    .suffix("x"),
                            );
                            ui.end_row();
                        });

                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "Format: {} | Scale: {:.2}x",
                            if ext == "stl" {
                                if gui.export_stl_binary {
                                    "Binary STL"
                                } else {
                                    "ASCII STL"
                                }
                            } else {
                                &ext
                            },
                            gui.export_scale
                        ))
                        .size(10.0)
                        .color(theme::COLOR_DIM)
                        .italics(),
                    );
                }

                let (ok, cancel, _) = button_bar(ui, "Export");
                if ok {
                    if let Some(path) = gui.export_path.clone() {
                        let ext = path
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_lowercase();
                        if ext == "stl" {
                            gui.actions.push(GuiAction::ExportStlWithOptions {
                                path,
                                binary: gui.export_stl_binary,
                                scale: gui.export_scale,
                            });
                        } else {
                            gui.actions.push(GuiAction::ExportStl(path));
                        }
                    }
                    gui.show_export_options = false;
                }
                if cancel {
                    gui.show_export_options = false;
                }
            });
    }
    gui.show_export_options = show_export;

    // -----------------------------------------------------------------------
    // FEM dialogs
    // -----------------------------------------------------------------------

    // FEM Material
    let mut show_fem_mat = gui.show_fem_material;
    if show_fem_mat {
        egui::Window::new("FEM Material")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_fem_mat)
            .show(ctx, |ui| {
                dialog_section(ui, "Material Selection");
                ui.label(egui::RichText::new("Preset:").size(11.0));
                egui::ComboBox::from_id_salt("fem_mat_preset")
                    .selected_text(&gui.fem_material_preset)
                    .show_ui(ui, |ui| {
                        for mat in [
                            "Steel",
                            "Aluminum",
                            "Titanium",
                            "Copper",
                            "Brass",
                            "Cast Iron",
                            "Custom",
                        ] {
                            ui.selectable_value(&mut gui.fem_material_preset, mat.to_string(), mat);
                        }
                    });
                let (ok, cancel, _) = button_bar(ui, "Apply");
                if ok {
                    gui.actions
                        .push(GuiAction::Fem(super::FemAction::SetMaterial(
                            gui.fem_material_preset.clone(),
                        )));
                    gui.show_fem_material = false;
                }
                if cancel {
                    gui.show_fem_material = false;
                }
            });
    }
    gui.show_fem_material = show_fem_mat;

    // FEM Mesh
    let mut show_fem_mesh = gui.show_fem_mesh;
    if show_fem_mesh {
        egui::Window::new("FEM Mesh Generation")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_fem_mesh)
            .show(ctx, |ui| {
                dialog_section(ui, "Mesh Parameters");
                egui::Grid::new("fem_mesh_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        param_field(
                            ui,
                            "Element Size",
                            &mut gui.fem_element_size,
                            0.1..=100.0,
                            0.1,
                            "Smaller = finer mesh",
                        );
                    });
                let (ok, cancel, reset) = button_bar(ui, "Generate");
                if ok {
                    gui.actions
                        .push(GuiAction::Fem(super::FemAction::GenTetMesh {
                            element_size: gui.fem_element_size,
                        }));
                    gui.show_fem_mesh = false;
                }
                if cancel {
                    gui.show_fem_mesh = false;
                }
                if reset {
                    gui.fem_element_size = 1.0;
                }
            });
    }
    gui.show_fem_mesh = show_fem_mesh;

    // FEM Constraint
    let mut show_fem_bc = gui.show_fem_constraint;
    if show_fem_bc {
        egui::Window::new("FEM Constraint")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_fem_bc)
            .show(ctx, |ui| {
                dialog_section(ui, "Constraint");
                ui.label(egui::RichText::new("Type:").size(11.0));
                egui::ComboBox::from_id_salt("fem_bc_type")
                    .selected_text(format!("{:?}", gui.fem_constraint_type))
                    .show_ui(ui, |ui| {
                        use super::FemConstraintType;
                        ui.selectable_value(
                            &mut gui.fem_constraint_type,
                            FemConstraintType::Fixed,
                            "Fixed",
                        );
                        ui.selectable_value(
                            &mut gui.fem_constraint_type,
                            FemConstraintType::Force,
                            "Force",
                        );
                        ui.selectable_value(
                            &mut gui.fem_constraint_type,
                            FemConstraintType::Pressure,
                            "Pressure",
                        );
                        ui.selectable_value(
                            &mut gui.fem_constraint_type,
                            FemConstraintType::Displacement,
                            "Displacement",
                        );
                        ui.selectable_value(
                            &mut gui.fem_constraint_type,
                            FemConstraintType::Gravity,
                            "Gravity",
                        );
                        ui.selectable_value(
                            &mut gui.fem_constraint_type,
                            FemConstraintType::Spring,
                            "Spring",
                        );
                    });
                if !matches!(gui.fem_constraint_type, super::FemConstraintType::Fixed) {
                    egui::Grid::new("fem_bc_val_grid")
                        .num_columns(2)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            param_field_no_unit(
                                ui,
                                "Value",
                                &mut gui.fem_constraint_value,
                                -1e6..=1e6,
                                1.0,
                            );
                        });
                }
                let (ok, cancel, _) = button_bar(ui, "Apply");
                if ok {
                    gui.actions
                        .push(GuiAction::Fem(super::FemAction::AddConstraint(
                            gui.fem_constraint_type,
                        )));
                    gui.show_fem_constraint = false;
                }
                if cancel {
                    gui.show_fem_constraint = false;
                }
            });
    }
    gui.show_fem_constraint = show_fem_bc;

    // FEM Solver
    let mut show_fem_sol = gui.show_fem_solver;
    if show_fem_sol {
        egui::Window::new("FEM Solver Settings")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_fem_sol)
            .show(ctx, |ui| {
                dialog_section(ui, "Solver Parameters");
                egui::Grid::new("fem_solver_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        param_field_no_unit(
                            ui,
                            "Tolerance",
                            &mut gui.fem_solver_tolerance,
                            1e-12..=1e-2,
                            1e-7,
                        );
                        ui.label(
                            egui::RichText::new("Max Iterations:")
                                .size(11.0)
                                .color(theme::COLOR_DIM),
                        );
                        let mut iter_i32 = gui.fem_solver_max_iter as i32;
                        ui.add(egui::DragValue::new(&mut iter_i32).range(10..=100000));
                        gui.fem_solver_max_iter = iter_i32.max(10) as usize;
                        ui.end_row();
                    });
                let (ok, cancel, reset) = button_bar(ui, "Apply");
                if ok {
                    gui.show_fem_solver = false;
                }
                if cancel {
                    gui.show_fem_solver = false;
                }
                if reset {
                    gui.fem_solver_tolerance = 1e-6;
                    gui.fem_solver_max_iter = 1000;
                }
            });
    }
    gui.show_fem_solver = show_fem_sol;

    // Exploded View
    let mut show_explode = gui.show_explode;
    if show_explode {
        egui::Window::new("Exploded View")
            .collapsible(false)
            .resizable(false)
            .open(&mut show_explode)
            .show(ctx, |ui| {
                dialog_section(ui, "Parameters");
                egui::Grid::new("explode_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        param_field_no_unit(ui, "Factor", &mut gui.explode_factor, 0.0..=2.0, 0.05);
                    });
                let (ok, cancel, reset) = button_bar(ui, "Apply");
                if ok {
                    gui.actions
                        .push(GuiAction::Assembly(super::AssemblyAction::Explode {
                            factor: gui.explode_factor,
                        }));
                    gui.show_explode = false;
                }
                if cancel {
                    gui.show_explode = false;
                }
                if reset {
                    gui.explode_factor = 0.0;
                }
            });
    }
    gui.show_explode = show_explode;
}

// ---------------------------------------------------------------------------
// About dialog
// ---------------------------------------------------------------------------

pub(crate) fn draw_about_dialog(ctx: &egui::Context, gui: &mut GuiState) {
    let mut show = gui.show_about;
    if show {
        egui::Window::new("About CADKernel")
            .collapsible(false)
            .resizable(false)
            .default_width(380.0)
            .open(&mut show)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("\u{2B22} CADKernel")
                            .size(28.0)
                            .strong()
                            .color(super::theme::COLOR_ACCENT),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new("Open-source CAD Software")
                            .size(14.0)
                            .color(super::theme::COLOR_DIM),
                    );
                    ui.add_space(12.0);
                });
                egui::Grid::new("about_grid")
                    .num_columns(2)
                    .spacing([12.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("Version");
                        ui.label("0.1.0 (pre-alpha)");
                        ui.end_row();
                        ui.strong("License");
                        ui.label("Apache-2.0");
                        ui.end_row();
                        ui.strong("Author");
                        ui.label("Kim DaeHyun");
                        ui.end_row();
                        ui.strong("Renderer");
                        ui.label("wgpu 24 + egui 0.31 + winit 0.30");
                        ui.end_row();
                        ui.strong("Kernel");
                        ui.label("B-Rep + NURBS, 9 crates");
                        ui.end_row();
                        ui.strong("Workbenches");
                        ui.label("Part, PartDesign, Sketcher, Mesh, TechDraw, Assembly, Draft, Surface, FEM");
                        ui.end_row();
                        ui.strong("I/O Formats");
                        ui.label("STL, OBJ, glTF, STEP, IGES, DXF, PLY, 3MF, SVG, BREP, CADK + 4 more");
                        ui.end_row();
                        ui.strong("Tests");
                        ui.label("1,133 passing");
                        ui.end_row();
                    });
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("Built with Rust \u{1F980}")
                            .size(11.0)
                            .color(super::theme::COLOR_DIM),
                    );
                });
            });
    }
    gui.show_about = show;
}

// ---------------------------------------------------------------------------
// `.cadk` Inspector dialog
// ---------------------------------------------------------------------------

/// Renders the `.cadk` Command-log inspector window when
/// `gui.cadk_inspector` is `Some`. Closing the window clears the field.
pub(crate) fn draw_cadk_inspector_dialog(ctx: &egui::Context, gui: &mut GuiState) {
    let Some(report) = gui.cadk_inspector.clone() else {
        return;
    };
    let mut open = true;
    egui::Window::new(".cadk Command Log Inspector")
        .collapsible(true)
        .resizable(true)
        .default_width(560.0)
        .default_height(420.0)
        .open(&mut open)
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new(&report.path)
                    .strong()
                    .color(super::theme::COLOR_ACCENT),
            );
            ui.add_space(4.0);
            egui::Grid::new("cadk_inspector_summary")
                .num_columns(2)
                .spacing([12.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("File size");
                    ui.label(format!("{} bytes", report.size));
                    ui.end_row();
                    ui.strong("Schema version");
                    ui.label(format!("{}", report.schema_version));
                    ui.end_row();
                    ui.strong("Flags");
                    let mut flag_text = format!("0x{:08x}", report.flags);
                    let mut named = Vec::new();
                    if report.flags & cadkernel_api::cadk::CadkFlags::MANIFEST_COMPRESSED != 0 {
                        named.push("MANIFEST_COMPRESSED");
                    }
                    if report.flags & cadkernel_api::cadk::CadkFlags::SIGNED != 0 {
                        named.push("SIGNED");
                    }
                    if report.flags & cadkernel_api::cadk::CadkFlags::HAS_THUMBNAIL != 0 {
                        named.push("HAS_THUMBNAIL");
                    }
                    if !named.is_empty() {
                        flag_text.push_str("  (");
                        flag_text.push_str(&named.join(" | "));
                        flag_text.push(')');
                    }
                    ui.label(flag_text);
                    ui.end_row();
                    ui.strong("Commands");
                    ui.label(format!("{}", report.command_count));
                    ui.end_row();
                    ui.strong("Thumbnail");
                    match report.thumbnail_size {
                        Some(n) => ui.label(format!("{n} bytes (CRC OK)")),
                        None => ui.label("absent"),
                    };
                    ui.end_row();
                });
            ui.add_space(6.0);
            if let Some(err) = &report.error {
                ui.colored_label(egui::Color32::from_rgb(220, 90, 90), format!("⚠ {err}"));
                ui.add_space(4.0);
            } else {
                ui.colored_label(egui::Color32::from_rgb(120, 200, 120), "✓ Status: OK");
                ui.add_space(4.0);
            }
            ui.separator();
            ui.label(
                egui::RichText::new(format!(
                    "Commands (showing first {} of {})",
                    report.commands_preview.len(),
                    report.command_count
                ))
                .color(super::theme::COLOR_DIM),
            );
            egui::ScrollArea::vertical()
                .max_height(220.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (i, line) in report.commands_preview.iter().enumerate() {
                        ui.monospace(format!("[{i:>4}] {line}"));
                    }
                });
        });
    if !open {
        gui.cadk_inspector = None;
    }
}

// ---------------------------------------------------------------------------
// Settings dialog
// ---------------------------------------------------------------------------

/// Settings tab (stored via egui temp data).
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum SettingsTab {
    #[default]
    General,
    Display,
    Navigation,
    Appearance,
    Lighting,
    Shortcuts,
}

impl SettingsTab {
    const ALL: &[SettingsTab] = &[
        SettingsTab::General,
        SettingsTab::Display,
        SettingsTab::Navigation,
        SettingsTab::Appearance,
        SettingsTab::Lighting,
        SettingsTab::Shortcuts,
    ];

    fn icon(self) -> &'static str {
        match self {
            Self::General => "\u{2699}",     // ⚙
            Self::Display => "\u{1F5B5}",    // 🖵
            Self::Navigation => "\u{21C4}",  // ⇄
            Self::Appearance => "\u{1F3A8}", // 🎨
            Self::Lighting => "\u{2600}",    // ☀
            Self::Shortcuts => "\u{2328}",   // ⌨
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Display => "Display",
            Self::Navigation => "Navigation",
            Self::Appearance => "Appearance",
            Self::Lighting => "Lighting",
            Self::Shortcuts => "Shortcuts",
        }
    }
}

pub(crate) fn draw_settings(ctx: &egui::Context, gui: &mut GuiState, nav: &mut NavConfig) {
    let mut show = gui.show_settings;
    if !show {
        return;
    }

    let tab_id = egui::Id::new("settings_active_tab");

    egui::Window::new("Preferences")
        .collapsible(false)
        .resizable(true)
        .default_width(560.0)
        .default_height(480.0)
        .open(&mut show)
        .show(ctx, |ui| {
            let active_tab: SettingsTab =
                ui.data_mut(|d| *d.get_temp_mut_or(tab_id, SettingsTab::General));

            ui.horizontal(|ui| {
                // Sidebar tabs
                ui.vertical(|ui| {
                    ui.set_min_width(120.0);
                    for &tab in SettingsTab::ALL {
                        let selected = active_tab == tab;
                        let label = egui::RichText::new(format!("{} {}", tab.icon(), tab.label()))
                            .size(12.0);
                        let label = if selected {
                            label.strong().color(theme::COLOR_ACCENT)
                        } else {
                            label
                        };
                        let resp = ui.selectable_label(selected, label);
                        if resp.clicked() {
                            ui.data_mut(|d| d.insert_temp(tab_id, tab));
                        }
                    }
                    ui.add_space(16.0);
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Reset All")
                                    .size(11.0)
                                    .color(theme::COLOR_DIM),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        *nav = NavConfig::new();
                        gui.theme_applied = false;
                        gui.status_message = "Settings reset to defaults".into();
                    }
                });

                ui.separator();

                // Content area (must be vertical for indent/heading to work)
                ui.vertical(|ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("settings_scroll")
                        .show(ui, |ui| {
                            ui.set_min_width(380.0);
                            match active_tab {
                                SettingsTab::General => draw_settings_general(ui, nav),
                                SettingsTab::Display => draw_settings_display(ui, nav),
                                SettingsTab::Navigation => draw_settings_navigation(ui, nav),
                                SettingsTab::Appearance => draw_settings_appearance(ui, gui, nav),
                                SettingsTab::Lighting => draw_settings_lighting(ui, nav),
                                SettingsTab::Shortcuts => draw_settings_shortcuts(ui),
                            }
                        });
                });
            });
        });
    gui.show_settings = show;
}

fn settings_heading(ui: &mut egui::Ui, label: &str) {
    ui.add_space(2.0);
    ui.label(
        egui::RichText::new(label)
            .size(14.0)
            .strong()
            .color(theme::COLOR_ACCENT),
    );
    ui.separator();
}

fn settings_subheading(ui: &mut egui::Ui, label: &str) {
    ui.add_space(6.0);
    ui.label(egui::RichText::new(label).size(11.5).strong());
}

// -- General tab --
fn draw_settings_general(ui: &mut egui::Ui, nav: &mut NavConfig) {
    settings_heading(ui, "Units");
    egui::Grid::new("units_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Unit system:");
            egui::ComboBox::from_id_salt("unit_system")
                .selected_text(nav.unit_system.long_label())
                .show_ui(ui, |ui| {
                    for &u in UnitSystem::ALL {
                        ui.selectable_value(&mut nav.unit_system, u, u.long_label());
                    }
                });
            ui.end_row();

            ui.label("Decimal places:");
            ui.add(egui::DragValue::new(&mut nav.decimal_places).range(0..=8));
            ui.end_row();
        });

    settings_heading(ui, "Files");
    ui.indent("files_indent", |ui| {
        ui.checkbox(&mut nav.auto_save_enabled, "Enable auto-save");
        if nav.auto_save_enabled {
            egui::Grid::new("autosave_grid")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Interval:");
                    ui.add(
                        egui::DragValue::new(&mut nav.auto_save_interval_secs)
                            .range(30..=3600)
                            .suffix(" sec"),
                    );
                    ui.end_row();
                });
        }
        ui.add_space(4.0);
        egui::Grid::new("files_grid")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Recent files limit:");
                ui.add(egui::DragValue::new(&mut nav.recent_files_max).range(1..=50));
                ui.end_row();
            });
    });

    settings_heading(ui, "Behavior");
    ui.indent("behavior_indent", |ui| {
        ui.checkbox(&mut nav.confirm_delete, "Confirm before deleting objects");
    });
}

// -- Display tab --
fn draw_settings_display(ui: &mut egui::Ui, nav: &mut NavConfig) {
    settings_heading(ui, "Background");
    ui.indent("bg_indent", |ui| {
        egui::Grid::new("bg_grid")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Preset:");
                egui::ComboBox::from_id_salt("bg_preset")
                    .selected_text(nav.bg_preset.label())
                    .show_ui(ui, |ui| {
                        for &p in BgPreset::ALL {
                            ui.selectable_value(&mut nav.bg_preset, p, p.label());
                        }
                    });
                ui.end_row();

                if nav.bg_preset == BgPreset::Custom {
                    ui.label("Top color:");
                    let mut top = [
                        (nav.bg_custom_top[0] * 255.0) as u8,
                        (nav.bg_custom_top[1] * 255.0) as u8,
                        (nav.bg_custom_top[2] * 255.0) as u8,
                    ];
                    if ui.color_edit_button_srgb(&mut top).changed() {
                        nav.bg_custom_top = [
                            top[0] as f32 / 255.0,
                            top[1] as f32 / 255.0,
                            top[2] as f32 / 255.0,
                        ];
                    }
                    ui.end_row();

                    ui.label("Bottom color:");
                    let mut bot = [
                        (nav.bg_custom_bottom[0] * 255.0) as u8,
                        (nav.bg_custom_bottom[1] * 255.0) as u8,
                        (nav.bg_custom_bottom[2] * 255.0) as u8,
                    ];
                    if ui.color_edit_button_srgb(&mut bot).changed() {
                        nav.bg_custom_bottom = [
                            bot[0] as f32 / 255.0,
                            bot[1] as f32 / 255.0,
                            bot[2] as f32 / 255.0,
                        ];
                    }
                    ui.end_row();
                }
            });
    });

    settings_heading(ui, "Viewport Overlays");
    ui.indent("overlays_indent", |ui| {
        ui.checkbox(&mut nav.show_axes_indicator, "Show coordinate axes");
        ui.checkbox(&mut nav.show_origin, "Show origin marker");
        ui.checkbox(&mut nav.show_grid_3d, "Show 3D grid");
        if nav.show_grid_3d {
            egui::Grid::new("grid_settings")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Grid spacing:");
                    ui.add(
                        egui::DragValue::new(&mut nav.grid_3d_spacing)
                            .range(0.01..=1000.0)
                            .speed(0.1)
                            .suffix(format!(" {}", nav.unit_system.label())),
                    );
                    ui.end_row();
                });
        }
        ui.checkbox(&mut nav.snap_to_grid_3d, "Snap to 3D grid");
        ui.checkbox(&mut nav.show_fps, "Show FPS counter");
    });

    settings_heading(ui, "Camera");
    ui.indent("cam_indent", |ui| {
        egui::Grid::new("cam_grid")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Default projection:");
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
                ui.end_row();
            });
    });

    settings_heading(ui, "Selection");
    ui.indent("sel_indent", |ui| {
        egui::Grid::new("sel_grid")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Selection color:");
                let mut sel_c = egui::Color32::from_rgb(
                    nav.selection_color[0],
                    nav.selection_color[1],
                    nav.selection_color[2],
                );
                if ui.color_edit_button_srgba(&mut sel_c).changed() {
                    nav.selection_color = [sel_c.r(), sel_c.g(), sel_c.b()];
                }
                ui.end_row();

                ui.label("Pre-selection color:");
                let mut pre_c = egui::Color32::from_rgb(
                    nav.preselection_color[0],
                    nav.preselection_color[1],
                    nav.preselection_color[2],
                );
                if ui.color_edit_button_srgba(&mut pre_c).changed() {
                    nav.preselection_color = [pre_c.r(), pre_c.g(), pre_c.b()];
                }
                ui.end_row();
            });
    });

    settings_heading(ui, "Tessellation");
    ui.indent("tess_indent", |ui| {
        egui::Grid::new("tess_grid")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Default segments:");
                ui.add(
                    egui::Slider::new(&mut nav.tessellation_segments, 8..=256).logarithmic(true),
                );
                ui.end_row();
            });
        ui.weak("Segments for cylinders, spheres, and other curved primitives.");
    });

    settings_heading(ui, "Section Plane");
    ui.indent("section_indent", |ui| {
        ui.checkbox(&mut nav.clip_enabled, "Enable section plane");
        if nav.clip_enabled {
            egui::Grid::new("section_grid")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Normal axis:");
                    ui.horizontal(|ui| {
                        let n = &mut nav.clip_plane_normal;
                        if ui.selectable_label(*n == [1.0, 0.0, 0.0], "X").clicked() {
                            *n = [1.0, 0.0, 0.0];
                        }
                        if ui.selectable_label(*n == [0.0, 1.0, 0.0], "Y").clicked() {
                            *n = [0.0, 1.0, 0.0];
                        }
                        if ui.selectable_label(*n == [0.0, 0.0, 1.0], "Z").clicked() {
                            *n = [0.0, 0.0, 1.0];
                        }
                    });
                    ui.end_row();

                    ui.label("Offset:");
                    ui.add(
                        egui::DragValue::new(&mut nav.clip_plane_offset)
                            .range(-1000.0..=1000.0)
                            .speed(0.1),
                    );
                    ui.end_row();
                });
        }
    });
}

// -- Navigation tab --
fn draw_settings_navigation(ui: &mut egui::Ui, nav: &mut NavConfig) {
    settings_heading(ui, "Mouse Style");
    ui.indent("orbit_style_indent", |ui| {
        egui::ComboBox::from_id_salt("mouse_style")
            .selected_text(nav.style.label())
            .width(200.0)
            .show_ui(ui, |ui| {
                for &style in NavStyle::ALL {
                    ui.selectable_value(&mut nav.style, style, style.label());
                }
            });
        ui.add_space(2.0);
        ui.weak(nav.style.description());
    });

    settings_heading(ui, "Orbit & Rotation");
    ui.indent("orbit_rot_indent", |ui| {
        egui::Grid::new("orbit_rot_grid")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Orbit style:");
                egui::ComboBox::from_id_salt("orbit_style")
                    .selected_text(nav.orbit_style.label())
                    .width(160.0)
                    .show_ui(ui, |ui| {
                        for &style in OrbitStyle::ALL {
                            ui.selectable_value(&mut nav.orbit_style, style, style.label());
                        }
                    });
                ui.end_row();

                ui.label("Rotation center:");
                egui::ComboBox::from_id_salt("rotation_mode")
                    .selected_text(nav.rotation_mode.label())
                    .width(160.0)
                    .show_ui(ui, |ui| {
                        for &mode in RotationMode::ALL {
                            ui.selectable_value(&mut nav.rotation_mode, mode, mode.label());
                        }
                    });
                ui.end_row();
            });
        ui.checkbox(
            &mut nav.show_rotation_center,
            "Show rotation center indicator",
        );
        if nav.show_rotation_center {
            ui.indent("rot_center_size", |ui| {
                ui.add(egui::Slider::new(&mut nav.rotation_center_size, 1.0..=20.0).text("Size"));
            });
        }
    });

    settings_heading(ui, "Sensitivity");
    ui.indent("sens_indent", |ui| {
        egui::Grid::new("sens_grid")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Orbit:");
                ui.add(
                    egui::Slider::new(&mut nav.orbit_sensitivity, 0.001..=0.02).logarithmic(true),
                );
                ui.end_row();
                ui.label("Pan:");
                ui.add(
                    egui::Slider::new(&mut nav.pan_sensitivity, 0.0005..=0.01).logarithmic(true),
                );
                ui.end_row();
                ui.label("Zoom:");
                ui.add(egui::Slider::new(&mut nav.zoom_sensitivity, 0.02..=0.5).logarithmic(true));
                ui.end_row();
                ui.label("Zoom step:");
                ui.add(egui::Slider::new(&mut nav.zoom_step, 0.01..=1.0).step_by(0.05));
                ui.end_row();
            });
        ui.checkbox(&mut nav.zoom_at_cursor, "Zoom at cursor");
        ui.checkbox(&mut nav.invert_zoom, "Invert zoom direction");
        ui.checkbox(
            &mut nav.disable_touch_tilt,
            "Disable touchscreen tilt gesture",
        );
    });

    settings_heading(ui, "Animation");
    ui.indent("anim_indent", |ui| {
        ui.checkbox(&mut nav.enable_view_animation, "Animate view transitions");
        if nav.enable_view_animation {
            egui::Grid::new("anim_grid")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Duration:");
                    ui.add(
                        egui::Slider::new(&mut nav.view_animation_duration, 0.1..=2.0)
                            .suffix(" s")
                            .step_by(0.05),
                    );
                    ui.end_row();
                });
        }
        ui.checkbox(&mut nav.enable_spinning, "Enable spinning animations");
    });

    settings_heading(ui, "View Cube");
    ui.indent("cube_indent", |ui| {
        ui.checkbox(&mut nav.show_view_cube, "Show View Cube");
        if nav.show_view_cube {
            egui::Grid::new("cube_settings")
                .num_columns(2)
                .spacing([12.0, 6.0])
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
                    ui.add(egui::Slider::new(&mut nav.cube_opacity, 0.1..=1.0).show_value(true));
                    ui.end_row();
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
            ui.checkbox(&mut nav.snap_to_nearest, "Snap to nearest standard view");
        }
    });
}

// -- Appearance tab --
fn draw_settings_appearance(ui: &mut egui::Ui, gui: &mut GuiState, nav: &mut NavConfig) {
    settings_heading(ui, "Theme");
    ui.indent("theme_indent", |ui| {
        ui.horizontal(|ui| {
            if ui
                .radio_value(&mut nav.theme_mode, super::theme::ThemeMode::Dark, "Dark")
                .changed()
                || ui
                    .radio_value(&mut nav.theme_mode, super::theme::ThemeMode::Light, "Light")
                    .changed()
            {
                gui.theme_applied = false;
            }
        });
    });

    settings_heading(ui, "UI Density");
    ui.indent("density_indent", |ui| {
        ui.horizontal(|ui| {
            if ui
                .radio_value(
                    &mut nav.ui_density,
                    super::theme::UiDensity::Compact,
                    "Compact",
                )
                .changed()
                || ui
                    .radio_value(
                        &mut nav.ui_density,
                        super::theme::UiDensity::Normal,
                        "Normal",
                    )
                    .changed()
                || ui
                    .radio_value(
                        &mut nav.ui_density,
                        super::theme::UiDensity::Spacious,
                        "Spacious",
                    )
                    .changed()
            {
                gui.theme_applied = false;
            }
        });
        ui.add_space(4.0);
        let desc = match nav.ui_density {
            super::theme::UiDensity::Compact => "Smaller spacing, fits more on screen.",
            super::theme::UiDensity::Normal => "Balanced spacing for most workflows.",
            super::theme::UiDensity::Spacious => "Extra padding, easier to click targets.",
        };
        ui.weak(desc);
    });
}

// -- Lighting tab --
fn draw_settings_lighting(ui: &mut egui::Ui, nav: &mut NavConfig) {
    settings_heading(ui, "Scene Lighting");
    ui.indent("light_indent", |ui| {
        ui.checkbox(&mut nav.enable_lighting, "Enable lighting");
        if nav.enable_lighting {
            ui.add_space(4.0);
            egui::Grid::new("light_grid")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Intensity:");
                    ui.add(egui::Slider::new(&mut nav.light_intensity, 0.0..=2.0));
                    ui.end_row();
                });

            settings_subheading(ui, "Light Direction");
            egui::Grid::new("light_dir_grid")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label("X:");
                    ui.add(
                        egui::DragValue::new(&mut nav.light_dir[0])
                            .speed(0.01)
                            .range(-1.0..=1.0),
                    );
                    ui.end_row();
                    ui.label("Y:");
                    ui.add(
                        egui::DragValue::new(&mut nav.light_dir[1])
                            .speed(0.01)
                            .range(-1.0..=1.0),
                    );
                    ui.end_row();
                    ui.label("Z:");
                    ui.add(
                        egui::DragValue::new(&mut nav.light_dir[2])
                            .speed(0.01)
                            .range(-1.0..=1.0),
                    );
                    ui.end_row();
                });
            ui.add_space(4.0);
            ui.weak("Camera-relative offset: light tracks the viewpoint.");
        }
    });
}

// ---------------------------------------------------------------------------
// Keyboard Shortcuts dialog
// ---------------------------------------------------------------------------

fn draw_all_shortcuts(ui: &mut egui::Ui) {
    shortcut_section(
        ui,
        "\u{1F4C1}",
        "File",
        &[
            ("Ctrl+N", "New project"),
            ("Ctrl+O", "Open file"),
            ("Ctrl+S", "Save file"),
        ],
    );
    shortcut_section(
        ui,
        "\u{270F}",
        "Edit",
        &[
            ("Ctrl+Z", "Undo"),
            ("Ctrl+Y / Ctrl+Shift+Z", "Redo"),
            ("Ctrl+A", "Select All"),
            ("Ctrl+C", "Copy (sketch)"),
            ("Ctrl+V", "Paste (sketch)"),
            ("Delete / Backspace", "Delete selected"),
            ("Escape", "Cancel / Deselect"),
        ],
    );
    shortcut_section(
        ui,
        "\u{1F3AF}",
        "Navigation",
        &[
            ("Left Mouse + Drag", "Orbit camera"),
            ("Middle Mouse + Drag", "Pan camera"),
            ("Scroll Wheel", "Zoom in/out"),
            ("V / F", "Fit All"),
            ("5", "Toggle Perspective / Orthographic"),
            ("G", "Toggle Grid"),
            ("Shift+S", "Toggle Section Plane"),
            ("D", "Cycle Display Mode"),
            ("H", "Toggle visibility of selected"),
        ],
    );
    shortcut_section(
        ui,
        "\u{1F441}",
        "Standard Views",
        &[
            ("1 / Ctrl+1", "Front / Back"),
            ("3 / Ctrl+3", "Right / Left"),
            ("7 / Ctrl+7", "Top / Bottom"),
            ("0", "Isometric"),
            ("Q / E", "Roll CW / CCW"),
        ],
    );
    shortcut_section(
        ui,
        "\u{1F5BC}",
        "Display Modes (V, then...)",
        &[
            ("V, 1", "As Is"),
            ("V, 2", "Points"),
            ("V, 3", "Wireframe"),
            ("V, 4", "Hidden Line"),
            ("V, 5", "No Shading"),
            ("V, 6", "Shading (default)"),
            ("V, 7", "Flat Lines"),
            ("V, 8", "Transparent"),
        ],
    );
    shortcut_section(
        ui,
        "\u{21C4}",
        "Transform Gizmo",
        &[
            ("W", "Move gizmo"),
            ("E", "Rotate gizmo"),
            ("R", "Scale gizmo"),
            ("Drag axis handle", "Transform along axis"),
        ],
    );
    shortcut_section(
        ui,
        "\u{1F50D}",
        "Selection Modes",
        &[
            ("2", "Face selection"),
            ("4", "Vertex selection"),
            ("C", "Clear measurement"),
        ],
    );
    shortcut_section(
        ui,
        "\u{2712}",
        "Sketcher (in sketch mode)",
        &[
            ("S", "Select tool"),
            ("L", "Line tool"),
            ("R", "Rectangle tool"),
            ("C", "Circle tool"),
            ("A", "Arc tool"),
            ("E", "Ellipse tool"),
            ("P", "Polyline tool"),
            ("B", "B-Spline tool"),
            ("W", "Slot tool"),
            ("H", "Horizontal constraint"),
            ("V", "Vertical constraint"),
            ("Enter", "Close sketch & extrude"),
            ("Escape", "Exit sketch"),
        ],
    );
    shortcut_section(
        ui,
        "\u{2139}",
        "General",
        &[("F1", "Toggle this shortcuts panel")],
    );
}

pub(crate) fn draw_shortcuts_dialog(ctx: &egui::Context, gui: &mut GuiState) {
    if !gui.show_shortcuts {
        return;
    }

    egui::Window::new("\u{2328}  Keyboard Shortcuts")
        .collapsible(false)
        .resizable(true)
        .default_width(420.0)
        .default_height(500.0)
        .open(&mut gui.show_shortcuts)
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new("Press F1 to toggle this panel")
                    .size(10.0)
                    .color(theme::COLOR_DIM)
                    .italics(),
            );
            ui.add_space(2.0);
            egui::ScrollArea::vertical().show(ui, |ui| {
                draw_all_shortcuts(ui);
            });
        });
}

fn draw_settings_shortcuts(ui: &mut egui::Ui) {
    settings_heading(ui, "Keyboard Shortcuts Reference");
    ui.weak(
        "All current keyboard shortcuts. Custom key binding will be available in a future release.",
    );
    ui.add_space(4.0);
    draw_all_shortcuts(ui);
}

// ---------------------------------------------------------------------------
// Plugin Manager dialog
// ---------------------------------------------------------------------------

pub(crate) fn draw_plugin_manager(ctx: &egui::Context, gui: &mut GuiState) {
    let mut show = gui.show_plugin_manager;
    if !show {
        return;
    }
    egui::Window::new("\u{1F9E9}  Plugin Manager")
        .collapsible(false)
        .resizable(true)
        .default_width(420.0)
        .default_height(300.0)
        .open(&mut show)
        .show(ctx, |ui| {
            dialog_section(ui, "Installed Plugins");

            // The plugin list is mirrored into gui by the action handler.
            // We read it from egui temp data (set by process_actions after
            // InitPlugins or each frame).
            let plugin_list_id = egui::Id::new("plugin_list_data");
            let plugins: Vec<(usize, String, String, String)> = ui.data_mut(|d| {
                d.get_temp::<Vec<(usize, String, String, String)>>(plugin_list_id)
                    .unwrap_or_default()
            });

            if plugins.is_empty() {
                ui.add_space(8.0);
                ui.weak("No plugins registered.");
                ui.add_space(4.0);
                ui.weak("Use Tools > Plugins > Initialize Plugins to load built-in plugins.");
                ui.add_space(8.0);
            } else {
                egui::Grid::new("plugin_grid")
                    .num_columns(4)
                    .spacing([12.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        // Header
                        ui.label(egui::RichText::new("Name").strong().size(11.0));
                        ui.label(egui::RichText::new("Version").strong().size(11.0));
                        ui.label(egui::RichText::new("State").strong().size(11.0));
                        ui.label(egui::RichText::new("").size(11.0));
                        ui.end_row();

                        for (_, name, version, state) in &plugins {
                            ui.label(egui::RichText::new(name).size(11.0));
                            ui.label(
                                egui::RichText::new(version)
                                    .size(11.0)
                                    .color(theme::COLOR_DIM),
                            );
                            let state_color = match state.as_str() {
                                "Active" => egui::Color32::from_rgb(50, 200, 100),
                                "Loaded" => egui::Color32::from_rgb(100, 180, 220),
                                "Unloaded" => theme::COLOR_DIM,
                                _ => egui::Color32::from_rgb(220, 60, 60),
                            };
                            ui.label(egui::RichText::new(state).size(11.0).color(state_color));
                            // Placeholder for future enable/disable
                            ui.label(egui::RichText::new("").size(11.0));
                            ui.end_row();
                        }
                    });
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Initialize All").clicked() {
                    gui.actions.push(GuiAction::InitPlugins);
                }
                ui.label(
                    egui::RichText::new(format!("{} plugin(s)", plugins.len()))
                        .size(10.0)
                        .color(theme::COLOR_DIM),
                );
            });
        });
    gui.show_plugin_manager = show;
}

fn shortcut_section(ui: &mut egui::Ui, icon: &str, title: &str, shortcuts: &[(&str, &str)]) {
    dialog_section(ui, &format!("{icon}  {title}"));
    egui::Grid::new(format!("shortcuts_{title}"))
        .num_columns(2)
        .spacing([16.0, 2.0])
        .striped(true)
        .show(ui, |ui| {
            for (key, desc) in shortcuts {
                // Key badge — monospace with subtle background
                let key_text = egui::RichText::new(*key)
                    .size(11.0)
                    .monospace()
                    .strong()
                    .color(egui::Color32::from_rgb(200, 210, 225));
                ui.label(key_text);
                ui.label(
                    egui::RichText::new(*desc)
                        .size(11.0)
                        .color(egui::Color32::from_rgb(160, 168, 180)),
                );
                ui.end_row();
            }
        });
}

// ---------------------------------------------------------------------------
// Assembly: Bill of Materials dialog
// ---------------------------------------------------------------------------

pub(crate) fn draw_bom_dialog(ctx: &egui::Context, gui: &mut GuiState) {
    let entries = match &gui.active_dialog {
        Some(crate::gui::ActiveDialog::Bom(e)) => e.clone(),
        _ => return,
    };
    let total: usize = entries.iter().map(|e| e.quantity).sum();
    let mut open = true;
    let mut close_clicked = false;
    egui::Window::new("Bill of Materials")
        .collapsible(false)
        .resizable(true)
        .default_width(360.0)
        .open(&mut open)
        .show(ctx, |ui| {
            dialog_section(ui, "Parts");
            egui::Grid::new("bom_grid")
                .num_columns(3)
                .spacing([14.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Index").strong());
                    ui.label(egui::RichText::new("Part Name").strong());
                    ui.label(egui::RichText::new("Qty").strong());
                    ui.end_row();
                    for e in &entries {
                        ui.label(format!("{}", e.index));
                        ui.label(&e.name);
                        ui.label(format!("{}", e.quantity));
                        ui.end_row();
                    }
                });
            ui.add_space(6.0);
            ui.label(egui::RichText::new(format!("Total parts: {total}")).strong());
            ui.add_space(4.0);
            if ui.button("Close").clicked() {
                close_clicked = true;
            }
        });
    if !open || close_clicked {
        gui.close_active_dialog();
    }
}

// ---------------------------------------------------------------------------
// Assembly: Joint editor dialog
// ---------------------------------------------------------------------------

pub(crate) fn draw_joint_editor_dialog(ctx: &egui::Context, gui: &mut GuiState) {
    // Snapshot component (idx, name) pairs for ComboBox population. Done
    // before the mutable borrow of `active_dialog`.
    let comps: Vec<(usize, String)> = gui
        .assembly
        .as_ref()
        .map(|a| {
            a.components
                .iter()
                .enumerate()
                .map(|(i, c)| (i, c.name.clone()))
                .collect()
        })
        .unwrap_or_default();

    let state = match gui.joint_editor_state_mut() {
        Some(s) => s,
        None => return,
    };
    let jtype = state.joint_type;
    if comps.len() < jtype.min_components() {
        gui.close_active_dialog();
        return;
    }
    let grounded = matches!(jtype, AssemblyJointType::Grounded);
    let mut open = true;
    let mut do_create = false;
    let mut do_cancel = false;
    let title = format!("Joint: {}", jtype.label());
    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .default_width(320.0)
        .open(&mut open)
        .show(ctx, |ui| {
            dialog_section(ui, "Components");
            egui::Grid::new("je_comps")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Component A:");
                    egui::ComboBox::from_id_salt("je_comp_a")
                        .selected_text(
                            comps
                                .get(state.comp_a)
                                .map(|(i, n)| format!("{i}: {n}"))
                                .unwrap_or_else(|| "?".into()),
                        )
                        .show_ui(ui, |ui| {
                            for (i, n) in &comps {
                                ui.selectable_value(&mut state.comp_a, *i, format!("{i}: {n}"));
                            }
                        });
                    ui.end_row();
                    if !grounded {
                        ui.label("Component B:");
                        egui::ComboBox::from_id_salt("je_comp_b")
                            .selected_text(
                                comps
                                    .get(state.comp_b)
                                    .map(|(i, n)| format!("{i}: {n}"))
                                    .unwrap_or_else(|| "?".into()),
                            )
                            .show_ui(ui, |ui| {
                                for (i, n) in &comps {
                                    ui.selectable_value(&mut state.comp_b, *i, format!("{i}: {n}"));
                                }
                            });
                        ui.end_row();
                    }
                });
            if !grounded {
                dialog_section(ui, "Parameters");
                egui::Grid::new("je_params")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        if joint_needs_axis(jtype) {
                            ui.label("Axis X/Y/Z:");
                            ui.horizontal(|ui| {
                                for k in 0..3 {
                                    ui.add(egui::DragValue::new(&mut state.axis[k]).speed(0.05));
                                }
                            });
                            ui.end_row();
                        }
                        if joint_needs_origin(jtype) {
                            let lab = if matches!(jtype, AssemblyJointType::Ball) {
                                "Center:"
                            } else {
                                "Origin:"
                            };
                            ui.label(lab);
                            ui.horizontal(|ui| {
                                for k in 0..3 {
                                    ui.add(egui::DragValue::new(&mut state.origin[k]).speed(0.1));
                                }
                            });
                            ui.end_row();
                        }
                        if matches!(
                            jtype,
                            AssemblyJointType::Angle | AssemblyJointType::Distance
                        ) {
                            ui.label("Angle (deg):");
                            ui.add(
                                egui::DragValue::new(&mut state.angle)
                                    .range(-360.0..=360.0)
                                    .speed(1.0),
                            );
                            ui.end_row();
                        }
                        if matches!(jtype, AssemblyJointType::Screw | AssemblyJointType::Rack) {
                            ui.label("Pitch:");
                            ui.add(
                                egui::DragValue::new(&mut state.pitch)
                                    .range(0.01..=1000.0)
                                    .speed(0.1),
                            );
                            ui.end_row();
                        }
                        if matches!(jtype, AssemblyJointType::Gear | AssemblyJointType::Belt) {
                            ui.label("Ratio:");
                            ui.add(
                                egui::DragValue::new(&mut state.ratio)
                                    .range(0.01..=100.0)
                                    .speed(0.05),
                            );
                            ui.end_row();
                        }
                    });
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Create")
                                .strong()
                                .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(0, 100, 180))
                        .min_size(egui::vec2(70.0, 24.0)),
                    )
                    .clicked()
                {
                    do_create = true;
                }
                if ui
                    .add(egui::Button::new("Cancel").min_size(egui::vec2(60.0, 24.0)))
                    .clicked()
                {
                    do_cancel = true;
                }
            });
        });
    if !open || do_cancel {
        gui.close_active_dialog();
    } else if do_create {
        gui.actions
            .push(GuiAction::Assembly(super::AssemblyAction::CommitJoint));
    }
}

fn joint_needs_axis(t: AssemblyJointType) -> bool {
    matches!(
        t,
        AssemblyJointType::Revolute
            | AssemblyJointType::Cylindrical
            | AssemblyJointType::Slider
            | AssemblyJointType::Parallel
            | AssemblyJointType::Perpendicular
            | AssemblyJointType::Screw
    )
}

fn joint_needs_origin(t: AssemblyJointType) -> bool {
    matches!(
        t,
        AssemblyJointType::Revolute | AssemblyJointType::Cylindrical | AssemblyJointType::Ball
    )
}

// ---------------------------------------------------------------------------
// TechDraw: page setup dialog
// ---------------------------------------------------------------------------

pub(crate) fn draw_techdraw_page_setup_dialog(ctx: &egui::Context, gui: &mut GuiState) {
    let state = match &mut gui.active_dialog {
        Some(crate::gui::ActiveDialog::TechDrawPageSetup(s)) => s,
        _ => return,
    };
    let mut open = true;
    let (mut do_ok, mut do_cancel) = (false, false);
    egui::Window::new("TechDraw Page Setup")
        .collapsible(false)
        .resizable(false)
        .default_width(340.0)
        .open(&mut open)
        .show(ctx, |ui| {
            dialog_section(ui, "Template");
            let mut selected = state.preset;
            egui::ComboBox::from_id_salt("techdraw_page_setup_template")
                .selected_text(selected.label())
                .show_ui(ui, |ui| {
                    for preset in TechDrawTemplatePreset::ALL {
                        ui.selectable_value(&mut selected, preset, preset.label());
                    }
                });
            if selected != state.preset {
                state.apply_preset(selected);
            }

            dialog_section(ui, "Sheet");
            egui::Grid::new("techdraw_page_setup_sheet")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Title:");
                    ui.text_edit_singleline(&mut state.title);
                    ui.end_row();

                    let width_changed =
                        param_field(ui, "Width", &mut state.width, 1.0..=2000.0, 1.0, "");
                    let height_changed =
                        param_field(ui, "Height", &mut state.height, 1.0..=2000.0, 1.0, "");
                    if width_changed || height_changed {
                        state.preset = TechDrawTemplatePreset::Custom;
                    }
                });

            let (ok, cancel, _) = button_bar(ui, "Apply");
            do_ok = ok;
            do_cancel = cancel;
        });
    if !open || do_cancel {
        gui.close_active_dialog();
    } else if do_ok {
        gui.actions
            .push(GuiAction::TechDraw(super::TechDrawAction::CommitPageSetup));
    }
}

// ---------------------------------------------------------------------------
// TechDraw: dimension setup dialog
// ---------------------------------------------------------------------------

pub(crate) fn draw_techdraw_dimension_setup_dialog(ctx: &egui::Context, gui: &mut GuiState) {
    let (sheet_width, sheet_height) = gui
        .techdraw_sheet
        .as_ref()
        .map(|s| (s.width, s.height))
        .unwrap_or((297.0, 210.0));
    let state = match &mut gui.active_dialog {
        Some(crate::gui::ActiveDialog::TechDrawDimensionSetup(s)) => s,
        _ => return,
    };
    let mut open = true;
    let (mut do_ok, mut do_cancel) = (false, false);
    egui::Window::new("TechDraw Dimension Setup")
        .collapsible(false)
        .resizable(false)
        .default_width(360.0)
        .open(&mut open)
        .show(ctx, |ui| {
            dialog_section(ui, "Dimension");
            let mut selected = state.kind;
            egui::ComboBox::from_id_salt("techdraw_dimension_setup_kind")
                .selected_text(selected.label())
                .show_ui(ui, |ui| {
                    for kind in TechDrawDimensionKind::ALL {
                        ui.selectable_value(&mut selected, kind, kind.label());
                    }
                });
            if selected != state.kind {
                state.apply_kind(selected, sheet_width, sheet_height);
            }

            dialog_section(ui, "Parameters");
            egui::Grid::new("techdraw_dimension_setup_fields")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| match state.kind {
                    TechDrawDimensionKind::Linear => {
                        ui.label("Text:");
                        ui.text_edit_singleline(&mut state.label);
                        ui.end_row();
                        param_field(ui, "X1", &mut state.x1, 0.0..=sheet_width, 1.0, "");
                        param_field(ui, "Y1", &mut state.y1, 0.0..=sheet_height, 1.0, "");
                        param_field(ui, "X2", &mut state.x2, 0.0..=sheet_width, 1.0, "");
                        param_field(ui, "Y2", &mut state.y2, 0.0..=sheet_height, 1.0, "");
                        param_field(ui, "Offset", &mut state.offset, -200.0..=200.0, 1.0, "");
                    }
                    TechDrawDimensionKind::Radius => {
                        ui.label("Text:");
                        ui.text_edit_singleline(&mut state.label);
                        ui.end_row();
                        param_field(
                            ui,
                            "Center X",
                            &mut state.center_x,
                            0.0..=sheet_width,
                            1.0,
                            "",
                        );
                        param_field(
                            ui,
                            "Center Y",
                            &mut state.center_y,
                            0.0..=sheet_height,
                            1.0,
                            "",
                        );
                        param_field(ui, "Visual radius", &mut state.radius, 1.0..=500.0, 1.0, "");
                        param_field_no_unit(
                            ui,
                            "Leader angle",
                            &mut state.value,
                            -360.0..=360.0,
                            1.0,
                        );
                    }
                    TechDrawDimensionKind::Diameter => {
                        param_field(
                            ui,
                            "Center X",
                            &mut state.center_x,
                            0.0..=sheet_width,
                            1.0,
                            "",
                        );
                        param_field(
                            ui,
                            "Center Y",
                            &mut state.center_y,
                            0.0..=sheet_height,
                            1.0,
                            "",
                        );
                        param_field(ui, "Diameter", &mut state.value, 0.001..=2000.0, 1.0, "");
                    }
                    TechDrawDimensionKind::Angle => {
                        param_field(
                            ui,
                            "Vertex X",
                            &mut state.center_x,
                            0.0..=sheet_width,
                            1.0,
                            "",
                        );
                        param_field(
                            ui,
                            "Vertex Y",
                            &mut state.center_y,
                            0.0..=sheet_height,
                            1.0,
                            "",
                        );
                        param_field(ui, "Arm length", &mut state.radius, 1.0..=500.0, 1.0, "");
                        param_field_no_unit(ui, "Angle deg", &mut state.value, 0.0..=360.0, 1.0);
                    }
                    TechDrawDimensionKind::ArcLength => {
                        param_field(
                            ui,
                            "Center X",
                            &mut state.center_x,
                            0.0..=sheet_width,
                            1.0,
                            "",
                        );
                        param_field(
                            ui,
                            "Center Y",
                            &mut state.center_y,
                            0.0..=sheet_height,
                            1.0,
                            "",
                        );
                        param_field(ui, "Radius", &mut state.radius, 1.0..=500.0, 1.0, "");
                        param_field_no_unit(
                            ui,
                            "Start deg",
                            &mut state.start_angle,
                            -360.0..=360.0,
                            1.0,
                        );
                        param_field_no_unit(
                            ui,
                            "End deg",
                            &mut state.end_angle,
                            -360.0..=360.0,
                            1.0,
                        );
                    }
                    TechDrawDimensionKind::Area => {
                        param_field(
                            ui,
                            "Label X",
                            &mut state.center_x,
                            0.0..=sheet_width,
                            1.0,
                            "",
                        );
                        param_field(
                            ui,
                            "Label Y",
                            &mut state.center_y,
                            0.0..=sheet_height,
                            1.0,
                            "",
                        );
                        param_field(
                            ui,
                            "Box width",
                            &mut state.area_width,
                            1.0..=1000.0,
                            1.0,
                            "",
                        );
                        param_field(
                            ui,
                            "Box height",
                            &mut state.area_height,
                            1.0..=1000.0,
                            1.0,
                            "",
                        );
                        param_field(ui, "Area value", &mut state.value, 0.0..=1.0e9, 1.0, "");
                    }
                });

            let (ok, cancel, _) = button_bar(ui, "Add");
            do_ok = ok;
            do_cancel = cancel;
        });
    if !open || do_cancel {
        gui.close_active_dialog();
    } else if do_ok {
        gui.actions.push(GuiAction::TechDraw(
            super::TechDrawAction::CommitDimensionSetup,
        ));
    }
}

// ---------------------------------------------------------------------------
// TechDraw: annotation setup dialog
// ---------------------------------------------------------------------------

pub(crate) fn draw_techdraw_annotation_setup_dialog(ctx: &egui::Context, gui: &mut GuiState) {
    let (sheet_width, sheet_height) = gui
        .techdraw_sheet
        .as_ref()
        .map(|s| (s.width, s.height))
        .unwrap_or((297.0, 210.0));
    let state = match &mut gui.active_dialog {
        Some(crate::gui::ActiveDialog::TechDrawAnnotationSetup(s)) => s,
        _ => return,
    };
    let mut open = true;
    let (mut do_ok, mut do_cancel) = (false, false);
    egui::Window::new("TechDraw Annotation Setup")
        .collapsible(false)
        .resizable(false)
        .default_width(360.0)
        .open(&mut open)
        .show(ctx, |ui| {
            dialog_section(ui, "Annotation");
            let mut selected = state.kind;
            egui::ComboBox::from_id_salt("techdraw_annotation_setup_kind")
                .selected_text(selected.label())
                .show_ui(ui, |ui| {
                    for kind in TechDrawAnnotationKind::ALL {
                        ui.selectable_value(&mut selected, kind, kind.label());
                    }
                });
            if selected != state.kind {
                state.apply_kind(selected, sheet_width, sheet_height);
            }

            dialog_section(ui, "Parameters");
            egui::Grid::new("techdraw_annotation_setup_fields")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| match state.kind {
                    TechDrawAnnotationKind::Text | TechDrawAnnotationKind::RichText => {
                        ui.label("Text:");
                        ui.text_edit_singleline(&mut state.text);
                        ui.end_row();
                        param_field(ui, "X", &mut state.x, 0.0..=sheet_width, 1.0, "");
                        param_field(ui, "Y", &mut state.y, 0.0..=sheet_height, 1.0, "");
                        param_field_no_unit(ui, "Font size", &mut state.font_size, 1.0..=72.0, 0.5);
                    }
                    TechDrawAnnotationKind::Balloon => {
                        ui.label("Number:");
                        ui.add(
                            egui::DragValue::new(&mut state.number)
                                .range(1..=999)
                                .speed(1),
                        );
                        ui.end_row();
                        param_field(ui, "Leader X", &mut state.x, 0.0..=sheet_width, 1.0, "");
                        param_field(ui, "Leader Y", &mut state.y, 0.0..=sheet_height, 1.0, "");
                        param_field(
                            ui,
                            "Balloon X",
                            &mut state.end_x,
                            0.0..=sheet_width,
                            1.0,
                            "",
                        );
                        param_field(
                            ui,
                            "Balloon Y",
                            &mut state.end_y,
                            0.0..=sheet_height,
                            1.0,
                            "",
                        );
                        param_field(ui, "Radius", &mut state.radius, 1.0..=100.0, 1.0, "");
                    }
                    TechDrawAnnotationKind::Leader => {
                        ui.label("Text:");
                        ui.text_edit_singleline(&mut state.text);
                        ui.end_row();
                        param_field(ui, "Start X", &mut state.x, 0.0..=sheet_width, 1.0, "");
                        param_field(ui, "Start Y", &mut state.y, 0.0..=sheet_height, 1.0, "");
                        param_field(ui, "End X", &mut state.end_x, 0.0..=sheet_width, 1.0, "");
                        param_field(ui, "End Y", &mut state.end_y, 0.0..=sheet_height, 1.0, "");
                    }
                    TechDrawAnnotationKind::Weld => {
                        param_field(ui, "X", &mut state.x, 0.0..=sheet_width, 1.0, "");
                        param_field(ui, "Y", &mut state.y, 0.0..=sheet_height, 1.0, "");
                        param_field(ui, "Size", &mut state.weld_size, 0.1..=100.0, 0.5, "");
                        param_field(ui, "Length", &mut state.weld_length, 0.0..=1000.0, 1.0, "");
                    }
                    TechDrawAnnotationKind::SurfaceFinish => {
                        param_field(ui, "X", &mut state.x, 0.0..=sheet_width, 1.0, "");
                        param_field(ui, "Y", &mut state.y, 0.0..=sheet_height, 1.0, "");
                        param_field_no_unit(ui, "Ra", &mut state.roughness, 0.0..=1000.0, 0.1);
                    }
                });

            let (ok, cancel, _) = button_bar(ui, "Add");
            do_ok = ok;
            do_cancel = cancel;
        });
    if !open || do_cancel {
        gui.close_active_dialog();
    } else if do_ok {
        gui.actions.push(GuiAction::TechDraw(
            super::TechDrawAction::CommitAnnotationSetup,
        ));
    }
}

// ---------------------------------------------------------------------------
// TechDraw: centerline setup dialog
// ---------------------------------------------------------------------------

pub(crate) fn draw_techdraw_centerline_setup_dialog(ctx: &egui::Context, gui: &mut GuiState) {
    let (sheet_width, sheet_height) = gui
        .techdraw_sheet
        .as_ref()
        .map(|s| (s.width, s.height))
        .unwrap_or((297.0, 210.0));
    let state = match &mut gui.active_dialog {
        Some(crate::gui::ActiveDialog::TechDrawCenterlineSetup(s)) => s,
        _ => return,
    };
    let mut open = true;
    let (mut do_ok, mut do_cancel) = (false, false);
    egui::Window::new("TechDraw Centerline Setup")
        .collapsible(false)
        .resizable(false)
        .default_width(360.0)
        .open(&mut open)
        .show(ctx, |ui| {
            dialog_section(ui, "Centerline");
            let mut selected = state.kind;
            egui::ComboBox::from_id_salt("techdraw_centerline_setup_kind")
                .selected_text(selected.label())
                .show_ui(ui, |ui| {
                    for kind in TechDrawCenterlineKind::ALL {
                        ui.selectable_value(&mut selected, kind, kind.label());
                    }
                });
            if selected != state.kind {
                state.apply_kind(selected, sheet_width, sheet_height);
            }

            dialog_section(ui, "Parameters");
            egui::Grid::new("techdraw_centerline_setup_fields")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| match state.kind {
                    TechDrawCenterlineKind::Face => {
                        param_field(
                            ui,
                            "Center X",
                            &mut state.center_x,
                            0.0..=sheet_width,
                            1.0,
                            "",
                        );
                        param_field(
                            ui,
                            "Center Y",
                            &mut state.center_y,
                            0.0..=sheet_height,
                            1.0,
                            "",
                        );
                        param_field(ui, "Face width", &mut state.width, 1.0..=1000.0, 1.0, "");
                        param_field(ui, "Face height", &mut state.height, 1.0..=1000.0, 1.0, "");
                        param_field(ui, "Extension", &mut state.extension, 0.0..=100.0, 0.5, "");
                    }
                    TechDrawCenterlineKind::BetweenLines => {
                        ui.label("Orientation:");
                        ui.checkbox(&mut state.horizontal, "Horizontal");
                        ui.end_row();
                        param_field(
                            ui,
                            "Center X",
                            &mut state.center_x,
                            0.0..=sheet_width,
                            1.0,
                            "",
                        );
                        param_field(
                            ui,
                            "Center Y",
                            &mut state.center_y,
                            0.0..=sheet_height,
                            1.0,
                            "",
                        );
                        param_field(ui, "Span", &mut state.span, 1.0..=1000.0, 1.0, "");
                        param_field(ui, "Gap", &mut state.gap, 0.0..=500.0, 1.0, "");
                        param_field(ui, "Extension", &mut state.extension, 0.0..=100.0, 0.5, "");
                    }
                    TechDrawCenterlineKind::CenterMark => {
                        param_field(
                            ui,
                            "Center X",
                            &mut state.center_x,
                            0.0..=sheet_width,
                            1.0,
                            "",
                        );
                        param_field(
                            ui,
                            "Center Y",
                            &mut state.center_y,
                            0.0..=sheet_height,
                            1.0,
                            "",
                        );
                        param_field(ui, "Mark size", &mut state.mark_size, 1.0..=200.0, 1.0, "");
                    }
                    TechDrawCenterlineKind::BoltCircle => {
                        ui.label("Bolt count:");
                        ui.add(
                            egui::DragValue::new(&mut state.bolt_count)
                                .range(1..=128)
                                .speed(1),
                        );
                        ui.end_row();
                        param_field(
                            ui,
                            "Center X",
                            &mut state.center_x,
                            0.0..=sheet_width,
                            1.0,
                            "",
                        );
                        param_field(
                            ui,
                            "Center Y",
                            &mut state.center_y,
                            0.0..=sheet_height,
                            1.0,
                            "",
                        );
                        param_field(ui, "Radius", &mut state.radius, 1.0..=500.0, 1.0, "");
                        param_field(ui, "Mark size", &mut state.mark_size, 1.0..=200.0, 1.0, "");
                    }
                });

            let (ok, cancel, _) = button_bar(ui, "Add");
            do_ok = ok;
            do_cancel = cancel;
        });
    if !open || do_cancel {
        gui.close_active_dialog();
    } else if do_ok {
        gui.actions.push(GuiAction::TechDraw(
            super::TechDrawAction::CommitCenterlineSetup,
        ));
    }
}

// ---------------------------------------------------------------------------
// TechDraw: view setup dialog
// ---------------------------------------------------------------------------

pub(crate) fn draw_techdraw_view_setup_dialog(ctx: &egui::Context, gui: &mut GuiState) {
    let (sheet_width, sheet_height) = gui
        .techdraw_sheet
        .as_ref()
        .map(|s| (s.width, s.height))
        .unwrap_or((297.0, 210.0));
    let state = match &mut gui.active_dialog {
        Some(crate::gui::ActiveDialog::TechDrawViewSetup(s)) => s,
        _ => return,
    };
    let mut open = true;
    let (mut do_ok, mut do_cancel) = (false, false);
    egui::Window::new("TechDraw View Setup")
        .collapsible(false)
        .resizable(false)
        .default_width(380.0)
        .open(&mut open)
        .show(ctx, |ui| {
            dialog_section(ui, "View");
            let mut selected = state.kind;
            egui::ComboBox::from_id_salt("techdraw_view_setup_kind")
                .selected_text(selected.label())
                .show_ui(ui, |ui| {
                    for kind in TechDrawViewKind::ALL {
                        ui.selectable_value(&mut selected, kind, kind.label());
                    }
                });
            if selected != state.kind {
                state.apply_kind(selected, sheet_width, sheet_height);
            }

            dialog_section(ui, "Placement");
            egui::Grid::new("techdraw_view_setup_placement")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    param_field(
                        ui,
                        "Sheet X",
                        &mut state.sheet_x,
                        0.0..=sheet_width,
                        1.0,
                        "",
                    );
                    param_field(
                        ui,
                        "Sheet Y",
                        &mut state.sheet_y,
                        0.0..=sheet_height,
                        1.0,
                        "",
                    );
                    ui.label("Scale mode:");
                    ui.checkbox(&mut state.auto_scale, "Auto fit");
                    ui.end_row();
                    if !state.auto_scale {
                        param_field_no_unit(
                            ui,
                            "Sheet scale",
                            &mut state.sheet_scale,
                            0.001..=5000.0,
                            1.0,
                        );
                    }
                    if state.kind == TechDrawViewKind::ThreeView {
                        param_field(
                            ui,
                            "Right spacing",
                            &mut state.spacing_x,
                            0.0..=sheet_width,
                            1.0,
                            "",
                        );
                        param_field(
                            ui,
                            "Top spacing",
                            &mut state.spacing_y,
                            0.0..=sheet_height,
                            1.0,
                            "",
                        );
                    }
                });

            if matches!(
                state.kind,
                TechDrawViewKind::Section | TechDrawViewKind::Detail | TechDrawViewKind::Broken
            ) {
                dialog_section(ui, "View Parameters");
                egui::Grid::new("techdraw_view_setup_fields")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| match state.kind {
                        TechDrawViewKind::Section => {
                            ui.label("Label:");
                            ui.text_edit_singleline(&mut state.section_label);
                            ui.end_row();
                        }
                        TechDrawViewKind::Detail => {
                            param_field(
                                ui,
                                "Source X",
                                &mut state.detail_center_x,
                                -10000.0..=10000.0,
                                1.0,
                                "",
                            );
                            param_field(
                                ui,
                                "Source Y",
                                &mut state.detail_center_y,
                                -10000.0..=10000.0,
                                1.0,
                                "",
                            );
                            param_field(
                                ui,
                                "Detail radius",
                                &mut state.detail_radius,
                                0.001..=10000.0,
                                1.0,
                                "",
                            );
                            param_field_no_unit(
                                ui,
                                "Magnification",
                                &mut state.detail_magnification,
                                0.001..=1000.0,
                                0.1,
                            );
                        }
                        TechDrawViewKind::Broken => {
                            param_field(
                                ui,
                                "Break start",
                                &mut state.break_start,
                                -10000.0..=10000.0,
                                1.0,
                                "",
                            );
                            param_field(
                                ui,
                                "Break end",
                                &mut state.break_end,
                                -10000.0..=10000.0,
                                1.0,
                                "",
                            );
                            param_field(ui, "Gap", &mut state.break_gap, 0.0..=10000.0, 1.0, "");
                        }
                        TechDrawViewKind::Front
                        | TechDrawViewKind::Top
                        | TechDrawViewKind::Right
                        | TechDrawViewKind::Isometric
                        | TechDrawViewKind::ThreeView => {}
                    });
            }

            let (ok, cancel, _) = button_bar(ui, "Add View");
            do_ok = ok;
            do_cancel = cancel;
        });
    if !open || do_cancel {
        gui.close_active_dialog();
    } else if do_ok {
        gui.actions
            .push(GuiAction::TechDraw(super::TechDrawAction::CommitViewSetup));
    }
}

// ---------------------------------------------------------------------------
// Phase O-a — FEM material picker / BC editor dialogs
// ---------------------------------------------------------------------------

const MATERIAL_PRESETS: [MaterialPreset; 7] = [
    MaterialPreset::Steel,
    MaterialPreset::Aluminum,
    MaterialPreset::Titanium,
    MaterialPreset::Copper,
    MaterialPreset::Concrete,
    MaterialPreset::CastIron,
    MaterialPreset::Custom,
];

pub(crate) fn draw_material_picker_dialog(ctx: &egui::Context, gui: &mut GuiState) {
    let state = match &mut gui.active_dialog {
        Some(crate::gui::ActiveDialog::MaterialPicker(s)) => s,
        _ => return,
    };
    let mut open = true;
    let (mut do_ok, mut do_cancel) = (false, false);
    egui::Window::new("FEM Material")
        .collapsible(false)
        .resizable(false)
        .default_width(320.0)
        .open(&mut open)
        .show(ctx, |ui| {
            dialog_section(ui, "Preset");
            for p in MATERIAL_PRESETS {
                ui.radio_value(&mut state.selected, p, p.label());
                ui.label(
                    egui::RichText::new(p.description())
                        .size(10.0)
                        .color(theme::COLOR_DIM)
                        .italics(),
                );
            }
            if state.selected == MaterialPreset::Custom {
                dialog_section(ui, "Custom Properties");
                egui::Grid::new("mat_custom")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Young's Modulus (Pa):");
                        ui.add(
                            egui::DragValue::new(&mut state.youngs_modulus)
                                .range(1.0..=1.0e13)
                                .speed(1.0e8),
                        );
                        ui.end_row();
                        ui.label("Poisson Ratio:");
                        ui.add(
                            egui::DragValue::new(&mut state.poisson_ratio)
                                .range(0.0..=0.499)
                                .speed(0.01),
                        );
                        ui.end_row();
                        ui.label("Density (kg/m\u{00B3}):");
                        ui.add(
                            egui::DragValue::new(&mut state.density)
                                .range(1.0..=30000.0)
                                .speed(10.0),
                        );
                        ui.end_row();
                    });
            }
            let (ok, cancel, _) = button_bar(ui, "OK");
            do_ok = ok;
            do_cancel = cancel;
        });
    if !open || do_cancel {
        gui.close_active_dialog();
    } else if do_ok {
        gui.actions
            .push(GuiAction::Fem(super::FemAction::CommitMaterialPicker));
    }
}

pub(crate) fn draw_bc_editor_dialog(ctx: &egui::Context, gui: &mut GuiState) {
    // Snapshot mesh sizes before taking the mutable borrow on `active_dialog`.
    let n_nodes = gui
        .fem_analysis
        .as_ref()
        .map(|a| a.mesh.nodes.len())
        .unwrap_or(0);
    let n_elems = gui
        .fem_analysis
        .as_ref()
        .map(|a| a.mesh.elements.len())
        .unwrap_or(0);
    let max_node = n_nodes.saturating_sub(1);
    let max_elem = n_elems.saturating_sub(1);
    let state = match &mut gui.active_dialog {
        Some(crate::gui::ActiveDialog::BcEditor(s)) => s,
        _ => return,
    };
    let mut open = true;
    let (mut do_ok, mut do_cancel) = (false, false);
    const ALL_KINDS: &[BcKind] = &[
        BcKind::FixedNode,
        BcKind::Force,
        BcKind::Pressure,
        BcKind::Displacement,
        BcKind::Gravity,
        BcKind::DistributedLoad,
        BcKind::Spring,
        BcKind::CentrifugalLoad,
        BcKind::SelfWeight,
        BcKind::SpringConstraint,
        BcKind::BodyLoad,
        BcKind::InitialTemperature,
        BcKind::SectionPrint,
        BcKind::TieConstraint,
        BcKind::RigidBody,
        BcKind::ContactConstraint,
    ];
    egui::Window::new("FEM Boundary Condition")
        .collapsible(false)
        .resizable(false)
        .default_width(320.0)
        .open(&mut open)
        .show(ctx, |ui| {
            dialog_section(ui, "Type");
            egui::ComboBox::from_id_salt("bc_kind")
                .selected_text(state.bc_kind.label())
                .show_ui(ui, |ui| {
                    for &k in ALL_KINDS {
                        ui.selectable_value(&mut state.bc_kind, k, k.label());
                    }
                });
            let inputs = state.bc_kind.inputs();
            if inputs.node {
                dialog_section(ui, "Node");
                egui::Grid::new("bc_node_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Node Index:");
                        let range = if n_nodes == 0 {
                            0..=usize::MAX
                        } else {
                            0..=max_node
                        };
                        ui.add(egui::DragValue::new(&mut state.node_index).range(range));
                        ui.end_row();
                    });
            }
            if inputs.element {
                dialog_section(ui, "Element");
                egui::Grid::new("bc_elem_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Element Index:");
                        let range = if n_elems == 0 {
                            0..=usize::MAX
                        } else {
                            0..=max_elem
                        };
                        ui.add(egui::DragValue::new(&mut state.element_index).range(range));
                        ui.end_row();
                    });
            }
            if inputs.set_a || inputs.set_b {
                dialog_section(ui, "Node Sets");
                egui::Grid::new("bc_node_sets_grid")
                    .num_columns(3)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        if inputs.set_a {
                            ui.label("Set A:");
                            ui.add(egui::DragValue::new(&mut state.set_a_start).range(
                                if n_nodes == 0 {
                                    0..=usize::MAX
                                } else {
                                    0..=max_node
                                },
                            ));
                            ui.add(egui::DragValue::new(&mut state.set_a_end).range(
                                if n_nodes == 0 {
                                    0..=usize::MAX
                                } else {
                                    0..=max_node
                                },
                            ));
                            ui.end_row();
                        }
                        if inputs.set_b {
                            ui.label("Set B:");
                            ui.add(egui::DragValue::new(&mut state.set_b_start).range(
                                if n_nodes == 0 {
                                    0..=usize::MAX
                                } else {
                                    0..=max_node
                                },
                            ));
                            ui.add(egui::DragValue::new(&mut state.set_b_end).range(
                                if n_nodes == 0 {
                                    0..=usize::MAX
                                } else {
                                    0..=max_node
                                },
                            ));
                            ui.end_row();
                        }
                    });
                ui.label(
                    egui::RichText::new("Ranges are inclusive node-index spans.")
                        .size(10.0)
                        .color(super::theme::COLOR_DIM),
                );
            }
            if n_nodes == 0 {
                validation_error(ui, "No analysis \u{2014} create one first.");
            }
            if inputs.vec3 {
                dialog_section(ui, state.bc_kind.vec3_label());
                egui::Grid::new("bc_vec3_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        for (lab, v) in [
                            ("X:", &mut state.vec3_x),
                            ("Y:", &mut state.vec3_y),
                            ("Z:", &mut state.vec3_z),
                        ] {
                            ui.label(lab);
                            ui.add(egui::DragValue::new(v).speed(1.0));
                            ui.end_row();
                        }
                    });
            }
            if inputs.point {
                dialog_section(ui, "Plane Point");
                egui::Grid::new("bc_point_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        for (lab, v) in [
                            ("X:", &mut state.point_x),
                            ("Y:", &mut state.point_y),
                            ("Z:", &mut state.point_z),
                        ] {
                            ui.label(lab);
                            ui.add(egui::DragValue::new(v).speed(1.0));
                            ui.end_row();
                        }
                    });
            }
            if inputs.scalar {
                dialog_section(ui, "Magnitude");
                egui::Grid::new("bc_scalar_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(state.bc_kind.scalar_label());
                        ui.add(egui::DragValue::new(&mut state.scalar_a).speed(1.0));
                        ui.end_row();
                    });
            }
            let (ok, cancel, _) = button_bar(ui, "OK");
            do_ok = ok;
            do_cancel = cancel;
        });
    if !open || do_cancel {
        gui.close_active_dialog();
    } else if do_ok {
        gui.actions
            .push(GuiAction::Fem(super::FemAction::CommitBcEditor));
    }
}

pub(crate) fn draw_fem_result_probe_dialog(ctx: &egui::Context, gui: &mut GuiState) {
    let n_nodes = gui
        .fem_analysis
        .as_ref()
        .map(|a| a.mesh.nodes.len())
        .unwrap_or(0);
    let n_elems = gui
        .fem_analysis
        .as_ref()
        .map(|a| a.mesh.elements.len())
        .unwrap_or(0);
    let max_node = n_nodes.saturating_sub(1);
    let max_elem = n_elems.saturating_sub(1);
    let state = match &mut gui.active_dialog {
        Some(crate::gui::ActiveDialog::FemResultProbe(s)) => s,
        _ => return,
    };
    let mut open = true;
    let (mut do_ok, mut do_cancel) = (false, false);
    egui::Window::new("FEM Result Probe")
        .collapsible(false)
        .resizable(false)
        .default_width(320.0)
        .open(&mut open)
        .show(ctx, |ui| {
            dialog_section(ui, "Probe Target");
            egui::Grid::new("fem_probe_grid")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Node Index:");
                    let node_range = if n_nodes == 0 {
                        0..=usize::MAX
                    } else {
                        0..=max_node
                    };
                    ui.add(egui::DragValue::new(&mut state.node_index).range(node_range));
                    ui.end_row();

                    ui.label("Element Index:");
                    let elem_range = if n_elems == 0 {
                        0..=usize::MAX
                    } else {
                        0..=max_elem
                    };
                    ui.add(egui::DragValue::new(&mut state.element_index).range(elem_range));
                    ui.end_row();
                });
            if n_nodes == 0 {
                validation_error(ui, "No analysis — create one first.");
            } else {
                ui.label(
                    egui::RichText::new(format!(
                        "Mesh has {n_nodes} nodes and {n_elems} elements."
                    ))
                    .size(10.0)
                    .color(theme::COLOR_DIM),
                );
            }
            let (ok, cancel, _) = button_bar(ui, "Probe");
            do_ok = ok;
            do_cancel = cancel;
        });
    if !open || do_cancel {
        gui.close_active_dialog();
    } else if do_ok {
        gui.actions
            .push(GuiAction::Fem(super::FemAction::CommitResultProbe));
    }
}

pub(crate) fn draw_fem_result_table_dialog(ctx: &egui::Context, gui: &mut GuiState) {
    let state = match &mut gui.active_dialog {
        Some(crate::gui::ActiveDialog::FemResultTable(s)) => s,
        _ => return,
    };
    let mut open = true;
    egui::Window::new("FEM Result Table")
        .resizable(true)
        .default_width(680.0)
        .default_height(460.0)
        .open(&mut open)
        .show(ctx, |ui| {
            dialog_section(ui, "Result Summary");
            ui.label(
                egui::RichText::new(format!(
                    "{} table — {} nodes, {} elements",
                    state.field.label(),
                    state.node_rows.len(),
                    state.element_rows.len()
                ))
                .color(theme::COLOR_ACCENT),
            );

            dialog_section(ui, "Node Results");
            egui::ScrollArea::vertical()
                .id_salt("fem_node_result_table")
                .max_height(190.0)
                .show(ui, |ui| {
                    egui::Grid::new("fem_node_result_grid")
                        .striped(true)
                        .num_columns(6)
                        .spacing([12.0, 3.0])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Node").strong());
                            ui.label(egui::RichText::new("X").strong());
                            ui.label(egui::RichText::new("Y").strong());
                            ui.label(egui::RichText::new("Z").strong());
                            ui.label(egui::RichText::new("|u| (m)").strong());
                            ui.label(egui::RichText::new("T (K)").strong());
                            ui.end_row();
                            for row in state.node_rows.iter().take(200) {
                                ui.label(row.index.to_string());
                                ui.label(format!("{:.3}", row.position[0]));
                                ui.label(format!("{:.3}", row.position[1]));
                                ui.label(format!("{:.3}", row.position[2]));
                                ui.label(format_optional(row.displacement_magnitude));
                                ui.label(format_optional(row.temperature));
                                ui.end_row();
                            }
                        });
                });
            if state.node_rows.len() > 200 {
                ui.label(
                    egui::RichText::new(format!(
                        "Showing first 200 of {} node rows.",
                        state.node_rows.len()
                    ))
                    .size(10.0)
                    .color(theme::COLOR_DIM),
                );
            }

            dialog_section(ui, "Element Results");
            egui::ScrollArea::vertical()
                .id_salt("fem_element_result_table")
                .max_height(150.0)
                .show(ui, |ui| {
                    egui::Grid::new("fem_element_result_grid")
                        .striped(true)
                        .num_columns(4)
                        .spacing([12.0, 3.0])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Element").strong());
                            ui.label(egui::RichText::new("Nodes").strong());
                            ui.label(egui::RichText::new("σ / Von Mises (Pa)").strong());
                            ui.label(egui::RichText::new("Field").strong());
                            ui.end_row();
                            for row in state.element_rows.iter().take(200) {
                                ui.label(row.index.to_string());
                                ui.label(format!(
                                    "{}, {}, {}, {}",
                                    row.nodes[0], row.nodes[1], row.nodes[2], row.nodes[3]
                                ));
                                ui.label(format_optional(row.stress));
                                ui.label(state.field.label());
                                ui.end_row();
                            }
                        });
                });
            if state.element_rows.len() > 200 {
                ui.label(
                    egui::RichText::new(format!(
                        "Showing first 200 of {} element rows.",
                        state.element_rows.len()
                    ))
                    .size(10.0)
                    .color(theme::COLOR_DIM),
                );
            }
        });
    if !open {
        gui.close_active_dialog();
    }
}

fn format_optional(value: Option<f64>) -> String {
    value
        .map(|v| format!("{v:.3e}"))
        .unwrap_or_else(|| "—".into())
}

// ---------------------------------------------------------------------------
// A3.1 — Autosave recovery dialog
// ---------------------------------------------------------------------------

/// Format a `SystemTime` as `YYYY-MM-DD HH:MM:SS` in UTC. Hand-rolled
/// because `chrono` is not in the workspace dep set and a one-call
/// formatter is not worth a new dependency.
fn format_modified(t: std::time::SystemTime) -> String {
    let secs = t
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64;
    // Days since 1970-01-01 (UTC), and seconds within the day.
    let days = secs.div_euclid(86_400);
    let day_secs = secs.rem_euclid(86_400) as u64;
    let hour = day_secs / 3_600;
    let minute = (day_secs % 3_600) / 60;
    let second = day_secs % 60;

    // Civil-from-days (Howard Hinnant's algorithm — public-domain).
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year, m, d, hour, minute, second
    )
}

/// Render the autosave recovery prompt when
/// `gui.active_dialog == Some(ActiveDialog::AutosaveRecovery(_))`.
/// Buttons write their choice into `gui.autosave_recovery_choice`;
/// the dispatcher (`CadApp::process_actions`) consumes it once and
/// closes the dialog.
pub(crate) fn draw_autosave_recovery_dialog(ctx: &egui::Context, gui: &mut GuiState) {
    let Some(super::ActiveDialog::AutosaveRecovery(entry)) = gui.active_dialog.clone() else {
        return;
    };
    let when = format_modified(entry.modified);
    let size_kb = (entry.size_bytes as f64) / 1024.0;
    egui::Window::new("Recover Autosave")
        .collapsible(false)
        .resizable(false)
        .default_width(420.0)
        .show(ctx, |ui| {
            ui.label(format!("Found autosave from {when}. Recover?"));
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(format!("{} ({:.1} KiB)", entry.path.display(), size_kb))
                    .size(11.0)
                    .color(super::theme::COLOR_DIM),
            );
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Recover").clicked() {
                    gui.autosave_recovery_choice = Some(super::AutosaveRecoveryChoice::Recover);
                }
                if ui.button("Discard").clicked() {
                    gui.autosave_recovery_choice = Some(super::AutosaveRecoveryChoice::Discard);
                }
                if ui.button("Cancel").clicked() {
                    gui.autosave_recovery_choice = Some(super::AutosaveRecoveryChoice::Cancel);
                }
            });
        });
}
