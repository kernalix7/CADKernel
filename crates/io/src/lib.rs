//! File I/O, tessellation, and mesh processing for the CAD kernel.
//!
//! This crate provides import/export for 18 file formats, face and solid
//! tessellation into indexed triangle meshes, 23+ mesh processing operations,
//! and technical drawing generation with SVG/PDF output.
//!
//! # Supported Formats
//!
//! | Format | Import | Export | Data Type |
//! |--------|--------|--------|-----------|
//! | STL (ASCII & binary) | `import_stl` | `export_stl_ascii`, `export_stl_binary` | `Mesh` |
//! | Wavefront OBJ | `import_obj` | `export_obj` | `Mesh` |
//! | glTF 2.0 | `import_gltf` | `export_gltf` | `Mesh` |
//! | STEP (ISO 10303-21) | `import_step` | `export_step` | `BRepModel` |
//! | IGES | `import_iges` | `export_iges` | `BRepModel` |
//! | DXF | `import_dxf` | `export_dxf` | `Mesh` |
//! | DWG | `import_dwg` | `export_dwg` | `Mesh` |
//! | PLY | `import_ply` | `export_ply` | `Mesh` |
//! | 3MF | `import_3mf` | `export_3mf` | `Mesh` |
//! | BREP | `import_brep` | `export_brep` | `BRepModel` |
//! | JSON | `import_json` | `export_json` | `BRepModel` |
//! | SVG | `import_svg` | `write_svg` | `Mesh` / `SvgDocument` |
//! | PDF | `import_pdf` | `export_pdf` | `Vec<u8>` |
//! | Collada (DAE) | `import_dae` | `export_dae` | `Mesh` |
//! | AMF | `import_amf` | `export_amf` | `Mesh` |
//! | VRML 2.0 | `import_vrml` | `export_vrml` | `Mesh` |
//! | OCA/GCAD | `import_oca` | `export_oca` | `Mesh` |
//! | Native (.cadk) | `load_project` | `save_project` | `BRepModel` |
//!
//! # Key Modules
//!
//! - [`tessellate`] -- B-Rep face/solid tessellation into indexed triangle meshes.
//! - [`mesh_ops`] -- 23+ mesh operations (decimate, subdivide, smooth, boolean, repair).
//! - [`techdraw`] -- Technical drawing generation with orthographic projection and annotations.
//! - [`step`] / [`iges`] -- ISO standard B-Rep format readers/writers.

pub mod amf;
pub mod brep_format;
pub mod collada;
pub mod dxf;
pub mod dwg;
pub mod gltf;
pub mod iges;
pub mod json;
pub mod mcp;
pub mod native;
pub mod oca;
pub mod obj;
pub mod pdf;
pub mod ply;
pub mod step;
pub mod stl;
pub mod svg;
pub mod techdraw;
pub mod threemf;
pub mod vrml;
pub mod mesh_ops;
pub mod tessellate;

