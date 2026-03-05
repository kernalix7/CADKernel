# 변경 이력

[English](CHANGELOG.md) | **한국어**

이 프로젝트의 주요 변경 사항은 이 문서에 기록됩니다.

형식은 [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)를 기반으로 하며,
버전 정책은 [Semantic Versioning](https://semver.org/lang/ko/)을 지향합니다.

## [Unreleased]

### 추가됨

#### Phase 1: Foundation
- Cargo workspace 구조 초기화 (7 크레이트 모노레포)
- `cadkernel-math`: Vec2/3/4, Point2/3, Mat3/4, Transform, Quaternion, Ray3, BoundingBox, Tolerance
- `cadkernel-geometry`: Curve/Surface 트레이트 + Line, Arc, Circle, Ellipse, NURBS 구현
- `cadkernel-topology`: Half-edge B-Rep 자료구조, EntityStore, Handle<T>
- 버전 배너 유틸리티 (`version_banner`) 및 단위 테스트
- GitHub Actions CI 파이프라인 (`ci.yml`: fmt, clippy, test)
- Apache 2.0 `LICENSE` 파일
- 이중언어 문서 세트 (README, SECURITY, CODE_OF_CONDUCT, CONTRIBUTING, CHANGELOG)
- Rust 개발용 `.gitignore`

#### Phase 2: Persistent Naming + Boolean
- `cadkernel-topology`: Persistent Naming 시스템 (Tag, NameMap, ShapeHistory, OperationId)
- `cadkernel-topology`: Geometry-Topology 바인딩 (Edge.curve, Face.surface) — feature flag 기반
- `cadkernel-geometry`: Surface-Surface Intersection (Plane-Plane, Plane-Sphere, Plane-Cylinder, Sphere-Sphere)
- `cadkernel-geometry`: Line-Surface Intersection (Line vs Plane, Sphere, Cylinder)
- `cadkernel-modeling`: Boolean 연산 (Union, Subtract, Intersect) — Broad Phase + Classify + Evaluate

#### Phase 3: Parametric + Sketch + I/O
- `cadkernel-sketch`: 2D 파라메트릭 스케치 시스템 (14개 제약 조건)
- `cadkernel-sketch`: Newton-Raphson 솔버 (Armijo 백트래킹)
- `cadkernel-modeling`: Feature Operations (Extrude, Revolve) — auto-tagging
- `cadkernel-modeling`: Primitive Builders (Box, Cylinder, Sphere)
- `cadkernel-io`: 테셀레이션 (Face/Solid → Triangle Mesh)
- `cadkernel-io`: STL 내보내기 (ASCII + Binary), OBJ 내보내기
- E2E 통합 테스트 (Sketch → Extrude → STL, Sketch → Revolve → OBJ, Persistent Naming)

#### Phase 4: Core Hardening
- `cadkernel-core`: KernelError/KernelResult 공유 타입 독립 크레이트 분리
- 전체 공개 API의 `assert!`/`expect()` → `KernelResult` 변환 (panic 경로 제거)
- `Arc<dyn Curve + Send + Sync>` / `Arc<dyn Surface + Send + Sync>` — 스레드 안전성
- Math 타입 표준 trait: Default, Display, From, AddAssign/SubAssign/MulAssign, Sum
- 점-벡터 완전 연산자: `Point - Vec`, `f64 * Vec`, `From<[f64;N]>`, `From<Vec3> for Point3`
- `EntityStore::len()` O(n) → O(1) 최적화
- `IntersectionEllipse`로 이름 충돌 해소 (커브 Ellipse와 분리)
- 기하 구조체에 `PartialEq` + `Copy` 추가
- NURBS: empty control_points 가드, tangent division-by-zero 가드
- `WireData`: 독립 반변 체인 + Persistent Naming 연동
- Topology: validation, traversal helpers (5개), transform
- 전체 크레이트 Prelude 모듈 추가
- Developer Wiki 가이드 문서 추가 (한국어/영문)

#### Phase 5: Mass Properties + Sweep
- `cadkernel-modeling`: `MassProperties` 구조체 (부피, 표면적, 무게중심)
- `cadkernel-modeling`: `compute_mass_properties()` — 발산 정리 기반 메시 체적/면적 계산
- `cadkernel-modeling`: `solid_mass_properties()` — B-Rep 솔리드 편의 함수
- `cadkernel-modeling`: Sweep 연산 (프로파일 × 경로 → 솔리드)
- Sweep: 회전 최소화 프레임(RMF) 전파, 자동 Persistent Naming
- GitHub Wiki 문서 체계 재구성 (13개 페이지: Architecture, 크레이트별 가이드, Cookbook 등)

#### Phase 6: Loft + Pattern
- `cadkernel-modeling`: Loft 연산 (N개 단면 프로파일 보간 → 솔리드, cap_start/cap_end 제어)
- `cadkernel-modeling`: Linear Pattern (방향 + 간격 + 횟수 → 반복 복사)
- `cadkernel-modeling`: Circular Pattern (축 + 각도 + 횟수 → 회전 복사)
- 솔리드 Deep-copy 인프라 (`copy_solid_with_transform`)
- 전체 Pattern에 Persistent Naming 태그 자동 부여

#### Phase 7: Chamfer + I/O Import
- `cadkernel-modeling`: Chamfer 연산 (모서리 면취 — 인접 면 탐색 + 토폴로지 재구축)
- `cadkernel-io`: STL Import (ASCII + Binary 자동 감지, 정점 중복 제거)
- `cadkernel-io`: OBJ Import (v/vt/vn 형식 파싱, N-gon 팬 삼각화)
- STL/OBJ 양방향 round-trip 지원

#### Phase 8: Modeling Enhancements (Mirror + Shell + Scale)
- `cadkernel-modeling`: Mirror 연산 (평면 반사 복사)
- `cadkernel-modeling`: Shell 연산 (박벽 중공 솔리드)
- `cadkernel-modeling`: 비균일 Scale 연산
- `copy_solid_with_transform` 공유 유틸리티를 pattern.rs에서 분리

#### Phase 9: Math & Geometry Enhancements
- `cadkernel-math`: 유틸리티 함수 11개 (거리, 각도, 투영, 보간, 면적)
- `cadkernel-geometry`: Plane — `from_three_points`, `signed_distance`, `project_point` 등
- `cadkernel-math`: BoundingBox — `overlaps`, `expand`, `volume`, `surface_area`, `longest_axis`, `size`

#### Phase 10: Quality & Testing
- E2E 통합 테스트 10개 (전체 파이프라인: 모델 → 내보내기 → 가져오기)
- B-Rep 검증: 댕글링 참조 감지, 방향 일관성 체크
- 새 API: `validate_manifold()`, `validate_detailed()`, `ValidationIssue`, `ValidationSeverity`

#### Phase 11: I/O Format Expansion
- `cadkernel-io`: SVG 2D 내보내기 (`SvgDocument`, 5가지 요소 타입, auto-fit viewBox)
- `cadkernel-io`: JSON 직렬화 (BRepModel ↔ JSON 라운드트립, 파일 I/O)
- 모든 토폴로지/수학 타입에 serde `Serialize`/`Deserialize` derive

#### Phase 12: Rustdoc Documentation
- 전체 크레이트 수준 문서 (`//!`) 추가
- 모든 `pub` 항목에 API 문서 주석 추가

#### Phase 13: 고우선순위 기능
- `cadkernel-modeling`: Fillet 연산 (`fillet_edge`) — 호 근사 기반 모서리 라운딩 (반경, 세그먼트 수 설정 가능)
- `cadkernel-modeling`: 솔리드 분할 연산 (`split_solid`) — 절단 평면을 이용한 솔리드 이분할
- `cadkernel-modeling`: 점-솔리드 포함 판정 (`point_in_solid`) — 레이캐스팅 기반 `Inside`/`Outside`/`OnBoundary` 반환

#### Phase 14: 기하 & 제조
- `cadkernel-geometry`: 2D 커브 오프셋 (`offset_polyline_2d`, `offset_polygon_2d`) — CNC/스케치용 평행 오프셋
- `cadkernel-modeling`: 구배 각도 연산 (`draft_faces`) — 금형 테이퍼 (풀 방향, 중립면 설정 가능)
- `cadkernel-geometry`: 적응형 테셀레이션 (`TessellationOptions`, `adaptive_tessellate_curve`, `adaptive_tessellate_surface`, `TessMesh`) — 현오차 및 각도 기반 세분화
- `cadkernel-geometry`: `TessellateCurve` / `TessellateSurface` 확장 트레이트

#### Phase 15: 인프라
- `cadkernel-topology`: Undo/Redo 시스템 (`ModelHistory`) — 스냅샷 기반 실행취소/재실행 (최대 깊이 설정 가능)
- `cadkernel-topology`: 속성 시스템 (`Color`, `Material`, `PropertyValue`, `PropertyStore`) — 엔티티 메타데이터 및 재질 프리셋 (Steel, Aluminum, ABS, Wood)
- `cadkernel-modeling`: 최근접점 쿼리 (`closest_point_on_solid`) — Voronoi 영역 삼각형 투영, `ClosestPointResult` (점, 거리, 면) 반환

#### Phase 16: 산업 형식
- `cadkernel-io`: STEP I/O (`StepWriter`, `read_step_points`, `parse_step_entities`, `export_step_mesh`) — ISO 10303-21 부분 지원 (AP214)
- `cadkernel-io`: IGES I/O (`IgesWriter`, `read_iges_points`, `read_iges_lines`) — IGES 5.3 고정폭 80열 포맷 기본 기하 교환

#### Phase 17: 품질 & 고급 기능
- `cadkernel-modeling`: 벤치마크 스위트 — 9개 criterion 벤치마크 (프리미티브, 불리언, 돌출, 스윕, 패턴, STL 쓰기, 질량 특성)
- `cadkernel-geometry`: NURBS 고급 기능 — 노트 삽입 (Boehm 알고리즘), 차수 승격 — 형상 보존 정밀화
- 전체 크레이트에 컴파일 타임 `Send + Sync` 어서션 (math, core, geometry, topology, io)

#### Application Phase 1: 네이티브 GUI 애플리케이션
- `cadkernel-viewer`: 네이티브 데스크톱 GUI 애플리케이션 (egui 0.31 + wgpu 24.x + winit 0.30)
- `cadkernel-viewer`: wgpu 렌더링 파이프라인 — 4가지 디스플레이 모드 (Solid, Wireframe, Transparent, Flat Lines)
- `cadkernel-viewer`: 3개 렌더 파이프라인 (솔리드, 와이어프레임/라인, 투명) + 동적 유니폼 버퍼 오프셋
- `cadkernel-viewer`: 궤도 카메라 시스템 (yaw/pitch/distance, 360° 회전, 화면 정렬 팬, 스크롤 줌)
- `cadkernel-viewer`: 투시/직교 투영 전환
- `cadkernel-viewer`: 표준 뷰 프리셋 (Front, Back, Right, Left, Top, Bottom, Isometric)
- `cadkernel-viewer`: 마우스 내비게이션 프리셋 설정 (FreeCAD Gesture, Blender, SolidWorks, Inventor, OpenCascade)
- `cadkernel-viewer`: 내비게이션 스타일 및 감도 커스터마이징 설정 다이얼로그
- `cadkernel-viewer`: 동적 그리드 오버레이 (줌 레벨에 따른 1-2-5 간격 자동 스케일링, minor/major 라인 구분)
- `cadkernel-viewer`: XYZ 원점 축 렌더링 (R/G/B 컬러)
- `cadkernel-viewer`: 다크 테마 그라디언트 배경
- `cadkernel-viewer`: 미니 축 표시기 (egui 오버레이, 좌하단)
- `cadkernel-viewer`: egui UI 패널 (메뉴바, 모델 트리, 속성 인스펙터, 상태 바)
- `cadkernel-viewer`: 형상 생성 다이얼로그 (Box, Cylinder, Sphere 매개변수 입력)
- `cadkernel-viewer`: 파일 열기/저장/내보내기 다이얼로그 (rfd 네이티브 파일 다이얼로그)
- `cadkernel-viewer`: 비동기 백그라운드 파일 로딩 (UI 멈춤 방지)
- `cadkernel-viewer`: FreeCAD 스타일 키보드 단축키 (1/3/7=뷰, Ctrl+1/3/7=역방향 뷰, 5=투영, D=디스플레이, V=맞춤, G=그리드)
- `cadkernel-io`: glTF 2.0 내보내기 (임베디드 base64, 정점별 법선, min/max 바운드)
- `cadkernel-io`: rayon 기반 멀티스레드 STL/OBJ 파싱 (O(N²) 선형 검색 → O(N) HashMap 정점 중복 제거)
- `cadkernel-io`: 멀티스레드 glTF 내보내기, 테셀레이션, 바운딩 박스 계산
- `cadkernel-python`: PyO3 기반 Python 바인딩 (BRepModel, 프리미티브, I/O, 질량 특성)

#### Application Phase 2: ViewCube & 카메라 고도화
- `cadkernel-viewer`: ViewCube — 절두 큐브 (chamfer 모서리, 6 팔각형 면 + 8 삼각형 코너 + 12 공유 엣지)
- `cadkernel-viewer`: ViewCube — 방향 조명 (top-right-front 광원, ambient+diffuse 셰이딩)
- `cadkernel-viewer`: ViewCube — 드롭 섀도, 오비트 링 + 나침반 레이블 (F/R/B/L)
- `cadkernel-viewer`: ViewCube — 면/엣지/코너 호버 감지 및 클릭 뷰 스냅 (6+12+8 = 26 뷰 방향)
- `cadkernel-viewer`: ViewCube — 화면 기준 스크린 스페이스 화살표 버튼 (▲▼◀▶, Rodrigues 회전으로 뷰 방향 계산)
- `cadkernel-viewer`: ViewCube — CW/CCW 인플레인 롤 버튼 (↺↻, 화면 기준 시계/반시계 회전)
- `cadkernel-viewer`: ViewCube — 사이드 버튼 (Home, 투영 전환 P/O, FitAll)
- `cadkernel-viewer`: 카메라 롤(roll) 시스템 — 시점 축 기준 인플레인 회전, 뷰 스냅 시 자동 리셋
- `cadkernel-viewer`: 카메라 애니메이션 시스템 — smooth-step 이징 (3t²−2t³), 최단 경로 yaw 보간
- `cadkernel-viewer`: 뷰 전환 애니메이션 설정 (활성화/비활성화, 지속 시간 0.1~1.0초 조절)
- `cadkernel-viewer`: 45도 단위 오비트 스텝 (화살표/회전 버튼)
- `cadkernel-viewer`: 미니 축 인디케이터 — 음방향 페이드 라인 추가, roll 반영
- `cadkernel-viewer`: `rodrigues()` 벡터 회전 유틸리티 (render.rs)
- `cadkernel-viewer`: ViewCube 면 레이블 회전 — TextShape angle로 큐브에 각인된 텍스트 구현
- `cadkernel-viewer`: 뷰 스냅 시 roll 90° 스냅 — 가장 가까운 90° 배수로 자동 스냅
- `cadkernel-viewer`: ViewCube 드롭다운 메뉴 (☰) — Orthographic/Perspective, Isometric, Fit All

#### Application Phase 3: 렌더링 & UI 개편
- `cadkernel-viewer`: 8가지 디스플레이 모드 (As Is, Points, Wireframe, Hidden Line, No Shading, Shading, Flat Lines, Transparent) — FreeCAD 렌더링 옵션 매칭
- `cadkernel-viewer`: CW/CCW 회전 아이콘 방향 수정 (↺=반시계, ↻=시계 — positive roll 규약 일치)
- `cadkernel-viewer`: FreeCAD 스타일 ViewCube 개선 — 반투명 면, XYZ 축 인디케이터, 엣지 선택, 앞면만 호버
- `cadkernel-viewer`: 화면 기준 Rodrigues 오비트 — 면 상대 회전 + yaw/pitch/roll 역추출 (직접 yaw/pitch 변경 방식 대체)
- `cadkernel-viewer`: 애니메이션 타겟 스냅 — 진행 중 애니메이션에서 연속 화살표 누름이 올바르게 체이닝
- `cadkernel-viewer`: Macro 메뉴 (placeholder: Console, Record, Execute)
- `cadkernel-viewer`: FreeCAD 스타일 설정 다이얼로그 — 3D View (축, FPS, 투영), Navigation (ViewCube, 궤도 스타일, 감도, 애니메이션), Lighting (강도, 방향 XYZ)
- `cadkernel-viewer`: NavConfig 10개 신규 설정 추가 (show_view_cube, cube_size, cube_opacity, orbit_steps, snap_to_nearest, show_axes_indicator, show_fps, enable_lighting, light_intensity, light_dir)
- `cadkernel-viewer`: Blinn-Phong 셰이딩 — 경면 반사(specular) 하이라이트 (강도 + 광택도 설정 가능) + 색상 클램핑으로 사실적 표면 렌더링
- `cadkernel-viewer`: 카메라 헤드라이트 — 광원이 카메라 우상단 오프셋 위치를 추적하여 오비트 시 실시간 반사각 갱신
- `cadkernel-viewer`: GPU 어댑터 폴백 — HighPerformance → LowPower → 소프트웨어(llvmpipe/swiftshader) 순차 시도 + 백엔드 로깅
- `cadkernel-viewer`: 마우스 오비트 방향 수정 — 우측 드래그 시 우측으로 회전 (yaw/pitch 부호 반전)
- `cadkernel-viewer`: ViewCube 면 라벨 수정 — FACE_TEXT_RIGHT를 실제 `cross3(f, up)` screen_right에 맞춤
- `cadkernel-viewer`: 크리즈 각도 기반 오토스무스 노말 (60° 임계값) — 평면의 면 경계 아티팩트 제거 + 날카로운 엣지 보존 (Blender/FreeCAD 방식)
- `cadkernel-viewer`: ViewCube 엣지 챔퍼 쿼드 — 12개 엣지 베벨을 라이팅 적용된 채워진 사각형으로 렌더링 (라인 세그먼트 대체), 면/코너와 함께 깊이 정렬
- `cadkernel-viewer`: 스무스 그룹 노말 (BFS) — 각 정점에서 크리즈 각도(60°) 내 면을 전이적으로 그룹화, 면적 가중 노말 누적 (비정규화 외적 합산, 크기 ∝ 삼각형 면적). 불균일한 메시 밀도에서의 불연속 제거 + 날카로운 모서리 보존
- `cadkernel-viewer`: ViewCube 단일 메시 렌더링 — 비-호버 폴리곤을 하나의 `epaint::Mesh`로 합쳐 렌더링 (팬 삼각분할, 내부 엣지에 안티앨리어싱 페더링 없음). 인접 면/엣지/코너 사이의 이음새 선 제거. 호버된 폴리곤은 별도로 스트로크 하이라이트와 함께 렌더링
- `cadkernel-viewer`: ViewCube 불투명 채우기 — XYZ 축 인디케이터를 큐브 폴리곤 위에 렌더링 (반투명 겹침의 더블 블렌딩 아티팩트 제거)
- `cadkernel-viewer`: Face normal 항상 정점 위치로 계산 — BFS 그루핑에 저장된 STL 노말 대신 기하학적 노말 사용 (일관되지 않은/반전된 파일 노말로 인한 이음새 제거)
- `cadkernel-viewer`: 4x MSAA (Multi-Sample Anti-Aliasing) — 스무스 서피스의 삼각형 경계 Mach band 아티팩트 제거. MSAA 컬러/깊이 텍스처 (`sample_count=4`), 모든 렌더 파이프라인 업데이트, 씬 패스에서 서피스 텍스처로 리졸브
- `cadkernel-io`: 테셀레이션 정점 공유 — `tessellate_solid`에서 bit-exact 위치 매칭 (`f64::to_bits` HashMap)으로 정점 중복 제거, 면 경계를 넘는 스무스 노말 계산 가능. 곡면의 삼각형 경계선 근본 수정
- `cadkernel-io`: STL 정점 중복 제거 정밀도 수정 — quantize를 1e8에서 1e4(0.1mm 허용 오차)로 변경, float32 정밀도의 동일 정점을 올바르게 병합하여 스무스 노말 생성
- `cadkernel-viewer`: 방향 인식 롤 스냅 — 두 90° 배수 사이의 45° 중간점에서 이전 롤 위치 방향으로 스냅 (예: 0°→45°는 0°으로, 90°→45°는 90°으로 복귀). `RollDelta`/`ScreenOrbit` 액션 전 `prev_roll` 추적
- `cadkernel-viewer`: Top/Bottom 뷰 yaw 보존 — Top/Bottom 클릭 시 현재 yaw 유지 (pitch만 변경), 수직에 가까운 뷰에서 불필요한 인플레인 회전 방지
- `cadkernel-viewer`: 롤 각도 정규화 — `wrap_angle()` 유틸리티로 각도를 (−π, π] 범위로 정규화. `snap_roll_90`은 입력값 정규화 후 처리. `RollDelta`는 버튼 누를 때마다 `camera.roll` 정규화하여 무한 누적 방지 (CW 8회 = 360° = 0°)
- `cadkernel-viewer`: ScreenOrbit `prev_roll` 타이밍 수정 — 애니메이션 타겟 스냅 이후에 `prev_roll` 저장 (이전이 아님), 보간 중간값 대신 정확한 90° 타겟값 보장
- `cadkernel-viewer`: 프리미티브 기본 테셀레이션 증가 — Cylinder 32→64 세그먼트, Sphere 32×16→64×32 세그먼트로 곡면 품질 향상
- `cadkernel-io`: 네이티브 `.cadk` 프로젝트 포맷 — 사람이 읽기 쉬운 JSON + 포맷 헤더 (`CADKernel` + semver), 기존 BRepModel JSON과 하위 호환
