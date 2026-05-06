# Condition-Number Analysis

**Status:** spec v1, last updated 2026-05-06
**Owner:** kernel-engineer

This document defines the **maximum problem condition number** at which each kernel algorithm produces results within tolerance. Beyond the documented condition number, the algorithm switches to a fallback path or returns `KernelError::IllConditioned`.

For matrix problems, condition number $\kappa(A) = \|A\| \cdot \|A^{-1}\| = \sigma_{\max}(A) / \sigma_{\min}(A)$ (singular value ratio). For nonlinear problems, we use the local Jacobian condition number at the solution point.

The rule of thumb: $\log_{10}(\kappa)$ digits of accuracy are lost during the solve. f64 has ~15 decimal digits; algorithms targeting tolerance $10^{-9}$ can therefore tolerate $\kappa \leq 10^6$ in the linear case before losing the 6 digits of margin.

---

## 1. Per-algorithm thresholds

| Algorithm | Quantity | Safe limit | Action above limit |
|---|---|---|---|
| Sketch LM solver | $\kappa(J^T J)$ at iteration | $10^8$ | Increase damping $\lambda$; if $\kappa > 10^{10}$ for 3 iterations, declare singular and run deflation |
| Sketch DR-planning | per-cluster Jacobian $\kappa$ | $10^6$ | Merge cluster with neighbour and re-solve monolithically |
| NURBS evaluation (de Boor) | knot multiplicity / span ratio | knot ratio $\leq 10^{12}$ | Refit basis with regularised knots |
| NURBS SSI Newton projection | $\kappa(J_F)$ for $F = S_1 - S_2$ | $10^7$ | Reduce step; if persists, switch to tangent-Krishnan-Manocha tracker |
| Boolean predicate (3D orientation) | f64 ULP distance to zero | $> 100$ ULP | Escalate to interval arithmetic, then exact rational, then Yap perturbation |
| Boolean face-pair Newton (face split) | $\kappa(J)$ for face-face params | $10^7$ | Halve marching step; if persists, fall back to mesh boolean for that pair |
| Sparse Cholesky (sketch) | $\kappa(A)$ for $A = J^T J + \lambda \cdot \text{diag}(J^T J)$ | $10^{10}$ (with damping) | Increase $\lambda$ |
| Curve fitting (least-squares NURBS) | $\kappa(N^T N)$ for basis matrix | $10^8$ | Insert knot; refit |
| Surface fitting | $\kappa$ of bi-tensor basis | $10^8$ | Insert knots in degenerate direction |
| Fillet centre-curve marching | $\|N_1 \times N_2\|$ at trace step | $\geq 10^{-9}$ | Switch to tangent fillet branch (variable-direction) |
| Tessellation cell flatness | curvature × diag$^2$ | < tolerance | Subdivide |
| BVH SAH cost | NaN / Inf check on cost | finite | If non-finite, fall back to median-split |
| Recompute cache key | BLAKE3 collision (for 128-bit truncation) | $2^{64}$ entries | Use full 256-bit key (we use 128-bit truncated for cache, full 256 for `.cadk` content hash) |

---

## 2. Detection mechanism

Each algorithm has a numerical guard that computes the relevant condition estimate **cheaply** (no full SVD) and gates the dispatch:

### 2.1 Sketch solver $\kappa$ estimate

After computing $A = J^T J + \lambda \cdot \text{diag}(J^T J)$:

```rust
let cond_est = nalgebra::linalg::cond_estimate(&a);   // 1-norm condition estimate (Hager 1984)
if cond_est > 1e8 && lambda < 1e6 {
    lambda *= 4.0;          // escalate damping
    continue;
}
```

The 1-norm estimate costs $O(n^2)$ vs full SVD $O(n^3)$.

### 2.2 NURBS SSI

Track $\sigma_{\min}(J_F)$ via SVD of the small $3 \times 4$ Jacobian (cheap; $O(1)$ for fixed-size matrix). Trigger fallback when $\sigma_{\min} < 10^{-7}$.

### 2.3 Boolean predicate

For each predicate, compute the "exact" determinant in interval arithmetic alongside the floating-point version. If interval contains 0, the f64 value is unreliable — escalate.

```rust
let f64_val = predicate_f64(&pts);
let interval_val = predicate_interval(&pts);
if interval_val.contains(0.0) {
    return predicate_exact(&pts);
}
sign_of(f64_val)
```

---

## 3. Worked numerical example — sketch solver near singularity

Consider a sketch with two coincident lines $L_1 = (P_a, P_b)$ and $L_2 = (P_c, P_d)$ constrained to be parallel **and** $P_a$ coincident with $P_c$ **and** $P_b$ coincident with $P_d$. The parallel constraint becomes redundant once both endpoint coincidences are satisfied — Jacobian column for the parallel residual is in the column span of the coincident rows.

`SVD(J)` would show $\sigma_4 \approx 0$, $\kappa \to \infty$.

