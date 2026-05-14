//! Lua scripting engine for CADKernel.
//!
//! Embeds a Lua 5.4 interpreter (via `mlua` with vendored Lua) and exposes
//! CADKernel modeling operations as `cad.*` functions. Scripts can create
//! primitives, perform booleans, apply transforms, query mass properties,
//! and import/export meshes.
//!
//! # Example
//!
//! ```no_run
//! use cadkernel_viewer::scripting::ScriptEngine;
//!
//! let mut engine = ScriptEngine::new().unwrap();
//! let output = engine.execute(r#"
//!     local b = cad.box(10, 20, 30)
//!     local info = cad.measure(b)
//!     return string.format("volume = %.1f", info.volume)
//! "#).unwrap();
//! assert!(output.contains("volume"));
//! ```

use std::path::Path;
use std::sync::{Arc, Mutex};

use cadkernel_api::{Command, ExtrudeKind, Outcome, Session, SolidId};
use cadkernel_core::{KernelError, KernelResult};
use cadkernel_topology::{BRepModel, Handle, SolidData};
use mlua::{Lua, Result as LuaResult, Table, Value};

// ---------------------------------------------------------------------------
// ScriptEngine
// ---------------------------------------------------------------------------

/// Lua-based scripting engine for CADKernel.
///
/// Each engine owns an independent Lua interpreter and a single API
/// [`Session`] whose document is addressable from Lua by `SolidId.0`.
pub struct ScriptEngine {
    lua: Lua,
    session: Arc<Mutex<Session>>,
}

impl ScriptEngine {
    /// Creates a new scripting engine, returning an error if Lua
    /// initialisation fails.
    pub fn new() -> KernelResult<Self> {
        let lua = Lua::new();

        // Sandbox: remove dangerous standard library globals
        lua.load(
            r#"
            os = nil
            io = nil
            require = nil
            dofile = nil
            loadfile = nil
            package = nil
        "#,
        )
        .exec()
        .map_err(|e| KernelError::InvalidArgument(format!("lua sandbox: {e}")))?;

        let session = Arc::new(Mutex::new(Session::new()));

        register_cad_table(&lua, Arc::clone(&session))
            .map_err(|e| KernelError::InvalidArgument(format!("lua init: {e}")))?;

        Ok(Self { lua, session })
    }

    /// Executes a Lua code string and returns its textual output.
    ///
    /// If the script returns a value, it is converted to a string via
    /// Lua `tostring`. Otherwise an empty string is returned.
    pub fn execute(&mut self, code: &str) -> KernelResult<String> {
        let val: Value = self
            .lua
            .load(code)
            .eval()
            .map_err(|e| KernelError::InvalidArgument(format!("lua exec: {e}")))?;
        Ok(lua_value_to_string(&val))
    }

    /// Executes a Lua script file.
    pub fn execute_file(&mut self, path: &str) -> KernelResult<String> {
        let code = std::fs::read_to_string(path)?;
        self.execute(&code)
    }

    /// Returns cloned models for all live solids held by the engine.
    pub fn get_models(&self) -> Vec<BRepModel> {
        let Ok(guard) = self.session.lock() else {
            return Vec::new();
        };
        guard
            .document()
            .solid_ids()
            .into_iter()
            .filter_map(|id| {
                guard
                    .document()
                    .clone_solid_brep(id)
                    .map(|(model, _)| model)
            })
            .collect()
    }

    /// Returns the number of live solids.
    pub fn solid_count(&self) -> usize {
        self.session
            .lock()
            .map(|s| s.document().solid_count())
            .unwrap_or(0)
    }

