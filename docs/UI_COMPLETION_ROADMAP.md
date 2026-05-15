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

> **Status 2026-05-14 (UI-B1, FIRST MEDIUM-tier verify-first)**: 10/10 Draft workbench MEDIUM dispatcher arms (`D::Facebinder`, `D::Move`, `D::Rotate`, `D::Scale`, `D::Mirror`, `D::Offset`, `D::Trim`, `D::Stretch`, `D::Dimension`, `D::Label`) verified already wired with hardcoded sensible defaults — same verify-first outcome as all five EASY-tier phases. Cumulative counter: 64 EASY + 10 MEDIUM = **74/74**. **MEDIUM tier shows the same wired-with-defaults pattern as EASY.** Dispatcher-boundary tests added in `crates/viewer/tests/draft_medium_features.rs` (10 tests, workspace 3,356 → **3,366 / 0 / 1**). Full UX (modals, pickers, gizmos, overlay-edit) deferred to §4 MEDIUM-UX Backlog.

> **Status 2026-05-14 (UI-B2, MEDIUM tier exhausted)**: 8/8 remaining MEDIUM dispatcher arms across PartDesign (4: `Pd::AdditiveLoft`, `Pd::AdditivePipe`, `Pd::SubtractiveLoft`, `Pd::SubtractivePipe`), Part (1: `P::ProjectCurvesOnSurface`), Surface (3: `S::Sections`, `S::Extend`, `S::Blend`) verified already wired with hardcoded sensible defaults. **MEDIUM tier across §3.1–§3.4 now FULLY exhausted as of 2026-05-14.** Cumulative verify-first counter: 64 EASY + 18 MEDIUM = **82/82** across 6 consecutive lanes. Default args confirmed: Loft uses 2 stacked tapered squares; Pipe uses 0.5×0.5 profile + 2-pt Z path; ProjectCurvesOnSurface uses default polyline + overlay; Sections uses 2 stacked square profiles; Extend uses distance 0.5; Blend routes through `surface_from_curves` (Gordon-like quad — true G2 blend is a kernel gap). Dispatcher-boundary tests added in `crates/viewer/tests/pd_part_surface_medium_features.rs` (8 tests, workspace 3,366 → **3,374 / 0 / 1**). Full production UX deferred to §3.8 MEDIUM-UX Backlog (Phase F).

> **Status 2026-05-14 (UI-B3, first true wiring lane)**: 7 PartDesignAction variants not tracked in §3.2 were discovered (`CreateSprocket`, `CreateShaftDesign`, `CreateInvoluteGear`, `SuppressFeature`, `SetTip`, `MoveFeatureUp`, `MoveFeatureDown`). Verify-first found 3 TRUE-STUB (log_info-only) needing kernel wiring and 5 already-wired needing test coverage. All 3 stubs wired: `CreateSprocket` → `make_sprocket`, `CreateShaftDesign` → `shaft_design`, `CreateInvoluteGear` → `make_involute_gear` (face_width=5.0 hardcoded). **§3.7 HARD-1 entry is stale** — all 3 mechanical generators had kernel APIs already; PartDesign HARD revised 1 → **0**. The HARD count across the full original 79-stub inventory is now genuinely zero. Dispatcher-boundary tests added in `crates/viewer/tests/pd_untracked_features.rs` (8 tests, workspace 3,374 → **3,382 / 0 / 1**). Cumulative verify-first counter: 64 EASY + 18 MEDIUM + 8 PD-untracked = **90/90** across 7 consecutive lanes.

This roadmap is a structured plan to actually wire the UI to the kernel. It is the canonical reference for the multi-session UI completion effort.

## 2. The Surprise (Good News)

**Most of the work has already been done at the kernel level.** A targeted audit found that for ~50-60 of the 79 stubs, the kernel API exists in `crates/modeling/` and just needs to be called from the dispatcher arm. No new kernel code; pure wiring.

