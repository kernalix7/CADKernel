# 0013. CBOR (RFC 8949) for embedded metadata

**Status:** Accepted
**Date:** 2026-05-06
**Deciders:** io-engineer

## Context

The `.cadk` file format ([brep-abi.md](../algorithms/brep-abi.md)) has a METADATA section storing:

- Application name + version.
- Creation / modification timestamps.
- Author, organisation, comments.
- User-defined key-value properties.
- Localisation strings.
- Plugin-defined extensions (each plugin has its own sub-namespace).

Requirements:
- Self-describing (no external schema).
- Compact binary.
- Streaming-friendly.
- Strong, well-documented spec.
- Cross-language libraries available.

## Decision

Use **CBOR (Concise Binary Object Representation, RFC 8949)** for the METADATA section payload.

Rust crate: `cbor4ii 0.3` (zero-copy decode where possible, no_std friendly, no panics on untrusted input).

## Alternatives considered

| Option | Rejected because |
|---|---|
| JSON | Text-based; 2-3× the bytes for the same data; floating-point round-trip ambiguous; lacks binary-blob type (must base64-wrap). Acceptable for human inspection (provided as side-export); not for embedded metadata. |
| MessagePack | Comparable to CBOR but smaller spec ecosystem; CBOR is an IETF standard with multiple compliance suites. CBOR was a close call; tie-broken by IETF status. |
| BSON | MongoDB-specific lineage; less general. |
| Protobuf | Requires schema; too rigid for user-defined and plugin-defined keys. |
| FlatBuffers / Cap'n Proto | Same — schema required. |
| YAML | Text; complex spec; security incidents historically (billion-laughs etc.). No. |
| TOML | Text; designed for config, not arbitrary structured data. |
| Custom binary | We'd reinvent CBOR. No. |

## Consequences

**Positive:**
- Self-describing tags (RFC 8949 §3.4): timestamps, big numbers, decimals, URIs all have well-defined CBOR tags.
- Deterministic encoding rule available (RFC 8949 §4.2) — important for reproducible `.cadk` content hashes.
- Compact: typical METADATA payload < 4 KB.
- Multiple language libraries: Rust (cbor4ii, ciborium), Python (cbor2), JavaScript (cbor-x), Go, etc. — useful for tooling and external readers.
- Streaming decode supported.

**Negative:**
- Less human-readable than JSON; debug tooling needs `cbor-diag` or similar. Mitigated by `cadkernel inspect <file>` CLI that decodes METADATA to JSON for viewing.
- CBOR's optionality (multiple valid encodings of the same value) means we must pin to canonical encoding rules for hash determinism. Spec'd in [brep-abi.md §3](../algorithms/brep-abi.md).

**Neutral:**
- IETF-standard; long-term stability assured.

## References

- RFC 8949: Concise Binary Object Representation: <https://www.rfc-editor.org/rfc/rfc8949.html>
- CBOR overview: <https://cbor.io/>
- cbor4ii: <https://github.com/quininer/cbor4ii>
- [docs/algorithms/brep-abi.md](../algorithms/brep-abi.md)
