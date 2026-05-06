# NURBS Surface-Surface Intersection (SSI)

**Status:** spec v1, last updated 2026-05-06
**Owner:** kernel-engineer (`crates/geometry`)
**Implements:** roadmap §3 (geometry), §4 (boolean prerequisites)
**Crate:** `cadkernel-geometry::intersect::ssi`

---

## 1. Problem statement

Given two NURBS surfaces $S_1: [u_1, v_1] \to \mathbb{R}^3$ and $S_2: [u_2, v_2] \to \mathbb{R}^3$, compute the intersection set $I = \{(p_1, p_2) : S_1(p_1) = S_2(p_2)\}$ as a finite collection of:

- **Curve branches** — connected 1-dimensional manifolds in $\mathbb{R}^3$ with parameter mappings $\gamma : [0, 1] \to ([u_1, v_1] \times [u_2, v_2] \times \mathbb{R}^3)$.
- **Isolated tangential points** — where $T_p S_1 = T_p S_2$ but the surfaces touch but do not cross.
- **Coincident regions** — where $S_1$ and $S_2$ overlap on a 2D patch.

The output is a **CompositeCurve** with:
- 3D point trace (sampled, B-spline approximated).
- Parameter trace in $S_1$ UV space.
- Parameter trace in $S_2$ UV space.
- Topological tags marking crossings, tangencies, branch endpoints.

---

## 2. Algorithm overview (Patrikalakis-Maekawa)

Three phases:

1. **Loop detection.** Find all topological components of $I$ — closed loops fully inside both surfaces, open arcs ending on surface boundaries, isolated points.
2. **Tracing.** For each component, walk along the intersection curve via marching equations.
3. **Approximation.** Fit a NURBS curve to the traced point sequence with prescribed tolerance.

Reference: Patrikalakis, Maekawa, *Shape Interrogation for Computer Aided Design and Manufacturing*, Springer, 2010, Ch. 4–5.

---

## 3. Phase 1 — Loop detection

### 3.1 Bounding box pruning

If $\text{BBox}(S_1) \cap \text{BBox}(S_2) = \emptyset$, return empty intersection.

### 3.2 Subdivision tree

