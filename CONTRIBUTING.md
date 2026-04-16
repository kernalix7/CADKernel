# Contributing to CADKernel

**English** | [한국어](docs/CONTRIBUTING.ko.md)

Thank you for your interest in contributing to CADKernel!

## Architecture Overview

CADKernel is a B-Rep CAD kernel written in Rust, organized as a 9-crate workspace.

**Dependency chain**: `core` → `math` → `geometry` → `topology` → `modeling` / `sketch` → `io` → `viewer`

| Crate | Purpose |
|-------|---------|
| `core` | `KernelResult<T>`, `KernelError`, predicates, constants |
| `math` | `Vec2/3`, `Point2/3`, `Mat3/4`, `Transform`, `Quaternion`, `BBox` |
| `geometry` | Curve/Surface traits, NURBS, tessellation, BVH, intersection |
| `topology` | Half-edge B-Rep: `Vertex`, `Edge`, `HalfEdge`, `Loop`, `Face`, `Shell`, `Solid` |
| `modeling` | 13 primitives, features, booleans, patterns, plugin API, quick API |
| `sketch` | 2D parametric sketch, 24 constraint types, Newton-Raphson solver |
| `io` | 11 file formats: STL, OBJ, glTF, STEP, IGES, DXF, PLY, 3MF, BREP, DWG, DAE |
| `viewer` | egui 0.31 + wgpu 24 desktop GUI, 3D viewport, Lua scripting console |
| `python` | PyO3 bindings (excluded from default workspace build) |

For full architecture documentation, see [docs/DEVELOPER_WIKI.md](docs/DEVELOPER_WIKI.md).

## Development Setup

### Prerequisites

- Rust 1.85+ (edition 2024, MSRV 1.85)
- CMake 3.16+ (required by some geometry dependencies)
- GPU driver with Vulkan, Metal, or DX12 support

### Clone and Build

```bash
git clone https://github.com/kernalix7/CADKernel.git
cd CADKernel
cargo build --workspace
```

### Run the Full Verification Suite

All three checks must pass before any commit or PR:

```bash
cargo build --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

### Run the GUI

```bash
cargo run --release
```

### Python Bindings (Optional)

The `python` crate is excluded from the default workspace build. To build and test it separately:

```bash
cd crates/python
PYO3_PYTHON=/usr/bin/python3 cargo build
cargo test --manifest-path crates/python/Cargo.toml
```

## Code Standards

### Error Handling

- All public APIs return `KernelResult<T>` — never panic on user-facing paths
- Use `with_context()` from `cadkernel-core` to attach context to errors
- No `.unwrap()` on public API paths; use `?` or explicit error handling

### Types and Naming

- Use `f64` for all geometric computation (never `f32`)
- `Handle<T>` for all topology entity references (generational arena indices)
- `Tag` for persistent naming: `Tag::generated(EntityKind, OperationId, local_index)`
- `OperationId` from `model.history.next_operation("op_name")`
- All trait objects require `Send + Sync` bounds: `Arc<dyn Curve + Send + Sync>`

### Quality

- Zero warnings: clippy strict mode is enforced in CI (`-D warnings`)
- Geometry constructors validate all parameters (e.g., radius > 0, segments >= 3)
- New primitives and features must include unit tests

## PR Workflow

1. Fork the repository and create a branch from `main`:
   - New feature: `feature/<name>`
   - Bug fix: `fix/<name>`
   - Urgent fix: `hotfix/<name>`
2. Make your changes following the standards above.
3. Run the full verification suite (all three checks must pass).
4. Push your branch and open a Pull Request against `main`.
5. PRs are merged via **squash merge** — keep your commit history clean but do not worry about squashing manually.

### Commit Message Convention

Use [Conventional Commits](https://www.conventionalcommits.org/):

| Prefix | Use for |
|--------|---------|
| `feat:` | New functionality |
| `fix:` | Bug fixes |
| `refactor:` | Internal improvements, no behavior change |
| `test:` | Test additions or updates |
| `docs:` | Documentation only |
| `chore:` | Build, CI, dependency updates |

### PR Checklist

- [ ] Branch name follows `feature/`, `fix/`, or `hotfix/` convention
- [ ] `cargo build --workspace` passes with zero errors
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes with zero warnings
- [ ] `cargo test --workspace` passes — all tests green
- [ ] New public APIs have doc comments (`///`)
- [ ] Tests are added or updated where applicable
- [ ] CHANGELOG.md is updated if the change is user-visible

## Where to Start

Look for issues labeled `good first issue`. Here are concrete starting points by area:

- **New primitive** — add a file in `crates/modeling/src/primitives/` following the pattern in `box_shape.rs`. Wire it up in `mod.rs`.
- **I/O format improvement** — extend STEP or IGES entity coverage in `crates/io/src/step.rs` or `crates/io/src/brep_format.rs`.
- **Sketch constraint** — implement a new constraint type in `crates/sketch/src/` following the Newton-Raphson solver pattern.
- **Lua script example** — add a `.lua` file in `examples/lua/` demonstrating a modeling workflow.
- **Doc comment pass** — add `///` doc comments to undocumented public functions in any crate.
- **Benchmark** — add a Criterion benchmark in `crates/modeling/benches/` for an operation that lacks one.

## Security

For security vulnerabilities, follow the responsible disclosure process in [SECURITY.md](SECURITY.md).
