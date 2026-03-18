# CADKernel Verification Checklist

커밋 전 코드 품질 검증 체크리스트. 매 릴리스/PR 마다 수행.

Pre-commit code quality verification checklist. Run before every release/PR.

---

## 1. Build & Toolchain

| # | Item | Command | Pass Criteria |
|---|------|---------|---------------|
| 1.1 | Workspace build | `cargo build --workspace` | Zero errors |
| 1.2 | Clippy lint | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Zero warnings |
| 1.3 | Format check | `cargo fmt --all -- --check` | Zero diffs |
| 1.4 | All tests | `cargo test --workspace` | All pass (currently 230) |
| 1.5 | Benchmarks compile | `cargo bench --no-run -p cadkernel-modeling` | Compiles |

---

## 2. Security

| # | Item | What to Check |
|---|------|---------------|
| 2.1 | Input validation | STL triangle count cap, OBJ index bounds, no unbounded allocations |
| 2.2 | No `unwrap()` on user input | Geometry constructors, file parsers, topology lookups |
| 2.3 | Division by zero guards | Mass properties, NURBS de_boor, normalize functions |
| 2.4 | No `todo!()` on reachable paths | STEP/IGES parsers, fillet, draft, split — guard or return `Err` |
| 2.5 | SVG/text escaping | XML entity escaping in SVG output |
| 2.6 | Memory bounds | Array index checks in mesh operations, entity store lookups |

---

## 3. Stability

| # | Item | What to Check |
|---|------|---------------|
| 3.1 | No infinite loops | `loop_half_edges()` has max iteration guard |
| 3.2 | NaN propagation | `asin` clamped to [-1,1] in Rodrigues, normalize handles zero vectors |
| 3.3 | Overflow protection | EntityStore generation counter, edge tag multipliers |
| 3.4 | Camera edge cases | `tick()` duration > 0, pitch clamped, roll normalized |
| 3.5 | Empty input handling | `compute_bounds` for empty mesh, zero-face models |
| 3.6 | Topology consistency | Euler characteristic validation, half-edge integrity |

---

## 4. Privacy & Personal Information

