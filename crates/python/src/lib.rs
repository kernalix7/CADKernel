use pyo3::prelude::*;
use pyo3::exceptions::{PyRuntimeError, PyValueError};

use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::BRepModel;
use cadkernel_io::{Mesh, tessellate_solid};
use cadkernel_modeling::{
    BooleanOp, boolean_op, compute_mass_properties,
    make_box, make_cylinder, make_sphere, make_cone, make_torus,
    make_tube, make_prism, make_wedge, make_ellipsoid, make_helix,
    extrude, revolve, mirror_solid, scale_solid,
    check_geometry,
};
use cadkernel_sketch::{Constraint, Sketch, WorkPlane, extract_profile, solve};

// ---------------------------------------------------------------------------
// Error conversion
// ---------------------------------------------------------------------------

fn to_py_err(e: impl std::fmt::Display) -> PyErr {
    PyRuntimeError::new_err(e.to_string())
}

// ---------------------------------------------------------------------------
// Handle wrapper — opaque index pair exposed to Python
// ---------------------------------------------------------------------------

#[pyclass(name = "SolidHandle")]
#[derive(Clone, Copy)]
struct PySolidHandle {
    inner: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
}

#[pymethods]
impl PySolidHandle {
    fn __repr__(&self) -> String {
        format!("SolidHandle(idx={}, gen={})", self.inner.index(), self.inner.generation())
    }
}

// ---------------------------------------------------------------------------
// Model wrapper
// ---------------------------------------------------------------------------

#[pyclass(name = "Model")]
struct PyModel {
    inner: BRepModel,
}

#[pymethods]
impl PyModel {
    #[new]
    fn new() -> Self {
        Self { inner: BRepModel::new() }
    }

    fn solid_count(&self) -> usize {
        self.inner.solids.len()
    }

    fn face_count(&self) -> usize {
        self.inner.faces.len()
    }

    fn edge_count(&self) -> usize {
        self.inner.edges.len()
    }

    fn vertex_count(&self) -> usize {
        self.inner.vertices.len()
    }

    fn __repr__(&self) -> String {
        format!(
            "Model(solids={}, faces={}, edges={}, vertices={})",
            self.inner.solids.len(),
            self.inner.faces.len(),
            self.inner.edges.len(),
            self.inner.vertices.len(),
        )
    }
}

// ---------------------------------------------------------------------------
// Mesh wrapper
// ---------------------------------------------------------------------------

#[pyclass(name = "Mesh")]
#[derive(Clone)]
struct PyMesh {
    inner: Mesh,
}

#[pymethods]
impl PyMesh {
    fn vertex_count(&self) -> usize {
        self.inner.vertices.len()
    }

    fn triangle_count(&self) -> usize {
        self.inner.triangle_count()
    }

    fn vertices(&self) -> Vec<[f64; 3]> {
        self.inner.vertices.iter().map(|p| [p.x, p.y, p.z]).collect()
    }

    fn normals(&self) -> Vec<[f64; 3]> {
        self.inner.normals.iter().map(|n| [n.x, n.y, n.z]).collect()
    }

    fn indices(&self) -> Vec<[u32; 3]> {
        self.inner.indices.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "Mesh(vertices={}, triangles={})",
            self.inner.vertices.len(),
            self.inner.triangle_count(),
        )
    }
}

// ---------------------------------------------------------------------------
// MassProperties wrapper
// ---------------------------------------------------------------------------

#[pyclass(name = "MassProperties")]
struct PyMassProperties {
    #[pyo3(get)]
    volume: f64,
    #[pyo3(get)]
    surface_area: f64,
    #[pyo3(get)]
    centroid: [f64; 3],
}

#[pymethods]
impl PyMassProperties {
    fn __repr__(&self) -> String {
        format!(
            "MassProperties(volume={:.6}, area={:.6}, centroid=[{:.4},{:.4},{:.4}])",
            self.volume, self.surface_area,
            self.centroid[0], self.centroid[1], self.centroid[2],
        )
    }
}

// ---------------------------------------------------------------------------
// GeometryCheckResult wrapper
// ---------------------------------------------------------------------------

#[pyclass(name = "GeometryCheck")]
struct PyGeometryCheck {
    #[pyo3(get)]
    is_valid: bool,
    #[pyo3(get)]
    issues: Vec<String>,
}

