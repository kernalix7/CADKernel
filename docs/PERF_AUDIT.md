# Performance Audit

## Gate 33: R3 1080p Repaint

**Target:** R3 reference part repaints at 30 fps or better on the 2020 reference-laptop class target.

**Budget:** mean frame time <= 33.3 ms at 1920 x 1080.

**Benchmark:** `crates/viewer/benches/r3_30fps_repaint.rs`

## Methodology

The benchmark loads R3 through the stable `.cadk` API:

```bash
cargo bench -p cadkernel-viewer --bench r3_30fps_repaint
bash scripts/bench_threshold.sh r3_repaint_1080p
```

If `CADKERNEL_R3_PATH` is set, the benchmark loads that fixture with
`Session::load_cadk_from_path`. Otherwise it writes the deterministic
`cadkernel_api::reference_parts::r3_bytes()` payload to
`target/cadkernel-benches/R3.cadk` and then loads that path.

The viewer stages the loaded session into the normal `Scene` path, then renders
against a 1920 x 1080 offscreen `wgpu` texture with:

- the production CAD shader,
- 4x MSAA,
- depth testing,
- the production gradient/grid background path,
- a headless `egui::Context`,
- `egui_wgpu::Renderer` overlay rendering.

The benchmark performs a 3-frame warm-up, then measures 100 repaint frames while
nudging camera yaw to cover the camera-move path. It records mean and p95 frame
time; the hard assertion is the Gate #33 mean <= 33.3 ms threshold.

## Optimizations Applied

- Added a surface-free repaint harness so shader/pipeline compilation and GPU
  submission can be measured on CI or headless Linux without creating a winit
  window.
- Kept scene tessellation outside the timed repaint loop. R3 is loaded and
  staged once; timed frames only update camera uniforms, redraw the fixed vertex
  buffer, tessellate egui output, and submit the offscreen render pass.
- Added the built-in `r3_repaint_1080p` budget to `scripts/bench_threshold.sh`
  at 33,300,000 ns.

## Results

Pending local measurement in this lane.

## Remaining Gaps

- `crates/viewer/Cargo.toml` currently registers only `first_paint` with
  `harness = false`. Register `r3_30fps_repaint` the same way before CI wires
  it as a hard gate.
- CI wiring for the new viewer bench is pending.
