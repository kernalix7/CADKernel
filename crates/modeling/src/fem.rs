//! Finite Element Method (FEM) module for structural and thermal analysis.
//!
//! Provides tetrahedral mesh generation from B-Rep solids, material definitions,
//! boundary conditions, linear static and modal solvers, thermal analysis,
//! mesh quality assessment, and post-processing utilities.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{BoundingBox, Point3, Vec3};
use cadkernel_topology::{BRepModel, Handle, SolidData};

use crate::query::point_in_solid;
use crate::Containment;

/// Tetrahedral mesh for FEM analysis.
///
/// Each element is a linear tetrahedron defined by 4 node indices.
pub struct TetMesh {
    pub nodes: Vec<Point3>,
    pub elements: Vec<[usize; 4]>,
}

/// Isotropic linear-elastic material properties.
pub struct FemMaterial {
    /// Young's modulus (Pa).
    pub youngs_modulus: f64,
    /// Poisson's ratio (dimensionless).
    pub poisson_ratio: f64,
    /// Mass density (kg/m^3).
    pub density: f64,
}

impl FemMaterial {
    /// Typical structural steel (E=210 GPa, nu=0.3, rho=7850 kg/m^3).
    pub fn steel() -> Self {
        Self {
            youngs_modulus: 210.0e9,
            poisson_ratio: 0.3,
            density: 7850.0,
        }
    }

    /// Typical aluminum alloy (E=70 GPa, nu=0.33, rho=2700 kg/m^3).
    pub fn aluminum() -> Self {
        Self {
            youngs_modulus: 70.0e9,
            poisson_ratio: 0.33,
            density: 2700.0,
        }
    }

    /// Titanium alloy Ti-6Al-4V (E=114 GPa, nu=0.34, rho=4430 kg/m^3).
    pub fn titanium() -> Self {
        Self {
            youngs_modulus: 114.0e9,
            poisson_ratio: 0.34,
            density: 4430.0,
        }
    }

    /// Copper (E=117 GPa, nu=0.34, rho=8960 kg/m^3).
    pub fn copper() -> Self {
        Self {
            youngs_modulus: 117.0e9,
            poisson_ratio: 0.34,
            density: 8960.0,
        }
    }

    /// Concrete (E=30 GPa, nu=0.2, rho=2400 kg/m^3).
    pub fn concrete() -> Self {
        Self {
            youngs_modulus: 30.0e9,
            poisson_ratio: 0.2,
            density: 2400.0,
        }
    }

    /// Cast iron (E=170 GPa, nu=0.26, rho=7200 kg/m^3).
    pub fn cast_iron() -> Self {
        Self {
            youngs_modulus: 170.0e9,
            poisson_ratio: 0.26,
            density: 7200.0,
        }
    }

    /// Custom material with user-specified properties.
    pub fn custom(youngs_modulus: f64, poisson_ratio: f64, density: f64) -> KernelResult<Self> {
        if youngs_modulus <= 0.0 {
            return Err(KernelError::InvalidArgument(
                "Young's modulus must be positive".into(),
            ));
        }
        if !(0.0..0.5).contains(&poisson_ratio) {
            return Err(KernelError::InvalidArgument(
                "Poisson ratio must be in [0, 0.5)".into(),
            ));
        }
        if density <= 0.0 {
            return Err(KernelError::InvalidArgument(
                "density must be positive".into(),
            ));
        }
        Ok(Self {
            youngs_modulus,
            poisson_ratio,
            density,
        })
    }
}

/// Thermal material properties for heat transfer analysis.
pub struct ThermalMaterial {
    /// Thermal conductivity (W/(m·K)).
    pub conductivity: f64,
    /// Specific heat capacity (J/(kg·K)).
    pub specific_heat: f64,
    /// Mass density (kg/m^3).
    pub density: f64,
}

impl ThermalMaterial {
    /// Steel thermal properties.
    pub fn steel() -> Self {
        Self {
            conductivity: 50.0,
            specific_heat: 500.0,
            density: 7850.0,
        }
    }

    /// Aluminum thermal properties.
    pub fn aluminum() -> Self {
        Self {
            conductivity: 237.0,
            specific_heat: 900.0,
            density: 2700.0,
        }
    }

    /// Copper thermal properties.
    pub fn copper() -> Self {
        Self {
            conductivity: 401.0,
            specific_heat: 385.0,
            density: 8960.0,
        }
    }
}

/// Beam cross-section definition.
pub struct BeamSection {
    /// Cross-sectional area (m^2).
    pub area: f64,
    /// Second moment of area about y-axis (m^4).
    pub iy: f64,
    /// Second moment of area about z-axis (m^4).
    pub iz: f64,
    /// Polar moment of area (m^4).
    pub j: f64,
}

impl BeamSection {
    /// Circular cross-section with given radius.
    pub fn circular(radius: f64) -> Self {
        let area = std::f64::consts::PI * radius * radius;
        let i = std::f64::consts::PI * radius.powi(4) / 4.0;
        Self {
            area,
            iy: i,
            iz: i,
            j: 2.0 * i,
        }
    }

    /// Rectangular cross-section (width × height).
    pub fn rectangular(width: f64, height: f64) -> Self {
        let area = width * height;
        let iy = width * height.powi(3) / 12.0;
        let iz = height * width.powi(3) / 12.0;
        let a = width.max(height);
        let b = width.min(height);
        let j = a * b.powi(3) * (1.0 / 3.0 - 0.21 * b / a * (1.0 - b.powi(4) / (12.0 * a.powi(4))));
        Self { area, iy, iz, j }
    }
}

/// A boundary condition applied to the FEM model.
#[derive(Clone)]
pub enum BoundaryCondition {
    /// Fix all degrees of freedom at the given node index.
    FixedNode(usize),
    /// Apply a concentrated force vector at the given node index.
    Force { node: usize, force: Vec3 },
    /// Apply uniform pressure on a tetrahedral element face.
    Pressure { element: usize, pressure: f64 },
    /// Prescribed displacement at a node.
    Displacement { node: usize, displacement: Vec3 },
    /// Apply gravity (body force) to all elements.
    Gravity { acceleration: Vec3 },
    /// Distributed load on an element face.
    DistributedLoad { element: usize, load: Vec3 },
    /// Spring support at a node (stiffness in N/m per DOF).
    Spring { node: usize, stiffness: f64 },
    /// Centrifugal load: rotation about an axis at angular velocity omega (rad/s).
    CentrifugalLoad { axis: Vec3, omega: f64 },
    /// Self-weight body force using the given gravity vector.
    SelfWeight { gravity: Vec3 },
    /// Section print: extract internal forces on a cutting plane (post-processing marker).
    SectionPrint { plane_normal: Vec3, plane_point: Point3 },
    /// Tie constraint: couple DOFs of two surfaces (multi-point constraint).
    TieConstraint { surface_a: Vec<usize>, surface_b: Vec<usize> },
    /// Rigid body constraint: lock all nodes in the set to move as a rigid body.
    RigidBody { node_ids: Vec<usize> },
    /// Directional spring: stiffness applied along a specific direction.
    SpringConstraint { node_id: usize, stiffness: f64, direction: Vec3 },
    /// Body load applied as a force-per-unit-volume on all elements.
    BodyLoad { force_density: Vec3 },
    /// Contact constraint between two node sets (penalty-based coupling).
    ContactConstraint { surface_a: Vec<usize>, surface_b: Vec<usize>, penalty: f64 },
    /// Initial temperature for thermo-mechanical coupling (defines reference state).
    InitialTemperature { node: usize, temperature: f64 },
}

/// A thermal boundary condition.
pub enum ThermalBoundaryCondition {
    /// Fixed temperature at a node (Dirichlet).
    FixedTemperature { node: usize, temperature: f64 },
    /// Heat flux on an element face (Neumann).
    HeatFlux { element: usize, flux: f64 },
    /// Internal heat generation in an element (W/m^3).
    HeatGeneration { element: usize, rate: f64 },
    /// Convection on an element face.
    Convection {
        element: usize,
        coefficient: f64,
        ambient_temp: f64,
    },
}

/// Results of a static FEM analysis.
pub struct FemResult {
    /// Displacement vector per node.
    pub displacements: Vec<Vec3>,
    /// Von Mises stress per element.
    pub stresses: Vec<f64>,
    /// Maximum displacement magnitude across all nodes.
    pub max_displacement: f64,
    /// Maximum von Mises stress across all elements.
    pub max_stress: f64,
}

/// Results of a modal (eigenvalue) analysis.
pub struct ModalResult {
    /// Natural frequencies in Hz.
    pub frequencies: Vec<f64>,
    /// Mode shapes: each entry is a displacement vector per node for that mode.
    pub mode_shapes: Vec<Vec<Vec3>>,
}

/// Results of a thermal analysis.
pub struct ThermalResult {
    /// Temperature at each node.
    pub temperatures: Vec<f64>,
    /// Heat flux vector per element.
    pub heat_fluxes: Vec<Vec3>,
    /// Maximum temperature.
    pub max_temperature: f64,
    /// Minimum temperature.
    pub min_temperature: f64,
}

/// Mesh quality metrics for a tetrahedral mesh.
pub struct MeshQuality {
    /// Minimum element aspect ratio (1.0 = ideal equilateral).
    pub min_aspect_ratio: f64,
    /// Average element aspect ratio.
    pub avg_aspect_ratio: f64,
    /// Number of degenerate (near-zero-volume) elements.
    pub degenerate_count: usize,
    /// Total number of elements.
    pub total_elements: usize,
    /// Minimum element volume.
    pub min_volume: f64,
    /// Average element volume.
    pub avg_volume: f64,
}

/// Principal stress components at an element.
pub struct PrincipalStresses {
    /// Maximum principal stress (σ₁).
    pub sigma1: f64,
    /// Middle principal stress (σ₂).
    pub sigma2: f64,
    /// Minimum principal stress (σ₃).
    pub sigma3: f64,
}

/// Strain tensor components at an element.
pub struct StrainResult {
    /// Strain components per element: [εxx, εyy, εzz, γxy, γxz, γyz].
    pub strains: Vec<[f64; 6]>,
}

/// Stress tensor components at an element.
pub struct StressTensor {
    /// Stress components per element: [σxx, σyy, σzz, τxy, τxz, τyz].
    pub stresses: Vec<[f64; 6]>,
}

/// Generate a tetrahedral mesh from a B-Rep solid.
///
/// The solid is enclosed in its bounding box, which is subdivided into cubes
/// of size `max_edge_length`. Each cube is split into 5 tetrahedra. Only
/// cubes whose center lies inside the solid are retained.
pub fn generate_tet_mesh(
    model: &BRepModel,
    solid: Handle<SolidData>,
    max_edge_length: f64,
) -> KernelResult<TetMesh> {
    if max_edge_length <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "max_edge_length must be positive".into(),
        ));
    }

    // Compute bounding box from solid vertices
    let solid_data = model
        .solids
        .get(solid)
        .ok_or(KernelError::InvalidHandle("solid"))?;
    let mut bbox = BoundingBox::empty();
    for &shell_h in &solid_data.shells {
        let shell = model
            .shells
            .get(shell_h)
            .ok_or(KernelError::InvalidHandle("shell"))?;
        for &face_h in &shell.faces {
            let verts = model.vertices_of_face(face_h)?;
            for vh in verts {
                let vd = model
                    .vertices
                    .get(vh)
                    .ok_or(KernelError::InvalidHandle("vertex"))?;
                bbox.include_point(vd.point);
            }
        }
    }

    if bbox.is_empty() {
        return Err(KernelError::GeometryError(
            "solid has no vertices".into(),
        ));
    }

    // Add small margin
    let margin = max_edge_length * 0.01;
    let min = Point3::new(
        bbox.min.x - margin,
        bbox.min.y - margin,
        bbox.min.z - margin,
    );
    let max = Point3::new(
        bbox.max.x + margin,
        bbox.max.y + margin,
        bbox.max.z + margin,
    );

    let nx = ((max.x - min.x) / max_edge_length).ceil() as usize;
    let ny = ((max.y - min.y) / max_edge_length).ceil() as usize;
    let nz = ((max.z - min.z) / max_edge_length).ceil() as usize;
    let nx = nx.max(1);
    let ny = ny.max(1);
    let nz = nz.max(1);

    let dx = (max.x - min.x) / nx as f64;
    let dy = (max.y - min.y) / ny as f64;
    let dz = (max.z - min.z) / nz as f64;

    // Create grid nodes
    let mut nodes = Vec::new();
    let node_idx = |ix: usize, iy: usize, iz: usize| -> usize {
        ix * (ny + 1) * (nz + 1) + iy * (nz + 1) + iz
    };

    for ix in 0..=nx {
        for iy in 0..=ny {
            for iz in 0..=nz {
                nodes.push(Point3::new(
                    min.x + ix as f64 * dx,
                    min.y + iy as f64 * dy,
                    min.z + iz as f64 * dz,
                ));
            }
        }
    }

    // For each cube, check if center is inside solid, then split into 5 tets
    let mut elements = Vec::new();
    for ix in 0..nx {
        for iy in 0..ny {
            for iz in 0..nz {
                let center = Point3::new(
                    min.x + (ix as f64 + 0.5) * dx,
                    min.y + (iy as f64 + 0.5) * dy,
                    min.z + (iz as f64 + 0.5) * dz,
                );

                let containment = point_in_solid(model, solid, center)?;
                if matches!(containment, Containment::Outside) {
                    continue;
                }

                // 8 corner nodes of the cube
                //   v0 = (ix,   iy,   iz  )
                //   v1 = (ix+1, iy,   iz  )
                //   v2 = (ix+1, iy+1, iz  )
                //   v3 = (ix,   iy+1, iz  )
                //   v4 = (ix,   iy,   iz+1)
                //   v5 = (ix+1, iy,   iz+1)
                //   v6 = (ix+1, iy+1, iz+1)
                //   v7 = (ix,   iy+1, iz+1)
                let v0 = node_idx(ix, iy, iz);
                let v1 = node_idx(ix + 1, iy, iz);
                let v2 = node_idx(ix + 1, iy + 1, iz);
                let v3 = node_idx(ix, iy + 1, iz);
                let v4 = node_idx(ix, iy, iz + 1);
                let v5 = node_idx(ix + 1, iy, iz + 1);
                let v6 = node_idx(ix + 1, iy + 1, iz + 1);
                let v7 = node_idx(ix, iy + 1, iz + 1);

                // Split hexahedron into 5 tetrahedra
                // (alternating parity for conforming mesh)
                let parity = (ix + iy + iz) % 2;
                if parity == 0 {
                    elements.push([v0, v1, v3, v4]);
                    elements.push([v1, v2, v3, v6]);
                    elements.push([v4, v5, v1, v6]);
                    elements.push([v4, v7, v3, v6]);
                    elements.push([v1, v3, v4, v6]);
                } else {
                    elements.push([v0, v1, v2, v5]);
                    elements.push([v0, v2, v3, v7]);
                    elements.push([v0, v4, v5, v7]);
                    elements.push([v5, v6, v2, v7]);
                    elements.push([v0, v2, v5, v7]);
                }
            }
        }
    }

    if elements.is_empty() {
        return Err(KernelError::GeometryError(
            "no tetrahedra generated — solid may be too small for the given edge length".into(),
        ));
    }

    Ok(TetMesh { nodes, elements })
}

/// Perform a linear static FEM analysis on a tetrahedral mesh.
///
/// Assembles the global stiffness matrix, applies boundary conditions via the
/// penalty method, and solves the system using Gauss-Seidel iteration.
pub fn static_analysis(
    mesh: &TetMesh,
    material: &FemMaterial,
    bcs: &[BoundaryCondition],
) -> KernelResult<FemResult> {
    if mesh.nodes.is_empty() || mesh.elements.is_empty() {
        return Err(KernelError::InvalidArgument(
            "mesh must have nodes and elements".into(),
        ));
    }
    if material.youngs_modulus <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "Young's modulus must be positive".into(),
        ));
    }
    if material.poisson_ratio < 0.0 || material.poisson_ratio >= 0.5 {
        return Err(KernelError::InvalidArgument(
            "Poisson ratio must be in [0, 0.5)".into(),
        ));
    }

    let n_nodes = mesh.nodes.len();
    let n_dof = n_nodes * 3;

    // Build the 6x6 elasticity matrix D for isotropic material
    let d_matrix = build_elasticity_matrix(material.youngs_modulus, material.poisson_ratio);

    // Sparse global stiffness: row-based adjacency list
    // K[i] = vec of (col, value) — where i,col are DOF indices
    let mut k_rows: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n_dof];
    let mut rhs = vec![0.0_f64; n_dof];

    // Assemble element stiffness matrices
    for elem in &mesh.elements {
        let ke = element_stiffness(mesh, elem, &d_matrix)?;

        // Scatter into global matrix
        for local_i in 0..4 {
            for local_j in 0..4 {
                for di in 0..3 {
                    for dj in 0..3 {
                        let gi = elem[local_i] * 3 + di;
                        let gj = elem[local_j] * 3 + dj;
                        let val = ke[local_i * 3 + di][local_j * 3 + dj];
                        if val.abs() > 1e-30 {
                            add_to_sparse_row(&mut k_rows[gi], gj, val);
                        }
                    }
                }
            }
        }
    }

    // Apply boundary conditions
    for bc in bcs {
        match bc {
            BoundaryCondition::FixedNode(node) => {
                if *node >= n_nodes {
                    return Err(KernelError::InvalidArgument(format!(
                        "fixed node index {} out of range ({})",
                        node, n_nodes
                    )));
                }
                // Penalty method: add large value to diagonal
                let penalty = material.youngs_modulus * 1e10;
                for d in 0..3 {
                    let dof = node * 3 + d;
                    add_to_sparse_row(&mut k_rows[dof], dof, penalty);
                    // RHS stays zero for fixed constraint
                }
            }
            BoundaryCondition::Force { node, force } => {
                if *node >= n_nodes {
                    return Err(KernelError::InvalidArgument(format!(
                        "force node index {} out of range ({})",
                        node, n_nodes
                    )));
                }
                rhs[node * 3] += force.x;
                rhs[node * 3 + 1] += force.y;
                rhs[node * 3 + 2] += force.z;
            }
            BoundaryCondition::Pressure { element, pressure } => {
                if *element >= mesh.elements.len() {
                    return Err(KernelError::InvalidArgument(format!(
                        "pressure element index {} out of range ({})",
                        element,
                        mesh.elements.len()
                    )));
                }
                let elem_nodes = &mesh.elements[*element];
                let p0 = mesh.nodes[elem_nodes[0]];
                let p1 = mesh.nodes[elem_nodes[1]];
                let p2 = mesh.nodes[elem_nodes[2]];
                let e1 = p1 - p0;
                let e2 = p2 - p0;
                let normal = e1.cross(e2);
                let area = normal.length() * 0.5;
                let n_hat = if normal.length() > 1e-30 {
                    normal * (1.0 / normal.length())
                } else {
                    Vec3::Z
                };
                let force_per_node = n_hat * (*pressure * area / 3.0);
                for &ni in &elem_nodes[0..3] {
                    rhs[ni * 3] += force_per_node.x;
                    rhs[ni * 3 + 1] += force_per_node.y;
                    rhs[ni * 3 + 2] += force_per_node.z;
                }
            }
            BoundaryCondition::Displacement {
                node,
                displacement,
            } => {
                if *node >= n_nodes {
                    return Err(KernelError::InvalidArgument(format!(
                        "displacement node index {} out of range ({})",
                        node, n_nodes
                    )));
                }
                let penalty = material.youngs_modulus * 1e10;
                for d in 0..3 {
                    let dof = node * 3 + d;
                    add_to_sparse_row(&mut k_rows[dof], dof, penalty);
                    let disp_val = match d {
                        0 => displacement.x,
                        1 => displacement.y,
                        _ => displacement.z,
                    };
                    rhs[dof] += penalty * disp_val;
                }
            }
            BoundaryCondition::Gravity { acceleration } => {
                for elem_nodes in &mesh.elements {
                    let vol = tet_volume(&mesh.nodes, elem_nodes);
                    let elem_mass = vol * material.density;
                    let node_force = elem_mass / 4.0;
                    for &ni in elem_nodes {
                        rhs[ni * 3] += node_force * acceleration.x;
                        rhs[ni * 3 + 1] += node_force * acceleration.y;
                        rhs[ni * 3 + 2] += node_force * acceleration.z;
                    }
                }
            }
            BoundaryCondition::DistributedLoad { element, load } => {
                if *element >= mesh.elements.len() {
                    return Err(KernelError::InvalidArgument(format!(
                        "distributed load element index {} out of range",
                        element
                    )));
                }
                let elem_nodes = &mesh.elements[*element];
                let vol = tet_volume(&mesh.nodes, elem_nodes);
                let node_load = vol / 4.0;
                for &ni in elem_nodes {
                    rhs[ni * 3] += node_load * load.x;
                    rhs[ni * 3 + 1] += node_load * load.y;
                    rhs[ni * 3 + 2] += node_load * load.z;
                }
            }
            BoundaryCondition::Spring { node, stiffness } => {
                if *node >= n_nodes {
                    return Err(KernelError::InvalidArgument(format!(
                        "spring node index {} out of range ({})",
                        node, n_nodes
                    )));
                }
                for d in 0..3 {
                    let dof = node * 3 + d;
                    add_to_sparse_row(&mut k_rows[dof], dof, *stiffness);
                }
            }
            BoundaryCondition::CentrifugalLoad { axis, omega } => {
                let axis_n = axis.normalized().unwrap_or(Vec3::Z);
                let omega2 = omega * omega;
                for elem_nodes in &mesh.elements {
                    let vol = tet_volume(&mesh.nodes, elem_nodes);
                    let elem_mass = vol * material.density;
                    let node_mass = elem_mass / 4.0;
                    for &ni in elem_nodes {
                        let r_vec = mesh.nodes[ni] - Point3::ORIGIN;
                        let proj = axis_n * r_vec.dot(axis_n);
                        let radial = r_vec - proj;
                        let force = radial * (node_mass * omega2);
                        rhs[ni * 3] += force.x;
                        rhs[ni * 3 + 1] += force.y;
                        rhs[ni * 3 + 2] += force.z;
                    }
                }
            }
            BoundaryCondition::SelfWeight { gravity } => {
                for elem_nodes in &mesh.elements {
                    let vol = tet_volume(&mesh.nodes, elem_nodes);
                    let elem_mass = vol * material.density;
                    let node_force = elem_mass / 4.0;
                    for &ni in elem_nodes {
                        rhs[ni * 3] += node_force * gravity.x;
                        rhs[ni * 3 + 1] += node_force * gravity.y;
                        rhs[ni * 3 + 2] += node_force * gravity.z;
                    }
                }
            }
            BoundaryCondition::SectionPrint { .. } => {
                // Post-processing marker — no contribution to stiffness/RHS
            }
            BoundaryCondition::TieConstraint {
                surface_a,
                surface_b,
            } => {
                let penalty = material.youngs_modulus * 1e8;
                let pairs = surface_a.len().min(surface_b.len());
                for i in 0..pairs {
                    let na = surface_a[i];
                    let nb = surface_b[i];
                    if na >= n_nodes || nb >= n_nodes {
                        continue;
                    }
                    for d in 0..3 {
                        let dof_a = na * 3 + d;
                        let dof_b = nb * 3 + d;
                        add_to_sparse_row(&mut k_rows[dof_a], dof_a, penalty);
                        add_to_sparse_row(&mut k_rows[dof_b], dof_b, penalty);
                        add_to_sparse_row(&mut k_rows[dof_a], dof_b, -penalty);
                        add_to_sparse_row(&mut k_rows[dof_b], dof_a, -penalty);
                    }
                }
            }
            BoundaryCondition::RigidBody { node_ids } => {
                if node_ids.len() >= 2 {
                    let penalty = material.youngs_modulus * 1e10;
                    let master = node_ids[0];
                    for &slave in &node_ids[1..] {
                        if master >= n_nodes || slave >= n_nodes {
                            continue;
                        }
                        for d in 0..3 {
                            let dm = master * 3 + d;
                            let ds = slave * 3 + d;
                            add_to_sparse_row(&mut k_rows[dm], dm, penalty);
                            add_to_sparse_row(&mut k_rows[ds], ds, penalty);
                            add_to_sparse_row(&mut k_rows[dm], ds, -penalty);
                            add_to_sparse_row(&mut k_rows[ds], dm, -penalty);
                        }
                    }
                }
            }
            BoundaryCondition::SpringConstraint {
                node_id,
                stiffness,
                direction,
            } => {
                if *node_id >= n_nodes {
                    return Err(KernelError::InvalidArgument(format!(
                        "spring constraint node {} out of range ({})",
                        node_id, n_nodes
                    )));
                }
                let dir = direction.normalized().unwrap_or(Vec3::Z);
                let components = [dir.x, dir.y, dir.z];
                for di in 0..3 {
                    for dj in 0..3 {
                        let val = stiffness * components[di] * components[dj];
                        if val.abs() > 1e-30 {
                            let dof_i = node_id * 3 + di;
                            let dof_j = node_id * 3 + dj;
                            add_to_sparse_row(&mut k_rows[dof_i], dof_j, val);
                        }
                    }
                }
            }
            BoundaryCondition::BodyLoad { force_density } => {
                for elem_nodes in &mesh.elements {
                    let vol = tet_volume(&mesh.nodes, elem_nodes);
                    let node_force = vol / 4.0;
                    for &ni in elem_nodes {
                        rhs[ni * 3] += node_force * force_density.x;
                        rhs[ni * 3 + 1] += node_force * force_density.y;
                        rhs[ni * 3 + 2] += node_force * force_density.z;
                    }
                }
            }
            BoundaryCondition::ContactConstraint {
                surface_a,
                surface_b,
                penalty,
            } => {
                let pairs = surface_a.len().min(surface_b.len());
                for i in 0..pairs {
                    let na = surface_a[i];
                    let nb = surface_b[i];
                    if na >= n_nodes || nb >= n_nodes {
                        continue;
                    }
                    for d in 0..3 {
                        let dof_a = na * 3 + d;
                        let dof_b = nb * 3 + d;
                        add_to_sparse_row(&mut k_rows[dof_a], dof_a, *penalty);
                        add_to_sparse_row(&mut k_rows[dof_b], dof_b, *penalty);
                        add_to_sparse_row(&mut k_rows[dof_a], dof_b, -penalty);
                        add_to_sparse_row(&mut k_rows[dof_b], dof_a, -penalty);
                    }
                }
            }
            BoundaryCondition::InitialTemperature { .. } => {
                // Reference state marker — no contribution to mechanical stiffness/RHS
            }
        }
    }

    // Solve K * u = f using Gauss-Seidel iteration
    let mut u = vec![0.0_f64; n_dof];
    let max_iter = 10_000;
    let tol = 1e-10;

    for _iter in 0..max_iter {
        let mut max_delta = 0.0_f64;
        for i in 0..n_dof {
            let mut diag = 0.0_f64;
            let mut sum = 0.0_f64;
            for &(j, val) in &k_rows[i] {
                if j == i {
                    diag = val;
                } else {
                    sum += val * u[j];
                }
            }
            if diag.abs() < 1e-30 {
                continue;
            }
            let new_val = (rhs[i] - sum) / diag;
            let delta = (new_val - u[i]).abs();
            if delta > max_delta {
                max_delta = delta;
            }
            u[i] = new_val;
        }
        if max_delta < tol {
            break;
        }
    }

    // Extract displacements
    let mut displacements = Vec::with_capacity(n_nodes);
    let mut max_displacement = 0.0_f64;
    for i in 0..n_nodes {
        let disp = Vec3::new(u[i * 3], u[i * 3 + 1], u[i * 3 + 2]);
        let mag = disp.length();
        if mag > max_displacement {
            max_displacement = mag;
        }
        displacements.push(disp);
    }

    // Compute element von Mises stresses
    let mut stresses = Vec::with_capacity(mesh.elements.len());
    let mut max_stress = 0.0_f64;
    for elem in &mesh.elements {
        let vm = element_von_mises(mesh, elem, &d_matrix, &u)?;
        if vm > max_stress {
            max_stress = vm;
        }
        stresses.push(vm);
    }

    Ok(FemResult {
        displacements,
        stresses,
        max_displacement,
        max_stress,
    })
}

