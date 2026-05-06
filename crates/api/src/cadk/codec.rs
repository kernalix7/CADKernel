//! `.cadk` codec — encode / decode the binary container.
//!
//! A3 v0 implementation: hand-rolled little-endian header (no `bincode`
//! workspace dep), `serde_json` blob body for the document log (will be
//! upgraded to `bincode 2` in A3.1), CRC-32-IEEE-802.3 integrity per blob.
//!
//! Layout:
//!
//! ```text
//! +------+----------------+------------------+----------------+
//! | "CADK" (4)            | header (64)      | manifest (var) |
//! +-----------------------+------------------+----------------+
//! | content blobs (var, in manifest order)                    |
//! +-----------------------------------------------------------+
//! ```
//!
//! No compression / signing yet — those will be additive (gated by
//! `CadkFlags::MANIFEST_COMPRESSED` / `SIGNED`).

use crate::cadk::header::{CadkFlags, CadkHeader, HEADER_SIZE, MAGIC, SCHEMA_VERSION};
use crate::cadk::manifest::{BlobKind, BlobRecord, Manifest};
use crate::command::Command;
use crate::{ApiError, ApiResult};

/// CRC-32 with the standard IEEE 802.3 reverse polynomial (0xEDB88320).
/// Lazy-initialised table; matches the output of `crc32fast` and most
/// Unix `cksum` implementations of the IEEE variant.
fn crc32_ieee(bytes: &[u8]) -> u32 {
    use std::sync::OnceLock;
    static TABLE: OnceLock<[u32; 256]> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut t = [0u32; 256];
        for (i, slot) in t.iter_mut().enumerate() {
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
        t
    });
    let mut c = 0xFFFF_FFFFu32;
    for &b in bytes {
        c = table[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

/// Encode the [`CadkHeader`] into its fixed [`HEADER_SIZE`]-byte
/// little-endian on-disk form.
fn write_header(h: &CadkHeader) -> [u8; HEADER_SIZE] {
    let mut out = [0u8; HEADER_SIZE];
    out[0..4].copy_from_slice(&h.schema_version.to_le_bytes());
    out[4..8].copy_from_slice(&h.flags.to_le_bytes());
    out[8..16].copy_from_slice(&h.total_size.to_le_bytes());
    out[16..24].copy_from_slice(&h.manifest_offset.to_le_bytes());
    out[24..32].copy_from_slice(&h.manifest_length.to_le_bytes());
    out[32..36].copy_from_slice(&h.manifest_crc32.to_le_bytes());
    out[36..HEADER_SIZE].copy_from_slice(&h.reserved);
    out
}

/// Parse the [`HEADER_SIZE`]-byte on-disk header.
fn read_header(buf: &[u8]) -> ApiResult<CadkHeader> {
    if buf.len() < HEADER_SIZE {
        return Err(ApiError::Codec(format!(
            "header buffer too short: got {} bytes, expected {HEADER_SIZE}",
            buf.len()
        )));
    }
    let schema_version = u32::from_le_bytes(buf[0..4].try_into().unwrap());
    let flags = u32::from_le_bytes(buf[4..8].try_into().unwrap());
    let total_size = u64::from_le_bytes(buf[8..16].try_into().unwrap());
    let manifest_offset = u64::from_le_bytes(buf[16..24].try_into().unwrap());
    let manifest_length = u64::from_le_bytes(buf[24..32].try_into().unwrap());
    let manifest_crc32 = u32::from_le_bytes(buf[32..36].try_into().unwrap());
    let mut reserved = [0u8; 28];
    reserved.copy_from_slice(&buf[36..HEADER_SIZE]);
    Ok(CadkHeader {
        schema_version,
        flags,
        total_size,
        manifest_offset,
        manifest_length,
        manifest_crc32,
        reserved,
    })
}

/// Encode a session command log to the `.cadk` v0 container format.
///
/// Currently writes a single `BlobKind::Document` blob whose body is the
/// `serde_json` encoding of `commands`. Designed to be forward-compatible:
/// a future A3.1 patch will swap the body for `bincode 2` and gate the
/// switch on a header flag without changing the on-disk envelope.
pub fn encode(commands: &[Command]) -> ApiResult<Vec<u8>> {
    encode_with_thumbnail(commands, None)
}

/// Encode a command log with an optional embedded thumbnail blob (raw
/// bytes — typically a PNG payload). When `thumbnail` is `Some`, a
/// second `BlobKind::Thumbnail` record is appended to the manifest and
/// the `CadkFlags::HAS_THUMBNAIL` bit is set in the header.
pub fn encode_with_thumbnail(
    commands: &[Command],
    thumbnail: Option<&[u8]>,
) -> ApiResult<Vec<u8>> {
    // 1. Encode the document blob body.
    let doc_body = serde_json::to_vec(commands)?;
    let doc_crc = crc32_ieee(&doc_body);
    let thumb_crc = thumbnail.map(crc32_ieee);

    // 2. Layout: magic[4] header[64] manifest_blob content_blobs.
    //    The manifest is JSON, so its serialized length depends on the
    //    digit-count of the offset values — which depends on the
    //    manifest length itself. Iterate to a fixed point (converges in
    //    at most a few passes for any realistic blob count).
    let manifest_offset = (MAGIC.len() + HEADER_SIZE) as u64;
    let mut content_offset = manifest_offset; // start guess
    let (manifest_bytes, manifest_length) = loop {
        let mut records = vec![BlobRecord {
            kind: BlobKind::Document,
            name: "document".into(),
            offset: content_offset,
            length: doc_body.len() as u64,
            crc32: doc_crc,
        }];
        if let (Some(thumb), Some(crc)) = (thumbnail, thumb_crc) {
            records.push(BlobRecord {
                kind: BlobKind::Thumbnail,
                name: "thumbnail".into(),
                offset: content_offset + doc_body.len() as u64,
                length: thumb.len() as u64,
                crc32: crc,
            });
        }
        let manifest = Manifest { records };
        let bytes = serde_json::to_vec(&manifest)?;
        let new_content_offset = manifest_offset + bytes.len() as u64;
        if new_content_offset == content_offset {
            let len = bytes.len() as u64;
            break (bytes, len);
        }
        content_offset = new_content_offset;
    };
    let manifest_crc32 = crc32_ieee(&manifest_bytes);

    let thumb_len = thumbnail.map(|t| t.len() as u64).unwrap_or(0);
    let total_size = content_offset + doc_body.len() as u64 + thumb_len;
    let flags = if thumbnail.is_some() {
        CadkFlags::HAS_THUMBNAIL
    } else {
        0
    };
    let header = CadkHeader {
        schema_version: SCHEMA_VERSION,
        flags,
        total_size,
        manifest_offset,
        manifest_length,
        manifest_crc32,
        reserved: [0u8; 28],
    };

    let mut out = Vec::with_capacity(total_size as usize);
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&write_header(&header));
    out.extend_from_slice(&manifest_bytes);
    out.extend_from_slice(&doc_body);
    if let Some(thumb) = thumbnail {
        out.extend_from_slice(thumb);
    }
    debug_assert_eq!(out.len() as u64, total_size);
    Ok(out)
}

/// Decode a `.cadk` v0 container back to its command log.
pub fn decode(bytes: &[u8]) -> ApiResult<Vec<Command>> {
    if bytes.len() < MAGIC.len() + HEADER_SIZE {
        return Err(ApiError::Codec(format!(
            "container too small: {} bytes",
            bytes.len()
        )));
    }
    if &bytes[..MAGIC.len()] != MAGIC.as_slice() {
        return Err(ApiError::Codec(format!(
            "bad magic: expected {:?}, got {:?}",
            MAGIC,
            &bytes[..MAGIC.len()]
        )));
    }
    let header = read_header(&bytes[MAGIC.len()..MAGIC.len() + HEADER_SIZE])?;
    if !header.is_supported() {
        return Err(ApiError::Codec(format!(
            "unsupported .cadk header: schema_version={}, flags=0x{:08x}",
            header.schema_version, header.flags
        )));
    }
    if (header.total_size as usize) != bytes.len() {
        return Err(ApiError::Codec(format!(
            "container truncated: header.total_size={}, file_size={}",
            header.total_size,
            bytes.len()
        )));
    }
    let manifest_start = header.manifest_offset as usize;
    let manifest_end = manifest_start + header.manifest_length as usize;
    if manifest_end > bytes.len() {
        return Err(ApiError::Codec(
            "manifest range exceeds container bounds".into(),
        ));
    }
    let manifest_bytes = &bytes[manifest_start..manifest_end];
    if crc32_ieee(manifest_bytes) != header.manifest_crc32 {
        return Err(ApiError::Codec("manifest crc32 mismatch".into()));
    }
    let manifest: Manifest = serde_json::from_slice(manifest_bytes)?;
    let doc_record = manifest
        .find_first(BlobKind::Document)
        .ok_or_else(|| ApiError::Codec("no Document blob in manifest".into()))?;
    let doc_start = doc_record.offset as usize;
    let doc_end = doc_start + doc_record.length as usize;
    if doc_end > bytes.len() {
        return Err(ApiError::Codec(
            "document blob range exceeds container bounds".into(),
        ));
    }
    let doc_body = &bytes[doc_start..doc_end];
    if crc32_ieee(doc_body) != doc_record.crc32 {
        return Err(ApiError::Codec("document blob crc32 mismatch".into()));
    }
    let commands: Vec<Command> = serde_json::from_slice(doc_body)?;
    Ok(commands)
}

/// Extract the embedded thumbnail blob, if any, from a `.cadk` container.
///
/// Returns `Ok(None)` if the container has no thumbnail (`HAS_THUMBNAIL`
/// flag clear or no `BlobKind::Thumbnail` record). Validates magic,
/// header support, total size, manifest CRC, and thumbnail blob CRC
/// before returning the raw bytes.
pub fn decode_thumbnail(bytes: &[u8]) -> ApiResult<Option<Vec<u8>>> {
    if bytes.len() < MAGIC.len() + HEADER_SIZE {
        return Err(ApiError::Codec(format!(
            "container too small: {} bytes",
            bytes.len()
        )));
    }
    if &bytes[..MAGIC.len()] != MAGIC.as_slice() {
        return Err(ApiError::Codec("bad magic".into()));
    }
    let header = read_header(&bytes[MAGIC.len()..MAGIC.len() + HEADER_SIZE])?;
    if !header.is_supported() {
        return Err(ApiError::Codec(format!(
            "unsupported .cadk header: schema_version={}, flags=0x{:08x}",
            header.schema_version, header.flags
        )));
    }
    if (header.total_size as usize) != bytes.len() {
        return Err(ApiError::Codec("container truncated".into()));
    }
    if header.flags & CadkFlags::HAS_THUMBNAIL == 0 {
        return Ok(None);
    }
    let manifest_start = header.manifest_offset as usize;
    let manifest_end = manifest_start + header.manifest_length as usize;
    if manifest_end > bytes.len() {
        return Err(ApiError::Codec(
            "manifest range exceeds container bounds".into(),
        ));
    }
    let manifest_bytes = &bytes[manifest_start..manifest_end];
    if crc32_ieee(manifest_bytes) != header.manifest_crc32 {
        return Err(ApiError::Codec("manifest crc32 mismatch".into()));
    }
    let manifest: Manifest = serde_json::from_slice(manifest_bytes)?;
    let Some(record) = manifest.find_first(BlobKind::Thumbnail) else {
        return Ok(None);
    };
    let start = record.offset as usize;
    let end = start + record.length as usize;
    if end > bytes.len() {
        return Err(ApiError::Codec(
            "thumbnail blob range exceeds container bounds".into(),
        ));
    }
    let body = &bytes[start..end];
    if crc32_ieee(body) != record.crc32 {
        return Err(ApiError::Codec("thumbnail blob crc32 mismatch".into()));
    }
    Ok(Some(body.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cadk::header::CadkFlags;

    #[test]
    fn crc32_of_empty_is_zero() {
        assert_eq!(crc32_ieee(b""), 0);
    }

    #[test]
    fn crc32_of_known_string_matches_reference() {
        // "123456789" -> 0xCBF43926 per the IEEE 802.3 / zlib check.
        assert_eq!(crc32_ieee(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn header_round_trip_preserves_all_fields() {
        let h = CadkHeader {
            schema_version: SCHEMA_VERSION,
            flags: CadkFlags::HAS_THUMBNAIL,
            total_size: 1234,
            manifest_offset: 68,
            manifest_length: 200,
            manifest_crc32: 0xDEAD_BEEF,
            reserved: [0xAA; 28],
        };
        let bytes = write_header(&h);
        assert_eq!(bytes.len(), HEADER_SIZE);
        let parsed = read_header(&bytes).unwrap();
        assert_eq!(parsed, h);
    }

    #[test]
    fn empty_log_round_trips_through_cadk() {
        let bytes = encode(&[]).unwrap();
        assert_eq!(&bytes[..4], b"CADK");
        let decoded = decode(&bytes).unwrap();
        assert!(decoded.is_empty());
    }

    #[test]
    fn five_command_log_round_trips_through_cadk() {
        let log = vec![
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
            Command::BooleanUnion {
                lhs: crate::SolidId(0),
                rhs: crate::SolidId(1),
            },
            Command::DeleteSolid {
                id: crate::SolidId(2),
            },
        ];
        let bytes = encode(&log).unwrap();
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded, log);
    }

    #[test]
    fn bad_magic_is_rejected() {
        let mut bytes = encode(&[Command::CreateSphere { radius: 1.0 }]).unwrap();
        bytes[0] = b'X';
        let err = decode(&bytes).unwrap_err();
        assert!(matches!(err, ApiError::Codec(_)));
    }

    #[test]
    fn truncated_container_is_rejected() {
        let bytes = encode(&[Command::CreateSphere { radius: 1.0 }]).unwrap();
        let truncated = &bytes[..bytes.len() - 5];
        let err = decode(truncated).unwrap_err();
        assert!(matches!(err, ApiError::Codec(_)));
    }

    #[test]
    fn corrupted_document_blob_is_rejected_via_crc() {
        let mut bytes = encode(&[Command::CreateSphere { radius: 1.0 }]).unwrap();
        // Flip a byte deep inside the document blob.
        let last = bytes.len() - 4;
        bytes[last] ^= 0xFF;
        let err = decode(&bytes).unwrap_err();
        assert!(matches!(err, ApiError::Codec(_)));
    }

    #[test]
    fn encode_without_thumbnail_clears_flag_and_decode_returns_none() {
        let bytes = encode(&[Command::CreateSphere { radius: 1.0 }]).unwrap();
        let header = read_header(&bytes[MAGIC.len()..MAGIC.len() + HEADER_SIZE]).unwrap();
        assert_eq!(header.flags & CadkFlags::HAS_THUMBNAIL, 0);
        assert_eq!(decode_thumbnail(&bytes).unwrap(), None);
    }

    #[test]
    fn encode_with_thumbnail_sets_flag_and_round_trips_payload() {
        let log = vec![
            Command::CreateBox {
                dx: 1.0,
                dy: 2.0,
                dz: 3.0,
            },
            Command::CreateSphere { radius: 0.5 },
        ];
        // Pretend-PNG payload (just bytes — codec is content-agnostic).
        let thumb: Vec<u8> = (0u8..=255).cycle().take(2048).collect();
        let bytes = encode_with_thumbnail(&log, Some(&thumb)).unwrap();

        let header = read_header(&bytes[MAGIC.len()..MAGIC.len() + HEADER_SIZE]).unwrap();
        assert_ne!(header.flags & CadkFlags::HAS_THUMBNAIL, 0);

        // Document still decodes correctly with thumbnail present.
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded, log);

        // Thumbnail bytes round-trip exactly.
        let extracted = decode_thumbnail(&bytes).unwrap().unwrap();
        assert_eq!(extracted, thumb);
    }

    #[test]
    fn corrupted_thumbnail_blob_is_rejected_via_crc() {
        let thumb = b"some-thumbnail-bytes".to_vec();
        let mut bytes = encode_with_thumbnail(
            &[Command::CreateSphere { radius: 1.0 }],
            Some(&thumb),
        )
        .unwrap();
        // Flip a byte at the very end (inside the thumbnail blob).
        let last = bytes.len() - 1;
        bytes[last] ^= 0xFF;
        // Document blob is intact, so commands still decode.
        assert!(decode(&bytes).is_ok());
        // But the thumbnail CRC must reject.
        let err = decode_thumbnail(&bytes).unwrap_err();
        assert!(matches!(err, ApiError::Codec(_)));
    }
}
