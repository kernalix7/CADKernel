//! Project curves onto solids.

use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, Handle, SolidData};

/// Projects a polyline onto the nearest faces of a solid.
///
/// For each input point, the closest point on the solid's tessellated
/// surface is returned. Points are projected independently.
pub fn project_curve_on_solid(
    model: &BRepModel,
    solid: Handle<SolidData>,
    points: &[Point3],
) -> Vec<Point3> {
    let triangles = collect_solid_triangles(model, solid);
    points
        .iter()
        .map(|p| closest_point_on_triangles(*p, &triangles))
        .collect()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn collect_solid_triangles(model: &BRepModel, solid: Handle<SolidData>) -> Vec<[Point3; 3]> {
    let mut tris = Vec::new();
    let Some(sd) = model.solids.get(solid) else {
        return tris;
    };
    for &shell_h in &sd.shells {
        let Some(sh) = model.shells.get(shell_h) else {
            continue;
        };
        for &face_h in &sh.faces {
            for tri in cadkernel_io::tessellate_face(model, face_h) {
                tris.push(tri.vertices);
            }
        }
    }
    tris
}

fn closest_point_on_triangles(p: Point3, triangles: &[[Point3; 3]]) -> Point3 {
    let mut best = p;
    let mut best_dist = f64::MAX;
    for tri in triangles {
        let cp = closest_point_on_triangle(p, tri[0], tri[1], tri[2]);
        let d = p - cp;
        let dist_sq = d.x * d.x + d.y * d.y + d.z * d.z;
        if dist_sq < best_dist {
            best_dist = dist_sq;
            best = cp;
        }
    }
    best
}

fn closest_point_on_triangle(p: Point3, a: Point3, b: Point3, c: Point3) -> Point3 {
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;

    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }

    let bp = p - b;
    let d3 = dot(ab, bp);
    let d4 = dot(ac, bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }

    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return Point3::new(a.x + v * ab.x, a.y + v * ab.y, a.z + v * ab.z);
    }

    let cp_v = p - c;
    let d5 = dot(ab, cp_v);
    let d6 = dot(ac, cp_v);
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

fn dot(a: Vec3, b: Vec3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_math::Point3;

    #[test]
    fn test_project_curve_on_box() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        // Points above the box should project onto the top face (z=2)
        let points = vec![
            Point3::new(1.0, 1.0, 5.0),
            Point3::new(0.5, 0.5, 10.0),
        ];
        let projected = project_curve_on_solid(&model, b.solid, &points);
        assert_eq!(projected.len(), 2);
        for p in &projected {
            // Should project onto the box surface, z should be <= 2 + epsilon
            assert!(p.z <= 2.0 + 0.1, "projected z = {} should be <= 2", p.z);
        }
    }
}
