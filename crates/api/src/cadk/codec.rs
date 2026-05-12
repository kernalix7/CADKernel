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
//! A3.0.2 (2026-05-12) adds opt-in zstd compression of the
//! `BlobKind::Document` blob via [`SaveOptions::compression_level`].
//! When set, the encoded blob bytes are zstd frames, the CRC is computed
//! over those frames, and the [`CadkFlags::DOCUMENT_COMPRESSED`] bit is
//! set on the header. Decode auto-detects the bit and decompresses
//! before parsing. Containers without the bit continue to round-trip
//! exactly as before (v0 fixture compatibility).

use crate::cadk::header::{CadkFlags, CadkHeader, HEADER_SIZE, MAGIC, SCHEMA_VERSION};
use crate::cadk::manifest::{BlobKind, BlobRecord, Manifest};
use crate::command::Command;
use crate::{ApiError, ApiResult};

/// Options controlling `.cadk` encoding side-effects. Default = no
/// compression, no thumbnail (matches the bytes produced by the
/// legacy [`encode`] entry point).
#[derive(Debug, Clone, Default)]
pub struct SaveOptions {
    /// `zstd` compression level for the `BlobKind::Document` body.
    /// `None` (the default) means no compression — bytes match the v0
    /// uncompressed layout, the header's [`CadkFlags::DOCUMENT_COMPRESSED`]
    /// bit stays clear, and the v0 golden fixture continues to match.
    /// When `Some(level)`, the document blob is zstd-encoded at that
    /// level (zstd accepts roughly `1..=22`; the `zstd` crate clamps
    /// invalid values).
    pub compression_level: Option<i32>,
    /// Optional thumbnail payload to embed (typically PNG). When
    /// `Some`, behaves like [`encode_with_thumbnail`].
    pub thumbnail: Option<Vec<u8>>,
}

impl SaveOptions {
    /// Builder helper: enable zstd compression at the given level.
    #[must_use]
    pub fn with_compression(mut self, level: i32) -> Self {
        self.compression_level = Some(level);
        self
    }

    /// Builder helper: embed a thumbnail payload.
    #[must_use]
    pub fn with_thumbnail(mut self, thumb: Vec<u8>) -> Self {
        self.thumbnail = Some(thumb);
        self
    }
}

/// Cheap read-only summary of a `.cadk` container.
///
/// Produced by [`inspect`]. Unlike [`decode`], reading a summary does not
/// touch the document blob body: only magic, header, and the manifest
/// table-of-contents are parsed and CRC-checked. This makes [`inspect`]
/// suitable for "Recent Files" lists, autosave dirs, and CI guards that
/// only need metadata (schema version, size, flags, blob count).
///
/// Added in A3.0.3 (2026-05-12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CadkSummary {
    /// Header schema version. Must equal [`SCHEMA_VERSION`] for this
    /// build — [`inspect`] errors out on mismatch before returning a
    /// summary.
    pub schema_version: u32,
    /// Raw flags word from the header. Use the
    /// [`document_compressed`](CadkSummary::document_compressed),
    /// [`has_thumbnail`](CadkSummary::has_thumbnail),
    /// [`is_signed`](CadkSummary::is_signed), and
    /// [`manifest_compressed`](CadkSummary::manifest_compressed) helpers
    /// for individual bit checks, or [`unknown_flags`](CadkSummary::unknown_flags)
    /// for forward-compat diagnostics.
    pub flags: u32,
    /// File size in bytes as declared by the header. [`inspect`] verifies
    /// this matches the input slice length before returning.
    pub total_size: u64,
    /// Number of records in the manifest. Includes the document blob,
    /// optional thumbnail, optional signature, and any forward-compat
    /// blobs preserved verbatim.
    pub blob_count: usize,
    /// Encoded length of the document blob in bytes. When the
    /// [`document_compressed`](CadkSummary::document_compressed) flag is
    /// set, this is the post-zstd size; otherwise it is the raw JSON
    /// length.
    pub document_length: u64,
    /// Encoded length of the thumbnail blob in bytes, if present. `None`
    /// when no `BlobKind::Thumbnail` record is in the manifest.
    pub thumbnail_length: Option<u64>,
}

