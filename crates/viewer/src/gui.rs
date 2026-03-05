//! egui-based UI panels for the CAD application.

use crate::nav::{NavConfig, NavStyle};
use crate::render::{
    Camera, DisplayMode, GridConfig, Projection, StandardView, dot3, normalize3, sub3,
};
use cadkernel_io::Mesh;
use cadkernel_modeling::{MassProperties, compute_mass_properties};
use cadkernel_topology::BRepModel;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Actions emitted by the GUI — processed by the application after each frame.
// ---------------------------------------------------------------------------

pub(crate) enum GuiAction {
    NewModel,
    OpenFile(PathBuf),
    SaveFile(PathBuf),
    ImportFile(PathBuf),
    ExportStl(PathBuf),
    ExportObj(PathBuf),
    ExportGltf(PathBuf),
    CreateBox {
        width: f64,
        height: f64,
        depth: f64,
    },
    CreateCylinder {
        radius: f64,
        height: f64,
    },
    CreateSphere {
        radius: f64,
    },
    ResetCamera,
    FitAll,
    ToggleProjection,
    SetDisplayMode(DisplayMode),
    SetStandardView(StandardView),
    SetCameraYawPitch(f32, f32),
    /// Screen-relative orbit: (right_angle, up_angle).
    ScreenOrbit(f32, f32),
    /// In-plane roll delta (positive = CW as seen by user).
    RollDelta(f32),
    ToggleGrid,
}

// ---------------------------------------------------------------------------
// GUI state persisted across frames.
// ---------------------------------------------------------------------------

pub(crate) struct GuiState {
    pub show_model_tree: bool,
    pub show_properties: bool,
    pub show_about: bool,
    pub show_settings: bool,
    pub show_create_box: bool,
    pub show_create_cylinder: bool,
    pub show_create_sphere: bool,
    pub status_message: String,
    pub current_file: Option<String>,

    pub create_box_size: [f64; 3],
    pub create_cylinder_radius: f64,
    pub create_cylinder_height: f64,
    pub create_sphere_radius: f64,

    pub actions: Vec<GuiAction>,
    pub request_quit: bool,
    pub show_view_menu: bool,

    cached_props: Option<MassProperties>,
    cached_props_tri_count: usize,
}

impl GuiState {
    pub fn new() -> Self {
        Self {
            show_model_tree: true,
            show_properties: true,
            show_about: false,
            show_settings: false,
            show_create_box: false,
            show_create_cylinder: false,
            show_create_sphere: false,
            status_message: "Ready".into(),
            current_file: None,
            create_box_size: [10.0, 10.0, 10.0],
            create_cylinder_radius: 5.0,
            create_cylinder_height: 10.0,
            create_sphere_radius: 5.0,
            actions: Vec::new(),
            request_quit: false,
            show_view_menu: false,
            cached_props: None,
            cached_props_tri_count: 0,
        }
    }

    pub fn invalidate_cache(&mut self) {
        self.cached_props = None;
        self.cached_props_tri_count = 0;
    }
}

// ---------------------------------------------------------------------------
// Top-level draw entry
// ---------------------------------------------------------------------------

/// Snapshot of the current viewport state, passed to `draw_ui` to avoid
/// excessive parameter counts.
pub(crate) struct ViewportInfo<'a> {
    pub camera: &'a Camera,
    pub display_mode: DisplayMode,
    pub grid_config: &'a GridConfig,
    pub show_grid: bool,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_ui(
    ctx: &egui::Context,
    gui: &mut GuiState,
    nav: &mut NavConfig,
    vp: &ViewportInfo<'_>,
    model: &BRepModel,
    mesh: &Option<Mesh>,
) {
    draw_menu_bar(ctx, gui, vp.camera, vp.display_mode);
    draw_model_tree(ctx, gui, model);
    draw_properties(ctx, gui, mesh);
    draw_status_bar(ctx, gui, vp.camera, vp.display_mode, mesh);
    draw_create_dialogs(ctx, gui);
    draw_about_dialog(ctx, gui);
    draw_settings(ctx, gui, nav);
    if nav.show_view_cube {
        draw_view_cube(ctx, vp.camera, gui);
    }
    if nav.show_axes_indicator {
        draw_axes_overlay(ctx, vp.camera);
    }
    if vp.show_grid {
        draw_grid_scale_label(ctx, vp.grid_config);
    }
}

// ---------------------------------------------------------------------------
// Menu bar
// ---------------------------------------------------------------------------

fn draw_menu_bar(
    ctx: &egui::Context,
    gui: &mut GuiState,
    camera: &Camera,
    display_mode: DisplayMode,
) {
    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
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

                ui.separator();

                if ui.button("Import STL…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("STL", &["stl"])
                        .pick_file()
                    {
                        gui.status_message = format!("Importing {}", path.display());
                        gui.actions.push(GuiAction::ImportFile(path));
                    }
                    ui.close_menu();
                }
                if ui.button("Import OBJ…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("OBJ", &["obj"])
                        .pick_file()
                    {
                        gui.status_message = format!("Importing {}", path.display());
                        gui.actions.push(GuiAction::ImportFile(path));
                    }
                    ui.close_menu();
                }

                ui.separator();

                if ui.button("Export STL…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("STL", &["stl"])
                        .set_file_name("model.stl")
                        .save_file()
                    {
                        gui.actions.push(GuiAction::ExportStl(path));
                    }
                    ui.close_menu();
                }
                if ui.button("Export OBJ…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("OBJ", &["obj"])
                        .set_file_name("model.obj")
                        .save_file()
                    {
                        gui.actions.push(GuiAction::ExportObj(path));
                    }
                    ui.close_menu();
                }
                if ui.button("Export glTF…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("glTF", &["gltf"])
                        .set_file_name("model.gltf")
                        .save_file()
                    {
                        gui.actions.push(GuiAction::ExportGltf(path));
                    }
                    ui.close_menu();
                }

                ui.separator();

                if ui.button("Quit").clicked() {
                    gui.request_quit = true;
                }
            });

            ui.menu_button("Edit", |ui| {
                if ui.button("Settings…").clicked() {
                    gui.show_settings = true;
                    ui.close_menu();
                }
            });

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
            });

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

            ui.menu_button("View", |ui| {
                ui.checkbox(&mut gui.show_model_tree, "Model Tree");
                ui.checkbox(&mut gui.show_properties, "Properties");
                ui.separator();

                ui.label("  Display Mode");
                for &mode in DisplayMode::ALL {
                    let selected = mode == display_mode;
                    let text = format!("    {}    {}", mode.label(), mode.shortcut());
                    if ui.selectable_label(selected, text).clicked() {
                        gui.actions.push(GuiAction::SetDisplayMode(mode));
                        ui.close_menu();
                    }
                }

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

            ui.menu_button("Help", |ui| {
                if ui.button("About CADKernel").clicked() {
                    gui.show_about = true;
                    ui.close_menu();
                }
            });
        });
    });
}

