# 변경 이력

[English](../CHANGELOG.md) | **한국어**

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

#### Application Phase 4: ViewCube 개선 + FPS
- `cadkernel-viewer`: ViewCube 면 옥타곤 인셋 — 인접면 방향 정점을 EDGE_BEVEL만큼 안쪽으로 이동하여 베벨 스트립 노출
- `cadkernel-viewer`: ViewCube 코너 헥사곤 — 3정점 코너 삼각형을 6정점 헥사곤으로 확장하여 인셋된 면 엣지와 매칭
- `cadkernel-viewer`: FPS 카운터 — 0.5초 롤링 평균 FPS 표시 (상태바, Settings > Show FPS로 토글)

#### Application Phase 5: 전체 이슈 수정 + 워크벤치 툴바
- `cadkernel-viewer`: FreeCAD 스타일 워크벤치 툴바 — Workbench 열거형 (Part Design, Sketcher, Mesh, Assembly), 공통 액션 툴바 (New/Open/Save/Undo/Redo), 워크벤치 탭바, 컨텍스트 도구 툴바
- `cadkernel-viewer`: NavConfig 설정 실제 적용 — `cube_size`로 ViewCube 크기, `cube_opacity`로 채우기 투명도, `orbit_steps`로 화살표 버튼 스텝 각도 조절
- `cadkernel-viewer`: 심플 뷰어 오비트 방향 수정 — dx/dy 부호 반전으로 자연스러운 오비트
- `cadkernel-viewer`: ScreenOrbit asin NaN 방지 — asin 입력을 [-1,1]로 클램프

#### Application Phase 7: FreeCAD 워크벤치 시스템 + 신규 프리미티브
- `cadkernel-modeling`: `make_cone()` 프리미티브 — 뾰족 원뿔 (꼭짓점) 및 절두체 (잘린 원뿔), base_radius/top_radius/height/segments 파라미터. EdgeCache 중복 제거, 완전 B-Rep 토폴로지 + 테스트
- `cadkernel-modeling`: `make_torus()` 프리미티브 — 도넛 형상 솔리드, major_radius/minor_radius/major·minor segments 파라미터. Quad 메시 토폴로지 + EdgeCache 중복 제거
- `cadkernel-viewer`: 워크벤치 시스템 확장 — 6개 워크벤치: Part (신규), Part Design, Sketcher, Mesh, TechDraw (신규), Assembly
- `cadkernel-viewer`: Part 워크벤치 — 5개 프리미티브 (Box, Cylinder, Sphere, Cone, Torus) + 불리언 연산 + Mirror/Scale 플레이스홀더
- `cadkernel-viewer`: Part Design 워크벤치 — 피처 기반 도구로 재구성 (Pad, Pocket, Revolve, Fillet, Chamfer, Draft, Mirror, Pattern)
- `cadkernel-viewer`: TechDraw 워크벤치 — 플레이스홀더 도구 (Front/Top/Right View, Section, Dimension, Export SVG)
- `cadkernel-viewer`: Assembly 워크벤치 — 플레이스홀더 도구 (Insert Component, Fixed, Coincident, Concentric, Distance)
- `cadkernel-viewer`: Sketcher 워크벤치 — Rectangle 도구 플레이스홀더 추가
- `cadkernel-viewer`: Cone 생성 다이얼로그 — base radius, top radius, height 파라미터 (top_radius=0이면 뾰족 원뿔)
- `cadkernel-viewer`: Torus 생성 다이얼로그 — major radius, minor radius 파라미터
- `cadkernel-viewer`: Create 메뉴 — Cone, Torus 항목 추가

#### Application Phase 8: PartDesign 피처 구현
- `cadkernel-modeling`: `mirror_solid()` — `copy_solid_transformed` 활용 평면 반사, 올바른 법선을 위한 와인딩 반전
- `cadkernel-modeling`: `scale_solid()` — 중심점 기준 균일 스케일링, 음수 팩터 시 미러 (와인딩 반전)
- `cadkernel-modeling`: `sweep()` 재작성 — Frenet 프레임 기반 프로파일을 경로 접선에 수직 배치, 바닥/상단 캡 + 측면 쿼드
- `cadkernel-modeling`: `loft()` 구현 — 2개 이상의 동일 점 수 단면 프로파일 사이 보간, 캡 + 측면 쿼드
- `cadkernel-modeling`: `shell_solid()` 구현 — 지정 면 제거 후 나머지 면을 두께만큼 내부 오프셋, 외부/내부 경계를 림 쿼드로 연결
- `cadkernel-modeling`: `linear_pattern()` 구현 — `copy_solid_transformed` 활용 방향 따라 균일 간격 N개 복사
- `cadkernel-modeling`: `circular_pattern()` 구현 — 쿼터니언 회전 활용 축 주위 등각 간격 N개 복사
- `cadkernel-modeling`: `copy_solid_transformed()` 공유 유틸리티 — 임의 점 변환 함수로 솔리드 토폴로지 딥카피, mirror/scale/pattern 공용

#### Application Phase 9: Sketcher 워크벤치 (인터랙티브 2D 스케치 편집)
- `cadkernel-viewer`: SketchMode 시스템 — XY 또는 XZ 작업 평면에서 스케치 편집 모드 진입/종료
- `cadkernel-viewer`: 5개 스케치 도구 — Select, Line (체인 모드), Rectangle (2클릭), Circle (중심+반지름), Arc (중심+반지름, 반원)
- `cadkernel-viewer`: 2D 스케치 오버레이 렌더링 — 작업 평면의 점/선/원/호를 `world_to_screen()` 투영으로 화면에 표시
- `cadkernel-viewer`: 제약조건 시각화 — H/V/Length/Fix/Parallel/Perpendicular/Coincident 인디케이터를 제약 엔티티 근처에 표시
- `cadkernel-viewer`: 스케치 툴바 — 동적 컨텍스트: 유휴 시 "New Sketch (XY/XZ)", 편집 시 도구 버튼 + 제약 버튼 + Close/Cancel
- `cadkernel-viewer`: 화면→평면 레이캐스팅 — `screen_to_sketch_plane()`으로 원근 카메라 마우스 클릭을 작업 평면 교차점으로 역투영
- `cadkernel-viewer`: Sketch → Solid 파이프라인 — Close Sketch 시 제약 해석 (Newton-Raphson) → WorkPlane으로 3D 프로파일 추출 → 평면 법선 방향 돌출
- `cadkernel-viewer`: 스케치 제약 툴바 — Horizontal, Vertical, Length (드래그 값) 마지막 선에 적용
- `cadkernel-viewer`: Escape 키로 스케치 모드 종료, 우클릭으로 대기점 초기화
- `cadkernel-viewer`: 스케치 모드 배너 — 평면, 활성 도구, 점/선 수를 뷰포트에 표시

