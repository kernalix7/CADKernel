# Changelog

**English** | [한국어](CHANGELOG.ko.md)

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project aims to follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

#### Phase 1: Foundation
- Cargo workspace structure (7-crate monorepo)
- `cadkernel-math`: Vec2/3/4, Point2/3, Mat3/4, Transform, Quaternion, Ray3, BoundingBox, Tolerance
- `cadkernel-geometry`: Curve/Surface traits + Line, Arc, Circle, Ellipse, NURBS implementations
- `cadkernel-topology`: Half-edge B-Rep data structure, EntityStore, Handle<T>
- Version banner utility (`version_banner`) with unit test
- CI pipeline via GitHub Actions (`ci.yml`: fmt, clippy, test)
- Apache 2.0 `LICENSE` file
- Comprehensive bilingual documentation set (README, SECURITY, CODE_OF_CONDUCT, CONTRIBUTING, CHANGELOG)
- Initial `.gitignore` for Rust development

#### Phase 2: Persistent Naming + Boolean
- Persistent Naming system (Tag, NameMap, ShapeHistory, OperationId)
- Geometry-Topology binding (Edge.curve, Face.surface) via feature flag
- Surface-Surface Intersection (Plane-Plane, Plane-Sphere, Plane-Cylinder, Sphere-Sphere)
- Line-Surface Intersection (Line vs Plane, Sphere, Cylinder)
- Boolean operations (Union, Subtract, Intersect) with Broad Phase + Classify + Evaluate pipeline

#### Phase 3: Parametric + Sketch + I/O
- 2D parametric sketch system with 14 constraint types
- Newton-Raphson constraint solver with Armijo backtracking
- Feature operations (Extrude, Revolve) with auto-tagging
- Primitive builders (Box, Cylinder, Sphere)
- Tessellation (Face/Solid → Triangle Mesh)
- STL export (ASCII + Binary), OBJ export
- E2E integration tests (Sketch → Extrude → STL, Sketch → Revolve → OBJ, Persistent Naming)

#### Phase 4: Core Hardening
- `cadkernel-core` crate: shared KernelError/KernelResult types
- All public `assert!`/`expect()` converted to `KernelResult` (panic-free public API)
- `Arc<dyn Curve + Send + Sync>` / `Arc<dyn Surface + Send + Sync>` for thread safety
- Math type standard traits: Default, Display, From, AddAssign/SubAssign/MulAssign, Sum
- Full point-vector operators: `Point - Vec`, `f64 * Vec`, `From<[f64;N]>`, `From<Vec3> for Point3`
- `EntityStore::len()` optimized from O(n) to O(1)
- `IntersectionEllipse` rename to resolve name collision with curve `Ellipse`
- `PartialEq` + `Copy` added to all value-type geometry structs
- NURBS safety: empty control_points guard, tangent division-by-zero guard
- `WireData`: standalone half-edge chain with Persistent Naming integration
- Topology: validation, 5 traversal helpers, transform
- Prelude modules across all crates
- Developer Wiki guide (Korean/English)

#### Phase 5: Mass Properties + Sweep
- `cadkernel-modeling`: `MassProperties` struct (volume, surface area, centroid)
- `cadkernel-modeling`: `compute_mass_properties()` — divergence theorem-based mesh volume/area calculation
- `cadkernel-modeling`: `solid_mass_properties()` — convenience function for B-Rep solids
- `cadkernel-modeling`: Sweep operation (profile × path → solid)
- Sweep: rotation-minimizing frame (RMF) propagation, automatic Persistent Naming
- GitHub Wiki documentation restructured (13 pages: Architecture, per-crate guides, Cookbook, etc.)

#### Phase 6: Loft + Pattern
- `cadkernel-modeling`: Loft operation (N cross-section profiles → interpolated solid, cap_start/cap_end control)
- `cadkernel-modeling`: Linear Pattern (direction + spacing + count → repeated copies)
- `cadkernel-modeling`: Circular Pattern (axis + angle + count → rotational copies)
- Solid deep-copy infrastructure (`copy_solid_with_transform`)
- Persistent Naming auto-tagging for all pattern instances

#### Phase 7: Chamfer + I/O Import
- `cadkernel-modeling`: Chamfer operation (edge bevel — adjacent face discovery + topology rebuild)
- `cadkernel-io`: STL Import (ASCII + Binary auto-detection, vertex deduplication)
- `cadkernel-io`: OBJ Import (v/vt/vn format parsing, N-gon fan triangulation)
- Full STL/OBJ bidirectional round-trip support

