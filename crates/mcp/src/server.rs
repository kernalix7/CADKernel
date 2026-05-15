//! MCP server — JSON-RPC 2.0 over `cadkernel-api`.
//!
//! Every state-mutating tool dispatches through
//! [`cadkernel_api::Session::execute`] so the model state is always
//! managed by the canonical API surface. The single exception is
//! `export_model`, which reaches into the underlying `BRepModel` via
//! [`Document::solid_brep`] because the file-format adapters live in
//! `cadkernel-io` and there is no serializing `Command` (and the
//! `STOP_LIST.md` scope freeze forbids adding one).

use cadkernel_api::{ApiError, Command, Outcome, Session, SolidId};
use cadkernel_core::KernelError;
use cadkernel_io as io;
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
const MAX_SOLIDS: usize = 1000;

// ---------------------------------------------------------------------------
// MCP Server
// ---------------------------------------------------------------------------

/// An MCP server that owns a single [`Session`] and maps stable MCP
/// integer ids onto the session's [`SolidId`]s. Slot indices are reused
/// after [`tool_delete_solid`](McpServer::tool_delete_solid) so the
/// integer-id surface mirrors the original `crates/io/src/mcp.rs`
/// behaviour.
pub struct McpServer {
    session: Session,
    slots: Vec<Option<SlotEntry>>,
}

