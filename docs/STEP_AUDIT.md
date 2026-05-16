# STEP AP214 Fidelity Audit

## Gate #16 Scope

Gate #16 requires STEP AP214 round-trip fidelity on the R1-R3 reference parts:

- Face-area Jaccard: `>= 0.999`
- Maximum vertex delta: `<= 1e-6`

The pre-Gate #16 Track 5a baseline was `0.7273` face-area Jaccard on the R1-R3
coverage set.

## Stage 4 Remediation

Mega-Push 1 Stage 4 added R1-R3 fidelity scaffolding and improved STEP export
coverage for B-Rep faces, edge loops, and analytic surface entities. That moved
the simple primitive baseline to passing, but the command-script reference parts
still exposed a boundary reconstruction loss in boolean-derived faces.

## Final Fix

The final shortfall came from STEP import rebuilding `EDGE_LOOP` vertices with a
global de-duplication step inside each face loop. Boolean split faces may revisit
the same topological vertex in a valid boundary walk; removing the repeat changed
the polygon fan and reduced measured face area while leaving vertex positions
unchanged.

The importer now preserves the ordered vertex walk from each STEP `EDGE_LOOP`
exactly. The writer also serializes floating-point STEP reals with explicit
17-significant-digit scientific notation, covering coordinates, directions,
radii, vector magnitudes, and NURBS knot values.

## Achieved Scores

Measured with:

```bash
cargo test -p cadkernel-io --test step_r1_r2_r3_fidelity -- --include-ignored --nocapture
```

| Part | Face-area Jaccard | Face area before -> after | Max vertex delta | Vertices | Faces |
|------|-------------------|---------------------------|------------------|----------|-------|
| R1 | `1.000000000000` | `15418.233670755028 -> 15418.233670755028` | `0.000000000000e0` | `534 -> 534` | `272 -> 272` |
| R2 | `1.000000000000` | `9622.884345487751 -> 9622.884345487751` | `0.000000000000e0` | `536 -> 536` | `276 -> 276` |
| R3 | `1.000000000000` | `2829.225828347114 -> 2829.225828347114` | `0.000000000000e0` | `2176 -> 2176` | `2114 -> 2114` |

## Known Limitations

- The fidelity tests verify tessellated face area, vertex positions, vertex
  count, and face count. They do not prove full analytic surface-kind or trim
  curve preservation for every STEP entity.
- STEP import still rebuilds B-Rep topology from `ADVANCED_FACE` and
  `EDGE_LOOP` entities. NURBS surface and parametric trim preservation should
  remain a separate audit target.
