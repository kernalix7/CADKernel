//! Chained transformations applied to solids.

use cadkernel_core::KernelResult;
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, FaceData, Handle, SolidData};

use crate::features::copy_utils::copy_solid_transformed;

/// A transformation that can be applied to a solid.
#[derive(Debug, Clone)]
pub enum Transform {
    /// Translate by a vector.
    Translation(Vec3),
    /// Rotate around an axis defined by a point and direction.
    Rotation {
        axis_origin: Point3,
        axis_dir: Vec3,
        angle: f64,
    },
    /// Uniform scale around a center point.
    Scale { center: Point3, factor: f64 },
    /// Mirror across a plane defined by a point and normal.
    Mirror {
        plane_point: Point3,
        plane_normal: Vec3,
    },
}

/// Result of a multi-transform operation.
pub struct MultiTransformResult {
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Applies a chain of transformations to a solid, creating a new copy.
///
/// All transforms are composed and then applied in order to each vertex
/// of the solid. The result is a new solid in the same model.
pub fn multi_transform(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    transforms: &[Transform],
) -> KernelResult<MultiTransformResult> {
    let op = model.history.next_operation("multi_transform");

    // Check if any mirror transform is present (odd number = reversed winding)
    let mirror_count = transforms
        .iter()
        .filter(|t| matches!(t, Transform::Mirror { .. }))
        .count();
    let reverse_winding = mirror_count % 2 == 1;

    // Clone transforms into owned data for the closure
    let transforms_owned: Vec<Transform> = transforms.to_vec();

    let result = copy_solid_transformed(
        model,
        solid,
        op,
        |pt| apply_transforms(pt, &transforms_owned),
        reverse_winding,
    )?;

    Ok(MultiTransformResult {
        solid: result.solid,
        faces: result.faces,
    })
}

/// Applies a sequence of transforms to a point.
fn apply_transforms(mut pt: Point3, transforms: &[Transform]) -> Point3 {
    for t in transforms {
        pt = apply_single(pt, t);
    }
    pt
}

/// Applies a single transform to a point.
fn apply_single(pt: Point3, transform: &Transform) -> Point3 {
    match transform {
        Transform::Translation(v) => Point3::new(pt.x + v.x, pt.y + v.y, pt.z + v.z),
        Transform::Rotation {
            axis_origin,
            axis_dir,
            angle,
        } => {
            // Rodrigues' rotation formula
            let v = pt - *axis_origin;
            let len = axis_dir.length();
            if len < 1e-14 {
                return pt;
            }
            let k = Vec3::new(axis_dir.x / len, axis_dir.y / len, axis_dir.z / len);
            let (s, c) = angle.sin_cos();
            let dot = k.x * v.x + k.y * v.y + k.z * v.z;
            let cross = k.cross(v);

            let rx = v.x * c + cross.x * s + k.x * dot * (1.0 - c);
            let ry = v.y * c + cross.y * s + k.y * dot * (1.0 - c);
            let rz = v.z * c + cross.z * s + k.z * dot * (1.0 - c);

            Point3::new(
                axis_origin.x + rx,
                axis_origin.y + ry,
                axis_origin.z + rz,
            )
        }
        Transform::Scale { center, factor } => {
            let dx = pt.x - center.x;
            let dy = pt.y - center.y;
            let dz = pt.z - center.z;
            Point3::new(
                center.x + dx * factor,
                center.y + dy * factor,
                center.z + dz * factor,
            )
        }
        Transform::Mirror {
            plane_point,
            plane_normal,
        } => {
            let len = plane_normal.length();
            if len < 1e-14 {
                return pt;
            }
            let n = Vec3::new(
                plane_normal.x / len,
                plane_normal.y / len,
                plane_normal.z / len,
            );
            let v = pt - *plane_point;
            let dist = n.x * v.x + n.y * v.y + n.z * v.z;
            Point3::new(
                pt.x - 2.0 * dist * n.x,
                pt.y - 2.0 * dist * n.y,
                pt.z - 2.0 * dist * n.z,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::make_box;

    #[test]
    fn test_translation() {
        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let result = multi_transform(
            &mut model,
            r.solid,
            &[Transform::Translation(Vec3::new(5.0, 0.0, 0.0))],
        )
        .unwrap();

        assert!(model.solids.is_alive(result.solid));
        assert_eq!(result.faces.len(), 6);
    }

    #[test]
    fn test_translation_rotation_chain() {
        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let transforms = vec![
            Transform::Translation(Vec3::new(2.0, 0.0, 0.0)),
            Transform::Rotation {
                axis_origin: Point3::ORIGIN,
                axis_dir: Vec3::Z,
                angle: std::f64::consts::FRAC_PI_2,
            },
        ];

        let result = multi_transform(&mut model, r.solid, &transforms).unwrap();
        assert!(model.solids.is_alive(result.solid));
        assert_eq!(result.faces.len(), 6);
    }

    #[test]
    fn test_scale() {
        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let result = multi_transform(
            &mut model,
            r.solid,
            &[Transform::Scale {
                center: Point3::ORIGIN,
                factor: 2.0,
            }],
        )
        .unwrap();

        assert!(model.solids.is_alive(result.solid));
    }

    #[test]
    fn test_mirror() {
        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::new(1.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap();

        let result = multi_transform(
            &mut model,
            r.solid,
            &[Transform::Mirror {
                plane_point: Point3::ORIGIN,
                plane_normal: Vec3::X,
            }],
        )
        .unwrap();

        assert!(model.solids.is_alive(result.solid));
        assert_eq!(result.faces.len(), 6);
    }
}
