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

/// Minimum distance between a ray and a line segment in 3D.
/// Returns the distance and the parameter `t` along the ray to the closest point.
fn ray_segment_distance(
    ray_o: [f32; 3],
    ray_d: [f32; 3],
    seg_a: [f32; 3],
    seg_b: [f32; 3],
) -> (f32, f32) {
    let u = [
        seg_b[0] - seg_a[0],
        seg_b[1] - seg_a[1],
        seg_b[2] - seg_a[2],
    ];
    let w = [
        ray_o[0] - seg_a[0],
        ray_o[1] - seg_a[1],
        ray_o[2] - seg_a[2],
    ];

    let a = dot(ray_d, ray_d);
    let b = dot(ray_d, u);
    let c = dot(u, u);
    let d = dot(ray_d, w);
    let e = dot(u, w);

    let denom = a * c - b * b;
    let (sc, tc) = if denom.abs() < 1e-10 {
        (0.0, if b > c { d / b } else { e / c })
    } else {
        let inv = 1.0 / denom;
        let s = (b * e - c * d) * inv;
        let t = (a * e - b * d) * inv;
        (s.max(0.0), t.clamp(0.0, 1.0))
    };

    let closest_ray = [
        ray_o[0] + ray_d[0] * sc,
        ray_o[1] + ray_d[1] * sc,
        ray_o[2] + ray_d[2] * sc,
    ];
    let closest_seg = [
        seg_a[0] + u[0] * tc,
        seg_a[1] + u[1] * tc,
        seg_a[2] + u[2] * tc,
    ];
    let diff = [
        closest_ray[0] - closest_seg[0],
        closest_ray[1] - closest_seg[1],
        closest_ray[2] - closest_seg[2],
    ];
    let dist = (diff[0] * diff[0] + diff[1] * diff[1] + diff[2] * diff[2]).sqrt();
    (dist, sc)
}

/// Pick the closest B-Rep edge to a ray.
///
/// `edges` is a list of `(start_position, end_position)` pairs for each edge.
/// `threshold` is the maximum 3D distance for a hit.
/// Returns `(edge_index, distance_to_ray, t_along_ray)`.
pub fn pick_edge(
    ray_origin: [f32; 3],
    ray_dir: [f32; 3],
    edges: &[([f32; 3], [f32; 3])],
    threshold: f32,
) -> Option<(usize, f32)> {
    let mut best: Option<(usize, f32, f32)> = None;

    for (i, (a, b)) in edges.iter().enumerate() {
        let (dist, t) = ray_segment_distance(ray_origin, ray_dir, *a, *b);
        if dist < threshold && t > 0.0 {
            let is_closer = best.as_ref().is_none_or(|(_, d, _)| dist < *d);
            if is_closer {
                best = Some((i, dist, t));
            }
        }
    }

    best.map(|(i, _dist, t)| (i, t))
}

