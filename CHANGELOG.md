# Changelog

**English** | [한국어](docs/CHANGELOG.ko.md)

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

#### Application Phase 4: ViewCube Polish + FPS
- `cadkernel-viewer`: ViewCube face octagon insetting — edge-adjacent vertices inset by EDGE_BEVEL so bevel strips are visible between adjacent faces
- `cadkernel-viewer`: ViewCube corner hexagons — 3-vertex corner triangles expanded to 6-vertex hexagons to match inset face edges
- `cadkernel-viewer`: FPS counter — 0.5s rolling-average FPS display in status bar (toggled via Settings > Show FPS)

#### Application Phase 5: Full Issue Fix + Workbench Toolbar
- `cadkernel-viewer`: FreeCAD-style workbench toolbar — Workbench enum (Part Design, Sketcher, Mesh, Assembly), common action toolbar (New/Open/Save/Undo/Redo), workbench tab bar, context-dependent tool toolbar
- `cadkernel-viewer`: NavConfig settings now applied — `cube_size` controls ViewCube size, `cube_opacity` controls fill transparency, `orbit_steps` controls arrow button step angle
- `cadkernel-viewer`: Simple viewer orbit direction fixed — negated dx/dy for natural orbit feel
- `cadkernel-viewer`: ScreenOrbit asin NaN guard — input clamped to [-1,1] before asin

#### Application Phase 7: FreeCAD Workbench System + New Primitives
- `cadkernel-modeling`: `make_cone()` primitive — pointed cone (apex) and frustum (truncated), parameterized by base_radius, top_radius, height, segments. EdgeCache dedup, full B-Rep topology with tests
- `cadkernel-modeling`: `make_torus()` primitive — ring-shaped solid, parameterized by major_radius, minor_radius, major/minor segments. Quad-mesh topology with EdgeCache dedup
- `cadkernel-viewer`: Workbench system expanded — 6 workbenches: Part (new), Part Design, Sketcher, Mesh, TechDraw (new), Assembly
- `cadkernel-viewer`: Part workbench — 5 primitives (Box, Cylinder, Sphere, Cone, Torus) + Boolean ops + Mirror/Scale placeholders
- `cadkernel-viewer`: Part Design workbench — reorganized with feature-based tools (Pad, Pocket, Revolve, Fillet, Chamfer, Draft, Mirror, Pattern)
- `cadkernel-viewer`: TechDraw workbench — placeholder tools (Front/Top/Right View, Section, Dimension, Export SVG)
- `cadkernel-viewer`: Assembly workbench — placeholder tools (Insert Component, Fixed, Coincident, Concentric, Distance)
- `cadkernel-viewer`: Sketcher workbench — added Rectangle tool placeholder
- `cadkernel-viewer`: Create Cone dialog — base radius, top radius, height parameters (top_radius=0 for pointed cone)
- `cadkernel-viewer`: Create Torus dialog — major radius, minor radius parameters
- `cadkernel-viewer`: Create menu expanded — Cone and Torus entries added

#### Application Phase 8: PartDesign Feature Implementations
- `cadkernel-modeling`: `mirror_solid()` — plane reflection via `copy_solid_transformed` with reversed winding for correct normals
- `cadkernel-modeling`: `scale_solid()` — uniform scaling about a center point, negative factor mirrors (reversed winding)
- `cadkernel-modeling`: `sweep()` rewritten — Frenet-frame sweep placing profile perpendicular to path tangent at each point, with bottom/top caps and side quads
- `cadkernel-modeling`: `loft()` implemented — blends between 2+ cross-section profiles with matching point counts, caps + side quads
- `cadkernel-modeling`: `shell_solid()` implemented — hollows out a solid by removing specified faces, offsetting remaining faces inward by thickness, connecting outer/inner boundaries with rim quads
- `cadkernel-modeling`: `linear_pattern()` implemented — creates N copies at uniform spacing along a direction using `copy_solid_transformed`
- `cadkernel-modeling`: `circular_pattern()` implemented — creates N copies at equal angular intervals around an axis using quaternion rotation
- `cadkernel-modeling`: `copy_solid_transformed()` shared utility — deep-copies solid topology with arbitrary point transform function, used by mirror/scale/pattern operations

#### Application Phase 9: Sketcher Workbench (Interactive 2D Sketch Editing)
- `cadkernel-viewer`: SketchMode system — enter/exit sketch editing mode on XY or XZ work planes
- `cadkernel-viewer`: 5 sketch drawing tools — Select, Line (chain mode), Rectangle (2-click), Circle (center+radius), Arc (center+radius, semicircle)
- `cadkernel-viewer`: 2D sketch overlay rendering — projects sketch points, lines, circles, and arcs from work plane to screen via `world_to_screen()` projection
- `cadkernel-viewer`: Constraint visualization — H/V/Length/Fix/Parallel/Perpendicular/Coincident indicators drawn near constrained entities
- `cadkernel-viewer`: Sketch toolbar — dynamic context: "New Sketch (XY/XZ)" when idle, tool buttons + constraint buttons + Close/Cancel when editing
- `cadkernel-viewer`: Screen-to-plane ray casting — `screen_to_sketch_plane()` unprojects mouse clicks through perspective camera to work plane intersection
- `cadkernel-viewer`: Sketch → Solid pipeline — Close Sketch solves constraints (Newton-Raphson), extracts 3D profile via WorkPlane, extrudes along plane normal
- `cadkernel-viewer`: Sketch constraint toolbar — Horizontal, Vertical, Length (with drag value) applied to last-drawn line
- `cadkernel-viewer`: Escape key exits sketch mode, right-click clears pending point
- `cadkernel-viewer`: Sketch mode banner — shows plane, active tool, point/line counts in viewport

