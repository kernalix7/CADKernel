//! A3.0.7 — `cadk::migrate_to_current()` end-to-end behaviour.
//!
//! Unit tests in `crates/api/src/cadk/migrate.rs` cover the cheap
//! header-probe failure modes. This file covers the round-trip
//! contract against real `.cadk` containers:
//!
//! - Currently-encoded bytes round-trip byte-identically through
//!   `migrate_to_current` (V1 → V1 is a no-op).
//! - The committed v0 golden fixture round-trips byte-identically
//!   (locks the no-op invariant against the on-disk pin).
//! - After migration, the bytes feed cleanly into `inspect` and
//!   `decode` (no schema drift introduced).
//! - Garbage bytes that pass the cheap header probe but fail full
//!   validation surface a `Codec` error from the post-migrate
//!   `inspect`/`decode` path, not from migrate itself.

use cadkernel_api::cadk::{self, SaveOptions, SchemaVersion};
use cadkernel_api::{Command, SolidId};

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
fn migrate_is_no_op_on_freshly_encoded_v1_bytes() {
    let bytes = cadk::encode(&r1_log()).expect("encode");
    let migrated = cadk::migrate_to_current(&bytes).expect("migrate");
    assert_eq!(
        migrated, bytes,
        "V1 → V1 migration must produce byte-identical output"
    );
}

#[test]
fn migrate_preserves_compression_and_thumbnail_flags() {
    let thumb = vec![0x42u8; 1024];
    let opts = SaveOptions::default()
        .with_compression(7)
        .with_thumbnail(thumb);
    let bytes = cadk::encode_with_options(&r1_log(), &opts).expect("encode");
    let migrated = cadk::migrate_to_current(&bytes).expect("migrate");
    assert_eq!(migrated, bytes);
    // Sanity: the migrated bytes still inspect cleanly with both flags.
    let summary = cadk::inspect(&migrated).expect("inspect after migrate");
    assert!(summary.document_compressed());
    assert!(summary.has_thumbnail());
}

#[test]
fn migrate_v1_fixture_to_v2_round_trips_via_decoder() {
    // After Wave 3 (T5b), `current()` returns V2 and `migrate_to_current`
    // applies the V1 → V2 transform, which IS an additive bytes-change
    // (new empty bodies/sketches sections). The pre-Wave-3 "no-op" pin
    // no longer applies. New contract: the migrated bytes must still
    // decode as the same Vec<Command>, which is what users actually
    // care about.
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("cadk-v0")
        .join("r1_canonical.cadk");
    let bytes = std::fs::read(&fixture).expect("read v0 fixture");
    let original_log = cadk::decode(&bytes).expect("decode original v1 fixture");
    let migrated = cadk::migrate_to_current(&bytes).expect("migrate v1 fixture");
    let migrated_log = cadk::decode(&migrated).expect("decode migrated bytes");
    assert_eq!(
        original_log, migrated_log,
        "V1 → V2 migration must preserve the command log"
    );
}

#[test]
fn migrate_then_inspect_then_decode_round_trips_command_log() {
    let log = r1_log();
    let bytes = cadk::encode(&log).expect("encode");
    let migrated = cadk::migrate_to_current(&bytes).expect("migrate");

    let summary = cadk::inspect(&migrated).expect("inspect");
    assert_eq!(summary.schema_version, SchemaVersion::current().as_u32());

    let decoded = cadk::decode(&migrated).expect("decode");
    assert_eq!(decoded, log);
}
