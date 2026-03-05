# Performance & Complexity

> 벤치마크 결과 + Big O 복잡도 분석

---

## 벤치마크 환경

- **CPU**: Linux x86_64
- **Rust**: 1.85+ (release profile, criterion 0.5)
- **날짜**: 2026-03-04

## 벤치마크 결과

### 프리미티브 생성

| 연산 | 시간 | Big O | N |
|------|------|-------|---|
| `make_box` | ~8.2 µs | O(1) | 고정 (8V, 12E, 6F) |
| `make_cylinder (32 seg)` | ~72 µs | O(N) | N = 세그먼트 수 |
| `make_cylinder (64 seg)` | ~127 µs | O(N) | N = 세그먼트 수 |
| `make_sphere (16×8)` | ~167 µs | O(N·M) | N×M = 경도×위도 |
| `make_sphere (32×16)` | ~749 µs | O(N·M) | N×M = 경도×위도 |

**스케일링 검증**: cylinder 32→64 (2×N) → 72→127 µs (1.76×) ≈ O(N). sphere 16×8→32×16 (4×) → 167→749 µs (4.49×) ≈ O(N·M).

### 테셀레이션

| 연산 | 시간 | Big O | 설명 |
|------|------|-------|------|
| `tessellate_box` | ~1.0 µs | O(F) | F = 면 수, 삼각형 팬 분해 |
| `tessellate_sphere (32×16)` | ~60 µs | O(F) | F = B-Rep 면 수 |

### 질량 특성

| 연산 | 시간 | Big O | 설명 |
|------|------|-------|------|
| `mass_properties (box)` | ~61 ns | O(T) | T = 삼각형 수 (12개) |
| `mass_properties (sphere)` | ~4.8 µs | O(T) | T = 삼각형 수 (~960개) |

**스케일링**: 12T → 61ns, 960T → 4.8µs → 80× 삼각형에 79× 시간 ≈ O(T).

### STL 내보내기

| 연산 | 시간 | Big O | 설명 |
|------|------|-------|------|
| `stl_write_ascii (sphere)` | ~1.45 ms | O(T) | T = 삼각형 수, 텍스트 포맷팅 |
| `stl_write_binary (sphere)` | ~223 µs | O(T) | T = 삼각형 수, 바이너리 직렬화 |

**ASCII vs Binary**: 바이너리가 ~6.5× 빠름 (문자열 포맷팅 오버헤드 없음).

### 불리언 연산

| 연산 | 시간 | Big O | 설명 |
|------|------|-------|------|
| `boolean_union (box+box)` | ~44 µs | O(Fa·Fb) | 면 쌍 교차 검사 |
| `boolean_difference (box-box)` | ~44 µs | O(Fa·Fb) | 면 쌍 교차 검사 |

### 피처 연산

| 연산 | 시간 | Big O | 설명 |
|------|------|-------|------|
| `extrude (4-point square)` | ~8.4 µs | O(N) | N = 프로파일 정점 수 |

## Big O 복잡도 요약

### 데이터 구조

| 연산 | 복잡도 | 설명 |
|------|--------|------|
| `EntityStore::insert` | O(1) amortized | Vec 기반 generational arena |
| `EntityStore::get` | O(1) | 인덱스 직접 접근 + 세대 검사 |
| `EntityStore::remove` | O(1) | 프리 리스트 푸시 |
| `EntityStore::len` | O(1) | 카운터 유지 |
| `EntityStore::iter` | O(capacity) | 전체 슬롯 순회 |
| `NameMap::lookup_tag` | O(1) avg | HashMap 기반 |
| `NameMap::lookup_handle` | O(1) avg | HashMap 기반 |

### 기하 연산

| 연산 | 복잡도 | 설명 |
|------|--------|------|
| `Curve::evaluate` (Line) | O(1) | 선형 보간 |
| `Curve::evaluate` (NURBS) | O(p·n) | p = 차수, n = 제어점 수 (de Boor) |
| `Surface::evaluate` (Plane) | O(1) | 직접 계산 |
| `Surface::evaluate` (NURBS) | O(p·q·n·m) | 이변수 de Boor |
| `SSI (Plane-Plane)` | O(1) | 법선 외적 |
| `SSI (Sphere-Sphere)` | O(1) | 중심/반경 기반 |

### 토폴로지 연산

| 연산 | 복잡도 | 설명 |
|------|--------|------|
| `validate_topology` | O(E + HE) | 트윈 대칭, 루프 순환 검사 |
| `euler_characteristic` | O(V + E + F) | V - E + F 계산 |
| `solid_faces` | O(Shells × Faces) | BFS 순회 |

### 모델링 연산

| 연산 | 복잡도 | 설명 |
|------|--------|------|
| `make_box` | O(1) | 고정 토폴로지 |
| `make_cylinder(N)` | O(N) | N 세그먼트 |
| `make_sphere(N, M)` | O(N·M) | N×M 격자 |
| `extrude(N)` | O(N) | N 프로파일 정점 |
| `revolve(N, M)` | O(N·M) | N 정점 × M 세그먼트 |
| `sweep(N, M)` | O(N·M) | N 프로파일 × M 경로 세그먼트 |
| `loft(K, N)` | O(K·N) | K 단면 × N 정점 |
| `boolean_op(A, B)` | O(Fa·Fb) | 면 쌍 교차 (broad phase 포함) |
| `linear_pattern(N)` | O(N·S) | N 복사 × S 솔리드 크기 |
| `circular_pattern(N)` | O(N·S) | N 복사 × S 솔리드 크기 |

### I/O 연산

| 연산 | 복잡도 | 설명 |
|------|--------|------|
| `tessellate_solid` | O(F) | F = B-Rep 면 수 |
| `write_stl_ascii` | O(T) | T = 삼각형 수 |
| `write_stl_binary` | O(T) | T = 삼각형 수 |
| `write_obj` | O(V + T) | V = 정점, T = 삼각형 |
| `import_stl` | O(T) | 파싱 + HashMap 정점 중복 제거 |
| `import_obj` | O(V + T) | 파싱 + 팬 삼각화 |
| `compute_mass_properties` | O(T) | 발산 정리 기반 |

## 벤치마크 실행 방법

```bash
cargo bench -p cadkernel-modeling
```

HTML 리포트는 `target/criterion/` 디렉토리에 생성됩니다.