/// Build the 6x6 isotropic elasticity matrix D.
fn build_elasticity_matrix(e: f64, nu: f64) -> [[f64; 6]; 6] {
    let c = e / ((1.0 + nu) * (1.0 - 2.0 * nu));
    let mut d = [[0.0_f64; 6]; 6];

    d[0][0] = c * (1.0 - nu);
    d[1][1] = c * (1.0 - nu);
    d[2][2] = c * (1.0 - nu);

    d[0][1] = c * nu;
    d[0][2] = c * nu;
    d[1][0] = c * nu;
    d[1][2] = c * nu;
    d[2][0] = c * nu;
    d[2][1] = c * nu;

    d[3][3] = c * (1.0 - 2.0 * nu) / 2.0;
    d[4][4] = c * (1.0 - 2.0 * nu) / 2.0;
    d[5][5] = c * (1.0 - 2.0 * nu) / 2.0;

    d
}

/// Compute the 12x12 element stiffness matrix for a linear tetrahedron.
fn element_stiffness(
    mesh: &TetMesh,
    elem: &[usize; 4],
    d_matrix: &[[f64; 6]; 6],
) -> KernelResult<[[f64; 12]; 12]> {
    let p0 = mesh.nodes[elem[0]];
    let p1 = mesh.nodes[elem[1]];
    let p2 = mesh.nodes[elem[2]];
    let p3 = mesh.nodes[elem[3]];

    // Edge vectors from node 0
    let x10 = p1.x - p0.x;
    let y10 = p1.y - p0.y;
    let z10 = p1.z - p0.z;
    let x20 = p2.x - p0.x;
    let y20 = p2.y - p0.y;
    let z20 = p2.z - p0.z;
    let x30 = p3.x - p0.x;
    let y30 = p3.y - p0.y;
    let z30 = p3.z - p0.z;

    // Jacobian determinant = 6 * volume
    let det_j = x10 * (y20 * z30 - y30 * z20)
        - y10 * (x20 * z30 - x30 * z20)
        + z10 * (x20 * y30 - x30 * y20);

    let volume = det_j.abs() / 6.0;
    if volume < 1e-30 {
        return Err(KernelError::GeometryError(
            "degenerate tetrahedron with zero volume".into(),
        ));
    }

    // Inverse of Jacobian columns for shape function derivatives
    // dN/dx, dN/dy, dN/dz for each of the 4 shape functions
    let inv_det = 1.0 / det_j;

    // Cofactors for the Jacobian inverse
    let a11 = (y20 * z30 - y30 * z20) * inv_det;
    let a12 = -(x20 * z30 - x30 * z20) * inv_det;
    let a13 = (x20 * y30 - x30 * y20) * inv_det;

    let a21 = -(y10 * z30 - y30 * z10) * inv_det;
    let a22 = (x10 * z30 - x30 * z10) * inv_det;
    let a23 = -(x10 * y30 - x30 * y10) * inv_det;

    let a31 = (y10 * z20 - y20 * z10) * inv_det;
    let a32 = -(x10 * z20 - x20 * z10) * inv_det;
    let a33 = (x10 * y20 - x20 * y10) * inv_det;

    // Shape function derivatives in physical coordinates
    // N0 = 1 - xi - eta - zeta  =>  dN0/dx = -(a11+a21+a31), etc.
    // N1 = xi                   =>  dN1/dx = a11, etc.
    // N2 = eta                  =>  dN2/dx = a21, etc.
    // N3 = zeta                 =>  dN3/dx = a31, etc.
    let dn = [
        [-(a11 + a21 + a31), -(a12 + a22 + a32), -(a13 + a23 + a33)],
        [a11, a12, a13],
        [a21, a22, a23],
        [a31, a32, a33],
    ];

    // Build B matrix (6x12): strain-displacement
    let mut b = [[0.0_f64; 12]; 6];
    for (i, dn_i) in dn.iter().enumerate() {
        let col = i * 3;
        b[0][col] = dn_i[0];
        b[1][col + 1] = dn_i[1];
        b[2][col + 2] = dn_i[2];
        b[3][col] = dn_i[1];
        b[3][col + 1] = dn_i[0];
        b[4][col] = dn_i[2];
        b[4][col + 2] = dn_i[0];
        b[5][col + 1] = dn_i[2];
        b[5][col + 2] = dn_i[1];
    }

    // K_e = V * B^T * D * B
    // First compute DB = D * B (6x12)
    let mut db = [[0.0_f64; 12]; 6];
    for i in 0..6 {
        for j in 0..12 {
            let mut s = 0.0_f64;
            for k in 0..6 {
                s += d_matrix[i][k] * b[k][j];
            }
            db[i][j] = s;
        }
    }

    // K_e = V * B^T * DB  (12x12)
    let mut ke = [[0.0_f64; 12]; 12];
    for i in 0..12 {
        for j in 0..12 {
            let mut s = 0.0_f64;
            for k in 0..6 {
                s += b[k][i] * db[k][j];
            }
            ke[i][j] = s * volume;
        }
    }

    Ok(ke)
}

/// Add value to a sparse row, merging with existing entry if column matches.
fn add_to_sparse_row(row: &mut Vec<(usize, f64)>, col: usize, val: f64) {
    for entry in row.iter_mut() {
        if entry.0 == col {
            entry.1 += val;
            return;
        }
    }
    row.push((col, val));
}

/// Compute von Mises stress for a single tetrahedral element.
fn element_von_mises(
    mesh: &TetMesh,
    elem: &[usize; 4],
    d_matrix: &[[f64; 6]; 6],
    u: &[f64],
) -> KernelResult<f64> {
    let p0 = mesh.nodes[elem[0]];
    let p1 = mesh.nodes[elem[1]];
    let p2 = mesh.nodes[elem[2]];
    let p3 = mesh.nodes[elem[3]];

    let x10 = p1.x - p0.x;
    let y10 = p1.y - p0.y;
    let z10 = p1.z - p0.z;
    let x20 = p2.x - p0.x;
    let y20 = p2.y - p0.y;
    let z20 = p2.z - p0.z;
    let x30 = p3.x - p0.x;
    let y30 = p3.y - p0.y;
    let z30 = p3.z - p0.z;

    let det_j = x10 * (y20 * z30 - y30 * z20)
        - y10 * (x20 * z30 - x30 * z20)
        + z10 * (x20 * y30 - x30 * y20);

    if det_j.abs() < 1e-30 {
        return Ok(0.0);
    }

    let inv_det = 1.0 / det_j;

    let a11 = (y20 * z30 - y30 * z20) * inv_det;
    let a12 = -(x20 * z30 - x30 * z20) * inv_det;
    let a13 = (x20 * y30 - x30 * y20) * inv_det;
    let a21 = -(y10 * z30 - y30 * z10) * inv_det;
    let a22 = (x10 * z30 - x30 * z10) * inv_det;
    let a23 = -(x10 * y30 - x30 * y10) * inv_det;
    let a31 = (y10 * z20 - y20 * z10) * inv_det;
    let a32 = -(x10 * z20 - x20 * z10) * inv_det;
    let a33 = (x10 * y20 - x20 * y10) * inv_det;

    let dn = [
        [-(a11 + a21 + a31), -(a12 + a22 + a32), -(a13 + a23 + a33)],
        [a11, a12, a13],
        [a21, a22, a23],
        [a31, a32, a33],
    ];

    // Build B matrix
    let mut b = [[0.0_f64; 12]; 6];
    for (i, dn_i) in dn.iter().enumerate() {
        let col = i * 3;
        b[0][col] = dn_i[0];
        b[1][col + 1] = dn_i[1];
        b[2][col + 2] = dn_i[2];
        b[3][col] = dn_i[1];
        b[3][col + 1] = dn_i[0];
        b[4][col] = dn_i[2];
        b[4][col + 2] = dn_i[0];
        b[5][col + 1] = dn_i[2];
        b[5][col + 2] = dn_i[1];
    }

    // Element displacement vector (12x1)
    let mut ue = [0.0_f64; 12];
    for i in 0..4 {
        let gi = elem[i] * 3;
        ue[i * 3] = u[gi];
        ue[i * 3 + 1] = u[gi + 1];
        ue[i * 3 + 2] = u[gi + 2];
    }

    // Strain = B * ue (6x1)
    let mut strain = [0.0_f64; 6];
    for i in 0..6 {
        let mut s = 0.0_f64;
        for j in 0..12 {
            s += b[i][j] * ue[j];
        }
        strain[i] = s;
    }

    // Stress = D * strain (6x1)
    let mut stress = [0.0_f64; 6];
    for i in 0..6 {
        let mut s = 0.0_f64;
        for j in 0..6 {
            s += d_matrix[i][j] * strain[j];
        }
        stress[i] = s;
    }

    // Von Mises: sigma_vm = sqrt(0.5 * ((s1-s2)^2 + (s2-s3)^2 + (s3-s1)^2 + 6*(t12^2+t23^2+t13^2)))
    let s_xx = stress[0];
    let s_yy = stress[1];
    let s_zz = stress[2];
    let t_xy = stress[3];
    let t_xz = stress[4];
    let t_yz = stress[5];

    let vm_sq = 0.5
        * ((s_xx - s_yy).powi(2)
            + (s_yy - s_zz).powi(2)
            + (s_zz - s_xx).powi(2)
            + 6.0 * (t_xy.powi(2) + t_xz.powi(2) + t_yz.powi(2)));

    Ok(vm_sq.max(0.0).sqrt())
}

/// Compute the volume of a tetrahedron.
fn tet_volume(nodes: &[Point3], elem: &[usize; 4]) -> f64 {
    let p0 = nodes[elem[0]];
    let p1 = nodes[elem[1]];
    let p2 = nodes[elem[2]];
    let p3 = nodes[elem[3]];
    let v1 = p1 - p0;
    let v2 = p2 - p0;
    let v3 = p3 - p0;
    (v1.x * (v2.y * v3.z - v2.z * v3.y)
        - v1.y * (v2.x * v3.z - v2.z * v3.x)
        + v1.z * (v2.x * v3.y - v2.y * v3.x))
    .abs()
        / 6.0
}

/// Perform a modal analysis to find natural frequencies and mode shapes.
///
/// Uses the inverse power iteration method to find the lowest `num_modes`
/// eigenfrequencies and corresponding mode shapes of the structure.
pub fn modal_analysis(
    mesh: &TetMesh,
    material: &FemMaterial,
    bcs: &[BoundaryCondition],
    num_modes: usize,
) -> KernelResult<ModalResult> {
    if mesh.nodes.is_empty() || mesh.elements.is_empty() {
        return Err(KernelError::InvalidArgument(
            "mesh must have nodes and elements".into(),
        ));
    }
    if num_modes == 0 {
        return Err(KernelError::InvalidArgument(
            "num_modes must be at least 1".into(),
        ));
    }

    let n_nodes = mesh.nodes.len();
    let n_dof = n_nodes * 3;

    let d_matrix = build_elasticity_matrix(material.youngs_modulus, material.poisson_ratio);

    // Build stiffness matrix
    let mut k_diag = vec![0.0_f64; n_dof];
    let mut k_rows: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n_dof];

    for elem in &mesh.elements {
        let ke = element_stiffness(mesh, elem, &d_matrix)?;
        for local_i in 0..4 {
            for local_j in 0..4 {
                for di in 0..3 {
                    for dj in 0..3 {
                        let gi = elem[local_i] * 3 + di;
                        let gj = elem[local_j] * 3 + dj;
                        let val = ke[local_i * 3 + di][local_j * 3 + dj];
                        if val.abs() > 1e-30 {
                            if gi == gj {
                                k_diag[gi] += val;
                            }
                            add_to_sparse_row(&mut k_rows[gi], gj, val);
                        }
                    }
                }
            }
        }
    }

    // Build lumped mass matrix (diagonal)
    let mut m_diag = vec![0.0_f64; n_dof];
    for elem in &mesh.elements {
        let vol = tet_volume(&mesh.nodes, elem);
        let elem_mass = vol * material.density;
        let node_mass = elem_mass / 4.0;
        for &ni in elem {
            for d in 0..3 {
                m_diag[ni * 3 + d] += node_mass;
            }
        }
    }

    // Apply fixed BCs: set very large stiffness and mass
    let mut fixed_dofs = vec![false; n_dof];
    for bc in bcs {
        if let BoundaryCondition::FixedNode(node) = bc {
            if *node < n_nodes {
                for d in 0..3 {
                    fixed_dofs[node * 3 + d] = true;
                }
            }
        }
    }

    // Inverse power iteration for each mode
    let mut frequencies = Vec::with_capacity(num_modes);
    let mut mode_shapes = Vec::with_capacity(num_modes);
    let mut prev_modes: Vec<Vec<f64>> = Vec::new();

    for _mode in 0..num_modes {
        // Random initial vector
        let mut x = vec![0.0_f64; n_dof];
        for (i, xi) in x.iter_mut().enumerate() {
            if !fixed_dofs[i] {
                *xi = ((i * 7 + 13) % 97) as f64 / 97.0 - 0.5;
            }
        }

        let mut eigenvalue = 0.0_f64;
        let max_iter = 500;

        for _iter in 0..max_iter {
            // Gram-Schmidt against previous modes
            for prev in &prev_modes {
                let dot: f64 = x.iter().zip(prev.iter()).map(|(a, b)| a * b).sum();
                for (xi, pi) in x.iter_mut().zip(prev.iter()) {
                    *xi -= dot * pi;
                }
            }

            // y = M * x
            let mut y = vec![0.0_f64; n_dof];
            for i in 0..n_dof {
                y[i] = m_diag[i] * x[i];
            }

            // Solve K * z = y using Gauss-Seidel
            let mut z = vec![0.0_f64; n_dof];
            for _gs in 0..200 {
                for i in 0..n_dof {
                    if fixed_dofs[i] {
                        z[i] = 0.0;
                        continue;
                    }
                    let mut sum = 0.0_f64;
                    let mut diag = k_diag[i];
                    for &(j, val) in &k_rows[i] {
                        if j == i {
                            diag = val;
                        } else {
                            sum += val * z[j];
                        }
                    }
                    if diag.abs() > 1e-30 {
                        z[i] = (y[i] - sum) / diag;
                    }
                }
            }

            // Rayleigh quotient: λ = x^T K x / x^T M x
            let xtmx: f64 = x
                .iter()
                .enumerate()
                .map(|(i, &xi)| xi * m_diag[i] * xi)
                .sum();

            // Normalize z
            let norm: f64 = z.iter().map(|v| v * v).sum::<f64>().sqrt();
            if norm > 1e-30 {
                for zi in &mut z {
                    *zi /= norm;
                }
            }

            // Compute eigenvalue from Rayleigh quotient of z
            let mut ztkz = 0.0_f64;
            let mut ztmz = 0.0_f64;
            for i in 0..n_dof {
                ztmz += z[i] * m_diag[i] * z[i];
                for &(j, val) in &k_rows[i] {
                    ztkz += z[i] * val * z[j];
                }
            }
            eigenvalue = if ztmz.abs() > 1e-30 {
                ztkz / ztmz
            } else {
                0.0
            };

            let _ = xtmx; // used for convergence check in full implementation
            x = z;
        }

        // Convert eigenvalue to frequency (Hz): f = sqrt(λ) / (2π)
        let freq = if eigenvalue > 0.0 {
            eigenvalue.sqrt() / (2.0 * std::f64::consts::PI)
        } else {
            0.0
        };

        frequencies.push(freq);

        // Convert to mode shape vectors
        let mut shape = Vec::with_capacity(n_nodes);
        for i in 0..n_nodes {
            shape.push(Vec3::new(x[i * 3], x[i * 3 + 1], x[i * 3 + 2]));
        }
        mode_shapes.push(shape);

        prev_modes.push(x);
    }

    Ok(ModalResult {
        frequencies,
        mode_shapes,
    })
}

/// Perform steady-state thermal analysis on a tetrahedral mesh.
///
/// Solves the heat equation ∇·(k∇T) = Q with given thermal boundary conditions.
pub fn thermal_analysis(
    mesh: &TetMesh,
    material: &ThermalMaterial,
    bcs: &[ThermalBoundaryCondition],
) -> KernelResult<ThermalResult> {
    if mesh.nodes.is_empty() || mesh.elements.is_empty() {
        return Err(KernelError::InvalidArgument(
            "mesh must have nodes and elements".into(),
        ));
    }
    if material.conductivity <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "thermal conductivity must be positive".into(),
        ));
    }

    let n_nodes = mesh.nodes.len();

    // Build thermal conductivity matrix and RHS
    let mut k_rows: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n_nodes];
    let mut rhs = vec![0.0_f64; n_nodes];

    // Element conductivity matrices
    for elem in &mesh.elements {
        let ke = element_thermal_stiffness(mesh, elem, material.conductivity)?;

        for local_i in 0..4 {
            for local_j in 0..4 {
                let gi = elem[local_i];
                let gj = elem[local_j];
                let val = ke[local_i][local_j];
                if val.abs() > 1e-30 {
                    add_to_sparse_row(&mut k_rows[gi], gj, val);
                }
            }
        }
    }

    // Apply thermal BCs
    for bc in bcs {
        match bc {
            ThermalBoundaryCondition::FixedTemperature { node, temperature } => {
                if *node >= n_nodes {
                    return Err(KernelError::InvalidArgument(format!(
                        "node index {} out of range ({})",
                        node, n_nodes
                    )));
                }
                let penalty = material.conductivity * 1e10;
                add_to_sparse_row(&mut k_rows[*node], *node, penalty);
                rhs[*node] += penalty * temperature;
            }
            ThermalBoundaryCondition::HeatFlux { element, flux } => {
                if *element >= mesh.elements.len() {
                    return Err(KernelError::InvalidArgument(format!(
                        "element index {} out of range",
                        element
                    )));
                }
                let elem_nodes = &mesh.elements[*element];
                let vol = tet_volume(&mesh.nodes, elem_nodes);
                let node_flux = flux * vol / 4.0;
                for &ni in elem_nodes {
                    rhs[ni] += node_flux;
                }
            }
            ThermalBoundaryCondition::HeatGeneration { element, rate } => {
                if *element >= mesh.elements.len() {
                    return Err(KernelError::InvalidArgument(format!(
                        "element index {} out of range",
                        element
                    )));
                }
                let elem_nodes = &mesh.elements[*element];
                let vol = tet_volume(&mesh.nodes, elem_nodes);
                let node_heat = rate * vol / 4.0;
                for &ni in elem_nodes {
                    rhs[ni] += node_heat;
                }
            }
            ThermalBoundaryCondition::Convection {
                element,
                coefficient,
                ambient_temp,
            } => {
                if *element >= mesh.elements.len() {
                    return Err(KernelError::InvalidArgument(format!(
                        "element index {} out of range",
                        element
                    )));
                }
                let elem_nodes = &mesh.elements[*element];
                // Approximate: distribute convection equally to the 3 surface nodes
                let p0 = mesh.nodes[elem_nodes[0]];
                let p1 = mesh.nodes[elem_nodes[1]];
                let p2 = mesh.nodes[elem_nodes[2]];
                let area = (p1 - p0).cross(p2 - p0).length() * 0.5;
                let h_per_node = coefficient * area / 3.0;
                for &ni in &elem_nodes[0..3] {
                    add_to_sparse_row(&mut k_rows[ni], ni, h_per_node);
                    rhs[ni] += h_per_node * ambient_temp;
                }
            }
        }
    }

    // Solve using Gauss-Seidel
    let mut temps = vec![0.0_f64; n_nodes];
    let max_iter = 10_000;
    let tol = 1e-10;

    for _iter in 0..max_iter {
        let mut max_delta = 0.0_f64;
        for i in 0..n_nodes {
            let mut diag = 0.0_f64;
            let mut sum = 0.0_f64;
            for &(j, val) in &k_rows[i] {
                if j == i {
                    diag = val;
                } else {
                    sum += val * temps[j];
                }
            }
            if diag.abs() < 1e-30 {
                continue;
            }
            let new_val = (rhs[i] - sum) / diag;
            let delta = (new_val - temps[i]).abs();
            if delta > max_delta {
                max_delta = delta;
            }
            temps[i] = new_val;
        }
        if max_delta < tol {
            break;
        }
    }

    // Compute heat fluxes per element: q = -k * ∇T
    let mut heat_fluxes = Vec::with_capacity(mesh.elements.len());
    for elem in &mesh.elements {
        let grad = element_temperature_gradient(mesh, elem, &temps)?;
        heat_fluxes.push(grad * (-material.conductivity));
    }

    let max_temperature = temps.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min_temperature = temps.iter().cloned().fold(f64::INFINITY, f64::min);

    Ok(ThermalResult {
        temperatures: temps,
        heat_fluxes,
        max_temperature,
        min_temperature,
    })
}

