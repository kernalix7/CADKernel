# STOP LIST — Scope Lock for Commercial CAD Push

**Active since 2026-05-11.** Lifted only by an explicit entry in [CHANGELOG.md](CHANGELOG.md) crediting a Commercial CAD Roadmap (`docs/COMMERCIAL_CAD_ROADMAP.md`) phase exit gate.

## Lift log

- **2026-05-16** — Item #1 (no new `Command`/`Outcome` variants) **LIFTED** for the mega-push at commits `814885a` → `ffbc310` (5-stage chain). Gate exit: **v1.0 Gate #29** (Part feature suite). Trust row green: `cargo clippy -p cadkernel-api -- -D clippy::unwrap_used -D clippy::panic` enforced in CI (deepens v0.5 Gate #12). 22 new Command variants admitted (8 sketch-driven + 4 dress-up + 6 body/feature-tree + 4 sketch persistence) + 3 new Outcome variants (`FeatureAdded`, `FeatureRecomputed`, `SketchCreated`). Downstream consumers unblocked: 11 viewer dispatcher arms in `crates/viewer/src/app.rs` (Pad/Pocket/Groove/Hole/CountersunkHole/Loft/Pipe/Fillet/Chamfer/Shell/Helix) migrated off direct kernel calls onto `Session::execute`. Workspace 3,382 → 3,662 tests / 0 / 1 ignored. **Item #1 remains lifted for any further Tier 5+ Command additions that block a v1.0 gate; the underlying discipline (no slop additions, real downstream consumer required) still applies.**

The point of this file is to keep the maintainer (and any AI agent operating on the repository) from re-creating the *V37 small-slice spiral* — every short tactical commit had near-zero marginal value vs. the actual commercial-readiness gates. Until the v0.5 / v1.0 gates close, the items below are off-limits.

## Off-limits until further notice

1. **No new `Command` / `Outcome` enum variants** in `crates/api/`. The API surface is feature-frozen at the May 11 2026 baseline (`Command::FirstOperation` was the last admitted slice). New variants only land if a real downstream consumer (script, MCP tool, plugin, viewer dispatcher) blocks on them.
2. **No new `cadkernel-api` "observer" / read-only slices.** Future read-only queries must be served by an existing observer (`Outcome::Stats`, `Outcome::Measured`, `Outcome::AabbSummary`, `Outcome::HistoryListed`, `Outcome::SolidsListed`). Composition over enumeration.
3. **No new file format crates.** Existing 11 formats (STL/OBJ/glTF/STEP/IGES/DXF/PLY/3MF/BREP/DWG/DAE) are the entire IO surface for v0.5. JT, Parasolid, 3D-PDF/PRC are explicit v2.0 / v3.0 gates per the roadmap.
4. **No new viewer workbenches.** The existing 9 (Part / PartDesign / Sketcher / Mesh / TechDraw / Assembly / Draft / Surface / FEM) close every parity-matrix row called for at v0.5 / v1.0. Sheet-metal / weldment / mold / CAM are v2.0+ per the roadmap.
5. **No new analytical surface or curve types** in `crates/geometry/`. The existing `Plane / Cylinder / Sphere / Cone / Torus / NURBS` families are sufficient for every v0.5 reference part. Subdivision (T-spline) is v2.0.
6. **No new sketch constraint types.** 24 constraints already implemented; the gap is the *solver UX* (DoF readout, drag-with-constraints performance, conflicting-constraint highlight) — not more constraint kinds.
7. **No new FEM analysis kinds.** Linear static / modal / thermal cover the v0.5 + v1.0 gates. Nonlinear, contact, CFD, plastics-flow are v2.0+ per the parity matrix.
8. **No new CAM / Sheet-metal / Mold / Weldment workbenches.** All four are v2.0+ exit gates.
9. **No "feature count" milestones.** Stop counting "576/576 features." That number conflated dispatcher reachability with end-to-end functionality and was materially misleading. Use the Commercial CAD Roadmap parity matrix (`docs/COMMERCIAL_CAD_ROADMAP.md` §1) as the only scoreboard.
10. **No git push without explicit user approval.** Branch is local-ahead-of-origin; phase work continues on local `main`.

## Why this exists

The May 11 2026 audit found that incremental `Outcome::*` slices (#32 → #48 over April–May) consumed multiple sessions while the v0.5 gates that actually unblock a "honest beta" remained partially open:

- Gate 5 (MCP server in `crates/mcp/`) — still embedded in `crates/io/`.
- Gate 9 / 10 (R1 / R2 build via API end-to-end with no direct kernel calls) — `examples/build_reference_parts.rs` was a stub returning `NotImplemented`.
- Gate 12 (panic-free public API) — clippy `unwrap_used` / `panic` not yet repo-wide.

The 13 remaining `Command` slice variants in the queue would not have closed any of those.

## How to lift an item

1. The exit gate is referenced by name in `docs/COMMERCIAL_CAD_ROADMAP.md` (e.g. "v1.0 Gate #16 — STEP AP242 PMI").
2. The gate's `Trust` row (clippy / panic / perf / determinism) is green.
3. `CHANGELOG.md` records the lift with the commit SHA that delivered the corresponding gate.

## Related documents

- [docs/COMMERCIAL_CAD_ROADMAP.md](docs/COMMERCIAL_CAD_ROADMAP.md) — canonical v3.5 roadmap, 60 non-negotiables.
- [docs/UI_COMPLETION_ROADMAP.md](docs/UI_COMPLETION_ROADMAP.md) — Track B feeder.
- [docs/FREECAD_PARITY_PLAN.md](docs/FREECAD_PARITY_PLAN.md) — Track C feeder (subordinate to roadmap).
- [docs/V36_BUG_TRIAGE.md](docs/V36_BUG_TRIAGE.md) — historical correctness audit (CLOSED at 2,590 / 0 / 0 in V36 R2c; **K1 / K3 / U1 are FIXED**, contrary to any older summary that lists them as deferred).
- [WORK_STATUS.md](WORK_STATUS.md) — current session state.
