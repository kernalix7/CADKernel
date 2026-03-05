pub mod history;
pub mod name_map;
pub mod tag;

pub use history::{Evolution, EvolutionRecord, ShapeHistory};
pub use name_map::{EntityRef, NameMap};
pub use tag::{EntityKind, OperationId, SegmentKind, Tag, TagSegment};
