# 0005. mlua (Lua 5.4 vendored) for in-app scripting

**Status:** Accepted
**Date:** 2026-05-06
**Deciders:** kernel-engineer, ui-engineer

## Context

CADKernel needs an in-app scripting language for:

- Macros (record + replay user actions).
- Parametric scripts (computed dimensions, advanced features).
- Plugin authors who don't want to recompile Rust.
- Quick automation by end users.

Requirements:

- Embeddable in Rust without unsafe ceremony.
- Sandboxable (no fs/net by default).
- Send + Sync (or carefully scoped) so the scripting engine can run alongside the kernel.
- Familiar / easy syntax for engineers (not Rust, not C++).
- Small runtime footprint.

## Decision

Use **mlua 0.10** with features `lua54, vendored, send`, exposing 22 `cad.*` functions for primitives, booleans, transforms, features, query, I/O, utility (see `crates/viewer/src/scripting.rs`).

`vendored` builds Lua 5.4 from source (no system dep). `send` enables `Send` on Lua state so we can move it across threads.

## Alternatives considered

| Option | Rejected because |
|---|---|
| Rhai | Pure Rust, attractive — but slower than Lua, smaller user community, less mature stdlib. Rejected for v1; revisit if Lua adoption is poor. |
| Python (PyO3 inproc) | PyO3 is strictly for external bindings ([ADR 0006](0006-pyo3-bindings.md)). Embedding a Python interpreter inside the desktop binary adds 30+ MB and a global GIL that conflicts with our Rayon model. |
| JavaScript (rusty_v8 / boa) | V8 is huge (>10 MB) and complex to embed cross-platform. Boa is too young. |
| Wasm (wasmtime) sandboxed scripts | Excellent for plugins ([Track E §1](../COMMERCIAL_CAD_ROADMAP.md)), wrong for ad-hoc user scripts (no REPL, complex toolchain). Plugins use Wasm; macros use Lua. |
| Custom DSL | Years of compiler work + nobody knows it. No. |
| Ruby (mruby) | Embeddable but less common in engineering. |
| Scheme / Lisp | Beautiful but small audience among CAD users. |

## Consequences

**Positive:**
- 22 `cad.*` API functions cover the primitive / boolean / transform / feature / query / I/O surface, demonstrating viability.
- Vendored build → no Lua install required by end users; one-binary distribution.
- Sandbox: by default no `os`, `io`, `package` modules; users can opt into `--unsafe-script` flag for explicit permission.
- Lua 5.4 native integer type maps cleanly to f64 conversion at API boundary; no surprise truncation.
- Familiar to game developers (huge Lua user base) and many engineering communities (FreeCAD has Python; AutoCAD has AutoLISP — Lua sits between in complexity).

**Negative:**
- Lua's 1-based indexing surprises some users; documented prominently in tutorials.
- Lua 5.4 is single-threaded per state; multi-threading requires multiple states or careful coroutine use. Mitigated: each script runs in its own state; long-running scripts get a worker thread.
- mlua adds ~600 KB to release binary. Acceptable.

**Neutral:**
- Two scripting surfaces in CADKernel: Lua (in-app) and Python (external bindings via PyO3, [ADR 0006](0006-pyo3-bindings.md)). Documented separation; both backed by a shared `cadkernel-modeling` API.

## References

- mlua: <https://github.com/mlua-rs/mlua>
- Lua 5.4 reference: <https://www.lua.org/manual/5.4/>
- `crates/viewer/src/scripting.rs`
- `examples/lua/` (5 demo scripts)
- [ADR 0006](0006-pyo3-bindings.md)
