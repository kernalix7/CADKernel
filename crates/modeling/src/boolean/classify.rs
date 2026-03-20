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

/// Collects the 3D polygon vertices of a face's outer loop.
fn face_polygon(model: &BRepModel, face: Handle<FaceData>) -> KernelResult<Vec<Point3>> {
    let face_data = model
        .faces
        .get(face)
        .ok_or(KernelError::InvalidHandle("face"))?;
    let loop_data = model
        .loops
        .get(face_data.outer_loop)
        .ok_or(KernelError::InvalidHandle("loop"))?;
    let hes = model.loop_half_edges(loop_data.half_edge);

    let mut pts = Vec::with_capacity(hes.len());
    for he_h in hes {
        let he = model
            .half_edges
            .get(he_h)
            .ok_or(KernelError::InvalidHandle("half_edge"))?;
        let v = model
            .vertices
            .get(he.origin)
            .ok_or(KernelError::InvalidHandle("vertex"))?;
        pts.push(v.point);
    }
    Ok(pts)
}

/// 2D point-in-polygon test using the crossing number (ray-casting) algorithm.
/// Projects the polygon and test point onto the 2D plane that drops `drop_axis`.
fn point_in_polygon_2d(hit: Point3, polygon: &[Point3], drop_axis: usize) -> bool {
    // Project to 2D by dropping the axis with the largest normal component.
    let proj = |p: Point3| -> (f64, f64) {
        match drop_axis {
            0 => (p.y, p.z),
            1 => (p.x, p.z),
            _ => (p.x, p.y),
        }
    };
    let (hx, hy) = proj(hit);
    let n = polygon.len();
    let mut crossings = 0u32;
    for i in 0..n {
        let (ax, ay) = proj(polygon[i]);
        let (bx, by) = proj(polygon[(i + 1) % n]);
        // Check if edge crosses the horizontal ray from (hx, hy) to +∞
        if (ay <= hy && by > hy) || (by <= hy && ay > hy) {
            let t = (hy - ay) / (by - ay);
            let ix = ax + t * (bx - ax);
            if hx < ix {
                crossings += 1;
            }
        }
    }
    crossings % 2 == 1
}

/// Ray-casting point-in-solid test.
///
/// Casts a ray from `point` along +X and counts how many face polygons it
/// crosses using proper ray-polygon intersection.
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
        let polygon = face_polygon(model, face_h)?;
        if polygon.len() < 3 {
            continue;
        }

        // Compute face plane from first 3 vertices.
        let e1 = polygon[1] - polygon[0];
        let e2 = polygon[2] - polygon[0];
        let normal = e1.cross(e2);
        let n_len = normal.length();
        if n_len < 1e-14 {
            continue;
        }
        let normal = normal / n_len;

        // Ray-plane intersection.
        let denom = normal.dot(ray_dir);
        if denom.abs() < 1e-10 {
            continue;
        }

        let t = normal.dot(polygon[0] - point) / denom;
        if t <= 0.0 {
            continue;
        }

        let hit = point + ray_dir * t;

        // Determine which axis to drop for 2D projection (largest normal component).
        let drop_axis = if normal.x.abs() >= normal.y.abs() && normal.x.abs() >= normal.z.abs() {
            0
        } else if normal.y.abs() >= normal.z.abs() {
            1
        } else {
            2
        };

        if point_in_polygon_2d(hit, &polygon, drop_axis) {
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
///
/// Uses multi-sample majority voting: tests the face centroid plus edge
/// midpoints offset inward. This handles near-boundary centroids that
/// could be misclassified by a single-point test.
pub fn classify_face(
    model_a: &BRepModel,
    face: Handle<FaceData>,
    model_b: &BRepModel,
    solid_b: Handle<SolidData>,
) -> KernelResult<FacePosition> {
    let centroid = face_centroid(model_a, face)?;
    let normal = face_normal_approx(model_a, face)?;
    let polygon = face_polygon(model_a, face)?;

    // Generate sample points: centroid + edge midpoints offset toward centroid
    let mut sample_points = vec![centroid + normal * 1e-6];
    let n = polygon.len().min(6); // limit to 6 edge midpoints
    for i in 0..n {
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];
        let mid = Point3::new(
            (a.x + b.x) * 0.5,
            (a.y + b.y) * 0.5,
            (a.z + b.z) * 0.5,
        );
        // Offset midpoint slightly toward centroid to stay inside the face
        let toward_center = (centroid - mid).normalized().unwrap_or(Vec3::ZERO);
        sample_points.push(mid + toward_center * 1e-4 + normal * 1e-6);
    }

    // Majority vote
    let mut inside_count = 0u32;
    let mut outside_count = 0u32;
    for pt in &sample_points {
        match point_in_solid(*pt, model_b, solid_b)? {
            FacePosition::Inside => inside_count += 1,
            FacePosition::Outside => outside_count += 1,
            FacePosition::OnBoundary => {}
        }
    }

    if inside_count > outside_count {
        Ok(FacePosition::Inside)
    } else if outside_count > inside_count {
        Ok(FacePosition::Outside)
    } else {
        // Tie — use centroid result as tiebreaker
        point_in_solid(centroid + normal * 1e-6, model_b, solid_b)
    }
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
