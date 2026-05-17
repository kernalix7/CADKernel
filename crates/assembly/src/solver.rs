use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::Vec3;
use nalgebra::DVector;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::mate::{AxisRef, FaceRef};
use crate::{Assembly, ComponentId, Mate, MateId, normalized};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SolverOptions {
    pub max_iterations: usize,
    pub tolerance: f64,
}

impl Default for SolverOptions {
    fn default() -> Self {
        Self {
            max_iterations: 64,
            tolerance: 1.0e-9,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveReport {
    pub converged: bool,
    pub iterations: usize,
    pub residual_norm: f64,
    pub blocks: usize,
}

#[derive(Debug, Clone, Copy)]
struct SparseEntry {
    row: usize,
    col: usize,
    value: f64,
}

#[derive(Debug, Clone)]
struct SparseJacobian {
    rows: usize,
    cols: usize,
    entries: Vec<SparseEntry>,
}

impl SparseJacobian {
    fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            entries: Vec::new(),
        }
    }

    fn push(&mut self, row: usize, col: usize, value: f64) {
        if row < self.rows && col < self.cols && value != 0.0 {
            self.entries.push(SparseEntry { row, col, value });
        }
    }

    fn diagonal_gauss_newton_step(&self, residual: &DVector<f64>) -> Vec<f64> {
        let mut gradient = vec![0.0; self.cols];
        let mut diagonal = vec![1.0e-12; self.cols];
        for entry in &self.entries {
            let r = residual.get(entry.row).copied().unwrap_or(0.0);
            gradient[entry.col] += entry.value * r;
            diagonal[entry.col] += entry.value * entry.value;
        }
        gradient
            .into_iter()
            .zip(diagonal)
            .map(|(g, d)| -g / d)
            .collect()
    }
}

#[derive(Debug, Clone)]
struct ResidualSystem {
    residual: DVector<f64>,
    jacobian: SparseJacobian,
}

pub fn solve(assembly: &mut Assembly, options: SolverOptions) -> KernelResult<SolveReport> {
    if !options.tolerance.is_finite() || options.tolerance <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "solver tolerance must be positive and finite".into(),
        ));
    }

    let blocks = decompose_blocks(assembly);
    let mut total_iterations = 0usize;
    let mut max_residual = 0.0_f64;
    let mut converged = true;

    for block in &blocks {
        let report = solve_block(assembly, block, options)?;
        total_iterations = total_iterations.max(report.iterations);
        max_residual = max_residual.max(report.residual_norm);
        converged &= report.converged;
    }

    clamp_single_dof_ranges(assembly)?;

    Ok(SolveReport {
        converged,
        iterations: total_iterations,
        residual_norm: max_residual,
        blocks: blocks.len(),
    })
}

fn solve_block(
    assembly: &mut Assembly,
    block: &[ComponentId],
    options: SolverOptions,
) -> KernelResult<SolveReport> {
    if block.is_empty() {
        return Ok(SolveReport {
            converged: true,
            iterations: 0,
            residual_norm: 0.0,
            blocks: 1,
        });
    }

    let block_set = block.iter().copied().collect::<BTreeSet<_>>();
    let block_mates = assembly
        .active_mates()
        .filter(|(_, mate)| {
            mate.component_ids()
                .into_iter()
                .any(|component| block_set.contains(&component))
        })
        .map(|(id, mate)| (id, mate.clone()))
        .collect::<Vec<_>>();
    if block_mates.is_empty() {
        return Ok(SolveReport {
            converged: true,
            iterations: 0,
            residual_norm: 0.0,
            blocks: 1,
        });
    }

    let locked = locked_components(&block_mates);
    let variable_map = variable_map(block, &locked);
    let mut positions = collect_positions(assembly, block)?;
    let mut best_norm = residual_norm(&block_mates, &positions, assembly)?;
    if best_norm <= options.tolerance {
        return Ok(SolveReport {
            converged: true,
            iterations: 0,
            residual_norm: best_norm,
            blocks: 1,
        });
    }

    let mut iterations = 0usize;
    for iteration in 0..options.max_iterations {
        iterations = iteration + 1;
        let system = build_system(&block_mates, &positions, assembly, &variable_map)?;
        let norm = system.residual.norm();
        best_norm = norm;
        if norm <= options.tolerance {
            break;
        }

        let step = system.jacobian.diagonal_gauss_newton_step(&system.residual);
        if step.iter().all(|value| value.abs() < 1.0e-14) {
            break;
        }

        let Some((trial, trial_norm)) = line_search(
            &block_mates,
            &positions,
            &step,
            &variable_map,
            assembly,
            norm,
        )?
        else {
            break;
        };
        positions = trial;
        best_norm = trial_norm;
        if best_norm <= options.tolerance {
            break;
        }
    }

    apply_positions(assembly, &positions)?;

    Ok(SolveReport {
        converged: best_norm <= options.tolerance,
        iterations,
        residual_norm: best_norm,
        blocks: 1,
    })
}