#### Application Phase 10: TechDraw 워크벤치
- `cadkernel-io`: TechDraw 모듈 — 7개 표준 뷰 직교 투영 (Front/Back/Top/Bottom/Right/Left/Isometric)
- `cadkernel-io`: 은선 제거 (HLR) — 투영 삼각형 대비 5-샘플 무게중심 좌표 깊이 테스트로 엣지 가시성 판별
- `cadkernel-io`: 3면도 레이아웃 (제3각법 투영: 정면, 평면, 우측면)
- `cadkernel-io`: 치수 주석 시스템 (선형, 각도, 반지름)
- `cadkernel-io`: `drawing_to_svg()` — 가시선/은선, 뷰 라벨, 치수가 포함된 완전한 SVG 내보내기
- `cadkernel-io`: SVG Text 요소 + stroke-dasharray 지원 (은선 점선 표현)
- `cadkernel-viewer`: TechDraw 툴바 — Front, Top, Right, Iso, 3-View, Export SVG, Clear
- `cadkernel-viewer`: TechDraw 뷰포트 오버레이 — 투영 엣지 (실선=가시선, 점선=은선), 뷰 라벨, 반투명 배경

#### Application Phase 11: NURBS 커널 강화
- `cadkernel-geometry`: 적응형 커브 테셀레이션 — 현 오차 + 각도 허용차 기반 재귀 이분법
- `cadkernel-geometry`: 적응형 서피스 테셀레이션 — 이선형 중심 vs 실제 중심 현 오차 기반 쿼드 분할
- `cadkernel-geometry`: `TessellationOptions` (chord_tolerance, angle_tolerance, min_segments, max_depth)
- `cadkernel-geometry`: `TessellateCurve` / `TessellateSurface` 블랭킷 확장 트레이트
- `cadkernel-geometry`: 커브-커브 교차 — 재귀 바운딩박스 분할 + Newton-Raphson 정밀화
- `cadkernel-geometry`: 2D 폴리곤/폴리라인 오프셋 — 마이터 조인 오프셋 (클램핑된 마이터 길이)
- `cadkernel-topology`: 지오메트리 바인딩 헬퍼 — `bind_edge_curve()`, `bind_face_surface()`, `face_has_surface()`, `edge_has_curve()`
- `cadkernel-io`: NURBS 인식 테셀레이션 — `tessellate_face`/`tessellate_solid`가 바인딩된 서피스 지오메트리로 적응형 분할 사용, 무한 서피스에 대해 경계 투영으로 파라미터 도메인 결정

#### Phase A: NURBS 커널 완성 (FreeCAD 패리티)
- `cadkernel-geometry`: B-spline 기저함수 모듈 (`bspline_basis.rs`) — `find_span`, `basis_funs`, `ders_basis_funs` (The NURBS Book A2.3, k차 미분)
- `cadkernel-geometry`: NurbsCurve 해석적 미분 — `tangent_at()`, `second_derivative_at()` 유리 몫 법칙 (유한차분 대체)
- `cadkernel-geometry`: NurbsSurface 해석적 편미분 — `du()`, `dv()`, `normal_at()` 동차 미분 (유한차분 대체)
- `cadkernel-geometry`: NurbsCurve 연산 — `reversed()`, `split_at(t)`, `join()` 곡선 조작
- `cadkernel-geometry`: NurbsCurve knot 정제 — `refine_knots()` 배치 knot 삽입 (A5.4)
- `cadkernel-geometry`: NurbsCurve knot 제거 — `remove_knot()` 허용오차 제어 (A5.8)
- `cadkernel-geometry`: NurbsCurve Bezier 분해 — `decompose_to_bezier()` 각 knot span 분리 (A5.6)
- `cadkernel-geometry`: NurbsCurve 보간 — `NurbsCurve::interpolate()` 현 길이 매개변수화 + 삼대각 솔버 (A9.1)
- `cadkernel-geometry`: NurbsCurve 근사 — `NurbsCurve::approximate()` 최소자승법 피팅 (A9.7)
- `cadkernel-geometry`: NurbsSurface knot 연산 — `insert_knot_u/v()`, `refine_knots_u/v()` 행/열 분해 방식
- `cadkernel-geometry`: NurbsSurface 차수 승격 — `elevate_degree_u/v()` 행/열별 곡선 차수 승격
- `cadkernel-geometry`: NurbsSurface 보간 — `NurbsSurface::interpolate()` 2-pass 텐서곱 방식
- `cadkernel-geometry`: 곡선→NURBS 변환 (`to_nurbs.rs`) — `LineSegment`, `Line`, `Circle`, `Arc`, `Ellipse` → 유리 NURBS
- `cadkernel-geometry`: 서피스→NURBS 변환 (`to_nurbs.rs`) — `Plane`, `Cylinder`, `Sphere` → 유리 NURBS 서피스
- `cadkernel-geometry`: NurbsCurve Newton `project_point()` — Bezier 분해 멀티스타트 + 해석적 Newton-Raphson
- `cadkernel-geometry`: NurbsSurface Newton `project_point()` — 20×20 조밀 그리드 + 2D Gauss-Newton 정밀화
- `cadkernel-geometry`: Curve2D 시스템 (`curve2d.rs`) — `Curve2D` 트레이트, `Line2D`, `Circle2D`, `NurbsCurve2D` (UV 공간 매개변수 곡선)
- `cadkernel-geometry`: TrimmedCurve (`trimmed.rs`) — 부분 구간 래퍼, [0,1] 리매핑
- `cadkernel-geometry`: TrimmedSurface (`trimmed.rs`) — UV 트림 루프 + 교차수 점-다각형 판별
- `cadkernel-geometry`: 곡선-서피스 교차 (`curve_surface.rs`) — 세분화 + 이분법 + F(t,u,v) = C(t) - S(u,v) = 0 Newton
- `cadkernel-geometry`: 서피스-서피스 교차 (`surface_surface.rs`) — 상호 투영 시드 탐색 + n1×n2 예측자/보정자 마칭
- `cadkernel-geometry`: NurbsCurve/NurbsSurface `bounding_box()` 오버라이드 — 볼록 껍질 속성 (제어점 AABB)

#### Phase B06-B14: SSI 기반 면 분할 & 정밀 불리언 (2026-03-12)
- `cadkernel-modeling`: SSI 곡선을 따른 면 분할 (`face_split.rs`) — `split_solids_at_intersection()`, `fit_ssi_to_nurbs()`, `fit_ssi_to_pcurve()`
- `cadkernel-geometry`: 트림 루프 유효성 검증 (`trim_validate.rs`) — `validate_trim()`, `ensure_correct_winding()`, `TrimValidation`, `TrimIssue`
- `cadkernel-modeling`: 정확한 불리언 연산 — `boolean_op_exact()` 면 분할 전처리 포함
- `cadkernel-modeling`: 불리언 연산에서 지오메트리 바인딩 보존 (서피스/커브 복사)
- `cadkernel-modeling`: 서피스 바인딩 없는 평면 면의 폴리곤 교차 연산
- `cadkernel-modeling`: shape_analysis `classify_solid`에서 테셀레이트된 실린더가 Prism으로 잘못 분류되던 문제 수정
- 테스트: 총 662개 (기존 609개에서 53개 추가)

