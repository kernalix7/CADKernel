# Crate: cadkernel-geometry

> **역할**: 매개변수 커브와 서피스 트레이트 정의 및 구현, 교차 연산, 2D 오프셋, 적응형 테셀레이션  
> **의존성**: `cadkernel-math`, `cadkernel-core`  
> **경로**: `crates/geometry/`

## Curve 트레이트

```rust
pub trait Curve: Send + Sync {
    // 필수 구현
    fn point_at(&self, t: f64) -> Point3;
    fn tangent_at(&self, t: f64) -> Vec3;
    fn domain(&self) -> (f64, f64);
    fn length(&self) -> f64;
    fn is_closed(&self) -> bool;

    // 기본 구현 (유한차분 기반)
    fn second_derivative_at(&self, t: f64) -> Vec3;
    fn curvature_at(&self, t: f64) -> f64;
    fn reversed(&self) -> Box<dyn Curve>;
    fn project_point(&self, point: Point3) -> f64;
    fn bounding_box(&self) -> BoundingBox;
}
```

## 구현된 커브

| 구조체 | Copy | PartialEq | 생성자 반환 | 도메인 |
|--------|:----:|:---------:|:----------:|--------|
| `Line` | ✅ | ✅ | `Line::new()` | `[0, ∞)` |
| `LineSegment` | ✅ | ✅ | `LineSegment::new()` | `[0, 1]` |
| `Arc` | ✅ | ✅ | `Arc::new()` | `[start, end]` |
| `Circle` | ✅ | ✅ | `KernelResult<Self>` | `[0, 2π]` |
| `Ellipse` | ✅ | ✅ | `Ellipse::new()` | `[0, 2π]` |
| `NurbsCurve` | — | — | `KernelResult<Self>` | `[knot_min, knot_max]` |

### 생성자 검증

`Circle::new()`와 `NurbsCurve::new()`는 입력을 검증하고 `KernelResult`를 반환합니다:

```rust
let circle = Circle::new(center, normal, radius)?;  // 영벡터 법선 거부
let nurbs = NurbsCurve::new(degree, points, weights, knots)?;  // 배열 크기 검증
```

## Surface 트레이트

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

## 구현된 서피스

| 구조체 | Copy | PartialEq | 생성자 반환 |
|--------|:----:|:---------:|:----------:|
| `Plane` | ✅ | ✅ | `KernelResult<Self>` |
| `Cylinder` | ✅ | ✅ | `KernelResult<Self>` |
| `Sphere` | ✅ | ✅ | `Sphere::new()` |
| `Cone` | ✅ | ✅ | `Cone::new()` |
| `Torus` | ✅ | ✅ | `Torus::new()` |
| `NurbsSurface` | — | — | `KernelResult<Self>` |

## Plane 향상 *(Phase 9)*

Plane 서피스에 6개의 편의 메서드가 추가되었습니다.

| 메서드 | 시그니처 | 설명 |
|--------|----------|------|
| `from_three_points` | `(p1, p2, p3) → KernelResult<Plane>` | 세 점에서 평면 생성 |
| `signed_distance` | `(&self, point) → f64` | 부호 있는 거리 (법선 방향 +) |
| `distance` | `(&self, point) → f64` | 절대 거리 |
| `project_point` | `(&self, point) → Point3` | 점을 평면에 투영 |
| `is_above` | `(&self, point) → bool` | 법선 방향 위에 있는지 |
| `contains_point` | `(&self, point) → bool` | 점이 평면 위에 있는지 (tolerance 내) |

```rust
use cadkernel_geometry::prelude::*;

// 세 점에서 평면 생성
let plane = Plane::from_three_points(
    Point3::new(0.0, 0.0, 0.0),
    Point3::new(1.0, 0.0, 0.0),
    Point3::new(0.0, 1.0, 0.0),
)?;

// 부호 있는 거리
let sd = plane.signed_distance(Point3::new(0.0, 0.0, 5.0));
assert!(sd > 0.0);  // 법선 방향

// 투영
let proj = plane.project_point(Point3::new(3.0, 4.0, 7.0));
assert!(plane.contains_point(proj));

// 위치 판별
assert!(plane.is_above(Point3::new(0.0, 0.0, 1.0)));
assert!(!plane.is_above(Point3::new(0.0, 0.0, -1.0)));
```

## 2D 커브 오프셋 *(Phase 14)*

