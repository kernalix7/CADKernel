#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod dof;
pub mod mate;
pub mod motion;
pub mod solver;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub use dof::DofAnalysis;
pub use mate::{AxisRef, CurveRef, EdgeRef, FaceRef, Mate};
pub use motion::MotionReport;
pub use solver::{SolveReport, SolverOptions};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct AssemblyId(pub u64);

impl std::fmt::Display for AssemblyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "assembly#{}", self.0)
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct ComponentId(pub u64);

impl std::fmt::Display for ComponentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "component#{}", self.0)
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct MateId(pub u64);

impl std::fmt::Display for MateId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mate#{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RigidTransform {
    pub translation: [f64; 3],
    pub rotation: [f64; 3],
}

impl Default for RigidTransform {
    fn default() -> Self {
        Self::identity()
    }
}

impl RigidTransform {
    pub const fn identity() -> Self {
        Self {
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        }
    }

    pub const fn translated(x: f64, y: f64, z: f64) -> Self {
        Self {
            translation: [x, y, z],
            rotation: [0.0, 0.0, 0.0],
        }
    }

    pub fn validate(&self) -> KernelResult<()> {
        validate_vec3(self.translation, "translation")?;
        validate_vec3(self.rotation, "rotation")
    }

    pub(crate) fn translation_vec(self) -> Vec3 {
        Vec3::from(self.translation)
    }

    pub(crate) fn set_translation_vec(&mut self, value: Vec3) {
        self.translation = [value.x, value.y, value.z];
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Component {
    pub id: ComponentId,
    pub name: String,
    pub source_solid: Option<u64>,
    pub placement: RigidTransform,
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Assembly {
    pub name: String,
    pub components: Vec<Option<Component>>,
    pub mates: Vec<Option<Mate>>,
    next_component_id: u64,
    next_mate_id: u64,
    #[serde(default)]
    lock_targets: BTreeMap<MateId, [f64; 3]>,
}

impl Default for Assembly {
    fn default() -> Self {
        Self::new("Assembly")
    }
}

impl Assembly {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            components: Vec::new(),
            mates: Vec::new(),
            next_component_id: 0,
            next_mate_id: 0,
            lock_targets: BTreeMap::new(),
        }
    }

    pub fn component_count(&self) -> usize {
        self.components
            .iter()
            .filter(|component| component.is_some())
            .count()
    }

    pub fn mate_count(&self) -> usize {
        self.mates.iter().filter(|mate| mate.is_some()).count()
    }

    pub fn component_ids(&self) -> Vec<ComponentId> {
        self.components
            .iter()
            .enumerate()
            .filter_map(|(idx, component)| component.as_ref().map(|_| ComponentId(idx as u64)))
            .collect()
    }

    pub fn mate_ids(&self) -> Vec<MateId> {
        self.mates
            .iter()
            .enumerate()
            .filter_map(|(idx, mate)| mate.as_ref().map(|_| MateId(idx as u64)))
            .collect()
    }

    pub fn component(&self, id: ComponentId) -> Option<&Component> {
        self.components
            .get(id.0 as usize)
            .and_then(|component| component.as_ref())
    }

    pub fn component_mut(&mut self, id: ComponentId) -> Option<&mut Component> {
        self.components
            .get_mut(id.0 as usize)
            .and_then(|component| component.as_mut())
    }

    pub fn mate(&self, id: MateId) -> Option<&Mate> {
        self.mates.get(id.0 as usize).and_then(|mate| mate.as_ref())
    }

    pub fn add_component(&mut self, name: impl Into<String>) -> KernelResult<ComponentId> {
        self.add_component_with_placement(name, None, RigidTransform::identity())
    }

    pub fn add_component_from_solid(
        &mut self,
        name: impl Into<String>,
        source_solid: u64,
    ) -> KernelResult<ComponentId> {
        self.add_component_with_placement(name, Some(source_solid), RigidTransform::identity())
    }

    pub fn add_component_with_placement(
        &mut self,
        name: impl Into<String>,
        source_solid: Option<u64>,
        placement: RigidTransform,
    ) -> KernelResult<ComponentId> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(KernelError::InvalidArgument(
                "component name must not be empty".into(),
            ));
        }
        placement.validate()?;