#### Phase 8: Modeling Enhancements (Mirror + Shell + Scale)
- `cadkernel-modeling`: Mirror operation (plane reflection copy)
- `cadkernel-modeling`: Shell operation (thin-wall / hollow solid)
- `cadkernel-modeling`: Non-uniform Scale operation
- Shared `copy_solid_with_transform` utility extracted from pattern.rs

#### Phase 9: Math & Geometry Enhancements
- `cadkernel-math`: 11 utility functions (distance, angle, projection, interpolation, area)
- `cadkernel-geometry`: Plane — `from_three_points`, `signed_distance`, `project_point`, etc.
- `cadkernel-math`: BoundingBox — `overlaps`, `expand`, `volume`, `surface_area`, `longest_axis`, `size`

#### Phase 10: Quality & Testing
- 10 E2E integration tests (full pipeline: model → export → import)
- B-Rep validation: dangling reference detection, orientation consistency check
- New API: `validate_manifold()`, `validate_detailed()`, `ValidationIssue`, `ValidationSeverity`

#### Phase 11: I/O Format Expansion
- `cadkernel-io`: SVG 2D Export (`SvgDocument`, 5 element types, auto-fit viewBox)
- `cadkernel-io`: JSON serialization (BRepModel ↔ JSON roundtrip, file I/O)
- serde `Serialize`/`Deserialize` on all topology and math types

#### Phase 12: Rustdoc Documentation
- Crate-level documentation (`//!`) on all crates
- Public API doc comments on all `pub` items

#### Phase 13: High Priority Features
- `cadkernel-modeling`: Fillet operation (`fillet_edge`) — arc-approximated edge rounding with configurable radius and segments
- `cadkernel-modeling`: Split Body operation (`split_solid`) — cuts solid into two halves using a cutting plane
- `cadkernel-modeling`: Point-in-Solid query (`point_in_solid`) — ray-casting based containment test returning `Inside`/`Outside`/`OnBoundary`

#### Phase 14: Geometry & Manufacturing
- `cadkernel-geometry`: 2D Curve Offset (`offset_polyline_2d`, `offset_polygon_2d`) — parallel offset for CNC/sketch workflows
- `cadkernel-modeling`: Draft Angle operation (`draft_faces`) — mold taper with configurable pull direction and neutral plane
- `cadkernel-geometry`: Adaptive Tessellation (`TessellationOptions`, `adaptive_tessellate_curve`, `adaptive_tessellate_surface`, `TessMesh`) — chord-error and angle-based subdivision
- `cadkernel-geometry`: `TessellateCurve` / `TessellateSurface` extension traits for convenient per-object tessellation

#### Phase 15: Infrastructure
- `cadkernel-topology`: Undo/Redo system (`ModelHistory`) — snapshot-based undo/redo with configurable max depth
- `cadkernel-topology`: Property System (`Color`, `Material`, `PropertyValue`, `PropertyStore`) — entity metadata with material presets (Steel, Aluminum, ABS, Wood)
- `cadkernel-modeling`: Closest Point Query (`closest_point_on_solid`) — Voronoi-region triangle projection returning `ClosestPointResult` (point, distance, face)

#### Phase 16: Industry Formats
- `cadkernel-io`: STEP I/O (`StepWriter`, `read_step_points`, `parse_step_entities`, `export_step_mesh`) — ISO 10303-21 subset (AP214)
- `cadkernel-io`: IGES I/O (`IgesWriter`, `read_iges_points`, `read_iges_lines`) — IGES 5.3 fixed-width 80-column format for basic geometry exchange

#### Phase 17: Quality & Advanced
- `cadkernel-modeling`: Benchmark Suite — 9 criterion benchmarks (primitives, boolean, extrude, sweep, pattern, STL write, mass props)
- `cadkernel-geometry`: NURBS Advanced — knot insertion (Boehm algorithm), degree elevation — shape-preserving refinement
- Compile-time `Send + Sync` assertions across all crates (math, core, geometry, topology, io)

