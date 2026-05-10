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
    /// Read-only AABB-containment predicate (`Command::ContainsAabb`).
    /// `contains = true` iff the bounding box of `id_outer` fully
    /// covers the bounding box of `id_inner` (closed intervals;
    /// touching faces count as contained). Cheap broad-phase test
    /// that avoids the per-face boolean / SAT machinery of the full
    /// containment check.
    AabbContainment {
        id_outer: SolidId,
        id_inner: SolidId,
        contains: bool,
    },
    /// Read-only AABB corner enumeration (`Command::AabbCorners`).
    /// Returns the eight corner points of the world-space bounding
    /// box in canonical order: `(min,min,min)`, `(max,min,min)`,
    /// `(min,max,min)`, `(max,max,min)`, `(min,min,max)`,
    /// `(max,min,max)`, `(min,max,max)`, `(max,max,max)`. Useful
    /// for camera-fit framing, debug-visualization wireframes, and
    /// seeding broad-phase intersection setups.
    AabbCorners { id: SolidId, corners: [[f64; 3]; 8] },
    /// Read-only label of a single solid (`Command::SolidLabel`).
    /// Cheap observer that returns just the label string for one
    /// id, without the `Vec<SolidEntry>` allocation `ListSolids`
    /// would produce. Useful for AI/scripts that already know the
    /// id and only need its display name.
    SolidLabel { id: SolidId, label: String },
    /// Read-only document-emptiness predicate (`Command::IsEmpty`).
    /// `is_empty = true` iff `solid_count() == 0`. Cheaper than
    /// `Stats` when the consumer only needs the boolean (skips the
    /// history-length field).
    IsEmpty { is_empty: bool },
    /// Read-only AABB surface area of a solid
    /// (`Command::AabbSurfaceArea`). Returns
    /// `2 * (dx*dy + dy*dz + dz*dx)` where `dx, dy, dz` are the
    /// per-axis bounding-box extents. Cheap upper bound on the
    /// solid's surface area; useful for LOD heuristics and
    /// proportional thresholds without paying for the per-face
    /// traversal performed by `SurfaceArea` / `Measure`.
    AabbSurfaceArea { id: SolidId, surface_area: f64 },
    /// Read-only solid-count observer (`Command::SolidCount`).
    /// Returns just the populated-slot count as a `u32`. Cheaper
    /// than `Stats` when the consumer only needs the count (skips
    /// the history-length field).
    SolidCount { count: u32 },
    /// Read-only history-count observer (`Command::HistoryCount`).
    /// Returns the number of recorded history events as a `u32`.
    /// Cheaper than `Stats` when only the history length is needed.
    HistoryCount { count: u32 },
    /// Read-only label-existence predicate (`Command::HasLabel`).
    /// `has_label = true` iff at least one solid's label matches
    /// the query (case-insensitive substring). Cheaper than
    /// `FindByLabel` when only the boolean is needed.
    HasLabel { query: String, has_label: bool },
    /// Read-only solid-id enumeration (`Command::SolidIds`).
    /// Returns just the populated `SolidId`s as a `Vec<SolidId>`,
    /// without labels. Cheaper than `ListSolids` when the consumer
    /// only needs ids (skips per-slot label cloning).
    SolidIds { ids: Vec<SolidId> },
    /// Read-only AABB-extents observer (`Command::AabbExtents`).
    /// Returns the per-axis bounding-box extents `[dx, dy, dz]`
    /// where `dx = max.x - min.x` etc. Cheaper than `Bounds`
    /// (skips the min/max points) and a raw companion to
    /// `Diagonal` (which returns `sqrt(dx*dx + dy*dy + dz*dz)`).
    AabbExtents { id: SolidId, extents: [f64; 3] },
    /// Read-only AABB longest-axis observer (`Command::AabbLongestAxis`).
    /// Returns the axis index (`0` = X, `1` = Y, `2` = Z) whose
    /// bounding-box extent is largest. Ties go to the lowest
    /// index (X over Y over Z). Useful for orientation heuristics
    /// and primary-axis detection without manual `[dx, dy, dz]`
    /// inspection.
    AabbLongestAxis { id: SolidId, axis: u8 },
    /// Read-only AABB shortest-axis observer (`Command::AabbShortestAxis`).
    /// Returns the axis index (`0` = X, `1` = Y, `2` = Z) whose
    /// bounding-box extent is smallest. Ties go to the lowest
    /// index. Symmetric counterpart to `AabbLongestAxis`; useful
    /// for thinness/sliver detection.
    AabbShortestAxis { id: SolidId, axis: u8 },
    /// Read-only AABB aspect-ratio observer (`Command::AabbAspectRatio`).
    /// Returns `longest_extent / shortest_extent` (>= 1.0). Returns
    /// `f64::INFINITY` when the shortest extent is exactly zero
    /// (degenerate AABB). Useful for slenderness/sliver detection.
    AabbAspectRatio { id: SolidId, ratio: f64 },
    /// Read-only cubic-AABB predicate (`Command::IsCubic`). `cubic`
    /// is `true` when all three bounding-box extents are equal
    /// within an absolute tolerance of `1e-9`. Useful for quick
    /// shape-classification heuristics.
    IsCubic { id: SolidId, cubic: bool },
    /// Read-only square-XY-footprint predicate (`Command::IsSquareXy`).
    /// `square` is `true` when the X and Z extents of the bounding
    /// box are equal within an absolute tolerance of `1e-9` (Z is
    /// unconstrained). Detects solids with a square footprint in
    /// the XY plane (e.g. square prisms / pillars / posts).
    IsSquareXy { id: SolidId, square: bool },
    /// Read-only single-event history lookup (`Command::HistoryDescription`).
    /// Returns the `description` string of the history event at the
    /// given index. Cheaper than `HistoryEvents` when the caller only
    /// needs one entry (e.g. tooltip rendering).
    HistoryDescription { index: u32, description: String },
    /// Read-only square-YZ-footprint predicate (`Command::IsSquareYz`).
    /// `square` is `true` when the Y and Z extents of the bounding
    /// box are equal within an absolute tolerance of `1e-9` (X is
    /// unconstrained). Detects solids with a square cross-section
    /// in the YZ plane (e.g. extrusions oriented along the X axis).
    IsSquareYz { id: SolidId, square: bool },
    /// Read-only square-XZ-footprint predicate (`Command::IsSquareXz`).
    /// `square` is `true` when the X and Z extents of the bounding
    /// box are equal within an absolute tolerance of `1e-9` (Y is
    /// unconstrained). Detects solids with a square cross-section
    /// in the XZ plane (e.g. extrusions oriented along the Y axis).
    IsSquareXz { id: SolidId, square: bool },
    /// Read-only history op-occurrence counter (`Command::OperationCount`).
    /// Returns the number of history events whose `op` field matches
    /// the queried `op_name` exactly (case-sensitive). Useful for
    /// introspection (“how many `create_box` calls have run?”) without
    /// scanning the full history vector manually.
    OperationCount { op_name: String, count: u32 },
    /// Read-only last-history-event lookup (`Command::LastOperation`).
    /// Returns the `op_name`, `description`, and 0-based `index` of
    /// the most recent history event. Equivalent to
    /// `HistoryDescription { index: HistoryCount - 1 }` plus the
    /// op name, but in a single call. Errors with `InvalidArgument`
    /// when the history is empty.
    LastOperation {
        index: u32,
        op_name: String,
        description: String,
    },
    /// Read-only history op-presence predicate (`Command::HasOperation`).
    /// `present` is `true` when at least one history event's `op`
    /// field matches the queried `op_name` exactly (case-sensitive).
    /// Lighter than `OperationCount` when callers only need a
    /// boolean answer.
    HasOperation { op_name: String, present: bool },
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
    AabbContainment,
    AabbCorners,
    SolidLabel,
    IsEmpty,
    AabbSurfaceArea,
    SolidCount,
    HistoryCount,
    HasLabel,
    SolidIds,
    AabbExtents,
    AabbLongestAxis,
    AabbShortestAxis,
    AabbAspectRatio,
    IsCubic,
    IsSquareXy,
    HistoryDescription,
    IsSquareYz,
    IsSquareXz,
    OperationCount,
    LastOperation,
    HasOperation,
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
            Self::AabbContainment { .. } => OutcomeKind::AabbContainment,
            Self::AabbCorners { .. } => OutcomeKind::AabbCorners,
            Self::SolidLabel { .. } => OutcomeKind::SolidLabel,
            Self::IsEmpty { .. } => OutcomeKind::IsEmpty,
            Self::AabbSurfaceArea { .. } => OutcomeKind::AabbSurfaceArea,
            Self::SolidCount { .. } => OutcomeKind::SolidCount,
            Self::HistoryCount { .. } => OutcomeKind::HistoryCount,
            Self::HasLabel { .. } => OutcomeKind::HasLabel,
            Self::SolidIds { .. } => OutcomeKind::SolidIds,
            Self::AabbExtents { .. } => OutcomeKind::AabbExtents,
            Self::AabbLongestAxis { .. } => OutcomeKind::AabbLongestAxis,
            Self::AabbShortestAxis { .. } => OutcomeKind::AabbShortestAxis,
            Self::AabbAspectRatio { .. } => OutcomeKind::AabbAspectRatio,
            Self::IsCubic { .. } => OutcomeKind::IsCubic,
            Self::IsSquareXy { .. } => OutcomeKind::IsSquareXy,
            Self::HistoryDescription { .. } => OutcomeKind::HistoryDescription,
            Self::IsSquareYz { .. } => OutcomeKind::IsSquareYz,
            Self::IsSquareXz { .. } => OutcomeKind::IsSquareXz,
            Self::OperationCount { .. } => OutcomeKind::OperationCount,
            Self::LastOperation { .. } => OutcomeKind::LastOperation,
            Self::HasOperation { .. } => OutcomeKind::HasOperation,
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
