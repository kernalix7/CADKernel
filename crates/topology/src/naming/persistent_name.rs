use std::collections::{HashMap, HashSet};

use cadkernel_math::{Point3, Vec3};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{EntityKind, EntityRef, NameMap, OperationId, SegmentKind, Tag};
use crate::{BRepModel, EdgeData, FaceData, Handle, VertexData};

/// Stable feature-axis identifier used by the topology naming table.
///
/// Higher layers may wrap the same numeric id in their own API type; topology
/// keeps this local to avoid depending on crates above it in the workspace.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct FeatureId(pub u64);

/// `(FeatureId, OperationId, EntityKind, local_index)`.
pub type PersistentNameKey = (FeatureId, OperationId, EntityKind, u32);

/// A document-level index from a feature/entity axis to the currently valid tag.
///
/// The table complements per-model [`NameMap`] handles: it remembers which
/// feature operation generated a persistent name, then can rebind those names
/// to freshly rebuilt topology after a parametric recompute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistentNameTable {
    table: HashMap<PersistentNameKey, Tag>,
}

#[derive(Serialize, Deserialize)]
struct PersistentNameRecord {
    feature: FeatureId,
    operation: OperationId,
    kind: EntityKind,
    local: u32,
    tag: Tag,
}

impl Serialize for PersistentNameTable {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let records: Vec<_> = self
            .sorted_entries()
            .into_iter()
            .map(|(key, tag)| PersistentNameRecord {
                feature: key.0,
                operation: key.1,
                kind: key.2,
                local: key.3,
                tag,
            })
            .collect();
        records.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PersistentNameTable {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let records = Vec::<PersistentNameRecord>::deserialize(deserializer)?;
        let table = records
            .into_iter()
            .map(|record| {
                (
                    (record.feature, record.operation, record.kind, record.local),
                    record.tag,
                )
            })
            .collect();
        Ok(Self { table })
    }
}

impl Default for PersistentNameTable {
    fn default() -> Self {
        Self::new()
    }
}

impl PersistentNameTable {
    /// Creates an empty persistent-name table.
    pub fn new() -> Self {
        Self {
            table: HashMap::new(),
        }
    }

    /// Number of tracked names.
    pub fn len(&self) -> usize {
        self.table.len()
    }

    /// Returns `true` when no names are tracked.
    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    /// Registers a generated entity axis and returns its stable tag.
    ///
    /// If the axis was rebound earlier, the existing tag is returned instead
    /// of allocating a new one.
    pub fn register(
        &mut self,
        feat: FeatureId,
        op: OperationId,
        kind: EntityKind,
        local: u32,
    ) -> Tag {
        let key = (feat, op, kind, local);
        if let Some(tag) = self.table.get(&key) {
            return tag.clone();
        }
        let tag = Tag::generated(kind, op, local);
        self.table.insert(key, tag.clone());
        tag
    }

    /// Registers an already-assigned tag for an entity axis.
    pub fn register_existing(
        &mut self,
        feat: FeatureId,
        op: OperationId,
        kind: EntityKind,
        local: u32,
        tag: Tag,
    ) -> Tag {
        let key = (feat, op, kind, local);
        if let Some(existing) = self.table.get(&key) {
            return existing.clone();
        }
        self.table.insert(key, tag.clone());
        tag
    }

    /// Registers every tagged entity in `model` that is not already known.
    ///
    /// This is intended for feature append/replay paths where the resulting
    /// B-Rep contains both previous feature tags and newly generated tags.
    pub fn register_model_feature(&mut self, feat: FeatureId, model: &BRepModel) {
        for (kind, tag) in collect_model_tags(model) {
            if self.resolve(&tag).is_some() {
                continue;
            }
            if let Some((op, local)) = generated_axis_from_tag(&tag, kind) {
                self.register_existing(feat, op, kind, local, tag);
            }
        }
    }

    /// Resolves a tag back to its `(FeatureId, OperationId, EntityKind, local_index)` axis.
    pub fn resolve(&self, tag: &Tag) -> Option<PersistentNameKey> {
        self.table
            .iter()
            .filter_map(|(key, candidate)| (candidate == tag).then_some(*key))
            .min_by_key(|key| key_sort_tuple(*key))
    }

    /// Returns all entries in deterministic order.
    pub fn entries(&self) -> Vec<(PersistentNameKey, Tag)> {
        self.sorted_entries()
    }

