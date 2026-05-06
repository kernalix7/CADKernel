# Sketch Solver

**Status:** spec v1, last updated 2026-05-06
**Owner:** kernel-engineer (`crates/sketch`)
**Implements:** roadmap §6 (C-Sketch v1 / v2 / v3 phases)
**Crates:** `cadkernel-sketch` (engine), `cadkernel-modeling` (consumer)

---

## 1. Mathematical formulation

A 2D parametric sketch is a system of equations $f(x) = 0$ where:

- $x \in \mathbb{R}^n$ is the **entity-parameter vector** (point coords, line endpoints, circle centre + radius, arc centre + radius + start/end angle, etc.).
- $f : \mathbb{R}^n \to \mathbb{R}^m$ is the **constraint residual vector**.
- The sketch is **well-determined** when $m + r_{\text{anchor}} = n$ and $J = \partial f / \partial x$ has full rank $m$.
- $r_{\text{anchor}} = 3$ for a 2D sketch (one anchored point fixes 2 DoF; one anchored direction fixes 1 DoF).

The solver finds $x^*$ such that $\|f(x^*)\|_\infty < \tau$ for tolerances $\tau_{\text{pos}} = 10^{-9}$ on positional residuals and $\tau_{\text{dim}} = 10^{-6}$ on dimensional residuals.

---

## 2. Constraint catalogue (residual + Jacobian row)

### 2.1 Geometric constraints

| Constraint | Inputs | Residual $f_i(x)$ | Jacobian row structure |
|---|---|---|---|
| Coincident | $P_a, P_b$ | $P_a - P_b$ | dense 2 entries: $+I_2, -I_2$ |
| Horizontal | $P_a, P_b$ | $P_{a,y} - P_{b,y}$ | row $(0, +1, 0, -1)$ |
| Vertical | $P_a, P_b$ | $P_{a,x} - P_{b,x}$ | row $(+1, 0, -1, 0)$ |
| Parallel | $L_1, L_2$ | $\hat{d}_1 \times \hat{d}_2 \cdot \hat{z}$ | computed via line endpoints |
| Perpendicular | $L_1, L_2$ | $\hat{d}_1 \cdot \hat{d}_2$ | computed via line endpoints |
| Tangent (line, circle) | $L, C, r$ | $\|P_C - L\|_\perp - r$ | distance-point-to-line + radius derivative |
| Tangent (circle, circle) | $C_1, r_1, C_2, r_2$ | $\|C_1 - C_2\| - (r_1 \pm r_2)$ | unit vector + sign-pair |
| Concentric | $C_1, C_2$ | $C_1 - C_2$ | dense 2 entries |
| Equal length | $L_1, L_2$ | $\|L_1\| - \|L_2\|$ | normalised endpoint diffs |
| Equal radius | $r_1, r_2$ | $r_1 - r_2$ | row $(+1, -1)$ on radius columns |
| Symmetric about axis | $P_a, P_b, A$ | $\text{mirror}_A(P_a) - P_b$ | linear |
| Point-on-line | $P, L$ | $(P - L_a) \times \hat{d}_L$ | linear in $P$, nonlinear in $L$ |
| Point-on-curve (NURBS) | $P, C(u)$ | $C(u^*) - P$ where $u^* = \arg\min_u \|C(u) - P\|$ | parameter elimination via projection |
| Curvature continuous (G2) | curve $C_1$ at $u_1$, $C_2$ at $u_2$ | $\kappa_1 - \kappa_2$ | requires second derivative |
| Smooth (G1) | tangent equality at join | $\hat{T}_1 - \hat{T}_2$ | first derivative |
| Block | rigid sub-cluster | aggregate residual | n/a (handled by DR-planning) |

### 2.2 Dimensional constraints

| Constraint | Inputs | Residual $f_i(x)$ | Jacobian row |
|---|---|---|---|
| Distance | $P_a, P_b, d$ | $\|P_a - P_b\| - d$ | $\frac{P_a - P_b}{\|P_a - P_b\|}$ on $P_a$, negative on $P_b$ |
| Distance H | $P_a, P_b, d$ | $|P_{a,x} - P_{b,x}| - d$ | sign-aware row |
| Distance V | $P_a, P_b, d$ | $|P_{a,y} - P_{b,y}| - d$ | sign-aware row |
| Radius | $C, r$ | $C.r - r$ | row $(+1)$ on radius column |
| Diameter | $C, d$ | $2 C.r - d$ | row $(+2)$ on radius column |
| Angle (line, line) | $L_1, L_2, \theta$ | $\arctan_2(\hat{d}_1 \times \hat{d}_2, \hat{d}_1 \cdot \hat{d}_2) - \theta$ | nonlinear; analytic via product rule |
| Angle (3 points) | $P_a, P_b, P_c, \theta$ | angle at $P_b$ between rays to $P_a$ and $P_c$, minus $\theta$ | nonlinear |
| Arc length | arc, $L$ | $r \cdot \Delta\theta - L$ | linear in $r$, linear in $\Delta\theta$ |

