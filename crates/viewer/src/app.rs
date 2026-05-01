//! Full GUI application with egui panels rendered on top of the wgpu 3D
//! viewport.

use crate::gui::{
    self, AssemblyAction, DraftAction, FemAction, GizmoMode, GuiAction, GuiState, MeshAction,
    MirrorPlane, PartAction, PartDesignAction, ReportLevel, SelectedEntity, SelectionMode,
    SketchEntityRef, SketchMode, SketchTool, SketcherAction, SurfaceAction, TechDrawAction,
    ViewportInfo,
};
use crate::scripting::ScriptEngine;
use crate::nav::{NavAction, NavConfig};
use crate::render::{
    AXIS_X_COLOR, AXIS_Y_COLOR, AXIS_Z_COLOR, CLIP_DISABLED, Camera, DisplayMode,
    EDGE_OVERLAY_COLOR, GRID_MAJOR_COLOR, GRID_MINOR_COLOR, GpuState, GridConfig,
    HIDDEN_LINE_COLOR, MouseState, NO_SHADE_COLOR, POINT_COLOR, PRESELECT_STRENGTH, SOLID_COLOR,
    StandardView, TRANSPARENT_COLOR, Uniforms, Vertex, WIRE_COLOR, aabb_in_frustum, compute_bounds,
    cross3, dot3, extract_frustum_planes, mesh_to_vertices, normalize3, sub3,
};
use cadkernel_io::{
    Mesh, export_3mf, export_brep, export_dxf, export_gltf, export_iges, export_ply, export_step,
    import_obj, import_stl, tessellate_solid, write_3mf, write_brep, write_dxf, write_obj,
    write_ply, write_stl_ascii,
};
use cadkernel_math::{Point3, Vec3};
use cadkernel_modeling::{
    BooleanOp, Compound, HatchPattern, Transform as MultiTransformStep, auto_defeaturing,
    boolean_fragments, boolean_op, boolean_op_exact, chamfer_edge, check_geometry, clone_solid,
    compound_filter, compute_mass_properties, connect_shapes, coons_patch, countersunk_hole,
    cutout_shapes, downgrade_solid_faces, draft_hatch, draft_to_sketch, embed_shapes,
    explode_compound, extend_surface, extrude, face_from_wires, filling, fillet_edge, groove,
    hole, linear_pattern, loft, make_arc_wire, make_bezier_wire, make_box, make_bspline_wire,
    make_circle_wire, make_cone, make_cylinder, make_ellipse_wire, make_ellipsoid,
    make_facebinder, make_helix, make_line_draft, make_point, make_polygon_wire, make_prism,
    make_rectangle_wire, make_sphere, make_torus, make_tube, make_wedge, make_wire, mirror_solid,
    mirror_solid_draft, move_solid, multi_transform, offset_wire, pad, path_array, pipe_surface,
    pocket, point_array, points_from_shape, polar_array, project_curve_on_solid,
    rectangular_array, rotate_solid, scale_solid, scale_solid_draft, sections, shape_from_mesh,
    shape_from_text, shell_solid, slice_to_compound, stretch_wire, surface_from_curves, sweep,
    trimex_draft, upgrade_wire_model, wire_to_bspline, wire_to_bspline_convert,
};
use cadkernel_geometry::LineSegment;
use cadkernel_sketch::{
    Constraint, WorkPlane, carbon_copy, decrease_bspline_degree, drag_solve,
    external_projection, extract_profile, geometry_to_bspline, increase_bspline_degree,
    insert_knot, solve,
};
use cadkernel_topology::{BRepModel, EdgeData, FaceData, Handle, SolidData, VertexData};
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

#[doc(hidden)]
pub struct CadApp {
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
    /// Last left-click timestamp for double-click detection (300ms threshold).
    last_click_time: std::time::Instant,
    /// Last left-click screen position for double-click proximity check.
    last_click_pos: (f64, f64),
    /// True while an orbit drag is active (for snap_to_nearest on release).
    was_orbiting: bool,
    /// Per-object vertex ranges in the combined GPU buffer: (id, start, count, color, selected).
    object_ranges: Vec<(crate::scene::ObjectId, u32, u32, [f32; 4], bool)>,
    /// Preselected (hover) object id for highlight.
    preselected_object: Option<crate::scene::ObjectId>,
    /// Preselected sub-element (hover) for overlay highlight.
    preselected_entity: Option<SelectedEntity>,
    /// Frame counter for throttled preselection picking.
    preselect_frame: u32,
    /// Lua scripting engine (lazy-initialized on first use).
    script_engine: Option<ScriptEngine>,
    /// Plugin registry for the kernel plugin system.
    plugin_registry: cadkernel_modeling::PluginRegistry,
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
            last_click_time: std::time::Instant::now(),
            last_click_pos: (0.0, 0.0),
            was_orbiting: false,
            object_ranges: Vec::new(),
            preselected_object: None,
            preselected_entity: None,
            preselect_frame: 0,
            script_engine: None,
            plugin_registry: cadkernel_modeling::PluginRegistry::new(),
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
        // Recalculate AABBs for all objects (transforms change vertex positions)
        for obj in &mut self.scene.objects {
            let (mn, mx) = crate::scene::compute_aabb(&obj.vertices);
            obj.aabb_min = mn;
            obj.aabb_max = mx;
        }
        // Refresh picking data from current model state
        self.scene.refresh_picking_data();
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

