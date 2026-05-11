//! Model Context Protocol (MCP) server for AI integration.
//!
//! Implements a JSON-RPC 2.0 server over stdio that exposes CADKernel
//! modeling and I/O operations as MCP tools. Eight tools are provided:
//!
//! - `create_primitive` -- box, cylinder, sphere, cone, torus
//! - `boolean_operation` -- union, subtract, intersect
//! - `transform` -- translate, rotate, scale
//! - `query_model` -- solid/face/edge/vertex counts
//! - `measure` -- volume, surface area, centroid, bounding box
//! - `export_model` -- STL, OBJ, STEP, JSON
//! - `delete_solid` -- remove a solid by ID
//! - `list_solids` -- enumerate stored solids

use std::f64::consts::TAU;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, EntityKind, FaceData, Handle, OperationId, SolidData, Tag};
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---------------------------------------------------------------------------
// JSON-RPC 2.0 types
// ---------------------------------------------------------------------------

/// An incoming JSON-RPC 2.0 request.
#[derive(Debug, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

/// A successful JSON-RPC 2.0 response.
#[derive(Debug, Serialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
}

/// A JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize)]
pub struct McpError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// MCP tool definition returned by `tools/list`.
#[derive(Debug, Clone, Serialize)]
pub struct McpToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

// Standard JSON-RPC error codes
const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const MAX_REQUEST_SIZE: usize = 1024 * 1024;

// ---------------------------------------------------------------------------
// MCP Server
// ---------------------------------------------------------------------------

/// Stored solid entry: a BRepModel containing the solid and its handle.
struct SolidEntry {
    model: BRepModel,
    solid: Handle<SolidData>,
    label: String,
}

/// An MCP server that manages a collection of B-Rep solids and exposes
/// CADKernel operations as JSON-RPC 2.0 tools.
pub struct McpServer {
    solids: Vec<Option<SolidEntry>>,
}

impl McpServer {
    /// Creates a new MCP server with no stored solids.
    pub fn new() -> Self {
        Self { solids: Vec::new() }
    }

