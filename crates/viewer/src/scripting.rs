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

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, Handle, SolidData};
use mlua::{Lua, Result as LuaResult, Table, Value};

// ---------------------------------------------------------------------------
// Internal solid store shared between Lua callbacks
// ---------------------------------------------------------------------------

/// A model paired with the handle to a single solid inside it.
struct SolidEntry {
    model: BRepModel,
    solid: Handle<SolidData>,
}

type SolidStore = Arc<Mutex<Vec<Option<SolidEntry>>>>;

// ---------------------------------------------------------------------------
// ScriptEngine
// ---------------------------------------------------------------------------

/// Lua-based scripting engine for CADKernel.
///
/// Each engine owns an independent Lua interpreter and a flat array of
/// [`BRepModel`] solids addressable by integer ID.
pub struct ScriptEngine {
    lua: Lua,
    store: SolidStore,
}

impl ScriptEngine {
    /// Creates a new scripting engine, returning an error if Lua
    /// initialisation fails.
    pub fn new() -> KernelResult<Self> {
        let lua = Lua::new();

        // Sandbox: remove dangerous standard library globals
        lua.load(r#"
            os = nil
            io = nil
            require = nil
            dofile = nil
            loadfile = nil
            package = nil
        "#).exec().map_err(|e| KernelError::InvalidArgument(format!("lua sandbox: {e}")))?;

        let store: SolidStore = Arc::new(Mutex::new(Vec::new()));

        register_cad_table(&lua, Arc::clone(&store))
            .map_err(|e| KernelError::InvalidArgument(format!("lua init: {e}")))?;

        Ok(Self { lua, store })
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
        let guard = self.store.lock().unwrap();
        guard
            .iter()
            .filter_map(|opt| opt.as_ref().map(|e| e.model.clone()))
            .collect()
    }

    /// Returns the number of live solids.
    pub fn solid_count(&self) -> usize {
        self.store.lock().unwrap().iter().filter(|e| e.is_some()).count()
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

/// Inserts a new solid entry into the store and returns its zero-based ID.
fn store_insert(store: &SolidStore, model: BRepModel, solid: Handle<SolidData>) -> usize {
    let mut vec = store.lock().unwrap();
    // Reuse a removed slot if available.
    for (i, slot) in vec.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(SolidEntry { model, solid });
            return i;
        }
    }
    let id = vec.len();
    vec.push(Some(SolidEntry { model, solid }));
    id
}

/// Validates that `id` refers to a live solid.
fn store_get_err(store: &SolidStore, id: usize) -> LuaResult<()> {
    let vec = store.lock().unwrap();
    if id >= vec.len() || vec[id].is_none() {
        return Err(mlua::Error::external(format!("no solid with id {id}")));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Registration — `cad` table
// ---------------------------------------------------------------------------

fn register_cad_table(lua: &Lua, store: SolidStore) -> LuaResult<()> {
    let cad = lua.create_table()?;

    // ---- Primitives -------------------------------------------------------

    register_box(lua, &cad, Arc::clone(&store))?;
    register_cylinder(lua, &cad, Arc::clone(&store))?;
    register_sphere(lua, &cad, Arc::clone(&store))?;
    register_cone(lua, &cad, Arc::clone(&store))?;
    register_torus(lua, &cad, Arc::clone(&store))?;

    // ---- Booleans ---------------------------------------------------------

    register_boolean(lua, &cad, Arc::clone(&store), "union", cadkernel_modeling::BooleanOp::Union)?;
    register_boolean(lua, &cad, Arc::clone(&store), "subtract", cadkernel_modeling::BooleanOp::Difference)?;
    register_boolean(lua, &cad, Arc::clone(&store), "intersect", cadkernel_modeling::BooleanOp::Intersection)?;

    // ---- Transforms -------------------------------------------------------

    register_translate(lua, &cad, Arc::clone(&store))?;
    register_rotate(lua, &cad, Arc::clone(&store))?;
    register_scale(lua, &cad, Arc::clone(&store))?;

    // ---- Features ---------------------------------------------------------

    register_extrude(lua, &cad, Arc::clone(&store))?;
    register_fillet(lua, &cad, Arc::clone(&store))?;
    register_chamfer(lua, &cad, Arc::clone(&store))?;

    // ---- Query ------------------------------------------------------------

    register_measure(lua, &cad, Arc::clone(&store))?;
    register_count(lua, &cad, Arc::clone(&store))?;

    // ---- I/O --------------------------------------------------------------

    register_export_stl(lua, &cad, Arc::clone(&store))?;
    register_export_obj(lua, &cad, Arc::clone(&store))?;
    register_import_stl(lua, &cad, Arc::clone(&store))?;

    // ---- Utility ----------------------------------------------------------

    register_list(lua, &cad, Arc::clone(&store))?;
    register_delete(lua, &cad, Arc::clone(&store))?;
    register_clear(lua, &cad, Arc::clone(&store))?;

    lua.globals().set("cad", cad)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Primitives
// ---------------------------------------------------------------------------

fn register_box(lua: &Lua, cad: &Table, store: SolidStore) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, (w, h, d): (f64, f64, f64)| {
        let mut model = BRepModel::new();
        let result = cadkernel_modeling::make_box(&mut model, Point3::ORIGIN, w, h, d)
            .map_err(|e| mlua::Error::external(e.to_string()))?;
        let id = store_insert(&store, model, result.solid);
        Ok(id as i64)
    })?;
    cad.set("box", f)
}

fn register_cylinder(lua: &Lua, cad: &Table, store: SolidStore) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, (r, h): (f64, f64)| {
        let mut model = BRepModel::new();
        let result = cadkernel_modeling::make_cylinder(&mut model, Point3::ORIGIN, r, h, 64)
            .map_err(|e| mlua::Error::external(e.to_string()))?;
        let id = store_insert(&store, model, result.solid);
        Ok(id as i64)
    })?;
    cad.set("cylinder", f)
}

