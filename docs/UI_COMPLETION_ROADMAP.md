# UI Completion Roadmap

**Status:** Active. Started 2026-04-30.
**Bilingual:** [한국어 요약](UI_COMPLETION_ROADMAP.ko.md)

## 1. Why This Document Exists

Prior status reports claimed "576/576 features dispatcher-reachable." This was technically true but materially misleading: ~79 of those dispatcher arms in `crates/viewer/src/app.rs` are pure `self.log_info("...")` stubs that route a UI click to a single log line and produce **no geometry, no scene change, no user-visible effect.** The user clicked PartDesign Pad and got a log message, not a solid.

The recent V37 session work (module split, sub-enum partition, panic-safety fixes, test corpus) was engineering hygiene — necessary, but it did not add a single working feature. The user's frustration ("간단한 도형 늘리기도 안되는 판에 이게 어떻게 cad라고 할 수 있겠어") is correct: **basic CAD operations including sketch-driven Pad / Pocket / Hole are non-functional today**.

> **Status 2026-05-14 (UI-A1)**: The 5 PartDesign sketch-driven dispatcher arms (Pad / Pocket / Groove / Hole / CountersunkHole) were verified wired and producing scene geometry as of HEAD `714136e`. The roadmap's earlier characterisation of these five as "ALSO log_info" stubs was outdated — the dispatcher arms call the modeling kernel and feed results into the scene. Dispatcher-boundary tests were added in `crates/viewer/tests/partdesign_sketch_features.rs` (6 tests, all green) to lock in this guarantee.

> **Status 2026-05-14 (UI-A2)**: All 19 Draft workbench EASY-tier dispatcher arms (`D::Line` through `D::ToSketch`) were verified already wired to `cadkernel_modeling::draft_ops::*` at HEAD `584d334`. The §3.1 table's "stub" classification was outdated. Dispatcher-boundary tests were added in `crates/viewer/tests/draft_easy_features.rs` (20 tests, all green, workspace 3,337/0/1).

> **Status 2026-05-14 (UI-A3)**: All 13 Part workbench EASY dispatcher arms (`P::FaceFromWires`, `P::ConnectShapes`, `P::EmbedShapes`, `P::CutoutShapes`, `P::ExplodeCompound`, `P::CompoundFilter`, `P::BooleanFragments`, `P::SliceToCompound`, `P::PointsFromShape`, `P::ConvertToSolid`, `P::AutoDefeaturing`, `P::TransformedCopy`, `P::CoonsPatch`) were verified already wired to `cadkernel_modeling::features::*`. The §3.3 table's "stub" classification was outdated. Dispatcher-boundary tests were added in `crates/viewer/tests/part_easy_features.rs` (14 tests, all green, workspace 3,351/0/1). `P::ProjectCurvesOnSurface` is MEDIUM tier and was excluded from UI-A3 scope.

> **Verify-first pattern (2026-05-14, 3rd confirmation)**: Three consecutive verify-first lanes (UI-A1 PartDesign 5, UI-A2 Draft 19, UI-A3 Part 13) all found everything already wired — **37 supposed "stubs" were actually functional**. The roadmap lagged behind the kernel wiring work. **Future UI-Ax phases must start verify-first** before any wiring work is planned: check the dispatcher arm bodies before assuming they are log_info-only stubs.

> **Status 2026-05-14 (UI-A4)**: 27/27 dispatcher arms across Surface §3.4 (1 = `S::Coons`), FEM §3.5 (2 = `FemAction::Summary`/`Report`), TechDraw §3.6 (24) verified already wired pre-A4. Cumulative verify-first counter: 5 + 19 + 13 + 27 = **64/64**. Additionally, 24 of 27 arms were already test-covered in prior test files (`S::Coons` by `gui_action_integration.rs:2851`; 23 TechDraw arms by non-trivial SVG/DXF/PDF assertions). Only 3 arms had tautological prior tests and needed strengthening: new tests in `crates/viewer/tests/fem_easy_features.rs` (4 tests) and `crates/viewer/tests/techdraw_done_features.rs` (1 test). Workspace: 3,351 → **3,356 / 0 / 1**.

This roadmap is a structured plan to actually wire the UI to the kernel. It is the canonical reference for the multi-session UI completion effort.

## 2. The Surprise (Good News)

**Most of the work has already been done at the kernel level.** A targeted audit found that for ~50-60 of the 79 stubs, the kernel API exists in `crates/modeling/` and just needs to be called from the dispatcher arm. No new kernel code; pure wiring.

