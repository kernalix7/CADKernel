//! Shape analysis utilities for classifying solids and finding face types.

use cadkernel_math::Vec3;
use cadkernel_topology::{BRepModel, FaceData, Handle, SolidData};

/// Classification of a solid based on geometry heuristics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolidType {
    Box,
    Cylinder,
    Sphere,
    Cone,
    Torus,
    Prism,
    General,
}

/// Collects all face handles of a solid.
fn faces_of_solid(model: &BRepModel, solid: Handle<SolidData>) -> Vec<Handle<FaceData>> {
    let Some(sd) = model.solids.get(solid) else {
        return Vec::new();
    };
    let mut faces = Vec::new();
    for &shell_h in &sd.shells {
        if let Some(shell) = model.shells.get(shell_h) {
            faces.extend_from_slice(&shell.faces);
        }
    }
    faces
}

/// Collects vertex positions for a face's outer loop.
fn face_vertex_positions(model: &BRepModel, face: Handle<FaceData>) -> Vec<cadkernel_math::Point3> {
    let Ok(verts) = model.vertices_of_face(face) else {
        return Vec::new();
    };
    verts
        .iter()
        .filter_map(|&vh| model.vertices.get(vh).map(|v| v.point))
        .collect()
}

/// Finds faces whose outer-loop vertices are coplanar (within tolerance).
///
/// A face is considered planar if all its vertices lie within `1e-6` distance
/// of the plane defined by the first three non-collinear vertices.
pub fn find_planar_faces(
    model: &BRepModel,
    solid: Handle<SolidData>,
) -> Vec<Handle<FaceData>> {
    const TOL: f64 = 1e-6;
    let all_faces = faces_of_solid(model, solid);
    let mut result = Vec::new();

    for face_h in all_faces {
        let pts = face_vertex_positions(model, face_h);
        if pts.len() < 3 {
            continue;
        }

        // Find normal from the first three non-collinear vertices.
        let mut normal = None;
        for i in 1..pts.len() - 1 {
            let v1 = pts[i] - pts[0];
            let v2 = pts[i + 1] - pts[0];
            let n = v1.cross(v2);
            if n.length() > 1e-12 {
                let len = n.length();
                normal = Some(Vec3::new(n.x / len, n.y / len, n.z / len));
                break;
            }
        }

        let Some(n) = normal else {
            // All collinear — degenerate, consider planar.
            result.push(face_h);
            continue;
        };

        let d = n.dot(Vec3::new(pts[0].x, pts[0].y, pts[0].z));
        let mut is_planar = true;
        for pt in &pts[1..] {
            let dist = (n.dot(Vec3::new(pt.x, pt.y, pt.z)) - d).abs();
            if dist > TOL {
                is_planar = false;
                break;
            }
        }

        if is_planar {
            result.push(face_h);
        }
    }

    result
}

/// Finds faces whose outer-loop vertices lie on a cylinder (heuristic).
///
/// A face is considered cylindrical if all its vertices are equidistant (within
/// tolerance) from a candidate axis. The axis is estimated from the first two
/// vertices (the midpoint-to-centroid direction).
pub fn find_cylindrical_faces(
    model: &BRepModel,
    solid: Handle<SolidData>,
) -> Vec<Handle<FaceData>> {
    const TOL: f64 = 1e-3;
    let all_faces = faces_of_solid(model, solid);

    // Compute solid centroid from all unique vertices
    let mut all_pts = Vec::new();
    for &fh in &all_faces {
        all_pts.extend(face_vertex_positions(model, fh));
    }
    if all_pts.is_empty() {
        return Vec::new();
    }
    let total = all_pts.len() as f64;
    let scx = all_pts.iter().map(|p| p.x).sum::<f64>() / total;
    let scy = all_pts.iter().map(|p| p.y).sum::<f64>() / total;
    let scz = all_pts.iter().map(|p| p.z).sum::<f64>() / total;

    let axes = [
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
    ];

    let mut result = Vec::new();

    for face_h in all_faces {
        let pts = face_vertex_positions(model, face_h);
        if pts.len() < 3 {
            continue;
        }

        let mut found = false;
        for axis in &axes {
            // Distance of each vertex from the axis through solid centroid
            let mut distances = Vec::with_capacity(pts.len());
            for pt in &pts {
                let v = Vec3::new(pt.x - scx, pt.y - scy, pt.z - scz);
                let along = axis.dot(v);
                let perp = v - *axis * along;
                distances.push(perp.length());
            }

            if distances.is_empty() {
                continue;
            }

            let avg_r = distances.iter().sum::<f64>() / distances.len() as f64;
            if avg_r < TOL {
                continue; // On the axis, not cylindrical.
            }

            let all_equidistant = distances.iter().all(|&d| (d - avg_r).abs() / avg_r < TOL);
            if all_equidistant {
                found = true;
                break;
            }
        }

        if found {
            result.push(face_h);
        }
    }

    result
}

