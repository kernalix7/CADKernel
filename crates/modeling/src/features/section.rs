//! Section operation — computes cross-section contours of a solid cut by a plane.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, Handle, SolidData};

use super::copy_utils::collect_solid_faces;

/// A single edge of the cross-section contour.
#[derive(Debug, Clone)]
pub struct SectionEdge {
    pub start: Point3,
    pub end: Point3,
}

/// Result of a section operation.
#[derive(Debug, Clone)]
pub struct SectionResult {
    pub edges: Vec<SectionEdge>,
}

/// Computes the cross-section contour of a solid cut by a plane.
///
/// The plane is defined by a point and normal. Returns a list of edges
/// that form the intersection contour(s).
pub fn section_solid(
    model: &BRepModel,
    solid: Handle<SolidData>,
    plane_point: Point3,
    plane_normal: Vec3,
) -> KernelResult<SectionResult> {
    let normal = plane_normal
        .normalized()
        .ok_or(KernelError::InvalidArgument(
            "plane_normal must be non-zero".into(),
        ))?;

    let face_handles = collect_solid_faces(model, solid)?;
    let mut edges = Vec::new();

    for &face_h in &face_handles {
        let verts = model.vertices_of_face(face_h)?;
        if verts.len() < 3 {
            continue;
        }

        // Collect vertex positions and signed distances to plane
        let positions: Vec<Point3> = verts
            .iter()
            .map(|&vh| {
                model
                    .vertices
                    .get(vh)
                    .map(|v| v.point)
                    .unwrap_or(Point3::ORIGIN)
            })
            .collect();

        let dists: Vec<f64> = positions
            .iter()
            .map(|p| {
                let d = *p - plane_point;
                d.x * normal.x + d.y * normal.y + d.z * normal.z
            })
            .collect();

        // Find edges that cross the plane
        let n = positions.len();
        let mut crossing_points = Vec::new();

        for i in 0..n {
            let j = (i + 1) % n;
            let di = dists[i];
            let dj = dists[j];

            if di.abs() < 1e-12 {
                crossing_points.push(positions[i]);
            } else if di * dj < 0.0 {
                // Edge crosses plane
                let t = di / (di - dj);
                let p = Point3::new(
                    positions[i].x + t * (positions[j].x - positions[i].x),
                    positions[i].y + t * (positions[j].y - positions[i].y),
                    positions[i].z + t * (positions[j].z - positions[i].z),
                );
                crossing_points.push(p);
            }
        }

        // Each face contributes at most one edge to the section
        if crossing_points.len() >= 2 {
            // Deduplicate nearby points
            let mut unique = vec![crossing_points[0]];
            for p in &crossing_points[1..] {
                let last = unique.last().unwrap();
                let dx = p.x - last.x;
                let dy = p.y - last.y;
                let dz = p.z - last.z;
                if dx * dx + dy * dy + dz * dz > 1e-20 {
                    unique.push(*p);
                }
            }

            if unique.len() >= 2 {
                edges.push(SectionEdge {
                    start: unique[0],
                    end: unique[1],
                });
            }
        }
    }

    Ok(SectionResult { edges })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_section_box_midplane() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let result = section_solid(
            &model,
            b.solid,
            Point3::new(0.0, 0.0, 2.0),
            Vec3::Z,
        )
        .unwrap();

        // A box cut at z=2 should produce 4 section edges (one per lateral face)
        assert_eq!(result.edges.len(), 4, "expected 4 section edges, got {}", result.edges.len());
    }

    #[test]
    fn test_section_no_intersection() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let result = section_solid(
            &model,
            b.solid,
            Point3::new(0.0, 0.0, 10.0),
            Vec3::Z,
        )
        .unwrap();

        assert!(result.edges.is_empty());
    }

    #[test]
    fn test_section_validation() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        assert!(section_solid(&model, b.solid, Point3::ORIGIN, Vec3::ZERO).is_err());
    }
}
