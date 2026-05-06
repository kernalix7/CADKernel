# 0009. Custom binary `.cadk` over JSON / msgpack / sqlite

**Status:** Accepted
**Date:** 2026-05-06
**Deciders:** io-engineer, kernel-engineer

## Context

CADKernel needs a native document format. Requirements:

- Lossless f64 round-trip.
- Streaming-friendly (sections independently readable / writable).
- Compact (industrial parts can have $10^6$ topology records; bloat hurts disk and network).
- Versionable with backward / forward compatibility.
- Inspectable (developers should be able to `xxd` the header).
- Stable across architectures.
- Embeddable (thumbnail PNG, metadata, plugin extensions).

## Decision

Define a custom binary container format `.cadk` with named sections (HEADER / METADATA / DOCUMENT / TOPO / GEOM / TAGS / HIST / THUMB / EXT / TRAILER) with section-level zstd compression and a CRC trailer. Specification: [brep-abi.md](../algorithms/brep-abi.md).

## Alternatives considered

| Option | Rejected because |
|---|---|
| JSON | Bloated (300 % vs binary), f64 precision lost in text serialisation (ambiguous "0.1" vs binary), parse cost dominates for large parts. Acceptable as **sidecar** debug export only. |
| MessagePack / CBOR top-level | Better than JSON but still allocates a single root structure; no section-level streaming. Used **inside** `.cadk` METADATA section because it fits there. |
| SQLite | Excellent for transactional state, overkill for documents. Adds 1 MB native dep. Random-access strength irrelevant for our load-once / save-once usage. |
| Apache Arrow / Parquet | Designed for tabular analytics; B-Rep is graph-shaped, not tabular. |
| HDF5 | Native dep + complexity; designed for scientific datasets, not graph topology. |
| Protobuf / FlatBuffers / Cap'n Proto | Schema-driven binary; great for RPC. For documents, version evolution via schemas is more rigid than our section-tagged design. Considered seriously; rejected because section-level compression and inspector friendliness mattered more. |
| OpenCascade BinXCAFFormat | Proprietary OCCT format; not portable to non-OCCT backends. |
| STEP as native | STEP is interchange, not native. Verbose ASCII; loses `Tag` and history without proprietary extensions. STEP is our **export**, not our save format. |

## Consequences

**Positive:**
- Section-level compression (zstd) → typical industrial part `.cadk` is 30–60 % the size of the equivalent OpenCascade `.brep`.
- Backward compatibility: v1 readers skip unknown sections (no crash).
- Forward compatibility: v2 readers run a migration shim ([brep-abi.md §15](../algorithms/brep-abi.md)).
- Header is human-readable in `xxd` (helpful for debugging corrupted files).
- Plugins can add EXT sections without touching core.

**Negative:**
- We own the format; we maintain the spec and migration shims forever.
- No off-the-shelf viewers; we ship our own `.cadk` inspector tool.
- More code to write than "just use serde to JSON". Worth it for the precision and compactness.

**Neutral:**
- Dependencies: `zstd 0.13`, `cbor4ii 0.3`, `blake3 1`, `byteorder 1`. Small, stable.

## References

- [docs/algorithms/brep-abi.md](../algorithms/brep-abi.md)
- zstd RFC 8878.
- CBOR RFC 8949.
- ISO 10303-21 (STEP) for comparison.
- OpenCascade BinXCAFFormat documentation.
