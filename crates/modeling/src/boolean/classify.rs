use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, FaceData, Handle, SolidData};

use super::broad_phase::collect_solid_faces;

/// Classification of a face relative to another solid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacePosition {
    Inside,
    Outside,
    OnBoundary,
}

/// Computes the centroid of a face by averaging its boundary vertices.
pub fn face_centroid(model: &BRepModel, face: Handle<FaceData>) -> KernelResult<Point3> {
    let face_data = model
        .faces
        .get(face)
        .ok_or(KernelError::InvalidHandle("face"))?;
    let loop_data = model
        .loops
        .get(face_data.outer_loop)
        .ok_or(KernelError::InvalidHandle("loop"))?;
    let hes = model.loop_half_edges(loop_data.half_edge);

    let mut sum = Vec3::ZERO;
    let mut count = 0.0;
    for he_h in hes {
        let he = model
            .half_edges
            .get(he_h)
            .ok_or(KernelError::InvalidHandle("half_edge"))?;
        let v = model
            .vertices
            .get(he.origin)
            .ok_or(KernelError::InvalidHandle("vertex"))?;
        sum += Vec3::from(v.point);
        count += 1.0;
    }
    Ok(Point3::new(sum.x / count, sum.y / count, sum.z / count))
}

/// Approximates face normal from the first 3 boundary vertices (flat-face assumption).
pub fn face_normal_approx(model: &BRepModel, face: Handle<FaceData>) -> KernelResult<Vec3> {
    let face_data = model
        .faces
        .get(face)
        .ok_or(KernelError::InvalidHandle("face"))?;
    let loop_data = model
        .loops
        .get(face_data.outer_loop)
        .ok_or(KernelError::InvalidHandle("loop"))?;
    let hes = model.loop_half_edges(loop_data.half_edge);

    if hes.len() < 3 {
        return Ok(Vec3::Z);
    }

    let p0 = model
        .vertices
        .get(
            model
                .half_edges
                .get(hes[0])
                .ok_or(KernelError::InvalidHandle("half_edge"))?
                .origin,
        )
        .ok_or(KernelError::InvalidHandle("vertex"))?
        .point;
    let p1 = model
        .vertices
        .get(
            model
                .half_edges
                .get(hes[1])
                .ok_or(KernelError::InvalidHandle("half_edge"))?
                .origin,
        )
        .ok_or(KernelError::InvalidHandle("vertex"))?
        .point;
    let p2 = model
        .vertices
        .get(
            model
                .half_edges
                .get(hes[2])
                .ok_or(KernelError::InvalidHandle("half_edge"))?
                .origin,
        )
        .ok_or(KernelError::InvalidHandle("vertex"))?
        .point;

    let e1 = p1 - p0;
    let e2 = p2 - p0;
    Ok(e1.cross(e2).normalized().unwrap_or(Vec3::Z))
}

/// Simple ray-casting point-in-solid test.
///
/// Casts a ray from `point` along +X and counts how many face planes it crosses.
/// Odd count → inside, even count → outside.
pub fn point_in_solid(
    point: Point3,
    model: &BRepModel,
    solid: Handle<SolidData>,
) -> KernelResult<FacePosition> {
    let faces = collect_solid_faces(model, solid)?;
    let ray_dir = Vec3::new(1.0, 0.0, 0.0);

    let mut crossings = 0u32;

    for &face_h in &faces {
        let centroid = face_centroid(model, face_h)?;
        let normal = face_normal_approx(model, face_h)?;

        let denom = normal.dot(ray_dir);
        if denom.abs() < 1e-10 {
            continue;
        }

        let t = normal.dot(centroid - point) / denom;
        if t <= 0.0 {
            continue;
        }

        let hit = point + ray_dir * t;

        let bb = super::broad_phase::face_bbox(model, face_h)?;
        let margin = 1e-6;
        if hit.x >= bb.min.x - margin
            && hit.x <= bb.max.x + margin
            && hit.y >= bb.min.y - margin
            && hit.y <= bb.max.y + margin
            && hit.z >= bb.min.z - margin
            && hit.z <= bb.max.z + margin
        {
            crossings += 1;
        }
    }

    if crossings % 2 == 1 {
        Ok(FacePosition::Inside)
    } else {
        Ok(FacePosition::Outside)
    }
}

/// Classifies a face of model_a relative to the solid in model_b.
pub fn classify_face(
    model_a: &BRepModel,
    face: Handle<FaceData>,
    model_b: &BRepModel,
    solid_b: Handle<SolidData>,
) -> KernelResult<FacePosition> {
    let centroid = face_centroid(model_a, face)?;
    let normal = face_normal_approx(model_a, face)?;
    let test_point = centroid + normal * (-1e-6);
    point_in_solid(test_point, model_b, solid_b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::make_box;

    #[test]
    fn test_point_inside_box() {
        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();
        let pos = point_in_solid(Point3::new(1.0, 1.0, 1.0), &model, r.solid).unwrap();
        assert_eq!(pos, FacePosition::Inside);
    }

    #[test]
    fn test_point_outside_box() {
        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();
        let pos = point_in_solid(Point3::new(5.0, 5.0, 5.0), &model, r.solid).unwrap();
        assert_eq!(pos, FacePosition::Outside);
    }
}
