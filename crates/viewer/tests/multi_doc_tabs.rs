use cadkernel_viewer::test_support::CadApp;

#[test]
fn new_tab_switches_active_document() {
    let mut app = CadApp::new_headless();
    assert_eq!(app.document_tab_count_for_test(), 1);
    assert_eq!(app.active_document_for_test(), 0);

    app.dispatch_new_tab_for_test();

    assert_eq!(app.document_tab_count_for_test(), 2);
    assert_eq!(app.active_document_for_test(), 1);
    assert_eq!(app.document_tab_names_for_test()[1], "Untitled 2");
}

#[test]
fn dirty_tab_requires_close_confirmation() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(1.0, 2.0, 3.0);
    assert_eq!(app.document_tab_dirty_for_test(0), Some(true));

    app.dispatch_new_tab_for_test();
    app.dispatch_close_tab_for_test(0);

    assert!(app.close_tab_confirm_is_open_for_test());
    assert_eq!(app.document_tab_count_for_test(), 2);

    app.dispatch_confirm_close_tab_for_test(0);
    assert_eq!(app.document_tab_count_for_test(), 1);
}

#[test]
fn switching_tabs_restores_session_backing_scene() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(1.0, 1.0, 1.0);
    assert_eq!(app.scene_ref().len(), 1);

    app.dispatch_new_tab_for_test();
    assert_eq!(app.scene_ref().len(), 0);

    app.dispatch_switch_tab_for_test(0);
    assert_eq!(app.scene_ref().len(), 1);
}
