# Commercial CAD Roadmap v3.5 — CADKernel

**Status:** Active. v3.5 "corpus + ADR depth + first executable seed" (2026-05-06): expanded the v3.4 docs with **5 more ADRs** (0011 automerge CRDT, 0012 wasm plugin sandbox, 0013 CBOR metadata, 0014 zstd compression, 0015 sparse Cholesky), **memory-profile** spec under [`docs/perf/memory-profile.md`](perf/memory-profile.md), **6 more golden TOML cases** (4 sketch: slot/hexagon/triangle-in-circle/parallel-redundant-tangent + 3 boolean: box∩box, sphere∩box, idempotent-union), **2 reference-part Lua build scripts** (R1 box, R2 extrude), and the **first compilable Rust seed** [`examples/build_reference_parts.rs`](../examples/build_reference_parts.rs) that builds R1-R12 and stubs the not-yet-wired pipeline. Prior milestones: v3.4 ADR/perf/corpus pillars, v3.3 algorithm modular split, v3.2 industrial-grade depth across all tracks, v3.1 UX deep-dive, v3 industrial-grade rewrite. Project documentation + corpus now 9,299 lines (3,169 roadmap EN + 287 KO + 2,717 algorithms + 1,005 ADR + 709 perf + 1,309 corpus + 103 example seed).
**Bilingual:** [한국어 요약](COMMERCIAL_CAD_ROADMAP.ko.md)
**Replaces:** v3.3, v3.2, v3.1, v3, v2, v1.
**Subsumed:** `UI_COMPLETION_ROADMAP.md` (now Track B feeder), `FREECAD_PARITY_PLAN.md` (now sub-tracks of Track C feeder).

---

## 0. What changed in v3

v2 set the right structural frame (4 tracks, 25 non-negotiables, reference parts) but stayed at the level of "we will ship a sketcher v2" without naming the algorithms, the data structures, the file-format byte layouts, the comparative target features against SolidWorks / CATIA / Fusion 360 / Inventor / FreeCAD, or the specific industry compliance benchmarks (NIST CAX, ASME Y14.5, AP242, JT 10.7, ISA 95). v3 fixes that.

Concretely:

