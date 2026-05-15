use cadkernel_viewer::test_support::CadApp;

fn open_title(kind: &str, title: &str) {
    let mut app = CadApp::new_headless();
    assert!(app.open_task_panel_for_test(kind));
    assert_eq!(app.active_task_title_for_test(), Some(title));
}

fn emit_summary(kind: &str, expected: &str) {
    let mut app = CadApp::new_headless();
    assert!(app.open_task_panel_for_test(kind));
    assert_eq!(app.emit_active_task_for_test().as_deref(), Some(expected));
}

fn cancel_panel(kind: &str) {
    let mut app = CadApp::new_headless();
    assert!(app.open_task_panel_for_test(kind));
    app.cancel_active_task_for_test();
    assert!(app.active_task_is_none_for_test());
}

fn assert_dispatch_records_history_or_warns<F>(seed_box: bool, dispatch: F)
where
    F: FnOnce(&mut CadApp),
{
    let mut app = CadApp::new_headless();
    if seed_box {
        app.dispatch_create_box(4.0, 4.0, 4.0);
    }
    let history_before = app.api_history_len_for_test();
    let warnings_before = app.report_warning_count_for_test();
    dispatch(&mut app);
    let history_after = app.api_history_len_for_test();
    let warnings_after = app.report_warning_count_for_test();
    assert!(
        history_after > history_before || warnings_after > warnings_before,
        "expected API history or warning, got history {history_before}->{history_after}, warnings {warnings_before}->{warnings_after}"
    );
}

fn assert_command_stack_unchanged<F>(seed_box: bool, dispatch: F)
where
    F: FnOnce(&mut CadApp),
{
    let mut app = CadApp::new_headless();
    if seed_box {
        app.dispatch_create_box(4.0, 4.0, 4.0);
    }
    let before = app.command_history_len_for_test();
    dispatch(&mut app);
    assert_eq!(app.command_history_len_for_test(), before);
}

macro_rules! panel_open_tests {
    ($($name:ident: $kind:literal => $title:literal),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                open_title($kind, $title);
            }
        )+
    };
}

macro_rules! panel_emit_tests {
    ($($name:ident: $kind:literal => $summary:literal),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                emit_summary($kind, $summary);
            }
        )+
    };
}

macro_rules! panel_cancel_tests {
    ($($name:ident: $kind:literal),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                cancel_panel($kind);
            }
        )+
    };
}

panel_open_tests! {
    panel_pad_opens: "pad" => "Pad Sketch",
    panel_pocket_opens: "pocket" => "Pocket Sketch",
    panel_revolve_opens: "revolve" => "Revolve",
    panel_hole_opens: "hole" => "Create Hole",
    panel_loft_opens: "loft" => "Loft",
    panel_sweep_opens: "sweep" => "Sweep",
    panel_fillet_opens: "fillet" => "Fillet Edges",
    panel_chamfer_opens: "chamfer" => "Chamfer Edges",
    panel_shell_opens: "shell" => "Shell Solid",
    panel_draft_opens: "draft" => "Draft",
}

panel_emit_tests! {
    panel_pad_emits_action_on_ok: "pad" => "partdesign:pad depth=10.000 symmetric=false",
    panel_pocket_emits_action_on_ok: "pocket" => "partdesign:pocket depth=5.000 through_all=false",
    panel_revolve_emits_action_on_ok: "revolve" => "status:Revolve dispatched",
    panel_hole_emits_action_on_ok: "hole" => "partdesign:hole radius=1.000 depth=8.000",
    panel_loft_emits_action_on_ok: "loft" => "partdesign:additive_loft",
    panel_sweep_emits_action_on_ok: "sweep" => "partdesign:additive_pipe",
    panel_fillet_emits_action_on_ok: "fillet" => "fillet radius=1.000",
    panel_chamfer_emits_action_on_ok: "chamfer" => "chamfer distance=1.000",
    panel_shell_emits_action_on_ok: "shell" => "shell thickness=1.000",
    panel_draft_emits_action_on_ok: "draft" => "status:Draft dispatched",
}

panel_cancel_tests! {
    panel_pad_cancel_drops_task: "pad",
    panel_pocket_cancel_drops_task: "pocket",
    panel_revolve_cancel_drops_task: "revolve",
    panel_hole_cancel_drops_task: "hole",
    panel_loft_cancel_drops_task: "loft",
    panel_sweep_cancel_drops_task: "sweep",
    panel_fillet_cancel_drops_task: "fillet",
    panel_chamfer_cancel_drops_task: "chamfer",
    panel_shell_cancel_drops_task: "shell",
    panel_draft_cancel_drops_task: "draft",
}