impl CadkSummary {
    /// Returns `true` iff the document blob is zstd-compressed
    /// (i.e. [`CadkFlags::DOCUMENT_COMPRESSED`] is set).
    pub fn document_compressed(&self) -> bool {
        self.flags & CadkFlags::DOCUMENT_COMPRESSED != 0
    }

    /// Returns `true` iff a thumbnail blob is present
    /// (i.e. [`CadkFlags::HAS_THUMBNAIL`] is set). Always agrees with
    /// `self.thumbnail_length.is_some()`.
    pub fn has_thumbnail(&self) -> bool {
        self.flags & CadkFlags::HAS_THUMBNAIL != 0
    }

    /// Returns `true` iff the container declares an Ed25519 signature
    /// blob (i.e. [`CadkFlags::SIGNED`] is set). The signature itself is
    /// not validated by [`inspect`]; this is purely the header bit.
    pub fn is_signed(&self) -> bool {
        self.flags & CadkFlags::SIGNED != 0
    }

    /// Returns `true` iff the manifest body itself is zstd-compressed.
    /// As of A3.0.3 the encoder never sets this bit (the manifest is
    /// always raw JSON), but the predicate exists for forward compat.
    pub fn manifest_compressed(&self) -> bool {
        self.flags & CadkFlags::MANIFEST_COMPRESSED != 0
    }

    /// Returns the set of flag bits that are set but unknown to this
    /// build. Zero means the file was produced by an equally-or-older
    /// encoder. Non-zero means a newer encoder set bits this build does
    /// not yet recognise — they round-trip verbatim per the format spec.
    pub fn unknown_flags(&self) -> u32 {
        self.flags & !CadkFlags::KNOWN
    }
}

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
    // SAFETY-by-construction: every slice below has the exact length
    // required by `from_le_bytes`; the bounds were validated above.
    // We use `copy_from_slice` into a fixed-size array to keep this
    // panic-free for clippy's `unwrap_used` lint.
    let schema_version = read_le_u32(&buf[0..4]);
    let flags = read_le_u32(&buf[4..8]);
    let total_size = read_le_u64(&buf[8..16]);
    let manifest_offset = read_le_u64(&buf[16..24]);
    let manifest_length = read_le_u64(&buf[24..32]);
    let manifest_crc32 = read_le_u32(&buf[32..36]);
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

/// Read a little-endian `u32` from a 4-byte slice. Caller guarantees
/// `slice.len() == 4`; shorter slices are zero-padded which would only
/// happen via a programmer error inside this module.
#[inline]
fn read_le_u32(slice: &[u8]) -> u32 {
    let mut arr = [0u8; 4];
    let n = slice.len().min(4);
    arr[..n].copy_from_slice(&slice[..n]);
    u32::from_le_bytes(arr)
}

/// Read a little-endian `u64` from an 8-byte slice. Caller guarantees
/// `slice.len() == 8`; see [`read_le_u32`] for the zero-pad fallback.
#[inline]
fn read_le_u64(slice: &[u8]) -> u64 {
    let mut arr = [0u8; 8];
    let n = slice.len().min(8);
    arr[..n].copy_from_slice(&slice[..n]);
    u64::from_le_bytes(arr)
}

/// Encode a session command log to the `.cadk` v0 container format.
///
/// Currently writes a single `BlobKind::Document` blob whose body is the
/// `serde_json` encoding of `commands`. Designed to be forward-compatible:
/// a future A3.1 patch will swap the body for `bincode 2` and gate the
/// switch on a header flag without changing the on-disk envelope.
pub fn encode(commands: &[Command]) -> ApiResult<Vec<u8>> {
    encode_with_options(commands, &SaveOptions::default())
}

/// Encode a command log with an optional embedded thumbnail blob (raw
/// bytes — typically a PNG payload). When `thumbnail` is `Some`, a
/// second `BlobKind::Thumbnail` record is appended to the manifest and
/// the `CadkFlags::HAS_THUMBNAIL` bit is set in the header.
pub fn encode_with_thumbnail(
    commands: &[Command],
    thumbnail: Option<&[u8]>,
) -> ApiResult<Vec<u8>> {
    let opts = SaveOptions {
        thumbnail: thumbnail.map(<[u8]>::to_vec),
        ..SaveOptions::default()
    };
    encode_with_options(commands, &opts)
}