#### Phase V1: 스케처 완성 (2026-03-15)
- `cadkernel-sketch`: 3개 신규 엔티티 타입 — `SketchEllipticalArc`, `SketchHyperbolicArc`, `SketchParabolicArc` (원뿔 곡선 호, `entity.rs`)
- `cadkernel-sketch`: 5개 스케치 편집 도구 (`tools.rs`) — `fillet_sketch_corner`, `chamfer_sketch_corner`, `trim_edge`, `split_edge`, `extend_edge`
- `cadkernel-sketch`: 스케치 유효성 검증 모듈 (`validate.rs`) — `validate_sketch`, 7가지 이슈 타입 (열린 프로파일, 중복 점, 길이 0 엣지 등)
- `cadkernel-sketch`: 보조선 기하 — `toggle_construction_mode`, `mark_construction_point`, `mark_construction_line`
- `cadkernel-sketch`: 새 기하 헬퍼 — `add_circle_3pt`, `add_ellipse_3pt`, `add_centered_rectangle`, `add_rounded_rectangle`, `add_arc_slot`

#### Phase V2: PartDesign 완성 (2026-03-15)
- `cadkernel-modeling`: 8개 신규 추가적/감산적 프리미티브 쌍 — `additive_helix`/`subtractive_helix`, `additive_ellipsoid`/`subtractive_ellipsoid`, `additive_prism`/`subtractive_prism`, `additive_wedge`/`subtractive_wedge` (`additive.rs`)
- `cadkernel-modeling`: 2개 신규 감산적 연산 — `subtractive_loft`, `subtractive_pipe` (`additive.rs`)
- `cadkernel-modeling`: 총 추가적/감산적 연산 10개 → 20개로 확장

#### Phase V3: Part 워크벤치 완성 (2026-03-15)
- `cadkernel-modeling`: 결합 연산 (`join.rs`) — `connect_shapes`, `embed_shapes`, `cutout_shapes`
- `cadkernel-modeling`: 컴파운드 연산 (`compound_ops.rs`) — `boolean_fragments`, `slice_to_compound`, `compound_filter`, `explode_compound`
- `cadkernel-modeling`: 형상 연산 (`face_from_wires.rs`) — `face_from_wires`, `points_from_shape`

#### Phase V4: TechDraw 확장 (2026-03-15)
- `cadkernel-io`: 10개 신규 TechDraw 주석 타입 — `ArcLengthDimension`, `ExtentDimension`, `ChamferDimension`, `WeldSymbol` (6개 용접 타입), `BalloonAnnotation`, `Centerline`, `BoltCircleCenterlines`, `CosmeticLine` (4개 스타일), `BreakLine`
- `cadkernel-io`: 모든 신규 주석 타입에 대한 SVG 렌더링

#### Phase V5: 어셈블리 솔버 (2026-03-15)
- `cadkernel-modeling`: DOF 분석 — `analyze_dof()` 구속조건/조인트별 자유도 카운팅
- `cadkernel-modeling`: 반복 구속조건 솔버 — `solve()` 거리 구속조건 지원
- `cadkernel-modeling`: 3개 신규 조인트 타입 — `RackAndPinion`, `ScrewJoint`, `BeltJoint` (총 13개)
- `cadkernel-modeling`: `rotation()` 배치 헬퍼

#### Phase V6: Surface 워크벤치 완성 (2026-03-15)
- `cadkernel-modeling`: `filling()` — N면 경계 패치
- `cadkernel-modeling`: `sections()` — 프로파일을 통한 서피스 스키닝
- `cadkernel-modeling`: `curve_on_mesh()` — 메시 위에 폴리라인 투영

#### Phase V8: 메시 완성 (2026-03-16)
- `cadkernel-io`: `mesh_boolean_intersection()` — AABB 필터링 메시 불리언 교집합
- `cadkernel-io`: `mesh_boolean_difference()` — AABB 필터링 메시 불리언 차집합
- `cadkernel-io`: `regular_solid()` — 5개 정다면체 (정사면체, 정육면체, 정팔면체, 정십이면체, 정이십면체), `RegularSolidType`
- `cadkernel-io`: `face_info()` — 면별 면적, 법선, 무게중심 (`FaceInfo`)
- `cadkernel-io`: `bounding_box_info()` — 메시 AABB (중심, 크기, 대각선) (`MeshBoundingBox`)
- `cadkernel-io`: `curvature_plot()` — 곡률→RGB 색상 매핑 (파랑→빨강)
- `cadkernel-io`: `add_triangle()` — 단일 삼각형 메시 추가
- `cadkernel-io`: `unwrap_mesh()` — 주축 투영 기반 UV 언래핑 (`UnwrapResult`, `UvCoord`)
- `cadkernel-io`: `unwrap_face()` — 단일 면 UV 좌표 계산
- `cadkernel-io`: `remove_components_by_size()` — 삼각형 수 임계값 기반 소규모 컴포넌트 제거
- `cadkernel-io`: `remove_component()` — 인덱스 기반 특정 컴포넌트 제거
- `cadkernel-io`: `trim_mesh()` — 다른 메시의 바운딩 박스로 메시 트리밍
- `cadkernel-io`: `mesh_cross_sections()` — 축 방향 다중 병렬 단면
- `cadkernel-io`: `segment_mesh()` — 법선 기반 메시 세그먼테이션 (영역 성장) (`MeshSegment`)
- `cadkernel-io`: `remesh()` — 적응형 엣지 길이 기반 리파인먼트
- `cadkernel-io`: `evaluate_and_repair()` — 퇴화 삼각형 제거 + 정점 병합 + 법선 조화 (`MeshRepairReport`)
- `cadkernel-io`: `scale_mesh()` — 축별 메시 스케일링
- 신규 내보내기 타입: `FaceInfo`, `MeshBoundingBox`, `MeshRepairReport`, `MeshSegment`, `RegularSolidType`, `UnwrapResult`, `UvCoord`
- 18개 신규 테스트, 총 680개 테스트 (기존 662개)

