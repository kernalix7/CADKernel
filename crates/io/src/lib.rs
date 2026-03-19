//! File I/O and tessellation for the CAD kernel.
//!
//! Supports reading and writing STL (ASCII & binary), Wavefront OBJ, SVG,
//! DXF, PLY, 3MF, BREP, and JSON formats, plus face/solid tessellation
//! into indexed triangle meshes.

pub mod brep_format;
pub mod collada;
pub mod dxf;
pub mod dwg;
pub mod gltf;
pub mod iges;
pub mod json;
pub mod native;
pub mod obj;
pub mod pdf;
pub mod ply;
pub mod step;
pub mod stl;
pub mod svg;
pub mod techdraw;
pub mod threemf;
pub mod mesh_ops;
pub mod tessellate;

pub use brep_format::{export_brep, import_brep, write_brep};
pub use dxf::{export_dxf, import_dxf, write_dxf};
pub use collada::{export_dae, import_dae, write_dae};
pub use dwg::{export_dwg, import_dwg, write_dwg};
pub use gltf::{export_gltf, import_gltf, write_gltf};
pub use pdf::{export_pdf, write_pdf};
pub use iges::{
    IgesEntity, IgesEntityType, IgesWriter, export_iges, export_iges_mesh, import_iges,
    parse_iges, read_iges_lines, read_iges_points,
};
pub use json::{export_json, import_json, model_from_json, model_to_json, read_json, write_json};
pub use native::{CADK_EXTENSION, load_project, save_project};
pub use obj::{export_obj, import_obj, read_obj, write_obj};
pub use ply::{export_ply, import_ply, write_ply};
pub use step::{
    ParsedStepEntity, StepEntity, StepFile, StepParam, StepWriter, export_step, export_step_mesh,
    import_step, parse_step, parse_step_entities, read_step_points, tokenize,
};
pub use stl::{
    export_stl_ascii, export_stl_binary, import_stl, read_stl_ascii, read_stl_binary,
    write_stl_ascii, write_stl_binary,
};
pub use svg::{SvgDocument, SvgElement, SvgStyle, profile_to_svg};
pub use techdraw::{
    ArcLengthDimension, BalloonAnnotation, BoltCircleCenterlines, BreakLine, CenterMark,
    Centerline, ChamferDimension, CosmeticLine, CosmeticLineStyle, Dimension, DimensionType,
    DrawingSheet, DrawingView, ExtentDimension, HatchPattern, LeaderLine, ProjectedEdge,
    ProjectionDir, SurfaceFinishSymbol, TextAnnotation, WeldSymbol, WeldType,
    arc_length_dimension_to_svg, balloon_annotation_to_svg, bolt_circle_centerlines_to_svg,
    break_line_to_svg, center_mark_to_svg, centerline_to_svg, chamfer_dimension_to_svg,
    cosmetic_line_to_svg, detail_view, dimension_to_svg, drawing_to_svg,
    extent_dimension_to_svg, hatch_pattern_to_svg, leader_line_to_svg, project_solid,
    section_view, surface_finish_to_svg, text_annotation_to_svg, three_view_drawing,
    weld_symbol_to_svg,
};
pub use threemf::{export_3mf, import_3mf, write_3mf};
pub use mesh_ops::{
    FaceInfo, MeshBoundingBox, MeshRepairReport, MeshSegment, RegularSolidType, UnwrapResult,
    UvCoord, add_triangle, bounding_box_info, check_mesh_watertight, compute_curvature,
    curvature_plot, cut_mesh_with_plane, decimate_mesh, evaluate_and_repair, face_info, fill_holes,
    flip_normals, harmonize_normals, mesh_boolean_difference, mesh_boolean_intersection,
    mesh_boolean_union, mesh_cross_sections, mesh_section_from_plane, regular_solid, remesh,
    remove_component, remove_components_by_size, scale_mesh, segment_mesh, smooth_mesh,
    split_mesh_by_components, subdivide_mesh, trim_mesh, unwrap_face, unwrap_mesh,
};
pub use tessellate::{Mesh, Triangle, merge_meshes, tessellate_face, tessellate_solid, tessellate_solid_parallel};

#[cfg(test)]
mod thread_safety_tests {
    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn io_types_are_send_sync() {
        assert_send_sync::<crate::Mesh>();
        assert_send_sync::<crate::Triangle>();
        assert_send_sync::<crate::StepWriter>();
        assert_send_sync::<crate::IgesWriter>();
    }
}
