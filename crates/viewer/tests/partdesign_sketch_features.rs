//! Dispatcher-boundary tests for the five PartDesign sketch-driven features.
//!
//! Each test seeds an active sketch via `test_support`, dispatches the
//! corresponding GuiAction, and asserts the scene gained a solid. This is the
//! user-visible contract: clicking Pad/Pocket/Hole/CountersunkHole/Groove with
//! valid inputs must create geometry, not just emit a log line.

use cadkernel_viewer::test_support::CadApp;

#[test]
fn pad_sketch_produces_scene_object() {
    let mut app = CadApp::new_headless();
    app.seed_test_sketch_square(2.0);
    let before = app.scene_ref().len();

    app.dispatch_pad_sketch(3.0, false);

    assert_eq!(
        app.scene_ref().len(),
        before + 1,
        "Pad with valid sketch must add exactly one solid to the scene"
    );
    let obj = app.scene_ref().objects.last().expect("pad result object");
    assert!(
        !obj.vertices.is_empty(),
        "Pad result must contain tessellated geometry"
    );
}

#[test]
fn pocket_sketch_produces_scene_object() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(4.0, 4.0, 4.0);
    let before = app.scene_ref().len();
    app.seed_test_sketch_square(1.5);

    app.dispatch_pocket_sketch(2.0, false);

    assert_eq!(
        app.scene_ref().len(),
        before + 1,
        "Pocket with base solid and valid sketch must add the pocketed result to the scene"
    );
    let obj = app
        .scene_ref()
        .objects
        .last()
        .expect("pocket result object");
    assert!(!obj.vertices.is_empty(), "Pocket result must have geometry");
}

#[test]
fn hole_sketch_produces_scene_object() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(4.0, 4.0, 4.0);
    let before = app.scene_ref().len();

    app.dispatch_hole_sketch(0.5, 4.0);

    assert_eq!(
        app.scene_ref().len(),
        before + 1,
        "Hole with base solid must add the drilled result to the scene"
    );
    let obj = app.scene_ref().objects.last().expect("hole result object");
    assert!(!obj.vertices.is_empty(), "Hole result must have geometry");
}

#[test]
fn countersunk_hole_sketch_produces_scene_object() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(6.0, 6.0, 6.0);
    let before = app.scene_ref().len();

    app.dispatch_countersunk_hole_sketch(0.4, 2.0, 90.0);

    assert_eq!(
        app.scene_ref().len(),
        before + 1,
        "CountersunkHole with valid params must produce a scene object"
    );
    let obj = app
        .scene_ref()
        .objects
        .last()
        .expect("countersunk hole result object");
    assert!(
        !obj.vertices.is_empty(),
        "CountersunkHole result must have geometry"
    );
}

#[test]
fn groove_sketch_produces_scene_object() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(4.0, 4.0, 4.0);
    let before = app.scene_ref().len();
    app.seed_test_sketch_square(0.5);

    app.dispatch_groove_sketch(90.0);

    assert_eq!(
        app.scene_ref().len(),
        before + 1,
        "Groove with valid sketch must produce a scene object"
    );
    let obj = app
        .scene_ref()
        .objects
        .last()
        .expect("groove result object");
    assert!(!obj.vertices.is_empty(), "Groove result must have geometry");
}

#[test]
fn pad_sketch_without_active_sketch_is_graceful() {
    let mut app = CadApp::new_headless();

    app.dispatch_pad_sketch(3.0, false);

    assert!(
        app.scene_ref().is_empty(),
        "Pad without an active sketch must reject gracefully with no scene change"
    );
}
