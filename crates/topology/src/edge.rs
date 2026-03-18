#[cfg(feature = "geometry-binding")]
use std::sync::Arc;

#[cfg(feature = "geometry-binding")]
use cadkernel_geometry::Curve;
#[cfg(feature = "geometry-binding")]
use cadkernel_geometry::curve::curve2d::Curve2D;
use serde::{Deserialize, Serialize};

use crate::halfedge::HalfEdgeData;
use crate::handle::Handle;
use crate::naming::Tag;
use crate::vertex::VertexData;

/// A topological edge connecting two vertices.
/// It owns two half-edges (one for each direction).
///
/// When the `geometry-binding` feature is enabled (default), the edge can
/// carry the underlying 3D curve, parameter domain, and UV pcurves for
/// adjacent faces.
#[derive(Clone, Serialize, Deserialize)]
pub struct EdgeData {
    pub start: Handle<VertexData>,
    pub end: Handle<VertexData>,
    pub half_edge_a: Option<Handle<HalfEdgeData>>,
    pub half_edge_b: Option<Handle<HalfEdgeData>>,
    pub tag: Option<Tag>,
    /// The 3D curve this edge lies on (not serialized).
    #[cfg(feature = "geometry-binding")]
    #[serde(skip)]
    pub curve: Option<Arc<dyn Curve + Send + Sync>>,
    /// Parameter range on the curve: `(t_start, t_end)`.
    #[cfg(feature = "geometry-binding")]
    pub curve_domain: Option<(f64, f64)>,
    /// UV pcurve on the left face (not serialized).
    #[cfg(feature = "geometry-binding")]
    #[serde(skip)]
    pub pcurve_left: Option<Arc<dyn Curve2D>>,
    /// UV pcurve on the right face (not serialized).
    #[cfg(feature = "geometry-binding")]
    #[serde(skip)]
    pub pcurve_right: Option<Arc<dyn Curve2D>>,
}

impl std::fmt::Debug for EdgeData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("EdgeData");
        s.field("start", &self.start)
            .field("end", &self.end)
            .field("half_edge_a", &self.half_edge_a)
            .field("half_edge_b", &self.half_edge_b)
            .field("tag", &self.tag);
        #[cfg(feature = "geometry-binding")]
        s.field("has_curve", &self.curve.is_some())
            .field("curve_domain", &self.curve_domain)
            .field("has_pcurve_left", &self.pcurve_left.is_some())
            .field("has_pcurve_right", &self.pcurve_right.is_some());
        s.finish()
    }
}

impl EdgeData {
    /// Creates an edge between two vertex handles with no half-edges or curve bound.
    pub fn new(start: Handle<VertexData>, end: Handle<VertexData>) -> Self {
        Self {
            start,
            end,
            half_edge_a: None,
            half_edge_b: None,
            tag: None,
            #[cfg(feature = "geometry-binding")]
            curve: None,
            #[cfg(feature = "geometry-binding")]
            curve_domain: None,
            #[cfg(feature = "geometry-binding")]
            pcurve_left: None,
            #[cfg(feature = "geometry-binding")]
            pcurve_right: None,
        }
    }
}
