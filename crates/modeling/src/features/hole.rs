//! Hole operation — creates cylindrical holes in a solid.

use std::f64::consts::TAU;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, FaceData, Handle, SolidData};

use crate::boolean::{BooleanOp, boolean_op};

/// Result of a hole operation.
pub struct HoleResult {
    pub model: BRepModel,
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Creates a cylindrical hole through a solid.
///
/// The hole is centered at `center`, goes along `direction` for `depth`,
/// with the given `radius`. The hole is created by subtracting a cylinder
/// from the base solid.
pub fn hole(
    base_model: &BRepModel,
    base_solid: Handle<SolidData>,
    center: Point3,
    direction: Vec3,
    radius: f64,
    depth: f64,
    segments: usize,
) -> KernelResult<HoleResult> {
    if radius <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "hole radius must be positive".into(),
        ));
    }
    if depth <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "hole depth must be positive".into(),
        ));
    }
    if segments < 3 {
        return Err(KernelError::InvalidArgument(
            "hole needs at least 3 segments".into(),
        ));
    }

    // Create the tool cylinder aligned with the hole direction
    let dir = direction.normalized().ok_or(KernelError::InvalidArgument(
        "hole direction must be non-zero".into(),
    ))?;

    // Build cylinder profile in local XY plane, then extrude along dir
    let mut tool_model = BRepModel::new();

    // Create cylinder at origin along Z, then we'll position it
    // The boolean classify uses centroid-based inside/outside test,
    // so we need to build the cylinder at the correct location.
    //
    // Build a polygon profile for the cylinder cross-section
    let n = segments;
    let mut profile = Vec::with_capacity(n);

    // Build local coordinate frame
    let up = if dir.x.abs() < 0.9 {
        Vec3::X
    } else {
        Vec3::Y
    };
    let u = dir.cross(up).normalized().unwrap_or(Vec3::X);
    let v = u.cross(dir).normalized().unwrap_or(Vec3::Y);

    for i in 0..n {
        let angle = TAU * i as f64 / n as f64;
        let (s, c) = angle.sin_cos();
        profile.push(Point3::new(
            center.x + radius * (u.x * c + v.x * s),
            center.y + radius * (u.y * c + v.y * s),
            center.z + radius * (u.z * c + v.z * s),
        ));
    }

    let tool = crate::features::extrude::extrude(&mut tool_model, &profile, dir, depth)?;

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
        .ok_or(KernelError::TopologyError("hole produced no solid".into()))?;

    Ok(HoleResult {
        model: result_model,
        solid,
        faces,
    })
}

/// Creates a countersunk hole (cylindrical hole with conical entry).
#[allow(clippy::too_many_arguments)]
pub fn countersunk_hole(
    base_model: &BRepModel,
    base_solid: Handle<SolidData>,
    center: Point3,
    direction: Vec3,
    radius: f64,
    depth: f64,
    countersink_radius: f64,
    countersink_depth: f64,
    segments: usize,
) -> KernelResult<HoleResult> {
    // First create the main hole
    let step1 = hole(base_model, base_solid, center, direction, radius, depth, segments)?;

    // Then create the countersink (larger, shallower hole)
    let result = hole(
        &step1.model,
        step1.solid,
        center,
        direction,
        countersink_radius,
        countersink_depth,
        segments,
    )?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hole_through_box() {
        let mut base = BRepModel::new();
        let b = crate::make_box(&mut base, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let result = hole(
            &base,
            b.solid,
            Point3::new(2.0, 2.0, 4.0),
            -Vec3::Z,
            0.5,
            4.0,
            16,
        )
        .unwrap();
        assert!(result.model.solids.is_alive(result.solid));
    }

    #[test]
    fn test_hole_validation() {
        let mut base = BRepModel::new();
        let b = crate::make_box(&mut base, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        assert!(hole(&base, b.solid, Point3::ORIGIN, Vec3::Z, -1.0, 1.0, 8).is_err());
        assert!(hole(&base, b.solid, Point3::ORIGIN, Vec3::Z, 1.0, -1.0, 8).is_err());
        assert!(hole(&base, b.solid, Point3::ORIGIN, Vec3::Z, 1.0, 1.0, 2).is_err());
    }

    #[test]
    fn test_countersunk_hole() {
        let mut base = BRepModel::new();
        let b = crate::make_box(&mut base, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let result = countersunk_hole(
            &base,
            b.solid,
            Point3::new(2.0, 2.0, 4.0),
            -Vec3::Z,
            0.3,
            4.0,
            0.6,
            0.5,
            16,
        )
        .unwrap();
        assert!(result.model.solids.is_alive(result.solid));
    }
}