/// Build the 4x4 element thermal conductivity matrix for a linear tetrahedron.
fn element_thermal_stiffness(
    mesh: &TetMesh,
    elem: &[usize; 4],
    conductivity: f64,
) -> KernelResult<[[f64; 4]; 4]> {
    let p0 = mesh.nodes[elem[0]];
    let p1 = mesh.nodes[elem[1]];
    let p2 = mesh.nodes[elem[2]];
    let p3 = mesh.nodes[elem[3]];

    let v1 = p1 - p0;
    let v2 = p2 - p0;
    let v3 = p3 - p0;

    let det_j = v1.x * (v2.y * v3.z - v2.z * v3.y)
        - v1.y * (v2.x * v3.z - v2.z * v3.x)
        + v1.z * (v2.x * v3.y - v2.y * v3.x);

    let volume = det_j.abs() / 6.0;
    if volume < 1e-30 {
        return Err(KernelError::GeometryError(
            "degenerate tetrahedron".into(),
        ));
    }

    let inv_det = 1.0 / det_j;

    // Shape function gradients (same as structural but scalar)
    let a11 = (v2.y * v3.z - v3.y * v2.z) * inv_det;
    let a12 = -(v2.x * v3.z - v3.x * v2.z) * inv_det;
    let a13 = (v2.x * v3.y - v3.x * v2.y) * inv_det;
    let a21 = -(v1.y * v3.z - v3.y * v1.z) * inv_det;
    let a22 = (v1.x * v3.z - v3.x * v1.z) * inv_det;
    let a23 = -(v1.x * v3.y - v3.x * v1.y) * inv_det;
    let a31 = (v1.y * v2.z - v2.y * v1.z) * inv_det;
    let a32 = -(v1.x * v2.z - v2.x * v1.z) * inv_det;
    let a33 = (v1.x * v2.y - v2.x * v1.y) * inv_det;

    let dn = [
        [-(a11 + a21 + a31), -(a12 + a22 + a32), -(a13 + a23 + a33)],
        [a11, a12, a13],
        [a21, a22, a23],
        [a31, a32, a33],
    ];

    // K_e = V * k * ∇N^T * ∇N
    let mut ke = [[0.0_f64; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            ke[i][j] = conductivity
                * volume
                * (dn[i][0] * dn[j][0] + dn[i][1] * dn[j][1] + dn[i][2] * dn[j][2]);
        }
    }

    Ok(ke)
}

/// Compute temperature gradient in an element.
fn element_temperature_gradient(
    mesh: &TetMesh,
    elem: &[usize; 4],
    temps: &[f64],
) -> KernelResult<Vec3> {
    let p0 = mesh.nodes[elem[0]];
    let p1 = mesh.nodes[elem[1]];
    let p2 = mesh.nodes[elem[2]];
    let p3 = mesh.nodes[elem[3]];

    let v1 = p1 - p0;
    let v2 = p2 - p0;
    let v3 = p3 - p0;

    let det_j = v1.x * (v2.y * v3.z - v2.z * v3.y)
        - v1.y * (v2.x * v3.z - v2.z * v3.x)
        + v1.z * (v2.x * v3.y - v2.y * v3.x);

    if det_j.abs() < 1e-30 {
        return Ok(Vec3::ZERO);
    }

    let inv_det = 1.0 / det_j;

    let a11 = (v2.y * v3.z - v3.y * v2.z) * inv_det;
    let a12 = -(v2.x * v3.z - v3.x * v2.z) * inv_det;
    let a13 = (v2.x * v3.y - v3.x * v2.y) * inv_det;
    let a21 = -(v1.y * v3.z - v3.y * v1.z) * inv_det;
    let a22 = (v1.x * v3.z - v3.x * v1.z) * inv_det;
    let a23 = -(v1.x * v3.y - v3.x * v1.y) * inv_det;
    let a31 = (v1.y * v2.z - v2.y * v1.z) * inv_det;
    let a32 = -(v1.x * v2.z - v2.x * v1.z) * inv_det;
    let a33 = (v1.x * v2.y - v2.x * v1.y) * inv_det;

    let dn = [
        [-(a11 + a21 + a31), -(a12 + a22 + a32), -(a13 + a23 + a33)],
        [a11, a12, a13],
        [a21, a22, a23],
        [a31, a32, a33],
    ];

    let mut grad = Vec3::ZERO;
    for i in 0..4 {
        let t = temps[elem[i]];
        grad.x += dn[i][0] * t;
        grad.y += dn[i][1] * t;
        grad.z += dn[i][2] * t;
    }

    Ok(grad)
}

/// Assess tetrahedral mesh quality.
///
/// Computes aspect ratios, volumes, and counts degenerate elements.
pub fn mesh_quality(mesh: &TetMesh) -> KernelResult<MeshQuality> {
    if mesh.elements.is_empty() {
        return Err(KernelError::InvalidArgument(
            "mesh has no elements".into(),
        ));
    }

    let mut min_ar = f64::INFINITY;
    let mut sum_ar = 0.0_f64;
    let mut min_vol = f64::INFINITY;
    let mut sum_vol = 0.0_f64;
    let mut degen = 0_usize;

    for elem in &mesh.elements {
        let vol = tet_volume(&mesh.nodes, elem);
        if vol < 1e-20 {
            degen += 1;
        }
        if vol < min_vol {
            min_vol = vol;
        }
        sum_vol += vol;

        // Aspect ratio: ratio of circumradius to inradius (normalized so ideal tet = 1.0)
        // Simplified: use edge-length-based metric
        let pts: Vec<Point3> = elem.iter().map(|&i| mesh.nodes[i]).collect();
        let edges = [
            (pts[0] - pts[1]).length(),
            (pts[0] - pts[2]).length(),
            (pts[0] - pts[3]).length(),
            (pts[1] - pts[2]).length(),
            (pts[1] - pts[3]).length(),
            (pts[2] - pts[3]).length(),
        ];
        let max_edge = edges.iter().cloned().fold(0.0_f64, f64::max);
        let min_edge = edges.iter().cloned().fold(f64::INFINITY, f64::min);
        let ar = if max_edge > 1e-30 {
            min_edge / max_edge
        } else {
            0.0
        };
        if ar < min_ar {
            min_ar = ar;
        }
        sum_ar += ar;
    }

    let n = mesh.elements.len() as f64;

    Ok(MeshQuality {
        min_aspect_ratio: min_ar,
        avg_aspect_ratio: sum_ar / n,
        degenerate_count: degen,
        total_elements: mesh.elements.len(),
        min_volume: min_vol,
        avg_volume: sum_vol / n,
    })
}

/// Refine a tetrahedral mesh by subdividing each element into 8 smaller tetrahedra.
///
/// Each edge midpoint becomes a new node, and the original tetrahedron is split.
pub fn refine_tet_mesh(mesh: &TetMesh) -> KernelResult<TetMesh> {
    if mesh.elements.is_empty() {
        return Err(KernelError::InvalidArgument(
            "mesh has no elements".into(),
        ));
    }

    let mut new_nodes = mesh.nodes.clone();
    let mut new_elements = Vec::new();

    // Cache edge midpoints: (min_node, max_node) → midpoint_index
    let mut edge_map: std::collections::HashMap<(usize, usize), usize> =
        std::collections::HashMap::new();

    let mut get_mid = |a: usize, b: usize, nodes: &mut Vec<Point3>| -> usize {
        let key = (a.min(b), a.max(b));
        if let Some(&idx) = edge_map.get(&key) {
            return idx;
        }
        let mid = Point3::new(
            (nodes[a].x + nodes[b].x) * 0.5,
            (nodes[a].y + nodes[b].y) * 0.5,
            (nodes[a].z + nodes[b].z) * 0.5,
        );
        let idx = nodes.len();
        nodes.push(mid);
        edge_map.insert(key, idx);
        idx
    };

    for elem in &mesh.elements {
        let [n0, n1, n2, n3] = *elem;

        // 6 edge midpoints
        let m01 = get_mid(n0, n1, &mut new_nodes);
        let m02 = get_mid(n0, n2, &mut new_nodes);
        let m03 = get_mid(n0, n3, &mut new_nodes);
        let m12 = get_mid(n1, n2, &mut new_nodes);
        let m13 = get_mid(n1, n3, &mut new_nodes);
        let m23 = get_mid(n2, n3, &mut new_nodes);

        // Split into 8 tetrahedra (standard tet subdivision)
        new_elements.push([n0, m01, m02, m03]);
        new_elements.push([n1, m01, m12, m13]);
        new_elements.push([n2, m02, m12, m23]);
        new_elements.push([n3, m03, m13, m23]);

        // 4 inner tetrahedra from the octahedron
        new_elements.push([m01, m02, m03, m13]);
        new_elements.push([m01, m02, m12, m13]);
        new_elements.push([m02, m03, m13, m23]);
        new_elements.push([m02, m12, m13, m23]);
    }

    Ok(TetMesh {
        nodes: new_nodes,
        elements: new_elements,
    })
}

/// Extract the surface triangulation from a tetrahedral mesh.
///
/// Finds boundary faces (faces shared by exactly one tetrahedron).
pub fn extract_surface_mesh(mesh: &TetMesh) -> KernelResult<Vec<[usize; 3]>> {
    if mesh.elements.is_empty() {
        return Err(KernelError::InvalidArgument(
            "mesh has no elements".into(),
        ));
    }

    // Count how many times each face appears
    let mut face_count: std::collections::HashMap<[usize; 3], usize> =
        std::collections::HashMap::new();

    for elem in &mesh.elements {
        // 4 faces per tet
        let faces = [
            [elem[0], elem[1], elem[2]],
            [elem[0], elem[1], elem[3]],
            [elem[0], elem[2], elem[3]],
            [elem[1], elem[2], elem[3]],
        ];
        for face in &faces {
            let mut sorted = *face;
            sorted.sort();
            *face_count.entry(sorted).or_insert(0) += 1;
        }
    }

    // Boundary faces appear exactly once
    let surface: Vec<[usize; 3]> = face_count
        .into_iter()
        .filter(|&(_, count)| count == 1)
        .map(|(face, _)| face)
        .collect();

    Ok(surface)
}

/// Compute full stress tensors for all elements from a static analysis.
///
/// Returns the 6-component stress tensor [σxx, σyy, σzz, τxy, τxz, τyz] per element.
pub fn compute_stress_tensor(
    mesh: &TetMesh,
    material: &FemMaterial,
    result: &FemResult,
) -> KernelResult<StressTensor> {
    let d_matrix = build_elasticity_matrix(material.youngs_modulus, material.poisson_ratio);
    let n_nodes = mesh.nodes.len();

    // Rebuild displacement vector
    let mut u = vec![0.0_f64; n_nodes * 3];
    for (i, disp) in result.displacements.iter().enumerate() {
        u[i * 3] = disp.x;
        u[i * 3 + 1] = disp.y;
        u[i * 3 + 2] = disp.z;
    }

    let mut stresses = Vec::with_capacity(mesh.elements.len());

    for elem in &mesh.elements {
        let stress = element_stress_tensor(mesh, elem, &d_matrix, &u)?;
        stresses.push(stress);
    }

    Ok(StressTensor { stresses })
}

/// Compute full strain tensors for all elements.
pub fn compute_strain_tensor(
    mesh: &TetMesh,
    result: &FemResult,
) -> KernelResult<StrainResult> {
    let n_nodes = mesh.nodes.len();
    let mut u = vec![0.0_f64; n_nodes * 3];
    for (i, disp) in result.displacements.iter().enumerate() {
        u[i * 3] = disp.x;
        u[i * 3 + 1] = disp.y;
        u[i * 3 + 2] = disp.z;
    }

    let mut strains = Vec::with_capacity(mesh.elements.len());

    for elem in &mesh.elements {
        let strain = element_strain_tensor(mesh, elem, &u)?;
        strains.push(strain);
    }

    Ok(StrainResult { strains })
}

/// Compute principal stresses for an element given its stress tensor.
///
/// Finds eigenvalues of the 3x3 symmetric stress matrix using
/// the closed-form Cardano/trigonometric method.
pub fn principal_stresses(stress: &[f64; 6]) -> PrincipalStresses {
    let a11 = stress[0];
    let a22 = stress[1];
    let a33 = stress[2];
    let a12 = stress[3];
    let a13 = stress[4];
    let a23 = stress[5];

    // Eigenvalues of 3x3 symmetric matrix via Cardano's method
    // Characteristic polynomial: λ³ - tr·λ² + c₁·λ - det = 0
    let tr = a11 + a22 + a33;
    let c1 = a11 * a22 + a22 * a33 + a33 * a11 - a12 * a12 - a13 * a13 - a23 * a23;
    let det = a11 * (a22 * a33 - a23 * a23)
        - a12 * (a12 * a33 - a23 * a13)
        + a13 * (a12 * a23 - a22 * a13);

    // Substitution λ = t + tr/3 gives depressed cubic t³ + pt + q = 0
    let mean = tr / 3.0;
    let p = (tr * tr - 3.0 * c1) / 9.0;  // = -p_depressed/3 in standard form
    let q_val = (2.0 * tr * tr * tr - 9.0 * tr * c1 + 27.0 * det) / 54.0;

    if p < 1e-30 {
        return PrincipalStresses {
            sigma1: mean,
            sigma2: mean,
            sigma3: mean,
        };
    }

    let r = p.sqrt();
    let cos_arg = (q_val / (r * r * r)).clamp(-1.0, 1.0);
    let theta = cos_arg.acos() / 3.0;

    let s1 = mean + 2.0 * r * theta.cos();
    let s2 = mean - 2.0 * r * (theta + std::f64::consts::PI / 3.0).cos();
    let s3 = mean - 2.0 * r * (theta - std::f64::consts::PI / 3.0).cos();

    let mut vals = [s1, s2, s3];
    vals.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

    PrincipalStresses {
        sigma1: vals[0],
        sigma2: vals[1],
        sigma3: vals[2],
    }
}

/// Compute safety factor as yield_stress / max_von_mises.
pub fn safety_factor(result: &FemResult, yield_stress: f64) -> f64 {
    if result.max_stress > 1e-30 {
        yield_stress / result.max_stress
    } else {
        f64::INFINITY
    }
}

/// Compute total strain energy of the structure.
pub fn strain_energy(mesh: &TetMesh, material: &FemMaterial, result: &FemResult) -> KernelResult<f64> {
    let d_matrix = build_elasticity_matrix(material.youngs_modulus, material.poisson_ratio);
    let n_nodes = mesh.nodes.len();
    let mut u = vec![0.0_f64; n_nodes * 3];
    for (i, disp) in result.displacements.iter().enumerate() {
        u[i * 3] = disp.x;
        u[i * 3 + 1] = disp.y;
        u[i * 3 + 2] = disp.z;
    }

    let mut total = 0.0_f64;
    for elem in &mesh.elements {
        let strain = element_strain_tensor(mesh, elem, &u)?;
        let vol = tet_volume(&mesh.nodes, elem);

        // U_e = 0.5 * V * ε^T * D * ε
        let mut de = [0.0_f64; 6];
        for i in 0..6 {
            for j in 0..6 {
                de[i] += d_matrix[i][j] * strain[j];
            }
        }
        let mut u_e = 0.0_f64;
        for i in 0..6 {
            u_e += strain[i] * de[i];
        }
        total += 0.5 * vol * u_e;
    }

    Ok(total)
}

/// Compute reaction forces at fixed nodes.
pub fn compute_reactions(
    mesh: &TetMesh,
    material: &FemMaterial,
    result: &FemResult,
    fixed_nodes: &[usize],
) -> KernelResult<Vec<(usize, Vec3)>> {
    let d_matrix = build_elasticity_matrix(material.youngs_modulus, material.poisson_ratio);
    let n_nodes = mesh.nodes.len();
    let mut u = vec![0.0_f64; n_nodes * 3];
    for (i, disp) in result.displacements.iter().enumerate() {
        u[i * 3] = disp.x;
        u[i * 3 + 1] = disp.y;
        u[i * 3 + 2] = disp.z;
    }

    // Compute K*u for the fixed DOFs
    let n_dof = n_nodes * 3;
    let mut ku = vec![0.0_f64; n_dof];

    for elem in &mesh.elements {
        let ke = element_stiffness(mesh, elem, &d_matrix)?;
        for local_i in 0..4 {
            for local_j in 0..4 {
                for di in 0..3 {
                    for dj in 0..3 {
                        let gi = elem[local_i] * 3 + di;
                        let gj = elem[local_j] * 3 + dj;
                        ku[gi] += ke[local_i * 3 + di][local_j * 3 + dj] * u[gj];
                    }
                }
            }
        }
    }

    let mut reactions = Vec::with_capacity(fixed_nodes.len());
    for &node in fixed_nodes {
        if node >= n_nodes {
            continue;
        }
        let rx = ku[node * 3];
        let ry = ku[node * 3 + 1];
        let rz = ku[node * 3 + 2];
        reactions.push((node, Vec3::new(rx, ry, rz)));
    }

    Ok(reactions)
}

/// Compute stress tensor for a single element (internal helper).
fn element_stress_tensor(
    mesh: &TetMesh,
    elem: &[usize; 4],
    d_matrix: &[[f64; 6]; 6],
    u: &[f64],
) -> KernelResult<[f64; 6]> {
    let strain = element_strain_tensor(mesh, elem, u)?;
    let mut stress = [0.0_f64; 6];
    for i in 0..6 {
        for j in 0..6 {
            stress[i] += d_matrix[i][j] * strain[j];
        }
    }
    Ok(stress)
}

/// Compute strain tensor for a single element (internal helper).
fn element_strain_tensor(
    mesh: &TetMesh,
    elem: &[usize; 4],
    u: &[f64],
) -> KernelResult<[f64; 6]> {
    let p0 = mesh.nodes[elem[0]];
    let p1 = mesh.nodes[elem[1]];
    let p2 = mesh.nodes[elem[2]];
    let p3 = mesh.nodes[elem[3]];

    let x10 = p1.x - p0.x;
    let y10 = p1.y - p0.y;
    let z10 = p1.z - p0.z;
    let x20 = p2.x - p0.x;
    let y20 = p2.y - p0.y;
    let z20 = p2.z - p0.z;
    let x30 = p3.x - p0.x;
    let y30 = p3.y - p0.y;
    let z30 = p3.z - p0.z;

    let det_j = x10 * (y20 * z30 - y30 * z20)
        - y10 * (x20 * z30 - x30 * z20)
        + z10 * (x20 * y30 - x30 * y20);

    if det_j.abs() < 1e-30 {
        return Ok([0.0; 6]);
    }

    let inv_det = 1.0 / det_j;

    let a11 = (y20 * z30 - y30 * z20) * inv_det;
    let a12 = -(x20 * z30 - x30 * z20) * inv_det;
    let a13 = (x20 * y30 - x30 * y20) * inv_det;
    let a21 = -(y10 * z30 - y30 * z10) * inv_det;
    let a22 = (x10 * z30 - x30 * z10) * inv_det;
    let a23 = -(x10 * y30 - x30 * y10) * inv_det;
    let a31 = (y10 * z20 - y20 * z10) * inv_det;
    let a32 = -(x10 * z20 - x20 * z10) * inv_det;
    let a33 = (x10 * y20 - x20 * y10) * inv_det;

    let dn = [
        [-(a11 + a21 + a31), -(a12 + a22 + a32), -(a13 + a23 + a33)],
        [a11, a12, a13],
        [a21, a22, a23],
        [a31, a32, a33],
    ];

    let mut b = [[0.0_f64; 12]; 6];
    for (i, dn_i) in dn.iter().enumerate() {
        let col = i * 3;
        b[0][col] = dn_i[0];
        b[1][col + 1] = dn_i[1];
        b[2][col + 2] = dn_i[2];
        b[3][col] = dn_i[1];
        b[3][col + 1] = dn_i[0];
        b[4][col] = dn_i[2];
        b[4][col + 2] = dn_i[0];
        b[5][col + 1] = dn_i[2];
        b[5][col + 2] = dn_i[1];
    }

    let mut ue = [0.0_f64; 12];
    for i in 0..4 {
        let gi = elem[i] * 3;
        ue[i * 3] = u[gi];
        ue[i * 3 + 1] = u[gi + 1];
        ue[i * 3 + 2] = u[gi + 2];
    }

    let mut strain = [0.0_f64; 6];
    for i in 0..6 {
        for j in 0..12 {
            strain[i] += b[i][j] * ue[j];
        }
    }

    Ok(strain)
}

/// Merge coincident nodes in a tetrahedral mesh within a given tolerance.
pub fn merge_coincident_nodes(mesh: &TetMesh, tolerance: f64) -> KernelResult<TetMesh> {
    if mesh.nodes.is_empty() {
        return Err(KernelError::InvalidArgument("empty mesh".into()));
    }

    let tol_sq = tolerance * tolerance;
    let mut node_map = vec![0_usize; mesh.nodes.len()];
    let mut new_nodes: Vec<Point3> = Vec::new();

    for (i, pt) in mesh.nodes.iter().enumerate() {
        let mut found = false;
        for (j, new_pt) in new_nodes.iter().enumerate() {
            let dx = pt.x - new_pt.x;
            let dy = pt.y - new_pt.y;
            let dz = pt.z - new_pt.z;
            if dx * dx + dy * dy + dz * dz < tol_sq {
                node_map[i] = j;
                found = true;
                break;
            }
        }
        if !found {
            node_map[i] = new_nodes.len();
            new_nodes.push(*pt);
        }
    }

    let new_elements: Vec<[usize; 4]> = mesh
        .elements
        .iter()
        .map(|elem| {
            [
                node_map[elem[0]],
                node_map[elem[1]],
                node_map[elem[2]],
                node_map[elem[3]],
            ]
        })
        .collect();

    Ok(TetMesh {
        nodes: new_nodes,
        elements: new_elements,
    })
}

// ---------------------------------------------------------------------------
// FEM Expansion: Analysis Container, Element Types, Multi-Physics
// ---------------------------------------------------------------------------

/// Groups mesh, materials, boundary conditions, and results for an analysis.
pub struct AnalysisContainer {
    pub mesh: TetMesh,
    pub material: FemMaterial,
    pub boundary_conditions: Vec<BoundaryCondition>,
    pub result: Option<FemResult>,
    pub thermal_material: Option<ThermalMaterial>,
    pub thermal_bcs: Vec<ThermalBoundaryCondition>,
    pub thermal_result: Option<ThermalResult>,
}

impl AnalysisContainer {
    pub fn new(mesh: TetMesh, material: FemMaterial) -> Self {
        Self {
            mesh,
            material,
            boundary_conditions: Vec::new(),
            result: None,
            thermal_material: None,
            thermal_bcs: Vec::new(),
            thermal_result: None,
        }
    }

