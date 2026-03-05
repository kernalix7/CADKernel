use serde::{Deserialize, Serialize};

use crate::edge::EdgeData;
use crate::handle::Handle;
use crate::loop_wire::LoopData;
use crate::naming::Tag;
use crate::vertex::VertexData;

/// A directed half-edge in the half-edge data structure.
///
/// Half-edges come in twin pairs sharing the same parent [`EdgeData`].
/// Within a face boundary ([`LoopData`]), they form a linked cycle via `next` / `prev`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HalfEdgeData {
    /// The vertex this half-edge originates from.
    pub origin: Handle<VertexData>,
    /// The twin half-edge (opposite direction on the same edge).
    pub twin: Option<Handle<HalfEdgeData>>,
    /// Next half-edge in the loop.
    pub next: Option<Handle<HalfEdgeData>>,
    /// Previous half-edge in the loop.
    pub prev: Option<Handle<HalfEdgeData>>,
    /// Parent edge.
    pub edge: Option<Handle<EdgeData>>,
    /// The loop (face boundary) this half-edge belongs to.
    pub loop_ref: Option<Handle<LoopData>>,
    pub tag: Option<Tag>,
}

impl HalfEdgeData {
    /// Creates a half-edge originating from `origin` with all link fields set to `None`.
    pub fn new(origin: Handle<VertexData>) -> Self {
        Self {
            origin,
            twin: None,
            next: None,
            prev: None,
            edge: None,
            loop_ref: None,
            tag: None,
        }
    }
}
