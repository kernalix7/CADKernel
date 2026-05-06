# Reference Parts R1–R12

These are the canonical reference solids that the kernel must build, recompute,
and round-trip through `.cadk` and STEP without loss. Defined in roadmap §3.

Each part is built by a script (Lua or Rust) that constructs it from
primitives + features. After construction, the resulting `.cadk` file's
content hash is stored in `<id>.expected_hash`. CI verifies cross-platform
determinism: the hash must be byte-identical on Linux x86-64, Linux arm64,
macOS arm64, and Windows x86-64.

| ID | Title | Source script | Topology summary |
|---|---|---|---|
| R1 | Box (axis-aligned 100 × 50 × 25 mm) | `r1_box.lua` | 8 V, 12 E, 6 F, 1 S |
| R2 | Extruded sketch (rectangle with circular hole) | `r2_extrude.lua` | 12 V, 18 E, 8 F, 1 S |
| R3 | Revolved bottle profile | `r3_revolve.lua` | varies; smooth NURBS surface |
| R4 | Loft (square → circle) | `r4_loft.lua` | 4 NURBS surfaces, 1 cap |
| R5 | Sweep (circle along helix) | `r5_sweep.lua` | toroidal-like surface |
| R6 | Boolean: union (box ∪ cylinder) | `r6_union.lua` | mixed primitive faces |
| R7 | Boolean: difference (block − cylinder hole) | `r7_diff.lua` | block with through hole |
| R8 | Fillet: 4 edges of box at r=5 mm | `r8_fillet.lua` | 4 cylindrical blends |
| R9 | Pattern: linear array (4×3 holes) | `r9_pattern.lua` | 12 cylindrical pockets |
| R10 | Mirror: half-bracket → full bracket | `r10_mirror.lua` | symmetric topology |
| R11 | Shell: hollow box (wall thickness 2 mm) | `r11_shell.lua` | inner + outer shells |
| R12 | Multi-feature mechanical part (combo) | `r12_combo.lua` | exercise of all R1–R11 features |

## Build all reference parts

```bash
cargo run --release --example build_reference_parts -- --output tests/corpus/reference_parts/
# Produces R1.cadk through R12.cadk and corresponding .expected_hash files
```

## Verify

```bash
cargo test --release --test reference_parts_corpus
# Each part: build, hash, compare against expected_hash
# Each part: round-trip .cadk → in-memory → .cadk, verify hash unchanged
# Each part: export STEP, re-import, compare structurally
```

## Per-part timing budget (R1 hardware tier; see docs/perf/methodology.md)

| ID | Build target | `.cadk` round-trip target | STEP round-trip target |
|---|---|---|---|
| R1 | < 1 ms | < 5 ms | < 20 ms |
| R2 | < 5 ms | < 10 ms | < 50 ms |
| R3 | < 50 ms | < 100 ms | < 500 ms |
| R4 | < 100 ms | < 200 ms | < 1 s |
| R5 | < 50 ms | < 100 ms | < 500 ms |
| R6 | < 50 ms | < 100 ms | < 500 ms |
| R7 | < 50 ms | < 100 ms | < 500 ms |
| R8 | < 100 ms | < 200 ms | < 1 s |
| R9 | < 50 ms | < 100 ms | < 500 ms |
| R10 | < 50 ms | < 100 ms | < 500 ms |
| R11 | < 100 ms | < 200 ms | < 1 s |
| R12 | < 500 ms | < 1 s | < 5 s |

These budgets are enforced by Criterion benches in `crates/modeling/benches/reference_parts.rs`.
