# CADKernel Developer Wiki

> **Version**: 0.1.0 (pre-alpha)  
> **Last updated**: 2026-03-16  
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

**GUI Module** (`gui/`): `mod.rs` (GuiState, GuiAction enum, draw_ui entry), `menu.rs` (File/Edit/Create/View/Tools/Help menu bar), `toolbar.rs` (common + 9 workbench toolbars), `tree.rs` (hierarchical B-Rep model tree), `properties.rs` (per-entity attribute panel), `status_bar.rs` (coordinates, FPS, mesh stats), `report.rs` (color-coded Info/Warning/Error log panel), `dialogs.rs` (11 creation dialogs + boolean + part ops), `sketch_ui.rs` (sketch overlay + grid label), `overlays.rs` (axes indicator + TechDraw overlay), `view_cube.rs` (truncated cube navigation), `context_menu.rs` (solid + viewport right-click menus).

**Report System**: `log_info()`/`log_warning()`/`log_error()` helper methods on `CadApp`. All file I/O, primitive creation, boolean, part operations, mesh operations, and analysis handlers log to the report panel via `gui.log(ReportLevel, msg)`. Status bar shows the latest message, report panel preserves full history.

**Camera**: Orbit (yaw/pitch/distance) + in-plane roll. View matrix applies roll rotation when non-zero. Screen-right/up methods are roll-aware. Roll snaps to nearest 90° on view transitions with direction-aware midpoint resolution (at 45°, snaps toward previous roll position via `prev_roll` tracking). Top/Bottom views preserve current yaw (only pitch changes). All roll angles normalized to (−π, π] via `wrap_angle()` — `snap_roll_90` normalizes inputs, `RollDelta` normalizes after each press, `ScreenOrbit` saves `prev_roll` after animation snap (not before).

**ViewCube**: Truncated cube with 26 depth-sorted polygons: 6 octagonal faces, 8 triangular corners, 12 edge bevel quads. Edge quads computed via face-normal offset from shared chamfer vertices (`EDGE_BEVEL=0.24`), with per-edge normals (`EDGE_NORMALS`). All non-hovered polygons rendered as a single `epaint::Mesh` (fan-triangulated) — eliminates egui anti-aliasing feathering seams between adjacent polygons. Opaque fill (`from_rgb`), XYZ axis indicator rendered ON TOP. Hovered polygon rendered separately via `convex_polygon` with stroke highlight. Face/edge/corner hover detection (point-in-polygon) and click-to-snap (26 view directions). Directional lighting, drop shadow, orbit ring with compass labels. Engraved face labels (TextShape rotation). Dropdown menu (☰) for projection/view shortcuts.

**Mesh Normals**: Smooth-group BFS algorithm (`mesh_to_vertices` in `render.rs`). At each vertex, faces are transitively grouped within crease angle (60°) via BFS. **Area-weighted** accumulation: raw cross products (magnitude ∝ triangle area) are summed, then normalized — large triangles contribute proportionally more, eliminating artifacts from non-uniform mesh density. Requires vertex sharing across faces — `tessellate_solid` deduplicates vertices via bit-exact `f64::to_bits` matching, STL import deduplicates via quantized keys (1e4 precision).

**4x MSAA**: All render pipelines use `MultisampleState { count: 4 }`. Scene pass renders to MSAA color+depth textures, then resolves to surface texture. Eliminates Mach band artifacts (visible triangle edges on smooth surfaces). egui pass remains sample_count=1 (2D UI on resolved surface).

**Navigation**: 5 presets (FreeCAD Gesture, Blender, SolidWorks, Inventor, OpenCascade). Camera animation with smooth-step easing (3t²−2t³).

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
Boolean: auto face-splitting in `boolean_op` (split→classify→evaluate), multi-sample classification (7-point majority voting). Sketch: DOF analysis via Jacobian rank, `drag_solve()` interactive point dragging. Viewer: Moller-Trumbore ray picking (`picking.rs`), undo/redo command stack (`command.rs`, `ModelSnapshot`-based).

