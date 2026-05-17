use cadkernel_viewer::test_support::CadApp;

#[test]
fn theme_mode_accepts_system_follow() {
    let mut app = CadApp::new_headless();
    app.set_theme_mode_for_test("system");
    assert_eq!(app.theme_mode_label_for_test(), "System");
}

#[test]
fn theme_mode_can_return_to_explicit_modes() {
    let mut app = CadApp::new_headless();
    app.set_theme_mode_for_test("light");
    assert_eq!(app.theme_mode_label_for_test(), "Light");
    app.set_theme_mode_for_test("dark");
    assert_eq!(app.theme_mode_label_for_test(), "Dark");
}