struct SlotEntry {
    solid: SolidId,
    label: String,
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl McpServer {
    /// Creates a new MCP server with an empty session.
    pub fn new() -> Self {
        Self {
            session: Session::new(),
            slots: Vec::new(),
        }
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
                            "description": "box: {dx,dy,dz}, cylinder: {radius,height}, sphere: {radius}, cone: {base_radius,top_radius=0,height}, torus: {major_radius,minor_radius}"
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
    pub fn handle_request(&mut self, json: &str) -> Result<String, KernelError> {
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
    // Slot helpers (MCP integer id ⇄ Session::SolidId)
    // -----------------------------------------------------------------------

    fn alloc_slot(&mut self, solid: SolidId, label: String) -> Result<usize, McpError> {
        let live = self.slots.iter().filter(|s| s.is_some()).count();
        if live >= MAX_SOLIDS {
            return Err(McpError {
                code: INVALID_PARAMS,
                message: format!("solid limit reached ({MAX_SOLIDS})"),
                data: None,
            });
        }
        let entry = SlotEntry { solid, label };
        for (i, slot) in self.slots.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(entry);
                return Ok(i);
            }
        }
        let id = self.slots.len();
        self.slots.push(Some(entry));
        Ok(id)
    }

    fn slot_solid(&self, id: usize) -> Result<SolidId, McpError> {
        self.slots
            .get(id)
            .and_then(|s| s.as_ref())
            .map(|e| e.solid)
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

        let (command, label) = match ptype {
            "box" => {
                let dx = get_f64(dims, "dx").unwrap_or(1.0);
                let dy = get_f64(dims, "dy").unwrap_or(1.0);
                let dz = get_f64(dims, "dz").unwrap_or(1.0);
                (
                    Command::CreateBox { dx, dy, dz },
                    format!("box({dx},{dy},{dz})"),
                )
            }
            "cylinder" => {
                let radius = get_f64(dims, "radius").unwrap_or(1.0);
                let height = get_f64(dims, "height").unwrap_or(2.0);
                (
                    Command::CreateCylinder { radius, height },
                    format!("cylinder(r={radius},h={height})"),
                )
            }
            "sphere" => {
                let radius = get_f64(dims, "radius").unwrap_or(1.0);
                (
                    Command::CreateSphere { radius },
                    format!("sphere(r={radius})"),
                )
            }
            "cone" => {
                let base_radius = get_f64(dims, "base_radius").unwrap_or(1.0);
                let top_radius = get_f64(dims, "top_radius").unwrap_or(0.0);
                let height = get_f64(dims, "height").unwrap_or(2.0);
                if top_radius.abs() > 1e-12 {
                    return Err(McpError {
                        code: INVALID_PARAMS,
                        message: "top_radius must be 0; non-zero cones deferred".into(),
                        data: None,
                    });
                }
                (
                    Command::CreateCone {
                        radius: base_radius,
                        height,
                        top_radius: 0.0,
                    },
                    format!("cone(br={base_radius},h={height})"),
                )
            }
            "torus" => {
                let major_radius = get_f64(dims, "major_radius").unwrap_or(2.0);
                let minor_radius = get_f64(dims, "minor_radius").unwrap_or(0.5);
                (
                    Command::CreateTorus {
                        major_radius,
                        minor_radius,
                    },
                    format!("torus(R={major_radius},r={minor_radius})"),
                )
            }
            other => {
                return Err(McpError {
                    code: INVALID_PARAMS,
                    message: format!("unknown primitive type: {other}"),
                    data: None,
                });
            }
        };

        let outcome = self.session.execute(command).map_err(api_to_mcp)?;
        let Outcome::SolidCreated { id: solid_id, .. } = outcome else {
            return Err(McpError {
                code: INVALID_PARAMS,
                message: "primitive command did not return SolidCreated".into(),
                data: None,
            });
        };
        let (faces, edges, vertices) = self.counts_for(solid_id);
        let mcp_id = self.alloc_slot(solid_id, label)?;
        Ok(serde_json::json!({
            "id": mcp_id,
            "faces": faces,
            "edges": edges,
            "vertices": vertices
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

        let lhs = self.slot_solid(target_id)?;
        let rhs = self.slot_solid(tool_id)?;

        let command = match op_str {
            "union" => Command::BooleanUnion { lhs, rhs },
            "subtract" => Command::BooleanSubtract { lhs, rhs },
            "intersect" => Command::BooleanIntersect { lhs, rhs },
            other => {
                return Err(McpError {
                    code: INVALID_PARAMS,
                    message: format!("unknown boolean op: {other}"),
                    data: None,
                });
            }
        };

        let outcome = self.session.execute(command).map_err(api_to_mcp)?;
        let Outcome::Booleaned { result, .. } = outcome else {
            return Err(McpError {
                code: INVALID_PARAMS,
                message: "boolean operation did not return Booleaned".into(),
                data: None,
            });
        };

        // Booleans consume both inputs — clear the old slots.
        self.slots[target_id] = None;
        self.slots[tool_id] = None;

        let label = format!("{op_str}({target_id},{tool_id})");
        let (faces, edges, vertices) = self.counts_for(result);
        let mcp_id = self.alloc_slot(result, label)?;

        Ok(serde_json::json!({
            "id": mcp_id,
            "faces": faces,
            "edges": edges,
            "vertices": vertices
        }))
    }

    fn tool_transform(&mut self, args: &Value) -> Result<Value, McpError> {
        let id = get_usize(args, "id")?;
        let solid = self.slot_solid(id)?;

        if let Some(tr) = args.get("translate").and_then(|v| v.as_array())
            && tr.len() == 3
        {
            let dx = tr[0].as_f64().unwrap_or(0.0);
            let dy = tr[1].as_f64().unwrap_or(0.0);
            let dz = tr[2].as_f64().unwrap_or(0.0);
            self.session
                .execute(Command::Translate {
                    id: solid,
                    dx,
                    dy,
                    dz,
                })
                .map_err(api_to_mcp)?;
        }

        if let Some(rot) = args.get("rotate").and_then(|v| v.as_array())
            && rot.len() == 3
        {
            let rx = rot[0].as_f64().unwrap_or(0.0).to_radians();
            let ry = rot[1].as_f64().unwrap_or(0.0).to_radians();
            let rz = rot[2].as_f64().unwrap_or(0.0).to_radians();
            for (axis, angle) in [
                ([1.0, 0.0, 0.0], rx),
                ([0.0, 1.0, 0.0], ry),
                ([0.0, 0.0, 1.0], rz),
            ] {
                if angle.abs() > f64::EPSILON {
                    self.session
                        .execute(Command::Rotate {
                            id: solid,
                            axis,
                            angle_rad: angle,
                            point: [0.0, 0.0, 0.0],
                        })
                        .map_err(api_to_mcp)?;
                }
            }
        }

        if let Some(sc) = args.get("scale").and_then(|v| v.as_array())
            && sc.len() == 3
        {
            let sx = sc[0].as_f64().unwrap_or(1.0);
            let sy = sc[1].as_f64().unwrap_or(1.0);
            let sz = sc[2].as_f64().unwrap_or(1.0);
            self.session
                .execute(Command::ScaleNonUniform {
                    id: solid,
                    factors: [sx, sy, sz],
                    point: [0.0, 0.0, 0.0],
                })
                .map_err(api_to_mcp)?;
        }

        let (faces, edges, vertices) = self.counts_for(solid);
        Ok(serde_json::json!({
            "id": id,
            "faces": faces,
            "edges": edges,
            "vertices": vertices
        }))
    }

    fn tool_query_model(&mut self) -> Result<Value, McpError> {
        let outcome = self.session.execute(Command::Stats).map_err(api_to_mcp)?;
        let solid_count = match outcome {
            Outcome::Stats { solid_count, .. } => solid_count as u64,
            _ => 0,
        };
        let mut total_faces = 0u64;
        let mut total_edges = 0u64;
        let mut total_vertices = 0u64;
        for slot in self.slots.iter().flatten() {
            if let Some((model, _)) = self.session.document().solid_brep(slot.solid) {
                total_faces += model.faces.len() as u64;
                total_edges += model.edges.len() as u64;
                total_vertices += model.vertices.len() as u64;
            }
        }
        Ok(serde_json::json!({
            "solid_count": solid_count,
            "total_faces": total_faces,
            "total_edges": total_edges,
            "total_vertices": total_vertices
        }))
    }

    fn tool_measure(&mut self, args: &Value) -> Result<Value, McpError> {
        let id = get_usize(args, "id")?;
        let solid = self.slot_solid(id)?;
        let outcome = self
            .session
            .execute(Command::Measure { id: solid })
            .map_err(api_to_mcp)?;
        let Outcome::Measured {
            volume,
            surface_area,
            centroid,
            bbox_min,
            bbox_max,
            ..
        } = outcome
        else {
            return Err(McpError {
                code: INVALID_PARAMS,
                message: "measure did not return Measured".into(),
                data: None,
            });
        };
        Ok(serde_json::json!({
            "volume": volume,
            "surface_area": surface_area,
            "centroid": centroid,
            "bounding_box": {
                "min": bbox_min,
                "max": bbox_max
            }
        }))
    }

    fn tool_export_model(&mut self, args: &Value) -> Result<Value, McpError> {
        let fmt = get_str(args, "format")?.to_string();
        let id = get_usize(args, "id")?;
        let solid = self.slot_solid(id)?;
        let label = self
            .slots
            .get(id)
            .and_then(|s| s.as_ref())
            .map(|e| e.label.clone())
            .unwrap_or_else(|| format!("solid{id}"));

        let data = match fmt.as_str() {
            "stl" => {
                let (model, handle) =
                    self.session
                        .document()
                        .solid_brep(solid)
                        .ok_or_else(|| McpError {
                            code: INVALID_PARAMS,
                            message: format!("no solid with id {id}"),
                            data: None,
                        })?;
                let mesh = io::tessellate_solid(model, handle);
                io::write_stl_ascii(&mesh, &label)
            }
            "obj" => {
                let (model, handle) =
                    self.session
                        .document()
                        .solid_brep(solid)
                        .ok_or_else(|| McpError {
                            code: INVALID_PARAMS,
                            message: format!("no solid with id {id}"),
                            data: None,
                        })?;
                let mesh = io::tessellate_solid(model, handle);
                io::write_obj(&mesh)
            }
            "step" => {
                let (model, _handle) =
                    self.session
                        .document()
                        .solid_brep(solid)
                        .ok_or_else(|| McpError {
                            code: INVALID_PARAMS,
                            message: format!("no solid with id {id}"),
                            data: None,
                        })?;
                io::export_step(model).map_err(kernel_to_mcp)?
            }
            "json" => {
                let (model, _handle) =
                    self.session
                        .document()
                        .solid_brep(solid)
                        .ok_or_else(|| McpError {
                            code: INVALID_PARAMS,
                            message: format!("no solid with id {id}"),
                            data: None,
                        })?;
                io::model_to_json(model).map_err(kernel_to_mcp)?
            }
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
        let solid = self.slot_solid(id)?;
        self.session
            .execute(Command::DeleteSolid { id: solid })
            .map_err(api_to_mcp)?;
        self.slots[id] = None;
        Ok(serde_json::json!({ "deleted": id }))
    }

    fn tool_list_solids(&self) -> Result<Value, McpError> {
        let mut list = Vec::new();
        for (i, slot) in self.slots.iter().enumerate() {
            if let Some(entry) = slot
                && let Some((model, _)) = self.session.document().solid_brep(entry.solid)
            {
                list.push(serde_json::json!({
                    "id": i,
                    "label": entry.label,
                    "faces": model.faces.len(),
                    "edges": model.edges.len(),
                    "vertices": model.vertices.len()
                }));
            }
        }
        Ok(serde_json::json!({ "solids": list }))
    }

    // -----------------------------------------------------------------------
    // Counts helper — kernel-side face/edge/vertex totals.
    // -----------------------------------------------------------------------

    fn counts_for(&self, solid: SolidId) -> (usize, usize, usize) {
        match self.session.document().solid_brep(solid) {
            Some((model, handle)) => {
                let mut face_count = 0usize;
                if let Some(sd) = model.solids.get(handle) {
                    for &sh in &sd.shells {
                        if let Some(shell) = model.shells.get(sh) {
                            face_count += shell.faces.len();
                        }
                    }
                }
                (face_count, model.edges.len(), model.vertices.len())
            }
            None => (0, 0, 0),
        }
    }
}

// ---------------------------------------------------------------------------
// Response helpers
// ---------------------------------------------------------------------------

fn to_json(resp: &McpResponse) -> Result<String, KernelError> {
    serde_json::to_string(resp).map_err(|e| KernelError::IoError(e.to_string()))
}

fn ok_response(id: Value, result: Result<Value, McpError>) -> Result<String, KernelError> {
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

fn api_to_mcp(e: ApiError) -> McpError {
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