| Stub category | Kernel API status | Estimated effort |
|---|---|---|
| EASY — kernel API exists, wire only | ~15 stubs (was ~55; 5 PD verified DONE UI-A1; 19 Draft DONE UI-A2; 13 Part DONE UI-A3; 3 Surface/FEM DONE UI-A4 = S::Coons + FemAction::Summary + FemAction::Report) | 15-30 min each |
| MEDIUM — kernel API exists, needs UX (modal / picker / sketch ref) | ~15 stubs | 1-2 hours each |
| HARD — kernel API missing | ~9 stubs | 3-10 hours each (new kernel work) |

The 9-sub-enum dispatcher partition we landed last session (commits `9e42ece` … `d2966b6`) actually makes this fix easier — each `process_*_action` helper is a clean isolated dispatch point.

## 3. Per-Stub Inventory (79 total)

Stub line numbers refer to `crates/viewer/src/app.rs` at commit `1875230` (HEAD as of 2026-04-30). They will shift as fixes land; **the workbench enum variant name is the stable identifier — use that, not the line number, when tracking a specific stub after Phase A-C3 landed.**

### 3.1 Draft workbench (29 stubs)

| Variant | Stub line | Kernel API | Tier | Notes |
|---|---|---|---|---|
| `D::Line` | 3904 | `draft_ops::make_line_draft` | DONE (verified 2026-05-14) | Two-point line wire. Test: overlay polyline grew. |
| `D::Wire` | 3905 | `draft_ops::make_wire` | DONE (verified 2026-05-14) | Polyline. Test: overlay polyline grew. |
| `D::Circle` | 3906 | `draft_ops::make_circle_wire` | DONE (verified 2026-05-14) | scene+1, non-empty vertices. |
| `D::Arc` | 3907 | `draft_ops::make_arc_wire`, `make_arc_3pt_wire` | DONE (verified 2026-05-14) | scene+1, non-empty vertices. |
| `D::Ellipse` | 3908 | `draft_ops::make_ellipse_wire` | DONE (verified 2026-05-14) | scene+1, non-empty vertices. |
| `D::BSpline` | 3956 | `draft_ops::make_bspline_wire` | DONE (verified 2026-05-14) | scene+1, overlay polyline grew. |
| `D::Bezier` | 3957 | `draft_ops::make_bezier_wire`, `make_cubic_bezier_wire` | DONE (verified 2026-05-14) | scene+1, overlay polyline grew. |
| `D::Point` | 3958 | `draft_ops::make_point` | DONE (verified 2026-05-14) | scene+1, overlay point grew. |
| `D::Facebinder` | 3959 | `draft_ops::make_facebinder` | MEDIUM | Needs face selection. |
| `D::Hatch` | 3960 | `draft_ops::draft_hatch` | DONE (verified 2026-05-14) | scene+1, multiple polylines (boundary + fill). |
| `D::Move` | 3961 | `draft_ops::move_solid` | MEDIUM | Needs gizmo / numeric input modal. |
| `D::Rotate` | 3962 | `draft_ops::rotate_solid` | MEDIUM | Same as Move. |
| `D::Scale` | 3963 | `draft_ops::scale_solid_draft` | MEDIUM | Same as Move. |
| `D::Mirror` | 3964 | `draft_ops::mirror_solid_draft` | MEDIUM | Needs plane picker (or default planes). |
| `D::Offset` | 3965 | `draft_ops::offset_wire` | MEDIUM | Needs distance modal + wire selection. |
| `D::Trim` | 3966 | `draft_ops::trimex_draft` | MEDIUM | Needs target-point selection. |
| `D::Stretch` | 3967 | `draft_ops::stretch_wire` | MEDIUM | Needs vertex-and-vector selection. |
| `D::Clone` | 3968 | `draft_ops::clone_solid` | DONE (verified 2026-05-14) | Both positive (with selection) and no-op (without) covered. |
| `D::ArrayRect` | 3969 | `draft_ops::rectangular_array` | DONE (verified 2026-05-14) | scene grew past base (Rect 3×2). |
| `D::ArrayPolar` | 3970 | `draft_ops::polar_array`, `circular_array` | DONE (verified 2026-05-14) | scene grew past base (Polar 6-fold). |
| `D::ArrayPath` | 3971 | `draft_ops::path_array`, `path_link_array` | DONE (verified 2026-05-14) | scene grew past base. |
| `D::ArrayPoint` | 3972 | `draft_ops::point_array`, `point_link_array` | DONE (verified 2026-05-14) | scene grew past base. |
| `D::Dimension` | 3973 | `draft_ops::make_draft_dimension`, `make_draft_dimension_full` | MEDIUM | Needs overlay rendering. |
| `D::Label` | 3974 | `draft_ops::make_label`, `make_label_full` | MEDIUM | Same as Dimension. |
| `D::Text` | 3975 | `draft_ops::shape_from_text` | DONE (verified 2026-05-14) | scene+1, polylines+labels grew. |
| `D::Upgrade` | 3976 | `draft_ops::upgrade_wire`, `upgrade_wire_model` | DONE (verified 2026-05-14) | scene+1, non-empty vertices. |
| `D::Downgrade` | 3977 | `draft_ops::downgrade_solid`, `downgrade_solid_faces` | DONE (verified 2026-05-14) | Bonus test: box → 6 face-solids. |
| `D::WireToBSpline` | 3978 | `draft_ops::wire_to_bspline_convert` | DONE (verified 2026-05-14) | scene+1, overlay polyline grew. |
| `D::ToSketch` | 3979 | `draft_ops::draft_to_sketch` | DONE (verified 2026-05-14) | `last_sketch_is_set()` flipped to true, scene unchanged. |

