//! Dispatcher-boundary tests for the 19 Draft workbench EASY-tier features.
//!
//! Each test dispatches the corresponding `GuiAction::Draft(...)` and asserts
//! the resulting scene / overlay / last_sketch state changed. This is the
//! user-visible contract: clicking any Draft button with valid defaults must
//! produce an observable result, not just emit a log line.
//!
//! Mirrors the pattern in `partdesign_sketch_features.rs` (UI-A1).

use cadkernel_viewer::test_support::CadApp;

fn assert_last_object_has_geometry(app: &CadApp, message: &str) {
    let obj = app.scene_ref().objects.last().expect("scene object");
    assert!(!obj.vertices.is_empty(), "{message}");
}

fn seed_selected_box(app: &mut CadApp) {
    app.dispatch_create_box(2.0, 2.0, 2.0);
    app.select_first_for_test();
}

#[test]
fn draft_line_adds_tree_entry_and_overlay_polyline() {
    let mut app = CadApp::new_headless();
    let before_scene = app.scene_ref().len();
    let (before_polylines, _, _) = app.overlay_counts();

    app.dispatch_draft_line();

    assert_eq!(
        app.scene_ref().len(),
        before_scene + 1,
        "Draft Line must add one tree entry"
    );
    let (after_polylines, _, _) = app.overlay_counts();
    assert!(
        after_polylines > before_polylines,
        "Draft Line must add at least one overlay polyline"
    );
}

#[test]
fn draft_wire_adds_tree_entry_and_overlay_polyline() {
    let mut app = CadApp::new_headless();
    let before_scene = app.scene_ref().len();
    let (before_polylines, _, _) = app.overlay_counts();

    app.dispatch_draft_wire();

    assert_eq!(
        app.scene_ref().len(),
        before_scene + 1,
        "Draft Wire must add one tree entry"
    );
    let (after_polylines, _, _) = app.overlay_counts();
    assert!(
        after_polylines > before_polylines,
        "Draft Wire must add at least one overlay polyline"
    );
}

#[test]
fn draft_circle_produces_solid_scene_object() {
    let mut app = CadApp::new_headless();
    let before_scene = app.scene_ref().len();

    app.dispatch_draft_circle();

    assert_eq!(
        app.scene_ref().len(),
        before_scene + 1,
        "Draft Circle must add one scene object"
    );
    assert_last_object_has_geometry(&app, "Draft Circle result must have geometry");
}

#[test]
fn draft_arc_produces_solid_scene_object() {
    let mut app = CadApp::new_headless();
    let before_scene = app.scene_ref().len();

    app.dispatch_draft_arc();

    assert_eq!(
        app.scene_ref().len(),
        before_scene + 1,
        "Draft Arc must add one scene object"
    );
    assert_last_object_has_geometry(&app, "Draft Arc result must have geometry");
}

#[test]
fn draft_ellipse_produces_solid_scene_object() {
    let mut app = CadApp::new_headless();
    let before_scene = app.scene_ref().len();

    app.dispatch_draft_ellipse();

    assert_eq!(
        app.scene_ref().len(),
        before_scene + 1,
        "Draft Ellipse must add one scene object"
    );
    assert_last_object_has_geometry(&app, "Draft Ellipse result must have geometry");
}

#[test]
fn draft_bspline_adds_tree_entry_and_overlay_polyline() {
    let mut app = CadApp::new_headless();
    let before_scene = app.scene_ref().len();
    let (before_polylines, _, _) = app.overlay_counts();

    app.dispatch_draft_bspline();

    assert_eq!(
        app.scene_ref().len(),
        before_scene + 1,
        "Draft B-spline must add one tree entry"
    );
    let (after_polylines, _, _) = app.overlay_counts();
    assert!(
        after_polylines > before_polylines,
        "Draft B-spline must add at least one overlay polyline"
    );
}

#[test]
fn draft_bezier_adds_tree_entry_and_overlay_polyline() {
    let mut app = CadApp::new_headless();
    let before_scene = app.scene_ref().len();
    let (before_polylines, _, _) = app.overlay_counts();

    app.dispatch_draft_bezier();

    assert_eq!(
        app.scene_ref().len(),
        before_scene + 1,
        "Draft Bezier must add one tree entry"
    );
    let (after_polylines, _, _) = app.overlay_counts();
    assert!(
        after_polylines > before_polylines,
        "Draft Bezier must add at least one overlay polyline"
    );
}

#[test]
fn draft_point_adds_tree_entry_and_overlay_point() {
    let mut app = CadApp::new_headless();
    let before_scene = app.scene_ref().len();
    let (_, before_points, _) = app.overlay_counts();

    app.dispatch_draft_point();

    assert_eq!(
        app.scene_ref().len(),
        before_scene + 1,
        "Draft Point must add one tree entry"
    );
    let (_, after_points, _) = app.overlay_counts();
    assert!(
        after_points > before_points,
        "Draft Point must add at least one overlay point"
    );
}