fn decompose_blocks(assembly: &Assembly) -> Vec<Vec<ComponentId>> {
    let components = assembly.component_ids();
    let mut parent = components
        .iter()
        .map(|id| (*id, *id))
        .collect::<BTreeMap<_, _>>();

    for (_, mate) in assembly.active_mates() {
        let ids = mate.component_ids();
        let Some(first) = ids.first().copied() else {
            continue;
        };
        for other in ids.into_iter().skip(1) {
            union(&mut parent, first, other);
        }
    }

    let mut groups: BTreeMap<ComponentId, Vec<ComponentId>> = BTreeMap::new();
    for id in components {
        let root = find(&mut parent, id);
        groups.entry(root).or_default().push(id);
    }
    groups.into_values().collect()
}

fn find(parent: &mut BTreeMap<ComponentId, ComponentId>, id: ComponentId) -> ComponentId {
    let current = parent.get(&id).copied().unwrap_or(id);
    if current == id {
        return id;
    }
    let root = find(parent, current);
    parent.insert(id, root);
    root
}

fn union(parent: &mut BTreeMap<ComponentId, ComponentId>, a: ComponentId, b: ComponentId) {
    let ra = find(parent, a);
    let rb = find(parent, b);
    if ra != rb {
        parent.insert(rb, ra);
    }
}

fn locked_components(mates: &[(MateId, Mate)]) -> BTreeSet<ComponentId> {
    mates
        .iter()
        .filter_map(|(_, mate)| match mate {
            Mate::Lock { component } => Some(*component),
            _ => None,
        })
        .collect()
}

fn variable_map(
    block: &[ComponentId],
    locked: &BTreeSet<ComponentId>,
) -> BTreeMap<ComponentId, usize> {
    block
        .iter()
        .filter(|id| !locked.contains(id))
        .enumerate()
        .map(|(idx, id)| (*id, idx * 3))
        .collect()
}

fn collect_positions(
    assembly: &Assembly,
    block: &[ComponentId],
) -> KernelResult<BTreeMap<ComponentId, Vec3>> {
    let mut positions = BTreeMap::new();
    for id in block {
        let component = assembly.require_component(*id)?;
        positions.insert(*id, component.placement.translation_vec());
    }
    Ok(positions)
}

fn apply_positions(
    assembly: &mut Assembly,
    positions: &BTreeMap<ComponentId, Vec3>,
) -> KernelResult<()> {
    for (id, position) in positions {
        let component = assembly
            .component_mut(*id)
            .ok_or_else(|| KernelError::InvalidArgument(format!("unknown component {id}")))?;
        component.placement.set_translation_vec(*position);
    }
    Ok(())
}

fn build_system(
    mates: &[(MateId, Mate)],
    positions: &BTreeMap<ComponentId, Vec3>,
    assembly: &Assembly,
    variable_map: &BTreeMap<ComponentId, usize>,
) -> KernelResult<ResidualSystem> {
    let mut residuals = Vec::new();
    let mut triplets: Vec<(usize, ComponentId, usize, f64)> = Vec::new();
    for (mate_id, mate) in mates {
        append_mate_residual(
            *mate_id,
            mate,
            positions,
            assembly,
            &mut residuals,
            &mut triplets,
        )?;
    }

    let mut jacobian = SparseJacobian::new(residuals.len(), variable_map.len() * 3);
    for (row, component, axis, value) in triplets {
        if let Some(base) = variable_map.get(&component) {
            jacobian.push(row, base + axis, value);
        }
    }

    Ok(ResidualSystem {
        residual: DVector::from_vec(residuals),
        jacobian,
    })
}

