//! Command — the serializable action enum.
//!
//! Every state-mutating operation in CADKernel funnels through this enum.
//! The variants are intentionally small and explicit so that:
//!
//! 1. The JSON representation is stable and self-documenting (one variant
//!    name per operation, named fields).
//! 2. AI agents can pick from a short list backed by JSON schemas.
//! 3. Tests can build deterministic command logs.
//!
//! Phase 1 ships a starter set of 14 commands covering primitives, booleans,
//! basic transforms, and lifecycle. Subsequent phases extend the enum;
//! adding a variant is non-breaking for existing replay logs.

use serde::{Deserialize, Serialize};

use crate::document::{FeatureId, SolidId};

/// Every state-mutating operation a [`Session`](crate::Session) accepts.
///
/// Variants are tagged in JSON via the field `op`, matching the snake_case
/// convention used by `cadkernel-io::mcp`. Field names match the kernel
/// parameters they are forwarded to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Command {
    /// Create an axis-aligned box at the origin.
    CreateBox { dx: f64, dy: f64, dz: f64 },
    /// Create a cylinder at the origin (axis along Z).
    CreateCylinder { radius: f64, height: f64 },
    /// Create a UV-sphere at the origin.
    CreateSphere { radius: f64 },
    /// Create a cone or frustum (cone with non-zero top) at the origin,
    /// axis along Z. `radius` is the base radius (must be > 0).
    /// `top_radius` defaults to `0.0` for a pointed cone; values > 0
    /// produce a truncated cone (frustum). The default keeps existing
    /// serialized commands (`{"op":"create_cone","radius":...,"height":...}`)
    /// deserializing unchanged as pure cones.
    CreateCone {
        radius: f64,
        height: f64,
        #[serde(default)]
        top_radius: f64,
    },
    /// Create a torus at the origin in the XY plane.
    CreateTorus {
        major_radius: f64,
        minor_radius: f64,
    },
    /// Boolean union of two existing solids. Both are removed; the result is
    /// stored under a fresh [`SolidId`].
    BooleanUnion { lhs: SolidId, rhs: SolidId },
    /// Boolean subtraction `lhs - rhs`. Both are removed; result stored fresh.
    BooleanSubtract { lhs: SolidId, rhs: SolidId },
    /// Boolean intersection. Both are removed; result stored fresh.
    BooleanIntersect { lhs: SolidId, rhs: SolidId },
    /// Translate a solid by `(dx, dy, dz)`.
    Translate {
        id: SolidId,
        dx: f64,
        dy: f64,
        dz: f64,
    },
    /// Uniformly scale a solid about its centroid.
    Scale { id: SolidId, factor: f64 },
    /// Non-uniform per-axis scale of a solid about an explicit pivot
    /// `point`. `factors = [sx, sy, sz]` are the per-axis multipliers;
    /// every component must be > 0. Vertex positions are rewritten;
    /// topology is preserved.
    ScaleNonUniform {
        id: SolidId,
        factors: [f64; 3],
        point: [f64; 3],
    },
    /// Translate a solid so that its centroid lands on the world
    /// origin. Convenience for AI / scripts that want to re-centre an
    /// imported part without first measuring its centroid. Equivalent
    /// to `Translate` by `-centroid` but expressed as a single command.
    CenterOnOrigin { id: SolidId },
    /// Translate `id` so that its centroid coincides with the centroid
    /// of `target_id`. Two-solid alignment convenience for AI / scripts
    /// that want to snap one part to another without first measuring
    /// either centroid. Equivalent to `Translate` by
    /// `target_centroid - source_centroid` but expressed as a single
    /// command. The target slot is read-only.
    AlignTo { id: SolidId, target_id: SolidId },
    /// Uniformly scale a solid about its centroid so that the largest
    /// axis-aligned bounding-box extent equals `target_size`. Convenience
    /// for AI / scripts that want to normalise an imported part to a
    /// canonical size without first measuring its bbox. `target_size`
    /// must be > 0; degenerate (zero-extent) bboxes are rejected.
    ScaleToFit { id: SolidId, target_size: f64 },
    /// Translate `id` so that its centroid coincides with `point`
    /// (in world coordinates). Generalisation of `CenterOnOrigin`
    /// (which is `TranslateTo` `[0, 0, 0]`) and `AlignTo`
    /// (which is `TranslateTo target.centroid`). Equivalent to
    /// `Translate` by `point - centroid` but expressed as a single
    /// command — saves AI / scripts an explicit `Measure` round-trip.
    TranslateTo { id: SolidId, point: [f64; 3] },
    /// Rename a solid (does not change its [`SolidId`]).
    Rename { id: SolidId, label: String },
    /// Delete a solid.
    DeleteSolid { id: SolidId },
    /// Extrude a planar polygonal profile along a direction by `distance`.
    /// `profile` is a list of `[x, y, z]` points forming a closed polygon
    /// (≥ 3 points, no duplicate closing point).
    ///
    /// `kind` selects the extrusion mode (defaults to `Blind`, which matches
    /// pre-A2.1 behavior). `MidPlane` and `TwoSided` use `distance` as the
    /// total span and reposition the profile accordingly. `ThroughAll` and
    /// `UpToFace` compute the span from existing document geometry.
    Extrude {
        profile: Vec<[f64; 3]>,
        direction: [f64; 3],
        distance: f64,
        #[serde(default)]
        kind: ExtrudeKind,
    },
    /// Linear pattern: produces `count` copies of `id` (including the
    /// original) at `spacing` intervals along `direction`. The original is
    /// preserved; new solids get fresh `SolidId`s.
    ///
    /// `skip_instances` lets callers suppress specific instance indices
    /// (0 = original, 1..count-1 = copies). Indices outside the valid
    /// range are ignored. When `features` is non-empty, `id` is ignored
    /// and each referenced feature's primary solid is patterned.
    LinearPattern {
        id: SolidId,
        direction: [f64; 3],
        spacing: f64,
        count: u32,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        skip_instances: Vec<u32>,
        /// A2.3 feature-list mode. When non-empty, `id` is ignored and the
        /// dispatcher patterns every solid produced by the listed features.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        features: Vec<FeatureId>,
        /// When true, every odd-indexed copied instance is reflected about
        /// the plane through that instance's spacing-derived position with
        /// normal equal to `direction`.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        mirror_alternate: bool,
        /// Per-instance suppression or position adjustment.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        instance_overrides: Vec<InstanceOverride>,
    },
    /// Mirror a solid (or a list of features) across a plane. Produces new
    /// solids; the originals are preserved unless `merge` is true, in which
    /// case each original and its mirrored copy are fused via boolean union
    /// and the source slots are consumed (matches FreeCAD/SolidWorks
    /// “mirror with merge” / PartDesign Mirrored feature behaviour).
    ///
    /// When `features` is non-empty (A2.2 mode), `id` is ignored and the
    /// dispatcher mirrors every solid produced by the listed feature ids.
    /// When `features` is empty (legacy / pre-A2.2 path), only the single
    /// solid `id` is mirrored.
    Mirror {
        id: SolidId,
        point: [f64; 3],
        normal: [f64; 3],
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        merge: bool,
        /// A2.2 feature-list mode. When non-empty, `id` is ignored and the
        /// dispatcher mirrors every solid produced by the listed
        /// [`FeatureId`]s. `#[serde(default)]` so pre-A2.2 fixtures (no
        /// `features` field) deserialise with an empty `Vec` and fall
        /// through to the legacy single-solid path.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        features: Vec<FeatureId>,
    },
    /// Create a freshly-named empty document. Discards every existing solid
    /// and resets the [`Session`] log. Useful as the first command of a
    /// replay test or AI session reset.
    NewDocument,
    /// Read-only measurement of a solid. Returns volume / surface area /
    /// centroid / axis-aligned bounding box wrapped in `Outcome::Measured`.
    /// Does not mutate the document or append a history event — the only
    /// command in the surface that is a pure observer.
    Measure { id: SolidId },
    /// Read-only document health check. Returns the list of
    /// `DocumentIssue`s wrapped in `Outcome::Validated` (empty when the
    /// document is clean). Does not mutate the document or append a
    /// history event — second pure-observer command alongside `Measure`.
    Validate,
    /// Read-only enumeration of every solid in the document. Returns
    /// `Outcome::SolidsListed { entries: [{ id, label }, ...] }`. Does
    /// not mutate the document or append a history event.
    ListSolids,
    /// Read-only label-based lookup. Returns
    /// `Outcome::SolidsListed { entries }` containing every solid whose
    /// label contains `query` (case-insensitive substring match).
    /// An empty `query` matches every solid (equivalent to `ListSolids`).
    /// Does not mutate the document or append a history event.
    FindByLabel { query: String },
    /// Read-only history dump. Returns
    /// `Outcome::HistoryListed { events }` with every recorded
    /// `HistoryEvent` in execution order. Equivalent to
    /// `session.document().history().to_vec()` but available through
    /// the command surface so AI/scripts can introspect history with a
    /// single dispatch. Does not mutate or append a history event.
    HistoryEvents,
    /// Read-only document statistics. Returns
    /// `Outcome::Stats { solid_count, history_count }`. Cheap O(1)
    /// counts — no mesh / volume traversal.
    Stats,
    /// Read-only axis-aligned bounding box of a solid. Returns
    /// `Outcome::Bounds { id, min, max }`. Cheap observer that skips
    /// the volume / surface-area / centroid pass performed by
    /// `Measure` — use this when only the AABB is needed (e.g.
    /// frustum culling, layout, snapping). Does not mutate or
    /// append a history event.
    Bounds { id: SolidId },
    /// Read-only centroid-to-centroid distance between two solids.
    /// Returns `Outcome::Distance { id_a, id_b, distance, delta }`
    /// where `delta = centroid_b - centroid_a`. Convenience for AI /
    /// scripts that need both magnitude and direction without two
    /// separate `Measure` calls. Does not mutate or append a history
    /// event.
    Distance { id_a: SolidId, id_b: SolidId },
    /// Read-only single-scalar volume of a solid. Returns
    /// `Outcome::Volume { id, volume }`. Lighter than `Measure` when
    /// only the volume is needed (skips surface-area / centroid /
    /// bbox traversal). Does not mutate or append a history event.
    Volume { id: SolidId },
    /// Read-only single-scalar surface area of a solid. Returns
    /// `Outcome::SurfaceArea { id, surface_area }`. Lighter than
    /// `Measure` when only the surface area is needed. Does not
    /// mutate or append a history event.
    SurfaceArea { id: SolidId },
    /// Read-only centroid of a solid. Returns
    /// `Outcome::Centroid { id, centroid }`. Lighter than `Measure`
    /// when only the centroid is needed. Does not mutate or append a
    /// history event.
    Centroid { id: SolidId },
    /// Read-only AABB-overlap predicate between two solids. Returns
    /// `Outcome::AabbIntersection { id_a, id_b, intersects, overlap_min, overlap_max }`.
    /// Cheap broad-phase collision check that only consults the
    /// world-space bounding boxes; touching boxes count as
    /// intersecting. Does not mutate or append a history event.
    IntersectsAabb { id_a: SolidId, id_b: SolidId },
    /// Read-only solid-existence predicate. Returns
    /// `Outcome::Exists { id, exists }` and **never errors** on a
    /// missing id — missing ids report `exists = false`. Cheap probe
    /// AI / scripts use to test ids without paying for `UnknownSolid`
    /// exception handling. Does not mutate or append a history event.
    Exists { id: SolidId },
    /// Read-only AABB-diagonal length of a solid. Returns
    /// `Outcome::Diagonal { id, length, extents }` where
    /// `extents = bbox.max - bbox.min` and `length = ||extents||`.
    /// Cheap heuristic for camera-fit, LOD thresholds, and tolerance
    /// scaling. Does not mutate or append a history event.
    Diagonal { id: SolidId },
    /// Read-only AABB center of a solid. Returns
    /// `Outcome::AabbCenter { id, center }` where
    /// `center = (bbox.min + bbox.max) * 0.5`. Distinct from
    /// `Centroid` (mass centroid). Useful for placement, grid
    /// snapping, and gizmo positioning. Does not mutate or append a
    /// history event.
    AabbCenter { id: SolidId },
    /// Read-only AABB volume of a solid. Returns
    /// `Outcome::AabbVolume { id, volume }` where
    /// `volume = dx * dy * dz` (product of per-axis extents). Cheap
    /// upper bound on mass volume; useful for LOD heuristics and
    /// proportional thresholds without paying for the mass-volume
    /// traversal performed by `Volume` / `Measure`. Does not mutate
    /// or append a history event.
    AabbVolume { id: SolidId },
    /// Read-only AABB-containment predicate. Returns
    /// `Outcome::AabbContainment { id_outer, id_inner, contains }`
    /// where `contains = true` iff the bounding box of `id_outer`
    /// fully covers the bounding box of `id_inner` (closed
    /// intervals; touching faces count as contained). Cheap
    /// broad-phase containment test using only the world-space
    /// AABBs of both solids.
    ContainsAabb {
        id_outer: SolidId,
        id_inner: SolidId,
    },
    /// Read-only AABB corner enumeration. Returns
    /// `Outcome::AabbCorners { id, corners }` where `corners`
    /// holds the eight bounding-box corner points in canonical
    /// order (low→high in x, then y, then z). Useful for
    /// camera-fit framing, debug-visualization wireframes, and
    /// seeding broad-phase intersection setups. Does not mutate
    /// or append a history event.
    AabbCorners { id: SolidId },
    /// Read-only label of a single solid. Returns
    /// `Outcome::SolidLabel { id, label }` where `label` is a
    /// freshly-allocated copy of the slot's label. Cheap observer
    /// for AI / scripts that already know the id and only need its
    /// display name (avoids the full `ListSolids` allocation).
    /// Does not mutate or append a history event.
    SolidLabel { id: SolidId },
    /// Read-only document-emptiness predicate. Returns
    /// `Outcome::IsEmpty { is_empty }` where
    /// `is_empty = solid_count() == 0`. Cheaper than `Stats` when
    /// the consumer only needs the boolean (skips the history
    /// length). Does not mutate or append a history event.
    IsEmpty,
    /// Read-only AABB surface area of a solid. Returns
    /// `Outcome::AabbSurfaceArea { id, surface_area }` where
    /// `surface_area = 2 * (dx*dy + dy*dz + dz*dx)` and
    /// `dx, dy, dz` are the per-axis bounding-box extents. Cheap
    /// upper bound on the solid's true surface area; useful for
    /// LOD heuristics and proportional thresholds without paying
    /// for the per-face traversal performed by `SurfaceArea` /
    /// `Measure`. Does not mutate or append a history event.
    AabbSurfaceArea { id: SolidId },
    /// Read-only solid-count observer. Returns
    /// `Outcome::SolidCount { count }` where `count` is the number
    /// of populated solid slots as a `u32`. Cheaper than `Stats`
    /// when only the count is needed. Does not mutate or append a
    /// history event.
    SolidCount,
    /// Read-only history-count observer. Returns
    /// `Outcome::HistoryCount { count }` where `count` is the
    /// number of recorded history events as a `u32`. Cheaper than
    /// `Stats` when only the history length is needed. Does not
    /// mutate or append a history event.
    HistoryCount,
    /// Read-only label-existence predicate. Returns
    /// `Outcome::HasLabel { query, has_label }` where
    /// `has_label = true` iff any solid's label matches the
    /// case-insensitive substring `query`. Cheaper than
    /// `FindByLabel` when only the boolean is needed (skips the
    /// `Vec<SolidEntry>` allocation). Does not mutate or append
    /// a history event.
    HasLabel { query: String },
    /// Read-only solid-id enumeration. Returns
    /// `Outcome::SolidIds { ids }` with the populated `SolidId`s
    /// only — no labels. Cheaper than `ListSolids` when the
    /// consumer only needs ids (skips per-slot label cloning).
    /// Does not mutate or append a history event.
    SolidIds,
    /// Read-only AABB-extents observer. Returns
    /// `Outcome::AabbExtents { id, extents }` where
    /// `extents = [max.x - min.x, max.y - min.y, max.z - min.z]`.
    /// Cheaper than `Bounds` (skips min/max points) and a raw
    /// companion to `Diagonal`. Does not mutate or append a
    /// history event.
    AabbExtents { id: SolidId },
    /// Read-only AABB longest-axis observer. Returns
    /// `Outcome::AabbLongestAxis { id, axis }` where `axis` is
    /// the index (`0` = X, `1` = Y, `2` = Z) of the largest
    /// bounding-box extent (ties favour the lower index). Useful
    /// for orientation heuristics. Does not mutate or append a
    /// history event.
    AabbLongestAxis { id: SolidId },
    /// Read-only AABB shortest-axis observer. Returns
    /// `Outcome::AabbShortestAxis { id, axis }` where `axis` is
    /// the index (`0` = X, `1` = Y, `2` = Z) of the smallest
    /// bounding-box extent (ties favour the lower index).
    /// Symmetric counterpart to `AabbLongestAxis`. Does not
    /// mutate or append a history event.
    AabbShortestAxis { id: SolidId },
    /// Read-only AABB aspect-ratio observer. Returns
    /// `Outcome::AabbAspectRatio { id, ratio }` where
    /// `ratio = longest_extent / shortest_extent` (>= 1.0).
    /// Returns `f64::INFINITY` when the shortest extent is
    /// exactly zero. Useful for slenderness/sliver detection.
    /// Does not mutate or append a history event.
    AabbAspectRatio { id: SolidId },
    /// Read-only cubic-AABB predicate. Returns
    /// `Outcome::IsCubic { id, cubic }` where `cubic` is
    /// `true` when all three bounding-box extents are equal
    /// within an absolute tolerance of `1e-9`. Quick shape
    /// classifier; does not mutate or append a history event.
    IsCubic { id: SolidId },
    /// Read-only square-XY-footprint predicate. Returns
    /// `Outcome::IsSquareXy { id, square }` where `square` is
    /// `true` when the X and Y AABB extents are equal within
    /// an absolute tolerance of `1e-9` (Z is unconstrained).
    /// Detects solids with a square footprint (square prisms,
    /// pillars, posts). Does not mutate or append a history event.
    IsSquareXy { id: SolidId },
    /// Read-only single-event history lookup. Returns
    /// `Outcome::HistoryDescription { index, description }` with
    /// the description string of the history event at `index`.
    /// Returns `InvalidArgument` if `index` is out of bounds.
    /// Cheaper than `HistoryEvents` when the caller only needs
    /// one entry; does not mutate or append a history event.
    HistoryDescription { index: u32 },
    /// Read-only square-YZ-footprint predicate. Returns
    /// `Outcome::IsSquareYz { id, square }` where `square` is
    /// `true` when the Y and Z AABB extents are equal within
    /// an absolute tolerance of `1e-9` (X is unconstrained).
    /// Detects solids with a square cross-section in the YZ
    /// plane (extrusions oriented along the X axis). Does not
    /// mutate or append a history event.
    IsSquareYz { id: SolidId },
    /// Read-only square-XZ-footprint predicate. Returns
    /// `Outcome::IsSquareXz { id, square }` where `square` is
    /// `true` when the X and Z AABB extents are equal within
    /// an absolute tolerance of `1e-9` (Y is unconstrained).
    /// Detects solids with a square cross-section in the XZ
    /// plane (extrusions oriented along the Y axis). Does not
    /// mutate or append a history event.
    IsSquareXz { id: SolidId },
    /// Read-only history op-occurrence counter. Returns
    /// `Outcome::OperationCount { op_name, count }` with the
    /// number of history events whose `op` field matches the
    /// queried `op_name` exactly (case-sensitive). Cheaper
    /// than `HistoryEvents` when callers only need a single
    /// op's count. Does not mutate or append a history event.
    OperationCount { op_name: String },
    /// Read-only last-history-event lookup. Returns
    /// `Outcome::LastOperation { index, op_name, description }`
    /// for the most recent history event. Equivalent to
    /// `HistoryDescription { index: HistoryCount - 1 }` plus
    /// the op name, but in a single call. Errors with
    /// `InvalidArgument` when the history is empty. Does not
    /// mutate or append a history event.
    LastOperation,
    /// Read-only history op-presence predicate. Returns
    /// `Outcome::HasOperation { op_name, present }` where
    /// `present` is `true` when at least one history event's
    /// `op` field matches `op_name` exactly (case-sensitive).
    /// Lighter than `OperationCount` when callers only need a
    /// boolean answer. Does not mutate or append a history event.
    HasOperation { op_name: String },
    /// Read-only first-history-event lookup. Returns
    /// `Outcome::FirstOperation { op_name, description }` for
    /// the oldest history event (always at index 0). Mirror of
    /// `LastOperation`. Errors with `InvalidArgument` when the
    /// history is empty. Does not mutate or append a history event.
    FirstOperation,
    /// `Outcome::SolidCreated { id, label }` where `label` is
    /// `"<source-label> (copy)"`. The source slot is left untouched.
    Duplicate { id: SolidId },
    /// Rotate a solid in place around an arbitrary axis through a pivot
    /// point. `axis` is the rotation axis in world space (need not be
    /// unit-length — the dispatcher normalises), `angle_rad` is the
    /// signed rotation angle in radians (right-hand rule), and `point`
    /// is the pivot in world coordinates. The slot's vertex positions
    /// are rewritten in place; topology is preserved.
    Rotate {
        id: SolidId,
        axis: [f64; 3],
        angle_rad: f64,
        point: [f64; 3],
    },
    /// No-op. Used by tests to verify the round-trip without side effects.
    Noop,
}