    /// Returns the shared API session used by the Lua bridge.
    pub fn session_arc(&self) -> Arc<Mutex<Session>> {
        Arc::clone(&self.session)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn lua_value_to_string(val: &Value) -> String {
    match val {
        Value::Nil => String::new(),
        Value::Boolean(b) => b.to_string(),
        Value::Integer(i) => i.to_string(),
        Value::Number(n) => format!("{n}"),
        Value::String(s) => s.to_str().map(|v| v.to_string()).unwrap_or_default(),
        _ => format!("{val:?}"),
    }
}

fn exec_cmd(session: &Arc<Mutex<Session>>, cmd: Command) -> LuaResult<Outcome> {
    let mut s = session
        .lock()
        .map_err(|e| mlua::Error::external(format!("script session lock poisoned: {e}")))?;
    s.execute(cmd)
        .map_err(|e| mlua::Error::external(e.to_string()))
}

fn clone_solid_brep(
    session: &Arc<Mutex<Session>>,
    id: SolidId,
) -> LuaResult<(BRepModel, Handle<SolidData>)> {
    let s = session
        .lock()
        .map_err(|e| mlua::Error::external(format!("script session lock poisoned: {e}")))?;
    s.document()
        .clone_solid_brep(id)
        .ok_or_else(|| mlua::Error::external(format!("no solid with id {}", id.0)))
}

fn expect_solid_created(outcome: Outcome, op: &str) -> LuaResult<i64> {
    match outcome {
        Outcome::SolidCreated { id, .. } => Ok(id.0 as i64),
        other => Err(mlua::Error::external(format!(
            "unexpected {op} outcome: {other:?}"
        ))),
    }
}

fn expect_booleaned(outcome: Outcome, op: &str) -> LuaResult<i64> {
    match outcome {
        Outcome::Booleaned { result, .. } => Ok(result.0 as i64),
        other => Err(mlua::Error::external(format!(
            "unexpected {op} outcome: {other:?}"
        ))),
    }
}

fn expect_modified(outcome: Outcome, op: &str) -> LuaResult<i64> {
    match outcome {
        Outcome::SolidModified { id } => Ok(id.0 as i64),
        other => Err(mlua::Error::external(format!(
            "unexpected {op} outcome: {other:?}"
        ))),
    }
}

#[derive(Copy, Clone)]
enum BooleanCommand {
    Union,
    Subtract,
    Intersect,
}

// ---------------------------------------------------------------------------
// Registration - `cad` table
// ---------------------------------------------------------------------------

fn register_cad_table(lua: &Lua, session: Arc<Mutex<Session>>) -> LuaResult<()> {
    let cad = lua.create_table()?;

    // ---- Primitives -------------------------------------------------------

    register_box(lua, &cad, Arc::clone(&session))?;
    register_cylinder(lua, &cad, Arc::clone(&session))?;
    register_sphere(lua, &cad, Arc::clone(&session))?;
    register_cone(lua, &cad, Arc::clone(&session))?;
    register_torus(lua, &cad, Arc::clone(&session))?;

    // ---- Booleans ---------------------------------------------------------

    register_boolean(
        lua,
        &cad,
        Arc::clone(&session),
        "union",
        BooleanCommand::Union,
    )?;
    register_boolean(
        lua,
        &cad,
        Arc::clone(&session),
        "subtract",
        BooleanCommand::Subtract,
    )?;
    register_boolean(
        lua,
        &cad,
        Arc::clone(&session),
        "intersect",
        BooleanCommand::Intersect,
    )?;

    // ---- Transforms -------------------------------------------------------

    register_translate(lua, &cad, Arc::clone(&session))?;
    register_rotate(lua, &cad, Arc::clone(&session))?;
    register_scale(lua, &cad, Arc::clone(&session))?;
    register_mirror(lua, &cad, Arc::clone(&session))?;

    // ---- Features ---------------------------------------------------------

    register_extrude(lua, &cad, Arc::clone(&session))?;
    register_fillet(lua, &cad, Arc::clone(&session))?;
    register_chamfer(lua, &cad, Arc::clone(&session))?;

    // ---- Query ------------------------------------------------------------

    register_measure(lua, &cad, Arc::clone(&session))?;
    register_count(lua, &cad, Arc::clone(&session))?;
    register_bounds(lua, &cad, Arc::clone(&session))?;

    // ---- I/O --------------------------------------------------------------

    register_export_stl(lua, &cad, Arc::clone(&session))?;
    register_export_obj(lua, &cad, Arc::clone(&session))?;
    register_import_stl(lua, &cad)?;

    // ---- Utility ----------------------------------------------------------

    register_list(lua, &cad, Arc::clone(&session))?;
    register_delete(lua, &cad, Arc::clone(&session))?;
    register_clear(lua, &cad, Arc::clone(&session))?;

    lua.globals().set("cad", cad)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Primitives
// ---------------------------------------------------------------------------

fn register_box(lua: &Lua, cad: &Table, session: Arc<Mutex<Session>>) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, (w, h, d): (f64, f64, f64)| {
        let outcome = exec_cmd(
            &session,
            Command::CreateBox {
                dx: w,
                dy: h,
                dz: d,
            },
        )?;
        expect_solid_created(outcome, "box")
    })?;
    cad.set("box", f)
}

fn register_cylinder(lua: &Lua, cad: &Table, session: Arc<Mutex<Session>>) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, (r, h): (f64, f64)| {
        let outcome = exec_cmd(
            &session,
            Command::CreateCylinder {
                radius: r,
                height: h,
            },
        )?;
        expect_solid_created(outcome, "cylinder")
    })?;
    cad.set("cylinder", f)
}

