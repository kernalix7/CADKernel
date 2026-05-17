use cadkernel_api::{Command, Session};
use cadkernel_viewer::test_support::CadApp;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn temp_autosave_dir(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_nanos();
    std::env::temp_dir().join(format!(
        "cadkernel-viewer-{name}-{}-{stamp}",
        std::process::id()
    ))
}

fn write_snapshot(dir: &std::path::Path, dx: f64) {
    let mut session = Session::new();
    session
        .execute(Command::CreateBox {
            dx,
            dy: 1.0,
            dz: 1.0,
        })
        .expect("create box");
    let policy = cadkernel_api::cadk::AutosavePolicy::new(dir, Duration::from_secs(30), 10);
    session
        .write_autosave_snapshot(&policy)
        .expect("write autosave");
}

#[test]
fn startup_scan_lists_newest_five_autosaves() {
    let dir = temp_autosave_dir("scan");
    for i in 0..6 {
        write_snapshot(&dir, i as f64 + 1.0);
        std::thread::sleep(Duration::from_millis(2));
    }

    let mut app = CadApp::new_headless();
    app.set_autosave_dir_for_test(dir.clone());
    app.run_autosave_recovery_scan_for_test();

    assert_eq!(app.autosave_recovery_entry_count_for_test(), 5);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn recover_choice_loads_session_and_clears_snapshots() {
    let dir = temp_autosave_dir("recover");
    write_snapshot(&dir, 5.0);

    let mut app = CadApp::new_headless();
    app.set_autosave_dir_for_test(dir.clone());
    app.run_autosave_recovery_scan_for_test();
    app.choose_autosave_recovery_for_test(0);

    assert_eq!(app.api_history_len_for_test(), 1);
    assert_eq!(app.scene_ref().len(), 1);
    assert_eq!(
        cadkernel_api::cadk::list_snapshots(&dir)
            .expect("list snapshots")
            .len(),
        0
    );
    let _ = std::fs::remove_dir_all(dir);
}
