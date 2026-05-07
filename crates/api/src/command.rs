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
    LinearPattern {
        id: SolidId,
        direction: [f64; 3],
        spacing: f64,
        count: u32,
    },
    /// Mirror a solid across a plane. Produces a new solid; the original is
    /// preserved.
    Mirror {
        id: SolidId,
        point: [f64; 3],
        normal: [f64; 3],
    },
    /// Create a freshly-named empty document. Discards every existing solid
    /// and resets the [`Session`] log. Useful as the first command of a
    /// replay test or AI session reset.
    NewDocument,
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
            Self::Rename { .. } => "rename",
            Self::DeleteSolid { .. } => "delete_solid",
            Self::Extrude { .. } => "extrude",
            Self::LinearPattern { .. } => "linear_pattern",
            Self::Mirror { .. } => "mirror",
            Self::NewDocument => "new_document",
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
            ],
        },
        CommandSchema {
            op: "new_document",
            description: "Reset the session to an empty document. Clears the command log.",
            params: &[],
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
