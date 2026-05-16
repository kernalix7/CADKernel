//! Document — the top-level model container.
//!
//! In Phase 1 the `Document` is a list of independent solids, each carried
//! by its own [`BRepModel`]. Phase 2 absorbs sketches, drawings, assembly,
//! FEM context, and a unified history into this type. The public surface
//! defined here is intentionally narrow so Phase 2 can extend it without
//! breaking changes.
//!
//! ## Stable IDs
//!
//! Solids are addressed by [`SolidId`], a monotonically-increasing `u32`.
//! Deleting a solid leaves the slot occupied by `None` so future `Command`s
//! that reference the deleted ID fail cleanly with `ApiError::UnknownSolid`
//! instead of silently aliasing onto a different solid.

use cadkernel_modeling::body::Body;
use cadkernel_modeling::measure::solid_mass_properties;
use cadkernel_topology::{
    BRepModel, EdgeData, FaceData, Handle, PersistentFeatureId, PersistentNameTable, SolidData,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::hash::{DefaultHasher, Hasher};

use crate::command::{BodyId, EdgeRef, FaceRef, SketchConstraint, SketchEntity, SketchId};
use crate::outcome::Plane;
use crate::{ApiError, ApiResult};

/// Stable, opaque identifier for a solid inside a [`Document`].
///
/// IDs are monotonically increasing and never reused. Persistent across
/// save/open via the JSON representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SolidId(pub u32);

impl std::fmt::Display for SolidId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "solid#{}", self.0)
    }
}

/// Internal slot. Each populated slot owns its own [`BRepModel`] containing
/// exactly one solid. This mirrors the MCP server's pre-existing layout and
/// keeps the kernel APIs (which work on `&mut BRepModel`) directly usable.
#[derive(Default)]
pub(crate) struct SolidSlot {
    pub(crate) model: BRepModel,
    pub(crate) handle: Option<Handle<SolidData>>,
    pub(crate) label: String,
}

/// LRU cache for replayed body-feature outputs.
#[derive(Debug, Clone)]
pub struct RecomputeCache {
    entries: HashMap<(u32, u64), Vec<u8>>,
    lru: VecDeque<(u32, u64)>,
    max_entries: usize,
    hits: u64,
    misses: u64,
}

impl Default for RecomputeCache {
    fn default() -> Self {
        Self::new(256)
    }
}

impl RecomputeCache {
    /// Creates an empty cache with a maximum number of entries.
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            lru: VecDeque::new(),
            max_entries,
            hits: 0,
            misses: 0,
        }
    }

    /// Returns the number of cached feature snapshots.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true when no feature snapshots are cached.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the configured LRU entry limit.
    pub fn max_entries(&self) -> usize {
        self.max_entries
    }

    /// Returns cumulative `(hits, misses)` since cache creation.
    pub fn stats(&self) -> (u64, u64) {
        (self.hits, self.misses)
    }

    /// Resets hit/miss counters without clearing cached entries.
    pub fn reset_stats(&mut self) {
        self.hits = 0;
        self.misses = 0;
    }

    /// Deterministic FNV-1a input hash over spec bytes and parent hashes.
    pub fn input_hash(spec_bytes: &[u8], parent_hashes: &[(u32, u64)]) -> u64 {
        let mut parents = parent_hashes.to_vec();
        parents.sort_unstable_by_key(|(feature_index, _)| *feature_index);

        let mut hash = FNV_OFFSET_BASIS;
        hash = fnv_write_u64(hash, spec_bytes.len() as u64);
        hash = fnv_write_bytes(hash, spec_bytes);
        hash = fnv_write_u64(hash, parents.len() as u64);
        for (feature_index, parent_hash) in parents {
            hash = fnv_write_u64(hash, u64::from(feature_index));
            hash = fnv_write_u64(hash, parent_hash);
        }
        hash
    }

    /// Returns cached bytes and refreshes the entry's LRU position.
    pub fn get(&mut self, feature_index: u32, input_hash: u64) -> Option<&[u8]> {
        let key = (feature_index, input_hash);
        if !self.entries.contains_key(&key) {
            self.misses += 1;
            return None;
        }
        self.hits += 1;
        self.touch(key);
        self.entries.get(&key).map(Vec::as_slice)
    }

    /// Inserts or replaces a cached snapshot.
    pub fn insert(&mut self, feature_index: u32, input_hash: u64, brep_bytes: Vec<u8>) {
        if self.max_entries == 0 {
            return;
        }

        let key = (feature_index, input_hash);
        if let Some(entry) = self.entries.get_mut(&key) {
            *entry = brep_bytes;
            self.touch(key);
            return;
        }

        while self.entries.len() >= self.max_entries {
            let Some(oldest) = self.lru.pop_front() else {
                break;
            };
            self.entries.remove(&oldest);
        }

        self.entries.insert(key, brep_bytes);
        self.touch(key);
    }

    /// Removes every cached snapshot for `feature_index`.
    pub fn invalidate_feature(&mut self, feature_index: u32) {
        self.entries
            .retain(|(cached_feature, _), _| *cached_feature != feature_index);
        self.lru
            .retain(|(cached_feature, _)| *cached_feature != feature_index);
    }

    fn touch(&mut self, key: (u32, u64)) {
        self.lru.retain(|existing| *existing != key);
        self.lru.push_back(key);
    }
}

