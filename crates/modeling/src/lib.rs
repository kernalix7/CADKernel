//! High-level modeling operations for the CAD kernel.
//!
//! Includes primitive construction ([`make_box`], [`make_cylinder`], [`make_sphere`]),
//! feature operations (extrude, revolve, sweep, loft, chamfer, shell, mirror, scale, pattern),
//! boolean operations (union, intersection, difference), and mass-property computation.

pub mod boolean;
pub mod features;
pub mod measure;
pub mod primitives;
pub mod query;

pub use boolean::{BooleanOp, boolean_op};
pub use features::{
    ChamferResult, DraftResult, ExtrudeResult, FilletResult, LoftResult, MirrorResult,
    PatternResult, RevolveResult, ScaleResult, ShellResult, SplitResult, SweepResult, chamfer_edge,
    circular_pattern, draft_faces, extrude, fillet_edge, linear_pattern, loft, mirror_solid,
    revolve, scale_solid, shell_solid, split_solid, sweep,
};
pub use measure::{MassProperties, compute_mass_properties, solid_mass_properties};
pub use primitives::{make_box, make_cylinder, make_sphere};
pub use query::{ClosestPointResult, Containment, closest_point_on_solid, point_in_solid};
