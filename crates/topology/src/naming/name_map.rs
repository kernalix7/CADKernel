use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::tag::{EntityKind, Tag};
use crate::handle::Handle;
use crate::{
    EdgeData, FaceData, HalfEdgeData, LoopData, ShellData, SolidData, VertexData, WireData,
};

/// A type-erased entity reference used in the name map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityRef {
    Vertex(Handle<VertexData>),
    Edge(Handle<EdgeData>),
    HalfEdge(Handle<HalfEdgeData>),
    Loop(Handle<LoopData>),
    Wire(Handle<WireData>),
    Face(Handle<FaceData>),
    Shell(Handle<ShellData>),
    Solid(Handle<SolidData>),
}

impl EntityRef {
    /// Returns the [`EntityKind`] that this reference points to.
    pub fn kind(&self) -> EntityKind {
        match self {
            Self::Vertex(_) => EntityKind::Vertex,
            Self::Edge(_) => EntityKind::Edge,
            Self::HalfEdge(_) => EntityKind::HalfEdge,
            Self::Loop(_) => EntityKind::Loop,
            Self::Wire(_) => EntityKind::Wire,
            Self::Face(_) => EntityKind::Face,
            Self::Shell(_) => EntityKind::Shell,
            Self::Solid(_) => EntityKind::Solid,
        }
    }
}

/// Bidirectional mapping between persistent [`Tag`]s and runtime entity handles.
///
/// Serialized as a JSON array of `[tag, entity_ref]` pairs because `Tag` is
/// not a string and cannot serve as a JSON object key directly.
#[derive(Debug, Clone)]
pub struct NameMap {
    tag_to_entity: HashMap<Tag, EntityRef>,
}

impl Serialize for NameMap {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let entries: Vec<(&Tag, &EntityRef)> = self.tag_to_entity.iter().collect();
        entries.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for NameMap {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let entries: Vec<(Tag, EntityRef)> = Vec::deserialize(deserializer)?;
        Ok(NameMap {
            tag_to_entity: entries.into_iter().collect(),
        })
    }
}

impl NameMap {
    /// Creates an empty name map.
    pub fn new() -> Self {
        Self {
            tag_to_entity: HashMap::new(),
        }
    }

    /// Associates a tag with an entity reference.
    pub fn insert(&mut self, tag: Tag, entity: EntityRef) {
        self.tag_to_entity.insert(tag, entity);
    }

    /// Looks up the entity for a given tag.
    pub fn get(&self, tag: &Tag) -> Option<EntityRef> {
        self.tag_to_entity.get(tag).copied()
    }

    /// Looks up a vertex handle by tag.
    pub fn get_vertex(&self, tag: &Tag) -> Option<Handle<VertexData>> {
        match self.get(tag)? {
            EntityRef::Vertex(h) => Some(h),
            _ => None,
        }
    }

    /// Looks up an edge handle by tag.
    pub fn get_edge(&self, tag: &Tag) -> Option<Handle<EdgeData>> {
        match self.get(tag)? {
            EntityRef::Edge(h) => Some(h),
            _ => None,
        }
    }

    /// Looks up a wire handle by tag.
    pub fn get_wire(&self, tag: &Tag) -> Option<Handle<WireData>> {
        match self.get(tag)? {
            EntityRef::Wire(h) => Some(h),
            _ => None,
        }
    }

    /// Looks up a face handle by tag.
    pub fn get_face(&self, tag: &Tag) -> Option<Handle<FaceData>> {
        match self.get(tag)? {
            EntityRef::Face(h) => Some(h),
            _ => None,
        }
    }

    /// Looks up a shell handle by tag.
    pub fn get_shell(&self, tag: &Tag) -> Option<Handle<ShellData>> {
        match self.get(tag)? {
            EntityRef::Shell(h) => Some(h),
            _ => None,
        }
    }

    /// Looks up a solid handle by tag.
    pub fn get_solid(&self, tag: &Tag) -> Option<Handle<SolidData>> {
        match self.get(tag)? {
            EntityRef::Solid(h) => Some(h),
            _ => None,
        }
    }

    /// Removes the mapping for a tag.
    pub fn remove(&mut self, tag: &Tag) -> Option<EntityRef> {
        self.tag_to_entity.remove(tag)
    }

    /// Number of mappings.
    pub fn len(&self) -> usize {
        self.tag_to_entity.len()
    }

    /// Returns `true` if the map contains no entries.
    pub fn is_empty(&self) -> bool {
        self.tag_to_entity.is_empty()
    }

    /// Iterates over all (tag, entity) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&Tag, &EntityRef)> {
        self.tag_to_entity.iter()
    }
}

impl Default for NameMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::naming::tag::OperationId;
    use std::marker::PhantomData;

    fn dummy_vertex_handle() -> Handle<VertexData> {
        Handle {
            index: 0,
            generation: 0,
            _marker: PhantomData,
        }
    }

    #[test]
    fn test_insert_and_get() {
        let mut map = NameMap::new();
        let tag = Tag::generated(EntityKind::Vertex, OperationId(1), 0);
        let h = dummy_vertex_handle();
        map.insert(tag.clone(), EntityRef::Vertex(h));
        assert_eq!(map.get_vertex(&tag), Some(h));
    }

    #[test]
    fn test_missing_tag_returns_none() {
        let map = NameMap::new();
        let tag = Tag::generated(EntityKind::Face, OperationId(99), 0);
        assert!(map.get(&tag).is_none());
    }
}