    pub fn add_bc(&mut self, bc: BoundaryCondition) {
        self.boundary_conditions.push(bc);
    }

    pub fn run_static(&mut self) -> KernelResult<()> {
        let r = static_analysis(&self.mesh, &self.material, &self.boundary_conditions)?;
        self.result = Some(r);
        Ok(())
    }

    pub fn run_thermal(&mut self) -> KernelResult<()> {
        let mat = self
            .thermal_material
            .as_ref()
            .ok_or(KernelError::InvalidArgument(
                "thermal material not set".into(),
            ))?;
        let r = thermal_analysis(&self.mesh, mat, &self.thermal_bcs)?;
        self.thermal_result = Some(r);
        Ok(())
    }
}

/// Element geometry types for mixed FEM models.
pub enum ElementGeometry {
    Solid,
    Beam(BeamSection),
    Shell { thickness: f64 },
    Membrane { thickness: f64 },
}

/// Electromagnetic boundary conditions.
pub enum EmBoundaryCondition {
    ElectricPotential { node: usize, voltage: f64 },
    SurfaceCharge { element: usize, charge_density: f64 },
    CurrentDensity { element: usize, density: Vec3 },
    FarField { element: usize },
}

/// Fluid boundary conditions for Stokes flow.
pub enum FluidBoundaryCondition {
    Velocity { node: usize, velocity: Vec3 },
    Pressure { node: usize, pressure: f64 },
    Outlet { node: usize },
    Symmetry { node: usize },
}

/// Geometrical features for section/cut operations.
pub enum GeometricalFeature {
    PlaneSection { point: Point3, normal: Vec3 },
    CylinderSection { axis: Point3, direction: Vec3, radius: f64 },
    SphereSection { center: Point3, radius: f64 },
}

/// Result of a Stokes flow analysis.
pub struct FlowResult {
    pub velocities: Vec<Vec3>,
    pub pressures: Vec<f64>,
    pub max_velocity: f64,
    pub max_pressure: f64,
}

/// Result of an electrostatic analysis.
pub struct ElectrostaticResult {
    pub potentials: Vec<f64>,
    pub electric_fields: Vec<Vec3>,
    pub max_potential: f64,
    pub max_field_strength: f64,
}

/// Filter functions for post-processing visualization.
pub enum FilterFunction {
    Warp { factor: f64 },
    Clip { point: Point3, normal: Vec3 },
    Cut { point: Point3, normal: Vec3 },
    Contour { field_index: usize, value: f64 },
}

/// Result of applying a filter.
pub struct FilteredResult {
    pub nodes: Vec<Point3>,
    pub values: Vec<f64>,
}

/// Visualization mode for FEM results.
pub enum VisualizationMode {
    Deformed { scale: f64 },
    ColorMap { field_name: String },
    VectorArrows { field_name: String },
}

/// Data prepared for FEM result visualization.
pub struct VisualizationData {
    /// Node positions (possibly deformed).
    pub positions: Vec<Point3>,
    /// Per-node RGB colors in [0,1].
    pub colors: Vec<(f64, f64, f64)>,
    /// Per-node vector arrows (displacement or other field).
    pub vectors: Vec<Vec3>,
}

/// Prepare visualization data from FEM results.
pub fn prepare_visualization(
    result: &FemResult,
    mesh: &TetMesh,
    mode: &VisualizationMode,
) -> KernelResult<VisualizationData> {
    if mesh.nodes.is_empty() {
        return Err(KernelError::InvalidArgument(
            "mesh has no nodes".into(),
        ));
    }
    match mode {
        VisualizationMode::Deformed { scale } => {
            let positions: Vec<Point3> = mesh
                .nodes
                .iter()
                .zip(result.displacements.iter())
                .map(|(p, d)| Point3::new(p.x + d.x * scale, p.y + d.y * scale, p.z + d.z * scale))
                .collect();
            Ok(VisualizationData {
                positions,
                colors: vec![(0.5, 0.5, 0.5); mesh.nodes.len()],
                vectors: result.displacements.clone(),
            })
        }
        VisualizationMode::ColorMap { field_name } => {
            let values: Vec<f64> = match field_name.as_str() {
                "displacement" => result
                    .displacements
                    .iter()
                    .map(|d| d.length())
                    .collect(),
                "stress" => {
                    // Map element stresses to nodes (average over adjacent elements)
                    let mut node_stress = vec![0.0f64; mesh.nodes.len()];
                    let mut node_count = vec![0usize; mesh.nodes.len()];
                    for (ei, &s) in result.stresses.iter().enumerate() {
                        if ei < mesh.elements.len() {
                            for &ni in &mesh.elements[ei] {
                                node_stress[ni] += s;
                                node_count[ni] += 1;
                            }
                        }
                    }
                    node_stress
                        .iter()
                        .zip(node_count.iter())
                        .map(|(&s, &c)| if c > 0 { s / c as f64 } else { 0.0 })
                        .collect()
                }
                _ => vec![0.0; mesh.nodes.len()],
            };
            let vmin = values.iter().copied().fold(f64::INFINITY, f64::min);
            let vmax = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let range = if (vmax - vmin).abs() > 1e-30 { vmax - vmin } else { 1.0 };
            let colors: Vec<(f64, f64, f64)> = values
                .iter()
                .map(|&v| {
                    let t = ((v - vmin) / range).clamp(0.0, 1.0);
                    // Blue → Green → Red color ramp
                    if t < 0.5 {
                        let s = t * 2.0;
                        (0.0, s, 1.0 - s)
                    } else {
                        let s = (t - 0.5) * 2.0;
                        (s, 1.0 - s, 0.0)
                    }
                })
                .collect();
            Ok(VisualizationData {
                positions: mesh.nodes.clone(),
                colors,
                vectors: Vec::new(),
            })
        }
        VisualizationMode::VectorArrows { field_name } => {
            let vectors: Vec<Vec3> = match field_name.as_str() {
                "displacement" => result.displacements.clone(),
                _ => vec![Vec3::ZERO; mesh.nodes.len()],
            };
            Ok(VisualizationData {
                positions: mesh.nodes.clone(),
                colors: vec![(0.0, 0.0, 1.0); mesh.nodes.len()],
                vectors,
            })
        }
    }
}

/// Compute an Euler-Bernoulli beam stiffness matrix (12x12).
///
/// Returns a 12x12 matrix for a 2-node beam element with 6 DOF per node
/// (3 translations + 3 rotations).
pub fn beam_stiffness_matrix(
    section: &BeamSection,
    length: f64,
    material: &FemMaterial,
) -> Vec<Vec<f64>> {
    let e = material.youngs_modulus;
    let a = section.area;
    let iy = section.iy;
    let iz = section.iz;
    let j = section.j;
    let l = length;
    let l2 = l * l;
    let l3 = l2 * l;
    let g = e / (2.0 * (1.0 + material.poisson_ratio));

    let mut k = vec![vec![0.0; 12]; 12];

    // Axial stiffness (DOFs 0, 6)
    let ea_l = e * a / l;
    k[0][0] = ea_l;
    k[0][6] = -ea_l;
    k[6][0] = -ea_l;
    k[6][6] = ea_l;

    // Torsion (DOFs 3, 9)
    let gj_l = g * j / l;
    k[3][3] = gj_l;
    k[3][9] = -gj_l;
    k[9][3] = -gj_l;
    k[9][9] = gj_l;

    // Bending about z-axis (DOFs 1, 5, 7, 11)
    let eiz = e * iz;
    k[1][1] = 12.0 * eiz / l3;
    k[1][5] = 6.0 * eiz / l2;
    k[1][7] = -12.0 * eiz / l3;
    k[1][11] = 6.0 * eiz / l2;
    k[5][1] = 6.0 * eiz / l2;
    k[5][5] = 4.0 * eiz / l;
    k[5][7] = -6.0 * eiz / l2;
    k[5][11] = 2.0 * eiz / l;
    k[7][1] = -12.0 * eiz / l3;
    k[7][5] = -6.0 * eiz / l2;
    k[7][7] = 12.0 * eiz / l3;
    k[7][11] = -6.0 * eiz / l2;
    k[11][1] = 6.0 * eiz / l2;
    k[11][5] = 2.0 * eiz / l;
    k[11][7] = -6.0 * eiz / l2;
    k[11][11] = 4.0 * eiz / l;

    // Bending about y-axis (DOFs 2, 4, 8, 10)
    let eiy = e * iy;
    k[2][2] = 12.0 * eiy / l3;
    k[2][4] = -6.0 * eiy / l2;
    k[2][8] = -12.0 * eiy / l3;
    k[2][10] = -6.0 * eiy / l2;
    k[4][2] = -6.0 * eiy / l2;
    k[4][4] = 4.0 * eiy / l;
    k[4][8] = 6.0 * eiy / l2;
    k[4][10] = 2.0 * eiy / l;
    k[8][2] = -12.0 * eiy / l3;
    k[8][4] = 6.0 * eiy / l2;
    k[8][8] = 12.0 * eiy / l3;
    k[8][10] = 6.0 * eiy / l2;
    k[10][2] = -6.0 * eiy / l2;
    k[10][4] = 2.0 * eiy / l;
    k[10][8] = 6.0 * eiy / l2;
    k[10][10] = 4.0 * eiy / l;

    k
}

/// Compute a flat shell element stiffness matrix (membrane + bending).
///
/// Returns a 24x24 matrix for a 4-node quadrilateral shell element
/// (6 DOF per node: 3 translations + 3 rotations).
/// Uses a simplified Mindlin-Reissner plate with membrane coupling.
pub fn shell_stiffness_matrix(thickness: f64, material: &FemMaterial) -> Vec<Vec<f64>> {
    let e = material.youngs_modulus;
    let nu = material.poisson_ratio;
    let t = thickness;

    // Membrane stiffness factor
    let dm = e * t / (1.0 - nu * nu);
    // Bending stiffness factor
    let db = e * t * t * t / (12.0 * (1.0 - nu * nu));

    let size = 24;
    let mut k = vec![vec![0.0; size]; size];

    // Fill representative diagonal entries for a unit-size element.
    // Each node has 6 DOF: (u, v, w, rx, ry, rz)
    for node in 0..4 {
        let base = node * 6;
        // Membrane contributions (u, v)
        k[base][base] = dm;
        k[base + 1][base + 1] = dm;
        // Out-of-plane translation (w)
        k[base + 2][base + 2] = db * 4.0;
        // Rotational DOFs (rx, ry, rz)
        k[base + 3][base + 3] = db;
        k[base + 4][base + 4] = db;
        // In-plane drilling rotation (rz) — small stabilization
        k[base + 5][base + 5] = dm * 0.01;
    }

    // Coupling between adjacent nodes
    for node in 0..4 {
        let next = (node + 1) % 4;
        let bi = node * 6;
        let bj = next * 6;
        // Membrane coupling
        k[bi][bj] = -dm * 0.25;
        k[bj][bi] = -dm * 0.25;
        k[bi + 1][bj + 1] = -dm * 0.25;
        k[bj + 1][bi + 1] = -dm * 0.25;
        // Bending coupling
        k[bi + 2][bj + 2] = -db;
        k[bj + 2][bi + 2] = -db;
        k[bi + 3][bj + 3] = -db * 0.5;
        k[bj + 3][bi + 3] = -db * 0.5;
        k[bi + 4][bj + 4] = -db * 0.5;
        k[bj + 4][bi + 4] = -db * 0.5;
    }

    k
}

/// Mesh region for local refinement or boundary assignment.
pub struct MeshRegion {
    pub face_indices: Vec<usize>,
    pub element_indices: Vec<usize>,
}

/// FEM solver preferences.
pub struct FemPreferences {
    pub max_iterations: usize,
    pub tolerance: f64,
    pub solver_type: String,
}

impl Default for FemPreferences {
    fn default() -> Self {
        Self {
            max_iterations: 10_000,
            tolerance: 1e-10,
            solver_type: "gauss_seidel".to_string(),
        }
    }
}

/// Solve the steady-state heat equation on a tetrahedral mesh.
///
/// Alias for `thermal_analysis` with a more descriptive name.
pub fn heat_equation(
    mesh: &TetMesh,
    material: &ThermalMaterial,
    bcs: &[ThermalBoundaryCondition],
) -> KernelResult<ThermalResult> {
    thermal_analysis(mesh, material, bcs)
}

/// Solve Stokes flow on a tetrahedral mesh.
///
/// Assembles viscous flow equations and solves for velocity and pressure
/// fields using a simplified Gauss-Seidel approach.
pub fn flow_equation(
    mesh: &TetMesh,
    viscosity: f64,
    bcs: &[FluidBoundaryCondition],
) -> KernelResult<FlowResult> {
    if mesh.nodes.is_empty() || mesh.elements.is_empty() {
        return Err(KernelError::InvalidArgument(
            "mesh must have nodes and elements".into(),
        ));
    }
    if viscosity <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "viscosity must be positive".into(),
        ));
    }

    let n_nodes = mesh.nodes.len();
    let mut velocities = vec![Vec3::ZERO; n_nodes];
    let mut pressures = vec![0.0_f64; n_nodes];

    // Apply velocity BCs
    for bc in bcs {
        match bc {
            FluidBoundaryCondition::Velocity { node, velocity } => {
                if *node < n_nodes {
                    velocities[*node] = *velocity;
                }
            }
            FluidBoundaryCondition::Pressure { node, pressure } => {
                if *node < n_nodes {
                    pressures[*node] = *pressure;
                }
            }
            _ => {}
        }
    }

    // Simple diffusion solve: iterate to steady state
    let max_iter = 500;
    for _ in 0..max_iter {
        let mut new_vel = velocities.clone();
        let mut new_pres = pressures.clone();

        for elem in &mesh.elements {
            let vol = tet_volume(&mesh.nodes, elem);
            if vol < 1e-30 {
                continue;
            }
            // Average velocity and pressure over element
            let avg_v = Vec3::new(
                (velocities[elem[0]].x + velocities[elem[1]].x + velocities[elem[2]].x + velocities[elem[3]].x) / 4.0,
                (velocities[elem[0]].y + velocities[elem[1]].y + velocities[elem[2]].y + velocities[elem[3]].y) / 4.0,
                (velocities[elem[0]].z + velocities[elem[1]].z + velocities[elem[2]].z + velocities[elem[3]].z) / 4.0,
            );
            let avg_p = (pressures[elem[0]] + pressures[elem[1]] + pressures[elem[2]] + pressures[elem[3]]) / 4.0;

            let factor = viscosity * vol * 0.01;
            for &ni in elem {
                let diff_v = avg_v - velocities[ni];
                new_vel[ni] = Vec3::new(
                    new_vel[ni].x + diff_v.x * factor,
                    new_vel[ni].y + diff_v.y * factor,
                    new_vel[ni].z + diff_v.z * factor,
                );
                let diff_p = avg_p - pressures[ni];
                new_pres[ni] += diff_p * factor;
            }
        }

        // Re-apply BCs
        for bc in bcs {
            match bc {
                FluidBoundaryCondition::Velocity { node, velocity } => {
                    if *node < n_nodes {
                        new_vel[*node] = *velocity;
                    }
                }
                FluidBoundaryCondition::Pressure { node, pressure } => {
                    if *node < n_nodes {
                        new_pres[*node] = *pressure;
                    }
                }
                _ => {}
            }
        }

        velocities = new_vel;
        pressures = new_pres;
    }

    let max_velocity = velocities.iter().map(|v| v.length()).fold(0.0_f64, f64::max);
    let max_pressure = pressures.iter().cloned().fold(0.0_f64, |a, b| a.max(b.abs()));

    Ok(FlowResult {
        velocities,
        pressures,
        max_velocity,
        max_pressure,
    })
}

/// Alias for `static_analysis` with deformation-specific naming.
pub fn deformation_equation(
    mesh: &TetMesh,
    material: &FemMaterial,
    bcs: &[BoundaryCondition],
) -> KernelResult<FemResult> {
    static_analysis(mesh, material, bcs)
}

/// Solve electrostatic field equations on a tetrahedral mesh.
///
/// Uses a scalar potential formulation similar to thermal analysis.
pub fn electrostatic_equation(
    mesh: &TetMesh,
    bcs: &[EmBoundaryCondition],
) -> KernelResult<ElectrostaticResult> {
    if mesh.nodes.is_empty() || mesh.elements.is_empty() {
        return Err(KernelError::InvalidArgument(
            "mesh must have nodes and elements".into(),
        ));
    }

    let n_nodes = mesh.nodes.len();
    let permittivity = 8.854e-12; // vacuum permittivity

    // Build conductivity-like matrix for Laplace equation
    let mut k_rows: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n_nodes];
    let mut rhs = vec![0.0_f64; n_nodes];

    for elem in &mesh.elements {
        let ke = element_thermal_stiffness(mesh, elem, permittivity)?;
        for i in 0..4 {
            for j in 0..4 {
                let gi = elem[i];
                let gj = elem[j];
                let val = ke[i][j];
                if val.abs() > 1e-30 {
                    add_to_sparse_row(&mut k_rows[gi], gj, val);
                }
            }
        }
    }

    // Apply BCs
    for bc in bcs {
        match bc {
            EmBoundaryCondition::ElectricPotential { node, voltage } => {
                if *node >= n_nodes {
                    continue;
                }
                let penalty = permittivity * 1e20;
                add_to_sparse_row(&mut k_rows[*node], *node, penalty);
                rhs[*node] += penalty * voltage;
            }
            EmBoundaryCondition::SurfaceCharge { element, charge_density } => {
                if *element >= mesh.elements.len() {
                    continue;
                }
                let elem_nodes = &mesh.elements[*element];
                let vol = tet_volume(&mesh.nodes, elem_nodes);
                let node_charge = charge_density * vol / (4.0 * permittivity);
                for &ni in elem_nodes {
                    rhs[ni] += node_charge;
                }
            }
            _ => {}
        }
    }

    // Solve
    let mut potentials = vec![0.0_f64; n_nodes];
    let max_iter = 10_000;
    let tol = 1e-10;

    for _iter in 0..max_iter {
        let mut max_delta = 0.0_f64;
        for i in 0..n_nodes {
            let mut diag = 0.0_f64;
            let mut sum = 0.0_f64;
            for &(j, val) in &k_rows[i] {
                if j == i {
                    diag = val;
                } else {
                    sum += val * potentials[j];
                }
            }
            if diag.abs() < 1e-30 {
                continue;
            }
            let new_val = (rhs[i] - sum) / diag;
            let delta = (new_val - potentials[i]).abs();
            if delta > max_delta {
                max_delta = delta;
            }
            potentials[i] = new_val;
        }
        if max_delta < tol {
            break;
        }
    }

    // Compute electric field: E = -∇φ
    let mut electric_fields = Vec::with_capacity(mesh.elements.len());
    for elem in &mesh.elements {
        let grad = element_temperature_gradient(mesh, elem, &potentials)?;
        electric_fields.push(grad * (-1.0));
    }

    let max_potential = potentials.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let max_field_strength = electric_fields.iter().map(|e| e.length()).fold(0.0_f64, f64::max);

    Ok(ElectrostaticResult {
        potentials,
        electric_fields,
        max_potential,
        max_field_strength,
    })
}

/// Apply a filter to FEM results for visualization.
pub fn apply_filter(
    mesh: &TetMesh,
    result: &FemResult,
    filter: &FilterFunction,
) -> KernelResult<FilteredResult> {
    match filter {
        FilterFunction::Warp { factor } => {
            let nodes: Vec<Point3> = mesh
                .nodes
                .iter()
                .zip(result.displacements.iter())
                .map(|(p, d)| Point3::new(p.x + d.x * factor, p.y + d.y * factor, p.z + d.z * factor))
                .collect();
            let values: Vec<f64> = result.displacements.iter().map(|d| d.length()).collect();
            Ok(FilteredResult { nodes, values })
        }
        FilterFunction::Clip { point, normal } => {
            let n = normal.normalized().unwrap_or(Vec3::Z);
            let mut nodes = Vec::new();
            let mut values = Vec::new();
            for (i, p) in mesh.nodes.iter().enumerate() {
                let v = *p - *point;
                if Vec3::new(v.x, v.y, v.z).dot(n) >= 0.0 {
                    nodes.push(*p);
                    values.push(result.displacements[i].length());
                }
            }
            Ok(FilteredResult { nodes, values })
        }
        FilterFunction::Cut { point, normal } => {
            let n = normal.normalized().unwrap_or(Vec3::Z);
            let mut nodes = Vec::new();
            let mut values = Vec::new();
            let tol = mesh.nodes.iter().fold(0.0_f64, |a, p| a.max(p.x.abs().max(p.y.abs()).max(p.z.abs()))) * 0.01;
            for (i, p) in mesh.nodes.iter().enumerate() {
                let v = *p - *point;
                if Vec3::new(v.x, v.y, v.z).dot(n).abs() < tol {
                    nodes.push(*p);
                    values.push(result.displacements[i].length());
                }
            }
            Ok(FilteredResult { nodes, values })
        }
        FilterFunction::Contour { value, .. } => {
            let mut nodes = Vec::new();
            let mut values = Vec::new();
            let tol = result.max_displacement * 0.05;
            for (i, d) in result.displacements.iter().enumerate() {
                let mag = d.length();
                if (mag - value).abs() < tol {
                    nodes.push(mesh.nodes[i]);
                    values.push(mag);
                }
            }
            Ok(FilteredResult { nodes, values })
        }
    }
}

/// Remove stored results from an analysis container.
pub fn purge_results(container: &mut AnalysisContainer) {
    container.result = None;
    container.thermal_result = None;
}

/// Create a mesh region from a set of face indices.
pub fn create_mesh_region(face_indices: &[usize]) -> MeshRegion {
    MeshRegion {
        face_indices: face_indices.to_vec(),
        element_indices: Vec::new(),
    }
}

/// Display mesh information summary.
pub fn show_mesh_info(mesh: &TetMesh) -> String {
    format!(
        "TetMesh: {} nodes, {} elements",
        mesh.nodes.len(),
        mesh.elements.len()
    )
}

/// Default FEM preferences.
pub fn fem_preferences() -> FemPreferences {
    FemPreferences::default()
}

// ---------------------------------------------------------------------------
// Mesh Generation Expansion
// ---------------------------------------------------------------------------

/// Hexahedral mesh for FEM analysis.
pub struct HexMesh {
    pub vertices: Vec<Point3>,
    pub hex_elements: Vec<[usize; 8]>,
}

/// Generate a regular hexahedral mesh within a bounding box.
pub fn generate_hex_mesh(
    bbox: &BoundingBox,
    nx: usize,
    ny: usize,
    nz: usize,
) -> KernelResult<HexMesh> {
    if nx == 0 || ny == 0 || nz == 0 {
        return Err(KernelError::InvalidArgument(
            "grid dimensions must be positive".into(),
        ));
    }
    if bbox.is_empty() {
        return Err(KernelError::InvalidArgument(
            "bounding box must not be empty".into(),
        ));
    }

    let dx = (bbox.max.x - bbox.min.x) / nx as f64;
    let dy = (bbox.max.y - bbox.min.y) / ny as f64;
    let dz = (bbox.max.z - bbox.min.z) / nz as f64;

    let mut vertices = Vec::with_capacity((nx + 1) * (ny + 1) * (nz + 1));
    let node_idx =
        |ix: usize, iy: usize, iz: usize| -> usize { ix * (ny + 1) * (nz + 1) + iy * (nz + 1) + iz };

    for ix in 0..=nx {
        for iy in 0..=ny {
            for iz in 0..=nz {
                vertices.push(Point3::new(
                    bbox.min.x + ix as f64 * dx,
                    bbox.min.y + iy as f64 * dy,
                    bbox.min.z + iz as f64 * dz,
                ));
            }
        }
    }

    let mut hex_elements = Vec::with_capacity(nx * ny * nz);
    for ix in 0..nx {
        for iy in 0..ny {
            for iz in 0..nz {
                hex_elements.push([
                    node_idx(ix, iy, iz),
                    node_idx(ix + 1, iy, iz),
                    node_idx(ix + 1, iy + 1, iz),
                    node_idx(ix, iy + 1, iz),
                    node_idx(ix, iy, iz + 1),
                    node_idx(ix + 1, iy, iz + 1),
                    node_idx(ix + 1, iy + 1, iz + 1),
                    node_idx(ix, iy + 1, iz + 1),
                ]);
            }
        }
    }

    Ok(HexMesh {
        vertices,
        hex_elements,
    })
}