fn append_mate_residual(
    mate_id: MateId,
    mate: &Mate,
    positions: &BTreeMap<ComponentId, Vec3>,
    assembly: &Assembly,
    residuals: &mut Vec<f64>,
    triplets: &mut Vec<(usize, ComponentId, usize, f64)>,
) -> KernelResult<()> {
    match mate {
        Mate::Coincident { face_a, face_b } => {
            append_point_coincident(face_a, face_b, positions, residuals, triplets)
        }
        Mate::Distance {
            face_a,
            face_b,
            distance,
        } => append_distance(face_a, face_b, *distance, positions, residuals, triplets),
        Mate::Concentric { axis_a, axis_b } => {
            append_axis_coincident(axis_a, axis_b, positions, residuals, triplets)
        }
        Mate::Lock { component } => {
            let target = assembly.lock_target(mate_id).unwrap_or([0.0, 0.0, 0.0]);
            append_lock(*component, target, positions, residuals, triplets)
        }
        _ => Ok(()),
    }
}

fn append_point_coincident(
    face_a: &FaceRef,
    face_b: &FaceRef,
    positions: &BTreeMap<ComponentId, Vec3>,
    residuals: &mut Vec<f64>,
    triplets: &mut Vec<(usize, ComponentId, usize, f64)>,
) -> KernelResult<()> {
    let pa = ref_point(face_a.component, face_a.point, positions)?;
    let pb = ref_point(face_b.component, face_b.point, positions)?;
    let row = residuals.len();
    let delta = pb - pa;
    residuals.extend([delta.x, delta.y, delta.z]);
    for axis in 0..3 {
        triplets.push((row + axis, face_a.component, axis, -1.0));
        triplets.push((row + axis, face_b.component, axis, 1.0));
    }
    Ok(())
}

fn append_axis_coincident(
    axis_a: &AxisRef,
    axis_b: &AxisRef,
    positions: &BTreeMap<ComponentId, Vec3>,
    residuals: &mut Vec<f64>,
    triplets: &mut Vec<(usize, ComponentId, usize, f64)>,
) -> KernelResult<()> {
    let pa = ref_point(axis_a.component, axis_a.origin, positions)?;
    let pb = ref_point(axis_b.component, axis_b.origin, positions)?;
    let row = residuals.len();
    let delta = pb - pa;
    residuals.extend([delta.x, delta.y, delta.z]);
    for axis in 0..3 {
        triplets.push((row + axis, axis_a.component, axis, -1.0));
        triplets.push((row + axis, axis_b.component, axis, 1.0));
    }
    Ok(())
}

fn append_distance(
    face_a: &FaceRef,
    face_b: &FaceRef,
    distance: f64,
    positions: &BTreeMap<ComponentId, Vec3>,
    residuals: &mut Vec<f64>,
    triplets: &mut Vec<(usize, ComponentId, usize, f64)>,
) -> KernelResult<()> {
    let pa = ref_point(face_a.component, face_a.point, positions)?;
    let pb = ref_point(face_b.component, face_b.point, positions)?;
    let delta = pb - pa;
    let len = delta.length();
    if len <= 1.0e-12 {
        residuals.push(-distance);
        return Ok(());
    }

    let unit = delta / len;
    let row = residuals.len();
    residuals.push(len - distance);
    for (axis, value) in [unit.x, unit.y, unit.z].into_iter().enumerate() {
        triplets.push((row, face_a.component, axis, -value));
        triplets.push((row, face_b.component, axis, value));
    }
    Ok(())
}

fn append_lock(
    component: ComponentId,
    target: [f64; 3],
    positions: &BTreeMap<ComponentId, Vec3>,
    residuals: &mut Vec<f64>,
    triplets: &mut Vec<(usize, ComponentId, usize, f64)>,
) -> KernelResult<()> {
    let pos = positions
        .get(&component)
        .copied()
        .ok_or_else(|| KernelError::InvalidArgument(format!("unknown component {component}")))?;
    let target = Vec3::from(target);
    let row = residuals.len();
    let delta = pos - target;
    residuals.extend([delta.x, delta.y, delta.z]);
    for axis in 0..3 {
        triplets.push((row + axis, component, axis, 1.0));
    }
    Ok(())
}

