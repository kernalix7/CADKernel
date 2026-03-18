//! High-level modeling operations for the CAD kernel.
//!
//! Includes primitive construction ([`make_box`], [`make_cylinder`], [`make_sphere`]),
//! feature operations (extrude, revolve, sweep, loft, chamfer, shell, mirror, scale, pattern),
//! boolean operations (union, intersection, difference), and mass-property computation.

pub mod assembly;
pub mod body;
pub mod boolean;
pub mod check;
pub mod compound;
pub mod draft_ops;
pub mod features;
pub mod fem;
pub mod gear;
pub mod measure;
pub mod multi_transform;
pub mod primitives;
pub mod query;
pub mod shape_analysis;
pub mod surface_ops;

pub use assembly::{
    Assembly, AssemblyConstraint, BomEntry, Component, ComponentId, DofAnalysis, JointType,
    rotation, translation,
};
pub use body::{Body, BodyFeature, FeatureKind};
pub use boolean::{
    BooleanOp, BooleanSplitResult, TrimIssue, TrimValidation, boolean_op, boolean_op_exact,
    boolean_xor, ensure_correct_winding, fit_ssi_to_nurbs, fit_ssi_to_pcurve,
    split_solids_at_intersection, validate_trim,
};
pub use check::{CheckResult, check_geometry, check_watertight};
pub use compound::Compound;
pub use draft_ops::{
    ArrayResult, BSplineWireResult, CloneResult, DraftDimension, DraftLabel, SnapResult,
    WireResult, bspline_to_wire, clone_solid, downgrade_solid, join_wires, make_arc_3pt_wire,
    make_arc_wire, make_bezier_wire, make_bspline_wire, make_chamfer_wire, make_circle_wire,
    make_dimension_text, make_draft_dimension, make_ellipse_wire, make_fillet_wire, make_label,
    make_point, make_polygon_wire, make_rectangle_wire, make_wire, mirror_solid_draft,
    move_solid, offset_wire, path_array, point_array, polar_array, rectangular_array,
    rotate_solid, scale_solid_draft, snap_to_endpoint, snap_to_midpoint, snap_to_nearest,
    split_wire, stretch_wire, upgrade_wire, wire_area, wire_length, wire_to_bspline,
};
pub use features::{
    BooleanFragmentsResult, ChamferResult, DraftResult, ExtrudeResult, FaceFromWiresResult,
    FilletResult, GrooveResult, HoleResult, JoinResult, LoftResult, MirrorResult, OffsetResult,
    PadResult, PatternResult, PocketResult, PointsFromShapeResult, RefineResult, RevolveResult,
    ReverseResult, ScaleResult, SectionEdge, SectionResult, ShapeFromMeshResult, ShellResult,
    SliceToCompoundResult, SplitResult, SweepResult, TaperExtrudeResult, ThicknessJoin,
    ThicknessResult,
    additive_box, additive_cone, additive_cylinder, additive_ellipsoid, additive_helix,
    additive_prism, additive_sphere, additive_torus, additive_wedge,
    boolean_fragments, chamfer_edge, circular_pattern, compound_filter, connect_shapes,
    countersunk_hole, cross_sections, cutout_shapes, draft_faces, embed_shapes, explode_compound,
    extrude, face_from_wires,
    project_curve_on_solid, remove_face, simplify_solid,
    fillet_edge, fillet_edge_segments, groove, hole, linear_pattern, loft, mirror_solid,
    offset_solid, pad, pocket, points_from_shape, project_points_on_surface, refine_shape,
    reverse_solid, revolve, scale_solid, section_solid, shape_from_mesh, shell_solid,
    slice_to_compound, split_solid,
    subtractive_box, subtractive_cone, subtractive_cylinder, subtractive_ellipsoid,
    subtractive_helix, subtractive_loft, subtractive_pipe, subtractive_prism, subtractive_sphere,
    subtractive_torus, subtractive_wedge, sweep, taper_extrude, thickness_solid,
};
pub use fem::{
    BeamSection, BoundaryCondition, FemMaterial, FemResult, MeshQuality, ModalResult,
    PrincipalStresses, StrainResult, StressTensor, TetMesh, ThermalBoundaryCondition,
    ThermalMaterial, ThermalResult, compute_reactions, compute_strain_tensor, compute_stress_tensor,
    extract_surface_mesh, generate_tet_mesh, merge_coincident_nodes, mesh_quality,
    modal_analysis, principal_stresses, refine_tet_mesh, safety_factor, static_analysis,
    strain_energy, thermal_analysis,
};
pub use gear::{GearResult, make_involute_gear};
pub use measure::{
    MassProperties, compute_mass_properties, measure_angle, measure_distance, measure_edge_length,
    measure_face_area, measure_solid_center_of_mass, measure_solid_volume, solid_mass_properties,
};
pub use multi_transform::{MultiTransformResult, Transform, multi_transform};
pub use primitives::{
    PlaneFaceResult, PolygonResult, SpiralResult, make_box, make_cone, make_cylinder,
    make_ellipsoid, make_helix, make_plane_face, make_polygon, make_prism, make_sphere,
    make_spiral, make_torus, make_tube, make_wedge,
};
pub use query::{ClosestPointResult, Containment, closest_point_on_solid, point_in_solid};
pub use shape_analysis::{SolidType, classify_solid, find_cylindrical_faces, find_planar_faces};
pub use surface_ops::{
    CurveOnMeshResult, ExtendResult, PipeSurfaceResult, RuledSurfaceResult,
    SurfaceFillingResult, SurfaceFromCurvesResult, SurfaceSectionsResult,
    curve_on_mesh, extend_surface, filling, pipe_surface, ruled_surface, sections,
    surface_from_curves,
};
