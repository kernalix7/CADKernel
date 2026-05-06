# Fillet & Chamfer

**Status:** spec v1, last updated 2026-05-06
**Owner:** kernel-engineer (`crates/modeling`)
**Implements:** roadmap §5 (D-Fillet ladder)
**Crate:** `cadkernel-modeling::feature::fillet`

---

## 1. Problem statement

Given a B-Rep solid and a set of edges $E = \{e_1, \ldots, e_k\}$ marked for filleting with radius $r$ (constant or variable along edge), replace each edge with a **smooth blend surface** that is tangent to both adjacent faces at parameter offset $r$ from the edge.

The output must:

1. Preserve manifold topology.
2. Produce blend surfaces that are at least $G^1$ continuous (tangent) with adjacent faces.
3. Optionally produce $G^2$ (curvature continuous) for class-A surface work.
4. Handle the **set-back** problem at vertices where 3+ filleted edges meet.

---

## 2. The 5-tier ladder

### Tier 1 — Constant-radius rolling-ball (v1.0 default)

For an edge $e$ with adjacent faces $F_1, F_2$:

1. Compute the **center curve** $c(t)$ as the locus of centers of a ball of radius $r$ that is tangent to both $F_1$ and $F_2$.
2. The blend surface is:
   $$B(t, \theta) = c(t) + r \cdot (\cos\theta \cdot \hat{u}(t) + \sin\theta \cdot \hat{v}(t))$$
   where $\hat{u}, \hat{v}$ span the plane normal to $\dot c(t)$.
3. $\theta$ ranges over the angular sector subtended by the contact points on $F_1$ and $F_2$.

The center curve $c(t)$ is the solution of the system:

$$\begin{cases} \text{dist}(c(t), F_1) = r \\ \text{dist}(c(t), F_2) = r \\ \text{dist}(c(t), e) \text{ minimised} \end{cases}$$

For analytical face pairs (plane-plane, plane-cylinder, cylinder-cylinder), $c(t)$ has closed form. For NURBS faces, solve via marching ODE similar to SSI tracing.

### Tier 2 — Variable-radius (v1.5)

Radius is a function $r(s)$ of arc length along the edge. The center curve becomes:

$$\text{dist}(c(s), F_i) = r(s)$$

with smooth interpolation of $r$ over user-specified key points. The marching equation gains a $dr/ds$ term:

$$\frac{d c}{ds} = \hat{e}(s) - \frac{dr}{ds} \cdot \hat{n}_{\text{bisector}}(s)$$

### Tier 3 — Set-back at vertices (v2.0)

When 3 filleted edges meet at vertex $V$, naïve rolling-ball produces self-intersection. Solution per Choi-Lee 1989:

1. **Set back** each blend surface by distance $\delta$ from $V$, terminating it on a circular arc.
2. Build a **tri-tangent corner patch** $C(u, v)$ which is:
   - Tangent to all three blend surfaces at their setback termination curves.
   - $G^1$ continuous with each.
3. The corner patch is a Gregory or NURBS patch with 4 (or 3) trimming curves.

Algorithm:

```text
fn setback_corner(V, blends_at_V):
    # Determine setback distance δ
    δ ← max(blend_radii) * SETBACK_FACTOR    # typically 1.5
    # Truncate each incoming blend surface at distance δ from V
    truncation_curves ← []
    for blend in blends_at_V:
        c ← truncate_at_distance(blend, V, δ)
        truncation_curves.push(c)
    # Build corner patch as Coons patch with these curves as boundary
    corner ← coons_patch(truncation_curves)
    # Verify G^1 continuity; if violated by ε, perturb δ and retry
    return corner
```

Reference: Choi, Lee. "Sweep surfaces modelling via coordinate transformation and blending." *CAD* 21(2), 1989.

### Tier 4 — G² curvature-continuous blend (v3.0)

For class-A surfaces (automotive, aerospace), $G^1$ tangent is insufficient — visible reflection-line discontinuities. Use **circular arc with conic continuation** or **NURBS swept-blend** with curvature minimisation per Vida-Martin-Várady 1994:

1. Profile is no longer a circular arc but a **conic section** (Bezier rational curve) with shape parameter $\rho$.
2. $\rho$ tuned so that curvature at contact points matches face curvature.
3. Result: zero curvature jump across blend boundary.

Energy minimisation:

$$E[\text{blend}] = \int \kappa'(s)^2 ds$$

minimised over the family of $G^2$ candidates.

### Tier 5 — G³ via 5-axis-machinable conic-Bezier (v3.5+)

