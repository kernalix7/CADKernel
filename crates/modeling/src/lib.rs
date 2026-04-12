//! High-level modeling operations for the CAD kernel.
//!
//! This crate builds on top of `cadkernel-topology` and `cadkernel-geometry` to
//! provide the constructive solid geometry (CSG) pipeline used by the GUI.
//!
//! # Primitives
//!
//! [`make_box`], [`make_cylinder`], [`make_sphere`], [`make_cone`],
//! [`make_torus`], and 8 more primitive constructors. Each returns a result
//! struct containing the solid handle and handles to all created faces/edges.
//!
//! # Feature Operations
//!
//! [`extrude`], [`revolve`], [`sweep`], [`loft`], [`chamfer_edge`],
//! [`fillet_edge`], [`draft_faces`], [`shell_solid`], [`mirror_solid`],
//! [`scale_solid`], [`linear_pattern`], [`circular_pattern`], [`pad`],
//! [`pocket`], and more. Most take a `&mut BRepModel` and return a result
//! struct.
//!
//! # Boolean Operations
//!
//! [`boolean_op`] performs union, difference, or intersection on two solids.
//! [`boolean_op_exact`] first splits faces along intersection curves for
//! correct handling of partially overlapping geometry.
//!
//! # Measurement
//!
//! [`compute_mass_properties`] computes volume, surface area, and centroid.
//! [`measure_distance`], [`measure_angle`], [`measure_edge_length`], etc.
//!
//! # Assembly
//!
//! [`Assembly`], [`Component`], [`AssemblyConstraint`] for multi-part models.

pub mod appearance;
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
pub mod plugin;
pub mod primitives;
pub mod query;
pub mod quick;
pub mod shape_analysis;
pub mod surface_ops;
pub mod templates;

