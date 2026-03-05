# Implementation Status

> 현재 단계: **Application Phase 2** (pre-alpha) | 테스트: **300개+** | Clippy: **0 warnings**

## Phase 개요

| Phase | 이름 | 상태 | 테스트 |
|:-----:|------|:----:|:------:|
| 1 | Foundation | ✅ 완료 | 기본 |
| 2 | Persistent Naming + Boolean | ✅ 완료 | +30 |
| 3 | Parametric + Sketch + I/O | ✅ 완료 | +50 |
| 4 | Core Hardening | ✅ 완료 | +20 |
| 5 | Mass Properties + Sweep | ✅ 완료 | +9 |
| 6 | Loft + Pattern | ✅ 완료 | +10 |
| 7 | Chamfer + I/O Import | ✅ 완료 | +23 |
| 8 | Modeling Enhancements | ✅ 완료 | +8 |
| 9 | Math & Geometry Enhancements | ✅ 완료 | +17 |
| 10 | Quality & Testing | ✅ 완료 | +10 |
| 11 | I/O Format Expansion | ✅ 완료 | +14 |
| 12 | Rustdoc Documentation | ✅ 완료 | +3 |
| 13 | High Priority Features | ✅ 완료 | +15 |
| 14 | Geometry & Manufacturing | ✅ 완료 | +12 |
| 15 | Infrastructure | ✅ 완료 | +16 |
| 16 | Industry Formats | ✅ 완료 | +18 |
| 17 | Quality & Advanced | ✅ 완료 | +10 |
| App 1 | Native GUI Application | ✅ 완료 | — |
| **App 2** | **ViewCube & Camera** | **🚧 진행 중** | — |

---

## Phase 1: Foundation ✅

핵심 커널 아키텍처 구축.

- [x] Cargo workspace (7 크레이트 모노레포)
- [x] `cadkernel-math`: Vec2/3/4, Point2/3, Mat3/4, Transform, Quaternion, Ray3, BoundingBox
- [x] `cadkernel-geometry`: Curve/Surface 트레이트 + Line, Arc, Circle, NURBS 구현
- [x] `cadkernel-topology`: Half-edge B-Rep, EntityStore, Handle<T>
- [x] CI/CD: GitHub Actions (fmt + clippy + test)
- [x] 프로젝트 문서 (README, LICENSE, CONTRIBUTING, SECURITY, CODE_OF_CONDUCT)

## Phase 2: Persistent Naming + Boolean ✅

파라메트릭 재구축 기반 + 불리언 연산.

- [x] Persistent Naming: Tag, NameMap, ShapeHistory, OperationId
- [x] Geometry-Topology 바인딩 (feature flag `geometry-binding`)
- [x] Surface-Surface Intersection: Plane-Plane, Plane-Sphere, Plane-Cylinder, Sphere-Sphere
- [x] Line-Surface Intersection: Line vs Plane/Sphere/Cylinder
- [x] Boolean Operations: Union, Subtract, Intersect (Broad Phase → Classify → Evaluate)

## Phase 3: Parametric + Sketch + I/O ✅

2D 스케치, Feature Operations, 파일 출력.

- [x] 2D 파라메트릭 스케치: Point, Line, Arc, Circle + **14개** 제약 조건
- [x] Newton-Raphson 솔버 + Armijo 백트래킹
- [x] Extrude 연산 (auto-tagging)
- [x] Revolve 연산 (N-segment rotation, auto-tagging)
- [x] Primitive Builders: Box, Cylinder, Sphere
- [x] 테셀레이션: Face/Solid → Triangle Mesh
- [x] STL 내보내기 (ASCII + Binary)
- [x] OBJ 내보내기
- [x] E2E 통합 테스트 3개

## Phase 4: Core Hardening ✅

안전성, 인체공학, 성능 강화.

