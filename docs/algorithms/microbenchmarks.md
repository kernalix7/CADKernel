# Microbenchmarks

**Status:** spec v1, last updated 2026-05-06
**Owner:** qa-engineer
**Implements:** roadmap §11 (verification), §12 (performance)
**Tool:** Criterion (`cadkernel-modeling/benches/`)

---

## 1. Benchmark catalogue

| Benchmark | Target on reference HW (i7-12700, 32 GB) | Crate |
|---|---|---|
| `make_box` | $< 1 \mu s$ | modeling |
| `make_cylinder (32 seg)` | $< 5 \mu s$ | modeling |
| `make_cylinder (64 seg)` | $< 10 \mu s$ | modeling |
| `make_sphere (16x8)` | $< 8 \mu s$ | modeling |
| `make_sphere (32x16)` | $< 25 \mu s$ | modeling |
| `extrude (square profile)` | $< 50 \mu s$ | modeling |
| `boolean_union (box+box)` | $< 1$ ms | modeling |
| `boolean_difference (box-box)` | $< 1$ ms | modeling |
| `boolean_union (cyl + cyl)` | $< 5$ ms | modeling |
| `tessellate_box` | $< 50 \mu s$ | geometry |
| `tessellate_sphere (32x16)` | $< 200 \mu s$ | geometry |
| `tessellate_part (50 faces)` | $< 5$ ms | geometry |
| `mass_properties (box mesh)` | $< 100 \mu s$ | modeling |
| `mass_properties (sphere mesh)` | $< 500 \mu s$ | modeling |
| `stl_write_ascii (sphere)` | $< 2$ ms | io |
| `stl_write_binary (sphere)` | $< 500 \mu s$ | io |
| `step_write_box` | $< 5$ ms | io |
| `step_read_industrial_part` | $< 100$ ms | io |
| `bvh_build (1M tris)` | $< 200$ ms | geometry |
| `bvh_query (1M tris)` | $< 4$ ms | geometry |

---

## 2. Reference hardware

- CPU: Intel Core i7-12700 (5.0 GHz boost, 12 cores).
- RAM: 32 GB DDR5-4800.
- OS: Ubuntu 24.04 (CI), macOS 14 (CI), Windows 11 (CI).
- Rust: 1.85 (workspace MSRV).
- Build: `cargo bench --release` with `RUSTFLAGS="-C target-cpu=native"`.

CI runners may be slower; targets are normalised by perf-counter ratio.

---

## 3. Regression detection

Criterion baseline stored in `target/criterion/baseline/`. CI compares each PR's bench against baseline:

- Pass: change within $\pm 5\%$.
- Warn: regression $5–15\%$ (PR comment + author review required).
- Fail: regression $> 15\%$ (PR blocked; manual override + comment required).

Improvements over baseline auto-update the baseline (with author confirmation).

---

## 4. Profiling integration

When a regression is detected, CI auto-runs:

- `perf record --call-graph dwarf` (Linux) — flame graph generated.
- Counter analysis: instructions, cycles, cache-misses, branch-mispredicts.
- Output uploaded as PR artifact for review.

---

## 5. Memory bench

Separate suite (`cargo bench --bench memory`):

| Bench | Target |
|---|---|
| Peak RSS (industrial part import) | $< 1$ GB |
| Peak RSS (1 M-triangle viewer) | $< 2$ GB |
| Per-feature heap overhead | $< 4$ KB |

Measured via `jemalloc-ctl` stats.

---

## 6. References

1. Criterion docs: <https://bheisler.github.io/criterion.rs/book/>
2. Brendan Gregg, *Systems Performance*, 2nd ed., Pearson, 2020.
