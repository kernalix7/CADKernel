//! Pad operation — additive extrusion from a sketch profile onto a base solid.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, FaceData, Handle, SolidData};

use crate::boolean::{BooleanOp, boolean_op};
use crate::features::extrude::extrude;

/// Result of a pad operation.
pub struct PadResult {
    pub model: BRepModel,
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Pads (additively extrudes) a sketch profile onto a base solid.
///
/// Creates an extrusion from `profile` along `direction` by `distance`,
/// then unions it with the base solid.
pub fn pad(
    base_model: &BRepModel,
    base_solid: Handle<SolidData>,
    profile: &[Point3],
    direction: Vec3,
    distance: f64,
) -> KernelResult<PadResult> {
    if profile.len() < 3 {
        return Err(KernelError::InvalidArgument(
            "pad requires at least 3 profile points".into(),
        ));
    }
    if distance <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "pad distance must be positive".into(),
        ));
    }

    let mut tool_model = BRepModel::new();
    let tool = extrude(&mut tool_model, profile, direction, distance)?;

    let result_model = boolean_op(
        base_model,
        base_solid,
        &tool_model,
        tool.solid,
        BooleanOp::Union,
    )?;

    let solids: Vec<Handle<SolidData>> = result_model.solids.iter().map(|(h, _)| h).collect();
    let faces: Vec<Handle<FaceData>> = result_model.faces.iter().map(|(h, _)| h).collect();

    let solid = solids
        .first()
        .copied()
        .ok_or(KernelError::TopologyError("pad produced no solid".into()))?;

    Ok(PadResult {
        model: result_model,
        solid,
        faces,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pad_onto_box() {
        let mut base = BRepModel::new();
        let b = crate::make_box(&mut base, Point3::ORIGIN, 4.0, 4.0, 2.0).unwrap();

        let profile = vec![
            Point3::new(1.0, 1.0, 2.0),
            Point3::new(3.0, 1.0, 2.0),
            Point3::new(3.0, 3.0, 2.0),
            Point3::new(1.0, 3.0, 2.0),
        ];

        let result = pad(&base, b.solid, &profile, Vec3::Z, 3.0).unwrap();
        assert!(result.model.solids.is_alive(result.solid));
        assert!(result.faces.len() >= 6);
    }

    #[test]
    fn test_pad_validation() {
        let mut base = BRepModel::new();
        let b = crate::make_box(&mut base, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let short_profile = vec![Point3::ORIGIN, Point3::new(1.0, 0.0, 0.0)];
        assert!(pad(&base, b.solid, &short_profile, Vec3::Z, 1.0).is_err());
    }
}