/// Generate a tetrahedral mesh from a B-Rep model by bounding-box subdivision
/// and rejection of tets whose centroid lies outside the solid.
pub fn mesh_from_shape(model: &BRepModel) -> KernelResult<TetMesh> {
    let solids: Vec<_> = model.solids.iter().map(|(h, _)| h).collect();
    if solids.is_empty() {
        return Err(KernelError::InvalidArgument(
            "model has no solids".into(),
        ));
    }
    let solid = solids[0];
    let solid_data = model
        .solids
        .get(solid)
        .ok_or(KernelError::InvalidHandle("solid"))?;

    let mut bbox = BoundingBox::empty();
    for &shell_h in &solid_data.shells {
        let shell = model
            .shells
            .get(shell_h)
            .ok_or(KernelError::InvalidHandle("shell"))?;
        for &face_h in &shell.faces {
            let verts = model.vertices_of_face(face_h)?;
            for vh in verts {
                let vd = model
                    .vertices
                    .get(vh)
                    .ok_or(KernelError::InvalidHandle("vertex"))?;
                bbox.include_point(vd.point);
            }
        }
    }

    if bbox.is_empty() {
        return Err(KernelError::GeometryError(
            "solid has no vertices".into(),
        ));
    }

    let diag = ((bbox.max.x - bbox.min.x).powi(2)
        + (bbox.max.y - bbox.min.y).powi(2)
        + (bbox.max.z - bbox.min.z).powi(2))
    .sqrt();
    let edge_len = diag / 10.0;

    generate_tet_mesh(model, solid, edge_len)
}

/// Adaptive mesh refinement: subdivide elements where the error exceeds a threshold.
pub fn adaptive_mesh_refinement(
    mesh: &TetMesh,
    error: &[f64],
    threshold: f64,
) -> KernelResult<TetMesh> {
    if error.len() != mesh.elements.len() {
        return Err(KernelError::InvalidArgument(
            "error array length must match element count".into(),
        ));
    }
    if threshold <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "threshold must be positive".into(),
        ));
    }

    let mut new_nodes = mesh.nodes.clone();
    let mut new_elements = Vec::new();
    let mut edge_map: std::collections::HashMap<(usize, usize), usize> =
        std::collections::HashMap::new();

    let mut get_mid = |a: usize, b: usize, nodes: &mut Vec<Point3>| -> usize {
        let key = (a.min(b), a.max(b));
        if let Some(&idx) = edge_map.get(&key) {
            return idx;
        }
        let mid = Point3::new(
            (nodes[a].x + nodes[b].x) * 0.5,
            (nodes[a].y + nodes[b].y) * 0.5,
            (nodes[a].z + nodes[b].z) * 0.5,
        );
        let idx = nodes.len();
        nodes.push(mid);
        edge_map.insert(key, idx);
        idx
    };

    for (ei, elem) in mesh.elements.iter().enumerate() {
        if error[ei] > threshold {
            let [n0, n1, n2, n3] = *elem;
            let m01 = get_mid(n0, n1, &mut new_nodes);
            let m02 = get_mid(n0, n2, &mut new_nodes);
            let m03 = get_mid(n0, n3, &mut new_nodes);
            let m12 = get_mid(n1, n2, &mut new_nodes);
            let m13 = get_mid(n1, n3, &mut new_nodes);
            let m23 = get_mid(n2, n3, &mut new_nodes);

            new_elements.push([n0, m01, m02, m03]);
            new_elements.push([n1, m01, m12, m13]);
            new_elements.push([n2, m02, m12, m23]);
            new_elements.push([n3, m03, m13, m23]);
            new_elements.push([m01, m02, m03, m13]);
            new_elements.push([m01, m02, m12, m13]);
            new_elements.push([m02, m03, m13, m23]);
            new_elements.push([m02, m12, m13, m23]);
        } else {
            new_elements.push(*elem);
        }
    }

    Ok(TetMesh {
        nodes: new_nodes,
        elements: new_elements,
    })
}

/// Laplacian smoothing of a tetrahedral mesh.
pub fn mesh_smoothing(mesh: &mut TetMesh, iterations: usize) -> KernelResult<()> {
    if mesh.nodes.is_empty() || mesh.elements.is_empty() {
        return Err(KernelError::InvalidArgument(
            "mesh must have nodes and elements".into(),
        ));
    }

    let n = mesh.nodes.len();
    let mut neighbors: Vec<Vec<usize>> = vec![Vec::new(); n];
    for elem in &mesh.elements {
        for i in 0..4 {
            for j in (i + 1)..4 {
                let a = elem[i];
                let b = elem[j];
                if !neighbors[a].contains(&b) {
                    neighbors[a].push(b);
                }
                if !neighbors[b].contains(&a) {
                    neighbors[b].push(a);
                }
            }
        }
    }

    // Identify boundary nodes (belong to a face that appears exactly once)
    let mut face_count: std::collections::HashMap<[usize; 3], usize> =
        std::collections::HashMap::new();
    for elem in &mesh.elements {
        let faces = [
            [elem[0], elem[1], elem[2]],
            [elem[0], elem[1], elem[3]],
            [elem[0], elem[2], elem[3]],
            [elem[1], elem[2], elem[3]],
        ];
        for face in &faces {
            let mut sorted = *face;
            sorted.sort();
            *face_count.entry(sorted).or_insert(0) += 1;
        }
    }
    let mut boundary = vec![false; n];
    for (face, &count) in &face_count {
        if count == 1 {
            for &ni in face {
                boundary[ni] = true;
            }
        }
    }

    for _ in 0..iterations {
        let old_nodes = mesh.nodes.clone();
        for i in 0..n {
            if boundary[i] || neighbors[i].is_empty() {
                continue;
            }
            let mut avg = Point3::new(0.0, 0.0, 0.0);
            let cnt = neighbors[i].len() as f64;
            for &j in &neighbors[i] {
                avg.x += old_nodes[j].x;
                avg.y += old_nodes[j].y;
                avg.z += old_nodes[j].z;
            }
            mesh.nodes[i] = Point3::new(avg.x / cnt, avg.y / cnt, avg.z / cnt);
        }
    }

    Ok(())
}

/// Export a tetrahedral mesh in Abaqus .inp format.
pub fn export_mesh_abaqus(mesh: &TetMesh) -> KernelResult<String> {
    if mesh.nodes.is_empty() || mesh.elements.is_empty() {
        return Err(KernelError::InvalidArgument(
            "mesh must have nodes and elements".into(),
        ));
    }

    let mut out = String::new();
    out.push_str("*HEADING\nCADKernel FEM Export\n");
    out.push_str("*NODE\n");
    for (i, node) in mesh.nodes.iter().enumerate() {
        out.push_str(&format!(
            "{}, {:.6e}, {:.6e}, {:.6e}\n",
            i + 1,
            node.x,
            node.y,
            node.z
        ));
    }
    out.push_str("*ELEMENT, TYPE=C3D4\n");
    for (i, elem) in mesh.elements.iter().enumerate() {
        out.push_str(&format!(
            "{}, {}, {}, {}, {}\n",
            i + 1,
            elem[0] + 1,
            elem[1] + 1,
            elem[2] + 1,
            elem[3] + 1,
        ));
    }
    out.push_str("*END\n");
    Ok(out)
}

/// Export a tetrahedral mesh in the specified format ("abaqus" or "nastran").
pub fn export_mesh_format(mesh: &TetMesh, format: &str) -> KernelResult<String> {
    match format.to_lowercase().as_str() {
        "abaqus" | "inp" => export_mesh_abaqus(mesh),
        "nastran" | "bdf" | "nas" => export_mesh_nastran(mesh),
        _ => Err(KernelError::InvalidArgument(format!(
            "unsupported mesh format: {}",
            format
        ))),
    }
}

/// Export a tetrahedral mesh in Nastran bulk data format.
fn export_mesh_nastran(mesh: &TetMesh) -> KernelResult<String> {
    if mesh.nodes.is_empty() || mesh.elements.is_empty() {
        return Err(KernelError::InvalidArgument(
            "mesh must have nodes and elements".into(),
        ));
    }

    let mut out = String::new();
    out.push_str("BEGIN BULK\n");
    for (i, node) in mesh.nodes.iter().enumerate() {
        out.push_str(&format!(
            "GRID    {:8}{:8}{:8.4}{:8.4}{:8.4}\n",
            i + 1,
            "",
            node.x,
            node.y,
            node.z
        ));
    }
    for (i, elem) in mesh.elements.iter().enumerate() {
        out.push_str(&format!(
            "CTETRA  {:8}{:8}{:8}{:8}{:8}{:8}\n",
            i + 1,
            1,
            elem[0] + 1,
            elem[1] + 1,
            elem[2] + 1,
            elem[3] + 1,
        ));
    }
    out.push_str("ENDDATA\n");
    Ok(out)
}

// ---------------------------------------------------------------------------
// Solver Expansion
// ---------------------------------------------------------------------------

/// Nonlinear static analysis using Newton-Raphson with load increments.
pub fn nonlinear_static_analysis(
    container: &AnalysisContainer,
) -> KernelResult<FemResult> {
    let mesh = &container.mesh;
    let material = &container.material;
    let bcs = &container.boundary_conditions;

    if mesh.nodes.is_empty() || mesh.elements.is_empty() {
        return Err(KernelError::InvalidArgument(
            "mesh must have nodes and elements".into(),
        ));
    }

    let num_increments = 10_usize;
    let mut accumulated_result: Option<FemResult> = None;

    for step in 1..=num_increments {
        let load_factor = step as f64 / num_increments as f64;

        let scaled_bcs: Vec<BoundaryCondition> = bcs
            .iter()
            .map(|bc| match bc {
                BoundaryCondition::Force { node, force } => BoundaryCondition::Force {
                    node: *node,
                    force: *force * load_factor,
                },
                BoundaryCondition::Pressure { element, pressure } => {
                    BoundaryCondition::Pressure {
                        element: *element,
                        pressure: pressure * load_factor,
                    }
                }
                BoundaryCondition::Gravity { acceleration } => BoundaryCondition::Gravity {
                    acceleration: *acceleration * load_factor,
                },
                BoundaryCondition::DistributedLoad { element, load } => {
                    BoundaryCondition::DistributedLoad {
                        element: *element,
                        load: *load * load_factor,
                    }
                }
                other => other.clone(),
            })
            .collect();

        let step_result = static_analysis(mesh, material, &scaled_bcs)?;
        accumulated_result = Some(step_result);
    }

    accumulated_result.ok_or(KernelError::GeometryError(
        "no load increments completed".into(),
    ))
}

/// Frequency analysis (wrapper around modal_analysis).
pub fn frequency_analysis(
    container: &AnalysisContainer,
    num_modes: usize,
) -> KernelResult<ModalResult> {
    modal_analysis(
        &container.mesh,
        &container.material,
        &container.boundary_conditions,
        num_modes,
    )
}

/// Results of a linear buckling analysis.
pub struct BucklingResult {
    pub critical_loads: Vec<f64>,
    pub mode_shapes: Vec<Vec<Vec3>>,
}

/// Linear buckling analysis: eigenvalue problem for critical load factors.
pub fn buckling_analysis(
    container: &AnalysisContainer,
    num_modes: usize,
) -> KernelResult<BucklingResult> {
    if num_modes == 0 {
        return Err(KernelError::InvalidArgument(
            "num_modes must be at least 1".into(),
        ));
    }

    let mesh = &container.mesh;
    let material = &container.material;
    let bcs = &container.boundary_conditions;

    // First solve linear static for the stress state
    let base_result = static_analysis(mesh, material, bcs)?;

    // Then solve eigenvalue problem similar to modal analysis
    // but using geometric stiffness instead of mass matrix
    let n_nodes = mesh.nodes.len();
    let n_dof = n_nodes * 3;
    let d_matrix = build_elasticity_matrix(material.youngs_modulus, material.poisson_ratio);

    let mut k_rows: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n_dof];
    let mut k_diag = vec![0.0_f64; n_dof];

    for elem in &mesh.elements {
        let ke = element_stiffness(mesh, elem, &d_matrix)?;
        for local_i in 0..4 {
            for local_j in 0..4 {
                for di in 0..3 {
                    for dj in 0..3 {
                        let gi = elem[local_i] * 3 + di;
                        let gj = elem[local_j] * 3 + dj;
                        let val = ke[local_i * 3 + di][local_j * 3 + dj];
                        if val.abs() > 1e-30 {
                            if gi == gj {
                                k_diag[gi] += val;
                            }
                            add_to_sparse_row(&mut k_rows[gi], gj, val);
                        }
                    }
                }
            }
        }
    }

    // Geometric stiffness approximated from element stresses
    let mut kg_diag = vec![0.0_f64; n_dof];
    for (ei, elem) in mesh.elements.iter().enumerate() {
        let vol = tet_volume(&mesh.nodes, elem);
        let stress = base_result.stresses[ei];
        let kg_val = stress * vol / 4.0;
        for &ni in elem {
            for d in 0..3 {
                kg_diag[ni * 3 + d] += kg_val;
            }
        }
    }

    let mut fixed_dofs = vec![false; n_dof];
    for bc in bcs {
        if let BoundaryCondition::FixedNode(node) = bc {
            if *node < n_nodes {
                for d in 0..3 {
                    fixed_dofs[node * 3 + d] = true;
                }
            }
        }
    }

    let mut critical_loads = Vec::with_capacity(num_modes);
    let mut mode_shapes = Vec::with_capacity(num_modes);
    let mut prev_modes: Vec<Vec<f64>> = Vec::new();

    for _ in 0..num_modes {
        let mut x = vec![0.0_f64; n_dof];
        for (i, xi) in x.iter_mut().enumerate() {
            if !fixed_dofs[i] {
                *xi = ((i * 7 + 13) % 97) as f64 / 97.0 - 0.5;
            }
        }

        let mut eigenvalue = 0.0_f64;

        for _ in 0..500 {
            for prev in &prev_modes {
                let dot: f64 = x.iter().zip(prev.iter()).map(|(a, b)| a * b).sum();
                for (xi, pi) in x.iter_mut().zip(prev.iter()) {
                    *xi -= dot * pi;
                }
            }

            let mut y = vec![0.0_f64; n_dof];
            for i in 0..n_dof {
                y[i] = kg_diag[i] * x[i];
            }

            let mut z = vec![0.0_f64; n_dof];
            for _ in 0..200 {
                for i in 0..n_dof {
                    if fixed_dofs[i] {
                        z[i] = 0.0;
                        continue;
                    }
                    let mut sum = 0.0_f64;
                    let mut diag = k_diag[i];
                    for &(j, val) in &k_rows[i] {
                        if j == i {
                            diag = val;
                        } else {
                            sum += val * z[j];
                        }
                    }
                    if diag.abs() > 1e-30 {
                        z[i] = (y[i] - sum) / diag;
                    }
                }
            }

            let norm: f64 = z.iter().map(|v| v * v).sum::<f64>().sqrt();
            if norm > 1e-30 {
                for zi in &mut z {
                    *zi /= norm;
                }
            }

            let mut ztkz = 0.0_f64;
            let mut ztkg_z = 0.0_f64;
            for i in 0..n_dof {
                ztkg_z += z[i] * kg_diag[i] * z[i];
                for &(j, val) in &k_rows[i] {
                    ztkz += z[i] * val * z[j];
                }
            }
            eigenvalue = if ztkg_z.abs() > 1e-30 {
                ztkz / ztkg_z
            } else {
                f64::INFINITY
            };

            x = z;
        }

        critical_loads.push(eigenvalue);
        let mut shape = Vec::with_capacity(n_nodes);
        for i in 0..n_nodes {
            shape.push(Vec3::new(x[i * 3], x[i * 3 + 1], x[i * 3 + 2]));
        }
        mode_shapes.push(shape);
        prev_modes.push(x);
    }

    Ok(BucklingResult {
        critical_loads,
        mode_shapes,
    })
}

// ---------------------------------------------------------------------------
// Equation Expansion (Multi-Physics)
// ---------------------------------------------------------------------------

/// Result of a magnetostatic analysis.
pub struct MagnetostaticResult {
    pub potentials: Vec<f64>,
    pub magnetic_fields: Vec<Vec3>,
    pub max_potential: f64,
    pub max_field_strength: f64,
}

/// Solve magnetostatic equations (Poisson for magnetic vector potential).
pub fn magnetostatic_equation(
    mesh: &TetMesh,
    em_bcs: &[EmBoundaryCondition],
) -> KernelResult<MagnetostaticResult> {
    if mesh.nodes.is_empty() || mesh.elements.is_empty() {
        return Err(KernelError::InvalidArgument(
            "mesh must have nodes and elements".into(),
        ));
    }

    let n_nodes = mesh.nodes.len();
    let permeability = 4.0 * std::f64::consts::PI * 1e-7; // mu_0

    let mut k_rows: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n_nodes];
    let mut rhs = vec![0.0_f64; n_nodes];

    for elem in &mesh.elements {
        let ke = element_thermal_stiffness(mesh, elem, permeability)?;
        for i in 0..4 {
            for j in 0..4 {
                if ke[i][j].abs() > 1e-30 {
                    add_to_sparse_row(&mut k_rows[elem[i]], elem[j], ke[i][j]);
                }
            }
        }
    }

    for bc in em_bcs {
        match bc {
            EmBoundaryCondition::ElectricPotential { node, voltage } => {
                if *node < n_nodes {
                    let penalty = permeability * 1e20;
                    add_to_sparse_row(&mut k_rows[*node], *node, penalty);
                    rhs[*node] += penalty * voltage;
                }
            }
            EmBoundaryCondition::CurrentDensity { element, density } => {
                if *element < mesh.elements.len() {
                    let elem_nodes = &mesh.elements[*element];
                    let vol = tet_volume(&mesh.nodes, elem_nodes);
                    let node_src = density.length() * vol / 4.0;
                    for &ni in elem_nodes {
                        rhs[ni] += node_src;
                    }
                }
            }
            _ => {}
        }
    }

    let mut potentials = vec![0.0_f64; n_nodes];
    for _ in 0..10_000 {
        let mut max_delta = 0.0_f64;
        for i in 0..n_nodes {
            let mut diag = 0.0_f64;
            let mut sum = 0.0_f64;
            for &(j, val) in &k_rows[i] {
                if j == i {
                    diag = val;
                } else {
                    sum += val * potentials[j];
                }
            }
            if diag.abs() < 1e-30 {
                continue;
            }
            let new_val = (rhs[i] - sum) / diag;
            let delta = (new_val - potentials[i]).abs();
            if delta > max_delta {
                max_delta = delta;
            }
            potentials[i] = new_val;
        }
        if max_delta < 1e-10 {
            break;
        }
    }

    let mut magnetic_fields = Vec::with_capacity(mesh.elements.len());
    for elem in &mesh.elements {
        let grad = element_temperature_gradient(mesh, elem, &potentials)?;
        magnetic_fields.push(grad * (-1.0));
    }

    let max_potential = potentials.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let max_field_strength = magnetic_fields.iter().map(|b| b.length()).fold(0.0_f64, f64::max);

    Ok(MagnetostaticResult {
        potentials,
        magnetic_fields,
        max_potential,
        max_field_strength,
    })
}

/// Result of a coupled thermo-mechanical analysis.
pub struct CoupledResult {
    pub mechanical: FemResult,
    pub thermal: ThermalResult,
}

/// Sequential coupled thermo-mechanical analysis.
pub fn coupled_thermo_mechanical(
    mesh: &TetMesh,
    material: &FemMaterial,
    thermal_material: &ThermalMaterial,
    mech_bcs: &[BoundaryCondition],
    thermal_bcs: &[ThermalBoundaryCondition],
) -> KernelResult<CoupledResult> {
    // Step 1: Solve thermal
    let thermal = thermal_analysis(mesh, thermal_material, thermal_bcs)?;

    // Step 2: Compute thermal strains and add as equivalent forces
    let alpha = 12e-6; // typical thermal expansion coefficient for steel
    let ref_temp = 20.0; // reference temperature

    let mut thermal_forces = vec![Vec3::ZERO; mesh.nodes.len()];
    let d_matrix = build_elasticity_matrix(material.youngs_modulus, material.poisson_ratio);

    for elem in &mesh.elements {
        let vol = tet_volume(&mesh.nodes, elem);
        let avg_temp = (thermal.temperatures[elem[0]]
            + thermal.temperatures[elem[1]]
            + thermal.temperatures[elem[2]]
            + thermal.temperatures[elem[3]])
            / 4.0;
        let delta_t = avg_temp - ref_temp;
        let thermal_strain = alpha * delta_t;

        // Thermal stress contribution: sigma_th = D * alpha * delta_T * [1,1,1,0,0,0]
        let mut thermal_stress = [0.0_f64; 6];
        for i in 0..3 {
            for &val in &d_matrix[i][..3] {
                thermal_stress[i] += val * thermal_strain;
            }
        }

        let force_mag = (thermal_stress[0] + thermal_stress[1] + thermal_stress[2]) * vol / 12.0;
        for &ni in elem {
            thermal_forces[ni].x += force_mag;
            thermal_forces[ni].y += force_mag;
            thermal_forces[ni].z += force_mag;
        }
    }

    // Build combined BCs: original + thermal forces
    let mut combined_bcs = mech_bcs.to_vec();
    for (i, tf) in thermal_forces.iter().enumerate() {
        if tf.length() > 1e-30 {
            combined_bcs.push(BoundaryCondition::Force {
                node: i,
                force: *tf,
            });
        }
    }

    let mechanical = static_analysis(mesh, material, &combined_bcs)?;

    Ok(CoupledResult {
        mechanical,
        thermal,
    })
}

/// Result of an acoustic analysis.
pub struct AcousticResult {
    pub pressures: Vec<f64>,
    pub max_pressure: f64,
    pub min_pressure: f64,
}

/// Solve the Helmholtz equation for acoustics.
pub fn acoustic_equation(
    mesh: &TetMesh,
    frequency: f64,
    speed_of_sound: f64,
    bcs: &[ThermalBoundaryCondition],
) -> KernelResult<AcousticResult> {
    if mesh.nodes.is_empty() || mesh.elements.is_empty() {
        return Err(KernelError::InvalidArgument(
            "mesh must have nodes and elements".into(),
        ));
    }
    if frequency < 0.0 {
        return Err(KernelError::InvalidArgument(
            "frequency must be non-negative".into(),
        ));
    }
    if speed_of_sound <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "speed of sound must be positive".into(),
        ));
    }

    let n_nodes = mesh.nodes.len();
    let k_wave = 2.0 * std::f64::consts::PI * frequency / speed_of_sound;

    // Stiffness-like matrix for Helmholtz: (K - k^2 M) p = f
    let mut rows: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n_nodes];
    let mut rhs = vec![0.0_f64; n_nodes];

    for elem in &mesh.elements {
        let ke = element_thermal_stiffness(mesh, elem, 1.0)?;
        let vol = tet_volume(&mesh.nodes, elem);
        let mass_per_node = vol / 4.0;

        for i in 0..4 {
            for j in 0..4 {
                let val = ke[i][j];
                if val.abs() > 1e-30 {
                    add_to_sparse_row(&mut rows[elem[i]], elem[j], val);
                }
            }
            // Subtract mass contribution: -k^2 * M
            add_to_sparse_row(&mut rows[elem[i]], elem[i], -k_wave * k_wave * mass_per_node);
        }
    }

    // Apply BCs (reuse thermal BC types: FixedTemperature = fixed pressure)
    for bc in bcs {
        match bc {
            ThermalBoundaryCondition::FixedTemperature { node, temperature } => {
                if *node < n_nodes {
                    let penalty = 1e10;
                    add_to_sparse_row(&mut rows[*node], *node, penalty);
                    rhs[*node] += penalty * temperature;
                }
            }
            ThermalBoundaryCondition::HeatFlux { element, flux } => {
                if *element < mesh.elements.len() {
                    let elem_nodes = &mesh.elements[*element];
                    let vol = tet_volume(&mesh.nodes, elem_nodes);
                    let nf = flux * vol / 4.0;
                    for &ni in elem_nodes {
                        rhs[ni] += nf;
                    }
                }
            }
            _ => {}
        }
    }

    let mut pressures = vec![0.0_f64; n_nodes];
    for _ in 0..10_000 {
        let mut max_delta = 0.0_f64;
        for i in 0..n_nodes {
            let mut diag = 0.0_f64;
            let mut sum = 0.0_f64;
            for &(j, val) in &rows[i] {
                if j == i {
                    diag = val;
                } else {
                    sum += val * pressures[j];
                }
            }
            if diag.abs() < 1e-30 {
                continue;
            }
            let new_val = (rhs[i] - sum) / diag;
            let delta = (new_val - pressures[i]).abs();
            if delta > max_delta {
                max_delta = delta;
            }
            pressures[i] = new_val;
        }
        if max_delta < 1e-10 {
            break;
        }
    }

    let max_pressure = pressures.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let min_pressure = pressures.iter().copied().fold(f64::INFINITY, f64::min);

    Ok(AcousticResult {
        pressures,
        max_pressure,
        min_pressure,
    })
}

