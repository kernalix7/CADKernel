# CADKernel 기여 가이드

[English](../CONTRIBUTING.md) | **한국어**

CADKernel에 기여해 주셔서 감사합니다!

## 아키텍처 개요

CADKernel은 Rust로 구축된 B-Rep CAD 커널이며, 9개 크레이트 워크스페이스로 구성됩니다:

| 크레이트 | 역할 |
|----------|------|
| `core` | 에러 타입 (`KernelResult`), 판별 함수, 상수 |
| `math` | Point/Vec/Mat 타입, 변환, 쿼터니언, BBox |
| `geometry` | Curve/Surface 트레이트, NURBS, 테셀레이션, BVH, 교차 |
| `topology` | 반변 B-Rep: Vertex, Edge, HalfEdge, Loop, Face, Shell, Solid |
| `modeling` | 13개 프리미티브, 피처, 불리언, 패턴, FEM, 어셈블리 |
| `sketch` | 24개 구속 조건의 2D 파라메트릭 스케치, Newton-Raphson 솔버 |
| `io` | 11개 파일 형식 (STL/OBJ/glTF/STEP/IGES/DXF/PLY/3MF/BREP/DWG/DAE) |
| `viewer` | egui + wgpu 데스크톱 GUI, 3D 뷰포트 |
| `python` | PyO3 바인딩 (기본 워크스페이스 빌드에서 제외) |

**의존성 흐름**: `core` → `math` → `geometry` → `topology` → `modeling`/`sketch` → `io` → `viewer`

상세 아키텍처는 [DEVELOPER_WIKI.ko.md](DEVELOPER_WIKI.ko.md)를 참조하세요.

## 시작하기

### 사전 요구사항
- Rust 1.85+ (edition 2024)
- CMake 3.16+
- GPU 드라이버 지원 (플랫폼별 Vulkan/Metal/DX12)

### 빌드 & 테스트
```bash
git clone https://github.com/kernalix7/CADKernel.git
cd CADKernel
cargo build --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

### GUI 실행
```bash
cargo run --release
```

## 처음 기여하기 좋은 영역

`good first issue` 라벨이 붙은 이슈를 찾아보거나, 다음 영역을 고려해 보세요:

- **새 프리미티브 추가** — `crates/modeling/src/primitives/`에서 `make_box` 같은 기존 패턴을 참고
- **I/O 형식 지원 개선** — STEP/IGES 파서에 누락된 엔티티 타입 추가
- **스케치 구속 조건 추가** — `crates/sketch/`의 솔버에 새 구속 조건 구현
- **예제 작성** — `examples/lua/`에 Lua 스크립트 또는 `examples/python/`에 Python 스크립트
- **문서 개선** — 공개 API에 문서 주석 추가, 위키 페이지 확장

## 작업 흐름

1. 저장소를 Fork 합니다
2. 기능 브랜치를 생성합니다: `git checkout -b feature/my-change`
3. 아래 코딩 규칙을 따라 수정합니다
4. 전체 검증을 실행합니다: `cargo build --workspace && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace`
5. Push 후 Pull Request를 생성합니다

## 코딩 규칙

- 모든 공개 API는 `KernelResult<T>` 반환 — 사용자 경로에서 절대 panic 금지
- 기하 연산에 `f64` 사용
- 토폴로지 엔티티 참조에 `Handle<T>` 사용 (세대 인덱스)
- 영속적 네이밍에 `Tag` 사용 (`Tag::generated(EntityKind, OperationId, local_index)`)
- 기하 생성자는 매개변수 검증 (radius > 0, segments >= 3 등)
- 경고 제로: CI에서 clippy strict 모드가 강제됩니다

## Pull Request 체크리스트

- [ ] 변경 범위와 목적이 명확한가?
- [ ] 필요한 테스트를 추가/갱신했는가?
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`를 통과하는가?
- [ ] `cargo test --workspace`를 통과하는가?
- [ ] 공개 API에 문서 주석이 있는가?
- [ ] 동작 변경 시 README/문서를 갱신했는가?

## 커밋 메시지 규칙

[Conventional Commits](https://www.conventionalcommits.org/)를 사용합니다:
- `feat:` 새 기능
- `fix:` 버그 수정
- `docs:` 문서 변경
- `refactor:` 동작 변경 없는 구조 개선
- `test:` 테스트 변경
- `chore:` 유지보수 작업

## 보안

보안 이슈는 [SECURITY.ko.md](SECURITY.ko.md)의 제보 절차를 따라 주세요.
