# Crate: cadkernel-viewer

> 네이티브 데스크톱 GUI 애플리케이션 (egui 0.31 + wgpu 24.x + winit 0.30)

---

## 역할

CADKernel의 3D 뷰포트 및 인터랙티브 GUI를 담당합니다. `cadkernel-io`, `cadkernel-modeling`, `cadkernel-topology`, `cadkernel-math` 크레이트에 의존합니다.

## 모듈 구조

| 파일 | 역할 |
|------|------|
| `app.rs` | 애플리케이션 상태, winit 이벤트 루프, 액션 처리, 카메라 애니메이션 |
| `render.rs` | GPU 렌더링 (wgpu 파이프라인, 유니폼, 셰이더), Camera, 그리드, 수학 헬퍼 |
| `gui.rs` | egui UI 패널 (메뉴바, 모델 트리, 속성, 상태 바), ViewCube, 축 인디케이터 |
| `nav.rs` | 마우스 내비게이션 설정 (5개 프리셋), 감도, 줌, 애니메이션 설정 |
| `lib.rs` | 모듈 선언 + `run_gui()` 진입점 |

## GPU 어댑터 선택

`GpuState::pick_adapter()` 순차 폴백:

1. **HighPerformance** — 외장 GPU (Vulkan/Metal/DX12)
2. **LowPower** — 내장 GPU
3. **Software fallback** — `force_fallback_adapter: true` (llvmpipe/swiftshader/warp)

선택된 어댑터 이름과 백엔드는 `stderr`에 로그됩니다.

## 셰이딩 모델

### Blinn-Phong 라이팅

| 컴포넌트 | 공식 | 파라미터 |
|----------|------|----------|
| Ambient | `base_color × 0.15` | 고정 |
| Diffuse | `base_color × max(N·L, 0) × 0.85` | `light_dir` |
| Specular | `pow(max(N·H, 0), shininess) × strength` | `params.y`=강도(0.15), `params.z`=광택도(128) |

- **카메라 헤드라이트**: 광원이 카메라 우상단(right×0.5 + up×0.7)에서 따라감
- 최종 색상은 `clamp(0, 1)` 적용
- `Uniforms.eye_pos`로 뷰 방향 계산 (specular half-vector)

## 카메라 시스템

### 파라미터

| 필드 | 타입 | 설명 |
|------|------|------|
| `target` | `[f32; 3]` | 카메라 주시점 |
| `distance` | `f32` | 주시점으로부터 거리 |
| `yaw` | `f32` | 수평 회전 (라디안) |
| `pitch` | `f32` | 수직 회전 (라디안) |
| `roll` | `f32` | 인플레인 롤 (시계 방향 양수, 시점 축 기준) |
| `aspect` | `f32` | 화면 비율 |
| `fovy` | `f32` | 수직 시야각 |
| `projection` | `Projection` | Perspective / Orthographic |

### Eye 위치 계산

```
eye = target + distance × (cos(yaw)·cos(pitch), sin(yaw)·cos(pitch), sin(pitch))
```

### 좌표계 규약

`cross3(f, up)` — 카메라 forward × world up = screen right:
- 이 규약이 전체 뷰어에서 일관되게 사용됨 (`look_at`, `view_matrix`, `screen_right`, `screen_up`, `FACE_TEXT_RIGHT`)
- 마우스 orbit: `yaw -= dx * sensitivity` (드래그 방향과 회전 방향 일치)

### Roll 반영

`view_matrix()`, `screen_right()`, `screen_up()` 메서드는 `roll` 값을 반영합니다:
- `roll == 0`: 월드 업 벡터 사용 (기본)
- `roll != 0`: 기본 right/up 벡터를 forward 축 기준으로 회전
- 뷰 스냅 시 roll은 가장 가까운 90° 배수로 스냅 (`snap_roll_90(roll, prev_roll)`)
- 45° 중간점에서는 이전 롤 위치(`prev_roll`) 방향으로 스냅 (0°→45°→0°, 90°→45°→90°)
- 모든 각도는 `wrap_angle()`로 (−π, π] 범위로 정규화 — 무한 누적 방지
- Top/Bottom 뷰는 현재 yaw 유지 (pitch만 변경)

### 카메라 애니메이션

뷰 전환 시 smooth-step 이징 (3t² − 2t³) 적용:
- 최단 경로 yaw 보간 (±π 범위 래핑)
- 설정에서 활성화/비활성화 및 지속 시간(0.1~1.0초) 조절 가능

## ViewCube

### 구조

24개 chamfer 정점으로 구성된 절두 큐브 (26 폴리곤):
- **6 팔각형 면**: FRONT, BACK, RIGHT, LEFT, TOP, BOTTOM
- **8 삼각형 코너**: 큐브 꼭짓점의 chamfer 면
- **12 엣지 쿼드**: 인접 면 사이의 베벨 스트립 (face normal offset 방식으로 3D 사각형 생성)

