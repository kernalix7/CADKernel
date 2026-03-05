# Crate: cadkernel-math

> **역할**: CAD 연산에 필요한 모든 수학 기본 타입  
> **의존성**: `nalgebra` 0.33, `glam` 0.29  
> **경로**: `crates/math/`

## 타입 요약

| 타입 | Copy | Default | Display | 설명 |
|------|:----:|:-------:|:-------:|------|
| `Vec2` | ✅ | `ZERO` | `(x, y)` | 2D 벡터 |
| `Vec3` | ✅ | `ZERO` | `(x, y, z)` | 3D 벡터 |
| `Vec4` | ✅ | `ZERO` | `(x, y, z, w)` | 동차 좌표 |
| `Point2` | ✅ | `ORIGIN` | `(x, y)` | 2D 점 |
| `Point3` | ✅ | `ORIGIN` | `(x, y, z)` | 3D 점 |
| `Mat3` | ✅ | `IDENTITY` | — | 3×3 행렬 (nalgebra 래퍼) |
| `Mat4` | ✅ | `IDENTITY` | — | 4×4 행렬 (nalgebra 래퍼) |
| `Transform` | ✅ | `IDENTITY` | 커스텀 | 어파인 변환 |
| `Quaternion` | ✅ | `IDENTITY` | `(w+xi+yj+zk)` | 단위 쿼터니언 |
| `Ray3` | ✅ | — | `Ray3(o→d)` | 3D 레이 |
| `BoundingBox` | ✅ | `empty()` | `BBox[min..max]` | AABB |
| `EPSILON` | — | — | — | 허용오차 `1e-9` |

## 연산자 매트릭스

### 벡터 연산자

| 연산 | Vec2 | Vec3 | Vec4 |
|------|:----:|:----:|:----:|
| `v + v` | ✅ | ✅ | — |
| `v - v` | ✅ | ✅ | — |
| `-v` | ✅ | ✅ | — |
| `v * f64` | ✅ | ✅ | — |
| `f64 * v` | ✅ | ✅ | ✅ |
| `v / f64` | ✅ | ✅ | — |
| `v += v` | ✅ | ✅ | — |
| `v -= v` | ✅ | ✅ | — |
| `v *= f64` | ✅ | ✅ | — |
| `v /= f64` | ✅ | ✅ | — |
| `.sum()` | ✅ | ✅ | — |

### 점-벡터 연산자

| 연산 | Point2 | Point3 | 결과 |
|------|:------:|:------:|------|
| `p + v` | ✅ | ✅ | Point |
| `p - v` | ✅ | ✅ | Point |
| `p - p` | ✅ | ✅ | Vec |
| `p += v` | ✅ | ✅ | — |
| `p -= v` | ✅ | ✅ | — |

### From 변환

| From → To | 지원 |
|-----------|:----:|
| `Vec3` → `Point3` | ✅ |
| `Point3` → `Vec3` | ✅ |
| `Vec2` → `Point2` | ✅ (via Point2) |
| `Point2` → `Vec2` | ✅ |
| `[f64; 3]` → `Vec3` | ✅ |
| `[f64; 3]` → `Point3` | ✅ |
| `(f64, f64, f64)` → `Vec3` | ✅ |
| `(f64, f64, f64)` → `Point3` | ✅ |
| `[f64; 2]` → `Vec2`/`Point2` | ✅ |
| `(f64, f64)` → `Vec2`/`Point2` | ✅ |

## Transform API

```rust
// 기본 변환
Transform::translation(tx, ty, tz)
Transform::uniform_scale(s)
Transform::scale(sx, sy, sz)
Transform::rotation_x(angle)
Transform::rotation_y(angle)
Transform::rotation_z(angle)

// 고급 변환
Transform::rotation_axis_angle(axis, angle)    // Rodrigues
Transform::from_quaternion(q)                   // 쿼터니언 → 변환
Transform::rotation_around_point(center, axis, angle)
Transform::mirror(plane_point, plane_normal)

// 합성
let t = Transform::translation(1.0, 0.0, 0.0)
    .then(Transform::rotation_z(PI / 2.0))
    .then(Transform::uniform_scale(2.0));

// 적용
let p = t.apply_point(Point3::ORIGIN);
let v = t.apply_vec(Vec3::X);
let inv = t.try_inverse();
```

## Quaternion API

```rust
Quaternion::IDENTITY
Quaternion::from_axis_angle(axis, angle)
q.to_axis_angle() → (Vec3, f64)
q.rotate_vec(v) → Vec3
q.slerp(other, t) → Quaternion
q.conjugate()
q.normalized()
q * q   // Hamilton product
```

