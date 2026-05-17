use cadkernel_api::cadk::{
    self, BlobKind, BlobRecord, Manifest, SchemaVersion, migrate_to_current,
};
use cadkernel_api::{ApiError, Command};
use std::sync::Arc;

const HEADER_SIZE: usize = 64;

fn crc32_ieee(bytes: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, slot) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
        *slot = c;
    }
    let mut c = 0xFFFF_FFFFu32;
    for &b in bytes {
        c = table[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

fn write_header(
    schema_version: u32,
    flags: u32,
    total_size: u64,
    manifest_length: u64,
    manifest_crc32: u32,
) -> [u8; HEADER_SIZE] {
    let mut out = [0u8; HEADER_SIZE];
    out[0..4].copy_from_slice(&schema_version.to_le_bytes());
    out[4..8].copy_from_slice(&flags.to_le_bytes());
    out[8..16].copy_from_slice(&total_size.to_le_bytes());
    out[16..24].copy_from_slice(&((cadk::MAGIC.len() + HEADER_SIZE) as u64).to_le_bytes());
    out[24..32].copy_from_slice(&manifest_length.to_le_bytes());
    out[32..36].copy_from_slice(&manifest_crc32.to_le_bytes());
    out
}

fn manifest_only_container(schema_version: u32, flags: u32, manifest: Manifest) -> Vec<u8> {
    let manifest_bytes = serde_json::to_vec(&manifest).expect("manifest json");
    let total_size = (cadk::MAGIC.len() + HEADER_SIZE + manifest_bytes.len()) as u64;
    let mut out = Vec::with_capacity(total_size as usize);
    out.extend_from_slice(&cadk::MAGIC);
    out.extend_from_slice(&write_header(
        schema_version,
        flags,
        total_size,
        manifest_bytes.len() as u64,
        crc32_ieee(&manifest_bytes),
    ));
    out.extend_from_slice(&manifest_bytes);
    out
}

fn encode_entries(
    schema_version: u32,
    flags: u32,
    entries: Vec<(BlobKind, &str, Vec<u8>)>,
) -> Vec<u8> {
    let manifest_offset = (cadk::MAGIC.len() + HEADER_SIZE) as u64;
    let mut content_offset = manifest_offset;
    let manifest_bytes = loop {
        let mut offset = content_offset;
        let records = entries
            .iter()
            .map(|(kind, name, body)| {
                let record = BlobRecord {
                    kind: *kind,
                    name: (*name).to_string(),
                    offset,
                    length: body.len() as u64,
                    crc32: crc32_ieee(body),
                };
                offset += body.len() as u64;
                record
            })
            .collect();
        let bytes = serde_json::to_vec(&Manifest { records }).expect("manifest json");
        let next_content_offset = manifest_offset + bytes.len() as u64;
        if next_content_offset == content_offset {
            break bytes;
        }
        content_offset = next_content_offset;
    };
    let total_size = content_offset + entries.iter().map(|(_, _, b)| b.len() as u64).sum::<u64>();
    let mut out = Vec::with_capacity(total_size as usize);
    out.extend_from_slice(&cadk::MAGIC);
    out.extend_from_slice(&write_header(
        schema_version,
        flags,
        total_size,
        manifest_bytes.len() as u64,
        crc32_ieee(&manifest_bytes),
    ));
    out.extend_from_slice(&manifest_bytes);
    for (_, _, body) in entries {
        out.extend_from_slice(&body);
    }
    out
}

fn fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/cadk-v0/r1_canonical.cadk")
}

#[test]
fn garbage_byte_input_errors() {
    let err = cadk::inspect(b"not a cadk file").unwrap_err();
    assert!(matches!(err, ApiError::Codec(_)));
}

#[test]
fn zero_length_input_errors() {
    let err = cadk::decode(&[]).unwrap_err();
    assert!(matches!(err, ApiError::Codec(_)));
}

#[test]
fn short_magic_only_input_errors() {
    let err = cadk::inspect(b"CADK").unwrap_err();
    assert!(matches!(err, ApiError::Codec(_)));
}

#[test]
fn v1_fixture_with_unknown_non_must_understand_flag_loads_degraded() {
    let mut bytes = std::fs::read(fixture_path()).expect("fixture");
    let mut flags = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
    flags |= 0x0000_0020;
    bytes[8..12].copy_from_slice(&flags.to_le_bytes());

    let summary = cadk::inspect(&bytes).expect("inspect");
    assert_eq!(summary.schema, SchemaVersion::V1);
    assert_eq!(summary.unknown_flags(), 0x0000_0020);
    assert_eq!(cadk::decode(&bytes).expect("decode").len(), 3);
}

#[test]
fn unknown_must_understand_flag_errors() {
    let mut bytes = cadk::encode(&[Command::Noop]).expect("encode");
    let mut flags = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
    flags |= 0x0001_0000;
    bytes[8..12].copy_from_slice(&flags.to_le_bytes());

    let err = cadk::inspect(&bytes).unwrap_err();
    assert!(matches!(err, ApiError::Codec(_)));
}

#[test]
fn unknown_schema_migration_errors_without_panic() {
    let mut bytes = cadk::encode(&[Command::Noop]).expect("encode");
    bytes[4..8].copy_from_slice(&3u32.to_le_bytes());

    let err = migrate_to_current(&bytes).unwrap_err();
    assert!(matches!(err, ApiError::Codec(_)));
}

#[test]
fn manifest_crc_mismatch_errors() {
    let mut bytes = cadk::encode(&[Command::Noop]).expect("encode");
    bytes[70] ^= 0xFF;

    let err = cadk::inspect(&bytes).unwrap_err();
    assert!(matches!(err, ApiError::Codec(_)));
}

#[test]
fn document_blob_range_overflow_errors_on_decode() {
    let manifest = Manifest {
        records: vec![BlobRecord {
            kind: BlobKind::Document,
            name: "document".into(),
            offset: u64::MAX,
            length: 8,
            crc32: 0,
        }],
    };
    let bytes = manifest_only_container(2, 0, manifest);
    let summary = cadk::inspect(&bytes).expect("inspect");
    assert_eq!(summary.document_length, 8);

    let err = cadk::decode(&bytes).unwrap_err();
    assert!(matches!(err, ApiError::Codec(_)));
}

#[test]
fn future_schema_with_unknown_blob_kind_inspects() {
    let bytes = encode_entries(
        3,
        0,
        vec![(BlobKind::Unknown, "future-blob", vec![1, 2, 3, 4])],
    );
    let summary = cadk::inspect(&bytes).expect("inspect");

    assert_eq!(summary.schema, SchemaVersion::Unknown(3));
    assert_eq!(summary.blobs.len(), 1);
    assert_eq!(summary.blobs[0].kind, BlobKind::Unknown);
}

#[test]
fn future_schema_with_unrecognized_blob_kind_string_inspects() {
    let manifest_offset = (cadk::MAGIC.len() + HEADER_SIZE) as u64;
    let mut content_offset = manifest_offset;
    let manifest_bytes = loop {
        let bytes = serde_json::to_vec(&serde_json::json!({
            "records": [{
                "kind": "mesh_preview",
                "name": "preview",
                "offset": content_offset,
                "length": 0,
                "crc32": 0
            }]
        }))
        .expect("manifest json");
        let next_content_offset = manifest_offset + bytes.len() as u64;
        if next_content_offset == content_offset {
            break bytes;
        }
        content_offset = next_content_offset;
    };
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&cadk::MAGIC);
    bytes.extend_from_slice(&write_header(
        3,
        0,
        content_offset,
        manifest_bytes.len() as u64,
        crc32_ieee(&manifest_bytes),
    ));
    bytes.extend_from_slice(&manifest_bytes);

    let summary = cadk::inspect(&bytes).expect("inspect");
    assert_eq!(summary.schema, SchemaVersion::Unknown(3));
    assert_eq!(summary.blobs[0].kind, BlobKind::Unknown);
    assert_eq!(summary.manifest.records[0].name, "preview");
}

