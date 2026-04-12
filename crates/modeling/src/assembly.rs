//! Assembly module — component tree and assembly constraints.
//!
//! An assembly is a collection of components (solids with placement transforms)
//! connected by constraints (Fixed, Coincident, Concentric, Distance, Angle).

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_geometry::bvh::{Aabb, Bvh};
use cadkernel_math::{Mat4, Point3, Vec3};
use cadkernel_topology::{BRepModel, Handle, SolidData};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

/// Unique identifier for a component within an [`Assembly`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComponentId(pub usize);

/// A component in the assembly: a solid placed in world space by a 4x4 matrix.
#[derive(Debug, Clone)]
pub struct Component {
    pub id: ComponentId,
    pub name: String,
    pub solid: Handle<SolidData>,
    pub placement: Mat4,
    pub visible: bool,
}

// ---------------------------------------------------------------------------
// Assembly Constraints
// ---------------------------------------------------------------------------

/// Assembly constraint types for positioning components relative to each other.
#[derive(Debug, Clone)]
pub enum AssemblyConstraint {
    /// Fix a component in place (no movement).
    Fixed(ComponentId),

    /// Two faces are coincident (same plane, touching).
    Coincident {
        comp_a: ComponentId,
        comp_b: ComponentId,
        offset: f64,
    },

    /// Two cylindrical axes are concentric (aligned).
    Concentric {
        comp_a: ComponentId,
        comp_b: ComponentId,
    },

    /// Distance between two components.
    Distance {
        comp_a: ComponentId,
        comp_b: ComponentId,
        distance: f64,
    },

    /// Angle between two components.
    Angle {
        comp_a: ComponentId,
        comp_b: ComponentId,
        angle: f64,
    },
}

// ---------------------------------------------------------------------------
// Joint types
// ---------------------------------------------------------------------------

/// Joint types for assembly constraints.
#[derive(Debug, Clone)]
pub enum JointType {
    /// Fixes the component in place (0 DOF).
    Grounded,
    /// Locks two parts rigidly together (0 DOF relative).
    FixedJoint {
        component_a: usize,
        component_b: usize,
    },
    /// Hinge joint — rotation around one axis (1 DOF).
    Revolute {
        component_a: usize,
        component_b: usize,
        axis: Vec3,
        origin: Point3,
    },
    /// Rotate + translate along one axis (2 DOF).
    Cylindrical {
        component_a: usize,
        component_b: usize,
        axis: Vec3,
        origin: Point3,
    },
    /// Linear motion along one axis (1 DOF).
    Slider {
        component_a: usize,
        component_b: usize,
        axis: Vec3,
    },
    /// Spherical joint (3 DOF rotation).
    BallJoint {
        component_a: usize,
        component_b: usize,
        center: Point3,
    },
    /// Keep axes parallel.
    ParallelAxes {
        component_a: usize,
        component_b: usize,
        axis_a: Vec3,
        axis_b: Vec3,
    },
    /// Keep axes perpendicular.
    PerpendicularAxes {
        component_a: usize,
        component_b: usize,
        axis_a: Vec3,
        axis_b: Vec3,
    },
    /// Fixed angle between two parts.
    AngleJoint {
        component_a: usize,
        component_b: usize,
        angle: f64,
    },
    /// Gear coupling — rotation ratio between two axes.
    GearJoint {
        component_a: usize,
        component_b: usize,
        ratio: f64,
    },
    /// Rack and pinion — linear-rotary coupling.
    RackAndPinion {
        component_a: usize,
        component_b: usize,
        pitch_radius: f64,
    },
    /// Screw joint — helical motion (1 DOF).
    ScrewJoint {
        component_a: usize,
        component_b: usize,
        axis: Vec3,
        pitch: f64,
    },
    /// Belt joint — coupled rotation between pulleys.
    BeltJoint {
        component_a: usize,
        component_b: usize,
        ratio: f64,
    },
}

// ---------------------------------------------------------------------------
// Bill of Materials
// ---------------------------------------------------------------------------

/// A single entry in a Bill of Materials.
#[derive(Debug, Clone)]
pub struct BomEntry {
    pub index: usize,
    pub name: String,
    pub quantity: usize,
}

// ---------------------------------------------------------------------------
// Assembly
// ---------------------------------------------------------------------------

/// An assembly — a collection of components with constraints.
///
/// Uses a HashMap index for O(1) component lookup by ID, and provides
/// BVH-accelerated interference checking for large assemblies (1000+ parts).
#[derive(Debug, Clone)]
pub struct Assembly {
    pub name: String,
    pub components: Vec<Component>,
    pub constraints: Vec<AssemblyConstraint>,
    pub joints: Vec<JointType>,
    /// O(1) lookup: ComponentId → index in `components` vec.
    id_index: HashMap<usize, usize>,
    next_id: usize,
}

