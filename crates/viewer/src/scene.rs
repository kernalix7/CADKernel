//! Multi-object scene management for the CAD viewer.
//!
//! Each `SceneObject` owns its own BRepModel, mesh, and GPU-ready vertex data.
//! The `Scene` holds all objects and provides methods for adding, removing,
//! toggling visibility, and iterating visible objects for rendering.

use cadkernel_io::{Mesh, tessellate_solid_with_face_map};
use cadkernel_topology::{BRepModel, Handle, SolidData, FaceData, EdgeData, VertexData};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::render::{Vertex, mesh_to_vertices};

/// Unique object identifier within a scene.
pub type ObjectId = u32;

/// Parameters used to create a scene object (for parametric editing).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CreationParams {
    // Primitives
    Box { width: f64, height: f64, depth: f64 },
    Cylinder { radius: f64, height: f64 },
    Sphere { radius: f64 },
    Cone { base_radius: f64, top_radius: f64, height: f64 },
    Torus { major_radius: f64, minor_radius: f64 },
    Tube { outer_radius: f64, inner_radius: f64, height: f64 },
    Prism { radius: f64, height: f64, sides: usize },
    Wedge { dx: f64, dy: f64, dz: f64, dx2: f64, dy2: f64 },
    Ellipsoid { rx: f64, ry: f64, rz: f64 },
    Helix { radius: f64, pitch: f64, turns: f64, tube_radius: f64 },
    Imported { path: String },
    Extruded,
    Revolved,
    Boolean { op: String },
    // PartDesign features
    Fillet { radius: f64 },
    Chamfer { distance: f64 },
    Shell { thickness: f64 },
    Mirror { plane: u8 },
    Pattern { count: usize, spacing: f64, axis: u8 },
    Groove { angle: f64 },
    Sprocket { teeth: u32, roller_diameter: f64, pitch: f64, bore: f64 },
    InvoluteGear { teeth: u32, module_val: f64, pressure_angle: f64 },
    // Draft
    DraftLine { length: f64, angle: f64 },
    DraftCircle { radius: f64 },
    DraftRectangle { width: f64, height: f64 },
    DraftPolygon { radius: f64, sides: usize },
    DraftArc { radius: f64, start_angle: f64, end_angle: f64 },
    DraftEllipse { rx: f64, ry: f64 },
    // Surface
    SurfacePipe { radius: f64, length: f64 },
    SurfaceRuled { width: f64, depth: f64, offset: f64 },
    // Boolean with tool
    BooleanOp { op_type: u8, width: f64, height: f64, depth: f64, offset_x: f64, offset_y: f64, offset_z: f64 },
    // Scale
    ScaleOp { factor: f64 },
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
}

