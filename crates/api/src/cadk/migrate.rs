//! `.cadk` schema-version dispatch and migration.
//!
//! [`SchemaVersion`] is the typed view over the raw `u32` carried in
//! [`crate::cadk::header::CadkHeader::schema_version`]. It lets callers
//! match exhaustively against known versions rather than threading
//! integer constants through the rest of the codebase.
//!
//! [`migrate_to_current`] is the entry point that transparently brings
//! older containers up to [`SchemaVersion::current()`]. As of A3.0.7
//! (2026-05-13) the format has exactly one version (V1), so the
//! function is always a no-op for healthy bytes. The scaffold exists
//! to pin the contract for future schema bumps: when V2 ships, the
//! match in [`migrate_to_current`] grows a `V1 → V2` transform and
//! callers reading old files transparently call this before
//! [`crate::cadk::decode`].
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
}

impl SchemaVersion {
    /// Returns the enum variant for `n`, or `None` if `n` is not
    /// recognised by this build. Used by [`migrate_to_current`] to
    /// dispatch to the right migrator.
    pub fn from_u32(n: u32) -> Option<Self> {
        match n {
            1 => Some(Self::V1),
            _ => None,
        }
    }

    /// Returns the on-disk integer for this version. Matches
    /// [`crate::cadk::header::SCHEMA_VERSION`] for [`Self::current`].
    pub fn as_u32(self) -> u32 {
        match self {
            Self::V1 => 1,
        }
    }

    /// Returns the version this build writes by default — the head of
    /// the version line that every fresh `encode` produces.
    pub fn current() -> Self {
        Self::V1
    }
}

/// Migrate a `.cadk` byte buffer from any supported schema version up
/// to [`SchemaVersion::current()`]. Returns the migrated bytes, which
/// are byte-identical to the input when no migration was necessary.
///
/// As of A3.0.7 the format has exactly one supported version (V1) so
/// this function is always a no-op for healthy bytes — it exists to
/// pin the migrator contract for future schema bumps.
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
    let version = SchemaVersion::from_u32(raw).ok_or_else(|| {
        ApiError::Codec(format!("unsupported schema version: {raw}"))
    })?;
    match version {
        // No migration needed — already at the current schema head.
        SchemaVersion::V1 => Ok(bytes.to_vec()),
        // Future bumps:
        // SchemaVersion::V2 => migrate_v2_to_v3(bytes),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_version_round_trips_through_u32() {
        let v = SchemaVersion::V1;
        assert_eq!(SchemaVersion::from_u32(v.as_u32()), Some(v));
    }

    #[test]
    fn current_schema_version_is_v1() {
        assert_eq!(SchemaVersion::current(), SchemaVersion::V1);
        assert_eq!(
            SchemaVersion::current().as_u32(),
            crate::cadk::header::SCHEMA_VERSION
        );
    }

    #[test]
    fn from_u32_rejects_unknown_versions() {
        assert_eq!(SchemaVersion::from_u32(0), None);
        assert_eq!(SchemaVersion::from_u32(2), None);
        assert_eq!(SchemaVersion::from_u32(u32::MAX), None);
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
            ApiError::Codec(msg) => assert!(
                msg.contains("unsupported schema version"),
                "got: {msg}"
            ),
            other => panic!("expected Codec, got {other:?}"),
        }
    }
}
