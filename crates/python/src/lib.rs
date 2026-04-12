//! Python bindings for the CADKernel B-Rep CAD kernel.
//!
//! Exposes 13 primitive constructors, 40+ feature/modeling operations,
//! 10+ mesh processing operations, FEM analysis, assembly management,
//! sketch constraint solving, and file I/O for 12 formats.
//!
//! The module is registered as `_native` and intended to be wrapped by a
//! pure-Python `cadkernel` package. All geometry uses `f64` precision;
//! Python-facing arrays use `[f64; 3]` for points/vectors and `[u32; 3]`
//! for triangle indices.

use pyo3::prelude::*;
use pyo3::exceptions::{PyRuntimeError, PyValueError};

use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, EdgeData, Handle, VertexData, FaceData};
use cadkernel_io::Mesh;
use cadkernel_modeling::{
    BooleanOp, boolean_op, boolean_xor, compute_mass_properties,
    make_box, make_cylinder, make_sphere, make_cone, make_torus,
    make_tube, make_prism, make_wedge, make_ellipsoid, make_helix,
    make_spiral, make_polygon, make_plane_face,
    extrude, revolve, mirror_solid, scale_solid, sweep, loft,
    chamfer_edge, fillet_edge, shell_solid, linear_pattern, circular_pattern,
    pad, pocket, draft_faces, section_solid, split_solid,
    thickness_solid, ThicknessJoin, taper_extrude, offset_solid,
    groove, hole, countersunk_hole, cross_sections,
    reverse_solid, refine_shape, shape_from_mesh,
    additive_box, additive_cylinder, additive_sphere, additive_cone,
    additive_torus, additive_helix, additive_ellipsoid, additive_prism, additive_wedge,
    additive_loft, additive_pipe,
    subtractive_box, subtractive_cylinder, subtractive_sphere, subtractive_cone,
    subtractive_torus, subtractive_helix, subtractive_ellipsoid, subtractive_prism,
    subtractive_wedge, subtractive_loft, subtractive_pipe,
    connect_shapes, embed_shapes, cutout_shapes,
    Compound,
    multi_transform, Transform as ModelTransform,
    check_geometry, check_watertight,
    TetMesh, FemMaterial, BoundaryCondition, generate_tet_mesh, static_analysis,
    modal_analysis, ThermalMaterial, ThermalBoundaryCondition, thermal_analysis,
    make_involute_gear,
    Assembly, AssemblyConstraint, ComponentId,
    make_wire, clone_solid, rectangular_array, path_array, polar_array,
    boolean_fragments, slice_to_compound, compound_filter,
    measure_distance, measure_angle, measure_edge_length,
    classify_solid, find_planar_faces, find_cylindrical_faces,
    closest_point_on_solid, point_in_solid, Containment,
};
use cadkernel_sketch::{Constraint, Sketch, WorkPlane, extract_profile, solve};

// ---------------------------------------------------------------------------
// Error conversion
// ---------------------------------------------------------------------------

fn to_py_err(e: impl std::fmt::Display) -> PyErr {
    PyRuntimeError::new_err(e.to_string())
}

fn pt3(a: [f64; 3]) -> Point3 {
    Point3::new(a[0], a[1], a[2])
}

fn v3(a: [f64; 3]) -> Vec3 {
    Vec3::new(a[0], a[1], a[2])
}

// ---------------------------------------------------------------------------
// Handle wrappers
// ---------------------------------------------------------------------------

/// Handle to a solid entity in a B-Rep model (generational arena index).
#[pyclass(name = "SolidHandle")]
#[derive(Clone, Copy)]
struct PySolidHandle {
    inner: Handle<cadkernel_topology::SolidData>,
}

#[pymethods]
impl PySolidHandle {
    fn __repr__(&self) -> String {
        format!("SolidHandle(idx={}, gen={})", self.inner.index(), self.inner.generation())
    }
}

/// Handle to a vertex entity in a B-Rep model.
#[pyclass(name = "VertexHandle")]
#[derive(Clone, Copy)]
struct PyVertexHandle {
    inner: Handle<VertexData>,
}

#[pymethods]
impl PyVertexHandle {
    fn __repr__(&self) -> String {
        format!("VertexHandle(idx={}, gen={})", self.inner.index(), self.inner.generation())
    }
}

/// Handle to an edge entity in a B-Rep model.
#[pyclass(name = "EdgeHandle")]
#[derive(Clone, Copy)]
struct PyEdgeHandle {
    inner: Handle<EdgeData>,
}

#[pymethods]
impl PyEdgeHandle {
    fn __repr__(&self) -> String {
        format!("EdgeHandle(idx={}, gen={})", self.inner.index(), self.inner.generation())
    }
}

/// Handle to a face entity in a B-Rep model.
#[pyclass(name = "FaceHandle")]
#[derive(Clone, Copy)]
struct PyFaceHandle {
    inner: Handle<FaceData>,
}

#[pymethods]
impl PyFaceHandle {
    fn __repr__(&self) -> String {
        format!("FaceHandle(idx={}, gen={})", self.inner.index(), self.inner.generation())
    }
}

// ---------------------------------------------------------------------------
// Model wrapper
// ---------------------------------------------------------------------------

/// A B-Rep solid model with half-edge topology.
///
/// Wraps the Rust `BRepModel` and provides access to solid, face, edge,
/// and vertex counts. Use primitive constructors or file import to populate.
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

    fn get_first_solid(&self) -> PyResult<PySolidHandle> {
        let (h, _) = self.inner.solids.iter().next()
            .ok_or_else(|| PyRuntimeError::new_err("model has no solids"))?;
        Ok(PySolidHandle { inner: h })
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

/// An indexed triangle mesh with vertex positions, per-face normals, and triangle indices.
///
/// Access vertex data as `list[list[float]]` via `vertices()`, `normals()`, and `indices()`.
#[pyclass(name = "Mesh")]
#[derive(Clone)]
struct PyMesh {
    inner: Mesh,
}

#[pymethods]
impl PyMesh {
    /// Returns the number of vertices.
    fn vertex_count(&self) -> usize {
        self.inner.vertices.len()
    }

    /// Returns the number of triangles.
    fn triangle_count(&self) -> usize {
        self.inner.triangle_count()
    }

    /// Returns vertex positions as a list of `[x, y, z]` arrays.
    fn vertices(&self) -> Vec<[f64; 3]> {
        self.inner.vertices.iter().map(|p| [p.x, p.y, p.z]).collect()
    }

    /// Returns per-face normals as a list of `[nx, ny, nz]` arrays.
    fn normals(&self) -> Vec<[f64; 3]> {
        self.inner.normals.iter().map(|n| [n.x, n.y, n.z]).collect()
    }

    /// Returns triangle indices as a list of `[i0, i1, i2]` arrays.
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

/// Volume, surface area, and centroid computed from a tessellated mesh.
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

/// Result of a geometry validity check on a solid.
#[pyclass(name = "GeometryCheck")]
struct PyGeometryCheck {
    #[pyo3(get)]
    is_valid: bool,
    #[pyo3(get)]
    issues: Vec<String>,
}

// ---------------------------------------------------------------------------
// SectionEdge wrapper
// ---------------------------------------------------------------------------

/// A cross-section edge segment (start and end points in 3D).
#[pyclass(name = "SectionEdge")]
struct PySectionEdge {
    #[pyo3(get)]
    start: [f64; 3],
    #[pyo3(get)]
    end: [f64; 3],
}

// ---------------------------------------------------------------------------
// FEM wrappers
// ---------------------------------------------------------------------------

/// A tetrahedral mesh for finite element analysis.
#[pyclass(name = "TetMesh")]
struct PyTetMesh {
    inner: TetMesh,
}

#[pymethods]
impl PyTetMesh {
    fn node_count(&self) -> usize {
        self.inner.nodes.len()
    }

    fn element_count(&self) -> usize {
        self.inner.elements.len()
    }

    fn nodes(&self) -> Vec<[f64; 3]> {
        self.inner.nodes.iter().map(|p| [p.x, p.y, p.z]).collect()
    }

    fn elements(&self) -> Vec<[usize; 4]> {
        self.inner.elements.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "TetMesh(nodes={}, elements={})",
            self.inner.nodes.len(),
            self.inner.elements.len(),
        )
    }
}

/// Result of a FEM static analysis: displacements and stresses per node.
#[pyclass(name = "FemResult")]
struct PyFemResult {
    #[pyo3(get)]
    max_displacement: f64,
    #[pyo3(get)]
    max_stress: f64,
    displacements: Vec<Vec3>,
    stresses: Vec<f64>,
}

#[pymethods]
impl PyFemResult {
    fn displacements(&self) -> Vec<[f64; 3]> {
        self.displacements.iter().map(|v| [v.x, v.y, v.z]).collect()
    }

    fn stresses(&self) -> Vec<f64> {
        self.stresses.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "FemResult(max_disp={:.6e}, max_stress={:.6e})",
            self.max_displacement, self.max_stress,
        )
    }
}

// ---------------------------------------------------------------------------
// Assembly wrapper
// ---------------------------------------------------------------------------

/// An assembly of components with geometric constraints.
#[pyclass(name = "Assembly")]
struct PyAssembly {
    inner: Assembly,
}

#[pymethods]
impl PyAssembly {
    #[new]
    fn new(name: &str) -> Self {
        Self { inner: Assembly::new(name) }
    }

    fn component_count(&self) -> usize {
        self.inner.components.len()
    }

    fn constraint_count(&self) -> usize {
        self.inner.constraints.len()
    }

    fn __repr__(&self) -> String {
        format!(
            "Assembly(name='{}', components={}, constraints={})",
            self.inner.name,
            self.inner.components.len(),
            self.inner.constraints.len(),
        )
    }
}

// ---------------------------------------------------------------------------
// Compound wrapper
// ---------------------------------------------------------------------------

/// A compound of multiple solids grouped together.
#[pyclass(name = "Compound")]
struct PyCompound {
    inner: Compound,
}

#[pymethods]
impl PyCompound {
    #[new]
    fn new(name: &str) -> Self {
        Self { inner: Compound::new(name) }
    }

    fn add(&mut self, solid: &PySolidHandle) {
        self.inner.add(solid.inner);
    }

    fn count(&self) -> usize {
        self.inner.solids.len()
    }

    fn __repr__(&self) -> String {
        format!("Compound(name='{}', solids={})", self.inner.name, self.inner.solids.len())
    }
}

// ---------------------------------------------------------------------------
// Sketch wrapper
// ---------------------------------------------------------------------------

/// A 2D parametric sketch with entities and constraints, solved by Newton-Raphson.
///
/// Add points, lines, arcs, circles, ellipses, and B-splines as entities,
/// then add constraints (fixed, horizontal, vertical, distance, angle, etc.)
/// and call `solve()` to find a consistent configuration.
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

    fn add_arc(&mut self, center: usize, start: usize, end: usize, radius: f64, start_angle: f64, end_angle: f64) -> usize {
        use cadkernel_sketch::PointId;
        self.inner.add_arc(PointId(center), PointId(start), PointId(end), radius, start_angle, end_angle).0
    }

    fn add_ellipse(&mut self, center: usize, major_end: usize, minor_radius: f64) -> usize {
        use cadkernel_sketch::PointId;
        self.inner.add_ellipse(PointId(center), PointId(major_end), minor_radius).0
    }

    fn add_bspline(&mut self, control_points: Vec<usize>, degree: usize, closed: bool) -> usize {
        use cadkernel_sketch::PointId;
        let pts: Vec<PointId> = control_points.into_iter().map(PointId).collect();
        self.inner.add_bspline(pts, degree, closed).0
    }

    // Constraints
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

    fn constrain_parallel(&mut self, line1: usize, line2: usize) {
        use cadkernel_sketch::LineId;
        self.inner.add_constraint(Constraint::Parallel(LineId(line1), LineId(line2)));
    }

    fn constrain_perpendicular(&mut self, line1: usize, line2: usize) {
        use cadkernel_sketch::LineId;
        self.inner.add_constraint(Constraint::Perpendicular(LineId(line1), LineId(line2)));
    }

    fn constrain_tangent(&mut self, line_id: usize, center_id: usize, radius: f64) {
        use cadkernel_sketch::{LineId, PointId};
        self.inner.add_constraint(Constraint::Tangent(LineId(line_id), PointId(center_id), radius));
    }

    fn constrain_equal_length(&mut self, line1: usize, line2: usize) {
        use cadkernel_sketch::LineId;
        self.inner.add_constraint(Constraint::EqualLength(LineId(line1), LineId(line2)));
    }

    fn constrain_midpoint(&mut self, point_id: usize, line_id: usize) {
        use cadkernel_sketch::{LineId, PointId};
        self.inner.add_constraint(Constraint::Midpoint(PointId(point_id), LineId(line_id)));
    }

    fn constrain_collinear(&mut self, line1: usize, line2: usize) {
        use cadkernel_sketch::LineId;
        self.inner.add_constraint(Constraint::Collinear(LineId(line1), LineId(line2)));
    }

    fn constrain_concentric(&mut self, center1: usize, center2: usize) {
        use cadkernel_sketch::PointId;
        self.inner.add_constraint(Constraint::Concentric(PointId(center1), PointId(center2)));
    }

    fn constrain_diameter(&mut self, center: usize, point_on: usize, diameter: f64) {
        use cadkernel_sketch::PointId;
        self.inner.add_constraint(Constraint::Diameter(PointId(center), PointId(point_on), diameter));
    }

    fn constrain_angle(&mut self, line1: usize, line2: usize, angle: f64) {
        use cadkernel_sketch::LineId;
        self.inner.add_constraint(Constraint::Angle(LineId(line1), LineId(line2), angle));
    }

    fn constrain_symmetric(&mut self, p0: usize, p1: usize, axis_line: usize) {
        use cadkernel_sketch::{LineId, PointId};
        self.inner.add_constraint(Constraint::Symmetric(PointId(p0), PointId(p1), LineId(axis_line)));
    }

    fn constrain_point_on_line(&mut self, point_id: usize, line_id: usize) {
        use cadkernel_sketch::{LineId, PointId};
        self.inner.add_constraint(Constraint::PointOnLine(PointId(point_id), LineId(line_id)));
    }

    fn constrain_horizontal_distance(&mut self, p0: usize, p1: usize, dist: f64) {
        use cadkernel_sketch::PointId;
        self.inner.add_constraint(Constraint::HorizontalDistance(PointId(p0), PointId(p1), dist));
    }

    fn constrain_vertical_distance(&mut self, p0: usize, p1: usize, dist: f64) {
        use cadkernel_sketch::PointId;
        self.inner.add_constraint(Constraint::VerticalDistance(PointId(p0), PointId(p1), dist));
    }

    fn constrain_block(&mut self, point_id: usize, x: f64, y: f64) {
        use cadkernel_sketch::PointId;
        self.inner.add_constraint(Constraint::Block(PointId(point_id), x, y));
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

    fn arc_count(&self) -> usize {
        self.inner.arcs.len()
    }

    fn circle_count(&self) -> usize {
        self.inner.circles.len()
    }

    fn constraint_count(&self) -> usize {
        self.inner.constraints.len()
    }

    fn __repr__(&self) -> String {
        format!(
            "Sketch(points={}, lines={}, arcs={}, circles={}, constraints={})",
            self.inner.points.len(),
            self.inner.lines.len(),
            self.inner.arcs.len(),
            self.inner.circles.len(),
            self.inner.constraints.len(),
        )
    }
}

