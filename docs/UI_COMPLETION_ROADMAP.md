# UI Completion Roadmap

**Status:** Active. Started 2026-04-30.
**Bilingual:** [한국어 요약](UI_COMPLETION_ROADMAP.ko.md)

## 1. Why This Document Exists

Prior status reports claimed "576/576 features dispatcher-reachable." This was technically true but materially misleading: ~79 of those dispatcher arms in `crates/viewer/src/app.rs` are pure `self.log_info("...")` stubs that route a UI click to a single log line and produce **no geometry, no scene change, no user-visible effect.** The user clicked PartDesign Pad and got a log message, not a solid.

The recent V37 session work (module split, sub-enum partition, panic-safety fixes, test corpus) was engineering hygiene — necessary, but it did not add a single working feature. The user's frustration ("간단한 도형 늘리기도 안되는 판에 이게 어떻게 cad라고 할 수 있겠어") is correct: **basic CAD operations including sketch-driven Pad / Pocket / Hole are non-functional today**.

This roadmap is a structured plan to actually wire the UI to the kernel. It is the canonical reference for the multi-session UI completion effort.

## 2. The Surprise (Good News)

**Most of the work has already been done at the kernel level.** A targeted audit found that for ~50-60 of the 79 stubs, the kernel API exists in `crates/modeling/` and just needs to be called from the dispatcher arm. No new kernel code; pure wiring.

| Stub category | Kernel API status | Estimated effort |
|---|---|---|
| EASY — kernel API exists, wire only | ~55 stubs | 15-30 min each |
| MEDIUM — kernel API exists, needs UX (modal / picker / sketch ref) | ~15 stubs | 1-2 hours each |
| HARD — kernel API missing | ~9 stubs | 3-10 hours each (new kernel work) |

The 9-sub-enum dispatcher partition we landed last session (commits `9e42ece` … `d2966b6`) actually makes this fix easier — each `process_*_action` helper is a clean isolated dispatch point.

## 3. Per-Stub Inventory (79 total)

Stub line numbers refer to `crates/viewer/src/app.rs` at commit `1875230` (HEAD as of 2026-04-30). They will shift as fixes land; the workbench enum variant name is the stable identifier.

### 3.1 Draft workbench (29 stubs)

| Variant | Stub line | Kernel API | Tier | Notes |
|---|---|---|---|---|
| `D::Line` | 3904 | `draft_ops::make_line_draft` | EASY | Two-point line wire. |
| `D::Wire` | 3905 | `draft_ops::make_wire` | EASY | Polyline. |
| `D::Circle` | 3906 | `draft_ops::make_circle_wire` | EASY | Already used by viewer in another path. |
| `D::Arc` | 3907 | `draft_ops::make_arc_wire`, `make_arc_3pt_wire` | EASY | Two flavours. |
| `D::Ellipse` | 3908 | `draft_ops::make_ellipse_wire` | EASY | |
| `D::BSpline` | 3956 | `draft_ops::make_bspline_wire` | EASY | |
| `D::Bezier` | 3957 | `draft_ops::make_bezier_wire`, `make_cubic_bezier_wire` | EASY | |
| `D::Point` | 3958 | `draft_ops::make_point` | EASY | |
| `D::Facebinder` | 3959 | `draft_ops::make_facebinder` | MEDIUM | Needs face selection. |
| `D::Hatch` | 3960 | `draft_ops::draft_hatch` | EASY | |
| `D::Move` | 3961 | `draft_ops::move_solid` | MEDIUM | Needs gizmo / numeric input modal. |
| `D::Rotate` | 3962 | `draft_ops::rotate_solid` | MEDIUM | Same as Move. |
| `D::Scale` | 3963 | `draft_ops::scale_solid_draft` | MEDIUM | Same as Move. |
| `D::Mirror` | 3964 | `draft_ops::mirror_solid_draft` | MEDIUM | Needs plane picker (or default planes). |
| `D::Offset` | 3965 | `draft_ops::offset_wire` | MEDIUM | Needs distance modal + wire selection. |
| `D::Trim` | 3966 | `draft_ops::trimex_draft` | MEDIUM | Needs target-point selection. |
| `D::Stretch` | 3967 | `draft_ops::stretch_wire` | MEDIUM | Needs vertex-and-vector selection. |
| `D::Clone` | 3968 | `draft_ops::clone_solid` | EASY | |
| `D::ArrayRect` | 3969 | `draft_ops::rectangular_array` | EASY | |
| `D::ArrayPolar` | 3970 | `draft_ops::polar_array`, `circular_array` | EASY | |
| `D::ArrayPath` | 3971 | `draft_ops::path_array`, `path_link_array` | EASY | |
| `D::ArrayPoint` | 3972 | `draft_ops::point_array`, `point_link_array` | EASY | |
| `D::Dimension` | 3973 | `draft_ops::make_draft_dimension`, `make_draft_dimension_full` | MEDIUM | Needs overlay rendering. |
| `D::Label` | 3974 | `draft_ops::make_label`, `make_label_full` | MEDIUM | Same as Dimension. |
| `D::Text` | 3975 | `draft_ops::shape_from_text` | EASY | Generates extruded text geometry. |
| `D::Upgrade` | 3976 | `draft_ops::upgrade_wire`, `upgrade_wire_model` | EASY | |
| `D::Downgrade` | 3977 | `draft_ops::downgrade_solid`, `downgrade_solid_faces` | EASY | |
| `D::WireToBSpline` | 3978 | `draft_ops::wire_to_bspline_convert` | EASY | |
| `D::ToSketch` | 3979 | `draft_ops::draft_to_sketch` | EASY | |

