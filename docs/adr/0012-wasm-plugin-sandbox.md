# 0012. Wasm-first plugin sandbox via wasmtime

**Status:** Accepted (Track E design)
**Date:** 2026-05-06
**Deciders:** io-engineer, tech-lead

## Context

Track E ships a plugin SDK. Plugins are third-party code that extends CADKernel — adding primitives, file formats, custom features, UI panels.

Threat model:
- Users will install plugins from a marketplace, including from authors they don't know.
- A malicious plugin must not be able to: read arbitrary files, exfiltrate data, execute shell commands, fork the process, or persist beyond the current session without permission.
- A buggy plugin must not crash the host.

Two execution models for plugins:

1. **Native** (`.so` / `.dylib` / `.dll` loaded via libloading) — full speed, full access, no isolation.
2. **Sandbox** (Wasm via wasmtime / wasmer / wasmedge) — slower, limited surface, strong isolation by default.

## Decision

- **Default plugin format: Wasm** with WASI Preview 2 component model.
- **Runtime: wasmtime 24+** (Bytecode Alliance, Apache 2.0).
- **Native plugins are allowed** but require: (1) explicit user opt-in per plugin, (2) source review by maintainer for marketplace distribution, (3) digital signature pinned to author identity.
- **Capability tokens**: plugins request capabilities (`fs:read /path`, `net:https *.example.com`, `clipboard:write`) in a declarative manifest; user grants per session or persistently.

## Alternatives considered

| Option | Rejected because |
|---|---|
| Native-only plugins | Unsafe by default; one bad plugin compromises user data. Industry consensus (VS Code, Figma, Obsidian) is sandbox-first. |
| Lua-only plugins (no Wasm) | Lua is for end-user scripting (ADR 0005), not for shipping compiled plugins. Lua's sandbox is weak (escape via metatables documented). |
| JavaScript plugins (deno-runtime) | Larger runtime; less performant for compute-heavy CAD plugins; node ecosystem brings supply-chain risk. |
| Process isolation (subprocess + IPC) | Stronger isolation but high IPC overhead per call (geometry data is large). Not feasible for per-feature plugins. |
| Linux seccomp / Windows AppContainer | Per-OS, not portable. |
| wasmer or wasmedge | Comparable; wasmtime has stronger institutional backing (Bytecode Alliance), faster Component Model adoption, better Rust-native API. Wasmer was a runner-up. |
| WASI Preview 1 | Superseded by Preview 2 component model. Component model gives us typed interfaces (essential for our Plugin trait FFI). |

## Consequences

**Positive:**
- Sandbox by default → users can install marketplace plugins safely.
- WIT (WebAssembly Interface Types) gives us type-safe ABI between host and plugin (no manual `unsafe extern` glue).
- Cross-platform binary distribution: one `.wasm` works on Linux x86, Linux arm, macOS arm, Windows x86, etc. — no per-OS build matrix for plugin authors.
- Capability-based security model is composable and auditable.
- Performance: wasmtime with PGO + Cranelift JIT is within 1.5–2.5× of native Rust for compute. Acceptable for most plugins; high-perf plugins can request native opt-in.

**Negative:**
- Runtime overhead: ~5 MB binary size for wasmtime; ~1-3 ms cold-start per plugin call.
- Wasm GC and threads still maturing; v1 plugins are single-threaded (host can call multiple plugins in parallel).
- Plugin authors learn WIT in addition to Rust; docs and examples mitigate.

**Neutral:**
- Marketplace QA process is independent of execution model.

## References

- WebAssembly Component Model: <https://component-model.bytecodealliance.org/>
- WASI Preview 2: <https://github.com/WebAssembly/WASI/tree/main/preview2>
- wasmtime: <https://wasmtime.dev/>
- Capability-based security: Miller, *Robust Composition*, 2006.
- Track E roadmap section.
- [ADR 0005](0005-mlua-scripting.md) (Lua for end-user scripting, not plugins).