### 2.3 Row count

Per scalar residual: 1 row. Therefore:

- Coincident contributes 2 rows.
- Distance contributes 1 row.
- Tangent (line, circle) contributes 1 row.

The total $m$ is the sum of rows across all constraints.

---

## 3. Solver loop (Levenberg-Marquardt)

```text
input:  x_0 ∈ R^n   initial guess
        C            constraint set
        τ_pos = 1e-9, τ_dim = 1e-6   tolerances
        max_iter = 100                budget
        λ_0 = 1e-3                    initial damping
output: Result {
            Converged(x*),
            UnderDetermined { dof },
            OverDetermined { redundant_constraints },
            Inconsistent { conflict_set },
            DivergedTimeout,
        }

x ← x_0
λ ← λ_0
for iter in 0..max_iter:
    (f, J) ← evaluate_residuals_and_jacobian(x, C)
    pos_res ← positional_subvector(f)
    dim_res ← dimensional_subvector(f)
    if ‖pos_res‖_∞ < τ_pos and ‖dim_res‖_∞ < τ_dim:
        return Converged(x)

    A ← J^T J + λ · diag(J^T J)        # Marquardt's scale-invariant damping
    b ← -J^T f
    Δx ← solve_sparse_linear(A, b)      # via cholmod or nalgebra-sparse
    if any(NaN(Δx)):
        return DiagnoseFailure(x, J, C)

    x_new ← x + Δx
    if ‖f(x_new)‖ < ‖f(x)‖:
        x ← x_new
        λ ← max(λ · 0.5, 1e-12)
    else:
        λ ← λ · 2.0
        if λ > 1e8:
            return DiagnoseFailure(x, J, C)

return DiagnoseFailure(x, J, C)
```

### 3.1 Damping strategy

Marquardt's scale-invariant variant: $A = J^T J + \lambda \cdot \text{diag}(J^T J)$ rather than $J^T J + \lambda I$. This makes the damping invariant to entity-parameter scaling (mm vs m vs inch).

- $\lambda \to 0$: pure Gauss-Newton, fastest near solution.
- $\lambda \to \infty$: pure gradient descent, slow but stable.
- Adaptive: halve on success, double on failure.

### 3.2 Linear solver

Sparse Cholesky on $A$ (which is symmetric positive semidefinite). Fall back to LU if rank-deficient. Use `nalgebra-sparse` Cholesky for portability; allow user to opt into `suitesparse-sys` (CHOLMOD) for performance in `cadkernel-sketch` feature flag `suitesparse`.

### 3.3 Warm start

For drag operations and parameter sweeps, warm start from previous solution. Convergence typically in 1-3 iterations.

### 3.4 Trust region fallback

When LM fails to converge for 5 consecutive outer iterations, switch to **Powell's dogleg** trust-region method:

1. Compute Cauchy step (steepest descent length).
2. Compute Gauss-Newton step.
3. Interpolate between them within current trust radius.
4. Update trust radius based on actual vs predicted reduction.

Reference: Nocedal & Wright, *Numerical Optimization*, 2nd ed., Ch. 4.

---

## 4. Well-determined classification

After solver halts, classify via Jacobian rank analysis:

```text
n  ← parameter count (effective: n - r_anchor)
m  ← constraint scalar count
r  ← rank(J)  evaluated by SVD threshold σ > 1e-12

if m + r_anchor < n:    return UnderDetermined { dof = n - m - r_anchor }
if r < m:               return OverDetermined { deflate(J) }
if ‖f‖_∞ > τ:           return Inconsistent { minimal_conflict_set(J, f, C) }
return WellDetermined
```

### 4.1 Deflation algorithm

For over-determined sketches:

