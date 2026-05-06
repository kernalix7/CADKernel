# Boolean Engine

**Status:** spec v1, last updated 2026-05-06
**Owner:** kernel-engineer (`crates/modeling`)
**Implements:** roadmap §4 (boolean v1 / v2 / v3)
**Crate:** `cadkernel-modeling::boolean`

---

## 1. Problem statement

Given two manifold solids $A, B \subset \mathbb{R}^3$ represented as half-edge B-Reps with NURBS-backed faces, compute one of:

- $A \cup B$ — union (regularised)
- $A \cap B$ — intersection (regularised)
- $A \setminus B$ — difference (regularised)
- $A \oplus B$ — symmetric difference (XOR) — only roadmap v3+

Output is a manifold solid with valid B-Rep topology and faces tagged for **persistent naming** (so feature edits in the parent feature do not invalidate downstream features).

"Regularised" means the result has no dangling faces, edges, or vertices: $A \cup_R B = \overline{\text{interior}(A \cup B)}$.

---

## 2. Pipeline

```text
Input:  Solids A, B
Output: Solid R, history Operation

Phase 1: Spatial pre-check
    1.1 BBox overlap test                      → empty? early-exit
    1.2 Inside-outside test (one vertex of A in B?)
        → no overlap & no containment → A ∪ B = disjoint, A ∩ B = ∅, A − B = A

Phase 2: Face-face intersection
    2.1 Pair faces with overlapping BBoxes (BVH on A faces vs BVH on B faces)
    2.2 For each candidate pair (F_a, F_b):
        2.2.1 Plane-plane special case (analytical)
        2.2.2 Quadric-quadric special case (analytical, e.g., Levin)
        2.2.3 Otherwise: SSI (see nurbs-ssi.md)
    2.3 Result: set of intersection curve segments with tags (edge_a, edge_b, kind)

Phase 3: Edge graph construction
    3.1 Split A's edges where intersection curves cross them (1D-1D intersection)
    3.2 Split B's edges similarly
    3.3 Build full edge graph: original A edges (split) + original B edges (split) + new SSI edges

Phase 4: Face splitting
    4.1 For each face F_a in A: collect all SSI segments lying on F_a
    4.2 2D arrangement in F_a's UV space: build planar graph
    4.3 Identify regions (faces of the arrangement) — each becomes a sub-face
    4.4 Repeat for each face F_b in B

Phase 5: Region classification
    5.1 For each sub-face F'_a: classify "inside B?" / "outside B?" / "on B?"
        via point-in-solid test on a representative interior point
    5.2 Repeat for sub-faces of B

Phase 6: Half-edge assembly
    6.1 Per operation:
        Union:        keep faces tagged outsideB ∪ outsideA ∪ onB-aligned
        Intersection: keep faces tagged insideB ∪ insideA ∪ onB-aligned
        Difference A−B: keep faces tagged outsideB ∪ insideA-flipped ∪ onB-flipped-aligned
    6.2 Stitch: for every kept face, find the other kept face sharing each edge;
        link half-edges (twin pointers).
    6.3 Validate: every half-edge has a twin; no naked edges; no T-junctions.

Phase 7: Tag propagation (persistent naming)
    7.1 Each new face inherits parent face's Tag (one of A or B).
    7.2 Each split sub-face gets composite Tag = parent_tag · split_index.
    7.3 SSI-generated edges get Tag::generated(EntityKind::Edge, op_id, k).
    7.4 New vertices at SSI intersections get Tag::generated(...).

Phase 8: Validation
    8.1 Manifold check: every edge has exactly 2 half-edges (twin pair).
    8.2 Euler-Poincaré check: V − E + F − 2(S − G) = 0 where G = genus, S = shells.
    8.3 Volume sign check: signed volume > 0 for solid (outward normals).
    8.4 If validation fails: rollback to pre-boolean state; report error.

Output: Solid R with Operation { kind: Boolean(op), inputs: [A, B], tags: [...] }
```

---

## 3. Robustness — symbolic perturbation (Yap 1990)

**Problem:** Floating-point arithmetic gives ambiguous results for degenerate cases (e.g., three planes meeting at a single point with shared coordinates). Symbolic perturbation resolves these as if each input had a unique infinitesimal offset.

### 3.1 Construction

Replace each coordinate $x_i$ with:

$$\tilde{x}_i = x_i + \varepsilon_i$$

where $\varepsilon_i = \varepsilon^{2^i}$ is a symbolic infinitesimal with $\varepsilon \to 0^+$ and $\varepsilon_i \ll \varepsilon_{i-1}$.

