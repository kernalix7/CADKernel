//! Uniform scaling operation for solids.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::Point3;
use cadkernel_topology::{BRepModel, FaceData, Handle, SolidData};

use super::copy_utils::copy_solid_transformed;

/// Result of a scale operation.
#[derive(Debug)]
pub struct ScaleResult {
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Scales a solid uniformly about a center point.
///
/// A **new** solid is created; the original is not modified.
/// Negative `factor` mirrors the solid (winding is reversed).
pub fn scale_solid(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    center: Point3,
    factor: f64,
) -> KernelResult<ScaleResult> {
    if factor.abs() < 1e-14 {
        return Err(KernelError::InvalidArgument(
            "scale factor must be non-zero".into(),
        ));
    }
    let op = model.history.next_operation("scale_solid");
    let reverse = factor < 0.0;

    let result = copy_solid_transformed(
        model,
        solid,
        op,
        |pt| {
            let d = pt - center;
            Point3::new(
                center.x + factor * d.x,
                center.y + factor * d.y,
                center.z + factor * d.z,
            )
        },
        reverse,
    )?;

    Ok(ScaleResult {
        solid: result.solid,
        faces: result.faces,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::make_box;

    #[test]
    fn test_scale_box_double() {
        let mut model = cadkernel_topology::BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let sr = scale_solid(&mut model, r.solid, Point3::ORIGIN, 2.0).unwrap();
        assert_eq!(sr.faces.len(), 6);

        // Check a vertex is scaled to 2.0
        let has_2 = model
            .vertices
            .iter()
            .any(|(_, v)| (v.point.x - 2.0).abs() < 1e-8 && (v.point.y - 2.0).abs() < 1e-8);
        assert!(has_2, "expected vertex at (2,2,...)");
    }

    #[test]
    fn test_scale_zero_rejected() {
        let mut model = cadkernel_topology::BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        assert!(scale_solid(&mut model, r.solid, Point3::ORIGIN, 0.0).is_err());
    }
}