// ---------------------------------------------------------------------------
// Model tree (left panel)
// ---------------------------------------------------------------------------

fn draw_model_tree(ctx: &egui::Context, gui: &mut GuiState, model: &BRepModel) {
    if !gui.show_model_tree {
        return;
    }
    egui::SidePanel::left("model_tree")
        .default_width(200.0)
        .show(ctx, |ui| {
            ui.heading("Model Tree");
            ui.separator();

            ui.label(format!("Vertices: {}", model.vertices.len()));
            ui.label(format!("Edges:    {}", model.edges.len()));
            ui.label(format!("Faces:    {}", model.faces.len()));
            ui.label(format!("Solids:   {}", model.solids.len()));

            ui.separator();

            egui::CollapsingHeader::new("Solids")
                .default_open(true)
                .show(ui, |ui| {
                    if model.solids.is_empty() {
                        ui.weak("(empty)");
                    } else {
                        for (handle, _data) in model.solids.iter() {
                            ui.label(format!("Solid #{:?}", handle));
                        }
                    }
                });

            egui::CollapsingHeader::new("Faces")
                .default_open(false)
                .show(ui, |ui| {
                    if model.faces.is_empty() {
                        ui.weak("(empty)");
                    } else {
                        for (handle, _data) in model.faces.iter() {
                            ui.label(format!("Face #{:?}", handle));
                        }
                    }
                });
        });
}

// ---------------------------------------------------------------------------
// Properties panel (right side)
// ---------------------------------------------------------------------------

fn draw_properties(ctx: &egui::Context, gui: &mut GuiState, mesh: &Option<Mesh>) {
    if !gui.show_properties {
        return;
    }
    egui::SidePanel::right("properties")
        .default_width(250.0)
        .show(ctx, |ui| {
            ui.heading("Properties");
            ui.separator();

            if let Some(mesh) = mesh {
                ui.label(format!("Mesh vertices:  {}", mesh.vertices.len()));
                ui.label(format!("Mesh triangles: {}", mesh.triangle_count()));

                ui.separator();
                ui.strong("Mass Properties");

                let tri_count = mesh.triangle_count();
                if gui.cached_props.is_none() || gui.cached_props_tri_count != tri_count {
                    gui.cached_props = Some(compute_mass_properties(mesh));
                    gui.cached_props_tri_count = tri_count;
                }
                if let Some(props) = &gui.cached_props {
                    ui.label(format!("Volume:       {:.4}", props.volume));
                    ui.label(format!("Surface area: {:.4}", props.surface_area));
                    ui.label(format!(
                        "Centroid:     ({:.2}, {:.2}, {:.2})",
                        props.centroid.x, props.centroid.y, props.centroid.z
                    ));
                }
            } else {
                ui.weak("No model loaded");
                ui.separator();
                ui.weak("Use File > Open or Create menu");
            }

            if let Some(path) = &gui.current_file {
                ui.separator();
                ui.strong("File");
                ui.label(path.as_str());
            }
        });
}

// ---------------------------------------------------------------------------
// Status bar (bottom)
// ---------------------------------------------------------------------------

fn draw_status_bar(
    ctx: &egui::Context,
    gui: &GuiState,
    camera: &Camera,
    display_mode: DisplayMode,
    mesh: &Option<Mesh>,
) {
    egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label(&gui.status_message);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let proj_tag = match camera.projection {
                    Projection::Perspective => "Persp",
                    Projection::Orthographic => "Ortho",
                };
                let dm_tag = display_mode.label();
                if let Some(mesh) = mesh {
                    ui.label(format!(
                        "V: {} | T: {} | {} | {}",
                        mesh.vertices.len(),
                        mesh.triangle_count(),
                        dm_tag,
                        proj_tag,
                    ));
                } else {
                    ui.label(format!("{dm_tag} | {proj_tag}"));
                }
            });
        });
    });
}

// ---------------------------------------------------------------------------
// Create dialogs (floating windows)
// ---------------------------------------------------------------------------

fn draw_create_dialogs(ctx: &egui::Context, gui: &mut GuiState) {
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
}

// ---------------------------------------------------------------------------
// About dialog
// ---------------------------------------------------------------------------

fn draw_about_dialog(ctx: &egui::Context, gui: &mut GuiState) {
    let mut show = gui.show_about;
    if show {
        egui::Window::new("About CADKernel")
            .collapsible(false)
            .resizable(false)
            .open(&mut show)
            .show(ctx, |ui| {
                ui.heading("CADKernel");
                ui.label("A modular B-Rep CAD kernel written in Rust.");
                ui.separator();
                ui.label("Version: 0.1.0 (pre-alpha)");
                ui.label("License: Apache-2.0");
                ui.hyperlink_to("GitHub", "https://github.com/kernalix7/CADKernel");
            });
    }
    gui.show_about = show;
}

// ---------------------------------------------------------------------------
// Settings dialog
// ---------------------------------------------------------------------------

