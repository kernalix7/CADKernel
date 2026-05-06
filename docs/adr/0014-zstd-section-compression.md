# 0014. zstd (RFC 8878) for section compression

**Status:** Accepted
**Date:** 2026-05-06
**Deciders:** io-engineer

## Context

`.cadk` sections (TOPO, GEOM, HIST, EXT) compress well — typical ratios 3-7× for industrial parts. We need a compression algorithm that is:

- Lossless.
- Fast at decompression (load-time matters more than save-time).
- Reasonable compression ratio.
- Streaming-friendly.
- Stable, audited, no known security issues.
- Permissively licensed.

## Decision

Use **zstd (RFC 8878)** with default level 3 (configurable; level 19 for archive).

Rust crate: `zstd 0.13` (bindings to upstream C library; pure-Rust `ruzstd` reserved for no_std builds).

Per-section compression: each major section header records its uncompressed size + compression algorithm + compressed size, allowing parallel section decompression and skipping unwanted sections without decompressing them.

## Alternatives considered

| Option | Rejected because |
|---|---|
| gzip / DEFLATE | Older; 30-50 % worse compression ratio at comparable speeds. Decompression slower than zstd. The default in many older formats; we are not constrained by legacy. |
| xz / LZMA | Best compression ratio (10-20 % better than zstd-19) but **decompression is 5-10× slower**. We optimise for load time. |
| brotli | Designed for text/web; comparable to zstd on text but worse on binary. Asymmetric — slow compress, fast decompress. Reasonable runner-up; tie-broken by zstd's better Rust ecosystem. |
| LZ4 | Ultra-fast but compression ratio 30-50 % worse than zstd. Used for in-memory snapshot caching, not on-disk archive. |
| snappy | Same trade-off as LZ4. |
| No compression | Industrial parts: 50 MB uncompressed → 8 MB compressed. 6× cost on disk for zero benefit. |
| Custom (e.g., delta-encoding topology IDs first then zstd) | We do exactly this: TOPO/GEOM payloads are delta-encoded before zstd, see [brep-abi.md §6.2](../algorithms/brep-abi.md). The compression stage is still zstd. |

## Consequences

**Positive:**
- Modern compression: ~3.5× ratio at level 3, ~5× at level 19, comparable to xz at much higher decompression speed.
- Pre-trained dictionaries supported (we plan a per-domain dictionary in v2 for sketch / boolean / NURBS payloads — projected +15 % ratio).
- IETF-standardised (RFC 8878 since 2021); long-term stability.
- Streaming API; can decompress section by section without loading all data.
- Wide library support: native libraries for every relevant language.

**Negative:**
- C dependency in default build path. Mitigated by `ruzstd` Rust-only fallback for no_std / wasm builds.
- Adds ~600 KB to release binary (zstd library). Acceptable.
- Compression-level choice is a trade-off (size vs time); we expose `--compression-level` flag for users who care.

**Neutral:**
- Default level 3 picks ~3× ratio at ~250 MB/s — good general-purpose balance.

## References

- RFC 8878: zstd: <https://www.rfc-editor.org/rfc/rfc8878>
- Facebook zstd: <https://github.com/facebook/zstd>
- Compression benchmark: <https://github.com/inikep/lzbench>
- Rust crate: <https://crates.io/crates/zstd>
- [docs/algorithms/brep-abi.md](../algorithms/brep-abi.md)