fn register_sphere(lua: &Lua, cad: &Table, session: Arc<Mutex<Session>>) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, r: f64| {
        let outcome = exec_cmd(&session, Command::CreateSphere { radius: r })?;
        expect_solid_created(outcome, "sphere")
    })?;
    cad.set("sphere", f)
}

fn register_cone(lua: &Lua, cad: &Table, session: Arc<Mutex<Session>>) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, (r, h): (f64, f64)| {
        let outcome = exec_cmd(
            &session,
            Command::CreateCone {
                radius: r,
                height: h,
                top_radius: 0.0,
            },
        )?;
        expect_solid_created(outcome, "cone")
    })?;
    cad.set("cone", f)
}

fn register_torus(lua: &Lua, cad: &Table, session: Arc<Mutex<Session>>) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, (major, minor): (f64, f64)| {
        let outcome = exec_cmd(
            &session,
            Command::CreateTorus {
                major_radius: major,
                minor_radius: minor,
            },
        )?;
        expect_solid_created(outcome, "torus")
    })?;
    cad.set("torus", f)
}

// ---------------------------------------------------------------------------
// Booleans
// ---------------------------------------------------------------------------

fn register_boolean(
    lua: &Lua,
    cad: &Table,
    session: Arc<Mutex<Session>>,
    name: &str,
    kind: BooleanCommand,
) -> LuaResult<()> {
    let op_name = name.to_string();
    let f = lua.create_function(move |_lua, (a, b): (u32, u32)| {
        let lhs = SolidId(a);
        let rhs = SolidId(b);
        let command = match kind {
            BooleanCommand::Union => Command::BooleanUnion { lhs, rhs },
            BooleanCommand::Subtract => Command::BooleanSubtract { lhs, rhs },
            BooleanCommand::Intersect => Command::BooleanIntersect { lhs, rhs },
        };
        let outcome = exec_cmd(&session, command)?;
        expect_booleaned(outcome, &op_name)
    })?;
    cad.set(name, f)
}

// ---------------------------------------------------------------------------
// Transforms - translate, rotate, scale, mirror
// ---------------------------------------------------------------------------

fn register_translate(lua: &Lua, cad: &Table, session: Arc<Mutex<Session>>) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, (id, x, y, z): (u32, f64, f64, f64)| {
        let outcome = exec_cmd(
            &session,
            Command::Translate {
                id: SolidId(id),
                dx: x,
                dy: y,
                dz: z,
            },
        )?;
        expect_modified(outcome, "translate")
    })?;
    cad.set("translate", f)
}

fn register_rotate(lua: &Lua, cad: &Table, session: Arc<Mutex<Session>>) -> LuaResult<()> {
    let f = lua.create_function(
        move |_lua, (id, ax, ay, az, angle): (u32, f64, f64, f64, f64)| {
            let outcome = exec_cmd(
                &session,
                Command::Rotate {
                    id: SolidId(id),
                    axis: [ax, ay, az],
                    angle_rad: angle.to_radians(),
                    point: [0.0, 0.0, 0.0],
                },
            )?;
            expect_modified(outcome, "rotate")
        },
    )?;
    cad.set("rotate", f)
}

