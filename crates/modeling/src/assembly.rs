//! Assembly module — component tree and assembly constraints.
//!
//! An assembly is a collection of components (solids with placement transforms)
//! connected by constraints (Fixed, Coincident, Concentric, Distance, Angle).

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Mat4, Point3, Vec3};
use cadkernel_topology::{BRepModel, Handle, SolidData};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

/// Unique identifier for a component within an assembly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComponentId(pub usize);

/// A component in the assembly — a solid with a placement transform.
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

/// Assembly constraint types.
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
#[derive(Debug, Clone)]
pub struct Assembly {
    pub name: String,
    pub components: Vec<Component>,
    pub constraints: Vec<AssemblyConstraint>,
    pub joints: Vec<JointType>,
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
        self.components.push(Component {
            id,
            name: name.into(),
            solid,
            placement: Mat4::IDENTITY,
            visible: true,
        });
        id
    }

    /// Sets the placement transform of a component.
    pub fn set_placement(
        &mut self,
        id: ComponentId,
        placement: Mat4,
    ) -> KernelResult<()> {
        let comp = self
            .components
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or(KernelError::InvalidArgument(
                "component not found".into(),
            ))?;
        comp.placement = placement;
        Ok(())
    }

    /// Sets visibility of a component.
    pub fn set_visible(&mut self, id: ComponentId, visible: bool) -> KernelResult<()> {
        let comp = self
            .components
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or(KernelError::InvalidArgument(
                "component not found".into(),
            ))?;
        comp.visible = visible;
        Ok(())
    }

    /// Adds a constraint to the assembly.
    pub fn add_constraint(&mut self, constraint: AssemblyConstraint) {
        self.constraints.push(constraint);
    }

    /// Returns the component by ID.
    pub fn get_component(&self, id: ComponentId) -> Option<&Component> {
        self.components.iter().find(|c| c.id == id)
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

    /// Generates a simple Bill of Materials.
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
}