/// Pick the closest vertex to a ray.
///
/// `vertices` is a list of 3D positions.
/// `threshold` is the maximum 3D distance from the ray for a hit.
/// Returns `(vertex_index, distance_along_ray)`.
pub fn pick_vertex(
    ray_origin: [f32; 3],
    ray_dir: [f32; 3],
    vertices: &[[f32; 3]],
    threshold: f32,
) -> Option<(usize, f32)> {
    let mut best: Option<(usize, f32, f32)> = None;

    for (i, v) in vertices.iter().enumerate() {
        // Project vertex onto ray: t = dot(v - origin, dir)
        let ov = [
            v[0] - ray_origin[0],
            v[1] - ray_origin[1],
            v[2] - ray_origin[2],
        ];
        let t = dot(ov, ray_dir);
        if t < 0.0 {
            continue; // behind camera
        }
        // Closest point on ray to vertex
        let cp = [
            ray_origin[0] + ray_dir[0] * t,
            ray_origin[1] + ray_dir[1] * t,
            ray_origin[2] + ray_dir[2] * t,
        ];
        let d = [cp[0] - v[0], cp[1] - v[1], cp[2] - v[2]];
        let dist = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        if dist < threshold {
            let is_closer = best.as_ref().is_none_or(|(_, _, prev_d)| dist < *prev_d);
            if is_closer {
                best = Some((i, t, dist));
            }
        }
    }

    best.map(|(i, t, _)| (i, t))
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

    let near = unproject(ndc_x, ndc_y, 0.0, inv_view_proj);
    let far = unproject(ndc_x, ndc_y, 1.0, inv_view_proj);

    let dir = [far[0] - near[0], far[1] - near[1], far[2] - near[2]];
    let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
    if len > 1e-8 {
        (
            [near[0], near[1], near[2]],
            [dir[0] / len, dir[1] / len, dir[2] / len],
        )
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

    /// End-to-end test: create a box, set up camera looking at it,
    /// compute the screen position of the box center, cast a pick ray,
    /// and verify it hits the box.
    #[test]
    fn test_pick_box_at_center() {
        use crate::render::Camera;
        use cadkernel_io::tessellate_solid;
        use cadkernel_modeling::make_box;
        use cadkernel_topology::BRepModel;

        // Create a 2x2x2 box centered at origin
        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::new(-1.0, -1.0, -1.0), 2.0, 2.0, 2.0).unwrap();
        let mesh = tessellate_solid(&model, r.solid);

        // Set up camera looking at origin from front (0 yaw, 0 pitch = +X axis)
        let mut camera = Camera::new(1440.0 / 900.0);
        camera.target = [0.0, 0.0, 0.0];
        camera.distance = 10.0;
        camera.yaw = 0.0;
        camera.pitch = 0.0;

        let inv_vp = camera.inv_view_proj();

        // Pick at screen center — should hit the box
        let (origin, dir) = screen_to_ray(720.0, 450.0, 1440.0, 900.0, inv_vp);
        let hit = pick_triangle(origin, dir, &mesh.vertices, &mesh.indices);
        assert!(hit.is_some(), "Pick at screen center should hit the box");

        // Pick far from center — should miss the box
        let (origin2, dir2) = screen_to_ray(1440.0, 0.0, 1440.0, 900.0, inv_vp);
        let miss = pick_triangle(origin2, dir2, &mesh.vertices, &mesh.indices);
        assert!(miss.is_none(), "Pick at far corner should miss the box");
    }

    /// Verify pick works at fractional scale factor (1.3x).
    /// Simulates 130% DPI: logical 1440x900 → physical 1872x1170.
    #[test]
    fn test_pick_box_at_130_percent_scale() {
        use crate::render::Camera;
        use cadkernel_io::tessellate_solid;
        use cadkernel_modeling::make_box;
        use cadkernel_topology::BRepModel;

        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::new(-1.0, -1.0, -1.0), 2.0, 2.0, 2.0).unwrap();
        let mesh = tessellate_solid(&model, r.solid);

        // Camera aspect uses PHYSICAL dimensions
        let mut camera = Camera::new(1872.0 / 1170.0);
        camera.target = [0.0, 0.0, 0.0];
        camera.distance = 10.0;
        camera.yaw = 0.0;
        camera.pitch = 0.0;

        let inv_vp = camera.inv_view_proj();

        // Physical center = 936, 585
        let (origin, dir) = screen_to_ray(936.0, 585.0, 1872.0, 1170.0, inv_vp);
        let hit = pick_triangle(origin, dir, &mesh.vertices, &mesh.indices);
        assert!(
            hit.is_some(),
            "Pick at physical center should hit box at 130% scale"
        );

        // If cursor coords were LOGICAL (720, 450) but used with PHYSICAL window size,
        // the NDC would be wrong: ndc_x = (2*720/1872)-1 = -0.231 ≠ 0
        let (origin_wrong, dir_wrong) = screen_to_ray(720.0, 450.0, 1872.0, 1170.0, inv_vp);
        let hit_wrong = pick_triangle(origin_wrong, dir_wrong, &mesh.vertices, &mesh.indices);
        // This may or may not hit depending on box angular size, but it targets the wrong pixel
        // The important thing is that the correct coords (936,585) DO hit
        let _ = hit_wrong; // just checking correct coords work
    }

    /// Project a world point to screen, then screen_to_ray back, verify the ray
    /// passes through the original point. Tests default camera (yaw=0.8, pitch=0.4).
    #[test]
    fn test_project_unproject_roundtrip() {
        use crate::render::Camera;

        let mut camera = Camera::new(1440.0 / 900.0);
        camera.target = [0.0, 0.0, 0.0];
        camera.distance = 20.0;
        camera.yaw = 0.8;
        camera.pitch = 0.4;

        let vp = camera.view_proj();
        let inv_vp = camera.inv_view_proj();
        let (w, h) = (1440.0_f32, 900.0_f32);

        // Test multiple world points
        let world_points: &[[f32; 3]] = &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [-2.0, 3.0, 1.0],
        ];

        for &wp in world_points {
            // Project world → clip
            let cx = vp[0][0] * wp[0] + vp[1][0] * wp[1] + vp[2][0] * wp[2] + vp[3][0];
            let cy = vp[0][1] * wp[0] + vp[1][1] * wp[1] + vp[2][1] * wp[2] + vp[3][1];
            let cw = vp[0][3] * wp[0] + vp[1][3] * wp[1] + vp[2][3] * wp[2] + vp[3][3];

            // clip → NDC
            let ndc_x = cx / cw;
            let ndc_y = cy / cw;

            // NDC → screen
            let sx = (ndc_x + 1.0) * 0.5 * w;
            let sy = (1.0 - ndc_y) * 0.5 * h;

            // Screen → ray
            let (origin, dir) = screen_to_ray(sx, sy, w, h, inv_vp);

            // Ray should pass near the original world point.
            let to_wp = [wp[0] - origin[0], wp[1] - origin[1], wp[2] - origin[2]];
            let t = dot(to_wp, dir);
            assert!(t > 0.0, "Point {wp:?} should be in front of camera (t={t})");

            // Closest point on ray to wp
            let closest = [
                origin[0] + dir[0] * t,
                origin[1] + dir[1] * t,
                origin[2] + dir[2] * t,
            ];
            let err = [closest[0] - wp[0], closest[1] - wp[1], closest[2] - wp[2]];
            let dist = (err[0] * err[0] + err[1] * err[1] + err[2] * err[2]).sqrt();
            assert!(
                dist < 0.01,
                "Roundtrip error {dist:.6} for point {wp:?} (screen {sx:.1},{sy:.1})"
            );
        }
    }

    /// Verify VP * inv_VP = Identity for the default camera.
    #[test]
    fn test_vp_inverse_identity() {
        use crate::render::Camera;

        let mut camera = Camera::new(1440.0 / 900.0);
        camera.target = [0.0, 0.0, 0.0];
        camera.distance = 20.0;
        camera.yaw = 0.8;
        camera.pitch = 0.4;

        let vp = camera.view_proj();
        let inv_vp = camera.inv_view_proj();

        let prod = mat4_mul_test(vp, inv_vp);
        for (r, _) in prod[0].iter().enumerate() {
            for (c, col) in prod.iter().enumerate() {
                let expected = if r == c { 1.0 } else { 0.0 };
                let actual = col[r]; // column-major
                assert!(
                    (actual - expected).abs() < 1e-3,
                    "VP*inv_VP[{r}][{c}] = {actual}, expected {expected}"
                );
            }
        }
    }

    fn mat4_mul_test(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
        let mut out = [[0.0f32; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                out[i][j] = (0..4).map(|k| a[k][j] * b[i][k]).sum();
            }
        }
        out
    }

    /// Pick a box with default isometric camera at multiple screen positions.
    #[test]
    fn test_pick_box_default_camera() {
        use crate::render::Camera;
        use cadkernel_io::tessellate_solid;
        use cadkernel_modeling::make_box;
        use cadkernel_topology::BRepModel;

        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::new(-1.0, -1.0, -1.0), 2.0, 2.0, 2.0).unwrap();
        let mesh = tessellate_solid(&model, r.solid);

        let mut camera = Camera::new(1440.0 / 900.0);
        camera.target = [0.0, 0.0, 0.0];
        camera.distance = 10.0;
        camera.yaw = 0.8;
        camera.pitch = 0.4;

        let vp = camera.view_proj();
        let inv_vp = camera.inv_view_proj();
        let (w, h) = (1440.0_f32, 900.0_f32);

        // Project the box center (0,0,0) to screen
        let cw = vp[0][3] * 0.0 + vp[1][3] * 0.0 + vp[2][3] * 0.0 + vp[3][3];
        let cx = vp[3][0] / cw;
        let cy = vp[3][1] / cw;
        let center_sx = (cx + 1.0) * 0.5 * w;
        let center_sy = (1.0 - cy) * 0.5 * h;

        // Pick at projected box center — must hit
        let (origin, dir) = screen_to_ray(center_sx, center_sy, w, h, inv_vp);
        let hit = pick_triangle(origin, dir, &mesh.vertices, &mesh.indices);
        assert!(
            hit.is_some(),
            "Pick at box center ({center_sx:.0},{center_sy:.0}) must hit"
        );
    }
}
