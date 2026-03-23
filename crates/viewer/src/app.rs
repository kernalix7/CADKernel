//! Full GUI application with egui panels rendered on top of the wgpu 3D
//! viewport.

use crate::gui::{
    self, GuiAction, GuiState, MirrorPlane, ReportLevel, SelectedEntity, SketchMode, SketchTool,
    ViewportInfo,
};
use crate::nav::{NavAction, NavConfig};
use crate::render::{
    AXIS_X_COLOR, AXIS_Y_COLOR, AXIS_Z_COLOR, Camera, DisplayMode, EDGE_OVERLAY_COLOR,
    GRID_MAJOR_COLOR, GRID_MINOR_COLOR, GpuState, GridConfig, HIDDEN_LINE_COLOR, MouseState,
    NO_SHADE_COLOR, POINT_COLOR, SOLID_COLOR, StandardView, TRANSPARENT_COLOR, Uniforms, Vertex,
    WIRE_COLOR, compute_bounds, cross3, dot3, mesh_to_vertices, normalize3, sub3,
};
use cadkernel_io::{
    Mesh, export_3mf, export_brep, export_dxf, export_gltf, export_iges, export_ply, export_step,
    import_obj, import_stl, tessellate_solid, write_3mf, write_brep, write_dxf, write_obj,
    write_ply, write_stl_ascii,
};
use cadkernel_math::{Point3, Vec3};
use cadkernel_modeling::{
    BooleanOp, boolean_op, chamfer_edge, check_geometry, compute_mass_properties,
    extrude, fillet_edge, linear_pattern, make_box, make_cone, make_cylinder, make_ellipsoid,
    make_helix, make_prism, make_sphere, make_torus, make_tube, make_wedge, mirror_solid,
    scale_solid, shell_solid,
};
use cadkernel_sketch::{Constraint, extract_profile, solve};
use cadkernel_topology::{BRepModel, FaceData, Handle, SolidData, VertexData};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

// ---------------------------------------------------------------------------
// Runtime state initialised on `resumed`
// ---------------------------------------------------------------------------

struct RuntimeState {
    gpu: GpuState,
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
}

// ---------------------------------------------------------------------------
// Camera animation
// ---------------------------------------------------------------------------

struct CameraAnimation {
    start_yaw: f32,
    start_pitch: f32,
    start_roll: f32,
    target_yaw: f32,
    target_pitch: f32,
    target_roll: f32,
    elapsed: f32,
    duration: f32,
}

impl CameraAnimation {
    fn new(
        from_yaw: f32,
        from_pitch: f32,
        from_roll: f32,
        to_yaw: f32,
        to_pitch: f32,
        to_roll: f32,
        duration: f32,
    ) -> Self {
        // Normalise start yaw so we take the shortest angular path.
        let mut dy = to_yaw - from_yaw;
        if dy > std::f32::consts::PI {
            dy -= std::f32::consts::TAU;
        } else if dy < -std::f32::consts::PI {
            dy += std::f32::consts::TAU;
        }
        // Same for roll.
        let mut dr = to_roll - from_roll;
        if dr > std::f32::consts::PI {
            dr -= std::f32::consts::TAU;
        } else if dr < -std::f32::consts::PI {
            dr += std::f32::consts::TAU;
        }
        Self {
            start_yaw: to_yaw - dy,
            start_pitch: from_pitch,
            start_roll: to_roll - dr,
            target_yaw: to_yaw,
            target_pitch: to_pitch,
            target_roll: to_roll,
            elapsed: 0.0,
            duration,
        }
    }

    /// Advance by `dt` seconds.  Returns `true` when finished.
    fn tick(&mut self, dt: f32, camera: &mut Camera) -> bool {
        self.elapsed += dt;
        let t = (self.elapsed / self.duration).min(1.0);
        // Smooth-step easing: 3t² − 2t³
        let s = t * t * (3.0 - 2.0 * t);
        camera.yaw = self.start_yaw + (self.target_yaw - self.start_yaw) * s;
        camera.pitch = self.start_pitch + (self.target_pitch - self.start_pitch) * s;
        camera.roll = self.start_roll + (self.target_roll - self.start_roll) * s;
        t >= 1.0
    }
}

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

pub(crate) struct CadApp {
    runtime: Option<RuntimeState>,
    /// Multi-object scene (replaces single model/mesh/solid).
    scene: crate::scene::Scene,
    /// Legacy single-model fields (kept for compatibility during transition).
    model: BRepModel,
    current_mesh: Option<Mesh>,
    current_solid: Option<Handle<SolidData>>,
    vertices: Vec<Vertex>,
    camera: Camera,
    mouse: MouseState,
    nav: NavConfig,
    gui: GuiState,
    display_mode: DisplayMode,
    show_grid: bool,
    grid_config: GridConfig,
    mesh_rx: Option<mpsc::Receiver<Result<(Mesh, PathBuf), String>>>,
    camera_anim: Option<CameraAnimation>,
    last_instant: std::time::Instant,
    prev_roll: f32,
    fps_frames: u32,
    fps_elapsed: f32,
    fps_display: f32,
    command_stack: crate::command::CommandStack,
    mouse_dragged: bool,
    /// Per-object vertex ranges in the combined GPU buffer: (id, start, count, color, selected).
    object_ranges: Vec<(crate::scene::ObjectId, u32, u32, [f32; 4], bool)>,
}

impl CadApp {
    fn new() -> Self {
        Self {
            runtime: None,
            scene: crate::scene::Scene::new(),
            model: BRepModel::new(),
            current_mesh: None,
            current_solid: None,
            vertices: Vec::new(),
            camera: Camera::new(16.0 / 9.0),
            mouse: MouseState::new(),
            nav: NavConfig::new(),
            gui: GuiState::new(),
            display_mode: DisplayMode::Shading,
            show_grid: true,
            grid_config: GridConfig::new(),
            mesh_rx: None,
            camera_anim: None,
            last_instant: std::time::Instant::now(),
            prev_roll: 0.0,
            fps_frames: 0,
            fps_elapsed: 0.0,
            fps_display: 0.0,
            command_stack: crate::command::CommandStack::new(50),
            mouse_dragged: false,
            object_ranges: Vec::new(),
        }
    }

    // -- helpers -----------------------------------------------------------

    fn set_mesh(&mut self, mesh: Mesh) {
        self.vertices = mesh_to_vertices(&mesh);
        if !self.vertices.is_empty() {
            let (min, max) = compute_bounds(&self.vertices);
            self.camera.fit_to_bounds(min, max);
            // Tell the grid how large the object is so it can grow accordingly.
            let dx = max[0] - min[0];
            let dy = max[1] - min[1];
            let dz = max[2] - min[2];
            let extent = dx.max(dy).max(dz);
            self.grid_config.set_object_extent(extent);
        }
        self.grid_config.force_rebuild();
        self.grid_config.update_for_camera(self.camera.distance);
        if let Some(rt) = &mut self.runtime {
            rt.gpu.update_mesh(&self.vertices);
            rt.gpu.rebuild_grid(&self.grid_config);
        }
        self.gui.invalidate_cache();
        self.current_mesh = Some(mesh);
    }

    /// Rebuild the GPU vertex buffer from the entire scene (all visible objects).
    fn rebuild_scene_gpu(&mut self) {
        let (combined, ranges) = self.scene.build_combined_vertices();
        // Store per-object ranges with color and selection state
        self.object_ranges = ranges.iter().map(|&(id, start, count)| {
            let obj = self.scene.get(id);
            let color = obj.map_or([0.7, 0.75, 0.8, 1.0], |o| o.color);
            let selected = obj.is_some_and(|o| o.selected);
            (id, start, count, color, selected)
        }).collect();
        if !combined.is_empty() {
            let (min, max) = compute_bounds(&combined);
            let dx = max[0] - min[0];
            let dy = max[1] - min[1];
            let dz = max[2] - min[2];
            self.grid_config.set_object_extent(dx.max(dy).max(dz));
        }
        self.vertices = combined;
        self.grid_config.force_rebuild();
        self.grid_config.update_for_camera(self.camera.distance);
        if let Some(rt) = &mut self.runtime {
            rt.gpu.update_mesh(&self.vertices);
            rt.gpu.rebuild_grid(&self.grid_config);
        }
        self.gui.invalidate_cache();
    }

    /// Add a newly created solid to the scene and update GPU.
    fn add_to_scene(
        &mut self,
        name: &str,
        model: BRepModel,
        solid: Handle<SolidData>,
        params: Option<crate::scene::CreationParams>,
    ) {
        let mesh = cadkernel_io::tessellate_solid(&model, solid);
        self.current_mesh = Some(mesh);
        self.current_solid = Some(solid);
        self.model = model.clone();
        let id = self.scene.add_object(name, model, solid, params);
        self.scene.select_single(id);
        self.rebuild_scene_gpu();
        // Fit camera if this is the first object
        if self.scene.len() == 1 && !self.vertices.is_empty() {
            let (min, max) = compute_bounds(&self.vertices);
            self.camera.fit_to_bounds(min, max);
        }
    }

