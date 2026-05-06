# CADKernel Algorithm Specifications

This directory contains the **canonical algorithm specifications** for CADKernel. Each file is a self-contained deep-dive: derivation, pseudocode, data structures, failure modes, numerical guards, references.

These documents are referenced from `docs/COMMERCIAL_CAD_ROADMAP.md` §16 (which carries only the high-level summary). The roadmap states *what* must be built and *when*; these files state *how* and *with what guarantees*.

## Index

| File | Topic | Owning team | Status |
|---|---|---|---|
| [sketch-solver.md](sketch-solver.md) | 2D sketch constraint solver (LM, DR-planning, deflation, drag, Jacobian) | kernel-engineer | spec v1 |
| [nurbs-ssi.md](nurbs-ssi.md) | NURBS surface-surface intersection (loop detection, tracing, approximation) | kernel-engineer | spec v1 |
| [boolean.md](boolean.md) | Boolean engine (CSG → B-Rep, symbolic perturbation, interval arithmetic) | kernel-engineer | spec v1 |
| [fillet.md](fillet.md) | Fillet algorithm ladder (rolling-ball → G3 conic-Bezier) | kernel-engineer | spec v1 |
| [brep-abi.md](brep-abi.md) | `.cadk` file format byte layout, half-edge ABI, version migration | io-engineer | spec v1 |
| [step-mapping.md](step-mapping.md) | STEP AP242 entity ↔ CADKernel entity mapping table + round-trip tests | io-engineer | spec v1 |
| [recompute.md](recompute.md) | Feature DAG dirty propagation, cache key, incremental recompute | kernel-engineer | spec v1 (planned) |
| [bvh.md](bvh.md) | Spatial index for selection, picking, boolean pre-test (SAH, refit, traversal) | kernel-engineer | spec v1 (planned) |
| [tessellation.md](tessellation.md) | Adaptive UV tessellation, edge respect, smooth-group merging, LoD | kernel-engineer | spec v1 (planned) |
| [persistent-naming.md](persistent-naming.md) | Tag generation, survival under upstream change, broken-reference detection | kernel-engineer | spec v1 (planned) |
| [test-corpus.md](test-corpus.md) | Boolean fuzz / sketch / STEP / visual corpora — sizes, sources, gates | qa-engineer | spec v1 (planned) |
| [microbenchmarks.md](microbenchmarks.md) | Per-bench targets, gate policy, regression detection | qa-engineer | spec v1 (planned) |
| [chaos-tests.md](chaos-tests.md) | Failure injection (OOM, disk, GPU device-lost, plugin panic, …) | qa-engineer | spec v1 (planned) |

## Authoring conventions

1. **One algorithm per file.** Splitting prevents the "everything is in one massive file" trap.
2. **Self-contained.** Each file states its inputs, outputs, complexity, references; reader does not need to follow back to the roadmap.
3. **Math notation.** KaTeX in markdown; inline `$...$`, block `$$...$$`. Avoid ASCII-only when LaTeX is clearer.
4. **Pseudocode language-neutral.** Looks like Rust but does not need to compile; favours clarity.
5. **Failure modes table.** Every algorithm must enumerate its known failure modes, the detection criterion, and the response.
6. **Numerical guards.** Tolerance constants named, not magic numbers.
7. **References.** Primary literature first (book/paper, year, page); FreeCAD / OCCT references second.
8. **Versioning.** Top of each file: `Status: spec vN, last updated YYYY-MM-DD`. Bump on substantive change.
9. **Cross-link.** Each file links back to the roadmap phase that owns it (e.g., "Implements §16.4 fillet ladder for C-Solid v1.0–v3.0").
10. **No code listings of full implementations** — those go in the crate. These files are *specifications*, not implementations.

## Relationship to other docs

- `docs/COMMERCIAL_CAD_ROADMAP.md` — *what* and *when*. Source of truth for scope.
- `docs/algorithms/*.md` — *how* and *with what guarantees*. Source of truth for algorithm correctness.
- `docs/adr/*.md` — *why*. Architecture decision records (e.g., why we chose half-edge over winged-edge).
- `docs/wiki/*.md` — user-facing reference (per-crate API guide, tutorials).
- `docs/DEVELOPER_WIKI.md` — onboarding tour (one-stop overview for new contributors).
- `crates/*/tests/` — executable proof of compliance with these specs.

Last updated: 2026-05-06 (v3.3 split).