A predicate $P(\tilde{x}_1, \ldots, \tilde{x}_n)$ that involves a polynomial of degree $d$ in coordinates expands as:

$$P = P_0 + \varepsilon \cdot P_1 + \varepsilon^2 \cdot P_2 + \cdots$$

The **sign of P** is determined by the sign of the first non-zero coefficient $P_k$.

### 3.2 Worked example

Three planes $\pi_1, \pi_2, \pi_3$ meet at common point $(0, 0, 0)$ — degenerate. Apply perturbation:

$$\tilde\pi_i: a_i x + b_i y + c_i z = d_i + \varepsilon_i$$

The intersection $\tilde\pi_1 \cap \tilde\pi_2 \cap \tilde\pi_3$ now has unique solution:

$$\tilde p = \pi^{-1} (d + \varepsilon)$$

where $\pi$ is the $3 \times 3$ normal matrix. As $\varepsilon \to 0$, $\tilde p \to 0$, but the **direction of approach** is well-defined and consistently signed across all predicates that reference these planes — so the boolean engine sees a consistent topology.

### 3.3 Implementation

We do not literally compute infinitesimals. Instead, we tag each input vertex/edge/face with a unique 64-bit perturbation index $i$. Predicates that would return zero (degenerate) are resolved by computing the next-order term using the perturbation indices to break ties deterministically.

Reference: Yap, "A geometric consistency theorem for a symbolic perturbation scheme," *J. Computer & System Sciences* 40(1), 1990.

---

## 4. Robustness — interval arithmetic + exact-rational fallback

For predicates whose floating-point evaluation is too close to zero (< 100 ULP), escalate:

1. **Interval evaluation.** Compute the predicate using interval arithmetic; if interval excludes 0, accept sign.
2. **Exact rational fallback.** If interval contains 0, re-evaluate with exact rational arithmetic (`num-rational::Rational64` or `dashu` for arbitrary precision). This is slow but gives the true sign.

Cache results in a per-operation memoisation table keyed by predicate (vertex, vertex, vertex) tuple. Typical hit rate ≥ 95 % during a boolean op.

---

## 5. Point-in-solid test

Used in Phase 5 region classification.

### 5.1 Algorithm — winding number (Hormann-Agathos generalised)

For point $P$ and solid $B$ with surface $\partial B$:

$$w(P, B) = \frac{1}{4\pi} \int_{\partial B} \frac{(\mathbf{r} - P) \cdot d\mathbf{A}}{\|\mathbf{r} - P\|^3}$$

If $w \approx 1$: $P$ inside $B$.
If $w \approx 0$: $P$ outside $B$.
Other values: $P$ on a non-manifold or self-intersecting surface — rejected.

We approximate by tessellating $\partial B$ and summing solid-angle contributions. Tessellation tolerance is 10× the boolean tolerance, ensuring sign reliability.

### 5.2 Ray-casting fallback

For watertight solids, simpler ray-casting with parity counting also works. We use winding number as it generalises to non-manifold and handles ray-edge ambiguity better.

Reference: Jacobson, Kavan, Sorkine-Hornung, "Robust inside-outside segmentation using generalized winding numbers," *ACM TOG* 2013.

---

## 6. Failure modes

| Failure | Detection | Mitigation |
|---|---|---|
| Coplanar faces (overlap) | dot of normals > $1 - 10^{-9}$, distance < $10^{-9}$ | Treat as 2D overlap; use 2D boolean on the shared plane |
| Tangent edges | edge of A coincident with edge of B over an interval | Merge edges; redirect half-edges |
| T-junction in input | input topology already invalid | Pre-validate; auto-heal up to tolerance, else reject |
| SSI returned partial | trace exited domain | Try reverse-direction trace; if still partial, fallback to tessellation-based boolean for that face pair |
| Empty result classification | every region tagged "outside" but result is non-empty | Recheck winding-number computation with finer tessellation |
| Duplicate half-edges after split | numerical drift created twin sub-edges | Snap-merge within tolerance; rebuild adjacency |
| Self-intersection in output | output volume signs inconsistent | Reject; report; rollback |
| Genus inflation (extra holes) | Euler-Poincaré mismatch | Reject; report; rollback |

---

## 7. Tessellation-based boolean (fallback v2)

When NURBS-level boolean fails (very rare, < 0.1 %), fall back to:

1. Tessellate both inputs at high tolerance ($\tau / 100$ of geometric tolerance).
2. Use a libigl-style mesh boolean (Cherchi-Pellacini-Attene-Livesu 2020 fast-and-robust).
3. Reconstruct B-Rep from output mesh: faces by smooth-group / coplanar clustering, edges by sharp-feature detection.
4. Tag with **lossy** annotation; user warned.

Reference: Cherchi, Pellacini, Attene, Livesu. "Fast and robust mesh arrangements using floating-point arithmetic." *ACM TOG* 39(6), 2020.

---

## 8. Numerical guards

| Constant | Value | Purpose |
|---|---|---|
| `BOOL_TOL_GEOM` | $10^{-7}$ | Geometric tolerance (vertex merge, edge snap) |
| `BOOL_TOL_PARAM` | $10^{-9}$ | Parameter-space tolerance |
| `BOOL_TOL_NORMAL` | $10^{-9}$ | Normal-vector tolerance for coplanar |
| `BOOL_TOL_VOL` | $10^{-12}$ | Volume sanity check |
| `WINDING_DECISION_THRESHOLD` | $0.5$ | Winding number above 0.5 → inside |
| `EXACT_FALLBACK_ULP` | 100 | ULP threshold to escalate to rational |
| `MAX_FACE_PAIRS` | $10^7$ | BVH pruning cap (avoid pathological inputs) |
| `OUTPUT_DEDUP_TOL` | $10^{-9}$ | Vertex deduplication on output |

---

## 9. Acceptance gates

### 9.1 v1.0 gate

- 12 × 12 = 144 primitive pairs (box, sphere, cylinder, cone, torus, prism, pyramid, wedge, ellipsoid, paraboloid, capped cylinder, hollow box) × 3 ops = 432 cases. **All pass with no panics.**
- Manual stress: 50 hand-crafted "tricky" cases (coplanar faces, tangent cylinders, etc.). **All pass.**
- Volume conservation: $|V(A \cup B) + V(A \cap B) - V(A) - V(B)| < 10^{-9}$ relative.

### 9.2 v2.0 gate

- 1 M random pairs (procedurally generated by perturbing primitives + composing). ≥ 99.9 % pass; 0 panics; failures degrade gracefully (clear error, no corrupt output).
- Round-trip: $A \cup B \setminus B \approx A$ within tolerance.
- Performance: median pair $< 5$ ms; 99th percentile $< 50$ ms.

### 9.3 v3.0 gate

- Industrial corpus: 290 STEP parts, pairwise booleans where geometrically meaningful: 100 % pass.
- All FreeCAD test suite booleans pass.
- Persistent naming: edit upstream feature, downstream feature still resolves correctly in 99 % of edits.

---

## 10. Test corpus

In `crates/modeling/tests/boolean_corpus/`:

- `primitives/` — 144 primitive pairs.
- `tricky/` — 50 hand-crafted edge cases.
- `random/` — 1 M generated cases (proptest with fixed seeds for reproducibility).
- `industrial/` — STEP-imported pairs.
- `regression/` — historical bugs frozen as tests.

Per-release: full corpus must pass on Linux, macOS, Windows in CI.

---

## 11. References

1. Yap. "A geometric consistency theorem for a symbolic perturbation scheme." *J. Computer & System Sciences* 40(1), 1990.
2. Mäntylä. *An Introduction to Solid Modeling*. Computer Science Press, 1988. (Euler-op assembly.)
3. Hoffmann. *Geometric and Solid Modeling*. Morgan Kaufmann, 1989.
4. Jacobson, Kavan, Sorkine-Hornung. "Robust inside-outside segmentation using generalized winding numbers." *ACM TOG* 32(4), 2013.
5. Cherchi, Pellacini, Attene, Livesu. "Fast and robust mesh arrangements using floating-point arithmetic." *ACM TOG* 39(6), 2020.
6. Hormann, Agathos. "The point in polygon problem for arbitrary polygons." *Computational Geometry* 20(3), 2001.
7. OCCT Boolean Operations User's Guide. <https://dev.opencascade.org/doc/overview/html/occt_user_guides__boolean_operations.html>

---

## 12. Implementation notes

- **No `unwrap()` in any boolean code path.** Errors propagate via `KernelResult`.
- **Operation transactional**: failure rolls back via shadow B-Rep state; original solids untouched.
- **Send + Sync**: parallelism by face pairs in Phase 2.
- **Determinism**: face pairs sorted by stable spatial hash before processing.
- **Deterministic perturbation indices**: assigned via topology-traversal order, not pointer addresses.
- **Memoisation**: predicate cache cleared per operation (no inter-op leakage).
