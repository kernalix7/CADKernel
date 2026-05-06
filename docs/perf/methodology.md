# Performance Methodology

**Status:** spec v1, last updated 2026-05-06
**Owner:** qa-engineer
**Companion:** [microbenchmarks.md](../algorithms/microbenchmarks.md)

This document defines **how** we measure, not **what** we measure. Without a fixed methodology, "$< 5 \mu s$" means nothing because the same code can vary 10× between warm cache, cold cache, single thread, multi thread, debug, release, native CPU flags, and different machines.

---

## 1. Reference hardware tiers

| Tier | Spec | Purpose |
|---|---|---|
| **R1 — Reference dev** | Intel i7-12700, 32 GB DDR5-4800, NVMe SSD, Ubuntu 24.04, no power limits | Authoritative. All targets in [microbenchmarks.md](../algorithms/microbenchmarks.md) measured here. |
| **R2 — CI Linux** | GitHub Actions `ubuntu-latest` (varies; track via lscpu in artifact) | Regression detection. Targets normalised by perf-counter ratio. |
| **R3 — CI macOS** | GitHub Actions `macos-14` arm64 | Cross-architecture validation. |
| **R4 — CI Windows** | GitHub Actions `windows-latest` | Cross-OS validation. |
| **R5 — Low-end laptop** | Surface Pro 9, i5-1235U, 8 GB | Acceptance: must hit 60 fps viewer, 50 ms LM solve, on this machine. |

Targets in `microbenchmarks.md` are R1 numbers. R5 acceptance is a separate gate documented per phase exit.

---

## 2. Build profile

```toml
[profile.bench]
opt-level = 3
codegen-units = 1
lto = "fat"
panic = "abort"
debug = false
incremental = false
```

```bash
RUSTFLAGS="-C target-cpu=native -C target-feature=+fma" cargo bench --release
```

`target-cpu=native` is required for the published numbers. CI uses a uniform `target-cpu=x86-64-v3` for cross-PR comparability.

`-C codegen-units=1` and `lto = fat` are mandatory for benches; without them, results vary 5–15 % between runs.

`panic = abort` removes unwind tables; benches measure happy-path code.

---

## 3. Criterion configuration

```rust
fn config() -> Criterion {
    Criterion::default()
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(10))
        .sample_size(100)
        .significance_level(0.01)
        .noise_threshold(0.02)   // 2% — flag as regression if exceeded
}
```

- **Warm-up 2 s** ensures CPU frequency scaling has stabilised at boost.
- **Measurement 10 s** with 100 samples gives narrow confidence intervals.
- **Significance level 0.01** rejects noise; only changes that are 99 % statistically significant are flagged.
- **Noise threshold 2 %** matches the per-run jitter we measure on R1 with no other load.

---

## 4. Cold vs warm

Two suites:

- **Warm** (default): inputs constructed once, then bench iterations call the function. Measures steady-state throughput.
- **Cold**: each iteration constructs fresh inputs and clears CPU cache hint via `criterion::black_box` on a 32 MB dummy buffer between iterations. Measures first-call latency.

Names: `make_box_warm`, `make_box_cold`.

The numbers in `microbenchmarks.md` are warm. Cold numbers are tracked separately and typically 2–5× larger.

---

## 5. RNG and reproducibility

Random inputs use `rand_chacha::ChaCha8Rng::seed_from_u64(0xCADC0DEC0DECADE)` — a fixed seed. Same seed → same inputs across runs and across machines. PR descriptions can paste exact seed for reproducibility of any reported anomaly.

---

## 6. CPU isolation (R1 only)

For authoritative R1 numbers:

```bash
sudo cpupower frequency-set -g performance
sudo cset shield -c 4-7 -k on        # isolate cores 4-7
sudo cset shield -e -- nice -n -19 cargo bench --bench booleans
```

This eliminates scheduler interference. CI runners cannot do this; their numbers are normalised, not absolute.

---

## 7. Determinism enforcement

Benchmarks must produce identical outputs across runs (modulo timing). Each bench has an assertion at the end:

```rust
assert_eq!(blake3::hash(&serialize(&output)), expected_hash);
```

If the assertion fires, the bench is non-deterministic and its number is meaningless until the determinism bug is fixed.

---

## 8. Memory measurement

Separate `cargo bench --bench memory` suite using `dhat-rs` (heap profiler):

```rust
let _profiler = dhat::Profiler::builder().testing().build();
let result = expensive_op();
let stats = dhat::HeapStats::get();
println!("peak heap: {} bytes", stats.max_bytes);
```