    /// Rebinds table entries to matching entities in `new_brep`.
    ///
    /// This immutable variant updates the table only. Use
    /// [`Self::rebind_model_after_recompute`] when the rebuilt B-Rep should
    /// also receive the surviving tags and a rebuilt [`NameMap`].
    pub fn rebind_after_recompute(&mut self, old_brep: &BRepModel, new_brep: &BRepModel) {
        let assignments = self.compute_rebind_assignments(old_brep, new_brep);
        let new_tags = model_tag_set(new_brep);
        let mut rebound = HashMap::new();

        for (key, tag) in &self.table {
            if new_tags.contains(tag) {
                rebound.insert(*key, tag.clone());
            }
        }
        for assignment in assignments {
            rebound.insert(assignment.key, assignment.tag);
        }
        self.table = rebound;
    }

    /// Rebinds a freshly rebuilt B-Rep in-place and rebuilds its runtime name map.
    pub fn rebind_model_after_recompute(&mut self, old_brep: &BRepModel, new_brep: &mut BRepModel) {
        let assignments = self.compute_rebind_assignments(old_brep, new_brep);
        let new_tags = model_tag_set(new_brep);
        let mut rebound = HashMap::new();

        for (key, tag) in &self.table {
            if new_tags.contains(tag) {
                rebound.insert(*key, tag.clone());
            }
        }
        for assignment in assignments {
            set_entity_tag(new_brep, assignment.handle, assignment.tag.clone());
            rebound.insert(assignment.key, assignment.tag);
        }

        self.table = rebound;
        assign_fresh_tags_to_untagged_entities(new_brep);
        rebuild_name_map(new_brep);
        self.prune_to_model(new_brep);
    }

    /// Drops table entries whose tags are not present in `model`.
    pub fn prune_to_model(&mut self, model: &BRepModel) {
        let tags = model_tag_set(model);
        self.table.retain(|_, tag| tags.contains(tag));
    }

    fn sorted_entries(&self) -> Vec<(PersistentNameKey, Tag)> {
        let mut entries: Vec<_> = self
            .table
            .iter()
            .map(|(key, tag)| (*key, tag.clone()))
            .collect();
        entries.sort_by_key(|(key, _)| key_sort_tuple(*key));
        entries
    }

    fn compute_rebind_assignments(
        &self,
        old_brep: &BRepModel,
        new_brep: &BRepModel,
    ) -> Vec<RebindAssignment> {
        let mut candidates = collect_candidates(new_brep);
        let max_score = rebind_score_limit(old_brep, new_brep);
        let mut assignments = Vec::new();

        for (key, old_tag) in self.sorted_entries() {
            let Some(old_entity) = descriptor_for_tag(old_brep, &old_tag, key.2) else {
                continue;
            };
            let mut best: Option<(usize, f64)> = None;
            for (idx, candidate) in candidates.iter().enumerate() {
                if candidate.kind != old_entity.kind {
                    continue;
                }
                let score = old_entity.score(candidate, key, &old_tag, max_score);
                if score.is_finite() && score <= max_score {
                    match best {
                        Some((_, best_score)) if score >= best_score => {}
                        _ => best = Some((idx, score)),
                    }
                }
            }

            if let Some((idx, _)) = best {
                let candidate = candidates.swap_remove(idx);
                assignments.push(RebindAssignment {
                    key,
                    tag: old_tag,
                    handle: candidate.handle,
                });
            }
        }

        assignments
    }
}

