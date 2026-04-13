# Contributing to CADKernel

**English** | [한국어](docs/CONTRIBUTING.ko.md)

Thanks for your interest in contributing to CADKernel!

## Architecture Overview

CADKernel is a B-Rep CAD kernel built with Rust, organized as a 9-crate workspace:

| Crate | Purpose |
|-------|---------|
| `core` | Error types (`KernelResult`), predicates, constants |
| `math` | Point/Vec/Mat types, transforms, quaternions, BBox |
| `geometry` | Curve/Surface traits, NURBS, tessellation, BVH, intersection |
| `topology` | Half-edge B-Rep: Vertex, Edge, HalfEdge, Loop, Face, Shell, Solid |
| `modeling` | 13 primitives, features, booleans, patterns, FEM, assembly |
| `sketch` | 2D parametric sketch with 24 constraint types, Newton-Raphson solver |
| `io` | 11 file formats (STL/OBJ/glTF/STEP/IGES/DXF/PLY/3MF/BREP/DWG/DAE) |
| `viewer` | egui + wgpu desktop GUI with 3D viewport |
| `python` | PyO3 bindings (excluded from default workspace build) |

**Dependency flow**: `core` → `math` → `geometry` → `topology` → `modeling`/`sketch` → `io` → `viewer`

For detailed architecture, see [DEVELOPER_WIKI.md](docs/DEVELOPER_WIKI.md).

## Getting Started

### Prerequisites
- Rust 1.85+ (edition 2024)
- CMake 3.16+
- GPU driver support (Vulkan/Metal/DX12 depending on platform)

### Build & Test
```bash
git clone https://github.com/kernalix7/CADKernel.git
cd CADKernel
cargo build --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

### Run the GUI
```bash
cargo run --release
```

## Good First Issues

Look for issues labeled `good first issue` or consider these areas:

- **Add a new primitive** in `crates/modeling/src/primitives/` — follow existing patterns like `make_box`
- **Improve I/O format support** — add missing entity types in STEP/IGES parsers
- **Add constraint types** to the sketch solver in `crates/sketch/`
- **Write examples** — Lua scripts in `examples/lua/` or Python scripts in `examples/python/`
- **Improve documentation** — add doc comments to public APIs, expand wiki pages

## Workflow

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-change`
3. Make your changes, following the conventions below
4. Run the full verification: `cargo build --workspace && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace`
5. Push and open a Pull Request

## Coding Conventions

- All public APIs return `KernelResult<T>` — never panic on user-facing paths
- Use `f64` for all geometric computation
- `Handle<T>` for topology entity references (generational arena indices)
- `Tag` for persistent naming (`Tag::generated(EntityKind, OperationId, local_index)`)
- Geometry constructors validate parameters (radius > 0, segments >= 3, etc.)
- Zero warnings: clippy strict mode is enforced in CI

## Pull Request Checklist

- [ ] The change has a clear scope and rationale
- [ ] Tests are added/updated where applicable
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes
- [ ] `cargo test --workspace` passes
- [ ] Public APIs include documentation comments
- [ ] README / docs are updated when behavior changes

## Commit Message Convention

Use [Conventional Commits](https://www.conventionalcommits.org/):
- `feat:` for new features
- `fix:` for bug fixes
- `docs:` for documentation changes
- `refactor:` for internal improvements without behavior changes
- `test:` for test updates
- `chore:` for maintenance tasks

## Security

For security issues, follow the process in [SECURITY.md](SECURITY.md).