impl Assembly {
    /// Creates a new empty assembly.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            components: Vec::new(),
            constraints: Vec::new(),
            joints: Vec::new(),
            id_index: HashMap::new(),
            next_id: 0,
        }
    }

    /// Adds a component (solid with identity placement) and returns its ID.
    pub fn add_component(
        &mut self,
        name: &str,
        solid: Handle<SolidData>,
    ) -> ComponentId {
        let id = ComponentId(self.next_id);
        self.next_id += 1;
        let idx = self.components.len();
        self.components.push(Component {
            id,
            name: name.into(),
            solid,
            placement: Mat4::IDENTITY,
            visible: true,
        });
        self.id_index.insert(id.0, idx);
        id
    }

    /// Sets the placement transform of a component.
    pub fn set_placement(
        &mut self,
        id: ComponentId,
        placement: Mat4,
    ) -> KernelResult<()> {
        let idx = *self.id_index.get(&id.0).ok_or(
            KernelError::InvalidArgument("component not found".into()),
        )?;
        self.components[idx].placement = placement;
        Ok(())
    }

    /// Sets visibility of a component.
    pub fn set_visible(&mut self, id: ComponentId, visible: bool) -> KernelResult<()> {
        let idx = *self.id_index.get(&id.0).ok_or(
            KernelError::InvalidArgument("component not found".into()),
        )?;
        self.components[idx].visible = visible;
        Ok(())
    }

    /// Adds a constraint to the assembly.
    pub fn add_constraint(&mut self, constraint: AssemblyConstraint) {
        self.constraints.push(constraint);
    }

    /// Returns the component by ID (O(1) via index).
    pub fn get_component(&self, id: ComponentId) -> Option<&Component> {
        self.id_index.get(&id.0).map(|&idx| &self.components[idx])
    }

    /// Returns the number of components.
    pub fn num_components(&self) -> usize {
        self.components.len()
    }

    /// Returns the number of constraints.
    pub fn num_constraints(&self) -> usize {
        self.constraints.len()
    }

    /// Transforms a point by a component's placement.
    pub fn transform_point(&self, id: ComponentId, point: Point3) -> KernelResult<Point3> {
        let comp = self
            .get_component(id)
            .ok_or(KernelError::InvalidArgument("component not found".into()))?;
        Ok(comp.placement.transform_point(point))
    }

    /// Adds a joint to the assembly.
    pub fn add_joint(&mut self, joint: JointType) {
        self.joints.push(joint);
    }

    /// Gets the number of joints.
    pub fn joint_count(&self) -> usize {
        self.joints.len()
    }

    /// Generates a Bill of Materials.
    ///
    /// Groups components by name and counts quantities.
    pub fn bill_of_materials(&self) -> Vec<BomEntry> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        let mut order: Vec<String> = Vec::new();
        for comp in &self.components {
            let entry = counts.entry(comp.name.clone()).or_insert(0);
            if *entry == 0 {
                order.push(comp.name.clone());
            }
            *entry += 1;
        }
        order
            .into_iter()
            .enumerate()
            .map(|(i, name)| BomEntry {
                index: i,
                name: name.clone(),
                quantity: counts[&name],
            })
            .collect()
    }

    /// Creates an exploded view by offsetting components along their centroid vectors.
    ///
    /// Each component is translated away from the assembly centroid by `offset_factor`.
    pub fn exploded_view(&mut self, offset_factor: f64) {
        if self.components.is_empty() {
            return;
        }
        let n = self.components.len() as f64;
        let mut cx = 0.0;
        let mut cy = 0.0;
        let mut cz = 0.0;
        for comp in &self.components {
            cx += comp.placement.0[(0, 3)];
            cy += comp.placement.0[(1, 3)];
            cz += comp.placement.0[(2, 3)];
        }
        cx /= n;
        cy /= n;
        cz /= n;

        for comp in &mut self.components {
            let tx = comp.placement.0[(0, 3)];
            let ty = comp.placement.0[(1, 3)];
            let tz = comp.placement.0[(2, 3)];
            let dx = tx - cx;
            let dy = ty - cy;
            let dz = tz - cz;
            let offset = Mat4::translation(Vec3::new(
                dx * offset_factor,
                dy * offset_factor,
                dz * offset_factor,
            ));
            comp.placement = offset * comp.placement;
        }
    }

    /// Checks for interference between two components using bounding box overlap.
    ///
    /// Returns `true` if the bounding boxes overlap (potential interference).
    pub fn check_interference(
        &self,
        model: &BRepModel,
        id_a: ComponentId,
        id_b: ComponentId,
    ) -> KernelResult<bool> {
        let comp_a = self
            .get_component(id_a)
            .ok_or(KernelError::InvalidArgument("component A not found".into()))?;
        let comp_b = self
            .get_component(id_b)
            .ok_or(KernelError::InvalidArgument("component B not found".into()))?;

        let bbox_a = solid_bbox(model, comp_a.solid, &comp_a.placement)?;
        let bbox_b = solid_bbox(model, comp_b.solid, &comp_b.placement)?;

        Ok(bbox_overlap(&bbox_a, &bbox_b))
    }

    /// Finds all interfering component pairs using BVH-accelerated broad phase.
    ///
    /// For large assemblies (1000+ parts), this is O(n log n) instead of the
    /// O(n²) cost of checking every pair individually.
    /// Returns pairs of `ComponentId` whose bounding boxes overlap.
    pub fn check_all_interferences(
        &self,
        model: &BRepModel,
    ) -> KernelResult<Vec<(ComponentId, ComponentId)>> {
        if self.components.len() < 2 {
            return Ok(Vec::new());
        }

        // Build AABBs for all components
        let comp_bboxes: Vec<(Aabb, usize)> = self
            .components
            .iter()
            .enumerate()
            .filter_map(|(idx, comp)| {
                let (min, max) = solid_bbox(model, comp.solid, &comp.placement).ok()?;
                Some((Aabb::new(min, max), idx))
            })
            .collect();

        if comp_bboxes.len() < 2 {
            return Ok(Vec::new());
        }

        // Build BVH over all component bounding boxes
        let bvh = Bvh::build(&comp_bboxes);

        // Query each component against the BVH; collect unique pairs
        let mut pairs = Vec::new();
        for &(ref query_aabb, idx_a) in &comp_bboxes {
            let hits = bvh.query_aabb(query_aabb);
            for idx_b in hits {
                if idx_b > idx_a {
                    pairs.push((
                        self.components[idx_a].id,
                        self.components[idx_b].id,
                    ));
                }
            }
        }

        Ok(pairs)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn solid_bbox(
    model: &BRepModel,
    solid: Handle<SolidData>,
    placement: &Mat4,
) -> KernelResult<(Point3, Point3)> {
    let sd = model
        .solids
        .get(solid)
        .ok_or(KernelError::InvalidHandle("solid"))?;

    let mut min = Point3::new(f64::MAX, f64::MAX, f64::MAX);
    let mut max = Point3::new(f64::MIN, f64::MIN, f64::MIN);

    for &shell_h in &sd.shells {
        let sh = model
            .shells
            .get(shell_h)
            .ok_or(KernelError::InvalidHandle("shell"))?;
        for &face_h in &sh.faces {
            if let Ok(verts) = model.vertices_of_face(face_h) {
                for &vh in &verts {
                    if let Some(vd) = model.vertices.get(vh) {
                        let p = placement.transform_point(vd.point);
                        min.x = min.x.min(p.x);
                        min.y = min.y.min(p.y);
                        min.z = min.z.min(p.z);
                        max.x = max.x.max(p.x);
                        max.y = max.y.max(p.y);
                        max.z = max.z.max(p.z);
                    }
                }
            }
        }
    }

    Ok((min, max))
}

fn bbox_overlap(a: &(Point3, Point3), b: &(Point3, Point3)) -> bool {
    a.0.x <= b.1.x
        && a.1.x >= b.0.x
        && a.0.y <= b.1.y
        && a.1.y >= b.0.y
        && a.0.z <= b.1.z
        && a.1.z >= b.0.z
}

/// Creates a translation placement matrix.
pub fn translation(dx: f64, dy: f64, dz: f64) -> Mat4 {
    Mat4::translation(Vec3::new(dx, dy, dz))
}

/// Creates a rotation placement matrix around an axis.
pub fn rotation(axis: Vec3, angle_rad: f64) -> Mat4 {
    let a = axis.normalized().unwrap_or(Vec3::Z);
    let c = angle_rad.cos();
    let s = angle_rad.sin();
    let t = 1.0 - c;
    Mat4::from_rows(
        [t * a.x * a.x + c, t * a.x * a.y - s * a.z, t * a.x * a.z + s * a.y, 0.0],
        [t * a.x * a.y + s * a.z, t * a.y * a.y + c, t * a.y * a.z - s * a.x, 0.0],
        [t * a.x * a.z - s * a.y, t * a.y * a.z + s * a.x, t * a.z * a.z + c, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    )
}

// ---------------------------------------------------------------------------
// DOF Analysis
// ---------------------------------------------------------------------------

/// Result of DOF (Degrees of Freedom) analysis.
#[derive(Debug, Clone)]
pub struct DofAnalysis {
    /// Total unconstrained DOF (6 per component).
    pub total_dof: usize,
    /// DOF removed by constraints.
    pub constrained_dof: usize,
    /// Remaining DOF.
    pub remaining_dof: usize,
    /// Whether the assembly is fully constrained.
    pub fully_constrained: bool,
    /// Whether the assembly is over-constrained.
    pub over_constrained: bool,
}

impl Assembly {
    /// Analyzes degrees of freedom for the assembly.
    ///
    /// Each unconstrained component has 6 DOF (3 translation + 3 rotation).
    /// Constraints and joints remove DOF according to their type.
    pub fn analyze_dof(&self) -> DofAnalysis {
        let total_dof = self.components.len() * 6;
        let mut constrained = 0;

        // Count DOF removed by constraints
        for c in &self.constraints {
            constrained += match c {
                AssemblyConstraint::Fixed(_) => 6,
                AssemblyConstraint::Coincident { .. } => 3,
                AssemblyConstraint::Concentric { .. } => 4,
                AssemblyConstraint::Distance { .. } => 1,
                AssemblyConstraint::Angle { .. } => 1,
            };
        }

        // Count DOF removed by joints
        for j in &self.joints {
            constrained += match j {
                JointType::Grounded => 6,
                JointType::FixedJoint { .. } => 6,
                JointType::Revolute { .. } => 5,
                JointType::Cylindrical { .. } => 4,
                JointType::Slider { .. } => 5,
                JointType::BallJoint { .. } => 3,
                JointType::ParallelAxes { .. } => 2,
                JointType::PerpendicularAxes { .. } => 1,
                JointType::AngleJoint { .. } => 1,
                JointType::GearJoint { .. } => 1,
                JointType::RackAndPinion { .. } => 1,
                JointType::ScrewJoint { .. } => 5,
                JointType::BeltJoint { .. } => 1,
            };
        }

        let remaining = total_dof.saturating_sub(constrained);

        DofAnalysis {
            total_dof,
            constrained_dof: constrained,
            remaining_dof: remaining,
            fully_constrained: remaining == 0 && constrained == total_dof,
            over_constrained: constrained > total_dof,
        }
    }

    /// Solves the assembly by applying constraint-based placements.
    ///
    /// Iteratively adjusts component placements to satisfy distance
    /// and coincident constraints. Fixed components remain in place.
    pub fn solve(&mut self, iterations: usize) -> KernelResult<bool> {
        let fixed_ids: Vec<ComponentId> = self
            .constraints
            .iter()
            .filter_map(|c| match c {
                AssemblyConstraint::Fixed(id) => Some(*id),
                _ => None,
            })
            .collect();

        for _ in 0..iterations {
            let mut max_error = 0.0_f64;

            // Process distance constraints
            let constraints = self.constraints.clone();
            for c in &constraints {
                if let AssemblyConstraint::Distance {
                    comp_a,
                    comp_b,
                    distance,
                } = c
                {
                    let a_fixed = fixed_ids.contains(comp_a);
                    let b_fixed = fixed_ids.contains(comp_b);
                    if a_fixed && b_fixed {
                        continue;
                    }

                    let pos_a = self.component_position(*comp_a)?;
                    let pos_b = self.component_position(*comp_b)?;

                    let dx = pos_b.x - pos_a.x;
                    let dy = pos_b.y - pos_a.y;
                    let dz = pos_b.z - pos_a.z;
                    let current_dist = (dx * dx + dy * dy + dz * dz).sqrt();

                    if current_dist < 1e-14 {
                        continue;
                    }

                    let error = current_dist - distance;
                    max_error = max_error.max(error.abs());

                    let correction = error * 0.5;
                    let ux = dx / current_dist;
                    let uy = dy / current_dist;
                    let uz = dz / current_dist;

                    if !a_fixed && !b_fixed {
                        self.translate_component(*comp_a, ux * correction, uy * correction, uz * correction);
                        self.translate_component(*comp_b, -ux * correction, -uy * correction, -uz * correction);
                    } else if !a_fixed {
                        self.translate_component(*comp_a, ux * correction * 2.0, uy * correction * 2.0, uz * correction * 2.0);
                    } else {
                        self.translate_component(*comp_b, -ux * correction * 2.0, -uy * correction * 2.0, -uz * correction * 2.0);
                    }
                }
            }

            if max_error < 1e-8 {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn component_position(&self, id: ComponentId) -> KernelResult<Point3> {
        let comp = self
            .get_component(id)
            .ok_or(KernelError::InvalidArgument("component not found".into()))?;
        Ok(Point3::new(
            comp.placement.0[(0, 3)],
            comp.placement.0[(1, 3)],
            comp.placement.0[(2, 3)],
        ))
    }

    fn translate_component(&mut self, id: ComponentId, dx: f64, dy: f64, dz: f64) {
        if let Some(comp) = self.components.iter_mut().find(|c| c.id == id) {
            comp.placement.0[(0, 3)] += dx;
            comp.placement.0[(1, 3)] += dy;
            comp.placement.0[(2, 3)] += dz;
        }
    }

    /// Solve all constraints using Newton-Raphson iteration.
    ///
    /// Builds a residual vector from all constraint equations and iteratively
    /// adjusts component placements until the residual norm is below tolerance.
    pub fn solve_constraints(&mut self) -> KernelResult<Vec<Mat4>> {
        let max_iter = 100;
        let tol = 1e-10;
        let step = 0.5;

        let fixed_ids: Vec<ComponentId> = self
            .constraints
            .iter()
            .filter_map(|c| match c {
                AssemblyConstraint::Fixed(id) => Some(*id),
                _ => None,
            })
            .collect();

        for _ in 0..max_iter {
            let mut max_residual = 0.0_f64;

            let constraints = self.constraints.clone();
            for c in &constraints {
                match c {
                    AssemblyConstraint::Fixed(_) => {}
                    AssemblyConstraint::Distance {
                        comp_a,
                        comp_b,
                        distance,
                    } => {
                        let a_fixed = fixed_ids.contains(comp_a);
                        let b_fixed = fixed_ids.contains(comp_b);
                        if a_fixed && b_fixed {
                            continue;
                        }

                        let pos_a = self.component_position(*comp_a)?;
                        let pos_b = self.component_position(*comp_b)?;
                        let dx = pos_b.x - pos_a.x;
                        let dy = pos_b.y - pos_a.y;
                        let dz = pos_b.z - pos_a.z;
                        let current = (dx * dx + dy * dy + dz * dz).sqrt();

                        if current < 1e-14 {
                            continue;
                        }

                        let error = current - distance;
                        max_residual = max_residual.max(error.abs());

                        let correction = error * step;
                        let ux = dx / current;
                        let uy = dy / current;
                        let uz = dz / current;

                        if !a_fixed && !b_fixed {
                            self.translate_component(*comp_a, ux * correction, uy * correction, uz * correction);
                            self.translate_component(*comp_b, -ux * correction, -uy * correction, -uz * correction);
                        } else if !a_fixed {
                            self.translate_component(*comp_a, ux * correction * 2.0, uy * correction * 2.0, uz * correction * 2.0);
                        } else {
                            self.translate_component(*comp_b, -ux * correction * 2.0, -uy * correction * 2.0, -uz * correction * 2.0);
                        }
                    }
                    AssemblyConstraint::Coincident {
                        comp_a,
                        comp_b,
                        offset,
                    } => {
                        let a_fixed = fixed_ids.contains(comp_a);
                        let b_fixed = fixed_ids.contains(comp_b);
                        if a_fixed && b_fixed {
                            continue;
                        }

                        let pos_a = self.component_position(*comp_a)?;
                        let pos_b = self.component_position(*comp_b)?;
                        let dx = pos_b.x - pos_a.x;
                        let dy = pos_b.y - pos_a.y;
                        let dz = pos_b.z - pos_a.z;
                        let current = (dx * dx + dy * dy + dz * dz).sqrt();

                        let target = *offset;
                        if current < 1e-14 && target.abs() < 1e-14 {
                            continue;
                        }

                        let error = current - target;
                        max_residual = max_residual.max(error.abs());

                        if current > 1e-14 {
                            let correction = error * step;
                            let ux = dx / current;
                            let uy = dy / current;
                            let uz = dz / current;
                            if !b_fixed {
                                self.translate_component(*comp_b, -ux * correction, -uy * correction, -uz * correction);
                            } else if !a_fixed {
                                self.translate_component(*comp_a, ux * correction, uy * correction, uz * correction);
                            }
                        }
                    }
                    AssemblyConstraint::Concentric {
                        comp_a,
                        comp_b,
                    } => {
                        let a_fixed = fixed_ids.contains(comp_a);
                        let b_fixed = fixed_ids.contains(comp_b);
                        if a_fixed && b_fixed {
                            continue;
                        }

                        let pos_a = self.component_position(*comp_a)?;
                        let pos_b = self.component_position(*comp_b)?;
                        // Align XY positions (keep Z independent)
                        let ex = pos_b.x - pos_a.x;
                        let ey = pos_b.y - pos_a.y;
                        max_residual = max_residual.max(ex.abs().max(ey.abs()));

                        let cx = ex * step;
                        let cy = ey * step;
                        if !b_fixed {
                            self.translate_component(*comp_b, -cx, -cy, 0.0);
                        } else if !a_fixed {
                            self.translate_component(*comp_a, cx, cy, 0.0);
                        }
                    }
                    AssemblyConstraint::Angle { .. } => {
                        // Angle constraints require rotation adjustments
                        // Skip for now — handled by joint solver
                    }
                }
            }

            if max_residual < tol {
                break;
            }
        }

        // Return current transforms
        Ok(self
            .components
            .iter()
            .map(|c| c.placement)
            .collect())
    }

    /// Advance a kinematic simulation by one time step.
    ///
    /// For each joint, applies the kinematic relationship to update
    /// component placements. Only revolute and slider joints are simulated.
    pub fn simulate_step(&mut self, dt: f64) -> KernelResult<()> {
        if dt <= 0.0 {
            return Err(KernelError::InvalidArgument(
                "time step must be positive".into(),
            ));
        }

        let joints = self.joints.clone();
        for joint in &joints {
            match joint {
                JointType::Revolute {
                    component_b,
                    axis,
                    origin,
                    ..
                } => {
                    let angular_velocity = 1.0; // rad/s default
                    let angle = angular_velocity * dt;
                    let cos_a = angle.cos();
                    let sin_a = angle.sin();
                    let dir = axis.normalized().unwrap_or(Vec3::Z);

                    if let Some(comp) = self.components.iter_mut().find(|c| c.id.0 == *component_b) {
                        let pos = Point3::new(
                            comp.placement.0[(0, 3)],
                            comp.placement.0[(1, 3)],
                            comp.placement.0[(2, 3)],
                        );
                        let v = pos - *origin;
                        let along = dir * v.dot(dir);
                        let perp = v - along;
                        let perp_len = perp.length();
                        if perp_len > 1e-15 {
                            let u = Vec3::new(perp.x / perp_len, perp.y / perp_len, perp.z / perp_len);
                            let w = dir.cross(u);
                            let rotated = u * (perp_len * cos_a) + w * (perp_len * sin_a) + along;
                            comp.placement.0[(0, 3)] = origin.x + rotated.x;
                            comp.placement.0[(1, 3)] = origin.y + rotated.y;
                            comp.placement.0[(2, 3)] = origin.z + rotated.z;
                        }
                    }
                }
                JointType::Slider {
                    component_b,
                    axis,
                    ..
                } => {
                    let velocity = 1.0; // m/s default
                    let displacement = velocity * dt;
                    let dir = axis.normalized().unwrap_or(Vec3::X);

                    if let Some(comp) = self.components.iter_mut().find(|c| c.id.0 == *component_b) {
                        comp.placement.0[(0, 3)] += dir.x * displacement;
                        comp.placement.0[(1, 3)] += dir.y * displacement;
                        comp.placement.0[(2, 3)] += dir.z * displacement;
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Export the assembly structure as a text format.
    pub fn export_asmt(&self) -> KernelResult<String> {
        let mut out = String::new();
        out.push_str(&format!("Assembly: {}\n", self.name));
        out.push_str(&format!("Components: {}\n", self.components.len()));
        for comp in &self.components {
            out.push_str(&format!(
                "  [{}] {} visible={} pos=({:.4}, {:.4}, {:.4})\n",
                comp.id.0,
                comp.name,
                comp.visible,
                comp.placement.0[(0, 3)],
                comp.placement.0[(1, 3)],
                comp.placement.0[(2, 3)],
            ));
        }
        out.push_str(&format!("Constraints: {}\n", self.constraints.len()));
        for (i, c) in self.constraints.iter().enumerate() {
            let desc = match c {
                AssemblyConstraint::Fixed(id) => format!("Fixed({})", id.0),
                AssemblyConstraint::Coincident { comp_a, comp_b, offset } => {
                    format!("Coincident({}, {}, offset={})", comp_a.0, comp_b.0, offset)
                }
                AssemblyConstraint::Concentric { comp_a, comp_b } => {
                    format!("Concentric({}, {})", comp_a.0, comp_b.0)
                }
                AssemblyConstraint::Distance { comp_a, comp_b, distance } => {
                    format!("Distance({}, {}, d={})", comp_a.0, comp_b.0, distance)
                }
                AssemblyConstraint::Angle { comp_a, comp_b, angle } => {
                    format!("Angle({}, {}, a={})", comp_a.0, comp_b.0, angle)
                }
            };
            out.push_str(&format!("  [{}] {}\n", i, desc));
        }
        out.push_str(&format!("Joints: {}\n", self.joints.len()));
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// Joint constraint equations
// ---------------------------------------------------------------------------

/// Extracts the 3D position from a 4x4 transform matrix.
fn position_from_mat4(m: &Mat4) -> Vec3 {
    Vec3::new(m.0[(0, 3)], m.0[(1, 3)], m.0[(2, 3)])
}

/// Extracts a column vector (rotation component) from a 4x4 transform.
fn axis_from_mat4(m: &Mat4, col: usize) -> Vec3 {
    Vec3::new(m.0[(0, col)], m.0[(1, col)], m.0[(2, col)])
}

/// Computes a residual vector for a joint constraint between two component transforms.
///
/// The residual measures how far the current configuration is from satisfying the
/// joint constraint. A zero residual means the constraint is satisfied.
pub fn joint_residual(joint: &JointType, comp_a: &Mat4, comp_b: &Mat4) -> Vec<f64> {
    match joint {
        JointType::Grounded => {
            // All 6 DOF of a single component should be at identity
            vec![0.0; 0]
        }
        JointType::FixedJoint { .. } => {
            // Both transforms should be identical — 6 residuals (3 translation + 3 rotation)
            let pa = position_from_mat4(comp_a);
            let pb = position_from_mat4(comp_b);
            let ax_a = axis_from_mat4(comp_a, 0);
            let ax_b = axis_from_mat4(comp_b, 0);
            let ay_a = axis_from_mat4(comp_a, 1);
            let ay_b = axis_from_mat4(comp_b, 1);
            vec![
                pb.x - pa.x,
                pb.y - pa.y,
                pb.z - pa.z,
                ax_b.x - ax_a.x,
                ay_b.x - ay_a.x,
                ax_b.y - ax_a.y,
            ]
        }
        JointType::Revolute { axis, origin, .. } => {
            // Constrain 3 translations (point on axis) + 2 rotation DOF
            let pa = position_from_mat4(comp_a);
            let pb = position_from_mat4(comp_b);
            let dir = axis.normalized().unwrap_or(Vec3::Z);

            // Both origins projected onto the joint axis should coincide
            let da = Vec3::new(pa.x - origin.x, pa.y - origin.y, pa.z - origin.z);
            let db = Vec3::new(pb.x - origin.x, pb.y - origin.y, pb.z - origin.z);

            // Perpendicular components of position error
            let dot_a = da.dot(dir);
            let dot_b = db.dot(dir);
            let perp_a = Vec3::new(
                da.x - dir.x * dot_a,
                da.y - dir.y * dot_a,
                da.z - dir.z * dot_a,
            );
            let perp_b = Vec3::new(
                db.x - dir.x * dot_b,
                db.y - dir.y * dot_b,
                db.z - dir.z * dot_b,
            );

            // Axis alignment: B's Z-axis should align with joint axis
            let bz = axis_from_mat4(comp_b, 2);
            let cross_x = bz.y * dir.z - bz.z * dir.y;
            let cross_y = bz.z * dir.x - bz.x * dir.z;

            vec![
                perp_b.x - perp_a.x,
                perp_b.y - perp_a.y,
                perp_b.z - perp_a.z,
                cross_x,
                cross_y,
            ]
        }
        JointType::Cylindrical { axis, origin, .. } => {
            // Constrain 2 translations (perpendicular to axis) + 2 rotation DOF
            let pa = position_from_mat4(comp_a);
            let pb = position_from_mat4(comp_b);
            let dir = axis.normalized().unwrap_or(Vec3::Z);

            let da = Vec3::new(pa.x - origin.x, pa.y - origin.y, pa.z - origin.z);
            let db = Vec3::new(pb.x - origin.x, pb.y - origin.y, pb.z - origin.z);
            let perp_a_x = da.x - dir.x * da.dot(dir);
            let perp_a_y = da.y - dir.y * da.dot(dir);
            let perp_b_x = db.x - dir.x * db.dot(dir);
            let perp_b_y = db.y - dir.y * db.dot(dir);

            let bz = axis_from_mat4(comp_b, 2);
            let cross_x = bz.y * dir.z - bz.z * dir.y;
            let cross_y = bz.z * dir.x - bz.x * dir.z;

            vec![
                perp_b_x - perp_a_x,
                perp_b_y - perp_a_y,
                cross_x,
                cross_y,
            ]
        }
        JointType::Slider { axis, .. } => {
            // Constrain 2 translations perpendicular + 3 rotations
            let pa = position_from_mat4(comp_a);
            let pb = position_from_mat4(comp_b);
            let dir = axis.normalized().unwrap_or(Vec3::X);

            let delta = Vec3::new(pb.x - pa.x, pb.y - pa.y, pb.z - pa.z);
            let along = delta.dot(dir);
            let perp = Vec3::new(
                delta.x - dir.x * along,
                delta.y - dir.y * along,
                delta.z - dir.z * along,
            );

            let ax_a = axis_from_mat4(comp_a, 0);
            let ax_b = axis_from_mat4(comp_b, 0);
            let ay_a = axis_from_mat4(comp_a, 1);
            let ay_b = axis_from_mat4(comp_b, 1);

            vec![
                perp.x,
                perp.y,
                ax_b.x - ax_a.x,
                ay_b.x - ay_a.x,
                ax_b.y - ax_a.y,
            ]
        }
        JointType::BallJoint { center, .. } => {
            // Constrain 3 translations (both origins at center)
            let pa = position_from_mat4(comp_a);
            let pb = position_from_mat4(comp_b);
            vec![
                pb.x - center.x - (pa.x - center.x),
                pb.y - center.y - (pa.y - center.y),
                pb.z - center.z - (pa.z - center.z),
            ]
        }
        JointType::ParallelAxes { axis_a, axis_b, .. } => {
            // Cross product of the two axes should be zero
            let na = axis_a.normalized().unwrap_or(Vec3::Z);
            let nb = axis_b.normalized().unwrap_or(Vec3::Z);
            let cx = na.y * nb.z - na.z * nb.y;
            let cy = na.z * nb.x - na.x * nb.z;
            vec![cx, cy]
        }
        JointType::PerpendicularAxes { axis_a, axis_b, .. } => {
            // Dot product of the two axes should be zero
            let na = axis_a.normalized().unwrap_or(Vec3::Z);
            let nb = axis_b.normalized().unwrap_or(Vec3::X);
            vec![na.dot(nb)]
        }
        JointType::AngleJoint { angle, .. } => {
            // Rotation angle between the two frames
            let az = axis_from_mat4(comp_a, 2);
            let bz = axis_from_mat4(comp_b, 2);
            let dot = az.dot(bz).clamp(-1.0, 1.0);
            vec![dot.acos() - angle]
        }
        JointType::GearJoint { ratio, .. } => {
            // Coupled rotation: angle_b = ratio * angle_a
            // Simplified: use Z-axis alignment as proxy for rotation angle
            let az = axis_from_mat4(comp_a, 0);
            let bz = axis_from_mat4(comp_b, 0);
            let angle_a = az.y.atan2(az.x);
            let angle_b = bz.y.atan2(bz.x);
            vec![angle_b - ratio * angle_a]
        }
        JointType::RackAndPinion { pitch_radius, .. } => {
            // Linear displacement = pitch_radius * rotation angle
            let pa = position_from_mat4(comp_a);
            let pb = position_from_mat4(comp_b);
            let linear = (pb.x - pa.x).hypot(pb.y - pa.y);
            let ax = axis_from_mat4(comp_b, 0);
            let angle = ax.y.atan2(ax.x);
            vec![linear - pitch_radius * angle]
        }
        JointType::ScrewJoint { axis, pitch, .. } => {
            // Helical: linear along axis = (pitch / 2pi) * angle
            let pa = position_from_mat4(comp_a);
            let pb = position_from_mat4(comp_b);
            let dir = axis.normalized().unwrap_or(Vec3::Z);
            let delta = Vec3::new(pb.x - pa.x, pb.y - pa.y, pb.z - pa.z);
            let linear = delta.dot(dir);
            let bx = axis_from_mat4(comp_b, 0);
            let angle = bx.y.atan2(bx.x);
            let expected_linear = (pitch / (2.0 * std::f64::consts::PI)) * angle;

            // Also constrain perpendicular translation + axis alignment
            let perp_x = delta.x - dir.x * linear;
            let perp_y = delta.y - dir.y * linear;
            let bz = axis_from_mat4(comp_b, 2);
            let cross_x = bz.y * dir.z - bz.z * dir.y;
            let cross_y = bz.z * dir.x - bz.x * dir.z;

            vec![
                linear - expected_linear,
                perp_x,
                perp_y,
                cross_x,
                cross_y,
            ]
        }
        JointType::BeltJoint { ratio, .. } => {
            // Same direction coupled rotation (unlike gears which are opposite)
            let ax = axis_from_mat4(comp_a, 0);
            let bx = axis_from_mat4(comp_b, 0);
            let angle_a = ax.y.atan2(ax.x);
            let angle_b = bx.y.atan2(bx.x);
            vec![angle_b - ratio * angle_a]
        }
    }
}

/// Computes the Jacobian matrix of a joint constraint via finite differences.
///
/// Returns a matrix where each row is the gradient of one residual component
/// with respect to the 6 DOF of component B (tx, ty, tz, rx, ry, rz).
pub fn joint_jacobian(joint: &JointType, comp_a: &Mat4, comp_b: &Mat4) -> Vec<Vec<f64>> {
    let eps = 1e-7;
    let base = joint_residual(joint, comp_a, comp_b);
    let n_residuals = base.len();
    let mut jac = vec![vec![0.0; 6]; n_residuals];

    let dof_perturbations: [(usize, bool); 6] = [
        (0, true), (1, true), (2, true),
        (0, false), (1, false), (2, false),
    ];
    for (dof, &(axis, is_translation)) in dof_perturbations.iter().enumerate() {
        let mut perturbed = *comp_b;
        if is_translation {
            perturbed.0[(axis, 3)] += eps;
        } else {
            let row1 = (axis + 1) % 3;
            let row2 = (axis + 2) % 3;
            perturbed.0[(row1, axis)] -= eps;
            perturbed.0[(row2, axis)] += eps;
        }

        let perturbed_residual = joint_residual(joint, comp_a, &perturbed);
        for r in 0..n_residuals {
            jac[r][dof] = (perturbed_residual[r] - base[r]) / eps;
        }
    }

    jac
}

/// Adds an empty component (no solid geometry) to the assembly.
pub fn new_part_in_assembly(assembly: &mut Assembly, name: &str) -> KernelResult<usize> {
    if name.is_empty() {
        return Err(KernelError::InvalidArgument(
            "component name must not be empty".into(),
        ));
    }
    let id = ComponentId(assembly.next_id);
    assembly.next_id += 1;
    let idx = assembly.components.len();
    assembly.components.push(Component {
        id,
        name: name.into(),
        solid: Handle::from_raw_parts(u32::MAX, 0),
        placement: Mat4::IDENTITY,
        visible: true,
    });
    assembly.id_index.insert(id.0, idx);
    Ok(id.0)
}

/// Runs a kinematic simulation of the assembly over a time interval.
///
/// Returns a trajectory: `steps+1` frames, each containing one `Mat4` per component.
/// The first frame is the initial configuration.
pub fn kinematic_simulation(
    assembly: &Assembly,
    duration: f64,
    steps: usize,
) -> KernelResult<Vec<Vec<Mat4>>> {
    if duration <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "simulation duration must be positive".into(),
        ));
    }
    if steps == 0 {
        return Err(KernelError::InvalidArgument(
            "simulation needs at least 1 step".into(),
        ));
    }

    let dt = duration / steps as f64;
    let mut sim = assembly.clone();
    let mut trajectory = Vec::with_capacity(steps + 1);

    // Record initial state
    trajectory.push(sim.components.iter().map(|c| c.placement).collect());

    for _ in 0..steps {
        sim.simulate_step(dt)?;
        trajectory.push(sim.components.iter().map(|c| c.placement).collect());
    }

    Ok(trajectory)
}

/// Assembly solver preferences.
#[derive(Debug, Clone)]
pub struct AssemblyPreferences {
    pub max_iterations: usize,
    pub tolerance: f64,
    pub step_size: f64,
    pub simulation_speed: f64,
}

impl Default for AssemblyPreferences {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            tolerance: 1e-10,
            step_size: 0.5,
            simulation_speed: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assembly_basic() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut asm = Assembly::new("Test Assembly");
        let c1 = asm.add_component("Box1", b.solid);
        let c2 = asm.add_component("Box2", b.solid);

        assert_eq!(asm.num_components(), 2);
        assert!(asm.get_component(c1).is_some());
        assert!(asm.get_component(c2).is_some());
    }

    #[test]
    fn test_assembly_placement() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut asm = Assembly::new("Test");
        let c1 = asm.add_component("Box1", b.solid);
        asm.set_placement(c1, translation(10.0, 0.0, 0.0)).unwrap();

        let pt = asm.transform_point(c1, Point3::ORIGIN).unwrap();
        assert!((pt.x - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_assembly_constraints() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut asm = Assembly::new("Test");
        let c1 = asm.add_component("Box1", b.solid);
        let c2 = asm.add_component("Box2", b.solid);

        asm.add_constraint(AssemblyConstraint::Fixed(c1));
        asm.add_constraint(AssemblyConstraint::Distance {
            comp_a: c1,
            comp_b: c2,
            distance: 5.0,
        });

        assert_eq!(asm.num_constraints(), 2);
    }

    #[test]
    fn test_assembly_interference() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut asm = Assembly::new("Test");
        let c1 = asm.add_component("Box1", b.solid);
        let c2 = asm.add_component("Box2", b.solid);

        // Both at origin — should overlap
        assert!(asm.check_interference(&model, c1, c2).unwrap());

        // Move c2 far away
        asm.set_placement(c2, translation(100.0, 0.0, 0.0)).unwrap();
        assert!(!asm.check_interference(&model, c1, c2).unwrap());
    }

    #[test]
    fn test_assembly_add_joint() {
        let mut asm = Assembly::new("Joint Test");
        assert_eq!(asm.joint_count(), 0);

        asm.add_joint(JointType::Grounded);
        asm.add_joint(JointType::Revolute {
            component_a: 0,
            component_b: 1,
            axis: Vec3::Z,
            origin: Point3::ORIGIN,
        });
        asm.add_joint(JointType::GearJoint {
            component_a: 0,
            component_b: 1,
            ratio: 2.0,
        });
        assert_eq!(asm.joint_count(), 3);
    }

    #[test]
    fn test_assembly_bill_of_materials() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut asm = Assembly::new("BOM Test");
        asm.add_component("Bolt", b.solid);
        asm.add_component("Bolt", b.solid);
        asm.add_component("Nut", b.solid);
        asm.add_component("Bolt", b.solid);
        asm.add_component("Washer", b.solid);

        let bom = asm.bill_of_materials();
        assert_eq!(bom.len(), 3);
        assert_eq!(bom[0].name, "Bolt");
        assert_eq!(bom[0].quantity, 3);
        assert_eq!(bom[1].name, "Nut");
        assert_eq!(bom[1].quantity, 1);
        assert_eq!(bom[2].name, "Washer");
        assert_eq!(bom[2].quantity, 1);
    }

    #[test]
    fn test_assembly_exploded_view() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut asm = Assembly::new("Explode Test");
        let c1 = asm.add_component("Part1", b.solid);
        let c2 = asm.add_component("Part2", b.solid);
        asm.set_placement(c2, translation(10.0, 0.0, 0.0)).unwrap();

        let before_c1_x = asm.get_component(c1).unwrap().placement.0[(0, 3)];
        let before_c2_x = asm.get_component(c2).unwrap().placement.0[(0, 3)];

        asm.exploded_view(1.0);

        let after_c1_x = asm.get_component(c1).unwrap().placement.0[(0, 3)];
        let after_c2_x = asm.get_component(c2).unwrap().placement.0[(0, 3)];

        // Components should have moved apart
        let dist_before = (before_c2_x - before_c1_x).abs();
        let dist_after = (after_c2_x - after_c1_x).abs();
        assert!(dist_after > dist_before, "Components should be further apart after explosion");
    }

    #[test]
    fn test_assembly_visibility() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut asm = Assembly::new("Test");
        let c1 = asm.add_component("Box1", b.solid);

        assert!(asm.get_component(c1).unwrap().visible);
        asm.set_visible(c1, false).unwrap();
        assert!(!asm.get_component(c1).unwrap().visible);
    }

    #[test]
    fn test_dof_analysis_unconstrained() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut asm = Assembly::new("DOF Test");
        asm.add_component("Box1", b.solid);
        asm.add_component("Box2", b.solid);

        let dof = asm.analyze_dof();
        assert_eq!(dof.total_dof, 12);
        assert_eq!(dof.constrained_dof, 0);
        assert_eq!(dof.remaining_dof, 12);
        assert!(!dof.fully_constrained);
    }

    #[test]
    fn test_dof_analysis_fixed() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut asm = Assembly::new("DOF Test");
        let c1 = asm.add_component("Box1", b.solid);
        let c2 = asm.add_component("Box2", b.solid);

        asm.add_constraint(AssemblyConstraint::Fixed(c1));
        asm.add_constraint(AssemblyConstraint::Fixed(c2));

        let dof = asm.analyze_dof();
        assert_eq!(dof.remaining_dof, 0);
        assert!(dof.fully_constrained);
    }

    #[test]
    fn test_dof_analysis_with_joints() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mut asm = Assembly::new("DOF Joint");
        asm.add_component("A", b.solid);
        asm.add_component("B", b.solid);

        asm.add_joint(JointType::Grounded);
        asm.add_joint(JointType::Revolute {
            component_a: 0,
            component_b: 1,
            axis: Vec3::Z,
            origin: Point3::ORIGIN,
        });

        let dof = asm.analyze_dof();
        assert_eq!(dof.constrained_dof, 11);
        assert_eq!(dof.remaining_dof, 1);
    }

    #[test]
    fn test_assembly_solve_distance() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mut asm = Assembly::new("Solve Test");
        let c1 = asm.add_component("A", b.solid);
        let c2 = asm.add_component("B", b.solid);

        asm.add_constraint(AssemblyConstraint::Fixed(c1));
        asm.add_constraint(AssemblyConstraint::Distance {
            comp_a: c1,
            comp_b: c2,
            distance: 10.0,
        });

        asm.set_placement(c2, translation(5.0, 0.0, 0.0)).unwrap();
        let converged = asm.solve(100).unwrap();
        assert!(converged);

        let pos_b = asm.component_position(c2).unwrap();
        let dist = (pos_b.x * pos_b.x + pos_b.y * pos_b.y + pos_b.z * pos_b.z).sqrt();
        assert!((dist - 10.0).abs() < 1e-6, "distance = {dist}");
    }

    #[test]
    fn test_rotation_matrix() {
        let r = rotation(Vec3::Z, std::f64::consts::FRAC_PI_2);
        let p = r.transform_point(Point3::new(1.0, 0.0, 0.0));
        assert!((p.x).abs() < 1e-10, "x = {}", p.x);
        assert!((p.y - 1.0).abs() < 1e-10, "y = {}", p.y);
    }

    #[test]
    fn test_new_joint_types() {
        let mut asm = Assembly::new("Joints");
        asm.add_joint(JointType::RackAndPinion {
            component_a: 0,
            component_b: 1,
            pitch_radius: 5.0,
        });
        asm.add_joint(JointType::ScrewJoint {
            component_a: 0,
            component_b: 1,
            axis: Vec3::Z,
            pitch: 2.0,
        });
        asm.add_joint(JointType::BeltJoint {
            component_a: 0,
            component_b: 1,
            ratio: 1.5,
        });
        assert_eq!(asm.joint_count(), 3);
    }

    #[test]
    fn test_solve_constraints() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mut asm = Assembly::new("Solver Test");
        let c1 = asm.add_component("A", b.solid);
        let c2 = asm.add_component("B", b.solid);

        asm.add_constraint(AssemblyConstraint::Fixed(c1));
        asm.add_constraint(AssemblyConstraint::Distance {
            comp_a: c1,
            comp_b: c2,
            distance: 10.0,
        });

        asm.set_placement(c2, translation(5.0, 0.0, 0.0)).unwrap();
        let transforms = asm.solve_constraints().unwrap();
        assert_eq!(transforms.len(), 2);

        let pos_b = asm.component_position(c2).unwrap();
        let dist = (pos_b.x * pos_b.x + pos_b.y * pos_b.y + pos_b.z * pos_b.z).sqrt();
        assert!((dist - 10.0).abs() < 1e-6, "distance = {dist}");
    }

    #[test]
    fn test_simulate_step() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mut asm = Assembly::new("Sim Test");
        asm.add_component("A", b.solid);
        asm.add_component("B", b.solid);
        asm.set_placement(ComponentId(1), translation(5.0, 0.0, 0.0)).unwrap();

        asm.add_joint(JointType::Slider {
            component_a: 0,
            component_b: 1,
            axis: Vec3::X,
        });

        let before_x = asm.get_component(ComponentId(1)).unwrap().placement.0[(0, 3)];
        asm.simulate_step(0.1).unwrap();
        let after_x = asm.get_component(ComponentId(1)).unwrap().placement.0[(0, 3)];
        assert!(after_x > before_x);
    }

    #[test]
    fn test_simulate_step_invalid() {
        let mut asm = Assembly::new("Test");
        assert!(asm.simulate_step(-1.0).is_err());
    }

    #[test]
    fn test_export_asmt() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mut asm = Assembly::new("Export Test");
        let c1 = asm.add_component("Box1", b.solid);
        let c2 = asm.add_component("Box2", b.solid);
        asm.add_constraint(AssemblyConstraint::Fixed(c1));
        asm.add_constraint(AssemblyConstraint::Distance {
            comp_a: c1,
            comp_b: c2,
            distance: 5.0,
        });

        let text = asm.export_asmt().unwrap();
        assert!(text.contains("Assembly: Export Test"));
        assert!(text.contains("Components: 2"));
        assert!(text.contains("Constraints: 2"));
        assert!(text.contains("Box1"));
    }

    #[test]
    fn test_assembly_preferences_default() {
        let prefs = AssemblyPreferences::default();
        assert_eq!(prefs.max_iterations, 100);
        assert!((prefs.tolerance - 1e-10).abs() < 1e-20);
    }

    #[test]
    fn test_joint_residual_fixed_joint_at_identity() {
        let m = Mat4::IDENTITY;
        let r = joint_residual(&JointType::FixedJoint { component_a: 0, component_b: 1 }, &m, &m);
        assert_eq!(r.len(), 6);
        for v in &r {
            assert!(v.abs() < 1e-10);
        }
    }

    #[test]
    fn test_joint_residual_revolute() {
        let m = Mat4::IDENTITY;
        let r = joint_residual(
            &JointType::Revolute {
                component_a: 0,
                component_b: 1,
                axis: Vec3::Z,
                origin: Point3::ORIGIN,
            },
            &m,
            &m,
        );
        assert_eq!(r.len(), 5);
        for v in &r {
            assert!(v.abs() < 1e-10);
        }
    }

    #[test]
    fn test_joint_residual_ball() {
        let m = Mat4::IDENTITY;
        let r = joint_residual(
            &JointType::BallJoint {
                component_a: 0,
                component_b: 1,
                center: Point3::ORIGIN,
            },
            &m,
            &m,
        );
        assert_eq!(r.len(), 3);
        for v in &r {
            assert!(v.abs() < 1e-10);
        }
    }

    #[test]
    fn test_joint_residual_parallel_axes() {
        let r = joint_residual(
            &JointType::ParallelAxes {
                component_a: 0,
                component_b: 1,
                axis_a: Vec3::Z,
                axis_b: Vec3::Z,
            },
            &Mat4::IDENTITY,
            &Mat4::IDENTITY,
        );
        assert_eq!(r.len(), 2);
        for v in &r {
            assert!(v.abs() < 1e-10);
        }
    }

    #[test]
    fn test_joint_residual_perpendicular_axes() {
        let r = joint_residual(
            &JointType::PerpendicularAxes {
                component_a: 0,
                component_b: 1,
                axis_a: Vec3::X,
                axis_b: Vec3::Z,
            },
            &Mat4::IDENTITY,
            &Mat4::IDENTITY,
        );
        assert_eq!(r.len(), 1);
        assert!(r[0].abs() < 1e-10);
    }

    #[test]
    fn test_joint_residual_slider() {
        let m = Mat4::IDENTITY;
        let r = joint_residual(
            &JointType::Slider {
                component_a: 0,
                component_b: 1,
                axis: Vec3::X,
            },
            &m,
            &m,
        );
        assert_eq!(r.len(), 5);
    }

    #[test]
    fn test_joint_residual_cylindrical() {
        let m = Mat4::IDENTITY;
        let r = joint_residual(
            &JointType::Cylindrical {
                component_a: 0,
                component_b: 1,
                axis: Vec3::Z,
                origin: Point3::ORIGIN,
            },
            &m,
            &m,
        );
        assert_eq!(r.len(), 4);
    }

    #[test]
    fn test_joint_jacobian_shape() {
        let m = Mat4::IDENTITY;
        let j = joint_jacobian(
            &JointType::BallJoint {
                component_a: 0,
                component_b: 1,
                center: Point3::ORIGIN,
            },
            &m,
            &m,
        );
        assert_eq!(j.len(), 3);
        for row in &j {
            assert_eq!(row.len(), 6);
        }
    }

    #[test]
    fn test_new_part_in_assembly() {
        let mut asm = Assembly::new("Test");
        let idx = new_part_in_assembly(&mut asm, "EmptyPart").unwrap();
        assert_eq!(idx, 0);
        assert_eq!(asm.num_components(), 1);
        assert_eq!(asm.get_component(ComponentId(idx)).unwrap().name, "EmptyPart");
    }

    #[test]
    fn test_new_part_empty_name() {
        let mut asm = Assembly::new("Test");
        assert!(new_part_in_assembly(&mut asm, "").is_err());
    }

    #[test]
    fn test_kinematic_simulation_basic() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mut asm = Assembly::new("Sim");
        asm.add_component("A", b.solid);
        asm.add_component("B", b.solid);
        asm.set_placement(ComponentId(1), translation(5.0, 0.0, 0.0)).unwrap();
        asm.add_joint(JointType::Slider {
            component_a: 0,
            component_b: 1,
            axis: Vec3::X,
        });

        let traj = kinematic_simulation(&asm, 1.0, 10).unwrap();
        assert_eq!(traj.len(), 11);
        assert_eq!(traj[0].len(), 2);
    }

    #[test]
    fn test_kinematic_simulation_invalid() {
        let asm = Assembly::new("Test");
        assert!(kinematic_simulation(&asm, -1.0, 10).is_err());
        assert!(kinematic_simulation(&asm, 1.0, 0).is_err());
    }

    #[test]
    fn test_joint_residual_gear() {
        let r = joint_residual(
            &JointType::GearJoint {
                component_a: 0,
                component_b: 1,
                ratio: 2.0,
            },
            &Mat4::IDENTITY,
            &Mat4::IDENTITY,
        );
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn test_joint_residual_screw() {
        let r = joint_residual(
            &JointType::ScrewJoint {
                component_a: 0,
                component_b: 1,
                axis: Vec3::Z,
                pitch: 1.0,
            },
            &Mat4::IDENTITY,
            &Mat4::IDENTITY,
        );
        assert_eq!(r.len(), 5);
    }

    #[test]
    fn test_joint_residual_belt() {
        let r = joint_residual(
            &JointType::BeltJoint {
                component_a: 0,
                component_b: 1,
                ratio: 1.0,
            },
            &Mat4::IDENTITY,
            &Mat4::IDENTITY,
        );
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn test_joint_residual_rack_and_pinion() {
        let r = joint_residual(
            &JointType::RackAndPinion {
                component_a: 0,
                component_b: 1,
                pitch_radius: 5.0,
            },
            &Mat4::IDENTITY,
            &Mat4::IDENTITY,
        );
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn test_assembly_dof_empty() {
        let assembly = Assembly::new("empty");
        let dof = assembly.analyze_dof();
        assert_eq!(dof.total_dof, 0);
        assert_eq!(dof.remaining_dof, 0);
        assert!(dof.fully_constrained);
        assert!(!dof.over_constrained);
    }

    #[test]
    fn test_assembly_dof_single_unconstrained() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let mut assembly = Assembly::new("test");
        assembly.add_component("part", b.solid);
        let dof = assembly.analyze_dof();
        assert_eq!(dof.total_dof, 6);
        assert_eq!(dof.remaining_dof, 6);
        assert!(!dof.fully_constrained);
    }

    #[test]
    fn test_assembly_dof_fixed_component() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let mut assembly = Assembly::new("test");
        let id = assembly.add_component("part", b.solid);
        assembly.add_constraint(AssemblyConstraint::Fixed(id));
        let dof = assembly.analyze_dof();
        assert_eq!(dof.constrained_dof, 6);
        assert_eq!(dof.remaining_dof, 0);
        assert!(dof.fully_constrained);
    }

    #[test]
    fn test_assembly_solve_fixed_not_moved() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let mut assembly = Assembly::new("test");
        let id = assembly.add_component("part", b.solid);
        assembly.add_constraint(AssemblyConstraint::Fixed(id));
        let converged = assembly.solve(100).unwrap();
        let pos = assembly.component_position(id).unwrap();
        assert!(pos.x.abs() < 1e-10);
        assert!(pos.y.abs() < 1e-10);
        assert!(pos.z.abs() < 1e-10);
        let _ = converged;
    }

    #[test]
    fn test_assembly_exploded_view_empty() {
        let mut assembly = Assembly::new("empty");
        assembly.exploded_view(2.0);
    }

    #[test]
    fn test_assembly_bom_grouping() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let mut assembly = Assembly::new("test");
        assembly.add_component("bolt", b.solid);
        assembly.add_component("bolt", b.solid);
        assembly.add_component("nut", b.solid);
        let bom = assembly.bill_of_materials();
        assert_eq!(bom.len(), 2);
        let bolt_entry = bom.iter().find(|e| e.name == "bolt").unwrap();
        assert_eq!(bolt_entry.quantity, 2);
    }

    #[test]
    fn test_assembly_visibility_toggle() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let mut assembly = Assembly::new("test");
        let id = assembly.add_component("part", b.solid);
        assembly.set_visible(id, false).unwrap();
        let comp = assembly.get_component(id).unwrap();
        assert!(!comp.visible);
        assembly.set_visible(id, true).unwrap();
        let comp = assembly.get_component(id).unwrap();
        assert!(comp.visible);
    }

    #[test]
    fn test_assembly_transform_point() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let mut assembly = Assembly::new("test");
        let id = assembly.add_component("part", b.solid);
        let tx = translation(5.0, 0.0, 0.0);
        assembly.set_placement(id, tx).unwrap();
        let pt = assembly.transform_point(id, Point3::ORIGIN).unwrap();
        assert!((pt.x - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_assembly_dof_two_components_distance() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let mut assembly = Assembly::new("test");
        let id_a = assembly.add_component("a", b.solid);
        let id_b = assembly.add_component("b", b.solid);
        assembly.add_constraint(AssemblyConstraint::Distance {
            comp_a: id_a,
            comp_b: id_b,
            distance: 5.0,
        });
        let dof = assembly.analyze_dof();
        assert_eq!(dof.constrained_dof, 1);
    }

    #[test]
    fn test_assembly_add_joints() {
        let mut assembly = Assembly::new("test");
        assembly.add_joint(JointType::Grounded);
        assembly.add_joint(JointType::FixedJoint { component_a: 0, component_b: 1 });
        assert_eq!(assembly.joint_count(), 2);
    }

    #[test]
    fn test_assembly_solve_distance_constraint() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let mut assembly = Assembly::new("test");
        let id_a = assembly.add_component("a", b.solid);
        let id_b = assembly.add_component("b", b.solid);
        assembly.add_constraint(AssemblyConstraint::Fixed(id_a));
        let tx = translation(8.0, 0.0, 0.0);
        assembly.set_placement(id_b, tx).unwrap();
        assembly.add_constraint(AssemblyConstraint::Distance {
            comp_a: id_a,
            comp_b: id_b,
            distance: 5.0,
        });
        let _ = assembly.solve(50).unwrap();
        let pos_b = assembly.component_position(id_b).unwrap();
        assert!(pos_b.x > 0.0, "b should have moved along x");
    }

    #[test]
    fn test_check_all_interferences_overlapping() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut asm = Assembly::new("BVH Interference");
        let c0 = asm.add_component("A", b.solid);
        let c1 = asm.add_component("B", b.solid);
        let c2 = asm.add_component("C", b.solid);

        // c0 and c1 overlap at origin; c2 far away
        asm.set_placement(c2, translation(100.0, 0.0, 0.0)).unwrap();

        let pairs = asm.check_all_interferences(&model).unwrap();
        assert!(pairs.contains(&(c0, c1)), "c0-c1 should overlap");
        assert!(!pairs.iter().any(|&(a, b)| (a == c0 && b == c2) || (a == c2 && b == c0)),
            "c0-c2 should not overlap");
    }

    #[test]
    fn test_check_all_interferences_empty() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mut asm = Assembly::new("Empty BVH");
        asm.add_component("A", b.solid);
        // Single component: no pairs
        let pairs = asm.check_all_interferences(&model).unwrap();
        assert!(pairs.is_empty());
    }

    #[test]
    fn test_large_assembly_500_parts() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mut asm = Assembly::new("Large Assembly");
        let n = 500;
        for i in 0..n {
            let id = asm.add_component(&format!("Part_{i}"), b.solid);
            // Place on a grid so most components don't overlap
            let x = (i % 50) as f64 * 5.0;
            let y = (i / 50) as f64 * 5.0;
            asm.set_placement(id, translation(x, y, 0.0)).unwrap();
        }

        assert_eq!(asm.num_components(), n);

        // BVH interference check should complete quickly
        let pairs = asm.check_all_interferences(&model).unwrap();
        // On a 5.0 spacing grid with 1.0 boxes, no overlaps
        assert!(pairs.is_empty(), "grid-spaced parts should not overlap");

        // BOM should group all unique names
        let bom = asm.bill_of_materials();
        assert_eq!(bom.len(), n); // all unique names

        // Component lookup by ID should work
        let last_id = ComponentId(n - 1);
        let comp = asm.get_component(last_id);
        assert!(comp.is_some());
        assert!(comp.unwrap().name.starts_with("Part_"));
    }

    #[test]
    fn test_large_assembly_bvh_finds_overlaps() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut asm = Assembly::new("Overlap Test");
        // Place 100 parts, every pair at same position overlaps
        for i in 0..100 {
            let id = asm.add_component(&format!("Part_{i}"), b.solid);
            let x = (i % 10) as f64 * 1.0; // 1.0 spacing < 2.0 size = overlaps
            let y = (i / 10) as f64 * 1.0;
            asm.set_placement(id, translation(x, y, 0.0)).unwrap();
        }

        let pairs = asm.check_all_interferences(&model).unwrap();
        // Adjacent parts should overlap (spacing 1.0 < size 2.0)
        assert!(!pairs.is_empty(), "adjacent parts with 1.0 spacing and 2.0 size should overlap");
    }
}
