//! A3.0.5 — `cadk-inspect` binary smoke tests.
//!
//! These exercise the rebuilt CLI end-to-end (process spawn, exit codes,
//! stdout/stderr contracts). The binary's structured summary now flows
//! through `cadk::inspect` / `cadk::CadkSummary`; the default path still
//! validates the document blob CRC via `cadk::decode`; `--quick` opts
//! into the cheap inspect-only path.
//!
//! Each test uses `env!("CARGO_BIN_EXE_cadk-inspect")` so cargo points
//! at the freshly-built binary for this run — no PATH lookup, no
//! pre-built dependency on external tooling.

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_cadk-inspect");

fn fixture_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("cadk-v0")
        .join("r1_canonical.cadk")
}

#[test]
fn default_run_against_v0_fixture_exits_zero_and_reports_all_crcs() {
    let output = Command::new(BIN)
        .arg(fixture_path())
        .output()
        .expect("spawn cadk-inspect");
    assert!(
        output.status.success(),
        "exit code: {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("magic:"),
        "stdout missing magic line:\n{stdout}"
    );
    assert!(
        stdout.contains("schema:         1"),
        "stdout missing schema line:\n{stdout}"
    );
    assert!(
        stdout.contains("blobs:          1"),
        "stdout missing blob count line:\n{stdout}"
    );
    assert!(
        stdout.contains("thumbnail:      absent"),
        "stdout missing thumbnail line:\n{stdout}"
    );
    assert!(
        stdout.contains("status:         OK (all CRCs verified)"),
        "default run must claim full CRC verification:\n{stdout}"
    );
}

#[test]
fn quick_run_skips_decode_and_reports_quick_status() {
    let output = Command::new(BIN)
        .arg("--quick")
        .arg(fixture_path())
        .output()
        .expect("spawn cadk-inspect");
    assert!(
        output.status.success(),
        "quick mode must succeed on a healthy fixture"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("status:         OK (header + manifest verified, --quick)"),
        "quick run must downgrade the status line:\n{stdout}"
    );
    // The full-path "commands: N" line lives in default/verbose modes.
    assert!(
        !stdout.contains("commands:"),
        "quick mode must not run decode and therefore must not print a command count:\n{stdout}"
    );
}

#[test]
fn verbose_run_prints_command_listing() {
    let output = Command::new(BIN)
        .arg("-v")
        .arg(fixture_path())
        .output()
        .expect("spawn cadk-inspect");
    assert!(output.status.success(), "verbose mode must succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--- commands ---"),
        "verbose must print commands section"
    );
    assert!(
        stdout.contains("CreateBox"),
        "v0 fixture starts with CreateBox: {stdout}"
    );
}

#[test]
fn verbose_run_prints_blob_listing_with_kinds_and_lengths() {
    // A3.0.6: verbose dumps the per-blob view ahead of the command
    // listing. For the v0 fixture (single Document blob, no thumbnail)
    // the section must enumerate exactly one entry with kind=Document
    // and name="document".
    let output = Command::new(BIN)
        .arg("-v")
        .arg(fixture_path())
        .output()
        .expect("spawn cadk-inspect");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--- blobs ---"),
        "verbose must print blobs section"
    );
    assert!(
        stdout.contains("Document"),
        "blob listing must surface kind=Document: {stdout}"
    );
    assert!(
        stdout.contains("name=\"document\""),
        "blob listing must surface name: {stdout}"
    );
}

#[test]
fn missing_path_exits_with_io_error_code_one() {
    let missing = std::env::temp_dir().join("cadk-inspect-missing-xyz-9999.cadk");
    let output = Command::new(BIN)
        .arg(&missing)
        .output()
        .expect("spawn cadk-inspect");
    assert_eq!(
        output.status.code(),
        Some(1),
        "missing-file must yield exit 1"
    );
}

#[test]
fn corrupted_bytes_exit_with_format_error_code_two() {
    // Write garbage (with a bad magic) to a temp file so the read
    // succeeds but the codec rejects the bytes.
    let dir = std::env::temp_dir().join(format!(
        "cadk-inspect-bin-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&dir).expect("mkdir tmp");
    let path = dir.join("garbage.cadk");
    std::fs::write(&path, vec![0u8; 128]).expect("write garbage");

    let output = Command::new(BIN)
        .arg(&path)
        .output()
        .expect("spawn cadk-inspect");
    assert_eq!(
        output.status.code(),
        Some(2),
        "format errors must yield exit 2, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn unknown_flag_exits_one() {
    let output = Command::new(BIN)
        .arg("--no-such-flag")
        .output()
        .expect("spawn cadk-inspect");
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn quick_combined_with_verbose_warns_but_succeeds() {
    let output = Command::new(BIN)
        .arg("--quick")
        .arg("-v")
        .arg(fixture_path())
        .output()
        .expect("spawn cadk-inspect");
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--verbose is a no-op when combined with --quick"),
        "expected the conflict note on stderr, got: {stderr}"
    );
}