#### Application Phase 10: TechDraw Workbench
- `cadkernel-io`: TechDraw module — orthographic projection with 7 standard views (Front/Back/Top/Bottom/Right/Left/Isometric)
- `cadkernel-io`: Hidden Line Removal (HLR) — edge visibility via 5-sample barycentric depth test against projected triangles
- `cadkernel-io`: Three-view drawing layout (third-angle projection: front, top, right)
- `cadkernel-io`: Dimension annotation system (Linear, Angular, Radius)
- `cadkernel-io`: `drawing_to_svg()` — complete SVG export with visible/hidden lines, view labels, dimensions
- `cadkernel-io`: SVG Text element + stroke-dasharray support for dashed hidden lines
- `cadkernel-viewer`: TechDraw toolbar — Front, Top, Right, Iso, 3-View, Export SVG, Clear
- `cadkernel-viewer`: TechDraw viewport overlay — projected edges (solid visible, dashed hidden), view labels, semi-transparent background

#### Application Phase 11: NURBS Kernel Strengthening
- `cadkernel-geometry`: Adaptive curve tessellation — recursive bisection with chord error + angle tolerance
- `cadkernel-geometry`: Adaptive surface tessellation — quad subdivision with bilinear center vs actual center chord error
- `cadkernel-geometry`: `TessellationOptions` (chord_tolerance, angle_tolerance, min_segments, max_depth)
- `cadkernel-geometry`: `TessellateCurve` / `TessellateSurface` blanket extension traits
- `cadkernel-geometry`: Curve-curve intersection — recursive bbox subdivision + Newton-Raphson refinement
- `cadkernel-geometry`: 2D polygon/polyline offset — miter-join offset with clamped miter length
- `cadkernel-topology`: Geometry binding helpers — `bind_edge_curve()`, `bind_face_surface()`, `face_has_surface()`, `edge_has_curve()`
- `cadkernel-io`: NURBS-aware tessellation — `tessellate_face`/`tessellate_solid` use bound surface geometry with adaptive subdivision, parameter domain from boundary projection for infinite surfaces

#### Phase A: NURBS Kernel Completion (FreeCAD Parity)
- `cadkernel-geometry`: B-spline basis function module (`bspline_basis.rs`) — `find_span`, `basis_funs`, `ders_basis_funs` (The NURBS Book A2.3, k-th order derivatives)
- `cadkernel-geometry`: NurbsCurve analytical derivatives — `tangent_at()` and `second_derivative_at()` via rational quotient rule (replaces finite differences)
- `cadkernel-geometry`: NurbsSurface analytical partial derivatives — `du()`, `dv()`, `normal_at()` via homogeneous derivatives (replaces finite differences)
- `cadkernel-geometry`: NurbsCurve operations — `reversed()`, `split_at(t)`, `join()` for curve manipulation
- `cadkernel-geometry`: NurbsCurve knot refinement — `refine_knots()` batch knot insertion (A5.4)
- `cadkernel-geometry`: NurbsCurve knot removal — `remove_knot()` with tolerance control (A5.8)
- `cadkernel-geometry`: NurbsCurve Bezier decomposition — `decompose_to_bezier()` splits at each knot span (A5.6)
- `cadkernel-geometry`: NurbsCurve interpolation — `NurbsCurve::interpolate()` chord-length parameterization + tridiagonal solver (A9.1)
- `cadkernel-geometry`: NurbsCurve approximation — `NurbsCurve::approximate()` least-squares fitting (A9.7)
- `cadkernel-geometry`: NurbsSurface knot operations — `insert_knot_u/v()`, `refine_knots_u/v()` via row/column decomposition
- `cadkernel-geometry`: NurbsSurface degree elevation — `elevate_degree_u/v()` via row/column curve elevation
- `cadkernel-geometry`: NurbsSurface interpolation — `NurbsSurface::interpolate()` two-pass tensor-product method
- `cadkernel-geometry`: Curve→NURBS conversion (`to_nurbs.rs`) — `LineSegment`, `Line`, `Circle`, `Arc`, `Ellipse` to rational NURBS
- `cadkernel-geometry`: Surface→NURBS conversion (`to_nurbs.rs`) — `Plane`, `Cylinder`, `Sphere` to rational NURBS surface
- `cadkernel-geometry`: NurbsCurve Newton `project_point()` — Bezier decompose multi-start + analytical Newton-Raphson
- `cadkernel-geometry`: NurbsSurface Newton `project_point()` — 20×20 coarse grid + 2D Gauss-Newton refinement
- `cadkernel-geometry`: Curve2D system (`curve2d.rs`) — `Curve2D` trait, `Line2D`, `Circle2D`, `NurbsCurve2D` for UV-space parametric curves
- `cadkernel-geometry`: TrimmedCurve (`trimmed.rs`) — re-parameterized sub-domain wrapper with [0,1] mapping
- `cadkernel-geometry`: TrimmedSurface (`trimmed.rs`) — UV trim loops with crossing-number point-in-polygon test
- `cadkernel-geometry`: Curve-surface intersection (`curve_surface.rs`) — subdivision + bisection + Newton on F(t,u,v) = C(t) - S(u,v) = 0
- `cadkernel-geometry`: Surface-surface intersection (`surface_surface.rs`) — seed finding via mutual projection + marching with predictor (n1×n2) and corrector
- `cadkernel-geometry`: NurbsCurve/NurbsSurface `bounding_box()` overrides — convex hull property (control point AABB)

