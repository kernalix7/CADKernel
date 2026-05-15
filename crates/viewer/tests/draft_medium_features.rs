//! Dispatcher-boundary tests for the 10 Draft workbench MEDIUM-tier features.
//!
//! Each test dispatches the corresponding `GuiAction::Draft(...)` MEDIUM action
//! and asserts the resulting scene / overlay state changed at the dispatcher
//! boundary. UX work (gizmos, numeric modals, plane pickers, vertex pickers,
//! endpoint drag) is a future Phase B/C/D deliverable — these tests guarantee
//! that the kernel-call wiring is intact while the UX layer is built on top.
//!
//! Mirrors the pattern in `draft_easy_features.rs` (UI-A2).

use cadkernel_viewer::test_support::CadApp;

fn seed_selected_box(app: &mut CadApp) {
    app.dispatch_create_box(2.0, 2.0, 2.0);
    app.select_first_for_test();
}

fn last_object_name(app: &CadApp) -> &str {
    app.scene_ref()
        .objects
        .last()
        .expect("last scene object")
        .name
        .as_str()
}

#[test]
fn draft_facebinder_creates_new_scene_object() {
    let mut app = CadApp::new_headless();
    seed_selected_box(&mut app);
    let scene_before = app.scene_ref().len();

    app.dispatch_draft_facebinder();

    assert_eq!(
        app.scene_ref().len(),
        scene_before + 1,
        "Draft Facebinder must add one scene object"
    );
    assert!(
        last_object_name(&app)
            .to_lowercase()
            .contains("face binder"),
        "Draft Facebinder result name should mention face binder"
    );
}

#[test]
fn draft_move_adds_moved_solid_to_scene() {
    let mut app = CadApp::new_headless();
    seed_selected_box(&mut app);
    let scene_before = app.scene_ref().len();

    app.dispatch_draft_move();

    assert_eq!(
        app.scene_ref().len(),
        scene_before + 1,
        "Draft Move must add one scene object"
    );
    assert!(
        last_object_name(&app).contains("moved"),
        "Draft Move result name should mention moved"
    );
}

#[test]
fn draft_rotate_adds_rotated_solid_to_scene() {
    let mut app = CadApp::new_headless();
    seed_selected_box(&mut app);
    let scene_before = app.scene_ref().len();

    app.dispatch_draft_rotate();

    assert_eq!(
        app.scene_ref().len(),
        scene_before + 1,
        "Draft Rotate must add one scene object"
    );
    assert!(
        last_object_name(&app).contains("rotated"),
        "Draft Rotate result name should mention rotated"
    );
}

#[test]
fn draft_scale_adds_scaled_solid_to_scene() {
    let mut app = CadApp::new_headless();
    seed_selected_box(&mut app);
    let scene_before = app.scene_ref().len();

    app.dispatch_draft_scale();

    assert_eq!(
        app.scene_ref().len(),
        scene_before + 1,
        "Draft Scale must add one scene object"
    );
    let name = last_object_name(&app);
    assert!(
        name.contains('×') || name.contains('2'),
        "Draft Scale result name should mention the scale factor"
    );
}

#[test]
fn draft_mirror_adds_mirrored_solid_to_scene() {
    let mut app = CadApp::new_headless();
    seed_selected_box(&mut app);
    let scene_before = app.scene_ref().len();

    app.dispatch_draft_mirror();

    assert_eq!(
        app.scene_ref().len(),
        scene_before + 1,
        "Draft Mirror must add one scene object"
    );
    assert!(
        last_object_name(&app).contains("mirrored"),
        "Draft Mirror result name should mention mirrored"
    );
}

#[test]
fn draft_offset_adds_overlay_polyline_and_scene_object() {
    let mut app = CadApp::new_headless();
    let scene_before = app.scene_ref().len();
    let (p_before, _, _) = app.overlay_counts();

    app.dispatch_draft_offset();

    assert_eq!(
        app.scene_ref().len(),
        scene_before + 1,
        "Draft Offset must add one scene object"
    );
    let (p_after, _, _) = app.overlay_counts();
    assert!(
        p_after > p_before,
        "Draft Offset must add at least one overlay polyline"
    );
}

#[test]
fn draft_trim_adds_overlay_polyline_and_scene_object() {
    let mut app = CadApp::new_headless();
    let scene_before = app.scene_ref().len();
    let (p_before, _, _) = app.overlay_counts();

    app.dispatch_draft_trim();

    assert_eq!(
        app.scene_ref().len(),
        scene_before + 1,
        "Draft Trim must add one scene object"
    );
    let (p_after, _, _) = app.overlay_counts();
    assert!(
        p_after > p_before,
        "Draft Trim must add at least one overlay polyline"
    );
}

#[test]
fn draft_stretch_adds_overlay_polyline_and_scene_object() {
    let mut app = CadApp::new_headless();
    let scene_before = app.scene_ref().len();
    let (p_before, _, _) = app.overlay_counts();

    app.dispatch_draft_stretch();

    assert_eq!(
        app.scene_ref().len(),
        scene_before + 1,
        "Draft Stretch must add one scene object"
    );
    let (p_after, _, _) = app.overlay_counts();
    assert!(
        p_after > p_before,
        "Draft Stretch must add at least one overlay polyline"
    );
}

#[test]
fn draft_dimension_adds_overlay_polylines_labels_and_scene_object() {
    let mut app = CadApp::new_headless();
    let scene_before = app.scene_ref().len();
    let (p_before, _, l_before) = app.overlay_counts();

    app.dispatch_draft_dimension();

    assert_eq!(
        app.scene_ref().len(),
        scene_before + 1,
        "Draft Dimension must add one scene object"
    );
    let (p_after, _, l_after) = app.overlay_counts();
    assert!(
        p_after >= p_before + 3,
        "Draft Dimension must add extension and dimension polylines"
    );
    assert!(
        l_after > l_before,
        "Draft Dimension must add at least one overlay label"
    );
}

#[test]
fn draft_label_adds_overlay_polyline_label_and_scene_object() {
    let mut app = CadApp::new_headless();
    let scene_before = app.scene_ref().len();
    let (p_before, _, l_before) = app.overlay_counts();

    app.dispatch_draft_label();

    assert_eq!(
        app.scene_ref().len(),
        scene_before + 1,
        "Draft Label must add one scene object"
    );
    let (p_after, _, l_after) = app.overlay_counts();
    assert!(
        p_after > p_before,
        "Draft Label must add at least one overlay polyline"
    );
    assert!(
        l_after > l_before,
        "Draft Label must add at least one overlay label"
    );
}
