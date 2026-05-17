use cadkernel_viewer::test_support::CadApp;
use cadkernel_viewer::{Projection, StandardView};

const EPS: f32 = 1e-5;

fn assert_camera_view(app: &CadApp, view: StandardView) {
    let cam = app.camera_ref();
    let (yaw, pitch) = view.yaw_pitch();
    assert!(
        (cam.yaw - yaw).abs() < EPS,
        "yaw {:?} != {:?}",
        cam.yaw,
        yaw
    );
    assert!(
        (cam.pitch - pitch).abs() < EPS,
        "pitch {:?} != {:?}",
        cam.pitch,
        pitch
    );
    assert_eq!(cam.projection, Projection::Orthographic);
}

#[test]
fn view_cube_front_face_snaps_orthographic() {
    let mut app = CadApp::new_headless();
    assert!(app.dispatch_view_cube_face_for_test(0));
    assert_camera_view(&app, StandardView::Front);
}

#[test]
fn view_cube_back_face_snaps_orthographic() {
    let mut app = CadApp::new_headless();
    assert!(app.dispatch_view_cube_face_for_test(1));
    assert_camera_view(&app, StandardView::Back);
}

#[test]
fn view_cube_right_face_snaps_orthographic() {
    let mut app = CadApp::new_headless();
    assert!(app.dispatch_view_cube_face_for_test(2));
    assert_camera_view(&app, StandardView::Right);
}

#[test]
fn view_cube_left_face_snaps_orthographic() {
    let mut app = CadApp::new_headless();
    assert!(app.dispatch_view_cube_face_for_test(3));
    assert_camera_view(&app, StandardView::Left);
}

#[test]
fn view_cube_top_face_uses_fixed_face_yaw() {
    let mut app = CadApp::new_headless();
    assert!(app.dispatch_view_cube_face_for_test(4));
    assert_camera_view(&app, StandardView::Top);
}

#[test]
fn view_cube_bottom_face_uses_fixed_face_yaw() {
    let mut app = CadApp::new_headless();
    assert!(app.dispatch_view_cube_face_for_test(5));
    assert_camera_view(&app, StandardView::Bottom);
}

#[test]
fn view_cube_edge_click_sets_orthographic_diagonal() {
    let mut app = CadApp::new_headless();
    assert!(app.dispatch_view_cube_edge_for_test(0));
    let cam = app.camera_ref();
    assert_eq!(cam.projection, Projection::Orthographic);
    assert!((cam.yaw + std::f32::consts::FRAC_PI_2).abs() < EPS);
    assert!((cam.pitch + std::f32::consts::FRAC_PI_4).abs() < EPS);
}

#[test]
fn view_cube_vertical_edge_click_sets_horizontal_diagonal() {
    let mut app = CadApp::new_headless();
    assert!(app.dispatch_view_cube_edge_for_test(10));
    let cam = app.camera_ref();
    assert_eq!(cam.projection, Projection::Orthographic);
    assert!((cam.yaw - std::f32::consts::FRAC_PI_4).abs() < EPS);
    assert!(cam.pitch.abs() < EPS);
}

#[test]
fn view_cube_corner_click_uses_approximately_35_degree_pitch() {
    let mut app = CadApp::new_headless();
    assert!(app.dispatch_view_cube_corner_for_test(6));
    let cam = app.camera_ref();
    assert_eq!(cam.projection, Projection::Orthographic);
    assert!((cam.yaw - std::f32::consts::FRAC_PI_4).abs() < EPS);
    assert!((cam.pitch.to_degrees() - 35.27).abs() < 0.05);
}

#[test]
fn view_cube_invalid_hit_indices_are_ignored() {
    let mut app = CadApp::new_headless();
    assert!(!app.dispatch_view_cube_face_for_test(99));
    assert!(!app.dispatch_view_cube_edge_for_test(99));
    assert!(!app.dispatch_view_cube_corner_for_test(99));
}

#[test]
fn view_gizmo_x_axis_clicks_snap_to_right_and_left() {
    let mut app = CadApp::new_headless();
    assert!(app.dispatch_view_gizmo_axis_for_test(0, true));
    assert_camera_view(&app, StandardView::Right);
    assert!(app.dispatch_view_gizmo_axis_for_test(0, false));
    assert_camera_view(&app, StandardView::Left);
}

#[test]
fn view_gizmo_y_and_z_axis_clicks_snap_to_standard_views() {
    let mut app = CadApp::new_headless();
    assert!(app.dispatch_view_gizmo_axis_for_test(1, true));
    assert_camera_view(&app, StandardView::Front);
    assert!(app.dispatch_view_gizmo_axis_for_test(1, false));
    assert_camera_view(&app, StandardView::Back);
    assert!(app.dispatch_view_gizmo_axis_for_test(2, true));
    assert_camera_view(&app, StandardView::Top);
    assert!(app.dispatch_view_gizmo_axis_for_test(2, false));
    assert_camera_view(&app, StandardView::Bottom);
}
