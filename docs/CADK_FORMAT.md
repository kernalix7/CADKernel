# CADK Native File Format

This document specifies the `.cadk` native binary container used by
`cadkernel-api`. Multi-byte integers are little-endian. The current writer
emits schema version 2.

## Container Layout

```text
offset  size  field
0       4     magic bytes: ASCII "CADK"
4       64    fixed header
68      N     manifest JSON blob
68+N    ...   content blobs in manifest order
```

The manifest is the table of contents. Blob offsets are absolute byte offsets
from the start of the file, not relative offsets from the end of the manifest.

## Header

The fixed header is 64 bytes immediately after the magic bytes.

```text
header offset  size  type  field
0              4     u32   schema_version
4              4     u32   flags
8              8     u64   total_size
16             8     u64   manifest_offset
24             8     u64   manifest_length
32             4     u32   manifest_crc32
36             28    u8[]  reserved, zero on write
```

`total_size` is the full file length, including magic, header, manifest, and
all content blobs. Readers reject a file whose declared size does not match the
input size.

`manifest_crc32` is CRC-32/IEEE over the encoded manifest bytes. It does not
cover content blobs; each blob has its own CRC in the manifest.

## Flags

```text
bit  value       name
0    0x00000001  MANIFEST_COMPRESSED
1    0x00000002  SIGNED
2    0x00000004  HAS_THUMBNAIL
3    0x00000008  DOCUMENT_COMPRESSED
```

The current writer never compresses the manifest. It may zstd-compress the
Document blob when `DOCUMENT_COMPRESSED` is set.

The high 16 bits are must-understand bits. A reader rejects any unknown bit in
that range. Unknown low bits are preserved as diagnostics through
`CadkSummary::unknown_flags()` and do not block manifest inspection.

## Manifest

The manifest is JSON encoded as:

```json
{
  "records": [
    {
      "kind": "document",
      "name": "document",
      "offset": 123,
      "length": 456,
      "crc32": 789
    }
  ]
}
```

`kind` uses snake_case. Known values are:

- `document`
- `thumbnail`
- `history`
- `bodies`
- `sketches`
- `attachment`
- `signature`
- `unknown`

`offset` and `length` describe the encoded on-disk blob bytes. `crc32` is
CRC-32/IEEE over those encoded bytes. For compressed document blobs this means
the CRC covers the zstd frame, not the decompressed JSON.

Known schema versions require a `document` record. Future unknown schema
versions may omit it; in that case `inspect()` still returns the manifest and
blob list, while document decode is refused.

Unknown manifest `kind` strings from future writers are reported as
`BlobKind::Unknown`; the record `name`, offsets, lengths, and CRC fields remain
available through the manifest.

## Blob Semantics

### Document

Schema v1 stores the Document blob as a JSON array of `Command` values.

Schema v2 stores the Document blob as:

```json
{
  "commands": [],
  "bodies": [],
  "sketches": []
}
```

`commands` is the canonical replay log. `bodies` stores PartDesign body
snapshots. `sketches` is reserved for persisted sketch snapshots and may be
empty.

When `DOCUMENT_COMPRESSED` is set, the Document blob is zstd-compressed. Decode
checks the blob CRC before decompression and caps decompressed output at 32 MiB.

### Thumbnail

Thumbnail blobs are raw bytes, normally PNG. They are present when the header
sets `HAS_THUMBNAIL` and the manifest includes a `thumbnail` record. A missing
thumbnail record with the flag set is treated as no thumbnail.

### Future Blobs

Readers expose unknown or unhandled blob records through `CadkSummary::blobs`
and `CadkSummary::manifest`. They do not decode future blob payloads.

## Schema Versions

### v0

The historical v0 fixture line is represented by the committed
`tests/fixtures/cadk-v0/r1_canonical.cadk` fixture. It uses the modern envelope
shape with `schema_version = 1` and a legacy command-array Document blob.

### v1

v1 is the released command-log container:

- fixed 64-byte header
- JSON manifest
- Document blob as `Vec<Command>` JSON
- optional thumbnail blob
- optional zstd-compressed Document blob

### v2

v2 keeps the same envelope and changes only the Document payload to include
explicit `commands`, `bodies`, and `sketches` sections. Missing `bodies` or
`sketches` fields default to empty arrays for compatibility.

## Migration

`migrate_to_current()` probes magic and `schema_version`, then dispatches:

- v1 -> v2: decode the legacy command array and re-encode the v2 Document
  wrapper with empty `bodies` and `sketches` unless current replay can derive
  body snapshots.
- v2 -> v2: byte-identical no-op.
- unknown future version: error. Future versions are inspect-only.

For v1 files with a compressed Document blob, migration preserves the compressed
Document state by re-encoding the v2 payload with zstd. Embedded thumbnails are
also preserved.

## Forward Compatibility

`SchemaVersion::Unknown(n)` is a read-only fence. `inspect()` accepts unknown
non-zero schema versions when the flags are compatible and the manifest is
valid. It returns:

- raw `schema_version`
- typed `schema = Unknown(n)`
- `flags`
- `blobs`
- raw `manifest`

`decode()`, `decode_document_data()`, `decode_thumbnail()`, and
`migrate_to_current()` do not decode unknown schema versions. Writers refuse to
emit `Unknown(n)`.

Schema version `0` is invalid and rejected rather than treated as a future
schema.

## Integrity Rules

Readers reject:

- bad magic
- input shorter than magic + header
- `total_size` mismatch
- manifest range outside the file
- manifest CRC mismatch
- missing Document blob for known schema versions
- Document or Thumbnail blob range outside the file when that blob is decoded
- Document or Thumbnail blob CRC mismatch when that blob is decoded
- unknown must-understand flags
- schema version `0`

`inspect()` deliberately does not validate content blob CRCs. It is a cheap
metadata path for recent-file lists, autosave scanning, and forward-compatible
blob enumeration. Use `decode()` or `decode_thumbnail()` to validate blob
payload bytes.