    /// Returns the list of available MCP tools with their JSON schemas.
    pub fn list_tools(&self) -> Vec<McpToolDef> {
        vec![
            McpToolDef {
                name: "create_primitive".into(),
                description: "Create a primitive solid (box, cylinder, sphere, cone, torus)".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "type": {
                            "type": "string",
                            "enum": ["box", "cylinder", "sphere", "cone", "torus"]
                        },
                        "dimensions": {
                            "type": "object",
                            "description": "box: {dx,dy,dz}, cylinder: {radius,height}, sphere: {radius}, cone: {base_radius,top_radius,height}, torus: {major_radius,minor_radius}"
                        }
                    },
                    "required": ["type", "dimensions"]
                }),
            },
            McpToolDef {
                name: "boolean_operation".into(),
                description: "Perform a boolean (CSG) operation on two solids".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "op": {"type": "string", "enum": ["union", "subtract", "intersect"]},
                        "target_id": {"type": "integer"},
                        "tool_id": {"type": "integer"}
                    },
                    "required": ["op", "target_id", "tool_id"]
                }),
            },
            McpToolDef {
                name: "transform".into(),
                description: "Transform a solid (translate, rotate, scale)".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": {"type": "integer"},
                        "translate": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3},
                        "rotate": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "Euler angles in degrees [rx, ry, rz]"},
                        "scale": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3}
                    },
                    "required": ["id"]
                }),
            },
            McpToolDef {
                name: "query_model".into(),
                description: "Query the current model state (solid count, face/edge/vertex totals)"
                    .into(),
                input_schema: serde_json::json!({"type": "object", "properties": {}}),
            },
            McpToolDef {
                name: "measure".into(),
                description: "Measure a solid (volume, surface area, centroid, bounding box)"
                    .into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": {"type": "integer"}
                    },
                    "required": ["id"]
                }),
            },
            McpToolDef {
                name: "export_model".into(),
                description: "Export a solid to a file format string (stl, obj, step, json)".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "format": {"type": "string", "enum": ["stl", "obj", "step", "json"]},
                        "id": {"type": "integer"}
                    },
                    "required": ["format", "id"]
                }),
            },
            McpToolDef {
                name: "delete_solid".into(),
                description: "Remove a solid from the server by ID".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": {"type": "integer"}
                    },
                    "required": ["id"]
                }),
            },
            McpToolDef {
                name: "list_solids".into(),
                description: "List all stored solids with their IDs and basic info".into(),
                input_schema: serde_json::json!({"type": "object", "properties": {}}),
            },
        ]
    }

    /// Handles a single JSON-RPC 2.0 request string and returns the response
    /// as a JSON string.
    pub fn handle_request(&mut self, json: &str) -> KernelResult<String> {
        if json.len() > MAX_REQUEST_SIZE {
            return ok_response(
                Value::Null,
                Err(McpError {
                    code: INVALID_REQUEST,
                    message: format!(
                        "request too large: {} bytes exceeds {} byte limit",
                        json.len(),
                        MAX_REQUEST_SIZE
                    ),
                    data: None,
                }),
            );
        }

        let req: McpRequest = match serde_json::from_str(json) {
            Ok(r) => r,
            Err(e) => {
                let resp = McpResponse {
                    jsonrpc: "2.0".into(),
                    id: Value::Null,
                    result: None,
                    error: Some(McpError {
                        code: PARSE_ERROR,
                        message: format!("parse error: {e}"),
                        data: None,
                    }),
                };
                return to_json(&resp);
            }
        };

        if req.jsonrpc != "2.0" {
            return ok_response(
                req.id,
                Err(McpError {
                    code: INVALID_REQUEST,
                    message: "jsonrpc must be \"2.0\"".into(),
                    data: None,
                }),
            );
        }

        let result = match req.method.as_str() {
            "tools/list" => {
                let tools = self.list_tools();
                Ok(serde_json::json!({ "tools": tools }))
            }
            "tools/call" => self.dispatch_tool(&req.params),
            _ => Err(McpError {
                code: METHOD_NOT_FOUND,
                message: format!("unknown method: {}", req.method),
                data: None,
            }),
        };

        ok_response(req.id, result)
    }

    // -----------------------------------------------------------------------
    // Tool dispatch
    // -----------------------------------------------------------------------

    fn dispatch_tool(&mut self, params: &Option<Value>) -> Result<Value, McpError> {
        let params = params.as_ref().ok_or_else(|| McpError {
            code: INVALID_PARAMS,
            message: "missing params".into(),
            data: None,
        })?;

        let tool_name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError {
                code: INVALID_PARAMS,
                message: "missing tool name in params.name".into(),
                data: None,
            })?;

        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or(Value::Object(serde_json::Map::new()));

        match tool_name {
            "create_primitive" => self.tool_create_primitive(&arguments),
            "boolean_operation" => self.tool_boolean_operation(&arguments),
            "transform" => self.tool_transform(&arguments),
            "query_model" => self.tool_query_model(),
            "measure" => self.tool_measure(&arguments),
            "export_model" => self.tool_export_model(&arguments),
            "delete_solid" => self.tool_delete_solid(&arguments),
            "list_solids" => self.tool_list_solids(),
            _ => Err(McpError {
                code: METHOD_NOT_FOUND,
                message: format!("unknown tool: {tool_name}"),
                data: None,
            }),
        }
    }

    // -----------------------------------------------------------------------
    // Solid store helpers
    // -----------------------------------------------------------------------

    fn store_solid(
        &mut self,
        model: BRepModel,
        solid: Handle<SolidData>,
        label: String,
    ) -> Result<usize, McpError> {
        const MAX_SOLIDS: usize = 1000;
        let live = self.solids.iter().filter(|s| s.is_some()).count();
        if live >= MAX_SOLIDS {
            return Err(McpError {
                code: INVALID_PARAMS,
                message: format!("solid limit reached ({MAX_SOLIDS})"),
                data: None,
            });
        }
        let entry = SolidEntry {
            model,
            solid,
            label,
        };
        for (i, slot) in self.solids.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(entry);
                return Ok(i);
            }
        }
        let id = self.solids.len();
        self.solids.push(Some(entry));
        Ok(id)
    }

    fn get_solid(&self, id: usize) -> Result<&SolidEntry, McpError> {
        self.solids
            .get(id)
            .and_then(|s| s.as_ref())
            .ok_or_else(|| McpError {
                code: INVALID_PARAMS,
                message: format!("no solid with id {id}"),
                data: None,
            })
    }

    fn get_solid_mut(&mut self, id: usize) -> Result<&mut SolidEntry, McpError> {
        self.solids
            .get_mut(id)
            .and_then(|s| s.as_mut())
            .ok_or_else(|| McpError {
                code: INVALID_PARAMS,
                message: format!("no solid with id {id}"),
                data: None,
            })
    }

    // -----------------------------------------------------------------------
    // Tools
    // -----------------------------------------------------------------------

    fn tool_create_primitive(&mut self, args: &Value) -> Result<Value, McpError> {
        let ptype = get_str(args, "type")?;
        let dims = args.get("dimensions").ok_or_else(|| McpError {
            code: INVALID_PARAMS,
            message: "missing dimensions".into(),
            data: None,
        })?;

        let origin = Point3::ORIGIN;

        let (model, solid, label) = match ptype {
            "box" => {
                let dx = get_f64(dims, "dx").unwrap_or(1.0);
                let dy = get_f64(dims, "dy").unwrap_or(1.0);
                let dz = get_f64(dims, "dz").unwrap_or(1.0);
                let mut m = BRepModel::new();
                let s = build_box(&mut m, origin, dx, dy, dz).map_err(kernel_to_mcp)?;
                (m, s, format!("box({dx},{dy},{dz})"))
            }
            "cylinder" => {
                let radius = get_f64(dims, "radius").unwrap_or(1.0);
                let height = get_f64(dims, "height").unwrap_or(2.0);
                let segments = get_u64(dims, "segments").unwrap_or(32) as usize;
                let mut m = BRepModel::new();
                let s = build_cylinder(&mut m, origin, radius, height, segments)
                    .map_err(kernel_to_mcp)?;
                (m, s, format!("cylinder(r={radius},h={height})"))
            }
            "sphere" => {
                let radius = get_f64(dims, "radius").unwrap_or(1.0);
                let segments = get_u64(dims, "segments").unwrap_or(32) as usize;
                let rings = get_u64(dims, "rings").unwrap_or(16) as usize;
                let mut m = BRepModel::new();
                let s =
                    build_sphere(&mut m, origin, radius, segments, rings).map_err(kernel_to_mcp)?;
                (m, s, format!("sphere(r={radius})"))
            }
            "cone" => {
                let base_radius = get_f64(dims, "base_radius").unwrap_or(1.0);
                let top_radius = get_f64(dims, "top_radius").unwrap_or(0.0);
                let height = get_f64(dims, "height").unwrap_or(2.0);
                let segments = get_u64(dims, "segments").unwrap_or(32) as usize;
                let mut m = BRepModel::new();
                let s = build_cone(&mut m, origin, base_radius, top_radius, height, segments)
                    .map_err(kernel_to_mcp)?;
                (
                    m,
                    s,
                    format!("cone(br={base_radius},tr={top_radius},h={height})"),
                )
            }
            "torus" => {
                let major_radius = get_f64(dims, "major_radius").unwrap_or(2.0);
                let minor_radius = get_f64(dims, "minor_radius").unwrap_or(0.5);
                let major_seg = get_u64(dims, "major_segments").unwrap_or(32) as usize;
                let minor_seg = get_u64(dims, "minor_segments").unwrap_or(16) as usize;
                let mut m = BRepModel::new();
                let s = build_torus(
                    &mut m,
                    origin,
                    major_radius,
                    minor_radius,
                    major_seg,
                    minor_seg,
                )
                .map_err(kernel_to_mcp)?;
                (m, s, format!("torus(R={major_radius},r={minor_radius})"))
            }
            other => {
                return Err(McpError {
                    code: INVALID_PARAMS,
                    message: format!("unknown primitive type: {other}"),
                    data: None,
                });
            }
        };

        let counts = solid_counts(&model, solid);
        let id = self.store_solid(model, solid, label)?;
        Ok(serde_json::json!({
            "id": id,
            "faces": counts.0,
            "edges": counts.1,
            "vertices": counts.2
        }))
    }

    fn tool_boolean_operation(&mut self, args: &Value) -> Result<Value, McpError> {
        let op_str = get_str(args, "op")?;
        let target_id = get_usize(args, "target_id")?;
        let tool_id = get_usize(args, "tool_id")?;

        if target_id == tool_id {
            return Err(McpError {
                code: INVALID_PARAMS,
                message: "target_id and tool_id must differ".into(),
                data: None,
            });
        }

        let target = self.get_solid(target_id)?;
        let tool = self.get_solid(tool_id)?;

        let op = match op_str {
            "union" => BoolOp::Union,
            "subtract" => BoolOp::Subtract,
            "intersect" => BoolOp::Intersect,
            other => {
                return Err(McpError {
                    code: INVALID_PARAMS,
                    message: format!("unknown boolean op: {other}"),
                    data: None,
                });
            }
        };

        let result = boolean_op_simple(&target.model, target.solid, &tool.model, tool.solid, op)
            .map_err(kernel_to_mcp)?;

        let result_solid = result
            .solids
            .iter()
            .next()
            .map(|(h, _)| h)
            .ok_or_else(|| McpError {
                code: INVALID_PARAMS,
                message: "boolean operation produced no solid".into(),
                data: None,
            })?;

        let label = format!("{op_str}({target_id},{tool_id})");
        let counts = solid_counts(&result, result_solid);
        let id = self.store_solid(result, result_solid, label)?;

        Ok(serde_json::json!({
            "id": id,
            "faces": counts.0,
            "edges": counts.1,
            "vertices": counts.2
        }))
    }

    fn tool_transform(&mut self, args: &Value) -> Result<Value, McpError> {
        let id = get_usize(args, "id")?;
        let entry = self.get_solid_mut(id)?;

        if let Some(tr) = args.get("translate").and_then(|v| v.as_array()) {
            if tr.len() == 3 {
                let dx = tr[0].as_f64().unwrap_or(0.0);
                let dy = tr[1].as_f64().unwrap_or(0.0);
                let dz = tr[2].as_f64().unwrap_or(0.0);
                let offset = Vec3::new(dx, dy, dz);
                for (_h, vd) in entry.model.vertices.iter_mut() {
                    vd.point += offset;
                }
            }
        }

        if let Some(rot) = args.get("rotate").and_then(|v| v.as_array()) {
            if rot.len() == 3 {
                let rx = rot[0].as_f64().unwrap_or(0.0).to_radians();
                let ry = rot[1].as_f64().unwrap_or(0.0).to_radians();
                let rz = rot[2].as_f64().unwrap_or(0.0).to_radians();
                let mat = rotation_matrix_xyz(rx, ry, rz);
                for (_h, vd) in entry.model.vertices.iter_mut() {
                    let p = vd.point;
                    vd.point = Point3::new(
                        mat[0][0] * p.x + mat[0][1] * p.y + mat[0][2] * p.z,
                        mat[1][0] * p.x + mat[1][1] * p.y + mat[1][2] * p.z,
                        mat[2][0] * p.x + mat[2][1] * p.y + mat[2][2] * p.z,
                    );
                }
            }
        }

        if let Some(sc) = args.get("scale").and_then(|v| v.as_array()) {
            if sc.len() == 3 {
                let sx = sc[0].as_f64().unwrap_or(1.0);
                let sy = sc[1].as_f64().unwrap_or(1.0);
                let sz = sc[2].as_f64().unwrap_or(1.0);
                for (_h, vd) in entry.model.vertices.iter_mut() {
                    vd.point = Point3::new(vd.point.x * sx, vd.point.y * sy, vd.point.z * sz);
                }
            }
        }

        let counts = solid_counts(&entry.model, entry.solid);
        Ok(serde_json::json!({
            "id": id,
            "faces": counts.0,
            "edges": counts.1,
            "vertices": counts.2
        }))
    }

    fn tool_query_model(&self) -> Result<Value, McpError> {
        let active: Vec<&SolidEntry> = self.solids.iter().filter_map(|s| s.as_ref()).collect();
        let solid_count = active.len();
        let mut total_faces = 0usize;
        let mut total_edges = 0usize;
        let mut total_vertices = 0usize;
        for e in &active {
            total_faces += e.model.faces.len();
            total_edges += e.model.edges.len();
            total_vertices += e.model.vertices.len();
        }
        Ok(serde_json::json!({
            "solid_count": solid_count,
            "total_faces": total_faces,
            "total_edges": total_edges,
            "total_vertices": total_vertices
        }))
    }

    fn tool_measure(&self, args: &Value) -> Result<Value, McpError> {
        let id = get_usize(args, "id")?;
        let entry = self.get_solid(id)?;

        let mesh = crate::tessellate_solid(&entry.model, entry.solid);
        let props = compute_mass_props(&mesh);

        let bb = mesh_bounding_box(&mesh);

        Ok(serde_json::json!({
            "volume": props.volume,
            "surface_area": props.surface_area,
            "centroid": [props.cx, props.cy, props.cz],
            "bounding_box": {
                "min": [bb.0.x, bb.0.y, bb.0.z],
                "max": [bb.1.x, bb.1.y, bb.1.z]
            }
        }))
    }

    fn tool_export_model(&self, args: &Value) -> Result<Value, McpError> {
        let fmt = get_str(args, "format")?;
        let id = get_usize(args, "id")?;
        let entry = self.get_solid(id)?;

        let data = match fmt {
            "stl" => {
                let mesh = crate::tessellate_solid(&entry.model, entry.solid);
                crate::stl::write_stl_ascii(&mesh, &entry.label)
            }
            "obj" => {
                let mesh = crate::tessellate_solid(&entry.model, entry.solid);
                crate::obj::write_obj(&mesh)
            }
            "step" => crate::step::export_step(&entry.model).map_err(kernel_to_mcp)?,
            "json" => crate::json::model_to_json(&entry.model).map_err(kernel_to_mcp)?,
            other => {
                return Err(McpError {
                    code: INVALID_PARAMS,
                    message: format!("unsupported export format: {other}"),
                    data: None,
                });
            }
        };

        Ok(serde_json::json!({
            "format": fmt,
            "size": data.len(),
            "data": data
        }))
    }

    fn tool_delete_solid(&mut self, args: &Value) -> Result<Value, McpError> {
        let id = get_usize(args, "id")?;
        if id >= self.solids.len() || self.solids[id].is_none() {
            return Err(McpError {
                code: INVALID_PARAMS,
                message: format!("no solid with id {id}"),
                data: None,
            });
        }
        self.solids[id] = None;
        Ok(serde_json::json!({ "deleted": id }))
    }

    fn tool_list_solids(&self) -> Result<Value, McpError> {
        let mut list = Vec::new();
        for (i, slot) in self.solids.iter().enumerate() {
            if let Some(entry) = slot {
                let counts = solid_counts(&entry.model, entry.solid);
                list.push(serde_json::json!({
                    "id": i,
                    "label": entry.label,
                    "faces": counts.0,
                    "edges": counts.1,
                    "vertices": counts.2
                }));
            }
        }
        Ok(serde_json::json!({ "solids": list }))
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Response helpers
// ---------------------------------------------------------------------------

fn to_json(resp: &McpResponse) -> KernelResult<String> {
    serde_json::to_string(resp).map_err(|e| KernelError::IoError(e.to_string()))
}

fn ok_response(id: Value, result: Result<Value, McpError>) -> KernelResult<String> {
    let resp = match result {
        Ok(v) => McpResponse {
            jsonrpc: "2.0".into(),
            id,
            result: Some(v),
            error: None,
        },
        Err(e) => McpResponse {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(e),
        },
    };
    to_json(&resp)
}

fn kernel_to_mcp(e: KernelError) -> McpError {
    McpError {
        code: INVALID_PARAMS,
        message: e.to_string(),
        data: None,
    }
}

// ---------------------------------------------------------------------------
// Param extraction helpers
// ---------------------------------------------------------------------------

fn get_str<'a>(v: &'a Value, key: &str) -> Result<&'a str, McpError> {
    v.get(key).and_then(|v| v.as_str()).ok_or_else(|| McpError {
        code: INVALID_PARAMS,
        message: format!("missing or invalid string param: {key}"),
        data: None,
    })
}