모든 비-호버 폴리곤은 하나의 `epaint::Mesh`로 합쳐서 렌더링됩니다 (팬 삼각분할). 이렇게 하면 egui의 안티앨리어싱 페더링이 내부 엣지에 적용되지 않아 인접 폴리곤 사이의 이음새 선이 제거됩니다. 불투명 채우기(`from_rgb`), XYZ 축 인디케이터는 큐브 위에 렌더링. 호버된 폴리곤은 `convex_polygon`으로 별도 렌더링 (스트로크 하이라이트).

모든 26개 폴리곤은 깊이 정렬 (back-to-front) 후 렌더링되며, 각각 고유 법선 벡터로 라이팅됩니다.

### 인터랙션

| 대상 | 호버 | 클릭 동작 |
|------|------|-----------|
| 면 (6개) | 밝은 하이라이트 | 표준 뷰 스냅 |
| 엣지 (12개) | 밝은 하이라이트 (quad hit-test) | 인접 면 중간 각도 스냅 |
| 코너 (8개) | 밝은 하이라이트 | 아이소메트릭 뷰 스냅 |

### 부가 요소

- **방향 조명**: top-right-front 광원 (ambient + diffuse)
- **드롭 섀도**: 반투명 원형 그림자
- **오비트 링**: 나침반 레이블 (F/R/B/L) + 틱 마크
- **화살표 버튼**: ▲▼◀▶ 스크린 스페이스 45° 오비트
- **CW/CCW 버튼**: ↺↻ 인플레인 롤 (45° 단위)
- **사이드 버튼**: Home (Isometric), P/O (투영 전환), ☰ 드롭다운 메뉴
- **드롭다운 메뉴**: Orthographic/Perspective 전환, Isometric, Fit All
- **면 레이블**: 큐브에 각인된 텍스트 (카메라 회전에 따라 함께 회전)

## 축 인디케이터 (좌하단)

- XYZ 양방향 라인 (양방향: 밝은 색 + 반대 방향 페이드)
- Roll 반영 렌더링 (`camera.screen_right()` / `screen_up()` 사용)

## 내비게이션 프리셋

| 스타일 | 오비트 | 팬 | 줌 |
|--------|--------|-----|-----|
| FreeCAD Gesture | LMB | RMB/MMB | Scroll / Ctrl+RMB |
| Blender | MMB | Shift+MMB | Scroll / Ctrl+MMB |
| SolidWorks | MMB | Ctrl+MMB | Scroll / Shift+MMB |
| Inventor | Shift+MMB | MMB | Scroll |
| OpenCascade | MMB | Shift+MMB | Scroll / Ctrl+MMB |

## 디스플레이 모드

| 모드 | 설명 |
|------|------|
| As Is | 기본 솔리드 렌더링 (Shading과 동일) |
| Points | 정점만 표시 (조명 없음) |
| Wireframe | 엣지만 표시 (조명 없음) |
| Hidden Line | 배경색 솔리드 + 엣지 오버레이 |
| No Shading | 단색 솔리드 (diffuse 없음) |
| Shading | Blinn-Phong 조명 솔리드 (기본값) |
| Flat Lines | 조명 솔리드 + 엣지 오버레이 |
| Transparent | 반투명 솔리드 |

## 설정 다이얼로그

### 3D View
- 축 인디케이터 표시 (`show_axes_indicator`)
- FPS 카운터 표시 (`show_fps`)
- 기본 투영 방식 (`default_projection`)

### Navigation
- ViewCube: 표시/숨김, 크기, 투명도, 오비트 스텝, 스냅 토글
- 오비트 스타일: 5개 프리셋 선택
- 감도: 오비트, 팬, 줌, 줌 반전
- 애니메이션: 활성화/비활성화, 지속 시간 슬라이더

### Lighting
- 조명 활성화/비활성화 (`enable_lighting`)
- 광도 (`light_intensity`)
- 방향 XYZ (`light_dir`)

## 키보드 단축키

| 키 | 동작 |
|----|------|
| 1 / Numpad1 | Front 뷰 |
| Ctrl+1 | Back 뷰 |
| 3 / Numpad3 | Right 뷰 |
| Ctrl+3 | Left 뷰 |
| 7 / Numpad7 | Top 뷰 |
| Ctrl+7 | Bottom 뷰 |
| 0 / Numpad0 | Isometric 뷰 |
| 5 / Numpad5 | 투영 전환 |
| D | 디스플레이 모드 순환 |
| V | 모델 맞춤 |
| G | 그리드 토글 |
| Esc | 종료 |

## GuiAction 열거형

GUI 이벤트는 `GuiAction` 열거형으로 수집되어 프레임 끝에 `process_actions()`에서 처리됩니다:

