//! `.cadk` schema-version dispatch and migration.
//!
//! [`SchemaVersion`] is the typed view over the raw `u32` carried in
//! [`crate::cadk::header::CadkHeader::schema_version`]. It lets callers
//! match exhaustively against known versions rather than threading
//! integer constants through the rest of the codebase.
//!
//! [`migrate_to_current`] is the entry point that transparently brings
//! older containers up to [`SchemaVersion::current()`]. Schema v2 adds
//! explicit document sections for Body and Sketch persistence while keeping
//! the command log as the replay source of truth.
//!
//! Note that [`migrate_to_current`] deliberately does **not** route
//! through [`crate::cadk::inspect`] — `inspect`'s
//! [`crate::cadk::header::CadkHeader::is_supported`] check would
//! reject unknown versions outright, which is the opposite of what a
//! migrator wants. Instead, we do a minimal cheap parse of magic +
//! `schema_version` to find the right migrator, run it, and only
//! then is the migrated payload safe to feed into the regular
//! validation path.

use crate::cadk::header::MAGIC;
use crate::{ApiError, ApiResult};

/// Typed representation of the on-disk `.cadk` schema version. Wraps
/// the raw `u32` in [`crate::cadk::header::CadkHeader::schema_version`]
/// so callers can match exhaustively against known versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SchemaVersion {
    /// The original `.cadk` container layout — current head of the
    /// version line, written by every build through A3.0.x. Carries
    /// integer `1` on disk.
    V1,
    /// Adds Body persistence. Wave 1 sub-phase 5a only reserves the
    /// variant; Wave 3 makes it the current encoder target.
    V2,
    /// A future schema version that this build can inspect at the
    /// container/manifest level but must not decode as a document log.
    Unknown(u32),
}

impl SchemaVersion {
    /// Returns the enum variant for `n`, or `None` if `n` is not
    /// a valid released schema marker. Future non-zero versions map to
    /// [`Self::Unknown`] so callers can preserve and report the raw value.
    pub fn from_u32(n: u32) -> Option<Self> {
        match n {
            0 => None,
            1 => Some(Self::V1),
            2 => Some(Self::V2),
            other => Some(Self::Unknown(other)),
        }
    }

    /// Returns the on-disk integer for this version. Matches
    /// [`crate::cadk::header::SCHEMA_VERSION`] for [`Self::current`].
    pub fn as_u32(self) -> u32 {
        match self {
            Self::V1 => 1,
            Self::V2 => 2,
            Self::Unknown(n) => n,
        }
    }

    /// Returns the version this build writes by default — the head of
    /// the version line that every fresh `encode` produces.
    pub fn current() -> Self {
        Self::V2
    }

    /// Returns true for versions whose document log this build can decode.
    pub fn is_known(self) -> bool {
        matches!(self, Self::V1 | Self::V2)
    }

    /// Convert to an on-disk value for writer paths.
    ///
    /// Unknown versions are read-only compatibility fences: the manifest can
    /// be inspected, but this build must never emit a container claiming to
    /// implement a schema it does not understand.
    pub fn writable_u32(self) -> ApiResult<u32> {
        match self {
            Self::V1 | Self::V2 => Ok(self.as_u32()),
            Self::Unknown(n) => Err(ApiError::Codec(format!(
                "cannot encode unknown .cadk schema version: {n}"
            ))),
        }
    }
}