pub use amf::{export_amf, import_amf, write_amf};
pub use brep_format::{export_brep, import_brep, write_brep};
pub use dxf::{
    export_centerlines_dxf, export_cosmetic_lines_dxf, export_drawing_dxf, export_dxf,
    export_hatch_dxf, export_leaders_dxf, export_text_annotations_dxf, import_dxf, write_dxf,
};
pub use collada::{export_dae, import_dae, write_dae};
pub use dwg::{export_dwg, import_dwg, write_dwg};
pub use gltf::{export_gltf, import_gltf, write_gltf};
pub use pdf::{
    PdfImportResult, PdfPage, PdfPathCmd, PdfText, PdfVector, export_pdf, import_pdf, write_pdf,
};
pub use iges::{
    IgesEntity, IgesEntityType, IgesWriter, export_iges, export_iges_mesh, import_iges,
    parse_iges, read_iges_lines, read_iges_points,
};
pub use json::{export_json, import_json, model_from_json, model_to_json, read_json, write_json};
pub use mcp::{McpError, McpRequest, McpResponse, McpServer, McpToolDef};
pub use native::{CADK_EXTENSION, SceneObjectData, load_project, load_scene, save_project, save_scene};
pub use oca::{export_oca, import_oca, write_oca};
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
pub use svg::{SvgDocument, SvgElement, SvgStyle, import_svg, profile_to_svg, write_svg};
pub use techdraw::{
    Alignment, ArcLengthDimension, AreaAnnotation, BalloonAnnotation, BitmapImage,
    BoltCircleCenterlines, BreakLine, CenterMark, Centerline, ChainDimension, ChamferDimension,
    ClipGroup, CoordinateDimension, CosmeticArc, CosmeticCircle, CosmeticLine, CosmeticLineStyle,
    CosmeticThread, CosmeticVertex, CosmeticVertexStyle, Dimension, DimensionType,
    DrawingElement, DrawingSheet, DrawingView, ExtentDimension, FormattedDimension,
    HatchPattern, HatchPatternName, HoleShaftFit, LeaderLine, PaperTemplate, ProjectedEdge,
    ProjectionDir, ProjectionType, RichTextAnnotation, StackOrder, SurfaceFinishSymbol,
    SvgInsert, TextAnnotation, WeldContour, WeldFinish, WeldSymbol, WeldSymbolFull, WeldType,
    active_view, align_elements, angle_from_3_points, arc_length_dimension,
    arc_length_dimension_to_svg, area_annotation, area_annotation_to_svg,
    axonometric_length_dimension, balloon_annotation, balloon_annotation_to_svg,
    bitmap_image, bolt_circle_centerlines, bolt_circle_centerlines_to_svg,
    break_line_to_svg, broken_view, center_mark_to_svg, centerline_between_lines,
    centerline_between_points, centerline_on_face, centerline_to_svg, chain_dimension,
    chain_dimension_to_svg, chamfer_dimension, chamfer_dimension_to_svg, clip_group,
    clip_group_to_svg, complex_section_view, contextual_dimension, coordinate_dimension,
    coordinate_dimension_to_svg, cosmetic_arc, cosmetic_arc_to_svg, cosmetic_circle,
    cosmetic_circle_to_svg, cosmetic_line, cosmetic_line_to_svg, cosmetic_parallel_line,
    cosmetic_perpendicular_line, cosmetic_thread_external, cosmetic_thread_internal,
    cosmetic_thread_to_svg, cosmetic_vertex, cosmetic_vertex_to_svg, detail_view,
    dimension_to_svg, drawing_to_svg, edit_line_appearance, extent_dimension_to_svg,
    formatted_dimension_to_svg, geometric_hatch, geometric_hatch_to_svg, hatch_pattern_to_svg,
    hole_shaft_fit, hole_shaft_fit_to_svg, hv_extent_dimension, insert_svg, leader_line_to_svg,
    lock_element, page_from_template, print_all_pages, project_shape_2d, project_solid,
    redraw_page, repair_dimension_refs, rich_text_annotation, rich_text_annotation_to_svg,
    section_view, set_decimal_places, set_prefix_symbol, share_view, stack_order,
    surface_finish_to_svg, text_annotation_to_svg, three_view_drawing, toggle_edge_visibility,
    update_template_fields, weld_symbol, weld_symbol_full_to_svg, weld_symbol_to_svg,
};
pub use threemf::{export_3mf, import_3mf, write_3mf};
pub use vrml::{export_vrml, import_vrml, write_vrml};
pub use mesh_ops::{
    FaceInfo, MeshBoundingBox, MeshRepairReport, MeshSegment, RegularSolidType, UnwrapResult,
    UvCoord, add_triangle, bounding_box_info, check_mesh_watertight, close_holes,
    compute_curvature, curvature_plot, cut_mesh_with_plane, decimate_mesh, evaluate_and_repair,
    face_info, fill_holes, flip_normals, harmonize_normals, mesh_boolean_difference,
    mesh_boolean_intersection, mesh_boolean_union, mesh_cross_sections, mesh_section_from_plane,
    regular_solid, remesh, remove_component, remove_components_by_size, scale_mesh,
    segmentation_best_fit, segment_mesh, smooth_mesh, split_mesh_by_components, subdivide_mesh,
    trim_mesh, unwrap_face, unwrap_mesh,
};
pub use tessellate::{
    Mesh, Triangle, merge_meshes, tessellate_face, tessellate_solid, tessellate_solid_lod,
    tessellate_solid_parallel, tessellate_solid_with_face_map, tessellate_solid_with_options,
};

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
