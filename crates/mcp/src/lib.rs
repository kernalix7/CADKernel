//! Model Context Protocol (MCP) server for CADKernel.
//!
//! Implements a JSON-RPC 2.0 server that exposes the
//! [`cadkernel_api::Session`] surface as MCP tools. Eight tools are
//! provided, all routed through `Session::execute(Command::*)` with the
//! single exception of `export_model`, which reaches into the
//! [`cadkernel_io`] adapters because no serializing `Command` exists
//! (and none is permitted by `STOP_LIST.md`).
//!
//! - `create_primitive` — box, cylinder, sphere, cone, torus
//! - `boolean_operation` — union, subtract, intersect
//! - `transform` — translate, rotate (XYZ Euler in degrees), scale
//! - `query_model` — solid / face / edge / vertex counts
//! - `measure` — volume, surface area, centroid, bounding box
//! - `export_model` — STL, OBJ, STEP, JSON
//! - `delete_solid` — remove a solid by ID
//! - `list_solids` — enumerate stored solids

// v0.5 Gate #12 — enforce panic-free public API. Non-test code in this
// crate must not contain `unwrap()`, `expect()`, or `panic!()`.
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod server;

pub use server::{McpError, McpRequest, McpResponse, McpServer, McpToolDef};
