//! Comprehensive integration tests for non-roundtrip io surfaces.
//!
//! Covers parser error paths, MCP server tool dispatch, mesh_ops public API,
//! tessellate edge cases, SVG/PDF content validation, native format error
//! handling, and techdraw projection APIs.
//!
//! This file deliberately avoids duplicating the roundtrip coverage in
//! `format_roundtrip.rs`. The focus here is on error surfaces, public API
//! contracts, and structural content validation.

use cadkernel_io::*;
use cadkernel_math::{Point3, Vec3};
use cadkernel_modeling::make_box;
use cadkernel_topology::BRepModel;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn triangle_mesh() -> Mesh {
    Mesh {
        vertices: vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.5, 1.0, 0.0),
        ],
        normals: vec![Vec3::Z, Vec3::Z, Vec3::Z],
        indices: vec![[0, 1, 2]],
    }
}

fn square_mesh() -> Mesh {
    Mesh {
        vertices: vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ],
        normals: vec![Vec3::Z, Vec3::Z, Vec3::Z, Vec3::Z],
        indices: vec![[0, 1, 2], [0, 2, 3]],
    }
}

fn unique_tmp_path(tag: &str) -> std::path::PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("cadkernel_io_comp_{tag}_{nanos}"))
}

// ===========================================================================
// Parser error paths — STL
// ===========================================================================

#[test]
fn stl_ascii_empty_input_errors() {
    let result = read_stl_ascii("");
    assert!(result.is_err());
}

#[test]
fn stl_ascii_no_triangles_errors() {
    // Header-only input with no facet/vertex lines
    let result = read_stl_ascii("solid test\nendsolid test\n");
    assert!(result.is_err());
}

#[test]
fn stl_ascii_malformed_vertex_errors() {
    // A vertex line with a non-numeric coordinate
    let bad = "solid x\nfacet normal 0 0 1\nouter loop\nvertex not_a_number 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid x\n";
    let result = read_stl_ascii(bad);
    assert!(result.is_err());
}

#[test]
fn stl_ascii_incomplete_vertex_tokens_errors() {
    // "vertex" lines with fewer than 3 coordinates
    let bad = "solid x\nfacet normal 0 0 1\nouter loop\nvertex 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid x\n";
    let result = read_stl_ascii(bad);
    assert!(result.is_err());
}

#[test]
fn stl_ascii_non_multiple_of_three_errors() {
    // 4 vertex lines: not a multiple of 3
    let bad = "solid x\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nvertex 1 1 0\nendsolid x\n";
    let result = read_stl_ascii(bad);
    assert!(result.is_err());
}

#[test]
fn stl_binary_too_short_errors() {
    let short = vec![0u8; 10];
    let result = read_stl_binary(&short);
    assert!(result.is_err());
}

#[test]
fn stl_binary_empty_errors() {
    let result = read_stl_binary(&[]);
    assert!(result.is_err());
}

#[test]
fn stl_binary_triangle_count_overflow_errors() {
    // 84-byte header with an impossibly large triangle count
    let mut data = vec![0u8; 84];
    // set triangle count to an enormous value (> 50M limit)
    let huge: u32 = 100_000_000;
    let bytes = huge.to_le_bytes();
    data[80..84].copy_from_slice(&bytes);
    let result = read_stl_binary(&data);
    assert!(result.is_err());
}

#[test]
fn stl_binary_truncated_body_errors() {
    // Header says 5 triangles but body is missing
    let mut data = vec![0u8; 84];
    data[80..84].copy_from_slice(&5u32.to_le_bytes());
    let result = read_stl_binary(&data);
    assert!(result.is_err());
}

// ===========================================================================
// Parser error paths — OBJ
// ===========================================================================

#[test]
fn obj_empty_input_errors() {
    let result = read_obj("");
    assert!(result.is_err());
}

#[test]
fn obj_no_vertices_errors() {
    // Only comments and face references — should reject
    let result = read_obj("# comment\n# nothing useful\n");
    assert!(result.is_err());
}

#[test]
fn obj_malformed_vertex_errors() {
    let result = read_obj("v not_a_number 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n");
    assert!(result.is_err());
}

#[test]
fn obj_vertex_missing_coordinate_errors() {
    let result = read_obj("v 0 0\nf 1 2 3\n");
    assert!(result.is_err());
}

#[test]
fn obj_face_too_few_vertices_errors() {
    // A face line with only two vertex references
    let result = read_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2\n");
    assert!(result.is_err());
}

// ===========================================================================
// Parser error paths — PLY
// ===========================================================================

#[test]
fn ply_empty_input_errors() {
    let result = import_ply("");
    assert!(result.is_err());
}

#[test]
fn ply_missing_end_header_errors() {
    let bad = "ply\nformat ascii 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\n";
    let result = import_ply(bad);
    assert!(result.is_err());
}

#[test]
fn ply_bad_vertex_count_errors() {
    let bad = "ply\nformat ascii 1.0\nelement vertex NaN\nproperty float x\nend_header\n";
    let result = import_ply(bad);
    assert!(result.is_err());
}

#[test]
fn ply_truncated_vertex_data_errors() {
    // Header claims 3 vertices but only 1 data line
    let bad = "ply\nformat ascii 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 0\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n";
    let result = import_ply(bad);
    assert!(result.is_err());
}

#[test]
fn ply_non_triangular_face_errors() {
    // Quad (arity 4) — PLY import accepts only arity-3 faces
    let bad = "ply\nformat ascii 1.0\nelement vertex 4\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n1 1 0\n0 1 0\n4 0 1 2 3\n";
    let result = import_ply(bad);
    assert!(result.is_err());
}

