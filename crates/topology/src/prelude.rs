//! Convenience re-exports of the most commonly used topology types.
//!
//! ```
//! use cadkernel_topology::prelude::*;
//! ```

pub use crate::BRepModel;
pub use crate::edge::EdgeData;
pub use crate::error::{KernelError, KernelResult};
pub use crate::face::{FaceData, Orientation};
pub use crate::halfedge::HalfEdgeData;
pub use crate::handle::Handle;
pub use crate::history::ModelHistory;
pub use crate::loop_wire::LoopData;
pub use crate::naming::{EntityKind, EntityRef, NameMap, Tag};
pub use crate::properties::{Color, Material, PropertyStore, PropertyValue};
pub use crate::shell::ShellData;
pub use crate::solid::SolidData;
pub use crate::vertex::VertexData;
pub use crate::wire::WireData;
pub use crate::{ValidationIssue, ValidationSeverity};
