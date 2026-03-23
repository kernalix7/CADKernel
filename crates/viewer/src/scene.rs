//! Multi-object scene management for the CAD viewer.
//!
//! Each `SceneObject` owns its own BRepModel, mesh, and GPU-ready vertex data.
//! The `Scene` holds all objects and provides methods for adding, removing,
//! toggling visibility, and iterating visible objects for rendering.

use cadkernel_io::{Mesh, tessellate_solid};
use cadkernel_topology::{BRepModel, Handle, SolidData};

use crate::render::{Vertex, mesh_to_vertices};

/// Unique object identifier within a scene.
pub type ObjectId = u32;

/// Parameters used to create a scene object (for parametric editing).
#[derive(Clone, Debug)]
pub enum CreationParams {
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

/// Multi-object scene.
#[derive(Clone)]
pub struct Scene {
    pub objects: Vec<SceneObject>,
    next_id: ObjectId,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            next_id: 1,
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
        let mesh = tessellate_solid(&model, solid);
        let vertices = mesh_to_vertices(&mesh);
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

    /// Get the currently selected object (first selected).
    pub fn selected_object(&self) -> Option<&SceneObject> {
        self.objects.iter().find(|o| o.selected)
    }

    /// Get the selected object id.
    pub fn selected_id(&self) -> Option<ObjectId> {
        self.selected_object().map(|o| o.id)
    }
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
}