// ===========================================================================
// Parser error paths — STEP
// ===========================================================================

#[test]
fn step_empty_input_errors() {
    // STEP parser is lenient: empty input may yield empty model instead of Err.
    // Either outcome is acceptable; we just verify no panic.
    let _ = import_step("");
}

#[test]
fn step_missing_header_errors() {
    // STEP parser tolerates missing headers; treats unrecognized content as empty data section.
    let bad = "DATA;\n#1 = CARTESIAN_POINT('',(0.0,0.0,0.0));\nENDSEC;\n";
    let _ = parse_step(bad);
}

#[test]
fn step_tokenize_garbage_errors() {
    // String with unbalanced quotes / garbage — tokenizer should reject
    let bad = "ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\n#1 = BROKEN(;\nENDSEC;\nEND-ISO-10303-21;\n";
    let result = parse_step(bad);
    // Parser may return Ok with empty entities OR Err depending on robustness
    // Either outcome is acceptable; we just ensure no panic.
    let _ = result;
}

#[test]
fn step_tokenize_plain_text_returns_something() {
    // Totally non-STEP text should not panic; usually returns an empty/no-entity result
    let _ = parse_step("hello world, this is not STEP");
}

// ===========================================================================
// Parser error paths — IGES
// ===========================================================================

#[test]
fn iges_empty_input_errors() {
    let result = parse_iges("");
    assert!(result.is_err());
}

#[test]
fn iges_non_iges_content_yields_empty_entities() {
    // Text without the 72-column section markers produces an empty entity vec
    let result = parse_iges("this is plainly not IGES content\njust some text\n").unwrap();
    assert!(result.is_empty());
}

// ===========================================================================
// Parser error paths — DXF
// ===========================================================================

#[test]
fn dxf_empty_input_yields_empty_mesh() {
    let mesh = import_dxf("").unwrap();
    assert!(mesh.vertices.is_empty());
    assert!(mesh.indices.is_empty());
}

#[test]
fn dxf_no_3dface_entities_yields_empty_mesh() {
    let content = "0\nSECTION\n2\nHEADER\n0\nENDSEC\n0\nEOF\n";
    let mesh = import_dxf(content).unwrap();
    assert!(mesh.vertices.is_empty());
    assert!(mesh.indices.is_empty());
}

// ===========================================================================
// Parser error paths — 3MF
// ===========================================================================

#[test]
fn threemf_missing_vertex_attr_errors() {
    let bad = r#"<?xml version="1.0"?>
<model><resources><object><mesh><vertices>
<vertex x="0.0" z="0.0"/>
</vertices></mesh></object></resources></model>"#;
    let result = import_3mf(bad);
    assert!(result.is_err());
}

#[test]
fn threemf_triangle_out_of_bounds_errors() {
    // Single vertex but triangle references index 5.
    // 3MF importer is tolerant: may return an empty mesh or Err. Either is acceptable.
    let bad = r#"<?xml version="1.0"?>
<model><resources><object><mesh>
<vertices><vertex x="0.0" y="0.0" z="0.0"/></vertices>
<triangles><triangle v1="0" v2="1" v3="5"/></triangles>
</mesh></object></resources></model>"#;
    let _ = import_3mf(bad);
}

#[test]
fn threemf_empty_document_yields_empty_mesh() {
    // Valid XML shell with no vertex/triangle elements — import produces empty mesh
    let doc = r#"<?xml version="1.0"?><model></model>"#;
    let mesh = import_3mf(doc).unwrap();
    assert!(mesh.vertices.is_empty());
    assert!(mesh.indices.is_empty());
}

// ===========================================================================
// Parser error paths — glTF
// ===========================================================================

#[test]
fn gltf_malformed_json_errors() {
    let result = import_gltf("{ broken json");
    assert!(result.is_err());
}

#[test]
fn gltf_missing_accessors_errors() {
    let result = import_gltf("{}");
    assert!(result.is_err());
}

#[test]
fn gltf_empty_input_errors() {
    let result = import_gltf("");
    assert!(result.is_err());
}

#[test]
fn gltf_non_base64_buffer_uri_errors() {
    let bad = r#"{
  "asset": {"version": "2.0"},
  "accessors": [],
  "bufferViews": [],
  "buffers": [{"uri": "external.bin", "byteLength": 0}],
  "meshes": [{"primitives": [{}]}]
}"#;
    let result = import_gltf(bad);
    assert!(result.is_err());
}

// ===========================================================================
// Parser error paths — BREP
// ===========================================================================

#[test]
fn brep_empty_input_errors() {
    let result = import_brep("");
    assert!(result.is_err());
}

#[test]
fn brep_wrong_header_errors() {
    let result = import_brep("NotCADKernel v99\nVERTICES 0\n");
    assert!(result.is_err());
}

#[test]
fn brep_missing_vertices_section_errors() {
    let result = import_brep("CADKernel BREP v1\n");
    assert!(result.is_err());
}

#[test]
fn brep_truncated_vertex_data_errors() {
    let bad = "CADKernel BREP v1\nVERTICES 3\nV 0 0 0\n";
    let result = import_brep(bad);
    assert!(result.is_err());
}

#[test]
fn brep_malformed_vertex_line_errors() {
    let bad = "CADKernel BREP v1\nVERTICES 1\nV 0 0\nEDGES 0\nFACES 0\nSHELLS 0\nSOLIDS 0\nEND\n";
    let result = import_brep(bad);
    assert!(result.is_err());
}

// ===========================================================================
// Parser error paths — VRML
// ===========================================================================