/// Migrate a `.cadk` byte buffer from any supported schema version up
/// to [`SchemaVersion::current()`]. Returns the migrated bytes, which
/// are byte-identical to the input when no migration was necessary.
///
/// Bypasses [`crate::cadk::inspect`] on purpose: that path rejects
/// unknown schema versions via
/// [`crate::cadk::header::CadkHeader::is_supported`], which is the
/// opposite of what a migrator needs. Instead, only magic and the
/// 4-byte `schema_version` field are parsed up front.
pub fn migrate_to_current(bytes: &[u8]) -> ApiResult<Vec<u8>> {
    if bytes.len() < MAGIC.len() + 4 {
        return Err(ApiError::Codec(format!(
            "container too small for migration probe: {} bytes",
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
    let raw = u32::from_le_bytes([
        bytes[MAGIC.len()],
        bytes[MAGIC.len() + 1],
        bytes[MAGIC.len() + 2],
        bytes[MAGIC.len() + 3],
    ]);
    let version = SchemaVersion::from_u32(raw)
        .ok_or_else(|| ApiError::Codec(format!("unsupported schema version: {raw}")))?;
    match version {
        SchemaVersion::V1 if SchemaVersion::current() == SchemaVersion::V2 => v1_to_v2(bytes),
        SchemaVersion::V1 => Ok(bytes.to_vec()),
        SchemaVersion::V2 => Ok(bytes.to_vec()),
        SchemaVersion::Unknown(n) => Err(ApiError::Codec(format!(
            "unsupported schema version for migration: {n}"
        ))),
    }
}

/// Migrate a V1 container to V2 by adapting the legacy command-log array
/// into the v2 document wrapper with empty Body/Sketch sections.
fn v1_to_v2(bytes: &[u8]) -> ApiResult<Vec<u8>> {
    let summary = crate::cadk::inspect(bytes)?;
    let commands = crate::cadk::decode(bytes)?;
    let thumbnail = crate::cadk::decode_thumbnail(bytes)?;
    let opts = crate::cadk::SaveOptions {
        compression_level: summary.document_compressed().then_some(3),
        thumbnail,
    };
    let document = crate::cadk::CadkDocumentData {
        commands,
        bodies: Vec::new(),
        sketches: Vec::new(),
    };
    crate::cadk::codec::encode_document_data_with_options(&document, &opts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_version_round_trips_through_u32() {
        for v in [SchemaVersion::V1, SchemaVersion::V2] {
            assert_eq!(SchemaVersion::from_u32(v.as_u32()), Some(v));
        }
    }

    #[test]
    fn schema_version_v2_round_trips_through_u32() {
        let v = SchemaVersion::V2;
        assert_eq!(v.as_u32(), 2);
        assert_eq!(SchemaVersion::from_u32(v.as_u32()), Some(v));
    }

    #[test]
    fn current_schema_version_is_v2() {
        assert_eq!(SchemaVersion::current(), SchemaVersion::V2);
        assert_eq!(
            SchemaVersion::current().as_u32(),
            crate::cadk::header::SCHEMA_VERSION
        );
    }

    #[test]
    fn from_u32_rejects_unknown_versions() {
        assert_eq!(SchemaVersion::from_u32(0), None);
        assert_eq!(SchemaVersion::from_u32(3), Some(SchemaVersion::Unknown(3)));
        assert_eq!(
            SchemaVersion::from_u32(u32::MAX),
            Some(SchemaVersion::Unknown(u32::MAX))
        );
    }

    #[test]
    fn unknown_schema_version_is_read_only() {
        let v = SchemaVersion::Unknown(3);
        assert_eq!(v.as_u32(), 3);
        assert!(!v.is_known());
        assert!(v.writable_u32().is_err());
    }

    #[test]
    fn migrate_rejects_truncated_buffer() {
        let err = migrate_to_current(b"CAD").unwrap_err();
        assert!(matches!(err, ApiError::Codec(_)));
    }

    #[test]
    fn migrate_rejects_bad_magic() {
        let mut bytes = Vec::from(*b"XXXX");
        bytes.extend_from_slice(&1u32.to_le_bytes());
        let err = migrate_to_current(&bytes).unwrap_err();
        match err {
            ApiError::Codec(msg) => assert!(msg.contains("bad magic"), "got: {msg}"),
            other => panic!("expected Codec, got {other:?}"),
        }
    }

    #[test]
    fn migrate_rejects_unknown_schema_version() {
        let mut bytes = Vec::from(MAGIC);
        bytes.extend_from_slice(&999u32.to_le_bytes());
        let err = migrate_to_current(&bytes).unwrap_err();
        match err {
            ApiError::Codec(msg) => {
                assert!(msg.contains("unsupported schema version"), "got: {msg}")
            }
            other => panic!("expected Codec, got {other:?}"),
        }
    }
}
