//! Outcome — typed result of executing a [`Command`](crate::Command).

use serde::{Deserialize, Serialize};

use crate::document::{DocumentIssue, HistoryEvent, SolidId};

/// Successful result of [`Session::execute`](crate::Session::execute).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Outcome {
    /// A new solid was created.
    SolidCreated { id: SolidId, label: String },
    /// A solid was deleted.
    SolidDeleted { id: SolidId },
    /// A boolean operation produced one new solid and consumed the inputs.
    Booleaned {
        result: SolidId,
        consumed: Vec<SolidId>,
    },
    /// A solid was modified in place.
    SolidModified { id: SolidId },
    /// A pattern (linear/mirror) produced one or more new solids; the
    /// original was preserved.
    ///
    /// - `pattern_id`: the source solid the pattern was generated from.
    /// - `instance_count`: total instances including the original.
    /// - `total_features`: number of newly inserted solids
    ///   (`instance_count - 1`, i.e. excluding the original).
    /// - `ids`: every solid in the pattern, with the original at index 0
    ///   and new instances appended in creation order.
    PatternCreated {
        pattern_id: SolidId,
        instance_count: u32,
        total_features: u32,
        ids: Vec<SolidId>,
    },
    /// The document was reset (`Command::NewDocument`).
    DocumentReset,
    /// Read-only measurement of a solid (`Command::Measure`). Carries the
    /// volume / surface area / centroid / axis-aligned bounding box —
    /// callers branch on this when scripting AI/test workflows that need
    /// to assert on geometry without separately calling the Document API.
    Measured {
        id: SolidId,
        volume: f64,
        surface_area: f64,
        centroid: [f64; 3],
        bbox_min: [f64; 3],
        bbox_max: [f64; 3],
    },
    /// Read-only document health report (`Command::Validate`). `issues`
    /// is empty when the document is clean. AI / test consumers branch
    /// on `issues.is_empty()` to decide whether to surface a warning.
    Validated {
        issues: Vec<DocumentIssue>,
    },
    /// Read-only enumeration of every solid in the document
    /// (`Command::ListSolids`). Pairs each `SolidId` with its label so
    /// AI / test consumers can render a tree view or pick targets for
    /// follow-up commands without juggling `Document::solid_ids()` and
    /// `Document::solid_label()` separately.
    SolidsListed {
        entries: Vec<SolidEntry>,
    },
    /// Read-only history dump (`Command::HistoryEvents`). Returns the
    /// list of events recorded by every previously-executed mutating
    /// command in execution order. Equivalent to
    /// `session.document().history().to_vec()` but available through
    /// the command surface so AI / scripts can introspect history
    /// without touching the `Document` API directly.
    HistoryListed {
        events: Vec<HistoryEvent>,
    },
    /// Read-only document statistics (`Command::Stats`). Reports the
    /// number of populated solid slots and the total recorded history
    /// event count. Cheap to compute (no mesh / volume traversal).
    Stats {
        solid_count: u32,
        history_count: u32,
    },
    /// Read-only axis-aligned bounding box (`Command::Bounds`). Cheap
    /// observer that skips the volume / surface-area / centroid pass
    /// performed by `Measure`. Coordinates are in world space and
    /// `min[i] <= max[i]` is guaranteed.
    Bounds {
        id: SolidId,
        min: [f64; 3],
        max: [f64; 3],
    },
    /// Read-only centroid-to-centroid distance between two solids
    /// (`Command::Distance`). Returns the Euclidean distance plus the
    /// per-axis delta `b - a` for downstream calculations (AI / scripts
    /// often want both magnitude and direction).
    Distance {
        id_a: SolidId,
        id_b: SolidId,
        distance: f64,
        delta: [f64; 3],
    },
    /// Read-only single-scalar volume of a solid (`Command::Volume`).
    /// Lighter than `Measure` when only the volume is needed (skips the
    /// surface-area / centroid / bbox traversal).
    Volume { id: SolidId, volume: f64 },
    /// Read-only single-scalar surface area of a solid
    /// (`Command::SurfaceArea`). Lighter than `Measure` when only the
    /// surface area is needed.
    SurfaceArea { id: SolidId, surface_area: f64 },
    /// Read-only centroid of a solid (`Command::Centroid`). Lighter
    /// than `Measure` when only the centroid is needed (skips
    /// surface-area / volume / bbox traversal).
    Centroid { id: SolidId, centroid: [f64; 3] },
    /// Read-only AABB-overlap predicate (`Command::IntersectsAabb`).
    /// Cheap broad-phase collision check using only the world-space
    /// bounding boxes of two solids. `intersects = true` iff every
    /// axis interval overlaps (touching counts). When intersecting,
    /// `overlap_min` / `overlap_max` carry the intersection AABB;
    /// otherwise they hold zero-length intervals at `[0,0,0]`.
    AabbIntersection {
        id_a: SolidId,
        id_b: SolidId,
        intersects: bool,
        overlap_min: [f64; 3],
        overlap_max: [f64; 3],
    },
    /// Read-only solid-existence predicate (`Command::Exists`).
    /// Never errors on missing ids — returns `exists = false` instead.
    /// Cheap probe AI / scripts use to test whether an id is still
    /// valid without paying for `UnknownSolid` exception handling.
    Exists { id: SolidId, exists: bool },
    /// Read-only AABB-diagonal length of a solid (`Command::Diagonal`).
    /// Returns `length(bbox.max - bbox.min)` together with the raw
    /// per-axis extents `[dx, dy, dz]`. Cheap heuristic used by
    /// camera-fit, level-of-detail thresholds, and tolerance scaling.
    Diagonal { id: SolidId, length: f64, extents: [f64; 3] },
    /// Read-only AABB center of a solid (`Command::AabbCenter`).
    /// Returns `(bbox.min + bbox.max) * 0.5`. Distinct from `Centroid`
    /// (which is the mass centroid). Useful for placement, grid
    /// snapping, and gizmo positioning.
    AabbCenter { id: SolidId, center: [f64; 3] },
    /// Read-only AABB volume of a solid (`Command::AabbVolume`).
    /// Returns `dx * dy * dz` (product of per-axis extents). Cheap
    /// upper bound on the solid's volume; useful for LOD heuristics
    /// and proportional thresholds without paying for the mass-volume
    /// traversal performed by `Volume` / `Measure`.
    AabbVolume { id: SolidId, volume: f64 },
    /// Nothing happened (`Command::Noop`).
    Empty,
}