#### Phase V9: Draft 워크벤치 완성 (2026-03-16)
- `cadkernel-modeling`: `draft_ops.rs`에 37개 Draft 연산 (32개 신규 함수 + 기존 5개)
- `cadkernel-modeling`: 와이어 생성 — `make_fillet_wire`, `make_circle_wire`, `make_arc_wire`, `make_ellipse_wire`, `make_rectangle_wire`, `make_polygon_wire`, `make_bezier_wire`, `make_arc_3pt_wire`, `make_chamfer_wire`, `make_point`
- `cadkernel-modeling`: 와이어 조작 — `offset_wire`, `join_wires`, `split_wire`, `upgrade_wire`, `downgrade_solid`, `wire_to_bspline`, `bspline_to_wire`, `stretch_wire`
- `cadkernel-modeling`: 솔리드 변환 — `move_solid`, `rotate_solid`, `scale_solid_draft`, `mirror_solid_draft`
- `cadkernel-modeling`: 배열 패턴 — `polar_array`, `point_array`
- `cadkernel-modeling`: 주석 — `make_draft_dimension`, `make_label`, `make_dimension_text`
- `cadkernel-modeling`: 스냅 — `snap_to_endpoint`, `snap_to_midpoint`, `snap_to_nearest`
- `cadkernel-modeling`: 쿼리 — `wire_length`, `wire_area`
- 신규 타입: `DraftDimension`, `DraftLabel`, `SnapResult`, `WireResult`, `BSplineWireResult`, `ArrayResult`, `CloneResult`
- 40개 신규 테스트, 총 705개 테스트 (기존 680개)

#### Phase V10: FEM 워크벤치 확장 (2026-03-16)
- `cadkernel-modeling`: 6개 신규 재료 프리셋 — `FemMaterial::titanium()`, `copper()`, `concrete()`, `cast_iron()`, `custom()`, `ThermalMaterial` (steel/aluminum/copper 프리셋)
- `cadkernel-modeling`: 8개 신규 FEM 타입 — `ThermalMaterial`, `ThermalBoundaryCondition` (4개 변형), `ThermalResult`, `BeamSection` (원형, 직사각형), `ModalResult`, `MeshQuality`, `PrincipalStresses`, `StrainResult`, `StressTensor`
- `cadkernel-modeling`: 4개 신규 구조 경계조건 — `Displacement`, `Gravity`, `DistributedLoad`, `Spring`
- `cadkernel-modeling`: 4개 신규 열 경계조건 — `FixedTemperature`, `HeatFlux`, `HeatGeneration`, `Convection`
- `cadkernel-modeling`: `modal_analysis()` — 역멱법(inverse power iteration) 기반 고유진동수 추출
- `cadkernel-modeling`: `thermal_analysis()` — Gauss-Seidel 솔버 기반 정상 상태 열전도 해석
- `cadkernel-modeling`: `mesh_quality()` — 종횡비, 체적, 퇴화 요소 검출
- `cadkernel-modeling`: `refine_tet_mesh()` — 엣지 중점 분할 (1→8 사면체)
- `cadkernel-modeling`: `extract_surface_mesh()` — 경계면 추출
- `cadkernel-modeling`: `merge_coincident_nodes()` — 허용 오차 내 노드 중복 제거
- `cadkernel-modeling`: `compute_stress_tensor()` — 요소별 6성분 응력 텐서 계산
- `cadkernel-modeling`: `compute_strain_tensor()` — 요소별 6성분 변형률 텐서 계산
- `cadkernel-modeling`: `principal_stresses()` — 3x3 응력 행렬의 Cardano 고유값 솔버
- `cadkernel-modeling`: `safety_factor()` — 항복응력 / 최대 von Mises 응력
- `cadkernel-modeling`: `strain_energy()` — 총 변형 에너지 계산
- `cadkernel-modeling`: `compute_reactions()` — 고정 노드의 반력 계산
- 34개 신규 테스트, 총 739개 테스트 (기존 705개)

#### Phase V11: 뷰어 UI 확장 (2026-03-17)
- `cadkernel-viewer`: 파일 메뉴 — STEP, IGES, DXF, PLY, 3MF, BREP 형식의 Import/Export
- `cadkernel-viewer`: 불리언 연산 다이얼로그 — 두 번째 박스 프리미티브로 Union/Subtract/Intersect (크기 + 오프셋 매개변수)
- `cadkernel-viewer`: Part 연산 — Mirror (XY/XZ/YZ), Scale, Shell, Fillet, Chamfer, Linear Pattern
- `cadkernel-viewer`: Mesh 툴바 — Smooth, Harmonize Normals, Check Watertight, Remesh, Repair
- `cadkernel-viewer`: 분석 도구 — Measure Solid (체적/면적/무게중심), Check Geometry (유효성 검증)
- `cadkernel-viewer`: PartDesign 툴바 업데이트 — Fillet/Chamfer/Shell/Mirror/Scale/Pattern 백엔드 연결
- `cadkernel-viewer`: ~20개 신규 `GuiAction` 변형 + 전체 `process_actions()` 핸들러
- `cadkernel-viewer`: 미사용 스텁 제거 (BooleanUnion/Subtract/Intersect, TrimDemo)

#### FreeCAD 수준 UI 대개편 Phase 2 (2026-03-23)

**다중 오브젝트 씬 아키텍처:**
- `scene.rs`: Scene + SceneObject — 오브젝트별 BRepModel, 메시, 색상, 가시성
- 모든 Create* 핸들러가 Scene에 오브젝트 추가 (다중 오브젝트 유지)
- 오브젝트별 GPU 렌더링 (개별 base_color 유니폼) + 선택 하이라이트 (초록 틴트)
- MAX_UNIFORM_SLOTS 64로 확장 (최대 ~58개 동시 오브젝트)

**모델 트리 (FreeCAD 스타일):**
- 오브젝트별 가시성 토글 (눈 아이콘, 초록/회색)
- 오브젝트별 색상 스와치 (8색 회전 팔레트)
- 선택 하이라이트 (파란 텍스트, 선택 시 토폴로지 상세)
- 컨텍스트 메뉴: 삭제, 복제, 변환, 측정, 지오메트리 검사
- 검색/필터 박스

**속성 패널 (Data/View 탭):**
- Data 탭: 기본 정보, 생성 매개변수, 토폴로지, 메시, 질량 속성
- View 탭: 색상, 가시성, 선택 상태
- 아무것도 선택 안 됐을 때 씬 개요

**하단 패널 (리포트 + Python 콘솔):**
- 탭: Report View + Python Console
- 콘솔: >>> 프롬프트, 명령 입력 + 이력 (PyO3 백엔드 플레이스홀더)

**다중 오브젝트 피킹:**
- 모든 가시 씬 오브젝트에 레이 테스트, 가장 가까운 히트 선택

**키보드 단축키:**
- Ctrl+Z (Undo), Ctrl+Y (Redo), Delete (삭제), Ctrl+N (새로만들기), F (맞춤), H (가시성)

**변환 도구:**
- Move (이동), Rotate (회전), Scale (크기), 컨텍스트 메뉴 프리셋, undo 지원

**툴바 아이콘 + 상태바:**
- Unicode 심볼, Show/Hide All, 오브젝트 수, 삼각형 수, 선택 이름

#### 심화 품질 개선 (2026-03-20)