/// Stable identifier for a [`HistoryEvent`] inside a [`Document`].
///
/// Assigned monotonically by [`Document::push_history`] starting at `1`
/// (the value `0` is reserved as a "not-yet-assigned" sentinel — events
/// deserialised from older `.cadk` files via [`serde(default)`] carry
/// `FeatureId(0)` until [`Session::replay`](crate::Session::replay)
/// runs them through `push_history` again and reassigns fresh IDs).
///
/// Foundation for the A2 feature surface: A2.2 [`Mirror`](crate::Command::Mirror),
/// A2.3 [`LinearPattern`](crate::Command::LinearPattern), and A2.4
/// [`Extrude::UpToFace`](crate::ExtrudeKind) all consume `FeatureId`
/// references instead of [`SolidId`]s — patterning a "feature" rather
/// than a finished solid is what unlocks PartDesign-style body editing.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct FeatureId(pub u64);

impl std::fmt::Display for FeatureId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FeatureId({})", self.0)
    }
}

/// The model container.
#[derive(Default)]
pub struct Document {
    slots: Vec<Option<SolidSlot>>,
    next_id: u32,
    history: Vec<HistoryEvent>,
    /// Monotonic counter for the next [`FeatureId`] handed out by
    /// [`Self::push_history`]. Starts at `0`; the increment-then-assign
    /// pattern in `push_history` means the first event gets `FeatureId(1)`,
    /// leaving `FeatureId(0)` as the sentinel for not-yet-assigned events.
    next_feature_id: u64,
    bodies: Vec<Option<Body>>,
    next_body_id: u64,
    active_body: Option<BodyId>,
    sketches: Vec<Option<PersistedSketch>>,
    next_sketch_id: u64,
    active_sketch: Option<SketchId>,
    persistent_names: PersistentNameTable,
    recompute_cache: RecomputeCache,
}

impl Document {
    /// Creates an empty document.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of populated solids.
    pub fn solid_count(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }

    /// Returns true if no solids are stored.
    pub fn is_empty(&self) -> bool {
        self.solid_count() == 0
    }

