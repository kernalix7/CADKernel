# Test Corpus

**Status:** spec v1, last updated 2026-05-06
**Owner:** qa-engineer
**Implements:** roadmap §11 (verification)

---

## 1. Corpus tiers

| Tier | Purpose | Source |
|---|---|---|
| Golden | Hand-crafted; reference outputs frozen | `crates/*/tests/golden/` |
| Random | Procedurally generated; proptest | `crates/*/tests/random/` (seeded) |
| Industrial | Real CAD models | External (CAX-IF, FreeCAD examples, OpenSCAD library) |
| Regression | Historical bugs frozen as tests | `crates/*/tests/regression/` |

---

## 2. Per-domain corpus

### 2.1 Sketch (`crates/sketch/tests/corpus/`)

- 20 hand-crafted (mechanical 2D parts).
- 1000 random well-determined.
- 1000 random under-determined (DoF readout verified).
- 1000 random over-determined (deflation verified).
- 100 industrial (FreeCAD Sketcher examples).
- 10 000 proptest perturbations per release.

### 2.2 Geometry SSI (`crates/geometry/tests/ssi_corpus/`)

- 24 analytical (plane × primitives).
- 12 tangential intersections.
- 8 coincident regions.
- 1000 random Bezier-patch pairs.
- 50 STEP-imported pairs.

### 2.3 Boolean (`crates/modeling/tests/boolean_corpus/`)

- 144 primitive pairs (12 × 12 primitives) × 3 ops = 432 cases.
- 50 hand-crafted "tricky" (coplanar, tangent cylinders, T-junctions).
- 1 M random pairs (proptest).
- 290 STEP-imported parts pairwise where meaningful.

### 2.4 Fillet (`crates/modeling/tests/fillet_corpus/`)

- 30 single-edge fillets on primitives.
- 50 multi-edge sequences.
- 20 setback corners (3-edge, 4-edge vertices).
- 20 variable-radius profiles.
- 100 industrial.

### 2.5 STEP I/O (`crates/io/tests/step_corpus/`)

- 290 CAX-IF round R1-R7 parts.
- 60 CAX-IF PMI parts (R8-R12) — v3+.
- 100 FreeCAD example parts.
- 50 industrial parts (Open CASCADE samples + provided test set).

### 2.6 Visual baselines (`crates/viewer/tests/visual_baselines/`)

- 60 reference renders covering:
  - All themes × camera positions × lighting modes.
  - All HUD states (axis, grid, gizmo, ruler).
  - Selection feedback (single, multi, marquee).
  - Edit-in-place gizmos.
- Comparison via SSIM (target $> 0.95$).

---

## 3. Boolean fuzz: 12×12 → 1M cases

Procedural generation:

```text
for each (prim_a, prim_b) in [12 primitives]^2:
    for each transform_a in 100 random transforms:
        for each transform_b in 100 random transforms:
            for each op in [union, intersection, difference]:
                test_boolean(prim_a*T_a, prim_b*T_b, op)
```

Total: $12 \cdot 12 \cdot 100 \cdot 100 \cdot 3 = 4.32$ M cases (we sample 1 M for CI runtime budget, full 4.32M for nightly).

Pass criteria:
- 0 panics.
- Manifold output for non-empty results.
- Volume sign positive.
- Boolean identities respected: $A \cup B = B \cup A$, $A \cap A = A$, $A \setminus A = \emptyset$, etc.

---

## 4. Determinism enforcement

CI runs the corpus on Linux x86-64, Linux aarch64, macOS x86-64, macOS arm64, Windows x86-64. Outputs must be **byte-identical** across architectures (modulo final tessellation rasterisation, which uses SSIM tolerance).

Floating-point determinism enforced via:
- `-ffp-contract=off` flag (no FMA fusion across architectures).
- Stable iteration order (avoid `HashMap` in deterministic paths; use `BTreeMap`).
- No reliance on pointer addresses for ordering.

---

## 5. Coverage targets

| Crate | Line coverage | Branch coverage |
|---|---|---|
| `cadkernel-core` | $> 95\%$ | $> 90\%$ |
| `cadkernel-math` | $> 95\%$ | $> 90\%$ |
| `cadkernel-geometry` | $> 90\%$ | $> 85\%$ |
| `cadkernel-topology` | $> 90\%$ | $> 85\%$ |
| `cadkernel-modeling` | $> 85\%$ | $> 80\%$ |
| `cadkernel-sketch` | $> 90\%$ | $> 85\%$ |
| `cadkernel-io` | $> 80\%$ | $> 75\%$ |
| `cadkernel-viewer` | $> 70\%$ | $> 65\%$ |

Measured via `cargo llvm-cov`; reported in CI summary.

---

## 6. CI gates

| Gate | Trigger | Time budget |
|---|---|---|
| Smoke | Every PR | $< 5$ min |
| Standard | Every PR (post smoke) | $< 30$ min |
| Full corpus | Pre-merge to main | $< 2$ h |
| Nightly | Cron | $< 8$ h (includes 4.32M boolean fuzz) |

---

## 7. References

1. CAX Implementor Forum: <https://www.cax-if.org/>
2. FreeCAD test models: <https://github.com/FreeCAD/FreeCAD>
3. Open CASCADE test suite.
4. Proptest: <https://github.com/proptest-rs/proptest>