pub use appearance::{
    AttachmentMode, FaceAppearance, FaceAppearanceMap, compute_attachment,
    defeaturing_remove_faces, get_face_appearance, set_face_appearance,
};
pub use assembly::{
    Assembly, AssemblyConstraint, AssemblyPreferences, BomEntry, Component, ComponentId,
    DofAnalysis, JointType, joint_jacobian, joint_residual, kinematic_simulation,
    new_part_in_assembly, rotation, translation,
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
    AnnotationStyle, ArrayResult, ArrowStyle, BSplineWireResult, CloneResult, DraftDimension,
    DraftDimensionFull, DraftDimensionType, DraftLabel, DraftLabelFull, DraftLayer, DraftStyle,
    DraftStyleManager, HatchPattern, HatchResult, LayerManager, SnapMode, SnapResult, WireResult,
    WorkingPlane, bspline_to_wire, bspline_to_wire_convert, circular_array, clone_solid,
    downgrade_solid, downgrade_solid_faces, draft_hatch, draft_to_sketch, edit_draft, join_wires,
    make_arc_3pt_wire, make_arc_wire, make_bezier_wire, make_bspline_wire, make_chamfer_wire,
    make_circle_wire, make_cubic_bezier_wire, make_dimension_text, make_draft_dimension,
    make_draft_dimension_full, make_ellipse_wire, make_facebinder, make_fillet_wire, make_label, make_line_draft,
    make_label_full, make_point, make_polygon_wire, make_rectangle_wire, make_wire,
    mirror_solid_draft, move_solid, offset_wire, path_array, path_link_array, point_array,
    point_link_array, polar_array, rectangular_array, rotate_solid, scale_solid_draft,
    shape_from_text, snap_lock, snap_to_angle, snap_to_center, snap_to_dimensions,
    snap_to_endpoint, snap_to_extension, snap_to_grid, snap_to_intersection, snap_to_midpoint,
    snap_to_nearest, snap_to_parallel, snap_to_perpendicular, snap_to_point, snap_to_special,
    snap_to_working_plane, split_wire, stretch_wire, trimex_draft, upgrade_wire,
    upgrade_wire_model, wire_area, wire_length, wire_to_bspline, wire_to_bspline_convert,
};
pub use features::{
    BooleanFragmentsResult, ChamferResult, DraftResult, ExtrudeResult, FaceFromWiresResult,
    FilletResult, GrooveResult, HoleResult, JoinResult, LoftResult, MirrorResult, OffsetResult,
    PadResult, PatternResult, PocketResult, PointsFromShapeResult, RefineResult, RevolveResult,
    ReverseResult, ScaleResult, SectionEdge, SectionResult, ShapeFromMeshResult, ShellResult,
    SliceToCompoundResult, SplitResult, SweepResult, TaperExtrudeResult, ThicknessJoin,
    ThicknessResult,
    additive_box, additive_cone, additive_cylinder, additive_ellipsoid, additive_helix,
    additive_loft, additive_pipe, additive_prism, additive_sphere, additive_torus, additive_wedge,
    auto_defeaturing, make_sprocket, shaft_design, shape_binder, sub_shape_binder,
    boolean_fragments, chamfer_edge, circular_pattern, compound_filter, connect_shapes,
    countersunk_hole, cross_sections, cutout_shapes, draft_faces, embed_shapes, explode_compound,
    extrude, face_from_wires,
    project_curve_on_solid, project_curves_on_surface, remove_face, simplify_solid,
    fillet_edge, fillet_edge_segments, groove, hole, linear_pattern, loft, mirror_solid,
    offset_solid, pad, pocket, points_from_shape, project_points_on_surface, refine_shape,
    reverse_solid, revolve, scale_solid, section_solid, shape_from_mesh, shell_solid,
    slice_to_compound, split_solid,
    subtractive_box, subtractive_cone, subtractive_cylinder, subtractive_ellipsoid,
    subtractive_helix, subtractive_loft, subtractive_pipe, subtractive_prism, subtractive_sphere,
    subtractive_torus, subtractive_wedge, sweep, taper_extrude, thickness_solid,
};
pub use fem::{
    AcousticResult, AnalysisContainer, BeamSection, BoundaryCondition, BucklingResult,
    CoupledResult, ElectrostaticResult, ElementGeometry, ElementQuality,
    EmBoundaryCondition, FemMaterial, FemPreferences, FemResult, FilterFunction, FilteredResult,
    FlowResult, FluidBoundaryCondition, GeometricalFeature, HexMesh, MagnetostaticResult,
    MeshQuality, MeshRegion, ModalResult, PrincipalStresses, ScalarResult, StrainResult,
    StressTensor, TetMesh, ThermalBoundaryCondition, ThermalMaterial, ThermalResult,
    VisualizationData, VisualizationMode, acoustic_equation, adaptive_mesh_refinement,
    apply_element_geometry, apply_filter, beam_stiffness_matrix, buckling_analysis,
    check_boundary_conditions, check_mesh_quality_detailed, compute_error_estimate,
    compute_reactions, compute_strain_tensor, compute_stress_tensor, coupled_thermo_mechanical,
    create_mesh_region, deformation_equation, diffusion_equation, electrostatic_equation,
    estimate_computation_time, export_fem_report, export_mesh_abaqus, export_mesh_format,
    extract_nodal_values,
    extract_surface_mesh, fem_preferences, fem_summary, flow_equation, frequency_analysis,
    generate_hex_mesh, generate_tet_mesh, heat_equation, integrate_over_surface,
    interpolate_to_nodes, magnetostatic_equation, max_min_values, merge_coincident_nodes,
    mesh_from_shape, mesh_quality, mesh_smoothing, modal_analysis, nonlinear_static_analysis,
    path_result, poisson_equation, prepare_visualization, principal_stresses, purge_results,
    reaction_forces, refine_tet_mesh, result_at_point, safety_factor, shell_stiffness_matrix,
    show_mesh_info, static_analysis, strain_energy, thermal_analysis,
};
pub use gear::{GearResult, make_involute_gear};
pub use measure::{
    MassProperties, compute_mass_properties, measure_angle, measure_distance, measure_edge_length,
    measure_face_area, measure_solid_center_of_mass, measure_solid_volume, solid_mass_properties,
};
pub use multi_transform::{MultiTransformResult, Transform, multi_transform};
pub use plugin::{
    AutoNamingPlugin, Plugin, PluginCommand, PluginId, PluginInfo, PluginRegistry, PluginState,
    PluginStatus, StatisticsPlugin, ValidationPlugin, model_statistics,
};
pub use primitives::{
    CircleShapeResult, ConvertToSolidResult, EllipseShapeResult, LineShapeResult,
    PlaneFaceResult, PointShapeResult, PolygonResult, PrimitiveParams, ShapeFromEdgesResult,
    SpiralResult,
    convert_to_solid, make_box, make_circle_shape, make_cone, make_cylinder, make_ellipse_shape,
    make_ellipsoid, make_helix, make_line_shape, make_plane_face, make_point_shape, make_polygon,
    make_primitive, make_prism, make_sphere, make_spiral, make_torus, make_tube, make_wedge,
    shape_builder_from_edges, transformed_copy,
};
pub use query::{ClosestPointResult, Containment, closest_point_on_solid, point_in_solid};
pub use quick::{
    BBoxResult, quick_area, quick_bbox, quick_box, quick_centroid, quick_cone, quick_cylinder,
    quick_intersect, quick_sphere, quick_subtract, quick_torus, quick_union, quick_volume,
};
pub use shape_analysis::{SolidType, classify_solid, find_cylindrical_faces, find_planar_faces};
pub use surface_ops::{
    CoonsPatchResult, CurveOnMeshResult, ExtendResult, PipeSurfaceResult, RuledSurfaceResult,
    SurfaceFillingResult, SurfaceFromCurvesResult, SurfaceSectionsResult,
    coons_patch, curve_on_mesh, extend_surface, filling, pipe_surface, ruled_surface, sections,
    surface_from_curves,
};
pub use templates::{
    PreviewType, TemplateInfo, template_basic_assembly, template_empty, template_gear_demo,
    template_list, template_mechanical_part, template_single_box,
};