1. SVD on $J = U \Sigma V^T$.
2. Identify columns of $U$ corresponding to $\sigma_i < 10^{-12}$ — these span the **deflation subspace**.
3. For each deflation vector $u_k$, find non-zero entries: these are the redundant constraints.
4. Remove redundant constraints (visually: greyed out in UI with "(redundant)" label); user can remove or convert to drive.

Reference: Hoffmann-Joan-Arinyo, "Symbolic constraints in constructive geometric constraint solving," *CAD* 1997.

### 4.2 Minimal conflict set (QuickXplain)

For inconsistent sketches:

1. Binary partition the constraint set $C$.
2. Test whether each half is consistent (run solver to convergence).
3. If both inconsistent: recurse into one half with the other fixed.
4. If one consistent: the inconsistency is in the other half; recurse.
5. Return the minimal subset whose removal restores consistency.

Complexity: $O(|C_{\min}| \cdot \log |C|)$ solver calls.

Reference: Junker, "QuickXplain: preferred explanations and relaxations for over-constrained problems," *AAAI* 2004.

---

## 5. DR-planning (decomposition-recombination)

For sketches with $> 50$ entities, monolithic LM is $O(n^3)$ per iteration on the linear solve. DR-planning reduces this to roughly $O(n^{1.3})$.

### 5.1 Algorithm

1. **Build constraint graph** $G = (V, E)$ where $V$ = entities, $E$ = constraints (multi-edge for multi-residual constraints).
2. **Identify rigid sub-clusters** via Hoffmann's degree-of-freedom analysis: a sub-cluster $S \subseteq V$ is rigid iff $\sum_{e \in E(S)} \text{rows}(e) \geq 2|S| - 3$.
3. **Topological order** of clusters: a cluster depends on another if they share an entity.
4. **Solve each cluster independently** via LM (small, fast).
5. **Recombine** via inter-cluster constraints, treating each cluster as a rigid body with 3 DoF (2 translation + 1 rotation in 2D).

### 5.2 Complexity

- Cluster identification: $O(|E|)$ via union-find.
- Per-cluster solve: $O(k^3)$ for cluster size $k$; with $k \ll n$, sum is much less than $O(n^3)$.
- Recombination: $O(c^3)$ for $c$ clusters; $c \ll n$.

Reference: Bouma, Fudos, Hoffmann, Cai, Paige, "Geometric constraint solver," *CAD* 1995.

### 5.3 When DR-planning fails

Some sketches resist clean decomposition (e.g., highly-coupled gear-train geometry). In these cases, fall back to monolithic LM with a warning to user about expected solve time.

---

## 6. Drag-with-constraints

Interactive drag: dragged entity has its position prescribed at every frame. Solver runs in **drag mode**:

### 6.1 Algorithm

```text
fn drag_step(x_current, dragged_entity, cursor_pos, C):
    # Add a virtual "drag" constraint pinning dragged entity to cursor
    drag_constraint ← Coincident(dragged_entity, cursor_pos)
    C_drag ← C ∪ {drag_constraint}

    x_new ← solve_LM(x_current, C_drag, max_iter = 5, warm_start = true)

    if convergence_quality(x_new) < acceptable:
        return WeakSolution { x_current, indicator = "soft" }

    return Solution { x_new }
```

### 6.2 Performance

- Per-frame budget: 10 ms for sketches ≤ 50 entities; 15 ms for ≤ 200 entities.
- If budget exceeded, render with last good state and "weak" indicator.
- DR-planning enables 60 fps drag on industrial sketches.

### 6.3 Visual feedback

- Resolved entities: solid colour (white in dark theme).
- Constrained entities: green outline.
- Conflict surface: red outline + tooltip.
- Drag cursor: highlighted in user's accent colour.

---

## 7. Failure modes and mitigations

| Failure mode | Detection | Mitigation |
|---|---|---|
| Singular Jacobian | $\sigma_{\min}(J) < 10^{-12}$ | Increase damping; perturb initial state by $10^{-6}$ |
| NaN propagation | `f64::is_nan` after evaluation | Reject step; revert to previous state |
| Zero-length line | $\|L\| < 10^{-9}$ | Refuse constraint application; user error |
| Coincident-distinct | distance=0 on coincident endpoints | Auto-convert to coincident constraint |
| Diverging iteration | $\|f_{k+1}\| > 10 \|f_k\|$ for 3 consecutive | Backtrack; halve step; re-evaluate |
| Cycle (oscillation) | gradient sign flip 5 consecutive | Switch to Powell's dogleg trust region |
| Tangent direction reversed | dot product changes sign mid-solve | Lock direction at solver start; reject sign-flip |
| Branch ambiguity (e.g., tangent has 2 solutions) | multiple valid solutions | Choose nearest to current state |
| Infinite arc / degenerate radius | $r > 10^9$ | Cap; user error |
| Sketch rotation drift | uncoincident reference frame | Re-anchor on every save |

