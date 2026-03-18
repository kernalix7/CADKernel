# CADKernel Developer Wiki

> **버전**: 0.1.0 (pre-alpha)  
> **최종 업데이트**: 2026-03-16  
> **대상 독자**: CADKernel 커널 개발자, 기여자

---

## 목차

- [1. 아키텍처 개요](#1-아키텍처-개요)
- [2. 크레이트 의존성 그래프](#2-크레이트-의존성-그래프)
- [3. 크레이트별 상세 가이드](#3-크레이트별-상세-가이드)
  - [3.1 cadkernel-core](#31-cadkernel-core)
  - [3.2 cadkernel-math](#32-cadkernel-math)
  - [3.3 cadkernel-geometry](#33-cadkernel-geometry)
  - [3.4 cadkernel-topology](#34-cadkernel-topology)
  - [3.5 cadkernel-sketch](#35-cadkernel-sketch)
  - [3.6 cadkernel-modeling](#36-cadkernel-modeling)
  - [3.7 cadkernel-io](#37-cadkernel-io)
- [4. 구현 완료 단계 (Phase 1–4)](#4-구현-완료-단계-phase-14)
  - [Phase 1: Foundation](#phase-1-foundation)
  - [Phase 2: Persistent Naming + Boolean](#phase-2-persistent-naming--boolean)
  - [Phase 3: Parametric + Sketch + I/O](#phase-3-parametric--sketch--io)
  - [Phase 4: Core Hardening](#phase-4-core-hardening)
- [5. API 설계 원칙](#5-api-설계-원칙)
- [6. 에러 처리 패턴](#6-에러-처리-패턴)
- [7. 테스트 전략](#7-테스트-전략)
- [8. 빌드 및 CI](#8-빌드-및-ci)
- [9. 다음 단계 (Phase 5+)](#9-다음-단계-phase-5)
- [10. 용어 사전](#10-용어-사전)
- [12. 정확한 불리언 연산 (Phase B06-B14)](#12-정확한-불리언-연산-phase-b06-b14)

---

## 1. 아키텍처 개요

CADKernel은 **단방향 계층 아키텍처**를 채택합니다. 상위 크레이트는 하위 크레이트에 의존하지만, 하위 크레이트는 상위를 알지 못합니다.

```
cadkernel (root)          ← 통합 re-export + prelude + E2E 테스트
├── cadkernel-viewer      ← 네이티브 데스크톱 GUI (egui + wgpu), 3D 렌더링, 카메라, 내비게이션
├── cadkernel-python      ← Python 바인딩 (PyO3)
├── cadkernel-io          ← STL/OBJ/glTF/STEP/IGES 테셀레이션 및 I/O
├── cadkernel-sketch      ← 2D 파라메트릭 스케치 + 제약 솔버
├── cadkernel-modeling    ← 프리미티브 빌더, Boolean, Feature Ops
│   ├── cadkernel-topology  ← B-Rep 반변 자료구조 + Persistent Naming
│   │   ├── cadkernel-geometry  ← Curve/Surface 트레이트 + 구현체 (feature flag)
│   │   │   ├── cadkernel-math  ← 벡터, 행렬, 변환, 허용오차
│   │   │   │   └── cadkernel-core  ← KernelError, KernelResult
│   │   │   └── cadkernel-core
│   │   └── cadkernel-core
│   ├── cadkernel-geometry
│   └── cadkernel-math
├── cadkernel-topology
├── cadkernel-geometry
└── cadkernel-math
```

### 핵심 설계 결정

| 결정 | 이유 |
|------|------|
| `cadkernel-core` 분리 | 에러 타입이 최하위 의존성으로, 모든 크레이트에서 공유 |
| topology의 geometry 의존을 `feature flag`로 | 순수 위상만 쓰는 경우 geometry 의존 제거 가능 |
| sketch의 nalgebra 직접 의존 제거 | math 크레이트의 `linalg` 모듈로 재수출하여 버전 충돌 방지 |
| 통합 `prelude` 모듈 | `use cadkernel::prelude::*` 한 줄로 전체 API 접근 |

---

## 2. 크레이트 의존성 그래프

```
cadkernel-core          (의존 없음)
    ↑
cadkernel-math          + nalgebra, glam
    ↑
cadkernel-geometry      + cadkernel-core
    ↑ (feature: geometry-binding)
cadkernel-topology      + cadkernel-core, cadkernel-math
    ↑
cadkernel-modeling      + cadkernel-core, cadkernel-math, cadkernel-geometry
    ↑
cadkernel-sketch        + cadkernel-math, cadkernel-topology
cadkernel-io            + cadkernel-math, cadkernel-topology
    ↑
cadkernel (root)        전체 통합
```

### Feature Flags

| 크레이트 | Feature | 기본값 | 효과 |
|----------|---------|--------|------|
| `cadkernel-topology` | `geometry-binding` | 활성 | `EdgeData.curve`, `FaceData.surface` 필드 포함 |

---

## 3. 크레이트별 상세 가이드

### 3.1 cadkernel-core

**역할**: 모든 크레이트가 공유하는 기초 타입 정의.

**주요 타입**:

```rust
pub enum KernelError {
    InvalidHandle(&'static str),
    InvalidArgument(String),
    ValidationFailed(String),
    TopologyError(String),
    GeometryError(String),
    IoError(String),
}

pub type KernelResult<T> = Result<T, KernelError>;
```

**설계 노트**: `KernelError`는 `Clone + PartialEq + Eq`를 구현하여 테스트에서 직접 비교 가능. `std::io::Error`에서 `From` 변환을 지원하여 `?` 연산자 사용 가능.

---

### 3.2 cadkernel-math

**역할**: CAD 연산에 필요한 모든 수학 기본 타입.

| 타입 | 파일 | 설명 |
|------|------|------|
| `Vec2`, `Vec3`, `Vec4` | `vector.rs` | 2D/3D/4D 벡터. `Copy`, `Default`, `Display`, `From` 지원 |
| `Point2`, `Point3` | `point.rs` | 2D/3D 점. 벡터와 상호 변환 (`From`) |
| `Mat3`, `Mat4` | `matrix.rs` | nalgebra 래퍼. 역행렬, 행렬식 |
| `Transform` | `transform.rs` | 이동, 회전, 스케일, 미러, 합성 |
| `Quaternion` | `quaternion.rs` | 단위 쿼터니언. 축-각 변환, SLERP |
| `Ray3` | `ray.rs` | 3D 레이. 투영, 최근접점, 거리 |
| `BoundingBox` | `bbox.rs` | AABB. 합집합, 교집합, 포함 테스트 |
| `EPSILON` | `tolerance.rs` | 기본 허용오차 `1e-9` |

**연산자 지원**:

```rust
// 벡터-스칼라 양방향 곱셈
let v = Vec3::X * 2.0;   // Vec3 * f64
let v = 2.0 * Vec3::X;   // f64 * Vec3

// 점-벡터 연산
let p = Point3::ORIGIN + Vec3::X;   // Point + Vec → Point
let p = Point3::ORIGIN - Vec3::X;   // Point - Vec → Point
let v = point_a - point_b;          // Point - Point → Vec

// 축약 연산자
let mut v = Vec3::X;
v += Vec3::Y;  // AddAssign
v *= 2.0;      // MulAssign

// 합산
let total: Vec3 = vec![Vec3::X, Vec3::Y].into_iter().sum();
```

**타입 변환**:

```rust
// Vec ↔ Point
let p = Point3::from(Vec3::new(1.0, 2.0, 3.0));
let v = Vec3::from(Point3::new(1.0, 2.0, 3.0));

// 배열/튜플에서 생성
let v = Vec3::from([1.0, 2.0, 3.0]);
let p = Point3::from((1.0, 2.0, 3.0));

// nalgebra 호환
let na_vec = v.to_nalgebra();
let v = Vec3::from_nalgebra(na_vec);
```

**`linalg` 모듈**: `nalgebra::DMatrix`, `DVector`, `LU`를 재수출. sketch 크레이트 등에서 nalgebra에 직접 의존하지 않고 이 모듈을 통해 접근.

---

### 3.3 cadkernel-geometry

**역할**: 매개변수 커브와 서피스의 트레이트 정의 및 구현.

#### Curve 트레이트

```rust
pub trait Curve: Send + Sync {
    fn point_at(&self, t: f64) -> Point3;
    fn tangent_at(&self, t: f64) -> Vec3;
    fn domain(&self) -> (f64, f64);
    fn length(&self) -> f64;
    fn is_closed(&self) -> bool;

    // 기본 구현 (유한차분)
    fn second_derivative_at(&self, t: f64) -> Vec3;
    fn curvature_at(&self, t: f64) -> f64;
    fn reversed(&self) -> Box<dyn Curve>;
    fn project_point(&self, point: Point3) -> f64;
    fn bounding_box(&self) -> BoundingBox;
}
```

#### 구현된 커브

| 타입 | 구조체 | 특이사항 |
|------|--------|----------|
| 직선 | `Line`, `LineSegment` | `Copy`, `PartialEq` |
| 원호 | `Arc` | 시작/끝 각도 |
| 원 | `Circle` | `new()` → `KernelResult<Self>` (영벡터 법선 체크) |
| 타원 | `Ellipse` | `Copy`. Ramanujan 근사 길이 |
| NURBS | `NurbsCurve` | `new()` → `KernelResult<Self>` (knot/weight 검증) |

#### Surface 트레이트

```rust
pub trait Surface: Send + Sync {
    fn point_at(&self, u: f64, v: f64) -> Point3;
    fn normal_at(&self, u: f64, v: f64) -> Vec3;
    fn domain(&self) -> ((f64, f64), (f64, f64));

    // 기본 구현
    fn du(&self, u: f64, v: f64) -> Vec3;
    fn dv(&self, u: f64, v: f64) -> Vec3;
    fn project_point(&self, point: Point3) -> (f64, f64);
    fn bounding_box(&self) -> BoundingBox;
}
```

#### 구현된 서피스

| 타입 | 구조체 | 특이사항 |
|------|--------|----------|
| 평면 | `Plane` | `new()` → `KernelResult`. 편의 생성자 `xy()`, `xz()`, `yz()` |
| 원통 | `Cylinder` | `new()` → `KernelResult` |
| 구 | `Sphere` | 표준 구면좌표 |
| 원뿔 | `Cone` | `Copy`, `PartialEq` |
| 토러스 | `Torus` | `Copy`, `PartialEq` |
| NURBS | `NurbsSurface` | `new()` → `KernelResult` |

#### 교차(Intersect) 모듈

- **Surface-Surface**: Plane-Plane, Plane-Sphere, Plane-Cylinder, Sphere-Sphere
- **Line-Surface**: Line vs Plane, Sphere, Cylinder
- **결과 타입**: `SsiResult` (Empty, Point, Line, Circle, Ellipse, Coincident), `RayHit`
- **이름 규칙**: 교차 결과 타원 = `IntersectionEllipse` (커브 타원 `Ellipse`와 구분)

---

### 3.4 cadkernel-topology

**역할**: B-Rep (Boundary Representation) 반변 자료구조와 Persistent Naming 시스템.

#### Entity 계층

```
Solid ← Shell ← Face ← Loop ← HalfEdge ← Edge ← Vertex
                                                     ↕
                                           Wire (독립 체인)
```

| 엔티티 | 구조체 | 설명 |
|--------|--------|------|
| Vertex | `VertexData` | 3D 점 + 태그 |
| Edge | `EdgeData` | 두 정점 연결. 선택적 `Arc<dyn Curve + Send + Sync>` |
| HalfEdge | `HalfEdgeData` | 방향성 반변. origin, twin, next, prev, edge, loop |
| Loop | `LoopData` | 반변의 순환 리스트. Face의 외곽/내곽(구멍) 경계 |
| Wire | `WireData` | 반변의 순서 체인 (Loop과 독립) |
| Face | `FaceData` | 외곽 루프 + 내곽 루프들. 선택적 `Arc<dyn Surface + Send + Sync>` |
| Shell | `ShellData` | Face의 집합 |
| Solid | `SolidData` | Shell의 집합 |

#### EntityStore<T>

Arena 기반 O(1) insert/remove/lookup 저장소. Generation 카운터로 stale handle 감지.

```rust
let mut store = EntityStore::new();
let h = store.insert(value);       // O(1)
let val = store.get(h);            // O(1), generation 체크
store.remove(h);                   // O(1), generation 증가
store.len();                       // O(1) (alive_count 캐시)
```

#### BRepModel API

```rust
let mut model = BRepModel::new();

// 생성
let v = model.make_vertex(Point3::new(0.0, 0.0, 0.0));
let e = model.add_edge(v1, v2);
let l = model.make_loop(&[he1, he2, he3])?;  // KernelResult
let f = model.make_face(l);
let s = model.make_shell(&[f1, f2, f3]);

// 태그 생성 (Persistent Naming)
let f = model.make_face_tagged(l, tag);
let w = model.make_wire_tagged(hes, true, tag);

// 조회
model.find_vertex_by_tag(&tag);
model.find_face_by_tag(&tag);
model.find_wire_by_tag(&tag);

// 순회
model.loop_half_edges(he);            // → Vec<Handle<HalfEdgeData>>
model.vertices_of_face(face)?;        // → KernelResult<Vec<Handle<VertexData>>>
model.edges_of_face(face)?;
model.faces_of_edge(edge)?;
model.faces_around_vertex(vertex)?;

// 검증 & 변환
model.validate()?;                    // twin 대칭, 루프 순환, 오일러 특성
model.transform(&transform);          // 모든 정점에 어파인 변환 적용
```

#### Persistent Naming

```rust
// Tag = 엔티티 종류 + 히스토리 세그먼트 체인
let tag = Tag::generated(EntityKind::Face, OperationId(1), 0);
let split_tag = tag.split(OperationId(2), 1);
let modified_tag = tag.modified(OperationId(3));

// NameMap: Tag ↔ Handle 양방향 매핑
let mut map = NameMap::new();
map.insert(tag.clone(), EntityRef::Face(face_h));
let found = map.get_face(&tag);       // Option<Handle<FaceData>>
```

---

### 3.5 cadkernel-sketch

**역할**: 2D 파라메트릭 스케치와 Newton-Raphson 기반 제약 솔버.

#### 스케치 요소

```rust
let mut sketch = Sketch::new();
let p0 = sketch.add_point(0.0, 0.0);
let p1 = sketch.add_point(10.0, 0.0);
let l  = sketch.add_line(p0, p1);
let a  = sketch.add_arc(center, start, end);
let c  = sketch.add_circle(center, radius_pt);
```

#### 엔티티 타입 (9개)

Point, Line, Arc, Circle, Ellipse, BSpline, EllipticalArc, HyperbolicArc, ParabolicArc

#### 기하 헬퍼

`add_polyline`, `add_regular_polygon`, `add_arc_3pt`, `add_circle_3pt`, `add_ellipse_3pt`, `add_centered_rectangle`, `add_rounded_rectangle`, `add_arc_slot`

#### 스케치 편집 도구 (`tools.rs`)

| 함수 | 설명 |
|------|------|
| `fillet_sketch_corner` | 코너 필렛 (호 삽입) |
| `chamfer_sketch_corner` | 코너 챔퍼 (직선 삽입) |
| `trim_edge` | 교차점에서 엣지 트리밍 |
| `split_edge` | 지정 점에서 엣지 분할 |
| `extend_edge` | 엣지를 대상까지 연장 |

#### 스케치 유효성 검증 (`validate.rs`)

`validate_sketch` — 7가지 이슈 타입 (열린 프로파일, 중복 점, 길이 0 엣지 등)

#### 보조선 기하

`toggle_construction_mode`, `mark_construction_point`, `mark_construction_line`

#### 24가지 제약 조건

| 제약 | 파라미터 |
|------|----------|
| `Fixed(point, x, y)` | 점을 고정 좌표에 |
| `Horizontal(line)` | 수평 |
| `Vertical(line)` | 수직 |
| `Length(line, length)` | 선분 길이 |
| `Distance(p1, p2, dist)` | 두 점 사이 거리 |
| `Coincident(p1, p2)` | 두 점 일치 |
| `Parallel(l1, l2)` | 두 선 평행 |
| `Perpendicular(l1, l2)` | 두 선 직교 |
| `Equal(l1, l2)` | 두 선 동일 길이 |
| `PointOnLine(point, line)` | 점이 선 위에 |
| `PointOnCircle(point, circle)` | 점이 원 위에 |
| `Symmetric(p1, p2, line)` | 선 대칭 |
| `Angle(l1, l2, angle)` | 두 선 사이 각도 |
| `Radius(circle, radius)` | 원 반지름 |
| `Tangent(line, circle)` | 선-원 접선 |
| `MidPoint(point, l)` | 선의 중점 |
| `Collinear(l1, l2)` | 동일 직선 위 |
| `EqualRadius(c1, c2)` | 동일 반지름 |
| `Concentric(c1, c2)` | 동심원 |
| `Diameter(p, c, d)` | 지름 |
| `Block(p, x, y)` | 위치 잠금 |
| `HorizontalDistance(p1, p2, d)` | 수평 거리 |
| `VerticalDistance(p1, p2, d)` | 수직 거리 |
| `PointOnObject(p, l)` | 객체 위 점 |

#### 솔버

```rust
let result = solve(&mut sketch, max_iter: 200, tolerance: 1e-10);
// SolverResult { converged, iterations, residual }
```

알고리즘: Newton-Raphson + Armijo 백트래킹 (nalgebra DMatrix/DVector 사용).

#### 3D 프로파일 추출

```rust
let wp = WorkPlane::xy();  // 또는 xz(), 커스텀
let profile_3d: Vec<Point3> = extract_profile(&sketch, &wp);
```

---

### 3.6 cadkernel-modeling

**역할**: 기하학적 솔리드 생성 및 변형 연산.

#### 프리미티브 빌더

| 함수 | 반환 | 생성 결과 |
|------|------|----------|
| `make_box(dx, dy, dz)` | `KernelResult<BoxResult>` | 8 vertices, 6 faces |
| `make_cylinder(radius, height, segments)` | `KernelResult<CylinderResult>` | N-gon 상하면 + N 측면 |
| `make_sphere(radius, segments, rings)` | `KernelResult<SphereResult>` | UV 구면 |
| `make_spiral(center, r, growth, turns, tube_r)` | `KernelResult<SpiralResult>` | 아르키메데스 나선 |
| `make_polygon(center, r, sides, height)` | `KernelResult<PolygonResult>` | 정다각형 프리즘 |
| `make_plane_face(origin, w, h)` | `KernelResult<PlaneFaceResult>` | 평면 직사각형 |
| `make_involute_gear(module, teeth, angle, width)` | `KernelResult<GearResult>` | 인볼류트 기어 |

#### Feature Operations

| 함수 | 파라미터 | 반환 |
|------|----------|------|
| `extrude`, `revolve`, `pad`, `pocket`, `groove` | 프로파일 + 방향/축 | `KernelResult<*Result>` |
| `fillet_edge`, `chamfer_edge`, `draft_faces` | 엣지/면 + 파라미터 | `KernelResult<*Result>` |
| `sweep`, `loft`, `mirror_solid`, `scale_solid` | 프로파일/솔리드 + 경로/인수 | `KernelResult<*Result>` |
| `shell_solid`, `split_solid`, `section_solid` | 솔리드 + 파라미터 | `KernelResult<*Result>` |
| `offset_solid`, `thickness_solid` | 솔리드 + 거리/두께 | `KernelResult<*Result>` |
| `linear_pattern`, `circular_pattern` | 솔리드 + 방향/축 + 개수 | `KernelResult<PatternResult>` |
| `hole`, `countersunk_hole` | 솔리드 + 위치/방향/반지름 | `KernelResult<HoleResult>` |

#### 추가적/감산적 연산 (20개, `additive.rs`)

| 추가적 | 감산적 | 설명 |
|--------|--------|------|
| `additive_box` | `subtractive_box` | 박스 |
| `additive_cylinder` | `subtractive_cylinder` | 실린더 |
| `additive_sphere` | `subtractive_sphere` | 구 |
| `additive_cone` | `subtractive_cone` | 원뿔 |
| `additive_torus` | `subtractive_torus` | 토러스 |
| `additive_helix` | `subtractive_helix` | 나선 |
| `additive_ellipsoid` | `subtractive_ellipsoid` | 타원체 |
| `additive_prism` | `subtractive_prism` | 프리즘 |
| `additive_wedge` | `subtractive_wedge` | 웨지 |
| — | `subtractive_loft` | 감산적 로프트 |
| — | `subtractive_pipe` | 감산적 파이프 |

#### 어셈블리

| 구조체/함수 | 설명 |
|------------|------|
| `Assembly` | 컴포넌트 트리 + 구속조건 시스템 |
| `Component` | 솔리드 + 배치 변환 (Mat4) + 가시성 |
| `AssemblyConstraint` | Fixed, Coincident, Concentric, Distance, Angle |
| `JointType` | 13개 조인트 (RackAndPinion, ScrewJoint, BeltJoint 포함) |
| `check_interference()` | 바운딩박스 기반 간섭 검출 |
| `analyze_dof()` | 구속조건/조인트별 자유도(DOF) 분석 |
| `solve()` | 반복 구속조건 솔버 (거리 구속 지원) |
| `rotation()` | 배치 변환 헬퍼 |

#### Draft 연산 (37개, `draft_ops.rs`)

| 함수 | 설명 |
|------|------|
| `make_wire()` | 3D 폴리라인 와이어 |
| `make_bspline_wire()` | B-spline 와이어 |
| `clone_solid()` | 솔리드 깊은 복사 |
| `rectangular_array()` | 2D 그리드 패턴 |
| `path_array()` | 경로를 따른 복사 |
| `make_fillet_wire()` | 필렛 와이어 |
| `make_circle_wire()` | 원형 와이어 |
| `make_arc_wire()` | 호 와이어 |
| `make_ellipse_wire()` | 타원 와이어 |
| `make_rectangle_wire()` | 직사각형 와이어 |
| `make_polygon_wire()` | 다각형 와이어 |
| `make_bezier_wire()` | 베지어 와이어 |
| `make_arc_3pt_wire()` | 3점 호 와이어 |
| `make_chamfer_wire()` | 챔퍼 와이어 |
| `make_point()` | 점 생성 |
| `offset_wire()` | 와이어 오프셋 |
| `join_wires()` | 와이어 결합 |
| `split_wire()` | 와이어 분할 |
| `upgrade_wire()` | 와이어 업그레이드 |
| `downgrade_solid()` | 솔리드 다운그레이드 |
| `wire_to_bspline()` | 와이어→B-spline 변환 |
| `bspline_to_wire()` | B-spline→와이어 변환 |
| `stretch_wire()` | 와이어 늘이기 |
| `move_solid()` | 솔리드 이동 |
| `rotate_solid()` | 솔리드 회전 |
| `scale_solid_draft()` | 솔리드 스케일 (Draft) |
| `mirror_solid_draft()` | 솔리드 미러 (Draft) |
| `polar_array()` | 극좌표 배열 |
| `point_array()` | 점 배열 |
| `make_draft_dimension()` | Draft 치수 생성 |
| `make_label()` | 라벨 생성 |
| `make_dimension_text()` | 치수 텍스트 생성 |
| `snap_to_endpoint()` | 끝점 스냅 |
| `snap_to_midpoint()` | 중점 스냅 |
| `snap_to_nearest()` | 최근접점 스냅 |
| `wire_length()` | 와이어 길이 |
| `wire_area()` | 와이어 면적 |

신규 타입: `DraftDimension`, `DraftLabel`, `SnapResult`, `WireResult`, `BSplineWireResult`, `ArrayResult`, `CloneResult`

#### Surface 연산

| 함수 | 설명 |
|------|------|
| `ruled_surface()` | 두 곡선 사이 선형 보간 서피스 |
| `surface_from_curves()` | 프로파일 곡선 네트워크 서피스 |
| `extend_surface()` | 법선 방향 서피스 확장 |
| `pipe_surface()` | 경로를 따른 관형 솔리드 |
| `filling()` | N면 경계 패치 |
| `sections()` | 프로파일을 통한 서피스 스키닝 |
| `curve_on_mesh()` | 메시 위에 폴리라인 투영 |

#### 결합 연산 (`join.rs`)

| 함수 | 설명 |
|------|------|
| `connect_shapes()` | 형상 연결 |
| `embed_shapes()` | 형상 임베딩 |
| `cutout_shapes()` | 형상 커트아웃 |

#### 컴파운드 연산 (`compound_ops.rs`)

| 함수 | 설명 |
|------|------|
| `boolean_fragments()` | 불리언 프래그먼트 |
| `slice_to_compound()` | 슬라이스 → 컴파운드 |
| `compound_filter()` | 컴파운드 필터 |
| `explode_compound()` | 컴파운드 분해 |

#### 형상 연산 (`face_from_wires.rs`)

| 함수 | 설명 |
|------|------|
| `face_from_wires()` | 와이어로부터 면 생성 |
| `points_from_shape()` | 형상에서 점 추출 |

> 모든 함수는 Persistent Naming 태그를 자동 생성합니다.

#### Boolean Operations

```rust
let result_model = boolean_op(&model_a, solid_a, &model_b, solid_b, BooleanOp::Union)?;
// BooleanOp: Union, Subtract, Intersect
```

파이프라인: Broad Phase (AABB) → Classification (Inside/Outside/Boundary) → Evaluation (결과 모델 구성).

---

### 3.7 cadkernel-io

**역할**: B-Rep 모델의 메시 테셀레이션 및 파일 내보내기.

#### 테셀레이션 & 내보내기

테셀레이션 (`tessellate_solid`, `tessellate_face`), 내보내기 (`write_stl_ascii`, `write_stl_binary`, `write_obj`, `export_*`), TechDraw (`project_solid`, `three_view_drawing`, `section_view`, `detail_view`, `drawing_to_svg`, 6개 기본 치수 타입 + 10개 고급 주석: ArcLengthDimension, ExtentDimension, ChamferDimension, WeldSymbol, BalloonAnnotation, Centerline, BoltCircleCenterlines, CosmeticLine, BreakLine).

#### 메시 연산 (29개)

| 함수 | 설명 |
|------|------|
| `decimate_mesh()` | 엣지 붕괴 메시 간소화 |
| `fill_holes()` | 경계 루프 감지 + 팬 삼각화 |
| `compute_curvature()` | 코탄젠트 가중 평균 곡률 |
| `subdivide_mesh()` | 중점 분할 (1→4 삼각형) |
| `flip_normals()` | 와인딩 반전 + 법선 부정 |
| `smooth_mesh()` | 라플라시안 스무딩 |
| `mesh_boolean_union()` | 메시 불리언 합집합 |
| `mesh_boolean_intersection()` | AABB 필터링 메시 불리언 교집합 |
| `mesh_boolean_difference()` | AABB 필터링 메시 불리언 차집합 |
| `cut_mesh_with_plane()` | 평면 클리핑 |
| `mesh_section_from_plane()` | 단면 윤곽선 추출 |
| `mesh_cross_sections()` | 축 방향 다중 병렬 단면 |
| `split_mesh_by_components()` | 컴포넌트 분리 |
| `harmonize_normals()` | BFS 와인딩 전파 |
| `check_mesh_watertight()` | 수밀성 검사 |
| `regular_solid()` | 5개 정다면체 (정사면체~정이십면체) |
| `face_info()` | 면별 면적, 법선, 무게중심 |
| `bounding_box_info()` | 메시 AABB (중심, 크기, 대각선) |
| `curvature_plot()` | 곡률→RGB 색상 매핑 |
| `add_triangle()` | 단일 삼각형 추가 |
| `unwrap_mesh()` | 주축 투영 UV 언래핑 |
| `unwrap_face()` | 단일 면 UV 좌표 계산 |
| `remove_components_by_size()` | 소규모 컴포넌트 제거 |
| `remove_component()` | 특정 컴포넌트 제거 |
| `trim_mesh()` | 바운딩 박스 기반 메시 트리밍 |
| `segment_mesh()` | 법선 기반 영역 성장 세그먼테이션 |
| `remesh()` | 적응형 엣지 길이 리파인먼트 |
| `evaluate_and_repair()` | 퇴화 제거 + 정점 병합 + 법선 조화 |
| `scale_mesh()` | 축별 메시 스케일링 |

내보내기 타입: `FaceInfo`, `MeshBoundingBox`, `MeshRepairReport`, `MeshSegment`, `RegularSolidType`, `UnwrapResult`, `UvCoord`

### 3.8 cadkernel-viewer

네이티브 데스크톱 GUI 애플리케이션 (egui 0.31 + wgpu 24.x + winit 0.30).

**모듈**: `app.rs` (상태 + 이벤트 루프), `render.rs` (GPU + 카메라 + 수학), `gui/` (12파일 모듈 디렉토리), `nav.rs` (마우스 내비게이션 프리셋).

**GUI 모듈** (`gui/`): `mod.rs` (GuiState, GuiAction enum, draw_ui 진입점), `menu.rs` (File/Edit/Create/View/Tools/Help 메뉴바), `toolbar.rs` (공통 + 9개 워크벤치 툴바), `tree.rs` (계층형 B-Rep 모델 트리), `properties.rs` (엔티티별 속성 패널), `status_bar.rs` (좌표, FPS, 메시 정보), `report.rs` (색상 코드별 Info/Warning/Error 로그 패널), `dialogs.rs` (11개 생성 다이얼로그 + 불리언 + Part 연산), `sketch_ui.rs` (스케치 오버레이 + 그리드 라벨), `overlays.rs` (축 인디케이터 + TechDraw 오버레이), `view_cube.rs` (절두 큐브 내비게이션), `context_menu.rs` (Solid + 뷰포트 우클릭 메뉴).

**리포트 시스템**: `CadApp`의 `log_info()`/`log_warning()`/`log_error()` 헬퍼 메서드. 파일 I/O, 프리미티브 생성, 불리언, Part 연산, Mesh 연산, 분석 핸들러가 `gui.log(ReportLevel, msg)`를 통해 리포트 패널에 기록. 상태바는 최신 메시지를 표시하고, 리포트 패널은 전체 이력을 보존.

**카메라**: 오비트 (yaw/pitch/distance) + 인플레인 롤. 뷰 매트릭스는 0이 아닌 롤 시 롤 회전 적용. screen_right/up 메서드는 롤 인식. 뷰 전환 시 롤은 가장 가까운 90°로 스냅 (45° 중간점에서는 이전 롤 위치 방향으로 `prev_roll` 추적). Top/Bottom 뷰는 현재 yaw 유지 (pitch만 변경). 모든 롤 각도는 `wrap_angle()`로 (−π, π] 범위로 정규화 — `snap_roll_90`은 입력값 정규화, `RollDelta`는 버튼마다 정규화, `ScreenOrbit`는 애니메이션 스냅 이후에 `prev_roll` 저장.

**ViewCube**: 26개 깊이 정렬 폴리곤의 절두 큐브: 6개 팔각형 면, 8개 삼각형 코너, 12개 엣지 베벨 쿼드. 엣지 쿼드는 공유 챔퍼 정점에서 face-normal offset으로 계산 (`EDGE_BEVEL=0.24`). 비-호버 폴리곤은 하나의 `epaint::Mesh`로 합쳐 렌더링 (팬 삼각분할) — egui 안티앨리어싱 페더링 이음새 제거. 불투명 채우기 (`from_rgb`), XYZ 축 인디케이터는 위에 렌더링. 호버된 폴리곤은 `convex_polygon`으로 별도 렌더링 (스트로크 하이라이트). 면/엣지/코너 호버 감지 (point-in-polygon) 및 클릭 스냅 (26개 뷰 방향).

**메시 노말**: 스무스 그룹 BFS 알고리즘 (`render.rs`의 `mesh_to_vertices`). 각 정점에서 크리즈 각도(60°) 내의 면을 BFS로 전이적으로 그룹화. **면적 가중** 누적: 비정규화 외적(크기 ∝ 삼각형 면적)을 합산 후 정규화 — 큰 삼각형이 비례적으로 더 기여하여 불균일 메시 밀도의 아티팩트 제거. 면 간 정점 공유 필수 — `tessellate_solid`에서 bit-exact `f64::to_bits` 매칭, STL 임포트에서 quantize 키(1e4 정밀도)로 정점 중복 제거.

**4x MSAA**: 모든 렌더 파이프라인에 `MultisampleState { count: 4 }` 적용. Scene pass는 MSAA 컬러+뎁스 텍스처에 렌더링 후 surface 텍스처로 리졸브. 마하 밴드 아티팩트 (스무스 서피스의 삼각형 경계선) 제거. egui pass는 sample_count=1 (2D UI, 리졸브된 surface에 직접 렌더링).

**내비게이션**: 5개 프리셋 (FreeCAD Gesture, Blender, SolidWorks, Inventor, OpenCascade). smooth-step 이징 (3t²−2t³) 카메라 애니메이션.

---

## 4. 구현 완료 단계 (Phase 1–4)

### Phase 1: Foundation

> 핵심 커널 아키텍처 구축

| 항목 | 상태 | 내용 |
|------|:----:|------|
| Cargo workspace | ✅ | 7 크레이트 모노레포 |
| Math library | ✅ | Vec, Point, Mat, Transform, Quaternion, Ray, BBox |
| Geometry engine | ✅ | Line, Arc, Circle, NURBS Curve/Surface |
| B-Rep topology | ✅ | Half-edge, EntityStore, Handle<T> |
| CI/CD | ✅ | GitHub Actions: fmt + clippy + test |

### Phase 2: Persistent Naming + Boolean

> 파라메트릭 재구축 기반 + 불리언 연산

| 항목 | 상태 | 내용 |
|------|:----:|------|
| Persistent Naming | ✅ | Tag, NameMap, ShapeHistory, EntityKind, OperationId |
| Boolean Operations | ✅ | Union, Subtract, Intersect (Broad/Classify/Evaluate) |
| Surface-Surface Intersection | ✅ | Plane-Plane, Plane-Sphere, Plane-Cylinder, Sphere-Sphere |
| Line-Surface Intersection | ✅ | Line vs Plane, Sphere, Cylinder |
| Geometry-Topology Binding | ✅ | Edge.curve, Face.surface (feature flag) |

### Phase 3: Parametric + Sketch + I/O

> 2D 스케치 시스템, Feature Operations, 파일 출력

| 항목 | 상태 | 내용 |
|------|:----:|------|
| 2D Sketch system | ✅ | Point, Line, Arc, Circle + 19 constraints |
| Newton-Raphson solver | ✅ | Armijo backtracking, nalgebra 기반 |
| Extrude operation | ✅ | Profile → Solid (auto-tagging) |
| Revolve operation | ✅ | Profile → Solid (N-segment rotation) |
| Primitive builders | ✅ | Box, Cylinder, Sphere |
| STL export | ✅ | ASCII + Binary |
| OBJ export | ✅ | Wavefront OBJ |
| Tessellation | ✅ | Face/Solid → Triangle Mesh |

### Phase 4: Core Hardening

> 안전성, 인체공학, 성능 강화

| 항목 | 상태 | 내용 |
|------|:----:|------|
| `cadkernel-core` 분리 | ✅ | KernelError/KernelResult 공유 타입 독립 크레이트 |
| assert! → Result 변환 | ✅ | 모든 공개 API에서 panic 제거 |
| Send + Sync bounds | ✅ | `Arc<dyn Curve + Send + Sync>` 스레드 안전 |
| Math trait 구현 | ✅ | Default, Display, From, AddAssign, Sum |
| 점-벡터 완전 연산자 | ✅ | Point - Vec, f64 * Vec, From<[f64;N]> |
| EntityStore O(1) len | ✅ | alive_count 캐시 |
| Ellipse 이름 충돌 해소 | ✅ | IntersectionEllipse 분리 |
| PartialEq for geometry | ✅ | 모든 기하 구조체에 PartialEq 추가 |
| NURBS 안전성 | ✅ | empty guard, tangent division-by-zero |
| Topology validation | ✅ | twin 대칭, 루프 순환, 오일러 특성 검사 |
| Wire entity | ✅ | 독립 반변 체인, Naming 시스템 연동 |
| Prelude 모듈 | ✅ | 모든 크레이트 + 루트에 통합 재수출 |

### Phase 5–9: Application (GUI + Workbenches)

egui+wgpu 뷰어, 8개 디스플레이 모드, ViewCube, Part/PartDesign/Sketcher 워크벤치, 인터랙티브 2D 스케치 편집, 피처 구현 (mirror, scale, sweep, loft, shell, pattern).

### Phase 10: TechDraw 워크벤치

직교 투영 (7개 표준 뷰), 은선 제거 (HLR), 3면도, 치수 주석 (선형/각도/반지름), SVG 내보내기, 뷰포트 오버레이.

### Phase 11: NURBS 커널 강화

적응형 커브/서피스 테셀레이션 (`TessellationOptions`), 커브-커브 교차 (분할 + Newton-Raphson), 2D 폴리곤/폴리라인 오프셋 (마이터 조인), 지오메트리 바인딩 헬퍼 (`bind_edge_curve`, `bind_face_surface`), io 크레이트의 NURBS 인식 테셀레이션.

### Phase A: NURBS 커널 완성

FreeCAD 패리티를 위한 NURBS 커널 전면 완성. 해석적 미분 (곡선: 유리 몫 법칙, 서피스: 동차 미분)으로 유한차분 대체. 완전한 knot 연산: 삽입, 정제, 제거, Bezier 분해. 곡선/서피스 피팅: 보간 (A9.1) + 최소자승 근사 (A9.7). 모든 해석적 기하 타입의 NURBS 변환 (Line, Circle, Arc, Ellipse, Plane, Cylinder, Sphere). NurbsCurve/NurbsSurface에 Newton-Raphson `project_point()` 오버라이드. UV 공간 매개변수 곡선 (Curve2D 시스템). TrimmedCurve/TrimmedSurface (트림 경계). 곡선-서피스/서피스-서피스 교차 (예측자-보정자 마칭). 볼록 껍질 속성 기반 바운딩 박스 오버라이드.

**주요 추가 모듈**: `bspline_basis.rs`, `curve/to_nurbs.rs`, `curve/curve2d.rs`, `curve/trimmed.rs`, `curve/nurbs_fitting.rs`, `surface/to_nurbs.rs`, `surface/trimmed.rs`, `surface/nurbs_fitting.rs`, `intersect/curve_surface.rs`, `intersect/surface_surface.rs`.

### Phase B–E: 트림 B-Rep + STEP + 고급 프리미티브
5개 프리미티브 지오메트리 바인딩, ParametricWire2D, 트림 테셀레이션, 완전한 STEP I/O, fillet/draft/split, 5개 새 프리미티브 (tube, prism, wedge, ellipsoid, helix).

### Phase F–K: Part 고급 + TechDraw + Assembly + Sketcher
Section/offset/thickness 연산, TechDraw 단면/상세 뷰, 어셈블리 모듈 (구속조건 + 간섭 검출), 5개 새 스케치 구속조건, PartDesign 피처 (pad/pocket/groove/hole).

### Phase L–O: Draft + Mesh + Surface 워크벤치
Draft 연산 (37개 함수: 와이어 생성/조작, 솔리드 변환, 배열, 주석, 스냅, 쿼리), 메시 연산 (decimate, fill holes, curvature, subdivide, flip normals), 서피스 연산 (ruled surface, surface from curves, extend, pipe surface).

### Phase N–P: FEM + IGES
FEM 모듈 (TetMesh 생성, Gauss-Seidel 정적 해석, von Mises 응력, 모달 해석, 열 해석, 메시 품질), 6개 재료 프리셋 (강철, 알루미늄, 티타늄, 구리, 콘크리트, 주철), 열 재료, 8개 경계조건 타입 (구조 4개 + 열 4개), 후처리 (응력/변형률 텐서, 주응력, 안전 계수, 변형 에너지, 반력), 메시 유틸리티 (리파인, 표면 추출, 노드 병합). IGES I/O (80열 고정 포맷, Point/Line/Arc/NURBS 엔터티).

### Phase Q–S: 성능 최적화 + 지오메트리 확장 + 모델링 확장
BVH 공간 인덱스, 병렬 테셀레이션, 등매개변수 곡선, 서피스 곡률, 오프셋/회전/돌출 서피스, 블렌드 곡선, 서피스 연속성 분석, 나선, 다각형, 평면 면, Boolean XOR, Compound, 기하/수밀 검사, multi_transform, Body, 인볼류트 기어.

### Phase T–U: 스케처 확장 + 파일 포맷 + 메시 연산
5개 새 구속 타입 (Diameter, Block, HorizontalDistance, VerticalDistance, PointOnObject), Ellipse/BSpline 엔티티, polyline/polygon/arc_3pt 헬퍼. DXF/PLY/3MF/BREP I/O, 7개 메시 연산 (smooth, boolean, cut, section, split, harmonize, watertight check), TechDraw 치수.

### Phase V1: 스케처 완성
3개 원뿔 곡선 호 엔티티 (EllipticalArc, HyperbolicArc, ParabolicArc), `tools.rs`에 5개 스케치 편집 도구 (fillet/chamfer corner, trim/split/extend edge), `validate.rs` 스케치 유효성 검증 모듈 (7가지 이슈 타입), 보조선 기하 지원, 5개 기하 헬퍼 (circle_3pt, ellipse_3pt, centered_rectangle, rounded_rectangle, arc_slot).

### Phase V2: PartDesign 완성
`additive.rs`에 8개 새 추가적/감산적 프리미티브 쌍 (helix, ellipsoid, prism, wedge), 2개 새 감산적 연산 (loft, pipe). 총 추가적/감산적 연산 10개 → 20개로 확장.

### Phase V3: Part 워크벤치 완성
`join.rs`에 결합 연산 (connect_shapes, embed_shapes, cutout_shapes), `compound_ops.rs`에 컴파운드 연산 (boolean_fragments, slice_to_compound, compound_filter, explode_compound), `face_from_wires.rs`에 형상 연산 (face_from_wires, points_from_shape).

### Phase V4: TechDraw 확장
10개 신규 주석 타입: ArcLengthDimension, ExtentDimension, ChamferDimension, WeldSymbol (6개 용접 타입), BalloonAnnotation, Centerline, BoltCircleCenterlines, CosmeticLine (4개 스타일), BreakLine. 모든 타입에 SVG 렌더링.

### Phase V5: 어셈블리 솔버
DOF 분석 (`analyze_dof()`) 구속조건/조인트별 자유도 카운팅, 반복 구속조건 솔버 (`solve()`) 거리 구속 지원, 3개 신규 조인트 (RackAndPinion, ScrewJoint, BeltJoint, 총 13개), `rotation()` 배치 헬퍼.

### Phase V6: Surface 워크벤치 완성
`filling()` (N면 경계 패치), `sections()` (프로파일 스키닝), `curve_on_mesh()` (메시 위 폴리라인 투영).

### Phase V8: 메시 완성
`mesh_ops.rs`에 17개 신규 메시 연산: `mesh_boolean_intersection`, `mesh_boolean_difference`, `regular_solid` (5개 정다면체), `face_info`, `bounding_box_info`, `curvature_plot`, `add_triangle`, `unwrap_mesh`, `unwrap_face`, `remove_components_by_size`, `remove_component`, `trim_mesh`, `mesh_cross_sections`, `segment_mesh`, `remesh`, `evaluate_and_repair`, `scale_mesh`. 신규 타입: `FaceInfo`, `MeshBoundingBox`, `MeshRepairReport`, `MeshSegment`, `RegularSolidType`, `UnwrapResult`, `UvCoord`.

### Phase V9: Draft 워크벤치 완성
`draft_ops.rs`에 37개 Draft 연산 (32개 신규 + 기존 5개). 와이어 생성 (fillet, circle, arc, ellipse, rectangle, polygon, bezier, arc_3pt, chamfer, point), 와이어 조작 (offset, join, split, upgrade, downgrade, to/from bspline, stretch), 솔리드 변환 (move, rotate, scale, mirror), 배열 패턴 (polar, point), 주석 (dimension, label, text), 스냅 (endpoint, midpoint, nearest), 쿼리 (length, area). 신규 타입: `DraftDimension`, `DraftLabel`, `SnapResult`, `WireResult`, `BSplineWireResult`, `ArrayResult`, `CloneResult`.

### Phase V10: FEM 워크벤치 확장
6개 신규 재료 프리셋 (`FemMaterial::titanium/copper/concrete/cast_iron/custom`, `ThermalMaterial` steel/aluminum/copper). 8개 신규 FEM 타입 (`ThermalMaterial`, `ThermalBoundaryCondition`, `ThermalResult`, `BeamSection`, `ModalResult`, `MeshQuality`, `PrincipalStresses`, `StrainResult`, `StressTensor`). 4개 구조 경계조건 (Displacement, Gravity, DistributedLoad, Spring) + 4개 열 경계조건 (FixedTemperature, HeatFlux, HeatGeneration, Convection). 3개 신규 해석 함수: `modal_analysis()` (역멱법 고유진동수), `thermal_analysis()` (정상 상태 열전도, Gauss-Seidel), `mesh_quality()` (종횡비, 체적, 퇴화 검출). 3개 신규 메시 함수: `refine_tet_mesh()` (1→8 분할), `extract_surface_mesh()` (경계면), `merge_coincident_nodes()` (허용 오차 중복 제거). 5개 후처리 함수: `compute_stress_tensor()`, `compute_strain_tensor()`, `principal_stresses()` (Cardano 고유값), `safety_factor()`, `strain_energy()`, `compute_reactions()`.

### Phase V11: 뷰어 UI 확장
파일 메뉴에 STEP/IGES/DXF/PLY/3MF/BREP Import/Export 추가. 불리언 연산 다이얼로그 (두 번째 박스 프리미티브로 Union/Subtract/Intersect). Part 연산 (Mirror XY/XZ/YZ, Scale, Shell, Fillet, Chamfer, Linear Pattern). Mesh 툴바 (Smooth, Harmonize Normals, Check Watertight, Remesh, Repair). 분석 도구 (Measure Solid 체적/면적/무게중심, Check Geometry 유효성). PartDesign 툴바 전체 백엔드 연결. ~20개 신규 `GuiAction` 변형 + 전체 `process_actions()` 핸들러. 미사용 스텁 제거 (BooleanUnion/Subtract/Intersect, TrimDemo).

### Phase V13: 성능 & 검증
BVH 가속 불리언 broad-phase (O(n log n) 면 쌍 중첩), 11개 신규 벤치마크 (총 25개): 프리미티브 (cone, torus), 피처 (mirror, scale, fillet), 검증 (check_geometry, check_watertight), 스트레스 (tessellate_sphere_64x32, tessellate_torus_64x32, boolean_intersection).

### Phase V12: Python 바인딩
PyO3 기반 `cadkernel` Python 모듈 (`crates/python/`, 독립 빌드, workspace 제외). 6개 클래스 (Model, SolidHandle, Mesh, MassProperties, GeometryCheck, Sketch), 10개 프리미티브 생성기, 4개 피처 연산, 3개 불리언 연산, 테셀레이션/분석, 10개 I/O 함수, 7개 제약조건 타입의 스케치 시스템.

### FreeCAD 수준 UI 대개편
`gui.rs` (3605줄) → `gui/` 모듈 디렉토리 리팩토링 (12개 파일). 계층형 모델 트리, 속성 편집기, 완전한 메뉴 시스템, 향상된 상태바, 색상 코드별 로깅 리포트 패널, Solid/뷰포트 컨텍스트 메뉴, 9개 워크벤치 툴바 (Part, PartDesign, Sketcher, Mesh, TechDraw, Assembly, Draft, Surface, FEM). 40+ 액션 핸들러에 리포트 로깅 추가.

**현재 상태**: 739 tests, 0 clippy warnings, 0 fmt diff.

---

## 5. API 설계 원칙

### 1. 실패 가능한 작업은 `KernelResult<T>` 반환

```rust
// Good
pub fn new(axis: Vec3) -> KernelResult<Self> {
    let n = axis.normalized()
        .ok_or_else(|| KernelError::InvalidArgument("axis must be non-zero".into()))?;
    Ok(Self { axis: n })
}

// Bad — panic은 테스트에서만
pub fn new(axis: Vec3) -> Self {
    let n = axis.normalized().expect("non-zero");
    Self { axis: n }
}
```

### 2. 슬라이스 우선, 소유권 이전은 필요할 때만

```rust
// Good — 호출자가 복사 비용 결정
pub fn make_shell(&mut self, faces: &[Handle<FaceData>]) -> Handle<ShellData>

// Avoid — 불필요한 소유권 이전
pub fn make_shell(&mut self, faces: Vec<Handle<FaceData>>) -> Handle<ShellData>
```

### 3. 작은 타입은 `Copy`, 큰 타입은 `Clone`

모든 math 타입(`Vec3`, `Point3`, `Transform`, `Quaternion`, `BoundingBox`)은 `Copy`.
`BRepModel`, `NurbsCurve` 같은 힙 타입은 `Clone`.

### 4. `Display`로 사용자 친화적 출력

```rust
println!("{}", Vec3::new(1.0, 2.0, 3.0));  // "(1, 2, 3)"
println!("{}", Point3::ORIGIN);              // "(0, 0, 0)"
println!("{}", Ray3::new(origin, dir));      // "Ray3((0, 0, 0) -> (1, 0, 0))"
```

---

## 6. 에러 처리 패턴

### 크레이트별 에러 매핑

```rust
// 하위 크레이트 에러는 KernelError variant로 통일
KernelError::InvalidHandle("face")     // topology
KernelError::InvalidArgument("...")    // geometry 생성자
KernelError::ValidationFailed("...")   // B-Rep 검증
KernelError::GeometryError("...")      // 기하 연산 실패
KernelError::IoError("...")            // 파일 I/O
```

### `?` 연산자 체이닝

```rust
pub fn vertices_of_face(&self, face: Handle<FaceData>) -> KernelResult<Vec<Handle<VertexData>>> {
    let fd = self.faces.get(face).ok_or(KernelError::InvalidHandle("face"))?;
    let ld = self.loops.get(fd.outer_loop).ok_or(KernelError::InvalidHandle("loop"))?;
    // ...
}
```

### 내부 handle은 `unwrap()` 허용

방금 `insert()`한 handle은 반드시 유효하므로 `unwrap()` 사용 가능:

```rust
let h = self.vertices.insert(data);
self.vertices.get_mut(h).unwrap().tag = Some(tag);  // OK: h는 방금 생성
```

---

## 7. 테스트 전략

### 계층별 테스트

| 계층 | 유형 | 위치 | 설명 |
|------|------|------|------|
| 단위 | `#[test]` | 각 크레이트 `src/*.rs` | 개별 함수/구조체 |
| 통합 | `#[test]` | `cadkernel/src/lib.rs` | E2E 파이프라인 |
| Doc | `///` + ` ``` ` | prelude 모듈 | API 사용 예제 |

### 실행 방법

```bash
cargo test --workspace                                       # 전체
cargo test -p cadkernel-math                                 # 특정 크레이트
cargo test --workspace -- --test-threads=1                   # 순차 실행
cargo clippy --workspace --all-targets --all-features -- -D warnings  # Lint
cargo fmt --all -- --check                                   # 포맷 검사
```

### 현재 테스트 현황

| 크레이트 | 테스트 수 |
|----------|:---------:|
| cadkernel (E2E + banner) | 4 |
| cadkernel-core | 1 |
| cadkernel-math | 44 |
| cadkernel-geometry | 138 |
| cadkernel-topology | 29 |
| cadkernel-modeling | 49 |
| cadkernel-sketch | 10 |
| cadkernel-io | 34 |
| Doctests | 3 |
| **합계** | **320** |

---

## 8. 빌드 및 CI

### 요구사항

- **Rust**: 1.85+ (Edition 2024)
- **외부 의존성**: `nalgebra` 0.33, `glam` 0.29

### GitHub Actions CI

```yaml
# .github/workflows/ci.yml
jobs:
  check:
    steps:
      - cargo fmt --all -- --check
      - cargo clippy --all-targets --all-features -- -D warnings
      - cargo test --all-targets --all-features --locked
```

### 로컬 개발 워크플로우

```bash
# 1. 코드 수정
# 2. 포맷 + 검사 + 테스트 (한 줄로)
cargo fmt --all && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace
```

---

## 9. 워크벤치 툴바 아키텍처

뷰어는 FreeCAD 스타일의 워크벤치 시스템을 사용합니다:

- **`Workbench` 열거형**: `Part`, `PartDesign`, `Sketcher`, `Mesh`, `TechDraw`, `Assembly`
- **공통 툴바**: New, Open, Save, Undo, Redo, Fit All, Reset View
- **워크벤치 탭바**: 활성 워크벤치 컨텍스트 전환
- **컨텍스트 툴바**: 활성 워크벤치에 따라 변경
  - Part: 10개 프리미티브, Boolean Union/Subtract/Intersect (두 번째 박스 다이얼로그), Mirror/Scale/Shell/Fillet/Chamfer/Linear Pattern, Measure/Check
  - Part Design: Pad/Pocket/Revolve/Fillet/Chamfer/Draft/Shell/Mirror/Scale/Pattern/Union/Subtract
  - Sketcher: 인터랙티브 모드 — Line, Rectangle, Circle, Arc 도구, 제약조건 (H/V/Length), Close→Extrude
  - Mesh: STL/OBJ/glTF Import/Export, Decimate/Subdivide/Fill Holes/Flip Normals, Smooth/Harmonize Normals/Check Watertight/Remesh/Repair
  - TechDraw: Front/Top/Right/Iso, 3-View, Export SVG, Clear
  - Assembly: Insert Component, Fixed, Coincident, Concentric, Distance
- **파일 메뉴**: STEP, IGES, DXF, PLY, 3MF, BREP, STL, OBJ, glTF, CADK Import/Export
- **다이얼로그 패턴**: `GuiAction` 열거형 → `gui.actions` 벡터 → `app.rs`의 `process_actions()`
- **불리언 다이얼로그**: 두 번째 박스 크기/오프셋용 DragValue가 있는 플로팅 `egui::Window`

## 10. 지오메트리 바인딩 (Phase B05)

5개 프리미티브 모두 이상적 지오메트리를 토폴로지에 바인딩:

| 프리미티브 | 면 서피스 | 엣지 커브 |
|-----------|----------|----------|
| **Box** | 6 `Plane` (면별 하나, u×v = 외향 법선) | 12 `LineSegment` |
| **Cylinder** | 2 `Plane` (캡) + 1 공유 `Cylinder` (측면) | 3N `LineSegment` |
| **Sphere** | 1 공유 `Sphere` (전체 면) | `LineSegment` (다각형 근사) |
| **Cone** | `Plane` (하단) + 선택적 `Plane` (상단, 절두체) + `Cone` (측면) | `LineSegment` |
| **Torus** | 1 공유 `Torus` (전체 면) | `LineSegment` |

핵심 인프라:
- **`EdgeCache`**: 엣지 중복 제거용 `Handle<EdgeData>` 추적 + `all_edges()` 검색
- **`bind_edge_line_segments()`**: 캐시된 모든 엣지에 `LineSegment` 커브 바인딩하는 공용 헬퍼
- **`bind_face_surface(face, Arc<dyn Surface>, Orientation)`**: 면에 이상적 서피스 연결
- **`bind_edge_curve(edge, Arc<dyn Curve>, domain)`**: 엣지에 이상적 커브 연결

## 11. 트림 인프라 (Phase B01–B04)

UV 매개변수 공간 트림 경계를 통한 정밀 B-Rep 면 표현.

| 구성 요소 | 위치 | 설명 |
|-----------|------|------|
| **ParametricWire2D** | `geometry/surface/parametric_wire.rs` | 닫힌 2D 곡선 체인 (와인딩 넘버 포함 판정, 호 길이 샘플링, 폴리라인) |
| **FaceData 트림** | `topology/face.rs` | `outer_trim: Option<ParametricWire2D>`, `inner_trims: Vec<ParametricWire2D>` |
| **EdgeData pcurve** | `topology/edge.rs` | `pcurve_left/right: Option<Arc<dyn Curve2D>>` — 인접 면별 UV 표현 |
| **bind_face_trim()** | `topology/lib.rs` | 면에 트림 와이어 바인딩 |
| **bind_edge_pcurve()** | `topology/lib.rs` | 엣지 측면에 UV pcurve 바인딩 |
| **트림 테셀레이션** | `io/tessellate.rs` | UV 중심점 필터링 — outer_trim 밖이거나 홀 내부인 삼각형 제외 |
| **Trim Demo** | `viewer/app.rs` | Part 워크벤치 액션 — 상단면에 원형 홀 트림된 박스 |

## 12. 정확한 불리언 연산 (Phase B06-B14)

불리언 모듈 (`crates/modeling/src/boolean/`)에 면 분할을 통한 정확한 불리언 연산이 추가되었습니다.

### 모듈 구조

| 모듈 | 목적 |
|------|------|
| `face_split.rs` | SSI 교차 곡선을 따른 면 분할 |
| `trim_validate.rs` | 트림 루프 유효성 검증 (감김, 폐합, 포함) |
| `evaluate.rs` | 면 분류 기반 불리언 연산 |
| `broad_phase.rs` | AABB 겹침 탐지 |
| `classify.rs` | 레이 캐스팅을 통한 면 내/외부 분류 |

### 주요 함수

- `boolean_op_exact()` — 면 분할 전처리를 포함한 불리언 연산
- `split_solids_at_intersection()` — SSI 곡선을 따른 면 분할
- `fit_ssi_to_nurbs()` — SSI 점 구름을 NURBS 곡선으로 피팅
- `fit_ssi_to_pcurve()` — UV 파라미터를 2D pcurve로 피팅
- `validate_trim()` — 트림 루프 일관성 검증
- `ensure_correct_winding()` — 감김 방향 수정 (외곽=CCW, 홀=CW)

### 알고리즘 파이프라인

1. **광역 단계**: AABB 교차를 통해 겹치는 면 쌍 탐색
2. **SSI 계산**: 서피스 바인딩이 있는 겹치는 쌍에 대해 마칭으로 교차 곡선 계산
3. **평면 교차**: 서피스 바인딩 없는 면에 대해 엣지-평면 교차 계산
4. **곡선 피팅**: SSI 점 구름을 NURBS 곡선으로 피팅 (보간 또는 근사)
5. **면 분할**: 교차 곡선을 면 경계에 클리핑, 진입/이탈 점에서 폴리곤 분할
6. **분류**: 분할된 하위 면을 상대 솔리드의 내/외부로 분류
7. **조립**: 연산 유형(합/교/차)에 따라 적절한 면 선택

### 지오메트리 보존

`copy_face_with_geometry()`가 보존하는 항목:
- 서피스 바인딩 (면 ↔ 파라메트릭 서피스)
- 엣지 곡선 바인딩 (엣지 ↔ 3D 곡선)
- 트림 루프 (외곽 + 내부 UV 경계)
- 영구 태그

## 13. 다음 단계

| Phase | 초점 | 주요 항목 |
|-------|------|-----------|
| B06–B14 | 정밀 B-Rep | SSI를 통한 면 분할, 불리언 정밀 교차, 트림 루프 검증 |
| C | STEP I/O | AP203/AP214 import/export |
| D | 필렛/드래프트/분할 | 롤링볼 필렛, 면 드래프트, 솔리드 분할 |

---

## 14. 용어 사전

| 용어 | 설명 |
|------|------|
| **B-Rep** | Boundary Representation. 솔리드를 경계 면으로 표현 |
| **Half-Edge** | 방향성 있는 변. 각 edge는 두 개의 half-edge (twin 관계) |
| **Handle<T>** | 엔티티 참조. index + generation으로 stale 감지 |
| **Tag** | Persistent name. 파라메트릭 재구축 시 엔티티 추적용 |
| **EntityKind** | Vertex, Edge, HalfEdge, Loop, Wire, Face, Shell, Solid |
| **NURBS** | Non-Uniform Rational B-Spline. 자유곡선/면 표현 |
| **TNP** | Topology Naming Problem. 모델 재구축 시 이름 안정성 문제 |
| **AABB** | Axis-Aligned Bounding Box |
| **SSI** | Surface-Surface Intersection |
| **Euler characteristic** | V - E + F = 2 (닫힌 다면체) |
| **Feature flag** | Cargo feature. 조건부 컴파일 |
| **Armijo backtracking** | 선탐색 알고리즘. Newton-Raphson 수렴 보장 |