fn draw_settings(ctx: &egui::Context, gui: &mut GuiState, nav: &mut NavConfig) {
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
                // ============================================================
                // 3D View
                // ============================================================
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

                // ============================================================
                // Navigation
                // ============================================================
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

                // ============================================================
                // Lighting
                // ============================================================
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

// ---------------------------------------------------------------------------
// Mini axes overlay (bottom-left corner)
// ---------------------------------------------------------------------------

fn draw_axes_overlay(ctx: &egui::Context, camera: &Camera) {
    let size = 45.0f32;
    let margin = 55.0f32;
    let center = egui::pos2(margin, ctx.screen_rect().bottom() - margin);

    // Roll-aware camera axes.
    let right = camera.screen_right();
    let up = camera.screen_up();

    let axes: [([f32; 3], egui::Color32, &str); 3] = [
        ([1.0, 0.0, 0.0], egui::Color32::from_rgb(200, 60, 60), "X"),
        ([0.0, 1.0, 0.0], egui::Color32::from_rgb(60, 200, 60), "Y"),
        ([0.0, 0.0, 1.0], egui::Color32::from_rgb(80, 80, 230), "Z"),
    ];

    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("axes_overlay"),
    ));

    painter.circle_filled(center, 3.0, egui::Color32::from_gray(120));

    for (axis, color, label) in &axes {
        let sx = dot3(*axis, right) * size;
        let sy = -dot3(*axis, up) * size;
        let end = egui::pos2(center.x + sx, center.y + sy);
        // Negative direction (faded, thinner)
        let neg_end = egui::pos2(center.x - sx, center.y - sy);
        let faded = egui::Color32::from_rgba_premultiplied(
            color.r() / 3,
            color.g() / 3,
            color.b() / 3,
            100,
        );
        painter.line_segment([center, neg_end], egui::Stroke::new(1.0, faded));
        // Positive direction
        painter.line_segment([center, end], egui::Stroke::new(2.0, *color));
        painter.text(
            end,
            egui::Align2::CENTER_CENTER,
            *label,
            egui::FontId::proportional(11.0),
            *color,
        );
    }
}

// ---------------------------------------------------------------------------
// Grid scale label (bottom-center, above status bar)
// ---------------------------------------------------------------------------

fn draw_grid_scale_label(ctx: &egui::Context, grid_config: &GridConfig) {
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("grid_scale_label"),
    ));
    let viewport = ctx.available_rect();
    let label = grid_config.scale_label();
    let pos = egui::pos2(viewport.center().x, viewport.bottom() - 12.0);
    painter.text(
        pos,
        egui::Align2::CENTER_CENTER,
        format!("Grid: {label}"),
        egui::FontId::proportional(11.0),
        egui::Color32::from_rgba_premultiplied(140, 140, 140, 180),
    );
}

// ---------------------------------------------------------------------------
// ViewCube (3D rounded cube, top-right corner)
// ---------------------------------------------------------------------------

use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};
const FRAC_3PI_4: f32 = PI * 0.75;
const CORNER_PITCH: f32 = 0.6155; // arcsin(1/√3) ≈ 35.26°
const CHAMFER: f32 = 0.22; // vertex truncation ratio (corners)
const EDGE_BEVEL: f32 = 0.24; // edge bevel strip width

/// Generates 24 chamfer vertices for a truncated unit cube.
/// Each of the 8 cube corners produces 3 vertices (one per incident edge).
/// Layout: indices [3*i .. 3*i+2] belong to corner i.
fn build_chamfer_verts() -> [[f32; 3]; 24] {
    let s = 1.0f32;
    let c = s - 2.0 * CHAMFER; // inset coordinate
    [
        [-c, -s, -s],
        [-s, -c, -s],
        [-s, -s, -c], //  0- 2: v0
        [c, -s, -s],
        [s, -c, -s],
        [s, -s, -c], //  3- 5: v1
        [s, c, -s],
        [c, s, -s],
        [s, s, -c], //  6- 8: v2
        [-c, s, -s],
        [-s, c, -s],
        [-s, s, -c], //  9-11: v3
        [-c, -s, s],
        [-s, -c, s],
        [-s, -s, c], // 12-14: v4
        [c, -s, s],
        [s, -c, s],
        [s, -s, c], // 15-17: v5
        [s, c, s],
        [c, s, s],
        [s, s, c], // 18-20: v6
        [-c, s, s],
        [-s, c, s],
        [-s, s, c], // 21-23: v7
    ]
}

// Face octagon vertex indices (into the 24-element chamfer array).
// Z-up: +Y=FRONT, -Y=BACK, +X=RIGHT, -X=LEFT, +Z=TOP, -Z=BOTTOM
const FACE_OCTAGONS: [[usize; 8]; 6] = [
    [21, 19, 20, 8, 7, 9, 11, 23],    // FRONT  +Y
    [0, 3, 5, 17, 15, 12, 14, 2],     // BACK   -Y
    [17, 5, 4, 6, 8, 20, 18, 16],     // RIGHT  +X
    [2, 14, 13, 22, 23, 11, 10, 1],   // LEFT   -X
    [12, 15, 16, 18, 19, 21, 22, 13], // TOP    +Z
    [3, 0, 1, 10, 9, 7, 6, 4],        // BOTTOM -Z
];
const FACE_NORMALS: [[f32; 3]; 6] = [
    [0.0, 1.0, 0.0],
    [0.0, -1.0, 0.0],
    [1.0, 0.0, 0.0],
    [-1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0],
    [0.0, 0.0, -1.0],
];
const FACE_LABELS: [&str; 6] = ["FRONT", "BACK", "RIGHT", "LEFT", "TOP", "BOTTOM"];
/// World-space "text right" direction for each face (baseline direction).
const FACE_TEXT_RIGHT: [[f32; 3]; 6] = [
    [-1.0, 0.0, 0.0], // FRONT  — screen_right = cross3((0,-1,0),(0,0,1)) = −X
    [1.0, 0.0, 0.0],  // BACK   — screen_right = cross3((0,+1,0),(0,0,1)) = +X
    [0.0, 1.0, 0.0],  // RIGHT  — screen_right = cross3((-1,0,0),(0,0,1)) = +Y
    [0.0, -1.0, 0.0], // LEFT   — screen_right = cross3((+1,0,0),(0,0,1)) = −Y
    [-1.0, 0.0, 0.0], // TOP    — screen_right ≈ −X (looking down)
    [-1.0, 0.0, 0.0], // BOTTOM — screen_right ≈ −X (looking up)
];
const FACE_VIEWS: [StandardView; 6] = [
    StandardView::Front,
    StandardView::Back,
    StandardView::Right,
    StandardView::Left,
    StandardView::Top,
    StandardView::Bottom,
];

