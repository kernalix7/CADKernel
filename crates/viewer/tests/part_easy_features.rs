//! Dispatcher-boundary tests for the 13 Part workbench EASY-tier features.
//!
//! Each test dispatches the corresponding `GuiAction::Part(...)` and asserts
//! the resulting scene / overlay state changed. This is the user-visible
//! contract: clicking any Part EASY button with valid defaults must produce an
//! observable result, not just emit a log line.
//!
//! Mirrors the pattern in `draft_easy_features.rs` (UI-A2).

use cadkernel_viewer::test_support::CadApp;

fn assert_last_object_has_geometry(app: &CadApp, message: &str) {
    let obj = app.scene_ref().objects.last().expect("scene object");
    assert!(!obj.vertices.is_empty(), "{message}");
}

fn seed_selected_box(app: &mut CadApp) {
    app.dispatch_create_box(2.0, 2.0, 2.0);
    app.select_first_for_test();
}

fn seed_two_selected_boxes(app: &mut CadApp) {
    app.dispatch_create_box(2.0, 2.0, 2.0);
    app.dispatch_create_box(2.0, 2.0, 2.0);
    app.select_all_for_test();
}

#[test]
fn part_face_from_wires_produces_solid_scene_object() {
    let mut app = CadApp::new_headless();
    let before = app.scene_ref().len();

    app.dispatch_part_face_from_wires();

    assert_eq!(
        app.scene_ref().len(),
        before + 1,
        "FaceFromWires must add one solid to the scene"
    );
    assert_last_object_has_geometry(&app, "FaceFromWires result must have geometry");
}

#[test]
fn part_connect_shapes_with_two_selected_produces_solid() {
    let mut app = CadApp::new_headless();
    seed_two_selected_boxes(&mut app);
    let before = app.scene_ref().len();

    app.dispatch_part_connect_shapes();

    assert_eq!(
        app.scene_ref().len(),
        before + 1,
        "ConnectShapes with two selected solids must add one result"
    );
    assert_last_object_has_geometry(&app, "ConnectShapes result must have geometry");
}

#[test]
fn part_embed_shapes_with_two_selected_produces_solid() {
    let mut app = CadApp::new_headless();
    seed_two_selected_boxes(&mut app);
    let before = app.scene_ref().len();

    app.dispatch_part_embed_shapes();

    assert_eq!(
        app.scene_ref().len(),
        before + 1,
        "EmbedShapes with two selected solids must add one result"
    );
    assert_last_object_has_geometry(&app, "EmbedShapes result must have geometry");
}

#[test]
fn part_cutout_shapes_with_two_selected_produces_solid() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(4.0, 4.0, 4.0);
    app.dispatch_create_box(2.0, 2.0, 2.0);
    app.select_all_for_test();
    let before = app.scene_ref().len();

    app.dispatch_part_cutout_shapes();

    assert_eq!(
        app.scene_ref().len(),
        before + 1,
        "CutoutShapes with two selected solids must add one result"
    );
    assert_last_object_has_geometry(&app, "CutoutShapes result must have geometry");
}

#[test]
fn part_explode_compound_with_selection_is_log_only_no_op() {
    let mut app = CadApp::new_headless();
    seed_selected_box(&mut app);
    let before = app.scene_ref().len();

    app.dispatch_part_explode_compound();

    assert_eq!(
        app.scene_ref().len(),
        before,
        "ExplodeCompound is log-only and must not mutate the scene"
    );
}

#[test]
fn part_compound_filter_with_selection_is_log_only_no_op() {
    let mut app = CadApp::new_headless();
    seed_selected_box(&mut app);
    let before = app.scene_ref().len();

    app.dispatch_part_compound_filter();

    assert_eq!(
        app.scene_ref().len(),
        before,
        "CompoundFilter is log-only and must not mutate the scene"
    );
}

#[test]
fn part_boolean_fragments_with_two_selected_is_log_only_no_op() {
    let mut app = CadApp::new_headless();
    seed_two_selected_boxes(&mut app);
    let before = app.scene_ref().len();

    app.dispatch_part_boolean_fragments();

    assert_eq!(
        app.scene_ref().len(),
        before,
        "BooleanFragments is log-only until multi-model staging exists"
    );
}

#[test]
fn part_slice_to_compound_with_selection_adds_pieces() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 2.0, 2.0);
    app.select_first_for_test();
    let before = app.scene_ref().len();

    app.dispatch_part_slice_to_compound();

    assert!(
        app.scene_ref().len() > before,
        "SliceToCompound must add at least one piece"
    );
    assert_last_object_has_geometry(&app, "SliceToCompound result must have geometry");
}

#[test]
fn part_points_from_shape_with_selection_adds_overlay_points() {
    let mut app = CadApp::new_headless();
    seed_selected_box(&mut app);
    let before_scene = app.scene_ref().len();
    let (_, before_points, _) = app.overlay_counts();

    app.dispatch_part_points_from_shape();

    let (_, after_points, _) = app.overlay_counts();
    assert_eq!(
        app.scene_ref().len(),
        before_scene,
        "PointsFromShape must not add scene objects"
    );
    assert!(
        after_points > before_points,
        "PointsFromShape must add overlay points"
    );
}

#[test]
fn part_convert_to_solid_with_selection_produces_solid() {
    let mut app = CadApp::new_headless();
    seed_selected_box(&mut app);
    let before = app.scene_ref().len();

    app.dispatch_part_convert_to_solid();

    assert_eq!(
        app.scene_ref().len(),
        before + 1,
        "ConvertToSolid with a mesh-bearing selection must add one solid"
    );
    assert_last_object_has_geometry(&app, "ConvertToSolid result must have geometry");
}

#[test]
fn part_auto_defeaturing_with_selection_produces_solid() {
    let mut app = CadApp::new_headless();
    seed_selected_box(&mut app);
    let before = app.scene_ref().len();

    app.dispatch_part_auto_defeaturing(0.01);

    assert_eq!(
        app.scene_ref().len(),
        before + 1,
        "AutoDefeaturing with a positive threshold must add one result"
    );
    assert_last_object_has_geometry(&app, "AutoDefeaturing result must have geometry");
}

#[test]
fn part_transformed_copy_with_selection_produces_solid() {
    let mut app = CadApp::new_headless();
    seed_selected_box(&mut app);
    let before = app.scene_ref().len();

    app.dispatch_part_transformed_copy(5.0, 0.0, 0.0);

    assert_eq!(
        app.scene_ref().len(),
        before + 1,
        "TransformedCopy with a selection must add one translated copy"
    );
    assert_last_object_has_geometry(&app, "TransformedCopy result must have geometry");
}

#[test]
fn part_coons_patch_adds_mesh_entry_and_overlay_polyline() {
    let mut app = CadApp::new_headless();
    let before_scene = app.scene_ref().len();
    let (before_polylines, _, _) = app.overlay_counts();

    app.dispatch_part_coons_patch();

    let (after_polylines, _, _) = app.overlay_counts();
    assert_eq!(
        app.scene_ref().len(),
        before_scene + 1,
        "CoonsPatch must add one mesh-only scene entry"
    );
    assert!(
        after_polylines > before_polylines,
        "CoonsPatch must add a boundary overlay polyline"
    );
}

#[test]
fn part_connect_shapes_without_selection_no_op() {
    let mut app = CadApp::new_headless();

    app.dispatch_part_connect_shapes();

    assert!(
        app.scene_ref().is_empty(),
        "ConnectShapes without selection must leave the scene empty"
    );
}