#### Phase B: Trimmed Surfaces & Exact B-Rep
- `cadkernel-modeling`: Geometry binding for all 5 primitives — Box (6 Plane + 12 LineSegment), Cylinder (2 Plane + Cylinder surface + LineSegments), Sphere (Sphere surface + LineSegments), Cone/Frustum (Plane caps + Cone surface + LineSegments), Torus (Torus surface + LineSegments)
- `cadkernel-modeling`: `EdgeCache` enhanced — stores `Handle<EdgeData>` alongside half-edges, `all_edges()` method, `bind_edge_line_segments()` shared helper
- `cadkernel-modeling`: Sphere south cap winding fix — reversed ring direction for correct outward normal (-Z)
- `cadkernel-geometry`: `ParametricWire2D` — closed 2D curve chain for UV trim boundaries with winding number containment test, arc-length sampling, polyline conversion
- `cadkernel-geometry`: `TrimmedSurface` refactored to use `ParametricWire2D` (new `from_curves()` convenience constructor)
- `cadkernel-topology`: `FaceData` extended with `outer_trim` / `inner_trims` fields (ParametricWire2D)
- `cadkernel-topology`: `EdgeData` extended with `pcurve_left` / `pcurve_right` fields (Curve2D)
- `cadkernel-topology`: `BRepModel::bind_face_trim()` and `BRepModel::bind_edge_pcurve()` APIs
- `cadkernel-io`: Trimmed tessellation — UV centroid filtering against trim wires (outer + hole exclusion)
- `cadkernel-viewer`: "Trim Demo" action in Part workbench — creates box with circular hole trim on top face

#### Phase B06-B14: Exact Boolean & Face Splitting (2026-03-12)
- `cadkernel-geometry`: Face splitting along SSI curves (`face_split.rs`) — `split_solids_at_intersection()` preprocessor for exact boolean operations
- `cadkernel-geometry`: SSI-to-NURBS fitting (`fit_ssi_to_nurbs()`) — converts intersection point clouds to NURBS curves for face splitting
- `cadkernel-geometry`: SSI-to-parametric-curve fitting (`fit_ssi_to_pcurve()`) — fits intersection points to UV-space parametric curves
- `cadkernel-geometry`: Trim loop validation (`trim_validate.rs`) — `validate_trim()` verifies trim loop closure, winding, and self-intersection
- `cadkernel-geometry`: Trim winding correction (`ensure_correct_winding()`) — auto-corrects trim loop orientation for consistent inside/outside classification
- `cadkernel-geometry`: `TrimValidation` / `TrimIssue` diagnostics — structured validation results with issue classification
- `cadkernel-modeling`: Exact boolean operations (`boolean_op_exact()`) — face-splitting preprocessing for precise boolean evaluation
- `cadkernel-modeling`: Copy with geometry binding preservation in boolean operations — copied faces retain surface/curve bindings
- `cadkernel-modeling`: Planar face polygon intersection for non-surface-bound faces — fallback intersection path for unbound planar geometry

### Fixed

- `cadkernel-modeling`: `shape_analysis::classify_solid` now correctly identifies tessellated cylinders (was misclassified as Prism due to face-count heuristic)

### Tests
- 662 total tests (was 609), 53 new tests covering V1-V6 phases

#### Phase V1: Sketcher Completion (2026-03-15)
- `cadkernel-sketch`: 3 new entity types — `SketchEllipticalArc`, `SketchHyperbolicArc`, `SketchParabolicArc` (conic arc entities in `entity.rs`)
- `cadkernel-sketch`: 5 sketch editing tools (`tools.rs`) — `fillet_sketch_corner`, `chamfer_sketch_corner`, `trim_edge`, `split_edge`, `extend_edge`
- `cadkernel-sketch`: Sketch validation module (`validate.rs`) — `validate_sketch` with 7 issue types (open profiles, duplicate points, zero-length edges, etc.)
- `cadkernel-sketch`: Construction geometry — `toggle_construction_mode`, `mark_construction_point`, `mark_construction_line`
- `cadkernel-sketch`: New geometry helpers — `add_circle_3pt`, `add_ellipse_3pt`, `add_centered_rectangle`, `add_rounded_rectangle`, `add_arc_slot`

