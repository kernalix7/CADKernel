#[cfg(feature = "geometry-binding")]
use std::sync::Arc;

#[cfg(feature = "geometry-binding")]
use cadkernel_geometry::Surface;
use serde::{Deserialize, Serialize};

use crate::handle::Handle;
use crate::loop_wire::LoopData;
use crate::naming::Tag;
use crate::shell::ShellData;

/// Whether the face normal agrees with the surface normal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Orientation {
    Forward,
    Reversed,
}

/// A topological face bounded by one outer loop and zero or more inner loops (holes).
///
/// When `geometry-binding` is enabled (default), the face can carry the
/// underlying surface, orientation flag, and UV trim boundaries.
#[derive(Clone, Serialize, Deserialize)]
pub struct FaceData {
    /// The outer boundary loop.
    pub outer_loop: Handle<LoopData>,
    /// Inner loops (holes within this face).
    pub inner_loops: Vec<Handle<LoopData>>,
    /// The shell this face belongs to.
    pub shell: Option<Handle<ShellData>>,
    pub tag: Option<Tag>,
    /// The underlying surface geometry (not serialized).
    #[cfg(feature = "geometry-binding")]
    #[serde(skip)]
    pub surface: Option<Arc<dyn Surface + Send + Sync>>,
    /// Whether the face normal agrees with the surface normal.
    pub orientation: Orientation,
    /// Outer trim loop in UV parameter space (not serialized).
    #[cfg(feature = "geometry-binding")]
    #[serde(skip)]
    pub outer_trim: Option<cadkernel_geometry::ParametricWire2D>,
    /// Inner trim loops (holes) in UV parameter space (not serialized).
    #[cfg(feature = "geometry-binding")]
    #[serde(skip)]
    pub inner_trims: Vec<cadkernel_geometry::ParametricWire2D>,
}

impl std::fmt::Debug for FaceData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("FaceData");
        s.field("outer_loop", &self.outer_loop)
            .field("inner_loops", &self.inner_loops)
            .field("shell", &self.shell)
            .field("tag", &self.tag);
        #[cfg(feature = "geometry-binding")]
        s.field("has_surface", &self.surface.is_some())
            .field("has_outer_trim", &self.outer_trim.is_some())
            .field("inner_trims_count", &self.inner_trims.len());
        s.field("orientation", &self.orientation).finish()
    }
}

impl FaceData {
    /// Creates a face bounded by `outer_loop` with forward orientation and no surface.
    pub fn new(outer_loop: Handle<LoopData>) -> Self {
        Self {
            outer_loop,
            inner_loops: Vec::new(),
            shell: None,
            tag: None,
            #[cfg(feature = "geometry-binding")]
            surface: None,
            orientation: Orientation::Forward,
            #[cfg(feature = "geometry-binding")]
            outer_trim: None,
            #[cfg(feature = "geometry-binding")]
            inner_trims: Vec::new(),
        }
    }
}
