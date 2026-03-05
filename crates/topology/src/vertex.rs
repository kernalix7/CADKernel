use cadkernel_math::Point3;
use serde::{Deserialize, Serialize};

use crate::halfedge::HalfEdgeData;
use crate::handle::Handle;
use crate::naming::Tag;

/// A topological vertex: a point in space with an optional reference to
/// one of its outgoing half-edges (for traversal).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexData {
    pub point: Point3,
    pub half_edge: Option<Handle<HalfEdgeData>>,
    pub tag: Option<Tag>,
}

impl VertexData {
    /// Creates a vertex at the given position with no outgoing half-edge or tag.
    pub fn new(point: Point3) -> Self {
        Self {
            point,
            half_edge: None,
            tag: None,
        }
    }
}