#[derive(Clone)]
struct RebindAssignment {
    key: PersistentNameKey,
    tag: Tag,
    handle: EntityHandle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum EntityHandle {
    Vertex(Handle<VertexData>),
    Edge(Handle<EdgeData>),
    Face(Handle<FaceData>),
}

#[derive(Clone)]
struct EntityDescriptor {
    kind: EntityKind,
    handle: EntityHandle,
    tag: Option<Tag>,
    anchor: Point3,
    normal: Option<Vec3>,
    direction: Option<Vec3>,
    length: Option<f64>,
    boundary_count: usize,
    geometry_kind: &'static str,
}

impl EntityDescriptor {
    fn score(&self, candidate: &Self, key: PersistentNameKey, old_tag: &Tag, scale_sq: f64) -> f64 {
        let mut score = self.anchor.distance_to(candidate.anchor).powi(2);

        if let (Some(a), Some(b)) = (self.normal, candidate.normal) {
            score += (1.0 - a.dot(b).clamp(-1.0, 1.0)).abs() * scale_sq;
        }
        if let (Some(a), Some(b)) = (self.direction, candidate.direction) {
            score += (1.0 - a.dot(b).abs().clamp(0.0, 1.0)) * scale_sq * 0.25;
        }
        if let (Some(a), Some(b)) = (self.length, candidate.length) {
            let denom = a.abs().max(b.abs()).max(1.0);
            let rel = (a - b).abs() / denom;
            score += rel * rel * scale_sq * 0.25;
        }
        if self.boundary_count != candidate.boundary_count {
            score += scale_sq * 0.1;
        }
        if self.geometry_kind != candidate.geometry_kind {
            score += scale_sq * 0.05;
        }
        if candidate.tag.as_ref() == Some(old_tag) {
            score *= 0.01;
        } else if candidate
            .tag
            .as_ref()
            .and_then(|tag| generated_axis_from_tag(tag, key.2))
            .is_some_and(|(op, local)| op == key.1 && local == key.3)
        {
            score *= 0.1;
        }

        score
    }
}

fn key_sort_tuple(key: PersistentNameKey) -> (u64, u64, u8, u32) {
    (key.0.0, key.1.0, kind_rank(key.2), key.3)
}

fn kind_rank(kind: EntityKind) -> u8 {
    match kind {
        EntityKind::Vertex => 0,
        EntityKind::Edge => 1,
        EntityKind::HalfEdge => 2,
        EntityKind::Loop => 3,
        EntityKind::Wire => 4,
        EntityKind::Face => 5,
        EntityKind::Shell => 6,
        EntityKind::Solid => 7,
    }
}

fn generated_axis_from_tag(tag: &Tag, expected_kind: EntityKind) -> Option<(OperationId, u32)> {
    if tag.kind != expected_kind {
        return None;
    }
    tag.segments
        .iter()
        .rev()
        .find_map(|segment| match &segment.kind {
            SegmentKind::Generated(local) => Some((segment.operation, *local)),
            _ => None,
        })
}

fn collect_model_tags(model: &BRepModel) -> Vec<(EntityKind, Tag)> {
    let mut tags = Vec::new();
    for (_, entity) in model.vertices.iter() {
        push_tag(&mut tags, EntityKind::Vertex, &entity.tag);
    }
    for (_, entity) in model.edges.iter() {
        push_tag(&mut tags, EntityKind::Edge, &entity.tag);
    }
    for (_, entity) in model.half_edges.iter() {
        push_tag(&mut tags, EntityKind::HalfEdge, &entity.tag);
    }
    for (_, entity) in model.loops.iter() {
        push_tag(&mut tags, EntityKind::Loop, &entity.tag);
    }
    for (_, entity) in model.wires.iter() {
        push_tag(&mut tags, EntityKind::Wire, &entity.tag);
    }
    for (_, entity) in model.faces.iter() {
        push_tag(&mut tags, EntityKind::Face, &entity.tag);
    }
    for (_, entity) in model.shells.iter() {
        push_tag(&mut tags, EntityKind::Shell, &entity.tag);
    }
    for (_, entity) in model.solids.iter() {
        push_tag(&mut tags, EntityKind::Solid, &entity.tag);
    }
    tags
}

fn push_tag(out: &mut Vec<(EntityKind, Tag)>, kind: EntityKind, tag: &Option<Tag>) {
    if let Some(tag) = tag {
        out.push((kind, tag.clone()));
    }
}

fn model_tag_set(model: &BRepModel) -> HashSet<Tag> {
    collect_model_tags(model)
        .into_iter()
        .map(|(_, tag)| tag)
        .collect()
}

fn collect_candidates(model: &BRepModel) -> Vec<EntityDescriptor> {
    let mut candidates = Vec::new();
    for (handle, entity) in model.vertices.iter() {
        candidates.push(EntityDescriptor {
            kind: EntityKind::Vertex,
            handle: EntityHandle::Vertex(handle),
            tag: entity.tag.clone(),
            anchor: entity.point,
            normal: None,
            direction: None,
            length: None,
            boundary_count: 0,
            geometry_kind: "vertex",
        });
    }
    for (handle, entity) in model.edges.iter() {
        if let Some((midpoint, direction, length)) = edge_geometry(model, entity) {
            candidates.push(EntityDescriptor {
                kind: EntityKind::Edge,
                handle: EntityHandle::Edge(handle),
                tag: entity.tag.clone(),
                anchor: midpoint,
                normal: None,
                direction,
                length: Some(length),
                boundary_count: 2,
                geometry_kind: edge_geometry_kind(entity),
            });
        }
    }
    for (handle, entity) in model.faces.iter() {
        if let Some((centroid, normal, boundary_count)) = face_geometry(model, handle) {
            candidates.push(EntityDescriptor {
                kind: EntityKind::Face,
                handle: EntityHandle::Face(handle),
                tag: entity.tag.clone(),
                anchor: centroid,
                normal,
                direction: None,
                length: None,
                boundary_count,
                geometry_kind: face_geometry_kind(entity),
            });
        }
    }
    candidates
}

fn descriptor_for_tag(model: &BRepModel, tag: &Tag, kind: EntityKind) -> Option<EntityDescriptor> {
    match kind {
        EntityKind::Vertex => {
            let handle = model.name_map.get_vertex(tag)?;
            let entity = model.vertices.get(handle)?;
            Some(EntityDescriptor {
                kind,
                handle: EntityHandle::Vertex(handle),
                tag: entity.tag.clone(),
                anchor: entity.point,
                normal: None,
                direction: None,
                length: None,
                boundary_count: 0,
                geometry_kind: "vertex",
            })
        }
        EntityKind::Edge => {
            let handle = model.name_map.get_edge(tag)?;
            let entity = model.edges.get(handle)?;
            let (midpoint, direction, length) = edge_geometry(model, entity)?;
            Some(EntityDescriptor {
                kind,
                handle: EntityHandle::Edge(handle),
                tag: entity.tag.clone(),
                anchor: midpoint,
                normal: None,
                direction,
                length: Some(length),
                boundary_count: 2,
                geometry_kind: edge_geometry_kind(entity),
            })
        }
        EntityKind::Face => {
            let handle = model.name_map.get_face(tag)?;
            let entity = model.faces.get(handle)?;
            let (centroid, normal, boundary_count) = face_geometry(model, handle)?;
            Some(EntityDescriptor {
                kind,
                handle: EntityHandle::Face(handle),
                tag: entity.tag.clone(),
                anchor: centroid,
                normal,
                direction: None,
                length: None,
                boundary_count,
                geometry_kind: face_geometry_kind(entity),
            })
        }
        _ => None,
    }
}

fn edge_geometry(model: &BRepModel, edge: &EdgeData) -> Option<(Point3, Option<Vec3>, f64)> {
    let start = model.vertices.get(edge.start)?.point;
    let end = model.vertices.get(edge.end)?.point;
    let delta = end - start;
    let length = delta.length();
    Some((start.midpoint(end), delta.normalized(), length))
}

fn face_geometry(
    model: &BRepModel,
    face: Handle<FaceData>,
) -> Option<(Point3, Option<Vec3>, usize)> {
    let vertices = model.vertices_of_face(face).ok()?;
    if vertices.is_empty() {
        return None;
    }

    let mut sx = 0.0;
    let mut sy = 0.0;
    let mut sz = 0.0;
    let mut points = Vec::with_capacity(vertices.len());
    for handle in vertices {
        let point = model.vertices.get(handle)?.point;
        sx += point.x;
        sy += point.y;
        sz += point.z;
        points.push(point);
    }

    let n = points.len() as f64;
    let centroid = Point3::new(sx / n, sy / n, sz / n);
    let normal = polygon_normal(&points);
    Some((centroid, normal, points.len()))
}

fn polygon_normal(points: &[Point3]) -> Option<Vec3> {
    if points.len() < 3 {
        return None;
    }
    let mut normal = Vec3::ZERO;
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        normal.x += (a.y - b.y) * (a.z + b.z);
        normal.y += (a.z - b.z) * (a.x + b.x);
        normal.z += (a.x - b.x) * (a.y + b.y);
    }
    normal.normalized()
}

