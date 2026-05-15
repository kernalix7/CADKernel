//! Dispatcher-boundary tests for 7 previously untracked PartDesign variants
//! and a re-verify test for ShapeBinder.
//!
//! The built-in mechanical generators now assert kernel-call wiring and scene
//! mutation. Feature-management variants assert dispatcher state mutation.

use cadkernel_viewer::test_support::CadApp;

fn last_object_name(app: &CadApp) -> String {
    app.scene_ref()
        .objects
        .last()
        .expect("last scene object")
        .name
        .clone()
}

fn seed_three_boxes(app: &mut CadApp) {
    app.dispatch_create_box(1.0, 1.0, 1.0);
    app.dispatch_create_box(1.0, 1.0, 1.0);
    app.dispatch_create_box(1.0, 1.0, 1.0);
}

#[test]
fn partdesign_create_sprocket_adds_solid_to_scene() {
    let mut app = CadApp::new_headless();
    let before = app.scene_ref().len();

    app.dispatch_partdesign_create_sprocket(16, 2.0, 8.0, 1.5);

    assert_eq!(
        app.scene_ref().len(),
        before + 1,
        "Sprocket must add one scene object"
    );
    let name = last_object_name(&app).to_lowercase();
    assert!(
        name.contains("sprocket"),
        "Sprocket result name should mention sprocket (got '{name}')"
    );
}

#[test]
fn partdesign_create_shaft_design_adds_solid_to_scene() {
    let mut app = CadApp::new_headless();
    let before = app.scene_ref().len();

    let segments = vec![(5.0, 8.0), (3.0, 10.0), (5.0, 8.0)];
    app.dispatch_partdesign_create_shaft_design(segments);

    assert_eq!(
        app.scene_ref().len(),
        before + 1,
        "Shaft Design must add one scene object"
    );
    let name = last_object_name(&app).to_lowercase();
    assert!(
        name.contains("shaft"),
        "Shaft Design result name should mention shaft (got '{name}')"
    );
}

#[test]
fn partdesign_create_involute_gear_adds_solid_to_scene() {
    let mut app = CadApp::new_headless();
    let before = app.scene_ref().len();

    app.dispatch_partdesign_create_involute_gear(20, 2.5, 20.0_f64.to_radians());

    assert_eq!(
        app.scene_ref().len(),
        before + 1,
        "Involute Gear must add one scene object"
    );
    let name = last_object_name(&app).to_lowercase();
    assert!(
        name.contains("gear"),
        "Involute Gear result name should mention gear (got '{name}')"
    );
}

#[test]
fn partdesign_shape_binder_with_selection_adds_binder() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 2.0, 2.0);
    app.select_first_for_test();
    let before = app.scene_ref().len();

    app.dispatch_partdesign_shape_binder();

    assert_eq!(
        app.scene_ref().len(),
        before + 1,
        "ShapeBinder must add one scene object when a source is selected"
    );
    let name = last_object_name(&app).to_lowercase();
    assert!(
        name.contains("binder") || name.contains("shapebinder"),
        "ShapeBinder result name should mention binder (got '{name}')"
    );
}

#[test]
fn partdesign_suppress_feature_toggles_suppressed_flag() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(1.0, 1.0, 1.0);
    app.select_first_for_test();

    let id = app.scene_ref().selected_id().expect("selected id");
    let before = app.scene_ref().get(id).expect("scene object").suppressed;

    app.dispatch_partdesign_suppress_feature();

    let after = app.scene_ref().get(id).expect("scene object").suppressed;
    assert_ne!(before, after, "SuppressFeature must toggle suppressed flag");
}

#[test]
fn partdesign_set_tip_marks_selected_object_as_tip() {
    let mut app = CadApp::new_headless();
    seed_three_boxes(&mut app);
    let ids: Vec<_> = app.scene_ref().objects.iter().map(|o| o.id).collect();
    app.select_for_test(ids[1]);

    app.dispatch_partdesign_set_tip();

    let scene = app.scene_ref();
    for obj in &scene.objects {
        if obj.id == ids[1] {
            assert!(obj.is_tip, "Selected object must be marked is_tip=true");
        } else {
            assert!(!obj.is_tip, "Non-selected object must have is_tip=false");
        }
    }
}

#[test]
fn partdesign_move_feature_up_reorders_scene() {
    let mut app = CadApp::new_headless();
    seed_three_boxes(&mut app);
    let ids_before: Vec<_> = app.scene_ref().objects.iter().map(|o| o.id).collect();
    app.select_for_test(ids_before[2]);

    app.dispatch_partdesign_move_feature_up();

    let ids_after: Vec<_> = app.scene_ref().objects.iter().map(|o| o.id).collect();
    assert_eq!(
        ids_after,
        vec![ids_before[0], ids_before[2], ids_before[1]],
        "MoveFeatureUp must move selected object one position earlier"
    );
}

#[test]
fn partdesign_move_feature_down_reorders_scene() {
    let mut app = CadApp::new_headless();
    seed_three_boxes(&mut app);
    let ids_before: Vec<_> = app.scene_ref().objects.iter().map(|o| o.id).collect();
    app.select_for_test(ids_before[0]);

    app.dispatch_partdesign_move_feature_down();

    let ids_after: Vec<_> = app.scene_ref().objects.iter().map(|o| o.id).collect();
    assert_eq!(
        ids_after,
        vec![ids_before[1], ids_before[0], ids_before[2]],
        "MoveFeatureDown must move selected object one position later"
    );
}