fn register_sphere(lua: &Lua, cad: &Table, store: SolidStore) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, r: f64| {
        let mut model = BRepModel::new();
        let result = cadkernel_modeling::make_sphere(&mut model, Point3::ORIGIN, r, 64, 32)
            .map_err(|e| mlua::Error::external(e.to_string()))?;
        let id = store_insert(&store, model, result.solid);
        Ok(id as i64)
    })?;
    cad.set("sphere", f)
}

fn register_cone(lua: &Lua, cad: &Table, store: SolidStore) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, (r, h): (f64, f64)| {
        let mut model = BRepModel::new();
        let result = cadkernel_modeling::make_cone(&mut model, Point3::ORIGIN, r, 0.0, h, 64)
            .map_err(|e| mlua::Error::external(e.to_string()))?;
        let id = store_insert(&store, model, result.solid);
        Ok(id as i64)
    })?;
    cad.set("cone", f)
}

fn register_torus(lua: &Lua, cad: &Table, store: SolidStore) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, (major, minor): (f64, f64)| {
        let mut model = BRepModel::new();
        let result = cadkernel_modeling::make_torus(&mut model, Point3::ORIGIN, major, minor, 64, 32)
            .map_err(|e| mlua::Error::external(e.to_string()))?;
        let id = store_insert(&store, model, result.solid);
        Ok(id as i64)
    })?;
    cad.set("torus", f)
}

// ---------------------------------------------------------------------------
// Booleans
// ---------------------------------------------------------------------------

fn register_boolean(
    lua: &Lua,
    cad: &Table,
    store: SolidStore,
    name: &str,
    op: cadkernel_modeling::BooleanOp,
) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, (a, b): (usize, usize)| {
        store_get_err(&store, a)?;
        store_get_err(&store, b)?;

        // Extract both entries temporarily.
        let (model_a, solid_a, model_b, solid_b) = {
            let vec = store.lock().unwrap();
            let ea = vec[a].as_ref().unwrap();
            let eb = vec[b].as_ref().unwrap();
            (ea.model.clone(), ea.solid, eb.model.clone(), eb.solid)
        };

        let result_model =
            cadkernel_modeling::boolean_op(&model_a, solid_a, &model_b, solid_b, op)
                .map_err(|e| mlua::Error::external(e.to_string()))?;

        // Find the first solid in the result model.
        let result_solid = result_model
            .solids
            .iter()
            .next()
            .map(|(h, _)| h)
            .ok_or_else(|| mlua::Error::external("boolean produced no solid"))?;

        let id = store_insert(&store, result_model, result_solid);
        Ok(id as i64)
    })?;
    cad.set(name, f)
}

