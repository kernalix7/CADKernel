//! Full GUI application with egui panels rendered on top of the wgpu 3D
//! viewport.

use crate::gui::{self, GuiAction, GuiState, ViewportInfo};
use crate::nav::{NavAction, NavConfig};
use crate::render::{
    AXIS_X_COLOR, AXIS_Y_COLOR, AXIS_Z_COLOR, Camera, DisplayMode, EDGE_OVERLAY_COLOR,
    GRID_MAJOR_COLOR, GRID_MINOR_COLOR, GpuState, GridConfig, HIDDEN_LINE_COLOR, MouseState,
    NO_SHADE_COLOR, POINT_COLOR, SOLID_COLOR, StandardView, TRANSPARENT_COLOR, Uniforms, Vertex,
    WIRE_COLOR, compute_bounds, cross3, dot3, mesh_to_vertices, normalize3,
};
use cadkernel_io::{
    Mesh, export_gltf, import_obj, import_stl, tessellate_solid, write_obj, write_stl_ascii,
};
use cadkernel_math::Point3;
use cadkernel_modeling::{make_box, make_cylinder, make_sphere};
use cadkernel_topology::{BRepModel, Handle, SolidData};
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
    /// Roll value before the most recent roll-changing action (RollDelta / ScreenOrbit).
    /// Used to resolve the 45° midpoint tie in snap_roll_90.
    prev_roll: f32,
}

impl CadApp {
    fn new() -> Self {
        Self {
            runtime: None,
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

    /// Tick the running camera animation, if any.
    fn tick_animation(&mut self) {
        let now = std::time::Instant::now();
        let dt = now.duration_since(self.last_instant).as_secs_f32();
        self.last_instant = now;

        if let Some(anim) = &mut self.camera_anim {
            let done = anim.tick(dt, &mut self.camera);
            if done {
                self.camera_anim = None;
            } else {
                self.request_redraw();
            }
        }
    }

    // -- action processing -------------------------------------------------

    fn process_actions(&mut self) {
        let actions: Vec<GuiAction> = self.gui.actions.drain(..).collect();
        for action in actions {
            match action {
                GuiAction::NewModel => {
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
                    self.gui.status_message = "New model created".into();
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
                                self.gui.status_message =
                                    format!("Exported glTF → {}", path.display());
                            }
                            Err(e) => {
                                self.gui.status_message = format!("Export error: {e}");
                            }
                        }
                    } else {
                        self.gui.status_message = "No mesh to export".into();
                    }
                }

                GuiAction::CreateBox {
                    width,
                    height,
                    depth,
                } => {
                    let mut model = BRepModel::new();
                    match make_box(&mut model, Point3::ORIGIN, width, height, depth) {
                        Ok(r) => {
                            let mesh = tessellate_solid(&model, r.solid);
                            self.model = model;
                            self.current_solid = Some(r.solid);
                            self.gui.current_file = None;
                            self.gui.status_message =
                                format!("Created box ({width} × {height} × {depth})");
                            self.set_mesh(mesh);
                        }
                        Err(e) => {
                            self.gui.status_message = format!("Error: {e}");
                        }
                    }
                }

                GuiAction::CreateCylinder { radius, height } => {
                    let mut model = BRepModel::new();
                    match make_cylinder(&mut model, Point3::ORIGIN, radius, height, 64) {
                        Ok(r) => {
                            let mesh = tessellate_solid(&model, r.solid);
                            self.model = model;
                            self.current_solid = Some(r.solid);
                            self.gui.current_file = None;
                            self.gui.status_message =
                                format!("Created cylinder (r={radius}, h={height})");
                            self.set_mesh(mesh);
                        }
                        Err(e) => {
                            self.gui.status_message = format!("Error: {e}");
                        }
                    }
                }

                GuiAction::CreateSphere { radius } => {
                    let mut model = BRepModel::new();
                    match make_sphere(&mut model, Point3::ORIGIN, radius, 64, 32) {
                        Ok(r) => {
                            let mesh = tessellate_solid(&model, r.solid);
                            self.model = model;
                            self.current_solid = Some(r.solid);
                            self.gui.current_file = None;
                            self.gui.status_message = format!("Created sphere (r={radius})");
                            self.set_mesh(mesh);
                        }
                        Err(e) => {
                            self.gui.status_message = format!("Error: {e}");
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
                        let new_pitch = (-f2[2]).asin().clamp(
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
            }
        }
    }

    fn load_mesh_file(&mut self, path: &Path) {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext != "stl" && ext != "obj" && ext != "cadk" {
            self.gui.status_message = format!("Unsupported format: .{ext}");
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
                        self.gui.status_message = format!(
                            "Loaded {} ({} solids, {} triangles)",
                            path.display(),
                            solid_count,
                            combined.triangle_count()
                        );
                        self.set_mesh(combined);
                    } else {
                        self.gui.current_file = Some(path.display().to_string());
                        self.gui.status_message =
                            format!("Loaded {} (empty model)", path.display());
                    }
                }
                Err(e) => {
                    self.gui.status_message = format!("Failed to load: {e}");
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
                    self.gui.status_message = format!(
                        "Loaded {} ({} vertices, {} triangles)",
                        path.display(),
                        mesh.vertices.len(),
                        mesh.triangle_count()
                    );
                    self.set_mesh(mesh);
                    true
                }
                Ok(Err(e)) => {
                    self.gui.status_message = format!("Failed to load: {e}");
                    true
                }
                Err(mpsc::TryRecvError::Empty) => false,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.gui.status_message = "Load thread crashed".into();
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
                    self.gui.status_message = format!("Saved → {}", path.display());
                }
                Err(e) => {
                    self.gui.status_message = format!("Save error: {e}");
                }
            }
            return;
        }