**STEP I/O:**
- `cadkernel-io`: 서피스 인식 STEP 익스포트 — 경계 정점에서 실제 면 평면 계산 (더미 ORIGIN 평면 대체)
- `cadkernel-io`: B-spline 서피스 직렬화 — 완전한 B_SPLINE_SURFACE_WITH_KNOTS 출력 (빈 스텁 대체)
- `cadkernel-io`: STEP 파서 오류 복구 — 엔티티 해석 시 `catch_unwind`, 잘못된 엔티티는 중단 대신 `Other`로 저장

**불리언 연산:**
- `cadkernel-modeling`: `boolean_op`에 자동 면 분할 — 겹치는 면 감지 시 `split_solids_at_intersection` → 분류 → 평가 체인
- `cadkernel-modeling`: 다중 샘플 면 분류 — 중심점 + 6개 엣지 중점의 다수결 투표 (단일 중심점 테스트 대체)

**스케치 솔버:**
- `cadkernel-sketch`: DOF 분석 — `SolverResult`에 `remaining_dof` (야코비안 대각 랭크) 및 `over_constrained` 플래그 추가
- `cadkernel-sketch`: `drag_solve()` — 제약조건 유지하며 점 이동 (임시 Fixed 제약조건 방식)

**뷰어 인프라:**
- `cadkernel-viewer`: `picking.rs` — Moller-Trumbore CPU 레이-삼각형 교차, `screen_to_ray` 역투영, `pick_triangle` 최근접 히트 선택
- `cadkernel-viewer`: `command.rs` — Undo/redo `CommandStack` + `ModelSnapshot` (push/undo/redo, 최대 깊이, 새 명령 시 redo 무효화)
- 6개 신규 테스트 (picking 3 + command 3)

#### Phase V7: 파일 포맷 확장 (2026-03-19)
- `cadkernel-io`: glTF 2.0 임포트 — 내장 base64 버퍼 디코딩, 위치/법선/인덱스 추출, 다중 컴포넌트 타입 지원 (u8/u16/u32)
- `cadkernel-io`: 3MF 임포트 — XML vertex/triangle 파싱, 면 법선 계산
- `cadkernel-io`: DWG 임포트/익스포트 — 버전 감지 (R2000–R2018+), 3DFACE 휴리스틱 추출, DXF 기반 익스포트 폴백
- `cadkernel-io`: PDF 익스포트 — TechDraw SVG에서 최소 PDF 1.4 생성, SVG line/text→PDF 스트림 변환
- `cadkernel-io`: DAE (Collada) 임포트/익스포트 — COLLADA 1.4.1 XML, geometry/visual_scene, float_array + 삼각형 인덱스 파싱
- `cadkernel-io`: 10개 신규 테스트 (glTF 라운드트립, 3MF 라운드트립, DWG 버전 감지, PDF 생성, DAE 라운드트립)

#### Phase V13: 성능 & 검증 (2026-03-19)
- `cadkernel-modeling`: BVH 가속 불리언 broad-phase — O(n²) → O(n log n) 면 쌍 중첩 감지
- `cadkernel-modeling`: 11개 신규 Criterion 벤치마크 (총 25개) — cone, torus, mirror, scale, fillet, check_geometry, check_watertight, tessellate_sphere_64x32, tessellate_torus_64x32, boolean_intersection

#### Phase V12: Python 바인딩 (2026-03-18)
- `cadkernel-python`: PyO3 기반 신규 크레이트, `cadkernel` Python 모듈 (독립 빌드, workspace에서 제외)
- `cadkernel-python`: 6개 Python 클래스 — `Model`, `SolidHandle`, `Mesh`, `MassProperties`, `GeometryCheck`, `Sketch`
- `cadkernel-python`: 10개 프리미티브 생성 함수 (box, cylinder, sphere, cone, torus, tube, prism, wedge, ellipsoid, helix)
- `cadkernel-python`: 피처 연산 — `extrude_profile`, `revolve_profile`, `mirror`, `scale`
- `cadkernel-python`: 불리언 연산 — `boolean_union`, `boolean_subtract`, `boolean_intersect`
- `cadkernel-python`: 테셀레이션 & 분석 — `tessellate`, `mass_properties`, `geometry_check`
- `cadkernel-python`: I/O — `export_stl`, `export_obj`, `export_gltf`, `export_step`, `export_iges`, `import_stl`, `import_obj`, `save_project`, `load_project`
- `cadkernel-python`: 스케치 시스템 — 점, 선, 원, 7개 제약조건 타입, 솔버

#### FreeCAD 수준 UI 대개편 (2026-03-18)
- `cadkernel-viewer`: `gui.rs` (3605줄) → `gui/` 모듈 디렉토리로 리팩토링 (12개 파일)
  - `mod.rs`, `menu.rs`, `toolbar.rs`, `tree.rs`, `properties.rs`, `status_bar.rs`, `report.rs`, `dialogs.rs`, `sketch_ui.rs`, `overlays.rs`, `view_cube.rs`, `context_menu.rs`
- `cadkernel-viewer`: 계층형 모델 트리 — Solid→Shell→Face 구조, 생성 이력, 엔티티 선택
- `cadkernel-viewer`: 속성 편집기 — 엔티티별 속성 (Solid/Shell/Face/Edge/Vertex), 질량 속성
- `cadkernel-viewer`: 완전한 메뉴 시스템 — File/Edit/Create/View/Tools/Help, Import/Export 서브메뉴
- `cadkernel-viewer`: 향상된 상태바 — 마우스 좌표, FPS, 메쉬 정보, 디스플레이 모드
- `cadkernel-viewer`: 리포트 패널 — 색상 코드별 로그 (Info/Warning/Error), 자동 스크롤, Clear 버튼
- `cadkernel-viewer`: 컨텍스트 메뉴 — Solid (Select/Delete/Measure/Export), Viewport (Views/Display/Select)
- `cadkernel-viewer`: 툴바 개선 — 툴팁, 그룹 라벨, 구분선
- `cadkernel-viewer`: 3개 신규 Workbench 툴바 (Draft, Surface, FEM)
- `cadkernel-viewer`: `gui.log()` 리포트 로깅 40+ 액션 핸들러 (파일 I/O, 프리미티브, 불리언, Part 연산, Mesh 연산, 분석)
- `cadkernel-viewer`: 뷰포트 우클릭 컨텍스트 메뉴 연결 (Fit All, Reset Camera, Standard Views, Display Mode, Select/Deselect)