impl Command {
    /// Returns the snake_case operation name (matches the JSON `op` tag).
    pub fn op_name(&self) -> &'static str {
        match self {
            Self::CreateBox { .. } => "create_box",
            Self::CreateCylinder { .. } => "create_cylinder",
            Self::CreateSphere { .. } => "create_sphere",
            Self::CreateCone { .. } => "create_cone",
            Self::CreateTorus { .. } => "create_torus",
            Self::BooleanUnion { .. } => "boolean_union",
            Self::BooleanSubtract { .. } => "boolean_subtract",
            Self::BooleanIntersect { .. } => "boolean_intersect",
            Self::Translate { .. } => "translate",
            Self::Scale { .. } => "scale",
            Self::ScaleNonUniform { .. } => "scale_non_uniform",
            Self::CenterOnOrigin { .. } => "center_on_origin",
            Self::AlignTo { .. } => "align_to",
            Self::ScaleToFit { .. } => "scale_to_fit",
            Self::TranslateTo { .. } => "translate_to",
            Self::Rename { .. } => "rename",
            Self::DeleteSolid { .. } => "delete_solid",
            Self::Extrude { .. } => "extrude",
            Self::LinearPattern { .. } => "linear_pattern",
            Self::Mirror { .. } => "mirror",
            Self::NewDocument => "new_document",
            Self::Measure { .. } => "measure",
            Self::Validate => "validate",
            Self::ListSolids => "list_solids",
            Self::FindByLabel { .. } => "find_by_label",
            Self::HistoryEvents => "history_events",
            Self::Stats => "stats",
            Self::Bounds { .. } => "bounds",
            Self::Distance { .. } => "distance",
            Self::Volume { .. } => "volume",
            Self::SurfaceArea { .. } => "surface_area",
            Self::Centroid { .. } => "centroid",
            Self::IntersectsAabb { .. } => "intersects_aabb",
            Self::Exists { .. } => "exists",
            Self::Diagonal { .. } => "diagonal",
            Self::AabbCenter { .. } => "aabb_center",
            Self::AabbVolume { .. } => "aabb_volume",
            Self::ContainsAabb { .. } => "contains_aabb",
            Self::AabbCorners { .. } => "aabb_corners",
            Self::SolidLabel { .. } => "solid_label",
            Self::IsEmpty => "is_empty",
            Self::AabbSurfaceArea { .. } => "aabb_surface_area",
            Self::SolidCount => "solid_count",
            Self::HistoryCount => "history_count",
            Self::HasLabel { .. } => "has_label",
            Self::SolidIds => "solid_ids",
            Self::AabbExtents { .. } => "aabb_extents",
            Self::AabbLongestAxis { .. } => "aabb_longest_axis",
            Self::AabbShortestAxis { .. } => "aabb_shortest_axis",
            Self::AabbAspectRatio { .. } => "aabb_aspect_ratio",
            Self::IsCubic { .. } => "is_cubic",
            Self::IsSquareXy { .. } => "is_square_xy",
            Self::HistoryDescription { .. } => "history_description",
            Self::IsSquareYz { .. } => "is_square_yz",
            Self::IsSquareXz { .. } => "is_square_xz",
            Self::OperationCount { .. } => "operation_count",
            Self::LastOperation => "last_operation",
            Self::HasOperation { .. } => "has_operation",
            Self::FirstOperation => "first_operation",
            Self::Duplicate { .. } => "duplicate",
            Self::Rotate { .. } => "rotate",
            Self::Noop => "noop",
        }
    }
}