// ---------------------------------------------------------------------------
// Transforms — translate, rotate, scale
// ---------------------------------------------------------------------------

fn register_translate(lua: &Lua, cad: &Table, store: SolidStore) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, (id, x, y, z): (usize, f64, f64, f64)| {
        store_get_err(&store, id)?;

        let (mut model, solid) = {
            let vec = store.lock().unwrap();
            let e = vec[id].as_ref().unwrap();
            (e.model.clone(), e.solid)
        };

        let transforms = [cadkernel_modeling::Transform::Translation(Vec3::new(x, y, z))];
        let result = cadkernel_modeling::multi_transform(&mut model, solid, &transforms)
            .map_err(|e| mlua::Error::external(e.to_string()))?;

        let new_id = store_insert(&store, model, result.solid);
        Ok(new_id as i64)
    })?;
    cad.set("translate", f)
}

fn register_rotate(lua: &Lua, cad: &Table, store: SolidStore) -> LuaResult<()> {
    let f =
        lua.create_function(move |_lua, (id, ax, ay, az, angle): (usize, f64, f64, f64, f64)| {
            store_get_err(&store, id)?;

            let (mut model, solid) = {
                let vec = store.lock().unwrap();
                let e = vec[id].as_ref().unwrap();
                (e.model.clone(), e.solid)
            };

            let transforms = [cadkernel_modeling::Transform::Rotation {
                axis_origin: Point3::ORIGIN,
                axis_dir: Vec3::new(ax, ay, az),
                angle: angle.to_radians(),
            }];
            let result = cadkernel_modeling::multi_transform(&mut model, solid, &transforms)
                .map_err(|e| mlua::Error::external(e.to_string()))?;

            let new_id = store_insert(&store, model, result.solid);
            Ok(new_id as i64)
        })?;
    cad.set("rotate", f)
}

fn register_scale(lua: &Lua, cad: &Table, store: SolidStore) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, (id, sx, sy, sz): (usize, f64, f64, f64)| {
        store_get_err(&store, id)?;

        let (mut model, solid) = {
            let vec = store.lock().unwrap();
            let e = vec[id].as_ref().unwrap();
            (e.model.clone(), e.solid)
        };

        // Validate uniform scale: non-uniform scaling not supported on B-Rep
        let eps = 1e-9;
        if (sx - sy).abs() > eps || (sy - sz).abs() > eps {
            return Err(mlua::Error::external(
                "non-uniform scaling not supported; sx, sy, sz must be equal",
            ));
        }
        let transforms = [cadkernel_modeling::Transform::Scale {
            center: Point3::ORIGIN,
            factor: sx,
        }];
        let result = cadkernel_modeling::multi_transform(&mut model, solid, &transforms)
            .map_err(|e| mlua::Error::external(e.to_string()))?;

        let new_id = store_insert(&store, model, result.solid);
        Ok(new_id as i64)
    })?;
    cad.set("scale", f)
}

// ---------------------------------------------------------------------------
// Features — extrude, fillet, chamfer
// ---------------------------------------------------------------------------

fn register_extrude(lua: &Lua, cad: &Table, store: SolidStore) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, (pts_table, height): (Table, f64)| {
        let mut profile = Vec::new();
        for pair in pts_table.sequence_values::<Table>() {
            let pt: Table = pair.map_err(|e| mlua::Error::external(e.to_string()))?;
            let x: f64 = pt.get(1)?;
            let y: f64 = pt.get(2)?;
            profile.push(Point3::new(x, y, 0.0));
        }
        if profile.len() < 3 {
            return Err(mlua::Error::external(
                "extrude requires at least 3 profile points",
            ));
        }

        let mut model = BRepModel::new();
        let result =
            cadkernel_modeling::extrude(&mut model, &profile, Vec3::new(0.0, 0.0, 1.0), height)
                .map_err(|e| mlua::Error::external(e.to_string()))?;

        let id = store_insert(&store, model, result.solid);
        Ok(id as i64)
    })?;
    cad.set("extrude", f)
}