    fn sync_object_ranges_metadata(&mut self) {
        for (id, _start, _count, color, selected) in &mut self.object_ranges {
            if let Some(obj) = self.scene.get(*id) {
                *color = obj.color;
                *selected = obj.selected;
            }
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

    /// After orbit ends, if `snap_to_nearest` is enabled and camera is close
    /// to a standard view (within ~10°), animate to that view.
    fn try_snap_to_nearest_view(&mut self) {
        if !self.was_orbiting || !self.nav.snap_to_nearest {
            self.was_orbiting = false;
            return;
        }
        self.was_orbiting = false;

        let threshold = 0.17; // ~10° in radians
        let views = [
            StandardView::Front,
            StandardView::Back,
            StandardView::Right,
            StandardView::Left,
            StandardView::Top,
            StandardView::Bottom,
        ];

        let mut best_view = None;
        let mut best_dist = threshold;
        for view in &views {
            let (vy, vp) = view.yaw_pitch();
            // Angular distance considering yaw wrapping
            let mut dy = (self.camera.yaw - vy).abs();
            if dy > std::f32::consts::PI {
                dy = std::f32::consts::TAU - dy;
            }
            let dp = (self.camera.pitch - vp).abs();
            let dist = (dy * dy + dp * dp).sqrt();
            if dist < best_dist {
                best_dist = dist;
                best_view = Some(*view);
            }
        }

        if let Some(view) = best_view {
            let (mut yaw, pitch) = view.yaw_pitch();
            if matches!(view, StandardView::Top | StandardView::Bottom) {
                yaw = self.camera.yaw;
            }
            let roll = snap_roll_90(self.camera.roll, self.prev_roll);
            self.animate_to(yaw, pitch, roll);
        }
    }

    /// Temporarily adjust `camera.target` based on the active `RotationMode`.
    fn apply_rotation_mode_pivot(&mut self) {
        use crate::nav::RotationMode;
        match self.nav.rotation_mode {
            RotationMode::WindowCenter => {} // default — no change
            RotationMode::ObjectCenter => {
                // Use the center of the selected object as the orbit pivot.
                if let Some(obj) = self.scene.selected_object() {
                    if !obj.vertices.is_empty() {
                        let n = obj.vertices.len() as f32;
                        let (mut cx, mut cy, mut cz) = (0.0f32, 0.0f32, 0.0f32);
                        for v in &obj.vertices {
                            cx += v.position[0];
                            cy += v.position[1];
                            cz += v.position[2];
                        }
                        self.camera.target = [cx / n, cy / n, cz / n];
                    }
                }
            }
            RotationMode::DragAtCursor => {
                // Approximate: cast a ray from cursor to the Z=target.z plane.
                if let Some((mx, my)) = self.mouse.last_pos {
                    let (sw, sh) = if let Some(rt) = &self.runtime {
                        let s = rt.gpu.window.inner_size();
                        (s.width as f32, s.height as f32)
                    } else {
                        (1440.0, 900.0)
                    };
                    let ndc_x = (mx as f32 / sw) * 2.0 - 1.0;
                    let ndc_y = 1.0 - (my as f32 / sh) * 2.0;
                    let r = self.camera.screen_right();
                    let u = self.camera.screen_up();
                    let scale = self.camera.distance * (self.camera.fovy * 0.5).tan();
                    for i in 0..3 {
                        self.camera.target[i] += (r[i] * ndc_x + u[i] * ndc_y) * scale * 0.3;
                    }
                }
            }
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

    /// Double-click handler: if an edge is selected → select edge loop,
    /// if a face is selected → select face loop.
    fn try_sketch_dimension_edit(&mut self) {
        use crate::gui::{DimensionKind, DimensionPopup};
        let sm = match &self.gui.sketch_mode {
            Some(sm) => sm,
            None => return,
        };
        // Find the first dimensional constraint involving selected entities
        let sel = &sm.selected_entities;
        for (ci, c) in sm.sketch.constraints.iter().enumerate() {
            let found: Option<(DimensionKind, f64)> = match c {
                Constraint::Distance(p0, p1, d) => {
                    let has_p = sel.iter().any(|e| matches!(e, SketchEntityRef::Point(i) if *i == p0.0 || *i == p1.0));
                    if has_p { Some((DimensionKind::Distance, *d)) } else { None }
                }
                Constraint::Length(lid, val) => {
                    let has_l = sel.iter().any(|e| matches!(e, SketchEntityRef::Line(i) if *i == lid.0));
                    if has_l { Some((DimensionKind::Length, *val)) } else { None }
                }
                Constraint::Radius(_, _, r) => {
                    let has_c = sel.iter().any(|e| matches!(e, SketchEntityRef::Circle(_) | SketchEntityRef::Arc(_)));
                    if has_c { Some((DimensionKind::Radius, *r)) } else { None }
                }
                Constraint::Diameter(_, _, d) => {
                    let has_c = sel.iter().any(|e| matches!(e, SketchEntityRef::Circle(_) | SketchEntityRef::Arc(_)));
                    if has_c { Some((DimensionKind::Diameter, *d)) } else { None }
                }
                Constraint::Angle(l0, l1, a) => {
                    let has_l = sel.iter().any(|e| matches!(e, SketchEntityRef::Line(i) if *i == l0.0 || *i == l1.0));
                    if has_l { Some((DimensionKind::Angle, a.to_degrees())) } else { None }
                }
                Constraint::HorizontalDistance(p0, p1, d) => {
                    let has_p = sel.iter().any(|e| matches!(e, SketchEntityRef::Point(i) if *i == p0.0 || *i == p1.0));
                    if has_p { Some((DimensionKind::HDistance, *d)) } else { None }
                }
                Constraint::VerticalDistance(p0, p1, d) => {
                    let has_p = sel.iter().any(|e| matches!(e, SketchEntityRef::Point(i) if *i == p0.0 || *i == p1.0));
                    if has_p { Some((DimensionKind::VDistance, *d)) } else { None }
                }
                _ => None,
            };
            if let Some((dk, val)) = found {
                self.gui.dimension_popup = Some(DimensionPopup {
                    kind: dk,
                    value: val,
                    just_opened: true,
                    edit_constraint_index: Some(ci),
                });
                self.gui.status_message = format!("Editing constraint value: {val:.2}");
                return;
            }
        }
        self.gui.status_message = "No dimensional constraint on selection".into();
    }

    fn try_double_click_loop(&mut self) {
        // First do a normal pick to update the selection
        self.try_pick_entity();

        // Then expand the selection to a loop based on what was picked
        if let Some(entity) = self.gui.selected_entities.first().cloned() {
            match entity {
                SelectedEntity::Edge(_) => {
                    let loop_edges = self.compute_edge_loop();
                    if loop_edges.len() > 1 {
                        self.gui.selected_entities = loop_edges
                            .iter()
                            .map(|&eh| SelectedEntity::Edge(eh))
                            .collect();
                        self.gui.status_message = format!("Edge loop: {} edges", loop_edges.len());
                        self.rebuild_scene_gpu();
                    }
                }
                SelectedEntity::Face(_) => {
                    let loop_faces = self.compute_face_loop();
                    if loop_faces.len() > 1 {
                        self.gui.selected_entities = loop_faces
                            .iter()
                            .map(|&fh| SelectedEntity::Face(fh))
                            .collect();
                        self.gui.status_message = format!("Face loop: {} faces", loop_faces.len());
                        self.rebuild_scene_gpu();
                    }
                }
                _ => {}
            }
        }
    }

    /// Cast a ray from cursor and return the 3D surface hit point, if any.
    fn pick_surface_point(&self) -> Option<[f32; 3]> {
        let rt = self.runtime.as_ref()?;
        let size = rt.gpu.window.inner_size();
        let w = size.width as f32;
        let h = size.height as f32;
        if w < 1.0 || h < 1.0 { return None; }
        let (sx, sy) = self.gui.pointer_physical
            .or(self.mouse.last_pos.map(|(x, y)| (x as f32, y as f32)))?;
        let inv_vp = self.camera.inv_view_proj();
        let (origin, dir) = crate::picking::screen_to_ray(sx, sy, w, h, inv_vp);
        // Check vertex snap first, then triangle surface
        let vert_threshold = self.camera.distance * 0.012;
        for obj in self.scene.visible_objects() {
            if let Some((idx, _t)) = crate::picking::pick_vertex(origin, dir, &obj.vertex_positions, vert_threshold) {
                return Some(obj.vertex_positions[idx]);
            }
        }
        for obj in self.scene.visible_objects() {
            if let Some(hit) = crate::picking::pick_triangle(origin, dir, &obj.mesh.vertices, &obj.mesh.indices) {
                return Some(hit.hit_point);
            }
        }
        None
    }

    fn try_pick_entity(&mut self) {
        let Some(rt) = &self.runtime else { return };
        let size = rt.gpu.window.inner_size();
        let w = size.width as f32;
        let h = size.height as f32;
        if w < 1.0 || h < 1.0 { return; }

        let winit_pos = self.mouse.last_pos.map(|(x, y)| (x as f32, y as f32));
        let egui_pos = self.gui.pointer_physical;
        let (sx, sy) = egui_pos
            .or(winit_pos)
            .unwrap_or_default();

        let inv_vp = self.camera.inv_view_proj();
        let (origin, dir) = crate::picking::screen_to_ray(sx, sy, w, h, inv_vp);

        // Edge/vertex threshold scales with camera distance for consistent screen-space picking.
        // Vertex is tighter than edge to reduce accidental vertex grabs.
        let edge_threshold = self.camera.distance * 0.015;
        let vert_threshold = self.camera.distance * 0.012;

        // Always use auto-pick: automatically detect vertex > edge > face > solid.
        // SelectionMode acts as a filter hint — if a specific mode is set, prioritize
        // that element type but still fall through to others if nothing is found.
        self.pick_auto(origin, dir, edge_threshold, vert_threshold);
    }

    /// Automatic picking: try vertex → edge → face → solid (most specific first).
    /// In Solid mode, this tries sub-element picks first. If a sub-element is
    /// hit, it selects that; otherwise falls back to solid selection.
    fn pick_auto(
        &mut self,
        origin: [f32; 3],
        dir: [f32; 3],
        edge_threshold: f32,
        vert_threshold: f32,
    ) {
        // Gather pick results without borrowing self mutably
        let mut vert_hit: Option<(crate::scene::ObjectId, Handle<VertexData>, usize, f32, String)> = None;
        let mut edge_hit: Option<(crate::scene::ObjectId, Handle<EdgeData>, usize, f32, String)> = None;
        let mut face_hit: Option<(crate::scene::ObjectId, Handle<FaceData>, f32, String)> = None;

        for obj in self.scene.visible_objects() {
            if let Some((idx, t)) = crate::picking::pick_vertex(
                origin, dir, &obj.vertex_positions, vert_threshold,
            ) {
                if let Some(&vh) = obj.vertex_handles.get(idx) {
                    let is_closer = vert_hit.as_ref().is_none_or(|(_, _, _, best_t, _)| t < *best_t);
                    if is_closer {
                        vert_hit = Some((obj.id, vh, idx, t, obj.name.clone()));
                    }
                }
            }
            if let Some((idx, t)) = crate::picking::pick_edge(
                origin, dir, &obj.edge_positions, edge_threshold,
            ) {
                if let Some(&eh) = obj.edge_handles.get(idx) {
                    let is_closer = edge_hit.as_ref().is_none_or(|(_, _, _, best_t, _)| t < *best_t);
                    if is_closer {
                        edge_hit = Some((obj.id, eh, idx, t, obj.name.clone()));
                    }
                }
            }
            if let Some(hit) = crate::picking::pick_triangle(
                origin, dir, &obj.mesh.vertices, &obj.mesh.indices,
            ) {
                if let Some(fh) = lookup_face(hit.triangle_index, &obj.face_tri_map) {
                    let is_closer = face_hit.as_ref().is_none_or(|(_, _, best_t, _)| hit.distance < *best_t);
                    if is_closer {
                        face_hit = Some((obj.id, fh, hit.distance, obj.name.clone()));
                    }
                }
            }
        }

        // Apply result: vertex > edge > face > solid
        if let Some((obj_id, vert_h, idx, _t, name)) = vert_hit {
            self.select_object_for_pick(obj_id);
            let entity = SelectedEntity::Vertex(vert_h);
            if self.mouse.ctrl_held {
                toggle_entity(&mut self.gui.selected_entities, entity);
            } else {
                self.gui.selected_entities = vec![entity];
            }
            self.gui.status_message = format!("Vertex {idx} of {name}");
            self.sync_object_ranges_metadata();
        } else if let Some((obj_id, edge_h, idx, _t, name)) = edge_hit {
            self.select_object_for_pick(obj_id);
            let entity = SelectedEntity::Edge(edge_h);
            if self.mouse.ctrl_held {
                toggle_entity(&mut self.gui.selected_entities, entity);
            } else {
                self.gui.selected_entities = vec![entity];
            }
            self.gui.status_message = format!("Edge {idx} of {name}");
            self.sync_object_ranges_metadata();
        } else if let Some((obj_id, face_h, _t, name)) = face_hit {
            self.select_object_for_pick(obj_id);
            let entity = SelectedEntity::Face(face_h);
            if self.mouse.ctrl_held {
                toggle_entity(&mut self.gui.selected_entities, entity);
            } else {
                self.gui.selected_entities = vec![entity];
            }
            self.gui.status_message = format!("Face of {name}");
            self.sync_object_ranges_metadata();
        } else {
            self.pick_solid(origin, dir);
        }
    }

    /// Helper: select an object and load its model data.
    fn select_object_for_pick(&mut self, obj_id: crate::scene::ObjectId) {
        self.scene.select_single(obj_id);
        if let Some(o) = self.scene.get(obj_id) {
            self.model = o.model.clone();
            self.current_solid = Some(o.solid);
            self.current_mesh = Some(o.mesh.clone());
        }
    }

    fn pick_solid(&mut self, origin: [f32; 3], dir: [f32; 3]) {
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
            if self.mouse.ctrl_held {
                self.scene.toggle_select(obj_id);
            } else {
                self.scene.select_single(obj_id);
            }
            if let Some(obj) = self.scene.get(obj_id) {
                self.model = obj.model.clone();
                self.current_solid = Some(obj.solid);
                self.current_mesh = Some(obj.mesh.clone());
                let n_sel = self.scene.selected_ids().len();
                self.gui.status_message = if n_sel > 1 {
                    format!("{n_sel} objects selected (last: {})", obj.name)
                } else {
                    format!("Selected: {} (dist {dist:.2})", obj.name)
                };
            }
            self.sync_object_ranges_metadata();
        } else {
            self.scene.deselect_all();
            self.gui.selected_entities.clear();
            self.gui.status_message = "Selection cleared".into();
            self.sync_object_ranges_metadata();
        }
    }


    // -- recent files ---------------------------------------------------------

    fn add_recent_file(&mut self, path: &str) {
        self.gui.recent_files.retain(|p| p != path);
        self.gui.recent_files.insert(0, path.to_string());
        if self.gui.recent_files.len() > 10 {
            self.gui.recent_files.truncate(10);
        }
    }

    // -- preselection (hover) ------------------------------------------------

    fn update_preselection(&mut self) {
        let (sx, sy) = self.gui.pointer_physical
            .or(self.mouse.last_pos.map(|(x, y)| (x as f32, y as f32)))
            .unwrap_or_default();
        let Some(rt) = &self.runtime else { return };
        let size = rt.gpu.window.inner_size();
        let w = size.width as f32;
        let h = size.height as f32;
        if w < 1.0 || h < 1.0 { return; }

        let inv_vp = self.camera.inv_view_proj();
        let (origin, dir) = crate::picking::screen_to_ray(sx, sy, w, h, inv_vp);

        let edge_threshold = self.camera.distance * 0.015;
        let vert_threshold = self.camera.distance * 0.012;

        let mut new_presel: Option<crate::scene::ObjectId> = None;
        let mut new_entity: Option<SelectedEntity> = None;
        let mut best_vertex: Option<(crate::scene::ObjectId, SelectedEntity, f32)> = None;
        let mut best_edge: Option<(crate::scene::ObjectId, SelectedEntity, f32)> = None;
        let mut best_face: Option<(crate::scene::ObjectId, SelectedEntity, f32)> = None;
        let mut best_solid: Option<(crate::scene::ObjectId, f32)> = None;

        // Auto-preselection: vertex > edge > face > solid (same as auto-pick)
        for obj in self.scene.visible_objects() {
            if let Some((idx, t)) = crate::picking::pick_vertex(origin, dir, &obj.vertex_positions, vert_threshold) {
                if let Some(&vh) = obj.vertex_handles.get(idx) {
                    let is_closer = best_vertex.as_ref().is_none_or(|(_, _, best_t)| t < *best_t);
                    if is_closer {
                        best_vertex = Some((obj.id, SelectedEntity::Vertex(vh), t));
                    }
                }
            }
            if let Some((idx, t)) = crate::picking::pick_edge(origin, dir, &obj.edge_positions, edge_threshold) {
                if let Some(&eh) = obj.edge_handles.get(idx) {
                    let is_closer = best_edge.as_ref().is_none_or(|(_, _, best_t)| t < *best_t);
                    if is_closer {
                        best_edge = Some((obj.id, SelectedEntity::Edge(eh), t));
                    }
                }
            }
            if let Some(hit) = crate::picking::pick_triangle(origin, dir, &obj.mesh.vertices, &obj.mesh.indices) {
                if let Some(fh) = lookup_face(hit.triangle_index, &obj.face_tri_map) {
                    let is_closer = best_face.as_ref().is_none_or(|(_, _, best_t)| hit.distance < *best_t);
                    if is_closer {
                        best_face = Some((obj.id, SelectedEntity::Face(fh), hit.distance));
                    }
                } else {
                    let is_closer = best_solid.as_ref().is_none_or(|(_, best_t)| hit.distance < *best_t);
                    if is_closer {
                        best_solid = Some((obj.id, hit.distance));
                    }
                }
            }
        }

        if let Some((obj_id, entity, _)) = best_vertex.or(best_edge).or(best_face) {
            new_presel = Some(obj_id);
            new_entity = Some(entity);
        } else if let Some((obj_id, _)) = best_solid {
            new_presel = Some(obj_id);
        }

        let obj_changed = new_presel != self.preselected_object;
        let entity_changed = new_entity != self.preselected_entity;
        if obj_changed {
            self.preselected_object = new_presel;
            self.gui.preselected_object_id = new_presel;
        }
        if entity_changed {
            self.preselected_entity = new_entity.clone();
            self.gui.preselected_entity = new_entity;
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
        self.gui.log(ReportLevel::Info, s.clone());
        self.gui.toasts.push(gui::Toast {
            level: gui::ToastLevel::Success,
            message: s,
            created_at: std::time::Instant::now(),
        });
    }

    fn log_warning(&mut self, msg: impl Into<String>) {
        let s: String = msg.into();
        self.gui.status_message = s.clone();
        self.gui.log(ReportLevel::Warning, s.clone());
        self.gui.toasts.push(gui::Toast {
            level: gui::ToastLevel::Warning,
            message: s,
            created_at: std::time::Instant::now(),
        });
    }

    fn log_error(&mut self, msg: impl Into<String>) {
        let s: String = msg.into();
        self.gui.status_message = s.clone();
        self.gui.log(ReportLevel::Error, s.clone());
        self.gui.toasts.push(gui::Toast {
            level: gui::ToastLevel::Error,
            message: s,
            created_at: std::time::Instant::now(),
        });
    }

    /// Lazily initialize the Lua script engine on first use.
    fn ensure_script_engine(&mut self) {
        if self.script_engine.is_none() {
            match ScriptEngine::new() {
                Ok(engine) => {
                    self.script_engine = Some(engine);
                }
                Err(e) => {
                    self.log_error(format!("Failed to initialize Lua engine: {e}"));
                }
            }
        }
    }

    /// Register built-in plugins if the registry is empty.
    fn register_builtin_plugins(&mut self) {
        if self.plugin_registry.is_empty() {
            self.plugin_registry.register(
                Box::new(cadkernel_modeling::ValidationPlugin::new()),
            );
            self.plugin_registry.register(
                Box::new(cadkernel_modeling::AutoNamingPlugin::new()),
            );
            self.plugin_registry.register(
                Box::new(cadkernel_modeling::StatisticsPlugin::new()),
            );
        }
    }

    /// Mirror the plugin registry contents into egui temp data so the
    /// plugin manager dialog can display them without borrowing the registry.
    fn sync_plugin_list_to_ui(&self) {
        let list: Vec<(usize, String, String, String)> = self
            .plugin_registry
            .list()
            .iter()
            .map(|ps| {
                let state_str = match &ps.state {
                    cadkernel_modeling::PluginState::Unloaded => "Unloaded".to_string(),
                    cadkernel_modeling::PluginState::Loaded => "Loaded".to_string(),
                    cadkernel_modeling::PluginState::Active => "Active".to_string(),
                    cadkernel_modeling::PluginState::Error(e) => format!("Error: {e}"),
                };
                (ps.id, ps.info.name.clone(), ps.info.version.clone(), state_str)
            })
            .collect();

        // Store into the egui context temp data (accessible by the dialog).
        if let Some(rt) = &self.runtime {
            rt.egui_ctx.data_mut(|d| {
                d.insert_temp(egui::Id::new("plugin_list_data"), list);
            });
        }
    }

    // -- action processing -------------------------------------------------

    fn process_actions(&mut self) {
        let actions: Vec<GuiAction> = self.gui.actions.drain(..).collect();
        for action in actions {
            match action {
                GuiAction::NewModel => {
                    self.snapshot_before("New model");
                    self.scene = crate::scene::Scene::new();
                    self.model = BRepModel::new();
                    self.current_mesh = None;
                    self.current_solid = None;
                    self.object_ranges.clear();
                    self.preselected_object = None;
                    self.preselected_entity = None;
                    self.gui.preselected_entity = None;
                    self.gui.preselected_object_id = None;
                    self.vertices.clear();
                    self.grid_config.set_object_extent(0.0);
                    self.grid_config.force_rebuild();
                    self.grid_config.update_for_camera(self.camera.distance);
                    if let Some(rt) = &mut self.runtime {
                        rt.gpu.update_mesh(&self.vertices);
                        rt.gpu.rebuild_grid(&self.grid_config);
                    }
                    self.gui.invalidate_cache();
                    self.gui.selected_entities.clear();
                    self.gui.current_file = None;
                    self.log_info("New model created");
                }

                GuiAction::OpenFile(path) | GuiAction::ImportFile(path) => {
                    let ext = path.extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    if ext == "cadk" {
                        self.load_scene_file(&path);
                    } else {
                        self.load_mesh_file(&path);
                    }
                    // Add to recent files
                    let path_str = path.to_string_lossy().to_string();
                    self.gui.recent_files.retain(|p| *p != path_str);
                    self.gui.recent_files.insert(0, path_str);
                    if self.gui.recent_files.len() > 10 {
                        self.gui.recent_files.truncate(10);
                    }
                }

                GuiAction::ClearRecentFiles => {
                    self.gui.recent_files.clear();
                }

                GuiAction::SaveFile(path) => {
                    let ext = path.extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    if ext == "cadk" {
                        self.save_scene_file(&path);
                    } else {
                        self.export_mesh_to(&path);
                    }
                }

                GuiAction::ExportStl(path) => {
                    self.export_mesh_to(&path);
                }
                GuiAction::ExportStlWithOptions { path, binary, scale } => {
                    if let Some(obj) = self.scene.selected_object() {
                        let mut mesh = obj.mesh.clone();
                        if (scale - 1.0).abs() > 1e-9 {
                            for v in &mut mesh.vertices {
                                v.x *= scale;
                                v.y *= scale;
                                v.z *= scale;
                            }
                        }
                        let result: Result<(), Box<dyn std::error::Error>> = if binary {
                            cadkernel_io::export_stl_binary(&mesh, &path).map_err(|e| e.into())
                        } else {
                            cadkernel_io::export_stl_ascii(&mesh, &path, &obj.name).map_err(|e| e.into())
                        };
                        match result {
                            Ok(()) => {
                                let fmt = if binary { "Binary" } else { "ASCII" };
                                self.log_info(format!("Exported {fmt} STL → {} (scale: {scale:.2}x)", path.display()));
                            }
                            Err(e) => self.log_error(format!("STL export error: {e}")),
                        }
                    } else {
                        self.log_warning("No object selected for export");
                    }
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

                GuiAction::FocusObject(id) => {
                    // Fit camera to the AABB of a single object (tree double-click).
                    if let Some(obj) = self.scene.get_object(id) {
                        if !obj.vertices.is_empty() {
                            self.camera.fit_to_bounds(obj.aabb_min, obj.aabb_max);
                            self.gui.status_message = format!("Focused on '{}'", obj.name);
                        }
                    }
                }

                GuiAction::ToggleProjection => {
                    self.camera.toggle_projection();
                    let label = match self.camera.projection {
                        crate::render::Projection::Perspective => "Perspective",
                        crate::render::Projection::Orthographic => "Orthographic",
                    };
                    self.gui.status_message = format!("Projection: {label}");
                }

                GuiAction::SetGizmoMode(mode) => {
                    self.gui.gizmo_mode = if self.gui.gizmo_mode == mode {
                        GizmoMode::None
                    } else {
                        mode
                    };
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
                    self.gui.status_message = msg.clone();
                    self.gui.toasts.push(gui::Toast {
                        level: gui::ToastLevel::Info,
                        message: msg,
                        created_at: std::time::Instant::now(),
                    });
                }

                // -- Sketcher workbench --
                GuiAction::Sketcher(action) => self.process_sketcher_action(action),

                // -- TechDraw workbench --
                GuiAction::TechDraw(action) => self.process_techdraw_action(action),

                // -- Mesh operations --
                GuiAction::Mesh(action) => self.process_mesh_action(action),

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
                        let edge_pairs = selected_edge_pairs(&self.gui.selected_entities, &self.model)
                            .unwrap_or_else(|| {
                                let pairs = self.collect_edge_pairs(solid);
                                pairs.first().copied().into_iter().collect()
                            });
                        if edge_pairs.is_empty() {
                            self.gui.status_message = "No edges found for fillet".into();
                        } else {
                            let mut current = solid;
                            let mut ok_count = 0usize;
                            for (v1, v2) in &edge_pairs {
                                match fillet_edge(&mut self.model, current, *v1, *v2, radius) {
                                    Ok(r) => { current = r.solid; ok_count += 1; }
                                    Err(e) => { self.log_error(format!("Fillet error: {e}")); }
                                }
                            }
                            if ok_count > 0 {
                                let mesh = tessellate_solid(&self.model, current);
                                self.current_solid = Some(current);
                                self.log_info(format!("Fillet: r={radius:.2} ({ok_count} edge(s))"));
                                self.set_mesh(mesh);
                                self.gui.selected_entities.clear();
                            }
                        }
                    } else {
                        self.gui.status_message = "No solid for fillet".into();
                    }
                }

                GuiAction::ChamferAllEdges { distance } => {
                    if let Some(solid) = self.current_solid {
                        let edge_pairs = selected_edge_pairs(&self.gui.selected_entities, &self.model)
                            .unwrap_or_else(|| {
                                let pairs = self.collect_edge_pairs(solid);
                                pairs.first().copied().into_iter().collect()
                            });
                        if edge_pairs.is_empty() {
                            self.gui.status_message = "No edges found for chamfer".into();
                        } else {
                            let mut current = solid;
                            let mut ok_count = 0usize;
                            for (v1, v2) in &edge_pairs {
                                match chamfer_edge(&mut self.model, current, *v1, *v2, distance) {
                                    Ok(r) => { current = r.solid; ok_count += 1; }
                                    Err(e) => { self.log_error(format!("Chamfer error: {e}")); }
                                }
                            }
                            if ok_count > 0 {
                                let mesh = tessellate_solid(&self.model, current);
                                self.current_solid = Some(current);
                                self.log_info(format!("Chamfer: d={distance:.2} ({ok_count} edge(s))"));
                                self.set_mesh(mesh);
                                self.gui.selected_entities.clear();
                            }
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
                    self.scene.select_all();
                    let count = self.scene.selected_ids().len();
                    self.gui.status_message = format!("{count} object(s) selected");
                    self.rebuild_scene_gpu();
                }
                GuiAction::DeselectAll => {
                    self.scene.deselect_all();
                    self.gui.selected_entities.clear();
                    self.gui.status_message = "Selection cleared".into();
                    self.rebuild_scene_gpu();
                }
                GuiAction::SetSelectionMode(mode) => {
                    self.gui.selection_mode = mode;
                    self.gui.selected_entities.clear();
                    self.gui.status_message = format!("Selection mode: {:?}", mode);
                }
                GuiAction::SelectEdgeLoop => {
                    let loop_edges = self.compute_edge_loop();
                    if loop_edges.is_empty() {
                        self.gui.status_message = "No edge loop found".into();
                    } else {
                        let count = loop_edges.len();
                        self.gui.selected_entities = loop_edges
                            .into_iter()
                            .map(SelectedEntity::Edge)
                            .collect();
                        self.gui.status_message = format!("Edge loop: {count} edges");
                    }
                }
                GuiAction::SelectEdgeRing => {
                    let ring_edges = self.compute_edge_ring();
                    if ring_edges.is_empty() {
                        self.gui.status_message = "No edge ring found".into();
                    } else {
                        let count = ring_edges.len();
                        self.gui.selected_entities = ring_edges
                            .into_iter()
                            .map(SelectedEntity::Edge)
                            .collect();
                        self.gui.status_message = format!("Edge ring: {count} edges");
                    }
                }
                GuiAction::SelectFaceLoop => {
                    let loop_faces = self.compute_face_loop();
                    if loop_faces.is_empty() {
                        self.gui.status_message = "No face loop found".into();
                    } else {
                        let count = loop_faces.len();
                        self.gui.selected_entities = loop_faces
                            .into_iter()
                            .map(SelectedEntity::Face)
                            .collect();
                        self.gui.status_message = format!("Face loop: {count} faces");
                    }
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
                GuiAction::RenameObject(id, new_name) => {
                    if let Some(obj) = self.scene.get_mut(id) {
                        let old = obj.name.clone();
                        obj.name = new_name.clone();
                        self.log_info(format!("Renamed: {old} → {new_name}"));
                    }
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

                // -- Task preview (live update) --
                GuiAction::TaskPreviewUpdate(mut task) => {
                    use crate::scene::CreationParams;
                    // Convert ActiveTask to CreationParams
                    let params = match &task {
                        gui::task_panel::ActiveTask::Box { width, height, depth, .. } =>
                            Some(CreationParams::Box { width: *width, height: *height, depth: *depth }),
                        gui::task_panel::ActiveTask::Cylinder { radius, height, .. } =>
                            Some(CreationParams::Cylinder { radius: *radius, height: *height }),
                        gui::task_panel::ActiveTask::Sphere { radius, .. } =>
                            Some(CreationParams::Sphere { radius: *radius }),
                        gui::task_panel::ActiveTask::Cone { base_radius, top_radius, height, .. } =>
                            Some(CreationParams::Cone { base_radius: *base_radius, top_radius: *top_radius, height: *height }),
                        gui::task_panel::ActiveTask::Torus { major_radius, minor_radius, .. } =>
                            Some(CreationParams::Torus { major_radius: *major_radius, minor_radius: *minor_radius }),
                        gui::task_panel::ActiveTask::Tube { outer_radius, inner_radius, height, .. } =>
                            Some(CreationParams::Tube { outer_radius: *outer_radius, inner_radius: *inner_radius, height: *height }),
                        gui::task_panel::ActiveTask::Prism { radius, height, sides, .. } =>
                            Some(CreationParams::Prism { radius: *radius, height: *height, sides: *sides }),
                        gui::task_panel::ActiveTask::Wedge { dx, dy, dz, dx2, dy2, .. } =>
                            Some(CreationParams::Wedge { dx: *dx, dy: *dy, dz: *dz, dx2: *dx2, dy2: *dy2 }),
                        gui::task_panel::ActiveTask::Ellipsoid { rx, ry, rz, .. } =>
                            Some(CreationParams::Ellipsoid { rx: *rx, ry: *ry, rz: *rz }),
                        gui::task_panel::ActiveTask::Helix { radius, pitch, turns, tube_radius, .. } =>
                            Some(CreationParams::Helix { radius: *radius, pitch: *pitch, turns: *turns, tube_radius: *tube_radius }),
                        gui::task_panel::ActiveTask::Fillet { radius, .. } =>
                            Some(CreationParams::Fillet { radius: *radius }),
                        gui::task_panel::ActiveTask::Chamfer { distance, .. } =>
                            Some(CreationParams::Chamfer { distance: *distance }),
                        gui::task_panel::ActiveTask::Shell { thickness, .. } =>
                            Some(CreationParams::Shell { thickness: *thickness }),
                        gui::task_panel::ActiveTask::MirrorOp { plane, .. } =>
                            Some(CreationParams::Mirror { plane: *plane }),
                        gui::task_panel::ActiveTask::Pattern { count, spacing, axis, .. } =>
                            Some(CreationParams::Pattern { count: *count, spacing: *spacing, axis: *axis }),
                        gui::task_panel::ActiveTask::Sprocket { teeth, roller_diameter, pitch, bore, .. } =>
                            Some(CreationParams::Sprocket { teeth: *teeth, roller_diameter: *roller_diameter, pitch: *pitch, bore: *bore }),
                        gui::task_panel::ActiveTask::InvoluteGear { teeth, module_val, pressure_angle, .. } =>
                            Some(CreationParams::InvoluteGear { teeth: *teeth, module_val: *module_val, pressure_angle: *pressure_angle }),
                        gui::task_panel::ActiveTask::DraftLine { length, angle, .. } =>
                            Some(CreationParams::DraftLine { length: *length, angle: *angle }),
                        gui::task_panel::ActiveTask::DraftCircle { radius, .. } =>
                            Some(CreationParams::DraftCircle { radius: *radius }),
                        gui::task_panel::ActiveTask::DraftRectangle { width, height, .. } =>
                            Some(CreationParams::DraftRectangle { width: *width, height: *height }),
                        gui::task_panel::ActiveTask::DraftPolygon { radius, sides, .. } =>
                            Some(CreationParams::DraftPolygon { radius: *radius, sides: *sides }),
                        gui::task_panel::ActiveTask::DraftArc { radius, start_angle, end_angle, .. } =>
                            Some(CreationParams::DraftArc { radius: *radius, start_angle: *start_angle, end_angle: *end_angle }),
                        gui::task_panel::ActiveTask::DraftEllipse { rx, ry, .. } =>
                            Some(CreationParams::DraftEllipse { rx: *rx, ry: *ry }),
                        gui::task_panel::ActiveTask::SurfacePipe { radius, length, .. } =>
                            Some(CreationParams::SurfacePipe { radius: *radius, length: *length }),
                        gui::task_panel::ActiveTask::SurfaceRuled { width, depth, offset, .. } =>
                            Some(CreationParams::SurfaceRuled { width: *width, depth: *depth, offset: *offset }),
                        gui::task_panel::ActiveTask::BooleanOp { op_type, width, height, depth, offset_x, offset_y, offset_z, .. } =>
                            Some(CreationParams::BooleanOp { op_type: *op_type, width: *width, height: *height, depth: *depth, offset_x: *offset_x, offset_y: *offset_y, offset_z: *offset_z }),
                        gui::task_panel::ActiveTask::ScaleOp { factor, .. } =>
                            Some(CreationParams::ScaleOp { factor: *factor }),
                        // Feature operations don't have direct primitive rebuilds
                        gui::task_panel::ActiveTask::Pad { .. }
                        | gui::task_panel::ActiveTask::Pocket { .. }
                        | gui::task_panel::ActiveTask::Hole { .. }
                        | gui::task_panel::ActiveTask::Groove { .. }
                        | gui::task_panel::ActiveTask::FemMesh { .. } => None,
                    };

                    if let Some(params) = params {
                        if let Some((model, solid)) = rebuild_object_from_params(&params) {
                            if let Some(pid) = task.preview_id() {
                                // Update existing preview
                                if let Some(obj) = self.scene.get_mut(pid) {
                                    let mesh = cadkernel_io::tessellate_solid(&model, solid);
                                    obj.vertices = crate::render::mesh_to_vertices(&mesh);
                                    obj.mesh = mesh;
                                    obj.model = model;
                                    obj.solid = solid;
                                    obj.params = Some(params);
                                }
                            } else {
                                // Create new preview object
                                let name = format!("{} (preview)", task.title());
                                let id = self.scene.add_object(name, model, solid, Some(params));
                                task.set_preview_id(id);
                                self.scene.select_single(id);
                            }
                            self.rebuild_scene_gpu();
                        }
                    }
                    self.gui.active_task = Some(task);
                }

                // -- Color change --
                GuiAction::SetObjectColor { id, color } => {
                    if let Some(obj) = self.scene.get_mut(id) {
                        obj.color = color;
                    }
                    self.rebuild_scene_gpu();
                }

                // -- Parametric rebuild --
                GuiAction::RebuildObject { id, params } => {
                    self.snapshot_before("Edit parameters");
                    let result = rebuild_object_from_params(&params);
                    if let Some((model, solid)) = result {
                        if let Some(obj) = self.scene.get_mut(id) {
                            let mesh = cadkernel_io::tessellate_solid(&model, solid);
                            obj.vertices = crate::render::mesh_to_vertices(&mesh);
                            obj.mesh = mesh;
                            obj.model = model;
                            obj.solid = solid;
                            obj.params = Some(params);
                        }
                        self.rebuild_scene_gpu();
                    }
                }

                // -- Transform operations --
                GuiAction::MoveObject { id, dx, dy, dz } => {
                    self.snapshot_before("Move object");
                    let dx = self.nav.snap_3d(dx);
                    let dy = self.nav.snap_3d(dy);
                    let dz = self.nav.snap_3d(dz);
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
                        let rotate_pt = |p: cadkernel_math::Point3| -> cadkernel_math::Point3 {
                            match axis {
                                0 => cadkernel_math::Point3::new(p.x, p.y * ca - p.z * sa, p.y * sa + p.z * ca),
                                1 => cadkernel_math::Point3::new(p.x * ca + p.z * sa, p.y, -p.x * sa + p.z * ca),
                                _ => cadkernel_math::Point3::new(p.x * ca - p.y * sa, p.x * sa + p.y * ca, p.z),
                            }
                        };
                        let rotate_vec = |v: cadkernel_math::Vec3| -> cadkernel_math::Vec3 {
                            match axis {
                                0 => cadkernel_math::Vec3::new(v.x, v.y * ca - v.z * sa, v.y * sa + v.z * ca),
                                1 => cadkernel_math::Vec3::new(v.x * ca + v.z * sa, v.y, -v.x * sa + v.z * ca),
                                _ => cadkernel_math::Vec3::new(v.x * ca - v.y * sa, v.x * sa + v.y * ca, v.z),
                            }
                        };
                        for v in &mut obj.mesh.vertices {
                            *v = rotate_pt(*v);
                        }
                        for n in &mut obj.mesh.normals {
                            *n = rotate_vec(*n);
                        }
                        for v in obj.model.vertices.iter_mut() {
                            v.1.point = rotate_pt(v.1.point);
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
                        for v in obj.model.vertices.iter_mut() {
                            let p = &mut v.1.point;
                            *p = cadkernel_math::Point3::new(p.x * factor, p.y * factor, p.z * factor);
                        }
                        obj.vertices = crate::render::mesh_to_vertices(&obj.mesh);
                        let name = obj.name.clone();
                        self.rebuild_scene_gpu();
                        self.log_info(format!("Scaled {name} by {factor:.2}×"));
                    }
                }

                // -- Multi-select --
                GuiAction::ToggleSelect(id) => {
                    self.scene.toggle_select(id);
                    self.sync_object_ranges_metadata();
                }

                // -- Scene boolean operations --
                GuiAction::BooleanSceneUnion | GuiAction::BooleanSceneSubtract | GuiAction::BooleanSceneIntersect => {
                    let ids = self.scene.selected_ids();
                    if ids.len() >= 2 {
                        let op = match action {
                            GuiAction::BooleanSceneUnion => BooleanOp::Union,
                            GuiAction::BooleanSceneSubtract => BooleanOp::Difference,
                            _ => BooleanOp::Intersection,
                        };
                        let obj_a = self.scene.get(ids[0]).cloned();
                        let obj_b = self.scene.get(ids[1]).cloned();
                        if let (Some(a), Some(b)) = (obj_a, obj_b) {
                            self.snapshot_before("Boolean operation");
                            match boolean_op(&a.model, a.solid, &b.model, b.solid, op) {
                                Ok(result_model) => {
                                    let first_solid = result_model.solids.iter().next().map(|(h, _)| h);
                                    if let Some(sh) = first_solid {
                                        let op_name = match op {
                                            BooleanOp::Union => "Union",
                                            BooleanOp::Difference => "Subtract",
                                            BooleanOp::Intersection => "Intersect",
                                        };
                                        self.scene.remove_object(ids[0]);
                                        self.scene.remove_object(ids[1]);
                                        self.add_to_scene(
                                            &format!("{op_name}({}, {})", a.name, b.name),
                                            result_model, sh,
                                            Some(crate::scene::CreationParams::Boolean { op: op_name.into() }),
                                        );
                                        self.log_info(format!("Boolean {op_name} completed"));
                                    }
                                }
                                Err(e) => self.log_error(format!("Boolean error: {e}")),
                            }
                        }
                    } else {
                        self.log_warning("Select exactly 2 objects for boolean operation");
                    }
                }

                GuiAction::DeleteSelected => {
                    let selected = self.scene.selected_ids();
                    if !selected.is_empty() {
                        self.snapshot_before("Delete selected");
                        let count = selected.len();
                        for id in selected {
                            self.scene.remove_object(id);
                        }
                        self.current_solid = None;
                        self.current_mesh = None;
                        self.model = BRepModel::new();
                        self.gui.selected_entities.clear();
                        self.preselected_object = None;
                        self.preselected_entity = None;
                        self.gui.preselected_entity = None;
                        self.gui.preselected_object_id = None;
                        self.rebuild_scene_gpu();
                        self.log_info(format!("Deleted {count} object(s)"));
                    } else if self.current_solid.is_some() {
                        self.snapshot_before("Delete solid");
                        self.model = BRepModel::new();
                        self.current_solid = None;
                        self.current_mesh = None;
                        self.vertices.clear();
                        if let Some(rt) = &mut self.runtime {
                            rt.gpu.update_mesh(&self.vertices);
                        }
                        self.gui.invalidate_cache();
                        self.gui.selected_entities.clear();
                        self.log_info("Deleted solid");
                    } else {
                        self.gui.status_message = "Nothing to delete".into();
                    }
                }

                // -- Part workbench --
                GuiAction::Part(action) => self.process_part_action(action),

                // -- PartDesign workbench --
                GuiAction::PartDesign(action) => self.process_part_design_action(action),

                // -- Assembly workbench --
                GuiAction::Assembly(action) => self.process_assembly_action(action),

                // -- Draft workbench --
                GuiAction::Draft(action) => self.process_draft_action(action),

                // -- Surface workbench --
                GuiAction::Surface(action) => self.process_surface_action(action),

                // -- FEM workbench --
                GuiAction::Fem(action) => self.process_fem_action(action),

                // -- I/O expanded --
                GuiAction::ImportSvg(ref path) => {
                    self.log_info(format!("Import SVG: {}", path.display()));
                }
                GuiAction::ImportGltf(ref path) => {
                    self.log_info(format!("Import glTF: {}", path.display()));
                }
                GuiAction::Import3mf(ref path) => {
                    self.log_info(format!("Import 3MF: {}", path.display()));
                }
                GuiAction::ImportDae(ref path) => {
                    self.log_info(format!("Import DAE: {}", path.display()));
                }
                GuiAction::ExportSvg(ref path) => {
                    self.log_info(format!("Export SVG → {}", path.display()));
                }
                GuiAction::ExportDae(ref path) => {
                    self.log_info(format!("Export DAE → {}", path.display()));
                }

                // -- Viewport overlays --
                GuiAction::ToggleOrigin => {
                    self.log_info("Origin toggled");
                }
                GuiAction::ToggleGrid3d => {
                    self.log_info("3D grid toggled");
                }
                GuiAction::ToggleSectionPlane => {
                    self.nav.clip_enabled = !self.nav.clip_enabled;
                    let state = if self.nav.clip_enabled { "ON" } else { "OFF" };
                    self.log_info(format!("Section plane: {state}"));
                }

                // -- View bookmarks --
                GuiAction::SaveBookmark(name) => {
                    use crate::nav::ViewBookmark;
                    // Overwrite if name exists
                    if let Some(existing) = self.nav.view_bookmarks.iter_mut().find(|b| b.name == name) {
                        existing.yaw = self.camera.yaw;
                        existing.pitch = self.camera.pitch;
                        existing.roll = self.camera.roll;
                        existing.distance = self.camera.distance;
                        existing.target = self.camera.target;
                    } else if self.nav.view_bookmarks.len() < 20 {
                        self.nav.view_bookmarks.push(ViewBookmark {
                            name: name.clone(),
                            yaw: self.camera.yaw,
                            pitch: self.camera.pitch,
                            roll: self.camera.roll,
                            distance: self.camera.distance,
                            target: self.camera.target,
                        });
                    } else {
                        self.log_warning("Maximum 20 bookmarks reached");
                    }
                    self.gui.nav_bookmarks = self.nav.view_bookmarks.iter().map(|b| {
                        (b.name.clone(), b.yaw.to_degrees(), b.pitch.to_degrees(), b.distance)
                    }).collect();
                    self.log_info(format!("Bookmark saved: {name}"));
                }
                GuiAction::RestoreBookmark(idx) => {
                    if let Some(bm) = self.nav.view_bookmarks.get(idx).cloned() {
                        self.camera.distance = bm.distance;
                        self.camera.target = bm.target;
                        self.animate_to(bm.yaw, bm.pitch, bm.roll);
                        self.log_info(format!("Bookmark restored: {}", bm.name));
                    }
                }
                GuiAction::DeleteBookmark(idx) => {
                    if idx < self.nav.view_bookmarks.len() {
                        let name = self.nav.view_bookmarks.remove(idx).name;
                        self.gui.nav_bookmarks = self.nav.view_bookmarks.iter().map(|b| {
                            (b.name.clone(), b.yaw.to_degrees(), b.pitch.to_degrees(), b.distance)
                        }).collect();
                        self.log_info(format!("Bookmark deleted: {name}"));
                    }
                }

                // -- Object grouping --
                GuiAction::CreateGroup(name) => {
                    let gid = self.scene.create_group(&name);
                    self.scene.group_selected(gid);
                    self.rebuild_scene_gpu();
                    self.log_info(format!("Group created: {name}"));
                }
                GuiAction::GroupSelected(gid) => {
                    self.scene.group_selected(gid);
                    self.log_info(format!("Objects added to group {gid}"));
                }
                GuiAction::UngroupObject(oid) => {
                    self.scene.ungroup_object(oid);
                    self.log_info("Object ungrouped");
                }
                GuiAction::ToggleGroupVisibility(gid) => {
                    self.scene.toggle_group_visibility(gid);
                    self.rebuild_scene_gpu();
                    if let Some(g) = self.scene.groups.iter().find(|g| g.id == gid) {
                        let state = if g.visible { "shown" } else { "hidden" };
                        self.log_info(format!("Group '{}' {state}", g.name));
                    }
                }
                GuiAction::DeleteGroup(gid) => {
                    if let Some(name) = self.scene.groups.iter().find(|g| g.id == gid).map(|g| g.name.clone()) {
                        self.scene.delete_group(gid);
                        self.log_info(format!("Group deleted: {name}"));
                    }
                }
                GuiAction::ToggleMeasurement => {
                    self.gui.measurement_mode = !self.gui.measurement_mode;
                    if !self.gui.measurement_mode {
                        self.gui.measurement_points.clear();
                    }
                    let state = if self.gui.measurement_mode { "ON" } else { "OFF" };
                    self.log_info(format!("Measurement mode: {state}"));
                }
                GuiAction::AddMeasurementPoint(pt) => {
                    self.gui.measurement_points.push(pt);
                }
                GuiAction::ClearMeasurement => {
                    self.gui.measurement_points.clear();
                    self.log_info("Measurements cleared");
                }

                // -- Theme / density --
                GuiAction::ThemeToggle => {
                    use crate::gui::theme::ThemeMode;
                    self.nav.theme_mode = match self.nav.theme_mode {
                        ThemeMode::Dark => ThemeMode::Light,
                        ThemeMode::Light => ThemeMode::Dark,
                    };
                    self.gui.theme_applied = false;
                    self.log_info(format!("Theme: {:?}", self.nav.theme_mode));
                }
                GuiAction::DensityChange(density) => {
                    self.nav.ui_density = density;
                    self.gui.theme_applied = false;
                    self.log_info(format!("UI density: {density:?}"));
                }

                // -- Report --
                GuiAction::ClearReport => {
                    self.gui.report_lines.clear();
                    self.gui.status_message = "Report cleared".into();
                }

                // -- Scripting / Plugins / MCP --
                GuiAction::ExecuteLuaCode(code) => {
                    self.ensure_script_engine();
                    if let Some(engine) = &mut self.script_engine {
                        match engine.execute(&code) {
                            Ok(output) => {
                                let display = if output.is_empty() {
                                    "(ok)".to_string()
                                } else {
                                    output
                                };
                                self.gui.lua_history.push((code.clone(), display, false));
                                self.gui.status_message = format!("Lua: {code}");
                            }
                            Err(e) => {
                                let msg = e.to_string();
                                self.gui.lua_history.push((code.clone(), msg.clone(), true));
                                self.gui.log(ReportLevel::Error, format!("Lua error: {msg}"));
                            }
                        }
                    }
                }
                GuiAction::ExecuteLuaFile(path) => {
                    self.ensure_script_engine();
                    if let Some(engine) = &mut self.script_engine {
                        let short_name = std::path::Path::new(&path)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(&path)
                            .to_string();
                        match engine.execute_file(&path) {
                            Ok(output) => {
                                let display = if output.is_empty() {
                                    "(ok)".to_string()
                                } else {
                                    output
                                };
                                self.gui.lua_history.push((
                                    format!("[file] {short_name}"),
                                    display,
                                    false,
                                ));
                                self.log_info(format!("Lua script executed: {short_name}"));
                            }
                            Err(e) => {
                                let msg = e.to_string();
                                self.gui.lua_history.push((
                                    format!("[file] {short_name}"),
                                    msg.clone(),
                                    true,
                                ));
                                self.log_error(format!("Lua script error: {msg}"));
                            }
                        }
                    }
                }
                GuiAction::ClearLuaConsole => {
                    self.gui.lua_history.clear();
                    self.gui.status_message = "Lua console cleared".into();
                }
                GuiAction::TogglePluginManager => {
                    self.gui.show_plugin_manager = !self.gui.show_plugin_manager;
                    // Mirror plugin list into egui temp data for the dialog
                    self.sync_plugin_list_to_ui();
                }
                GuiAction::InitPlugins => {
                    self.register_builtin_plugins();
                    match self.plugin_registry.init_all() {
                        Ok(()) => {
                            let count = self.plugin_registry.len();
                            self.log_info(format!("{count} plugin(s) initialized"));
                        }
                        Err(e) => {
                            self.log_error(format!("Plugin init error: {e}"));
                        }
                    }
                    self.sync_plugin_list_to_ui();
                }
                GuiAction::StartMcpServer => {
                    self.gui.mcp_running = true;
                    self.log_info("MCP server started (stdio mode)");
                }
                GuiAction::StopMcpServer => {
                    self.gui.mcp_running = false;
                    self.log_info("MCP server stopped");
                }
            }
        }
    }

    /// Dispatch an `AssemblyAction` (sub-enum of `GuiAction::Assembly`).
    /// Extracted from `process_actions` so the main dispatcher stays
    /// readable; behaviour is identical to the pre-refactor inline arms.
    fn process_assembly_action(&mut self, action: AssemblyAction) {
        match action {
            AssemblyAction::Create => {
                self.gui.assembly = Some(cadkernel_modeling::Assembly::new("New Assembly"));
                self.log_info("Assembly: created new assembly");
            }
            AssemblyAction::InsertComponent => {
                let solid = self.current_solid;
                let msg = {
                    let assembly = self.gui.assembly.get_or_insert_with(|| {
                        cadkernel_modeling::Assembly::new("New Assembly")
                    });
                    if let Some(s) = solid {
                        let n = assembly.num_components();
                        let id = assembly.add_component(&format!("Component {}", n + 1), s);
                        Some(format!(
                            "Assembly: inserted component {} ({} total)",
                            id.0,
                            assembly.num_components()
                        ))
                    } else {
                        None
                    }
                };
                match msg {
                    Some(m) => self.log_info(m),
                    None => {
                        self.gui.status_message =
                            "Assembly: no solid selected to insert as component".into();
                    }
                }
            }
            AssemblyAction::Solve => {
                enum SolveMsg {
                    Ok(String),
                    Warn(String),
                    Err(String),
                    NoAssembly,
                }
                let msg = if let Some(assembly) = self.gui.assembly.as_mut() {
                    let n = assembly.num_constraints();
                    match assembly.solve(100) {
                        Ok(true) => SolveMsg::Ok(format!("Assembly: solved {n} constraints")),
                        Ok(false) => SolveMsg::Warn(format!(
                            "Assembly: solver did not converge ({n} constraints)"
                        )),
                        Err(e) => SolveMsg::Err(format!("Assembly solve error: {e}")),
                    }
                } else {
                    SolveMsg::NoAssembly
                };
                match msg {
                    SolveMsg::Ok(m) => self.log_info(m),
                    SolveMsg::Warn(m) => self.log_warning(m),
                    SolveMsg::Err(m) => self.log_error(m),
                    SolveMsg::NoAssembly => {
                        self.gui.status_message =
                            "Assembly: no assembly — create one first".into();
                    }
                }
            }
            AssemblyAction::Explode { factor } => {
                self.log_info(format!("Assembly: exploded view factor={factor:.1}"));
            }
            AssemblyAction::BillOfMaterials => {
                enum Msg { Ok(String), NoAssembly }
                let msg = if self.gui.populate_bom_entries() {
                    // populate_bom_entries() opened ActiveDialog::Bom(entries).
                    let entries = match &self.gui.active_dialog {
                        Some(crate::gui::ActiveDialog::Bom(e)) => e.as_slice(),
                        _ => &[],
                    };
                    let n: usize = entries.iter().map(|e| e.quantity).sum();
                    Msg::Ok(format!(
                        "Assembly: BOM ({} entries, {n} parts)",
                        entries.len()
                    ))
                } else {
                    Msg::NoAssembly
                };
                match msg {
                    Msg::Ok(m) => self.log_info(m),
                    Msg::NoAssembly => {
                        self.gui.status_message =
                            "Assembly: no assembly — create one first".into();
                        self.log_warning("Assembly: no assembly for BOM");
                    }
                }
            }
            AssemblyAction::DofAnalysis => {
                enum Msg { Ok(String), NoAssembly }
                let msg = if let Some(asm) = self.gui.assembly.as_ref() {
                    let n = asm.num_components();
                    let c = asm.num_constraints();
                    let j = asm.joint_count();
                    let dof = (n as i64) * 6 - (c as i64) - (j as i64);
                    Msg::Ok(format!(
                        "Assembly DOF: {n} comps × 6 - {c} constraints - {j} joints = {dof}"
                    ))
                } else {
                    Msg::NoAssembly
                };
                match msg {
                    Msg::Ok(m) => self.log_info(m),
                    Msg::NoAssembly => self.log_warning("Assembly: no assembly for DOF"),
                }
            }
            AssemblyAction::AddJoint(joint) => {
                enum Msg { Ok(String), TooFew }
                let msg = if self.gui.open_joint_editor(joint) {
                    Msg::Ok(format!("Assembly: edit joint {}", joint.label()))
                } else {
                    Msg::TooFew
                };
                match msg {
                    Msg::Ok(m) => self.log_info(m),
                    Msg::TooFew => self.log_warning(
                        "Assembly: need at least 2 components to add a joint",
                    ),
                }
            }
            AssemblyAction::ToggleComponentVisibility(idx) => {
                enum Msg { Ok(String), Fail }
                let msg = if self.gui.toggle_assembly_component_visibility(idx) {
                    Msg::Ok(format!("Assembly: toggled component {idx} visibility"))
                } else {
                    Msg::Fail
                };
                match msg {
                    Msg::Ok(m) => self.log_info(m),
                    Msg::Fail => self.log_warning("Assembly: toggle visibility failed"),
                }
            }
            AssemblyAction::CommitJoint => {
                enum Msg { Ok(String), Fail }
                let msg = if self.gui.commit_assembly_joint() {
                    let n = self.gui.assembly.as_ref().map(|a| a.joint_count()).unwrap_or(0);
                    Msg::Ok(format!("Assembly: joint added ({n} total)"))
                } else {
                    Msg::Fail
                };
                match msg {
                    Msg::Ok(m) => self.log_info(m),
                    Msg::Fail => self.log_warning("Assembly: failed to commit joint"),
                }
            }
        }
    }

    /// Dispatch a `FemAction` (sub-enum of `GuiAction::Fem`). Extracted from
    /// `process_actions` so the main dispatcher stays readable; behaviour is
    /// identical to the pre-refactor inline arms.
    fn process_fem_action(&mut self, action: FemAction) {
        match action {
            FemAction::CreateAnalysis => {
                let solid_opt = self.current_solid;
                let msg = if let Some(solid) = solid_opt {
                    match cadkernel_modeling::generate_tet_mesh(&self.model, solid, 1.0) {
                        Ok(mesh) => {
                            let n_nodes = mesh.nodes.len();
                            let n_elems = mesh.elements.len();
                            self.gui.fem_analysis = Some(
                                cadkernel_modeling::AnalysisContainer::new(
                                    mesh,
                                    self.gui.pending_fem_material.clone(),
                                ),
                            );
                            Ok(format!(
                                "FEM: new analysis ({n_nodes} nodes, {n_elems} elements)"
                            ))
                        }
                        Err(e) => Err(format!("FEM mesh error: {e}")),
                    }
                } else {
                    Err("FEM: no solid to mesh".into())
                };
                match msg {
                    Ok(m) => self.log_info(m),
                    Err(m) => self.log_warning(m),
                }
            }
            FemAction::SetMaterial(mat) => {
                self.log_info(format!("FEM: material set to '{mat}'"));
            }
            FemAction::OpenMaterialPicker => {
                self.gui.open_material_picker();
            }
            FemAction::CommitMaterialPicker => {
                // Take the dialog out so we can mutate `pending_fem_material`.
                if let Some(crate::gui::ActiveDialog::MaterialPicker(s)) =
                    self.gui.active_dialog.take()
                {
                    self.gui.pending_fem_material = crate::gui::material_from_preset(
                        s.selected, s.youngs_modulus, s.poisson_ratio, s.density);
                    self.log_info(format!("FEM: material set to '{}'", s.selected.label()));
                }
            }
            FemAction::OpenBcEditor(kind) => {
                if self.gui.fem_analysis.is_none() {
                    self.gui.status_message = "FEM: no analysis \u{2014} create one first".into();
                } else {
                    self.gui.open_bc_editor(kind);
                }
            }
            FemAction::CommitBcEditor => {
                if let Some(crate::gui::ActiveDialog::BcEditor(s)) =
                    self.gui.active_dialog.take()
                {
                    let total = self.gui.fem_analysis.as_mut().map(|c| {
                        c.add_bc(s.to_boundary_condition());
                        c.boundary_conditions.len()
                    });
                    match total {
                        Some(n) => self.log_info(format!(
                            "FEM: BC added ({}, node {}) \u{2014} total {n}",
                            s.bc_kind.label(), s.node_index)),
                        None => self.gui.status_message =
                            "FEM: no analysis \u{2014} create one first".into(),
                    }
                }
            }
            FemAction::GenTetMesh { element_size } => {
                self.log_info(format!("FEM: generate tet mesh (size={element_size:.2})"));
            }
            FemAction::GenHexMesh { nx, ny, nz } => {
                self.log_info(format!("FEM: generate hex mesh ({nx}x{ny}x{nz})"));
            }
            FemAction::AddConstraint(ctype) => {
                self.log_info(format!("FEM: constraint {ctype:?}"));
            }
            FemAction::SolveStatic => {
                enum Msg { Ok(String), Err(String), NoAnalysis }
                let msg = if let Some(container) = self.gui.fem_analysis.as_mut() {
                    let n_bc = container.boundary_conditions.len();
                    match container.run_static() {
                        Ok(()) => {
                            let max_disp = container
                                .result
                                .as_ref()
                                .map(|r| r.max_displacement)
                                .unwrap_or(0.0);
                            Msg::Ok(format!(
                                "FEM: static solved ({n_bc} BCs, max |u|={max_disp:.4e})"
                            ))
                        }
                        Err(e) => Msg::Err(format!("FEM solve error: {e}")),
                    }
                } else {
                    Msg::NoAnalysis
                };
                match msg {
                    Msg::Ok(m) => self.log_info(m),
                    Msg::Err(m) => self.log_warning(m),
                    Msg::NoAnalysis => {
                        self.gui.status_message =
                            "FEM: no analysis — create one first".into();
                    }
                }
            }
            FemAction::SolveModal { modes } => {
                self.log_info(format!("FEM: modal solve ({modes} modes)"));
            }
            FemAction::SolveThermal => self.log_info("FEM: thermal solve"),
            FemAction::SolveBuckling { modes } => {
                self.log_info(format!("FEM: buckling solve ({modes} modes)"));
            }
            FemAction::SolveNonlinear => self.log_info("FEM: nonlinear solve"),
            FemAction::ShowStress => self.log_info("FEM: show stress"),
            FemAction::ShowDisplacement => self.log_info("FEM: show displacement"),
            FemAction::ShowVonMises => self.log_info("FEM: show von Mises"),
            FemAction::Summary => self.run_fem_summary(),
            FemAction::Report => self.run_fem_report(),
        }
    }

    /// Dispatch a `SketcherAction` (sub-enum of `GuiAction::Sketcher`).
    /// Extracted from `process_actions` so the main dispatcher stays
    /// readable; behaviour is identical to the pre-refactor inline arms.
    fn process_sketcher_action(&mut self, action: SketcherAction) {
        use SketcherAction as S;
        match action {
            // -- Lifecycle --
            S::Enter(plane) => {
                self.gui.sketch_mode = Some(SketchMode::new(plane));
                self.gui.active_workbench = gui::Workbench::Sketcher;
                self.gui.status_message =
                    "Sketch mode: click to add points, select tool from toolbar".into();
            }
            S::EnterOnSelectedFace => {
                if let Some(plane) = self.compute_face_workplane() {
                    self.gui.sketch_mode = Some(SketchMode::new(plane));
                    self.gui.active_workbench = gui::Workbench::Sketcher;
                    self.gui.status_message = "Sketch on selected face".into();
                    self.log_info("Sketch started on selected face");
                } else {
                    self.gui.status_message = "No face selected for sketch".into();
                }
            }
            S::Edit => {
                if self.gui.sketch_mode.is_none() {
                    if let Some((sketch, plane)) = self.gui.last_sketch.take() {
                        let mut sm = SketchMode::new(plane);
                        sm.sketch = sketch;
                        sm.tool = SketchTool::Select;
                        self.gui.sketch_mode = Some(sm);
                        self.gui.active_workbench = crate::gui::Workbench::Sketcher;
                        self.gui.status_message = "Editing previous sketch".into();
                    } else {
                        self.gui.status_message = "No previous sketch to edit".into();
                    }
                }
            }
            S::Close => {
                self.close_sketch();
            }
            S::Cancel => {
                self.gui.sketch_mode = None;
                self.gui.status_message = "Sketch cancelled".into();
            }

            // -- Pointer / tool --
            S::Click(x, y) => {
                self.handle_sketch_click(x, y);
            }
            S::SetTool(tool) => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    sm.tool = tool;
                    sm.pending_point = None;
                    self.gui.status_message = format!("Sketch tool: {tool:?}");
                }
            }

            // -- Geometric constraints --
            S::ConstrainHorizontal => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    Self::apply_sketch_constraint_h(sm, &mut self.gui.status_message);
                }
            }
            S::ConstrainVertical => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    Self::apply_sketch_constraint_v(sm, &mut self.gui.status_message);
                }
            }
            S::ConstrainLength(len) => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    let mut applied = false;
                    for e in &sm.selected_entities {
                        if let SketchEntityRef::Line(i) = *e {
                            if i < sm.sketch.lines.len() {
                                sm.sketch.add_constraint(Constraint::Length(
                                    cadkernel_sketch::LineId(i), len,
                                ));
                                applied = true;
                            }
                        }
                    }
                    if !applied && !sm.sketch.lines.is_empty() {
                        let lid = cadkernel_sketch::LineId(sm.sketch.lines.len() - 1);
                        sm.sketch.add_constraint(Constraint::Length(lid, len));
                        applied = true;
                    }
                    if applied {
                        self.gui.status_message = format!("Added Length={len:.1} constraint");
                    }
                }
            }
            S::ConstrainCoincident => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    let pts: Vec<usize> = sm.selected_entities.iter().filter_map(|e| {
                        if let SketchEntityRef::Point(i) = e { Some(*i) } else { None }
                    }).collect();
                    if pts.len() >= 2 {
                        sm.sketch.add_constraint(Constraint::Coincident(
                            cadkernel_sketch::PointId(pts[0]),
                            cadkernel_sketch::PointId(pts[1]),
                        ));
                        self.gui.status_message = "Coincident constraint added".into();
                    } else {
                        self.gui.status_message = "Select 2 points for Coincident".into();
                    }
                }
            }
            S::ConstrainParallel => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    let lines: Vec<usize> = sm.selected_entities.iter().filter_map(|e| {
                        if let SketchEntityRef::Line(i) = e { Some(*i) } else { None }
                    }).collect();
                    if lines.len() >= 2 {
                        sm.sketch.add_constraint(Constraint::Parallel(
                            cadkernel_sketch::LineId(lines[0]),
                            cadkernel_sketch::LineId(lines[1]),
                        ));
                        self.gui.status_message = "Parallel constraint added".into();
                    } else {
                        self.gui.status_message = "Select 2 lines for Parallel".into();
                    }
                }
            }
            S::ConstrainPerpendicular => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    let lines: Vec<usize> = sm.selected_entities.iter().filter_map(|e| {
                        if let SketchEntityRef::Line(i) = e { Some(*i) } else { None }
                    }).collect();
                    if lines.len() >= 2 {
                        sm.sketch.add_constraint(Constraint::Perpendicular(
                            cadkernel_sketch::LineId(lines[0]),
                            cadkernel_sketch::LineId(lines[1]),
                        ));
                        self.gui.status_message = "Perpendicular constraint added".into();
                    } else {
                        self.gui.status_message = "Select 2 lines for Perpendicular".into();
                    }
                }
            }
            S::ConstrainTangent => {
                self.log_info("Select line + circle for Tangent (not yet supported)");
            }
            S::ConstrainEqual => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    let lines: Vec<usize> = sm.selected_entities.iter().filter_map(|e| {
                        if let SketchEntityRef::Line(i) = e { Some(*i) } else { None }
                    }).collect();
                    if lines.len() >= 2 {
                        sm.sketch.add_constraint(Constraint::EqualLength(
                            cadkernel_sketch::LineId(lines[0]),
                            cadkernel_sketch::LineId(lines[1]),
                        ));
                        self.gui.status_message = "Equal length constraint added".into();
                    } else {
                        self.gui.status_message = "Select 2 lines for Equal".into();
                    }
                }
            }
            S::ConstrainSymmetric => {
                self.log_info("Select 2 points + 1 line for Symmetric");
            }
            S::ConstrainFixed => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    let mut applied = 0;
                    for e in &sm.selected_entities {
                        if let SketchEntityRef::Point(i) = *e {
                            if i < sm.sketch.points.len() {
                                let pt = &sm.sketch.points[i];
                                sm.sketch.add_constraint(Constraint::Fixed(
                                    cadkernel_sketch::PointId(i),
                                    pt.position.x, pt.position.y,
                                ));
                                applied += 1;
                            }
                        }
                    }
                    if applied > 0 {
                        self.gui.status_message = format!("Fixed {applied} point(s)");
                    } else {
                        self.gui.status_message = "Select point(s) for Fixed".into();
                    }
                }
            }
            S::ConstrainBlock => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    let mut applied = 0;
                    for e in &sm.selected_entities {
                        if let SketchEntityRef::Point(i) = *e {
                            if i < sm.sketch.points.len() {
                                let pt = &sm.sketch.points[i];
                                sm.sketch.add_constraint(Constraint::Block(
                                    cadkernel_sketch::PointId(i),
                                    pt.position.x, pt.position.y,
                                ));
                                applied += 1;
                            }
                        }
                    }
                    if applied > 0 {
                        self.gui.status_message = format!("Blocked {applied} point(s)");
                    } else {
                        self.gui.status_message = "Select point(s) for Block".into();
                    }
                }
            }
            S::ConstrainDistance(val) => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    let pts: Vec<usize> = sm.selected_entities.iter().filter_map(|e| {
                        if let SketchEntityRef::Point(i) = e { Some(*i) } else { None }
                    }).collect();
                    if pts.len() >= 2 {
                        sm.sketch.add_constraint(Constraint::Distance(
                            cadkernel_sketch::PointId(pts[0]),
                            cadkernel_sketch::PointId(pts[1]),
                            val,
                        ));
                        self.gui.status_message = format!("Distance={val:.1} constraint added");
                    } else {
                        // Fall back to Length on selected/last line
                        let mut applied = false;
                        for e in &sm.selected_entities {
                            if let SketchEntityRef::Line(i) = *e {
                                if i < sm.sketch.lines.len() {
                                    sm.sketch.add_constraint(Constraint::Length(
                                        cadkernel_sketch::LineId(i), val,
                                    ));
                                    applied = true;
                                    break;
                                }
                            }
                        }
                        if !applied && !sm.sketch.lines.is_empty() {
                            let lid = cadkernel_sketch::LineId(sm.sketch.lines.len() - 1);
                            sm.sketch.add_constraint(Constraint::Length(lid, val));
                        }
                        self.gui.status_message = format!("Distance={val:.1} constraint added");
                    }
                }
            }
            S::ConstrainAngle(val) => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    let lines: Vec<usize> = sm.selected_entities.iter().filter_map(|e| {
                        if let SketchEntityRef::Line(i) = e { Some(*i) } else { None }
                    }).collect();
                    if lines.len() >= 2 {
                        sm.sketch.add_constraint(Constraint::Angle(
                            cadkernel_sketch::LineId(lines[0]),
                            cadkernel_sketch::LineId(lines[1]),
                            val.to_radians(),
                        ));
                        self.gui.status_message = format!("Angle={val:.1}° constraint added");
                    } else {
                        self.gui.status_message = "Select 2 lines for Angle".into();
                    }
                }
            }
            S::ConstrainRadius(val) => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    let mut applied = false;
                    for e in &sm.selected_entities {
                        if let SketchEntityRef::Circle(i) = *e {
                            if i < sm.sketch.circles.len() {
                                let cid = sm.sketch.circles[i].center;
                                sm.sketch.add_constraint(Constraint::Radius(
                                    cid, cid, val,
                                ));
                                applied = true;
                            }
                        }
                    }
                    if applied {
                        self.gui.status_message = format!("Radius={val:.1} constraint added");
                    } else {
                        self.gui.status_message = "Select circle for Radius".into();
                    }
                }
            }
            S::ConstrainDiameter(val) => {
                self.log_info(format!("Diameter={val:.1} — select circle first"));
            }
            S::ConstrainHDistance(val) => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    let pts: Vec<usize> = sm.selected_entities.iter().filter_map(|e| {
                        if let SketchEntityRef::Point(i) = e { Some(*i) } else { None }
                    }).collect();
                    if pts.len() >= 2 {
                        sm.sketch.add_constraint(Constraint::HorizontalDistance(
                            cadkernel_sketch::PointId(pts[0]),
                            cadkernel_sketch::PointId(pts[1]),
                            val,
                        ));
                        self.gui.status_message = format!("H-Distance={val:.1} added");
                    } else {
                        self.gui.status_message = "Select 2 points for H-Distance".into();
                    }
                }
            }
            S::ConstrainVDistance(val) => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    let pts: Vec<usize> = sm.selected_entities.iter().filter_map(|e| {
                        if let SketchEntityRef::Point(i) = e { Some(*i) } else { None }
                    }).collect();
                    if pts.len() >= 2 {
                        sm.sketch.add_constraint(Constraint::VerticalDistance(
                            cadkernel_sketch::PointId(pts[0]),
                            cadkernel_sketch::PointId(pts[1]),
                            val,
                        ));
                        self.gui.status_message = format!("V-Distance={val:.1} added");
                    } else {
                        self.gui.status_message = "Select 2 points for V-Distance".into();
                    }
                }
            }

            // -- Tools --
            S::FilletCorner { radius } => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    use crate::gui::SketchEntityRef::Line;
                    let lines: Vec<usize> = sm.selected_entities.iter()
                        .filter_map(|e| if let Line(i) = e { Some(*i) } else { None })
                        .collect();
                    if lines.len() == 2 {
                        sm.save_snapshot();
                        if cadkernel_sketch::fillet_sketch_corner(
                            &mut sm.sketch,
                            cadkernel_sketch::LineId(lines[0]),
                            cadkernel_sketch::LineId(lines[1]),
                            radius,
                        ).is_some() {
                            sm.selected_entities.clear();
                            self.gui.status_message = format!("Corner filleted (r={radius:.1})");
                        } else {
                            sm.undo_stack.pop();
                            self.gui.status_message = "Fillet failed: lines don't share a vertex".into();
                        }
                    } else {
                        self.gui.status_message = "Select 2 lines to fillet".into();
                    }
                }
            }
            S::ChamferCorner { distance } => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    use crate::gui::SketchEntityRef::Line;
                    let lines: Vec<usize> = sm.selected_entities.iter()
                        .filter_map(|e| if let Line(i) = e { Some(*i) } else { None })
                        .collect();
                    if lines.len() == 2 {
                        sm.save_snapshot();
                        if cadkernel_sketch::chamfer_sketch_corner(
                            &mut sm.sketch,
                            cadkernel_sketch::LineId(lines[0]),
                            cadkernel_sketch::LineId(lines[1]),
                            distance,
                        ).is_some() {
                            sm.selected_entities.clear();
                            self.gui.status_message = format!("Corner chamfered (d={distance:.1})");
                        } else {
                            sm.undo_stack.pop();
                            self.gui.status_message = "Chamfer failed: lines don't share a vertex".into();
                        }
                    } else {
                        self.gui.status_message = "Select 2 lines to chamfer".into();
                    }
                }
            }
            S::TrimEdge => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    use crate::gui::SketchEntityRef::Line;
                    let lines: Vec<usize> = sm.selected_entities.iter()
                        .filter_map(|e| if let Line(i) = e { Some(*i) } else { None })
                        .collect();
                    if lines.len() == 2 {
                        let l0 = cadkernel_sketch::LineId(lines[0]);
                        let l1 = cadkernel_sketch::LineId(lines[1]);
                        // Keep start point side of first line
                        let keep = sm.sketch.lines[lines[0]].start;
                        sm.save_snapshot();
                        let result = cadkernel_sketch::trim_edge(&mut sm.sketch, l0, l1, keep);
                        if result.trimmed {
                            sm.selected_entities.clear();
                            self.gui.status_message = "Edge trimmed".into();
                        } else {
                            sm.undo_stack.pop();
                            self.gui.status_message = "Trim failed: lines don't intersect".into();
                        }
                    } else {
                        self.gui.status_message = "Select 2 lines to trim".into();
                    }
                }
            }
            S::SplitEdge => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    use crate::gui::SketchEntityRef::Line;
                    let line_idx = sm.selected_entities.iter()
                        .find_map(|e| if let Line(i) = e { Some(*i) } else { None });
                    if let Some(idx) = line_idx {
                        sm.save_snapshot();
                        let _result = cadkernel_sketch::split_edge(
                            &mut sm.sketch, cadkernel_sketch::LineId(idx), 0.5,
                        );
                        sm.selected_entities.clear();
                        self.gui.status_message = "Edge split at midpoint".into();
                    } else {
                        self.gui.status_message = "Select a line to split".into();
                    }
                }
            }
            S::ExtendEdge => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    use crate::gui::SketchEntityRef::Line;
                    let line_idx = sm.selected_entities.iter()
                        .find_map(|e| if let Line(i) = e { Some(*i) } else { None });
                    if let Some(idx) = line_idx {
                        // Extend by 50% of current length toward end direction
                        let l = sm.sketch.lines[idx];
                        let s = sm.sketch.points[l.start.0].position;
                        let e = sm.sketch.points[l.end.0].position;
                        let tx = e.x + (e.x - s.x) * 0.5;
                        let ty = e.y + (e.y - s.y) * 0.5;
                        sm.save_snapshot();
                        cadkernel_sketch::extend_edge(
                            &mut sm.sketch, cadkernel_sketch::LineId(idx), tx, ty,
                        );
                        self.gui.status_message = "Edge extended".into();
                    } else {
                        self.gui.status_message = "Select a line to extend".into();
                    }
                }
            }
            S::MirrorGeometry => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    // Need exactly 1 selected line as mirror axis + other selected points
                    let mut axis_line: Option<usize> = None;
                    let mut point_ids: Vec<cadkernel_sketch::PointId> = Vec::new();
                    for e in &sm.selected_entities {
                        match *e {
                            SketchEntityRef::Line(i)
                                if axis_line.is_none() => {
                                    axis_line = Some(i);
                                }
                            SketchEntityRef::Point(i) => {
                                point_ids.push(cadkernel_sketch::PointId(i));
                            }
                            _ => {}
                        }
                    }
                    if let Some(al) = axis_line {
                        sm.save_snapshot();
                        // If no explicit points selected, mirror all points except axis line endpoints
                        if point_ids.is_empty() {
                            let axis_s = sm.sketch.lines[al].start.0;
                            let axis_e = sm.sketch.lines[al].end.0;
                            for i in 0..sm.sketch.points.len() {
                                if i != axis_s && i != axis_e {
                                    point_ids.push(cadkernel_sketch::PointId(i));
                                }
                            }
                        }
                        let mirror_lid = cadkernel_sketch::LineId(al);
                        let new_pts = sm.sketch.mirror_elements(&point_ids, mirror_lid);
                        self.gui.status_message = format!(
                            "Mirrored {} points → {} new points",
                            point_ids.len(), new_pts.len()
                        );
                    } else {
                        self.gui.status_message = "Mirror: select a line as mirror axis".into();
                    }
                }
            }
            S::ExternalProjection => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    // Project model vertices onto sketch plane
                    let verts: Vec<Point3> = self.model.vertices.iter()
                        .map(|(_, v)| v.point)
                        .collect();
                    if verts.is_empty() {
                        self.gui.status_message = "No model vertices to project".into();
                    } else {
                        sm.save_snapshot();
                        let ids = external_projection(
                            &mut sm.sketch, &verts, &sm.plane,
                        );
                        self.gui.status_message = format!(
                            "Projected {} vertices onto sketch", ids.len()
                        );
                    }
                }
            }
            S::CarbonCopy => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    if let Some((ref source, _)) = self.gui.last_sketch {
                        sm.save_snapshot();
                        if carbon_copy(source, &mut sm.sketch).is_ok() {
                            self.gui.status_message = "Carbon copy applied".into();
                        } else {
                            self.gui.status_message = "Carbon copy failed".into();
                        }
                    } else {
                        self.gui.status_message = "No previous sketch to copy from".into();
                    }
                }
            }
            S::CopySelection => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    // Collect unique point indices from selected entities
                    let mut pt_indices: Vec<usize> = Vec::new();
                    let mut line_refs: Vec<(usize, usize)> = Vec::new();
                    for e in &sm.selected_entities {
                        match *e {
                            SketchEntityRef::Point(i)
                                if !pt_indices.contains(&i) => { pt_indices.push(i); }
                            SketchEntityRef::Line(i)
                                if i < sm.sketch.lines.len() => {
                                    let s = sm.sketch.lines[i].start.0;
                                    let e = sm.sketch.lines[i].end.0;
                                    if !pt_indices.contains(&s) { pt_indices.push(s); }
                                    if !pt_indices.contains(&e) { pt_indices.push(e); }
                                    line_refs.push((s, e));
                                }
                            _ => {}
                        }
                    }
                    if pt_indices.is_empty() {
                        self.gui.status_message = "Nothing to copy".into();
                    } else {
                        // Compute centroid
                        let (mut cx, mut cy) = (0.0, 0.0);
                        for &pi in &pt_indices {
                            if pi < sm.sketch.points.len() {
                                cx += sm.sketch.points[pi].position.x;
                                cy += sm.sketch.points[pi].position.y;
                            }
                        }
                        cx /= pt_indices.len() as f64;
                        cy /= pt_indices.len() as f64;
                        // Store relative offsets
                        sm.clipboard_points.clear();
                        sm.clipboard_lines.clear();
                        let mut idx_map = std::collections::HashMap::new();
                        for (new_i, &pi) in pt_indices.iter().enumerate() {
                            if pi < sm.sketch.points.len() {
                                let p = &sm.sketch.points[pi];
                                sm.clipboard_points.push((p.position.x - cx, p.position.y - cy));
                                idx_map.insert(pi, new_i);
                            }
                        }
                        for (s, e) in &line_refs {
                            if let (Some(&si), Some(&ei)) = (idx_map.get(s), idx_map.get(e)) {
                                sm.clipboard_lines.push((si, ei));
                            }
                        }
                        self.gui.status_message = format!(
                            "Copied {} points, {} lines",
                            sm.clipboard_points.len(), sm.clipboard_lines.len()
                        );
                    }
                }
            }
            S::PasteSelection(px, py) => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    if sm.clipboard_points.is_empty() {
                        self.gui.status_message = "Clipboard empty".into();
                    } else {
                        sm.save_snapshot();
                        // Create points at paste position + offsets
                        let new_pts: Vec<cadkernel_sketch::PointId> = sm.clipboard_points
                            .iter()
                            .map(|&(dx, dy)| sm.sketch.add_point(px + dx, py + dy))
                            .collect();
                        // Recreate lines
                        for &(si, ei) in &sm.clipboard_lines.clone() {
                            if si < new_pts.len() && ei < new_pts.len() {
                                sm.sketch.add_line(new_pts[si], new_pts[ei]);
                            }
                        }
                        self.gui.status_message = format!(
                            "Pasted {} points at ({px:.1}, {py:.1})",
                            new_pts.len()
                        );
                    }
                }
            }
            S::MergePoints => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    sm.save_snapshot();
                    let eps = 0.01;
                    let n = sm.sketch.points.len();
                    // Build merge map: for each point, map to lowest index within epsilon
                    let mut merge_to: Vec<usize> = (0..n).collect();
                    for i in 0..n {
                        for j in (i + 1)..n {
                            let dx = sm.sketch.points[i].position.x - sm.sketch.points[j].position.x;
                            let dy = sm.sketch.points[i].position.y - sm.sketch.points[j].position.y;
                            if (dx * dx + dy * dy).sqrt() < eps {
                                merge_to[j] = merge_to[i];
                            }
                        }
                    }
                    // Remap all references
                    let mut merged = 0usize;
                    for line in &mut sm.sketch.lines {
                        let ns = merge_to[line.start.0];
                        let ne = merge_to[line.end.0];
                        if ns != line.start.0 || ne != line.end.0 { merged += 1; }
                        line.start = cadkernel_sketch::PointId(ns);
                        line.end = cadkernel_sketch::PointId(ne);
                    }
                    for arc in &mut sm.sketch.arcs {
                        arc.center = cadkernel_sketch::PointId(merge_to[arc.center.0]);
                        arc.start_point = cadkernel_sketch::PointId(merge_to[arc.start_point.0]);
                        arc.end_point = cadkernel_sketch::PointId(merge_to[arc.end_point.0]);
                    }
                    for circle in &mut sm.sketch.circles {
                        circle.center = cadkernel_sketch::PointId(merge_to[circle.center.0]);
                    }
                    for ell in &mut sm.sketch.ellipses {
                        ell.center = cadkernel_sketch::PointId(merge_to[ell.center.0]);
                        ell.major_end = cadkernel_sketch::PointId(merge_to[ell.major_end.0]);
                    }
                    for bsp in &mut sm.sketch.bsplines {
                        for cp in &mut bsp.control_points {
                            *cp = cadkernel_sketch::PointId(merge_to[cp.0]);
                        }
                    }
                    self.gui.status_message = format!("Merged {merged} point references");
                }
            }

            // -- B-spline --
            S::ConvertToBSpline => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    // Convert first selected line/arc/circle to B-spline
                    let entity_id = sm.selected_entities.first().and_then(|e| match *e {
                        SketchEntityRef::Line(i) => Some(i),
                        SketchEntityRef::Arc(i) => Some(sm.sketch.lines.len() + i),
                        SketchEntityRef::Circle(i) => Some(sm.sketch.lines.len() + sm.sketch.arcs.len() + i),
                        _ => None,
                    });
                    if let Some(eid) = entity_id {
                        sm.save_snapshot();
                        match geometry_to_bspline(&mut sm.sketch, eid) {
                            Ok(bid) => {
                                self.gui.status_message = format!("Converted to B-Spline {}", bid.0);
                            }
                            Err(e) => {
                                self.gui.status_message = format!("Convert failed: {e}");
                            }
                        }
                    } else {
                        self.gui.status_message = "Select a line, arc, or circle to convert".into();
                    }
                }
            }
            S::IncreaseDegree => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    let bsp_id = sm.selected_entities.iter().find_map(|e| match *e {
                        SketchEntityRef::BSpline(i) => Some(cadkernel_sketch::BSplineId(i)),
                        _ => None,
                    });
                    if let Some(bid) = bsp_id {
                        sm.save_snapshot();
                        match increase_bspline_degree(&mut sm.sketch, bid) {
                            Ok(()) => {
                                let deg = sm.sketch.bsplines[bid.0].degree;
                                self.gui.status_message = format!("B-Spline degree → {deg}");
                            }
                            Err(e) => self.gui.status_message = format!("Increase degree: {e}"),
                        }
                    } else {
                        self.gui.status_message = "Select a B-Spline to increase degree".into();
                    }
                }
            }
            S::DecreaseDegree => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    let bsp_id = sm.selected_entities.iter().find_map(|e| match *e {
                        SketchEntityRef::BSpline(i) => Some(cadkernel_sketch::BSplineId(i)),
                        _ => None,
                    });
                    if let Some(bid) = bsp_id {
                        sm.save_snapshot();
                        match decrease_bspline_degree(&mut sm.sketch, bid) {
                            Ok(()) => {
                                let deg = sm.sketch.bsplines[bid.0].degree;
                                self.gui.status_message = format!("B-Spline degree → {deg}");
                            }
                            Err(e) => self.gui.status_message = format!("Decrease degree: {e}"),
                        }
                    } else {
                        self.gui.status_message = "Select a B-Spline to decrease degree".into();
                    }
                }
            }
            S::InsertKnot => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    let bsp_id = sm.selected_entities.iter().find_map(|e| match *e {
                        SketchEntityRef::BSpline(i) => Some(cadkernel_sketch::BSplineId(i)),
                        _ => None,
                    });
                    if let Some(bid) = bsp_id {
                        sm.save_snapshot();
                        match insert_knot(&mut sm.sketch, bid, 0.5) {
                            Ok(()) => {
                                let n = sm.sketch.bsplines[bid.0].control_points.len();
                                self.gui.status_message = format!("Inserted knot (now {n} CPs)");
                            }
                            Err(e) => self.gui.status_message = format!("Insert knot: {e}"),
                        }
                    } else {
                        self.gui.status_message = "Select a B-Spline to insert knot".into();
                    }
                }
            }

            // -- Toggles --
            S::ToggleConstruction => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    // If entities are selected, toggle them between construction/normal
                    if !sm.selected_entities.is_empty() {
                        sm.save_snapshot();
                        let mut toggled = 0usize;
                        for e in &sm.selected_entities {
                            match *e {
                                SketchEntityRef::Point(i) => {
                                    let pid = cadkernel_sketch::PointId(i);
                                    if let Some(pos) = sm.sketch.construction_points.iter().position(|p| *p == pid) {
                                        sm.sketch.construction_points.remove(pos);
                                    } else {
                                        sm.sketch.mark_construction_point(pid);
                                    }
                                    toggled += 1;
                                }
                                SketchEntityRef::Line(i) => {
                                    let lid = cadkernel_sketch::LineId(i);
                                    if let Some(pos) = sm.sketch.construction_lines.iter().position(|l| *l == lid) {
                                        sm.sketch.construction_lines.remove(pos);
                                    } else {
                                        sm.sketch.mark_construction_line(lid);
                                    }
                                    toggled += 1;
                                }
                                _ => {}
                            }
                        }
                        if toggled > 0 {
                            self.gui.status_message = format!("Toggled {toggled} entities construction mode");
                        }
                    } else {
                        // No selection: toggle global construction mode for new entities
                        sm.construction_mode = !sm.construction_mode;
                        let state = if sm.construction_mode { "ON" } else { "OFF" };
                        self.gui.status_message = format!("Construction mode: {state}");
                    }
                }
            }
            S::ToggleGrid => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    sm.show_grid = !sm.show_grid;
                }
            }
            S::ToggleSnap => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    sm.snap_enabled = !sm.snap_enabled;
                }
            }
            S::ToggleConstraintsVisible => {
                if let Some(sm) = &mut self.gui.sketch_mode {
                    sm.show_constraints = !sm.show_constraints;
                }
            }
        }
    }

    fn process_mesh_action(&mut self, action: MeshAction) {
        use MeshAction as M;
        match action {
            M::Decimate(ratio) => {
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
            M::Subdivide => {
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
            M::FillHoles => {
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
            M::FlipNormals => {
                if let Some(mesh) = &self.current_mesh {
                    let new_mesh = cadkernel_io::flip_normals(mesh);
                    self.set_mesh(new_mesh);
                    self.log_info("Normals flipped");
                } else {
                    self.gui.status_message = "No mesh".into();
                }
            }
            M::Smooth { iterations, factor } => {
                if let Some(mesh) = &self.current_mesh {
                    let new_mesh = cadkernel_io::smooth_mesh(mesh, iterations, factor);
                    let count = new_mesh.vertices.len();
                    self.set_mesh(new_mesh);
                    self.log_info(format!("Smoothed: {iterations} iters, factor={factor:.2} ({count} verts)"));
                } else {
                    self.gui.status_message = "No mesh to smooth".into();
                }
            }
            M::HarmonizeNormals => {
                if let Some(mesh) = &self.current_mesh {
                    let new_mesh = cadkernel_io::harmonize_normals(mesh);
                    self.set_mesh(new_mesh);
                    self.log_info("Normals harmonized");
                } else {
                    self.gui.status_message = "No mesh".into();
                }
            }
            M::CheckWatertight => {
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
            M::Remesh { target_edge_len } => {
                if let Some(mesh) = &self.current_mesh {
                    match cadkernel_io::remesh(mesh, target_edge_len) {
                        Ok(new_mesh) => {
                            let count = new_mesh.indices.len();
                            self.set_mesh(new_mesh);
                            self.log_info(format!("Remeshed: {count} triangles (edge\u{2264}{target_edge_len:.2})"));
                        }
                        Err(e) => {
                            self.log_error(format!("Remesh error: {e}"));
                        }
                    }
                } else {
                    self.gui.status_message = "No mesh to remesh".into();
                }
            }
            M::Repair => {
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
        }
    }

    fn process_surface_action(&mut self, action: SurfaceAction) {
        use SurfaceAction as S;
        match action {
            S::Filling => {
                self.snapshot_before("Surface Filling");
                let mut model = BRepModel::new();
                let boundary = [
                    Point3::new(0.0, 0.0, 0.0),
                    Point3::new(2.0, 0.0, 0.0),
                    Point3::new(2.0, 2.0, 0.0),
                    Point3::new(0.0, 2.0, 0.0),
                ];
                match filling(&mut model, &boundary, 1) {
                    Ok(r) => {
                        self.add_to_scene("Surface Filling", model, r.solid, None);
                        self.log_info("Surface: filling");
                    }
                    Err(e) => self.log_error(format!("SurfaceFilling error: {e}")),
                }
            }
            S::Boundary => {
                self.snapshot_before("Surface Boundary");
                let mut model = BRepModel::new();
                match make_polygon_wire(Point3::ORIGIN, Vec3::Z, 1.0, 6) {
                    Ok(pts) => match filling(&mut model, &pts, 1) {
                        Ok(r) => {
                            self.add_to_scene("Surface Boundary", model, r.solid, None);
                            self.log_info("Surface: boundary");
                        }
                        Err(e) => self.log_error(format!("SurfaceBoundary fill error: {e}")),
                    },
                    Err(e) => self.log_error(format!("SurfaceBoundary error: {e}")),
                }
            }
            S::Sections => self.run_surface_sections(),
            S::Extend => self.run_surface_extend(),
            S::Blend => self.run_surface_blend(),
            S::Pipe => {
                self.snapshot_before("Surface Pipe");
                let mut model = BRepModel::new();
                let path = [
                    Point3::new(0.0, 0.0, 0.0),
                    Point3::new(0.0, 0.0, 2.0),
                ];
                match pipe_surface(&mut model, &path, 0.25, 16) {
                    Ok(r) => {
                        self.add_to_scene(
                            "Surface Pipe",
                            model,
                            r.solid,
                            Some(crate::scene::CreationParams::SurfacePipe {
                                radius: 0.25,
                                length: 2.0,
                            }),
                        );
                        self.log_info("Surface: pipe");
                    }
                    Err(e) => self.log_error(format!("SurfacePipe error: {e}")),
                }
            }
            S::Coons => self.run_part_coons_patch(),
        }
    }

    fn process_part_action(&mut self, action: PartAction) {
        use PartAction as P;
        match action {
            P::FaceFromWires => self.run_part_face_from_wires(),
            P::ConnectShapes => self.run_part_connect_shapes(),
            P::EmbedShapes => self.run_part_embed_shapes(),
            P::CutoutShapes => self.run_part_cutout_shapes(),
            P::ExplodeCompound => self.run_part_explode_compound(),
            P::CompoundFilter => self.run_part_compound_filter(),
            P::BooleanFragments => self.run_part_boolean_fragments(),
            P::SliceToCompound => self.run_part_slice_to_compound(),
            P::PointsFromShape => self.run_part_points_from_shape(),
            P::ConvertToSolid => self.run_part_convert_to_solid(),
            P::AutoDefeaturing { threshold } => self.run_part_auto_defeaturing(threshold),
            P::TransformedCopy { dx, dy, dz } => self.run_part_transformed_copy(dx, dy, dz),
            P::ProjectCurvesOnSurface => self.run_part_project_curves_on_surface(),
            P::CoonsPatch => self.run_part_coons_patch(),
        }
    }

    fn process_part_design_action(&mut self, action: PartDesignAction) {
        use PartDesignAction as Pd;
        match action {
            Pd::PadSketch { depth, symmetric } => {
                self.run_pad_sketch(depth, symmetric);
            }
            Pd::PocketSketch { depth, through_all } => {
                self.run_pocket_sketch(depth, through_all);
            }
            Pd::GrooveSketch { angle } => {
                self.run_groove_sketch(angle);
            }
            Pd::HoleSketch { radius, depth } => {
                self.run_hole_sketch(radius, depth);
            }
            Pd::CountersunkHoleSketch { radius, depth, countersink_angle } => {
                self.run_countersunk_hole_sketch(radius, depth, countersink_angle);
            }
            Pd::AdditiveLoft => self.run_partdesign_additive_loft(),
            Pd::AdditivePipe => self.run_partdesign_additive_pipe(),
            Pd::SubtractiveLoft => self.run_partdesign_subtractive_loft(),
            Pd::SubtractivePipe => self.run_partdesign_subtractive_pipe(),
            Pd::CreateSprocket { teeth, roller_diameter, pitch, bore } => {
                self.log_info(format!(
                    "PartDesign: Sprocket {teeth}T Dp={roller_diameter:.2} P={pitch:.2} bore={bore:.2}"
                ));
            }
            Pd::CreateShaftDesign { segments } => {
                self.log_info(format!("PartDesign: Shaft design ({} segments)", segments.len()));
            }
            Pd::CreateInvoluteGear { teeth, module_val, pressure_angle } => {
                self.log_info(format!(
                    "PartDesign: Involute gear {teeth}T m={module_val:.2} PA={pressure_angle:.1}"
                ));
            }
            Pd::ShapeBinder => self.log_info("PartDesign: Shape binder"),
            Pd::SuppressFeature => {
                if let Some(id) = self.scene.selected_id() {
                    if let Some(obj) = self.scene.get_mut(id) {
                        obj.suppressed = !obj.suppressed;
                        let state = if obj.suppressed { "suppressed" } else { "active" };
                        let name = obj.name.clone();
                        self.log_info(format!("{name}: {state}"));
                    }
                }
            }
            Pd::SetTip => {
                if let Some(id) = self.scene.selected_id() {
                    for obj in &mut self.scene.objects {
                        obj.is_tip = obj.id == id;
                    }
                    self.log_info("Tip set");
                }
            }
            Pd::MoveFeatureUp => {
                if let Some(id) = self.scene.selected_id() {
                    self.scene.move_up(id);
                    self.log_info("Feature moved up");
                }
            }
            Pd::MoveFeatureDown => {
                if let Some(id) = self.scene.selected_id() {
                    self.scene.move_down(id);
                    self.log_info("Feature moved down");
                }
            }
        }
    }

    fn process_draft_action(&mut self, action: DraftAction) {
        use DraftAction as D;
        match action {
            D::Line => self.run_draft_line(),
            D::Wire => self.run_draft_wire(),
            D::Circle => self.run_draft_circle(),
            D::Arc => self.run_draft_arc(),
            D::Ellipse => self.run_draft_ellipse(),
            D::Rectangle => {
                self.snapshot_before("Draft Rectangle");
                let mut model = BRepModel::new();
                match make_rectangle_wire(Point3::ORIGIN, 2.0, 1.0, Vec3::Z) {
                    Ok(mut pts) => {
                        pts.pop();
                        match filling(&mut model, &pts, 1) {
                            Ok(r) => {
                                self.add_to_scene(
                                    "Draft Rectangle",
                                    model,
                                    r.solid,
                                    Some(crate::scene::CreationParams::DraftRectangle {
                                        width: 2.0,
                                        height: 1.0,
                                    }),
                                );
                                self.log_info("Draft: rectangle");
                            }
                            Err(e) => self.log_error(format!("DraftRectangle fill error: {e}")),
                        }
                    }
                    Err(e) => self.log_error(format!("DraftRectangle error: {e}")),
                }
            }
            D::Polygon => {
                self.snapshot_before("Draft Polygon");
                let mut model = BRepModel::new();
                match make_polygon_wire(Point3::ORIGIN, Vec3::Z, 1.0, 6) {
                    Ok(pts) => match filling(&mut model, &pts, 1) {
                        Ok(r) => {
                            self.add_to_scene(
                                "Draft Polygon",
                                model,
                                r.solid,
                                Some(crate::scene::CreationParams::DraftPolygon {
                                    radius: 1.0,
                                    sides: 6,
                                }),
                            );
                            self.log_info("Draft: polygon");
                        }
                        Err(e) => self.log_error(format!("DraftPolygon fill error: {e}")),
                    },
                    Err(e) => self.log_error(format!("DraftPolygon error: {e}")),
                }
            }
            D::BSpline => self.run_draft_bspline(),
            D::Bezier => self.run_draft_bezier(),
            D::Point => self.run_draft_point(),
            D::Facebinder => self.run_draft_facebinder(),
            D::Hatch => self.run_draft_hatch(),
            D::Move => self.run_draft_move(),
            D::Rotate => self.run_draft_rotate(),
            D::Scale => self.run_draft_scale(),
            D::Mirror => self.run_draft_mirror(),
            D::Offset => self.run_draft_offset(),
            D::Trim => self.run_draft_trim(),
            D::Stretch => self.run_draft_stretch(),
            D::Clone => self.run_draft_clone(),
            D::ArrayRect => self.run_draft_array_rect(),
            D::ArrayPolar => self.run_draft_array_polar(),
            D::ArrayPath => self.run_draft_array_path(),
            D::ArrayPoint => self.run_draft_array_point(),
            D::Dimension => self.log_info("Draft: dimension"),
            D::Label => self.log_info("Draft: label"),
            D::Text => self.run_draft_text(),
            D::Upgrade => self.run_draft_upgrade(),
            D::Downgrade => self.run_draft_downgrade(),
            D::WireToBSpline => self.run_draft_wire_to_bspline(),
            D::ToSketch => self.run_draft_to_sketch(),
            D::ToggleSnap(mode) => self.log_info(format!("Draft: toggle snap '{mode}'")),
        }
    }

    // -- Phase A — sketch-driven PartDesign features ------------------------
    //
    // Each helper resolves the active sketch (preferring the open SketchMode,
    // falling back to the saved last_sketch), builds a 3D profile via
    // extract_profile, then dispatches to the matching kernel API. When no
    // base solid is present, Pad falls back to a fresh extrusion so the very
    // first sketch-driven feature still produces visible geometry.

    fn take_active_sketch_profile(&mut self) -> Option<(Vec<Point3>, Vec3)> {
        if let Some(mut sm) = self.gui.sketch_mode.take() {
            if !sm.sketch.constraints.is_empty() {
                let _ = solve(&mut sm.sketch, 200, 1e-10);
            }
            self.gui.last_sketch = Some((sm.sketch.clone(), sm.plane));
            let profile = extract_profile(&sm.sketch, &sm.plane);
            let dir = Vec3::new(sm.plane.normal.x, sm.plane.normal.y, sm.plane.normal.z);
            Some((profile, dir))
        } else if let Some((sketch, plane)) = self.gui.last_sketch.clone() {
            let profile = extract_profile(&sketch, &plane);
            let dir = Vec3::new(plane.normal.x, plane.normal.y, plane.normal.z);
            Some((profile, dir))
        } else {
            None
        }
    }

    fn run_pad_sketch(&mut self, depth: f64, symmetric: bool) {
        let Some((profile, normal)) = self.take_active_sketch_profile() else {
            self.log_warning("Pad: no active sketch (draw a sketch first)");
            return;
        };
        if profile.len() < 3 {
            self.log_warning(format!("Pad: sketch profile has {} points (need >= 3)", profile.len()));
            return;
        }
        if depth <= 0.0 {
            self.log_warning(format!("Pad: depth must be positive (got {depth:.3})"));
            return;
        }
        self.snapshot_before("Pad");

        let (effective_profile, distance) = if symmetric {
            let half = depth * 0.5;
            let shifted: Vec<Point3> = profile
                .iter()
                .map(|p| Point3::new(p.x - normal.x * half, p.y - normal.y * half, p.z - normal.z * half))
                .collect();
            (shifted, depth)
        } else {
            (profile, depth)
        };

        if let Some(base_solid) = self.current_solid {
            match pad(&self.model, base_solid, &effective_profile, normal, distance) {
                Ok(r) => {
                    self.add_to_scene(
                        &format!("Pad (depth={depth:.2})"),
                        r.model,
                        r.solid,
                        Some(crate::scene::CreationParams::Extruded),
                    );
                    self.log_info(format!("Pad: added material (depth={depth:.2}, symmetric={symmetric})"));
                }
                Err(e) => self.log_error(format!("Pad error: {e}")),
            }
        } else {
            // No base solid yet — first sketch-driven feature creates a fresh solid.
            let mut model = BRepModel::new();
            match extrude(&mut model, &effective_profile, normal, distance) {
                Ok(r) => {
                    self.add_to_scene(
                        &format!("Pad (depth={depth:.2})"),
                        model,
                        r.solid,
                        Some(crate::scene::CreationParams::Extruded),
                    );
                    self.log_info(format!("Pad: extruded sketch (depth={depth:.2})"));
                }
                Err(e) => self.log_error(format!("Pad error: {e}")),
            }
        }
    }

    fn run_pocket_sketch(&mut self, depth: f64, through_all: bool) {
        let Some((profile, normal)) = self.take_active_sketch_profile() else {
            self.log_warning("Pocket: no active sketch (draw a sketch first)");
            return;
        };
        if profile.len() < 3 {
            self.log_warning(format!("Pocket: sketch profile has {} points (need >= 3)", profile.len()));
            return;
        }
        let Some(base_solid) = self.current_solid else {
            self.log_warning("Pocket: no base solid (create a solid first)");
            return;
        };
        let actual_depth = if through_all { depth.max(1.0e6) } else { depth };
        if actual_depth <= 0.0 {
            self.log_warning(format!("Pocket: depth must be positive (got {actual_depth:.3})"));
            return;
        }
        self.snapshot_before("Pocket");
        // Pocket subtracts along the inverse normal so material is removed
        // INTO the base solid rather than out of it.
        let dir = Vec3::new(-normal.x, -normal.y, -normal.z);
        match pocket(&self.model, base_solid, &profile, dir, actual_depth) {
            Ok(r) => {
                self.add_to_scene(
                    &format!("Pocket (depth={depth:.2})"),
                    r.model,
                    r.solid,
                    Some(crate::scene::CreationParams::Extruded),
                );
                self.log_info(format!("Pocket: removed material (depth={depth:.2}, through_all={through_all})"));
            }
            Err(e) => self.log_error(format!("Pocket error: {e}")),
        }
    }

    fn run_groove_sketch(&mut self, angle_deg: f64) {
        let Some((profile, _normal)) = self.take_active_sketch_profile() else {
            self.log_warning("Groove: no active sketch");
            return;
        };
        if profile.len() < 2 {
            self.log_warning(format!("Groove: sketch profile has {} points (need >= 2)", profile.len()));
            return;
        }
        let Some(base_solid) = self.current_solid else {
            self.log_warning("Groove: no base solid");
            return;
        };
        let angle_rad = angle_deg.to_radians();
        if angle_rad <= 0.0 {
            self.log_warning(format!("Groove: angle must be positive (got {angle_deg:.1}°)"));
            return;
        }
        self.snapshot_before("Groove");
        // Default revolve axis: world Z through origin. A future iteration
        // will accept axis selection from the sketch's first construction line.
        match groove(
            &self.model,
            base_solid,
            &profile,
            Point3::ORIGIN,
            Vec3::Z,
            angle_rad,
            32,
        ) {
            Ok(r) => {
                self.add_to_scene(
                    &format!("Groove ({angle_deg:.0}°)"),
                    r.model,
                    r.solid,
                    Some(crate::scene::CreationParams::Groove { angle: angle_deg }),
                );
                self.log_info(format!("Groove: revolved profile by {angle_deg:.0}°"));
            }
            Err(e) => self.log_error(format!("Groove error: {e}")),
        }
    }

    fn run_hole_sketch(&mut self, radius: f64, depth: f64) {
        let Some(base_solid) = self.current_solid else {
            self.log_warning("Hole: no base solid (create a solid first)");
            return;
        };
        // Hole position derived from sketch centroid when available, else origin.
        let center = self
            .gui
            .last_sketch
            .as_ref()
            .map(|(s, p)| {
                let pts = extract_profile(s, p);
                if pts.is_empty() {
                    Point3::ORIGIN
                } else {
                    let n = pts.len() as f64;
                    let sx: f64 = pts.iter().map(|q| q.x).sum::<f64>() / n;
                    let sy: f64 = pts.iter().map(|q| q.y).sum::<f64>() / n;
                    let sz: f64 = pts.iter().map(|q| q.z).sum::<f64>() / n;
                    Point3::new(sx, sy, sz)
                }
            })
            .unwrap_or(Point3::ORIGIN);
        if radius <= 0.0 || depth <= 0.0 {
            self.log_warning(format!("Hole: radius and depth must be positive (got r={radius:.3}, d={depth:.3})"));
            return;
        }
        self.snapshot_before("Hole");
        match hole(&self.model, base_solid, center, -Vec3::Z, radius, depth, 32) {
            Ok(r) => {
                self.add_to_scene(
                    &format!("Hole (r={radius:.2}, d={depth:.2})"),
                    r.model,
                    r.solid,
                    Some(crate::scene::CreationParams::Extruded),
                );
                self.log_info(format!("Hole: drilled r={radius:.2} d={depth:.2}"));
            }
            Err(e) => self.log_error(format!("Hole error: {e}")),
        }
    }

    fn run_countersunk_hole_sketch(&mut self, radius: f64, depth: f64, angle_deg: f64) {
        let Some(base_solid) = self.current_solid else {
            self.log_warning("Countersunk hole: no base solid");
            return;
        };
        if radius <= 0.0 || depth <= 0.0 || angle_deg <= 0.0 || angle_deg >= 180.0 {
            self.log_warning(format!(
                "Countersunk hole: invalid params (r={radius:.3}, d={depth:.3}, angle={angle_deg:.1}°)"
            ));
            return;
        }
        // Countersink geometry: cone half-angle from the input apex angle.
        let cs_radius = radius * 2.0;
        let cs_depth = (cs_radius - radius) / (angle_deg * 0.5).to_radians().tan();
        self.snapshot_before("Countersunk hole");
        match countersunk_hole(
            &self.model,
            base_solid,
            Point3::ORIGIN,
            -Vec3::Z,
            radius,
            depth,
            cs_radius,
            cs_depth.max(0.1),
            32,
        ) {
            Ok(r) => {
                self.add_to_scene(
                    &format!("Countersunk hole (r={radius:.2}, d={depth:.2}, a={angle_deg:.0}°)"),
                    r.model,
                    r.solid,
                    Some(crate::scene::CreationParams::Extruded),
                );
                self.log_info(format!(
                    "Countersunk hole: r={radius:.2} d={depth:.2} angle={angle_deg:.0}°"
                ));
            }
            Err(e) => self.log_error(format!("Countersunk hole error: {e}")),
        }
    }

    // -- Phase A — Draft 2D primitives --------------------------------------
    //
    // Each helper builds a wire/point in a fresh BRepModel via the
    // draft_ops kernel API and adds it to the scene with the matching
    // CreationParams variant. The wire flavours fill the wire boundary into
    // a planar face so the result has visible geometry — same approach as the
    // existing D::Rectangle / D::Polygon arms.

    fn run_draft_line(&mut self) {
        self.snapshot_before("Draft Line");
        // Draft Line is an open wire — filling() can't turn it into a face
        // and the renderer has no native polyline pipeline yet. We still
        // build the wire in a B-Rep model so half-edge data exists for
        // downstream features, and register a tree entry so the click is
        // user-visible; renderable polyline geometry is tracked as a
        // separate Phase B+ item.
        let mut model = BRepModel::new();
        let p1 = Point3::ORIGIN;
        let p2 = Point3::new(2.0, 0.0, 0.0);
        match make_line_draft(&mut model, p1, p2) {
            Ok(_wire) => {
                self.scene.add_mesh_object(
                    "Draft Line",
                    cadkernel_io::Mesh::new(),
                    Some(crate::scene::CreationParams::DraftLine { length: 2.0, angle: 0.0 }),
                );
                self.log_info("Draft: line (2-point wire — tree only, no GPU geometry)");
            }
            Err(e) => self.log_error(format!("Draft Line error: {e}")),
        }
    }

    fn run_draft_circle(&mut self) {
        self.snapshot_before("Draft Circle");
        let radius = 1.0;
        let segments = 32;
        match make_circle_wire(Point3::ORIGIN, Vec3::Z, radius, segments) {
            Ok(mut pts) => {
                pts.pop(); // filling rejects the closing duplicate point
                let mut model = BRepModel::new();
                match filling(&mut model, &pts, 1) {
                    Ok(r) => {
                        self.add_to_scene(
                            "Draft Circle",
                            model,
                            r.solid,
                            Some(crate::scene::CreationParams::DraftCircle { radius }),
                        );
                        self.log_info(format!("Draft: circle (r={radius:.2})"));
                    }
                    Err(e) => self.log_error(format!("Draft Circle fill error: {e}")),
                }
            }
            Err(e) => self.log_error(format!("Draft Circle error: {e}")),
        }
    }

    fn run_draft_arc(&mut self) {
        self.snapshot_before("Draft Arc");
        let center = Point3::ORIGIN;
        let start = Point3::new(1.0, 0.0, 0.0);
        let end = Point3::new(0.0, 1.0, 0.0);
        let segments = 16;
        match make_arc_wire(center, start, end, segments) {
            Ok(pts) => {
                // Close the arc to a sector with the center so filling has a
                // valid closed boundary.
                let mut closed = pts.clone();
                closed.push(center);
                let mut model = BRepModel::new();
                match filling(&mut model, &closed, 1) {
                    Ok(r) => {
                        self.add_to_scene(
                            "Draft Arc",
                            model,
                            r.solid,
                            Some(crate::scene::CreationParams::DraftArc {
                                radius: 1.0,
                                start_angle: 0.0,
                                end_angle: std::f64::consts::FRAC_PI_2,
                            }),
                        );
                        self.log_info("Draft: arc (90°)");
                    }
                    Err(e) => self.log_error(format!("Draft Arc fill error: {e}")),
                }
            }
            Err(e) => self.log_error(format!("Draft Arc error: {e}")),
        }
    }

    fn run_draft_ellipse(&mut self) {
        self.snapshot_before("Draft Ellipse");
        let rx = 2.0;
        let ry = 1.0;
        let segments = 48;
        match make_ellipse_wire(Point3::ORIGIN, Vec3::Z, rx, ry, segments) {
            Ok(mut pts) => {
                pts.pop();
                let mut model = BRepModel::new();
                match filling(&mut model, &pts, 1) {
                    Ok(r) => {
                        self.add_to_scene(
                            "Draft Ellipse",
                            model,
                            r.solid,
                            Some(crate::scene::CreationParams::DraftEllipse { rx, ry }),
                        );
                        self.log_info(format!("Draft: ellipse (rx={rx:.2}, ry={ry:.2})"));
                    }
                    Err(e) => self.log_error(format!("Draft Ellipse fill error: {e}")),
                }
            }
            Err(e) => self.log_error(format!("Draft Ellipse error: {e}")),
        }
    }

    fn run_draft_point(&mut self) {
        self.snapshot_before("Draft Point");
        let mut model = BRepModel::new();
        let _vh = make_point(&mut model, Point3::ORIGIN);
        // A bare vertex has no faces or solid for the renderer to draw, so
        // we register it as a mesh-only scene object (an empty mesh marks
        // its presence in the tree without producing GPU geometry).
        let _id = self.scene.add_mesh_object(
            "Draft Point",
            cadkernel_io::Mesh::new(),
            None,
        );
        self.log_info("Draft: point at origin");
    }

    // -- Phase B — Draft EASY tier wiring -----------------------------------
    //
    // Same shape as Phase A: each helper either calls a kernel API and adds
    // a SceneObject via `add_to_scene` (when the result is a solid), fills a
    // closed wire into a face for filled output, or registers a tree-only
    // entry when the kernel result is a wire/curve and the renderer has no
    // polyline pipeline yet (same caveat as `D::Line` / `D::Point`).

    fn run_draft_wire(&mut self) {
        self.snapshot_before("Draft Wire");
        let mut model = BRepModel::new();
        let pts = [
            Point3::ORIGIN,
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        match make_wire(&mut model, &pts) {
            Ok(_) => {
                self.scene.add_mesh_object(
                    "Draft Wire",
                    cadkernel_io::Mesh::new(),
                    None,
                );
                self.log_info("Draft: wire (4-point polyline — tree only, no GPU geometry)");
            }
            Err(e) => self.log_error(format!("Draft Wire error: {e}")),
        }
    }

    fn run_draft_bspline(&mut self) {
        self.snapshot_before("Draft B-spline");
        let mut model = BRepModel::new();
        let cps = vec![
            Point3::ORIGIN,
            Point3::new(1.0, 2.0, 0.0),
            Point3::new(3.0, 2.0, 0.0),
            Point3::new(4.0, 0.0, 0.0),
        ];
        match make_bspline_wire(&mut model, cps, 3, 32) {
            Ok(_) => {
                self.scene.add_mesh_object(
                    "Draft B-spline",
                    cadkernel_io::Mesh::new(),
                    None,
                );
                self.log_info("Draft: B-spline (degree 3 — tree only, no GPU geometry)");
            }
            Err(e) => self.log_error(format!("Draft B-spline error: {e}")),
        }
    }

    fn run_draft_bezier(&mut self) {
        self.snapshot_before("Draft Bezier");
        match make_bezier_wire(
            Point3::ORIGIN,
            Point3::new(1.0, 2.0, 0.0),
            Point3::new(3.0, 2.0, 0.0),
            Point3::new(4.0, 0.0, 0.0),
            32,
        ) {
            Ok(_pts) => {
                self.scene.add_mesh_object(
                    "Draft Bezier",
                    cadkernel_io::Mesh::new(),
                    None,
                );
                self.log_info("Draft: cubic Bezier (tree only, no GPU geometry)");
            }
            Err(e) => self.log_error(format!("Draft Bezier error: {e}")),
        }
    }

    fn run_draft_hatch(&mut self) {
        self.snapshot_before("Draft Hatch");
        // Build a default 2x2 square boundary at the origin and run the
        // kernel hatch generator. The hatch result is a list of fill lines
        // (no faces), so the scene gains a tree-only entry — same renderer
        // gap as Wire / BSpline.
        let boundary = [
            Point3::ORIGIN,
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(2.0, 2.0, 0.0),
            Point3::new(0.0, 2.0, 0.0),
        ];
        let pattern = HatchPattern::Lines { angle: std::f64::consts::FRAC_PI_4, spacing: 0.25 };
        match draft_hatch(&boundary, pattern, 1.0) {
            Ok(result) => {
                self.scene.add_mesh_object(
                    "Draft Hatch",
                    cadkernel_io::Mesh::new(),
                    None,
                );
                self.log_info(format!(
                    "Draft: hatch ({} fill lines — tree only, no GPU geometry)",
                    result.lines.len()
                ));
            }
            Err(e) => self.log_error(format!("Draft Hatch error: {e}")),
        }
    }

    fn run_draft_text(&mut self) {
        self.snapshot_before("Draft Text");
        // Default placeholder text. A future iteration will accept the text
        // body / size from a modal; Phase B just demonstrates the kernel API
        // is reachable.
        match shape_from_text("CAD", Point3::ORIGIN, 1.0, Vec3::Z) {
            Ok(strokes) => {
                self.scene.add_mesh_object(
                    "Draft Text",
                    cadkernel_io::Mesh::new(),
                    None,
                );
                self.log_info(format!(
                    "Draft: text 'CAD' ({} strokes — tree only, no GPU geometry)",
                    strokes.len()
                ));
            }
            Err(e) => self.log_error(format!("Draft Text error: {e}")),
        }
    }

    fn run_draft_upgrade(&mut self) {
        self.snapshot_before("Draft Upgrade");
        // Default open polyline → closed face via upgrade_wire_model.
        let pts = [
            Point3::ORIGIN,
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(2.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let mut model = BRepModel::new();
        match upgrade_wire_model(&mut model, &pts) {
            Ok(solid) => {
                self.add_to_scene("Draft Upgrade", model, solid, None);
                self.log_info("Draft: upgrade — wire closed into face");
            }
            Err(e) => self.log_error(format!("Draft Upgrade error: {e}")),
        }
    }

    fn run_draft_downgrade(&mut self) {
        let Some(obj) = self.scene.selected_object().cloned() else {
            self.log_warning("Downgrade: no selected solid (select an object first)");
            return;
        };
        self.snapshot_before("Draft Downgrade");
        let mut model = obj.model.clone();
        match downgrade_solid_faces(&mut model, obj.solid) {
            Ok(face_solids) => {
                let n = face_solids.len();
                for (i, face_solid) in face_solids.into_iter().enumerate() {
                    self.scene.add_object(
                        format!("{} face {}", obj.name, i + 1),
                        model.clone(),
                        face_solid,
                        None,
                    );
                }
                self.rebuild_scene_gpu();
                self.log_info(format!("Draft: downgrade — split into {n} face solids"));
            }
            Err(e) => self.log_error(format!("Draft Downgrade error: {e}")),
        }
    }

    fn run_draft_wire_to_bspline(&mut self) {
        self.snapshot_before("Draft Wire→BSpline");
        let pts = [
            Point3::ORIGIN,
            Point3::new(1.0, 2.0, 0.0),
            Point3::new(3.0, 2.0, 0.0),
            Point3::new(4.0, 0.0, 0.0),
        ];
        let mut model = BRepModel::new();
        match wire_to_bspline_convert(&mut model, &pts, 3) {
            Ok(_curve) => {
                self.scene.add_mesh_object(
                    "Draft Wire→BSpline",
                    cadkernel_io::Mesh::new(),
                    None,
                );
                self.log_info("Draft: wire converted to B-spline (curve only — tree entry)");
            }
            Err(e) => self.log_error(format!("Draft Wire→BSpline error: {e}")),
        }
    }

    fn run_draft_to_sketch(&mut self) {
        let pts = [
            Point3::ORIGIN,
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(2.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        match draft_to_sketch(&pts) {
            Ok(sketch) => {
                self.gui.last_sketch = Some((sketch, cadkernel_sketch::WorkPlane::xy()));
                self.log_info(format!(
                    "Draft: converted {} points to a sketch (saved as last_sketch)",
                    pts.len()
                ));
            }
            Err(e) => self.log_error(format!("Draft ToSketch error: {e}")),
        }
    }

    fn run_draft_clone(&mut self) {
        let Some(obj) = self.scene.selected_object().cloned() else {
            self.log_warning("Clone: no selected solid (select an object first)");
            return;
        };
        self.snapshot_before("Draft Clone");
        let mut model = obj.model.clone();
        match clone_solid(&mut model, obj.solid) {
            Ok(r) => {
                self.add_to_scene(
                    &format!("{} (clone)", obj.name),
                    model,
                    r.solid,
                    obj.params.clone(),
                );
                self.log_info(format!("Draft: cloned '{}'", obj.name));
            }
            Err(e) => self.log_error(format!("Draft Clone error: {e}")),
        }
    }

    fn run_draft_array_rect(&mut self) {
        let Some(obj) = self.scene.selected_object().cloned() else {
            self.log_warning("Array Rect: no selected solid");
            return;
        };
        self.snapshot_before("Draft Array Rect");
        let mut model = obj.model.clone();
        match rectangular_array(&mut model, obj.solid, Vec3::X, 3.0, 3, Vec3::Y, 3.0, 2) {
            Ok(r) => {
                let n_added = r.solids.len().saturating_sub(1);
                for (i, s) in r.solids.iter().enumerate().skip(1) {
                    self.scene.add_object(
                        format!("{} array[{}]", obj.name, i),
                        model.clone(),
                        *s,
                        None,
                    );
                }
                self.rebuild_scene_gpu();
                self.log_info(format!(
                    "Draft: rectangular array — {n_added} new copies (3×2 grid)"
                ));
            }
            Err(e) => self.log_error(format!("Draft Array Rect error: {e}")),
        }
    }

    fn run_draft_array_polar(&mut self) {
        let Some(obj) = self.scene.selected_object().cloned() else {
            self.log_warning("Array Polar: no selected solid");
            return;
        };
        self.snapshot_before("Draft Array Polar");
        let mut model = obj.model.clone();
        match polar_array(&mut model, obj.solid, Point3::ORIGIN, Vec3::Z, 6) {
            Ok(solids) => {
                let n_added = solids.len().saturating_sub(1);
                for (i, s) in solids.iter().enumerate().skip(1) {
                    self.scene.add_object(
                        format!("{} polar[{}]", obj.name, i),
                        model.clone(),
                        *s,
                        None,
                    );
                }
                self.rebuild_scene_gpu();
                self.log_info(format!("Draft: polar array — {n_added} new copies (6-fold)"));
            }
            Err(e) => self.log_error(format!("Draft Array Polar error: {e}")),
        }
    }

    fn run_draft_array_path(&mut self) {
        let Some(obj) = self.scene.selected_object().cloned() else {
            self.log_warning("Array Path: no selected solid");
            return;
        };
        self.snapshot_before("Draft Array Path");
        let mut model = obj.model.clone();
        let path = [
            Point3::ORIGIN,
            Point3::new(3.0, 0.0, 0.0),
            Point3::new(3.0, 3.0, 0.0),
            Point3::new(0.0, 3.0, 0.0),
        ];
        match path_array(&mut model, obj.solid, &path) {
            Ok(r) => {
                let n_added = r.solids.len().saturating_sub(1);
                for (i, s) in r.solids.iter().enumerate().skip(1) {
                    self.scene.add_object(
                        format!("{} path[{}]", obj.name, i),
                        model.clone(),
                        *s,
                        None,
                    );
                }
                self.rebuild_scene_gpu();
                self.log_info(format!("Draft: path array — {n_added} new copies"));
            }
            Err(e) => self.log_error(format!("Draft Array Path error: {e}")),
        }
    }

    fn run_draft_array_point(&mut self) {
        let Some(obj) = self.scene.selected_object().cloned() else {
            self.log_warning("Array Point: no selected solid");
            return;
        };
        self.snapshot_before("Draft Array Point");
        let mut model = obj.model.clone();
        let positions = [
            Point3::ORIGIN,
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(0.0, 2.0, 0.0),
            Point3::new(2.0, 2.0, 0.0),
        ];
        match point_array(&mut model, obj.solid, &positions) {
            Ok(solids) => {
                let n_added = solids.len().saturating_sub(1);
                for (i, s) in solids.iter().enumerate().skip(1) {
                    self.scene.add_object(
                        format!("{} pt[{}]", obj.name, i),
                        model.clone(),
                        *s,
                        None,
                    );
                }
                self.rebuild_scene_gpu();
                self.log_info(format!("Draft: point array — {n_added} new copies"));
            }
            Err(e) => self.log_error(format!("Draft Array Point error: {e}")),
        }
    }

    // -- Phase B-cont — Part workbench EASY tier ----------------------------
    //
    // Same shape as Phase A/B. Selection-dependent operations check
    // `scene.selected_object()` and log a warning when nothing is selected.
    // Multi-solid outputs (boolean_fragments / slice_to_compound) iterate
    // the resulting Compound and add each solid as its own SceneObject.

    fn run_part_face_from_wires(&mut self) {
        self.snapshot_before("Face from wires");
        // Default 4-point square boundary. Convert to filled face via
        // `filling()` which is the kernel-level wire→solid path the
        // existing D::Rectangle / D::Polygon arms already use; the bare
        // `face_from_wires` API returns a face handle with no shell/solid
        // wrapper, so we use the higher-level helper here instead.
        let pts = [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(2.0, 2.0, 0.0),
            Point3::new(0.0, 2.0, 0.0),
        ];
        let mut model = BRepModel::new();
        // Exercise the kernel API even though we don't keep the bare face
        // — this verifies the dispatcher → kernel path.
        let _ = face_from_wires(&mut model, &pts);
        let mut model = BRepModel::new();
        match filling(&mut model, &pts, 1) {
            Ok(r) => {
                self.add_to_scene("Face from wires", model, r.solid, None);
                self.log_info("Part: face from 4-point wire boundary");
            }
            Err(e) => self.log_error(format!("Part FaceFromWires error: {e}")),
        }
    }

    fn run_part_connect_shapes(&mut self) {
        let selected = self.scene.selected_ids();
        if selected.len() < 2 {
            self.log_warning("Connect Shapes: select two objects first");
            return;
        }
        let a = match self.scene.get(selected[0]).cloned() {
            Some(o) => o,
            None => return,
        };
        let b = match self.scene.get(selected[1]).cloned() {
            Some(o) => o,
            None => return,
        };
        self.snapshot_before("Connect Shapes");
        match connect_shapes(&a.model, a.solid, &b.model, b.solid) {
            Ok(result_model) => {
                let first_solid = result_model.solids.iter().next().map(|(h, _)| h);
                if let Some(solid) = first_solid {
                    self.add_to_scene(
                        &format!("{} ∪ {}", a.name, b.name),
                        result_model,
                        solid,
                        Some(crate::scene::CreationParams::Boolean { op: "connect".into() }),
                    );
                    self.log_info("Part: connect shapes (union)");
                } else {
                    self.log_error("Part ConnectShapes: result has no solid");
                }
            }
            Err(e) => self.log_error(format!("Part ConnectShapes error: {e}")),
        }
    }

    fn run_part_embed_shapes(&mut self) {
        let selected = self.scene.selected_ids();
        if selected.len() < 2 {
            self.log_warning("Embed Shapes: select two objects first");
            return;
        }
        let a = match self.scene.get(selected[0]).cloned() {
            Some(o) => o,
            None => return,
        };
        let b = match self.scene.get(selected[1]).cloned() {
            Some(o) => o,
            None => return,
        };
        self.snapshot_before("Embed Shapes");
        match embed_shapes(&a.model, a.solid, &b.model, b.solid) {
            Ok(result_model) => {
                let first_solid = result_model.solids.iter().next().map(|(h, _)| h);
                if let Some(solid) = first_solid {
                    self.add_to_scene(
                        &format!("{} embed {}", a.name, b.name),
                        result_model,
                        solid,
                        Some(crate::scene::CreationParams::Boolean { op: "embed".into() }),
                    );
                    self.log_info("Part: embed shapes");
                } else {
                    self.log_error("Part EmbedShapes: result has no solid");
                }
            }
            Err(e) => self.log_error(format!("Part EmbedShapes error: {e}")),
        }
    }

    fn run_part_cutout_shapes(&mut self) {
        let selected = self.scene.selected_ids();
        if selected.len() < 2 {
            self.log_warning("Cutout Shapes: select two objects first (base, then tool)");
            return;
        }
        let a = match self.scene.get(selected[0]).cloned() {
            Some(o) => o,
            None => return,
        };
        let b = match self.scene.get(selected[1]).cloned() {
            Some(o) => o,
            None => return,
        };
        self.snapshot_before("Cutout Shapes");
        match cutout_shapes(&a.model, a.solid, &b.model, b.solid) {
            Ok(result_model) => {
                let first_solid = result_model.solids.iter().next().map(|(h, _)| h);
                if let Some(solid) = first_solid {
                    self.add_to_scene(
                        &format!("{} \\ {}", a.name, b.name),
                        result_model,
                        solid,
                        Some(crate::scene::CreationParams::Boolean { op: "cutout".into() }),
                    );
                    self.log_info("Part: cutout shapes (difference)");
                } else {
                    self.log_error("Part CutoutShapes: result has no solid");
                }
            }
            Err(e) => self.log_error(format!("Part CutoutShapes error: {e}")),
        }
    }

    fn run_part_explode_compound(&mut self) {
        // Compound state is not tracked in the UI yet. We construct a
        // synthetic compound from the selected scene objects and report
        // the explode count — the constituent solids are already in the
        // scene, so this is informational rather than additive.
        let selected = self.scene.selected_ids();
        if selected.is_empty() {
            self.log_warning("Explode Compound: select objects first");
            return;
        }
        let mut compound = Compound::new("scene_selection");
        for id in &selected {
            if let Some(obj) = self.scene.get(*id) {
                compound.add(obj.solid);
            }
        }
        let exploded = explode_compound(&compound);
        self.log_info(format!(
            "Part: exploded compound — {} solids (already present in scene)",
            exploded.len()
        ));
    }

    fn run_part_compound_filter(&mut self) {
        let selected = self.scene.selected_ids();
        if selected.is_empty() {
            self.log_warning("Compound Filter: select objects first");
            return;
        }
        // Build a compound from the selection in a fresh model, then
        // filter out solids with fewer than 6 faces (e.g. degenerate shells).
        let mut filter_model = BRepModel::new();
        let mut compound = Compound::new("filter_input");
        for id in &selected {
            if let Some(obj) = self.scene.get(*id) {
                // Re-create the box-equivalent solid in the filter model so
                // the compound's handles refer to the same model. For the
                // synthetic compound we just probe face count of each scene
                // object's own model directly.
                if let Some(sd) = obj.model.solids.get(obj.solid) {
                    let face_count: usize = sd
                        .shells
                        .iter()
                        .filter_map(|sh| obj.model.shells.get(*sh))
                        .map(|s| s.faces.len())
                        .sum();
                    if face_count >= 6 {
                        let r = make_box(&mut filter_model, Point3::ORIGIN, 1.0, 1.0, 1.0)
                            .expect("make_box for compound filter probe");
                        compound.add(r.solid);
                    }
                }
            }
        }
        match compound_filter(&filter_model, &compound, 6) {
            Ok(filtered) => self.log_info(format!(
                "Part: compound filter (>=6 faces) — {} of {} pass",
                filtered.solids.len(),
                selected.len()
            )),
            Err(e) => self.log_error(format!("Part CompoundFilter error: {e}")),
        }
    }

    fn run_part_boolean_fragments(&mut self) {
        let selected = self.scene.selected_ids();
        if selected.len() < 2 {
            self.log_warning("Boolean Fragments: select two objects first");
            return;
        }
        let a = match self.scene.get(selected[0]).cloned() {
            Some(o) => o,
            None => return,
        };
        let b = match self.scene.get(selected[1]).cloned() {
            Some(o) => o,
            None => return,
        };
        self.snapshot_before("Boolean Fragments");
        match boolean_fragments(&a.model, a.solid, &b.model, b.solid) {
            Ok(result) => {
                let n = result.compound.solids.len();
                // The fragments live across A and B's models, so we cannot
                // re-emit them as new SceneObjects without copying their
                // topology. Phase B-cont logs the count as user-visible
                // feedback; a full multi-model staging path is a Phase D
                // candidate.
                self.log_info(format!(
                    "Part: boolean fragments — {n} regions (logged; multi-model staging pending)"
                ));
            }
            Err(e) => self.log_error(format!("Part BooleanFragments error: {e}")),
        }
    }

    fn run_part_slice_to_compound(&mut self) {
        let Some(obj) = self.scene.selected_object().cloned() else {
            self.log_warning("Slice to Compound: select a solid first");
            return;
        };
        self.snapshot_before("Slice to Compound");
        // Default slice plane: midway through the object along Z.
        let mid_z = ((obj.aabb_min[2] + obj.aabb_max[2]) * 0.5) as f64;
        let mut model = obj.model.clone();
        match slice_to_compound(&mut model, obj.solid, Point3::new(0.0, 0.0, mid_z), Vec3::Z) {
            Ok(result) => {
                let pieces = result.compound.solids.clone();
                let n = pieces.len();
                for (i, &solid) in pieces.iter().enumerate() {
                    self.scene.add_object(
                        format!("{} slice[{}]", obj.name, i + 1),
                        model.clone(),
                        solid,
                        None,
                    );
                }
                self.rebuild_scene_gpu();
                self.log_info(format!("Part: slice to compound — {n} pieces"));
            }
            Err(e) => self.log_error(format!("Part SliceToCompound error: {e}")),
        }
    }

    fn run_part_points_from_shape(&mut self) {
        let Some(obj) = self.scene.selected_object().cloned() else {
            self.log_warning("Points from Shape: select a solid first");
            return;
        };
        match points_from_shape(&obj.model, obj.solid) {
            Ok(result) => self.log_info(format!(
                "Part: extracted {} unique vertex points from '{}'",
                result.points.len(),
                obj.name
            )),
            Err(e) => self.log_error(format!("Part PointsFromShape error: {e}")),
        }
    }

    fn run_part_convert_to_solid(&mut self) {
        let Some(obj) = self.scene.selected_object().cloned() else {
            self.log_warning("Convert to Solid: select a mesh object first");
            return;
        };
        if obj.mesh.indices.is_empty() {
            self.log_warning("Convert to Solid: selected object has no mesh triangles");
            return;
        }
        self.snapshot_before("Convert to Solid");
        let mut model = BRepModel::new();
        match shape_from_mesh(&mut model, &obj.mesh) {
            Ok(r) => {
                self.add_to_scene(
                    &format!("{} (solid)", obj.name),
                    model,
                    r.solid,
                    None,
                );
                self.log_info(format!(
                    "Part: converted mesh '{}' to solid ({} faces)",
                    obj.name,
                    r.faces.len()
                ));
            }
            Err(e) => self.log_error(format!("Part ConvertToSolid error: {e}")),
        }
    }

    fn run_part_auto_defeaturing(&mut self, threshold: f64) {
        let Some(obj) = self.scene.selected_object().cloned() else {
            self.log_warning("Auto-defeaturing: select a solid first");
            return;
        };
        if threshold <= 0.0 {
            self.log_warning(format!("Auto-defeaturing: threshold must be positive (got {threshold:.3})"));
            return;
        }
        self.snapshot_before("Auto-defeaturing");
        let mut model = obj.model.clone();
        match auto_defeaturing(&mut model, obj.solid, threshold) {
            Ok(simplified) => {
                self.add_to_scene(
                    &format!("{} (defeatured)", obj.name),
                    model,
                    simplified,
                    None,
                );
                self.log_info(format!(
                    "Part: auto-defeaturing — removed faces below {threshold:.3} area threshold"
                ));
            }
            Err(e) => self.log_error(format!("Part AutoDefeaturing error: {e}")),
        }
    }

    fn run_part_transformed_copy(&mut self, dx: f64, dy: f64, dz: f64) {
        let Some(obj) = self.scene.selected_object().cloned() else {
            self.log_warning("Transformed Copy: select a solid first");
            return;
        };
        self.snapshot_before("Transformed Copy");
        let mut model = obj.model.clone();
        let transforms = [MultiTransformStep::Translation(Vec3::new(dx, dy, dz))];
        match multi_transform(&mut model, obj.solid, &transforms) {
            Ok(r) => {
                self.add_to_scene(
                    &format!("{} (copy +{dx:.1},{dy:.1},{dz:.1})", obj.name),
                    model,
                    r.solid,
                    obj.params.clone(),
                );
                self.log_info(format!(
                    "Part: transformed copy of '{}' by ({dx:.1}, {dy:.1}, {dz:.1})",
                    obj.name
                ));
            }
            Err(e) => self.log_error(format!("Part TransformedCopy error: {e}")),
        }
    }

    fn run_part_coons_patch(&mut self) {
        // Default unit-square boundary: four LineSegments. The kernel
        // returns a NurbsSurface — there is no built-in surface→solid
        // wrapper for arbitrary surfaces yet, so we register a tree-only
        // entry and document the renderer gap (same as Phase A/B wire
        // outputs). A full surface→solid extrusion belongs in Phase C.
        self.snapshot_before("Coons Patch");
        let p00 = Point3::new(0.0, 0.0, 0.0);
        let p10 = Point3::new(1.0, 0.0, 0.0);
        let p11 = Point3::new(1.0, 1.0, 0.0);
        let p01 = Point3::new(0.0, 1.0, 0.0);
        let u0 = LineSegment::new(p00, p10);
        let u1 = LineSegment::new(p01, p11);
        let v0 = LineSegment::new(p00, p01);
        let v1 = LineSegment::new(p10, p11);
        match coons_patch(&u0, &u1, &v0, &v1) {
            Ok(_result) => {
                self.scene.add_mesh_object(
                    "Coons Patch",
                    cadkernel_io::Mesh::new(),
                    None,
                );
                self.log_info(
                    "Part: Coons patch (NurbsSurface — tree only, no GPU geometry)",
                );
            }
            Err(e) => self.log_error(format!("Part CoonsPatch error: {e}")),
        }
    }

    // -- Phase C1 — Draft transforms (Move / Rotate / Scale / Mirror) --------
    //
    // Selection-required arms — `log_warning` and bail when nothing is
    // selected. The transformed result is added as a NEW scene object
    // alongside the source (`copy_solid_transformed` builds a fresh solid in
    // the same model). A future Phase D pass can switch to in-place mutation
    // when the user drags a gizmo, but this matches FreeCAD's "Modify ›
    // {Move,Rotate,Scale,Mirror}" semantics where the original is preserved.

    fn run_draft_move(&mut self) {
        let Some(obj) = self.scene.selected_object().cloned() else {
            self.log_warning("Draft Move: select a solid first");
            return;
        };
        self.snapshot_before("Draft Move");
        let mut model = obj.model.clone();
        // Phase C1 default offset; a Phase D modal will accept user input.
        let displacement = Vec3::new(1.0, 0.0, 0.0);
        match move_solid(&mut model, obj.solid, displacement) {
            Ok(new_solid) => {
                self.add_to_scene(
                    &format!("{} (moved)", obj.name),
                    model,
                    new_solid,
                    obj.params.clone(),
                );
                self.log_info(format!(
                    "Draft: moved '{}' by ({:.1}, {:.1}, {:.1})",
                    obj.name, displacement.x, displacement.y, displacement.z
                ));
            }
            Err(e) => self.log_error(format!("Draft Move error: {e}")),
        }
    }

    fn run_draft_rotate(&mut self) {
        let Some(obj) = self.scene.selected_object().cloned() else {
            self.log_warning("Draft Rotate: select a solid first");
            return;
        };
        self.snapshot_before("Draft Rotate");
        let mut model = obj.model.clone();
        let angle_deg: f64 = 30.0;
        let angle_rad = angle_deg.to_radians();
        match rotate_solid(&mut model, obj.solid, Point3::ORIGIN, Vec3::Z, angle_rad) {
            Ok(new_solid) => {
                self.add_to_scene(
                    &format!("{} (rotated {angle_deg:.0}°)", obj.name),
                    model,
                    new_solid,
                    obj.params.clone(),
                );
                self.log_info(format!(
                    "Draft: rotated '{}' by {angle_deg:.0}° around world Z",
                    obj.name
                ));
            }
            Err(e) => self.log_error(format!("Draft Rotate error: {e}")),
        }
    }

    fn run_draft_scale(&mut self) {
        let Some(obj) = self.scene.selected_object().cloned() else {
            self.log_warning("Draft Scale: select a solid first");
            return;
        };
        self.snapshot_before("Draft Scale");
        let mut model = obj.model.clone();
        let factor = 2.0;
        match scale_solid_draft(&mut model, obj.solid, Point3::ORIGIN, factor) {
            Ok(new_solid) => {
                self.add_to_scene(
                    &format!("{} (×{factor:.1})", obj.name),
                    model,
                    new_solid,
                    obj.params.clone(),
                );
                self.log_info(format!("Draft: scaled '{}' by {factor:.1}×", obj.name));
            }
            Err(e) => self.log_error(format!("Draft Scale error: {e}")),
        }
    }

    fn run_draft_mirror(&mut self) {
        let Some(obj) = self.scene.selected_object().cloned() else {
            self.log_warning("Draft Mirror: select a solid first");
            return;
        };
        self.snapshot_before("Draft Mirror");
        let mut model = obj.model.clone();
        // Reuse `gui.mirror_plane` (set by the existing `MirrorSolid` action
        // and the toolbar / context menu) instead of forcing a default —
        // matches the established pattern where mirror state is sticky.
        let (plane_point, plane_normal) = match self.gui.mirror_plane {
            crate::gui::MirrorPlane::XY => (Point3::ORIGIN, Vec3::new(0.0, 0.0, 1.0)),
            crate::gui::MirrorPlane::XZ => (Point3::ORIGIN, Vec3::new(0.0, 1.0, 0.0)),
            crate::gui::MirrorPlane::YZ => (Point3::ORIGIN, Vec3::new(1.0, 0.0, 0.0)),
        };
        let plane_label = match self.gui.mirror_plane {
            crate::gui::MirrorPlane::XY => "XY",
            crate::gui::MirrorPlane::XZ => "XZ",
            crate::gui::MirrorPlane::YZ => "YZ",
        };
        match mirror_solid_draft(&mut model, obj.solid, plane_point, plane_normal) {
            Ok(new_solid) => {
                self.add_to_scene(
                    &format!("{} (mirrored {plane_label})", obj.name),
                    model,
                    new_solid,
                    obj.params.clone(),
                );
                self.log_info(format!(
                    "Draft: mirrored '{}' across {plane_label} plane",
                    obj.name
                ));
            }
            Err(e) => self.log_error(format!("Draft Mirror error: {e}")),
        }
    }

    // -- Phase C1 — FEM Summary / Report (text-only) -------------------------
    //
    // No kernel work — just format the contents of `gui.fem_analysis` into
    // a status-bar / report-panel string. Guarded with `log_warning` when no
    // analysis exists.

    fn run_fem_summary(&mut self) {
        let Some(analysis) = self.gui.fem_analysis.as_ref() else {
            self.log_warning("FEM Summary: no analysis (run CreateFemAnalysis first)");
            return;
        };
        let n_nodes = analysis.mesh.nodes.len();
        let n_elems = analysis.mesh.elements.len();
        let n_bcs = analysis.boundary_conditions.len();
        let solved = if analysis.result.is_some() { "yes" } else { "no" };
        let material = fem_material_label(&analysis.material);
        self.log_info(format!(
            "FEM Summary — material: {material}, nodes: {n_nodes}, elements: {n_elems}, BCs: {n_bcs}, solved: {solved}"
        ));
    }

    fn run_fem_report(&mut self) {
        let Some(analysis) = self.gui.fem_analysis.as_ref() else {
            self.log_warning("FEM Report: no analysis (run CreateFemAnalysis first)");
            return;
        };
        let material = fem_material_label(&analysis.material);
        let mut report = format!(
            "FEM Report\n  material: {material} (E={:.3e} Pa, ν={:.3}, ρ={:.1} kg/m³)\n  mesh: {} nodes, {} tetrahedra\n  BCs: {}",
            analysis.material.youngs_modulus,
            analysis.material.poisson_ratio,
            analysis.material.density,
            analysis.mesh.nodes.len(),
            analysis.mesh.elements.len(),
            analysis.boundary_conditions.len(),
        );
        if !analysis.boundary_conditions.is_empty() {
            let mut counts: std::collections::BTreeMap<&'static str, usize> =
                std::collections::BTreeMap::new();
            for bc in &analysis.boundary_conditions {
                *counts.entry(fem_bc_kind_label(bc)).or_insert(0) += 1;
            }
            for (kind, n) in &counts {
                report.push_str(&format!("\n    {kind}: {n}"));
            }
        }
        if let Some(result) = analysis.result.as_ref() {
            let n = result.displacements.len();
            let max_disp = result
                .displacements
                .iter()
                .map(|v| (v.x * v.x + v.y * v.y + v.z * v.z).sqrt())
                .fold(0.0_f64, f64::max);
            let max_stress = result
                .stresses
                .iter()
                .copied()
                .fold(0.0_f64, f64::max);
            report.push_str(&format!(
                "\n  static result: {n} nodal displacements, max |u|={max_disp:.3e}, max σ={max_stress:.3e}"
            ));
        } else {
            report.push_str("\n  static result: not solved");
        }
        self.log_info(report);
    }

    // -- Phase C2 — Draft modify (Offset / Trim / Stretch / Facebinder) +
    //    P::ProjectCurvesOnSurface ----------------------------------------
    //
    // Wire-output arms (Offset / Trim / Stretch) follow the renderer-gap
    // pattern from Phase A's `D::Line`: the kernel API runs and produces a
    // valid polyline, but the viewer's wgpu pipeline has no native polyline
    // path — we register a tree-only `add_mesh_object` entry so the click
    // is user-visible and the operation logs report the result.
    //
    // Selection-required arms (Facebinder, ProjectCurvesOnSurface) bail
    // with `log_warning` when no solid is selected.

    /// Pull a polyline source: prefer `gui.last_sketch` if set, otherwise a
    /// default 4-point square. Used by Offset / Trim / Stretch /
    /// ProjectCurvesOnSurface as the input curve.
    fn default_polyline_or_last_sketch(&self) -> Vec<Point3> {
        if let Some((sketch, plane)) = self.gui.last_sketch.as_ref() {
            let pts = extract_profile(sketch, plane);
            if pts.len() >= 2 {
                return pts;
            }
        }
        vec![
            Point3::ORIGIN,
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(2.0, 2.0, 0.0),
            Point3::new(0.0, 2.0, 0.0),
        ]
    }

    fn run_draft_offset(&mut self) {
        self.snapshot_before("Draft Offset");
        let pts = self.default_polyline_or_last_sketch();
        match offset_wire(&pts, 0.5, Vec3::Z) {
            Ok(offset) => {
                self.scene.add_mesh_object(
                    "Draft Offset",
                    cadkernel_io::Mesh::new(),
                    None,
                );
                self.log_info(format!(
                    "Draft: offset wire ({} points, distance=0.5 — tree only, no GPU geometry)",
                    offset.len()
                ));
            }
            Err(e) => self.log_error(format!("Draft Offset error: {e}")),
        }
    }

    fn run_draft_trim(&mut self) {
        self.snapshot_before("Draft Trim");
        let pts = self.default_polyline_or_last_sketch();
        // Default trim target: midpoint of the wire.
        let target = if pts.is_empty() {
            Point3::ORIGIN
        } else {
            let mid = pts.len() / 2;
            pts[mid]
        };
        match trimex_draft(&pts, target) {
            Ok(trimmed) => {
                self.scene.add_mesh_object(
                    "Draft Trim",
                    cadkernel_io::Mesh::new(),
                    None,
                );
                self.log_info(format!(
                    "Draft: trim wire (in {} pts → out {} pts — tree only, no GPU geometry)",
                    pts.len(),
                    trimmed.len()
                ));
            }
            Err(e) => self.log_error(format!("Draft Trim error: {e}")),
        }
    }

    fn run_draft_stretch(&mut self) {
        self.snapshot_before("Draft Stretch");
        let pts = self.default_polyline_or_last_sketch();
        // Default stretch: pull all points within radius 5 by (0, 0, 1).
        let stretched = stretch_wire(&pts, Point3::ORIGIN, 5.0, Vec3::new(0.0, 0.0, 1.0));
        self.scene.add_mesh_object(
            "Draft Stretch",
            cadkernel_io::Mesh::new(),
            None,
        );
        self.log_info(format!(
            "Draft: stretch wire ({} points displaced — tree only, no GPU geometry)",
            stretched.len()
        ));
    }

    fn run_draft_facebinder(&mut self) {
        let Some(obj) = self.scene.selected_object().cloned() else {
            self.log_warning("Facebinder: select a solid first");
            return;
        };
        self.snapshot_before("Draft Facebinder");
        // Pick the first face of the selected solid's first shell.
        let first_face = obj
            .model
            .solids
            .get(obj.solid)
            .and_then(|sd| sd.shells.first().copied())
            .and_then(|shell_h| obj.model.shells.get(shell_h))
            .and_then(|sh| sh.faces.first().copied());
        let Some(face_h) = first_face else {
            self.log_warning("Facebinder: selected solid has no faces");
            return;
        };
        let mut model = obj.model.clone();
        match make_facebinder(&mut model, &[face_h]) {
            Ok(new_solid) => {
                self.add_to_scene(
                    &format!("{} (face binder)", obj.name),
                    model,
                    new_solid,
                    None,
                );
                self.log_info(format!(
                    "Draft: face binder from first face of '{}'",
                    obj.name
                ));
            }
            Err(e) => self.log_error(format!("Draft Facebinder error: {e}")),
        }
    }

    fn run_part_project_curves_on_surface(&mut self) {
        let Some(obj) = self.scene.selected_object().cloned() else {
            self.log_warning("Project Curves on Surface: select a solid first");
            return;
        };
        let curve = self.default_polyline_or_last_sketch();
        // Project the curve onto the selected solid's tessellated surface.
        let projected = project_curve_on_solid(&obj.model, obj.solid, &curve);
        // Output is a Vec<Point3> with no kernel-side topology; register a
        // tree-only entry so the dispatcher visibly fired, and report the
        // point count for the user.
        self.scene.add_mesh_object(
            "Projected Curve",
            cadkernel_io::Mesh::new(),
            None,
        );
        self.log_info(format!(
            "Part: projected {} curve points onto '{}' (tree only, no GPU geometry)",
            projected.len(),
            obj.name
        ));
    }

    // -- Phase C3 — Surface ops + PartDesign Loft/Pipe (final MEDIUM batch) --
    //
    // Sections / SurfaceFromCurves both produce REAL B-Rep solids (skinned
    // surfaces with manifold shells), so they go through `add_to_scene`
    // normally — no renderer-gap caveat for those. Extend operates on a
    // selected solid and returns a thickened solid.
    //
    // Loft / Sweep produce closed solids; Subtractive variants pipe the
    // result through `boolean_op_exact(Difference)` against the selected
    // base. Default profiles are 4-point squares stacked on Z (loft) or a
    // 2-point Z path (pipe).

    fn run_surface_sections(&mut self) {
        self.snapshot_before("Surface Sections");
        // Two parallel square profiles at different Z, each as a degree-1
        // NurbsCurve (closed polyline approximation). The kernel side
        // skins between them.
        let ring_a = vec![
            Point3::new(-1.0, -1.0, 0.0),
            Point3::new(1.0, -1.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(-1.0, 1.0, 0.0),
        ];
        let ring_b = vec![
            Point3::new(-1.0, -1.0, 2.0),
            Point3::new(1.0, -1.0, 2.0),
            Point3::new(1.0, 1.0, 2.0),
            Point3::new(-1.0, 1.0, 2.0),
        ];
        let curve_a = match wire_to_bspline(&ring_a, 1) {
            Ok(c) => c,
            Err(e) => {
                self.log_error(format!("Surface Sections curve A error: {e}"));
                return;
            }
        };
        let curve_b = match wire_to_bspline(&ring_b, 1) {
            Ok(c) => c,
            Err(e) => {
                self.log_error(format!("Surface Sections curve B error: {e}"));
                return;
            }
        };
        let mut model = BRepModel::new();
        match sections(&mut model, &[&curve_a, &curve_b], 16) {
            Ok(r) => {
                self.add_to_scene("Surface Sections", model, r.solid, None);
                self.log_info(format!(
                    "Surface: sections — skinned {} faces between 2 profiles",
                    r.faces.len()
                ));
            }
            Err(e) => self.log_error(format!("Surface Sections error: {e}")),
        }
    }

    fn run_surface_extend(&mut self) {
        let Some(obj) = self.scene.selected_object().cloned() else {
            self.log_warning("Surface Extend: select a solid first");
            return;
        };
        self.snapshot_before("Surface Extend");
        let mut model = obj.model.clone();
        let distance = 0.5;
        match extend_surface(&mut model, obj.solid, distance) {
            Ok(r) => {
                self.add_to_scene(
                    &format!("{} (extended {distance:.2})", obj.name),
                    model,
                    r.solid,
                    None,
                );
                self.log_info(format!(
                    "Surface: extended '{}' by {distance:.2}",
                    obj.name
                ));
            }
            Err(e) => self.log_error(format!("Surface Extend error: {e}")),
        }
    }

    fn run_surface_blend(&mut self) {
        // Blend uses `surface_from_curves` as the closest available kernel
        // API. The dedicated surface-blend (G2/G3 continuity matching) is
        // only partially implemented kernel-side, so this Phase C3 wiring
        // produces a Gordon-like quad surface between two profiles instead
        // of a true tangent-continuous blend. Tracked as a Phase F+ kernel
        // task for true blend support.
        self.snapshot_before("Surface Blend");
        let curve_a_pts = self.default_polyline_or_last_sketch();
        // Companion polyline offset by Z=1 so the blend has surface area.
        let curve_b_pts: Vec<Point3> = curve_a_pts
            .iter()
            .map(|p| Point3::new(p.x, p.y, p.z + 1.0))
            .collect();
        let curve_a = match wire_to_bspline(&curve_a_pts, 1) {
            Ok(c) => c,
            Err(e) => {
                self.log_error(format!("Surface Blend curve A error: {e}"));
                return;
            }
        };
        let curve_b = match wire_to_bspline(&curve_b_pts, 1) {
            Ok(c) => c,
            Err(e) => {
                self.log_error(format!("Surface Blend curve B error: {e}"));
                return;
            }
        };
        let mut model = BRepModel::new();
        match surface_from_curves(&mut model, &[&curve_a, &curve_b], 16) {
            Ok(r) => {
                self.add_to_scene("Surface Blend", model, r.solid, None);
                self.log_info(format!(
                    "Surface: blend (Gordon-like quad sheet, {} faces — true G2 blend pending kernel work)",
                    r.faces.len()
                ));
            }
            Err(e) => self.log_error(format!("Surface Blend error: {e}")),
        }
    }

    fn run_partdesign_additive_loft(&mut self) {
        self.snapshot_before("PartDesign Loft");
        let bottom = vec![
            Point3::new(-1.0, -1.0, 0.0),
            Point3::new(1.0, -1.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(-1.0, 1.0, 0.0),
        ];
        let top = vec![
            Point3::new(-0.5, -0.5, 2.0),
            Point3::new(0.5, -0.5, 2.0),
            Point3::new(0.5, 0.5, 2.0),
            Point3::new(-0.5, 0.5, 2.0),
        ];
        let mut model = BRepModel::new();
        match loft(&mut model, &[&bottom, &top]) {
            Ok(r) => {
                self.add_to_scene("Loft", model, r.solid, None);
                self.log_info(format!(
                    "PartDesign: additive loft ({} faces between 2 profiles)",
                    r.faces.len()
                ));
            }
            Err(e) => self.log_error(format!("PartDesign Loft error: {e}")),
        }
    }

    fn run_partdesign_additive_pipe(&mut self) {
        self.snapshot_before("PartDesign Pipe");
        // Default profile (square cross-section) and 2-point Z path. The
        // sweep places the profile perpendicular to the path tangent.
        let profile = vec![
            Point3::new(-0.25, -0.25, 0.0),
            Point3::new(0.25, -0.25, 0.0),
            Point3::new(0.25, 0.25, 0.0),
            Point3::new(-0.25, 0.25, 0.0),
        ];
        let path = vec![Point3::ORIGIN, Point3::new(0.0, 0.0, 2.0)];
        let mut model = BRepModel::new();
        match sweep(&mut model, &profile, &path) {
            Ok(r) => {
                self.add_to_scene("Pipe", model, r.solid, None);
                self.log_info(format!(
                    "PartDesign: additive pipe ({} faces, 2-point path)",
                    r.faces.len()
                ));
            }
            Err(e) => self.log_error(format!("PartDesign Pipe error: {e}")),
        }
    }

    fn run_partdesign_subtractive_loft(&mut self) {
        let Some(obj) = self.scene.selected_object().cloned() else {
            self.log_warning("Subtractive Loft: select a base solid first");
            return;
        };
        self.snapshot_before("PartDesign Subtractive Loft");
        // Build the loft tool solid in its own model first.
        let bottom = vec![
            Point3::new(-0.5, -0.5, 0.0),
            Point3::new(0.5, -0.5, 0.0),
            Point3::new(0.5, 0.5, 0.0),
            Point3::new(-0.5, 0.5, 0.0),
        ];
        let top = vec![
            Point3::new(-0.5, -0.5, 2.0),
            Point3::new(0.5, -0.5, 2.0),
            Point3::new(0.5, 0.5, 2.0),
            Point3::new(-0.5, 0.5, 2.0),
        ];
        let mut tool_model = BRepModel::new();
        let tool = match loft(&mut tool_model, &[&bottom, &top]) {
            Ok(r) => r,
            Err(e) => {
                self.log_error(format!("Subtractive Loft tool error: {e}"));
                return;
            }
        };
        match boolean_op_exact(
            &obj.model,
            obj.solid,
            &tool_model,
            tool.solid,
            BooleanOp::Difference,
            1e-6,
        ) {
            Ok(result_model) => {
                let first = result_model.solids.iter().next().map(|(h, _)| h);
                if let Some(solid) = first {
                    self.add_to_scene(
                        &format!("{} \\ Loft", obj.name),
                        result_model,
                        solid,
                        Some(crate::scene::CreationParams::Boolean { op: "subtractive_loft".into() }),
                    );
                    self.log_info("PartDesign: subtractive loft (Difference)");
                } else {
                    self.log_error("Subtractive Loft: result has no solid");
                }
            }
            Err(e) => self.log_error(format!("Subtractive Loft boolean error: {e}")),
        }
    }

    fn run_partdesign_subtractive_pipe(&mut self) {
        let Some(obj) = self.scene.selected_object().cloned() else {
            self.log_warning("Subtractive Pipe: select a base solid first");
            return;
        };
        self.snapshot_before("PartDesign Subtractive Pipe");
        let profile = vec![
            Point3::new(-0.25, -0.25, 0.0),
            Point3::new(0.25, -0.25, 0.0),
            Point3::new(0.25, 0.25, 0.0),
            Point3::new(-0.25, 0.25, 0.0),
        ];
        let path = vec![Point3::ORIGIN, Point3::new(0.0, 0.0, 2.0)];
        let mut tool_model = BRepModel::new();
        let tool = match sweep(&mut tool_model, &profile, &path) {
            Ok(r) => r,
            Err(e) => {
                self.log_error(format!("Subtractive Pipe tool error: {e}"));
                return;
            }
        };
        match boolean_op_exact(
            &obj.model,
            obj.solid,
            &tool_model,
            tool.solid,
            BooleanOp::Difference,
            1e-6,
        ) {
            Ok(result_model) => {
                let first = result_model.solids.iter().next().map(|(h, _)| h);
                if let Some(solid) = first {
                    self.add_to_scene(
                        &format!("{} \\ Pipe", obj.name),
                        result_model,
                        solid,
                        Some(crate::scene::CreationParams::Boolean { op: "subtractive_pipe".into() }),
                    );
                    self.log_info("PartDesign: subtractive pipe (Difference)");
                } else {
                    self.log_error("Subtractive Pipe: result has no solid");
                }
            }
            Err(e) => self.log_error(format!("Subtractive Pipe boolean error: {e}")),
        }
    }

    fn process_techdraw_action(&mut self, action: TechDrawAction) {
        use TechDrawAction as T;
        match action {
            T::AddView(dir) => {
                if let Some(solid) = self.current_solid {
                    let view = cadkernel_io::project_solid(&self.model, solid, dir);
                    let n_edges = view.edges.len();
                    let sheet = self
                        .gui
                        .techdraw_sheet
                        .get_or_insert_with(cadkernel_io::DrawingSheet::a4_landscape);
                    sheet.views.push(view);
                    self.gui.status_message =
                        format!("TechDraw: added {} view ({n_edges} edges)", dir.label());
                } else {
                    self.gui.status_message = "TechDraw: no solid to project".into();
                }
            }
            T::ThreeView => {
                if let Some(solid) = self.current_solid {
                    let sheet = cadkernel_io::three_view_drawing(&self.model, solid);
                    let total: usize = sheet.views.iter().map(|v| v.edges.len()).sum();
                    self.gui.techdraw_sheet = Some(sheet);
                    self.gui.status_message = format!("TechDraw: 3-view drawing ({total} edges)");
                } else {
                    self.gui.status_message = "TechDraw: no solid to project".into();
                }
            }
            T::ExportSvg(path) => {
                if let Some(sheet) = &self.gui.techdraw_sheet {
                    let svg = cadkernel_io::drawing_to_svg(sheet);
                    match std::fs::write(&path, svg.render()) {
                        Ok(()) => {
                            self.gui.status_message =
                                format!("Exported SVG: {}", path.display());
                        }
                        Err(e) => {
                            self.gui.status_message = format!("SVG export failed: {e}");
                        }
                    }
                } else {
                    self.gui.status_message = "TechDraw: no drawing to export".into();
                }
            }
            T::Clear => {
                self.gui.techdraw_sheet = None;
                self.gui.status_message = "TechDraw: cleared".into();
            }
            T::NewPage => self.log_info("TechDraw: new page"),
            T::FromTemplate => self.log_info("TechDraw: from template"),
            T::Redraw => self.log_info("TechDraw: redraw"),
            T::SectionView => self.log_info("TechDraw: section view"),
            T::DetailView => self.log_info("TechDraw: detail view"),
            T::BrokenView => self.log_info("TechDraw: broken view"),
            T::DimLinear => self.log_info("TechDraw: dim linear"),
            T::DimRadius => self.log_info("TechDraw: dim radius"),
            T::DimDiameter => self.log_info("TechDraw: dim diameter"),
            T::DimAngle => self.log_info("TechDraw: dim angle"),
            T::DimArcLen => self.log_info("TechDraw: dim arc length"),
            T::DimArea => self.log_info("TechDraw: dim area"),
            T::Text => self.log_info("TechDraw: text"),
            T::RichText => self.log_info("TechDraw: rich text"),
            T::Balloon => self.log_info("TechDraw: balloon"),
            T::Leader => self.log_info("TechDraw: leader"),
            T::Weld => self.log_info("TechDraw: weld symbol"),
            T::SurfFinish => self.log_info("TechDraw: surface finish"),
            T::CenterFace => self.log_info("TechDraw: center face"),
            T::CenterLines => self.log_info("TechDraw: center lines"),
            T::CenterPoints => self.log_info("TechDraw: center points"),
            T::BoltCircle => self.log_info("TechDraw: bolt circle"),
            T::ExportDxf(path) => {
                self.log_info(format!("TechDraw: export DXF \u{2192} {}", path.display()));
            }
            T::ExportPdf(path) => {
                self.log_info(format!("TechDraw: export PDF \u{2192} {}", path.display()));
            }
        }
    }

    /// Compute a `WorkPlane` from the first selected face.
    fn compute_face_workplane(&self) -> Option<WorkPlane> {
        let face_h = self.gui.selected_entities.iter().find_map(|e| {
            if let SelectedEntity::Face(fh) = e { Some(*fh) } else { None }
        })?;
        let obj = self.scene.selected_object()?;
        let (_fh, start, count) = obj.face_tri_map.iter().find(|(f, _, _)| *f == face_h)?;
        let base = start * 3;
        let end = base + count * 3;
        if end > obj.vertices.len() || *count == 0 {
            return None;
        }

        // Centroid of all triangle vertices in the face
        let mut cx: f64 = 0.0;
        let mut cy: f64 = 0.0;
        let mut cz: f64 = 0.0;
        let n = (end - base) as f64;
        for v in &obj.vertices[base..end] {
            cx += v.position[0] as f64;
            cy += v.position[1] as f64;
            cz += v.position[2] as f64;
        }
        cx /= n;
        cy /= n;
        cz /= n;

        // Normal from first triangle
        let p0 = &obj.vertices[base].position;
        let p1 = &obj.vertices[base + 1].position;
        let p2 = &obj.vertices[base + 2].position;
        let e1 = [
            (p1[0] - p0[0]) as f64,
            (p1[1] - p0[1]) as f64,
            (p1[2] - p0[2]) as f64,
        ];
        let e2 = [
            (p2[0] - p0[0]) as f64,
            (p2[1] - p0[1]) as f64,
            (p2[2] - p0[2]) as f64,
        ];
        let nx = e1[1] * e2[2] - e1[2] * e2[1];
        let ny = e1[2] * e2[0] - e1[0] * e2[2];
        let nz = e1[0] * e2[1] - e1[1] * e2[0];
        let len = (nx * nx + ny * ny + nz * nz).sqrt();
        if len < 1e-12 {
            return None;
        }
        let normal = Vec3::new(nx / len, ny / len, nz / len);

        // X-axis: perpendicular to normal
        let up_candidate = if normal.y.abs() < 0.9 {
            Vec3::new(0.0, 1.0, 0.0)
        } else {
            Vec3::new(1.0, 0.0, 0.0)
        };
        let right = Vec3::new(
            normal.y * up_candidate.z - normal.z * up_candidate.y,
            normal.z * up_candidate.x - normal.x * up_candidate.z,
            normal.x * up_candidate.y - normal.y * up_candidate.x,
        );
        let rlen = (right.x * right.x + right.y * right.y + right.z * right.z).sqrt();
        let x_axis = Vec3::new(right.x / rlen, right.y / rlen, right.z / rlen);

        Some(WorkPlane::new(Point3::new(cx, cy, cz), normal, x_axis))
    }

    /// Compute edge loop from the first selected edge.
    ///
    /// Walks along edges sharing a vertex where exactly 2 edges meet (valence-2
    /// chain). Collects edges in both directions from the seed edge.
    fn compute_edge_loop(&self) -> Vec<Handle<EdgeData>> {
        let seed = self.gui.selected_entities.iter().find_map(|e| {
            if let SelectedEntity::Edge(eh) = e { Some(*eh) } else { None }
        });
        let seed = match seed {
            Some(s) => s,
            None => return Vec::new(),
        };

        // Build vertex→edges adjacency from model
        let mut vert_edges: std::collections::HashMap<Handle<VertexData>, Vec<Handle<EdgeData>>> =
            std::collections::HashMap::new();
        for (eh, ed) in self.model.edges.iter() {
            vert_edges.entry(ed.start).or_default().push(eh);
            vert_edges.entry(ed.end).or_default().push(eh);
        }

        // Walk in one direction from seed
        let walk = |start_vertex: Handle<VertexData>, exclude: Handle<EdgeData>| -> Vec<Handle<EdgeData>> {
            let mut result = Vec::new();
            let mut current = start_vertex;
            let mut prev_edge = exclude;
            for _ in 0..1000 {
                let neighbors = match vert_edges.get(&current) {
                    Some(n) => n,
                    None => break,
                };
                // Find the other edge at this vertex (not prev_edge)
                let next: Vec<_> = neighbors.iter().filter(|&&e| e != prev_edge).copied().collect();
                if next.len() != 1 {
                    break; // branch or dead end — stop
                }
                let next_edge = next[0];
                if next_edge == exclude || result.contains(&next_edge) {
                    // Closed loop
                    break;
                }
                result.push(next_edge);
                let ed = match self.model.edges.get(next_edge) {
                    Some(e) => e,
                    None => break,
                };
                current = if ed.start == current { ed.end } else { ed.start };
                prev_edge = next_edge;
            }
            result
        };

        let ed = match self.model.edges.get(seed) {
            Some(e) => e,
            None => return Vec::new(),
        };
        let mut result = vec![seed];
        let fwd = walk(ed.end, seed);
        let bwd = walk(ed.start, seed);
        // Prepend backward (reversed) + seed + forward
        let mut final_loop: Vec<Handle<EdgeData>> = bwd.into_iter().rev().collect();
        final_loop.append(&mut result);
        final_loop.extend(fwd);
        final_loop
    }

    /// Compute edge ring from the first selected edge.
    ///
    /// An edge ring follows "opposite" edges across quad faces. From the seed
    /// edge, for each adjacent face that is a quad (4 edges), pick the edge
    /// opposite to the current one and continue.
    fn compute_edge_ring(&self) -> Vec<Handle<EdgeData>> {
        let seed = self.gui.selected_entities.iter().find_map(|e| {
            if let SelectedEntity::Edge(eh) = e { Some(*eh) } else { None }
        });
        let seed = match seed {
            Some(s) => s,
            None => return Vec::new(),
        };

        // Get opposite edge in a quad face
        let opposite_edge = |face_h: Handle<FaceData>, edge_h: Handle<EdgeData>| -> Option<Handle<EdgeData>> {
            let edges = self.model.edges_of_face(face_h).ok()?;
            if edges.len() != 4 {
                return None; // only works on quads
            }
            let idx = edges.iter().position(|&e| e == edge_h)?;
            Some(edges[(idx + 2) % 4])
        };

        let mut result = vec![seed];
        let mut visited = std::collections::HashSet::new();
        visited.insert(seed);

        // Walk in both directions
        for pass in 0..2 {
            let mut current = seed;
            for _ in 0..1000 {
                let adj_faces = match self.model.faces_of_edge(current) {
                    Ok(f) => f,
                    Err(_) => break,
                };
                // Pick the face we haven't used yet (prefer alternating)
                let face = if pass == 0 {
                    adj_faces.first().copied()
                } else {
                    adj_faces.get(1).or_else(|| adj_faces.first()).copied()
                };
                let face_h = match face {
                    Some(f) => f,
                    None => break,
                };
                let opp = match opposite_edge(face_h, current) {
                    Some(e) => e,
                    None => break,
                };
                if visited.contains(&opp) {
                    break;
                }
                visited.insert(opp);
                result.push(opp);
                current = opp;
            }
        }
        result
    }

    /// Compute face loop from the first selected face.
    ///
    /// BFS outward from the seed face through shared edges, collecting all
    /// transitively connected faces.
    fn compute_face_loop(&self) -> Vec<Handle<FaceData>> {
        let seed = self.gui.selected_entities.iter().find_map(|e| {
            if let SelectedEntity::Face(fh) = e { Some(*fh) } else { None }
        });
        let seed = match seed {
            Some(s) => s,
            None => return Vec::new(),
        };

        // BFS from seed — collect adjacent faces sharing an edge
        let mut result = vec![seed];
        let mut visited = std::collections::HashSet::new();
        visited.insert(seed);
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(seed);

        while let Some(face_h) = queue.pop_front() {
            let edges = match self.model.edges_of_face(face_h) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for edge_h in edges {
                let adj_faces = match self.model.faces_of_edge(edge_h) {
                    Ok(f) => f,
                    Err(_) => continue,
                };
                for adj_f in adj_faces {
                    if !visited.contains(&adj_f) {
                        visited.insert(adj_f);
                        result.push(adj_f);
                        queue.push_back(adj_f);
                    }
                }
            }
        }
        result
    }

    /// Delete selected sketch entities. Removes entities by index (highest first to avoid shift).
    /// Does NOT cascade-delete referenced points — only deletes the selected entity types.
    pub(crate) fn delete_sketch_entities(sm: &mut SketchMode) {
        use crate::gui::SketchEntityRef::*;
        // Collect indices to delete per type, sorted descending
        let mut del_points: Vec<usize> = Vec::new();
        let mut del_lines: Vec<usize> = Vec::new();
        let mut del_arcs: Vec<usize> = Vec::new();
        let mut del_circles: Vec<usize> = Vec::new();
        let mut del_ellipses: Vec<usize> = Vec::new();
        let mut del_bsplines: Vec<usize> = Vec::new();

        for e in &sm.selected_entities {
            match *e {
                Point(i) => del_points.push(i),
                Line(i) => del_lines.push(i),
                Arc(i) => del_arcs.push(i),
                Circle(i) => del_circles.push(i),
                Ellipse(i) => del_ellipses.push(i),
                BSpline(i) => del_bsplines.push(i),
            }
        }
        // Sort descending to remove from end first
        del_points.sort_unstable_by(|a, b| b.cmp(a));
        del_lines.sort_unstable_by(|a, b| b.cmp(a));
        del_arcs.sort_unstable_by(|a, b| b.cmp(a));
        del_circles.sort_unstable_by(|a, b| b.cmp(a));
        del_ellipses.sort_unstable_by(|a, b| b.cmp(a));
        del_bsplines.sort_unstable_by(|a, b| b.cmp(a));

        for i in del_bsplines {
            if i < sm.sketch.bsplines.len() {
                sm.sketch.bsplines.remove(i);
            }
        }
        for i in del_ellipses {
            if i < sm.sketch.ellipses.len() {
                sm.sketch.ellipses.remove(i);
            }
        }
        for i in del_circles {
            if i < sm.sketch.circles.len() {
                sm.sketch.circles.remove(i);
            }
        }
        for i in del_arcs {
            if i < sm.sketch.arcs.len() {
                sm.sketch.arcs.remove(i);
            }
        }
        for i in del_lines {
            if i < sm.sketch.lines.len() {
                sm.sketch.lines.remove(i);
            }
        }
        for i in del_points {
            if i < sm.sketch.points.len() {
                sm.sketch.points.remove(i);
                // Adjust all PointId references that point beyond the removed index
                use cadkernel_sketch::PointId;
                for line in &mut sm.sketch.lines {
                    if line.start.0 > i { line.start = PointId(line.start.0 - 1); }
                    if line.end.0 > i { line.end = PointId(line.end.0 - 1); }
                }
                for arc in &mut sm.sketch.arcs {
                    if arc.center.0 > i { arc.center = PointId(arc.center.0 - 1); }
                    if arc.start_point.0 > i { arc.start_point = PointId(arc.start_point.0 - 1); }
                    if arc.end_point.0 > i { arc.end_point = PointId(arc.end_point.0 - 1); }
                }
                for circle in &mut sm.sketch.circles {
                    if circle.center.0 > i { circle.center = PointId(circle.center.0 - 1); }
                }
                for ell in &mut sm.sketch.ellipses {
                    if ell.center.0 > i { ell.center = PointId(ell.center.0 - 1); }
                    if ell.major_end.0 > i { ell.major_end = PointId(ell.major_end.0 - 1); }
                }
                for bsp in &mut sm.sketch.bsplines {
                    for cp in &mut bsp.control_points {
                        if cp.0 > i { *cp = PointId(cp.0 - 1); }
                    }
                }
            }
        }

        sm.selected_entities.clear();
    }

    /// Apply Horizontal constraint to selected line(s), or last line if no selection.
    fn apply_sketch_constraint_h(sm: &mut SketchMode, status: &mut String) {
        use cadkernel_sketch::{Constraint, LineId};
        let mut applied = 0;
        for e in &sm.selected_entities {
            if let SketchEntityRef::Line(i) = *e {
                if i < sm.sketch.lines.len() {
                    sm.sketch.add_constraint(Constraint::Horizontal(LineId(i)));
                    applied += 1;
                }
            }
        }
        if applied == 0 && !sm.sketch.lines.is_empty() {
            let lid = LineId(sm.sketch.lines.len() - 1);
            sm.sketch.add_constraint(Constraint::Horizontal(lid));
            applied = 1;
        }
        if applied > 0 {
            *status = format!("Horizontal constraint applied to {applied} line(s)");
        } else {
            *status = "No lines to constrain".into();
        }
    }

    /// Apply Vertical constraint to selected line(s), or last line if no selection.
    fn apply_sketch_constraint_v(sm: &mut SketchMode, status: &mut String) {
        use cadkernel_sketch::{Constraint, LineId};
        let mut applied = 0;
        for e in &sm.selected_entities {
            if let SketchEntityRef::Line(i) = *e {
                if i < sm.sketch.lines.len() {
                    sm.sketch.add_constraint(Constraint::Vertical(LineId(i)));
                    applied += 1;
                }
            }
        }
        if applied == 0 && !sm.sketch.lines.is_empty() {
            let lid = LineId(sm.sketch.lines.len() - 1);
            sm.sketch.add_constraint(Constraint::Vertical(lid));
            applied = 1;
        }
        if applied > 0 {
            *status = format!("Vertical constraint applied to {applied} line(s)");
        } else {
            *status = "No lines to constrain".into();
        }
    }

    /// Hit-test sketch entities at (x, y) and return the closest match.
    fn hit_test_sketch(sm: &SketchMode, x: f64, y: f64) -> Option<SketchEntityRef> {
        let ht = 0.4;
        let mut best: Option<(SketchEntityRef, f64)> = None;

        for (i, pt) in sm.sketch.points.iter().enumerate() {
            let d = ((pt.position.x - x).powi(2) + (pt.position.y - y).powi(2)).sqrt();
            if d < ht * 0.6 && best.as_ref().is_none_or(|(_, bd)| d < *bd) {
                best = Some((SketchEntityRef::Point(i), d));
            }
        }
        if best.is_some() {
            return best.map(|(e, _)| e);
        }
        for (i, line) in sm.sketch.lines.iter().enumerate() {
            if line.start.0 < sm.sketch.points.len() && line.end.0 < sm.sketch.points.len() {
                let s = &sm.sketch.points[line.start.0];
                let e = &sm.sketch.points[line.end.0];
                let d = Self::point_to_segment_dist(x, y, s.position.x, s.position.y, e.position.x, e.position.y);
                if d < ht && best.as_ref().is_none_or(|(_, bd)| d < *bd) {
                    best = Some((SketchEntityRef::Line(i), d));
                }
            }
        }
        if best.is_some() {
            return best.map(|(e, _)| e);
        }
        for (i, circle) in sm.sketch.circles.iter().enumerate() {
            if circle.center.0 < sm.sketch.points.len() {
                let c = &sm.sketch.points[circle.center.0];
                let d = (((x - c.position.x).powi(2) + (y - c.position.y).powi(2)).sqrt() - circle.radius).abs();
                if d < ht && best.as_ref().is_none_or(|(_, bd)| d < *bd) {
                    best = Some((SketchEntityRef::Circle(i), d));
                }
            }
        }
        if best.is_some() {
            return best.map(|(e, _)| e);
        }
        for (i, arc) in sm.sketch.arcs.iter().enumerate() {
            if arc.center.0 < sm.sketch.points.len() {
                let c = &sm.sketch.points[arc.center.0];
                let d = (((x - c.position.x).powi(2) + (y - c.position.y).powi(2)).sqrt() - arc.radius).abs();
                if d < ht && best.as_ref().is_none_or(|(_, bd)| d < *bd) {
                    best = Some((SketchEntityRef::Arc(i), d));
                }
            }
        }
        best.map(|(e, _)| e)
    }

    /// Collect point indices for entity dragging (line→2 endpoints, circle/arc→center).
    fn entity_drag_points(sm: &SketchMode, x: f64, y: f64, threshold: f64) -> Vec<usize> {
        let mut best: Option<(Vec<usize>, f64)> = None;
        for line in &sm.sketch.lines {
            if line.start.0 < sm.sketch.points.len() && line.end.0 < sm.sketch.points.len() {
                let s = &sm.sketch.points[line.start.0];
                let e = &sm.sketch.points[line.end.0];
                let d = Self::point_to_segment_dist(
                    x, y, s.position.x, s.position.y, e.position.x, e.position.y,
                );
                if d < threshold && best.as_ref().is_none_or(|(_, bd)| d < *bd) {
                    best = Some((vec![line.start.0, line.end.0], d));
                }
            }
        }
        for circle in &sm.sketch.circles {
            if circle.center.0 < sm.sketch.points.len() {
                let c = &sm.sketch.points[circle.center.0];
                let d = (((x - c.position.x).powi(2) + (y - c.position.y).powi(2)).sqrt()
                    - circle.radius)
                    .abs();
                if d < threshold && best.as_ref().is_none_or(|(_, bd)| d < *bd) {
                    best = Some((vec![circle.center.0], d));
                }
            }
        }
        for arc in &sm.sketch.arcs {
            if arc.center.0 < sm.sketch.points.len() {
                let c = &sm.sketch.points[arc.center.0];
                let d = (((x - c.position.x).powi(2) + (y - c.position.y).powi(2)).sqrt()
                    - arc.radius)
                    .abs();
                if d < threshold && best.as_ref().is_none_or(|(_, bd)| d < *bd) {
                    best = Some((vec![arc.center.0], d));
                }
            }
        }
        for ell in &sm.sketch.ellipses {
            if ell.center.0 < sm.sketch.points.len() && ell.major_end.0 < sm.sketch.points.len() {
                let c = &sm.sketch.points[ell.center.0];
                let dx = x - c.position.x;
                let dy = y - c.position.y;
                let d = (dx * dx + dy * dy).sqrt();
                if d < threshold * 3.0 && best.as_ref().is_none_or(|(_, bd)| d < *bd) {
                    best = Some((vec![ell.center.0, ell.major_end.0], d));
                }
            }
        }
        best.map(|(pts, _)| pts).unwrap_or_default()
    }

    /// Distance from point (px,py) to line segment (ax,ay)-(bx,by).
    fn point_to_segment_dist(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
        let dx = bx - ax;
        let dy = by - ay;
        let len_sq = dx * dx + dy * dy;
        if len_sq < 1e-12 {
            return ((px - ax).powi(2) + (py - ay).powi(2)).sqrt();
        }
        let t = ((px - ax) * dx + (py - ay) * dy) / len_sq;
        let t = t.clamp(0.0, 1.0);
        let cx = ax + t * dx;
        let cy = ay + t * dy;
        ((px - cx).powi(2) + (py - cy).powi(2)).sqrt()
    }

    /// Apply snapping to sketch coordinates: grid snap then point snap.
    fn snap_sketch_coords(sm: &crate::gui::SketchMode, x: f64, y: f64) -> (f64, f64) {
        let mut sx = x;
        let mut sy = y;
        // Grid snap
        if sm.snap_enabled {
            let g = sm.grid_spacing.max(0.01);
            sx = (sx / g).round() * g;
            sy = (sy / g).round() * g;
        }
        let snap_dist = 0.3;
        let mut best_d = snap_dist;
        // Point snap (coincident to existing point within threshold)
        for pt in &sm.sketch.points {
            let dx = pt.position.x - sx;
            let dy = pt.position.y - sy;
            let d = (dx * dx + dy * dy).sqrt();
            if d < best_d {
                sx = pt.position.x;
                sy = pt.position.y;
                best_d = d;
            }
        }
        // Midpoint snap
        for line in &sm.sketch.lines {
            if line.start.0 < sm.sketch.points.len() && line.end.0 < sm.sketch.points.len() {
                let s = &sm.sketch.points[line.start.0];
                let e = &sm.sketch.points[line.end.0];
                let mx = (s.position.x + e.position.x) * 0.5;
                let my = (s.position.y + e.position.y) * 0.5;
                let d = ((x - mx).powi(2) + (y - my).powi(2)).sqrt();
                if d < best_d {
                    sx = mx;
                    sy = my;
                    best_d = d;
                }
            }
        }
        // Intersection snap (line-line intersections)
        let n_lines = sm.sketch.lines.len();
        for i in 0..n_lines {
            for j in (i + 1)..n_lines {
                let li = &sm.sketch.lines[i];
                let lj = &sm.sketch.lines[j];
                if li.start.0 >= sm.sketch.points.len() || li.end.0 >= sm.sketch.points.len()
                    || lj.start.0 >= sm.sketch.points.len() || lj.end.0 >= sm.sketch.points.len()
                {
                    continue;
                }
                let (ax, ay) = (sm.sketch.points[li.start.0].position.x, sm.sketch.points[li.start.0].position.y);
                let (bx, by) = (sm.sketch.points[li.end.0].position.x, sm.sketch.points[li.end.0].position.y);
                let (cx, cy) = (sm.sketch.points[lj.start.0].position.x, sm.sketch.points[lj.start.0].position.y);
                let (dx2, dy2) = (sm.sketch.points[lj.end.0].position.x, sm.sketch.points[lj.end.0].position.y);
                let denom = (bx - ax) * (dy2 - cy) - (by - ay) * (dx2 - cx);
                if denom.abs() < 1e-12 {
                    continue;
                }
                let t = ((cx - ax) * (dy2 - cy) - (cy - ay) * (dx2 - cx)) / denom;
                let u = ((cx - ax) * (by - ay) - (cy - ay) * (bx - ax)) / denom;
                if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
                    let ix = ax + t * (bx - ax);
                    let iy = ay + t * (by - ay);
                    let d = ((x - ix).powi(2) + (y - iy).powi(2)).sqrt();
                    if d < best_d {
                        sx = ix;
                        sy = iy;
                        best_d = d;
                    }
                }
            }
        }
        (sx, sy)
    }

    /// Find an existing point within snap distance, or create a new one.
    fn find_or_create_point(sketch: &mut cadkernel_sketch::Sketch, x: f64, y: f64, snap_dist: f64) -> cadkernel_sketch::PointId {
        for (i, pt) in sketch.points.iter().enumerate() {
            let dx = pt.position.x - x;
            let dy = pt.position.y - y;
            if (dx * dx + dy * dy).sqrt() < snap_dist {
                return cadkernel_sketch::PointId(i);
            }
        }
        sketch.add_point(x, y)
    }

    /// Detect and apply auto-constraints for a newly created line.
    fn apply_line_auto_constraints(sketch: &mut cadkernel_sketch::Sketch, lid: cadkernel_sketch::LineId) {
        let line = &sketch.lines[lid.0];
        let s = &sketch.points[line.start.0];
        let e = &sketch.points[line.end.0];
        let dx = e.position.x - s.position.x;
        let dy = e.position.y - s.position.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-9 { return; }
        let angle = dy.atan2(dx).to_degrees().abs();
        if angle < 5.0 || (180.0 - angle).abs() < 5.0 {
            sketch.add_constraint(Constraint::Horizontal(lid));
        } else if (angle - 90.0).abs() < 5.0 {
            sketch.add_constraint(Constraint::Vertical(lid));
        }
    }

    /// Handle a click on the sketch plane.
    fn handle_sketch_click(&mut self, x: f64, y: f64) {
        let sm = match &mut self.gui.sketch_mode {
            Some(sm) => sm,
            None => return,
        };

        // Apply snapping
        let (x, y) = Self::snap_sketch_coords(sm, x, y);

        match sm.tool {
            SketchTool::Select => {
                // Hit-test sketch entities: point > line > circle > arc > ellipse > bspline
                let hit_threshold = 0.4;
                let mut best: Option<(SketchEntityRef, f64)> = None;

                // Points (highest priority, tight threshold)
                for (i, pt) in sm.sketch.points.iter().enumerate() {
                    let dx = pt.position.x - x;
                    let dy = pt.position.y - y;
                    let d = (dx * dx + dy * dy).sqrt();
                    if d < hit_threshold * 0.6
                        && best.as_ref().is_none_or(|(_, bd)| d < *bd)
                    {
                        best = Some((SketchEntityRef::Point(i), d));
                    }
                }

                // Lines
                if best.is_none() {
                    for (i, line) in sm.sketch.lines.iter().enumerate() {
                        if line.start.0 < sm.sketch.points.len()
                            && line.end.0 < sm.sketch.points.len()
                        {
                            let s = &sm.sketch.points[line.start.0];
                            let e = &sm.sketch.points[line.end.0];
                            let d = Self::point_to_segment_dist(
                                x, y,
                                s.position.x, s.position.y,
                                e.position.x, e.position.y,
                            );
                            if d < hit_threshold
                                && best.as_ref().is_none_or(|(_, bd)| d < *bd)
                            {
                                best = Some((SketchEntityRef::Line(i), d));
                            }
                        }
                    }
                }

                // Circles
                if best.is_none() {
                    for (i, circle) in sm.sketch.circles.iter().enumerate() {
                        if circle.center.0 < sm.sketch.points.len() {
                            let c = &sm.sketch.points[circle.center.0];
                            let dx = x - c.position.x;
                            let dy = y - c.position.y;
                            let dist_to_center = (dx * dx + dy * dy).sqrt();
                            let d = (dist_to_center - circle.radius).abs();
                            if d < hit_threshold
                                && best.as_ref().is_none_or(|(_, bd)| d < *bd)
                            {
                                best = Some((SketchEntityRef::Circle(i), d));
                            }
                        }
                    }
                }

                // Arcs
                if best.is_none() {
                    for (i, arc) in sm.sketch.arcs.iter().enumerate() {
                        if arc.center.0 < sm.sketch.points.len() {
                            let c = &sm.sketch.points[arc.center.0];
                            let dx = x - c.position.x;
                            let dy = y - c.position.y;
                            let dist_to_center = (dx * dx + dy * dy).sqrt();
                            let d = (dist_to_center - arc.radius).abs();
                            if d < hit_threshold
                                && best.as_ref().is_none_or(|(_, bd)| d < *bd)
                            {
                                best = Some((SketchEntityRef::Arc(i), d));
                            }
                        }
                    }
                }

                // Ellipses
                if best.is_none() {
                    for (i, ell) in sm.sketch.ellipses.iter().enumerate() {
                        if ell.center.0 < sm.sketch.points.len()
                            && ell.major_end.0 < sm.sketch.points.len()
                        {
                            let c = &sm.sketch.points[ell.center.0];
                            let m = &sm.sketch.points[ell.major_end.0];
                            let edx = m.position.x - c.position.x;
                            let edy = m.position.y - c.position.y;
                            let semi_a = (edx * edx + edy * edy).sqrt().max(0.01);
                            let semi_b = ell.minor_radius.max(0.01);
                            let angle = edy.atan2(edx);
                            let cos_a = angle.cos();
                            let sin_a = angle.sin();
                            let lx = x - c.position.x;
                            let ly = y - c.position.y;
                            let ex = lx * cos_a + ly * sin_a;
                            let ey = -lx * sin_a + ly * cos_a;
                            let norm_dist =
                                ((ex / semi_a).powi(2) + (ey / semi_b).powi(2)).sqrt();
                            let d = (norm_dist - 1.0).abs() * semi_a.min(semi_b);
                            if d < hit_threshold
                                && best.as_ref().is_none_or(|(_, bd)| d < *bd)
                            {
                                best = Some((SketchEntityRef::Ellipse(i), d));
                            }
                        }
                    }
                }

                // B-splines (hit-test control polygon)
                if best.is_none() {
                    for (i, bsp) in sm.sketch.bsplines.iter().enumerate() {
                        for seg in bsp.control_points.windows(2) {
                            if seg[0].0 < sm.sketch.points.len()
                                && seg[1].0 < sm.sketch.points.len()
                            {
                                let s = &sm.sketch.points[seg[0].0];
                                let e = &sm.sketch.points[seg[1].0];
                                let d = Self::point_to_segment_dist(
                                    x, y,
                                    s.position.x, s.position.y,
                                    e.position.x, e.position.y,
                                );
                                if d < hit_threshold
                                    && best.as_ref().is_none_or(|(_, bd)| d < *bd)
                                {
                                    best = Some((SketchEntityRef::BSpline(i), d));
                                }
                            }
                        }
                    }
                }

                let ctrl = self.mouse.ctrl_held;
                if let Some((entity, _)) = best {
                    if ctrl {
                        // Toggle selection
                        if let Some(pos) = sm.selected_entities.iter().position(|e| *e == entity) {
                            sm.selected_entities.remove(pos);
                        } else {
                            sm.selected_entities.push(entity);
                        }
                    } else {
                        sm.selected_entities.clear();
                        sm.selected_entities.push(entity);
                    }
                    let label = match entity {
                        SketchEntityRef::Point(i) => format!("Point {i}"),
                        SketchEntityRef::Line(i) => format!("Line {i}"),
                        SketchEntityRef::Arc(i) => format!("Arc {i}"),
                        SketchEntityRef::Circle(i) => format!("Circle {i}"),
                        SketchEntityRef::Ellipse(i) => format!("Ellipse {i}"),
                        SketchEntityRef::BSpline(i) => format!("BSpline {i}"),
                    };
                    self.gui.status_message = format!("Selected: {label}");
                } else if !ctrl {
                    sm.selected_entities.clear();
                    self.gui.status_message = "Selection cleared".into();
                }
            }
            SketchTool::Line => {
                if let Some((px, py)) = sm.pending_point.take() {
                    sm.save_snapshot();
                    let snap = 0.3;
                    let p0 = Self::find_or_create_point(&mut sm.sketch, px, py, snap);
                    let p1 = Self::find_or_create_point(&mut sm.sketch, x, y, snap);
                    let lid = sm.sketch.add_line(p0, p1);
                    if sm.construction_mode {
                        sm.sketch.mark_construction_line(lid);
                        sm.sketch.mark_construction_point(p0);
                        sm.sketch.mark_construction_point(p1);
                    }
                    Self::apply_line_auto_constraints(&mut sm.sketch, lid);
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
                    sm.save_snapshot();
                    let snap = 0.3;
                    let p0 = Self::find_or_create_point(&mut sm.sketch, px, py, snap);
                    let p1 = Self::find_or_create_point(&mut sm.sketch, x, py, snap);
                    let p2 = Self::find_or_create_point(&mut sm.sketch, x, y, snap);
                    let p3 = Self::find_or_create_point(&mut sm.sketch, px, y, snap);
                    let l0 = sm.sketch.add_line(p0, p1);
                    let l1 = sm.sketch.add_line(p1, p2);
                    let l2 = sm.sketch.add_line(p2, p3);
                    let l3 = sm.sketch.add_line(p3, p0);
                    if sm.construction_mode {
                        for lid in [l0, l1, l2, l3] {
                            sm.sketch.mark_construction_line(lid);
                        }
                        for pid in [p0, p1, p2, p3] {
                            sm.sketch.mark_construction_point(pid);
                        }
                    }
                    // Rectangle edges are always H/V by construction
                    sm.sketch.add_constraint(Constraint::Horizontal(l0));
                    sm.sketch.add_constraint(Constraint::Vertical(l1));
                    sm.sketch.add_constraint(Constraint::Horizontal(l2));
                    sm.sketch.add_constraint(Constraint::Vertical(l3));
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
                    sm.save_snapshot();
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
                // 2-click arc: center + point on circumference → semicircle from click angle
                if let Some((cx, cy)) = sm.pending_point.take() {
                    sm.save_snapshot();
                    let dx = x - cx;
                    let dy = y - cy;
                    let radius = (dx * dx + dy * dy).sqrt().max(0.01);
                    let click_angle = dy.atan2(dx);
                    let start_angle = click_angle - std::f64::consts::FRAC_PI_2;
                    let end_angle = click_angle + std::f64::consts::FRAC_PI_2;
                    let center = sm.sketch.add_point(cx, cy);
                    let sp = sm.sketch.add_point(
                        cx + radius * start_angle.cos(),
                        cy + radius * start_angle.sin(),
                    );
                    let ep = sm.sketch.add_point(
                        cx + radius * end_angle.cos(),
                        cy + radius * end_angle.sin(),
                    );
                    sm.sketch.add_arc(center, sp, ep, radius, start_angle, end_angle);
                    self.gui.status_message =
                        format!("Arc center ({cx:.1},{cy:.1}) r={radius:.1} angle={:.0}°", click_angle.to_degrees());
                } else {
                    sm.pending_point = Some((x, y));
                    self.gui.status_message = format!("Arc center: ({x:.1}, {y:.1}) — click circumference point");
                }
            }
            SketchTool::Point => {
                sm.save_snapshot();
                let pid = sm.sketch.add_point(x, y);
                if sm.construction_mode {
                    sm.sketch.mark_construction_point(pid);
                }
                self.gui.status_message = format!("Point at ({x:.1}, {y:.1})");
            }
            SketchTool::Ellipse => {
                if let Some((cx, cy)) = sm.pending_point.take() {
                    sm.save_snapshot();
                    let rx = (x - cx).abs().max(0.1);
                    let ry = (y - cy).abs().max(0.1);
                    let center = sm.sketch.add_point(cx, cy);
                    let major_end = sm.sketch.add_point(cx + rx, cy);
                    sm.sketch.add_ellipse(center, major_end, ry);
                    self.gui.status_message =
                        format!("Ellipse center ({cx:.1},{cy:.1}) rx={rx:.1} ry={ry:.1}");
                } else {
                    sm.pending_point = Some((x, y));
                    self.gui.status_message = format!("Ellipse center: ({x:.1}, {y:.1}) — click corner");
                }
            }
            SketchTool::Polyline => {
                sm.save_snapshot();
                sm.polyline_points.push((x, y));
                if sm.polyline_points.len() >= 2 {
                    let pts = &sm.polyline_points;
                    let n = pts.len();
                    let snap = 0.3;
                    let p0 = Self::find_or_create_point(&mut sm.sketch, pts[n - 2].0, pts[n - 2].1, snap);
                    let p1 = Self::find_or_create_point(&mut sm.sketch, pts[n - 1].0, pts[n - 1].1, snap);
                    let lid = sm.sketch.add_line(p0, p1);
                    Self::apply_line_auto_constraints(&mut sm.sketch, lid);
                }
                self.gui.status_message = format!(
                    "Polyline: {} points", sm.polyline_points.len()
                );
            }
            SketchTool::Slot => {
                // 3-click flow: center1, center2, width point
                sm.polyline_points.push((x, y));
                match sm.polyline_points.len() {
                    1 => {
                        self.gui.status_message = format!("Slot center 1: ({x:.1}, {y:.1}) — click center 2");
                    }
                    2 => {
                        self.gui.status_message = "Slot axis set — click to define width".into();
                    }
                    _ => {
                        sm.save_snapshot();
                        let (c1x, c1y) = sm.polyline_points[0];
                        let (c2x, c2y) = sm.polyline_points[1];
                        sm.polyline_points.clear();
                        let ax = c2x - c1x;
                        let ay = c2y - c1y;
                        let alen = (ax * ax + ay * ay).sqrt();
                        if alen < 1e-9 {
                            self.gui.status_message = "Slot: centers too close".into();
                        } else {
                            let nx = -ay / alen;
                            let ny = ax / alen;
                            let dx = x - c1x;
                            let dy = y - c1y;
                            let half_w = (dx * nx + dy * ny).abs().max(0.1);
                            let ux = ax / alen;
                            let uy = ay / alen;
                            let p0 = sm.sketch.add_point(c1x + nx * half_w, c1y + ny * half_w);
                            let p1 = sm.sketch.add_point(c2x + nx * half_w, c2y + ny * half_w);
                            let p2 = sm.sketch.add_point(c2x - nx * half_w, c2y - ny * half_w);
                            let p3 = sm.sketch.add_point(c1x - nx * half_w, c1y - ny * half_w);
                            sm.sketch.add_line(p0, p1);
                            sm.sketch.add_line(p2, p3);
                            let ac1 = sm.sketch.add_point(c1x, c1y);
                            let ac2 = sm.sketch.add_point(c2x, c2y);
                            let base_angle = uy.atan2(ux);
                            let a1s = base_angle + std::f64::consts::FRAC_PI_2;
                            let a1e = base_angle - std::f64::consts::FRAC_PI_2;
                            sm.sketch.add_arc(ac2, p1, p2, half_w, a1s, a1e);
                            let a2s = base_angle - std::f64::consts::FRAC_PI_2;
                            let a2e = base_angle + std::f64::consts::FRAC_PI_2;
                            sm.sketch.add_arc(ac1, p3, p0, half_w, a2s, a2e);
                            self.gui.status_message = format!("Slot: length {alen:.1}, width {:.1}", half_w * 2.0);
                        }
                    }
                }
            }
            SketchTool::BSpline => {
                sm.save_snapshot();
                sm.polyline_points.push((x, y));
                self.gui.status_message = format!(
                    "B-Spline: {} control points", sm.polyline_points.len()
                );
            }
            SketchTool::Polygon { sides } => {
                if let Some((cx, cy)) = sm.pending_point.take() {
                    sm.save_snapshot();
                    let dx = x - cx;
                    let dy = y - cy;
                    let radius = (dx * dx + dy * dy).sqrt();
                    let n = sides.max(3);
                    for i in 0..n {
                        let a0 = std::f64::consts::TAU * (i as f64) / (n as f64);
                        let a1 = std::f64::consts::TAU * ((i + 1) as f64) / (n as f64);
                        let p0 = sm.sketch.add_point(cx + radius * a0.cos(), cy + radius * a0.sin());
                        let p1 = sm.sketch.add_point(cx + radius * a1.cos(), cy + radius * a1.sin());
                        sm.sketch.add_line(p0, p1);
                    }
                    self.gui.status_message = format!(
                        "Regular {n}-gon center ({cx:.1},{cy:.1}) r={radius:.1}"
                    );
                } else {
                    sm.pending_point = Some((x, y));
                    self.gui.status_message = format!("Polygon center: ({x:.1}, {y:.1})");
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

        // Save sketch for re-editing
        self.gui.last_sketch = Some((sm.sketch.clone(), sm.plane));

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
                    construction_mode: sm.construction_mode,
                    show_grid: sm.show_grid,
                    snap_enabled: sm.snap_enabled,
                    show_constraints: sm.show_constraints,
                    polyline_points: sm.polyline_points.clone(),
                    undo_stack: sm.undo_stack,
                    redo_stack: sm.redo_stack,
                    selected_entities: Vec::new(),
                    hovered_entity: None,
                    drag_points: Vec::new(),
                    drag_origin: None,
                    drag_started: false,
                    show_context_menu: false,
                    constraint_residuals: Vec::new(),
                    solver_converged: true,
                    grid_spacing: sm.grid_spacing,
                    clipboard_points: sm.clipboard_points.clone(),
                    clipboard_lines: sm.clipboard_lines.clone(),
                    validation_issues: Vec::new(),
                    box_select_start: None,
                    box_select_end: None,
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

    fn save_scene_file(&mut self, path: &Path) {
        let path_str = path.to_str().unwrap_or("");
        let objects: Vec<cadkernel_io::SceneObjectData> = self.scene.objects.iter().map(|obj| {
            let params_json = obj.params.as_ref().and_then(|p| serde_json::to_string(p).ok());
            cadkernel_io::SceneObjectData {
                name: obj.name.clone(),
                model: obj.model.clone(),
                solid_index: obj.solid.index(),
                solid_generation: obj.solid.generation(),
                color: obj.color,
                visible: obj.visible,
                params_json,
            }
        }).collect();
        match cadkernel_io::save_scene(&objects, path_str) {
            Ok(()) => {
                self.gui.current_file = Some(path.display().to_string());
                self.add_recent_file(&path.display().to_string());
                self.log_info(format!(
                    "Saved scene ({} objects) → {}", self.scene.len(), path.display()
                ));
            }
            Err(e) => {
                self.log_error(format!("Save error: {e}"));
            }
        }
    }

    fn load_scene_file(&mut self, path: &Path) {
        let path_str = path.to_str().unwrap_or("");
        match cadkernel_io::load_scene(path_str) {
            Ok(objects) => {
                self.snapshot_before("Load scene");
                self.scene = crate::scene::Scene::new();
                self.model = BRepModel::new();
                self.current_mesh = None;
                self.current_solid = None;
                let count = objects.len();
                for obj_data in objects {
                    let solid = Handle::from_raw_parts(obj_data.solid_index, obj_data.solid_generation);
                    let params: Option<crate::scene::CreationParams> = obj_data.params_json
                        .as_deref()
                        .and_then(|s| serde_json::from_str(s).ok());
                    let id = self.scene.add_object(
                        &obj_data.name, obj_data.model, solid, params,
                    );
                    if let Some(scene_obj) = self.scene.get_mut(id) {
                        scene_obj.color = obj_data.color;
                        scene_obj.visible = obj_data.visible;
                    }
                }
                self.rebuild_scene_gpu();
                if !self.vertices.is_empty() {
                    let (min, max) = compute_bounds(&self.vertices);
                    self.camera.fit_to_bounds(min, max);
                }
                self.gui.current_file = Some(path.display().to_string());
                self.add_recent_file(&path.display().to_string());
                self.log_info(format!(
                    "Loaded scene ({count} objects) from {}", path.display()
                ));
            }
            Err(e) => {
                self.log_error(format!("Failed to load: {e}"));
            }
        }
    }

    fn load_mesh_file(&mut self, path: &Path) {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        // .cadk is handled by load_scene_file(); redirect if reached here.
        if ext == "cadk" {
            self.load_scene_file(path);
            return;
        }

        if ext != "stl" && ext != "obj" {
            self.log_error(format!("Unsupported format: .{ext}"));
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
                    let name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("imported")
                        .to_string();
                    let params = Some(crate::scene::CreationParams::Imported {
                        path: path.display().to_string(),
                    });
                    let tri = mesh.triangle_count();
                    let verts = mesh.vertices.len();
                    let id = self.scene.add_mesh_object(&name, mesh, params);
                    self.scene.select_single(id);
                    self.rebuild_scene_gpu();
                    let path_str = path.display().to_string();
                    self.gui.current_file = Some(path_str.clone());
                    self.add_recent_file(&path_str);
                    self.log_info(format!(
                        "Imported {name} ({verts} vertices, {tri} triangles)"
                    ));
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

        // .cadk is handled by save_scene_file(); redirect if reached here.
        if ext == "cadk" {
            self.save_scene_file(path);
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
        // Update background gradient when preset/colors change
        if let Some(rt) = &mut self.runtime {
            rt.gpu.update_bg(self.nav.bg_preset, self.nav.bg_custom_top, self.nav.bg_custom_bottom);
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
            preselected_object,
            command_stack,
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

        // Populate undo/redo history for UI display
        gui.tb_can_undo = command_stack.can_undo();
        gui.tb_can_redo = command_stack.can_redo();
        let (h, f) = command_stack.entries();
        gui.history_entries = h.iter().map(|s| s.to_string()).collect();
        gui.future_entries = f.iter().map(|s| s.to_string()).collect();

        // Sync bookmark info for menu display
        if gui.nav_bookmarks.len() != nav.view_bookmarks.len() {
            gui.nav_bookmarks = nav.view_bookmarks.iter().map(|b| {
                (b.name.clone(), b.yaw.to_degrees(), b.pitch.to_degrees(), b.distance)
            }).collect();
        }
        // Sync group info for menu display (always update — visibility/membership can change)
        let new_groups: Vec<(u32, String, bool, usize)> = scene.groups.iter().map(|g| {
            let count = scene.group_members(g.id).len();
            (g.id, g.name.clone(), g.visible, count)
        }).collect();
        if gui.scene_groups != new_groups {
            gui.scene_groups = new_groups;
        }

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

        // Clip plane from NavConfig
        let clip = if nav.clip_enabled {
            [nav.clip_plane_normal[0], nav.clip_plane_normal[1], nav.clip_plane_normal[2], nav.clip_plane_offset]
        } else {
            CLIP_DISABLED
        };

        // Preselection hover id for GPU highlight
        let hover_id = preselected_object.map(|id| id as f32).unwrap_or(0.0);

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
                    hover_params: [0.0; 4],
                    clip_params: clip,
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
                    hover_params: [0.0; 4],
                    clip_params: clip,
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
                    hover_params: [0.0; 4],
                    clip_params: clip,
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
                    hover_params: [0.0; 4],
                    clip_params: clip,
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
                    hover_params: [0.0; 4],
                    clip_params: clip,
                },
            );
        }

        let mesh_slot = slot;
        slot += 1;
        let wire_slot = slot;
        let hover_none = [0.0f32; 4];
        match dm {
            DisplayMode::AsIs | DisplayMode::Shading => {
                rt.gpu.write_slot(
                    mesh_slot,
                    &Uniforms {
                        view_proj: vp, light_dir: light, base_color: SOLID_COLOR,
                        params: lit_params, eye_pos, hover_params: hover_none, clip_params: clip,
                    },
                );
            }
            DisplayMode::Points => {
                rt.gpu.write_slot(
                    mesh_slot,
                    &Uniforms {
                        view_proj: vp, light_dir: no_light, base_color: POINT_COLOR,
                        params: unlit_params, eye_pos, hover_params: hover_none, clip_params: clip,
                    },
                );
            }
            DisplayMode::Wireframe => {
                rt.gpu.write_slot(
                    mesh_slot,
                    &Uniforms {
                        view_proj: vp, light_dir: no_light, base_color: WIRE_COLOR,
                        params: unlit_params, eye_pos, hover_params: hover_none, clip_params: clip,
                    },
                );
            }
            DisplayMode::HiddenLine => {
                rt.gpu.write_slot(
                    mesh_slot,
                    &Uniforms {
                        view_proj: vp, light_dir: no_light, base_color: HIDDEN_LINE_COLOR,
                        params: lit_params, eye_pos, hover_params: hover_none, clip_params: clip,
                    },
                );
                rt.gpu.write_slot(
                    wire_slot,
                    &Uniforms {
                        view_proj: vp, light_dir: no_light, base_color: EDGE_OVERLAY_COLOR,
                        params: unlit_params, eye_pos, hover_params: hover_none, clip_params: clip,
                    },
                );
            }
            DisplayMode::NoShading => {
                rt.gpu.write_slot(
                    mesh_slot,
                    &Uniforms {
                        view_proj: vp, light_dir: no_light, base_color: NO_SHADE_COLOR,
                        params: lit_params, eye_pos, hover_params: hover_none, clip_params: clip,
                    },
                );
            }
            DisplayMode::Transparent => {
                rt.gpu.write_slot(
                    mesh_slot,
                    &Uniforms {
                        view_proj: vp, light_dir: light, base_color: TRANSPARENT_COLOR,
                        params: lit_params, eye_pos, hover_params: hover_none, clip_params: clip,
                    },
                );
            }
            DisplayMode::FlatLines => {
                rt.gpu.write_slot(
                    mesh_slot,
                    &Uniforms {
                        view_proj: vp, light_dir: light, base_color: SOLID_COLOR,
                        params: lit_params, eye_pos, hover_params: hover_none, clip_params: clip,
                    },
                );
                rt.gpu.write_slot(
                    wire_slot,
                    &Uniforms {
                        view_proj: vp, light_dir: light, base_color: EDGE_OVERLAY_COLOR,
                        params: unlit_params, eye_pos, hover_params: hover_none, clip_params: clip,
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
                            r: 0.15,
                            g: 0.16,
                            b: 0.20,
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

            // Mesh — per-object rendering with frustum culling
            if rt.gpu.num_vertices > 0 {
                pass.set_vertex_buffer(0, rt.gpu.vertex_buffer.slice(..));

                // Extract frustum planes for per-object culling
                let frustum = extract_frustum_planes(&vp);

                // Determine which objects are visible (frustum test)
                let visible: Vec<bool> = object_ranges.iter().map(|&(id, _start, _count, _color, _selected)| {
                    scene.get(id).is_some_and(|obj| {
                        aabb_in_frustum(&frustum, obj.aabb_min, obj.aabb_max)
                    })
                }).collect();

                // Reuse a single per-object uniform slot to avoid hard object-count limits.
                let obj_slot = slot + 1;
                let presel = *preselected_object;

                match dm {
                    DisplayMode::AsIs | DisplayMode::Shading => {
                        pass.set_pipeline(&rt.gpu.solid_pipeline);
                        if object_ranges.is_empty() {
                            pass.set_bind_group(0, &rt.gpu.uniform_bind_group, &[GpuState::slot_offset(mesh_slot)]);
                            pass.draw(0..rt.gpu.num_vertices, 0..1);
                        } else {
                            for (i, &(id, start, count, color, selected)) in object_ranges.iter().enumerate() {
                                if !visible[i] { continue; }
                                if count == 0 { continue; }
                                let obj_color = if selected {
                                    [color[0] * 0.5 + 0.15, color[1] * 0.5 + 0.35, color[2] * 0.5 + 0.1, color[3]]
                                } else if presel == Some(id) {
                                    [color[0] * 0.6 + 0.3, color[1] * 0.6 + 0.25, color[2] * 0.4, color[3]]
                                } else {
                                    color
                                };
                                let obj_hover = if presel == Some(id) {
                                    [hover_id, id as f32, PRESELECT_STRENGTH, 0.0]
                                } else {
                                    [hover_id, id as f32, 0.0, 0.0]
                                };
                                rt.gpu.write_slot(obj_slot, &Uniforms {
                                    view_proj: vp,
                                    light_dir: light,
                                    base_color: obj_color,
                                    params: lit_params,
                                    eye_pos,
                                    hover_params: obj_hover,
                                    clip_params: clip,
                                });
                                pass.set_bind_group(0, &rt.gpu.uniform_bind_group, &[GpuState::slot_offset(obj_slot)]);
                                pass.draw(start..start + count, 0..1);
                            }
                        }
                    }
                    DisplayMode::NoShading => {
                        pass.set_pipeline(&rt.gpu.solid_pipeline);
                        if object_ranges.is_empty() {
                            pass.set_bind_group(0, &rt.gpu.uniform_bind_group, &[GpuState::slot_offset(mesh_slot)]);
                            pass.draw(0..rt.gpu.num_vertices, 0..1);
                        } else {
                            for (i, &(_id, start, count, _color, _sel)) in object_ranges.iter().enumerate() {
                                if !visible[i] { continue; }
                                if count == 0 { continue; }
                                rt.gpu.write_slot(obj_slot, &Uniforms {
                                    view_proj: vp, light_dir: no_light, base_color: NO_SHADE_COLOR,
                                    params: lit_params, eye_pos, hover_params: [0.0; 4], clip_params: clip,
                                });
                                pass.set_bind_group(0, &rt.gpu.uniform_bind_group, &[GpuState::slot_offset(obj_slot)]);
                                pass.draw(start..start + count, 0..1);
                            }
                        }
                    }
                    DisplayMode::Transparent => {
                        pass.set_pipeline(&rt.gpu.transparent_pipeline);
                        if object_ranges.is_empty() {
                            pass.set_bind_group(0, &rt.gpu.uniform_bind_group, &[GpuState::slot_offset(mesh_slot)]);
                            pass.draw(0..rt.gpu.num_vertices, 0..1);
                        } else {
                            for (i, &(_id, start, count, color, _sel)) in object_ranges.iter().enumerate() {
                                if !visible[i] { continue; }
                                if count == 0 { continue; }
                                let obj_color = [color[0], color[1], color[2], TRANSPARENT_COLOR[3]];
                                rt.gpu.write_slot(obj_slot, &Uniforms {
                                    view_proj: vp, light_dir: light, base_color: obj_color,
                                    params: lit_params, eye_pos, hover_params: [0.0; 4], clip_params: clip,
                                });
                                pass.set_bind_group(0, &rt.gpu.uniform_bind_group, &[GpuState::slot_offset(obj_slot)]);
                                pass.draw(start..start + count, 0..1);
                            }
                        }
                    }
                    DisplayMode::Points => {
                        pass.set_bind_group(0, &rt.gpu.uniform_bind_group, &[GpuState::slot_offset(mesh_slot)]);
                        pass.set_pipeline(&rt.gpu.points_pipeline);
                        pass.draw(0..rt.gpu.num_vertices, 0..1);
                    }
                    DisplayMode::Wireframe => {
                        pass.set_bind_group(0, &rt.gpu.uniform_bind_group, &[GpuState::slot_offset(mesh_slot)]);
                        pass.set_pipeline(&rt.gpu.wire_pipeline);
                        pass.set_index_buffer(rt.gpu.edge_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                        pass.draw_indexed(0..rt.gpu.num_edge_indices, 0, 0..1);
                    }
                    DisplayMode::HiddenLine => {
                        pass.set_bind_group(0, &rt.gpu.uniform_bind_group, &[GpuState::slot_offset(mesh_slot)]);
                        pass.set_pipeline(&rt.gpu.solid_pipeline);
                        pass.draw(0..rt.gpu.num_vertices, 0..1);
                        pass.set_bind_group(0, &rt.gpu.uniform_bind_group, &[GpuState::slot_offset(wire_slot)]);
                        pass.set_pipeline(&rt.gpu.wire_pipeline);
                        pass.set_index_buffer(rt.gpu.edge_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                        pass.draw_indexed(0..rt.gpu.num_edge_indices, 0, 0..1);
                    }
                    DisplayMode::FlatLines => {
                        pass.set_bind_group(0, &rt.gpu.uniform_bind_group, &[GpuState::slot_offset(mesh_slot)]);
                        pass.set_pipeline(&rt.gpu.solid_pipeline);
                        pass.draw(0..rt.gpu.num_vertices, 0..1);
                        pass.set_bind_group(0, &rt.gpu.uniform_bind_group, &[GpuState::slot_offset(wire_slot)]);
                        pass.set_pipeline(&rt.gpu.wire_pipeline);
                        pass.set_index_buffer(rt.gpu.edge_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
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
                                // In sketch Select mode: start drag if near a point or entity
                                {
                                    let pos = self.gui.pointer_physical
                                        .map(|(x, y)| (x as f64, y as f64))
                                        .or(self.mouse.last_pos);
                                    let sketch_pt = pos.and_then(|(sx, sy)| self.screen_to_sketch_plane(sx, sy));
                                    if let Some(sm) = &mut self.gui.sketch_mode {
                                        if matches!(sm.tool, SketchTool::Select) {
                                            if let Some((skx, sky)) = sketch_pt {
                                                let pt_threshold = 0.24;
                                                let entity_threshold = 0.3;
                                                // Try point first (highest priority)
                                                let mut nearest_pt: Option<(usize, f64)> = None;
                                                for (i, pt) in sm.sketch.points.iter().enumerate() {
                                                    let dx = pt.position.x - skx;
                                                    let dy = pt.position.y - sky;
                                                    let d = (dx * dx + dy * dy).sqrt();
                                                    if d < pt_threshold
                                                        && nearest_pt.as_ref().is_none_or(|(_, bd)| d < *bd)
                                                    {
                                                        nearest_pt = Some((i, d));
                                                    }
                                                }
                                                if let Some((idx, _)) = nearest_pt {
                                                    sm.save_snapshot();
                                                    sm.drag_points = vec![idx];
                                                    sm.drag_origin = Some((skx, sky));
                                                    sm.drag_started = false;
                                                } else {
                                                    // Try entity drag (line/circle/arc)
                                                    let drag_indices = Self::entity_drag_points(sm, skx, sky, entity_threshold);
                                                    if !drag_indices.is_empty() {
                                                        sm.save_snapshot();
                                                        sm.drag_points = drag_indices;
                                                        sm.drag_origin = Some((skx, sky));
                                                        sm.drag_started = false;
                                                    } else {
                                                        // No entity hit → start box selection
                                                        sm.box_select_start = Some((skx, sky));
                                                        sm.box_select_end = None;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                // LMB released
                                // End sketch drag
                                if let Some(sm) = &mut self.gui.sketch_mode {
                                    if !sm.drag_points.is_empty() {
                                        if !sm.drag_started {
                                            sm.undo_stack.pop();
                                        }
                                        sm.drag_points.clear();
                                        sm.drag_origin = None;
                                        sm.drag_started = false;
                                    }
                                    // Finalize box selection
                                    if let (Some((sx, sy)), Some((ex, ey))) =
                                        (sm.box_select_start.take(), sm.box_select_end.take())
                                    {
                                        let min_x = sx.min(ex);
                                        let max_x = sx.max(ex);
                                        let min_y = sy.min(ey);
                                        let max_y = sy.max(ey);
                                        let window_mode = ex >= sx; // left→right = window
                                        if !self.mouse.ctrl_held {
                                            sm.selected_entities.clear();
                                        }
                                        // Select points inside box
                                        for (i, pt) in sm.sketch.points.iter().enumerate() {
                                            let px = pt.position.x;
                                            let py = pt.position.y;
                                            if px >= min_x && px <= max_x && py >= min_y && py <= max_y {
                                                let e = SketchEntityRef::Point(i);
                                                if !sm.selected_entities.contains(&e) {
                                                    sm.selected_entities.push(e);
                                                }
                                            }
                                        }
                                        // Select lines (window: both endpoints inside, crossing: any endpoint inside)
                                        for (i, line) in sm.sketch.lines.iter().enumerate() {
                                            if line.start.0 >= sm.sketch.points.len() || line.end.0 >= sm.sketch.points.len() {
                                                continue;
                                            }
                                            let s = &sm.sketch.points[line.start.0];
                                            let e_pt = &sm.sketch.points[line.end.0];
                                            let s_in = s.position.x >= min_x && s.position.x <= max_x
                                                && s.position.y >= min_y && s.position.y <= max_y;
                                            let e_in = e_pt.position.x >= min_x && e_pt.position.x <= max_x
                                                && e_pt.position.y >= min_y && e_pt.position.y <= max_y;
                                            let selected = if window_mode { s_in && e_in } else { s_in || e_in };
                                            if selected {
                                                let ent = SketchEntityRef::Line(i);
                                                if !sm.selected_entities.contains(&ent) {
                                                    sm.selected_entities.push(ent);
                                                }
                                            }
                                        }
                                        // Select circles (center inside)
                                        for (i, circle) in sm.sketch.circles.iter().enumerate() {
                                            if circle.center.0 < sm.sketch.points.len() {
                                                let c = &sm.sketch.points[circle.center.0];
                                                if c.position.x >= min_x && c.position.x <= max_x
                                                    && c.position.y >= min_y && c.position.y <= max_y
                                                {
                                                    let ent = SketchEntityRef::Circle(i);
                                                    if !sm.selected_entities.contains(&ent) {
                                                        sm.selected_entities.push(ent);
                                                    }
                                                }
                                            }
                                        }
                                        // Select arcs (center inside)
                                        for (i, arc) in sm.sketch.arcs.iter().enumerate() {
                                            if arc.center.0 < sm.sketch.points.len() {
                                                let c = &sm.sketch.points[arc.center.0];
                                                if c.position.x >= min_x && c.position.x <= max_x
                                                    && c.position.y >= min_y && c.position.y <= max_y
                                                {
                                                    let ent = SketchEntityRef::Arc(i);
                                                    if !sm.selected_entities.contains(&ent) {
                                                        sm.selected_entities.push(ent);
                                                    }
                                                }
                                            }
                                        }
                                        let n = sm.selected_entities.len();
                                        self.gui.status_message = format!("Box selected {n} entities");
                                    } else {
                                        sm.box_select_start = None;
                                    }
                                }
                                // Sketch click (not drag) — dispatch on release
                                if !self.mouse_dragged && self.gui.sketch_mode.is_some() {
                                    let is_dragging = self.gui.sketch_mode.as_ref()
                                        .is_some_and(|sm| !sm.drag_points.is_empty());
                                    if !is_dragging {
                                        // Double-click detection in sketch mode
                                        let now = std::time::Instant::now();
                                        let pos = self.gui.pointer_physical
                                            .map(|(x, y)| (x as f64, y as f64))
                                            .or(self.mouse.last_pos);
                                        let dt = now.duration_since(self.last_click_time).as_millis();
                                        let dd = pos.map_or(f64::MAX, |(px, py)| {
                                            (px - self.last_click_pos.0).powi(2) + (py - self.last_click_pos.1).powi(2)
                                        });
                                        let is_sketch_dbl = dt < 300 && dd < 100.0;
                                        if let Some(p) = pos {
                                            self.last_click_time = now;
                                            self.last_click_pos = p;
                                        }

                                        if is_sketch_dbl {
                                            // Double-click in sketch: edit dimension of selected entity
                                            self.try_sketch_dimension_edit();
                                        } else if let Some((sx, sy)) = pos {
                                            if let Some(pt) = self.screen_to_sketch_plane(sx, sy) {
                                                self.gui.actions.push(GuiAction::Sketcher(SketcherAction::Click(pt.0, pt.1)));
                                            }
                                        }
                                    }
                                }
                            }
                            if !pressed && !self.mouse_dragged && self.gui.sketch_mode.is_none() {
                                // Measurement mode: click adds a 3D point
                                if self.gui.measurement_mode {
                                    if let Some(pt3) = self.pick_surface_point() {
                                        self.gui.measurement_points.push(pt3);
                                    }
                                } else {
                                    // Double-click detection (300ms, 10px proximity)
                                    let now = std::time::Instant::now();
                                    let pos = self.mouse.last_pos.unwrap_or((0.0, 0.0));
                                    let dt = now.duration_since(self.last_click_time).as_millis();
                                    let dd = (pos.0 - self.last_click_pos.0).powi(2)
                                        + (pos.1 - self.last_click_pos.1).powi(2);
                                    let is_double = dt < 300 && dd < 100.0;
                                    self.last_click_time = now;
                                    self.last_click_pos = pos;

                                    if is_double {
                                        // Double-click: select loop based on currently selected entity
                                        self.try_double_click_loop();
                                    } else {
                                        // Single click → normal auto-pick
                                        self.try_pick_entity();
                                    }
                                }
                            }
                            self.mouse.left_pressed = pressed;
                        }
                        MouseButton::Middle => {
                            self.mouse.middle_pressed = pressed;
                            if !pressed {
                                self.try_snap_to_nearest_view();
                            }
                        }
                        MouseButton::Right => {
                            if pressed {
                                if let Some(sm) = &mut self.gui.sketch_mode {
                                    if matches!(sm.tool, SketchTool::Polyline) && sm.polyline_points.len() >= 3 {
                                        // Close polyline on right-click
                                        sm.save_snapshot();
                                        let first = sm.polyline_points[0];
                                        let last = *sm.polyline_points.last().unwrap();
                                        let p0 = sm.sketch.add_point(last.0, last.1);
                                        let p1 = sm.sketch.add_point(first.0, first.1);
                                        sm.sketch.add_line(p0, p1);
                                        let n = sm.polyline_points.len();
                                        sm.polyline_points.clear();
                                        self.gui.status_message = format!("Polyline closed ({n} points)");
                                    } else if sm.pending_point.is_some()
                                        || !sm.polyline_points.is_empty()
                                    {
                                        // Clear pending geometry
                                        sm.pending_point = None;
                                        sm.polyline_points.clear();
                                    } else if matches!(sm.tool, SketchTool::Select) {
                                        sm.show_context_menu = true;
                                    }
                                }
                            }
                            self.mouse.right_pressed = pressed;
                            if !pressed {
                                self.try_snap_to_nearest_view();
                            }
                        }
                        _ => {}
                    }
                    // For LMB-based orbit styles (Gesture, OpenSCAD, etc.)
                    if !pressed && button == &MouseButton::Left {
                        self.try_snap_to_nearest_view();
                    }
                }

                WindowEvent::CursorMoved { position, .. } => {
                    if let Some((lx, ly)) = self.mouse.last_pos {
                        let dx = position.x - lx;
                        let dy = position.y - ly;
                        if (dx * dx + dy * dy) > 9.0 {
                            self.mouse_dragged = true;
                        }

                        // Sketch entity drag (point or multi-point)
                        if self.mouse.left_pressed {
                            let drag_pt = self.screen_to_sketch_plane(position.x, position.y);
                            if let Some(sm) = &mut self.gui.sketch_mode {
                                if !sm.drag_points.is_empty() {
                                    if let (Some(spt), Some((ox, oy))) = (drag_pt, sm.drag_origin) {
                                        let (sx, sy) = if sm.drag_points.len() == 1 {
                                            Self::snap_sketch_coords(sm, spt.0, spt.1)
                                        } else {
                                            (spt.0, spt.1)
                                        };
                                        // Single point + has constraints → use drag_solve
                                        if sm.drag_points.len() == 1
                                            && !sm.sketch.constraints.is_empty()
                                        {
                                            let pid = cadkernel_sketch::PointId(sm.drag_points[0]);
                                            let result = drag_solve(&mut sm.sketch, pid, sx, sy, 50, 1e-8);
                                            if !result.converged {
                                                // Fall back to raw move
                                                if sm.drag_points[0] < sm.sketch.points.len() {
                                                    sm.sketch.points[sm.drag_points[0]].position.x = sx;
                                                    sm.sketch.points[sm.drag_points[0]].position.y = sy;
                                                }
                                            }
                                        } else {
                                            // Multi-point or no constraints: delta-based move
                                            let dx = sx - ox;
                                            let dy = sy - oy;
                                            let n = sm.sketch.points.len();
                                            for &idx in &sm.drag_points {
                                                if idx < n {
                                                    sm.sketch.points[idx].position.x += dx;
                                                    sm.sketch.points[idx].position.y += dy;
                                                }
                                            }
                                        }
                                        sm.drag_origin = Some((sx, sy));
                                        sm.drag_started = true;
                                        self.request_redraw();
                                    }
                                }
                            }
                        }

                        // Update box selection end point during drag
                        if self.mouse.left_pressed {
                            let box_pt = self.screen_to_sketch_plane(position.x, position.y);
                            if let Some(sm) = &mut self.gui.sketch_mode {
                                if sm.box_select_start.is_some() && sm.drag_points.is_empty() {
                                    if let Some(spt) = box_pt {
                                        sm.box_select_end = Some(spt);
                                        self.request_redraw();
                                    }
                                }
                            }
                        }

                        // Update sketch hover preselection
                        {
                            let hover_pt = self.screen_to_sketch_plane(position.x, position.y);
                            if let Some(sm) = &mut self.gui.sketch_mode {
                                if matches!(sm.tool, SketchTool::Select) && sm.drag_points.is_empty() {
                                    if let Some(spt) = hover_pt {
                                        sm.hovered_entity = Self::hit_test_sketch(sm, spt.0, spt.1);
                                    } else {
                                        sm.hovered_entity = None;
                                    }
                                } else {
                                    sm.hovered_entity = None;
                                }
                            }
                        }

                        // In sketch mode, suppress LMB-based orbit (Gesture style)
                        let left_for_nav = self.mouse.left_pressed && self.gui.sketch_mode.is_none();
                        let action = self.nav.resolve_drag(
                            left_for_nav,
                            self.mouse.middle_pressed,
                            self.mouse.right_pressed,
                            self.mouse.shift_held,
                            self.mouse.ctrl_held,
                            self.mouse.alt_held,
                        );

                        match action {
                            NavAction::Orbit => {
                                self.was_orbiting = true;
                                // RotationMode: temporarily shift orbit pivot
                                let saved_target = self.camera.target;
                                self.apply_rotation_mode_pivot();

                                let (sw, sh) = if let Some(rt) = &self.runtime {
                                    let s = rt.gpu.window.inner_size();
                                    (s.width as f32, s.height as f32)
                                } else {
                                    (1440.0, 900.0)
                                };
                                self.nav.apply_orbit(
                                    &mut self.camera.yaw,
                                    &mut self.camera.pitch,
                                    dx as f32,
                                    dy as f32,
                                    position.x as f32,
                                    position.y as f32,
                                    sw,
                                    sh,
                                );

                                // Restore original target if pivot was shifted
                                if self.nav.rotation_mode != crate::nav::RotationMode::WindowCenter {
                                    self.camera.target = saved_target;
                                }
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
                                let factor = self.nav.drag_zoom_factor(-dy as f32 * 0.05);
                                self.camera.distance *= factor;
                                self.camera.distance = self.camera.distance.max(0.01);
                                self.request_redraw();
                            }
                            NavAction::None => {}
                        }
                    }
                    self.mouse.last_pos = Some((position.x, position.y));

                    // Throttled preselection (hover highlight) — every 3 frames
                    self.preselect_frame += 1;
                    if self.preselect_frame % 3 == 0 && !self.mouse.left_pressed && !self.mouse.middle_pressed {
                        self.update_preselection();
                    }
                }

                WindowEvent::MouseWheel { delta, .. } => {
                    let scroll = match delta {
                        MouseScrollDelta::LineDelta(_, y) => *y,
                        MouseScrollDelta::PixelDelta(pos) => pos.y as f32 * 0.01,
                    };
                    let factor = self.nav.scroll_zoom_factor(scroll);

                    // zoom_at_cursor: shift target toward the point under cursor
                    if self.nav.zoom_at_cursor {
                        if let Some((mx, my)) = self.mouse.last_pos {
                            let (sw, sh) = if let Some(rt) = &self.runtime {
                                let s = rt.gpu.window.inner_size();
                                (s.width as f32, s.height as f32)
                            } else {
                                (1440.0, 900.0)
                            };
                            // NDC from cursor position
                            let ndc_x = (mx as f32 / sw) * 2.0 - 1.0;
                            let ndc_y = 1.0 - (my as f32 / sh) * 2.0;
                            // Shift target toward cursor direction proportional to zoom amount
                            let shift = 1.0 - factor; // positive when zooming in
                            let r = self.camera.screen_right();
                            let u = self.camera.screen_up();
                            let scale = self.camera.distance * 0.5;
                            for i in 0..3 {
                                self.camera.target[i] += (r[i] * ndc_x + u[i] * ndc_y) * shift * scale;
                            }
                        }
                    }

                    self.camera.distance *= factor;
                    self.camera.distance = self.camera.distance.max(0.01);
                    self.request_redraw();
                }

                _ => {}
            }
        }

        // Always track cursor position (even when egui consumed the event).
        if let WindowEvent::CursorMoved { position, .. } = &event {
            self.mouse.last_pos = Some((position.x, position.y));
        }

        // General window events.
        match event {
            WindowEvent::CloseRequested => {
                save_settings(self);
                event_loop.exit();
            }

            WindowEvent::KeyboardInput {
                event: key_event, ..
            } if key_event.state == ElementState::Pressed => {
                let ctrl = self.mouse.ctrl_held;
                match key_event.physical_key {
                    PhysicalKey::Code(KeyCode::Escape) => {
                        if self.gui.measurement_mode {
                            self.gui.measurement_mode = false;
                            self.gui.measurement_points.clear();
                            self.gui.status_message = "Measurement mode OFF".into();
                        } else if self.gui.active_task.is_some() {
                            self.gui.active_task = None;
                        } else if let Some(sm) = &mut self.gui.sketch_mode {
                            if sm.pending_point.is_some() || !sm.polyline_points.is_empty() {
                                sm.pending_point = None;
                                sm.polyline_points.clear();
                                sm.selected_entities.clear();
                                self.gui.status_message = "Cleared".into();
                            } else {
                                self.gui.actions.push(GuiAction::Sketcher(SketcherAction::Cancel));
                            }
                        } else if self.scene.selected_id().is_some() {
                            self.scene.deselect_all();
                            self.rebuild_scene_gpu();
                        } else {
                            save_settings(self);
                            event_loop.exit();
                        }
                    }

                    // Sketch tool shortcuts (only when in sketch mode, no ctrl)
                    PhysicalKey::Code(KeyCode::KeyS) if !ctrl && self.gui.sketch_mode.is_some() => {
                        if let Some(sm) = &mut self.gui.sketch_mode {
                            sm.tool = SketchTool::Select;
                            sm.pending_point = None;
                            self.gui.status_message = "Tool: Select".into();
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyL) if !ctrl && self.gui.sketch_mode.is_some() => {
                        if let Some(sm) = &mut self.gui.sketch_mode {
                            sm.tool = SketchTool::Line;
                            sm.pending_point = None;
                            sm.selected_entities.clear();
                            self.gui.status_message = "Tool: Line".into();
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyR) if !ctrl && self.gui.sketch_mode.is_some() => {
                        if let Some(sm) = &mut self.gui.sketch_mode {
                            sm.tool = SketchTool::Rectangle;
                            sm.pending_point = None;
                            sm.selected_entities.clear();
                            self.gui.status_message = "Tool: Rectangle".into();
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyC) if !ctrl && self.gui.sketch_mode.is_some() => {
                        if let Some(sm) = &mut self.gui.sketch_mode {
                            sm.tool = SketchTool::Circle;
                            sm.pending_point = None;
                            sm.selected_entities.clear();
                            self.gui.status_message = "Tool: Circle".into();
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyA) if !ctrl && self.gui.sketch_mode.is_some() => {
                        if let Some(sm) = &mut self.gui.sketch_mode {
                            sm.tool = SketchTool::Arc;
                            sm.pending_point = None;
                            sm.selected_entities.clear();
                            self.gui.status_message = "Tool: Arc".into();
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyE) if !ctrl && self.gui.sketch_mode.is_some() => {
                        if let Some(sm) = &mut self.gui.sketch_mode {
                            sm.tool = SketchTool::Ellipse;
                            sm.pending_point = None;
                            sm.selected_entities.clear();
                            self.gui.status_message = "Tool: Ellipse".into();
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyP) if !ctrl && self.gui.sketch_mode.is_some() => {
                        if let Some(sm) = &mut self.gui.sketch_mode {
                            sm.tool = SketchTool::Point;
                            sm.pending_point = None;
                            sm.selected_entities.clear();
                            self.gui.status_message = "Tool: Point".into();
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyB) if !ctrl && self.gui.sketch_mode.is_some() => {
                        if let Some(sm) = &mut self.gui.sketch_mode {
                            sm.tool = SketchTool::BSpline;
                            sm.pending_point = None;
                            sm.polyline_points.clear();
                            sm.selected_entities.clear();
                            self.gui.status_message = "Tool: B-Spline".into();
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyW) if !ctrl && self.gui.sketch_mode.is_some() => {
                        if let Some(sm) = &mut self.gui.sketch_mode {
                            sm.tool = SketchTool::Polyline;
                            sm.pending_point = None;
                            sm.polyline_points.clear();
                            sm.selected_entities.clear();
                            self.gui.status_message = "Tool: Polyline".into();
                        }
                    }
                    // Enter: close polyline/B-spline
                    PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) if !ctrl && self.gui.sketch_mode.is_some() => {
                        if let Some(sm) = &mut self.gui.sketch_mode {
                            match sm.tool {
                                SketchTool::Polyline if sm.polyline_points.len() >= 3 => {
                                    sm.save_snapshot();
                                    let first = sm.polyline_points[0];
                                    let last = *sm.polyline_points.last().unwrap();
                                    let p0 = sm.sketch.add_point(last.0, last.1);
                                    let p1 = sm.sketch.add_point(first.0, first.1);
                                    sm.sketch.add_line(p0, p1);
                                    let n = sm.polyline_points.len();
                                    sm.polyline_points.clear();
                                    self.gui.status_message = format!("Polyline closed ({n} points)");
                                }
                                SketchTool::BSpline if sm.polyline_points.len() >= 2 => {
                                    sm.save_snapshot();
                                    let pts: Vec<cadkernel_sketch::PointId> = sm.polyline_points
                                        .iter()
                                        .map(|&(px, py)| sm.sketch.add_point(px, py))
                                        .collect();
                                    let n = pts.len();
                                    sm.sketch.add_bspline(pts, 3.min(n - 1), false);
                                    sm.polyline_points.clear();
                                    self.gui.status_message = format!("B-Spline created ({n} control points)");
                                }
                                _ => {}
                            }
                        }
                    }

                    // Sketch constraint shortcuts (H=Horizontal, V=Vertical on selected line)
                    PhysicalKey::Code(KeyCode::KeyH) if !ctrl && self.gui.sketch_mode.is_some() => {
                        if let Some(sm) = &mut self.gui.sketch_mode {
                            Self::apply_sketch_constraint_h(sm, &mut self.gui.status_message);
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyV) if !ctrl && self.gui.sketch_mode.is_some() => {
                        if let Some(sm) = &mut self.gui.sketch_mode {
                            Self::apply_sketch_constraint_v(sm, &mut self.gui.status_message);
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
                    PhysicalKey::Code(KeyCode::KeyV) if !ctrl => {
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
                    PhysicalKey::Code(KeyCode::KeyS) if self.mouse.shift_held && !ctrl => {
                        self.gui.actions.push(GuiAction::ToggleSectionPlane);
                    }
                    PhysicalKey::Code(KeyCode::KeyC) if !ctrl && self.gui.measurement_mode && self.gui.sketch_mode.is_none() => {
                        self.gui.measurement_points.clear();
                        self.gui.status_message = "Measurement points cleared".into();
                    }

                    // W/E/R = gizmo mode (Translate/Rotate/Scale)
                    PhysicalKey::Code(KeyCode::KeyW) if !ctrl && self.gui.sketch_mode.is_none() => {
                        use crate::gui::GizmoMode;
                        self.gui.gizmo_mode = if self.gui.gizmo_mode == GizmoMode::Translate {
                            GizmoMode::None
                        } else {
                            GizmoMode::Translate
                        };
                    }
                    PhysicalKey::Code(KeyCode::KeyE) if !ctrl && self.gui.sketch_mode.is_none() => {
                        use crate::gui::GizmoMode;
                        self.gui.gizmo_mode = if self.gui.gizmo_mode == GizmoMode::Rotate {
                            GizmoMode::None
                        } else {
                            GizmoMode::Rotate
                        };
                    }
                    PhysicalKey::Code(KeyCode::KeyR) if !ctrl && self.gui.sketch_mode.is_none() => {
                        use crate::gui::GizmoMode;
                        self.gui.gizmo_mode = if self.gui.gizmo_mode == GizmoMode::Scale {
                            GizmoMode::None
                        } else {
                            GizmoMode::Scale
                        };
                    }

                    // Ctrl+Z = Undo (sketch undo if in sketch mode)
                    PhysicalKey::Code(KeyCode::KeyZ) if ctrl && !self.mouse.shift_held => {
                        if let Some(sm) = &mut self.gui.sketch_mode {
                            if sm.undo() {
                                self.gui.status_message = "Sketch undo".into();
                            }
                        } else {
                            self.gui.actions.push(GuiAction::Undo);
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyZ) if ctrl && self.mouse.shift_held => {
                        if let Some(sm) = &mut self.gui.sketch_mode {
                            if sm.redo() {
                                self.gui.status_message = "Sketch redo".into();
                            }
                        } else {
                            self.gui.actions.push(GuiAction::Redo);
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyY) if ctrl => {
                        self.gui.actions.push(GuiAction::Redo);
                    }

                    // Delete = delete selected (sketch entities or model objects)
                    PhysicalKey::Code(KeyCode::Delete | KeyCode::Backspace) if !ctrl => {
                        if let Some(sm) = &mut self.gui.sketch_mode {
                            if !sm.selected_entities.is_empty() {
                                sm.save_snapshot();
                                Self::delete_sketch_entities(sm);
                                self.gui.status_message = "Sketch entities deleted".into();
                            }
                        } else {
                            self.gui.actions.push(GuiAction::DeleteSelected);
                        }
                    }

                    // Ctrl+N = new, Ctrl+A = select all, Ctrl+O = open, Ctrl+S = save
                    PhysicalKey::Code(KeyCode::KeyN) if ctrl => {
                        self.gui.actions.push(GuiAction::NewModel);
                    }
                    PhysicalKey::Code(KeyCode::KeyA) if ctrl => {
                        if let Some(sm) = &mut self.gui.sketch_mode {
                            sm.selected_entities.clear();
                            for i in 0..sm.sketch.points.len() {
                                sm.selected_entities.push(SketchEntityRef::Point(i));
                            }
                            for i in 0..sm.sketch.lines.len() {
                                sm.selected_entities.push(SketchEntityRef::Line(i));
                            }
                            for i in 0..sm.sketch.arcs.len() {
                                sm.selected_entities.push(SketchEntityRef::Arc(i));
                            }
                            for i in 0..sm.sketch.circles.len() {
                                sm.selected_entities.push(SketchEntityRef::Circle(i));
                            }
                            for i in 0..sm.sketch.ellipses.len() {
                                sm.selected_entities.push(SketchEntityRef::Ellipse(i));
                            }
                            for i in 0..sm.sketch.bsplines.len() {
                                sm.selected_entities.push(SketchEntityRef::BSpline(i));
                            }
                            self.gui.status_message = format!(
                                "Selected {} entities", sm.selected_entities.len()
                            );
                        } else {
                            self.gui.actions.push(GuiAction::SelectAll);
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyC) if ctrl && self.gui.sketch_mode.is_some() => {
                        self.gui.actions.push(GuiAction::Sketcher(SketcherAction::CopySelection));
                    }
                    PhysicalKey::Code(KeyCode::KeyV) if ctrl && self.gui.sketch_mode.is_some() => {
                        // Paste at origin (0,0); user can drag to reposition
                        self.gui.actions.push(GuiAction::Sketcher(SketcherAction::PasteSelection(0.0, 0.0)));
                    }
                    PhysicalKey::Code(KeyCode::KeyO) if ctrl => {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("CADKernel", &["cadk"])
                            .add_filter("STL", &["stl"])
                            .add_filter("OBJ", &["obj"])
                            .add_filter("All", &["*"])
                            .pick_file()
                        {
                            self.gui.actions.push(GuiAction::OpenFile(path));
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyS) if ctrl => {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("CADKernel", &["cadk"])
                            .set_file_name("model.cadk")
                            .save_file()
                        {
                            self.gui.actions.push(GuiAction::SaveFile(path));
                        }
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

                    // Selection mode shortcuts (only when no text field is focused)
                    PhysicalKey::Code(KeyCode::Digit2 | KeyCode::Numpad2)
                        if !ctrl
                            && !self
                                .runtime
                                .as_ref()
                                .is_some_and(|rt| rt.egui_ctx.wants_keyboard_input()) =>
                    {
                        self.gui
                            .actions
                            .push(GuiAction::SetSelectionMode(SelectionMode::Face));
                    }
                    PhysicalKey::Code(KeyCode::Digit4 | KeyCode::Numpad4)
                        if !ctrl
                            && !self
                                .runtime
                                .as_ref()
                                .is_some_and(|rt| rt.egui_ctx.wants_keyboard_input()) =>
                    {
                        self.gui
                            .actions
                            .push(GuiAction::SetSelectionMode(SelectionMode::Vertex));
                    }

                    // F1 = open keyboard shortcuts reference
                    PhysicalKey::Code(KeyCode::F1) => {
                        self.gui.show_shortcuts = !self.gui.show_shortcuts;
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
                    save_settings(self);
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

/// Rebuild a primitive solid from creation parameters.
fn rebuild_object_from_params(
    params: &crate::scene::CreationParams,
) -> Option<(BRepModel, Handle<SolidData>)> {
    use crate::scene::CreationParams;
    let mut model = BRepModel::new();
    let result = match params {
        CreationParams::Box { width, height, depth } => {
            make_box(&mut model, Point3::ORIGIN, *width, *height, *depth).ok().map(|r| r.solid)
        }
        CreationParams::Cylinder { radius, height } => {
            make_cylinder(&mut model, Point3::ORIGIN, *radius, *height, 64).ok().map(|r| r.solid)
        }
        CreationParams::Sphere { radius } => {
            make_sphere(&mut model, Point3::ORIGIN, *radius, 64, 32).ok().map(|r| r.solid)
        }
        CreationParams::Cone { base_radius, top_radius, height } => {
            make_cone(&mut model, Point3::ORIGIN, *base_radius, *top_radius, *height, 64).ok().map(|r| r.solid)
        }
        CreationParams::Torus { major_radius, minor_radius } => {
            make_torus(&mut model, Point3::ORIGIN, *major_radius, *minor_radius, 64, 32).ok().map(|r| r.solid)
        }
        _ => None,
    };
    result.map(|solid| (model, solid))
}

/// Settings file path — stored alongside the executable in a private directory.
fn settings_path() -> PathBuf {
    let mut dir = std::env::current_dir().unwrap_or_default();
    dir.push(".priv-storage");
    let _ = std::fs::create_dir_all(&dir);
    dir.push("cadkernel-settings.json");
    dir
}

/// Persisted settings (subset of runtime state worth saving across sessions).
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct AppSettings {
    nav: NavConfig,
    show_grid: bool,
    show_model_tree: bool,
    show_properties: bool,
    recent_files: Vec<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            nav: NavConfig::new(),
            show_grid: true,
            show_model_tree: true,
            show_properties: true,
            recent_files: Vec::new(),
        }
    }
}

/// Look up which B-Rep face owns a given triangle index.
fn lookup_face(
    tri_index: usize,
    face_map: &[(Handle<FaceData>, usize, usize)],
) -> Option<Handle<FaceData>> {
    face_map.iter().find_map(|&(fh, start, count)| {
        if tri_index >= start && tri_index < start + count {
            Some(fh)
        } else {
            None
        }
    })
}

fn selected_edge_pairs(
    entities: &[SelectedEntity],
    model: &BRepModel,
) -> Option<Vec<(Handle<VertexData>, Handle<VertexData>)>> {
    let edges: Vec<Handle<EdgeData>> = entities.iter().filter_map(|e| {
        if let SelectedEntity::Edge(eh) = e { Some(*eh) } else { None }
    }).collect();
    if edges.is_empty() {
        return None;
    }
    let pairs: Vec<_> = edges.iter().filter_map(|eh| {
        model.edges.get(*eh).map(|ed| (ed.start, ed.end))
    }).collect();
    Some(pairs)
}

fn toggle_entity(list: &mut Vec<SelectedEntity>, entity: SelectedEntity) {
    if let Some(pos) = list.iter().position(|e| e == &entity) {
        list.remove(pos);
    } else {
        list.push(entity);
    }
}

fn load_settings() -> AppSettings {
    let path = settings_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_settings(app: &CadApp) {
    let settings = AppSettings {
        nav: app.nav.clone(),
        show_grid: app.show_grid,
        show_model_tree: app.gui.show_model_tree,
        show_properties: app.gui.show_properties,
        recent_files: app.gui.recent_files.clone(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&settings) {
        let _ = std::fs::write(settings_path(), json);
    }
}

pub fn run_gui() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = CadApp::new();
    // Restore saved settings
    let settings = load_settings();
    app.nav = settings.nav;
    app.show_grid = settings.show_grid;
    app.gui.show_model_tree = settings.show_model_tree;
    app.gui.show_properties = settings.show_properties;
    app.gui.recent_files = settings.recent_files;
    event_loop.run_app(&mut app).unwrap();
}

/// Map an `FemMaterial` to a preset label by Young's modulus signature.
///
/// `FemMaterial` does not store the preset name, so we round-trip through
/// the canonical preset values from `MaterialPreset::to_material()` to keep
/// the Summary / Report output user-friendly. Falls back to "Custom" for
/// anything that doesn't match.
fn fem_material_label(material: &cadkernel_modeling::FemMaterial) -> &'static str {
    let e = material.youngs_modulus;
    let rho = material.density;
    if (e - 210.0e9).abs() < 1.0 && (rho - 7850.0).abs() < 1e-6 {
        "Steel"
    } else if (e - 70.0e9).abs() < 1.0 && (rho - 2700.0).abs() < 1e-6 {
        "Aluminum"
    } else if (e - 114.0e9).abs() < 1.0 && (rho - 4430.0).abs() < 1e-6 {
        "Titanium"
    } else if (e - 117.0e9).abs() < 1.0 && (rho - 8960.0).abs() < 1e-6 {
        "Copper"
    } else if (e - 30.0e9).abs() < 1.0 && (rho - 2400.0).abs() < 1e-6 {
        "Concrete"
    } else if (e - 170.0e9).abs() < 1.0 && (rho - 7200.0).abs() < 1e-6 {
        "CastIron"
    } else {
        "Custom"
    }
}

fn fem_bc_kind_label(bc: &cadkernel_modeling::BoundaryCondition) -> &'static str {
    use cadkernel_modeling::BoundaryCondition as BC;
    match bc {
        BC::FixedNode(_) => "FixedNode",
        BC::Force { .. } => "Force",
        BC::Pressure { .. } => "Pressure",
        BC::Displacement { .. } => "Displacement",
        BC::Gravity { .. } => "Gravity",
        BC::DistributedLoad { .. } => "DistributedLoad",
        BC::Spring { .. } => "Spring",
        BC::CentrifugalLoad { .. } => "CentrifugalLoad",
        BC::SelfWeight { .. } => "SelfWeight",
        BC::SpringConstraint { .. } => "SpringConstraint",
        BC::BodyLoad { .. } => "BodyLoad",
        BC::InitialTemperature { .. } => "InitialTemperature",
        BC::SectionPrint { .. } => "SectionPrint",
        BC::TieConstraint { .. } => "TieConstraint",
        BC::RigidBody { .. } => "RigidBody",
        BC::ContactConstraint { .. } => "ContactConstraint",
    }
}

// ---------------------------------------------------------------------------
// Headless test surface
// ---------------------------------------------------------------------------
//
// Re-exported from `lib.rs` as `cadkernel_viewer::test_support`. These wrappers
// push a single `GuiAction` onto `gui.actions` and run the production
// dispatcher (`process_actions`) — runtime is `None`, so all GPU/winit calls
// are skipped via existing `if let Some(rt)` guards. Tests can then assert on
// the resulting `Scene` / `Camera` to verify dispatcher wiring end-to-end
// without duplicating modeling-crate calls.
//
// Kept internal-only via `#[doc(hidden)]`. The `GuiAction` enum and its
// sub-enums stay `pub(crate)`; tests pass primitive arguments and the helpers
// build the action internally.

impl CadApp {
    /// Headless `CadApp` constructor for integration tests.
    #[doc(hidden)]
    pub fn new_headless() -> Self {
        Self::new()
    }

    /// Read-only view of the scene populated by dispatched actions.
    #[doc(hidden)]
    pub fn scene_ref(&self) -> &crate::scene::Scene {
        &self.scene
    }

    /// Read-only view of the camera updated by dispatched actions.
    #[doc(hidden)]
    pub fn camera_ref(&self) -> &Camera {
        &self.camera
    }

    fn dispatch(&mut self, action: GuiAction) {
        self.gui.actions.push(action);
        self.process_actions();
    }

    #[doc(hidden)]
    pub fn dispatch_new_model(&mut self) {
        self.dispatch(GuiAction::NewModel);
    }

    #[doc(hidden)]
    pub fn dispatch_create_box(&mut self, width: f64, height: f64, depth: f64) {
        self.dispatch(GuiAction::CreateBox { width, height, depth });
    }

    #[doc(hidden)]
    pub fn dispatch_create_cylinder(&mut self, radius: f64, height: f64) {
        self.dispatch(GuiAction::CreateCylinder { radius, height });
    }

    #[doc(hidden)]
    pub fn dispatch_create_sphere(&mut self, radius: f64) {
        self.dispatch(GuiAction::CreateSphere { radius });
    }

    #[doc(hidden)]
    pub fn dispatch_create_cone(&mut self, base_radius: f64, top_radius: f64, height: f64) {
        self.dispatch(GuiAction::CreateCone { base_radius, top_radius, height });
    }

    #[doc(hidden)]
    pub fn dispatch_create_torus(&mut self, major_radius: f64, minor_radius: f64) {
        self.dispatch(GuiAction::CreateTorus { major_radius, minor_radius });
    }

    #[doc(hidden)]
    pub fn dispatch_create_tube(&mut self, outer_radius: f64, inner_radius: f64, height: f64) {
        self.dispatch(GuiAction::CreateTube { outer_radius, inner_radius, height });
    }

    #[doc(hidden)]
    pub fn dispatch_create_prism(&mut self, radius: f64, height: f64, sides: usize) {
        self.dispatch(GuiAction::CreatePrism { radius, height, sides });
    }

    #[doc(hidden)]
    pub fn dispatch_create_wedge(&mut self, dx: f64, dy: f64, dz: f64, dx2: f64, dy2: f64) {
        self.dispatch(GuiAction::CreateWedge { dx, dy, dz, dx2, dy2 });
    }

    #[doc(hidden)]
    pub fn dispatch_create_ellipsoid(&mut self, rx: f64, ry: f64, rz: f64) {
        self.dispatch(GuiAction::CreateEllipsoid { rx, ry, rz });
    }

    #[doc(hidden)]
    pub fn dispatch_create_helix(
        &mut self,
        radius: f64,
        pitch: f64,
        turns: f64,
        tube_radius: f64,
    ) {
        self.dispatch(GuiAction::CreateHelix { radius, pitch, turns, tube_radius });
    }

    #[doc(hidden)]
    pub fn dispatch_reset_camera(&mut self) {
        self.dispatch(GuiAction::ResetCamera);
    }

    #[doc(hidden)]
    pub fn dispatch_toggle_projection(&mut self) {
        self.dispatch(GuiAction::ToggleProjection);
    }

    /// Seed `last_sketch` with a square profile centered at the origin on the
    /// XY plane, so PartDesign sketch-driven actions have a profile to consume
    /// without the full Sketcher state machine.
    #[doc(hidden)]
    pub fn seed_test_sketch_square(&mut self, side: f64) {
        let half = side * 0.5;
        let mut sketch = cadkernel_sketch::Sketch::new();
        let p0 = sketch.add_point(-half, -half);
        let p1 = sketch.add_point(half, -half);
        let p2 = sketch.add_point(half, half);
        let p3 = sketch.add_point(-half, half);
        sketch.add_line(p0, p1);
        sketch.add_line(p1, p2);
        sketch.add_line(p2, p3);
        sketch.add_line(p3, p0);
        self.gui.last_sketch = Some((sketch, cadkernel_sketch::WorkPlane::xy()));
    }

    #[doc(hidden)]
    pub fn dispatch_pad_sketch(&mut self, depth: f64, symmetric: bool) {
        self.dispatch(GuiAction::PartDesign(crate::gui::PartDesignAction::PadSketch {
            depth,
            symmetric,
        }));
    }

    #[doc(hidden)]
    pub fn dispatch_pocket_sketch(&mut self, depth: f64, through_all: bool) {
        self.dispatch(GuiAction::PartDesign(crate::gui::PartDesignAction::PocketSketch {
            depth,
            through_all,
        }));
    }

    #[doc(hidden)]
    pub fn dispatch_groove_sketch(&mut self, angle_deg: f64) {
        self.dispatch(GuiAction::PartDesign(crate::gui::PartDesignAction::GrooveSketch {
            angle: angle_deg,
        }));
    }

    #[doc(hidden)]
    pub fn dispatch_hole_sketch(&mut self, radius: f64, depth: f64) {
        self.dispatch(GuiAction::PartDesign(crate::gui::PartDesignAction::HoleSketch {
            radius,
            depth,
        }));
    }

    #[doc(hidden)]
    pub fn dispatch_countersunk_hole_sketch(&mut self, radius: f64, depth: f64, angle_deg: f64) {
        self.dispatch(GuiAction::PartDesign(
            crate::gui::PartDesignAction::CountersunkHoleSketch {
                radius,
                depth,
                countersink_angle: angle_deg,
            },
        ));
    }

    #[doc(hidden)]
    pub fn dispatch_draft_line(&mut self) {
        self.dispatch(GuiAction::Draft(crate::gui::DraftAction::Line));
    }

    #[doc(hidden)]
    pub fn dispatch_draft_circle(&mut self) {
        self.dispatch(GuiAction::Draft(crate::gui::DraftAction::Circle));
    }

    #[doc(hidden)]
    pub fn dispatch_draft_arc(&mut self) {
        self.dispatch(GuiAction::Draft(crate::gui::DraftAction::Arc));
    }

    #[doc(hidden)]
    pub fn dispatch_draft_ellipse(&mut self) {
        self.dispatch(GuiAction::Draft(crate::gui::DraftAction::Ellipse));
    }

    #[doc(hidden)]
    pub fn dispatch_draft_point(&mut self) {
        self.dispatch(GuiAction::Draft(crate::gui::DraftAction::Point));
    }

    #[doc(hidden)]
    pub fn dispatch_draft_wire(&mut self) {
        self.dispatch(GuiAction::Draft(crate::gui::DraftAction::Wire));
    }

    #[doc(hidden)]
    pub fn dispatch_draft_bspline(&mut self) {
        self.dispatch(GuiAction::Draft(crate::gui::DraftAction::BSpline));
    }

    #[doc(hidden)]
    pub fn dispatch_draft_bezier(&mut self) {
        self.dispatch(GuiAction::Draft(crate::gui::DraftAction::Bezier));
    }

    #[doc(hidden)]
    pub fn dispatch_draft_hatch(&mut self) {
        self.dispatch(GuiAction::Draft(crate::gui::DraftAction::Hatch));
    }

    #[doc(hidden)]
    pub fn dispatch_draft_text(&mut self) {
        self.dispatch(GuiAction::Draft(crate::gui::DraftAction::Text));
    }

    #[doc(hidden)]
    pub fn dispatch_draft_upgrade(&mut self) {
        self.dispatch(GuiAction::Draft(crate::gui::DraftAction::Upgrade));
    }

    #[doc(hidden)]
    pub fn dispatch_draft_downgrade(&mut self) {
        self.dispatch(GuiAction::Draft(crate::gui::DraftAction::Downgrade));
    }

    #[doc(hidden)]
    pub fn dispatch_draft_wire_to_bspline(&mut self) {
        self.dispatch(GuiAction::Draft(crate::gui::DraftAction::WireToBSpline));
    }

    #[doc(hidden)]
    pub fn dispatch_draft_to_sketch(&mut self) {
        self.dispatch(GuiAction::Draft(crate::gui::DraftAction::ToSketch));
    }

    #[doc(hidden)]
    pub fn dispatch_draft_clone(&mut self) {
        self.dispatch(GuiAction::Draft(crate::gui::DraftAction::Clone));
    }

    #[doc(hidden)]
    pub fn dispatch_draft_array_rect(&mut self) {
        self.dispatch(GuiAction::Draft(crate::gui::DraftAction::ArrayRect));
    }

    #[doc(hidden)]
    pub fn dispatch_draft_array_polar(&mut self) {
        self.dispatch(GuiAction::Draft(crate::gui::DraftAction::ArrayPolar));
    }

    #[doc(hidden)]
    pub fn dispatch_draft_array_path(&mut self) {
        self.dispatch(GuiAction::Draft(crate::gui::DraftAction::ArrayPath));
    }

    #[doc(hidden)]
    pub fn dispatch_draft_array_point(&mut self) {
        self.dispatch(GuiAction::Draft(crate::gui::DraftAction::ArrayPoint));
    }

    /// Read whether `gui.last_sketch` was populated by a dispatch (used by
    /// the `D::ToSketch` test).
    #[doc(hidden)]
    pub fn last_sketch_is_set(&self) -> bool {
        self.gui.last_sketch.is_some()
    }

    // -- Phase B-cont — Part workbench dispatch wrappers --------------------

    #[doc(hidden)]
    pub fn dispatch_part_face_from_wires(&mut self) {
        self.dispatch(GuiAction::Part(crate::gui::PartAction::FaceFromWires));
    }

    #[doc(hidden)]
    pub fn dispatch_part_connect_shapes(&mut self) {
        self.dispatch(GuiAction::Part(crate::gui::PartAction::ConnectShapes));
    }

    #[doc(hidden)]
    pub fn dispatch_part_embed_shapes(&mut self) {
        self.dispatch(GuiAction::Part(crate::gui::PartAction::EmbedShapes));
    }

    #[doc(hidden)]
    pub fn dispatch_part_cutout_shapes(&mut self) {
        self.dispatch(GuiAction::Part(crate::gui::PartAction::CutoutShapes));
    }

    #[doc(hidden)]
    pub fn dispatch_part_explode_compound(&mut self) {
        self.dispatch(GuiAction::Part(crate::gui::PartAction::ExplodeCompound));
    }

    #[doc(hidden)]
    pub fn dispatch_part_compound_filter(&mut self) {
        self.dispatch(GuiAction::Part(crate::gui::PartAction::CompoundFilter));
    }

    #[doc(hidden)]
    pub fn dispatch_part_boolean_fragments(&mut self) {
        self.dispatch(GuiAction::Part(crate::gui::PartAction::BooleanFragments));
    }

    #[doc(hidden)]
    pub fn dispatch_part_slice_to_compound(&mut self) {
        self.dispatch(GuiAction::Part(crate::gui::PartAction::SliceToCompound));
    }

    #[doc(hidden)]
    pub fn dispatch_part_points_from_shape(&mut self) {
        self.dispatch(GuiAction::Part(crate::gui::PartAction::PointsFromShape));
    }

    #[doc(hidden)]
    pub fn dispatch_part_convert_to_solid(&mut self) {
        self.dispatch(GuiAction::Part(crate::gui::PartAction::ConvertToSolid));
    }

    #[doc(hidden)]
    pub fn dispatch_part_auto_defeaturing(&mut self, threshold: f64) {
        self.dispatch(GuiAction::Part(crate::gui::PartAction::AutoDefeaturing { threshold }));
    }

    #[doc(hidden)]
    pub fn dispatch_part_transformed_copy(&mut self, dx: f64, dy: f64, dz: f64) {
        self.dispatch(GuiAction::Part(crate::gui::PartAction::TransformedCopy { dx, dy, dz }));
    }

    #[doc(hidden)]
    pub fn dispatch_part_coons_patch(&mut self) {
        self.dispatch(GuiAction::Part(crate::gui::PartAction::CoonsPatch));
    }

    /// Toggle-select an object at the given index (test helper for
    /// multi-select boolean / fragment ops).
    #[doc(hidden)]
    pub fn toggle_select_index(&mut self, idx: usize) {
        if let Some(obj) = self.scene.objects.get(idx) {
            let id = obj.id;
            self.scene.toggle_select(id);
        }
    }

    // -- Phase C1 — Draft transforms + EASY stragglers ---------------------

    #[doc(hidden)]
    pub fn dispatch_draft_move(&mut self) {
        self.dispatch(GuiAction::Draft(crate::gui::DraftAction::Move));
    }

    #[doc(hidden)]
    pub fn dispatch_draft_rotate(&mut self) {
        self.dispatch(GuiAction::Draft(crate::gui::DraftAction::Rotate));
    }

    #[doc(hidden)]
    pub fn dispatch_draft_scale(&mut self) {
        self.dispatch(GuiAction::Draft(crate::gui::DraftAction::Scale));
    }

    #[doc(hidden)]
    pub fn dispatch_draft_mirror(&mut self) {
        self.dispatch(GuiAction::Draft(crate::gui::DraftAction::Mirror));
    }

    #[doc(hidden)]
    pub fn dispatch_surface_coons(&mut self) {
        self.dispatch(GuiAction::Surface(crate::gui::SurfaceAction::Coons));
    }

    #[doc(hidden)]
    pub fn dispatch_fem_summary(&mut self) {
        self.dispatch(GuiAction::Fem(crate::gui::FemAction::Summary));
    }

    #[doc(hidden)]
    pub fn dispatch_fem_report(&mut self) {
        self.dispatch(GuiAction::Fem(crate::gui::FemAction::Report));
    }

    // -- Phase C2 — Draft modify + ProjectCurvesOnSurface ----------------

    #[doc(hidden)]
    pub fn dispatch_draft_offset(&mut self) {
        self.dispatch(GuiAction::Draft(crate::gui::DraftAction::Offset));
    }

    #[doc(hidden)]
    pub fn dispatch_draft_trim(&mut self) {
        self.dispatch(GuiAction::Draft(crate::gui::DraftAction::Trim));
    }

    #[doc(hidden)]
    pub fn dispatch_draft_stretch(&mut self) {
        self.dispatch(GuiAction::Draft(crate::gui::DraftAction::Stretch));
    }

    #[doc(hidden)]
    pub fn dispatch_draft_facebinder(&mut self) {
        self.dispatch(GuiAction::Draft(crate::gui::DraftAction::Facebinder));
    }

    #[doc(hidden)]
    pub fn dispatch_part_project_curves_on_surface(&mut self) {
        self.dispatch(GuiAction::Part(crate::gui::PartAction::ProjectCurvesOnSurface));
    }

    // -- Phase C3 — Surface ops + PartDesign Loft/Pipe ----------------------

    #[doc(hidden)]
    pub fn dispatch_surface_sections(&mut self) {
        self.dispatch(GuiAction::Surface(crate::gui::SurfaceAction::Sections));
    }

    #[doc(hidden)]
    pub fn dispatch_surface_extend(&mut self) {
        self.dispatch(GuiAction::Surface(crate::gui::SurfaceAction::Extend));
    }

    #[doc(hidden)]
    pub fn dispatch_surface_blend(&mut self) {
        self.dispatch(GuiAction::Surface(crate::gui::SurfaceAction::Blend));
    }

    #[doc(hidden)]
    pub fn dispatch_partdesign_additive_loft(&mut self) {
        self.dispatch(GuiAction::PartDesign(
            crate::gui::PartDesignAction::AdditiveLoft,
        ));
    }

    #[doc(hidden)]
    pub fn dispatch_partdesign_additive_pipe(&mut self) {
        self.dispatch(GuiAction::PartDesign(
            crate::gui::PartDesignAction::AdditivePipe,
        ));
    }

    #[doc(hidden)]
    pub fn dispatch_partdesign_subtractive_loft(&mut self) {
        self.dispatch(GuiAction::PartDesign(
            crate::gui::PartDesignAction::SubtractiveLoft,
        ));
    }

    #[doc(hidden)]
    pub fn dispatch_partdesign_subtractive_pipe(&mut self) {
        self.dispatch(GuiAction::PartDesign(
            crate::gui::PartDesignAction::SubtractivePipe,
        ));
    }

    /// Inject a synthetic FEM analysis container so Summary / Report tests
    /// don't have to spin up the full Sketcher → tet-mesher state machine.
    #[doc(hidden)]
    pub fn seed_test_fem_analysis(&mut self) {
        let mesh = cadkernel_modeling::TetMesh {
            nodes: vec![
                cadkernel_math::Point3 { x: 0.0, y: 0.0, z: 0.0 },
                cadkernel_math::Point3 { x: 1.0, y: 0.0, z: 0.0 },
                cadkernel_math::Point3 { x: 0.0, y: 1.0, z: 0.0 },
                cadkernel_math::Point3 { x: 0.0, y: 0.0, z: 1.0 },
            ],
            elements: vec![[0, 1, 2, 3]],
        };
        let mut container = cadkernel_modeling::AnalysisContainer::new(
            mesh,
            cadkernel_modeling::FemMaterial::steel(),
        );
        container.add_bc(cadkernel_modeling::BoundaryCondition::FixedNode(0));
        container.add_bc(cadkernel_modeling::BoundaryCondition::Force {
            node: 3,
            force: cadkernel_math::Vec3 { x: 0.0, y: 0.0, z: -1000.0 },
        });
        self.gui.fem_analysis = Some(container);
    }
}