Toolpath-aware blends. Profile is degree-5 Bezier with curvature derivative continuity. Used for show surfaces in automotive class-A.

---

## 3. Edge-set planning

Before generating blends, plan the edge order:

1. **Topological sort** edges by setback dependency: edges whose setback affects another's geometry come first.
2. **Group connected edges** with same radius into a **fillet sequence** — a single continuous blend strip.
3. **Detect conflicts**: two edges meeting at a vertex with incompatible radii → setback corner required.

```text
fn plan_fillets(edges, radii):
    # Build graph: edges as nodes, shared vertices as graph edges
    graph ← build_edge_adjacency(edges)
    # Group connected same-radius components
    groups ← connected_components(graph, predicate = same_radius)
    # Topological sort by setback dependency
    order ← topo_sort(groups)
    # For each group, plan blend strip
    plans ← [plan_blend_strip(g) for g in order]
    return plans
```

---

## 4. Failure modes

| Failure | Detection | Mitigation |
|---|---|---|
| Radius too large for face | rolling-ball center exits face domain | Reject; recommend smaller $r$; visualise max feasible $r$ |
| Self-intersecting blend | blend surface has fold (Jacobian sign change) | Detect via signed area test; reject with diagnostic |
| Adjacent feature interference | blend overlaps another feature | Detect via BBox; report and offer auto-trim |
| Tangency failure at endpoint | edge endpoint on smooth surface (no clean tangent plane) | Auto-truncate to extend smoothly; user notified |
| Setback too aggressive | corner patch self-intersects | Reduce setback factor; if persists, refuse vertex group |
| Curvature exceeds face curvature | $G^2$ infeasible (would produce inflection) | Fall back to $G^1$ with warning |
| Variable radius monotonicity violation | $r(s)$ overshoots feasible range | Clamp; warn |
| Mixed-radius corner without setback | 3 edges different radii | Force setback corner mode; warn user |
| Disconnected blend | edge group fragments | Generate per-fragment blends; introduce explicit corner patches |

---

## 5. Numerical guards

| Constant | Value | Purpose |
|---|---|---|
| `FILLET_TOL` | $10^{-7}$ | Tangency tolerance |
| `FILLET_MAX_RADIUS_RATIO` | 0.49 | Max $r$ as fraction of smaller face curvature radius |
| `SETBACK_FACTOR` | 1.5 | Setback distance = factor × max radius at vertex |
| `BLEND_TRACE_STEPS` | 256 | Initial trace resolution along center curve |
| `BLEND_REFIT_TOL` | $10^{-6}$ | NURBS approximation tolerance |
| `G2_RESIDUAL_TOL` | $10^{-4}$ | Curvature continuity tolerance |
| `CORNER_PATCH_DEGREE` | 5 | NURBS degree for setback corner |

---

## 6. Acceptance gates

### 6.1 v1.0 gate

- All 12 primitive edges (box edges, cylinder cap edges, etc.) fillet with constant radius.
- Round-trip: fillet then unfillet (regenerate) restores topology.
- Performance: 100-edge sequence in $< 200$ ms.

### 6.2 v2.0 gate

- Variable radius: 50 hand-crafted cases pass.
- Setback corners: all 3-edge and 4-edge vertex configurations on box/cylinder unions pass.
- Industrial: FreeCAD fillet test suite 100 %.

### 6.3 v3.0 gate

- $G^2$: zone-plate reflection test passes (no visible curvature discontinuity).
- Class-A: automotive door-panel fillet sequence (provided as STEP) succeeds.

---

## 7. References

1. Choi, Lee. "Sweep surfaces modelling via coordinate transformation and blending." *CAD* 21(2), 1989.
2. Vida, Martin, Várady. "A survey of blending methods that use parametric surfaces." *CAD* 26(5), 1994.
3. Várady, Martin. "Reverse engineering." in *Handbook of Computer Aided Geometric Design*, 2002.
4. Hoschek, Lasser. *Fundamentals of Computer Aided Geometric Design*. AK Peters, 1993.
5. OCCT BlendFace documentation. <https://dev.opencascade.org/doc/refman/html/class_b_rep_fillet_a_p_i___make_fillet.html>
6. SolidWorks "Understanding Fillets" white paper, 2018.

---

## 8. Implementation notes

- **Tier dispatch:** runtime decision based on user request and face types. Default tier 1.
- **Caching:** center curves cached per edge; reused across radius adjustments.
- **Cancellable:** long-running blend sequences honour cancellation tokens.
- **History tracking:** every blend records its tier, radius profile, setback config in the operation history for replay.