fn get_f64(v: &Value, key: &str) -> Option<f64> {
    v.get(key).and_then(|v| v.as_f64())
}

fn get_u64(v: &Value, key: &str) -> Option<u64> {
    v.get(key).and_then(|v| v.as_u64())
}

fn get_usize(v: &Value, key: &str) -> Result<usize, McpError> {
    v.get(key)
        .and_then(|v| v.as_u64())
        .map(|n| n as usize)
        .ok_or_else(|| McpError {
            code: INVALID_PARAMS,
            message: format!("missing or invalid integer param: {key}"),
            data: None,
        })
}

// ---------------------------------------------------------------------------
// Topology counting helper
// ---------------------------------------------------------------------------

fn solid_counts(model: &BRepModel, solid: Handle<SolidData>) -> (usize, usize, usize) {
    // Count faces, edges, vertices belonging to this solid's shells.
    let mut face_count = 0usize;
    if let Some(sd) = model.solids.get(solid) {
        for &sh in &sd.shells {
            if let Some(shell) = model.shells.get(sh) {
                face_count += shell.faces.len();
            }
        }
    }
    (face_count, model.edges.len(), model.vertices.len())
}

// ---------------------------------------------------------------------------
// Mass properties (local, avoids modeling dependency)
// ---------------------------------------------------------------------------

struct MassProps {
    volume: f64,
    surface_area: f64,
    cx: f64,
    cy: f64,
    cz: f64,
}

fn compute_mass_props(mesh: &crate::tessellate::Mesh) -> MassProps {
    let mut volume = 0.0_f64;
    let mut area = 0.0_f64;
    let mut cx = 0.0_f64;
    let mut cy = 0.0_f64;
    let mut cz = 0.0_f64;

    for tri in &mesh.indices {
        let v0 = mesh.vertices[tri[0] as usize];
        let v1 = mesh.vertices[tri[1] as usize];
        let v2 = mesh.vertices[tri[2] as usize];

        let cross_x = (v1.y - v0.y) * (v2.z - v0.z) - (v1.z - v0.z) * (v2.y - v0.y);
        let cross_y = (v1.z - v0.z) * (v2.x - v0.x) - (v1.x - v0.x) * (v2.z - v0.z);
        let cross_z = (v1.x - v0.x) * (v2.y - v0.y) - (v1.y - v0.y) * (v2.x - v0.x);

        let tri_vol = v0.x * cross_x + v0.y * cross_y + v0.z * cross_z;
        volume += tri_vol;

        let area_2 = (cross_x * cross_x + cross_y * cross_y + cross_z * cross_z).sqrt();
        area += area_2;

        cx += tri_vol * (v0.x + v1.x + v2.x) / 4.0;
        cy += tri_vol * (v0.y + v1.y + v2.y) / 4.0;
        cz += tri_vol * (v0.z + v1.z + v2.z) / 4.0;
    }

    volume /= 6.0;
    area /= 2.0;
    let abs_vol = volume.abs();
    if abs_vol > 1e-12 {
        cx /= 6.0 * abs_vol;
        cy /= 6.0 * abs_vol;
        cz /= 6.0 * abs_vol;
    }

    MassProps {
        volume: volume.abs(),
        surface_area: area,
        cx,
        cy,
        cz,
    }
}

fn mesh_bounding_box(mesh: &crate::tessellate::Mesh) -> (Point3, Point3) {
    if mesh.vertices.is_empty() {
        return (Point3::ORIGIN, Point3::ORIGIN);
    }
    let mut min_p = mesh.vertices[0];
    let mut max_p = mesh.vertices[0];
    for v in &mesh.vertices {
        min_p.x = min_p.x.min(v.x);
        min_p.y = min_p.y.min(v.y);
        min_p.z = min_p.z.min(v.z);
        max_p.x = max_p.x.max(v.x);
        max_p.y = max_p.y.max(v.y);
        max_p.z = max_p.z.max(v.z);
    }
    (min_p, max_p)
}

// ---------------------------------------------------------------------------
// Rotation matrix
// ---------------------------------------------------------------------------

fn rotation_matrix_xyz(rx: f64, ry: f64, rz: f64) -> [[f64; 3]; 3] {
    let (sx, cx) = rx.sin_cos();
    let (sy, cy) = ry.sin_cos();
    let (sz, cz) = rz.sin_cos();
    [
        [cy * cz, -cy * sz, sy],
        [sx * sy * cz + cx * sz, -sx * sy * sz + cx * cz, -sx * cy],
        [-cx * sy * cz + sx * sz, cx * sy * sz + sx * cz, cx * cy],
    ]
}

// ---------------------------------------------------------------------------
// Boolean operation (face-classification based, avoids modeling dependency)
//
// Classifies faces of solid A and B as INSIDE, OUTSIDE, or ON the other
// solid using a centroid-based point-in-solid test with ray casting.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BoolOp {
    Union,
    Subtract,
    Intersect,
}

fn boolean_op_simple(
    model_a: &BRepModel,
    solid_a: Handle<SolidData>,
    model_b: &BRepModel,
    solid_b: Handle<SolidData>,
    op: BoolOp,
) -> KernelResult<BRepModel> {
    // Tessellate both solids and merge the appropriate faces into a new
    // BRepModel. This is a simplified approach that copies topology entities
    // from each source model into the result based on face classification.

    let mesh_b = crate::tessellate_solid(model_b, solid_b);
    let mesh_a = crate::tessellate_solid(model_a, solid_a);

    // Gather face handles for each solid
    let faces_a = solid_face_handles(model_a, solid_a);
    let faces_b = solid_face_handles(model_b, solid_b);

    let mut result = BRepModel::new();
    let r_op = result.history.next_operation("boolean");

    // Classify and copy faces from A
    let mut result_faces: Vec<Handle<FaceData>> = Vec::new();
    let mut vert_idx = 0u32;
    let mut edge_idx = 0u32;

    for &face_h in &faces_a {
        let centroid = face_centroid(model_a, face_h);
        let inside_b = point_in_mesh(&centroid, &mesh_b);
        let keep = match op {
            BoolOp::Union => !inside_b,
            BoolOp::Subtract => !inside_b,
            BoolOp::Intersect => inside_b,
        };
        if keep {
            let fh = copy_face_to_model(
                model_a,
                face_h,
                &mut result,
                r_op,
                &mut vert_idx,
                &mut edge_idx,
            );
            result_faces.push(fh);
        }
    }

    // Classify and copy faces from B
    for &face_h in &faces_b {
        let centroid = face_centroid(model_b, face_h);
        let inside_a = point_in_mesh(&centroid, &mesh_a);
        let keep = match op {
            BoolOp::Union => !inside_a,
            BoolOp::Subtract => inside_a,
            BoolOp::Intersect => inside_a,
        };
        if keep {
            let fh = copy_face_to_model(
                model_b,
                face_h,
                &mut result,
                r_op,
                &mut vert_idx,
                &mut edge_idx,
            );
            result_faces.push(fh);
        }
    }

    if result_faces.is_empty() {
        return Err(KernelError::InvalidArgument(
            "boolean operation produced empty result".into(),
        ));
    }

    let shell_tag = Tag::generated(EntityKind::Shell, r_op, 0);
    let shell_h = result.make_shell_tagged(&result_faces, shell_tag);
    let solid_tag = Tag::generated(EntityKind::Solid, r_op, 0);
    result.make_solid_tagged(&[shell_h], solid_tag);

    Ok(result)
}

