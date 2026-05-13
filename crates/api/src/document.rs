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

use cadkernel_modeling::measure::solid_mass_properties;
use cadkernel_topology::{BRepModel, Handle, SolidData};
use serde::{Deserialize, Serialize};
use std::hash::{DefaultHasher, Hasher};

/// Stable, opaque identifier for a solid inside a [`Document`].
///
/// IDs are monotonically increasing and never reused. Persistent across
/// save/open via the JSON representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

/// The model container.
#[derive(Default)]
pub struct Document {
    slots: Vec<Option<SolidSlot>>,
    next_id: u32,
    history: Vec<HistoryEvent>,
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

    pub(crate) fn push_history(&mut self, event: HistoryEvent) {
        self.history.push(event);
    }

    #[allow(dead_code)]
    pub(crate) fn clear_history(&mut self) {
        self.history.clear();
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
        h.finish()
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
}