Subdivide each surface into Bezier patches via knot insertion (Boehm's algorithm). For each pair of patches $(P_1, P_2)$ with overlapping bounding boxes:

- If both $\text{BBox}(P_1)$ and $\text{BBox}(P_2)$ have diameter $< \tau_{\text{patch}}$, record a **candidate intersection seed** at the patch centroids.
- Otherwise, subdivide further.

### 3.3 Topology hint

For each surface, compute the **silhouette curve** with respect to the other surface's normal field: points where $(S_1(u, v) - S_2(p_2^*)) \cdot N_2 = 0$. Loops lie inside silhouette regions.

### 3.4 Boundary entry detection

For each surface boundary edge, sample $n=64$ points and project onto the other surface. If the projected residual passes through zero on a sub-interval, that interval contains a **boundary intersection** — the start/end of an open arc.

### 3.5 Output

A list of seeds: `(uv_1, uv_2, point_3d, kind ∈ {ClosedLoop, BoundaryArc, Tangent, Isolated})`.

---

## 4. Phase 2 — Tracing

### 4.1 Marching equation

At a regular intersection point, the intersection curve has tangent direction:

$$\hat{t} = \frac{N_1 \times N_2}{\|N_1 \times N_2\|}$$

where $N_i = \partial_u S_i \times \partial_v S_i$ is the surface normal.

The curve evolves via the ODE:

$$\frac{d}{ds} \begin{pmatrix} u_1 \\ v_1 \\ u_2 \\ v_2 \end{pmatrix} = J^{-1} \begin{pmatrix} \hat{t} \cdot \partial_u S_1^{-1} \\ \hat{t} \cdot \partial_v S_1^{-1} \\ \hat{t} \cdot \partial_u S_2^{-1} \\ \hat{t} \cdot \partial_v S_2^{-1} \end{pmatrix}$$

where $J$ is the $4 \times 4$ tangent-plane Jacobian.

### 4.2 Predictor-corrector

```text
fn trace(seed_uv1, seed_uv2, kind, surfaces):
    points ← []
    state ← (seed_uv1, seed_uv2)
    h ← step_initial = 0.01

    while not_terminated(state, kind, points):
        # Predictor: 4th-order Runge-Kutta along marching tangent
        state_pred ← rk4_step(state, h)

        # Corrector: project back to intersection manifold
        state_new ← newton_project(state_pred, surfaces, max_iter = 5)

        if newton_failed:
            h ← h / 2
            if h < h_min: return PartialTrace(points)
            continue

        # Adaptive step
        curvature ← estimate_curvature(state, state_new)
        if curvature > κ_max:
            h ← h / 2
        elif curvature < κ_min:
            h ← min(h * 1.5, h_max)

        points.push(evaluate(state_new))
        state ← state_new

    return Trace(points)
```

### 4.3 Newton projection

Given predicted $(u_1^*, v_1^*, u_2^*, v_2^*)$:

$$F(u_1, v_1, u_2, v_2) = S_1(u_1, v_1) - S_2(u_2, v_2)$$

solve $F = 0$ via 3D Newton's method:

$$\Delta x = -[J_F]^+ F$$

where $[J_F]^+$ is the Moore-Penrose pseudoinverse of the $3 \times 4$ Jacobian. The 1-dimensional null space of $J_F$ corresponds to motion along the intersection curve, so we constrain $\Delta x \perp \hat{t}$ to project orthogonally to the curve.

### 4.4 Termination conditions

- Reached UV-domain boundary of $S_1$ or $S_2$ (open arc end).
- Returned within $\tau_{\text{loop}}$ of seed (closed loop complete).
- Tangent direction degenerated ($\|N_1 \times N_2\| < \tau_{\text{tangent}}$ → tangential branch).
- Step count exceeded budget (failure: log warning, return partial).

---

## 5. Phase 3 — Approximation

### 5.1 Knot vector construction

Given $n$ traced points $\{P_k\}$ with chord-length parameters $\bar{u}_k$:

$$\bar{u}_0 = 0, \quad \bar{u}_k = \bar{u}_{k-1} + \|P_k - P_{k-1}\|, \quad \bar{u}_n = 1 \text{ (normalised)}$$

Insert knots adaptively where:

$$\frac{|P_{k-1} P_k P_{k+1}|}{\|P_{k+1} - P_{k-1}\|^2} > \tau_{\text{curvature}}$$

(area-over-chord ratio thresholds curvature).

### 5.2 Least-squares fit

Cubic NURBS curve $C(u) = \sum N_{i, 3}(u) P_i^*$ with control points $P_i^*$ minimising:

$$\sum_k \|C(\bar{u}_k) - P_k\|^2$$

Solved via QR decomposition of the basis matrix (Lee 1989).

### 5.3 Tolerance check

For each traced sample $P_k$, verify $\|C(\bar{u}_k) - P_k\| < \tau_{\text{fit}}$. If exceeded, insert additional knots and refit.

### 5.4 Endpoint pinning

For boundary arcs, force start/end control points to lie exactly on surface boundaries (no interpolation drift).

---

## 6. Failure modes

| Failure | Detection | Mitigation |
|---|---|---|
| Tangential intersection | $\|N_1 \times N_2\| < 10^{-9}$ | Switch to **Krishnan-Manocha bracketing**: parameterise via osculating-circle radius difference |
| Coincident region | local signed distance ≡ 0 over UV patch | Detect via patch flatness test; emit `CoincidentRegion` topology |
| Loop closure miss | trace exits domain mid-loop | Restart from far seed; log warning |
| High-curvature spike | step explodes | Halve step, retry; if 5 consecutive failures, mark "unresolved" |
| Self-intersection on curve | 3D self-distance $< 10^{-9}$ before parameter equality | Split into two branches |
| Numerical drift | traced point distance to surface $> 10 \tau$ | Re-project; if recurring, mark partial trace |
| Boundary entry missed | open arc endpoint inside domain | Re-scan boundary at finer resolution |

References:
- Krishnan, Manocha. "An efficient surface intersection algorithm based on lower dimensional formulation." *ACM TOG* 1997.
- Sederberg, Parry. "Comparison of three curve intersection algorithms." *CAD* 1986.

---

## 7. Numerical guards

| Constant | Value | Purpose |
|---|---|---|
| `EPSILON_PATCH` | $10^{-3}$ | Bounding-box subdivision threshold |
| `EPSILON_NEWTON` | $10^{-10}$ | Newton convergence on intersection |
| `EPSILON_TANGENT` | $10^{-9}$ | Tangential degeneracy threshold |
| `EPSILON_LOOP_CLOSE` | $10^{-7}$ | Closed-loop closure tolerance |
| `EPSILON_FIT` | $10^{-6}$ | NURBS approximation tolerance |
| `STEP_INITIAL` | 0.01 | Initial trace step size (UV units) |
| `STEP_MIN` | $10^{-8}$ | Minimum step before failure |
| `STEP_MAX` | 0.1 | Maximum step (avoid skipping features) |
| `KAPPA_MAX` | 1.0 | Upper curvature for step decrease |
| `KAPPA_MIN` | 0.01 | Lower curvature for step increase |
| `TRACE_BUDGET` | 100 000 | Max points per branch |

---

## 8. Edge cases handled

1. **Surface-plane intersection.** Plane treated as zero-curvature surface; trace simplifies but algorithm is unchanged.
2. **Cylinder-cylinder identical axes.** Coincident region detected → emit full surface.
3. **Sphere-sphere tangent.** Single-point intersection; isolated-point branch.
4. **Trimmed surface intersection.** Pre-trim against UV trimming loops; intersect only valid regions.
5. **Periodic surfaces.** Treat $u = 0$ and $u = u_{\max}$ as identified; trace can wrap.
6. **C0 surface (knot multiplicity = degree).** Step around the discontinuity via boundary projection.
7. **Self-intersecting input surface.** Pre-validate; split into self-non-intersecting patches.

---

## 9. Acceptance gates

### 9.1 v1.0 gate (roadmap §4 prerequisite)

- 144 primitive-pair intersections (12 primitives × 12) all produce valid CompositeCurve or correctly classified as empty.
- Plane × {sphere, cylinder, cone, torus}: closed-form verified; SSI agrees within $10^{-6}$.
- Sphere × sphere tangent: isolated point detected.
- Cylinder × cylinder identical axis: coincident region detected.

### 9.2 v2.0 gate (roadmap §4 boolean v2)

- 1 M random surface pairs (procedurally generated): 0 panics, ≤ 0.1 % unresolved (fall-back tessellation tolerance).
- Self-intersection detection on traced curves.
- Round-trip: trace SSI, re-evaluate at every traced point, residual $< \tau$.

### 9.3 v3.0 gate (industrial)

- STEP-imported part library (290 parts) booleans use SSI exclusively for non-primitive pairs; 100 % robustness.
- Performance: SSI of two B-spline surfaces (degree 3, 10×10 control net) in $< 50$ ms on reference hardware.

---

## 10. Test corpus

Located in `crates/geometry/tests/ssi_corpus/`:

- `analytical/` — 24 cases with closed-form intersection (plane × {primitives}); checked against analytical reference.
- `tangent/` — 12 cases of tangential intersection.
- `coincident/` — 8 cases of coincident regions.
- `random_bezier/` — 1000 random Bezier-patch pairs.
- `industrial/` — 50 STEP-imported surface pairs.

CI gate: 100 % on analytical + tangent + coincident; ≥ 99.5 % on random; 0 panics.

---

## 11. References

1. Patrikalakis, Maekawa. *Shape Interrogation for Computer Aided Design and Manufacturing*. Springer, 2010, Chapters 4–5.
2. Sederberg, Parry. "Comparison of three curve intersection algorithms." *CAD* 18(1), 1986.
3. Krishnan, Manocha. "An efficient surface intersection algorithm based on lower-dimensional formulation." *ACM TOG* 16(1), 1997.
4. Boehm. "Inserting new knots into B-spline curves." *CAD* 12(4), 1980.
5. Lee. "Choosing nodes in parametric curve interpolation." *CAD* 21(6), 1989.
6. Hoschek, Lasser. *Fundamentals of Computer Aided Geometric Design*. AK Peters, 1993.
7. Piegl, Tiller. *The NURBS Book*, 2nd ed., Springer, 1997.

---

## 12. Implementation notes

- **Send + Sync** trait objects: tracing is parallelisable across branches.
- **No global state**: tracer holds its own scratch buffer.
- **Deterministic**: seed ordering fixed by spatial hash to avoid floating-point ordering drift across architectures.
- **Cancellable**: long-running traces respect a cancellation token (UI responsiveness).
- **Logs**: each phase emits structured logs at `INFO` (success) / `WARN` (fallback) / `ERROR` (giving up), tagged with `ssi_request_id` for tracing.
