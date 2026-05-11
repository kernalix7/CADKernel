//! Smoke tests for the new cadkernel-mcp server. The full migrated
//! battery lands in task #5; this file just proves the round-trip works
//! while #2 and #3 are settling.

use cadkernel_mcp::McpServer;

fn call(server: &mut McpServer, json: &str) -> serde_json::Value {
    let resp = server.handle_request(json).expect("handle_request");
    serde_json::from_str(&resp).expect("response is valid JSON")
}

#[test]
fn tools_list_returns_eight() {
    let server = McpServer::new();
    assert_eq!(server.list_tools().len(), 8);
}

#[test]
fn create_box_then_export_stl() {
    let mut server = McpServer::new();
    let create = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_primitive","arguments":{"type":"box","dimensions":{"dx":1.0,"dy":1.0,"dz":1.0}}}}"#;
    let r = call(&mut server, create);
    assert!(r["error"].is_null(), "create errored: {r:?}");
    let export = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"export_model","arguments":{"id":0,"format":"stl"}}}"#;
    let r = call(&mut server, export);
    assert!(r["error"].is_null(), "export errored: {r:?}");
    let data = r["result"]["data"].as_str().unwrap();
    assert!(data.starts_with("solid "));
    assert!(data.contains("facet normal"));
}
