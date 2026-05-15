//! Dispatcher-boundary tests for the 8 remaining MEDIUM-tier features
//! across PartDesign, Part, and Surface workbenches.
//!
//! Each test dispatches the corresponding GuiAction MEDIUM action and asserts
//! the resulting scene state changed at the dispatcher boundary. UX work
//! (profile-list pickers, path pickers, distance modals, plane pickers,
//! curve/surface selection) is a future Phase deliverable -- these tests
//! guarantee that the kernel-call wiring is intact while the UX layer is
//! built on top.
//!
//! Mirrors the pattern in `draft_medium_features.rs` (UI-B1).

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
fn partdesign_additive_loft_adds_solid_to_scene() {
    let mut app = CadApp::new_headless();
    let scene_before = app.scene_ref().len();

    app.dispatch_partdesign_additive_loft();

    assert_eq!(
        app.scene_ref().len(),
        scene_before + 1,
        "PartDesign Additive Loft must add one scene object"
    );
    assert!(
        last_object_name(&app).to_lowercase().contains("loft"),
        "PartDesign Additive Loft result name should mention loft"
    );
}

#[test]
fn partdesign_additive_pipe_adds_solid_to_scene() {
    let mut app = CadApp::new_headless();
    let scene_before = app.scene_ref().len();

    app.dispatch_partdesign_additive_pipe();

    assert_eq!(
        app.scene_ref().len(),
        scene_before + 1,
        "PartDesign Additive Pipe must add one scene object"
    );
    assert!(
        last_object_name(&app).to_lowercase().contains("pipe"),
        "PartDesign Additive Pipe result name should mention pipe"
    );
}

#[test]
fn partdesign_subtractive_loft_adds_solid_to_scene() {
    let mut app = CadApp::new_headless();
    seed_selected_box(&mut app);
    let scene_before = app.scene_ref().len();

    app.dispatch_partdesign_subtractive_loft();

    assert_eq!(
        app.scene_ref().len(),
        scene_before + 1,
        "PartDesign Subtractive Loft must add one scene object"
    );
    assert!(
        last_object_name(&app).to_lowercase().contains("loft"),
        "PartDesign Subtractive Loft result name should mention loft"
    );
}

#[test]
fn partdesign_subtractive_pipe_adds_solid_to_scene() {
    let mut app = CadApp::new_headless();
    seed_selected_box(&mut app);
    let scene_before = app.scene_ref().len();

    app.dispatch_partdesign_subtractive_pipe();

    assert_eq!(
        app.scene_ref().len(),
        scene_before + 1,
        "PartDesign Subtractive Pipe must add one scene object"
    );
    assert!(
        last_object_name(&app).to_lowercase().contains("pipe"),
        "PartDesign Subtractive Pipe result name should mention pipe"
    );
}

#[test]
fn part_project_curves_on_surface_adds_overlay_and_scene_object() {
    let mut app = CadApp::new_headless();
    seed_selected_box(&mut app);
    let scene_before = app.scene_ref().len();
    let (p_before, _, _) = app.overlay_counts();

    app.dispatch_part_project_curves_on_surface();

    assert_eq!(
        app.scene_ref().len(),
        scene_before + 1,
        "Part Project Curves on Surface must add one scene object"
    );
    assert!(
        last_object_name(&app).to_lowercase().contains("projected"),
        "Part Project Curves on Surface result name should mention projected"
    );
    let (p_after, _, _) = app.overlay_counts();
    assert!(
        p_after > p_before,
        "Part Project Curves on Surface must add at least one overlay polyline"
    );
}

#[test]
fn surface_sections_adds_skinned_solid() {
    let mut app = CadApp::new_headless();
    let scene_before = app.scene_ref().len();

    app.dispatch_surface_sections();

    assert_eq!(
        app.scene_ref().len(),
        scene_before + 1,
        "Surface Sections must add one scene object"
    );
    assert!(
        last_object_name(&app).to_lowercase().contains("sections"),
        "Surface Sections result name should mention sections"
    );
}

#[test]
fn surface_extend_extends_selected_solid() {
    let mut app = CadApp::new_headless();
    seed_selected_box(&mut app);
    let scene_before = app.scene_ref().len();

    app.dispatch_surface_extend();

    assert_eq!(
        app.scene_ref().len(),
        scene_before + 1,
        "Surface Extend must add one scene object"
    );
    assert!(
        last_object_name(&app).to_lowercase().contains("extended"),
        "Surface Extend result name should mention extended"
    );
}

#[test]
fn surface_blend_adds_skinned_solid() {
    let mut app = CadApp::new_headless();
    let scene_before = app.scene_ref().len();

    app.dispatch_surface_blend();

    assert_eq!(
        app.scene_ref().len(),
        scene_before + 1,
        "Surface Blend must add one scene object"
    );
    assert!(
        last_object_name(&app).to_lowercase().contains("blend"),
        "Surface Blend result name should mention blend"
    );
}