#### Phase V2: PartDesign Completion (2026-03-15)
- `cadkernel-modeling`: 8 new additive/subtractive primitive pairs — `additive_helix`/`subtractive_helix`, `additive_ellipsoid`/`subtractive_ellipsoid`, `additive_prism`/`subtractive_prism`, `additive_wedge`/`subtractive_wedge` (in `additive.rs`)
- `cadkernel-modeling`: 2 new subtractive operations — `subtractive_loft`, `subtractive_pipe` (in `additive.rs`)
- `cadkernel-modeling`: Total additive/subtractive operations expanded from 10 to 20

#### Phase V3: Part Workbench Completion (2026-03-15)
- `cadkernel-modeling`: Join operations (`join.rs`) — `connect_shapes`, `embed_shapes`, `cutout_shapes`
- `cadkernel-modeling`: Compound operations (`compound_ops.rs`) — `boolean_fragments`, `slice_to_compound`, `compound_filter`, `explode_compound`
- `cadkernel-modeling`: Shape operations (`face_from_wires.rs`) — `face_from_wires`, `points_from_shape`

#### Phase V4: TechDraw Expansion (2026-03-15)
- `cadkernel-io`: 10 new TechDraw annotation types — `ArcLengthDimension`, `ExtentDimension`, `ChamferDimension`, `WeldSymbol` (6 weld types), `BalloonAnnotation`, `Centerline`, `BoltCircleCenterlines`, `CosmeticLine` (4 styles), `BreakLine`
- `cadkernel-io`: SVG rendering for all new annotation types

#### Phase V5: Assembly Solver (2026-03-15)
- `cadkernel-modeling`: DOF analysis — `analyze_dof()` with per-constraint/joint DOF counting
- `cadkernel-modeling`: Iterative constraint solver — `solve()` with distance constraints
- `cadkernel-modeling`: 3 new joint types — `RackAndPinion`, `ScrewJoint`, `BeltJoint` (13 total)
- `cadkernel-modeling`: `rotation()` placement helper

#### Phase V6: Surface Workbench Completion (2026-03-15)
- `cadkernel-modeling`: `filling()` — N-sided boundary patch
- `cadkernel-modeling`: `sections()` — surface skinning through profiles
- `cadkernel-modeling`: `curve_on_mesh()` — project polyline onto mesh

#### Phase V8: Mesh Completion (2026-03-16)
- `cadkernel-io`: `mesh_boolean_intersection()` — AABB-filtered mesh boolean intersection
- `cadkernel-io`: `mesh_boolean_difference()` — AABB-filtered mesh boolean difference
- `cadkernel-io`: `regular_solid()` — 5 Platonic solids (Tetrahedron, Cube, Octahedron, Dodecahedron, Icosahedron) via `RegularSolidType`
- `cadkernel-io`: `face_info()` — per-face area, normal, centroid (`FaceInfo`)
- `cadkernel-io`: `bounding_box_info()` — mesh AABB with center, size, diagonal (`MeshBoundingBox`)
- `cadkernel-io`: `curvature_plot()` — curvature-to-RGB color mapping (blue→red)
- `cadkernel-io`: `add_triangle()` — add single triangle to mesh
- `cadkernel-io`: `unwrap_mesh()` — UV unwrapping via principal axis projection (`UnwrapResult`, `UvCoord`)
- `cadkernel-io`: `unwrap_face()` — single face UV coordinate computation
- `cadkernel-io`: `remove_components_by_size()` — remove small components by triangle count threshold
- `cadkernel-io`: `remove_component()` — remove specific component by index
- `cadkernel-io`: `trim_mesh()` — trim mesh with another mesh's bounding box
- `cadkernel-io`: `mesh_cross_sections()` — multiple parallel cross-sections along axis
- `cadkernel-io`: `segment_mesh()` — normal-based mesh segmentation via region growing (`MeshSegment`)
- `cadkernel-io`: `remesh()` — adaptive edge-length-based refinement
- `cadkernel-io`: `evaluate_and_repair()` — degenerate removal + vertex merge + normal harmonization (`MeshRepairReport`)
- `cadkernel-io`: `scale_mesh()` — per-axis mesh scaling
- New exported types: `FaceInfo`, `MeshBoundingBox`, `MeshRepairReport`, `MeshSegment`, `RegularSolidType`, `UnwrapResult`, `UvCoord`
- 18 new tests, total 680 tests (was 662)