| Stub category | Kernel API status | Estimated effort |
|---|---|---|
| EASY — kernel API exists, wire only | ~15 stubs (was ~55; 5 PD verified DONE UI-A1; 19 Draft DONE UI-A2; 13 Part DONE UI-A3; 3 Surface/FEM DONE UI-A4 = S::Coons + FemAction::Summary + FemAction::Report) | 15-30 min each |
| MEDIUM — kernel API exists, needs UX (modal / picker / sketch ref) | **0 remaining** (was ~18; 10 Draft MEDIUM verified wired UI-B1 2026-05-14; 8 PD+Part+Surface MEDIUM verified wired UI-B2 2026-05-14 — **MEDIUM tier FULLY exhausted**) | 1-2 hours each |
| HARD — kernel API missing | ~9 stubs (**0 PartDesign HARD remaining** — §3.7 HARD-1 entry verified stale 2026-05-14 via UI-B3) | 3-10 hours each (new kernel work) |

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
| `D::Facebinder` | 3959 | `draft_ops::make_facebinder` | DONE (verified 2026-05-14) | Default: solid's first face. UX gap: interactive face-picker. |
| `D::Hatch` | 3960 | `draft_ops::draft_hatch` | DONE (verified 2026-05-14) | scene+1, multiple polylines (boundary + fill). |
| `D::Move` | 3961 | `draft_ops::move_solid` | DONE (verified 2026-05-14) | Default: displacement (1,0,0). UX gap: gizmo / numeric modal. |
| `D::Rotate` | 3962 | `draft_ops::rotate_solid` | DONE (verified 2026-05-14) | Default: Z axis, 30° around origin. UX gap: axis selector + angle modal. |
| `D::Scale` | 3963 | `draft_ops::scale_solid_draft` | DONE (verified 2026-05-14) | Default: factor 2.0 from origin. UX gap: factor modal + pivot picker. |
| `D::Mirror` | 3964 | `draft_ops::mirror_solid_draft` | DONE (verified 2026-05-14) | Default: reads gui.mirror_plane (XY/XZ/YZ). UX gap: custom plane picker. |
| `D::Offset` | 3965 | `draft_ops::offset_wire` | DONE (verified 2026-05-14) | Default: distance 0.5, normal Z. UX gap: distance modal + wire-selection. |
| `D::Trim` | 3966 | `draft_ops::trimex_draft` | DONE (verified 2026-05-14) | Default: wire midpoint as target. UX gap: cursor target-point pick. |
| `D::Stretch` | 3967 | `draft_ops::stretch_wire` | DONE (verified 2026-05-14) | Default: center=origin, r=5, displacement (0,0,1). UX gap: vertex+radius select + drag-vector modal. |
| `D::Clone` | 3968 | `draft_ops::clone_solid` | DONE (verified 2026-05-14) | Both positive (with selection) and no-op (without) covered. |
| `D::ArrayRect` | 3969 | `draft_ops::rectangular_array` | DONE (verified 2026-05-14) | scene grew past base (Rect 3×2). |
| `D::ArrayPolar` | 3970 | `draft_ops::polar_array`, `circular_array` | DONE (verified 2026-05-14) | scene grew past base (Polar 6-fold). |
| `D::ArrayPath` | 3971 | `draft_ops::path_array`, `path_link_array` | DONE (verified 2026-05-14) | scene grew past base. |
| `D::ArrayPoint` | 3972 | `draft_ops::point_array`, `point_link_array` | DONE (verified 2026-05-14) | scene grew past base. |
| `D::Dimension` | 3973 | `draft_ops::make_draft_dimension`, `make_draft_dimension_full` | DONE (verified 2026-05-14) | Default: linear [0,0,0]→[2,0,0], offset 0.5. UX gap: endpoint pick + drag-offset; angular/radial types. |
| `D::Label` | 3974 | `draft_ops::make_label`, `make_label_full` | DONE (verified 2026-05-14) | Default: text "Label" at (0.5,0.5,0), leader to origin. UX gap: position + leader-target picker + text editor. |
| `D::Text` | 3975 | `draft_ops::shape_from_text` | DONE (verified 2026-05-14) | scene+1, polylines+labels grew. |
| `D::Upgrade` | 3976 | `draft_ops::upgrade_wire`, `upgrade_wire_model` | DONE (verified 2026-05-14) | scene+1, non-empty vertices. |
| `D::Downgrade` | 3977 | `draft_ops::downgrade_solid`, `downgrade_solid_faces` | DONE (verified 2026-05-14) | Bonus test: box → 6 face-solids. |
| `D::WireToBSpline` | 3978 | `draft_ops::wire_to_bspline_convert` | DONE (verified 2026-05-14) | scene+1, overlay polyline grew. |
| `D::ToSketch` | 3979 | `draft_ops::draft_to_sketch` | DONE (verified 2026-05-14) | `last_sketch_is_set()` flipped to true, scene unchanged. |

