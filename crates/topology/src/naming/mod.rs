//! Persistent naming system for B-Rep entities.
//!
//! Provides [`Tag`] (hierarchical name), [`OperationId`] (operation counter),
//! [`ShapeHistory`] (construction history), and [`NameMap`] (tag-to-handle
//! bidirectional lookup). Together they ensure that external references remain
//! stable when a parametric model is rebuilt.

pub mod history;
pub mod name_map;
pub mod persistent_name;
pub mod tag;

pub use history::{Evolution, EvolutionRecord, ShapeHistory};
pub use name_map::{EntityRef, NameMap};
pub use persistent_name::{FeatureId, PersistentNameKey, PersistentNameTable};
pub use tag::{EntityKind, OperationId, SegmentKind, Tag, TagSegment};
