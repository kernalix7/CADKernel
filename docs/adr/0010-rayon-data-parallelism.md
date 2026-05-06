# 0010. Rayon for data-parallel kernel work; no async kernel

**Status:** Accepted
**Date:** 2026-05-06
**Deciders:** kernel-engineer, tech-lead

## Context

The kernel does a lot of CPU-bound work that benefits from parallelism:

- Tessellation of N faces (independent per face).
- BVH build (parallel SAH binning).
- Boolean face-pair intersection (independent per pair).
- Sketch DR-planning (independent per sub-cluster).
- Recompute scheduler (parallel sub-DAGs).

The Rust ecosystem offers two main parallelism stories:

1. **Data parallelism**: rayon (`par_iter`, scoped threads, work-stealing pool).
2. **Async**: tokio / async-std / smol (cooperative tasks on a runtime).

## Decision

- **Kernel uses Rayon.** No async in `crates/core`, `crates/math`, `crates/geometry`, `crates/topology`, `crates/modeling`, `crates/sketch`.
- **I/O for network features** (collaboration, plugin marketplace download — Track F / E) **may use async** (tokio) inside `crates/io::cloud` and `crates/viewer::collab`, behind feature flags.
- **No kernel function returns `impl Future`.** Kernel entrypoints are sync. Async wrappers, if needed, live at the application boundary.

## Alternatives considered

| Option | Rejected because |
|---|---|
| Async kernel (tokio) | Tokio's executor is built for I/O concurrency, not CPU parallelism. Its work-stealing is general-purpose; Rayon's is tuned for compute. Async colour explosion would infect every kernel API. CPU-bound compute on a tokio runtime is an antipattern (the docs explicitly warn against it). |
| Threads + manual mpsc | Rayon's work-stealing pool is more efficient than naive thread-per-task. Manual lock-free coding is bug-prone. |
| crossbeam-channel + scoped threads | Lower-level than Rayon; we'd reinvent Rayon's join/scope/par_iter. Used **inside** Rayon for specific patterns where useful. |
| Single-threaded only | Misses 80 % of available CPU on modern machines. Tessellation budget would balloon from 200 ms to 1.5 s. |
| GPU compute via wgpu compute shaders | Considered for tessellation. Rejected v1: f64 not supported on most GPUs; transfer overhead dominates for small jobs; complexity high. May revisit for class-A surface refinement in v3+. |

## Consequences

**Positive:**
- `par_iter` makes parallelism a one-line change for embarrassingly-parallel loops.
- Scoped threads (`rayon::scope`) give safe non-`'static` parallelism (borrow stack data).
- No async colour: kernel APIs read as plain Rust functions returning `KernelResult<T>`.
- Determinism preserved: Rayon iterators with stable ordering produce deterministic results when the closure is deterministic.

**Negative:**
- Mixing Rayon with tokio at app boundary requires care (don't block tokio executor with rayon::join). Documented; we use `tokio::task::spawn_blocking` for the bridge.
- Rayon's global pool is fixed at startup; we configure to `num_cpus::get()` and don't tune per-job. Acceptable for our workloads.

**Neutral:**
- Both crates are mature, stable, widely deployed.

## References

- Rayon: <https://github.com/rayon-rs/rayon>
- "Why async Rust is hard for CPU-bound work" — Tokio docs: <https://docs.rs/tokio/latest/tokio/#cpu-bound-tasks-and-blocking-code>
- [docs/algorithms/tessellation.md](../algorithms/tessellation.md) §6
- [docs/algorithms/recompute.md](../algorithms/recompute.md) §7