- [x] `cadkernel-core` 독립 크레이트 (KernelError/KernelResult)
- [x] 전체 공개 API panic 경로 제거 (assert!/expect → KernelResult)
- [x] `Send + Sync` bounds (`Arc<dyn Curve/Surface + Send + Sync>`)
- [x] Math 표준 trait: Default, Display, From, AddAssign, Sum, f64*Vec, Point±Vec
- [x] EntityStore::len() O(1) 최적화
- [x] IntersectionEllipse 이름 충돌 해소
- [x] 기하 구조체 PartialEq + Copy 추가
- [x] NURBS 안전성 가드 (empty, division-by-zero)
- [x] Wire entity + Naming 시스템 연동
- [x] Topology validation + 5개 traversal helper
- [x] Prelude 모듈 (전 크레이트)

## Phase 5: Mass Properties + Sweep ✅

측정 모듈 + 경로 기반 모델링.

- [x] `MassProperties`: 부피, 표면적, 무게중심 (발산 정리 기반)
- [x] `compute_mass_properties` / `solid_mass_properties`
- [x] Sweep 연산: 프로파일 × 경로 → 솔리드 (회전 최소화 프레임)
- [x] Sweep auto-tagging (Persistent Naming)

## Phase 6: Loft + Pattern ✅

모델링 확장: 복수 단면 보간 + 반복 복사 패턴.

- [x] Loft 연산: N개 단면 프로파일 보간 → 솔리드 (cap_start/cap_end 제어)
- [x] Linear Pattern: 방향 + 간격 + 횟수 → 반복 복사 솔리드
- [x] Circular Pattern: 축 + 각도 + 횟수 → 회전 반복 복사
- [x] `copy_solid_with_transform`: 솔리드 Deep-copy 인프라
- [x] 전체 Pattern에 Persistent Naming 자동 태깅

## Phase 7: Chamfer + I/O Import ✅

첫 번째 토폴로지 수정 연산 + 파일 읽기 기능.

- [x] Chamfer 연산: 모서리 면취 (인접 면 자동 탐색, 토폴로지 재구축)
- [x] STL Import: ASCII + Binary 자동 감지, tolerance 기반 정점 중복 제거
- [x] OBJ Import: v/vt/vn 형식 파싱, N-gon 팬 삼각화
- [x] STL/OBJ 양방향 round-trip 테스트

---

## Phase 8: Modeling Enhancements (Mirror + Shell + Scale) ✅

모델링 확장: 평면 반사, 박벽 쉘, 비균일 스케일.

- [x] Mirror 연산: `mirror_solid(model, solid, plane_point, plane_normal)` — 평면 반사 복사
- [x] Shell 연산: `shell_solid(model, solid, face_to_remove, thickness)` — 박벽 중공 솔리드
- [x] Scale 연산: `scale_solid(model, solid, center, sx, sy, sz)` — 비균일 스케일 복사
- [x] `copy_solid_with_transform` 공유 유틸리티를 pattern.rs에서 분리

## Phase 9: Math & Geometry Enhancements ✅

수학/기하 라이브러리 대폭 확장.

- [x] `cadkernel-math`: 유틸리티 함수 11개 (`distance_point_line`, `distance`, `angle_between`, `angle_between_2d`, `project_point_on_plane`, `project_point_on_line`, `lerp_point`, `lerp_point_2d`, `triangle_area`, `polygon_area_2d`, `is_ccw`)
- [x] `cadkernel-geometry`: Plane 향상 6개 메서드 (`from_three_points`, `signed_distance`, `distance`, `project_point`, `is_above`, `contains_point`)
- [x] `cadkernel-math`: BoundingBox 향상 6개 메서드 (`overlaps`, `expand`, `volume`, `surface_area`, `longest_axis`, `size`)

## Phase 10: Quality & Testing ✅

E2E 테스트 대폭 확충 및 B-Rep 검증 강화.

- [x] 10개 E2E 통합 테스트 (box→STL roundtrip, sweep→OBJ, loft→mass props, pattern copies, chamfer→export, mirror topology, shell faces, cylinder pipeline, sphere roundtrip, chamfer→mirror pipeline)
- [x] B-Rep 검증 강화: 댕글링 참조 감지, 방향 일관성 체크
- [x] 새 검증 API: `validate_manifold()`, `validate_detailed()`, `ValidationIssue`, `ValidationSeverity`

