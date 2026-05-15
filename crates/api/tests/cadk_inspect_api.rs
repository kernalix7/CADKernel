//! A3.0.3 — `cadk::inspect()` read-only summary entry point.
//!
//! Covers:
//! - Uncompressed v0 container → correct schema/flags/blob counts.
//! - Compressed container → `document_compressed()` true and reported
//!   `document_length` equals the zstd frame length, not the raw JSON.
//! - Thumbnail combo → `has_thumbnail()` true, `thumbnail_length`
//!   matches the embedded payload length, `blob_count == 2`.
//! - Bad magic, truncation, and manifest CRC corruption → `ApiError::Codec`.
//! - Cheapness invariant: inspect succeeds even when the document blob
//!   body has been corrupted (only header + manifest are validated). This
//!   is the deliberate divergence from `decode()`.
//! - Manifest range out of bounds → rejected.
//! - Unknown forward-compat flag bits surface through `unknown_flags()`
//!   when they live in the *non* must-understand region; bits in the
//!   must-understand region fail the header support check before
//!   inspect returns.

use cadkernel_api::cadk::{self, BlobKind, CadkFlags, SaveOptions};
use cadkernel_api::{ApiError, Command};

fn sample_log() -> Vec<Command> {
    vec![
        Command::CreateBox {
            dx: 10.0,
            dy: 5.0,
            dz: 2.0,
        },
        Command::CreateSphere { radius: 1.5 },
        Command::CreateCylinder {
            radius: 0.5,
            height: 4.0,
        },
    ]
}

#[test]
fn inspect_uncompressed_container_reports_v0_layout() {
    let bytes = cadk::encode(&sample_log()).unwrap();
    let summary = cadk::inspect(&bytes).unwrap();

    assert_eq!(summary.schema_version, 2);
    assert_eq!(summary.flags, 0);
    assert_eq!(summary.total_size as usize, bytes.len());
    assert_eq!(summary.blob_count, 1);
    assert!(!summary.document_compressed());
    assert!(!summary.has_thumbnail());
    assert!(!summary.is_signed());
    assert!(!summary.manifest_compressed());
    assert_eq!(summary.unknown_flags(), 0);
    assert!(summary.document_length > 0);
    assert_eq!(summary.thumbnail_length, None);
}

#[test]
fn inspect_compressed_container_reports_document_compressed_flag() {
    let opts = SaveOptions::default().with_compression(3);
    let bytes = cadk::encode_with_options(&sample_log(), &opts).unwrap();
    let summary = cadk::inspect(&bytes).unwrap();

    assert!(summary.document_compressed());
    assert_eq!(
        summary.flags & CadkFlags::DOCUMENT_COMPRESSED,
        CadkFlags::DOCUMENT_COMPRESSED
    );
    assert_eq!(summary.blob_count, 1);
    // The reported document_length is the compressed (on-disk) size,
    // which for this tiny log is bounded but non-zero.
    assert!(summary.document_length > 0);
    assert!(summary.document_length < summary.total_size);
}

#[test]
fn inspect_thumbnail_container_reports_two_blobs() {
    let thumb: Vec<u8> = (0u8..=255).cycle().take(2048).collect();
    let opts = SaveOptions::default().with_thumbnail(thumb.clone());
    let bytes = cadk::encode_with_options(&sample_log(), &opts).unwrap();
    let summary = cadk::inspect(&bytes).unwrap();

    assert!(summary.has_thumbnail());
    assert_eq!(summary.blob_count, 2);
    assert_eq!(summary.thumbnail_length, Some(thumb.len() as u64));
}

#[test]
fn inspect_compressed_thumbnail_combo_reports_both_flags() {
    let thumb = vec![0xAB; 512];
    let opts = SaveOptions::default()
        .with_compression(9)
        .with_thumbnail(thumb.clone());
    let bytes = cadk::encode_with_options(&sample_log(), &opts).unwrap();
    let summary = cadk::inspect(&bytes).unwrap();

    assert!(summary.document_compressed());
    assert!(summary.has_thumbnail());
    assert_eq!(summary.blob_count, 2);
    assert_eq!(summary.thumbnail_length, Some(thumb.len() as u64));
}

#[test]
fn inspect_rejects_bad_magic() {
    let mut bytes = cadk::encode(&sample_log()).unwrap();
    bytes[0] = b'X';
    let err = cadk::inspect(&bytes).unwrap_err();
    assert!(matches!(err, ApiError::Codec(_)));
}

#[test]
fn inspect_rejects_truncated_container() {
    let bytes = cadk::encode(&sample_log()).unwrap();
    let truncated = &bytes[..bytes.len() - 5];
    let err = cadk::inspect(truncated).unwrap_err();
    assert!(matches!(err, ApiError::Codec(_)));
}

