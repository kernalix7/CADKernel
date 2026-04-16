# CADKernel 기여 가이드

[English](../CONTRIBUTING.md) | **한국어**

CADKernel에 기여해 주셔서 감사합니다!

## 아키텍처 개요

CADKernel은 Rust로 작성된 B-Rep CAD 커널이며, 9개 크레이트 워크스페이스로 구성됩니다.

**의존성 흐름**: `core` → `math` → `geometry` → `topology` → `modeling` / `sketch` → `io` → `viewer`

| 크레이트 | 역할 |
|----------|------|
| `core` | `KernelResult<T>`, `KernelError`, 판별 함수, 상수 |
| `math` | `Vec2/3`, `Point2/3`, `Mat3/4`, `Transform`, `Quaternion`, `BBox` |
| `geometry` | Curve/Surface 트레이트, NURBS, 테셀레이션, BVH, 교차 계산 |
| `topology` | 반모서리 B-Rep: `Vertex`, `Edge`, `HalfEdge`, `Loop`, `Face`, `Shell`, `Solid` |
| `modeling` | 13개 프리미티브, 피처, 불리언, 패턴, 플러그인 API, quick API |
| `sketch` | 2D 파라메트릭 스케치, 24개 구속 조건, Newton-Raphson 솔버 |
| `io` | 11개 파일 형식: STL, OBJ, glTF, STEP, IGES, DXF, PLY, 3MF, BREP, DWG, DAE |
| `viewer` | egui 0.31 + wgpu 24 데스크톱 GUI, 3D 뷰포트, Lua 스크립팅 콘솔 |
| `python` | PyO3 바인딩 (기본 워크스페이스 빌드에서 제외) |

전체 아키텍처 문서는 [DEVELOPER_WIKI.ko.md](DEVELOPER_WIKI.ko.md)를 참조하세요.

## 개발 환경 설정

### 사전 요구사항

- Rust 1.85+ (edition 2024, MSRV 1.85)
- CMake 3.16+ (일부 기하 의존성에 필요)
- Vulkan, Metal, 또는 DX12를 지원하는 GPU 드라이버

### 클론 및 빌드

```bash
git clone https://github.com/kernalix7/CADKernel.git
cd CADKernel
cargo build --workspace
```

### 전체 검증 실행

커밋 또는 PR 전에 세 가지 검사를 모두 통과해야 합니다:

```bash
cargo build --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

### GUI 실행

```bash
cargo run --release
```

### Python 바인딩 (선택 사항)

`python` 크레이트는 기본 워크스페이스 빌드에서 제외됩니다. 별도로 빌드 및 테스트하려면:

```bash
cd crates/python
PYO3_PYTHON=/usr/bin/python3 cargo build
cargo test --manifest-path crates/python/Cargo.toml
```

## 코딩 규칙

### 에러 처리

- 모든 공개 API는 `KernelResult<T>` 반환 — 사용자 경로에서 절대 panic 금지
- 에러에 맥락 정보를 추가할 때는 `cadkernel-core`의 `with_context()` 사용
- 공개 API 경로에서 `.unwrap()` 금지; `?` 또는 명시적 에러 처리 사용

### 타입 및 네이밍

- 모든 기하 연산에 `f64` 사용 (`f32` 금지)
- 토폴로지 엔티티 참조에 `Handle<T>` 사용 (세대 인덱스)
- 영속적 네이밍에 `Tag` 사용: `Tag::generated(EntityKind, OperationId, local_index)`
- `OperationId`는 `model.history.next_operation("op_name")`에서 취득
- 모든 트레이트 오브젝트에 `Send + Sync` 바운드 필요: `Arc<dyn Curve + Send + Sync>`

### 품질

- 경고 제로: CI에서 clippy strict 모드 강제 (`-D warnings`)
- 기하 생성자는 모든 매개변수를 검증 (예: radius > 0, segments >= 3)
- 새 프리미티브와 피처에는 반드시 단위 테스트를 포함

## PR 작업 흐름

1. 저장소를 Fork하고 `main`에서 브랜치를 생성합니다:
   - 새 기능: `feature/<name>`
   - 버그 수정: `fix/<name>`
   - 긴급 수정: `hotfix/<name>`
2. 위의 코딩 규칙에 따라 수정합니다.
3. 전체 검증 실행 (세 가지 검사 모두 통과 필요).
4. 브랜치를 Push하고 `main`을 대상으로 Pull Request를 생성합니다.
5. PR은 **스쿼시 머지** 방식으로 합쳐집니다 — 커밋 히스토리를 깔끔하게 유지하되 수동 스쿼시는 불필요합니다.

### 커밋 메시지 규칙

[Conventional Commits](https://www.conventionalcommits.org/)를 사용합니다:

| 접두사 | 사용 목적 |
|--------|---------|
| `feat:` | 새 기능 |
| `fix:` | 버그 수정 |
| `refactor:` | 동작 변경 없는 구조 개선 |
| `test:` | 테스트 추가 또는 수정 |
| `docs:` | 문서만 변경 |
| `chore:` | 빌드, CI, 의존성 업데이트 |

### PR 체크리스트

- [ ] 브랜치 이름이 `feature/`, `fix/`, `hotfix/` 규칙을 따르는가?
- [ ] `cargo build --workspace`가 에러 없이 통과하는가?
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`가 경고 없이 통과하는가?
- [ ] `cargo test --workspace`가 통과하는가 — 모든 테스트 녹색?
- [ ] 새 공개 API에 doc 주석(`///`)이 있는가?
- [ ] 필요한 테스트를 추가하거나 갱신했는가?
- [ ] 사용자에게 보이는 변경이라면 CHANGELOG.md를 갱신했는가?

## 처음 기여하기 좋은 영역

`good first issue` 라벨이 붙은 이슈를 찾아보세요. 분야별 구체적인 시작점:

- **새 프리미티브** — `crates/modeling/src/primitives/`에 파일을 추가하고 `box_shape.rs` 패턴을 참고하세요. `mod.rs`에 연결합니다.
- **I/O 형식 개선** — `crates/io/src/step.rs` 또는 `crates/io/src/brep_format.rs`에서 STEP/IGES 엔티티 지원을 확장합니다.
- **스케치 구속 조건** — `crates/sketch/src/`에서 Newton-Raphson 솔버 패턴을 따라 새 구속 조건 타입을 구현합니다.
- **Lua 스크립트 예제** — `examples/lua/`에 모델링 워크플로우를 보여주는 `.lua` 파일을 추가합니다.
- **doc 주석 작성** — 어느 크레이트든 문서화되지 않은 공개 함수에 `///` doc 주석을 추가합니다.
- **벤치마크 추가** — `crates/modeling/benches/`에 벤치마크가 없는 연산에 대한 Criterion 벤치마크를 추가합니다.

## 보안

보안 취약점은 [SECURITY.ko.md](SECURITY.ko.md)의 책임 있는 공개 절차를 따라 주세요.