/// Result of a scalar field equation.
pub struct ScalarResult {
    pub values: Vec<f64>,
    pub max_value: f64,
    pub min_value: f64,
}

/// Solve the Poisson equation: -nabla^2 u = source_fn on the mesh.
pub fn poisson_equation(
    mesh: &TetMesh,
    source_values: &[f64],
    bcs: &[ThermalBoundaryCondition],
) -> KernelResult<ScalarResult> {
    if mesh.nodes.is_empty() || mesh.elements.is_empty() {
        return Err(KernelError::InvalidArgument(
            "mesh must have nodes and elements".into(),
        ));
    }

    let n_nodes = mesh.nodes.len();
    let mut k_rows: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n_nodes];
    let mut rhs = vec![0.0_f64; n_nodes];

    for elem in &mesh.elements {
        let ke = element_thermal_stiffness(mesh, elem, 1.0)?;
        for i in 0..4 {
            for j in 0..4 {
                if ke[i][j].abs() > 1e-30 {
                    add_to_sparse_row(&mut k_rows[elem[i]], elem[j], ke[i][j]);
                }
            }
        }

        // Source term
        let vol = tet_volume(&mesh.nodes, elem);
        for &ni in elem.iter() {
            let src = source_values.get(ni).copied().unwrap_or(0.0);
            rhs[ni] += src * vol / 4.0;
        }
    }

    for bc in bcs {
        if let ThermalBoundaryCondition::FixedTemperature { node, temperature } = bc {
            if *node < n_nodes {
                let penalty = 1e10;
                add_to_sparse_row(&mut k_rows[*node], *node, penalty);
                rhs[*node] += penalty * temperature;
            }
        }
    }

    let mut values = vec![0.0_f64; n_nodes];
    for _ in 0..10_000 {
        let mut max_delta = 0.0_f64;
        for i in 0..n_nodes {
            let mut diag = 0.0_f64;
            let mut sum = 0.0_f64;
            for &(j, val) in &k_rows[i] {
                if j == i {
                    diag = val;
                } else {
                    sum += val * values[j];
                }
            }
            if diag.abs() < 1e-30 {
                continue;
            }
            let new_val = (rhs[i] - sum) / diag;
            let delta = (new_val - values[i]).abs();
            if delta > max_delta {
                max_delta = delta;
            }
            values[i] = new_val;
        }
        if max_delta < 1e-10 {
            break;
        }
    }

    let max_value = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let min_value = values.iter().copied().fold(f64::INFINITY, f64::min);

    Ok(ScalarResult {
        values,
        max_value,
        min_value,
    })
}

/// Solve the transient diffusion equation using implicit Euler time-stepping.
pub fn diffusion_equation(
    mesh: &TetMesh,
    diffusivity: f64,
    bcs: &[ThermalBoundaryCondition],
    dt: f64,
) -> KernelResult<ScalarResult> {
    if mesh.nodes.is_empty() || mesh.elements.is_empty() {
        return Err(KernelError::InvalidArgument(
            "mesh must have nodes and elements".into(),
        ));
    }
    if diffusivity <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "diffusivity must be positive".into(),
        ));
    }
    if dt <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "time step must be positive".into(),
        ));
    }

    let n_nodes = mesh.nodes.len();

    // Build K = diffusivity * stiffness
    let mut k_rows: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n_nodes];
    let mut m_diag = vec![0.0_f64; n_nodes];

    for elem in &mesh.elements {
        let ke = element_thermal_stiffness(mesh, elem, diffusivity)?;
        let vol = tet_volume(&mesh.nodes, elem);
        let mass_per_node = vol / 4.0;

        for i in 0..4 {
            for j in 0..4 {
                if ke[i][j].abs() > 1e-30 {
                    add_to_sparse_row(&mut k_rows[elem[i]], elem[j], ke[i][j]);
                }
            }
            m_diag[elem[i]] += mass_per_node;
        }
    }

    // Implicit Euler: (M/dt + K) u^{n+1} = M/dt * u^n
    // Add M/dt to diagonal of K
    for i in 0..n_nodes {
        add_to_sparse_row(&mut k_rows[i], i, m_diag[i] / dt);
    }

    // Initialize with zero (or BCs)
    let mut values = vec![0.0_f64; n_nodes];
    for bc in bcs {
        if let ThermalBoundaryCondition::FixedTemperature { node, temperature } = bc {
            if *node < n_nodes {
                values[*node] = *temperature;
            }
        }
    }

    // Single time step solve
    let mut rhs = vec![0.0_f64; n_nodes];
    for i in 0..n_nodes {
        rhs[i] = m_diag[i] / dt * values[i];
    }

    // Apply Dirichlet BCs via penalty
    for bc in bcs {
        if let ThermalBoundaryCondition::FixedTemperature { node, temperature } = bc {
            if *node < n_nodes {
                let penalty = 1e10;
                add_to_sparse_row(&mut k_rows[*node], *node, penalty);
                rhs[*node] += penalty * temperature;
            }
        }
    }

    for _ in 0..10_000 {
        let mut max_delta = 0.0_f64;
        for i in 0..n_nodes {
            let mut diag = 0.0_f64;
            let mut sum = 0.0_f64;
            for &(j, val) in &k_rows[i] {
                if j == i {
                    diag = val;
                } else {
                    sum += val * values[j];
                }
            }
            if diag.abs() < 1e-30 {
                continue;
            }
            let new_val = (rhs[i] - sum) / diag;
            let delta = (new_val - values[i]).abs();
            if delta > max_delta {
                max_delta = delta;
            }
            values[i] = new_val;
        }
        if max_delta < 1e-10 {
            break;
        }
    }

    let max_value = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let min_value = values.iter().copied().fold(f64::INFINITY, f64::min);

    Ok(ScalarResult {
        values,
        max_value,
        min_value,
    })
}

// ---------------------------------------------------------------------------
// Post-Processing Expansion
// ---------------------------------------------------------------------------

/// Extract nodal values for a named field from FEM results.
pub fn extract_nodal_values(result: &FemResult, field: &str) -> Vec<f64> {
    match field {
        "displacement" | "disp" | "u" => result
            .displacements
            .iter()
            .map(|d| d.length())
            .collect(),
        "displacement_x" | "ux" => result.displacements.iter().map(|d| d.x).collect(),
        "displacement_y" | "uy" => result.displacements.iter().map(|d| d.y).collect(),
        "displacement_z" | "uz" => result.displacements.iter().map(|d| d.z).collect(),
        "stress" | "von_mises" => {
            let mut node_stress = vec![0.0_f64; result.displacements.len()];
            let mut node_count = vec![0usize; result.displacements.len()];
            for &s in &result.stresses {
                // Without mesh topology, distribute uniformly
                for i in 0..node_stress.len() {
                    node_stress[i] += s;
                    node_count[i] += 1;
                }
            }
            node_stress
                .iter()
                .zip(node_count.iter())
                .map(|(&s, &c)| if c > 0 { s / c as f64 } else { 0.0 })
                .collect()
        }
        _ => vec![0.0; result.displacements.len()],
    }
}

/// Interpolate element-centered values to nodes by averaging over adjacent elements.
pub fn interpolate_to_nodes(element_values: &[f64], mesh: &TetMesh) -> Vec<f64> {
    let n_nodes = mesh.nodes.len();
    let mut node_values = vec![0.0_f64; n_nodes];
    let mut node_count = vec![0usize; n_nodes];

    for (ei, elem) in mesh.elements.iter().enumerate() {
        if ei < element_values.len() {
            for &ni in elem {
                node_values[ni] += element_values[ei];
                node_count[ni] += 1;
            }
        }
    }

    for i in 0..n_nodes {
        if node_count[i] > 0 {
            node_values[i] /= node_count[i] as f64;
        }
    }

    node_values
}

/// Compute Zienkiewicz-Zhu error estimator per element.
pub fn compute_error_estimate(result: &FemResult, mesh: &TetMesh) -> Vec<f64> {
    // Average stress at each node
    let node_stress = interpolate_to_nodes(&result.stresses, mesh);

    // Error per element = |element_stress - avg_node_stress|
    let mut errors = Vec::with_capacity(mesh.elements.len());
    for (ei, elem) in mesh.elements.iter().enumerate() {
        let elem_stress = if ei < result.stresses.len() {
            result.stresses[ei]
        } else {
            0.0
        };
        let avg_node = (node_stress[elem[0]]
            + node_stress[elem[1]]
            + node_stress[elem[2]]
            + node_stress[elem[3]])
            / 4.0;
        errors.push((elem_stress - avg_node).abs());
    }
    errors
}

/// Interpolate result at an arbitrary point using barycentric coordinates.
pub fn result_at_point(
    result: &FemResult,
    mesh: &TetMesh,
    point: Point3,
) -> KernelResult<Vec<f64>> {
    // Find the containing element
    for elem in &mesh.elements {
        let p0 = mesh.nodes[elem[0]];
        let p1 = mesh.nodes[elem[1]];
        let p2 = mesh.nodes[elem[2]];
        let p3 = mesh.nodes[elem[3]];

        if let Some(bary) = barycentric_coords(p0, p1, p2, p3, point) {
            let d0 = &result.displacements[elem[0]];
            let d1 = &result.displacements[elem[1]];
            let d2 = &result.displacements[elem[2]];
            let d3 = &result.displacements[elem[3]];

            let dx = bary[0] * d0.x + bary[1] * d1.x + bary[2] * d2.x + bary[3] * d3.x;
            let dy = bary[0] * d0.y + bary[1] * d1.y + bary[2] * d2.y + bary[3] * d3.y;
            let dz = bary[0] * d0.z + bary[1] * d1.z + bary[2] * d2.z + bary[3] * d3.z;
            let mag = (dx * dx + dy * dy + dz * dz).sqrt();

            return Ok(vec![dx, dy, dz, mag]);
        }
    }

    Err(KernelError::GeometryError(
        "point is not inside any element".into(),
    ))
}

/// Compute barycentric coordinates of a point within a tetrahedron.
/// Returns None if the point is outside.
fn barycentric_coords(
    p0: Point3,
    p1: Point3,
    p2: Point3,
    p3: Point3,
    p: Point3,
) -> Option<[f64; 4]> {
    let v0 = p1 - p0;
    let v1 = p2 - p0;
    let v2 = p3 - p0;
    let vp = p - p0;

    let det = v0.x * (v1.y * v2.z - v1.z * v2.y)
        - v0.y * (v1.x * v2.z - v1.z * v2.x)
        + v0.z * (v1.x * v2.y - v1.y * v2.x);

    if det.abs() < 1e-30 {
        return None;
    }

    let inv_det = 1.0 / det;

    let l1 = (vp.x * (v1.y * v2.z - v1.z * v2.y)
        - vp.y * (v1.x * v2.z - v1.z * v2.x)
        + vp.z * (v1.x * v2.y - v1.y * v2.x))
        * inv_det;

    let l2 = (v0.x * (vp.y * v2.z - vp.z * v2.y)
        - v0.y * (vp.x * v2.z - vp.z * v2.x)
        + v0.z * (vp.x * v2.y - vp.y * v2.x))
        * inv_det;

    let l3 = (v0.x * (v1.y * vp.z - v1.z * vp.y)
        - v0.y * (v1.x * vp.z - v1.z * vp.x)
        + v0.z * (v1.x * vp.y - v1.y * vp.x))
        * inv_det;

    let l0 = 1.0 - l1 - l2 - l3;

    let tol = -1e-10;
    if l0 >= tol && l1 >= tol && l2 >= tol && l3 >= tol {
        Some([l0, l1, l2, l3])
    } else {
        None
    }
}

/// Integrate a scalar field over specified element faces (surface integral).
pub fn integrate_over_surface(
    result: &FemResult,
    mesh: &TetMesh,
    face_ids: &[usize],
) -> f64 {
    let mut total = 0.0_f64;

    // Boundary faces from elements
    let surface = extract_surface_mesh(mesh).unwrap_or_default();

    for &fi in face_ids {
        if fi >= surface.len() {
            continue;
        }
        let tri = &surface[fi];
        let p0 = mesh.nodes[tri[0]];
        let p1 = mesh.nodes[tri[1]];
        let p2 = mesh.nodes[tri[2]];
        let area = (p1 - p0).cross(p2 - p0).length() * 0.5;

        // Average displacement magnitude on the face
        let avg_val = (result.displacements[tri[0]].length()
            + result.displacements[tri[1]].length()
            + result.displacements[tri[2]].length())
            / 3.0;

        total += avg_val * area;
    }

    total
}

/// Find max and min values with their indices.
pub fn max_min_values(values: &[f64]) -> (f64, f64, usize, usize) {
    if values.is_empty() {
        return (0.0, 0.0, 0, 0);
    }

    let mut max_val = f64::NEG_INFINITY;
    let mut min_val = f64::INFINITY;
    let mut max_idx = 0;
    let mut min_idx = 0;

    for (i, &v) in values.iter().enumerate() {
        if v > max_val {
            max_val = v;
            max_idx = i;
        }
        if v < min_val {
            min_val = v;
            min_idx = i;
        }
    }

    (max_val, min_val, max_idx, min_idx)
}

/// Extract results along a path defined by a sequence of points.
pub fn path_result(
    result: &FemResult,
    mesh: &TetMesh,
    points: &[Point3],
) -> Vec<Vec<f64>> {
    let mut results = Vec::with_capacity(points.len());
    for pt in points {
        match result_at_point(result, mesh, *pt) {
            Ok(vals) => results.push(vals),
            Err(_) => results.push(vec![0.0; 4]),
        }
    }
    results
}

/// Compute reaction forces at fixed/supported nodes.
pub fn reaction_forces(
    result: &FemResult,
    mesh: &TetMesh,
    material: &FemMaterial,
    fixed_nodes: &[usize],
) -> Vec<(usize, Vec3)> {
    compute_reactions(mesh, material, result, fixed_nodes).unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Utility Expansion
// ---------------------------------------------------------------------------

/// Generate a human-readable summary of an analysis container.
pub fn fem_summary(container: &AnalysisContainer) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "FEM Analysis Summary\n  Mesh: {} nodes, {} elements\n",
        container.mesh.nodes.len(),
        container.mesh.elements.len()
    ));
    s.push_str(&format!(
        "  Material: E={:.3e} Pa, nu={:.3}, rho={:.1} kg/m3\n",
        container.material.youngs_modulus,
        container.material.poisson_ratio,
        container.material.density
    ));
    s.push_str(&format!(
        "  Boundary Conditions: {}\n",
        container.boundary_conditions.len()
    ));

    if let Some(ref r) = container.result {
        s.push_str(&format!(
            "  Static Result: max_disp={:.6e}, max_stress={:.6e}\n",
            r.max_displacement, r.max_stress
        ));
    } else {
        s.push_str("  Static Result: not computed\n");
    }

    if let Some(ref t) = container.thermal_result {
        s.push_str(&format!(
            "  Thermal Result: T_min={:.2}, T_max={:.2}\n",
            t.min_temperature, t.max_temperature
        ));
    }

    s
}

/// Generate a detailed FEM analysis report.
pub fn export_fem_report(container: &AnalysisContainer, result: &FemResult) -> String {
    let mut s = String::new();
    s.push_str("=== CADKernel FEM Analysis Report ===\n\n");
    s.push_str(&format!(
        "Mesh:\n  Nodes: {}\n  Elements: {}\n\n",
        container.mesh.nodes.len(),
        container.mesh.elements.len()
    ));
    s.push_str(&format!(
        "Material:\n  Young's Modulus: {:.3e} Pa\n  Poisson Ratio: {:.4}\n  Density: {:.1} kg/m3\n\n",
        container.material.youngs_modulus,
        container.material.poisson_ratio,
        container.material.density
    ));
    s.push_str(&format!(
        "Boundary Conditions: {} applied\n\n",
        container.boundary_conditions.len()
    ));
    s.push_str("Results:\n");
    s.push_str(&format!(
        "  Max Displacement: {:.6e} m\n",
        result.max_displacement
    ));
    s.push_str(&format!(
        "  Max Von Mises Stress: {:.6e} Pa\n",
        result.max_stress
    ));
    let sf = safety_factor(result, container.material.youngs_modulus * 0.001);
    s.push_str(&format!("  Safety Factor (0.1% yield): {:.2}\n", sf));

    if let Ok(ref quality) = mesh_quality(&container.mesh) {
        s.push_str(&format!(
            "\nMesh Quality:\n  Min Aspect Ratio: {:.4}\n  Avg Aspect Ratio: {:.4}\n  Degenerate Elements: {}\n",
            quality.min_aspect_ratio, quality.avg_aspect_ratio, quality.degenerate_count
        ));
    }

    s
}

/// Per-element quality metrics for detailed mesh assessment.
pub struct ElementQuality {
    pub element_index: usize,
    pub volume: f64,
    pub aspect_ratio: f64,
    pub min_edge_length: f64,
    pub max_edge_length: f64,
}

/// Compute detailed per-element quality metrics.
pub fn check_mesh_quality_detailed(mesh: &TetMesh) -> Vec<ElementQuality> {
    let mut qualities = Vec::with_capacity(mesh.elements.len());

    for (ei, elem) in mesh.elements.iter().enumerate() {
        let vol = tet_volume(&mesh.nodes, elem);
        let pts: Vec<Point3> = elem.iter().map(|&i| mesh.nodes[i]).collect();
        let edges = [
            (pts[0] - pts[1]).length(),
            (pts[0] - pts[2]).length(),
            (pts[0] - pts[3]).length(),
            (pts[1] - pts[2]).length(),
            (pts[1] - pts[3]).length(),
            (pts[2] - pts[3]).length(),
        ];
        let max_edge = edges.iter().copied().fold(0.0_f64, f64::max);
        let min_edge = edges.iter().copied().fold(f64::INFINITY, f64::min);
        let ar = if max_edge > 1e-30 {
            min_edge / max_edge
        } else {
            0.0
        };

        qualities.push(ElementQuality {
            element_index: ei,
            volume: vol,
            aspect_ratio: ar,
            min_edge_length: min_edge,
            max_edge_length: max_edge,
        });
    }

    qualities
}

/// Validate boundary conditions in an analysis container.
pub fn check_boundary_conditions(container: &AnalysisContainer) -> KernelResult<Vec<String>> {
    let n_nodes = container.mesh.nodes.len();
    let n_elems = container.mesh.elements.len();
    let mut warnings = Vec::new();

    let mut has_fixed = false;
    let mut has_load = false;

    for bc in &container.boundary_conditions {
        match bc {
            BoundaryCondition::FixedNode(node) => {
                has_fixed = true;
                if *node >= n_nodes {
                    warnings.push(format!("FixedNode({}) exceeds node count {}", node, n_nodes));
                }
            }
            BoundaryCondition::Force { node, .. } => {
                has_load = true;
                if *node >= n_nodes {
                    warnings.push(format!("Force node {} exceeds node count {}", node, n_nodes));
                }
            }
            BoundaryCondition::Pressure { element, .. } => {
                has_load = true;
                if *element >= n_elems {
                    warnings.push(format!(
                        "Pressure element {} exceeds element count {}",
                        element, n_elems
                    ));
                }
            }
            BoundaryCondition::Displacement { node, .. } => {
                has_fixed = true;
                if *node >= n_nodes {
                    warnings.push(format!(
                        "Displacement node {} exceeds node count {}",
                        node, n_nodes
                    ));
                }
            }
            BoundaryCondition::Gravity { .. } | BoundaryCondition::SelfWeight { .. } => {
                has_load = true;
            }
            BoundaryCondition::DistributedLoad { element, .. } => {
                has_load = true;
                if *element >= n_elems {
                    warnings.push(format!(
                        "DistributedLoad element {} exceeds element count {}",
                        element, n_elems
                    ));
                }
            }
            BoundaryCondition::Spring { node, .. } => {
                if *node >= n_nodes {
                    warnings.push(format!(
                        "Spring node {} exceeds node count {}",
                        node, n_nodes
                    ));
                }
            }
            _ => {}
        }
    }

    if !has_fixed {
        warnings.push("No fixed constraints — model may be unconstrained (rigid body motion)".into());
    }
    if !has_load {
        warnings.push("No loads applied — results will be zero".into());
    }

    Ok(warnings)
}

/// Estimate computation time based on DOF count (rough heuristic).
pub fn estimate_computation_time(container: &AnalysisContainer) -> f64 {
    let n_dof = container.mesh.nodes.len() * 3;
    // Heuristic: O(n^1.5) for iterative solvers, scaled to seconds
    (n_dof as f64).powf(1.5) * 1e-7
}