```rust
pub(crate) enum GuiAction {
    NewModel, OpenFile, SaveFile, ImportFile,
    ExportStl, ExportObj, ExportGltf,
    CreateBox, CreateCylinder, CreateSphere,
    ResetCamera, FitAll, ToggleProjection,
    SetDisplayMode, SetStandardView,
    SetCameraYawPitch,
    ScreenOrbit,    // 스크린 스페이스 오비트
    RollDelta,      // 인플레인 롤
    ToggleGrid,
}
```

## Uniforms 구조

```rust
pub struct Uniforms {
    pub view_proj: [[f32; 4]; 4],   // MVP 행렬
    pub light_dir: [f32; 4],         // 광원 방향 (헤드라이트)
    pub base_color: [f32; 4],        // 기본 색상 + 알파
    pub params: [f32; 4],            // x=조명, y=specular 강도, z=shininess
    pub eye_pos: [f32; 4],           // 카메라 위치 (specular 계산용)
}
```

## 메시 노말 (render.rs)

`mesh_to_vertices()` 함수는 스무스 그룹 BFS 알고리즘으로 코너 노말을 계산합니다:

1. 각 정점에서 인접 면 리스트 구축 (`vert_faces`)
2. BFS로 크리즈 각도(60°) 내의 면을 전이적으로 그룹화
3. 같은 그룹의 모든 면 코너에 동일한 평균 노말 할당
4. 그룹 간 경계는 날카로운 엣지로 보존 (텍스트 각인, 챔퍼 등)

이 방식은 per-corner 크리즈 각도(미세 불연속 발생)와 per-vertex 평균(날카로운 엣지 파괴)의 문제를 동시에 해결합니다.

**정점 공유**: `tessellate_solid()`는 bit-exact 위치 매칭 (`f64::to_bits`)으로 정점을 중복 제거하여 인접 면이 동일한 정점 인덱스를 공유합니다. 이를 통해 BFS가 면 경계를 넘어 스무스 노말을 올바르게 계산합니다.

## 4x MSAA (Multi-Sample Anti-Aliasing)

스무스 서피스에서 삼각형 경계선이 보이는 **마하 밴드** 현상의 근본 해결책.

| 항목 | 설명 |
|------|------|
| 샘플 수 | `MSAA_SAMPLES = 4` |
| MSAA 컬러 텍스처 | `create_msaa_texture()` — surface format, sample_count=4, RENDER_ATTACHMENT |
| MSAA 뎁스 텍스처 | `create_depth_texture()` — sample_count=4 |
| 렌더 패스 | Scene pass: `view = msaa_view`, `resolve_target = surface_view` |
| egui 패스 | sample_count=1 (2D UI, 리졸브된 surface에 직접 렌더링) |

4개 파이프라인 (solid, wireframe, transparent, gradient) 모두 `MultisampleState { count: 4 }` 적용. 리사이즈 시 `msaa_view` + `depth_view` 재생성.

## 수학 유틸리티 (render.rs)

| 함수 | 용도 |
|------|------|
| Rodrigues rotation (inline) | 임의 축 기준 벡터 회전 — `ScreenOrbit` 핸들러 내 인라인 구현 (app.rs) |
| `normalize3`, `cross3`, `dot3`, `sub3` | 3D 벡터 연산 |
| `look_at`, `perspective`, `orthographic` | 뷰/투영 행렬 생성 |
| `mat4_mul` | 4×4 행렬 곱 |

---

# Crate: cadkernel-viewer (English)

> Native desktop GUI application (egui 0.31 + wgpu 24.x + winit 0.30)

## Role

Provides the 3D viewport and interactive GUI for CADKernel. Depends on `cadkernel-io`, `cadkernel-modeling`, `cadkernel-topology`, `cadkernel-math`.

## Module Structure

| File | Role |
|------|------|
| `app.rs` | Application state, winit event loop, action processing, camera animation |
| `render.rs` | GPU rendering (wgpu pipeline, uniforms, shaders), Camera, grid, math helpers |
| `gui.rs` | egui UI panels (menubar, model tree, properties, status bar), ViewCube, axis indicator |
| `nav.rs` | Mouse navigation settings (5 presets), sensitivity, zoom, animation |
| `lib.rs` | Module declarations + `run_gui()` entry point |

## GPU Adapter Selection

`GpuState::pick_adapter()` sequential fallback:

1. **HighPerformance** — discrete GPU (Vulkan/Metal/DX12)
2. **LowPower** — integrated GPU
3. **Software fallback** — `force_fallback_adapter: true` (llvmpipe/swiftshader/warp)

Selected adapter name and backend are logged to `stderr`.

## Shading

Blinn-Phong lighting model:
- **Ambient**: base illumination (0.15)
- **Diffuse**: Lambert term `max(0, N·L)`
- **Specular**: Blinn-Phong half-vector `pow(max(0, N·H), 128) × 0.15`