#[test]
fn pad_dispatches_command() {
    assert_dispatch_records_history_or_warns(true, |app| {
        app.dispatch_pad_sketch_via_session_for_test(3.0, false)
    });
}

#[test]
fn pocket_dispatches_command() {
    assert_dispatch_records_history_or_warns(true, |app| {
        app.dispatch_pocket_sketch_via_session_for_test(2.0, false)
    });
}

#[test]
fn groove_dispatches_command() {
    assert_dispatch_records_history_or_warns(true, |app| {
        app.dispatch_groove_sketch_via_session_for_test(90.0)
    });
}

#[test]
fn hole_dispatches_command() {
    assert_dispatch_records_history_or_warns(true, |app| {
        app.dispatch_hole_sketch_via_session_for_test(0.5, 2.0)
    });
}

#[test]
fn countersunk_hole_dispatches_command() {
    assert_dispatch_records_history_or_warns(true, |app| {
        app.dispatch_countersunk_hole_sketch_via_session_for_test(0.5, 2.0, 90.0);
    });
}

#[test]
fn additive_loft_dispatches_command() {
    assert_dispatch_records_history_or_warns(true, |app| {
        app.dispatch_partdesign_additive_loft_via_session_for_test()
    });
}

#[test]
fn additive_pipe_dispatches_command() {
    assert_dispatch_records_history_or_warns(true, |app| {
        app.dispatch_partdesign_additive_pipe_via_session_for_test()
    });
}

#[test]
fn fillet_dispatches_command() {
    assert_dispatch_records_history_or_warns(true, |app| app.dispatch_fillet_all_edges(0.2));
}

#[test]
fn chamfer_dispatches_command() {
    assert_dispatch_records_history_or_warns(true, |app| app.dispatch_chamfer_all_edges(0.2));
}

#[test]
fn shell_dispatches_command() {
    assert_dispatch_records_history_or_warns(true, |app| app.dispatch_shell_solid(0.2));
}

#[test]
fn helix_dispatches_command() {
    assert_dispatch_records_history_or_warns(false, |app| {
        app.dispatch_create_helix(1.0, 0.5, 2.0, 0.15);
    });
}

#[test]
fn pad_dispatch_no_panic_on_degenerate_input() {
    let mut app = CadApp::new_headless();
    app.dispatch_pad_sketch_via_session_for_test(0.0, false);
}

#[test]
fn pocket_dispatch_no_panic_on_degenerate_input() {
    let mut app = CadApp::new_headless();
    app.dispatch_pocket_sketch_via_session_for_test(0.0, false);
}

#[test]
fn groove_dispatch_no_panic_on_degenerate_input() {
    let mut app = CadApp::new_headless();
    app.dispatch_groove_sketch_via_session_for_test(0.0);
}

#[test]
fn hole_dispatch_no_panic_on_degenerate_input() {
    let mut app = CadApp::new_headless();
    app.dispatch_hole_sketch_via_session_for_test(0.0, 0.0);
}

#[test]
fn countersunk_hole_dispatch_no_panic_on_degenerate_input() {
    let mut app = CadApp::new_headless();
    app.dispatch_countersunk_hole_sketch_via_session_for_test(0.0, 0.0, 0.0);
}

#[test]
fn additive_loft_dispatch_no_panic_without_base() {
    let mut app = CadApp::new_headless();
    app.dispatch_partdesign_additive_loft_via_session_for_test();
}

#[test]
fn additive_pipe_dispatch_no_panic_without_base() {
    let mut app = CadApp::new_headless();
    app.dispatch_partdesign_additive_pipe_via_session_for_test();
}

#[test]
fn fillet_dispatch_no_panic_without_base() {
    let mut app = CadApp::new_headless();
    app.dispatch_fillet_all_edges(0.0);
}

#[test]
fn chamfer_dispatch_no_panic_without_base() {
    let mut app = CadApp::new_headless();
    app.dispatch_chamfer_all_edges(0.0);
}

#[test]
fn shell_dispatch_no_panic_without_base() {
    let mut app = CadApp::new_headless();
    app.dispatch_shell_solid(0.0);
}

#[test]
fn helix_dispatch_no_panic_on_degenerate_input() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_helix(0.0, 0.0, 0.0, 0.0);
}

#[test]
fn pad_does_not_push_snapshot() {
    assert_command_stack_unchanged(true, |app| {
        app.dispatch_pad_sketch_via_session_for_test(3.0, false)
    });
}