---

## 8. Acceptance gates

### 8.1 v0.5 gate

- All 24 constraints implemented.
- DoF readout (well-/under-/over-determined).
- LM converges on 20 hand-crafted sketches.
- Drag at 60 fps for ≤ 50 entities.

### 8.2 v1.0 gate

- 1000 random valid sketches, 100 % solved.
- 1000 random under-determined: correct DoF reported.
- 1000 random over-determined: deflation produces a valid subset.
- Inconsistent: minimal conflict set returned.
- Drag at 60 fps for ≤ 200 entities.

### 8.3 v2.0 gate

- DR-planning with $O(n^{1.3})$ scaling demonstrated on 500-entity sketch.
- Industrial sketches scraped from FreeCAD examples + OpenSCAD library: 100 % solve.
- Drag at 60 fps for ≤ 500 entities (DR-decomposed).

### 8.4 v3.0 gate

- 28 constraints (full table §2 above).
- G2 / G3 curvature constraints.
- Variational direct edit: drag a face, sketch updates.

---

## 9. Test corpus

Stored in `crates/sketch/tests/corpus/`:

- `golden/` — 20 hand-crafted sketches with reference solutions.
- `random_well/` — 1000 random valid sketches generated per release.
- `random_under/` — 1000 random under-determined.
- `random_over/` — 1000 random over-determined.
- `industrial/` — 100 sketches scraped from open-source CAD.
- `proptest/` — 10 000 perturbations per release via `proptest`.

CI gate: 100 % pass on golden + industrial; ≥ 99 % on random; 0 panics on proptest.

---

## 10. Numerical guards

| Guard | Tolerance | Purpose |
|---|---|---|
| `EPSILON_POS` | $10^{-9}$ | Positional residual convergence |
| `EPSILON_DIM` | $10^{-6}$ | Dimensional residual convergence |
| `EPSILON_RANK` | $10^{-12}$ | SVD rank threshold |
| `EPSILON_LINE` | $10^{-9}$ | Zero-length line detection |
| `EPSILON_RADIUS` | $10^{-9}$ | Zero-radius circle detection |
| `MAX_RADIUS` | $10^9$ | Sanity cap (effectively infinite arc) |
| `MAX_LM_ITER` | 100 | Outer iteration cap |
| `LAMBDA_MAX` | $10^8$ | Damping cap (above this → fail) |

All guards are named constants in `crates/sketch/src/numerical.rs`, not magic numbers.

---

## 11. References

1. Bouma, Fudos, Hoffmann, Cai, Paige. "Geometric constraint solver." *CAD* 1995.
2. Hoffmann, Joan-Arinyo. "Symbolic constraints in constructive geometric constraint solving." *CAD* 1997.
3. Junker. "QuickXplain: preferred explanations and relaxations for over-constrained problems." *AAAI* 2004.
4. Marquardt. "An algorithm for least-squares estimation of nonlinear parameters." *J. SIAM* 1963.
5. Nocedal, Wright. *Numerical Optimization*, 2nd ed., Springer, 2006.
6. Powell. "A new algorithm for unconstrained optimization." in *Nonlinear Programming*, 1970.
7. Foufou, Michelucci. "Using Cayley-Menger determinants for geometric constraint solving." *Computer-Aided Design and Applications* 2004.
8. FreeCAD Sketcher solver reference: <https://wiki.freecadweb.org/Sketcher_solver>

---

## 12. Implementation notes

- **No `unwrap()` in solver hot path.** All `Result<T, SolverError>`.
- **Deterministic.** Same inputs (entity order + constraint order) → identical output.
- **Send + Sync.** Solver state is shareable across threads (each thread holds its own state).
- **Reentrant.** Solver may be nested (e.g., point-on-NURBS uses nested 1D solver).
- **No allocation in inner loop.** Reuse Jacobian buffer across LM iterations.
- **SIMD-friendly.** Constraint residual evaluation auto-vectorised where possible.