// ---------------------------------------------------------------------------
// Sketch wrapper
// ---------------------------------------------------------------------------

#[pyclass(name = "Sketch")]
struct PySketch {
    inner: Sketch,
}

#[pymethods]
impl PySketch {
    #[new]
    fn new() -> Self {
        Self { inner: Sketch::new() }
    }

    fn add_point(&mut self, x: f64, y: f64) -> usize {
        self.inner.add_point(x, y).0
    }

    fn add_line(&mut self, p0: usize, p1: usize) -> usize {
        use cadkernel_sketch::PointId;
        self.inner.add_line(PointId(p0), PointId(p1)).0
    }

    fn add_circle(&mut self, center: usize, radius: f64) -> usize {
        use cadkernel_sketch::PointId;
        self.inner.add_circle(PointId(center), radius).0
    }

    fn constrain_fixed(&mut self, point_id: usize, x: f64, y: f64) {
        use cadkernel_sketch::PointId;
        self.inner.add_constraint(Constraint::Fixed(PointId(point_id), x, y));
    }

    fn constrain_horizontal(&mut self, line_id: usize) {
        use cadkernel_sketch::LineId;
        self.inner.add_constraint(Constraint::Horizontal(LineId(line_id)));
    }

    fn constrain_vertical(&mut self, line_id: usize) {
        use cadkernel_sketch::LineId;
        self.inner.add_constraint(Constraint::Vertical(LineId(line_id)));
    }

    fn constrain_length(&mut self, line_id: usize, length: f64) {
        use cadkernel_sketch::LineId;
        self.inner.add_constraint(Constraint::Length(LineId(line_id), length));
    }

    fn constrain_distance(&mut self, p0: usize, p1: usize, dist: f64) {
        use cadkernel_sketch::PointId;
        self.inner.add_constraint(Constraint::Distance(PointId(p0), PointId(p1), dist));
    }

    fn constrain_coincident(&mut self, p0: usize, p1: usize) {
        use cadkernel_sketch::PointId;
        self.inner.add_constraint(Constraint::Coincident(PointId(p0), PointId(p1)));
    }

    fn constrain_radius(&mut self, center_id: usize, point_on_circle_id: usize, radius: f64) {
        use cadkernel_sketch::PointId;
        self.inner.add_constraint(Constraint::Radius(PointId(center_id), PointId(point_on_circle_id), radius));
    }

    fn solve(&mut self, max_iter: usize, tolerance: f64) -> PyResult<bool> {
        let result = solve(&mut self.inner, max_iter, tolerance);
        Ok(result.converged)
    }

    fn point_count(&self) -> usize {
        self.inner.points.len()
    }

    fn line_count(&self) -> usize {
        self.inner.lines.len()
    }

    fn __repr__(&self) -> String {
        format!(
            "Sketch(points={}, lines={}, circles={}, constraints={})",
            self.inner.points.len(),
            self.inner.lines.len(),
            self.inner.circles.len(),
            self.inner.constraints.len(),
        )
    }
}

// ---------------------------------------------------------------------------
// Free functions — Primitives
// ---------------------------------------------------------------------------