fn register_fillet(lua: &Lua, cad: &Table, store: SolidStore) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, (id, radius): (usize, f64)| {
        store_get_err(&store, id)?;

        let (mut model, solid) = {
            let vec = store.lock().unwrap();
            let e = vec[id].as_ref().unwrap();
            (e.model.clone(), e.solid)
        };

        // Fillet the first edge found in the solid.
        let edge = model
            .edges
            .iter()
            .next()
            .map(|(_, e)| (e.start, e.end));

        let (v1, v2) = edge
            .ok_or_else(|| mlua::Error::external("solid has no edges for fillet"))?;

        let result = cadkernel_modeling::fillet_edge(&mut model, solid, v1, v2, radius)
            .map_err(|e| mlua::Error::external(e.to_string()))?;

        let new_id = store_insert(&store, model, result.solid);
        Ok(new_id as i64)
    })?;
    cad.set("fillet", f)
}

fn register_chamfer(lua: &Lua, cad: &Table, store: SolidStore) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, (id, dist): (usize, f64)| {
        store_get_err(&store, id)?;

        let (mut model, solid) = {
            let vec = store.lock().unwrap();
            let e = vec[id].as_ref().unwrap();
            (e.model.clone(), e.solid)
        };

        let edge = model
            .edges
            .iter()
            .next()
            .map(|(_, e)| (e.start, e.end));

        let (v1, v2) = edge
            .ok_or_else(|| mlua::Error::external("solid has no edges for chamfer"))?;

        let result = cadkernel_modeling::chamfer_edge(&mut model, solid, v1, v2, dist)
            .map_err(|e| mlua::Error::external(e.to_string()))?;

        let new_id = store_insert(&store, model, result.solid);
        Ok(new_id as i64)
    })?;
    cad.set("chamfer", f)
}

// ---------------------------------------------------------------------------
// Query — measure, count
// ---------------------------------------------------------------------------

fn register_measure(lua: &Lua, cad: &Table, store: SolidStore) -> LuaResult<()> {
    let f = lua.create_function(move |lua, id: usize| {
        store_get_err(&store, id)?;

        let (model, solid) = {
            let vec = store.lock().unwrap();
            let e = vec[id].as_ref().unwrap();
            (e.model.clone(), e.solid)
        };

        let mesh = cadkernel_io::tessellate_solid(&model, solid);
        let props = cadkernel_modeling::compute_mass_properties(&mesh);

        let tbl = lua.create_table()?;
        tbl.set("volume", props.volume)?;
        tbl.set("area", props.surface_area)?;
        tbl.set("centroid_x", props.centroid.x)?;
        tbl.set("centroid_y", props.centroid.y)?;
        tbl.set("centroid_z", props.centroid.z)?;
        Ok(tbl)
    })?;
    cad.set("measure", f)
}

fn register_count(lua: &Lua, cad: &Table, store: SolidStore) -> LuaResult<()> {
    let f = lua.create_function(move |lua, id: usize| {
        store_get_err(&store, id)?;

        let vec = store.lock().unwrap();
        let e = vec[id].as_ref().unwrap();

        let tbl = lua.create_table()?;
        tbl.set("faces", e.model.faces.len() as i64)?;
        tbl.set("edges", e.model.edges.len() as i64)?;
        tbl.set("vertices", e.model.vertices.len() as i64)?;
        Ok(tbl)
    })?;
    cad.set("count", f)
}

// ---------------------------------------------------------------------------
// I/O — export_stl, export_obj, import_stl
// ---------------------------------------------------------------------------