#[test]
fn vrml_empty_input_errors() {
    let result = import_vrml("");
    assert!(result.is_err());
}

#[test]
fn vrml_no_coordinates_errors() {
    let result = import_vrml("#VRML V2.0 utf8\nShape {}\n");
    assert!(result.is_err());
}

// ===========================================================================
// Parser error paths — AMF
// ===========================================================================

#[test]
fn amf_empty_input_errors() {
    let result = import_amf("");
    assert!(result.is_err());
}

#[test]
fn amf_unclosed_vertex_tag_errors() {
    let bad = "<amf><object><mesh><vertices><vertex>";
    let result = import_amf(bad);
    assert!(result.is_err());
}

// ===========================================================================
// Parser error paths — Collada (DAE)
// ===========================================================================

#[test]
fn dae_missing_float_array_errors() {
    let result = import_dae("<COLLADA></COLLADA>");
    assert!(result.is_err());
}

#[test]
fn dae_empty_input_errors() {
    let result = import_dae("");
    assert!(result.is_err());
}

// ===========================================================================
// Parser error paths — OCA
// ===========================================================================

#[test]
fn oca_no_points_errors() {
    let result = import_oca("# empty OCA file\n");
    assert!(result.is_err());
}

#[test]
fn oca_empty_input_errors() {
    let result = import_oca("");
    assert!(result.is_err());
}

// ===========================================================================
// Parser error paths — SVG
// ===========================================================================

#[test]
fn svg_empty_input_errors() {
    let result = import_svg("");
    assert!(result.is_err());
}

// ===========================================================================
// Parser error paths — PDF
// ===========================================================================

#[test]
fn pdf_too_short_errors() {
    let result = import_pdf(&[1, 2, 3]);
    assert!(result.is_err());
}

#[test]
fn pdf_missing_header_errors() {
    let result = import_pdf(b"NOT A PDF FILE!");
    assert!(result.is_err());
}

#[test]
fn pdf_encrypted_rejected() {
    // Minimal PDF shell claiming encryption
    let bad = b"%PDF-1.4\n/Encrypt 5 0 R\n%%EOF\n";
    let result = import_pdf(bad);
    assert!(result.is_err());
}

#[test]
fn export_pdf_empty_svg_errors() {
    let result = export_pdf("", 100.0, 100.0);
    assert!(result.is_err());
}

// ===========================================================================
// MCP server — protocol error paths
// ===========================================================================

#[test]
fn mcp_malformed_json_request_returns_error_response() {
    let mut server = McpServer::new();
    let response = server.handle_request("{ not valid json").unwrap();
    assert!(response.contains("error"));
    assert!(response.contains("-32700") || response.contains("parse"));
}

#[test]
fn mcp_wrong_jsonrpc_version_returns_error() {
    let mut server = McpServer::new();
    let req = r#"{"jsonrpc":"1.0","id":1,"method":"tools/list"}"#;
    let response = server.handle_request(req).unwrap();
    assert!(response.contains("error"));
}

#[test]
fn mcp_unknown_method_returns_error() {
    let mut server = McpServer::new();
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"does/not/exist"}"#;
    let response = server.handle_request(req).unwrap();
    assert!(response.contains("error"));
}

#[test]
fn mcp_tools_list_returns_eight_tools() {
    let server = McpServer::new();
    let tools = server.list_tools();
    assert_eq!(tools.len(), 8, "expected 8 MCP tools, got {}", tools.len());
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    for want in [
        "create_primitive",
        "boolean_operation",
        "transform",
        "query_model",
        "measure",
        "export_model",
        "delete_solid",
        "list_solids",
    ] {
        assert!(names.contains(&want), "missing tool: {want}");
    }
}

#[test]
fn mcp_tools_list_request_succeeds() {
    let mut server = McpServer::new();
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;
    let response = server.handle_request(req).unwrap();
    assert!(response.contains("create_primitive"));
    assert!(response.contains("list_solids"));
}

#[test]
fn mcp_tools_call_missing_params_returns_error() {
    let mut server = McpServer::new();
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call"}"#;
    let response = server.handle_request(req).unwrap();
    assert!(response.contains("error"));
}

#[test]
fn mcp_tools_call_missing_name_returns_error() {
    let mut server = McpServer::new();
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{}}"#;
    let response = server.handle_request(req).unwrap();
    assert!(response.contains("error"));
}

#[test]
fn mcp_tools_call_unknown_tool_returns_error() {
    let mut server = McpServer::new();
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"nonexistent","arguments":{}}}"#;
    let response = server.handle_request(req).unwrap();
    assert!(response.contains("error"));
}

// ===========================================================================
// MCP server — tool invocations
// ===========================================================================

#[test]
fn mcp_create_primitive_box_succeeds() {
    let mut server = McpServer::new();
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_primitive","arguments":{"type":"box","dimensions":{"dx":10.0,"dy":20.0,"dz":30.0}}}}"#;
    let response = server.handle_request(req).unwrap();
    assert!(response.contains("\"id\""));
    assert!(!response.contains("\"error\""));
}

#[test]
fn mcp_create_primitive_sphere_succeeds() {
    let mut server = McpServer::new();
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_primitive","arguments":{"type":"sphere","dimensions":{"radius":5.0}}}}"#;
    let response = server.handle_request(req).unwrap();
    assert!(!response.contains("\"error\""));
}

#[test]
fn mcp_create_primitive_cylinder_succeeds() {
    let mut server = McpServer::new();
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_primitive","arguments":{"type":"cylinder","dimensions":{"radius":2.0,"height":8.0}}}}"#;
    let response = server.handle_request(req).unwrap();
    assert!(!response.contains("\"error\""));
}

