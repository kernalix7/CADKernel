# Algorithm Dispatch Matrix

**Status:** spec v1, last updated 2026-05-06
**Owner:** kernel-engineer

When CADKernel receives a geometric query (intersect surfaces, fillet edge, project point) it must choose **which** algorithm to run. A naïve "always run the fully general NURBS algorithm" path wastes 100–1000× on cases that have closed-form solutions.

This document defines the dispatch matrices used at each major decision point. Implementations live in `crates/geometry/src/dispatch.rs` and `crates/modeling/src/dispatch.rs`.

---

## 1. Surface-Surface Intersection (SSI) dispatch

When asked to intersect $S_1 \cap S_2$:

| Type of $S_1$ | Type of $S_2$ | Algorithm | Spec |
|---|---|---|---|
| Plane | Plane | Closed-form: line of intersection from cross product of normals | analytical |
| Plane | Cylinder | Closed-form: ellipse, line, or empty depending on plane-axis angle | analytical |
| Plane | Sphere | Closed-form: circle (or single tangent point or empty) | analytical |
| Plane | Cone | Closed-form: conic section by Dandelin spheres construction | analytical |
| Plane | Torus | Up to 4 closed curves (Villarceau circles for tangent plane); algebraic | analytical + Krishnan-Manocha |
| Plane | NURBS | Iso-curve intersection: project NURBS control points to plane normal axis, find sign-change spans, solve via 1D Newton | iso-curve |
| Cylinder | Cylinder (parallel axes) | Closed-form: 2 lines, 1 line, or empty | analytical |
| Cylinder | Cylinder (intersecting axes) | Closed-form via Cassini ovals (Levin's algorithm) | analytical |
| Cylinder | Sphere | Closed-form: up to 4 circles | analytical |
| Cylinder | Cone | Quadric-quadric: Levin 1976 algorithm | analytical |
| Cylinder | Torus | Up to 8 curves; degree-4 algebraic | algebraic |
| Sphere | Sphere | Closed-form: circle, point, or empty | analytical |
| Sphere | Cone | Closed-form: up to 2 circles | analytical |
| Sphere | Torus | Up to 4 circles | analytical |
| Cone | Cone | Quadric-quadric: Levin | analytical |
| Cone | Torus | Algebraic degree 4 | algebraic |
| Torus | Torus | Algebraic degree 8 — fall through to general NURBS SSI | NURBS SSI |
| Quadric | NURBS | Lift quadric to NURBS (degree 2) and use general SSI; **or** when normal field ≠ 0 use marching with quadric-side closed-form parametrisation | NURBS SSI / hybrid |
| NURBS | NURBS | General Patrikalakis 3-phase SSI | [nurbs-ssi.md](../algorithms/nurbs-ssi.md) |

### 1.1 Dispatch pseudocode

```text
fn intersect_surfaces(S1, S2) -> CompositeCurve:
    # Canonicalise: prefer closed-form on either side
    (A, B) ← canonicalise(S1, S2)         # so A.kind ≤ B.kind in dispatch table
    match (A.kind, B.kind):
        (Plane, Plane)    => plane_plane(A, B)
        (Plane, Cylinder) => plane_cylinder(A, B)
        ...
        (NURBS, NURBS)    => ssi_general(A, B)
        _ => ssi_general(A, B)            # fallback
```

### 1.2 Performance per row

Closed-form analytical: $< 10 \mu s$. NURBS general: $\sim 50$ ms typical, up to $500$ ms pathological. 4 orders of magnitude — dispatch matters.

---

## 2. Boolean face-pair classification

Phase 2 of the boolean pipeline ([boolean.md §2](../algorithms/boolean.md)) intersects each candidate face pair. The choice is the same as SSI dispatch above. Additionally, before SSI:

| Predicate | Decision |
|---|---|
| $\text{BBox}(F_a) \cap \text{BBox}(F_b) = \emptyset$ | Skip (no intersection possible) |
| $F_a$ and $F_b$ coplanar within tolerance | 2D boolean on shared plane (uses Triangle + winding); skip 3D SSI |
| $F_a$ and $F_b$ are coaxial cylinders with same radius | Merge as coincident region; skip SSI |
| $F_a$ and $F_b$ are concentric spheres with same radius | Merge as coincident region |
| Otherwise | Dispatch per §1 |

---

## 3. Fillet algorithm tier dispatch

When user requests fillet on edge set $E$ with radius profile $r(s)$:

| Condition | Tier |
|---|---|
| Constant $r$, planar adjacent faces | Tier 1 (rolling-ball, closed-form centre curve) |
| Constant $r$, ≥1 NURBS adjacent face | Tier 1 (rolling-ball, marching centre curve via ODE) |
| Variable $r(s)$ | Tier 2 (variable-radius, marching with $dr/ds$ term) |
| 3+ filleted edges meet at vertex with possibly different radii | Tier 3 (Choi-Lee setback corner) |
| User requests $G^2$ (class-A) | Tier 4 (Vida-Martin-Várady curvature minimisation) |
| User requests $G^3$ (show surface) | Tier 5 (conic-Bezier degree-5) |

Spec: [fillet.md](../algorithms/fillet.md).

### 3.1 Auto-tier rules

- Default tier: lowest tier sufficient for the request.
- Up-tier on user request only (`fillet --g2` or UI checkbox).
- Down-tier never (would silently degrade quality).

---

## 4. Sketch solver step dispatch

LM step ([sketch-solver.md §3](../algorithms/sketch-solver.md)) chooses between damped Gauss-Newton and Powell-dogleg trust-region:

| Condition | Strategy |
|---|---|
| First iteration | LM with $\lambda_0 = 10^{-3}$ |
| 5 consecutive successful steps | Decrease $\lambda$ by half (more aggressive Gauss-Newton) |
| Step rejected (residual increased) | Increase $\lambda$ by 2 |
| 5 consecutive rejections | Switch to Powell-dogleg trust region |
| 10 consecutive rejections in dogleg | Diagnose failure |

---

## 5. Tessellation refinement dispatch

For each UV cell during adaptive tessellation ([tessellation.md §3](../algorithms/tessellation.md)):

| Condition | Action |
|---|---|
| Cell deviation < $\tau$ | Emit triangles |
| Cell deviation > $\tau$, depth < `MAX_REFINEMENT_DEPTH` | Subdivide 4-way; recurse |
| Cell deviation > $\tau$, depth ≥ `MAX_REFINEMENT_DEPTH` | Emit triangles with warning (rare; indicates pathological surface) |
| Cell contains trim curve | Subdivide regardless of deviation; force CDT path |
| Surface is plane (zero curvature) | Skip refinement; emit cell as 2 triangles |

---

## 6. BVH leaf vs split dispatch

| Condition | Action |
|---|---|
| `prim_count ≤ LEAF_SIZE` (default 4) | Emit leaf |
| `depth ≥ MAX_DEPTH` (default 64) | Emit leaf (defensive) |
| Best SAH split has cost ≥ `LEAF_INTERSECT_COST × prim_count` | Emit leaf (split worse than just intersecting all) |
| Otherwise | Split, recurse |

---

## 7. Numerical-precision escalation dispatch

When a predicate (e.g., 3D orientation) returns a value too close to zero in `f64`:

| ULP distance to 0 | Action |
|---|---|
| > 100 ULP | Trust f64 sign |
| 10–100 ULP | Recompute with interval arithmetic (`inari` crate) |
| < 10 ULP or interval contains 0 | Recompute with exact rational (`num-rational::BigRational`) |
| Exact rational also returns 0 | Apply Yap 1990 symbolic perturbation; tie-break by perturbation indices |

Spec: [boolean.md §3, §4](../algorithms/boolean.md).

---

## 8. STEP entity export dispatch

When exporting a CADKernel surface to STEP ([step-mapping.md §2](../algorithms/step-mapping.md)):

| Surface kind | Default STEP entity | If $\rho \neq 1$ (rational) |
|---|---|---|
| Plane | `PLANE` | n/a |
| Cylinder | `CYLINDRICAL_SURFACE` | n/a |
| Sphere | `SPHERICAL_SURFACE` | n/a |
| Cone | `CONICAL_SURFACE` | n/a |
| Torus | `TOROIDAL_SURFACE` | n/a |
| Surface of revolution | `SURFACE_OF_REVOLUTION` | upgrade to `RATIONAL_B_SPLINE_SURFACE` |
| Extrusion | `SURFACE_OF_LINEAR_EXTRUSION` | upgrade to `RATIONAL_B_SPLINE_SURFACE` |
| Ruled surface | `B_SPLINE_SURFACE_WITH_KNOTS` (degree 1 in $v$) | `RATIONAL_B_SPLINE_SURFACE` |
| NURBS non-rational | `B_SPLINE_SURFACE_WITH_KNOTS` | n/a |
| NURBS rational | `RATIONAL_B_SPLINE_SURFACE` (complex inheritance with `B_SPLINE_SURFACE_WITH_KNOTS`) | itself |

The first column choice is preferred because primitive entities preserve semantics through round-trips that go through systems with strong typing (e.g., AP242-aware PMI tools). Falling back to `B_SPLINE_*` works always but loses semantics.

---

## 9. Maintenance

- Each new geometric primitive added to the kernel must add a row to §1 and §8.
- Each new sketch constraint must be classified in [sketch-solver.md §2](../algorithms/sketch-solver.md).
- Each new fillet research result must be tier-classified in §3.
- Dispatch tests live in `crates/geometry/tests/dispatch.rs` and assert that the right code path is taken for each row.

---

## 10. References

- Levin, "A parametric algorithm for drawing pictures of solid objects composed of quadric surfaces," *Comm. ACM* 19(10), 1976.
- Patrikalakis, Maekawa, *Shape Interrogation*, 2010.
- [docs/algorithms/nurbs-ssi.md](../algorithms/nurbs-ssi.md)
- [docs/algorithms/boolean.md](../algorithms/boolean.md)
- [docs/algorithms/fillet.md](../algorithms/fillet.md)
- [docs/algorithms/step-mapping.md](../algorithms/step-mapping.md)
