//! Gate #6 — picking accuracy at multiple camera distances.
//!
//! Locks in the invariant that face / edge / vertex picking returns a stable,
//! well-defined hit on a known B-Rep box regardless of camera distance. The
//! tests exercise the same low-level pipeline the GUI uses when the user
//! clicks the viewport:
//!
//!   1. `screen_to_ray` builds a world-space ray from cursor pixel + camera.
//!   2. `pick_vertex` / `pick_edge` / `pick_triangle` test that ray against
//!      the SceneObject's `vertex_positions` / `edge_positions` / `mesh`.
//!
//! `pick_auto` in `app.rs` calls these same three functions with thresholds
//! that scale with `camera.distance` (vertex 0.012, edge 0.015 in world units
//! per unit of camera distance). The tests use the same scaling so that the
//! threshold is large enough at far distances but tight enough at near ones.
//!
//! Box dimensions are 100 x 50 x 25, centered around (50, 25, 12.5). The +X
//! face of the box lives at x = 100. The camera looks down the +X axis from
//! (100 + d, 25, 12.5) toward the box center.

use cadkernel_math::Point3;
use cadkernel_modeling::make_box;
use cadkernel_topology::BRepModel;
use cadkernel_viewer::{
    Camera,
    picking::{pick_edge, pick_triangle, pick_vertex, screen_to_ray},
    scene::{CreationParams, Scene},
};

/// Width (X), height (Y), depth (Z) of the test box.
const W: f64 = 100.0;
const H: f64 = 50.0;
const D: f64 = 25.0;

/// Threshold scaling factors that mirror `app.rs::try_pick_entity`.
const VERT_FACTOR: f32 = 0.012;
const EDGE_FACTOR: f32 = 0.015;

const SCREEN_W: f32 = 1440.0;
const SCREEN_H: f32 = 900.0;

/// Build the scene and a camera positioned `gap` units away from the +X face
/// of the box (so the eye sits at (W + gap, cy, cz) looking down -X toward
/// the box centre). Returns the screen-centre ray and per-distance pick
/// thresholds, where the threshold scales with `camera.distance` exactly as
/// `app.rs::try_pick_entity` does in production.
fn setup(gap: f32) -> (Scene, Camera, [f32; 3], [f32; 3], f32, f32) {
    let mut scene = Scene::new();
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::ORIGIN, W, H, D).expect("make_box");
    let params = CreationParams::Box {
        width: W,
        height: H,
        depth: D,
    };
    let _id = scene.add_object("Box", model, r.solid, Some(params), None);

    let cx = (W * 0.5) as f32;
    let cy = (H * 0.5) as f32;
    let cz = (D * 0.5) as f32;

    // Eye sits on +X at (W + gap, cy, cz); `camera.distance` is the eye-to-
    // target distance, which equals W/2 + gap because target is the centre.
    let distance = (W as f32) * 0.5 + gap;
    let mut camera = Camera::new(SCREEN_W / SCREEN_H);
    camera.target = [cx, cy, cz];
    camera.distance = distance;
    camera.yaw = 0.0;
    camera.pitch = 0.0;
    camera.znear = 0.01;
    camera.zfar = (distance * 10.0).max(10000.0);

    let inv_vp = camera.inv_view_proj();
    let (origin, dir) = screen_to_ray(SCREEN_W * 0.5, SCREEN_H * 0.5, SCREEN_W, SCREEN_H, inv_vp);

    let vert_threshold = camera.distance * VERT_FACTOR;
    let edge_threshold = camera.distance * EDGE_FACTOR;
    (scene, camera, origin, dir, vert_threshold, edge_threshold)
}

