use cadkernel_math::Point3;
use cadkernel_modeling::make_box;
use cadkernel_topology::BRepModel;
use cadkernel_viewer::StandardView;
use cadkernel_viewer::scene::{Scene, SectionBox};
use cadkernel_viewer::test_support::CadApp;

fn add_box(scene: &mut Scene, origin: Point3, size: f64) {
    let mut model = BRepModel::new();
    let result = make_box(&mut model, origin, size, size, size).unwrap();
    scene.add_object("Box", model, result.solid, None, None);
}

fn assert_vertices_inside(scene: &Scene) {
    let (vertices, _ranges) = scene.build_combined_vertices();
    assert!(!vertices.is_empty());
    for vertex in vertices {
        assert!(
            scene.section_box.contains_f32(vertex.position),
            "vertex {:?} outside {:?}",
            vertex.position,
            scene.section_box
        );
    }
}

#[test]
fn section_box_defaults_inactive() {
    let scene = Scene::new();
    assert!(!scene.section_box.active);
    assert_eq!(scene.section_box, SectionBox::inactive());
}

#[test]
fn section_box_bounds_are_sanitized() {
    let mut scene = Scene::new();
    scene.set_section_box_bounds([2.0, 3.0, 4.0], [-1.0, -2.0, -3.0]);
    assert_eq!(scene.section_box.min, [-1.0, -2.0, -3.0]);
    assert_eq!(scene.section_box.max, [2.0, 3.0, 4.0]);
}

#[test]
fn reset_section_box_to_visible_bounds_uses_scene_geometry() {
    let mut scene = Scene::new();
    add_box(&mut scene, Point3::ORIGIN, 2.0);
    assert!(scene.reset_section_box_to_visible_bounds());
    let (mn, mx) = scene.visible_bounds().unwrap();
    assert_eq!(scene.section_box.min, mn);
    assert_eq!(scene.section_box.max, mx);
}

#[test]
fn section_box_clips_vertices_to_half_space() {
    let mut scene = Scene::new();
    add_box(&mut scene, Point3::ORIGIN, 2.0);
    let (mn, mx) = scene.visible_bounds().unwrap();
    let mid_x = (mn[0] + mx[0]) * 0.5;
    scene.section_box = SectionBox::new([mn[0], mn[1], mn[2]], [mid_x, mx[1], mx[2]], true);
    assert_vertices_inside(&scene);
}

#[test]
fn section_box_can_clip_to_thin_center_slab() {
    let mut scene = Scene::new();
    add_box(&mut scene, Point3::ORIGIN, 3.0);
    let (mn, mx) = scene.visible_bounds().unwrap();
    let mid_z = (mn[2] + mx[2]) * 0.5;
    scene.section_box = SectionBox::new(
        [mn[0], mn[1], mid_z - 0.2],
        [mx[0], mx[1], mid_z + 0.2],
        true,
    );
    assert_vertices_inside(&scene);
}

#[test]
fn resize_positive_section_face_moves_max() {
    let mut scene = Scene::new();
    scene.section_box = SectionBox::new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], true);
    assert!(scene.resize_section_box_face(0, true, 0.5));
    assert_eq!(scene.section_box.max[0], 1.5);
}

#[test]
fn resize_negative_section_face_moves_min() {
    let mut scene = Scene::new();
    scene.section_box = SectionBox::new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], true);
    assert!(scene.resize_section_box_face(1, false, -0.25));
    assert_eq!(scene.section_box.min[1], -0.25);
}

#[test]
fn app_toggle_section_box_on_and_off() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 2.0, 2.0);
    app.dispatch_toggle_section_box_for_test();
    assert!(app.scene_ref().section_box.active);
    app.dispatch_toggle_section_box_for_test();
    assert!(!app.scene_ref().section_box.active);
}

#[test]
fn section_box_state_survives_camera_change() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 2.0, 2.0);
    app.dispatch_toggle_section_box_for_test();
    app.dispatch_set_section_box_for_test([-0.5, -0.5, -0.5], [0.5, 0.5, 0.5]);
    let before = app.scene_ref().section_box;
    assert!(app.dispatch_view_gizmo_axis_for_test(2, true));
    assert_eq!(app.scene_ref().section_box, before);
    let (yaw, pitch) = StandardView::Top.yaw_pitch();
    assert!((app.camera_ref().yaw - yaw).abs() < 1e-5);
    assert!((app.camera_ref().pitch - pitch).abs() < 1e-5);
}