        let id = ComponentId(self.next_component_id);
        self.next_component_id += 1;
        let idx = id.0 as usize;
        if self.components.len() <= idx {
            self.components.resize_with(idx + 1, || None);
        }
        self.components[idx] = Some(Component {
            id,
            name,
            source_solid,
            placement,
            visible: true,
        });
        Ok(id)
    }

    pub fn set_component_placement(
        &mut self,
        id: ComponentId,
        placement: RigidTransform,
    ) -> KernelResult<()> {
        placement.validate()?;
        let component = self
            .component_mut(id)
            .ok_or_else(|| KernelError::InvalidArgument(format!("unknown component {id}")))?;
        component.placement = placement;
        Ok(())
    }

    pub fn translate_component(&mut self, id: ComponentId, delta: [f64; 3]) -> KernelResult<()> {
        validate_vec3(delta, "delta")?;
        let component = self
            .component_mut(id)
            .ok_or_else(|| KernelError::InvalidArgument(format!("unknown component {id}")))?;
        for (axis, value) in delta.iter().enumerate() {
            component.placement.translation[axis] += value;
        }
        Ok(())
    }

    pub fn add_mate(&mut self, mate: Mate) -> KernelResult<MateId> {
        mate.validate()?;
        for component in mate.component_ids() {
            self.require_component(component)?;
        }

        let id = MateId(self.next_mate_id);
        self.next_mate_id += 1;
        let idx = id.0 as usize;
        if self.mates.len() <= idx {
            self.mates.resize_with(idx + 1, || None);
        }
        if let Mate::Lock { component } = &mate {
            let target = self.require_component(*component)?.placement.translation;
            self.lock_targets.insert(id, target);
        }
        self.mates[idx] = Some(mate);
        Ok(id)
    }

    pub fn remove_mate(&mut self, id: MateId) -> KernelResult<Mate> {
        let slot = self
            .mates
            .get_mut(id.0 as usize)
            .ok_or_else(|| KernelError::InvalidArgument(format!("unknown mate {id}")))?;
        let mate = slot
            .take()
            .ok_or_else(|| KernelError::InvalidArgument(format!("unknown mate {id}")))?;
        self.lock_targets.remove(&id);
        Ok(mate)
    }

    pub fn dof_count(&self) -> i32 {
        dof::dof_count(self)
    }

    pub fn analyze_dof(&self) -> DofAnalysis {
        dof::analyze(self)
    }

    pub fn solve(&mut self, options: SolverOptions) -> KernelResult<SolveReport> {
        solver::solve(self, options)
    }

    pub fn drag_component(
        &mut self,
        component_id: ComponentId,
        screen_delta: [f64; 2],
    ) -> KernelResult<MotionReport> {
        motion::drag_component(self, component_id, screen_delta)
    }

    pub(crate) fn active_mates(&self) -> impl Iterator<Item = (MateId, &Mate)> {
        self.mates
            .iter()
            .enumerate()
            .filter_map(|(idx, mate)| mate.as_ref().map(|mate| (MateId(idx as u64), mate)))
    }

    pub(crate) fn lock_target(&self, id: MateId) -> Option<[f64; 3]> {
        self.lock_targets.get(&id).copied()
    }

    pub(crate) fn require_component(&self, id: ComponentId) -> KernelResult<&Component> {
        self.component(id)
            .ok_or_else(|| KernelError::InvalidArgument(format!("unknown component {id}")))
    }
}

pub(crate) fn validate_vec3(value: [f64; 3], label: &str) -> KernelResult<()> {
    for component in value {
        if !component.is_finite() {
            return Err(KernelError::InvalidArgument(format!(
                "{label} must be finite"
            )));
        }
    }
    Ok(())
}

pub(crate) fn normalized(value: [f64; 3], label: &str) -> KernelResult<Vec3> {
    validate_vec3(value, label)?;
    Vec3::from(value)
        .normalized()
        .ok_or_else(|| KernelError::InvalidArgument(format!("{label} must be non-zero")))
}