## Phase 11: I/O Format Expansion ✅

SVG 2D 내보내기 및 JSON 직렬화 지원.

- [x] SVG 2D Export: `SvgDocument`, `SvgElement` (Line, Polyline, Circle, Arc, Polygon), `SvgStyle`, `profile_to_svg`
- [x] JSON 직렬화: `model_to_json`, `model_from_json`, `write_json`, `read_json`, `export_json`, `import_json`
- [x] 모든 토폴로지/수학 타입에 serde `Serialize`/`Deserialize` derive

---

## Phase 12: Rustdoc Documentation ✅

Rustdoc 문서화 완료.

- [x] 모든 `pub` 항목에 `///` 문서 주석
- [x] 각 크레이트 `lib.rs`에 `//!` 문서
- [x] 주요 API에 `# Examples` 블록 추가
- [x] 문서 내 코드 블록 컴파일 검증

## Phase 13: High Priority Features ✅

고우선순위 모델링 기능.

- [x] Fillet 연산: `fillet_edge(model, solid, v1, v2, radius, segments)` — 호 근사 기반 모서리 라운딩
- [x] 솔리드 분할: `split_solid(model, solid, plane_point, plane_normal)` — 절단 평면으로 솔리드 이분할 → `SplitResult { above, below }`
- [x] 점-솔리드 포함 판정: `point_in_solid(model, solid, point)` → `Containment` (Inside/Outside/OnBoundary)

## Phase 14: Geometry & Manufacturing ✅

기하 오프셋, 제조 지원 연산, 적응형 테셀레이션.

- [x] 2D 커브 오프셋: `offset_polyline_2d`, `offset_polygon_2d` — CNC/스케치용 평행 오프셋
- [x] 구배 각도: `draft_faces(model, solid, pull_dir, neutral_plane, angle, faces)` — 금형 테이퍼
- [x] 적응형 테셀레이션: `TessellationOptions` (chord_tolerance, angle_tolerance, min_segments, max_depth)
- [x] `adaptive_tessellate_curve` / `adaptive_tessellate_surface` 함수
- [x] `TessellateCurve` / `TessellateSurface` 확장 트레이트

## Phase 15: Infrastructure ✅

Undo/Redo, 속성 시스템, 최근접점 쿼리.

- [x] `ModelHistory`: 스냅샷 기반 undo/redo (record, undo, redo, can_undo, can_redo, max_history)
- [x] 속성 시스템: `Color` (RGBA + 6개 상수), `Material` (프리셋: Steel, Aluminum, ABS, Wood), `PropertyValue`, `PropertyStore`
- [x] 최근접점 쿼리: `closest_point_on_solid(model, solid, query)` → `ClosestPointResult { point, distance, face }`

## Phase 16: Industry Formats ✅

STEP 및 IGES 산업 표준 파일 형식 지원.

- [x] STEP I/O: `StepWriter` (CARTESIAN_POINT, DIRECTION, AXIS2_PLACEMENT_3D, 메시 내보내기)
- [x] STEP 읽기: `read_step_points`, `parse_step_entities`, `export_step_mesh`
- [x] IGES I/O: `IgesWriter` (Point type 116, Line type 110, CircularArc type 100)
- [x] IGES 읽기: `read_iges_points`, `read_iges_lines`

## Phase 17: Quality & Advanced ✅

벤치마크, NURBS 고급 기능, 스레드 안전성.

- [x] 벤치마크 스위트: 14개 criterion 벤치마크 (make_box, make_cylinder×2, make_sphere×2, tessellate×2, boolean_union, boolean_difference, extrude, revolve, stl_write_binary, stl_write_ascii, mass_properties)
- [x] NURBS 고급: `insert_knot` (Boehm 알고리즘) — 형상 보존 노트 삽입
- [x] NURBS 고급: `elevate_degree` — 형상 보존 차수 승격
- [x] 스레드 안전성: 전체 크레이트 (math, core, geometry, topology, io)에 컴파일 타임 `Send + Sync` 어서션

---