    /// Returns the IDs of all populated solids in creation order.
    pub fn solid_ids(&self) -> Vec<SolidId> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(idx, slot)| slot.as_ref().map(|_| SolidId(idx as u32)))
            .collect()
    }

    /// Returns the human-readable label of a solid, if it exists.
    pub fn solid_label(&self, id: SolidId) -> Option<&str> {
        self.get_slot(id).map(|s| s.label.as_str())
    }

    /// Returns the recorded history events in execution order. Each
    /// successful [`Command`](crate::Command) appends exactly one event.
    pub fn history(&self) -> &[HistoryEvent] {
        &self.history
    }

    /// Alias for [`Self::history`] using the PartDesign-style "feature"
    /// vocabulary. Returned slice is the same `&[HistoryEvent]`; callers
    /// pick the name that reads better at the call site.
    pub fn features(&self) -> &[HistoryEvent] {
        &self.history
    }

    /// Returns the document-level persistent-name table.
    pub fn persistent_names(&self) -> &PersistentNameTable {
        &self.persistent_names
    }

    pub(crate) fn persistent_names_mut(&mut self) -> &mut PersistentNameTable {
        &mut self.persistent_names
    }

    /// Looks up a recorded event by its [`FeatureId`]. Linear scan over
    /// the history vector — `O(n)` in history length but typical CAD
    /// sessions stay well under 10k events.
    ///
    /// Returns `None` when `id` does not match any event, including the
    /// reserved sentinel `FeatureId(0)`.
    pub fn feature(&self, id: FeatureId) -> Option<&HistoryEvent> {
        if id.0 == 0 {
            return None;
        }
        self.history.iter().find(|ev| ev.feature_id == id)
    }

    /// Returns a body by stable id.
    pub fn body(&self, id: BodyId) -> Option<&Body> {
        if id.0 == 0 {
            return None;
        }
        self.bodies
            .get(id.0 as usize)
            .and_then(|body| body.as_ref())
    }

    pub(crate) fn body_mut(&mut self, id: BodyId) -> Option<&mut Body> {
        if id.0 == 0 {
            return None;
        }
        self.bodies
            .get_mut(id.0 as usize)
            .and_then(|body| body.as_mut())
    }

    pub(crate) fn recompute_cache_mut(&mut self) -> &mut RecomputeCache {
        &mut self.recompute_cache
    }

    /// Returns the number of cached body-feature recompute snapshots.
    pub fn recompute_cache_len(&self) -> usize {
        self.recompute_cache.len()
    }

    /// Returns cumulative recompute-cache `(hits, misses)`.
    pub fn recompute_cache_stats(&self) -> (u64, u64) {
        self.recompute_cache.stats()
    }

    /// Returns the number of populated bodies.
    pub fn body_count(&self) -> usize {
        self.bodies.iter().filter(|body| body.is_some()).count()
    }

    /// Returns populated body ids in ascending id order.
    pub fn body_ids(&self) -> Vec<BodyId> {
        self.bodies
            .iter()
            .enumerate()
            .filter_map(|(idx, body)| body.as_ref().map(|_| BodyId(idx as u64)))
            .filter(|id| id.0 != 0)
            .collect()
    }

    /// Returns the active body id, if one is selected.
    pub fn active_body(&self) -> Option<BodyId> {
        self.active_body.filter(|id| self.body(*id).is_some())
    }

    pub(crate) fn set_active_body(&mut self, id: BodyId) {
        if self.body(id).is_some() {
            self.active_body = Some(id);
        }
    }

    pub(crate) fn push_body(&mut self, mut body: Body) -> BodyId {
        let id = BodyId(self.next_body_id + 1);
        self.next_body_id = id.0;
        body.id = id.0;
        let idx = id.0 as usize;
        if self.bodies.len() <= idx {
            self.bodies.resize_with(idx + 1, || None);
        }
        self.bodies[idx] = Some(body);
        if self.active_body.is_none() {
            self.active_body = Some(id);
        }
        id
    }

    /// Returns a persisted sketch by stable id.
    pub fn sketch(&self, id: SketchId) -> Option<&PersistedSketch> {
        if id.0 == 0 {
            return None;
        }
        self.sketches
            .get(id.0 as usize)
            .and_then(|sketch| sketch.as_ref())
    }

    pub(crate) fn sketch_mut(&mut self, id: SketchId) -> Option<&mut PersistedSketch> {
        if id.0 == 0 {
            return None;
        }
        self.sketches
            .get_mut(id.0 as usize)
            .and_then(|sketch| sketch.as_mut())
    }

    /// Returns the number of populated persisted sketches.
    pub fn sketch_count(&self) -> usize {
        self.sketches
            .iter()
            .filter(|sketch| sketch.is_some())
            .count()
    }

    /// Returns populated sketch ids in ascending id order.
    pub fn sketch_ids(&self) -> Vec<SketchId> {
        self.sketches
            .iter()
            .enumerate()
            .filter_map(|(idx, sketch)| sketch.as_ref().map(|_| SketchId(idx as u64)))
            .filter(|id| id.0 != 0)
            .collect()
    }

    /// Returns the active sketch id, if the active slot is still populated.
    pub fn active_sketch(&self) -> Option<SketchId> {
        self.active_sketch.filter(|id| self.sketch(*id).is_some())
    }

    pub(crate) fn set_active_sketch(&mut self, id: SketchId) {
        if self.sketch(id).is_some() {
            self.active_sketch = Some(id);
        }
    }

    pub(crate) fn push_sketch(&mut self, mut sketch: PersistedSketch) -> SketchId {
        let id = SketchId(self.next_sketch_id + 1);
        self.next_sketch_id = id.0;
        sketch.id = id;
        let idx = id.0 as usize;
        if self.sketches.len() <= idx {
            self.sketches.resize_with(idx + 1, || None);
        }
        self.sketches[idx] = Some(sketch);
        if self.active_sketch.is_none() {
            self.active_sketch = Some(id);
        }
        id
    }

    pub(crate) fn remove_sketch(&mut self, id: SketchId) -> bool {
        if id.0 == 0 {
            return false;
        }
        let idx = id.0 as usize;
        if let Some(slot) = self.sketches.get_mut(idx)
            && slot.is_some()
        {
            *slot = None;
            if self.active_sketch == Some(id) {
                self.active_sketch = self.sketch_ids().into_iter().next();
            }
            return true;
        }
        false
    }

    pub(crate) fn find_body_for_feature(&self, feature_id: FeatureId) -> Option<BodyId> {
        if feature_id.0 == 0 {
            return None;
        }
        self.bodies.iter().enumerate().find_map(|(idx, body)| {
            let body = body.as_ref()?;
            body.feature_index_of(feature_id.0)
                .map(|_| BodyId(idx as u64))
        })
    }

    pub(crate) fn push_history(&mut self, mut event: HistoryEvent) {
        self.next_feature_id += 1;
        event.feature_id = FeatureId(self.next_feature_id);
        self.history.push(event);
    }

    pub(crate) fn register_persistent_names_for_solid(
        &mut self,
        feature_id: FeatureId,
        solid: SolidId,
    ) {
        if feature_id.0 == 0 {
            return;
        }
        let Some(slot) = self
            .slots
            .get(solid.0 as usize)
            .and_then(|slot| slot.as_ref())
        else {
            return;
        };
        self.persistent_names
            .register_model_feature(PersistentFeatureId(feature_id.0), &slot.model);
    }

    #[allow(dead_code)]
    pub(crate) fn clear_history(&mut self) {
        self.history.clear();
        self.next_feature_id = 0;
    }

    /// Inserts a freshly-built model + handle and returns the assigned ID.
    pub(crate) fn insert(
        &mut self,
        model: BRepModel,
        handle: Handle<SolidData>,
        label: impl Into<String>,
    ) -> SolidId {
        let id = SolidId(self.next_id);
        self.next_id += 1;
        self.slots.push(Some(SolidSlot {
            model,
            handle: Some(handle),
            label: label.into(),
        }));
        id
    }

    /// Removes a solid from the document. Returns `false` if the ID is
    /// unknown or already removed.
    pub(crate) fn remove(&mut self, id: SolidId) -> bool {
        let idx = id.0 as usize;
        if let Some(slot) = self.slots.get_mut(idx)
            && slot.is_some()
        {
            *slot = None;
            return true;
        }
        false
    }

    pub(crate) fn replace_solid(
        &mut self,
        id: SolidId,
        model: BRepModel,
        handle: Handle<SolidData>,
        label: impl Into<String>,
    ) -> bool {
        let idx = id.0 as usize;
        if let Some(slot) = self.slots.get_mut(idx)
            && slot.is_some()
        {
            *slot = Some(SolidSlot {
                model,
                handle: Some(handle),
                label: label.into(),
            });
            return true;
        }
        false
    }

    pub(crate) fn get_slot(&self, id: SolidId) -> Option<&SolidSlot> {
        self.slots.get(id.0 as usize).and_then(|s| s.as_ref())
    }

    pub(crate) fn get_slot_mut(&mut self, id: SolidId) -> Option<&mut SolidSlot> {
        self.slots.get_mut(id.0 as usize).and_then(|s| s.as_mut())
    }

    /// Computes a basic health check. An empty result means the document is
    /// well-formed. Phase 2 will extend this to cover sketches, drawings,
    /// and dependency-graph integrity.
    pub fn validate(&self) -> Vec<DocumentIssue> {
        let mut out = Vec::new();
        for (idx, slot) in self.slots.iter().enumerate() {
            let Some(slot) = slot else { continue };
            let id = SolidId(idx as u32);
            let Some(handle) = slot.handle else {
                out.push(DocumentIssue::DanglingSlot(id));
                continue;
            };
            if slot.model.solids.get(handle).is_none() {
                out.push(DocumentIssue::MissingSolid(id));
                continue;
            }
            // Mass-properties failure indicates degenerate topology.
            if solid_mass_properties(&slot.model, handle).is_err() {
                out.push(DocumentIssue::DegenerateSolid(id));
            }
        }
        out
    }

    /// Returns the volume / area / centroid of a solid, if measurable.
    pub fn measure_solid(&self, id: SolidId) -> Option<MeasureSummary> {
        let slot = self.get_slot(id)?;
        let handle = slot.handle?;
        let mp = solid_mass_properties(&slot.model, handle).ok()?;
        Some(MeasureSummary {
            id,
            volume: mp.volume,
            surface_area: mp.surface_area,
            centroid: [mp.centroid.x, mp.centroid.y, mp.centroid.z],
        })
    }

    /// Returns the underlying `(BRepModel, SolidData handle)` pair for a
    /// solid, if it exists. Intended for adapters (export, tessellation)
    /// that need to reach the kernel surface for operations not yet
    /// reachable through [`Command`](crate::Command). Returns `None` if
    /// the slot is missing or has no handle. Read-only — the document's
    /// stored state is unaffected.
    pub fn solid_brep(&self, id: SolidId) -> Option<(&BRepModel, Handle<SolidData>)> {
        let slot = self.get_slot(id)?;
        let handle = slot.handle?;
        Some((&slot.model, handle))
    }

    /// Resolves a persistent edge reference to the current runtime edge handle.
    pub fn resolve_edge_ref(&self, edge_ref: &EdgeRef) -> ApiResult<Handle<EdgeData>> {
        let slot = self
            .get_slot(edge_ref.solid)
            .ok_or_else(|| ApiError::UnknownSolid(format!("{}", edge_ref.solid)))?;
        slot.model.name_map.get_edge(&edge_ref.tag).ok_or_else(|| {
            ApiError::InvalidArgument(format!(
                "Track 4: unknown edge tag in solid {}",
                edge_ref.solid
            ))
        })
    }

    /// Resolves a persistent face reference to the current runtime face handle.
    pub fn resolve_face_ref(&self, face_ref: &FaceRef) -> ApiResult<Handle<FaceData>> {
        let slot = self
            .get_slot(face_ref.solid)
            .ok_or_else(|| ApiError::UnknownSolid(format!("{}", face_ref.solid)))?;
        slot.model.name_map.get_face(&face_ref.tag).ok_or_else(|| {
            ApiError::InvalidArgument(format!(
                "Track 4: unknown face tag in solid {}",
                face_ref.solid
            ))
        })
    }

    /// Returns an owned clone of the underlying `(BRepModel, SolidData handle)`
    /// pair for a solid, if it exists.
    ///
    /// Used by adapters (notably the viewer's A3.2 GuiAction bridge) that need
    /// to hand a fully-owned `BRepModel` to APIs like `add_to_scene(model, ...)`
    /// while keeping the `Session`'s borrow short — calling
    /// `Session::execute(...)` returns an `Outcome`, releases the `&mut self`
    /// borrow, and the caller then calls `session.document().clone_solid_brep(id)`.
    ///
    /// `BRepModel` derives `Clone` (see `crates/topology/src/lib.rs`), so this
    /// is a straightforward deep clone of the topology stores plus the
    /// `Handle<SolidData>` (which is `Copy`). Mutations to the returned model
    /// do not affect the document's stored state.
    pub fn clone_solid_brep(&self, id: SolidId) -> Option<(BRepModel, Handle<SolidData>)> {
        let slot = self.get_slot(id)?;
        let handle = slot.handle?;
        Some((slot.model.clone(), handle))
    }

    /// Returns the axis-aligned bounding box of a solid in world coordinates,
    /// computed by walking the underlying `BRepModel` vertex store. Returns
    /// `None` if the slot is missing, has no handle, or contains no vertices.
    ///
    /// This is intended for AI / test consumers that need a quick spatial
    /// extent check without tessellating the solid — e.g. asserting a
    /// `MidPlane` extrusion is centered on z=0, or that a translation
    /// shifted a part by the expected delta.
    pub fn bounding_box(&self, id: SolidId) -> Option<AabbSummary> {
        let slot = self.get_slot(id)?;
        // Note: we don't strictly need `handle` to compute the bbox, but we
        // require it to match the rest of the Document API contract — a
        // solid without a handle is considered a dangling slot and returns
        // None for measurements.
        let _ = slot.handle?;
        let mut iter = slot.model.vertices.iter();
        let (_h, first) = iter.next()?;
        let mut min = [first.point.x, first.point.y, first.point.z];
        let mut max = min;
        for (_h, v) in iter {
            let p = [v.point.x, v.point.y, v.point.z];
            for axis in 0..3 {
                if p[axis] < min[axis] {
                    min[axis] = p[axis];
                }
                if p[axis] > max[axis] {
                    max[axis] = p[axis];
                }
            }
        }
        Some(AabbSummary { id, min, max })
    }

    /// Content-addressable hash of the document's observable state.
    ///
    /// Returns a `u64` that is stable across `save_cadk` / `load_cadk`
    /// roundtrips: any two documents that produce equal canonical hashes
    /// expose identical solid counts, per-solid metadata (id, label,
    /// rounded AABB, rounded mass properties), and history-event sequence.
    ///
    /// Used by the autosave subsystem (A3.1) to skip writing a snapshot
    /// when the document state has not meaningfully changed since the
    /// last write. Deterministic across runs of the same build — uses
    /// [`std::hash::DefaultHasher`] (currently SipHash-1-3) with a fixed
    /// feed order. The numeric value is **not** stable across Rust
    /// releases that change `DefaultHasher`; only intra-build equality
    /// is guaranteed.
    ///
    /// Rounded to ~1e-9 to absorb the tiny float noise that survives a
    /// JSON serialize / parse roundtrip.
    pub fn canonical_hash(&self) -> u64 {
        let mut h = DefaultHasher::new();
        let ids = self.solid_ids();
        h.write_u64(ids.len() as u64);
        for id in ids {
            h.write_u64(u64::from(id.0));
            if let Some(label) = self.solid_label(id) {
                h.write_u64(label.len() as u64);
                h.write(label.as_bytes());
            } else {
                h.write_u64(u64::MAX);
            }
            if let Some(aabb) = self.bounding_box(id) {
                h.write_u64(1);
                for axis in 0..3 {
                    h.write_u64(quantize_f64(aabb.min[axis]) as u64);
                    h.write_u64(quantize_f64(aabb.max[axis]) as u64);
                }
            } else {
                h.write_u64(0);
            }
            if let Some(mp) = self.measure_solid(id) {
                h.write_u64(1);
                h.write_u64(quantize_f64(mp.volume) as u64);
                h.write_u64(quantize_f64(mp.surface_area) as u64);
                for axis in 0..3 {
                    h.write_u64(quantize_f64(mp.centroid[axis]) as u64);
                }
            } else {
                h.write_u64(0);
            }
        }
        h.write_u64(self.history.len() as u64);
        for event in &self.history {
            h.write_u64(event.op.len() as u64);
            h.write(event.op.as_bytes());
            match event.primary {
                Some(id) => {
                    h.write_u64(1);
                    h.write_u64(u64::from(id.0));
                }
                None => h.write_u64(0),
            }
            h.write_u64(event.description.len() as u64);
            h.write(event.description.as_bytes());
        }
        h.write_u64(self.body_count() as u64);
        for body in self.bodies.iter().flatten() {
            h.write_u64(body.id);
            h.write_u64(body.name.len() as u64);
            h.write(body.name.as_bytes());
            h.write_u64(body.features.len() as u64);
            for feature in &body.features {
                h.write_u64(feature.feature_id);
                h.write_u64(if feature.suppressed { 1 } else { 0 });
                h.write_u64(feature.spec_kind.len() as u64);
                h.write(feature.spec_kind.as_bytes());
            }
            h.write_u64(body.tip.map(|tip| tip as u64).unwrap_or(u64::MAX));
        }
        h.write_u64(self.sketch_count() as u64);
        for sketch in self.sketches.iter().flatten() {
            h.write_u64(sketch.id.0);
            h.write_u64(sketch.name.len() as u64);
            h.write(sketch.name.as_bytes());
            for value in sketch.plane.origin {
                h.write_u64(quantize_f64(value) as u64);
            }
            for value in sketch.plane.normal {
                h.write_u64(quantize_f64(value) as u64);
            }
            h.write_u64(sketch.entities.len() as u64);
            h.write_u64(sketch.constraints.len() as u64);
        }
        h.finish()
    }
}

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn fnv_write_bytes(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn fnv_write_u64(hash: u64, value: u64) -> u64 {
    fnv_write_bytes(hash, &value.to_le_bytes())
}

/// Persisted 2D sketch stored in the API document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistedSketch {
    pub id: SketchId,
    pub name: String,
    pub plane: Plane,
    pub entities: Vec<SketchEntity>,
    pub constraints: Vec<SketchConstraint>,
}