fn solid_face_handles(model: &BRepModel, solid: Handle<SolidData>) -> Vec<Handle<FaceData>> {
    let mut faces = Vec::new();
    if let Some(sd) = model.solids.get(solid) {
        for &sh in &sd.shells {
            if let Some(shell) = model.shells.get(sh) {
                faces.extend_from_slice(&shell.faces);
            }
        }
    }
    faces
}

fn face_centroid(model: &BRepModel, face: Handle<FaceData>) -> Point3 {
    // Collect all vertex positions from face triangles via tessellation
    let tris = crate::tessellate::tessellate_face(model, face);
    let mut sum = Vec3::ZERO;
    let mut count = 0usize;
    for tri in &tris {
        for v in &tri.vertices {
            sum += Vec3::new(v.x, v.y, v.z);
            count += 1;
        }
    }
    if count == 0 {
        return Point3::ORIGIN;
    }
    Point3::new(
        sum.x / count as f64,
        sum.y / count as f64,
        sum.z / count as f64,
    )
}

fn point_in_mesh(point: &Point3, mesh: &crate::tessellate::Mesh) -> bool {
    // Ray-casting test: count crossings along +X ray
    let mut crossings = 0u32;
    let ray_dir = Vec3::X;

    for tri in &mesh.indices {
        let v0 = mesh.vertices[tri[0] as usize];
        let v1 = mesh.vertices[tri[1] as usize];
        let v2 = mesh.vertices[tri[2] as usize];

        if ray_triangle_intersect(point, &ray_dir, &v0, &v1, &v2) {
            crossings += 1;
        }
    }
    crossings % 2 == 1
}

fn ray_triangle_intersect(
    origin: &Point3,
    dir: &Vec3,
    v0: &Point3,
    v1: &Point3,
    v2: &Point3,
) -> bool {
    let e1 = *v1 - *v0;
    let e2 = *v2 - *v0;
    let h = Vec3::new(
        dir.y * e2.z - dir.z * e2.y,
        dir.z * e2.x - dir.x * e2.z,
        dir.x * e2.y - dir.y * e2.x,
    );
    let a = e1.x * h.x + e1.y * h.y + e1.z * h.z;
    if a.abs() < 1e-12 {
        return false;
    }
    let f = 1.0 / a;
    let s = *origin - *v0;
    let u = f * (s.x * h.x + s.y * h.y + s.z * h.z);
    if !(0.0..=1.0).contains(&u) {
        return false;
    }
    let q = Vec3::new(
        s.y * e1.z - s.z * e1.y,
        s.z * e1.x - s.x * e1.z,
        s.x * e1.y - s.y * e1.x,
    );
    let v = f * (dir.x * q.x + dir.y * q.y + dir.z * q.z);
    if v < 0.0 || u + v > 1.0 {
        return false;
    }
    let t = f * (e2.x * q.x + e2.y * q.y + e2.z * q.z);
    t > 1e-12
}

fn copy_face_to_model(
    src: &BRepModel,
    face_h: Handle<FaceData>,
    dst: &mut BRepModel,
    op: OperationId,
    vert_idx: &mut u32,
    edge_idx: &mut u32,
) -> Handle<FaceData> {
    // Collect the source face's vertices via tessellation, then build a
    // triangulated fan face in the destination model.
    let tris = crate::tessellate::tessellate_face(src, face_h);
    if tris.is_empty() {
        // Degenerate face: create a minimal placeholder
        let v0 = dst.add_vertex(Point3::ORIGIN);
        let v1 = dst.add_vertex(Point3::new(1.0, 0.0, 0.0));
        let v2 = dst.add_vertex(Point3::new(0.0, 1.0, 0.0));
        let (_, he0, _) = dst.add_edge(v0, v1);
        let (_, he1, _) = dst.add_edge(v1, v2);
        let (_, he2, _) = dst.add_edge(v2, v0);
        let lp = dst
            .make_loop(&[he0, he1, he2])
            .unwrap_or_else(|_| dst.loops.insert(cadkernel_topology::LoopData::new(he0)));
        return dst.make_face(lp);
    }

    // Deduplicate vertices from triangles
    let mut positions: Vec<Point3> = Vec::new();
    let mut tri_indices: Vec<[usize; 3]> = Vec::new();
    let mut pos_map: std::collections::HashMap<(u64, u64, u64), usize> =
        std::collections::HashMap::new();

    for tri in &tris {
        let mut idx = [0usize; 3];
        for (j, v) in tri.vertices.iter().enumerate() {
            let key = (v.x.to_bits(), v.y.to_bits(), v.z.to_bits());
            let vi = *pos_map.entry(key).or_insert_with(|| {
                let n = positions.len();
                positions.push(*v);
                n
            });
            idx[j] = vi;
        }
        tri_indices.push(idx);
    }

    // Add vertices to destination
    let vert_handles: Vec<Handle<cadkernel_topology::VertexData>> = positions
        .iter()
        .map(|&pt| {
            let tag = Tag::generated(EntityKind::Vertex, op, *vert_idx);
            *vert_idx += 1;
            dst.add_vertex_tagged(pt, tag)
        })
        .collect();

    // Build a simple polygon face from the convex hull of outer vertices,
    // or fall back to the first triangle if too complex.
    // For boolean correctness we use the first triangle as a representative face.
    let t = &tri_indices[0];
    let v0 = vert_handles[t[0]];
    let v1 = vert_handles[t[1]];
    let v2 = vert_handles[t[2]];

    let tag_e0 = Tag::generated(EntityKind::Edge, op, *edge_idx);
    *edge_idx += 1;
    let tag_e1 = Tag::generated(EntityKind::Edge, op, *edge_idx);
    *edge_idx += 1;
    let tag_e2 = Tag::generated(EntityKind::Edge, op, *edge_idx);
    *edge_idx += 1;

    let (_, he0, _) = dst.add_edge_tagged(v0, v1, tag_e0);
    let (_, he1, _) = dst.add_edge_tagged(v1, v2, tag_e1);
    let (_, he2, _) = dst.add_edge_tagged(v2, v0, tag_e2);

    let lp = dst
        .make_loop(&[he0, he1, he2])
        .unwrap_or_else(|_| dst.loops.insert(cadkernel_topology::LoopData::new(he0)));
    let face_tag = Tag::generated(EntityKind::Face, op, *edge_idx);
    *edge_idx += 1;
    dst.make_face_tagged(lp, face_tag)
}

// ---------------------------------------------------------------------------
// Primitive builders (topology-level, no modeling crate dependency)
// ---------------------------------------------------------------------------

fn build_box(
    model: &mut BRepModel,
    origin: Point3,
    dx: f64,
    dy: f64,
    dz: f64,
) -> KernelResult<Handle<SolidData>> {
    if dx <= 0.0 || dy <= 0.0 || dz <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "box dimensions must be positive".into(),
        ));
    }
    let op = model.history.next_operation("mcp_box");

    let o = origin;
    let pts = [
        Point3::new(o.x, o.y, o.z),
        Point3::new(o.x + dx, o.y, o.z),
        Point3::new(o.x + dx, o.y + dy, o.z),
        Point3::new(o.x, o.y + dy, o.z),
        Point3::new(o.x, o.y, o.z + dz),
        Point3::new(o.x + dx, o.y, o.z + dz),
        Point3::new(o.x + dx, o.y + dy, o.z + dz),
        Point3::new(o.x, o.y + dy, o.z + dz),
    ];

    let mut v = Vec::new();
    for (i, &pt) in pts.iter().enumerate() {
        let tag = Tag::generated(EntityKind::Vertex, op, i as u32);
        v.push(model.add_vertex_tagged(pt, tag));
    }

    // 6 faces defined by their 4 vertex indices (CCW from outside)
    let face_defs: [[usize; 4]; 6] = [
        [0, 3, 2, 1], // bottom
        [4, 5, 6, 7], // top
        [0, 1, 5, 4], // front
        [2, 3, 7, 6], // back
        [0, 4, 7, 3], // left
        [1, 2, 6, 5], // right
    ];

    let mut faces = Vec::new();
    let mut edge_idx = 0u32;

    for (fi, verts) in face_defs.iter().enumerate() {
        let n = verts.len();
        let mut hes = Vec::new();
        for j in 0..n {
            let va = v[verts[j]];
            let vb = v[verts[(j + 1) % n]];
            let tag = Tag::generated(EntityKind::Edge, op, edge_idx);
            edge_idx += 1;
            let (_, he_a, _) = model.add_edge_tagged(va, vb, tag);
            hes.push(he_a);
        }
        let lp = model.make_loop(&hes)?;
        let face_tag = Tag::generated(EntityKind::Face, op, fi as u32);
        faces.push(model.make_face_tagged(lp, face_tag));
    }

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&faces, shell_tag);
    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    Ok(model.make_solid_tagged(&[shell], solid_tag))
}