#[cfg(feature = "geometry-binding")]
fn edge_geometry_kind(edge: &EdgeData) -> &'static str {
    edge.curve
        .as_deref()
        .map(std::any::type_name_of_val)
        .unwrap_or("none")
}

#[cfg(not(feature = "geometry-binding"))]
fn edge_geometry_kind(_edge: &EdgeData) -> &'static str {
    "none"
}

#[cfg(feature = "geometry-binding")]
fn face_geometry_kind(face: &FaceData) -> &'static str {
    face.surface
        .as_deref()
        .map(std::any::type_name_of_val)
        .unwrap_or("none")
}

#[cfg(not(feature = "geometry-binding"))]
fn face_geometry_kind(_face: &FaceData) -> &'static str {
    "none"
}

fn set_entity_tag(model: &mut BRepModel, handle: EntityHandle, tag: Tag) {
    match handle {
        EntityHandle::Vertex(handle) => {
            if let Some(entity) = model.vertices.get_mut(handle) {
                entity.tag = Some(tag);
            }
        }
        EntityHandle::Edge(handle) => {
            if let Some(entity) = model.edges.get_mut(handle) {
                entity.tag = Some(tag);
            }
        }
        EntityHandle::Face(handle) => {
            if let Some(entity) = model.faces.get_mut(handle) {
                entity.tag = Some(tag);
            }
        }
    }
}