/// `pick_triangle` at a given distance must return a hit on a face whose
/// outward normal faces the camera (the +X face for a +X camera).
fn assert_face_hit(scene: &Scene, origin: [f32; 3], dir: [f32; 3], distance: f32) {
    let obj = scene.objects.first().expect("scene object");
    let hit = pick_triangle(origin, dir, &obj.mesh.vertices, &obj.mesh.indices);
    assert!(
        hit.is_some(),
        "pick_triangle must return a hit at distance {distance}"
    );
    let hit = hit.unwrap();
    // Ray origin lives on +X side at x = W + distance; closest face is the
    // +X face at x = 100. Hit point x must be near W (within tessellation
    // precision and small floating drift).
    assert!(
        (hit.hit_point[0] - W as f32).abs() < 1e-2,
        "expected hit on +X face (x~{W}), got hit_point.x={}, distance={distance}",
        hit.hit_point[0]
    );
    // Triangle index must map to a face handle.
    let face_handle = lookup_face(hit.triangle_index, &obj.face_tri_map);
    assert!(
        face_handle.is_some(),
        "triangle {} must belong to a B-Rep face at distance {distance}",
        hit.triangle_index
    );
}

fn lookup_face(
    tri_index: usize,
    face_tri_map: &[(
        cadkernel_topology::Handle<cadkernel_topology::FaceData>,
        usize,
        usize,
    )],
) -> Option<cadkernel_topology::Handle<cadkernel_topology::FaceData>> {
    for &(fh, start, count) in face_tri_map {
        if tri_index >= start && tri_index < start + count {
            return Some(fh);
        }
    }
    None
}

/// `pick_edge` at a given distance must find at least one edge along a ray
/// aimed at a box corner. The returned edge handle must be in `edge_handles`.
fn assert_edge_hit_near_corner(scene: &Scene, camera: &Camera, edge_threshold: f32, distance: f32) {
    // Aim the ray at the (W, 0, 0) corner — slightly off-screen-centre. Project
    // that world point to screen, then build the ray from those pixel coords.
    let corner = [W as f32, 0.0, 0.0];
    let (sx, sy) = project_to_screen(corner, camera.view_proj());
    let (origin, dir) = screen_to_ray(sx, sy, SCREEN_W, SCREEN_H, camera.inv_view_proj());

    let obj = scene.objects.first().unwrap();
    let hit = pick_edge(origin, dir, &obj.edge_positions, edge_threshold);
    assert!(
        hit.is_some(),
        "pick_edge must return an edge near (W,0,0) at distance {distance} (threshold={edge_threshold})"
    );
    let (idx, _t) = hit.unwrap();
    assert!(
        obj.edge_handles.get(idx).is_some(),
        "edge index {idx} must map to a Handle at distance {distance}"
    );
}

/// `pick_vertex` at a given distance must find a vertex when aiming at a
/// known corner.
fn assert_vertex_hit_at_corner(scene: &Scene, camera: &Camera, vert_threshold: f32, distance: f32) {
    let corner = [W as f32, 0.0, 0.0];
    let (sx, sy) = project_to_screen(corner, camera.view_proj());
    let (origin, dir) = screen_to_ray(sx, sy, SCREEN_W, SCREEN_H, camera.inv_view_proj());

    let obj = scene.objects.first().unwrap();
    let hit = pick_vertex(origin, dir, &obj.vertex_positions, vert_threshold);
    assert!(
        hit.is_some(),
        "pick_vertex must return a vertex at (W,0,0) at distance {distance} (threshold={vert_threshold})"
    );
    let (idx, _t) = hit.unwrap();
    let pos = obj.vertex_positions[idx];
    // The closest vertex to (W, 0, 0) must be on the +X face, within
    // tessellation tolerance.
    assert!(
        (pos[0] - W as f32).abs() < 1e-3,
        "picked vertex x={} should be ~{W}",
        pos[0]
    );
    assert!(
        obj.vertex_handles.get(idx).is_some(),
        "vertex index {idx} must map to a Handle at distance {distance}"
    );
}

fn project_to_screen(world: [f32; 3], vp: [[f32; 4]; 4]) -> (f32, f32) {
    let cx = vp[0][0] * world[0] + vp[1][0] * world[1] + vp[2][0] * world[2] + vp[3][0];
    let cy = vp[0][1] * world[0] + vp[1][1] * world[1] + vp[2][1] * world[2] + vp[3][1];
    let cw = vp[0][3] * world[0] + vp[1][3] * world[1] + vp[2][3] * world[2] + vp[3][3];
    let ndc_x = cx / cw;
    let ndc_y = cy / cw;
    let sx = (ndc_x + 1.0) * 0.5 * SCREEN_W;
    let sy = (1.0 - ndc_y) * 0.5 * SCREEN_H;
    (sx, sy)
}