#### Phase B: 트림 서피스 & 정밀 B-Rep
- `cadkernel-modeling`: 5개 프리미티브 지오메트리 바인딩 — Box (6 Plane + 12 LineSegment), Cylinder (2 Plane + Cylinder 서피스 + LineSegment), Sphere (Sphere 서피스 + LineSegment), Cone/Frustum (Plane 캡 + Cone 서피스 + LineSegment), Torus (Torus 서피스 + LineSegment)
- `cadkernel-modeling`: `EdgeCache` 강화 — `Handle<EdgeData>` 저장, `all_edges()` 메서드, `bind_edge_line_segments()` 공용 헬퍼
- `cadkernel-modeling`: Sphere 남극 캡 와인딩 수정 — 링 방향 반전으로 올바른 외향 법선 (-Z)
- `cadkernel-geometry`: `ParametricWire2D` — UV 트림 경계용 닫힌 2D 곡선 체인 (와인딩 넘버 포함 판정, 호 길이 샘플링, 폴리라인 변환)
- `cadkernel-geometry`: `TrimmedSurface` 리팩터링 — `ParametricWire2D` 사용 (`from_curves()` 편의 생성자 추가)
- `cadkernel-topology`: `FaceData`에 `outer_trim` / `inner_trims` 필드 추가 (ParametricWire2D)
- `cadkernel-topology`: `EdgeData`에 `pcurve_left` / `pcurve_right` 필드 추가 (Curve2D)
- `cadkernel-topology`: `BRepModel::bind_face_trim()` 및 `BRepModel::bind_edge_pcurve()` API
- `cadkernel-io`: 트림 테셀레이션 — UV 중심점 기반 트림 와이어 필터링 (외곽 + 홀 제외)
- `cadkernel-viewer`: Part 워크벤치에 "Trim Demo" 액션 — 상단면에 원형 홀 트림된 박스 생성

#### Phase C: STEP I/O (전체 구현)
- `cadkernel-io`: 완전한 STEP 토크나이저 — ISO 10303-21 렉서 (부호-숫자 검증 포함)
- `cadkernel-io`: STEP 파서 — 엔터티 해석, 중첩 파라미터 파싱
- `cadkernel-io`: STEP 지오메트리 매핑 — CARTESIAN_POINT, DIRECTION, B_SPLINE_CURVE/SURFACE
- `cadkernel-io`: STEP 토폴로지 매핑 — VERTEX_POINT, EDGE_CURVE, FACE_BOUND, CLOSED_SHELL, MANIFOLD_SOLID_BREP
- `cadkernel-io`: STEP 내보내기 — `export_step()` B-Rep 모델용, `export_step_mesh()` 삼각형 메시용
- `cadkernel-io`: STEP 가져오기 — `import_step()` 엔터티 교차 참조 포함

#### Phase D: Fillet/Draft/Split (전체 구현)
- `cadkernel-modeling`: `fillet_edge()` — 호 근사 엣지 라운딩 (설정 가능한 반지름/세그먼트)
- `cadkernel-modeling`: `fillet_edge_segments()` — 세그먼트 수 조절 가능한 변형
- `cadkernel-modeling`: `draft_faces()` — 풀 축으로부터 방사형 정점 변위 (높이 × tan(각도))
- `cadkernel-modeling`: `split_solid()` — 평면에 대한 부호 거리로 정점 분류, 엣지-평면 교차, 캡 면 생성

#### Phase E: 고급 프리미티브
- `cadkernel-modeling`: `make_tube()` — 중공 실린더 (4개 정점 링, 4N 면, 외부/내부 Cylinder + 상/하 Plane 바인딩)
- `cadkernel-modeling`: `make_prism()` — 정다각형 프리즘 (N각형 캡 + N개 측면 쿼드)
- `cadkernel-modeling`: `make_wedge()` — 테이퍼 박스/피라미드 (WedgeParams, 상단 치수 < epsilon일 때 피라미드 모드)
- `cadkernel-modeling`: `make_ellipsoid()` — 3축 타원체 (독립적 rx, ry, rz 반축)
- `cadkernel-modeling`: `make_helix()` — 나선형 튜브/스프링 (로컬 Frenet 프레임, 튜브 단면 스윕)

#### Phase G: PartDesign 피처 연산
- `cadkernel-modeling`: `pad()` — 추가적 돌출 (프로파일 돌출 → 기본 솔리드와 Boolean 합집합)
- `cadkernel-modeling`: `pocket()` — 감산적 돌출 (프로파일 돌출 → 기본 솔리드에서 Boolean 차집합)
- `cadkernel-modeling`: `groove()` — 감산적 회전 (프로파일 회전 → 기본 솔리드에서 Boolean 차집합)
- `cadkernel-modeling`: `hole()` — 원통형 구멍 (다각형 원형 프로파일, 임의 방향, 돌출 + Boolean 차집합)
- `cadkernel-modeling`: `countersunk_hole()` — 2단계 구멍 (메인 + 더 큰 카운터싱크)

#### Phase H-I: 스케처 고급 구속조건
- `cadkernel-sketch`: `EqualLength` 구속조건 — 두 선분의 길이 동일 구속 (제곱 거리 공식)
- `cadkernel-sketch`: `Midpoint` 구속조건 — 점을 선분의 중점으로 구속 (2개 방정식)
- `cadkernel-sketch`: `Collinear` 구속조건 — 두 직선을 동일선상으로 구속 (점-직선 + 평행, 2개 방정식)
- `cadkernel-sketch`: `EqualRadius` 구속조건 — 두 원/호의 반지름 동일 구속 (제곱 거리 공식)
- `cadkernel-sketch`: `Concentric` 구속조건 — 두 중심점 일치 구속 (2개 방정식)
- 모든 5개 구속조건에 Newton-Raphson 솔버용 해석적 야코비안 포함

#### Phase F: Part 고급 연산
- `cadkernel-modeling`: `section_solid()` — 평면-면 교차에 의한 단면 윤곽선 계산 (면 경계에서의 엣지 감지)
- `cadkernel-modeling`: `offset_solid()` — 정점-법선 기반 솔리드 오프셋 (정점별 평균 법선, 설정 가능한 거리)
- `cadkernel-modeling`: `thickness_solid()` — 벽 두께 연산: 내부/외부 면 + 림 쿼드 생성 (Inward/Outward/Centered 결합 타입)
- `cadkernel-math`: `Mat4::translation(Vec3)` — 4x4 이동 행렬 생성
- `cadkernel-math`: `Mat4::transform_point(Point3)` — 동차좌표 점 변환 (w 나누기 포함)

#### Phase J: TechDraw 단면 & 상세 뷰
- `cadkernel-io`: `section_view()` — 솔리드 테셀레이트 → 삼각형-평면 교차 검출 → 2D 절단면 좌표 투영
- `cadkernel-io`: `detail_view()` — 기존 도면 뷰의 원형 영역 확대 (설정 가능한 배율)

#### Phase K: 어셈블리 기초
- `cadkernel-modeling`: Assembly 모듈 — `Assembly` 구조체: 컴포넌트 트리 + 구속조건 시스템
- `cadkernel-modeling`: `Component` — 배치 변환 (`Mat4`), 가시성 토글, 명명된 식별
- `cadkernel-modeling`: `AssemblyConstraint` 열거형 — Fixed, Coincident, Concentric, Distance, Angle 구속 타입
- `cadkernel-modeling`: 어셈블리 컴포넌트 간 바운딩박스 간섭 검출
- `cadkernel-modeling`: `translation(dx, dy, dz)` 컴포넌트 배치 헬퍼