Solver behaviour:
1. Iteration 0: $\lambda = 10^{-3}$. $A = J^T J + \lambda \text{diag}$. $\kappa(A) \approx 10^{12}$ (close to singular).
2. Hager 1-norm estimate fires: `cond_est > 1e8` → escalate $\lambda = 4 \cdot 10^{-3}$.
3. With damping, $\kappa(A) \approx 10^{10}$ — safe to solve, but solution is dominated by damping (close to gradient descent).
4. After 5 iterations of damped descent, residual $\| f \| \approx \tau$ (since residuals are achievable). Solver terminates as `Converged`.
5. Post-solve classification ([sketch-solver.md §4](../algorithms/sketch-solver.md)) detects $\sigma_4 \approx 0$ via final SVD → reports `OverDetermined { redundant_constraints: [parallel_id] }`.
6. UI greys out the parallel constraint with "(redundant)" label.

Outcome: user sees their sketch solved, with a non-disruptive warning that one constraint is redundant — they can remove it if they want, but the system did not crash and did not silently produce a wrong answer.

---

## 4. Worked numerical example — boolean coplanar coincident faces

Two boxes $A, B$ share a face exactly (same plane, same outline, same orientation). Boolean $A \cup B$:

1. **Phase 2**: face-pair $F_a \cap F_b$ has $|N_1 \times N_2| = 0$ (parallel normals) and signed distance = 0 (coplanar). Coplanar branch triggered.
2. **2D boolean on shared plane**: 2D outlines compared. They are identical → 2D union = same outline.
3. **Decision**: in $A \cup B$, this face is interior to the result (it separates two interior regions, so it must be removed). The two faces are deleted from the output.
4. **Phase 6 stitching**: half-edges that bordered $F_a$ and $F_b$ are re-paired (twin pointers updated to skip the now-removed faces).
5. **Phase 8 validation**: Euler-Poincaré check on result. Since one face was removed but its bounding edges/vertices are still shared with surviving faces, Euler is preserved.

Without coplanar detection, the SSI engine would attempt to march on a degenerate surface intersection ($\hat{t} = N_1 \times N_2 / 0$), produce NaN, and the boolean would fail. The dispatch matrix ([dispatch-matrix.md §2](dispatch-matrix.md)) routes coplanar pairs to the 2D path *before* SSI is invoked — that is the correctness mechanism.

---

## 5. Tolerance hierarchy

CADKernel uses a fixed tolerance hierarchy. Each tolerance is a named constant; no magic numbers.

| Constant | Value | Rationale |
|---|---|---|
| `EPSILON_GEOM` | $10^{-9}$ | Vertex coincidence in mm units (1 nm). Below f64 representable distance for typical part scales. |
| `EPSILON_PARAM` | $10^{-9}$ | Curve / surface parameter coincidence in normalised $[0, 1]$ units. |
| `EPSILON_NORMAL` | $10^{-9}$ | Unit-vector dot-product tolerance for "parallel". |
| `EPSILON_AREA` | $10^{-12}$ | Triangle area; below this triangle is degenerate. |
| `EPSILON_VOLUME` | $10^{-12}$ | Solid volume sanity. |
| `EPSILON_FIT` | $10^{-6}$ | Curve / surface fitting target. Loose so fitting converges quickly. |
| `EPSILON_DIM_RES` | $10^{-6}$ | Sketch dimensional residual (mm). |
| `EPSILON_POS_RES` | $10^{-9}$ | Sketch positional residual (mm). |
| `EPSILON_RANK` | $10^{-12}$ | SVD rank cutoff for deflation. |
| `BOOL_TOL_GEOM` | $10^{-7}$ | Boolean geometric tolerance (vertex merge). Looser than `EPSILON_GEOM` because boolean intermediate states accumulate error. |

These constants are defined in `crates/core/src/numerical.rs`. Any algorithm wanting a different tolerance must justify it with a comment citing why the global default is wrong for that case.

---

## 6. Cross-architecture consistency

`f64` is IEEE 754 binary64, identical across x86-64, arm64, and Power. However:

- **FMA (fused multiply-add)**: x86 with FMA3 vs without can differ by 1 ULP per operation.
- **Order of operations** in compiler reordering can drift signs of near-zero predicates.

We force determinism with:

- `RUSTFLAGS="-Cllvm-args=-fp-contract=off"` for the kernel crates (no FMA fusion across architectures).
- Stable iteration order: never iterate `HashMap` in deterministic paths; use `BTreeMap` or sorted `Vec`.
- No reliance on pointer addresses for ordering.

Cross-arch CI verifies bit-identical output of golden tests on Linux x86-64, Linux arm64, macOS arm64, Windows x86-64. Tessellation-rendered images compared via SSIM (target > 0.95) since rasterisation differs slightly per GPU driver.

---

## 7. References

- Higham, *Accuracy and Stability of Numerical Algorithms*, 2nd ed., SIAM 2002.
- Goldberg, "What every computer scientist should know about floating-point arithmetic," *ACM Comp. Surveys* 1991.
- Hager, "Condition estimates," *SIAM J. Sci. Stat. Comp.* 1984.
- Demmel, *Applied Numerical Linear Algebra*, SIAM 1997.
- Shewchuk, "Adaptive precision floating-point arithmetic and fast robust geometric predicates," *Discrete & Computational Geometry* 1997.
- [docs/algorithms/sketch-solver.md](../algorithms/sketch-solver.md)
- [docs/algorithms/boolean.md](../algorithms/boolean.md)