/// Returns the JSON-schema-style metadata for every [`Command`] variant.
///
/// Used by the MCP migration shim and by AI integrations to discover the
/// available command surface without scraping the Rust source.
pub fn command_schemas() -> Vec<CommandSchema> {
    vec![
        CommandSchema {
            op: "create_box",
            description: "Create an axis-aligned box at the origin.",
            params: &[
                ParamSchema {
                    name: "dx",
                    ty: "number",
                    required: true,
                    doc: "Width along +X.",
                },
                ParamSchema {
                    name: "dy",
                    ty: "number",
                    required: true,
                    doc: "Depth along +Y.",
                },
                ParamSchema {
                    name: "dz",
                    ty: "number",
                    required: true,
                    doc: "Height along +Z.",
                },
            ],
        },
        CommandSchema {
            op: "create_cylinder",
            description: "Create a cylinder at the origin with axis along +Z.",
            params: &[
                ParamSchema {
                    name: "radius",
                    ty: "number",
                    required: true,
                    doc: "Cylinder radius.",
                },
                ParamSchema {
                    name: "height",
                    ty: "number",
                    required: true,
                    doc: "Cylinder height along +Z.",
                },
            ],
        },
        CommandSchema {
            op: "create_sphere",
            description: "Create a UV-sphere at the origin.",
            params: &[ParamSchema {
                name: "radius",
                ty: "number",
                required: true,
                doc: "Sphere radius.",
            }],
        },
        CommandSchema {
            op: "create_cone",
            description: "Create a cone (apex at +Z, base on XY) at the origin.",
            params: &[
                ParamSchema {
                    name: "radius",
                    ty: "number",
                    required: true,
                    doc: "Base radius.",
                },
                ParamSchema {
                    name: "height",
                    ty: "number",
                    required: true,
                    doc: "Cone height along +Z.",
                },
            ],
        },
        CommandSchema {
            op: "create_torus",
            description: "Create a torus at the origin in the XY plane.",
            params: &[
                ParamSchema {
                    name: "major_radius",
                    ty: "number",
                    required: true,
                    doc: "Centerline radius.",
                },
                ParamSchema {
                    name: "minor_radius",
                    ty: "number",
                    required: true,
                    doc: "Tube radius (< major).",
                },
            ],
        },
        CommandSchema {
            op: "boolean_union",
            description: "Boolean union of two solids. Both inputs are consumed; the result is a new solid.",
            params: &[
                ParamSchema {
                    name: "lhs",
                    ty: "solid_id",
                    required: true,
                    doc: "First operand.",
                },
                ParamSchema {
                    name: "rhs",
                    ty: "solid_id",
                    required: true,
                    doc: "Second operand.",
                },
            ],
        },
        CommandSchema {
            op: "boolean_subtract",
            description: "Boolean lhs - rhs. Both inputs are consumed; the result is a new solid.",
            params: &[
                ParamSchema {
                    name: "lhs",
                    ty: "solid_id",
                    required: true,
                    doc: "Subject.",
                },
                ParamSchema {
                    name: "rhs",
                    ty: "solid_id",
                    required: true,
                    doc: "Tool.",
                },
            ],
        },
        CommandSchema {
            op: "boolean_intersect",
            description: "Boolean intersection. Both inputs are consumed; the result is a new solid.",
            params: &[
                ParamSchema {
                    name: "lhs",
                    ty: "solid_id",
                    required: true,
                    doc: "First operand.",
                },
                ParamSchema {
                    name: "rhs",
                    ty: "solid_id",
                    required: true,
                    doc: "Second operand.",
                },
            ],
        },
        CommandSchema {
            op: "translate",
            description: "Translate a solid by (dx, dy, dz).",
            params: &[
                ParamSchema {
                    name: "id",
                    ty: "solid_id",
                    required: true,
                    doc: "Target solid.",
                },
                ParamSchema {
                    name: "dx",
                    ty: "number",
                    required: true,
                    doc: "Offset along X.",
                },
                ParamSchema {
                    name: "dy",
                    ty: "number",
                    required: true,
                    doc: "Offset along Y.",
                },
                ParamSchema {
                    name: "dz",
                    ty: "number",
                    required: true,
                    doc: "Offset along Z.",
                },
            ],
        },
        CommandSchema {
            op: "scale",
            description: "Uniformly scale a solid about its centroid.",
            params: &[
                ParamSchema {
                    name: "id",
                    ty: "solid_id",
                    required: true,
                    doc: "Target solid.",
                },
                ParamSchema {
                    name: "factor",
                    ty: "number",
                    required: true,
                    doc: "Scale factor (> 0).",
                },
            ],
        },
        CommandSchema {
            op: "rename",
            description: "Rename a solid; the SolidId is preserved.",
            params: &[
                ParamSchema {
                    name: "id",
                    ty: "solid_id",
                    required: true,
                    doc: "Target solid.",
                },
                ParamSchema {
                    name: "label",
                    ty: "string",
                    required: true,
                    doc: "New label.",
                },
            ],
        },
        CommandSchema {
            op: "delete_solid",
            description: "Delete a solid. Its SolidId is retired permanently.",
            params: &[ParamSchema {
                name: "id",
                ty: "solid_id",
                required: true,
                doc: "Target solid.",
            }],
        },
        CommandSchema {
            op: "extrude",
            description: "Extrude a planar polygonal profile along a direction. `kind` (optional, default ‘blind’) selects Blind/MidPlane/TwoSided/ThroughAll/UpToFace.",
            params: &[
                ParamSchema {
                    name: "profile",
                    ty: "array<point3>",
                    required: true,
                    doc: "Closed polygon as a list of [x,y,z] points (≥ 3, no closing duplicate).",
                },
                ParamSchema {
                    name: "direction",
                    ty: "vec3",
                    required: true,
                    doc: "Extrusion direction [x,y,z]; will be normalized.",
                },
                ParamSchema {
                    name: "distance",
                    ty: "number",
                    required: true,
                    doc: "Extrusion length (> 0). Total span for MidPlane/TwoSided is interpreted per kind.",
                },
                ParamSchema {
                    name: "kind",
                    ty: "extrude_kind",
                    required: false,
                    doc: "Optional. Discriminated union with mode = blind | mid_plane | two_sided | through_all | up_to_face. TwoSided requires back_distance; UpToFace requires face_solid and face_index.",
                },
            ],
        },
        CommandSchema {
            op: "linear_pattern",
            description: "Produce a linear pattern from one solid or from every primary solid produced by a feature list, with optional skipped, mirrored, or offset instances.",
            params: &[
                ParamSchema {
                    name: "id",
                    ty: "solid_id",
                    required: true,
                    doc: "Source solid (preserved). Ignored when `features` is non-empty.",
                },
                ParamSchema {
                    name: "direction",
                    ty: "vec3",
                    required: true,
                    doc: "Pattern direction [x,y,z]; will be normalized.",
                },
                ParamSchema {
                    name: "spacing",
                    ty: "number",
                    required: true,
                    doc: "Distance between successive copies.",
                },
                ParamSchema {
                    name: "count",
                    ty: "integer",
                    required: true,
                    doc: "Total number of copies including the original (≥ 2).",
                },
                ParamSchema {
                    name: "skip_instances",
                    ty: "integer[]",
                    required: false,
                    doc: "Optional. Instance indices to suppress (0 = original, 1..count-1 = copies). Out-of-range entries are ignored.",
                },
                ParamSchema {
                    name: "features",
                    ty: "integer[]",
                    required: false,
                    doc: "Optional A2.3 feature-list mode. Array of FeatureIds whose primary solids will all be patterned in one combined PatternCreated outcome. When non-empty, `id` is ignored.",
                },
                ParamSchema {
                    name: "mirror_alternate",
                    ty: "boolean",
                    required: false,
                    doc: "Optional. When true, copied instances with odd indices are reflected about their spacing-derived plane with normal = direction.",
                },
                ParamSchema {
                    name: "instance_overrides",
                    ty: "object[]",
                    required: false,
                    doc: "Optional. Per-instance overrides with index, suppress, and offset_adjust fields. Suppression takes precedence over offset adjustment.",
                },
            ],
        },
        CommandSchema {
            op: "mirror",
            description: "Mirror a solid (or every solid produced by a list of FeatureIds) across a plane. Originals are preserved unless merge=true. When `features` is empty (default), only the single solid `id` is mirrored (legacy path); when non-empty, `id` is ignored and every listed feature's primary solid is mirrored as one combined PatternCreated outcome.",
            params: &[
                ParamSchema {
                    name: "id",
                    ty: "solid_id",
                    required: true,
                    doc: "Source solid (preserved). Ignored when `features` is non-empty.",
                },
                ParamSchema {
                    name: "point",
                    ty: "point3",
                    required: true,
                    doc: "A point lying on the mirror plane.",
                },
                ParamSchema {
                    name: "normal",
                    ty: "vec3",
                    required: true,
                    doc: "Mirror plane normal [x,y,z]; will be normalized.",
                },
                ParamSchema {
                    name: "merge",
                    ty: "boolean",
                    required: false,
                    doc: "Optional. When true, fuse each mirrored copy with its source via boolean union and consume the source slots.",
                },
                ParamSchema {
                    name: "features",
                    ty: "integer[]",
                    required: false,
                    doc: "Optional A2.2 feature-list mode. Array of FeatureIds whose primary solids will all be mirrored in one combined PatternCreated outcome. When non-empty, `id` is ignored.",
                },
            ],
        },
        CommandSchema {
            op: "new_document",
            description: "Reset the session to an empty document. Clears the command log.",
            params: &[],
        },
        CommandSchema {
            op: "measure",
            description: "Read-only: return volume / surface area / centroid / bbox of a solid as Outcome::Measured. Does not mutate the document or append history.",
            params: &[ParamSchema {
                name: "id",
                ty: "solid_id",
                required: true,
                doc: "Solid to measure.",
            }],
        },
        CommandSchema {
            op: "validate",
            description: "Read-only: run Document::validate() and return any DocumentIssues as Outcome::Validated. Does not mutate the document or append history.",
            params: &[],
        },
        CommandSchema {
            op: "list_solids",
            description: "Read-only: enumerate every solid in the document as Outcome::SolidsListed { entries: [{ id, label }, ...] }. Does not mutate the document or append history.",
            params: &[],
        },
        CommandSchema {
            op: "duplicate",
            description: "Deep-clone a solid into a new slot. Returns Outcome::SolidCreated with the new id and a '<source> (copy)' label. The source slot is preserved.",
            params: &[ParamSchema {
                name: "id",
                ty: "solid_id",
                required: true,
                doc: "Source solid to clone.",
            }],
        },
        CommandSchema {
            op: "find_by_label",
            description: "Case-insensitive substring search by label. Returns Outcome::SolidsListed with the matching rows. Empty query returns every solid.",
            params: &[ParamSchema {
                name: "query",
                ty: "string",
                required: true,
                doc: "Substring to match against each solid's label (case-insensitive).",
            }],
        },
        CommandSchema {
            op: "history_events",
            description: "Return every recorded HistoryEvent in execution order. Read-only; does not append a new event.",
            params: &[],
        },
        CommandSchema {
            op: "stats",
            description: "Return cheap O(1) document statistics: solid_count and history_count. Read-only.",
            params: &[],
        },
        CommandSchema {
            op: "bounds",
            description: "Return the axis-aligned bounding box of a solid. Cheap observer (skips volume/centroid).",
            params: &[ParamSchema {
                name: "id",
                ty: "solid_id",
                required: true,
                doc: "Solid to query.",
            }],
        },
        CommandSchema {
            op: "distance",
            description: "Return centroid-to-centroid distance and per-axis delta (b - a) between two solids.",
            params: &[
                ParamSchema {
                    name: "id_a",
                    ty: "solid_id",
                    required: true,
                    doc: "First solid (centroid is origin of delta).",
                },
                ParamSchema {
                    name: "id_b",
                    ty: "solid_id",
                    required: true,
                    doc: "Second solid (centroid is target of delta).",
                },
            ],
        },
        CommandSchema {
            op: "volume",
            description: "Return the volume of a solid (single scalar). Lighter than measure (skips surface-area/centroid/bbox).",
            params: &[ParamSchema {
                name: "id",
                ty: "solid_id",
                required: true,
                doc: "Solid to query.",
            }],
        },
        CommandSchema {
            op: "surface_area",
            description: "Return the surface area of a solid (single scalar). Lighter than measure.",
            params: &[ParamSchema {
                name: "id",
                ty: "solid_id",
                required: true,
                doc: "Solid to query.",
            }],
        },
        CommandSchema {
            op: "centroid",
            description: "Return the centroid of a solid (3-vector). Lighter than measure (skips volume/surface-area/bbox).",
            params: &[ParamSchema {
                name: "id",
                ty: "solid_id",
                required: true,
                doc: "Solid to query.",
            }],
        },
        CommandSchema {
            op: "intersects_aabb",
            description: "AABB-overlap predicate between two solids (cheap broad-phase). Touching boxes count as intersecting.",
            params: &[
                ParamSchema {
                    name: "id_a",
                    ty: "solid_id",
                    required: true,
                    doc: "First solid.",
                },
                ParamSchema {
                    name: "id_b",
                    ty: "solid_id",
                    required: true,
                    doc: "Second solid.",
                },
            ],
        },
        CommandSchema {
            op: "exists",
            description: "Solid-existence predicate. Never errors on missing ids; returns exists = false instead.",
            params: &[ParamSchema {
                name: "id",
                ty: "solid_id",
                required: true,
                doc: "Solid to probe.",
            }],
        },
        CommandSchema {
            op: "diagonal",
            description: "AABB diagonal length and per-axis extents (cheap; uses bounding box only).",
            params: &[ParamSchema {
                name: "id",
                ty: "solid_id",
                required: true,
                doc: "Solid to query.",
            }],
        },
        CommandSchema {
            op: "aabb_center",
            description: "AABB center (bbox.min + bbox.max) * 0.5. Distinct from centroid (mass centroid).",
            params: &[ParamSchema {
                name: "id",
                ty: "solid_id",
                required: true,
                doc: "Solid to query.",
            }],
        },
        CommandSchema {
            op: "aabb_volume",
            description: "AABB volume dx * dy * dz (cheap upper bound on mass volume; uses bounding box only).",
            params: &[ParamSchema {
                name: "id",
                ty: "solid_id",
                required: true,
                doc: "Solid to query.",
            }],
        },
        CommandSchema {
            op: "contains_aabb",
            description: "AABB-containment predicate. True iff outer's bounding box fully covers inner's bounding box (closed intervals).",
            params: &[
                ParamSchema {
                    name: "id_outer",
                    ty: "solid_id",
                    required: true,
                    doc: "Outer solid (potential container).",
                },
                ParamSchema {
                    name: "id_inner",
                    ty: "solid_id",
                    required: true,
                    doc: "Inner solid (potential containee).",
                },
            ],
        },
        CommandSchema {
            op: "aabb_corners",
            description: "Eight corner points of the world-space AABB in canonical order (low→high in x, then y, then z).",
            params: &[ParamSchema {
                name: "id",
                ty: "solid_id",
                required: true,
                doc: "Solid to query.",
            }],
        },
        CommandSchema {
            op: "solid_label",
            description: "Single-solid label observer. Returns the slot's display name without the ListSolids allocation.",
            params: &[ParamSchema {
                name: "id",
                ty: "solid_id",
                required: true,
                doc: "Solid to query.",
            }],
        },
        CommandSchema {
            op: "is_empty",
            description: "Document-emptiness predicate. True iff the document contains no solids.",
            params: &[],
        },
        CommandSchema {
            op: "aabb_surface_area",
            description: "AABB surface area 2 * (dx*dy + dy*dz + dz*dx) (cheap upper bound on mass surface area; uses bounding box only).",
            params: &[ParamSchema {
                name: "id",
                ty: "solid_id",
                required: true,
                doc: "Solid to query.",
            }],
        },
        CommandSchema {
            op: "solid_count",
            description: "Solid-count observer. Returns the number of populated solid slots as a u32.",
            params: &[],
        },
        CommandSchema {
            op: "history_count",
            description: "History-count observer. Returns the number of recorded history events as a u32.",
            params: &[],
        },
        CommandSchema {
            op: "has_label",
            description: "Label-existence predicate. Returns true iff any solid's label matches the case-insensitive substring query.",
            params: &[ParamSchema {
                name: "query",
                ty: "string",
                required: true,
                doc: "Case-insensitive substring to search for in solid labels.",
            }],
        },
        CommandSchema {
            op: "solid_ids",
            description: "Solid-id enumeration. Returns the populated SolidIds without labels (cheaper than list_solids).",
            params: &[],
        },
        CommandSchema {
            op: "aabb_extents",
            description: "AABB-extents observer. Returns the per-axis bounding-box extents [dx, dy, dz].",
            params: &[ParamSchema {
                name: "id",
                ty: "solid_id",
                required: true,
                doc: "Solid to query.",
            }],
        },
        CommandSchema {
            op: "aabb_longest_axis",
            description: "AABB longest-axis observer. Returns axis index (0=X, 1=Y, 2=Z) of the largest extent; ties favour the lower index.",
            params: &[ParamSchema {
                name: "id",
                ty: "solid_id",
                required: true,
                doc: "Solid to query.",
            }],
        },
        CommandSchema {
            op: "aabb_shortest_axis",
            description: "AABB shortest-axis observer. Returns axis index (0=X, 1=Y, 2=Z) of the smallest extent; ties favour the lower index.",
            params: &[ParamSchema {
                name: "id",
                ty: "solid_id",
                required: true,
                doc: "Solid to query.",
            }],
        },
        CommandSchema {
            op: "aabb_aspect_ratio",
            description: "AABB aspect-ratio observer. Returns longest_extent / shortest_extent (>= 1.0); +inf when shortest extent is zero.",
            params: &[ParamSchema {
                name: "id",
                ty: "solid_id",
                required: true,
                doc: "Solid to query.",
            }],
        },
        CommandSchema {
            op: "is_cubic",
            description: "Cubic-AABB predicate. Returns true when all three bounding-box extents are equal within an absolute tolerance of 1e-9.",
            params: &[ParamSchema {
                name: "id",
                ty: "solid_id",
                required: true,
                doc: "Solid to query.",
            }],
        },
        CommandSchema {
            op: "is_square_xy",
            description: "Square-XY-footprint predicate. Returns true when X and Y AABB extents are equal within an absolute tolerance of 1e-9 (Z unconstrained).",
            params: &[ParamSchema {
                name: "id",
                ty: "solid_id",
                required: true,
                doc: "Solid to query.",
            }],
        },
        CommandSchema {
            op: "history_description",
            description: "Single-event history lookup. Returns the description string of the history event at the given index. InvalidArgument when out of bounds.",
            params: &[ParamSchema {
                name: "index",
                ty: "u32",
                required: true,
                doc: "0-based index into the history event list.",
            }],
        },
        CommandSchema {
            op: "is_square_yz",
            description: "Square-YZ-footprint predicate. Returns true when Y and Z AABB extents are equal within an absolute tolerance of 1e-9 (X unconstrained).",
            params: &[ParamSchema {
                name: "id",
                ty: "solid_id",
                required: true,
                doc: "Solid to query.",
            }],
        },
        CommandSchema {
            op: "is_square_xz",
            description: "Square-XZ-footprint predicate. Returns true when X and Z AABB extents are equal within an absolute tolerance of 1e-9 (Y unconstrained).",
            params: &[ParamSchema {
                name: "id",
                ty: "solid_id",
                required: true,
                doc: "Solid to query.",
            }],
        },
        CommandSchema {
            op: "operation_count",
            description: "History op-occurrence counter. Returns the number of history events whose op matches the queried op_name exactly (case-sensitive).",
            params: &[ParamSchema {
                name: "op_name",
                ty: "string",
                required: true,
                doc: "Op name to count (e.g. \"create_box\").",
            }],
        },
        CommandSchema {
            op: "last_operation",
            description: "Last-history-event lookup. Returns op_name + description + 0-based index of the most recent history event. Errors with InvalidArgument when the history is empty.",
            params: &[],
        },
        CommandSchema {
            op: "has_operation",
            description: "History op-presence predicate. Returns true when at least one history event's op matches the queried op_name exactly (case-sensitive). Lighter than operation_count when only a boolean is needed.",
            params: &[ParamSchema {
                name: "op_name",
                ty: "string",
                required: true,
                doc: "Op name to test (e.g. \"create_box\").",
            }],
        },
        CommandSchema {
            op: "first_operation",
            description: "First-history-event lookup. Returns op_name + description of the oldest history event (always at index 0). Mirror of last_operation. Errors with InvalidArgument when the history is empty.",
            params: &[],
        },
        CommandSchema {
            op: "scale_non_uniform",
            description: "Non-uniform per-axis scale of a solid about an explicit pivot. Vertex positions are rewritten; topology is preserved.",
            params: &[
                ParamSchema {
                    name: "id",
                    ty: "solid_id",
                    required: true,
                    doc: "Solid to scale.",
                },
                ParamSchema {
                    name: "factors",
                    ty: "[f64; 3]",
                    required: true,
                    doc: "Per-axis multipliers [sx, sy, sz]; every component must be > 0.",
                },
                ParamSchema {
                    name: "point",
                    ty: "[f64; 3]",
                    required: true,
                    doc: "Pivot point in world coordinates.",
                },
            ],
        },
        CommandSchema {
            op: "center_on_origin",
            description: "Translate a solid so its centroid lands on the world origin. Convenience equivalent to Translate by -centroid.",
            params: &[ParamSchema {
                name: "id",
                ty: "solid_id",
                required: true,
                doc: "Solid to re-centre.",
            }],
        },
        CommandSchema {
            op: "align_to",
            description: "Translate `id` so its centroid coincides with the centroid of `target_id`. Two-solid alignment convenience.",
            params: &[
                ParamSchema {
                    name: "id",
                    ty: "solid_id",
                    required: true,
                    doc: "Solid to move.",
                },
                ParamSchema {
                    name: "target_id",
                    ty: "solid_id",
                    required: true,
                    doc: "Solid whose centroid is the alignment target (read-only).",
                },
            ],
        },
        CommandSchema {
            op: "scale_to_fit",
            description: "Uniformly scale a solid about its centroid so its largest bbox extent equals target_size.",
            params: &[
                ParamSchema {
                    name: "id",
                    ty: "solid_id",
                    required: true,
                    doc: "Solid to scale.",
                },
                ParamSchema {
                    name: "target_size",
                    ty: "number",
                    required: true,
                    doc: "Desired largest bbox extent (must be > 0).",
                },
            ],
        },
        CommandSchema {
            op: "translate_to",
            description: "Translate a solid so its centroid coincides with `point`. Generalises CenterOnOrigin/AlignTo.",
            params: &[
                ParamSchema {
                    name: "id",
                    ty: "solid_id",
                    required: true,
                    doc: "Solid to move.",
                },
                ParamSchema {
                    name: "point",
                    ty: "[f64; 3]",
                    required: true,
                    doc: "World-coordinate target for the solid's centroid.",
                },
            ],
        },
        CommandSchema {
            op: "rotate",
            description: "Rotate a solid in place around an arbitrary axis through a pivot point. Vertex positions are rewritten; topology is preserved.",
            params: &[
                ParamSchema {
                    name: "id",
                    ty: "solid_id",
                    required: true,
                    doc: "Solid to rotate.",
                },
                ParamSchema {
                    name: "axis",
                    ty: "[f64; 3]",
                    required: true,
                    doc: "Rotation axis in world space (does not need to be unit-length).",
                },
                ParamSchema {
                    name: "angle_rad",
                    ty: "f64",
                    required: true,
                    doc: "Signed rotation angle in radians (right-hand rule).",
                },
                ParamSchema {
                    name: "point",
                    ty: "[f64; 3]",
                    required: true,
                    doc: "Pivot point in world coordinates.",
                },
            ],
        },
        CommandSchema {
            op: "noop",
            description: "No-op; useful for round-trip and ping tests.",
            params: &[],
        },
    ]
}