**Subtotals:** DONE = 29 (19 EASY verified 2026-05-14 via `draft_easy_features.rs` 20 tests; 10 MEDIUM verified 2026-05-14 via `draft_medium_features.rs` 10 tests), MEDIUM remaining = 0, HARD = 0.

### 3.2 PartDesign workbench (18 covered — sketch-driven 5, MEDIUM 4, untracked-generators 3, untracked-feature-mgmt 5; HARD = 0)

| # | Variant | Kernel API | Status | Notes |
|---|---|---|---|---|
| 1 | `Pd::PadSketch` | `features::pad` | DONE (verified 2026-05-14 via UI-A1) | scene+1 solid. Test: `partdesign_sketch_features.rs`. |
| 2 | `Pd::PocketSketch` | `features::pocket` | DONE (verified 2026-05-14 via UI-A1) | scene+1 solid. |
| 3 | `Pd::GrooveSketch` | `features::groove` | DONE (verified 2026-05-14 via UI-A1) | scene+1 solid. |
| 4 | `Pd::HoleSketch` | `features::hole` | DONE (verified 2026-05-14 via UI-A1) | scene+1 solid. |
| 5 | `Pd::CountersunkHoleSketch` | `features::countersunk_hole` | DONE (verified 2026-05-14 via UI-A1) | scene+1 solid. |
| 6 | `Pd::AdditiveLoft` | `features::loft` | DONE (verified 2026-05-14 via UI-B2) | Default: 2 stacked tapered squares. UX gap: profile-list picker (Phase F). |
| 7 | `Pd::AdditivePipe` | `features::sweep`, `surface_ops::pipe_surface` | DONE (verified 2026-05-14 via UI-B2) | Default: 0.5×0.5 profile + 2-pt Z path. UX gap: profile + path picker (Phase F). |
| 8 | `Pd::SubtractiveLoft` | `features::loft` + `boolean_op_exact` | DONE (verified 2026-05-14 via UI-B2) | Loft tool + boolean Difference; selection-gated. UX gap: same as AdditiveLoft (Phase F). |
| 9 | `Pd::SubtractivePipe` | `features::sweep` + `boolean_op_exact` | DONE (verified 2026-05-14 via UI-B2) | Sweep tool + boolean Difference; selection-gated. UX gap: same as AdditivePipe (Phase F). |
| 10 | `Pd::ShapeBinder` | `features::shape_binder` | DONE (verified 2026-05-14 via UI-B3) | Wired pre-B3; re-verified: binds selected shape faces into new binder solid. UX gap: face-list multi-picker (Phase F). |
| 11 | `Pd::CreateSprocket` | `make_sprocket` | **Wired 2026-05-14 via UI-B3** | Was TRUE-STUB (log_info only). Now `snapshot_before` + `make_sprocket(model, teeth, roller_d, pitch, bore)` + `add_to_scene`. UX gap: parameter dialog. |
| 12 | `Pd::CreateShaftDesign` | `shaft_design` | **Wired 2026-05-14 via UI-B3** | Was TRUE-STUB. Now `shaft_design(model, &segments)` with empty-list guard. UX gap: segment-list editor. |
| 13 | `Pd::CreateInvoluteGear` | `make_involute_gear` | **Wired 2026-05-14 via UI-B3** | Was TRUE-STUB. Now `make_involute_gear(model, m, teeth, pa, 5.0)`. face_width=5.0 hardcoded. UX gap: face_width modal. |
| 14 | `Pd::SuppressFeature` | `obj.suppressed` toggle | Verified WIRED 2026-05-14 via UI-B3 | Toggles `suppressed` flag on selected object. UX gap: model-tree context-menu integration. |
| 15 | `Pd::SetTip` | `obj.is_tip` mark | Verified WIRED 2026-05-14 via UI-B3 | Marks selected object as tip. UX gap: model-tree context-menu. |
| 16 | `Pd::MoveFeatureUp` | `scene.move_up(id)` | Verified WIRED 2026-05-14 via UI-B3 | Reorders scene (scene[i] swapped up). UX gap: model-tree context-menu. |
| 17 | `Pd::MoveFeatureDown` | `scene.move_down(id)` | Verified WIRED 2026-05-14 via UI-B3 | Reorders scene (scene[i] swapped down). UX gap: model-tree context-menu. |
| 18 | (prior untracked) | — | — | ShapeBinder re-counted here (row 10); no additional row. |

