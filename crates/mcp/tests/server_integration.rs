//! Integration tests for cadkernel-mcp — full battery migrated from
//! crates/io/tests/io_comprehensive.rs (was: mcp_* block, lines 491-813)
//! plus one new cone-invalid test per v0.5 Gate #5 scope.
//!
//! Behavioural deltas from the old cadkernel-io McpServer:
//!   - Cone: top_radius != 0 rejected with INVALID_PARAMS.
//!   - Boolean: both operands consumed; result allocated to first free slot.
//!   - Transform/rotate: Euler degrees → sequential Command::Rotate (X,Y,Z).
//!   - Transform/scale: [sx,sy,sz] → Command::ScaleNonUniform about origin.

use cadkernel_mcp::McpServer;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn call(server: &mut McpServer, json: &str) -> serde_json::Value {
    let resp = server
        .handle_request(json)
        .expect("handle_request must not fail at the KernelError level");
    serde_json::from_str(&resp).expect("response must be valid JSON")
}

// ---------------------------------------------------------------------------
// Protocol error paths
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// tools/list
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// tools/call — missing / invalid params
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// create_primitive — success cases
// ---------------------------------------------------------------------------

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
fn mcp_create_primitive_cone_nonzero_top_radius_rejected() {
    // top_radius != 0 is rejected with INVALID_PARAMS per Gate #5 scope
    // (Command::CreateCone has no top_radius field; frustum not supported).
    let mut server = McpServer::new();
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_primitive","arguments":{"type":"cone","dimensions":{"base_radius":3.0,"top_radius":1.0,"height":5.0}}}}"#;
    let r = call(&mut server, req);
    assert!(
        !r["error"].is_null(),
        "expected INVALID_PARAMS for cone with top_radius=1.0, got: {r:?}"
    );
    assert_eq!(r["error"]["code"].as_i64(), Some(-32602));
}

#[test]
fn mcp_create_primitive_cone_zero_top_radius_succeeds() {
    // cone with top_radius == 0 (or omitted) must succeed.
    let mut server = McpServer::new();
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_primitive","arguments":{"type":"cone","dimensions":{"base_radius":3.0,"top_radius":0.0,"height":5.0}}}}"#;
    let r = call(&mut server, req);
    assert!(
        r["error"].is_null(),
        "cone with top_radius=0 should succeed, got error: {r:?}"
    );
    assert!(r["result"]["id"].as_u64().is_some());
}

#[test]
fn mcp_create_primitive_torus_succeeds() {
    let mut server = McpServer::new();
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_primitive","arguments":{"type":"torus","dimensions":{"major_radius":4.0,"minor_radius":1.0}}}}"#;
    let response = server.handle_request(req).unwrap();
    assert!(!response.contains("\"error\""));
}

// ---------------------------------------------------------------------------
// create_primitive — error cases
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// transform
// ---------------------------------------------------------------------------

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
    // Rotate dispatches sequential Command::Rotate (X, Y, Z) — test only
    // verifies no error is returned (exact geometry not asserted here).
    let mut server = McpServer::new();
    let create = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_primitive","arguments":{"type":"box","dimensions":{"dx":1.0,"dy":1.0,"dz":1.0}}}}"#;
    server.handle_request(create).unwrap();

    let rotate = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"transform","arguments":{"id":0,"rotate":[45.0,0.0,0.0]}}}"#;
    let response = server.handle_request(rotate).unwrap();
    assert!(!response.contains("\"error\""));
}

#[test]
fn mcp_transform_scale_succeeds() {
    // Scale dispatches Command::ScaleNonUniform with factors [sx,sy,sz] — all > 0.
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

// ---------------------------------------------------------------------------
// query_model
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// measure
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// export_model
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// delete_solid
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// list_solids
// ---------------------------------------------------------------------------

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
    // slot 0 cleared; slot 1 remains
    assert!(list2.contains("\"id\":1"));
    assert!(!list2.contains("\"id\":0"));
}

// ---------------------------------------------------------------------------
// boolean_operation
// ---------------------------------------------------------------------------

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

#[test]
fn mcp_boolean_union_consumes_inputs_and_returns_new_slot() {
    // Both operand slots are cleared; the result lands under a fresh slot id.
    // The new slot id is >= 0 and the result has faces/edges/vertices fields.
    let mut server = McpServer::new();
    let c1 = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_primitive","arguments":{"type":"box","dimensions":{"dx":2.0,"dy":2.0,"dz":2.0}}}}"#;
    let c2 = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"create_primitive","arguments":{"type":"box","dimensions":{"dx":1.0,"dy":1.0,"dz":1.0}}}}"#;
    server.handle_request(c1).unwrap();
    server.handle_request(c2).unwrap();

    let union_req = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"boolean_operation","arguments":{"op":"union","target_id":0,"tool_id":1}}}"#;
    let r = call(&mut server, union_req);
    assert!(
        r["error"].is_null(),
        "boolean union should succeed, got: {r:?}"
    );
    let result_id = r["result"]["id"].as_u64().expect("result must have integer id");
    // Slot 0 was freed first, so result reuses slot 0.
    assert_eq!(result_id, 0);

    // Verify the result solid is listed and slot 1 is gone.
    let list_req = r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"list_solids","arguments":{}}}"#;
    let list = call(&mut server, list_req);
    let solids = list["result"]["solids"].as_array().expect("solids array");
    assert_eq!(solids.len(), 1);
    assert_eq!(solids[0]["id"].as_u64(), Some(0));
}
