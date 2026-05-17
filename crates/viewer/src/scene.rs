//! Multi-object scene management for the CAD viewer.
//!
//! Each `SceneObject` owns its own BRepModel, mesh, and GPU-ready vertex data.
//! The `Scene` holds all objects and provides methods for adding, removing,
//! toggling visibility, and iterating visible objects for rendering.

use cadkernel_api::{EdgeRef, FaceRef, SolidId};
use cadkernel_io::{Mesh, tessellate_solid_with_face_map};
use cadkernel_topology::{BRepModel, EdgeData, FaceData, Handle, SolidData, VertexData};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::render::{Vertex, mesh_to_vertices};

/// Unique object identifier within a scene.
pub type ObjectId = u32;

/// Parameters used to create a scene object (for parametric editing).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CreationParams {
    // Primitives
    Box {
        width: f64,
        height: f64,
        depth: f64,
    },
    Cylinder {
        radius: f64,
        height: f64,
    },
    Sphere {
        radius: f64,
    },
    Cone {
        base_radius: f64,
        top_radius: f64,
        height: f64,
    },
    Torus {
        major_radius: f64,
        minor_radius: f64,
    },
    Tube {
        outer_radius: f64,
        inner_radius: f64,
        height: f64,
    },
    Prism {
        radius: f64,
        height: f64,
        sides: usize,
    },
    Wedge {
        dx: f64,
        dy: f64,
        dz: f64,
        dx2: f64,
        dy2: f64,
    },
    Ellipsoid {
        rx: f64,
        ry: f64,
        rz: f64,
    },
    Helix {
        radius: f64,
        pitch: f64,
        turns: f64,
        tube_radius: f64,
    },
    Imported {
        path: String,
    },
    Extruded,
    Revolved,
    Boolean {
        op: String,
    },
    // PartDesign features
    Fillet {
        radius: f64,
    },
    Chamfer {
        distance: f64,
    },
    Shell {
        thickness: f64,
    },
    Mirror {
        plane: u8,
    },
    Pattern {
        count: usize,
        spacing: f64,
        axis: u8,
    },
    Groove {
        angle: f64,
    },
    Sprocket {
        teeth: u32,
        roller_diameter: f64,
        pitch: f64,
        bore: f64,
    },
    InvoluteGear {
        teeth: u32,
        module_val: f64,
        pressure_angle: f64,
    },
    // Draft
    DraftLine {
        length: f64,
        angle: f64,
    },
    DraftCircle {
        radius: f64,
    },
    DraftRectangle {
        width: f64,
        height: f64,
    },
    DraftPolygon {
        radius: f64,
        sides: usize,
    },
    DraftArc {
        radius: f64,
        start_angle: f64,
        end_angle: f64,
    },
    DraftEllipse {
        rx: f64,
        ry: f64,
    },
    // Surface
    SurfacePipe {
        radius: f64,
        length: f64,
    },
    SurfaceRuled {
        width: f64,
        depth: f64,
        offset: f64,
    },
    // Boolean with tool
    BooleanOp {
        op_type: u8,
        width: f64,
        height: f64,
        depth: f64,
        offset_x: f64,
        offset_y: f64,
        offset_z: f64,
    },
    // Scale
    ScaleOp {
        factor: f64,
    },
}

/// A single object in the 3D scene.
#[derive(Clone)]
pub struct SceneObject {
    pub id: ObjectId,
    pub name: String,
    pub model: BRepModel,
    pub solid: Handle<SolidData>,
    pub mesh: Mesh,
    pub vertices: Vec<Vertex>,
    pub color: [f32; 4],
    pub visible: bool,
    pub selected: bool,
    pub params: Option<CreationParams>,
    pub parent_id: Option<ObjectId>,
    pub is_body: bool,
    pub is_tip: bool,
    pub suppressed: bool,
    pub has_error: bool,
    pub needs_recompute: bool,
    /// Mapping from triangle index ranges to B-Rep face handles.
    /// Each entry: (face_handle, start_triangle_index, triangle_count).
    pub face_tri_map: Vec<(Handle<FaceData>, usize, usize)>,
    /// Edge endpoints for picking: (start_pos, end_pos) per edge, parallel to edge_handles.
    pub edge_positions: Vec<([f32; 3], [f32; 3])>,
    /// B-Rep edge handles, parallel to edge_positions.
    pub edge_handles: Vec<Handle<EdgeData>>,
    /// Vertex positions for picking, parallel to vertex_handles.
    pub vertex_positions: Vec<[f32; 3]>,
    /// B-Rep vertex handles, parallel to vertex_positions.
    pub vertex_handles: Vec<Handle<VertexData>>,
    /// Group id this object belongs to (0 = ungrouped).
    pub group_id: u32,
    /// Axis-aligned bounding box minimum (for frustum culling).
    pub aabb_min: [f32; 3],
    /// Axis-aligned bounding box maximum (for frustum culling).
    pub aabb_max: [f32; 3],
    /// A3.3 — api `SolidId` for objects created via `Session::execute`.
    ///
    /// Invariant: when `Some(id)`, `Session::document()` contains a matching
    /// entry — autosave snapshots will round-trip this object. When `None`,
    /// the solid is viewer-local only (file import, undo restore, tree-only
    /// Draft/Surface stubs) and will not appear in autosave snapshots.
    pub solid_id: Option<SolidId>,
}

