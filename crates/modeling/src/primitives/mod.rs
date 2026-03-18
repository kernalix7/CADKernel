pub mod box_shape;
pub mod cone_shape;
pub mod cylinder_shape;
pub mod ellipsoid_shape;
pub mod helix_shape;
pub mod plane_face_shape;
pub mod polygon_shape;
pub mod prism_shape;
pub mod sphere_shape;
pub mod spiral_shape;
pub mod torus_shape;
pub mod tube_shape;
pub mod wedge_shape;

pub use box_shape::{BoxResult, make_box};
pub use cone_shape::{ConeResult, make_cone};
pub use cylinder_shape::{CylinderResult, make_cylinder};
pub use ellipsoid_shape::{EllipsoidResult, make_ellipsoid};
pub use helix_shape::{HelixResult, make_helix};
pub use plane_face_shape::{PlaneFaceResult, make_plane_face};
pub use polygon_shape::{PolygonResult, make_polygon};
pub use prism_shape::{PrismResult, make_prism};
pub use sphere_shape::{SphereResult, make_sphere};
pub use spiral_shape::{SpiralResult, make_spiral};
pub use torus_shape::{TorusResult, make_torus};
pub use tube_shape::{TubeResult, make_tube};
pub use wedge_shape::{WedgeResult, make_wedge};

use std::collections::HashMap;
use std::sync::Arc;

use cadkernel_geometry::LineSegment;
use cadkernel_topology::{
    BRepModel, EdgeData, EntityKind, HalfEdgeData, Handle, OperationId, Tag, VertexData,
};

/// Edge deduplication cache for primitive construction.
///
/// Tracks edges by their vertex endpoint indices. When an edge between two
/// vertices already exists, returns the twin half-edge instead of creating a
/// duplicate topological edge.
type EdgeEntry = (Handle<EdgeData>, Handle<HalfEdgeData>, Handle<HalfEdgeData>);

pub(crate) struct EdgeCache {
    /// Maps `(v_start_index, v_end_index)` → `(edge, he_forward, he_backward)`.
    map: HashMap<(u32, u32), EdgeEntry>,
}

impl EdgeCache {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Returns the appropriate half-edge for the directed edge from `v_start` to
    /// `v_end`. If the edge already exists (in either direction), returns the
    /// matching twin half-edge. Otherwise creates a new edge.
    pub fn get_or_create(
        &mut self,
        model: &mut BRepModel,
        v_start: Handle<VertexData>,
        v_end: Handle<VertexData>,
        tag: Tag,
    ) -> Handle<HalfEdgeData> {
        let key_fwd = (v_start.index(), v_end.index());
        let key_rev = (v_end.index(), v_start.index());

        // Reverse direction edge exists — return the twin half-edge.
        if let Some(&(_, _he_fwd, he_bwd)) = self.map.get(&key_rev) {
            return he_bwd;
        }

        // Create a new edge and cache it.
        let (edge_h, he_fwd, he_bwd) = model.add_edge_tagged(v_start, v_end, tag);
        self.map.insert(key_fwd, (edge_h, he_fwd, he_bwd));
        he_fwd
    }

    /// Returns all unique edge handles created by this cache.
    pub fn all_edges(&self) -> Vec<Handle<EdgeData>> {
        self.map.values().map(|&(e, _, _)| e).collect()
    }

    /// Returns the number of unique edges created.
    #[allow(dead_code)]
    pub fn edge_count(&self) -> usize {
        self.map.len()
    }
}

/// Helper to create an edge tag with the given operation and a mutable edge
/// index counter.
pub(crate) fn next_edge_tag(op: OperationId, edge_idx: &mut u32) -> Tag {
    let tag = Tag::generated(EntityKind::Edge, op, *edge_idx);
    *edge_idx += 1;
    tag
}

/// Binds `LineSegment` curves to all edges tracked by the cache.
pub(crate) fn bind_edge_line_segments(model: &mut BRepModel, ec: &EdgeCache) {
    let bindings: Vec<_> = ec
        .all_edges()
        .into_iter()
        .filter_map(|edge_h| {
            let ed = model.edges.get(edge_h)?;
            let he_a = model.half_edges.get(ed.half_edge_a?)?;
            let he_b = model.half_edges.get(ed.half_edge_b?)?;
            let p0 = model.vertices.get(he_a.origin)?.point;
            let p1 = model.vertices.get(he_b.origin)?.point;
            Some((edge_h, p0, p1))
        })
        .collect();
    for (eh, p0, p1) in bindings {
        model.bind_edge_curve(eh, Arc::new(LineSegment::new(p0, p1)), (0.0, 1.0));
    }
}