#### Phase V9: Draft Workbench Completion (2026-03-16)
- `cadkernel-modeling`: 37 draft operations in `draft_ops.rs` (32 new functions + 5 existing)
- `cadkernel-modeling`: Wire creation — `make_fillet_wire`, `make_circle_wire`, `make_arc_wire`, `make_ellipse_wire`, `make_rectangle_wire`, `make_polygon_wire`, `make_bezier_wire`, `make_arc_3pt_wire`, `make_chamfer_wire`, `make_point`
- `cadkernel-modeling`: Wire manipulation — `offset_wire`, `join_wires`, `split_wire`, `upgrade_wire`, `downgrade_solid`, `wire_to_bspline`, `bspline_to_wire`, `stretch_wire`
- `cadkernel-modeling`: Solid transformation — `move_solid`, `rotate_solid`, `scale_solid_draft`, `mirror_solid_draft`
- `cadkernel-modeling`: Array patterns — `polar_array`, `point_array`
- `cadkernel-modeling`: Annotation — `make_draft_dimension`, `make_label`, `make_dimension_text`
- `cadkernel-modeling`: Snapping — `snap_to_endpoint`, `snap_to_midpoint`, `snap_to_nearest`
- `cadkernel-modeling`: Query — `wire_length`, `wire_area`
- New types: `DraftDimension`, `DraftLabel`, `SnapResult`, `WireResult`, `BSplineWireResult`, `ArrayResult`, `CloneResult`
- 40 new tests, total 705 tests (from 680)

#### Phase V10: FEM Workbench Expansion (2026-03-16)
- `cadkernel-modeling`: 6 new material presets — `FemMaterial::titanium()`, `copper()`, `concrete()`, `cast_iron()`, `custom()`, `ThermalMaterial` with `steel()`/`aluminum()`/`copper()` presets
- `cadkernel-modeling`: 8 new FEM types — `ThermalMaterial`, `ThermalBoundaryCondition` (4 variants), `ThermalResult`, `BeamSection` (circular, rectangular), `ModalResult`, `MeshQuality`, `PrincipalStresses`, `StrainResult`, `StressTensor`
- `cadkernel-modeling`: 4 new structural boundary conditions — `Displacement`, `Gravity`, `DistributedLoad`, `Spring`
- `cadkernel-modeling`: 4 new thermal boundary conditions — `FixedTemperature`, `HeatFlux`, `HeatGeneration`, `Convection`
- `cadkernel-modeling`: `modal_analysis()` — eigenfrequency extraction via inverse power iteration
- `cadkernel-modeling`: `thermal_analysis()` — steady-state heat conduction with Gauss-Seidel solver
- `cadkernel-modeling`: `mesh_quality()` — aspect ratio, volume, degenerate element detection
- `cadkernel-modeling`: `refine_tet_mesh()` — edge midpoint subdivision (1→8 tets)
- `cadkernel-modeling`: `extract_surface_mesh()` — boundary face extraction
- `cadkernel-modeling`: `merge_coincident_nodes()` — node deduplication within tolerance
- `cadkernel-modeling`: `compute_stress_tensor()` — full 6-component stress per element
- `cadkernel-modeling`: `compute_strain_tensor()` — full 6-component strain per element
- `cadkernel-modeling`: `principal_stresses()` — Cardano eigenvalue solver for 3x3 stress matrix
- `cadkernel-modeling`: `safety_factor()` — yield_stress / max_von_mises
- `cadkernel-modeling`: `strain_energy()` — total strain energy computation
- `cadkernel-modeling`: `compute_reactions()` — reaction forces at fixed nodes
- 34 new tests, total 739 tests (from 705)

#### Phase V11: Viewer UI Expansion (2026-03-17)
- `cadkernel-viewer`: File menu — Import/Export for STEP, IGES, DXF, PLY, 3MF, BREP formats
- `cadkernel-viewer`: Boolean operation dialogs — Union/Subtract/Intersect with second box primitive (size + offset parameters)
- `cadkernel-viewer`: Part operations — Mirror (XY/XZ/YZ), Scale, Shell, Fillet, Chamfer, Linear Pattern
- `cadkernel-viewer`: Mesh toolbar — Smooth, Harmonize Normals, Check Watertight, Remesh, Repair
- `cadkernel-viewer`: Analysis tools — Measure Solid (volume/area/centroid), Check Geometry (validity)
- `cadkernel-viewer`: PartDesign toolbar updated — Fillet/Chamfer/Shell/Mirror/Scale/Pattern connected to backend
- `cadkernel-viewer`: ~20 new `GuiAction` variants with full `process_actions()` handlers
- `cadkernel-viewer`: Removed unused stubs (BooleanUnion/Subtract/Intersect, TrimDemo)

#### Phase C: STEP I/O (Full Implementation)
- `cadkernel-io`: Full STEP tokenizer — ISO 10303-21 lexer with proper sign-digit validation
- `cadkernel-io`: STEP parser — entity resolution, nested parameter parsing
- `cadkernel-io`: STEP geometry mapping — CARTESIAN_POINT, DIRECTION, B_SPLINE_CURVE/SURFACE
- `cadkernel-io`: STEP topology mapping — VERTEX_POINT, EDGE_CURVE, FACE_BOUND, CLOSED_SHELL, MANIFOLD_SOLID_BREP
- `cadkernel-io`: STEP export — `export_step()` for B-Rep models, `export_step_mesh()` for triangle meshes
- `cadkernel-io`: STEP import — `import_step()` with entity cross-referencing

