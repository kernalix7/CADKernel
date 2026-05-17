# NIST CAx-IF Round-Trip Audit

## Gate Requirement

v1.0 Gate #17 requires a STEP corpus check that loads representative NIST CAx-IF-style files, re-exports them, re-imports the result, and verifies B-Rep geometric fidelity for part and assembly cases.

The automated gate lives in `crates/io/tests/nist_cax_if_corpus.rs`. Each fixture is read from `crates/io/tests/fixtures/nist/`, imported with `import_step`, exported with `export_step`, re-imported, and checked against:

- face-area Jaccard >= 0.999
- nearest-vertex delta <= 1e-6
- unchanged imported vertex count
- non-empty imported source solids

## Fixture Policy

Official CAx-IF exchange-round files are not bundled in this repository. This lane uses license-clear public corpus files already present in the project and exposes them through a dedicated `fixtures/nist/` corpus. The selected files cover the same gate concerns: AP203/AP214 B-Rep solids, planar/cylindrical/conical/toroidal surfaces, B-spline trims, trimmed NURBS surfaces, and assemblies with placements.

Every fixture in `fixtures/nist/` is a symlink to the canonical public-corpus file under `fixtures/step/`, with a sibling `.md` file documenting source and license.

## Fixture Results

Measured on 2026-05-17 with `cargo test -p cadkernel-io --test nist_cax_if_corpus -- --nocapture`.

| Fixture | Source | License | Coverage | Jaccard | Vertex delta | Vertices | Faces | Solids | Result |
|---|---|---|---|---:|---:|---:|---:|---:|---|
| `caxif_planar_cylindrical_ap203.step` | NIST EDM `design2.step` | Public domain | AP203, planar + cylindrical B-Rep | 1.000000000000 | 0.0 | 60 -> 60 | 32 -> 32 | 1 -> 1 | Pass |
| `caxif_interacting_pockets_ap203.stp` | NIST EDM `interactingPockets.stp` | Public domain | AP203, pocket interactions | 1.000000000000 | 0.0 | 84 -> 84 | 48 -> 48 | 1 -> 1 | Pass |
| `caxif_conical_clevis_ap203.stp` | NIST EDM `clevis21.stp` | Public domain | AP203, conical + cylindrical faces | 1.000000000000 | 0.0 | 62 -> 62 | 18 -> 18 | 1 -> 1 | Pass |
| `caxif_bspline_bracket_ap203.stp` | NIST EDM `bracket1-Part.stp` | Public domain | AP203, B-spline curve trims | 1.000000000000 | 0.0 | 91 -> 91 | 43 -> 43 | 1 -> 1 | Pass |
| `caxif_assembly_ap203.stp` | NIST EDM `as1_pe.stp` | Public domain | AP203 assembly placements | 1.000000000000 | 0.0 | 84 -> 84 | 49 -> 49 | 5 -> 1 | Pass geometry |
| `caxif_trimmed_nurbs_assembly_ap214.stp` | STEPcode `as1-oc-214.stp` | BSD-3-Clause | AP214, B-spline surfaces + assembly | 1.000000000000 | 0.0 | 84 -> 84 | 49 -> 49 | 5 -> 1 | Pass geometry |
| `caxif_toroidal_ap214.stp` | STEPcode `io1-cm-214.stp` | BSD-3-Clause | AP214, toroidal + cylindrical faces | 1.000000000000 | 0.0 | 46 -> 46 | 29 -> 29 | 1 -> 1 | Pass |
| `caxif_vendor_cone_ap214.stp` | STEPcode `sg1-c5-214.stp` | BSD-3-Clause | AP214, vendor conical/cylindrical faces | 1.000000000000 | 0.0 | 20 -> 20 | 16 -> 16 | 1 -> 1 | Pass |

To regenerate per-fixture metric output:

```bash
cargo test -p cadkernel-io --test nist_cax_if_corpus -- --nocapture
```

## Known Gaps

- Official CAx-IF exchange-round files still need legal review before direct redistribution.
- AP242 PMI, GD&T semantic validation, and presentation validation remain deferred to v2.0.
- The current STEP round-trip gate validates geometric B-Rep fidelity. It does not yet assert preservation of the full STEP product-structure graph after export.
- Assembly fixtures currently retain exact vertices/faces/area, but `export_step` flattens imported multi-solid STEP assemblies into one exported `MANIFOLD_SOLID_BREP`. Full assembly product-structure export remains a v2.0 gap.

## Verification

```bash
cargo test -p cadkernel-io --test nist_cax_if_corpus
cargo build --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```