#### Application Phase 1: Native GUI Application
- `cadkernel-viewer`: Full native desktop GUI application (egui 0.31 + wgpu 24.x + winit 0.30)
- `cadkernel-viewer`: wgpu rendering pipeline with 4 display modes (Solid, Wireframe, Transparent, Flat Lines)
- `cadkernel-viewer`: 3 render pipelines (solid, wireframe/line, transparent) with dynamic uniform buffer offsets
- `cadkernel-viewer`: Orbit camera system (yaw/pitch/distance, 360° rotation, screen-aligned pan, scroll zoom)
- `cadkernel-viewer`: Perspective and Orthographic projection toggle
- `cadkernel-viewer`: Standard view presets (Front, Back, Right, Left, Top, Bottom, Isometric)
- `cadkernel-viewer`: Configurable mouse navigation presets (FreeCAD Gesture, Blender, SolidWorks, Inventor, OpenCascade)
- `cadkernel-viewer`: Settings dialog for navigation style and sensitivity customization
- `cadkernel-viewer`: Dynamic grid overlay (auto-scaling 1-2-5 spacing based on zoom level, minor/major line distinction)
- `cadkernel-viewer`: XYZ origin axes rendering (R/G/B colored)
- `cadkernel-viewer`: Dark theme gradient background
- `cadkernel-viewer`: Mini axes indicator (egui overlay, bottom-left corner)
- `cadkernel-viewer`: egui UI panels (menu bar, model tree, properties inspector, status bar)
- `cadkernel-viewer`: Shape creation dialogs (Box, Cylinder, Sphere with parameter input)
- `cadkernel-viewer`: File open/save/export dialogs (native file dialogs via rfd)
- `cadkernel-viewer`: Asynchronous background file loading (no UI freeze)
- `cadkernel-viewer`: FreeCAD-style keyboard shortcuts (1/3/7=views, Ctrl+1/3/7=reverse views, 5=projection, D=display, V=fit, G=grid)
- `cadkernel-io`: glTF 2.0 export (embedded base64, per-vertex normals, min/max bounds)
- `cadkernel-io`: Multi-threaded STL/OBJ parsing with rayon (O(N) HashMap vertex deduplication replacing O(N²) linear search)
- `cadkernel-io`: Multi-threaded glTF export, tessellation, and bounding box computation
- `cadkernel-python`: Python bindings via PyO3 (BRepModel, primitives, I/O, mass properties)

#### Application Phase 2: ViewCube & Camera Enhancements
- `cadkernel-viewer`: ViewCube — truncated cube geometry (chamfered edges, 6 octagonal faces + 8 triangular corners + 12 shared edges)
- `cadkernel-viewer`: ViewCube — directional lighting (top-right-front light, ambient+diffuse shading)
- `cadkernel-viewer`: ViewCube — drop shadow, orbit ring with compass labels (F/R/B/L)
- `cadkernel-viewer`: ViewCube — face/edge/corner hover detection and click-to-snap (6+12+8 = 26 view directions)
- `cadkernel-viewer`: ViewCube — screen-space arrow buttons (▲▼◀▶, Rodrigues rotation for view direction computation)
- `cadkernel-viewer`: ViewCube — CW/CCW in-plane roll buttons (↺↻, screen-relative clockwise/counter-clockwise rotation)
- `cadkernel-viewer`: ViewCube — side buttons (Home, projection toggle P/O, FitAll)
- `cadkernel-viewer`: Camera roll system — in-plane rotation around view axis, auto-reset on view snap
- `cadkernel-viewer`: Camera animation system — smooth-step easing (3t²−2t³), shortest-path yaw interpolation
- `cadkernel-viewer`: View transition animation settings (enable/disable toggle, duration slider 0.1–1.0s)
- `cadkernel-viewer`: 45° orbit step for arrow/rotation buttons
- `cadkernel-viewer`: Mini axis indicator — negative direction faded lines, roll-aware rendering
- `cadkernel-viewer`: `rodrigues()` vector rotation utility (render.rs)
- `cadkernel-viewer`: ViewCube engraved face labels — TextShape rotation matching cube orientation
- `cadkernel-viewer`: Roll snap to nearest 90° on view snaps (face/edge/corner click)
- `cadkernel-viewer`: ViewCube dropdown menu (☰) — Orthographic/Perspective, Isometric, Fit All

