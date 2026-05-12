//! A3.0.4 — `cadk::inspect_path()` filesystem wrapper.
//!
//! Covers:
//! - Path roundtrip on an uncompressed v0 container (summary fields match
//!   the byte-buffer `inspect()` result).
//! - Compression + thumbnail combo through disk.
//! - Missing path surfaces `ApiError::Codec("file io: ...")` (same
//!   contract as `Session::load_cadk_from_path`).
//! - Corrupt-on-disk file surfaces a Codec diagnostic without a `file io:`
//!   prefix (because the read succeeded — only the parse fails).
//! - Committed v0 golden fixture loads via `inspect_path`: blob_count == 1,
//!   uncompressed, no thumbnail. Pins the fixture against silent codec
//!   regressions that would survive `cadk_v0_migration.rs` but break
//!   metadata-only readers.

use cadkernel_api::cadk::{self, SaveOptions};
use cadkernel_api::{ApiError, Command, Session, SolidId};

fn unique_tmp_path(label: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let pid = std::process::id();
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("cadk-inspect-path-{pid}-{nanos}-{n}"));
    std::fs::create_dir_all(&dir).expect("temp dir create");
    dir.join(format!("{label}.cadk"))
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
fn inspect_path_matches_inspect_bytes_for_uncompressed_save() {
    let path = unique_tmp_path("uncompressed");
    let session = Session::replay(&r1_log()).expect("replay");
    session.save_cadk_to_path(&path).expect("save_cadk_to_path");

    let bytes = std::fs::read(&path).expect("re-read bytes");
    let summary_bytes = cadk::inspect(&bytes).expect("inspect bytes");
    let summary_path = cadk::inspect_path(&path).expect("inspect_path");

    assert_eq!(summary_bytes, summary_path);
    assert_eq!(summary_path.blob_count, 1);
    assert!(!summary_path.document_compressed());
    assert!(!summary_path.has_thumbnail());
    assert_eq!(summary_path.total_size as usize, bytes.len());
}

#[test]
fn inspect_path_reports_compression_and_thumbnail_combo() {
    let path = unique_tmp_path("combo");
    let session = Session::replay(&r1_log()).expect("replay");
    let thumb = vec![0xCDu8; 1024];
    let opts = SaveOptions::default()
        .with_compression(9)
        .with_thumbnail(thumb.clone());
    session
        .save_cadk_to_path_with_options(&path, &opts)
        .expect("save_cadk_to_path_with_options");

    let summary = cadk::inspect_path(&path).expect("inspect_path");
    assert!(summary.document_compressed());
    assert!(summary.has_thumbnail());
    assert_eq!(summary.blob_count, 2);
    assert_eq!(summary.thumbnail_length, Some(thumb.len() as u64));
}

#[test]
fn inspect_path_missing_file_reports_file_io_prefix() {
    let missing = std::env::temp_dir().join("cadk-inspect-missing-xyz-9999.cadk");
    match cadk::inspect_path(&missing) {
        Ok(_) => panic!("inspect_path on missing file must fail"),
        Err(ApiError::Codec(msg)) => assert!(
            msg.starts_with("file io:"),
            "missing-file errors must be prefixed `file io:`, got: {msg}"
        ),
        Err(other) => panic!("expected ApiError::Codec, got {other:?}"),
    }
}

#[test]
fn inspect_path_corrupted_file_reports_codec_without_file_io_prefix() {
    let path = unique_tmp_path("corrupt");
    // Write enough bytes to pass the size check (>= magic + header = 68)
    // but with a bad magic so the codec rejects it. The read succeeds,
    // so the diagnostic must NOT carry the `file io:` prefix.
    std::fs::write(&path, vec![0u8; 128]).expect("write corrupt file");
    match cadk::inspect_path(&path) {
        Ok(_) => panic!("inspect_path on garbage bytes must fail"),
        Err(ApiError::Codec(msg)) => assert!(
            !msg.starts_with("file io:"),
            "parse errors must NOT carry `file io:` prefix, got: {msg}"
        ),
        Err(other) => panic!("expected ApiError::Codec, got {other:?}"),
    }
}

#[test]
fn inspect_path_loads_committed_v0_golden_fixture() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("cadk-v0")
        .join("r1_canonical.cadk");
    assert!(
        fixture.exists(),
        "v0 golden fixture missing: {}",
        fixture.display()
    );
    let summary = cadk::inspect_path(&fixture).expect("inspect_path on v0 fixture");
    assert_eq!(summary.schema_version, 1);
    assert_eq!(summary.blob_count, 1);
    assert!(!summary.document_compressed());
    assert!(!summary.has_thumbnail());
    assert_eq!(summary.unknown_flags(), 0);
}