Camera headlight: light follows camera position with upper-right offset (right×0.5 + up×0.7) for real-time reflection updates during orbit.

## Camera

Orbit camera: yaw/pitch/roll/distance around a target point.

- `view_matrix()`: applies roll rotation — `rolled_up = [-r*sin(θ) + u*cos(θ)]`
- `screen_right()`: `r*cos(θ) + u*sin(θ)`, `screen_up()`: `-r*sin(θ) + u*cos(θ)`
- View snaps: roll snaps to nearest 90° via `snap_roll_90(roll, prev_roll)` — at 45° midpoint, snaps toward previous roll position. All angles normalized to (−π, π] via `wrap_angle()`

### Camera Animation

View transitions use smooth-step easing (3t² − 2t³):
- Shortest-path yaw interpolation (±π wrapping)
- Enable/disable and duration (0.1–1.0s) adjustable in settings

## ViewCube

### Structure

Truncated cube with 24 chamfer vertices (26 polygons):
- **6 octagonal faces**: FRONT, BACK, RIGHT, LEFT, TOP, BOTTOM
- **8 triangular corners**: chamfered cube vertex faces
- **12 edge quads**: bevel strips between adjacent faces (face normal offset method)

All non-hovered polygons are rendered as a single `epaint::Mesh` (fan-triangulated). This eliminates egui's anti-aliasing feathering on internal edges, removing visible seam lines between adjacent polygons. Opaque fill (`from_rgb`), XYZ axis indicator rendered ON TOP. Hovered polygon rendered separately via `convex_polygon` with stroke highlight.

All 26 polygons are depth-sorted (back-to-front) and rendered with per-polygon normal-based lighting.

### Interaction

| Target | Hover | Click |
|--------|-------|-------|
| Face (6) | Bright highlight | Standard view snap |
| Edge (12) | Bright highlight (quad hit-test) | Mid-angle snap between adjacent faces |
| Corner (8) | Bright highlight | Isometric view snap |

### Extras

- **Directional lighting**: top-right-front light source (ambient + diffuse)
- **Drop shadow**: semi-transparent circle
- **Orbit ring**: compass labels (F/R/B/L) + tick marks
- **Arrow buttons**: ▲▼◀▶ screen-space 45° orbit
- **CW/CCW buttons**: ↺↻ in-plane roll (45° increments)
- **Side buttons**: Home (Isometric), P/O (projection toggle), ☰ dropdown menu
- **Dropdown menu**: Orthographic/Perspective toggle, Isometric, Fit All
- **Face labels**: engraved text (rotates with cube)

## Axis Indicator (bottom-left)

- XYZ bidirectional lines (bright color + faded opposite direction)
- Roll-aware rendering (uses `camera.screen_right()` / `screen_up()`)

## Mesh Normals (render.rs)

`mesh_to_vertices()` computes corner normals via smooth-group BFS algorithm:

1. Build per-vertex face adjacency list (`vert_faces`)
2. BFS groups faces transitively within crease angle (60°)
3. All face corners in same group receive exact same averaged normal
4. Boundaries between groups are preserved as sharp edges (text engravings, chamfers, etc.)

This approach solves both per-corner crease-angle issues (subtle discontinuities) and per-vertex averaging issues (sharp edge destruction).

**Vertex sharing**: `tessellate_solid()` deduplicates vertices via bit-exact position matching (`f64::to_bits`), so adjacent faces share the same vertex index. This allows BFS to correctly compute smooth normals across face boundaries.

## 4x MSAA (Multi-Sample Anti-Aliasing)

Fundamental solution for **Mach band** artifacts — visible triangle edges on smooth surfaces caused by 2nd derivative discontinuity in normal interpolation.

| Item | Description |
|------|-------------|
| Sample count | `MSAA_SAMPLES = 4` |
| MSAA color texture | `create_msaa_texture()` — surface format, sample_count=4, RENDER_ATTACHMENT |
| MSAA depth texture | `create_depth_texture()` — sample_count=4 |
| Render pass | Scene pass: `view = msaa_view`, `resolve_target = surface_view` |
| egui pass | sample_count=1 (2D UI, renders directly to resolved surface texture) |

All 4 pipelines (solid, wireframe, transparent, gradient) use `MultisampleState { count: 4 }`. Both `msaa_view` and `depth_view` are recreated on resize.

## Math Utilities (render.rs)

| Function | Purpose |
|----------|---------|
| Rodrigues rotation (inline) | Rotate vector around arbitrary axis — inline implementation in `ScreenOrbit` handler (app.rs) |
| `normalize3`, `cross3`, `dot3`, `sub3` | 3D vector operations |
| `look_at`, `perspective`, `orthographic` | View/projection matrix generation |
| `mat4_mul` | 4×4 matrix multiplication |