## 테스트 분포

| 크레이트 | 단위 테스트 | Doc 테스트 |
|----------|:---------:|:----------:|
| cadkernel (E2E) | 4 | 1 |
| cadkernel-core | 1 | 1 |
| cadkernel-math | 44 | 1 |
| cadkernel-geometry | 56 | — |
| cadkernel-topology | 29 | 1 |
| cadkernel-modeling | 28 | — |
| cadkernel-sketch | 10 | — |
| cadkernel-io | 34 | — |
| cadkernel-viewer | — | — |
| cadkernel-python | — | — |
| **합계** | **206** | **3** |

---

## Application Phase 1: Native GUI Application ✅

네이티브 데스크톱 GUI 애플리케이션 (egui + wgpu).

- [x] wgpu 렌더링 파이프라인: Solid, Wireframe, Transparent, Flat Lines 디스플레이 모드
- [x] 3개 렌더 파이프라인 (솔리드, 와이어프레임, 투명) + 동적 유니폼 버퍼 오프셋
- [x] 궤도 카메라 시스템: yaw/pitch/distance, 360° 회전, 화면 정렬 팬, 줌
- [x] 투시/직교 투영 전환
- [x] 표준 뷰 프리셋: Front, Back, Right, Left, Top, Bottom, Isometric
- [x] 마우스 내비게이션 프리셋: FreeCAD Gesture (기본), Blender, SolidWorks, Inventor, OpenCascade
- [x] 내비게이션 스타일/감도 커스터마이징 설정 다이얼로그
- [x] 동적 그리드 오버레이: 줌 레벨 따라 1-2-5 시퀀스 자동 스케일링, minor/major 구분
- [x] XYZ 원점 축 (R/G/B) + 미니 축 표시기 (egui 오버레이)
- [x] 다크 테마 그라디언트 배경
- [x] egui UI: 메뉴바, 모델 트리, 속성 인스펙터, 상태 바
- [x] 형상 생성 다이얼로그 (Box, Cylinder, Sphere)
- [x] 파일 열기/저장/내보내기 (rfd 네이티브 다이얼로그)
- [x] 비동기 백그라운드 파일 로딩
- [x] FreeCAD 스타일 키보드 단축키
- [x] glTF 2.0 내보내기
- [x] rayon 멀티스레드 I/O (STL/OBJ/glTF 병렬 파싱)
- [x] Python 바인딩 (PyO3)

## Application Phase 2: ViewCube & 카메라 고도화 🚧

3D 뷰포트 내비게이션 UX 강화.

- [x] ViewCube: 절두 큐브 (chamfer 모서리, 6 팔각형 면 + 8 삼각형 코너 + 12 공유 엣지)
- [x] ViewCube: 방향 조명 (top-right-front 광원, ambient+diffuse 셰이딩)
- [x] ViewCube: 드롭 섀도, 오비트 링 + 나침반 레이블 (F/R/B/L)
- [x] ViewCube: 면/엣지/코너 호버 감지 및 클릭 뷰 스냅 (6+12+8 = 26 뷰 방향)
- [x] ViewCube: 화면 기준 스크린 스페이스 화살표 버튼 (▲▼◀▶, Rodrigues 회전)
- [x] ViewCube: CW/CCW 인플레인 롤 버튼 (↺↻, 시점 축 기준 회전)
- [x] ViewCube: 사이드 버튼 (Home, 투영 전환 P/O, FitAll)
- [x] 카메라 롤(roll) 시스템: 시점 축 기준 인플레인 회전, 뷰 스냅 시 자동 리셋
- [x] 카메라 애니메이션 시스템: smooth-step 이징 (3t²−2t³), 최단 경로 yaw 보간
- [x] 뷰 전환 애니메이션 설정: 활성화/비활성화 토글, 지속 시간 0.1~1.0초 슬라이더
- [x] 45도 단위 오비트 스텝 (화살표/회전 버튼)
- [x] 미니 축 인디케이터: 음방향 페이드 라인, roll 반영 렌더링
- [x] `rodrigues()` 벡터 회전 유틸리티