#[pyfunction]
fn create_box(model: &mut PyModel, origin: [f64; 3], w: f64, h: f64, d: f64) -> PyResult<PySolidHandle> {
    let o = Point3::new(origin[0], origin[1], origin[2]);
    let r = make_box(&mut model.inner, o, w, h, d).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

#[pyfunction]
fn create_cylinder(model: &mut PyModel, origin: [f64; 3], radius: f64, height: f64) -> PyResult<PySolidHandle> {
    let o = Point3::new(origin[0], origin[1], origin[2]);
    let r = make_cylinder(&mut model.inner, o, radius, height, 64).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

#[pyfunction]
fn create_sphere(model: &mut PyModel, origin: [f64; 3], radius: f64) -> PyResult<PySolidHandle> {
    let o = Point3::new(origin[0], origin[1], origin[2]);
    let r = make_sphere(&mut model.inner, o, radius, 64, 32).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

#[pyfunction]
fn create_cone(model: &mut PyModel, origin: [f64; 3], base_r: f64, top_r: f64, height: f64) -> PyResult<PySolidHandle> {
    let o = Point3::new(origin[0], origin[1], origin[2]);
    let r = make_cone(&mut model.inner, o, base_r, top_r, height, 64).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

#[pyfunction]
fn create_torus(model: &mut PyModel, origin: [f64; 3], major_r: f64, minor_r: f64) -> PyResult<PySolidHandle> {
    let o = Point3::new(origin[0], origin[1], origin[2]);
    let r = make_torus(&mut model.inner, o, major_r, minor_r, 64, 32).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

#[pyfunction]
fn create_tube(model: &mut PyModel, origin: [f64; 3], outer_r: f64, inner_r: f64, height: f64) -> PyResult<PySolidHandle> {
    let o = Point3::new(origin[0], origin[1], origin[2]);
    let r = make_tube(&mut model.inner, o, outer_r, inner_r, height, 64).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

#[pyfunction]
fn create_prism(model: &mut PyModel, origin: [f64; 3], radius: f64, height: f64, sides: usize) -> PyResult<PySolidHandle> {
    let o = Point3::new(origin[0], origin[1], origin[2]);
    let r = make_prism(&mut model.inner, o, radius, height, sides).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

#[pyfunction]
fn create_wedge(model: &mut PyModel, origin: [f64; 3], dx: f64, dy: f64, dz: f64, dx2: f64, dy2: f64) -> PyResult<PySolidHandle> {
    let o = Point3::new(origin[0], origin[1], origin[2]);
    let r = make_wedge(&mut model.inner, o, dx, dy, dz, dx2, dy2, 0.0, 0.0).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

#[pyfunction]
fn create_ellipsoid(model: &mut PyModel, origin: [f64; 3], rx: f64, ry: f64, rz: f64) -> PyResult<PySolidHandle> {
    let o = Point3::new(origin[0], origin[1], origin[2]);
    let r = make_ellipsoid(&mut model.inner, o, rx, ry, rz, 64, 32).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

#[pyfunction]
fn create_helix(model: &mut PyModel, origin: [f64; 3], radius: f64, pitch: f64, turns: f64, tube_r: f64) -> PyResult<PySolidHandle> {
    let o = Point3::new(origin[0], origin[1], origin[2]);
    let r = make_helix(&mut model.inner, o, radius, pitch, turns, tube_r, 16, 8).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

// ---------------------------------------------------------------------------
// Free functions — Feature operations
// ---------------------------------------------------------------------------

#[pyfunction]
fn extrude_profile(model: &mut PyModel, sketch: &PySketch, plane: &str, distance: f64) -> PyResult<PySolidHandle> {
    let wp = match plane {
        "xy" | "XY" => WorkPlane::xy(),
        "xz" | "XZ" => WorkPlane::xz(),
        "yz" | "YZ" => WorkPlane::new(Point3::ORIGIN, Vec3::Y, Vec3::Z),
        _ => return Err(PyValueError::new_err("plane must be 'xy', 'xz', or 'yz'")),
    };
    let profile = extract_profile(&sketch.inner, &wp);
    if profile.is_empty() {
        return Err(PyRuntimeError::new_err("empty profile"));
    }
    let dir = Vec3::new(wp.normal.x, wp.normal.y, wp.normal.z);
    let r = extrude(&mut model.inner, &profile, dir, distance).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

#[pyfunction]
fn revolve_profile(model: &mut PyModel, sketch: &PySketch, plane: &str, axis_origin: [f64; 3], axis_dir: [f64; 3], angle: f64, segments: usize) -> PyResult<PySolidHandle> {
    let wp = match plane {
        "xy" | "XY" => WorkPlane::xy(),
        "xz" | "XZ" => WorkPlane::xz(),
        "yz" | "YZ" => WorkPlane::new(Point3::ORIGIN, Vec3::Y, Vec3::Z),
        _ => return Err(PyValueError::new_err("plane must be 'xy', 'xz', or 'yz'")),
    };
    let profile = extract_profile(&sketch.inner, &wp);
    let o = Point3::new(axis_origin[0], axis_origin[1], axis_origin[2]);
    let d = Vec3::new(axis_dir[0], axis_dir[1], axis_dir[2]);
    let r = revolve(&mut model.inner, &profile, o, d, angle, segments).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

#[pyfunction]
fn mirror(model: &mut PyModel, solid: &PySolidHandle, point: [f64; 3], normal: [f64; 3]) -> PyResult<PySolidHandle> {
    let p = Point3::new(point[0], point[1], point[2]);
    let n = Vec3::new(normal[0], normal[1], normal[2]);
    let r = mirror_solid(&mut model.inner, solid.inner, p, n).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

#[pyfunction]
fn scale(model: &mut PyModel, solid: &PySolidHandle, center: [f64; 3], factor: f64) -> PyResult<PySolidHandle> {
    let c = Point3::new(center[0], center[1], center[2]);
    let r = scale_solid(&mut model.inner, solid.inner, c, factor).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

// ---------------------------------------------------------------------------
// Free functions — Boolean operations
// ---------------------------------------------------------------------------

#[pyfunction]
fn boolean_union(model_a: &PyModel, solid_a: &PySolidHandle, model_b: &PyModel, solid_b: &PySolidHandle) -> PyResult<PyModel> {
    let result = boolean_op(&model_a.inner, solid_a.inner, &model_b.inner, solid_b.inner, BooleanOp::Union).map_err(to_py_err)?;
    Ok(PyModel { inner: result })
}

#[pyfunction]
fn boolean_subtract(model_a: &PyModel, solid_a: &PySolidHandle, model_b: &PyModel, solid_b: &PySolidHandle) -> PyResult<PyModel> {
    let result = boolean_op(&model_a.inner, solid_a.inner, &model_b.inner, solid_b.inner, BooleanOp::Difference).map_err(to_py_err)?;
    Ok(PyModel { inner: result })
}

#[pyfunction]
fn boolean_intersect(model_a: &PyModel, solid_a: &PySolidHandle, model_b: &PyModel, solid_b: &PySolidHandle) -> PyResult<PyModel> {
    let result = boolean_op(&model_a.inner, solid_a.inner, &model_b.inner, solid_b.inner, BooleanOp::Intersection).map_err(to_py_err)?;
    Ok(PyModel { inner: result })
}

// ---------------------------------------------------------------------------
// Free functions — Tessellation & Measurement
// ---------------------------------------------------------------------------

#[pyfunction]
fn tessellate(model: &PyModel, solid: &PySolidHandle) -> PyMesh {
    let mesh = tessellate_solid(&model.inner, solid.inner);
    PyMesh { inner: mesh }
}

#[pyfunction]
fn mass_properties(mesh: &PyMesh) -> PyMassProperties {
    let props = compute_mass_properties(&mesh.inner);
    PyMassProperties {
        volume: props.volume,
        surface_area: props.surface_area,
        centroid: [props.centroid.x, props.centroid.y, props.centroid.z],
    }
}

#[pyfunction]
fn geometry_check(model: &PyModel, solid: &PySolidHandle) -> PyGeometryCheck {
    let result = check_geometry(&model.inner, solid.inner);
    PyGeometryCheck {
        is_valid: result.is_valid,
        issues: result.issues,
    }
}

// ---------------------------------------------------------------------------
// Free functions — I/O
// ---------------------------------------------------------------------------

#[pyfunction]
fn export_stl(path: &str, mesh: &PyMesh) -> PyResult<()> {
    let content = cadkernel_io::write_stl_ascii(&mesh.inner, "CADKernel");
    std::fs::write(path, content).map_err(to_py_err)
}

#[pyfunction]
fn export_stl_binary(path: &str, mesh: &PyMesh) -> PyResult<()> {
    let bytes = cadkernel_io::write_stl_binary(&mesh.inner).map_err(to_py_err)?;
    std::fs::write(path, bytes).map_err(to_py_err)
}

#[pyfunction]
fn export_obj(path: &str, mesh: &PyMesh) -> PyResult<()> {
    let content = cadkernel_io::write_obj(&mesh.inner);
    std::fs::write(path, content).map_err(to_py_err)
}

#[pyfunction]
fn export_gltf(path: &str, mesh: &PyMesh) -> PyResult<()> {
    cadkernel_io::export_gltf(&mesh.inner, path).map_err(to_py_err)
}

#[pyfunction]
fn export_step(path: &str, model: &PyModel) -> PyResult<()> {
    let content = cadkernel_io::export_step(&model.inner).map_err(to_py_err)?;
    std::fs::write(path, content).map_err(to_py_err)
}

#[pyfunction]
fn export_iges(path: &str, model: &PyModel) -> PyResult<()> {
    let content = cadkernel_io::export_iges(&model.inner).map_err(to_py_err)?;
    std::fs::write(path, content).map_err(to_py_err)
}

#[pyfunction]
fn import_stl(path: &str) -> PyResult<PyMesh> {
    let mesh = cadkernel_io::import_stl(path).map_err(to_py_err)?;
    Ok(PyMesh { inner: mesh })
}

#[pyfunction]
fn import_obj(path: &str) -> PyResult<PyMesh> {
    let mesh = cadkernel_io::import_obj(path).map_err(to_py_err)?;
    Ok(PyMesh { inner: mesh })
}

#[pyfunction]
fn save_project(path: &str, model: &PyModel) -> PyResult<()> {
    cadkernel_io::save_project(&model.inner, path).map_err(to_py_err)
}

#[pyfunction]
fn load_project(path: &str) -> PyResult<PyModel> {
    let model = cadkernel_io::load_project(path).map_err(to_py_err)?;
    Ok(PyModel { inner: model })
}

// ---------------------------------------------------------------------------
// Python module
// ---------------------------------------------------------------------------

#[pymodule]
fn cadkernel(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Classes
    m.add_class::<PyModel>()?;
    m.add_class::<PySolidHandle>()?;
    m.add_class::<PyMesh>()?;
    m.add_class::<PyMassProperties>()?;
    m.add_class::<PyGeometryCheck>()?;
    m.add_class::<PySketch>()?;

    // Primitives
    m.add_function(wrap_pyfunction!(create_box, m)?)?;
    m.add_function(wrap_pyfunction!(create_cylinder, m)?)?;
    m.add_function(wrap_pyfunction!(create_sphere, m)?)?;
    m.add_function(wrap_pyfunction!(create_cone, m)?)?;
    m.add_function(wrap_pyfunction!(create_torus, m)?)?;
    m.add_function(wrap_pyfunction!(create_tube, m)?)?;
    m.add_function(wrap_pyfunction!(create_prism, m)?)?;
    m.add_function(wrap_pyfunction!(create_wedge, m)?)?;
    m.add_function(wrap_pyfunction!(create_ellipsoid, m)?)?;
    m.add_function(wrap_pyfunction!(create_helix, m)?)?;

    // Feature operations
    m.add_function(wrap_pyfunction!(extrude_profile, m)?)?;
    m.add_function(wrap_pyfunction!(revolve_profile, m)?)?;
    m.add_function(wrap_pyfunction!(mirror, m)?)?;
    m.add_function(wrap_pyfunction!(scale, m)?)?;

    // Boolean operations
    m.add_function(wrap_pyfunction!(boolean_union, m)?)?;
    m.add_function(wrap_pyfunction!(boolean_subtract, m)?)?;
    m.add_function(wrap_pyfunction!(boolean_intersect, m)?)?;

    // Tessellation & measurement
    m.add_function(wrap_pyfunction!(tessellate, m)?)?;
    m.add_function(wrap_pyfunction!(mass_properties, m)?)?;
    m.add_function(wrap_pyfunction!(geometry_check, m)?)?;

    // I/O
    m.add_function(wrap_pyfunction!(export_stl, m)?)?;
    m.add_function(wrap_pyfunction!(export_stl_binary, m)?)?;
    m.add_function(wrap_pyfunction!(export_obj, m)?)?;
    m.add_function(wrap_pyfunction!(export_gltf, m)?)?;
    m.add_function(wrap_pyfunction!(export_step, m)?)?;
    m.add_function(wrap_pyfunction!(export_iges, m)?)?;
    m.add_function(wrap_pyfunction!(import_stl, m)?)?;
    m.add_function(wrap_pyfunction!(import_obj, m)?)?;
    m.add_function(wrap_pyfunction!(save_project, m)?)?;
    m.add_function(wrap_pyfunction!(load_project, m)?)?;

    Ok(())
}
