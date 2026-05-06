# Test Corpus

This directory holds **golden** test inputs and expected outputs. Goldens are regenerated only by an authorised maintainer with explicit reason; CI verifies that current output matches the goldens bit-for-bit (or via SSIM for rendered images).

## Layout

```
tests/corpus/
├── README.md                            # this file
├── reference_parts/                     # R1-R12 from roadmap §3
│   ├── R1_box.cadk.expected_hash
│   ├── R2_extruded_sketch.cadk.expected_hash
│   └── ...
├── sketch/
│   ├── golden/                          # hand-crafted reference sketches
│   │   ├── 001_rectangle.toml
│   │   ├── 002_concentric_circles.toml
│   │   └── ...
│   ├── random_well/                     # generator seeds, not files
│   │   └── seeds.txt                    # ChaCha8 seeds
│   ├── random_under/
│   ├── random_over/
│   ├── industrial/                      # scraped from FreeCAD examples
│   └── regression/                      # historical bugs frozen
│       └── ISSUE_NNNN.toml
├── boolean/
│   ├── primitives/                      # 144 primitive pairs
│   ├── tricky/                          # 50 hand-crafted edge cases
│   ├── random/                          # generator seeds
│   └── regression/
├── ssi/
│   ├── analytical/                      # 24 closed-form-comparable cases
│   ├── tangent/                         # 12 tangential cases
│   ├── coincident/                      # 8 coincident-region cases
│   └── industrial/                      # STEP-imported pairs
├── step/
│   ├── cax_if/                          # CAX-IF rounds R1-R7 (R8-R12 v3+)
│   ├── freecad/                         # FreeCAD example STEP files
│   └── round_trip_hashes.txt
└── visual_baselines/                    # 60 reference renders for SSIM
    ├── light_iso_blinn.png
    ├── dark_iso_blinn.png
    └── ...
```

## File formats

Goldens use TOML for sketches and boolean inputs (human-readable, diffable, no binary noise). Expected outputs are the BLAKE3 content hash of the canonical `.cadk` representation, stored in companion `.expected_hash` files.

For visual baselines, PNG (lossless) is the storage format; comparison uses SSIM (structural similarity index) with target $\geq 0.95$ between current render and baseline.

## Regeneration

To regenerate goldens (rare, requires reason):

```bash
cargo test --workspace --features regenerate-goldens
git diff tests/corpus/    # review changes
git add tests/corpus/ && git commit -m "test: regenerate goldens (reason: ...)"
```

CI's nightly job verifies that goldens are deterministic across all 5 reference architectures (Linux x86-64 / Linux arm64 / macOS arm64 / Windows x86-64 / Windows arm64).

## Coverage gates

Per release, all goldens must pass:

| Tier | Pass rate required |
|---|---|
| Reference parts (R1-R12) | 100 % |
| Sketch golden | 100 % |
| Boolean primitives | 100 % |
| SSI analytical | 100 % |
| Random | ≥ 99 % |
| Industrial | ≥ 99 % |
| Visual baselines | SSIM ≥ 0.95 (median), ≥ 0.90 (worst) |

Any gate failure blocks merge to main.

## Adding a regression test

When a bug is fixed, a regression test is **required** in the corresponding `regression/` directory:

```bash
# Save the failing input as TOML
cargo run --example save_toml -- --output tests/corpus/sketch/regression/ISSUE_0042.toml < failing_sketch.json

# Add the issue number to the regression manifest
echo "ISSUE_0042 | constraint X with input Y previously panicked" >> tests/corpus/sketch/regression/MANIFEST.md

# Verify the test now passes
cargo test --test regression_corpus -- ISSUE_0042
```

## See also

- [docs/algorithms/test-corpus.md](../../docs/algorithms/test-corpus.md) — corpus design
- [docs/algorithms/microbenchmarks.md](../../docs/algorithms/microbenchmarks.md) — perf corpus
- [docs/perf/methodology.md](../../docs/perf/methodology.md) — measurement methodology