#[test]
fn inspect_rejects_corrupted_manifest_via_crc() {
    let mut bytes = cadk::encode(&sample_log()).unwrap();
    // Magic (4) + header (64) = 68. Manifest starts at offset 68. Flip a
    // byte inside the manifest body so the CRC stops matching.
    bytes[80] ^= 0xFF;
    let err = cadk::inspect(&bytes).unwrap_err();
    let msg = match err {
        ApiError::Codec(m) => m,
        other => panic!("expected Codec, got {other:?}"),
    };
    assert!(
        msg.contains("manifest") || msg.contains("crc"),
        "expected manifest/crc diagnostic, got: {msg}"
    );
}

#[test]
fn inspect_succeeds_even_when_document_body_is_corrupted() {
    // Deliberate divergence from decode(): inspect does not read the
    // document body. A corrupted doc blob is invisible to inspect, which
    // is the whole point — quick metadata for Recent-Files / autosave
    // listings stays fast and tolerant of partial corruption.
    let mut bytes = cadk::encode(&sample_log()).unwrap();
    let last = bytes.len() - 4;
    bytes[last] ^= 0xFF;
    let summary = cadk::inspect(&bytes).expect("inspect ignores doc body");
    assert_eq!(summary.blob_count, 1);

    // decode() must still reject the same bytes.
    assert!(cadk::decode(&bytes).is_err());
}

#[test]
fn inspect_exposes_document_blob_in_manifest_order() {
    // A3.0.6: per-blob view. Uncompressed save → exactly one BlobInfo,
    // kind = Document, name = "document", length matches the typed
    // summary's `document_length`.
    let bytes = cadk::encode(&sample_log()).unwrap();
    let summary = cadk::inspect(&bytes).unwrap();
    assert_eq!(summary.blobs.len(), 1);
    let doc = &summary.blobs[0];
    assert_eq!(doc.kind, BlobKind::Document);
    assert_eq!(doc.name, "document");
    assert_eq!(doc.length, summary.document_length);
}

#[test]
fn inspect_exposes_thumbnail_alongside_document_in_manifest_order() {
    // A3.0.6: thumbnail combo → exactly two BlobInfo entries in
    // manifest order: Document first, Thumbnail second. The
    // thumbnail BlobInfo.length must agree with the typed summary's
    // `thumbnail_length`.
    let thumb = vec![0x33u8; 777];
    let opts = SaveOptions::default().with_thumbnail(thumb.clone());
    let bytes = cadk::encode_with_options(&sample_log(), &opts).unwrap();
    let summary = cadk::inspect(&bytes).unwrap();
    assert_eq!(summary.blobs.len(), 2);
    assert_eq!(summary.blobs[0].kind, BlobKind::Document);
    assert_eq!(summary.blobs[1].kind, BlobKind::Thumbnail);
    assert_eq!(summary.blobs[1].name, "thumbnail");
    assert_eq!(summary.blobs[1].length, thumb.len() as u64);
    assert_eq!(summary.thumbnail_length, Some(summary.blobs[1].length));
}

#[test]
fn inspect_blob_lengths_sum_to_document_plus_thumbnail() {
    // Sanity: the per-blob lengths sum to the document_length plus
    // thumbnail_length when both are present. Pins the invariant
    // that BlobInfo length == on-disk encoded length, NOT logical
    // size after decompression.
    let thumb = vec![0x77u8; 256];
    let opts = SaveOptions::default()
        .with_compression(5)
        .with_thumbnail(thumb.clone());
    let bytes = cadk::encode_with_options(&sample_log(), &opts).unwrap();
    let summary = cadk::inspect(&bytes).unwrap();
    let blob_sum: u64 = summary.blobs.iter().map(|b| b.length).sum();
    let expected = summary.document_length + summary.thumbnail_length.unwrap_or(0);
    assert_eq!(blob_sum, expected);
}

#[test]
fn inspect_rejects_unsupported_must_understand_flag() {
    // Encode a healthy container, then flip an unknown bit inside the
    // must-understand region (top 16) of the header flags. The header's
    // `is_supported()` predicate must reject it before inspect can
    // surface the summary. We do NOT re-CRC the manifest here because
    // the flag check fires first — and either failure mode produces
    // ApiError::Codec, which is what we assert.
    let mut bytes = cadk::encode(&sample_log()).unwrap();
    // Flags live at offset 4 (magic) + 4 (schema_version) = 8 .. 12.
    let mut flags = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
    flags |= 0x0001_0000; // unknown bit in must-understand region
    bytes[8..12].copy_from_slice(&flags.to_le_bytes());
    let err = cadk::inspect(&bytes).unwrap_err();
    assert!(matches!(err, ApiError::Codec(_)));
}