        let Some(mesh) = &self.current_mesh else {
            self.gui.status_message = "No mesh to export".into();
            return;
        };

        let result = match ext.as_str() {
            "stl" => std::fs::write(path, write_stl_ascii(mesh, "CADKernel")).map_err(Into::into),
            "obj" => std::fs::write(path, write_obj(mesh)).map_err(Into::into),
            _ => Err(format!("Unsupported export format: .{ext}").into()),
        };

        match result {
            Ok(()) => {
                self.gui.status_message = format!("Exported → {}", path.display());
            }
            Err(e) => {
                let e: Box<dyn std::error::Error> = e;
                self.gui.status_message = format!("Export error: {e}");
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
            camera,
            gui,
            nav,
            model,
            current_mesh,
            display_mode,
            show_grid,
            grid_config,
            ..
        } = self;
        let Some(rt) = runtime else { return };
        let dm = *display_mode;
        let vp_info = ViewportInfo {
            camera,
            display_mode: dm,
            grid_config,
            show_grid: *show_grid,
        };

        // 1. Run egui ---------------------------------------------------
        let raw_input = rt.egui_state.take_egui_input(&rt.gpu.window);
        let full_output = rt.egui_ctx.run(raw_input, |ctx| {
            gui::draw_ui(ctx, gui, nav, &vp_info, model, current_mesh);
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

            // Mesh
            if rt.gpu.num_vertices > 0 {
                pass.set_vertex_buffer(0, rt.gpu.vertex_buffer.slice(..));
                match dm {
                    DisplayMode::AsIs | DisplayMode::Shading => {
                        pass.set_bind_group(
                            0,
                            &rt.gpu.uniform_bind_group,
                            &[GpuState::slot_offset(mesh_slot)],
                        );
                        pass.set_pipeline(&rt.gpu.solid_pipeline);
                        pass.draw(0..rt.gpu.num_vertices, 0..1);
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
                        MouseButton::Left => self.mouse.left_pressed = pressed,
                        MouseButton::Middle => self.mouse.middle_pressed = pressed,
                        MouseButton::Right => self.mouse.right_pressed = pressed,
                        _ => {}
                    }
                }

                WindowEvent::CursorMoved { position, .. } => {
                    if let Some((lx, ly)) = self.mouse.last_pos {
                        let dx = position.x - lx;
                        let dy = position.y - ly;

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
                    PhysicalKey::Code(KeyCode::Escape) => event_loop.exit(),

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
                    PhysicalKey::Code(KeyCode::KeyG) => {
                        self.gui.actions.push(GuiAction::ToggleGrid);
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