**Subtotals:** DONE = 19 (verified 2026-05-14 via `crates/viewer/tests/draft_easy_features.rs`, 20 tests), MEDIUM = 10, HARD = 0.

### 3.2 PartDesign workbench (5 stubs — sketch-driven pad-family verified DONE 2026-05-14; 4 MEDIUM + 1 HARD remaining)

| Variant | Stub line | Kernel API | Tier | Notes |
|---|---|---|---|---|
| `Pd::AdditiveLoft` | 3850 | `features::loft` | MEDIUM | Needs profile-list selection. |
| `Pd::AdditivePipe` | 3851 | `features::sweep`, `surface_ops::pipe_surface` | MEDIUM | Needs profile + path selection. |
| `Pd::SubtractiveLoft` | 3852 | `features::loft` + `boolean_op_exact` | MEDIUM | Same as AdditiveLoft + boolean. |
| `Pd::SubtractivePipe` | 3853 | `features::sweep` + `boolean_op_exact` | MEDIUM | Same as AdditivePipe + boolean. |
| `Pd::ShapeBinder` | 3867 | `features::shape_binder` | DONE | Wired in working tree: copies selected shape faces into a new binder solid. |

**Status 2026-05-14 — DONE (verified)**: `Pd::PadSketch`, `Pd::PocketSketch`, `Pd::GrooveSketch`, `Pd::HoleSketch`, `Pd::CountersunkHoleSketch` were verified wired to `cadkernel_modeling::{pad, pocket, groove, hole, countersunk_hole}` in HEAD `714136e`. These dispatcher arms call the kernel and pass produced solids into the scene via `add_to_scene`. The earlier "ALSO log_info" characterisation was stale. Dispatcher-boundary tests in `crates/viewer/tests/partdesign_sketch_features.rs` (6 tests, all green as of 2026-05-14) lock in this guarantee. The "user's 간단한 도형 늘리기 complaint" that prompted this roadmap is resolved for the PartDesign sketch-driven tier.

**Subtotals:** EASY = 5 (the format-printing pad-family, all DONE 2026-05-14), MEDIUM = 4, HARD = 1.

### 3.3 Part workbench (14 stubs)

| Variant | Stub line | Kernel API | Tier | Notes |
|---|---|---|---|---|
| `P::FaceFromWires` | 3809 | `features::face_from_wires` | DONE (verified 2026-05-14) | scene+1 solid. |
| `P::ConnectShapes` | 3810 | `features::connect_shapes` | DONE (verified 2026-05-14) | scene+1; graceful no-op without selection. |
| `P::EmbedShapes` | 3811 | `features::embed_shapes` | DONE (verified 2026-05-14) | scene+1. |
| `P::CutoutShapes` | 3812 | `features::cutout_shapes` | DONE (verified 2026-05-14) | scene+1. |
| `P::ExplodeCompound` | 3813 | `features::compound_ops::explode_compound` | DONE (verified 2026-05-14) | log-only by design; scene unchanged. |
| `P::CompoundFilter` | 3814 | `features::compound_ops::compound_filter` | DONE (verified 2026-05-14) | log-only by design; scene unchanged. |
| `P::BooleanFragments` | 3815 | `features::compound_ops::boolean_fragments` | DONE (verified 2026-05-14) | log-only — multi-model staging pending Phase D-cont; scene unchanged. |
| `P::SliceToCompound` | 3816 | `features::compound_ops::slice_to_compound` | DONE (verified 2026-05-14) | scene > before (N pieces). |
| `P::PointsFromShape` | 3817 | `features::face_from_wires::points_from_shape` | DONE (verified 2026-05-14) | scene+1, overlay points grew. |
| `P::ConvertToSolid` | 3818 | `features::shape_convert::shape_from_mesh` | DONE (verified 2026-05-14) | scene+1 solid from mesh. |
| `P::AutoDefeaturing` | already wired | `features::defeature::auto_defeaturing` | DONE (verified 2026-05-14) | scene+1. |
| `P::TransformedCopy` | already wired | `multi_transform::multi_transform` | DONE (verified 2026-05-14) | scene+1. |
| `P::ProjectCurvesOnSurface` | 3825 | `features::projection::project_curve_on_solid` | MEDIUM | Needs curve + surface selection. Excluded from UI-A3 scope; pick up in MEDIUM-tier phase. |
| `P::CoonsPatch` | 3826 | `surface_ops::coons_patch` | DONE (verified 2026-05-14) | scene+1 solid, overlay polyline grew. |