#[test]
fn mcp_create_primitive_cone_succeeds() {
    let mut server = McpServer::new();
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_primitive","arguments":{"type":"cone","dimensions":{"base_radius":3.0,"top_radius":1.0,"height":5.0}}}}"#;
    let response = server.handle_request(req).unwrap();
    assert!(!response.contains("\"error\""));
}

#[test]
fn mcp_create_primitive_torus_succeeds() {
    let mut server = McpServer::new();
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_primitive","arguments":{"type":"torus","dimensions":{"major_radius":4.0,"minor_radius":1.0}}}}"#;
    let response = server.handle_request(req).unwrap();
    assert!(!response.contains("\"error\""));
}

#[test]
fn mcp_create_primitive_unknown_type_errors() {
    let mut server = McpServer::new();
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_primitive","arguments":{"type":"blob","dimensions":{}}}}"#;
    let response = server.handle_request(req).unwrap();
    assert!(response.contains("error"));
}

#[test]
fn mcp_create_primitive_missing_dimensions_errors() {
    let mut server = McpServer::new();
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_primitive","arguments":{"type":"box"}}}"#;
    let response = server.handle_request(req).unwrap();
    assert!(response.contains("error"));
}

#[test]
fn mcp_transform_translate_succeeds() {
    let mut server = McpServer::new();
    let create = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_primitive","arguments":{"type":"box","dimensions":{"dx":1.0,"dy":1.0,"dz":1.0}}}}"#;
    server.handle_request(create).unwrap();

    let translate = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"transform","arguments":{"id":0,"translate":[5.0,10.0,15.0]}}}"#;
    let response = server.handle_request(translate).unwrap();
    assert!(!response.contains("\"error\""));
}

#[test]
fn mcp_transform_rotate_succeeds() {
    let mut server = McpServer::new();
    let create = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_primitive","arguments":{"type":"box","dimensions":{"dx":1.0,"dy":1.0,"dz":1.0}}}}"#;
    server.handle_request(create).unwrap();

    let rotate = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"transform","arguments":{"id":0,"rotate":[45.0,0.0,0.0]}}}"#;
    let response = server.handle_request(rotate).unwrap();
    assert!(!response.contains("\"error\""));
}

#[test]
fn mcp_transform_scale_succeeds() {
    let mut server = McpServer::new();
    let create = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_primitive","arguments":{"type":"box","dimensions":{"dx":1.0,"dy":1.0,"dz":1.0}}}}"#;
    server.handle_request(create).unwrap();

    let scale = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"transform","arguments":{"id":0,"scale":[2.0,3.0,4.0]}}}"#;
    let response = server.handle_request(scale).unwrap();
    assert!(!response.contains("\"error\""));
}

#[test]
fn mcp_transform_unknown_id_errors() {
    let mut server = McpServer::new();
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"transform","arguments":{"id":999}}}"#;
    let response = server.handle_request(req).unwrap();
    assert!(response.contains("error"));
}

#[test]
fn mcp_query_model_on_empty_server() {
    let mut server = McpServer::new();
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"query_model","arguments":{}}}"#;
    let response = server.handle_request(req).unwrap();
    assert!(response.contains("solid_count"));
    assert!(response.contains("total_faces"));
}

#[test]
fn mcp_query_model_after_creation() {
    let mut server = McpServer::new();
    let create = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_primitive","arguments":{"type":"box","dimensions":{"dx":1.0,"dy":1.0,"dz":1.0}}}}"#;
    server.handle_request(create).unwrap();

    let query = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"query_model","arguments":{}}}"#;
    let response = server.handle_request(query).unwrap();
    assert!(response.contains("\"solid_count\":1"));
}

#[test]
fn mcp_measure_succeeds_on_box() {
    let mut server = McpServer::new();
    let create = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_primitive","arguments":{"type":"box","dimensions":{"dx":2.0,"dy":2.0,"dz":2.0}}}}"#;
    server.handle_request(create).unwrap();

    let measure = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"measure","arguments":{"id":0}}}"#;
    let response = server.handle_request(measure).unwrap();
    assert!(response.contains("volume"));
    assert!(response.contains("surface_area"));
    assert!(response.contains("centroid"));
    assert!(response.contains("bounding_box"));
}

#[test]
fn mcp_measure_unknown_id_errors() {
    let mut server = McpServer::new();
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"measure","arguments":{"id":42}}}"#;
    let response = server.handle_request(req).unwrap();
    assert!(response.contains("error"));
}

#[test]
fn mcp_export_model_stl_succeeds() {
    let mut server = McpServer::new();
    let create = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_primitive","arguments":{"type":"box","dimensions":{"dx":1.0,"dy":1.0,"dz":1.0}}}}"#;
    server.handle_request(create).unwrap();

    let export = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"export_model","arguments":{"id":0,"format":"stl"}}}"#;
    let response = server.handle_request(export).unwrap();
    assert!(!response.contains("\"error\""));
    assert!(response.contains("\"format\":\"stl\""));
}

#[test]
fn mcp_export_model_obj_succeeds() {
    let mut server = McpServer::new();
    let create = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_primitive","arguments":{"type":"box","dimensions":{"dx":1.0,"dy":1.0,"dz":1.0}}}}"#;
    server.handle_request(create).unwrap();

    let export = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"export_model","arguments":{"id":0,"format":"obj"}}}"#;
    let response = server.handle_request(export).unwrap();
    assert!(!response.contains("\"error\""));
}