// Corner triangle vertex indices (CCW winding from outside).
const CORNER_TRIS: [[usize; 3]; 8] = [
    [0, 2, 1],    // v0 (-,-,-)
    [3, 4, 5],    // v1 (+,-,-)
    [6, 7, 8],    // v2 (+,+,-)
    [9, 10, 11],  // v3 (-,+,-)
    [12, 13, 14], // v4 (-,-,+)
    [15, 17, 16], // v5 (+,-,+)
    [18, 20, 19], // v6 (+,+,+)
    [21, 23, 22], // v7 (-,+,+)
];
const CORNER_NORMALS: [[f32; 3]; 8] = [
    [-1.0, -1.0, -1.0],
    [1.0, -1.0, -1.0],
    [1.0, 1.0, -1.0],
    [-1.0, 1.0, -1.0],
    [-1.0, -1.0, 1.0],
    [1.0, -1.0, 1.0],
    [1.0, 1.0, 1.0],
    [-1.0, 1.0, 1.0],
];
// Z-up corner views: pitch from Z component, yaw from X/Y.
const CORNER_YAW_PITCH: [[f32; 2]; 8] = [
    [-FRAC_3PI_4, -CORNER_PITCH],
    [-FRAC_PI_4, -CORNER_PITCH],
    [FRAC_PI_4, -CORNER_PITCH],
    [FRAC_3PI_4, -CORNER_PITCH],
    [-FRAC_3PI_4, CORNER_PITCH],
    [-FRAC_PI_4, CORNER_PITCH],
    [FRAC_PI_4, CORNER_PITCH],
    [FRAC_3PI_4, CORNER_PITCH],
];

// Shared edge segments between adjacent octagons (chamfer vertex index pairs).
const EDGE_SEGS: [[usize; 2]; 12] = [
    [0, 3],
    [4, 6],
    [7, 9],
    [10, 1], // back edges
    [12, 15],
    [16, 18],
    [19, 21],
    [22, 13], // front edges
    [2, 14],
    [5, 17],
    [8, 20],
    [11, 23], // depth edges
];
// Z-up edge views: pitch from Z component, yaw from X/Y.
const EDGE_YAW_PITCH: [[f32; 2]; 12] = [
    [-FRAC_PI_2, -FRAC_PI_4],
    [0.0, -FRAC_PI_4],
    [FRAC_PI_2, -FRAC_PI_4],
    [PI, -FRAC_PI_4],
    [-FRAC_PI_2, FRAC_PI_4],
    [0.0, FRAC_PI_4],
    [FRAC_PI_2, FRAC_PI_4],
    [PI, FRAC_PI_4],
    [-FRAC_3PI_4, 0.0],
    [-FRAC_PI_4, 0.0],
    [FRAC_PI_4, 0.0],
    [FRAC_3PI_4, 0.0],
];
// Adjacent face indices for each edge (for front-face visibility checks).
// Face order: 0=FRONT(+Y), 1=BACK(-Y), 2=RIGHT(+X), 3=LEFT(-X), 4=TOP(+Z), 5=BOTTOM(-Z).
const EDGE_ADJ_FACES: [[usize; 2]; 12] = [
    [5, 1],
    [5, 2],
    [5, 0],
    [5, 3], // bottom(-Z) edges
    [4, 1],
    [4, 2],
    [4, 0],
    [4, 3], // top(+Z) edges
    [3, 1],
    [2, 1],
    [2, 0],
    [3, 0], // side edges
];
// Adjacent face indices for each corner (for front-face visibility checks).
const CORNER_ADJ_FACES: [[usize; 3]; 8] = [
    [5, 3, 1],
    [5, 2, 1],
    [5, 2, 0],
    [5, 3, 0],
    [4, 3, 1],
    [4, 2, 1],
    [4, 2, 0],
    [4, 3, 0],
];
// For each face, the 4 "other face" indices for vertex pairs (0-1, 2-3, 4-5, 6-7).
// Kept for potential future face-inset use.
#[allow(dead_code)]
const FACE_EDGE_ADJ: [[usize; 4]; 6] = [
    [4, 2, 5, 3], // FRONT(0): TOP, RIGHT, BOTTOM, LEFT
    [5, 2, 4, 3], // BACK(1):  BOTTOM, RIGHT, TOP, LEFT
    [1, 5, 0, 4], // RIGHT(2): BACK, BOTTOM, FRONT, TOP
    [1, 4, 0, 5], // LEFT(3):  BACK, TOP, FRONT, BOTTOM
    [1, 2, 0, 3], // TOP(4):   BACK, RIGHT, FRONT, LEFT
    [1, 3, 0, 2], // BOTTOM(5):BACK, LEFT, FRONT, RIGHT
];
// Pre-normalized outward normals for each edge (average of two adjacent face normals).
const INV_SQRT2: f32 = std::f32::consts::FRAC_1_SQRT_2;
const EDGE_NORMALS: [[f32; 3]; 12] = [
    [0.0, -INV_SQRT2, -INV_SQRT2], // 0: BOTTOM–BACK
    [INV_SQRT2, 0.0, -INV_SQRT2],  // 1: BOTTOM–RIGHT
    [0.0, INV_SQRT2, -INV_SQRT2],  // 2: BOTTOM–FRONT
    [-INV_SQRT2, 0.0, -INV_SQRT2], // 3: BOTTOM–LEFT
    [0.0, -INV_SQRT2, INV_SQRT2],  // 4: TOP–BACK
    [INV_SQRT2, 0.0, INV_SQRT2],   // 5: TOP–RIGHT
    [0.0, INV_SQRT2, INV_SQRT2],   // 6: TOP–FRONT
    [-INV_SQRT2, 0.0, INV_SQRT2],  // 7: TOP–LEFT
    [-INV_SQRT2, -INV_SQRT2, 0.0], // 8: LEFT–BACK
    [INV_SQRT2, -INV_SQRT2, 0.0],  // 9: RIGHT–BACK
    [INV_SQRT2, INV_SQRT2, 0.0],   // 10: RIGHT–FRONT
    [-INV_SQRT2, INV_SQRT2, 0.0],  // 11: LEFT–FRONT
];

