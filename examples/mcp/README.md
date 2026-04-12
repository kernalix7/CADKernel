# CADKernel MCP Server

The Model Context Protocol (MCP) server exposes CADKernel modeling operations
as JSON-RPC 2.0 tools over stdio.  Any MCP-compatible client (Claude Desktop,
Continue, custom scripts) can drive the CAD kernel programmatically.

## Protocol

All communication uses **JSON-RPC 2.0** over standard input/output.  Each line
on stdin is a complete JSON request; each line on stdout is a complete JSON
response.

### Available Methods

| Method | Description |
|--------|-------------|
| `tools/list` | Enumerate all available tools and their input schemas |
| `tools/call` | Invoke a tool by name with arguments |

### Available Tools

| Tool | Description | Key Parameters |
|------|-------------|----------------|
| `create_primitive` | Create box, cylinder, sphere, cone, or torus | `type`, `dimensions` |
| `boolean_operation` | Union, subtract, or intersect two solids | `op`, `target_id`, `tool_id` |
| `transform` | Translate, rotate, and/or scale a solid | `id`, `translate`, `rotate`, `scale` |
| `query_model` | Get solid/face/edge/vertex counts | (none) |
| `measure` | Volume, surface area, centroid, bounding box | `id` |
| `export_model` | Export solid as STL, OBJ, STEP, or JSON string | `format`, `id` |
| `delete_solid` | Remove a solid by ID | `id` |
| `list_solids` | List all stored solids with metadata | (none) |

## Example Session

See `example_session.json` for a complete request sequence.  Each entry is a
standalone request that the server processes in order.

### Quick test with jq

```bash
# Start the MCP server (when the binary supports --mcp flag)
cargo run -- --mcp

# Or pipe requests manually (one JSON object per line):
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | cargo run -- --mcp
```

### Sending a create_primitive request

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "create_primitive",
    "arguments": {
      "type": "box",
      "dimensions": { "dx": 20, "dy": 30, "dz": 40 }
    }
  }
}
```

### Expected response

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "id": 0,
    "faces": 6,
    "edges": 24,
    "vertices": 8
  }
}
```

### Measuring a solid

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "measure",
    "arguments": { "id": 0 }
  }
}
```

### Expected response

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "volume": 24000.0,
    "surface_area": 5200.0,
    "centroid": [10.0, 15.0, 20.0],
    "bounding_box": {
      "min": [0.0, 0.0, 0.0],
      "max": [20.0, 30.0, 40.0]
    }
  }
}
```

## Programmatic Usage (Rust)

```rust
use cadkernel_io::McpServer;

let mut server = McpServer::new();
let request = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_primitive","arguments":{"type":"box","dimensions":{"dx":10,"dy":10,"dz":10}}}}"#;
let response = server.handle_request(request).unwrap();
println!("{response}");
```

## Dimension Reference

| Primitive | Required dimensions |
|-----------|-------------------|
| `box` | `dx`, `dy`, `dz` |
| `cylinder` | `radius`, `height` (optional: `segments`) |
| `sphere` | `radius` (optional: `segments`, `rings`) |
| `cone` | `base_radius`, `top_radius`, `height` (optional: `segments`) |
| `torus` | `major_radius`, `minor_radius` (optional: `major_segments`, `minor_segments`) |

## Error Handling

Errors follow JSON-RPC 2.0 conventions:

| Code | Meaning |
|------|---------|
| -32700 | Parse error (malformed JSON) |
| -32600 | Invalid request (wrong jsonrpc version) |
| -32601 | Method not found |
| -32602 | Invalid parameters |