impl PersistedSketch {
    pub fn new(name: impl Into<String>, plane: Plane) -> Self {
        Self {
            id: SketchId(0),
            name: name.into(),
            plane,
            entities: Vec::new(),
            constraints: Vec::new(),
        }
    }
}

/// Quantise an `f64` to ~1e-9 resolution so canonical-hash equality
/// survives a JSON serialize/parse roundtrip. Negative zero collapses
/// to zero; NaN collapses to a fixed sentinel.
#[inline]
fn quantize_f64(x: f64) -> i64 {
    if x.is_nan() {
        i64::MIN
    } else {
        (x * 1.0e9).round() as i64
    }
}

/// A document-level health issue surfaced by [`Document::validate`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum DocumentIssue {
    /// Slot is occupied but has no solid handle.
    DanglingSlot(SolidId),
    /// Slot's handle no longer resolves to a stored solid.
    MissingSolid(SolidId),
    /// Solid's mass properties cannot be computed (degenerate geometry).
    DegenerateSolid(SolidId),
}

/// Lightweight measurement summary returned by [`Document::measure_solid`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeasureSummary {
    pub id: SolidId,
    pub volume: f64,
    pub surface_area: f64,
    /// `[x, y, z]` centroid in world coordinates.
    pub centroid: [f64; 3],
}