CNC 가공이나 스케치 작업을 위한 2D 평행 오프셋 연산을 제공합니다.

```rust
use cadkernel_geometry::offset::*;
use cadkernel_math::Point2;

// 폴리라인 오프셋 (열린 경로)
let polyline = vec![
    Point2::new(0.0, 0.0),
    Point2::new(10.0, 0.0),
    Point2::new(10.0, 5.0),
];
let offset = offset_polyline_2d(&polyline, 1.0);  // 좌측으로 1.0mm 오프셋

// 폴리곤 오프셋 (닫힌 경로)
let polygon = vec![
    Point2::new(0.0, 0.0),
    Point2::new(10.0, 0.0),
    Point2::new(10.0, 10.0),
    Point2::new(0.0, 10.0),
];
let offset = offset_polygon_2d(&polygon, -0.5);  // 안쪽으로 0.5mm 오프셋
```

**양수 오프셋**: CCW 기준 좌측(외부), **음수 오프셋**: 우측(내부).

## 적응형 테셀레이션 *(Phase 14)*

현오차(chord tolerance)와 각도(angle tolerance) 기반의 적응형 세분화로 커브/서피스를 테셀레이션합니다.

### TessellationOptions

```rust
use cadkernel_geometry::tessellate::*;

let opts = TessellationOptions {
    chord_tolerance: 0.01,     // 현오차 허용량 (mm)
    angle_tolerance: 15.0_f64.to_radians(), // 각도 허용량
    min_segments: 4,           // 최소 세그먼트 수
    max_depth: 8,              // 최대 재귀 깊이
};
```

### 커브 테셀레이션

```rust
// 함수 방식
let points: Vec<Point3> = adaptive_tessellate_curve(&circle, &opts);

// 트레이트 방식 (TessellateCurve 확장 트레이트)
use cadkernel_geometry::TessellateCurve;
let points = circle.tessellate(&opts);
```

### 서피스 테셀레이션

```rust
// 함수 방식
let tess_mesh: TessMesh = adaptive_tessellate_surface(&sphere, &opts);
// tess_mesh.vertices: Vec<Point3>
// tess_mesh.indices: Vec<[u32; 3]>

// 트레이트 방식 (TessellateSurface 확장 트레이트)
use cadkernel_geometry::TessellateSurface;
let tess_mesh = sphere.tessellate(&opts);
```

### TessMesh

```rust
pub struct TessMesh {
    pub vertices: Vec<Point3>,
    pub indices: Vec<[u32; 3]>,
}
```

## NURBS 고급 기능 *(Phase 17)*

형상 보존 NURBS 정밀화 연산.

### 노트 삽입 (Boehm 알고리즘)

기존 형상을 변경하지 않으면서 노트 벡터에 새 노트를 삽입합니다.

```rust
let mut nurbs = NurbsCurve::new(3, points, weights, knots)?;

// 파라미터 t=0.5 위치에 노트 삽입
nurbs.insert_knot(0.5);

// 형상은 유지되면서 제어점이 추가됨
```

### 차수 승격

기존 형상을 유지하면서 NURBS 커브의 차수를 올립니다.

```rust
let mut nurbs = NurbsCurve::new(2, points, weights, knots)?;

// 2차 → 3차로 승격
nurbs.elevate_degree();
// degree: 3, 제어점과 노트가 그에 맞게 확장됨
```

## 교차(Intersect) 모듈

### Surface-Surface Intersection

| 조합 | 결과 타입 |
|------|----------|
| Plane-Plane | Line, Coincident, Empty |
| Plane-Sphere | Circle, Point, Empty |
| Plane-Cylinder | `IntersectionEllipse`, Line pair, Empty |
| Sphere-Sphere | Circle, Point, Coincident, Empty |

### Line-Surface Intersection

| 조합 | 결과 타입 |
|------|----------|
| Line-Plane | RayHit, Empty (parallel) |
| Line-Sphere | 0, 1, or 2 RayHit |
| Line-Cylinder | 0, 1, or 2 RayHit |

### 결과 타입

```rust
pub enum SsiResult {
    Empty,
    Point(Point3),
    Line { origin: Point3, direction: Vec3 },
    Circle { center: Point3, normal: Vec3, radius: f64 },
    Ellipse(IntersectionEllipse),
    Coincident,
}

pub struct RayHit {
    pub t: f64,        // 레이 파라미터
    pub point: Point3,  // 교차점
}
```