| # | Item | What to Check |
|---|------|---------------|
| 4.1 | Hardcoded system paths | No `/home/username`, `C:\Users\` in tracked files |
| 4.2 | API keys / tokens / secrets | No hardcoded credentials, passwords, or API keys |
| 4.3 | IP addresses | No hardcoded IP addresses |
| 4.4 | Hardware info | No GPU/CPU model names hardcoded (runtime query OK) |
| 4.5 | Author info (intentional) | Email/name in Cargo.toml, LICENSE, README — expected for open-source |
| 4.6 | Credential files | No `.env`, `.pem`, `.key`, `.secrets` files in repo |
| 4.7 | `target/` directory | Properly listed in `.gitignore` |

---

## 5. Performance

| # | Item | What to Check |
|---|------|---------------|
| 5.1 | BFS complexity | Smooth-group BFS neighbor lookups — O(N) per vertex |
| 5.2 | No O(n²) in hot paths | `edges_of_face`, `faces_around_vertex`, `find_overlapping_face_pairs` |
| 5.3 | Heap allocations | ViewCube per-frame allocations minimized (single Mesh) |
| 5.4 | History buffer | `ModelHistory::record()` uses VecDeque, not Vec::remove(0) |
| 5.5 | File I/O | STL/OBJ streaming vs full-file read for large models |
| 5.6 | GPU resources | MSAA textures recreated only on resize |
| 5.7 | Benchmarks | `cargo bench -p cadkernel-modeling` — 14 benchmarks pass |

---

## 6. Documentation Accuracy

| # | Item | Files | What to Check |
|---|------|-------|---------------|
| 6.1 | Test count | DEVELOPER_WIKI.md/ko, README.md/ko | Matches `cargo test --workspace` output |
| 6.2 | Benchmark count | DEVELOPER_WIKI.md, README.md/ko | Matches `.bench_function` count (currently 14) |
| 6.3 | Shading parameters | wiki/Crate:-viewer.md, MEMORY.md | ambient=0.15, spec_str=0.15, shininess=128 |
| 6.4 | Headlight offset | wiki/Crate:-viewer.md, MEMORY.md | right×0.5 + up×0.7 |
| 6.5 | Crease angle | wiki/Crate:-viewer.md, MEMORY.md | 60° (matches SMOOTH_ANGLE_DEG) |
| 6.6 | File extension | README.md/ko, DEVELOPER_WIKI.ko.md | `.cadk` (not `.cadkernel`) |
| 6.7 | Crate existence | README.md/ko architecture table | Mark planned crates as `(planned)` |
| 6.8 | Function signatures | wiki, DEVELOPER_WIKI.ko.md | Match actual code (animate_to, rodrigues, traits) |
| 6.9 | CHANGELOG bilingual | CHANGELOG.md, CHANGELOG.ko.md | Both updated with same entries |
| 6.10 | MEMORY.md | Auto-memory file | Values match code (headlight, CW/CCW, params) |

---

## 7. Code Quality

| # | Item | What to Check |
|---|------|---------------|
| 7.1 | Error handling | `KernelResult<T>` for fallible ops, no panic on user paths |
| 7.2 | Consistent naming | Snake_case functions, CamelCase types, UPPER_SNAKE constants |
| 7.3 | Dead code | No unused imports, functions, or modules |
| 7.4 | Type safety | `Handle<T>` generational arena — no raw index access |
| 7.5 | Cross product convention | `cross3(f, up)` everywhere — NEVER swap |
| 7.6 | Angle normalization | `wrap_angle()` for all roll/yaw values that can accumulate |

---

## Quick One-liner

```bash
cargo fmt --all && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace
```

---

## Known Issues (tracked for future fix)

### CRITICAL (all fixed)
- [x] `arbitrary_perpendicular` uses `.unwrap()` — **FIXED**: `.unwrap_or(Vec3::X)`
- [x] Binary STL: no triangle count upper bound — **FIXED**: MAX_STL_TRIANGLES = 50M
- [x] `todo!()` panics in STEP/IGES public APIs — **FIXED**: `Err(IoError)`
- [x] `point_in_solid()` ray-casting uses incorrect plane-based test — **FIXED**: proper 2D point-in-polygon test
- [x] `classify_face()` offsets test point in wrong direction — **FIXED**
- [x] `compute_mass_properties` divides by near-zero volume — **FIXED**: early return guard
- [x] EntityStore generation counter u32 overflow — **FIXED**: u64

### HIGH (all fixed)
- [x] Infinite domain breaks `bounding_box`/`project_point` for Plane/Line — **FIXED**: analytical overrides + finite fallback
- [x] No radius validation in Sphere, Cone, Torus constructors — **FIXED**: `KernelResult` + validation
- [x] `NurbsCurve::de_boor` can divide by zero weight — **FIXED**: w < 1e-14 guard
- [x] `loop_half_edges()` can loop infinitely — **FIXED**: MAX_LOOP = 100K
- [x] Duplicate edges in all primitives — **FIXED**: EdgeCache dedup (shared half-edges)
- [x] Binary STL writer u32 overflow — **FIXED**: `KernelResult<Vec<u8>>`
- [x] `asin` can produce NaN in ScreenOrbit — **FIXED**: clamp(-1, 1) before asin
- [x] Angle constraint `tan()` singularity — **FIXED**: atan2(cross, dot)
- [x] No bounds check on PointId indices — **FIXED**: `.get()` fallback
- [x] Simple viewer orbit direction inverted — **FIXED**: negated dx/dy

### MEDIUM (all fixed)
- [x] NavConfig settings not used — **FIXED**: cube_size, cube_opacity, orbit_steps applied
- [x] `validate()` Euler characteristic — **FIXED**: V-E+F=2 check added
- [x] SVG output lacks XML entity escaping — **FIXED**: `xml_escape()` helper
- [x] `WorkPlane::new` doesn't orthogonalize — **FIXED**: Gram-Schmidt
- [x] BFS smooth-group O(n²) — **FIXED**: edge-based local adjacency

All 22 known issues have been resolved.