fn ref_point(
    component: ComponentId,
    local: [f64; 3],
    positions: &BTreeMap<ComponentId, Vec3>,
) -> KernelResult<Vec3> {
    let base = positions
        .get(&component)
        .copied()
        .ok_or_else(|| KernelError::InvalidArgument(format!("unknown component {component}")))?;
    Ok(base + Vec3::from(local))
}

fn residual_norm(
    mates: &[(MateId, Mate)],
    positions: &BTreeMap<ComponentId, Vec3>,
    assembly: &Assembly,
) -> KernelResult<f64> {
    let mut residuals = Vec::new();
    let mut triplets = Vec::new();
    for (mate_id, mate) in mates {
        append_mate_residual(
            *mate_id,
            mate,
            positions,
            assembly,
            &mut residuals,
            &mut triplets,
        )?;
    }
    Ok(DVector::from_vec(residuals).norm())
}

fn line_search(
    mates: &[(MateId, Mate)],
    positions: &BTreeMap<ComponentId, Vec3>,
    step: &[f64],
    variable_map: &BTreeMap<ComponentId, usize>,
    assembly: &Assembly,
    current_norm: f64,
) -> KernelResult<Option<(BTreeMap<ComponentId, Vec3>, f64)>> {
    let mut alpha = 1.0;
    let mut best: Option<(BTreeMap<ComponentId, Vec3>, f64)> = None;
    for _ in 0..12 {
        let trial = trial_positions(positions, step, variable_map, alpha);
        let norm = residual_norm(mates, &trial, assembly)?;
        if norm < current_norm {
            best = Some((trial, norm));
            break;
        }
        alpha *= 0.5;
    }
    Ok(best)
}

fn trial_positions(
    positions: &BTreeMap<ComponentId, Vec3>,
    step: &[f64],
    variable_map: &BTreeMap<ComponentId, usize>,
    alpha: f64,
) -> BTreeMap<ComponentId, Vec3> {
    let mut out = positions.clone();
    for (component, base) in variable_map {
        let dx = step.get(*base).copied().unwrap_or(0.0);
        let dy = step.get(*base + 1).copied().unwrap_or(0.0);
        let dz = step.get(*base + 2).copied().unwrap_or(0.0);
        if let Some(position) = out.get_mut(component) {
            *position += Vec3::new(dx * alpha, dy * alpha, dz * alpha);
        }
    }
    out
}

pub(crate) fn clamp_single_dof_ranges(assembly: &mut Assembly) -> KernelResult<()> {
    let mates = assembly
        .active_mates()
        .map(|(_, mate)| mate.clone())
        .collect::<Vec<_>>();
    for mate in mates {
        match mate {
            Mate::Slider {
                axis,
                range_min,
                range_max,
            } => clamp_axis_translation(assembly, &axis, range_min, range_max)?,
            Mate::Hinge {
                axis,
                range_min,
                range_max,
            } => clamp_axis_rotation(assembly, axis.component, range_min, range_max)?,
            _ => {}
        }
    }
    Ok(())
}

fn clamp_axis_translation(
    assembly: &mut Assembly,
    axis: &AxisRef,
    range_min: f64,
    range_max: f64,
) -> KernelResult<()> {
    let dir = normalized(axis.direction, "axis direction")?;
    let origin = Vec3::from(axis.origin);
    let component = assembly.component_mut(axis.component).ok_or_else(|| {
        KernelError::InvalidArgument(format!("unknown component {}", axis.component))
    })?;
    let pos = component.placement.translation_vec();
    let offset = pos - origin;
    let current = offset.dot(dir);
    let clamped = current.clamp(range_min, range_max);
    component
        .placement
        .set_translation_vec(pos + dir * (clamped - current));
    Ok(())
}

fn clamp_axis_rotation(
    assembly: &mut Assembly,
    component: ComponentId,
    range_min: f64,
    range_max: f64,
) -> KernelResult<()> {
    let component = assembly
        .component_mut(component)
        .ok_or_else(|| KernelError::InvalidArgument(format!("unknown component {component}")))?;
    component.placement.rotation[2] = component.placement.rotation[2].clamp(range_min, range_max);
    Ok(())
}
