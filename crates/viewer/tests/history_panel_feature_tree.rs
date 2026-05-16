use cadkernel_viewer::test_support::CadApp;

fn seeded_tree() -> (CadApp, u64, Vec<u64>) {
    let mut app = CadApp::new_headless();
    app.seed_api_feature_body_for_test();
    let tree = app.api_body_tree_for_test();
    assert_eq!(tree.len(), 1, "expected one seeded body");
    let body_id = tree[0].0;
    let features = tree[0].3.iter().map(|entry| entry.0).collect();
    (app, body_id, features)
}

#[test]
fn tree_snapshot_renders_body_node() {
    let (mut app, body_id, _) = seeded_tree();
    let tree = app.api_body_tree_for_test();
    assert_eq!(tree[0].0, body_id);
    assert!(tree[0].1.starts_with("Body"));
}

#[test]
fn tree_snapshot_renders_features_in_order() {
    let (mut app, _, features) = seeded_tree();
    let tree = app.api_body_tree_for_test();
    let rendered: Vec<_> = tree[0].3.iter().map(|entry| entry.0).collect();
    assert_eq!(rendered, features);
}

#[test]
fn tree_snapshot_marks_last_feature_as_tip() {
    let (mut app, _, features) = seeded_tree();
    let tree = app.api_body_tree_for_test();
    assert_eq!(tree[0].2, features.last().copied());
    assert!(tree[0].3.last().expect("feature").4);
}

#[test]
fn tree_snapshot_exposes_feature_kind_icon_source() {
    let (mut app, _, _) = seeded_tree();
    let tree = app.api_body_tree_for_test();
    assert!(tree[0].3.iter().all(|entry| entry.2 == "helix"));
}

#[test]
fn toggle_suppression_dispatches_suppress_feature() {
    let (mut app, _, features) = seeded_tree();
    app.dispatch_feature_suppression_for_test(features[0], true);
    let tree = app.api_body_tree_for_test();
    assert!(tree[0].3[0].3, "feature should be suppressed");
}

#[test]
fn toggle_suppression_can_restore_feature() {
    let (mut app, _, features) = seeded_tree();
    app.dispatch_feature_suppression_for_test(features[0], true);
    app.dispatch_feature_suppression_for_test(features[0], false);
    let tree = app.api_body_tree_for_test();
    assert!(!tree[0].3[0].3, "feature should be active again");
}

#[test]
fn drag_reorder_down_dispatches_reorder_feature() {
    let (mut app, _, features) = seeded_tree();
    app.dispatch_feature_reorder_for_test(features[0], 1);
    let tree = app.api_body_tree_for_test();
    assert_eq!(tree[0].3[1].0, features[0]);
}

#[test]
fn drag_reorder_up_dispatches_reorder_feature() {
    let (mut app, _, features) = seeded_tree();
    app.dispatch_feature_reorder_for_test(features[1], 0);
    let tree = app.api_body_tree_for_test();
    assert_eq!(tree[0].3[0].0, features[1]);
}

#[test]
fn context_menu_set_as_tip_dispatches_set_tip() {
    let (mut app, body_id, features) = seeded_tree();
    app.dispatch_feature_set_tip_for_test(body_id, features[0]);
    let tree = app.api_body_tree_for_test();
    assert_eq!(tree[0].2, Some(features[0]));
    assert!(tree[0].3[0].4);
}

#[test]
fn double_click_edit_dispatches_edit_feature() {
    let (mut app, _, features) = seeded_tree();
    app.dispatch_feature_edit_helix_for_test(features[0], 2.75);
    let spec = app
        .api_feature_spec_json_for_test(features[0])
        .expect("feature spec");
    assert!(spec.contains("\"height\": 2.75"));
}

#[test]
fn branch_indicator_marks_linear_history() {
    let (mut app, _, _) = seeded_tree();
    let label = app
        .feature_tree_branch_label_for_test()
        .expect("branch label");
    assert!(label.starts_with("Linear history"));
}

#[test]
fn body_tree_tracks_feature_count_after_editing() {
    let (mut app, _, features) = seeded_tree();
    app.dispatch_feature_suppression_for_test(features[0], true);
    app.dispatch_feature_reorder_for_test(features[1], 0);
    let tree = app.api_body_tree_for_test();
    assert_eq!(tree[0].3.len(), 2);
}
