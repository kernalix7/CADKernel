# CADKernel 검증 체크리스트

커밋 전 코드 품질 검증 체크리스트. 매 릴리스/PR 마다 수행.

---

## 1. 빌드 & 툴체인

| # | 항목 | 명령어 | 통과 기준 |
|---|------|--------|-----------|
| 1.1 | 워크스페이스 빌드 | `cargo build --workspace` | 에러 0건 |
| 1.2 | Clippy 린트 | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | 경고 0건 |
| 1.3 | 포맷 검사 | `cargo fmt --all -- --check` | 차이 0건 |
| 1.4 | 전체 테스트 | `cargo test --workspace` | 전체 통과 (현재 230개) |
| 1.5 | 벤치마크 컴파일 | `cargo bench --no-run -p cadkernel-modeling` | 컴파일 성공 |

---

## 2. 보안

| # | 항목 | 확인 내용 |
|---|------|-----------|
| 2.1 | 입력 유효성 검증 | STL 삼각형 수 제한, OBJ 인덱스 범위, 무제한 할당 없음 |
| 2.2 | 사용자 입력에 `unwrap()` 없음 | 기하 생성자, 파일 파서, 토폴로지 조회 |
| 2.3 | 0 나누기 방어 | 질량 속성, NURBS de_boor, normalize 함수 |
| 2.4 | 도달 가능 경로에 `todo!()` 없음 | STEP/IGES 파서, fillet, sweep, loft — guard 또는 `Err` 반환 |
| 2.5 | SVG/텍스트 이스케이핑 | SVG 출력의 XML 엔티티 이스케이핑 |
| 2.6 | 메모리 범위 검사 | 메시 연산 배열 인덱스, 엔티티 스토어 조회 |

---

## 3. 안정성

| # | 항목 | 확인 내용 |
|---|------|-----------|
| 3.1 | 무한 루프 없음 | `loop_half_edges()` 최대 반복 가드 |
| 3.2 | NaN 전파 방지 | `asin` [-1,1] 클램프, normalize 영벡터 처리 |
| 3.3 | 오버플로 보호 | EntityStore 세대 카운터, 엣지 태그 승수 |
| 3.4 | 카메라 엣지 케이스 | `tick()` duration > 0, pitch 클램프, roll 정규화 |
| 3.5 | 빈 입력 처리 | 빈 메시 `compute_bounds`, 0면 모델 |
| 3.6 | 토폴로지 일관성 | 오일러 특성 검증, 하프엣지 무결성 |

---

## 4. 개인정보 보호

