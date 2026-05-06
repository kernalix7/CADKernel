# 0015. Sparse Cholesky for sketch solver linear step

**Status:** Accepted
**Date:** 2026-05-06
**Deciders:** kernel-engineer

## Context

The sketch solver ([sketch-solver.md](../algorithms/sketch-solver.md)) solves a normal-equations system at every Levenberg-Marquardt iteration:

$$ (J^T J + \lambda \cdot \text{diag}(J^T J)) \cdot \delta x = -J^T f $$

For typical industrial sketches:
- Number of variables $n$: 50-500.
- Jacobian density: ~5 % (each constraint touches 2-4 entities; each entity has 2-3 coordinates).
- $A = J^T J$ density: ~10 %.
- Symmetric positive definite (because of $+ \lambda I$ damping).

Choices:

1. Dense LU (nalgebra `LU::new`).
2. Dense Cholesky (nalgebra `Cholesky::new`).
3. Sparse Cholesky (nalgebra-sparse `CscMatrix::cholesky` or external CHOLMOD).
4. Iterative (CG, MINRES) on the symmetric system.

## Decision

- **Default: sparse Cholesky** via `nalgebra-sparse 0.10` (pure Rust; no C dep).
- **Fallback: dense Cholesky** for $n < 32$ (overhead of sparse setup not worth it).
- **CHOLMOD-bridge: optional feature** behind `--features cholmod` for users with industrial sketches > 1,000 variables (rare). CHOLMOD has reordering heuristics (AMD, METIS) that beat nalgebra-sparse on huge sparse matrices.
- **Iterative methods: explicitly rejected** for the LM step.

## Alternatives considered

| Option | Rejected because |
|---|---|
| Dense LU | Matrix is SPD; Cholesky is 2× faster than LU on SPD. No reason to use LU. |
| Dense Cholesky everywhere | $O(n^3)$; for $n = 500$ that's $1.25 \cdot 10^8$ ops per iteration. Sparse drops this to $\sim O(n^{1.5})$ for typical sketch graphs. |
| CG iterative | Convergence depends on $\kappa(A)$; for ill-conditioned LM systems (over-constrained sketches) CG can stall. Direct factorisation is more robust. |
| MINRES iterative | Same family as CG; same problem. |
| Iterative refinement on dense Cholesky | Hybrid; works but more complex than sparse direct. Reserved for future investigation if profiling shows benefit. |
| GPU dense (cuSOLVER) | GPU upload latency dominates for $n < 5000$. Wrong tool. |
| External library (Eigen sparse) | Eigen is C++; PyO3-style bridge would add complexity. nalgebra-sparse is the Rust-native choice. |

## Consequences

**Positive:**
- Sparse direct factorisation handles ill-conditioned systems robustly (when combined with LM damping per [condition-numbers.md](../perf/condition-numbers.md)).
- 5-50× faster than dense Cholesky for typical industrial sketches.
- Pure Rust default; CHOLMOD opt-in keeps binary lean for users who don't need it.
- Symbolic factorisation cached across iterations (only numeric refactor per LM step) — order-of-magnitude speedup.

**Negative:**
- Sparse Cholesky has more code paths; bugs are subtler. Mitigated by golden tests in `tests/corpus/sketch/` and the dense-Cholesky fallback acts as an oracle in development builds.
- Reordering matters for performance; nalgebra-sparse uses simple AMD; for huge systems CHOLMOD's METIS/AMD blend wins.

**Neutral:**
- Solver convergence is unaffected by linear-step choice (the math is the same); only speed and robustness.

## References

- Davis, *Direct Methods for Sparse Linear Systems*, SIAM 2006.
- Chen et al., "Algorithm 887: CHOLMOD," *ACM TOMS* 35(3), 2008.
- nalgebra-sparse: <https://docs.rs/nalgebra-sparse/>
- [docs/algorithms/sketch-solver.md](../algorithms/sketch-solver.md)
- [docs/perf/condition-numbers.md](../perf/condition-numbers.md)
