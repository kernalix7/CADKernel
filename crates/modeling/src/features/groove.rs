//! Groove operation — subtractive revolution (material removal by revolving a profile).

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, FaceData, Handle, SolidData};

use crate::boolean::{BooleanOp, boolean_op};
use crate::features::revolve::revolve;

/// Result of a groove operation.
pub struct GrooveResult {
    pub model: BRepModel,
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Creates a groove (subtractive revolution) in a base solid.
///
/// Revolves `profile` around `axis_origin`/`axis_dir` by `angle`,
/// then subtracts the tool solid from the base.
#[allow(clippy::too_many_arguments)]
pub fn groove(
    base_model: &BRepModel,
    base_solid: Handle<SolidData>,
    profile: &[Point3],
    axis_origin: Point3,
    axis_dir: Vec3,
    angle: f64,
    segments: usize,
) -> KernelResult<GrooveResult> {
    if profile.len() < 2 {
        return Err(KernelError::InvalidArgument(
            "groove requires at least 2 profile points".into(),
        ));
    }
    if angle <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "groove angle must be positive".into(),
        ));
    }

    let mut tool_model = BRepModel::new();
    let tool = revolve(
        &mut tool_model,
        profile,
        axis_origin,
        axis_dir,
        angle,
        segments,
    )?;

    let result_model = boolean_op(
        base_model,
        base_solid,
        &tool_model,
        tool.solid,
        BooleanOp::Difference,
    )?;

    let solids: Vec<Handle<SolidData>> = result_model.solids.iter().map(|(h, _)| h).collect();
    let faces: Vec<Handle<FaceData>> = result_model.faces.iter().map(|(h, _)| h).collect();

    let solid = solids
        .first()
        .copied()
        .ok_or(KernelError::TopologyError(
            "groove produced no solid".into(),
        ))?;

    Ok(GrooveResult {
        model: result_model,
        solid,
        faces,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::TAU;

    #[test]
    fn test_groove_from_cylinder() {
        let mut base = BRepModel::new();
        let b = crate::make_cylinder(&mut base, Point3::ORIGIN, 5.0, 10.0, 32).unwrap();

        // Groove profile: a small rectangle to cut a ring groove
        let profile = vec![
            Point3::new(4.0, 0.0, 4.0),
            Point3::new(5.5, 0.0, 4.0),
            Point3::new(5.5, 0.0, 6.0),
            Point3::new(4.0, 0.0, 6.0),
        ];

        let result = groove(
            &base,
            b.solid,
            &profile,
            Point3::ORIGIN,
            Vec3::Z,
            TAU,
            16,
        )
        .unwrap();
        assert!(result.model.solids.is_alive(result.solid));
    }

    #[test]
    fn test_groove_validation() {
        let mut base = BRepModel::new();
        let b = crate::make_box(&mut base, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let profile = vec![Point3::ORIGIN];
        assert!(groove(&base, b.solid, &profile, Point3::ORIGIN, Vec3::Z, TAU, 8).is_err());
    }
}