Targets:

- Per-feature heap overhead ≤ 4 KB (industrial parts have $10^4$ features → 40 MB total).
- Peak RSS during industrial-part import ≤ 1 GB.
- Peak RSS during 1 M-triangle viewer rendering ≤ 2 GB.

Measured on R1; spot-checked on R5.

---

## 9. Frame budget breakdown (viewer)

60 fps = 16.67 ms per frame. Budget allocation:

| Component | Budget | Measured via |
|---|---|---|
| egui layout + UI draw | 2.5 ms | egui internal profiler |
| Scene update (kernel→viewer conversion) | 1.0 ms | tracing span |
| BVH update if dirty | 1.5 ms | tracing span |
| Render: solid pass (4× MSAA) | 5.0 ms | wgpu timestamp query |
| Render: wireframe pass | 2.0 ms | wgpu timestamp query |
| Render: gradient sky / overlays | 1.0 ms | wgpu timestamp query |
| GPU present + slack | 3.67 ms | swap-chain delta |

Per-frame trace logged with `tracing::span!` at TRACE level; can be viewed in `chrome://tracing` via `tracing-chrome` integration.

---

## 10. Regression policy

CI compares each PR's bench against the baseline stored in `target/criterion/baseline/`:

| Change | Action |
|---|---|
| ≤ 5 % | No action (within noise). |
| 5–15 % regression | Warn: PR comment with diff table; reviewer must acknowledge. |
| > 15 % regression | **Block merge.** Manual override requires (1) reviewer approval, (2) explanation in PR body, (3) opt-in label `perf-regression-acknowledged`. |
| ≥ 5 % improvement | Auto-update baseline (with author confirmation in PR comment). |
| ≥ 50 % change either way | Re-run 3× to confirm not transient. If consistent, treat per the table above. |

---

## 11. Profile-guided optimisation (PGO)

For release binaries, we plan PGO in v2.0:

1. Build with instrumentation: `RUSTFLAGS="-Cprofile-generate=/tmp/pgo-data"`.
2. Run a representative workload (R3 industrial part recompute + 60 s viewer interaction).
3. Build with profile data: `RUSTFLAGS="-Cprofile-use=/tmp/pgo-data/merged.profdata"`.

Expected gain: 5–10 % across geometric kernel hot paths (LLVM PGO data on similar workloads).

---

## 12. Counter-based analysis

When a regression is detected, CI auto-runs `perf stat`:

```bash
perf stat -e instructions,cycles,cache-misses,branch-misses,L1-dcache-load-misses cargo bench --bench <regressed_bench>
```

Output appended to PR as artifact. Common signals:

- IPC dropped → branch misses ↑ → inspect new branches in hot path.
- Cache misses ↑ → check struct layout changes (use `cargo asm` or `cargo bloat`).
- Instruction count ↑ but IPC unchanged → algorithmic regression (not micro-arch).

---

## 13. Flame graphs

For larger investigations, `cargo flamegraph`:

```bash
cargo flamegraph --bench <name> -- --bench
```

Output SVG attached to investigation issue. Look for:

- Tall narrow stacks → recursion or unintended deep call.
- Wide shallow stacks → loop dominating; candidate for SIMD or parallel.
- Allocator (`alloc::alloc::alloc`) prominent → memory churn; consider arena or reuse.

---

## 14. Cross-architecture parity

For each public bench in `microbenchmarks.md`, R3 (macOS arm64) and R4 (Windows x86-64) numbers are tracked. We allow ± 50 % deviation from R1 (different µarch, different memory subsystems). Outliers (> 50 % slower than R1) get an issue and an architecture-specific tuning task.

---

## 15. Public benchmark page

Per release, the `microbenchmarks.md` table is regenerated with actual R1 numbers and published to <https://cadkernel.org/perf/> (planned). Each row links to the Criterion HTML report for that bench, with full statistical detail (mean, median, std dev, samples).

---

## 16. References

- Criterion handbook: <https://bheisler.github.io/criterion.rs/book/>
- Brendan Gregg, *Systems Performance*, 2nd ed., Pearson 2020.
- Andi Kleen, "Linux Multi-core Scalability." 2009.
- Wittenbrink et al., "GPU performance counter intuition," *NVIDIA blog*.
- Rust performance book: <https://nnethercote.github.io/perf-book/>
- [docs/algorithms/microbenchmarks.md](../algorithms/microbenchmarks.md)
