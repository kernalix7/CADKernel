pub mod additive;
pub mod chamfer;
pub mod compound_ops;
pub(crate) mod copy_utils;
pub mod cross_sections;
pub mod defeature;
pub mod draft;
pub mod extrude;
pub mod face_from_wires;
pub mod fillet;
pub mod groove;
pub mod hole;
pub mod join;
pub mod loft;
pub mod mirror;
pub mod offset;
pub mod pad;
pub mod pattern;
pub mod pocket;
pub mod project_on_surface;
pub mod projection;
pub mod revolve;
pub mod scale;
pub mod section;
pub mod shape_convert;
pub mod shell;
pub mod split;
pub mod sweep;
pub mod taper_extrude;
pub mod thickness;

pub use additive::{
    additive_box, additive_cone, additive_cylinder, additive_ellipsoid, additive_helix,
    additive_prism, additive_sphere, additive_torus, additive_wedge, subtractive_box,
    subtractive_cone, subtractive_cylinder, subtractive_ellipsoid, subtractive_helix,
    subtractive_loft, subtractive_pipe, subtractive_prism, subtractive_sphere, subtractive_torus,
    subtractive_wedge,
};
pub use compound_ops::{
    BooleanFragmentsResult, SliceToCompoundResult, boolean_fragments, compound_filter,
    explode_compound, slice_to_compound,
};
pub use chamfer::{ChamferResult, chamfer_edge};
pub use defeature::{remove_face, simplify_solid};
pub use cross_sections::cross_sections;
pub use draft::{DraftResult, draft_faces};
pub use extrude::{ExtrudeResult, extrude};
pub use fillet::{FilletResult, fillet_edge, fillet_edge_segments};
pub use face_from_wires::{FaceFromWiresResult, PointsFromShapeResult, face_from_wires, points_from_shape};
pub use groove::{GrooveResult, groove};
pub use hole::{HoleResult, countersunk_hole, hole};
pub use join::{JoinResult, connect_shapes, cutout_shapes, embed_shapes};
pub use loft::{LoftResult, loft};
pub use mirror::{MirrorResult, mirror_solid};
pub use offset::{OffsetResult, offset_solid};
pub use pad::{PadResult, pad};
pub use pattern::{PatternResult, circular_pattern, linear_pattern};
pub use pocket::{PocketResult, pocket};
pub use project_on_surface::project_points_on_surface;
pub use projection::project_curve_on_solid;
pub use revolve::{RevolveResult, revolve};
pub use scale::{ScaleResult, scale_solid};
pub use section::{SectionEdge, SectionResult, section_solid};
pub use shape_convert::{RefineResult, ReverseResult, ShapeFromMeshResult, refine_shape, reverse_solid, shape_from_mesh};
pub use shell::{ShellResult, shell_solid};
pub use split::{SplitResult, split_solid};
pub use sweep::{SweepResult, sweep};
pub use taper_extrude::{TaperExtrudeResult, taper_extrude};
pub use thickness::{ThicknessJoin, ThicknessResult, thickness_solid};
