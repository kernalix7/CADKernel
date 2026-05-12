//! A3.0.2 — zstd compression roundtrip and backward-compat.
//!
//! Covers:
//! - Compressed encode → decode equality (level 3 and level 22).
//! - Header flag bookkeeping: `DOCUMENT_COMPRESSED` set iff compression
//!   was requested.
//! - Size reduction: a deliberately redundant log compresses strictly
//!   smaller than its uncompressed counterpart at level 22.
//! - Backward compat: the committed v0 golden fixture is uncompressed
//!   and continues to decode through the unchanged `Session::load_cadk*`
//!   path (covered by `cadk_v0_migration.rs` indirectly; here we assert
//!   explicitly that mixing the new SaveOptions {default} with the
//!   existing fixture is a no-op).
//! - Compressed-with-thumbnail combo.

use cadkernel_api::cadk::{self, CadkFlags, SaveOptions};
use cadkernel_api::{Command, Session};

fn redundant_log(reps: usize) -> Vec<Command> {
    // A repeating pattern compresses well; level 22 must shrink it
    // measurably vs the JSON baseline. Uses only primitives so every
    // command is unconditionally valid in replay — boolean ops would
    // consume their inputs and break iteration-2 onward.
    let mut out = Vec::with_capacity(reps * 2);
    for _ in 0..reps {
        out.push(Command::CreateBox {
            dx: 1.0,
            dy: 2.0,
            dz: 3.0,
        });
        out.push(Command::CreateCylinder {
            radius: 1.0,
            height: 4.0,
        });
    }
    out
}

fn header_flags(bytes: &[u8]) -> u32 {
    // Magic is 4 bytes; flags live at offset 4 inside the header
    // (schema_version takes the first 4 bytes of the header).
    let off = 4 + 4;
    u32::from_le_bytes(bytes[off..off + 4].try_into().expect("4-byte flags slice"))
}

#[test]
fn compressed_round_trip_preserves_log_at_level_3() {
    let log = redundant_log(20);
    let session = Session::replay(&log).expect("replay");
    let opts = SaveOptions::default().with_compression(3);
    let bytes = session.save_cadk_with_options(&opts).expect("encode");

    assert_ne!(
        header_flags(&bytes) & CadkFlags::DOCUMENT_COMPRESSED,
        0,
        "level 3 must set DOCUMENT_COMPRESSED"
    );
    let reloaded = Session::load_cadk(&bytes).expect("decode");
    assert_eq!(reloaded.log(), log.as_slice());
}

#[test]
fn compressed_round_trip_preserves_log_at_level_22() {
    let log = redundant_log(50);
    let session = Session::replay(&log).expect("replay");
    let opts = SaveOptions::default().with_compression(22);
    let bytes = session.save_cadk_with_options(&opts).expect("encode");
    let reloaded = Session::load_cadk(&bytes).expect("decode");
    assert_eq!(reloaded.log(), log.as_slice());
}

#[test]
fn default_options_match_legacy_uncompressed_encoding() {
    // Default SaveOptions { compression_level: None, thumbnail: None }
    // must produce byte-identical output to the legacy encode() path so
    // the v0 fixture and all existing decoders stay valid.
    let log = redundant_log(10);
    let session = Session::replay(&log).expect("replay");
    let legacy = session.save_cadk().expect("legacy encode");
    let via_opts = session
        .save_cadk_with_options(&SaveOptions::default())
        .expect("options encode");
    assert_eq!(legacy, via_opts);
    assert_eq!(
        header_flags(&legacy) & CadkFlags::DOCUMENT_COMPRESSED,
        0,
        "uncompressed default must leave DOCUMENT_COMPRESSED clear"
    );
}

#[test]
fn level_22_strictly_shrinks_redundant_log() {
    let log = redundant_log(50);
    let session = Session::replay(&log).expect("replay");
    let plain = session.save_cadk().expect("plain encode");
    let opts = SaveOptions::default().with_compression(22);
    let squeezed = session.save_cadk_with_options(&opts).expect("zstd encode");
    assert!(
        squeezed.len() < plain.len(),
        "compressed must be strictly smaller for redundant input \
         (plain={}, zstd22={})",
        plain.len(),
        squeezed.len()
    );
}

#[test]
fn compressed_round_trips_with_embedded_thumbnail() {
    let log = redundant_log(10);
    let session = Session::replay(&log).expect("replay");
    let thumb: Vec<u8> = (0u8..=255).cycle().take(2048).collect();
    let opts = SaveOptions::default()
        .with_compression(9)
        .with_thumbnail(thumb.clone());
    let bytes = session.save_cadk_with_options(&opts).expect("encode");

    let flags = header_flags(&bytes);
    assert_ne!(flags & CadkFlags::DOCUMENT_COMPRESSED, 0);
    assert_ne!(flags & CadkFlags::HAS_THUMBNAIL, 0);

    let reloaded = Session::load_cadk(&bytes).expect("decode");
    assert_eq!(reloaded.log(), log.as_slice());
    let recovered = cadk::decode_thumbnail(&bytes)
        .expect("decode_thumbnail")
        .expect("thumbnail present");
    assert_eq!(recovered, thumb);
}

#[test]
fn compressed_save_to_path_round_trips_through_filesystem() {
    let path = std::env::temp_dir().join(format!(
        "cadk-compressed-{}-{}.cadk",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let log = redundant_log(15);
    let session = Session::replay(&log).expect("replay");
    let opts = SaveOptions::default().with_compression(11);
    session
        .save_cadk_to_path_with_options(&path, &opts)
        .expect("save_to_path");
    let reloaded = Session::load_cadk_from_path(&path).expect("load_from_path");
    assert_eq!(reloaded.log(), log.as_slice());
    let _ = std::fs::remove_file(&path);
}