fn register_export_stl(lua: &Lua, cad: &Table, store: SolidStore) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, (id, path): (usize, String)| {
        store_get_err(&store, id)?;

        let (model, solid) = {
            let vec = store.lock().unwrap();
            let e = vec[id].as_ref().unwrap();
            (e.model.clone(), e.solid)
        };

        let mesh = cadkernel_io::tessellate_solid(&model, solid);
        cadkernel_io::export_stl_ascii(&mesh, Path::new(&path), "cadkernel")
            .map_err(|e| mlua::Error::external(e.to_string()))?;
        Ok(format!("exported STL to {path}"))
    })?;
    cad.set("export_stl", f)
}

fn register_export_obj(lua: &Lua, cad: &Table, store: SolidStore) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, (id, path): (usize, String)| {
        store_get_err(&store, id)?;

        let (model, solid) = {
            let vec = store.lock().unwrap();
            let e = vec[id].as_ref().unwrap();
            (e.model.clone(), e.solid)
        };

        let mesh = cadkernel_io::tessellate_solid(&model, solid);
        cadkernel_io::export_obj(&mesh, Path::new(&path))
            .map_err(|e| mlua::Error::external(e.to_string()))?;
        Ok(format!("exported OBJ to {path}"))
    })?;
    cad.set("export_obj", f)
}

fn register_import_stl(lua: &Lua, cad: &Table, store: SolidStore) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, path: String| {
        let mesh = cadkernel_io::import_stl(&path)
            .map_err(|e| mlua::Error::external(e.to_string()))?;

        // Build a trivial BRepModel containing the mesh as a single solid.
        let mut model = BRepModel::new();
        let result = cadkernel_modeling::shape_from_mesh(&mut model, &mesh)
            .map_err(|e| mlua::Error::external(e.to_string()))?;

        let id = store_insert(&store, model, result.solid);
        Ok(id as i64)
    })?;
    cad.set("import_stl", f)
}

// ---------------------------------------------------------------------------
// Utility — list, delete, clear
// ---------------------------------------------------------------------------

fn register_list(lua: &Lua, cad: &Table, store: SolidStore) -> LuaResult<()> {
    let f = lua.create_function(move |lua, ()| {
        let vec = store.lock().unwrap();
        let tbl = lua.create_table()?;
        let mut seq = 1i64;
        for (i, slot) in vec.iter().enumerate() {
            if slot.is_some() {
                tbl.set(seq, i as i64)?;
                seq += 1;
            }
        }
        Ok(tbl)
    })?;
    cad.set("list", f)
}

fn register_delete(lua: &Lua, cad: &Table, store: SolidStore) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, id: usize| {
        let mut vec = store.lock().unwrap();
        if id >= vec.len() || vec[id].is_none() {
            return Err(mlua::Error::external(format!("no solid with id {id}")));
        }
        vec[id] = None;
        Ok(format!("deleted solid {id}"))
    })?;
    cad.set("delete", f)
}

fn register_clear(lua: &Lua, cad: &Table, store: SolidStore) -> LuaResult<()> {
    let f = lua.create_function(move |_lua, ()| {
        store.lock().unwrap().clear();
        Ok("cleared all solids")
    })?;
    cad.set("clear", f)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

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
        // c should be id 2 (a=0, b=1, union=2)
        assert_eq!(out, "2");
        assert_eq!(engine.solid_count(), 3);
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
        // Volume should be preserved after translation.
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
        assert_eq!(out, "1");
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
        let out = engine
            .execute(
                r#"
                local b = cad.box(20, 20, 20)
                local f = cad.fillet(b, 2)
                return f
            "#,
            )
            .unwrap();
        assert_eq!(out, "1");
    }

    #[test]
    fn chamfer_box() {
        let mut engine = ScriptEngine::new().unwrap();
        let out = engine
            .execute(
                r#"
                local b = cad.box(20, 20, 20)
                local c = cad.chamfer(b, 2)
                return c
            "#,
            )
            .unwrap();
        assert_eq!(out, "1");
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

        // Export a box, then re-import it.
        let out = engine
            .execute(&format!(
                r#"
                local b = cad.box(10, 10, 10)
                cad.export_stl(b, "{path}")
                local imported = cad.import_stl("{path}")
                return imported
            "#
            ))
            .unwrap();
        assert_eq!(out, "1");
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
        // Result should be a valid number, not empty.
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
