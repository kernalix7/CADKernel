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

use crate::document::SolidId;

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
    /// Create a cone (top radius = 0) at the origin.
    CreateCone { radius: f64, height: f64 },
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
    /// `UpToFace` are reserved for A2.2 once `sketch_id` lands on the
    /// document and feature-id selection is wired through the API.
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
    /// range are ignored. This is a precursor to the full A2
    /// `instance_overrides` map (skip / suppress / offset-adjust); for
    /// now we only model the “skip” case which is by far the most
    /// common use (e.g. mounting flange with a missing bolt position).
    LinearPattern {
        id: SolidId,
        direction: [f64; 3],
        spacing: f64,
        count: u32,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        skip_instances: Vec<u32>,
    },
    /// Mirror a solid across a plane. Produces a new solid; the original is
    /// preserved unless `merge` is true, in which case the original and the
    /// mirrored copy are fused via boolean union and the source slot is
    /// consumed (matches FreeCAD/SolidWorks “mirror with merge” /
    /// PartDesign Mirrored feature behaviour).
    Mirror {
        id: SolidId,
        point: [f64; 3],
        normal: [f64; 3],
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        merge: bool,
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
    /// Deep-clone a solid into a new slot. Returns the new
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
            description: "Extrude a planar polygonal profile along a direction by a distance. `kind` (optional, default ‘blind’) selects Blind/MidPlane/TwoSided. ThroughAll and UpToFace are reserved for A2.2.",
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
                    doc: "Optional. Discriminated union with mode = blind | mid_plane | two_sided. TwoSided requires back_distance.",
                },
            ],
        },
        CommandSchema {
            op: "linear_pattern",
            description: "Produce `count` copies of a solid at `spacing` along `direction`.",
            params: &[
                ParamSchema {
                    name: "id",
                    ty: "solid_id",
                    required: true,
                    doc: "Source solid (preserved).",
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
            ],
        },
        CommandSchema {
            op: "mirror",
            description: "Mirror a solid across a plane. The original is preserved.",
            params: &[
                ParamSchema {
                    name: "id",
                    ty: "solid_id",
                    required: true,
                    doc: "Source solid (preserved).",
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
                    doc: "Optional. When true, fuse the mirrored copy with the original via boolean union and consume the source slot.",
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
/// Subset of the A2 spec landed in 2026-05-07. `ThroughAll` and `UpToFace`
/// are reserved for A2.2 — they require sketch/feature-id concepts that are
/// not yet wired through the API surface.
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
}
