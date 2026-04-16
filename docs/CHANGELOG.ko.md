# 변경 이력

[English](../CHANGELOG.md) | **한국어**

이 프로젝트의 주요 변경 사항은 이 문서에 기록됩니다.

형식은 [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)를 기반으로 하며,
버전 정책은 [Semantic Versioning](https://semver.org/lang/ko/)을 지향합니다.

## [Unreleased]

### 추가됨

#### V33: 프로덕션 준비 — 테스트 커버리지, 패키징 & 문서 (2026-04-17)

**코어 크레이트 신규 통합 테스트 39개** (`crates/core/tests/core_comprehensive.rs`):

- **에러 생성** (6개): 6가지 `KernelError` 변형 전체 생성 및 매칭 확인
- **Display 포맷** (7개): 각 변형이 올바른 접두어 및 전체 메시지를 포함하는지 확인
- **Predicate 메서드** (3개): `is_invalid_handle`, `is_invalid_argument`, `is_io_error`가 해당 변형에서만 true 반환
- **with_context** (7개): 5개 문자열 바디 변형에 컨텍스트 선두 추가, `InvalidHandle`은 그대로 통과, 변형 판별자 보존
- **KernelResult 패턴** (5개): Ok 통과, Err 유지, `?` 전파, map, map_err
- **From<std::io::Error>** (3개): NotFound, PermissionDenied, `?` 연산자 변환이 모두 IoError 생성
- **std::error::Error 트레이트** (2개): Display와 to_string 일치, 기본 source 없음
- **Clone + PartialEq** (4개): 복제 동등, 변형 간 불일치, 동일/상이 메시지 비교
- **Send + Sync** (2개): `KernelError` 및 `KernelResult<()>` 스레드 안전성 확인

코어 크레이트 커버리지: 10 → 49 테스트 (390% 증가).

**뷰어 크레이트 신규 통합 테스트 101개** (`crates/viewer/tests/viewer_comprehensive.rs`):

- **compute_aabb** (4개): 빈 입력, 단일 정점, 다중 정점 범위, 단위 큐브
- **DisplayMode** (6개): ALL 슬라이스 크기, 레이블, 단축키, 동등성
- **Projection / StandardView / Camera** (16개): 투영 전환, 뷰 스냅, 리셋, 바운드 맞춤, eye 위치, 행렬 크기, 화면 right/up 단위 길이
- **NavConfig** (9개): 기본 스타일, 줌 팩터, FreeCAD/Blender/Maya resolve_drag, snap_3d 켜짐/꺼짐
- **NavStyle / OrbitStyle / RotationMode / UnitSystem / BgPreset** (7개): 레이블/설명 비어있지 않음, mm 단위 레이블
- **CreationParams 직렬화** (2개): Box, Sphere JSON 왕복
- **Scene (헤드리스, add_mesh_object 사용)** (27개): 빈 씬, 추가/삭제, ID 순서, 가시성, 선택, 정렬, 부모/자식, 그룹 관리
- **ScriptEngine** (29개): 엔진 생성, 숫자/문자/불리언/nil 반환, 샌드박스, cad 테이블, 5가지 기본 도형, 다중 누적, clear/delete/list/measure/count/translate, 문법 오류

뷰어 크레이트 커버리지: 50 → 151 테스트 (202% 증가).

**IO 라운드트립 통합 테스트 39개 신규** (`crates/io/tests/format_roundtrip.rs`):

- 빈 모델/다중 솔리드 라운드트립 (STEP, BREP, JSON)
- STEP 이중 라운드트립 일관성 및 면 수 보존
- 신규 포맷 라운드트립: glTF, 3MF, DAE, AMF, VRML, OCA, DXF, DWG
- SVG/PDF 내보내기 검증, 네이티브 .cadk 저장/로드
- 대용량 메시 라운드트립 (10-박스 STL, 900-정점 OBJ 그리드)
- 크로스 포맷 삼각형 수 일관성 (8개 포맷 비교)
- 메시 연산: 법선 조화/반전, 수밀 검사, 스케일, 불리언, 플라토닉 다면체
- 엣지 케이스: 음수 좌표, 부동소수점 정밀도, 바이너리 STL 크기

IO 크레이트 라운드트립 커버리지: 22 → 61 테스트 (177% 증가).

**Python 패키징 정비:**
- `pyproject.toml`에 Changelog/Documentation URL, dev 의존성, manylinux2014 호환성 추가

**CONTRIBUTING.md 개선 (EN + KO):**
- 개발 환경 설정, 코드 표준, PR 워크플로우, 아키텍처 개요, 기여 시작점

---

#### V32: 토폴로지 크레이트 테스트 커버리지 확장 (2026-04-15)

**33개 신규 통합 테스트** (`crates/topology/tests/topology_comprehensive.rs`):

- **Tag/Naming 시스템** (8개): `modified()`, `merged()`, 체인 연산, display/debug, 해시 일관성
- **NameMap** (5개): 타입별 getter, remove, len/is_empty, iter, 직렬화 왕복
- **ShapeHistory** (3개): `current_op_id`, `get_record`, 전체 `Evolution` 변형
- **ModelHistory 실행 취소/재실행** (6개): 기본 undo/redo, 빈 상태 None 반환, 녹화 시 redo 스택 삭제, max_history 제한, 이력 설명, 다단계 undo/redo
- **기하 바인딩** (5개): 곡선/곡면/트림/pcurve 바인딩, 유효하지 않은 핸들 처리
- **내부 루프** (2개): 단일 및 다중 구멍 추가
- **와이어** (4개): 개방/폐쇄, 태그 부여, 빈 와이어
- **태그 엔티티** (5개): 정점/에지/셸/솔리드 태그 생성 및 조회
- **순회/속성/핸들** (11개): `faces_around_vertex`, PropertyStore, Color, Material, Handle, EntityStore 확장

토폴로지 크레이트 커버리지: 29 → 62 테스트 (114% 증가).

#### V31: 포괄적 코드 감사 — 정확성, 보안 & 성능 (2026-04-13)

**치명적/높은 정확성 수정:**
- `compute_mass_properties()` 중심점 계산 수정 — 4.0으로 나누던 것을 발산 정리 공식(1/24 정규화)으로 변경
- 스케치 솔버 Jacobian 랭크 추정 수정 — 신뢰할 수 없는 대각선 검사를 SVD 특이값 임계값으로 교체
- Armijo 선형 탐색 폴백 수정 — alpha가 임계값 이하일 때 거부된 근소 스텝을 적용하지 않도록 변경
- glTF import에서 NORMAL accessor의 per-vertex 법선을 읽도록 수정 (이전에는 무시)
- 빈 vertex의 AABB 센티넬 수정 — `[MIN,MAX]` 대신 `[0,0,0]` 반환 (절두체 컬링 오류 방지)

**보안 하드닝:**
- Lua 스크립팅 엔진 샌드박스 — `os`, `io`, `require`, `dofile`, `loadfile`, `package` 전역 제거
- STEP 토크나이저가 종료되지 않은 주석/문자열/열거형에서 무음 OOB 대신 오류 반환하도록 수정
- 입력 크기 제한 추가: STEP (512MB), ASCII STL (256MB), OBJ (256MB)
- MCP 서버 solid 개수 제한(1000) 추가 — 반복적 `create_primitive` 호출로 인한 메모리 소진 방지
- 3MF, PLY, BREP export 시 NaN/무한대 좌표 검증 추가

**정확성 개선:**
- NurbsCurve 생성자 검증 추가: 양수 가중치, 비감소 매듭 벡터, 최소 차수/제어점 수
- Lua `cad.scale`에서 비균일(sx≠sy≠sz) 스케일링을 무음 평균 대신 명확한 오류로 거부하도록 수정
- glTF `compute_per_vertex_normals`가 per-face/per-vertex 법선 배열 모두 처리하도록 수정

**성능 최적화:**
- 적응적 곡선 테셀레이션을 O(n²) Vec::insert에서 O(n) 배치 스윕으로 재작성

#### V30: 감사 기반 하드닝 & 뷰어 선택 수정 (2026-04-13)

**뷰어 선택 & 렌더링 안정성:**
- 자동 피킹과 프리셀렉션이 씬에서 처음 만난 객체가 아니라 가장 가까운 vertex/edge/face 히트를 선택하도록 수정
- 커서 hover 변경 시 전체 scene GPU 데이터 재빌드를 수행하지 않도록 수정
- 오브젝트별 솔리드 렌더링이 단일 dynamic uniform slot을 재사용하도록 변경되어, shaded per-object 모드의 조용한 객체 수 상한 제거
- edge/vertex 피킹이 outer loop뿐 아니라 inner loop(홀)도 포함하도록 수정
- 선택 토글 시 결합된 scene geometry를 재생성하지 않고 오브젝트 메타데이터만 갱신하도록 수정
- `DisplayMode::Points`가 wireframe line pipeline이 아니라 전용 GPU point pipeline을 사용하도록 수정

**I/O 검증 하드닝:**
- `import_ply()`가 vertex/face 개수 상한을 적용하고 범위를 벗어난 face index를 거부하도록 수정
- `import_gltf()`가 accessor/bufferView 인덱스, 바이트 범위, index-to-vertex 참조를 디코딩 전에 검증하도록 수정
- 과도하게 큰 MCP JSON-RPC 요청은 파싱 전에 크기 제한으로 거부하도록 수정
- `import_3mf()`가 범위를 벗어난 triangle index를 거부하고 vertex/triangle 개수 상한을 적용하도록 수정
- `import_step()`가 해석할 수 없는 필수 point 참조를 오류로 처리하고, STEP entity 해석 실패 시 파싱을 중단하도록 수정
- `export_step()`과 `export_brep()`가 끊어진 topology 참조를 기본값으로 직렬화하지 않고 오류를 반환하도록 수정

**CI 커버리지 개선:**
- `.github/workflows/ci.yml`이 제외되어 있던 `crates/python` 바인딩 크레이트를 별도 빌드/테스트하도록 수정
- `.github/workflows/python-release.yml` publish job을 단순화해 잘못된 workflow environment 검증 문제를 제거

### 변경됨

- 감사 기반 검증 범위가 workspace 1,616개 테스트와 Python 바인딩 42개 테스트까지 확장됨

#### V29: 성능, 테스트 & 패키징 (2026-04-13)

**프러스텀 컬링 & 오브젝트별 AABB:**
- 테셀레이션 시점에 `SceneObject`에 축 정렬 바운딩 박스(AABB) 계산
- 뷰-프로젝션 행렬에서 뷰 프러스텀 평면 추출 (`extract_frustum_planes`)
- AABB-프러스텀 교차 테스트 (`aabb_in_frustum`)로 화면 밖 오브젝트 스킵
- 모든 솔리드 디스플레이 모드에서 오브젝트별 프러스텀 컬링 (Shading, NoShading, Transparent)
- 카메라 뷰 밖의 오브젝트가 있는 대규모 어셈블리에서 GPU 드로우 콜 감소

**형식 라운드트립 통합 테스트:**
- `crates/io/tests/format_roundtrip.rs`에 22개 통합 테스트
- STEP, IGES, STL (ascii/binary), OBJ, PLY, BREP, JSON 라운드트립 커버
- 메쉬 처리 테스트: 데시메이션, 세분화, 스무딩
- 테셀레이션 검증: 직렬, 병렬, 페이스 맵 변형
- 멀티 솔리드 및 꼭짓점 좌표 보존 테스트

**Python 휠 CI:**
- `.github/workflows/python-release.yml` — `PyO3/maturin-action`을 통한 자동 휠 빌드
- 5개 타겟 매트릭스: Linux (x86_64, aarch64), macOS (x86_64, aarch64), Windows (x86_64)
- 소스 배포(sdist)
- 버전 태그 시 PyPI 자동 배포

**커뮤니티 기여 온보딩:**
- `CONTRIBUTING.md`에 아키텍처 개요, 크레이트 의존성 다이어그램 추가
- 구체적 기여 영역이 포함된 "처음 기여하기 좋은 영역" 섹션
- 신규 기여자를 위한 코딩 규칙 요약
- `docs/CONTRIBUTING.ko.md` 한국어 번역 업데이트

### 변경됨
- 감사 기반 수정이 `cargo build`, `cargo clippy -D warnings`, `cargo test --workspace`, `cargo test --manifest-path crates/python/Cargo.toml`로 검증됨
- NoShading, Transparent 모드에서도 오브젝트별 렌더링 확장 (기존 Shading만)
- 전체 크레이트에서 1,612개 테스트 통과 (기존 1,606개)

#### V28: 뷰어 통합 & 프로덕션 폴리시 (2026-04-08)

**Lua 콘솔:**
- Report 패널에 대화형 Lua 탭 추가: 코드 입력 필드, 출력 표시, 명령 이력
- `GuiAction::ExecuteLuaCode` — 현재 입력 줄 실행
- `GuiAction::ExecuteLuaFile` — 디스크에서 `.lua` 파일 불러와 실행
- `GuiAction::ClearLuaConsole` — 콘솔 출력 버퍼 지우기

**플러그인 매니저:**
- 등록된 플러그인 목록(이름, 상태, 활성화/비활성화 토글)을 표시하는 UI 다이얼로그
- `GuiAction::TogglePluginManager` — 플러그인 매니저 다이얼로그 열기/닫기
- `GuiAction::InitPlugins` — 시작 시 플러그인 레지스트리 (재)초기화

**MCP 서버 컨트롤:**
- Tools 메뉴에서 MCP 서버 시작/중지 (CLI 불필요)
- `GuiAction::StartMcpServer` — JSON-RPC 2.0 MCP 서버 시작
- `GuiAction::StopMcpServer` — 실행 중인 MCP 서버 정상 종료
- 상태 바에 서버 상태 인디케이터 표시

**예제 스크립트:**
- `examples/lua/hello_cad.lua` — 기본 지오메트리 생성 및 내보내기
- `examples/lua/boolean_operations.lua` — union/subtract/intersect 파이프라인
- `examples/lua/parametric_part.lua` — 파라미터 기반 브래킷 모델
- `examples/lua/batch_export.lua` — 솔리드를 여러 형식으로 한 번에 내보내기
- `examples/lua/assembly.lua` — 다중 컴포넌트 어셈블리 (제약 조건 포함)
- `examples/python/basic_modeling.py` — PyO3 프리미티브 및 불리언 예제
- `examples/python/batch_analysis.py` — 파일 목록에 대한 질량 속성 루프
- `examples/mcp/session.json` — 주석이 달린 JSON-RPC 세션 트랜스크립트

**프로젝트 템플릿:**
- `templates/template_empty.cadk` — 빈 모델 (올바른 스키마)
- `templates/template_single_box.cadk` — 10×10×10 박스 하나
- `templates/template_basic_assembly.cadk` — 두 컴포넌트 어셈블리 골격
- `templates/template_mechanical_part.cadk` — 필렛이 있는 플랜지 브래킷
- `templates/template_gear_demo.cadk` — 파라메트릭 스퍼 기어 (m=2, z=20)

**편의 API (`cadkernel-modeling`):**
- `quick_box / quick_cylinder / quick_sphere / quick_cone / quick_torus` — 위치 인수를 사용하는 단일 호출 프리미티브 생성자
- `quick_union / quick_subtract / quick_intersect` — 두 솔리드 불리언 헬퍼
- `quick_volume / quick_area / quick_centroid / quick_bbox` — 단일 호출 질량 속성 조회

**에러 메시지 개선:**
- 모든 `KernelError` 변형에 사람이 읽을 수 있는 컨텍스트 문자열 추가 (연산명, 파라미터 값)
- 뷰어 상태 바 및 Report 패널에서 원시 디버그 출력 대신 구조화된 에러 메시지 표시

#### V27: 확장 생태계 — 플러그인 API, MCP 서버, Lua 스크립팅 (2026-04-08)

**플러그인 API:**
- 생명주기 관리(init/shutdown/commands)가 있는 Plugin 트레이트
- register/unregister/execute_command를 갖춘 PluginRegistry
- 내장 예제 플러그인: ValidationPlugin, AutoNamingPlugin, StatisticsPlugin

**MCP 서버 (Model Context Protocol):**
- JSON-RPC 2.0 프로토콜 구현
- 8개 AI 통합 도구: create_primitive, boolean_operation, transform, query_model, measure, export_model, delete_solid, list_solids
- handle_request() 디스패치를 갖춘 McpServer 구조체

**Lua 스크립팅 엔진:**
- 자동화를 위한 임베디드 스크립팅 엔진
- 프리미티브, 불리언, 변환, 피처, I/O, 쿼리 명령 지원
- 스크립트 실행을 위한 execute()/execute_file() API

**CI/CD & 패키징:**
- GitHub Actions CI: 매트릭스 빌드(Linux/macOS/Windows), cargo 캐시
- GitHub Actions Release: 태그 푸시 시 자동 바이너리 빌드
- Python 패키징: maturin 백엔드를 사용하는 pyproject.toml

#### V26: API 레퍼런스 & 문서화 (2026-04-08)

**Rust Doc Comments:**
- 8개 크레이트 `lib.rs` 전체에 모듈 수준 `//!` 문서 추가 (사용 예제 포함)
- 55개 파일에 걸쳐 ~295개 공개 타입, 함수, 필드에 `///` 문서 주석 추가
- 8개 새 컴파일 가능 `# Examples` doc test 블록 (core, math, geometry, topology, modeling, sketch)
- 주요 문서화 타입: `KernelError`, `Vec3`, `Point3`, `Mat4`, `Transform`, `Curve`, `Surface`, `NurbsCurve`, `NurbsSurface`, `BRepModel`, `Handle<T>`, `EntityStore`, `Sketch`, 전체 I/O 함수

**README 업데이트 (영문 + 한국어):**
- 비교표 업데이트: 3D 모델링, 파라메트릭 설계, B-Rep+NURBS, STEP → 전부 ✅
- Python 바인딩 행, 테스트 커버리지 행 추가 (1,450+)
- 파일 형식 테이블: 15+개 형식 🔲/🚧 → ✅ (STEP, IGES, BREP, DXF, DWG, SVG, PDF, glTF, PLY, 3MF, AMF, COLLADA, VRML, .cadk)
- 로드맵: Application Phase 3 (FreeCAD 동등성), Phase 4 (성능) 완료 표시
- 데모 섹션 확장: 14개 기능 카테고리, 전체 기능 목록
- FAQ 업데이트: 프로덕션 준비 상태 — 576/576 기능 동등성 반영
- 한국어 README (`docs/README.ko.md`) 영문과 동일하게 미러링

#### V25: 성능, 테스트 & Python 스프린트 (2026-04-08)

**성능 최적화:**
- `crates/modeling/src/boolean/`: `rayon::par_iter()`를 사용한 불리언 면 분류 루프 병렬화 — 멀티코어 환경에서 불리언 연산 시간 단축
- `crates/modeling/src/features/`: `linear_pattern`, `circular_pattern`의 복제 루프에 `rayon` 병렬 반복 적용
- `crates/geometry/src/bvh.rs`: `query_nearest()` — 가장 가까운 단일 AABB 항목 반환; `query_ray()` — 레이와 교차하는 모든 AABB 항목 반환
- `crates/geometry/src/bvh.rs`: 노드 탐색 시 스택 할당 버퍼 재사용으로 쿼리당 힙 할당 감소
- `crates/geometry/src/curve/bspline_basis.rs`: `BasisCache` LRU 캐시(1024개 항목) — `CachedNurbsCurve`와 `NurbsSurface::evaluate_cached()`에서 공유
- `crates/modeling/benches/modeling_benchmarks.rs`: 4개 신규 Criterion 벤치마크 추가 — `bench_parallel_boolean`, `bench_bvh_query_nearest`, `bench_bvh_query_ray`, `bench_pattern_parallel` (총 25개)

**종합 스트레스 테스트:**
- `crates/modeling/tests/stress_tests.rs`: 18개 신규 스트레스 테스트 추가:
  - 다중 불리언 체인 (10회 이상 순차 union/subtract/intersect 연산)
  - 대형 어셈블리 (50개 컴포넌트 간섭 검출)
  - 복합 스케치 (접선 호 + 일치 체인을 포함한 30개 이상 제약조건 시스템)
  - 패턴 스트레스 (64개 인스턴스 linear/circular 패턴)
  - FEM 메시 품질 (대형 바디에서 tet 메시 생성 + 품질 지표)
  - Surface 연산 체인 (ruled → extend → pipe 파이프라인)
  - 에지 케이스 커버: 퇴화 입력, 길이 0 엣지, 거의 일치하는 정점, 빈 컴파운드, 단일 면 솔리드 불리언
- 스트레스 테스트 총계: 49개 → 67개

**Python 바인딩 확장:**
- `crates/python/src/lib.rs`: `PyAssembly` 클래스 — `add_component`, `add_constraint`, `solve_constraints`, `check_interference`, `bill_of_materials` 노출
- `crates/python/src/lib.rs`: `PyFem` 클래스 — `generate_tet_mesh`, `static_analysis`, `modal_analysis`, `mesh_quality` 노출
- `crates/python/src/lib.rs`: Draft 연산 바인딩 — `make_wire`, `make_bspline_wire`, `rectangular_array`, `path_array`, `clone_solid`
- `crates/python/src/lib.rs`: Surface 연산 바인딩 — `ruled_surface`, `surface_from_curves`, `extend_surface`, `pipe_surface`
- `crates/python/src/lib.rs`: Compound 연산 바인딩 — `boolean_fragments`, `slice_to_compound`, `compound_filter`, `explode_compound`
- 신규 바인딩 클래스를 커버하는 Python 통합 테스트 74개 추가

**I/O 에지 케이스 테스트:**
- `crates/io/`: STL, OBJ, glTF, PLY, BREP 포맷 라운드트립 일관성 테스트
- `crates/io/`: 빈 모델 익스포트/임포트 테스트 (면이 없는 솔리드)
- `crates/io/`: 대형 메시 테스트 (100K+ 삼각형 STL 읽기 + 쓰기)
- `crates/io/`: 포맷별 에지 케이스 — 삼각형 수 0인 바이너리 STL 헤더, 법선 없는 OBJ, 다중 프리미티브 glTF 메시

#### V45: 재질 프리셋, Undo 히스토리 드롭다운 & 익스포트 옵션 (2026-04-08)

**프로퍼티 View 탭 재질 프리셋:**
- `crates/viewer/src/gui/properties.rs`: 16개 재질 프리셋 (Steel, Aluminum, Brass, Copper, Gold, Titanium, Cast Iron, Plastic White/Black/Red/Blue, Glass, Wood Light/Dark, Rubber, Carbon Fiber) + 사실적 색상
- 2열 그리드 레이아웃 — 아이콘+이름 버튼 + 색상 스와치 프리뷰
- 현재 재질 색상 근접도로 자동 감지; 활성 재질 파란색 하이라이트
- Glass 프리셋 투명도 포함 (알파 0.35)

**Undo/Redo 히스토리 드롭다운:**
- `crates/viewer/src/gui/toolbar.rs`: Undo/Redo 툴바 버튼 옆에 드롭다운 화살표(▾) 추가
- 드롭다운 클릭 시 최근 10개 히스토리 항목 + 단계 번호 표시
- 항목 클릭 시 여러 단계 한번에 undo/redo
- 히스토리 항목 존재 시 + undo/redo 가능 시에만 표시

**STL 익스포트 옵션 다이얼로그:**
- `crates/viewer/src/gui/dialogs.rs`: Export Options 윈도우 — 포맷(Binary/ASCII 라디오), 스케일 팩터 DragValue
- `crates/viewer/src/gui/mod.rs`: `ExportStlWithOptions` 액션 (binary/scale 파라미터), 익스포트 다이얼로그 상태 필드
- `crates/viewer/src/app.rs`: 핸들러에서 버텍스 스케일링 적용 후 `export_stl_binary`/`export_stl_ascii` 직접 호출
- 메뉴 STL 익스포트 시 옵션 다이얼로그 먼저 표시; OBJ/PLY는 직접 익스포트 유지

#### V44: 브레드크럼 바, 최근 파일 & 토스트 알림 (2026-04-08)

**브레드크럼 네비게이션 바:**
- `crates/viewer/src/gui/overlays.rs`: `draw_breadcrumb_bar()` — 컨텍스트 툴바와 뷰포트 사이에 TopBottomPanel 렌더링
- 경로 세그먼트: Scene › ObjectName › Face/Edge/Vertex/Solid, 스케치 모드 시 Sketch 추가
- 클릭 가능한 "Scene" 루트 세그먼트 → `DeselectAll`로 상위 탐색
- 다중 선택 시 우측에 악센트 블루로 선택 수 표시
- 셰브론 구분자(›), 활성 세그먼트 흰색, 비활성 연회색

**File 메뉴 최근 파일:**
- `crates/viewer/src/gui/menu.rs`: File 메뉴에 "Recent Files" 서브메뉴 추가
- 최근 열린 파일 최대 10개 표시 (파일명 + 전체 경로 툴팁)
- 하단에 "Clear Recent Files" 옵션; `GuiAction::ClearRecentFiles` 추가
- `crates/viewer/src/app.rs`: 파일 열기/임포트 시 `recent_files`에 추가 (중복 제거, 최대 10개)

**토스트 알림 시스템:**
- `crates/viewer/src/gui/mod.rs`: `Toast` 구조체와 `ToastLevel` 열거형 (Success/Info/Warning/Error)
- `crates/viewer/src/gui/overlays.rs`: `draw_toast_overlay()` — 우하단 플로팅 알림 렌더링
- 3초 후 자동 사라짐 + 0.5초 페이드아웃 + 0.15초 페이드인 애니메이션
- 레벨별 스타일링: 컬러 악센트 바, 배경 틴트, 아이콘 (✓/ℹ/⚠/✖)
- 최대 5개 토스트 수직 스택; 텍스트 길면 말줄임표(…) 처리
- `log_info()` → Success 토스트, `log_warning()` → Warning 토스트, `log_error()` → Error 토스트
- `StatusMessage` 액션도 Info 토스트 트리거 — 즉각적 시각 피드백

#### V43: 메뉴 단축키 텍스트 정렬 (2026-04-08)

**메뉴 바 — 네이티브 단축키 텍스트 우측 정렬:**
- `crates/viewer/src/gui/menu.rs`: `menu_action_sc()` 헬퍼 추가 — `egui::Button::new().shortcut_text()`로 우측 정렬 단축키 힌트
- 모든 메뉴 항목을 인라인 형식(`"New  (Ctrl+N)"`)에서 egui 네이티브 `shortcut_text()` API로 변환
- File 메뉴: New, Open, Save As, Quit; Edit 메뉴: Undo, Redo, Copy, Paste, Select All, Deselect All, Delete
- View 메뉴: 투영 토글, Standard Views, Grid, Fit All, Section Plane
- Sketch 메뉴: 모든 지오메트리 도구 (Select/Line/Rectangle/Circle/Arc/Ellipse/Polyline/B-Spline/Polygon), 구속조건 (Horizontal/Vertical/Fixed), 토글 (Construction Mode/Grid/Snap), Close/Cancel
- PartDesign 메뉴: Pad 단축키

#### V42: 컨텍스트 메뉴 아이콘, 웰컴 스크린 & 디스플레이 모드 셀렉터 (2026-04-08)

**컨텍스트 메뉴 아이콘 & 단축키 힌트:**
- `crates/viewer/src/gui/context_menu.rs`: `menu_item()` 헬퍼 추가 — 버튼 텍스트에 이모지 아이콘 접두사 + 단축키 힌트 접미사
- 오브젝트 컨텍스트 메뉴: Select(📌), Duplicate(⎘/Ctrl+D), Rename(✏/F2), Hide/Show(👁/H), Measure(📏), Check(✔), Delete(🗑/Del)
- 뷰포트 컨텍스트 메뉴: Fit All(🔍/V), Reset Camera(🏠), Grid(▦/G), Projection(▣/5), Origin Axes(✥), 3D Grid(▦), Measurement(📏), Select All(☐/Ctrl+A), Deselect(☒/Esc)

**빈 씬 웰컴 스크린:**
- `crates/viewer/src/gui/overlays.rs`: `draw_welcome_screen()` — 씬 비어있을 때 중앙 오버레이 렌더링. CADKernel 타이틀/서브타이틀, 3개 퀵 액션 버튼(Create Box, Import File, Open Project) + 호버 하이라이트 + 커서 아이콘
- 페인터 기반 렌더링 + 수동 히트 테스트(pointer_pos + click 감지)
- 하단에 F1 단축키 힌트; 스케치 모드/활성 태스크 없을 때만 표시

**디스플레이 모드 툴바 셀렉터:**
- `crates/viewer/src/gui/toolbar.rs`: View 섹션에 ComboBox 드롭다운 추가 — 현재 디스플레이 모드 + 8개 모드 단축키 표시
- `crates/viewer/src/gui/mod.rs`: `tb_display_mode: DisplayMode` 필드 추가, 매 프레임 ViewportInfo에서 미러링

#### V41: 상태 바, 기즈모 툴바 & 투영 토글 (2026-04-08)

**상태 바 — 세분화된 레이아웃 + 클릭 가능한 투영:**
- `crates/viewer/src/gui/status_bar.rs`: 우측 섹션을 개별 스타일 세그먼트로 분리 (투영 | 디스플레이 모드 | 씬 통계 | 선택 | 측정 | FPS) + `vert_divider()` 구분선
- 투영 인디케이터 (`Persp`/`Ortho`) 클릭으로 원근/직교 투영 토글 — `GuiAction::ToggleProjection`; 색상 코드: 파랑=원근, 녹색=직교
- 씬 통계 `vis/total obj` 포맷 + K/M 삼각형 수 포맷팅
- 선택 정보 악센트 블루 표시; 측정 모드 인디케이터 노란색

**툴바 — 트랜스폼 기즈모 버튼:**
- `crates/viewer/src/gui/toolbar.rs`: "Transform" 섹션 추가 — Move(W), Rotate(E), Scale(R) 기즈모 토글 버튼 + `icon_toggle` 활성 상태 하이라이트
- `crates/viewer/src/gui/mod.rs`: `GuiAction::SetGizmoMode(GizmoMode)` 액션 추가
- `crates/viewer/src/app.rs`: `SetGizmoMode` 핸들러 — 같은 모드 클릭 시 해제, 다른 모드 클릭 시 전환

#### V40: 다이얼로그 일관성, 키보드 단축키 & 트리 다듬기 (2026-04-08)

**다이얼로그 그리드 — 2열 레이아웃 + DragValue 접미사:**
- `crates/viewer/src/gui/dialogs.rs`: 전체 32개 다이얼로그 그리드를 3열(Label | DragValue | "mm")에서 2열(Label | DragValue `.suffix(" mm")`)로 변환, V39 프로퍼티 패널 패턴과 일치
- 그리드 간격 `[8.0, 4.0]` → `[10.0, 4.0]`; 모든 수동 `ui.label("")` 3열 잔재 제거
- 샤프트 세그먼트 행 `.suffix(" mm")` 적용 (L=, D= 프리픽스 DragValue)
- 모든 레이블 색상 `theme::COLOR_DIM` 일관 적용

**키보드 단축키 참조 (F1):**
- `crates/viewer/src/app.rs`: F1 키로 `show_shortcuts` 패널 토글
- `crates/viewer/src/gui/dialogs.rs`: 단축키 다이얼로그 개선 — 9개 카테고리 섹션(File, Edit, Navigation, Standard Views, Display Modes, Transform Gizmo, Selection Modes, Sketcher, General) + `dialog_section()` 악센트 헤더 + 아이콘
- 스케처 키바인딩 추가 (S/L/R/C/A/E/P/B/W/H/V/Enter/Escape)
- 키 이름 모노스페이스 볼드, 설명 연한 색상

**모델 트리 — 전체 펼치기/접기 & 필터 다듬기:**
- `crates/viewer/src/gui/tree.rs`: 검색 상자와 트리 내용 사이에 미니 툴바 추가 — 전체 펼치기(▿), 전체 접기(▹) 버튼
- 버튼 클릭 시 모든 최상위/하위 트리 노드의 `("tree_expand", obj_id)` 상태 토글
- 필터 활성 시 미니 툴바 좌측에 결과 수 표시

#### V39: 프로퍼티, 리포트, 히스토리 & 스케치 폴리시 (2026-04-08)

**프로퍼티 패널 — 개선된 편집:**
- `crates/viewer/src/gui/properties.rs`: 파라미터 에디터 그리드 간격 `[4.0, 2.0]` → `[10.0, 4.0]`; DragValue `.suffix(" mm")` 사용하여 별도 단위 레이블 열 제거, 2열 레이아웃
- 씬 개요: 삼각형/정점 수 K/M 포맷 추가, 아이콘 헤더, 개선된 빈 상태 표시

**리포트 패널 — 강화된 로그 표시:**
- `crates/viewer/src/gui/report.rs`: 로그 항목에 레벨 아이콘(ℹ/⚠/✖) + 모노스페이스 타임스탬프 열; 경고/오류 행에 미세한 틴트 배경

**히스토리 패널 — 작업 아이콘:**
- `crates/viewer/src/gui/report.rs`: 히스토리 항목에 컨텍스트 인식 아이콘(➕ 생성, ➖ 삭제, → 이동, ↻ 회전, ⤢ 크기, ∪ 불리언, ⬆ 돌출); 모노스페이스 번호 정렬; 초록 화살표 현재 상태 마커

**스케치 UI — 추가 폴리시:**
- `crates/viewer/src/gui/sketch_ui.rs`: 치수 입력 팝업 재설계 — 아이콘 제목, DragValue 접미사(mm/°), 태스크 패널 스타일 OK/Cancel 버튼
- 스케치 컨텍스트 메뉴: 섹션 헤더(Edit, Constraints, Selection) + 메뉴 항목 아이콘

#### V38: FreeCAD 스타일 스케치 UI, 태스크 패널 & 테마 확장 (2026-04-08)

**태스크 패널 — FreeCAD TaskView 오버홀:**
- `crates/viewer/src/gui/task_panel.rs`: 모든 태스크 헤더를 `theme::draw_task_header()`로 교체 — 악센트 그라디언트 바 + 아이콘 + 제목
- `crates/viewer/src/gui/task_panel.rs`: 그리드 섹션에 `theme::draw_task_section()` 사용 — 악센트 밑줄 레이블
- `crates/viewer/src/gui/task_panel.rs`: OK/Cancel 버튼에 `theme::draw_task_buttons()` 사용 — 악센트 블루 기본 버튼 + 흰색 텍스트
- DRY 매크로: `plabel!`, `pmm!`, `pdeg!`, `pval!` — DragValue 접미사(" mm", "°") 지원 그리드 파라미터 렌더링
- 모든 그리드 간격 `[10.0, 4.0]`으로 확대; BooleanOp을 Operation/Tool Shape/Offset 섹션으로 분리

**메뉴 바 — 섹션 헤더:**
- `crates/viewer/src/gui/menu.rs`: `menu_section()` 헬퍼 추가, `theme::MENU_SECTION_COLOR` 사용
- File 메뉴: "Project"/"Transfer"; Edit: "History"; View: "Layout"/"Overlays"/"Camera"
- Create: "Basic Primitives"/"Extended Primitives"; Tools: "Analysis"/"Measurement"
- Sketch > Constraints: "Geometric"/"Dimensional" 섹션 레이블

**스케치 UI — FreeCAD 스타일 시각 오버홀:**
- `crates/viewer/src/gui/sketch_ui.rs`: 엔티티 색상을 FreeCAD 팔레트로 변경 — 흰색 지오메트리, 파란색 보조선, 녹색 선택, 밝은 녹색 호버, 금색 대기, ��간색 구속
- 커서 십자선: 갭 센터 스타일 (내부/외부 세그먼트 + 4px 갭)
- 스냅 인디케이터: 일치점 = 채운 점 + 링, H/V = 빨간 점선 가이드라인 + 방향 배지, 중점 = 채운 다이아몬드, 그리드 = 미세 십자, 교차점 = X + 원
- 배너: 둥근 배경 필 + 색상 코드(빨강=충돌, 녹색=완전 구속, 진한 파랑=기본), 불릿 구분자
- DOF 화살표: 주황색(기존 녹색), 14px 길이
- 과잉 구속 경고: 배경 필 오버레이
- 보조선 점: 파란 X 마커(기존 링)
- ��스 선택: 더 은은한 파랑/녹색 채우기
- OVP 패널: 도구 아이콘 + 이름 헤더, 어두운 배경, 좁은 여백
- 미리보기/러버밴드 색상: 금색-노랑 (255, 200, 50)

**테마 시스템 확장:**
- `crates/viewer/src/gui/theme.rs`: 스케치 오버레이 색상 상수 추가 — `SKETCH_GEOMETRY`, `SKETCH_CONSTRUCTION`, `SKETCH_SELECTED`, `SKETCH_HOVERED`, `SKETCH_PENDING`, `SKETCH_CONSTRAINT`, `SKETCH_VIOLATED`, `SKETCH_DOF`
- `crates/viewer/src/gui/theme.rs`: `MENU_SECTION_COLOR` 상수 추가 — 통합 메뉴/툴바/컨텍스트 메뉴 섹션 레이블
- 툴바, 메뉴, 컨텍스트 메뉴 섹션 헬퍼가 `theme::MENU_SECTION_COLOR` 사용

**레이아웃 & 간격:**
- ComboView 패널: 기본 너비 300px (기존 280), 최소 너비 220px (기존 200)
- 트리/속성 분할: 45%/55% (기존 50/50) — 태스크 패널과 속성에 더 많은 공간

#### V37: 툴바, 다이얼로그 & 컨텍스트 메뉴 다듬기 (2026-04-08)

**툴바 — 섹션 레이블 & 개선된 구분선:**
- `crates/viewer/src/gui/toolbar.rs`: `section_label()` 헬퍼 추가 — 9px 연회색 레이블로 모든 9개 워크벤치 컨텍스트 툴바와 메인 툴바(File/Edit/View/Scene/Select)의 각 도구 그룹 앞에 표시
- `crates/viewer/src/gui/toolbar.rs`: `toolbar_separator()` 개선 — 그라디언트 페이드 효과(투명→회색→투명) + 밝은 중심 구간, 기존 1px 단색 선 대체
- Part 툴바 섹션: Primitives, Boolean, Transform, Join, Compound, Convert, Analysis
- PartDesign 툴바 섹션: Features, Additive, Dress-up, Transform, Extras, Body, Boolean
- Sketcher 툴바 섹션: Geometry, Constraints, Tools, B-Spline, Options
- Mesh 툴바 섹션: Import/Export, Repair
- TechDraw 툴바 섹션: Page, Views, Dimensions, Annotations, Centerlines, Export
- Assembly 툴바 섹션: Assembly, Joints
- Draft 툴바 섹션: Draw, Modify, Array, Annotation, Convert, Snap
- Surface 툴바 섹션: Surface
- FEM 툴바 섹션: Setup, Mesh, Constraints, Solve, Results

**다이얼로그 — FreeCAD 스타일 섹션 헤더 & 간격:**
- `crates/viewer/src/gui/dialogs.rs`: `dialog_section()` 재작성 — 파란 배경 틴트 + 3px 좌측 악센트 바 + 악센트 색상 레이블
- `crates/viewer/src/gui/dialogs.rs`: `button_bar()` 재작성 — 악센트 블루 채움 + 흰색 텍스트 기본 버튼, 최소 크기(OK 70px, Cancel 60px), 커스텀 구분선
- 모든 31개 다이얼로그 그리드 간격 `[4.0, 2.0]` → `[8.0, 4.0]` 확대

**컨텍스트 메뉴 — 섹션 헤더:**
- `crates/viewer/src/gui/context_menu.rs`: `menu_section()` 헬퍼 추가 — 10px 연회색 굵은 레이블로 시각적 그루핑
- 오브젝트 컨텍스트 메뉴 섹션: Selection, Edit, Appearance, Analysis
- 뷰포트 컨텍스트 메뉴 섹션: View, Display, Overlays, Selection, Create

#### V36: FreeCAD 스타일 UI 오버홀 (2026-04-08)

**테마 시스템 — 패널 크롬 & 섹션 헤더:**
- `crates/viewer/src/gui/theme.rs`: `panel_header_bg/text`, `panel_separator`, `section_header_bg/text` 색상 추가; `panel_header_height`, `section_header_height`, `tree_row_height` 사이징; 밀도별 스케일링 (Compact 20px / Normal 24px / Spacious 28px)
- `draw_panel_header()` — FreeCAD 스타일 독 헤더 바 (어두운 배경, 제목, 닫기 버튼); `draw_section_header()` — 접기 가능한 섹션 바; `draw_separator()` — 얇은 구분선

**패널 레이아웃 — FreeCAD 스타일 ComboView:**
- `crates/viewer/src/gui/mod.rs`: 좌측 패널에 타이틀 헤더 바 ("Model" / "Properties" / "Tasks"), 각각 닫기 가능; 제로 내부 마진 + 콘텐츠 레벨 패딩; 트리와 프로퍼티 사이 구분선

**모델 트리 — FreeCAD 스타일 계층 구조:**
- `crates/viewer/src/gui/tree.rs`: 문서 루트 노드 ("CADKernel" + 파일 아이콘 + 오브젝트 수); 루트 아래 인덴트; 선택 시 좌측 액센트 바 (2px 파란색); 눈 아이콘 호버/숨김 시만 표시; 색상 스와치 외곽선; 그룹을 접기 가능 섹션으로; 커스텀 검색 박스; 빈 씬 도움말 텍스트

**프로퍼티 패널 — 섹션 헤더 & 간격:**
- `crates/viewer/src/gui/properties.rs`: Data/View 탭에 언더라인 스타일 활성 표시기; 접기 그룹을 `draw_section_header()`로 교체; 그리드 간격 확대 (10px 수평, 4px 수직); 타입 아이콘 배지가 있는 오브젝트 이름 헤더

**리포트 패널 — 언더라인 탭 바:**
- `crates/viewer/src/gui/report.rs`: 커스텀 탭 바 렌더링 (26px 높이, 개별 탭 호버 하이라이트, 액센트 블루 활성 언더라인); 우측 심각도 요약 카운트

**상태 바 — 수직 구분선 & 폴리싱:**
- `crates/viewer/src/gui/status_bar.rs`: `vert_divider()` — 얇은 0.5px 수직선; 어두운 배경; 상단 액센트 라인; 좌표에 "mm" 단위 포함

#### V35: 뷰어 사용성 & 인터랙션 (2026-04-08)

**뷰어 — 카메라 뷰 북마크 (폴리싱):**
- `crates/viewer/src/nav.rs`: `ViewBookmark` 구조체 (이름, yaw, pitch, roll, distance, target); NavConfig에 `view_bookmarks` 벡터; 최대 20개 제한
- `crates/viewer/src/gui/menu.rs`: View > Bookmarks 서브메뉴 — 220px 너비, 힌트 텍스트 입력, 활성/비활성 저장 버튼, 카메라 각도 툴팁이 있는 번호 매긴 항목, 우측 정렬 삭제 버튼, "X/20 bookmarks" 카운트, 빈 상태 메시지
- `crates/viewer/src/app.rs`: `.cloned()`로 빌림 충돌 방지 후 저장 뷰로 애니메이션

**뷰어 — 프리셀렉션 하이라이트 (폴리싱):**
- `crates/viewer/src/gui/overlays.rs`: `draw_selection_overlay()` — 선택 없이도 프리셀렉션 표시; `nav.preselection_color`/`nav.selection_color`로 색상 설정 가능; 커서 따라다니는 엔티티 타입 라벨; 엣지 8px 글로우 + 3.5px 코어; 버텍스 10px 글로우 링 + 6px 마커 + 2px 중심점
- `crates/viewer/src/nav.rs`: NavConfig에 `preselection_color`, `selection_color` ([u8; 3]) 설정 가능 필드

**뷰어 — 오브젝트 그룹핑 (폴리싱):**
- `crates/viewer/src/scene.rs`: `ObjectGroup` 구조체; Scene 메서드: `create_group()`, `group_selected()`, `ungroup_object()`, `toggle_group_visibility()`, `delete_group()`, `group_members()`
- `crates/viewer/src/gui/menu.rs`: Edit > Groups 서브메뉴 — 눈 아이콘 (◉/○) 색상 코딩, 멤버 수 라벨 "(N)", 힌트 텍스트 입력, 빈 상태 처리
- `crates/viewer/src/gui/tree.rs`: 모델 트리에 그룹 섹션 — 헤더에 눈 토글, 폴더 아이콘, 이름, 카운트, 삭제 버튼; 인덴트된 멤버 이름

**뷰어 — 3D 측정 오버레이 (폴리싱):**
- `crates/viewer/src/gui/overlays.rs`: `draw_measurement_overlay()` — `nav.unit_system.label()`/`nav.decimal_places`로 단위 인식 표시; 번호 매긴 포인트 마커 (P1, P2...); 좌표 표시; 연속 쌍 간 거리; 3점 이상 총 경로 길이; ΔX/ΔY/ΔZ 성분 분해; 각도 호 + 도 표시; 컨텍스트 모드 인디케이터; 라벨 배경에 둥근 사각형 + 외곽선
- `crates/viewer/src/app.rs`: `pick_surface_point()` — 레이 캐스트, 버텍스 스냅 우선 (임계값 `camera.distance * 0.012`), 삼각형 표면 폴백; C로 클리어, Escape로 종료

**뷰어 — 좌표축 인디케이터 (폴리싱):**
- `crates/viewer/src/gui/overlays.rs`: `draw_axes_overlay()` — 클릭으로 표준 뷰 스냅 (X+→Right, X−→Left, Y+→Front, Y−→Back, Z+→Top, Z−→Bottom); 호버 시 링 하이라이트 + 커서 변경; 깊이 기반 투명도 페이드; 글로우 라인으로 안티앨리어싱; 라벨 텍스트 그림자; 그라데이션 배경 링; 스페큘러 하이라이트 중심 구체
- `crates/viewer/src/render.rs`: `Camera::forward()` 메서드

#### V34: 뷰어 프로덕션 품질 (2026-04-07)

**뷰어 — ViewCube 드래그 회전:**
- `crates/viewer/src/gui/view_cube.rs`: ViewCube 면/엣지/코너를 드래그하면 `ScreenOrbit`으로 카메라 연속 궤도 회전; 움직임 없는 클릭은 표준 뷰로 스냅 유지; GuiState에 `cube_dragging`/`cube_drag_moved` 상태
- `crates/viewer/src/gui/mod.rs`: GuiState에 `cube_dragging`, `cube_drag_moved` 필드 추가

**뷰어 — 3D 그리드 스냅:**
- `crates/viewer/src/nav.rs`: NavConfig에 `snap_to_grid_3d` 토글; `snap_3d()` 헬퍼가 활성화 시 가장 가까운 그리드 간격으로 반올림
- `crates/viewer/src/gui/dialogs.rs`: Display 설정 — "Snap to 3D grid" 체크박스
- `crates/viewer/src/app.rs`: `MoveObject` 액션이 스냅 활성화 시 dx/dy/dz에 `snap_3d()` 적용

**뷰어 — 인터랙티브 변환 기즈모:**
- `crates/viewer/src/gui/overlays.rs`: `draw_transform_gizmo()`가 `&mut GuiState` 수용; 호버 감지 (커서-축 선분 거리 `point_to_segment_dist()` 사용); 드래그 시 마우스 델타를 축 방향에 투영하여 `MoveObject`/`RotateObject`/`ScaleObjectUniform` 액션 발생
- `crates/viewer/src/app.rs`: W/E/R 키보드 단축키로 이동/회전/스케일 기즈모 모드 전환 (스케치 모드가 아닐 때만)

**뷰어 — 실행 취소/다시 실행 기록 패널:**
- `crates/viewer/src/gui/report.rs`: 하단 패널에 "History" 탭 추가; 번호 매긴 작업 목록 + 현재 위치 마커 (초록 화살표); 흐릿한 redo 항목; Undo/Redo 버튼
- `crates/viewer/src/gui/mod.rs`: GuiState에 `history_entries`/`future_entries` 필드, 매 프레임 `CommandStack::entries()`에서 채움
- `crates/viewer/src/command.rs`: UI 표시용 `entries()` 메서드 추가 (history, future 설명 목록 반환)
- `crates/viewer/src/app.rs`: draw_ui 전에 `tb_can_undo`/`tb_can_redo` 및 history 항목 채움

**뷰어 — 환경 설정에 단축키 탭:**
- `crates/viewer/src/gui/dialogs.rs`: 환경 설정에 6번째 "Shortcuts" 탭 추가하여 모든 키보드 단축키 표시; `draw_all_shortcuts()` 함수를 탭과 독립 다이얼로그에서 공유; 새 단축키 추가 (Shift+S 단면, W/E/R 기즈모, Ctrl+Shift+Z redo)

#### V33: 3D 모델링 & 뷰포트 향상 (2026-04-07)

**뷰어 — FlatLines 디스플레이 모드 조정:**
- `crates/viewer/src/render.rs`: `EDGE_OVERLAY_COLOR`를 `[0.05, 0.05, 0.05, 1.0]`에서 `[0.08, 0.08, 0.10, 1.0]`으로 조정하여 음영 표면 위 와이어프레임 오버레이를 더 자연스럽게 개선

**뷰어 — 미구속 점에 DOF 화살표:**
- `crates/viewer/src/gui/sketch_ui.rs`: 미구속 스케치 점에 방향별 DOF 화살표 표시 — 모든 제약조건 스캔하여 점별 X/Y 구속 상태 판단; 미구속 축에 빨간 화살표, 완전 구속 점에 초록 체크마크 표시

**뷰어 — 단면 평면 토글:**
- `crates/viewer/src/gui/mod.rs`: `GuiAction` 열거형에 `ToggleSectionPlane` 액션 추가
- `crates/viewer/src/app.rs`: Shift+S 단축키로 단면 평면 토글; 액션 핸들러가 `nav.clip_enabled` 전환
- `crates/viewer/src/gui/menu.rs`: View 메뉴 "Section Plane (Shift+S)" 항목
- `crates/viewer/src/gui/dialogs.rs`: Display 설정 탭 — 단면 평면 제어 (활성화 체크박스, 축 선택기 X/Y/Z, 오프셋 DragValue)
- `crates/viewer/src/nav.rs`: 기존 `clip_enabled`, `clip_plane_normal`, `clip_plane_offset`이 UI에 연결

**뷰어 — 커스텀 배경 그라디언트:**
- `crates/viewer/src/nav.rs`: `BgPreset::Custom` 변형 추가, NavConfig에 `bg_custom_top`/`bg_custom_bottom` 색상 필드
- `crates/viewer/src/render.rs`: `gradient_colors()`가 Custom 포함 모든 프리셋의 상단/하단 색상 추출; `gradient_shader_src_colors()`가 명시적 색상으로 셰이더 생성; `update_bg()`가 색상 변경 시만 파이프라인 재구축 (`bg_colors` 비교)
- `crates/viewer/src/gui/dialogs.rs`: Display 설정 — Custom 프리셋 선택 시 상단/하단 색상 선택기 표시
- `crates/viewer/src/app.rs`: `render_frame()`이 매 프레임 `gpu.update_bg()` 호출하여 설정 변경 동기화

**뷰어 — 오브젝트 불투명도 슬라이더:**
- `crates/viewer/src/gui/properties.rs`: Properties 패널 View 탭에 투명도 슬라이더 (0–90%) 이미 존재; `SetObjectColor` 액션으로 오브젝트별 알파 조정, 투명 파이프라인으로 렌더링

#### V32: 스케치 인터랙션 & 고급 스냅 (2026-04-07)

**뷰어 — 박스 선택 (러버밴드) 스케치:**
- `crates/viewer/src/app.rs`: Select 도구에서 엔티티 없이 드래그 시 박스 선택 시작; `box_select_start`/`box_select_end` 필드로 러버밴드 사각형 추적
- `crates/viewer/src/app.rs`: 릴리스 시 박스 내부 엔티티 선택 — 좌→우 (윈도우)는 모든 끝점 포함 필요; 우→좌 (크로싱)는 아무 끝점 포함으로 충분; Ctrl로 선택에 추가
- `crates/viewer/src/gui/sketch_ui.rs`: 반투명 파랑 (윈도우) 또는 초록 (크로싱) 사각형 + 실선/파선 테두리

**뷰어 — 스냅 시각 표시기:**
- `crates/viewer/src/gui/sketch_ui.rs`: 그리기 중 커서 근처 캔버스 스냅 표시 — X 마커 (일치/점), 파선 H/V 가이드라인 (축 정렬), 삼각형 (중점), 사각형 (그리드), 원형 X (교차점)

**뷰어 — 더블클릭 제약조건 편집:**
- `crates/viewer/src/app.rs`: `try_sketch_dimension_edit()` — 치수 제약조건이 있는 선택 엔티티에 더블클릭 시 현재 값으로 팝업 열기; Distance, Length, Radius, Diameter, Angle, H/V-Distance 지원
- `crates/viewer/src/gui/sketch_ui.rs`: `DimensionPopup`의 `edit_constraint_index` — 확인 시 중복 추가 대신 기존 제약조건 값을 직접 수정 (undo 스냅샷 포함)

**뷰어 — 스케치 커서 형상:**
- `crates/viewer/src/gui/sketch_ui.rs`: 모든 그리기 도구에 십자 커서, Select 모드에서 엔티티 호버 시 포인팅 핸드, 점 드래그 중 그래빙

**뷰어 — 중점 & 교차점 스냅:**
- `crates/viewer/src/app.rs`: `snap_sketch_coords()`에 중점 스냅 (선 중점) 및 교차점 스냅 (선-선 교차 매개변수 t/u 테스트) 추가
- `crates/viewer/src/gui/sketch_ui.rs`: `detect_auto_constraints()`에 `Intersection` 종류 추가; 교차점 표시기는 주황색 원형 X 마커

#### V31: 스케치→솔리드 파이프라인 & 치수 UX (2026-04-07)

**뷰어 — 치수 입력 팝업:**
- `crates/viewer/src/gui/sketch_ui.rs`: `draw_dimension_popup()` — Distance/Radius/Angle/Length/H-Dist/V-Dist/Diameter 제약조건 버튼 클릭 시 중앙 입력 팝업 표시; Enter/OK로 확인, Escape로 취소; 값이 툴바 기본값에 저장
- `crates/viewer/src/gui/mod.rs`: `DimensionPopup` 구조체 + `DimensionKind` 열거형 (7종) 추가
- `crates/viewer/src/gui/toolbar.rs`: 7개 치수 제약조건 버튼이 직접 적용 대신 팝업 열기로 변경; 인라인 DragValue 필드 제거

**뷰어 — 닫힌 프로파일 감지 & 하이라이트:**
- `crates/viewer/src/gui/sketch_ui.rs`: `find_closed_loops()` — 선 인접성 탐색으로 닫힌 루프 감지; 닫힌 프로파일을 반투명 초록 채움 (rgba 80,200,120,30)으로 렌더링하여 압출 가능 영역 표시

**뷰어 — 압출 방향 미리보기 화살표:**
- `crates/viewer/src/gui/sketch_ui.rs`: 닫힌 프로파일 존재 시 스케치 중심에서 작업 평면 법선 방향으로 초록 화살표 표시; 화살촉 삼각형 + 거리 라벨 ("10.0 mm")

**뷰어 — 스케치 평면 축 레이블:**
- `crates/viewer/src/gui/sketch_ui.rs`: 스케치 원점에 색상 X (빨강) / Y (초록) 축 화살표 + 텍스트 레이블; 좌표계 시각화를 위한 흰색 원점 표시

#### V30: 스케치 시각 개선 & 슬롯 도구 (2026-04-07)

**뷰어 — 스케치 엔티티 호버 정보:**
- `crates/viewer/src/gui/status_bar.rs`: `sketch_hover_info()`로 호버된 스케치 엔티티 속성을 상태바에 표시 — Point(x,y), Line(길이, 각도), Circle(중심, 반지름), Arc(중심, 반지름, 범위), Ellipse(중심, 단축 반지름), B-Spline(차수, 제어점 수)

**뷰어 — 슬롯 도구 개선:**
- `crates/viewer/src/app.rs`: 슬롯 도구가 단순 선에서 실제 스타디움 형상으로 업그레이드 — 3클릭 흐름 (중심1, 중심2, 너비 점)으로 2개 평행선 + 2개 반원호 생성하여 닫힌 슬롯/직사각원 형성
- `crates/viewer/src/gui/status_bar.rs`: 슬롯 도구 힌트를 "Click center 1, center 2, then width"로 업데이트

**뷰어 — 부드러운 B-Spline 렌더링:**
- `crates/viewer/src/gui/sketch_ui.rs`: B-스플라인 곡선이 제어점 직선 연결 대신 de Boor 알고리즘으로 부드러운 곡선 렌더링; `de_boor_eval()` + `clamped_uniform_knots()` 헬퍼 함수; 곡선당 4N+16 샘플 포인트로 시각적 부드러움; 제어 다각형은 여전히 파선 + 다이아몬드 마커로 표시

**뷰어 — 스케치 툴바 버튼:**
- `crates/viewer/src/gui/toolbar.rs`: 카본 카피 버튼 뒤에 Copy, Paste, Merge Pts 버튼 추가

#### V29: 스케치 도구 완성 & 유효성 검증 (2026-04-07)

**뷰어 — B-Spline 도구 연결:**
- `crates/viewer/src/app.rs`: `SketchConvertToBSpline` → `geometry_to_bspline()`으로 선택된 선/호/원을 B-스플라인 변환; `SketchIncreaseDegree`/`SketchDecreaseDegree` → 선택된 B-스플라인 차수 증가/감소; `SketchInsertKnot` → t=0.5에 매듭 삽입

**뷰어 — 외부 투영 & 카본 카피:**
- `crates/viewer/src/app.rs`: `SketchExternalProjection` → `external_projection()`으로 모든 모델 꼭짓점을 스케치 평면에 투영; `SketchCarbonCopy` → `carbon_copy()`로 마지막 닫은 스케치를 현재 스케치에 복사

**뷰어 — 스케치 복사/붙여넣기:**
- `crates/viewer/src/gui/mod.rs`: `SketchMode`에 `clipboard_points` + `clipboard_lines` — 중심 기준 상대 좌표 저장
- `crates/viewer/src/app.rs`: `SketchCopySelection`으로 선택 엔티티 복사, `SketchPasteSelection(x, y)`으로 대상 위치에 재생성; 스케치 모드에서 Ctrl+C/Ctrl+V 단축키

**뷰어 — 점 병합:**
- `crates/viewer/src/app.rs`: `SketchMergePoints` — 일치 점(엡실론 0.01) 검색, 모든 엔티티 참조(선, 호, 원, 타원, B-스플라인) 병합 인덱스로 재매핑

**뷰어 — 스케치 유효성 검증 오버레이:**
- `crates/viewer/src/gui/mod.rs`: `SketchMode`에 `validation_issues` 필드, 매 프레임 `validate_sketch()` 호출
- `crates/viewer/src/gui/sketch_ui.rs`: 길이 0 선 근처 경고 아이콘, 근접 일치 점 "merge?" 표시, 과잉 구속 배너 경고

**뷰어 — 스케치 스텁 제로:**
- 6개 스케치 액션 스텁 모두 실제 구현으로 교체 (ConvertToBSpline, IncreaseDegree, DecreaseDegree, InsertKnot, ExternalProjection, CarbonCopy)

#### V28: 스케치 보조선 모드 & 자동 제약조건 (2026-04-07)

**뷰어 — 보조선 지오메트리 토글:**
- `crates/viewer/src/app.rs`: `ToggleSketchConstruction`이 선택된 엔티티(점/선)를 보조선/일반 모드 간 전환; 선택 없으면 새 엔티티의 전역 보조선 모드 토글
- `crates/viewer/src/app.rs`: 선, 사각형, 점 도구가 `construction_mode` ON일 때 새 지오메트리를 자동으로 보조선으로 표시

**뷰어 — 스케치 대칭 지오메트리:**
- `crates/viewer/src/app.rs`: `SketchMirrorGeometry`를 `sketch.mirror_elements()`에 연결 — 1개 선을 대칭축으로 선택 + 선택적 점 대칭; 점 미선택 시 축 외 모든 점 대칭

**뷰어 — 폴리라인 닫기:**
- `crates/viewer/src/app.rs`: Enter 키로 폴리라인 루프 닫기 (3개 이상 점 → 마지막에서 첫 번째로 선 추가); 우클릭도 폴리라인 닫기 (3개 이상 점)
- `crates/viewer/src/app.rs`: B-Spline 모드에서 Enter로 축적된 제어점으로 B-스플라인 확정

**뷰어 — 제약조건 색상 코딩 완성:**
- `crates/viewer/src/gui/sketch_ui.rs`: 모든 제약조건 렌더링 함수 (Radius, Diameter, Angle, H/V-Distance, Perpendicular, Tangent, Midpoint, Collinear, Concentric, Symmetric)가 고정 색상 대신 제약조건별 잔차 색상 (초록/노랑/빨강) 사용
- 미사용 `DIM_COLOR` 상수 제거

**뷰어 — 자동 제약조건 적용:**
- `crates/viewer/src/app.rs`: `find_or_create_point()` — 기존 근접 점 재사용 (스냅 거리 0.3)으로 중복 생성 방지, 암시적 일치 제약조건 구현
- `crates/viewer/src/app.rs`: `apply_line_auto_constraints()` — 선 각도가 축과 5° 이내일 때 자동으로 수평/수직 제약조건 추가
- 선, 폴리라인, 사각형 도구가 자동 제약조건 사용; 사각형은 4변 모두에 H/V 제약조건 자동 추가

#### V27: 스케치 정밀 편집 (2026-04-06)

**뷰어 — 제약조건 인식 드래그:**
- `crates/viewer/src/app.rs`: 단일 점 드래그 시 `drag_solve()`를 사용하여 기존 제약조건 유지 — 솔버 미수렴 시 원시 이동으로 폴백; 다중 점 엔티티 드래그는 여전히 델타 기반 이동

**뷰어 — 완전한 스케치 Undo/Redo:**
- `crates/viewer/src/gui/mod.rs`: `SketchSnapshot`이 엔티티 수 대신 전체 `Sketch` 클론 저장 — undo와 redo 모두 삭제, 점 이동, 제약조건 변경 포함 완전한 스케치 상태 복원
- `crates/viewer/src/gui/mod.rs`: `redo()` 완전 작동 — `redo_stack`에서 스케치 복원, 선택 초기화

**뷰어 — 인터랙티브 Trim/Split/Extend:**
- `crates/viewer/src/app.rs`: `SketchTrimEdge` — 2개 선 선택 후 교차점에서 첫 번째 선 트리밍 (시작점 쪽 유지); `SketchSplitEdge` — 1개 선 선택 후 중점에서 분할 (t=0.5); `SketchExtendEdge` — 1개 선 선택 후 끝점을 현재 길이의 50% 연장
- `crates/viewer/src/app.rs`: `SketchFilletCorner` / `SketchChamferCorner` — 꼭짓점을 공유하는 2개 선 선택 후 설정된 반경/거리로 필릿 호 또는 챔퍼 선 적용

**뷰어 — Ctrl+A 스케치 전체 선택:**
- `crates/viewer/src/app.rs`: 스케치 모드에서 Ctrl+A로 모든 엔티티 선택 (점, 선, 호, 원, 타원, B-스플라인); 스케치 모드 밖에서는 글로벌 SelectAll로 전달

**뷰어 — 설정 가능한 그리드 간격:**
- `crates/viewer/src/gui/mod.rs`: `SketchMode`에 `grid_spacing: f64` 필드 (기본값 1.0)
- `crates/viewer/src/gui/toolbar.rs`: 툴바 토글 섹션에 DragValue 입력 (G: 접두사, 0.1–10.0 범위)
- `crates/viewer/src/gui/sketch_ui.rs`: 그리드 렌더링이 설정된 간격 사용; 자동 제약조건 그리드 스냅도 간격 반영
- `crates/viewer/src/app.rs`: `snap_sketch_coords()`가 하드코딩된 0.5 대신 설정된 그리드 간격으로 스냅

#### V26: 스케치 고급 편집 (2026-04-06)

**뷰어 — 엔티티 드래그:**
- `crates/viewer/src/app.rs`: 선/원/호 클릭+드래그 시 모든 구성 점을 한 단위로 이동 — `entity_drag_points()`가 엔티티 타입별 점 인덱스 수집 (선→2개 끝점, 원/호→중심, 타원→중심+장축 끝점); `drag_origin` 추적을 통한 델타 기반 이동
- `crates/viewer/src/gui/mod.rs`: `drag_point: Option<usize>`를 `drag_points: Vec<usize>` + `drag_origin: Option<(f64, f64)>`로 교체하여 다중 점 드래그 지원

**뷰어 — 제약조건 솔버 피드백:**
- `crates/sketch/src/solver.rs`: `constraint_residuals()` — 전체 솔버 실행 없이 제약조건별 L2 잔차 노름 계산
- `crates/viewer/src/gui/mod.rs`: `update_constraint_status()` — 프레임당 `constraint_residuals()` 호출, `constraint_residuals: Vec<f64>`와 `solver_converged: bool`에 결과 저장
- `crates/viewer/src/gui/sketch_ui.rs`: 제약조건 표시기 색상 코딩: 초록=만족 (<1e-6), 노랑=경고 (<0.1), 빨강=위반; 제약조건 위반 시 배너 빨간색으로 변경 + "N conflicting" 표시
- `crates/viewer/src/gui/sketch_ui.rs`: 제약조건별 잔차 색상을 위한 색상 오버라이드 변형 (`draw_distance_c`, `draw_geo_line_sym_c`, `draw_coincident_c`, `draw_fixed_c` 등)

**뷰어 — 스케치 재편집:**
- `crates/viewer/src/gui/mod.rs`: `GuiState`에 `last_sketch: Option<(Sketch, WorkPlane)>` 필드 — 닫을 때 스케치 데이터 저장
- `crates/viewer/src/app.rs`: `EditSketch` 액션 — 마지막으로 닫은 스케치를 Select 모드로 다시 열어 모든 엔티티와 제약조건 보존
- `crates/viewer/src/gui/toolbar.rs`: 이전 스케치가 있을 때 Sketcher 툴바에 "Edit Sketch" 버튼 표시

**뷰어 — 수치 제약조건 입력:**
- `crates/viewer/src/gui/toolbar.rs`: 툴바에 Distance (D:), Angle (A: 도 단위 접미사), Radius (R:) 제약조건의 DragValue 인라인 입력 — 제약조건 적용 전 정확한 값 설정 가능

#### V25: 스케치 인터랙티브 편집 (2026-04-06)

**뷰어 — 스케치 점 드래그:**
- `crates/viewer/src/app.rs`: Select 모드에서 클릭+드래그로 스케치 점 이동 — 누를 때 undo 스냅샷 저장, 드래그 중 스냅 적용하여 점 위치 업데이트, 실제 이동 없으면 스냅샷 취소

**뷰어 — 스케치 호버 프리셀렉션:**
- `crates/viewer/src/app.rs`: CursorMoved에서 `hit_test_sketch()`로 `hovered_entity` 업데이트 — 클릭 선택과 동일한 히트테스트 로직
- `crates/viewer/src/gui/sketch_ui.rs`: 호버된 엔티티 초록색 하이라이트 (rgb 100,255,150), 두꺼운 스트로크 (2.5px), 점에 링

**뷰어 — 스케치 다시 실행 + Escape:**
- `crates/viewer/src/gui/mod.rs`: `redo()` 메서드가 `redo_stack`에서 이전에 취소된 스냅샷 복원
- `crates/viewer/src/app.rs`: Ctrl+Shift+Z로 스케치 모드에서 다시 실행; Escape는 먼저 pending_point/polyline_points 초기화, 대기 중인 것 없을 때만 스케치 취소

**뷰어 — 스케치 우클릭 컨텍스트 메뉴:**
- `crates/viewer/src/gui/sketch_ui.rs`: `draw_sketch_context_menu()` — Delete (Del), Horizontal (H), Vertical (V), Fixed, Select All (Ctrl+A), Clear Selection이 있는 egui 팝업; 컨텍스트 감지 (선 선택 시에만 제약조건 항목 표시)
- `crates/viewer/src/app.rs`: Select 모드에서 대기 중인 지오메트리 없을 때 우클릭으로 컨텍스트 메뉴 트리거; 대기 중인 지오메트리 있으면 초기화

**뷰어 — DOF 표시기 + 선택 정보:**
- `crates/viewer/src/gui/sketch_ui.rs`: 배너에 DOF 수 표시 (`degrees_of_freedom()`로 제약조건 타입별 가중치 계산), 완전 구속 시 초록색; 선택 수 표시
- `crates/viewer/src/gui/status_bar.rs`: 상태 바에서 정확한 DOF 계산을 위해 `degrees_of_freedom()` 사용; 스케치 선택 수 표시

#### V24: 스케치 편집 기반 (2026-04-06)

**뷰어 — 스케치 엔티티 선택:**
- `crates/viewer/src/gui/mod.rs`: `SketchEntityRef` 열거형 (Point/Line/Arc/Circle/Ellipse/BSpline) + `SketchMode`에 `selected_entities: Vec<SketchEntityRef>` 필드
- `crates/viewer/src/app.rs`: Select 도구 히트테스트 — 점 근접 (0.24 임계값), 선분 거리, 원/호 반경 거리, 타원 정규화 거리, B-스플라인 제어 다각형 거리; Ctrl+클릭으로 다중 선택 토글
- `crates/viewer/src/gui/sketch_ui.rs`: 선택된 엔티티 파란색 하이라이트 (rgb 80,160,255), 두꺼운 스트로크 (3px vs 2px), 점에 선택 링

**뷰어 — 스케치 엔티티 삭제:**
- `crates/viewer/src/app.rs`: `delete_sketch_entities()` — 스케치 벡터에서 선택된 엔티티 제거 (높은 인덱스부터 시프트 방지); 참조하는 모든 엔티티의 PointId 연쇄 조정 (lines, arcs, circles, ellipses, B-splines)
- `crates/viewer/src/app.rs`: 스케치 모드에서 Delete/Backspace로 스냅샷 저장 후 선택된 엔티티 삭제 (undo 지원)

**뷰어 — 스케치 키보드 단축키:**
- `crates/viewer/src/app.rs`: S=Select, L=Line, R=Rectangle, C=Circle, A=Arc, E=Ellipse, P=Point, B=B-Spline, W=Polyline (스케치 모드에서만, ctrl 없이)
- `crates/viewer/src/app.rs`: H=Horizontal 제약조건, V=Vertical 제약조건 (선택된 선에 적용, 스케치 모드 전용)
- `crates/viewer/src/gui/toolbar.rs`: 모든 스케치 도구 및 제약조건 버튼에 단축키 힌트 업데이트

**뷰어 — 인터랙티브 제약조건 (선택 기반):**
- `crates/viewer/src/app.rs`: 모든 제약조건 툴바 버튼이 선택된 엔티티에 적용: Coincident (2점), Parallel/Perpendicular/Equal (2선), Fixed/Block (현재 위치 점), Distance (2점 또는 선 길이), Angle (2선, 도 단위), Radius (원), H-Distance/V-Distance (2점); 선택 없으면 마지막 엔티티로 폴백
- `crates/viewer/src/app.rs`: 제약조건 액션의 모든 `log_info()` 스텁 제거 — 실제 제약조건 적용으로 대체

**뷰어 — Extrude 거리 UI:**
- `crates/viewer/src/gui/toolbar.rs`: Sketcher 툴바에 Close/Cancel 전 DragValue 입력 (0–1000mm 범위, 0.5 스텝); 스케치 중 편집 가능

#### V23: 스케치 인터랙션 개선 (2026-04-06)

**뷰어 — 스케치 실시간 미리보기:**
- `crates/viewer/src/gui/sketch_ui.rs`: 모든 그리기 도구에서 펜딩 포인트와 커서 사이 고무줄 미리보기: Line/Slot (점선), Rectangle (4개 점선), Circle (점선 원 + 반지름), Arc (반지름 + 반원), Ellipse (점선 타원), Polygon (점선 윤곽), Polyline/BSpline (마지막 점에서 커서까지 점선)

**뷰어 — 스케치 스냅:**
- `crates/viewer/src/app.rs`: `snap_sketch_coords()` — 그리드 스냅 (0.5 격자) + 점 스냅 (가장 가까운 기존 점에 0.3 거리 임계값), 모든 도구에서 엔티티 생성 전 적용

**뷰어 — 스케치 도구 수정:**
- `crates/viewer/src/app.rs`: 타원 도구가 `add_circle()` 대신 `add_ellipse(center, major_end, minor_radius)` 사용 — 독립적 rx/ry로 실제 타원 생성
- `crates/viewer/src/app.rs`: 호 도구가 하드코딩된 0→π 호 대신 클릭 각도 기반 ±90° 반원 사용 — 중심에서 커서 방향을 따라 호 방향 결정

**뷰어 — 스케치 실행 취소:**
- `crates/viewer/src/gui/mod.rs`: `SketchSnapshot` 구조체 + `SketchMode`에 `undo_stack` 필드 — 각 연산 전 엔티티 수 (points/lines/arcs/circles/ellipses/bsplines/constraints) 기록
- `crates/viewer/src/app.rs`: 스케치 모드에서 Ctrl+Z가 `SketchMode::undo()` 호출 — 모든 엔티티 벡터를 연산 전 스냅샷으로 절단, 펜딩 포인트 초기화; 스택 비어있으면 전역 실행취소로 전달

**뷰어 — 다각형 미리보기 수정:**
- `crates/viewer/src/gui/sketch_ui.rs`: 다각형 미리보기 정점 할당의 `u32` → `usize` 용량 변환 수정

#### V22: 통합 자동 피킹 + 더블클릭 루프 + 호버 미리보기 (2026-04-06)

**뷰어 — 통합 자동 피킹:**
- `crates/viewer/src/app.rs`: 모든 선택 모드에서 `pick_auto()` 사용 — 툴바 선택 모드와 관계없이 vertex > edge > face > solid 자동 감지; 사용하지 않는 `pick_face()`, `pick_edge_mode()`, `pick_vertex_mode()` 함수 제거
- `crates/viewer/src/app.rs`: 프리셀렉션(호버)도 통합 자동 피킹 사용 — 커서 아래 가장 구체적인 엔티티 하이라이트
- `crates/viewer/src/gui/context_menu.rs`: 컨텍스트 메뉴가 실제 선택된 엔티티 타입에 따라 적응 (선택 모드가 아닌 `selected_entities` 내용 기반)
- `crates/viewer/src/gui/properties.rs`: 엔티티 선택 시 항상 하위 요소 속성 표시 (모드 확인 제거)
- `crates/viewer/src/gui/overlays.rs`: 선택 오버레이가 `selected_entities.is_empty()` 기준으로 그려짐 (모드 아님)

**뷰어 — 더블클릭 루프 선택:**
- `crates/viewer/src/app.rs`: `try_double_click_loop()` — 더블클릭 감지 (300ms, 10px 근접); 엣지 더블클릭 → 엣지 루프 선택, 면 더블클릭 → 면 루프 선택
- `crates/viewer/src/app.rs`: `last_click_time` / `last_click_pos` 필드 (더블클릭 타이밍)

**뷰어 — 호버 미리보기:**
- `crates/viewer/src/gui/status_bar.rs`: `build_hover_preview()` — 커서 아래 엔티티 타입 및 인덱스 표시 (예: "Edge 12", "Face 3", "Vertex 5")
- `crates/viewer/src/gui/status_bar.rs`: 선택 모드 표시기가 이제 "Auto" 표시 (항상 자동 피킹 활성)

**뷰어 — 가장 가까운 표준 뷰 스냅:**
- `crates/viewer/src/app.rs`: `try_snap_to_nearest_view()` — 궤도 드래그 종료 후 카메라가 표준 뷰(Front/Back/Right/Left/Top/Bottom)에서 ~10° 이내이면 자동 애니메이션 스냅; `nav.snap_to_nearest` 설정으로 제어
- `crates/viewer/src/app.rs`: `was_orbiting` 플래그가 마우스 버튼 릴리즈 이벤트에서 궤도 상태 추적 (모든 네비게이션 스타일 지원)

**뷰어 — 피킹 임계값 튜닝:**
- `crates/viewer/src/app.rs`: 정점 임계값 (0.012×거리) 및 엣지 임계값 (0.015×거리)을 조정하여 우발적 정점 선택 감소, 엣지/면 선택 정확도 유지

#### V21: 엣지/면 루프 선택 + 자동 피킹 + 네비게이션 수정 (2026-04-03)

**뷰어 — 엣지 루프 선택:**
- `crates/viewer/src/app.rs`: `compute_edge_loop()` — 시드 엣지에서 공유 꼭짓점을 통한 BFS valence-2 체인 워크; 양방향으로 연결된 엣지 수집하여 루프 형성
- `crates/viewer/src/gui/mod.rs`: `GuiAction::SelectEdgeLoop` — 첫 번째 선택된 엣지를 포함하는 루프의 모든 엣지 선택

**뷰어 — 엣지 링 선택:**
- `crates/viewer/src/app.rs`: `compute_edge_ring()` — 쿼드 면을 가로지르는 반대편 엣지 순회; 시드 엣지에서 4변 면의 index+2 엣지 선택
- `crates/viewer/src/gui/mod.rs`: `GuiAction::SelectEdgeRing` — 링의 모든 엣지 선택

**뷰어 — 면 루프 선택:**
- `crates/viewer/src/app.rs`: `compute_face_loop()` — 시드 면에서 공유 엣지를 통한 BFS 외향 탐색, 전이적으로 연결된 모든 면 수집
- `crates/viewer/src/gui/mod.rs`: `GuiAction::SelectFaceLoop` — 연결 영역의 모든 면 선택

**뷰어 — 컨텍스트 메뉴 연결:**
- `crates/viewer/src/gui/context_menu.rs`: 엣지 컨텍스트 메뉴 — "Select Edge Loop" 및 "Select Edge Ring"이 실제 토폴로지 기반 액션 디스패치 (기존 플레이스홀더 StatusMessage 대체)
- `crates/viewer/src/gui/context_menu.rs`: 면 컨텍스트 메뉴 — "Select Face Loop"가 실제 BFS 면 루프 선택 디스패치

**뷰어 — 필릿/챔퍼 파라미터 UI:**
- `crates/viewer/src/gui/toolbar.rs`: Part 툴바 Fillet/Chamfer가 항상 태스크 패널 열기 (엣지 선택 시에도) — 툴팁이 선택/전체 엣지 구분 표시
- `crates/viewer/src/gui/toolbar.rs`: PartDesign 드레스업 플라이아웃이 Fillet/Chamfer 항상 태스크 패널 열기
- `crates/viewer/src/gui/task_panel.rs`: Fillet/Chamfer 태스크 패널에 사전 선택된 엣지 수 "N selected" 표시
- `crates/viewer/src/gui/context_menu.rs`: 엣지 Fillet/Chamfer 컨텍스트 메뉴가 직접 디스패치 대신 태스크 패널 열기

**뷰어 — 자동 피킹 (Solid 모드):**
- `crates/viewer/src/app.rs`: `pick_auto()` — Solid 선택 모드에서 클릭 위치의 가장 구체적인 하위 요소를 자동 감지 (vertex > edge > face > solid), 수동 모드 전환 불필요
- `crates/viewer/src/app.rs`: `select_object_for_pick()` — 공유 헬퍼, 객체 선택 및 모델 데이터 로드

**뷰어 — 네비게이션 스타일 수정 & 확장:**
- `crates/viewer/src/nav.rs`: FreeCADGesture 설명 텍스트 수정 (기존 "LMB: Orbit" → "LMB: Select | MMB: Orbit")
- `crates/viewer/src/nav.rs`: FreeCADGesture에 `Shift+MMB → Pan` 추가 (실제 FreeCAD와 일치)
- `crates/viewer/src/nav.rs`: Inventor 매핑 수정 (기존 MMB=Pan/Shift+MMB=Orbit → MMB=Orbit/Shift+MMB=Pan)
- `crates/viewer/src/nav.rs`: FreeCAD 전체 12개 네비게이션 스타일, 정확한 버튼+수정키 매핑:
  - CAD(기본), Gesture, Blender, Maya, SolidWorks, OpenInventor, OpenCascade, OpenSCAD, Revit, SiemensNX, TinkerCAD, Touchpad
  - 수정: OpenInventor (LMB=Orbit), OpenCascade (Ctrl+RMB=Orbit, Ctrl+LMB=Zoom), OpenSCAD (LMB=Orbit, MMB=Zoom)
  - 신규: Gesture (LMB 드래그=Orbit), Maya (Alt+LMB/MMB/RMB), SiemensNX (MMB+RMB=Pan), Touchpad (Alt+Move=Orbit)
- `crates/viewer/src/nav.rs`: `OrbitStyle` 열거형 — Turntable, Trackball, Free Turntable, Trackball Classic, Rounded Arcball (기본)
- `crates/viewer/src/nav.rs`: `RotationMode` 열거형 — Window center (기본), Drag at cursor, Object center
- `crates/viewer/src/nav.rs`: NavConfig 신규 필드: `orbit_style`, `rotation_mode`, `zoom_step`, `zoom_at_cursor`, `disable_touch_tilt`, `enable_spinning`, `show_rotation_center`, `rotation_center_size`
- `crates/viewer/src/nav.rs`: `resolve_drag()`에 `alt` 파라미터 (Maya/Touchpad Alt+버튼 네비게이션)
- `crates/viewer/src/app.rs`: 스케치 모드에서 LMB 기반 orbit 억제 (Gesture/OpenInventor/OpenSCAD 충돌 방지)
- `crates/viewer/src/gui/dialogs.rs`: 설정 다이얼로그 — "Orbit & Rotation" 섹션 (orbit style, rotation center, rotation mode 드롭다운), 감도 섹션 (zoom step, zoom-at-cursor, touch tilt), 애니메이션 (spinning 토글)

**뷰어 — 네비게이션 동작 구현:**
- `crates/viewer/src/nav.rs`: `apply_orbit()` — OrbitStyle 기반 궤도 회전 연산:
  - Turntable: yaw/pitch 회전, pitch ±89° 클램핑 (짐벌 잠금 방지)
  - FreeTurntable: yaw/pitch 회전, pitch 제한 없음 (극점 통과 자유 회전)
  - Trackball/TrackballClassic/RoundedArcball: 가상 구체 트랙볼 매핑 — 커서 위치를 가상 구체에 투영, 이전/현재 벡터 간 호 각도에서 회전각 계산, 화면 공간 회전축에서 교차 결합된 yaw/pitch 도출
- `crates/viewer/src/nav.rs`: `drag_zoom_factor()` — 연속 마우스 드래그 줌용 별도 줌 계산 (`zoom_sensitivity` 사용)
- `crates/viewer/src/nav.rs`: `scroll_zoom_factor()`가 이제 `zoom_step` 사용 (0.2 = 스크롤당 20% 줌, FreeCAD 기본값과 일치)
- `crates/viewer/src/app.rs`: `apply_rotation_mode_pivot()` — RotationMode 기반 궤도 중심점:
  - WindowCenter: 카메라 타겟 중심 궤도 (기본값)
  - ObjectCenter: 선택된 객체의 정점 무게 중심 궤도
  - DragAtCursor: 화면 공간 레이 근사로 커서 위치 방향으로 궤도 중심 이동
- `crates/viewer/src/app.rs`: zoom_at_cursor 구현 — 스크롤 줌 시 카메라 타겟을 커서 아래 지점으로 줌 양에 비례하여 이동 (NDC 기반 화면 공간 투영)
- `crates/viewer/src/lib.rs`: 단순 뷰어에 `apply_orbit()` 및 `drag_zoom_factor()` 호출 적용

#### V20: 선택 기반 연산 (2026-04-03)

**뷰어 — 선택된 엣지에 필릿/챔퍼:**
- `crates/viewer/src/app.rs`: `selected_edge_pairs()` — `SelectedEntity::Edge` 핸들을 필릿/챔퍼 연산용 `(Handle<VertexData>, Handle<VertexData>)` 쌍으로 변환
- `crates/viewer/src/app.rs`: `FilletAllEdges`/`ChamferAllEdges` 핸들러가 선택된 엣지를 순회하며 순차적으로 연산 적용; 엣지 미선택 시 첫 번째 엣지 쌍으로 폴백

**뷰어 — 선택된 면에 스케치:**
- `crates/viewer/src/app.rs`: `compute_face_workplane()` — 선택된 첫 번째 면에서 WorkPlane 계산 (면 삼각형에서 무게중심 + 법선, 수직 x축)
- `crates/viewer/src/gui/mod.rs`: `GuiAction::SketchOnSelectedFace` — 선택된 면의 계산된 작업 평면에서 스케치 모드 진입
- `crates/viewer/src/app.rs`: `SketchOnSelectedFace` 핸들러 — 면 기반 WorkPlane으로 Sketcher 워크벤치 진입

**뷰어 — 컨텍스트 메뉴 연결:**
- `crates/viewer/src/gui/context_menu.rs`: 면 컨텍스트 메뉴 — "Create Sketch on Face"가 `SketchOnSelectedFace` 디스패치 (기존 플레이스홀더 `WorkPlane::xy()` 대체)
- `crates/viewer/src/gui/context_menu.rs`: 엣지 컨텍스트 메뉴 — "Fillet/Chamfer Selected Edges"가 선택된 엣지 사용하여 `FilletAllEdges`/`ChamferAllEdges` 디스패치
- `crates/viewer/src/gui/context_menu.rs`: 꼭짓점 컨텍스트 메뉴 — "Fillet at Vertex"가 `FilletAllEdges` 디스패치
- `crates/viewer/src/gui/context_menu.rs`: 측정 버튼이 `ToggleMeasurement` 디스패치 (기존 플레이스홀더 `StatusMessage` 대체)

**뷰어 — 선택 인식 툴바:**
- `crates/viewer/src/gui/toolbar.rs`: Part 툴바 Fillet/Chamfer 버튼 — 엣지 선택 시 `FilletAllEdges`/`ChamferAllEdges` 직접 디스패치; 미선택 시 태스크 패널 열기
- `crates/viewer/src/gui/toolbar.rs`: PartDesign 드레스업 플라이아웃 — 엣지 선택 시 Fillet/Chamfer 직접 디스패치
- `crates/viewer/src/gui/toolbar.rs`: Sketcher 툴바 — 면 선택 시 "On Face" 버튼 표시, `SketchOnSelectedFace` 디스패치

#### V19: 다중 선택 + 측정 + 선택 툴바 (2026-04-03)

**뷰어 — 다중 선택 (Ctrl+클릭):**
- `crates/viewer/src/gui/mod.rs`: `selected_entities: Vec<SelectedEntity>` — 단일 `selected_entity` 대체, Face/Edge/Vertex 다중 선택 지원
- `crates/viewer/src/app.rs`: `pick_face()`, `pick_edge_mode()`, `pick_vertex_mode()` — Ctrl+클릭으로 선택 목록에 토글; Ctrl 없이 클릭하면 교체
- `crates/viewer/src/app.rs`: `toggle_entity()` 헬퍼 — 선택 Vec에 엔티티 추가/제거
- `crates/viewer/src/scene.rs`: `Scene::select_all()` — 모든 보이는 객체 선택
- `crates/viewer/src/gui/overlays.rs`: `draw_selection_overlay()` 모든 `selected_entities` 순회하여 파란색 하이라이트

**뷰어 — 하위 요소 간 측정:**
- `crates/viewer/src/gui/overlays.rs`: `draw_measurement_overlay_between()` — 정확히 2개 엔티티 선택 시 대표점 사이에 점선 청록색 라인 + 거리 라벨(mm) 표시
- `crates/viewer/src/gui/overlays.rs`: `representative_point()` — 꼭짓점 위치 / 엣지 중점 / 면 무게중심
- `crates/viewer/src/gui/properties.rs`: 다중 선택 요약에 "Measurement" 접이식 그룹 — 2개 엔티티 선택 시 거리(mm) 표시

**뷰어 — 다중 선택 속성 패널:**
- `crates/viewer/src/gui/properties.rs`: `draw_multi_selection_summary()` — 면/엣지/꼭짓점 수, 총 면적, 총 길이 표시

**뷰어 — 선택 모드 툴바:**
- `crates/viewer/src/gui/toolbar.rs`: 4개 토글 버튼(Solid/Face/Edge/Vertex) + 선택 수 배지 + Select All / Deselect 버튼
- `crates/viewer/src/gui/mod.rs`: `GuiAction::SetSelectionMode(SelectionMode)` — 모드 변경 액션
- `crates/viewer/src/app.rs`: 키보드 단축키 — Key 2 → Face 모드, Key 4 → Vertex 모드 (Keys 1/3은 표준 뷰에 예약됨)

### 수정됨

#### V18: 핵심 피킹/선택 수정 + 하위 요소 시각적 하이라이트 (2026-04-02)

**뷰어 — 3D 피킹 수정 (근본 원인):**
- `crates/viewer/src/render.rs`: `mat4_inv()` 수정 — 4×4 행렬 역행렬에 두 가지 버그가 있어 `inv_view_proj()`가 완전히 잘못된 결과를 생성:
  - 버그 1: 여인수(cofactor) `c` 값 이름이 역순 (`c5,c4,c3,c2,c1,c0` → `c0,c1,c2,c3,c4,c5`), 모든 수반행렬(adjugate) 항이 잘못된 2×2 소행렬(minor)을 사용
  - 버그 2: 마지막 4개 수반행렬 항이 행 2 요소(`m(2,...)`) 대신 행 3 요소(`m(3,...)`)를 사용하여 잘못된 여인수 생성
  - 결합 효과: `VP * inv_VP ≠ 단위행렬` — 피킹 레이가 잘못된 월드 좌표에서 계산되어 커서-객체 선택이 완전히 불안정
  - GLM 기반 올바른 구현으로 교체, `VP * inv_VP = I` 테스트로 검증
- `crates/viewer/src/picking.rs`: 하위 요소 피킹 추가 (Face/Edge/Vertex 모드), B-Rep 토폴로지 순회
- `crates/viewer/src/scene.rs`: SceneObject에 face→삼각형 맵, 엣지 끝점, 꼭짓점 위치 저장

**뷰어 — 하위 요소 시각적 하이라이트 오버레이:**
- `crates/viewer/src/gui/overlays.rs`: `draw_selection_overlay()` + `draw_entity_highlight()` — 선택/프리셀렉션 하위 요소에 대한 화면 공간 시각 피드백:
  - Face: 반투명 파란색 채움 + 삼각형 가장자리 윤곽선 (`face_tri_map` 기반)
  - Edge: 두꺼운 하이라이트 라인 + 끝점 도트 (`edge_positions`/`edge_handles` 기반)
  - Vertex: 채워진 원 + 흰색 윤곽 링 (`vertex_positions`/`vertex_handles` 기반)
- 프리셀렉션(호버) 하이라이트: 연한 녹색 색조, 선택과 동일하면 생략
- 선택 하이라이트: 파란색 색조, 프리셀렉션 위에 렌더링
- `crates/viewer/src/gui/mod.rs`: `GuiState`에 `preselected_entity` + `preselected_object_id` 필드 추가
- `crates/viewer/src/app.rs`: `update_preselection()`이 이제 객체 ID만이 아닌 하위 요소 핸들(Face/Edge/Vertex)도 추적
- `crates/viewer/src/scene.rs`: `Scene::get_object(id)` — 프리셀렉션 객체 조회용 접근자

**뷰어 — 하위 요소 속성 패널:**
- `crates/viewer/src/gui/properties.rs`: 속성 패널이 선택된 하위 요소의 상세 정보를 표시:
  - Face: 핸들 ID, 삼각형 수, 계산된 면적 (mm²), 루프 수
  - Edge: 핸들 ID, 시작/끝 꼭짓점 위치, 계산된 길이 (mm)
  - Vertex: 핸들 ID, X/Y/Z 좌표 (소수점 6자리)
- 모든 속성이 기존 검색 필터를 지원
- Base와 Creation Parameters 사이에 접을 수 있는 "Selected Face/Edge/Vertex" 그룹으로 표시

**뷰어 — 신규 피킹 테스트 (+3):**
- `test_project_unproject_roundtrip`: 5개 월드 포인트를 스크린에 투영 후 역투영, 레이가 원래 점을 통과하는지 검증 (기본 카메라 yaw=0.8, pitch=0.4)
- `test_vp_inverse_identity`: `VP * inv_VP = 단위행렬` f32 허용 범위 내 검증
- `test_pick_box_default_camera`: VP 행렬로 박스 중심을 스크린에 투영 후 해당 위치에서 피킹, 히트 검증

### 추가됨

#### V17: 복합 모델 스트레스 테스트 및 Python 패키징 (2026-03-31)

**QA — 복합 모델 스트레스 테스트 (+49개 신규 테스트):**
- `crates/modeling/tests/stress_tests.rs`: 5가지 실제 CAD 워크플로우 카테고리를 커버하는 49개 통합 테스트 추가
- 카테고리 1 — 다중 피처 PartDesign Body (9개 테스트): pad+pocket+chamfer 체인, 5-피처 순차 body, 피처 억제/재정렬/되감기, mirror/linear-pattern 피처, revolve+groove 피처 체인, 바디 간 객체 이동
- 카테고리 2 — 10개 이상 파트 어셈블리 (9개 테스트): 10-박스 어셈블리, 수량 포함 12-파트 BOM, 배치 변환, 가시성 토글, Coincident/Distance 구속조건, 점 변환, 분해 뷰, 10개 파트 BVH 간섭 감지
- 카테고리 3 — 20개 이상 구속조건 스케치 (10개 테스트): 19개 구속조건 L-형상, 대칭, 동심원, 수평/수직 거리, 완전 구속 검증, 중점, 등길이, 호-접선, 반지름, 공선
- 카테고리 4 — 불리언 체인 5회 이상 (6개 테스트): 5-실린더 빼기 플레이트, 5-박스 합집합 십자형, 교집합 체인, XOR+빼기 체인, 교대 합집합/빼기 (6회 연산), 정밀 불리언 5-박스 합집합
- 카테고리 5 — 전체 I/O 라운드트립 (10개 테스트): JSON 박스/구, ASCII STL 박스, 이진 STL 실린더, STEP 박스, OBJ 구, glTF 박스, PLY 토러스, BREP 박스, 병렬 테셀레이션
- 크로스 도메인 워크플로우 (5개 테스트): 스케치→압출→검사→테셀레이션→STL 내보내기, 다중 불리언→내보내기 브라켓, 어셈블리 BOM 내보내기, 모든 프리미티브 형상 검사, 스케일→미러→패턴 체인
- 테스트 스위트 1300개 → 1369개로 확장 (+69개 추가, 실패 0개)

#### V16: 성능 최적화, Python 바인딩 및 테스트 확장 (2026-04-01)

**성능 — 병렬 불리언 연산:**
- `cadkernel-modeling`: `boolean_op()`에 rayon `par_iter()` 적용 — 페이스 분류 내부 루프 병렬 처리
- `cadkernel-modeling`: `BoolOp::evaluate_faces_parallel()` — rayon 기반 병렬 페이스 분류 (Inside/Outside/OnBoundary)
- `cadkernel-modeling`: `merge_boolean_results()` — 병렬 분류된 페이스를 최종 B-Rep으로 통합

**성능 — BVH 및 공간 인덱싱:**
- `cadkernel-geometry` (BVH): `query_aabb_parallel()` — 배치 쿼리를 위한 rayon 병렬 BVH 탐색
- `cadkernel-geometry` (BVH): `build_sah()` — 최적 트리 품질을 위한 Surface Area Heuristic 구성
- `cadkernel-geometry` (BVH): `refit()` — 전체 재구성 없이 동적 장면을 위한 상향식 AABB 갱신

**성능 — NURBS 기저 함수 캐싱:**
- `cadkernel-geometry`: `BasisCache` — `basis_funs()` 결과를 위한 LRU 캐시 (용량 1024), (degree, knot_hash, span, t) 키 사용
- `cadkernel-geometry`: `CachedNurbsCurve` 래퍼 — NurbsCurve 평가 위에 투명한 캐싱 레이어
- `cadkernel-geometry`: `NurbsSurface::evaluate_cached()` — 반복 UV 쿼리를 위한 캐시된 곡면 평가

**Python 바인딩 (PyO3 0.23):**
- `cadkernel-python`: Sprint 2/3 API 전체 바인딩 업데이트: `make_cone()`, `make_torus()`, 어셈블리 DOF 분석, FEM 메시 생성
- `cadkernel-python`: `PyAssembly` 클래스 — `add_component()`, `add_constraint()`, `solve()`, `analyze_dof()`
- `cadkernel-python`: `PyFem` 클래스 — `generate_tet_mesh()`, `static_analysis()`, `thermal_analysis()`
- `cadkernel-python`: 6개 Python 클래스 전체를 커버하는 74개 Python 통합 테스트

**QA — 테스트 확장:**
- 테스트 스위트 1136개 → 1300개로 확장 (+164개 신규 테스트)
- 신규 테스트 영역: 컴파운드 연산, 조인 연산, 서피스 연산, 어셈블리 솔버, FEM 분석, 메시 연산, 파일 포맷 라운드트립 (DXF, PLY, 3MF, BREP, VRML, AMF, OCA, COLLADA, DWG), 형상 분석, 바디 연산, 기어, 공간 쿼리, 멀티 트랜스폼, 질량 계산
- BREP 포맷: 14개 테스트 (정점/엣지/페이스/셸/솔리드 수 라운드트립, 잘못된 입력, 섹션 순서)
- OCA 포맷: 15개 테스트 (명령어, 노멀, 대소문자 무관, 복수 페이스 라운드트립)
- 전체 1300개 테스트 통과, clippy 경고 0개, 빌드 오류 0개

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
- 9개 워크벤치 전체 ~70개 버튼에 Unicode 심볼
- Show/Hide All, 오브젝트 수, 삼각형 수, 선택 이름

**추가:**
- About 다이얼로그: 크레이트 정보, 렌더러, 기능 수
- Escape 계층: task panel → 선택해제 → sketch → 종료
- Ctrl+O (열기), Ctrl+S (저장) 단축키
- Import STL/OBJ가 Scene에 추가 (다중 오브젝트 유지)

**Phase 3 (FreeCAD ComboView + 고급 인터랙션):**
- ComboView: 트리 + 속성 단일 좌측 패널 (55/45 스플리터)
- Task Panel 라이브 프리뷰 (파라미터 조정 시 프리뷰 오브젝트 실시간 업데이트)
- 인라인 이름 변경 (트리에서 더블클릭, Enter 확인)
- 다중 선택: Ctrl+클릭 (트리 + 3D 뷰포트)
- 씬 불리언 연산 (2개 오브젝트 선택 → Union/Subtract/Intersect)
- 오브젝트 투명도 슬라이더 (View 탭, opacity 0.1–1.0)
- 최근 파일 (File 메뉴, 최근 10개)
- Color Picker (View 탭)
- 스플래시 화면 (중앙, 페이드아웃 애니메이션)
- 피처 히스토리 (트리에서 생성 이력 표시)
- NavCube 코너 위치 선택기 (4코너, Settings에서 설정)
- Workbench 드롭다운 선택기

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

#### Phase W: FreeCAD 패리티 스프린트 (2026-03-24)

**Part 형상 프리미티브:**
- `cadkernel-modeling`: `make_circle_shape()`, `make_ellipse_shape()`, `make_point_shape()`, `make_line_shape()` — Part 워크벤치 형상 프리미티브
- `cadkernel-modeling`: `shape_builder_from_edges()` — 엣지 리스트에서 형상 조합
- `cadkernel-modeling`: `convert_to_solid()` — 셸/메시를 솔리드로 변환

**PartDesign 완성:**
- `cadkernel-modeling`: `additive_loft()`, `additive_pipe()` — 통합 가산 로프트 및 파이프 스위프 연산
- `cadkernel-modeling`: `make_sprocket()` — 매개변수 스프로킷 프로파일 생성기
- `cadkernel-modeling`: `shaft_design()` — 단차 프로파일의 샤프트 설계 마법사
- `cadkernel-modeling`: `shape_binder()`, `sub_shape_binder()` — 기하 참조 도구
- `cadkernel-modeling`: Body 컨텍스트 메뉴 — `suppress_feature()`, `set_tip()`, `move_feature()`

**스케처 기하 확장:**
- `cadkernel-sketch`: `add_periodic_bspline()`, `add_bspline_from_knots()` — 고급 B-스플라인 생성
- `cadkernel-sketch`: `add_centered_rectangle()`, `add_rounded_rectangle()` — 사각형 변형
- `cadkernel-sketch`: `add_slot()`, `add_arc_slot()` — 슬롯 기하 생성
- `cadkernel-sketch`: `add_circle_3pt()`, `add_ellipse_3pt()` — 3점 원 및 타원
- `cadkernel-sketch`: Refraction 제약조건 (스넬 법칙)
- `cadkernel-sketch`: `toggle_driving_reference()` — 구동/참조 제약 전환
- `cadkernel-sketch`: `attach_to_plane()`, `reorient()`, `merge_with()`, `mirror_geometry()` — 스케치 관리

**스케처 B-스플라인 도구:**
- `cadkernel-sketch`: `geometry_to_bspline()` — 기하를 B-스플라인으로 변환
- `cadkernel-sketch`: `increase_bspline_degree()`, `decrease_bspline_degree()` — 차수 조정
- `cadkernel-sketch`: `increase_knot_multiplicity()`, `decrease_knot_multiplicity()` — 노트 다중도 연산
- `cadkernel-sketch`: `insert_knot()`, `join_curves()` — 노트 삽입 및 커브 결합
- `cadkernel-sketch`: `external_projection()`, `carbon_copy()` — 외부 기하 도구
- `cadkernel-sketch`: `move_geometry()`, `rotate_geometry()`, `scale_geometry()`, `offset_geometry()`, `mirror_geometry()` — 기하 편집
- `cadkernel-sketch`: `delete_all_geometry()`, `delete_all_constraints()` — 일괄 삭제

**TechDraw 뷰 & 치수:**
- `cadkernel-io`: `broken_view()`, `complex_section_view()`, `clip_group()`, `active_view()`, `project_shape_2d()` — 새 뷰 타입
- `cadkernel-io`: `contextual_dimension()`, `angle_from_3_points()`, `area_annotation()`, `arc_length_dimension()`, `hv_extent_dimension()` — 새 치수 타입
- `cadkernel-io`: `repair_dimension_refs()` — 치수 참조 수리
- `cadkernel-io`: `rich_text_annotation()`, `balloon_annotation()`, `axonometric_length_dimension()` — 새 주석
- `cadkernel-io`: `geometric_hatch()`, `weld_symbol()` (ISO 2553), `hole_shaft_fit()` — 기호

**TechDraw 중심선, 장식, 서식:**
- `cadkernel-io`: `centerline_on_face()`, `centerline_between_lines()`, `centerline_between_points()`, `bolt_circle_centerlines()` — 중심선 도구
- `cadkernel-io`: `cosmetic_line()`, `cosmetic_thread_internal()`, `cosmetic_thread_external()`, `cosmetic_vertex()`, `cosmetic_circle()`, `cosmetic_arc()` — 장식 요소
- `cadkernel-io`: `cosmetic_parallel_line()`, `cosmetic_perpendicular_line()` — 장식 선 도구
- `cadkernel-io`: `chain_dimension()`, `coordinate_dimension()`, `chamfer_dimension()`, `FormattedDimension` — 치수 서식
- `cadkernel-io`: `stack_order()`, `align_elements()`, `lock_element()` — 요소 관리
- `cadkernel-io`: `page_from_template()`, `update_template_fields()`, `redraw_page()`, `print_all_pages()` — 페이지 관리
- `cadkernel-io`: `edit_line_appearance()`, `toggle_edge_visibility()` — 선 외관

**Draft 워크벤치:**
- `cadkernel-modeling`: `make_arc_3pt_draft()`, `make_ellipse_wire()`, `make_rectangle_wire()`, `make_polygon_wire()` — 와이어 생성
- `cadkernel-modeling`: `make_bezier_wire()`, `make_cubic_bezier_wire()`, `make_point_draft()`, `make_facebinder()`, `draft_hatch()` — 제도 도구
- `cadkernel-modeling`: `make_draft_dimension_full()`, `make_label_full()`, `AnnotationStyle` — 주석 시스템
- `cadkernel-modeling`: `move_draft()`, `rotate_draft()`, `scale_draft()`, `mirror_draft()`, `offset_draft()`, `trimex_draft()`, `stretch_draft()` — 수정 도구
- `cadkernel-modeling`: `circular_array()`, `path_link_array()`, `point_link_array()` — 배열 패턴
- `cadkernel-modeling`: `edit_draft()`, `join_draft()`, `split_draft()`, `draft_to_sketch()` — Draft 편집
- `cadkernel-modeling`: `SnapMode` 열거형, `snap_to_point()`, `snap_lock()` — 스냅 시스템

**어셈블리 & FEM:**
- `cadkernel-modeling`: Assembly `solve_constraints()` (Newton-Raphson), `simulate_step()`, `export_asmt()`, `AssemblyPreferences`
- `cadkernel-modeling`: FEM `AnalysisContainer`, `ElementGeometry`, `EmBoundaryCondition`, `FluidBoundaryCondition`, `GeometricalFeature`
- `cadkernel-modeling`: FEM `heat_equation()`, `flow_equation()`, `deformation_equation()`, `electrostatic_equation()`
- `cadkernel-modeling`: FEM `apply_filter()`, `FilterFunction`, `VisualizationMode`, `purge_results()`, `create_mesh_region()`

**I/O 포맷:**
- `cadkernel-io`: VRML 가져오기/내보내기 (`vrml.rs`) — VRML97 기하 노드
- `cadkernel-io`: AMF 가져오기/내보내기 (`amf.rs`) — XML 기반 적층 제조 파일 포맷

#### FreeCAD 호환성 스프린트 2 (2026-03-24)

**Part 워크벤치 완성 (91%):**
- `cadkernel-modeling`: `face_from_wires()` — 와이어 경계에서 페이스 생성
- `cadkernel-modeling`: `explode_compound()`, `compound_filter()`, `boolean_fragments()`, `slice_to_compound()` — 컴파운드 연산
- `cadkernel-modeling`: `connect_shapes()`, `embed_shapes()`, `cutout_shapes()` — 결합 연산
- `cadkernel-modeling`: `points_from_shape()` — 형상에서 꼭짓점 추출
- `cadkernel-modeling`: `set_face_appearance()`, `FaceAppearanceMap` — 면별 외관 시스템
- `cadkernel-modeling`: `compute_attachment()`, `AttachmentMode` (6가지 모드) — 면/엣지에 객체 부착

**PartDesign 완성 (98%):**
- `cadkernel-modeling`: `additive_helix()`, `subtractive_helix()` — 나선형 스위프 연산
- `cadkernel-modeling`: `additive_ellipsoid()`, `subtractive_ellipsoid()` — 타원체 프리미티브
- `cadkernel-modeling`: `additive_prism()`, `subtractive_prism()` — 프리즘 프리미티브
- `cadkernel-modeling`: `additive_wedge()`, `subtractive_wedge()` — 쐐기 프리미티브
- `cadkernel-modeling`: `subtractive_loft()`, `subtractive_pipe()` — 감산 복합 연산

**스케처 완성 (89%):**
- `cadkernel-sketch`: `SketchEllipticalArc`, `SketchHyperbolicArc`, `SketchParabolicArc` — 3개 신규 엔티티
- `cadkernel-sketch`: `add_periodic_bspline_from_knots()` — 주기적 B-스플라인 생성
- `cadkernel-sketch`: `SketchDisplayOptions` — 13가지 시각적 도우미 토글
- `cadkernel-sketch`: `SketchGrid`, `SketchSnap` — 그리드 및 스냅 시스템
- `cadkernel-sketch`: `align_view_to_sketch()`, `stop_operation()`, `select_origin()` — UI 도구
- `cadkernel-sketch`: `copy_entities()`, `paste_entities()` — 클립보드 연산

**TechDraw 완성 (76%):**
- `cadkernel-io`: `SvgInsert`, `BitmapImage`, `share_view()` — 뷰 삽입/공유

**어셈블리 완성 (100%):**
- `cadkernel-modeling`: `ParallelAxes`, `PerpendicularAxes` — 평행/수직 축 구속
- `cadkernel-modeling`: 12개 조인트 타입 모두 Newton-Raphson 구속 방정식 완성
- `cadkernel-modeling`: `new_part_in_assembly()` — 어셈블리 내 새 부품 생성

**메시 워크벤치 완성 (100%):**
- `cadkernel-io`: `close_holes()`, `segmentation_best_fit()` — 구멍 닫기, 최적 분할

**Draft 워크벤치 완성 (95%):**
- `cadkernel-modeling`: `upgrade_wire()`, `downgrade_solid()`, `wire_to_bspline()` — 형상 변환
- `cadkernel-modeling`: `shape_from_text()` — 텍스트에서 형상 생성
- `cadkernel-modeling`: `DraftLayer`, `LayerManager`, `WorkingPlane`, `DraftStyle` — 레이어/작업면/스타일 관리

**I/O 포맷 완성 (89%):**
- `cadkernel-io`: `import_oca()`, `export_oca()` — OCA/GCAD 포맷 지원

#### FreeCAD 호환성 스프린트 3 (2026-03-25)

**Part 워크벤치 완성 (98%):**
- `cadkernel-modeling`: `PrimitiveParams` + `make_primitive()` — 통합 프리미티브 생성자 (enum 디스패치)
- `cadkernel-geometry`: `offset_polygon_2d_checked()` — `KernelResult` 반환하는 개선된 2D 오프셋
- `cadkernel-modeling`: `project_curves_on_surface()` — 서피스에 커브 투영 (점뿐만 아니라 커브도)
- `cadkernel-modeling`: `auto_defeaturing()` — 크기 임계값 기반 자동 소형 피처 제거
- `cadkernel-modeling`: `transformed_copy()` — 변환이 적용된 솔리드 복사본 생성

**PartDesign 완성 (100%):**
- `cadkernel-modeling`: `Body::move_object_to_body()` — Body 간 피처 이동

**스케처 완성 (96%):**
- `cadkernel-sketch`: `add_triangle()`, `add_square()`, `add_pentagon()`, `add_hexagon()`, `add_heptagon()`, `add_octagon()` — 전용 다각형 단축 래퍼
- `cadkernel-sketch`: `external_intersection()` — 외부 지오메트리 엣지와 스케치 교차
- `cadkernel-sketch`: `toggle_section_view()` + `SectionViewState` — 스케처용 토글 단면 뷰

**서피스 워크벤치 완성 (100%):**
- `cadkernel-modeling`: `coons_patch()` — 4개 경계 커브에서 쌍선형 블렌딩 서피스

**Draft 워크벤치 (96%):**
- `cadkernel-modeling`: `make_line_draft()` — Draft 워크벤치용 2점 선 생성

**FEM 워크벤치 완성 (90%):**
- `cadkernel-modeling`: `HexMesh`, `generate_hex_mesh()`, `mesh_from_shape()`, `adaptive_mesh_refinement()`, `mesh_smoothing()` — 메시 생성
- `cadkernel-modeling`: `export_mesh_abaqus()`, `export_mesh_nastran()` — 메시 내보내기 형식
- `cadkernel-modeling`: `nonlinear_static_analysis()`, `frequency_analysis()`, `buckling_analysis()` — 신규 해석 타입
- `cadkernel-modeling`: `magnetostatic_equation()`, `coupled_thermo_mechanical()`, `acoustic_equation()`, `poisson_equation()`, `diffusion_equation()` — 5개 신규 방정식
- `cadkernel-modeling`: `extract_nodal_values()`, `interpolate_to_nodes()`, `compute_error_estimate()`, `result_at_point()`, `integrate_over_surface()`, `max_min_values()`, `path_result()`, `reaction_forces()` — 후처리 함수
- `cadkernel-modeling`: `fem_summary()`, `export_fem_report()`, `check_mesh_quality_detailed()`, `check_boundary_conditions()`, `estimate_computation_time()`, `apply_element_geometry()` — 유틸리티
- `cadkernel-modeling`: `BodyLoad`, `ContactConstraint`, `InitialTemperature` — 신규 경계 조건 타입
- `cadkernel-modeling`: `BucklingResult`, `MagnetostaticResult`, `CoupledResult`, `AcousticResult`, `ScalarResult`, `ElementQuality` — 신규 결과 타입

**I/O 완성 (100%):**
- `cadkernel-io`: `import_svg()` — 7가지 요소 타입, 경로 명령, 변환, 이어-클리핑 삼각분할의 SVG 가져오기
- `cadkernel-io`: `import_pdf()` — 벡터/텍스트 추출의 PDF 가져오기 (`PdfImportResult`)
- `cadkernel-io`: `export_drawing_dxf()` — 치수, 중심선, 해칭, 리더, 텍스트가 포함된 전체 TechDraw→DXF 내보내기

### 테스트
- 총 1133개 테스트 (기존 1037개), Sprint 3에서 96개 신규 테스트
- 전체 FreeCAD 기능 호환성: 100% (576/576)

#### UI 스프린트: FreeCAD 100% 패리티 달성 (2026-03-25)

**마일스톤: 576/576 FreeCAD 기능 구현 완료 (100% 패리티)**

**뷰어 — GuiAction 시스템 확장:**
- `cadkernel-viewer`: GuiAction 열거형 ~40개에서 130개 이상으로 확장, 9개 워크벤치 전체 커버
- `cadkernel-viewer`: `app.rs`의 `process_actions()`가 130개 이상의 액션을 실제 백엔드 호출로 처리 (FEM 사면체 메시 생성, 불리언 연산, TechDraw 페이지 관리, I/O 가져오기/내보내기 등)
- `cadkernel-viewer`: `AssemblyJointType` 열거형 (13가지 조인트 타입) — 어셈블리 툴바 연동
- `cadkernel-viewer`: `FemConstraintType` 열거형 (6가지 제약 타입) — FEM 툴바 연동

**뷰어 — 9개 워크벤치 툴바 (toolbar.rs):**
- Part: 13개 프리미티브 + 3개 불리언 + shape builder + shape analysis + attachment + appearance
- PartDesign: pad/pocket/revolve/groove/hole + 추가적/감산적 프리미티브 + 피처 (fillet/chamfer/draft/shell) + body 연산 + shape binder
- Sketcher: 8개 기하 도구 + B-spline 도구 + 7개 제약 버튼 + 디스플레이 옵션 + 스케치 관리
- Mesh: 가져오기/내보내기 + 15개 메시 연산 + 분석 (곡률, 수밀성, 바운딩 박스, 면 정보)
- TechDraw: 7개 뷰 + 12개 치수 + 6개 중심선 + 8개 장식 + 서식 + 페이지 관리
- Assembly: 컴포넌트 삽입 + 13가지 조인트 타입 + 솔버 + 시뮬레이션 + DOF 분석 + 환경설정
- Draft: 10개 와이어 생성 + 8개 수정 + 5개 배열 패턴 + 3개 주석 + 스냅 + 쿼리 + 레이어 관리
- Surface: ruled surface + filling + sections + extend + pipe + coons patch + curve on mesh
- FEM: 4개 메시 타입 + 6개 재료 프리셋 + 8개 경계조건 + 6가지 해석 타입 + 9개 방정식 + 후처리 + 내보내기

**뷰어 — 생성 다이얼로그 (dialogs.rs):**
- 13개 프리미티브 생성 다이얼로그 (매개변수 입력)
- 불리언 연산 다이얼로그 (두 번째 피연산자 매개변수)
- Part 연산 다이얼로그 (미러/스케일/셸/필렛/챔퍼/패턴/두께/오프셋/단면)
- FEM 해석 설정 다이얼로그
- 어셈블리 조인트 구성 다이얼로그

**뷰어 — 스케치 UI (sketch_ui.rs):**
- 전체 제약조건 시각화 오버레이 (24가지 제약 타입별 적절한 인디케이터 렌더링)
- 간격 및 세분화 설정 가능한 스케치 그리드
- 스냅 인디케이터 시스템 (7가지 스냅 타입: 끝점, 중점, 중심, 그리드, 교차, 수직, 최근접)
- B-spline 제어 다각형 및 노트 다중도 표시
- 보조선 기하 시각적 구분

**뷰어 — 컨텍스트 메뉴 (context_menu.rs):**
- 오브젝트 컨텍스트 메뉴: 선택, 삭제, 복제, 변환, 측정, 지오메트리 검사, 내보내기, 숨기기/표시
- 뷰포트 컨텍스트 메뉴: 표준 뷰, 디스플레이 모드, 전체 맞춤, 카메라 초기화, 전체 선택/해제

**뷰어 — 앱 통합 (app.rs):**
- 130개 이상의 GuiAction을 백엔드 크레이트 호출에 연결하는 완전한 `process_actions()` 구현
- FEM 연동: 사면체 메시 생성, 정적/모달/열/주파수/좌굴 해석 디스패치
- 어셈블리 연동: 제약 해석, DOF 분석, 시뮬레이션 스텝, 내보내기
- TechDraw 연동: 뷰 생성, 치수 배치, 중심선/장식 도구, SVG/DXF 내보내기
- Draft 연동: 와이어 생성, 수정 도구, 배열 패턴, 스냅 시스템
- Surface 연동: ruled surface, filling, sections, coons patch 생성
- I/O 연동: 15개 이상의 파일 포맷 가져오기/내보내기 (리포트 패널 로깅 포함)

#### UI 폴리시 스프린트: 전문 CAD 품질 달성 (2026-03-25)

**테마 시스템 (theme.rs):**
- `CadTheme` 구조체: 30개 이상의 색상/간격/타이포그래피 필드, Dark/Light 프리셋
- `ThemeMode` (Dark/Light), `UiDensity` (Compact/Normal/Spacious) 열거형
- `apply_to_egui()`: 완전한 egui visuals + style + text styles 통합
- `object_type_icon()`: `CreationParams` 변형에 매핑된 15개 유니코드 아이콘
- 테마 색상 상수: `COLOR_INFO`, `COLOR_WARN`, `COLOR_ERROR`, `COLOR_SUCCESS`, `COLOR_ACCENT`, `COLOR_DIM`

**벡터 아이콘 툴바 (toolbar.rs):**
- 150개 이상의 `ToolIcon` 열거형 변형, `draw_icon()`으로 `egui::Painter` 벡터 도형 렌더링
- `icon_button()` (28×28 호버 반응형), `icon_toggle()`, `toolbar_separator()`
- 액센트 색상 언더라인이 있는 스타일된 워크벤치 탭
- 워크벤치별 시각적 구분자가 있는 그룹화된 툴바 섹션

**계층적 모델 트리 (tree.rs):**
- `EntityIcon` 열거형 (14가지 타입: Solid, Face, Edge, Vertex, Sketch, Extrude, Revolve 등)
- `TreeNode` 계층구조: 생성 이력(`CreationParams`)에서 자동 구축
- 트리 가이드 라인, 접기/펼치기 노드, 검색/필터 (클리어 버튼 포함)
- 인라인 이름 변경 (더블클릭), 드래그앤드롭 재정렬 지원
- 아이콘, 이름, 가시성 토글이 있는 전문적 행 렌더링
- 우클릭 컨텍스트 메뉴 (이름 변경, 삭제, 복제, 위/아래 이동)

**향상된 다이얼로그 (dialogs.rs):**
- 공유 헬퍼: `dialog_section()`, `param_field()`, `validation_error()`, `button_bar()`
- 빨간색 오류 메시지와 입력 검증 (예: "반지름은 0보다 커야 합니다")
- "mm" 단위 라벨, 도움말 텍스트, 28개 전체 다이얼로그에 기본값 버튼
- 일관된 3열 그리드 레이아웃 (라벨 | DragValue | 단위)

**속성 패널 (properties.rs):**
- 액센트 색상 섹션 헤더
- 3열 매개변수 그리드 (라벨 | DragValue | "mm")
- 뷰 탭: 8개 프리셋이 있는 색상 선택기, 투명도 슬라이더
- 오브젝트 수, 면/엣지 통계가 있는 씬 개요
- ID, 타입, 생성 매개변수가 있는 오브젝트 정보 표시

**상태 바 (status_bar.rs):**
- 왼쪽: 마우스 좌표 (고정폭 글꼴)
- 중앙: 활성 도구 이름 + 힌트 (스케치 모드) / 선택 모드 (일반 모드)
- 오른쪽: 씬 통계 (오브젝트, 면, 엣지) + FPS 카운터

**리포트 패널 (report.rs):**
- 번호 매김 타임스탬프, 심각도 필터 토글 (Info/Warn/Error)
- 심각도별 카운트 배지
- 긴 메시지 접기 (>80자), 클리어 버튼

**뷰포트 오버레이 (overlays.rs):**
- `draw_origin_overlay()`: 색상 화살표와 축 라벨이 있는 XYZ 축
- `draw_grid_3d_overlay()`: 거리 기반 페이드가 있는 주/보조 그리드 라인
- `draw_measurement_overlay()`: 선택한 2점 사이의 거리 및 각도
- `draw_snap_overlay()`: 정점/그리드 스냅 하이라이트 인디케이터
- `draw_sketch_plane_preview()`: 반투명 평면 시각화

**컨텍스트 메뉴 (context_menu.rs):**
- 향상된 뷰포트 메뉴: 표준 뷰, 디스플레이 모드, 전체 맞춤, 오버레이 토글
- 향상된 오브젝트 메뉴: 이름 변경, 삭제, 복제, 변환, 피처 재정렬
- 면/엣지 컨텍스트 메뉴: 면에 스케치 생성, 필렛/챔퍼 엣지

**내비게이션 설정 (nav.rs):**
- `theme_mode`, `ui_density` 필드: 영구 테마 설정
- `show_origin`, `show_grid_3d`, `grid_3d_spacing`: 뷰포트 오버레이 제어

**렌더 헬퍼 (render.rs):**
- `selection_color()`, `preselection_color()` 헬퍼 함수

#### UI 폴리시 스프린트 2: 전문 인터랙션 품질 (2026-03-25)

**모델 트리 개선 (tree.rs):**
- 인라인 가시성 눈 아이콘 (우측 정렬) — 클릭하여 가시성 토글
- 팁 마커 (`\u{25B8}` 액센트 색상) — 마지막 오브젝트 및 마지막 이력 레코드 표시
- 억제 디밍: "suppressed" 포함 자식 노드를 흐린 색상으로 렌더링
- 눈 아이콘 겹침 방지를 위한 이름 변경 TextEdit 폭 조정

**툴바 활성 도구 하이라이트 (toolbar.rs):**
- `icon_button_active()` / `icon_button_ex()`: 파란색 배경 + 2px 액센트 하단 테두리
- 선택 모드 버튼 (Solid/Face/Edge/Vertex) `gui.selection_mode` 기반 하이라이트
- Part 툴바 프리미티브 버튼: 태스크 패널의 `ActiveTask` 기반 하이라이트
- 스케치 도구: `sketch_mode.tool` 기반 하이라이트
- `icon_toggle()`: 선택 시 하단 액센트 테두리 추가

**키보드 단축키 다이얼로그 (dialogs.rs):**
- 5개 섹션 단축키 레퍼런스 창: 내비게이션, 표준 뷰, 디스플레이 모드, 편집, 파일
- 고정폭 키 라벨, 줄무늬 그리드 행, 액센트 색상 섹션 헤더
- Help > Keyboard Shortcuts 메뉴에서 접근

**설정 다이얼로그 개선 (dialogs.rs):**
- 상단에 "외관" 섹션 추가: 테마 토글 (Dark/Light), UI 밀도 (Compact/Normal/Spacious)
- 테마/밀도 변경 즉시 적용 (`theme_applied` 플래그 리셋)

**정보 다이얼로그 개선 (dialogs.rs):**
- 액센트 색상 중앙 정렬 로고, 부제목, 줄무늬 정보 그리드
- 버전, 라이선스, 저자, 렌더러, 커널 정보, 워크벤치 목록, I/O 포맷, 테스트 수 표시

**패널 레이아웃 개선 (mod.rs):**
- ComboView 좌측 패널: 리사이즈 가능 폭 (200-450px), 테두리 스트로크, 트리/속성 간 미묘한 액센트 구분선

#### UI 폴리시 스프린트 3: 설정 & 렌더링 (2026-03-25)

**투명 패널 수정 (render.rs):**
- Surface 구성에 `wgpu::CompositeAlphaMode::Opaque` 설정 — Linux 컴포지터 블렌딩으로 인한 3D 뷰포트가 UI 패널을 투과하는 현상 해결

**환경 설정 다이얼로그 재설계 (dialogs.rs):**
- 평면 스크롤 레이아웃을 탭 내비게이션 사이드바로 교체 (일반, 디스플레이, 내비게이션, 외관, 조명)
- 일반 탭: 단위 시스템 (mm/cm/m/in/ft), 소수점 자릿수, 자동 저장 토글 + 간격, 최근 파일 제한, 삭제 확인
- 디스플레이 탭: 배경 그라디언트 프리셋 (Dark/Medium/Light/Blueprint) + 실시간 파이프라인 재빌드, 뷰포트 오버레이 (축/원점/그리드/FPS), 카메라 기본값, 선택/사전선택 색상 선택기, 테셀레이션 품질 슬라이더
- 내비게이션 탭: 마우스 스타일 프리셋, 감도 슬라이더, 애니메이션 제어, View Cube 설정
- 외관 탭: 테마 (Dark/Light), UI 밀도 (Compact/Normal/Spacious) + 설명
- 조명 탭: 활성화 토글, 강도 슬라이더, 방향광 XYZ 제어
- 사이드바에 "전체 초기화" 버튼

**동적 배경 그라디언트 (render.rs + nav.rs):**
- `BgPreset` 열거형 (4가지 변형: Dark, Medium, Light, Blueprint) + `label()` 메서드
- `GpuState::update_bg_preset()`를 통한 런타임 셰이더 재생성 배경 프리셋 시스템
- 애플리케이션 재시작 없이 실시간 그라디언트 전환

**NavConfig 확장 (nav.rs):**
- 신규 필드: `unit_system`, `decimal_places`, `bg_preset`, `selection_color`, `preselection_color`, `tessellation_segments`, `auto_save_enabled`, `auto_save_interval_secs`, `recent_files_max`, `confirm_delete`
- `UnitSystem` 열거형 (5가지 변형: Millimeter, Centimeter, Meter, Inch, Foot) + `label()`/`long_label()`
- `NavConfig`에 `Clone` derive 추가 — `save_settings()`를 `nav.clone()` 방식으로 간소화

#### V11: 뷰어 UI 확장 (2026-03-25)
- 작업 패널: 5 → 13개 프리미티브 (Tube, Prism, Wedge, Ellipsoid, Helix) + PartDesign (Pad, Pocket, Hole) 인라인 편집
- 확장 프리미티브 전체 인라인 작업 패널 전환 (팝업 다이얼로그 대체), 실시간 3D 프리뷰
- 스케처: B-spline 도구 (변환, 차수+/-, 노트 삽입), Split/Mirror/External Projection/Carbon Copy, Block/HDist/VDist 구속조건
- 메뉴: 거리 측정 연결, 워크벤치 전환 메뉴, Macro 메뉴 비활성화 (계획 표시), Origin/Grid3D 토글
- 원점 축: wgpu 전용 렌더링 (egui 오버레이 제거), Z축 전체 길이, show_origin 독립 토글
- 그리드 오버레이 뷰포트 영역 클리핑 (패널 관통 방지)

#### V11 UI 오버홀: 완전한 액션 처리 & 폴리시 (2026-03-25)

**GuiAction 처리 완성 (app.rs):**
- 130개 이상의 모든 `GuiAction` 변형에 백엔드 크레이트 호출을 포함한 완전한 `process_actions()` 핸들러 구현
- Part 연산: Join (connect/embed/cutout), compound 연산 (fragments/slice/filter/explode), auto-defeaturing, transformed copy, project curves, Coons patch
- PartDesign: Pad/Pocket/Groove/Hole 스케치 연동, additive/subtractive loft/pipe, sprocket, shaft design, involute gear, shape binder, suppress/set tip/move feature
- Assembly: 컴포넌트 삽입, 13개 조인트 타입, 제약 해석 (Newton-Raphson), DOF 분석, 분해도, BOM, 시뮬레이션 스텝
- Draft: 10개 와이어 생성 + 8개 수정 + 5개 배열 패턴 + 주석 + 스냅 시스템 + 레이어 관리 + upgrade/downgrade
- Surface: ruled surface, filling, sections, extend, pipe, Coons patch, curve on mesh
- FEM: tet/hex 메시 생성, 6개 재료 프리셋, 8개 경계조건, 6가지 해석 타입 (static/nonlinear/frequency/buckling/modal/thermal), 9개 방정식, 후처리 (응력/변형률 텐서, 주응력, 반력), Abaqus/Nastran 내보내기
- TechDraw: 페이지 관리, 7개 뷰 타입, 12개 치수 타입, 중심선/장식, SVG/DXF/PDF 내보내기
- I/O: 15개 이상 포맷 가져오기/내보내기 (SVG, glTF, 3MF, DAE, DWG, VRML, AMF, OCA, PDF) — 리포트 패널 로깅 포함
- 모든 핸들러에 리포트 로깅 적용으로 완전한 작업 추적 가능

**속성 패널 개선 (properties.rs):**
- Data 탭: 오브젝트 이름, 편집 가능한 DragValue 필드의 생성 매개변수, 토폴로지 통계 (솔리드/셸/면/엣지/정점), 메시 정보, 질량 속성
- View 탭: 8개 프리셋이 있는 색상 선택기, 투명도 슬라이더, 가시성 토글
- 씬 개요: 오브젝트 수, 집계된 면/엣지 통계
- 변환 편집: 이동 (dx/dy/dz), 회전 (축 + 각도), 스케일 (균일 비율) — `MoveObject`/`RotateObject`/`ScaleObjectUniform` 액션 연동
- 파라메트릭 리빌드: DragValue 변경 시 `RebuildObject` 트리거로 실시간 매개변수 편집

**컨텍스트 메뉴 확장 (context_menu.rs):**
- 오브젝트 메뉴: 선택, 복제, 이름 변경, 숨기기/표시, 색상 설정 (8개 프리셋), 변환 하위메뉴 (이동/회전/스케일 프리셋), 측정, 지오메트리 검사, 연산 (미러/셸/필렛/챔퍼/패턴), 다른 형식으로 내보내기 (9개 포맷), 삭제
- 뷰포트 메뉴: 전체 맞춤, 카메라 초기화, 표준 뷰, 디스플레이 모드, 그리드/투영/원점/3D 그리드/측정 토글, 전체 선택/해제, 생성 하위메뉴 (5개 프리미티브 + 3개 스케치 평면), 전체 표시/숨기기
- 트리 메뉴: 오브젝트 메뉴 확장 + PartDesign 피처 연산 (억제, 팁 설정, 위/아래 이동)
- 면/엣지 메뉴: 면에 스케치 생성, 필렛/챔퍼 엣지, 측정, 지오메트리 검사

**워크벤치 툴바 연결 (toolbar.rs):**
- 9개 워크벤치 툴바 전체 `GuiAction` 디스패치를 통한 백엔드 연결 완성
- Part: 13개 프리미티브, 3개 불리언, shape builder, 변환, join/compound 연산, mirror/scale/shell/fillet/chamfer/pattern/thickness/offset/section, attachment, appearance, 분석
- PartDesign: pad/pocket/revolve/groove/hole, 10개 additive/subtractive 프리미티브, 피처, body 연산, shape binder
- Sketcher: 8개 기하 도구, B-spline 도구, 7개 구속, 디스플레이 옵션, 스케치 관리, external projection, carbon copy
- Mesh: 가져오기/내보내기, 15개 연산, 분석 (곡률, 수밀성, 바운딩 박스, 면 정보, 정다면체, UV 전개)
- TechDraw: 7개 뷰, 12개 치수, 6개 중심선, 8개 장식, 서식, 페이지 관리
- Assembly: 컴포넌트, 13개 조인트, 솔버, 시뮬레이션, DOF 분석, 환경설정, 내보내기
- Draft: 10개 생성, 8개 수정, 5개 배열, 3개 주석, 스냅, 쿼리, 레이어
- Surface: 7개 서피스 연산
- FEM: 4개 메시 타입, 6개 재료, 8개 경계조건, 6개 해석, 9개 방정식, 후처리, 내보내기

**상태 바 개선 (status_bar.rs):**
- 좌측: 마우스 월드 좌표 (고정폭, X/Y/Z)
- 중앙 (스케치 모드): 활성 도구 이름 + 힌트, DOF 상태 (완전/과소 구속 색상 표시), 스냅/그리드 인디케이터
- 중앙 (일반 모드): 활성 워크벤치 표시, 선택 모드 (Solid/Face/Edge/Vertex)
- 우측: 씬 통계 (오브젝트 가시/전체, 면, 엣지), 투영 모드 (Persp/Ortho), FPS 카운터

**리포트 패널 개선 (report.rs):**
- Report/Python Console 탭 및 개별 Clear 버튼
- 심각도 필터: 레벨별 (Info/Warn/Error) 카운트 배지 및 색상 코딩
- Console: 이력이 있는 명령 입력, `>>>` 프롬프트 (PyO3 백엔드 플레이스홀더)
- 130개 이상 액션 핸들러 전체에 리포트 패널 로깅 적용

**모델 트리 개선 (tree.rs):**
- `EntityIcon` 열거형 (14가지 타입) — 엔티티 타입별 14x14 프로시저럴 벡터 아이콘
- `TreeNode` 계층구조 — `CreationParams` 생성 이력에서 자동 구축
- 트리 가이드 라인, 접기/펼치기 노드, 검색/필터 (클리어 버튼 포함)
- 인라인 이름 변경 (더블클릭), 드래그앤드롭 재정렬 지원
- 가시성 눈 아이콘 (우측 정렬), 팁 마커 (액센트 색상), 억제 디밍
- 우클릭 컨텍스트 메뉴 (이름 변경, 삭제, 복제, 위/아래 이동, 억제, 팁 설정)

#### V15 프로페셔널 인터랙션: 3D 기즈모, 클립 플레인, 단축키 & 월드 좌표 (2026-03-30)

**3D 변환 기즈모 (overlays.rs + mod.rs):**
- 선택된 오브젝트 중심에 인터랙티브 변환 기즈모: 이동(XYZ 화살표), 회전(XYZ 호), 스케일(XYZ 사각형)
- `GizmoMode` 열거형, 축별 호버 하이라이팅, 모드 라벨

**클립 플레인 / 단면 뷰 (render.rs + nav.rs):**
- GPU 클립 플레인: WGSL 프래그먼트 셰이더의 `clip_params` vec4 유니폼
- 클립 면 뒤의 프래그먼트 폐기, 절단면 주황색 에지 하이라이트
- NavConfig: `clip_enabled`, `clip_plane_normal`, `clip_plane_offset`

**키보드 단축키 패널 (app.rs):**
- `?` / F1으로 5개 카테고리 단축키 창 토글

**마우스 월드 좌표 (app.rs + status_bar.rs):**
- CursorMoved 시 Z=0 지면 평면 레이 캐스트, 상태바에 표시

#### V14 인터랙티브 선택: 박스 선택, 선택 게이트 & 내비게이션 수정 (2026-03-30)

**박스 선택 / 러버밴드:**
- 뷰포트에서 좌클릭 드래그로 선택 사각형 그리기 (수정자 키 불필요)
- 좌→우 드래그 = 윈도우 선택 (파란색, 실선) — 완전히 포함된 오브젝트
- 우→좌 드래그 = 크로싱 선택 (녹색, 점선) — 겹치는 오브젝트
- Ctrl+드래그로 선택 추가 (누적 박스 선택)
- `object_screen_aabb()`로 각 가시 오브젝트의 스크린 공간 AABB 투영
- `overlays::draw_rubber_band()`에서 방향별 색상의 러버밴드 오버레이

**선택 게이트 / 필터:**
- SelectionMode (Solid/Face/Edge/Vertex)가 `try_pick_entity()`에 연결됨
- 활성 모드에 따라 `selected_entity` 설정 (Solid → SolidData, Face → FaceData 등)
- 상태 메시지에 모드 레이블 표시 ("[Face]", "[Edge]", "[Vertex]")

**내비게이션 수정:**
- FreeCADGesture: 좌클릭 드래그가 더 이상 궤도 회전하지 않음 (실제 FreeCAD는 중간 버튼 사용)
- 5개 내비게이션 스타일 모두 일관성 확보: 좌클릭 드래그 = 박스 선택

#### V13 FreeCAD 패리티: 프로페셔널 UI 대개편 (2026-03-30)

**프리셀렉션 호버 하이라이트 (render.rs):**
- Uniforms 구조체 및 WGSL 셰이더에 `hover_params` vec4 유니폼 추가
- GPU 측 호버 블렌딩: `PRESELECT_COLOR` (연한 시안), `PRESELECT_STRENGTH` (0.3)
- 프래그먼트 셰이더에서 오브젝트별 호버 ID 비교
- GpuState에 `hover_object_id` 필드 — 매 프레임 커서 기반 프리셀렉션

**계층형 모델 트리 (tree.rs + scene.rs):**
- SceneObject 확장: `parent_id`, `is_body`, `is_tip`, `suppressed`, `has_error`, `needs_recompute`
- Scene 메서드: `children_of()`, `root_objects()` 계층 탐색
- Body > Feature 중첩 렌더링 (접기/펼치기)
- 14종 절차적 엔티티 아이콘 (Solid, Face, Edge, Vertex, Sketch, Extrude 등)
- Tip 마커 (녹색 화살표), 억제 흐리게, 오류/재계산 상태 표시
- 드래그앤드롭 피처 재정렬 힌트, Ctrl/Shift 다중 선택

**향상된 프로퍼티 패널 (properties.rs):**
- `collapsible_group()` 헬퍼로 접을 수 있는 속성 섹션
- Placement 편집기: 위치 (X/Y/Z) + 회전 (X/Y/Z) DragValue 컨트롤
- 계산된 속성: 부피, 표면적, 무게중심, 바운딩 박스
- View 탭: 디스플레이 모드 선택, 투명도 슬라이더, 색상 피커
- 속성 검색/필터 바

**플라이아웃 툴바 시스템 (toolbar.rs + context_menu.rs):**
- `flyout_button()` / `flyout_button_with_active()` 그룹 도구 드롭다운 버튼
- `FlyoutEntry` 타입: (ToolIcon, title, description, shortcut)
- egui 메모리로 그룹별 마지막 사용 도구 기억
- Part/PartDesign 툴바 플라이아웃 그룹화
- 컨텍스트 메뉴: 하위 요소 메뉴, 변환 서브메뉴, 내보내기 서브메뉴, 색상 피커

**빠른 측정 & 상태 표시줄 (status_bar.rs + overlays.rs):**
- 선택 모드 표시 (Solid/Face/Edge/Vertex)
- 프리셀렉션 정보, 빠른 측정 자동 치수 표시
- 내비게이션 모드 표시, 스냅/그리드 토글, 단위계 표시

**프로페셔널 테마 시스템 (theme.rs):**
- `CadTheme` 구조체: 25+ 색상 상수 (accent, selection, preselection, error 등)
- Dark/Light 테마 프리셋, `UiDensity` 열거형 (Compact/Normal/Spacious)
- 전체 UI 패널에 테마 적용 색상 사용

**스케치 UI 향상 (sketch_ui.rs):**
- 11종 스케치 도구: Select, Line, Rectangle, Circle, Arc, Point, Ellipse, Polyline, Slot, BSpline, Polygon
- 그리기 중 스케치 평면 커서 십자선
- 향상된 구속 시각화 (치수선, 색상 코드 표시)
- 컨스트럭션 모드/그리드/스냅 토글 상태 배너 표시

**확장된 워크벤치 (mod.rs + menu.rs + dialogs.rs):**
- 9개 워크벤치: Part, PartDesign, Sketcher, Mesh, TechDraw, Assembly, Draft, Surface, FEM
- SelectionMode 열거형 (Solid/Face/Edge/Vertex) — 하위 요소 피킹
- 13종 AssemblyJointType, 6종 FemConstraintType
- 5종 NavStyle 프리셋, 5종 UnitSystem, 4종 BgPreset

#### V12 핵심 UI 대개편: 통합 작업 시스템, CAD 임포트 & 툴바 상태 (2026-03-27)

**통합 작업 시스템 (이중 UI 제거):**
- 모든 메뉴/컨텍스트 메뉴의 프리미티브 생성이 레거시 팝업 대화상자 대신 ActiveTask 인라인 패널로 전환
- 레거시 `draw_create_dialogs()` — `gui.active_task.is_none()` 게이팅으로 팝업/패널 충돌 제거
- 메뉴, 컨텍스트 메뉴, 툴바 코드에서 `show_create_*` 플래그 할당 완전 제거
- File > Create 메뉴, 뷰포트 우클릭 > Create, 툴바 모두 통합 ActiveTask 플로우 사용

**CAD 포맷 임포트 수정 (STEP/IGES/BREP/DXF/PLY/3MF):**
- `load_mesh_file()` — 모든 지원 포맷을 적절한 임포터로 라우팅
- `FileLoadResult` 열거형 — BRep 모델 임포트 (STEP/IGES/BREP)와 메시 임포트 (DXF/PLY/3MF/STL/OBJ) 구분
- BRep 임포트: `import_step()`, `import_iges()`, `import_brep()` → 테셀레이션 → `scene.add_object()`
- 메시 임포트: `import_dxf()`, `import_ply()`, `import_3mf()` → `scene.add_mesh_object()`
- 백그라운드 스레드 로딩 모든 포맷에 유지

**툴바 비활성화 상태:**
- `icon_button_disabled()` — 액션 불가 시 회색 비인터랙티브 버튼 렌더링
- `ToolbarContext` 구조체 — 씬 상태 전달 (has_selection, has_objects, can_undo, can_redo, in_sketch)
- 객체 미선택 시 Part/PartDesign 작업 비활성화
- Undo/Redo 스택 비어있을 때 버튼 비활성화
- `gated_button!` 매크로로 DRY 패턴

#### V11 인터랙티브 작업 패널 확장 (2026-03-26)

**인터랙티브 작업 패널 (task_panel.rs):**
- 35개 `ActiveTask` 변형: 프리미티브 (Box/Cylinder/Sphere/Cone/Torus/Tube/Prism/Wedge/Ellipsoid/Helix), PartDesign (Pad/Pocket/Hole/Groove/Fillet/Chamfer/Shell/Mirror/Pattern/Sprocket/InvoluteGear), Draft (Line/Circle/Rectangle/Polygon/Arc/Ellipse), Surface (Pipe/Ruled), FEM (Mesh), Boolean (Union/Subtract/Intersect), Scale
- 각 변형은 타입별 매개변수 + `preview_id: Option<ObjectId>`를 저장하여 실시간 3D 프리뷰 제공
- 툴바 버튼이 즉시 실행 대신 인라인 작업 패널을 열어 OK 확인, Cancel/Escape 취소 방식으로 전환
- `draw_task_panel()`이 변형별 매개변수 편집기를 DragValue 슬라이더 + 단위 라벨로 렌더링
- `app.rs`의 `apply_task()`가 확정된 작업을 백엔드 크레이트 호출로 디스패치, 리포트 로깅 포함

**툴바 연결 (toolbar.rs):**
- 9개 전체 워크벤치 툴바가 `GuiAction::Create*` 즉시 디스패치에서 `GuiAction::StartTask(ActiveTask::*)` 패턴으로 전환
- 활성 작업 하이라이트: 해당 `ActiveTask` 변형이 활성화된 툴바 버튼에 파란색 액센트 표시
- Part/PartDesign/Draft/Surface/FEM 프리미티브 버튼 전체가 작업 패널 흐름 사용

#### V11 심층 개편: 전체 액션 연결, 메뉴 & 스케치 완성 (2026-03-26)

**GuiAction 처리 (app.rs):**
- 200개 이상의 `GuiAction` 변형에 대해 백엔드 크레이트 호출을 포함한 실제 `process_actions()` 핸들러 완성
- 스텁 핸들러를 실제 구현으로 교체: Draft (프리미티브를 통한 라인/원/호/타원/사각형/다각형/점 생성), Assembly (조인트 생성, 구속 풀기, DOF 분석, 분해도, BOM), FEM (tet/hex 메시 생성, 정적/모달/열/좌굴/비선형 해석, 재료 할당, 후처리), Surface (filling/boundary/sections/extend/blend/pipe/coons)
- TechDraw: 페이지 관리, 단면/상세/절단 뷰, 치수 (선형/반지름/지름/각도/호 길이/면적), 주석 (텍스트/리치 텍스트/벌룬/리더/용접/표면 마감), 중심선, SVG/DXF/PDF 내보내기
- 스케치 구속 핸들러 수정: `arc.start`를 `arc.start_point`로 수정, 원 반지름 구속에 중심점 사용
- Draft 프리미티브 시각화: 얇은 실린더 라인, 평면 실린더 원, 토러스 호, 타원체, 박스 사각형, 프리즘 다각형, 구 포인트

**워크벤치별 메뉴 (menu.rs):**
- `gui.active_workbench` 기반 동적 워크벤치별 메뉴 그룹 9개
- Part 메뉴: 13개 프리미티브, 3개 불리언, join/compound 연산, mirror/scale/shell/fillet/chamfer/pattern/thickness/offset/section, shape builder, 변환, attachment, appearance, 분석
- PartDesign 메뉴: pad/pocket/revolve/groove/hole, additive/subtractive 프리미티브, 피처, body 연산, shape binder, sprocket/shaft/gear
- Sketcher 메뉴: 8개 기하 도구, B-spline 도구, 구속, 디스플레이 옵션, 스케치 관리, external projection, carbon copy
- Mesh 메뉴: 가져오기/내보내기, 15개 연산, 분석 도구
- TechDraw 메뉴: 뷰, 치수, 중심선, 장식, 서식, 페이지 관리
- Assembly 메뉴: 컴포넌트, 13개 조인트 타입, 솔버, 시뮬레이션, DOF 분석
- Draft 메뉴: 12개 생성, 8개 수정, 5개 배열, 주석, 스냅, 레이어, upgrade/downgrade
- Surface 메뉴: 7개 서피스 연산
- FEM 메뉴: 메시, 재료, 경계조건, 해석, 방정식, 후처리

**스케치 구속 시각화 (sketch_ui.rs):**
- 24개 전체 구속 타입에 대한 치수 라벨 포함 구속 렌더링 개선
- 구속 인디케이터: 기하 구속에 H/V/P/T/E/S/F/B 기호
- Distance/Length/Angle/Radius/Diameter 구속: 구속된 엔티티 근처에 수치 표시
- HorizontalDistance/VerticalDistance 구속: 방향 화살표 렌더링
- 구속 색상 코딩: 만족 (녹색) vs 미만족 (빨강) 시각 피드백
- 스케치 그리드 오버레이 (간격 및 세분화 설정 가능)
- 스냅 인디케이터 시스템: 7개 스냅 타입 (끝점, 중점, 중심, 그리드, 교차점, 수직, 근접)
- B-스플라인 제어 다각형 및 노트 다중도 표시
- 구성 기하 시각 구분 (점선)

**컨텍스트 메뉴 확장 (context_menu.rs):**
- 활성 워크벤치별 관련 연산을 표시하는 워크벤치 인식 컨텍스트 메뉴
- 9개 워크벤치 전체 아이콘 버튼에 툴바 툴팁 추가

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
