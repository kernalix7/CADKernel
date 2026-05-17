//! `.cadk` magic bytes + header.
//!
//! The header is a fixed 64-byte block immediately after the 4-byte
//! magic. It is the minimum information needed to:
//!
//! - identify the file format (`magic`),
//! - reject incompatible schemas (`schema_version`),
//! - verify the container hasn't been truncated (`total_size`),
//! - detect manifest corruption before parsing the manifest body
//!   (`manifest_offset`, `manifest_length`, `manifest_crc32`).
//!
//! All multi-byte fields are little-endian. Reserved bytes are zero on
//! write, ignored on read, and reserved for future feature flags.

use serde::{Deserialize, Serialize};

/// `"CADK"` in ASCII. The 4-byte preamble that identifies a `.cadk` file.
pub const MAGIC: [u8; 4] = *b"CADK";

/// Current header schema version. Bumped whenever the on-disk layout
/// changes in a way migrators must handle.
pub const SCHEMA_VERSION: u32 = 2;

/// Fixed on-disk header size in bytes (excluding the 4-byte magic).
pub const HEADER_SIZE: usize = 64;

/// Feature flags carried in [`CadkHeader::flags`].
///
/// Unknown bits MUST be preserved on round-trip. Unknown bits inside the
/// [`CadkFlags::MUST_UNDERSTAND_MASK`] (top 16) cause
/// [`CadkHeader::is_supported`] to return `false` so older readers
/// refuse to load files they would silently misinterpret.
pub struct CadkFlags;

impl CadkFlags {
    /// Manifest blob is zstd-compressed.
    pub const MANIFEST_COMPRESSED: u32 = 0b0000_0001;
    /// Container carries an Ed25519 signature blob.
    pub const SIGNED: u32 = 0b0000_0010;
    /// Container has an embedded thumbnail blob.
    pub const HAS_THUMBNAIL: u32 = 0b0000_0100;
    /// `BlobKind::Document` payload is zstd-compressed (added A3.0.2).
    /// Decoders without zstd support encounter a CRC match but a JSON
    /// parse failure on the raw payload, which surfaces as `ApiError::Codec`.
    pub const DOCUMENT_COMPRESSED: u32 = 0b0000_1000;

    /// Bitwise OR of every flag this build understands.
    pub const KNOWN: u32 =
        Self::MANIFEST_COMPRESSED | Self::SIGNED | Self::HAS_THUMBNAIL | Self::DOCUMENT_COMPRESSED;

    /// Bits in the top half of the flag word are "must understand": an
    /// unknown bit there forces a load-side reject.
    pub const MUST_UNDERSTAND_MASK: u32 = 0xFFFF_0000;
}

/// 64-byte header that follows the 4-byte magic preamble.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CadkHeader {
    /// Header schema version. Readers accept v1 and the current v2.
    pub schema_version: u32,
    /// Feature flags. See [`CadkFlags`].
    pub flags: u32,
    /// Total size of the file in bytes (magic + header + manifest +
    /// content). Used to detect truncation.
    pub total_size: u64,
    /// Byte offset of the manifest blob from the start of the file.
    pub manifest_offset: u64,
    /// Manifest blob length in bytes.
    pub manifest_length: u64,
    /// CRC-32 (IEEE 802.3 polynomial) of the manifest blob.
    pub manifest_crc32: u32,
    /// Padding so the header always serializes to exactly
    /// [`HEADER_SIZE`] bytes. Must be zero on write.
    pub reserved: [u8; 28],
}

impl Default for CadkHeader {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            flags: 0,
            total_size: 0,
            manifest_offset: 0,
            manifest_length: 0,
            manifest_crc32: 0,
            reserved: [0u8; 28],
        }
    }
}

impl CadkHeader {
    /// Returns `true` if this header is compatible with the current
    /// reader schema (i.e. no unknown must-understand flags are set).
    pub fn is_supported(&self) -> bool {
        if !(1..=SCHEMA_VERSION).contains(&self.schema_version) {
            return false;
        }
        self.has_supported_flags()
    }

    /// Returns true when the flag word does not set unknown
    /// must-understand bits. This is weaker than [`Self::is_supported`]:
    /// it intentionally does not reject future schema versions so cheap
    /// manifest inspection can still report their blob table.
    pub fn has_supported_flags(&self) -> bool {
        let unknown_must_understand =
            self.flags & CadkFlags::MUST_UNDERSTAND_MASK & !CadkFlags::KNOWN;
        unknown_must_understand == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magic_is_ascii_cadk() {
        assert_eq!(&MAGIC, b"CADK");
    }

    #[test]
    fn default_header_is_supported() {
        let h = CadkHeader::default();
        assert_eq!(h.schema_version, SCHEMA_VERSION);
        assert!(h.is_supported());
    }

    #[test]
    fn unknown_must_understand_flag_is_rejected() {
        // Set an unknown bit in the must-understand region.
        let h = CadkHeader {
            flags: 0x0001_0000,
            ..CadkHeader::default()
        };
        assert!(!h.is_supported());
    }

    #[test]
    fn known_flags_are_supported() {
        let h = CadkHeader {
            flags: CadkFlags::MANIFEST_COMPRESSED
                | CadkFlags::SIGNED
                | CadkFlags::HAS_THUMBNAIL
                | CadkFlags::DOCUMENT_COMPRESSED,
            ..CadkHeader::default()
        };
        assert!(h.is_supported());
    }

    #[test]
    fn document_compressed_bit_is_distinct() {
        // Guard against accidental flag overlap during future additions.
        let all = CadkFlags::MANIFEST_COMPRESSED
            | CadkFlags::SIGNED
            | CadkFlags::HAS_THUMBNAIL
            | CadkFlags::DOCUMENT_COMPRESSED;
        // Each flag is exactly one bit, and they do not overlap, so the
        // population count of the union equals the number of flags.
        assert_eq!(all.count_ones(), 4);
        assert_eq!(CadkFlags::KNOWN, all);
    }

    #[test]
    fn schema_version_mismatch_is_unsupported() {
        let h = CadkHeader {
            schema_version: 999,
            ..CadkHeader::default()
        };
        assert!(!h.is_supported());
        assert!(h.has_supported_flags());
    }

    #[test]
    fn v1_header_remains_supported() {
        let h = CadkHeader {
            schema_version: 1,
            ..CadkHeader::default()
        };
        assert!(h.is_supported());
    }
}