pub fn compute_aabb(vertices: &[Vertex]) -> ([f32; 3], [f32; 3]) {
    if vertices.is_empty() {
        return ([0.0; 3], [0.0; 3]);
    }
    let mut mn = [f32::MAX; 3];
    let mut mx = [f32::MIN; 3];
    for v in vertices {
        for i in 0..3 {
            mn[i] = mn[i].min(v.position[i]);
            mx[i] = mx[i].max(v.position[i]);
        }
    }
    (mn, mx)
}

/// Default color palette (rotating, similar to FreeCAD).
const DEFAULT_COLORS: &[[f32; 4]] = &[
    [0.70, 0.75, 0.80, 1.0], // steel blue
    [0.85, 0.55, 0.40, 1.0], // terracotta
    [0.45, 0.75, 0.50, 1.0], // sage green
    [0.75, 0.60, 0.80, 1.0], // lavender
    [0.90, 0.80, 0.45, 1.0], // gold
    [0.55, 0.70, 0.85, 1.0], // sky blue
    [0.80, 0.50, 0.55, 1.0], // rose
    [0.60, 0.80, 0.75, 1.0], // teal
];

/// Selection highlight color multiplier.
pub const SELECTION_TINT: [f32; 4] = [0.3, 0.9, 0.3, 1.0];