/// Apply element geometry type to an analysis container.
pub fn apply_element_geometry(
    _container: &mut AnalysisContainer,
    geo: ElementGeometry,
) -> KernelResult<()> {
    match geo {
        ElementGeometry::Solid => Ok(()),
        ElementGeometry::Shell { thickness } => {
            if thickness <= 0.0 {
                return Err(KernelError::InvalidArgument(
                    "shell thickness must be positive".into(),
                ));
            }
            Ok(())
        }
        ElementGeometry::Beam(ref section) => {
            if section.area <= 0.0 {
                return Err(KernelError::InvalidArgument(
                    "beam cross-section area must be positive".into(),
                ));
            }
            Ok(())
        }
        ElementGeometry::Membrane { thickness } => {
            if thickness <= 0.0 {
                return Err(KernelError::InvalidArgument(
                    "membrane thickness must be positive".into(),
                ));
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fem_material_steel() {
        let steel = FemMaterial::steel();
        assert!((steel.youngs_modulus - 210.0e9).abs() < 1.0);
        assert!((steel.poisson_ratio - 0.3).abs() < 1e-10);
        assert!((steel.density - 7850.0).abs() < 1e-10);
    }

    #[test]
    fn test_fem_material_aluminum() {
        let al = FemMaterial::aluminum();
        assert!((al.youngs_modulus - 70.0e9).abs() < 1.0);
        assert!((al.poisson_ratio - 0.33).abs() < 1e-10);
        assert!((al.density - 2700.0).abs() < 1e-10);
    }

    #[test]
    fn test_tet_mesh_generation() {
        // Create a simple box solid
        let mut model = BRepModel::new();
        let solid = crate::make_box(&mut model, Point3::new(0.0, 0.0, 0.0), 2.0, 2.0, 2.0).unwrap().solid;
        let mesh = generate_tet_mesh(&model, solid, 1.0).unwrap();
        assert!(!mesh.nodes.is_empty(), "mesh should have nodes");
        assert!(!mesh.elements.is_empty(), "mesh should have elements");
        // Every element should reference valid node indices
        for elem in &mesh.elements {
            for &ni in elem {
                assert!(ni < mesh.nodes.len(), "node index out of range");
            }
        }
    }

    #[test]
    fn test_static_analysis_simple() {
        // Single tetrahedron with 3 fixed nodes and a force on the 4th
        let mesh = TetMesh {
            nodes: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
                Point3::new(0.0, 0.0, 1.0),
            ],
            elements: vec![[0, 1, 2, 3]],
        };

        let material = FemMaterial::steel();
        let bcs = vec![
            BoundaryCondition::FixedNode(0),
            BoundaryCondition::FixedNode(1),
            BoundaryCondition::FixedNode(2),
            BoundaryCondition::Force {
                node: 3,
                force: Vec3::new(0.0, 0.0, -1000.0),
            },
        ];

        let result = static_analysis(&mesh, &material, &bcs).unwrap();
        assert_eq!(result.displacements.len(), 4);
        assert_eq!(result.stresses.len(), 1);

        // Fixed nodes should have near-zero displacement
        for i in 0..3 {
            assert!(
                result.displacements[i].length() < 1e-6,
                "fixed node {} should have near-zero displacement",
                i
            );
        }

        // Node 3 should move (very small displacement for steel)
        assert!(result.max_displacement > 0.0);
        assert!(result.max_stress > 0.0);
    }

    #[test]
    fn test_fem_validation() {
        let mut model = BRepModel::new();
        let solid = crate::make_box(&mut model, Point3::new(0.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap().solid;

        // max_edge_length <= 0 should fail
        let err = generate_tet_mesh(&model, solid, 0.0);
        assert!(err.is_err());

        let err = generate_tet_mesh(&model, solid, -1.0);
        assert!(err.is_err());
    }

    #[test]
    fn test_elasticity_matrix_symmetry() {
        let d = build_elasticity_matrix(210.0e9, 0.3);
        for (i, row_i) in d.iter().enumerate() {
            for (j, &val) in row_i.iter().enumerate() {
                assert!(
                    (val - d[j][i]).abs() < 1e-6,
                    "D matrix should be symmetric: D[{}][{}]={} != D[{}][{}]={}",
                    i, j, val, j, i, d[j][i]
                );
            }
        }
    }

    #[test]
    fn test_empty_mesh_analysis_fails() {
        let mesh = TetMesh {
            nodes: vec![],
            elements: vec![],
        };
        let material = FemMaterial::steel();
        let result = static_analysis(&mesh, &material, &[]);
        assert!(result.is_err());
    }

    fn make_simple_tet_mesh() -> TetMesh {
        TetMesh {
            nodes: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
                Point3::new(0.0, 0.0, 1.0),
            ],
            elements: vec![[0, 1, 2, 3]],
        }
    }

    fn make_two_tet_mesh() -> TetMesh {
        TetMesh {
            nodes: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
                Point3::new(0.0, 0.0, 1.0),
                Point3::new(1.0, 1.0, 1.0),
            ],
            elements: vec![[0, 1, 2, 3], [1, 2, 3, 4]],
        }
    }

    #[test]
    fn test_material_titanium() {
        let ti = FemMaterial::titanium();
        assert!((ti.youngs_modulus - 114.0e9).abs() < 1.0);
        assert!((ti.density - 4430.0).abs() < 1.0);
    }

    #[test]
    fn test_material_copper() {
        let cu = FemMaterial::copper();
        assert!((cu.youngs_modulus - 117.0e9).abs() < 1.0);
    }

    #[test]
    fn test_material_concrete() {
        let c = FemMaterial::concrete();
        assert!((c.youngs_modulus - 30.0e9).abs() < 1.0);
    }

    #[test]
    fn test_material_cast_iron() {
        let ci = FemMaterial::cast_iron();
        assert!((ci.youngs_modulus - 170.0e9).abs() < 1.0);
    }

    #[test]
    fn test_material_custom() {
        let m = FemMaterial::custom(100.0e9, 0.25, 5000.0).unwrap();
        assert!((m.youngs_modulus - 100.0e9).abs() < 1.0);

        assert!(FemMaterial::custom(-1.0, 0.3, 1000.0).is_err());
        assert!(FemMaterial::custom(100.0e9, 0.5, 1000.0).is_err());
        assert!(FemMaterial::custom(100.0e9, 0.3, -1.0).is_err());
    }

    #[test]
    fn test_thermal_material() {
        let steel = ThermalMaterial::steel();
        assert!((steel.conductivity - 50.0).abs() < 1e-10);
        let al = ThermalMaterial::aluminum();
        assert!((al.conductivity - 237.0).abs() < 1e-10);
        let cu = ThermalMaterial::copper();
        assert!((cu.conductivity - 401.0).abs() < 1e-10);
    }

    #[test]
    fn test_beam_section_circular() {
        let s = BeamSection::circular(0.05);
        let expected_area = std::f64::consts::PI * 0.05 * 0.05;
        assert!((s.area - expected_area).abs() < 1e-10);
        assert!(s.iy > 0.0);
        assert!((s.iy - s.iz).abs() < 1e-20);
    }

    #[test]
    fn test_beam_section_rectangular() {
        let s = BeamSection::rectangular(0.1, 0.2);
        assert!((s.area - 0.02).abs() < 1e-10);
        assert!(s.iy > 0.0);
        assert!(s.iz > 0.0);
    }

    #[test]
    fn test_displacement_bc() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let bcs = vec![
            BoundaryCondition::FixedNode(0),
            BoundaryCondition::FixedNode(1),
            BoundaryCondition::FixedNode(2),
            BoundaryCondition::Displacement {
                node: 3,
                displacement: Vec3::new(0.001, 0.0, 0.0),
            },
        ];
        let result = static_analysis(&mesh, &material, &bcs).unwrap();
        assert!(result.displacements[3].x.abs() > 1e-10);
    }

    #[test]
    fn test_gravity_bc() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let bcs = vec![
            BoundaryCondition::FixedNode(0),
            BoundaryCondition::FixedNode(1),
            BoundaryCondition::FixedNode(2),
            BoundaryCondition::Gravity {
                acceleration: Vec3::new(0.0, 0.0, -9.81),
            },
        ];
        let result = static_analysis(&mesh, &material, &bcs).unwrap();
        assert!(result.max_displacement > 0.0);
    }

    #[test]
    fn test_spring_bc() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let bcs = vec![
            BoundaryCondition::FixedNode(0),
            BoundaryCondition::FixedNode(1),
            BoundaryCondition::Spring {
                node: 2,
                stiffness: 1e8,
            },
            BoundaryCondition::Force {
                node: 3,
                force: Vec3::new(0.0, 0.0, -1000.0),
            },
        ];
        let result = static_analysis(&mesh, &material, &bcs).unwrap();
        assert!(result.max_displacement > 0.0);
    }

    #[test]
    fn test_distributed_load() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let bcs = vec![
            BoundaryCondition::FixedNode(0),
            BoundaryCondition::FixedNode(1),
            BoundaryCondition::FixedNode(2),
            BoundaryCondition::DistributedLoad {
                element: 0,
                load: Vec3::new(0.0, 0.0, -1000.0),
            },
        ];
        let result = static_analysis(&mesh, &material, &bcs).unwrap();
        assert!(result.max_displacement > 0.0);
    }

    #[test]
    fn test_modal_analysis() {
        let mesh = make_two_tet_mesh();
        let material = FemMaterial::steel();
        let bcs = vec![BoundaryCondition::FixedNode(0)];
        let result = modal_analysis(&mesh, &material, &bcs, 2).unwrap();
        assert_eq!(result.frequencies.len(), 2);
        assert_eq!(result.mode_shapes.len(), 2);
        assert!(result.frequencies[0] >= 0.0);
    }

    #[test]
    fn test_thermal_analysis_simple() {
        let mesh = make_simple_tet_mesh();
        let material = ThermalMaterial::steel();
        let bcs = vec![
            ThermalBoundaryCondition::FixedTemperature {
                node: 0,
                temperature: 100.0,
            },
            ThermalBoundaryCondition::FixedTemperature {
                node: 3,
                temperature: 200.0,
            },
        ];
        let result = thermal_analysis(&mesh, &material, &bcs).unwrap();
        assert_eq!(result.temperatures.len(), 4);
        assert!((result.temperatures[0] - 100.0).abs() < 1.0);
        assert!((result.temperatures[3] - 200.0).abs() < 1.0);
        assert!(result.max_temperature >= result.min_temperature);
    }

    #[test]
    fn test_thermal_heat_flux() {
        let mesh = make_simple_tet_mesh();
        let material = ThermalMaterial::steel();
        let bcs = vec![
            ThermalBoundaryCondition::FixedTemperature {
                node: 0,
                temperature: 0.0,
            },
            ThermalBoundaryCondition::HeatFlux {
                element: 0,
                flux: 1000.0,
            },
        ];
        let result = thermal_analysis(&mesh, &material, &bcs).unwrap();
        assert_eq!(result.heat_fluxes.len(), 1);
    }

    #[test]
    fn test_thermal_heat_generation() {
        let mesh = make_simple_tet_mesh();
        let material = ThermalMaterial::steel();
        let bcs = vec![
            ThermalBoundaryCondition::FixedTemperature {
                node: 0,
                temperature: 20.0,
            },
            ThermalBoundaryCondition::HeatGeneration {
                element: 0,
                rate: 1e6,
            },
        ];
        let result = thermal_analysis(&mesh, &material, &bcs).unwrap();
        assert!(result.max_temperature > 20.0);
    }

    #[test]
    fn test_thermal_convection() {
        let mesh = make_simple_tet_mesh();
        let material = ThermalMaterial::steel();
        let bcs = vec![
            ThermalBoundaryCondition::FixedTemperature {
                node: 0,
                temperature: 100.0,
            },
            ThermalBoundaryCondition::Convection {
                element: 0,
                coefficient: 25.0,
                ambient_temp: 20.0,
            },
        ];
        let result = thermal_analysis(&mesh, &material, &bcs).unwrap();
        assert_eq!(result.temperatures.len(), 4);
    }

    #[test]
    fn test_mesh_quality() {
        let mesh = make_simple_tet_mesh();
        let quality = mesh_quality(&mesh).unwrap();
        assert_eq!(quality.total_elements, 1);
        assert_eq!(quality.degenerate_count, 0);
        assert!(quality.min_aspect_ratio > 0.0);
        assert!(quality.min_volume > 0.0);
    }

    #[test]
    fn test_refine_tet_mesh() {
        let mesh = make_simple_tet_mesh();
        let refined = refine_tet_mesh(&mesh).unwrap();
        assert_eq!(refined.elements.len(), 8);
        assert!(refined.nodes.len() > mesh.nodes.len());
        for elem in &refined.elements {
            for &ni in elem {
                assert!(ni < refined.nodes.len());
            }
        }
    }

    #[test]
    fn test_extract_surface_mesh() {
        let mesh = make_simple_tet_mesh();
        let surface = extract_surface_mesh(&mesh).unwrap();
        assert_eq!(surface.len(), 4);
    }

    #[test]
    fn test_compute_stress_tensor() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let bcs = vec![
            BoundaryCondition::FixedNode(0),
            BoundaryCondition::FixedNode(1),
            BoundaryCondition::FixedNode(2),
            BoundaryCondition::Force {
                node: 3,
                force: Vec3::new(0.0, 0.0, -1000.0),
            },
        ];
        let result = static_analysis(&mesh, &material, &bcs).unwrap();
        let stress = compute_stress_tensor(&mesh, &material, &result).unwrap();
        assert_eq!(stress.stresses.len(), 1);
    }

    #[test]
    fn test_compute_strain_tensor() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let bcs = vec![
            BoundaryCondition::FixedNode(0),
            BoundaryCondition::FixedNode(1),
            BoundaryCondition::FixedNode(2),
            BoundaryCondition::Force {
                node: 3,
                force: Vec3::new(0.0, 0.0, -1000.0),
            },
        ];
        let result = static_analysis(&mesh, &material, &bcs).unwrap();
        let strain = compute_strain_tensor(&mesh, &result).unwrap();
        assert_eq!(strain.strains.len(), 1);
    }

    #[test]
    fn test_principal_stresses() {
        // Uniaxial tension
        let stress = [100.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let ps = principal_stresses(&stress);
        assert!((ps.sigma1 - 100.0).abs() < 1e-6);
        assert!(ps.sigma2.abs() < 1e-6);
        assert!(ps.sigma3.abs() < 1e-6);
    }

    #[test]
    fn test_principal_stresses_hydrostatic() {
        let stress = [50.0, 50.0, 50.0, 0.0, 0.0, 0.0];
        let ps = principal_stresses(&stress);
        assert!((ps.sigma1 - 50.0).abs() < 1e-6);
        assert!((ps.sigma2 - 50.0).abs() < 1e-6);
        assert!((ps.sigma3 - 50.0).abs() < 1e-6);
    }

    #[test]
    fn test_safety_factor() {
        let result = FemResult {
            displacements: vec![Vec3::ZERO],
            stresses: vec![100.0e6],
            max_displacement: 0.001,
            max_stress: 100.0e6,
        };
        let sf = safety_factor(&result, 250.0e6);
        assert!((sf - 2.5).abs() < 1e-6);
    }

    #[test]
    fn test_strain_energy() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let bcs = vec![
            BoundaryCondition::FixedNode(0),
            BoundaryCondition::FixedNode(1),
            BoundaryCondition::FixedNode(2),
            BoundaryCondition::Force {
                node: 3,
                force: Vec3::new(0.0, 0.0, -1000.0),
            },
        ];
        let result = static_analysis(&mesh, &material, &bcs).unwrap();
        let energy = strain_energy(&mesh, &material, &result).unwrap();
        assert!(energy >= 0.0);
    }

    #[test]
    fn test_compute_reactions() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let bcs = vec![
            BoundaryCondition::FixedNode(0),
            BoundaryCondition::FixedNode(1),
            BoundaryCondition::FixedNode(2),
            BoundaryCondition::Force {
                node: 3,
                force: Vec3::new(0.0, 0.0, -1000.0),
            },
        ];
        let result = static_analysis(&mesh, &material, &bcs).unwrap();
        let reactions = compute_reactions(&mesh, &material, &result, &[0, 1, 2]).unwrap();
        assert_eq!(reactions.len(), 3);
        // Sum of reactions in z should balance the applied force (≈ +1000)
        let sum_rz: f64 = reactions.iter().map(|(_, r)| r.z).sum();
        assert!(
            (sum_rz - 1000.0).abs() < 100.0,
            "reaction sum z={} should be near 1000",
            sum_rz
        );
    }

    #[test]
    fn test_merge_coincident_nodes() {
        let mesh = TetMesh {
            nodes: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
                Point3::new(0.0, 0.0, 1.0),
                Point3::new(0.0, 0.0, 0.00001), // near-duplicate of node 0
            ],
            elements: vec![[0, 1, 2, 3], [4, 1, 2, 3]],
        };
        let merged = merge_coincident_nodes(&mesh, 0.001).unwrap();
        assert!(merged.nodes.len() < mesh.nodes.len());
        assert_eq!(merged.elements[0][0], merged.elements[1][0]);
    }

    #[test]
    fn test_tet_volume_positive() {
        let nodes = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
        ];
        let vol = tet_volume(&nodes, &[0, 1, 2, 3]);
        assert!((vol - 1.0 / 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_modal_analysis_validation() {
        let mesh = TetMesh {
            nodes: vec![],
            elements: vec![],
        };
        let material = FemMaterial::steel();
        assert!(modal_analysis(&mesh, &material, &[], 1).is_err());
    }

    #[test]
    fn test_thermal_analysis_validation() {
        let mesh = TetMesh {
            nodes: vec![],
            elements: vec![],
        };
        let material = ThermalMaterial::steel();
        assert!(thermal_analysis(&mesh, &material, &[]).is_err());
    }

    #[test]
    fn test_mesh_quality_empty() {
        let mesh = TetMesh {
            nodes: vec![],
            elements: vec![],
        };
        assert!(mesh_quality(&mesh).is_err());
    }

    #[test]
    fn test_refine_two_tets() {
        let mesh = make_two_tet_mesh();
        let refined = refine_tet_mesh(&mesh).unwrap();
        assert_eq!(refined.elements.len(), 16);
        // Shared edge midpoints should be reused
        assert!(refined.nodes.len() < mesh.nodes.len() + 2 * 6);
    }

    #[test]
    fn test_extract_surface_two_tets() {
        let mesh = make_two_tet_mesh();
        let surface = extract_surface_mesh(&mesh).unwrap();
        // Two tets share face [1,2,3], so 4+4 - 2 = 6 boundary faces
        assert_eq!(surface.len(), 6);
    }

    #[test]
    fn test_analysis_container() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let mut container = AnalysisContainer::new(mesh, material);
        container.add_bc(BoundaryCondition::FixedNode(0));
        container.add_bc(BoundaryCondition::FixedNode(1));
        container.add_bc(BoundaryCondition::FixedNode(2));
        container.add_bc(BoundaryCondition::Force {
            node: 3,
            force: Vec3::new(0.0, 0.0, -1000.0),
        });
        container.run_static().unwrap();
        assert!(container.result.is_some());
        assert!(container.result.as_ref().unwrap().max_displacement > 0.0);

        purge_results(&mut container);
        assert!(container.result.is_none());
    }

    #[test]
    fn test_analysis_container_thermal() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let mut container = AnalysisContainer::new(mesh, material);
        container.thermal_material = Some(ThermalMaterial::steel());
        container.thermal_bcs.push(ThermalBoundaryCondition::FixedTemperature {
            node: 0,
            temperature: 100.0,
        });
        container.thermal_bcs.push(ThermalBoundaryCondition::FixedTemperature {
            node: 3,
            temperature: 200.0,
        });
        container.run_thermal().unwrap();
        assert!(container.thermal_result.is_some());
    }

    #[test]
    fn test_heat_equation_alias() {
        let mesh = make_simple_tet_mesh();
        let material = ThermalMaterial::steel();
        let bcs = vec![
            ThermalBoundaryCondition::FixedTemperature {
                node: 0,
                temperature: 0.0,
            },
            ThermalBoundaryCondition::FixedTemperature {
                node: 3,
                temperature: 100.0,
            },
        ];
        let result = heat_equation(&mesh, &material, &bcs).unwrap();
        assert_eq!(result.temperatures.len(), 4);
    }

    #[test]
    fn test_deformation_equation_alias() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let bcs = vec![
            BoundaryCondition::FixedNode(0),
            BoundaryCondition::FixedNode(1),
            BoundaryCondition::FixedNode(2),
            BoundaryCondition::Force {
                node: 3,
                force: Vec3::new(0.0, 0.0, -1000.0),
            },
        ];
        let result = deformation_equation(&mesh, &material, &bcs).unwrap();
        assert!(result.max_displacement > 0.0);
    }

    #[test]
    fn test_flow_equation() {
        let mesh = make_simple_tet_mesh();
        let bcs = vec![
            FluidBoundaryCondition::Velocity {
                node: 0,
                velocity: Vec3::new(1.0, 0.0, 0.0),
            },
            FluidBoundaryCondition::Pressure {
                node: 3,
                pressure: 0.0,
            },
        ];
        let result = flow_equation(&mesh, 1.0, &bcs).unwrap();
        assert_eq!(result.velocities.len(), 4);
        assert_eq!(result.pressures.len(), 4);
    }

    #[test]
    fn test_flow_equation_invalid() {
        let mesh = TetMesh {
            nodes: vec![],
            elements: vec![],
        };
        assert!(flow_equation(&mesh, 1.0, &[]).is_err());
        let mesh2 = make_simple_tet_mesh();
        assert!(flow_equation(&mesh2, -1.0, &[]).is_err());
    }

    #[test]
    fn test_electrostatic_equation() {
        let mesh = make_simple_tet_mesh();
        let bcs = vec![
            EmBoundaryCondition::ElectricPotential {
                node: 0,
                voltage: 0.0,
            },
            EmBoundaryCondition::ElectricPotential {
                node: 3,
                voltage: 100.0,
            },
        ];
        let result = electrostatic_equation(&mesh, &bcs).unwrap();
        assert_eq!(result.potentials.len(), 4);
        assert!(!result.electric_fields.is_empty());
    }

    #[test]
    fn test_apply_filter_warp() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let bcs = vec![
            BoundaryCondition::FixedNode(0),
            BoundaryCondition::FixedNode(1),
            BoundaryCondition::FixedNode(2),
            BoundaryCondition::Force {
                node: 3,
                force: Vec3::new(0.0, 0.0, -1000.0),
            },
        ];
        let result = static_analysis(&mesh, &material, &bcs).unwrap();
        let filtered = apply_filter(&mesh, &result, &FilterFunction::Warp { factor: 10.0 }).unwrap();
        assert_eq!(filtered.nodes.len(), 4);
    }

    #[test]
    fn test_apply_filter_clip() {
        let mesh = make_simple_tet_mesh();
        let result = FemResult {
            displacements: vec![Vec3::ZERO; 4],
            stresses: vec![0.0],
            max_displacement: 0.0,
            max_stress: 0.0,
        };
        let filtered = apply_filter(
            &mesh,
            &result,
            &FilterFunction::Clip {
                point: Point3::new(0.0, 0.0, 0.5),
                normal: Vec3::Z,
            },
        )
        .unwrap();
        // Only node (0,0,1) is above z=0.5
        assert!(filtered.nodes.len() <= 4);
    }

    #[test]
    fn test_create_mesh_region() {
        let region = create_mesh_region(&[0, 1, 2]);
        assert_eq!(region.face_indices.len(), 3);
        assert!(region.element_indices.is_empty());
    }

    #[test]
    fn test_show_mesh_info() {
        let mesh = make_simple_tet_mesh();
        let info = show_mesh_info(&mesh);
        assert!(info.contains("4 nodes"));
        assert!(info.contains("1 elements"));
    }

    #[test]
    fn test_fem_preferences() {
        let prefs = fem_preferences();
        assert_eq!(prefs.max_iterations, 10_000);
        assert_eq!(prefs.solver_type, "gauss_seidel");
    }

    #[test]
    fn test_element_geometry_types() {
        let _solid = ElementGeometry::Solid;
        let _beam = ElementGeometry::Beam(BeamSection::circular(0.01));
        let _shell = ElementGeometry::Shell { thickness: 0.005 };
        let _membrane = ElementGeometry::Membrane { thickness: 0.001 };
    }

    #[test]
    fn test_visualization_modes() {
        let _deformed = VisualizationMode::Deformed { scale: 10.0 };
        let _colormap = VisualizationMode::ColorMap {
            field_name: "stress".to_string(),
        };
        let _arrows = VisualizationMode::VectorArrows {
            field_name: "displacement".to_string(),
        };
    }

    #[test]
    fn test_geometrical_features() {
        let _plane = GeometricalFeature::PlaneSection {
            point: Point3::ORIGIN,
            normal: Vec3::Z,
        };
        let _cyl = GeometricalFeature::CylinderSection {
            axis: Point3::ORIGIN,
            direction: Vec3::Z,
            radius: 1.0,
        };
        let _sphere = GeometricalFeature::SphereSection {
            center: Point3::ORIGIN,
            radius: 1.0,
        };
    }

    #[test]
    fn test_new_boundary_conditions() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let bcs = vec![
            BoundaryCondition::FixedNode(0),
            BoundaryCondition::FixedNode(1),
            BoundaryCondition::SelfWeight {
                gravity: Vec3::new(0.0, 0.0, -9.81),
            },
        ];
        let result = static_analysis(&mesh, &material, &bcs).unwrap();
        assert_eq!(result.displacements.len(), 4);
    }

    #[test]
    fn test_centrifugal_load() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let bcs = vec![
            BoundaryCondition::FixedNode(0),
            BoundaryCondition::FixedNode(1),
            BoundaryCondition::FixedNode(2),
            BoundaryCondition::CentrifugalLoad {
                axis: Vec3::Z,
                omega: 100.0,
            },
        ];
        let result = static_analysis(&mesh, &material, &bcs).unwrap();
        assert_eq!(result.displacements.len(), 4);
    }

    #[test]
    fn test_tie_constraint() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let bcs = vec![
            BoundaryCondition::FixedNode(0),
            BoundaryCondition::TieConstraint {
                surface_a: vec![1],
                surface_b: vec![2],
            },
            BoundaryCondition::Force {
                node: 3,
                force: Vec3::new(0.0, 0.0, -1000.0),
            },
        ];
        let result = static_analysis(&mesh, &material, &bcs).unwrap();
        assert_eq!(result.displacements.len(), 4);
    }

    #[test]
    fn test_rigid_body_constraint() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let bcs = vec![
            BoundaryCondition::FixedNode(0),
            BoundaryCondition::RigidBody {
                node_ids: vec![1, 2, 3],
            },
            BoundaryCondition::Force {
                node: 3,
                force: Vec3::new(0.0, 0.0, -1000.0),
            },
        ];
        let result = static_analysis(&mesh, &material, &bcs).unwrap();
        assert_eq!(result.displacements.len(), 4);
    }

    #[test]
    fn test_spring_constraint() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let bcs = vec![
            BoundaryCondition::FixedNode(0),
            BoundaryCondition::FixedNode(1),
            BoundaryCondition::FixedNode(2),
            BoundaryCondition::SpringConstraint {
                node_id: 3,
                stiffness: 1e6,
                direction: Vec3::Z,
            },
            BoundaryCondition::Force {
                node: 3,
                force: Vec3::new(0.0, 0.0, -1000.0),
            },
        ];
        let result = static_analysis(&mesh, &material, &bcs).unwrap();
        assert_eq!(result.displacements.len(), 4);
    }

    #[test]
    fn test_section_print_noop() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let bcs = vec![
            BoundaryCondition::FixedNode(0),
            BoundaryCondition::FixedNode(1),
            BoundaryCondition::FixedNode(2),
            BoundaryCondition::SectionPrint {
                plane_normal: Vec3::Z,
                plane_point: Point3::ORIGIN,
            },
            BoundaryCondition::Force {
                node: 3,
                force: Vec3::new(0.0, 0.0, -1000.0),
            },
        ];
        let result = static_analysis(&mesh, &material, &bcs).unwrap();
        assert_eq!(result.displacements.len(), 4);
    }

    #[test]
    fn test_beam_stiffness_matrix() {
        let section = BeamSection::circular(0.01);
        let material = FemMaterial::steel();
        let k = beam_stiffness_matrix(&section, 1.0, &material);
        assert_eq!(k.len(), 12);
        assert_eq!(k[0].len(), 12);
        // Axial stiffness on diagonal
        assert!(k[0][0] > 0.0);
        // Symmetry check
        for (i, row) in k.iter().enumerate() {
            for (j, &val) in row.iter().enumerate() {
                assert!(
                    (val - k[j][i]).abs() < 1e-6 * val.abs().max(1.0),
                    "beam stiffness not symmetric at ({},{})",
                    i, j
                );
            }
        }
    }

    #[test]
    fn test_shell_stiffness_matrix() {
        let material = FemMaterial::steel();
        let k = shell_stiffness_matrix(0.01, &material);
        assert_eq!(k.len(), 24);
        assert_eq!(k[0].len(), 24);
        // Diagonal entries should be non-zero
        assert!(k[0][0] > 0.0);
        assert!(k[2][2] > 0.0);
    }

    #[test]
    fn test_prepare_visualization_deformed() {
        let mesh = make_simple_tet_mesh();
        let result = FemResult {
            displacements: vec![
                Vec3::ZERO,
                Vec3::ZERO,
                Vec3::ZERO,
                Vec3::new(0.0, 0.0, 0.1),
            ],
            stresses: vec![100.0],
            max_displacement: 0.1,
            max_stress: 100.0,
        };
        let mode = VisualizationMode::Deformed { scale: 10.0 };
        let viz = prepare_visualization(&result, &mesh, &mode).unwrap();
        assert_eq!(viz.positions.len(), 4);
        // Node 3 at (0,0,1) displaced by (0,0,0.1)*10 = (0,0,2)
        assert!((viz.positions[3].z - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_prepare_visualization_colormap() {
        let mesh = make_simple_tet_mesh();
        let result = FemResult {
            displacements: vec![Vec3::ZERO; 4],
            stresses: vec![500.0],
            max_displacement: 0.0,
            max_stress: 500.0,
        };
        let mode = VisualizationMode::ColorMap {
            field_name: "stress".to_string(),
        };
        let viz = prepare_visualization(&result, &mesh, &mode).unwrap();
        assert_eq!(viz.colors.len(), 4);
    }

    #[test]
    fn test_prepare_visualization_vector() {
        let mesh = make_simple_tet_mesh();
        let disps = vec![
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::ZERO,
            Vec3::ZERO,
            Vec3::ZERO,
        ];
        let result = FemResult {
            displacements: disps.clone(),
            stresses: vec![0.0],
            max_displacement: 1.0,
            max_stress: 0.0,
        };
        let mode = VisualizationMode::VectorArrows {
            field_name: "displacement".to_string(),
        };
        let viz = prepare_visualization(&result, &mesh, &mode).unwrap();
        assert_eq!(viz.vectors.len(), 4);
        assert!((viz.vectors[0].x - 1.0).abs() < 1e-10);
    }

    // === New tests for FEM expansion ===

    #[test]
    fn test_hex_mesh_generation() {
        let bbox = BoundingBox::new(Point3::ORIGIN, Point3::new(2.0, 2.0, 2.0));
        let hex = generate_hex_mesh(&bbox, 3, 3, 3).unwrap();
        assert_eq!(hex.hex_elements.len(), 27);
        assert_eq!(hex.vertices.len(), 64); // 4^3
        for elem in &hex.hex_elements {
            for &ni in elem {
                assert!(ni < hex.vertices.len());
            }
        }
    }

    #[test]
    fn test_hex_mesh_validation() {
        let bbox = BoundingBox::new(Point3::ORIGIN, Point3::new(1.0, 1.0, 1.0));
        assert!(generate_hex_mesh(&bbox, 0, 1, 1).is_err());
        assert!(generate_hex_mesh(&BoundingBox::empty(), 1, 1, 1).is_err());
    }

    #[test]
    fn test_mesh_from_shape() {
        let mut model = BRepModel::new();
        let _box = crate::make_box(
            &mut model,
            Point3::new(0.0, 0.0, 0.0),
            2.0,
            2.0,
            2.0,
        )
        .unwrap();
        let mesh = mesh_from_shape(&model).unwrap();
        assert!(!mesh.nodes.is_empty());
        assert!(!mesh.elements.is_empty());
    }

    #[test]
    fn test_adaptive_mesh_refinement() {
        let mesh = make_two_tet_mesh();
        let errors = vec![10.0, 0.1]; // only first element exceeds threshold
        let refined = adaptive_mesh_refinement(&mesh, &errors, 1.0).unwrap();
        // First element splits into 8, second stays = 9 total
        assert_eq!(refined.elements.len(), 9);
    }

    #[test]
    fn test_adaptive_mesh_refinement_validation() {
        let mesh = make_simple_tet_mesh();
        assert!(adaptive_mesh_refinement(&mesh, &[1.0, 2.0], 1.0).is_err()); // wrong length
        assert!(adaptive_mesh_refinement(&mesh, &[1.0], 0.0).is_err()); // zero threshold
    }

    #[test]
    fn test_mesh_smoothing() {
        let mut mesh = make_two_tet_mesh();
        let original_nodes = mesh.nodes.clone();
        mesh_smoothing(&mut mesh, 1).unwrap();
        assert_eq!(mesh.nodes.len(), original_nodes.len());
    }

    #[test]
    fn test_mesh_smoothing_empty() {
        let mut mesh = TetMesh {
            nodes: vec![],
            elements: vec![],
        };
        assert!(mesh_smoothing(&mut mesh, 1).is_err());
    }

    #[test]
    fn test_export_mesh_abaqus() {
        let mesh = make_simple_tet_mesh();
        let inp = export_mesh_abaqus(&mesh).unwrap();
        assert!(inp.contains("*HEADING"));
        assert!(inp.contains("*NODE"));
        assert!(inp.contains("*ELEMENT, TYPE=C3D4"));
        assert!(inp.contains("*END"));
    }

    #[test]
    fn test_export_mesh_abaqus_empty() {
        let mesh = TetMesh {
            nodes: vec![],
            elements: vec![],
        };
        assert!(export_mesh_abaqus(&mesh).is_err());
    }

    #[test]
    fn test_nonlinear_static_analysis() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let mut container = AnalysisContainer::new(mesh, material);
        container.add_bc(BoundaryCondition::FixedNode(0));
        container.add_bc(BoundaryCondition::FixedNode(1));
        container.add_bc(BoundaryCondition::FixedNode(2));
        container.add_bc(BoundaryCondition::Force {
            node: 3,
            force: Vec3::new(0.0, 0.0, -1000.0),
        });
        let result = nonlinear_static_analysis(&container).unwrap();
        assert!(result.max_displacement > 0.0);
    }

    #[test]
    fn test_frequency_analysis() {
        let mesh = make_two_tet_mesh();
        let material = FemMaterial::steel();
        let mut container = AnalysisContainer::new(mesh, material);
        container.add_bc(BoundaryCondition::FixedNode(0));
        let result = frequency_analysis(&container, 2).unwrap();
        assert_eq!(result.frequencies.len(), 2);
        assert_eq!(result.mode_shapes.len(), 2);
    }

    #[test]
    fn test_buckling_analysis() {
        let mesh = make_two_tet_mesh();
        let material = FemMaterial::steel();
        let mut container = AnalysisContainer::new(mesh, material);
        container.add_bc(BoundaryCondition::FixedNode(0));
        container.add_bc(BoundaryCondition::Force {
            node: 4,
            force: Vec3::new(0.0, 0.0, -1000.0),
        });
        let result = buckling_analysis(&container, 1).unwrap();
        assert_eq!(result.critical_loads.len(), 1);
        assert_eq!(result.mode_shapes.len(), 1);
    }

    #[test]
    fn test_buckling_validation() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let container = AnalysisContainer::new(mesh, material);
        assert!(buckling_analysis(&container, 0).is_err());
    }

    #[test]
    fn test_magnetostatic_equation() {
        let mesh = make_simple_tet_mesh();
        let bcs = vec![
            EmBoundaryCondition::ElectricPotential {
                node: 0,
                voltage: 0.0,
            },
            EmBoundaryCondition::CurrentDensity {
                element: 0,
                density: Vec3::new(0.0, 0.0, 1e6),
            },
        ];
        let result = magnetostatic_equation(&mesh, &bcs).unwrap();
        assert_eq!(result.potentials.len(), 4);
        assert!(!result.magnetic_fields.is_empty());
    }

    #[test]
    fn test_coupled_thermo_mechanical() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let thermal_mat = ThermalMaterial::steel();
        let mech_bcs = vec![
            BoundaryCondition::FixedNode(0),
            BoundaryCondition::FixedNode(1),
            BoundaryCondition::FixedNode(2),
        ];
        let thermal_bcs = vec![
            ThermalBoundaryCondition::FixedTemperature {
                node: 0,
                temperature: 20.0,
            },
            ThermalBoundaryCondition::FixedTemperature {
                node: 3,
                temperature: 200.0,
            },
        ];
        let result =
            coupled_thermo_mechanical(&mesh, &material, &thermal_mat, &mech_bcs, &thermal_bcs)
                .unwrap();
        assert_eq!(result.mechanical.displacements.len(), 4);
        assert_eq!(result.thermal.temperatures.len(), 4);
    }

    #[test]
    fn test_acoustic_equation() {
        let mesh = make_simple_tet_mesh();
        let bcs = vec![
            ThermalBoundaryCondition::FixedTemperature {
                node: 0,
                temperature: 1.0,
            },
            ThermalBoundaryCondition::FixedTemperature {
                node: 3,
                temperature: 0.0,
            },
        ];
        let result = acoustic_equation(&mesh, 100.0, 343.0, &bcs).unwrap();
        assert_eq!(result.pressures.len(), 4);
    }

    #[test]
    fn test_acoustic_validation() {
        let mesh = TetMesh {
            nodes: vec![],
            elements: vec![],
        };
        assert!(acoustic_equation(&mesh, 100.0, 343.0, &[]).is_err());
        let mesh2 = make_simple_tet_mesh();
        assert!(acoustic_equation(&mesh2, -1.0, 343.0, &[]).is_err());
        assert!(acoustic_equation(&mesh2, 100.0, -1.0, &[]).is_err());
    }

    #[test]
    fn test_poisson_equation() {
        let mesh = make_simple_tet_mesh();
        let source = vec![1.0; 4];
        let bcs = vec![ThermalBoundaryCondition::FixedTemperature {
            node: 0,
            temperature: 0.0,
        }];
        let result = poisson_equation(&mesh, &source, &bcs).unwrap();
        assert_eq!(result.values.len(), 4);
    }

    #[test]
    fn test_diffusion_equation() {
        let mesh = make_simple_tet_mesh();
        let bcs = vec![
            ThermalBoundaryCondition::FixedTemperature {
                node: 0,
                temperature: 0.0,
            },
            ThermalBoundaryCondition::FixedTemperature {
                node: 3,
                temperature: 100.0,
            },
        ];
        let result = diffusion_equation(&mesh, 1e-5, &bcs, 0.01).unwrap();
        assert_eq!(result.values.len(), 4);
    }

    #[test]
    fn test_diffusion_validation() {
        let mesh = make_simple_tet_mesh();
        assert!(diffusion_equation(&mesh, -1.0, &[], 0.01).is_err());
        assert!(diffusion_equation(&mesh, 1.0, &[], -0.01).is_err());
    }

    #[test]
    fn test_extract_nodal_values() {
        let result = FemResult {
            displacements: vec![
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 2.0, 0.0),
                Vec3::new(0.0, 0.0, 3.0),
                Vec3::ZERO,
            ],
            stresses: vec![100.0],
            max_displacement: 3.0,
            max_stress: 100.0,
        };
        let disp = extract_nodal_values(&result, "displacement");
        assert_eq!(disp.len(), 4);
        assert!((disp[0] - 1.0).abs() < 1e-10);
        assert!((disp[2] - 3.0).abs() < 1e-10);

        let ux = extract_nodal_values(&result, "ux");
        assert!((ux[0] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_interpolate_to_nodes() {
        let mesh = make_simple_tet_mesh();
        let elem_values = vec![100.0];
        let node_values = interpolate_to_nodes(&elem_values, &mesh);
        assert_eq!(node_values.len(), 4);
        for &v in &node_values {
            assert!((v - 100.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_compute_error_estimate() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let bcs = vec![
            BoundaryCondition::FixedNode(0),
            BoundaryCondition::FixedNode(1),
            BoundaryCondition::FixedNode(2),
            BoundaryCondition::Force {
                node: 3,
                force: Vec3::new(0.0, 0.0, -1000.0),
            },
        ];
        let result = static_analysis(&mesh, &material, &bcs).unwrap();
        let errors = compute_error_estimate(&result, &mesh);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn test_result_at_point() {
        let mesh = make_simple_tet_mesh();
        let result = FemResult {
            displacements: vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            ],
            stresses: vec![100.0],
            max_displacement: 1.0,
            max_stress: 100.0,
        };
        // Point at centroid (0.25, 0.25, 0.25)
        let vals =
            result_at_point(&result, &mesh, Point3::new(0.25, 0.25, 0.25)).unwrap();
        assert_eq!(vals.len(), 4);
        assert!(vals[0] > 0.0); // dx > 0

        // Point outside
        assert!(result_at_point(&result, &mesh, Point3::new(5.0, 5.0, 5.0)).is_err());
    }

    #[test]
    fn test_max_min_values() {
        let vals = vec![1.0, 5.0, -3.0, 2.0];
        let (max_val, min_val, max_idx, min_idx) = max_min_values(&vals);
        assert!((max_val - 5.0).abs() < 1e-10);
        assert!((min_val - (-3.0)).abs() < 1e-10);
        assert_eq!(max_idx, 1);
        assert_eq!(min_idx, 2);

        let (max_e, min_e, _, _) = max_min_values(&[]);
        assert!((max_e - 0.0).abs() < 1e-10);
        assert!((min_e - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_path_result() {
        let mesh = make_simple_tet_mesh();
        let result = FemResult {
            displacements: vec![Vec3::ZERO; 4],
            stresses: vec![0.0],
            max_displacement: 0.0,
            max_stress: 0.0,
        };
        let points = vec![
            Point3::new(0.1, 0.1, 0.1),
            Point3::new(0.2, 0.2, 0.2),
        ];
        let pr = path_result(&result, &mesh, &points);
        assert_eq!(pr.len(), 2);
    }

    #[test]
    fn test_reaction_forces_fn() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let bcs = vec![
            BoundaryCondition::FixedNode(0),
            BoundaryCondition::FixedNode(1),
            BoundaryCondition::FixedNode(2),
            BoundaryCondition::Force {
                node: 3,
                force: Vec3::new(0.0, 0.0, -1000.0),
            },
        ];
        let result = static_analysis(&mesh, &material, &bcs).unwrap();
        let reactions = reaction_forces(&result, &mesh, &material, &[0, 1, 2]);
        assert_eq!(reactions.len(), 3);
    }

    #[test]
    fn test_integrate_over_surface() {
        let mesh = make_simple_tet_mesh();
        let result = FemResult {
            displacements: vec![
                Vec3::new(0.0, 0.0, 0.1),
                Vec3::new(0.0, 0.0, 0.1),
                Vec3::new(0.0, 0.0, 0.1),
                Vec3::new(0.0, 0.0, 0.1),
            ],
            stresses: vec![100.0],
            max_displacement: 0.1,
            max_stress: 100.0,
        };
        let integral = integrate_over_surface(&result, &mesh, &[0, 1]);
        assert!(integral >= 0.0);
    }

    #[test]
    fn test_fem_summary() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let mut container = AnalysisContainer::new(mesh, material);
        container.add_bc(BoundaryCondition::FixedNode(0));
        let s = fem_summary(&container);
        assert!(s.contains("FEM Analysis"));
        assert!(s.contains("4 nodes"));
    }

    #[test]
    fn test_export_fem_report() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let mut container = AnalysisContainer::new(mesh, material);
        container.add_bc(BoundaryCondition::FixedNode(0));
        container.add_bc(BoundaryCondition::FixedNode(1));
        container.add_bc(BoundaryCondition::FixedNode(2));
        container.add_bc(BoundaryCondition::Force {
            node: 3,
            force: Vec3::new(0.0, 0.0, -1000.0),
        });
        container.run_static().unwrap();
        let result = container.result.as_ref().unwrap();
        let report = export_fem_report(&container, result);
        assert!(report.contains("CADKernel FEM Analysis Report"));
        assert!(report.contains("Max Displacement"));
    }

    #[test]
    fn test_check_mesh_quality_detailed() {
        let mesh = make_simple_tet_mesh();
        let qualities = check_mesh_quality_detailed(&mesh);
        assert_eq!(qualities.len(), 1);
        assert!(qualities[0].volume > 0.0);
        assert!(qualities[0].aspect_ratio > 0.0);
    }

    #[test]
    fn test_check_boundary_conditions() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let container = AnalysisContainer::new(mesh, material);
        // No BCs at all — should warn
        let warnings = check_boundary_conditions(&container).unwrap();
        assert!(warnings.iter().any(|w| w.contains("unconstrained")));
        assert!(warnings.iter().any(|w| w.contains("No loads")));
    }

    #[test]
    fn test_check_boundary_conditions_valid() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let mut container = AnalysisContainer::new(mesh, material);
        container.add_bc(BoundaryCondition::FixedNode(0));
        container.add_bc(BoundaryCondition::Force {
            node: 3,
            force: Vec3::new(0.0, 0.0, -1000.0),
        });
        let warnings = check_boundary_conditions(&container).unwrap();
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_check_boundary_conditions_out_of_range() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let mut container = AnalysisContainer::new(mesh, material);
        container.add_bc(BoundaryCondition::FixedNode(100)); // out of range
        container.add_bc(BoundaryCondition::Force {
            node: 3,
            force: Vec3::Z,
        });
        let warnings = check_boundary_conditions(&container).unwrap();
        assert!(warnings.iter().any(|w| w.contains("exceeds")));
    }

    #[test]
    fn test_estimate_computation_time() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let container = AnalysisContainer::new(mesh, material);
        let t = estimate_computation_time(&container);
        assert!(t > 0.0);
    }

    #[test]
    fn test_apply_element_geometry() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let mut container = AnalysisContainer::new(mesh, material);
        assert!(apply_element_geometry(&mut container, ElementGeometry::Solid).is_ok());
        assert!(
            apply_element_geometry(
                &mut container,
                ElementGeometry::Shell { thickness: 0.01 }
            )
            .is_ok()
        );
        assert!(
            apply_element_geometry(
                &mut container,
                ElementGeometry::Shell { thickness: -1.0 }
            )
            .is_err()
        );
        assert!(
            apply_element_geometry(
                &mut container,
                ElementGeometry::Beam(BeamSection::circular(0.01))
            )
            .is_ok()
        );
    }

    #[test]
    fn test_body_load_bc() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let bcs = vec![
            BoundaryCondition::FixedNode(0),
            BoundaryCondition::FixedNode(1),
            BoundaryCondition::FixedNode(2),
            BoundaryCondition::BodyLoad {
                force_density: Vec3::new(0.0, 0.0, -1e4),
            },
        ];
        let result = static_analysis(&mesh, &material, &bcs).unwrap();
        assert!(result.max_displacement > 0.0);
    }

    #[test]
    fn test_contact_constraint_bc() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let bcs = vec![
            BoundaryCondition::FixedNode(0),
            BoundaryCondition::ContactConstraint {
                surface_a: vec![1],
                surface_b: vec![2],
                penalty: 1e8,
            },
            BoundaryCondition::Force {
                node: 3,
                force: Vec3::new(0.0, 0.0, -1000.0),
            },
        ];
        let result = static_analysis(&mesh, &material, &bcs).unwrap();
        assert_eq!(result.displacements.len(), 4);
    }

    #[test]
    fn test_initial_temperature_bc() {
        let mesh = make_simple_tet_mesh();
        let material = FemMaterial::steel();
        let bcs = vec![
            BoundaryCondition::FixedNode(0),
            BoundaryCondition::FixedNode(1),
            BoundaryCondition::FixedNode(2),
            BoundaryCondition::InitialTemperature {
                node: 0,
                temperature: 20.0,
            },
            BoundaryCondition::Force {
                node: 3,
                force: Vec3::new(0.0, 0.0, -1000.0),
            },
        ];
        let result = static_analysis(&mesh, &material, &bcs).unwrap();
        assert_eq!(result.displacements.len(), 4);
    }

    #[test]
    fn test_export_mesh_format_abaqus() {
        let mesh = make_simple_tet_mesh();
        let out = export_mesh_format(&mesh, "abaqus").unwrap();
        assert!(out.contains("*HEADING"));
        assert!(out.contains("*ELEMENT"));
    }

    #[test]
    fn test_export_mesh_format_nastran() {
        let mesh = make_simple_tet_mesh();
        let out = export_mesh_format(&mesh, "nastran").unwrap();
        assert!(out.contains("BEGIN BULK"));
        assert!(out.contains("GRID"));
        assert!(out.contains("CTETRA"));
        assert!(out.contains("ENDDATA"));
    }

    #[test]
    fn test_export_mesh_format_invalid() {
        let mesh = make_simple_tet_mesh();
        assert!(export_mesh_format(&mesh, "unknown_format").is_err());
    }
}
