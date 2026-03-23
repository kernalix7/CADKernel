//! egui-based UI panels for the CAD application.

mod context_menu;
mod dialogs;
mod menu;
mod overlays;
mod properties;
mod report;
mod sketch_ui;
mod status_bar;
pub(crate) mod task_panel;
pub(crate) mod theme;
mod toolbar;
mod tree;
mod view_cube;

use crate::nav::NavConfig;
use crate::render::{Camera, DisplayMode, GridConfig, StandardView};
use cadkernel_io::Mesh;
use cadkernel_modeling::MassProperties;
use cadkernel_sketch::{Sketch, WorkPlane};
use cadkernel_topology::{
    BRepModel, EdgeData, FaceData, Handle, ShellData, SolidData, VertexData,
};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Workbench system
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Workbench {
    Part,
    PartDesign,
    Sketcher,
    Mesh,
    TechDraw,
    Assembly,
    Draft,
    Surface,
    Fem,
}

impl Workbench {
    pub const ALL: &[Workbench] = &[
        Workbench::Part,
        Workbench::PartDesign,
        Workbench::Sketcher,
        Workbench::Mesh,
        Workbench::TechDraw,
        Workbench::Assembly,
        Workbench::Draft,
        Workbench::Surface,
        Workbench::Fem,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Part => "\u{2B22} Part",
            Self::PartDesign => "\u{2699} PartDesign",
            Self::Sketcher => "\u{270F} Sketcher",
            Self::Mesh => "\u{25A6} Mesh",
            Self::TechDraw => "\u{1F4D0} TechDraw",
            Self::Assembly => "\u{1F527} Assembly",
            Self::Draft => "\u{2712} Draft",
            Self::Surface => "\u{223F} Surface",
            Self::Fem => "\u{2206} FEM",
        }
    }
}

// ---------------------------------------------------------------------------
// Sketch editing mode
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SketchTool {
    Select,
    Line,
    Rectangle,
    Circle,
    Arc,
}

pub(crate) struct SketchMode {
    pub sketch: Sketch,
    pub plane: WorkPlane,
    pub tool: SketchTool,
    pub pending_point: Option<(f64, f64)>,
    pub extrude_distance: f64,
}

