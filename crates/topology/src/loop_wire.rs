use serde::{Deserialize, Serialize};

use crate::face::FaceData;
use crate::halfedge::HalfEdgeData;
use crate::handle::Handle;
use crate::naming::Tag;

/// A closed loop of half-edges forming a face boundary.
///
/// A face has exactly one outer loop and zero or more inner loops (holes).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopData {
    /// Any half-edge on this loop (entry point for traversal).
    pub half_edge: Handle<HalfEdgeData>,
    /// The face this loop belongs to.
    pub face: Option<Handle<FaceData>>,
    pub tag: Option<Tag>,
}

impl LoopData {
    /// Creates a loop starting at the given half-edge, not yet attached to a face.
    pub fn new(half_edge: Handle<HalfEdgeData>) -> Self {
        Self {
            half_edge,
            face: None,
            tag: None,
        }
    }
}