## 유틸리티 함수 *(Phase 9)*

거리, 각도, 투영, 보간, 면적 계산을 위한 11개 유틸리티 함수.

| 함수 | 시그니처 | 설명 |
|------|----------|------|
| `distance_point_line` | `(point, line_origin, line_dir) → f64` | 점과 직선 사이 거리 |
| `distance` | `(a, b) → f64` | 두 점 사이 유클리드 거리 |
| `angle_between` | `(v1, v2) → f64` | 두 3D 벡터 사이 각도 (라디안) |
| `angle_between_2d` | `(v1, v2) → f64` | 두 2D 벡터 사이 각도 (라디안) |
| `project_point_on_plane` | `(point, plane_origin, plane_normal) → Point3` | 점을 평면에 투영 |
| `project_point_on_line` | `(point, line_origin, line_dir) → Point3` | 점을 직선에 투영 |
| `lerp_point` | `(a, b, t) → Point3` | 두 3D 점 선형 보간 |
| `lerp_point_2d` | `(a, b, t) → Point2` | 두 2D 점 선형 보간 |
| `triangle_area` | `(a, b, c) → f64` | 삼각형 면적 |
| `polygon_area_2d` | `(points) → f64` | 2D 다각형 면적 (Shoelace 공식) |
| `is_ccw` | `(points) → bool` | 2D 점 리스트의 반시계방향 여부 |

```rust
use cadkernel_math::utils::*;

let d = distance(Point3::ORIGIN, Point3::new(3.0, 4.0, 0.0));
assert!((d - 5.0).abs() < 1e-9);

let angle = angle_between(Vec3::X, Vec3::Y);
assert!((angle - std::f64::consts::FRAC_PI_2).abs() < 1e-9);

let projected = project_point_on_plane(
    Point3::new(5.0, 5.0, 3.0),
    Point3::ORIGIN,
    Vec3::Z,
);
assert!((projected.z).abs() < 1e-9);

let area = triangle_area(
    Point3::new(0.0, 0.0, 0.0),
    Point3::new(4.0, 0.0, 0.0),
    Point3::new(0.0, 3.0, 0.0),
);
assert!((area - 6.0).abs() < 1e-9);
```

## BoundingBox 향상 *(Phase 9)*

| 메서드 | 반환 | 설명 |
|--------|------|------|
| `overlaps(&other)` | `bool` | 두 AABB의 겹침 여부 |
| `expand(margin)` | `BoundingBox` | 모든 방향으로 margin만큼 확장 |
| `volume()` | `f64` | AABB 부피 |
| `surface_area()` | `f64` | AABB 표면적 |
| `longest_axis()` | `usize` | 가장 긴 축 (0=X, 1=Y, 2=Z) |
| `size()` | `Vec3` | 각 축의 크기 (dx, dy, dz) |

```rust
let mut a = BoundingBox::empty();
a.include_point(Point3::new(0.0, 0.0, 0.0));
a.include_point(Point3::new(10.0, 5.0, 3.0));

assert!((a.volume() - 150.0).abs() < 1e-9);
assert!((a.surface_area() - 190.0).abs() < 1e-9);
assert_eq!(a.longest_axis(), 0);  // X축이 가장 긺
assert_eq!(a.size(), Vec3::new(10.0, 5.0, 3.0));

let expanded = a.expand(1.0);
assert!((expanded.volume() - 252.0).abs() < 1e-9);

let mut b = BoundingBox::empty();
b.include_point(Point3::new(5.0, 2.0, 1.0));
b.include_point(Point3::new(15.0, 7.0, 4.0));
assert!(a.overlaps(&b));
```

## linalg 모듈

nalgebra의 동적 선형대수 타입 재수출:

```rust
use cadkernel_math::linalg::{DMatrix, DVector, LU};
```

## 파일 구조

```
crates/math/src/
├── lib.rs        ← 모듈 등록 + re-export
├── vector.rs     ← Vec2, Vec3, Vec4
├── point.rs      ← Point2, Point3
├── matrix.rs     ← Mat3, Mat4
├── transform.rs  ← Transform
├── quaternion.rs ← Quaternion
├── ray.rs        ← Ray3
├── bbox.rs       ← BoundingBox (+ overlaps, expand, volume, surface_area, longest_axis, size)
├── tolerance.rs  ← EPSILON, approx_eq, is_zero
├── interop.rs    ← nalgebra From/Into 구현
├── linalg.rs     ← DMatrix/DVector 재수출
├── utils.rs      ← 유틸리티 함수 11개 (distance, angle, projection, interpolation, area)
└── prelude.rs    ← 편의 re-export
```
