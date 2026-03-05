use std::fmt;

use serde::{Deserialize, Serialize};

/// Unique identifier for a modeling operation in the construction history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OperationId(pub u64);

/// The kind of topological entity a [`Tag`] refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityKind {
    Vertex,
    Edge,
    HalfEdge,
    Loop,
    Wire,
    Face,
    Shell,
    Solid,
}

/// Describes how an entity came into existence within a single operation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SegmentKind {
    /// Newly generated entity with a local index within the operation.
    Generated(u32),
    /// Geometry changed but topological role is the same.
    Modified,
    /// Parent entity was split; `u32` is the part index.
    Split(u32),
    /// Entity resulted from merging multiple parents.
    Merged,
}

/// One segment of a hierarchical [`Tag`] path.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TagSegment {
    pub operation: OperationId,
    pub kind: SegmentKind,
}

/// A persistent, hierarchical name for a topological entity.
///
/// A `Tag` is a sequence of [`TagSegment`]s encoding the construction history
/// that produced this entity. When a parametric model is rebuilt in the same
/// operation order, the same `Tag`s are generated, which keeps external
/// references (fillets, chamfers, constraints, etc.) valid.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tag {
    pub kind: EntityKind,
    pub segments: Vec<TagSegment>,
}

impl Tag {
    /// Creates a tag from a kind and a pre-built list of segments.
    pub fn new(kind: EntityKind, segments: Vec<TagSegment>) -> Self {
        Self { kind, segments }
    }

    /// Creates a tag for a freshly generated entity.
    pub fn generated(kind: EntityKind, op: OperationId, index: u32) -> Self {
        Self {
            kind,
            segments: vec![TagSegment {
                operation: op,
                kind: SegmentKind::Generated(index),
            }],
        }
    }

    /// Derives a new tag by appending a `Split` segment.
    pub fn split(&self, op: OperationId, part_index: u32) -> Self {
        let mut segments = self.segments.clone();
        segments.push(TagSegment {
            operation: op,
            kind: SegmentKind::Split(part_index),
        });
        Self {
            kind: self.kind,
            segments,
        }
    }

    /// Derives a new tag by appending a `Modified` segment.
    pub fn modified(&self, op: OperationId) -> Self {
        let mut segments = self.segments.clone();
        segments.push(TagSegment {
            operation: op,
            kind: SegmentKind::Modified,
        });
        Self {
            kind: self.kind,
            segments,
        }
    }

    /// Derives a new tag by appending a `Merged` segment.
    pub fn merged(&self, op: OperationId) -> Self {
        let mut segments = self.segments.clone();
        segments.push(TagSegment {
            operation: op,
            kind: SegmentKind::Merged,
        });
        Self {
            kind: self.kind,
            segments,
        }
    }
}

impl fmt::Debug for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tag({:?}, [", self.kind)?;
        for (i, seg) in self.segments.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "Op{}:{:?}", seg.operation.0, seg.kind)?;
        }
        write!(f, "])")
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_generated() {
        let t = Tag::generated(EntityKind::Face, OperationId(1), 0);
        assert_eq!(t.segments.len(), 1);
        assert_eq!(t.kind, EntityKind::Face);
    }

    #[test]
    fn test_tag_split_extends() {
        let t = Tag::generated(EntityKind::Face, OperationId(1), 0);
        let s = t.split(OperationId(2), 1);
        assert_eq!(s.segments.len(), 2);
        assert_eq!(s.segments[1].kind, SegmentKind::Split(1));
    }

    #[test]
    fn test_tag_equality() {
        let a = Tag::generated(EntityKind::Edge, OperationId(1), 3);
        let b = Tag::generated(EntityKind::Edge, OperationId(1), 3);
        assert_eq!(a, b);
    }
}
