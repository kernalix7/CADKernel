# Memory Profile

**Status:** spec v1, last updated 2026-05-06
**Owner:** qa-engineer
**Companion:** [methodology.md](methodology.md)

This document defines the **memory contract** of CADKernel: how much heap each subsystem uses for representative workloads, and how that is measured.

---

## 1. Per-entity heap cost

| Entity | Bytes (approx) | Source |
|---|---|---|
| `Vertex` | 32 (Point3 24 + Tag 8) | `crates/topology/src/vertex.rs` |
| `HalfEdge` | 64 (4 × Handle u64 + flags) | `crates/topology/src/half_edge.rs` |
| `Edge` | 48 (2 × Handle + curve binding 24 + Tag 8) | `crates/topology/src/edge.rs` |
| `Loop` | 24 (1 × Handle to first half-edge + count + flags) | `crates/topology/src/loop.rs` |
| `Face` | 80 (loop list 24 + surface binding 24 + normal-cache 24 + Tag 8) | `crates/topology/src/face.rs` |
| `Shell` | 32 (face Vec + flags) | `crates/topology/src/shell.rs` |
| `Solid` | 48 (shell Vec + bbox cache 24) | `crates/topology/src/solid.rs` |
| NURBS curve (degree 3, 10 CP) | 320 (10 × Vec3 + knot Vec) | `crates/geometry/src/nurbs/curve.rs` |
| NURBS surface (degree 3×3, 6×6 CP) | 1,728 (36 × Vec3 + 2 knot Vecs) | `crates/geometry/src/nurbs/surface.rs` |
| BVH leaf (4 prims) | 96 (AABB 48 + 4 × handle 32 + flags) | `crates/geometry/src/bvh.rs` |
| BVH internal node | 80 (AABB 48 + 2 × child idx 16 + split-axis + count) | `crates/geometry/src/bvh.rs` |
| Tessellation triangle | 36 (3 × index u32 + face id u32 + smooth-group u8) | `crates/geometry/src/tessellation/mod.rs` |
| Tessellation vertex (full) | 32 (pos 12 + normal 12 + uv 8) | `crates/geometry/src/tessellation/mod.rs` |
| `OperationId` history record | 64 (op-kind discriminant + params + parent IDs) | `crates/modeling/src/history.rs` |

---

## 2. Industrial-part budget (R12 stress part)

R12 is the 10k-feature monolithic stress model from the reference part suite ([reference_parts/MANIFEST.md](../../tests/corpus/reference_parts/MANIFEST.md)).

Approximate counts:
- 100 K vertices, 300 K half-edges, 150 K edges, 30 K faces, 10 K features.

Memory budget:

| Component | Estimated bytes | Justification |
|---|---|---|
| Topology | 100K × 32 + 300K × 64 + 150K × 48 + 30K × 80 + … ≈ 32 MB | per-entity table above |
| Geometry bindings | 30K faces × ~600 B avg surface (mix of plane + NURBS) ≈ 18 MB | NURBS dominates |
| BVH (faces) | ~50 K nodes × 80 ≈ 4 MB | depth ~17 |
| Tessellation cache (high LOD) | ~3 M triangles × 36 + 1 M verts × 32 ≈ 140 MB | dominates! |
| History | 10K ops × 64 ≈ 640 KB | small |
| Recompute cache | bounded by config; default 256 MB cap | LRU evict |
| Misc / overhead / fragmentation | ~50 MB | est. 15% |
| **Total target** | **≤ 1 GB peak RSS** | with 256 MB recompute cache |

Tessellation is the single largest cost. Mitigation: tessellation cache is LOD-keyed; viewer requests only visible-LOD triangles; off-screen faces drop to coarse LOD.

---

## 3. Measurement

```rust
// crates/modeling/benches/memory_industrial.rs
use dhat::{Profiler, HeapStats};

fn main() {
    let _profiler = Profiler::builder().testing().build();
    let part = build_r12();
    let stats_after_build = HeapStats::get();

    tessellate_high_lod(&part);
    let stats_after_tess = HeapStats::get();

    println!("R12 build peak: {} MB", stats_after_build.max_bytes / 1_048_576);
    println!("R12 tess peak:  {} MB", stats_after_tess.max_bytes  / 1_048_576);

    assert!(stats_after_tess.max_bytes <= 1_073_741_824, "R12 exceeded 1 GB target");
}
```

Run: `cargo bench --bench memory_industrial`.

---

## 4. Per-OS RSS measurement

dhat measures **logical heap** (allocator-tracked). Actual RSS includes:

- Stack (per-thread; default 8 MB on Linux).
- Code / read-only data (~30 MB binary).
- Allocator metadata + fragmentation (~10-20 % overhead with default allocator).
- GPU staging buffers (wgpu; ~50 MB typical).

Total RSS budget: **logical heap + 200 MB headroom**.

OS measurement:
- Linux: `getrusage(RUSAGE_SELF, ru_maxrss)` (returns KB on Linux despite the man page saying bytes).
- macOS: same call returns bytes.
- Windows: `GetProcessMemoryInfo`.

Cross-platform helper in `crates/core/src/mem.rs::peak_rss_bytes()`.

---

## 5. Allocator choice

- **Default**: system allocator (jemalloc-rs reserved for opt-in via `--features jemalloc`).
- **Rationale**: system allocator (glibc malloc, macOS libsystem, Windows HeapAlloc) is good enough for our workload sizes; jemalloc adds ~1 MB binary and licence considerations on some platforms.
- **Profile-time**: use `mimalloc-rs` for lower allocator-internal noise during benchmarking.

---

## 6. Allocation hotspots policy

If a flame graph shows allocator > 5 % of total CPU in a hot path, the path is added to the "must use arena / pool" list:

- Tessellation triangle buffers → `bumpalo` arena per face.
- Boolean intermediate edge lists → `Vec` reused via `clear()` (capacity retained).
- BVH build node array → preallocate by upper-bound count.

These patterns are documented in [docs/algorithms/](../algorithms/) per algorithm.

---

## 7. Memory regression CI

Same pattern as time regression ([methodology.md §10](methodology.md)):

| Change | Action |
|---|---|
| ≤ 5 % | No action. |
| 5-15 % | Warn. |
| > 15 % | Block merge. |
| > 50 % | Re-run; treat as block-merge with required investigation. |

Memory baselines stored in `target/dhat/baseline.json` per benchmark.

---

## 8. References

- dhat-rs: <https://docs.rs/dhat/>
- Massif (Valgrind): <https://valgrind.org/docs/manual/ms-manual.html>
- jemalloc: <http://jemalloc.net/>
- mimalloc: <https://github.com/microsoft/mimalloc>
- Rust performance book, Memory chapter: <https://nnethercote.github.io/perf-book/heap-allocations.html>
- [docs/perf/methodology.md](methodology.md)
- [docs/algorithms/microbenchmarks.md](../algorithms/microbenchmarks.md)