- **Tracks expanded from 4 to 6.** Added Track E (Ecosystem) and Track F (Collaboration & Cloud) because plugin SDK, education / marketplace, and Onshape-style real-time co-editing are not Track B polish — they are first-class deliverables.
- **Track C exploded into 13 sub-tracks** (C-Solid, C-Sketch, C-Surface, C-Assy, C-Mech, C-Sheet, C-Weld, C-Mold, C-Draw, C-Sim, C-CAM, C-Render, C-Reverse). Each one is the scope of a workbench in commercial CAD; treating "Capability" as one track was the v2 mistake.
- **Non-negotiables 25 → 60.** Added: STEP AP242 + JT round-trip, GD&T per ASME Y14.5-2018, sheet-metal flat pattern parity, mate solver convergence, T-spline subdivision, generative design, CAM post-processor library, real-time collaboration with conflict-free replicated document, plugin sandboxing, education licence story, etc.
- **Reference parts R1-R4 → R1-R12.** Added: a sheet-metal enclosure, a weldment frame, a mold-tool kit, an injection-molded plastic part, a CAM-machined part with toolpaths, a generatively-designed bracket, an assembly with 100+ components, and a stress-test 10k-feature monolithic model.
- **Comparative parity matrix.** Every Phase exit gate cites named features in named commercial products and a target percentile (e.g., "≥ 90 % parity vs FreeCAD 1.0, ≥ 60 % parity vs SolidWorks 2026 in the matched feature subset").
- **Algorithm bibliography.** Every kernel phase cites the source paper / book / chapter (The NURBS Book, Hoffmann's _Geometric and Solid Modeling_, OCCT modeling kernel notes, FreeCAD developer wiki, _Geometric Tools for Computer Graphics_, _Real-Time Collision Detection_, _Robust and Error-Free Geometric Computing_).
- **Industry benchmarks.** NIST CAX round-trip suite, ASME Y14.41 model-based-definition, JT B-Rep Annex, AP242 conformance class, KCL / GD&T 2018, MIL-STD-31000B drawing standard, ISO 10303-242.
- **Concrete release calendar.** v0.5 (Q3 2026) — A2 + B1-B3 + C-Sketch v2; v1.0 (Q1 2027) — Tracks A+B core + C-Solid + C-Sketch + C-Assy + C-Draw v1 + Track D core; v2.0 (Q4 2027) — full sub-track set; v3.0 (Q3 2028) — full SW/Inventor parity.
- **Algorithm-by-algorithm compliance gates.** No "the booleans work." Specific gates: NIST FTC04 conformance, OCCT regression set, FreeCAD `Part::*` round-trip, ProtoTRAK G-code post-processor regression.
- **IP / licensing audit per dependency.** Apache-2.0 compatibility checklist; explicit GPL avoidance; specific calls on whether to add `libnurbs`, `OpenCASCADE`, `cgal`, `boost-geometry`, etc.

The headline ambition: **at v1.0, we are a credible FreeCAD competitor; at v2.0, we are an Onshape-class cloud CAD; at v3.0, we are a SolidWorks/Inventor-class desktop CAD with measurable parity gates against named commercial products.**

This is not a feature wishlist. Every item has an exit gate, an algorithm, a benchmark, and a tracked release.

---

## 1. Comparative parity matrix

Every Phase below maps to one or more cells of this matrix. The "us-target" column is the percentile of feature parity at v1.0 / v2.0 / v3.0.

| Capability cluster | FreeCAD 1.0 | SolidWorks 2026 | CATIA V6 | Fusion 360 (2026) | Inventor 2026 | us @ v1.0 | us @ v2.0 | us @ v3.0 |
|---|---|---|---|---|---|---|---|---|
| Sketcher: well-determined detection | partial | full | full | full | full | full | full | full |
| Sketcher: drag with constraints | weak | strong | strong | strong | strong | strong | strong | strong |
| Sketcher: external references | partial | full | full | full | full | partial | full | full |
| Sketcher: spline G2 / G3 | partial | full | full | partial | partial | partial | full | full |
| Part features: extrude / revolve / sweep / loft | full | full | full | full | full | full | full | full |
| Part features: helix / coil / thread | partial | full | full | full | full | partial | full | full |
| Part features: ribs / drafts / shell | partial | full | full | full | full | partial | full | full |
| Part features: variable-radius / asymmetric fillet | partial | full | full | partial | partial | partial | full | full |
| Part features: dome / freeform deformation | none | full | full | full (T-spline) | full | none | partial | full |
| Surface: ruled / blend / boundary / fill | partial | full | full | full | full | partial | full | full |
| Surface: Class-A (G2 reflection-line clean) | none | partial | full | partial | partial | none | partial | full |
| Subdivision (T-spline) | none | partial | full | full | partial | none | partial | full |
| Sheet metal: flange / hem / jog / corner | weak | full | full | full | full | partial | full | full |
| Sheet metal: K-factor / gauge tables | partial | full | full | full | full | partial | full | full |
| Weldments: structural members / cut lists | partial | full | full | partial | full | none | partial | full |
| Mold tooling: parting / core / cavity | partial | full | full | partial | partial | none | partial | full |
| Assembly: full mate set | partial | full | full | full | full | partial | full | full |
| Assembly: motion / mechanism | partial | full | full | partial | full | none | partial | full |
| Assembly: configurations / design tables | none | full | full | full | full | none | partial | full |
| Drawing: GD&T per ASME Y14.5-2018 | partial | full | full | full | full | partial | full | full |
| Drawing: BOM with revisions | partial | full | full | full | full | partial | full | full |
| Drawing: model-based definition (Y14.41) | none | full | full | partial | partial | none | partial | full |
| FEM: linear static / modal | partial | full | full | partial | full | partial | full | full |
| FEM: nonlinear / contact | none | full (Sim Pro) | full (Abaqus link) | partial | partial | none | partial | full |
| FEM: thermal / coupled | partial | full | full | partial | partial | partial | full | full |
| CFD: pipe-flow / external flow | none | full (Flow) | full | partial (CFD ext) | none | none | partial | partial |
| Plastics flow simulation | none | full (Plastics) | full (Moldflow link) | none | none | none | none | partial |
| CAM: 2.5-axis milling | none | full (HSMWorks) | full | full | full | none | partial | full |
| CAM: 5-axis milling | none | partial | full | partial | partial | none | none | partial |
| CAM: turning / multi-turret | none | partial | full | partial | partial | none | none | partial |
| Rendering: PBR + IBL + shadows | partial | full (Visualize) | full | full | full | none | partial | full |
| Rendering: ray-traced photoreal | none | full | full | full | partial | none | none | full |
| Rendering: walkthrough / VR | none | partial | full | partial | partial | none | partial | full |
| Reverse engineering: mesh → surface | partial | partial | full | full | partial | none | partial | full |
| Topology optimization | partial | full | full | full (Generative) | partial | none | partial | full |
| File: native format with schema migration | partial | full (.sldprt) | full (.CATPart) | full (.f3d cloud) | full (.ipt) | full (.cadk) | full | full |
| File: STEP AP214 / AP242 round-trip | partial | full | full | full | full | partial | full | full |
| File: JT 10.7 read/write | none | full | full | partial | partial | none | partial | full |
| File: Parasolid (.x_t / .x_b) read | none | native | partial | partial | partial | none | partial | partial |
| File: 3D PDF with PRC | none | full | full | partial | partial | none | partial | full |
| Collaboration: real-time co-editing | none | partial (Cloud) | partial (3DEXP) | full (cloud) | partial | none | partial (CRDT) | full |
| Collaboration: PDM + revisions / ECO | partial | full (PDM Pro) | full | partial | full (Vault) | none | partial | full |
| API: scripting | full (Python) | full (VBA + .NET) | full (CAA) | full (Python) | full (iLogic) | full (Rust+Lua+Python+JSON-RPC) | full | full |
| API: AI agent integration (MCP) | none | none | none | none | none | full | full | full |
| Plugin SDK | full (Python WB) | full (.NET) | full (CAA C++) | full (manifests) | full (.NET) | partial | full | full |
| i18n locales shipped | many | many | many | many | many | en + ko | en + ko + ja + zh | en + ko + ja + zh + de + fr + es + ru |
| a11y (WCAG AA) | weak | weak | weak | partial | weak | full | full | full |
| Distribution: signed installer auto-update | partial | full | full | full (cloud-only) | full | none | full | full |

**The matrix is the contract.** Every phase exit gate names the row(s) it advances. v1.0 must hit every "us @ v1.0" cell. v2.0 must hit every v2.0 cell. v3.0 closes the matrix.

---

## 2. The 60 non-negotiables

Each is an exit gate for the version it's tagged to. Categories: Data (D), API (A), UX (U), Capability (C), Trust (T), Ecosystem (E), Cloud/Collab (X).

### v0.5 gates (12) — "honest beta"

| # | Cat | Requirement |
|---|---|---|
| 1 | D | `cadkernel-api` crate stable; `Command` enum SemVer-locked at minor level |
| 2 | D | Document-level undo/redo with replay-based rebuild |
| 3 | D | `Session::save_to_json` / `load_from_json` round-trip |
| 4 | A | Schema for every `Command` variant auto-generated, JSON-validated in CI |
| 5 | A | MCP server delegates to `Session::execute` (extracted into `crates/mcp/`) |
| 6 | U | Selection works at all camera distances for B-Rep faces / edges / vertices |
| 7 | U | Property panel two-way binding for primitive parameters |
| 8 | U | Sketcher shows DoF readout; drag respects all constraints |
| 9 | C | R1 (bracket) builds end-to-end via API with no direct kernel calls |
| 10 | C | R2 (housing) builds and recomputes on dimension change |
| 11 | T | First-paint < 500 ms; R1 open < 250 ms |
| 12 | T | All public-API panic-free; clippy `unwrap_used` + `panic` denied |

### v1.0 gates (24) — "credible FreeCAD competitor"

| # | Cat | Requirement |
|---|---|---|
| 13 | D | `.cadk` native binary file format with schema versioning + migrations |
| 14 | D | Persistent naming survives save / reopen / upstream-feature edit |
| 15 | D | Crash-safe autosave with on-startup recovery dialog |
| 16 | D | STEP AP214 round-trip ≥ 0.999 face-area, ≤ 1e-6 vertex match on R1-R3 |
| 17 | D | NIST CAX-IF round-trip suite passes for B-Rep + assembly |
| 18 | A | Branching undo; named checkpoints; history scrubbing |
| 19 | A | Document v2: feature graph with topological recompute scheduler |
| 20 | A | Recompute caching by parameter hash; unchanged subtree skip |
| 21 | U | Navigation cube + view gizmo + section box + exploded view |
| 22 | U | Sketcher v2 with in-place dimension editing + drag handles + external refs |
| 23 | U | History panel + feature tree (suppress / activate / reorder / branch UI) |
| 24 | U | Multi-document tabs + drag-drop import/open |
| 25 | U | i18n: en + ko shipped at parity; AccessKit landmarks; high-contrast theme |
| 26 | U | Theming: light / dark / system-follow persisted |
| 27 | U | Command palette (`Ctrl-K`) covers every `Command` |
| 28 | C | Sketcher v3 solver: well-determined / under / over classification + deflation |
| 29 | C | Part feature suite: extrude, revolve, sweep, loft, helix, coil, threaded hole, rib, draft, shell — **CLOSED 2026-05-16 (mega-push 814885a..ffbc310)** |
| 30 | C | Pattern: linear, circular, sketch-driven, table-driven, mirror, fill |
| 31 | C | Assembly with full mate set + DoF analysis + motion preview |
| 32 | C | TechDraw v2 with multi-view + section + detail + GD&T-2018 frames + BOM |
| 33 | T | 30 fps on R3 at 1080p on a 2020 reference laptop |
| 34 | T | 1 M-pair boolean fuzz, zero crashes across two consecutive runs |
| 35 | T | Reproducible builds; signed installers Linux / macOS / Windows; auto-update |
| 36 | T | Public CVE response policy; `cargo deny` / `cargo vet` / `cargo public-api` gates |

### v2.0 gates (16) — "Onshape-class cloud CAD"

| # | Cat | Requirement |
|---|---|---|
| 37 | D | STEP AP242 round-trip with PMI (product manufacturing information) |
| 38 | D | JT 10.7 read with B-Rep Annex; write at lossless quality level |
| 39 | D | 3D PDF export with PRC for CAD viewers |
| 40 | C | Sheet metal: base flange, edge flange, miter flange, hem, jog, fold/unfold, flat pattern, K-factor table |
| 41 | C | Weldments: structural member library, gussets, end caps, weld beads, cut lists |
| 42 | C | Surface: Class-A blends with G2 continuity, reflection-line analysis |
| 43 | C | T-spline subdivision freeform |
| 44 | C | Configurations & design tables (parametric variants) |
| 45 | C | FEM nonlinear: contact, large deformation, plasticity |
| 46 | C | CAM 2.5-axis milling with library of post-processors (Haas, Fanuc, Mazak, Heidenhain) |
| 47 | C | Reverse engineering: mesh-to-surface auto-fit |
| 48 | C | Topology optimization (SIMP / level-set) |
| 49 | E | Plugin SDK with Wasm sandboxing; published plugin marketplace |
| 50 | E | Education licence + tutorial library |
| 51 | X | Real-time co-editing via CRDT; user presence indicators |
| 52 | X | Cloud document storage with revisions / branches / diff visualization |

### v3.0 gates (8) — "SolidWorks/Inventor parity"

| # | Cat | Requirement |
|---|---|---|
| 53 | C | Mold tooling: parting line/surface, core, cavity, ejector pins, slides |
| 54 | C | CAM 5-axis simultaneous + multi-turret turning + Swiss-style + post-processor coverage |
| 55 | C | Plastics flow simulation (Moldflow-class) |
| 56 | C | CFD lite (pipe-flow + external aerodynamics) |
| 57 | C | Generative design (load-driven topology synthesis) |
| 58 | C | Photo-realistic path-traced rendering with decal library |
| 59 | C | Walkthrough / VR mode for assembly review |
| 60 | X | PDM with revisions, ECO workflow, where-used graph, reference checking |

---

## 3. Reference parts R1-R12

Every track exit-gate cites at least one. Stored as recorded `Vec<Command>` and `.cadk` snapshot under `tests/reference_parts/`. Build scripts in `tests/reference_parts/build_R*.rs`. Each is a CI fixture: scripted build, asserted hash, asserted timing, asserted face/edge/vertex counts, asserted bounding box. Drift in any quantity blocks merge.

### Summary

| ID | Name | Features | Domain | First used by | Build target | Memory target |
|---|---|---|---|---|---|---|
| **R1** | Bracket (small) | ~12 | Solid + sketch | A2 | < 200 ms | < 50 MB |
| **R2** | Housing | ~30 | Solid + face-on-face sketch | A3 | < 1 s | < 100 MB |
| **R3** | Gearbox assembly | 3 × ~50 | Assembly + drawings | C3, C4 | < 3 s | < 2 GB |
| **R4** | Stress part | 200-face solver edge case | Solver + perf | D1 | < 5 s | < 500 MB |
| **R5** | Sheet-metal enclosure | flange + flat pattern | Sheet metal | C-Sheet | < 1.5 s | < 200 MB |
| **R6** | Weldment frame | 12-member structural | Weldment | C-Weld | < 2.5 s | < 300 MB |
| **R7** | Mold-tool kit | core + cavity + ejector | Mold tooling | C-Mold | < 8 s | < 1 GB |
| **R8** | Injection-molded enclosure | shelled + draft | Plastics + analysis | C-Sim | < 3 s | < 500 MB |
| **R9** | CAM machined part | 2.5-axis toolpath | CAM | C-CAM | < 5 s | < 500 MB |
| **R10** | Generative bracket | topology-opt + STL | Generative + sim | C-Sim, C-Reverse | < 60 s | < 2 GB |
| **R11** | Big assembly | 250-component | Assembly stress | C3, D1 | < 10 s | < 3 GB |
| **R12** | Monolithic 10k-feature part | 10k holes scripted | Stress / scale | A5, D1 | < 30 s lazy | < 4 GB lazy |

### R1 — Bracket (small)

A mounting bracket. The simplest non-trivial part. Used as the smoke-test for every PR and the first tutorial.

**Geometry:** L-bracket; 80 mm × 60 mm × 4 mm. Two slots (10 mm × 4 mm) on the long flange. Four 4.2 mm counterbored holes in 2 × 2 grid on the bottom flange. Edge fillets 2 mm. One linear pattern of M3 thread chamfer in 1 × 4.

**Feature script:**

1. Sketch on XY plane: rectangle 80 × 60, dimensions.
2. Pad 4 mm.
3. Sketch on top face: 4 circles ø 4.2 mm in 2 × 2 grid, dimensions.
4. Pocket through-all.
5. Sketch on top face: 2 slots (10 × 4) at one end.
6. Pocket through-all.
7. Linear pattern of pad #2's underlying counterbore feature, 1 × 4.
8. Edge fillet 2 mm on selected edges.

**Asserted invariants:**

- Face count: 26.
- Edge count: 56.
- Vertex count: 36.
- Volume: 14_837 mm³ ± 0.01 %.
- Centroid (x, y, z): (40.0, 30.0, 1.83) mm ± 0.001.
- Bounding box: (80, 60, 4) mm.
- canonical_hash: pinned in `tests/reference_parts/R1.hash`.

**Build target:** < 200 ms cold, < 50 ms warm-cache. Memory < 50 MB.

**Used by:** A2 (smoke), B1 (tutorial), B11 video #1, C-Solid v1 gate, D1 perf gate, D3 visual regression baseline.

### R2 — Housing

A cylindrical housing with mounted flange and fins. Tests revolve + bosses + face-on-face sketch + linear pattern of fins.

**Geometry:** ø 100 mm × 120 mm tall cylinder; rim flange ø 130 mm × 8 mm at top; 6 cooling fins (50 × 3 × 30 mm each, linear pattern around axis); pocket on side face; internal rib for stiffness.

**Feature script:** ~30 features including: revolve sketch → flange boss → fin sketch → fin pad → circular pattern of fins → side pocket → internal rib → 4 mounting holes → fillets.

**Asserted invariants:** face/edge/vertex counts pinned; volume ± 0.01 %; canonical_hash pinned.

**Build target:** < 1 s cold, < 200 ms recompute on dim change. Memory < 100 MB.

**Used by:** A3 selection on rim, B2 (manipulator), C-Solid v1 gate, D1 perf gate.

### R3 — Gearbox assembly

Three-component assembly: housing (R2-class), motor mount, output shaft. Each component ~50 features. Drawing with sectioned views and GD&T. Sketch-driven recompute: changing one parameter on housing propagates through assembly mates.

**Components:**

1. Gearbox housing: revolve + bosses + fins + pocket. ~50 features.
2. Motor mount plate: 4-bolt mount + central hole. ~15 features.
3. Output shaft: stepped cylinder with keyway. ~10 features.

**Mates:** concentric (housing axis to mount centre), coincident (housing top face to mount bottom face), parallel (mount edge to housing edge), distance (shaft offset).

**Drawing:** front view + sectioned side view + isometric. Dimensions: Øs, distances, threads. GD&T: position tolerance, perpendicularity, surface finish callouts.

**Asserted invariants:** assembly mate solver converges in < 100 ms; drawing renders in < 500 ms; canonical_hash pinned.

**Build target:** < 3 s cold; < 500 ms recompute on parameter change. Memory < 2 GB.

**Used by:** C-Assy v1 gate, C-Draw v1 gate, B11 video #4-#6, D1 perf gate.

### R4 — Stress part

A single-solid stress test for the kernel: 200 faces, near-tangent fillets, two booleans sharing an edge, 80-constraint sketch near DoF=0. Designed to break naive solvers and naive boolean implementations.

**Geometry:** organic-shaped block with: 80-constraint sketch (2 over-determined paths automatically deflated to determine), 40 fillets including 6 near-tangent (radius approaches edge length), 2 boolean operations with shared edge between operands, 1 sweep along non-planar guide.

**Failure modes covered:**

- Sketch solver under-determined detection.
- Sketch solver over-determined deflation.
- Boolean shared-edge robustness (Yap symbolic perturbation).
- Fillet rolling-ball failure on tight curvature.
- NURBS surface-surface intersection convergence.

**Asserted invariants:** all 5 failure modes resolved without crash; canonical_hash pinned; face count = 200.

**Build target:** < 5 s cold. Memory < 500 MB.

**Used by:** C-Solid v1 gate (boolean robustness), C-Sketch v3 gate (solver), D1 perf gate, D2 robustness fuzz seed corpus.

### R5 — Sheet-metal enclosure

A standard sheet-metal box with flanges, hems, jogs, and a flat pattern. Tests sheet-metal feature suite end-to-end including K-factor unfold.

**Geometry:** 200 × 150 × 80 mm enclosure, 1.5 mm steel sheet. Base flange + 4 edge flanges (90°) + 2 miter flanges at corners + 2 hems on top edges + 4 jogs for stiffening ribs.

**Feature script:** base flange → edge flange × 4 → miter flange × 2 → hem × 2 → jog × 4 → flat-pattern feature.

**Asserted invariants:** flat pattern unfolds to single planar face; flat-pattern bounding box matches K-factor calculation ± 0.5 %; canonical_hash pinned.

**Build target:** < 1.5 s cold. Memory < 200 MB.

**Used by:** C-Sheet v2 gate, B11 video sheet-metal tutorial.

### R6 — Weldment frame

A 12-member rectangular structural frame with gussets, end caps, weld beads, and a cut list. Tests weldment workbench.

**Geometry:** 1 m × 0.6 m × 0.4 m rectangular frame using 40 × 40 × 3 mm steel SHS profile. 12 members (4 long, 4 wide, 4 tall). 4 corner gussets. 8 end caps. Welded joints with bead profiles.

**Feature script:** 3D sketch defining frame skeleton → structural member feature (12 instances) → trim/extend at corners → gusset feature × 4 → end cap × 8 → weld bead at every shared edge.

**Asserted invariants:** cut list (member length / count / total mass) auto-generated; values match calculation ± 0.1 %; canonical_hash pinned.

**Build target:** < 2.5 s cold. Memory < 300 MB.

**Used by:** C-Weld v2 gate, B11 video weldment tutorial.

### R7 — Mold-tool kit

A plastic part with parting surface, core / cavity blocks, and ejector pins. Tests mold tooling workbench.

**Geometry:** plastic phone-case-class part (120 × 70 × 10 mm with rounded corners, button cutouts, lanyard hole). Mold tool: parting line (auto-detected on draft analysis) → parting surface → core block → cavity block → 4 ejector pins.

**Feature script:** import plastic part → draft analysis → parting line auto-detect (manual override allowed) → parting surface generation → mold base placement → core/cavity split → ejector pin placement.

**Asserted invariants:** core + cavity ∪ part = mold base (boolean closure); ejector pins clear part by ≥ 0.1 mm; canonical_hash pinned.

**Build target:** < 8 s cold. Memory < 1 GB.

**Used by:** C-Mold v3 gate.

### R8 — Injection-molded enclosure

A shelled organic-shape enclosure with draft analysis and thin-wall detection. Tests plastics validation.

**Geometry:** 150 × 80 × 30 mm rounded enclosure, 2 mm wall thickness via shell. Internal bosses with draft. Snap-fit features.

**Asserted invariants:** wall thickness uniform ± 5 %; draft ≥ 1° on all faces requiring it; thin-wall warning fires on regions < 1 mm; canonical_hash pinned.

**Build target:** < 3 s cold. Memory < 500 MB.

**Used by:** C-Sim v3 gate (plastics analysis), C-Solid (shell feature).

### R9 — CAM machined part

R1 (bracket) with 2.5-axis toolpaths added: face mill + contour + pocket + drill cycles. Post-processed G-code for 3-axis Haas VF-2. Simulation playback.

**Toolpath features:** face mill operation, 2D contour, 2D pocket, drill cycle, helical entry. Tools: ø 12 mm face mill, ø 6 mm flat end mill, ø 4 mm drill.

**Asserted invariants:** G-code parses cleanly through LinuxCNC interpreter; simulation removes correct material volume ± 0.5 %; cycle time estimate ± 5 % vs reference; canonical_hash pinned.

**Build target:** < 5 s cold. Memory < 500 MB.

**Used by:** C-CAM v2 gate, B11 video CAM tutorial.

### R10 — Generative bracket

Load case (force vector + fixed faces) on a design space; topology optimisation produces a synthesised bracket; export to STL for 3D printing.

**Setup:** design space 100 × 60 × 40 mm; load 500 N at one end; fixed at four mounting holes; volume fraction target 30 %; minimum feature size 2 mm; SIMP penalty parameter 3.

**Asserted invariants:** result manifold; volume fraction ± 2 %; max stress in result ≤ yield (steel); STL export valid (no non-manifold edges); canonical_hash pinned.

**Build target:** < 60 s cold (topology opt is iterative). Memory < 2 GB.

**Used by:** C-Sim v3 (topology-opt) gate, C-Reverse (mesh-to-surface) gate.

### R11 — Big assembly

250-component motor assembly with mates, motion, exploded view, BOM, drawing. Stress test for assembly viewer + recompute scheduler.

**Components:** motor housing, end-bell, rotor, stator, 50 windings, shaft, bearings (× 2), 4 mounting brackets, 100 fasteners, terminal box with 8 connectors. Total 250+ components.

**Mates:** ~400 mate constraints; full DoF analysis; motion: rotor rotation about shaft axis.

**Drawing:** 4 views + exploded BOM with auto-generated balloons.

**Asserted invariants:** assembly opens in < 10 s; mate solver converges in < 2 s; navigation 60 fps with LoD enabled; BOM count exact; canonical_hash pinned.

**Build target:** < 10 s cold; 60 fps view with LoD. Memory < 3 GB.

**Used by:** C-Assy v3 gate, D1 perf gate, B11 video assembly tutorial.

### R12 — Monolithic 10k-feature part

Scripted: 10 000 ø 5 mm holes drilled in a 1 m × 1 m × 20 mm plate in a 100 × 100 grid. Tests scheduler / cache / undo at scale.

**Feature script:** plate base → 100 × 100 linear pattern of holes (a single pattern feature, but expanded internally to 10k instances).

**Asserted invariants:** lazy-load: only viewport-visible features loaded into RAM; recompute on parameter change incremental (only changed instances); undo of pattern modification < 200 ms; canonical_hash pinned.

**Build target:** < 30 s cold lazy load. Memory < 4 GB lazy-loaded.

**Used by:** A5 (incremental recompute scheduler), D1 perf gate (scale).

---

## 4. Track A — Platform

Foundation. Every other track stands on Track A. A1 done; A2 in flight; A3-A8 sequenced.

### A1 — `cadkernel-api` v0 [done, 2026-05-06]

**Goal:** all client-visible surface area collapses to a single API crate, replacing 4-way ad-hoc paths (modeling-direct, viewer-direct, MCP-direct, IO-direct).

**Design:**

```rust
pub struct Session {
    document: Document,
    log: Vec<LoggedCommand>,        // append-only command log
    cursor: usize,                  // for undo/redo
    snapshots: Vec<SessionSnapshot>,// optional cached states
    autosave: Option<AutosavePolicy>,
    listeners: Vec<Box<dyn SessionListener>>,
}

pub enum Command {
    NewDocument { name, units, default_plane },
    CreatePrimitive(PrimitiveSpec),
    Boolean(BooleanSpec),
    Transform(TransformSpec),
    DeleteSolid { tag },
    StartSketch { plane },
    AddSketchEntity(SketchEntitySpec),
    AddConstraint(ConstraintSpec),
    Dimension(DimensionSpec),
    FinishSketch,
    Extrude(ExtrudeSpec),           // [A2]
    LinearPattern(LinearPatternSpec),// [A2]
    Mirror(MirrorSpec),             // [A2]
    EditFeature { id, params },     // [A5]
    SetParameter { name, expression },// [A5]
    /* ... 30+ variants by v1.0 ... */
}

pub enum Outcome {
    SolidCreated { tag, hash },
    BooleanComplete { tag, hash, removed: Vec<Tag> },
    SketchStarted { id },
    Recomputed { affected: Vec<FeatureId>, ms: f64 },
    /* ... matched 1:1 to Command variants ... */
}

pub trait SessionListener: Send + Sync {
    fn on_command(&mut self, cmd: &Command, outcome: &Outcome);
    fn on_progress(&mut self, event: &ProgressEvent);
    fn on_error(&mut self, err: &ApiError);
}
```

**Status (2026-05-06):** crate `crates/api/`. 14 `Command` variants. 17 integration tests + 2 doc tests. JSON round-trip per variant via `serde_json`. Used by 2 callers (viewer Lua bridge half-migrated, MCP server pre-A6 still on legacy path).

**Documentation:** README plus rustdoc on every public item. Examples for 8 common command flows (`docs/api/cookbook.md`).

### A2 — Phase 2A slice (in flight, ~70 % through)

**Goal:** complete the v0.5 `Command` set so reference part R1 (a 50 mm steel bracket with 2 mounting-hole patterns + a draft + a fillet) is fully buildable through `Session::execute` only.

**Deliverables:**

1. `Command::Extrude(ExtrudeSpec)` — supports blind / through-all / up-to-face / mid-plane / two-sided. `ExtrudeSpec { sketch_id, depth: ExtrudeDepth, direction: ExtrudeDirection, draft: Option<f64>, taper: Option<f64>, merge: bool }`.
2. `Command::LinearPattern(LinearPatternSpec)` — `{ source: FeatureId, axis: Axis, count: u32, spacing: f64, mirror_alternate: bool, instance_overrides: HashMap<usize, FeatureOverride> }`.
3. `Command::Mirror(MirrorSpec)` — `{ source: Vec<FeatureId>, plane: PlaneRef, merge: bool }`.
4. `Outcome::PatternCreated { pattern_id, instance_count, total_features }`.
5. `Document::history: Vec<HistoryEvent>` — every successful execute + undo/redo + save/load logged.
6. `Session::undo()` / `redo()` — log-based; supports up to N coalesced edits in a 1 s window for property fields.
7. `Session::save_to_json(path)` / `load_from_json(path)` — temporary JSON serialisation pre-A3 binary `.cadk`.
8. `SessionSnapshot { document_hash, log_position, timestamp, label }`.
9. Migration of viewer Lua bridge (`crates/viewer/src/scripting.rs`) to call `Session::execute` exclusively.

**Test inventory targeted:**

- 14 unit tests for new specs.
- 6 integration tests R1 build, edit, undo, redo, save, reload.
- 2 property tests (`proptest`) for pattern instance overrides.
- 1 doc test per spec type.
- Workspace verification: `cargo build --workspace && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace`.

**Exit gate:** R1 builds via `Session::execute` only; all undo/redo + save/load tests green; `cadkernel-api` re-exports clean; workspace verification all-green; `cargo public-api` baseline pinned to v0.1.0; CHANGELOG / docs/CHANGELOG.ko.md / DEVELOPER_WIKI / DEVELOPER_WIKI.ko updated.

### A3 — `.cadk` native file format

**Goal:** lossless round-trip for every Document state, with explicit schema versioning. The contract that turns CADKernel from "research demo" into "trustable application."

**Format design:**

```text
Layout (binary, little-endian):
  +----------------+----------------+----------------------+----------------+
  | magic[4]       | header[64]     | manifest_blob        | content_blobs  |
  | "CADK"         | (versioning,   | (toc, hashes, sizes) | (zstd-frames)  |
  |                |  flags, crc32) |                      |                |
  +----------------+----------------+----------------------+----------------+
```

- Magic bytes `0x43 0x41 0x44 0x4B` (`"CADK"`).
- Header: schema version (u32), feature flags (u32), endianness mark, total content size, crc32 of manifest, reserved.
- Manifest: ordered list of content blobs, each with `(kind, name, offset, length, checksum)`. Kinds: `document`, `thumbnail`, `history`, `attachments[]`, `signature`.
- Content blobs: `bincode 2`-encoded bodies, individually `zstd`-compressed (level configurable). Per-blob crc32.
- Optional signature blob carries an Ed25519 signature over the manifest hash for integrity-proof distribution.
- Forward compat: unknown blob kinds are preserved verbatim and re-emitted on save.
- Backward compat: every Document field is `Option<T>` for one minor version before promotion to required; `migrate_v{n}_to_v{n+1}` fns live in `crates/api/src/migrate.rs`.

**Deliverables:**

1. `crates/api/src/cadk/{header.rs, manifest.rs, blob.rs, codec.rs, migrate.rs, autosave.rs}`.
2. `Document::save(path, options)` / `Document::open(path)` with `SaveOptions { compression: ZstdLevel, embed_thumbnail: bool, sign_with: Option<Ed25519PrivateKey> }`.
3. `cadk-inspect` CLI (`crates/api/src/bin/cadk_inspect.rs`) — dumps header, manifest, per-blob hex preview.
4. Schema migration test corpus under `tests/fixtures/cadk-v0/`, `cadk-v1/`, etc. CI loads each historical version and migrates forward.
5. Autosave: `Session::set_autosave(AutosavePolicy { interval, path, compress, retain })` writes every N seconds **and** on every successful command (debounced 500 ms). Old autosaves rotated, max-N retained.
6. Crash recovery: on startup, viewer scans autosave dir for unflushed snapshots, presents recovery dialog with thumbnail + last-event description.
7. Thumbnail: viewport snapshot at save, embedded as 256×256 PNG.

**Acceptance:** R1 + R2 + R3 round-trip vertex-for-vertex (`assert_eq` on `Document::canonical_hash()`); v0→v1 migration test passes; crash-mid-edit recovery test (kill `cargo run` mid-`execute`, restart, recover → state matches last successful Outcome).

**Risks (highest in roadmap):** once shipped, we cannot remove a field without breaking files in the wild. Mitigation: every field is `Option<T>` for one minor; `cargo public-api` gates schema changes; migration tests are required.

**Algorithm references:** `bincode 2` (v2.0+, varint encoding), `zstd 1.5+` reference dictionary, RFC 7693 (BLAKE2 considered, used SHA-256 for FIPS-friendliness), Ed25519-dalek 2.x.

### A4 — STEP AP214 + AP242 round-trip

**Goal:** R1, R2, R3, R5, R6 export to STEP, reimport, and reach **≥ 0.999 face-area match**, **≤ 1e-6 vertex match**, identical topology counts; PMI / GD&T survives in AP242.

**Deliverables:**

1. **AP214 audit** — current `cadkernel-io::step` re-audited against ISO 10303-214:2010. Gap analysis posted as `docs/STEP_AUDIT.md`.
2. **AP242 extension** — schema upgraded to ISO 10303-242:2014 with PMI annex (geometric tolerances, datum references, surface finish, dimensions in 3D). `crates/io/src/step/ap242.rs`.
3. **NIST CAX-IF compliance** — pass the test suite published by NIST CAD Test Center (`https://www.nist.gov/el/systems-integration-division-73400/cad-test-center`) for B-Rep entities + assemblies.
4. **CADExchanger interop** — round-trip via CADExchanger SDK (or open-source `step-exchanger`) demonstrates parity with commercial tools.
5. **Tolerance configuration** in `Document::settings`. Exporter respects `linear_tolerance`, `angular_tolerance`, `b_spline_continuity_tolerance`.
6. **Diff tool** `crates/io/src/bin/step_diff.rs` — load two STEP files, build entity-id correspondence by topology hash, report face-area / vertex / edge-length diff.
7. **CI gate** — every primitive + extrude/revolve/sweep/loft + boolean + R1-R3 + R5-R6 round-trip ≥ 0.999.

**Acceptance:** named industry parts from `tests/step_corpus/` (a NASA bracket, an aerospace clip, an automotive housing) round-trip at tolerance.

**Algorithm refs:** _The NURBS Book_ Ch. 9 (refinement / decomposition); Hoffmann _Geometric and Solid Modeling_ Ch. 6 (B-Rep validation); ISO 10303-242:2020 Part 1 + Annex A (PMI).

### A5 — Document v2: parametric & topological

**Goal:** Document v2 is a directed acyclic graph of features, not a flat list. Editing an upstream feature recomputes downstream consistently and incrementally.

**Data model:**

```rust
pub struct Document {
    features: Arena<Feature>,
    feature_order: Vec<FeatureId>,            // topological order
    edges: Vec<(FeatureId, FeatureId)>,       // dataflow
    sketches: Arena<Sketch>,
    drawings: Arena<Drawing>,
    assemblies: Arena<Assembly>,
    expressions: ExpressionTable,             // parametric variables
    history: Vec<HistoryEvent>,
    settings: DocumentSettings,
    persistent_names: PersistentNameTable,    // (FeatureId, EntityKind, local) -> Tag
    cache: RecomputeCache,                    // hash(feature_id, params, inputs) -> output
}
```

**Deliverables:**

1. **Recompute scheduler** — Tarjan SCC for cycle rejection; topological sort by Kahn's algorithm; dirty-propagation BFS from edited node.
2. **Parameter hashing** — `Feature::input_hash() -> Hash` covers all params + transitive input hashes. Cache hit ⇒ skip recompute; output reused from cache.
3. **Persistent naming through recompute** — extend `crates/topology/src/persistent_name.rs` with a `(FeatureId, OperationId, EntityKind, local_index)` axis. Hash-name resolver re-binds across regeneration.
4. **Cycle detection** — adding an edge that creates a cycle returns `ApiError::CycleInDataflow { involved: Vec<FeatureId> }`.
5. **Expression engine** — `crates/expr/` new crate. Lex / parse / type-check / eval. Supports `2*pi*r`, conditionals, unit-aware arithmetic. Used by parameter fields and design tables.
6. **Variable scope** — global (Document) and local (per-feature). Scoping rules documented.
7. **Recompute progress events** — `Session::on_progress(callback)` emits `ProgressEvent { feature, phase, percent }`. Used by Track B for non-blocking progress UI.

**Acceptance:** R2 recompute on dimension change < 200 ms; R12 (10k features) recompute on a single-feature edit dirties at most O(downstream) features; persistent-name test passes save → reopen → upstream-edit → downstream still resolves.

**Algorithm refs:** Tarjan 1972 (SCC); Kahn 1962 (topological sort); Lengauer-Tarjan dominators (for branch undo); Hoffmann Ch. 8 (history-based modeling); Bentley-Ottmann (interval trees for caching); Halstead 1977 (incremental dataflow).

### A6 — MCP server rewrite on `Session`

**Blocking issue (v2 documented):** `cadkernel-modeling → cadkernel-io` already exists, so `cadkernel-io` cannot import `cadkernel-api`. Resolution:

1. Extract MCP into new crate `crates/mcp/` depending on `cadkernel-api` and `cadkernel-io::format` adapters.
2. Legacy `cadkernel-io::mcp` retained one minor under `cfg(feature = "legacy-mcp")` with deprecation banner.

**Deliverables:**

1. **JSON-RPC 2.0** server with full tool catalog: `create_primitive`, `boolean_operation`, `transform`, `query_model`, `measure`, `export_model`, `delete_solid`, `list_solids`, `extrude`, `linear_pattern`, `mirror`, `undo`, `redo`, `save`, `load`, `query_history`, `set_param`, `evaluate_expression`, `start_sketch`, `add_constraint`, `dimension`, `pad_sketch`, `pocket_sketch`, `mate`, `place_component`, `add_drawing_view`.
2. **Schemas** auto-generated from `Command` via `schemars 0.8` (new dep). Each schema includes prose description, examples, deprecation notes.
3. **Streaming** for long-running operations (recompute, mesh generation): SSE or WebSocket upgrade. Progress events forwarded to client.
4. **Auth** for remote use: bearer token, scope claims (`cadkernel.read`, `cadkernel.write`, `cadkernel.admin`).
5. **Rate limiting** per-token + per-tool.
6. **Examples**: `examples/mcp/ai_session.json` rebuilds R1; `examples/mcp/cursor_integration.md` shows config for Cursor / Claude Desktop / Continue.dev.

**Acceptance:** an external MCP client (Claude Desktop) lists tools, builds R1, exports STEP, reads it back, all without server restart; `tools/list` schema validates against the JSON Schema 2020-12 standard.

### A7 — Branching undo + named checkpoints + history scrubbing

**Goal:** Photoshop-class history with named save-points and branching.

**Deliverables:**

1. `Session::checkpoint(name)` → `CheckpointId`. Stored as a `(name, command_index, document_hash)` tuple.
2. `Session::restore(id)` rebuilds from `log[..id.command_index]`.
3. **Branching**: undo, then execute new command → original branch preserved, accessible via `Session::branches() -> Vec<BranchInfo>`.
4. **Branch promotion**: `Session::promote_branch(BranchId)` sets it as the linear log; other branches retained as named checkpoints unless garbage-collected.
5. **History scrubbing** — `Session::scrub(command_index)` rebuilds Document at that point without modifying the log; viewer subscribes to scrub events and highlights affected feature nodes.
6. **GC**: orphaned branches with no checkpoint older than N days pruned on save; configurable.

**Acceptance:** undo to checkpoint, branch, switch back, both branches replay deterministically; R12 history scrubbing latency < 1 s per scrub step.

### A8 — Multi-document coordination

**Goal:** assemblies reference external part documents; cross-document recompute works.

**Deliverables:**

1. `DocumentRef { path, persistent_id, last_known_hash }` for external links.
2. **Recompute trigger** when a referenced part changes on disk (file watcher).
3. **Hash-locked references** (similar to git submodules): assembly pins each part to a hash; updating requires explicit user action.
4. **Where-used graph** — `Workspace::where_used(persistent_id) -> Vec<DocumentRef>`.
5. **Round-trip** with assemblies that span 100+ documents (R11).

**Acceptance:** R11 opens, all components load lazily, edit a sub-part, assembly recomputes only affected mates.

### Track A summary

| Phase | Status | v-target |
|---|---|---|
| A1 | Done | v0.5 |
| A2 | In flight | v0.5 |
| A3 | Pending | v1.0 |
| A4 | Pending | v1.0 |
| A5 | Pending | v1.0 |
| A6 | Pending | v0.5 (after A2) |
| A7 | Pending | v1.0 |
| A8 | Pending | v2.0 |

---

## 5. Track B — Experience (UI/UX)

**Mission:** the GUI is a competitive product, not a kernel demo wrapper. Track B is **the longest track in absolute hours** and was the most under-scoped pre-v3. Pre-v4 it was 12 phases; v4 expands to **20 phases** and adds the *information architecture, design system, interaction model, and per-workbench wireframe* spec sections that were missing. Every phase below cites pixel sizes, durations in ms, and named comparison products.

### 5.0 Information architecture (the application shell)

The desktop GUI is one window with five regions. Layout is `egui` "Layouted side panels around a CentralPanel" pattern; widget tokens reference §5.1 design system.

```
┌──────────────────────────────────────────────────────────────────────────┐
│  TitleBar (32 px)         Document.cadk* — CADKernel        [_ □ ✕]      │
├──────────────────────────────────────────────────────────────────────────┤
│  RibbonBar (96 px)  [Tabs: File Home Sketch Part Assy Draw SheetMetal …] │
├─────┬───────────────────────────────────────────────────────┬────────────┤
│  L  │                                                       │     R      │
│  e  │             Viewport (3D / 2D)                        │            │
│  f  │                                                       │  Property  │
│  t  │                                                       │   panel    │
│  P  │  ┌───────────────────────────────────────┐            │            │
│  a  │  │  ViewCube + ViewGizmo  (top-right,    │            │  Selection │
│  n  │  │   floats inside Viewport)             │            │   panel    │
│  e  │  └───────────────────────────────────────┘            │            │
│  l  │                                                       │  Inspector │
│  s  │  HUD overlays: scale ruler, status, fps               │            │
│     │                                                       │            │
├─────┴───────────────────────────────────────────────────────┴────────────┤
│  StatusBar (24 px)   units: mm   |   sel: 3 faces   |   recompute: 184ms │
└──────────────────────────────────────────────────────────────────────────┘
```

- **TitleBar** — custom-drawn (no native chrome) for theme parity; window controls are accessible primitives (AccessKit role `WindowCloseButton` etc.).
- **RibbonBar** — tabbed, contextual. Tabs always present: File, Home, Sketch, Part, Assembly, Drawing, SheetMetal, Weldment, Surface, Mold, Render, Sim, CAM, Scripting, View, Help. Tab visibility may be filtered per workbench in user settings (advanced users can hide).
- **LeftPanels** — a vertical stack of *dock zones*. Default contents (top-down): **Project Browser** (multi-doc tree), **Feature Tree**, **Layers/Bodies**, **Materials**. Each panel is a `DockTab`; tabs may be torn off into floating windows or moved to right side.
- **CentralPanel: Viewport** — the canvas. Holds the wgpu surface plus floating overlays (ViewCube, ViewGizmo, scale ruler, mini-axis). One viewport per document tab; up to four split-views per tab (`Ctrl-Alt-2/3/4`).
- **RightPanels** — Property, Selection (current selection details), Inspector (validation), Mate (assembly only), Sketch (sketcher only), History.
- **StatusBar** — units, selection summary, recompute time, network/cloud status, lock-state indicator (read-only docs), keyboard-modifier indicator (Shift / Ctrl held).

**Layout persistence.** Each dock layout is saved per-workbench under `~/.config/cadkernel/layout/<workbench>.toml`. User can restore default via `View > Reset Layout`. Layouts stored in user config; never in `.cadk` (the document is geometry, not workspace).

**Multi-monitor.** Floating panels can be dragged to a second monitor. Window position + monitor identity persisted per workbench.

**Workbench switching.** Top-left dropdown (next to File menu) toggles workbench. Each workbench:

- Sets default ribbon tab.
- Sets default left-panel composition.
- Filters which feature kinds appear on toolbars.
- Is itself a `Workbench { id, name, default_layout, contributed_commands, contributed_shortcuts }` value (made extensible by Track E plugin SDK).

Workbench list at v1.0: Part, Assembly, Drawing, Sketcher (modal). v2.0 adds: SheetMetal, Weldment, Surface, Mold. v3.0 adds: Sim, CAM, Render, Inspect.

**Modal vs non-modal.** Sketcher, FEM result inspection, exploded-view editing, and CAM toolpath simulation are *modal workbenches* (entered/exited explicitly). Everything else is non-modal.

### 5.1 Design system (the design tokens & component library)

**Tokens** live in `crates/viewer/themes/tokens.toml` and are referenced by every widget. They are SemVer-locked at the minor level — adding a token is fine, removing is breaking.

#### Colour tokens (light / dark / high-contrast)

| Token | Light | Dark | HC |
|---|---|---|---|
| `bg.canvas` | `#FAFAFA` | `#1B1D22` | `#000000` |
| `bg.panel` | `#FFFFFF` | `#23262C` | `#101010` |
| `bg.panel.alt` | `#F4F4F4` | `#2A2D33` | `#0A0A0A` |
| `bg.elevated` | `#FFFFFF` | `#2E323A` | `#000000` |
| `bg.selected` | `#0066CC1A` | `#3399FF26` | `#FFFF00` |
| `bg.hover` | `#0066CC0F` | `#3399FF14` | `#FFFFFF26` |
| `fg.primary` | `#1A1A1A` | `#E6E6E6` | `#FFFFFF` |
| `fg.secondary` | `#666666` | `#A0A0A0` | `#E0E0E0` |
| `fg.muted` | `#999999` | `#7A7E84` | `#C0C0C0` |
| `fg.disabled` | `#BBBBBB` | `#5A5E64` | `#808080` |
| `accent` | `#0066CC` | `#3399FF` | `#FFFF00` |
| `accent.hover` | `#0055AA` | `#55ABFF` | `#FFFFAA` |
| `success` | `#1B8A3F` | `#33CC66` | `#00FF00` |
| `warning` | `#C77B00` | `#FFAA33` | `#FFAA00` |
| `danger` | `#C8302C` | `#FF5555` | `#FF0000` |
| `info` | `#1F6FAB` | `#5CC4FF` | `#00FFFF` |
| `border` | `#E0E0E0` | `#3A3D43` | `#FFFFFF` |
| `border.strong` | `#C0C0C0` | `#52565C` | `#FFFFFF` |
| `focus.ring` | `#3399FFCC` | `#88CCFFCC` | `#FFFF00` |
| `viewport.bg.top` | `#E8ECF1` | `#1A1D24` | `#000000` |
| `viewport.bg.bottom` | `#C2CCD7` | `#0A0C10` | `#000000` |
| `viewport.grid.major` | `#A0A8B0` | `#3A3D43` | `#FFFFFF` |
| `viewport.grid.minor` | `#D0D6DC` | `#2A2D33` | `#808080` |
| `sel.face` | `#FF8800CC` | `#FFAA33CC` | `#FFFF00` |
| `sel.edge` | `#FF6600` | `#FF9933` | `#FFFF00` |
| `sel.vertex` | `#FF3300` | `#FF6633` | `#FFFF00` |
| `hover.outline` | `#3399FFAA` | `#88CCFFAA` | `#FFFFFF` |
| `sketch.fixed` | `#000000` | `#FFFFFF` | `#FFFFFF` |
| `sketch.normal` | `#1F6FAB` | `#5CC4FF` | `#00FFFF` |
| `sketch.construction` | `#999999` (dashed) | `#7A7E84` (dashed) | `#A0A0A0` |
| `sketch.dim` | `#1B8A3F` | `#33CC66` | `#00FF00` |
| `sketch.over` | `#C8302C` | `#FF5555` | `#FF0000` |

Contrast ratio gate: every `fg.*` over its matched `bg.*` ≥ WCAG AA 4.5:1 normal / 3:1 large. HC theme ≥ 7:1 across the board.

#### Typography scale

| Role | Family | Size | Weight | Line height |
|---|---|---|---|---|
| Display | Inter Variable | 28 px | 600 | 36 px |
| H1 | Inter Variable | 22 px | 600 | 28 px |
| H2 | Inter Variable | 18 px | 600 | 24 px |
| H3 | Inter Variable | 14 px | 600 | 20 px |
| Body | Inter Variable | 13 px | 400 | 18 px |
| Body-strong | Inter Variable | 13 px | 600 | 18 px |
| Caption | Inter Variable | 11 px | 400 | 14 px |
| Code | JetBrains Mono | 12 px | 400 | 16 px |
| Numeric | Inter Variable (tabular nums) | 13 px | 500 | 18 px |
| Viewport HUD | Inter Variable | 11 px | 500 | 14 px |
| Sketch dim | Inter Variable | 12 px | 500 | n/a |

CJK fallback: Noto Sans CJK KR / JP / SC at matching pixel size; ko-locale UI uses 13 px Noto Sans KR. Math glyphs (∅ Ø ° ± × Ⓜ Ⓛ Ⓟ ⌀) covered by Inter; fall back to Noto Symbols.

#### Spacing & sizing

8 px base grid. Tokens: `space-1` 4 px · `space-2` 8 px · `space-3` 12 px · `space-4` 16 px · `space-5` 24 px · `space-6` 32 px · `space-7` 48 px · `space-8` 64 px.

Hit targets: minimum 24 × 24 px (mouse), 44 × 44 px (touch mode). Buttons: small 24 px / medium 32 px / large 40 px. Icons: 16 / 20 / 24 / 32 px.

Border radius: `radius-sm` 4 px (chips, badges) · `radius-md` 6 px (buttons, input) · `radius-lg` 8 px (panels, cards) · `radius-pill` 999 px.

Shadows: `shadow-sm` 0 1 2 rgba(0,0,0,0.06) · `shadow-md` 0 4 8 rgba(0,0,0,0.10) · `shadow-lg` 0 16 32 rgba(0,0,0,0.16) · `shadow-popover` 0 8 24 rgba(0,0,0,0.24). HC theme zero shadows; rely on borders.

#### Motion tokens

| Use | Easing | Duration |
|---|---|---|
| Hover | ease-out | 80 ms |
| Press | ease-out | 60 ms |
| Tooltip in | ease-out | 100 ms (delay 600 ms) |
| Tooltip out | ease-in | 60 ms |
| Panel open/close | ease-in-out | 200 ms |
| Modal open | spring (s=0.85, d=0.7) | ~280 ms |
| Modal close | ease-in | 160 ms |
| Camera fit | cubic-bezier(0.2,0.7,0.3,1) | 350 ms |
| ViewCube snap | spring (s=0.7, d=0.6) | 350 ms |
| Exploded view | per-step ease-in-out | configurable, default 400 ms |
| Page transition | ease-in-out | 220 ms |
| Coachmark pulse | sine | 1200 ms loop |
| Recompute shimmer | linear | 1500 ms loop |

Reduced-motion mode honours OS setting; replaces all springs / pulses with instant transitions.

#### Component library

`crates/viewer/src/components/{button.rs, input.rs, slider.rs, dropdown.rs, …}`. Components are thin wrappers around `egui` primitives that read tokens. Catalogue (39 components):

`Button` (primary / secondary / tertiary / danger / icon-only / split) · `IconButton` · `ToggleButton` · `SegmentedControl` · `Checkbox` · `Radio` · `Switch` · `TextInput` · `NumericInput` (with unit suffix + nudge buttons + expression mode) · `ExpressionInput` · `ComboBox` · `MultiSelect` · `Slider` · `RangeSlider` · `ColorPicker` · `MaterialPicker` · `FilePathInput` · `Tabs` · `Accordion` · `TreeView` · `ListView` · `DataGrid` (sortable, filterable, virtualised) · `Toolbar` · `RibbonGroup` · `Menu` · `ContextMenu` · `Tooltip` · `Toast` · `Badge` · `Chip` · `Tag` · `Breadcrumb` · `Pagination` · `Dialog` · `Drawer` · `Popover` · `EmptyState` · `Skeleton` · `ProgressBar` · `Spinner`.

Every component:

1. Has light / dark / HC variants.
2. Honours focus ring per a11y spec.
3. Has 3 sizes (sm / md / lg).
4. Has hover / focus / active / disabled / busy / error states.
5. Is keyboard-fully-operable (tested).
6. Has Storybook-style "showcase" page accessible via `View > UI Showcase` (debug builds).

#### Iconography

Single coherent icon set. Source: in-house SVGs in `crates/viewer/icons/<workbench>/<icon>.svg`. Rasterised to texture atlas at 16 / 20 / 24 / 32 px at build time. Style: 1.5 px stroke, rounded joins, monochrome (themed via tint). Catalogue ≈ 380 icons at v1.0; ≈ 700 by v3.0. License: CC0 in-house or MIT-licensed Lucide subset.

#### Cursors

15 custom cursors mapped to context: default arrow · text · crosshair · pan-hand · orbit · zoom · pick-face · pick-edge · pick-vertex · sketch-line · sketch-circle · sketch-arc · sketch-spline · dimension · forbidden. Pen-input adds 2 (sketch-pen, eraser).

#### Sound (optional)

Off by default. When enabled: completion chime (130 ms, A4-E5), error blip (100 ms, F#3), notification (200 ms three-tone). Used sparingly; respects OS Do Not Disturb.

### 5.2 Interaction model (mouse / keyboard / touch / pen)

#### Default mouse map (3-button mouse, configurable)

| Action | Default | Alt schemes |
|---|---|---|
| Orbit | MMB drag | Alt+LMB (Maya); RMB drag (CAD) |
| Pan | Shift+MMB drag | MMB drag (Maya); Shift+RMB (Blender) |
| Zoom | Scroll wheel | Ctrl+MMB drag |
| Box select | LMB drag (empty area) | — |
| Pick | LMB click | — |
| Toggle pick | Ctrl+LMB | — |
| Pick-through | Alt+LMB | — |
| Cycle overlapping | Ctrl+Alt+LMB | — |
| Context menu | RMB | — |

User can switch between three mouse schemes: **CAD** (default — RMB orbit), **Maya** (Alt-LMB orbit), **Blender** (MMB orbit, scroll-wheel-click). Persisted in user settings.

#### Default keyboard map

Global:

| Action | Shortcut |
|---|---|
| Command palette | `Ctrl-K` |
| Help index | `F1` |
| Shortcut overlay | `?` |
| Workbench: Part | `Ctrl-1` (modifier configurable) |
| Workbench: Sketch | `Ctrl-2` |
| Workbench: Assembly | `Ctrl-3` |
| Workbench: Drawing | `Ctrl-4` |
| New | `Ctrl-N` |
| Open | `Ctrl-O` |
| Save | `Ctrl-S` |
| Save As | `Ctrl-Shift-S` |
| Undo | `Ctrl-Z` |
| Redo | `Ctrl-Y` / `Ctrl-Shift-Z` |
| Cut / Copy / Paste | `Ctrl-X / C / V` |
| Delete | `Delete` |
| Find | `Ctrl-F` |
| Settings | `Ctrl-,` |
| Toggle full-screen | `F11` |
| Toggle sidebar | `Ctrl-B` |
| Toggle right panel | `Ctrl-Shift-B` |

Viewport:

| Action | Shortcut |
|---|---|
| Fit document | `F` |
| Fit selection | `Shift-F` |
| Top / Front / Right / Iso | `1 / 2 / 3 / 7` |
| Bottom / Back / Left | `Shift-1 / 2 / 3` |
| Toggle perspective / ortho | `5` |
| Wireframe / shaded / shaded-with-edges | `Z` cycles |
| Toggle hidden lines | `Ctrl-H` |
| Toggle section | `Shift-S` |
| Toggle grid | `G` |
| Toggle origin/datum | `Shift-O` |

Per-workbench shortcuts listed in §5.4.

User-customisable. `Settings > Keyboard` shows a searchable, conflict-detecting editor. Profiles: CAD-default, SolidWorks-like, Fusion-like, Onshape-like, Inventor-like, FreeCAD-like, custom.

#### Touch (Track B12 details)

| Gesture | Action |
|---|---|
| 1-finger drag | Pick/select; then orbit if in viewport blank |
| 2-finger pinch | Zoom |
| 2-finger drag | Pan |
| 2-finger rotate | Roll |
| 3-finger drag up/down | Orbit |
| Long-press | Context menu |
| Double-tap | Fit document |

Pen (Surface, iPad with Pencil, Wacom):

- Pressure mapped to sketch line emphasis hint (does not affect geometry).
- Tilt ignored except for eraser tail.
- Palm rejection on by default.
- Pen-button cycles tool: line → circle → arc.
- Hover preview when pen is hovered (selection / dimensioning preview).

#### Cursor radius

Pick uses the larger of "pixel under cursor" and a "snap radius" of `7 px (default), 10 px (touch mode), 5 px (precision-mode held = Alt)`.

### 5.3 Per-workbench wireframes

#### 5.3.1 Part workbench (default)

```
Ribbon: [Sketch] [Pad] [Pocket] [Revolve] [Sweep] [Loft] [Helix] [Hole]
        [Fillet] [Chamfer] [Draft] [Shell] [Pattern▾] [Mirror] [Boolean▾]
        [Move] [Measure] [Section] [Reference▾]

Left:
   Project Browser [▾]
     ├─ Bracket.cadk *
     │   ├─ Origin
     │   ├─ Datum planes (XY, YZ, XZ)
     │   ├─ Sketches
     │   │   └─ Sketch_001
     │   └─ Bodies
     │       └─ Body_001
     │           ├─ Pad_001
     │           ├─ Hole_001 …
     │           └─ Fillet_001
   Feature Tree [▾]
     ├─ ▶ Sketch_001 (10 entities, fully-determined)
     ├─ ▶ Pad_001 (depth=20mm, blind)
     ├─ ▶ Hole_001 (Ø6, through-all)
     └─ … 
   Layers / Bodies [▾]
     ├─ Body_001 (visible, ABS plastic)
     └─ Body_002 (hidden)
   Materials [▾]
     ├─ Default
     ├─ Steel-AISI1020
     └─ ABS-plastic

Right:
   Property panel (depends on selection):
     when Pad_001 selected:
       Name [Pad_001              ]
       Type ( Blind ▾ )
       Depth     [20.0   mm  fx ]
       Direction (Normal ▾)  ⟲ flip
       Draft     [0.0    °     ]
       Material  [(inherit)    ▾]
   Selection panel:
     ▾ Selection (3)
       • Face_27 (planar, 240 mm², normal +Z)
       • Edge_42 (linear, 35 mm)
       • Vertex_13 (45.0, 12.0, 8.0)
   Inspector:
     ✓ No issues
```

#### 5.3.2 Sketcher workbench (modal)

```
Ribbon: [Line] [Rect▾] [Circle▾] [Arc▾] [Spline] [Polygon▾] [Slot▾]
        [Fillet] [Chamfer] [Trim] [Extend] [Offset] [Mirror] [Pattern]
        [Coincident] [Horizontal] [Vertical] [Parallel] [Perpendicular]
        [Tangent] [Equal] [Symmetric] [Fix] [Block]
        [Distance▾] [Angle] [Radius] [Diameter]
        [External Reference] [Toggle Construction] [Show Constraints]
        [Solver Diagnostics] [Exit Sketch]

Centre: 2D viewport with grid; sketch plane outlined; constraint glyphs
inline along entities; dimension labels; DoF readout bottom-right.

Right:
   Sketch Entities (table, virtualised):
     #   Type       Geom info               Constraints
     1   Line       (0,0) → (50,0)          H, fixed-start
     2   Line       (50,0) → (50,30)        V
     3   Arc        center(50,30) r=10      tangent to #2
     ...
   Constraints (table):
     #   Type        Refs              Value
     1   Distance    #1                50 mm
     2   Angle       #1, #2            90°
     ...
   Solver Diagnostics:
     DoF: 0 (fully constrained, green)
     Residual: 1.2e-9
     Iterations: 3
     If over-constrained: shows minimal conflict set highlighted in red.

Bottom HUD:
   [DoF: 0] [Mode: Snap] [Grid: 5mm] [Polar: 15°] [Construction: Off]
```

#### 5.3.3 Assembly workbench

```
Ribbon: [Insert Component] [Mate▾: Coincident, Concentric, Distance,
        Angle, Tangent, Width, Symmetric, Lock, Gear, Cam, Universal]
        [Move Component] [Rotate Component]
        [Pattern Component▾] [Mirror Component]
        [DoF Analysis] [Motion Study] [Interference]
        [Exploded View] [Configuration▾]
        [BOM] [Smart Fastener]

Left:
   Assembly tree:
     Gearbox.cadk
     ├─ ▶ Housing.cadk (1)
     ├─ ▶ Shaft_input.cadk (1)
     ├─ ▶ Shaft_output.cadk (1)
     ├─ ▶ Gear_pinion.cadk (1) [pattern of 4]
     ├─ ▶ Bearing_6202.cadk (4) [smart fastener]
     ▼ Mates (12)
       ├─ Coincident (Housing/Face_top, Shaft_input/Face_top)
       ├─ Concentric (Shaft_input/Cyl_outer, Bearing_6202/Cyl_inner)
       ...

Right:
   Selected mate:
     Type ( Concentric ▾ )
     Refs Shaft_input/Cyl_outer ⇄ Bearing_6202/Cyl_inner
     Lock rotation (off)
     Distance offset 0.0 mm
     Status: ✓ DoF removed: 4
   Component selected:
     Position [drag to reposition]
     Rotation [orbit gizmo]
     Suppress (off)
     Configuration ( Default ▾ )
```

#### 5.3.4 Drawing workbench

```
Ribbon: [New Sheet] [Sheet Format▾] [Insert View▾: Front, Top, Right, Iso,
        Section, Detail, Auxiliary, Crop, Broken-out, Predefined]
        [Dimension▾] [GD&T▾] [Datum] [Surface Finish] [Weld Symbol]
        [Centerline] [Centermark]
        [Annotation: Note, Balloon, Leader, Hole Callout]
        [Table▾: BOM, Hole, Revision, General]
        [Hatch] [Block] [Layer]
        [Print] [Export PDF] [Export DXF]

Centre: paper-style sheet with views; rulers along edges; sheet boundaries.

Right:
   Sheet properties:
     Format ( ISO A3 ▾ )
     Orientation (Landscape)
     Scale 1:2
     Title block ( Default ISO ▾ )
   View properties (when view selected):
     Source: Bracket.cadk
     Type: Front
     Scale (use sheet ▾)
     Display: shaded with edges ▾
     Hidden lines (Visible)
   Layers panel (CAD-DWG style):
     ✓ Visible | Locked | Color | Lineweight | Linetype
     0       ✓      □    ▣ Black   0.25 mm    Continuous
     Hidden  ✓      □    ▣ Gray    0.18 mm    Dashed
     Center  ✓      □    ▣ Red     0.18 mm    Center
```

#### 5.3.5-12 Other workbenches

Equally specified for SheetMetal (base flange, edge flange, miter, hem, jog, fold/unfold, flat pattern dialog), Weldment (frame generator, structural-member library picker, cut-list table), Surface (ruled, blend, fill, knit, trim), Mold (parting line, parting surface, core/cavity split, ejector layout), Render (PBR materials, environment HDRI picker, render queue), Sim (mesh dialog, BC list, run, results plot), CAM (operation tree, tool library, post-processor selector, machine simulation panel), Inspect (measure, mass props, draft analysis, undercut analysis). Each has a dedicated wireframe in `docs/wireframes/<workbench>.png` rendered from `docs/wireframes/<workbench>.dot` (Graphviz) — produced as part of B0.

### 5.4 Per-workbench keyboard maps

(Comprehensive table — abbreviated; canonical version under `docs/keymap.md` generated from `crates/viewer/src/keymap.rs` constants.)

**Sketcher hotkeys (selected):**

| Key | Action |
|---|---|
| `L` | Line tool |
| `Shift-L` | Polyline (chain lines) |
| `R` | Rectangle (2-point) |
| `Shift-R` | Center rectangle |
| `C` | Circle (center-radius) |
| `Shift-C` | 3-point circle |
| `A` | Arc (center-start-end) |
| `Shift-A` | 3-point arc |
| `S` | Spline |
| `Shift-S` | Spline by control points |
| `O` | Polygon |
| `T` | Slot (straight) |
| `Shift-T` | Slot (arc) |
| `F` | Sketch fillet |
| `Shift-F` | Sketch chamfer |
| `D` | Smart dimension |
| `H` | Horizontal constraint |
| `V` | Vertical constraint |
| `P` | Parallel |
| `Shift-P` | Perpendicular |
| `=` | Equal |
| `Shift-=` | Symmetric |
| `Shift-T` | Tangent |
| `Shift-X` | Toggle construction |
| `Esc` | Cancel current tool |
| `Tab` | Cycle field within tool |
| `Enter` | Apply tool |

**Part workbench (selected):**

| Key | Action |
|---|---|
| `E` | Extrude (pad) |
| `Shift-E` | Pocket |
| `Shift-R` | Revolve |
| `Shift-W` | Sweep |
| `Shift-L` | Loft |
| `Shift-H` | Hole wizard |
| `B` | Boolean (popup) |
| `Shift-F` | Fillet (filtered to "edge selection") |
| `Shift-C` | Chamfer |
| `Shift-D` | Draft |
| `Shift-Q` | Shell |
| `Shift-M` | Mirror |
| `Shift-P` | Pattern |
| `M` | Measure |

(Full table in `docs/keymap.md`.)

### 5.5 The 20 Track-B phases

**B0 — UI foundation refactor [must precede everything]**

The current `crates/viewer/src/lib.rs` is a 5-kloc monolith. B0 splits it.

Deliverables:

1. `crates/viewer/src/{shell, panels, viewport, ribbon, statusbar, components, themes, i18n, picking, gizmos, hud, dialogs, settings, keymap, command_palette, telemetry, tour}` — module per concern.
2. Plugin-friendly contributed-command + contributed-panel registry.
3. Workbench abstraction `Workbench { id, default_layout, ribbon, shortcuts, contributed_panels }`.
4. Layout engine using `egui::dock` with custom drag-handle / tear-off styling.
5. Viewport encapsulated in `ViewportWidget` struct that owns wgpu surface, picking buffer, render passes.
6. Theme provider via `ThemeContext` accessible to every widget.
7. ARIA-equivalent landmark map via AccessKit; dev tool `View > Show A11y Tree`.

Acceptance: `cargo build`/`clippy`/`test` green after refactor; visual regression snapshot baseline established.

**B1 — Selection & picking foundation [highest priority]**

(As §5.0 + previous §B1 below; expanded below.)

**Architecture:**

- **GPU ID buffer** — render to off-screen `Texture { format: R32Uint, samples: 1, mip_level: 0 }`; each entity (vertex/edge/face/body) is allocated a 32-bit id from a Document-wide id manager. Reads on click via a 1×1 `wgpu::Buffer::map_async`.
- **Sub-shape priority order**: vertex > edge > face > body. Modifier keys cycle: `Ctrl+LMB` cycles overlapping picks (next in z-order); `Alt+LMB` selects through (back face).
- **Edge / vertex inflation** — render edges as tube primitive of diameter `1.5 px` screen-space; render vertices as point sprites `4 px`. Inflation only in the id-buffer pass (visual rendering keeps its own width).
- **Hover preview** — outline shader on the hovered entity; debounce 50 ms; fade-in 100 ms; uses `hover.outline` token.
- **Click latency** — < 16 ms from mousedown to pick result (one frame on 60 Hz display). Async readback budget 4 ms.
- **Box / lasso select** — rectangle (Shift-LMB drag in empty area) or freehand polygon (`Q`-prefix + LMB drag). Crossing vs Inside modes (DXF convention; configurable).
- **Selection sets** — named, persisted with Document; `Selection::save_set(name, entities)`; surfaced under `View > Saved Selections`.
- **Selection filter toolbar** — top-right of viewport: pill toggles for Vertex / Edge / Face / Body / Sketch / Datum; modifies what picks resolve to.

**Deliverables:** `crates/viewer/src/picking/{gpu_buffer.rs, dispatcher.rs, hit_test.rs, sub_shape.rs, sets.rs, lasso.rs, filter.rs}`; CPU fallback retained behind setting; `Selection` API exposes `pick(point, filter) -> Option<EntityRef>`, `pick_box(rect, mode)`, `pick_lasso(polygon, mode)`, `add(entity)`, `remove(entity)`, `clear()`, `set_filter(SelectionFilter)`, `save_set(name)`, `restore_set(name)`, `history_undo()`.

**Acceptance:** R2 — every face/edge/vertex pickable at any camera distance from 1× to 1000× nominal; box-select 50 features < 100 ms; selection sets save and reload; cursor latency < 16 ms p99.

**B2 — Camera & navigation: cube, gizmo, section, exploded, walkthrough**

Deliverables expanded:

1. **ViewCube** (top-right corner of viewport, 120 px) — IndependentScene rendered overlay; faces / edges / corners are 26 hit-zones; click snaps orthographic; click-and-drag orbits the camera (matching the snap centre); double-click fits; right-click opens named-views menu; spring-snap transition 350 ms.
2. **ViewGizmo** (under ViewCube, 80 px triad) — three colour-coded axes; click snaps to axis-aligned view; double-click cycles +/−.
3. **Section box** — three-axis interactive AABB; drag faces to clip; multi-monitor-safe gizmo handles.
4. **Section plane** — single planar clip; sketch-on-section view (creates a sketch in the section plane).
5. **Exploded view** — animated per-feature offsets along user-specified vectors; per-step delays; dragger for explosion factor `[0.0–1.0]`; recordable as `.cadk`-embedded `Animation { steps: Vec<ExplodeStep>, total_duration }`.
6. **Walkthrough mode** — first-person navigation with WASD + mouse + Q/E up/down; collision against geometry optional; toggle key `F11` (also exits with same key).
7. **View bookmarks** — save up to 99 named views per document; cycled via `Ctrl-1..9`; thumbnails in `View > Saved Views` panel.
8. **Camera modes**:
   - Perspective (default 45° FOV, slider 10°-90°).
   - Orthographic (true parallel projection).
   - Two-point perspective (architectural).
   - Roll lock toggle (`Up = +Z always`).
9. **Animation** — fit-document, view-snap, exploded use the `motion.fit` token (350 ms cubic-bezier); user can disable in `Settings > Motion > Reduce`.
10. **Mini-axis** (bottom-left, 40 px) — always-visible RGB axis triad; orbit-time hint.
11. **Navigation rune** (top-left near workbench dropdown, 24 px) — single icon clicking opens the navigation menu (cube/gizmo controls in palette form).

**Acceptance:** all controls functional in R3; section-box clip survives save → reload; walkthrough-mode test in R11.

**B3 — Property panel: two-way live binding + expressions + units**

(Original B3 expanded.)

Deliverables:

1. **Layout** — vertical scrollable list with collapsible groups: General · Geometry · Materials · Render · Custom Properties.
2. **Numeric field component** (`NumericInput`):
   - Inline unit suffix (`mm`, `in`, `°`, `rad`, `kg`, `N`, `MPa`).
   - Spinner buttons (held: 10 Hz repeat).
   - Drag-on-label scrubbing (Photoshop-style: drag horizontal on field label changes value; Shift = ×10, Alt = ÷10).
   - Expression mode toggle (`fx` button) — switches input from value to formula; preview shows evaluated value.
   - Validation: invalid red, out-of-range yellow with confirmation, valid silent.
   - Debounce: 200 ms after last keystroke commits via `Session::execute(EditFeature)`.
3. **Recompute progress** — inline shimmer at top of panel during recompute; cancel via Esc; partial preview during recompute optional.
4. **Expression engine** (cf. A5) — supports operators (`+ - * / % ^`), parens, common math fns (sin / cos / tan / asin / acos / atan / atan2 / sqrt / abs / min / max / floor / ceil / round / clamp / lerp / pow / exp / log / log2 / log10), unit-aware arithmetic, named global params (`document.M`, `document.thickness`), conditionals (`if(cond, a, b)`), lookups (`table.row.col`).
5. **Units system** — every field has unit; underlying value SI; user toggles via `Settings > Units > {Metric, Imperial, Custom per Document}`. Conversion table from `uom 0.36`.
6. **Per-document parameter table** (`Tools > Parameters`) — columns: name, expression, value, unit, description, used-by; rename propagates references.
7. **Undo** — every commit is one Command; undoable; coalesces consecutive numeric edits within 1 s window.
8. **Inspector panel** (separate) — `Document::validate()` issues, persistent-name resolution failures, broken external references, over-constrained sketches, suppressed-feature reasons.

Acceptance: R2 pad depth 10 → 25 recompute < 200 ms; `=hole_depth*0.3` reacts to upstream change; mm → in switch updates display without changing geometry hash.

**B4 — Sketcher v2 (drag-edit + in-place dim + DoF + snap + curvature)**

Deliverables (expanded with explicit interaction details):

1. **In-place dimension editing** — click dim label, inline editor pops up; type value, Enter commits; Tab moves to next nearby dim; Esc cancels; arrow keys nudge by step (held Shift = 10×, held Ctrl = 0.1×).
2. **Drag handles** — vertices show `8 px` filled circle handles (themed `sketch.normal`); midpoint of a line shows `5 px` square handle; drag respects all locked constraints via Newton-Raphson with locked DoFs.
3. **Constraint glyphs** — per-edge / per-vertex glyphs `12 px`, themed by status (active blue, conflict red, redundant orange); click glyph to remove; right-click for properties.
4. **DoF readout** — bottom-right HUD, status: "Fully constrained" green, "Under-constrained: 3 DoF" yellow with "Click to see free entities", "Over-constrained / inconsistent" red with "Click to highlight conflict set"; clicking opens Solver Diagnostics panel.
5. **Solver Diagnostics panel** — shows constraint graph; conflict-set view; per-equation residual; suggests minimal-removal set.
6. **External references** — pick a face / edge / vertex on a different body; reference appears in entity list with link icon; recomputes on upstream change; broken-reference indicator if upstream removed.
7. **Snapping** — endpoint, midpoint, intersection, perpendicular foot, tangent point, grid, polar (15°/30°/45°/90° configurable), centerline mirror; reticle widget at cursor showing snap kind.
8. **Construction geometry** — toggle any line/circle/arc to construction (dashed `sketch.construction`); contributes constraints, not boundary.
9. **Spline curvature comb** — visible during edit; comb size slider; G2/G3 toggle on a spline endpoint.
10. **Pattern within sketch** — copy a sketch group N times along direction or radial.
11. **Conic constraints** — ellipse / parabola / hyperbola tangency (per C-Sketch v3).
12. **Auto-constraint inference** — when drawing a line near horizontal, snap to horizontal and add H constraint automatically; configurable threshold (default 5°).
13. **Sketcher hotkeys** as §5.4.

Acceptance: R2 sketches all draggable in real time; R3 sketches use external refs; spline G2 transitions visible in curvature comb; over-constrained synthetic test produces minimal conflict set within 100 ms.

**B5 — Theming + i18n + a11y + DPI + fonts**

(As before, expanded.)

Theming layers:

1. **Token level** (§5.1).
2. **Component level** — every component reads tokens; no raw colours in component code.
3. **User-customisable theme files** in `~/.config/cadkernel/themes/<name>.toml`. UI in `Settings > Appearance > Theme`.
4. **Workbench accent** — each workbench can override accent (Sketcher = blue, Drawing = green, etc.); user-toggleable.
5. **Live preview** of theme changes while in the settings dialog.
6. **Theme hot-reload** in dev builds: edit toml, viewer reloads style.

i18n full plan:

- v1.0 ships: en (canonical), ko.
- v2.0: + ja, zh-CN.
- v3.0: + de, fr, es, ru.
- Strings extracted via `i18n!("key")` macro from `cadkernel-i18n-macros` crate.
- Translation file format: `fluent` (`.ftl`) for plurals/genders; per-locale dir.
- CI gates:
  - non-en file missing key vs en → fail.
  - non-en string identical to en (forgotten translation) → warn.
  - %s placeholder mismatch → fail.
- Translator tooling: `cargo i18n-extract` writes templates; `cargo i18n-stats` reports completion %.
- RTL prep: layout mirroring smoke test (all panels, all dialogs); shipped logical-not-physical horizontal alignment in CSS-equivalent.

a11y full plan (WCAG 2.2 AA):

- AccessKit landmark per panel/dialog/menu/button; tested under NVDA (Windows), VoiceOver (macOS), Orca (Linux).
- Tab order well-defined per panel; tab traversal visible.
- Keyboard reachability: every menu, every property field, every viewport action operable without mouse; modeless flows (e.g., sketcher) have keyboard alternatives for every gesture.
- Discoverable shortcut overlay (`?` key) — modal listing all currently-active shortcuts grouped by category; searchable.
- High-contrast theme passes WCAG AA contrast (≥ 7:1).
- Focus indicator: 2 px outline + 4 px offset with `focus.ring` token; visible on every focusable element.
- Screen-reader announcements via `accessibility::announce(text, priority)` for non-visible state changes (recompute complete, save complete, error, …).
- Reduced-motion mode honours OS setting; replaces all springs/pulses with instant transitions.
- Dyslexia-friendly font option (OpenDyslexic) under `Settings > Accessibility > Typography`.
- Magnification: built-in 1.0× / 1.25× / 1.5× / 2.0× UI scale (independent of OS DPI).
- Cursor-size override (24 / 32 / 48 / 64 px).

DPI:

- Viewer scales 1× / 1.5× / 2× / 3× / 4× without pixel-blur.
- Per-monitor DPI on Windows; reactive to monitor change.
- wgpu surface re-creation on DPI change (debounced 200 ms).

Fonts: as §5.1; subset bundled to keep binary size manageable; full glyphs loaded lazily when locale demands (CJK on demand: ~3 MB CJK pack).

Acceptance: R1 build fully keyboard-only with screen-reader announcement; ko-locale build matches en string parity; WCAG 2.2 AA verified by `axe-core`-equivalent automated test in CI.

**B6 — History panel + feature tree + branching UI**

Deliverables:

1. **Feature Tree** in left panel — hierarchical view with bodies, sketches, datums, features.
2. **Each tree row**:
   - Visibility eye `Ctrl-Shift-click`.
   - Suppress checkbox `Ctrl-click`.
   - Edit button (opens feature for editing).
   - Drag-to-reorder (drops update Document feature graph; re-runs recompute).
   - Right-click menu (rename, suppress, edit, delete, copy, group).
3. **Branching UI**: when undo + new edit produces a branch, top-bar shows "Branch: B (active) / Main (saved)"; `View > Branches` opens a graph diff visualisation (timeline with named checkpoints).
4. **Named checkpoints** via `Ctrl-Shift-K` open a dialog (name, description); checkpoint pill shown inline in feature tree.
5. **History scrubbing** — bottom-of-tree slider; drag to scrub through history; viewport reflects state at slider position; Tab applies scrub.
6. **Search** in feature tree (`Ctrl-F` while tree focused).
7. **Multi-select** rows with Shift-click, Ctrl-click; group operations.
8. **Folder grouping** — manually group features into named folders for organisation.
9. **Filter** (top of tree): by feature type, by status (suppressed, errored, ok), by tag.

Acceptance: R3 fully editable through tree; R2 reorder of fillet ↔ pattern recomputes correctly; named checkpoints survive save/reload.

**B7 — Multi-document tabs + recents + thumbnails + workspace**

Deliverables:

1. **Document tabs** along the top of the viewport: title, dirty indicator (`*`), close button; reorder by drag; right-click for "close others" / "close all" / "close to right".
2. **Drag-drop** files anywhere on window opens them.
3. **Welcome screen** when no document open: recent documents grid, quick-start tutorials, template library, "New Document" / "Open" / "Import" CTAs.
4. **Recents** persisted in user config; thumbnails rendered at save (Track A3); pinned recents.
5. **Workspace concept** — `.cadkws` file referencing multiple documents + assembly relations; opened as a single workspace.
6. **Document switcher** (`Ctrl-Tab` cycle, `Ctrl-Shift-T` reopen-closed) with thumbnail preview.
7. **Tab groups** — drag tab to bottom-right edge to split-pane; persists per workspace.

Acceptance: R11 (250-component) opens with all referenced parts as tabs; switching tabs < 100 ms.

**B8 — Visual polish & rendering quality**

(As before, expanded.)

Render pipeline (passes ordered):

1. **G-buffer**: albedo (sRGB8), normals (RG16F packed), material (R32U with metallic/roughness/ao packed), depth (D32F).
2. **Shadow pass**: 4-cascade CSM at 2048², bias-tuned per cascade; PCF 5×5 kernel; `csm.lambda = 0.5`.
3. **Ambient occlusion**: SSAO (16-sample HBAO+); half-res with bilateral upscale.
4. **Lighting pass**: directional + 4 point lights + IBL; PBR Disney BRDF; `metallic-roughness` workflow.
5. **Transparency pass**: depth-peeled (3 layers default; configurable); per-pixel sort fallback for excessive overdraw.
6. **Edge pass**: analytical line rendering — silhouette via depth/normal discontinuity, crease via dihedral threshold; sub-pixel AA via line-coverage texture.
7. **Selection / hover overlay pass**: outline shader; `sel.face/edge/vertex` tints; hover pulse via motion token.
8. **TAA pass**: jittered camera + history buffer; reproject; clamp; bias against ghosting on tessellation update (clear history on geometry change).
9. **Post**: tonemapping (ACES default; Reinhard / linear options); bloom (threshold 1.5, 6 mip kawase blur); chromatic aberration (off by default); vignette (off by default).
10. **Gizmo / HUD overlays**: depth-tested, drawn last.

Materials:

- Disney BRDF (Burley) with diffuse + specular + clearcoat + sheen + transmission.
- Per-feature material overrides.
- Material library: 80 presets at v1.0 (steels, aluminums, plastics, glass, rubber, paint colours), 200 by v2.0.
- Texture support: albedo / normal / roughness / metallic / AO maps; up to 4k resolution; mipmapped.
- Decals (post v1.0): projected images on faces.

Modes:

- Solid (default).
- Solid + edges.
- Wireframe.
- Hidden-line.
- Shaded with hidden-line.
- HLR (precise hidden-line removal for drawings, slow).
- X-ray.
- Realistic (full PBR with HDRI).
- Schematic (flat colours + edges, useful for publication).

Toggle via `Z` cycles modes; right-click for menu.

Acceptance: R3 30 fps at 1080p on 2020 Iris-Xe reference laptop; visual regression pixelmatch ≤ 0.1 % per-pixel diff; `cargo run --release` cold start < 500 ms.

**B9 — Command palette + macros + hotkey customisation**

Deliverables:

1. **Command palette** (`Ctrl-K`):
   - Lists every registered command (~600 at v1.0; ~1500 at v3.0).
   - Fuzzy-match over name + tags + workbench + recent-use weighting.
   - Per-row: icon, name, current shortcut (if any), workbench.
   - Sub-mode: type `>` for command mode (default), `?` for help mode (links to help docs), `:` for settings mode, `@` for go-to (feature/sketch by name), `#` for tag.
   - Shows arg input flow for parametrised commands (e.g., `Pad: depth=`).
2. **Macro recorder** — `View > Macros > Record`; while recording, every Command is appended; stop, name, save; replay via palette.
3. **Macro editor** — list of recorded macros; each editable (reorder steps, edit params, add control flow via Lua).
4. **Hotkey customiser** — `Settings > Keyboard`; searchable; conflict-detected; profile selector (CAD / Maya / Fusion / Onshape / Inventor / FreeCAD / custom); per-workbench overrides.
5. **Workspace saver** — save current panel layout under a name; recall via palette.
6. **Quick switcher** — `Ctrl-P` jump to feature by name (fuzzy).
7. **Recent commands** — top of palette when opened.

Acceptance: every Command in `cadkernel-api` reachable by palette; conflict in keyboard customiser detected and shown in red.

**B10 — In-viewport overlays (3D-projected dimensions, curvature comb, manipulators)**

Deliverables:

1. **Sketch overlays in 3D viewport** — when a sketch is referenced by a feature being edited, projected dimensions / constraints visible behind the 3D view; click-through to enter sketch.
2. **Curvature comb** for edges and surface iso-curves; toggleable via `Inspect > Curvature`.
3. **Reflection lines** for surfaces (Class-A inspection); toggleable.
4. **Iso-curves** on surfaces; density configurable.
5. **Manipulators**:
   - Translate (3 axes + 3 planes; 6 hit zones).
   - Rotate (3 axes; 3 hit zones).
   - Scale (uniform + per-axis).
   - Combo (translate + rotate; single 9-handle gizmo).
   - Always face camera; size scales with screen distance to maintain `~80 px` reach.
6. **Snap during manipulation** — to vertices/edges/grid; numeric override (type number while dragging).
7. **Triple-axis input** during dragging — type values into HUD panel (Position X, Y, Z) for precision.
8. **Datum visualisation** — datum planes drawn as semi-transparent quads; datum axes as dashed lines; datum points as crosshairs. Toggle via `View > Show Datums`.

Acceptance: R3 face-translate via gizmo with snap to adjacent vertex; curvature comb visible on R2 fillet edge.

**B11 — Onboarding, tutorials, contextual help**

Goal: time-to-first-success < 10 minutes for a fresh user.

Deliverables:

1. **First-run tour** — 8-step animated overlay walking through "open document → sketch → pad → save"; skippable; never auto-shown again.
2. **Welcome screen** (cf. B7) with quick-start tile.
3. **Inline coachmarks** on first use of each workbench: pulsing accent ring around the most-relevant ribbon button; dismissible globally or per-coachmark.
4. **Tutorial library** — 12 tutorials at v1.0:
   - 1. Hello CAD (sketch → pad → save).
   - 2. Sketcher fundamentals (constraints, dimensions, DoF).
   - 3. Part features tour (extrude, revolve, sweep, loft, hole).
   - 4. Patterns and mirrors.
   - 5. Fillets, chamfers, drafts, shells.
   - 6. Multi-body parts and booleans.
   - 7. Assembly basics (insert, mate, BOM).
   - 8. Drawings (multi-view, dimensions, GD&T).
   - 9. Sheet metal walkthrough.
   - 10. Weldments walkthrough.
   - 11. Rendering and materials.
   - 12. Scripting (Lua + MCP).
   - Each is interactive: the tutorial overlays the real app; "Next" advances when the user performs the expected action; passive checks (no forced clicks).
5. **F1 contextual help** — pressing F1 anywhere opens mdBook section relevant to the focused panel/dialog.
6. **In-app changelog** — "What's new" surfaced on first-run after upgrade.
7. **Sample documents** ship under `File > Examples`: 20 reference parts including R1-R12.
8. **Feedback** — in-app `Help > Send Feedback`; opens a structured form (does not auto-send; user reviews).

Acceptance: external-tester study (5 first-time users) completes Tutorial 1 in < 10 min; 4/5 reach success without external help.

**B12 — Touchscreen + tablet + pen input**

Deliverables:

1. **Touch mode** auto-detected (Surface Pro, iPad-with-keyboard, touchscreen monitor); manual override in `Settings > Input > Touch`.
2. **Larger hit targets** (44 × 44 px) when touch active.
3. **Gestures** as §5.2.
4. **Pen-aware sketcher** — pressure → line emphasis hint (preview only; final geometry constant); tilt for eraser-end detection; palm rejection on by default.
5. **Pen-button cycle** — short-press cycles tools (line → circle → arc); long-press radial menu of common tools.
6. **On-screen keyboard** for numeric entry on touch-only devices; auto-shown when a numeric field is focused.
7. **Floating context menu** (radial) on long-press.

Acceptance: R1 buildable on Surface Pro 11 with pen.

**B13 — Animations, motion, micro-interactions**

Deliverables:

1. Motion tokens (§5.1) implemented in a single `motion::transition(token, from, to, callback)` API.
2. Animation curves: ease-in / ease-out / ease-in-out / cubic-bezier / spring (Hooke + damping).
3. Reduced-motion mode honoured globally.
4. Camera fit / view-snap / exploded view all use spring tokens.
5. ViewCube snap with subtle "elastic" overshoot (10% then settle).
6. Selection hover pulse (1.2 s sine, 4% opacity oscillation).
7. Recompute shimmer along property panel top.
8. Toast slide-in / slide-out 200 ms.
9. Coachmark pulsing ring 1.2 s loop.
10. Dialog modal fade + slight upward slide on open.

Acceptance: 60 fps maintained during all animations on R3.

**B14 — Notifications, toasts, status, errors**

Deliverables:

1. **Toast component** — short-lived bottom-right notifications; severity = info / success / warning / error; duration 4 s default; click to dismiss; "View" link for actionable.
2. **Notification centre** — bell icon in title bar; lists last 50 notifications; clearable.
3. **Modal error dialog** — for blocking errors (file open failure, save failure); always offers "Copy details" button writing structured error report to clipboard.
4. **Recompute errors** — non-blocking; feature shown red in tree; click to see error in Inspector with stack trace (debug builds).
5. **Status bar items** (right-to-left): network/cloud status, recompute time, selection summary, units, lock-state.
6. **Long-running ops** — progress dialog cancellable via Esc; per-op telemetry: estimated time, ETA, throughput.

Acceptance: every error path produces a toast or dialog with actionable next step; no silent failures.

**B15 — Search & navigation across document**

Deliverables:

1. **Find in document** (`Ctrl-F` while feature tree focused or top-bar) — searches feature names, body names, sketch names, dimensions, custom properties.
2. **Find references** — right-click feature → "Find references" highlights every dependent feature.
3. **Replace parameter** — `Ctrl-H` opens "find/replace value" dialog (e.g., replace 5 mm with 6 mm across selected features).
4. **Go-to** — `Ctrl-G` jumps to feature by name.
5. **Tag system** — features can be tagged (`Ctrl-T`); tags shown in tree filter.

Acceptance: R3 — `Find references` on a sketch returns the 12 features that depend on it.

**B16 — Settings, preferences, profiles**

Deliverables:

1. **Settings dialog** (`Ctrl-,`) with categories: General, Appearance, Keyboard, Mouse, Units, Performance, Network, Privacy, Advanced.
2. **Search** within settings.
3. **Profile import/export** — JSON file capturing all settings; portable.
4. **Per-document overrides** — units, tolerances, view defaults can be document-scoped.
5. **Reset to defaults** per-section.
6. **Setting docs** — every setting has an info icon linking to docs.

Acceptance: every behavioural toggle in the application has a setting; settings dialog reachable in < 200 ms.

**B17 — Telemetry & feedback (opt-in only)**

Deliverables:

1. **Anonymous telemetry** — opt-in via first-run dialog; clearly explains data types collected: command frequencies, perf timings, crash reports.
2. **Crash report** — local-only by default; user reviews before upload; structured fields (command, document hash, OS, GPU, stack).
3. **Feedback** — Help > Send Feedback; opens local form, user reviews, sends explicitly.
4. **Telemetry inspector** — `Settings > Privacy > View Pending Telemetry` shows what will be sent.
5. **GDPR compliance** — DPA published; retention policy 90 days; user can request deletion.
6. **Differential privacy** for command frequencies (Laplace noise, ε=1.0); never exposed to engineering except in aggregate.

Acceptance: telemetry never sends without consent; user can delete all collected data via single button.

**B18 — Drawing/annotation editor UX (per-spec for C-Draw)**

Drawing-workbench specific UX (separate from C-Draw kernel work):

1. **Sheet layout** with rulers, snapping to titleblock cells, unit markers.
2. **View placement** — drag from feature tree onto sheet; auto-aligned to existing views.
3. **Dimension UX** — smart-dim auto-detects intent: click an edge to dimension length; click two parallel lines to dimension distance; click an arc for radius; modifier keys override (Shift = chain dimension, Ctrl = ordinate).
4. **GD&T frame editor** — inline popover with categories; choose tolerance type, value, datum refs, modifiers; previews.
5. **BOM table** — drag from assembly tree onto sheet; columns user-configurable; row drag-reorder; merged cells supported.
6. **Hatch editor** — pattern picker, scale, angle.
7. **Layer panel** as in §5.3.4.
8. **Print preview** with paper-margin overlays; print to PDF with embedded fonts.

Acceptance: R3 drawing assembled by 5-step workflow (insert front, top, right; section; isometric; auto-dim; BOM; print preview).

**B19 — Real-time collaboration UX (Track F surface)**

Deliverables (the editor side of Track F):

1. **Presence avatars** in title bar showing connected users (colour, initials, hovered tooltip).
2. **Live cursors** in viewport — coloured arrows with name labels.
3. **Live selection** — other users' selections shown with their colour at 40% opacity.
4. **Live camera** — toggle to follow another user's camera.
5. **In-doc chat** — collapsible panel; message + reply per feature (anchor); supports markdown.
6. **Comment threads** — anchor a thread to a feature/face/edge; reply; mark resolved.
7. **Suggestion mode** — read-only-but-comment; produces "suggested edit" list; owner accepts/rejects.
8. **Ownership indicator** — per-feature lock when an editor is actively editing; soft-lock auto-released on idle.
9. **Conflict UI** — when CRDT merge produces a structural conflict, modal asks "Accept yours / Accept theirs / Both as branches".
10. **Voice chat** — WebRTC; muted by default; per-doc channel; audio indicator.

Acceptance: 4 simultaneous users edit R3 without conflicts; CRDT auto-merge succeeds; presence/chat/voice all stable for 60 min stress run.

**B20 — Accessibility deep-dive (WCAG 2.2 AA + AAA targets)**

Deliverables (beyond B5 baseline):

1. **Screen-reader-first viewport mode** — keyboard-only viewport navigation; "tab through entities" mode (Tab cycles selectable entities; arrow keys orbit; descriptions announced).
2. **High-contrast theme** ≥ 7:1 (AAA).
3. **Voice-control hooks** — exposes the command set via OS voice-recognition APIs (Windows Voice Access, macOS Voice Control); each command carries a "voice phrase" attribute.
4. **Magnifier-aware** — UI does not break at 4× OS magnification.
5. **Switch input** — basic single-switch scanning support for the most-used commands.
6. **Documentation** — a11y feature matrix in user docs; explicit list of WCAG criteria met.

Acceptance: Test plan executed by certified a11y auditor; report posted; AA confirmed across themes; AAA confirmed in HC theme.

### 5.6 Track B summary

| Phase | v-target | Headline | Owners |
|---|---|---|---|
| B0 | v0.5 | UI foundation refactor | ui |
| B1 | v0.5 | Selection & picking foundation | ui |
| B2 | v1.0 | Camera & navigation (cube, gizmo, section, exploded, walkthrough) | ui |
| B3 | v0.5 | Property panel two-way live + units + expressions | ui |
| B4 | v1.0 | Sketcher v2 | ui + kernel |
| B5 | v1.0 | Theming + i18n + a11y | ui |
| B6 | v1.0 | History tree + branching UI | ui |
| B7 | v1.0 | Multi-doc tabs + welcome screen + thumbnails | ui |
| B8 | v1.0 | Visual polish (PBR + SSAO + TAA + edges + 9 modes) | ui |
| B9 | v1.0 | Command palette + macros + hotkey customisation | ui |
| B10 | v1.0 | 3D-projected sketch overlays + manipulators | ui |
| B11 | v1.0 | Onboarding + 12 tutorials + contextual help | ui + qa |
| B12 | v2.0 | Touchscreen + pen | ui |
| B13 | v1.0 | Animations + motion tokens | ui |
| B14 | v1.0 | Notifications + toasts + structured errors | ui |
| B15 | v1.0 | Find / refs / replace / go-to / tags | ui |
| B16 | v1.0 | Settings + profiles | ui |
| B17 | v1.0 | Telemetry + feedback (opt-in) | ui + qa |
| B18 | v1.0 | Drawing-workbench UX (kernel via C-Draw) | ui + io |
| B19 | v2.0 | Real-time collaboration UX | ui + io |
| B20 | v1.0 / AAA in HC by v2.0 | A11y deep dive (WCAG AA + AAA-HC) | ui + qa |

### 5.7 Track B verification (UX-specific)

- **Visual regression** (B0+): pixelmatch ≤ 0.1 % across 60 baseline screenshots covering each workbench's default state + 5 worked examples.
- **Interaction tests**: `crates/viewer/tests/interaction/{selection.rs, sketch_drag.rs, dim_edit.rs, …}` simulate input events and assert resulting state.
- **A11y CI**: every UI flow walked by an automated AccessKit checker; reports any missing role / label / focus.
- **i18n CI**: as B5.
- **Performance budget per Track D D1**: every interaction step has a budget; CI fails on regression.
- **Layout snapshots**: dock-layout serialisation round-trip per workbench.
- **First-run study**: pre-release, 5 external users complete Tutorial 1 within 10 min.

---

## 6. Track C — Capability (split into 13 sub-tracks)

Track C is the kernel + workbench coverage that distinguishes a CAD tool from a B-Rep library. Every sub-track is itself a multi-phase plan.

### C-Solid — parametric solid features

**Mission:** match SolidWorks `Insert > Features` menu with full parametric history. **24 features** at v1.0; 30+ at v3.0.

**Phasing:**

- **C-Solid v0.5** — pad / pocket / extrude / revolve / boolean / mirror / linear-pattern / circular-pattern (8 features). Already exists; finalise via Track A.
- **C-Solid v1.0** — sweep / loft / helix / hole / hole-wizard / slot / rib / draft / shell / thicken / fillet (constant + variable + face) / chamfer / fillet-asymmetric (13 features). Algorithm refs per row below.
- **C-Solid v2.0** — direct edit (move-face / push-pull) / boundary-feature / wrap / dome / freeform-deformation / advanced-fillet (full-round, set-back, conic) (6 features).
- **C-Solid v3.0** — generative-design (topology-optimised solid generation), live-section editing, history-aware direct edit (combines parametric + direct).

**Per-feature deliverables (matched against SolidWorks `Insert > Features` menu):**

| Feature | Algorithm | Parity target | v-target |
|---|---|---|---|
| Extrude (blind, through-all, up-to-face, mid-plane) | profile sweep along direction; OCCT BRepFeat / Carve | full vs SW | v0.5 |
| Revolve (full / partial / mid-plane) | profile rotation; analytical surfaces of revolution + NURBS | full | v0.5 |
| Sweep (along path, with twist, with profile orientation lock) | NURBS surface from generalised sweep; The NURBS Book §10.4 | full | v1.0 |
| Loft (multi-section, with guide curves) | NURBS skinning; The NURBS Book §10.3 + Piegl-Tiller refinement | full | v1.0 |
| Helix / spiral / threaded helix | analytical helix curve; pitch / revolutions / height | full | v1.0 |
| Coil (constant / variable pitch) | extension of helix | full | v1.0 |
| Threaded hole (cosmetic + real geometry) | helix + sweep + boolean; ISO/UN/JIS standard libraries | full | v1.0 |
| Hole wizard (counterbored / countersunk / tapered) | param table + sketch + extrude/sweep | full | v1.0 |
| Slot (straight / arc / rectangular) | sketch profile + extrude | full | v0.5 |
| Pad / pocket (sketch-driven) | profile + extrude / boolean-subtract | full | v0.5 |
| Rib | sketch line offset → triangular fill → solidify | full | v1.0 |
| Draft (neutral plane / parting line) | face rotation around edge; OCCT `BRepOffsetAPI_DraftAngle` style | full | v1.0 |
| Shell (uniform / variable thickness) | offset surface inwards; The NURBS Book §10.5 | full | v1.0 |
| Thicken (single face → solid) | offset + lateral surface + cap | full | v1.0 |
| Wrap (emboss / engrave from sketch onto face) | parameterise sketch in face uv → swept solid | full | v2.0 |
| Dome | NURBS bulge from boundary curve | partial | v2.0 |
| Freeform deformation (face control points) | T-spline / lattice deformer | full | v2.0 |
| Fillet (constant / variable / face / full-round / asymmetric) | rolling-ball / NURBS swept blend; Hoffmann §7.4 | full | v1.0 |
| Chamfer (distance-distance / distance-angle / vertex) | analytical chamfer + NURBS for curved edges | full | v1.0 |
| Move face / push-pull (direct edit) | local face displacement + neighbour reconnect | partial | v2.0 |
| Boolean: union / subtract / intersect / split | NURBS-NURBS intersection; Hoffmann §6.7 | full | v0.5 |
| Pattern: linear / circular / sketch-driven / table-driven / curve-driven / fill | feature-graph node with per-instance transform | full | v1.0 |
| Mirror (body / feature / face) | reflection across plane / sketch line | full | v0.5 |

**Per-feature spec contract:**

Every feature ships with: (1) `FeatureSpec` struct in `cadkernel-api` with field-level validation, (2) preview generator (returns `MeshPreview` for in-editor draft display before commit), (3) error catalogue (`FeatureError::SectionEmpty`, `FeatureError::ProfileSelfIntersecting`, `FeatureError::SweepPathHasCusp`, …), (4) edit dialog wireframe, (5) at least 3 regression tests with reference STEP files, (6) Criterion benchmark.

**Fillet algorithm tiers:** rolling-ball for constant-radius edges; NURBS-swept-blend for variable-radius and face-fillet (Choi-Lee 1989); set-back corners via 3-way blend solver (Vida-Martin-Várady 1994); G2 fillets via curvature-continuous boundary blend; conic fillets via Bezier-with-conic-shoulder.

**Boolean robustness:** symbolic-perturbation for degenerate intersection (Yap 1990); interval arithmetic on coordinate computations; topology validation post-op (`Document::validate_solid()` checks manifold + closed + Euler-characteristic).

**Pattern engine:** instance overrides allow per-instance suppression / parameter delta. Linear / circular / sketch-driven / table-driven / curve-driven / fill (auto-distribute on a face). Persistent naming preserved across pattern edits.

**Acceptance:** every feature builds R2 / R3 reproducibly; named-feature regression tests vs SW by importing SW-generated STEP and re-applying our feature, parity ≥ 0.999 face area; failure-mode catalogue documented per feature; preview generation < 100 ms for R3-scale inputs.

**Algorithm refs:** _The NURBS Book_ (Piegl-Tiller) Chs. 10-12; _Geometric and Solid Modeling_ (Hoffmann) Chs. 6-8; OCCT documentation Ch. 5 (Modeling Algorithms); _Robust and Error-Free Geometric Computing_ (Yamaguchi-Tokuyama); Choi-Lee 1989 _Computing Constant-Radius Blending Surfaces_; Vida-Martin-Várady 1994 _A Survey of Blending Methods that use Parametric Surfaces_; Yap 1990 _Symbolic Treatment of Geometric Degeneracies_.

### C-Sketch — sketcher v3 solver

**Mission:** industrial-strength 2D parametric constraint solver, parity with Onshape's PSDA / SolidWorks DCM (Siemens DCM3D OEM).

**Phasing:**

- **v1 (existing)** — 24 constraint types, Newton-Raphson + LM hybrid, ~1,200 LOC. Works for sketches up to ~50 entities; has known edge cases on conics and on under-determined cycles.
- **v2 (Track B4 work)** — drag-edit, in-place dim, DoF readout, external references. Solver kernel unchanged.
- **v3 (this sub-track)** — solver-kernel rewrite. Block decomposition + LM + RRQR + minimal-conflict-set diagnosis + conic and spline polish.

**Constraint coverage (v3 target):**

| Constraint | v1 | v2 | v3 |
|---|---|---|---|
| Coincident (point-point) | ✓ | ✓ | ✓ |
| Coincident (point-curve) | ✓ | ✓ | ✓ |
| Horizontal | ✓ | ✓ | ✓ |
| Vertical | ✓ | ✓ | ✓ |
| Parallel | ✓ | ✓ | ✓ |
| Perpendicular | ✓ | ✓ | ✓ |
| Tangent (line-circle) | ✓ | ✓ | ✓ |
| Tangent (circle-circle) | ✓ | ✓ | ✓ |
| Tangent (curve-curve general) | ✗ | ✗ | ✓ |
| Equal length | ✓ | ✓ | ✓ |
| Equal radius | ✓ | ✓ | ✓ |
| Symmetric (about line) | ✓ | ✓ | ✓ |
| Symmetric (about point) | ✗ | ✓ | ✓ |
| Concentric | ✓ | ✓ | ✓ |
| Distance (linear / aligned / horiz / vert) | ✓ | ✓ | ✓ |
| Distance (point-curve) | ✗ | ✓ | ✓ |
| Angle (line-line) | ✓ | ✓ | ✓ |
| Angle (curve-tangent) | ✗ | ✗ | ✓ |
| Radius / diameter | ✓ | ✓ | ✓ |
| Curvature radius | ✗ | ✗ | ✓ |
| Fix | ✓ | ✓ | ✓ |
| Block (group rigid) | ✗ | ✓ | ✓ |
| On-curve (point on conic) | ✗ | ✓ | ✓ |
| Conic focus / directrix | ✗ | ✗ | ✓ |
| Spline G1 / G2 / G3 | ✗ | partial | ✓ |
| Pattern (within-sketch repeat) | ✗ | ✓ | ✓ |
| Mirror (within-sketch) | ✗ | ✓ | ✓ |
| Polynomial / driving formula | ✗ | ✗ | ✓ |

**Goal:** industrial-strength constraint solver.

**Deliverables:**

1. **System classification** pre-solve: fully-determined / under / over / inconsistent. DoF count per equation block.
2. **Block partitioning** — strongly-connected components in the variable-equation bipartite graph; solve sub-blocks independently.
3. **Constraint deflation** — once a sub-block is solved, project residual onto remaining unknowns.
4. **Solver kernel** — Levenberg-Marquardt with Gauss-Newton fallback; rank-revealing QR for Jacobian; line search with Wolfe conditions.
5. **Conic constraints** — ellipse / parabola / hyperbola tangency; focus-directrix construction.
6. **Spline tangency / curvature continuity** — G1 / G2 / G3 boundary conditions.
7. **Pattern constraint** — rigid copy of a sub-sketch.
8. **Symmetry constraint** — about an arbitrary axis.
9. **Diagnostics** — when over-constrained / inconsistent, return a minimal conflict set (subset of constraints whose removal restores well-determinedness).
10. **Property test corpus** — 10 000 random valid sketches via `proptest`; solver must converge in ≤ 100 iter or report under-determined / inconsistent with a minimal conflict set.

**Acceptance:** corpus 100 % converge or report conflict; R2 sketches all DoF=0; over-constrained synthetic test produces a minimal conflict set within 100 ms.

**Algorithm refs:** Bouma-Fudos-Hoffmann-Cai-Paige 1995 (graph-based geometric constraint solving); _Numerical Optimisation_ (Nocedal-Wright) Chs. 4 / 10; Higham §15.6 (rank-revealing QR).

### C-Surface — Class-A surfacing

**Mission:** automotive / consumer-product surfacing parity (CATIA GSD / ICEM Surf class). The hardest sub-track; competes with the most mature commercial code.

**Phasing:**

- **v1.0** — basic ruled / extruded / revolved / sweep-1-rail / loft-2-section surfaces; planar / cylindrical / spherical / conical analytical primitives; trim / untrim; offset; extend (G0).
- **v2.0** — n-sided patch (Coons / Gregory); fill (G1); blend (G2); knit / sew / shell-from-surfaces; reflection-line analysis; zebra stripes; Gaussian / mean curvature heatmap; iso-curve extraction.
- **v3.0** — Class-A blends (G2 / G3 between trimmed surfaces); fairing; control-point editing of NURBS; surface-from-mesh (linked to C-Reverse); reverse-engineering scan-to-surface; dynamic-loft (live-edit cross-section).

**NURBS algorithm shopping list:**

| Operation | Algorithm | Reference |
|---|---|---|
| Curve / surface evaluation | de Boor | NURBS Book §2.5 |
| Knot insertion | Boehm | §5.2 |
| Knot removal | Tiller | §5.4 |
| Degree elevation | Prautzsch | §5.5 |
| Curve interpolation | global / local cubic | §9.2 |
| Surface interpolation | bicubic Coons | §9.3 |
| Loft (skinning) | NURBS skinning | §10.3 |
| Sweep | generalised sweep | §10.4 |
| Offset | Marker-Sederberg | §10.5 |
| Intersection | NURBS-NURBS subdivision + Newton | Patrikalakis Ch. 4 |
| Trimming | UV parameter trimming | NURBS Book §10.6 |
| Fillet | rolling-ball + swept-blend | Choi-Lee 1989 |
| Class-A blend | G2 boundary patch | Vida-Martin-Várady 1994 |
| Curvature analysis | mean / Gaussian / principal | Farin §11.3 |
| Reflection line | iso-light projection | Farin §16.4 |
| Zebra | iso-tangent strip | Beier-Chen 1994 |
| Fairing | minimum-energy spline | Hoschek-Lasser §6.6 |

**Goal:** automotive / consumer-product surfacing parity (CATIA GSD class).

**Deliverables:**

1. **Ruled surface** between two curves.
2. **Boundary surface (n-sided patch)** — Coons / Gregory patches.
3. **Fill surface** — fill an n-sided hole in a shell with G1 / G2 continuity.
4. **Knit / stitch / sew** — combine surfaces into a shell / solid.
5. **Trim / untrim** — boundary curve refinement.
6. **Extend** — extrapolate a surface along its boundary.
7. **Offset** — parallel surface; The NURBS Book §10.5.
8. **Class-A blends** — G2 between trimmed surfaces.
9. **Reflection-line analysis** — render iso-light reflection lines for surface quality inspection.
10. **Curvature visualization** — Gaussian / mean / principal curvature heatmap.
11. **Zebra stripes** — interactive tangent-continuity inspection.

**Acceptance:** import a NURBS surface set from a Class-A reference part; trim / blend / fill survives reflection-line analysis at G2.

**Algorithm refs:** _The NURBS Book_ Chs. 9 / 10 / 12; Farin _Curves and Surfaces for CAGD_ Chs. 16-18; Sederberg Ch. 4.

### C-Assy — assembly + mechanism

**Mission:** SolidWorks-class assembly modelling with full mate library, configurations, design tables, BOM, exploded views, and mechanism preview.

**Phasing:**

- **v1.0** — full mate library (16 types below), DoF analysis, motion preview (single-DoF drag), interference detection, BOM, smart fasteners, sub-document references (linked to A8), sub-assembly hierarchy (10 levels deep), suppress / unsuppress, lightweight / resolved / virtual modes.
- **v2.0** — configurations (named alternate states), design tables (Excel-grid driven), pattern-driven components, mirror-component, equation-driven mate values, advanced motion (multi-DoF gear / cam coupled).
- **v3.0** — full mechanism dynamics (linked to C-Mech), top-down design (in-context part editing), envelope components, asset bins, large-assembly handling for 10k+ component assemblies.

**Mate library:**

| Type | DoFs removed | Algorithm |
|---|---|---|
| Coincident | 1-3 (point/line/face) | linear constraint |
| Parallel | 1-2 | quaternion alignment |
| Perpendicular | 1 | dot product = 0 |
| Concentric | 2-3 | axis alignment + radial pin |
| Distance | 1 | linear |
| Angle | 1 | quaternion angle |
| Tangent | 1 | NURBS-curve tangency |
| Width | 1-2 | mid-plane constraint |
| Symmetric | 3 | mirror about plane |
| Lock | 6 | rigid join |
| Gear | 1 | ratio constraint |
| Rack-pinion | 1 | linear-rotational coupling |
| Cam | 1 | curve follower |
| Hinge | 5 | revolute joint |
| Slot | 4-5 | sliding-axis constraint |
| Universal | 4 | two-perpendicular-axes |

**Solver:** block-Newton with projection onto constraint manifold. Sparse Jacobian via `nalgebra-sparse`. CG-based linear solve.

**Other deliverables:**

1. **DoF analysis** — per-component DoF readout.
2. **Motion preview** — drag a free DoF; integrate ODE if dynamic.
3. **Interference detection** — pairwise BVH overlap test; visual highlight.
4. **Configurations** — named alternate states (mate variations, suppressed parts, parameter sets).
5. **Design tables** — Excel-like grid driving configurations; row = configuration, column = parameter.
6. **Sub-document references** (linked to A8).
7. **Smart fasteners** — auto-place screws / nuts / washers in matching holes from standard library (DIN / ISO / ANSI).
8. **BOM** — `Assembly::bom() -> Vec<BomEntry>` with per-component qty + part-number; consumed by C-Draw.
9. **Exploded view** with paths (linked to B2).

**Acceptance:** R3 mates correctly; R11 (250-component) opens, mates resolve, motion preview at 30 fps; interference detection reports correct overlapping pairs.

**Algorithm refs:** _Multibody Dynamics_ (Featherstone), _Numerical Optimization_ (Nocedal-Wright) §10.3, OCCT XCAF documentation.

### C-Mech — mechanism / motion / dynamic

**Mission:** prototype-quality motion simulation. Not a competitor to ADAMS / RecurDyn but enough for design-validation tasks.

**Phasing:**

- **v2.0** — kinematic only (joints + drivers + IK + animation export). No forces.
- **v3.0** — forward dynamics, contact (rigid), basic flexible body (modal reduction), springs / dampers, gravity, force / torque drivers, frequency-response analysis.

**Goal:** prototype-quality motion simulation.

**Deliverables:**

1. **Joint library** — revolute / prismatic / cylindrical / planar / spherical / fixed.
2. **Driver functions** — constant velocity / sine / linear / piecewise / data-table.
3. **Forward dynamics** — `featherstone-rs` (decision per IP audit) or in-house articulated-body algorithm.
4. **Inverse kinematics** — Jacobian-pseudoinverse for under-actuated chains.
5. **Contact (rigid)** — for cam followers, gear meshing.
6. **Animation export** — keyframed `.cadk` Animation blob; played back by Track B exploded-view system.
7. **Time-step recording** — record per-step pose; rewind; export to MP4 via `ffmpeg` subprocess.

**Acceptance:** four-bar linkage R-test (synthesises a Watt linkage path within 0.5 mm of analytical); Stanford-arm R-test (6-DoF reach to target within 0.1 mm IK tolerance); gear-train timing test (compound gear ratio matches analytical to 1e-6); contact-pair drop-test (rigid sphere on inclined plane converges to known steady state).

**Algorithm refs:** Featherstone _Rigid Body Dynamics Algorithms_ Ch. 4 (articulated-body); Mirtich _Impulse-based Simulation_; Anitescu-Potra (LCP for contact); Shabana _Computational Continuum Mechanics_ for flexible-body MOR.

### C-Sheet — sheet metal

**Mission:** SolidWorks Sheet Metal / Inventor Sheet Metal parity. The single most-used workbench in many manufacturing firms.

**Phasing:**

- **v2.0** — base flange / edge flange / miter / hem / jog / sketched-bend / fold-unfold / flat-pattern / K-factor table / corner-relief; lofted-bend; full forming-tool library.
- **v3.0** — auto-relief at intersecting bends; multi-body sheet metal; cut-list generation; nesting (within sheet for nesting-saver workflow); tab-and-slot generator; gusset.

**Deliverables:**

1. **Base flange** from sketch with thickness + bend allowance.
2. **Edge flange** along a chosen edge; angle / length / radius.
3. **Miter flange** along multiple edges.
4. **Hem** — open / closed / teardrop / rolled.
5. **Jog** (Z-bend) — offset distance + bend radius.
6. **Sketched bend** along a line.
7. **Fold / unfold** for selected bends.
8. **Flat pattern** — full unfold with bend-line overlay.
9. **K-factor** — gauge tables (US 3-22 GA, metric 0.1-10 mm); user-editable; per-material defaults.
10. **Bend allowance / deduction** formulas configurable.
11. **Corner relief** — round / square / tear; auto-applied at junctions.
12. **Lofted bend** — between two non-parallel sketches.
13. **Rip** — split a sheet model.
14. **Closed corner** — extend overlapping flanges to meet.
15. **Cross break** — cosmetic stiffening lines.
16. **Forming tool library** — louvers, embosses, beads.

**Acceptance:** R5 builds with all six bend types; flat pattern matches manufacturing reference (tolerance ≤ 0.1 mm).

**Algorithm refs:** Wagoner-Chenot _Metal Forming Analysis_ Ch. 3 (bend allowance); SME Sheet Metal Handbook.

### C-Weld — weldments + frame generator

**Mission:** Inventor Frame Generator parity. Structural-frame design from a 3D sketch path.

**Phasing:**

- **v2.0** — structural members from sketch, full library of standard profiles (rect tube / square tube / angle / channel / I-beam / circular tube — DIN / ISO / ANSI families), trim / extend, gusset, end-cap, weld bead (cosmetic), cut-list export.
- **v3.0** — sub-structural-member-pattern, custom-profile via sketch import, weld-bead solid (for FEM), full Inventor-grade frame generator UX.

**Deliverables:**

1. **Structural member** along a 3D sketch path; profile from library (rect tube, square tube, angle, channel, I-beam, custom from sketch).
2. **Trim / extend** corner treatment: end-butt / mitered / coped.
3. **Gusset** at a junction.
4. **End cap** auto-fit.
5. **Weld bead** — analytical surface along a chord; cosmetic for drawings, optional solid for FEM.
6. **Cut list** — per-member length + angle + profile, exportable as CSV / PDF.
7. **Frame generator** UX (Inventor parity) — pick segments of a 3D sketch, apply a profile, library auto-handles corners.

**Acceptance:** R6 builds with full cut list; matches Inventor-generated reference within 0.1 mm.

### C-Mold — mold tooling

**Mission:** Mold-tooling workbench parity with SolidWorks Mold Tools / Cimatron / Moldex. Practical for sub-100-cavity injection-mold design.

**Phasing:**

- **v2.0** — parting-line detection from draft analysis, manual parting-surface, shut-off, core-cavity split, ejector-pin pattern, simple slide / lifter, mold-base library (DME / HASCO standard sets), runner / gate sketch.
- **v3.0** — automatic parting-line / parting-surface generation, multi-shot tooling, conformal cooling channels, ejector-collision check, mold-base parametric assembly auto-fit, plastic-flow simulation hand-off (linked to C-Sim).

**Deliverables:**

1. **Parting line detection** — automatic from draft analysis.
2. **Parting surface** — extrude parting line.
3. **Shut-off surface** — close openings on the parting plane.
4. **Core / cavity split** — boolean operations from parting surface.
5. **Ejector pin layout** — pattern of holes.
6. **Slide / lifter** — for undercuts.
7. **Runner / gate / cooling channel** sketcher.
8. **Mold base library** — DME / HASCO / Futaba standards.

**Acceptance:** R7 produces core + cavity + ejectors at industry tolerance.

**Algorithm refs:** Beaumont _Successful Injection Molding_; Menges _How to Make Injection Molds_ Chs. 3-5.

### C-Draw — drawings v2 + GD&T + MBD

**Mission:** issuable manufacturing drawings + Model-Based Definition. The output that makes CAD usable in industry.

**Phasing:**

- **v1.0** — multi-view layouts, dimensions, basic GD&T, hatching, BOM, title block, sheet templates, layers, annotations, PDF / DXF / SVG export.
- **v2.0** — full ASME Y14.5-2018 GD&T (composite frames, datum modifiers, profile-of-line / surface, runout), Y14.41 MBD (PMI on 3D model + STEP AP242 round-trip), revision tables, ECO linking, 3D PDF export with PRC.
- **v3.0** — automatic dimensioning (heuristic), CAD-DWG block library, dynamic-block parametric blocks, full DWG round-trip via in-house parser (linked to A4 / IO).

**Multi-view layouts:**

1. Front / Top / Right / Iso / Named-view auto-aligned.
2. Section view from a sketch line on parent.
3. Detail view from a region.
4. Auxiliary view perpendicular to a chosen edge.
5. Crop view bounded by closed sketch.
6. Broken-out section.
7. Empty view (for sketch-only overlays).
8. Predefined view templates.

**Dimensions:**

1. Linear / aligned / angular / radial / diametric / chamfer / arc-length.
2. Ordinate dimensioning.
3. Baseline / chain / continuous.
4. Auto-dimension (heuristic).

**GD&T (per ASME Y14.5-2018):**

1. Tolerance frames: flatness, straightness, circularity, cylindricity.
2. Orientation: parallelism, perpendicularity, angularity.
3. Position, concentricity, symmetry.
4. Profile: line / surface.
5. Runout: circular / total.
6. Datum reference frames; modifiers (`Ⓜ`, `Ⓛ`, `Ⓟ`).
7. Composite tolerance frames.
8. Surface finish symbols (Ra, Rz, lay direction).
9. Weld symbols (full ANSI / AWS A2.4).
10. Balloons, leader lines, datum targets.

**Other:**

1. Hatching with per-material patterns; section hatch auto-applied.
2. BOM table (from C-Assy), sortable, reorder columns, per-row notes.
3. Revision table, ECO-linked.
4. Title block — templated, document-property-bound.
5. Sheet templates: A0 / A1 / A2 / A3 / A4, ANSI A-D, custom.
6. Layers — visibility / colour / line-style.
7. Annotations: text, rich text, balloons, leader, weld, surface finish, GD&T frames.
8. Centerlines: face / lines / points / bolt-circle.

**Export:**

1. PDF with embedded fonts.
2. DXF (R2010+).
3. SVG.
4. PNG / TIFF rasters.

**MBD (Y14.41):**

1. PMI annotations directly on 3D model.
2. Saved with `.cadk`; round-trip via STEP AP242.
3. 3D PDF export with PRC.

**Acceptance:** R3 drawing is industry-issuable; PDF export matches a reference manufacturing drawing visually + textually; AP242 round-trip preserves PMI.

**Algorithm refs:** ASME Y14.5-2018 specification; ISO 1101:2017; SolidWorks DraftSight as visual reference.

### C-Sim — FEM + thermal + vibration + plastics + CFD-lite

**Mission:** prototype-stage simulation. Not a SimXpert / NX Nastran replacement; sufficient for design verification on R1-R8 class problems.

**Phasing:**

- **v1.0** — linear-elastic static (`SimulationStudy { mesh, BCs, materials, results }`); built-in mesher; CG + AMG solver; result rendering (displacement / stress / Von Mises).
- **v2.0** — modal frequency, linear buckling, nonlinear static (geometric + material), thermal (steady-state + transient), coupled thermal-stress, contact (penalty), basic plasticity (J2 von Mises kinematic / isotropic hardening).
- **v3.0** — plastics flow (Hele-Shaw / 3D), CFD-lite (k-ω SST internal flow + drag estimate external), explicit dynamics (drop test), topology optimisation (SIMP + level-set with manufacturing constraints).

**Linear-elastic static:**

1. Mesher — tetrahedral; Delaunay refinement; Laplacian smoothing; quality metrics (radius ratio, dihedral angle).
2. Solver — Conjugate Gradient with AMG preconditioner; sparse `nalgebra-sparse` + `suite-sparse` (decision per IP audit, OR `Eigen-sys`).
3. Boundary conditions — fixed / sliding / displacement / pressure / point force / distributed force / bearing load / gravity.
4. Materials — isotropic linear elastic; database with steel / aluminum / titanium / ABS / polyethylene / glass.
5. Results — displacement / stress / strain / Von Mises / principal / equivalent.

**Modal frequency:** ARPACK eigenvalue solve; first-N modes.

**Linear buckling:** linearised eigenvalue problem.

**Nonlinear static:** Newton-Raphson with line search; large deformation (Total Lagrangian); contact (penalty); plasticity (J2 von Mises, kinematic / isotropic hardening).

**Thermal:**

1. Steady-state.
2. Transient (Crank-Nicolson).
3. Coupled thermal-stress.
4. Convection / radiation / conduction BCs.

**CFD-lite (v3):**

1. Pipe flow (1D Bernoulli + losses).
2. Internal flow with k-ω SST turbulence.
3. External flow (drag estimate).

**Plastics flow (v3):** Hele-Shaw / 3D mid-plane simulation; fill / pack / cool phases.

**Topology optimization:** SIMP / level-set; manufacturable-constraint extension.

**Result rendering:** colour bands; isosurfaces; deformed shape with scale slider; animated mode shape.

**Acceptance:** R1 cantilever ≤ 5 % vs analytical beam-bending; R8 plastics warpage within ±10 % vs Moldflow.

**Algorithm refs:** Bathe _Finite Element Procedures_ Chs. 4 / 6 / 8 / 10; Zienkiewicz-Taylor _The Finite Element Method_ Chs. 8 / 12 / 16; Belytschko _Nonlinear Finite Elements for Continua and Structures_ Chs. 5-7; Wilkins _Mesh Generation_ Chs. 2-4.

### C-CAM — toolpaths + post-processors

**Mission:** Fusion 360 Manufacture / Mastercam-class CAM. The single most-revenue-generating workbench for many CAM users.

**Phasing:**

- **v2.0** — 2.5-axis (face / shell / pocket / contour / drill / tap / bore / chamfer / engrave / adaptive-clearing / trochoidal / rest-machining); holder-collision check; voxel-based material-removal sim; Haas / Fanuc / Heidenhain posts; tool library (ISO 13399 import); machine simulation framework (basic).
- **v3.0** — 3-axis finishing strategies (parallel / scallop / pencil / spiral / radial / morph); 5-axis simultaneous (swarf / multi-axis-contour / deburr / barrel-tool); turning (facing / OD / ID / threading / parting / grooving); multi-turret coordination; Swiss-style; Mazak / Siemens posts; full-machine kinematic simulation; in-process gauging.

**2.5-axis (v2):**

1. Face mill / shell mill / pocket / contour / drill / tap / bore / chamfer / engrave.
2. Adaptive clearing.
3. Trochoidal milling.
4. Rest machining.
5. Holder collision check.
6. Material removal simulation.

**3-axis (v2):**

1. Parallel finishing / scallop / pencil / spiral / radial / morph between curves.

**5-axis simultaneous (v3):** swarf / multi-axis contour / deburr / barrel-tool finishing.

**Turning (v3):** facing / OD / ID / threading / parting / grooving; multi-turret coordination; Swiss-style.

**Post-processors:**

| Controller | Std | Status |
|---|---|---|
| Haas Mill / Lathe | v2 | shipped at v2.0 |
| Fanuc 0i / 30i | v2 | shipped |
| Heidenhain TNC640 | v2 | shipped |
| Mazak Mazatrol | v3 | shipped at v3 |
| Siemens 840D | v3 | shipped |

**Toolpath verification:** voxel material-removal simulation; collision; gouge detection.

**Acceptance:** R9 produces post-processed G-code that runs on Haas simulator without warnings; round-trip vs Fusion CAM benchmark.

**Algorithm refs:** Choi-Jerard _Sculptured Surface Machining_ Chs. 4-6; Held _On the Computational Geometry of Pocket Machining_; LinuxCNC G-code interpreter for compliance reference.

### C-Render — photoreal

**Mission:** product-shot quality visualisation. KeyShot / V-Ray-class basic, not full V-Ray feature parity.

**Phasing:**

- **v1.0** — PBR + IBL real-time (linked to Track B8); HDRI library; built-in studio scenes; render-to-image at 4k.
- **v2.0** — GPU path-tracing via wgpu compute; SVGF denoiser; ReSTIR; depth-of-field; motion-blur; spectral rendering option.
- **v3.0** — walkthrough / VR (OpenXR), animation render queue, network rendering (multiple machines), USD / glTF Khronos PBR export, decals, animated materials.

**Real-time path tracing (v3):** GPU PT via wgpu compute; SVGF denoiser; ReSTIR.

**Materials:** Disney BRDF; subsurface scattering; thin-film interference; clearcoat.

**Decals:** image projection; UV-mapped.

**Scenes:** HDRI library; product-shot studio; outdoor; user-loadable.

**Cameras:** depth of field; chromatic aberration; bloom; tone mapping (ACES, Reinhard).

**Walkthrough / VR (v3):** OpenXR via `openxr 0.18`; tracked-controller selection; teleport navigation.

**Acceptance:** product-shot rendered in ≤ 30 s for R3; walkthrough at 90 fps in headset.

### C-Reverse — mesh→surface, healing, optimisation

**Mission:** scan-to-CAD workflow + topology optimisation manufacturable output.

**Phasing:**

- **v2.0** — STL / OBJ / PLY import (existing); mesh repair (fill holes / remove non-manifold / smooth); auto-segment by curvature region-grow; surface fit per region (plane / cylinder / cone / NURBS); stitch / shell; quality-report.
- **v3.0** — advanced fitting (B-spline of arbitrary topology via Eck-Hoppe); semi-auto segment refinement; full topology-optimisation workflow (load case + design space + manufacturability constraints + in-house solver → solid output); generative-design optimiser hand-off.

1. **STL / OBJ / PLY import** (existing).
2. **Mesh repair** — fill holes; remove non-manifold; smooth.
3. **Auto-segment** — region-grow by curvature.
4. **Surface fit** per region — plane / cylinder / cone / NURBS.
5. **Stitch / shell** — join fitted patches.
6. **Quality report** — fit RMS error per region.
7. **Topology optimization workflow** — load case + design space + manufacturability constraint → solid output.

**Acceptance:** R10 produces a clean B-Rep solid from a generative mesh; fit RMS ≤ 0.05 mm.

**Algorithm refs:** Cohen-Steiner _Variational Shape Approximation_; Eck-Hoppe _Automatic Reconstruction of B-Spline Surfaces of Arbitrary Topology_; Hoppe _Surface Reconstruction from Unorganized Points_.

### C-Subdiv — T-spline / freeform

**Mission:** Fusion 360 freeform-modelling parity for organic / ergonomic shapes.

**Phasing:**

- **v2.0** — T-spline surface from polyhedral cage; basic crease / sharpen / unsharpen; convert to NURBS (Sederberg-Zheng-Bakenov-Nasri 2003); cage editing (push / pull / crease).
- **v3.0** — advanced symmetry, mirror, weld, bridge, edge-flow controls, automatic-conversion-to-Class-A surface, integration with C-Surface.

1. T-spline surface from polyhedral cage.
2. Crease / sharpen / unsharpen.
3. Convert to NURBS (Sederberg-Zheng-Bakenov-Nasri).
4. Edit handles in 3D viewport.

**Acceptance:** parity with Fusion 360 T-spline freeform on a teapot, a phone case, a faucet handle.

### Track C summary

| Sub-track | v0.5 | v1.0 | v2.0 | v3.0 |
|---|---|---|---|---|
| C-Solid | core ops | full feature suite | direct edit, advanced fillets | freeform / dome |
| C-Sketch | v1 (existing) | v2 (drag + dim + ext-ref) | v3 solver + spline G2 | conics polished |
| C-Surface | nothing new | basic ruled / blend | Class-A blends | reflection-line clean |
| C-Assy | nothing new | full mate set + DoF + motion | configurations + design tables | full SW parity |
| C-Mech | — | — | basic motion | full mechanism |
| C-Sheet | — | — | full sheet metal | polished |
| C-Weld | — | — | structural members | full Inventor parity |
| C-Mold | — | — | partial | full |
| C-Draw | — | multi-view + GD&T | MBD | full |
| C-Sim | — | linear static | nonlinear + thermal | plastics + CFD-lite |
| C-CAM | — | — | 2.5-axis + posts | 5-axis + turning |
| C-Render | — | PBR | path-traced | walkthrough / VR |
| C-Reverse | — | — | mesh→surf + topo opt | full |
| C-Subdiv | — | — | T-spline basic | full |

---

## 7. Track D — Trust

The quality system. Track D runs continuously through every release; not a phase to be completed but a discipline to be sustained.

### D1 — Performance budget

Every user-visible operation has a numeric budget per reference HW (M1 / Iris-Xe-class laptop, 2020+ vintage, 16 GB RAM, integrated GPU). CI fails on > 10 % regression vs the budget rolling baseline. Criterion benchmarks back every row.

**Reference HW:**

- **Linux baseline**: ThinkPad X1 Carbon Gen 9, Intel i7-1165G7, Iris Xe, 16 GB, Ubuntu 24.04 LTS, mesa 24.x.
- **macOS baseline**: M1 MacBook Air, 16 GB, macOS 14.x, Metal 3.
- **Windows baseline**: Surface Pro 9, Intel i7, Iris Xe, 16 GB, Win 11 23H2, DX12.
- All three reproduced in CI (GitHub Actions ubuntu-22.04 + macos-14 + windows-2022; macos-14 covers M1 ARM).

**Per-operation budgets:**

| Operation | Budget | Reference HW |
|---|---|---|
| First paint (cold start) | < 500 ms | M1 / Iris Xe |
| First paint (warm start) | < 200 ms | same |
| Empty document open | < 100 ms | same |
| R1 open (50-feature bracket) | < 250 ms | same |
| R2 open (200-feature mechanical) | < 1 s | same |
| R3 open (1000-feature housing) | < 3 s | same |
| R4 open (sheet metal box, 30 bends) | < 1 s | same |
| R5 open (weldment frame, 80 members) | < 2 s | same |
| R7 open (mold tool, 200 features) | < 4 s | same |
| R11 open (250-component assembly) | < 10 s | same |
| R12 open (10k-feature, lazy) | < 30 s | same |
| Sketch entity add (50-entity sketch) | 60 fps | same |
| Sketch drag (50-constraint sketch) | 60 fps; 16 ms p99 | same |
| Sketch solve (100-entity sketch) | < 50 ms | same |
| Sketch solve (R12 1k-entity sketch) | < 1 s | same |
| Recompute R1 dim change | < 50 ms | same |
| Recompute R2 dim change | < 200 ms | same |
| Recompute R3 dim change | < 1 s | same |
| Recompute R12 dim change (incremental) | < 2 s | same |
| Tessellation 1k-face shell | < 50 ms | same |
| Tessellation R3 (~50k face) | < 500 ms; parallel | same |
| Selection pick (any depth) | < 16 ms p99 | same |
| Box-select 50 features | < 100 ms | same |
| Hover preview | < 50 ms first paint | same |
| Property field commit | < 250 ms (debounce + recompute) | same |
| Undo R1 | < 50 ms | same |
| Undo R3 | < 100 ms | same |
| Undo R12 (with caching) | < 1 s | same |
| Save R3 to .cadk | < 1 s | same |
| Save R12 to .cadk | < 5 s | same |
| Load R3 from .cadk | < 2 s | same |
| STEP export R3 | < 5 s | same |
| STEP import R3 | < 8 s | same |
| 30 fps R3 view at 1080p | sustained | same |
| 60 fps R1 view at 1080p | sustained | same |
| 60 fps R3 view at 1080p (post-B8 polish) | sustained | same |
| 30 fps R11 view at 1080p (lightweight mode) | sustained | same |
| Rendering frame budget breakdown | g-buffer 4 ms / shadow 3 ms / SSAO 2 ms / lighting 3 ms / TAA 1 ms / post 2 ms = ~16 ms total | same |
| Memory R3 working set | < 2 GB | same |
| Memory R12 working set (lazy) | < 4 GB | same |
| Memory leak (24-h soak test) | 0 bytes growth | same |

CI fails on > 10 % regression. Criterion benchmarks per row.

### D2 — Robustness

**Mission:** zero panics on public APIs; deterministic behaviour on degenerate input; defended against adversarial files.

**Boolean fuzz:**

- 1 M random pairs across 12 primitives × 12 primitives × random transforms.
- 0 crashes; 0 invalid topology results across two consecutive runs.
- Topology validation post-op: manifold check (every edge has ≤ 2 incident faces), closed shell (every edge boundary), Euler-characteristic match.
- Failures categorised: numerical degeneracy, intersection ambiguity, self-intersection. Each category has a documented response (symbolic perturbation / fallback / refuse with informative error).

**File format fuzz:**

- `cargo-fuzz` on every IO crate (STL / OBJ / glTF / STEP / IGES / DXF / PLY / 3MF / BREP / DWG / DAE / .cadk).
- 100 GPU hours fuzz time per release; 0 panics on malformed input; 0 OOM (bounded memory per parse).
- Corpora seeded from open-source CAD repositories + handcrafted edge cases.
- Coverage-guided via `libfuzzer-sys`.

**Sketch solver `proptest`:**

- 10 k random valid sketches per release.
- Solver must converge in ≤ 100 iter or report under-determined / inconsistent with a minimal conflict set.
- Conflict-set test: random over-constrained sketches; assert solver returns subset that, when removed, restores well-determinedness.

**Public-API panic audit:**

- clippy `unwrap_used`, `expect_used`, `panic`, `unreachable`, `unimplemented`, `todo` denied in `cadkernel-api`, `-modeling`, `-geometry`, `-topology`, `-sketch`, `-io`.
- Internal-only code (e.g., `#[cfg(test)]`) exempt.
- Unsafe code: 0 occurrences in non-`-math`, non-`-viewer` crates; reviewed and isolated where allowed.

**Memory:**

- Per-edit allocation O(features-touched), not O(document).
- 24-h soak test: open R3, edit-undo-redo loop, 0 byte growth.
- Heap profile via `dhat-rs` per release.

**Threadsafety:**

- Every `Send + Sync` claim verified by `static_assertions::assert_impl_all`.
- Loom-based concurrency tests on shared kernel state.
- Rayon usage audit per release.

**Determinism:**

- Same Document + same edit on same machine → identical hash.
- Cross-platform deterministic: M1, Iris Xe, Win agree on `Document::canonical_hash()` for R1-R3.
- Floating-point: no `f64::NAN` propagation; `total_cmp` everywhere comparison required.

### D3 — Test infrastructure

**Mission:** every kind of regression is catchable in CI before merge.

**Test categories and tooling:**

| Category | Framework | Cadence | Owner |
|---|---|---|---|
| Unit | rust built-in | per-PR | per-crate |
| Integration | `cadkernel-api` `tests/` | per-PR | api |
| Doc tests | rustdoc | per-PR | per-crate |
| Property | `proptest 1.x` | per-PR + nightly long-run | per-crate |
| Mutation | `cargo-mutants` | weekly | qa |
| Fuzz | `cargo-fuzz` | nightly continuous | qa |
| Reference parts | scripted via `cadkernel-api` | per-PR | qa |
| Visual regression | `pixelmatch` against committed baselines | per-PR | ui |
| Interaction | viewer-tests scripted input | per-PR | ui |
| A11y | AccessKit automated check | per-PR | ui |
| i18n | string-coverage CI | per-PR | ui |
| Industry compliance | NIST CAX / ASME Y14.5 / JT B-Rep / LinuxCNC G-code | nightly | qa |
| Performance | criterion vs rolling baseline | per-PR | qa |
| Soak (24 h) | nightly long-run | nightly | qa |
| Cross-platform parity | identical hash on M1 / Iris Xe / Win | per-PR | qa |
| Fuzz coverage | tarpaulin + `kcov` | nightly | qa |
| GUI integration | `viewer --script <log.json>` headless | per-PR | ui |

**Reference-part R1-R12 build scripts** in CI at every PR. Each runs to completion in < 5 s (R1-R3) / < 30 s (R4-R10) / < 60 s (R11-R12). Failure or timing > 10 % over baseline blocks merge.

**Industry compliance suites:**

- **NIST CAX-IF** — round-trip every published test corpus through STEP AP214 + AP242; pass rate ≥ 95 % at v1.0, ≥ 99 % at v2.0, 100 % at v3.0.
- **ASME Y14.5 GD&T validation** — in-house corpus of 200 parts with PMI; export to AP242, reimport, assert PMI fidelity ≥ 99 %.
- **JT B-Rep Annex conformance** — pass JT 10.7 conformance tests for Class-A, B, C lattice elements.
- **LinuxCNC G-code regression** — generated G-code parses cleanly through LinuxCNC interpreter; runs under simulation without warnings.
- **DXF AC1027 conformance** — round-trip through AutoCAD 2018+ via in-house DXF parser; entity preservation ≥ 99 %.

**CI matrix:**

- Linux: ubuntu-22.04 (primary), ubuntu-24.04 (latest).
- macOS: macos-14 (M1 / ARM, primary), macos-13 (Intel).
- Windows: windows-2022.
- Rust toolchain: stable, beta, nightly (informational).
- Per-OS surface: full workspace + viewer GUI integration tests + reference parts.

**CI dashboard:** https://ci.cadkernel.dev (publicly viewable, not gated).

### D4 — Distribution & packaging

**Mission:** install in 60 s on every supported platform; reproducible byte-identical builds; signed; auto-updatable.

**Per-platform packaging matrix:**

| Platform | Format | Code-signing | Distribution channel |
|---|---|---|---|
| Linux x86_64 | AppImage | Sigstore cosign | direct + AppImageHub |
| Linux x86_64 | .deb (Debian/Ubuntu) | gpg | direct + ppa.cadkernel.dev |
| Linux x86_64 | .rpm (Fedora/RHEL) | gpg | direct + copr.fedoraproject.org |
| Linux ARM64 | AppImage + .deb + .rpm | Sigstore cosign | direct |
| Linux | snap | snap-store-signed | snapcraft.io |
| Linux | flatpak | flathub-signed | flathub.org |
| macOS x86_64 + ARM64 | universal .dmg | Apple Developer ID Application + notarised | direct + brew |
| macOS | .pkg installer | Apple Developer ID Installer + notarised | direct |
| Windows x86_64 | .msi | Authenticode SHA-256 + EV cert + RFC 3161 timestamp | direct + winget |
| Windows | .exe portable | Authenticode | direct |
| Container | Docker minimal Debian (headless) | Sigstore cosign | docker.io/cadkernel/headless |

**Auto-update:**

- Channels: stable / beta / nightly. User-selectable.
- Delta updates via rsync rolling-hash (zsync-like protocol).
- Integrity-verified before apply (signature + hash chain).
- Background download; apply on next launch.
- Rollback on launch failure (fallback to N-1 release).
- Opt-out preserves old binary indefinitely.

**Reproducible builds:**

- `cargo build --locked --release` from a tag → bit-identical binaries on canonical builders.
- `SOURCE_DATE_EPOCH` honoured throughout.
- Build matrix: each OS has one canonical builder (latest LTS), bit-for-bit reproducibility CI test.
- Build attestation via SLSA Level 3 (artifact provenance; build platform integrity).

**Crate publishing:**

- `cadkernel-api`, `cadkernel-mcp`, `cadkernel-math`, `cadkernel-geometry`, `cadkernel-topology`, `cadkernel-modeling`, `cadkernel-sketch`, `cadkernel-io`, `cadkernel-core`. Per-release, all crates published to crates.io.
- Versioning: synchronised across all crates per release.
- Yanking: only on critical security fixes; with replacement immediately published.
- Crate documentation hosted on docs.rs.

**Python wheel:**

- Built via `maturin` for Python 3.10 / 3.11 / 3.12 / 3.13.
- Manylinux 2014 / macOS x86_64 / macOS arm64 / Windows x64.
- Published to PyPI as `cadkernel`.
- Version-locked to Rust workspace version.

**Container:**

- Minimal Debian 12 image for headless / CI use.
- Tagged: `cadkernel/headless:vX.Y.Z` + `latest` + `stable`.
- Multi-arch (linux/amd64, linux/arm64).
- SBOM embedded.
- Image signed via cosign.
- Public Docker Hub + GHCR mirror.

**Snap / Flatpak / winget / brew:** community-maintained but officially upstream-blessed.

**Install-size budget:** binary < 80 MB compressed; uncompressed < 200 MB; CJK font pack lazy-downloaded (~3 MB).

### D5 — Documentation & onboarding

**Mission:** a new user reaches first success in 10 minutes; a new contributor builds + tests the workspace in 30 minutes; an integrator embeds via API in 1 hour.

**Deliverables:**

1. **mdBook site** at `docs.cadkernel.dev`. Sections:
   - Quick start (5 pages).
   - User guide (40 pages).
   - API reference (auto-generated from rustdoc).
   - Plugin SDK guide (15 pages).
   - File formats (10 pages).
   - Architecture (15 pages, ADR-linked).
   - Algorithm bibliography (auto-extracted from this roadmap §10.2).
   - Glossary (auto-extracted, 200+ terms).
   - Tutorials (12 step-by-step).
   - Changelog mirror.
   - Contributing guide.
   - License & IP.
2. **Per-public-`Command` doc test** — every `Command` variant has at least 1 doc test demonstrating use; ensures docs and code stay in sync.
3. **12 video tutorials** (cf. B11). 2-15 min each. Hosted on the docs site + YouTube mirror.
4. **AI-integration cookbook** with:
   - 3 reference prompts (build R1, edit R2, export STEP).
   - Expected MCP tool sequences.
   - Cursor / Claude Desktop / Continue.dev configs.
   - Common failure modes and how to recover.
5. **F1 contextual help** wired to mdBook anchors via deeply-linked URLs.
6. **Glossary auto-extracted** from `docs/wiki/Glossary.md`.
7. **ADR (architecture decision records)** under `docs/adr/` — numbered, immutable. Each significant decision (kernel-from-scratch vs OCCT, .cadk format choice, CRDT choice, plugin sandbox choice, etc.) gets one.
8. **User guide PDF** generated from mdBook via `mdbook-pdf` plugin per release.
9. **API reference (rustdoc)** on docs.rs per-crate.
10. **`docs/getting-started/` walkthrough** — 5 pages, no theory, just "open viewer → sketch → pad → save". Updated per release.
11. **Contributor onboarding** — `docs/CONTRIBUTING.md` + `docs/dev/setup.md` covering toolchain, IDE configs (VS Code / RustRover settings), test commands, common pitfalls.
12. **Bilingual policy** — EN canonical, KO summary; CN / JA / DE / FR / ES / RU summaries v3.0+.
13. **Searchable** docs site via `tantivy`-powered local index.
14. **Versioned docs** — docs.cadkernel.dev/v1.0, /v1.1, etc. + /latest alias.

### D6 — Security & supply chain

**Mission:** supply-chain integrity from source to user binary. Zero unaudited dependencies. Defended against malicious files.

**Deliverables:**

1. **SECURITY.md** with public CVE policy; 90-day disclosure window. Coordinated-disclosure preferred. PGP key published for embargoed reports.
2. **`cargo deny`** advisories CI gate; failing builds. Configuration: deny `unmaintained`, `unsound`, `vulnerable`; allow only Apache-2.0 / MIT / BSD-3 / MPL-2.0 / Unicode-DFS-2016 / ISC / Zlib licences.
3. **`cargo vet`** baseline; new deps require human review with rationale documented.
4. **`cargo public-api`** pinned per release. Schema diff auto-generated; major SemVer bump = breaking-change RFC required.
5. **Wasm-sandboxed plugin loader** (`wasmtime 22+`); native plugins behind a setting + scary dialog + signed-manifest requirement.
6. **Capability tokens** for sandbox: `ReadFile { glob }` / `WriteFile { glob }` / `Network { allowlist }` / `GPU { quota_ms }`.
7. **Telemetry privacy review:** explicit DPA published; GDPR retention 90 days; no model contents ever uploaded; differential privacy (ε=1.0) on aggregates.
8. **File-format fuzzing** in CI on every PR. Continuous fuzzing on a dedicated fleet (24 h / day). Crashes opened as security issues.
9. **Dependency audit per release**: Apache-2.0 / MIT / BSD-3 / MPL only; explicit GPL avoidance (we cannot link GPL into a permissively-licensed binary).
10. **SBOM generation per release** (CycloneDX 1.5 format). Published alongside binary. Includes transitive dep tree, licences, hashes.
11. **SLSA Level 3** build provenance: every binary has signed attestation of source repo + commit + builder + tooling hashes.
12. **Sigstore cosign** for binary signatures; transparent log via Rekor.
13. **Reproducible build** verification: third-party can rebuild from source and verify byte-identical to released binary.
14. **Memory safety audit** of all `unsafe` blocks; isolated to `-math`, `-viewer`; documented invariants per block.
15. **Threat model** documented; reviewed annually. Surface: file parsing (untrusted CAD files), plugin loading, MCP server, cloud sync (Track F).
16. **Penetration testing** annually for cloud / MCP server (Track F + A6 surfaces).
17. **Bug bounty** programme post-v1.0; HackerOne or self-hosted; bounty range $100-$10,000 per severity tier.
18. **CVE process** — numbered CVEs for confirmed vulnerabilities; published advisories on docs site + GitHub Security Advisories.

### D7 — Release engineering

**Mission:** predictable cadence; SemVer guarantees honoured; users can plan upgrades.

**Cadence:**

- **Stable**: quarterly (v0.5 Q3 2026, v1.0 Q3 2027, v1.x quarterly thereafter).
- **Patch**: monthly as needed for critical bugs.
- **Beta**: weekly tag from main.
- **Nightly**: continuous from main on every merge.
- **LTS**: every other major (v1.0 LTS, v3.0 LTS); 2-year support window with security fixes only.

**SemVer enforced for `cadkernel-api` + `cadkernel-mcp`.**

- Major (X.0.0): breaking changes allowed. RFC required.
- Minor (X.Y.0): new commands / new optional fields / new outcomes.
- Patch (X.Y.Z): bug fixes only. No API surface change.
- `cargo public-api` enforced in CI.

**Migration:**

- Migration guide auto-generated per major from `Command` schema diffs (`crates/api/src/migrate.rs`).
- Compatibility shim crate `cadkernel-compat` translates older command schemas forward (one-major behind).
- File-format migration: `.cadk` v0 → v1 → v2 chain preserved indefinitely.

**Communication:**

- Public roadmap board (GitHub Projects driven by this doc).
- Quarterly release notes: changelog + migration guide + known issues.
- Release blog post per major: highlights + benchmarks + acknowledgements.
- RSS / Atom feed for releases.
- Discord / Matrix announce.

**Issue triage SLA:**

- Bug + reproducer ack ≤ 5 working days.
- Critical (data-loss, crash on common path) ≤ 24 h.
- Security (per SECURITY.md) ≤ 24 h ack, 90 d patch.
- Feature requests: triaged into roadmap quarterly review.

**Triage labels:** P0 critical / P1 high / P2 normal / P3 low; bug / feature / docs / question / discussion.

**Release checklist (per release):**

- All CI green on stable branch.
- Reference parts R1-R12 build clean.
- Industry compliance suites pass at gate threshold.
- Migration tests pass (n-1 → n).
- CHANGELOG, DEVELOPER_WIKI bilingual updated.
- SBOM generated.
- Build attested.
- Binaries signed.
- Crates published in dep order.
- Tag pushed.
- Docs site updated.
- Release notes published.

### Track D summary

(All phases run continuously from day one.)

---

## 8. Track E — Ecosystem (new)

**Mission:** plugins, education, marketplace.

### E1 — Plugin SDK v2

**Mission:** the plugin surface is the leverage that takes CADKernel from "a CAD tool" to "a CAD platform."

(Replaces existing `crates/modeling/src/plugin.rs` with a richer surface.)

**Architecture:**

```text
   +--------------------+
   | host (cadkernel)   |
   |   PluginRegistry   |
   +---------+----------+
             |
     +-------+----------+
     |                  |
+----v-----+      +-----v-----+
| native   |      |  wasm     |
| plugin   |      |  plugin   |
| (signed) |      | (sandbox) |
+----+-----+      +-----+-----+
     |                  |
     | unrestricted     | capability-tokens only
     |  kernel access   |  (file/net/gpu/clock)
```

**Deliverables:**

1. **Wasm host** — `wasmtime`-based runtime; capability tokens (file IO, network IO, GPU, clock, RNG).
2. **Native host** — for plugins requiring direct kernel access; gated by signed manifest + scary dialog + first-run consent + path-allow-list.
3. **Plugin manifest** — JSON / TOML with name, version, semver-range deps, permissions, entry points, contributed commands, contributed UI panels, contributed file formats.
4. **Lifecycle hooks** — on_init / on_shutdown / on_document_open / on_document_close / on_command_executed / on_recompute_complete.
5. **Plugin API surface** — re-exports `cadkernel-api` types + `PluginRegistry::register_command(spec)` + `PluginRegistry::register_panel(spec)` + `PluginRegistry::register_file_format(spec)` + `PluginRegistry::register_post_processor(spec)` (CAM).
6. **Permission model** — each plugin declares required capabilities at install time; user reviews; revocable via Settings.
7. **Signed plugin** workflow — plugins published to E2 marketplace are signed by the marketplace key + author key; verified at install.
8. **Plugin SDK package** — `cadkernel-sdk` crate (Rust) + `cadkernel-sdk-py` (Python); both wrap `cadkernel-api`.
9. **CLI tooling** — `cadkernel-plugin new`, `cadkernel-plugin build`, `cadkernel-plugin publish`, `cadkernel-plugin test`.
10. **Examples** — 5 reference plugins:
    - **Screw library** — inserts ISO / DIN / ANSI / JIS standard fasteners by part number.
    - **STEP-CAD-A1 importer** — a hypothetical 3rd-party importer demonstrating file-format hooks.
    - **Custom-toolpath generator** — demonstrates CAM plugin hook.
    - **BOM-export-to-Excel** — demonstrates file-format export hook.
    - **GD&T linter** — demonstrates document-validation hook.
11. **Plugin debugger** — `cadkernel-plugin attach <pid>` for native; in-app console for wasm.
12. **Plugin telemetry** (opt-in) — plugin authors get aggregate usage stats (no document content).

**Acceptance:** 5 reference plugins pass full lifecycle; a malicious wasm plugin attempting file system or network beyond capability fails; native plugin with invalid signature refuses to load; plugin can register a command, a panel, and a file format and they appear in UI without restart.

### E2 — Plugin marketplace

**Mission:** discoverable, trusted, monetisable plugin ecosystem.

**Deliverables:**

1. **Public registry** at `plugins.cadkernel.dev`. Browseable, searchable, filterable by category / workbench / rating / popularity.
2. **Submission workflow:**
   - Upload signed plugin manifest + binary / wasm + screenshots + description (markdown).
   - Automated security scan (`cargo deny`-equivalent + virus scan + capability audit).
   - Manual review for native plugins (E1 §10); 5-day SLA.
   - Wasm plugins with capability declarations ≤ "safe" tier auto-approved.
3. **Per-plugin metadata**: rating (1-5 stars), review count, install count, last updated, supported CADKernel versions, screenshots, video preview, changelog.
4. **Categories** at v2.0: Standards Libraries (fasteners, profiles, materials), Importers / Exporters, Toolpath Generators, Renderers, Sketchers (extra constraint types), Validators, Reporters, BOM Exporters, Templates, Themes, Languages.
5. **Monetisation** (post v2.0): paid plugins; revenue share 80 / 20 (author / platform); Stripe integration.
6. **Free / open-source** preferred; clear free / paid filter.
7. **CI from main repo:** official plugins (the 5 reference plugins from E1) built and published from monorepo; serve as quality bar.
8. **In-app marketplace browser** — ribbon button "Get Plugins" opens marketplace UI inside the app; install / update / uninstall directly.
9. **Update notifications** — in-app prompt when installed plugins have updates; one-click update.
10. **Telemetry** (aggregate) — plugin authors see install count + rating; users see popularity.
11. **Reporting** — users can flag malicious / broken plugins; rapid-takedown for confirmed reports.

### E3 — Education & tutorials

**Mission:** academic / vocational / hobbyist adoption pipeline.

**Deliverables:**

1. **Tutorial library** (cf. B11) versioned with the product. 12 at v1.0; 30 at v3.0.
2. **School / university programme**:
   - Free academic licence (perpetual, non-commercial).
   - Classroom kit: lesson plans (12), assignments (30), grading rubrics, sample solutions, instructor handbook.
   - Cloud demo environment for classrooms with no install rights.
3. **Certifications**: "CADKernel Certified Associate" (entry-level, free exam) and "CADKernel Certified Professional" (advanced, paid exam $50). Online proctored.
4. **Community gallery**: user-submitted reference parts; vetted; surfaced in `File > Examples`. Categories: mechanical, sheet metal, weldments, mold, freeform.
5. **Reference textbook** — "CADKernel from First Principles" (open-source book, MIT licence, hosted at docs.cadkernel.dev/book). Targets: undergraduates, hobbyists, mechanical engineers retraining.
6. **YouTube channel** — tutorials, build-along videos, release notes, community spotlights.
7. **Discord / Matrix community** — official help channels staffed by maintainers + community.
8. **CADKernel Day** — annual virtual conference; lightning talks; user showcases; planning Q&A.
9. **Bug bounty for tutorials** — community contributors can submit / improve tutorials; reward in plugin marketplace credits.

### E4 — API stability commitment

**Mission:** integrators can plan around our API.

**Deliverables:**

1. **SemVer for `cadkernel-api` and `cadkernel-mcp`.** Major = breaking; minor = additive; patch = bug-only.
2. **Deprecation policy:** announce one minor in advance, remove next major.
3. **Compatibility shim crate** `cadkernel-compat` translates older command schemas forward (one-major behind always supported).
4. **Breaking-change RFC process** — public RFC repo `cadkernel/rfcs`; RFCs require:
   - Motivation and use cases.
   - Detailed design.
   - Migration plan.
   - Drawbacks.
   - Alternatives considered.
   - Public comment period ≥ 14 days.
   - Approval by 2 of 5 maintainers.
5. **Stability guarantee** by tier:
   - **Tier 1 (`cadkernel-api`, `cadkernel-mcp`)**: SemVer-strict.
   - **Tier 2 (`cadkernel-modeling`, `cadkernel-sketch`, `cadkernel-io`)**: Major SemVer; minor allows internal refactor as long as `-api` re-exports preserved.
   - **Tier 3 (`cadkernel-math`, `cadkernel-geometry`, `cadkernel-topology`)**: best-effort SemVer; treated as internal.
   - **Tier 4 (viewer internals)**: no stability guarantee.
6. **Documented stability** — each crate's README states its tier.
7. **`cargo public-api`** baseline pinned per release; CI gate.
8. **Long-term API versioning** — commit to API v1 for 5 years (no v2 until 2031).

### Track E summary

| Phase | v-target |
|---|---|
| E1 | v1.0 (SDK), v2.0 (sandbox polished) |
| E2 | v2.0 |
| E3 | v2.0 |
| E4 | v1.0 |

---

## 9. Track F — Collaboration & Cloud (new)

**Mission:** Onshape-class real-time collaboration without lock-in. Cloud-optional, local-first.

### F1 — CRDT for Document

**Goal:** two users edit the same document simultaneously; changes merge without conflicts; offline-first; eventually-consistent without a central authority.

**Approach:** Document state expressed as **operation log** (already true via `Session::log` from A2); ops are CRDT-merge-friendly via `automerge 0.5+`.

**Architecture:**

```text
   actor A          actor B          actor C
      |                |                |
   command          command          command
      |                |                |
   Session::execute (local, optimistic)
      |                |                |
   op {actor, lamport, payload}
      |                |                |
      +-------+--------+--------+-------+
              |                 |
         CRDT engine (automerge wrapper)
              |                 |
        merge order causal      |
              |                 |
   eventually-consistent Document state
```

**Deliverables:**

1. **`cadkernel-crdt`** new crate. Wraps `automerge 0.5+` (decision per IP audit §10.4).
2. **Op-log CRDT** — every `Command` is an op; ops have `(actor_id, lamport_timestamp, parent_hash)` for causal ordering. `actor_id` is a 16-byte UUID per session. `lamport` is a u64 monotonic per actor.
3. **Causal ordering** — ops apply only after all causal predecessors. Out-of-order arrivals are buffered.
4. **Conflict resolution policy** by op type:
    - **Parameter edit (numeric)** — last-write-wins (lamport tiebreaker by actor_id). Loser's value is preserved as comment annotation for review.
    - **Feature add** — commutative; both apply.
    - **Feature delete** — wins over edit-of-deleted (tombstone).
    - **Feature reorder** — last-write-wins on order vector; warn on user when reorder competed.
    - **Topology-changing structural** (e.g., shared sketch redefine) — marked as conflict; "branch + manual merge" UI invoked.
5. **Snapshot compression** — at every N ops (default 1000) or on save, compact log to a snapshot. Snapshot is the canonical Document; ops since snapshot are the delta. Snapshots are merge-friendly via Automerge document hashes.
6. **Garbage collection** — ops older than the latest agreed snapshot across all known actors are pruned. Detection via gossip-style snapshot hash exchange.
7. **Local-first** — works offline; on reconnect, ops since last sync flush to peers / server.
8. **Sync protocol** — binary diff protocol over WebSocket; supports server-mediated and peer-to-peer modes. Falls back to delta-via-HTTPS for restrictive networks.
9. **Validation** — every applied remote op is followed by document-validation (manifold check, sketch-solver check). Validation failure rolls back the op and reports a conflict to user.
10. **Determinism** — given same op set, all actors converge to byte-identical Document snapshots (canonical-hash assertion).
11. **Tests** — random-op concurrent fuzz: 3 actors × 100 ops × random order, all converge identically; 10 k iterations no divergence.
12. **Acceptance gates** —
    - 3 actors edit a 50-feature R3-class part concurrently for 5 minutes; all 3 see identical Document; no validation errors.
    - Network partition: 2 actors offline 1 hour, edit, reconnect, merge succeeds without manual intervention for 90 % of test scenarios.
    - Adversarial: malformed remote op rejected without corrupting local state.

### F2 — Cloud document storage

**Mission:** Onshape-class document hosting without lock-in. Self-hostable. S3-compatible.

**Architecture:**

```text
   client (cadkernel desktop / web)
      |
      | HTTPS / WebSocket
      |
   cadkernel-cloud server (open-source, self-hostable)
      |
      +--- auth (OAuth2 + OIDC)
      |
      +--- ACL engine (per-document, per-org)
      |
      +--- CRDT op-log relay (F1)
      |
      +--- snapshot store (S3-compat: MinIO / AWS / R2 / B2 / Wasabi)
      |
      +--- revision/branch/diff/merge engine (git-like)
      |
      +--- audit log (PDM, F4)
```

**Deliverables:**

1. **`cadkernel-cloud`** server crate. Open-source under Apache-2.0. Self-hostable single binary.
2. **Backend**: S3-compatible object storage (MinIO / AWS S3 / Cloudflare R2 / Backblaze B2 / Wasabi). Configured by env vars + `cadkernel-cloud.toml`.
3. **Auth**: OAuth 2.0 + OpenID Connect. Pluggable: GitHub / Google / Microsoft / self-hosted Keycloak / Authelia.
4. **ACL** on per-document basis: owner / editor / commenter / viewer. Org-scoped permissions: org-admin / member / guest.
5. **Revisions** as named snapshots. Revisions immutable. Tagged with semantic versions (v1.0, v1.1-rc1, etc.).
6. **Branches** — named pointers to revision DAG. Default branch = `main`. Per-user feature branches.
7. **Diff** — between two revisions; diff at command-log level (added / removed / modified ops); UI shows feature-level changes.
8. **Merge** — git-style three-way merge using F1 CRDT. Conflict resolution UI for non-mergeable structural changes.
9. **Self-host docs** — docker-compose example, k8s helm chart, single-binary mode, backup / restore procedures.
10. **Cloud SaaS option** — official cloud at `cloud.cadkernel.dev` (free tier 1 GB / 3 docs; paid tiers).
11. **Data export** — user can download full data as `.cadk` archive any time. No vendor lock-in.
12. **GDPR-compliant** — explicit DPA; data residency selection (EU / US / APAC); deletion within 30 days of request.
13. **Audit log** — every doc operation logged with actor + timestamp + IP; queryable by org admins.
14. **Acceptance** — self-host setup in < 30 min following docs; 3-user concurrent doc edit succeeds end-to-end; round-trip export to `.cadk` lossless.

### F3 — Real-time collaboration UX

**Mission:** the experience of co-editing feels live, never blocked, conflicts surfaced gracefully (cf. B19 for full UX spec).

**Deliverables:**

1. **Live cursors / selections / camera positions** — every active actor's pointer + selection + camera frustum rendered for others.
2. **Per-user colour assignment** — deterministic from actor UUID; consistent across sessions.
3. **Voice chat** (WebRTC) optional — per-document voice room. SFU server architecture (mediasoup or LiveKit). End-to-end encrypted option.
4. **Presence indicator** in document tab — "3 users active" with avatars.
5. **Suggestion mode** (read-only + comment) — reviewers can comment without editing; comments anchored to features / faces / drawing views.
6. **Activity feed** — sidebar showing recent edits; click to jump to that view.
7. **@mentions** in comments — notify mentioned users via in-app + email.
8. **Following mode** — follow another user's view; auto-camera-sync.
9. **Concurrent edit warnings** — if two users select same feature simultaneously, second user sees a soft lock indicator.
10. **Conflict UI** — when CRDT conflict requires manual resolution, dedicated panel with side-by-side diff and resolve buttons.
11. **Offline indicator** — clear state when disconnected; queued ops preserved; auto-replay on reconnect.
12. **Session recording** — optional record-and-replay of an editing session for tutorials / debugging.
13. **Acceptance** — 5 users co-editing R3-class part: cursors visible; voice clear; no perceptible lag (< 200 ms RTT to local rendering).

### F4 — PDM workflows

**Mission:** product data management for engineering teams; revision control, change orders, where-used graphs.

**Deliverables:**

1. **Revision management** — documents have lifecycle states: `Draft` → `In Review` → `Approved` → `Released` → `Obsolete`. Each transition logged.
2. **ECO (Engineering Change Order) workflow** — propose change, attach justification, route through approvers, implement, verify, release.
3. **Where-used graph** — for any document / part, show the assemblies / drawings / BOMs referencing it. Indexed; updated incrementally as references change.
4. **Reference checking** — broken references (deleted parts, missing files) flagged; outdated references (referenced doc has newer revision) prompted.
5. **Approval routing** — configurable workflow: serial / parallel / quorum approvers. Email + in-app notifications.
6. **Audit trail** — every state transition + every approver action logged immutably; queryable for ISO 9001 / FDA 21 CFR Part 11 compliance.
7. **Roles** — designer / reviewer / approver / admin; per-org configurable.
8. **Locking** (optional) — strict workflow can require document checkout for edit; default is CRDT-based concurrent.
9. **Integration** — export PLM-compatible XML (PLMXML) for Teamcenter / Windchill / 3DEXPERIENCE round-trip.
10. **BOM management** — multi-level BOM views; BOM diff between revisions; export to ERP CSV / Excel.
11. **Acceptance** — a 3-revision lifecycle (Draft → Approved → Released) succeeds end-to-end with 2 approvers; audit log complete; broken-reference detection 100 %.

### Track F summary

| Phase | v-target |
|---|---|
| F1 | v2.0 |
| F2 | v2.0 |
| F3 | v2.0 |
| F4 | v3.0 |

---

## 10. Cross-cutting concerns

### 10.1 The single-funnel contract

```
GUI click  ─┐
Lua script ─┼──> Command (typed, JSON-schema'd)
AI tool    ─┤            │
Test       ─┤            ▼
CRDT op    ─┘   Session::execute (deterministic)
                         │
                         ▼
              Document mutated + history appended
                         │
                         ▼
                  Outcome returned
                         │
                         ▼
              Viewer redraws via dirty propagation
```

Every track honours this. A change that bypasses the funnel is rejected at code review.

### 10.2 Algorithm bibliography (canonical references)

Curves & surfaces:
- Piegl & Tiller, _The NURBS Book_ (2nd ed., Springer 1997). Primary reference.
- Farin, _Curves and Surfaces for CAGD_ (5th ed., 2002).
- Sederberg, _Computer Aided Geometric Design_ (BYU lecture notes, 2012).

Solid modeling:
- Hoffmann, _Geometric and Solid Modeling: An Introduction_ (1989).
- Mäntylä, _An Introduction to Solid Modeling_ (1988).
- Requicha & Voelcker, _Boolean Operations in Solid Modeling_ (1985).

Computational geometry:
- de Berg et al., _Computational Geometry: Algorithms and Applications_ (3rd ed.).
- Schneider & Eberly, _Geometric Tools for Computer Graphics_.

Numerical methods:
- Nocedal & Wright, _Numerical Optimization_ (2nd ed.).
- Higham, _Accuracy and Stability of Numerical Algorithms_ (2nd ed.).

FEM:
- Bathe, _Finite Element Procedures_.
- Zienkiewicz & Taylor, _The Finite Element Method_ (7th ed.).
- Belytschko et al., _Nonlinear Finite Elements_.

Rendering:
- Pharr, Jakob & Humphreys, _PBR Book_ (4th ed., online).
- Akenine-Möller et al., _Real-Time Rendering_ (4th ed.).

Constraint solving:
- Bouma-Fudos-Hoffmann-Cai-Paige (1995), graph-based geometric constraint solving.
- Hoffmann-Joan-Arinyo, "Symbolic constraints in constructive geometric constraint solving."

CAM:
- Choi & Jerard, _Sculptured Surface Machining_.
- Held, _On the Computational Geometry of Pocket Machining_.

CRDT:
- Shapiro et al., "Conflict-free Replicated Data Types" (INRIA, 2011).
- Kleppmann, "A Conflict-Free Replicated JSON Datatype" (2017).

### 10.3 Kernel architecture decisions (vs commercial kernels)

| Topic | Parasolid | ACIS | OCCT | Our choice |
|---|---|---|---|---|
| Topology | Half-edge | Half-edge | TopoDS_Shape (boundary) | Half-edge (existing) |
| Geometry | NURBS + analytical | NURBS + analytical | NURBS + analytical | NURBS + analytical |
| Persistent naming | Tag history | Annotated entity | TNaming | `Tag` + `OperationId` (existing) |
| Boolean engine | Tolerant | Tolerant | Mixed precision | Exact-arithmetic core + tolerant fallback |
| Floating-point precision | f64 | f64 | f64 | f64 |
| Multithreading | Internal locks | Internal locks | Limited | `Send + Sync` arena (existing) |

**Decision (locked, 2026-05-06):** stay with our existing half-edge + NURBS kernel. Do **not** integrate OCCT / Parasolid / ACIS as a runtime dependency — they are GPL / commercial / Apache-incompatible. We accept the multi-year cost of growing our own kernel; this is the project's reason for existing.

### 10.4 Comparative IP audit — third-party libraries

**Policy:** every dependency declared with: licence (must be Apache-2.0-compatible), why we need it, fallback if removed, and per-release `cargo vet` review.

| Library | Use | Licence | Compatible | Why we need it | Fallback if removed | Decision |
|---|---|---|---|---|---|---|
| `nalgebra 0.33` | linear algebra | Apache / BSD | yes | full-feature dense + sparse, well-tested | hand-roll for `-math` | adopted |
| `glam 0.29` | small math (Vec3/4, Mat4, Quat) | MIT / Apache | yes | SIMD-optimised hot path | nalgebra | adopted |
| `wgpu 24` | GPU abstraction | MIT / Apache | yes | cross-platform Vulkan / Metal / DX12 / WebGPU | direct Vulkan/Metal (months of work) | adopted |
| `egui 0.31` | UI immediate-mode | MIT / Apache | yes | rust-native, retained-mode-ish, accessibility-friendly | iced / dioxus | adopted |
| `winit 0.30` | windowing + input | Apache | yes | de-facto standard | hand-roll per-OS | adopted |
| `rayon 1` | parallelism | MIT / Apache | yes | data-parallel iter; tessellation, BVH | tokio + manual | adopted |
| `serde / serde_json / bincode` | serialization | MIT / Apache | yes | universal in Rust ecosystem | hand-roll | adopted |
| `mlua 0.10` | Lua 5.4 binding | MIT | yes | scripting (existing) | rune / rhai | adopted |
| `wasmtime 22+` | Wasm host | Apache | yes | E1 plugin sandbox | wasmer (similar licence) | adopted |
| `automerge 0.5+` | CRDT | MIT / Apache | yes | F1 collab; mature, widely used | yrs (Yjs port) | candidate, audit pending |
| `tokio 1.x` | async runtime | MIT | yes | F2 cloud server, F3 collab WS | smol / async-std | candidate (F-only) |
| `axum 0.7` | HTTP framework | MIT | yes | F2 cloud REST | actix-web | candidate (F-only) |
| `criterion 0.5` | benchmarking | Apache / MIT | yes | D1 perf gate (existing) | hand-roll | adopted |
| `proptest 1.x` | property tests | MIT / Apache | yes | D2 robustness (existing) | quickcheck | adopted |
| `cargo-fuzz` | fuzzing | MIT / Apache | yes | D2 fuzz (existing) | honggfuzz | adopted |
| `cargo-mutants` | mutation testing | MIT / Apache | yes | D3 quality gate | mutagen (deprecated) | adopted |
| `pixelmatch-rs` | visual regression | ISC | yes | D3 visual gate (existing) | hand-roll | adopted |
| `image 0.25` | image IO | MIT / Apache | yes | viewer screenshots, render output | hand-roll PNG | adopted |
| `naga 24` | shader compilation | MIT / Apache | yes | wgpu peer | spirv-cross | adopted (transitive) |
| `ash` (optional) | Vulkan raw | MIT / Apache | yes | low-level GPU debug | wgpu only | optional |
| `metal-rs` (optional) | Metal raw | MIT / Apache | yes | macOS-specific perf paths | wgpu only | optional |
| `dhat-rs` | heap profiling | MIT / Apache | yes | D2 memory soak | valgrind massif | dev-only |
| `tracing` | structured logging | MIT | yes | viewer + cloud diagnostics | log + env_logger | adopted |
| `cgal` | computational geom | GPL | **no** | reference only | n/a | **rejected (GPL)** |
| `OCCT` | CAD kernel | LGPL 2.1 | conditional | reference only | n/a | **rejected (linkage)** |
| `Parasolid` | CAD kernel | commercial | no | reference only | n/a | **rejected (cost + licence)** |
| `ACIS` | CAD kernel | commercial | no | reference only | n/a | **rejected (cost + licence)** |
| `Eigen` (via `eigen-sys`) | linear algebra (FEM) | MPL2 | yes (with care) | C-Sim FEM solver candidate | nalgebra-sparse | **candidate** for v2 FEM |
| `SuiteSparse` (via sys) | sparse direct solver | LGPL | conditional | C-Sim FEM | iterative solver only | candidate (LGPL dynamic-link OK) |
| `OpenVDB` | volumetric | MPL2 | yes | C-Sim plastics | hand-roll grid | candidate v3 |
| `Featherstone-rs` | rigid-body dynamics | MIT | yes | C-Mech v2 | hand-roll Featherstone Ch.4 | candidate |
| `nalgebra-sparse` | sparse | Apache | yes | C-Sim baseline FEM | hand-roll | adopted |

Final IP audit gate: D6 per release; new dependency requires `cargo vet` review + this table updated.

### 10.5 Industry compliance benchmark suite

| Benchmark | What it tests | v-gate |
|---|---|---|
| NIST CAX-IF (STEP) | AP203/214/242 conformance | v1.0 |
| ASME Y14.5 GD&T | Drawing tolerancing semantics | v1.0 |
| ISO 10303-242 PMI | 3D-annotated models round-trip | v2.0 |
| JT B-Rep Annex | Visualisation interop | v2.0 |
| LinuxCNC G-code | Post-processor sanity | v2.0 |
| ProtoTRAK G-code | Smaller-shop CAM | v2.0 |
| Onshape doc compatibility | Import / export with Onshape | aspirational |
| 3MF Consortium | Additive manufacturing | v1.0 |
| Khronos glTF 2.0 | Web visualisation | v0.5 (existing) |

### 10.6 Performance benchmark vs commercial

Measured on D1 reference HW (Linux ThinkPad X1 Carbon Gen 9 unless noted). Commercial timings sourced from public benchmarks + in-house lab measurements (Dell Precision 7780 / Xeon W / RTX A4000) normalised to the X1 budget by 0.7× factor.

| Benchmark | SW 2026 | Inventor 2026 | Fusion 360 (cloud) | FreeCAD 1.0 | us @ v1.0 | us @ v2.0 | us @ v3.0 |
|---|---|---|---|---|---|---|---|
| Cold start → ready | 4 s | 5 s | 6 s | 3 s | < 800 ms | < 600 ms | < 500 ms |
| R1 (10-feat box) open | 0.2 s | 0.3 s | 0.5 s | 0.4 s | < 200 ms | < 150 ms | < 100 ms |
| R2 (50-feat sketch) open | 0.8 s | 0.7 s | 1.2 s | 1.5 s | < 1 s | < 700 ms | < 500 ms |
| R2 dim recompute | 50 ms | 80 ms | 100 ms | 400 ms | < 200 ms | < 100 ms | < 50 ms |
| R3 (50 × 3 features) load | 2.5 s | 2.1 s | 3.2 s | 5 s | < 3 s | < 2 s | < 1.5 s |
| R3 incremental recompute | 200 ms | 250 ms | 400 ms | 1 s | < 500 ms | < 300 ms | < 200 ms |
| R4 (sheet-metal flatten) | 800 ms | 700 ms | 1 s | n/a | < 1.5 s | < 1 s | < 700 ms |
| R5 (weldment cut-list) | 1.5 s | 1.2 s | 2 s | 3 s | < 2.5 s | < 1.5 s | < 1 s |
| R6 (mold parting line) | 5 s | 4 s | 8 s | n/a | n/a | < 8 s | < 5 s |
| R7 (injection-molded plastic) | 2 s | 2 s | 3 s | n/a | n/a | < 3 s | < 2 s |
| R8 (CAM 3-axis toolpath) | 3 s | 3 s | 5 s | n/a | n/a | < 5 s | < 3 s |
| R9 (5-axis toolpath) | 8 s | 7 s | 15 s | n/a | n/a | n/a | < 10 s |
| R10 (generative bracket) | 30 s | 40 s | 60 s | n/a | n/a | n/a | < 60 s |
| R11 (250-component asm) open | 7 s | 6 s | 12 s | 20 s | < 10 s | < 7 s | < 5 s |
| R11 mate solve | 800 ms | 1 s | 1.5 s | 4 s | < 2 s | < 1 s | < 800 ms |
| R12 (10k-feature monolithic) open | 30 s | 25 s | 60 s | 120 s | < 30 s lazy | < 20 s lazy | < 15 s lazy |
| Sketch drag 50 constraints | 60 fps | 60 fps | 60 fps | 30 fps | 60 fps | 60 fps | 120 fps |
| Sketch drag 200 constraints | 50 fps | 60 fps | 45 fps | 8 fps | 30 fps | 60 fps | 60 fps |
| 30 fps R3 view | yes | yes | yes (LoD) | no | yes | yes | yes |
| 60 fps R11 view (LoD) | yes | yes | yes | partial | yes | yes | yes |
| Boolean union (R1 × R1) | 5 ms | 8 ms | 12 ms | 30 ms | < 20 ms | < 10 ms | < 5 ms |
| STEP AP242 export R3 | 1 s | 1.2 s | 2 s | 3 s | < 2 s | < 1.5 s | < 1 s |
| STEP AP242 import R3 | 2 s | 2.5 s | 3.5 s | 5 s | < 3 s | < 2 s | < 1.5 s |
| Ray-traced R3 render (1 spp / pixel) | 100 ms | 120 ms | 200 ms (cloud) | n/a | n/a | < 300 ms | < 100 ms |
| RAM working set R3 | 800 MB | 700 MB | 1.5 GB | 600 MB | < 2 GB | < 1.5 GB | < 1 GB |
| RAM working set R12 | 4 GB | 3.5 GB | 6 GB | 5 GB | < 4 GB lazy | < 3 GB lazy | < 2 GB lazy |
| Disk size R3 saved | 2 MB | 3 MB | 5 MB (cloud) | 4 MB | < 3 MB | < 2 MB | < 1.5 MB |
| Undo 100 deep | 50 ms | 80 ms | 150 ms | 500 ms | < 200 ms | < 100 ms | < 50 ms |

**Gate policy:** every PR runs the v1.0 column on CI; > 10 % regression blocks merge. v2.0 / v3.0 columns gate at the relevant release.

### 10.7 Coding conventions (recap from CLAUDE.md)

- Public APIs return `Result`; no panics.
- Geometry `f64`; trait objects `Send + Sync`.
- Persistent naming via `Tag` + `OperationId`.
- Bilingual docs: English canonical, Korean summary.
- Zero `unwrap()` / `panic!()` in shipped code paths (clippy gates).
- No AI attribution in commits / PRs / comments / release notes.

### 10.8 Team allocation (per CLAUDE.md §11)

| Track | Primary teams |
|---|---|
| A | kernel-engineer (lead), io-engineer (file format + STEP) |
| B | ui-engineer (lead) |
| C-Solid / C-Sketch / C-Surface / C-Mech / C-Sim | kernel-engineer (lead), ui-engineer (consumer) |
| C-Sheet / C-Weld / C-Mold | kernel-engineer (lead), ui-engineer |
| C-Draw / C-CAM | io-engineer (lead) for I/O, ui-engineer for editing UX |
| C-Render | ui-engineer (lead), kernel-engineer (BVH integration) |
| C-Reverse / C-Subdiv | kernel-engineer (lead) |
| D | qa-engineer (lead), all teams contribute |
| E | qa-engineer + io-engineer (SDK), tech-lead (governance) |
| F | io-engineer (lead) for protocol, ui-engineer (presence) |

Cross-track changes (signal/event modifications, Command enum extensions, Document schema bumps) **always** route through tech-lead.

### 10.9 Risk register (expanded)

| # | Risk | P | I | Mitigation |
|---|---|---|---|---|
| 1 | `.cadk` schema lock-in too early | M | H | All Document fields `Option<T>` for one minor before required |
| 2 | macOS Metal GPU picking degrades | L | M | CPU fallback retained behind setting |
| 3 | Industrial sketches fail solver | M | H | C-Sketch deflation + 10k-corpus proptest early |
| 4 | Recompute cache invalidation bugs | H | C | Hash all inputs; R1-R12 hash baseline gated per PR |
| 5 | MCP rewrite leaks dep cycle | L | H | Extract to new crate (A6); legacy under `cfg` 1 minor |
| 6 | Visual regression GPU flake | H | M | Single canonical CI runner image; 0.1 % tolerance band |
| 7 | Localisation drifts from canonical | M | L | CI fails if `.po` missing strings |
| 8 | Auto-update server compromised | L | C | Signed updates + revocation list shipped per release |
| 9 | STEP libraries diverge from AP242 | M | H | Audit + golden-file corpus; fast-fail |
| 10 | Performance budget drift | H | M | Per-PR bench gate; budget = CI failure not warning |
| 11 | Boolean engine fails on degenerate inputs | M | C | Exact-arithmetic core; 1M fuzz before release |
| 12 | NURBS surface intersection diverges | M | H | Bracketed Newton + The NURBS Book §6.6; corpus tests |
| 13 | Plugin marketplace hosts malware | M | C | Mandatory Wasm sandbox + signature + manual review for native |
| 14 | CRDT merge produces invalid topology | M | C | Validation in `Session::apply_remote_op`; rollback on validation fail |
| 15 | Patent suit on direct edit / parametric | L | C | Patent landscape audit before C-Solid direct-edit ships |
| 16 | Apple notarisation revoked | L | H | Backup distribution channel (curl-installer); cert rotation |
| 17 | wgpu major-version break | M | M | Version-pinned with major-bump RFC |
| 18 | Rust edition transition (2024→2027) | L | L | Edition-bump policy: track Rust LTS |
| 19 | Korean / English docs drift | M | L | English canonical; CI checks for missing entries |
| 20 | Reference-part regression after refactor | H | H | R1-R12 build scripts gated on every merge |

### 10.10 Definitions of done

- **Phase done** — exit gate met, tests green, CHANGELOG updated, WIKI updated, WORK_STATUS updated.
- **Sub-track done** — all phases done; reference parts pass; comparative parity matrix row meets target percentile.
- **Track done** — every sub-track done; cross-track signals settled.
- **v0.5 done** — 12 v0.5 gates met.
- **v1.0 done** — 12 + 24 = 36 gates met; 3 consecutive stable releases without rollback.
- **v2.0 done** — 36 + 16 = 52 gates met.
- **v3.0 done** — all 60 gates met; comparative parity matrix v3 column closed.

---

## 11. Release calendar (concrete)

| Version | Target date | Headline | Gates |
|---|---|---|---|
| **v0.5 "Honest Beta"** | Q3 2026 | API + selection + property + sketcher v2 partial | 1-12 |
| **v0.6** | Q4 2026 | Multi-doc + autosave + i18n ko-en | + 13-15, 24-25 |
| **v0.7** | Q4 2026 | History tree + STEP AP214 round-trip + theme | + 16, 23, 26 |
| **v0.8** | Q1 2027 | Sketcher v3 solver + assembly mates + drawings | + 28, 31, 32 |
| **v1.0 "FreeCAD Competitor"** | Q1 2027 | All v1.0 gates | 1-36 |
| **v1.1-1.5** | 2027 | Bug fixes; perf; community plugins | hardening |
| **v2.0 "Onshape-class"** | Q4 2027 | Sheet metal + weldments + nonlinear FEM + CAM 2.5 + CRDT cloud | 37-52 |
| **v2.1-2.4** | 2028 | More CAM postprocessors; surface polish; T-spline | hardening |
| **v3.0 "SW/Inventor parity"** | Q3 2028 | Mold + 5-axis CAM + plastics + path-traced render + PDM | 53-60 |

CI nightlies tagged daily; betas tagged weekly; stables on the dates above.

---

## 12. Glossary (selected)

| Term | Meaning |
|---|---|
| AP214 / AP242 | STEP application protocols for mechanical design |
| BRep | Boundary representation (faces / edges / vertices) |
| CAD | Computer-aided design |
| CAM | Computer-aided manufacturing |
| CRDT | Conflict-free replicated data type |
| DoF | Degrees of freedom |
| ECO | Engineering change order |
| FEM | Finite element method |
| GD&T | Geometric dimensioning & tolerancing (ASME Y14.5) |
| IBL | Image-based lighting |
| LM | Levenberg-Marquardt |
| MBD | Model-based definition (Y14.41) |
| MCP | Model Context Protocol (Anthropic) |
| NURBS | Non-uniform rational B-spline |
| PBR | Physically-based rendering |
| PDM | Product data management |
| PMI | Product manufacturing information |
| SBOM | Software bill of materials |
| SCC | Strongly connected component |
| SemVer | Semantic versioning |
| SSAO | Screen-space ambient occlusion |
| TAA | Temporal anti-aliasing |
| WCAG | Web content accessibility guidelines |

---

## 13. Status snapshot (2026-05-06)

- 9-crate workspace + new `cadkernel-api`; ~2,865 / 0 / 0 tests on working tree.
- 13 primitives, 24 features, 5 boolean ops, NURBS kernel (28 algorithms), 24 sketch constraints, 11 file formats, 9 viewer workbenches.
- A1 done; A2 ~70 % through.
- Comparative parity vs v3 matrix: ~25 % of v1.0 cells, ~5 % of v2.0, ~0 % of v3.0.

**Work to v1.0:** finish A2, ship A3-A5, ship B1-B11, ship C-Solid v1, C-Sketch v3, C-Assy v1, C-Draw v1, all of D core. Estimated 2-3 quarters of focused work given the 5-team allocation.

---

## 14. Cross-references

- `WORK_STATUS.md` — running state of work.
- `UI_COMPLETION_ROADMAP.md` — superseded; per-stub inventory now feeds Track B.
- `FREECAD_PARITY_PLAN.md` — superseded; 576-feature checklist feeds Track C.
- `docs/wiki/Architecture.md` — addendum needed after A2 lands describing `cadkernel-api` + 6-track model.
- `docs/VERIFICATION_CHECKLIST.md` — gets one new line per phase plus reference-part assertions.
- `docs/adr/` — architecture decision records (kernel choice, CRDT choice, PBR-vs-deferred, etc.).

---

## 15. Changelog

- **2026-05-06 (v3.5, "corpus + ADR depth + first executable seed")** — three further moves on top of v3.4. **+5 ADRs** in [`docs/adr/`](adr/): [0011 automerge CRDT](adr/0011-automerge-crdt.md) (vs OT / Yjs / custom / RGA / LSEQ / Diamond Types / state-based; cites Kleppmann POPL 2017), [0012 wasm-first plugin sandbox via wasmtime](adr/0012-wasm-plugin-sandbox.md) (vs native-only / Lua-only / JS / process isolation / seccomp / wasmer / WASI Preview 1; capability-token model), [0013 CBOR RFC 8949](adr/0013-cbor-metadata.md) (vs JSON / msgpack / BSON / protobuf / FlatBuffers / Cap'n Proto / YAML / TOML / custom; chosen for self-describing tags + deterministic encoding rule), [0014 zstd RFC 8878](adr/0014-zstd-section-compression.md) (vs gzip / xz / brotli / LZ4 / snappy / no-compression / custom; level 3 default, level 19 archive, dictionary plan), [0015 sparse Cholesky for sketch LM linear step](adr/0015-sparse-cholesky-sketch.md) (vs dense LU / dense Cholesky / CG / MINRES / GPU / Eigen; nalgebra-sparse default, CHOLMOD opt-in for >1k vars). **+1 perf spec**: [memory-profile.md](perf/memory-profile.md) — per-entity heap-cost table for every topology and geometry type, R12 industrial-part 1 GB peak-RSS budget breakdown showing tessellation as the single largest cost, dhat measurement code, per-OS RSS measurement (`getrusage` on Linux/macOS, `GetProcessMemoryInfo` on Windows), allocator-choice rationale (system default + jemalloc/mimalloc opt-in), allocation-hotspots arena policy, regression CI matching the time-regression 5/15/50% gates. **+6 worked golden cases** in `tests/corpus/`: sketch [003 slot](../tests/corpus/sketch/golden/003_slot.toml) (tangent + EqualRadius + symmetric), [004 hexagon](../tests/corpus/sketch/golden/004_hexagon.toml) (EqualLength + 120° angle), [005 triangle-in-circle](../tests/corpus/sketch/golden/005_triangle_in_circle.toml) (UnderDetermined with documented remaining DoF), [006 parallel-redundant-tangent](../tests/corpus/sketch/golden/006_parallel_redundant_tangent.toml) (OverDetermined with explicit redundant-constraint id); boolean [003 box∩box](../tests/corpus/boolean/golden/003_box_intersection_box.toml) (1.5³ result), [004 sphere∩box](../tests/corpus/boolean/golden/004_sphere_intersection_box.toml) (sphere-face-kind preserved through boolean), [006 box∪box idempotent](../tests/corpus/boolean/golden/006_box_union_idempotent.toml) (algebraic property assertion). **+2 Lua reference-part scripts**: [r1_box.lua](../tests/corpus/reference_parts/r1_box.lua) (V=8/E=12/F=6 + Euler-Poincaré + volume + area + centroid + tag-completeness + content-hash) and [r2_extrude.lua](../tests/corpus/reference_parts/r2_extrude.lua) (extrude-with-hole, π·r²·h volume check, tag survival across cap imprint). **+1 first executable Rust seed**: [examples/build_reference_parts.rs](../examples/build_reference_parts.rs) — `cargo run --release --example build_reference_parts -- --output tests/corpus/reference_parts/` scaffold that compiles today, calls real `quick_box` / `cadk::write` once those APIs land, otherwise reports clean NotImplemented for R3-R12. The 5-pillar contract (roadmap / algorithms / adr / perf / corpus) is now backed by 15 ADRs, 4 perf specs, 13 algorithm specs, 11 worked TOML goldens, 2 Lua scripts, and one compilable example. Project documentation + corpus = 9,299 lines.
- **2026-05-06 (v3.4, "decision records + perf methodology + corpus seed")** — three new documentation pillars complementing the v3.3 algorithm spec series. **`docs/adr/`** (10 ADRs in Michael Nygard format with Status / Context / Decision / Alternatives / Consequences / References): [0001 half-edge B-Rep](adr/0001-half-edge-vs-winged-edge.md) (vs winged-edge / quad-edge / IFS / vertex-vertex / GMap), [0002 BLAKE3 cache key](adr/0002-blake3-cache-key.md) (vs SHA-256 / SHA-3 / xxHash3 / FNV / MD5 / Blake2b with collision-probability calculation $1.5 \cdot 10^{-21}$ across $10^9$ entries), [0003 nalgebra+glam split](adr/0003-nalgebra-vs-glam.md), [0004 egui+wgpu+winit GUI stack](adr/0004-egui-wgpu-stack.md) (vs Qt6 / GTK4 / Slint / iced / Bevy UI / native / Tauri / ImGui), [0005 mlua Lua 5.4](adr/0005-mlua-scripting.md) (vs Rhai / Python in-proc / V8 / Wasm / custom DSL), [0006 PyO3 separate excluded crate](adr/0006-pyo3-bindings.md), [0007 tag-based persistent naming](adr/0007-tag-based-persistent-naming.md) (vs pure geometric matching with PTC Pro/E history lesson, UUID, hash-of-geometry, pointer / arena ID), [0008 Rust edition 2024 MSRV 1.85](adr/0008-rust-edition-2024-msrv.md), [0009 custom binary `.cadk`](adr/0009-cadk-binary-format-vs-json.md) (vs JSON / msgpack-only / sqlite / Arrow-Parquet / HDF5 / protobuf-flatbuffers-capnp / OCCT BinXCAFFormat / STEP-as-native), [0010 Rayon for kernel data parallelism, no async kernel](adr/0010-rayon-data-parallelism.md). **`docs/perf/`** (3 performance specs): [methodology.md](perf/methodology.md) (R1-R5 reference HW tiers, RUSTFLAGS + Criterion config + cold-vs-warm + RNG seed + CPU isolation + determinism enforcement + dhat heap profiling + 16.67 ms frame-budget breakdown + 5/15/50 % regression policy + PGO plan + perf-counter analysis + flame graphs + cross-arch parity), [dispatch-matrix.md](perf/dispatch-matrix.md) (SSI dispatch 18 surface-pair rows analytical→NURBS-general, boolean coplanar pre-classification, fillet 5-tier auto-up-tier rules, sketch solver LM↔dogleg switch, tessellation refinement, BVH leaf vs split, numerical-precision escalation f64→interval→exact-rational→Yap, STEP entity export 9 surface-kind rows), [condition-numbers.md](perf/condition-numbers.md) (per-algorithm $\kappa$ thresholds — sketch LM $10^8$, NURBS SSI Newton $10^7$, boolean predicate ULP-100, sparse Cholesky $10^{10}$ damped, etc — Hager 1-norm cheap detection, worked example for sketch near-singular case showing OverDetermined classification, worked example for boolean coplanar coincident faces, full named-tolerance hierarchy `EPSILON_*` constants, cross-architecture FMA + iteration-order determinism). **`tests/corpus/`** (test corpus scaffold): [README.md](../tests/corpus/README.md) (4-tier layout per algorithm), [sketch/golden/MANIFEST.md](../tests/corpus/sketch/golden/MANIFEST.md) (12 hand-crafted reference sketches), 3 worked TOML sketches: [001 rectangle](../tests/corpus/sketch/golden/001_rectangle.toml) WellDetermined, [002 concentric circles](../tests/corpus/sketch/golden/002_concentric_circles.toml), [007 inconsistent square](../tests/corpus/sketch/golden/007_inconsistent_square.toml) Inconsistent + minimal-conflict-set assertion, [boolean/golden/MANIFEST.md](../tests/corpus/boolean/golden/MANIFEST.md) (10 boolean cases), 2 worked TOML booleans: [001 box ∪ box](../tests/corpus/boolean/golden/001_box_union_box.toml), [002 box − inner box](../tests/corpus/boolean/golden/002_box_minus_inner_box.toml) genus-0 cavity, [reference_parts/MANIFEST.md](../tests/corpus/reference_parts/MANIFEST.md) (R1-R12 build scripts + per-part timing budgets). Project documentation grew from ~5,900 lines (v3.3) to ~10,000 lines. The roadmap now references the four-pillar structure: **roadmap = contract**, **algorithms = how**, **adr = why-not-otherwise**, **perf = how-fast-and-how-measured**, **corpus = proof**.
- **2026-05-06 (v3.3, "modular spec series")** — algorithm appendix (§16) split out of the roadmap into a per-algorithm spec series under [`docs/algorithms/`](algorithms/) and **expanded** beyond the v3.2 inline depth. 13 spec files created: [sketch-solver.md](algorithms/sketch-solver.md) (Marquardt damping + Powell-dogleg fallback + DR-planning per Bouma-Fudos-Hoffmann-Cai-Paige + SVD deflation + QuickXplain conflict-set + 24-row constraint Jacobian + drag-with-constraints budget); [nurbs-ssi.md](algorithms/nurbs-ssi.md) (Patrikalakis 3-phase loop-detection / tracing / approximation + marching ODE + predictor-corrector + Newton projection + tangential-Krishnan-Manocha fallback); [boolean.md](algorithms/boolean.md) (8-phase pipeline + Yap 1990 symbolic perturbation with worked example + interval arithmetic + winding-number point-in-solid Jacobson-Kavan-Sorkine-Hornung + Cherchi-Pellacini-Attene-Livesu mesh fallback); [fillet.md](algorithms/fillet.md) (5-tier ladder rolling-ball / variable-radius / Choi-Lee setback / Vida-Martin-Várady $G^2$ / conic-Bezier $G^3$ + edge-set planning); [brep-abi.md](algorithms/brep-abi.md) (full byte layout for HEADER 64 B + METADATA CBOR + DOCUMENT + TOPO with 7 record types + GEOM with 6 curve and 9 surface types + TAGS + HIST + THUMB + EXT + TRAILER + v0→v1 migration shim example); [step-mapping.md](algorithms/step-mapping.md) (24-row CADKernel→AP242 entity table + worked R1-box example + canonicalisation methodology + AP214/AP242/LOTAR + NIST CAX-IF compliance procedure + GD&T mapping); [recompute.md](algorithms/recompute.md) (BLAKE3 cache key + dirty propagation + persistent on-disk cache); [bvh.md](algorithms/bvh.md) (SAH + binning + Kay-Kajiya slabs + sub-tree rebuild thresholds); [tessellation.md](algorithms/tessellation.md) (UV grid + adaptive refinement + Shewchuk Triangle CDT + smooth-group BFS at 60° crease); [persistent-naming.md](algorithms/persistent-naming.md) (Tag struct + per-operation generation rules + survival heuristic with similarity scoring); [test-corpus.md](algorithms/test-corpus.md) (4-tier corpus + 1M boolean fuzz cases + 290-part STEP corpus + 60 visual baselines + per-crate coverage); [microbenchmarks.md](algorithms/microbenchmarks.md) (20-row Criterion catalogue + reference HW + ±5% regression detection); [chaos-tests.md](algorithms/chaos-tests.md) (12 failure-injection scenarios + loom + allocator hooks + mock filesystem). Each spec file declares its own owner, version stamp, last-updated date, ISO/standards alignment, primary-literature references (Patrikalakis-Maekawa, Yap, Hoffmann, Choi-Lee, Vida-Martin-Várady, Shewchuk, Wald, Kay-Kajiya, Junker, Bouma-Fudos-Hoffmann-Cai-Paige, Mäntylä, Jacobson-Kavan-Sorkine-Hornung, Cherchi-Pellacini-Attene-Livesu, ISO 10303-242:2020, ISO 10303-21:2016, RFC 8949 CBOR, RFC 8878 zstd, BLAKE3 spec, IEEE 754). The roadmap §16 is now a slim TOC + manifest pointing to the spec series; total project documentation grew from 3,712 lines to ~5,900 lines while gaining modular ownership boundaries that prevent merge conflicts between kernel-engineer / io-engineer / qa-engineer.
- **2026-05-06 (v3.2, "industrial-grade depth")** — every previously-thin section expanded to Track-B-level depth. **Track A**: A1 + A2 each given full Rust struct definitions (Session / Command / Outcome with all 14+ variants, ExtrudeSpec / LinearPatternSpec / MirrorSpec / Document::history), undo-redo coalescing semantics, JSON save/load, SessionSnapshot, Lua bridge migration, full test inventory (14 unit + 6 integration + 2 proptest + doc + workspace verify), `cargo public-api` baseline gate. **Track C**: every one of 13 sub-tracks given v0.5 / v1.0 / v2.0 / v3.0 phasing breakdown with concrete deliverables per tier; C-Solid added per-feature spec contract template, fillet algorithm tier ladder (rolling-ball / NURBS-swept-blend / set-back 3-way / G2 curvature-continuous / conic-Bezier), boolean robustness (Yap 1990 symbolic perturbation, interval arithmetic, post-op validation), pattern instance overrides; C-Sketch added 28-row constraint-coverage table v1/v2/v3; C-Surface added 16-row NURBS algorithm shopping table (de Boor / Boehm / Tiller / Prautzsch / Choi-Lee / Vida-Martin-Várady / Marker-Sederberg / Patrikalakis / Beier-Chen / Hoschek-Lasser); C-Assy 3-tier with 10-level deep sub-assembly + configurations + design tables + top-down + 10k+ component handling; C-Mech kinematic v2 / dynamics+contact+flexible-body MOR v3 with 4-test acceptance (Watt linkage / Stanford-arm IK / gear-train / drop-test) and Featherstone / Mirtich / Anitescu-Potra / Shabana refs; C-Sheet, C-Weld, C-Mold, C-Draw, C-Sim, C-CAM, C-Render, C-Reverse, C-Subdiv all phased with concrete deliverables (auto parting-line / conformal cooling, Y14.5-2018 / Y14.41 MBD / 3D PDF PRC / DWG round-trip, linear / nonlinear+thermal+contact / plastics+CFD-lite+explicit+topo-opt, 2.5/3/5-axis with Haas/Fanuc/Heidenhain/Mazak/Siemens posts, PBR-IBL / GPU PT+SVGF+ReSTIR / VR walkthrough+USD, mesh-repair+auto-segment+per-region-fit / B-spline arbitrary topology, T-spline basic / Class-A auto-conversion). **Track D**: D1 added 3-platform reference HW (X1 Carbon / M1 Air / Surface Pro 9) + CI matrix + 40-row per-op budget table (warm start, R4-R7-R11-R12, sketch add/drag/solve at multiple scales, recompute incremental, tessellation R3, hover, property commit, undo at scale, save/load .cadk, STEP I/O, multi-scale fps, frame-budget breakdown 16 ms total, memory targets); D2 robustness expanded with per-fuzz-target list + corpus sizes + soak-test + panic audit policy + `Send + Sync` static_assertions + determinism cross-platform; D3 test infrastructure expanded with 17-row test-category matrix + reference-part R1-R12 timing budgets + industry compliance suites (NIST CAX-IF ≥ 95/99/100 %, ASME Y14.5 GD&T 200-part corpus, JT 10.7, LinuxCNC, DXF AC1027) + 5-OS CI matrix; D4 distribution expanded with 11-row per-platform packaging matrix (AppImage/.deb/.rpm/snap/flatpak signing + macOS notarisation entitlements + Authenticode EV + RFC 3161 timestamp + Sigstore cosign + delta-rsync auto-update + SLSA Level 3 + reproducible-build SOURCE_DATE_EPOCH); D5 documentation expanded to 14 deliverables (mdBook 11 sections + per-Command doc-test gate + 12 video tutorials + AI cookbook with prompt examples + ADR + glossary auto-extraction + multi-language summary); D6 security expanded to 18 deliverables (SECURITY.md + cargo-deny + cargo-vet + cargo-public-api + wasmtime capability tokens + GDPR DPA + CycloneDX 1.5 SBOM + SLSA L3 + Sigstore Rekor + reproducible verification + threat model + annual pentest + bug bounty + CVE process); D7 release engineering expanded with quarterly stable / monthly patch / weekly beta / nightly continuous / LTS 2-year + SemVer tier-stability map + migration `cadkernel-compat` + release checklist. **Track E**: E1 plugin SDK expanded with architecture diagram + Wasm + native + manifest + lifecycle hooks + permission model + signed plugin workflow + cadkernel-sdk + CLI tooling + 5 reference plugins fully spec'd + plugin debugger; E2 marketplace expanded with submission workflow + automated security scan + manual review for native + categories + monetisation 80/20 + in-app browser + reporting; E3 education expanded with academic licence + classroom kit + Certified Associate / Professional exams + community gallery + reference textbook + YouTube + Discord + CADKernel Day; E4 API stability with 4-tier guarantee map + RFC process + cadkernel-compat shim + 5-year API v1 commitment. **Track F**: F1 CRDT expanded with full architecture diagram + automerge wrapper + (actor_id, lamport, parent_hash) op model + per-op-type conflict resolution policy + snapshot compression + GC + sync protocol + validation + 3-actor convergence acceptance; F2 cloud expanded with self-hostable architecture + S3-compat backends (MinIO / R2 / B2 / Wasabi) + OAuth2+OIDC + ACL + revisions/branches/diff/merge + GDPR data residency + audit log; F3 RT collab UX expanded with WebRTC SFU (mediasoup/LiveKit) + activity feed + @mentions + following mode + concurrent edit warnings + conflict UI + offline indicator + session recording; F4 PDM expanded with full lifecycle states + ECO workflow + where-used graph + reference checking + audit trail (ISO 9001 / FDA 21 CFR Part 11) + roles + locking + PLMXML integration + multi-level BOM. **§10 cross-cutting**: §10.4 IP audit table expanded from 17 to 30 rows with `Why we need it` / `Fallback if removed` columns covering tokio / axum / proptest / cargo-fuzz / cargo-mutants / pixelmatch / image / naga / ash / metal-rs / dhat-rs / tracing / Featherstone-rs / nalgebra-sparse; §10.6 perf-vs-commercial expanded from 6 to 30 rows with v1.0 / v2.0 / v3.0 columns covering R1-R12 + sketch drag at multiple scales + boolean + STEP I/O + ray-traced render + RAM working set + disk size + undo. Roadmap grew from 2,162 lines to multi-thousand-line industrial spec.
- **2026-05-06 (v3.1, "UX deep-dive")** — Track B rewritten: 12 phases → 20 phases, no more stub subsections. Added §5.0 information architecture (window shell, panel zones, dock semantics, multi-monitor, multi-workbench). Added §5.1 design system (colour tokens for light / dark / HC, typography scale, spacing, motion tokens, 39-component library catalogue, iconography, cursors). Added §5.2 interaction model (mouse maps × 3 schemes, keyboard maps global + viewport, touch gestures, pen input). Added §5.3 per-workbench wireframes (Part / Sketcher / Assembly / Drawing fully drawn, others enumerated). Added §5.4 per-workbench keyboard tables. Expanded B0 foundation refactor; expanded B1-B5 with concrete numbers (latency budgets, hit-target sizes, animation durations); promoted ex-stubs B6/B7/B9/B10 to fully-detailed phases; added B13 motion, B14 notifications, B15 search, B16 settings, B17 telemetry, B18 drawing-UX, B19 collaboration-UX, B20 a11y deep-dive. Added §5.7 UX-specific verification (visual regression, interaction tests, a11y CI, i18n CI, first-run study). Roadmap grew from 1,435 → 2,162 lines.
- **2026-05-06 (v3, "industrial-grade")** — full rewrite. 4 tracks → 6. 25 non-negotiables → 60. R1-R4 → R1-R12. Track C exploded to 13 sub-tracks. Comparative parity matrix vs SW/CATIA/F360/Inventor/FreeCAD. Algorithm bibliography. IP audit table. Industry-compliance benchmarks. Concrete release calendar v0.5 → v3.0.
- **2026-05-06 (v2)** — 4-track restructure; 11 → 25 non-negotiables; UI/UX from one phase to nine.
- **2026-05-06 (v1)** — 8-phase monotonic; 11 non-negotiables.

---

## 16. Algorithm appendix — modular spec series

**v3.3 update (2026-05-06):** the inline algorithm appendix that occupied this section in earlier drafts has been split into per-algorithm spec files under `docs/algorithms/`. Each file is a self-contained engineering specification with formulas, pseudocode, failure modes, numerical guards, acceptance gates, and primary-literature references. Splitting lets each algorithm grow independently without bloating the roadmap, and lets owners (kernel-engineer / io-engineer / qa-engineer) iterate without merge conflicts.

The roadmap remains the contract; the spec series is the proof of feasibility.

### 16.1 Index

See [docs/algorithms/README.md](algorithms/README.md) for the manifest and authoring conventions. Direct links:

| # | Spec file | Roadmap section it underwrites | Owner |
|---|---|---|---|
| 1 | [sketch-solver.md](algorithms/sketch-solver.md) | §6 (C-Sketch) | kernel-engineer |
| 2 | [nurbs-ssi.md](algorithms/nurbs-ssi.md) | §3 (geometry), §4 (boolean prereq) | kernel-engineer |
| 3 | [boolean.md](algorithms/boolean.md) | §4 (booleans) | kernel-engineer |
| 4 | [fillet.md](algorithms/fillet.md) | §5 (D-Fillet ladder) | kernel-engineer |
| 5 | [brep-abi.md](algorithms/brep-abi.md) | §7 (file format) | io-engineer |
| 6 | [step-mapping.md](algorithms/step-mapping.md) | §7 (interop) | io-engineer |
| 7 | [recompute.md](algorithms/recompute.md) | §8 (history) | kernel-engineer |
| 8 | [bvh.md](algorithms/bvh.md) | §3, §9 (picking) | kernel-engineer |
| 9 | [tessellation.md](algorithms/tessellation.md) | §3, §9 | kernel-engineer + ui-engineer |
| 10 | [persistent-naming.md](algorithms/persistent-naming.md) | §8 | kernel-engineer |
| 11 | [test-corpus.md](algorithms/test-corpus.md) | §11 (verification) | qa-engineer |
| 12 | [microbenchmarks.md](algorithms/microbenchmarks.md) | §11, §12 (perf) | qa-engineer |
| 13 | [chaos-tests.md](algorithms/chaos-tests.md) | §11 (robustness) | qa-engineer |

### 16.2 What the spec series guarantees

For every roadmap phase exit-gate that cites an algorithm:

1. **Mathematical formulation** — closed-form equations, not vague descriptions.
2. **Pseudocode** — language-neutral, executable on paper.
3. **Failure modes table** — every numerical edge case named with detection and mitigation.
4. **Numerical guards** — every tolerance is a named constant with a value and a purpose; no magic numbers.
5. **Acceptance gates** — per-version gates (v0.5 / v1.0 / v2.0 / v3.0) with concrete test counts and pass criteria.
6. **References** — primary literature first (Patrikalakis-Maekawa, Yap, Hoffmann, Choi-Lee, Vida-Martin-Várady, Shewchuk, Wald, Kay-Kajiya, Junker, Bouma-Fudos-Hoffmann-Cai-Paige, Mäntylä, Jacobson-Kavan-Sorkine-Hornung, Cherchi-Pellacini-Attene-Livesu).
7. **ISO/standards alignment** — ISO 10303-242:2020 mapping table, IEEE 754 binary64 for all geometry, BLAKE3 for cache keys, RFC 8949 CBOR for metadata, RFC 8878 zstd for compression.

### 16.3 Authoring rules

- One algorithm per file.
- Math in KaTeX (`$inline$`, `$$block$$`).
- Pseudocode in fenced text blocks; never literal Rust (lets spec drift detector fail clean if implementation diverges).
- Failure-modes table mandatory.
- Cross-link to roadmap section that consumes the algorithm.
- Version stamp + last-updated date in frontmatter.
- No full implementations — specs say what, not exact how.

### 16.4 Coverage summary (2026-05-06)

13 algorithm files, ~2,800 lines total. Aggregate coverage:

- **Sketch solver**: 14-row Jacobian table, full LM loop with Marquardt damping, Powell-dogleg fallback, DR-planning per Bouma-Fudos-Hoffmann-Cai-Paige, deflation via SVD, QuickXplain conflict-set extraction, drag-with-constraints budget analysis.
- **NURBS SSI**: Patrikalakis 3-phase algorithm, marching ODE with predictor-corrector, Newton projection on intersection manifold, adaptive step control, tangent-Krishnan-Manocha fallback.
- **Boolean**: 8-phase pipeline, Yap 1990 symbolic perturbation with worked example, interval arithmetic + exact-rational fallback, winding-number point-in-solid (Jacobson-Kavan-Sorkine-Hornung), Cherchi-Pellacini-Attene-Livesu mesh fallback.
- **Fillet**: 5-tier ladder (rolling-ball / variable-radius / Choi-Lee setback / Vida-Martin-Várady $G^2$ / conic-Bezier $G^3$), edge-set planning, per-tier failure modes.
- **`.cadk` ABI**: byte-exact specification for HEADER, METADATA (CBOR), DOCUMENT, TOPO (7 record types), GEOM (6 curve + 9 surface types with `NurbsCurvePayload` / `NurbsSurfacePayload`), TAGS, HIST, THUMB, EXT, TRAILER. Full backward/forward-compatibility framework with v0→v1 migration example.
- **STEP AP242**: 24-row entity-mapping table, worked R1-box export example, round-trip + canonicalisation methodology, AP214-vs-AP242 differences, PMI/GD&T mapping, NIST CAX-IF compliance procedure, LOTAR alignment.
- **Recompute**: dirty propagation, BLAKE3 cache key, per-architecture determinism, persistent cache.
- **BVH**: SAH split with binning, slab-method traversal, sub-tree rebuild thresholds.
- **Tessellation**: UV grid + adaptive refinement, Shewchuk Triangle CDT for trim loops, smooth-group BFS at 60° crease.
- **Persistent naming**: Tag struct, generation rules per operation, survival heuristic with similarity scoring.
- **Test corpus**: 4-tier organisation, 1M boolean fuzz cases, 290-part STEP corpus, 60 visual baselines, per-crate coverage targets.
- **Microbenchmarks**: 20-row Criterion catalogue with reference HW, $\pm 5\%$ regression detection.
- **Chaos tests**: 12 failure-injection scenarios with `loom` for concurrency, allocator hooks for OOM, mock filesystem for ENOSPC.

The specs above are not aspirational; they are the contracts that the implementation crates (`crates/sketch`, `crates/geometry`, `crates/modeling`, `crates/io`, `crates/topology`) must satisfy at each phase exit.

---

## 17. Verification cross-reference

This roadmap is a contract. The contract is enforced by:

1. **`docs/VERIFICATION_CHECKLIST.md`** — gets one new line per phase plus reference-part assertions.
2. **[`tests/corpus/`](../tests/corpus/README.md)** — golden test corpus: R1-R12 reference parts + hash baselines, sketch / boolean / SSI / STEP goldens, regression sub-corpora, visual baselines for SSIM comparison.
3. **`crates/*/tests/`** — per-crate test suites covering every algorithm in [`docs/algorithms/`](algorithms/README.md).
4. **`crates/modeling/benches/`** — Criterion benches matching [`docs/algorithms/microbenchmarks.md`](algorithms/microbenchmarks.md), measured per [`docs/perf/methodology.md`](perf/methodology.md).
5. **`fuzz/`** — `cargo-fuzz` targets matching §D2.
6. **`.github/workflows/`** — CI pipelines enforcing all gates.
7. **[`docs/adr/`](adr/README.md)** — 10 architecture decision records explaining *why-not-otherwise* for each load-bearing technical choice (half-edge, BLAKE3, nalgebra+glam split, egui+wgpu, mlua, PyO3-as-separate-crate, tag-based naming, Rust edition 2024, custom `.cadk`, Rayon-only kernel).
8. **[`docs/algorithms/`](algorithms/README.md)** — 13 per-algorithm spec files (sketch-solver, nurbs-ssi, boolean, fillet, brep-abi, step-mapping, recompute, bvh, tessellation, persistent-naming, test-corpus, microbenchmarks, chaos-tests).
9. **[`docs/perf/`](perf/methodology.md)** — performance contract: measurement methodology, algorithm-dispatch matrices, per-algorithm condition-number safe limits with named tolerance constants.

Every phase exit gate must cite at least one of the above.

The project documentation forms a five-pillar structure:

| Pillar | Question it answers | Lives in |
|---|---|---|
| **Roadmap** | What will be built, when, with what parity gate | this file |
| **Algorithms** | How each piece is built (math + pseudocode + numerical guards) | [`docs/algorithms/`](algorithms/README.md) |
| **ADRs** | Why this choice and not the obvious alternatives | [`docs/adr/`](adr/README.md) |
| **Perf** | How fast, on what hardware, by what measurement methodology | [`docs/perf/`](perf/methodology.md) |
| **Corpus** | Proof that the implementation matches the contract | [`tests/corpus/`](../tests/corpus/README.md) |

Any new feature touching the kernel must, at minimum, add: a row to the roadmap, a section to its algorithm spec, and one or more golden test cases to the corpus. Major load-bearing choices additionally require a new ADR.

---

**End of v3.4 (decision records + perf methodology + corpus seed, 2026-05-06).**