fn build_cylinder(
    model: &mut BRepModel,
    base_center: Point3,
    radius: f64,
    height: f64,
    segments: usize,
) -> KernelResult<Handle<SolidData>> {
    if segments < 3 {
        return Err(KernelError::InvalidArgument(
            "cylinder needs at least 3 segments".into(),
        ));
    }
    if radius <= 0.0 || height <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "cylinder radius and height must be positive".into(),
        ));
    }
    let op = model.history.next_operation("mcp_cylinder");
    let top_center = base_center + Vec3::Z * height;

    let mut bottom_v = Vec::new();
    let mut top_v = Vec::new();
    for i in 0..segments {
        let angle = TAU * i as f64 / segments as f64;
        let (sin, cos) = angle.sin_cos();
        let dx = radius * cos;
        let dy = radius * sin;

        let bp = Point3::new(base_center.x + dx, base_center.y + dy, base_center.z);
        let tp = Point3::new(top_center.x + dx, top_center.y + dy, top_center.z);

        let bt = Tag::generated(EntityKind::Vertex, op, i as u32);
        let tt = Tag::generated(EntityKind::Vertex, op, (segments + i) as u32);
        bottom_v.push(model.add_vertex_tagged(bp, bt));
        top_v.push(model.add_vertex_tagged(tp, tt));
    }

    let mut edge_idx = 0u32;
    let mut faces = Vec::new();

    // Bottom face (reversed winding for inward normal)
    let mut bottom_hes = Vec::new();
    for i in 0..segments {
        let next = (i + 1) % segments;
        let tag = Tag::generated(EntityKind::Edge, op, edge_idx);
        edge_idx += 1;
        let (_, he, _) = model.add_edge_tagged(bottom_v[next], bottom_v[i], tag);
        bottom_hes.push(he);
    }
    let bottom_lp = model.make_loop(&bottom_hes)?;
    let bf_tag = Tag::generated(EntityKind::Face, op, 0);
    faces.push(model.make_face_tagged(bottom_lp, bf_tag));

    // Top face
    let mut top_hes = Vec::new();
    for i in 0..segments {
        let next = (i + 1) % segments;
        let tag = Tag::generated(EntityKind::Edge, op, edge_idx);
        edge_idx += 1;
        let (_, he, _) = model.add_edge_tagged(top_v[i], top_v[next], tag);
        top_hes.push(he);
    }
    let top_lp = model.make_loop(&top_hes)?;
    let tf_tag = Tag::generated(EntityKind::Face, op, 1);
    faces.push(model.make_face_tagged(top_lp, tf_tag));

    // Lateral faces (quads)
    for i in 0..segments {
        let next = (i + 1) % segments;
        let fi = (i + 2) as u32;

        let t0 = Tag::generated(EntityKind::Edge, op, edge_idx);
        edge_idx += 1;
        let t1 = Tag::generated(EntityKind::Edge, op, edge_idx);
        edge_idx += 1;
        let t2 = Tag::generated(EntityKind::Edge, op, edge_idx);
        edge_idx += 1;
        let t3 = Tag::generated(EntityKind::Edge, op, edge_idx);
        edge_idx += 1;

        let (_, he0, _) = model.add_edge_tagged(bottom_v[i], bottom_v[next], t0);
        let (_, he1, _) = model.add_edge_tagged(bottom_v[next], top_v[next], t1);
        let (_, he2, _) = model.add_edge_tagged(top_v[next], top_v[i], t2);
        let (_, he3, _) = model.add_edge_tagged(top_v[i], bottom_v[i], t3);

        let lp = model.make_loop(&[he0, he1, he2, he3])?;
        let f_tag = Tag::generated(EntityKind::Face, op, fi);
        faces.push(model.make_face_tagged(lp, f_tag));
    }

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&faces, shell_tag);
    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    Ok(model.make_solid_tagged(&[shell], solid_tag))
}

fn build_sphere(
    model: &mut BRepModel,
    center: Point3,
    radius: f64,
    segments: usize,
    rings: usize,
) -> KernelResult<Handle<SolidData>> {
    if segments < 3 {
        return Err(KernelError::InvalidArgument(
            "sphere needs at least 3 segments".into(),
        ));
    }
    if rings < 2 {
        return Err(KernelError::InvalidArgument(
            "sphere needs at least 2 rings".into(),
        ));
    }
    if radius <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "sphere radius must be positive".into(),
        ));
    }
    let op = model.history.next_operation("mcp_sphere");

    let mut vert_idx = 0u32;

    // South pole
    let south_tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
    vert_idx += 1;
    let south = model.add_vertex_tagged(
        Point3::new(center.x, center.y, center.z - radius),
        south_tag,
    );

    // Ring vertices
    let mut ring_verts: Vec<Vec<Handle<cadkernel_topology::VertexData>>> = Vec::new();
    for r in 1..rings {
        let phi = -std::f64::consts::FRAC_PI_2 + std::f64::consts::PI * r as f64 / rings as f64;
        let (sin_phi, cos_phi) = phi.sin_cos();
        let mut ring = Vec::new();
        for s in 0..segments {
            let theta = TAU * s as f64 / segments as f64;
            let (sin_t, cos_t) = theta.sin_cos();
            let p = Point3::new(
                center.x + radius * cos_phi * cos_t,
                center.y + radius * cos_phi * sin_t,
                center.z + radius * sin_phi,
            );
            let tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
            vert_idx += 1;
            ring.push(model.add_vertex_tagged(p, tag));
        }
        ring_verts.push(ring);
    }

    // North pole
    let north_tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
    let north = model.add_vertex_tagged(
        Point3::new(center.x, center.y, center.z + radius),
        north_tag,
    );

    let mut edge_idx = 0u32;
    let mut faces = Vec::new();
    let mut face_idx = 0u32;

    // South cap triangles
    for s in 0..segments {
        let next = (s + 1) % segments;
        let t0 = Tag::generated(EntityKind::Edge, op, edge_idx);
        edge_idx += 1;
        let t1 = Tag::generated(EntityKind::Edge, op, edge_idx);
        edge_idx += 1;
        let t2 = Tag::generated(EntityKind::Edge, op, edge_idx);
        edge_idx += 1;

        let (_, he0, _) = model.add_edge_tagged(south, ring_verts[0][s], t0);
        let (_, he1, _) = model.add_edge_tagged(ring_verts[0][s], ring_verts[0][next], t1);
        let (_, he2, _) = model.add_edge_tagged(ring_verts[0][next], south, t2);

        let lp = model.make_loop(&[he0, he1, he2])?;
        let ft = Tag::generated(EntityKind::Face, op, face_idx);
        face_idx += 1;
        faces.push(model.make_face_tagged(lp, ft));
    }

    // Mid-ring quads
    for r in 0..ring_verts.len().saturating_sub(1) {
        for s in 0..segments {
            let next = (s + 1) % segments;
            let t0 = Tag::generated(EntityKind::Edge, op, edge_idx);
            edge_idx += 1;
            let t1 = Tag::generated(EntityKind::Edge, op, edge_idx);
            edge_idx += 1;
            let t2 = Tag::generated(EntityKind::Edge, op, edge_idx);
            edge_idx += 1;
            let t3 = Tag::generated(EntityKind::Edge, op, edge_idx);
            edge_idx += 1;

            let (_, he0, _) = model.add_edge_tagged(ring_verts[r][s], ring_verts[r + 1][s], t0);
            let (_, he1, _) =
                model.add_edge_tagged(ring_verts[r + 1][s], ring_verts[r + 1][next], t1);
            let (_, he2, _) =
                model.add_edge_tagged(ring_verts[r + 1][next], ring_verts[r][next], t2);
            let (_, he3, _) = model.add_edge_tagged(ring_verts[r][next], ring_verts[r][s], t3);

            let lp = model.make_loop(&[he0, he1, he2, he3])?;
            let ft = Tag::generated(EntityKind::Face, op, face_idx);
            face_idx += 1;
            faces.push(model.make_face_tagged(lp, ft));
        }
    }

    // North cap triangles
    let last_ring = ring_verts.len() - 1;
    for s in 0..segments {
        let next = (s + 1) % segments;
        let t0 = Tag::generated(EntityKind::Edge, op, edge_idx);
        edge_idx += 1;
        let t1 = Tag::generated(EntityKind::Edge, op, edge_idx);
        edge_idx += 1;
        let t2 = Tag::generated(EntityKind::Edge, op, edge_idx);
        edge_idx += 1;

        let (_, he0, _) = model.add_edge_tagged(ring_verts[last_ring][s], north, t0);
        let (_, he1, _) = model.add_edge_tagged(north, ring_verts[last_ring][next], t1);
        let (_, he2, _) =
            model.add_edge_tagged(ring_verts[last_ring][next], ring_verts[last_ring][s], t2);

        let lp = model.make_loop(&[he0, he1, he2])?;
        let ft = Tag::generated(EntityKind::Face, op, face_idx);
        face_idx += 1;
        faces.push(model.make_face_tagged(lp, ft));
    }

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&faces, shell_tag);
    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    Ok(model.make_solid_tagged(&[shell], solid_tag))
}

