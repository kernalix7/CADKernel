//! Pocket operation — subtractive extrusion (material removal) from a base solid.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, FaceData, Handle, SolidData};

use crate::boolean::{BooleanOp, boolean_op};
use crate::features::extrude::extrude;

/// Result of a pocket operation.
pub struct PocketResult {
    pub model: BRepModel,
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Creates a pocket (subtractive extrusion) in a base solid.
///
/// Extrudes `profile` along `direction` by `depth`, then subtracts
/// the tool solid from the base.
pub fn pocket(
    base_model: &BRepModel,
    base_solid: Handle<SolidData>,
    profile: &[Point3],
    direction: Vec3,
    depth: f64,
) -> KernelResult<PocketResult> {
    if profile.len() < 3 {
        return Err(KernelError::InvalidArgument(
            "pocket requires at least 3 profile points".into(),
        ));
    }
    if depth <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "pocket depth must be positive".into(),
        ));
    }

    let mut tool_model = BRepModel::new();
    let tool = extrude(&mut tool_model, profile, direction, depth)?;

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
            "pocket produced no solid".into(),
        ))?;

    Ok(PocketResult {
        model: result_model,
        solid,
        faces,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pocket_from_box() {
        let mut base = BRepModel::new();
        let b = crate::make_box(&mut base, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        // Pocket on top face going downward
        let profile = vec![
            Point3::new(1.0, 1.0, 4.0),
            Point3::new(3.0, 1.0, 4.0),
            Point3::new(3.0, 3.0, 4.0),
            Point3::new(1.0, 3.0, 4.0),
        ];

        let result = pocket(&base, b.solid, &profile, -Vec3::Z, 2.0).unwrap();
        assert!(result.model.solids.is_alive(result.solid));
    }

    #[test]
    fn test_pocket_validation() {
        let mut base = BRepModel::new();
        let b = crate::make_box(&mut base, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let profile = vec![Point3::ORIGIN, Point3::new(1.0, 0.0, 0.0)];
        assert!(pocket(&base, b.solid, &profile, Vec3::Z, 1.0).is_err());
    }
}