/// Classifies a solid as a known shape type based on face count and geometry.
///
/// Heuristics:
/// - 6 planar faces → `Box`
/// - 3 faces (2 planar + 1 cylindrical) → `Cylinder`
/// - All non-planar with ≈ many faces → `Sphere`
/// - 3 faces (1 planar + 1 conical + 1 maybe planar) → `Cone`
/// - Otherwise → `General`
pub fn classify_solid(model: &BRepModel, solid: Handle<SolidData>) -> SolidType {
    let all_faces = faces_of_solid(model, solid);
    let total = all_faces.len();
    let planar = find_planar_faces(model, solid);
    let cylindrical = find_cylindrical_faces(model, solid);
    let planar_count = planar.len();
    let cylindrical_count = cylindrical.len();

    if total == 6 && planar_count == 6 {
        return SolidType::Box;
    }

    // Cylinder: 2 planar caps + N lateral faces with cylindrical character
    if planar_count == 2 && cylindrical_count > 0 && total > 2 {
        return SolidType::Cylinder;
    }

    // Cone: 1 planar base + lateral faces
    if planar_count == 1 && cylindrical_count >= 1 && total > 2 {
        return SolidType::Cone;
    }

    // Prism vs Cylinder: all planar, many faces
    // If all vertices lie on a cylindrical surface (equidistant from a principal axis),
    // it's a tessellated cylinder, not a prism.
    if planar_count == total && total >= 5 && total != 6 {
        if cylindrical_count > 0 {
            return SolidType::Cylinder;
        }
        return SolidType::Prism;
    }

    // Torus: no planar faces, many faces, some cylindrical
    if planar_count == 0 && cylindrical_count > 0 && total > 6 {
        return SolidType::Torus;
    }

    // Sphere: no planar faces, many faces
    if planar_count == 0 && total > 6 {
        return SolidType::Sphere;
    }

    SolidType::General
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::{make_box, make_cylinder};
    use cadkernel_math::Point3;

    #[test]
    fn test_find_planar_faces_box() {
        let mut model = BRepModel::new();
        let bx = make_box(&mut model, Point3::new(0.0, 0.0, 0.0), 2.0, 3.0, 4.0).unwrap();
        let planar = find_planar_faces(&model, bx.solid);
        assert_eq!(planar.len(), 6);
    }

    #[test]
    fn test_classify_box() {
        let mut model = BRepModel::new();
        let bx = make_box(&mut model, Point3::new(0.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap();
        assert_eq!(classify_solid(&model, bx.solid), SolidType::Box);
    }

    #[test]
    fn test_classify_cylinder() {
        let mut model = BRepModel::new();
        let cyl = make_cylinder(
            &mut model,
            Point3::new(0.0, 0.0, 0.0),
            1.0,
            3.0,
            64,
        )
        .unwrap();
        let st = classify_solid(&model, cyl.solid);
        assert_eq!(st, SolidType::Cylinder);
    }
}
