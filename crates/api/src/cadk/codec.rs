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
use serde::{Deserialize, Serialize};

/// Hard upper bound for the decompressed `BlobKind::Document` payload, used to
/// refuse zstd decompression-bomb payloads where a tiny on-disk blob expands
/// to gigabytes. 32 MiB sits well above realistic command-log JSON (the R1/R2
/// reference parts decompress to < 50 KiB) while bounding adversarial memory.
const MAX_DECOMPRESSED_DOCUMENT_BYTES: u64 = 32 * 1024 * 1024;

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

/// Logical document payload stored in `BlobKind::Document` for schema v2.
///
/// Schema v1 used the document blob as a bare `Vec<Command>`. The v2 wrapper
/// keeps that command log as the replay source of truth and adds snapshot
/// sections for state that is expensive or impossible to infer in future
/// schema lines.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CadkDocumentData {
    /// Applied command prefix. This remains the canonical replay log.
    #[serde(default)]
    pub commands: Vec<Command>,
    /// Persisted PartDesign body snapshots.
    #[serde(default)]
    pub bodies: Vec<CadkBodySnapshot>,
    /// Persisted sketch snapshots. Empty until the sketch persistence lane
    /// adds a concrete public sketch schema.
    #[serde(default)]
    pub sketches: Vec<serde_json::Value>,
}

/// Serializable v2 snapshot of a PartDesign body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CadkBodySnapshot {
    pub id: u64,
    pub name: String,
    pub features: Vec<CadkBodyFeatureSnapshot>,
    pub tip: Option<usize>,
    pub base_plane_origin: [f64; 3],
    pub base_plane_normal: [f64; 3],
    pub current_solid: Option<u32>,
}

/// Serializable v2 snapshot of one Body feature.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CadkBodyFeatureSnapshot {
    pub feature_id: u64,
    pub name: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec: Option<serde_json::Value>,
    pub spec_kind: String,
    pub suppressed: bool,
    pub solid: CadkHandleSnapshot,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cached_solid: Option<CadkHandleSnapshot>,
}

/// Raw generational handle representation for snapshots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CadkHandleSnapshot {
    pub index: u32,
    pub generation: u64,
}

impl CadkDocumentData {
    fn from_commands(commands: &[Command]) -> Self {
        Self {
            commands: commands.to_vec(),
            bodies: body_snapshots_from_commands(commands),
            sketches: Vec::new(),
        }
    }

    fn from_legacy_commands(commands: Vec<Command>) -> Self {
        Self {
            commands,
            bodies: Vec::new(),
            sketches: Vec::new(),
        }
    }
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

/// Public per-blob view returned via [`CadkSummary::blobs`]. Excludes
/// internal codec fields (offset, CRC) that callers should not depend
/// on across format versions — those live on the on-disk [`BlobRecord`]
/// type. `kind`, `name`, and `length` are stable cross-version.
///
/// Added in A3.0.6 (2026-05-13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobInfo {
    /// Blob kind tag (Document / Thumbnail / Signature / Attachment /
    /// History / Unknown — see [`BlobKind`]).
    pub kind: BlobKind,
    /// Application-level name as written in the manifest (e.g.
    /// `"document"`, `"thumbnail"`).
    pub name: String,
    /// Encoded blob length in bytes. Matches the on-disk size — when
    /// `kind == Document` and the [`CadkFlags::DOCUMENT_COMPRESSED`]
    /// header bit is set, this is the post-zstd length.
    pub length: u64,
}