#### Phase D: Fillet/Draft/Split (Full Implementation)
- `cadkernel-modeling`: `fillet_edge()` — arc-approximated edge rounding with configurable radius and segments
- `cadkernel-modeling`: `fillet_edge_segments()` — configurable segment count variant
- `cadkernel-modeling`: `draft_faces()` — vertex displacement radially from pull axis proportional to height × tan(angle)
- `cadkernel-modeling`: `split_solid()` — vertex classification by signed distance to plane, edge-plane intersection, cap face generation

#### Phase E: Advanced Primitives
- `cadkernel-modeling`: `make_tube()` — hollow cylinder (4 vertex rings, 4N faces, outer/inner Cylinder + top/bottom Plane binding)
- `cadkernel-modeling`: `make_prism()` — regular polygon prism (N-sided polygon caps + N lateral quads)
- `cadkernel-modeling`: `make_wedge()` — tapered box/pyramid (WedgeParams, pyramid mode when top dims < epsilon)
- `cadkernel-modeling`: `make_ellipsoid()` — tri-axial ellipsoid (independent rx, ry, rz semi-axes)
- `cadkernel-modeling`: `make_helix()` — helical tube/spring (local Frenet frame, tube cross-section sweep)

#### Phase G: PartDesign Feature Operations
- `cadkernel-modeling`: `pad()` — additive extrusion (extrude profile → boolean union with base)
- `cadkernel-modeling`: `pocket()` — subtractive extrusion (extrude profile → boolean difference from base)
- `cadkernel-modeling`: `groove()` — subtractive revolution (revolve profile → boolean difference from base)
- `cadkernel-modeling`: `hole()` — cylindrical hole (polygon circle profile, arbitrary direction, extrude + boolean difference)
- `cadkernel-modeling`: `countersunk_hole()` — two-step hole (main + larger countersink)

#### Phase H-I: Sketcher Advanced Constraints
- `cadkernel-sketch`: `EqualLength` constraint — enforces two line segments have equal length (squared-distance formulation)
- `cadkernel-sketch`: `Midpoint` constraint — constrains a point to the midpoint of a line segment (2 equations)
- `cadkernel-sketch`: `Collinear` constraint — constrains two lines to be collinear (point-on-line + parallel, 2 equations)
- `cadkernel-sketch`: `EqualRadius` constraint — enforces two circles/arcs have equal radius (squared-distance formulation)
- `cadkernel-sketch`: `Concentric` constraint — constrains two center points to coincide (2 equations)
- All 5 constraints include analytical Jacobian entries for Newton-Raphson solver

#### Phase F: Part Advanced Operations
- `cadkernel-modeling`: `section_solid()` — cross-section contour computation by plane-face intersection (edge detection at face boundaries)
- `cadkernel-modeling`: `offset_solid()` — vertex-normal-based solid offset (averaged per-vertex normals, configurable distance)
- `cadkernel-modeling`: `thickness_solid()` — wall thickness operation creating inner/outer faces + rim quads (Inward/Outward/Centered join types)
- `cadkernel-math`: `Mat4::translation(Vec3)` — creates a 4x4 translation matrix
- `cadkernel-math`: `Mat4::transform_point(Point3)` — homogeneous point transformation with w-divide

#### Phase J: TechDraw Section & Detail Views
- `cadkernel-io`: `section_view()` — tessellate solid, find triangle-plane intersections, project cut contour to 2D cutting plane coordinates
- `cadkernel-io`: `detail_view()` — magnified circular region of an existing drawing view with configurable magnification factor

#### Phase K: Assembly Basics
- `cadkernel-modeling`: Assembly module — `Assembly` struct with component tree and constraint system
- `cadkernel-modeling`: `Component` with placement transform (`Mat4`), visibility toggle, named identification
- `cadkernel-modeling`: `AssemblyConstraint` enum — Fixed, Coincident, Concentric, Distance, Angle constraint types
- `cadkernel-modeling`: Bounding-box interference detection between assembly components
- `cadkernel-modeling`: `translation(dx, dy, dz)` helper for component placement

#### Phase L: Draft Workbench
- `cadkernel-modeling`: `make_wire()` — creates 3D polyline wire from point sequence (auto-detects closed wires)
- `cadkernel-modeling`: `make_bspline_wire()` — creates B-spline wire from control points with clamped uniform knot vector
- `cadkernel-modeling`: `clone_solid()` — deep copy of solid at same position via identity transform
- `cadkernel-modeling`: `rectangular_array()` — 2D grid pattern (count_x × count_y) along two direction vectors
- `cadkernel-modeling`: `path_array()` — copies solid to each path point with translation offset

#### Phase M: Mesh Advanced Operations
- `cadkernel-io`: `decimate_mesh()` — edge-collapse mesh decimation with target ratio (shortest-edge priority)
- `cadkernel-io`: `fill_holes()` — boundary edge detection, loop chaining, centroid fan triangulation
- `cadkernel-io`: `compute_curvature()` — per-vertex mean curvature via cotangent-weighted Laplace-Beltrami operator
- `cadkernel-io`: `subdivide_mesh()` — midpoint subdivision (each triangle → 4 triangles) with edge midpoint deduplication
- `cadkernel-io`: `flip_normals()` — reverse winding order and negate normals