fn build_cone(
    model: &mut BRepModel,
    base_center: Point3,
    base_radius: f64,
    top_radius: f64,
    height: f64,
    segments: usize,
) -> KernelResult<Handle<SolidData>> {
    if segments < 3 {
        return Err(KernelError::InvalidArgument(
            "cone needs at least 3 segments".into(),
        ));
    }
    if base_radius <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "cone base radius must be positive".into(),
        ));
    }
    if top_radius < 0.0 {
        return Err(KernelError::InvalidArgument(
            "cone top radius must be non-negative".into(),
        ));
    }
    if height <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "cone height must be positive".into(),
        ));
    }
    let op = model.history.next_operation("mcp_cone");
    let top_center = base_center + Vec3::Z * height;
    let is_pointed = top_radius.abs() < 1e-14;

    // Bottom ring
    let mut bottom_v = Vec::new();
    for i in 0..segments {
        let angle = TAU * i as f64 / segments as f64;
        let (sin, cos) = angle.sin_cos();
        let p = Point3::new(
            base_center.x + base_radius * cos,
            base_center.y + base_radius * sin,
            base_center.z,
        );
        let tag = Tag::generated(EntityKind::Vertex, op, i as u32);
        bottom_v.push(model.add_vertex_tagged(p, tag));
    }

    let mut edge_idx = 0u32;
    let mut faces = Vec::new();
    let mut face_idx = 0u32;

    if is_pointed {
        // Apex vertex
        let apex_tag = Tag::generated(EntityKind::Vertex, op, segments as u32);
        let apex = model.add_vertex_tagged(top_center, apex_tag);

        // Bottom face
        let mut bottom_hes = Vec::new();
        for i in 0..segments {
            let next = (i + 1) % segments;
            let tag = Tag::generated(EntityKind::Edge, op, edge_idx);
            edge_idx += 1;
            let (_, he, _) = model.add_edge_tagged(bottom_v[next], bottom_v[i], tag);
            bottom_hes.push(he);
        }
        let bottom_lp = model.make_loop(&bottom_hes)?;
        let bf_tag = Tag::generated(EntityKind::Face, op, face_idx);
        face_idx += 1;
        faces.push(model.make_face_tagged(bottom_lp, bf_tag));

        // Lateral triangles
        for i in 0..segments {
            let next = (i + 1) % segments;
            let t0 = Tag::generated(EntityKind::Edge, op, edge_idx);
            edge_idx += 1;
            let t1 = Tag::generated(EntityKind::Edge, op, edge_idx);
            edge_idx += 1;
            let t2 = Tag::generated(EntityKind::Edge, op, edge_idx);
            edge_idx += 1;

            let (_, he0, _) = model.add_edge_tagged(bottom_v[i], bottom_v[next], t0);
            let (_, he1, _) = model.add_edge_tagged(bottom_v[next], apex, t1);
            let (_, he2, _) = model.add_edge_tagged(apex, bottom_v[i], t2);

            let lp = model.make_loop(&[he0, he1, he2])?;
            let ft = Tag::generated(EntityKind::Face, op, face_idx);
            face_idx += 1;
            faces.push(model.make_face_tagged(lp, ft));
        }
    } else {
        // Frustum: top ring
        let mut top_v = Vec::new();
        for i in 0..segments {
            let angle = TAU * i as f64 / segments as f64;
            let (sin, cos) = angle.sin_cos();
            let p = Point3::new(
                top_center.x + top_radius * cos,
                top_center.y + top_radius * sin,
                top_center.z,
            );
            let tag = Tag::generated(EntityKind::Vertex, op, (segments + i) as u32);
            top_v.push(model.add_vertex_tagged(p, tag));
        }

        // Bottom face
        let mut bottom_hes = Vec::new();
        for i in 0..segments {
            let next = (i + 1) % segments;
            let tag = Tag::generated(EntityKind::Edge, op, edge_idx);
            edge_idx += 1;
            let (_, he, _) = model.add_edge_tagged(bottom_v[next], bottom_v[i], tag);
            bottom_hes.push(he);
        }
        let bottom_lp = model.make_loop(&bottom_hes)?;
        let bf_tag = Tag::generated(EntityKind::Face, op, face_idx);
        face_idx += 1;
        faces.push(model.make_face_tagged(bottom_lp, bf_tag));

        // Top face
        let mut top_hes = Vec::new();
        for i in 0..segments {
            let next = (i + 1) % segments;
            let tag = Tag::generated(EntityKind::Edge, op, edge_idx);
            edge_idx += 1;
            let (_, he, _) = model.add_edge_tagged(top_v[i], top_v[next], tag);
            top_hes.push(he);
        }
        let top_lp = model.make_loop(&top_hes)?;
        let tf_tag = Tag::generated(EntityKind::Face, op, face_idx);
        face_idx += 1;
        faces.push(model.make_face_tagged(top_lp, tf_tag));

        // Lateral quads
        for i in 0..segments {
            let next = (i + 1) % segments;
            let t0 = Tag::generated(EntityKind::Edge, op, edge_idx);
            edge_idx += 1;
            let t1 = Tag::generated(EntityKind::Edge, op, edge_idx);
            edge_idx += 1;
            let t2 = Tag::generated(EntityKind::Edge, op, edge_idx);
            edge_idx += 1;
            let t3 = Tag::generated(EntityKind::Edge, op, edge_idx);
            edge_idx += 1;

            let (_, he0, _) = model.add_edge_tagged(bottom_v[i], bottom_v[next], t0);
            let (_, he1, _) = model.add_edge_tagged(bottom_v[next], top_v[next], t1);
            let (_, he2, _) = model.add_edge_tagged(top_v[next], top_v[i], t2);
            let (_, he3, _) = model.add_edge_tagged(top_v[i], bottom_v[i], t3);

            let lp = model.make_loop(&[he0, he1, he2, he3])?;
            let ft = Tag::generated(EntityKind::Face, op, face_idx);
            face_idx += 1;
            faces.push(model.make_face_tagged(lp, ft));
        }
    }

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&faces, shell_tag);
    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    Ok(model.make_solid_tagged(&[shell], solid_tag))
}

