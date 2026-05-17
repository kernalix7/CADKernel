use cadkernel_math::Point3;
use cadkernel_modeling::make_box;
use cadkernel_topology::BRepModel;
use cadkernel_viewer::Vertex;
use cadkernel_viewer::scene::Scene;
use cadkernel_viewer::test_support::CadApp;

fn add_box(scene: &mut Scene, origin: Point3, size: f64) {
    let mut model = BRepModel::new();
    let result = make_box(&mut model, origin, size, size, size).unwrap();
    scene.add_object("Box", model, result.solid, None, None);
}

fn center(vertices: &[Vertex]) -> [f32; 3] {
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for vertex in vertices {
        for axis in 0..3 {
            min[axis] = min[axis].min(vertex.position[axis]);
            max[axis] = max[axis].max(vertex.position[axis]);
        }
    }
    [
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    ]
}

fn range_center(vertices: &[Vertex], start: u32, count: u32) -> [f32; 3] {
    let begin = start as usize;
    let end = begin + count as usize;
    center(&vertices[begin..end])
}

#[test]
fn exploded_view_defaults_to_zero_factor() {
    let scene = Scene::new();
    assert_eq!(scene.exploded_view.factor, 0.0);
}

#[test]
fn zero_factor_preserves_combined_vertices() {
    let mut scene = Scene::new();
    add_box(&mut scene, Point3::ORIGIN, 1.0);
    add_box(&mut scene, Point3::new(4.0, 0.0, 0.0), 1.0);
    let (before, _) = scene.build_combined_vertices();
    scene.exploded_view.factor = 0.0;
    let (after, _) = scene.build_combined_vertices();
    assert_eq!(before.len(), after.len());
    for (a, b) in before.iter().zip(after.iter()) {
        assert_eq!(a.position, b.position);
    }
}

#[test]
fn factor_one_doubles_object_distance_from_scene_centroid() {
    let mut scene = Scene::new();
    add_box(&mut scene, Point3::ORIGIN, 1.0);
    add_box(&mut scene, Point3::new(4.0, 0.0, 0.0), 1.0);
    let (base, ranges) = scene.build_combined_vertices();
    let c0 = range_center(&base, ranges[0].1, ranges[0].2);
    let c1 = range_center(&base, ranges[1].1, ranges[1].2);
    let centroid_x = (c0[0] + c1[0]) * 0.5;

    scene.exploded_view.factor = 1.0;
    let (exploded, exploded_ranges) = scene.build_combined_vertices();
    let e0 = range_center(&exploded, exploded_ranges[0].1, exploded_ranges[0].2);
    let e1 = range_center(&exploded, exploded_ranges[1].1, exploded_ranges[1].2);

    assert!((e0[0] - centroid_x - (c0[0] - centroid_x) * 2.0).abs() < 1e-5);
    assert!((e1[0] - centroid_x - (c1[0] - centroid_x) * 2.0).abs() < 1e-5);
}

#[test]
fn half_factor_moves_objects_halfway_outward() {
    let mut scene = Scene::new();
    add_box(&mut scene, Point3::ORIGIN, 1.0);
    add_box(&mut scene, Point3::new(6.0, 0.0, 0.0), 1.0);
    let (base, ranges) = scene.build_combined_vertices();
    let c0 = range_center(&base, ranges[0].1, ranges[0].2);
    let c1 = range_center(&base, ranges[1].1, ranges[1].2);
    let centroid_x = (c0[0] + c1[0]) * 0.5;

    scene.exploded_view.factor = 0.5;
    let (exploded, exploded_ranges) = scene.build_combined_vertices();
    let e0 = range_center(&exploded, exploded_ranges[0].1, exploded_ranges[0].2);
    let e1 = range_center(&exploded, exploded_ranges[1].1, exploded_ranges[1].2);

    assert!((e0[0] - centroid_x - (c0[0] - centroid_x) * 1.5).abs() < 1e-5);
    assert!((e1[0] - centroid_x - (c1[0] - centroid_x) * 1.5).abs() < 1e-5);
}

#[test]
fn single_object_at_centroid_does_not_move() {
    let mut scene = Scene::new();
    add_box(&mut scene, Point3::new(3.0, 2.0, 1.0), 1.0);
    let (base, ranges) = scene.build_combined_vertices();
    let c0 = range_center(&base, ranges[0].1, ranges[0].2);
    scene.exploded_view.factor = 2.0;
    let (exploded, exploded_ranges) = scene.build_combined_vertices();
    let e0 = range_center(&exploded, exploded_ranges[0].1, exploded_ranges[0].2);
    assert_eq!(c0, e0);
}

#[test]
fn hidden_objects_do_not_contribute_to_exploded_output() {
    let mut scene = Scene::new();
    add_box(&mut scene, Point3::ORIGIN, 1.0);
    add_box(&mut scene, Point3::new(4.0, 0.0, 0.0), 1.0);
    scene.objects[1].visible = false;
    scene.exploded_view.factor = 1.0;
    let (_vertices, ranges) = scene.build_combined_vertices();
    assert_eq!(ranges.len(), 1);
    assert_eq!(ranges[0].0, scene.objects[0].id);
}

#[test]
fn app_exploded_view_dispatch_clamps_factor_and_rebuilds_vertices() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(1.0, 1.0, 1.0);
    app.dispatch_create_box(1.0, 1.0, 1.0);
    let before = app.display_vertices_for_test().len();
    app.dispatch_exploded_view_factor_for_test(4.0);
    assert_eq!(app.scene_ref().exploded_view.factor, 2.0);
    assert_eq!(app.display_vertices_for_test().len(), before);
}
