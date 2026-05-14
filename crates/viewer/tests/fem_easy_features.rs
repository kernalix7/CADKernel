use cadkernel_viewer::test_support::CadApp;

#[test]
fn fem_summary_with_seeded_analysis_reports_status_text() {
    let mut app = CadApp::new_headless();
    app.seed_test_fem_analysis();

    app.dispatch_fem_summary();

    let status = app.status_message();
    assert!(status.contains("FEM Summary"));
    assert!(status.contains("material:"));
    assert!(status.contains("nodes:"));
    assert!(status.contains("elements:"));
    assert!(status.contains("BCs:"));
    assert!(status.contains("solved:"));
}

#[test]
fn fem_summary_without_analysis_reports_warning_status() {
    let mut app = CadApp::new_headless();

    app.dispatch_fem_summary();

    assert_eq!(
        app.status_message(),
        "FEM Summary: no analysis (run CreateFemAnalysis first)"
    );
}

#[test]
fn fem_report_with_seeded_analysis_reports_status_text() {
    let mut app = CadApp::new_headless();
    app.seed_test_fem_analysis();

    app.dispatch_fem_report();

    let status = app.status_message();
    assert!(status.contains("FEM Report"));
    assert!(status.contains("material:"));
    assert!(status.contains("mesh:"));
    assert!(status.contains("BCs:"));
}

#[test]
fn fem_report_without_analysis_reports_warning_status() {
    let mut app = CadApp::new_headless();

    app.dispatch_fem_report();

    assert_eq!(
        app.status_message(),
        "FEM Report: no analysis (run CreateFemAnalysis first)"
    );
}