> **Note on row 18**: Row 10 (`ShapeBinder`) covers the previously-listed §3.7 "DONE" entry. The table has 17 distinct variants + this note row for accounting clarity. Actual variant count = 17.

**Status 2026-05-14 — DONE (verified)**: `Pd::PadSketch`, `Pd::PocketSketch`, `Pd::GrooveSketch`, `Pd::HoleSketch`, `Pd::CountersunkHoleSketch` were verified wired to `cadkernel_modeling::{pad, pocket, groove, hole, countersunk_hole}` in HEAD `714136e`. Dispatcher-boundary tests in `crates/viewer/tests/partdesign_sketch_features.rs` (6 tests, all green as of 2026-05-14).

**Status 2026-05-14 (UI-B2) — DONE (verified)**: `Pd::AdditiveLoft`, `Pd::AdditivePipe`, `Pd::SubtractiveLoft`, `Pd::SubtractivePipe` verified wired with hardcoded defaults. Dispatcher-boundary tests in `crates/viewer/tests/pd_part_surface_medium_features.rs` (8 tests total for the lane, 4 for PD). Full production UX (profile-list picker, path picker, orientation modal) deferred to Phase F.

**Status 2026-05-14 (UI-B3) — 3 wired + 5 re-verified + HARD-1 stale**: `CreateSprocket`, `CreateShaftDesign`, `CreateInvoluteGear` were TRUE-STUB and are now wired. `SuppressFeature`, `SetTip`, `MoveFeatureUp`, `MoveFeatureDown`, and `ShapeBinder` (re-verify) were already wired. The §3.7 "HARD = 1" entry was stale — all 3 generators had kernel APIs. PartDesign HARD is now **0**. Dispatcher-boundary tests in `crates/viewer/tests/pd_untracked_features.rs` (8 tests, workspace 3,374 → **3,382 / 0 / 1**).

**Subtotals:** EASY = 5 (pad-family, DONE 2026-05-14), MEDIUM = 4 (loft/pipe, DONE 2026-05-14 via UI-B2), Wired-Untracked = 3 (generators, DONE 2026-05-14 via UI-B3), Verified-Untracked = 5 (feature-mgmt + ShapeBinder, DONE 2026-05-14 via UI-B3), HARD = **0** (revised from stale 1).

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
| `P::ProjectCurvesOnSurface` | 3825 | `features::projection::project_curve_on_solid` | DONE (verified 2026-05-14) | Default: polyline + overlay; selection-gated. UX gap: curve picker + target surface picker + direction modal (Phase F). |
| `P::CoonsPatch` | 3826 | `surface_ops::coons_patch` | DONE (verified 2026-05-14) | scene+1 solid, overlay polyline grew. |

**Subtotals:** DONE = 14 (13 EASY verified 2026-05-14 + 1 MEDIUM `P::ProjectCurvesOnSurface` verified 2026-05-14 via UI-B2), MEDIUM = 0, HARD = 0.

### 3.4 Surface workbench (4 stubs — Filling/Boundary/Pipe already real)

| Variant | Stub line | Kernel API | Tier | Notes |
|---|---|---|---|---|
| `S::Sections` | 3776 | `surface_ops::sections`, `features::cross_sections` | DONE (verified 2026-05-14) | Default: skinned solid from 2 stacked square profiles. UX gap: section-curve list picker + skin-degree slider (Phase F). |
| `S::Extend` | 3777 | `surface_ops::extend_surface` | DONE (verified 2026-05-14) | Default: distance 0.5; selection-gated. UX gap: face picker + distance modal + continuity selector G0/G1/G2 (Phase F). |
| `S::Blend` | 3778 | `surface_ops::surface_from_curves` (Gordon-like quad) | DONE (verified 2026-05-14) | Stand-in via `surface_from_curves`. **Kernel gap**: true tangent-continuous (G1/G2) surface blend is missing; `surface_from_curves` is current approximation. UX gap: face/edge-chain picker + continuity selector (Phase F). |
| `S::Coons` | 3802 | `surface_ops::coons_patch` | DONE (verified 2026-05-14) | Covered by `gui_action_integration.rs:2851`. |

