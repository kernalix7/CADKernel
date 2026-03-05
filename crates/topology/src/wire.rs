use serde::{Deserialize, Serialize};

use crate::halfedge::HalfEdgeData;
use crate::handle::Handle;
use crate::naming::Tag;

/// An ordered chain of half-edges that may be open or closed.
///
/// Unlike [`LoopData`](super::LoopData), a wire does not necessarily belong
/// to a face and may represent a standalone path (e.g. a sketch profile
/// before it is used to build a face).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireData {
    /// Ordered list of half-edges forming the chain.
    pub half_edges: Vec<Handle<HalfEdgeData>>,
    /// Whether the first and last half-edges form a cycle.
    pub is_closed: bool,
    pub tag: Option<Tag>,
}

impl WireData {
    /// Creates a wire from an ordered list of half-edges.
    pub fn new(half_edges: Vec<Handle<HalfEdgeData>>, is_closed: bool) -> Self {
        Self {
            half_edges,
            is_closed,
            tag: None,
        }
    }

    /// Number of half-edges in this wire.
    pub fn len(&self) -> usize {
        self.half_edges.len()
    }

    /// Returns `true` if the wire contains no half-edges.
    pub fn is_empty(&self) -> bool {
        self.half_edges.is_empty()
    }
}