> **이름 규칙**: 교차 결과 타원 = `IntersectionEllipse` (커브 `Ellipse`와 구분)

## 2D 커브 오프셋 *(Phase 14)*

2D 폴리라인/폴리곤을 일정 거리만큼 평행 오프셋합니다. CNC 툴패스 생성이나 스케치 오프셋에 활용됩니다.

```rust
use cadkernel_geometry::offset::*;
use cadkernel_math::Point2;

// 열린 폴리라인 오프셋 (양수 = 왼쪽/CCW 방향)
let polyline = vec![
    Point2::new(0.0, 0.0),
    Point2::new(2.0, 0.0),
    Point2::new(2.0, 2.0),
];
let offset = offset_polyline_2d(&polyline, 1.0);

// 닫힌 폴리곤 오프셋 (양수 = 외곽 확장, 음수 = 내곽 축소)
let square = vec![
    Point2::new(0.0, 0.0),
    Point2::new(1.0, 0.0),
    Point2::new(1.0, 1.0),
    Point2::new(0.0, 1.0),
];
let expanded = offset_polygon_2d(&square, 0.5);    // 1.5x1.5 사각형
let shrunk  = offset_polygon_2d(&square, -0.1);    // 0.8x0.8 사각형
```

## 적응형 테셀레이션 *(Phase 14)*

곡률이 높은 곳에서는 밀도 높게, 평탄한 곳에서는 성글게 분할하는 적응형 테셀레이션입니다.

### TessellationOptions

```rust
let opts = TessellationOptions {
    chord_tolerance: 0.01,   // 최대 현오차
    angle_tolerance: 0.1,    // 최대 각도 편차 (rad)
    min_segments: 4,         // 초기 최소 세그먼트
    max_depth: 10,           // 최대 재귀 깊이
};
```

### 커브 테셀레이션

```rust
// 함수형 API
let pts = adaptive_tessellate_curve(
    |t| curve.point_at(t),
    |t| curve.tangent_at(t),
    t_start, t_end,
    &opts,
);

// 확장 트레이트 API
let pts = my_circle.tessellate_adaptive(&opts);
```

### 서피스 테셀레이션

```rust
// 함수형 API
let mesh: TessMesh = adaptive_tessellate_surface(
    |u, v| surface.point_at(u, v),
    |u, v| surface.normal_at(u, v),
    (u0, u1), (v0, v1),
    &opts,
);

// 확장 트레이트 API
let mesh = my_sphere.tessellate_adaptive(&opts);
```

## NURBS 고급 기능 *(Phase 17)*

### 노트 삽입 (Boehm 알고리즘)

커브 형상을 정확히 보존하면서 제어점을 하나 추가합니다.

```rust
let refined = nurbs_curve.insert_knot(0.5)?;
assert_eq!(refined.control_point_count(), nurbs_curve.control_point_count() + 1);
```

### 차수 승격

커브 형상을 정확히 보존하면서 차수를 1 올립니다.

```rust
let elevated = nurbs_curve.elevate_degree()?;
assert_eq!(elevated.degree(), nurbs_curve.degree() + 1);
```

## 파일 구조

```
crates/geometry/src/
├── lib.rs
├── prelude.rs
├── offset.rs        ← offset_polyline_2d, offset_polygon_2d
├── tessellate.rs    ← TessellationOptions, adaptive_tessellate_*, TessMesh, extension traits
├── curve/
│   ├── mod.rs       ← Curve 트레이트 (Send + Sync) + 상수
│   ├── line.rs      ← Line, LineSegment
│   ├── arc.rs       ← Arc
│   ├── circle.rs    ← Circle
│   ├── ellipse.rs   ← Ellipse
│   └── nurbs.rs     ← NurbsCurve (+ insert_knot, elevate_degree)
├── surface/
│   ├── mod.rs       ← Surface 트레이트 (Send + Sync) + 상수
│   ├── plane.rs     ← Plane
│   ├── cylinder.rs  ← Cylinder
│   ├── sphere.rs    ← Sphere
│   ├── cone.rs      ← Cone
│   ├── torus.rs     ← Torus
│   └── nurbs.rs     ← NurbsSurface
└── intersect/
    ├── mod.rs
    ├── types.rs     ← SsiResult, IntersectionEllipse, RayHit
    ├── plane_plane.rs
    ├── plane_sphere.rs
    ├── plane_cylinder.rs
    ├── sphere_sphere.rs
    └── line_surface.rs
```
