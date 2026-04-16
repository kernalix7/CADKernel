# CADKernel Developer Wiki

> **버전**: 0.1.0 (pre-alpha)  
> **최종 업데이트**: 2026-04-13
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
- [15. 확장 아키텍처](#15-확장-아키텍처)
- [16. MCP 통합](#16-mcp-통합)
- [17. 스크립팅](#17-스크립팅)
- [18. CI/CD](#18-cicd)
- [19. Lua 콘솔](#19-lua-콘솔)
- [20. 프로젝트 템플릿](#20-프로젝트-템플릿)
- [21. 편의 API](#21-편의-api)
- [22. 예제 스크립트](#22-예제-스크립트)

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

**GUI 모듈** (`gui/`): `mod.rs` (GuiState, 130개 이상 변형의 GuiAction 열거형, draw_ui 진입점), `menu.rs` (File/Edit/Create/View/Tools/Help 메뉴바), `toolbar.rs` (공통 + 9개 워크벤치 툴바, 전체 백엔드 연결), `tree.rs` (EntityIcon, 검색/필터, 인라인 이름 변경, 가시성 토글, 팁 마커를 포함한 계층형 모델 트리), `properties.rs` (파라메트릭 편집, 변환 제어, 색상 선택기가 있는 Data/View 탭), `status_bar.rs` (마우스 좌표, DOF 상태, 워크벤치/선택 모드, 씬 통계, FPS), `report.rs` (심각도 필터링과 카운트 배지가 있는 Report/Console 탭), `dialogs.rs` (28개 생성 다이얼로그 + 불리언 + Part 연산), `sketch_ui.rs` (스케치 오버레이 + 그리드 + 스냅 인디케이터), `overlays.rs` (원점 축 + 3D 그리드 + 측정 + 스냅 하이라이트), `view_cube.rs` (절두 큐브 내비게이션), `context_menu.rs` (워크벤치별 연산이 포함된 오브젝트/뷰포트/트리/면-엣지 컨텍스트 메뉴), `task_panel.rs` (35개 ActiveTask 변형: 프리미티브 10 + PartDesign 11 + Draft 6 + Surface 2 + FEM 1 + Boolean 1 + Scale 1, 각각 실시간 3D 프리뷰), `theme.rs` (Dark/Light + 3가지 밀도 모드의 CadTheme).

**액션 처리** (`app.rs`): `process_actions()`가 130개 이상의 모든 `GuiAction` 변형을 처리. 9개 워크벤치 전체에 대한 완전한 백엔드 통합: Part (join/compound/shape 연산), PartDesign (pad/pocket/groove/hole + additive/subtractive), Sketcher (구속 + 도구), Mesh (15개 연산 + 분석), TechDraw (뷰/치수/중심선/장식), Assembly (13개 조인트 + 솔버 + 시뮬레이션), Draft (와이어 생성/수정/배열), Surface (7개 연산), FEM (메시 생성 + 6가지 해석 타입 + 9개 방정식 + 후처리). 모든 핸들러에 리포트 패널 로깅 적용.

**리포트 시스템**: `CadApp`의 `log_info()`/`log_warning()`/`log_error()` 헬퍼 메서드. 130개 이상의 모든 액션 핸들러가 `gui.log(ReportLevel, msg)`를 통해 리포트 패널에 기록. 상태바는 최신 메시지를 표시하고, 리포트 패널은 심각도 필터링과 함께 전체 이력을 보존.

**카메라**: 오비트 (yaw/pitch/distance) + 인플레인 롤. 뷰 매트릭스는 0이 아닌 롤 시 롤 회전 적용. screen_right/up 메서드는 롤 인식. 뷰 전환 시 롤은 가장 가까운 90°로 스냅 (45° 중간점에서는 이전 롤 위치 방향으로 `prev_roll` 추적). Top/Bottom 뷰는 현재 yaw 유지 (pitch만 변경). 모든 롤 각도는 `wrap_angle()`로 (−π, π] 범위로 정규화 — `snap_roll_90`은 입력값 정규화, `RollDelta`는 버튼마다 정규화, `ScreenOrbit`는 애니메이션 스냅 이후에 `prev_roll` 저장.

**ViewCube**: 26개 깊이 정렬 폴리곤의 절두 큐브: 6개 팔각형 면, 8개 삼각형 코너, 12개 엣지 베벨 쿼드. 엣지 쿼드는 공유 챔퍼 정점에서 face-normal offset으로 계산 (`EDGE_BEVEL=0.24`). 비-호버 폴리곤은 하나의 `epaint::Mesh`로 합쳐 렌더링 (팬 삼각분할) — egui 안티앨리어싱 페더링 이음새 제거. 불투명 채우기 (`from_rgb`), XYZ 축 인디케이터는 위에 렌더링. 호버된 폴리곤은 `convex_polygon`으로 별도 렌더링 (스트로크 하이라이트). 면/엣지/코너 호버 감지 (point-in-polygon) 및 클릭 스냅 (26개 뷰 방향).

**메시 노말**: 스무스 그룹 BFS 알고리즘 (`render.rs`의 `mesh_to_vertices`). 각 정점에서 크리즈 각도(60°) 내의 면을 BFS로 전이적으로 그룹화. **면적 가중** 누적: 비정규화 외적(크기 ∝ 삼각형 면적)을 합산 후 정규화 — 큰 삼각형이 비례적으로 더 기여하여 불균일 메시 밀도의 아티팩트 제거. 면 간 정점 공유 필수 — `tessellate_solid`에서 bit-exact `f64::to_bits` 매칭, STL 임포트에서 quantize 키(1e4 정밀도)로 정점 중복 제거.

**4x MSAA**: 모든 렌더 파이프라인에 `MultisampleState { count: 4 }` 적용. Scene pass는 MSAA 컬러+뎁스 텍스처에 렌더링 후 surface 텍스처로 리졸브. 마하 밴드 아티팩트 (스무스 서피스의 삼각형 경계선) 제거. egui pass는 sample_count=1 (2D UI, 리졸브된 surface에 직접 렌더링).

**컴포지트 알파**: Surface 구성에 `wgpu::CompositeAlphaMode::Opaque` 사용 — Linux 컴포지터 블렌딩 아티팩트 (3D 뷰포트가 egui 패널을 투과하는 현상) 방지.

**내비게이션**: FreeCAD 12개 스타일 완전 매핑 (CAD/Gesture/Blender/Maya/SolidWorks/OpenInventor/OpenCascade/OpenSCAD/Revit/SiemensNX/TinkerCAD/Touchpad). smooth-step 이징 (3t²−2t³) 카메라 애니메이션. `OrbitStyle` (5종): Turntable (pitch 클램핑), FreeTurntable (미클램핑), Trackball/TrackballClassic/RoundedArcball (가상 구체 매핑). `RotationMode` (3종): WindowCenter (타겟 중심 궤도), ObjectCenter (선택 객체 무게중심), DragAtCursor (화면 공간 피벗). `zoom_at_cursor`: 스크롤 시 커서 방향으로 타겟 이동. `zoom_step`: 이산 스크롤 줌량 (기본 20%).

**설정 다이얼로그**: 6개 탭 구성의 환경 설정 다이얼로그 (일반, 디스플레이, 내비게이션, 외관, 조명, 단축키). 일반: 단위 시스템 (`UnitSystem` 열거형), 소수점 자릿수, 자동 저장, 최근 파일 제한, 삭제 확인. 디스플레이: 배경 그라디언트 프리셋 (`BgPreset` 열거형 4가지 변형), 뷰포트 오버레이, 선택/사전선택 색상, 테셀레이션 품질. 내비게이션: 마우스 프리셋, 감도, 애니메이션, View Cube. 외관: 테마, 밀도. 조명: 활성화, 강도, 방향. 사이드바에 "전체 초기화".

**동적 배경 그라디언트**: `nav.rs`의 `BgPreset` 열거형 (Dark/Medium/Light/Blueprint/Custom). `GpuState::update_bg()`가 색상 변경 시 런타임에 배경 셰이더 파이프라인을 재생성하여 재시작 없이 실시간 그라디언트 전환 가능. Custom 프리셋은 `bg_custom_top`/`bg_custom_bottom` 색상 필드를 사용하며 Display 설정에서 색상 선택기 제공.

**단면 평면**: NavConfig의 `clip_enabled`, `clip_plane_normal`, `clip_plane_offset`. WGSL 셰이더가 `uniforms.clip_params`를 사용하여 평면 너머 프래그먼트를 폐기. Shift+S 또는 View 메뉴로 토글. Display 설정 탭에서 축 선택기 (X/Y/Z)와 오프셋 DragValue 제공. 비활성화 시 `CLIP_DISABLED` 센티널 `[0,0,0,1000]`이 모든 프래그먼트 통과.

**ViewCube 드래그 회전**: ViewCube 위에서 드래그하면 `ScreenOrbit`으로 카메라 연속 궤도 회전. 움직임 없는 클릭은 면/엣지/코너 스냅 동작 유지. GuiState의 `cube_dragging`/`cube_drag_moved`로 상태 추적.

**변환 기즈모**: 선택된 오브젝트 중심에 인터랙티브 이동/회전/스케일 기즈모 표시. `GizmoMode` 열거형 (None/Translate/Rotate/Scale), W/E/R 키로 전환. 호버 감지: `point_to_segment_dist()`로 커서-축 화살표 거리 측정; 하이라이트 축 두껍게 표시. 드래그: 마우스 델타를 화면 공간 축 방향에 투영하여 월드 공간 변환으로 변환. `camera.distance`에 비례하는 속도로 줌 레벨과 무관한 일관된 조작감.

**3D 그리드 스냅**: NavConfig의 `snap_to_grid_3d`, `snap_3d()` 유틸리티가 가장 가까운 `grid_3d_spacing`으로 반올림. MoveObject 액션 핸들러에서 적용. Display 설정에서 토글.

**실행 취소/다시 실행 기록 패널**: 하단 패널의 "History" 탭 (Report, Console과 나란히). `CommandStack::entries()`가 (history, future) 설명 목록 반환. 매 프레임 egui 그리기 전 history 채움. 번호 매긴 작업 목록, 현재 위치 마커, 흐릿한 redo 항목, Undo/Redo 버튼 표시.

**재질 프리셋, Undo 히스토리 드롭다운 & 익스포트 옵션 (V45)**: Properties View 탭에 16개 재질 프리셋 — Steel, Aluminum, Brass, Copper, Gold, Titanium, Cast Iron, 4 Plastics, Glass(알파0.35), 2 Woods, Rubber, Carbon Fiber. 2열 그리드(아이콘+이름 버튼 + 색상 스와치), 색상 근접도로 현재 재질 자동 감지. Undo/Redo 툴바에 드롭다운 화살표(▾) — 최대 10개 히스토리 항목 + 단계번호, 클릭으로 다단계 undo/redo. STL Export Options 다이얼로그: Binary/ASCII 포맷 라디오 + 스케일 팩터 DragValue. `ExportStlWithOptions` 액션으로 버텍스 스케일링 적용 후 `export_stl_binary/ascii` 호출. 메뉴 STL 익스포트 시 옵션 다이얼로그 먼저 표시.

**브레드크럼 바, 최근 파일 & 토스트 알림 (V44)**: `draw_breadcrumb_bar()`로 컨텍스트 툴바~뷰포트 사이에 TopBottomPanel 브레드크럼 네비게이션 바 추가. Scene › ObjectName › Face/Edge/Vertex/Solid 경로 표시, 스케치 모드 시 Sketch 추가. Scene 루트 클릭 시 DeselectAll. 다중 선택 수 우측 악센트 블루. 셰브론 구분자, 활성=흰색, 비활성=연회색. File 메뉴에 Recent Files 서브메뉴 — 최대 10파일(파일명 + 경로 툴팁), ClearRecentFiles 액션. 열기/임포트 시 `recent_files` 추가(중복 제거, 최대 10). 토스트 알림: `Toast` + `ToastLevel`(Success/Info/Warning/Error), `draw_toast_overlay()` 우하단 플로팅 알림. 3초 자동 사라짐 + 페이드인/아웃. 레벨별 악센트 바+배경+아이콘(✓/ℹ/⚠/✖). 최대 5개 스택, 긴 텍스트 말줄임표. `log_info/warning/error`와 StatusMessage 모두 토스트 트리거.

**메뉴 단축키 텍스트 정렬 (V43)**: `menu.rs`에 `menu_action_sc()` 헬퍼 추가 — `egui::Button::new().shortcut_text()`로 메뉴에서 단축키 힌트 우측 정렬. 모든 메뉴 항목을 인라인 형식(`"New  (Ctrl+N)"`)에서 네이티브 `shortcut_text()` API로 변환. File(New/Open/Save As/Quit), Edit(Undo/Redo/Copy/Paste/Select All/Deselect All/Delete), View(투영/Standard Views/Grid/Fit All/Section Plane), Sketch(모든 지오메트리 도구, 구속조건, 토글, Close/Cancel), PartDesign(Pad) 메뉴 적용.

**컨텍스트 메뉴 아이콘, 웰컴 스크린 & 디스플레이 모드 셀렉터 (V42)**: `menu_item()` 헬퍼로 컨텍스트 메뉴 개선 — 이모지 아이콘 접두사 + 단축키 힌트 접미사. 오브젝트 메뉴: Select(📌), Duplicate(⎘), Rename(✏), Hide/Show(👁), Measure(📏), Check(✔), Delete(🗑). 뷰포트 메뉴에 단축키 키 표시. `draw_welcome_screen()` 빈 씬 오버레이 — CADKernel 타이틀, 3개 퀵 액션 버튼(Create Box, Import File, Open Project) + 페인터 기반 렌더링 + 수동 히트 테스트. 호버 시 배경/커서 변경. 하단 F1 힌트. `scene.is_empty() && sketch_mode.is_none() && active_task.is_none()` 조건에서만 표시. 툴바 View 섹션에 디스플레이 모드 ComboBox 드롭다운 추가 — 현재 모드 표시, 8개 모드 + `mode.shortcut()` 나열. `tb_display_mode: DisplayMode` GuiState 필드, 매 프레임 `ViewportInfo`에서 미러링.

**상태 바, 기즈모 툴바 & 투영 토글 (V41)**: 상태 바 우측 섹션을 `vert_divider()`로 구분된 개별 스타일 세그먼트로 분리 — 투영(클릭 가능, `Persp` 파랑 / `Ortho` 녹색, `GuiAction::ToggleProjection` 토글), 디스플레이 모드, 씬 통계(`vis/total obj` + K/M 삼각형 포맷팅), 선택 정보(악센트 블루), 측정 모드(노란색), FPS. `draw_status_bar()` 시그니처 `&mut GuiState`로 변경(액션 푸시용). 툴바에 "Transform" 섹션 추가 — Move(W), Rotate(E), Scale(R) 기즈모 토글 버튼 + `icon_toggle` 활성 하이라이트. `GuiAction::SetGizmoMode(GizmoMode)` 추가 — 같은 모드 클릭 시 해제, 다른 모드 클릭 시 전환. `GizmoMode` `toolbar.rs`와 `app.rs` 최상위 임포트.

**다이얼로그 일관성, 키보드 단축키 & 트리 다듬기 (V40)**: `dialogs.rs`의 전체 32개 다이얼로그 그리드를 3열(Label | DragValue | "mm")에서 2열 레이아웃 + DragValue `.suffix(" mm")`로 변환 — V39 프로퍼티 패널 패턴과 일치. 그리드 간격 `[10.0, 4.0]`으로 확대, 모든 수동 3열 `ui.label("")` 잔재 제거, 레이블 색상 `theme::COLOR_DIM` 통일. 샤프트 세그먼트 DragValue에 `.suffix(" mm")` 적용(L=/D= 프리픽스). F1 키로 키보드 단축키 참조 패널 토글 — 9개 카테고리 섹션(File, Edit, Navigation, Standard Views, Display Modes, Transform Gizmo, Selection Modes, Sketcher, General) + `dialog_section()` 악센트 헤더 + 이모지 아이콘. Sketcher 섹션에 스케치 모드 내 모든 키바인딩 문서화(S/L/R/C/A/E/P/B/W/H/V/Enter/Escape). 모델 트리 미니 툴바 추가(검색 상자~트리 사이) — 전체 펼치기(▿)/전체 접기(▹) 버튼으로 모든 최상위/하위 노드 `("tree_expand", obj_id)` 상태 토글. 필터 활성 시 미니 툴바에 결과 수 표시.

**프로퍼티, 리포트, 히스토리 & 스케치 다듬기 (V39)**: 프로퍼티 패널 파라미터 그리드 간격 `[10.0, 4.0]`으로 확대, 2열 레이아웃 — DragValue `.suffix(" mm")`로 별도 단위 레이블 열 대체. 씬 오버뷰에 삼각형/버텍스 집계 수 K/M 포맷팅(`format_count()` 헬퍼) + 아이콘 헤더 표시. 리포트 패널 로그 항목에 레벨 아이콘(ℹ/⚠/✖) + 모노스페이스 타임스탬프 열 + 경고/에러 행 틴트 배경. 히스토리 패널 항목에 `history_icon()` 함수로 컨텍스트 인식 작업 아이콘(➕ 생성, ➖ 삭제, → 이동, ↻ 회전, ⤢ 스케일, ∪ 불리언, ⬆ 돌출) + 모노스페이스 번호 정렬 + 녹색 화살표 현재 상태 마커. 스케치 치수 입력 팝업 재설계 — 아이콘 타이틀, DragValue 접미사(mm/°), 스타일된 OK/Cancel 버튼. 스케치 컨텍스트 메뉴 섹션 헤더(Edit, Constraints, Selection) + 메뉴 항목 아이콘.

**FreeCAD 스타일 스케치 UI, 태스크 패널 & 테마 확장 (V38)**: 태스크 패널 완전 오버홀 — `draw_task_header()` (악센트 그라디언트 바 + 아이콘), `draw_task_section()` (악센트 밑줄 레이블), `draw_task_buttons()` (악센트 블루 OK + 기본 Cancel). DRY 매크로 `plabel!/pmm!/pdeg!/pval!`로 DragValue 접미사(" mm", "°") 지원 그리드 렌더링. 메뉴 바에 `menu_section()` + `theme::MENU_SECTION_COLOR`로 그룹 섹션 표시. 스케치 UI 색상을 FreeCAD 팔레트로 변경 — 흰색 지오메트리, 파란 보조선, 녹색 선택, 금색 대기, 빨간 구속. 커서 십자선 갭-센터 스타일. 스냅 인디케이터 재설계: 일치점 = 점+링, H/V = 빨간 점선 가이드라인 + 텍스트 배지, 중점 = 채운 다이아몬드. 배너에 둥근 배경 필 + 색상 코드(빨강=충돌, 녹색=구속, 파랑=기본). DOF 화살표 주황색. 보조선 점 파란 X 마커. OVP 패널에 도구 아이콘+이름 헤더. 테마에 8개 스케치 색상 상수(`SKETCH_GEOMETRY/CONSTRUCTION/SELECTED/HOVERED/PENDING/CONSTRAINT/VIOLATED/DOF`) + `MENU_SECTION_COLOR` 추가. ComboView 기본 300px, 트리/속성 45/55 분할.

**툴바, 다이얼로그 & 컨텍스트 메뉴 다듬기 (V37)**: `section_label()` — 9px 연회색 텍스트 레이블을 모든 9개 워크벤치 컨텍스트 툴바(Part, PartDesign, Sketcher, Mesh, TechDraw, Assembly, Draft, Surface, FEM)와 메인 툴바(File/Edit/View/Scene/Select)의 각 도구 그룹 앞에 렌더링. `toolbar_separator()` 그라디언트 페이드(투명→회색→투명 + 밝은 중심). `dialog_section()` FreeCAD 스타일 액센트 바 헤더 — 파란 틴트 배경 + 3px 좌측 액센트 바. `button_bar()` 악센트 블루 기본 버튼(흰색 텍스트, 최소 크기). 모든 31개 다이얼로그 그리드 간격 `[4.0, 2.0]` → `[8.0, 4.0]`. 컨텍스트 메뉴 `menu_section()` 연회색 굵은 레이블 그루핑(오브젝트: Selection/Edit/Appearance/Analysis, 뷰포트: View/Display/Overlays/Selection/Create).

**FreeCAD 스타일 UI 크롬 (V36)**: 테마 시스템에 패널 크롬 색상 (`panel_header_bg/text`, `section_header_bg/text`, `panel_separator`) 및 사이징 (`panel_header_height`, `section_header_height`, `tree_row_height`) 확장 — 밀도별 스케일링. `draw_panel_header()` FreeCAD 스타일 독 타이틀 바 (어두운 배경, 제목, 닫기 버튼). `draw_section_header()` 접기 가능한 프로퍼티 그룹. ComboView 패널 제로 내부 마진 + 타이틀 헤더. 모델 트리 문서 루트 노드, 선택 시 좌측 액센트 바, 호버 전용 눈 아이콘. 프로퍼티 패널 언더라인 Data/View 탭, 섹션 헤더 배경 바, 확장된 그리드 간격. 리포트 패널 커스텀 탭 바 + 언더라인. 상태 바 `vert_divider()` 얇은 선 + 좌표 단위 접미사.

**카메라 뷰 북마크**: `nav.rs`의 `ViewBookmark` 구조체 (이름, yaw, pitch, roll, distance, target). NavConfig에 `view_bookmarks` (최대 20개). View > Bookmarks 서브메뉴 (220px 너비): 힌트 텍스트 입력, 활성/비활성 Save 버튼, 카메라 각도 툴팁(yaw°/pitch°/dist)이 있는 번호 항목, 우측 정렬 삭제, "X/20 bookmarks" 카운트. 저장 시 이름 매칭으로 덮어쓰기. 복원은 `.cloned()`로 빌림 충돌 방지 후 애니메이션. 북마크 정보 매 프레임 GuiState에 미러링.

**프리셀렉션 하이라이트**: `draw_selection_overlay()`가 선택 없이도 엣지/페이스/버텍스 프리셀렉션 표시. `nav.preselection_color`/`nav.selection_color` ([u8; 3])로 설정 가능. 커서 따라다니는 엔티티 타입 라벨. 엣지: 8px 글로우 + 3.5px 코어. 버텍스: 10px 글로우 링 + 6px 마커 + 2px 중심점. hover/프리셀렉션 갱신은 이제 전체 scene 재빌드와 분리되며, 선택은 씬 순회 첫 객체가 아니라 깊이 기준 nearest-hit를 사용합니다.

**디스플레이 모드 렌더링**: `DisplayMode::Points`는 전용 `PointList` GPU 파이프라인으로 렌더링됩니다. 오브젝트별 shaded draw는 draw call마다 하나의 dynamic uniform slot을 재사용하여 대규모 씬에서의 조용한 객체 누락을 방지합니다.

**오브젝트 그룹핑**: `scene.rs`의 `ObjectGroup` 구조체. `group_id` 필드 (0 = 미지정). Scene 메서드: `create_group()`, `group_selected()`, `ungroup_object()`, `toggle_group_visibility()`, `delete_group()`, `group_members()`. Edit > Groups 서브메뉴: 눈 아이콘 (◉/○) 색상 코딩, 멤버 수 "(N)", 힌트 텍스트 입력. 모델 트리 그룹 섹션: 헤더에 눈 토글/폴더 아이콘/이름/카운트/삭제; 인덴트된 멤버 이름.

**3D 측정 오버레이**: `draw_measurement_overlay()` — `nav.unit_system.label()`/`nav.decimal_places`로 단위 인식 표시. 번호 포인트 마커 (P1, P2...), 좌표 표시, 연속 쌍 거리, 3점 이상 총 경로, ΔX/ΔY/ΔZ 분해, 각도 호+도 표시. `draw_label_with_bg()`로 라벨 배경 (둥근 사각형+외곽선). `pick_surface_point()` 버텍스 스냅 (임계값 `camera.distance * 0.012`), 삼각형 폴백. C 클리어, Escape 종료.

**좌표축 인디케이터**: `draw_axes_overlay()` — 클릭으로 표준 뷰 스냅 (X+→Right, X−→Left, Y+→Front, Y−→Back, Z+→Top, Z−→Bottom) `SetStandardView` 액션. 호버 시 링 하이라이트 + 포인팅 커서. 깊이 정렬 투명도 페이드. 글로우 라인 (5px 글로우 + 2.5px 코어). 라벨 텍스트 그림자. 그라데이션 배경 링. 스페큘러 중심 구체. `Camera::forward()` 깊이 계산.

**NavConfig** (`nav.rs`): `Clone` derive가 적용된 영구 설정 구조체. 핵심 필드: 마우스 스타일, 감도, 애니메이션, View Cube, theme_mode, ui_density, 오버레이 토글. 확장 필드: `unit_system` (`UnitSystem`), `decimal_places`, `bg_preset` (`BgPreset`), `selection_color`, `preselection_color`, `tessellation_segments`, `auto_save_enabled`, `auto_save_interval_secs`, `recent_files_max`, `confirm_delete`. 네비게이션 동작 필드: `orbit_style` (`OrbitStyle`), `rotation_mode` (`RotationMode`), `zoom_step` (f32, 0.2), `zoom_at_cursor` (bool), `disable_touch_tilt`, `enable_spinning`, `show_rotation_center`, `rotation_center_size`. 메서드: `apply_orbit()` OrbitStyle별 궤도 연산 분기, `scroll_zoom_factor()` `zoom_step` 사용, `drag_zoom_factor()` `zoom_sensitivity` 사용. 신규 열거형: `UnitSystem` (5종), `BgPreset` (4종), `OrbitStyle` (5종), `RotationMode` (3종).

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

### 심화 품질 개선
불리언: `boolean_op`에 자동 면 분할 (split→classify→evaluate), 다중 샘플 분류 (7점 다수결). 스케치: 야코비안 랭크 DOF 분석, `drag_solve()` 대화형 점 드래그. 뷰어: Moller-Trumbore 레이 피킹 (`picking.rs`), 하위 요소 피킹 (Face/Edge/Vertex) B-Rep 토폴로지 순회, undo/redo 명령 스택 (`command.rs`, `ModelSnapshot` 기반). 치명적 `mat4_inv` 버그 수정 (역순 여인수 명명 + 잘못된 행 인덱스)으로 3D 피킹 안정화. 하위 요소 시각적 하이라이트 오버레이 (`draw_selection_overlay` + `draw_entity_highlight`): Face 삼각형 반투명 파란색 채움 + 가장자리 윤곽선, Edge 두꺼운 하이라이트 라인 + 끝점 도트, Vertex 채워진 원 + 흰색 링. 프리셀렉션(호버) 녹색 색조, 선택 파란색 색조. `update_preselection()`이 하위 요소 핸들을 추적하여 실시간 호버 피드백 제공. 속성 패널 (`properties.rs`)이 하위 요소 세부 정보 표시 (면적/삼각형/루프, 엣지 길이/끝점, 꼭짓점 좌표) — Face/Edge/Vertex 선택 모드 활성 시 접이식 그룹으로 표시. 다중 선택: `selected_entities: Vec<SelectedEntity>` Ctrl+클릭 토글, 오버레이가 모든 선택 엔티티 렌더링, 속성 패널에 다중 선택 요약 (수량, 총 면적/길이). 측정 오버레이: `draw_measurement_overlay_between()` — 2개 선택 시 점선 청록색 라인 + 거리 라벨 표시. 선택 모드 툴바: 4개 토글 버튼(Solid/Face/Edge/Vertex) + 수량 배지 + Select All/Deselect, `SetSelectionMode` 액션, 키보드 단축키 (Key 2→Face, Key 4→Vertex). 선택 기반 연산: `selected_edge_pairs()`가 선택된 엣지를 필릿/챔퍼용 꼭짓점 쌍 핸들로 변환; `compute_face_workplane()`이 선택된 면 삼각형에서 WorkPlane 도출 (무게중심 + 법선 + 수직 x축); `SketchOnSelectedFace` 액션이 면 기반 평면에서 스케치 모드 진입. 컨텍스트 메뉴 실제 연산 연결: 면 메뉴에서 `SketchOnSelectedFace` 디스패치, 엣지 메뉴에서 선택된 엣지로 `FilletAllEdges`/`ChamferAllEdges` 디스패치, 측정 버튼이 `ToggleMeasurement` 디스패치. 툴바 선택 인식: Part/PartDesign Fillet/Chamfer 버튼이 선택된 엣지 감지하여 직접 디스패치 vs. 태스크 패널 열기; Sketcher 툴바에 면 선택 시 "On Face" 버튼 표시. 엣지 루프 선택: `compute_edge_loop()` 시드 엣지에서 공유 꼭짓점을 통한 BFS valence-2 체인 워크. 엣지 링 선택: `compute_edge_ring()` 쿼드 면의 반대편 엣지 순회 (4변 면에서 index+2). 면 루프 선택: `compute_face_loop()` 시드 면에서 공유 엣지를 통한 BFS 외향 탐색. 세 가지 모두 컨텍스트 메뉴에 연결 (`SelectEdgeLoop`, `SelectEdgeRing`, `SelectFaceLoop` 액션). 필릿/챔퍼 파라미터 UI: 툴바가 항상 DragValue 슬라이더 포함 태스크 패널 열기; 태스크 패널에 사전 선택된 엣지 수 표시. Solid 모드 자동 피킹: `pick_auto()`가 vertex → edge → face → solid 순서로 가장 구체적인 요소 자동 감지, 수동 모드 전환 불필요; `select_object_for_pick()` 공유 헬퍼. 네비게이션: FreeCAD 12개 스타일 정확한 매핑, OrbitStyle (5종 — 가상 구체 트랙볼 포함), RotationMode (3종), zoom_at_cursor, zoom_step. 통합 자동 피킹: 모든 선택 모드에서 `pick_auto()` 사용 (vertex > edge > face > solid 우선순위), 프리셀렉션도 자동 피킹; 모드별 `pick_face()/pick_edge_mode()/pick_vertex_mode()` 제거. 더블클릭 루프 선택: `try_double_click_loop()` (300ms/10px 임계값) — 엣지 더블클릭 → 엣지 루프, 면 더블클릭 → 면 루프. 호버 미리보기: 상태 바에 커서 아래 엔티티 타입+인덱스 표시 `build_hover_preview()`. 컨텍스트 메뉴, 속성 패널, 선택 오버레이 모두 `SelectionMode` 대신 `selected_entities` 내용에서 엔티티 타입 감지. 스케치 인터랙션: 모든 도구에 대한 실시간 고무줄 미리보기 (Line/Rectangle/Circle/Arc/Ellipse/Polygon/Polyline/BSpline/Slot), 그리드 스냅 (0.5) + 점 스냅 (0.3 임계값), 타원 도구가 `add_ellipse()` 사용 (`add_circle()` 아님), 호 도구가 클릭 각도 기반 ±90° 반원 사용. 스케치 실행취소: `SketchSnapshot`이 각 연산 전 엔티티 수 기록, `SketchMode::undo()`가 벡터를 스냅샷으로 절단; 스케치 모드에서 Ctrl+Z가 전역 실행취소 대신 마지막 스케치 연산 취소. 스케치 편집: `SketchEntityRef` 열거형으로 엔티티 선택, Select 도구에 히트테스트 (point > line > circle > arc > ellipse > B-spline 우선순위), 선택된 엔티티 파란색 하이라이트 (3px 스트로크), Ctrl+클릭 다중 선택, Delete 키로 선택 엔티티 삭제 (연쇄 PointId 조정). 스케치 키보드 단축키: S/L/R/C/A/E/P/B/W (도구), H/V (제약조건). 인터랙티브 제약조건: 모든 툴바 버튼이 선택된 엔티티에 적용 (Coincident, Parallel, Perpendicular, Equal, Fixed, Block, Distance, Angle, Radius, H/V-Distance); 선택 없으면 마지막 엔티티로 폴백. Extrude 거리: Sketcher 툴바에 DragValue UI (0–1000mm).

### Phase V7: 파일 포맷 확장
5개 신규 포맷: glTF 임포트 (base64 버퍼 디코드, 다중 컴포넌트 인덱스), 3MF 임포트 (XML 파싱), DWG 임포트/익스포트 (R2000–2018+ 버전 감지, 3DFACE 휴리스틱, DXF 폴백), PDF 익스포트 (SVG→PDF 1.4 변환, TechDraw용), DAE Collada 임포트/익스포트 (COLLADA 1.4.1 XML geometry). 총 I/O 포맷: 15개 (기존 11개). 신규 모듈: `collada.rs`, `dwg.rs`, `pdf.rs`.

### Phase V13: 성능 & 검증
BVH 가속 불리언 broad-phase (O(n log n) 면 쌍 중첩), 11개 신규 벤치마크 (총 25개): 프리미티브 (cone, torus), 피처 (mirror, scale, fillet), 검증 (check_geometry, check_watertight), 스트레스 (tessellate_sphere_64x32, tessellate_torus_64x32, boolean_intersection).

### Phase V12: Python 바인딩
PyO3 기반 `cadkernel` Python 모듈 (`crates/python/`, 독립 빌드, workspace 제외). 6개 클래스 (Model, SolidHandle, Mesh, MassProperties, GeometryCheck, Sketch), 10개 프리미티브 생성기, 4개 피처 연산, 3개 불리언 연산, 테셀레이션/분석, 10개 I/O 함수, 7개 제약조건 타입의 스케치 시스템.

### FreeCAD 수준 UI 대개편
`gui.rs` (3605줄) → `gui/` 모듈 디렉토리 리팩토링 (12개 파일). 계층형 모델 트리, 속성 편집기, 완전한 메뉴 시스템, 향상된 상태바, 색상 코드별 로깅 리포트 패널, Solid/뷰포트 컨텍스트 메뉴, 9개 워크벤치 툴바 (Part, PartDesign, Sketcher, Mesh, TechDraw, Assembly, Draft, Surface, FEM). 40+ 액션 핸들러에 리포트 로깅 추가.

### Phase W: FreeCAD 패리티 스프린트
Part 형상 프리미티브 (circle, ellipse, point, line shape + shape builder + convert to solid). PartDesign 완성 (additive_loft/pipe, sprocket, shaft_design, shape/sub-shape binder, body 컨텍스트 메뉴: suppress_feature/set_tip/move_feature). 스케처 기하 확장 (periodic B-spline, B-spline from knots, 중심/둥근 사각형, 슬롯, 아크 슬롯, 3점 원/타원, 굴절 제약, toggle_driving_reference, attach/reorient/merge/mirror 스케치). 스케처 B-스플라인 도구 (geometry_to_bspline, 차수 증감, 노트 다중도 조정, insert_knot, join_curves, external_projection, carbon_copy, move/rotate/scale/offset/mirror 기하, delete all geometry/constraints). TechDraw 뷰 (broken, complex section, clip group, active, project shape 2D), 치수 (contextual, 3점 각도, 면적, 호 길이, H/V extent, 치수 참조 수리), 주석 (서식 텍스트, 풍선, 축측 길이, 기하 해칭, 용접 기호 ISO 2553, 구멍/축 끼워맞춤). TechDraw 중심선 (면, 선/점 사이, 볼트 원), 장식 (선, 내/외부 나사, 정점, 원, 호, 평행/수직선), 서식 (체인/좌표/챔퍼 치수, FormattedDimension), 관리 (stack order, align, lock, 페이지 템플릿, 필드 갱신, 재그리기, 전체 인쇄, 외관 편집, 엣지 가시성 토글). Draft 워크벤치 (3점 호, 타원/사각형/다각형 와이어, 베지어/큐빅 베지어, 점, facebinder, 해칭, 치수/레이블/주석 스타일, move/rotate/scale/mirror/offset/trimex/stretch, circular/path link/point link 배열, edit/join/split, draft to sketch, 스냅 시스템). Assembly (solve_constraints Newton-Raphson, simulate_step, export_asmt, AssemblyPreferences). FEM (AnalysisContainer, ElementGeometry, EM/유체 경계조건, GeometricalFeature, heat/flow/deformation/electrostatic 방정식, 필터 함수, 시각화 모드, purge results, 메시 영역). I/O (VRML/AMF 가져오기/내보내기). 182개 신규 테스트 (총 942개).

**어셈블리 솔버 아키텍처:**
`assembly.rs`의 Newton-Raphson 반복 제약 솔버. `solve_constraints()`는 제약 잔차와 야코비안을 계산하고 수렴까지 반복. `simulate_step()`은 운동학 시뮬레이션의 시간 스텝 진행. `AssemblyPreferences`로 솔버 허용오차 및 반복 한계 설정.

**FEM 방정식 시스템:**
`fem.rs`의 4가지 방정식 타입: `heat_equation()` (정상 상태 열), `flow_equation()` (스토크스 유동), `deformation_equation()` (선형 탄성), `electrostatic_equation()` (포아송). 각각 필드 결과 (온도, 속도, 변위, 전위) 반환. `AnalysisContainer`가 메시, 재료, 경계조건, 결과를 그룹화. `apply_filter()` + `FilterFunction` 열거형 (Threshold, Clip, Contour, Gradient)으로 후처리. `VisualizationMode` 열거형으로 결과 표시.

**TechDraw 서식 시스템:**
`FormattedDimension` 구조체 (접두사, 접미사, 공차, 오버라이드 필드). `chain_dimension()` / `coordinate_dimension()`으로 연속 치수. `chamfer_dimension()`으로 챔퍼 주석. `stack_order()`, `align_elements()`, `lock_element()`로 요소 레이아웃 관리. `page_from_template()`으로 템플릿 필드 치환.

**Draft 스냅 시스템:**
`SnapMode` 열거형 (16가지 스냅 모드: Endpoint, Midpoint, Center, Quadrant, Intersection, Extension, Perpendicular, Parallel, Tangent, Nearest, Grid, Ortho, Special, Dimension, WorkingPlane, Angle). `snap_to_point()`로 가장 가까운 스냅 대상 탐색. `snap_lock()`으로 스냅 모드 활성화 토글.

**스케처 B-스플라인 도구:**
`geometry_to_bspline()`은 모든 스케치 엔티티를 B-스플라인으로 변환. `increase/decrease_bspline_degree()`로 차수 승격/감소. `increase/decrease_knot_multiplicity()`로 연속성 제어를 위한 노트 다중도 조정. `insert_knot()`으로 노트 삽입. `join_curves()`로 인접 B-스플라인 세그먼트 병합.

**VRML/AMF 포맷:**
`vrml.rs`: VRML97 가져오기/내보내기 (Shape, IndexedFaceSet, Coordinate, Normal 노드). `amf.rs`: AMF 1.1 XML 가져오기/내보내기 (object/mesh/volume/vertex/triangle 계층).

**현재 상태**: 1133 tests, 0 clippy warnings, 0 build errors. FreeCAD 패리티: 576/576 (100%).

### FreeCAD 호환성 스프린트 3 (2026-03-25)
커널 갭 해소 (12개 항목): PrimitiveParams + make_primitive (통합 생성자), offset_polygon_2d_checked (KernelResult), project_curves_on_surface (커브 투영), auto_defeaturing (크기 임계값), transformed_copy (복제+변환), Body::move_object_to_body (피처 이동), add_triangle/square/pentagon/hexagon/heptagon/octagon (다각형 단축), external_intersection (스케치 도구), toggle_section_view + SectionViewState, coons_patch (쌍선형 블렌딩), make_line_draft (2점 선). FEM 완성 (28개+ 항목): HexMesh + generate_hex_mesh, mesh_from_shape, adaptive_mesh_refinement, mesh_smoothing, export_mesh_abaqus/nastran, nonlinear_static/frequency/buckling_analysis, magnetostatic/coupled_thermo_mechanical/acoustic/poisson/diffusion 방정식, 전체 후처리 (노달 값, 보간, 오차 추정, 점별 결과, 표면 적분, 경로 결과, 반력), fem_summary, export_fem_report, 메시 품질, BC 검증, 계산 시간 추정. I/O: import_svg (7가지 SVG 요소, 경로 명령, 변환, 이어-클리핑), import_pdf (벡터/텍스트 추출), export_drawing_dxf (전체 TechDraw→DXF). 96개 신규 테스트 (총 1133개).

### UI 스프린트: FreeCAD 100% 패리티 달성 (2026-03-25)
576/576 FreeCAD 기능 호환성(100%)을 달성한 뷰어 UI 완성. GuiAction 열거형이 ~40개에서 130개 이상으로 확장되어 9개 워크벤치 전체를 커버. `app.rs`의 `process_actions()`가 모든 액션을 백엔드 크레이트 호출에 연결. `toolbar.rs`에 9개 완전 기능 워크벤치 툴바 (Part, PartDesign, Sketcher, Mesh, TechDraw, Assembly, Draft, Surface, FEM). `dialogs.rs`에 모든 프리미티브와 연산을 위한 생성 다이얼로그. `sketch_ui.rs`에 전체 제약조건 시각화 (24가지 타입), 설정 가능한 그리드, 스냅 인디케이터 (7가지 타입), B-spline 표시를 포함한 스케치 UI. `context_menu.rs`에 오브젝트 및 뷰포트 컨텍스트 메뉴. 신규 열거형 타입: `AssemblyJointType` (13가지 조인트), `FemConstraintType` (6가지 제약 타입). 백엔드 연동: FEM 사면체 메시 생성 + 6가지 해석 + 9개 방정식, 어셈블리 제약 해석 + 시뮬레이션, TechDraw 뷰/치수/중심선/장식 도구, Draft 와이어 생성/수정/배열, Surface 연산, 15개 이상 I/O 포맷.

### V15 프로페셔널 인터랙션: 3D 기즈모, 클립 플레인, 단축키 & 월드 좌표 (2026-03-30)
3D 변환 기즈모: `GizmoMode` (Translate/Rotate/Scale), 스크린 공간 축 투영, 축별 호버 하이라이트. `draw_transform_gizmo()` → `draw_translate_gizmo()/draw_rotate_gizmo()/draw_scale_gizmo()` 디스패치. GPU 클립 플레인: WGSL 셰이더에 `clip_params` vec4 유니폼, 프래그먼트 discard + 주황색 절단면 하이라이트. `world_position`을 VertexOutput에 추가하여 프래그먼트별 클립 거리 계산. 키보드 단축키 패널: `?`/F1 토글, 5개 카테고리, 2열 그리드 레이아웃. 마우스 월드 좌표: Z=0 지면 평면 레이 캐스트, 상태 표시줄 3D 좌표 표시.

### V14 인터랙티브 선택: 박스 선택, 선택 게이트 & 내비게이션 수정 (2026-03-30)
박스 선택: 좌클릭 드래그로 선택 사각형, 좌→우 = 윈도우(파란색), 우→좌 = 크로싱(녹색/점선). `object_screen_aabb()`: 메시 AABB 8개 꼭짓점을 스크린 공간 투영. 선택 게이트: `try_pick_entity()`가 `SelectionMode` (Solid/Face/Edge/Vertex) 반영, 모드별 `selected_entity` 설정. `first_face()/first_edge()/first_vertex()` 헬퍼로 B-Rep 토폴로지 탐색. FreeCADGesture 내비게이션 수정: 좌클릭 드래그 → 박스 선택 (기존 궤도 회전 오류), 중간 버튼 → 궤도 회전.

### V13 FreeCAD 패리티: 프로페셔널 UI 대개편 (2026-03-30)
프리셀렉션 호버 하이라이트: WGSL 셰이더에 `hover_params` vec4 유니폼, GPU 측 호버 블렌딩(PRESELECT_COLOR). 계층형 모델 트리: Body > Feature 중첩, 14종 엔티티 아이콘, Tip 마커, 상태 표시기, 접기/펼치기, 드래그앤드롭. 향상된 프로퍼티 패널: 접을 수 있는 그룹, Placement 편집기(위치+회전), 계산된 속성(부피/표면적/무게중심), 검색/필터. 플라이아웃 툴바: `flyout_button()` 그룹 드롭다운, 마지막 사용 도구 기억, Part/PartDesign 플라이아웃 그룹화. 빠른 측정 & 상태 표시줄: 선택 모드 표시, 프리셀렉션 정보, 자동 치수. 프로페셔널 테마: `CadTheme`(25+ 색상), Dark/Light, `UiDensity`(Compact/Normal/Spacious). 스케치 UI: 11종 도구(Point/Ellipse/Polyline/Slot/BSpline/Polygon 추가), 커서 십자선, 향상된 구속 시각화. 9개 워크벤치, SelectionMode(Solid/Face/Edge/Vertex), 13종 AssemblyJointType, 6종 FemConstraintType, 5종 NavStyle, 5종 UnitSystem, 4종 BgPreset.

### V12 핵심 UI 대개편 (2026-03-27)
통합 작업 시스템: 모든 메뉴/컨텍스트 메뉴/툴바의 프리미티브 생성이 ActiveTask 인라인 패널로 전환 — 레거시 팝업 대화상자 시스템 제거. `draw_create_dialogs()` — `gui.active_task.is_none()` 게이팅. CAD 포맷 임포트 수정: `load_mesh_file()`이 STEP/IGES/BREP/DXF/PLY/3MF을 올바른 임포터로 라우팅 (기존에는 STL/OBJ 외 모든 포맷 무시). `FileLoadResult` 열거형: BRep (STEP/IGES/BREP) vs 메시 (DXF/PLY/3MF/STL/OBJ) 구분. 툴바 비활성화 상태: `icon_button_disabled()` + `ToolbarContext`로 컨텍스트 인식 버튼 활성화/비활성화. `gated_button!` 매크로. Part/PartDesign 작업 선택 없을 때 비활성화; Undo/Redo 스택 비었을 때 비활성화.

### UI 폴리시 스프린트: 전문 CAD 품질 달성 (2026-03-25)
FreeCAD/CATIA/SolidWorks/Fusion 360 수준의 전문 UI 개선. 테마 시스템 (`theme.rs`): `CadTheme` 30개 이상 색상/간격 필드, Dark/Light 프리셋, 3가지 밀도 모드 (Compact/Normal/Spacious), `apply_to_egui()` 완전 통합. 벡터 아이콘 툴바 (`toolbar.rs`): 150개 이상 `ToolIcon` 변형, `egui::Painter` 렌더링, `icon_button()` (28×28), 액센트 언더라인 워크벤치 탭. 계층적 모델 트리 (`tree.rs`): `EntityIcon` (14가지 타입), 생성 이력 기반 `TreeNode`, 트리 가이드 라인, 검색/필터, 인라인 이름 변경, 컨텍스트 메뉴. 향상된 다이얼로그 (`dialogs.rs`): 공유 헬퍼, 입력 검증 (빨간색 오류), "mm" 단위, 기본값 버튼. 속성 패널 (`properties.rs`): 섹션 헤더, 3열 그리드, 8개 프리셋 색상 선택기, 투명도 슬라이더. 상태 바 (`status_bar.rs`): 마우스 좌표, 도구 힌트, 씬 통계 + FPS. 리포트 패널 (`report.rs`): 타임스탬프, 심각도 필터, 접기 가능 메시지. 뷰포트 오버레이 (`overlays.rs`): 원점 XYZ 축, 3D 그리드, 측정 도구, 스냅 하이라이트. 내비게이션 설정 (`nav.rs`): 영구 `theme_mode`, `ui_density`, 오버레이 토글.

**FEM 아키텍처 (Sprint 3):**
`fem.rs`의 확장 FEM 모듈은 9가지 방정식 타입을 지원: heat, flow, deformation, electrostatic, magnetostatic, acoustic, poisson, diffusion, coupled_thermo_mechanical. 6가지 해석 타입: static, nonlinear_static, frequency, buckling, modal, thermal. 메시 생성: TetMesh 및 HexMesh (적응형 세분화 및 스무딩 지원). 후처리 파이프라인: extract_nodal_values -> interpolate_to_nodes -> compute_error_estimate. 결과 쿼리: result_at_point (무게중심 보간), integrate_over_surface, path_result, reaction_forces. 내보내기: Abaqus (.inp) 및 Nastran (.bdf) 메시 형식. 품질 보증: check_mesh_quality_detailed (요소별 ElementQuality), check_boundary_conditions (BC 일관성 검증), estimate_computation_time (런타임 추정).

**SVG 가져오기 아키텍처:**
`svg.rs`의 `import_svg()`는 7가지 SVG 요소 타입 (rect, circle, ellipse, line, polyline, polygon, path)을 파싱. 경로 명령: M/L/H/V/C/S/Q/T/A/Z (상대 명령 포함). 변환 파싱: matrix/translate/rotate/scale/skewX/skewY. 다각형은 이어-클리핑 알고리즘으로 삼각분할. `Mesh` (정점 및 삼각형 인덱스) 반환.

**Coons 패치:**
`surface_ops.rs`의 `coons_patch()`는 4개 경계 커브에서 쌍선형 블렌딩 서피스를 구현. S(u,v) = L1(u,v) + L2(u,v) - B(u,v) (L1/L2는 u/v 방향 ruled surface, B는 쌍선형 보정). NxN 그리드에서 샘플링된 NurbsSurface 반환.

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

전체 크레이트에 걸쳐 **1369개 테스트**. `cargo test --workspace`로 실행.

### 통합 테스트 카테고리 (stress_tests.rs)

`crates/modeling/tests/stress_tests.rs`에 워크플로우별로 구성된 67개 스트레스 테스트:

| 카테고리 | 테스트 수 | 커버리지 |
|----------|-----------|----------|
| 다중 피처 PartDesign Body | 9 | pad→pocket→chamfer 체인, 억제/재정렬, 미러, 패턴, 회전 |
| 10개 이상 파트 어셈블리 | 9 | 10-박스 어셈블리, BOM, 배치, 가시성, 구속조건, 간섭 |
| 20개 이상 구속조건 스케치 | 10 | L-형상(19개 구속조건), 대칭, H/V 거리, 중점, 호-접선 |
| 불리언 체인 (5회 이상) | 6 | 5-실린더 빼기, 합집합 십자형, XOR 체인, 교대 6회 연산 |
| 전체 I/O 라운드트립 | 10 | JSON, ASCII STL, 이진 STL, STEP, OBJ, glTF, PLY, BREP |
| 크로스 도메인 워크플로우 | 5 | 스케치→압출→검사→테셀레이션→STL, 어셈블리 BOM, 프리미티브 |
| 다중 불리언 체인 | 3 | 10회 이상 순차 union/subtract/intersect, 퇴화 입력, 빈 컴파운드 |
| 대형 어셈블리 | 2 | 50개 컴포넌트 간섭 검출, 대형 어셈블리 BOM |
| 복합 스케치 | 2 | 접선 호를 포함한 30개 이상 제약조건, 일치 체인 스트레스 |
| 패턴 스트레스 | 2 | 64개 인스턴스 linear/circular 패턴 |
| FEM & Surface 연산 | 4 | tet 메시 품질, 모달 해석, ruled→extend→pipe 체인, surface 스트레스 |
| I/O 에지 케이스 | 5 | 삼각형 수 0 바이너리 STL, 법선 없는 OBJ, 다중 프리미티브 glTF, 100K+ 대형 메시, 라운드트립 일관성 |

### 토폴로지 크레이트 테스트 — V32

33개 통합 테스트(`crates/topology/tests/topology_comprehensive.rs`): Tag/Naming (modified/merged/체인 연산, NameMap CRUD, ShapeHistory Evolution 변형), ModelHistory undo/redo (기본 사이클, 최대 이력 제한, redo 삭제, 이력 설명), 기하 바인딩 (curve/surface/trim/pcurve, 유효하지 않은 핸들 안전성), 내부 루프, 와이어 연산, 태그 엔티티 생성자 + 조회, faces_around_vertex, PropertyStore material/metadata, Color/Material 프리셋, Handle from_raw_parts, EntityStore is_alive/get_mut/iter_mut/슬롯 재사용, 검증 에지 케이스. 토폴로지 크레이트: 29 → 62 테스트.

### 테스트 전략 — V25 추가 사항

**스트레스 테스트 철학**: 각 스트레스 테스트는 3개 이상의 크레이트를 결합하는 현실적인 다단계 워크플로우를 실행합니다. 에지 케이스 테스트는 유효하지만 퇴화된 입력(길이 0, 거의 일치하는 정점, 빈 컴파운드)을 커버하여 사용자 경로에서 패닉이 발생하지 않도록 합니다.

**I/O 라운드트립 테스트**: 포맷을 임포트하고, 동일 모델을 익스포트한 후 다시 임포트하여 정점/삼각형 수를 허용 오차 내에서 비교합니다. 직렬화 정밀도의 회귀를 감지합니다.

**Python 바인딩 테스트**: `crates/python/`에 74개 통합 테스트 — `PyAssembly`, `PyFem`, draft/surface/compound 연산 커버. 별도 실행: `cd crates/python && PYO3_PYTHON=/usr/bin/python3 cargo test`.

### 벤치마크

`cadkernel-modeling`에 25개 Criterion 벤치마크. `cargo bench -p cadkernel-modeling`으로 실행.

| 연산 | 시간 | 복잡도 |
|------|------|--------|
| `make_box` | ~8 µs | O(1) |
| `make_cylinder(32)` | ~72 µs | O(N) |
| `make_sphere(32×16)` | ~749 µs | O(N·M) |
| `tessellate_sphere(32×16)` | ~60 µs | O(F) |
| `boolean_union(box+box)` | ~44 µs | O(Fa·Fb) |
| `stl_write_binary(sphere)` | ~223 µs | O(T) |
| `bench_parallel_boolean` | ~18 µs | O(Fa·Fb / 코어 수) |
| `bench_bvh_query_nearest` | ~3 µs | O(log N) |
| `bench_bvh_query_ray` | ~7 µs | O(log N) |
| `bench_pattern_parallel(64)` | ~210 µs | O(N / 코어 수) |

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

뷰어는 FreeCAD 스타일의 워크벤치 시스템을 사용하며, 130개 이상의 `GuiAction` 변형, 9개의 완전 기능 워크벤치 툴바, 완전한 백엔드 연동을 갖추고 있습니다.

- **`Workbench` 열거형**: `Part`, `PartDesign`, `Sketcher`, `Mesh`, `TechDraw`, `Assembly`, `Draft`, `Surface`, `FEM`
- **공통 툴바**: New, Open, Save, Undo, Redo, Fit All, Reset View
- **워크벤치 탭바**: 활성 워크벤치 컨텍스트 전환
- **컨텍스트 툴바**: 활성 워크벤치에 따라 변경
  - **Part**: 13개 프리미티브 (Box/Cylinder/Sphere/Cone/Torus/Tube/Prism/Wedge/Ellipsoid/Helix/Circle/Ellipse/Point/Line shapes), Boolean Union/Subtract/Intersect (다이얼로그), shape builder, convert to solid, Mirror/Scale/Shell/Fillet/Chamfer/Pattern/Thickness/Offset/Section, attachment, appearance, shape analysis, Measure/Check
  - **PartDesign**: Pad/Pocket/Revolve/Groove/Hole/Countersunk Hole, 10개 추가적/감산적 프리미티브 (box/cylinder/sphere/cone/torus/helix/ellipsoid/prism/wedge + loft/pipe), Fillet/Chamfer/Draft/Shell/Mirror/Scale/Pattern, body 연산 (suppress/set tip/move feature/move to body), shape binder, sprocket, shaft design
  - **Sketcher**: 8개 기하 도구 (Line/Rectangle/Circle/Arc/Ellipse/BSpline/Polygon/Slot), B-spline 도구 (차수/노트 다중도 조정, insert knot, join curves), 7개 제약 버튼, 디스플레이 옵션 (13가지 시각 토글), 스케치 관리 (attach/reorient/merge/mirror), external projection, carbon copy
  - **Mesh**: STL/OBJ/glTF Import/Export, 15개 메시 연산 (Decimate/Subdivide/Fill Holes/Flip Normals/Smooth/Harmonize/Boolean Union/Intersect/Difference/Cut/Remesh/Repair/Trim/Scale/Section), 분석 (곡률 플롯, 수밀성 검사, 바운딩 박스, 면 정보, 정다면체, UV 언래핑)
  - **TechDraw**: 7개 뷰 타입 (Front/Top/Right/Iso/3-View/Broken/Complex Section), 12개 치수 타입 (선형/각도/반지름/호 길이/범위/챔퍼/컨텍스트/체인/좌표), 6개 중심선 도구, 8개 장식 도구 (선/나사/정점/원/호/평행/수직), 서식 (FormattedDimension), 페이지 관리 (템플릿/갱신/재그리기/인쇄), SVG/DXF 내보내기
  - **Assembly**: Insert Component, 13가지 조인트 (Fixed/Revolute/Cylindrical/Slider/Ball/Distance/Angle/RackAndPinion/Screw/Belt/Gear/Parallel/Perpendicular), 제약 해석 (Newton-Raphson), 시뮬레이션 스텝, DOF 분석, 환경설정, ASMT 내보내기
  - **Draft**: 10개 와이어 생성 (fillet/circle/arc/ellipse/rectangle/polygon/bezier/arc_3pt/chamfer/point), 8개 수정 (move/rotate/scale/mirror/offset/trimex/stretch/edit), 5개 배열 패턴 (rectangular/circular/polar/path link/point link), 3개 주석 (dimension/label/text), 스냅 시스템 (16가지 모드), 쿼리 (length/area), 레이어 관리, upgrade/downgrade, wire-to-bspline
  - **Surface**: Ruled surface, Filling, Sections, Extend, Pipe, Coons patch, Curve on mesh
  - **FEM**: 4개 메시 타입 (TetMesh/HexMesh/generate/from shape), 6개 재료 프리셋 (steel/aluminum/titanium/copper/concrete/cast iron + thermal + custom), 8개 경계조건 (구조 4 + 열 4 + body load + contact + initial temperature), 6가지 해석 (static/nonlinear/frequency/buckling/modal/thermal), 9개 방정식 (heat/flow/deformation/electrostatic/magnetostatic/acoustic/poisson/diffusion/coupled), 후처리 (응력/변형률 텐서, 주응력, 안전 계수, 절점값, 오차 추정, 경로 결과, 반력), 내보내기 (Abaqus/Nastran), 품질 검사
- **파일 메뉴**: STEP, IGES, DXF, PLY, 3MF, BREP, STL, OBJ, glTF, CADK, VRML, AMF, OCA, SVG, PDF, DWG, DAE Import/Export
- **다이얼로그 패턴**: `GuiAction` 열거형 (130개 이상) → `gui.actions` 벡터 → `app.rs`의 `process_actions()`
- **불리언 다이얼로그**: 두 번째 박스 크기/오프셋용 DragValue가 있는 플로팅 `egui::Window`
- **FEM 타입**: `AssemblyJointType` (13가지), `FemConstraintType` (6가지) — 툴바 연동

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
| *(Trim Demo 제거됨)* | — | Phase V11에서 정리됨 |

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

## 13. 현재 상태 및 다음 단계

### 성능 아키텍처 (V25)

CADKernel은 세 가지 병렬 처리 계층을 사용합니다:

| 계층 | 메커니즘 | 적용 위치 |
|------|----------|-----------|
| 연산 수준 | `rayon::par_iter()` | 불리언 면 분류, 패턴 복제 루프 |
| 공간 쿼리 | BVH (`bvh.rs`) | 불리언 광역 단계, 테셀레이션, 레이 피킹 |
| 지오메트리 캐싱 | `BasisCache` LRU | NURBS 기저 함수 평가 |

**BVH API** (`crates/geometry/src/bvh.rs`):
- `Bvh::build(aabbs)` — SAH 기반 구성
- `Bvh::build_sah(aabbs)` — 명시적 SAH 변형
- `Bvh::query_aabb(aabb)` — 겹치는 모든 항목
- `Bvh::query_aabb_parallel(aabb)` — rayon 병렬 변형
- `Bvh::query_nearest(point)` → `Option<usize>` — 단일 최근접 항목
- `Bvh::query_ray(origin, dir)` → `Vec<usize>` — 레이 교차 항목 전체
- `Bvh::refit(new_aabbs)` — 동적 씬 업데이트

**NURBS 캐싱** (`crates/geometry/src/curve/bspline_basis.rs`):
- `BasisCache` — `(knot_hash, degree, u)`를 키로 하는 스레드 로컬 LRU (1024 슬롯)
- `CachedNurbsCurve` — `NurbsCurve`를 래핑하여 `point_at()` 시 캐시 경유
- `NurbsSurface::evaluate_cached()` — 서피스 평가 시 호출별 캐시 조회

**Python 바인딩 아키텍처** (`crates/python/src/lib.rs`):
- PyO3 `#[pyclass]` 래퍼 타입: `PyModel`, `PySolidHandle`, `PyMesh`, `PyMassProperties`, `PyGeometryCheck`, `PySketch`, `PyAssembly`, `PyFem`
- 모든 메서드는 `PyResult<T>` 반환 — `KernelError`를 `PyRuntimeError`로 매핑
- 빌드: `cd crates/python && PYO3_PYTHON=/usr/bin/python3 cargo build` (workspace 제외)
- Python 테스트: `cd crates/python && PYO3_PYTHON=/usr/bin/python3 cargo test`

**V16 완료 (2026-04-01)** — 1300개 테스트, FreeCAD 기능 호환성 100% (576/576)

| 워크벤치 | 커버리지 | 상태 |
|-----------|:--------:|------|
| Part | 100% | 58/58 구현 |
| PartDesign | 100% | 53/53 구현 |
| Sketcher | 100% | 109/109 구현 |
| TechDraw | 100% | 114/114 구현 |
| Assembly | 100% | 23/23 구현 |
| Mesh | 100% | 35/35 구현 |
| Surface | 100% | 6/6 구현 |
| Draft | 100% | 80/80 구현 |
| FEM | 100% | 80/80 구현 |
| I/O | 100% | 18/18 포맷 |

**UI 스프린트 요약 (2026-03-25):**
- `GuiAction` 열거형 ~40개에서 130개 이상으로 확장
- 9개 워크벤치 툴바 전체 백엔드 연동 완료
- 모든 `process_actions()` 핸들러가 실제 백엔드 크레이트 호출과 연결
- 스케치 UI: 전체 제약조건 시각화, 그리드, 스냅 인디케이터
- FEM 연동: 사면체 메시 생성, 6가지 해석 타입, 9개 방정식
- 어셈블리 연동: 13가지 조인트, Newton-Raphson 솔버, 시뮬레이션
- 15개 이상 I/O 포맷 파일 메뉴에서 접근 가능

**UI 폴리시 스프린트 (2026-03-25):**
- 전문 테마 시스템: `CadTheme` Dark/Light + 3가지 밀도 프리셋
- 벡터 아이콘 툴바: 150개 이상 `ToolIcon` 변형, `egui::Painter` 렌더링
- 계층적 모델 트리: `EntityIcon` (14가지 타입), 이력 기반 `TreeNode`, 검색/필터, 인라인 이름 변경
- 향상된 다이얼로그: 입력 검증, 단위 라벨, 공유 헬퍼, 기본값 버튼
- 속성 패널: 색상 선택기 (8개 프리셋), 투명도, 3열 매개변수 그리드
- 상태 바: 마우스 좌표 + 도구 힌트 + 씬 통계 + FPS
- 리포트 패널: 타임스탬프, 심각도 필터, 카운트 배지, 접기 가능 메시지
- 뷰포트 오버레이: 원점 축 (wgpu 3D 패스, V11), 그리드 (egui, 뷰포트 클리핑), 측정 도구, 스냅 하이라이트
- 컨텍스트 메뉴: 뷰포트, 오브젝트, 면/엣지 + 피처 재정렬 지원

**UI 폴리시 스프린트 2 (2026-03-25):**
- 트리: 인라인 눈 아이콘 가시성 토글, 팁 마커, 억제 디밍
- 툴바: `icon_button_active()` 액센트 하단 테두리로 활성 도구 피드백
- 키보드 단축키 레퍼런스 다이얼로그 (5개 섹션, 줄무늬 그리드, Help 메뉴)
- 설정: 외관 섹션 — 테마 (Dark/Light), 밀도 (Compact/Normal/Spacious)
- 정보 다이얼로그: 중앙 로고, 줄무늬 정보 그리드, 종합 프로젝트 정보
- ComboView 패널: 리사이즈 가능 폭 (200-450px), 액센트 구분선

**UI 폴리시 스프린트 3 (2026-03-25):**
- 투명 패널 수정: `CompositeAlphaMode::Opaque`로 Linux 컴포지터 블렌딩 아티팩트 방지
- 설정 재설계: 탭 방식 환경 설정 다이얼로그 (일반/디스플레이/내비게이션/외관/조명)
- 동적 배경: 5가지 그라디언트 프리셋 (Custom 포함) + `GpuState::update_bg()` 런타임 셰이더 재빌드
- NavConfig 확장: 단위 시스템, 소수점 자릿수, 배경 프리셋, 선택 색상, 테셀레이션 품질, 자동 저장, 최근 파일
- 신규 열거형: `UnitSystem` (5가지 변형), `BgPreset` (5가지 변형, Custom 포함)

**V11: 뷰어 UI 확장 (2026-03-25):**
- 작업 패널: 5 → 35개 ActiveTask 변형으로 인라인 매개변수 편집 + 실시간 3D 프리뷰 (프리미티브 10, PartDesign 11, Draft 6, Surface 2, FEM 1, Boolean 1, Scale 1); 툴바 버튼이 즉시 실행 대신 작업 패널을 열어 OK/Cancel 흐름으로 확인 또는 취소
- 레거시 팝업 다이얼로그 대체: 모든 생성 연산이 팝업 다이얼로그나 즉시 실행 대신 OK/Cancel 흐름의 인라인 작업 패널 사용
- 스케처 툴바 완성: B-spline 도구 (변환, 차수+/-, 노트 삽입), Split/Mirror/External Projection/Carbon Copy, Block/HDistance/VDistance 구속조건
- 메뉴 개선: 거리 측정 측정 모드 연결, Macro 메뉴 비활성화 (계획 표시), 워크벤치 전환 메뉴, Tools 메뉴에 Origin/Grid3D 토글
- 원점 오버레이: wgpu 전용 축 렌더링 (중복 egui 오버레이 제거), Z축 전체 길이, show_origin이 그리드와 독립적으로 wgpu 축 제어
- 그리드 오버레이: egui 그리드를 뷰포트 rect로 클리핑 (clip_rect) — 패널 관통 현상 해결

**V11 심층 개편 (2026-03-26):**
- GuiAction 처리: 200개 이상의 변형에 대한 구체적 백엔드 구현 완성 (스텁 핸들러 0개)
- 워크벤치 메뉴: 활성 워크벤치 기반 동적 표시되는 9개 워크벤치별 메뉴 그룹 (Part/PartDesign/Sketcher/Mesh/TechDraw/Assembly/Draft/Surface/FEM)
- 스케치 구속 시각화: 치수 라벨, 색상 코딩된 만족 상태, 스냅 인디케이터, B-스플라인 표시, 구성 기하 구분을 포함한 향상된 렌더링
- 컨텍스트 메뉴: 활성 워크벤치별 워크벤치 인식 연산, 모든 아이콘 버튼에 툴바 툴팁
- 메뉴 바: File/Edit/View/Tools/Help + 워크벤치별 메뉴를 포함한 완전한 메뉴 시스템
- 인터랙티브 작업 패널: 35개 `ActiveTask` 변형 (프리미티브 10 + PartDesign 11 + Draft 6 + Surface 2 + FEM 1 + Boolean 1 + Scale 1) — 툴바 버튼이 즉시 실행 대신 실시간 3D 프리뷰가 포함된 인라인 매개변수 패널을 열어 OK/Cancel로 확인 또는 취소
- 뷰어 파일 구조 (16개 파일): mod.rs (1096), menu.rs (903), toolbar.rs (2945), tree.rs (897), properties.rs (757), sketch_ui.rs (785), dialogs.rs (1577), overlays.rs (524), view_cube.rs (825), context_menu.rs (393), task_panel.rs (785), theme.rs (386), status_bar.rs (215), report.rs (278), app.rs (5585), render.rs (2009)

**V16 성능 및 품질 스프린트 (2026-04-01):**
- 병렬 불리언 연산: `boolean_op()`의 페이스 분류 루프에 rayon `par_iter()` 적용
- BVH 개선: `query_aabb_parallel()` (rayon), `build_sah()` (Surface Area Heuristic), `refit()` (동적 씬)
- NURBS 기저 캐싱: `BasisCache` LRU 캐시 (1024개 항목), `CachedNurbsCurve`, `NurbsSurface::evaluate_cached()`
- Python 바인딩 업데이트: Sprint 2/3 API (`make_cone/torus`, `PyAssembly`, `PyFem`), Python 통합 테스트 74개
- 테스트 확장: 1136개 → 1300개 (+164개), 컴파운드 연산/조인/서피스/어셈블리/FEM/메시/9개 파일 포맷 커버

**V32 스케치 인터랙션 & 고급 스냅 (2026-04-07):**
- 박스 선택: Select 도구에서 러버밴드 — 좌→우 (윈도우, 실선, 파랑) vs 우→좌 (크로싱, 파선, 초록); Ctrl 추가 선택; 점/선/원/호 선택
- 스냅 시각 표시: 캔버스 커서 마커 — X (일치/점), 파선 H/V 가이드라인, 삼각형 (중점), 사각형 (그리드), 원형 X (교차점, 주황)
- 더블클릭 제약조건 편집: `try_sketch_dimension_edit()`로 선택 엔티티의 치수 제약조건 감지, 팝업에 현재 값 표시; `edit_constraint_index`로 기존 값 직접 수정
- 커서 형상: 그리기 도구 십자, Select 호버 포인팅 핸드, 드래그 그래빙
- 중점 + 교차점 스냅: `snap_sketch_coords()`에 선 중점 + 선-선 교차 (매개변수 t/u ∈ [0,1]) 추가; `Intersection` 자동 제약조건 + 시각 표시

**V31 스케치→솔리드 파이프라인 & 치수 UX (2026-04-07):**
- 치수 입력 팝업: `DimensionPopup` + `DimensionKind` (7종 — Distance/Radius/Angle/Length/H-Dist/V-Dist/Diameter); 중앙 egui::Window에 DragValue + OK/Cancel; Enter 확인, Escape 취소; 값이 툴바 기본값에 저장
- 닫힌 프로파일 감지: `find_closed_loops()`로 선 인접성 탐색하여 닫힌 루프 감지; 반투명 초록 채움 (알파 30)으로 압출 가능 영역 하이라이트
- 압출 방향 화살표: 스케치 중심에서 작업 평면 법선 방향으로 초록 화살표 + 화살촉 + 거리 라벨; 닫힌 프로파일 존재 시에만 표시
- 스케치 축 레이블: 빨강 X / 초록 Y 축 화살표 + 텍스트 레이블 + 흰색 원점 표시

**V30 스케치 시각 개선 & 슬롯 도구 (2026-04-07):**
- 스케치 엔티티 호버 정보: 상태바에 `sketch_hover_info()`로 Point(x,y), Line(길이/각도), Circle(중심/반지름), Arc(중심/반지름/범위), Ellipse(중심/단축), B-Spline(차수/점수) 표시
- 슬롯 도구 업그레이드: 3클릭 흐름 (중심1, 중심2, 너비)으로 실제 스타디움 형상 생성 — 2개 평행선 + 2개 반원호 (닫힌 슬롯 지오메트리)
- 부드러운 B-스플라인 렌더링: `de_boor_eval()` + `clamped_uniform_knots()`로 4N+16 샘플에서 실제 B-스플라인 곡선 평가; 제어 다각형은 파선 + 다이아몬드 마커
- 스케치 툴바: Copy, Paste, Merge Pts 버튼 추가

**V29 스케치 도구 완성 & 유효성 검증 (2026-04-07):**
- B-스플라인 도구: ConvertToBSpline, IncreaseDegree, DecreaseDegree, InsertKnot 모두 `bspline_tools` 함수에 연결
- 외부 투영 + 카본 카피: 모델 꼭짓점 스케치 투영, 이전 스케치 현재에 복사
- 스케치 복사/붙여넣기: Ctrl+C/V, 중심 기준 상대 좌표 클립보드
- 점 병합: `SketchMergePoints` — 모든 엔티티 참조를 병합된 일치 점으로 재매핑
- 유효성 검증 오버레이: 매 프레임 `validate_sketch()`, 길이 0 선/근접 점/과잉 구속 경고 아이콘
- 스케치 스텁 제로 — 6개 액션 스텁 모두 실제 구현 교체

**V28 스케치 보조선 모드 & 자동 제약조건 (2026-04-07):**
- 보조선 지오메트리 토글: 선택 엔티티를 보조선/일반 간 전환; 전역 보조선 모드로 새 지오메트리를 보조선으로 표시 (파선 표시, 프로파일 제외)
- 대칭 지오메트리: `SketchMirrorGeometry`를 `mirror_elements()`에 연결 — 축 선 + 선택적 점
- 폴리라인 닫기: Enter/우클릭으로 폴리라인 루프 닫기 (3개 이상 점); Enter로 B-스플라인 확정
- 제약조건 색상 코딩 완성: 12개 제약조건 타입 모두 제약조건별 잔차 색상 (초록/노랑/빨강) 사용
- 자동 제약조건: `find_or_create_point()`로 근접 점 재사용 (일치 스냅); `apply_line_auto_constraints()`로 축 근접 선에 H/V 추가; 사각형은 4변 모두에 H/V 자동 추가

**V27 스케치 정밀 편집 (2026-04-06):**
- 제약조건 인식 드래그: 단일 점 드래그 시 `drag_solve()`로 제약조건 유지; 솔버 미수렴 시 원시 이동 폴백; 다중 점 엔티티 드래그는 여전히 델타 기반
- 완전한 undo/redo: `SketchSnapshot`이 전체 `Sketch` 클론 저장; undo와 redo 모두 삭제/제약조건 변경 포함 완전한 상태 복원
- 인터랙티브 trim/split/extend: `SketchTrimEdge` (2선 → 교차점에서 트리밍), `SketchSplitEdge` (1선 → t=0.5에서 분할), `SketchExtendEdge` (1선 → 50% 연장)
- 필릿/챔퍼 코너: `SketchFilletCorner`/`SketchChamferCorner` — 꼭짓점 공유하는 2선에 적용
- Ctrl+A 전체 선택: 스케치 모드에서 모든 엔티티 선택; 스케치 밖에서는 글로벌 SelectAll로 전달
- 설정 가능한 그리드 간격: `SketchMode`의 `grid_spacing` 필드, 툴바 DragValue (0.1–10.0), 그리드 렌더링과 스냅에 반영

**V26 스케치 고급 편집 (2026-04-06):**
- 엔티티 드래그: 선/원/호 클릭+드래그 시 모든 구성 점을 한 단위로 이동; `entity_drag_points()`로 인덱스 수집; `drag_origin`을 통한 델타 기반 이동
- 제약조건 솔버 피드백: `constraint_residuals()`로 제약조건별 L2 노름 계산; 표시기 초록/노랑/빨강 색상 코딩; 위반 시 배너 빨간색
- 스케치 재편집: `last_sketch`에 닫을 때 데이터 저장; `EditSketch` 액션으로 Select 모드에서 재오픈; 툴바에 "Edit Sketch" 버튼
- 수치 제약조건 입력: 툴바에 Distance/Angle/Radius DragValue 인라인 입력

**V25: 성능, 테스트 & Python 스프린트 (2026-04-08):**
- 병렬 불리언: `boolean_op()`의 면 분류 루프에 `rayon::par_iter()` 적용 — 4코어 이상 환경에서 유의미한 속도 향상
- 병렬 패턴: `linear_pattern()`과 `circular_pattern()` 복제 루프를 rayon으로 병렬화
- BVH: `query_nearest()` (단일 최근접 AABB 항목) 및 `query_ray()` (레이와 교차하는 모든 항목) `geometry/bvh.rs`의 `Bvh`에 추가
- BVH 할당: 스택 할당 노드 탐색 버퍼 재사용으로 쿼리당 힙 할당 감소
- NURBS 캐싱: `BasisCache` LRU 캐시(1024개 항목), `CachedNurbsCurve`, `NurbsSurface::evaluate_cached()` (`bspline_basis.rs`)
- 벤치마크: 4개 신규 — `bench_parallel_boolean`, `bench_bvh_query_nearest`, `bench_bvh_query_ray`, `bench_pattern_parallel` (총 25개)
- 스트레스 테스트: `stress_tests.rs`에 18개 신규 — 다중 불리언 체인, 50개 파트 어셈블리, 30개 이상 제약조건 스케치, 64인스턴스 패턴, FEM 품질, surface 체인, I/O 에지 케이스 (총 67개)
- Python: `PyAssembly` (5개 메서드), `PyFem` (4개 메서드), draft/surface/compound 연산 바인딩; Python 통합 테스트 74개
- I/O: 라운드트립, 빈 모델, 100K+ 메시, 포맷별 에지 케이스 테스트

**V25 스케치 인터랙티브 편집 (2026-04-06):**
- 점 드래그: Select 모드에서 클릭+드래그로 점 이동 (스냅 적용), 누를 때 undo 스냅샷, 이동 없으면 취소
- 호버 프리셀렉션: CursorMoved에서 `hit_test_sketch()`로 `hovered_entity` 업데이트, 초록색 렌더링 (rgb 100,255,150)
- 다시 실행: Ctrl+Shift+Z로 `redo_stack`에서 복원; Escape는 대기 중인 지오메트리 먼저 초기화 후 스케치 취소
- 우클릭 컨텍스트 메뉴: `draw_sketch_context_menu()` 팝업에 Delete/Horizontal/Vertical/Fixed/Select All/Clear Selection; 선 선택 시에만 제약조건 항목 표시
- DOF 표시기: `degrees_of_freedom()` (제약조건 타입별 가중치)를 배너와 상태 바에 표시; 완전 구속 시 배너 초록색; 선택 수를 배너 + 상태 바에 표시

**향후 핵심 영역:**
| 우선순위 | 초점 | 주요 항목 |
|----------|------|-----------|
| 높음 | 성능 | 대형 어셈블리 최적화, GPU 테셀레이션 |
| 중간 | 실전 테스트 | 복잡 모델 검증, STEP 라운드트립 정확도 |
| 중간 | 문서화 | API 레퍼런스 생성, 튜토리얼 |

---

## 15. 확장 아키텍처

CADKernel은 서드파티 확장이 명령을 등록하고 모델링 파이프라인에 훅을 걸 수 있는 플러그인 시스템을 지원합니다.

### Plugin 트레이트

```rust
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn init(&mut self) -> KernelResult<()>;
    fn shutdown(&mut self);
    fn commands(&self) -> Vec<String>;
    fn execute_command(&self, cmd: &str, args: &[String]) -> KernelResult<String>;
}
```

### PluginRegistry

`PluginRegistry`는 `Vec<Box<dyn Plugin>>`을 소유하며 다음 메서드를 제공합니다:

| 메서드 | 설명 |
|--------|------|
| `register(plugin)` | `init()` 호출 후 플러그인 저장 |
| `unregister(name)` | `shutdown()` 호출 후 플러그인 제거 |
| `execute_command(name, cmd, args)` | 지정한 플러그인에 명령 디스패치 |
| `list_plugins()` | 등록된 모든 플러그인 이름 반환 |

### 내장 예제 플러그인

| 플러그인 | 명령 | 목적 |
|----------|------|------|
| `ValidationPlugin` | `validate` | 활성 모델에 `check_geometry` + `check_watertight` 실행 |
| `AutoNamingPlugin` | `auto_name` | 이름 없는 솔리드에 순차 이름 부여 |
| `StatisticsPlugin` | `stats` | V/E/F 개수, 체적, 표면적 보고 |

---

## 16. MCP 통합

CADKernel은 JSON-RPC 2.0을 통해 커널을 AI 어시스턴트에 노출하는 **Model Context Protocol(MCP)** 서버를 구현합니다.

### 프로토콜

- 전송: stdin/stdout (라인 구분 JSON)
- 요청 형식: `{ "jsonrpc": "2.0", "id": N, "method": "<tool>", "params": { ... } }`
- 응답 형식: `{ "jsonrpc": "2.0", "id": N, "result": { ... } }` 또는 `"error": { ... }`

### McpServer

```rust
pub struct McpServer {
    model: BRepModel,
}

impl McpServer {
    pub fn handle_request(&mut self, request: &str) -> String;
}
```

### 지원 도구 (8개)

| 도구 | 파라미터 | 설명 |
|------|----------|------|
| `create_primitive` | `shape`, `params` | 치수를 지정한 Box/Cylinder/Sphere/Cone/Torus |
| `boolean_operation` | `op`, `target`, `tool` | 솔리드 이름으로 Union/Subtract/Intersect |
| `transform` | `solid`, `tx`, `ty`, `tz`, `rx`, `ry`, `rz` | 솔리드 이동 + 회전 |
| `query_model` | — | 모든 솔리드 이름 및 개수 반환 |
| `measure` | `solid` | 체적, 표면적, 바운딩 박스 |
| `export_model` | `format`, `path` | STL/OBJ/glTF/STEP/BREP 내보내기 |
| `delete_solid` | `solid` | 모델에서 지정한 솔리드 제거 |
| `list_solids` | — | 솔리드 이름 문자열 배열 |

### 사용법

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"create_primitive","params":{"shape":"box","params":{"x":10,"y":10,"z":10}}}' | cadkernel --mcp
```

---

## 17. 스크립팅

CADKernel은 자동화 및 배치 처리를 위해 Lua 스크립팅 엔진을 임베드합니다.

### 보안 샌드박스

스크립팅 엔진은 초기화 시 위험한 Lua 표준 라이브러리 전역을 제거합니다:
`os`, `io`, `require`, `dofile`, `loadfile`, `package`. 스크립트는 `string`,
`table`, `math` 및 `cad.*` API에만 접근 가능합니다. 파일 I/O는 검증된 경로 내에서
동작하는 `cad.export()`와 `cad.import()` 함수를 통해서만 사용할 수 있습니다.

### API 범위

```lua
-- 프리미티브
local s = cad.make_box(10, 10, 10)
local c = cad.make_cylinder(5, 20)

-- 불리언
local result = cad.union(s, c)
local result = cad.subtract(s, c)
local result = cad.intersect(s, c)

-- 변환
cad.translate(s, 10, 0, 0)
cad.rotate(s, 0, 0, 90)   -- Z축 기준 각도(도)

-- 피처
cad.fillet(s, edge_index, 1.0)
cad.chamfer(s, edge_index, 1.0)
cad.extrude(profile, 20.0)

-- I/O
cad.export(s, "output.stl")
cad.export(s, "output.step")

-- 쿼리
local vol = cad.volume(s)
local area = cad.surface_area(s)
print(cad.list_solids())
```

### 실행 API

```rust
pub struct LuaEngine {
    lua: mlua::Lua,
    model: Arc<Mutex<BRepModel>>,
}

impl LuaEngine {
    pub fn execute(&self, script: &str) -> KernelResult<String>;
    pub fn execute_file(&self, path: &Path) -> KernelResult<String>;
}
```

### 스크립트 실행

```bash
cadkernel --script my_model.lua
cadkernel --script batch_export.lua --output /tmp/
```

---

## 18. CI/CD

### CI 워크플로우 (`.github/workflows/ci.yml`)

`main` 브랜치 푸시 및 모든 풀 리퀘스트에서 실행됩니다.

**매트릭스**: `ubuntu-latest`, `macos-latest`, `windows-latest`

**플랫폼별 단계**:
1. `actions/checkout@v4`
2. `dtolnay/rust-toolchain@stable` (components: rustfmt, clippy)
3. `actions/cache@v4` — `~/.cargo/registry/`, `~/.cargo/git/`, `target/`
4. `cargo build --workspace`
5. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
6. `cargo test --workspace`

**추가 잡**:
- `python-bindings` (ubuntu-latest) — `cargo build/test --manifest-path crates/python/Cargo.toml`로 `crates/python`을 별도 빌드 및 테스트

캐시 키는 OS별 `Cargo.lock` 해시를 기반으로 하며, OS 수준의 폴백 복원 키를 사용합니다.

### 릴리즈 워크플로우 (`.github/workflows/release.yml`)

`v*` 형식의 태그 푸시(예: `v0.2.0`)에서 실행됩니다.

**빌드 매트릭스**:

| OS | 타깃 |
|----|------|
| ubuntu-latest | x86_64-unknown-linux-gnu |
| macos-latest | x86_64-apple-darwin |
| macos-latest | aarch64-apple-darwin |
| windows-latest | x86_64-pc-windows-msvc |

**타깃별 단계**:
1. 체크아웃 + 타깃 포함 툴체인 설치
2. Cargo 레지스트리 + 빌드 캐시 (타깃 + `Cargo.lock` 기반)
3. `cargo build --release --target <target>`
4. `actions/upload-artifact@v4` — 아티팩트 이름 `cadkernel-<target>`

**릴리즈 잡** (모든 빌드 완료 후):
1. `actions/download-artifact@v4` — 전체 플랫폼 아티팩트 다운로드
2. `softprops/action-gh-release@v2` — 모든 바이너리 포함 GitHub 릴리즈 생성, 릴리즈 노트 자동 생성

### Python 휠 워크플로우 (`.github/workflows/python-release.yml`)

`v*` 형식의 태그 푸시 및 수동 디스패치에서 실행됩니다.

**빌드 매트릭스**:

| 잡 | OS | 타깃 |
|-----|-----|------|
| linux | ubuntu-latest | x86_64, aarch64 |
| macos | macos-13 / macos-latest | x86_64-apple-darwin, aarch64-apple-darwin |
| windows | windows-latest | x86_64-pc-windows-msvc |
| sdist | ubuntu-latest | 소스 배포 |

**타깃별 단계**:
1. 체크아웃 + `actions/setup-python@v5` (Python 3.13)
2. `PyO3/maturin-action@v1` — `crates/python/dist`에 휠 빌드
3. `actions/upload-artifact@v4` — 플랫폼별 아티팩트

**퍼블리시 잡** (모든 빌드 완료 후, 태그만):
1. 전체 휠 아티팩트 다운로드
2. `pypa/gh-action-pypi-publish@release/v1` — 신뢰할 수 있는 퍼블리셔를 사용하여 PyPI에 배포

### 로컬 실행

```bash
# CI 전체 검사 (GitHub Actions와 동일)
cargo build --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

# 현재 플랫폼용 릴리즈 빌드
cargo build --release
```

---

## 19. Lua 콘솔

Lua 콘솔은 Report 패널(하단 독) 안의 대화형 탭입니다. GUI를 벗어나지 않고 현재 라이브 모델에 Lua 코드를 실행할 수 있습니다.

### 레이아웃

```
[ Report | Lua Console ]       ← 탭 스트립

┌─────────────────────────────────────────────────────┐
│ > local s = cad.make_box(10,10,10)                  │  ← 출력 영역 (스크롤 가능)
│ > cad.export(s, "/tmp/out.stl")                     │
│ Exported: /tmp/out.stl                              │
└─────────────────────────────────────────────────────┘
[ 입력 필드 ............................... ] [실행]   ← 입력 행
```

### GuiAction 변형

| 변형 | 트리거 | 설명 |
|------|--------|------|
| `ExecuteLuaCode(String)` | 실행 버튼 / Enter | 코드 문자열을 `LuaEngine::execute()`에 전달 |
| `ExecuteLuaFile(PathBuf)` | 파일 선택기 | 파일 로드 후 `LuaEngine::execute_file()` 호출 |
| `ClearLuaConsole` | 지우기 버튼 | 출력 버퍼 비우기 |

### 이력 탐색

- 입력 필드에서 위/아래 방향키로 이전 명령어를 순환합니다 (현재 세션).
- 이력은 재시작 시 유지되지 않습니다.

### 에러 표시

`LuaEngine::execute()`의 에러는 출력 영역에 빨간색으로 표시되며 `Error: ` 접두사가 붙습니다.

---

## 20. 프로젝트 템플릿

프로젝트 템플릿은 애플리케이션과 함께 제공되는 미리 만들어진 `.cadk` 파일입니다. **파일 → 템플릿으로 새 파일 만들기**를 통해 접근할 수 있습니다.

### 사용 가능한 템플릿

| 파일 | 내용 | 용도 |
|------|------|------|
| `template_empty.cadk` | 빈 모델, 올바른 스키마 | 처음부터 새로 설계 시작 |
| `template_single_box.cadk` | 10×10×10 mm 박스 | 빠른 지오메트리 테스트 또는 튜토리얼 |
| `template_basic_assembly.cadk` | 두 컴포넌트 어셈블리 골격 | 어셈블리 워크플로 시작점 |
| `template_mechanical_part.cadk` | 필렛이 있는 플랜지 브래킷 | 기계 설계 참조 |
| `template_gear_demo.cadk` | 파라메트릭 스퍼 기어 (m=2, z=20) | 기어 설계 및 수정 |

### 템플릿 위치

템플릿은 저장소 루트의 `templates/` 디렉토리에 저장되며, 빌드 스크립트를 통해 릴리즈 바이너리에 포함됩니다. 런타임에는 다음을 통해 접근합니다:

```rust
cadkernel_io::templates::list_templates() -> Vec<TemplateInfo>
cadkernel_io::templates::load_template(name: &str) -> KernelResult<BRepModel>
```

---

## 21. 편의 API

`cadkernel-modeling`은 간결한 스크립팅과 테스트 코드를 위한 `quick_*` 함수 집합을 제공합니다. 이 함수들은 내부 검증 생성자를 더 간단한 위치 인수 시그니처로 감쌉니다.

### 프리미티브 생성자

```rust
use cadkernel_modeling::quick::*;

let b  = quick_box(10.0, 20.0, 5.0)?;          // x, y, z
let cy = quick_cylinder(5.0, 20.0)?;            // 반지름, 높이
let sp = quick_sphere(8.0)?;                    // 반지름
let co = quick_cone(4.0, 2.0, 15.0)?;           // 아래 반지름, 위 반지름, 높이
let to = quick_torus(10.0, 2.0)?;               // 대반지름, 소반지름
```

### 불리언 헬퍼

```rust
let result = quick_union(solid_a, solid_b)?;
let result = quick_subtract(solid_a, tool)?;
let result = quick_intersect(solid_a, solid_b)?;
```

### 질량 속성 조회

```rust
let vol  = quick_volume(&solid)?;               // f64 (mm³)
let area = quick_area(&solid)?;                 // f64 (mm²)
let cen  = quick_centroid(&solid)?;             // Point3
let bb   = quick_bbox(&solid)?;                 // BoundingBox
```

모든 함수는 `KernelResult<T>`를 반환하며, 함수 이름과 범위를 벗어난 파라미터 값을 포함하는 설명적인 에러 메시지를 제공합니다.

---

## 22. 예제 스크립트

예제 스크립트는 저장소 루트의 `examples/` 디렉토리에 있습니다.

### 디렉토리 구조

```
examples/
├── lua/
│   ├── hello_cad.lua            — 지오메트리 생성 + STL 내보내기
│   ├── boolean_operations.lua   — union / subtract / intersect 파이프라인
│   ├── parametric_part.lua      — 치수 기반 브래킷 모델
│   ├── batch_export.lua         — 하나의 솔리드를 STL + OBJ + STEP으로 내보내기
│   └── assembly.lua             — 두 부품 어셈블리 (제약 조건 포함)
├── python/
│   ├── basic_modeling.py        — PyO3 프리미티브 + 불리언
│   └── batch_analysis.py        — 파일 목록에 대한 질량 속성 루프
└── mcp/
    └── session.json             — 주석이 달린 JSON-RPC 2.0 세션 트랜스크립트
```

### Lua 스크립트 실행

**GUI에서**: Report 패널의 Lua Console 탭 열기 → 폴더 아이콘 클릭 → `.lua` 파일 선택. 스크립트가 현재 라이브 모델에 대해 실행됩니다.

**CLI에서**:

```bash
cadkernel --script examples/lua/hello_cad.lua
cadkernel --script examples/lua/batch_export.lua --output /tmp/
```

### Python 스크립트 실행

```bash
# cadkernel Python 패키지 필요 (maturin 빌드)
cd crates/python
PYO3_PYTHON=/usr/bin/python3 cargo build
maturin develop
python3 ../../examples/python/basic_modeling.py
```

### MCP 세션 재현

`examples/mcp/session.json`은 완전한 생성 → 불리언 → 내보내기 워크플로에 대한 요청/응답 쌍을 보여주는 주석 트랜스크립트입니다. 실행 중인 MCP 서버에 대해 재현할 수 있습니다:

```bash
cadkernel --mcp < examples/mcp/session.json
```

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
