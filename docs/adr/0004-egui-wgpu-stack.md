# 0004. egui + wgpu desktop GUI stack

**Status:** Accepted
**Date:** 2026-05-06
**Deciders:** ui-engineer, tech-lead

## Context

CADKernel ships as a native desktop app (Linux/macOS/Windows). The viewer needs:

- Hardware-accelerated 3D rendering (millions of triangles at 60 fps).
- A panel/dock UI for property inspectors, history tree, console, and toolbars.
- Cross-platform with one codebase (no per-OS branches in app code).
- 4× MSAA, custom shaders (Blinn-Phong, gradient sky, wireframe with depth offset).
- Pure Rust if possible (avoid Qt/C++ build complexity).

## Decision

- **Rendering**: `wgpu 24` (the Rust implementation of WebGPU; targets Vulkan/Metal/DX12/GL).
- **Immediate-mode GUI**: `egui 0.31`.
- **Window/event loop**: `winit 0.30`.

## Alternatives considered

| Option | Rejected because |
|---|---|
| Qt 6 (cxx-qt) | Excellent UI but C++ build chain (qmake/CMake), licensing (GPL/LGPL/commercial complexity), 200+ MB redistribution. Heavy. |
| GTK4 (gtk-rs) | Linux-first; macOS/Windows ports work but feel non-native. CSS theming clashes with custom CAD widgets. |
| Slint | Promising but younger ecosystem; custom 3D viewport integration less mature. Watch list, not first pick. |
| iced | Elm-style retained mode; weaker integration with custom 3D viewport (we'd be embedding a wgpu surface anyway). |
| Bevy UI | Tied to Bevy ECS; using Bevy for a non-game CAD app brings ECS overhead we don't need. |
| Native + per-OS UI (Cocoa, Win32, GTK) | 3× UI code. No. |
| Web (Tauri + browser UI) | Not native enough for CAD muscle memory; viewer FPS via web worker is worse than native wgpu. |
| ImGui (imgui-rs) | C++ binding, less Rust-idiomatic than egui. |

## Consequences

**Positive:**
- Pure Rust toolchain; `cargo build` produces a ready binary.
- wgpu abstracts Vulkan / Metal / DX12 with one shader language (WGSL).
- egui is immediate-mode → zero retained-state bugs (no "view out of sync with model" class).
- 4× MSAA across all pipelines (solid, wireframe, transparent, gradient) consistent on all backends.
- Hot-reload of UI via egui's stateless model is trivial.

**Negative:**
- Immediate-mode UI re-evaluates layout every frame → CPU cost. Mitigated by egui's region-based caching; not visible at 60 fps for our panel counts.
- egui's layout algorithm is less powerful than retained UI for advanced panel docking. We've added a custom dock-host layer ([Track B §5.0](../COMMERCIAL_CAD_ROADMAP.md)).
- wgpu API churns across releases; pinning to 24 means ~6-month upgrade cadence.
- WGSL is newer than GLSL/HLSL; some shader patterns less documented. Mitigated by referencing wgpu's own examples.

**Neutral:**
- File picker, native menus: handled by `rfd` and `muda` crates, well-supported.

## References

- wgpu: <https://wgpu.rs/>
- egui: <https://www.egui.rs/>
- winit: <https://github.com/rust-windowing/winit>
- WGSL spec: <https://www.w3.org/TR/WGSL/>
- [CLAUDE.md §4 Coding Conventions / Viewer DO NOT CHANGE](../../CLAUDE.md)