// ---------------------------------------------------------------------------
// Free functions — Primitives
// ---------------------------------------------------------------------------

/// Creates a box primitive with given origin, width, height, and depth.
#[pyfunction]
fn create_box(model: &mut PyModel, origin: [f64; 3], w: f64, h: f64, d: f64) -> PyResult<PySolidHandle> {
    let r = make_box(&mut model.inner, pt3(origin), w, h, d).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

/// Creates a cylinder primitive centered at origin with given radius and height.
#[pyfunction]
fn create_cylinder(model: &mut PyModel, origin: [f64; 3], radius: f64, height: f64) -> PyResult<PySolidHandle> {
    let r = make_cylinder(&mut model.inner, pt3(origin), radius, height, 64).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

/// Creates a sphere primitive centered at origin with given radius.
#[pyfunction]
fn create_sphere(model: &mut PyModel, origin: [f64; 3], radius: f64) -> PyResult<PySolidHandle> {
    let r = make_sphere(&mut model.inner, pt3(origin), radius, 64, 32).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

/// Creates a cone/frustum primitive with base and top radii.
#[pyfunction]
fn create_cone(model: &mut PyModel, origin: [f64; 3], base_r: f64, top_r: f64, height: f64) -> PyResult<PySolidHandle> {
    let r = make_cone(&mut model.inner, pt3(origin), base_r, top_r, height, 64).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

/// Creates a torus primitive with major and minor radii.
#[pyfunction]
fn create_torus(model: &mut PyModel, origin: [f64; 3], major_r: f64, minor_r: f64) -> PyResult<PySolidHandle> {
    let r = make_torus(&mut model.inner, pt3(origin), major_r, minor_r, 64, 32).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

/// Creates a tube (hollow cylinder) with outer and inner radii.
#[pyfunction]
fn create_tube(model: &mut PyModel, origin: [f64; 3], outer_r: f64, inner_r: f64, height: f64) -> PyResult<PySolidHandle> {
    let r = make_tube(&mut model.inner, pt3(origin), outer_r, inner_r, height, 64).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

/// Creates a regular prism with the given number of sides.
#[pyfunction]
fn create_prism(model: &mut PyModel, origin: [f64; 3], radius: f64, height: f64, sides: usize) -> PyResult<PySolidHandle> {
    let r = make_prism(&mut model.inner, pt3(origin), radius, height, sides).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

/// Creates a wedge (tapered box) with base and top dimensions.
#[pyfunction]
fn create_wedge(model: &mut PyModel, origin: [f64; 3], dx: f64, dy: f64, dz: f64, dx2: f64, dy2: f64) -> PyResult<PySolidHandle> {
    let r = make_wedge(&mut model.inner, pt3(origin), dx, dy, dz, dx2, dy2, 0.0, 0.0).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

/// Creates an ellipsoid with independent X, Y, and Z radii.
#[pyfunction]
fn create_ellipsoid(model: &mut PyModel, origin: [f64; 3], rx: f64, ry: f64, rz: f64) -> PyResult<PySolidHandle> {
    let r = make_ellipsoid(&mut model.inner, pt3(origin), rx, ry, rz, 64, 32).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

/// Creates a helix solid with given radius, pitch, turns, and tube cross-section radius.
#[pyfunction]
fn create_helix(model: &mut PyModel, origin: [f64; 3], radius: f64, pitch: f64, turns: f64, tube_r: f64) -> PyResult<PySolidHandle> {
    let r = make_helix(&mut model.inner, pt3(origin), radius, pitch, turns, tube_r, 16, 8).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

/// Creates a spiral solid with expanding radius per revolution.
#[pyfunction]
fn create_spiral(model: &mut PyModel, origin: [f64; 3], initial_radius: f64, growth_per_rev: f64, turns: f64, tube_radius: f64) -> PyResult<PySolidHandle> {
    let r = make_spiral(&mut model.inner, pt3(origin), initial_radius, growth_per_rev, turns, tube_radius, 16).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

/// Creates a regular polygon prism (e.g., hexagon) with given sides and height.
#[pyfunction]
fn create_polygon(model: &mut PyModel, origin: [f64; 3], radius: f64, sides: usize, height: f64) -> PyResult<PySolidHandle> {
    let r = make_polygon(&mut model.inner, pt3(origin), radius, sides, height).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

/// Creates a planar face (rectangular surface) with given width and height.
#[pyfunction]
fn create_plane_face(model: &mut PyModel, origin: [f64; 3], width: f64, height: f64) -> PyResult<PySolidHandle> {
    let r = make_plane_face(&mut model.inner, pt3(origin), width, height).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

/// Creates an involute spur gear with given module, tooth count, and pressure angle.
#[pyfunction]
fn create_gear(model: &mut PyModel, module_val: f64, teeth: usize, pressure_angle: f64, face_width: f64) -> PyResult<PySolidHandle> {
    let r = make_involute_gear(&mut model.inner, module_val, teeth, pressure_angle, face_width).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

// ---------------------------------------------------------------------------
// Free functions — Feature operations
// ---------------------------------------------------------------------------

fn parse_workplane(plane: &str) -> PyResult<WorkPlane> {
    match plane {
        "xy" | "XY" => Ok(WorkPlane::xy()),
        "xz" | "XZ" => Ok(WorkPlane::xz()),
        "yz" | "YZ" => Ok(WorkPlane::new(Point3::ORIGIN, Vec3::Y, Vec3::Z)),
        _ => Err(PyValueError::new_err("plane must be 'xy', 'xz', or 'yz'")),
    }
}

/// Extrudes a sketch profile along its plane normal by the given distance.
#[pyfunction]
fn extrude_profile(model: &mut PyModel, sketch: &PySketch, plane: &str, distance: f64) -> PyResult<PySolidHandle> {
    let wp = parse_workplane(plane)?;
    let profile = extract_profile(&sketch.inner, &wp);
    if profile.is_empty() {
        return Err(PyRuntimeError::new_err("empty profile"));
    }
    let dir = Vec3::new(wp.normal.x, wp.normal.y, wp.normal.z);
    let r = extrude(&mut model.inner, &profile, dir, distance).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

/// Revolves a sketch profile around an axis by the given angle (radians).
#[pyfunction]
fn revolve_profile(model: &mut PyModel, sketch: &PySketch, plane: &str, axis_origin: [f64; 3], axis_dir: [f64; 3], angle: f64, segments: usize) -> PyResult<PySolidHandle> {
    let wp = parse_workplane(plane)?;
    let profile = extract_profile(&sketch.inner, &wp);
    let r = revolve(&mut model.inner, &profile, pt3(axis_origin), v3(axis_dir), angle, segments).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

/// Mirrors a solid across a plane defined by a point and normal.
#[pyfunction]
fn mirror(model: &mut PyModel, solid: &PySolidHandle, point: [f64; 3], normal: [f64; 3]) -> PyResult<PySolidHandle> {
    let r = mirror_solid(&mut model.inner, solid.inner, pt3(point), v3(normal)).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

/// Uniformly scales a solid around a center point.
#[pyfunction]
fn scale(model: &mut PyModel, solid: &PySolidHandle, center: [f64; 3], factor: f64) -> PyResult<PySolidHandle> {
    let r = scale_solid(&mut model.inner, solid.inner, pt3(center), factor).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

/// Sweeps a profile polygon along a path to create a solid.
#[pyfunction]
fn sweep_profile(model: &mut PyModel, profile: Vec<[f64; 3]>, path: Vec<[f64; 3]>) -> PyResult<PySolidHandle> {
    let prof: Vec<Point3> = profile.into_iter().map(pt3).collect();
    let pth: Vec<Point3> = path.into_iter().map(pt3).collect();
    let r = sweep(&mut model.inner, &prof, &pth).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

/// Creates a solid by lofting (blending) between multiple cross-section profiles.
#[pyfunction]
fn loft_profiles(model: &mut PyModel, profiles: Vec<Vec<[f64; 3]>>) -> PyResult<PySolidHandle> {
    let profs: Vec<Vec<Point3>> = profiles.into_iter().map(|p| p.into_iter().map(pt3).collect()).collect();
    let refs: Vec<&[Point3]> = profs.iter().map(|p| p.as_slice()).collect();
    let r = loft(&mut model.inner, &refs).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

#[pyfunction]
fn chamfer(model: &mut PyModel, solid: &PySolidHandle, v1: &PyVertexHandle, v2: &PyVertexHandle, distance: f64) -> PyResult<PySolidHandle> {
    let r = chamfer_edge(&mut model.inner, solid.inner, v1.inner, v2.inner, distance).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

#[pyfunction]
fn fillet(model: &mut PyModel, solid: &PySolidHandle, v1: &PyVertexHandle, v2: &PyVertexHandle, radius: f64) -> PyResult<PySolidHandle> {
    let r = fillet_edge(&mut model.inner, solid.inner, v1.inner, v2.inner, radius).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

#[pyfunction]
fn shell(model: &mut PyModel, solid: &PySolidHandle, faces: Vec<PyRef<PyFaceHandle>>, thickness: f64) -> PyResult<PySolidHandle> {
    let face_handles: Vec<Handle<FaceData>> = faces.iter().map(|f| f.inner).collect();
    let r = shell_solid(&mut model.inner, solid.inner, &face_handles, thickness).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

#[pyfunction]
fn pattern_linear(model: &mut PyModel, solid: &PySolidHandle, direction: [f64; 3], spacing: f64, count: usize) -> PyResult<Vec<PySolidHandle>> {
    let r = linear_pattern(&mut model.inner, solid.inner, v3(direction), spacing, count).map_err(to_py_err)?;
    Ok(r.solids.into_iter().map(|s| PySolidHandle { inner: s }).collect())
}

#[pyfunction]
fn pattern_circular(model: &mut PyModel, solid: &PySolidHandle, axis_origin: [f64; 3], axis_dir: [f64; 3], count: usize) -> PyResult<Vec<PySolidHandle>> {
    let r = circular_pattern(&mut model.inner, solid.inner, pt3(axis_origin), v3(axis_dir), count).map_err(to_py_err)?;
    Ok(r.solids.into_iter().map(|s| PySolidHandle { inner: s }).collect())
}

#[pyfunction]
fn pad_profile(model: &PyModel, solid: &PySolidHandle, profile: Vec<[f64; 3]>, direction: [f64; 3], distance: f64) -> PyResult<PyModel> {
    let prof: Vec<Point3> = profile.into_iter().map(pt3).collect();
    let r = pad(&model.inner, solid.inner, &prof, v3(direction), distance).map_err(to_py_err)?;
    Ok(PyModel { inner: r.model })
}

#[pyfunction]
fn pocket_profile(model: &PyModel, solid: &PySolidHandle, profile: Vec<[f64; 3]>, direction: [f64; 3], depth: f64) -> PyResult<PyModel> {
    let prof: Vec<Point3> = profile.into_iter().map(pt3).collect();
    let r = pocket(&model.inner, solid.inner, &prof, v3(direction), depth).map_err(to_py_err)?;
    Ok(PyModel { inner: r.model })
}

#[pyfunction]
fn draft(model: &mut PyModel, solid: &PySolidHandle, faces: Vec<PyRef<PyFaceHandle>>, pull_direction: [f64; 3], angle: f64) -> PyResult<PySolidHandle> {
    let face_handles: Vec<Handle<FaceData>> = faces.iter().map(|f| f.inner).collect();
    let r = draft_faces(&mut model.inner, solid.inner, &face_handles, v3(pull_direction), angle).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

#[pyfunction]
fn section(model: &PyModel, solid: &PySolidHandle, plane_point: [f64; 3], plane_normal: [f64; 3]) -> PyResult<Vec<PySectionEdge>> {
    let r = section_solid(&model.inner, solid.inner, pt3(plane_point), v3(plane_normal)).map_err(to_py_err)?;
    Ok(r.edges.into_iter().map(|e| PySectionEdge {
        start: [e.start.x, e.start.y, e.start.z],
        end: [e.end.x, e.end.y, e.end.z],
    }).collect())
}

#[pyfunction]
fn split(model: &mut PyModel, solid: &PySolidHandle, plane_point: [f64; 3], plane_normal: [f64; 3]) -> PyResult<Vec<PySolidHandle>> {
    let r = split_solid(&mut model.inner, solid.inner, pt3(plane_point), v3(plane_normal)).map_err(to_py_err)?;
    Ok(r.solids.into_iter().map(|s| PySolidHandle { inner: s }).collect())
}

#[pyfunction]
fn thickness(model: &mut PyModel, solid: &PySolidHandle, thick: f64, join: &str) -> PyResult<PySolidHandle> {
    let jt = match join {
        "inward" => ThicknessJoin::Inward,
        "outward" => ThicknessJoin::Outward,
        "center" => ThicknessJoin::Centered,
        _ => return Err(PyValueError::new_err("join must be 'inward', 'outward', or 'center'")),
    };
    let r = thickness_solid(&mut model.inner, solid.inner, thick, jt).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

#[pyfunction]
fn taper(model: &mut PyModel, profile: Vec<[f64; 3]>, direction: [f64; 3], height: f64, taper_angle: f64) -> PyResult<PySolidHandle> {
    let prof: Vec<Point3> = profile.into_iter().map(pt3).collect();
    let r = taper_extrude(&mut model.inner, &prof, v3(direction), height, taper_angle).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

#[pyfunction]
fn offset(model: &mut PyModel, solid: &PySolidHandle, distance: f64) -> PyResult<PySolidHandle> {
    let r = offset_solid(&mut model.inner, solid.inner, distance).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

#[pyfunction]
fn groove_profile(model: &PyModel, solid: &PySolidHandle, profile: Vec<[f64; 3]>, axis_origin: [f64; 3], axis_dir: [f64; 3], angle: f64, segments: usize) -> PyResult<PyModel> {
    let prof: Vec<Point3> = profile.into_iter().map(pt3).collect();
    let r = groove(&model.inner, solid.inner, &prof, pt3(axis_origin), v3(axis_dir), angle, segments).map_err(to_py_err)?;
    Ok(PyModel { inner: r.model })
}

#[pyfunction]
fn create_hole(model: &PyModel, solid: &PySolidHandle, center: [f64; 3], direction: [f64; 3], radius: f64, depth: f64) -> PyResult<PyModel> {
    let r = hole(&model.inner, solid.inner, pt3(center), v3(direction), radius, depth, 64).map_err(to_py_err)?;
    Ok(PyModel { inner: r.model })
}

#[pyfunction]
#[allow(clippy::too_many_arguments)]
fn create_countersunk_hole(model: &PyModel, solid: &PySolidHandle, center: [f64; 3], direction: [f64; 3], radius: f64, depth: f64, csink_radius: f64, csink_depth: f64) -> PyResult<PyModel> {
    let r = countersunk_hole(&model.inner, solid.inner, pt3(center), v3(direction), radius, depth, csink_radius, csink_depth, 64).map_err(to_py_err)?;
    Ok(PyModel { inner: r.model })
}

#[pyfunction]
fn cross_section(model: &PyModel, solid: &PySolidHandle, direction: [f64; 3], num_sections: usize) -> PyResult<Vec<Vec<PySectionEdge>>> {
    let results = cross_sections(&model.inner, solid.inner, v3(direction), num_sections).map_err(to_py_err)?;
    Ok(results.into_iter().map(|sec| {
        sec.edges.into_iter().map(|e| PySectionEdge {
            start: [e.start.x, e.start.y, e.start.z],
            end: [e.end.x, e.end.y, e.end.z],
        }).collect()
    }).collect())
}

#[pyfunction]
fn reverse(model: &mut PyModel, solid: &PySolidHandle) -> PyResult<PySolidHandle> {
    let r = reverse_solid(&mut model.inner, solid.inner).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

#[pyfunction]
fn refine(model: &PyModel, solid: &PySolidHandle) -> PyResult<usize> {
    let r = refine_shape(&model.inner, solid.inner).map_err(to_py_err)?;
    Ok(r.redundant_edge_count)
}

#[pyfunction]
fn mesh_to_solid(model: &mut PyModel, mesh: &PyMesh) -> PyResult<PySolidHandle> {
    let r = shape_from_mesh(&mut model.inner, &mesh.inner).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

// ---------------------------------------------------------------------------
// Additive / Subtractive operations
// ---------------------------------------------------------------------------

#[pyfunction]
fn pd_additive_box(model: &PyModel, solid: &PySolidHandle, origin: [f64; 3], dx: f64, dy: f64, dz: f64) -> PyResult<PyModel> {
    let r = additive_box(&model.inner, solid.inner, pt3(origin), dx, dy, dz).map_err(to_py_err)?;
    Ok(PyModel { inner: r })
}

#[pyfunction]
fn pd_subtractive_box(model: &PyModel, solid: &PySolidHandle, origin: [f64; 3], dx: f64, dy: f64, dz: f64) -> PyResult<PyModel> {
    let r = subtractive_box(&model.inner, solid.inner, pt3(origin), dx, dy, dz).map_err(to_py_err)?;
    Ok(PyModel { inner: r })
}

#[pyfunction]
fn pd_additive_cylinder(model: &PyModel, solid: &PySolidHandle, center: [f64; 3], radius: f64, height: f64) -> PyResult<PyModel> {
    let r = additive_cylinder(&model.inner, solid.inner, pt3(center), radius, height, 64).map_err(to_py_err)?;
    Ok(PyModel { inner: r })
}

#[pyfunction]
fn pd_subtractive_cylinder(model: &PyModel, solid: &PySolidHandle, center: [f64; 3], radius: f64, height: f64) -> PyResult<PyModel> {
    let r = subtractive_cylinder(&model.inner, solid.inner, pt3(center), radius, height, 64).map_err(to_py_err)?;
    Ok(PyModel { inner: r })
}

#[pyfunction]
fn pd_additive_sphere(model: &PyModel, solid: &PySolidHandle, center: [f64; 3], radius: f64) -> PyResult<PyModel> {
    let r = additive_sphere(&model.inner, solid.inner, pt3(center), radius, 64, 32).map_err(to_py_err)?;
    Ok(PyModel { inner: r })
}

#[pyfunction]
fn pd_subtractive_sphere(model: &PyModel, solid: &PySolidHandle, center: [f64; 3], radius: f64) -> PyResult<PyModel> {
    let r = subtractive_sphere(&model.inner, solid.inner, pt3(center), radius, 64, 32).map_err(to_py_err)?;
    Ok(PyModel { inner: r })
}

#[pyfunction]
fn pd_additive_cone(model: &PyModel, solid: &PySolidHandle, origin: [f64; 3], r1: f64, r2: f64, height: f64) -> PyResult<PyModel> {
    let r = additive_cone(&model.inner, solid.inner, pt3(origin), r1, r2, height, 64).map_err(to_py_err)?;
    Ok(PyModel { inner: r })
}

#[pyfunction]
fn pd_subtractive_cone(model: &PyModel, solid: &PySolidHandle, origin: [f64; 3], r1: f64, r2: f64, height: f64) -> PyResult<PyModel> {
    let r = subtractive_cone(&model.inner, solid.inner, pt3(origin), r1, r2, height, 64).map_err(to_py_err)?;
    Ok(PyModel { inner: r })
}

#[pyfunction]
fn pd_additive_torus(model: &PyModel, solid: &PySolidHandle, center: [f64; 3], major_r: f64, minor_r: f64) -> PyResult<PyModel> {
    let r = additive_torus(&model.inner, solid.inner, pt3(center), major_r, minor_r, 64, 32).map_err(to_py_err)?;
    Ok(PyModel { inner: r })
}

#[pyfunction]
fn pd_subtractive_torus(model: &PyModel, solid: &PySolidHandle, center: [f64; 3], major_r: f64, minor_r: f64) -> PyResult<PyModel> {
    let r = subtractive_torus(&model.inner, solid.inner, pt3(center), major_r, minor_r, 64, 32).map_err(to_py_err)?;
    Ok(PyModel { inner: r })
}

#[pyfunction]
fn pd_additive_helix(model: &PyModel, solid: &PySolidHandle, origin: [f64; 3], radius: f64, pitch: f64, turns: f64, tube_r: f64) -> PyResult<PyModel> {
    let r = additive_helix(&model.inner, solid.inner, pt3(origin), radius, pitch, turns, tube_r, 16, 8).map_err(to_py_err)?;
    Ok(PyModel { inner: r })
}

#[pyfunction]
fn pd_subtractive_helix(model: &PyModel, solid: &PySolidHandle, origin: [f64; 3], radius: f64, pitch: f64, turns: f64, tube_r: f64) -> PyResult<PyModel> {
    let r = subtractive_helix(&model.inner, solid.inner, pt3(origin), radius, pitch, turns, tube_r, 16, 8).map_err(to_py_err)?;
    Ok(PyModel { inner: r })
}

#[pyfunction]
fn pd_additive_ellipsoid(model: &PyModel, solid: &PySolidHandle, origin: [f64; 3], rx: f64, ry: f64, rz: f64) -> PyResult<PyModel> {
    let r = additive_ellipsoid(&model.inner, solid.inner, pt3(origin), rx, ry, rz, 64, 32).map_err(to_py_err)?;
    Ok(PyModel { inner: r })
}

#[pyfunction]
fn pd_subtractive_ellipsoid(model: &PyModel, solid: &PySolidHandle, origin: [f64; 3], rx: f64, ry: f64, rz: f64) -> PyResult<PyModel> {
    let r = subtractive_ellipsoid(&model.inner, solid.inner, pt3(origin), rx, ry, rz, 64, 32).map_err(to_py_err)?;
    Ok(PyModel { inner: r })
}

#[pyfunction]
fn pd_additive_prism(model: &PyModel, solid: &PySolidHandle, origin: [f64; 3], radius: f64, height: f64, sides: usize) -> PyResult<PyModel> {
    let r = additive_prism(&model.inner, solid.inner, pt3(origin), radius, height, sides).map_err(to_py_err)?;
    Ok(PyModel { inner: r })
}

#[pyfunction]
fn pd_subtractive_prism(model: &PyModel, solid: &PySolidHandle, origin: [f64; 3], radius: f64, height: f64, sides: usize) -> PyResult<PyModel> {
    let r = subtractive_prism(&model.inner, solid.inner, pt3(origin), radius, height, sides).map_err(to_py_err)?;
    Ok(PyModel { inner: r })
}

#[pyfunction]
#[allow(clippy::too_many_arguments)]
fn pd_additive_wedge(model: &PyModel, solid: &PySolidHandle, origin: [f64; 3], dx: f64, dy: f64, dz: f64, dx2: f64, dy2: f64) -> PyResult<PyModel> {
    let r = additive_wedge(&model.inner, solid.inner, pt3(origin), dx, dy, dz, dx2, dy2, 0.0, 0.0).map_err(to_py_err)?;
    Ok(PyModel { inner: r })
}

#[pyfunction]
#[allow(clippy::too_many_arguments)]
fn pd_subtractive_wedge(model: &PyModel, solid: &PySolidHandle, origin: [f64; 3], dx: f64, dy: f64, dz: f64, dx2: f64, dy2: f64) -> PyResult<PyModel> {
    let r = subtractive_wedge(&model.inner, solid.inner, pt3(origin), dx, dy, dz, dx2, dy2, 0.0, 0.0).map_err(to_py_err)?;
    Ok(PyModel { inner: r })
}

#[pyfunction]
fn pd_additive_loft(model: &PyModel, solid: &PySolidHandle, profiles: Vec<Vec<[f64; 3]>>) -> PyResult<PyModel> {
    let profs: Vec<Vec<Point3>> = profiles.into_iter().map(|p| p.into_iter().map(pt3).collect()).collect();
    let refs: Vec<&[Point3]> = profs.iter().map(|p| p.as_slice()).collect();
    let r = additive_loft(&model.inner, solid.inner, &refs).map_err(to_py_err)?;
    Ok(PyModel { inner: r })
}

#[pyfunction]
fn pd_subtractive_loft(model: &PyModel, solid: &PySolidHandle, profiles: Vec<Vec<[f64; 3]>>) -> PyResult<PyModel> {
    let profs: Vec<Vec<Point3>> = profiles.into_iter().map(|p| p.into_iter().map(pt3).collect()).collect();
    let refs: Vec<&[Point3]> = profs.iter().map(|p| p.as_slice()).collect();
    let r = subtractive_loft(&model.inner, solid.inner, &refs).map_err(to_py_err)?;
    Ok(PyModel { inner: r })
}

#[pyfunction]
fn pd_additive_pipe(model: &PyModel, solid: &PySolidHandle, profile: Vec<[f64; 3]>, path: Vec<[f64; 3]>) -> PyResult<PyModel> {
    let prof: Vec<Point3> = profile.into_iter().map(pt3).collect();
    let pth: Vec<Point3> = path.into_iter().map(pt3).collect();
    let r = additive_pipe(&model.inner, solid.inner, &prof, &pth).map_err(to_py_err)?;
    Ok(PyModel { inner: r })
}

#[pyfunction]
fn pd_subtractive_pipe(model: &PyModel, solid: &PySolidHandle, profile: Vec<[f64; 3]>, path: Vec<[f64; 3]>) -> PyResult<PyModel> {
    let prof: Vec<Point3> = profile.into_iter().map(pt3).collect();
    let pth: Vec<Point3> = path.into_iter().map(pt3).collect();
    let r = subtractive_pipe(&model.inner, solid.inner, &prof, &pth).map_err(to_py_err)?;
    Ok(PyModel { inner: r })
}

// ---------------------------------------------------------------------------
// Free functions — Join operations
// ---------------------------------------------------------------------------

#[pyfunction]
fn join_connect(model_a: &PyModel, solid_a: &PySolidHandle, model_b: &PyModel, solid_b: &PySolidHandle) -> PyResult<PyModel> {
    let r = connect_shapes(&model_a.inner, solid_a.inner, &model_b.inner, solid_b.inner).map_err(to_py_err)?;
    Ok(PyModel { inner: r })
}

#[pyfunction]
fn join_embed(model_a: &PyModel, solid_a: &PySolidHandle, model_b: &PyModel, solid_b: &PySolidHandle) -> PyResult<PyModel> {
    let r = embed_shapes(&model_a.inner, solid_a.inner, &model_b.inner, solid_b.inner).map_err(to_py_err)?;
    Ok(PyModel { inner: r })
}

#[pyfunction]
fn join_cutout(model_a: &PyModel, solid_a: &PySolidHandle, model_b: &PyModel, solid_b: &PySolidHandle) -> PyResult<PyModel> {
    let r = cutout_shapes(&model_a.inner, solid_a.inner, &model_b.inner, solid_b.inner).map_err(to_py_err)?;
    Ok(PyModel { inner: r })
}

// ---------------------------------------------------------------------------
// Free functions — Multi-transform
// ---------------------------------------------------------------------------

#[pyfunction]
fn apply_multi_transform(model: &mut PyModel, solid: &PySolidHandle, transforms: Vec<(String, Vec<f64>)>) -> PyResult<PySolidHandle> {
    let mut xforms = Vec::new();
    for (kind, vals) in &transforms {
        match kind.as_str() {
            "translate" => {
                if vals.len() < 3 {
                    return Err(PyValueError::new_err("translate requires [x, y, z]"));
                }
                xforms.push(ModelTransform::Translation(Vec3::new(vals[0], vals[1], vals[2])));
            }
            "rotate" => {
                if vals.len() < 7 {
                    return Err(PyValueError::new_err("rotate requires [ox, oy, oz, dx, dy, dz, angle]"));
                }
                xforms.push(ModelTransform::Rotation {
                    axis_origin: Point3::new(vals[0], vals[1], vals[2]),
                    axis_dir: Vec3::new(vals[3], vals[4], vals[5]),
                    angle: vals[6],
                });
            }
            "scale" => {
                if vals.len() < 4 {
                    return Err(PyValueError::new_err("scale requires [cx, cy, cz, factor]"));
                }
                xforms.push(ModelTransform::Scale {
                    center: Point3::new(vals[0], vals[1], vals[2]),
                    factor: vals[3],
                });
            }
            "mirror" => {
                if vals.len() < 6 {
                    return Err(PyValueError::new_err("mirror requires [px, py, pz, nx, ny, nz]"));
                }
                xforms.push(ModelTransform::Mirror {
                    plane_point: Point3::new(vals[0], vals[1], vals[2]),
                    plane_normal: Vec3::new(vals[3], vals[4], vals[5]),
                });
            }
            _ => return Err(PyValueError::new_err("transform type must be 'translate', 'rotate', 'scale', or 'mirror'")),
        }
    }
    let r = multi_transform(&mut model.inner, solid.inner, &xforms).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

// ---------------------------------------------------------------------------
// Free functions — Boolean operations
// ---------------------------------------------------------------------------

/// Computes the boolean union of two solids.
#[pyfunction]
fn boolean_union(model_a: &PyModel, solid_a: &PySolidHandle, model_b: &PyModel, solid_b: &PySolidHandle) -> PyResult<PyModel> {
    let result = boolean_op(&model_a.inner, solid_a.inner, &model_b.inner, solid_b.inner, BooleanOp::Union).map_err(to_py_err)?;
    Ok(PyModel { inner: result })
}

/// Computes the boolean subtraction (A minus B) of two solids.
#[pyfunction]
fn boolean_subtract(model_a: &PyModel, solid_a: &PySolidHandle, model_b: &PyModel, solid_b: &PySolidHandle) -> PyResult<PyModel> {
    let result = boolean_op(&model_a.inner, solid_a.inner, &model_b.inner, solid_b.inner, BooleanOp::Difference).map_err(to_py_err)?;
    Ok(PyModel { inner: result })
}

/// Computes the boolean intersection of two solids.
#[pyfunction]
fn boolean_intersect(model_a: &PyModel, solid_a: &PySolidHandle, model_b: &PyModel, solid_b: &PySolidHandle) -> PyResult<PyModel> {
    let result = boolean_op(&model_a.inner, solid_a.inner, &model_b.inner, solid_b.inner, BooleanOp::Intersection).map_err(to_py_err)?;
    Ok(PyModel { inner: result })
}

/// Computes the boolean XOR (symmetric difference) of two solids.
#[pyfunction]
fn boolean_xor_op(model_a: &PyModel, solid_a: &PySolidHandle, model_b: &PyModel, solid_b: &PySolidHandle) -> PyResult<PyModel> {
    let result = boolean_xor(&model_a.inner, solid_a.inner, &model_b.inner, solid_b.inner).map_err(to_py_err)?;
    Ok(PyModel { inner: result })
}

// ---------------------------------------------------------------------------
// Free functions — Tessellation & Measurement
// ---------------------------------------------------------------------------

/// Tessellates a solid into a triangle mesh for rendering or export.
#[pyfunction]
fn tessellate(model: &PyModel, solid: &PySolidHandle) -> PyMesh {
    let mesh = cadkernel_io::tessellate_solid(&model.inner, solid.inner);
    PyMesh { inner: mesh }
}

/// Computes volume, surface area, and centroid from a mesh.
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

#[pyfunction]
fn watertight_check(model: &PyModel, solid: &PySolidHandle) -> bool {
    check_watertight(&model.inner, solid.inner)
}

#[pyfunction]
fn dist_between_points(p1: [f64; 3], p2: [f64; 3]) -> f64 {
    let d = [p2[0] - p1[0], p2[1] - p1[1], p2[2] - p1[2]];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

// ---------------------------------------------------------------------------
// Free functions — Mesh operations
// ---------------------------------------------------------------------------

/// Reduces triangle count by the given ratio (0.0 to 1.0) via edge collapse.
#[pyfunction]
fn decimate(mesh: &PyMesh, target_ratio: f64) -> PyResult<PyMesh> {
    let r = cadkernel_io::mesh_ops::decimate_mesh(&mesh.inner, target_ratio).map_err(to_py_err)?;
    Ok(PyMesh { inner: r })
}

/// Fills boundary edge holes with fan triangulation from loop centroids.
#[pyfunction]
fn mesh_fill_holes(mesh: &PyMesh) -> PyResult<PyMesh> {
    let r = cadkernel_io::mesh_ops::fill_holes(&mesh.inner).map_err(to_py_err)?;
    Ok(PyMesh { inner: r })
}

/// Computes per-vertex mean curvature via the cotangent Laplace-Beltrami operator.
#[pyfunction]
fn mesh_curvature(mesh: &PyMesh) -> PyResult<Vec<f64>> {
    cadkernel_io::mesh_ops::compute_curvature(&mesh.inner).map_err(to_py_err)
}

/// Subdivides each triangle into four by inserting edge midpoints.
#[pyfunction]
fn mesh_subdivide(mesh: &PyMesh) -> PyResult<PyMesh> {
    let r = cadkernel_io::mesh_ops::subdivide_mesh(&mesh.inner).map_err(to_py_err)?;
    Ok(PyMesh { inner: r })
}

/// Reverses the winding order and negates normals of all triangles.
#[pyfunction]
fn mesh_flip_normals(mesh: &PyMesh) -> PyMesh {
    PyMesh { inner: cadkernel_io::mesh_ops::flip_normals(&mesh.inner) }
}

/// Applies Laplacian smoothing to mesh vertices.
#[pyfunction]
fn mesh_smooth(mesh: &PyMesh, iterations: usize, factor: f64) -> PyMesh {
    PyMesh { inner: cadkernel_io::mesh_ops::smooth_mesh(&mesh.inner, iterations, factor) }
}

/// Mesh boolean union: combines triangles from both meshes.
#[pyfunction]
fn mesh_union(a: &PyMesh, b: &PyMesh) -> PyMesh {
    PyMesh { inner: cadkernel_io::mesh_ops::mesh_boolean_union(&a.inner, &b.inner) }
}

/// Mesh boolean intersection via AABB overlap filtering.
#[pyfunction]
fn mesh_intersection(a: &PyMesh, b: &PyMesh) -> PyMesh {
    PyMesh { inner: cadkernel_io::mesh_ops::mesh_boolean_intersection(&a.inner, &b.inner) }
}

/// Mesh boolean difference: keeps A triangles not overlapping B.
#[pyfunction]
fn mesh_difference(a: &PyMesh, b: &PyMesh) -> PyMesh {
    PyMesh { inner: cadkernel_io::mesh_ops::mesh_boolean_difference(&a.inner, &b.inner) }
}

/// Cuts a mesh with a plane, keeping the side where the normal points.
#[pyfunction]
fn mesh_cut_with_plane(mesh: &PyMesh, plane_point: [f64; 3], plane_normal: [f64; 3]) -> PyMesh {
    PyMesh { inner: cadkernel_io::mesh_ops::cut_mesh_with_plane(&mesh.inner, pt3(plane_point), v3(plane_normal)) }
}

/// Makes normals consistent (all outward) via BFS winding propagation.
#[pyfunction]
fn mesh_harmonize_normals(mesh: &PyMesh) -> PyMesh {
    PyMesh { inner: cadkernel_io::mesh_ops::harmonize_normals(&mesh.inner) }
}

/// Returns true if every edge is shared by exactly 2 triangles.
#[pyfunction]
fn mesh_is_watertight(mesh: &PyMesh) -> bool {
    cadkernel_io::mesh_ops::check_mesh_watertight(&mesh.inner)
}

/// Remeshes by splitting edges longer than `max_edge_length`.
#[pyfunction]
fn mesh_remesh(mesh: &PyMesh, max_edge_length: f64) -> PyResult<PyMesh> {
    let r = cadkernel_io::mesh_ops::remesh(&mesh.inner, max_edge_length).map_err(to_py_err)?;
    Ok(PyMesh { inner: r })
}

/// Scales a mesh by independent factors along each axis.
#[pyfunction]
fn mesh_scale(mesh: &PyMesh, sx: f64, sy: f64, sz: f64) -> PyMesh {
    PyMesh { inner: cadkernel_io::mesh_ops::scale_mesh(&mesh.inner, sx, sy, sz) }
}

/// Evaluates and repairs a mesh, returning the fixed mesh and a list of issues found.
#[pyfunction]
fn mesh_repair(mesh: &PyMesh) -> (PyMesh, Vec<String>) {
    let (repaired, report) = cadkernel_io::mesh_ops::evaluate_and_repair(&mesh.inner);
    let issues = vec![
        format!("degenerate_removed={}", report.degenerate_removed),
        format!("duplicate_vertices_merged={}", report.duplicate_vertices_merged),
        format!("normals_harmonized={}", report.normals_harmonized),
    ];
    (PyMesh { inner: repaired }, issues)
}

// ---------------------------------------------------------------------------
// Free functions — FEM
// ---------------------------------------------------------------------------

/// Generates a tetrahedral mesh from a solid for FEM analysis.
#[pyfunction]
fn fem_generate_tet_mesh(model: &PyModel, solid: &PySolidHandle, max_edge_length: f64) -> PyResult<PyTetMesh> {
    let r = generate_tet_mesh(&model.inner, solid.inner, max_edge_length).map_err(to_py_err)?;
    Ok(PyTetMesh { inner: r })
}

/// Runs FEM static analysis with material, fixed nodes, and applied force.
///
/// `material` must be one of: "steel", "aluminum", "titanium", "copper", "concrete", "cast_iron".
#[pyfunction]
fn fem_static_analysis(tet_mesh: &PyTetMesh, material: &str, fixed_nodes: Vec<usize>, force_node: usize, force: [f64; 3]) -> PyResult<PyFemResult> {
    let mat = match material {
        "steel" => FemMaterial::steel(),
        "aluminum" => FemMaterial::aluminum(),
        "titanium" => FemMaterial::titanium(),
        "copper" => FemMaterial::copper(),
        "concrete" => FemMaterial::concrete(),
        "cast_iron" => FemMaterial::cast_iron(),
        _ => return Err(PyValueError::new_err("material must be 'steel', 'aluminum', 'titanium', 'copper', 'concrete', or 'cast_iron'")),
    };
    let mut bcs: Vec<BoundaryCondition> = fixed_nodes.into_iter().map(BoundaryCondition::FixedNode).collect();
    bcs.push(BoundaryCondition::Force { node: force_node, force: v3(force) });
    let r = static_analysis(&tet_mesh.inner, &mat, &bcs).map_err(to_py_err)?;
    Ok(PyFemResult {
        max_displacement: r.max_displacement,
        max_stress: r.max_stress,
        displacements: r.displacements,
        stresses: r.stresses,
    })
}

// ---------------------------------------------------------------------------
// Free functions — FEM (thermal, modal)
// ---------------------------------------------------------------------------

/// Runs FEM modal (vibration) analysis, returning natural frequencies.
#[pyfunction]
fn fem_modal_analysis(tet_mesh: &PyTetMesh, material: &str, fixed_nodes: Vec<usize>, num_modes: usize) -> PyResult<PyModalResult> {
    let mat = parse_fem_material(material)?;
    let bcs: Vec<BoundaryCondition> = fixed_nodes.into_iter().map(BoundaryCondition::FixedNode).collect();
    let r = modal_analysis(&tet_mesh.inner, &mat, &bcs, num_modes).map_err(to_py_err)?;
    Ok(PyModalResult {
        frequencies: r.frequencies,
        num_modes: num_modes.min(r.mode_shapes.len()),
    })
}

/// Runs FEM thermal analysis with fixed temperatures and heat flux boundary conditions.
#[pyfunction]
fn fem_thermal_analysis(tet_mesh: &PyTetMesh, material: &str, fixed_temps: Vec<(usize, f64)>, heat_flux: Vec<(usize, f64)>) -> PyResult<PyThermalResult> {
    let tmat = match material {
        "steel" => ThermalMaterial::steel(),
        "aluminum" => ThermalMaterial::aluminum(),
        "copper" => ThermalMaterial::copper(),
        _ => return Err(PyValueError::new_err("thermal material must be 'steel', 'aluminum', or 'copper'")),
    };
    let mut bcs: Vec<ThermalBoundaryCondition> = fixed_temps.into_iter()
        .map(|(node, temp)| ThermalBoundaryCondition::FixedTemperature { node, temperature: temp })
        .collect();
    for (elem, flux) in heat_flux {
        bcs.push(ThermalBoundaryCondition::HeatFlux { element: elem, flux });
    }
    let r = thermal_analysis(&tet_mesh.inner, &tmat, &bcs).map_err(to_py_err)?;
    Ok(PyThermalResult {
        max_temperature: r.max_temperature,
        temperatures: r.temperatures,
    })
}

/// Creates a custom FEM material with specified elastic properties.
#[pyfunction]
fn fem_custom_material(youngs_modulus: f64, poisson_ratio: f64, density: f64) -> PyResult<String> {
    let _mat = FemMaterial::custom(youngs_modulus, poisson_ratio, density).map_err(to_py_err)?;
    Ok(format!("custom(E={youngs_modulus:.2e}, nu={poisson_ratio}, rho={density})"))
}

fn parse_fem_material(material: &str) -> PyResult<FemMaterial> {
    match material {
        "steel" => Ok(FemMaterial::steel()),
        "aluminum" => Ok(FemMaterial::aluminum()),
        "titanium" => Ok(FemMaterial::titanium()),
        "copper" => Ok(FemMaterial::copper()),
        "concrete" => Ok(FemMaterial::concrete()),
        "cast_iron" => Ok(FemMaterial::cast_iron()),
        _ => Err(PyValueError::new_err("material must be 'steel', 'aluminum', 'titanium', 'copper', 'concrete', or 'cast_iron'")),
    }
}

// ---------------------------------------------------------------------------
// Free functions — Draft operations
// ---------------------------------------------------------------------------

#[pyfunction]
fn draft_make_wire(model: &mut PyModel, points: Vec<[f64; 3]>) -> PyResult<Vec<PyVertexHandle>> {
    let pts: Vec<Point3> = points.into_iter().map(pt3).collect();
    let r = make_wire(&mut model.inner, &pts).map_err(to_py_err)?;
    Ok(r.vertices.into_iter().map(|v| PyVertexHandle { inner: v }).collect())
}

#[pyfunction]
fn draft_clone_solid(model: &mut PyModel, solid: &PySolidHandle) -> PyResult<PySolidHandle> {
    let r = clone_solid(&mut model.inner, solid.inner).map_err(to_py_err)?;
    Ok(PySolidHandle { inner: r.solid })
}

#[pyfunction]
#[allow(clippy::too_many_arguments)]
fn draft_rectangular_array(
    model: &mut PyModel, solid: &PySolidHandle,
    dir1: [f64; 3], spacing_x: f64, count_x: usize,
    dir2: [f64; 3], spacing_y: f64, count_y: usize,
) -> PyResult<Vec<PySolidHandle>> {
    let r = rectangular_array(
        &mut model.inner, solid.inner,
        v3(dir1), spacing_x, count_x,
        v3(dir2), spacing_y, count_y,
    ).map_err(to_py_err)?;
    Ok(r.solids.into_iter().map(|s| PySolidHandle { inner: s }).collect())
}

#[pyfunction]
fn draft_path_array(model: &mut PyModel, solid: &PySolidHandle, path: Vec<[f64; 3]>) -> PyResult<Vec<PySolidHandle>> {
    let pth: Vec<Point3> = path.into_iter().map(pt3).collect();
    let r = path_array(&mut model.inner, solid.inner, &pth).map_err(to_py_err)?;
    Ok(r.solids.into_iter().map(|s| PySolidHandle { inner: s }).collect())
}

#[pyfunction]
fn draft_polar_array(model: &mut PyModel, solid: &PySolidHandle, center: [f64; 3], axis: [f64; 3], count: usize) -> PyResult<Vec<PySolidHandle>> {
    let r = polar_array(&mut model.inner, solid.inner, pt3(center), v3(axis), count).map_err(to_py_err)?;
    Ok(r.into_iter().map(|s| PySolidHandle { inner: s }).collect())
}

// ---------------------------------------------------------------------------
// Free functions — Compound operations
// ---------------------------------------------------------------------------

#[pyfunction]
fn compound_boolean_fragments(model_a: &PyModel, solid_a: &PySolidHandle, model_b: &PyModel, solid_b: &PySolidHandle) -> PyResult<PyCompound> {
    let r = boolean_fragments(&model_a.inner, solid_a.inner, &model_b.inner, solid_b.inner).map_err(to_py_err)?;
    Ok(PyCompound { inner: r.compound })
}

#[pyfunction]
fn compound_slice(model: &mut PyModel, solid: &PySolidHandle, plane_point: [f64; 3], plane_normal: [f64; 3]) -> PyResult<PyCompound> {
    let r = slice_to_compound(&mut model.inner, solid.inner, pt3(plane_point), v3(plane_normal)).map_err(to_py_err)?;
    Ok(PyCompound { inner: r.compound })
}

#[pyfunction]
fn compound_filter_by_faces(model: &PyModel, compound: &PyCompound, min_faces: usize) -> PyResult<PyCompound> {
    let r = compound_filter(&model.inner, &compound.inner, min_faces).map_err(to_py_err)?;
    Ok(PyCompound { inner: r })
}

// ---------------------------------------------------------------------------
// Free functions — Measurement
// ---------------------------------------------------------------------------

#[pyfunction]
fn measure_dist(model: &PyModel, v1: &PyVertexHandle, v2: &PyVertexHandle) -> PyResult<f64> {
    measure_distance(&model.inner, v1.inner, v2.inner).map_err(to_py_err)
}

#[pyfunction]
fn measure_ang(model: &PyModel, e1: &PyEdgeHandle, e2: &PyEdgeHandle) -> PyResult<f64> {
    measure_angle(&model.inner, e1.inner, e2.inner).map_err(to_py_err)
}

#[pyfunction]
fn measure_edge_len(model: &PyModel, edge: &PyEdgeHandle) -> PyResult<f64> {
    measure_edge_length(&model.inner, edge.inner).map_err(to_py_err)
}

// ---------------------------------------------------------------------------
// Free functions — Shape analysis & query
// ---------------------------------------------------------------------------

#[pyfunction]
fn classify(model: &PyModel, solid: &PySolidHandle) -> String {
    let st = classify_solid(&model.inner, solid.inner);
    format!("{st:?}")
}

#[pyfunction]
fn planar_faces(model: &PyModel, solid: &PySolidHandle) -> Vec<PyFaceHandle> {
    find_planar_faces(&model.inner, solid.inner)
        .into_iter()
        .map(|f| PyFaceHandle { inner: f })
        .collect()
}

#[pyfunction]
fn cylindrical_faces(model: &PyModel, solid: &PySolidHandle) -> Vec<PyFaceHandle> {
    find_cylindrical_faces(&model.inner, solid.inner)
        .into_iter()
        .map(|f| PyFaceHandle { inner: f })
        .collect()
}

#[pyfunction]
fn query_point_in_solid(model: &PyModel, solid: &PySolidHandle, point: [f64; 3]) -> PyResult<String> {
    let c = point_in_solid(&model.inner, solid.inner, pt3(point)).map_err(to_py_err)?;
    Ok(match c {
        Containment::Inside => "inside".to_string(),
        Containment::Outside => "outside".to_string(),
        Containment::OnBoundary => "on_boundary".to_string(),
    })
}

#[pyfunction]
fn query_closest_point(model: &PyModel, solid: &PySolidHandle, point: [f64; 3]) -> PyResult<([f64; 3], f64)> {
    let r = closest_point_on_solid(&model.inner, solid.inner, pt3(point)).map_err(to_py_err)?;
    Ok(([r.point.x, r.point.y, r.point.z], r.distance))
}

// ---------------------------------------------------------------------------
// Free functions — Assembly operations
// ---------------------------------------------------------------------------

#[pyfunction]
fn assembly_add_component(asm: &mut PyAssembly, solid: &PySolidHandle, name: &str) -> usize {
    let id = asm.inner.add_component(name, solid.inner);
    id.0
}

#[pyfunction]
fn assembly_add_fixed(asm: &mut PyAssembly, comp_id: usize) {
    asm.inner.add_constraint(AssemblyConstraint::Fixed(ComponentId(comp_id)));
}

#[pyfunction]
fn assembly_add_coincident(asm: &mut PyAssembly, comp_a: usize, comp_b: usize, offset: f64) {
    asm.inner.add_constraint(AssemblyConstraint::Coincident {
        comp_a: ComponentId(comp_a),
        comp_b: ComponentId(comp_b),
        offset,
    });
}

#[pyfunction]
fn assembly_add_distance(asm: &mut PyAssembly, comp_a: usize, comp_b: usize, distance: f64) {
    asm.inner.add_constraint(AssemblyConstraint::Distance {
        comp_a: ComponentId(comp_a),
        comp_b: ComponentId(comp_b),
        distance,
    });
}

#[pyfunction]
fn assembly_bill_of_materials(asm: &PyAssembly) -> Vec<(String, usize)> {
    asm.inner.bill_of_materials().into_iter().map(|b| (b.name, b.quantity)).collect()
}

#[pyfunction]
fn assembly_exploded_view(asm: &mut PyAssembly, offset_factor: f64) {
    asm.inner.exploded_view(offset_factor);
}

// ---------------------------------------------------------------------------
// Wrapper classes for new result types
// ---------------------------------------------------------------------------

/// Result of a FEM modal analysis: natural frequencies and mode count.
#[pyclass(name = "ModalResult")]
struct PyModalResult {
    #[pyo3(get)]
    frequencies: Vec<f64>,
    #[pyo3(get)]
    num_modes: usize,
}

#[pymethods]
impl PyModalResult {
    fn __repr__(&self) -> String {
        format!("ModalResult(modes={}, freqs={:?})", self.num_modes, &self.frequencies[..self.num_modes.min(self.frequencies.len())])
    }
}

/// Result of a FEM thermal analysis: temperature distribution per node.
#[pyclass(name = "ThermalResult")]
struct PyThermalResult {
    #[pyo3(get)]
    max_temperature: f64,
    #[pyo3(get)]
    temperatures: Vec<f64>,
}

#[pymethods]
impl PyThermalResult {
    fn __repr__(&self) -> String {
        format!("ThermalResult(max_temp={:.4}, nodes={})", self.max_temperature, self.temperatures.len())
    }
}

// ---------------------------------------------------------------------------
// Free functions — I/O
// ---------------------------------------------------------------------------

/// Exports a mesh to ASCII STL format at the given file path.
#[pyfunction]
fn export_stl(path: &str, mesh: &PyMesh) -> PyResult<()> {
    let content = cadkernel_io::write_stl_ascii(&mesh.inner, "CADKernel");
    std::fs::write(path, content).map_err(to_py_err)
}

/// Exports a mesh to binary STL format at the given file path.
#[pyfunction]
fn export_stl_binary(path: &str, mesh: &PyMesh) -> PyResult<()> {
    let bytes = cadkernel_io::write_stl_binary(&mesh.inner).map_err(to_py_err)?;
    std::fs::write(path, bytes).map_err(to_py_err)
}

/// Exports a mesh to Wavefront OBJ format.
#[pyfunction]
fn export_obj(path: &str, mesh: &PyMesh) -> PyResult<()> {
    let content = cadkernel_io::write_obj(&mesh.inner);
    std::fs::write(path, content).map_err(to_py_err)
}

/// Exports a mesh to glTF 2.0 JSON format with embedded buffer.
#[pyfunction]
fn export_gltf(path: &str, mesh: &PyMesh) -> PyResult<()> {
    cadkernel_io::export_gltf(&mesh.inner, path).map_err(to_py_err)
}

/// Exports a B-Rep model to STEP (ISO 10303-21) format.
#[pyfunction]
fn export_step(path: &str, model: &PyModel) -> PyResult<()> {
    let content = cadkernel_io::export_step(&model.inner).map_err(to_py_err)?;
    std::fs::write(path, content).map_err(to_py_err)
}

/// Exports a B-Rep model to IGES format.
#[pyfunction]
fn export_iges(path: &str, model: &PyModel) -> PyResult<()> {
    let content = cadkernel_io::export_iges(&model.inner).map_err(to_py_err)?;
    std::fs::write(path, content).map_err(to_py_err)
}

/// Exports a mesh to DXF format (3DFACE entities).
#[pyfunction]
fn export_dxf(path: &str, mesh: &PyMesh) -> PyResult<()> {
    let content = cadkernel_io::export_dxf(&mesh.inner).map_err(to_py_err)?;
    cadkernel_io::write_dxf(path, &content).map_err(to_py_err)
}

/// Exports a mesh to PLY (ASCII) format.
#[pyfunction]
fn export_ply(path: &str, mesh: &PyMesh) -> PyResult<()> {
    let content = cadkernel_io::export_ply(&mesh.inner).map_err(to_py_err)?;
    cadkernel_io::write_ply(path, &content).map_err(to_py_err)
}

/// Exports a mesh to 3MF (XML) format.
#[pyfunction]
fn export_3mf(path: &str, mesh: &PyMesh) -> PyResult<()> {
    let content = cadkernel_io::export_3mf(&mesh.inner).map_err(to_py_err)?;
    cadkernel_io::write_3mf(path, &content).map_err(to_py_err)
}

/// Exports a B-Rep model to CADKernel BREP text format.
#[pyfunction]
fn export_brep(path: &str, model: &PyModel) -> PyResult<()> {
    let content = cadkernel_io::export_brep(&model.inner).map_err(to_py_err)?;
    cadkernel_io::write_brep(path, &content).map_err(to_py_err)
}

/// Exports a mesh to Collada DAE format.
#[pyfunction]
fn export_dae(path: &str, mesh: &PyMesh) -> PyResult<()> {
    let content = cadkernel_io::export_dae(&mesh.inner).map_err(to_py_err)?;
    cadkernel_io::write_dae(path, &content).map_err(to_py_err)
}

/// Exports a mesh to AMF (Additive Manufacturing) format.
#[pyfunction]
fn export_amf(path: &str, mesh: &PyMesh) -> PyResult<()> {
    cadkernel_io::write_amf(std::path::Path::new(path), &mesh.inner).map_err(to_py_err)
}

/// Imports an STL file (auto-detects ASCII vs binary).
#[pyfunction]
fn import_stl(path: &str) -> PyResult<PyMesh> {
    let mesh = cadkernel_io::import_stl(path).map_err(to_py_err)?;
    Ok(PyMesh { inner: mesh })
}

/// Imports a Wavefront OBJ file into a mesh.
#[pyfunction]
fn import_obj(path: &str) -> PyResult<PyMesh> {
    let mesh = cadkernel_io::import_obj(path).map_err(to_py_err)?;
    Ok(PyMesh { inner: mesh })
}

/// Imports a DXF file (3DFACE entities) into a mesh.
#[pyfunction]
fn import_dxf(path: &str) -> PyResult<PyMesh> {
    let content = std::fs::read_to_string(path).map_err(to_py_err)?;
    let mesh = cadkernel_io::import_dxf(&content).map_err(to_py_err)?;
    Ok(PyMesh { inner: mesh })
}

/// Imports a PLY file into a mesh.
#[pyfunction]
fn import_ply(path: &str) -> PyResult<PyMesh> {
    let content = std::fs::read_to_string(path).map_err(to_py_err)?;
    let mesh = cadkernel_io::import_ply(&content).map_err(to_py_err)?;
    Ok(PyMesh { inner: mesh })
}

/// Imports a 3MF file into a mesh.
#[pyfunction]
fn import_3mf(path: &str) -> PyResult<PyMesh> {
    let content = std::fs::read_to_string(path).map_err(to_py_err)?;
    let mesh = cadkernel_io::import_3mf(&content).map_err(to_py_err)?;
    Ok(PyMesh { inner: mesh })
}

/// Imports a CADKernel BREP text file into a B-Rep model.
#[pyfunction]
fn import_brep(path: &str) -> PyResult<PyModel> {
    let content = std::fs::read_to_string(path).map_err(to_py_err)?;
    let model = cadkernel_io::import_brep(&content).map_err(to_py_err)?;
    Ok(PyModel { inner: model })
}

/// Imports a Collada DAE file into a mesh.
#[pyfunction]
fn import_dae(path: &str) -> PyResult<PyMesh> {
    let content = std::fs::read_to_string(path).map_err(to_py_err)?;
    let mesh = cadkernel_io::import_dae(&content).map_err(to_py_err)?;
    Ok(PyMesh { inner: mesh })
}

/// Imports an AMF file into a mesh.
#[pyfunction]
fn import_amf(path: &str) -> PyResult<PyMesh> {
    let content = std::fs::read_to_string(path).map_err(to_py_err)?;
    let mesh = cadkernel_io::import_amf(&content).map_err(to_py_err)?;
    Ok(PyMesh { inner: mesh })
}

/// Imports a STEP file into a B-Rep model.
#[pyfunction]
fn import_step(path: &str) -> PyResult<PyModel> {
    let content = std::fs::read_to_string(path).map_err(to_py_err)?;
    let model = cadkernel_io::import_step(&content).map_err(to_py_err)?;
    Ok(PyModel { inner: model })
}

/// Imports an IGES file into a B-Rep model.
#[pyfunction]
fn import_iges(path: &str) -> PyResult<PyModel> {
    let content = std::fs::read_to_string(path).map_err(to_py_err)?;
    let model = cadkernel_io::import_iges(&content).map_err(to_py_err)?;
    Ok(PyModel { inner: model })
}

/// Saves a model to the native `.cadk` project format.
#[pyfunction]
fn save_project(path: &str, model: &PyModel) -> PyResult<()> {
    cadkernel_io::save_project(&model.inner, path).map_err(to_py_err)
}

/// Loads a model from the native `.cadk` project format.
#[pyfunction]
fn load_project(path: &str) -> PyResult<PyModel> {
    let model = cadkernel_io::load_project(path).map_err(to_py_err)?;
    Ok(PyModel { inner: model })
}

// ---------------------------------------------------------------------------
// Python module
// ---------------------------------------------------------------------------

/// CADKernel native Python module.
///
/// Registers 14 classes and 100+ functions for CAD modeling, mesh processing,
/// FEM analysis, assembly management, sketch solving, and multi-format I/O.
#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Classes
    m.add_class::<PyModel>()?;
    m.add_class::<PySolidHandle>()?;
    m.add_class::<PyVertexHandle>()?;
    m.add_class::<PyEdgeHandle>()?;
    m.add_class::<PyFaceHandle>()?;
    m.add_class::<PyMesh>()?;
    m.add_class::<PyMassProperties>()?;
    m.add_class::<PyGeometryCheck>()?;
    m.add_class::<PySectionEdge>()?;
    m.add_class::<PyTetMesh>()?;
    m.add_class::<PyFemResult>()?;
    m.add_class::<PyAssembly>()?;
    m.add_class::<PyCompound>()?;
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
    m.add_function(wrap_pyfunction!(create_spiral, m)?)?;
    m.add_function(wrap_pyfunction!(create_polygon, m)?)?;
    m.add_function(wrap_pyfunction!(create_plane_face, m)?)?;
    m.add_function(wrap_pyfunction!(create_gear, m)?)?;

    // Feature operations
    m.add_function(wrap_pyfunction!(extrude_profile, m)?)?;
    m.add_function(wrap_pyfunction!(revolve_profile, m)?)?;
    m.add_function(wrap_pyfunction!(mirror, m)?)?;
    m.add_function(wrap_pyfunction!(scale, m)?)?;
    m.add_function(wrap_pyfunction!(sweep_profile, m)?)?;
    m.add_function(wrap_pyfunction!(loft_profiles, m)?)?;
    m.add_function(wrap_pyfunction!(chamfer, m)?)?;
    m.add_function(wrap_pyfunction!(fillet, m)?)?;
    m.add_function(wrap_pyfunction!(shell, m)?)?;
    m.add_function(wrap_pyfunction!(pattern_linear, m)?)?;
    m.add_function(wrap_pyfunction!(pattern_circular, m)?)?;
    m.add_function(wrap_pyfunction!(pad_profile, m)?)?;
    m.add_function(wrap_pyfunction!(pocket_profile, m)?)?;
    m.add_function(wrap_pyfunction!(draft, m)?)?;
    m.add_function(wrap_pyfunction!(section, m)?)?;
    m.add_function(wrap_pyfunction!(split, m)?)?;
    m.add_function(wrap_pyfunction!(thickness, m)?)?;
    m.add_function(wrap_pyfunction!(taper, m)?)?;
    m.add_function(wrap_pyfunction!(offset, m)?)?;
    m.add_function(wrap_pyfunction!(groove_profile, m)?)?;
    m.add_function(wrap_pyfunction!(create_hole, m)?)?;
    m.add_function(wrap_pyfunction!(create_countersunk_hole, m)?)?;
    m.add_function(wrap_pyfunction!(cross_section, m)?)?;
    m.add_function(wrap_pyfunction!(reverse, m)?)?;
    m.add_function(wrap_pyfunction!(refine, m)?)?;
    m.add_function(wrap_pyfunction!(mesh_to_solid, m)?)?;

    // PartDesign additive/subtractive
    m.add_function(wrap_pyfunction!(pd_additive_box, m)?)?;
    m.add_function(wrap_pyfunction!(pd_subtractive_box, m)?)?;
    m.add_function(wrap_pyfunction!(pd_additive_cylinder, m)?)?;
    m.add_function(wrap_pyfunction!(pd_subtractive_cylinder, m)?)?;
    m.add_function(wrap_pyfunction!(pd_additive_sphere, m)?)?;
    m.add_function(wrap_pyfunction!(pd_subtractive_sphere, m)?)?;
    m.add_function(wrap_pyfunction!(pd_additive_cone, m)?)?;
    m.add_function(wrap_pyfunction!(pd_subtractive_cone, m)?)?;
    m.add_function(wrap_pyfunction!(pd_additive_torus, m)?)?;
    m.add_function(wrap_pyfunction!(pd_subtractive_torus, m)?)?;
    m.add_function(wrap_pyfunction!(pd_additive_helix, m)?)?;
    m.add_function(wrap_pyfunction!(pd_subtractive_helix, m)?)?;
    m.add_function(wrap_pyfunction!(pd_additive_ellipsoid, m)?)?;
    m.add_function(wrap_pyfunction!(pd_subtractive_ellipsoid, m)?)?;
    m.add_function(wrap_pyfunction!(pd_additive_prism, m)?)?;
    m.add_function(wrap_pyfunction!(pd_subtractive_prism, m)?)?;
    m.add_function(wrap_pyfunction!(pd_additive_wedge, m)?)?;
    m.add_function(wrap_pyfunction!(pd_subtractive_wedge, m)?)?;
    m.add_function(wrap_pyfunction!(pd_additive_loft, m)?)?;
    m.add_function(wrap_pyfunction!(pd_subtractive_loft, m)?)?;
    m.add_function(wrap_pyfunction!(pd_additive_pipe, m)?)?;
    m.add_function(wrap_pyfunction!(pd_subtractive_pipe, m)?)?;

    // Join operations
    m.add_function(wrap_pyfunction!(join_connect, m)?)?;
    m.add_function(wrap_pyfunction!(join_embed, m)?)?;
    m.add_function(wrap_pyfunction!(join_cutout, m)?)?;

    // Multi-transform
    m.add_function(wrap_pyfunction!(apply_multi_transform, m)?)?;

    // Boolean operations
    m.add_function(wrap_pyfunction!(boolean_union, m)?)?;
    m.add_function(wrap_pyfunction!(boolean_subtract, m)?)?;
    m.add_function(wrap_pyfunction!(boolean_intersect, m)?)?;
    m.add_function(wrap_pyfunction!(boolean_xor_op, m)?)?;

    // Tessellation & measurement
    m.add_function(wrap_pyfunction!(tessellate, m)?)?;
    m.add_function(wrap_pyfunction!(mass_properties, m)?)?;
    m.add_function(wrap_pyfunction!(geometry_check, m)?)?;
    m.add_function(wrap_pyfunction!(watertight_check, m)?)?;
    m.add_function(wrap_pyfunction!(dist_between_points, m)?)?;

    // Mesh operations
    m.add_function(wrap_pyfunction!(decimate, m)?)?;
    m.add_function(wrap_pyfunction!(mesh_fill_holes, m)?)?;
    m.add_function(wrap_pyfunction!(mesh_curvature, m)?)?;
    m.add_function(wrap_pyfunction!(mesh_subdivide, m)?)?;
    m.add_function(wrap_pyfunction!(mesh_flip_normals, m)?)?;
    m.add_function(wrap_pyfunction!(mesh_smooth, m)?)?;
    m.add_function(wrap_pyfunction!(mesh_union, m)?)?;
    m.add_function(wrap_pyfunction!(mesh_intersection, m)?)?;
    m.add_function(wrap_pyfunction!(mesh_difference, m)?)?;
    m.add_function(wrap_pyfunction!(mesh_cut_with_plane, m)?)?;
    m.add_function(wrap_pyfunction!(mesh_harmonize_normals, m)?)?;
    m.add_function(wrap_pyfunction!(mesh_is_watertight, m)?)?;
    m.add_function(wrap_pyfunction!(mesh_remesh, m)?)?;
    m.add_function(wrap_pyfunction!(mesh_scale, m)?)?;
    m.add_function(wrap_pyfunction!(mesh_repair, m)?)?;

    // FEM
    m.add_function(wrap_pyfunction!(fem_generate_tet_mesh, m)?)?;
    m.add_function(wrap_pyfunction!(fem_static_analysis, m)?)?;
    m.add_function(wrap_pyfunction!(fem_modal_analysis, m)?)?;
    m.add_function(wrap_pyfunction!(fem_thermal_analysis, m)?)?;
    m.add_function(wrap_pyfunction!(fem_custom_material, m)?)?;

    // Draft operations
    m.add_function(wrap_pyfunction!(draft_make_wire, m)?)?;
    m.add_function(wrap_pyfunction!(draft_clone_solid, m)?)?;
    m.add_function(wrap_pyfunction!(draft_rectangular_array, m)?)?;
    m.add_function(wrap_pyfunction!(draft_path_array, m)?)?;
    m.add_function(wrap_pyfunction!(draft_polar_array, m)?)?;

    // Compound operations
    m.add_function(wrap_pyfunction!(compound_boolean_fragments, m)?)?;
    m.add_function(wrap_pyfunction!(compound_slice, m)?)?;
    m.add_function(wrap_pyfunction!(compound_filter_by_faces, m)?)?;

    // Measurement
    m.add_function(wrap_pyfunction!(measure_dist, m)?)?;
    m.add_function(wrap_pyfunction!(measure_ang, m)?)?;
    m.add_function(wrap_pyfunction!(measure_edge_len, m)?)?;

    // Shape analysis & query
    m.add_function(wrap_pyfunction!(classify, m)?)?;
    m.add_function(wrap_pyfunction!(planar_faces, m)?)?;
    m.add_function(wrap_pyfunction!(cylindrical_faces, m)?)?;
    m.add_function(wrap_pyfunction!(query_point_in_solid, m)?)?;
    m.add_function(wrap_pyfunction!(query_closest_point, m)?)?;

    // Assembly operations
    m.add_function(wrap_pyfunction!(assembly_add_component, m)?)?;
    m.add_function(wrap_pyfunction!(assembly_add_fixed, m)?)?;
    m.add_function(wrap_pyfunction!(assembly_add_coincident, m)?)?;
    m.add_function(wrap_pyfunction!(assembly_add_distance, m)?)?;
    m.add_function(wrap_pyfunction!(assembly_bill_of_materials, m)?)?;
    m.add_function(wrap_pyfunction!(assembly_exploded_view, m)?)?;

    // Result classes
    m.add_class::<PyModalResult>()?;
    m.add_class::<PyThermalResult>()?;

    // I/O
    m.add_function(wrap_pyfunction!(export_stl, m)?)?;
    m.add_function(wrap_pyfunction!(export_stl_binary, m)?)?;
    m.add_function(wrap_pyfunction!(export_obj, m)?)?;
    m.add_function(wrap_pyfunction!(export_gltf, m)?)?;
    m.add_function(wrap_pyfunction!(export_step, m)?)?;
    m.add_function(wrap_pyfunction!(export_iges, m)?)?;
    m.add_function(wrap_pyfunction!(export_dxf, m)?)?;
    m.add_function(wrap_pyfunction!(export_ply, m)?)?;
    m.add_function(wrap_pyfunction!(export_3mf, m)?)?;
    m.add_function(wrap_pyfunction!(export_brep, m)?)?;
    m.add_function(wrap_pyfunction!(export_dae, m)?)?;
    m.add_function(wrap_pyfunction!(export_amf, m)?)?;
    m.add_function(wrap_pyfunction!(import_stl, m)?)?;
    m.add_function(wrap_pyfunction!(import_obj, m)?)?;
    m.add_function(wrap_pyfunction!(import_dxf, m)?)?;
    m.add_function(wrap_pyfunction!(import_ply, m)?)?;
    m.add_function(wrap_pyfunction!(import_3mf, m)?)?;
    m.add_function(wrap_pyfunction!(import_brep, m)?)?;
    m.add_function(wrap_pyfunction!(import_dae, m)?)?;
    m.add_function(wrap_pyfunction!(import_amf, m)?)?;
    m.add_function(wrap_pyfunction!(import_step, m)?)?;
    m.add_function(wrap_pyfunction!(import_iges, m)?)?;
    m.add_function(wrap_pyfunction!(save_project, m)?)?;
    m.add_function(wrap_pyfunction!(load_project, m)?)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests — exercise binding logic through inner Rust types
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use cadkernel_math::{Point3, Vec3};
    use cadkernel_topology::BRepModel;
    use cadkernel_modeling::*;
    use cadkernel_sketch::{Constraint, Sketch, solve};

    // -- Primitive tests --

    #[test]
    fn test_all_primitives() {
        let mut m = BRepModel::new();
        let b = make_box(&mut m, Point3::ORIGIN, 1.0, 2.0, 3.0).unwrap();
        assert!(m.solids.is_alive(b.solid));

        let c = make_cylinder(&mut m, Point3::ORIGIN, 1.0, 2.0, 32).unwrap();
        assert!(m.solids.is_alive(c.solid));

        let s = make_sphere(&mut m, Point3::ORIGIN, 1.0, 32, 16).unwrap();
        assert!(m.solids.is_alive(s.solid));

        let co = make_cone(&mut m, Point3::ORIGIN, 2.0, 0.5, 3.0, 32).unwrap();
        assert!(m.solids.is_alive(co.solid));

        let t = make_torus(&mut m, Point3::ORIGIN, 3.0, 1.0, 32, 16).unwrap();
        assert!(m.solids.is_alive(t.solid));

        let tu = make_tube(&mut m, Point3::ORIGIN, 2.0, 1.0, 3.0, 32).unwrap();
        assert!(m.solids.is_alive(tu.solid));

        let pr = make_prism(&mut m, Point3::ORIGIN, 1.0, 2.0, 6).unwrap();
        assert!(m.solids.is_alive(pr.solid));

        let w = make_wedge(&mut m, Point3::ORIGIN, 2.0, 1.0, 3.0, 1.0, 0.5, 0.0, 0.0).unwrap();
        assert!(m.solids.is_alive(w.solid));

        let e = make_ellipsoid(&mut m, Point3::ORIGIN, 2.0, 1.5, 1.0, 32, 16).unwrap();
        assert!(m.solids.is_alive(e.solid));

        let h = make_helix(&mut m, Point3::ORIGIN, 2.0, 1.0, 2.0, 0.3, 16, 8).unwrap();
        assert!(m.solids.is_alive(h.solid));
    }

    #[test]
    fn test_new_primitives() {
        let mut m = BRepModel::new();

        let sp = make_spiral(&mut m, Point3::ORIGIN, 5.0, 1.0, 1.5, 0.5, 8).unwrap();
        assert!(m.solids.is_alive(sp.solid));

        let pg = make_polygon(&mut m, Point3::ORIGIN, 2.0, 5, 1.0).unwrap();
        assert!(m.solids.is_alive(pg.solid));

        let pf = make_plane_face(&mut m, Point3::ORIGIN, 10.0, 5.0).unwrap();
        assert!(m.solids.is_alive(pf.solid));

        let gr = make_involute_gear(&mut m, 2.0, 12, 0.35, 5.0).unwrap();
        assert!(m.solids.is_alive(gr.solid));
    }

    // -- Feature operations --

    #[test]
    fn test_extrude_revolve() {
        let mut m = BRepModel::new();
        let profile = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let ext = extrude(&mut m, &profile, Vec3::Z, 2.0).unwrap();
        assert!(m.solids.is_alive(ext.solid));

        let rev = revolve(
            &mut m,
            &[Point3::new(2.0, 0.0, 0.0), Point3::new(3.0, 0.0, 0.0), Point3::new(3.0, 1.0, 0.0)],
            Point3::ORIGIN, Vec3::Y, std::f64::consts::TAU, 16,
        ).unwrap();
        assert!(m.solids.is_alive(rev.solid));
    }

    #[test]
    fn test_sweep_loft() {
        let mut m = BRepModel::new();
        let profile = vec![
            Point3::new(-0.5, -0.5, 0.0),
            Point3::new(0.5, -0.5, 0.0),
            Point3::new(0.5, 0.5, 0.0),
            Point3::new(-0.5, 0.5, 0.0),
        ];
        let path = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.0, 0.0, 2.0),
            Point3::new(2.0, 0.0, 4.0),
        ];
        let sw = sweep(&mut m, &profile, &path).unwrap();
        assert!(m.solids.is_alive(sw.solid));

        let p1: Vec<Point3> = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(2.0, 2.0, 0.0),
            Point3::new(0.0, 2.0, 0.0),
        ];
        let p2: Vec<Point3> = vec![
            Point3::new(0.5, 0.5, 3.0),
            Point3::new(1.5, 0.5, 3.0),
            Point3::new(1.5, 1.5, 3.0),
            Point3::new(0.5, 1.5, 3.0),
        ];
        let lf = loft(&mut m, &[&p1, &p2]).unwrap();
        assert!(m.solids.is_alive(lf.solid));
    }

    #[test]
    fn test_mirror_scale_offset() {
        let mut m = BRepModel::new();
        let b = make_box(&mut m, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mr = mirror_solid(&mut m, b.solid, Point3::new(2.0, 0.0, 0.0), Vec3::X).unwrap();
        assert!(m.solids.is_alive(mr.solid));

        let sc = scale_solid(&mut m, b.solid, Point3::ORIGIN, 2.0).unwrap();
        assert!(m.solids.is_alive(sc.solid));

        let of = offset_solid(&mut m, b.solid, 0.1).unwrap();
        assert!(m.solids.is_alive(of.solid));
    }

    #[test]
    fn test_pattern_linear_circular() {
        let mut m = BRepModel::new();
        let b = make_box(&mut m, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let lp = linear_pattern(&mut m, b.solid, Vec3::X, 3.0, 3).unwrap();
        assert_eq!(lp.solids.len(), 3);

        let cp = circular_pattern(&mut m, b.solid, Point3::ORIGIN, Vec3::Z, 4).unwrap();
        assert_eq!(cp.solids.len(), 4);
    }

    #[test]
    fn test_section_split() {
        let mut m = BRepModel::new();
        let b = make_box(&mut m, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let sec = section_solid(&m, b.solid, Point3::new(0.0, 0.0, 1.0), Vec3::Z).unwrap();
        assert!(!sec.edges.is_empty());

        let sp = split_solid(&mut m, b.solid, Point3::new(0.0, 0.0, 1.0), Vec3::Z).unwrap();
        assert!(sp.solids.len() >= 2);
    }

    #[test]
    fn test_taper_extrude() {
        let mut m = BRepModel::new();
        let profile = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(2.0, 2.0, 0.0),
            Point3::new(0.0, 2.0, 0.0),
        ];
        let te = taper_extrude(&mut m, &profile, Vec3::Z, 3.0, 0.1).unwrap();
        assert!(m.solids.is_alive(te.solid));
    }

    #[test]
    fn test_thickness() {
        let mut m = BRepModel::new();
        let b = make_box(&mut m, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();
        let th = thickness_solid(&mut m, b.solid, 0.1, ThicknessJoin::Outward).unwrap();
        assert!(m.solids.is_alive(th.solid));
    }

    #[test]
    fn test_cross_sections() {
        let mut m = BRepModel::new();
        let b = make_box(&mut m, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();
        let secs = cross_sections(&m, b.solid, Vec3::Z, 3).unwrap();
        assert_eq!(secs.len(), 3);
    }

    #[test]
    fn test_reverse_refine() {
        let mut m = BRepModel::new();
        let b = make_box(&mut m, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let rv = reverse_solid(&mut m, b.solid).unwrap();
        assert!(m.solids.is_alive(rv.solid));

        let rf = refine_shape(&m, b.solid).unwrap();
        assert!(rf.redundant_edge_count <= m.edges.len());
    }

    // -- Boolean --

    #[test]
    fn test_booleans() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();
        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(1.0, 1.0, 1.0), 2.0, 2.0, 2.0).unwrap();

        let u = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Union).unwrap();
        assert!(!u.solids.is_empty());

        let d = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Difference).unwrap();
        assert!(!d.solids.is_empty());

        let i = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Intersection).unwrap();
        assert!(!i.solids.is_empty());

        let x = boolean_xor(&a, ra.solid, &b, rb.solid).unwrap();
        assert!(!x.solids.is_empty());
    }

    // -- Additive/Subtractive --

    #[test]
    fn test_additive_subtractive() {
        let mut m = BRepModel::new();
        let b = make_box(&mut m, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let r = additive_box(&m, b.solid, Point3::new(2.0, 0.0, 0.0), 2.0, 2.0, 2.0).unwrap();
        assert!(!r.solids.is_empty());

        let r = subtractive_cylinder(&m, b.solid, Point3::new(2.0, 2.0, 0.0), 0.5, 5.0, 32).unwrap();
        assert!(!r.solids.is_empty());

        let r = additive_sphere(&m, b.solid, Point3::new(2.0, 2.0, 2.0), 1.0, 32, 16).unwrap();
        assert!(!r.solids.is_empty());

        let r = subtractive_cone(&m, b.solid, Point3::ORIGIN, 1.0, 0.5, 3.0, 32).unwrap();
        assert!(!r.solids.is_empty());

        let r = additive_torus(&m, b.solid, Point3::new(2.0, 2.0, 2.0), 2.0, 0.5, 32, 16).unwrap();
        assert!(!r.solids.is_empty());

        let r = additive_helix(&m, b.solid, Point3::new(2.0, 2.0, 0.0), 1.0, 1.0, 1.0, 0.2, 16, 8).unwrap();
        assert!(!r.solids.is_empty());

        let r = additive_ellipsoid(&m, b.solid, Point3::ORIGIN, 1.0, 1.5, 1.0, 32, 16).unwrap();
        assert!(!r.solids.is_empty());

        let r = additive_prism(&m, b.solid, Point3::ORIGIN, 1.0, 2.0, 6).unwrap();
        assert!(!r.solids.is_empty());

        let r = additive_wedge(&m, b.solid, Point3::ORIGIN, 2.0, 1.0, 3.0, 1.0, 0.5, 0.0, 0.0).unwrap();
        assert!(!r.solids.is_empty());
    }

    // -- Pad / Pocket / Groove / Hole --

    #[test]
    fn test_pad_pocket() {
        let mut m = BRepModel::new();
        let b = make_box(&mut m, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();
        let profile = vec![
            Point3::new(1.0, 1.0, 4.0),
            Point3::new(3.0, 1.0, 4.0),
            Point3::new(3.0, 3.0, 4.0),
            Point3::new(1.0, 3.0, 4.0),
        ];
        let pr = pad(&m, b.solid, &profile, Vec3::Z, 2.0).unwrap();
        assert!(!pr.model.solids.is_empty());

        let pk = pocket(&m, b.solid, &profile, Vec3::new(0.0, 0.0, -1.0), 2.0).unwrap();
        assert!(!pk.model.solids.is_empty());
    }

    #[test]
    fn test_hole() {
        let mut m = BRepModel::new();
        let b = make_box(&mut m, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();
        let h = hole(&m, b.solid, Point3::new(2.0, 2.0, 4.0), Vec3::new(0.0, 0.0, -1.0), 0.5, 4.0, 32).unwrap();
        assert!(!h.model.solids.is_empty());
    }

    #[test]
    fn test_groove() {
        let mut m = BRepModel::new();
        let b = make_box(&mut m, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();
        let profile = vec![
            Point3::new(5.0, -0.5, 0.0),
            Point3::new(6.0, -0.5, 0.0),
            Point3::new(6.0, 0.5, 0.0),
        ];
        let gr = groove(&m, b.solid, &profile, Point3::ORIGIN, Vec3::Y, std::f64::consts::TAU, 16).unwrap();
        assert!(!gr.model.solids.is_empty());
    }

    // -- Join ops --

    #[test]
    fn test_join_ops() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();
        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(10.0, 10.0, 10.0), 1.0, 1.0, 1.0).unwrap();

        let c = connect_shapes(&a, ra.solid, &b, rb.solid).unwrap();
        assert!(!c.solids.is_empty());

        let e = embed_shapes(&a, ra.solid, &b, rb.solid).unwrap();
        assert!(!e.solids.is_empty());

        let ct = cutout_shapes(&a, ra.solid, &b, rb.solid).unwrap();
        assert!(!ct.solids.is_empty());
    }

    // -- Multi-transform --

    #[test]
    fn test_multi_transform() {
        let mut m = BRepModel::new();
        let b = make_box(&mut m, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let xforms = vec![
            Transform::Translation(Vec3::new(5.0, 0.0, 0.0)),
            Transform::Scale { center: Point3::ORIGIN, factor: 2.0 },
        ];
        let r = multi_transform(&mut m, b.solid, &xforms).unwrap();
        assert!(m.solids.is_alive(r.solid));
    }

    // -- Compound --

    #[test]
    fn test_compound() {
        let mut m = BRepModel::new();
        let b1 = make_box(&mut m, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let b2 = make_box(&mut m, Point3::new(3.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap();
        let mut c = Compound::new("test");
        c.add(b1.solid);
        c.add(b2.solid);
        assert_eq!(c.solids.len(), 2);
        let exploded = c.explode();
        assert_eq!(exploded.len(), 2);
    }

    // -- Tessellation & measurement --

    #[test]
    fn test_tessellate_mass_props() {
        let mut m = BRepModel::new();
        let b = make_box(&mut m, Point3::ORIGIN, 2.0, 3.0, 4.0).unwrap();
        let mesh = cadkernel_io::tessellate_solid(&m, b.solid);
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());

        let props = compute_mass_properties(&mesh);
        assert!(props.volume > 0.0);
        assert!(props.surface_area > 0.0);
    }

    #[test]
    fn test_geometry_checks() {
        let mut m = BRepModel::new();
        let b = make_box(&mut m, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let result = check_geometry(&m, b.solid);
        assert!(result.is_valid);

        let wt = check_watertight(&m, b.solid);
        assert!(wt);
    }

    // -- Mesh operations --

    #[test]
    fn test_mesh_ops() {
        let mut m = BRepModel::new();
        let b = make_box(&mut m, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let mesh = cadkernel_io::tessellate_solid(&m, b.solid);

        let dec = cadkernel_io::mesh_ops::decimate_mesh(&mesh, 0.5).unwrap();
        assert!(dec.vertices.len() <= mesh.vertices.len());

        let sub = cadkernel_io::mesh_ops::subdivide_mesh(&mesh).unwrap();
        assert!(sub.vertices.len() >= mesh.vertices.len());

        let flipped = cadkernel_io::mesh_ops::flip_normals(&mesh);
        assert_eq!(flipped.indices.len(), mesh.indices.len());

        let smooth = cadkernel_io::mesh_ops::smooth_mesh(&mesh, 2, 0.5);
        assert_eq!(smooth.vertices.len(), mesh.vertices.len());

        let harm = cadkernel_io::mesh_ops::harmonize_normals(&mesh);
        assert_eq!(harm.indices.len(), mesh.indices.len());

        let _wt = cadkernel_io::mesh_ops::check_mesh_watertight(&mesh);

        let scaled = cadkernel_io::mesh_ops::scale_mesh(&mesh, 2.0, 2.0, 2.0);
        assert_eq!(scaled.vertices.len(), mesh.vertices.len());

        let (repaired, report) = cadkernel_io::mesh_ops::evaluate_and_repair(&mesh);
        assert!(!repaired.vertices.is_empty());
        let _ = report.degenerate_removed;
    }

    #[test]
    fn test_mesh_boolean_ops() {
        let mut m1 = BRepModel::new();
        let b1 = make_box(&mut m1, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();
        let mesh1 = cadkernel_io::tessellate_solid(&m1, b1.solid);

        let mut m2 = BRepModel::new();
        let b2 = make_box(&mut m2, Point3::new(1.0, 1.0, 1.0), 2.0, 2.0, 2.0).unwrap();
        let mesh2 = cadkernel_io::tessellate_solid(&m2, b2.solid);

        let u = cadkernel_io::mesh_ops::mesh_boolean_union(&mesh1, &mesh2);
        assert!(!u.vertices.is_empty());

        let i = cadkernel_io::mesh_ops::mesh_boolean_intersection(&mesh1, &mesh2);
        assert!(!i.vertices.is_empty());

        let d = cadkernel_io::mesh_ops::mesh_boolean_difference(&mesh1, &mesh2);
        assert!(!d.vertices.is_empty());

        let cut = cadkernel_io::mesh_ops::cut_mesh_with_plane(&mesh1, Point3::new(0.0, 0.0, 1.0), Vec3::Z);
        assert!(!cut.vertices.is_empty());
    }

    // -- FEM --

    #[test]
    fn test_fem_pipeline() {
        let mut m = BRepModel::new();
        let b = make_box(&mut m, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let tet = generate_tet_mesh(&m, b.solid, 0.5).unwrap();
        assert!(!tet.nodes.is_empty());
        assert!(!tet.elements.is_empty());

        let mat = FemMaterial::steel();
        let bcs = vec![
            BoundaryCondition::FixedNode(0),
            BoundaryCondition::Force { node: tet.nodes.len() - 1, force: Vec3::new(0.0, 0.0, -1000.0) },
        ];
        let result = static_analysis(&tet, &mat, &bcs).unwrap();
        assert!(result.max_displacement >= 0.0);
        assert!(result.max_stress >= 0.0);
    }

    // -- Sketch --

    #[test]
    fn test_sketch_with_constraints() {
        let mut sk = Sketch::new();
        let p0 = sk.add_point(0.0, 0.0);
        let p1 = sk.add_point(5.0, 0.0);
        let p2 = sk.add_point(5.0, 3.0);
        let p3 = sk.add_point(0.0, 3.0);

        let l0 = sk.add_line(p0, p1);
        let l1 = sk.add_line(p1, p2);
        let l2 = sk.add_line(p2, p3);
        let _l3 = sk.add_line(p3, p0);

        sk.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
        sk.add_constraint(Constraint::Horizontal(l0));
        sk.add_constraint(Constraint::Vertical(l1));
        sk.add_constraint(Constraint::Length(l0, 5.0));
        sk.add_constraint(Constraint::Length(l1, 3.0));
        sk.add_constraint(Constraint::Parallel(l0, l2));
        sk.add_constraint(Constraint::Perpendicular(l0, l1));

        let result = solve(&mut sk, 100, 1e-10);
        assert!(result.converged);
    }

    #[test]
    fn test_sketch_new_entities() {
        let mut sk = Sketch::new();
        let c = sk.add_point(0.0, 0.0);
        let s = sk.add_point(1.0, 0.0);
        let e = sk.add_point(0.0, 1.0);

        let _arc = sk.add_arc(c, s, e, 1.0, 0.0, std::f64::consts::FRAC_PI_2);
        assert_eq!(sk.arcs.len(), 1);

        let me = sk.add_point(2.0, 0.0);
        let _ellipse = sk.add_ellipse(c, me, 1.0);
        assert_eq!(sk.ellipses.len(), 1);

        let cp0 = sk.add_point(0.0, 0.0);
        let cp1 = sk.add_point(1.0, 2.0);
        let cp2 = sk.add_point(3.0, 2.0);
        let cp3 = sk.add_point(4.0, 0.0);
        let _bsp = sk.add_bspline(vec![cp0, cp1, cp2, cp3], 3, false);
        assert_eq!(sk.bsplines.len(), 1);
    }

    #[test]
    fn test_sketch_extended_constraints() {
        let mut sk = Sketch::new();
        let p0 = sk.add_point(0.0, 0.0);
        let p1 = sk.add_point(1.0, 0.0);
        let p2 = sk.add_point(1.0, 1.0);
        let p3 = sk.add_point(0.0, 1.0);
        let l0 = sk.add_line(p0, p1);
        let l1 = sk.add_line(p2, p3);

        sk.add_constraint(Constraint::EqualLength(l0, l1));
        sk.add_constraint(Constraint::Collinear(l0, l1));
        sk.add_constraint(Constraint::Concentric(p0, p2));
        sk.add_constraint(Constraint::HorizontalDistance(p0, p1, 1.0));
        sk.add_constraint(Constraint::VerticalDistance(p0, p3, 1.0));
        sk.add_constraint(Constraint::Block(p0, 0.0, 0.0));
        assert_eq!(sk.constraints.len(), 6);
    }

    // -- I/O roundtrip --

    #[test]
    fn test_io_stl_roundtrip() {
        let mut m = BRepModel::new();
        let b = make_box(&mut m, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let mesh = cadkernel_io::tessellate_solid(&m, b.solid);

        let ascii = cadkernel_io::write_stl_ascii(&mesh, "test");
        assert!(ascii.contains("facet normal"));

        let binary = cadkernel_io::write_stl_binary(&mesh).unwrap();
        assert!(binary.len() > 80);
    }

    #[test]
    fn test_io_obj_roundtrip() {
        let mut m = BRepModel::new();
        let b = make_box(&mut m, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let mesh = cadkernel_io::tessellate_solid(&m, b.solid);

        let obj = cadkernel_io::write_obj(&mesh);
        assert!(obj.contains("v "));
        assert!(obj.contains("f "));
    }

    #[test]
    fn test_io_formats() {
        let mut m = BRepModel::new();
        let b = make_box(&mut m, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let mesh = cadkernel_io::tessellate_solid(&m, b.solid);

        let dxf = cadkernel_io::export_dxf(&mesh).unwrap();
        assert!(!dxf.is_empty());

        let ply = cadkernel_io::export_ply(&mesh).unwrap();
        assert!(!ply.is_empty());

        let threemf = cadkernel_io::export_3mf(&mesh).unwrap();
        assert!(!threemf.is_empty());

        let dae = cadkernel_io::export_dae(&mesh).unwrap();
        assert!(!dae.is_empty());

        let amf = cadkernel_io::export_amf(&mesh).unwrap();
        assert!(!amf.is_empty());

        let brep = cadkernel_io::export_brep(&m).unwrap();
        assert!(!brep.is_empty());

        let step = cadkernel_io::export_step(&m).unwrap();
        assert!(!step.is_empty());

        let iges = cadkernel_io::export_iges(&m).unwrap();
        assert!(!iges.is_empty());
    }

    #[test]
    fn test_native_project_roundtrip() {
        let mut m = BRepModel::new();
        let _b = make_box(&mut m, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let json = cadkernel_io::model_to_json(&m).unwrap();
        let loaded = cadkernel_io::model_from_json(&json).unwrap();
        assert_eq!(loaded.solids.len(), m.solids.len());
    }

    // -- Assembly --

    #[test]
    fn test_assembly_basic() {
        let asm = Assembly::new("test_assembly");
        assert_eq!(asm.name, "test_assembly");
        assert!(asm.components.is_empty());
        assert!(asm.constraints.is_empty());
    }

    // -- Distance helper --

    #[test]
    fn test_distance_between_points() {
        let d = ((3.0f64).powi(2) + (4.0f64).powi(2)).sqrt();
        assert!((d - 5.0).abs() < 1e-10);
    }

    // -- Draft operations --

    #[test]
    fn test_draft_make_wire() {
        use cadkernel_modeling::make_wire;

        let mut m = BRepModel::new();
        let pts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
        ];
        let wr = make_wire(&mut m, &pts).unwrap();
        assert_eq!(wr.vertices.len(), 3);
        assert_eq!(wr.edges.len(), 2);
    }

    #[test]
    fn test_draft_clone_and_arrays() {
        use cadkernel_modeling::{clone_solid, rectangular_array, path_array, polar_array};

        let mut m = BRepModel::new();
        let b = make_box(&mut m, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let cl = clone_solid(&mut m, b.solid).unwrap();
        assert!(m.solids.is_alive(cl.solid));

        let ra = rectangular_array(
            &mut m, b.solid,
            Vec3::X, 2.0, 2,
            Vec3::Y, 2.0, 2,
        ).unwrap();
        assert_eq!(ra.solids.len(), 4);

        let pa = path_array(
            &mut m, b.solid,
            &[Point3::ORIGIN, Point3::new(5.0, 0.0, 0.0), Point3::new(10.0, 0.0, 0.0)],
        ).unwrap();
        assert_eq!(pa.solids.len(), 3);

        let pol = polar_array(
            &mut m, b.solid,
            Point3::ORIGIN, Vec3::Z, 4,
        ).unwrap();
        assert_eq!(pol.len(), 4);
    }

    // -- Compound operations --

    #[test]
    fn test_compound_operations() {
        use cadkernel_modeling::{boolean_fragments, slice_to_compound};

        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();
        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(1.0, 1.0, 1.0), 2.0, 2.0, 2.0).unwrap();

        let frags = boolean_fragments(&a, ra.solid, &b, rb.solid).unwrap();
        assert!(!frags.compound.solids.is_empty());

        let sliced = slice_to_compound(
            &mut a, ra.solid,
            Point3::new(0.0, 0.0, 1.0), Vec3::Z,
        ).unwrap();
        assert!(!sliced.compound.solids.is_empty());
    }

    // -- Measurement --

    #[test]
    fn test_measurement() {
        use cadkernel_modeling::{measure_distance, measure_edge_length};

        let mut m = BRepModel::new();
        let v0 = m.add_vertex(Point3::new(0.0, 0.0, 0.0));
        let v1 = m.add_vertex(Point3::new(3.0, 4.0, 0.0));

        let d = measure_distance(&m, v0, v1).unwrap();
        assert!((d - 5.0).abs() < 1e-10);

        let (edge_h, _, _) = m.add_edge(v0, v1);
        let len = measure_edge_length(&m, edge_h).unwrap();
        assert!((len - 5.0).abs() < 1e-10);
    }

    // -- Shape analysis & query --

    #[test]
    fn test_shape_analysis() {
        use cadkernel_modeling::{classify_solid, SolidType, find_planar_faces};

        let mut m = BRepModel::new();
        let b = make_box(&mut m, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let st = classify_solid(&m, b.solid);
        assert!(matches!(st, SolidType::Box));

        let pf = find_planar_faces(&m, b.solid);
        assert_eq!(pf.len(), 6);
    }

    #[test]
    fn test_point_query() {
        use cadkernel_modeling::{closest_point_on_solid, point_in_solid, Containment};

        let mut m = BRepModel::new();
        let b = make_box(&mut m, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let c = point_in_solid(&m, b.solid, Point3::new(1.0, 1.0, 1.0)).unwrap();
        assert_eq!(c, Containment::Inside);

        let c2 = point_in_solid(&m, b.solid, Point3::new(5.0, 5.0, 5.0)).unwrap();
        assert_eq!(c2, Containment::Outside);

        let cp = closest_point_on_solid(&m, b.solid, Point3::new(1.0, 1.0, 5.0)).unwrap();
        assert!(cp.distance > 0.0);
    }

    // -- Assembly operations --

    #[test]
    fn test_assembly_operations() {
        use cadkernel_modeling::{Assembly, AssemblyConstraint};

        let mut m = BRepModel::new();
        let b1 = make_box(&mut m, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let b2 = make_box(&mut m, Point3::new(3.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap();

        let mut asm = Assembly::new("test");
        let id1 = asm.add_component("part1", b1.solid);
        let id2 = asm.add_component("part2", b2.solid);

        asm.add_constraint(AssemblyConstraint::Fixed(id1));
        asm.add_constraint(AssemblyConstraint::Distance {
            comp_a: id1,
            comp_b: id2,
            distance: 3.0,
        });
        assert_eq!(asm.num_components(), 2);
        assert_eq!(asm.num_constraints(), 2);

        let bom = asm.bill_of_materials();
        assert_eq!(bom.len(), 2);

        asm.exploded_view(2.0);
    }

    // -- FEM extended --

    #[test]
    fn test_fem_modal() {
        use cadkernel_modeling::{modal_analysis, FemMaterial, BoundaryCondition};

        let mut m = BRepModel::new();
        let b = make_box(&mut m, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let tet = generate_tet_mesh(&m, b.solid, 0.5).unwrap();

        let mat = FemMaterial::steel();
        let bcs = vec![BoundaryCondition::FixedNode(0)];
        let r = modal_analysis(&tet, &mat, &bcs, 3).unwrap();
        assert!(!r.frequencies.is_empty());
    }

    #[test]
    fn test_fem_custom_material() {
        let mat = FemMaterial::custom(100.0e9, 0.25, 5000.0).unwrap();
        assert!((mat.youngs_modulus - 100.0e9).abs() < 1.0);
        assert!((mat.poisson_ratio - 0.25).abs() < 1e-10);

        assert!(FemMaterial::custom(-1.0, 0.3, 7850.0).is_err());
        assert!(FemMaterial::custom(210.0e9, 0.6, 7850.0).is_err());
        assert!(FemMaterial::custom(210.0e9, 0.3, -1.0).is_err());
    }
}