#### Phase L: Draft 워크벤치
- `cadkernel-modeling`: `make_wire()` — 점 시퀀스로부터 3D 폴리라인 와이어 생성 (닫힌 와이어 자동 감지)
- `cadkernel-modeling`: `make_bspline_wire()` — 제어점으로부터 B-spline 와이어 생성 (클램핑된 균일 knot 벡터)
- `cadkernel-modeling`: `clone_solid()` — 항등 변환을 통한 동일 위치 솔리드 깊은 복사
- `cadkernel-modeling`: `rectangular_array()` — 2D 그리드 패턴 (count_x × count_y) 두 방향 벡터를 따라
- `cadkernel-modeling`: `path_array()` — 경로 점들을 따라 솔리드 복사 (이동 오프셋)

#### Phase M: 메시 고급 연산
- `cadkernel-io`: `decimate_mesh()` — 엣지 붕괴 메시 간소화 (목표 비율, 최단 엣지 우선)
- `cadkernel-io`: `fill_holes()` — 경계 엣지 검출, 루프 체이닝, 중심점 팬 삼각화
- `cadkernel-io`: `compute_curvature()` — 코탄젠트 가중 Laplace-Beltrami 연산자에 의한 정점별 평균 곡률
- `cadkernel-io`: `subdivide_mesh()` — 중점 분할 (각 삼각형 → 4 삼각형) 엣지 중점 중복 제거 포함
- `cadkernel-io`: `flip_normals()` — 와인딩 순서 반전 및 법선 부정

#### Phase O: Surface 워크벤치
- `cadkernel-modeling`: `ruled_surface()` — 두 NurbsCurve 사이의 선형 보간 서피스
- `cadkernel-modeling`: `surface_from_curves()` — 프로파일 곡선 네트워크로부터 Gordon형 서피스 구성
- `cadkernel-modeling`: `extend_surface()` — 기존 솔리드 면의 정점-법선 오프셋 확장
- `cadkernel-modeling`: `pipe_surface()` — 경로 곡선을 따른 관형 솔리드 (Frenet 프레임 + 끝단 캡)

#### Phase N: FEM 기초
- `cadkernel-modeling`: `TetMesh` 구조체 — 노드 + 사면체 요소 인덱스
- `cadkernel-modeling`: `FemMaterial` — 철강 `steel()` / 알루미늄 `aluminum()` 프리셋
- `cadkernel-modeling`: `BoundaryCondition` — FixedNode, Force, Pressure 경계조건
- `cadkernel-modeling`: `generate_tet_mesh()` — 바운딩박스 분할 → 적합 사면체 생성 (교대 패리티)
- `cadkernel-modeling`: `static_analysis()` — 요소 강성 행렬 조립, Gauss-Seidel 솔버, von Mises 응력 계산

#### Phase P: IGES I/O
- `cadkernel-io`: 80열 고정 포맷 IGES 리더/라이터
- `cadkernel-io`: `IgesEntity` + `IgesEntityType` (Point 116, Line 110, Arc 100, NURBS Curve 126, Surface 128)
- `cadkernel-io`: `parse_iges()` — 섹션 분류 (S/G/D/P/T), Directory Entry 쌍, Parameter Data 추출
- `cadkernel-io`: `import_iges()` — Point/Line 엔터티 → BRepModel 정점/엣지
- `cadkernel-io`: `export_iges()` / `export_iges_mesh()` — B-Rep/메시 → IGES 포맷

#### Phase Q: 성능 최적화
- `cadkernel-geometry`: BVH (Bounding Volume Hierarchy) — AABB 기반 공간 인덱스 트리 (최장축 중점 분할)
- `cadkernel-geometry`: `Aabb` 구조체 — 축 정렬 바운딩 박스 (merge, intersects, contains_point, surface_area, 광선 교차 slab 테스트)
- `cadkernel-geometry`: `Bvh` 구조체 — build, query_aabb, query_point, query_ray 메서드
- `cadkernel-io`: `tessellate_solid_parallel()` — rayon 기반 병렬 면 테셀레이션 + 메시 병합
- `cadkernel-io`: `merge_meshes()` — 다중 Mesh 객체 결합 (정점/인덱스 오프셋 추적)

#### Phase R: 지오메트리 커널 확장
- `cadkernel-geometry`: `IsocurveU` / `IsocurveV` — 서피스에서 일정 u/v 파라미터에서 곡선 추출
- `cadkernel-geometry`: `surface_curvatures()` — 제1/제2 기본형식을 통한 가우스, 평균, 주곡률 계산
- `cadkernel-geometry`: `OffsetCurve` — 참조 평면 내 고정 거리 3D 평행 곡선
- `cadkernel-geometry`: `RevolutionSurface` — 프로파일 곡선의 로드리게스 회전을 통한 회전 서피스
- `cadkernel-geometry`: `ExtrusionSurface` — 해석적 du/dv를 갖는 병진 스위프 서피스
- `cadkernel-geometry`: `blend_curve()` — 두 곡선 사이의 3차 베지어 G0/G1 브릿지
- `cadkernel-geometry`: `check_surface_continuity()` — 인접 서피스 간 G0/G1/G2 연속성 분석

#### Phase S: 모델링 확장
- `cadkernel-modeling`: `make_spiral()` — 평면 아르키메데스 나선 튜브 솔리드
- `cadkernel-modeling`: `make_polygon()` — 정다각형 프리즘 (make_prism 위임)
- `cadkernel-modeling`: `make_plane_face()` — 얇은 박스 형태의 평면 직사각형 면
- `cadkernel-modeling`: `boolean_xor()` — 배타적 OR 불리언 (합집합 빼기 교집합)
- `cadkernel-modeling`: `Compound` — 불리언 없이 솔리드 그룹화 (add/explode)
- `cadkernel-modeling`: `check_geometry()` — 토폴로지 유효성 검사 (쉘, 면, 루프, 엣지, 정점)
- `cadkernel-modeling`: `check_watertight()` — 매니폴드 엣지 공유 검증
- `cadkernel-modeling`: `multi_transform()` — Translation/Rotation/Scale/Mirror 변환 체인
- `cadkernel-modeling`: `Body` — PartDesign 피처 트리 컨테이너 (팁 추적)
- `cadkernel-modeling`: `make_involute_gear()` — 매개변수 치형 프로파일의 인볼류트 스퍼 기어 솔리드