#[test]
fn draft_hatch_adds_tree_entry_and_overlay_polylines() {
    let mut app = CadApp::new_headless();
    let before_scene = app.scene_ref().len();
    let (before_polylines, _, _) = app.overlay_counts();

    app.dispatch_draft_hatch();

    assert_eq!(
        app.scene_ref().len(),
        before_scene + 1,
        "Draft Hatch must add one tree entry"
    );
    let (after_polylines, _, _) = app.overlay_counts();
    assert!(
        after_polylines >= before_polylines + 2,
        "Draft Hatch must add boundary and fill-line overlays"
    );
}

#[test]
fn draft_text_adds_tree_entry_overlay_strokes_and_label() {
    let mut app = CadApp::new_headless();
    let before_scene = app.scene_ref().len();
    let (before_polylines, _, before_labels) = app.overlay_counts();

    app.dispatch_draft_text();

    assert_eq!(
        app.scene_ref().len(),
        before_scene + 1,
        "Draft Text must add one tree entry"
    );
    let (after_polylines, _, after_labels) = app.overlay_counts();
    assert!(
        after_polylines > before_polylines,
        "Draft Text must add stroke overlays"
    );
    assert_eq!(
        after_labels,
        before_labels + 1,
        "Draft Text must add one overlay label"
    );
}

#[test]
fn draft_upgrade_produces_solid_scene_object() {
    let mut app = CadApp::new_headless();
    let before_scene = app.scene_ref().len();

    app.dispatch_draft_upgrade();

    assert_eq!(
        app.scene_ref().len(),
        before_scene + 1,
        "Draft Upgrade must add one scene object"
    );
    assert_last_object_has_geometry(&app, "Draft Upgrade result must have geometry");
}

#[test]
fn draft_wire_to_bspline_adds_tree_entry_and_overlay_polyline() {
    let mut app = CadApp::new_headless();
    let before_scene = app.scene_ref().len();
    let (before_polylines, _, _) = app.overlay_counts();

    app.dispatch_draft_wire_to_bspline();

    assert_eq!(
        app.scene_ref().len(),
        before_scene + 1,
        "Draft WireToBSpline must add one tree entry"
    );
    let (after_polylines, _, _) = app.overlay_counts();
    assert!(
        after_polylines > before_polylines,
        "Draft WireToBSpline must add at least one overlay polyline"
    );
}

#[test]
fn draft_to_sketch_sets_last_sketch_without_touching_scene() {
    let mut app = CadApp::new_headless();
    let before_scene = app.scene_ref().len();
    assert!(
        !app.last_sketch_is_set(),
        "headless app should start without last_sketch"
    );

    app.dispatch_draft_to_sketch();

    assert!(
        app.last_sketch_is_set(),
        "Draft ToSketch must populate last_sketch"
    );
    assert_eq!(
        app.scene_ref().len(),
        before_scene,
        "Draft ToSketch must not add a scene object"
    );
}

#[test]
fn draft_clone_with_selection_duplicates_solid() {
    let mut app = CadApp::new_headless();
    seed_selected_box(&mut app);
    let before_scene = app.scene_ref().len();

    app.dispatch_draft_clone();

    assert_eq!(
        app.scene_ref().len(),
        before_scene + 1,
        "Draft Clone must duplicate the selected solid"
    );
    assert_last_object_has_geometry(&app, "Draft Clone result must have geometry");
}

#[test]
fn draft_clone_without_selection_no_op() {
    let mut app = CadApp::new_headless();

    app.dispatch_draft_clone();

    assert_eq!(
        app.scene_ref().len(),
        0,
        "Draft Clone without selection must leave the scene unchanged"
    );
}

#[test]
fn draft_array_rect_with_selection_adds_copies() {
    let mut app = CadApp::new_headless();
    seed_selected_box(&mut app);
    let before_scene = app.scene_ref().len();

    app.dispatch_draft_array_rect();

    assert!(
        app.scene_ref().len() > before_scene,
        "Draft ArrayRect must add at least one copy"
    );
}

#[test]
fn draft_array_polar_with_selection_adds_copies() {
    let mut app = CadApp::new_headless();
    seed_selected_box(&mut app);
    let before_scene = app.scene_ref().len();

    app.dispatch_draft_array_polar();

    assert!(
        app.scene_ref().len() > before_scene,
        "Draft ArrayPolar must add copies"
    );
}

#[test]
fn draft_array_path_with_selection_adds_copies() {
    let mut app = CadApp::new_headless();
    seed_selected_box(&mut app);
    let before_scene = app.scene_ref().len();

    app.dispatch_draft_array_path();

    assert!(
        app.scene_ref().len() > before_scene,
        "Draft ArrayPath must add copies"
    );
}

#[test]
fn draft_array_point_with_selection_adds_copies() {
    let mut app = CadApp::new_headless();
    seed_selected_box(&mut app);
    let before_scene = app.scene_ref().len();

    app.dispatch_draft_array_point();

    assert!(
        app.scene_ref().len() > before_scene,
        "Draft ArrayPoint must add copies"
    );
}

#[test]
fn draft_downgrade_with_selection_adds_face_solids() {
    let mut app = CadApp::new_headless();
    seed_selected_box(&mut app);
    let before_scene = app.scene_ref().len();

    app.dispatch_draft_downgrade();

    assert!(
        app.scene_ref().len() > before_scene,
        "Draft Downgrade must add face-solid scene objects"
    );
}