### Phase V7: File Format Expansion
5 new format implementations: glTF import (base64 buffer decode, multi-component indices), 3MF import (XML parsing), DWG import/export (version detection R2000–2018+, 3DFACE heuristic, DXF fallback), PDF export (SVG→PDF 1.4 conversion for TechDraw), DAE Collada import/export (COLLADA 1.4.1 XML geometry). Total I/O formats: 15 (was 11). New modules: `collada.rs`, `dwg.rs`, `pdf.rs`.

### Phase V13: Performance & Validation
BVH-accelerated boolean broad-phase (O(n log n) face-pair overlap), 11 new benchmarks (25 total): primitives (cone, torus), features (mirror, scale, fillet), validation (check_geometry, check_watertight), stress (tessellate_sphere_64x32, tessellate_torus_64x32, boolean_intersection).

### Phase V12: Python Bindings
PyO3-based `cadkernel` Python module in `crates/python/` (standalone build, excluded from workspace). 6 classes (Model, SolidHandle, Mesh, MassProperties, GeometryCheck, Sketch), 10 primitive creators, 4 feature ops, 3 boolean ops, tessellation/analysis, 10 I/O functions, sketch system with 7 constraint types.

### FreeCAD-Level UI Overhaul
`gui.rs` (3605 lines) refactored into `gui/` module directory (12 files). Hierarchical model tree, property editor, full menu system, enhanced status bar, report panel with color-coded logging, solid/viewport context menus, 9 workbench toolbars (Part, PartDesign, Sketcher, Mesh, TechDraw, Assembly, Draft, Surface, FEM). Report logging added to 40+ action handlers.

**Current status**: 739 tests, 0 clippy warnings, 0 fmt diff.

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

**739 tests** across all crates. Run with `cargo test --workspace`.

### Benchmarks

14 criterion benchmarks in `cadkernel-modeling`. Key results (release build):

| Operation | Time | Complexity |
|-----------|------|------------|
| `make_box` | ~8 µs | O(1) |
| `make_cylinder(32)` | ~72 µs | O(N) |
| `make_sphere(32×16)` | ~749 µs | O(N·M) |
| `tessellate_sphere(32×16)` | ~60 µs | O(F) |
| `boolean_union(box+box)` | ~44 µs | O(Fa·Fb) |
| `stl_write_binary(sphere)` | ~223 µs | O(T) |

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

The viewer uses a FreeCAD-inspired workbench system:

- **`Workbench` enum**: `Part`, `PartDesign`, `Sketcher`, `Mesh`, `TechDraw`, `Assembly`
- **Common toolbar**: New, Open, Save, Undo, Redo, Fit All, Reset View
- **Workbench tab bar**: Switches active workbench context
- **Context toolbar**: Changes based on active workbench
  - Part: 10 primitives (Box/Cylinder/Sphere/Cone/Torus/Tube/Prism/Wedge/Ellipsoid/Helix), Boolean Union/Subtract/Intersect (dialog with second box), Mirror/Scale/Shell/Fillet/Chamfer/Linear Pattern, Measure/Check
  - Part Design: Pad/Pocket/Revolve/Fillet/Chamfer/Draft/Shell/Mirror/Scale/Pattern/Union/Subtract
  - Sketcher: Interactive mode — Line, Rectangle, Circle, Arc tools, constraints (H/V/Length), Close→Extrude
  - Mesh: Import/Export STL/OBJ/glTF, Decimate/Subdivide/Fill Holes/Flip Normals, Smooth/Harmonize Normals/Check Watertight/Remesh/Repair
  - TechDraw: Front/Top/Right/Iso, 3-View, Export SVG, Clear
  - Assembly: Insert Component, Fixed, Coincident, Concentric, Distance
- **File menu**: Import/Export for STEP, IGES, DXF, PLY, 3MF, BREP, STL, OBJ, glTF, CADK
- **Dialog pattern**: `GuiAction` enum → `gui.actions` vec → `process_actions()` in `app.rs`
- **Boolean dialog**: Floating `egui::Window` with DragValue parameters for second box size/offset

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

## 13. Next Steps

| Phase | Focus | Key Items |
|-------|-------|-----------|
| C | STEP I/O | AP203/AP214 import/export |
| D | Fillet/Draft/Split | Rolling-ball fillet, face draft, solid split |

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
