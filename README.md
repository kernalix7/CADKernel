<div align="center">

# CADKernel

**Open-Source CAD Software Built with Rust**

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![CI](https://github.com/kernalix7/CADKernel/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/kernalix7/CADKernel/actions/workflows/ci.yml)

*Stable, fast, compatible, and extensible — the next-generation open-source CAD software*

**English** | [한국어](README.ko.md)

</div>

---

## Table of Contents

- [About](#about)
- [Core Values](#core-values)
  - [Stability](#stability)
  - [Performance](#performance)
  - [Compatibility](#compatibility)
  - [Extensibility](#extensibility)
- [Tech Stack](#tech-stack)
- [Comparison with Other CAD Software](#comparison-with-other-cad-software)
- [Architecture Overview](#architecture-overview)
- [Supported File Formats](#supported-file-formats)
- [Extension System](#extension-system)
- [AI Integration (MCP)](#ai-integration-mcp)
- [Roadmap](#roadmap)
- [Build & Install](#build--install)
- [Demo](#demo)
- [FAQ](#faq)
- [Versioning](#versioning)
- [Changelog](#changelog)
- [Acknowledgments](#acknowledgments)
- [Contributing](#contributing)
- [License](#license)

---

## About

**CADKernel** is an open-source CAD software written in Rust. It was created to address the high cost, platform lock-in, and limited extensibility of existing commercial CAD software.

By leveraging Rust's memory safety and zero-cost abstractions, CADKernel aims to deliver a crash-free, stable CAD environment with near-native performance. Combined with broad CAD file format compatibility, a plugin-based extension system, and generative AI integration, CADKernel builds an open CAD ecosystem that anyone can freely use and extend.

### Why CADKernel?

- **No Cost Barrier** — Free and open-source for everyone
- **Platform Freedom** — Runs on Windows, macOS, and Linux
- **Community-Driven** — Built together by users and developers
- **Future-Ready** — Design tools for the AI era

---

## Core Values

### Stability

> Stability is the non-negotiable top priority for CAD software.

CAD workflows involve hours of precise design work, and unexpected crashes can lead to significant data loss. CADKernel leverages Rust's language-level guarantees to ensure rock-solid stability.

- **Memory Safety** — Rust's Ownership system and Borrow Checker eliminate memory-related bugs at compile time
- **Thread Safety** — A concurrency model that makes data races impossible, ensuring reliable multi-threaded operation
- **Robust Error Handling** — Explicit error handling via `Result`/`Option` types ensures no exception goes unhandled
- **Auto-Save & Recovery** — Built-in periodic auto-save and crash recovery mechanisms minimize data loss
- **Comprehensive Testing** — Continuous quality assurance through unit tests, integration tests, and fuzz testing
- **Geometric Robustness** — A robust geometry engine that accounts for floating-point arithmetic limitations

### Performance

> Delivering best-in-class performance to handle complex 3D models in real time.

CAD software must process hundreds of thousands of geometric elements in real time. CADKernel achieves C/C++-level performance through Rust's zero-cost abstractions and optimized algorithms.

- **Zero-Cost Abstractions** — High-level abstractions with no runtime overhead
- **Parallel Computation** — Rayon-based data parallelism to maximize multi-core CPU utilization
- **GPU Acceleration** — Cross-platform GPU compute and rendering pipeline via wgpu
- **Lazy Evaluation** — Computations are deferred until needed, eliminating unnecessary calculations
- **Efficient Memory Management** — Deterministic memory management without a garbage collector for predictable performance
- **Caching Strategy** — Intelligent caching of repeated geometric computations to reduce recalculation costs
- **Spatial Indexing** — Efficient spatial queries using BVH, R-Tree, and other spatial data structures

### Compatibility

> Broad compatibility with the existing CAD ecosystem and true cross-platform support.

CAD work doesn't happen in isolation. Data exchange with team members, partners, and tools is essential. CADKernel minimizes platform dependency and supports a wide range of major CAD file formats.

- **Cross-Platform** — Consistent experience on Windows, macOS, and Linux with automatic rendering backend selection (Vulkan / Metal / DX12 / OpenGL)
- **60+ File Formats** — Comprehensive support across 8 categories: industry standard (STEP, IGES, Parasolid, ACIS, JT, IFC), commercial CAD (SolidWorks, CATIA, Creo, Inventor, Fusion 360, Rhino, SketchUp, AutoCAD), mesh/visualization (glTF, FBX, USD, COLLADA), 3D printing (3MF, AMF, G-code), 2D drawing (SVG, PDF, DXF/DWG), point cloud (PCD, LAS, E57), and more
- **Commercial CAD Import** — Read-only support for proprietary formats like SolidWorks, CATIA V5, PTC Creo, Autodesk Inventor, and Fusion 360
- **Point Cloud & Scan Data** — Import/export LiDAR (LAS/LAZ), structured 3D scan (E57), and raw point cloud (PCD, XYZ) data
- **Full Unicode Support** — Complete multilingual support across file paths, layer names, annotations, and all text elements
- **Native Format** — Lossless storage of all features and metadata in the proprietary `.cadkernel` format

### Extensibility

> An open ecosystem where users can build, share, and integrate the features they need.

CAD requirements vary across domains — architecture, mechanical engineering, electronics, industrial design, and more. No single core can cover every specialized need. CADKernel solves this with a powerful extension system.

- **Add-on System** — Develop and distribute custom features through a public Plugin API
- **Official Merge Path** — Community-vetted, high-quality Add-ons can be merged into official releases
- **Scripting Support** — Lua/Python scripting interface for automating repetitive tasks
- **MCP (Model Context Protocol) Support** — Standardized integration interface for generative AI (LLM) integration
- **Parametric Design** — Parameter-driven modeling with a constraint system for flexible design changes
- **Custom Renderers** — User-defined visualization through rendering pipeline extensions

---

## Tech Stack

CADKernel is built on a carefully selected set of Rust crates and technologies:

| Category | Technology | Purpose |
|----------|------------|----------|
| **Language** | Rust 1.85+ | Memory safety, performance, fearless concurrency |
| **GPU / Rendering** | wgpu | Cross-platform GPU API (Vulkan / Metal / DX12 / OpenGL) |
| **Parallelism** | Rayon | Data-parallel computation for multi-core CPUs |
| **Math** | nalgebra, glam | Linear algebra, vectors, matrices, transforms |
| **Geometry** | Custom B-Rep / NURBS engine | Boundary representation and freeform surface modeling |
| **Spatial Index** | bvh, rstar | BVH and R-Tree for efficient spatial queries |
| **GUI** | egui / iced *(TBD)* | Cross-platform immediate / retained mode GUI |
| **Scripting** | mlua, PyO3 | Lua and Python scripting bindings |
| **Serialization** | serde, bincode | High-performance data serialization |
| **File I/O** | iso-10303 *(planned)* | STEP file parsing and writing |
| **AI / MCP** | JSON-RPC, tower | MCP server for generative AI integration |
| **Testing** | cargo-fuzz, proptest | Fuzz testing and property-based testing |

---

## Comparison with Other CAD Software

| Feature | **CADKernel** | FreeCAD | OpenSCAD | BRL-CAD | LibreCAD |
|---------|:------------:|:-------:|:--------:|:-------:|:--------:|
| Language | Rust | C++ / Python | C++ | C / Tcl | C++ |
| 3D Modeling | 🚧 | ✅ | ✅ | ✅ | ❌ (2D only) |
| Parametric Design | 🚧 | ✅ | ✅ (code) | ✅ | ❌ |
| B-Rep + NURBS | 🚧 | ✅ (OCCT) | ❌ (CSG) | ✅ | ❌ |
| GUI | 🚧 | ✅ | Minimal | ✅ | ✅ |
| Plugin System | 🚧 | ✅ (Python) | ❌ | ❌ | ❌ |
| STEP Support | 🚧 | ✅ | ❌ | ✅ | ❌ |
| 60+ Formats | 🚧 | Partial | ❌ | Partial | ❌ |
| GPU Rendering | 🚧 (wgpu) | Partial | OpenGL | OpenGL | ❌ |
| Memory Safety | ✅ (Rust) | ❌ | ❌ | ❌ | ❌ |
| AI / MCP | 🚧 | ❌ | ❌ | ❌ | ❌ |
| Cross-Platform | 🚧 (Target: Win/Mac/Linux) | Win/Mac/Linux | Win/Mac/Linux | Win/Mac/Linux | Win/Mac/Linux |
| License | Apache 2.0 | LGPL 2.1 | GPL 2 | LGPL 2.1 | GPL 2 |

> Note: CADKernel status reflects the current **pre-alpha** implementation state.

---

## Architecture Overview

CADKernel adopts a layered modular architecture that ensures independence and replaceability of each layer.

```
┌───────────────────────────────────────────────────────────┐
│                     Application Layer                     │
│         (GUI · CLI · Scripting · AI/MCP Interface)        │
├───────────────────────────────────────────────────────────┤
│                      Extension Layer                      │
│          (Add-on Manager · Plugin API · Registry)         │
├───────────────────────────────────────────────────────────┤
│                       Service Layer                       │
│      (Modeling · Rendering · I/O · Constraint · Undo)     │
├───────────────────────────────────────────────────────────┤
│                     Core Kernel Layer                     │
│  (Geometry Engine · Topology · Spatial Index · Math Lib)  │
├───────────────────────────────────────────────────────────┤
│                    Platform Abstraction                   │
│          (Window · GPU · FileSystem · Threading)          │
└───────────────────────────────────────────────────────────┘
```

| Layer | Role | Key Crates |
|-------|------|------------|
| **Core Kernel** | Geometry operations, Topology, Math library | `cadkernel-core`, `cadkernel-math` |
| **Service** | Modeling operations, Rendering, File I/O | `cadkernel-modeling`, `cadkernel-render`, `cadkernel-io` |
| **Extension** | Plugin loading/management, API exposure | `cadkernel-extension` |
| **Application** | GUI, CLI, Scripting, AI integration | `cadkernel-app`, `cadkernel-mcp` |
| **Platform** | OS and hardware abstraction | `cadkernel-platform` |

---

## Supported File Formats

### Native

| Format | Extension | Read | Write | Notes |
|--------|-----------|:----:|:-----:|-------|
| CADKernel | `.cadkernel` | 🔲 | 🔲 | Lossless native format |

### Industry Standard (Neutral Exchange)

| Format | Extension | Read | Write | Notes |
|--------|-----------|:----:|:-----:|-------|
| STEP AP203 | `.step`, `.stp` | 🔲 | 🔲 | Geometry exchange standard |
| STEP AP214 | `.step`, `.stp` | 🔲 | 🔲 | Automotive industry standard |
| STEP AP242 | `.step`, `.stp` | 🔲 | 🔲 | Includes PMI/GD&T |
| IGES | `.iges`, `.igs` | 🔲 | 🔲 | Legacy exchange format |
| Parasolid | `.x_t`, `.x_b` | 🔲 | 🔲 | Siemens Parasolid kernel |
| ACIS SAT/SAB | `.sat`, `.sab` | 🔲 | 🔲 | Spatial ACIS kernel |
| JT | `.jt` | 🔲 | 🔲 | Siemens lightweight visualization |
| IFC | `.ifc` | 🔲 | 🔲 | BIM / Architecture (ISO 16739) |
| BREP | `.brep`, `.brp` | 🔲 | 🔲 | OpenCASCADE boundary representation |

### Commercial CAD (3D)

| Format | Extension | Read | Write | Notes |
|--------|-----------|:----:|:-----:|-------|
| DWG | `.dwg` | 🔲 | 🔲 | AutoCAD native |
| DXF | `.dxf` | 🔲 | 🔲 | AutoCAD exchange format |
| 3DM | `.3dm` | 🔲 | 🔲 | Rhino / OpenNURBS |
| FCStd | `.fcstd` | 🔲 | 🔲 | FreeCAD |
| SLDPRT / SLDASM | `.sldprt`, `.sldasm` | 🔲 | — | SolidWorks Part / Assembly |
| IPT / IAM | `.ipt`, `.iam` | 🔲 | — | Autodesk Inventor |
| CATPART / CATPRODUCT | `.catpart`, `.catproduct` | 🔲 | — | Dassault CATIA V5 |
| PRT / ASM | `.prt`, `.asm` | 🔲 | — | PTC Creo (Pro/E) |
| F3D | `.f3d` | 🔲 | — | Autodesk Fusion 360 |
| DGN | `.dgn` | 🔲 | 🔲 | Bentley MicroStation |
| SKP | `.skp` | 🔲 | 🔲 | Trimble SketchUp |
| 3DS | `.3ds` | 🔲 | 🔲 | Autodesk 3ds Max (legacy) |
| BLEND | `.blend` | 🔲 | — | Blender (import only) |

### 2D Drawing & Vector

| Format | Extension | Read | Write | Notes |
|--------|-----------|:----:|:-----:|-------|
| SVG | `.svg` | 🔲 | 🔲 | Scalable Vector Graphics |
| PDF | `.pdf` | 🔲 | 🔲 | 2D drawings / 3D PDF export |
| EPS | `.eps` | 🔲 | 🔲 | Encapsulated PostScript |
| HPGL | `.plt`, `.hpgl` | 🔲 | 🔲 | Plotter output format |

### Mesh & Visualization

| Format | Extension | Read | Write | Notes |
|--------|-----------|:----:|:-----:|-------|
| STL | `.stl` | 🔲 | 🔲 | 3D printing standard (ASCII/Binary) |
| OBJ | `.obj` | 🔲 | 🔲 | Wavefront mesh format |
| glTF / GLB | `.gltf`, `.glb` | 🔲 | 🔲 | Web 3D standard (Khronos) |
| FBX | `.fbx` | 🔲 | 🔲 | Autodesk exchange format |
| COLLADA | `.dae` | 🔲 | 🔲 | XML-based 3D exchange |
| PLY | `.ply` | 🔲 | 🔲 | Polygon / Stanford format |
| OFF | `.off` | 🔲 | 🔲 | Object File Format |
| VRML | `.wrl` | 🔲 | 🔲 | Virtual Reality Modeling Language |
| X3D | `.x3d` | 🔲 | 🔲 | VRML successor (ISO/IEC 19775) |
| USD / USDA / USDC | `.usd`, `.usda`, `.usdc` | 🔲 | 🔲 | Pixar Universal Scene Description |

### 3D Printing & Manufacturing

| Format | Extension | Read | Write | Notes |
|--------|-----------|:----:|:-----:|-------|
| 3MF | `.3mf` | 🔲 | 🔲 | Next-gen 3D printing (3MF Consortium) |
| AMF | `.amf` | 🔲 | 🔲 | Additive Manufacturing File (ISO/ASTM 52915) |
| G-code | `.gcode`, `.nc` | — | 🔲 | CNC / 3D printer toolpath |
| SLC | `.slc` | 🔲 | 🔲 | Stereolithography contour |

### Point Cloud & Scan Data

| Format | Extension | Read | Write | Notes |
|--------|-----------|:----:|:-----:|-------|
| PCD | `.pcd` | 🔲 | 🔲 | Point Cloud Library format |
| LAS / LAZ | `.las`, `.laz` | 🔲 | 🔲 | LiDAR data (ASPRS) |
| E57 | `.e57` | 🔲 | 🔲 | 3D scan data (ASTM E2807) |
| XYZ / PTS | `.xyz`, `.pts` | 🔲 | 🔲 | ASCII point cloud |
| PLY | `.ply` | 🔲 | 🔲 | Point cloud variant |

### Image & Texture

| Format | Extension | Read | Write | Notes |
|--------|-----------|:----:|:-----:|-------|
| PNG | `.png` | 🔲 | 🔲 | Render / texture export |
| JPEG | `.jpg`, `.jpeg` | 🔲 | 🔲 | Texture import |
| HDR / EXR | `.hdr`, `.exr` | 🔲 | 🔲 | HDR environment maps |
| BMP | `.bmp` | 🔲 | 🔲 | Bitmap image |
| TIFF | `.tif`, `.tiff` | 🔲 | 🔲 | High-quality image export |

> 🔲 = Planned · ✅ = Supported · 🚧 = In Progress · — = Not Applicable

---

## Extension System

CADKernel's extension system aims to create a virtuous cycle of **Develop → Share → Validate → Integrate**.

```
User Add-on Development
        │
        ▼
  Community Sharing & Usage
        │
        ▼
  Quality Validation & Review
        │
        ▼
  Official Release Merge ◀── Core Team Approval
```

### Add-on Development

- **Plugin API** — Develop Add-ons through a versioned, stable public API
- **Sandboxed Execution** — Add-ons run in isolated environments without affecting core system stability
- **Hot Reload** — Instantly reflect Add-on changes during development without restarting

### Official Feature Merge

Community-developed Add-ons that meet the following criteria can be merged as official features:

1. Sufficient user base and positive feedback
2. Code quality standards met (test coverage, documentation, code review)
3. Architectural alignment with the core system
4. License compatibility (Apache 2.0)

---

## AI Integration (MCP)

CADKernel supports **MCP (Model Context Protocol)** for seamless integration with generative AI.

```
┌──────────────┐     MCP      ┌──────────────────┐
│  AI / LLM    │◄────────────►│  CADKernel       │
│  (Client)    │  (JSON-RPC)  │  (MCP Server)    │
└──────────────┘              └──────────────────┘
```

### What's Possible with MCP

- **Natural Language → 3D Models** — Convert natural language commands like "Create a cylinder with 50mm diameter and 100mm height" into 3D models
- **Design Assistant** — AI understands design intent and suggests optimal modeling approaches
- **Automated Design Validation** — AI automatically reviews design rule violations, manufacturability, and more
- **Parametric Optimization** — AI explores optimal parameter combinations under given constraints
- **Automated Documentation** — Auto-generate drawings, BOMs, and reports from design data

### MCP Tools

| Tool | Description |
|------|-------------|
| `create_geometry` | Create geometric elements (points, lines, surfaces, solids) |
| `transform` | Translation, rotation, scale, and other transformations |
| `boolean_operation` | Union, subtract, and intersect boolean operations |
| `query_model` | Query model properties (volume, area, mass, etc.) |
| `export_model` | Export model in a specified format |
| `apply_constraint` | Apply dimensional and geometric constraints |
| `undo` / `redo` | Operation history management |

---

## Roadmap

### Phase 1 — Foundation
- [ ] Project structure and build system setup (Cargo workspace)
- [ ] Core math library (vectors, matrices, transforms)
- [ ] Basic geometry engine (B-Rep, NURBS curves/surfaces)
- [ ] Basic topology structures (Vertex, Edge, Face, Shell, Solid)
- [ ] Basic rendering pipeline (wgpu)
- [ ] Native format (`.cadkernel`) read/write
- [ ] STL import/export (ASCII / Binary)
- [ ] OBJ import/export
- [ ] CLI interface

### Phase 2 — Core Features
- [ ] 3D solid modeling operations (Extrude, Revolve, Sweep, Loft)
- [ ] Boolean operations (Union, Subtract, Intersect)
- [ ] Constraint Solver
- [ ] Undo/Redo system
- [ ] GUI framework implementation
- [ ] STEP AP203/AP214 import/export
- [ ] IGES import/export
- [ ] glTF / GLB import/export
- [ ] 3MF import/export
- [ ] SVG / PDF 2D drawing export

### Phase 3 — Compatibility (Industry Standard)
- [ ] STEP AP242 (PMI/GD&T) support
- [ ] DXF/DWG import/export
- [ ] 3DM (OpenNURBS / Rhino) import/export
- [ ] Parasolid (`.x_t`, `.x_b`) import/export
- [ ] ACIS SAT/SAB import/export
- [ ] JT (Siemens) import/export
- [ ] IFC (BIM / Architecture) import/export
- [ ] BREP (OpenCASCADE) import/export
- [ ] FCStd (FreeCAD) import/export
- [ ] Cross-platform testing and stabilization (Windows / macOS / Linux)

### Phase 4 — Compatibility (Commercial CAD & Extended Formats)
- [ ] SolidWorks (`.sldprt`, `.sldasm`) import
- [ ] CATIA V5 (`.catpart`, `.catproduct`) import
- [ ] PTC Creo (`.prt`, `.asm`) import
- [ ] Autodesk Inventor (`.ipt`, `.iam`) import
- [ ] Autodesk Fusion 360 (`.f3d`) import
- [ ] SketchUp (`.skp`) import/export
- [ ] Bentley MicroStation (`.dgn`) import/export
- [ ] Blender (`.blend`) import
- [ ] FBX import/export
- [ ] COLLADA (`.dae`) import/export
- [ ] 3DS import/export
- [ ] VRML / X3D import/export
- [ ] USD / USDA / USDC import/export
- [ ] PLY / OFF mesh import/export
- [ ] AMF import/export
- [ ] EPS / HPGL 2D export
- [ ] G-code / SLC manufacturing output

### Phase 5 — Point Cloud, Scan Data & Imaging
- [ ] PCD (Point Cloud Library) import/export
- [ ] LAS / LAZ (LiDAR) import/export
- [ ] E57 (3D scan) import/export
- [ ] XYZ / PTS (ASCII point cloud) import/export
- [ ] PNG / JPEG / BMP / TIFF render export
- [ ] HDR / EXR environment map support

### Phase 6 — Extension Ecosystem
- [ ] Plugin API design and implementation
- [ ] Add-on manager and registry
- [ ] Lua/Python scripting interface
- [ ] Community Add-on marketplace
- [ ] Custom file format import/export via Add-on API

### Phase 7 — AI Integration
- [ ] MCP server implementation
- [ ] Natural language → modeling command translation
- [ ] AI-powered design assistant
- [ ] Automated design validation
- [ ] AI-driven parametric optimization

---

## Build & Install

### Requirements

- **Rust** 1.85 or later (install via [rustup](https://rustup.rs/))
- **GPU Drivers** — Vulkan, Metal, or DX12 support
- **CMake** 3.16+ (for some native dependencies)
- **Python** 3.10+ *(optional, for scripting support)*

### Platform-Specific Prerequisites

<details>
<summary><b>Linux (Ubuntu / Debian)</b></summary>

```bash
# Install system dependencies
sudo apt update
sudo apt install -y build-essential cmake pkg-config \
  libx11-dev libxkbcommon-dev libwayland-dev \
  libvulkan-dev mesa-vulkan-drivers

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
</details>

<details>
<summary><b>macOS</b></summary>

```bash
# Install Xcode command line tools
xcode-select --install

# Install dependencies via Homebrew
brew install cmake

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
> macOS uses Metal as the rendering backend automatically.
</details>

<details>
<summary><b>Windows</b></summary>

1. Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (C++ workload)
2. Install [CMake](https://cmake.org/download/)
3. Install Rust via [rustup](https://rustup.rs/)
4. Ensure up-to-date GPU drivers are installed (Vulkan or DX12)
</details>

### Build from Source

```bash
# Clone the repository
git clone https://github.com/kernalix7/CADKernel.git
cd CADKernel

# Build
cargo build --release

# Run
cargo run --release
```

### Testing

```bash
# Run all tests
cargo test

# Run benchmarks
cargo bench

# Run fuzz tests (requires nightly)
cargo +nightly fuzz run geometry_fuzz
```

---

## Demo

- Demo build and screenshots are currently in preparation.
- Until the first public alpha, this section will be updated with preview assets and usage videos.

---

## FAQ

### Is CADKernel already production-ready?

Not yet. CADKernel is currently in active development and should be considered pre-alpha.

### Which platforms are supported?

Windows, macOS, and Linux are target platforms.

### Is commercial CAD file import supported?

Planned. Proprietary format support is listed in the roadmap and compatibility matrix as implementation targets.

### Can I automate workflows?

Yes. Lua/Python scripting and MCP-based AI integration are core extensibility goals.

---

## Versioning

CADKernel follows [Semantic Versioning](https://semver.org/) (`MAJOR.MINOR.PATCH`).

- `MAJOR` — Breaking API/format changes
- `MINOR` — Backward-compatible features
- `PATCH` — Backward-compatible fixes

---

## Changelog

Project change history is maintained in [CHANGELOG.md](CHANGELOG.md).

---

## Acknowledgments

CADKernel stands on the shoulders of the open-source ecosystem. We especially appreciate communities around:

- Rust and Cargo ecosystem
- wgpu and graphics infrastructure projects
- geometric computing and CAD interoperability standards

---

## Contributing

We welcome contributions from the community.

For the full contribution process and checklist, see [CONTRIBUTING.md](CONTRIBUTING.md).

Quick start:

1. **Fork** this repository
2. Create a new branch: `git checkout -b feature/amazing-feature`
3. Commit your changes: `git commit -m 'feat: add amazing feature'`
4. Push to the branch: `git push origin feature/amazing-feature`
5. Open a **Pull Request**

### Contribution Guidelines

- Use [Conventional Commits](https://www.conventionalcommits.org/) style commit messages
- Include test code for every new feature
- Verify code style with `cargo fmt` and `cargo clippy`
- Document all public APIs with doc comments (`///`)

### Security

If you discover a security vulnerability, please report it responsibly via [GitHub Security Advisories](https://github.com/kernalix7/CADKernel/security/advisories/new) instead of opening a public issue. See [SECURITY.md](SECURITY.md) for details.

### Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report unacceptable behavior via the project maintainers.

---

## License

This project is licensed under [Apache License 2.0](LICENSE).

```
Copyright 2026 Kim DaeHyun (kernalix7@kodenet.io)

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0
```

---

<div align="center">

**CADKernel** — *Open-Source CAD for Everyone*

[Report Issue](https://github.com/kernalix7/CADKernel/issues) · [Request Feature](https://github.com/kernalix7/CADKernel/issues) · [Discussions](https://github.com/kernalix7/CADKernel/discussions)

</div>
