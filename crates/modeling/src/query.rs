//! Spatial queries for B-Rep solids.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, FaceData, Handle, SolidData, VertexData};

/// Result of a closest-point query.
#[derive(Debug, Clone)]
pub struct ClosestPointResult {
    pub point: Point3,
    pub distance: f64,
}

/// Containment classification of a point with respect to a solid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Containment {
    Inside,
    Outside,
    OnBoundary,
}

/// Finds the closest point on a solid's surface to the given query point.
pub fn closest_point_on_solid(
    model: &BRepModel,
    solid: Handle<SolidData>,
    point: Point3,
) -> KernelResult<ClosestPointResult> {
    let faces = collect_solid_faces(model, solid)?;
    let mut best_dist = f64::INFINITY;
    let mut best_pt = point;

    for &fh in &faces {
        let verts = model.vertices_of_face(fh)?;
        if verts.len() < 3 {
            continue;
        }
        // Triangulate face and find closest point on each triangle
        let pts: Vec<Point3> = verts
            .iter()
            .map(|&vh| vertex_point(model, vh))
            .collect::<KernelResult<Vec<_>>>()?;

        for i in 1..pts.len() - 1 {
            let cp = closest_point_on_triangle(point, pts[0], pts[i], pts[i + 1]);
            let d = point.distance_to(cp);
            if d < best_dist {
                best_dist = d;
                best_pt = cp;
            }
        }
    }

    Ok(ClosestPointResult {
        point: best_pt,
        distance: best_dist,
    })
}

/// Classifies whether a point is inside, outside, or on the boundary of a solid.
///
/// Uses ray casting along +X direction and counts intersections with faces.
pub fn point_in_solid(
    model: &BRepModel,
    solid: Handle<SolidData>,
    point: Point3,
) -> KernelResult<Containment> {
    let faces = collect_solid_faces(model, solid)?;
    let boundary_eps = 1e-8;

    // Check if point is on boundary first
    let closest = closest_point_on_solid(model, solid, point)?;
    if closest.distance < boundary_eps {
        return Ok(Containment::OnBoundary);
    }

    // Use non-axis-aligned direction to avoid edge/face coincidence
    let ray_dir = Vec3::new(1.0, 0.31, 0.37)
        .normalized()
        .unwrap_or(Vec3::X);
    let mut crossings = 0u32;

    for &fh in &faces {
        let verts = model.vertices_of_face(fh)?;
        if verts.len() < 3 {
            continue;
        }
        let pts: Vec<Point3> = verts
            .iter()
            .map(|&vh| vertex_point(model, vh))
            .collect::<KernelResult<Vec<_>>>()?;

        for i in 1..pts.len() - 1 {
            if ray_intersects_triangle(point, ray_dir, pts[0], pts[i], pts[i + 1]) {
                crossings += 1;
            }
        }
    }

    if crossings % 2 == 1 {
        Ok(Containment::Inside)
    } else {
        Ok(Containment::Outside)
    }
}

fn vertex_point(model: &BRepModel, v: Handle<VertexData>) -> KernelResult<Point3> {
    Ok(model
        .vertices
        .get(v)
        .ok_or(KernelError::InvalidHandle("vertex"))?
        .point)
}

fn collect_solid_faces(
    model: &BRepModel,
    solid: Handle<SolidData>,
) -> KernelResult<Vec<Handle<FaceData>>> {
    let sd = model
        .solids
        .get(solid)
        .ok_or(KernelError::InvalidHandle("solid"))?;
    let mut faces = Vec::new();
    for &shell_h in &sd.shells {
        let sh = model
            .shells
            .get(shell_h)
            .ok_or(KernelError::InvalidHandle("shell"))?;
        faces.extend_from_slice(&sh.faces);
    }
    Ok(faces)
}

/// Closest point on triangle ABC to point P.
fn closest_point_on_triangle(p: Point3, a: Point3, b: Point3, c: Point3) -> Point3 {
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;

    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }

    let bp = p - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }

    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return Point3::new(a.x + v * ab.x, a.y + v * ab.y, a.z + v * ab.z);
    }

    let cp = p - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }

    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return Point3::new(a.x + w * ac.x, a.y + w * ac.y, a.z + w * ac.z);
    }

    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        let bc = c - b;
        return Point3::new(b.x + w * bc.x, b.y + w * bc.y, b.z + w * bc.z);
    }

    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    Point3::new(
        a.x + ab.x * v + ac.x * w,
        a.y + ab.y * v + ac.y * w,
        a.z + ab.z * v + ac.z * w,
    )
}

/// Moller-Trumbore ray-triangle intersection test.
fn ray_intersects_triangle(origin: Point3, dir: Vec3, v0: Point3, v1: Point3, v2: Point3) -> bool {
    let eps = 1e-12;
    let e1 = v1 - v0;
    let e2 = v2 - v0;
    let h = dir.cross(e2);
    let a = e1.dot(h);

    if a.abs() < eps {
        return false;
    }

    let f = 1.0 / a;
    let s = origin - v0;
    let u = f * s.dot(h);
    if !(0.0..=1.0).contains(&u) {
        return false;
    }

    let q = s.cross(e1);
    let v = f * dir.dot(q);
    if v < 0.0 || u + v > 1.0 {
        return false;
    }

    let t = f * e2.dot(q);
    t > eps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_closest_point_on_box() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let result =
            closest_point_on_solid(&model, b.solid, Point3::new(1.0, 1.0, 5.0)).unwrap();
        assert!((result.distance - 3.0).abs() < 1e-6);
        assert!((result.point.z - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_point_inside_box() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let c = point_in_solid(&model, b.solid, Point3::new(1.0, 1.0, 1.0)).unwrap();
        assert_eq!(c, Containment::Inside);
    }

    #[test]
    fn test_point_outside_box() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let c = point_in_solid(&model, b.solid, Point3::new(5.0, 5.0, 5.0)).unwrap();
        assert_eq!(c, Containment::Outside);
    }

    #[test]
    fn test_point_on_boundary() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let c = point_in_solid(&model, b.solid, Point3::new(1.0, 1.0, 2.0)).unwrap();
        assert_eq!(c, Containment::OnBoundary);
    }

    #[test]
    fn test_closest_point_on_triangle() {
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);

        // Point above center
        let cp = closest_point_on_triangle(Point3::new(0.25, 0.25, 1.0), a, b, c);
        assert!((cp.z).abs() < 1e-10);
        assert!((cp.x - 0.25).abs() < 1e-10);
        assert!((cp.y - 0.25).abs() < 1e-10);
    }
}