fn assign_fresh_tags_to_untagged_entities(model: &mut BRepModel) {
    let op = model.history.current_op_id().unwrap_or(OperationId(1));
    for (handle, entity) in model.vertices.iter_mut() {
        if entity.tag.is_none() {
            entity.tag = Some(Tag::generated(EntityKind::Vertex, op, handle.index()));
        }
    }
    for (handle, entity) in model.edges.iter_mut() {
        if entity.tag.is_none() {
            entity.tag = Some(Tag::generated(EntityKind::Edge, op, handle.index()));
        }
    }
    for (handle, entity) in model.faces.iter_mut() {
        if entity.tag.is_none() {
            entity.tag = Some(Tag::generated(EntityKind::Face, op, handle.index()));
        }
    }
}

fn rebuild_name_map(model: &mut BRepModel) {
    let mut name_map = NameMap::new();
    for (handle, entity) in model.vertices.iter() {
        insert_if_tagged(&mut name_map, &entity.tag, EntityRef::Vertex(handle));
    }
    for (handle, entity) in model.edges.iter() {
        insert_if_tagged(&mut name_map, &entity.tag, EntityRef::Edge(handle));
    }
    for (handle, entity) in model.half_edges.iter() {
        insert_if_tagged(&mut name_map, &entity.tag, EntityRef::HalfEdge(handle));
    }
    for (handle, entity) in model.loops.iter() {
        insert_if_tagged(&mut name_map, &entity.tag, EntityRef::Loop(handle));
    }
    for (handle, entity) in model.wires.iter() {
        insert_if_tagged(&mut name_map, &entity.tag, EntityRef::Wire(handle));
    }
    for (handle, entity) in model.faces.iter() {
        insert_if_tagged(&mut name_map, &entity.tag, EntityRef::Face(handle));
    }
    for (handle, entity) in model.shells.iter() {
        insert_if_tagged(&mut name_map, &entity.tag, EntityRef::Shell(handle));
    }
    for (handle, entity) in model.solids.iter() {
        insert_if_tagged(&mut name_map, &entity.tag, EntityRef::Solid(handle));
    }
    model.name_map = name_map;
}

fn insert_if_tagged(name_map: &mut NameMap, tag: &Option<Tag>, entity: EntityRef) {
    if let Some(tag) = tag {
        name_map.insert(tag.clone(), entity);
    }
}

fn rebind_score_limit(old_brep: &BRepModel, new_brep: &BRepModel) -> f64 {
    let diag = model_diagonal(old_brep)
        .max(model_diagonal(new_brep))
        .max(1.0);
    diag * diag * 16.0 + 1.0e-9
}

fn model_diagonal(model: &BRepModel) -> f64 {
    let mut iter = model.vertices.iter();
    let Some((_, first)) = iter.next() else {
        return 1.0;
    };
    let mut min = first.point;
    let mut max = first.point;
    for (_, vertex) in iter {
        min.x = min.x.min(vertex.point.x);
        min.y = min.y.min(vertex.point.y);
        min.z = min.z.min(vertex.point.z);
        max.x = max.x.max(vertex.point.x);
        max.y = max.y.max(vertex.point.y);
        max.z = max.z.max(vertex.point.z);
    }
    min.distance_to(max)
}
