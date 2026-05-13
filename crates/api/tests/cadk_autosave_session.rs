//! A3.1 task #3 — `Session::write_autosave_snapshot` end-to-end.
//!
//! Verifies:
//! - filenames follow the `autosave-{epoch_ms}-{hash16}.cadk` pattern,
//! - written snapshots round-trip through `load_cadk_from_path`,
//! - `canonical_hash` is preserved across the save/load roundtrip,
//! - rotation enforces the `retain` window,
//! - `policy.compress` propagates to the cadk `DOCUMENT_COMPRESSED` flag.

use cadkernel_api::cadk::{AutosavePolicy, inspect_path, list_snapshots};
use cadkernel_api::{Command, Session, SolidId};
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
    std::env::temp_dir().join(format!("cadk-autosave-sess-{label}-{pid}-{nanos}-{n}"))
}

fn r1_log() -> Vec<Command> {
    vec![
        Command::CreateBox {
            dx: 50.0,
            dy: 30.0,
            dz: 10.0,
        },
        Command::CreateCylinder {
            radius: 3.0,
            height: 12.0,
        },
        Command::BooleanSubtract {
            lhs: SolidId(0),
            rhs: SolidId(1),
        },
    ]
}

#[test]
fn write_autosave_creates_file_matching_filename_pattern() {
    let dir = unique_tmp_dir("fname-pattern");
    let policy = AutosavePolicy::new(&dir, Duration::from_secs(30), 5);
    let session = Session::replay(&r1_log()).expect("replay");

    let path = session
        .write_autosave_snapshot(&policy)
        .expect("write autosave");

    assert!(path.exists(), "snapshot file must exist on disk");
    assert_eq!(path.extension().and_then(|e| e.to_str()), Some("cadk"));
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .expect("utf-8 filename");
    assert!(name.starts_with("autosave-"), "got {name}");
    // Body matches autosave-{digits}-{16 hex}.cadk
    let stem = name.trim_start_matches("autosave-").trim_end_matches(".cadk");
    let (ts, hash16) = stem.split_once('-').expect("epoch-hash split");
    assert!(ts.chars().all(|c| c.is_ascii_digit()), "epoch={ts}");
    assert_eq!(hash16.len(), 16, "hash16 must be 16 chars, got {hash16}");
    assert!(
        hash16.chars().all(|c| c.is_ascii_hexdigit()),
        "hash16={hash16} not hex"
    );
}

#[test]
fn snapshot_round_trips_through_load_cadk_from_path() {
    let dir = unique_tmp_dir("roundtrip");
    let policy = AutosavePolicy::new(&dir, Duration::from_secs(30), 5);
    let session = Session::replay(&r1_log()).expect("replay");
    let path = session
        .write_autosave_snapshot(&policy)
        .expect("write autosave");

    let reloaded = Session::load_cadk_from_path(&path).expect("load");
    assert_eq!(reloaded.log(), session.log(), "log must roundtrip");
    assert_eq!(reloaded.document().solid_count(), 1);
}

#[test]
fn canonical_hash_preserved_across_autosave_roundtrip() {
    let dir = unique_tmp_dir("hash-rt");
    let policy = AutosavePolicy::new(&dir, Duration::from_secs(30), 5);
    let session = Session::replay(&r1_log()).expect("replay");
    let before = session.canonical_hash();

    let path = session
        .write_autosave_snapshot(&policy)
        .expect("write autosave");
    let reloaded = Session::load_cadk_from_path(&path).expect("load");
    assert_eq!(reloaded.canonical_hash(), before);
}

#[test]
fn interval_is_not_enforced_by_writer() {
    // The viewer owns the interval gate; the writer always writes.
    // Two back-to-back writes inside the same millisecond must both
    // succeed.
    let dir = unique_tmp_dir("no-interval");
    // Long interval, but the writer must ignore it.
    let policy = AutosavePolicy::new(&dir, Duration::from_secs(3600), 5);
    let session = Session::replay(&r1_log()).expect("replay");

    let _ = session.write_autosave_snapshot(&policy).expect("write 1");
    let _ = session.write_autosave_snapshot(&policy).expect("write 2");

    let entries = list_snapshots(&dir).expect("list");
    // Could be 1 (filenames collided on the same epoch_ms+hash) or
    // 2 (clock advanced one ms between calls). Either is acceptable
    // — what matters is that neither write erred.
    assert!(!entries.is_empty());
    assert!(entries.len() <= 2);
}

#[test]
fn rotation_enforces_retain_window() {
    let dir = unique_tmp_dir("rotation");
    let policy = AutosavePolicy::new(&dir, Duration::from_secs(30), 3);
    let session = Session::replay(&r1_log()).expect("replay");

    // Drive 5 writes with a label-change command between each so each
    // produces a distinct canonical_hash and therefore a distinct
    // filename, even at sub-millisecond clock resolution. After
    // BooleanSubtract the surviving solid id is whatever
    // `solid_ids()` reports first.
    for i in 0..5 {
        let mut s = session.clone_for_autosave_test();
        let id = *s
            .document()
            .solid_ids()
            .first()
            .expect("at least one solid");
        s.execute(Command::Rename {
            id,
            label: format!("part-{i}"),
        })
        .expect("rename");
        s.write_autosave_snapshot(&policy)
            .expect("write autosave");
        std::thread::sleep(Duration::from_millis(15));
    }

    let entries = list_snapshots(&dir).expect("list");
    assert_eq!(entries.len(), 3, "retain=3 must cap directory at 3 files");
}

#[test]
fn compress_true_sets_document_compressed_flag() {
    let dir = unique_tmp_dir("compress-flag");
    let policy = AutosavePolicy::new(&dir, Duration::from_secs(30), 5).with_compression(true);
    let session = Session::replay(&r1_log()).expect("replay");

    let path = session
        .write_autosave_snapshot(&policy)
        .expect("write autosave");
    let summary = inspect_path(&path).expect("inspect");
    assert!(
        summary.document_compressed(),
        "compress=true must set DOCUMENT_COMPRESSED bit"
    );
}

#[test]
fn compress_false_clears_document_compressed_flag() {
    let dir = unique_tmp_dir("no-compress");
    let policy = AutosavePolicy::new(&dir, Duration::from_secs(30), 5);
    // policy.compress defaults to false.
    let session = Session::replay(&r1_log()).expect("replay");

    let path = session
        .write_autosave_snapshot(&policy)
        .expect("write autosave");
    let summary = inspect_path(&path).expect("inspect");
    assert!(
        !summary.document_compressed(),
        "compress=false must leave DOCUMENT_COMPRESSED bit clear"
    );
}

#[test]
fn write_creates_dir_on_demand() {
    let mut dir = unique_tmp_dir("create-dir");
    dir.push("deeply");
    dir.push("nested");
    dir.push("autosaves");
    assert!(!dir.exists());

    let policy = AutosavePolicy::new(&dir, Duration::from_secs(30), 5);
    let session = Session::replay(&r1_log()).expect("replay");
    let path = session
        .write_autosave_snapshot(&policy)
        .expect("write autosave");
    assert!(dir.is_dir(), "writer must create dir on demand");
    assert!(path.starts_with(&dir));
}

// --- test-only helpers ---------------------------------------------------

trait SessionTestExt {
    /// Build a fresh Session from this one's applied log, used by tests
    /// that need to mutate state without affecting the caller's session.
    fn clone_for_autosave_test(&self) -> Session;
}

impl SessionTestExt for Session {
    fn clone_for_autosave_test(&self) -> Session {
        Session::replay(self.log()).expect("replay clone")
    }
}