**Subtotals:** EASY = 19, MEDIUM = 10, HARD = 0.

### 3.2 PartDesign workbench (5 stubs — most are real, 5 remaining)

| Variant | Stub line | Kernel API | Tier | Notes |
|---|---|---|---|---|
| `Pd::AdditiveLoft` | 3850 | `features::loft` | MEDIUM | Needs profile-list selection. |
| `Pd::AdditivePipe` | 3851 | `features::sweep`, `surface_ops::pipe_surface` | MEDIUM | Needs profile + path selection. |
| `Pd::SubtractiveLoft` | 3852 | `features::loft` + `boolean_op_exact` | MEDIUM | Same as AdditiveLoft + boolean. |
| `Pd::SubtractivePipe` | 3853 | `features::sweep` + `boolean_op_exact` | MEDIUM | Same as AdditivePipe + boolean. |
| `Pd::ShapeBinder` | 3867 | (none) | HARD | New kernel concept; needs design. |

**CRITICAL — separate from stubs but functionally broken**: `Pd::PadSketch`, `Pd::PocketSketch`, `Pd::GrooveSketch`, `Pd::HoleSketch`, `Pd::CountersunkHoleSketch` ALSO log_info even though `features::pad`, `pocket`, `groove`, `hole`, `countersunk_hole` all exist. These are not in the 79-stub count above because the dispatcher format-prints params (so `grep` missed them), but they're equally non-functional. Kernel API: all exist, all EASY tier wiring. **These are the user's "간단한 도형 늘리기" complaint.**

**Subtotals:** EASY = 5 (the format-printing pad-family), MEDIUM = 4, HARD = 1.

### 3.3 Part workbench (14 stubs)

