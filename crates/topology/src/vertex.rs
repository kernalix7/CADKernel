use cadkernel_math::Point3;
use serde::{Deserialize, Serialize};

use crate::halfedge::HalfEdgeData;
use crate::handle::Handle;
use crate::naming::Tag;

/// A topological vertex: a point in 3D space with an optional reference to
/// one of its outgoing half-edges (for traversal).
///
/// The `half_edge` link is an entry point for iterating over all edges and
/// faces incident to this vertex. The optional `tag` provides persistent
/// naming for parametric model rebuilds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexData {
    /// Position of this vertex in 3D space.
    pub point: Point3,
    /// One outgoing half-edge originating from this vertex (traversal entry point).
    pub half_edge: Option<Handle<HalfEdgeData>>,
    /// Persistent name for this vertex.
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