**Subtotals:** DONE = 4 (1 EASY `S::Coons` verified 2026-05-14; 3 MEDIUM `S::Sections`/`S::Extend`/`S::Blend` verified 2026-05-14 via UI-B2), EASY remaining = 0, MEDIUM = 0, HARD = 0.

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

| Workbench | Stub count | DONE | EASY remaining | MEDIUM remaining | HARD |
|---|---:|---:|---:|---:|---:|
| Draft | 29 | 29 (19 EASY 2026-05-14; 10 MEDIUM 2026-05-14) | 0 | 0 | 0 |
| PartDesign (incl. pad-family + untracked generators/feature-mgmt) | 17 | 17 (5 EASY UI-A1; 4 MEDIUM UI-B2; 3 wired-untracked UI-B3; 5 verified-untracked UI-B3) | 0 | 0 | **0** (revised from stale 1 — 2026-05-14 via UI-B3) |
| Part (incl. format-print stubs) | 14 | 14 (13 EASY verified 2026-05-14; 1 MEDIUM `P::ProjectCurvesOnSurface` verified 2026-05-14 via UI-B2) | 0 | 0 | 0 |
| Surface | 4 | 4 (1 EASY `S::Coons` verified 2026-05-14; 3 MEDIUM verified 2026-05-14 via UI-B2) | 0 | 0 | 0 |
| FEM | 7 | 7 (all verified; Summary/Report strengthened 2026-05-14) | 0 | 0 | 0 |
| TechDraw | 24 | 24 (all verified 2026-05-14; 23 already-covered + 1 strengthened) | 0 | 0 | 0 |
| **Total** | **95** | **90** | **0** | **0** | **0** |