**Subtotals:** DONE = 13 (verified 2026-05-14 via `crates/viewer/tests/part_easy_features.rs`, 14 tests), MEDIUM = 1, HARD = 0.

### 3.4 Surface workbench (4 stubs — Filling/Boundary/Pipe already real)

| Variant | Stub line | Kernel API | Tier | Notes |
|---|---|---|---|---|
| `S::Sections` | 3776 | `surface_ops::sections`, `features::cross_sections` | MEDIUM | Needs profile selection. |
| `S::Extend` | 3777 | `surface_ops::extend_surface` | MEDIUM | Needs surface selection + distance. |
| `S::Blend` | 3778 | `surface_ops::surface_from_curves` (closest) | MEDIUM | API is partial. |
| `S::Coons` | 3802 | `surface_ops::coons_patch` | DONE (verified 2026-05-14) | Covered by `gui_action_integration.rs:2851`. |

**Subtotals:** DONE = 1 (verified 2026-05-14), EASY remaining = 0, MEDIUM = 3, HARD = 0.

### 3.5 FEM workbench (7 stubs)

| Variant | Stub line | Kernel API | Tier | Notes |
|---|---|---|---|---|
| `FemAction::SolveThermal` | 2826 | `AnalysisContainer::run_thermal_static` | DONE | Wired in working tree: steady-state thermal solve stores `temperature_field`. |
| `FemAction::SolveNonlinear` | 2830 | `AnalysisContainer::run_nonlinear` | DONE | Wired in working tree: Newton-Raphson nonlinear static solve stores `FemResult`. |
| `FemAction::ShowStress` | 2831 | viewer FEM colormap boundary mesh | DONE | Working tree: 7-band scene-object colormap from active `FemResult`. |
| `FemAction::ShowDisplacement` | 2832 | viewer FEM colormap boundary mesh | DONE | Working tree: nodal displacement magnitude colormap. |
| `FemAction::ShowVonMises` | 2833 | viewer FEM colormap boundary mesh | DONE | Working tree: per-element Von Mises averaged to boundary nodes. |
| `FemAction::Summary` | 2834 | (text output trivial) | DONE (verified 2026-05-14) | Strengthening tests in `fem_easy_features.rs` assert status-text + warning-text. |
| `FemAction::Report` | 2835 | (text output trivial) | DONE (verified 2026-05-14) | Same; strengthened in `fem_easy_features.rs`. |

**Subtotals:** DONE = 7 (all verified; Summary/Report strengthened 2026-05-14), EASY remaining = 0, HARD remaining = 0.

### 3.6 TechDraw workbench (24 stubs)

Working tree status: `NewPage`, `FromTemplate`, `Redraw`, `SectionView`, `DetailView`, `BrokenView`, all six dimension variants (`DimLinear`, `DimRadius`, `DimDiameter`, `DimAngle`, `DimArcLen`, `DimArea`), all six annotation variants (`Text`, `RichText`, `Balloon`, `Leader`, `Weld`, `SurfFinish`), all four centerline variants (`CenterFace`, `CenterLines`, `CenterPoints`, `BoltCircle`), `ExportDxf`, and `ExportPdf` are now wired. DXF/PDF export uses new `crates/io/src/techdraw_dxf.rs` and `crates/io/src/techdraw_pdf.rs` helpers; dimensions, annotations, and centerlines render through `drawing_to_svg` (and therefore PDF). TechDraw HARD backlog is now closed at the dispatcher/storage/rendering level.

**Status 2026-05-14 (UI-A4)**: All 24 TechDraw arms verified already wired and test-covered. 23 arms covered by prior non-trivial tests (SVG/DXF/PDF content assertions, sheet-size checks, etc.). `T::Redraw` no-sheet path strengthened in `crates/viewer/tests/techdraw_done_features.rs` (1 test) — prior test was a tautology.

**Subtotals:** DONE = 24 (all verified 2026-05-14; 23 already-covered + 1 strengthened), HARD remaining = 0.

### 3.7 Workbench-totals roll-up

