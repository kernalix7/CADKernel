# Crate: cadkernel-modeling

> **역할**: 솔리드 생성(프리미티브), Feature Operations, Boolean, 질량 특성, Mirror/Shell/Scale, Fillet/Split/Draft, 공간 쿼리  
> **의존성**: `cadkernel-core`, `cadkernel-math`, `cadkernel-geometry`, `cadkernel-topology`, `cadkernel-io`  
> **경로**: `crates/modeling/`

## 프리미티브 빌더

| 함수 | 반환 | 파라미터 |
|------|------|----------|
| `make_box` | `KernelResult<BoxResult>` | `dx, dy, dz` |
| `make_cylinder` | `KernelResult<CylinderResult>` | `radius, height, segments` |
| `make_sphere` | `KernelResult<SphereResult>` | `radius, segments, rings` |

```rust
let mut model = BRepModel::new();
let b = make_box(&mut model, 10.0, 5.0, 3.0)?;
// b.solid, b.faces[0..6]
```

## Feature Operations

### Extrude

프로파일을 방향으로 밀어내어 솔리드 생성.

```rust
let ext = extrude(&mut model, &profile, Vec3::Z, 5.0)?;
// ext.solid, ext.bottom_face, ext.top_face, ext.side_faces
```

### Revolve

프로파일을 축 주위로 회전하여 솔리드 생성.

```rust
let rev = revolve(&mut model, &profile, Point3::ORIGIN, Vec3::Y, TAU, 24)?;
// rev.solid, rev.faces
```

### Sweep *(Phase 5)*

프로파일을 경로를 따라 이동하여 솔리드 생성.

```rust
let path = vec![
    Point3::new(0.0, 0.0, 0.0),
    Point3::new(0.0, 0.0, 5.0),
    Point3::new(5.0, 0.0, 5.0),
];
let sw = sweep(&mut model, &profile, &path)?;
// sw.solid, sw.faces
```

**알고리즘**: 경로를 따라 회전 최소화 프레임(Rotation-Minimizing Frame)을 전파하고, 각 경로점에서 프로파일을 배치한 뒤 인접 섹션을 quad 면으로 연결.

### Loft *(Phase 6)*

복수 단면 프로파일을 보간하여 솔리드 생성.

```rust
let sections = vec![
    vec![/* square 2x2 at z=0 */],
    vec![/* square 3x3 at z=3 */],
    vec![/* square 1x1 at z=6 */],
];
let lt = loft(&mut model, &sections, true, true)?;
// lt.solid, lt.faces
```

**파라미터**: `cap_start` / `cap_end`로 양 끝 캡 생성 여부 제어. 모든 섹션의 점 개수가 동일해야 함.

## Pattern Operations *(Phase 6)*

### Linear Pattern

```rust
let b = make_box(&mut model, 2.0, 2.0, 2.0)?;
let pat = linear_pattern(&mut model, b.solid, Vec3::X, 5.0, 4)?;
// pat.solids: 3개 복사본 (원본 제외)
```

### Circular Pattern

```rust
let b = make_box(&mut model, 2.0, 2.0, 2.0)?;
let pat = circular_pattern(
    &mut model, b.solid,
    Point3::ORIGIN, Vec3::Z,
    std::f64::consts::TAU, 6,
)?;
// pat.solids: 5개 복사본 (60° 간격)
```

**내부**: `copy_solid_with_transform` 헬퍼가 솔리드의 전체 토폴로지(vertex→edge→face→shell→solid)를 deep-copy하고 Transform 적용.

## Mirror / Shell / Scale *(Phase 8)*

### Mirror

솔리드를 평면에 대해 반사 복사합니다.

```rust
let mut model = BRepModel::new();
let b = make_box(&mut model, 10.0, 5.0, 3.0)?;

// XY 평면 기준 미러
let mirrored = mirror_solid(
    &mut model,
    b.solid,
    Point3::ORIGIN,    // 평면 위의 점
    Vec3::Z,           // 평면 법선
)?;
// mirrored.solid: 반사된 복사본
```

**알고리즘**: `copy_solid_with_transform` 유틸리티와 `Transform::mirror(plane_point, plane_normal)`을 조합하여 전체 토폴로지를 deep-copy 후 반사 변환 적용.

### Shell

솔리드에서 지정 면을 제거하고 일정 두께의 박벽을 만듭니다.

```rust
let mut model = BRepModel::new();
let b = make_box(&mut model, 10.0, 10.0, 10.0)?;

// 윗면 제거, 두께 1.0mm
let shelled = shell_solid(
    &mut model,
    b.solid,
    b.faces[5],   // 제거할 면 (top face)
    1.0,          // 벽 두께
)?;
// shelled.solid: 중공 박벽 솔리드
```