    fn collect_edge_pairs(
        &self,
        solid: Handle<SolidData>,
    ) -> Vec<(Handle<VertexData>, Handle<VertexData>)> {
        let mut edge_pairs: Vec<(Handle<VertexData>, Handle<VertexData>)> = Vec::new();
        if let Some(solid_data) = self.model.solids.get(solid) {
            for shell_h in &solid_data.shells {
                if let Some(shell) = self.model.shells.get(*shell_h) {
                    for face_h in &shell.faces {
                        if let Some(face) = self.model.faces.get(*face_h) {
                            // Collect all loops (outer + inner)
                            let mut all_loops = vec![face.outer_loop];
                            all_loops.extend_from_slice(&face.inner_loops);
                            for loop_h in &all_loops {
                                if let Some(lp) = self.model.loops.get(*loop_h) {
                                    let hes = self.model.loop_half_edges(lp.half_edge);
                                    for he_h in &hes {
                                        if let Some(he) = self.model.half_edges.get(*he_h) {
                                            if let Some(edge_h) = he.edge {
                                                if let Some(edge) =
                                                    self.model.edges.get(edge_h)
                                                {
                                                    edge_pairs.push((edge.start, edge.end));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        edge_pairs.sort_by_key(|pair| (pair.0.index(), pair.1.index()));
        edge_pairs.dedup();
        edge_pairs
    }

    fn boolean_with_box(
        &mut self,
        width: f64,
        height: f64,
        depth: f64,
        offset: [f64; 3],
        op: BooleanOp,
    ) {
        if let Some(solid_a) = self.current_solid {
            let mut model_b = BRepModel::new();
            let origin = Point3::new(offset[0], offset[1], offset[2]);
            match make_box(&mut model_b, origin, width, height, depth) {
                Ok(r_b) => {
                    match boolean_op(&self.model, solid_a, &model_b, r_b.solid, op) {
                        Ok(result_model) => {
                            // Extract the first solid handle before moving
                            let first_solid = result_model.solids.iter().next().map(|(h, _)| h);
                            if let Some(result_solid) = first_solid {
                                let mesh = tessellate_solid(&result_model, result_solid);
                                self.model = result_model;
                                self.current_solid = Some(result_solid);
                                self.gui.current_file = None;
                                self.log_info(format!("Boolean {op:?}: box {width}×{height}×{depth} at ({:.1},{:.1},{:.1})", offset[0], offset[1], offset[2]));
                                self.set_mesh(mesh);
                            } else {
                                self.log_warning("Boolean result is empty");
                            }
                        }
                        Err(e) => {
                            self.log_error(format!("Boolean error: {e}"));
                        }
                    }
                }
                Err(e) => {
                    self.log_error(format!("Box creation error: {e}"));
                }
            }
        } else {
            self.log_warning("No solid for boolean operation");
        }
    }

    fn request_redraw(&self) {
        if let Some(rt) = &self.runtime {
            rt.gpu.window.request_redraw();
        }
    }

    /// Start (or instant-snap) a camera transition to the given yaw/pitch/roll.
    fn animate_to(&mut self, yaw: f32, pitch: f32, roll: f32) {
        if self.nav.enable_view_animation {
            self.camera_anim = Some(CameraAnimation::new(
                self.camera.yaw,
                self.camera.pitch,
                self.camera.roll,
                yaw,
                pitch,
                roll,
                self.nav.view_animation_duration,
            ));
        } else {
            self.camera.yaw = yaw;
            self.camera.pitch = pitch;
            self.camera.roll = roll;
        }
    }

    /// Tick the running camera animation, if any, and update FPS counter.
    fn tick_animation(&mut self) {
        let now = std::time::Instant::now();
        let dt = now.duration_since(self.last_instant).as_secs_f32();
        self.last_instant = now;

        // FPS counter — update display value every ~0.5 s.
        self.fps_frames += 1;
        self.fps_elapsed += dt;
        if self.fps_elapsed >= 0.5 {
            self.fps_display = self.fps_frames as f32 / self.fps_elapsed;
            self.fps_frames = 0;
            self.fps_elapsed = 0.0;
        }

        if let Some(anim) = &mut self.camera_anim {
            let done = anim.tick(dt, &mut self.camera);
            if done {
                self.camera_anim = None;
            } else {
                self.request_redraw();
            }
        }
    }

    // -- 3D picking ---------------------------------------------------------

    fn try_pick_entity(&mut self) {
        let Some((sx, sy)) = self.mouse.last_pos else { return };
        let Some(rt) = &self.runtime else { return };
        let size = rt.gpu.window.inner_size();
        let w = size.width as f32;
        let h = size.height as f32;
        if w < 1.0 || h < 1.0 { return; }

        let inv_vp = self.camera.inv_view_proj();
        let (origin, dir) = crate::picking::screen_to_ray(
            sx as f32, sy as f32, w, h, inv_vp,
        );

        // Test all visible scene objects and find the closest hit
        let mut best_hit: Option<(crate::scene::ObjectId, f32)> = None;
        for obj in self.scene.visible_objects() {
            if let Some(hit) = crate::picking::pick_triangle(
                origin, dir, &obj.mesh.vertices, &obj.mesh.indices,
            ) {
                let is_closer = best_hit.as_ref().is_none_or(|(_, d)| hit.distance < *d);
                if is_closer {
                    best_hit = Some((obj.id, hit.distance));
                }
            }
        }

        if let Some((obj_id, dist)) = best_hit {
            self.scene.select_single(obj_id);
            if let Some(obj) = self.scene.get(obj_id) {
                self.model = obj.model.clone();
                self.current_solid = Some(obj.solid);
                self.current_mesh = Some(obj.mesh.clone());
                self.gui.status_message = format!(
                    "Selected: {} (dist {dist:.2})", obj.name
                );
            }
            self.rebuild_scene_gpu();
        } else {
            self.scene.deselect_all();
            self.gui.selected_entity = None;
            self.gui.status_message = "Selection cleared".into();
            self.rebuild_scene_gpu();
        }
    }

    // -- snapshot helper (for undo/redo) ------------------------------------

    fn take_snapshot(&self) -> crate::command::ModelSnapshot {
        crate::command::ModelSnapshot {
            model: self.model.clone(),
            current_solid: self.current_solid,
            current_mesh: self.current_mesh.clone(),
        }
    }

    fn restore_snapshot(&mut self, snap: crate::command::ModelSnapshot) {
        self.model = snap.model;
        self.current_solid = snap.current_solid;
        if let Some(mesh) = snap.current_mesh {
            self.set_mesh(mesh);
        } else {
            self.vertices.clear();
            self.current_mesh = None;
            if let Some(rt) = &mut self.runtime {
                rt.gpu.update_mesh(&self.vertices);
            }
        }
        self.gui.invalidate_cache();
    }

    /// Save snapshot before a model-modifying action.
    fn snapshot_before(&mut self, description: &str) {
        let snap = self.take_snapshot();
        self.command_stack.push(description, snap);
    }

    // -- report helpers -----------------------------------------------------

    fn log_info(&mut self, msg: impl Into<String>) {
        let s: String = msg.into();
        self.gui.status_message = s.clone();
        self.gui.log(ReportLevel::Info, s);
    }

    fn log_warning(&mut self, msg: impl Into<String>) {
        let s: String = msg.into();
        self.gui.status_message = s.clone();
        self.gui.log(ReportLevel::Warning, s);
    }

    fn log_error(&mut self, msg: impl Into<String>) {
        let s: String = msg.into();
        self.gui.status_message = s.clone();
        self.gui.log(ReportLevel::Error, s);
    }

    // -- action processing -------------------------------------------------

    fn process_actions(&mut self) {
        let actions: Vec<GuiAction> = self.gui.actions.drain(..).collect();
        for action in actions {
            match action {
                GuiAction::NewModel => {
                    self.snapshot_before("New model");
                    self.model = BRepModel::new();
                    self.current_mesh = None;
                    self.current_solid = None;
                    self.vertices.clear();
                    self.grid_config.set_object_extent(0.0);
                    self.grid_config.force_rebuild();
                    self.grid_config.update_for_camera(self.camera.distance);
                    if let Some(rt) = &mut self.runtime {
                        rt.gpu.update_mesh(&self.vertices);
                        rt.gpu.rebuild_grid(&self.grid_config);
                    }
                    self.gui.invalidate_cache();
                    self.gui.current_file = None;
                    self.log_info("New model created");
                }

                GuiAction::OpenFile(path) | GuiAction::ImportFile(path) => {
                    self.load_mesh_file(&path);
                }

                GuiAction::SaveFile(path) => {
                    self.export_mesh_to(&path);
                }

                GuiAction::ExportStl(path) => {
                    self.export_mesh_to(&path);
                }
                GuiAction::ExportObj(path) => {
                    self.export_mesh_to(&path);
                }
                GuiAction::ExportGltf(path) => {
                    if let Some(mesh) = &self.current_mesh {
                        match export_gltf(mesh, &path.display().to_string()) {
                            Ok(()) => {
                                self.log_info(format!("Exported glTF → {}", path.display()));
                            }
                            Err(e) => {
                                self.log_error(format!("Export error: {e}"));
                            }
                        }
                    } else {
                        self.log_warning("No mesh to export");
                    }
                }

                GuiAction::CreateBox {
                    width,
                    height,
                    depth,
                } => {
                    self.snapshot_before("Create Box");
                    let mut model = BRepModel::new();
                    match make_box(&mut model, Point3::ORIGIN, width, height, depth) {
                        Ok(r) => {
                            self.add_to_scene(
                                &format!("Box ({width}×{height}×{depth})"),
                                model, r.solid,
                                Some(crate::scene::CreationParams::Box { width, height, depth }),
                            );
                            self.log_info(format!("Created box ({width} × {height} × {depth})"));
                        }
                        Err(e) => {
                            self.log_error(format!("CreateBox error: {e}"));
                        }
                    }
                }

                GuiAction::CreateCylinder { radius, height } => {
                    self.snapshot_before("Create Cylinder");
                    let mut model = BRepModel::new();
                    match make_cylinder(&mut model, Point3::ORIGIN, radius, height, 64) {
                        Ok(r) => {
                            self.add_to_scene(
                                &format!("Cylinder (r={radius}, h={height})"),
                                model, r.solid,
                                Some(crate::scene::CreationParams::Cylinder { radius, height }),
                            );
                            self.log_info(format!("Created cylinder (r={radius}, h={height})"));
                        }
                        Err(e) => {
                            self.log_error(format!("CreateCylinder error: {e}"));
                        }
                    }
                }

                GuiAction::CreateSphere { radius } => {
                    self.snapshot_before("Create Sphere");
                    let mut model = BRepModel::new();
                    match make_sphere(&mut model, Point3::ORIGIN, radius, 64, 32) {
                        Ok(r) => {
                            self.add_to_scene(
                                &format!("Sphere (r={radius})"),
                                model, r.solid,
                                Some(crate::scene::CreationParams::Sphere { radius }),
                            );
                            self.log_info(format!("Created sphere (r={radius})"));
                        }
                        Err(e) => {
                            self.log_error(format!("CreateSphere error: {e}"));
                        }
                    }
                }

                GuiAction::CreateCone {
                    base_radius,
                    top_radius,
                    height,
                } => {
                    self.snapshot_before("Create Cone");
                    let mut model = BRepModel::new();
                    match make_cone(
                        &mut model,
                        Point3::ORIGIN,
                        base_radius,
                        top_radius,
                        height,
                        64,
                    ) {
                        Ok(r) => {
                            let kind = if top_radius < 1e-14 {
                                "cone"
                            } else {
                                "frustum"
                            };
                            self.add_to_scene(
                                &format!("Cone (r1={base_radius}, r2={top_radius}, h={height})"),
                                model, r.solid,
                                Some(crate::scene::CreationParams::Cone { base_radius, top_radius, height }),
                            );
                            self.log_info(format!(
                                "Created {kind} (r1={base_radius}, r2={top_radius}, h={height})"
                            ));
                        }
                        Err(e) => {
                            self.log_error(format!("CreateCone error: {e}"));
                        }
                    }
                }

                GuiAction::CreateTorus {
                    major_radius,
                    minor_radius,
                } => {
                    self.snapshot_before("Create Torus");
                    let mut model = BRepModel::new();
                    match make_torus(
                        &mut model,
                        Point3::ORIGIN,
                        major_radius,
                        minor_radius,
                        64,
                        32,
                    ) {
                        Ok(r) => {
                            self.add_to_scene(
                                &format!("Torus (R={major_radius}, r={minor_radius})"),
                                model, r.solid,
                                Some(crate::scene::CreationParams::Torus { major_radius, minor_radius }),
                            );
                            self.log_info(format!("Created torus (R={major_radius}, r={minor_radius})"));
                        }
                        Err(e) => {
                            self.log_error(format!("CreateTorus error: {e}"));
                        }
                    }
                }

                GuiAction::CreateTube {
                    outer_radius,
                    inner_radius,
                    height,
                } => {
                    self.snapshot_before("Create Tube");
                    let mut model = BRepModel::new();
                    match make_tube(
                        &mut model,
                        Point3::ORIGIN,
                        outer_radius,
                        inner_radius,
                        height,
                        64,
                    ) {
                        Ok(r) => {
                            self.add_to_scene(
                                &format!("Tube (R={outer_radius}, r={inner_radius}, h={height})"),
                                model, r.solid,
                                Some(crate::scene::CreationParams::Tube { outer_radius, inner_radius, height }),
                            );
                            self.log_info(format!(
                                "Created tube (R={outer_radius}, r={inner_radius}, h={height})"
                            ));
                        }
                        Err(e) => {
                            self.log_error(format!("CreateTube error: {e}"));
                        }
                    }
                }

                GuiAction::CreatePrism {
                    radius,
                    height,
                    sides,
                } => {
                    self.snapshot_before("Create Prism");
                    let mut model = BRepModel::new();
                    match make_prism(&mut model, Point3::ORIGIN, radius, height, sides) {
                        Ok(r) => {
                            self.add_to_scene(
                                &format!("{sides}-sided Prism"),
                                model, r.solid,
                                Some(crate::scene::CreationParams::Prism { radius, height, sides }),
                            );
                            self.log_info(format!("Created {sides}-sided prism (r={radius}, h={height})"));
                        }
                        Err(e) => {
                            self.log_error(format!("CreatePrism error: {e}"));
                        }
                    }
                }

                GuiAction::CreateWedge {
                    dx,
                    dy,
                    dz,
                    dx2,
                    dy2,
                } => {
                    self.snapshot_before("Create Wedge");
                    let mut model = BRepModel::new();
                    match make_wedge(&mut model, Point3::ORIGIN, dx, dy, dz, dx2, dy2, 0.0, 0.0) {
                        Ok(r) => {
                            self.add_to_scene(
                                &format!("Wedge ({dx}×{dy}×{dz})"),
                                model, r.solid,
                                Some(crate::scene::CreationParams::Wedge { dx, dy, dz, dx2, dy2 }),
                            );
                            self.log_info(format!("Created wedge ({dx}×{dy}×{dz}, top {dx2}×{dy2})"));
                        }
                        Err(e) => {
                            self.log_error(format!("CreateWedge error: {e}"));
                        }
                    }
                }

                GuiAction::CreateEllipsoid { rx, ry, rz } => {
                    self.snapshot_before("Create Ellipsoid");
                    let mut model = BRepModel::new();
                    match make_ellipsoid(&mut model, Point3::ORIGIN, rx, ry, rz, 64, 32) {
                        Ok(r) => {
                            self.add_to_scene(
                                &format!("Ellipsoid ({rx}×{ry}×{rz})"),
                                model, r.solid,
                                Some(crate::scene::CreationParams::Ellipsoid { rx, ry, rz }),
                            );
                            self.log_info(format!("Created ellipsoid ({rx}×{ry}×{rz})"));
                        }
                        Err(e) => {
                            self.log_error(format!("CreateEllipsoid error: {e}"));
                        }
                    }
                }

                GuiAction::CreateHelix {
                    radius,
                    pitch,
                    turns,
                    tube_radius,
                } => {
                    self.snapshot_before("Create Helix");
                    let mut model = BRepModel::new();
                    match make_helix(
                        &mut model,
                        Point3::ORIGIN,
                        radius,
                        pitch,
                        turns,
                        tube_radius,
                        16,
                        8,
                    ) {
                        Ok(r) => {
                            self.add_to_scene(
                                &format!("Helix (R={radius})"),
                                model, r.solid,
                                Some(crate::scene::CreationParams::Helix { radius, pitch, turns, tube_radius }),
                            );
                            self.log_info(format!(
                                "Created helix (R={radius}, pitch={pitch}, turns={turns})"
                            ));
                        }
                        Err(e) => {
                            self.log_error(format!("CreateHelix error: {e}"));
                        }
                    }
                }

                GuiAction::ResetCamera => {
                    self.camera.reset(); // roll is reset inside reset()
                    self.gui.status_message = "Camera reset".into();
                }

                GuiAction::FitAll => {
                    if !self.vertices.is_empty() {
                        let (min, max) = compute_bounds(&self.vertices);
                        self.camera.fit_to_bounds(min, max);
                    }
                    self.gui.status_message = "Camera fit to model".into();
                }

                GuiAction::ToggleProjection => {
                    self.camera.toggle_projection();
                    let label = match self.camera.projection {
                        crate::render::Projection::Perspective => "Perspective",
                        crate::render::Projection::Orthographic => "Orthographic",
                    };
                    self.gui.status_message = format!("Projection: {label}");
                }

                GuiAction::SetDisplayMode(mode) => {
                    self.display_mode = mode;
                    self.gui.status_message = format!("Display: {}", mode.label());
                }

                GuiAction::SetStandardView(view) => {
                    let (mut yaw, pitch) = view.yaw_pitch();
                    // Top/Bottom: preserve current yaw (only pitch changes).
                    // At pitch ≈ ±90° the yaw determines screen orientation,
                    // so forcing a fixed yaw causes unwanted in-plane rotation.
                    if matches!(view, StandardView::Top | StandardView::Bottom) {
                        yaw = self.camera.yaw;
                    }
                    // Roll: snap to nearest 90°; at midpoint, prefer prev_roll side.
                    let roll = snap_roll_90(self.camera.roll, self.prev_roll);
                    self.animate_to(yaw, pitch, roll);
                    self.gui.status_message = format!("View: {}", view.label());
                }

                GuiAction::SetCameraYawPitch(yaw, pitch) => {
                    let roll = snap_roll_90(self.camera.roll, self.prev_roll);
                    self.animate_to(yaw, pitch, roll);
                }

                GuiAction::ScreenOrbit(right, up) => {
                    // Snap to animation target so consecutive presses chain correctly.
                    if let Some(anim) = self.camera_anim.take() {
                        self.camera.yaw = anim.target_yaw;
                        self.camera.pitch = anim.target_pitch;
                        self.camera.roll = anim.target_roll;
                    }
                    // Save prev_roll AFTER snap — captures the clean target, not
                    // an intermediate interpolated value mid-animation.
                    self.prev_roll = self.camera.roll;

                    // Current camera basis.
                    let sr = self.camera.screen_right();
                    let su = self.camera.screen_up();
                    let fwd = {
                        let e = self.camera.eye();
                        let t = self.camera.target;
                        normalize3([t[0] - e[0], t[1] - e[1], t[2] - e[2]])
                    };

                    // Rotation axis (local): screen_up for L/R, screen_right for U/D.
                    let axis_raw = [
                        su[0] * right - sr[0] * up,
                        su[1] * right - sr[1] * up,
                        su[2] * right - sr[2] * up,
                    ];
                    let len = dot3(axis_raw, axis_raw).sqrt();
                    if len > 1e-6 {
                        let k = [axis_raw[0] / len, axis_raw[1] / len, axis_raw[2] / len];
                        let angle = std::f32::consts::FRAC_PI_4; // 45° per press
                        let (ca, sa) = (angle.cos(), angle.sin());

                        // Rodrigues helper: rotate v around k by angle.
                        let rod = |v: [f32; 3]| -> [f32; 3] {
                            let kxv = cross3(k, v);
                            let kdv = dot3(k, v);
                            [
                                v[0] * ca + kxv[0] * sa + k[0] * kdv * (1.0 - ca),
                                v[1] * ca + kxv[1] * sa + k[1] * kdv * (1.0 - ca),
                                v[2] * ca + kxv[2] * sa + k[2] * kdv * (1.0 - ca),
                            ]
                        };

                        // Rotate BOTH forward and up vectors.
                        let f2 = rod(fwd);
                        let u2 = rod(su);

                        // Extract yaw / pitch from the new forward vector (Z-up).
                        let new_pitch = (-f2[2]).clamp(-1.0, 1.0).asin().clamp(
                            -std::f32::consts::FRAC_PI_2 + 0.01,
                            std::f32::consts::FRAC_PI_2 - 0.01,
                        );
                        let new_yaw = (-f2[1]).atan2(-f2[0]);

                        // Compute default up for (new_yaw, new_pitch, roll=0).
                        let nf = normalize3([
                            -new_yaw.cos() * new_pitch.cos(),
                            -new_yaw.sin() * new_pitch.cos(),
                            -new_pitch.sin(),
                        ]);
                        let up_z = if new_pitch.cos() >= 0.0 { 1.0 } else { -1.0 };
                        let def_r = normalize3(cross3(nf, [0.0, 0.0, up_z]));
                        let def_u = cross3(def_r, nf);

                        // Roll = angle from default_up to u2, measured around nf.
                        let new_roll = (-dot3(u2, def_r)).atan2(dot3(u2, def_u));

                        self.animate_to(new_yaw, new_pitch, new_roll);
                    }
                }

                GuiAction::RollDelta(delta) => {
                    self.prev_roll = self.camera.roll;
                    self.camera.roll = wrap_angle(self.camera.roll + delta);
                }

                GuiAction::ToggleGrid => {
                    self.show_grid = !self.show_grid;
                    self.gui.status_message = if self.show_grid {
                        "Grid shown"
                    } else {
                        "Grid hidden"
                    }
                    .into();
                }

                GuiAction::Undo => {
                    let current = self.take_snapshot();
                    if let Some(snap) = self.command_stack.undo(current) {
                        self.restore_snapshot(snap);
                        let desc = self.command_stack.redo_description()
                            .unwrap_or("action").to_string();
                        self.log_info(format!("Undo: {desc}"));
                    } else {
                        self.gui.status_message = "Nothing to undo".into();
                    }
                }
                GuiAction::Redo => {
                    let current = self.take_snapshot();
                    if let Some(snap) = self.command_stack.redo(current) {
                        self.restore_snapshot(snap);
                        let desc = self.command_stack.undo_description()
                            .unwrap_or("action").to_string();
                        self.log_info(format!("Redo: {desc}"));
                    } else {
                        self.gui.status_message = "Nothing to redo".into();
                    }
                }
                GuiAction::StatusMessage(msg) => {
                    self.gui.status_message = msg;
                }

                // -- Sketch actions --
                GuiAction::EnterSketch(plane) => {
                    self.gui.sketch_mode = Some(SketchMode::new(plane));
                    self.gui.active_workbench = gui::Workbench::Sketcher;
                    self.gui.status_message =
                        "Sketch mode: click to add points, select tool from toolbar".into();
                }

                GuiAction::SketchClick(x, y) => {
                    self.handle_sketch_click(x, y);
                }

                GuiAction::SetSketchTool(tool) => {
                    if let Some(sm) = &mut self.gui.sketch_mode {
                        sm.tool = tool;
                        sm.pending_point = None;
                        self.gui.status_message = format!("Sketch tool: {tool:?}");
                    }
                }

                GuiAction::SketchConstrainHorizontal => {
                    if let Some(sm) = &mut self.gui.sketch_mode {
                        if !sm.sketch.lines.is_empty() {
                            let lid = cadkernel_sketch::LineId(sm.sketch.lines.len() - 1);
                            sm.sketch.add_constraint(Constraint::Horizontal(lid));
                            self.gui.status_message = "Added Horizontal constraint".into();
                        }
                    }
                }

                GuiAction::SketchConstrainVertical => {
                    if let Some(sm) = &mut self.gui.sketch_mode {
                        if !sm.sketch.lines.is_empty() {
                            let lid = cadkernel_sketch::LineId(sm.sketch.lines.len() - 1);
                            sm.sketch.add_constraint(Constraint::Vertical(lid));
                            self.gui.status_message = "Added Vertical constraint".into();
                        }
                    }
                }

                GuiAction::SketchConstrainLength(len) => {
                    if let Some(sm) = &mut self.gui.sketch_mode {
                        if !sm.sketch.lines.is_empty() {
                            let lid = cadkernel_sketch::LineId(sm.sketch.lines.len() - 1);
                            sm.sketch.add_constraint(Constraint::Length(lid, len));
                            self.gui.status_message = format!("Added Length={len:.1} constraint");
                        }
                    }
                }

                GuiAction::CloseSketch => {
                    self.close_sketch();
                }

                GuiAction::CancelSketch => {
                    self.gui.sketch_mode = None;
                    self.gui.status_message = "Sketch cancelled".into();
                }

                // -- TechDraw actions --
                GuiAction::TechDrawAddView(dir) => {
                    if let Some(solid) = self.current_solid {
                        let view =
                            cadkernel_io::project_solid(&self.model, solid, dir);
                        let n_edges = view.edges.len();
                        let sheet = self
                            .gui
                            .techdraw_sheet
                            .get_or_insert_with(cadkernel_io::DrawingSheet::a4_landscape);
                        sheet.views.push(view);
                        self.gui.status_message =
                            format!("TechDraw: added {} view ({n_edges} edges)", dir.label());
                    } else {
                        self.gui.status_message =
                            "TechDraw: no solid to project".into();
                    }
                }

                GuiAction::TechDrawThreeView => {
                    if let Some(solid) = self.current_solid {
                        let sheet =
                            cadkernel_io::three_view_drawing(&self.model, solid);
                        let total: usize = sheet.views.iter().map(|v| v.edges.len()).sum();
                        self.gui.techdraw_sheet = Some(sheet);
                        self.gui.status_message =
                            format!("TechDraw: 3-view drawing ({total} edges)");
                    } else {
                        self.gui.status_message =
                            "TechDraw: no solid to project".into();
                    }
                }

                GuiAction::TechDrawExportSvg(path) => {
                    if let Some(sheet) = &self.gui.techdraw_sheet {
                        let svg = cadkernel_io::drawing_to_svg(sheet);
                        match std::fs::write(&path, svg.render()) {
                            Ok(()) => {
                                self.gui.status_message =
                                    format!("Exported SVG: {}", path.display());
                            }
                            Err(e) => {
                                self.gui.status_message =
                                    format!("SVG export failed: {e}");
                            }
                        }
                    } else {
                        self.gui.status_message =
                            "TechDraw: no drawing to export".into();
                    }
                }

                GuiAction::TechDrawClear => {
                    self.gui.techdraw_sheet = None;
                    self.gui.status_message = "TechDraw: cleared".into();
                }

                // -- Mesh operations --
                GuiAction::MeshDecimate(ratio) => {
                    if let Some(mesh) = &self.current_mesh {
                        match cadkernel_io::decimate_mesh(mesh, ratio) {
                            Ok(new_mesh) => {
                                let count = new_mesh.indices.len();
                                self.set_mesh(new_mesh);
                                self.log_info(format!("Decimated to {count} triangles"));
                            }
                            Err(e) => {
                                self.log_error(format!("Decimate failed: {e}"));
                            }
                        }
                    } else {
                        self.log_warning("No mesh to decimate");
                    }
                }
                GuiAction::MeshSubdivide => {
                    if let Some(mesh) = &self.current_mesh {
                        match cadkernel_io::subdivide_mesh(mesh) {
                            Ok(new_mesh) => {
                                let count = new_mesh.indices.len();
                                self.set_mesh(new_mesh);
                                self.log_info(format!("Subdivided to {count} triangles"));
                            }
                            Err(e) => {
                                self.log_error(format!("Subdivide failed: {e}"));
                            }
                        }
                    } else {
                        self.log_warning("No mesh to subdivide");
                    }
                }
                GuiAction::MeshFillHoles => {
                    if let Some(mesh) = &self.current_mesh {
                        match cadkernel_io::fill_holes(mesh) {
                            Ok(new_mesh) => {
                                let count = new_mesh.indices.len();
                                self.set_mesh(new_mesh);
                                self.log_info(format!("Filled holes: {count} triangles"));
                            }
                            Err(e) => {
                                self.log_error(format!("Fill holes failed: {e}"));
                            }
                        }
                    } else {
                        self.gui.status_message = "No mesh".into();
                    }
                }
                GuiAction::MeshFlipNormals => {
                    if let Some(mesh) = &self.current_mesh {
                        let new_mesh = cadkernel_io::flip_normals(mesh);
                        self.set_mesh(new_mesh);
                        self.log_info("Normals flipped");
                    } else {
                        self.gui.status_message = "No mesh".into();
                    }
                }

                // -- Export formats --
                GuiAction::ExportStep(path) => {
                    match export_step(&self.model) {
                        Ok(content) => match std::fs::write(&path, &content) {
                            Ok(()) => {
                                self.log_info(format!("Exported STEP → {}", path.display()));
                            }
                            Err(e) => {
                                self.log_error(format!("Write error: {e}"));
                            }
                        },
                        Err(e) => {
                            self.log_error(format!("STEP export error: {e}"));
                        }
                    }
                }
                GuiAction::ExportIges(path) => {
                    match export_iges(&self.model) {
                        Ok(content) => match std::fs::write(&path, &content) {
                            Ok(()) => {
                                self.log_info(format!("Exported IGES → {}", path.display()));
                            }
                            Err(e) => {
                                self.log_error(format!("Write error: {e}"));
                            }
                        },
                        Err(e) => {
                            self.log_error(format!("IGES export error: {e}"));
                        }
                    }
                }
                GuiAction::ExportDxf(path) => {
                    if let Some(mesh) = &self.current_mesh {
                        match export_dxf(mesh) {
                            Ok(content) => {
                                let path_str = path.to_str().unwrap_or("");
                                match write_dxf(path_str, &content) {
                                    Ok(()) => {
                                        self.log_info(format!("Exported DXF → {}", path.display()));
                                    }
                                    Err(e) => {
                                        self.log_error(format!("Write error: {e}"));
                                    }
                                }
                            }
                            Err(e) => {
                                self.log_error(format!("DXF export error: {e}"));
                            }
                        }
                    } else {
                        self.gui.status_message = "No mesh to export".into();
                    }
                }
                GuiAction::ExportPly(path) => {
                    if let Some(mesh) = &self.current_mesh {
                        match export_ply(mesh) {
                            Ok(content) => {
                                let path_str = path.to_str().unwrap_or("");
                                match write_ply(path_str, &content) {
                                    Ok(()) => {
                                        self.log_info(format!("Exported PLY → {}", path.display()));
                                    }
                                    Err(e) => {
                                        self.log_error(format!("Write error: {e}"));
                                    }
                                }
                            }
                            Err(e) => {
                                self.log_error(format!("PLY export error: {e}"));
                            }
                        }
                    } else {
                        self.gui.status_message = "No mesh to export".into();
                    }
                }
                GuiAction::Export3mf(path) => {
                    if let Some(mesh) = &self.current_mesh {
                        match export_3mf(mesh) {
                            Ok(content) => {
                                let path_str = path.to_str().unwrap_or("");
                                match write_3mf(path_str, &content) {
                                    Ok(()) => {
                                        self.log_info(format!("Exported 3MF → {}", path.display()));
                                    }
                                    Err(e) => {
                                        self.log_error(format!("Write error: {e}"));
                                    }
                                }
                            }
                            Err(e) => {
                                self.log_error(format!("3MF export error: {e}"));
                            }
                        }
                    } else {
                        self.gui.status_message = "No mesh to export".into();
                    }
                }
                GuiAction::ExportBrep(path) => {
                    match export_brep(&self.model) {
                        Ok(content) => {
                            let path_str = path.to_str().unwrap_or("");
                            match write_brep(path_str, &content) {
                                Ok(()) => {
                                    self.log_info(format!("Exported BREP → {}", path.display()));
                                }
                                Err(e) => {
                                    self.log_error(format!("Write error: {e}"));
                                }
                            }
                        }
                        Err(e) => {
                            self.log_error(format!("BREP export error: {e}"));
                        }
                    }
                }

                // -- Boolean operations with second primitive --
                GuiAction::BooleanUnionWith {
                    width,
                    height,
                    depth,
                    offset,
                } => {
                    self.boolean_with_box(width, height, depth, offset, BooleanOp::Union);
                }
                GuiAction::BooleanSubtractWith {
                    width,
                    height,
                    depth,
                    offset,
                } => {
                    self.boolean_with_box(width, height, depth, offset, BooleanOp::Difference);
                }
                GuiAction::BooleanIntersectWith {
                    width,
                    height,
                    depth,
                    offset,
                } => {
                    self.boolean_with_box(width, height, depth, offset, BooleanOp::Intersection);
                }

                // -- Part operations --
                GuiAction::MirrorSolid(plane) => {
                    if let Some(solid) = self.current_solid {
                        let (point, normal) = match plane {
                            MirrorPlane::XY => (Point3::ORIGIN, Vec3::new(0.0, 0.0, 1.0)),
                            MirrorPlane::XZ => (Point3::ORIGIN, Vec3::new(0.0, 1.0, 0.0)),
                            MirrorPlane::YZ => (Point3::ORIGIN, Vec3::new(1.0, 0.0, 0.0)),
                        };
                        match mirror_solid(&mut self.model, solid, point, normal) {
                            Ok(r) => {
                                let mesh = tessellate_solid(&self.model, r.solid);
                                self.current_solid = Some(r.solid);
                                self.log_info(format!("Mirrored across {plane:?} plane"));
                                self.set_mesh(mesh);
                            }
                            Err(e) => {
                                self.log_error(format!("Mirror error: {e}"));
                            }
                        }
                    } else {
                        self.log_warning("No solid to mirror");
                    }
                }

                GuiAction::ScaleSolid { factor } => {
                    if let Some(solid) = self.current_solid {
                        match scale_solid(&mut self.model, solid, Point3::ORIGIN, factor) {
                            Ok(r) => {
                                let mesh = tessellate_solid(&self.model, r.solid);
                                self.current_solid = Some(r.solid);
                                self.log_info(format!("Scaled by {factor:.2}×"));
                                self.set_mesh(mesh);
                            }
                            Err(e) => {
                                self.log_error(format!("Scale error: {e}"));
                            }
                        }
                    } else {
                        self.gui.status_message = "No solid to scale".into();
                    }
                }

                GuiAction::ShellSolid { thickness } => {
                    if let Some(solid) = self.current_solid {
                        // Remove the top face (last face in the solid's shell)
                        let faces_to_remove: Vec<Handle<FaceData>> = {
                            if let Some(solid_data) = self.model.solids.get(solid) {
                                if let Some(shell_h) = solid_data.shells.first() {
                                    if let Some(shell) = self.model.shells.get(*shell_h) {
                                        shell.faces.last().copied().into_iter().collect()
                                    } else {
                                        vec![]
                                    }
                                } else {
                                    vec![]
                                }
                            } else {
                                vec![]
                            }
                        };
                        match shell_solid(
                            &mut self.model,
                            solid,
                            &faces_to_remove,
                            thickness,
                        ) {
                            Ok(r) => {
                                let mesh = tessellate_solid(&self.model, r.solid);
                                self.current_solid = Some(r.solid);
                                self.log_info(format!("Shell: thickness={thickness:.2}"));
                                self.set_mesh(mesh);
                            }
                            Err(e) => {
                                self.log_error(format!("Shell error: {e}"));
                            }
                        }
                    } else {
                        self.gui.status_message = "No solid for shell".into();
                    }
                }

                GuiAction::FilletAllEdges { radius } => {
                    if let Some(solid) = self.current_solid {
                        let edges = self.collect_edge_pairs(solid);
                        // Apply fillet to first edge only (all-edges would compound errors)
                        if let Some(&(v1, v2)) = edges.first() {
                            match fillet_edge(&mut self.model, solid, v1, v2, radius) {
                                Ok(r) => {
                                    let mesh = tessellate_solid(&self.model, r.solid);
                                    self.current_solid = Some(r.solid);
                                    self.log_info(format!("Fillet: r={radius:.2} (1 edge)"));
                                    self.set_mesh(mesh);
                                }
                                Err(e) => {
                                    self.log_error(format!("Fillet error: {e}"));
                                }
                            }
                        } else {
                            self.gui.status_message = "No edges found for fillet".into();
                        }
                    } else {
                        self.gui.status_message = "No solid for fillet".into();
                    }
                }

                GuiAction::ChamferAllEdges { distance } => {
                    if let Some(solid) = self.current_solid {
                        let edges = self.collect_edge_pairs(solid);
                        if let Some(&(v1, v2)) = edges.first() {
                            match chamfer_edge(&mut self.model, solid, v1, v2, distance) {
                                Ok(r) => {
                                    let mesh = tessellate_solid(&self.model, r.solid);
                                    self.current_solid = Some(r.solid);
                                    self.log_info(format!("Chamfer: d={distance:.2} (1 edge)"));
                                    self.set_mesh(mesh);
                                }
                                Err(e) => {
                                    self.log_error(format!("Chamfer error: {e}"));
                                }
                            }
                        } else {
                            self.gui.status_message = "No edges found for chamfer".into();
                        }
                    } else {
                        self.gui.status_message = "No solid for chamfer".into();
                    }
                }

                GuiAction::LinearPattern {
                    count,
                    spacing,
                    axis,
                } => {
                    if let Some(solid) = self.current_solid {
                        let dir = match axis {
                            0 => Vec3::new(1.0, 0.0, 0.0),
                            1 => Vec3::new(0.0, 1.0, 0.0),
                            _ => Vec3::new(0.0, 0.0, 1.0),
                        };
                        match linear_pattern(&mut self.model, solid, dir, spacing, count) {
                            Ok(r) => {
                                // Tessellate & show the last copy
                                if let Some(&last) = r.solids.last() {
                                    let mesh = tessellate_solid(&self.model, last);
                                    self.current_solid = Some(last);
                                    self.log_info(format!(
                                        "Linear pattern: {count}× along {:?}, spacing={spacing:.1}",
                                        ["X", "Y", "Z"][axis.min(2) as usize]
                                    ));
                                    self.set_mesh(mesh);
                                }
                            }
                            Err(e) => {
                                self.log_error(format!("Pattern error: {e}"));
                            }
                        }
                    } else {
                        self.gui.status_message = "No solid for pattern".into();
                    }
                }

                // -- Mesh operations (new) --
                GuiAction::MeshSmooth { iterations, factor } => {
                    if let Some(mesh) = &self.current_mesh {
                        let new_mesh = cadkernel_io::smooth_mesh(mesh, iterations, factor);
                        let count = new_mesh.vertices.len();
                        self.set_mesh(new_mesh);
                        self.log_info(format!("Smoothed: {iterations} iters, factor={factor:.2} ({count} verts)"));
                    } else {
                        self.gui.status_message = "No mesh to smooth".into();
                    }
                }
                GuiAction::MeshHarmonizeNormals => {
                    if let Some(mesh) = &self.current_mesh {
                        let new_mesh = cadkernel_io::harmonize_normals(mesh);
                        self.set_mesh(new_mesh);
                        self.log_info("Normals harmonized");
                    } else {
                        self.gui.status_message = "No mesh".into();
                    }
                }
                GuiAction::MeshCheckWatertight => {
                    if let Some(mesh) = &self.current_mesh {
                        let is_wt = cadkernel_io::check_mesh_watertight(mesh);
                        if is_wt {
                            self.log_info("Mesh is watertight");
                        } else {
                            self.log_warning("Mesh is NOT watertight (has boundary edges)");
                        }
                    } else {
                        self.gui.status_message = "No mesh".into();
                    }
                }
                GuiAction::MeshRemesh { target_edge_len } => {
                    if let Some(mesh) = &self.current_mesh {
                        match cadkernel_io::remesh(mesh, target_edge_len) {
                            Ok(new_mesh) => {
                                let count = new_mesh.indices.len();
                                self.set_mesh(new_mesh);
                                self.log_info(format!("Remeshed: {count} triangles (edge≤{target_edge_len:.2})"));
                            }
                            Err(e) => {
                                self.log_error(format!("Remesh error: {e}"));
                            }
                        }
                    } else {
                        self.gui.status_message = "No mesh to remesh".into();
                    }
                }
                GuiAction::MeshRepair => {
                    if let Some(mesh) = &self.current_mesh {
                        let (repaired, report) = cadkernel_io::evaluate_and_repair(mesh);
                        let msg = format!(
                            "Repair: {} degenerate removed, {} duplicates merged, normals {}",
                            report.degenerate_removed,
                            report.duplicate_vertices_merged,
                            if report.normals_harmonized {
                                "harmonized"
                            } else {
                                "OK"
                            }
                        );
                        self.set_mesh(repaired);
                        self.log_info(msg);
                    } else {
                        self.gui.status_message = "No mesh to repair".into();
                    }
                }

                // -- Measure / Analysis --
                GuiAction::MeasureSolid => {
                    if let Some(mesh) = &self.current_mesh {
                        let props = compute_mass_properties(mesh);
                        self.log_info(format!(
                            "Volume={:.3}, Area={:.3}, Center=({:.2},{:.2},{:.2})",
                            props.volume,
                            props.surface_area,
                            props.centroid.x,
                            props.centroid.y,
                            props.centroid.z,
                        ));
                    } else {
                        self.log_warning("No mesh to measure");
                    }
                }
                GuiAction::CheckGeometry => {
                    if let Some(solid) = self.current_solid {
                        let result = check_geometry(&self.model, solid);
                        if result.is_valid {
                            self.gui.status_message =
                                "Geometry check: VALID (no issues found)".into();
                            self.gui.log(ReportLevel::Info, "Geometry check: VALID");
                        } else {
                            let msg = format!(
                                "Geometry check: INVALID — {} issue(s): {}",
                                result.issues.len(),
                                result.issues.first().map_or("", |s| s.as_str()),
                            );
                            self.gui.status_message = msg.clone();
                            self.gui.log(ReportLevel::Warning, msg);
                        }
                    } else {
                        self.gui.status_message = "No solid to check".into();
                    }
                }

                GuiAction::SelectAll => {
                    if let Some((handle, _)) = self.model.solids.iter().next() {
                        self.gui.selected_entity = Some(SelectedEntity::Solid(handle));
                        self.gui.status_message = "Selected all".into();
                    }
                }
                GuiAction::DeselectAll => {
                    self.gui.selected_entity = None;
                    self.gui.status_message = "Selection cleared".into();
                }
                // -- Scene management --
                GuiAction::SelectObject(id) => {
                    self.scene.select_single(id);
                    if let Some(obj) = self.scene.get(id) {
                        self.model = obj.model.clone();
                        self.current_solid = Some(obj.solid);
                        self.current_mesh = Some(obj.mesh.clone());
                        self.gui.status_message = format!("Selected: {}", obj.name);
                    }
                }
                GuiAction::ToggleVisibility(id) => {
                    if let Some(obj) = self.scene.get_mut(id) {
                        obj.visible = !obj.visible;
                        let state = if obj.visible { "shown" } else { "hidden" };
                        let name = obj.name.clone();
                        self.log_info(format!("{name}: {state}"));
                    }
                    self.rebuild_scene_gpu();
                }
                GuiAction::RemoveObject(id) => {
                    self.snapshot_before("Remove object");
                    if let Some(obj) = self.scene.get(id) {
                        let name = obj.name.clone();
                        self.scene.remove_object(id);
                        self.log_info(format!("Removed: {name}"));
                    }
                    self.rebuild_scene_gpu();
                }
                GuiAction::DuplicateObject(id) => {
                    self.snapshot_before("Duplicate object");
                    if let Some(obj) = self.scene.get(id).cloned() {
                        let new_name = format!("{} (copy)", obj.name);
                        self.scene.add_object(
                            new_name,
                            obj.model,
                            obj.solid,
                            obj.params,
                        );
                        self.rebuild_scene_gpu();
                        self.log_info("Object duplicated");
                    }
                }
                GuiAction::ShowAll => {
                    for obj in &mut self.scene.objects {
                        obj.visible = true;
                    }
                    self.rebuild_scene_gpu();
                    self.log_info("All objects shown");
                }
                GuiAction::HideAll => {
                    for obj in &mut self.scene.objects {
                        obj.visible = false;
                    }
                    self.rebuild_scene_gpu();
                    self.log_info("All objects hidden");
                }

                // -- Transform operations --
                GuiAction::MoveObject { id, dx, dy, dz } => {
                    self.snapshot_before("Move object");
                    if let Some(obj) = self.scene.get_mut(id) {
                        // Move all vertices in the mesh
                        let offset = cadkernel_math::Vec3::new(dx, dy, dz);
                        for v in &mut obj.mesh.vertices {
                            *v += offset;
                        }
                        for v in &mut obj.model.vertices.iter_mut() {
                            v.1.point += offset;
                        }
                        obj.vertices = crate::render::mesh_to_vertices(&obj.mesh);
                        let name = obj.name.clone();
                        self.rebuild_scene_gpu();
                        self.log_info(format!("Moved {name} by ({dx:.1}, {dy:.1}, {dz:.1})"));
                    }
                }
                GuiAction::RotateObject { id, axis, angle_deg } => {
                    self.snapshot_before("Rotate object");
                    if let Some(obj) = self.scene.get_mut(id) {
                        let angle = angle_deg.to_radians();
                        let (ca, sa) = (angle.cos(), angle.sin());
                        for v in &mut obj.mesh.vertices {
                            let p = *v;
                            *v = match axis {
                                0 => cadkernel_math::Point3::new(p.x, p.y * ca - p.z * sa, p.y * sa + p.z * ca),
                                1 => cadkernel_math::Point3::new(p.x * ca + p.z * sa, p.y, -p.x * sa + p.z * ca),
                                _ => cadkernel_math::Point3::new(p.x * ca - p.y * sa, p.x * sa + p.y * ca, p.z),
                            };
                        }
                        for n in &mut obj.mesh.normals {
                            let v = *n;
                            *n = match axis {
                                0 => cadkernel_math::Vec3::new(v.x, v.y * ca - v.z * sa, v.y * sa + v.z * ca),
                                1 => cadkernel_math::Vec3::new(v.x * ca + v.z * sa, v.y, -v.x * sa + v.z * ca),
                                _ => cadkernel_math::Vec3::new(v.x * ca - v.y * sa, v.x * sa + v.y * ca, v.z),
                            };
                        }
                        obj.vertices = crate::render::mesh_to_vertices(&obj.mesh);
                        let axis_name = ["X", "Y", "Z"][axis.min(2) as usize];
                        let name = obj.name.clone();
                        self.rebuild_scene_gpu();
                        self.log_info(format!("Rotated {name} {angle_deg:.1}° around {axis_name}"));
                    }
                }
                GuiAction::ScaleObjectUniform { id, factor } => {
                    self.snapshot_before("Scale object");
                    if let Some(obj) = self.scene.get_mut(id) {
                        for v in &mut obj.mesh.vertices {
                            *v = cadkernel_math::Point3::new(v.x * factor, v.y * factor, v.z * factor);
                        }
                        obj.vertices = crate::render::mesh_to_vertices(&obj.mesh);
                        let name = obj.name.clone();
                        self.rebuild_scene_gpu();
                        self.log_info(format!("Scaled {name} by {factor:.2}×"));
                    }
                }

                GuiAction::DeleteSelected => {
                    if self.current_solid.is_some() {
                        self.snapshot_before("Delete solid");
                        self.model = BRepModel::new();
                        self.current_solid = None;
                        self.current_mesh = None;
                        self.vertices.clear();
                        if let Some(rt) = &mut self.runtime {
                            rt.gpu.update_mesh(&self.vertices);
                        }
                        self.gui.invalidate_cache();
                        self.gui.selected_entity = None;
                        self.log_info("Deleted solid");
                    } else {
                        self.gui.status_message = "Nothing to delete".into();
                    }
                }
            }
        }
    }

    /// Handle a click on the sketch plane.
    fn handle_sketch_click(&mut self, x: f64, y: f64) {
        let sm = match &mut self.gui.sketch_mode {
            Some(sm) => sm,
            None => return,
        };

        match sm.tool {
            SketchTool::Select => {
                // Select mode: nothing for now
            }
            SketchTool::Line => {
                if let Some((px, py)) = sm.pending_point.take() {
                    let p0 = sm.sketch.add_point(px, py);
                    let p1 = sm.sketch.add_point(x, y);
                    sm.sketch.add_line(p0, p1);
                    // Chain: start next line from end of previous
                    sm.pending_point = Some((x, y));
                    self.gui.status_message = format!(
                        "Line added ({:.1},{:.1}) -> ({:.1},{:.1})",
                        px, py, x, y
                    );
                } else {
                    sm.pending_point = Some((x, y));
                    self.gui.status_message = format!("Line start: ({x:.1}, {y:.1})");
                }
            }
            SketchTool::Rectangle => {
                if let Some((px, py)) = sm.pending_point.take() {
                    // Create 4 corners and 4 lines
                    let p0 = sm.sketch.add_point(px, py);
                    let p1 = sm.sketch.add_point(x, py);
                    let p2 = sm.sketch.add_point(x, y);
                    let p3 = sm.sketch.add_point(px, y);
                    sm.sketch.add_line(p0, p1);
                    sm.sketch.add_line(p1, p2);
                    sm.sketch.add_line(p2, p3);
                    sm.sketch.add_line(p3, p0);
                    self.gui.status_message = format!(
                        "Rectangle ({:.1},{:.1}) -> ({:.1},{:.1})",
                        px, py, x, y
                    );
                } else {
                    sm.pending_point = Some((x, y));
                    self.gui.status_message =
                        format!("Rectangle corner 1: ({x:.1}, {y:.1})");
                }
            }
            SketchTool::Circle => {
                if let Some((cx, cy)) = sm.pending_point.take() {
                    let dx = x - cx;
                    let dy = y - cy;
                    let radius = (dx * dx + dy * dy).sqrt();
                    let center = sm.sketch.add_point(cx, cy);
                    sm.sketch.add_circle(center, radius);
                    self.gui.status_message =
                        format!("Circle center ({cx:.1},{cy:.1}) r={radius:.1}");
                } else {
                    sm.pending_point = Some((x, y));
                    self.gui.status_message = format!("Circle center: ({x:.1}, {y:.1})");
                }
            }
            SketchTool::Arc => {
                // Arc needs 3 clicks: center, start, end. Simplified: 2 clicks = center + radius point
                if let Some((cx, cy)) = sm.pending_point.take() {
                    let dx = x - cx;
                    let dy = y - cy;
                    let radius = (dx * dx + dy * dy).sqrt();
                    let start_angle = 0.0;
                    let end_angle = std::f64::consts::PI;
                    let center = sm.sketch.add_point(cx, cy);
                    let sp = sm
                        .sketch
                        .add_point(cx + radius, cy);
                    let ep = sm
                        .sketch
                        .add_point(cx - radius, cy);
                    sm.sketch
                        .add_arc(center, sp, ep, radius, start_angle, end_angle);
                    self.gui.status_message =
                        format!("Arc center ({cx:.1},{cy:.1}) r={radius:.1}");
                } else {
                    sm.pending_point = Some((x, y));
                    self.gui.status_message = format!("Arc center: ({x:.1}, {y:.1})");
                }
            }
        }
    }

    /// Close the sketch: solve constraints, extract profile, extrude if distance > 0.
    fn close_sketch(&mut self) {
        let sm = match self.gui.sketch_mode.take() {
            Some(sm) => sm,
            None => return,
        };

        let mut sketch = sm.sketch;

        // Solve constraints
        if !sketch.constraints.is_empty() {
            let result = solve(&mut sketch, 200, 1e-10);
            if !result.converged {
                self.gui.status_message = format!(
                    "Sketch solver: did not converge (residual={:.2e})",
                    result.residual
                );
                // Put sketch back for user to fix
                self.gui.sketch_mode = Some(SketchMode {
                    sketch,
                    plane: sm.plane,
                    tool: sm.tool,
                    pending_point: None,
                    extrude_distance: sm.extrude_distance,
                });
                return;
            }
        }

        // Extract profile
        let profile = extract_profile(&sketch, &sm.plane);
        if profile.len() < 3 {
            self.gui.status_message = format!(
                "Sketch has only {} points, need at least 3 for extrude",
                profile.len()
            );
            return;
        }

        // Extrude along plane normal
        let distance = sm.extrude_distance;
        if distance.abs() < 1e-10 {
            self.gui.status_message =
                "Sketch closed (no extrude — distance is 0)".into();
            return;
        }

        let dir = Vec3::new(
            sm.plane.normal.x,
            sm.plane.normal.y,
            sm.plane.normal.z,
        );
        let mut model = BRepModel::new();
        match extrude(&mut model, &profile, dir, distance) {
            Ok(r) => {
                let mesh = tessellate_solid(&model, r.solid);
                self.model = model;
                self.current_solid = Some(r.solid);
                self.gui.current_file = None;
                self.gui.status_message = format!(
                    "Sketch → Extrude: {} faces, distance={distance:.1}",
                    mesh.indices.len()
                );
                self.set_mesh(mesh);
            }
            Err(e) => {
                self.gui.status_message = format!("Extrude error: {e}");
            }
        }
    }

    /// Unproject a screen pixel to the sketch work plane.
    /// Returns 2D coordinates in the sketch plane, or None if the ray is parallel.
    fn screen_to_sketch_plane(&self, sx: f64, sy: f64) -> Option<(f64, f64)> {
        let sm = self.gui.sketch_mode.as_ref()?;
        let rt = self.runtime.as_ref()?;
        let size = rt.gpu.window.inner_size();
        let w = size.width as f32;
        let h = size.height as f32;

        // NDC from screen pixel
        let ndc_x = (sx as f32 / w) * 2.0 - 1.0;
        let ndc_y = 1.0 - (sy as f32 / h) * 2.0; // invert Y

        // Ray from camera through pixel
        let eye = self.camera.eye();
        let r = self.camera.screen_right();
        let u = self.camera.screen_up();
        let f = normalize3(sub3(self.camera.target, eye));

        // For perspective: ray = eye + t * dir, where dir = f + ndc_x*right*tan(fov/2)*aspect + ndc_y*up*tan(fov/2)
        let half_fov_tan = (self.camera.fovy * 0.5).tan();
        let dir = normalize3([
            f[0] + ndc_x * r[0] * half_fov_tan * self.camera.aspect
                + ndc_y * u[0] * half_fov_tan,
            f[1] + ndc_x * r[1] * half_fov_tan * self.camera.aspect
                + ndc_y * u[1] * half_fov_tan,
            f[2] + ndc_x * r[2] * half_fov_tan * self.camera.aspect
                + ndc_y * u[2] * half_fov_tan,
        ]);

        // Intersect ray with sketch plane: dot(origin + t*dir - plane_point, plane_normal) = 0
        let pn = [
            sm.plane.normal.x as f32,
            sm.plane.normal.y as f32,
            sm.plane.normal.z as f32,
        ];
        let po = [
            sm.plane.origin.x as f32,
            sm.plane.origin.y as f32,
            sm.plane.origin.z as f32,
        ];
        let denom = dot3(dir, pn);
        if denom.abs() < 1e-6 {
            return None; // ray parallel to plane
        }
        let t = dot3(sub3(po, eye), pn) / denom;
        if t < 0.0 {
            return None; // behind camera
        }
        let hit = [eye[0] + t * dir[0], eye[1] + t * dir[1], eye[2] + t * dir[2]];

        // Project 3D hit point to sketch 2D coordinates
        let rel = sub3(hit, po);
        let xa = [
            sm.plane.x_axis.x as f32,
            sm.plane.x_axis.y as f32,
            sm.plane.x_axis.z as f32,
        ];
        let ya = [
            sm.plane.y_axis.x as f32,
            sm.plane.y_axis.y as f32,
            sm.plane.y_axis.z as f32,
        ];
        let sx2d = dot3(rel, xa) as f64;
        let sy2d = dot3(rel, ya) as f64;
        Some((sx2d, sy2d))
    }

    fn load_mesh_file(&mut self, path: &Path) {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext != "stl" && ext != "obj" && ext != "cadk" {
            self.log_error(format!("Unsupported format: .{ext}"));
            return;
        }

        // .cadk files load a BRepModel + tessellate; mesh files load directly.
        if ext == "cadk" {
            let path_str = path.to_str().unwrap_or("").to_string();
            match cadkernel_io::load_project(&path_str) {
                Ok(model) => {
                    self.model = model;
                    // Tessellate all solids into a combined mesh.
                    let mut combined = Mesh::new();
                    let mut solid_count = 0usize;
                    for (sh, _) in self.model.solids.iter() {
                        let m = tessellate_solid(&self.model, sh);
                        if !m.indices.is_empty() {
                            let off = combined.vertices.len() as u32;
                            combined.vertices.extend_from_slice(&m.vertices);
                            combined.normals.extend_from_slice(&m.normals);
                            for idx in &m.indices {
                                combined
                                    .indices
                                    .push([idx[0] + off, idx[1] + off, idx[2] + off]);
                            }
                        }
                        solid_count += 1;
                    }
                    if !combined.indices.is_empty() {
                        self.gui.current_file = Some(path.display().to_string());
                        self.log_info(format!(
                            "Loaded {} ({} solids, {} triangles)",
                            path.display(),
                            solid_count,
                            combined.triangle_count()
                        ));
                        self.set_mesh(combined);
                    } else {
                        self.gui.current_file = Some(path.display().to_string());
                        self.log_info(format!("Loaded {} (empty model)", path.display()));
                    }
                }
                Err(e) => {
                    self.log_error(format!("Failed to load: {e}"));
                }
            }
            return;
        }

        let path_buf = path.to_path_buf();
        let (tx, rx) = mpsc::channel();
        self.mesh_rx = Some(rx);
        self.gui.status_message = format!("Loading {}…", path.display());

        std::thread::spawn(move || {
            let path_str = path_buf.to_str().unwrap_or("");
            let ext = path_buf
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            let result = match ext.as_str() {
                "stl" => import_stl(path_str),
                "obj" => import_obj(path_str),
                _ => unreachable!(),
            };
            let _ = tx.send(result.map(|m| (m, path_buf)).map_err(|e| e.to_string()));
        });
    }

    fn poll_background_load(&mut self) {
        let done = if let Some(rx) = &self.mesh_rx {
            match rx.try_recv() {
                Ok(Ok((mesh, path))) => {
                    self.model = BRepModel::new();
                    self.current_solid = None;
                    self.gui.current_file = Some(path.display().to_string());
                    self.log_info(format!(
                        "Loaded {} ({} vertices, {} triangles)",
                        path.display(),
                        mesh.vertices.len(),
                        mesh.triangle_count()
                    ));
                    self.set_mesh(mesh);
                    true
                }
                Ok(Err(e)) => {
                    self.log_error(format!("Failed to load: {e}"));
                    true
                }
                Err(mpsc::TryRecvError::Empty) => false,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.log_error("Load thread crashed");
                    true
                }
            }
        } else {
            false
        };
        if done {
            self.mesh_rx = None;
        }
    }

    fn export_mesh_to(&mut self, path: &Path) {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        // .cadk saves the BRepModel (not just the mesh).
        if ext == "cadk" {
            let path_str = path.to_str().unwrap_or("");
            match cadkernel_io::save_project(&self.model, path_str) {
                Ok(()) => {
                    self.gui.current_file = Some(path.display().to_string());
                    self.log_info(format!("Saved → {}", path.display()));
                }
                Err(e) => {
                    self.log_error(format!("Save error: {e}"));
                }
            }
            return;
        }

        let Some(mesh) = &self.current_mesh else {
            self.log_warning("No mesh to export");
            return;
        };

        let result = match ext.as_str() {
            "stl" => std::fs::write(path, write_stl_ascii(mesh, "CADKernel")).map_err(Into::into),
            "obj" => std::fs::write(path, write_obj(mesh)).map_err(Into::into),
            _ => Err(format!("Unsupported export format: .{ext}").into()),
        };

        match result {
            Ok(()) => {
                self.log_info(format!("Exported → {}", path.display()));
            }
            Err(e) => {
                let e: Box<dyn std::error::Error> = e;
                self.log_error(format!("Export error: {e}"));
            }
        }
    }

    // -- combined 3D + egui render -----------------------------------------

    fn render_frame(&mut self) {
        // Rebuild grid when zoom level changes
        if self.grid_config.update_for_camera(self.camera.distance) {
            if let Some(rt) = &mut self.runtime {
                rt.gpu.rebuild_grid(&self.grid_config);
            }
        }

        let Self {
            runtime,
            scene,
            camera,
            gui,
            nav,
            model,
            current_mesh,
            display_mode,
            show_grid,
            grid_config,
            fps_display,
            object_ranges,
            ..
        } = self;
        let Some(rt) = runtime else { return };
        let dm = *display_mode;
        let vp_info = ViewportInfo {
            camera,
            display_mode: dm,
            grid_config,
            show_grid: *show_grid,
            fps: *fps_display,
            show_fps: nav.show_fps,
        };

        // 1. Run egui ---------------------------------------------------
        let raw_input = rt.egui_state.take_egui_input(&rt.gpu.window);
        let full_output = rt.egui_ctx.run(raw_input, |ctx| {
            gui::draw_ui(ctx, gui, nav, &vp_info, model, current_mesh, scene);
        });
        rt.egui_state
            .handle_platform_output(&rt.gpu.window, full_output.platform_output);
        let paint_jobs = rt
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        // 2. Prepare uniforms (dynamic offset slots) --------------------
        let vp = camera.view_proj();
        let eye = camera.eye();
        let eye_pos = [eye[0], eye[1], eye[2], 0.0];

        // Headlight: offset slightly up-right from camera direction to avoid
        // specular blow-out when looking straight at surfaces.
        let cam_fwd = normalize3([
            camera.target[0] - eye[0],
            camera.target[1] - eye[1],
            camera.target[2] - eye[2],
        ]);
        let cam_r = camera.screen_right();
        let cam_u = camera.screen_up();
        // Light from upper-right of camera (larger offset to avoid frontal blow-out)
        let ld = normalize3([
            -cam_fwd[0] + cam_r[0] * 0.5 + cam_u[0] * 0.7,
            -cam_fwd[1] + cam_r[1] * 0.5 + cam_u[1] * 0.7,
            -cam_fwd[2] + cam_r[2] * 0.5 + cam_u[2] * 0.7,
        ]);
        let light = [ld[0], ld[1], ld[2], 0.0];
        let no_light = [0.0f32; 4];
        // params: x=use_lighting, y=specular_strength, z=shininess
        let lit_params = [1.0f32, 0.15, 128.0, 0.0];
        let unlit_params = [0.0f32; 4];
        let mut slot: u32 = 0;

        let grid_minor_slot = slot;
        slot += 1;
        let grid_major_slot = slot;
        slot += 1;
        let grid_ax_slot = slot;
        slot += 1;
        let grid_ay_slot = slot;
        slot += 1;
        let grid_az_slot = slot;
        slot += 1;

        if *show_grid {
            rt.gpu.write_slot(
                grid_minor_slot,
                &Uniforms {
                    view_proj: vp,
                    light_dir: no_light,
                    base_color: GRID_MINOR_COLOR,
                    params: unlit_params,
                    eye_pos,
                },
            );
            rt.gpu.write_slot(
                grid_major_slot,
                &Uniforms {
                    view_proj: vp,
                    light_dir: no_light,
                    base_color: GRID_MAJOR_COLOR,
                    params: unlit_params,
                    eye_pos,
                },
            );
            rt.gpu.write_slot(
                grid_ax_slot,
                &Uniforms {
                    view_proj: vp,
                    light_dir: no_light,
                    base_color: AXIS_X_COLOR,
                    params: unlit_params,
                    eye_pos,
                },
            );
            rt.gpu.write_slot(
                grid_ay_slot,
                &Uniforms {
                    view_proj: vp,
                    light_dir: no_light,
                    base_color: AXIS_Y_COLOR,
                    params: unlit_params,
                    eye_pos,
                },
            );
            rt.gpu.write_slot(
                grid_az_slot,
                &Uniforms {
                    view_proj: vp,
                    light_dir: no_light,
                    base_color: AXIS_Z_COLOR,
                    params: unlit_params,
                    eye_pos,
                },
            );
        }

        let mesh_slot = slot;
        slot += 1;
        let wire_slot = slot;
        match dm {
            DisplayMode::AsIs | DisplayMode::Shading => {
                rt.gpu.write_slot(
                    mesh_slot,
                    &Uniforms {
                        view_proj: vp,
                        light_dir: light,
                        base_color: SOLID_COLOR,
                        params: lit_params,
                        eye_pos,
                    },
                );
            }
            DisplayMode::Points => {
                rt.gpu.write_slot(
                    mesh_slot,
                    &Uniforms {
                        view_proj: vp,
                        light_dir: no_light,
                        base_color: POINT_COLOR,
                        params: unlit_params,
                        eye_pos,
                    },
                );
            }
            DisplayMode::Wireframe => {
                rt.gpu.write_slot(
                    mesh_slot,
                    &Uniforms {
                        view_proj: vp,
                        light_dir: no_light,
                        base_color: WIRE_COLOR,
                        params: unlit_params,
                        eye_pos,
                    },
                );
            }
            DisplayMode::HiddenLine => {
                rt.gpu.write_slot(
                    mesh_slot,
                    &Uniforms {
                        view_proj: vp,
                        light_dir: no_light,
                        base_color: HIDDEN_LINE_COLOR,
                        params: lit_params,
                        eye_pos,
                    },
                );
                rt.gpu.write_slot(
                    wire_slot,
                    &Uniforms {
                        view_proj: vp,
                        light_dir: no_light,
                        base_color: EDGE_OVERLAY_COLOR,
                        params: unlit_params,
                        eye_pos,
                    },
                );
            }
            DisplayMode::NoShading => {
                rt.gpu.write_slot(
                    mesh_slot,
                    &Uniforms {
                        view_proj: vp,
                        light_dir: no_light,
                        base_color: NO_SHADE_COLOR,
                        params: lit_params,
                        eye_pos,
                    },
                );
            }
            DisplayMode::Transparent => {
                rt.gpu.write_slot(
                    mesh_slot,
                    &Uniforms {
                        view_proj: vp,
                        light_dir: light,
                        base_color: TRANSPARENT_COLOR,
                        params: lit_params,
                        eye_pos,
                    },
                );
            }
            DisplayMode::FlatLines => {
                rt.gpu.write_slot(
                    mesh_slot,
                    &Uniforms {
                        view_proj: vp,
                        light_dir: light,
                        base_color: SOLID_COLOR,
                        params: lit_params,
                        eye_pos,
                    },
                );
                rt.gpu.write_slot(
                    wire_slot,
                    &Uniforms {
                        view_proj: vp,
                        light_dir: light,
                        base_color: EDGE_OVERLAY_COLOR,
                        params: unlit_params,
                        eye_pos,
                    },
                );
            }
        }

        // 3. Acquire frame -----------------------------------------------
        let frame = match rt.gpu.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                rt.gpu.surface.configure(&rt.gpu.device, &rt.gpu.config);
                return;
            }
            Err(e) => {
                eprintln!("surface error: {e:?}");
                return;
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = rt
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        // 4. 3D render pass (gradient bg + grid + mesh) -----------------
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &rt.gpu.msaa_view,
                    resolve_target: Some(&view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.12,
                            g: 0.12,
                            b: 0.16,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &rt.gpu.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            // Background gradient
            pass.set_pipeline(&rt.gpu.gradient_pipeline);
            pass.draw(0..3, 0..1);

            // Grid
            if *show_grid {
                pass.set_pipeline(&rt.gpu.wire_pipeline);
                pass.set_vertex_buffer(0, rt.gpu.grid.buffer.slice(..));
                pass.set_bind_group(
                    0,
                    &rt.gpu.uniform_bind_group,
                    &[GpuState::slot_offset(grid_minor_slot)],
                );
                if !rt.gpu.grid.minor_range.is_empty() {
                    pass.draw(rt.gpu.grid.minor_range.clone(), 0..1);
                }
                pass.set_bind_group(
                    0,
                    &rt.gpu.uniform_bind_group,
                    &[GpuState::slot_offset(grid_major_slot)],
                );
                if !rt.gpu.grid.major_range.is_empty() {
                    pass.draw(rt.gpu.grid.major_range.clone(), 0..1);
                }
                pass.set_bind_group(
                    0,
                    &rt.gpu.uniform_bind_group,
                    &[GpuState::slot_offset(grid_ax_slot)],
                );
                if !rt.gpu.grid.axis_x_range.is_empty() {
                    pass.draw(rt.gpu.grid.axis_x_range.clone(), 0..1);
                }
                pass.set_bind_group(
                    0,
                    &rt.gpu.uniform_bind_group,
                    &[GpuState::slot_offset(grid_ay_slot)],
                );
                if !rt.gpu.grid.axis_y_range.is_empty() {
                    pass.draw(rt.gpu.grid.axis_y_range.clone(), 0..1);
                }
                pass.set_bind_group(
                    0,
                    &rt.gpu.uniform_bind_group,
                    &[GpuState::slot_offset(grid_az_slot)],
                );
                if !rt.gpu.grid.axis_z_range.is_empty() {
                    pass.draw(rt.gpu.grid.axis_z_range.clone(), 0..1);
                }
            }

            // Mesh — per-object rendering with individual colors
            if rt.gpu.num_vertices > 0 {
                pass.set_vertex_buffer(0, rt.gpu.vertex_buffer.slice(..));

                // Write per-object uniform slots (starting after grid+mesh slots)
                let obj_slot_base = slot + 1;
                for (i, &(_id, _start, _count, color, selected)) in object_ranges.iter().enumerate() {
                    let obj_slot = obj_slot_base + i as u32;
                    if obj_slot >= 62 { break; } // leave room
                    let obj_color = if selected {
                        // Selection highlight: green tint
                        [color[0] * 0.5 + 0.15, color[1] * 0.5 + 0.35, color[2] * 0.5 + 0.1, color[3]]
                    } else {
                        color
                    };
                    rt.gpu.write_slot(obj_slot, &Uniforms {
                        view_proj: vp,
                        light_dir: light,
                        base_color: obj_color,
                        params: lit_params,
                        eye_pos,
                    });
                }

                match dm {
                    DisplayMode::AsIs | DisplayMode::Shading => {
                        pass.set_pipeline(&rt.gpu.solid_pipeline);
                        if object_ranges.is_empty() {
                            // Legacy: single mesh fallback
                            pass.set_bind_group(0, &rt.gpu.uniform_bind_group, &[GpuState::slot_offset(mesh_slot)]);
                            pass.draw(0..rt.gpu.num_vertices, 0..1);
                        } else {
                            for (i, &(_id, start, count, _color, _sel)) in object_ranges.iter().enumerate() {
                                let obj_slot = obj_slot_base + i as u32;
                                if obj_slot >= 62 || count == 0 { continue; }
                                pass.set_bind_group(0, &rt.gpu.uniform_bind_group, &[GpuState::slot_offset(obj_slot)]);
                                pass.draw(start..start + count, 0..1);
                            }
                        }
                    }
                    DisplayMode::Points => {
                        pass.set_bind_group(
                            0,
                            &rt.gpu.uniform_bind_group,
                            &[GpuState::slot_offset(mesh_slot)],
                        );
                        pass.set_pipeline(&rt.gpu.wire_pipeline);
                        pass.draw(0..rt.gpu.num_vertices, 0..1);
                    }
                    DisplayMode::Wireframe => {
                        pass.set_bind_group(
                            0,
                            &rt.gpu.uniform_bind_group,
                            &[GpuState::slot_offset(mesh_slot)],
                        );
                        pass.set_pipeline(&rt.gpu.wire_pipeline);
                        pass.set_index_buffer(
                            rt.gpu.edge_index_buffer.slice(..),
                            wgpu::IndexFormat::Uint32,
                        );
                        pass.draw_indexed(0..rt.gpu.num_edge_indices, 0, 0..1);
                    }
                    DisplayMode::HiddenLine => {
                        pass.set_bind_group(
                            0,
                            &rt.gpu.uniform_bind_group,
                            &[GpuState::slot_offset(mesh_slot)],
                        );
                        pass.set_pipeline(&rt.gpu.solid_pipeline);
                        pass.draw(0..rt.gpu.num_vertices, 0..1);
                        pass.set_bind_group(
                            0,
                            &rt.gpu.uniform_bind_group,
                            &[GpuState::slot_offset(wire_slot)],
                        );
                        pass.set_pipeline(&rt.gpu.wire_pipeline);
                        pass.set_index_buffer(
                            rt.gpu.edge_index_buffer.slice(..),
                            wgpu::IndexFormat::Uint32,
                        );
                        pass.draw_indexed(0..rt.gpu.num_edge_indices, 0, 0..1);
                    }
                    DisplayMode::NoShading => {
                        pass.set_bind_group(
                            0,
                            &rt.gpu.uniform_bind_group,
                            &[GpuState::slot_offset(mesh_slot)],
                        );
                        pass.set_pipeline(&rt.gpu.solid_pipeline);
                        pass.draw(0..rt.gpu.num_vertices, 0..1);
                    }
                    DisplayMode::Transparent => {
                        pass.set_bind_group(
                            0,
                            &rt.gpu.uniform_bind_group,
                            &[GpuState::slot_offset(mesh_slot)],
                        );
                        pass.set_pipeline(&rt.gpu.transparent_pipeline);
                        pass.draw(0..rt.gpu.num_vertices, 0..1);
                    }
                    DisplayMode::FlatLines => {
                        pass.set_bind_group(
                            0,
                            &rt.gpu.uniform_bind_group,
                            &[GpuState::slot_offset(mesh_slot)],
                        );
                        pass.set_pipeline(&rt.gpu.solid_pipeline);
                        pass.draw(0..rt.gpu.num_vertices, 0..1);
                        pass.set_bind_group(
                            0,
                            &rt.gpu.uniform_bind_group,
                            &[GpuState::slot_offset(wire_slot)],
                        );
                        pass.set_pipeline(&rt.gpu.wire_pipeline);
                        pass.set_index_buffer(
                            rt.gpu.edge_index_buffer.slice(..),
                            wgpu::IndexFormat::Uint32,
                        );
                        pass.draw_indexed(0..rt.gpu.num_edge_indices, 0, 0..1);
                    }
                }
            }
        }

        // 5. egui render pass (overlay on top) --------------------------
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [rt.gpu.config.width, rt.gpu.config.height],
            pixels_per_point: full_output.pixels_per_point,
        };
        for (id, delta) in &full_output.textures_delta.set {
            rt.egui_renderer
                .update_texture(&rt.gpu.device, &rt.gpu.queue, *id, delta);
        }
        rt.egui_renderer.update_buffers(
            &rt.gpu.device,
            &rt.gpu.queue,
            &mut encoder,
            &paint_jobs,
            &screen,
        );
        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("egui_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                })
                .forget_lifetime();
            rt.egui_renderer.render(&mut pass, &paint_jobs, &screen);
        }
        for id in &full_output.textures_delta.free {
            rt.egui_renderer.free_texture(id);
        }

        // 6. Submit & present -------------------------------------------
        rt.gpu.queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        if rt.egui_ctx.has_requested_repaint() {
            rt.gpu.window.request_redraw();
        }
    }
}

// ---------------------------------------------------------------------------
// winit ApplicationHandler
// ---------------------------------------------------------------------------

impl ApplicationHandler for CadApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.runtime.is_some() {
            return;
        }
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("CADKernel")
                        .with_inner_size(winit::dpi::LogicalSize::new(1440, 900)),
                )
                .unwrap(),
        );
        let size = window.inner_size();
        self.camera.aspect = size.width as f32 / size.height.max(1) as f32;

        let gpu = pollster::block_on(GpuState::new(window.clone(), &self.vertices));

        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            None,
            None,
            None,
        );
        let egui_renderer =
            egui_wgpu::Renderer::new(&gpu.device, gpu.config.format, None, 1, false);

        self.runtime = Some(RuntimeState {
            gpu,
            egui_ctx,
            egui_state,
            egui_renderer,
        });
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // Forward to egui first; honour its repaint request.
        let egui_consumed = if let Some(rt) = &mut self.runtime {
            let response = rt.egui_state.on_window_event(&rt.gpu.window, &event);
            if response.repaint {
                rt.gpu.window.request_redraw();
            }
            response.consumed
        } else {
            false
        };

        // Modifier key tracking (always, regardless of egui).
        if let WindowEvent::ModifiersChanged(mods) = &event {
            let s = mods.state();
            self.mouse.shift_held = s.shift_key();
            self.mouse.ctrl_held = s.control_key();
            self.mouse.alt_held = s.alt_key();
        }

        // Camera orbit / pan / zoom only when egui did not consume the event.
        if !egui_consumed {
            match &event {
                WindowEvent::MouseInput { state, button, .. } => {
                    let pressed = *state == ElementState::Pressed;
                    match button {
                        MouseButton::Left => {
                            if pressed {
                                self.mouse_dragged = false;
                                // In sketch mode, left-click adds entities
                                if self.gui.sketch_mode.is_some() {
                                    if let Some((sx, sy)) = self.mouse.last_pos {
                                        if let Some(pt) = self.screen_to_sketch_plane(sx, sy) {
                                            self.gui.actions.push(GuiAction::SketchClick(pt.0, pt.1));
                                        }
                                    }
                                }
                            } else if !self.mouse_dragged && self.gui.sketch_mode.is_none() {
                                // Left-click release without drag → 3D picking
                                self.try_pick_entity();
                            }
                            self.mouse.left_pressed = pressed;
                        }
                        MouseButton::Middle => self.mouse.middle_pressed = pressed,
                        MouseButton::Right => {
                            // Right-click in sketch mode: clear pending point
                            if pressed {
                                if let Some(sm) = &mut self.gui.sketch_mode {
                                    sm.pending_point = None;
                                }
                            }
                            self.mouse.right_pressed = pressed;
                        }
                        _ => {}
                    }
                }

                WindowEvent::CursorMoved { position, .. } => {
                    if let Some((lx, ly)) = self.mouse.last_pos {
                        let dx = position.x - lx;
                        let dy = position.y - ly;
                        if (dx * dx + dy * dy) > 9.0 {
                            self.mouse_dragged = true;
                        }

                        let action = self.nav.resolve_drag(
                            self.mouse.left_pressed,
                            self.mouse.middle_pressed,
                            self.mouse.right_pressed,
                            self.mouse.shift_held,
                            self.mouse.ctrl_held,
                        );

                        match action {
                            NavAction::Orbit => {
                                self.camera.yaw -= dx as f32 * self.nav.orbit_sensitivity;
                                self.camera.pitch += dy as f32 * self.nav.orbit_sensitivity;
                                self.request_redraw();
                            }
                            NavAction::Pan => {
                                let speed = self.camera.distance * self.nav.pan_sensitivity;
                                let r = self.camera.screen_right();
                                let u = self.camera.screen_up();
                                let mx = -dx as f32 * speed;
                                let my = dy as f32 * speed;
                                self.camera.target[0] += r[0] * mx + u[0] * my;
                                self.camera.target[1] += r[1] * mx + u[1] * my;
                                self.camera.target[2] += r[2] * mx + u[2] * my;
                                self.request_redraw();
                            }
                            NavAction::Zoom => {
                                let factor = self.nav.scroll_zoom_factor(-dy as f32 * 0.05);
                                self.camera.distance *= factor;
                                self.camera.distance = self.camera.distance.max(0.01);
                                self.request_redraw();
                            }
                            NavAction::None => {}
                        }
                    }
                    self.mouse.last_pos = Some((position.x, position.y));
                }

                WindowEvent::MouseWheel { delta, .. } => {
                    let scroll = match delta {
                        MouseScrollDelta::LineDelta(_, y) => *y,
                        MouseScrollDelta::PixelDelta(pos) => pos.y as f32 * 0.01,
                    };
                    let factor = self.nav.scroll_zoom_factor(scroll);
                    self.camera.distance *= factor;
                    self.camera.distance = self.camera.distance.max(0.01);
                    self.request_redraw();
                }

                _ => {}
            }
        }

        // General window events.
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::KeyboardInput {
                event: key_event, ..
            } if key_event.state == ElementState::Pressed => {
                let ctrl = self.mouse.ctrl_held;
                match key_event.physical_key {
                    PhysicalKey::Code(KeyCode::Escape) => {
                        if self.gui.sketch_mode.is_some() {
                            self.gui.actions.push(GuiAction::CancelSketch);
                        } else {
                            event_loop.exit();
                        }
                    }

                    // Standard views (FreeCAD numpad / regular keys)
                    PhysicalKey::Code(KeyCode::Digit1 | KeyCode::Numpad1) if !ctrl => {
                        self.gui
                            .actions
                            .push(GuiAction::SetStandardView(StandardView::Front));
                    }
                    PhysicalKey::Code(KeyCode::Digit1 | KeyCode::Numpad1) if ctrl => {
                        self.gui
                            .actions
                            .push(GuiAction::SetStandardView(StandardView::Back));
                    }
                    PhysicalKey::Code(KeyCode::Digit3 | KeyCode::Numpad3) if !ctrl => {
                        self.gui
                            .actions
                            .push(GuiAction::SetStandardView(StandardView::Right));
                    }
                    PhysicalKey::Code(KeyCode::Digit3 | KeyCode::Numpad3) if ctrl => {
                        self.gui
                            .actions
                            .push(GuiAction::SetStandardView(StandardView::Left));
                    }
                    PhysicalKey::Code(KeyCode::Digit7 | KeyCode::Numpad7) if !ctrl => {
                        self.gui
                            .actions
                            .push(GuiAction::SetStandardView(StandardView::Top));
                    }
                    PhysicalKey::Code(KeyCode::Digit7 | KeyCode::Numpad7) if ctrl => {
                        self.gui
                            .actions
                            .push(GuiAction::SetStandardView(StandardView::Bottom));
                    }
                    PhysicalKey::Code(KeyCode::Digit0 | KeyCode::Numpad0) => {
                        self.gui
                            .actions
                            .push(GuiAction::SetStandardView(StandardView::Isometric));
                    }
                    PhysicalKey::Code(KeyCode::Digit5 | KeyCode::Numpad5) => {
                        self.gui.actions.push(GuiAction::ToggleProjection);
                    }

                    // V = fit all, D = cycle display
                    PhysicalKey::Code(KeyCode::KeyV) => {
                        self.gui.actions.push(GuiAction::FitAll);
                    }
                    PhysicalKey::Code(KeyCode::KeyD) => {
                        let idx = DisplayMode::ALL
                            .iter()
                            .position(|&m| m == self.display_mode)
                            .unwrap_or(0);
                        let next = DisplayMode::ALL[(idx + 1) % DisplayMode::ALL.len()];
                        self.gui.actions.push(GuiAction::SetDisplayMode(next));
                    }
                    PhysicalKey::Code(KeyCode::KeyG) if !ctrl => {
                        self.gui.actions.push(GuiAction::ToggleGrid);
                    }

                    // Ctrl+Z = Undo, Ctrl+Shift+Z / Ctrl+Y = Redo
                    PhysicalKey::Code(KeyCode::KeyZ) if ctrl && !self.mouse.shift_held => {
                        self.gui.actions.push(GuiAction::Undo);
                    }
                    PhysicalKey::Code(KeyCode::KeyZ) if ctrl && self.mouse.shift_held => {
                        self.gui.actions.push(GuiAction::Redo);
                    }
                    PhysicalKey::Code(KeyCode::KeyY) if ctrl => {
                        self.gui.actions.push(GuiAction::Redo);
                    }

                    // Delete = delete selected
                    PhysicalKey::Code(KeyCode::Delete | KeyCode::Backspace) if !ctrl => {
                        self.gui.actions.push(GuiAction::DeleteSelected);
                    }

                    // Ctrl+N = new, Ctrl+A = select all
                    PhysicalKey::Code(KeyCode::KeyN) if ctrl => {
                        self.gui.actions.push(GuiAction::NewModel);
                    }
                    PhysicalKey::Code(KeyCode::KeyA) if ctrl => {
                        self.gui.actions.push(GuiAction::SelectAll);
                    }

                    // F = fit all, H = toggle visibility of selected
                    PhysicalKey::Code(KeyCode::KeyF) if !ctrl => {
                        self.gui.actions.push(GuiAction::FitAll);
                    }
                    PhysicalKey::Code(KeyCode::KeyH) if !ctrl => {
                        if let Some(id) = self.scene.selected_id() {
                            self.gui.actions.push(GuiAction::ToggleVisibility(id));
                        }
                    }
                    _ => {}
                }
            }

            WindowEvent::Resized(new_size) => {
                if let Some(rt) = &mut self.runtime {
                    rt.gpu.resize(new_size, &mut self.camera);
                    rt.gpu.window.request_redraw();
                }
            }

            WindowEvent::RedrawRequested => {
                self.tick_animation();
                self.poll_background_load();
                self.render_frame();
                self.process_actions();
                if self.gui.request_quit {
                    event_loop.exit();
                }
                if self.mesh_rx.is_some() {
                    self.request_redraw();
                }
            }

            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

/// Normalize an angle to the range (−π, π].
fn wrap_angle(a: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    let mut r = a % tau;
    if r > std::f32::consts::PI {
        r -= tau;
    } else if r <= -std::f32::consts::PI {
        r += tau;
    }
    r
}

/// Snap roll to the nearest 90° (0, ±π/2, ±π).
/// At the exact midpoint (45°) between two 90° multiples, snap toward
/// `prev_roll` — i.e. back to where the user came from.
fn snap_roll_90(roll: f32, prev_roll: f32) -> f32 {
    let r = wrap_angle(roll);
    let pr = wrap_angle(prev_roll);
    let half_pi = std::f32::consts::FRAC_PI_2;
    let q = r / half_pi;
    let frac = q.fract().abs();
    // Check if we're within ~0.6° of the exact midpoint.
    if (frac - 0.5).abs() < 0.01 {
        let lo = q.floor() * half_pi;
        let hi = q.ceil() * half_pi;
        // Pick whichever 90° multiple is closer to prev_roll.
        if (pr - lo).abs() <= (pr - hi).abs() {
            lo
        } else {
            hi
        }
    } else {
        q.round() * half_pi
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub fn run_gui() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = CadApp::new();
    event_loop.run_app(&mut app).unwrap();
}