#### Phase T: 스케처 확장
- `cadkernel-sketch`: 5개 새 구속 타입 — Diameter, Block, HorizontalDistance, VerticalDistance, PointOnObject
- `cadkernel-sketch`: `SketchEllipse` / `EllipseId` — 중심, 장축 끝점, 단축 반지름을 갖는 타원 엔티티
- `cadkernel-sketch`: `SketchBSpline` / `BSplineId` — 제어점, 차수, 닫힘 플래그를 갖는 B-스플라인 엔티티
- `cadkernel-sketch`: `add_polyline()` — 점 시퀀스로부터 다중 세그먼트 선 생성
- `cadkernel-sketch`: `add_regular_polygon()` — 자동 생성된 점과 선을 갖는 정 N각형
- `cadkernel-sketch`: `add_arc_3pt()` — 외접원 계산을 통한 3점 호

#### Phase U: 파일 포맷 확장 & 메시 연산
- `cadkernel-io`: DXF 가져오기/내보내기 — 3DFACE 엔티티 매핑
- `cadkernel-io`: PLY 가져오기/내보내기 — 법선 포함 ASCII 포맷
- `cadkernel-io`: 3MF 내보내기 — XML 기반 3D 제조 포맷
- `cadkernel-io`: BREP 텍스트 포맷 가져오기/내보내기 — CADKernel 네이티브 B-Rep 직렬화
- `cadkernel-io`: `smooth_mesh()` — 인접 기반 반복 라플라시안 스무딩
- `cadkernel-io`: `mesh_boolean_union()` — 단순 삼각형 레벨 메시 병합
- `cadkernel-io`: `cut_mesh_with_plane()` — 삼각형 세분화를 통한 평면 클리핑
- `cadkernel-io`: `mesh_section_from_plane()` — 단면 윤곽선 추출
- `cadkernel-io`: `split_mesh_by_components()` — 유니온-파인드 컴포넌트 분리
- `cadkernel-io`: `harmonize_normals()` — 일관된 법선을 위한 BFS 와인딩 전파
- `cadkernel-io`: `check_mesh_watertight()` — 엣지 카운트 수밀성 검사
- `cadkernel-io`: `DimensionType` 열거형 — 6개 TechDraw 치수 타입 (길이, 수평/수직, 반지름, 지름, 각도) + SVG 렌더링

#### UI: 메시 연산 + 새 프리미티브 툴바
- `cadkernel-viewer`: Mesh 워크벤치 툴바 — Decimate 50%, Subdivide, Fill Holes, Flip Normals 버튼
- `cadkernel-viewer`: 메시 연산 액션 처리 (오류 처리 + 상태 메시지 포함)
- `cadkernel-viewer`: 5개 새 프리미티브 생성 다이얼로그 — Tube, Prism, Wedge, Ellipsoid, Helix (파라미터 입력)
- `cadkernel-viewer`: Part 워크벤치 툴바 확장 — 총 10개 프리미티브 (기존 5 + Tube, Prism, Wedge, Ellipsoid, Helix)
- `cadkernel-viewer`: Create 메뉴 확장 — 구분선과 함께 5개 새 항목
- `cadkernel-viewer`: 5개 새 프리미티브에 대한 완전한 액션 처리 (모델 생성 + 테셀레이션 + 표시)

#### Application Phase 6: 나머지 이슈 해결
- `cadkernel-modeling`: `point_in_solid()` 2D 점-다각형 판별 테스트로 재작성 (교차 수 알고리즘 + 면 평면 투영, 부정확한 바운딩 박스 검사 대체)
- `cadkernel-geometry`: Line/Plane 해석적 `project_point` 오버라이드 (무한 기하에 대한 정확한 해, 샘플링 NaN 방지)
- `cadkernel-geometry`: Line/Plane `bounding_box` 유한 폴백 도메인 오버라이드 (±1e6)
- `cadkernel-modeling`: 프리미티브 엣지 중복 제거 `EdgeCache` — Box (24→12 엣지), Cylinder (6N→3N 엣지), Sphere 올바른 하프엣지 공유. B-Rep 검증을 위한 정확한 매니폴드 토폴로지

### 수정됨

#### CRITICAL
- `cadkernel-geometry`: `arbitrary_perpendicular` unwrap → `unwrap_or(Vec3::X)` (circle.rs, cylinder.rs)
- `cadkernel-io`: 바이너리 STL 읽기 시 삼각형 수 제한 (5000만 개) — 악성 파일의 OOM 방지
- `cadkernel-io`: 바이너리 STL 쓰기 시 u32 오버플로우 검사 (`write_stl_binary`가 `KernelResult` 반환)
- `cadkernel-io`: STEP/IGES `todo!()` 패닉을 `Err(IoError)`로 대체 — 안전한 오류 처리
- `cadkernel-modeling`: `classify_face` 오프셋 방향 수정 (내부 → 외부 법선 오프셋)
- `cadkernel-modeling`: `compute_mass_properties` 0에 가까운 체적 가드 + 조기 반환
- `cadkernel-modeling`: `solid_mass_properties` `todo!()`를 `Err`로 대체
- `cadkernel-topology`: EntityStore generation 타입 u32 → u64 (장시간 실행 시 오버플로우 방지)
- `cadkernel-modeling`: `point_in_solid()` 재작성 — 2D 교차 수 테스트를 사용한 정확한 레이-다각형 교차 (부정확한 바운딩 박스 검사 대체)

#### HIGH
- `cadkernel-geometry`: Sphere/Torus/Cone 생성자 매개변수 검증 (`radius > 0`, `half_angle ∈ (0, π/2)`) — `KernelResult` 반환
- `cadkernel-geometry`: NurbsCurve de_boor 0 가중치 가드 (0으로 나눗셈 방지)
- `cadkernel-topology`: `loop_half_edges` 최대 반복 가드 (10만 회 제한 — 손상된 토폴로지에서 무한 루프 방지)
- `cadkernel-sketch`: 각도 제약조건 `tan()` 특이점을 `atan2(cross, dot) - theta`로 대체
- `cadkernel-sketch`: Profile `extract_profile` 경계 검사된 포인트 접근
- `cadkernel-geometry`: Line/Plane 무한 도메인 — 해석적 `project_point` + 유한 `bounding_box` 오버라이드 (기본 샘플링 NaN 방지)
- `cadkernel-modeling`: 프리미티브 중복 엣지 — Box/Cylinder/Sphere용 `EdgeCache` 중복 제거 시스템 (올바른 매니폴드 하프엣지 토폴로지)

#### MEDIUM
- `cadkernel-topology`: `validate()`에서 오일러 특성 V-E+F=2 검증 추가
- `cadkernel-io`: SVG XML 엔티티 이스케이핑 (`&`, `<`, `>`, `"`, `'`) — 스타일 속성값
- `cadkernel-sketch`: `WorkPlane::new` Gram-Schmidt 직교화 (x_axis가 법선에 수직)
- `cadkernel-viewer`: BFS 스무스 그룹 최적화 — 엣지 기반 로컬 인접 리스트로 정점별 면 그루핑