| # | 항목 | 확인 내용 |
|---|------|-----------|
| 4.1 | 하드코딩된 시스템 경로 | 추적 파일에 `/home/username`, `C:\Users\` 없음 |
| 4.2 | API 키/토큰/시크릿 | 하드코딩된 자격증명, 비밀번호, API 키 없음 |
| 4.3 | IP 주소 | 하드코딩된 IP 주소 없음 |
| 4.4 | 하드웨어 정보 | GPU/CPU 모델명 하드코딩 없음 (런타임 조회 OK) |
| 4.5 | 저작자 정보 (의도적) | Cargo.toml, LICENSE, README의 이메일/이름 — 오픈소스 표준 |
| 4.6 | 자격증명 파일 | `.env`, `.pem`, `.key`, `.secrets` 파일 없음 |
| 4.7 | `target/` 디렉토리 | `.gitignore`에 올바르게 등록 |

---

## 5. 성능

| # | 항목 | 확인 내용 |
|---|------|-----------|
| 5.1 | BFS 복잡도 | 스무스그룹 BFS 인접 조회 — 정점당 O(N) |
| 5.2 | 핫패스에 O(n²) 없음 | `edges_of_face`, `faces_around_vertex`, `find_overlapping_face_pairs` |
| 5.3 | 힙 할당 최소화 | ViewCube 프레임당 할당 최소화 (단일 Mesh) |
| 5.4 | 히스토리 버퍼 | `ModelHistory::record()` VecDeque 사용 (Vec::remove(0) 아님) |
| 5.5 | 파일 I/O | 대용량 모델 STL/OBJ 스트리밍 vs 전체 읽기 |
| 5.6 | GPU 리소스 | MSAA 텍스처는 리사이즈 시에만 재생성 |
| 5.7 | 벤치마크 | `cargo bench -p cadkernel-modeling` — 14개 벤치마크 통과 |

---

## 6. 문서 정확성

| # | 항목 | 대상 파일 | 확인 내용 |
|---|------|-----------|-----------|
| 6.1 | 테스트 수 | DEVELOPER_WIKI.md/ko, README.md/ko | `cargo test --workspace` 출력과 일치 |
| 6.2 | 벤치마크 수 | DEVELOPER_WIKI.md, README.md/ko | `.bench_function` 수와 일치 (현재 14개) |
| 6.3 | 셰이딩 파라미터 | wiki/Crate:-viewer.md, MEMORY.md | ambient=0.15, spec_str=0.15, shininess=128 |
| 6.4 | 헤드라이트 오프셋 | wiki/Crate:-viewer.md, MEMORY.md | right×0.5 + up×0.7 |
| 6.5 | 크리즈 각도 | wiki/Crate:-viewer.md, MEMORY.md | 60° (SMOOTH_ANGLE_DEG 일치) |
| 6.6 | 파일 확장자 | README.md/ko, DEVELOPER_WIKI.ko.md | `.cadk` (`.cadkernel` 아님) |
| 6.7 | 크레이트 존재 여부 | README.md/ko 아키텍처 테이블 | 예정 크레이트에 `(예정)` 표기 |
| 6.8 | 함수 시그니처 | wiki, DEVELOPER_WIKI.ko.md | 실제 코드와 일치 (animate_to, rodrigues, 트레이트) |
| 6.9 | CHANGELOG 이중언어 | CHANGELOG.md, CHANGELOG.ko.md | 동일 항목으로 양쪽 업데이트 |
| 6.10 | MEMORY.md | 자동 메모리 파일 | 코드와 값 일치 (헤드라이트, CW/CCW, 파라미터) |

---

## 7. 코드 품질

| # | 항목 | 확인 내용 |
|---|------|-----------|
| 7.1 | 에러 처리 | 실패 가능 작업에 `KernelResult<T>`, 사용자 경로에서 패닉 없음 |
| 7.2 | 일관된 네이밍 | snake_case 함수, CamelCase 타입, UPPER_SNAKE 상수 |
| 7.3 | 데드 코드 | 미사용 import, 함수, 모듈 없음 |
| 7.4 | 타입 안전성 | `Handle<T>` 세대 아레나 — 원시 인덱스 접근 없음 |
| 7.5 | 외적 규칙 | `cross3(f, up)` 전체 통일 — 절대 순서 변경 금지 |
| 7.6 | 각도 정규화 | 누적 가능한 모든 roll/yaw 값에 `wrap_angle()` 적용 |

---

## 빠른 원라이너

```bash
cargo fmt --all && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace
```

---

## 알려진 이슈 (향후 수정 예정)

### CRITICAL (치명적) — 전부 수정됨
- [x] `arbitrary_perpendicular` `.unwrap()` — **수정**: `.unwrap_or(Vec3::X)`
- [x] 바이너리 STL 삼각형 수 상한 — **수정**: MAX_STL_TRIANGLES = 5000만
- [x] STEP/IGES `todo!()` — **수정**: `Err(IoError)`
- [x] `point_in_solid()` 레이캐스팅 부정확 — **수정**: 2D 점-다각형 판별 테스트
- [x] `classify_face()` 오프셋 방향 — **수정**
- [x] `compute_mass_properties` 0 볼륨 — **수정**: 조기 반환 가드
- [x] EntityStore u32 오버플로 — **수정**: u64

### HIGH (높음) — 전부 수정됨
- [x] 무한 도메인 Plane/Line — **수정**: 해석적 오버라이드 + 유한 폴백
- [x] Sphere/Torus/Cone 반지름 검증 — **수정**: `KernelResult` + 유효성 검사
- [x] `NurbsCurve::de_boor` 0 가중치 — **수정**: w < 1e-14 가드
- [x] `loop_half_edges()` 무한 루프 — **수정**: MAX_LOOP = 10만
- [x] 프리미티브 중복 엣지 — **수정**: EdgeCache 중복 제거 (하프엣지 공유)
- [x] STL 쓰기 u32 오버플로 — **수정**: `KernelResult<Vec<u8>>`
- [x] ScreenOrbit `asin` NaN — **수정**: clamp(-1, 1)
- [x] 각도 제약 `tan()` 특이점 — **수정**: `atan2(cross, dot)`
- [x] PointId 범위 검사 — **수정**: `.get()` 폴백
- [x] 간단 뷰어 궤도 방향 — **수정**: dx/dy 부호 반전

### MEDIUM (중간) — 전부 수정됨
- [x] NavConfig 설정 미사용 — **수정**: cube_size, cube_opacity, orbit_steps 적용
- [x] `validate()` 오일러 특성 — **수정**: V-E+F=2 검증 추가
- [x] SVG XML 이스케이핑 — **수정**: `xml_escape()` 헬퍼
- [x] `WorkPlane::new` 직교화 — **수정**: Gram-Schmidt
- [x] BFS 스무스그룹 O(n²) — **수정**: 엣지 기반 로컬 인접 리스트

22개 알려진 이슈 전부 해결 완료.