pub fn compute_aabb(vertices: &[Vertex]) -> ([f32; 3], [f32; 3]) {
    if vertices.is_empty() {
        return ([f32::MIN; 3], [f32::MAX; 3]);
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

/// Multi-object scene.
#[derive(Clone)]
pub struct Scene {
    pub objects: Vec<SceneObject>,
    next_id: ObjectId,
    pub active_body_id: Option<ObjectId>,
    pub groups: Vec<ObjectGroup>,
    next_group_id: u32,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            next_id: 1,
            active_body_id: None,
            groups: Vec::new(),
            next_group_id: 1,
        }
    }

    /// Add a new object to the scene. Returns its ObjectId.
    pub fn add_object(
        &mut self,
        name: impl Into<String>,
        model: BRepModel,
        solid: Handle<SolidData>,
        params: Option<CreationParams>,
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
        });
        id
    }

    /// Add object from a pre-tessellated mesh (for imported files).
    pub fn add_mesh_object(
        &mut self,
        name: impl Into<String>,
        mesh: Mesh,
        params: Option<CreationParams>,
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
        for obj in self.visible_objects() {
            let start = combined.len() as u32;
            combined.extend_from_slice(&obj.vertices);
            let count = obj.vertices.len() as u32;
            ranges.push((obj.id, start, count));
        }
        (combined, ranges)
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
        self.objects.iter().filter(|o| o.selected).map(|o| o.id).collect()
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
        self.objects.iter().filter(|o| o.parent_id.is_none()).collect()
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
        self.objects.iter().filter(|o| o.group_id == group_id).map(|o| o.id).collect()
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
        let Some(shell) = model.shells.get(sh) else { continue };
        for &fh in &shell.faces {
            let Some(face) = model.faces.get(fh) else { continue };
            let loop_h = face.outer_loop;
            let hes = model.loop_half_edges(model.loops.get(loop_h).map_or(
                Handle::from_raw_parts(0, 0),
                |l| l.half_edge,
            ));
            for heh in hes {
                let Some(he) = model.half_edges.get(heh) else { continue };
                let Some(eh) = he.edge else { continue };
                if !seen.insert(eh) { continue; }
                let Some(ed) = model.edges.get(eh) else { continue };
                let Some(sv) = model.vertices.get(ed.start) else { continue };
                let Some(ev) = model.vertices.get(ed.end) else { continue };
                let sp = [sv.point.x as f32, sv.point.y as f32, sv.point.z as f32];
                let ep = [ev.point.x as f32, ev.point.y as f32, ev.point.z as f32];
                positions.push((sp, ep));
                handles.push(eh);
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
        let Some(shell) = model.shells.get(sh) else { continue };
        for &fh in &shell.faces {
            let Some(face) = model.faces.get(fh) else { continue };
            let loop_h = face.outer_loop;
            let hes = model.loop_half_edges(model.loops.get(loop_h).map_or(
                Handle::from_raw_parts(0, 0),
                |l| l.half_edge,
            ));
            for heh in hes {
                let Some(he) = model.half_edges.get(heh) else { continue };
                if !seen.insert(he.origin) { continue; }
                let Some(vd) = model.vertices.get(he.origin) else { continue };
                positions.push([vd.point.x as f32, vd.point.y as f32, vd.point.z as f32]);
                handles.push(he.origin);
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
        let id = scene.add_object("Box", model, r.solid, None);
        assert_eq!(scene.len(), 1);
        assert!(scene.remove_object(id));
        assert!(scene.is_empty());
    }

    #[test]
    fn test_visibility_toggle() {
        let mut scene = Scene::new();
        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let id = scene.add_object("Box", model, r.solid, None);
        assert_eq!(scene.visible_objects().count(), 1);
        scene.get_mut(id).unwrap().visible = false;
        assert_eq!(scene.visible_objects().count(), 0);
    }

    #[test]
    fn test_selection() {
        let mut scene = Scene::new();
        let mut m1 = BRepModel::new();
        let r1 = make_box(&mut m1, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let id1 = scene.add_object("Box1", m1, r1.solid, None);
        let mut m2 = BRepModel::new();
        let r2 = make_box(&mut m2, Point3::new(5.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap();
        let _id2 = scene.add_object("Box2", m2, r2.solid, None);
        scene.select_single(id1);
        assert_eq!(scene.selected_id(), Some(id1));
    }

    #[test]
    fn test_combined_vertices() {
        let mut scene = Scene::new();
        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        scene.add_object("Box", model, r.solid, None);
        let (verts, ranges) = scene.build_combined_vertices();
        assert!(!verts.is_empty());
        assert_eq!(ranges.len(), 1);
    }

    #[test]
    fn test_default_colors_rotate() {
        let mut scene = Scene::new();
        for i in 0..10 {
            let mut model = BRepModel::new();
            let r = make_box(&mut model, Point3::new(i as f64 * 3.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap();
            scene.add_object(format!("Box{i}"), model, r.solid, None);
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
        let body_id = scene.add_object("Body", m1, r1.solid, None);
        scene.get_mut(body_id).unwrap().is_body = true;

        let mut m2 = BRepModel::new();
        let r2 = make_box(&mut m2, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let child_id = scene.add_object("Pad", m2, r2.solid, None);
        scene.get_mut(child_id).unwrap().parent_id = Some(body_id);

        let mut m3 = BRepModel::new();
        let r3 = make_box(&mut m3, Point3::new(3.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap();
        let _standalone = scene.add_object("StandaloneBox", m3, r3.solid, None);

        assert_eq!(scene.root_objects().len(), 2);
        assert_eq!(scene.children_of(body_id).len(), 1);
        assert_eq!(scene.children_of(body_id)[0].id, child_id);
    }

    #[test]
    fn test_active_body() {
        let mut scene = Scene::new();
        let mut m1 = BRepModel::new();
        let r1 = make_box(&mut m1, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let body_id = scene.add_object("Body", m1, r1.solid, None);
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
        let id = scene.add_object("Box", model, r.solid, None);
        let obj = scene.get(id).unwrap();
        assert!(!obj.is_body);
        assert!(!obj.is_tip);
        assert!(!obj.suppressed);
        assert!(!obj.has_error);
        assert!(!obj.needs_recompute);
        assert!(obj.parent_id.is_none());
    }
}