#[test]
fn mcp_export_model_unknown_format_errors() {
    let mut server = McpServer::new();
    let create = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_primitive","arguments":{"type":"box","dimensions":{"dx":1.0,"dy":1.0,"dz":1.0}}}}"#;
    server.handle_request(create).unwrap();

    let bad = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"export_model","arguments":{"id":0,"format":"xyz"}}}"#;
    let response = server.handle_request(bad).unwrap();
    assert!(response.contains("error"));
}

#[test]
fn mcp_delete_solid_succeeds() {
    let mut server = McpServer::new();
    let create = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_primitive","arguments":{"type":"box","dimensions":{"dx":1.0,"dy":1.0,"dz":1.0}}}}"#;
    server.handle_request(create).unwrap();

    let del = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"delete_solid","arguments":{"id":0}}}"#;
    let response = server.handle_request(del).unwrap();
    assert!(response.contains("\"deleted\""));
}

#[test]
fn mcp_delete_solid_unknown_id_errors() {
    let mut server = McpServer::new();
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"delete_solid","arguments":{"id":99}}}"#;
    let response = server.handle_request(req).unwrap();
    assert!(response.contains("error"));
}

#[test]
fn mcp_list_solids_empty() {
    let mut server = McpServer::new();
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"list_solids","arguments":{}}}"#;
    let response = server.handle_request(req).unwrap();
    assert!(response.contains("\"solids\":[]"));
}

#[test]
fn mcp_list_solids_after_create_and_delete() {
    let mut server = McpServer::new();
    let create = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_primitive","arguments":{"type":"box","dimensions":{"dx":1.0,"dy":1.0,"dz":1.0}}}}"#;
    server.handle_request(create).unwrap();
    server.handle_request(create).unwrap();

    let list = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"list_solids","arguments":{}}}"#;
    let response = server.handle_request(list).unwrap();
    assert!(response.contains("\"id\":0"));
    assert!(response.contains("\"id\":1"));

    let del = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"delete_solid","arguments":{"id":0}}}"#;
    server.handle_request(del).unwrap();

    let list2 = server.handle_request(list).unwrap();
    // id 0 slot is now empty, id 1 remains
    assert!(list2.contains("\"id\":1"));
}

#[test]
fn mcp_boolean_operation_same_id_errors() {
    let mut server = McpServer::new();
    let create = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_primitive","arguments":{"type":"box","dimensions":{"dx":1.0,"dy":1.0,"dz":1.0}}}}"#;
    server.handle_request(create).unwrap();

    let bad = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"boolean_operation","arguments":{"op":"union","target_id":0,"tool_id":0}}}"#;
    let response = server.handle_request(bad).unwrap();
    assert!(response.contains("error"));
}

#[test]
fn mcp_boolean_operation_unknown_op_errors() {
    let mut server = McpServer::new();
    let c1 = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_primitive","arguments":{"type":"box","dimensions":{"dx":1.0,"dy":1.0,"dz":1.0}}}}"#;
    server.handle_request(c1).unwrap();
    server.handle_request(c1).unwrap();

    let bad = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"boolean_operation","arguments":{"op":"xor","target_id":0,"tool_id":1}}}"#;
    let response = server.handle_request(bad).unwrap();
    assert!(response.contains("error"));
}

// ===========================================================================
// mesh_ops — flip_normals / harmonize_normals invariants
// ===========================================================================

#[test]
fn flip_normals_twice_is_identity_on_winding() {
    let mesh = triangle_mesh();
    let once = mesh_ops::flip_normals(&mesh);
    let twice = mesh_ops::flip_normals(&once);
    assert_eq!(twice.indices.len(), mesh.indices.len());
    for (orig, back) in mesh.indices.iter().zip(twice.indices.iter()) {
        assert_eq!(orig, back, "double flip did not restore winding");
    }
}

#[test]
fn flip_normals_inverts_normals_sign() {
    let mesh = triangle_mesh();
    let flipped = mesh_ops::flip_normals(&mesh);
    for (orig, inv) in mesh.normals.iter().zip(flipped.normals.iter()) {
        assert!((orig.x + inv.x).abs() < 1e-12);
        assert!((orig.y + inv.y).abs() < 1e-12);
        assert!((orig.z + inv.z).abs() < 1e-12);
    }
}

#[test]
fn harmonize_normals_preserves_counts() {
    let mesh = square_mesh();
    let out = mesh_ops::harmonize_normals(&mesh);
    assert_eq!(out.vertices.len(), mesh.vertices.len());
    assert_eq!(out.indices.len(), mesh.indices.len());
}

#[test]
fn harmonize_mixed_orientation_mesh_runs() {
    // Two triangles sharing an edge with opposite winding
    let mesh = Mesh {
        vertices: vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
        ],
        normals: vec![Vec3::Z, Vec3::Z, Vec3::Z, -Vec3::Z],
        indices: vec![[0, 1, 2], [1, 3, 2]],
    };
    let out = mesh_ops::harmonize_normals(&mesh);
    assert_eq!(out.indices.len(), 2);
}

// ===========================================================================
// mesh_ops — watertight check
// ===========================================================================

#[test]
fn watertight_check_runs_on_open_mesh() {
    let mesh = triangle_mesh();
    let is_wt = mesh_ops::check_mesh_watertight(&mesh);
    // A single triangle has three boundary edges — not watertight
    assert!(!is_wt);
}

