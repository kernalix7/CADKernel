# CADKernel — FreeCAD 기능 동등성 마스터 플랜

> **목표**: FreeCAD의 모든 기능을 CADKernel에 구현
> **참고**: wiki.freecad.org (전체 워크벤치 페이지)
> **날짜**: 2026-03-06

---

## 목차

1. [현재 상태 인벤토리](#1-현재-상태-인벤토리)
2. [FreeCAD 기능 갭 분석](#2-freecad-기능-갭-분석)
3. [Phase 로드맵](#3-phase-로드맵)
4. [Phase 상세](#4-phase-상세)
5. [크레이트 아키텍처 변경](#5-크레이트-아키텍처-변경)
6. [검증 기준](#6-검증-기준)

---

## 1. 현재 상태 인벤토리

### 1.1 완전 구현됨

| 카테고리 | 기능 | 크레이트 |
|---------|------|---------|
| **프리미티브** | Box, Cylinder, Sphere, Cone, Torus | modeling |
| **불리언** | Union, Intersection, Difference | modeling |
| **피처** | Extrude, Revolve, Sweep, Loft, Chamfer, Shell, Mirror, Scale, Linear/Circular Pattern | modeling |
| **커브** | Line, LineSegment, Circle, Arc, Ellipse, NurbsCurve | geometry |
| **서피스** | Plane, Cylinder, Sphere, Cone, Torus, NurbsSurface | geometry |
| **교차** | Curve-Curve, Plane-Plane, Plane-Sphere, Plane-Cylinder, Sphere-Sphere, Ray-Surface | geometry |
| **테셀레이션** | 적응형 curve/surface 테셀레이션, Solid→Mesh | geometry, io |
| **NURBS 커브** | De Boor 평가, 노트 삽입, 차수 상승, 접선, 곡률 | geometry |
| **NURBS 서피스** | 텐서곱 평가, 법선, 편미분 | geometry |
| **스케치** | Point, Line, Arc, Circle, 13개 제약조건, Newton-Raphson 솔버, extract_profile | sketch |
| **I/O** | STL (ASCII+Binary), OBJ, glTF, JSON, Native(.cadk), SVG, TechDraw 투영 | io |
| **뷰어** | 6개 워크벤치 탭, ViewCube, 8개 디스플레이 모드, MSAA, 카메라 애니메이션 | viewer |
| **측정** | 질량 특성 (체적, 중심, 관성) | modeling |

### 1.2 스텁 (todo!() / IoError)

| 기능 | 파일 | 상태 |
|-----|------|------|
| `fillet_edge()` | modeling/features/fillet.rs | todo!() |
| `draft_faces()` | modeling/features/draft.rs | todo!() |
| `split_solid()` | modeling/features/split.rs | todo!() |
| `closest_point_on_solid()` | modeling/query.rs | todo!() |
| `point_in_solid()` | modeling/query.rs | todo!() |
| STEP 가져오기/내보내기 | io/step.rs | IoError 스텁 |
| IGES 가져오기/내보내기 | io/iges.rs | IoError 스텁 |

### 1.3 테스트: 252개 통과, 경고 0개

---

## 2. FreeCAD 기능 갭 분석

### 범례
- ✅ = 구현됨
- 🔶 = 부분 / UI 스텁만
- ❌ = 완전 미구현

---

### 2.1 Part 워크벤치 (~50 도구)

#### 프리미티브 (8)
| FreeCAD 도구 | CADKernel | 갭 |
|-------------|-----------|-----|
| Box | ✅ make_box | — |
| Cylinder | ✅ make_cylinder | — |
| Sphere | ✅ make_sphere | — |
| Cone | ✅ make_cone | — |
| Torus | ✅ make_torus | — |
| Tube | ❌ | make_tube (속이 빈 원통) |
| Ellipsoid | ❌ | make_ellipsoid |
| Prism | ❌ | make_prism (정다각형 돌출) |
| Wedge | ❌ | make_wedge (테이퍼 박스) |
| Helix | ❌ | make_helix (3D 나선형 커브/와이어) |
| Spiral | ❌ | make_spiral (평면 나선형) |
| Plane (유한) | ❌ | make_plane_face |
| Regular Polygon | ❌ | make_polygon |
| Shape Builder | ❌ | 인터랙티브 셰이프 생성 |

#### 모델링 연산 (10)
| FreeCAD 도구 | CADKernel | 갭 |
|-------------|-----------|-----|
| Extrude | ✅ extrude | 테이퍼 각도 미지원 |
| Revolve | ✅ revolve | — |
| Mirror | ✅ mirror_solid | 임의 평면 미러 미지원 |
| Scale | ✅ scale_solid | 비균일 XYZ 스케일 미지원 |
| Fillet | ❌ 스텁 | 롤링볼 필렛 |
| Chamfer | ✅ chamfer_edge | 두 거리 모드 미지원 |
| Loft | ✅ loft | Ruled surface 모드 미지원 |
| Sweep | ✅ sweep | Transition 모드 미지원 |
| Section | ❌ | 교차에서의 단면 |
| Cross-Sections | ❌ | 다중 평행 단면 |
| 3D Offset | ❌ | 솔리드 서피스 전체 오프셋 |
| 2D Offset | ✅ offset_polygon_2d | 폴리곤만, 임의 와이어 아님 |
| Thickness (Shell) | ✅ shell_solid | Join type 옵션 미지원 |
| Ruled Surface | ❌ | 2 엣지에서 서피스 |
| Make Face from Wires | ❌ | Wire → Face 변환 |
| Project on Surface | ❌ | 서피스에 와이어/스케치 투영 |

#### 불리언 연산 (7)
| FreeCAD 도구 | CADKernel | 갭 |
|-------------|-----------|-----|
| Union (Fuse) | ✅ | — |
| Cut (Difference) | ✅ | — |
| Intersection (Common) | ✅ | — |
| Section | ❌ | 셰이프 간 교차 커브 |
| Boolean Fragments | ❌ | 불리언에서의 모든 조각 |
| Boolean XOR | ❌ | 배타적 OR 불리언 |
| Compound | ❌ | 불리언 없이 셰이프 그룹화 |
| Explode Compound | ❌ | 컴파운드 분리 |
| Compound Filter | ❌ | 체적/면적 등으로 필터링 |

#### 검사 / 변환 (12)
| FreeCAD 도구 | CADKernel | 갭 |
|-------------|-----------|-----|
| Check Geometry | ❌ | BRep 유효성 검사 |
| Defeaturing | ❌ | 셰이프에서 피처 제거 |
| Shape From Mesh | ❌ | Mesh → BRep |
| Convert to Solid | ❌ | Shell → Solid |
| Reverse Shapes | ❌ | 법선 반전 |
| Refine Shape | ❌ | 불필요한 엣지 제거 |

---

### 2.2 PartDesign 워크벤치 (~52 도구)

#### Body / 구조 (4)
| FreeCAD 도구 | CADKernel | 갭 |
|-------------|-----------|-----|
| New Body | ❌ | Body 컨테이너 (파라메트릭 피처 트리) |
| New Sketch | 🔶 | 면에 부착된 스케치 |
| Shape Binder | ❌ | 다른 Body에서 참조 지오메트리 |
| Clone | ❌ | 파라메트릭 클론 |

#### 첨가 피처 (13)
Pad, Revolution, Additive Loft/Pipe/Helix, 7개 Additive 프리미티브 — 대부분 개별 연산은 존재하나 Body에 대한 통합 워크플로우 미구현 (🔶)

#### 절삭 피처 (14)
Pocket, Hole, Groove, Subtractive Loft/Pipe/Helix, 7개 Subtractive 프리미티브 — 모두 ❌ (통합 워크플로우)

#### 치장 (4)
| FreeCAD 도구 | CADKernel | 갭 |
|-------------|-----------|-----|
| Fillet | ❌ 스텁 | fillet_edge |
| Chamfer | ✅ chamfer_edge | — |
| Draft | ❌ 스텁 | draft_faces |
| Thickness | ✅ shell_solid | — |

#### 변환 (5)
| FreeCAD 도구 | CADKernel | 갭 |
|-------------|-----------|-----|
| Mirrored | ✅ mirror_solid | — |
| Linear Pattern | ✅ linear_pattern | — |
| Polar Pattern | ✅ circular_pattern | — |
| MultiTransform | ❌ | 연쇄 변환 |
| Scaled | ✅ scale_solid | — |

---

### 2.3 Sketcher 워크벤치 (~95 도구)

#### 지오메트리 생성 — 현재 5개 / FreeCAD 30+개
- ✅ Point, Line, Arc, Circle, Rectangle
- ❌ Polyline, Ellipse, Elliptical/Hyperbolic/Parabolic Arc, Polygon (3~N), Slot, Arc Slot, B-Spline (4종류), Construction Mode

#### 제약조건 — 현재 13개 / FreeCAD 22개
- ✅ Fixed, Horizontal, Vertical, Parallel, Perpendicular, Tangent, Coincident, Concentric, Equal, Symmetry, Length, Radius, Angle
- ❌ PointOnObject, Block, HorizontalDistance, VerticalDistance, Diameter, LockPosition, Distance, Refraction

#### 편집 도구 — 0개 / FreeCAD 12개
- ❌ Fillet, Chamfer, Trim, Split, Extend, External, Carbon Copy, Move, Rotate, Scale, Offset, Mirror

#### B-Spline 도구 — 0개 / FreeCAD 7개
- ❌ Convert, Degree Up/Down, Knot Multiplicity Up/Down, Insert Knot, Join

---

### 2.4 TechDraw 워크벤치 (~105 도구)

| 카테고리 | FreeCAD | CADKernel | 갭 |
|---------|:---:|:---:|:---:|
| 페이지 | 7 | 2 | 5 |
| 뷰 | 10 | 3 | 7 |
| 치수 | 12 | 1 | 11 |
| 주석/선/기호 | 76 | 0 | 76 |

---

### 2.5 Assembly 워크벤치 (22 도구)

- 모두 ❌ (어셈블리 문서, 컴포넌트, 14개 조인트 타입, 솔버, 폭발뷰, 시뮬레이션, BOM)

---

### 2.6 Mesh 워크벤치 (34 도구)

- ✅ 가져오기/내보내기 (STL, OBJ, glTF), Mesh From Shape
- ❌ 분석, 수리, 스무딩, 데시메이트, 불리언, 절단, 세그멘테이션, 언래핑 (~29개)

---

### 2.7 Surface 워크벤치 (6 도구)
- 모두 ❌ (Filling, Fill Boundary, Sections, Extend, Curve on Mesh, Blend Curve)

---

### 2.8 I/O 포맷 지원

| 포맷 | FreeCAD | CADKernel | 우선순위 |
|------|---------|-----------|----------|
| **STEP** | ✅ | ❌ 스텁 | **최고** |
| **IGES** | ✅ | ❌ 스텁 | 높음 |
| **BREP** | ✅ | ❌ | 높음 |
| **DXF** | ✅ | ❌ | 높음 |
| **DWG** | ✅ | ❌ | 중간 |
| **3MF** | ✅ | ❌ | 중간 |
| STL | ✅ | ✅ | — |
| OBJ | ✅ | ✅ | — |
| glTF | ✅ | ✅ export | Import 미지원 |
| SVG | ✅ | ✅ | — |

---

### 2.9 NURBS/지오메트리 커널 갭 (vs OCCT)

| OCCT 기능 | CADKernel | 갭 |
|----------|-----------|-----|
| NURBS 커브 해석적 미분 | ❌ | 유한 차분만 |
| NURBS 커브 피팅 (보간) | ❌ | 점 → 커브 |
| NURBS 커브 근사 (최소제곱) | ❌ | 점군 → 커브 |
| NURBS 서피스 해석적 미분 | ❌ | 유한 차분만 |
| NURBS 서피스 피팅 | ❌ | 격자점 → 서피스 |
| 트림 커브 | ❌ | 커브 + 영역 제한 |
| 트림 서피스 | ❌ | 서피스 + 경계 루프 |
| 일반 서피스-서피스 교차 | ❌ | NURBS ↔ NURBS 교차 |
| 커브 오프셋 (3D) | ❌ | 거리만큼 평행 커브 |
| 서피스 오프셋 (3D) | ❌ | 오프셋 서피스 쉘 |
| 노트 제거 | ❌ | 노트 벡터 단순화 |
| 노트 세분화 | ❌ | 노트 벡터 세분화 |
| 재매개변수화 | ❌ | 매개변수 영역 변경 |
| 연속성 분석 (G0/G1/G2) | ❌ | 서피스 연속성 확인 |
| 표면 곡률 (가우스/평균) | ❌ | 법선만 있음 |
| 등매개변수 커브 추출 | ❌ | 일정 u/v에서의 커브 |
| 룰드 서피스 생성 | ❌ | 2 커브에서 서피스 |
| 회전 서피스 생성 | ❌ | 커브 회전으로 서피스 |
| 돌출 서피스 생성 | ❌ | 커브 이동으로 서피스 |
| Gordon 서피스 (Coons 패치) | ❌ | 경계 커브에서 서피스 |
| 채우기 서피스 (N면) | ❌ | 임의 경계 채우기 서피스 |

---

## 3. Phase 로드맵

### 개요 (17 Phases)

| Phase | 이름 | 예상 규모 | 우선순위 |
|-------|-----|----------|----------|
| **A** | **NURBS 커널 완성** | 대규모 | **최고** |
| **B** | **트림 서피스 & 정확한 B-Rep** | 대규모 | **최고** |
| **C** | **서피스-서피스 교차** | 대규모 | **최고** |
| **D** | **STEP 가져오기/내보내기** | 대규모 | **최고** |
| **E** | **나머지 모델링 연산** | 중간 | 높음 |
| **F** | **추가 프리미티브** | 중간 | 높음 |
| **G** | **고급 불리언 연산** | 중간 | 높음 |
| **H** | **Sketcher 완성** | 대규모 | 높음 |
| **I** | **PartDesign 파라메트릭 워크플로우** | 대규모 | 높음 |
| **J** | **Surface 워크벤치** | 중간 | 높음 |
| **K** | **TechDraw 완성** | 대규모 | 중간 |
| **L** | **Assembly 워크벤치** | 대규모 | 중간 |
| **M** | **Mesh 워크벤치 완성** | 중간 | 중간 |
| **N** | **파일 포맷 확장** | 중간 | 중간 |
| **O** | **Draft 워크벤치** | 대규모 | 낮음 |
| **P** | **FEM 워크벤치** | 매우 대규모 | 낮음 |
| **Q** | **품질 검증 & 마무리** | 중간 | 최종 |

---

## 4. Phase 상세

---

### Phase A: NURBS 커널 완성

> **왜 먼저**: NURBS는 모든 것의 수학적 기초. STEP 파일, 정확한 불리언, 서피스 연산 — 모두 견고한 NURBS 커널에 의존.

#### A.1 해석적 미분
- **파일**: `geometry/src/curve/nurbs.rs`, `geometry/src/surface/nurbs.rs`
- **작업**:
  - `derivative_basis_funs()` 구현 — 기저함수 미분 (The NURBS Book, Algorithm A2.3)
  - NurbsCurve: 해석적 `tangent_at()`, `second_derivative_at()` (유한 차분 대체)
  - NurbsSurface: 해석적 `du()`, `dv()`, `normal_at()`, `second_derivatives()`

#### A.2 노트 연산
- **파일**: `geometry/src/curve/nurbs.rs`, `geometry/src/surface/nurbs.rs`
- **작업**:
  - `remove_knot()` — 허용 오차 범위 내 노트 제거
  - `refine_knots()` — 다수 노트 한번에 삽입
  - `reparameterize()` — 아핀 재매개변수화
  - `decompose_to_bezier()` — NURBS에서 베지에 세그먼트 추출
  - 서피스 버전: `insert_knot_u/v()`, `refine_knots_u/v()`, `remove_knot_u/v()`

#### A.3 커브 피팅 & 보간
- **새 파일**: `geometry/src/curve/fitting.rs`
- **작업**:
  - `interpolate_points()` — 점을 통과하는 전역 커브 보간 (Algorithm A9.1)
  - `interpolate_points_with_tangents()` — 에르미트 보간
  - `approximate_points()` — 최소제곱 B-spline 근사 (Algorithm A9.7)
  - 현장 길이 매개변수화, 구심 매개변수화

#### A.4 서피스 피팅 & 생성
- **새 파일**: `geometry/src/surface/fitting.rs`
- **작업**:
  - `interpolate_grid()` — 점 격자에서의 전역 서피스 보간
  - `approximate_grid()` — 최소제곱 서피스 근사
  - `make_ruled_surface()` — 2 커브 사이 선형 보간
  - `make_revolution_surface()` — 커브 + 축에서의 회전 서피스
  - `make_extrusion_surface()` — 커브를 벡터 방향으로 이동하여 서피스
  - `make_pipe_surface()` — 경로를 따라 커브를 이동하여 서피스

#### A.5 커브 & 서피스 유틸리티
- **작업**:
  - `reversed()` — 역방향 매개변수화
  - `split_at()` — 매개변수에서 커브 분할
  - `join()` — 두 커브 끝끝 연결
  - `offset_curve()` — 거리만큼 평행 커브 (3D)
  - `isocurve()` — 일정 u 또는 v에서 커브 추출
  - `gaussian_curvature()`, `mean_curvature()` — (u,v)에서의 서피스 곡률
  - `principal_curvatures()` — (u,v)에서의 최소/최대 곡률

---

### Phase B: 트림 서피스 & 정확한 B-Rep

> **왜**: 지오메트리와 토폴로지의 다리. 트림 서피스 없이는 면이 임의 경계를 가질 수 없음.

#### B.1 트림 커브
- **새 파일**: `geometry/src/curve/trimmed.rs`
- `TrimmedCurve` 구조체: 기본 커브 + (t_start, t_end)

#### B.2 트림 서피스 (파라메트릭 면)
- **새 파일**: `geometry/src/surface/trimmed.rs`
- `TrimmedSurface` 구조체: 기본 서피스 + 외부 와이어 (UV 공간) + 내부 와이어 (구멍)
- `point_in_face()` — (u,v)가 트림 영역 내부인지 테스트
- 트림 서피스 테셀레이트: 트림 경계 내부에서만 메시 생성

#### B.3 지오메트리-토폴로지 바인딩 (자동 바인딩)
- 각 프리미티브 (Box, Cylinder, Sphere, Cone, Torus)가 생성 시 자동으로 면에 서피스, 엣지에 커브 바인딩

#### B.4 정확한 테셀레이션
- 바인딩된 서피스 지오메트리를 사용하여 면 테셀레이트 (팬 삼각분할이 아닌)

---

### Phase C: 서피스-서피스 교차

> **왜**: 곡면에서의 정확한 불리언 연산에 필수.

#### C.1 일반 SSI (마칭 방법)
- `intersect_surfaces()` — 일반 서피스-서피스 교차
- 시작점 찾기: 세분할 + Newton-Raphson
- 마칭 알고리즘: 접선 예측자 + Newton 보정자
- 적응형 스텝 크기
- 루프 검출 및 닫기
- 출력: `Vec<IntersectionCurve>`

#### C.2 해석적 SSI (특수 경우)
- 기존: plane-plane, plane-sphere, plane-cylinder, sphere-sphere
- 새: cylinder-cylinder, cone-plane, cone-sphere, torus-plane

#### C.3 커브-서피스 교차
- `intersect_curve_surface()` — 일반 커브-서피스 교차

#### C.4 정확한 불리언 연산
- 메시 기반 불리언을 서피스 기반 접근으로 대체

---

### Phase D: STEP 가져오기/내보내기

> **왜**: STEP (ISO 10303)은 CAD 데이터 교환의 표준. 이것 없이는 다른 CAD 시스템과 호환 불가.

#### D.1 STEP 파서
- ISO 10303-21 물리 파일 형식 토크나이저
- 엔티티 파서, 참조 해결
- AP203/AP214 지원

#### D.2 STEP 기하학 엔티티
- CARTESIAN_POINT, DIRECTION, VECTOR, LINE, CIRCLE, ELLIPSE
- B_SPLINE_CURVE_WITH_KNOTS, RATIONAL_B_SPLINE_CURVE
- PLANE, CYLINDRICAL_SURFACE, CONICAL_SURFACE, SPHERICAL_SURFACE, TOROIDAL_SURFACE
- B_SPLINE_SURFACE_WITH_KNOTS, RATIONAL_B_SPLINE_SURFACE

#### D.3 STEP 토폴로지 엔티티
- VERTEX_POINT, EDGE_CURVE, ORIENTED_EDGE
- EDGE_LOOP, FACE_BOUND, ADVANCED_FACE
- CLOSED_SHELL, MANIFOLD_SOLID_BREP

#### D.4 STEP 어셈블리 구조
- PRODUCT, PRODUCT_DEFINITION, NEXT_ASSEMBLY_USAGE_OCCURRENCE
- 색상/재질: COLOUR_RGB, SURFACE_STYLE_USAGE

#### D.5 STEP 작성기
- BRepModel → STEP 엔티티 직렬화

---

### Phase E: 나머지 모델링 연산

- **Fillet** — 롤링볼 필렛 (fillet.rs 스텁 대체)
- **Draft** — 면 법선을 각도만큼 회전 (draft.rs 스텁 대체)
- **Split Solid** — 평면 교차 → 면 트림 → 두 솔리드 재구성 (split.rs 스텁 대체)
- **3D Offset Surface** — 법선 방향으로 각 면 오프셋
- **Defeaturing** — 선택 면 제거, 인접 면 확장
- **Shape Queries** — closest_point, point_in_solid, check_geometry
- **MultiTransform** — 연쇄 변환

---

### Phase F: 추가 프리미티브

- `make_tube()` — 속이 빈 원통 (내부/외부 반지름 + 높이)
- `make_ellipsoid()` — 3 반축
- `make_prism()` — 정다각형 베이스 + 돌출
- `make_wedge()` — 테이퍼진 직사각형 솔리드
- `make_helix()` — 나선형 와이어/엣지
- `make_spiral()` — 평면 나선형 커브
- `make_plane_face()` — 유한 평면 면
- `make_polygon_wire()` — 정다각형 와이어

---

### Phase G: 고급 불리언 연산

- `boolean_section()` — 교차 커브 (솔리드가 아님)
- `boolean_fragments()` — 다수 셰이프에서의 모든 조각
- `boolean_xor()` — 배타적 OR 불리언
- `slice_by_plane()` — 평면으로 솔리드 절단 → 두 반쪽
- `compound_from_shapes()` — 불리언 없이 그룹화
- `explode_compound()` — 컴파운드 분리

---

### Phase H: Sketcher 완성

- 추가 지오메트리: Ellipse, BSpline, Slot, Polygon, Construction mode
- 추가 제약조건: PointOnObject, Block, Distance, HorizontalDistance, VerticalDistance, Diameter, LockPosition
- 편집 도구: Trim, Split, Extend, Fillet, Chamfer, External, Carbon Copy, Move, Rotate, Scale, Offset, Mirror
- B-Spline 도구: Convert, Degree Up/Down, Knot Multiplicity, Insert Knot, Join
- GUI: Polyline, Snap system, Auto-constraint

---

### Phase I: PartDesign 파라메트릭 워크플로우

- Body 컨테이너 (피처 트리)
- Additive/Subtractive 파이프라인 (Pad, Pocket, Revolution, Groove)
- Hole 피처 (parametric hole: through, blind, countersink, counterbore, thread)
- 피처 재계산 엔진 (의존성 그래프, 점진적 재계산)

---

### Phase J: Surface 워크벤치

- `coons_patch()` — 4 경계 커브에서의 Coons 쌍선형 패치
- `gordon_surface()` — 커브 네트워크에서의 Gordon 서피스
- `filling_surface()` — N면 패치 (에너지 최소화)
- `skinning_surface()` — 단면 커브를 통과하는 서피스
- `extend_surface()` — 경계에서 외삽
- `blend_curve()` — G0/G1/G2 브리지 커브

---

### Phase K: TechDraw 완성

- 향상된 뷰: 단면뷰, 상세뷰, 숨은선 제거 (HLR)
- 치수 시스템: 선형, 반경, 지름, 각도, 서수, 체인
- 주석: 텍스트, 리더선, 풍선, 용접 기호, 표면 거칠기, GD&T
- 해칭: 단면 해치 패턴
- 템플릿 시스템: A0-A4, 타이틀 블록
- 출력: SVG (향상), DXF, PDF

---

### Phase L: Assembly 워크벤치

- Assembly 문서/컨테이너
- Component (BRepModel + Transform + 메타데이터)
- 14개 조인트 타입 (Grounded, Fixed, Revolute, Cylindrical, Slider, Ball, Distance, Parallel, Perpendicular, Angle, Rack&Pinion, Screw, Gears, Belt)
- 6-DOF 어셈블리 솔버
- 폭발뷰, 간섭 검출
- BOM (재료명세서) 생성

---

### Phase M: Mesh 워크벤치 완성

- 분석: evaluate_mesh, is_watertight, curvature
- 수리: harmonize_normals, fill_holes, remove_degenerate
- 수정: smooth, decimate, refine, scale
- 불리언: union, intersection, difference (메시 레벨)
- 절단: cut_with_plane, section_from_plane, cross_sections
- 유틸: merge, split_by_components, mesh_to_brep, unwrap

---

### Phase N: 파일 포맷 확장

- IGES 가져오기/내보내기 (스텁 대체)
- BREP 형식
- DXF 가져오기/내보내기
- 3MF 가져오기/내보내기
- glTF 가져오기 (현재 내보내기만)
- PLY, AMF, DAE (저우선순위)

---

### Phase O: Draft 워크벤치 (저우선순위)

- 2D 그리기 도구, 수정 도구, 배열 도구, 주석, 스냅 시스템, 레이어

---

### Phase P: FEM 워크벤치 (선택사항 / 장기)

- 메시 생성, 재료 시스템, 경계 조건, 솔버 연동, 후처리

---

### Phase Q: 품질 검증 & 마무리

- 지오메트리 유효성 검사, 성능 최적화, 문서화, 테스트 커버리지 (500+ 테스트)

---

## 5. 크레이트 아키텍처 변경

### 현재 (9 크레이트)
```
core → math → geometry → topology → modeling → io → viewer
                                  ↗              ↗
                            sketch ─────────────┘
                            python (바인딩)
```

### 추가 예정 모듈

| 새 모듈 | 목적 | 위치 |
|---------|------|------|
| `geometry/curve/fitting.rs` | 커브 보간/근사 | geometry |
| `geometry/surface/fitting.rs` | 서피스 보간/근사 | geometry |
| `geometry/curve/trimmed.rs` | 트림 커브 | geometry |
| `geometry/surface/trimmed.rs` | UV 경계 루프 포함 트림 서피스 | geometry |
| `geometry/intersect/surface_surface.rs` | 일반 SSI | geometry |
| `geometry/intersect/curve_surface.rs` | 커브-서피스 교차 | geometry |
| `modeling/src/body.rs` | 파라메트릭 Body/피처 트리 | modeling |
| `modeling/src/assembly/` | 어셈블리 문서, 컴포넌트, 조인트 | modeling |
| `io/src/step.rs` | STEP 파서/작성기 (스텁 대체) | io |
| `io/src/iges.rs` | IGES 파서/작성기 (스텁 대체) | io |
| `io/src/dxf.rs` | DXF 읽기/쓰기 | io |
| `io/src/brep.rs` | BREP 형식 읽기/쓰기 | io |
| `io/src/threemf.rs` | 3MF 형식 읽기/쓰기 | io |

---

## 6. 검증 기준

### Phase별 체크리스트
1. `cargo build --workspace` — 에러 0개
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings` — 경고 0개
3. `cargo test --workspace` — 모든 테스트 통과
4. 문서 업데이트 (CHANGELOG, DEVELOPER_WIKI — ko/en 모두)
5. 메모리 파일 업데이트 (MEMORY.md)

### 최종 목표 메트릭

| 메트릭 | 목표 |
|--------|-----|
| 테스트 | 500+ |
| Clippy 경고 | 0 |
| 워크벤치 | 6+ |
| 파일 포맷 | STEP, IGES, STL, OBJ, glTF, DXF, SVG, 3MF, BREP, CADK |
| 프리미티브 | 12+ |
| 피처 연산 | 20+ |
| 스케치 엔티티 | 8+ |
| 스케치 제약조건 | 20+ |
| 어셈블리 조인트 | 14 |
| NURBS 연산 | 완전 (미분, 피팅, 노트, 트리밍, SSI) |

---

## 부록: FreeCAD 기능 수 요약

| 워크벤치 | FreeCAD 도구 | CADKernel 현재 | 갭 |
|---------|:----------:|:----------:|:---:|
| Part | ~50 | ~18 | ~32 |
| PartDesign | ~52 | ~5 | ~47 |
| Sketcher | ~95 | ~18 | ~77 |
| TechDraw | ~105 | ~8 | ~97 |
| Assembly | ~22 | 0 | ~22 |
| Mesh | ~34 | ~5 | ~29 |
| Surface | ~6 | 0 | ~6 |
| Draft | ~80 | 0 | ~80 |
| FEM | ~90 | 0 | ~90 |
| **I/O 포맷** | ~15 core | ~7 | ~8 |
| **NURBS 커널** | ~27 ops | ~8 | ~19 |
| **합계** | **~576** | **~69** | **~507** |