fn register_scale(lua: &Lua, cad: &Table, session: Arc<Mutex<Session>>) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, (id, sx, sy, sz): (u32, f64, f64, f64)| {
        let eps = 1e-9;
        if (sx - sy).abs() > eps || (sy - sz).abs() > eps {
            return Err(mlua::Error::external(
                "non-uniform scaling not supported; sx, sy, sz must be equal",
            ));
        }
        let outcome = exec_cmd(
            &session,
            Command::Scale {
                id: SolidId(id),
                factor: sx,
            },
        )?;
        expect_modified(outcome, "scale")
    })?;
    cad.set("scale", f)
}

fn register_mirror(lua: &Lua, cad: &Table, session: Arc<Mutex<Session>>) -> LuaResult<()> {
    let f = lua.create_function(
        move |_lua, (id, px, py, pz, nx, ny, nz): (u32, f64, f64, f64, f64, f64, f64)| {
            let outcome = exec_cmd(
                &session,
                Command::Mirror {
                    id: SolidId(id),
                    point: [px, py, pz],
                    normal: [nx, ny, nz],
                    merge: false,
                    features: Vec::new(),
                },
            )?;
            match outcome {
                Outcome::PatternCreated { ids, .. } => {
                    let new_id = ids
                        .last()
                        .copied()
                        .ok_or_else(|| mlua::Error::external("mirror produced no ids"))?;
                    Ok(new_id.0 as i64)
                }
                Outcome::SolidCreated { id, .. } => Ok(id.0 as i64),
                other => Err(mlua::Error::external(format!(
                    "unexpected mirror outcome: {other:?}"
                ))),
            }
        },
    )?;
    cad.set("mirror", f)
}

// ---------------------------------------------------------------------------
// Features - extrude, fillet, chamfer
// ---------------------------------------------------------------------------

fn register_extrude(lua: &Lua, cad: &Table, session: Arc<Mutex<Session>>) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, (pts_table, height): (Table, f64)| {
        let mut profile = Vec::new();
        for pair in pts_table.sequence_values::<Table>() {
            let pt: Table = pair.map_err(|e| mlua::Error::external(e.to_string()))?;
            let x: f64 = pt.get(1)?;
            let y: f64 = pt.get(2)?;
            profile.push([x, y, 0.0]);
        }
        if profile.len() < 3 {
            return Err(mlua::Error::external(
                "extrude requires at least 3 profile points",
            ));
        }

        let outcome = exec_cmd(
            &session,
            Command::Extrude {
                profile,
                direction: [0.0, 0.0, 1.0],
                distance: height,
                kind: ExtrudeKind::default(),
            },
        )?;
        expect_solid_created(outcome, "extrude")
    })?;
    cad.set("extrude", f)
}

fn register_fillet(lua: &Lua, cad: &Table, session: Arc<Mutex<Session>>) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, (id, _radius): (u32, f64)| {
        let _ = clone_solid_brep(&session, SolidId(id))?;
        // TODO(A2.10): route through Session once Command::FilletEdge exists.
        Err::<i64, _>(mlua::Error::external(
            "fillet/chamfer: not yet routed through Session — A2.10 deliverable",
        ))
    })?;
    cad.set("fillet", f)
}

fn register_chamfer(lua: &Lua, cad: &Table, session: Arc<Mutex<Session>>) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, (id, _dist): (u32, f64)| {
        let _ = clone_solid_brep(&session, SolidId(id))?;
        // TODO(A2.10): route through Session once Command::ChamferEdge exists.
        Err::<i64, _>(mlua::Error::external(
            "fillet/chamfer: not yet routed through Session — A2.10 deliverable",
        ))
    })?;
    cad.set("chamfer", f)
}

// ---------------------------------------------------------------------------
// Query - measure, count, bounds
// ---------------------------------------------------------------------------

fn register_measure(lua: &Lua, cad: &Table, session: Arc<Mutex<Session>>) -> LuaResult<()> {
    let f = lua.create_function(move |lua, id: u32| {
        let outcome = exec_cmd(&session, Command::Measure { id: SolidId(id) })?;
        match outcome {
            Outcome::Measured {
                volume,
                surface_area,
                centroid,
                ..
            } => {
                let tbl = lua.create_table()?;
                tbl.set("volume", volume)?;
                tbl.set("area", surface_area)?;
                tbl.set("centroid_x", centroid[0])?;
                tbl.set("centroid_y", centroid[1])?;
                tbl.set("centroid_z", centroid[2])?;
                Ok(tbl)
            }
            other => Err(mlua::Error::external(format!(
                "unexpected measure outcome: {other:?}"
            ))),
        }
    })?;
    cad.set("measure", f)
}