/// Encode a command log with the supplied [`SaveOptions`]. The single
/// entry point that all other `encode*` variants funnel through.
/// Honours both [`SaveOptions::compression_level`] (zstd-compresses the
/// document blob and sets [`CadkFlags::DOCUMENT_COMPRESSED`]) and
/// [`SaveOptions::thumbnail`] (appends a `BlobKind::Thumbnail` record).
pub fn encode_with_options(commands: &[Command], opts: &SaveOptions) -> ApiResult<Vec<u8>> {
    // 1. Encode the document blob body.
    let raw_doc = serde_json::to_vec(commands)?;
    let (doc_body, doc_compressed) = match opts.compression_level {
        Some(level) => {
            let compressed = zstd::encode_all(raw_doc.as_slice(), level)
                .map_err(|e| ApiError::Codec(format!("zstd encode: {e}")))?;
            (compressed, true)
        }
        None => (raw_doc, false),
    };
    let doc_crc = crc32_ieee(&doc_body);
    let thumbnail = opts.thumbnail.as_deref();
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
    let mut flags = 0u32;
    if thumbnail.is_some() {
        flags |= CadkFlags::HAS_THUMBNAIL;
    }
    if doc_compressed {
        flags |= CadkFlags::DOCUMENT_COMPRESSED;
    }
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
    // Auto-detect zstd compression via the DOCUMENT_COMPRESSED flag. The
    // CRC is computed over the on-disk (potentially compressed) bytes;
    // decompression happens after the integrity check.
    let commands: Vec<Command> = if header.flags & CadkFlags::DOCUMENT_COMPRESSED != 0 {
        let decoded = zstd::decode_all(doc_body)
            .map_err(|e| ApiError::Codec(format!("zstd decode: {e}")))?;
        serde_json::from_slice(&decoded)?
    } else {
        serde_json::from_slice(doc_body)?
    };
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

/// Inspect a `.cadk` container without decoding the document log.
///
/// Validates magic, header (schema version + must-understand flags),
/// declared `total_size` against the input length, manifest range
/// in-bounds, and manifest CRC. Returns a [`CadkSummary`] with version,
/// flags, and blob counts.
///
/// Deliberately cheaper than [`decode`]: the document body is **not**
/// CRC-checked, not zstd-decompressed, and not JSON-parsed. Use this for
/// metadata-only queries (Recent Files, autosave directory listings, CI
/// fixture guards). Use [`decode`] when you need the actual commands or
/// full per-blob integrity.
///
/// Added in A3.0.3 (2026-05-12).
pub fn inspect(bytes: &[u8]) -> ApiResult<CadkSummary> {
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

    let document_length = manifest
        .find_first(BlobKind::Document)
        .map(|r| r.length)
        .ok_or_else(|| ApiError::Codec("no Document blob in manifest".into()))?;
    let thumbnail_length = manifest.find_first(BlobKind::Thumbnail).map(|r| r.length);

    Ok(CadkSummary {
        schema_version: header.schema_version,
        flags: header.flags,
        total_size: header.total_size,
        blob_count: manifest.records.len(),
        document_length,
        thumbnail_length,
    })
}

/// Filesystem wrapper around [`inspect`]. Reads the file at `path` and
/// returns a [`CadkSummary`] without keeping the bytes in memory beyond
/// the parse. I/O failures are surfaced as
/// [`ApiError::Codec`] with a `file io:` prefix so existing match arms
/// over `ApiError` continue to compile (matches the convention set by
/// [`crate::Session::save_cadk_to_path`] and
/// [`crate::Session::load_cadk_from_path`]).
///
/// Added in A3.0.4 (2026-05-13).
pub fn inspect_path(path: impl AsRef<std::path::Path>) -> ApiResult<CadkSummary> {
    let bytes = std::fs::read(path.as_ref())
        .map_err(|err| ApiError::Codec(format!("file io: {err}")))?;
    inspect(&bytes)
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