/// Axis-aligned bounding box returned by [`Document::bounding_box`]. Both
/// corners are in world coordinates and `min[i] <= max[i]` is guaranteed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AabbSummary {
    pub id: SolidId,
    pub min: [f64; 3],
    pub max: [f64; 3],
}

impl AabbSummary {
    /// `[dx, dy, dz]` size along each axis.
    pub fn size(&self) -> [f64; 3] {
        [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ]
    }

    /// Geometric center (midpoint of `min` and `max`).
    pub fn center(&self) -> [f64; 3] {
        [
            (self.min[0] + self.max[0]) * 0.5,
            (self.min[1] + self.max[1]) * 0.5,
            (self.min[2] + self.max[2]) * 0.5,
        ]
    }
}

/// One entry in [`Document::history`]. Recorded by [`Session`](crate::Session)
/// after every successful command. Kept lightweight — sufficient to render a
/// history tree without re-running the kernel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryEvent {
    /// `Command::op_name()` of the command that produced this event.
    pub op: String,
    /// Primary solid the event affected, if any.
    pub primary: Option<SolidId>,
    /// Human-readable description (label or short summary).
    pub description: String,
    /// Stable identifier assigned by [`Document::push_history`]. Foundation
    /// for the A2 feature-based command surface. Older `.cadk` files
    /// missing the field deserialise with `FeatureId(0)` (sentinel) and
    /// receive fresh IDs the moment they replay through the session.
    /// Excluded from [`Document::canonical_hash`] so its addition does
    /// not invalidate pre-A2.1 fixtures.
    #[serde(default)]
    pub feature_id: FeatureId,
}