fn draw_view_cube(ctx: &egui::Context, camera: &Camera, gui: &mut GuiState) {
    let actions = &mut gui.actions;
    let cube_half = 42.0f32;
    let margin = 70.0f32;
    let viewport = ctx.available_rect();
    let center = egui::pos2(viewport.right() - margin, viewport.top() + margin + 10.0);

    // Camera orientation vectors (roll-aware).
    let eye = camera.eye();
    let target = camera.target;
    let fwd = normalize3(sub3(target, eye));
    let cam_right = camera.screen_right();
    let cam_up = camera.screen_up();
    let light_dir = normalize3([0.4, 0.5, 0.75]);

    // Build and project chamfer vertices.
    let cverts = build_chamfer_verts();
    let project = |v: [f32; 3]| -> (egui::Pos2, f32) {
        let sx = dot3(v, cam_right) * cube_half;
        let sy = -dot3(v, cam_up) * cube_half;
        (egui::pos2(center.x + sx, center.y + sy), dot3(v, fwd))
    };
    let cv2d: Vec<(egui::Pos2, f32)> = cverts.iter().map(|v| project(*v)).collect();

    // Compute edge quad vertices (beveled strips between adjacent faces).
    let edge_quads_2d: Vec<[(egui::Pos2, f32); 4]> = (0..12)
        .map(|ei| {
            let [a, b] = EDGE_SEGS[ei];
            let [fi1, fi2] = EDGE_ADJ_FACES[ei];
            let n1 = FACE_NORMALS[fi1];
            let n2 = FACE_NORMALS[fi2];
            let va = cverts[a];
            let vb = cverts[b];
            let bv = EDGE_BEVEL;
            [
                project([va[0] - n2[0] * bv, va[1] - n2[1] * bv, va[2] - n2[2] * bv]),
                project([vb[0] - n2[0] * bv, vb[1] - n2[1] * bv, vb[2] - n2[2] * bv]),
                project([vb[0] - n1[0] * bv, vb[1] - n1[1] * bv, vb[2] - n1[2] * bv]),
                project([va[0] - n1[0] * bv, va[1] - n1[1] * bv, va[2] - n1[2] * bv]),
            ]
        })
        .collect();

    // Face octagon 2D projections (using original chamfer vertices — no inset).
    let face_2d: Vec<Vec<(egui::Pos2, f32)>> = (0..6)
        .map(|fi| FACE_OCTAGONS[fi].iter().map(|&vi| cv2d[vi]).collect())
        .collect();

    // Sort all 26 polygons (6 faces + 8 corners + 12 edges) back-to-front.
    // Polygon ID: 0..5 = face octagons, 6..13 = corner triangles, 14..25 = edge quads.
    let mut poly_order: Vec<usize> = (0..26).collect();
    poly_order.sort_by(|&a, &b| {
        let avg_depth = |id: usize| -> f32 {
            if id < 6 {
                face_2d[id].iter().map(|q| q.1).sum::<f32>() / 8.0
            } else if id < 14 {
                CORNER_TRIS[id - 6].iter().map(|&i| cv2d[i].1).sum::<f32>() / 3.0
            } else {
                edge_quads_2d[id - 14].iter().map(|q| q.1).sum::<f32>() / 4.0
            }
        };
        avg_depth(a)
            .partial_cmp(&avg_depth(b))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("view_cube"),
    ));
    let mouse_pos = ctx.input(|i| i.pointer.hover_pos());
    let clicked = ctx.input(|i| i.pointer.button_clicked(egui::PointerButton::Primary));

    // ---- 1. Drop shadow ----
    painter.circle_filled(
        egui::pos2(center.x + 2.0, center.y + 3.0),
        cube_half * 1.45,
        egui::Color32::from_rgba_premultiplied(0, 0, 0, 35),
    );

    // ---- 2. Orbit ring + compass ----
    let ring_r = cube_half * 1.6;
    painter.circle_stroke(
        center,
        ring_r,
        egui::Stroke::new(1.5, egui::Color32::from_rgba_premultiplied(70, 72, 82, 120)),
    );
    let compass: [([f32; 3], &str); 4] = [
        ([0.0, 1.0, 0.0], "F"),
        ([1.0, 0.0, 0.0], "R"),
        ([0.0, -1.0, 0.0], "B"),
        ([-1.0, 0.0, 0.0], "L"),
    ];
    for (dir, label) in &compass {
        let dx = dot3(*dir, cam_right);
        let dy = -dot3(*dir, cam_up);
        let len = (dx * dx + dy * dy).sqrt().max(0.001);
        let (nx, ny) = (dx / len, dy / len);
        let alpha = (len * 220.0).min(200.0) as u8;
        if alpha > 40 {
            painter.text(
                egui::pos2(
                    center.x + nx * (ring_r + 10.0),
                    center.y + ny * (ring_r + 10.0),
                ),
                egui::Align2::CENTER_CENTER,
                *label,
                egui::FontId::proportional(9.0),
                egui::Color32::from_rgba_premultiplied(150, 155, 165, alpha),
            );
        }
        let (ti, to) = (ring_r - 3.0, ring_r + 3.0);
        painter.line_segment(
            [
                egui::pos2(center.x + nx * ti, center.y + ny * ti),
                egui::pos2(center.x + nx * to, center.y + ny * to),
            ],
            egui::Stroke::new(
                1.0,
                egui::Color32::from_rgba_premultiplied(90, 92, 100, alpha),
            ),
        );
    }

    // ---- 3. Hover detection ----
    #[derive(Clone, Copy)]
    enum HoverTarget {
        Face(usize),
        Edge(usize),
        Corner(usize),
    }
    let mut hover: Option<HoverTarget> = None;

    // Helper: is face fi front-facing?
    let face_visible = |fi: usize| dot3(FACE_NORMALS[fi], fwd) < 0.0;

    if let Some(mpos) = mouse_pos {
        // Corners — visible only if at least one adjacent face is front-facing.
        for ci in 0..8 {
            let [f1, f2, f3] = CORNER_ADJ_FACES[ci];
            if !face_visible(f1) && !face_visible(f2) && !face_visible(f3) {
                continue;
            }
            let pts: Vec<egui::Pos2> = CORNER_TRIS[ci].iter().map(|&i| cv2d[i].0).collect();
            if point_in_convex_poly(mpos, &pts) {
                hover = Some(HoverTarget::Corner(ci));
                break;
            }
        }
        // Edges — visible if at least one adjacent face is front-facing.
        if hover.is_none() {
            for ei in 0..12 {
                let [f1, f2] = EDGE_ADJ_FACES[ei];
                if !face_visible(f1) && !face_visible(f2) {
                    continue;
                }
                let pts: Vec<egui::Pos2> = edge_quads_2d[ei].iter().map(|q| q.0).collect();
                if point_in_convex_poly(mpos, &pts) {
                    hover = Some(HoverTarget::Edge(ei));
                    break;
                }
            }
        }
        // Faces (octagon interiors — front-facing only).
        if hover.is_none() {
            for &pi in poly_order.iter().rev() {
                if pi >= 6 {
                    continue;
                }
                if !face_visible(pi) {
                    continue;
                }
                let pts: Vec<egui::Pos2> = face_2d[pi].iter().map(|q| q.0).collect();
                if point_in_convex_poly(mpos, &pts) {
                    hover = Some(HoverTarget::Face(pi));
                    break;
                }
            }
        }
    }

    // ---- 4. Draw all polygons as a single epaint::Mesh ----
    // Using Shape::mesh instead of convex_polygon eliminates egui's automatic
    // anti-aliasing feathering on internal polygon edges, which caused visible
    // seam lines between adjacent faces/edges/corners.
    let mut cube_mesh = egui::epaint::Mesh::default();
    let mut hovered_poly: Option<(Vec<egui::Pos2>, egui::Color32)> = None;

    struct FaceLabelInfo {
        cx: f32,
        cy: f32,
        pi: usize,
        facing: f32,
        is_hovered: bool,
    }
    let mut face_labels: Vec<FaceLabelInfo> = Vec::new();

    for &pi in &poly_order {
        let (normal, pts) = if pi < 6 {
            let pts: Vec<egui::Pos2> = face_2d[pi].iter().map(|q| q.0).collect();
            (FACE_NORMALS[pi], pts)
        } else if pi < 14 {
            let ci = pi - 6;
            let pts: Vec<egui::Pos2> = CORNER_TRIS[ci].iter().map(|&i| cv2d[i].0).collect();
            (normalize3(CORNER_NORMALS[ci]), pts)
        } else {
            let ei = pi - 14;
            let pts: Vec<egui::Pos2> = edge_quads_2d[ei].iter().map(|q| q.0).collect();
            (EDGE_NORMALS[ei], pts)
        };

        let facing = -dot3(normal, fwd);
        if facing <= 0.0 {
            continue;
        }

        let diffuse = dot3(normal, light_dir).max(0.0);
        let shade = (0.35 + diffuse * 0.65).min(1.0);

        let is_hovered = if pi < 6 {
            matches!(hover, Some(HoverTarget::Face(fi)) if fi == pi)
        } else if pi < 14 {
            matches!(hover, Some(HoverTarget::Corner(ci)) if ci == pi - 6)
        } else {
            matches!(hover, Some(HoverTarget::Edge(ei)) if ei == pi - 14)
        };

        let (br, bg, bb) = if is_hovered {
            (75.0, 88.0, 115.0)
        } else if pi < 6 {
            (45.0, 50.0, 65.0)
        } else if pi < 14 {
            (38.0, 42.0, 55.0) // corners slightly darker
        } else {
            (40.0, 45.0, 58.0) // edge bevels
        };
        let r = (br * shade + 22.0).min(255.0) as u8;
        let g = (bg * shade + 22.0).min(255.0) as u8;
        let b = (bb * shade + 24.0).min(255.0) as u8;
        let fill = egui::Color32::from_rgb(r, g, b);

        if is_hovered {
            // Hovered polygon: rendered separately with stroke highlight.
            hovered_poly = Some((pts.clone(), fill));
        } else {
            // Non-hovered: add directly to mesh (no feathering = no seams).
            let base = cube_mesh.vertices.len() as u32;
            for &p in &pts {
                cube_mesh.vertices.push(egui::epaint::Vertex {
                    pos: p,
                    uv: egui::epaint::WHITE_UV,
                    color: fill,
                });
            }
            // Fan triangulation: (0,1,2), (0,2,3), ..., (0,N-2,N-1)
            for i in 1..pts.len() as u32 - 1 {
                cube_mesh.indices.push(base);
                cube_mesh.indices.push(base + i);
                cube_mesh.indices.push(base + i + 1);
            }
        }

        // Collect face label info for rendering after the mesh.
        if pi < 6 {
            let n_pts = pts.len() as f32;
            let cx = pts.iter().map(|p| p.x).sum::<f32>() / n_pts;
            let cy = pts.iter().map(|p| p.y).sum::<f32>() / n_pts;
            face_labels.push(FaceLabelInfo {
                cx,
                cy,
                pi,
                facing,
                is_hovered,
            });
        }
    }

    // Render the single mesh (all non-hovered polygons, zero internal seams).
    painter.add(egui::Shape::mesh(cube_mesh));

    // Render hovered polygon separately (convex_polygon for stroke highlight).
    if let Some((pts, fill)) = hovered_poly {
        painter.add(egui::Shape::convex_polygon(
            pts,
            fill,
            egui::Stroke::new(
                2.0,
                egui::Color32::from_rgba_premultiplied(130, 155, 210, 255),
            ),
        ));
    }

    // Render face labels on top.
    for lbl in &face_labels {
        let font_size = 9.0 + lbl.facing * 3.0;
        let alpha = ((lbl.facing * 255.0) as u8).max(80);
        let tc = if lbl.is_hovered {
            egui::Color32::from_rgba_premultiplied(255, 255, 255, alpha)
        } else {
            egui::Color32::from_rgba_premultiplied(190, 195, 210, alpha)
        };
        let tr = FACE_TEXT_RIGHT[lbl.pi];
        let sx = dot3(tr, cam_right);
        let sy = -dot3(tr, cam_up);
        let angle = sy.atan2(sx);
        let font_id = egui::FontId::proportional(font_size);
        let galley = painter.layout_no_wrap(FACE_LABELS[lbl.pi].to_string(), font_id, tc);
        let gw = galley.rect.width();
        let gh = galley.rect.height();
        let text_pos = egui::pos2(
            lbl.cx - (gw * angle.cos() - gh * angle.sin()) * 0.5,
            lbl.cy - (gw * angle.sin() + gh * angle.cos()) * 0.5,
        );
        let mut ts = egui::epaint::TextShape::new(text_pos, galley, tc);
        ts.angle = angle;
        painter.add(ts);
    }

    // ---- 5. XYZ axis indicator (drawn ON TOP of cube polygons) ----
    let axis_len = cube_half * 0.7;
    let cube_axes: [([f32; 3], egui::Color32, &str); 3] = [
        ([1.0, 0.0, 0.0], egui::Color32::from_rgb(220, 60, 60), "X"),
        ([0.0, 1.0, 0.0], egui::Color32::from_rgb(60, 200, 60), "Y"),
        ([0.0, 0.0, 1.0], egui::Color32::from_rgb(70, 90, 230), "Z"),
    ];
    for (axis, color, label) in &cube_axes {
        let sx = dot3(*axis, cam_right) * axis_len;
        let sy = -dot3(*axis, cam_up) * axis_len;
        let end = egui::pos2(center.x + sx, center.y + sy);
        let neg = egui::pos2(center.x - sx * 0.3, center.y - sy * 0.3);
        let faded =
            egui::Color32::from_rgba_premultiplied(color.r() / 3, color.g() / 3, color.b() / 3, 60);
        painter.line_segment([center, neg], egui::Stroke::new(1.0, faded));
        painter.line_segment([center, end], egui::Stroke::new(2.0, *color));
        painter.text(
            end,
            egui::Align2::CENTER_CENTER,
            *label,
            egui::FontId::proportional(8.0),
            *color,
        );
    }

    // ---- 6. Handle click ----
    if clicked {
        if let Some(target) = hover {
            match target {
                HoverTarget::Face(fi) => {
                    actions.push(GuiAction::SetStandardView(FACE_VIEWS[fi]));
                }
                HoverTarget::Edge(ei) => {
                    let [yaw, pitch] = EDGE_YAW_PITCH[ei];
                    actions.push(GuiAction::SetCameraYawPitch(yaw, pitch));
                }
                HoverTarget::Corner(ci) => {
                    let [yaw, pitch] = CORNER_YAW_PITCH[ci];
                    actions.push(GuiAction::SetCameraYawPitch(yaw, pitch));
                }
            }
        }
    }

    // ---- 7. Arrow buttons on the orbit ring ----
    let arrow_r = 10.0;
    let btn_base = egui::Color32::from_rgba_premultiplied(50, 52, 62, 170);
    let btn_hover_c = egui::Color32::from_rgba_premultiplied(80, 90, 115, 230);
    let btn_border = egui::Stroke::new(
        0.8,
        egui::Color32::from_rgba_premultiplied(90, 92, 100, 140),
    );
    let orbit_step = FRAC_PI_4;

    let arrow_dist = ring_r + 14.0; // push arrows beyond the orbit ring
    struct ArrowBtn {
        dx: f32,
        dy: f32,
        symbol: &'static str,
        screen_right: f32,
        screen_up: f32,
    }
    let arrows = [
        ArrowBtn {
            dx: 0.0,
            dy: -arrow_dist,
            symbol: "\u{25B2}",
            screen_right: 0.0,
            screen_up: orbit_step,
        },
        ArrowBtn {
            dx: 0.0,
            dy: arrow_dist,
            symbol: "\u{25BC}",
            screen_right: 0.0,
            screen_up: -orbit_step,
        },
        ArrowBtn {
            dx: -arrow_dist,
            dy: 0.0,
            symbol: "\u{25C0}",
            screen_right: -orbit_step,
            screen_up: 0.0,
        },
        ArrowBtn {
            dx: arrow_dist,
            dy: 0.0,
            symbol: "\u{25B6}",
            screen_right: orbit_step,
            screen_up: 0.0,
        },
    ];
    for btn in &arrows {
        let bpos = egui::pos2(center.x + btn.dx, center.y + btn.dy);
        let is_hov = mouse_pos
            .map(|mp| (mp - bpos).length_sq() < arrow_r * arrow_r)
            .unwrap_or(false);
        painter.circle_filled(bpos, arrow_r, if is_hov { btn_hover_c } else { btn_base });
        painter.circle_stroke(bpos, arrow_r, btn_border);
        painter.text(
            bpos,
            egui::Align2::CENTER_CENTER,
            btn.symbol,
            egui::FontId::proportional(9.0),
            egui::Color32::from_gray(if is_hov { 240 } else { 190 }),
        );
        if is_hov && clicked {
            actions.push(GuiAction::ScreenOrbit(btn.screen_right, btn.screen_up));
        }
    }

    // ---- 8. CW/CCW roll buttons (in-plane rotation) ----
    let rot_y = center.y + arrow_dist + 22.0;
    let rot_sp = 22.0;
    for (dx_sign, symbol, roll_delta) in [
        (-0.5f32, "\u{21BA}", -orbit_step),
        (0.5, "\u{21BB}", orbit_step),
    ] {
        let bpos = egui::pos2(center.x + rot_sp * dx_sign, rot_y);
        let is_hov = mouse_pos
            .map(|mp| (mp - bpos).length_sq() < arrow_r * arrow_r)
            .unwrap_or(false);
        painter.circle_filled(bpos, arrow_r, if is_hov { btn_hover_c } else { btn_base });
        painter.circle_stroke(bpos, arrow_r, btn_border);
        painter.text(
            bpos,
            egui::Align2::CENTER_CENTER,
            symbol,
            egui::FontId::proportional(13.0),
            egui::Color32::from_gray(if is_hov { 240 } else { 190 }),
        );
        if is_hov && clicked {
            actions.push(GuiAction::RollDelta(roll_delta));
        }
    }

    // ---- 9. Side buttons (Home / Projection / Menu) ----
    let side_r = 10.0;
    let side_x = center.x + arrow_dist + 18.0;
    struct SideBtn {
        y_off: f32,
        symbol: &'static str,
        font: f32,
    }
    let side_btns = [
        SideBtn {
            y_off: -24.0,
            symbol: "\u{2302}",
            font: 13.0,
        },
        SideBtn {
            y_off: 0.0,
            symbol: "",
            font: 11.0,
        }, // proj (dynamic)
        SideBtn {
            y_off: 24.0,
            symbol: "\u{2630}",
            font: 12.0,
        }, // ☰ menu
    ];
    for (si, sb) in side_btns.iter().enumerate() {
        let bpos = egui::pos2(side_x, center.y + sb.y_off);
        let is_hov = mouse_pos
            .map(|mp| (mp - bpos).length_sq() < side_r * side_r)
            .unwrap_or(false);
        painter.circle_filled(bpos, side_r, if is_hov { btn_hover_c } else { btn_base });
        painter.circle_stroke(bpos, side_r, btn_border);
        let sym = if si == 1 {
            match camera.projection {
                Projection::Perspective => "P",
                Projection::Orthographic => "O",
            }
        } else {
            sb.symbol
        };
        painter.text(
            bpos,
            egui::Align2::CENTER_CENTER,
            sym,
            egui::FontId::proportional(sb.font),
            egui::Color32::from_gray(if is_hov { 240 } else { 185 }),
        );
        if is_hov && clicked {
            match si {
                0 => actions.push(GuiAction::SetStandardView(StandardView::Isometric)),
                1 => actions.push(GuiAction::ToggleProjection),
                _ => gui.show_view_menu = !gui.show_view_menu,
            }
        }
    }

    // ---- 10. View dropdown menu ----
    if gui.show_view_menu {
        let menu_x = side_x - side_r;
        let menu_y = center.y + side_btns[2].y_off + side_r + 4.0;
        let menu_id = egui::Id::new("viewcube_menu");
        let mut close = false;
        egui::Area::new(menu_id)
            .fixed_pos(egui::pos2(menu_x, menu_y))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(140.0);
                    let proj_label = match camera.projection {
                        Projection::Orthographic => "Orthographic  \u{2714}",
                        Projection::Perspective => "Orthographic",
                    };
                    if ui.button(proj_label).clicked() {
                        if matches!(camera.projection, Projection::Perspective) {
                            actions.push(GuiAction::ToggleProjection);
                        }
                        close = true;
                    }
                    let persp_label = match camera.projection {
                        Projection::Perspective => "Perspective  \u{2714}",
                        Projection::Orthographic => "Perspective",
                    };
                    if ui.button(persp_label).clicked() {
                        if matches!(camera.projection, Projection::Orthographic) {
                            actions.push(GuiAction::ToggleProjection);
                        }
                        close = true;
                    }
                    ui.separator();
                    if ui.button("Isometric").clicked() {
                        actions.push(GuiAction::SetStandardView(StandardView::Isometric));
                        close = true;
                    }
                    ui.separator();
                    if ui.button("Fit All").clicked() {
                        actions.push(GuiAction::FitAll);
                        close = true;
                    }
                });
            });
        if close {
            gui.show_view_menu = false;
        }
        // Close on click outside the menu area.
        if clicked && !close {
            let menu_rect =
                egui::Rect::from_min_size(egui::pos2(menu_x, menu_y), egui::vec2(150.0, 100.0));
            if let Some(mp) = mouse_pos {
                if !menu_rect.contains(mp) {
                    gui.show_view_menu = false;
                }
            }
        }
    }
}

/// Check if a point lies inside a convex polygon (3+ vertices).
fn point_in_convex_poly(p: egui::Pos2, poly: &[egui::Pos2]) -> bool {
    if poly.len() < 3 {
        return false;
    }
    let mut sign = 0i32;
    let n = poly.len();
    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + 1) % n];
        let cross = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
        let s = if cross > 0.0 {
            1
        } else if cross < 0.0 {
            -1
        } else {
            0
        };
        if s != 0 {
            if sign == 0 {
                sign = s;
            } else if sign != s {
                return false;
            }
        }
    }
    true
}
