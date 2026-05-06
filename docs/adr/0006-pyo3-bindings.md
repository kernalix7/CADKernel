# 0006. PyO3 Python bindings, separate crate, excluded from default workspace build

**Status:** Accepted
**Date:** 2026-05-06
**Deciders:** kernel-engineer, io-engineer

## Context

A Python binding to the kernel is needed for:

- Notebook-driven CAD (Jupyter / Marimo).
- Integration with the scientific-Python ecosystem (numpy, matplotlib, scipy).
- Headless scripting in CI pipelines.
- FreeCAD users who already use Python and want to migrate gradually.

PyO3 is the de-facto Rust-Python binding crate. The question is **how** to integrate it without slowing down regular kernel development.

## Decision

- Create a dedicated crate `crates/python` with PyO3 bindings.
- **Exclude it from the default workspace build** (`Cargo.toml` `workspace.default-members` does not include it).
- Build it explicitly via `cd crates/python && maturin develop` or via a CI matrix entry.
- The crate is a thin wrapper over the public Rust API; no kernel logic lives there.

## Alternatives considered

| Option | Rejected because |
|---|---|
| Bindings included in default build | Every contributor needs Python toolchain (interpreter + headers + maturin) installed. Adds 60+ s to clean build. Blocks contributors who don't care about Python. |
| Bindings in same crate as kernel | Couples kernel release to PyO3 ABI. Hard to test independently. Forces feature-flag complexity throughout the kernel. |
| ctypes / cffi over a C ABI | Loses Rust type safety at boundary; manual memory management; awful UX. |
| RustPython embedded | Slow; immature; doesn't give users access to the real numpy/scipy ecosystem. |
| Python plugin via subprocess + stdio | Forces serialization on every call; no streaming. Acceptable for tool integration but not for interactive scripting. |

## Consequences

**Positive:**
- `cargo build --workspace` runs without Python. Contributors who only touch the kernel are unaffected.
- Python users get a proper PEP 518 wheel via `maturin build`.
- PyO3 ABI break = bump `crates/python/Cargo.toml` only; kernel untouched.
- The binding crate provides idiomatic Python: NumPy interop for vertex arrays, context managers for documents, exceptions wrapping `KernelResult::Err`.

**Negative:**
- Two release artifacts to ship: the desktop binary and the Python wheel.
- Documentation duplication: same API in two languages. Mitigated by generating Python docstrings from Rust doc comments via PyO3's built-in support.
- CI matrix grows by Python-version × OS combinations.

**Neutral:**
- Two scripting surfaces (Lua in-app + Python external) ([ADR 0005](0005-mlua-scripting.md)) — already accepted.

## References

- PyO3: <https://pyo3.rs/>
- Maturin: <https://www.maturin.rs/>
- `crates/python/pyproject.toml`
- `examples/python/` (basic_usage.py, parametric.py)
- [ADR 0005](0005-mlua-scripting.md)
