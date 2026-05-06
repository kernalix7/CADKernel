//! `.cadk` — CADKernel's native binary container format.
//!
//! A3 scaffold (2026-05-07). Magic bytes, header, manifest, and blob
//! envelope types live here. The full encoder / decoder lands in
//! follow-up patches. **No file produced by this scaffold should be
//! considered stable** until the v1.0 release calendar reaches A3 closure.
//!
//! Layout (binary, little-endian):
//!
//! ```text
//! +----------------+----------------+----------------------+----------------+
//! | magic[4]       | header[64]     | manifest_blob        | content_blobs  |
//! | "CADK"         | (versioning,   | (toc, hashes, sizes) | (zstd-frames)  |
//! |                |  flags, crc32) |                      |                |
//! +----------------+----------------+----------------------+----------------+
//! ```

pub mod header;
pub mod manifest;

pub use header::{CadkFlags, CadkHeader, MAGIC};
pub use manifest::{BlobKind, BlobRecord, Manifest};