// ---------------------------------------------------------------------------
// 4 distance tiers x 3 entity types = 12 assertions across these tests.
// ---------------------------------------------------------------------------

#[test]
fn pick_face_at_distance_1m() {
    let (scene, _, origin, dir, _, _) = setup(1.0);
    assert_face_hit(&scene, origin, dir, 1.0);
}

#[test]
fn pick_face_at_distance_10m() {
    let (scene, _, origin, dir, _, _) = setup(10.0);
    assert_face_hit(&scene, origin, dir, 10.0);
}

#[test]
fn pick_face_at_distance_100m() {
    let (scene, _, origin, dir, _, _) = setup(100.0);
    assert_face_hit(&scene, origin, dir, 100.0);
}

#[test]
fn pick_face_at_distance_1000m() {
    let (scene, _, origin, dir, _, _) = setup(1000.0);
    assert_face_hit(&scene, origin, dir, 1000.0);
}

#[test]
fn pick_edge_at_distance_1m() {
    let (scene, camera, _, _, _, edge_t) = setup(1.0);
    assert_edge_hit_near_corner(&scene, &camera, edge_t, 1.0);
}

#[test]
fn pick_edge_at_distance_10m() {
    let (scene, camera, _, _, _, edge_t) = setup(10.0);
    assert_edge_hit_near_corner(&scene, &camera, edge_t, 10.0);
}

#[test]
fn pick_edge_at_distance_100m() {
    let (scene, camera, _, _, _, edge_t) = setup(100.0);
    assert_edge_hit_near_corner(&scene, &camera, edge_t, 100.0);
}

#[test]
fn pick_edge_at_distance_1000m() {
    let (scene, camera, _, _, _, edge_t) = setup(1000.0);
    assert_edge_hit_near_corner(&scene, &camera, edge_t, 1000.0);
}

#[test]
fn pick_vertex_at_distance_1m() {
    let (scene, camera, _, _, vert_t, _) = setup(1.0);
    assert_vertex_hit_at_corner(&scene, &camera, vert_t, 1.0);
}

#[test]
fn pick_vertex_at_distance_10m() {
    let (scene, camera, _, _, vert_t, _) = setup(10.0);
    assert_vertex_hit_at_corner(&scene, &camera, vert_t, 10.0);
}

#[test]
fn pick_vertex_at_distance_100m() {
    let (scene, camera, _, _, vert_t, _) = setup(100.0);
    assert_vertex_hit_at_corner(&scene, &camera, vert_t, 100.0);
}

#[test]
fn pick_vertex_at_distance_1000m() {
    let (scene, camera, _, _, vert_t, _) = setup(1000.0);
    assert_vertex_hit_at_corner(&scene, &camera, vert_t, 1000.0);
}

/// Sanity: a ray that misses the box must return None at every distance.
#[test]
fn miss_returns_none_at_all_distances() {
    for d in [1.0_f32, 10.0, 100.0, 1000.0] {
        let (scene, camera, _, _, _, _) = setup(d);
        // A ray far above the box (+Z), well past any threshold.
        let outside = [W as f32 * 0.5, H as f32 * 0.5, D as f32 * 1000.0];
        let (sx, sy) = project_to_screen(outside, camera.view_proj());
        let (origin, dir) = screen_to_ray(sx, sy, SCREEN_W, SCREEN_H, camera.inv_view_proj());
        let obj = scene.objects.first().unwrap();
        let hit = pick_triangle(origin, dir, &obj.mesh.vertices, &obj.mesh.indices);
        assert!(
            hit.is_none(),
            "pick_triangle aimed above the box should miss at distance {d}, got {hit:?}"
        );
    }
}
