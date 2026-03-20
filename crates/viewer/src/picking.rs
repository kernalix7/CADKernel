//! CPU-based ray-triangle picking for 3D viewport entity selection.

use cadkernel_math::Point3;

/// Result of a ray pick operation.
#[derive(Debug, Clone)]
pub struct PickResult {
    /// Index of the hit triangle in the mesh.
    pub triangle_index: usize,
    /// Distance from ray origin to hit point.
    pub distance: f32,
    /// 3D hit point on the triangle.
    pub hit_point: [f32; 3],
}

/// Moller-Trumbore ray-triangle intersection test.
///
/// Returns the distance `t` along the ray if the ray hits the triangle, or None.
fn ray_triangle_intersect(
    origin: [f32; 3],
    dir: [f32; 3],
    v0: [f32; 3],
    v1: [f32; 3],
    v2: [f32; 3],
) -> Option<f32> {
    let e1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
    let e2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

    let h = cross(dir, e2);
    let a = dot(e1, h);
    if a.abs() < 1e-8 {
        return None;
    }

    let f = 1.0 / a;
    let s = [origin[0] - v0[0], origin[1] - v0[1], origin[2] - v0[2]];
    let u = f * dot(s, h);
    if !(0.0..=1.0).contains(&u) {
        return None;
    }

    let q = cross(s, e1);
    let v = f * dot(dir, q);
    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    let t = f * dot(e2, q);
    if t > 1e-6 { Some(t) } else { None }
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Pick the closest triangle hit by a ray through the mesh.
///
/// `vertices` are the mesh vertices (Point3), `indices` are triangle index triples.
/// Returns the closest hit triangle index and distance.
pub fn pick_triangle(
    ray_origin: [f32; 3],
    ray_dir: [f32; 3],
    vertices: &[Point3],
    indices: &[[u32; 3]],
) -> Option<PickResult> {
    let mut best: Option<PickResult> = None;

    for (ti, tri) in indices.iter().enumerate() {
        let v0 = pt_to_f32(vertices[tri[0] as usize]);
        let v1 = pt_to_f32(vertices[tri[1] as usize]);
        let v2 = pt_to_f32(vertices[tri[2] as usize]);

        if let Some(t) = ray_triangle_intersect(ray_origin, ray_dir, v0, v1, v2) {
            let is_closer = best.as_ref().is_none_or(|b| t < b.distance);
            if is_closer {
                best = Some(PickResult {
                    triangle_index: ti,
                    distance: t,
                    hit_point: [
                        ray_origin[0] + ray_dir[0] * t,
                        ray_origin[1] + ray_dir[1] * t,
                        ray_origin[2] + ray_dir[2] * t,
                    ],
                });
            }
        }
    }

    best
}

fn pt_to_f32(p: Point3) -> [f32; 3] {
    [p.x as f32, p.y as f32, p.z as f32]
}

/// Convert screen coordinates to a world-space ray using inverse view-projection.
///
/// `screen_x`, `screen_y` are pixel coordinates (top-left origin).
/// Returns `(origin, direction)` in world space.
pub fn screen_to_ray(
    screen_x: f32,
    screen_y: f32,
    width: f32,
    height: f32,
    inv_view_proj: [[f32; 4]; 4],
) -> ([f32; 3], [f32; 3]) {
    // Convert to NDC [-1, 1]
    let ndc_x = (2.0 * screen_x / width) - 1.0;
    let ndc_y = 1.0 - (2.0 * screen_y / height); // flip Y

    let near = unproject(ndc_x, ndc_y, -1.0, inv_view_proj);
    let far = unproject(ndc_x, ndc_y, 1.0, inv_view_proj);

    let dir = [far[0] - near[0], far[1] - near[1], far[2] - near[2]];
    let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
    if len > 1e-8 {
        ([near[0], near[1], near[2]], [dir[0] / len, dir[1] / len, dir[2] / len])
    } else {
        (near, [0.0, 0.0, -1.0])
    }
}

fn unproject(x: f32, y: f32, z: f32, m: [[f32; 4]; 4]) -> [f32; 3] {
    let w = m[0][3] * x + m[1][3] * y + m[2][3] * z + m[3][3];
    let inv_w = if w.abs() > 1e-10 { 1.0 / w } else { 1.0 };
    [
        (m[0][0] * x + m[1][0] * y + m[2][0] * z + m[3][0]) * inv_w,
        (m[0][1] * x + m[1][1] * y + m[2][1] * z + m[3][1]) * inv_w,
        (m[0][2] * x + m[1][2] * y + m[2][2] * z + m[3][2]) * inv_w,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ray_triangle_hit() {
        let origin = [0.0, 0.0, 5.0];
        let dir = [0.0, 0.0, -1.0];
        let v0 = [-1.0, -1.0, 0.0];
        let v1 = [1.0, -1.0, 0.0];
        let v2 = [0.0, 1.0, 0.0];
        let t = ray_triangle_intersect(origin, dir, v0, v1, v2);
        assert!(t.is_some());
        assert!((t.unwrap() - 5.0).abs() < 1e-5);
    }

    #[test]
    fn test_ray_triangle_miss() {
        let origin = [5.0, 5.0, 5.0];
        let dir = [0.0, 0.0, -1.0];
        let v0 = [-1.0, -1.0, 0.0];
        let v1 = [1.0, -1.0, 0.0];
        let v2 = [0.0, 1.0, 0.0];
        assert!(ray_triangle_intersect(origin, dir, v0, v1, v2).is_none());
    }

    #[test]
    fn test_pick_closest() {
        let vertices = vec![
            Point3::new(-1.0, -1.0, 0.0),
            Point3::new(1.0, -1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(-1.0, -1.0, -5.0),
            Point3::new(1.0, -1.0, -5.0),
            Point3::new(0.0, 1.0, -5.0),
        ];
        let indices = vec![[0, 1, 2], [3, 4, 5]];
        let result = pick_triangle([0.0, 0.0, 10.0], [0.0, 0.0, -1.0], &vertices, &indices);
        assert!(result.is_some());
        assert_eq!(result.unwrap().triangle_index, 0); // closer triangle
    }
}
