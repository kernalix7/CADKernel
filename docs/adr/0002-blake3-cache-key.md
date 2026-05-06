# 0002. BLAKE3 for recompute cache keys

**Status:** Accepted
**Date:** 2026-05-06
**Deciders:** kernel-engineer, qa-engineer

## Context

The recompute scheduler ([recompute.md](../algorithms/recompute.md)) caches operation outputs keyed by a hash of `(op_kind, sorted_input_keys, canonicalised_parameter_map)`. The hash must:

- Have **negligible collision probability** across $10^9$ entries (industrial PLM scale).
- Be **fast** — recompute checks may evaluate $10^4$ keys per second during interactive editing.
- Be **deterministic across architectures** (Linux x86-64, macOS arm64, Windows x86-64).
- Be **streaming-friendly** — large parameter maps accumulate incrementally.
- Be **available as a small Rust crate** with no C deps, MIT/Apache.

## Decision

Use **BLAKE3** (256-bit output, truncated to 128 bits for cache keys, full 256 bits for `.cadk` content hash).

Crate: `blake3 = "1"` (workspace dep).

## Alternatives considered

| Option | Rejected because |
|---|---|
| SHA-256 | ~5–10× slower than BLAKE3 at 32-byte input on modern CPUs. Designed for cryptographic strength we don't need beyond collision resistance. |
| SHA-3 (Keccak) | Even slower than SHA-256 in software. NIST-standard but no advantage for our use case. |
| xxHash3 | Non-cryptographic; **adversarial collisions trivial.** Cache poisoning risk if a plugin can craft inputs. |
| FNV / CityHash / MurmurHash | Same as xxHash3 — non-cryptographic. Higher collision rate even on benign input. |
| MD5 | Broken (collisions demonstrated). Same speed as SHA-256 anyway. |
| Blake2b | Predecessor of BLAKE3; slower (~3× at 32 B). BLAKE3 supersedes it. |

## Consequences

**Positive:**
- ~3 GB/s on a single i7-12700 core (BLAKE3 official benchmarks).
- Built-in parallelism via Bao tree for files > 16 KB (used for `.cadk` content hash).
- Collision probability for 128-bit truncation: $2^{-64}$ per pair, $\binom{10^9}{2} \cdot 2^{-128} \approx 1.5 \cdot 10^{-21}$ across $10^9$ entries — **astronomically safe**.
- Cryptographically secure: a malicious plugin cannot craft input collisions (unlike non-crypto hashes).
- Pure Rust (`blake3` crate has SIMD intrinsics via portable + arch-specific code paths; no C build dep on default profile).

**Negative:**
- Crate is ~12 K SLOC; adds ~400 KB to release binary. Acceptable.
- Not a NIST-standardised hash. Not a problem for cache keys; we use SHA-256 separately for any external compliance signing (cosign, etc.).

**Neutral:**
- Output size 32 bytes (or 16 truncated) — same order as alternatives.

## References

- BLAKE3 spec: <https://github.com/BLAKE3-team/BLAKE3-specs>
- Aumasson et al., "BLAKE3: one function, fast everywhere," 2020.
- Crate: <https://crates.io/crates/blake3>
- Bernstein collision-probability calculator notes.
- [docs/algorithms/recompute.md](../algorithms/recompute.md) §4
- [docs/algorithms/brep-abi.md](../algorithms/brep-abi.md) §13 (TRAILER content_hash)
