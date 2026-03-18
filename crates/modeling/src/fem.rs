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
}