fn register_count(lua: &Lua, cad: &Table, session: Arc<Mutex<Session>>) -> LuaResult<()> {
    let f = lua.create_function(move |lua, id: u32| {
        let (faces, edges, vertices) = {
            let s = session
                .lock()
                .map_err(|e| mlua::Error::external(format!("script session lock poisoned: {e}")))?;
            let (model, _) = s
                .document()
                .solid_brep(SolidId(id))
                .ok_or_else(|| mlua::Error::external(format!("no solid with id {id}")))?;
            (model.faces.len(), model.edges.len(), model.vertices.len())
        };

        let tbl = lua.create_table()?;
        tbl.set("faces", faces as i64)?;
        tbl.set("edges", edges as i64)?;
        tbl.set("vertices", vertices as i64)?;
        Ok(tbl)
    })?;
    cad.set("count", f)
}

fn register_bounds(lua: &Lua, cad: &Table, session: Arc<Mutex<Session>>) -> LuaResult<()> {
    let f = lua.create_function(move |lua, id: u32| {
        let outcome = exec_cmd(&session, Command::Bounds { id: SolidId(id) })?;
        match outcome {
            Outcome::Bounds { min, max, .. } => {
                let tbl = lua.create_table()?;
                tbl.set("min_x", min[0])?;
                tbl.set("min_y", min[1])?;
                tbl.set("min_z", min[2])?;
                tbl.set("max_x", max[0])?;
                tbl.set("max_y", max[1])?;
                tbl.set("max_z", max[2])?;
                Ok(tbl)
            }
            other => Err(mlua::Error::external(format!(
                "unexpected bounds outcome: {other:?}"
            ))),
        }
    })?;
    cad.set("bounds", f)
}

// ---------------------------------------------------------------------------
// I/O - export_stl, export_obj, import_stl
// ---------------------------------------------------------------------------

fn register_export_stl(lua: &Lua, cad: &Table, session: Arc<Mutex<Session>>) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, (id, path): (u32, String)| {
        let (model, solid) = clone_solid_brep(&session, SolidId(id))?;
        let mesh = cadkernel_io::tessellate_solid(&model, solid);
        cadkernel_io::export_stl_ascii(&mesh, Path::new(&path), "cadkernel")
            .map_err(|e| mlua::Error::external(e.to_string()))?;
        Ok(format!("exported STL to {path}"))
    })?;
    cad.set("export_stl", f)
}

fn register_export_obj(lua: &Lua, cad: &Table, session: Arc<Mutex<Session>>) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, (id, path): (u32, String)| {
        let (model, solid) = clone_solid_brep(&session, SolidId(id))?;
        let mesh = cadkernel_io::tessellate_solid(&model, solid);
        cadkernel_io::export_obj(&mesh, Path::new(&path))
            .map_err(|e| mlua::Error::external(e.to_string()))?;
        Ok(format!("exported OBJ to {path}"))
    })?;
    cad.set("export_obj", f)
}

fn register_import_stl(lua: &Lua, cad: &Table) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, _path: String| {
        // TODO(A2.10): route through Session once raw BRep import commands exist.
        Err::<i64, _>(mlua::Error::external(
            "import_stl: not yet routed through Session — A2.10 deliverable",
        ))
    })?;
    cad.set("import_stl", f)
}

// ---------------------------------------------------------------------------
// Utility - list, delete, clear
// ---------------------------------------------------------------------------

fn register_list(lua: &Lua, cad: &Table, session: Arc<Mutex<Session>>) -> LuaResult<()> {
    let f = lua.create_function(move |lua, ()| {
        let outcome = exec_cmd(&session, Command::ListSolids)?;
        match outcome {
            Outcome::SolidsListed { entries } => {
                let tbl = lua.create_table()?;
                for (idx, entry) in entries.into_iter().enumerate() {
                    tbl.set((idx + 1) as i64, entry.id.0 as i64)?;
                }
                Ok(tbl)
            }
            other => Err(mlua::Error::external(format!(
                "unexpected list outcome: {other:?}"
            ))),
        }
    })?;
    cad.set("list", f)
}

