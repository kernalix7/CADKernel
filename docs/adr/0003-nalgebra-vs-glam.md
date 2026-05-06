# 0003. nalgebra for kernel math; glam reserved for viewer hot path

**Status:** Accepted
**Date:** 2026-05-06
**Deciders:** kernel-engineer, ui-engineer

## Context

CADKernel needs linear algebra for:

- **Kernel** (`crates/math`, `crates/geometry`, `crates/modeling`): NURBS evaluation, matrix decomposition (LU/QR/SVD/Cholesky), sparse linear solve in sketch solver, transforms, ray–geometry intersection. Requires `f64`.
- **Viewer** (`crates/viewer`): per-frame vertex transforms, camera math, frustum culling. `f32` is acceptable and preferred (GPU-side is f32 anyway).

Two leading Rust libraries:

| Library | Strengths | Weaknesses |
|---|---|---|
| nalgebra 0.33 | Generic over scalar (f32/f64/Complex), full LA suite (decomp, sparse, geometric), well-tested | Slower than glam for f32 small fixed-size ops; heavier compile time |
| glam 0.29 | Optimised for f32 graphics math (SIMD-friendly small types), minimal API, fast compile | f64 support secondary; no decomposition / sparse / generic scalar |

## Decision

- **`crates/math`, `crates/geometry`, `crates/modeling`, `crates/sketch`, `crates/topology`**: use **nalgebra** with `f64`.
- **`crates/viewer`**: use **glam** with `f32` for per-frame and per-vertex transforms; convert from kernel's `nalgebra::Matrix4<f64>` to `glam::Mat4` (cast f64→f32) at the kernel↔viewer boundary.
- The boundary conversion is a single helper in `crates/viewer/src/scene/conversion.rs`.

## Alternatives considered

| Option | Rejected because |
|---|---|
| nalgebra everywhere (including viewer) | Per-vertex `f64` matrix math is 2× memory + ~30 % slower transform throughput. GPU upload still casts to f32, so the f64 precision is wasted at the viewer layer. |
| glam everywhere (including kernel) | No SVD, no sparse, no Cholesky, no rich generic. Sketch solver cannot use it. NURBS knot insertion stability suffers from f32. |
| ultraviolet | Less mature than glam; smaller community. |
| cgmath | Maintenance has slowed; nalgebra is the active leader. |
| Custom from scratch | Reinventing decomposition algorithms. Years of work. No. |

## Consequences

**Positive:**
- Kernel gets full numerical-analysis-grade LA via nalgebra (LU with partial pivoting, QR, SVD, Cholesky, sparse Cholesky via `nalgebra-sparse`).
- Viewer frame budget (16 ms at 60 fps, see [microbenchmarks.md](../algorithms/microbenchmarks.md)) is met by glam's SIMD f32.
- Clean separation of concerns: precision belongs to the kernel, throughput belongs to the viewer.

**Negative:**
- Two LA libraries in dep tree → +200 KB release binary, +6 s incremental compile.
- Boundary cast f64→f32 loses precision at viewer; acceptable because viewer is informational, not authoritative.
- Two coordinate-convention worlds (nalgebra column-major default; glam column-major; both the same in practice).

**Neutral:**
- Both libraries mature, well-documented, active.

## References

- nalgebra: <https://nalgebra.org/>
- glam: <https://docs.rs/glam/>
- Crate version pin: `Cargo.toml` `[workspace.dependencies]`.
- [docs/algorithms/sketch-solver.md](../algorithms/sketch-solver.md) §3 (sparse Cholesky use case)
- [docs/algorithms/tessellation.md](../algorithms/tessellation.md)
