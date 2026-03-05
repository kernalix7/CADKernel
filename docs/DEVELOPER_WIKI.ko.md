# CADKernel Developer Wiki

> **버전**: 0.1.0 (pre-alpha)  
> **최종 업데이트**: 2026-02-26  
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

#### 14가지 제약 조건

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

#### Feature Operations

| 함수 | 파라미터 | 반환 |
|------|----------|------|
| `extrude(&mut model, &profile, direction, distance)` | 3D 점 배열 + 방향 + 거리 | `KernelResult<ExtrudeResult>` |
| `revolve(&mut model, &profile, axis_origin, axis_dir, angle, segments)` | 3D 점 배열 + 회전축 | `KernelResult<RevolveResult>` |

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

#### 테셀레이션

```rust
let mesh = tessellate_solid(&model, solid_handle);
let mesh = tessellate_face(&model, face_handle);
// Mesh { vertices: Vec<Point3>, triangles: Vec<Triangle> }
```

#### 내보내기

```rust
// STL
let ascii = write_stl_ascii(&mesh, "name");
let binary: Vec<u8> = write_stl_binary(&mesh);
export_stl_ascii(&mesh, "output.stl", "name")?;  // 파일로 직접

// OBJ
let obj = write_obj(&mesh);
export_obj(&mesh, "output.obj")?;
```

### 3.8 cadkernel-viewer

네이티브 데스크톱 GUI 애플리케이션 (egui 0.31 + wgpu 24.x + winit 0.30).

**모듈**: `app.rs` (상태 + 이벤트 루프), `render.rs` (GPU + 카메라 + 수학), `gui.rs` (UI 패널 + ViewCube), `nav.rs` (마우스 내비게이션 프리셋).

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
| 2D Sketch system | ✅ | Point, Line, Arc, Circle + 14 constraints |
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
| cadkernel-geometry | 56 |
| cadkernel-topology | 29 |
| cadkernel-modeling | 28 |
| cadkernel-sketch | 10 |
| cadkernel-io | 34 |
| Doctests | 3 |
| **합계** | **209** |

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

## 9. 다음 단계 (Phase 5+)

### Phase 5: Advanced Geometry

| 항목 | 우선순위 | 설명 |
|------|:--------:|------|
| Fillet / Chamfer | 높음 | 모서리 라운딩/면취 |
| Shell (offset) | 높음 | 서피스 오프셋 |
| Mass properties | 중간 | 부피, 면적, 무게중심, 관성 모멘트 |
| Curved tessellation | 중간 | NURBS 서피스 기반 적응형 테셀레이션 |
| Sweep / Loft | 중간 | 경로 따라 스윕, 단면 간 로프트 |

### Phase 6: File I/O 확장

| 항목 | 우선순위 |
|------|:--------:|
| STEP AP203 import/export | 높음 |
| IGES import/export | 중간 |
| glTF/GLB export | 중간 |
| 3MF export | 낮음 |
| Native `.cadk` 형식 | 중간 |

### Phase 7: Framework Layer

| 항목 | 설명 |
|------|------|
| Undo/Redo | 명령 패턴 기반 히스토리 |
| Parametric rebuild | Tag 기반 모델 재구축 엔진 |
| Assembly | 부품 참조 + 메이트 제약 |
| Material system | 재질 속성 + 밀도 |

---

## 10. 용어 사전

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