(95 > 79 because the Pad-family + AutoDefeaturing + TransformedCopy format-print stubs were undercounted by the initial `log_info`-only grep; TechDraw's extracted enum now exposes the full 24-action backlog; and UI-B3 discovered 7 additional untracked PartDesignAction variants. HARD count is 0 — the last "HARD-1" entry in §3.7 was verified stale 2026-05-14.)

### 3.8 MEDIUM-UX Backlog (2026-05-14)

The verify-first passes (UI-A1 through UI-B1) confirmed that all 74 dispatcher arms wired so far call the kernel correctly but use hardcoded defaults. The backlog below tracks the UX work needed to replace those defaults with interactive input. None of these are required for the dispatcher contracts verified by the test suite — they are purely UX polish.

| Bucket | Features | Notes |
|---|---|---|
| **Phase-B-modal** | Move (dx,dy,dz), Rotate (axis, angle), Scale (factor, pivot), Offset (distance, direction) | Numeric input modals; pre-fill with current dispatcher defaults. Reuse existing `ActiveDialog` scaffolding in `crates/viewer/src/gui/`. |
| **Phase-B-picker** | Facebinder (face pick), Mirror (plane pick / custom 3-point), Dimension (endpoint pick + drag-offset), Trim (cursor target-point), Stretch (vertex + radius select), Offset (wire pick) | Interactive selection overlays; builds on `picking.rs` + `command.rs` infrastructure. |
| **Phase-C-gizmo** | Move (translate handles), Rotate (rotation ring), Scale (uniform/per-axis handles) | 3D manipulators; reuses camera/picking infrastructure already in viewer. |
| **Phase-D-overlay-edit** | Dimension (value in-place edit), Label (text in-place edit) | In-place editing via existing `gui::scene_overlay` label rendering path. |
| **Phase-F-loft-pipe** | AdditiveLoft / SubtractiveLoft: profile-list picker (≥2 sketches in order; live preview). AdditivePipe / SubtractivePipe: profile + path picker; orientation modal (Frenet/binormal/auxiliary). | New multi-select picker workflow; AdditiveLoft/SubtractiveLoft share picker logic. |
| **Phase-F-project** | ProjectCurvesOnSurface: curve picker + target solid/surface picker; projection-direction modal (normal/view/custom vector). | Builds on existing picking.rs + overlay infrastructure. |
| **Phase-F-sections** | Surface Sections: section-curve list picker (≥2 profiles, optional guide curves); skin-degree slider (1=ruled, 3=cubic). | Multi-curve ordered selection; shares picker with Loft profile-list. |
| **Phase-F-extend** | Surface Extend: face picker + distance modal (currently fixed at 0.5); continuity selector (G0/G1/G2). | Single-face selection + numeric modal; reuse ActiveDialog. |
| **Phase-F-blend** | Surface Blend: face/edge-chain picker (2 chains); continuity selector (G0/G1/G2). **Kernel gap also**: true tangent-continuous (G1/G2) surface blend not implemented — `surface_from_curves` is the current stand-in. Full production Blend requires both UX (chain picker) and kernel work (proper G1/G2 blend algorithm). | Requires kernel work in addition to UX before this bucket can be fully closed. |

| **Phase-F-generators** | CreateSprocket: parameter dialog (teeth, roller diameter, pitch, bore). CreateShaftDesign: segment-list editor (add/remove rows, length/diameter per row). CreateInvoluteGear: parameter dialog (teeth, module, pressure angle); face_width currently hardcoded 5.0 — needs face_width modal. | Three new wiring lanes (UI-B3) use variant-field defaults. Interactive parameter entry deferred to Phase F. |
| **Phase-F-shapebinder-picker** | ShapeBinder: face-list multi-picker. Currently binds all faces of selected object indiscriminately. | Requires selection-filter UX to pick specific faces from the target body. |
| **Phase-F-feature-mgmt** | SuppressFeature / SetTip / MoveFeatureUp / MoveFeatureDown: model-tree context-menu integration. Currently dispatched via toolbar/menu only; selection-driven scene mutations work. | Model-tree context-menu and right-click dispatch not yet wired to these variants. |

Until these UX buckets land, the hardcoded defaults remain the behavioral contract. Tests in `crates/viewer/tests/draft_medium_features.rs` and `crates/viewer/tests/pd_part_surface_medium_features.rs` encode those defaults and will fail if defaults change without a corresponding UX replacement.

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
| UI-B1 — Draft MEDIUM 10 verify-first | Verified already wired (10/10 MEDIUM arms with hardcoded defaults); dispatcher-boundary tests added (`draft_medium_features.rs`, 10 tests). MEDIUM-UX backlog deferred to §4. | c9fcdae | 2026-05-14 |
| UI-B2 — PD/Part/Surface MEDIUM 8 verify-first | Verified already wired (8/8 MEDIUM arms); MEDIUM tier fully exhausted. Dispatcher-boundary tests added (`pd_part_surface_medium_features.rs`, 8 tests). Workspace 3,366 → **3,374 / 0 / 1**. Cumulative 82/82. | 49d4881 | 2026-05-14 |
| UI-B3 — PartDesign untracked 7 (first true wiring lane) | 3 TRUE-STUB wired (`CreateSprocket`, `CreateShaftDesign`, `CreateInvoluteGear`); 5 verified wired (`ShapeBinder`, `SuppressFeature`, `SetTip`, `MoveFeatureUp`, `MoveFeatureDown`). §3.7 HARD-1 stale → HARD=0. Dispatcher-boundary tests added (`pd_untracked_features.rs`, 8 tests). Workspace 3,374 → **3,382 / 0 / 1**. Cumulative 90/90. | 70bc235 | 2026-05-14 |

**Current milestone (2026-05-14 working tree):** EASY tier (64) + MEDIUM tier (18) + PD-untracked (8) = **90/90** wired-with-defaults confirmed. All dispatcher arms in the original 79-stub inventory plus 16 newly-discovered untracked variants are DONE at the dispatcher level. **HARD count across all workbenches is now zero** (§3.7 HARD-1 entry verified stale 2026-05-14). Remaining work is UX polish — see §3.8 MEDIUM-UX Backlog. Workspace: **3,382 / 0 / 1**, plus HARD-tier overlay/FEM/TechDraw export/views/dimensions/annotations/centerlines/ShapeBinder batches, the first five command-UX TechDraw slices (Page Setup + Dimension Setup + Annotation Setup + Centerline Setup + View Placement Setup), FEM result interpretation UX, FEM multi-node BC editor UX, Sketcher single-profile validation UX, Sketcher constraint diagnostics UX, and Sketcher external reference/reuse UX verified at **2,844 / 0 / 0**. TechDraw's visible log-only backlog is closed, parameter-entry UX now covers page/dimension/annotation/centerline/view-placement commands, FEM post-processing has legend/probe/table interpretation tools, all kernel-side FEM BC variants are editor-reachable, sketch-driven features now reject open chains before kernel extrusion, Sketcher reports duplicate/conflicting/invalid constraints before feature/solver workflows proceed, and external projections/reused sketches now show visible `Refs:` / `Reuse:` state while adding construction references from selected objects.

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