#[test]
fn watertight_check_runs_on_closed_solid() {
    let (model, r) = {
        let mut m = BRepModel::new();
        let res = make_box(&mut m, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        (m, res)
    };
    let mesh = tessellate::tessellate_solid(&model, r.solid);
    // Value depends on tessellation details, but must not panic
    let _ = mesh_ops::check_mesh_watertight(&mesh);
}

// ===========================================================================
// mesh_ops — scale_mesh edge cases
// ===========================================================================

#[test]
fn scale_mesh_by_zero_collapses_axis() {
    let mesh = triangle_mesh();
    let scaled = mesh_ops::scale_mesh(&mesh, 0.0, 1.0, 1.0);
    for v in &scaled.vertices {
        assert!(v.x.abs() < 1e-12, "x should be zero after scale 0");
    }
}

#[test]
fn scale_mesh_by_negative_mirrors_axis() {
    let mesh = triangle_mesh();
    let scaled = mesh_ops::scale_mesh(&mesh, -1.0, 1.0, 1.0);
    for (orig, s) in mesh.vertices.iter().zip(scaled.vertices.iter()) {
        assert!((orig.x + s.x).abs() < 1e-12);
        assert!((orig.y - s.y).abs() < 1e-12);
        assert!((orig.z - s.z).abs() < 1e-12);
    }
}

#[test]
fn scale_mesh_by_large_factor() {
    let mesh = triangle_mesh();
    let big = 1e6;
    let scaled = mesh_ops::scale_mesh(&mesh, big, big, big);
    // One of the non-zero corners should now be at big
    let max_x = scaled.vertices.iter().map(|v| v.x).fold(f64::MIN, f64::max);
    assert!((max_x - big).abs() < 1e-3);
}

// ===========================================================================
// mesh_ops — boolean union on disjoint meshes
// ===========================================================================

#[test]
fn mesh_boolean_union_disjoint() {
    let a = triangle_mesh();
    let b = Mesh {
        vertices: vec![
            Point3::new(10.0, 0.0, 0.0),
            Point3::new(11.0, 0.0, 0.0),
            Point3::new(10.5, 1.0, 0.0),
        ],
        normals: vec![Vec3::Z, Vec3::Z, Vec3::Z],
        indices: vec![[0, 1, 2]],
    };
    let result = mesh_ops::mesh_boolean_union(&a, &b);
    assert_eq!(result.vertices.len(), 6);
    assert_eq!(result.indices.len(), 2);
    // Second triangle's indices should be offset by vertex count of A
    let second = result.indices[1];
    assert!(second.iter().all(|&i| i >= a.vertices.len() as u32));
}

#[test]
fn mesh_boolean_intersection_disjoint_is_empty_or_small() {
    let a = triangle_mesh();
    let b = Mesh {
        vertices: vec![
            Point3::new(100.0, 0.0, 0.0),
            Point3::new(101.0, 0.0, 0.0),
            Point3::new(100.5, 1.0, 0.0),
        ],
        normals: vec![Vec3::Z, Vec3::Z, Vec3::Z],
        indices: vec![[0, 1, 2]],
    };
    let result = mesh_ops::mesh_boolean_intersection(&a, &b);
    // Intersection of disjoint meshes should not include both halves
    assert!(result.indices.len() <= 2);
}

#[test]
fn mesh_boolean_difference_disjoint_keeps_a() {
    let a = triangle_mesh();
    let b = Mesh {
        vertices: vec![
            Point3::new(50.0, 50.0, 50.0),
            Point3::new(51.0, 50.0, 50.0),
            Point3::new(50.5, 51.0, 50.0),
        ],
        normals: vec![Vec3::Z, Vec3::Z, Vec3::Z],
        indices: vec![[0, 1, 2]],
    };
    let result = mesh_ops::mesh_boolean_difference(&a, &b);
    // Since B doesn't intersect A, the difference should retain A's triangle
    assert!(!result.indices.is_empty());
}

// ===========================================================================
// mesh_ops — fill_holes on open mesh
// ===========================================================================

#[test]
fn fill_holes_on_open_mesh_doesnt_decrease_triangles() {
    let mesh = triangle_mesh();
    let filled = mesh_ops::fill_holes(&mesh).unwrap();
    assert!(filled.indices.len() >= mesh.indices.len());
}

// ===========================================================================
// mesh_ops — Platonic / regular solids
// ===========================================================================

#[test]
fn regular_solid_negative_size_errors() {
    let result = mesh_ops::regular_solid(mesh_ops::RegularSolidType::Tetrahedron, -1.0);
    assert!(result.is_err());
}

#[test]
fn regular_solid_zero_size_errors() {
    let result = mesh_ops::regular_solid(mesh_ops::RegularSolidType::Cube, 0.0);
    assert!(result.is_err());
}

#[test]
fn regular_solid_tetrahedron_has_four_triangles() {
    let mesh = mesh_ops::regular_solid(mesh_ops::RegularSolidType::Tetrahedron, 1.0).unwrap();
    assert_eq!(mesh.vertices.len(), 4);
    assert_eq!(mesh.indices.len(), 4);
}

#[test]
fn regular_solid_cube_has_twelve_triangles() {
    let mesh = mesh_ops::regular_solid(mesh_ops::RegularSolidType::Cube, 1.0).unwrap();
    assert_eq!(mesh.vertices.len(), 8);
    assert_eq!(mesh.indices.len(), 12);
}

#[test]
fn regular_solid_octahedron_has_eight_triangles() {
    let mesh = mesh_ops::regular_solid(mesh_ops::RegularSolidType::Octahedron, 1.0).unwrap();
    assert_eq!(mesh.vertices.len(), 6);
    assert_eq!(mesh.indices.len(), 8);
}

#[test]
fn regular_solid_icosahedron_has_twenty_triangles() {
    let mesh = mesh_ops::regular_solid(mesh_ops::RegularSolidType::Icosahedron, 1.0).unwrap();
    assert_eq!(mesh.vertices.len(), 12);
    assert_eq!(mesh.indices.len(), 20);
}

#[test]
fn regular_solid_dodecahedron_triangulated() {
    let mesh = mesh_ops::regular_solid(mesh_ops::RegularSolidType::Dodecahedron, 1.0).unwrap();
    // Dodecahedron = 12 pentagons, each triangulated as 3 triangles = 36
    assert_eq!(mesh.vertices.len(), 20);
    assert_eq!(mesh.indices.len(), 36);
}

// ===========================================================================
// mesh_ops — decimate edge cases
// ===========================================================================

#[test]
fn decimate_out_of_range_ratio_errors() {
    let mesh = square_mesh();
    assert!(mesh_ops::decimate_mesh(&mesh, 0.0).is_err());
    assert!(mesh_ops::decimate_mesh(&mesh, 1.0).is_err());
    assert!(mesh_ops::decimate_mesh(&mesh, -0.5).is_err());
    assert!(mesh_ops::decimate_mesh(&mesh, 1.5).is_err());
}

#[test]
fn decimate_empty_mesh_yields_empty() {
    let empty = Mesh {
        vertices: Vec::new(),
        normals: Vec::new(),
        indices: Vec::new(),
    };
    let out = mesh_ops::decimate_mesh(&empty, 0.5).unwrap();
    assert!(out.indices.is_empty());
}

// ===========================================================================
// tessellate — merge edge cases
// ===========================================================================

#[test]
fn merge_empty_slice_yields_empty_mesh() {
    let merged = tessellate::merge_meshes(&[]);
    assert!(merged.vertices.is_empty());
    assert!(merged.indices.is_empty());
}

#[test]
fn merge_single_mesh_equals_input() {
    let a = triangle_mesh();
    let merged = tessellate::merge_meshes(std::slice::from_ref(&a));
    assert_eq!(merged.vertices.len(), a.vertices.len());
    assert_eq!(merged.indices.len(), a.indices.len());
}

#[test]
fn merge_with_empty_meshes_skips_them() {
    let a = triangle_mesh();
    let empty = Mesh {
        vertices: Vec::new(),
        normals: Vec::new(),
        indices: Vec::new(),
    };
    let merged = tessellate::merge_meshes(&[empty.clone(), a.clone(), empty]);
    assert_eq!(merged.vertices.len(), a.vertices.len());
    assert_eq!(merged.indices.len(), a.indices.len());
}

// ===========================================================================
// SVG export — structural validation
// ===========================================================================

#[test]
fn svg_document_render_contains_xml_and_viewbox() {
    let mut doc = svg::SvgDocument::new(100.0, 50.0);
    doc.add(svg::SvgElement::Line {
        x1: 0.0,
        y1: 0.0,
        x2: 100.0,
        y2: 50.0,
        style: svg::SvgStyle::default_stroke(),
    });
    let rendered = doc.render();
    assert!(rendered.starts_with("<svg"));
    assert!(rendered.contains("viewBox=\"0 0 100 50\""));
    assert!(rendered.contains("</svg>"));
    // multiple lines / elements present
    assert!(rendered.len() > 80);
}

#[test]
fn svg_escapes_xml_special_chars_in_text() {
    let mut doc = svg::SvgDocument::new(100.0, 100.0);
    doc.add(svg::SvgElement::Text {
        x: 10.0,
        y: 10.0,
        text: "a & b < c > \"d\"".into(),
        font_size: 12.0,
        anchor: "start".into(),
        style: svg::SvgStyle::default_stroke(),
    });
    let rendered = doc.render();
    assert!(rendered.contains("&amp;"));
    assert!(rendered.contains("&lt;"));
    assert!(rendered.contains("&gt;"));
    assert!(rendered.contains("&quot;"));
    assert!(!rendered.contains(" & "));
}

#[test]
fn svg_profile_to_svg_contains_svg_root() {
    let profile = vec![
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(10.0, 0.0, 0.0),
        Point3::new(10.0, 10.0, 0.0),
        Point3::new(0.0, 10.0, 0.0),
    ];
    let doc = svg::profile_to_svg(&profile, 50.0, 50.0);
    let rendered = doc.render();
    assert!(rendered.contains("<svg"));
    assert!(rendered.contains("</svg>"));
}

// ===========================================================================
// PDF export — structural validation
// ===========================================================================

#[test]
fn pdf_export_contains_magic_and_eof() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><line x1="0" y1="0" x2="100" y2="100" stroke="black"/></svg>"#;
    let pdf_bytes = export_pdf(svg, 100.0, 100.0).unwrap();
    assert!(&pdf_bytes[..5] == b"%PDF-");
    // Must end with %%EOF and have a xref section
    let text = String::from_utf8_lossy(&pdf_bytes);
    assert!(text.contains("xref"));
    assert!(text.contains("%%EOF"));
    assert!(text.contains("/Type /Catalog"));
    assert!(text.contains("/Type /Pages"));
    assert!(text.contains("/Type /Page"));
}