impl SketchMode {
    pub fn new(plane: WorkPlane) -> Self {
        Self {
            sketch: Sketch::new(),
            plane,
            tool: SketchTool::Line,
            pending_point: None,
            extrude_distance: 10.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Entity selection
// ---------------------------------------------------------------------------

/// Selection mode determines what gets picked in the 3D viewport.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SelectionMode {
    Solid,
    Face,
    Edge,
    Vertex,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub(crate) enum SelectedEntity {
    Solid(Handle<SolidData>),
    Shell(Handle<ShellData>),
    Face(Handle<FaceData>),
    Edge(Handle<EdgeData>),
    Vertex(Handle<VertexData>),
}

// ---------------------------------------------------------------------------
// Report / log levels
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReportLevel {
    Info,
    Warning,
    Error,
}

// ---------------------------------------------------------------------------
// Actions emitted by the GUI
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MirrorPlane {
    XY,
    XZ,
    YZ,
}

pub(crate) enum GuiAction {
    NewModel,
    OpenFile(PathBuf),
    SaveFile(PathBuf),
    ImportFile(PathBuf),
    ExportStl(PathBuf),
    ExportObj(PathBuf),
    ExportGltf(PathBuf),
    ExportStep(PathBuf),
    ExportIges(PathBuf),
    ExportDxf(PathBuf),
    ExportPly(PathBuf),
    Export3mf(PathBuf),
    ExportBrep(PathBuf),
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
    CreateCone {
        base_radius: f64,
        top_radius: f64,
        height: f64,
    },
    CreateTorus {
        major_radius: f64,
        minor_radius: f64,
    },
    CreateTube {
        outer_radius: f64,
        inner_radius: f64,
        height: f64,
    },
    CreatePrism {
        radius: f64,
        height: f64,
        sides: usize,
    },
    CreateWedge {
        dx: f64,
        dy: f64,
        dz: f64,
        dx2: f64,
        dy2: f64,
    },
    CreateEllipsoid {
        rx: f64,
        ry: f64,
        rz: f64,
    },
    CreateHelix {
        radius: f64,
        pitch: f64,
        turns: f64,
        tube_radius: f64,
    },
    ResetCamera,
    FitAll,
    ToggleProjection,
    SetDisplayMode(DisplayMode),
    SetStandardView(StandardView),
    SetCameraYawPitch(f32, f32),
    ScreenOrbit(f32, f32),
    RollDelta(f32),
    ToggleGrid,
    Undo,
    Redo,
    BooleanUnionWith {
        width: f64,
        height: f64,
        depth: f64,
        offset: [f64; 3],
    },
    BooleanSubtractWith {
        width: f64,
        height: f64,
        depth: f64,
        offset: [f64; 3],
    },
    BooleanIntersectWith {
        width: f64,
        height: f64,
        depth: f64,
        offset: [f64; 3],
    },
    StatusMessage(String),
    MirrorSolid(MirrorPlane),
    ScaleSolid {
        factor: f64,
    },
    ShellSolid {
        thickness: f64,
    },
    FilletAllEdges {
        radius: f64,
    },
    ChamferAllEdges {
        distance: f64,
    },
    LinearPattern {
        count: usize,
        spacing: f64,
        axis: u8,
    },
    EnterSketch(WorkPlane),
    SketchClick(f64, f64),
    SetSketchTool(SketchTool),
    SketchConstrainHorizontal,
    SketchConstrainVertical,
    SketchConstrainLength(f64),
    CloseSketch,
    CancelSketch,
    TechDrawAddView(cadkernel_io::ProjectionDir),
    TechDrawThreeView,
    TechDrawExportSvg(PathBuf),
    TechDrawClear,
    MeshDecimate(f64),
    MeshSubdivide,
    MeshFlipNormals,
    MeshFillHoles,
    MeshSmooth {
        iterations: usize,
        factor: f64,
    },
    MeshHarmonizeNormals,
    MeshCheckWatertight,
    MeshRemesh {
        target_edge_len: f64,
    },
    MeshRepair,
    MeasureSolid,
    CheckGeometry,
    SelectAll,
    DeselectAll,
    DeleteSelected,
    // Scene management
    SelectObject(crate::scene::ObjectId),
    ToggleVisibility(crate::scene::ObjectId),
    RemoveObject(crate::scene::ObjectId),
    DuplicateObject(crate::scene::ObjectId),
    RenameObject(crate::scene::ObjectId, String),
    ShowAll,
    HideAll,
    // Parametric rebuild + color + task preview
    RebuildObject { id: crate::scene::ObjectId, params: crate::scene::CreationParams },
    SetObjectColor { id: crate::scene::ObjectId, color: [f32; 4] },
    TaskPreviewUpdate(task_panel::ActiveTask),
    // Transform operations
    MoveObject { id: crate::scene::ObjectId, dx: f64, dy: f64, dz: f64 },
    RotateObject { id: crate::scene::ObjectId, axis: u8, angle_deg: f64 },
    ScaleObjectUniform { id: crate::scene::ObjectId, factor: f64 },
    // Multi-select
    ToggleSelect(crate::scene::ObjectId),
    // Scene boolean (pick two objects)
    BooleanSceneUnion,
    BooleanSceneSubtract,
    BooleanSceneIntersect,
}

// ---------------------------------------------------------------------------
// GUI state persisted across frames
// ---------------------------------------------------------------------------

pub(crate) struct GuiState {
    pub show_model_tree: bool,
    pub tree_filter: String,
    pub show_properties: bool,
    pub property_tab: properties::PropertyTab,
    pub show_about: bool,
    pub show_settings: bool,
    pub show_create_box: bool,
    pub show_create_cylinder: bool,
    pub show_create_sphere: bool,
    pub show_create_cone: bool,
    pub show_create_torus: bool,
    pub show_create_tube: bool,
    pub show_create_prism: bool,
    pub show_create_wedge: bool,
    pub show_create_ellipsoid: bool,
    pub show_create_helix: bool,
    pub status_message: String,
    pub current_file: Option<String>,

    pub create_box_size: [f64; 3],
    pub create_cylinder_radius: f64,
    pub create_cylinder_height: f64,
    pub create_sphere_radius: f64,
    pub create_cone_base_radius: f64,
    pub create_cone_top_radius: f64,
    pub create_cone_height: f64,
    pub create_torus_major_radius: f64,
    pub create_torus_minor_radius: f64,
    pub create_tube_outer_radius: f64,
    pub create_tube_inner_radius: f64,
    pub create_tube_height: f64,
    pub create_prism_radius: f64,
    pub create_prism_height: f64,
    pub create_prism_sides: usize,
    pub create_wedge_dx: f64,
    pub create_wedge_dy: f64,
    pub create_wedge_dz: f64,
    pub create_wedge_dx2: f64,
    pub create_wedge_dy2: f64,
    pub create_ellipsoid_rx: f64,
    pub create_ellipsoid_ry: f64,
    pub create_ellipsoid_rz: f64,
    pub create_helix_radius: f64,
    pub create_helix_pitch: f64,
    pub create_helix_turns: f64,
    pub create_helix_tube_radius: f64,

    pub actions: Vec<GuiAction>,
    pub request_quit: bool,
    pub show_view_menu: bool,
    pub active_workbench: Workbench,

    pub sketch_mode: Option<SketchMode>,
    pub constraint_length_value: f64,

    pub techdraw_sheet: Option<cadkernel_io::DrawingSheet>,

    // Boolean dialog state
    pub show_boolean_union: bool,
    pub show_boolean_subtract: bool,
    pub show_boolean_intersect: bool,
    pub bool_box_size: [f64; 3],
    pub bool_offset: [f64; 3],

    // Part operation dialog state
    pub show_mirror: bool,
    pub mirror_plane: MirrorPlane,
    pub show_scale: bool,
    pub scale_factor: f64,
    pub show_shell: bool,
    pub shell_thickness: f64,
    pub show_fillet: bool,
    pub fillet_radius: f64,
    pub show_chamfer: bool,
    pub chamfer_distance: f64,
    pub show_pattern: bool,
    pub pattern_count: usize,
    pub pattern_spacing: f64,
    pub pattern_axis: u8,

    // Mesh dialog state
    pub show_mesh_smooth: bool,
    pub mesh_smooth_iters: usize,
    pub mesh_smooth_factor: f64,
    pub show_mesh_remesh: bool,
    pub mesh_remesh_edge_len: f64,

    // Entity selection (Block 1)
    pub selected_entity: Option<SelectedEntity>,
    pub selection_mode: SelectionMode,
    pub active_task: Option<task_panel::ActiveTask>,

    // Report panel (Block 5)
    pub report_lines: Vec<(ReportLevel, String)>,
    pub show_report_panel: bool,
    pub bottom_tab: report::BottomTab,
    pub console_history: Vec<String>,
    pub console_input: String,
    pub recent_files: Vec<String>,
    /// Object being renamed in tree: (object_id, current_text)
    pub rename_edit: Option<(crate::scene::ObjectId, String)>,
    /// Splash screen frame counter (shown for first ~60 frames).
    pub splash_frames: u32,
    pub theme_applied: bool,
    pub show_viewport_context_menu: bool,

    // Mouse world position for status bar (Block 4)
    pub mouse_world_pos: Option<[f64; 3]>,

    cached_props: Option<MassProperties>,
    cached_props_tri_count: usize,
}

impl GuiState {
    pub fn new() -> Self {
        Self {
            show_model_tree: true,
            tree_filter: String::new(),
            show_properties: true,
            property_tab: properties::PropertyTab::Data,
            show_about: false,
            show_settings: false,
            show_create_box: false,
            show_create_cylinder: false,
            show_create_sphere: false,
            show_create_cone: false,
            show_create_torus: false,
            show_create_tube: false,
            show_create_prism: false,
            show_create_wedge: false,
            show_create_ellipsoid: false,
            show_create_helix: false,
            status_message: "Ready".into(),
            current_file: None,
            create_box_size: [10.0, 10.0, 10.0],
            create_cylinder_radius: 5.0,
            create_cylinder_height: 10.0,
            create_sphere_radius: 5.0,
            create_cone_base_radius: 5.0,
            create_cone_top_radius: 0.0,
            create_cone_height: 10.0,
            create_torus_major_radius: 5.0,
            create_torus_minor_radius: 1.5,
            create_tube_outer_radius: 5.0,
            create_tube_inner_radius: 3.0,
            create_tube_height: 10.0,
            create_prism_radius: 5.0,
            create_prism_height: 10.0,
            create_prism_sides: 6,
            create_wedge_dx: 10.0,
            create_wedge_dy: 10.0,
            create_wedge_dz: 10.0,
            create_wedge_dx2: 5.0,
            create_wedge_dy2: 5.0,
            create_ellipsoid_rx: 5.0,
            create_ellipsoid_ry: 3.0,
            create_ellipsoid_rz: 2.0,
            create_helix_radius: 5.0,
            create_helix_pitch: 3.0,
            create_helix_turns: 3.0,
            create_helix_tube_radius: 0.5,
            actions: Vec::new(),
            request_quit: false,
            show_view_menu: false,
            active_workbench: Workbench::Part,
            sketch_mode: None,
            constraint_length_value: 10.0,
            techdraw_sheet: None,
            show_boolean_union: false,
            show_boolean_subtract: false,
            show_boolean_intersect: false,
            bool_box_size: [5.0, 5.0, 5.0],
            bool_offset: [5.0, 0.0, 0.0],
            show_mirror: false,
            mirror_plane: MirrorPlane::YZ,
            show_scale: false,
            scale_factor: 2.0,
            show_shell: false,
            shell_thickness: 1.0,
            show_fillet: false,
            fillet_radius: 1.0,
            show_chamfer: false,
            chamfer_distance: 1.0,
            show_pattern: false,
            pattern_count: 3,
            pattern_spacing: 15.0,
            pattern_axis: 0,
            show_mesh_smooth: false,
            mesh_smooth_iters: 3,
            mesh_smooth_factor: 0.5,
            show_mesh_remesh: false,
            mesh_remesh_edge_len: 1.0,
            selected_entity: None,
            selection_mode: SelectionMode::Solid,
            active_task: None,
            report_lines: Vec::new(),
            show_report_panel: true,
            bottom_tab: report::BottomTab::Report,
            console_history: Vec::new(),
            console_input: String::new(),
            recent_files: Vec::new(),
            rename_edit: None,
            splash_frames: 0,
            theme_applied: false,
            show_viewport_context_menu: false,
            mouse_world_pos: None,
            cached_props: None,
            cached_props_tri_count: 0,
        }
    }

    pub fn invalidate_cache(&mut self) {
        self.cached_props = None;
        self.cached_props_tri_count = 0;
    }

    pub fn log(&mut self, level: ReportLevel, msg: impl Into<String>) {
        self.report_lines.push((level, msg.into()));
    }
}

// ---------------------------------------------------------------------------
// Top-level draw entry
// ---------------------------------------------------------------------------

pub(crate) struct ViewportInfo<'a> {
    pub camera: &'a Camera,
    pub display_mode: DisplayMode,
    pub grid_config: &'a GridConfig,
    pub show_grid: bool,
    pub fps: f32,
    pub show_fps: bool,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_ui(
    ctx: &egui::Context,
    gui: &mut GuiState,
    nav: &mut NavConfig,
    vp: &ViewportInfo<'_>,
    _model: &BRepModel,
    _mesh: &Option<Mesh>,
    scene: &crate::scene::Scene,
) {
    // Apply dark theme on first frame
    if !gui.theme_applied {
        theme::apply_cad_theme(ctx);
        gui.theme_applied = true;
    }

    // Splash screen (first ~90 frames)
    if gui.splash_frames < 90 {
        gui.splash_frames += 1;
        let alpha = if gui.splash_frames < 60 { 1.0 } else { (90 - gui.splash_frames) as f32 / 30.0 };
        egui::Area::new(egui::Id::new("splash"))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(egui::Color32::from_rgba_unmultiplied(25, 28, 38, (alpha * 240.0) as u8))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(80, 120, 180, (alpha * 100.0) as u8)))
                    .corner_radius(16.0)
                    .inner_margin(48.0)
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new("\u{2B22} CADKernel")
                                .size(36.0)
                                .color(egui::Color32::from_rgba_unmultiplied(200, 220, 255, (alpha * 255.0) as u8)));
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new("Open-source CAD Software")
                                .size(16.0)
                                .color(egui::Color32::from_rgba_unmultiplied(160, 180, 200, (alpha * 255.0) as u8)));
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new("v0.1.0 — 9 crates, 15 I/O formats, NURBS kernel")
                                .size(11.0)
                                .color(egui::Color32::from_rgba_unmultiplied(120, 140, 160, (alpha * 255.0) as u8)));
                        });
                    });
            });
    }

    menu::draw_menu_bar(ctx, gui, vp.camera, vp.display_mode);
    toolbar::draw_toolbar(ctx, gui);
    toolbar::draw_workbench_tabs(ctx, gui);
    toolbar::draw_context_toolbar(ctx, gui);
    // FreeCAD-style ComboView: single left panel with tree (top) + properties/task (bottom)
    if gui.show_model_tree || gui.show_properties {
        egui::SidePanel::left("combo_view")
            .default_width(270.0)
            .frame(egui::Frame::new()
                .fill(egui::Color32::from_rgb(38, 41, 48))
                .inner_margin(egui::Margin::symmetric(6, 4)))
            .show(ctx, |ui| {
                // Top half: Model Tree
                let avail = ui.available_height();
                let tree_height = avail * 0.55;
                egui::ScrollArea::vertical()
                    .id_salt("combo_tree")
                    .max_height(tree_height)
                    .show(ui, |ui| {
                        tree::draw_model_tree_inline(ui, gui, scene);
                    });
                ui.separator();
                // Bottom half: Task Panel OR Properties
                egui::ScrollArea::vertical()
                    .id_salt("combo_props")
                    .show(ui, |ui| {
                        if !task_panel::draw_task_panel_inline(ui, gui) {
                            properties::draw_properties_inline(ui, gui, scene);
                        }
                    });
            });
    }
    report::draw_report_panel(ctx, gui);
    status_bar::draw_status_bar(ctx, gui, vp, scene);
    dialogs::draw_create_dialogs(ctx, gui);
    sketch_ui::draw_sketch_overlay(ctx, gui, vp.camera);
    overlays::draw_techdraw_overlay(ctx, gui);
    dialogs::draw_about_dialog(ctx, gui);
    dialogs::draw_settings(ctx, gui, nav);
    if nav.show_view_cube {
        view_cube::draw_view_cube(ctx, vp.camera, gui, nav);
    }
    if nav.show_axes_indicator {
        overlays::draw_axes_overlay(ctx, vp.camera);
    }
    if vp.show_grid {
        sketch_ui::draw_grid_scale_label(ctx, vp.grid_config);
    }

    // Viewport right-click context menu — use hover sense only so left-click passes
    // through to the winit event handler for 3D orbit/pick/pan.
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE)
        .show(ctx, |ui| {
            // Only capture right-click for context menu, not left-click
            let resp = ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
            if resp.secondary_clicked() {
                gui.show_viewport_context_menu = true;
            }
        });
    // Show viewport context menu as a floating window when triggered
    if gui.show_viewport_context_menu {
        let mut show = true;
        egui::Area::new(egui::Id::new("viewport_ctx"))
            .fixed_pos(ctx.pointer_latest_pos().unwrap_or(egui::pos2(100.0, 100.0)))
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    context_menu::viewport_context_menu(ui, gui);
                    if ui.button("Close").clicked() {
                        show = false;
                    }
                });
            });
        if !show || ctx.input(|i| i.pointer.any_click()) {
            gui.show_viewport_context_menu = false;
        }
    }
}
