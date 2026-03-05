# Crate: cadkernel-sketch

> **역할**: 2D 파라메트릭 스케치 시스템 + Newton-Raphson 제약 솔버  
> **의존성**: `cadkernel-math`, `cadkernel-topology`  
> **경로**: `crates/sketch/`

## 개요

CAD에서 3D 모델은 보통 2D 스케치에서 시작합니다. 이 크레이트는 점/선/호/원으로 구성된 2D 스케치를 정의하고, 기하학적 제약 조건을 만족하도록 풀어주는 솔버를 제공합니다.

## 워크플로우

```
1. Sketch 생성 → 2. 요소 추가 → 3. 제약 추가 → 4. Solve → 5. 3D 추출
```

## 스케치 요소

```rust
let mut sketch = Sketch::new();

let p = sketch.add_point(x, y);              // SketchPoint
let l = sketch.add_line(p0, p1);             // SketchLine
let a = sketch.add_arc(center, start, end);  // SketchArc
let c = sketch.add_circle(center, radius_pt); // SketchCircle
```

## 14개 제약 조건

| 제약 | 구문 | 설명 |
|------|------|------|
| Fixed | `Constraint::Fixed(p, x, y)` | 점을 고정 좌표에 |
| Horizontal | `Constraint::Horizontal(l)` | 선을 수평으로 |
| Vertical | `Constraint::Vertical(l)` | 선을 수직으로 |
| Length | `Constraint::Length(l, len)` | 선분 길이 지정 |
| Distance | `Constraint::Distance(p1, p2, d)` | 두 점 사이 거리 |
| Coincident | `Constraint::Coincident(p1, p2)` | 두 점 일치 |
| Parallel | `Constraint::Parallel(l1, l2)` | 두 선 평행 |
| Perpendicular | `Constraint::Perpendicular(l1, l2)` | 두 선 직교 |
| Equal | `Constraint::Equal(l1, l2)` | 동일 길이 |
| PointOnLine | `Constraint::PointOnLine(p, l)` | 점이 선 위에 |
| PointOnCircle | `Constraint::PointOnCircle(p, c)` | 점이 원 위에 |
| Symmetric | `Constraint::Symmetric(p1, p2, l)` | 선 대칭 |
| Angle | `Constraint::Angle(l1, l2, θ)` | 두 선 사이 각도 |
| Radius | `Constraint::Radius(c, r)` | 원 반지름 |
| Tangent | `Constraint::Tangent(l, c)` | 선-원 접선 |
| MidPoint | `Constraint::MidPoint(p, l)` | 선의 중점 |

## 솔버

```rust
let result = solve(&mut sketch, max_iterations, tolerance);
// SolverResult { converged: bool, iterations: usize, residual: f64 }
```

**알고리즘**: Newton-Raphson + Armijo 백트래킹 라인 서치

**내부**:
1. 제약 조건 → 잔차 벡터(residual) + 야코비안(Jacobian) 생성
2. `J * dx = -r` 선형 시스템 풀이 (LU 분해)
3. Armijo 조건으로 스텝 크기 조정
4. 수렴할 때까지 반복

## 3D 프로파일 추출

```rust
let wp = WorkPlane::xy();   // XY 평면
let wp = WorkPlane::xz();   // XZ 평면

let profile: Vec<Point3> = extract_profile(&sketch, &wp);
// 2D 스케치 점들을 3D 공간으로 매핑
```

## 전체 예제

```rust
use cadkernel::prelude::*;

let mut sketch = Sketch::new();

// L자 프로파일
let p0 = sketch.add_point(0.0, 0.0);
let p1 = sketch.add_point(4.0, 0.0);
let p2 = sketch.add_point(4.0, 1.0);
let p3 = sketch.add_point(1.0, 1.0);
let p4 = sketch.add_point(1.0, 3.0);
let p5 = sketch.add_point(0.0, 3.0);

let l0 = sketch.add_line(p0, p1);
let l1 = sketch.add_line(p1, p2);
let l2 = sketch.add_line(p2, p3);
let l3 = sketch.add_line(p3, p4);
let l4 = sketch.add_line(p4, p5);
let l5 = sketch.add_line(p5, p0);

sketch.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
sketch.add_constraint(Constraint::Horizontal(l0));
sketch.add_constraint(Constraint::Vertical(l1));
// ... 나머지 제약 ...

let result = solve(&mut sketch, 200, 1e-10);
assert!(result.converged);

let profile = extract_profile(&sketch, &WorkPlane::xy());
```

## 파일 구조

```
crates/sketch/src/
├── lib.rs          ← Sketch, WorkPlane, re-exports
├── constraint.rs   ← Constraint enum + residual/jacobian
├── solver.rs       ← Newton-Raphson solver
└── profile.rs      ← extract_profile (2D→3D)
```
