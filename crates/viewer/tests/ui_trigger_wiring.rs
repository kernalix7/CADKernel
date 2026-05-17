use cadkernel_viewer::test_support::CadApp;

const MENU_RS: &str = include_str!("../src/gui/menu.rs");
const TOOLBAR_RS: &str = include_str!("../src/gui/toolbar.rs");
const DIALOGS_RS: &str = include_str!("../src/gui/dialogs.rs");
const GUI_MOD_RS: &str = include_str!("../src/gui/mod.rs");
const APP_RS: &str = include_str!("../src/app.rs");
const SKETCH_UI_RS: &str = include_str!("../src/gui/sketch_ui.rs");

#[test]
fn view_menu_has_section_box_trigger() {
    assert!(MENU_RS.contains("\"Section Box\""));
    assert!(MENU_RS.contains("\"Shift+B\""));
    assert!(MENU_RS.contains("GuiAction::ToggleSectionBox"));
}

#[test]
fn section_box_action_still_toggles_scene_state() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(1.0, 1.0, 1.0);
    app.dispatch_toggle_section_box_for_test();
    assert!(app.scene_ref().section_box.active);
}

#[test]
fn file_menu_has_new_and_close_tab_triggers() {
    assert!(MENU_RS.contains("\"menu.file.new_tab\""));
    assert!(MENU_RS.contains("shortcut_text(\"Ctrl+T\")"));
    assert!(MENU_RS.contains("GuiAction::NewTab"));
    assert!(MENU_RS.contains("GuiAction::CloseTab(gui.active_document)"));
}

#[test]
fn keyboard_shortcut_ctrl_t_dispatches_new_tab() {
    assert!(APP_RS.contains("KeyCode::KeyT) if ctrl"));
    assert!(APP_RS.contains("GuiAction::NewTab"));
}

#[test]
fn dropped_cad_files_open_in_new_tab_path() {
    assert!(APP_RS.contains("WindowEvent::DroppedFile(path)"));
    assert!(APP_RS.contains("GuiAction::OpenFileInNewTab(path.clone())"));
}

#[test]
fn settings_dialog_has_language_combo() {
    assert!(DIALOGS_RS.contains("ComboBox::from_id_salt(\"language\")"));
    assert!(DIALOGS_RS.contains("Language::ALL"));
}

#[test]
fn settings_dialog_theme_combo_exposes_three_modes() {
    assert!(DIALOGS_RS.contains("ComboBox::from_id_salt(\"theme_mode\")"));
    assert!(DIALOGS_RS.contains("ThemeMode::ALL"));
    let mut app = CadApp::new_headless();
    app.set_theme_mode_for_test("system");
    assert_eq!(app.theme_mode_label_for_test(), "System");
}

#[test]
fn edit_menu_has_branching_undo_entries() {
    assert!(MENU_RS.contains("\"Save Checkpoint...\""));
    assert!(MENU_RS.contains("\"Restore Checkpoint...\""));
    assert!(MENU_RS.contains("\"Branches...\""));
}

#[test]
fn checkpoint_save_dialog_opens_from_action() {
    let mut app = CadApp::new_headless();
    app.dispatch_open_checkpoint_save_for_test();
    assert!(app.checkpoint_save_dialog_is_open_for_test());
}

#[test]
fn checkpoint_save_and_restore_are_wired_to_session() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(1.0, 1.0, 1.0);
    app.dispatch_save_checkpoint_for_test("base");
    assert_eq!(app.checkpoint_count_for_test(), 1);

    app.dispatch_create_box(2.0, 2.0, 2.0);
    assert_eq!(app.session_solid_count_for_test(), 2);
    assert!(app.dispatch_restore_first_checkpoint_for_test());
    assert_eq!(app.session_solid_count_for_test(), 1);
}

#[test]
fn restore_checkpoint_dialog_lists_saved_checkpoints() {
    let mut app = CadApp::new_headless();
    app.dispatch_save_checkpoint_for_test("empty");
    app.dispatch_open_restore_checkpoint_for_test();
    assert_eq!(app.checkpoint_restore_dialog_count_for_test(), 1);
}

#[test]
fn branch_dialog_lists_branch_created_by_restore() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(1.0, 1.0, 1.0);
    app.dispatch_save_checkpoint_for_test("base");
    app.dispatch_create_box(2.0, 2.0, 2.0);
    assert!(app.dispatch_restore_first_checkpoint_for_test());
    assert_eq!(app.branch_count_for_test(), 1);
    app.dispatch_open_branch_list_for_test();
    assert_eq!(app.branch_list_dialog_count_for_test(), 1);
}

#[test]
fn techdraw_menu_has_gdt_frame_trigger() {
    assert!(MENU_RS.contains("\"Add GD&T Frame\""));
    assert!(MENU_RS.contains("GuiAction::OpenGdtFrameDialog"));
}

#[test]
fn techdraw_toolbar_has_gdt_frame_button() {
    assert!(TOOLBAR_RS.contains("\"GD&T\""));
    assert!(TOOLBAR_RS.contains("\"Add GD&T frame\""));
    assert!(TOOLBAR_RS.contains("GuiAction::OpenGdtFrameDialog"));
}

#[test]
fn gdt_dialog_commits_frame_to_active_sheet() {
    let mut app = CadApp::new_headless();
    app.dispatch_techdraw_new_page();
    app.dispatch_open_gdt_frame_dialog_for_test();
    assert!(app.gdt_frame_dialog_is_open_for_test());
    app.set_gdt_frame_for_test(0.125, "A");
    assert!(app.dispatch_commit_gdt_frame_for_test());

    let (text_count, _, _, _, _, _) = app.techdraw_annotation_counts();
    assert_eq!(text_count, 1);
    let svg = app.techdraw_svg().expect("techdraw svg");
    assert!(svg.contains("0.125 [A]"));
}

#[test]
fn gdt_dialog_is_registered_in_active_dialogs() {
    assert!(GUI_MOD_RS.contains("ActiveDialog::GdtFrame"));
    assert!(DIALOGS_RS.contains("draw_gdt_frame_dialog"));
}

#[test]
fn sketch_overlay_calls_conflict_detector() {
    assert!(SKETCH_UI_RS.contains("detect_sketch_conflicts"));
    assert!(SKETCH_UI_RS.contains("cadkernel_sketch::detect_conflict"));
}

#[test]
fn sketch_conflict_warning_uses_overconstrained_status_text() {
    assert!(SKETCH_UI_RS.contains("Over-constrained: {} conflicts"));
    assert!(SKETCH_UI_RS.contains("CONFLICT_COLOR"));
    assert!(SKETCH_UI_RS.contains("conflict_ids.contains(&idx)"));
}

#[test]
fn assembly_menu_has_add_mate_trigger() {
    assert!(MENU_RS.contains("\"Add Mate...\""));
    assert!(MENU_RS.contains("GuiAction::OpenAddMateDialog"));
}

#[test]
fn assembly_toolbar_has_add_mate_button() {
    assert!(TOOLBAR_RS.contains("\"Add Mate\""));
    assert!(TOOLBAR_RS.contains("\"Add assembly mate\""));
    assert!(TOOLBAR_RS.contains("GuiAction::OpenAddMateDialog"));
}

#[test]
fn add_mate_dialog_opens_from_action() {
    let mut app = CadApp::new_headless();
    app.dispatch_open_add_mate_dialog_for_test();
    assert!(app.add_mate_dialog_is_open_for_test());
}