| Variant | Stub line | Kernel API | Tier | Notes |
|---|---|---|---|---|
| `P::FaceFromWires` | 3809 | `features::face_from_wires` | EASY | |
| `P::ConnectShapes` | 3810 | `features::connect_shapes` | EASY | |
| `P::EmbedShapes` | 3811 | `features::embed_shapes` | EASY | |
| `P::CutoutShapes` | 3812 | `features::cutout_shapes` | EASY | |
| `P::ExplodeCompound` | 3813 | `features::compound_ops::explode_compound` | EASY | |
| `P::CompoundFilter` | 3814 | `features::compound_ops::compound_filter` | EASY | |
| `P::BooleanFragments` | 3815 | `features::compound_ops::boolean_fragments` | EASY | |
| `P::SliceToCompound` | 3816 | `features::compound_ops::slice_to_compound` | EASY | |
| `P::PointsFromShape` | 3817 | `features::face_from_wires::points_from_shape` | EASY | |
| `P::ConvertToSolid` | 3818 | `features::shape_convert::shape_from_mesh` | EASY | |
| `P::AutoDefeaturing` | already wired (logs but doesn't apply) | `features::defeature::auto_defeaturing` | EASY | Format-print stub like Pad-family. |
| `P::TransformedCopy` | already wired (logs but doesn't apply) | `multi_transform::multi_transform` | EASY | Format-print stub. |
| `P::ProjectCurvesOnSurface` | 3825 | `features::projection::project_curve_on_solid` | MEDIUM | Needs curve + surface selection. |
| `P::CoonsPatch` | 3826 | `surface_ops::coons_patch` | EASY | |

**Subtotals:** EASY = 13, MEDIUM = 1, HARD = 0.

### 3.4 Surface workbench (4 stubs — Filling/Boundary/Pipe already real)

| Variant | Stub line | Kernel API | Tier | Notes |
|---|---|---|---|---|
| `S::Sections` | 3776 | `surface_ops::sections`, `features::cross_sections` | MEDIUM | Needs profile selection. |
| `S::Extend` | 3777 | `surface_ops::extend_surface` | MEDIUM | Needs surface selection + distance. |
| `S::Blend` | 3778 | `surface_ops::surface_from_curves` (closest) | MEDIUM | API is partial. |
| `S::Coons` | 3802 | `surface_ops::coons_patch` | EASY | |

**Subtotals:** EASY = 1, MEDIUM = 3, HARD = 0.

### 3.5 FEM workbench (7 stubs)

| Variant | Stub line | Kernel API | Tier | Notes |
|---|---|---|---|---|
| `FemAction::SolveThermal` | 2826 | (none — solver only does static linear) | HARD | New kernel work. |
| `FemAction::SolveNonlinear` | 2830 | (none) | HARD | New kernel work. |
| `FemAction::ShowStress` | 2831 | (none — no per-vertex colormap pipeline) | HARD | Render pipeline work (Phase O-b). |
| `FemAction::ShowDisplacement` | 2832 | (none) | HARD | Same. |
| `FemAction::ShowVonMises` | 2833 | (none) | HARD | Same. |
| `FemAction::Summary` | 2834 | (text output trivial) | EASY | Just format `fem_analysis` fields. |
| `FemAction::Report` | 2835 | (text output trivial) | EASY | Same. |

**Subtotals:** EASY = 2, MEDIUM = 0, HARD = 5.

### 3.6 TechDraw workbench (22 stubs)

All 22 TechDraw stubs (NewPage, FromTemplate, Redraw, SectionView, DetailView, BrokenView, all Dim variants, Text, RichText, Balloon, Leader, Weld, SurfFinish, all Center variants, BoltCircle, ExportDxf, ExportPdf) are **HARD tier** because the TechDraw kernel-side support in `crates/io/src/techdraw.rs` (or equivalent) currently provides only AddView, ThreeView, ExportSvg, Clear. Section views, detail views, dimension placement, annotations, centerlines all need new kernel work.

**Subtotals:** EASY = 0, MEDIUM = 0, HARD = 22.

### 3.7 Workbench-totals roll-up

| Workbench | Stub count | EASY | MEDIUM | HARD |
|---|---:|---:|---:|---:|
| Draft | 29 | 19 | 10 | 0 |
| PartDesign (incl. format-print Pad-family) | 10 | 5 | 4 | 1 |
| Part (incl. format-print stubs) | 14 | 13 | 1 | 0 |
| Surface | 4 | 1 | 3 | 0 |
| FEM | 7 | 2 | 0 | 5 |
| TechDraw | 22 | 0 | 0 | 22 |
| **Total** | **86** | **40** | **18** | **28** |

(86 > 79 because the Pad-family + AutoDefeaturing + TransformedCopy format-print stubs were undercounted by the initial `log_info`-only grep but are the same kind of broken.)

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

**Estimated effort**: 8-12 hours (overlay rendering work).

### Phase E — FEM SOLVER + COLORMAP VISUALIZATION (Tier 3 HARD, 5 features + 2 EASY)

- `Summary`, `Report` — EASY (text formatting)
- `ShowStress`, `ShowDisplacement`, `ShowVonMises` — render pipeline work (Phase O-b from prior plan)
- `SolveThermal`, `SolveNonlinear` — kernel solver extension

**Estimated effort**: 30-50 hours. Multi-session.

### Phase F — TECHDRAW WORKBENCH (Tier 3 HARD, 22 features)

The entire TechDraw workbench except the 4 already-real entries (AddView, ThreeView, ExportSvg, Clear) needs kernel work in `crates/io/src/techdraw*.rs` plus dispatcher wiring plus overlay rendering.

**Estimated effort**: 60-100 hours. Multi-session, kernel-engineer + ui-engineer collaboration.

### Phase G — `Pd::ShapeBinder` (Tier 3 HARD, 1 feature)

ShapeBinder is a FreeCAD concept (a body-binder that re-uses external shape inside a Body container). Needs new kernel concept design.

**Estimated effort**: 5-10 hours research + design + implementation.

### Phase totals

| Phase | Tier | Feature count | Estimated effort |
|---|---|---:|---:|
| A | Critical | 10 | 4-6 h |
| B | EASY wiring | 18 | 8-12 h |
| C | MEDIUM UX | 12 | 15-20 h |
| D | Annotation rendering | 3 | 8-12 h |
| E | FEM | 7 | 30-50 h |
| F | TechDraw | 22 | 60-100 h |
| G | ShapeBinder | 1 | 5-10 h |
| **Total** | | **73** | **130-210 h** |

Phases A + B + C are **40 features in 27-38 hours** and would close out the entire EASY+MEDIUM tier on Draft / Part / Surface / PartDesign — covering the practical "make basic CAD work" goal. Phases D / E / F / G are large multi-session efforts that we'd schedule after the user agrees Phases A-C have made the project usable.

## 5. Quality Gates (per phase)

Every phase must pass before moving to the next:

1. `cargo build --workspace` — clean.
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings` — clean.
3. `cargo test --workspace --no-fail-fast` — at minimum the prior baseline (currently 2,662). Each Tier 1/2 feature should add 1+ regression test using the `test_support` harness.
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
| A — Critical CAD | Not started | — | — |
| B — EASY wiring | Not started | — | — |
| C — MEDIUM UX | Not started | — | — |
| D — Annotation | Not started | — | — |
| E — FEM | Not started | — | — |
| F — TechDraw | Not started | — | — |
| G — ShapeBinder | Not started | — | — |

## 8. Reference

- Stub source-of-truth: `crates/viewer/src/app.rs` (search for `=> self.log_info("`)
- Kernel API surface: `crates/modeling/src/features/` and `crates/modeling/src/draft_ops.rs`
- Test harness: `crates/viewer/src/lib.rs` `test_support` module + `tests/gui_action_integration.rs`
- Prior session work: commits `16192ad` (ActiveDialog) through `1875230` (corpus Phase 1)