fn register_delete(lua: &Lua, cad: &Table, session: Arc<Mutex<Session>>) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, id: u32| {
        let outcome = exec_cmd(&session, Command::DeleteSolid { id: SolidId(id) })?;
        match outcome {
            Outcome::SolidDeleted { id } => Ok(format!("deleted solid {}", id.0)),
            other => Err(mlua::Error::external(format!(
                "unexpected delete outcome: {other:?}"
            ))),
        }
    })?;
    cad.set("delete", f)
}

fn register_clear(lua: &Lua, cad: &Table, session: Arc<Mutex<Session>>) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, ()| {
        let outcome = exec_cmd(&session, Command::NewDocument)?;
        match outcome {
            Outcome::DocumentReset => Ok("cleared all solids"),
            other => Err(mlua::Error::external(format!(
                "unexpected clear outcome: {other:?}"
            ))),
        }
    })?;
    cad.set("clear", f)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn err_string(result: KernelResult<String>) -> String {
        result.expect_err("script must fail").to_string()
    }

    #[test]
    fn engine_creation() {
        let engine = ScriptEngine::new();
        assert!(engine.is_ok());
    }

    #[test]
    fn primitive_box() {
        let mut engine = ScriptEngine::new().unwrap();
        let out = engine.execute("return cad.box(10, 20, 30)").unwrap();
        assert_eq!(out, "0");
        assert_eq!(engine.solid_count(), 1);
    }

    #[test]
    fn primitive_cylinder() {
        let mut engine = ScriptEngine::new().unwrap();
        let out = engine.execute("return cad.cylinder(5, 15)").unwrap();
        assert_eq!(out, "0");
        assert_eq!(engine.solid_count(), 1);
    }

    #[test]
    fn primitive_sphere() {
        let mut engine = ScriptEngine::new().unwrap();
        let out = engine.execute("return cad.sphere(8)").unwrap();
        assert_eq!(out, "0");
    }

    #[test]
    fn primitive_cone() {
        let mut engine = ScriptEngine::new().unwrap();
        let out = engine.execute("return cad.cone(5, 12)").unwrap();
        assert_eq!(out, "0");
    }

    #[test]
    fn primitive_torus() {
        let mut engine = ScriptEngine::new().unwrap();
        let out = engine.execute("return cad.torus(10, 3)").unwrap();
        assert_eq!(out, "0");
    }

    #[test]
    fn boolean_union() {
        let mut engine = ScriptEngine::new().unwrap();
        let out = engine
            .execute(
                r#"
                local a = cad.box(10, 10, 10)
                local b = cad.sphere(5)
                local c = cad.union(a, b)
                return c
            "#,
            )
            .unwrap();
        assert_eq!(out, "2");
        assert_eq!(engine.solid_count(), 1);
    }

    #[test]
    fn boolean_subtract() {
        let mut engine = ScriptEngine::new().unwrap();
        let out = engine
            .execute(
                r#"
                local a = cad.box(20, 20, 20)
                local b = cad.cylinder(3, 25)
                local c = cad.subtract(a, b)
                return c
            "#,
            )
            .unwrap();
        assert_eq!(out, "2");
    }

    #[test]
    fn boolean_intersect() {
        let mut engine = ScriptEngine::new().unwrap();
        let out = engine
            .execute(
                r#"
                local a = cad.box(10, 10, 10)
                local b = cad.box(5, 5, 5)
                local c = cad.intersect(a, b)
                return c
            "#,
            )
            .unwrap();
        assert_eq!(out, "2");
    }

    #[test]
    fn measure_box() {
        let mut engine = ScriptEngine::new().unwrap();
        let out = engine
            .execute(
                r#"
                local b = cad.box(10, 20, 30)
                local m = cad.measure(b)
                return string.format("%.0f", m.volume)
            "#,
            )
            .unwrap();
        assert_eq!(out, "6000");
    }

    #[test]
    fn count_box() {
        let mut engine = ScriptEngine::new().unwrap();
        let out = engine
            .execute(
                r#"
                local b = cad.box(10, 20, 30)
                local c = cad.count(b)
                return c.faces
            "#,
            )
            .unwrap();
        assert_eq!(out, "6");
    }

    #[test]
    fn bounds_box() {
        let mut engine = ScriptEngine::new().unwrap();
        let out = engine
            .execute(
                r#"
                local b = cad.box(10, 20, 30)
                local bounds = cad.bounds(b)
                return string.format("%.0f", bounds.max_z - bounds.min_z)
            "#,
            )
            .unwrap();
        assert_eq!(out, "30");
    }

    #[test]
    fn translate_solid() {
        let mut engine = ScriptEngine::new().unwrap();
        let out = engine
            .execute(
                r#"
                local b = cad.box(10, 10, 10)
                local t = cad.translate(b, 5, 5, 5)
                local m = cad.measure(t)
                return string.format("%.0f", m.volume)
            "#,
            )
            .unwrap();
        assert_eq!(out, "1000");
    }

    #[test]
    fn rotate_solid() {
        let mut engine = ScriptEngine::new().unwrap();
        let out = engine
            .execute(
                r#"
                local b = cad.box(10, 10, 10)
                local r = cad.rotate(b, 0, 0, 1, 45)
                return r
            "#,
            )
            .unwrap();
        assert_eq!(out, "0");
    }

    #[test]
    fn scale_solid() {
        let mut engine = ScriptEngine::new().unwrap();
        let out = engine
            .execute(
                r#"
                local b = cad.box(10, 10, 10)
                local s = cad.scale(b, 2, 2, 2)
                local m = cad.measure(s)
                return string.format("%.0f", m.volume)
            "#,
            )
            .unwrap();
        assert_eq!(out, "8000");
    }

    #[test]
    fn mirror_solid() {
        let mut engine = ScriptEngine::new().unwrap();
        let out = engine
            .execute(
                r#"
                local b = cad.box(10, 10, 10)
                local m = cad.mirror(b, 0, 0, 0, 1, 0, 0)
                return m
            "#,
            )
            .unwrap();
        assert_eq!(out, "1");
        assert_eq!(engine.solid_count(), 2);
    }

    #[test]
    fn extrude_profile() {
        let mut engine = ScriptEngine::new().unwrap();
        let out = engine
            .execute(
                r#"
                local pts = {{0,0}, {10,0}, {10,10}, {0,10}}
                local e = cad.extrude(pts, 5)
                local m = cad.measure(e)
                return string.format("%.0f", m.volume)
            "#,
            )
            .unwrap();
        assert_eq!(out, "500");
    }

    #[test]
    fn fillet_box() {
        let mut engine = ScriptEngine::new().unwrap();
        let err = err_string(engine.execute(
            r#"
                local b = cad.box(20, 20, 20)
                local f = cad.fillet(b, 2)
                return f
            "#,
        ));
        assert!(err.contains("fillet/chamfer: not yet routed through Session"));
    }

    #[test]
    fn chamfer_box() {
        let mut engine = ScriptEngine::new().unwrap();
        let err = err_string(engine.execute(
            r#"
                local b = cad.box(20, 20, 20)
                local c = cad.chamfer(b, 2)
                return c
            "#,
        ));
        assert!(err.contains("fillet/chamfer: not yet routed through Session"));
    }

    #[test]
    fn export_stl_via_script() {
        let mut engine = ScriptEngine::new().unwrap();
        let tmp = std::env::temp_dir().join("scripting_test.stl");
        let path = tmp.to_str().unwrap();
        let out = engine
            .execute(&format!(
                r#"
                local b = cad.box(10, 10, 10)
                return cad.export_stl(b, "{path}")
            "#
            ))
            .unwrap();
        assert!(out.contains("exported STL"));
        assert!(tmp.exists());
        std::fs::remove_file(tmp).ok();
    }

    #[test]
    fn export_obj_via_script() {
        let mut engine = ScriptEngine::new().unwrap();
        let tmp = std::env::temp_dir().join("scripting_test.obj");
        let path = tmp.to_str().unwrap();
        let out = engine
            .execute(&format!(
                r#"
                local b = cad.box(10, 10, 10)
                return cad.export_obj(b, "{path}")
            "#
            ))
            .unwrap();
        assert!(out.contains("exported OBJ"));
        assert!(tmp.exists());
        std::fs::remove_file(tmp).ok();
    }

    #[test]
    fn import_stl_roundtrip() {
        let mut engine = ScriptEngine::new().unwrap();
        let tmp = std::env::temp_dir().join("scripting_import_test.stl");
        let path = tmp.to_str().unwrap();

        let err = err_string(engine.execute(&format!(
            r#"
                local b = cad.box(10, 10, 10)
                cad.export_stl(b, "{path}")
                local imported = cad.import_stl("{path}")
                return imported
            "#
        )));
        assert!(err.contains("import_stl: not yet routed through Session"));
        std::fs::remove_file(tmp).ok();
    }

    #[test]
    fn list_solids() {
        let mut engine = ScriptEngine::new().unwrap();
        let out = engine
            .execute(
                r#"
                cad.box(10, 10, 10)
                cad.cylinder(5, 10)
                cad.sphere(3)
                local ids = cad.list()
                return #ids
            "#,
            )
            .unwrap();
        assert_eq!(out, "3");
    }

    #[test]
    fn delete_solid() {
        let mut engine = ScriptEngine::new().unwrap();
        let out = engine
            .execute(
                r#"
                local a = cad.box(10, 10, 10)
                local b = cad.cylinder(5, 10)
                cad.delete(a)
                local ids = cad.list()
                return #ids
            "#,
            )
            .unwrap();
        assert_eq!(out, "1");
    }

    #[test]
    fn clear_all() {
        let mut engine = ScriptEngine::new().unwrap();
        let out = engine
            .execute(
                r#"
                cad.box(10, 10, 10)
                cad.cylinder(5, 10)
                cad.sphere(3)
                cad.clear()
                local ids = cad.list()
                return #ids
            "#,
            )
            .unwrap();
        assert_eq!(out, "0");
    }

    #[test]
    fn error_invalid_id() {
        let mut engine = ScriptEngine::new().unwrap();
        let result = engine.execute("cad.measure(999)");
        assert!(result.is_err());
    }

    #[test]
    fn error_invalid_syntax() {
        let mut engine = ScriptEngine::new().unwrap();
        let result = engine.execute("this is not valid lua!!!");
        assert!(result.is_err());
    }

    #[test]
    fn error_missing_args() {
        let mut engine = ScriptEngine::new().unwrap();
        let result = engine.execute("cad.box()");
        assert!(result.is_err());
    }

    #[test]
    fn error_negative_radius() {
        let mut engine = ScriptEngine::new().unwrap();
        let result = engine.execute("cad.sphere(-5)");
        assert!(result.is_err());
    }

    #[test]
    fn error_delete_nonexistent() {
        let mut engine = ScriptEngine::new().unwrap();
        let result = engine.execute("cad.delete(42)");
        assert!(result.is_err());
    }

    #[test]
    fn multiple_operations_sequence() {
        let mut engine = ScriptEngine::new().unwrap();
        let out = engine
            .execute(
                r#"
                local b = cad.box(10, 10, 10)
                local c = cad.cylinder(5, 20)
                local u = cad.union(b, c)
                local t = cad.translate(u, 100, 0, 0)
                local s = cad.scale(t, 2, 2, 2)
                local m = cad.measure(s)
                return string.format("%.1f", m.volume)
            "#,
            )
            .unwrap();
        let vol: f64 = out.parse().unwrap();
        assert!(vol > 0.0);
    }

    #[test]
    fn get_models_returns_live() {
        let mut engine = ScriptEngine::new().unwrap();
        engine.execute("cad.box(10, 10, 10)").unwrap();
        engine.execute("cad.sphere(5)").unwrap();
        let models = engine.get_models();
        assert_eq!(models.len(), 2);
    }

    #[test]
    fn solid_count_tracks_deletes() {
        let mut engine = ScriptEngine::new().unwrap();
        engine.execute("cad.box(1,1,1)").unwrap();
        engine.execute("cad.box(2,2,2)").unwrap();
        assert_eq!(engine.solid_count(), 2);
        engine.execute("cad.delete(0)").unwrap();
        assert_eq!(engine.solid_count(), 1);
        engine.execute("cad.clear()").unwrap();
        assert_eq!(engine.solid_count(), 0);
    }
}
