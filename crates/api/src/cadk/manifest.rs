//! `.cadk` manifest — table of contents for the content blobs.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Kind tag for a content blob. The reader uses this to decide which
/// decoder to apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlobKind {
    /// Document body (`bincode 2`-encoded `Document` after migration).
    Document,
    /// Embedded 256×256 PNG thumbnail.
    Thumbnail,
    /// Command history log.
    History,
    /// PartDesign Body snapshot payload introduced by schema v2.
    Bodies,
    /// Persisted sketch snapshot payload introduced by schema v2.
    Sketches,
    /// User-attached file (DXF, image, PDF reference).
    Attachment,
    /// Ed25519 signature over the manifest hash.
    Signature,
    /// Forward-compat fallback. Unknown manifest kind strings map here so
    /// newer blob tables can still be inspected.
    Unknown,
}

impl BlobKind {
    /// Stable lowercase wire spelling used in manifest JSON.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::Thumbnail => "thumbnail",
            Self::History => "history",
            Self::Bodies => "bodies",
            Self::Sketches => "sketches",
            Self::Attachment => "attachment",
            Self::Signature => "signature",
            Self::Unknown => "unknown",
        }
    }

    fn from_wire(s: &str) -> Self {
        match s {
            "document" => Self::Document,
            "thumbnail" => Self::Thumbnail,
            "history" => Self::History,
            "bodies" => Self::Bodies,
            "sketches" => Self::Sketches,
            "attachment" => Self::Attachment,
            "signature" => Self::Signature,
            "unknown" => Self::Unknown,
            _ => Self::Unknown,
        }
    }
}

impl Serialize for BlobKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for BlobKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = String::deserialize(deserializer)?;
        Ok(Self::from_wire(&wire))
    }
}

/// One entry in the manifest table of contents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobRecord {
    /// Kind tag.
    pub kind: BlobKind,
    /// Human / app-level name (e.g. `"document"`, `"thumb-256"`).
    pub name: String,
    /// Byte offset from the start of the file.
    pub offset: u64,
    /// Length of the encoded (zstd-compressed) blob in bytes.
    pub length: u64,
    /// CRC-32 (IEEE 802.3 polynomial) of the encoded blob bytes.
    pub crc32: u32,
}

/// Manifest body. Serialized as a `bincode 2` blob, optionally
/// zstd-compressed (see [`crate::cadk::header::CadkFlags::MANIFEST_COMPRESSED`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Manifest {
    /// Ordered list of blob records.
    pub records: Vec<BlobRecord>,
}

impl Manifest {
    /// Returns the first record whose `kind` matches, if any.
    pub fn find_first(&self, kind: BlobKind) -> Option<&BlobRecord> {
        self.records.iter().find(|r| r.kind == kind)
    }

    /// Returns the total uncompressed manifest payload size as the sum
    /// of all blob lengths. Useful for sanity-checking against
    /// [`crate::cadk::header::CadkHeader::total_size`].
    pub fn total_blob_bytes(&self) -> u64 {
        self.records.iter().map(|r| r.length).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_manifest_has_zero_total() {
        let m = Manifest::default();
        assert_eq!(m.total_blob_bytes(), 0);
        assert!(m.find_first(BlobKind::Document).is_none());
    }

    #[test]
    fn find_first_returns_first_matching_record() {
        let m = Manifest {
            records: vec![
                BlobRecord {
                    kind: BlobKind::Thumbnail,
                    name: "thumb-256".into(),
                    offset: 100,
                    length: 4096,
                    crc32: 0xDEAD_BEEF,
                },
                BlobRecord {
                    kind: BlobKind::Document,
                    name: "document".into(),
                    offset: 4196,
                    length: 1024,
                    crc32: 0xCAFE_F00D,
                },
            ],
        };
        let doc = m.find_first(BlobKind::Document).unwrap();
        assert_eq!(doc.name, "document");
        assert_eq!(m.total_blob_bytes(), 4096 + 1024);
    }

    #[test]
    fn unknown_wire_blob_kind_deserializes_to_unknown() {
        let raw = r#"{"records":[{"kind":"mesh_preview","name":"preview","offset":68,"length":0,"crc32":0}]}"#;
        let manifest: Manifest = serde_json::from_str(raw).unwrap();
        assert_eq!(manifest.records[0].kind, BlobKind::Unknown);
        assert_eq!(
            serde_json::to_string(&BlobKind::Bodies).unwrap(),
            r#""bodies""#
        );
    }
}