| Workbench | Stub count | DONE | EASY remaining | MEDIUM | HARD |
|---|---:|---:|---:|---:|---:|
| Draft | 29 | 19 (verified 2026-05-14) | 0 | 10 | 0 |
| PartDesign (incl. format-print Pad-family) | 10 | 5 (verified 2026-05-14) | 0 | 4 | 1 |
| Part (incl. format-print stubs) | 14 | 13 (verified 2026-05-14) | 0 | 1 | 0 |
| Surface | 4 | 1 (verified 2026-05-14) | 0 | 3 | 0 |
| FEM | 7 | 7 (all verified; Summary/Report strengthened 2026-05-14) | 0 | 0 | 0 |
| TechDraw | 24 | 24 (all verified 2026-05-14; 23 already-covered + 1 strengthened) | 0 | 0 | 0 |
| **Total** | **88** | **64** | **0** | **18** | **1** |

(88 > 79 because the Pad-family + AutoDefeaturing + TransformedCopy format-print stubs were undercounted by the initial `log_info`-only grep, and TechDraw's extracted enum now exposes the full 24-action page/view/dimension/annotation/centerline/export backlog.)

## 4. Tiered Execution Plan

### Phase A — CRITICAL CAD WORKFLOW (Tier 1, ~10 features)

**Goal**: Sketch → Pad → Solid in scene works end-to-end. Without this the project cannot be called CAD.

Features:
- `Pd::PadSketch` — wire to `features::pad`
- `Pd::PocketSketch` — wire to `features::pocket`
- `Pd::GrooveSketch` — wire to `features::groove`
- `Pd::HoleSketch` — wire to `features::hole`
- `Pd::CountersunkHoleSketch` — wire to `features::countersunk_hole`
- `D::Line` — wire to `make_line_draft`
- `D::Circle` — wire to `make_circle_wire`
- `D::Arc` — wire to `make_arc_wire`
- `D::Ellipse` — wire to `make_ellipse_wire`
- `D::Point` — wire to `make_point`

**Estimated effort**: 4-6 hours. Each is a 15-30 minute wiring task.

**Acceptance test (manual, must pass)**: Open viewer, switch to Sketcher workbench, draw a closed rectangle profile, exit sketcher, switch to PartDesign, click "Pad…", enter depth=10, click Apply. **A new solid must appear in the scene with correct geometry.** Repeat for Pocket. Repeat for Hole.

**Deliverable**: 10 commits (one per feature) or one bundled commit. Tests stay 2,662 / 0 / 0 with at least 5 new dispatcher integration tests (one per Tier 1 feature) using the `test_support` harness from `dc55537`.

### Phase B — DRAFT 2D + ARRAYS (Tier 2 EASY, ~18 features)

Wire all remaining EASY-tier Draft variants and Part workbench EASY stubs:

- Draft 2D primitives: Wire, BSpline, Bezier, Hatch, Text, Upgrade, Downgrade, WireToBSpline, ToSketch
- Draft arrays: ArrayRect, ArrayPolar, ArrayPath, ArrayPoint, Clone
- Part operations: FaceFromWires, ConnectShapes, EmbedShapes, CutoutShapes, ExplodeCompound, CompoundFilter, BooleanFragments, SliceToCompound, PointsFromShape, ConvertToSolid, CoonsPatch, AutoDefeaturing (real apply), TransformedCopy (real apply)

**Estimated effort**: 8-12 hours. Per-task wiring + integration test.

**Deliverable**: One commit per workbench (Draft + Part) or per logical group. Test count grows.

### Phase C — SURFACE + ADDITIVE/SUBTRACTIVE LOFT/PIPE (Tier 2 MEDIUM, ~12 features)

Features that need UX (selection / modals):

- Draft transforms: Move, Rotate, Scale, Mirror (use existing gizmo as input)
- Draft modify: Offset, Trim, Stretch, Facebinder
- Surface: Sections, Extend, Blend, Coons
- PartDesign: AdditiveLoft, AdditivePipe, SubtractiveLoft, SubtractivePipe
- Part: ProjectCurvesOnSurface

**Estimated effort**: 15-20 hours. Each feature needs a small modal or selection workflow.

**Deliverable**: Per-feature commits with manual acceptance test described in commit message.

### Phase D — DRAFT ANNOTATION (Tier 2 MEDIUM, ~3 features)

- Draft Dimension, Label (need overlay rendering — non-trivial)
- These are visual-only, no scene-geometry change.

**Status (2026-05-04 working tree)**: Implemented. `gui::scene_overlay` paints world-space polylines, points, and labels through an egui foreground layer. `D::Dimension` and `D::Label` now produce visible overlay annotations, and prior wire-output Draft/Part/Surface features now render instead of being tree-only.

### Phase E — FEM SOLVER + COLORMAP VISUALIZATION (Tier 3 HARD, 5 features + 2 EASY)

- `Summary`, `Report` — EASY (text formatting)
- `ShowStress`, `ShowDisplacement`, `ShowVonMises` — render pipeline work (Phase O-b from prior plan)
- `SolveThermal`, `SolveNonlinear` — kernel solver extension

**Status (2026-05-04 working tree)**: Implemented. `SolveThermal` and `SolveNonlinear` run through `AnalysisContainer`; `ShowStress`, `ShowDisplacement`, and `ShowVonMises` now render boundary-surface result meshes with a 7-band blue→green→red colormap. Stress and Von Mises share the current per-element Von Mises scalar until a tensor-field UI is added.

### Phase F — TECHDRAW WORKBENCH (Tier 3 HARD, 24 features)

Working tree status: page management (`NewPage`, `FromTemplate`, `Redraw`), advanced view generation (`SectionView`, `DetailView`, `BrokenView`), drawing dimensions (`DimLinear`, `DimRadius`, `DimDiameter`, `DimAngle`, `DimArcLen`, `DimArea`), drawing annotations (`Text`, `RichText`, `Balloon`, `Leader`, `Weld`, `SurfFinish`), centerlines (`CenterFace`, `CenterLines`, `CenterPoints`, `BoltCircle`), and DXF/PDF export are implemented and tested. Remaining TechDraw scope is UX polish for parameter entry and export parity, not log-only dispatcher gaps.

### Phase G — `Pd::ShapeBinder` (Tier 3 HARD, 1 feature)

ShapeBinder is a FreeCAD concept (a body-binder that re-uses external shape inside a Body container). Working tree status: implemented via `features::shape_binder` and wired to `Pd::ShapeBinder` for selected-shape face binding.

### Phase totals

| Phase | Tier | Feature count | Actual landed | Status |
|---|---|---:|---:|---|
| A | Critical | 10 | 10 | Landed `abbfbda` |
| B | EASY wiring (Draft) | 14 | 14 | Landed `75d7705` |
| B-cont | EASY wiring (Part) | 13 | 13 | Landed `4e16eba` |
| C1 | MEDIUM UX (Draft transforms + EASY stragglers) | 7 | 7 | Landed `c2d3006` |
| C2 | MEDIUM UX (Draft modify + ProjectCurvesOnSurface) | 5 | 5 | Landed `0808d9a` |
| C3 | MEDIUM UX (Surface + PartDesign Loft/Pipe) | 7 | 7 | Landed `94396bb` |
| D | Annotation rendering | 3 | 3 | Working tree verified |
| E | FEM solver + colormap | 7 | 7 | Working tree verified |
| F | TechDraw | 24 | 24 | Page/export/view/dimension/annotation/centerline actions verified |
| G | ShapeBinder | 1 | 1 | Working tree verified |
| **Total** | | **91** | **56 landed + 25 working tree** | |

**Reconciliation note (2026-05-04).** The original plan estimated A(10) + B(18) + C(12) = 40 features and 73 total. The actual execution split Phase B and C into sub-phases for reviewability and found 3 more EASY stragglers (AutoDefeaturing, TransformedCopy in Part; FemAction::Summary/Report) that were undercounted in the original grep-based audit because they format-print params rather than log_info. The TechDraw enum audit now tracks all 24 page/view/dimension/annotation/centerline/export actions. Final landed EASY+MEDIUM count is **56/88** (accounting for Section 3.7 roll-up). HARD-tier phase tracking is D=3, E=7, F=24, G=1.

Phases A-C3 wired **56 features in ~35 hours** and closed the entire EASY+MEDIUM tier on Draft / Part / Surface / PartDesign. The 2026-05-04 working tree adds the first HARD-tier batches: annotation overlay, FEM thermal/nonlinear solvers, FEM result colormaps, TechDraw page/export/views/dimensions/annotations/centerlines, and ShapeBinder. The TechDraw log-only backlog is now closed; the next user-visible lane is command UX and parameter-entry polish.

## 5. Quality Gates (per phase)

Every phase must pass before moving to the next:

1. `cargo build --workspace` — clean.
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings` — clean.
3. `cargo test --workspace --no-fail-fast` — at minimum the prior baseline (currently 2,844). Each Tier 1/2 feature should add 1+ regression test using the `test_support` harness.
4. **Manual acceptance test** — at minimum one critical feature per phase tested end-to-end in the GUI. Result documented in commit message.
5. `CHANGELOG.md` (English canonical) updated. `docs/CHANGELOG.ko.md` summary entry per the bilingual policy.
6. This roadmap (`docs/UI_COMPLETION_ROADMAP.md`) progress section updated.

## 6. Team Routing

Per CLAUDE.md Section 11, Phase A-C work is shared between:

- **ui-engineer** (`crates/viewer/`) — dispatcher arm wiring, modals, selection workflows.
- **kernel-engineer** (`crates/modeling/`, `crates/geometry/`, `crates/sketch/`) — only invoked when a kernel API is missing or needs extension. Phase A-C should be ui-engineer dominant.
- **qa-engineer** — regression tests + documentation update per phase.
- **io-engineer** — Phase F (TechDraw) when it starts; otherwise parked. Phase 2 corpus task is parked indefinitely.

Phase D-G work involves more cross-team coordination; routing decided when those phases start.

## 7. Progress Tracker

Updated as phases land.

| Phase | Status | Commit range | Date |
|---|---|---|---|
| A — Critical CAD | Landed | abbfbda | 2026-04-29 |
| B — EASY wiring (Draft) | Landed | 75d7705 | 2026-05-01 |
| B-cont — EASY wiring (Part) | Landed | 4e16eba | 2026-05-01 |
| C1 — Draft transforms + EASY stragglers | Landed | c2d3006 | 2026-05-01 |
| C2 — Draft modify + ProjectCurvesOnSurface | Landed | 0808d9a | 2026-05-01 |
| C3 — Surface ops + PartDesign Loft/Pipe | Landed | 94396bb | 2026-05-01 |
| D — Annotation overlay (foundation + Dimension/Label) | Working tree verified (`gui::scene_overlay` module + 15 wire-output features now render + D::Dimension / D::Label) | — | 2026-05-04 |
| E-solver — FEM thermal/nonlinear | Working tree verified (`SolveThermal` / `SolveNonlinear`) | — | 2026-05-04 |
| E-render — FEM colormaps | Working tree verified (`ShowStress` / `ShowDisplacement` / `ShowVonMises`) | — | 2026-05-04 |
| F-page — TechDraw page management | Working tree verified (T::NewPage / FromTemplate / Redraw) | — | 2026-05-04 |
| F-export — TechDraw DXF/PDF | Working tree verified (new IO exporters + dispatcher wiring) | — | 2026-05-04 |
| F-view — TechDraw Section/Detail/Broken views | Working tree verified (`SectionView` / `DetailView` / `BrokenView`) | — | 2026-05-04 |
| F-dim — TechDraw dimensions | Working tree verified (`DimLinear` / `DimRadius` / `DimDiameter` / `DimAngle` / `DimArcLen` / `DimArea`) | — | 2026-05-04 |
| F-anno — TechDraw annotations | Working tree verified (`Text` / `RichText` / `Balloon` / `Leader` / `Weld` / `SurfFinish`) | — | 2026-05-04 |
| F-rest — TechDraw centerlines | Working tree verified (`CenterFace` / `CenterLines` / `CenterPoints` / `BoltCircle`) | — | 2026-05-04 |
| G — ShapeBinder | Working tree verified | — | 2026-05-04 |
| H-page — TechDraw page setup command UX | Working tree verified (`OpenPageSetup` / `CommitPageSetup`, template/title/page-size dialog) | — | 2026-05-05 |
| H-dim — TechDraw dimension setup command UX | Working tree verified (`OpenDimensionSetup` / `CommitDimensionSetup`, editable dimension dialog) | — | 2026-05-05 |
| H-anno — TechDraw annotation setup command UX | Working tree verified (`OpenAnnotationSetup` / `CommitAnnotationSetup`, editable annotation dialog) | — | 2026-05-05 |
| H-center — TechDraw centerline setup command UX | Working tree verified (`OpenCenterlineSetup` / `CommitCenterlineSetup`, editable centerline dialog) | — | 2026-05-05 |
| H-view — TechDraw view placement setup command UX | Working tree verified (`OpenViewSetup` / `CommitViewSetup`, editable placement/scale/spacing dialog + SVG manual placement) | — | 2026-05-05 |
| I-fem-results — FEM result interpretation UX | Working tree verified (legend overlay + result probe + result table dialogs) | — | 2026-05-05 |
| J-fem-bc — FEM multi-node boundary-condition UX | Working tree verified (`SectionPrint`, `TieConstraint`, `RigidBody`, `ContactConstraint` editor support + toolbar BC editor routing) | — | 2026-05-05 |
| K-sketch-profile — Sketcher profile validation UX | Working tree verified (`analyze_profiles` / `extract_profile_checked`, Profile-ready banner, open-profile Pad guard, construction-line-aware profile extraction) | — | 2026-05-05 |
| K-sketch-constraints — Sketcher constraint diagnostics UX | Working tree verified (duplicate constraints, conflicting dimensional values, invalid dimensional values, banner/status-bar diagnostics) | — | 2026-05-05 |
| K-sketch-refs — Sketcher external reference and reuse UX | Working tree verified (selected-object external projection, construction reference edges/points, `Refs:` / `Reuse:` banner and status labels, carbon-copy reuse counts) | — | 2026-05-05 |

**Current milestone (2026-05-05 working tree):** EASY + MEDIUM tiers complete, plus HARD-tier overlay/FEM/TechDraw export/views/dimensions/annotations/centerlines/ShapeBinder batches, the first five command-UX TechDraw slices (Page Setup + Dimension Setup + Annotation Setup + Centerline Setup + View Placement Setup), FEM result interpretation UX, FEM multi-node BC editor UX, Sketcher single-profile validation UX, Sketcher constraint diagnostics UX, and Sketcher external reference/reuse UX verified at **2,844 / 0 / 0**. TechDraw's visible log-only backlog is closed, parameter-entry UX now covers page/dimension/annotation/centerline/view-placement commands, FEM post-processing has legend/probe/table interpretation tools, all kernel-side FEM BC variants are editor-reachable, sketch-driven features now reject open chains before kernel extrusion, Sketcher reports duplicate/conflicting/invalid constraints before feature/solver workflows proceed, and external projections/reused sketches now show visible `Refs:` / `Reuse:` state while adding construction references from selected objects.

## 8. Long-Term Sequential Completion Plan

The project should not jump between isolated stubs. From this point onward, completion proceeds in narrow, verified slices that each end with tests, bilingual documentation, and `WORK_STATUS.md` updates.

| Order | Lane | Goal | Exit Criteria |
|---:|---|---|---|
| 0 | Stabilize verified worktree | Preserve and commit the current verified HARD-tier work in surgical chunks. | Build/clippy/test clean; no verified uncommitted work lost. |
| 1 | UI completion — TechDraw | Finish the last high-visibility UI gaps: dimensions, annotations, centerlines, bolt circles, and drawing overlay/export parity. | TechDraw clicks produce sheet changes and SVG/DXF/PDF output, not log-only messages. |
| 2 | UI completion — command UX | Replace remaining placeholder defaults with task panels/modals, selection prompts, previews, and undoable transactions. | Main workbench actions are discoverable and parameter-editable. |
| 3 | Sketcher production workflow | Improve profile validation, constraint diagnostics, construction geometry, external references, and sketch reuse. | Sketch → feature flows are robust enough for multi-feature parts. |
| 4 | PartDesign history/body model | Add editable feature history, Body-local dependencies, recompute ordering, and persistent naming repair. | A modeled part can be edited parametrically without rebuilding from scratch. |
| 5 | Assembly workflow | Polish mates/joints, exploded views, interference review, BOM export, product tree operations, and large assembly navigation. | Small product assemblies can be constrained, inspected, and documented. |
| 6 | FEM workflow | Add node/face set picking, mesh controls, legends, probes, result tables, and richer post-processing. | Users can set up, solve, and interpret a simple mechanical/thermal study from UI. |
| 7 | I/O interoperability | Validate STEP/IGES/DXF/SVG/PDF against real-world corpora, preserve units/layers/metadata, and add import healing. | External CAD exchange is regression-tested with representative files. |
| 8 | Performance and large-model UX | Add async jobs, progress/cancel, GPU/wire pipelines, cache invalidation, and 1000+ part scene performance gates. | Large drawings/assemblies stay responsive. |
| 9 | Release readiness | Package binaries, Python wheels, docs/tutorials, crash-safe settings, and CI release gates. | A non-developer can install, run, and follow tutorials end-to-end. |

**Active lane:** Order 2 command UX is complete through the TechDraw Page/Dimension/Annotation/Centerline/View Placement slices, Order 6 FEM now has result interpretation plus range-based multi-node BC entry, and Order 3 Sketcher production workflow now has profile-readiness, actionable constraint diagnostics, and visible external-reference/sketch-reuse feedback. Next Sketcher slices should deepen external-reference management and sketch reuse editing; FEM node/face viewport picking remains the next FEM-specific polish lane.

## 9. Reference

- Stub source-of-truth: `crates/viewer/src/app.rs` (search for `=> self.log_info("`)
- Kernel API surface: `crates/modeling/src/features/` and `crates/modeling/src/draft_ops.rs`
- Test harness: `crates/viewer/src/lib.rs` `test_support` module + `tests/gui_action_integration.rs`
- Prior session work: commits `16192ad` (ActiveDialog) through `1875230` (corpus Phase 1)