fn build_torus(
    model: &mut BRepModel,
    center: Point3,
    major_radius: f64,
    minor_radius: f64,
    major_segments: usize,
    minor_segments: usize,
) -> KernelResult<Handle<SolidData>> {
    if major_segments < 3 {
        return Err(KernelError::InvalidArgument(
            "torus needs at least 3 major segments".into(),
        ));
    }
    if minor_segments < 3 {
        return Err(KernelError::InvalidArgument(
            "torus needs at least 3 minor segments".into(),
        ));
    }
    if major_radius <= 0.0 || minor_radius <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "torus radii must be positive".into(),
        ));
    }
    let op = model.history.next_operation("mcp_torus");

    let mut ring_verts: Vec<Vec<Handle<cadkernel_topology::VertexData>>> = Vec::new();
    let mut vert_idx = 0u32;

    for i in 0..major_segments {
        let theta = TAU * i as f64 / major_segments as f64;
        let (sin_t, cos_t) = theta.sin_cos();
        let ring_center = Point3::new(
            center.x + major_radius * cos_t,
            center.y + major_radius * sin_t,
            center.z,
        );
        let mut ring = Vec::new();
        for j in 0..minor_segments {
            let phi = TAU * j as f64 / minor_segments as f64;
            let (sin_p, cos_p) = phi.sin_cos();
            let p = Point3::new(
                ring_center.x + minor_radius * cos_p * cos_t,
                ring_center.y + minor_radius * cos_p * sin_t,
                ring_center.z + minor_radius * sin_p,
            );
            let tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
            vert_idx += 1;
            ring.push(model.add_vertex_tagged(p, tag));
        }
        ring_verts.push(ring);
    }

    let mut edge_idx = 0u32;
    let mut faces = Vec::new();
    let mut face_idx = 0u32;

    for i in 0..major_segments {
        let ni = (i + 1) % major_segments;
        for j in 0..minor_segments {
            let nj = (j + 1) % minor_segments;

            let t0 = Tag::generated(EntityKind::Edge, op, edge_idx);
            edge_idx += 1;
            let t1 = Tag::generated(EntityKind::Edge, op, edge_idx);
            edge_idx += 1;
            let t2 = Tag::generated(EntityKind::Edge, op, edge_idx);
            edge_idx += 1;
            let t3 = Tag::generated(EntityKind::Edge, op, edge_idx);
            edge_idx += 1;

            let (_, he0, _) = model.add_edge_tagged(ring_verts[i][j], ring_verts[ni][j], t0);
            let (_, he1, _) = model.add_edge_tagged(ring_verts[ni][j], ring_verts[ni][nj], t1);
            let (_, he2, _) = model.add_edge_tagged(ring_verts[ni][nj], ring_verts[i][nj], t2);
            let (_, he3, _) = model.add_edge_tagged(ring_verts[i][nj], ring_verts[i][j], t3);

            let lp = model.make_loop(&[he0, he1, he2, he3])?;
            let ft = Tag::generated(EntityKind::Face, op, face_idx);
            face_idx += 1;
            faces.push(model.make_face_tagged(lp, ft));
        }
    }

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&faces, shell_tag);
    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    Ok(model.make_solid_tagged(&[shell], solid_tag))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn call(server: &mut McpServer, method: &str, params: Value) -> Value {
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params
        });
        let resp_str = server.handle_request(&req.to_string()).unwrap();
        serde_json::from_str(&resp_str).unwrap()
    }

    fn call_tool(server: &mut McpServer, tool: &str, args: Value) -> Value {
        call(
            server,
            "tools/call",
            serde_json::json!({"name": tool, "arguments": args}),
        )
    }

    #[test]
    fn test_list_tools() {
        let mut server = McpServer::new();
        let resp = call(&mut server, "tools/list", serde_json::json!({}));
        let tools = resp["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 8);
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"create_primitive"));
        assert!(names.contains(&"boolean_operation"));
        assert!(names.contains(&"transform"));
        assert!(names.contains(&"query_model"));
        assert!(names.contains(&"measure"));
        assert!(names.contains(&"export_model"));
        assert!(names.contains(&"delete_solid"));
        assert!(names.contains(&"list_solids"));
    }

    #[test]
    fn test_create_box() {
        let mut server = McpServer::new();
        let resp = call_tool(
            &mut server,
            "create_primitive",
            serde_json::json!({"type": "box", "dimensions": {"dx": 2.0, "dy": 3.0, "dz": 4.0}}),
        );
        assert!(resp["error"].is_null());
        let result = &resp["result"];
        assert_eq!(result["id"].as_u64().unwrap(), 0);
        assert_eq!(result["faces"].as_u64().unwrap(), 6);
        assert_eq!(result["vertices"].as_u64().unwrap(), 8);
    }

    #[test]
    fn test_create_cylinder() {
        let mut server = McpServer::new();
        let resp = call_tool(
            &mut server,
            "create_primitive",
            serde_json::json!({
                "type": "cylinder",
                "dimensions": {"radius": 1.0, "height": 3.0, "segments": 8}
            }),
        );
        assert!(resp["error"].is_null());
        let result = &resp["result"];
        assert!(result["id"].as_u64().is_some());
        assert!(result["faces"].as_u64().unwrap() >= 10); // 2 caps + 8 lateral
    }

    #[test]
    fn test_create_sphere() {
        let mut server = McpServer::new();
        let resp = call_tool(
            &mut server,
            "create_primitive",
            serde_json::json!({
                "type": "sphere",
                "dimensions": {"radius": 2.0, "segments": 8, "rings": 4}
            }),
        );
        assert!(resp["error"].is_null());
        let result = &resp["result"];
        assert!(result["faces"].as_u64().unwrap() > 0);
    }

    #[test]
    fn test_create_cone() {
        let mut server = McpServer::new();
        // Pointed cone
        let resp = call_tool(
            &mut server,
            "create_primitive",
            serde_json::json!({
                "type": "cone",
                "dimensions": {"base_radius": 1.5, "top_radius": 0.0, "height": 3.0, "segments": 8}
            }),
        );
        assert!(resp["error"].is_null());
        assert!(resp["result"]["faces"].as_u64().unwrap() >= 9); // 1 base + 8 lateral
    }

    #[test]
    fn test_create_torus() {
        let mut server = McpServer::new();
        let resp = call_tool(
            &mut server,
            "create_primitive",
            serde_json::json!({
                "type": "torus",
                "dimensions": {"major_radius": 3.0, "minor_radius": 0.5, "major_segments": 8, "minor_segments": 6}
            }),
        );
        assert!(resp["error"].is_null());
        assert_eq!(resp["result"]["faces"].as_u64().unwrap(), 48); // 8*6
    }

    #[test]
    fn test_query_model() {
        let mut server = McpServer::new();
        // Empty
        let resp = call_tool(&mut server, "query_model", serde_json::json!({}));
        assert_eq!(resp["result"]["solid_count"].as_u64().unwrap(), 0);

        // After creating a box
        call_tool(
            &mut server,
            "create_primitive",
            serde_json::json!({"type": "box", "dimensions": {"dx": 1.0, "dy": 1.0, "dz": 1.0}}),
        );
        let resp = call_tool(&mut server, "query_model", serde_json::json!({}));
        assert_eq!(resp["result"]["solid_count"].as_u64().unwrap(), 1);
        assert_eq!(resp["result"]["total_faces"].as_u64().unwrap(), 6);
    }

    #[test]
    fn test_measure_box() {
        let mut server = McpServer::new();
        call_tool(
            &mut server,
            "create_primitive",
            serde_json::json!({"type": "box", "dimensions": {"dx": 2.0, "dy": 3.0, "dz": 4.0}}),
        );
        let resp = call_tool(&mut server, "measure", serde_json::json!({"id": 0}));
        assert!(resp["error"].is_null());
        let result = &resp["result"];
        // Volume should be approximately 24
        let vol = result["volume"].as_f64().unwrap();
        assert!((vol - 24.0).abs() < 1.0, "volume {vol} expected ~24");
        // Bounding box max should be (2, 3, 4)
        let bb_max = result["bounding_box"]["max"].as_array().unwrap();
        assert!((bb_max[0].as_f64().unwrap() - 2.0).abs() < 0.01);
        assert!((bb_max[1].as_f64().unwrap() - 3.0).abs() < 0.01);
        assert!((bb_max[2].as_f64().unwrap() - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_transform_translate() {
        let mut server = McpServer::new();
        call_tool(
            &mut server,
            "create_primitive",
            serde_json::json!({"type": "box", "dimensions": {"dx": 1.0, "dy": 1.0, "dz": 1.0}}),
        );
        let resp = call_tool(
            &mut server,
            "transform",
            serde_json::json!({"id": 0, "translate": [10.0, 20.0, 30.0]}),
        );
        assert!(resp["error"].is_null());

        // Measure to verify translation
        let m = call_tool(&mut server, "measure", serde_json::json!({"id": 0}));
        let bb_min = m["result"]["bounding_box"]["min"].as_array().unwrap();
        assert!((bb_min[0].as_f64().unwrap() - 10.0).abs() < 0.01);
        assert!((bb_min[1].as_f64().unwrap() - 20.0).abs() < 0.01);
        assert!((bb_min[2].as_f64().unwrap() - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_export_stl() {
        let mut server = McpServer::new();
        call_tool(
            &mut server,
            "create_primitive",
            serde_json::json!({"type": "box", "dimensions": {"dx": 1.0, "dy": 1.0, "dz": 1.0}}),
        );
        let resp = call_tool(
            &mut server,
            "export_model",
            serde_json::json!({"format": "stl", "id": 0}),
        );
        assert!(resp["error"].is_null());
        let data = resp["result"]["data"].as_str().unwrap();
        assert!(data.starts_with("solid "));
        assert!(data.contains("facet normal"));
        assert!(resp["result"]["size"].as_u64().unwrap() > 0);
    }

    #[test]
    fn test_export_obj() {
        let mut server = McpServer::new();
        call_tool(
            &mut server,
            "create_primitive",
            serde_json::json!({"type": "box", "dimensions": {"dx": 1.0, "dy": 1.0, "dz": 1.0}}),
        );
        let resp = call_tool(
            &mut server,
            "export_model",
            serde_json::json!({"format": "obj", "id": 0}),
        );
        assert!(resp["error"].is_null());
        let data = resp["result"]["data"].as_str().unwrap();
        assert!(data.contains("v "));
        assert!(data.contains("f "));
    }

    #[test]
    fn test_export_step() {
        let mut server = McpServer::new();
        call_tool(
            &mut server,
            "create_primitive",
            serde_json::json!({"type": "box", "dimensions": {"dx": 1.0, "dy": 1.0, "dz": 1.0}}),
        );
        let resp = call_tool(
            &mut server,
            "export_model",
            serde_json::json!({"format": "step", "id": 0}),
        );
        assert!(resp["error"].is_null());
        let data = resp["result"]["data"].as_str().unwrap();
        assert!(data.contains("ISO-10303-21"));
    }

    #[test]
    fn test_export_json() {
        let mut server = McpServer::new();
        call_tool(
            &mut server,
            "create_primitive",
            serde_json::json!({"type": "box", "dimensions": {"dx": 1.0, "dy": 1.0, "dz": 1.0}}),
        );
        let resp = call_tool(
            &mut server,
            "export_model",
            serde_json::json!({"format": "json", "id": 0}),
        );
        assert!(resp["error"].is_null());
        let data = resp["result"]["data"].as_str().unwrap();
        // Should be valid JSON
        let parsed: Value = serde_json::from_str(data).unwrap();
        assert!(parsed.is_object());
    }

    #[test]
    fn test_delete_solid() {
        let mut server = McpServer::new();
        call_tool(
            &mut server,
            "create_primitive",
            serde_json::json!({"type": "box", "dimensions": {"dx": 1.0, "dy": 1.0, "dz": 1.0}}),
        );
        let resp = call_tool(&mut server, "delete_solid", serde_json::json!({"id": 0}));
        assert!(resp["error"].is_null());
        assert_eq!(resp["result"]["deleted"].as_u64().unwrap(), 0);

        // Verify it was deleted
        let q = call_tool(&mut server, "query_model", serde_json::json!({}));
        assert_eq!(q["result"]["solid_count"].as_u64().unwrap(), 0);
    }

    #[test]
    fn test_list_solids() {
        let mut server = McpServer::new();
        call_tool(
            &mut server,
            "create_primitive",
            serde_json::json!({"type": "box", "dimensions": {"dx": 1.0, "dy": 1.0, "dz": 1.0}}),
        );
        call_tool(
            &mut server,
            "create_primitive",
            serde_json::json!({"type": "sphere", "dimensions": {"radius": 2.0, "segments": 8, "rings": 4}}),
        );
        let resp = call_tool(&mut server, "list_solids", serde_json::json!({}));
        assert!(resp["error"].is_null());
        let solids = resp["result"]["solids"].as_array().unwrap();
        assert_eq!(solids.len(), 2);
        assert_eq!(solids[0]["id"].as_u64().unwrap(), 0);
        assert_eq!(solids[1]["id"].as_u64().unwrap(), 1);
    }

    #[test]
    fn test_error_invalid_tool() {
        let mut server = McpServer::new();
        let resp = call_tool(&mut server, "nonexistent_tool", serde_json::json!({}));
        assert!(!resp["error"].is_null());
    }

    #[test]
    fn test_error_missing_params() {
        let mut server = McpServer::new();
        let resp = call_tool(
            &mut server,
            "create_primitive",
            serde_json::json!({}), // missing type and dimensions
        );
        assert!(!resp["error"].is_null());
    }

    #[test]
    fn test_error_invalid_id() {
        let mut server = McpServer::new();
        let resp = call_tool(&mut server, "measure", serde_json::json!({"id": 999}));
        assert!(!resp["error"].is_null());
    }

    #[test]
    fn test_error_invalid_method() {
        let mut server = McpServer::new();
        let resp = call(&mut server, "unknown/method", serde_json::json!({}));
        assert!(!resp["error"].is_null());
    }

    #[test]
    fn test_error_bad_json() {
        let mut server = McpServer::new();
        let resp_str = server.handle_request("not valid json{{{").unwrap();
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        assert!(!resp["error"].is_null());
        assert_eq!(resp["error"]["code"].as_i64().unwrap(), PARSE_ERROR);
    }

    #[test]
    fn test_error_invalid_jsonrpc_version() {
        let mut server = McpServer::new();
        let req = serde_json::json!({
            "jsonrpc": "1.0",
            "id": 1,
            "method": "tools/list",
            "params": {}
        });
        let resp_str = server.handle_request(&req.to_string()).unwrap();
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        assert!(!resp["error"].is_null());
    }

    #[test]
    fn test_boolean_union() {
        let mut server = McpServer::new();
        // Two overlapping boxes
        call_tool(
            &mut server,
            "create_primitive",
            serde_json::json!({"type": "box", "dimensions": {"dx": 2.0, "dy": 2.0, "dz": 2.0}}),
        );
        call_tool(
            &mut server,
            "create_primitive",
            serde_json::json!({"type": "box", "dimensions": {"dx": 2.0, "dy": 2.0, "dz": 2.0}}),
        );
        // Translate second box
        call_tool(
            &mut server,
            "transform",
            serde_json::json!({"id": 1, "translate": [1.0, 1.0, 1.0]}),
        );
        let resp = call_tool(
            &mut server,
            "boolean_operation",
            serde_json::json!({"op": "union", "target_id": 0, "tool_id": 1}),
        );
        assert!(resp["error"].is_null());
        assert!(resp["result"]["id"].as_u64().is_some());
        assert!(resp["result"]["faces"].as_u64().unwrap() > 0);
    }

    #[test]
    fn test_full_workflow() {
        let mut server = McpServer::new();

        // 1. Create box
        let r1 = call_tool(
            &mut server,
            "create_primitive",
            serde_json::json!({"type": "box", "dimensions": {"dx": 10.0, "dy": 10.0, "dz": 10.0}}),
        );
        assert!(r1["error"].is_null());
        let box_id = r1["result"]["id"].as_u64().unwrap();

        // 2. Create cylinder
        let r2 = call_tool(
            &mut server,
            "create_primitive",
            serde_json::json!({
                "type": "cylinder",
                "dimensions": {"radius": 3.0, "height": 15.0, "segments": 12}
            }),
        );
        assert!(r2["error"].is_null());
        let cyl_id = r2["result"]["id"].as_u64().unwrap();

        // 3. Query
        let q = call_tool(&mut server, "query_model", serde_json::json!({}));
        assert_eq!(q["result"]["solid_count"].as_u64().unwrap(), 2);

        // 4. List
        let ls = call_tool(&mut server, "list_solids", serde_json::json!({}));
        assert_eq!(ls["result"]["solids"].as_array().unwrap().len(), 2);

        // 5. Measure box
        let m = call_tool(&mut server, "measure", serde_json::json!({"id": box_id}));
        assert!(m["error"].is_null());
        let vol = m["result"]["volume"].as_f64().unwrap();
        assert!(
            (vol - 1000.0).abs() < 10.0,
            "box volume {vol} expected ~1000"
        );

        // 6. Export as STL
        let e = call_tool(
            &mut server,
            "export_model",
            serde_json::json!({"format": "stl", "id": box_id}),
        );
        assert!(e["error"].is_null());
        assert!(
            e["result"]["data"]
                .as_str()
                .unwrap()
                .contains("facet normal")
        );

        // 7. Delete cylinder
        let d = call_tool(
            &mut server,
            "delete_solid",
            serde_json::json!({"id": cyl_id}),
        );
        assert!(d["error"].is_null());

        // 8. Verify only box remains
        let q2 = call_tool(&mut server, "query_model", serde_json::json!({}));
        assert_eq!(q2["result"]["solid_count"].as_u64().unwrap(), 1);
    }

    #[test]
    fn test_slot_reuse_after_delete() {
        let mut server = McpServer::new();
        // Create solid at slot 0
        call_tool(
            &mut server,
            "create_primitive",
            serde_json::json!({"type": "box", "dimensions": {"dx": 1.0, "dy": 1.0, "dz": 1.0}}),
        );
        // Delete slot 0
        call_tool(&mut server, "delete_solid", serde_json::json!({"id": 0}));
        // Create another -- should reuse slot 0
        let r = call_tool(
            &mut server,
            "create_primitive",
            serde_json::json!({"type": "sphere", "dimensions": {"radius": 1.0, "segments": 6, "rings": 3}}),
        );
        assert_eq!(r["result"]["id"].as_u64().unwrap(), 0);
    }

    #[test]
    fn test_transform_scale() {
        let mut server = McpServer::new();
        call_tool(
            &mut server,
            "create_primitive",
            serde_json::json!({"type": "box", "dimensions": {"dx": 1.0, "dy": 1.0, "dz": 1.0}}),
        );
        call_tool(
            &mut server,
            "transform",
            serde_json::json!({"id": 0, "scale": [2.0, 3.0, 4.0]}),
        );
        let m = call_tool(&mut server, "measure", serde_json::json!({"id": 0}));
        let bb_max = m["result"]["bounding_box"]["max"].as_array().unwrap();
        assert!((bb_max[0].as_f64().unwrap() - 2.0).abs() < 0.01);
        assert!((bb_max[1].as_f64().unwrap() - 3.0).abs() < 0.01);
        assert!((bb_max[2].as_f64().unwrap() - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_error_delete_nonexistent() {
        let mut server = McpServer::new();
        let resp = call_tool(&mut server, "delete_solid", serde_json::json!({"id": 42}));
        assert!(!resp["error"].is_null());
    }

    #[test]
    fn test_error_boolean_same_id() {
        let mut server = McpServer::new();
        call_tool(
            &mut server,
            "create_primitive",
            serde_json::json!({"type": "box", "dimensions": {"dx": 1.0, "dy": 1.0, "dz": 1.0}}),
        );
        let resp = call_tool(
            &mut server,
            "boolean_operation",
            serde_json::json!({"op": "union", "target_id": 0, "tool_id": 0}),
        );
        assert!(!resp["error"].is_null());
    }

    #[test]
    fn test_error_unsupported_export_format() {
        let mut server = McpServer::new();
        call_tool(
            &mut server,
            "create_primitive",
            serde_json::json!({"type": "box", "dimensions": {"dx": 1.0, "dy": 1.0, "dz": 1.0}}),
        );
        let resp = call_tool(
            &mut server,
            "export_model",
            serde_json::json!({"format": "fbx", "id": 0}),
        );
        assert!(!resp["error"].is_null());
    }

    #[test]
    fn test_mcp_server_default() {
        let server = McpServer::default();
        assert!(server.solids.is_empty());
    }

    #[test]
    fn test_create_frustum() {
        let mut server = McpServer::new();
        let resp = call_tool(
            &mut server,
            "create_primitive",
            serde_json::json!({
                "type": "cone",
                "dimensions": {"base_radius": 2.0, "top_radius": 1.0, "height": 3.0, "segments": 8}
            }),
        );
        assert!(resp["error"].is_null());
        // Frustum: 2 caps + 8 lateral = 10 faces
        assert_eq!(resp["result"]["faces"].as_u64().unwrap(), 10);
    }
}