### Scale

솔리드를 중심점 기준으로 비균일 스케일 복사합니다.

```rust
let mut model = BRepModel::new();
let b = make_box(&mut model, 5.0, 5.0, 5.0)?;

// X 2배, Y 1배, Z 3배 스케일
let scaled = scale_solid(
    &mut model,
    b.solid,
    Point3::ORIGIN,   // 스케일 중심
    2.0, 1.0, 3.0,    // sx, sy, sz
)?;
// scaled.solid: 스케일된 복사본
```

**내부**: `copy_solid_with_transform` 공유 유틸리티가 `pattern.rs`에서 분리되어 Mirror, Shell, Scale 모두에서 재사용됨.

## Fillet / Split / Draft *(Phase 13–14)*

### Fillet

모서리를 호 근사 방식으로 라운딩합니다.

```rust
let b = make_box(&mut model, Point3::ORIGIN, 10.0, 10.0, 10.0)?;

let filleted = fillet_edge(
    &mut model,
    b.solid,
    edge_v1,        // 모서리 시작 vertex
    edge_v2,        // 모서리 끝 vertex
    1.0,            // 필렛 반경
    8,              // 세그먼트 수
)?;
// filleted.solid, filleted.fillet_faces, filleted.faces
```

### Split Body

절단 평면으로 솔리드를 두 개로 분할합니다.

```rust
let split = split_solid(
    &mut model,
    b.solid,
    Point3::new(0.0, 0.0, 5.0),  // 평면 위의 점
    Vec3::Z,                       // 평면 법선
)?;
// split.above: 평면 위쪽 솔리드
// split.below: 평면 아래쪽 솔리드
```

### Draft Angle

금형 이형을 위한 구배 각도를 적용합니다.

```rust
let drafted = draft_faces(
    &mut model,
    b.solid,
    Vec3::Z,                       // 풀 방향
    Point3::new(0.0, 0.0, 5.0),   // 중립면 위의 점
    3.0_f64.to_radians(),          // 구배 각도 (3°)
    &faces_to_draft,
)?;
// drafted.solid, drafted.faces
```

## Spatial Queries *(Phase 13, 15)*

### Point-in-Solid

```rust
let result = point_in_solid(&model, solid, Point3::new(5.0, 5.0, 5.0))?;
match result {
    Containment::Inside => println!("내부"),
    Containment::Outside => println!("외부"),
    Containment::OnBoundary => println!("경계"),
}
```

### Closest Point on Solid

```rust
let result = closest_point_on_solid(&model, solid, Point3::new(5.0, 5.0, 20.0))?;
println!("최근접점: {}", result.point);
println!("거리: {:.3}", result.distance);
println!("면: {:?}", result.face);
```

## Chamfer *(Phase 7)*

모서리에 면취(bevel)를 적용합니다.

```rust
let mut model = BRepModel::new();
let b = make_box(&mut model, 10.0, 10.0, 10.0)?;
let chamfered = chamfer_edge(&mut model, b.solid, v0, v1, 1.0)?;
```

## Fillet *(Phase 13)*

모서리에 호 근사 기반 라운딩을 적용합니다.

```rust
let mut model = BRepModel::new();
let b = make_box(&mut model, 10.0, 10.0, 10.0)?;

let filleted = fillet_edge(
    &mut model,
    b.solid,
    v_start,       // 모서리 시작 꼭짓점
    v_end,         // 모서리 끝 꼭짓점
    1.0,           // 반경
    8,             // 세그먼트 수 (호 근사 정밀도)
)?;
// filleted.solid: 라운딩된 솔리드
```

**알고리즘**: 지정된 모서리의 인접 면을 탐색하고, 호(arc)로 근사한 fillet 면을 삽입한 뒤 토폴로지를 재구축.

## Split Body *(Phase 13)*

절단 평면으로 솔리드를 두 개로 분할합니다.

```rust
let mut model = BRepModel::new();
let b = make_box(&mut model, 10.0, 10.0, 10.0)?;

let split = split_solid(
    &mut model,
    b.solid,
    Point3::new(0.0, 0.0, 5.0),   // 평면 위의 점
    Vec3::Z,                        // 평면 법선
)?;
// split.above: 평면 위쪽 솔리드
// split.below: 평면 아래쪽 솔리드
```

## Point-in-Solid *(Phase 13)*

점이 솔리드 내부/외부/경계에 있는지 판정합니다 (레이캐스팅 기반).

```rust
let mut model = BRepModel::new();
let b = make_box(&mut model, 10.0, 10.0, 10.0)?;

let result = point_in_solid(&model, b.solid, Point3::new(5.0, 5.0, 5.0));
assert_eq!(result, Containment::Inside);

let result = point_in_solid(&model, b.solid, Point3::new(100.0, 0.0, 0.0));
assert_eq!(result, Containment::Outside);
```