#### Phase O: Surface Workbench
- `cadkernel-modeling`: `ruled_surface()` — linear interpolation surface between two NurbsCurves
- `cadkernel-modeling`: `surface_from_curves()` — Gordon-like surface construction from profile curve network
- `cadkernel-modeling`: `extend_surface()` — vertex-normal offset extension of existing solid faces
- `cadkernel-modeling`: `pipe_surface()` — tubular solid along path curve with Frenet frame and end caps

#### Phase N: FEM Basics
- `cadkernel-modeling`: `TetMesh` struct — tetrahedral mesh with nodes and element indices
- `cadkernel-modeling`: `FemMaterial` with preset `steel()` and `aluminum()` constructors
- `cadkernel-modeling`: `BoundaryCondition` enum — FixedNode, Force, Pressure
- `cadkernel-modeling`: `generate_tet_mesh()` — bounding box subdivision into conforming tets (alternating parity)
- `cadkernel-modeling`: `static_analysis()` — element stiffness assembly, Gauss-Seidel solver, von Mises stress computation

#### Phase P: IGES Import/Export
- `cadkernel-io`: Full IGES reader/writer with 80-column fixed-format records
- `cadkernel-io`: `IgesEntity` + `IgesEntityType` (Point 116, Line 110, Arc 100, NURBS Curve 126, Surface 128)
- `cadkernel-io`: `parse_iges()` — section classification (S/G/D/P/T), Directory Entry pairs, Parameter Data extraction
- `cadkernel-io`: `import_iges()` — Point/Line entities → BRepModel vertices/edges
- `cadkernel-io`: `export_iges()` / `export_iges_mesh()` — B-Rep/mesh → IGES format

#### Phase Q: Performance Optimization
- `cadkernel-geometry`: BVH (Bounding Volume Hierarchy) — AABB-based spatial index tree with midpoint split along longest axis
- `cadkernel-geometry`: `Aabb` struct — axis-aligned bounding box with merge, intersects, contains_point, surface_area, ray intersection (slab test)
- `cadkernel-geometry`: `Bvh` struct — build from items, query_aabb, query_point, query_ray methods
- `cadkernel-io`: `tessellate_solid_parallel()` — rayon-based parallel face tessellation with mesh merging
- `cadkernel-io`: `merge_meshes()` — combine multiple Mesh objects with vertex/index offset tracking

#### Phase R: Geometry Kernel Expansion
- `cadkernel-geometry`: `IsocurveU` / `IsocurveV` — extract curve from surface at constant u or v parameter
- `cadkernel-geometry`: `surface_curvatures()` — Gaussian, mean, and principal curvatures via first/second fundamental forms
- `cadkernel-geometry`: `OffsetCurve` — 3D parallel curve at fixed distance in a reference plane
- `cadkernel-geometry`: `RevolutionSurface` — surface of revolution via Rodrigues' rotation of a profile curve
- `cadkernel-geometry`: `ExtrusionSurface` — translational sweep surface with analytical du/dv
- `cadkernel-geometry`: `blend_curve()` — cubic Bezier G0/G1 bridge between two curves
- `cadkernel-geometry`: `check_surface_continuity()` — G0/G1/G2 continuity analysis between adjacent surfaces

#### Phase S: Modeling Expansion
- `cadkernel-modeling`: `make_spiral()` — flat Archimedean spiral tube solid
- `cadkernel-modeling`: `make_polygon()` — regular polygon prism (delegates to make_prism)
- `cadkernel-modeling`: `make_plane_face()` — flat rectangular face as thin box
- `cadkernel-modeling`: `boolean_xor()` — exclusive-OR boolean (Union minus Intersection)
- `cadkernel-modeling`: `Compound` — group solids without boolean (add/explode)
- `cadkernel-modeling`: `check_geometry()` — topological validity check (shells, faces, loops, edges, vertices)
- `cadkernel-modeling`: `check_watertight()` — manifold edge sharing verification
- `cadkernel-modeling`: `multi_transform()` — chained Translation/Rotation/Scale/Mirror transforms
- `cadkernel-modeling`: `Body` — PartDesign feature tree container with tip tracking
- `cadkernel-modeling`: `make_involute_gear()` — involute spur gear solid with parametric tooth profiles

#### Phase T: Sketcher Expansion
- `cadkernel-sketch`: 5 new constraint types — Diameter, Block, HorizontalDistance, VerticalDistance, PointOnObject
- `cadkernel-sketch`: `SketchEllipse` / `EllipseId` — ellipse entity with center, major axis endpoint, minor radius
- `cadkernel-sketch`: `SketchBSpline` / `BSplineId` — B-spline entity with control points, degree, closed flag
- `cadkernel-sketch`: `add_polyline()` — multi-segment line creation from point sequence
- `cadkernel-sketch`: `add_regular_polygon()` — regular N-sided polygon with auto-generated points and lines
- `cadkernel-sketch`: `add_arc_3pt()` — arc from 3 points with circumcircle computation

