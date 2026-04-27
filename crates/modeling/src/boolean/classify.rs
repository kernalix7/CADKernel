use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, FaceData, Handle, SolidData};

use super::broad_phase::collect_solid_faces;

/// Classification of a face relative to another solid.
///
/// `OnBoundary` is further split into two subtypes that distinguish
/// *co-facing* overlap (both solids lie on the SAME side of the shared
/// face plane — e.g. two identical boxes) from *mating* overlap (the two
/// solids lie on OPPOSITE sides of the plane — e.g. two boxes touching
/// along one face). This distinction is necessary for correct boolean
/// face-kept rules:
///   - Union of co-facing pair → keep one copy (A) to avoid duplication.
///   - Union of mating pair → drop both (the shared face is interior
///     to the union).
///   - Difference of mating pair → keep A (B doesn't carve material
///     from A through that face).
///   - Difference of co-facing pair → drop A (A's face coincides with
///     B's face and B's interior fully occupies A's side).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacePosition {
    Inside,
    Outside,
    /// Coincident with B's boundary; A-interior and B-interior on same side.
    OnBoundarySame,
    /// Coincident with B's boundary; A-interior and B-interior on opposite sides.
    OnBoundaryOpposite,
}

impl FacePosition {
    /// True when the face lies on the other solid's boundary (either subtype).
    pub fn is_on_boundary(self) -> bool {
        matches!(self, FacePosition::OnBoundarySame | FacePosition::OnBoundaryOpposite)
    }
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
///
/// Returns `Outside` for degenerate faces (< 3 vertices, zero-area)
/// rather than panicking.
pub fn classify_face(
    model_a: &BRepModel,
    face: Handle<FaceData>,
    model_b: &BRepModel,
    solid_b: Handle<SolidData>,
) -> KernelResult<FacePosition> {
    let centroid = face_centroid(model_a, face)?;
    let normal = face_normal_approx(model_a, face)?;
    let polygon = face_polygon(model_a, face)?;

    // Degenerate face guard: faces with < 3 vertices cannot be classified
    if polygon.len() < 3 {
        return Ok(FacePosition::Outside);
    }

    // Check for zero-area (degenerate) face
    let normal_len = normal.length();
    if normal_len < 1e-14 {
        return Ok(FacePosition::Outside);
    }

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
            FacePosition::OnBoundarySame | FacePosition::OnBoundaryOpposite => {}
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

/// Classifies a face of model_a against solid_b with awareness of
/// coplanar-overlap (shared-boundary) regions. Returns
/// `OnBoundarySame` when A-interior and B-interior lie on the same side
/// of the shared face plane (co-facing), `OnBoundaryOpposite` when they
/// lie on opposite sides (mating). The classification is decided by
/// majority-voting interior-biased edge midpoints sampled on BOTH sides
/// of the face plane. This lets the caller make an operation-specific
/// decision (Difference removes the A-face on co-facing but keeps it on
/// mating; Union keeps one copy of co-facing but drops mating; etc.).
pub fn classify_face_with_coplanar(
    model_a: &BRepModel,
    face: Handle<FaceData>,
    model_b: &BRepModel,
    solid_b: Handle<SolidData>,
) -> KernelResult<FacePosition> {
    let centroid = face_centroid(model_a, face)?;
    let normal = face_normal_approx(model_a, face)?;
    let polygon = face_polygon(model_a, face)?;

    if polygon.len() < 3 || normal.length() < 1e-14 {
        return Ok(FacePosition::Outside);
    }

    // Collect interior sample points. Strategy: prefer midpoints of the
    // LONGEST edges as primary samples, offset toward the centroid by a
    // fraction of the edge length. Long edges belong to the outer
    // polygon perimeter (box sides, not the faceted hole boundary) and
    // their offset-midpoints reliably land inside the polygon's interior
    // even for keyhole shapes where the vertex-average centroid itself
    // may lie inside the hole.
    let n_poly = polygon.len();
    let mut edge_list: Vec<(f64, usize)> = (0..n_poly)
        .map(|i| {
            let a = polygon[i];
            let b = polygon[(i + 1) % n_poly];
            ((b - a).length(), i)
        })
        .collect();
    edge_list.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    // Choose the projection axis (drop the dominant normal component) so
    // point-in-polygon filters keyhole-slit samples correctly.
    let (nx, ny, nz) = (normal.x.abs(), normal.y.abs(), normal.z.abs());
    let drop_axis = if nx >= ny && nx >= nz {
        0
    } else if ny >= nz {
        1
    } else {
        2
    };

    const SAMPLE_COUNT: usize = 16;
    let mut interior: Vec<Point3> = Vec::new();
    // Generate candidate samples from edge-midpoints offset by the edge's
    // INWARD perpendicular (tangent × face-normal). This is more robust
    // than "toward centroid" for keyhole polygons whose vertex-average
    // centroid may lie in the hole rather than in the polygon interior.
    for &(edge_len, i) in edge_list.iter().take(SAMPLE_COUNT) {
        if edge_len < 1e-10 {
            continue;
        }
        let a = polygon[i];
        let b = polygon[(i + 1) % n_poly];
        let mid = Point3::new(
            (a.x + b.x) * 0.5,
            (a.y + b.y) * 0.5,
            (a.z + b.z) * 0.5,
        );
        let tangent = (b - a).normalized().unwrap_or(Vec3::X);
        // Inward perpendicular: (face_normal × tangent) lies in the polygon
        // plane and points into the polygon interior when the polygon is
        // wound CCW viewed from the +normal side.
        let inward = normal.cross(tangent).normalized().unwrap_or(Vec3::ZERO);
        let toward_cen = (centroid - mid).normalized().unwrap_or(Vec3::ZERO);
        // Use inward unless it disagrees with the centroid direction for a
        // non-keyhole polygon (in which case winding is reversed).
        let inward = if inward.dot(toward_cen) >= 0.0 {
            inward
        } else {
            -inward
        };
        let offset = (edge_len * 0.25).clamp(1e-3, 0.5);
        // Propose two candidates at different offsets to survive skinny
        // keyhole geometry.
        let cand1 = mid + inward * offset;
        let cand2 = mid + inward * (offset * 0.5);
        for cand in [cand1, cand2] {
            // Only keep candidates that lie strictly inside the polygon
            // (projected to 2D). This correctly excludes samples that
            // strayed into a keyhole hole region despite a winding-based
            // inward offset.
            if point_in_polygon_2d(cand, &polygon, drop_axis) {
                interior.push(cand);
            }
        }
    }
    if interior.is_empty() {
        // Fall back to centroid-offset samples.
        for &(_, i) in edge_list.iter().take(8) {
            let a = polygon[i];
            let b = polygon[(i + 1) % n_poly];
            let mid = Point3::new(
                (a.x + b.x) * 0.5,
                (a.y + b.y) * 0.5,
                (a.z + b.z) * 0.5,
            );
            let toward = (centroid - mid).normalized().unwrap_or(Vec3::ZERO);
            let cand = mid + toward * 1e-3;
            if point_in_polygon_2d(cand, &polygon, drop_axis) {
                interior.push(cand);
            }
        }
    }
    if interior.is_empty() {
        interior.push(centroid);
    }

    // Vote on both the outward and inward sides of each interior point.
    let mut out_inside = 0u32;
    let mut out_outside = 0u32;
    let mut in_inside = 0u32;
    let mut in_outside = 0u32;
    for pt in &interior {
        match point_in_solid(*pt + normal * 1e-6, model_b, solid_b)? {
            FacePosition::Inside => out_inside += 1,
            FacePosition::Outside => out_outside += 1,
            FacePosition::OnBoundarySame | FacePosition::OnBoundaryOpposite => {}
        }
        match point_in_solid(*pt - normal * 1e-6, model_b, solid_b)? {
            FacePosition::Inside => in_inside += 1,
            FacePosition::Outside => in_outside += 1,
            FacePosition::OnBoundarySame | FacePosition::OnBoundaryOpposite => {}
        }
    }

    let out_side = if out_inside > out_outside {
        FacePosition::Inside
    } else if out_outside > out_inside {
        FacePosition::Outside
    } else {
        // Tie: treat as the centroid outward result.
        point_in_solid(centroid + normal * 1e-6, model_b, solid_b)?
    };
    let in_side = if in_inside > in_outside {
        FacePosition::Inside
    } else if in_outside > in_inside {
        FacePosition::Outside
    } else {
        point_in_solid(centroid - normal * 1e-6, model_b, solid_b)?
    };

    // Coplanar overlap detection — reliable majority signals on both sides.
    //
    // A-face outward normal points away from A's interior. Decide co-facing
    // vs mating by where B's interior lies:
    //   out=Outside, in=Inside  → B-interior on A's INWARD side → co-facing
    //       (A-interior and B-interior both on the negative-normal side).
    //   out=Inside,  in=Outside → B-interior on A's OUTWARD side → mating
    //       (A-interior and B-interior on opposite sides of the plane).
    if out_side == FacePosition::Outside && in_side == FacePosition::Inside {
        return Ok(FacePosition::OnBoundarySame);
    }
    if out_side == FacePosition::Inside && in_side == FacePosition::Outside {
        return Ok(FacePosition::OnBoundaryOpposite);
    }

    // Otherwise defer to the standard outward-side classification.
    Ok(out_side)
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