#### Application Phase 3: Rendering & UI Overhaul
- `cadkernel-viewer`: 8 display modes (As Is, Points, Wireframe, Hidden Line, No Shading, Shading, Flat Lines, Transparent) — matching FreeCAD rendering options
- `cadkernel-viewer`: CW/CCW rotation icon direction fix (↺=CCW, ↻=CW matching positive roll convention)
- `cadkernel-viewer`: FreeCAD-style ViewCube enhancements — semi-transparent faces, XYZ axis indicator, edge selection, front-face-only hover
- `cadkernel-viewer`: Screen-space Rodrigues orbit — face-relative rotation with yaw/pitch/roll extraction (replaces direct yaw/pitch)
- `cadkernel-viewer`: Animation target snap — consecutive arrow presses chain correctly during ongoing animations
- `cadkernel-viewer`: Macro menu (placeholder: Console, Record, Execute)
- `cadkernel-viewer`: FreeCAD-style Settings dialog — 3D View (axes, FPS, projection), Navigation (ViewCube, orbit style, sensitivity, animation), Lighting (intensity, direction XYZ)
- `cadkernel-viewer`: NavConfig expanded with 10 new settings (show_view_cube, cube_size, cube_opacity, orbit_steps, snap_to_nearest, show_axes_indicator, show_fps, enable_lighting, light_intensity, light_dir)
- `cadkernel-viewer`: Blinn-Phong shading — specular highlights (configurable strength + shininess) with color clamping for realistic surface rendering
- `cadkernel-viewer`: Camera headlight — light source follows camera (upper-right offset) for real-time reflection updates on orbit
- `cadkernel-viewer`: GPU adapter fallback — HighPerformance → LowPower → software (llvmpipe/swiftshader) with backend logging
- `cadkernel-viewer`: Fixed mouse orbit direction — drag-right now orbits right (negated yaw/pitch delta)
- `cadkernel-viewer`: Fixed ViewCube face labels — FACE_TEXT_RIGHT matched to actual `cross3(f, up)` screen_right per view
- `cadkernel-viewer`: Crease-angle auto-smooth normals (60° threshold) — eliminates faceting artifacts on flat surfaces while preserving sharp edges (like Blender/FreeCAD)
- `cadkernel-viewer`: ViewCube edge chamfer quads — 12 edge bevels rendered as filled quads with per-edge lighting (replaces line segments), depth-sorted with faces/corners
- `cadkernel-viewer`: Smooth-group normals via BFS — transitive face grouping at each vertex within crease angle (60°), area-weighted normal accumulation (raw cross product sum, magnitude ∝ triangle area). Eliminates discontinuities from non-uniform mesh density while preserving sharp edges
- `cadkernel-viewer`: ViewCube single-mesh rendering — all non-hovered polygons rendered as one `epaint::Mesh` (fan-triangulated, no anti-aliasing feathering on internal edges). Eliminates visible seam lines between adjacent faces/edges/corners. Hovered polygon rendered separately with stroke highlight
- `cadkernel-viewer`: ViewCube opaque fill — XYZ axis indicator now renders ON TOP of cube polygons (eliminates double-blending artifacts from semi-transparent overlap)
- `cadkernel-viewer`: Face normals always computed from vertex positions — ignores stored STL normals for BFS grouping (eliminates seams from inconsistent/inverted file normals)
- `cadkernel-viewer`: 4x MSAA (Multi-Sample Anti-Aliasing) — eliminates triangle edge Mach band artifacts on smooth surfaces. MSAA color and depth textures with `sample_count=4`, all render pipelines updated, scene pass resolves to surface texture
- `cadkernel-io`: Tessellation vertex sharing — `tessellate_solid` now deduplicates vertices via bit-exact position matching (`f64::to_bits` HashMap), enabling cross-face smooth normal computation. Root fix for visible triangle edges on curved surfaces
- `cadkernel-io`: STL vertex deduplication precision fix — quantize changed from 1e8 to 1e4 (0.1mm tolerance) to properly merge float32-precision coincident vertices for correct smooth normals
- `cadkernel-viewer`: Direction-aware roll snap — at the 45° midpoint between two 90° multiples, snap direction follows previous roll position (e.g. 0°→45° snaps back to 0°, 90°→45° snaps back to 90°). Tracks `prev_roll` before `RollDelta` and `ScreenOrbit` actions
- `cadkernel-viewer`: Top/Bottom view yaw preservation — clicking Top/Bottom preserves current yaw (only pitch changes), preventing unwanted in-plane rotation at near-vertical views
- `cadkernel-viewer`: Roll angle normalization — `wrap_angle()` utility normalizes angles to (−π, π]. `snap_roll_90` normalizes both inputs before processing. `RollDelta` normalizes `camera.roll` after each button press, preventing unbounded accumulation (8× CW = 360° = 0°)
- `cadkernel-viewer`: ScreenOrbit `prev_roll` timing fix — saves `prev_roll` after animation target snap (not before), ensuring clean 90° target values instead of intermediate interpolated angles
- `cadkernel-viewer`: Higher default primitive tessellation — Cylinder 32→64 segments, Sphere 32×16→64×32 segments for smoother curved surfaces
- `cadkernel-io`: Native `.cadk` project format — human-readable JSON with format header (`CADKernel` + semver), backward-compatible with bare BRepModel JSON