/// One row in [`Outcome::SolidsListed`]. Keeps the wire format flat for
/// JSON consumers (`{"id": 0, "label": "Box"}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolidEntry {
    pub id: SolidId,
    pub label: String,
}

/// Coarse-grained tag, useful for AI/test branching without pattern matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeKind {
    SolidCreated,
    SolidDeleted,
    Booleaned,
    SolidModified,
    PatternCreated,
    DocumentReset,
    Measured,
    Validated,
    SolidsListed,
    HistoryListed,
    Stats,
    Bounds,
    Distance,
    Volume,
    SurfaceArea,
    Centroid,
    AabbIntersection,
    Exists,
    Diagonal,
    AabbCenter,
    AabbVolume,
    Empty,
}

impl Outcome {
    /// Returns the coarse-grained tag for this outcome.
    pub fn kind(&self) -> OutcomeKind {
        match self {
            Self::SolidCreated { .. } => OutcomeKind::SolidCreated,
            Self::SolidDeleted { .. } => OutcomeKind::SolidDeleted,
            Self::Booleaned { .. } => OutcomeKind::Booleaned,
            Self::SolidModified { .. } => OutcomeKind::SolidModified,
            Self::PatternCreated { .. } => OutcomeKind::PatternCreated,
            Self::DocumentReset => OutcomeKind::DocumentReset,
            Self::Measured { .. } => OutcomeKind::Measured,
            Self::Validated { .. } => OutcomeKind::Validated,
            Self::SolidsListed { .. } => OutcomeKind::SolidsListed,
            Self::HistoryListed { .. } => OutcomeKind::HistoryListed,
            Self::Stats { .. } => OutcomeKind::Stats,
            Self::Bounds { .. } => OutcomeKind::Bounds,
            Self::Distance { .. } => OutcomeKind::Distance,
            Self::Volume { .. } => OutcomeKind::Volume,
            Self::SurfaceArea { .. } => OutcomeKind::SurfaceArea,
            Self::Centroid { .. } => OutcomeKind::Centroid,
            Self::AabbIntersection { .. } => OutcomeKind::AabbIntersection,
            Self::Exists { .. } => OutcomeKind::Exists,
            Self::Diagonal { .. } => OutcomeKind::Diagonal,
            Self::AabbCenter { .. } => OutcomeKind::AabbCenter,
            Self::AabbVolume { .. } => OutcomeKind::AabbVolume,
            Self::Empty => OutcomeKind::Empty,
        }
    }

    /// Returns the primary [`SolidId`] introduced by this outcome, if any.
    pub fn primary_id(&self) -> Option<SolidId> {
        match self {
            Self::SolidCreated { id, .. } => Some(*id),
            Self::SolidDeleted { id } => Some(*id),
            Self::Booleaned { result, .. } => Some(*result),
            Self::SolidModified { id } => Some(*id),
            Self::PatternCreated { ids, .. } => ids.first().copied(),
            Self::Measured { id, .. } => Some(*id),
            _ => None,
        }
    }
}