/// Cheap read-only summary of a `.cadk` container.
///
/// Produced by [`inspect`]. Unlike [`decode`], reading a summary does not
/// touch the document blob body: only magic, header, and the manifest
/// table-of-contents are parsed and CRC-checked. This makes [`inspect`]
/// suitable for "Recent Files" lists, autosave dirs, and CI guards that
/// only need metadata (schema version, size, flags, blob count).
///
/// Added in A3.0.3 (2026-05-12); per-blob `blobs` list added in A3.0.6
/// (2026-05-13).
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
    /// Per-blob summary in manifest order. Each entry exposes the
    /// stable subset of the underlying [`BlobRecord`] — kind, name,
    /// encoded length — so callers can enumerate forward-compat blob
    /// kinds (e.g. `BlobKind::Unknown` or future variants) without
    /// re-parsing the manifest.
    ///
    /// Added in A3.0.6 (2026-05-13).
    pub blobs: Vec<BlobInfo>,
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

fn handle_snapshot<T>(handle: cadkernel_topology::Handle<T>) -> CadkHandleSnapshot {
    CadkHandleSnapshot {
        index: handle.index(),
        generation: handle.generation(),
    }
}

fn body_snapshots_from_commands(commands: &[Command]) -> Vec<CadkBodySnapshot> {
    let Ok(session) = crate::Session::replay(commands) else {
        return Vec::new();
    };
    session
        .document()
        .body_ids()
        .into_iter()
        .filter_map(|id| {
            let body = session.document().body(id)?;
            let features = body
                .features
                .iter()
                .map(|feature| CadkBodyFeatureSnapshot {
                    feature_id: feature.feature_id,
                    name: feature.name.clone(),
                    kind: feature.kind.as_str().to_string(),
                    spec: feature.spec.clone(),
                    spec_kind: feature.spec_kind.clone(),
                    suppressed: feature.suppressed,
                    solid: handle_snapshot(feature.solid),
                    cached_solid: feature.cached_solid.map(handle_snapshot),
                })
                .collect();
            Some(CadkBodySnapshot {
                id: body.id,
                name: body.name.clone(),
                features,
                tip: body.tip,
                base_plane_origin: body.base_plane_origin,
                base_plane_normal: body.base_plane_normal,
                current_solid: body.current_solid,
            })
        })
        .collect()
}

fn parse_document_data(raw_doc: &[u8]) -> ApiResult<CadkDocumentData> {
    let value: serde_json::Value = serde_json::from_slice(raw_doc)?;
    if value.is_array() {
        let commands: Vec<Command> = serde_json::from_value(value)?;
        return Ok(CadkDocumentData::from_legacy_commands(commands));
    }
    let data: CadkDocumentData = serde_json::from_value(value)?;
    Ok(data)
}

