//! A3.0.1 — filesystem path API roundtrip.
//!
//! Verifies that `Session::save_cadk_to_path` + `load_cadk_from_path` (and
//! the thumbnail-bearing variant) survive a real disk roundtrip. Uses a
//! per-test directory under `std::env::temp_dir()` so concurrent test
//! invocations do not collide.

use cadkernel_api::{ApiError, Command, Session, SolidId, cadk};

fn unique_tmp_path(label: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let pid = std::process::id();
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("cadk-api-{pid}-{nanos}-{n}"));
    std::fs::create_dir_all(&dir).expect("temp dir create");
    dir.join(format!("{label}.cadk"))
}

fn r1_command_log() -> Vec<Command> {
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
fn save_and_load_through_filesystem_preserves_log() {
    let path = unique_tmp_path("roundtrip");
    let log = r1_command_log();
    let session = Session::replay(&log).expect("replay");
    session.save_cadk_to_path(&path).expect("save_cadk_to_path");
    assert!(path.exists(), ".cadk file must exist after save");

    let reloaded = Session::load_cadk_from_path(&path).expect("load_cadk_from_path");
    assert_eq!(
        reloaded.log(),
        log.as_slice(),
        "applied prefix must roundtrip"
    );
    assert_eq!(reloaded.document().solid_count(), 1);
}

#[test]
fn save_with_thumbnail_to_path_round_trips_payload() {
    let path = unique_tmp_path("thumb");
    let log = r1_command_log();
    let session = Session::replay(&log).expect("replay");
    let thumb: Vec<u8> = (0u8..=255).cycle().take(1024).collect();
    session
        .save_cadk_to_path_with_thumbnail(&path, &thumb)
        .expect("save_cadk_to_path_with_thumbnail");

    let bytes = std::fs::read(&path).expect("re-read bytes");
    let recovered = cadk::decode_thumbnail(&bytes)
        .expect("decode_thumbnail")
        .expect("thumbnail blob must be present");
    assert_eq!(recovered, thumb);

    let reloaded = Session::load_cadk_from_path(&path).expect("load_cadk_from_path");
    assert_eq!(reloaded.log(), log.as_slice());
}

#[test]
fn load_from_missing_path_reports_codec_error() {
    let missing = std::env::temp_dir().join("cadk-definitely-does-not-exist-xyz-9999.cadk");
    match Session::load_cadk_from_path(&missing) {
        Ok(_) => panic!("loading a missing path must fail"),
        Err(ApiError::Codec(msg)) => assert!(
            msg.starts_with("file io:"),
            "missing-file errors must be prefixed `file io:`, got: {msg}"
        ),
        Err(other) => panic!("expected ApiError::Codec, got {other:?}"),
    }
}

#[test]
fn save_to_unwritable_path_reports_codec_error() {
    // A path whose parent directory does not exist forces fs::write to fail.
    let bad = std::env::temp_dir()
        .join("cadk-no-such-parent-xyz-9999")
        .join("nested")
        .join("out.cadk");
    let session = Session::new();
    match session.save_cadk_to_path(&bad) {
        Ok(_) => panic!("saving to an unwritable path must fail"),
        Err(ApiError::Codec(msg)) => assert!(
            msg.starts_with("file io:"),
            "io errors must be prefixed `file io:`, got: {msg}"
        ),
        Err(other) => panic!("expected ApiError::Codec, got {other:?}"),
    }
}