## Draft Angle *(Phase 14)*

금형 이형을 위한 구배 각도(테이퍼)를 적용합니다.

```rust
let mut model = BRepModel::new();
let b = make_box(&mut model, 10.0, 10.0, 10.0)?;

let drafted = draft_faces(
    &mut model,
    b.solid,
    Vec3::Z,                        // 풀 방향
    Point3::new(0.0, 0.0, 0.0),    // 중립면 점
    5.0_f64.to_radians(),           // 구배 각도
    &[b.faces[0], b.faces[1]],     // 대상 면들
)?;
```

## Closest Point Query *(Phase 15)*

솔리드 위의 최근접점을 찾습니다 (Voronoi 영역 삼각형 투영).

```rust
let mut model = BRepModel::new();
let b = make_box(&mut model, 10.0, 10.0, 10.0)?;

let result = closest_point_on_solid(&model, b.solid, Point3::new(15.0, 5.0, 5.0));
// result.point: 솔리드 표면의 최근접점
// result.distance: 쿼리 점과 최근접점 사이의 거리
// result.face: 최근접점이 위치한 면의 핸들
```

## Boolean Operations

```rust
let result = boolean_op(&model_a, solid_a, &model_b, solid_b, BooleanOp::Union)?;
let result = boolean_op(&model_a, solid_a, &model_b, solid_b, BooleanOp::Subtract)?;
let result = boolean_op(&model_a, solid_a, &model_b, solid_b, BooleanOp::Intersect)?;
```

**파이프라인**:
1. **Broad Phase**: AABB 겹침 테스트로 후보 면 쌍 필터링
2. **Classification**: 면의 중심점이 상대 솔리드의 내부/외부/경계인지 판별
3. **Evaluation**: 판별 결과에 따라 면 복사/제거 → 결과 솔리드 조립

## Mass Properties *(Phase 5)*

테셀레이션된 메시에서 질량 특성 계산 (발산 정리 기반).

```rust
let props = solid_mass_properties(&model, solid_handle);
println!("Volume:  {:.3}", props.volume);
println!("Area:    {:.3}", props.surface_area);
println!("Center:  {}", props.centroid);

// 또는 메시에서 직접
let mesh = tessellate_solid(&model, solid_h);
let props = compute_mass_properties(&mesh);
```

## Persistent Naming

모든 Feature Operation은 자동으로 Persistent Naming 태그를 생성합니다:

```rust
let ext = extrude(&mut model, &profile, Vec3::Z, 5.0)?;
let op = model.history.records().last().unwrap().operation;

let bottom = Tag::generated(EntityKind::Face, op, 0);
let top    = Tag::generated(EntityKind::Face, op, 1);
// side faces: Tag::generated(EntityKind::Face, op, 2..)

assert_eq!(model.find_face_by_tag(&bottom), Some(ext.bottom_face));
```

## 파일 구조

```
crates/modeling/src/
├── lib.rs
├── measure.rs         ← MassProperties, compute/solid_mass_properties
├── query.rs           ← point_in_solid, closest_point_on_solid
├── features/
│   ├── mod.rs
│   ├── extrude.rs     ← extrude, ExtrudeResult
│   ├── revolve.rs     ← revolve, RevolveResult
│   ├── sweep.rs       ← sweep, SweepResult
│   ├── loft.rs        ← loft, LoftResult
│   ├── pattern.rs     ← linear_pattern, circular_pattern, PatternResult
│   ├── mirror.rs      ← mirror_solid, MirrorResult
│   ├── shell.rs       ← shell_solid, ShellResult
│   ├── scale.rs       ← scale_solid, ScaleResult
│   ├── fillet.rs      ← fillet_edge, FilletResult
│   ├── split.rs       ← split_solid, SplitResult
│   ├── draft.rs       ← draft_faces, DraftResult
│   └── copy_utils.rs  ← copy_solid_with_transform (공유 유틸리티)
├── primitives/
│   ├── mod.rs
│   ├── box_shape.rs   ← make_box, BoxResult
│   ├── cylinder_shape.rs ← make_cylinder, CylinderResult
│   └── sphere_shape.rs   ← make_sphere, SphereResult
├── boolean/
│   ├── mod.rs          ← boolean_op, BooleanOp
│   ├── broad_phase.rs  ← AABB 기반 필터링
│   ├── classify.rs     ← Inside/Outside/Boundary 분류
│   └── evaluate.rs     ← 결과 모델 조립
└── benches/
    └── modeling_benchmarks.rs  ← 14개 criterion 벤치마크
```