#[test]
fn schema_version_zero_is_rejected() {
    let bytes = manifest_only_container(0, 0, Manifest::default());
    let err = cadk::inspect(&bytes).unwrap_err();
    assert!(matches!(err, ApiError::Codec(_)));
}

#[test]
fn future_schema_document_crc_is_not_read_by_inspect() {
    let mut bytes = encode_entries(
        3,
        0,
        vec![(
            BlobKind::Document,
            "document",
            b"{\"future\":true}".to_vec(),
        )],
    );
    let summary = cadk::inspect(&bytes).expect("inspect");
    let doc = summary
        .manifest
        .find_first(BlobKind::Document)
        .expect("document");
    bytes[doc.offset as usize] ^= 0xFF;

    assert_eq!(
        cadk::inspect(&bytes).expect("inspect").schema,
        SchemaVersion::Unknown(3)
    );
    assert!(cadk::decode(&bytes).is_err());
}

#[test]
fn concurrent_encode_decode_is_race_free() {
    let bytes = Arc::new(cadk::encode(&[Command::CreateSphere { radius: 1.0 }]).expect("encode"));
    let mut threads = Vec::new();
    for _ in 0..8 {
        let bytes = Arc::clone(&bytes);
        threads.push(std::thread::spawn(move || {
            for _ in 0..64 {
                let decoded = cadk::decode(bytes.as_slice()).expect("decode");
                assert_eq!(decoded, vec![Command::CreateSphere { radius: 1.0 }]);
                assert_eq!(
                    cadk::inspect(bytes.as_slice()).expect("inspect").blob_count,
                    1
                );
            }
        }));
    }

    for thread in threads {
        thread.join().expect("thread");
    }
}

#[test]
fn corrupted_thumbnail_crc_errors_without_blocking_document_decode() {
    let thumb = vec![1, 2, 3, 4, 5];
    let mut bytes = cadk::encode_with_thumbnail(&[Command::Noop], Some(&thumb)).expect("encode");
    let summary = cadk::inspect(&bytes).expect("inspect");
    let thumb_record = summary
        .manifest
        .find_first(BlobKind::Thumbnail)
        .expect("thumbnail");
    bytes[thumb_record.offset as usize] ^= 0xEE;

    assert_eq!(cadk::decode(&bytes).expect("decode"), vec![Command::Noop]);
    let err = cadk::decode_thumbnail(&bytes).unwrap_err();
    assert!(matches!(err, ApiError::Codec(_)));
}

#[test]
fn manifest_without_document_is_rejected_for_known_schema() {
    let bytes = encode_entries(
        2,
        0,
        vec![(BlobKind::Attachment, "attachment", vec![1, 2, 3])],
    );
    let err = cadk::inspect(&bytes).unwrap_err();
    assert!(matches!(err, ApiError::Codec(_)));
}
