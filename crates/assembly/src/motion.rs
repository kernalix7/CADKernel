use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::Vec3;
use serde::{Deserialize, Serialize};

use crate::mate::{AxisRef, CurveRef};
use crate::solver::{SolverOptions, clamp_single_dof_ranges};
use crate::{Assembly, ComponentId, Mate, normalized, validate_vec3};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MotionReport {
    pub component: ComponentId,
    pub applied_delta: f64,
    pub solved: bool,
}

pub fn drag_component(
    assembly: &mut Assembly,
    component_id: ComponentId,
    screen_delta: [f64; 2],
) -> KernelResult<MotionReport> {
    validate_screen_delta(screen_delta)?;
    assembly.require_component(component_id)?;

    let candidates = assembly
        .active_mates()
        .filter_map(|(_, mate)| single_dof_for_component(mate, component_id))
        .collect::<Vec<_>>();

    if candidates.is_empty() {
        return Err(KernelError::InvalidArgument(format!(
            "{component_id} has no single-DoF mate"
        )));
    }
    if candidates.len() > 1 {
        return Err(KernelError::InvalidArgument(format!(
            "{component_id} has multiple single-DoF mates"
        )));
    }

    let scalar = if screen_delta[0].abs() >= screen_delta[1].abs() {
        screen_delta[0]
    } else {
        screen_delta[1]
    };
    apply_single_dof(assembly, component_id, &candidates[0], scalar)?;
    clamp_single_dof_ranges(assembly)?;
    let report = assembly.solve(SolverOptions::default())?;

    Ok(MotionReport {
        component: component_id,
        applied_delta: scalar,
        solved: report.converged,
    })
}

#[derive(Debug, Clone)]
enum SingleDof {
    Slider(AxisRef),
    Hinge(AxisRef),
    Rack(AxisRef, f64),
    Screw(AxisRef, f64),
    Path(CurveRef),
}

fn single_dof_for_component(mate: &Mate, component: ComponentId) -> Option<SingleDof> {
    match mate {
        Mate::Slider { axis, .. } if axis.component == component => {
            Some(SingleDof::Slider(axis.clone()))
        }
        Mate::Hinge { axis, .. } if axis.component == component => {
            Some(SingleDof::Hinge(axis.clone()))
        }
        Mate::Rack { axis, pitch } if axis.component == component => {
            Some(SingleDof::Rack(axis.clone(), *pitch))
        }
        Mate::Screw { axis, pitch } if axis.component == component => {
            Some(SingleDof::Screw(axis.clone(), *pitch))
        }
        Mate::Path {
            component: path_component,
            path_curve,
        } if *path_component == component => Some(SingleDof::Path(path_curve.clone())),
        _ => None,
    }
}

fn apply_single_dof(
    assembly: &mut Assembly,
    component: ComponentId,
    dof: &SingleDof,
    scalar: f64,
) -> KernelResult<()> {
    match dof {
        SingleDof::Slider(axis) => translate_along_axis(assembly, component, axis, scalar),
        SingleDof::Hinge(axis) => rotate_about_axis(assembly, component, axis, scalar),
        SingleDof::Rack(axis, pitch) => {
            translate_along_axis(assembly, component, axis, scalar * pitch)
        }
        SingleDof::Screw(axis, pitch) => {
            let linear = scalar * pitch / std::f64::consts::TAU;
            translate_along_axis(assembly, component, axis, linear)?;
            rotate_about_axis(assembly, component, axis, scalar)
        }
        SingleDof::Path(curve) => translate_along_path(assembly, component, curve, scalar),
    }
}

fn translate_along_axis(
    assembly: &mut Assembly,
    component: ComponentId,
    axis: &AxisRef,
    amount: f64,
) -> KernelResult<()> {
    let dir = normalized(axis.direction, "axis direction")?;
    let delta = dir * amount;
    assembly.translate_component(component, [delta.x, delta.y, delta.z])
}

fn rotate_about_axis(
    assembly: &mut Assembly,
    component: ComponentId,
    axis: &AxisRef,
    amount: f64,
) -> KernelResult<()> {
    let dir = normalized(axis.direction, "axis direction")?;
    let component = assembly
        .component_mut(component)
        .ok_or_else(|| KernelError::InvalidArgument(format!("unknown component {component}")))?;
    component.placement.rotation[0] += dir.x * amount;
    component.placement.rotation[1] += dir.y * amount;
    component.placement.rotation[2] += dir.z * amount;
    Ok(())
}

fn translate_along_path(
    assembly: &mut Assembly,
    component: ComponentId,
    curve: &CurveRef,
    amount: f64,
) -> KernelResult<()> {
    let first = curve.control_points.first().copied().ok_or_else(|| {
        KernelError::InvalidArgument("path curve requires at least two control points".into())
    })?;
    let last = curve.control_points.last().copied().ok_or_else(|| {
        KernelError::InvalidArgument("path curve requires at least two control points".into())
    })?;
    let dir = (Vec3::from(last) - Vec3::from(first))
        .normalized()
        .ok_or_else(|| KernelError::InvalidArgument("path curve direction is degenerate".into()))?;
    let delta = dir * amount;
    assembly.translate_component(component, [delta.x, delta.y, delta.z])
}

fn validate_screen_delta(delta: [f64; 2]) -> KernelResult<()> {
    validate_vec3([delta[0], delta[1], 0.0], "screen_delta")
}
