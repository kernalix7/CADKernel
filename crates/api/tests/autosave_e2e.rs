//! A3.2 end-to-end autosave + recovery integration test.
//!
//! Verifies the full pipeline that A3.2 activates:
//! Session::execute → write_autosave_snapshot → recover_latest →
//! load_cadk_from_path → canonical_hash preserved + document state correct.

use cadkernel_api::cadk::{AutosavePolicy, recover_latest};
use cadkernel_api::{Command, Session};
use std::path::PathBuf;
use std::time::Duration;

fn unique_tmp_dir(label: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let pid = std::process::id();
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("cadk-e2e-{label}-{pid}-{nanos}-{n}"));
    std::fs::create_dir_all(&dir).expect("temp dir create");
    dir
}

#[test]
fn autosave_e2e_createbox_roundtrip() {
    let dir = unique_tmp_dir("e2e-box");
    let policy = AutosavePolicy::new(&dir, Duration::from_secs(60), 3).with_compression(true);

    let mut session = Session::new();
    session
        .execute(Command::CreateBox {
            dx: 50.0,
            dy: 30.0,
            dz: 10.0,
        })
        .expect("CreateBox");

    let hash_before = session.canonical_hash();
    assert_eq!(session.document().solid_count(), 1);

    let path = session
        .write_autosave_snapshot(&policy)
        .expect("write_autosave_snapshot");
    assert!(path.exists(), "snapshot must exist on disk");

    let entry = recover_latest(&dir)
        .expect("recover_latest")
        .expect("at least one snapshot");
    assert_eq!(entry.path, path, "recover_latest must return the written snapshot");

    let recovered = Session::load_cadk_from_path(&entry.path).expect("load_cadk_from_path");
    assert_eq!(
        recovered.canonical_hash(),
        hash_before,
        "canonical_hash must survive autosave roundtrip"
    );
    assert_eq!(
        recovered.document().solid_count(),
        1,
        "recovered document must contain exactly 1 solid"
    );
}

#[test]
fn autosave_e2e_retain_enforced() {
    let dir = unique_tmp_dir("e2e-retain");
    let policy = AutosavePolicy::new(&dir, Duration::from_secs(60), 3);

    let mut session = Session::new();
    session
        .execute(Command::CreateBox {
            dx: 10.0,
            dy: 10.0,
            dz: 10.0,
        })
        .expect("CreateBox");

    for i in 0..5u32 {
        let id = *session
            .document()
            .solid_ids()
            .first()
            .expect("solid exists");
        session
            .execute(Command::Rename {
                id,
                label: format!("part-{i}"),
            })
            .expect("rename");
        session
            .write_autosave_snapshot(&policy)
            .expect("write snapshot");
        std::thread::sleep(Duration::from_millis(15));
    }

    let entries = cadkernel_api::cadk::list_snapshots(&dir).expect("list");
    assert_eq!(entries.len(), 3, "retain=3 must cap the directory at 3 files");
}

#[test]
fn autosave_e2e_empty_session_produces_valid_snapshot() {
    let dir = unique_tmp_dir("e2e-empty");
    let policy = AutosavePolicy::new(&dir, Duration::from_secs(60), 5);

    let session = Session::new();
    let path = session
        .write_autosave_snapshot(&policy)
        .expect("write empty session snapshot");
    assert!(path.exists());

    let recovered = Session::load_cadk_from_path(&path).expect("load empty snapshot");
    assert_eq!(recovered.document().solid_count(), 0);
    assert_eq!(recovered.canonical_hash(), session.canonical_hash());
}

#[test]
fn autosave_e2e_compression_flag_set() {
    let dir = unique_tmp_dir("e2e-compress");
    let policy = AutosavePolicy::new(&dir, Duration::from_secs(60), 5).with_compression(true);

    let mut session = Session::new();
    session
        .execute(Command::CreateBox {
            dx: 20.0,
            dy: 20.0,
            dz: 20.0,
        })
        .expect("CreateBox");

    let path = session
        .write_autosave_snapshot(&policy)
        .expect("write snapshot");

    let summary = cadkernel_api::cadk::inspect_path(&path).expect("inspect");
    assert!(
        summary.document_compressed(),
        "with_compression(true) must set DOCUMENT_COMPRESSED flag"
    );
}
