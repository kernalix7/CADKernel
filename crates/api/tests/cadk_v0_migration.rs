//! A3.0.1 — `.cadk` v0 schema migration guard.
//!
//! Pins the v0 on-disk format to a committed golden fixture. Future
//! schema bumps must either keep v0 readable (the path we are guarding
//! here — `cadk` decoders must accept every released schema_version) or
//! land a `migrate_v0_to_v1` shim before the v0 reader is allowed to
//! reject this fixture. Either way, a silent format break is impossible
//! without this test going red.
//!
//! The fixture lives at `tests/fixtures/cadk-v0/r1_canonical.cadk` and
//! encodes the canonical R1 command prefix below. If you need to
//! regenerate the fixture (only legitimate reason: codec emitted bytes
//! changed), run:
//!
//! ```ignore
//! cargo test -p cadkernel-api --test cadk_v0_migration regenerate -- --ignored --exact
//! ```
//!
//! and commit the resulting file alongside the change.

use cadkernel_api::{Command, Session, SolidId, cadk};

const FIXTURE_REL: &str = "tests/fixtures/cadk-v0/r1_canonical.cadk";

/// Canonical R1 prefix. Must stay byte-stable for the v0 fixture to be
/// meaningful — adding commands here invalidates the committed fixture.
fn r1_canonical_log() -> Vec<Command> {
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

fn fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_REL)
}

#[test]
fn v0_fixture_decodes_to_canonical_log() {
    let path = fixture_path();
    assert!(
        path.exists(),
        "v0 fixture missing at {}; run `cargo test --test cadk_v0_migration \
         regenerate -- --ignored --exact` and commit the output",
        path.display()
    );
    let bytes = std::fs::read(&path).expect("read fixture");
    let decoded = cadk::decode(&bytes).expect("v0 .cadk must decode");
    assert_eq!(
        decoded,
        r1_canonical_log(),
        "v0 fixture decoded a different command log — schema break or \
         canonical log drift"
    );
}

#[test]
fn v0_fixture_replays_to_single_solid() {
    // Stronger guard than command-equality: the fixture must produce the
    // same document state. If a future patch silently changes a command's
    // semantics (e.g. CreateBox parameter meaning), the byte log still
    // round-trips but the document differs — this test catches that.
    let session = Session::load_cadk_from_path(fixture_path())
        .expect("v0 fixture must load through Session::load_cadk_from_path");
    assert_eq!(
        session.document().solid_count(),
        1,
        "R1 = box minus cylinder = one solid"
    );
    assert_eq!(session.log(), r1_canonical_log().as_slice());
}

#[test]
fn v0_fixture_starts_with_cadk_magic() {
    let bytes = std::fs::read(fixture_path()).expect("read fixture");
    assert!(bytes.len() >= 4, "fixture too small");
    assert_eq!(
        &bytes[..4],
        b"CADK",
        "v0 fixture must begin with 'CADK' magic"
    );
}

/// Regenerate the committed fixture. Ignored by default so an accidental
/// `cargo test` invocation never overwrites the on-disk pin. Use only
/// after a deliberate codec change that preserves v0 semantics.
#[test]
#[ignore]
fn regenerate() {
    let path = fixture_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create fixture dir");
    }
    let bytes = cadk::encode(&r1_canonical_log()).expect("encode v0 fixture");
    std::fs::write(&path, &bytes).expect("write fixture");
    eprintln!(
        "wrote {} bytes to {}; remember to commit this file",
        bytes.len(),
        path.display()
    );
}