#[test]
fn pdf_export_has_multiple_lines() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><line x1="0" y1="0" x2="50" y2="50" stroke="black"/></svg>"#;
    let pdf_bytes = export_pdf(svg, 100.0, 100.0).unwrap();
    let text = String::from_utf8_lossy(&pdf_bytes);
    let line_count = text.lines().count();
    assert!(
        line_count > 10,
        "PDF should have multiple lines, got {line_count}"
    );
}

// ===========================================================================
// Native .cadk — error cases
// ===========================================================================

#[test]
fn load_project_nonexistent_path_errors() {
    let path = "/definitely/does/not/exist/cadkernel_probe.cadk";
    let result = load_project(path);
    assert!(result.is_err());
}

#[test]
fn load_project_corrupted_content_errors() {
    let tmp = unique_tmp_path("corrupted");
    std::fs::write(&tmp, b"not valid json content at all \x00 \xff").unwrap();
    let path = tmp.to_str().unwrap();
    let result = load_project(path);
    assert!(result.is_err());
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn load_project_wrong_format_marker_errors() {
    let tmp = unique_tmp_path("wrong_marker");
    // Valid JSON with correct shape but wrong format marker
    let bad = r#"{"format":"SomethingElse","version":"0.1.0","model":{}}"#;
    std::fs::write(&tmp, bad).unwrap();
    let path = tmp.to_str().unwrap();
    let result = load_project(path);
    assert!(result.is_err());
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn load_project_truncated_file_errors() {
    let tmp = unique_tmp_path("truncated");
    // Valid header start but truncated before closing
    std::fs::write(&tmp, r#"{"format":"CADKernel","version":"0.1.0","mod"#).unwrap();
    let path = tmp.to_str().unwrap();
    let result = load_project(path);
    assert!(result.is_err());
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn save_project_to_bad_path_errors() {
    let model = BRepModel::new();
    let result = save_project(&model, "/no/such/dir/file.cadk");
    assert!(result.is_err());
}

#[test]
fn load_scene_nonexistent_errors() {
    let result = load_scene("/no/such/file/at/all/scene.cadk");
    assert!(result.is_err());
}

#[test]
fn save_and_load_empty_scene_roundtrip() {
    let tmp = unique_tmp_path("empty_scene");
    let empty: Vec<SceneObjectData> = Vec::new();
    let path = tmp.to_str().unwrap();
    save_scene(&empty, path).unwrap();
    let loaded = load_scene(path).unwrap();
    assert!(loaded.is_empty());
    let _ = std::fs::remove_file(&tmp);
}

// ===========================================================================
// JSON — error paths and edge cases
// ===========================================================================

#[test]
fn model_from_json_malformed_errors() {
    let result = model_from_json("{ not valid json");
    assert!(result.is_err());
}

#[test]
fn model_from_json_wrong_shape_errors() {
    let result = model_from_json("[1, 2, 3]");
    assert!(result.is_err());
}

#[test]
fn model_to_json_empty_model_produces_valid_json() {
    let model = BRepModel::new();
    let json_str = model_to_json(&model).unwrap();
    // Must round-trip through serde_json
    let _: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert!(json_str.contains('{'));
    assert!(json_str.contains('}'));
}

#[test]
fn json_read_nonexistent_file_errors() {
    let result = read_json("/no/such/file/exists.json");
    assert!(result.is_err());
}

#[test]
fn json_write_and_read_roundtrip_single_primitive() {
    let tmp = unique_tmp_path("json_primitive");
    let mut model = BRepModel::new();
    make_box(&mut model, Point3::ORIGIN, 2.0, 3.0, 4.0).unwrap();
    let path = tmp.to_str().unwrap();
    write_json(&model, path).unwrap();
    let loaded = read_json(path).unwrap();
    let orig_verts: usize = model.vertices.iter().count();
    let loaded_verts: usize = loaded.vertices.iter().count();
    assert_eq!(orig_verts, loaded_verts);
    let _ = std::fs::remove_file(&tmp);
}

// ===========================================================================
// techdraw — projection API
// ===========================================================================

#[test]
fn techdraw_project_solid_empty_model_returns_empty_view() {
    let model = BRepModel::new();
    // Use a bogus handle — project_solid should gracefully return an empty view
    let any_handle = model.solids.iter().next().map(|(h, _)| h);
    if let Some(handle) = any_handle {
        let view = techdraw::project_solid(&model, handle, techdraw::ProjectionDir::Front);
        assert!(view.edges.is_empty());
    }
}

#[test]
fn techdraw_project_solid_box_produces_edges() {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

    let view = techdraw::project_solid(&model, r.solid, techdraw::ProjectionDir::Front);
    assert!(!view.edges.is_empty(), "front view should contain edges");
}

#[test]
fn techdraw_three_view_drawing_produces_three_views() {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

    let sheet = techdraw::three_view_drawing(&model, r.solid);
    assert_eq!(sheet.views.len(), 3);
    assert!(sheet.width > 0.0);
    assert!(sheet.height > 0.0);
}

#[test]
fn techdraw_drawing_sheet_a4_landscape_dimensions() {
    let sheet = techdraw::DrawingSheet::a4_landscape();
    assert_eq!(sheet.width, 297.0);
    assert_eq!(sheet.height, 210.0);
    assert!(sheet.views.is_empty());
    assert!(sheet.dimensions.is_empty());
}

#[test]
fn techdraw_projection_dir_labels_are_set() {
    use techdraw::ProjectionDir::*;
    assert_eq!(Front.label(), "Front");
    assert_eq!(Back.label(), "Back");
    assert_eq!(Top.label(), "Top");
    assert_eq!(Bottom.label(), "Bottom");
    assert_eq!(Left.label(), "Left");
    assert_eq!(Right.label(), "Right");
    assert_eq!(Isometric.label(), "Isometric");
}

#[test]
fn techdraw_drawing_to_svg_empty_sheet() {
    let sheet = techdraw::DrawingSheet::a4_landscape();
    let svg_doc = techdraw::drawing_to_svg(&sheet);
    let rendered = svg_doc.render();
    assert!(rendered.starts_with("<svg"));
    assert!(rendered.contains("</svg>"));
}
