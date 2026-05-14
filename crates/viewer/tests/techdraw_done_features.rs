use cadkernel_viewer::test_support::CadApp;

#[test]
fn techdraw_redraw_without_sheet_reports_warning_status() {
    let mut app = CadApp::new_headless();

    app.dispatch_techdraw_redraw();

    assert!(app.techdraw_sheet_size().is_none());
    assert_eq!(
        app.status_message(),
        "TechDraw Redraw: no sheet open (use New Page first)"
    );
}
