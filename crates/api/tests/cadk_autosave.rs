//! A3.1 task #2 — `cadk::autosave` directory primitives.
//!
//! Exercises [`list_snapshots`], [`prune`], and [`recover_latest`] in
//! isolation from the [`crate::Session`] writer (task #3 covers that).
//! Tests use a per-test temp directory under `std::env::temp_dir()` so
//! parallel invocations do not collide.

use cadkernel_api::cadk::{AutosaveEntry, list_snapshots, prune, recover_latest};
use std::path::{Path, PathBuf};

fn unique_tmp_dir(label: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let pid = std::process::id();
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("cadk-autosave-{label}-{pid}-{nanos}-{n}"));
    std::fs::create_dir_all(&dir).expect("temp dir create");
    dir
}

/// Write a recognised autosave snapshot at `dir/name`. Content is
/// a single-byte tag so each file has a known length and distinct
/// content from its neighbours.
fn write_snapshot(dir: &Path, name: &str, tag: u8) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, [tag; 16]).expect("write snapshot");
    path
}

/// Sleep just long enough that the next file's modification time is
/// strictly greater than ours on the target filesystem. 25 ms is
/// comfortably above the Linux jiffy and macOS HFS+/APFS resolution.
fn bump_mtime() {
    std::thread::sleep(std::time::Duration::from_millis(25));
}

fn paths(entries: &[AutosaveEntry]) -> Vec<PathBuf> {
    entries.iter().map(|e| e.path.clone()).collect()
}

#[test]
fn list_snapshots_returns_newest_first() {
    let dir = unique_tmp_dir("newest-first");
    let a = write_snapshot(&dir, "autosave-100-aaaa.cadk", 1);
    bump_mtime();
    let b = write_snapshot(&dir, "autosave-200-bbbb.cadk", 2);
    bump_mtime();
    let c = write_snapshot(&dir, "autosave-300-cccc.cadk", 3);

    let entries = list_snapshots(&dir).expect("list");
    assert_eq!(
        paths(&entries),
        vec![c, b, a],
        "newest mtime must come first"
    );
}

#[test]
fn list_snapshots_skips_non_cadk_files() {
    let dir = unique_tmp_dir("skip-ext");
    let keep = write_snapshot(&dir, "autosave-100-aaaa.cadk", 1);
    // Foreign extensions — must be filtered out.
    write_snapshot(&dir, "autosave-200-bbbb.txt", 2);
    write_snapshot(&dir, "autosave-300-cccc", 3);
    let entries = list_snapshots(&dir).expect("list");
    assert_eq!(paths(&entries), vec![keep]);
}

#[test]
fn list_snapshots_skips_cadk_without_autosave_prefix() {
    let dir = unique_tmp_dir("skip-prefix");
    let keep = write_snapshot(&dir, "autosave-100-aaaa.cadk", 1);
    // `.cadk` files that the user saved manually — must not be
    // treated as autosave snapshots.
    write_snapshot(&dir, "manual-save.cadk", 2);
    write_snapshot(&dir, "release-v0.5.cadk", 3);
    let entries = list_snapshots(&dir).expect("list");
    assert_eq!(paths(&entries), vec![keep]);
}

#[test]
fn list_snapshots_on_missing_dir_returns_empty_ok() {
    let mut dir = unique_tmp_dir("missing");
    dir.push("does-not-exist");
    let entries = list_snapshots(&dir).expect("missing dir → Ok([])");
    assert!(entries.is_empty());
}

#[test]
fn prune_with_retain_zero_deletes_all() {
    let dir = unique_tmp_dir("prune-zero");
    write_snapshot(&dir, "autosave-100-aaaa.cadk", 1);
    bump_mtime();
    write_snapshot(&dir, "autosave-200-bbbb.cadk", 2);
    bump_mtime();
    write_snapshot(&dir, "autosave-300-cccc.cadk", 3);

    let removed = prune(&dir, 0).expect("prune");
    assert_eq!(removed, 3);
    let entries = list_snapshots(&dir).expect("list");
    assert!(entries.is_empty());
}

#[test]
fn prune_with_retain_above_len_deletes_nothing() {
    let dir = unique_tmp_dir("prune-noop");
    write_snapshot(&dir, "autosave-100-aaaa.cadk", 1);
    bump_mtime();
    write_snapshot(&dir, "autosave-200-bbbb.cadk", 2);

    let removed = prune(&dir, 10).expect("prune");
    assert_eq!(removed, 0);
    let entries = list_snapshots(&dir).expect("list");
    assert_eq!(entries.len(), 2);
}

#[test]
fn prune_keeps_the_n_newest() {
    let dir = unique_tmp_dir("prune-keep-n");
    let a = write_snapshot(&dir, "autosave-100-aaaa.cadk", 1);
    bump_mtime();
    let b = write_snapshot(&dir, "autosave-200-bbbb.cadk", 2);
    bump_mtime();
    let c = write_snapshot(&dir, "autosave-300-cccc.cadk", 3);
    bump_mtime();
    let d = write_snapshot(&dir, "autosave-400-dddd.cadk", 4);
    bump_mtime();
    let e = write_snapshot(&dir, "autosave-500-eeee.cadk", 5);

    let removed = prune(&dir, 2).expect("prune");
    assert_eq!(removed, 3, "5 files, retain 2 → 3 deletes");

    let entries = list_snapshots(&dir).expect("list");
    assert_eq!(paths(&entries), vec![e.clone(), d.clone()]);
    assert!(!a.exists());
    assert!(!b.exists());
    assert!(!c.exists());
    assert!(d.exists());
    assert!(e.exists());
}

#[test]
fn recover_latest_picks_newest_mtime() {
    let dir = unique_tmp_dir("recover-latest");
    write_snapshot(&dir, "autosave-100-aaaa.cadk", 1);
    bump_mtime();
    write_snapshot(&dir, "autosave-200-bbbb.cadk", 2);
    bump_mtime();
    let newest = write_snapshot(&dir, "autosave-300-cccc.cadk", 3);

    let entry = recover_latest(&dir)
        .expect("recover_latest")
        .expect("at least one snapshot");
    assert_eq!(entry.path, newest);
    assert_eq!(entry.size_bytes, 16);
}

#[test]
fn recover_latest_on_empty_or_missing_returns_none() {
    let empty = unique_tmp_dir("recover-empty");
    assert!(recover_latest(&empty).expect("empty dir").is_none());

    let mut missing = unique_tmp_dir("recover-missing");
    missing.push("does-not-exist");
    assert!(recover_latest(&missing).expect("missing dir").is_none());
}