#### Phase U: File Format Expansion & Mesh Operations
- `cadkernel-io`: DXF import/export — 3DFACE entity mapping
- `cadkernel-io`: PLY import/export — ASCII format with normals
- `cadkernel-io`: 3MF export — XML-based 3D manufacturing format
- `cadkernel-io`: BREP text format import/export — CADKernel native B-Rep serialization
- `cadkernel-io`: `smooth_mesh()` — Laplacian smoothing with adjacency-based iteration
- `cadkernel-io`: `mesh_boolean_union()` — simple triangle-level mesh merge
- `cadkernel-io`: `cut_mesh_with_plane()` — plane clipping with triangle subdivision
- `cadkernel-io`: `mesh_section_from_plane()` — cross-section contour extraction
- `cadkernel-io`: `split_mesh_by_components()` — union-find component separation
- `cadkernel-io`: `harmonize_normals()` — BFS winding propagation for consistent normals
- `cadkernel-io`: `check_mesh_watertight()` — edge-count watertightness check
- `cadkernel-io`: `DimensionType` enum — 6 TechDraw dimension types (Length, H/V, Radius, Diameter, Angle) with SVG rendering

#### UI: Mesh Operations + New Primitives in Toolbar
- `cadkernel-viewer`: Mesh workbench toolbar — Decimate 50%, Subdivide, Fill Holes, Flip Normals buttons
- `cadkernel-viewer`: Mesh operation action processing with error handling and status messages
- `cadkernel-viewer`: 5 new primitive creation dialogs — Tube, Prism, Wedge, Ellipsoid, Helix with parameter input
- `cadkernel-viewer`: Part workbench toolbar expanded — 10 primitives total (Box, Cylinder, Sphere, Cone, Torus + Tube, Prism, Wedge, Ellipsoid, Helix)
- `cadkernel-viewer`: Create menu expanded — 5 new entries with separator (Tube, Prism, Wedge, Ellipsoid, Helix)
- `cadkernel-viewer`: Full action processing for all 5 new primitives (model creation + tessellation + display)

#### Application Phase 6: Remaining Issue Resolution
- `cadkernel-modeling`: `point_in_solid()` rewritten with proper 2D point-in-polygon test (crossing number algorithm with face-plane projection, replacing inaccurate bounding-box check)
- `cadkernel-geometry`: Line/Plane analytical `project_point` overrides (exact solution for infinite geometry, no NaN from sampling)
- `cadkernel-geometry`: Line/Plane `bounding_box` overrides with finite fallback domain (±1e6)
- `cadkernel-modeling`: Primitive edge deduplication via `EdgeCache` — Box (24→12 edges), Cylinder (6N→3N edges), Sphere proper shared half-edges. Correct manifold topology for B-Rep validation

### Fixed

#### CRITICAL
- `cadkernel-geometry`: `arbitrary_perpendicular` unwrap → `unwrap_or(Vec3::X)` (circle.rs, cylinder.rs)
- `cadkernel-io`: Binary STL reader triangle count cap (50M limit) to prevent OOM from malformed files
- `cadkernel-io`: Binary STL writer u32 overflow check (`write_stl_binary` returns `KernelResult`)
- `cadkernel-io`: STEP/IGES `todo!()` panics replaced with `Err(IoError)` for safe error handling
- `cadkernel-modeling`: `classify_face` offset direction corrected (inward → outward normal offset)
- `cadkernel-modeling`: `compute_mass_properties` near-zero volume guard with early return
- `cadkernel-modeling`: `solid_mass_properties` `todo!()` replaced with `Err`
- `cadkernel-topology`: EntityStore generation type widened from u32 to u64 (prevents overflow on long-running sessions)
- `cadkernel-modeling`: `point_in_solid()` rewritten — proper ray-polygon intersection with 2D crossing number test (replaces inaccurate bounding-box check)

#### HIGH
- `cadkernel-geometry`: Sphere/Torus/Cone constructors now validate parameters (`radius > 0`, `half_angle ∈ (0, π/2)`) and return `KernelResult`
- `cadkernel-geometry`: NurbsCurve de_boor zero-weight guard (prevents division by zero)
- `cadkernel-topology`: `loop_half_edges` max iteration guard (100K limit prevents infinite loops on corrupted topology)
- `cadkernel-sketch`: Angle constraint `tan()` singularity replaced with `atan2(cross, dot) - theta`
- `cadkernel-sketch`: Profile `extract_profile` bounds-checked point access
- `cadkernel-geometry`: Line/Plane infinite domain — analytical `project_point` + finite `bounding_box` overrides (prevents NaN from default sampling)
- `cadkernel-modeling`: Primitive duplicate edges — `EdgeCache` dedup system for Box/Cylinder/Sphere (correct manifold half-edge topology)

#### MEDIUM
- `cadkernel-topology`: `validate()` now enforces Euler characteristic V-E+F=2
- `cadkernel-io`: SVG XML entity escaping (`&`, `<`, `>`, `"`, `'`) in style attribute values
- `cadkernel-sketch`: `WorkPlane::new` Gram-Schmidt orthogonalization (x_axis perpendicular to normal)
- `cadkernel-viewer`: BFS smooth-group optimization — edge-based local adjacency for per-vertex face grouping

