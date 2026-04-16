# CADKernel Developer Wiki

> **Version**: 0.1.0 (pre-alpha)  
> **Last updated**: 2026-04-13
> **Audience**: CADKernel kernel developers and contributors

[한국어](DEVELOPER_WIKI.ko.md) | **English**

---

## Table of Contents

- [1. Architecture Overview](#1-architecture-overview)
- [2. Crate Dependency Graph](#2-crate-dependency-graph)
- [3. Crate-by-Crate Guide](#3-crate-by-crate-guide)
- [4. Implementation Phases (1–4)](#4-implementation-phases-14)
- [5. API Design Principles](#5-api-design-principles)
- [6. Error Handling Patterns](#6-error-handling-patterns)
- [7. Testing Strategy](#7-testing-strategy)
- [8. Build & CI](#8-build--ci)
- [9. Workbench Toolbar Architecture](#9-workbench-toolbar-architecture)
- [10. Geometry Binding (Phase B05)](#10-geometry-binding-phase-b05)
- [11. Trim Infrastructure (Phase B01–B04)](#11-trim-infrastructure-phase-b01b04)
- [12. Exact Boolean Operations (Phase B06-B14)](#12-exact-boolean-operations-phase-b06-b14)
- [13. Next Steps](#13-next-steps)
- [14. Glossary](#14-glossary)
- [15. Extension Architecture](#15-extension-architecture)
- [16. MCP Integration](#16-mcp-integration)
- [17. Scripting](#17-scripting)
- [18. CI/CD](#18-cicd)
- [19. Lua Console](#19-lua-console)
- [20. Project Templates](#20-project-templates)
- [21. Convenience API](#21-convenience-api)
- [22. Example Scripts](#22-example-scripts)

---

## 1. Architecture Overview

CADKernel uses a **unidirectional layered architecture**. Upper crates depend on lower crates, never the reverse.

```
cadkernel (root)          ← unified re-export + prelude + E2E tests
├── cadkernel-viewer      ← native desktop GUI (egui + wgpu), 3D rendering, camera, navigation
├── cadkernel-python      ← Python bindings (PyO3)
├── cadkernel-io          ← STL/OBJ/glTF/STEP/IGES tessellation and I/O
├── cadkernel-sketch      ← 2D parametric sketch + constraint solver
├── cadkernel-modeling    ← primitive builders, Boolean, Feature ops
│   ├── cadkernel-topology  ← B-Rep half-edge data structure + Persistent Naming
│   │   ├── cadkernel-geometry  ← Curve/Surface traits + impls (feature flag)
│   │   │   ├── cadkernel-math  ← vectors, matrices, transforms, tolerance
│   │   │   │   └── cadkernel-core  ← KernelError, KernelResult
│   │   │   └── cadkernel-core
│   │   └── cadkernel-core
│   ├── cadkernel-geometry
│   └── cadkernel-math
├── cadkernel-topology
├── cadkernel-geometry
└── cadkernel-math
```

### Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| Separate `cadkernel-core` | Error types as lowest dependency, shared by all crates |
| Topology's geometry dep via feature flag | Pure topology use without geometry dependency |
| Remove sketch's direct nalgebra dep | Re-export through math crate's `linalg` module to avoid version conflicts |
| Unified `prelude` module | Single `use cadkernel::prelude::*` for full API access |

---

## 2. Crate Dependency Graph

```
cadkernel-core          (no dependencies)
    ↑
cadkernel-math          + nalgebra, glam
    ↑
cadkernel-geometry      + cadkernel-core
    ↑ (feature: geometry-binding)
cadkernel-topology      + cadkernel-core, cadkernel-math
    ↑
cadkernel-modeling      + cadkernel-core, cadkernel-math, cadkernel-geometry
    ↑
cadkernel-sketch        + cadkernel-math, cadkernel-topology
cadkernel-io            + cadkernel-math, cadkernel-topology
    ↑
cadkernel (root)        full integration
```

### Feature Flags

| Crate | Feature | Default | Effect |
|-------|---------|---------|--------|
| `cadkernel-topology` | `geometry-binding` | enabled | Includes `EdgeData.curve`, `FaceData.surface` fields |

---

## 3. Crate-by-Crate Guide

### 3.1 cadkernel-core

Shared foundational types: `KernelError` (6 variants), `KernelResult<T>`. Implements `Display`, `Error`, `From<std::io::Error>`.

### 3.2 cadkernel-math

All math primitives needed for CAD operations.

**Types**: `Vec2/3/4`, `Point2/3`, `Mat3/4`, `Transform`, `Quaternion`, `Ray3`, `BoundingBox`, `EPSILON`.

**Full operator support**: `+`, `-`, `*`, `/`, `+=`, `-=`, `*=`, `/=`, `Neg`, `Sum`, `f64 * Vec`, `Point ± Vec`, `From<[f64;N]>`, `From<(f64,...)>`, `From<Vec3> for Point3` (and reverse).

**`linalg` module**: Re-exports `nalgebra::DMatrix`, `DVector`, `LU`.

### 3.3 cadkernel-geometry

**Curve trait** with 10 methods (5 required, 5 default). Implementations: `Line`, `LineSegment`, `Arc`, `Circle`, `Ellipse`, `NurbsCurve`.

**Surface trait** with 7 methods (3 required, 4 default). Implementations: `Plane`, `Cylinder`, `Sphere`, `Cone`, `Torus`, `NurbsSurface`.

**Intersect module**: Surface-surface (4 pairs) and line-surface (3 pairs). Result types: `SsiResult`, `RayHit`, `IntersectionEllipse`.

All constructors return `KernelResult<Self>` for input validation.

### 3.4 cadkernel-topology

B-Rep half-edge data structure with generational arena storage.

**Entities**: Vertex, Edge, HalfEdge, Loop, Wire, Face, Shell, Solid.

**Persistent Naming**: `Tag` (hierarchical history path), `NameMap` (bidirectional Tag ↔ Handle mapping), `ShapeHistory` (operation records).

**BRepModel**: Full API for creation, traversal (5 helpers), validation (twin symmetry, loop cycles, Euler characteristic), and transformation.

### 3.5 cadkernel-sketch

2D parametric sketch with 24 constraint types (Coincident, Horizontal, Vertical, Parallel, Perpendicular, PointOnLine, PointOnCircle, Symmetric, Distance, Angle, Radius, Length, Fixed, Tangent, EqualLength, Midpoint, Collinear, EqualRadius, Concentric, Diameter, Block, HorizontalDistance, VerticalDistance, PointOnObject). Entity types: Point, Line, Arc, Circle, Ellipse, BSpline, EllipticalArc, HyperbolicArc, ParabolicArc (9 total). Helper methods: `add_polyline`, `add_regular_polygon`, `add_arc_3pt`, `add_circle_3pt`, `add_ellipse_3pt`, `add_centered_rectangle`, `add_rounded_rectangle`, `add_arc_slot`. Sketch editing tools (`tools.rs`): `fillet_sketch_corner`, `chamfer_sketch_corner`, `trim_edge`, `split_edge`, `extend_edge`. Sketch validation (`validate.rs`): `validate_sketch` with 7 issue types. Construction geometry: `toggle_construction_mode`, `mark_construction_point`, `mark_construction_line`. Newton-Raphson solver with Armijo backtracking. Profile extraction to 3D via `WorkPlane`.

### 3.6 cadkernel-modeling

Primitive builders (13 total: `make_box`, `make_cylinder`, `make_sphere`, `make_cone`, `make_torus`, `make_tube`, `make_prism`, `make_wedge`, `make_ellipsoid`, `make_helix`, `make_spiral`, `make_polygon`, `make_plane_face`), feature operations (`extrude`, `revolve`, `pad`, `pocket`, `groove`, `hole`, `countersunk_hole`, `fillet_edge`, `draft_faces`, `split_solid`, `chamfer_edge`, `sweep`, `loft`, `mirror_solid`, `scale_solid`, `shell_solid`, `linear_pattern`, `circular_pattern`, `section_solid`, `offset_solid`, `thickness_solid`, `multi_transform`), boolean operations (`Union`, `Subtract`, `Intersect`, `boolean_xor`), query functions (`point_in_solid`, `closest_point_on_solid`, `check_geometry`, `check_watertight`), assembly module (`Assembly`, `Component`, `AssemblyConstraint`, interference detection), draft operations (37 functions in `draft_ops.rs`: wire creation, manipulation, solid transforms, arrays, annotations, snapping, queries), surface operations (`ruled_surface`, `surface_from_curves`, `extend_surface`, `pipe_surface`, `filling`, `sections`, `curve_on_mesh`), join operations (`connect_shapes`, `embed_shapes`, `cutout_shapes` in `join.rs`), compound operations (`boolean_fragments`, `slice_to_compound`, `compound_filter`, `explode_compound` in `compound_ops.rs`), shape operations (`face_from_wires`, `points_from_shape` in `face_from_wires.rs`), `Body` (PartDesign feature tree), `Compound` (solid grouping), `make_involute_gear` (parametric gear profile). Assembly module with DOF analysis (`analyze_dof`), iterative constraint solver (`solve`), 13 joint types including RackAndPinion/ScrewJoint/BeltJoint, `rotation()` placement helper. Additive/subtractive operations (20 total, in `additive.rs`): `additive_box`/`subtractive_box`, `additive_cylinder`/`subtractive_cylinder`, `additive_sphere`/`subtractive_sphere`, `additive_cone`/`subtractive_cone`, `additive_torus`/`subtractive_torus`, `additive_helix`/`subtractive_helix`, `additive_ellipsoid`/`subtractive_ellipsoid`, `additive_prism`/`subtractive_prism`, `additive_wedge`/`subtractive_wedge`, plus `subtractive_loft` and `subtractive_pipe`. All return `KernelResult`. All auto-generate persistent naming tags.

### 3.7 cadkernel-io

Tessellation (`tessellate_solid`, `tessellate_face`, `tessellate_solid_parallel`), export/import for 10 formats (STL, OBJ, glTF, SVG, JSON, STEP, IGES, DXF, PLY, 3MF, BREP), TechDraw (`project_solid`, `three_view_drawing`, `section_view`, `detail_view`, `drawing_to_svg`, `DimensionType` with 6 annotation types, 10 advanced annotations: ArcLengthDimension, ExtentDimension, ChamferDimension, WeldSymbol, BalloonAnnotation, Centerline, BoltCircleCenterlines, CosmeticLine, BreakLine), mesh operations (29 total: `decimate_mesh`, `fill_holes`, `compute_curvature`, `subdivide_mesh`, `flip_normals`, `smooth_mesh`, `mesh_boolean_union`, `mesh_boolean_intersection`, `mesh_boolean_difference`, `cut_mesh_with_plane`, `mesh_section_from_plane`, `mesh_cross_sections`, `split_mesh_by_components`, `harmonize_normals`, `check_mesh_watertight`, `regular_solid`, `face_info`, `bounding_box_info`, `curvature_plot`, `add_triangle`, `unwrap_mesh`, `unwrap_face`, `remove_components_by_size`, `remove_component`, `trim_mesh`, `segment_mesh`, `remesh`, `evaluate_and_repair`, `scale_mesh`). Exported types: `FaceInfo`, `MeshBoundingBox`, `MeshRepairReport`, `MeshSegment`, `RegularSolidType`, `UnwrapResult`, `UvCoord`.

### 3.8 cadkernel-viewer

Native desktop GUI application (egui 0.31 + wgpu 24.x + winit 0.30).

**Modules**: `app.rs` (state + event loop), `render.rs` (GPU + camera + math), `gui/` (12-file module directory), `nav.rs` (mouse navigation presets).

**GUI Module** (`gui/`): `mod.rs` (GuiState, GuiAction enum with 130+ variants, draw_ui entry), `menu.rs` (File/Edit/Create/View/Tools/Help menu bar), `toolbar.rs` (common + 9 workbench toolbars with full backend wiring), `tree.rs` (hierarchical model tree with EntityIcon, search/filter, inline rename, visibility toggle, tip markers), `properties.rs` (Data/View tabs with parametric editing, transform controls, color picker), `status_bar.rs` (mouse coords, DOF status, workbench/selection mode, scene stats, FPS), `report.rs` (Report/Console tabs with severity filtering and count badges), `dialogs.rs` (28 creation dialogs + boolean + part ops), `sketch_ui.rs` (sketch overlay + grid + snap indicators), `overlays.rs` (origin axes + 3D grid + measurement + snap highlights), `view_cube.rs` (truncated cube navigation), `context_menu.rs` (object/viewport/tree/face-edge context menus with workbench-aware operations), `task_panel.rs` (35 ActiveTask variants: 10 primitives + 11 PartDesign + 6 Draft + 2 Surface + 1 FEM + 1 Boolean + 1 Scale, each with live 3D preview), `theme.rs` (CadTheme with Dark/Light + 3 density modes).

**Action Processing** (`app.rs`): `process_actions()` handles all 130+ `GuiAction` variants. Complete backend integration for all 9 workbenches: Part (join/compound/shape ops), PartDesign (pad/pocket/groove/hole + additive/subtractive), Sketcher (constraints + tools), Mesh (15 ops + analysis), TechDraw (views/dimensions/centerlines/cosmetics), Assembly (13 joints + solver + simulation), Draft (wire creation/modification/arrays), Surface (7 ops), FEM (mesh generation + 6 analysis types + 9 equations + post-processing). All handlers log to report panel.

**Report System**: `log_info()`/`log_warning()`/`log_error()` helper methods on `CadApp`. All 130+ action handlers log to the report panel via `gui.log(ReportLevel, msg)`. Status bar shows the latest message, report panel preserves full history with severity filtering.

**Camera**: Orbit (yaw/pitch/distance) + in-plane roll. View matrix applies roll rotation when non-zero. Screen-right/up methods are roll-aware. Roll snaps to nearest 90° on view transitions with direction-aware midpoint resolution (at 45°, snaps toward previous roll position via `prev_roll` tracking). Top/Bottom views preserve current yaw (only pitch changes). All roll angles normalized to (−π, π] via `wrap_angle()` — `snap_roll_90` normalizes inputs, `RollDelta` normalizes after each press, `ScreenOrbit` saves `prev_roll` after animation snap (not before).

**ViewCube**: Truncated cube with 26 depth-sorted polygons: 6 octagonal faces, 8 triangular corners, 12 edge bevel quads. Edge quads computed via face-normal offset from shared chamfer vertices (`EDGE_BEVEL=0.24`), with per-edge normals (`EDGE_NORMALS`). All non-hovered polygons rendered as a single `epaint::Mesh` (fan-triangulated) — eliminates egui anti-aliasing feathering seams between adjacent polygons. Opaque fill (`from_rgb`), XYZ axis indicator rendered ON TOP. Hovered polygon rendered separately via `convex_polygon` with stroke highlight. Face/edge/corner hover detection (point-in-polygon) and click-to-snap (26 view directions). Directional lighting, drop shadow, orbit ring with compass labels. Engraved face labels (TextShape rotation). Dropdown menu (☰) for projection/view shortcuts.

**Mesh Normals**: Smooth-group BFS algorithm (`mesh_to_vertices` in `render.rs`). At each vertex, faces are transitively grouped within crease angle (60°) via BFS. **Area-weighted** accumulation: raw cross products (magnitude ∝ triangle area) are summed, then normalized — large triangles contribute proportionally more, eliminating artifacts from non-uniform mesh density. Requires vertex sharing across faces — `tessellate_solid` deduplicates vertices via bit-exact `f64::to_bits` matching, STL import deduplicates via quantized keys (1e4 precision).

**4x MSAA**: All render pipelines use `MultisampleState { count: 4 }`. Scene pass renders to MSAA color+depth textures, then resolves to surface texture. Eliminates Mach band artifacts (visible triangle edges on smooth surfaces). egui pass remains sample_count=1 (2D UI on resolved surface).

**Composite Alpha**: Surface configuration uses `wgpu::CompositeAlphaMode::Opaque` to prevent Linux compositor blending artifacts (3D viewport bleeding through egui panels).

**Navigation**: 12 presets matching FreeCAD exactly (CAD/Gesture/Blender/Maya/SolidWorks/OpenInventor/OpenCascade/OpenSCAD/Revit/SiemensNX/TinkerCAD/Touchpad). Camera animation with smooth-step easing (3t²−2t³). `OrbitStyle` (5): Turntable (pitch-clamped), FreeTurntable (unclamped), Trackball/TrackballClassic/RoundedArcball (virtual-sphere mapping). `RotationMode` (3): WindowCenter (orbit around target), ObjectCenter (selected object centroid), DragAtCursor (screen-space pivot). `zoom_at_cursor`: shifts target toward cursor on scroll. `zoom_step`: discrete scroll zoom amount (20% default).

**Settings Dialog**: Tabbed Preferences dialog with 6 tabs (General, Display, Navigation, Appearance, Lighting, Shortcuts). General: unit system (`UnitSystem` enum), decimal places, auto-save, recent files limit, confirm delete. Display: background gradient presets (`BgPreset` enum with 5 variants including Custom), viewport overlays, selection/pre-selection colors, tessellation quality. Navigation: mouse presets, sensitivity, animation, View Cube. Appearance: theme, density. Lighting: enable, intensity, direction. "Reset All" in sidebar.

**Dynamic Background Gradient**: `BgPreset` enum (Dark/Medium/Light/Blueprint/Custom) in `nav.rs`. `GpuState::update_bg()` regenerates the background shader pipeline at runtime when colors change, enabling live gradient switching without restart. Custom preset uses `bg_custom_top`/`bg_custom_bottom` color fields with color pickers in Display settings.

**Section Plane**: `clip_enabled`, `clip_plane_normal`, `clip_plane_offset` in NavConfig. WGSL shader uses `uniforms.clip_params` to discard fragments beyond the plane. Toggle via Shift+S or View menu. Display settings tab provides axis selector (X/Y/Z) and offset DragValue. When disabled, `CLIP_DISABLED` sentinel `[0,0,0,1000]` passes all fragments.

**ViewCube Drag Rotation**: Dragging on the ViewCube orbits the camera continuously via `ScreenOrbit`. Click-without-move preserves face/edge/corner snap behavior. State tracked via `cube_dragging`/`cube_drag_moved` in GuiState.

**Transform Gizmo**: Interactive move/rotate/scale gizmo drawn at selected object center. `GizmoMode` enum (None/Translate/Rotate/Scale), toggled via W/E/R keys. Hover detection: `point_to_segment_dist()` measures cursor distance to axis arrows; highlighted axis drawn thicker. Drag interaction: mouse delta projected onto screen-space axis direction, converted to world-space transform. Speed scales with `camera.distance` for consistent feel at any zoom.

**3D Grid Snap**: `snap_to_grid_3d` in NavConfig, `snap_3d()` utility rounds values to nearest `grid_3d_spacing`. Applied in MoveObject action handler. Toggle in Display settings.

**Undo/Redo History Panel**: "History" tab in bottom panel (alongside Report and Console). `CommandStack::entries()` returns (history_descriptions, future_descriptions). History populated each frame before egui draw. Shows numbered operation list, current position marker, dimmed redo entries, Undo/Redo buttons.

**Material Presets, Undo History Dropdown & Export Options (V45)**: 16 material presets in Properties View tab — Steel, Aluminum, Brass, Copper, Gold, Titanium, Cast Iron, 4 Plastics, Glass (alpha 0.35), 2 Woods, Rubber, Carbon Fiber. 2-column grid with icon+name buttons and color swatches, auto-detect current material by color proximity. Undo/Redo toolbar gets dropdown arrow buttons (▾) — click to show up to 10 history entries with step numbers, click entry to multi-step undo/redo. STL Export Options dialog: Binary/ASCII format radio, scale factor DragValue. `ExportStlWithOptions` action applies vertex scaling before calling `export_stl_binary`/`export_stl_ascii`. Menu STL export opens options dialog first.

**Breadcrumb Bar, Recent Files & Toast Notifications (V44)**: Breadcrumb navigation bar added as TopBottomPanel between context toolbar and viewport via `draw_breadcrumb_bar()`. Shows path: Scene › ObjectName › Face/Edge/Vertex/Solid, with Sketch appended in sketch mode. Clickable Scene root triggers DeselectAll. Multi-selection count right-aligned in accent blue. Chevron separators, active=white, inactive=dim. Recent Files submenu in File menu — up to 10 files with filename display + full path tooltip, ClearRecentFiles action. Files added to `recent_files` on open/import (dedup, max 10). Toast notification system: `Toast` struct with `ToastLevel` (Success/Info/Warning/Error), `draw_toast_overlay()` renders bottom-right floating notifications. 3-second auto-dismiss with fade-in/fade-out animations. Level-specific accent bar, background tint, and icon (✓/ℹ/⚠/✖). Max 5 stacked, text truncated with ellipsis. `log_info/warning/error` and StatusMessage all trigger corresponding toasts.

**Menu Shortcut Text Alignment (V43)**: `menu_action_sc()` helper added to `menu.rs` — uses `egui::Button::new().shortcut_text()` for right-aligned shortcut hints in menus. All menu items converted from inline format (`"New  (Ctrl+N)"`) to native `shortcut_text()` API. Covers File (New/Open/Save As/Quit), Edit (Undo/Redo/Copy/Paste/Select All/Deselect All/Delete), View (projection/Standard Views/Grid/Fit All/Section Plane), Sketch (all geometry tools, constraints, toggles, Close/Cancel), and PartDesign (Pad) menus.

**Context Menu Icons, Welcome Screen & Display Mode Selector (V42)**: Context menus enhanced with `menu_item()` helper — emoji icon prefix + shortcut hint suffix on button text. Object menu items: Select(📌), Duplicate(⎘), Rename(✏), Hide/Show(👁), Measure(📏), Check(✔), Delete(🗑). Viewport menu items with shortcut keys (V, G, 5, Ctrl+A, Esc). `draw_welcome_screen()` overlay for empty scenes — CADKernel title, 3 quick-action buttons (Create Box, Import File, Open Project) with painter-based rendering and manual hit-testing (`pointer_pos` + `button_clicked`). Hover effects change background/cursor. F1 hint at bottom. Only shown when `scene.is_empty() && sketch_mode.is_none() && active_task.is_none()`. Display mode ComboBox dropdown added to toolbar View section — shows current mode, lists all 8 modes with `mode.shortcut()` in dropdown. `tb_display_mode: DisplayMode` field added to GuiState, mirrored from `ViewportInfo` each frame before toolbar draw.

**Status Bar, Gizmo Toolbar & Projection Toggle (V41)**: Status bar right section split into separate styled segments with `vert_divider()` dividers — projection (clickable, `Persp` blue / `Ortho` green, toggles via `GuiAction::ToggleProjection`), display mode, scene stats (`vis/total obj` + K/M triangle formatting), selection info (accent blue), measure mode (yellow), FPS. `draw_status_bar()` signature changed to `&mut GuiState` for action push. Toolbar gains "Transform" section with Move (W), Rotate (E), Scale (R) gizmo toggle buttons via `icon_toggle` with active highlight. `GuiAction::SetGizmoMode(GizmoMode)` added — toggles off if same mode clicked, switches otherwise. `GizmoMode` imported at top level in both `toolbar.rs` and `app.rs`.

**Dialog Consistency, Keyboard Shortcuts & Tree Polish (V40)**: All 32 dialog grids in `dialogs.rs` converted from 3-column (Label | DragValue | "mm") to 2-column layout with DragValue `.suffix(" mm")` — matching V39 properties panel pattern. Grid spacings widened to `[10.0, 4.0]`, all manual third-column `ui.label("")` artifacts removed, label colors standardized to `theme::COLOR_DIM`. Shaft segment DragValues use `.suffix(" mm")` with L=/D= prefixes. F1 key toggles keyboard shortcuts reference panel — 9 categorized sections (File, Edit, Navigation, Standard Views, Display Modes, Transform Gizmo, Selection Modes, Sketcher, General) with `dialog_section()` accent headers and emoji icons. Sketcher section documents all in-sketch keybindings (S/L/R/C/A/E/P/B/W/H/V/Enter/Escape). Model tree mini toolbar added between search box and tree content — expand all (▿) and collapse all (▹) buttons toggle `("tree_expand", obj_id)` state for all top-level and child nodes. Filter result count displayed in mini toolbar when filter active.

**Properties, Report, History & Sketch Polish (V39)**: Properties panel parameter grids widened to `[10.0, 4.0]` spacing with 2-column layout — DragValue `.suffix(" mm")` replaces separate unit label column. Scene overview shows aggregate triangle/vertex counts with K/M formatting (`format_count()` helper) and icon header. Report panel log entries show level icons (ℹ/⚠/✖) with monospace timestamp column and tinted background for warning/error rows. History panel entries show context-aware operation icons via `history_icon()` function (➕ create, ➖ delete, → move, ↻ rotate, ⤢ scale, ∪ boolean, ⬆ extrude) with monospace numbered alignment and green arrow current-state marker. Sketch dimension input popup redesigned with icon title, DragValue suffix (mm/°), styled OK/Cancel buttons. Sketch context menu uses section headers (Edit, Constraints, Selection) with icons on menu items.

**FreeCAD-Style Sketch UI, Task Panel & Theme Expansion (V38)**: Task panel fully overhauled with `draw_task_header()` (accent gradient bar + icon), `draw_task_section()` (accent underline labels), `draw_task_buttons()` (accent blue OK + plain Cancel). DRY macros `plabel!/pmm!/pdeg!/pval!` for grid rendering with DragValue suffixes. Menu bar uses `menu_section()` with `theme::MENU_SECTION_COLOR` for grouped sections. Sketch UI colors changed to FreeCAD palette — white geometry, blue construction, green selected, golden pending, red constraints. Cursor crosshair uses gap-center style. Snap indicators redesigned: coincident = dot+ring, H/V = red dashed guidelines with text badge, midpoint = filled diamond. Banner uses rounded background pill with color-coding (red=conflict, green=constrained, blue=default). DOF arrows changed to orange. Construction points shown as blue X markers. OVP panel shows tool icon+name header. Theme expanded with 8 sketch color constants (`SKETCH_GEOMETRY/CONSTRUCTION/SELECTED/HOVERED/PENDING/CONSTRAINT/VIOLATED/DOF`) and `MENU_SECTION_COLOR`. ComboView panel widened to 300px default, tree/properties split adjusted to 45/55.

**Toolbar, Dialog & Context Menu Polish (V37)**: `section_label()` renders small 9px dim gray text labels before each tool group in all 9 workbench context toolbars (Part, PartDesign, Sketcher, Mesh, TechDraw, Assembly, Draft, Surface, FEM) and the main toolbar (File/Edit/View/Scene/Select). `toolbar_separator()` uses gradient fade (transparent→gray→transparent with brighter center). `dialog_section()` renders FreeCAD-style accent bar headers — blue-tinted background rect with 3px left accent bar. `button_bar()` uses accent blue primary button with white text and minimum sizes. All 31 dialog grids widened from `[4.0, 2.0]` to `[8.0, 4.0]`. Context menus use `menu_section()` dim bold labels for grouping (object menu: Selection/Edit/Appearance/Analysis; viewport menu: View/Display/Overlays/Selection/Create).

**FreeCAD-Inspired UI Chrome (V36)**: Theme system extended with panel chrome colors (`panel_header_bg/text`, `section_header_bg/text`, `panel_separator`) and sizing (`panel_header_height`, `section_header_height`, `tree_row_height`) — density-scaled. `draw_panel_header()` renders FreeCAD-style dock title bars (dark bg, title text, close button). `draw_section_header()` renders collapsible property groups with expand arrow. ComboView panel uses zero inner margin + titled headers ("Model", "Properties"). Model tree has document root node, selection with left accent bar, hover-only eye icons. Properties panel uses underline-style Data/View tabs, section headers with background bars, widened grid spacing. Report panel uses custom tab bar with underline active indicator. Status bar uses `vert_divider()` thin lines, top accent edge, coordinate unit suffix.

**Camera View Bookmarks**: `ViewBookmark` struct (name, yaw, pitch, roll, distance, target) in `nav.rs`. `view_bookmarks: Vec<ViewBookmark>` in NavConfig (max 20). View > Bookmarks submenu (220px width): hint text input, enabled/disabled Save button, numbered entries with camera angle tooltips (yaw°/pitch°/dist), right-aligned delete, "X/20 bookmarks" count. Save uses name match for overwrite. Restore uses `.cloned()` to avoid borrow conflict, then animates. Bookmark info mirrored to GuiState each frame.

**Preselection Highlight**: `draw_selection_overlay()` shows edge/face/vertex preselection on hover even with no active selection. Configurable colors: `nav.preselection_color`/`nav.selection_color` ([u8; 3]). Cursor-following entity type label. Edge highlight: 8px glow + 3.5px core line. Vertex: 10px glow ring + 6px marker + 2px center dot. Runtime hover/preselection updates stay off the full scene rebuild path, and nearest-hit selection now uses depth ordering rather than first-object traversal.

**Display Mode Rendering**: `DisplayMode::Points` renders with a dedicated `PointList` GPU pipeline. Per-object shaded draws reuse one dynamic uniform slot per draw call, removing the previous silent object-count cap in large scenes.

**Object Grouping**: `ObjectGroup` struct in `scene.rs` with id/name/visibility. Objects have `group_id` field (0 = ungrouped). Scene methods: `create_group()`, `group_selected()`, `ungroup_object()`, `toggle_group_visibility()`, `delete_group()`, `group_members()`. Edit > Groups submenu: eye icons (◉/○) with color coding, member count "(N)", hint text input. Model tree groups section: header with eye toggle, folder icon, name, count, delete; indented member names.

**3D Measurement Overlay**: `draw_measurement_overlay()` with unit-aware display (`nav.unit_system.label()`, `nav.decimal_places`). Numbered point markers (P1, P2...), coordinate display, distances between ALL consecutive pairs, total path length for 3+ points, ΔX/ΔY/ΔZ component breakdown, angle arc with degree display for 3+ points. Context-aware mode indicator. Labels use `draw_label_with_bg()` (rounded rect + outline). `pick_surface_point()` ray casts with vertex snap (threshold `camera.distance * 0.012`), triangle surface fallback. C clears, Escape exits.

**Coordinate Axes Indicator**: `draw_axes_overlay()` — clickable axis tips snap to standard views (X+→Right, X−→Left, Y+→Front, Y−→Back, Z+→Top, Z−→Bottom) via `SetStandardView` action. Hover detection with ring highlight + pointing hand cursor. Depth-sorted back-to-front with opacity fade (facing factor). Glow lines (5px glow + 2.5px core) for anti-aliased appearance. Label text shadows (1px offset dark behind). Gradient background ring. Center sphere with specular highlight. `Camera::forward()` for depth computation.

**NavConfig** (`nav.rs`): Persistent settings struct with `Clone` derive. Core fields: mouse style, sensitivities, animation, View Cube, theme_mode, ui_density, overlay toggles. Extended fields: `unit_system` (`UnitSystem`), `decimal_places`, `bg_preset` (`BgPreset`), `selection_color`, `preselection_color`, `tessellation_segments`, `auto_save_enabled`, `auto_save_interval_secs`, `recent_files_max`, `confirm_delete`. Navigation behavior fields: `orbit_style` (`OrbitStyle`), `rotation_mode` (`RotationMode`), `zoom_step` (f32, 0.2), `zoom_at_cursor` (bool), `disable_touch_tilt`, `enable_spinning`, `show_rotation_center`, `rotation_center_size`. Methods: `apply_orbit()` dispatches orbit computation per OrbitStyle, `scroll_zoom_factor()` uses `zoom_step`, `drag_zoom_factor()` uses `zoom_sensitivity`. New enums: `UnitSystem` (Millimeter/Centimeter/Meter/Inch/Foot with `label()`/`long_label()`), `BgPreset` (Dark/Medium/Light/Blueprint with `label()`), `OrbitStyle` (5 variants), `RotationMode` (3 variants).

---

## 4. Implementation Phases (1–4)

### Phase 1: Foundation
Core kernel architecture: 7-crate workspace, math library, geometry engine, B-Rep topology, CI/CD.

### Phase 2: Persistent Naming + Boolean
Tag-based persistent naming, boolean operations (Union/Subtract/Intersect), surface-surface/line-surface intersections, geometry-topology binding via feature flag.

### Phase 3: Parametric + Sketch + I/O
2D sketch system with 19 constraints and Newton-Raphson solver, extrude/revolve feature ops, box/cylinder/sphere primitives, STL/OBJ export, tessellation.

### Phase 4: Core Hardening
- Extracted `cadkernel-core` for shared error types
- Converted all public `assert!`/`expect()` to `KernelResult`
- Added `Send + Sync` bounds to trait object fields
- Full math ergonomics: `Default`, `Display`, `From`, `AddAssign`, `Sum`, reverse operators
- `EntityStore::len()` optimized to O(1)
- Resolved `Ellipse` name collision → `IntersectionEllipse`
- Added `PartialEq` + `Copy` to all value-type geometry structs
- Wire entity integration with naming system
- Topology validation and traversal helpers

### Phase 5–9: Application (GUI + Workbenches)
Viewer with egui+wgpu, display modes, ViewCube, Part/PartDesign/Sketcher workbenches, interactive 2D sketch editing, feature implementations (mirror, scale, sweep, loft, shell, pattern).

### Phase A–B: NURBS Kernel + Trimmed B-Rep
Analytical derivatives, knot operations, curve/surface fitting, NURBS conversions, SSI, trimmed surfaces, geometry binding for all primitives.

### Phase C–E: STEP I/O + Fillet/Draft/Split + Advanced Primitives
Full STEP import/export, fillet/draft/split implementations, 5 new primitives (tube, prism, wedge, ellipsoid, helix).

### Phase F–K: Part Ops + TechDraw + Assembly + Sketcher
Section/offset/thickness operations, TechDraw section/detail views, assembly module with constraints and interference detection, 5 new sketch constraints (EqualLength, Midpoint, Collinear, EqualRadius, Concentric).

### Phase L–O: Draft + Mesh + Surface Workbenches
Draft operations (wire, B-spline wire, clone, rectangular/path array), mesh operations (decimate, fill holes, curvature, subdivide, flip normals), surface operations (ruled surface, surface from curves, extend, pipe surface).

### Phase N–P: FEM + IGES
FEM module (TetMesh generation, static analysis with Gauss-Seidel solver, von Mises stress, modal analysis, thermal analysis, mesh quality), 6 material presets (steel, aluminum, titanium, copper, concrete, cast iron), thermal materials, 8 boundary condition types (4 structural + 4 thermal), post-processing (stress/strain tensors, principal stresses, safety factor, strain energy, reactions), mesh utilities (refine, extract surface, merge nodes). IGES import/export (80-column fixed-format, Point/Line/Arc/NURBS entities).

### Phase 10: TechDraw Workbench
Orthographic projection (7 standard views), hidden-line removal, three-view drawing, dimension annotations (linear, angular, radius), full SVG export, viewport overlay.

### Phase 11: NURBS Kernel Strengthening
Adaptive curve/surface tessellation (`TessellationOptions`), curve-curve intersection (subdivision + Newton-Raphson), 2D polygon/polyline offset (miter-join), geometry binding helpers (`bind_edge_curve`, `bind_face_surface`), NURBS-aware tessellation in io crate.

### Phase A: NURBS Kernel Completion
Complete NURBS kernel for FreeCAD parity. Analytical derivatives (rational quotient rule for curves, homogeneous derivatives for surfaces), replacing finite differences. Full knot operations: insertion, refinement, removal, Bezier decomposition. Curve/surface fitting: interpolation (A9.1) and least-squares approximation (A9.7). Analytical→NURBS conversion for all primitive types (Line, Circle, Arc, Ellipse, Plane, Cylinder, Sphere). Newton-Raphson `project_point()` overrides for both NurbsCurve and NurbsSurface. Curve2D system for UV-space parametric curves. TrimmedCurve/TrimmedSurface for face-level trimming. Curve-surface and surface-surface intersection (marching algorithm with predictor-corrector). Bounding box overrides via convex hull property.

**Key modules added**: `bspline_basis.rs`, `curve/to_nurbs.rs`, `curve/curve2d.rs`, `curve/trimmed.rs`, `curve/nurbs_fitting.rs`, `surface/to_nurbs.rs`, `surface/trimmed.rs`, `surface/nurbs_fitting.rs`, `intersect/curve_surface.rs`, `intersect/surface_surface.rs`.

### Phase Q–S: Performance + Geometry Expansion + Modeling Expansion
BVH spatial index, parallel tessellation, isocurves, surface curvatures, offset/revolution/extrusion surfaces, blend curves, surface continuity analysis, spiral, polygon, plane face, boolean XOR, compound, check geometry/watertight, multi-transform, Body, involute gear.

### Phase T–U: Sketcher Expansion + File Formats + Mesh Ops
5 new constraint types (Diameter, Block, HorizontalDistance, VerticalDistance, PointOnObject), Ellipse/BSpline entities, polyline/polygon/arc_3pt helpers. DXF/PLY/3MF/BREP I/O, 7 mesh operations (smooth, boolean, cut, section, split, harmonize, watertight check), TechDraw dimensions.

### Phase V1: Sketcher Completion
3 new conic arc entity types (EllipticalArc, HyperbolicArc, ParabolicArc), 5 sketch editing tools in `tools.rs` (fillet/chamfer corner, trim/split/extend edge), sketch validation module `validate.rs` (7 issue types), construction geometry support, 5 new geometry helpers (circle_3pt, ellipse_3pt, centered_rectangle, rounded_rectangle, arc_slot).

### Phase V2: PartDesign Completion
8 new additive/subtractive primitive pairs in `additive.rs` (helix, ellipsoid, prism, wedge), 2 new subtractive operations (loft, pipe). Total additive/subtractive operations expanded from 10 to 20.

### Phase V3: Part Workbench Completion
Join operations in `join.rs` (connect_shapes, embed_shapes, cutout_shapes), compound operations in `compound_ops.rs` (boolean_fragments, slice_to_compound, compound_filter, explode_compound), shape operations in `face_from_wires.rs` (face_from_wires, points_from_shape).

### Phase V4: TechDraw Expansion
10 new annotation types: ArcLengthDimension, ExtentDimension, ChamferDimension, WeldSymbol (6 weld types), BalloonAnnotation, Centerline, BoltCircleCenterlines, CosmeticLine (4 styles), BreakLine. SVG rendering for all types.

### Phase V5: Assembly Solver
DOF analysis (`analyze_dof()`) with per-constraint/joint DOF counting, iterative constraint solver (`solve()`) with distance constraints, 3 new joint types (RackAndPinion, ScrewJoint, BeltJoint, 13 total), `rotation()` placement helper.

### Phase V6: Surface Workbench Completion
`filling()` (N-sided boundary patch), `sections()` (surface skinning through profiles), `curve_on_mesh()` (project polyline onto mesh).

### Phase V8: Mesh Completion
17 new mesh operations in `mesh_ops.rs`: `mesh_boolean_intersection`, `mesh_boolean_difference`, `regular_solid` (5 Platonic solids), `face_info`, `bounding_box_info`, `curvature_plot`, `add_triangle`, `unwrap_mesh`, `unwrap_face`, `remove_components_by_size`, `remove_component`, `trim_mesh`, `mesh_cross_sections`, `segment_mesh`, `remesh`, `evaluate_and_repair`, `scale_mesh`. New types: `FaceInfo`, `MeshBoundingBox`, `MeshRepairReport`, `MeshSegment`, `RegularSolidType`, `UnwrapResult`, `UvCoord`.

### Phase V9: Draft Workbench Completion
37 draft operations in `draft_ops.rs` (32 new + 5 existing). Wire creation (fillet, circle, arc, ellipse, rectangle, polygon, bezier, arc_3pt, chamfer, point), wire manipulation (offset, join, split, upgrade, downgrade, to/from bspline, stretch), solid transforms (move, rotate, scale, mirror), array patterns (polar, point), annotations (dimension, label, text), snapping (endpoint, midpoint, nearest), queries (length, area). New types: `DraftDimension`, `DraftLabel`, `SnapResult`, `WireResult`, `BSplineWireResult`, `ArrayResult`, `CloneResult`.

### Phase V10: FEM Workbench Expansion
6 new material presets (`FemMaterial::titanium/copper/concrete/cast_iron/custom`, `ThermalMaterial` with steel/aluminum/copper). 8 new FEM types (`ThermalMaterial`, `ThermalBoundaryCondition`, `ThermalResult`, `BeamSection`, `ModalResult`, `MeshQuality`, `PrincipalStresses`, `StrainResult`, `StressTensor`). 4 structural boundary conditions (Displacement, Gravity, DistributedLoad, Spring) + 4 thermal boundary conditions (FixedTemperature, HeatFlux, HeatGeneration, Convection). 3 new analysis functions: `modal_analysis()` (eigenfrequency via inverse power iteration), `thermal_analysis()` (steady-state heat conduction, Gauss-Seidel), `mesh_quality()` (aspect ratio, volume, degenerate detection). 3 new mesh functions: `refine_tet_mesh()` (1→8 subdivision), `extract_surface_mesh()` (boundary faces), `merge_coincident_nodes()` (tolerance-based dedup). 5 post-processing functions: `compute_stress_tensor()`, `compute_strain_tensor()`, `principal_stresses()` (Cardano eigenvalue), `safety_factor()`, `strain_energy()`, `compute_reactions()`.

### Phase V11: Viewer UI Expansion
File menu with 6 Import/Export formats (STEP, IGES, DXF, PLY, 3MF, BREP). Boolean dialogs, Part operations toolbar (Mirror/Scale/Shell/Fillet/Chamfer/Pattern), Mesh toolbar (Smooth, Harmonize, Watertight, Remesh, Repair), Analysis (Measure Solid, Check Geometry). ~20 new GuiAction variants.

### Deep Quality Improvements
Boolean: auto face-splitting in `boolean_op` (split→classify→evaluate), multi-sample classification (7-point majority voting). Sketch: DOF analysis via Jacobian rank, `drag_solve()` interactive point dragging. Viewer: Moller-Trumbore ray picking (`picking.rs`), sub-element picking (Face/Edge/Vertex) with B-Rep topology traversal, undo/redo command stack (`command.rs`, `ModelSnapshot`-based). Fixed critical `mat4_inv` bug (reversed cofactor naming + wrong row indices) that made 3D picking unreliable. Sub-element visual highlight overlay (`draw_selection_overlay` + `draw_entity_highlight`): Face triangles filled with semi-transparent blue + edge outlines, Edge as thick highlighted line with endpoint dots, Vertex as filled circle with white ring. Preselection (hover) uses green tint, selection uses blue tint. `update_preselection()` tracks sub-element handles for real-time hover feedback. Properties panel (`properties.rs`) shows sub-element details (face area/triangles/loops, edge length/endpoints, vertex coordinates) in a collapsible group when Face/Edge/Vertex selection mode is active. Multi-selection: `selected_entities: Vec<SelectedEntity>` with Ctrl+click toggle, overlay draws all selected entities, properties panel shows multi-selection summary (counts, total area/length). Measurement overlay: `draw_measurement_overlay_between()` draws dashed cyan line + distance label between 2 selected sub-elements. Selection mode toolbar: 4 toggle buttons (Solid/Face/Edge/Vertex) + count badges + Select All/Deselect, `SetSelectionMode` action, keyboard shortcuts (Key 2→Face, Key 4→Vertex). Selection-based operations: `selected_edge_pairs()` converts selected edges to vertex-pair handles for fillet/chamfer; `compute_face_workplane()` derives WorkPlane from selected face triangles (centroid + normal + perpendicular x_axis); `SketchOnSelectedFace` action enters sketch mode on face-derived plane. Context menus wired to real operations: face menu dispatches `SketchOnSelectedFace`, edge menu dispatches `FilletAllEdges`/`ChamferAllEdges` on selected edges, measurement buttons dispatch `ToggleMeasurement`. Toolbar selection-awareness: Part/PartDesign Fillet/Chamfer buttons detect selected edges and dispatch directly vs. opening task panel; Sketcher toolbar shows "On Face" button when face is selected. Edge loop selection: `compute_edge_loop()` BFS valence-2 chain walk from seed edge through shared vertices. Edge ring selection: `compute_edge_ring()` traverses opposite edges across quad faces (index+2 on 4-sided faces). Face loop selection: `compute_face_loop()` BFS outward from seed face through shared edges. All three wired to context menus (`SelectEdgeLoop`, `SelectEdgeRing`, `SelectFaceLoop` actions). Fillet/Chamfer parameter UI: toolbar always opens task panel with DragValue slider; task panel shows pre-selected edge count when edges are selected. Auto-pick in Solid mode: `pick_auto()` tries vertex → edge → face → solid (most specific first) without manual mode switching; `select_object_for_pick()` shared helper. Navigation: 12 FreeCAD styles with exact mappings, OrbitStyle (5 types with virtual-sphere trackball), RotationMode (3 types), zoom_at_cursor, zoom_step. Universal auto-pick: all selection modes use `pick_auto()` (vertex > edge > face > solid priority), preselection also auto-picks; removed mode-specific `pick_face()/pick_edge_mode()/pick_vertex_mode()`. Double-click loop selection: `try_double_click_loop()` (300ms/10px threshold) — edge double-click → edge loop, face double-click → face loop. Hover preview: status bar shows entity type+index under cursor via `build_hover_preview()`. Context menu, properties panel, selection overlay all adapted to detect entity type from `selected_entities` content instead of `SelectionMode`. Sketch interaction: live rubber-band preview for all tools (Line/Rectangle/Circle/Arc/Ellipse/Polygon/Polyline/BSpline/Slot), grid snap (0.5) + point snap (0.3 threshold), Ellipse tool uses `add_ellipse()` (not `add_circle()`), Arc tool uses click-angle-based ±90° semicircle. Sketch undo: `SketchSnapshot` records entity counts before each operation, `SketchMode::undo()` truncates vectors to snapshot; Ctrl+Z in sketch mode undoes last sketch operation instead of global undo. Sketch editing: `SketchEntityRef` enum for entity selection, Select tool with hit-testing (point > line > circle > arc > ellipse > B-spline priority), selected entities highlighted in blue (3px stroke), Ctrl+click multi-selection, Delete key removes selected entities with cascading PointId adjustment. Sketch keyboard shortcuts: S/L/R/C/A/E/P/B/W for tools, H/V for constraints. Interactive constraints: all toolbar buttons apply to selected entities (Coincident, Parallel, Perpendicular, Equal, Fixed, Block, Distance, Angle, Radius, H/V-Distance); falls back to last entity when no selection. Extrude distance: DragValue UI in Sketcher toolbar (0–1000mm).

### Phase V7: File Format Expansion
5 new format implementations: glTF import (base64 buffer decode, multi-component indices), 3MF import (XML parsing), DWG import/export (version detection R2000–2018+, 3DFACE heuristic, DXF fallback), PDF export (SVG→PDF 1.4 conversion for TechDraw), DAE Collada import/export (COLLADA 1.4.1 XML geometry). Total I/O formats: 15 (was 11). New modules: `collada.rs`, `dwg.rs`, `pdf.rs`.

### Phase V13: Performance & Validation
BVH-accelerated boolean broad-phase (O(n log n) face-pair overlap), 11 new benchmarks (25 total): primitives (cone, torus), features (mirror, scale, fillet), validation (check_geometry, check_watertight), stress (tessellate_sphere_64x32, tessellate_torus_64x32, boolean_intersection).

### Phase V12: Python Bindings
PyO3-based `cadkernel` Python module in `crates/python/` (standalone build, excluded from workspace). 6 classes (Model, SolidHandle, Mesh, MassProperties, GeometryCheck, Sketch), 10 primitive creators, 4 feature ops, 3 boolean ops, tessellation/analysis, 10 I/O functions, sketch system with 7 constraint types.

### FreeCAD-Level UI Overhaul
`gui.rs` (3605 lines) refactored into `gui/` module directory (12 files). Hierarchical model tree, property editor, full menu system, enhanced status bar, report panel with color-coded logging, solid/viewport context menus, 9 workbench toolbars (Part, PartDesign, Sketcher, Mesh, TechDraw, Assembly, Draft, Surface, FEM). Report logging added to 40+ action handlers.

### Phase W: FreeCAD Parity Sprint
Part shape primitives (circle, ellipse, point, line shapes + shape builder + convert to solid). PartDesign completion (additive_loft/pipe, sprocket, shaft_design, shape/sub-shape binders, body context menu: suppress_feature/set_tip/move_feature). Sketcher geometry expansion (periodic B-spline, B-spline from knots, centered/rounded rectangle, slot, arc slot, circle/ellipse 3pt, Refraction constraint, toggle_driving_reference, attach/reorient/merge/mirror sketch). Sketcher B-spline tools (geometry_to_bspline, degree increase/decrease, knot multiplicity ops, insert knot, join curves, external projection, carbon copy, move/rotate/scale/offset/mirror geometry, delete all geometry/constraints). TechDraw views (broken, complex section, clip group, active, project shape 2D), dimensions (contextual, angle 3pt, area, arc length, H/V extent, repair refs), annotations (rich text, balloon, axonometric length, geometric hatch, weld symbol ISO 2553, hole/shaft fit). TechDraw centerlines (face, between lines/points, bolt circle), cosmetics (line, thread internal/external, vertex, circle, arc, parallel/perpendicular line), formatting (chain/coordinate/chamfer dimension, FormattedDimension), management (stack order, align, lock, page template, update fields, redraw, print all, edit appearance, toggle edge visibility). Draft workbench (arc 3pt, ellipse/rectangle/polygon wire, bezier/cubic bezier, point, facebinder, hatch, dimension/label/annotation style, move/rotate/scale/mirror/offset/trimex/stretch, circular/path link/point link array, edit/join/split, draft to sketch, snap system). Assembly (solve_constraints with Newton-Raphson, simulate_step, export_asmt, AssemblyPreferences). FEM (AnalysisContainer, ElementGeometry, EM/Fluid boundary conditions, GeometricalFeature, heat/flow/deformation/electrostatic equations, filter functions, visualization modes, purge results, mesh regions). I/O (VRML import/export, AMF import/export). 182 new tests (942 total).

**Assembly Solver Architecture:**
Newton-Raphson iterative constraint solver in `assembly.rs`. `solve_constraints()` computes constraint residuals and Jacobian, iterates until convergence. `simulate_step()` advances time step for kinematic simulation. `AssemblyPreferences` configures solver tolerances and iteration limits.

**FEM Equation System:**
Four equation types in `fem.rs`: `heat_equation()` (steady-state thermal), `flow_equation()` (Stokes flow), `deformation_equation()` (linear elasticity), `electrostatic_equation()` (Poisson). Each returns field results (temperature, velocity, displacement, potential). `AnalysisContainer` groups mesh, materials, boundary conditions, and results. `apply_filter()` with `FilterFunction` enum (Threshold, Clip, Contour, Gradient) for post-processing. `VisualizationMode` enum for result display.

**TechDraw Formatting System:**
`FormattedDimension` struct with prefix, suffix, tolerance, and override fields. `chain_dimension()` and `coordinate_dimension()` for sequential dimensioning. `chamfer_dimension()` for chamfer annotations. `stack_order()`, `align_elements()`, `lock_element()` for element layout management. `page_from_template()` with template field substitution.

**Draft Snap System:**
`SnapMode` enum with 16 snap modes (Endpoint, Midpoint, Center, Quadrant, Intersection, Extension, Perpendicular, Parallel, Tangent, Nearest, Grid, Ortho, Special, Dimension, WorkingPlane, Angle). `snap_to_point()` finds nearest snap target. `snap_lock()` toggles snap mode activation.

**Sketcher B-Spline Tools:**
`geometry_to_bspline()` converts any sketch entity to B-spline. `increase/decrease_bspline_degree()` elevates or reduces degree. `increase/decrease_knot_multiplicity()` adjusts knot multiplicities for continuity control. `insert_knot()` adds knots. `join_curves()` merges adjacent B-spline segments.

**VRML/AMF Formats:**
`vrml.rs`: VRML97 import/export with Shape, IndexedFaceSet, Coordinate, Normal nodes. `amf.rs`: AMF 1.1 XML import/export with object/mesh/volume/vertex/triangle hierarchy.

**Current status**: 1133 tests, 0 clippy warnings, 0 build errors. FreeCAD parity: 576/576 (100%).

### FreeCAD Parity Sprint 3 (2026-03-25)
Kernel gaps closed (12 items): PrimitiveParams + make_primitive (unified constructor), offset_polygon_2d_checked (KernelResult), project_curves_on_surface (curve projection), auto_defeaturing (size threshold), transformed_copy (clone + transform), Body::move_object_to_body (feature transfer), add_triangle/square/pentagon/hexagon/heptagon/octagon (polygon shortcuts), external_intersection (sketch tool), toggle_section_view + SectionViewState, coons_patch (bilinear blending), make_line_draft (2-point line). FEM completion (28+ items): HexMesh + generate_hex_mesh, mesh_from_shape, adaptive_mesh_refinement, mesh_smoothing, export_mesh_abaqus/nastran, nonlinear_static/frequency/buckling_analysis, magnetostatic/coupled_thermo_mechanical/acoustic/poisson/diffusion equations, full post-processing (nodal values, interpolation, error estimate, result at point, surface integration, path result, reaction forces), fem_summary, export_fem_report, mesh quality, BC validation, computation time estimate. I/O: import_svg (7 SVG elements, path commands, transforms, ear-clipping), import_pdf (vector/text extraction), export_drawing_dxf (full TechDraw to DXF). 96 new tests (1133 total).

### UI Sprint: FreeCAD 100% Parity (2026-03-25)
Viewer UI completion achieving 576/576 FreeCAD feature parity (100%). GuiAction enum expanded from ~40 to 130+ variants across all 9 workbenches. `process_actions()` in `app.rs` connects all actions to backend crate calls. 9 fully functional workbench toolbars in `toolbar.rs` (Part, PartDesign, Sketcher, Mesh, TechDraw, Assembly, Draft, Surface, FEM). Full creation dialogs in `dialogs.rs` for all primitives and operations. Sketch UI in `sketch_ui.rs` with complete constraint visualization (24 types), configurable grid, snap indicators (7 types), B-spline display. Context menus in `context_menu.rs` for objects and viewport. New enum types: `AssemblyJointType` (13 joint types), `FemConstraintType` (6 constraint types). Backend integration covers FEM tet mesh generation + 6 analysis types + 9 equations, assembly constraint solving + simulation, TechDraw view/dimension/centerline/cosmetic tools, draft wire creation/modification/arrays, surface operations, and all 15+ I/O formats.

### UI Polish Sprint: Professional CAD Quality (2026-03-25)
Professional UI overhaul targeting FreeCAD/CATIA/SolidWorks/Fusion 360 quality levels. Theme system (`theme.rs`): `CadTheme` with 30+ color/spacing fields, Dark/Light presets, 3 density modes (Compact/Normal/Spacious), `apply_to_egui()` for full egui integration. Vector icon toolbar (`toolbar.rs`): 150+ `ToolIcon` variants drawn via `egui::Painter`, `icon_button()` (28×28), styled workbench tabs with accent underline. Hierarchical model tree (`tree.rs`): `EntityIcon` (14 types), `TreeNode` from construction history, tree guide lines, search/filter, inline rename, context menu. Enhanced dialogs (`dialogs.rs`): shared helpers (`dialog_section`, `param_field`, `validation_error`, `button_bar`), input validation with red errors, "mm" units, Defaults button. Properties panel (`properties.rs`): section headers, 3-column parameter grid, color picker with 8 presets, transparency slider. Status bar (`status_bar.rs`): mouse coords (left), tool hint (center), scene stats + FPS (right). Report panel (`report.rs`): numbered timestamps, severity filters, count badges, collapsible messages. Viewport overlays (`overlays.rs`): origin XYZ axes, 3D grid with fade, measurement tool, snap highlights. Navigation settings (`nav.rs`): persistent `theme_mode`, `ui_density`, overlay toggles.

**FEM Architecture (Sprint 3):**
Extended FEM module in `fem.rs` now supports 9 equation types: heat, flow, deformation, electrostatic, magnetostatic, acoustic, poisson, diffusion, coupled_thermo_mechanical. Six analysis types: static, nonlinear_static, frequency, buckling, modal, thermal. Mesh generation supports TetMesh and HexMesh with adaptive refinement and smoothing. Post-processing pipeline: extract_nodal_values -> interpolate_to_nodes -> compute_error_estimate for error-driven refinement. Result queries: result_at_point (barycentric interpolation), integrate_over_surface, path_result, reaction_forces. Export: Abaqus (.inp) and Nastran (.bdf) mesh formats. Quality assurance: check_mesh_quality_detailed returns per-element ElementQuality, check_boundary_conditions validates BC consistency, estimate_computation_time provides runtime estimates.

**SVG Import Architecture:**
`import_svg()` in `svg.rs` parses 7 SVG element types (rect, circle, ellipse, line, polyline, polygon, path). Path commands: M/L/H/V/C/S/Q/T/A/Z with relative variants. Transform parsing: matrix/translate/rotate/scale/skewX/skewY. Polygons triangulated via ear-clipping algorithm. Returns `Mesh` with vertices and triangle indices.

**Coons Patch:**
`coons_patch()` in `surface_ops.rs` implements bilinear blending surface from 4 boundary curves. Evaluates boundary curves at parameter t, blends using bilinear interpolation formula: S(u,v) = L1(u,v) + L2(u,v) - B(u,v) where L1/L2 are ruled surfaces along u/v and B is the bilinear correction. Returns NurbsSurface sampled on NxN grid.

---

## 5. API Design Principles

1. **Fallible operations return `KernelResult<T>`** — no panics on user-facing paths
2. **Prefer slices over owned Vecs** in function parameters
3. **Small types are `Copy`**, heap types are `Clone`
4. **`Display` for user-friendly printing** on all math types
5. **`From`/`Into` for natural conversions** between related types

---

## 6. Error Handling Patterns

- Map to `KernelError` variants via `.ok_or(KernelError::InvalidHandle(...))?`
- `?` operator chaining for clean error propagation
- `unwrap()` only on handles known to be valid (just-inserted)

---

## 7. Testing Strategy

| Layer | Type | Location | Description |
|-------|------|----------|-------------|
| Unit | `#[test]` | Each crate `src/*.rs` | Individual functions/structs |
| Integration | `#[test]` | `cadkernel/src/lib.rs` | E2E pipelines |
| Doc | `/// ` + ` ``` ` | prelude modules | API usage examples |

**1509 tests** across all crates. Run with `cargo test --workspace`.

### Integration Test Categories (stress_tests.rs)

The `crates/modeling/tests/stress_tests.rs` file contains 67 stress tests organized by workflow:

| Category | Tests | Coverage |
|----------|-------|----------|
| Multi-feature PartDesign Body | 9 | pad→pocket→chamfer chain, suppress/reorder, mirror, linear-pattern, revolve |
| 10+ part assembly | 9 | 10-box assembly, BOM, placement, visibility, constraints, interference |
| 20+ constraint sketch | 10 | L-shape (19 constraints), symmetry, H/V distance, midpoint, arc-tangent |
| Boolean chain (5+ ops) | 6 | 5-cylinder-subtract, union cross, XOR chain, alternating 6-op chain |
| Full I/O round-trip | 10 | JSON, ASCII STL, binary STL, STEP, OBJ, glTF, PLY, BREP, parallel tessellation |
| Cross-domain workflow | 5 | sketch→extrude→check→tessellate→STL, assembly BOM, all primitives check |
| Multi-boolean chain | 3 | 10+ sequential union/subtract/intersect, degenerate input, empty compound |
| Large assembly | 2 | 50-component interference detection, BOM on large assembly |
| Complex sketch | 2 | 30+ constraints with tangent arcs, coincident chain stress |
| Pattern stress | 2 | linear/circular pattern with 64 instances |
| FEM & surface ops | 4 | tet mesh quality, modal analysis, ruled→extend→pipe chain, surface stress |
| I/O edge cases | 5 | binary STL zero-count header, OBJ missing normals, glTF multi-primitive, large mesh 100K+, round-trip consistency |

### IO Roundtrip Tests — V33

39 new integration tests in `crates/io/tests/format_roundtrip.rs` (total: 61). Empty/multi-solid roundtrips for STEP/BREP/JSON, STEP double-roundtrip consistency, new format coverage (glTF, 3MF, DAE, AMF, VRML, OCA, DXF, DWG), SVG/PDF export, native .cadk save/load, large mesh roundtrips (10-box STL, 900-vertex OBJ), cross-format triangle count consistency across 8 formats, mesh operations (harmonize/flip normals, watertight, scale, boolean, Platonic solids), and edge cases (negative coords, precision, special floats, binary STL size). IO roundtrip: 22 → 61 tests.

### Core Crate Tests — V33

39 integration tests in `crates/core/tests/core_comprehensive.rs`. All six `KernelError` variants are tested for construction, Display output, and predicate methods. `with_context` is verified for all string-body variants (context prepended) and `InvalidHandle` (passed through unchanged). `KernelResult` patterns cover `Ok` pass-through, `Err` carry, `?` propagation, `map`, and `map_err`. `From<std::io::Error>` conversion is tested for `NotFound`, `PermissionDenied`, and `?`-operator usage. `Clone + PartialEq`, `Send + Sync`, and `std::error::Error` trait implementation are also verified. Core crate: 10 → 49 tests.

### Viewer Crate Tests — V33

101 integration tests in `crates/viewer/tests/viewer_comprehensive.rs`. Tests cover `compute_aabb` (4 cases including empty input), `DisplayMode` / `Projection` / `StandardView` enums, `Camera` (projection toggle, snap to view, reset, fit to bounds, eye position, matrix shapes, screen right/up unit length), `NavConfig` (scroll/drag zoom factors, resolve_drag for FreeCAD/Blender/Maya styles, snap_3d), all label/description arrays for `NavStyle`/`OrbitStyle`/`RotationMode`/`UnitSystem`/`BgPreset`, `CreationParams` serde roundtrip (Box, Sphere), `ObjectGroup` field storage, and `Scene` headless management via `add_mesh_object` (27 tests covering add/remove, visibility, selection, ordering, hierarchy, active body, and group management). `ScriptEngine` is covered by 29 tests: engine creation, Lua value types, sandbox globals, cad table presence, all five primitives, multi-solid accumulation, clear/delete/list, measure volume, count faces, translate, get_models, syntax error handling, arithmetic, and local variables. Viewer crate: 50 → 151 tests.

### Topology Crate Tests — V32

33 integration tests in `crates/topology/tests/topology_comprehensive.rs` covering Tag/Naming (modified/merged/chained ops, NameMap CRUD, ShapeHistory evolution variants), ModelHistory undo/redo (basic cycle, cap, redo clearing, descriptions), geometry binding (curve/surface/trim/pcurve, dead handle safety), inner loops, wire ops, tagged entity constructors + lookups, faces_around_vertex, PropertyStore material/metadata, Color/Material presets, Handle from_raw_parts, EntityStore is_alive/get_mut/iter_mut/slot reuse, and validation edge cases (serialization roundtrip, invalid handle errors). Topology crate: 29 → 62 tests.

### Testing Strategy — V25 Additions

**Stress test philosophy**: Each stress test exercises a realistic, multi-step workflow that combines 3 or more crates. Edge case tests cover inputs that are valid but degenerate (zero-length, near-coincident, empty), ensuring no panics on user-facing paths.

**I/O round-trip tests**: Import a format, export the same model, import again, and compare vertex/triangle counts within a tolerance. Detects regressions in serialization precision.

**Python binding tests**: 74 integration tests in `crates/python/` covering `PyAssembly`, `PyFem`, draft ops, surface ops, and compound ops. Run separately: `cd crates/python && PYO3_PYTHON=/usr/bin/python3 cargo test`.

### Benchmarks

25 criterion benchmarks in `cadkernel-modeling`. Key results (release build):

| Operation | Time | Complexity |
|-----------|------|------------|
| `make_box` | ~8 µs | O(1) |
| `make_cylinder(32)` | ~72 µs | O(N) |
| `make_sphere(32×16)` | ~749 µs | O(N·M) |
| `tessellate_sphere(32×16)` | ~60 µs | O(F) |
| `boolean_union(box+box)` | ~44 µs | O(Fa·Fb) |
| `stl_write_binary(sphere)` | ~223 µs | O(T) |
| `bench_parallel_boolean` | ~18 µs | O(Fa·Fb / cores) |
| `bench_bvh_query_nearest` | ~3 µs | O(log N) |
| `bench_bvh_query_ray` | ~7 µs | O(log N) |
| `bench_pattern_parallel(64)` | ~210 µs | O(N / cores) |

Run: `cargo bench -p cadkernel-modeling`. See [Performance wiki](wiki/Performance.md) for full analysis.

---

## 8. Build & CI

**Requirements**: Rust 1.85+ (Edition 2024), nalgebra 0.33, glam 0.29.

```bash
# Full check pipeline
cargo fmt --all && \
cargo clippy --workspace --all-targets --all-features -- -D warnings && \
cargo test --workspace
```

---

## 9. Workbench Toolbar Architecture

The viewer uses a FreeCAD-inspired workbench system with 130+ `GuiAction` variants, 9 fully functional workbench toolbars, and complete backend integration.

- **`Workbench` enum**: `Part`, `PartDesign`, `Sketcher`, `Mesh`, `TechDraw`, `Assembly`, `Draft`, `Surface`, `FEM`
- **Common toolbar**: New, Open, Save, Undo, Redo, Fit All, Reset View
- **Workbench tab bar**: Switches active workbench context
- **Context toolbar**: Changes based on active workbench
  - **Part**: 13 primitives (Box/Cylinder/Sphere/Cone/Torus/Tube/Prism/Wedge/Ellipsoid/Helix/Circle/Ellipse/Point/Line shapes), Boolean Union/Subtract/Intersect (dialog), shape builder, convert to solid, Mirror/Scale/Shell/Fillet/Chamfer/Pattern/Thickness/Offset/Section, attachment, appearance, shape analysis, Measure/Check
  - **PartDesign**: Pad/Pocket/Revolve/Groove/Hole/Countersunk Hole, 10 additive/subtractive primitives (box/cylinder/sphere/cone/torus/helix/ellipsoid/prism/wedge + loft/pipe), Fillet/Chamfer/Draft/Shell/Mirror/Scale/Pattern, body ops (suppress/set tip/move feature/move to body), shape binders, sprocket, shaft design
  - **Sketcher**: 8 geometry tools (Line/Rectangle/Circle/Arc/Ellipse/BSpline/Polygon/Slot), B-spline tools (degree/knot multiplicity ops, insert knot, join curves), 7 constraint buttons, display options (13 visual toggles), sketch management (attach/reorient/merge/mirror), external projection, carbon copy
  - **Mesh**: Import/Export STL/OBJ/glTF, 15 mesh operations (Decimate/Subdivide/Fill Holes/Flip Normals/Smooth/Harmonize/Boolean Union/Intersect/Difference/Cut/Remesh/Repair/Trim/Scale/Section), analysis (curvature plot, watertight check, bounding box info, face info, regular solids, UV unwrap)
  - **TechDraw**: 7 view types (Front/Top/Right/Iso/3-View/Broken/Complex Section), 12 dimension types (Linear/Angular/Radius/Arc Length/Extent/Chamfer/Contextual/Chain/Coordinate), 6 centerline tools, 8 cosmetic tools (line/thread/vertex/circle/arc/parallel/perpendicular), formatting (FormattedDimension), page management (template/update/redraw/print), SVG/DXF export
  - **Assembly**: Insert Component, 13 joint types (Fixed/Revolute/Cylindrical/Slider/Ball/Distance/Angle/RackAndPinion/Screw/Belt/Gear/Parallel/Perpendicular), solve constraints (Newton-Raphson), simulate step, DOF analysis, preferences, export ASMT
  - **Draft**: 10 wire creation (fillet/circle/arc/ellipse/rectangle/polygon/bezier/arc_3pt/chamfer/point), 8 modification (move/rotate/scale/mirror/offset/trimex/stretch/edit), 5 array patterns (rectangular/circular/polar/path link/point link), 3 annotations (dimension/label/text), snap system (16 modes), query (length/area), layer management, upgrade/downgrade, wire-to-bspline
  - **Surface**: Ruled surface, Filling, Sections, Extend, Pipe, Coons patch, Curve on mesh
  - **FEM**: 4 mesh types (TetMesh/HexMesh/generate/from shape), 6 material presets (steel/aluminum/titanium/copper/concrete/cast iron + thermal + custom), 8 boundary conditions (4 structural + 4 thermal + body load + contact + initial temperature), 6 analysis types (static/nonlinear/frequency/buckling/modal/thermal), 9 equations (heat/flow/deformation/electrostatic/magnetostatic/acoustic/poisson/diffusion/coupled), post-processing (stress/strain tensors, principal stresses, safety factor, nodal values, error estimate, path result, reaction forces), export (Abaqus/Nastran), quality check
- **File menu**: Import/Export for STEP, IGES, DXF, PLY, 3MF, BREP, STL, OBJ, glTF, CADK, VRML, AMF, OCA, SVG, PDF, DWG, DAE
- **Dialog pattern**: `GuiAction` enum (130+ variants) → `gui.actions` vec → `process_actions()` in `app.rs`
- **Boolean dialog**: Floating `egui::Window` with DragValue parameters for second box size/offset
- **FEM types**: `AssemblyJointType` (13 variants), `FemConstraintType` (6 variants) for toolbar integration

## 10. Geometry Binding (Phase B05)

All 5 primitives now bind ideal geometry to their topology:

| Primitive | Face Surfaces | Edge Curves |
|-----------|--------------|-------------|
| **Box** | 6 `Plane` (one per face, u×v = outward normal) | 12 `LineSegment` |
| **Cylinder** | 2 `Plane` (caps) + 1 shared `Cylinder` (lateral) | 3N `LineSegment` |
| **Sphere** | 1 shared `Sphere` (all faces) | `LineSegment` (polygon approx) |
| **Cone** | `Plane` (bottom) + optional `Plane` (top, frustum) + `Cone` (lateral) | `LineSegment` |
| **Torus** | 1 shared `Torus` (all faces) | `LineSegment` |

Key infrastructure:
- **`EdgeCache`**: Tracks `Handle<EdgeData>` for edge deduplication + `all_edges()` retrieval
- **`bind_edge_line_segments()`**: Shared helper to bind `LineSegment` curves to all cached edges
- **`bind_face_surface(face, Arc<dyn Surface>, Orientation)`**: Associates ideal surface with face
- **`bind_edge_curve(edge, Arc<dyn Curve>, domain)`**: Associates ideal curve with edge

## 11. Trim Infrastructure (Phase B01–B04)

UV parameter-space trim boundaries for exact B-Rep faces.

| Component | Location | Description |
|-----------|----------|-------------|
| **ParametricWire2D** | `geometry/surface/parametric_wire.rs` | Closed 2D curve chain with winding-number containment, arc-length sampling, polyline |
| **FaceData trim** | `topology/face.rs` | `outer_trim: Option<ParametricWire2D>`, `inner_trims: Vec<ParametricWire2D>` |
| **EdgeData pcurve** | `topology/edge.rs` | `pcurve_left/right: Option<Arc<dyn Curve2D>>` — UV representation per adjacent face |
| **bind_face_trim()** | `topology/lib.rs` | Binds trim wires to face |
| **bind_edge_pcurve()** | `topology/lib.rs` | Binds UV pcurve to edge side |
| **Trimmed tessellation** | `io/tessellate.rs` | UV centroid filtering — triangles outside outer_trim or inside holes are excluded |
| *(Trim Demo removed)* | — | Was in `viewer/app.rs`, removed in Phase V11 cleanup |

## 12. Exact Boolean Operations (Phase B06-B14)

The boolean module (`crates/modeling/src/boolean/`) now includes face splitting for exact boolean results.

### Modules

| Module | Purpose |
|--------|---------|
| `face_split.rs` | Face splitting along SSI intersection curves |
| `trim_validate.rs` | Trim loop validation (winding, closure, containment) |
| `evaluate.rs` | Face classification-based boolean operations |
| `broad_phase.rs` | AABB overlap detection |
| `classify.rs` | Face inside/outside classification via ray casting |

### Key Functions

- `boolean_op_exact(model_a, solid_a, model_b, solid_b, op, tolerance)` — Boolean with face splitting preprocessing
- `split_solids_at_intersection(model_a, solid_a, model_b, solid_b, tolerance)` — Split faces along SSI curves
- `fit_ssi_to_nurbs(points, tolerance)` — Fit SSI point cloud to NURBS curve
- `fit_ssi_to_pcurve(params)` — Fit UV parameters to 2D pcurve
- `validate_trim(outer, holes, tolerance)` — Validate trim loop consistency
- `ensure_correct_winding(outer, holes)` — Fix winding order (outer=CCW, holes=CW)

### Algorithm Pipeline

1. **Broad phase**: Find overlapping face pairs via AABB intersection
2. **SSI computation**: For overlapping pairs with surface binding, compute intersection curves via marching
3. **Planar intersection**: For faces without surface binding, compute edge-plane intersection
4. **Curve fitting**: Fit SSI point clouds to NURBS curves (interpolation or approximation)
5. **Face splitting**: Clip intersection curves to face boundaries, split polygons at entry/exit points
6. **Classification**: Classify split sub-faces as inside/outside the other solid
7. **Assembly**: Select appropriate faces based on operation type (union/intersection/difference)

### Geometry Preservation

`copy_face_with_geometry()` preserves:
- Surface binding (face ↔ parametric surface)
- Edge curve binding (edge ↔ 3D curve)
- Trim loops (outer + inner UV boundaries)
- Persistent tags

## 13. Current Status & Next Steps

### Performance Architecture (V25)

CADKernel uses three tiers of parallelism:

| Tier | Mechanism | Where Used |
|------|-----------|------------|
| Operation-level | `rayon::par_iter()` | Boolean face classification, pattern clone loops |
| Spatial query | BVH (`bvh.rs`) | Boolean broad-phase, tessellation, ray picking |
| Geometry caching | `BasisCache` LRU | NURBS basis function evaluation |

**BVH API** (`crates/geometry/src/bvh.rs`):
- `Bvh::build(aabbs)` — SAH-based construction
- `Bvh::build_sah(aabbs)` — explicit SAH variant
- `Bvh::query_aabb(aabb)` — all overlapping entries
- `Bvh::query_aabb_parallel(aabb)` — rayon parallel variant
- `Bvh::query_nearest(point)` → `Option<usize>` — single nearest entry
- `Bvh::query_ray(origin, dir)` → `Vec<usize>` — all ray-intersecting entries
- `Bvh::refit(new_aabbs)` — dynamic scene update

**NURBS Caching** (`crates/geometry/src/curve/bspline_basis.rs`):
- `BasisCache` — thread-local LRU with 1024 slots keyed by `(knot_hash, degree, u)`
- `CachedNurbsCurve` — wraps `NurbsCurve`, routes `point_at()` through cache
- `NurbsSurface::evaluate_cached()` — per-call cache lookup for surface evaluation

**Python Bindings Architecture** (`crates/python/src/lib.rs`):
- PyO3 `#[pyclass]` wrapper types: `PyModel`, `PySolidHandle`, `PyMesh`, `PyMassProperties`, `PyGeometryCheck`, `PySketch`, `PyAssembly`, `PyFem`
- All methods return `PyResult<T>`, mapping `KernelError` to `PyRuntimeError`
- Build: `cd crates/python && PYO3_PYTHON=/usr/bin/python3 cargo build` (excluded from workspace)
- Python test: `cd crates/python && PYO3_PYTHON=/usr/bin/python3 cargo test`

**V16 Complete (2026-04-01)** — 1300 tests, 100% FreeCAD feature parity (576/576)

| Workbench | Coverage | Status |
|-----------|:--------:|--------|
| Part | 100% | 58/58 implemented |
| PartDesign | 100% | 53/53 implemented |
| Sketcher | 100% | 109/109 implemented |
| TechDraw | 100% | 114/114 implemented |
| Assembly | 100% | 23/23 implemented |
| Mesh | 100% | 35/35 implemented |
| Surface | 100% | 6/6 implemented |
| Draft | 100% | 80/80 implemented |
| FEM | 100% | 80/80 implemented |
| I/O | 100% | 18/18 formats |

**UI Sprint Summary (2026-03-25):**
- `GuiAction` enum expanded from ~40 to 130+ variants
- 9 workbench toolbars fully functional with backend integration
- All `process_actions()` handlers connect to actual backend crate calls
- Sketch UI: full constraint visualization, grid, snap indicators
- FEM integration: tet mesh generation, 6 analysis types, 9 equations
- Assembly integration: 13 joint types, Newton-Raphson solver, simulation
- All 15+ I/O formats accessible from File menu

**UI Polish Sprint (2026-03-25):**
- Professional theme system: `CadTheme` with Dark/Light + 3 density presets
- Vector icon toolbar: 150+ `ToolIcon` variants, `egui::Painter` rendering
- Hierarchical model tree: `EntityIcon` (14 types), `TreeNode` from history, search/filter, inline rename
- Enhanced dialogs: input validation, unit labels, shared helpers, Defaults button
- Properties panel: color picker (8 presets), transparency, 3-column parameter grid
- Status bar: mouse coords + tool hint + scene stats + FPS
- Report panel: timestamps, severity filters, count badges, collapsible messages
- Viewport overlays: origin axes (wgpu 3D pass, V11), grid (egui, viewport-clipped), measurement tool, snap highlights
- Context menus: viewport, object, face/edge with feature reorder support

**UI Polish Sprint 2 (2026-03-25):**
- Tree: inline eye icon visibility toggle, tip marker, suppression dimming
- Toolbar: `icon_button_active()` with accent bottom border for active tool feedback
- Keyboard shortcuts reference dialog (5 sections, striped grid, Help menu)
- Settings: Appearance section with theme (Dark/Light) and density (Compact/Normal/Spacious)
- About dialog: centered logo, striped info grid, comprehensive project info
- ComboView panel: resizable width (200-450px), accent separator

**UI Polish Sprint 3 (2026-03-25):**
- Transparent panel fix: `CompositeAlphaMode::Opaque` prevents Linux compositor blending artifacts
- Settings redesign: tabbed Preferences dialog (General/Display/Navigation/Appearance/Lighting)
- Dynamic background: 4 gradient presets with runtime shader rebuild via `GpuState::update_bg_preset()`
- NavConfig expansion: unit system, decimal places, background preset, selection colors, tessellation quality, auto-save, recent files
- New enums: `UnitSystem` (5 variants), `BgPreset` (5 variants incl. Custom)

**V11: Viewer UI Expansion (2026-03-25):**
- Task panel: 5 → 35 ActiveTask variants with inline parameter editing + real-time 3D preview (Primitives 10, PartDesign 11, Draft 6, Surface 2, FEM 1, Boolean 1, Scale 1); toolbar buttons open task panel instead of immediate execution
- Legacy popup dialogs replaced: all creation operations now use inline task panel with OK/Cancel flow instead of popup dialogs or immediate execution
- Sketcher toolbar completed: B-spline tools (Convert, Degree+/-, Insert Knot), Split/Mirror/External Projection/Carbon Copy, Block/HDistance/VDistance constraints
- Menu improvements: Measure Distance connected to measurement mode, Macro menu disabled with planned note, Workbench switcher menu, Origin/Grid3D toggles in Tools menu
- Origin overlay: wgpu-only axis rendering (removed duplicate egui overlay), Z axis extended to full length, show_origin controls wgpu axes independently from grid
- Grid overlay: egui grid clipped to viewport rect (clip_rect) — no longer bleeds through panels

**V11 Deep Overhaul (2026-03-26):**
- GuiAction processing: all 200+ variants with concrete backend implementations (zero stub handlers remaining)
- Workbench menus: 9 workbench-specific menu groups dynamically shown based on active workbench (Part/PartDesign/Sketcher/Mesh/TechDraw/Assembly/Draft/Surface/FEM)
- Sketch constraint visualization: enhanced rendering with dimension labels, color-coded satisfaction status, snap indicators, B-spline display, construction geometry distinction
- Context menus: workbench-aware operations per active workbench, toolbar tooltips on all icon buttons
- Menu bar: complete menu system with File/Edit/View/Tools/Help plus workbench-specific menus
- Interactive task panel: 35 `ActiveTask` variants (Primitives 10 + PartDesign 11 + Draft 6 + Surface 2 + FEM 1 + Boolean 1 + Scale 1) — toolbar buttons open inline parameter panel with real-time 3D preview instead of immediate execution; OK/Cancel confirm or discard
- Viewer file structure (16 files): mod.rs (1096), menu.rs (903), toolbar.rs (2945), tree.rs (897), properties.rs (757), sketch_ui.rs (785), dialogs.rs (1577), overlays.rs (524), view_cube.rs (825), context_menu.rs (393), task_panel.rs (785), theme.rs (386), status_bar.rs (215), report.rs (278), app.rs (5585), render.rs (2009)

**V15 Professional Interaction: 3D Gizmo, Clip Plane, Shortcuts & World Coords (2026-03-30):**
- 3D transform gizmo: GizmoMode (Translate/Rotate/Scale), screen-space axis projection, per-axis hover highlighting
- GPU clip plane: `clip_params` in WGSL shader, fragment discard behind plane, orange cut-edge highlight
- `world_position` in VertexOutput for per-fragment clip distance
- Keyboard shortcuts panel: `?`/F1 toggle, 5 categories, two-column grid
- Mouse world coordinates: Z=0 ground plane ray cast, status bar display

**V14 Interactive Selection: Box Select, Selection Gate & Nav Fix (2026-03-30):**
- Box selection / rubber band: left-drag draws selection rectangle, left-to-right = window (blue), right-to-left = crossing (green/dashed)
- `object_screen_aabb()`: projects mesh AABB 8 corners to screen space for intersection test
- `finish_box_select()`: window (fully inside) vs crossing (any overlap) semantics, Ctrl+drag additive
- Selection gate: `try_pick_entity()` respects `SelectionMode` (Solid/Face/Edge/Vertex), sets `selected_entity` per mode
- `first_face()`, `first_edge()`, `first_vertex()` helpers traverse BRep topology for sub-element selection
- FreeCADGesture nav fix: left-drag → box select (was incorrectly orbit), middle → orbit, matching real FreeCAD

**V13 FreeCAD Parity: Professional UI Overhaul (2026-03-30):**
- Preselection hover highlight: `hover_params` vec4 uniform in WGSL shader, GPU-side hover blending with PRESELECT_COLOR
- Hierarchical model tree: Body > Feature nesting, 14 entity icons, Tip marker, status indicators, expand/collapse, drag-drop hint
- Enhanced properties panel: collapsible groups, Placement editor (Position + Rotation), computed properties (Volume/Area/CoM), search/filter
- Flyout toolbar system: `flyout_button()` grouped dropdown buttons with persistent last-used tool, Part/PartDesign flyout grouping
- Quick Measure & status bar: selection mode indicator, preselection info, auto-dimension, navigation mode, snap/grid toggles
- Professional theme system: `CadTheme` struct (25+ colors), Dark/Light presets, `UiDensity` (Compact/Normal/Spacious)
- Sketch UI: 11 tools (Select/Line/Rect/Circle/Arc/Point/Ellipse/Polyline/Slot/BSpline/Polygon), cursor crosshair, enhanced constraint visualization
- 9 workbenches: Part, PartDesign, Sketcher, Mesh, TechDraw, Assembly, Draft, Surface, FEM
- SelectionMode (Solid/Face/Edge/Vertex), 13 AssemblyJointType, 6 FemConstraintType
- 5 NavStyle presets, 5 UnitSystem options, 5 BgPreset gradients (incl. Custom with color pickers)

**V12 Critical UI Overhaul (2026-03-27):**
- Unified task system: all menu/context-menu/toolbar primitive creation routed through ActiveTask inline panel — legacy popup dialog system eliminated
- `draw_create_dialogs()` gated behind `gui.active_task.is_none()` — no dual UI conflicts
- CAD format import fix: `load_mesh_file()` now routes STEP/IGES/BREP/DXF/PLY/3MF through correct importers (was silently rejecting all non-STL/OBJ)
- `FileLoadResult` enum: distinguishes BRep (STEP/IGES/BREP) vs mesh-only (DXF/PLY/3MF/STL/OBJ) import results
- Toolbar disabled states: `icon_button_disabled()` + `ToolbarContext` for context-aware button enabling/disabling
- `gated_button!` macro: DRY pattern for conditionally enabled/disabled toolbar buttons
- Part/PartDesign ops disabled when no selection; Undo/Redo disabled when stacks empty

**V16 Performance & Quality Sprint (2026-04-01):**
- Parallel boolean operations: rayon `par_iter()` for face-classification loop in `boolean_op()`
- BVH improvements: `query_aabb_parallel()` (rayon), `build_sah()` (Surface Area Heuristic), `refit()` (dynamic scenes)
- NURBS basis caching: `BasisCache` LRU cache (1024 entries), `CachedNurbsCurve`, `NurbsSurface::evaluate_cached()`
- Python bindings updated: Sprint 2/3 APIs (`make_cone/torus`, `PyAssembly`, `PyFem`), 74 Python integration tests
- Test expansion: 1136 → 1300 tests (+164), covering compound ops, join ops, surface ops, assembly solver, FEM, mesh, 9 file formats (DXF/PLY/3MF/BREP/VRML/AMF/OCA/COLLADA/DWG)

**V32 Sketch Interaction & Advanced Snap (2026-04-07):**
- Box selection: rubber band in Select tool — left→right (window, solid border, blue fill) vs right→left (crossing, dashed border, green fill); Ctrl adds to selection; selects points, lines, circles, arcs inside box
- Snap visual indicators: on-canvas markers near cursor — X (coincident/point), dashed H/V guidelines, triangle (midpoint), square (grid), circled-X (intersection, orange)
- Double-click constraint edit: `try_sketch_dimension_edit()` detects dimensional constraints on selected entity, opens popup pre-filled; `edit_constraint_index` enables in-place value modification
- Cursor shape: crosshair for drawing tools, PointingHand for hover in Select, Grabbing during drag
- Midpoint + intersection snap: `snap_sketch_coords()` extended with line midpoint detection and line-line intersection (parametric t/u ∈ [0,1]); `Intersection` auto-constraint kind with visual indicator

**V31 Sketch-to-Solid Pipeline & Dimension UX (2026-04-07):**
- Dimension input popup: `DimensionPopup` with `DimensionKind` (7 types — Distance/Radius/Angle/Length/H-Dist/V-Dist/Diameter); centered egui::Window with DragValue + OK/Cancel; Enter confirms, Escape cancels; value persisted to toolbar defaults
- Closed profile detection: `find_closed_loops()` traces line adjacency to find closed loops; translucent green fill (alpha 30) highlights extrudable regions in overlay
- Extrude direction arrow: green arrow from sketch centroid along work plane normal with arrowhead + distance label; only shown when closed profile exists
- Sketch axis labels: red X / green Y axis arrows with arrowheads and text labels at sketch origin; white origin dot

**V30 Sketch Visual Polish & Slot Tool (2026-04-07):**
- Sketch entity hover info: `sketch_hover_info()` in status bar shows Point(x,y), Line(length/angle), Circle(center/radius), Arc(center/radius/span), Ellipse(center/minor), B-Spline(degree/pts)
- Slot tool upgrade: 3-click flow (center 1, center 2, width) creates proper stadium shape — 2 parallel lines + 2 semicircular arcs (closed slot geometry)
- Smooth B-spline rendering: `de_boor_eval()` + `clamped_uniform_knots()` evaluates actual B-spline curve at 4N+16 samples; control polygon displayed as dashed lines with diamond markers
- Sketch toolbar: Copy, Paste, Merge Pts buttons added to sketcher toolbar

**V29 Sketch Tool Completion & Validation (2026-04-07):**
- B-spline tools: ConvertToBSpline, IncreaseDegree, DecreaseDegree, InsertKnot all wired to `bspline_tools` functions
- External projection + carbon copy: project model vertices onto sketch, copy previous sketch into current
- Sketch copy/paste: Ctrl+C/V with centroid-relative clipboard storage
- Point merge: `SketchMergePoints` remaps all entity references to merged coincident points
- Validation overlay: `validate_sketch()` per frame, warning icons for zero-length lines, near-coincident points, over-constrained status
- Zero sketch stubs remaining — all 6 action stubs replaced with real implementations

**V28 Sketch Construction Mode & Auto-Constraints (2026-04-07):**
- Construction geometry toggle: selected entities toggle between construction/normal; global construction_mode marks new geometry as construction (dashed display, excluded from profile)
- Mirror geometry: `SketchMirrorGeometry` wired to `mirror_elements()` — select axis line + optional points
- Polyline close: Enter/right-click closes polyline loop (3+ points); Enter finalizes B-spline
- Constraint color-coding complete: all 12 constraint types use per-constraint residual color (green/yellow/red)
- Auto-constraints: `find_or_create_point()` reuses nearby points (coincident snap); `apply_line_auto_constraints()` adds H/V for near-axis lines; Rectangle auto-adds H/V on all edges

**V27 Sketch Precision Editing (2026-04-06):**
- Constraint-aware dragging: single-point drag uses `drag_solve()` to maintain constraints; falls back to raw move if solver doesn't converge; multi-point entity drags still delta-based
- Full undo/redo: `SketchSnapshot` stores full `Sketch` clone; both undo and redo correctly restore complete sketch state including deletions and constraint changes
- Interactive trim/split/extend: `SketchTrimEdge` (2 lines → trim at intersection), `SketchSplitEdge` (1 line → split at t=0.5), `SketchExtendEdge` (1 line → extend 50%)
- Fillet/chamfer corners: `SketchFilletCorner`/`SketchChamferCorner` for 2 lines sharing a vertex
- Ctrl+A select all: selects all sketch entities in sketch mode; falls through to global SelectAll outside
- Configurable grid spacing: `grid_spacing` field on `SketchMode`, DragValue in toolbar (0.1–10.0), grid rendering and snap use it

**V26 Sketch Advanced Editing (2026-04-06):**
- Entity dragging: click+drag on lines/circles/arcs moves all constituent points as a unit; `entity_drag_points()` collects indices; delta-based movement via `drag_origin`
- Constraint solver feedback: `constraint_residuals()` computes per-constraint L2 norms; indicators color-coded green/yellow/red; banner turns red on violation
- Sketch re-editing: `last_sketch` stores data on close; `EditSketch` action reopens in Select mode; toolbar shows "Edit Sketch" button
- Numeric constraint input: DragValue inputs for Distance/Angle/Radius inline in toolbar

**V25: Performance, Testing & Python Sprint (2026-04-08):**
- Parallel boolean: `boolean_op()` face-classification loop uses `rayon::par_iter()` — significant speedup on 4+ cores
- Parallel patterns: `linear_pattern()` and `circular_pattern()` clone loops parallelized with rayon
- BVH: `query_nearest()` (single nearest AABB entry) and `query_ray()` (all entries intersecting ray) added to `Bvh` in `geometry/bvh.rs`
- BVH allocation: stack-allocated node traversal buffer reuse reduces per-query heap allocations
- NURBS caching: `BasisCache` LRU cache (1024 entries), `CachedNurbsCurve`, `NurbsSurface::evaluate_cached()` in `bspline_basis.rs`
- Benchmarks: 4 new benchmarks — `bench_parallel_boolean`, `bench_bvh_query_nearest`, `bench_bvh_query_ray`, `bench_pattern_parallel` (25 total)
- Stress tests: 18 new tests in `stress_tests.rs` — multi-boolean chain, 50-part assembly, 30+ constraint sketch, 64-instance pattern, FEM quality, surface chain, I/O edge cases (67 total)
- Python: `PyAssembly` (5 methods), `PyFem` (4 methods), draft/surface/compound op bindings; 74 Python integration tests
- I/O: round-trip, empty model, 100K+ mesh, and format-specific edge case tests

**V25 Sketch Interactive Editing (2026-04-06):**
- Point dragging: click+drag in Select mode moves points with snap, undo snapshot on press, cancel if no movement
- Hover preselection: `hit_test_sketch()` on CursorMoved updates `hovered_entity`, rendered green (rgb 100,255,150)
- Redo: Ctrl+Shift+Z restores from `redo_stack`; Escape clears pending geometry before cancelling sketch
- Right-click context menu: `draw_sketch_context_menu()` popup with Delete/Horizontal/Vertical/Fixed/Select All/Clear Selection; context-sensitive constraint items
- DOF indicator: `degrees_of_freedom()` (per-constraint-type weighting) shown in banner and status bar; banner turns green when fully constrained; selection count in banner + status bar

**Future Focus Areas:**
| Priority | Focus | Key Items |
|----------|-------|-----------|
| High | Performance | Large assembly optimization, GPU tessellation |
| Medium | Real-world Testing | Complex model validation, STEP roundtrip fidelity |
| Medium | Documentation | API reference generation, tutorials |

---

## 15. Extension Architecture

CADKernel supports a plugin system that allows third-party extensions to register commands and hook into the modeling pipeline.

### Plugin Trait

```rust
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn init(&mut self) -> KernelResult<()>;
    fn shutdown(&mut self);
    fn commands(&self) -> Vec<String>;
    fn execute_command(&self, cmd: &str, args: &[String]) -> KernelResult<String>;
}
```

### PluginRegistry

`PluginRegistry` owns a `Vec<Box<dyn Plugin>>` and provides:

| Method | Description |
|--------|-------------|
| `register(plugin)` | Calls `init()`, stores plugin |
| `unregister(name)` | Calls `shutdown()`, removes plugin |
| `execute_command(name, cmd, args)` | Dispatches to named plugin |
| `list_plugins()` | Returns names of all registered plugins |

### Built-in Example Plugins

| Plugin | Commands | Purpose |
|--------|----------|---------|
| `ValidationPlugin` | `validate` | Runs `check_geometry` + `check_watertight` on active model |
| `AutoNamingPlugin` | `auto_name` | Assigns sequential names to unnamed solids |
| `StatisticsPlugin` | `stats` | Reports V/E/F counts, volume, surface area |

---

## 16. MCP Integration

CADKernel implements a **Model Context Protocol (MCP)** server that exposes the kernel to AI assistants via JSON-RPC 2.0.

### Protocol

- Transport: stdin/stdout (line-delimited JSON)
- Request format: `{ "jsonrpc": "2.0", "id": N, "method": "<tool>", "params": { ... } }`
- Response format: `{ "jsonrpc": "2.0", "id": N, "result": { ... } }` or `"error": { ... }`

### McpServer

```rust
pub struct McpServer {
    model: BRepModel,
}

impl McpServer {
    pub fn handle_request(&mut self, request: &str) -> String;
}
```

### Supported Tools (8)

| Tool | Params | Description |
|------|--------|-------------|
| `create_primitive` | `shape`, `params` | Box/Cylinder/Sphere/Cone/Torus with dimensions |
| `boolean_operation` | `op`, `target`, `tool` | Union/Subtract/Intersect by solid name |
| `transform` | `solid`, `tx`, `ty`, `tz`, `rx`, `ry`, `rz` | Translate + rotate solid |
| `query_model` | — | Returns all solid names, counts |
| `measure` | `solid` | Volume, surface area, bounding box |
| `export_model` | `format`, `path` | STL/OBJ/glTF/STEP/BREP export |
| `delete_solid` | `solid` | Remove named solid from model |
| `list_solids` | — | Array of solid name strings |

### Usage

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"create_primitive","params":{"shape":"box","params":{"x":10,"y":10,"z":10}}}' | cadkernel --mcp
```

---

## 17. Scripting

CADKernel embeds a Lua scripting engine for automation and batch processing.

### Security Sandbox

The scripting engine removes dangerous Lua standard library globals on initialization:
`os`, `io`, `require`, `dofile`, `loadfile`, `package`. Scripts have access to `string`,
`table`, `math`, and the `cad.*` API only. File I/O is only available through the `cad.export()`
and `cad.import()` functions which operate within validated paths.

### API Surface

```lua
-- Primitives
local s = cad.make_box(10, 10, 10)
local c = cad.make_cylinder(5, 20)

-- Booleans
local result = cad.union(s, c)
local result = cad.subtract(s, c)
local result = cad.intersect(s, c)

-- Transforms
cad.translate(s, 10, 0, 0)
cad.rotate(s, 0, 0, 90)   -- degrees around Z

-- Features
cad.fillet(s, edge_index, 1.0)
cad.chamfer(s, edge_index, 1.0)
cad.extrude(profile, 20.0)

-- I/O
cad.export(s, "output.stl")
cad.export(s, "output.step")

-- Query
local vol = cad.volume(s)
local area = cad.surface_area(s)
print(cad.list_solids())
```

### Execution API

```rust
pub struct LuaEngine {
    lua: mlua::Lua,
    model: Arc<Mutex<BRepModel>>,
}

impl LuaEngine {
    pub fn execute(&self, script: &str) -> KernelResult<String>;
    pub fn execute_file(&self, path: &Path) -> KernelResult<String>;
}
```

### Running Scripts

```bash
cadkernel --script my_model.lua
cadkernel --script batch_export.lua --output /tmp/
```

---

## 18. CI/CD

### CI Workflow (`.github/workflows/ci.yml`)

Triggers on push to `main` and all pull requests.

**Matrix**: `ubuntu-latest`, `macos-latest`, `windows-latest`

**Steps per platform**:
1. `actions/checkout@v4`
2. `dtolnay/rust-toolchain@stable` (components: rustfmt, clippy)
3. `actions/cache@v4` — `~/.cargo/registry/`, `~/.cargo/git/`, `target/`
4. `cargo build --workspace`
5. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
6. `cargo test --workspace`

**Additional job**:
- `python-bindings` (ubuntu-latest) builds and tests `crates/python` explicitly via `cargo build/test --manifest-path crates/python/Cargo.toml`

Cache keys are keyed on `Cargo.lock` hash per OS, with OS-level fallback restore keys.

### Release Workflow (`.github/workflows/release.yml`)

Triggers on tag push matching `v*` (e.g. `v0.2.0`).

**Build matrix**:

| OS | Target |
|----|--------|
| ubuntu-latest | x86_64-unknown-linux-gnu |
| macos-latest | x86_64-apple-darwin |
| macos-latest | aarch64-apple-darwin |
| windows-latest | x86_64-pc-windows-msvc |

**Steps per target**:
1. Checkout + install toolchain with target
2. Cargo registry + build cache (keyed on target + `Cargo.lock`)
3. `cargo build --release --target <target>`
4. `actions/upload-artifact@v4` — artifact name `cadkernel-<target>`

**Release job** (after all builds pass):
1. `actions/download-artifact@v4` — downloads all platform artifacts
2. `softprops/action-gh-release@v2` — creates GitHub release with all binaries, auto-generates release notes

### Python Wheel Workflow (`.github/workflows/python-release.yml`)

Triggers on tag push matching `v*` and manual dispatch.

**Build matrix**:

| Job | OS | Target(s) |
|-----|----|-----------|
| linux | ubuntu-latest | x86_64, aarch64 |
| macos | macos-13 / macos-latest | x86_64-apple-darwin, aarch64-apple-darwin |
| windows | windows-latest | x86_64-pc-windows-msvc |
| sdist | ubuntu-latest | source distribution |

**Steps per target**:
1. Checkout + `actions/setup-python@v5` (Python 3.13)
2. `PyO3/maturin-action@v1` — builds wheels in `crates/python/dist`
3. `actions/upload-artifact@v4` — artifact per platform

**Publish job** (after all builds pass, tags only):
1. Download all wheel artifacts
2. `pypa/gh-action-pypi-publish@release/v1` — publish to PyPI using trusted publishing

### Running Locally

```bash
# Full CI check (same as GitHub Actions)
cargo build --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

# Release build for current platform
cargo build --release
```

---

## 19. Lua Console

The Lua Console is an interactive panel tab in the Report panel (bottom dock). It allows running Lua code against the live model without leaving the GUI.

### Layout

```
[ Report | Lua Console ]       ← tab strip

┌─────────────────────────────────────────────────────┐
│ > local s = cad.make_box(10,10,10)                  │  ← output area (scrollable)
│ > cad.export(s, "/tmp/out.stl")                     │
│ Exported: /tmp/out.stl                              │
└─────────────────────────────────────────────────────┘
[ input field ................................ ] [Run]  ← input row
```

### GuiAction Variants

| Variant | Trigger | Description |
|---------|---------|-------------|
| `ExecuteLuaCode(String)` | Run button / Enter | Passes code string to `LuaEngine::execute()` |
| `ExecuteLuaFile(PathBuf)` | File picker | Loads file then calls `LuaEngine::execute_file()` |
| `ClearLuaConsole` | Clear button | Empties the output buffer |

### History Navigation

- Up/Down arrow keys in the input field cycle through previous commands (same session).
- History is not persisted across restarts.

### Error Display

Errors from `LuaEngine::execute()` are printed in red in the output area, prefixed with `Error: `.

---

## 20. Project Templates

Project templates are pre-built `.cadk` files shipped with the application. They are accessible via **File → New from Template**.

### Available Templates

| File | Contents | Use Case |
|------|----------|----------|
| `template_empty.cadk` | Empty model, valid schema | Starting a new design from scratch |
| `template_single_box.cadk` | 10×10×10 mm box | Quick geometry test or tutorial |
| `template_basic_assembly.cadk` | Two-component assembly skeleton | Assembly workflow starting point |
| `template_mechanical_part.cadk` | Flanged bracket with fillets | Mechanical design reference |
| `template_gear_demo.cadk` | Parametric spur gear (m=2, z=20) | Gear design and modification |

### Template Location

Templates are stored in the `templates/` directory at the repository root and are embedded into the release binary via the build script. At runtime they are accessible through:

```rust
cadkernel_io::templates::list_templates() -> Vec<TemplateInfo>
cadkernel_io::templates::load_template(name: &str) -> KernelResult<BRepModel>
```

---

## 21. Convenience API

`cadkernel-modeling` exposes a set of `quick_*` functions for concise scripting and test code. These wrap the underlying validated constructors with a simpler positional-argument signature.

### Primitive Constructors

```rust
use cadkernel_modeling::quick::*;

let b  = quick_box(10.0, 20.0, 5.0)?;          // x, y, z
let cy = quick_cylinder(5.0, 20.0)?;            // radius, height
let sp = quick_sphere(8.0)?;                    // radius
let co = quick_cone(4.0, 2.0, 15.0)?;           // r_bottom, r_top, height
let to = quick_torus(10.0, 2.0)?;               // major_r, minor_r
```

### Boolean Helpers

```rust
let result = quick_union(solid_a, solid_b)?;
let result = quick_subtract(solid_a, tool)?;
let result = quick_intersect(solid_a, solid_b)?;
```

### Mass-Property Queries

```rust
let vol  = quick_volume(&solid)?;               // f64 in mm³
let area = quick_area(&solid)?;                 // f64 in mm²
let cen  = quick_centroid(&solid)?;             // Point3
let bb   = quick_bbox(&solid)?;                 // BoundingBox
```

All functions return `KernelResult<T>` and carry descriptive error messages that include the function name and any out-of-range parameter values.

---

## 22. Example Scripts

Example scripts live in the `examples/` directory at the repository root.

### Directory Structure

```
examples/
├── lua/
│   ├── hello_cad.lua            — geometry creation + STL export
│   ├── boolean_operations.lua   — union / subtract / intersect pipeline
│   ├── parametric_part.lua      — dimension-driven bracket model
│   ├── batch_export.lua         — export one solid to STL + OBJ + STEP
│   └── assembly.lua             — two-part assembly with constraints
├── python/
│   ├── basic_modeling.py        — PyO3 primitives + boolean
│   └── batch_analysis.py        — mass-property loop over file list
└── mcp/
    └── session.json             — annotated JSON-RPC 2.0 session transcript
```

### Running Lua Scripts

**From the GUI**: Open the Lua Console tab (Report panel) → click the folder icon → select a `.lua` file. The script runs against the current live model.

**From the CLI**:

```bash
cadkernel --script examples/lua/hello_cad.lua
cadkernel --script examples/lua/batch_export.lua --output /tmp/
```

### Running Python Scripts

```bash
# Requires the cadkernel Python package (maturin build)
cd crates/python
PYO3_PYTHON=/usr/bin/python3 cargo build
maturin develop
python3 ../../examples/python/basic_modeling.py
```

### MCP Session Replay

`examples/mcp/session.json` is an annotated transcript showing the request/response pairs for a complete create → boolean → export workflow. It can be replayed against a running MCP server:

```bash
cadkernel --mcp < examples/mcp/session.json
```

---

## 14. Glossary

| Term | Description |
|------|-------------|
| **B-Rep** | Boundary Representation — solids defined by boundary faces |
| **Half-Edge** | Directed edge; each edge has two half-edges (twin pair) |
| **Handle<T>** | Entity reference with index + generation for stale detection |
| **Tag** | Persistent name for parametric rebuild entity tracking |
| **NURBS** | Non-Uniform Rational B-Spline |
| **TNP** | Topology Naming Problem |
| **AABB** | Axis-Aligned Bounding Box |
| **SSI** | Surface-Surface Intersection |
