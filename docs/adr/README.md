# Architecture Decision Records

This directory holds **ADRs** — short, dated, immutable records explaining *why* a non-obvious technical decision was made. The format follows Michael Nygard's original ADR proposal (2011) with the Status / Context / Decision / Consequences template.

## Why ADRs?

Roadmaps say *what* will be built. Specs (`docs/algorithms/`) say *how* it will be built. ADRs say **why** the *what* and *how* are not something else. Without ADRs, the next maintainer reads the code and asks "why didn't they just use $X$ instead?" and has to either guess or rebuild the trade-off analysis from scratch.

## Authoring rules

1. **One decision per file.** No omnibus ADRs.
2. **Numbered, sequential.** Once a number is assigned, never reuse it.
3. **Immutable once accepted.** If a decision is reversed, write a new ADR that supersedes it; never edit the old one.
4. **Status field** is one of: `Proposed`, `Accepted`, `Deprecated`, `Superseded by NNNN`.
5. **Date stamped** in ISO format.
6. **Trade-off table required** — list at least 2 alternatives that were rejected with the *specific* reason each was rejected.
7. **Concrete metrics where possible.** "Faster" is not a reason. "30 % less heap allocation in $X$ benchmark" is.

## Index

| # | Title | Status |
|---|---|---|
| [0001](0001-half-edge-vs-winged-edge.md) | B-Rep topology: half-edge data structure | Accepted |
| [0002](0002-blake3-cache-key.md) | BLAKE3 for recompute cache keys | Accepted |
| [0003](0003-nalgebra-vs-glam.md) | nalgebra for kernel math, glam reserved for viewer | Accepted |
| [0004](0004-egui-wgpu-stack.md) | egui + wgpu desktop GUI stack | Accepted |
| [0005](0005-mlua-scripting.md) | mlua (Lua 5.4 vendored) for in-app scripting | Accepted |
| [0006](0006-pyo3-bindings.md) | PyO3 Python bindings, separate crate, excluded from default build | Accepted |
| [0007](0007-tag-based-persistent-naming.md) | Tag-based persistent naming over geometric matching | Accepted |
| [0008](0008-rust-edition-2024-msrv.md) | Rust edition 2024, MSRV 1.85 | Accepted |
| [0009](0009-cadk-binary-format-vs-json.md) | Custom binary `.cadk` over JSON / msgpack / sqlite | Accepted |
| [0010](0010-rayon-data-parallelism.md) | Rayon for data-parallel kernel work; no async kernel | Accepted |
| [0011](0011-automerge-crdt.md) | automerge CRDT for real-time collaboration | Accepted |
| [0012](0012-wasm-plugin-sandbox.md) | Wasm-first plugin sandbox via wasmtime | Accepted |
| [0013](0013-cbor-metadata.md) | CBOR (RFC 8949) for embedded metadata | Accepted |
| [0014](0014-zstd-section-compression.md) | zstd (RFC 8878) for `.cadk` section compression | Accepted |
| [0015](0015-sparse-cholesky-sketch.md) | Sparse Cholesky for sketch solver linear step | Accepted |

## Format

Each ADR uses this template:

```markdown
# NNNN. Short imperative title

**Status:** Accepted
**Date:** YYYY-MM-DD
**Deciders:** role list (e.g., kernel-engineer, tech-lead)

## Context
What problem are we solving? What constraints apply?

## Decision
What did we decide? State it imperatively.

## Alternatives considered
| Option | Rejected because |
|---|---|

## Consequences
Positive, negative, neutral. Be concrete with metrics.

## References
Links to specs, papers, prior art.
```