fn decode_document_bytes(header: &CadkHeader, doc_body: &[u8]) -> ApiResult<Vec<u8>> {
    if header.flags & CadkFlags::DOCUMENT_COMPRESSED == 0 {
        return Ok(doc_body.to_vec());
    }
    use std::io::Read;
    let decoder =
        zstd::Decoder::new(doc_body).map_err(|e| ApiError::Codec(format!("zstd decode: {e}")))?;
    let mut decoded = Vec::new();
    decoder
        .take(MAX_DECOMPRESSED_DOCUMENT_BYTES + 1)
        .read_to_end(&mut decoded)
        .map_err(|e| ApiError::Codec(format!("zstd decode: {e}")))?;
    if decoded.len() as u64 > MAX_DECOMPRESSED_DOCUMENT_BYTES {
        return Err(ApiError::Codec(format!(
            "decompressed document blob exceeds {} MB cap",
            MAX_DECOMPRESSED_DOCUMENT_BYTES / 1024 / 1024
        )));
    }
    Ok(decoded)
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
pub fn encode_with_thumbnail(commands: &[Command], thumbnail: Option<&[u8]>) -> ApiResult<Vec<u8>> {
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
    let document = CadkDocumentData::from_commands(commands);
    encode_document_data_with_options(&document, opts)
}

pub(crate) fn encode_document_data_with_options(
    document: &CadkDocumentData,
    opts: &SaveOptions,
) -> ApiResult<Vec<u8>> {
    // 1. Encode the document blob body.
    let raw_doc = serde_json::to_vec(document)?;
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

/// Decode a `.cadk` container back to its command log.
pub fn decode(bytes: &[u8]) -> ApiResult<Vec<Command>> {
    Ok(decode_document_data(bytes)?.commands)
}

/// Decode a `.cadk` container into the logical v2 document payload.
///
/// Schema v1 containers are accepted by adapting the legacy bare command-log
/// JSON array to a payload with empty `bodies` and `sketches` sections.
pub fn decode_document_data(bytes: &[u8]) -> ApiResult<CadkDocumentData> {
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
    // decompression happens after the integrity check, bounded by
    // [`MAX_DECOMPRESSED_DOCUMENT_BYTES`] to refuse zstd "decompression bomb"
    // payloads where a small on-disk blob expands to gigabytes.
    let raw_doc = decode_document_bytes(&header, doc_body)?;
    parse_document_data(&raw_doc)
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

    let blobs = manifest
        .records
        .iter()
        .map(|r| BlobInfo {
            kind: r.kind,
            name: r.name.clone(),
            length: r.length,
        })
        .collect();

    Ok(CadkSummary {
        schema_version: header.schema_version,
        flags: header.flags,
        total_size: header.total_size,
        blob_count: manifest.records.len(),
        document_length,
        thumbnail_length,
        blobs,
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
    let bytes =
        std::fs::read(path.as_ref()).map_err(|err| ApiError::Codec(format!("file io: {err}")))?;
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
        let mut bytes =
            encode_with_thumbnail(&[Command::CreateSphere { radius: 1.0 }], Some(&thumb)).unwrap();
        // Flip a byte at the very end (inside the thumbnail blob).
        let last = bytes.len() - 1;
        bytes[last] ^= 0xFF;
        // Document blob is intact, so commands still decode.
        assert!(decode(&bytes).is_ok());
        // But the thumbnail CRC must reject.
        let err = decode_thumbnail(&bytes).unwrap_err();
        assert!(matches!(err, ApiError::Codec(_)));
    }

    #[test]
    fn decompression_cap_constant_is_sane() {
        // Cap must be large enough for realistic command logs (R1/R2 are
        // < 50 KiB) but small enough to bound zstd-bomb memory cost.
        // Both bounds are compile-time checked so a future cap change that
        // violates them fails the build, not just this test.
        const { assert!(MAX_DECOMPRESSED_DOCUMENT_BYTES >= 1024 * 1024) };
        const { assert!(MAX_DECOMPRESSED_DOCUMENT_BYTES <= 128 * 1024 * 1024) };
    }

    #[test]
    fn streaming_decoder_take_rejects_oversized_payload() {
        // Mirror the cap-enforcement primitive used in `decode()`: a payload
        // that decompresses past the cap fills the buffer to (cap + 1) bytes,
        // which is the exact signal `decode()` uses to refuse the document.
        use std::io::Read;
        let raw = vec![b'x'; 1024];
        let compressed = zstd::encode_all(raw.as_slice(), 1).expect("encode");
        let cap: u64 = 512;
        let decoder = zstd::Decoder::new(compressed.as_slice()).expect("init");
        let mut out = Vec::new();
        decoder.take(cap + 1).read_to_end(&mut out).expect("read");
        assert!(
            (out.len() as u64) > cap,
            "cap-trip condition must hold (got {} bytes, cap {})",
            out.len(),
            cap
        );
    }

    #[test]
    fn streaming_decoder_take_accepts_within_cap() {
        use std::io::Read;
        let raw = vec![b'x'; 256];
        let compressed = zstd::encode_all(raw.as_slice(), 1).expect("encode");
        let cap: u64 = 1024;
        let decoder = zstd::Decoder::new(compressed.as_slice()).expect("init");
        let mut out = Vec::new();
        decoder.take(cap + 1).read_to_end(&mut out).expect("read");
        assert_eq!(out.len(), 256);
        assert!((out.len() as u64) <= cap);
    }
}
