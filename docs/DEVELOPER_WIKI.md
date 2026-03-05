# CADKernel Developer Wiki

> **Version**: 0.1.0 (pre-alpha)  
> **Last updated**: 2026-02-26  
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
- [9. Next Steps (Phase 5+)](#9-next-steps-phase-5)
- [10. Glossary](#10-glossary)

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

2D parametric sketch with 14 constraint types. Newton-Raphson solver with Armijo backtracking. Profile extraction to 3D via `WorkPlane`.

### 3.6 cadkernel-modeling

Primitive builders (`make_box`, `make_cylinder`, `make_sphere`), feature operations (`extrude`, `revolve`), boolean operations (`Union`, `Subtract`, `Intersect`). All return `KernelResult`. All auto-generate persistent naming tags.

### 3.7 cadkernel-io

Tessellation (`tessellate_solid`, `tessellate_face`), export (`write_stl_ascii`, `write_stl_binary`, `write_obj`, `export_*` file variants).

### 3.8 cadkernel-viewer

Native desktop GUI application (egui 0.31 + wgpu 24.x + winit 0.30).

**Modules**: `app.rs` (state + event loop), `render.rs` (GPU + camera + math), `gui.rs` (UI panels + ViewCube), `nav.rs` (mouse navigation presets).

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
2D sketch system with 14 constraints and Newton-Raphson solver, extrude/revolve feature ops, box/cylinder/sphere primitives, STL/OBJ export, tessellation.

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

**Current status**: 209 tests, 0 clippy warnings, 0 fmt diff.

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

**209 tests** across all crates. Run with `cargo test --workspace`.

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

## 9. Next Steps (Phase 5+)

| Phase | Focus | Key Items |
|-------|-------|-----------|
| 5 | Advanced Geometry | Fillet/Chamfer, Shell offset, Mass properties, Sweep/Loft |
| 6 | File I/O | STEP AP203, IGES, glTF, 3MF, native format |
| 7 | Framework | Undo/Redo, Parametric rebuild, Assembly, Materials |

---

## 10. Glossary

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