#[test]
fn pocket_does_not_push_snapshot() {
    assert_command_stack_unchanged(true, |app| {
        app.dispatch_pocket_sketch_via_session_for_test(2.0, false)
    });
}

#[test]
fn groove_does_not_push_snapshot() {
    assert_command_stack_unchanged(true, |app| {
        app.dispatch_groove_sketch_via_session_for_test(90.0)
    });
}

#[test]
fn hole_does_not_push_snapshot() {
    assert_command_stack_unchanged(true, |app| {
        app.dispatch_hole_sketch_via_session_for_test(0.5, 2.0)
    });
}

#[test]
fn countersunk_hole_does_not_push_snapshot() {
    assert_command_stack_unchanged(true, |app| {
        app.dispatch_countersunk_hole_sketch_via_session_for_test(0.5, 2.0, 90.0);
    });
}

#[test]
fn additive_loft_does_not_push_snapshot() {
    assert_command_stack_unchanged(true, |app| {
        app.dispatch_partdesign_additive_loft_via_session_for_test()
    });
}

#[test]
fn additive_pipe_does_not_push_snapshot() {
    assert_command_stack_unchanged(true, |app| {
        app.dispatch_partdesign_additive_pipe_via_session_for_test()
    });
}

#[test]
fn fillet_does_not_push_snapshot() {
    assert_command_stack_unchanged(true, |app| app.dispatch_fillet_all_edges(0.2));
}

#[test]
fn chamfer_does_not_push_snapshot() {
    assert_command_stack_unchanged(true, |app| app.dispatch_chamfer_all_edges(0.2));
}

#[test]
fn shell_does_not_push_snapshot() {
    assert_command_stack_unchanged(true, |app| app.dispatch_shell_solid(0.2));
}

#[test]
fn helix_does_not_push_snapshot() {
    assert_command_stack_unchanged(false, |app| {
        app.dispatch_create_helix(1.0, 0.5, 2.0, 0.15);
    });
}

#[test]
fn pad_panel_passes_symmetric_through() {
    emit_summary(
        "pad_symmetric",
        "partdesign:pad depth=10.000 symmetric=true",
    );
}

#[test]
fn pocket_panel_passes_through_all_through() {
    emit_summary(
        "pocket_through_all",
        "partdesign:pocket depth=5.000 through_all=true",
    );
}

#[test]
fn hole_panel_maps_countersink_to_countersunk_action() {
    emit_summary(
        "hole_countersink",
        "partdesign:countersunk_hole radius=1.000 depth=8.000 angle=82.000",
    );
}

#[test]
fn revolve_panel_default_axis_is_z() {
    let mut app = CadApp::new_headless();
    assert!(app.open_task_panel_for_test("revolve"));
    assert_eq!(
        app.active_task_summary_for_test().as_deref(),
        Some("revolve axis=2 angle=360.000 symmetric=false")
    );
}

#[test]
fn loft_panel_tracks_smooth_ruled_closed_fields() {
    let mut app = CadApp::new_headless();
    assert!(app.open_task_panel_for_test("loft_smooth_closed"));
    assert_eq!(
        app.active_task_summary_for_test().as_deref(),
        Some("loft mode=1 ruled=true closed=true")
    );
}

#[test]
fn sweep_panel_tracks_auxiliary_mode() {
    let mut app = CadApp::new_headless();
    assert!(app.open_task_panel_for_test("sweep_auxiliary"));
    assert_eq!(
        app.active_task_summary_for_test().as_deref(),
        Some("sweep mode=2")
    );
}

#[test]
fn draft_panel_tracks_push_direction() {
    let mut app = CadApp::new_headless();
    assert!(app.open_task_panel_for_test("draft_push"));
    assert_eq!(
        app.active_task_summary_for_test().as_deref(),
        Some("draft angle=-5.000 direction=1")
    );
}

#[test]
fn fillet_panel_tracks_radius() {
    let mut app = CadApp::new_headless();
    assert!(app.open_task_panel_for_test("fillet"));
    assert_eq!(
        app.active_task_summary_for_test().as_deref(),
        Some("fillet radius=1.000")
    );
}

#[test]
fn chamfer_panel_tracks_distance() {
    let mut app = CadApp::new_headless();
    assert!(app.open_task_panel_for_test("chamfer"));
    assert_eq!(
        app.active_task_summary_for_test().as_deref(),
        Some("chamfer distance=1.000")
    );
}

#[test]
fn shell_panel_tracks_thickness() {
    let mut app = CadApp::new_headless();
    assert!(app.open_task_panel_for_test("shell"));
    assert_eq!(
        app.active_task_summary_for_test().as_deref(),
        Some("shell thickness=1.000")
    );
}
