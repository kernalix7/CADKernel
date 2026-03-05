# Architecture

CADKernel은 **단방향 계층 아키텍처**를 채택합니다. 상위 크레이트는 하위에 의존하지만, 하위는 상위를 알지 못합니다.

## 계층 다이어그램

```
┌─────────────────────────────────────────────────────────────┐
│  cadkernel (root)                                           │
│  통합 re-export · prelude · E2E 테스트                       │
├─────────────────────────────────────────────────────────────┤
│  Application Layer                                          │
│  ┌────────────────────────────┐ ┌─────────────────────────┐ │
│  │ cadkernel-viewer            │ │ cadkernel-python        │ │
│  │ egui GUI · wgpu Rendering  │ │ PyO3 Bindings           │ │
│  │ Camera · Grid · Navigation │ │                         │ │
│  └──────────────┬─────────────┘ └────────────┬────────────┘ │
├─────────────────┼────────────────────────────┼──────────────┤
│  Service Layer  │                            │              │
│  ┌─────────────┐ ┌──────────────┐ ┌───────────────────────┐ │
│  │ cadkernel-io│ │cadkernel-    │ │ cadkernel-modeling     │ │
│  │ STL/OBJ     │ │sketch        │ │ Primitives · Boolean  │ │
│  │ Tessellation│ │2D + Solver   │ │ Extrude · Revolve     │ │
│  │ glTF · STEP │ │              │ │ Sweep · Loft · Pattern│ │
│  │ IGES · JSON │ │              │ │ MassProperties        │ │
│  └──────┬──────┘ └──────┬───────┘ └───────────┬───────────┘ │
├─────────┼───────────────┼─────────────────────┼─────────────┤
│  Core Kernel Layer      │                     │             │
│  ┌──────┴───────────────┴─────────────────────┴───────────┐ │
│  │ cadkernel-topology                                      │ │
│  │ B-Rep Half-Edge · EntityStore · Persistent Naming       │ │
│  │ Validation · Traversal · Wire                           │ │
│  ├─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─[feature: geometry-binding]─ ─ ┤ │
│  │ cadkernel-geometry                                      │ │
│  │ Curve (Line·Arc·Circle·Ellipse·NURBS)                   │ │
│  │ Surface (Plane·Cylinder·Sphere·Cone·Torus·NURBS)        │ │
│  │ Intersect (SSI · Line-Surface)                          │ │
│  ├─────────────────────────────────────────────────────────┤ │
│  │ cadkernel-math                                          │ │
│  │ Vec2/3/4 · Point2/3 · Mat3/4 · Transform · Quaternion  │ │
│  │ Ray3 · BoundingBox · Tolerance · linalg                 │ │
│  ├─────────────────────────────────────────────────────────┤ │
│  │ cadkernel-core                                          │ │
│  │ KernelError · KernelResult                              │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## 의존성 그래프

```
cadkernel-core        (의존 없음)
    ↑
cadkernel-math        + nalgebra 0.33, glam 0.29
    ↑
cadkernel-geometry    + cadkernel-core
    ↑ [feature: geometry-binding, default]
cadkernel-topology    + cadkernel-core, cadkernel-math
    ↑
cadkernel-modeling    + cadkernel-core, cadkernel-math, cadkernel-geometry, cadkernel-io
cadkernel-sketch      + cadkernel-math, cadkernel-topology
cadkernel-io          + cadkernel-math, cadkernel-topology
    ↑
cadkernel-viewer      + cadkernel-io, cadkernel-modeling, cadkernel-topology, cadkernel-math
cadkernel-python      + cadkernel (PyO3 바인딩)
    ↑
cadkernel (root)      전체 통합
```

## 핵심 설계 결정

### 1. `cadkernel-core` 분리

`KernelError`와 `KernelResult`를 최하위 독립 크레이트에 배치하여 모든 크레이트가 공유.
topology가 에러 타입의 허브가 되는 것을 방지.

### 2. Geometry를 Feature Flag으로

`cadkernel-topology`의 `cadkernel-geometry` 의존을 `geometry-binding` feature flag로 분리.
순수 위상 연산만 필요한 경우 기하 의존 제거 가능.

```toml
# crates/topology/Cargo.toml
[features]
default = ["geometry-binding"]
geometry-binding = ["dep:cadkernel-geometry"]
```

효과: `EdgeData.curve: Option<Arc<dyn Curve + Send + Sync>>` 필드가 조건부 컴파일됨.

### 3. nalgebra 의존 중앙화

sketch 크레이트가 nalgebra에 직접 의존하지 않고, math 크레이트의 `linalg` 모듈을 통해 재수출된 `DMatrix`/`DVector`를 사용. 버전 충돌 방지.

### 4. 통합 Prelude

```rust
use cadkernel::prelude::*;
// Vec3, Point3, Transform, BRepModel, Handle, extrude, make_box, solve, ... 모두 사용 가능
```

## Feature Flags

| 크레이트 | Feature | 기본값 | 효과 |
|----------|---------|:------:|------|
| `cadkernel-topology` | `geometry-binding` | ✅ | Edge/Face에 기하 바인딩 포함 |