/// A named group of objects for batch visibility/selection.
#[derive(Clone, Debug)]
pub struct ObjectGroup {
    pub id: u32,
    pub name: String,
    pub visible: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TechDrawBomPart {
    pub description: String,
    pub material: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SectionBox {
    pub min: [f64; 3],
    pub max: [f64; 3],
    pub active: bool,
}

impl SectionBox {
    pub fn inactive() -> Self {
        Self {
            min: [-1.0, -1.0, -1.0],
            max: [1.0, 1.0, 1.0],
            active: false,
        }
    }

    pub fn new(min: [f64; 3], max: [f64; 3], active: bool) -> Self {
        let (min, max) = sanitize_bounds(min, max);
        Self { min, max, active }
    }

    pub fn contains_f32(&self, p: [f32; 3]) -> bool {
        const EPS: f64 = 1e-5;
        (0..3).all(|axis| {
            let v = p[axis] as f64;
            v >= self.min[axis] - EPS && v <= self.max[axis] + EPS
        })
    }
}

impl Default for SectionBox {
    fn default() -> Self {
        Self::inactive()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExplodedView {
    pub factor: f64,
}

impl Default for ExplodedView {
    fn default() -> Self {
        Self { factor: 0.0 }
    }
}

/// Multi-object scene.
#[derive(Clone)]
pub struct Scene {
    pub objects: Vec<SceneObject>,
    next_id: ObjectId,
    pub active_body_id: Option<ObjectId>,
    pub groups: Vec<ObjectGroup>,
    next_group_id: u32,
    pub section_box: SectionBox,
    pub exploded_view: ExplodedView,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            next_id: 1,
            active_body_id: None,
            groups: Vec::new(),
            next_group_id: 1,
            section_box: SectionBox::default(),
            exploded_view: ExplodedView::default(),
        }
    }

    /// Add a new object to the scene. Returns its ObjectId.
    ///
    /// `solid_id` is `Some(_)` when the solid was produced by
    /// `Session::execute(...)` so the api `Document` already has a matching
    /// entry; pass `None` for legacy/viewer-local solids (file import, undo
    /// restore, etc.) — those will not be tracked by autosave.
    pub fn add_object(
        &mut self,
        name: impl Into<String>,
        model: BRepModel,
        solid: Handle<SolidData>,
        params: Option<CreationParams>,
        solid_id: Option<SolidId>,
    ) -> ObjectId {
        let (mesh, face_tri_map) = tessellate_solid_with_face_map(&model, solid);
        let vertices = mesh_to_vertices(&mesh);
        let (edge_positions, edge_handles) = collect_edge_data(&model, solid);
        let (vertex_positions, vertex_handles) = collect_vertex_data(&model, solid);
        let (aabb_min, aabb_max) = compute_aabb(&vertices);
        let id = self.next_id;
        self.next_id += 1;
        let color_idx = (id as usize - 1) % DEFAULT_COLORS.len();
        self.objects.push(SceneObject {
            id,
            name: name.into(),
            model,
            solid,
            mesh,
            vertices,
            color: DEFAULT_COLORS[color_idx],
            visible: true,
            selected: false,
            params,
            parent_id: None,
            is_body: false,
            is_tip: false,
            suppressed: false,
            has_error: false,
            needs_recompute: false,
            face_tri_map,
            edge_positions,
            edge_handles,
            vertex_positions,
            vertex_handles,
            group_id: 0,
            aabb_min,
            aabb_max,
            solid_id,
        });
        id
    }

    /// Add object from a pre-tessellated mesh (for imported files).
    ///
    /// `solid_id` is `Some(_)` only when a Session-tracked solid backs this
    /// mesh; mesh-only entries (Draft stubs, raw imported meshes) pass `None`.
    pub fn add_mesh_object(
        &mut self,
        name: impl Into<String>,
        mesh: Mesh,
        params: Option<CreationParams>,
        solid_id: Option<SolidId>,
    ) -> ObjectId {
        let vertices = mesh_to_vertices(&mesh);
        let (aabb_min, aabb_max) = compute_aabb(&vertices);
        let id = self.next_id;
        self.next_id += 1;
        let color_idx = (id as usize - 1) % DEFAULT_COLORS.len();
        self.objects.push(SceneObject {
            id,
            name: name.into(),
            model: BRepModel::new(),
            solid: Handle::from_raw_parts(0, 0),
            mesh,
            vertices,
            color: DEFAULT_COLORS[color_idx],
            visible: true,
            selected: false,
            params,
            parent_id: None,
            is_body: false,
            is_tip: false,
            suppressed: false,
            has_error: false,
            needs_recompute: false,
            face_tri_map: Vec::new(),
            edge_positions: Vec::new(),
            edge_handles: Vec::new(),
            vertex_positions: Vec::new(),
            vertex_handles: Vec::new(),
            group_id: 0,
            aabb_min,
            aabb_max,
            solid_id,
        });
        id
    }

    /// Remove an object by id.
    pub fn remove_object(&mut self, id: ObjectId) -> bool {
        let len = self.objects.len();
        self.objects.retain(|o| o.id != id);
        self.objects.len() < len
    }

    /// Get a mutable reference to an object.
    pub fn get_mut(&mut self, id: ObjectId) -> Option<&mut SceneObject> {
        self.objects.iter_mut().find(|o| o.id == id)
    }

    /// Get a reference to an object.
    pub fn get(&self, id: ObjectId) -> Option<&SceneObject> {
        self.objects.iter().find(|o| o.id == id)
    }

    /// Iterate visible objects.
    pub fn visible_objects(&self) -> impl Iterator<Item = &SceneObject> {
        self.objects.iter().filter(|o| o.visible)
    }

    pub fn techdraw_bom_parts(&self) -> Vec<TechDrawBomPart> {
        self.objects
            .iter()
            .filter(|obj| obj.visible && !obj.suppressed)
            .map(|obj| TechDrawBomPart {
                description: if obj.name.trim().is_empty() {
                    format!("Object {}", obj.id)
                } else {
                    obj.name.clone()
                },
                material: "Unspecified".into(),
            })
            .collect()
    }

    pub fn visible_bounds(&self) -> Option<([f64; 3], [f64; 3])> {
        let mut mn = [f64::MAX; 3];
        let mut mx = [f64::MIN; 3];
        let mut any = false;

        for obj in self.visible_objects() {
            if obj.vertices.is_empty() {
                continue;
            }
            let (obj_min, obj_max) = compute_aabb(&obj.vertices);
            for axis in 0..3 {
                mn[axis] = mn[axis].min(obj_min[axis] as f64);
                mx[axis] = mx[axis].max(obj_max[axis] as f64);
            }
            any = true;
        }

        any.then_some((mn, mx))
    }

    pub fn reset_section_box_to_visible_bounds(&mut self) -> bool {
        let Some((min, max)) = self.visible_bounds() else {
            return false;
        };
        self.section_box = SectionBox::new(min, max, self.section_box.active);
        true
    }

    pub fn set_section_box_bounds(&mut self, min: [f64; 3], max: [f64; 3]) {
        let (min, max) = sanitize_bounds(min, max);
        self.section_box.min = min;
        self.section_box.max = max;
    }

    pub fn resize_section_box_face(&mut self, axis: u8, positive: bool, delta: f64) -> bool {
        let axis = axis as usize;
        if axis >= 3 || !delta.is_finite() {
            return false;
        }
        const MIN_SPAN: f64 = 1e-6;
        if positive {
            self.section_box.max[axis] =
                (self.section_box.max[axis] + delta).max(self.section_box.min[axis] + MIN_SPAN);
        } else {
            self.section_box.min[axis] =
                (self.section_box.min[axis] + delta).min(self.section_box.max[axis] - MIN_SPAN);
        }
        true
    }

    /// Refresh picking data (edge_positions, vertex_positions) for all objects
    /// from their current model state. Call after transforms that modify geometry.
    pub fn refresh_picking_data(&mut self) {
        for obj in &mut self.objects {
            let (ep, eh) = collect_edge_data(&obj.model, obj.solid);
            let (vp, vh) = collect_vertex_data(&obj.model, obj.solid);
            obj.edge_positions = ep;
            obj.edge_handles = eh;
            obj.vertex_positions = vp;
            obj.vertex_handles = vh;
        }
    }

    /// Collect all visible vertices into a single buffer for GPU upload.
    /// Returns (combined_vertices, object_ranges) where each range maps
    /// object id to (start_vertex, vertex_count) in the combined buffer.
    pub fn build_combined_vertices(&self) -> (Vec<Vertex>, Vec<(ObjectId, u32, u32)>) {
        let mut combined = Vec::new();
        let mut ranges = Vec::new();
        let explosion_centroid = self.explosion_centroid();
        let explosion_factor = self.exploded_view.factor.max(0.0) as f32;
        for obj in self.visible_objects() {
            let start = combined.len() as u32;
            if explosion_factor <= 0.0 && !self.section_box.active {
                combined.extend_from_slice(&obj.vertices);
                ranges.push((obj.id, start, obj.vertices.len() as u32));
                continue;
            }
            let mut display_vertices = if explosion_factor > 0.0 {
                let offset = explosion_centroid
                    .map(|centroid| {
                        let center = object_center(obj);
                        [
                            (center[0] - centroid[0]) * explosion_factor,
                            (center[1] - centroid[1]) * explosion_factor,
                            (center[2] - centroid[2]) * explosion_factor,
                        ]
                    })
                    .unwrap_or([0.0; 3]);
                translate_vertices(&obj.vertices, offset)
            } else {
                obj.vertices.clone()
            };
            if self.section_box.active {
                display_vertices =
                    clip_vertices_to_section_box(&display_vertices, &self.section_box);
            }
            combined.extend_from_slice(&display_vertices);
            let count = display_vertices.len() as u32;
            ranges.push((obj.id, start, count));
        }
        (combined, ranges)
    }

    fn explosion_centroid(&self) -> Option<[f32; 3]> {
        if self.exploded_view.factor <= 0.0 {
            return None;
        }
        let mut sum = [0.0f32; 3];
        let mut count = 0.0f32;
        for obj in self.visible_objects() {
            if obj.vertices.is_empty() {
                continue;
            }
            let center = object_center(obj);
            for axis in 0..3 {
                sum[axis] += center[axis];
            }
            count += 1.0;
        }
        (count > 0.0).then(|| [sum[0] / count, sum[1] / count, sum[2] / count])
    }

    /// Total number of objects.
    pub fn len(&self) -> usize {
        self.objects.len()
    }

    /// Whether the scene is empty.
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    /// Select all visible objects.
    pub fn select_all(&mut self) {
        for obj in &mut self.objects {
            obj.selected = obj.visible;
        }
    }

    /// Deselect all objects.
    pub fn deselect_all(&mut self) {
        for obj in &mut self.objects {
            obj.selected = false;
        }
    }

    /// Select a single object (deselects others).
    pub fn select_single(&mut self, id: ObjectId) {
        for obj in &mut self.objects {
            obj.selected = obj.id == id;
        }
    }

    /// Toggle selection on a single object (for Ctrl+click multi-select).
    pub fn toggle_select(&mut self, id: ObjectId) {
        if let Some(obj) = self.get_mut(id) {
            obj.selected = !obj.selected;
        }
    }

    /// Get all selected objects.
    pub fn selected_objects(&self) -> Vec<&SceneObject> {
        self.objects.iter().filter(|o| o.selected).collect()
    }

    /// Get selected object ids.
    pub fn selected_ids(&self) -> Vec<ObjectId> {
        self.objects
            .iter()
            .filter(|o| o.selected)
            .map(|o| o.id)
            .collect()
    }

    /// Get an object by its ID.
    pub fn get_object(&self, id: ObjectId) -> Option<&SceneObject> {
        self.objects.iter().find(|o| o.id == id)
    }

    /// Get the currently selected object (first selected).
    pub fn selected_object(&self) -> Option<&SceneObject> {
        self.objects.iter().find(|o| o.selected)
    }

    /// Get the selected object id.
    pub fn selected_id(&self) -> Option<ObjectId> {
        self.selected_object().map(|o| o.id)
    }

    /// Move an object one position earlier in the list (feature tree "up").
    pub fn move_up(&mut self, id: ObjectId) {
        if let Some(idx) = self.objects.iter().position(|o| o.id == id) {
            if idx > 0 {
                self.objects.swap(idx, idx - 1);
            }
        }
    }

    /// Move an object one position later in the list (feature tree "down").
    pub fn move_down(&mut self, id: ObjectId) {
        if let Some(idx) = self.objects.iter().position(|o| o.id == id) {
            if idx + 1 < self.objects.len() {
                self.objects.swap(idx, idx + 1);
            }
        }
    }

    /// Get children of a given object (features inside a Body).
    pub fn children_of(&self, parent_id: ObjectId) -> Vec<&SceneObject> {
        self.objects
            .iter()
            .filter(|o| o.parent_id == Some(parent_id))
            .collect()
    }

    /// Get root objects (no parent).
    pub fn root_objects(&self) -> Vec<&SceneObject> {
        self.objects
            .iter()
            .filter(|o| o.parent_id.is_none())
            .collect()
    }

    /// Set the active body. Pass `None` to deactivate.
    pub fn set_active_body(&mut self, id: Option<ObjectId>) {
        self.active_body_id = id;
    }

    // -- Group management --

    /// Create a new group and return its id.
    pub fn create_group(&mut self, name: impl Into<String>) -> u32 {
        let id = self.next_group_id;
        self.next_group_id += 1;
        self.groups.push(ObjectGroup {
            id,
            name: name.into(),
            visible: true,
        });
        id
    }

    /// Add selected objects to a group.
    pub fn group_selected(&mut self, group_id: u32) {
        for obj in &mut self.objects {
            if obj.selected {
                obj.group_id = group_id;
            }
        }
    }

    /// Remove an object from its group.
    pub fn ungroup_object(&mut self, obj_id: ObjectId) {
        if let Some(obj) = self.get_mut(obj_id) {
            obj.group_id = 0;
        }
    }

    /// Toggle visibility for all objects in a group.
    pub fn toggle_group_visibility(&mut self, group_id: u32) {
        if let Some(g) = self.groups.iter_mut().find(|g| g.id == group_id) {
            g.visible = !g.visible;
            let vis = g.visible;
            for obj in &mut self.objects {
                if obj.group_id == group_id {
                    obj.visible = vis;
                }
            }
        }
    }

    /// Delete a group (ungroups its members, doesn't delete objects).
    pub fn delete_group(&mut self, group_id: u32) {
        for obj in &mut self.objects {
            if obj.group_id == group_id {
                obj.group_id = 0;
            }
        }
        self.groups.retain(|g| g.id != group_id);
    }

    /// Get objects belonging to a group.
    pub fn group_members(&self, group_id: u32) -> Vec<ObjectId> {
        self.objects
            .iter()
            .filter(|o| o.group_id == group_id)
            .map(|o| o.id)
            .collect()
    }

    /// Convert edge handles from one scene object into persistent API refs.
    pub fn selected_edges(
        &self,
        object_id: ObjectId,
        selected: &[Handle<EdgeData>],
    ) -> Vec<EdgeRef> {
        let Some(obj) = self.get(object_id) else {
            return Vec::new();
        };
        let Some(solid_id) = obj.solid_id else {
            return Vec::new();
        };

        let mut out = Vec::new();
        for &eh in selected {
            let Some(edge_data) = obj.model.edges.get(eh) else {
                continue;
            };
            let Some(tag) = edge_data.tag.as_ref() else {
                continue;
            };
            out.push(EdgeRef {
                solid: solid_id,
                tag: tag.clone(),
            });
        }
        out
    }

    /// Convert face handles from one scene object into persistent API refs.
    pub fn selected_faces(
        &self,
        object_id: ObjectId,
        selected: &[Handle<FaceData>],
    ) -> Vec<FaceRef> {
        let Some(obj) = self.get(object_id) else {
            return Vec::new();
        };
        let Some(solid_id) = obj.solid_id else {
            return Vec::new();
        };

        let mut out = Vec::new();
        for &fh in selected {
            let Some(face_data) = obj.model.faces.get(fh) else {
                continue;
            };
            let Some(tag) = face_data.tag.as_ref() else {
                continue;
            };
            out.push(FaceRef {
                solid: solid_id,
                tag: tag.clone(),
            });
        }
        out
    }

    /// Convert edge handles into persistent refs by searching scene objects.
    pub fn selected_edges_any_object(&self, selected: &[Handle<EdgeData>]) -> Vec<EdgeRef> {
        let mut out = Vec::new();
        for &eh in selected {
            let mut found = None;
            for obj in self
                .objects
                .iter()
                .filter(|obj| obj.selected)
                .chain(self.objects.iter().filter(|obj| !obj.selected))
            {
                let Some(solid_id) = obj.solid_id else {
                    continue;
                };
                let Some(edge_data) = obj.model.edges.get(eh) else {
                    continue;
                };
                let Some(tag) = edge_data.tag.as_ref() else {
                    continue;
                };
                found = Some(EdgeRef {
                    solid: solid_id,
                    tag: tag.clone(),
                });
                break;
            }
            if let Some(edge_ref) = found {
                out.push(edge_ref);
            }
        }
        out
    }

    /// Convert face handles into persistent refs by searching scene objects.
    pub fn selected_faces_any_object(&self, selected: &[Handle<FaceData>]) -> Vec<FaceRef> {
        let mut out = Vec::new();
        for &fh in selected {
            let mut found = None;
            for obj in self
                .objects
                .iter()
                .filter(|obj| obj.selected)
                .chain(self.objects.iter().filter(|obj| !obj.selected))
            {
                let Some(solid_id) = obj.solid_id else {
                    continue;
                };
                let Some(face_data) = obj.model.faces.get(fh) else {
                    continue;
                };
                let Some(tag) = face_data.tag.as_ref() else {
                    continue;
                };
                found = Some(FaceRef {
                    solid: solid_id,
                    tag: tag.clone(),
                });
                break;
            }
            if let Some(face_ref) = found {
                out.push(face_ref);
            }
        }
        out
    }
}

/// Collect all unique edges from a solid, returning endpoint positions and handles.
#[allow(clippy::type_complexity)]
fn collect_edge_data(
    model: &BRepModel,
    solid: Handle<SolidData>,
) -> (Vec<([f32; 3], [f32; 3])>, Vec<Handle<EdgeData>>) {
    let mut positions = Vec::new();
    let mut handles = Vec::new();
    let mut seen = HashSet::new();

    let Some(sd) = model.solids.get(solid) else {
        return (positions, handles);
    };
    for &sh in &sd.shells {
        let Some(shell) = model.shells.get(sh) else {
            continue;
        };
        for &fh in &shell.faces {
            let Some(face) = model.faces.get(fh) else {
                continue;
            };
            for loop_h in std::iter::once(face.outer_loop).chain(face.inner_loops.iter().copied()) {
                let hes = model.loop_half_edges(
                    model
                        .loops
                        .get(loop_h)
                        .map_or(Handle::from_raw_parts(0, 0), |l| l.half_edge),
                );
                for heh in hes {
                    let Some(he) = model.half_edges.get(heh) else {
                        continue;
                    };
                    let Some(eh) = he.edge else { continue };
                    if !seen.insert(eh) {
                        continue;
                    }
                    let Some(ed) = model.edges.get(eh) else {
                        continue;
                    };
                    let Some(sv) = model.vertices.get(ed.start) else {
                        continue;
                    };
                    let Some(ev) = model.vertices.get(ed.end) else {
                        continue;
                    };
                    let sp = [sv.point.x as f32, sv.point.y as f32, sv.point.z as f32];
                    let ep = [ev.point.x as f32, ev.point.y as f32, ev.point.z as f32];
                    positions.push((sp, ep));
                    handles.push(eh);
                }
            }
        }
    }
    (positions, handles)
}

/// Collect all unique vertices from a solid, returning positions and handles.
fn collect_vertex_data(
    model: &BRepModel,
    solid: Handle<SolidData>,
) -> (Vec<[f32; 3]>, Vec<Handle<VertexData>>) {
    let mut positions = Vec::new();
    let mut handles = Vec::new();
    let mut seen = HashSet::new();

    let Some(sd) = model.solids.get(solid) else {
        return (positions, handles);
    };
    for &sh in &sd.shells {
        let Some(shell) = model.shells.get(sh) else {
            continue;
        };
        for &fh in &shell.faces {
            let Some(face) = model.faces.get(fh) else {
                continue;
            };
            for loop_h in std::iter::once(face.outer_loop).chain(face.inner_loops.iter().copied()) {
                let hes = model.loop_half_edges(
                    model
                        .loops
                        .get(loop_h)
                        .map_or(Handle::from_raw_parts(0, 0), |l| l.half_edge),
                );
                for heh in hes {
                    let Some(he) = model.half_edges.get(heh) else {
                        continue;
                    };
                    if !seen.insert(he.origin) {
                        continue;
                    }
                    let Some(vd) = model.vertices.get(he.origin) else {
                        continue;
                    };
                    positions.push([vd.point.x as f32, vd.point.y as f32, vd.point.z as f32]);
                    handles.push(he.origin);
                }
            }
        }
    }
    (positions, handles)
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

fn sanitize_bounds(mut min: [f64; 3], mut max: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    const MIN_SPAN: f64 = 1e-6;
    for axis in 0..3 {
        if !min[axis].is_finite() {
            min[axis] = 0.0;
        }
        if !max[axis].is_finite() {
            max[axis] = min[axis] + 1.0;
        }
        if min[axis] > max[axis] {
            std::mem::swap(&mut min[axis], &mut max[axis]);
        }
        if max[axis] - min[axis] < MIN_SPAN {
            let mid = (min[axis] + max[axis]) * 0.5;
            min[axis] = mid - MIN_SPAN * 0.5;
            max[axis] = mid + MIN_SPAN * 0.5;
        }
    }
    (min, max)
}

fn object_center(obj: &SceneObject) -> [f32; 3] {
    if obj.vertices.is_empty() {
        return [0.0; 3];
    }
    let (mn, mx) = compute_aabb(&obj.vertices);
    [
        (mn[0] + mx[0]) * 0.5,
        (mn[1] + mx[1]) * 0.5,
        (mn[2] + mx[2]) * 0.5,
    ]
}

fn translate_vertices(vertices: &[Vertex], offset: [f32; 3]) -> Vec<Vertex> {
    vertices
        .iter()
        .map(|v| Vertex {
            position: [
                v.position[0] + offset[0],
                v.position[1] + offset[1],
                v.position[2] + offset[2],
            ],
            normal: v.normal,
        })
        .collect()
}

fn clip_vertices_to_section_box(vertices: &[Vertex], section: &SectionBox) -> Vec<Vertex> {
    let mut out = Vec::new();
    for tri in vertices.chunks_exact(3) {
        let mut poly = tri.to_vec();
        for axis in 0..3 {
            poly = clip_polygon_axis(&poly, axis, section.min[axis] as f32, true);
            if poly.len() < 3 {
                break;
            }
            poly = clip_polygon_axis(&poly, axis, section.max[axis] as f32, false);
            if poly.len() < 3 {
                break;
            }
        }
        if poly.len() < 3 {
            continue;
        }
        for i in 1..poly.len() - 1 {
            out.push(poly[0]);
            out.push(poly[i]);
            out.push(poly[i + 1]);
        }
    }
    out
}

fn clip_polygon_axis(poly: &[Vertex], axis: usize, plane: f32, keep_greater: bool) -> Vec<Vertex> {
    let mut out = Vec::new();
    let Some(mut prev) = poly.last().copied() else {
        return out;
    };
    let mut prev_inside = vertex_inside_plane(prev, axis, plane, keep_greater);
    for &curr in poly {
        let curr_inside = vertex_inside_plane(curr, axis, plane, keep_greater);
        match (prev_inside, curr_inside) {
            (true, true) => out.push(curr),
            (true, false) => out.push(intersect_vertex_plane(prev, curr, axis, plane)),
            (false, true) => {
                out.push(intersect_vertex_plane(prev, curr, axis, plane));
                out.push(curr);
            }
            (false, false) => {}
        }
        prev = curr;
        prev_inside = curr_inside;
    }
    out
}

fn vertex_inside_plane(v: Vertex, axis: usize, plane: f32, keep_greater: bool) -> bool {
    if keep_greater {
        v.position[axis] >= plane - 1e-6
    } else {
        v.position[axis] <= plane + 1e-6
    }
}

fn intersect_vertex_plane(a: Vertex, b: Vertex, axis: usize, plane: f32) -> Vertex {
    let denom = b.position[axis] - a.position[axis];
    let t = if denom.abs() < 1e-8 {
        0.0
    } else {
        ((plane - a.position[axis]) / denom).clamp(0.0, 1.0)
    };
    let mut position = [0.0; 3];
    let mut normal = [0.0; 3];
    for i in 0..3 {
        position[i] = a.position[i] + (b.position[i] - a.position[i]) * t;
        normal[i] = a.normal[i] + (b.normal[i] - a.normal[i]) * t;
    }
    Vertex {
        position,
        normal: normalize_normal(normal),
    }
}

fn normalize_normal(n: [f32; 3]) -> [f32; 3] {
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if len > 1e-8 {
        [n[0] / len, n[1] / len, n[2] / len]
    } else {
        [0.0, 0.0, 1.0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_math::Point3;
    use cadkernel_modeling::make_box;

    #[test]
    fn test_add_remove_object() {
        let mut scene = Scene::new();
        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let id = scene.add_object("Box", model, r.solid, None, None);
        assert_eq!(scene.len(), 1);
        assert!(scene.remove_object(id));
        assert!(scene.is_empty());
    }

    #[test]
    fn test_visibility_toggle() {
        let mut scene = Scene::new();
        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let id = scene.add_object("Box", model, r.solid, None, None);
        assert_eq!(scene.visible_objects().count(), 1);
        scene.get_mut(id).unwrap().visible = false;
        assert_eq!(scene.visible_objects().count(), 0);
    }

    #[test]
    fn test_selection() {
        let mut scene = Scene::new();
        let mut m1 = BRepModel::new();
        let r1 = make_box(&mut m1, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let id1 = scene.add_object("Box1", m1, r1.solid, None, None);
        let mut m2 = BRepModel::new();
        let r2 = make_box(&mut m2, Point3::new(5.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap();
        let _id2 = scene.add_object("Box2", m2, r2.solid, None, None);
        scene.select_single(id1);
        assert_eq!(scene.selected_id(), Some(id1));
    }

    #[test]
    fn test_combined_vertices() {
        let mut scene = Scene::new();
        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        scene.add_object("Box", model, r.solid, None, None);
        let (verts, ranges) = scene.build_combined_vertices();
        assert!(!verts.is_empty());
        assert_eq!(ranges.len(), 1);
    }

    #[test]
    fn test_default_colors_rotate() {
        let mut scene = Scene::new();
        for i in 0..10 {
            let mut model = BRepModel::new();
            let r = make_box(
                &mut model,
                Point3::new(i as f64 * 3.0, 0.0, 0.0),
                1.0,
                1.0,
                1.0,
            )
            .unwrap();
            scene.add_object(format!("Box{i}"), model, r.solid, None, None);
        }
        // Colors should rotate through the palette
        let c0 = scene.objects[0].color;
        let c8 = scene.objects[8].color;
        assert_eq!(c0, c8); // palette length is 8, so 0 and 8 match
    }

    #[test]
    fn test_hierarchy_root_and_children() {
        let mut scene = Scene::new();
        let mut m1 = BRepModel::new();
        let r1 = make_box(&mut m1, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let body_id = scene.add_object("Body", m1, r1.solid, None, None);
        scene.get_mut(body_id).unwrap().is_body = true;

        let mut m2 = BRepModel::new();
        let r2 = make_box(&mut m2, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let child_id = scene.add_object("Pad", m2, r2.solid, None, None);
        scene.get_mut(child_id).unwrap().parent_id = Some(body_id);

        let mut m3 = BRepModel::new();
        let r3 = make_box(&mut m3, Point3::new(3.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap();
        let _standalone = scene.add_object("StandaloneBox", m3, r3.solid, None, None);

        assert_eq!(scene.root_objects().len(), 2);
        assert_eq!(scene.children_of(body_id).len(), 1);
        assert_eq!(scene.children_of(body_id)[0].id, child_id);
    }

    #[test]
    fn test_active_body() {
        let mut scene = Scene::new();
        let mut m1 = BRepModel::new();
        let r1 = make_box(&mut m1, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let body_id = scene.add_object("Body", m1, r1.solid, None, None);
        scene.get_mut(body_id).unwrap().is_body = true;

        assert!(scene.active_body_id.is_none());
        scene.set_active_body(Some(body_id));
        assert_eq!(scene.active_body_id, Some(body_id));
        scene.set_active_body(None);
        assert!(scene.active_body_id.is_none());
    }

    #[test]
    fn test_status_flags_default() {
        let mut scene = Scene::new();
        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let id = scene.add_object("Box", model, r.solid, None, None);
        let obj = scene.get(id).unwrap();
        assert!(!obj.is_body);
        assert!(!obj.is_tip);
        assert!(!obj.suppressed);
        assert!(!obj.has_error);
        assert!(!obj.needs_recompute);
        assert!(obj.parent_id.is_none());
        assert!(obj.solid_id.is_none());
    }

    #[test]
    fn test_solid_id_round_trip() {
        let mut scene = Scene::new();
        let mut m1 = BRepModel::new();
        let r1 = make_box(&mut m1, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let tracked = scene.add_object("Tracked", m1, r1.solid, None, Some(SolidId(7)));
        let mut m2 = BRepModel::new();
        let r2 = make_box(&mut m2, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let legacy = scene.add_object("Legacy", m2, r2.solid, None, None);
        assert_eq!(scene.get(tracked).unwrap().solid_id, Some(SolidId(7)));
        assert_eq!(scene.get(legacy).unwrap().solid_id, None);
    }
}
