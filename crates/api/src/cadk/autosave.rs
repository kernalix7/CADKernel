//! Rotated autosave snapshots for `.cadk` documents.
//!
//! A3.1 (2026-05-13). Adds three concerns on top of the existing
//! [`crate::cadk`] codec:
//!
//! 1. A policy type ([`AutosavePolicy`]) that captures where snapshots
//!    live on disk, how frequently to write them, how many to retain,
//!    and whether to zstd-compress the document blob.
//! 2. A directory enumerator ([`list_snapshots`]) that returns
//!    `autosave-*.cadk` files sorted newest-first so the viewer's
//!    recovery modal can present the most recent snapshot prominently.
//! 3. A rotation primitive ([`prune`]) that deletes everything older
//!    than the `retain` window, plus a convenience [`recover_latest`]
//!    for the single-shot "open the most recent autosave" flow.
//!
//! The actual snapshot *writer* lives on [`crate::Session`]
//! (`write_autosave_snapshot`) because writing needs an in-memory
//! document — see task #3 in the A3.1 plan.
//!
//! Snapshot filenames follow the pattern
//! `autosave-{epoch_ms}-{hash16}.cadk`, where `hash16` is the leading
//! 16 hex chars of [`crate::Document::canonical_hash`]. This keeps the
//! filename self-describing (timestamp + content fingerprint) and
//! tolerates clock skew without losing ordering, since we sort by the
//! filesystem's modification time rather than the embedded timestamp.

use crate::{ApiError, ApiResult};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Where and how an autosave loop writes rotated `.cadk` snapshots.
///
/// Created and owned by the viewer; passed by reference into
/// [`crate::Session::write_autosave_snapshot`]. The codec module only
/// reads the policy — it never mutates it and never persists it to
/// disk (the policy is a runtime configuration, not part of the
/// document itself).
#[derive(Debug, Clone)]
pub struct AutosavePolicy {
    /// Directory holding rotated snapshots. Created on demand by
    /// [`crate::Session::write_autosave_snapshot`] (see task #3).
    pub dir: PathBuf,
    /// Minimum wall-clock interval between successive autosaves.
    /// Enforced by the viewer's tick loop, **not** by the writer —
    /// `write_autosave_snapshot` writes unconditionally.
    pub interval: Duration,
    /// Maximum number of snapshots to keep in `dir`. Older snapshots
    /// are deleted by [`prune`] after every write.
    pub retain: usize,
    /// When `true`, the writer enables zstd compression of the
    /// document blob (level 3) via [`crate::cadk::SaveOptions::with_compression`].
    pub compress: bool,
}

impl AutosavePolicy {
    /// Builder-style helper used by tests; viewer code constructs
    /// the struct directly through field init.
    pub fn new(dir: impl Into<PathBuf>, interval: Duration, retain: usize) -> Self {
        Self {
            dir: dir.into(),
            interval,
            retain,
            compress: false,
        }
    }

    /// Builder helper: enable zstd compression for the document blob.
    #[must_use]
    pub fn with_compression(mut self, enabled: bool) -> Self {
        self.compress = enabled;
        self
    }
}

/// One row from [`list_snapshots`]. Mirrors the small subset of
/// `std::fs::Metadata` that callers actually need — full path,
/// modification time, and encoded size — without forcing every
/// consumer to depend on `std::fs::Metadata` directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutosaveEntry {
    /// Absolute path to the `.cadk` snapshot file.
    pub path: PathBuf,
    /// Filesystem modification time. Used as the sort key by
    /// [`list_snapshots`] (newest-first).
    pub modified: SystemTime,
    /// File size in bytes (matches `Metadata::len`).
    pub size_bytes: u64,
}

/// List the autosave snapshots in `dir`, newest-first by filesystem
/// modification time.
///
/// A snapshot is recognised when:
/// - the entry is a regular file (not a directory or symlink loop), and
/// - the filename starts with `autosave-`, and
/// - the filename ends with `.cadk` (extension match).
///
/// A missing `dir` returns `Ok(Vec::new())` — this is the expected
/// state on first launch, before any autosave has been written, and
/// is **not** an error. Permission failures and other I/O errors are
/// surfaced as [`ApiError::Codec`] with the `file io:` prefix that
/// the rest of the `cadk` module uses.
pub fn list_snapshots(dir: &Path) -> ApiResult<Vec<AutosaveEntry>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let read = std::fs::read_dir(dir).map_err(|err| ApiError::Codec(format!("file io: {err}")))?;
    let mut out: Vec<AutosaveEntry> = Vec::new();
    for entry in read {
        let entry = entry.map_err(|err| ApiError::Codec(format!("file io: {err}")))?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.starts_with("autosave-") {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("cadk") {
            continue;
        }
        let meta = entry
            .metadata()
            .map_err(|err| ApiError::Codec(format!("file io: {err}")))?;
        if !meta.is_file() {
            continue;
        }
        let modified = meta
            .modified()
            .map_err(|err| ApiError::Codec(format!("file io: {err}")))?;
        out.push(AutosaveEntry {
            path,
            modified,
            size_bytes: meta.len(),
        });
    }
    out.sort_by_key(|e| std::cmp::Reverse(e.modified));
    Ok(out)
}

/// Trim the snapshot directory down to at most `retain` entries.
///
/// Listing happens via [`list_snapshots`], so the same filename rules
/// apply. The newest `retain` entries are kept; all older entries are
/// deleted. Returns the number of files removed.
///
/// `retain == 0` deletes every recognised snapshot — useful for tests
/// and for the "disable autosave and clear history" preference toggle.
///
/// Errors from individual deletes are surfaced as
/// [`ApiError::Codec`] with the `file io:` prefix; a partial run that
/// hits a permission failure stops at the first error and reports it.
pub fn prune(dir: &Path, retain: usize) -> ApiResult<usize> {
    let entries = list_snapshots(dir)?;
    if entries.len() <= retain {
        return Ok(0);
    }
    let mut removed = 0usize;
    for entry in entries.into_iter().skip(retain) {
        std::fs::remove_file(&entry.path)
            .map_err(|err| ApiError::Codec(format!("file io: {err}")))?;
        removed += 1;
    }
    Ok(removed)
}

/// Return the most recent autosave entry in `dir`, if any.
///
/// Equivalent to `list_snapshots(dir)?.into_iter().next()` — exposed
/// as a named function because the recovery modal's "open last
/// autosave" path is the single hottest caller and the explicit
/// `recover_latest` name reads better at the call site.
///
/// Missing dir → `Ok(None)` (no autosave to recover).
pub fn recover_latest(dir: &Path) -> ApiResult<Option<AutosaveEntry>> {
    let mut entries = list_snapshots(dir)?;
    if entries.is_empty() {
        Ok(None)
    } else {
        Ok(Some(entries.swap_remove(0)))
    }
}