/// Schema entry returned by [`command_schemas`].
#[derive(Debug, Clone, Copy)]
pub struct CommandSchema {
    pub op: &'static str,
    pub description: &'static str,
    pub params: &'static [ParamSchema],
}

/// Parameter description inside a [`CommandSchema`].
#[derive(Debug, Clone, Copy)]
pub struct ParamSchema {
    pub name: &'static str,
    /// Logical type. `"number"`, `"string"`, `"solid_id"` are the current set.
    pub ty: &'static str,
    pub required: bool,
    pub doc: &'static str,
}

/// Extrusion mode for [`Command::Extrude`].
///
/// JSON serialization uses the `mode` tag for clean discriminated-union
/// matching by AI agents and tests; for example `MidPlane` becomes
/// `{ "mode": "mid_plane" }` and `TwoSided` becomes
/// `{ "mode": "two_sided", "back_distance": 5.0 }`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum ExtrudeKind {
    /// One-sided extrusion: the profile is the start cap and the result
    /// extends `distance` units along `direction`. Matches pre-A2.1 behavior
    /// and is the default when the `kind` field is omitted from JSON.
    #[default]
    Blind,
    /// Symmetric extrusion: the profile sits at the mid-plane and the result
    /// extends `distance / 2` units in both `+direction` and `-direction`,
    /// for a total span of `distance` units.
    MidPlane,
    /// Asymmetric two-sided extrusion: the profile sits at the join, the
    /// result extends `distance` units along `+direction` and
    /// `back_distance` units along `-direction`. Total span is
    /// `distance + back_distance`.
    TwoSided {
        /// Length along `-direction`. Must be > 0; equal to `distance` is
        /// allowed (and identical in result to `MidPlane` with twice the
        /// distance).
        back_distance: f64,
    },
    /// One-sided extrusion whose span is computed from the current document
    /// bounding box along the requested direction. The command's `distance`
    /// field is ignored.
    ThroughAll,
    /// One-sided extrusion whose span is computed by intersecting the
    /// extrusion ray with the plane of a target face. The command's
    /// `distance` field is ignored.
    UpToFace {
        face_solid: SolidId,
        face_index: u32,
    },
}

/// Per-instance adjustment for [`Command::LinearPattern`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct InstanceOverride {
    /// Instance index targeted by this override. 0 = original; 1..count-1 =
    /// copied instances. Out-of-range indices are ignored.
    pub index: u32,
    /// When true, the instance is omitted from the pattern and
    /// `offset_adjust` is ignored.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub suppress: bool,
    /// Per-axis nudge added to the instance's spacing-derived position.
    #[serde(default, skip_serializing_if = "is_zero_vec3")]
    pub offset_adjust: [f64; 3],
}

fn is_zero_vec3(v: &[f64; 3]) -> bool {
    v[0] == 0.0 && v[1] == 0.0 && v[2] == 0.0
}
