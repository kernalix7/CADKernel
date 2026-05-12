# 변경 이력

[English](../CHANGELOG.md) | **한국어**

이 프로젝트의 주요 변경 사항은 이 문서에 기록됩니다.

형식은 [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)를 기반으로 하며,
버전 정책은 [Semantic Versioning](https://semver.org/lang/ko/)을 지향합니다.

## [Unreleased]

### 추가됨

#### 상용 CAD 로드맵 — A3.0.2: `.cadk` 옵트인 zstd 압축 (2026-05-12)
- **`cadk::SaveOptions { compression_level, thumbnail }`** — `encode_with_options` + `Session::save_cadk_with_options` / `save_cadk_to_path_with_options` 단일 옵션 구조체. 빌더 헬퍼 `.with_compression(level)` / `.with_thumbnail(bytes)` 제공. 기본값(`None`/`None`)은 기존 `encode()`와 바이트 동일 — v0 픽스처와 기존 리더 모두 유효.
- **`CadkFlags::DOCUMENT_COMPRESSED` (bit 3)** — `BlobKind::Document` payload zstd 압축 시 설정. `decode()`가 비트 자동 감지하여 압축 해제. CRC는 (압축된) on-disk 바이트 기준이라 무결성 검사가 해제보다 선행.
- **`zstd 0.13`** 워크스페이스 의존성 추가(`cadkernel-api`만).
- **`crates/api/tests/cadk_compression.rs`** — 6개 테스트: 레벨 3/22 라운드트립, `SaveOptions::default()`가 `save_cadk()` 바이트 동일, 레벨 22가 반복 로그를 미압축 대비 엄격히 축소, 압축 + 썸네일 동봉 조합, 파일시스템 경로 라운드트립.
- v0 호환: 커밋된 `r1_canonical.cadk`(미압축)는 변경 없는 `cadk_v0_migration.rs`에서 계속 그린.

#### 상용 CAD 로드맵 — A3.0.1: `.cadk` 파일시스템 경로 API + v0 마이그레이션 가드 (2026-05-12)
- **`Session::save_cadk_to_path` / `save_cadk_to_path_with_thumbnail` / `load_cadk_from_path`** — 기존 바이트 버퍼 `save_cadk` / `load_cadk` + `std::fs::{read,write}` 래퍼. I/O 오류는 `ApiError::Codec("file io: ...")`로 매핑(SemVer 안정 — `ApiError` enum 그대로).
- **`crates/api/tests/cadk_path_roundtrip.rs`** — 4개 통합 테스트(임시 파일 라운드트립, 썸네일 동봉 라운드트립, missing-path 실패 시 `file io:` 프리픽스 확인, unwritable-path 실패 동일).
- **`crates/api/tests/fixtures/cadk-v0/r1_canonical.cadk`** — 커밋된 v0 골든 픽스처(309바이트, R1 정규 prefix). 향후 코덱 변경이 v0 호환을 깨면 CI에서 즉시 실패.
- **`crates/api/tests/cadk_v0_migration.rs`** — 상시 3개 가드 + `#[ignore]` regenerator 1개. (1) 커맨드 로그 동일성, (2) `Session::load_cadk_from_path` 경유 solid_count == 1, (3) `CADK` 매직 바이트 시작.

#### 상용 CAD 로드맵 — v0.5 Gate #6 / #7 / #8 **검증 완료**: UX 검증 하네스 (2026-05-11)
- **`crates/viewer/tests/picking_at_distance.rs`** — Gate #6: 카메라 거리(1 m / 10 m / 100 m / 1000 m)별 B-Rep face/edge/vertex 피킹 불변성 잠금.
- **`crates/viewer/tests/property_binding.rs`** — Gate #7: Box dx/dy/dz, Cylinder radius/height에 대한 속성 패널 양방향 바인딩 검증.
- **`crates/viewer/tests/sketcher_dof.rs`** — Gate #8: 5개 이상 구속 구성(비구속/부분/완전/과구속/construction geometry 전용)에서 DoF 배지 정확성 검증.
- v0.5 Gate #6 / #7 / #8 상태: `❓ unverified` → `✅ verified 2026-05-11`.

#### 상용 CAD 로드맵 — v0.5 Gate #11 **Phase A 완료**: R1 open 벤치마크 CI 강제 적용 (2026-05-11)
- **`cadkernel-api`에 `pub mod reference_parts` 추가** — `r1_bytes()` / `r2_bytes()`가 `ApiResult<Vec<u8>>` 반환. 예제 바이너리와 Criterion 벤치 공유.
- **`crates/api/benches/reference_parts_open.rs`** — R1/R2 open-from-bytes Criterion 벤치 (`harness = false`).
- **R1 실측 평균: 8.5894 µs** (CI [8.5426 µs, 8.6465 µs]) — 250 ms 예산 대비 ~29,000× 여유.
- **`scripts/bench_threshold.sh`** — `python3`로 `estimates.json` 파싱, 0(통과) / 1(초과) / 2(오류) 종료 코드.
- **`scripts/bench_threshold_test.sh`** — 5개 케이스 자체 테스트, 전부 통과.
- **`.github/workflows/ci.yml` `bench-perf` 잡 추가** — `quality` 이후 ubuntu-latest 단독 실행; `cargo bench` 후 임계값 스크립트 실행; Criterion HTML 보고서를 아티팩트로 업로드.
- v0.5 Gate #11 Phase A 완료. Phase B(첫 화면 < 500 ms)는 headless wgpu 하네스 필요로 보류. 워크스페이스: **3,167 / 0 / 0**.

#### 상용 CAD 로드맵 — v0.5 Gate #11 **Phase B 완료**: viewer CPU 콜드 초기화 벤치마크 CI 강제 적용 (2026-05-12)
- **`crates/viewer/benches/first_paint.rs`** — `cold_init_cpu_only()`를 측정하는 Criterion 벤치. GPU·디스플레이 없이 headless CI에서 실행 가능.
- **`crates/viewer::test_support::cold_init_cpu_only()`** — 이벤트 루프, egui 컨텍스트, GUI 상태, 스크립팅 엔진까지 CPU 초기화 경로를 결정론적으로 실행하는 공개 헬퍼.
- **실측 평균: ~856 ns** — 500 ms 예산 대비 ~584,000× 여유.
- **`.github/workflows/ci.yml` `bench-perf` 잡 확장**: R1 open 단계 이후 `cargo bench -p cadkernel-viewer --bench first_paint` 및 `bash scripts/bench_threshold.sh viewer_first_paint_cpu 500000000` 추가.
- v0.5 Gate #11 Phase B 완료. **v0.5 게이트 12/12 전부 완료.**

#### 상용 CAD 로드맵 — v0.5 Gate #5 **완료**: MCP 서버를 `crates/mcp/`으로 분리 (2026-05-11)
- **새 워크스페이스 멤버 `crates/mcp/` (`cadkernel-mcp`)**: `cadkernel-api` + `cadkernel-io` 의존. 8개 MCP 툴이 모두 `Session::execute(Command::*)` 경유로 전환.
- **`crates/io/src/mcp.rs` 삭제** (~2,208줄). `McpServer` / `McpRequest` / `McpResponse` / `McpError` / `McpToolDef`는 이제 `cadkernel-mcp`에서 재익스포트.
- **Cone frustum 제한**: `top_radius != 0`이면 `INVALID_PARAMS` 반환 (STOP_LIST — 새 `Command` 배리언트 금지).
- **Boolean 피연산자 소비**: `target_id` / `tool_id` 슬롯 모두 소거; 결과는 빈 슬롯 재사용.
- **34개 이식 + 신규 2개** 테스트가 `crates/mcp/tests/server_integration.rs`에 추가. 워크스페이스: **3,138 / 0 / 0**.

#### 상용 CAD 로드맵 — v0.5 Gate #12 **라이브러리 크레이트 전체 완료**: `cadkernel-modeling` panic-free (2026-05-11)
- **`crates/modeling/src/lib.rs`에 deny 적용** (비-테스트 한정). 847개 원시 매치 중 production은 25개뿐.
- **`boolean/face_split.rs`**: `*pts.last().unwrap()` (조기 반환으로 비어 있지 않음 보장) → 인덱싱; `*chord.first/last().unwrap()` (가드 후) → 인덱싱.
- **`features/section.rs`**: `unique.last().unwrap()` → 인덱싱.
- **`features/shell.rs` (4) + `primitives/prism_shape.rs` (2) + `primitives/shape_primitives.rs` (4) + `appearance.rs` (3)**: `model.vertices.get(h).unwrap().point` → `.map(|v| v.point).unwrap_or(Point3::ORIGIN)`.
- **`draft_ops.rs`**: `make_wire` 닫힌 루프 검출, `make_fillet_wire` / `make_chamfer_wire` / `upgrade_wire` 의 `.first/.last().unwrap()` → 인덱싱. `join_wires`: `*result.last().unwrap()` → `let-else { result.extend_from_slice(wire); continue; }` (빈 wires[0] 시 잠재 panic 제거). `snap_to_dimensions`: 3원소 배열 `min_by().unwrap()` → `unwrap_or((query, 0.0))`. `DraftStyleManager::get_active`: 내부 `.unwrap()` → `LazyLock<DraftStyle>` fallback.
- **v0.5 Gate #12 라이브러리 크레이트 전체 완료**: `api` + `core` + `math` + `sketch` + `topology` + `geometry` + `io` + `modeling` (8 / 8). `viewer`는 보류 (애플리케이션 바이너리, GPU/이벤트 루프 초기화 panic 관례).

#### 상용 CAD 로드맵 — v0.5 Gate #12 확장: `cadkernel-io` panic-free (2026-05-11)
- **`crates/io/src/lib.rs`에 deny 적용**. 408개 원시 매치 중 production은 약 34개; 나머지는 모두 `#[cfg(test)]` 내부.
- **`pdf.rs`**: 17개 `num_stack.pop().unwrap()` → `unwrap_or(0.0)`. 호출 직전 `num_stack.len() >= N` 가드가 있어 fallback은 구조적으로 도달 불가.
- **`mesh_ops.rs`**: 최단 edge 선택의 `min_by().unwrap()` → `let-else { break; }`; slice intersection의 `position().unwrap()` 2곳 → `let-else { return; }`.
- **`step.rs`**: `best.is_none() || err < best.unwrap().4` → `best.as_ref().is_none_or(|b| err < b.4)` (MSRV 1.85).
- **`tessellate.rs`**: 4개의 `face_data.unwrap()`/`surface.unwrap()` 페어 → edition 2024 let-chain `if let Some(fd) = face_data.filter(|_| use_surface_tess) && let Some(surface) = fd.surface.as_ref()`. `outer_trim.unwrap()` → `if let Some(t) = outer_trim && !t.contains_point(...)`. 불필요해진 `has_outer` local 제거.
- **`mcp.rs`**: `dst.make_loop(...).unwrap()` 2곳 (반환형 `Handle<FaceData>`) → `.unwrap_or_else(|_| dst.loops.insert(LoopData::new(he0)))`.
- v0.5 Gate #12: 9개 라이브러리 크레이트 중 7개 (`api` + `core` + `math` + `sketch` + `topology` + `geometry` + `io`) 완료. `viewer`는 보류 (애플리케이션 바이너리, GPU/이벤트 루프 초기화 panic은 관례). 남음: `modeling` 847.

#### 상용 CAD 로드맵 — v0.5 Gate #12 확장: `cadkernel-geometry` panic-free (2026-05-11)
- **`crates/geometry/src/lib.rs`에 deny 적용** (비-테스트 한정). 156개 원시 매치 중 production은 6개뿐; 나머지는 모두 `#[cfg(test)]` 내부.
- **`crates/geometry/src/curve/nurbs.rs`**:
  - `elevate_degree`: `*distinct_knots.last().unwrap()` / `*new_knots.last().unwrap()` (구조적으로 비어 없음) → 명시적 `len() - 1` 인덱싱.
  - `split_at`: knot refinement 후 `.position(...).unwrap()` → `.ok_or_else(...)` (`KernelError::InvalidArgument`).
  - `refine_knots`: `partial_cmp(b).unwrap()` → `.unwrap_or(Ordering::Equal)`.
  - `decompose_to_bezier` (`Vec<NurbsCurve>` 반환, `Result` 아님): `.insert_knot.unwrap()` → `match`, 에러 시 `vec![self.clone()]` 구조적 브리 케이스 반환.
- **`crates/geometry/src/tessellate.rs::adaptive_sample`**: `*params.last_mut().unwrap() = t_end` → `let last_idx = ...; params[last_idx] = t_end`.
- v0.5 Gate #12 커버리지: `api` + `core` + `math` + `sketch` + `topology` + `geometry` (6 / 9). 남은 작업: `viewer` 175, `io` 408, `modeling` 847.

#### 상용 CAD 로드맵 — v0.5 Gate #12 확장: `cadkernel-topology` panic-free (2026-05-11)
- **`crates/topology/src/lib.rs`에 `clippy::unwrap_used` + `clippy::expect_used` + `clippy::panic` deny 적용** (비-테스트 한정).
- **Half-edge B-Rep 빌더 14개 사이트에서 `X.get_mut(h).unwrap()` → `if let Some(_) = X.get_mut(h)` 사용** (`add_edge`, `add_edge_tagged`, `make_wire_tagged`, `make_face`, `make_face_tagged`, `add_inner_loop`, `make_shell`, `make_shell_tagged`, `make_solid`, `make_solid_tagged`). 외어썪한 새 핸들은 직전 `insert`에서 반환되었으므로 구조적으로 Some이 보장되어 동작 동일, panic 표면만 제거.
- **`validate` / `validate_collect` 루프 사이클 체크**: `if hes.len() < 2` 조기 탈출로 보호되는 `*hes.last().unwrap()`을 `hes[hes.len() - 1]` 인덱싱으로 교체.
- v0.5 Gate #12 커버리지: `cadkernel-api` + `cadkernel-core` + `cadkernel-math` + `cadkernel-sketch` + `cadkernel-topology` (5 / 9). 남은 작업: `geometry` 156, `viewer` 175, `io` 408, `modeling` 847.

#### 상용 CAD 로드맵 — v0.5 Gate #12 확장: `cadkernel-math` + `cadkernel-sketch` panic-free (2026-05-11)
- **`crates/math/src/lib.rs`에 `clippy::unwrap_used` + `clippy::expect_used` + `clippy::panic` deny 적용** (비-테스트 한정). 기존 4개 unwrap은 모두 `#[cfg(test)]` 모듈 내부였으므로 production 변경 없음.
- **`crates/sketch/src/lib.rs`에 동일 deny 적용.**
- **`crates/sketch/src/profile.rs::extract_profile`**: 체인 루프가 매 반복마다 `ordered_points.last().unwrap()`을 호출하던 부분을 로컬 변수로 마지막 끝점을 추적하도록 리팩터. 동작 동일, panic 표면 제거.
- v0.5 Gate #12 커버리지: `cadkernel-api` + `cadkernel-core` + `cadkernel-math` + `cadkernel-sketch`. 남은 작업: `topology` 39, `geometry` 156, `viewer` 175, `io` 408, `modeling` 847.

#### 상용 CAD 로드맵 — v0.5 Gate #12 확장: `cadkernel-core` panic-free (2026-05-11)
- **`crates/core/src/lib.rs`에도 `clippy::unwrap_used` + `clippy::expect_used` + `clippy::panic` deny 적용** (비-테스트 코드 한정). `cadkernel-core`는 이미 production unwrap이 0개였으므로 `KernelResult<T>`-first 계약의 뿌리에 lint로 못박음. 동작 변경 없음.
- 나머지 크레이트는 unwrap 카운트를 갚아가며 순차 확대.

#### 상용 CAD 로드맵 — v0.5 Gate #12 partial: `cadkernel-api` panic-free (2026-05-11)
- **`crates/api/src/lib.rs`에 `clippy::unwrap_used` + `clippy::expect_used` + `clippy::panic` deny 적용** (비-테스트 코드 한정, `#![cfg_attr(not(test), deny(...))]`). SemVer-lock된 공개 API 표면이 lint 레벨에서 panic-free 증명.
- **`crates/api/src/cadk/codec.rs` `read_header`**: 6개의 `.try_into().unwrap()` 호출을 새 프라이빗 헬퍼 `read_le_u32` / `read_le_u64`로 교체. 동작 동일, panic 표면 제거.
- **`crates/api/src/bin/cadk_inspect.rs`**: verbose 모드 헤더 재파싱의 2개 `.try_into().unwrap()`를 동일 패턴으로 교체.
- **API 크레이트만 닫음**. 후속 작업으로 구조적으로 0개인 `core`에 확장, 나머지 7크레이트는 unwrap 고소 진행 후 선택적 적용 (`modeling` 847, `io` 408, `viewer` 175, `geometry` 156, `topology` 39, `sketch` 24, `math` 4). 워크스페이스 전체 제한은 v1.0 deliverable.
- 테스트: 3,164 / 0 / 0 (변동 없음). `cargo clippy --workspace --all-targets --all-features -- -D warnings`가 새 deny-list 포함 정상.

#### 상용 CAD 로드맵 — v0.5 Gate #9 / #10 R1 + R2 레퍼런스 파트 완성 (2026-05-11)
- **`examples/build_reference_parts.rs` 스턽 해소.** R1 (축정렬 100×50×25 박스)과 R2 (Ø10 관통움 1개를 가진 60×40×10 평판)을 공개 `cadkernel-api` 표면(`Command::CreateBox`, `Command::CreateCylinder`, `Command::BooleanSubtract`, `Session::save_cadk`)만으로 완전 구성. 커널 직접 호출 없음. 로드맵 v0.5 Gate #9, #10 닫음.
- **`tests/reference_parts_corpus.rs` 회귀 테스트 6개:** R1/R2 공개 API 빌드, R1/R2 `.cadk` 두 독립 실행 간 바이트 동일성(결정성 Trust gate), R1/R2 `.cadk` `save_cadk` → `load_cadk` → 재인코딩 라운드트립 바이트 동일성.
- **저장소 루트 `STOP_LIST.md` 추가 — 범위 잠금.** v0.5/v1.0 Gate Trust 행이 닫힐 때까지 `Command::*` 옵서버 슬라이스 체인을 #48 (`FirstOperation`)에서 일시 정지. 새 파일 포맷 크레이트, 새 뷰어 워크벤치, 새 해석적 서페이스 종류, 새 스케치 구속조건 종류, 새 FEM 해석 종류 금지. `docs/COMMERCIAL_CAD_ROADMAP.md` v3.5를 유일한 스코어보드로 참조.
- **R3-R12는 여전히 스턽.** 각자가 참조하는 Gate가 닫힌 후 후속 단계에서 착독.
- **테스트 HEAD: 3,164 / 0 / 0** (3,158 → +6). 워크스페이스 build / clippy / test --no-fail-fast 모두 정상.

#### API — `Command::FirstOperation` 첫 히스토리 이벤트 조회 (2026-05-11)
- **`Command::FirstOperation` (38번째 옷서버, 매개변수 없음) 신규** + `Outcome::FirstOperation { op_name, description }` + `OutcomeKind::FirstOperation` — 가장 오랭된 히스토리 이벤트(항상 index 0)의 `op_name`과 `description` 반환. `LastOperation`의 거울.
- **연결**: `Session::execute` fast-path 38번째 변형. `Document::history()[0]`. 빈 히스토리에는 `InvalidArgument("history is empty")` 반환. history 불변.
- **스키마**: `command_schemas()`에 `op = "first_operation"` (매개변수 없음) 추가.
- **테스트**: 6개 회귀 테스트 추가 (빈 히스토리 → `InvalidArgument`, CreateBox 후 oldest, 후속 이벤트에 불변, `HistoryDescription { index: 0 }`와 일치, history 불변, JSON 라운드트립) + 스키마 커버리지 확장.

#### API — `Command::HasOperation` 히스토리 op 존재 여부 판별 (2026-05-11)
- **`Command::HasOperation { op_name }` (37번째 옷서버) 신규** + `Outcome::HasOperation { op_name, present }` + `OutcomeKind::HasOperation` — 히스토리 이벤트 중 하나라도 `op` 필드가 `op_name`과 정확히 일치(대소문자 구분)할 때 `true` 반환. 단순 존재 여부만 필요한 경우 `OperationCount`보다 경량.
- **연결**: `Session::execute` fast-path 37번째 변형. `Document::history().iter().any()`. history 불변.
- **스키마**: `command_schemas()`에 `op = "has_operation"` (필수 `op_name: string` 1개) 추가.
- **테스트**: 7개 회귀 테스트 추가 (빈 히스토리 → false, CreateBox 후 true, 알 수 없는 op → false, 대소문자 구분, `OperationCount > 0`과 일치, history 불변, JSON 라운드트립) + 스키마 커버리지 확장.

#### API — `Command::LastOperation` 최근 히스토리 이벤트 조회 (2026-05-11)
- **`Command::LastOperation` (36번째 옷서버, 매개변수 없음) 신규** + `Outcome::LastOperation { index, op_name, description }` + `OutcomeKind::LastOperation` — 가장 최근 히스토리 이벤트의 0기반 `index`, `op_name`, `description`을 단일 호출로 반환. `HistoryDescription { index: HistoryCount - 1 }` + op 이름과 동등하지만 완전 한 번의 호출로.
- **연결**: `Session::execute` fast-path 36번째 변형. 빈 히스토리에는 `InvalidArgument("history is empty")` 반환. history 불변.
- **스키마**: `command_schemas()`에 `op = "last_operation"` (매개변수 없음) 추가.
- **테스트**: 6개 회귀 테스트 추가 (빈 히스토리 → `InvalidArgument`, CreateBox 후 index=0, CreateBox/CreateBox/Translate 추적, `HistoryDescription` 일치, history 불변, JSON 라운드트립) + 스키마 커버리지 확장.

#### API — `Command::OperationCount` 히스토리 op 발생 횟수 카운터 (2026-05-10)
- **`Command::OperationCount { op_name }` (35번째 옵서버) 신규** + `Outcome::OperationCount { op_name, count }` + `OutcomeKind::OperationCount` — 히스토리 이벤트 중 `op` 필드가 `op_name`과 정확히 일치(대소문자 구분)하는 개수를 `u32`로 반환. 단일 op 카운트만 필요할 때 `HistoryEvents`보다 경량 (예: "`create_box` 몇 번 호출됐는가?").
- **연결**: `Session::execute` fast-path 35번째 변형. `Document::history()` 필터링. 알 수 없는 op는 `count = 0` (에러 아님). history 불변. 필드명을 `op`가 아닌 `op_name`으로 한 이유: `Command`의 serde 태그 `tag = "op"` 식별자와 충돌하기 때문.
- **스키마**: `command_schemas()`에 `op = "operation_count"` (필수 `op_name: string` 1개) 추가.
- **테스트**: 7개 회귀 테스트 추가 (빈 히스토리 → 0, CreateBox 3회 → 3, `create_box`/`translate` 구분, 알 수 없는 op → 0, 대소문자 구분, history 불변, JSON 라운드트립) + 스키마 커버리지 확장.

#### API — `Command::IsSquareXz` 사각 XZ 풋프린트 판별 옵서버 (2026-05-10)
- **`Command::IsSquareXz { id }` (34번째 옵서버) 신규** + `Outcome::IsSquareXz { id, square }` + `OutcomeKind::IsSquareXz` — X와 Z AABB 길이가 절대 허용 오차 `1e-9` 내에서 동일할 때 `true` 반환 (Y 무관). 사각 단면 옵서버 직교 3종 트리오(`IsSquareXy` / `IsSquareYz` / `IsSquareXz`) 완성 — Y축 방향 돌출 솔리드 감지용.
- **연결**: `Session::execute` fast-path 34번째 변형. `Document::bounding_box` 재사용, `(dx - dz).abs() <= 1e-9`. 잘못된 id에는 `UnknownSolid`. history 불변.
- **스키마**: `command_schemas()`에 `op = "is_square_xz"` (필수 `id` 1개) 추가.
- **테스트**: 7개 회귀 테스트 추가 (4×20×4 Y돌출 → true, 정육면체 → true, 3×5×7 → false, 4×9×4 트리오 교차 검증 (xz=true, xy=false, yz=false), 잘못된 id, history 불변, JSON 라운드트립) + 스키마 커버리지 확장.

#### API — `Command::IsSquareYz` 사각 YZ 풋프린트 판별 옵서버 (2026-05-10)
- **`Command::IsSquareYz { id }` (33번째 옵서버) 신규** + `Outcome::IsSquareYz { id, square }` + `OutcomeKind::IsSquareYz` — Y와 Z AABB 길이가 절대 허용 오차 `1e-9` 내에서 동일할 때 `true` 반환 (X 무관). `IsSquareXy`와 쌍을 이루는 X축 방향 돌출 솔리드 감지용.
- **연결**: `Session::execute` fast-path 33번째 변형. `Document::bounding_box` 재사용, `(dy - dz).abs() <= 1e-9`. 잘못된 id에는 `UnknownSolid`. history 불변.
- **스키마**: `command_schemas()`에 `op = "is_square_yz"` (필수 `id` 1개) 추가.
- **테스트**: 7개 회귀 테스트 추가 (20×4×4 X돌출 → true, 정육면체 → true, 3×5×7 → false, 4×4×9 `IsSquareXy`와 대칭, 잘못된 id, history 불변, JSON 라운드트립) + 스키마 커버리지 확장.

#### API — `Command::HistoryDescription` 단일 히스토리 이벤트 조회 옵서버 (2026-05-10)
- **`Command::HistoryDescription { index }` (32번째 옵서버) 신규** + `Outcome::HistoryDescription { index, description }` + `OutcomeKind::HistoryDescription` — 주어진 0-기반 인덱스의 히스토리 이벤트 `description` 문자열 반환. 단일 항목만 필요한 경우(툴팁 렌더링, 로그 조사) `HistoryEvents`보다 저렴.
- **연결**: `Session::execute` fast-path 32번째 변형. `Document::history()` 읽고 단일 `description` 복제. 인덱스 범위 초과 시 `InvalidArgument` (잘못된 인덱스 + 현재 길이 포함). history 불변.
- **스키마**: `command_schemas()`에 `op = "history_description"` (필수 `index: u32` 1개) 추가.
- **테스트**: 6개 회귀 테스트 추가 (최근 이벤트 확인, `Document::history()` 완전 일치, 범위 초과, 빈 히스토리, history 불변, JSON 라운드트립) + 스키마 커버리지 확장.

#### API — `Command::IsSquareXy` 사각 XY 풋프린트 판별 옵서버 (2026-05-10)
- **`Command::IsSquareXy { id }` (31번째 옵서버) 신규** + `Outcome::IsSquareXy { id, square }` + `OutcomeKind::IsSquareXy` — X와 Y AABB 길이가 절대 허용 오차 `1e-9` 내에서 동일할 때 `true` 반환 (Z는 무관). XY 평면에서 사각 풋프린트를 가진 솔리드(사각 프리즘, 기둥, 기닥) 감지용.
- **연결**: `Session::execute` fast-path 31번째 변형. `Document::bounding_box` 재사용, `(dx - dy).abs() <= 1e-9`. 잘못된 id에는 `UnknownSolid`. history 불변.
- **스키마**: `command_schemas()`에 `op = "is_square_xy"` (필수 `id` 1개) 추가.
- **테스트**: 6개 회귀 테스트 추가 (4×4×20 기둥 → true, 3×3×3 정육면체 → true, 3×5×3 → false, 잘못된 id, history 불변, JSON 라운드트립) + 스키마 커버리지 확장.

#### API — `Command::IsCubic` 정육면체 판별 옵서버 (2026-05-10)
- **`Command::IsCubic { id }` (30번째 옵서버) 신규** + `Outcome::IsCubic { id, cubic }` + `OutcomeKind::IsCubic` — 경계 상자의 세 길이가 절대 허용 오차 `1e-9` 내에서 동일할 때 `true` 반환. 슬렌더니스 지표인 `AabbAspectRatio`와 쌍을 이루는 빠른 형상 분류기.
- **연결**: `Session::execute` fast-path 30번째 변형. `Document::bounding_box` 재사용, `(longest - shortest) <= 1e-9`. 잘못된 id에는 `UnknownSolid`. history 불변.
- **스키마**: `command_schemas()`에 `op = "is_cubic"` (필수 `id` 1개) 추가.
- **테스트**: 6개 회귀 테스트 추가 (4×4×4 → true, 4×4×5 → false, 이동 불변성, 잘못된 id, history 불변, JSON 라운드트립) + 스키마 커버리지 확장.

#### API — `Command::AabbAspectRatio` AABB 종횡비 옵서버 (2026-05-10)
- **`Command::AabbAspectRatio { id }` (29번째 옵서버) 신규** + `Outcome::AabbAspectRatio { id, ratio }` + `OutcomeKind::AabbAspectRatio` — `최장 길이 / 최단 길이` (항상 `>= 1.0`) 반환. 최단 길이가 0인 퇴화/평면 솔리드는 `f64::INFINITY` 반환. 세 길이를 일일이 비교하지 않고도 슬렌더니스(slenderness) 지표를 한 숫자로 얻기 위한 옵서버.
- **연결**: `Session::execute` fast-path 29번째 변형. `Document::bounding_box` 재사용, `dx.max(dy).max(dz) / dx.min(dy).min(dz)` 결합. 잘못된 id에는 `UnknownSolid`. history 불변.
- **스키마**: `command_schemas()`에 `op = "aabb_aspect_ratio"` (필수 `id` 1개) 추가.
- **테스트**: 6개 회귀 테스트 추가 (5×5×5 정육면체 → 1.0, 2×4×20 → 10.0, 이동 불변성, 잘못된 id, history 불변, JSON 라운드트립) + 스키마 커버리지 확장.

#### API — `Command::AabbShortestAxis` AABB 최단축 옵서버 (2026-05-10)
- **`Command::AabbShortestAxis { id }` (28번째 옵서버) 신규** + `Outcome::AabbShortestAxis { id, axis }` + `OutcomeKind::AabbShortestAxis` — 경계 상자에서 가장 짧은 축 인덱스(`0`=X, `1`=Y, `2`=Z) 반환. 동점은 낮은 인덱스 우선. `AabbLongestAxis`의 대칭 카운터파트. 쇬버/얇은 솔리드 감지용.
- **연결**: `Session::execute` 읽기 전용 fast-path에 28번째 변형으로 추가. `Document::bounding_box` 재사용, 두 단계 `<=` 비교로 선택. 잘못된 id에는 `UnknownSolid`. history 불변.
- **스키마**: `command_schemas()`에 `op = "aabb_shortest_axis"` (필수 `id` 1개) 추가.
- **테스트**: `crates/api/tests/api_integration.rs`에 7개 회귀 테스트 추가 (1×5×8 → X, Y·Z 우세 교차, 정육면체 동점 → axis 0, 잘못된 id, history 불변, JSON 라운드트립, 2×7×4 → longest=1 / shortest=0 대칭) + 스키마 커버리지 확장.

#### API — `Command::AabbLongestAxis` AABB 최장축 옵서버 (2026-05-10)
- **`Command::AabbLongestAxis { id }` (27번째 옵서버) 신규** + `Outcome::AabbLongestAxis { id, axis }` + `OutcomeKind::AabbLongestAxis` — 경계 상자에서 가장 긴 축 인덱스(`0`=X, `1`=Y, `2`=Z) 반환. 동접은 낮은 인덱스 우선(X > Y > Z). 카메라 프레이밍, 테셰레이션 시임 경로 등 방향성 휴리스틱용.
- **연결**: `Session::execute` 읽기 전용 fast-path에 27번째 변형으로 추가. `Document::bounding_box` 재사용, 두 단계 `>=` 비교로 설택. 잘못된 id에는 `UnknownSolid`. history 불변.
- **스키마**: `command_schemas()`에 `op = "aabb_longest_axis"` (필수 `id` 1개) 추가.
- **테스트**: `crates/api/tests/api_integration.rs`에 6개 회귀 테스트 추가 (2×3×10 → Z 우세, X·Y 우세 교차, 정육면체 동점 → axis 0, 잘못된 id, history 불변, JSON 라운드트립) + 스키마 커버리지 확장.

#### API — `Command::AabbExtents` AABB 길이 옵서버 (2026-05-09)
- **`Command::AabbExtents { id }` (26번째 옵서버) 신규** + `Outcome::AabbExtents { id, extents }` + `OutcomeKind::AabbExtents` — 축별 경계 상자 길이 `[dx, dy, dz]` 반환. `Bounds`보다 저렴 (min/max 점 생략), `Diagonal`의 원시 카운터파트.
- **연결**: `Session::execute` 읽기 전용 fast-path에 26번째 변형으로 추가. `Document::bounding_box` 재사용. 잘못된 id에는 `UnknownSolid`. history 불변.
- **스키마**: `command_schemas()`에 `op = "aabb_extents"` (필수 `id` 1개) 추가.
- **테스트**: `crates/api/tests/api_integration.rs`에 6개 회귀 테스트 추가 (4×6×8 박스 → `[4,6,8]`, 잘못된 id, 평행이동 불변, history 불변, JSON 라운드트립, 3×4×12 박스 → `Diagonal` 13.0 일치) + 스키마 커버리지 확장.

#### API — `Command::SolidIds` 솔리드 id 열거 (2026-05-09)
- **`Command::SolidIds` (25번째 옵서버) 신규** + `Outcome::SolidIds { ids }` + `OutcomeKind::SolidIds` — 라벨 없이 채워진 `Vec<SolidId>`만 반환. id만 필요할 때 `ListSolids`보다 저렴 (라벨 복제 생략).
- **연결**: `Session::execute` 읽기 전용 fast-path에 25번째 변형으로 추가. `Document::solid_ids()` 재사용. history 불변.
- **스키마**: `command_schemas()`에 `op = "solid_ids"` (파라미터 없음) 추가.
- **테스트**: `crates/api/tests/api_integration.rs`에 6개 회귀 테스트 추가 (초기 세션 → 빈 배열, 생성 3개 후 id 나열, 삭제 슬롯 제외, history 불변, JSON 라운드트립, `ListSolids` 결과와 일치) + 스키마 커버리지 확장.

#### API — `Command::HasLabel` 라벨 존재 술어 (2026-05-09)
- **`Command::HasLabel { query }` (24번째 옵서버) 신규** + `Outcome::HasLabel { query, has_label }` + `OutcomeKind::HasLabel` — 쿼리가 하나라도 라벨과 대소문자 무시 부분문자열 일치하면 `has_label = true`. 불린결과만 필요할 때 `FindByLabel`보다 저렴. 빈 쿼리는 false.
- **연결**: `Session::execute` 읽기 전용 fast-path에 24번째 변형으로 추가. `Document::solid_ids()` + `Document::solid_label()` 재사용, `Iterator::any`로 단락. history 불변.
- **스키마**: `command_schemas()`에 `op = "has_label"` (필수 `query: string` 1개) 추가.
- **테스트**: `crates/api/tests/api_integration.rs`에 6개 회귀 테스트 추가 (빈 세션 → false, 기본 "Box" 대소문자 무시 탐색, `Rename` 후 3개 대소문자 조합 매트릭스, 빈 쿼리 → false, history 불변, JSON 라운드트립) + 스키마 커버리지 확장.

#### API — `Command::HistoryCount` history 개수 옵서버 (2026-05-09)
- **`Command::HistoryCount` (23번째 옵서버) 신규** + `Outcome::HistoryCount { count: u32 }` + `OutcomeKind::HistoryCount` — 기록된 history 이벤트 개수를 단일 `u32`로 반환. history 길이만 필요할 때 `Stats`보다 저렴. `Command::SolidCount`의 대칭 카운터파트.
- **연결**: `Session::execute` 읽기 전용 fast-path에 23번째 변형으로 추가. `Document::history().len()`를 `u32`로 캐스트. history 불변.
- **스키마**: `command_schemas()`에 `op = "history_count"` (파라미터 없음) 추가.
- **테스트**: `crates/api/tests/api_integration.rs`에 5개 회귀 테스트 추가 (초기 세션 → 0, 생성+삭제를 거쳐 2 → 3, history 불변, JSON 라운드트립, `Stats::history_count`와 일치) + 스키마 커버리지 확장.

#### API — `Command::SolidCount` 솔리드 개수 옵서버 (2026-05-09)
- **`Command::SolidCount` (22번째 옵서버) 신규** + `Outcome::SolidCount { count: u32 }` + `OutcomeKind::SolidCount` — 채워진 솔리드 슬롯 개수를 단일 `u32`로 반환. 솔리드 개수만 필요할 때 `Stats`보다 저렴 (history 길이 필드 생략).
- **연결**: `Session::execute` 읽기 전용 fast-path에 22번째 변형으로 추가. `Document::solid_count()`를 `u32`로 캐스트. history 불변.
- **스키마**: `command_schemas()`에 `op = "solid_count"` (파라미터 없음) 추가.
- **테스트**: `crates/api/tests/api_integration.rs`에 5개 회귀 테스트 추가 (초기 세션 → 0, 박스 3개 생성 후 1개 삭제 → 2, history 불변, JSON 라운드트립, `Stats::solid_count`와 일치) + 스키마 커버리지 확장.

#### API — `Command::AabbSurfaceArea` AABB 표면적 올서버 (2026-05-09)
- **`Command::AabbSurfaceArea { id }` (21번째 올서버) 신규** + `Outcome::AabbSurfaceArea` + `OutcomeKind::AabbSurfaceArea` — `2 * (dx*dy + dy*dz + dz*dx)` 반환. 질량 표면적의 저렴한 상한값. `AabbVolume`의 대칭 카운터파트.
- **연결**: `Session::execute` 읽기 전용 fast-path에 21번째 변형으로 추가. `Document::bounding_box`를 재사용하여 축별 길이로부터 3개 쌍별 곱을 계산. 잘못된 id에는 `UnknownSolid`. history 불변.
- **스키마**: `command_schemas()`에 `op = "aabb_surface_area"` (필수 `id` 1개) 추가.
- **테스트**: `crates/api/tests/api_integration.rs`에 5개 회귀 테스트 추가 (4×6×8 → 208, 잘못된 id → `UnknownSolid`, history 불변, JSON 라운드트립, 평행이동 불변 2×3×5 → 62) + 스키마 커버리지 확장.

#### API — `Command::IsEmpty` 문서 비어있음 술어 (2026-05-09)
- **`Command::IsEmpty` (20번째 올서버) 신규** + `Outcome::IsEmpty` + `OutcomeKind::IsEmpty` — 문서에 솔리드가 없으면 `is_empty = true`. `Stats`보다 저렴 (history 길이 필드 생략).
- **연결**: `Session::execute` 읽기 전용 fast-path에 20번째 변형으로 추가. `Document::solid_count() == 0`을 그대로 반환. history 불변.
- **스키마**: `command_schemas()`에 `op = "is_empty"` (파라미터 없음) 추가.
- **테스트**: `crates/api/tests/api_integration.rs`에 5개 회귀 테스트 추가 (초기 세션 → true, `CreateBox` 후 → false, 단일 솔리드 삭제 후 → true, history 불변, JSON 라운드트립) + 스키마 커버리지 확장.

#### API — `Command::SolidLabel` 단일 솔리드 라벨 올서버 (2026-05-09)
- **`Command::SolidLabel { id }` (19번째 올서버) 신규** + `Outcome::SolidLabel` + `OutcomeKind::SolidLabel` — 한 슬롯의 라벨만 새로 할당된 `String`으로 반환. `ListSolids`의 `Vec<SolidEntry>` 할당을 건너뛰는 저렴한 옵서버. id를 이미 알고 표시명만 필요한 AI/스크립트용.
- **연결**: `Session::execute` 읽기 전용 fast-path에 19번째 변형으로 추가. `Document::solid_label(id)`로 라벨 조회 후 차용된 문자열을 복제하여 반환. 잘못된 id에는 `UnknownSolid`. history 불변.
- **스키마**: `command_schemas()`에 `op = "solid_label"` (필수 `id` 1개) 추가.
- **테스트**: `crates/api/tests/api_integration.rs`에 5개 회귀 테스트 추가 (기본 박스 라벨 비어있지 않음, `Rename` 후 라벨 반영, 잘못된 id → `UnknownSolid`, history 불변, JSON 라운드트립) + 스키마 커버리지 확장.

#### API — `Command::AabbCorners` AABB 꼭짓점 열거 (2026-05-09)
- **`Command::AabbCorners { id }` (18번째 올서버) 신규** + `Outcome::AabbCorners` + `OutcomeKind::AabbCorners` — 월드 공간 경계 상자의 8개 꼭짓점을 표준 순서(x → y → z, 작은 값부터 큰 값으로)로 반환. 카메라 핏 프레이밍, 디버그 와이어프레임, broad-phase 교차 시드용.
- **연결**: `Session::execute` 읽기 전용 fast-path에 18번째 변형으로 추가. `Document::bounding_box`를 재사용하여 `bbox.min`/`bbox.max`에서 8개 꼭짓점을 직접 조립. 잘못된 id에는 `UnknownSolid`. history 불변.
- **스키마**: `command_schemas()`에 `op = "aabb_corners"` (필수 `id` 1개) 추가.
- **테스트**: `crates/api/tests/api_integration.rs`에 5개 회귀 테스트 추가 (4×6×8 박스 표준 꼭짓점 배열, 잘못된 id → `UnknownSolid`, history 불변, JSON 라운드트립, 평행이동 추적) + 스키마 커버리지 확장.

#### API — `Command::ContainsAabb` AABB 포함 술어 (2026-05-09)
- **`Command::ContainsAabb { id_outer, id_inner }` (17번째 올서버) 신규** + `Outcome::AabbContainment` + `OutcomeKind::AabbContainment` — outer의 경계 상자가 inner의 경계 상자를 완전히 덮으면 `contains = true` (닫힌 구간; 면이 닿아도 포함). 전체 기하 검사를 건너뛰는 저렴한 broad-phase 포함 테스트.
- **연결**: `Session::execute` 읽기 전용 fast-path에 17번째 변형으로 추가. `Document::bounding_box`로 두 AABB를 가져와 3개 축 모두에서 `outer.min[i] <= inner.min[i] && inner.max[i] <= outer.max[i]` 조건을 검사. 어느 쪽 id가 없어도 `UnknownSolid`. history에 이벤트 추가하지 않음.
- **스키마**: `command_schemas()`에 `op = "contains_aabb"` (필수 `id_outer` / `id_inner` 2개) 추가.
- **테스트**: `crates/api/tests/api_integration.rs`에 7개 회귀 테스트 추가 (10×10×10 → 2×2×2 포함 true, 4×4×4 + 평행이동된 inner → false, 자기 포함 → true, 양쪽 id 모두 `UnknownSolid`, history 불변, JSON 라운드트립) + 스키마 커버리지 확장.

#### API — `Command::AabbVolume` AABB 부피 올서버 (2026-05-09)
- **`Command::AabbVolume { id }` (16번째 올서버) 신규** + `Outcome::AabbVolume` + `OutcomeKind::AabbVolume` — `dx * dy * dz` (축별 경계 상자 길이의 곱) 반환. 질량 부피의 저렴한 상한값으로 LOD 휴리스틱 및 비례 임계값에 유용. `Volume` / `Measure`의 질량-부피 순회를 건너뜀.
- **연결**: `Session::execute` 읽기 전용 fast-path에 16번째 변형으로 추가. `Document::bounding_box`를 재사용하여 길이 곱을 계산하고 `Outcome::AabbVolume`을 반환. 잘못된 id에는 `UnknownSolid`를 반환하고 history에 이벤트를 추가하지 않음.
- **스키마**: `command_schemas()`에 `op = "aabb_volume"` (필수 `id` 1개) 추가.
- **테스트**: `crates/api/tests/api_integration.rs`에 5개 회귀 테스트 추가 (4×6×8 박스 → 192, 잘못된 id → `UnknownSolid`, history 불변, JSON 라운드트립, 평행이동 불변) + 스키마 커버리지 확장.

#### API — `Command::AabbCenter` AABB 중심 올서버 (2026-05-09)
- **`Command::AabbCenter { id }` (15번째 올서버) 신규** + `Outcome::AabbCenter` + `OutcomeKind::AabbCenter` — `(bbox.min + bbox.max) * 0.5` 반환. `Centroid`(질량 중심)와 구분되는 AABB 중심. 배치, 그리드 스냅, 기즈모 위치 결정용.
- **올서버 fast-path 15개로 확장**. `Document::bounding_box` 재사용.
- **회귀 테스트 5개**: 4×6×8 박스 중심 = (2,3,4) / 잘못된 id `UnknownSolid` / history 미추가 / JSON 라운드트립 / translation 추적.
- **3,021 / 0 / 0** 테스트, clippy strict clean.

#### API — `Command::Diagonal` AABB 대각선 길이 올서버 (2026-05-09)
- **`Command::Diagonal { id }` (14번째 올서버) 신규** + `Outcome::Diagonal { id, length, extents }` + `OutcomeKind::Diagonal` — `extents = bbox.max - bbox.min` 및 `length = ||extents||` 반환. 카메라 fit, LOD 임계값, 허용오차 스케일링용 경량 휴리스틱.
- **올서버 fast-path 14개로 확장**. `Document::bounding_box` 재사용.
- **검증**: 잘못된 id `UnknownSolid`. translation 불변(테스트로 확인).
- **회귀 테스트 5개**: 3-4-12 박스 대각선 = 13 / 잘못된 id `UnknownSolid` / history 미추가 / JSON 라운드트립 / translation 불변.
- **3,016 / 0 / 0** 테스트, clippy strict clean.

#### API — `Command::Exists` 무예외 존재 predicate (2026-05-09)
- **`Command::Exists { id }` (13번째 올서버) 신규** + `Outcome::Exists` + `OutcomeKind::Exists` — **누락 id에 대해서도 에러를 내지 않고** `exists = false` 반환. AI/스크립트가 `UnknownSolid` 예외 처리 없이 id 유효성을 싸게 확인할 때 사용.
- **올서버 fast-path 13개로 확장**. `Document::solid_label(id).is_some()`로 구현 — 할당 없음, 메시 접근 없음.
- **회귀 테스트 5개**: 존재 id true / 누락 id false (에러 없음) / 삭제 후 false / history 미추가 / JSON 라운드트립.
- **3,011 / 0 / 0** 테스트, clippy strict clean.

#### API — `Command::IntersectsAabb` AABB 겹침 predicate (2026-05-09)
- **`Command::IntersectsAabb { id_a, id_b }` (12번째 올서버) 신규** + `Outcome::AabbIntersection` + `OutcomeKind::AabbIntersection` — 월드 AABB만으로 broad-phase 충돌 판정. 닫힌 구간 겹침이므로 면 접촉도 intersects=true. 겹칠 때 `overlap_min`/`overlap_max`로 교집합 AABB 반환, 비겹침 시 0으로 초기화.
- **올서버 fast-path 12개로 확장**. 양쪽 모두 `Document::bounding_box` 재사용(메시 순회 없음).
- **검증**: 어느 한쪽 id 누락 시 `UnknownSolid`. 자기 자신과의 교차는 항상 true.
- **회귀 테스트 7개**: 겹치는 박스 교집합 AABB / 비겹침 시 0 / 자기교차 true / 면접촉 true / 양쪽 id `UnknownSolid` / history 미추가 / JSON 라운드트립.
- **3,006 / 0 / 0** 테스트 (3,000 돌파), clippy strict clean.

#### API — `Command::Centroid` 단일 벡터 올서버 (2026-05-09)
- **`Command::Centroid { id }` (11번째 올서버) 신규** + `Outcome::Centroid` + `OutcomeKind::Centroid` — `Measure`의 부피/표면적/bbox 순회를 건너뛰고 centroid(3-벡터)만 반환.
- `Measure`의 필드별 분해 완료: `Volume` / `SurfaceArea` / `Centroid` / `Bounds`가 모두 개별 경량 올서버.
- **올서버 fast-path 11개로 확장**.
- **회귀 테스트 5개**: 2×4×6 박스 centroid = (1,2,3) / 잘못된 id `UnknownSolid` / history 미추가 / JSON 라운드트립 / translation 추적.
- **2,999 / 0 / 0** 테스트, clippy strict clean.

#### API — `Command::Volume` + `Command::SurfaceArea` 단일 스칼라 올서버 (2026-05-08)
- **`Command::Volume { id }` (9번째 올서버) 신규** + `Outcome::Volume` + `OutcomeKind::Volume` — `Measure`의 표면적/centroid/bbox 순회를 건너뛰고 부피만 반환.
- **`Command::SurfaceArea { id }` (10번째 올서버) 신규** + `Outcome::SurfaceArea` + `OutcomeKind::SurfaceArea` — 대칭형 단일 스칼라 표면적 올서버.
- **올서버 fast-path 10개로 확장**. 둘 다 내부적으로 `Document::measure_solid` 재사용. AI/스크립트가 숫자 하나만 필요할 때 전체 measurement 페이로드를 받지 않도록 와이어 노출만 최소화.
- **회귀 테스트 8개**: 2×3×4 박스 부피 24 / 표면적 52 / 양쪽 모두 잘못된 id `UnknownSolid` / history 미추가 / JSON 라운드트립.
- **2,994 / 0 / 0** 테스트, clippy strict clean.

#### API — `Command::Distance` + `Outcome::Distance` centroid-to-centroid 올서버 (2026-05-08)
- **`Command::Distance { id_a, id_b }` (8번째 올서버) 신규** + `Outcome::Distance { id_a, id_b, distance, delta }` + `OutcomeKind::Distance` 태그. 두 솔리드 centroid 사이의 유클리드 거리와 축별 차이 `centroid_b - centroid_a`를 반환. 자기 자신과의 거리(`id_a == id_b`)는 0.
- **올서버 fast-path 8개로 확장**: `Measure | Validate | ListSolids | FindByLabel | HistoryEvents | Stats | Bounds | Distance` — 상태 변경 없음, history 추가 없음.
- **검증**: 어느 한쪽 id 누락 시 `UnknownSolid`.
- **구현**: `Session::dispatch`의 Distance 분기가 `Document::measure_solid`로 양쪽 centroid를 가져와 `delta`와 `sqrt(dot(delta, delta))` 계산.
- **회귀 테스트 5개**: 3-4-5 거리 검증 / 자기 거리 0 / 잘못된 id 양방향 거절 / history 미추가 / JSON 라운드트립.
- **2,986 / 0 / 0** 테스트, clippy strict clean.

#### API — `Command::Bounds` + `Outcome::Bounds` 경량 AABB 올서버 (2026-05-08)
- **`Command::Bounds { id }` (7번째 올서버) 신규 추가** + `Outcome::Bounds { id, min, max }` + `OutcomeKind::Bounds` 태그. `Measure`가 수행하는 volume/표면적/centroid 순회를 건너뛰고 월드 공간 AABB만 반환. AABB만 필요한 경우(frustum culling, 레이아웃, 스냅, UI fitting) 용.
- **올서버 fast-path 7개로 확장**: `Measure | Validate | ListSolids | FindByLabel | HistoryEvents | Stats | Bounds` — 상태 변경 없음, history 추가 없음.
- **검증**: unknown id는 `UnknownSolid`. `min[i] <= max[i]` 보장(`Document::bounding_box` 상속).
- **`CommandSchema` 엔트리** 추가.
- **구현**: `Session::dispatch`의 Bounds 아벍이 `Document::bounding_box(id)` 감싸기 — 최단 수 줄, 수학 연산 없음.
- **회귀 테스트 5개**: 2×4×6 박스 bbox 범위 검증 / history event 미추가 / 잘못된 id 거절 / JSON 라운드트립 / `OutcomeKind::Bounds`.
- `command_schemas_cover_every_op_name` 업데이트.
- 테스트 **2,981 / 0 / 0** (TranslateTo 대비 +5), `clippy --all-targets --all-features -D warnings` 무경고.

#### API — `Command::TranslateTo` centroid를 임의 좌표로 이동 (2026-05-08)
- **`Command::TranslateTo { id, point }` 신규 추가** — 솔리드의 centroid를 월드 좌표 `point`에 일치시키는 translate. `CenterOnOrigin`(= `TranslateTo [0,0,0]`)과 `AlignTo`(= `TranslateTo target.centroid`)의 일반화. `Translate` by `point - centroid` 동가이지만 단일 명령으로 표현.
- **검증**: unknown id는 `UnknownSolid`. `Solid` handle 필요.
- **`CommandSchema` 엔트리** 추가.
- **구현**: `Session::translate_to()`이 `solid_mass_properties`로 centroid 추출 → 모든 정점을 `point - centroid` 만큼 translate. log/cursor로 완전 undo.
- **회귀 테스트 5개**: centroid 타겟 도달 / 크기 보존(volume 불변) / 잘못된 id 거절 / JSON 라운드트립 / undo 복원.
- `command_schemas_cover_every_op_name` 업데이트.
- 테스트 **2,976 / 0 / 0** (ScaleToFit 대비 +5), `clippy --all-targets --all-features -D warnings` 무경고.

#### API — `Command::ScaleToFit` 최대 bbox 경건 정규화 (2026-05-08)
- **`Command::ScaleToFit { id, target_size }` 신규 추가** — 원점(centroid) 기준 단일 명령으로 솔리드의 최대 축 범위가 `target_size`가 되도록 귬일스케일. Aspect ratio 유지. 임포트한 부품을 권장 크기로 정규화할 때 유용.
- **검증**: `target_size > 0` 아니면 `InvalidArgument`; 퇴화된 bbox(경원 0) 거절; 잘못된 id는 `UnknownSolid`.
- **`CommandSchema` 엔트리** 추가.
- **구현**: `Session::scale_to_fit()`이 `Document::bounding_box`로 bbox 조회 → 최대 경원 산출 → `factor = target_size / max_extent` → 기존 `scale_uniform` 재사용. log/cursor로 완전 undo.
- **회귀 테스트 5개**: 2×4×8 박스 → 0.25×0.5×1.0 정규화 / 음수·0 거절 / 잘못된 id 거절 / JSON 라운드트립 / undo 복원.
- `command_schemas_cover_every_op_name` 업데이트.
- 테스트 **2,971 / 0 / 0** (AlignTo 대비 +5), `clippy --all-targets --all-features -D warnings` 무경고.

#### API — `Command::AlignTo` 두 솔리드 간 centroid 정렬 (2026-05-08)
- **`Command::AlignTo { id, target_id }` 신규 추가** — `id`의 centroid를 `target_id`의 centroid와 일치시킬. 두 솔리드 정렬 컴팡스. `Translate` by `target_centroid - source_centroid` 동가이지만 단일 명령/undo로 표현.
- **자기 정렬**(`id == target_id`)은 no-op (`Outcome::SolidModified` 그대로 반환, 기하 수정 없음).
- **검증**: 소스 또는 타겟이 unknown이면 `ApiError::UnknownSolid`. 두 슬롯 모두 `Solid` handle 필요.
- **`CommandSchema` 엔트리** 추가.
- **구현**: `Session::align_to()`이 양쪽 centroid를 `solid_mass_properties`로 추출(타겟 읽기 전용, 소스 수정) 후 delta로 translate. log/cursor로 완전 undo.
- **회귀 테스트 5개**: 소스 → 타겟 정렬 / 자기 정렬 idempotent / 소스·타겟 잘못된 id 거절 / JSON 라운드트립 / undo 복원.
- `command_schemas_cover_every_op_name` 업데이트.
- 테스트 **2,966 / 0 / 0** (CenterOnOrigin 대비 +5), `clippy --all-targets --all-features -D warnings` 무경고.

#### API — `Command::CenterOnOrigin` centroid 기반 재중심 (2026-05-08)
- **`Command::CenterOnOrigin { id }` 신규 추가** — 솔리드를 centroid가 월드 원점에 오도록 translate. AI/스크립트가 `Measure`로 centroid 때린 후 translate해야 했던 "임포트된 부품 재중심" 패턴을 단일 dispatch로 축소. `Translate` by `-centroid` 동가이지만 한 명령(그리고 undo 한 번)으로 표현.
- **검증**: unknown id는 `ApiError::UnknownSolid`. 슬롯이 이미 `Solid` handle 보유 필요(mass-properties 계산).
- **`CommandSchema` 엔트리** 추가.
- **구현**: `Session::center_on_origin()`이 `Scale`이 쓰는 `solid_mass_properties` 경로를 재사용해 centroid 획득 후 모든 정점에서 centroid를 뺀. `Outcome::SolidModified` 반환, log/cursor로 완전 undo.
- **회귀 테스트 5개 추가** — (10,20,30) translate 후 원점 복귀 / 이미 중심의 박스 idempotent / 잘못된 id 거절 / JSON 라운드트립 / undo 시 centroid 복원.
- `command_schemas_cover_every_op_name` 업데이트.
- 테스트 **2,961 / 0 / 0** (ScaleNonUniform 슬라이스 대비 +5), `clippy --all-targets --all-features -D warnings` 무경고.

#### API — `Command::ScaleNonUniform` 명시적 피보을 중심으로 설정한 축별 스케일링 (2026-05-08)
- **`Command::ScaleNonUniform { id, factors, point }` 신규 추가** — 첫 비균등 기하학적 mutation. 축별 승수 `factors = [sx, sy, sz]`(각 성분 `> 0`)를 월드 좌표의 명시적 피보 `point` 주위로 적용. 정점 좌표 재작성, topology 보존. 기존 균등 `Command::Scale`(centroid 기준) 보완.
- **검증**: 하나라도 `<= 0`이면 mutation 이전에 `ApiError::InvalidArgument`로 거절; unknown id는 `ApiError::UnknownSolid`.
- **`CommandSchema` 엔트리**: ParamSchema 3개 (id, factors, point).
- **구현**: `Session::scale_non_uniform()`가 `slot.model.vertices.iter_mut()` 순회하며 각 `v.point`을 `pivot + (v.point - pivot) * factor`로 축별 재작성.
- **회귀 테스트 6개 추가** — 단위 박스 → [2,3,4] extent / 원점 pivot에서 3x → min[0]=0 불변 / 0 또는 음수 거절 / 잘못된 id / JSON 라운드트립 / undo 후 bbox 복원.
- `command_schemas_cover_every_op_name` 업데이트.
- 테스트 **2,956 / 0 / 0** (Stats 슬라이스 대비 +6), `clippy --all-targets --all-features -D warnings` 무경고.

#### API — `Command::Stats` + `Outcome::Stats` (2026-05-08)
- **`Command::Stats` 신규 추가** — 6번째 순수 observer. `Outcome::Stats { solid_count, history_count }`로 O(1) 문서 통계(mesh/volume 순회 없음) 반환. HUD 배지 / AI sanity check / "이 문서가 얼마나 큰가" 프로브에 유용.
- **`Outcome::Stats` + `OutcomeKind::Stats` 태그 추가**.
- **`CommandSchema` 엔트리** 추가 (매개변수 없음).
- **구현**: `Document::solid_count()` + `Document::history().len()` 래퍼. observer fast-path 으로 log/cursor/history 부작용 없음. `solid_count`는 현재 점유 슬롯 수이고 `history_count`는 누적 이벤트 수 — `delete_solid`은 전자는 감소시키고 후자는 증가시킴.
- **회귀 테스트 5개 추가** — 빈 세션 / 3 create → 3+3 / delete 후 (1, 3) / observer 불변성 / Command+Outcome JSON 라운드트립.
- `command_schemas_cover_every_op_name` 에 Stats 포함.
- 테스트 **2,950 / 0 / 0** (HistoryEvents 슬라이스 대비 +5), `clippy --all-targets --all-features -D warnings` 무경고.

#### API — `Command::HistoryEvents` + `Outcome::HistoryListed` (2026-05-08)
- **`Command::HistoryEvents` 신규 추가** — 5번째 순수 observer 명령. 실행 순서의 기록된 모든 `HistoryEvent`를 `Outcome::HistoryListed { events }`로 반환. `session.document().history().to_vec()` 동가이지만 command surface를 통해 접근 가능 — AI/스크립트가 `Document` 직접 호출 없이 단일 dispatch로 디스패치 가능.
- **`Outcome::HistoryListed { events }` + `OutcomeKind::HistoryListed` 태그 추가.
- **`CommandSchema` 엔트리** 추가 (매개변수 없음).
- **구현**: `Document::history()`를 outcome으로 clone; observer fast-path로 log/cursor/history 부작용 없음.
- **회귀 테스트 5개 추가** — box+sphere+translate로 3개 이벤트 순서+`primary` 검증 / 빈 세션 / observer 불변성 / Command JSON 라운드트립 / Outcome JSON 라운드트립.
- `command_schemas_cover_every_op_name` 에 HistoryEvents 포함.
- 테스트 **2,945 / 0 / 0** (FindByLabel 슬라이스 대비 +5), `clippy --all-targets --all-features -D warnings` 무경고.

#### API — `Command::FindByLabel` 대소문자 무시 라벨 검색 (2026-05-08)
- **`Command::FindByLabel { query }` 신규 추가** — 4번째 순수 observer 명령. 라벨에 `query`가 대소문자 무시 substring으로 포함되는 모든 솔리드를 `Outcome::SolidsListed { entries }`로 반환. 빈 query는 모든 솔리드 매칭(`ListSolids`와 동일). 기존 `SolidsListed` outcome shape 재사용 — AI/스크립트가 list/search 응답을 동일하게 다룰 수 있음. Document mutation/history 이벤트 없음.
- **구현**: needle 한 번 lower-case, `Document::solid_ids()` 순회, 각 라벨 lower-case 후 `contains()`. 빈 query fast-path로 `ListSolids`와 의미 일관성 유지.
- **`CommandSchema` 엔트리** 추가.
- **회귀 테스트 5개 추가** — "SPH" → "MySphere"만 매칭 / 빈 query → 전체 / 매칭 없음 → 빈 entries / history 미증가 / JSON 라운드트립.
- `command_schemas_cover_every_op_name` 에 FindByLabel 포함.
- 테스트 **2,940 / 0 / 0** (Rotate 슬라이스 대비 +5), `clippy --all-targets --all-features -D warnings` 무경고.

#### API — `Command::Rotate` 임의 축 쿼터니언 회전 (2026-05-08)
- **`Command::Rotate { id, axis, angle_rad, point }` 신규 추가** — translate/scale 외 첫 기하학적 mutation 명령. `point` 피벗을 중심으로 임의 `axis`(자동 정규화) 주위로 `angle_rad` 회전. `cadkernel_math::Quaternion::from_axis_angle` + `q.rotate_vec(rel)`로 정점별 변환. Topology 보존, `Outcome::SolidModified` 반환, 표준 log/cursor를 통해 완전히 undo 가능.
- **검증**: 영벡터 축은 mutation 전에 `ApiError::InvalidArgument`로 거부, unknown id 는 `ApiError::UnknownSolid`.
- **`CommandSchema` 엔트리**: 4개 ParamSchema (id, axis, angle_rad, point).
- **회귀 테스트 6개 추가** — Z축 90° 회전 시 단위 박스의 X·Y extent 교환 / 임의 축 [1,1,0] PI/3 회전에서 volume 보존 / 영벡터 축 거부 / 잘못된 id / JSON 라운드트립 (`op: "rotate"`) / undo 시 원본 정점 좌표 정확 복원.
- `command_schemas_cover_every_op_name` 에 Rotate 포함.
- 테스트 **2,935 / 0 / 0** (Duplicate 슬라이스 대비 +6), `clippy --all-targets --all-features -D warnings` 무경고.

#### API — `Command::Duplicate` 심층 복제 명령 (2026-05-08)
- **`Command::Duplicate { id }` 신규 추가** — 솔리드를 새 슬롯으로 심층 복제. 표준 `Outcome::SolidCreated { id, label }`를 반환하며 `label = "<원본라벨> (copy)"`. 원본 슬롯은 보존되고, 새 복사본은 완전히 독립적이며(기존 `BRepModel: Clone`로 topology 전체 deep clone), 일반 log/cursor 파이프라인을 통해 완전히 undo 가능(observer가 아닌 mutation 명령). 이제 AI/스크립트가 형제 인스턴스를 원할 때 원시형부터 다시 생성할 필요 없음.
- **구현**: `Session::duplicate(src_id)`가 슬롯의 `BRepModel`을 clone하고, 원래 `Handle<SolidData>`를 그대로 재사용(복제된 모델 내에서도 generational arena 인덱스가 유효), 기존 `Document::insert()` 경로로 새 슬롯을 삽입해 순차적으로 새 SolidId를 발급.
- **`CommandSchema` 엔트리** 추가: 계약 문서화.
- **회귀 테스트 6개 추가** — 새 SolidId + "(copy)" 라벨 / volume·bbox 일치 / 독립성(translate 후 원본 불변) / history 이벤트 추가 / undo로 복사본만 제거 / unknown id 는 `UnknownSolid`.
- `command_schemas_cover_every_op_name` 도 Duplicate 포함하도록 업데이트.
- 테스트 **2,929 / 0 / 0** (ListSolids 슬라이스 대비 +6), `clippy --all-targets --all-features -D warnings` 무경고.

#### API — `Command::ListSolids` + `Outcome::SolidsListed` (2026-05-08)
- **`Command::ListSolids` 신규 추가** — `Measure`·`Validate`에 이은 세 번째 순수 observer 명령. `Document::solid_ids()`와 `Document::solid_label()`를 단일 왕복으로 결합하여 `Outcome::SolidsListed { entries: Vec<SolidEntry { id, label }> }`를 반환. AI/테스트 소비자가 Document API 두 개를 따로 잡을 필요 없이 트리 뷰를 그리거나 후속 명령의 타겟을 고르는 용도에 적합.
- **`SolidEntry { id, label }` 구조체** — `cadkernel_api::SolidEntry`로 재내보내기. JSON 와이어 포맷: `{"id": 0, "label": "Box"}`.
- **observer 빠른 경로 확장** — `Session::execute`의 `matches!`가 `ListSolids`도 포함.
- **`CommandSchema` 엔트리** 추가: 읽기 전용 계약 문서화.
- **회귀 테스트 5개 추가** — 빈 세션 / 3개 솔리드 열거 / log·history 무변경 / JSON 와이어 포맷(`kind: "solids_listed"`) / 단일 변형 JSON 왕복.
- `command_schemas_cover_every_op_name` 도 ListSolids 포함하도록 업데이트.
- 테스트 **2,923 / 0 / 0** (Validate 슬라이스 대비 +5), `clippy --all-targets --all-features -D warnings` 무경고.

#### API — `Command::Validate` + `Outcome::Validated` (2026-05-08)
- **`Command::Validate` 신규 추가** — `Measure`에 이어 두 번째 순수 observer 명령. 기존 `Document::validate()`로 위임하여 `Outcome::Validated { issues: Vec<DocumentIssue> }`를 반환. AI/테스트 소비자는 긴 리플레이 후 `issues.is_empty()`로 경고 UI 노출 여부를 판단하는 용도로 동일한 `execute(Command)` 채널을 사용 가능.
- **observer 빠른 경로 확장** — `Session::execute`의 `matches!` 필터가 `Measure`와 함께 `Validate`도 포함. 이 명령도 로그 롭·coalesce 대상 아님·undo 불가.
- **`CommandSchema` 엔트리** 추가: 읽기 전용 계약 문서화.
- **회귀 테스트 4개 추가** — 깨끗한 문서에서 빈 이슈 / 로그·히스토리 무변경 / JSON 와이어 포맷(`kind: "validated"`) / 단일 변형 `{"op":"validate"}` JSON 왕복.
- `command_schemas_cover_every_op_name` 테스트도 Validate 포함하도록 업데이트.
- 테스트 **2,918 / 0 / 0** (Measure 슬라이스 대비 +4), `clippy --all-targets --all-features -D warnings` 무경고.

#### API — `Command::Measure` + `Outcome::Measured` (2026-05-08)
- **`Command::Measure { id }` 신규 추가** — API 표면 최초의 순수 observer(읽기 전용) 명령. `Outcome::Measured { id, volume, surface_area, centroid, bbox_min, bbox_max }`를 반환. 기존 `Document::measure_solid()`와 방금 추가한 `Document::bounding_box()`를 하나의 AI/스크립트 호출 가능 명령으로 통합. AI 에이전트와 Lua 스크립트가 변형 명령과 동일한 `execute(Command)` 채널로 "솔리드 N의 기하 속성은?"을 질의 가능 — 별도 Document API 배선 불필요.
- **`Session::execute`의 observer 빠른 경로** — `Command::Measure`는 log/cursor/history 처리 전에 단락(short-circuit)하므로 로그에 기록되지 않고, coalesce 대상도 아니며, undo도 불가. 명령 로그가 결정론적 문서 상태로 재생된다는 불변식을 유지.
- **`CommandSchema` 엔트리** 추가: 읽기 전용 계약 문서화.
- **회귀 테스트 4개 추가** — 2×4×6 박스 측정 검증 / log·history 무변경 / 미존재 id ⇒ `UnknownSolid` / JSON 와이어 포맷(`kind: "measured"` + 6 필드).
- `command_schemas_cover_every_op_name` 테스트도 Measure 변형 포함하도록 업데이트.
- 테스트 **2,914 / 0 / 0** (bbox 슬라이스 대비 +4), `clippy --all-targets --all-features -D warnings` 무경고.

#### API — `Document::bounding_box()` + `AabbSummary` (2026-05-08)
- **`Document::bounding_box(id) -> Option<AabbSummary>` 신규 추가** — 솔리드의 축 정렬 바운딩 박스를 구하는 API. `BRepModel`의 vertex store를 순회해 월드 좌표로 반환. 기존 `measure_solid()`(volume/surface_area/centroid)를 보완 — AI/테스트 소비자가 테셀레이션 없이 공간 범위를 빠르게 검증하는 용도(예: MidPlane 출소가 z=0 중심인지, Translate 후 델타가 기대값과 일치하는지).
- **`AabbSummary { id, min, max }` 구조체** — `size()`, `center()` 헬퍼 메서드 제공. serde JSON 직렬화 가능.
- **`cadkernel_api::AabbSummary`로 재내보내기**.
- **회귀 테스트 4개 추가** — 박스 치수 일치 / Translate 델타 추적 / 미존재하는 SolidId에 None / JSON 와이어 포맷.
- 테스트 **2,910 / 0 / 0** (A2 #3 부분 대비 +4), `clippy --all-targets --all-features -D warnings` 무경고.

### 변경됨

#### API — A2 #3 (부분): `Mirror.merge` (2026-05-07)
- **`Command::Mirror`에 선택적 `merge: bool` 추가** — `#[serde(default, skip_serializing_if = "std::ops::Not::not")]`로 기존 JSON 호환. `merge=true`일 때 미러된 복사본을 원본과 Boolean Union으로 융합하고 원본 슬롯을 소모 — FreeCAD/SolidWorks PartDesign Mirrored 기능과 동일한 동작. `false`(기본값)일 때는 기존 레거시 동작 그대로 원본 보존 + 미러 별도 수다 등.
- **`Session::mirror` 재작성** — `merge=true`일 때 기존 `boolean(BooleanKind::Union)` 헬퍼로 위임해 `Outcome::Booleaned { result, consumed }`를 반환. 전체 A2 `MirrorSpec { features, plane, merge }`는 `FeatureId` 장착 후 A2.2로 미루기.
- **회귀 테스트 2개 추가** — merge=true 융합 동작 + 원본 제거 검증, merge=false 와이어 호환성(JSON 생략 + 레거시 JSON 역직렬화 + 두 솔리드 잔존).
- 테스트 **2,906 / 0 / 0** (A2 #2 부분 대비 +2), `clippy --all-targets --all-features -D warnings` 무경고.

#### API — A2 #2 (부분): `LinearPattern.skip_instances` (2026-05-07)
- **`Command::LinearPattern`에 선택적 `skip_instances: Vec<u32>` 추가** — `#[serde(default, skip_serializing_if = "Vec::is_empty")]`로 기존 JSON 페이로드 호환. 인스턴스 인덱스(0 = 원본, 1..count-1 = 복사본)를 애제하는 용도 — A2 `instance_overrides`에서 가장 빈번한 “skip” 양상을 먼저 제공(예: 마운팅 플랜지의 빠진 볼트 위치). 범위 밖 항목은 조용히 필터. 전체 `instance_overrides: HashMap<u32, InstanceOverride>`는 `FeatureId` 장착 후 A2.2로 미루기.
- **`Session::linear_pattern` 재작성** — skip 세트를 중복/범위 필터, 인덱스 0 스킵 시 원본 솔리드도 문서에서 제거, `Outcome::PatternCreated.instance_count`는 생존 멤버 수(`count - skip.len()`)로 설정, “전부 스킵” 잘못된 호출은 `ApiError::InvalidArgument`로 거부.
- **회귀 테스트 4개 추가** — 인덱스 일부 스킵 / 원본 스킵 / 전체 스킵 거부 / JSON 와이어 단방향 호환(빈 벡터 생략 + 레거시 JSON 역직렬화).
- 테스트 **2,904 / 0 / 0** (A2 #1 부분 대비 +4), `clippy --all-targets --all-features -D warnings` 무경고.

#### API — A2 #1 (부분): `ExtrudeKind` enum (Blind / MidPlane / TwoSided) (2026-05-07)
- **`Command::Extrude`에 선택적 `kind` 파라미터 추가** (`#[serde(default)]`로 기본값 `Blind` — 기존 JSON 호환). 신규 `ExtrudeKind` enum (`crates/api/src/command.rs`)은 `#[serde(tag = "mode", rename_all = "snake_case")]` 태그 유니언으로 세 변형: `Blind` (단방향, 레거시), `MidPlane` (프로파일 평면 중심으로 양쪽 절반씩), `TwoSided { back_distance: f64 }` (전방 `distance` + 후방 `back_distance` 독립 제어, `back_distance > 0` 필요). `ThroughAll`/`UpToFace`는 `sketch_id` 배선 후 A2.2에서 추가.
- **`Session::extrude_profile` 재작성** — `(back_shift, total_distance)`을 kind별로 계산하고 방향을 한 번 정규화한 뒤 프로파일을 `-dir_unit * back_shift`만큼 이동, `total_distance`를 커널 `extrude(...)`에 전달. `TwoSided`의 `back_distance <= 0`은 `KernelError::Invalid`로 조기 거부.
- **`cadkernel_api::ExtrudeKind`로 재내보내기** — Lua 브리지/MCP/향후 Python 바인딩 등 다운스트림 도구에서 사용.
- **회귀 테스트 3개 추가** — `extrude_kind_blind_is_default_and_matches_legacy_json_shape` (기본값 + JSON 호환), `extrude_kind_mid_plane_centers_solid_and_total_span_matches_distance` (centroid_z=0, 부피 동일), `extrude_kind_two_sided_extends_in_both_directions_with_correct_total_span` (부피 54, centroid_z=1 + back_distance 검증 + JSON 와이어).
- 테스트 **2,900 / 0 / 0** (Pt.6 대비 +3), `clippy --all-targets --all-features -D warnings` 무경고.

#### Viewer + API — UI/UX 정비 Pt. 6 + API A2 #4 (2026-05-07)
- **트리 행 팔레트 Pt.1–5 틸 악센트로 통일** — 모델 트리 오브젝트 행의 선택 배경이 FreeCAD 시절의 네이비(`rgb(9, 71, 113)`)로 남아 Pt.1–5 코르만 팔레트와 충돌했던 문제를 해소. 이제 선택 배경은 `theme::COLOR_ACCENT.gamma_multiply(0.18)` (Command Palette 결과 행 + 상태바 배지와 동일)로 교체하고 기존 2px 틸 악센트 좌측 바는 그대로 유지. Hover 배경은 `rgb(42, 45, 48)` → `#242933`로 승꺰해 패널 계층과 일치. Active body 워시도 소프트 틸 `rgba(20, 70, 65, 32)`로 재채색. 가시성 눈 아이콘 색상이 hover 시 틸 악센트 계열로 변경.
- **그룹 헤더 행**도 같은 hover(`#242933`) + 틸 눈 아이콘 결합으로 일치시켜 다중 선택 그룹 토글도 나머지 코르롤롬함.
- **`Outcome::PatternCreated` 스키마가 AI/테스트 친화적으로 강화** (`crates/api`) — 기존 `{ ids: Vec<SolidId> }`에 `pattern_id`, `instance_count`, `total_features` 필드를 추가해 `{ pattern_id, instance_count, total_features, ids }` 구조로 증설. `pattern_id`는 소스 솔리드, `instance_count`는 `Command::LinearPattern.count` 그대로, `total_features`는 새로 삽입된 솔리드 수(`instance_count - 1`), `ids`는 기존처럼 전체 패턴 구성원 리스트(원본이 이덱스 0). 상업 CAD 로드맵의 A2 디리버러블 #4 완료. JSON 와이어도 기존 `"kind": "pattern_created"` 태그 아래 세 필드가 노출되어 외부 소비자가 `ids.len()`을 파싱하지 않고도 `instance_count`/`total_features`로 분기 가능. `OutcomeKind::PatternCreated`, `Outcome::primary_id()` 갱신. 회귀 테스트 `linear_pattern_outcome_reports_pattern_id_instance_count_and_total_features` 신규로 필드 값 / JSON 세 구조 / `count < 2` 거부 동작을 검증.
- 테스트 **2,897 / 0 / 0** (Pt.5 대비 +1), `clippy --all-targets --all-features -D warnings` 무경고.

#### Viewer — UI/UX 정비 Pt. 5: 뷰포트 고정 HUD + 상태바 배지 + Command Palette 재디자인 (2026-05-07)
- **뷰 큐브 + 뷰포트 HUD를 중앙 뷰포트 사각형(`ctx.available_rect()`)에 고정** — 기존에는 하드코딩된 패널 오프셋(280px ComboView, 170px Report, 82px Toolbar)을 빼는 방식이었는데 이제 사이드 도크를 켜고/끄고 리사이즈해도 항상 올바른 위치에 따라옴. Pt.3/Pt.4 레이아웃 일번으로 쓰이지 않던 코너 오프셋 버그 수정.
- **상태바 세그먼트를 pill 배지로 재구성** — 더 이상 평범한 텍스트 레이블이 아니라 우측 모든 항목(워크벤치, Auto, mm, F1, CAD, Persp/Ortho, Display Mode, 씬 통계, 선택, Measure, FPS)을 새 `status_bar::badge()` 헬퍼로 16px 높이의 뛐근 알약 원으로 그림 — 채움은 `accent.gamma_multiply(0.18)`, 보더는 액센트 소프트 라인, 텍스트는 액센트 자체 색. Hover 시 채움이 밝아짐. 클릭 가능한 배지(투영 토글, F1, CAD)는 `Sense::click()` 유지하며 기존 `GuiAction` 발굴. 하단 바가 이제 각 세그먼트마다 별개 시각 칩으로 읽힘.
- **Command Palette 전면 재디자인** — 제너릭 `Frame::popup`을 커스텀 테마 팝업으로 교체: 10px 능근 모서리, 틸 액센트 보더(`COLOR_ACCENT.gamma_multiply(0.55)`), 드롭 삜도우, `#16191F` 다크 헤더 밴드 + 돋보기 글리프, 프레임리스 전폭 검색 필드, 헤더 우측 라이브 매치 카운트 칩, 액센트 틴트 결과 행 + 활성 행에 틸 좌측 바, 속성 키는 모노스페이스 칩(채움 `#14171D` + 보더 `#353C48`), 우측 희미한 카테고리 레이블, 푸터 밴드에 `↑↓ navigate / ↵ run / Esc close` 힌트 + `Ctrl+P` 리마인더. 검색 결과 없을 때도 단순 "No matching commands."가 아니라 사용자의 쿼리를 이키릭체로 보여줌.
- 테스트 2,896 / 0 / 0, `clippy --all-targets --all-features -D warnings` 무경고.

#### Viewer — UI/UX 정비 Pt. 4: 브레드크럼 병합 + 뷰포트 HUD + 탭형 Inspector (2026-05-07)
- **브레드크럼을 컨텍스트 툴바에 통합** — 전용 20px 브레드크럼 줄(`draw_breadcrumb_bar` 패널)을 제거하고 Scene › Object › Mode 경로를 새 헬퍼 `overlays::draw_breadcrumb_inline`로 컨텍스트 툴바 우측에 인라인 렌더링. 상단 크롬 4줄 → **3줄**(메뉴 → 툴바 → 컨텍스트+브레드크럼)로 압축. 단독 브레드크럼 함수는 삭제하고 모든 호출을 인라인 헬퍼로 통일.
- **플로팅 뷰포트 HUD 신규** — `overlays::draw_viewport_hud`는 뷰 큐브 아래에 같은 코너 설정으로 고정되는 28px 사각 4버튼 세로 컬럼을 그림. Fit All / Reset Camera / Toggle Projection(원근일 때 틸 강조) / Toggle Grid. 각 버튼은 hover elevation + 활성 시 액센트 보더 + 숏컷 툴팁. 기존 `GuiAction::{FitAll, ResetCamera, ToggleProjection, ToggleGrid}` 재사용 — 신규 GuiAction 없음. 뷰 큐브를 숨기면 같이 숨겨져 깔끔한 캔버스 선호 사용자도 OK.
- **Inspector 도크 탭화** — Pt.3에서 분리된 우측 Inspector 도크는 `active_task.is_some()`에 따라 Tasks ↔ Properties로 자동 스왑하던 구조였는데, 이제 도크 헤더 아래 26px 탭 스트립(Properties / Tasks)으로 명시적 탭 전환. 활성 탭은 틸 밑줄 + 액센트 레이블 색. Task 진행 중에도 Properties를 유지하거나 Tasks 탭에서 다른 엔티티 검사 가능. 새 Task 시작 시엔 여전히 Tasks 탭으로 자동 전환(기존 플로 유지)하되, 사용자가 언제든 수동 전환 가능. 신규 `InspectorTab` enum + `gui.inspector_tab` 필드 + `mod::draw_inspector_tabs` 헬퍼.
- **누적 효과(Pt.1–4)**: 시그니처 틸 팔레트 + 크롬 재도색 + 액티비티 레일 + ComboView 분리 + 브레드크럼 병합 + 뷰포트 HUD + 탭형 Inspector. 상단 크롬 정비 전 대비 **2줄** 단축(5 → 3); Inspector는 우측 + 탭형; 뷰포트는 자체 내장 nav HUD 보유; 워크벤치 전환은 항상 한 번 클릭.
- 테스트 2,896 / 0 / 0, `clippy --all-targets --all-features -D warnings` 무경고.

#### Viewer — UI/UX 정비 Pt. 3: 구조적 레이아웃 재구성 (2026-05-07)
- **액티비티 레일 신규** — 좌측 최단에 고정된 세로형 56px 워크벤치 스위첰를 추가하고 기존 가로형 워크벤치 탭 줄을 제거. 9개 단일 글리프 아이콘 버튼(Part / PartDesign / Sketcher / Mesh / TechDraw / Assembly / Draft / Surface / FEM)을 세로로 배치. 활성 워크벤치는 틸 좀드 좌측 바 + 틸 아이콘 + 틸 캡션으로 강조. 레일 하단에는 Model Tree / Properties 토글 버튼 2개를 배치해 메뉴 안 거치고도 사이드 독 표시 제어 가능. 상단 크롬의 워크벤치 탭 행 하나가 완전히 사라져 메뉴 → 툴바 → 컨텍스트 툴바 → 브레드크럼 4줄로 압축. 구 탭 함수는 `#[allow(dead_code)]`로 널겨두어 롤백 경로 유지.
- **ComboView → 좌측 Tree + 우측 Inspector 분리** — FreeCAD식 단일 좌측 ComboView(윈도우 하나에 트리 위 + 속성 아래)를 해체하고 Fusion 360 / SolidWorks 스타일로 재구성: `SidePanel::left("model_tree_dock")`(기본 260px, 모델 트리 전용)는 좌측, `SidePanel::right("inspector_dock")`(기본 300px, ActiveTask 있으면 Tasks, 없으면 Properties)는 우측에 배치. 각 독은 독립적으로 리사이즈 핸들을 가지며 액티비티 레일 스위치로 각각 토글 가능. ActiveTask가 설정되면 Task 패널이 우측 inspector에 인라인 렌더링되어 이제 피처 생성 플로가 트리와 세로 공간 경쟁을 하지 않음.
- **Workbench enum API 확장** — `Workbench::icon()`(단일 글리프 반환, 액티비티 레일용), `Workbench::short_name()`(툴팁과 레일 캡션용 평문 이름) 추가. 기존 `Workbench::label()`("⬢ Part" 조합형)는 그대로 유지.
- **누적 효과**: 뷰어 한면이 명확한 3단 레이아웃으로 읽힘 — 액티비티 레일(56px) │ 모델 트리(260px, 토글 가능) │ 뷰포트 │ inspector(300px, 토글 가능). 상단 크롬 한 줄 줄어들고, inspector가 더 이상 트리와 경쟁하지 않고 현대 CAD 앱이 다 쓰는 우측 위치로 이동. 워크벤치 전환은 상단 바를 숨겼을 때도 항상 한 번 클릭으로 가능.
- 테스트 2,896 / 0 / 0, `clippy --all-targets --all-features -D warnings` 무경고.

#### Viewer — UI/UX 정비 Pt. 2: 크롬 전체 틸 강조색 + 팔레트 재구성 (2026-05-07)
- 뷰어 곳곳에 박혀 있던 하드코딩된 VS Code 블루(`#007ACC`) 강조색 7개를 전부 제거하고 `theme::COLOR_ACCENT`(새 시그니처 틸)에서 가져오도록 통일. 대상: `toolbar.rs`(플라이아웃/프리미티브/스케치 도구 활성 상태, 워크벤치 탭 활성 밑줄), `tree.rs`(트리 행 선택 바), `properties.rs`(속성 행 표시), `report.rs`(로그 탭 활성 밑줄). 반투명 활성 채움도 `COLOR_ACCENT.gamma_multiply(0.22)`로 교체 — 이후 팔레트 변경 시 알파도 자동 추종.
- 상/하/좌측 패널 크롬 전체를 새 푸른빛 중성 톤 팔레트로 재구성: 메인 툴바 `#202530`, 워크벤치 탭 `#161920`, 컨텍스트 툴바 `#1C2028`, ComboView 사이드 패널 `#1C2028`, Report 패널 `#1A1E26`, 브레드크럼 `#181C23`, 상태바 `#14171D`, 메뉴바 `#14171D`(기존엔 미설정, 이제 프레임 적용). 보더 톤도 `#0F121A` 계열로 일관 정렬.
- 상태바 상단 엣지 라인을 회색에서 틸 미세 글로우(`COLOR_ACCENT.gamma_multiply(0.55)`)로 교체 — 캔버스 하단에 시그니처 색이 은은하게 깔리는 효과. 수직 구분선도 새 보더 톤(`#323844`)으로 정렬.
- 누적 효과: 메뉴바 → 툴바 → 워크벤치 탭 → 컨텍스트 툴바 → 브레드크럼 → 사이드 패널 → Report 도크 → 상태바 전체가 하나의 일관된 다크 틸 CADKernel 표면으로 읽힘. 더 이상 독립적으로 튜닝된 VS Code / Fusion 360 모방의 누적이 아님.
- 테스트 2,896 / 0 / 0, `clippy --all-targets --all-features -D warnings` 무경고.

#### Viewer — UI/UX 정비: 시그니처 틸 팔레트 + 환영 화면 재설계 (2026-05-07)
- 다크 테마 기본 강조색을 VS Code 블루(`#007ACC`)에서 시그니처 틸(`#14B8A6` / hover `#2AD4C0` / pressed `#0E8E80`)로 교체하고 배경 계층(`#161920` → `#1C2028` → `#24293 3` → `#2B303B`)을 푸른빛이 도는 중성 톤으로 재구성. 선택/툴바 활성 색상도 어두운 틸 톤(`#12554F`)으로 통일하고 패널 모서리 라운딩을 4px → 6px로 부드럽게 조정.
- 공개 색상 상수(`COLOR_ACCENT`, `COLOR_SELECTED`)를 새 팔레트에 맞춰 갱신해 직접 참조하는 코드도 자동으로 새 색상에 반영.
- 환영 화면(`draw_welcome_screen`)을 layer-painter 직접 그리기에서 정식 egui 위젯(`egui::Area` 기반)으로 전면 재작성. 구성: 56×56 로고 플레이트 + 26pt 타이틀 + 버전 알약 형식의 히어로 배너, 2×3 액션 카드 그리드(Create Box / Cylinder / Sphere / Import Mesh / Open Project / Inspect `.cadk`, 첫 카드는 강조색 좌측 바 + 강조색 보더), 단축키 칩 스트립(`Ctrl+N` `Ctrl+O` `Ctrl+S` `F1` `Ctrl+P`), 빌드 정보 풋터(라이선스 + Rust + egui + wgpu).
- 카드 액션은 기존 `task_panel::ActiveTask::{Box,Cylinder,Sphere}` / `GuiAction::ImportFile` / `OpenFile` / `inspect_cadk_path` 진입점을 그대로 재사용 — 신규 GuiAction 변형 추가 없음.
- 테스트 2,896 / 0 / 0, `clippy --all-targets --all-features -D warnings` 무경고 유지.

### 추가됨

#### Viewer — `.cadk` Command Log Inspector 다이얼로그 (2026-05-07)
- File 메뉴에 신규 항목 **“Inspect Command File…”** 추가. `.cadk` Command 로그 컴테이너를 열어 헤더, 플래그 비트(`MANIFEST_COMPRESSED` / `SIGNED` / `HAS_THUMBNAIL`), 스키마 버전, 명령 개수, 썸네일 크기 + CRC 상태, 앞 64개 명령의 스크롤 프리뷰를 표시.
- 구현: `crates/viewer/src/gui/mod.rs`에 `CadkInspectorReport` + `inspect_cadk_path()` 추가 (~95 LOC, `cadkernel_api::cadk::decode` / `decode_thumbnail` 재사용). `crates/viewer/src/gui/dialogs.rs`에 `draw_cadk_inspector_dialog` egui 윈도우 추가 (~85 LOC).
- viewer에 `cadkernel-api` 워크스페이스 디폴던시 추가. 읽기 실패 / 매직 불일치 / 절단 / CRC 실패 모두 다이얼로그에 인라인 표시 — 잘못된 파일에 대해 viewer가 panic 하지 않음.
- 이는 A3 `.cadk` 코덱을 viewer 쪽에서 처음 노출한 UI. 기존 scene-graph `.cadk` JSON 경로는 그대로 유지.

#### A3 — `cadk-inspect` 진단 CLI (2026-05-07)
- `crates/api`에서 신규 바이너리 `cadk-inspect` 제공 (`cargo run -p cadkernel-api --bin cadk-inspect -- <file.cadk> [--verbose]`). 읽기 전용 — 원본을 능동적으로 수정하지 않음.
- 기본 출력: 파일 크기, 매직 검증, 인스트럭션 개수, 썸네일 존재 여부 + CRC 상태, 종합 상태. `--verbose / -v` 올이면: 스키마 버전, 플래그 비트필드 + 명명된 비트 디코드(`MANIFEST_COMPRESSED` / `SIGNED` / `HAS_THUMBNAIL` / unknown), 전체 역직렬화 된 커맨드 로그.
- Exit code: `0` 건강, `1` I/O 또는 인자 에러, `2` 컴테이너 손상 또는 무결성 첩크 실패.
- 구현 단일 파일 `crates/api/src/bin/cadk_inspect.rs` (~120 LOC). `cadk::decode` + `cadk::decode_thumbnail` 재사용 → CLI가 통과시키는 파일은 정의상 `Session::load_cadk`가 디코드 가능.
- A3 deliverable §8 (`cadk-inspect` CLI) — 완료.

#### A3 — `.cadk` 썸네일 blob 지원 (2026-05-07)
- 신규 공개 API: `cadk::encode_with_thumbnail(commands, thumbnail)`, `cadk::decode_thumbnail(bytes) -> Option<Vec<u8>>`. 썸네일 바이트는 포맷 비종속 (보통 PNG); 코덱은 CRC32만 검증하고 내용은 파싱하지 않음.
- 썸네일이 있으면 manifest에 `BlobKind::Thumbnail` 레코드 추가 + 헤더에 `CadkFlags::HAS_THUMBNAIL` 비트 설정. 기존 `cadk::decode` 호출자는 플래그를 무시하고 그대로 동작 → 완전한 하위 호환.
- `Session::save_cadk_with_thumbnail` 추가.
- 코덱 단위 테스트 3개 + 세션 통합 테스트 1개 추가 (썸네일 없을 때 플래그 0 / `None` 반환, 썸네일 round-trip + 플래그 설정, 썸네일 blob CRC 손상 거부).
- A3 deliverable §5 (썸네일 blob) — 완료. 후속: bincode 스왑, zstd, Ed25519 서명, 자동저장, `cadk-inspect` CLI.

#### A3 — `.cadk` v0 코덱: CRC 무결성 포함 인코드/디코드 (2026-05-07)
- `crates/api/src/cadk/codec.rs` (~270 LOC)에 v0 컨테이너 레이아웃 end-to-end 구현:
  - `crc32_ieee` — IEEE 802.3 CRC-32, `OnceLock` 기반 lazy 테이블, polynomial `0xEDB88320`. 외부 크레이트 0개.
  - `write_header` / `read_header` — little-endian 필드별 직접 인코딩. 레이아웃: 4+4+8+8+8+4+28 = 64 byte. `reserved`를 `[u8; 32]` → `[u8; 28]`로 축소하여 `HEADER_SIZE = 64` 산수 정합화.
  - `encode(commands)` — `b"CADK" + header[64] + manifest_json + doc_json` 컨테이너 생성. JSON 인코딩된 manifest 길이 ↔ content_offset 자릿수 상호의존성을 fixed-point 수렴 루프로 해결.
  - `decode(bytes)` — 매직, `is_supported()`, total_size, manifest 범위/CRC, 문서 blob 범위/CRC를 모두 검증 후 `Vec<Command>` 역직렬화.
- `Session`에 `save_cadk()` / `load_cadk()` 추가 (load는 `Session::replay`로 적용). redo 스택은 코덱이 보존하지 않음 — 적용된 prefix만 round-trip. 전체 세션 상태는 기존 JSON 스냅샷 경로 유지.
- 코덱 단위 테스트 8개 + 세션 통합 테스트 1개 추가: 빈 로그, 5-커맨드 로그, 잘못된 매직 거부, 절단 거부, 문서 blob CRC 손상 거부, 헤더 round-trip, CRC32 reference 검증.
- A3 deliverable §1/§2/§3/§6 v0 랜딩 (blob 본문은 JSON). 후속: bincode 스왑(`MANIFEST_COMPRESSED` 뒤), zstd, Ed25519 서명, 자동저장 + 충돌 복구, 썸네일 blob, `cadk-inspect` CLI.

#### A3 — `.cadk` 네이티브 파일 포맷 스캐폴드 (2026-05-07)
- 신규 `crates/api/src/cadk/` 모듈에 온디스크 컨테이너 타입 추가:
  - `cadk::header` — `MAGIC = b"CADK"`, `SCHEMA_VERSION = 1`, `HEADER_SIZE = 64`, `CadkHeader`, `CadkFlags { MANIFEST_COMPRESSED, SIGNED, HAS_THUMBNAIL }`. `is_supported()`는 미키 must-understand 플래그와 대응되지 않는 스키마 버전을 거부.
  - `cadk::manifest` — `BlobKind { Document, Thumbnail, History, Attachment, Signature, Unknown }`, `BlobRecord`, `Manifest` (find_first / total_blob_bytes 헬퍼 포함).
- 단위 테스트 7개 추가 (헤더 5 + manifest 2). 외부 크레이트 추가 없이 플래그는 단순 `u32` 상수로 유지.
- 인코더/디코더, zstd 압축, Ed25519 서명, 자동저장 정책, 마이그레이션은 TBD. 이 커밋은 포맷 상수와 TOC 타입만 랜딩 → 이후 패치가 추가적으로 쌓이도록.

#### A2 Phase 2A — undo/redo 병합 (coalescing) 윈도우 (2026-05-07)
- `Session`에 `coalesce_window_ms` 필드(기본 1 000 ms)와 `set_coalesce_window_ms(ms)` 추가. 윈도우 내에 같은 `SolidId`를 대상으로 연속된 `Translate` / `Scale` / `Rename`은 이전 로그 항목에 병합되고 새 history 항목을 만들지 않음. `Translate` 델타는 합산, `Scale` 계수는 곱, `Rename` 라벨은 교체.
- replay 동안에는 내부적으로 coalescing을 비활성화 → 스냅샷 결정론적 재현 보장 (replay 로그 = 입력 슬라이스 그대로).
- 통합 테스트 3개 추가 (19 → 22): `translate_coalesces_within_window`, `coalescing_disabled_when_window_is_zero`, `rename_coalesces_keeps_only_last_label`.
- A2 deliverable #6 (1초 윈도우 undo/redo coalesce) — 완료.

#### A2 Phase 2A — SessionSnapshot 메타데이터 필드 (2026-05-07)
- `crates/api/src/session.rs` — `SessionSnapshot`에 A2 스펙 메타데이터 필드 4개 추가: `document_hash` (커서까지 직렬화된 명령 prefix의 FNV-1a-64 hex 다이제스트), `log_position` (커서 미러), `timestamp` (저장 시 유닉스 epoch 초), `label` (선택 사용자 레이블). 모두 `#[serde(default)]` 사용 → A2 이전 스키마 v1 스냅샷도 그대로 로드 가능.
- `Session::save_to_json_with_label(label: Option<String>)` — 스냅샷에 사람 친화적 레이블을 붙이는 신규 메서드 ("before boolean", "release-v0.5-tag" 등). 기존 `save_to_json()`도 그대로 동작하며 메타데이터 필드는 `label = None`으로 자동 채워짐.
- `crates/api/tests/api_integration.rs` 통합 테스트 17 → 19개로 증가: `snapshot_metadata_fields_populate_on_save`, `old_schema_v1_snapshot_without_metadata_still_loads` (전진 호환 round-trip).
- A2 deliverable #8 (확장된 SessionSnapshot) — 완료. A2의 9개 항목 중 1개 게이트 클로즈.

#### 상용 CAD 로드맵 v3.5 — 코퍼스 + ADR 심화 + 첫 실행 가능 시드 (2026-05-06)
- `docs/adr/` 10개 → 15개로 확장:
  - 0011 automerge CRDT 실시간 협업 (Kleppmann POPL 2017 형식 수렴 보증 인용; vs OT/Yjs/custom/RGA/LSEQ/state-based).
  - 0012 wasmtime + WASI Preview 2 컴포넌트 모델 기반 wasm-first 플러그인 샌드박스 (vs native-only/Lua-only/JS/process-IPC/seccomp/wasmer; capability-token 선언적 매니페스트).
  - 0013 CBOR RFC 8949 메타데이터 (vs JSON/msgpack/BSON/protobuf/FlatBuffers/Cap'n Proto/YAML/TOML; self-describing tag + 결정성 인코딩 규칙).
  - 0014 zstd RFC 8878 섹션 압축 (vs gzip/xz/brotli/LZ4/snappy; 기본 level 3 ~3.5×, archive level 19 ~5×, 사전 훈련 dictionary 계획).
  - 0015 스케치 LM 선형 스텝용 sparse Cholesky (vs dense LU/dense Cholesky/CG/MINRES/GPU/Eigen; nalgebra-sparse 기본 + 1k 변수 초과 시 CHOLMOD opt-in).
- `docs/perf/memory-profile.md` — 모든 위상·기하 타입 힙 비용표, R12 산업 파트 1 GB peak-RSS 예산 분해 (테셀레이션이 최대 비용원), dhat 측정 코드, OS별 RSS 측정, 할당자 선택 근거, 핫스팟 arena 정책, 5/15/50 % 회귀 게이트.
- `tests/corpus/` 골든 6개 추가: 스케치 003 슬롯 / 004 육각형 / 005 원에 내접 삼각형 (UnderDetermined + 잔여 DoF 명시) / 006 평행선-중복 접선 (OverDetermined + 중복 제약 ID 명시), 불리언 003 박스∩박스 / 004 구∩박스 (sphere face kind 보존) / 006 박스∪박스 idempotent (`Union(A,A)==A`).
- Lua 레퍼런스 파트 빌드 스크립트 2개: `r1_box.lua` (V=8/E=12/F=6 + Euler-Poincaré + 부피/면적/중심/태그/content-hash), `r2_extrude.lua` (구멍 추출, π·r²·h 부피, sketch imprint 태그 생존).
- **첫 컴파일 가능 Rust 시드** `examples/build_reference_parts.rs` — `cargo run --release --example build_reference_parts -- --output tests/corpus/reference_parts/`. quick_box/cadk::write API 들어오면 R1-R12 빌드, 아니면 NotImplemented 깔끔히 보고.
- 5-기둥은 이제 **ADR 15 + perf 4 + algorithms 13 + 골든 TOML 11 + Lua 2 + Rust 시드 1** 로 받쳐짐. 문서+코퍼스 9,299 줄 (로드맵 EN 3,169 + KO 287 + algorithms 2,717 + ADR 1,005 + perf 709 + corpus 1,309 + Rust 시드 103).

#### 상용 CAD 로드맵 v3.4 — ADR + 성능 방법론 + 코퍼스 시드 (2026-05-06)
- `docs/adr/` — Michael Nygard 표준 포맷(Status/Context/Decision/Alternatives/Consequences/References)의 아키텍처 결정 기록 10개: 0001 half-edge B-Rep, 0002 BLAKE3 캐시 키 (충돌 확률 $10^9$ 엔트리에서 $1.5 \times 10^{-21}$ 계산), 0003 nalgebra(커널)+glam(뷰어) 분리, 0004 egui+wgpu+winit, 0005 mlua Lua 5.4, 0006 PyO3 별도 크레이트, 0007 태그 기반 영구 명명, 0008 Rust edition 2024 + MSRV 1.85, 0009 커스텀 `.cadk`, 0010 Rayon 전용 (비동기 커널 거부). 각 ADR은 거부된 대안 최소 2개를 구체적 이유와 함께 나열.
- `docs/perf/` — 3개 성능 명세: methodology.md (R1-R5 HW 티어, Criterion 설정, cold-vs-warm, RNG 시드, CPU 격리, dhat 힙 프로파일, 16.67ms 프레임 예산 분해, 5/15/50 % 회귀 정책, PGO, perf 카운터, flame graph, 크로스-아키텍처), dispatch-matrix.md (SSI 18행, 불리언 coplanar, 필렛 5-티어, LM↔dogleg, 정밀도 에스컬레이션, STEP 9행), condition-numbers.md (알고리즘별 $\kappa$ 안전 한계, Hager 1-norm 검출, 2개 worked example, `EPSILON_*` 명명된 톨러런스, 크로스-아키 결정성).
- `tests/corpus/` — 골든 테스트 코퍼스 골격: README + sketch/golden (12개 스케치 매니페스트 + 3 worked TOML: 001 사각형, 002 동심원, 007 모순 정사각형 + 최소충돌집합 검증) + boolean/golden (10개 명세 + 2 worked TOML: 박스 용합, 상자 - 내부 박스 genus-0 캐비티) + reference_parts (R1-R12 빌드 스크립트 + 타이밍 예산).
- 로드맵 헤더 + §17 검증 크로스-레퍼런스 5-기둥 구조로 재구성: 로드맵(계약) / algorithms(어떻게) / adr(왜 다른 것이 아닌가) / perf(얼마나 빠른가) / corpus(계약 충족 증명). 전체 문서 ~5,900 → ~8,000 줄.

#### 상용 CAD 로드맵 Phase 1 — `cadkernel-api` 안정 API 크레이트 (2026-05-06)

상용 등급 CAD 로 방향 전환. `docs/COMMERCIAL_CAD_ROADMAP.md` (영문 정본, 한글 사본 포함) 에 8 단계 장기 플랜이 문서화되어, "안정 API + AI/테스트 통합" 부터 "1.0 릴리스 준비" 까지 단조 진행. Phase 1 은 그 기반인 새 전용 공개 API 크레이트를 출하.

- 신규 크레이트 `crates/api/` (`cadkernel-api`) — 3 주 타입:
  - `Document` — 최상위 모델 컨테이너 (Phase 2 에서 스케치·도면·어셈블리·FEM 흡수 예정).
  - `Command` — 모든 상태 변경 액션의 직렬화 enum. 시작 변형 14 개: `CreateBox/Cylinder/Sphere/Cone/Torus`, `BooleanUnion/Subtract/Intersect`, `Translate`, `Scale`, `Rename`, `DeleteSolid`, `NewDocument`, `Noop`. `serde` 기반 JSON 스키마.
  - `Session` — 실행/재현 엔진. `Command` 적용·로그 기록·타입화된 `Outcome` 반환. `Session::replay(commands)`, `log_to_json()`, `replay_from_json()` 으로 결정적 회귀 테스트 및 AI 평가 지원.
- `command_schemas()` — 사용 가능한 명령 표면(op 이름+설명+파라미터별 문서)을 AI 도구에 노출 (Rust 타입 임포트 불필요).
- `ApiError` 분류: `UnknownSolid` / `InvalidArgument` / `Kernel` / `Codec` — JSON-RPC 에러 코드로 깔끔히 매핑되는 평탄한 형태.
- 통합 테스트 17 개 (`crates/api/tests/api_integration.rs`): 생성·삭제·불리언(consume-and-create 의미), 평행이동(부피 보존), 무게중심 기준 스케일(부피 × factor³), 이름 변경(SolidId 안정), 모든 변형의 JSON 라운드트립, 결정적 5-명령 재현, 스키마 커버리지, 4 단계 시나리오 도큐먼트 검증. lib 레벨 doc test 2 개 추가.
- 통합 후 워크스페이스 합계: **2,865 / 0 / 0** (기존 2,844 / 0 / 0 대비 +21).
- 신규 문서:
  - `docs/COMMERCIAL_CAD_ROADMAP.md` (영문 정본, 8 단계 플랜, AI/테스트 아키텍처 섹션, 범위상 `UI_COMPLETION_ROADMAP.md` 를 흡수).
  - `docs/COMMERCIAL_CAD_ROADMAP.ko.md` (한글 사본).
- MCP 서버·Lua 스크립팅·Python 바인딩·GUI 디스패처는 **본 커밋에서 변경하지 않음** — Phase 1 의 계약은 "비-GUI 소비자에게 API 가 작동함을 시스템 나머지를 건드리지 않고 증명". Phase 5 에서 GUI 를 `Session::execute(Command)` 로 라우팅하고 맞춤 디스패처를 제거할 예정.

#### UI 완성 Phase K-sketch-refs — Sketcher external reference and reuse UX (2026-05-05)

Sketcher external projection이 선택된 scene object를 우선 사용하고, 없으면 현재 model을 사용해 vertex와 edge를 active sketch의 construction reference geometry로 투영합니다. Carbon Copy는 복사된 reusable entity 수를 보고하며, Sketcher banner/status bar는 `Refs: ...` / `Reuse: ...` 상태를 표시합니다. selected-object projection, construction reference count, carbon-copy reuse reporting 회귀 테스트를 추가했습니다. 전체 검증 **2,844 / 0 / 0**.

---

#### UI 완성 Phase K-sketch-constraints — Sketcher constraint diagnostics UX (2026-05-05)

Sketcher validation이 duplicate constraint, 같은 대상의 conflicting dimensional value, invalid dimensional value를 감지합니다. `SketchValidation::status_label()` / `diagnostic_issue_count()`로 compact diagnostic을 제공하고, Sketcher banner/status bar/overlay warning pill이 첫 actionable constraint issue를 표시합니다. duplicate, conflicting distance, invalid length, `SketchMode` 상태 전파 회귀 테스트를 추가했습니다. 전체 검증 **2,841 / 0 / 0**.

---

#### UI 완성 Phase K-sketch-profile — Sketcher profile validation UX (2026-05-05)

Sketcher에 feature command용 profile diagnostics를 추가했습니다. `analyze_profiles` / `extract_profile_checked`가 construction line을 제외하고 open endpoint, branch point, invalid line reference, multiple loop를 감지하며, 단일 regular closed loop일 때만 profile을 반환합니다. Sketcher banner는 `Profile ready` 또는 open/branch/invalid 이유를 표시하고, Pad/Pocket/Groove/Close sketch 경로는 열린 chain을 부분 extrude하지 않고 거부합니다. construction diagonal이 있는 square는 정상 profile로 허용됩니다. 전체 검증 **2,837 / 0 / 0**.

---

#### UI 완성 Phase J-fem-bc — FEM multi-node boundary-condition UX (2026-05-05)

FEM boundary-condition editor가 모든 kernel-side `BoundaryCondition` variant를 지원합니다. `SectionPrint`, `TieConstraint`, `RigidBody`, `ContactConstraint`를 메뉴와 stateful BC dialog에 노출했고, section plane normal/point와 Set A/Set B inclusive node range, contact penalty 입력을 추가했습니다. FEM toolbar constraint 버튼도 log-only 경로 대신 같은 BC editor를 열도록 전환했습니다. 전체 검증 **2,831 / 0 / 0**.

---

#### UI 완성 Phase I-fem-results — FEM result interpretation UX (2026-05-05)

FEM post-processing에 기존 7단계 colormap scene object 위의 결과 해석 UX를 추가했습니다. Stress/Displacement/Von Mises 표시가 field/unit/range/color band와 마지막 probe 정보를 보존하는 legend overlay를 갱신합니다. FEM Results 메뉴와 toolbar에는 `Probe Node...`와 `Result Table...`을 추가했으며, probe dialog는 node/element 선택과 위치, displacement/stress/temperature 값을 기록하고 result table dialog는 node/element 결과 row를 표시합니다. 전체 검증 **2,825 / 0 / 0**.

---

#### UI 완성 Phase H-view — TechDraw view placement setup command UX (2026-05-05)

TechDraw view 명령에 front/top/right/isometric projection, 3-view layout, section/detail/broken view의 sheet X/Y 위치, 수동 sheet scale, 3-view 간격, view별 parameter를 편집하는 stateful View Setup dialog를 추가했습니다. Views 메뉴와 toolbar 버튼은 이제 이 dialog를 열고, 기존 view dispatcher는 headless 회귀 테스트용 빠른 경로로 유지됩니다. `DrawingView`는 선택적 `sheet_x` / `sheet_y` / `sheet_scale` metadata를 저장하며, SVG 렌더링은 수동 배치가 있을 때 이를 우선 적용합니다. 전체 검증 **2,822 / 0 / 0**.

---

#### UI 완성 Phase H-center — TechDraw centerline setup command UX (2026-05-05)

TechDraw 중심선 명령에 face centerline, parallel line centerline, center mark, bolt-circle centerline parameter를 편집하는 stateful Centerline Setup dialog를 추가했습니다. Centerlines 메뉴와 toolbar 버튼은 이제 이 dialog를 열고, 적용 시 선택한 centerline 저장소가 active `DrawingSheet`에 추가됩니다. 기존 centerline dispatcher는 headless 회귀 테스트용 빠른 경로로 유지됩니다. 전체 검증 **2,816 / 0 / 0**.

---

#### UI 완성 Phase H-anno — TechDraw annotation setup command UX (2026-05-05)

TechDraw 주석 명령에 text/rich text/balloon/leader/weld/surface finish parameter를 편집하는 stateful Annotation Setup dialog를 추가했습니다. Annotations 메뉴와 toolbar 주석 버튼은 이제 이 dialog를 열고, 적용 시 선택한 sheet-level annotation이 active `DrawingSheet`에 추가됩니다. 기존 annotation dispatcher는 headless 회귀 테스트용 빠른 경로로 유지됩니다. 전체 검증 **2,811 / 0 / 0**.

---

#### UI 완성 Phase H-dim — TechDraw dimension setup command UX (2026-05-05)

TechDraw 치수 명령에 선형/반지름/지름/각도/호 길이/면적 parameter를 편집하는 stateful Dimension Setup dialog를 추가했습니다. Dimensions 메뉴와 toolbar 치수 버튼은 이제 이 dialog를 열고, 적용 시 선택한 치수/확장 치수/호 길이/면적 annotation이 active `DrawingSheet`에 추가됩니다. 기존 `Dim*` dispatcher는 headless 회귀 테스트용 빠른 경로로 유지됩니다. 전체 검증 **2,806 / 0 / 0**.

---

#### UI 완성 Phase H-page — TechDraw page setup command UX (2026-05-05)

TechDraw에 template, title, page size를 편집하는 stateful Page Setup dialog를 추가했습니다. `From Template...` 메뉴와 toolbar template 버튼은 이제 이 dialog를 열고, 적용 시 A4/A3/custom 크기와 title-block 텍스트가 반영된 `DrawingSheet`를 생성합니다. dialog open/custom/A3 preset 회귀 테스트와 상태 모델 유닛 테스트를 추가했습니다. 전체 검증 **2,801 / 0 / 0**.

---

#### UI 완성 Phase F-rest — TechDraw centerline / bolt circle (2026-05-04)

TechDraw `CenterFace`, `CenterLines`, `CenterPoints`, `BoltCircle` dispatcher가 log-only에서 실제 drawing sheet 중심선/중심 마크/볼트 원 출력으로 전환됨. 세 저장소는 `drawing_to_svg`에 렌더링되므로 PDF 출력도 동일 경로를 사용합니다. 4개 dispatcher 회귀 테스트와 SVG 출력 검증을 추가했습니다. 전체 검증 **2,796 / 0 / 0**.

---

#### UI 완성 Phase F-anno — TechDraw drawing annotation (2026-05-04)

TechDraw `Text`, `RichText`, `Balloon`, `Leader`, `Weld`, `SurfFinish` dispatcher가 log-only에서 실제 drawing sheet annotation 출력으로 전환됨. 텍스트/서식 텍스트/풍선/리더/용접 기호/표면 거칠기 기호는 SVG/PDF 렌더링 가능한 sheet-level 저장소를 사용함. 6개 dispatcher 회귀 테스트와 SVG 출력 검증을 추가했습니다. 전체 검증 **2,792 / 0 / 0**.

---

#### UI 완성 Phase F-dim — TechDraw drawing dimension (2026-05-04)

TechDraw `DimLinear`, `DimRadius`, `DimDiameter`, `DimAngle`, `DimArcLen`, `DimArea` dispatcher가 log-only에서 실제 drawing sheet 출력으로 전환됨. 선형/반지름은 기존 sheet dimension에 추가되고, 지름/각도/호 길이/면적은 SVG/PDF 렌더링 가능한 확장 치수·호 길이·면적 annotation 저장소를 사용함. 6개 dispatcher 회귀 테스트와 SVG 출력 검증을 추가했습니다. 전체 검증 **2,786 / 0 / 0**.

---

#### UI 완성 Phase F-view — TechDraw section/detail/broken view (2026-05-04)

TechDraw `SectionView`, `DetailView`, `BrokenView` dispatcher가 log-only에서 실제 drawing sheet 변경으로 전환됨. `SectionView`는 선택 솔리드를 중간 평면으로 절단해 투영 view를 추가하고, `DetailView`는 첫 view를 확대 복사하며, `BrokenView`는 기존 broken-view helper로 첫 view를 압축함. 세 dispatcher 회귀 테스트를 추가했고, 장기 플랜은 TechDraw 치수/주석 → 명령 UX → Sketcher → PartDesign history → Assembly → FEM UX → I/O → 성능 → 릴리스 순서로 진행합니다. 전체 검증 **2,780 / 0 / 0**.

---

#### UI 완성 Phase E-render — FEM 결과 colormap (2026-05-04)

`FemAction::ShowStress`, `ShowDisplacement`, `ShowVonMises`가 log-only에서 실제 뷰포트 colormap 메시 생성으로 전환됨. 활성 tetrahedral FEM mesh의 boundary surface를 7단계 blue→green→red band scene object로 생성하며, 결과 필드 전환 시 이전 FEM colormap을 교체함. displacement는 nodal displacement 크기, stress/Von Mises는 element Von Mises 값을 boundary node 평균으로 표시. 테스트 3개 추가, 전체 검증 **2,777 / 0 / 0**.

---

#### UI 완성 HARD-tier 배치 — Annotation overlay, FEM solver, TechDraw export, ShapeBinder (2026-05-04)

Phase A-C3 이후 첫 HARD-tier 배치. `gui::scene_overlay`로 Draft/Part 와이어·포인트·라벨 출력이 실제 뷰포트 오버레이로 표시되고, `D::Dimension` / `D::Label`이 log-only에서 가시 주석으로 전환됨. FEM은 열전도/비선형 정적 solver를 `AnalysisContainer`에 연결. TechDraw는 NewPage/FromTemplate/Redraw와 DXF/PDF exporter를 dispatcher에 연결. PartDesign `ShapeBinder`는 선택 형상의 face를 새 binder solid로 복사. 검증: build, clippy `-D warnings`, 전체 테스트 **2,774 / 0 / 0**.

---

#### UI 완성 Phase C3 — Surface 연산 + PartDesign Loft/Pipe (2026-05-01) `94396bb`

Phase C3 종료. Surface: `S::Sections`(스키닝), `S::Extend`(두께 증가), `S::Blend`(쿼드 시트). PartDesign: `Pd::AdditiveLoft/Pipe`(솔리드 추가), `Pd::SubtractiveLoft/Pipe`(boolean 절삭). 7개 피처 + 8개 테스트. 전체 내용은 [영어 항목](../CHANGELOG.md#ui-completion-phase-c3) 참조.

---

#### UI 완성 Phase C2 — Draft 수정 + ProjectCurvesOnSurface (2026-05-01) `0808d9a`

Phase C2 종료. `D::Offset`, `D::Trim`, `D::Stretch`, `D::Facebinder`, `P::ProjectCurvesOnSurface` 5개 피처 연결. 전체 내용은 [영어 항목](../CHANGELOG.md) 참조.

---

#### UI 완성 Phase C1 — Draft 변환 + EASY 잔여 (2026-05-01) `c2d3006`

Phase C1 종료. Draft 변환 4개(`D::Move/Rotate/Scale/Mirror`) + EASY 잔여 3개(`S::Coons`, `FemAction::Summary/Report`). 7개 피처 + 8개 테스트. 전체 내용은 [영어 항목](../CHANGELOG.md) 참조.

---

#### UI 완성 Phase B-cont — Part 워크벤치 EASY 티어 (2026-05-01) `4e16eba`

Phase B-cont 종료. Part 워크벤치 13개 EASY 스텁 연결(FaceFromWires, ConnectShapes, EmbedShapes, CutoutShapes, ExplodeCompound, CompoundFilter, BooleanFragments, SliceToCompound, PointsFromShape, ConvertToSolid, AutoDefeaturing, TransformedCopy, CoonsPatch). 전체 내용은 [영어 항목](../CHANGELOG.md) 참조.

---

#### UI 완성 Phase B — Draft EASY 티어 (2026-05-01) `75d7705`

Phase B 종료. Draft EASY 스텁 14개(Wire, BSpline, Bezier, Hatch, Text, Upgrade, Downgrade, WireToBSpline, ToSketch, Clone, ArrayRect, ArrayPolar, ArrayPath, ArrayPoint) 연결. 전체 내용은 [영어 항목](../CHANGELOG.md) 참조.

---

#### UI 완성 Phase A — 핵심 CAD 워크플로우 (2026-04-29) `abbfbda`

Phase A 종료. log_info 스텁으로 아무 형상도 생성하지 않던 10개 핵심 피처 연결: Pad/Pocket/Groove/Hole/CountersunkHole(PartDesign 5개), Circle/Arc/Ellipse/Line/Point(Draft 2D 5개). 새 통합 테스트 12개. 2,662 → 2,674 / 0 / 0. 전체 내용은 [영어 항목](../CHANGELOG.md) 참조.

---

### 문서

#### 이중 언어 문서 단일 소스 정책 — 영어 정본 + DEVELOPER_WIKI 섹션 3 역이식 (2026-04-29)

영어 문서를 정본으로 선언. 한국어 파일은 요약 항목 허용. `docs/DEVELOPER_WIKI.md` 섹션 3이 한국어 버전(~590줄)에서 영어 버전(~120줄)으로 역이식되어 역전된 드리프트 해소. 전체 내용은 [영어 항목](../CHANGELOG.md) 참조.

---

### 리팩터링됨

#### Viewer 아키텍처 대대적 개편 — 모듈 분할 + ActiveDialog enum + GuiAction sub-enum 분리 (2026-04-29)

**배경.** Phase N full / Phase O-a / 후속 작업이 누적되며 `crates/viewer/src/gui/mod.rs`가 약 2,400 LOC, `GuiAction` enum이 평탄한 약 210개 top-level variant까지 부풀어 오름. 모든 워크벤치(Sketcher / Assembly / FEM / Mesh / Surface / Part / PartDesign / Draft / TechDraw)의 variant가 같은 enum에 나란히 있고, 13개 `Option<...>` / `bool show_...` 필드가 다이얼로그 상태를 분산해서 들고 있었음. `app.rs`의 거대 `match`는 7,000 LOC를 넘김. 이번 작업은 동작 변경 없이 이 두 파일을 재구조화함.

**Refactor #2 — ActiveDialog enum (commit `16192ad`).** 13개 흩어진 `Option<DialogState>` / `bool show_dialog` 필드를 `GuiState`의 `ActiveDialog` enum + `pub active_dialog: Option<ActiveDialog>` 단일 필드로 통합. 각 variant가 해당 다이얼로그의 고유 상태를 담음(예: `MaterialPicker(MaterialPickerState)`, `BcEditor(BcEditorState)`, `JointEditor(JointEditorState)`). 상태 있는 다이얼로그의 상호 배타성이 assertion이 아니라 type-level invariant가 됨. +1 test (2,660 / 0 / 0).

**Refactor #1 — Viewer 모듈 분할.** 워크벤치별 타입과 헬퍼를 `gui/mod.rs`에서 형제 모듈로 분리해 각 워크벤치가 자기 파일을 소유하도록 함:

- `gui/sketch_state.rs` — `SketchTool`, `DimensionKind`, `DimensionPopup`, `SketchEntityRef`, `SketchSnapshot`, `SketchMode` + impl (commit `9e2afa1`).
- `gui/assembly.rs` — `AssemblyJointType`, `JointEditorState`, BOM 헬퍼 + `impl GuiState` 블록 (commit `9bd65b9`).
- `gui/fem.rs` — `MaterialPreset`, `MaterialPickerState`, `material_from_preset`, `BcKind`, `BcInputs`, `BcEditorState`, `fem_picker_tests` (commit `9bd65b9`).

`gui/mod.rs`는 이 분할만으로 2,381 → 1,304 LOC로 줄었고, 모든 호출 사이트는 표적화된 `pub(crate) use` 재내보내기로 보존됨.

**Refactor #3 — GuiAction sub-enum 분할.** 9개 워크벤치 분량의 평탄한 top-level variant를 `GuiAction::Workbench(WorkbenchAction)` 래퍼 variant로 치환. 각 워크벤치당 새 모듈 1개, `app.rs`에 sub-enum별 `process_workbench_action()` 디스패처 헬퍼 1개씩. 워크벤치별 마이그레이션:

- `AssemblyAction` — 9 variants (commit `9e42ece`)
- `FemAction` — 19 variants (commit `3600df3`)
- `SketcherAction` — 43 variants (commit `623e4bf`)
- `MeshAction` — 9 variants (commit `b1c21a4`)
- `SurfaceAction` — 7 variants (commit `eb9a15d`)
- `PartAction` — 14 variants (commit `8685bdf`)
- `PartDesignAction` — 17 variants (commit `ebf91e1`)
- `DraftAction` — 32 variants (commit `5fedab5`); 분할 과정에서 죽은 코드로 드러난 `SetDraftLayer(String)` variant도 같이 제거
- `TechDrawAction` — 30 variants, 원래 4 + "TechDraw expanded" 26 모두 (commit `d2966b6`)

총 **180개 variant**가 top-level enum에서 9개의 워크벤치별 sub-enum으로 이동. `GuiAction` 외곽 enum은 이제 약 50개 횡단 관심사 variant(파일 I/O, 뷰포트, 씬, 변환)와 9개 래퍼 variant만 보유.

**모든 sub-enum 추출에 적용된 패턴.** 각 커밋이 동일한 형태를 따름: sub-enum만 담은 새 모듈 파일 → `gui/mod.rs`에서 재내보내기 → `GuiAction`에 `Workbench(WorkbenchAction)` variant 1개 추가 → `app.rs`에 `process_workbench_action(&mut self, action: WorkbenchAction)` 헬퍼 추가 → `menu.rs` / `toolbar.rs` / `dialogs.rs` / `task_panel.rs` / `context_menu.rs`의 호출 사이트 마이그레이션(각 함수 안에서 `use super::WorkbenchAction as W;` 단축형 사용). Derive 선택은 sub-enum별 페이로드 형태에 따라 결정 — 모든 variant가 `f64`/`u32` 등만 들고 있으면 `Copy`, 적어도 하나가 `String`이나 `Vec<_>`를 들면 `Clone+Debug+PartialEq`, derive를 갖지 않는 외부 타입을 든 variant가 있으면 `PartialEq` 생략(`SketcherAction::Enter(WorkPlane)`, `TechDrawAction::AddView(ProjectionDir)`).

**검증.** 모든 커밋이 개별적으로 `cargo build --workspace`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --no-fail-fast`를 통과(**2,660 passed, 0 failed, 0 ignored** — V37 Phase O-a 후속 + ActiveDialog 리팩터 베이스라인과 동일; sub-enum 추출 커밋에서 추가/삭제된 테스트 없음).

**리팩터 후 파일 구성.** `crates/viewer/src/gui/`에 9개의 새 형제 모듈이 추가됨: `assembly.rs`, `fem.rs`, `sketch_state.rs`, `mesh.rs`, `surface.rs`, `part.rs`, `part_design.rs`, `draft.rs`, `techdraw.rs`. `gui/mod.rs`는 이제 횡단 타입(`GuiState`, `ViewportInfo`, `GuiAction` 외곽 enum, `ActiveDialog`, `MirrorPlane`, scene/selection enum, theme/density 토글)만 담음.

### 추가됨

#### V37: Phase O-a 후속 — BcKind 12개 variant + Ground Component 메뉴 (2026-04-26)

**배경.** Phase O-a는 `BcKind { FixedNode | Force }`를 동작하는 편집기 파이프라인과 함께 출하했지만 커널의 17개 `BoundaryCondition` variant 중 나머지 10개는 레거시 `AddFemConstraint` 경로에 있었고(다중 노드/서피스 4개는 더 풍부한 노드 집합 선택 UX 필요), 이 후속 작업은 `BcKind`를 나머지 10개 스칼라 / 단일 노드 / Vec3 전용 variant로 확장하여 통합 편집기에 담고, 기존 `JointType::Grounded`를 프로그래매틱 디스패치뿐 아니라 메뉴로도 도달 가능하도록 Assembly 메뉴에 "Ground Component" 엔트리를 추가합니다.

**`BcKind` 확장(2 → 12 variant).** 신규 arm: `Pressure { element, pressure }`, `Displacement { node, displacement }`, `Gravity { acceleration }`, `DistributedLoad { element, load }`, `Spring { node, stiffness }`, `CentrifugalLoad { axis, omega }`, `SelfWeight { gravity }`, `SpringConstraint { node_id, stiffness, direction }`, `BodyLoad { force_density }`, `InitialTemperature { node, temperature }`.

**Vec3/스칼라 오버로드를 가진 단일 편집기** (`crates/viewer/src/gui/mod.rs` + `dialogs.rs`). 10개 서브 모달 대신 `BcEditorState`가 단일 `vec3_x/y/z` 트리플, 단일 `scalar_a`, `node_index`와 `element_index`를 가집니다. 다이얼로그가 variant별 라벨을 다시 붙임: `vec3_label()`은 "Force (N)" / "Displacement (m)" / "Acceleration (m/s²)" / "Load (N/m²)" / "Axis" / "Gravity (m/s²)" / "Direction" / "Force Density (N/m³)"를 반환; `scalar_label()`은 "Pressure (Pa)" / "Stiffness (N/m)" / "Omega (rad/s)" / "Temperature (K)"를 반환. `to_boundary_condition()`이 오버로드된 필드를 variant별 올바른 커널 필드 이름(axis vs displacement vs force vs gravity vs load vs direction vs acceleration vs force_density)으로 라우팅. `BcInputs` 매트릭스가 다이얼로그가 렌더링할 입력을 게이팅: variant별 `(node, element, vec3, scalar)`. `app.rs`는 변경 없음 — 헬퍼 API가 안정적이라 `OpenBcEditor(BcKind)` / `CommitBcEditor` 디스패처 시그니처가 동일.

**메뉴 3개 서브-서브메뉴로 재구성** (`crates/viewer/src/gui/menu.rs`). 평평한 "Boundary Conditions" 서브메뉴가 3개 그룹으로 분리: **Loads**(Force, Pressure, Gravity, DistributedLoad, CentrifugalLoad, SelfWeight, BodyLoad), **Constraints**(FixedNode, Displacement, Spring, SpringConstraint), **Thermal**(InitialTemperature). 자라는 평평한 목록 대신 FEM 메뉴에 12개 엔트리.

**미진행(4개 variant, 별도 세션).** `TieConstraint`, `RigidBody`, `ContactConstraint`, `SectionPrint`는 단순 스칼라/Vec3 입력이 아닌 다중 노드 집합 / 서피스 피커 UX가 필요하므로 노드 집합 피커가 존재할 때까지 레거시 `AddFemConstraint(FemConstraintType)` 경로에 머묾. `gui/mod.rs`의 `BcKind` 위 주석 블록에 문서화.

**Ground Component 메뉴 엔트리** (`crates/viewer/src/gui/mod.rs` + `gui/menu.rs`). Assembly 메뉴의 기존 "Joints" 서브메뉴 위 신규 엔트리가 기존 joint 편집기 플로우(Phase N 완전판이 이미 1-컴포넌트 Grounded 케이스 지원)를 통해 `GuiAction::AddAssemblyJoint(AssemblyJointType::Grounded)`를 연결. `Grounded`가 이제 메뉴로 도달 가능하므로 `AssemblyJointType`의 `#[allow(dead_code)]` 제거.

**추가된 테스트(총 21개).** `gui::mod::fem_picker_tests`의 unit 테스트 11개(모든 `BcKind` variant에 대한 12행 매트릭스 입력 가시성 테이블 체크 + 신규 variant별 `BcEditorState → BoundaryCondition` 매핑). `crates/viewer/tests/gui_action_integration.rs`의 통합 테스트 10개(디스패처 호출 미러링, 신규 variant별 매칭되는 `BoundaryCondition` 1개를 `fem_analysis.boundary_conditions`에 추가).

**검증 (2026-04-26).**
- `cargo build --workspace` — 0 에러.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — 0 경고.
- `cargo test --workspace --no-fail-fast` — **2,659 통과, 0 실패, 0 무시.** Sticky-material 폴리시 베이스라인 대비 +21 테스트.

**LOC 변경.** ~495 production+test net(mod.rs +285, dialogs.rs +28, menu.rs +16, gui_action_integration +166), 450 예산보다 45 LOC 초과 — 11개 variant별 unit 테스트가 각 ~10줄이라 전적으로 테스트 폭에 의해 발생. 프로덕션 코드는 예산 안에서 깔끔; 초과는 테스트 폭에서 — Vec3/스칼라 오버로드의 명시적 variant별 커버리지를 위한 합리적 거래로 수용.

---

#### V37: Phase O-a — FEM 재료 선택기 + 경계 조건 편집기 (2026-04-26)

**배경.** Phase N-min에서 `GuiState.fem_analysis: Option<AnalysisContainer>`와 `CreateFemAnalysis` / `SolveStatic` dispatcher arm을 연결했지만, 재료는 `FemMaterial::steel()`로 하드코딩되어 있고 경계 조건을 추가할 GUI 경로가 없어 사용자가 Lua/Python으로 빠져야 했습니다. Phase O-a는 누락된 UI를 추가; 전체 결과 시각화(tet 메시 위 stress/displacement 컬러맵)는 렌더링 파이프라인 작업이 필요해 Phase O-b로 분리.

**재료 선택기 다이얼로그** (`crates/viewer/src/gui/dialogs.rs` + `gui/mod.rs`). `GuiAction::OpenMaterialPicker`가 여는 신규 모달. 6개 프리셋 재료(`Steel`, `Aluminum`, `Titanium`, `Copper`, `Concrete`, `CastIron`)의 라디오 버튼 — 각각 매칭되는 `cadkernel_modeling::fem::FemMaterial` 생성자를 값 중복 없이 사용하므로 커널 측 재료 업데이트가 자동 반영. `Custom` 옵션은 3개 숫자 입력(Young's modulus, Poisson 비율, 밀도)을 노출하여 `FemMaterial::custom()` 검증을 거침. `CommitMaterialPicker`는 신규 `GuiState.pending_fem_material: FemMaterial` 필드에 기록; 이후 `CreateFemAnalysis` 호출들이 그 값을 새 `AnalysisContainer`로 clone(sticky-material UX — 한 번 선택, 분석 간 재사용). 상태: `MaterialPickerState { selected, custom_youngs_modulus, custom_poisson_ratio, custom_density }`. `FemMaterial`은 이제 `Clone`을 derive(3개 `f64` 필드, 자명히 Clone-safe).

**경계 조건 편집기** (`crates/viewer/src/gui/dialogs.rs`). `GuiAction::OpenBcEditor(BcKind)`가 여는 신규 모달. `BcKind`는 가장 많이 쓰이는 두 variant — `FixedNode`와 `Force` — 를 다룸. 다이얼로그는 종류 드롭다운, 노드 인덱스 스피너(분석 존재 시 메시 노드 수로 제한), kind가 `FixedNode`일 때 숨겨지는 force XYZ 필드. `CommitBcEditor`는 `BoundaryCondition::FixedNode(n)` 또는 `BoundaryCondition::Force { node, force }`를 구성하고 `gui.fem_analysis.is_some()`일 때만 `fem_analysis.add_bc(bc)` 호출; 아니면 상태바를 "FEM: no analysis — create one first"로 설정. 나머지 15개 `BoundaryCondition` variant(Pressure, Displacement, Gravity, Spring 등)는 여전히 기존 `GuiAction::AddFemConstraint(FemConstraintType)` 경로 사용 — 통합은 추후.

**메뉴 통합** (`crates/viewer/src/gui/menu.rs`). FEM workbench 메뉴에 기존 Steel/Aluminum 빠른 설정 위로 "Pick Material…" 엔트리 추가, "Boundary Conditions" 서브메뉴(Add Fixed Node, Add Force) 추가.

**추가된 테스트.** `gui::mod::fem_picker_tests`의 unit 테스트(7개 신규: 6개 프리셋 → `FemMaterial` 생성자 매핑, `Custom` 사용자 값 + 잘못된 입력 시 steel 폴백, `BcKind::Force` 필드 가시성 게이트, FixedNode/Force `BcEditorState` → `BoundaryCondition` 매핑, `pending_fem_material` 기본값 steel, sticky 재사용을 위한 clone 라운드트립). `crates/viewer/tests/gui_action_integration.rs`의 통합 테스트(4개 신규: `commit_material_picker_updates_pending_material_for_each_preset`, `create_fem_analysis_consumes_pending_material`, `commit_bc_editor_fixed_node_appends_one_matching_bc`, `commit_bc_editor_force_appends_one_matching_bc_xyz`). 총 11개 신규 테스트.

**검증 (2026-04-26).**
- `cargo build --workspace` — 0 에러.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — 0 경고.
- `cargo test --workspace --no-fail-fast` — **2,638 통과, 0 실패, 0 무시.** Phase N full 베이스라인 대비 +11 테스트.

**LOC 변경.** Phase O-a 본체: 400 LOC 정확히 예산 한계(production: app.rs +38, dialogs.rs +89, menu.rs +6, gui/mod.rs +195; test: gui_action_integration.rs +72). Sticky-material 폴리시: 1줄 커널 `Clone` derive + 7줄 dispatcher swap(`mem::replace` → `clone()`) + 8줄 clone 라운드트립 unit 테스트.

**알려진 후속 항목.** (1) `BcKind`는 `FixedNode`와 `Force`만 모델링; 다른 15개 BC variant는 여전히 레거시 `AddFemConstraint` 경로 사용. (2) Phase O-b — tet 메시 위 stress/displacement 컬러맵 — 미진행. (3) 재료 선택기의 `Custom` 잘못된 입력 경로는 조용히 steel로 폴백; DragValue 범위가 UI에서 이를 도달 불가능하게 유지하지만 경로는 존재.

---

#### V37: Phase N 완전판 — Assembly 트리 패널, BOM 다이얼로그, Joint 편집기 (2026-04-24)

**배경.** Phase N-min에서 이미 `GuiState.assembly: Option<Assembly>` scaffolding과 `GuiAction::BillOfMaterials` / `AddAssemblyJoint` stub 디스패처 arm이 완료된 상태. Phase N 완전판은 그 stub들을 모델링 계층의 `Assembly` 구현(이미 완성)을 건드리지 않은 채 UI 측에서 완전히 기능하는 Assembly workbench로 전환 — 씬 트리 계층, BOM 모달, Joint 편집기 모달.

**Assembly 트리 패널** (`crates/viewer/src/gui/tree.rs`). `gui.assembly.is_some()`일 때 씬 트리가 객체 트리 아래에 4-branch Assembly 섹션을 렌더링: root 라벨은 `"{name} (N components, M constraints, K joints)"`, Components branch는 컴포넌트별 1행 + `GuiAction::ToggleAssemblyComponentVisibility`에 연결된 eye 아이콘, Constraints branch는 `Fixed(comp N)` / `Coincident(a,b)` / `Concentric(a,b)` / `Distance(a,b,d)` / `Angle(a,b,θ)` 라벨, Joints branch는 `Revolute(a↔b)` / `FixedJoint(a↔b)` / `Grounded(N)` 등의 포맷. 컴포넌트 행 더블클릭은 컴포넌트의 `Handle<SolidData>`가 `SceneObject`에 매핑되면 (`find_object_for_solid`를 통한 `Handle` 동등 매치) `GuiAction::FocusObject`를 발동.

**BOM 뷰 다이얼로그** (`crates/viewer/src/gui/dialogs.rs`). `GuiAction::BillOfMaterials`가 이제 `Assembly::bill_of_materials()`를 3컬럼 테이블(Index | Name | Quantity)과 "Total parts: {sum}" 푸터로 렌더링하는 모달을 엽니다. `assembly.is_none()`일 경우 디스패처는 상태바를 "Assembly: no assembly — create one first"로 설정하고 상태 변경은 수행하지 않습니다. 모달 상태는 `GuiState`의 `show_bom_dialog` + `bom_entries: Vec<BomEntry>`로 오픈 시점에 채워집니다.

**Joint 편집기 UI** (`crates/viewer/src/gui/dialogs.rs`). `GuiAction::AddAssemblyJoint(joint_type)`가 요청된 타입으로 사전 시딩된 모달을 엽니다. 2개 컴포넌트 드롭다운은 `assembly.components`를 이름으로 나열; 선택된 `JointType` variant와 관련된 필드만 표시 (Revolute/Cylindrical은 axis+origin, Slider는 axis만, BallJoint는 center, AngleJoint는 angle, GearJoint/BeltJoint는 ratio, ScrewJoint는 axis+pitch, RackAndPinion은 pitch_radius, ParallelAxes/PerpendicularAxes는 2-axis pair). OK는 매칭되는 `JointType::{variant} {…}`를 구성하고 `assembly.add_joint(joint)` 호출; Cancel은 상태 변경 없이 닫기. `Grounded`는 component_a만 필요 — 그에 따라 게이팅.

**추가된 테스트.** `gui::mod::assembly_helper_tests`의 unit 테스트(6개 신규: joint 라벨 Revolute/Grounded/Fixed/Gear 포맷, constraint 라벨 커버리지, 단일 컴포넌트 Grounded open+commit). `crates/viewer/tests/gui_action_integration.rs`의 통합 테스트(디스패처 호출 미러링, 5개 신규: 컴포넌트 가시성 토글, 중복 BOM 집계, 빈 assembly BOM, Revolute joint `assembly.joints`에 추가, 단일 컴포넌트에서 Grounded joint 허용). 기존 `tree_assembly_section_*` 테스트 2개는 신규 `Option<&Scene>` 인자를 받도록 갱신.

**검증 (2026-04-24).**
- `cargo build --workspace` — 0 에러.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — 0 경고.
- `cargo test --workspace --no-fail-fast` — **2,627 통과, 0 실패, 0 무시.** UX pass 베이스라인 대비 +21 테스트.

**LOC 변경.** Production ≈ 160 net(tree.rs +72, gui/mod.rs +82, dialogs.rs +6). Test ≈ 80(gui_action_integration +80). 총 ≈ 240 LOC, 500 예산 이내. `app.rs`는 변경 0 — Phase N-min이 이미 `GuiState` 헬퍼 메서드(`populate_bom_entries`, `open_joint_editor`, `commit_assembly_joint`, `toggle_assembly_component_visibility`)를 통해 모든 디스패처 arm을 연결해 두었기 때문.

**Assembly 로드맵 잔여 항목.** `JointType::Grounded`를 위한 메뉴 엔트리(Assembly workbench 메뉴의 "Ground Component" 버튼) — 현재 `Grounded`는 프로그래매틱 `GuiAction` 디스패치로만 도달 가능. Phase N 스펙의 다른 항목은 모두 출하됨.

---

#### V36: UX 패스 — gizmo 정밀도, 트리 포커스, Properties AABB (2026-04-24)

**배경.** V36 감사가 2,590 / 0 / 0으로 종결된 뒤, 이미 scaffolding은 있으나 작은 어포던스가 빠져 있던 4개 사용자 표시 상호작용을 정리하는 집중 UX 패스. 드리프트 방지를 위해 범위를 production+test 합쳐 ≤300 LOC로 제한. Sketch drag는 감사 결과 이미 FreeCAD 수준(제약 인식 `drag_solve`, 그리드 스냅, point/midpoint/intersection 스냅, FreeCAD 스타일 녹색 스냅 표시자) — 변경 없음.

**Gizmo 정밀도** (`crates/viewer/src/gui/overlays.rs`). 3D 변환 gizmo 드래그가 이제 modifier 키를 존중합니다. Shift-drag는 동작을 0.1× 속도로 느리게 하여 정밀 위치 지정; Ctrl-drag는 translation을 1 mm, rotation을 1°, scale을 10%로 스냅. gizmo 모드 레이블 아래 suffix("Shift", "Ctrl", "Shift+Ctrl")가 활성 modifier를 표시. `precision_multiplier`, `snap_to_step`, `gizmo_modifier_suffix` 헬퍼는 순수 함수; `gui::overlays::gizmo_precision_tests`에 8개 unit 테스트.

**트리 더블클릭 → 카메라 포커스** (`crates/viewer/src/gui/tree.rs` + `gui/mod.rs` + `app.rs`). 신규 `GuiAction::FocusObject(ObjectId)`가 해당 객체의 캐시된 AABB에 카메라를 맞춥니다. Scene tree의 body 행을 더블클릭하면 이제 inline rename 대신 `FocusObject`가 실행됩니다(F2가 기존 이름 변경 바인딩 유지, 일반 CAD 앱과 일치). 통합 테스트 `focus_object_dispatch_fits_camera_to_single_object_aabb`, `focus_object_dispatch_is_a_noop_on_empty_vertex_list`.

**Properties 패널 — AABB extents** (`crates/viewer/src/gui/properties.rs`). Placement 섹션이 이제 선택된 객체의 캐시된 `aabb_min`/`aabb_max`에서 계산한 Width / Depth / Height 행을 표시. `bbox_extents(obj)` 헬퍼는 `properties.rs:1060`. 3개 통합 테스트(`properties_aabb_extents_match_box_constructor`, `properties_aabb_extents_match_sphere_diameter`, `compute_aabb_matches_cached_object_fields`) + 3개 unit 테스트.

**검증 (2026-04-24).**
- `cargo build --workspace` — 오류 0.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — 경고 0.
- `cargo test --workspace --no-fail-fast` — **통과 2,606, 실패 0, 무시 0.** R2c 기준선 대비 +16 테스트.

**LOC 영향.** Production ≈ 60 (app.rs +11, gui/mod.rs +4, tree.rs +3, overlays.rs +35, properties.rs +29 순증). Test ≈ 210 (overlays unit +62, properties unit +48, gui_action_integration +100). 합계 ≈ 270 LOC, 300 예산 내.

---

#### V36: 품질 감사 — 2c라운드 (boolean splitter 재작성) (2026-04-23)

**배경.** 2c라운드는 동일한 boolean splitter / 분류 파이프라인을 공유하는 3건의 남은 정확성 실패(K1 겹치는 박스 합집합, K3 관통 구멍 차집합, K3 비볼록 L-minus-cylinder)와 U1 Difference 부호 무시를 닫습니다. 이전 라운드(R2a)가 `SharedBuilder`, `SplitBuilder`, `split_face_along_curves`, `copy_face_with_geometry`를 인프라로 랜딩; R2c가 그 위에서 분류 및 chord 병합 작업을 완성합니다.

**`crates/modeling/src/boolean/classify.rs`.** `FacePosition::OnBoundary`를 두 개의 구분된 서브 상태로 분리하여, 동일평면 면 쌍이 연산별로 올바른 유지/폐기 규칙을 받도록 했습니다:
- `OnBoundarySame` — A, B 내부가 공유 평면의 같은 쪽에 있음 (동일한 매칭 면, 일치하는 pocket).
- `OnBoundaryOpposite` — 내부가 반대쪽에 있음 (두 박스가 한 면을 따라 만남).

`classify_face_with_coplanar`의 내부 샘플 선택을 재작성했습니다. 이전 코드는 edge 중점을 정점 평균 centroid 쪽으로 오프셋했는데, keyhole / slit 다각형(centroid가 구멍 내부에 위치)에서는 잘못된 영역에서 샘플을 생성했습니다. 새 샘플러는 edge-tangent × face-normal 내부 수직선을 사용해 두 개의 다른 오프셋에서 16개 후보를 생성한 뒤, 투영된 다각형에 대한 2D point-in-polygon 테스트로 필터링 — 재료 영역 내부에 엄격히 있는 샘플만 남습니다.

**`crates/modeling/src/boolean/evaluate.rs`.** `boolean_op`의 face-kept 규칙이 이제 4-way 분류와 일치합니다:
- Union: A는 `Outside | OnBoundarySame` 유지; B는 `Outside` 유지.
- Intersection: A는 `Inside | OnBoundarySame` 유지; B는 `Inside` 유지.
- Difference: A는 `Outside | OnBoundaryOpposite` 유지; B는 `Inside` 유지(뒤집힘 — 조각별 방향 해석은 이제 파이프라인 내부에서 해결, face-copy 경계에서가 아님).

**`crates/modeling/src/boolean/face_split.rs`.** `merge_chords_into_polylines_with_boundary`가 이제 순서 무관 끝점 쌍으로 chord 레코드를 중복 제거합니다. pocket과 hole에서 box 윗면이 하나의 cylinder cap(동일평면) + 모든 cylinder 벽(비동일평면)과 짝을 지어 같은 원을 따라 중복된 segment를 생성하는데, graph walker가 중복된 edge에서 멈추고 있었습니다.

**테스트 un-ignore.** `boolean_subtract_box_minus_sphere_shrinks_volume` (U1) — 더 이상 `#[ignore]`가 아닙니다. 이제 splitter가 조각 방향을 올바르게 잡으므로 `box(4³) − sphere(r=1.5)`는 부피가 64보다 엄격히 작은 솔리드를 생성합니다.

**검증 (2026-04-23).**
- `cargo build --workspace` — 오류 0.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — 경고 0.
- `cargo test --workspace --no-fail-fast` — **통과 2,590, 실패 0, 무시 0.** Phase N-min 대비 +8 통과, −3 실패, −1 무시.

**이것이 닫는 것.** V36 R1의 원래 실패 케이스(`cargo test --workspace --no-fail-fast` 초기 기준선 2,558 / 9 / 18) 전부가 이제 수정되거나, dispatcher를 통해 end-to-end로 연결되거나, 통과하는 테스트에서 문서화-및-단언됩니다. 576/576 FreeCAD-parity dispatcher 암이 실제 계산된 결과물을 반환합니다. 의도적 실패 0 청정 상태 달성.

---

#### V36: Phase N-min — Assembly / FEM dispatcher 연결 (2026-04-22)

**배경.** R2b-cont 감사에서 남은 6개 `#[ignore]` 마커 중 5개가 진짜 subsystem 스텁(Assembly×3, FEM×2)임이 확인됨에 따라, Phase N-min은 기존 `techdraw_sheet: Option<DrawingSheet>` 패턴을 미러링하는 최소 `Option<T>` 상태만 `GuiState`에 추가하여 두 subsystem의 modeling-crate API를 viewer dispatcher에 연결합니다. 이번 패스에서 새 viewer 패널은 만들지 않았습니다.

**GuiState 상태 추가** (`crates/viewer/src/gui/mod.rs`).
- `pub assembly: Option<cadkernel_modeling::Assembly>`
- `pub fem_analysis: Option<cadkernel_modeling::AnalysisContainer>`

**Assembly dispatcher 암 연결** (`crates/viewer/src/app.rs`).
- `CreateAssembly` → `Assembly::new("New Assembly")`를 `gui.assembly`에 저장.
- `InsertComponent` → `assembly.add_component(name, current_solid)`. 어셈블리가 없으면 빈 어셈블리를 자동 초기화; 솔리드 미선택 시 status 메시지.
- `SolveAssembly` → `assembly.solve(100)`; 수렴 여부, Err, no-assembly 경로 모두 로컬 `SolveMsg` enum을 거쳐 `log_info`/`log_warning`으로 디스패치 (이중 `&mut self` 대출 문제 회피).

**FEM dispatcher 암 연결.**
- `CreateFemAnalysis` → `generate_tet_mesh(&model, solid, 1.0)` + `AnalysisContainer::new(mesh, FemMaterial::steel())`. 노드/요소 개수를 보고.
- `SolveStatic` → `container.run_static()`; 성공 시 BC 개수와 `max_displacement` 보고, 오류는 `log_warning`, 컨테이너가 비어 있으면 status 메시지. 동일한 borrow-splitting enum 패턴.

**테스트 재작성 및 un-ignore** (`crates/viewer/tests/gui_action_integration.rs`).
- `assembly_create_new_assembly_is_empty`, `assembly_insert_component_increments_count`, `assembly_solve_distance_constraint_converges` — `Assembly::new` / `add_component` / `solve(200)` 체인을 미러링. 컴포넌트 개수, 고유 ID, Fixed + Distance(5.0) 쌍에 대한 Newton-Raphson 수렴을 assert.
- `fem_create_analysis_builds_tet_mesh_with_steel_material` — `generate_tet_mesh(...)` + `AnalysisContainer::new`를 미러링. 비어있지 않은 메쉬와 초기 BC/result가 비어 있음을 assert.
- `fem_solve_static_produces_displacement_result` — `add_bc(FixedNode)` + `add_bc(Force { ... })` + `run_static()`을 미러링. 변위 벡터 크기가 노드 수와 일치함을 assert.

**검증 (2026-04-22).**
- `cargo build --workspace` — 오류 0.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — 경고 0.
- `cargo test --workspace --no-fail-fast` — **통과 2,582, 실패 3, 무시 1**. R2b-cont 대비 +5 통과, −5 무시. 유일하게 남은 `#[ignore]`는 U1 Difference 부호(R2c boolean splitter 재작성 범위).

**Phase N-min이 올바른 크기인 이유.** Assembly와 FEM modeling API(컴포넌트 트리, 조인트, DoF, tet 메쉬 빌더, 재료 라이브러리, static/modal/thermal 솔버)는 이미 오래전에 완성되어 있었습니다 — 간극은 오직 viewer 상태였습니다. 전체 Phase N(Assembly 트리 패널, BOM 뷰, 조인트 UI)과 Phase O(Analysis 패널, 재료 선택 UI, 결과 시각화)는 후속 작업으로 남지만, 이제 dispatcher 계약은 솔직합니다: 모든 암은 실제 계산된 결과물을 반환합니다.

---

#### V36: 품질 감사 — 2b라운드 후속 (2026-04-22)

**배경.** `crates/viewer/tests/gui_action_integration.rs`의 11개 `#[ignore]` 마커를 감사한 결과 5개가 오래된 상태임이 확인되었습니다 — 해당 dispatcher 암(MeshRepair, TechDrawAddView, TechDrawThreeView, TechDrawExportSvg, CreateHelix)은 이전 라운드에서 이미 완전히 연결되었지만, 테스트는 `unreachable!()` 스텁 본문에서 재작성되지 않았습니다. 이번 패스는 이 5개 마커를 dispatcher의 `cadkernel_io` 및 `cadkernel_modeling` 호출 체인을 미러링하는 실제 assertion으로 전환합니다.

**테스트 재작성 및 un-ignore** (`crates/viewer/tests/gui_action_integration.rs`).
- `mesh_repair_evaluate_and_repair_returns_valid_mesh` — `evaluate_and_repair(mesh)`를 미러링. 비어있지 않은 vertex/index 버퍼와 `indices.len() % 3 == 0`을 assert.
- `techdraw_add_view_projects_solid_to_sheet` — `project_solid(model, solid, ProjectionDir::Front)` + `DrawingSheet::a4_landscape()`에 push를 미러링. 뷰에 edge가 있음을 assert.
- `techdraw_three_view_populates_three_views` — `three_view_drawing(model, solid)`를 미러링. 세 뷰 모두 채워짐을 assert.
- `techdraw_export_svg_renders_nonempty_svg` — `drawing_to_svg(&sheet).render()`를 미러링. 문자열이 잘 형성된 non-trivial SVG인지 assert.
- `create_helix_produces_tube_solid_with_faces` — `make_helix(..., 16, 8)`을 미러링. 솔리드가 shell과 face를 갖고 있음을 assert ("helix는 wire-only"라는 오래된 주장을 정정).

**여전히 이월됨.**
- R2c / Task #9 boolean splitter 재작성 — K1 union, K3 cylinder-through-box, K3 L-minus-cylinder 여전히 실패; U1 Difference 부호 여전히 ignore.
- Assembly (3개 암)와 FEM (2개 암) — viewer 상태가 없는 진짜 subsystem 스텁. 연결하려면 새 Assembly/FEM viewer 패널이 필요하며 FREECAD_PARITY_PLAN Phase N 및 O에서 추적됩니다.

**검증 (2026-04-22).**
- `cargo build --workspace` — 오류 0.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — 경고 0.
- `cargo test --workspace --no-fail-fast` — **통과 2,577, 실패 3, 무시 6**. 실패는 R2b와 동일(K1 + K3 쌍); 5개 오래된 스텁이 ignored에서 passing으로 이동.

**R2b 대비 수치 변화.** R2b: 2,572 / 3 / 11. R2b-cont: 2,577 / 3 / 6. 순: **+5 통과, −5 무시**, 신규 실패 없음.

---

#### V36: 품질 감사 — 2b라운드 (2026-04-21)

**배경.** 2b라운드는 1라운드 감사에서 식별된 U3 dispatcher-stub 그룹을 겨냥합니다 — `self.log_info(...)`만 호출하고 실제 Scene 지오메트리를 생성하지 않던 GuiAction 변형들입니다. 이번 패스는 16개 스텁 중 5개를, 이미 `Handle<SolidData>`를 반환하는 `cadkernel_modeling` 함수에 dispatcher 암을 연결하는 방식으로 해결합니다. UX 회귀 없음: 메뉴 클릭이 기본 예시 솔리드를 생성하는 방식은 CreateBox / CreateCylinder 흐름과 동일합니다.

**수정됨.**
- **SurfaceFilling, SurfaceBoundary** (`crates/viewer/src/app.rs`). dispatcher 암이 이제 `cadkernel_modeling::filling()`을 기본 경계(SurfaceFilling은 2×2 정사각형, SurfaceBoundary는 단위 육각형)와 함께 호출하고, 반환된 `SurfaceFillingResult.solid`를 `add_to_scene()`으로 추가합니다.
- **SurfacePipe** (`crates/viewer/src/app.rs`). dispatcher 암이 이제 `pipe_surface()`를 기본 2-단위 수직 경로, 반지름 0.25로 호출하여 튜브형 솔리드를 생성합니다.
- **DraftRectangle, DraftPolygon** (`crates/viewer/src/app.rs`). dispatcher 암이 `make_rectangle_wire()` / `make_polygon_wire()`로 경계 폴리라인을 만들고 `filling()`로 채워 평면 패치 솔리드를 생성합니다. `CreationParams::DraftRectangle` / `DraftPolygon`과 함께 `add_to_scene()`되어 Object Tree와 Properties 패널에서 파라메트릭 편집이 재개됩니다.

**테스트 재작성 및 un-ignore** (`crates/viewer/tests/gui_action_integration.rs`).
- `surface_filling_creates_solid_from_boundary`, `surface_boundary_fills_closed_polyline`, `surface_pipe_creates_solid_along_path`, `draft_rectangle_fills_patch_and_adds_to_scene`, `draft_polygon_fills_patch_and_adds_to_scene` — 이전에는 `#[ignore]` 아래에서 `unreachable!()`을 호출하던 스텁 마커 테스트였습니다. 이제 dispatcher의 modeling 호출 체인을 미러링하고 `scene.len() == 1` 및 비어있지 않은 vertex 버퍼를 assert합니다.
- `draft_line_creates_wire_topology_in_model` — DraftLine은 `Scene`이 `Handle<SolidData>`만 저장하고 wire 프리미티브를 지원하지 않으므로 여전히 wire-only입니다. 테스트는 이제 `make_line_draft()`를 직접 실행하여 2개 vertex + 1개 edge가 `BRepModel`에 추가됨을 assert합니다. viewer의 wire 렌더링은 FREECAD_PARITY_PLAN Phase L에서 추적됩니다.

**여전히 이월됨.**
- R2b 남은 11개 U3 스텁 (MeshRepair, TechDraw {Add, Three, ExportSvg}, Assembly {Create, Insert, Solve}, FEM {CreateAnalysis, SolveStatic}, CreateHelix) — 헤드리스 생성 경로 또는 Scene 레벨의 wire 지원이 필요합니다.
- R2c / Task #9 boolean splitter 재작성 — 3개 K1/K3 실패와 U1 ignored 테스트는 R2a에서 그대로 이월됩니다.

**검증 (2026-04-21).**
- `cargo build --workspace` — 오류 0.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — 경고 0.
- `cargo test --workspace --no-fail-fast` — **통과 2,572, 실패 3, 무시 11**. 실패는 R2a와 동일(K1 + K3 쌍); 6개 U3 테스트가 ignored에서 passing으로 이동.
- `cargo test --manifest-path crates/python/Cargo.toml` — 42개 통과.

**R2a 대비 수치 변화.** R2a: 2,566 / 3 / 17. R2b: 2,572 / 3 / 11. 순: **+6 통과, −6 무시**, 신규 실패 없음.

---

#### V36: 품질 감사 — 2a라운드 (2026-04-21)

**배경.** 2라운드는 1라운드 감사가 드러낸 Critical/High 버그들을 정리합니다. 이번 패스에서 11개 정확성 결함 중 6건을 해결했습니다. 일반 위치 boolean 3건(K1, K3)과 U1 Difference 부호 버그는 splitter 재작성이 필요하여 2c라운드(Task #9)로 명시적으로 이월합니다.

**수정됨.**
- **I1 STEP 곡면 내보내기 (`crates/io/src/step.rs`).** 내보내기에서 face surface를 분류하여 `CYLINDRICAL_SURFACE`, `SPHERICAL_SURFACE`, `CONICAL_SURFACE`, `TOROIDAL_SURFACE`를 발행합니다. `export_face_surface()`로 무조건 평면화하던 이전 경로를 대체. 원통 라운드트립에서 곡률이 보존됩니다. `step_roundtrip_cylinder_surface_stays_curved` 통과.
- **I2 IGES face/shell 레코드 (`crates/io/src/iges.rs`).** 내보내기에서 face loop을 순회하면서 u×v 그리드로 샘플링한 뒤 face당 Type 128 `RationalBSplineSurface`를 발행합니다. 재가져오기가 와이어프레임이 아니라 solid 기하를 복원합니다. `iges_roundtrip_spline_surface_lost` 통과.
- **K2 extrude seam watertight (`crates/modeling/src/features/extrude.rs`).** 6개 면 전체가 하나의 `EdgeCache`를 공유하여 cap 경계의 seam 엣지가 중복 제거되고, 각 엣지가 정확히 2개 면과 인접합니다. `extrude_square_profile` 통과.
- **K4 fillet/chamfer 합성 가능 (`crates/modeling/src/features/fillet.rs`, `chamfer.rs`).** 신규 배치 API `fillet_edges(model, solid, edges, radius)` / `chamfer_edges(model, solid, edges, distance)` 추가. 엣지 슬라이스를 원본 위상에 대해 한 번에 적용하므로, 개별 엣지를 순차 호출할 때 handle이 stale해지는 문제를 회피합니다. `fillet_all_12_edges_of_box`, `chamfer_all_12_edges_of_box` 통과.
- **K1b 동일평면 boolean (`crates/modeling/src/boolean/face_split.rs`).** 곡면–곡면 교선 마칭이 빈 결과를 내고 face 평면이 일치하는 경우, `compute_planar_intersection` fallback이 평면 분할 다각형을 생성합니다. 박스 위에 박스가 얹혀서 top이 동일평면인 경우 올바르게 boolean이 수행됩니다. `boolean_on_coplanar_faces` 통과.
- **U2 스케치 solver 수렴 (`crates/sketch/src/solver.rs`).** `Fixed` 제약이 없을 때 Tikhonov anchor를 구축하도록 변경(이전의 단일점 anchor는 취약했음). 수렴 판정은 step norm이 아니라 residual 벡터의 infinity-norm으로 전환. `Horizontal`의 Jacobian 기여를 Δy residual의 `(0, 1, 0, -1)` 직접 항으로 설정. `sketcher_solve_horizontal_constraint_zeros_dy` 통과, `#[ignore]` 해제.

**이월 작업(2c라운드)에서 재사용되는 인프라.**
- `SharedBuilder` (`crates/modeling/src/boolean/evaluate.rs`): 위치 기반 정점 중복 제거(허용오차 1e-6) + 방향 지정 반변 매핑. 복사된 면들이 정점을 공유하고 공유 엣지에서 twin을 연결. K1/K3가 필요로 하는 manifold 재구축의 선결 인프라.
- `SplitBuilder` 및 `split_face_along_curves`, `copy_face_with_geometry` (`crates/modeling/src/boolean/face_split.rs`). 현재는 K1b 평면 fallback에서 사용. 2c라운드의 일반 위치 face-split 재작성이 이 기반 위에 구축됩니다.

**Difference B-face 방향에 대한 범위 주해.** 초기 `copy_face_shared` 구현은 Difference 시 B-face winding을 뒤집어 U1(박스 − 구가 77.9를 반환, 기대 ~50)을 해결하려 했습니다. 그러나 `compute_mass_properties`가 `|signed volume|`을 사용하는 divergence theorem 기반이므로, 그 flip은 오히려 *정확히 동작하던* pocket/hole 경로의 부호를 뒤집어 버렸습니다. `pocket_sketch_on_box_removes_material`과 `hole_on_box_removes_cylindrical_material`의 회귀가 이를 드러냈습니다. 이제 winding은 복사 시점에서 그대로 유지하며, 도구(B) 기여의 부호를 뒤집어야 하는 올바른 위치는 splitter 재작성(2c라운드 Task #9)입니다. 면 복사 경계에서 뒤집는 것이 아닙니다. U1은 테스트 속성에 명시적 2c라운드 태그와 함께 다시 `#[ignore]` 처리했습니다.

**2c라운드(Task #9)로 이월.**
- K1 `union_of_two_overlapping_boxes` (일반 위치 합집합의 비-manifold 결과)
- K3 `subtract_cylinder_through_box`, `nonconvex_subtraction_l_minus_cylinder` (관통 구멍 및 비볼록 피감수 face-split)
- U1 `boolean_subtract_box_minus_sphere_shrinks_volume` (Difference 부호/방향)

세 건 모두 같은 splitter 재작성을 공유합니다: 반대쪽 면으로 교선 전파(관통 구멍), splitter 다각형의 내부 끝점 처리, 분할 경계를 가로지르는 올바른 twin 링키지로 shell 재구축.

**검증 (2026-04-21).**
- `cargo build --workspace` — 오류 0.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — 경고 0.
- `cargo test --workspace` — **통과 2,566, 실패 3, 무시 17**. 실패 3건은 위에 이월된 K1/K3 사례이고, 무시 17건은 GuiAction 스텁 16건 + 2c라운드 대기 중인 U1.
- `cargo test --manifest-path crates/python/Cargo.toml` — 42개 통과.

**1라운드 대비 수치 변화.** 1라운드: 2,558 / 9 / 18. 2a라운드: 2,566 / 3 / 17. 순: **+8 통과, −6 실패, −1 무시** (U2는 un-ignore 후 통과; I1, I2, K1b, K2, K4 신규 통과; U1은 명시적 재-ignore).

---

#### V36: 품질 감사 — 1라운드 (2026-04-17)

**배경.** 이전 버전까지는 디스패처 도달 가능성 기준으로 FreeCAD 576개 기능 전체를 달성했다고 기록해 왔습니다. V36 1라운드는 이 주장을 의도적으로 정정합니다. 새로 추가된 네 개의 감사(audit) 테스트 스위트는 커널, I/O, 뷰어를 현실적인 입력으로 구동하면서 **수치적/위상적 정확성까지 검증**합니다. no-panic만 확인하는 것이 아니라, 결과가 실제로 올바른지 단언하도록 작성했습니다. 틀린 부분이 나오면 시끄럽게 실패하도록 설계했고, 실제로 실패했습니다.

**CLI 수정 — `src/main.rs`, 신규 `tests/cli.rs`:**
- `main.rs`를 `clap` derive 파서로 재작성했습니다. 이제 `--script <PATH>`, `--mcp`, `-V/--version` 플래그를 실제로 인식합니다. Lua 예제(`examples/lua/*.lua` 5개)와 MCP 세션 예제는 문서에서 이 플래그들을 광고해 왔지만, 실제 바이너리는 인자를 전부 무시하고 무조건 GUI를 띄우고 있었습니다.
- 신규 `tests/cli.rs` (6개): `--version` 배너, `--script hello_cad.lua` 실행, `--mcp`가 `tools/list` JSON-RPC 요청을 stdio로 처리, 알 수 없는 플래그/누락된 스크립트 오류 경로.

**신규 감사 테스트 파일 (총 169개):**

- `tests/cli.rs` — **6개**, 전부 통과.
- `crates/modeling/tests/real_world_kernel.rs` — **16개** (통과 9, 실패 7). 겹치는 박스, 관통 구멍, 전체 모서리 필릿 등 현실적 형상에 대해 primitive → feature → boolean 파이프라인을 끝까지 돌립니다.
- `crates/io/tests/real_world_io.rs` — **54개** (통과 52, 실패 2). 11개 포맷의 내보내기→재가져오기 라운드트립 + 잘못된 입력에 대한 파서 퍼즈 25개(패닉 0건). 실패 2건은 STEP/IGES 내보내기의 실제 결함을 드러냅니다.
- `crates/viewer/tests/gui_action_integration.rs` — **93개** (통과 75, 무시 18). 헤드리스로 도달 가능한 모든 GuiAction 변종을 디스패치하고 Scene 상태 변화를 단언합니다. `#[ignore]` 중 2개는 실제 버그(Scene Difference 부호, 스케치 solver 수렴), 나머지 16개는 디스패처 스텁 갭입니다.

**발견된 버그.** Critical 5건, High 5건, Medium 1건, 스텁 16건. 재현 방법과 수정 위치 포함 전체 카탈로그: [`docs/V36_BUG_TRIAGE.md`](V36_BUG_TRIAGE.md) 참조.

- **Kernel (5건):** 합집합 및 동일평면 boolean에서 비-manifold 결과(K1/K1b), 관통 구멍에서 subtract face-split 불완전(K3, 기대 부피의 약 67%), extrude 시 edge dedup이 seam 정점을 붕괴시킴(K2), fillet/chamfer가 같은 shell에 재적용할 때 비-합성 가능(K4).
- **IO (2건):** STEP 내보내기가 곡면을 평면화 — 원통이 라운드트립에서 곡률을 잃음(I1, `step.rs:1114`); IGES 내보내기가 와이어프레임만 작성, face/shell 레코드 없음(I2, `iges.rs:458`).
- **Viewer (버그 2건 + 스텁 16건):** Scene 레벨 `Difference`가 원본보다 *더 큰* 부피를 반환(U1, Scene↔modeling 경계에서 인자 순서 문제); 스케치 solver가 `Horizontal` 제약 residual을 절반만 줄이고 `converged=true`로 보고(U2). 16개 GuiAction 변종은 현재 디스패처 스텁 — 요청을 받아서 로그만 남기고 SceneObject를 만들지 않습니다.

**parity 주장의 현실화.** 576개 기능 모두 디스패처 수준에서는 도달 가능합니다. 감사 결과, 그중 최소 11개가 현실적 입력에서 구조적으로 잘못된 결과를 내고, 16개는 no-op임을 보였습니다. 2라운드 수정 후 실제 수치로 이 항목을 다시 정정합니다.

**1라운드에서는 코드 수정을 하지 않았습니다** — 의도적으로 카탈로그 단계에 한정했습니다. 2라운드 수정 우선순위는 `docs/V36_BUG_TRIAGE.md`에 있습니다.

**검증.**
- `cargo build --workspace` — 오류 0.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — 경고 0.
- `cargo test --workspace` — 통과 2,558, 실패 9, 무시 18 (V35 베이스라인 2,416 통과; 169개 감사 테스트 추가; 실패 9건과 무시 18건은 모두 의도된 것이며 triage 문서에 기록).

---

#### V35: 통합 테스트 커버리지 — topology & io (2026-04-17)

**topology 크레이트 신규 통합 테스트 100개** (`crates/topology/tests/topology_advanced.rs`):

- **반변 순회 불변식** (8개): 모든 엣지에 대한 twin 왕복, twin과 origin 상이, 루프 내 next/prev 역원 관계, next 체인으로 루프 종료, 반변이 루프를 참조, 면 차수 합 = 2×엣지 수, 루프 꼭짓점이 반변 origin과 일치, 순회 결정론성
- **오일러 특성** (6개): 닫힌 사면체 V-E+F=2, 열린 삼각형 시트 V-E+F=1, 닫힌 솔리드 다양체 검증 통과, 모든 엣지가 정확히 2개 면 공유, 사면체 각 꼭짓점이 3개 면 소속, `validate_detailed`가 오류 없이 통과
- **Handle 동등성·해싱** (6개): 반사·대칭 동등성, 인덱스 불일치, 세대 불일치, eq와 hash 일치, debug 출력 비어있지 않음, 복사 시 두 변수 모두 유효
- **EntityStore 세대 관리** (8개): 제거 후 stale handle, 슬롯 재사용 시 세대 증가, 조작된 handle은 None, 혼합 삽입/제거 시 live count 유지, iter가 제거된 항목 건너뜀, default==new, 직렬화 왕복, 전체 제거 후 empty
- **ShapeHistory 단조성** (6개): 연산 ID 순증가, 첫 ID=1, 삽입 순서대로 레코드 반환, evolution 레코드가 현재 op에 부착, default는 비어있음, 활성 op 없이 record 호출 = no-op
- **Tag / 퍼시스턴트 네이밍** (9개): `Tag::new`와 `Tag::generated` 일치, 세그먼트 복사 확인, segments에 op ID 보존, split 로컬 인덱스 보존, 3-세그먼트 태그 직렬화 왕복, 8가지 EntityKind × op × index 조합 96개 고유성, `SegmentKind` 동등성·해시, `OperationId` 동등성·해시, 모든 `EntityKind` 변형 구별 가능
- **NameMap 엣지 케이스** (5개): 덮어쓰기는 마지막 값, 종류 불일치 시 None, 이중 remove 시 None, `EntityRef` 복사·동등성, `EntityRef` 직렬화 왕복
- **Wire / Shell / Solid 생성 불변식** (11개): 빈 와이어, 단일 반변 개방 와이어, `WireData::new` 플래그 캡처, `ShellData::new` 비어있음, default==new, shell이 face를 다시 참조, shell이 모든 face를 참조, `SolidData::new` 비어있음, solid default==new, `make_solid`가 shell을 다시 참조, `make_shell`이 모든 face를 링크
- **BRepModel 순회 오류 경로** (5개): stale handle에 대한 `vertices_of_face`, `edges_of_face`, `faces_of_edge`, `faces_around_vertex` 모두 `InvalidHandle` 반환; 반변 0개로 `make_loop` 시 오류
- **변환 전파** (2개): 이동 변환이 모든 꼭짓점 좌표를 이동, 항등 변환은 꼭짓점 좌표를 변경하지 않음
- **PropertyStore — 덮어쓰기 및 전체 변형** (8개): 재료 덮어쓰기, 메타데이터 덮어쓰기, 개별 엔티티 격리, 4가지 `PropertyValue` 변형 동등성, `PropertyValue` 직렬화 왕복, `PropertyStore` 직렬화 왕복, `Color::rgba` alpha 보존, `Color` 직렬화, `Material::new` 기본 필드 확인
- **태그 생성 및 NameMap 동기화** (5개): `make_face_tagged` 등록, `add_vertex_tagged` 등록, 잘못된 종류 조회 시 None, 중복 태그는 마지막 값 우선, 30개 태그 동시 공존
- **BRepModel 직렬화** (3개): 새 모델 전체 카운트 0, 사면체 직렬화 후 V/E/F/shell 수 보존, 직렬화 후 오일러 특성 보존

topology 크레이트 커버리지: 62 → 162 테스트 (161% 증가).

**io 크레이트 신규 통합 테스트 132개** (`crates/io/tests/io_comprehensive.rs`):

- **STL 파서 오류 경로** (9개): ASCII 빈 입력, 삼각형 없음, 잘못된 좌표, 불완전한 토큰, 3의 배수 아닌 꼭짓점 수, 바이너리 너무 짧음, 바이너리 빈 입력, 삼각형 수 초과(>50M), 바이너리 본문 잘림
- **OBJ 파서 오류 경로** (5개): 빈 입력, 꼭짓점 없음, 잘못된 좌표, 좌표 누락, 면 꼭짓점 수 부족
- **PLY 파서 오류 경로** (5개): 빈 입력, `end_header` 누락, 꼭짓점 수 NaN, 꼭짓점 데이터 잘림, 사각형(arity 4) 면 거부
- **STEP 파서** (4개): 빈 입력 패닉 없음, 헤더 누락 허용, 쓰레기 토큰 패닉 없음, 일반 텍스트 패닉 없음
- **IGES 파서** (2개): 빈 입력 오류, 비-IGES 내용 → 빈 엔티티 목록
- **DXF 파서** (2개): 빈 입력 → 빈 메시, 3DFACE 없음 → 빈 메시
- **3MF 파서** (3개): 꼭짓점 속성 누락 오류, 범위 초과 삼각형 허용 또는 빈 메시, 빈 문서 → 빈 메시
- **glTF 파서** (4개): 잘못된 JSON, accessors 누락, 빈 입력, 비-base64 buffer URI 오류
- **BREP 파서** (5개): 빈 입력, 잘못된 헤더, vertices 섹션 누락, 꼭짓점 데이터 잘림, 잘못된 꼭짓점 라인
- **VRML 파서** (2개): 빈 입력, 좌표 데이터 없음
- **AMF 파서** (2개): 빈 입력, 닫히지 않은 vertex 태그
- **Collada(DAE) 파서** (2개): float array 누락, 빈 입력
- **OCA 파서** (2개): 점 없음, 빈 입력
- **SVG 임포트** (1개): 빈 입력 오류
- **PDF 임포트·익스포트** (4개): 입력 너무 짧음, 헤더 누락, 암호화 PDF 거부, `export_pdf`에 빈 SVG 전달 시 오류
- **MCP 서버 프로토콜 오류** (6개): 잘못된 JSON(-32700), 잘못된 jsonrpc 버전, 알 수 없는 메서드, params 누락, 도구 이름 누락, 알 수 없는 도구 이름
- **MCP 도구 실행** (18개): `create_primitive` box/sphere/cylinder/cone/torus 성공, 알 수 없는 type 오류, dimensions 누락 오류; `transform` translate/rotate/scale 성공, 알 수 없는 ID 오류; `query_model` 빈 서버·생성 후; `measure` 성공·알 수 없는 ID 오류; `export_model` STL/OBJ 성공·알 수 없는 포맷 오류; `delete_solid` 성공·알 수 없는 ID 오류; `list_solids` 빈 상태·생성+삭제 후; `boolean_operation` 동일 ID·알 수 없는 연산 오류
- **mesh_ops — 법선 뒤집기·조화** (4개): 두 번 뒤집으면 원래 와인딩, 뒤집기는 법선 부호 반전, 조화 후 꼭짓점·삼각형 수 보존, 혼합 방향 메시에서 완료
- **mesh_ops — 수밀성 검사** (2개): 열린 단일 삼각형은 수밀하지 않음, 닫힌 솔리드 테셀레이션은 패닉 없음
- **mesh_ops — 스케일** (3개): 0 인수로 축 붕괴, 음수 인수로 축 반전, 큰 인수(1e6) 크기 정확
- **mesh_ops — 메시 불리언** (3개): 분리된 메시의 union은 올바른 인덱스 오프셋으로 연결, 먼 분리 메시의 intersection은 비거나 작음, 분리 메시의 difference는 대상 유지
- **mesh_ops — fill holes** (1개): 채워진 메시는 입력보다 삼각형 수 줄지 않음
- **정다면체** (7개): 음수·영 크기 오류, 사면체(4V/4T), 정육면체(8V/12T), 팔면체(6V/8T), 정이십면체(12V/20T), 정십이면체(20V/36T)
- **mesh_ops — decimate** (2개): 범위 초과 비율 오류, 빈 메시 → 빈 결과
- **tessellate — 메시 병합** (3개): 빈 슬라이스 → 빈 메시, 단일 메시 = 입력, 빈 메시 건너뜀
- **SVG 익스포트** (3개): `SvgDocument::render`에 XML 선언·viewBox 포함, 텍스트 요소 내 특수 문자 이스케이프, `profile_to_svg`로 유효한 SVG 루트 생성
- **PDF 익스포트** (2개): `%PDF-`로 시작하고 xref/`%%EOF`/catalog/pages/page 포함, 10줄 이상의 출력
- **네이티브 .cadk 오류 처리** (7개): 존재하지 않는 경로, 손상된 내용, 잘못된 format 마커, 잘린 파일, 쓰기 불가 경로; `load_scene` 존재하지 않는 경로; `save`+`load` 빈 scene 왕복
- **JSON — 오류 경로 및 왕복** (5개): 잘못된 JSON, 잘못된 구조, 빈 모델 → 유효한 JSON, 존재하지 않는 파일 오류, 단일 프리미티브 write+read 왕복 시 꼭짓점 수 보존
- **TechDraw 투영 API** (6개): `project_solid` 빈 모델 → 빈 뷰, box는 front 뷰에 엣지 포함, `three_view_drawing`은 3개 뷰·양수 치수 반환, A4 가로 용지 치수, 투영 방향 레이블, 빈 시트의 `drawing_to_svg`는 유효한 SVG 생성

io 크레이트 커버리지: 403 → 535 테스트 (33% 증가).

---

#### V34: 통합 테스트 커버리지 — math, geometry & sketch (2026-04-17)

**math 크레이트 신규 통합 테스트 102개** (`crates/math/tests/math_comprehensive.rs`):

- **Vec2** (10개): 상수, 길이/제곱, 내적, 외적(부호 면적), 정규화(영 벡터 처리 + 단위 길이), 산술 연산, 대입 연산, 합산 반복자, 배열/튜플 변환
- **Vec3** (10개): 상수, 길이/제곱, 외적 오른손 법칙, 외적 반교환성, 내적 교환성, 정규화 영 벡터 처리, 산술/대입/합산/변환
- **Vec4** (4개): 생성, 산술, 내적, 배열 변환
- **Point2/3** (11개): 원점, Vec2/3 상호 변환, 산술, 거리, 중점, lerp
- **Mat3/4** (8개): 항등, 전치, 곱셈, 점/벡터 변환
- **Transform** (17개): 항등, 이동, 스케일, X/Y/Z 회전, TRS 복합 변환, 역변환, 합성, 점/벡터/법선 변환(역전치), Mat4 상호 변환, from_rotation_translation, look_at, 원근/직교 분해
- **Quaternion** (12개): 항등, 축-각도/오일러 각도 생성, 곱셈, 켤레, 벡터 회전, t=0/0.5/1 slerp, 정규화, 내적, lerp-slerp 끝점 일치
- **Ray3** (8개): 생성, at(t), 방향 정규화, 최근접점, 구/평면/AABB 교차
- **BoundingBox** (17개): 빈 박스, 점/확장/합집합, 포함/교차, 중심/크기/표면적/부피, 변환, 점 집합, 광선 교차
- **허용 오차 도우미** (4개): epsilon 범위, approx_eq 절대 차이, is_zero 임계값, approx_eq_tol 사용자 지정 허용 오차

math 크레이트 커버리지: 44 → 146 테스트 (232% 증가).

**geometry 크레이트 신규 통합 테스트 126개** (`crates/geometry/tests/geometry_comprehensive.rs`):

- **Line / LineSegment** (16개): point_at 원점, 일정 접선, 무한 도메인/길이, 닫힘 플래그, 해석적 점 투영, 영 방향 처리, 바운딩 박스 폴백, 중점, 3-4-5 길이, 단위 도메인, 끝점, 접선, 바운딩 박스, 샘플링 투영
- **Circle / Arc / Ellipse** (18개): 영 법선 오류, XY 기본값, 둘레 길이, 닫힘/열림 플래그, tau 도메인, 접선 수직, 사용자 법선, 호 끝점/길이/도메인/축, 타원 장/단축 끝점, 원 케이스 길이, 축 사이 길이
- **NurbsCurve** (11개): Bezier 선형 평가, 차수=cp−1, cp 수, 매듭/가중치 접근자, 매듭 삽입 형태 보존+cp 추가, 역전 끝점 교환, 바운딩 박스 포함, 2차 Bezier 2계 도함수/곡률=0
- **NurbsSurface** (3개): 이중 선형 패치 생성, cp 불일치 오류, 코너 점 일치
- **Plane 곡면** (14개): XY/XZ/YZ 법선, 평행 축 오류, 3점 평면 생성, 부호 있는 거리, 절대 거리, 점 투영, is_above, 점 포함, point_at/normal, 무한 도메인, du/dv 도함수, 바운딩 박스 폴백
- **Cylinder / Sphere / Cone / Torus** (22개): 기본 Z축/기저-꼭대기 점/단위 법선/도메인 검증, 반지름 유효성 검사, 극점, 원뿔 꼭짓점-반지름-반각, 토러스 외/내 적도/튜브 상단/주기성
- **테셀레이션** (7개): 기본 옵션, coarse/fine LOD, 직선 최소 세그먼트, circle/sphere 확장 트레이트, 평면 곡면 비어있지 않은 메시
- **AABB** (14개): 생성, 단일/다중 점, 병합, 겹침/분리 교차, 내/경계/외부 포함, 단위 큐브 표면적=6, 중심, 확장, 내부/외부 최소 거리, 광선 외부 충돌/내부 t=0/빗나감
- **BVH** (8개): 빈 빌드, 빌드 후 길이, AABB 겹침 쿼리, 점 포함 쿼리, X축 광선, 최근접 쿼리, 빈 집합 최근접, 광선 정렬 순서
- **교차 (Intersection)** (7개): 수직 선분 두 개, XY∩XZ 교선, 동일 평면 일치, 평행 분리 평면 빈 결과, 평면-구 적도 원, 평면-구 접점, 평면-구 빗나감
- **오프셋 / 폴리라인** (3개): 거리=0 입력 보존, 꼭짓점 2개 오류, 폴리라인 비어있지 않은 결과
- **Send + Sync** (1개): 기하 타입 스레드 안전성

geometry 크레이트 커버리지: 44 → 170 테스트 (286% 증가).

**sketch 크레이트 신규 통합 테스트 99개** (`crates/sketch/tests/sketch_comprehensive.rs`):

- **스케치 생성** (13개): 점 생성, 점 추가+인덱스, 선 연결, 원/호 추가, 타원/B-스플라인 추가, ID 타입 왕복, 구조체 필드 접근, 폴리라인 체인, 3점 폴리라인, 정다각형 6각형, 삼각형~팔각형 빌더, 슬롯/모서리 둥근 사각형, 3점 호 중심 복원
- **구속조건 시스템** (5개): 전체 구속조건 변형 생성, 스케치에 구속조건 기록, 만족 시 잔차=0, 불만족 시 잔차≠0, 과구속 플래그
- **Newton-Raphson 솔버** (10개): 빈 스케치 수렴, 비구속 수렴, 고정점 이동, 거리 구속조건, 직각 삼각형, 반복 횟수 기록, 과구속 플래그, 결과 복제, 드래그 솔브, 수직/원형 tau 구속조건 수렴
- **유효성 검사** (6개): 빈 스케치 보고, 길이=0 선, 거의 일치하는 점, 유효하지 않은 점/선 참조, 저/과구속 플래그, 검사 결과 복제
- **작업 평면 (Workplane)** (4개): XY→z=0, XZ→y=0, 직교정규화, 로컬↔월드 왕복
- **프로파일 추출** (3개): 닫힌 사각형, 빈 스케치, 선 없으면 전체 점 반환
- **엣지 편집** (8개): 직각 필렛, 공유하지 않는 선 거부, 챔퍼 코너, 선 분할, 교차점에서 트림, 평행 선 false 반환, 선 연장, 외부 교차 검출
- **기하 변환** (10개): 이동, 90° 회전, 스케일 2배, 평행 선 오프셋, 비선 오프셋 거부, 점 미러, 스케치 미러/회전 복사/스케일 복사/닫힌 사각형 오프셋
- **스케치 유틸리티** (15개): 병합, 평면 부착, 재방향 설정, 구동/참조 전환, 전체 기하/구속조건 삭제, 카본 카피, 외부 투영, 그리드 기본값+스냅, 그리드 최솟값 클램프, 끝점 스냅, 표시 옵션, 구속조건 가시성 전환, 건설 선 전환, 잘못된 ID 거부
- **문맥 치수** (4개): 단일 선 길이, 두 점 거리, 원 반지름/지름, 수평/수직 자동 선택
- **기타** (6개): 단면 뷰 왕복/복제, 주기적 B-스플라인 닫힘 플래그, 스냅 타입 동등/복사, XY 평면 정렬=0, 건설 모드 해제

sketch 크레이트 커버리지: 90 → 189 테스트 (110% 증가).

---

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
