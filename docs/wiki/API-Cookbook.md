# API Cookbook

실전에서 자주 쓰이는 코드 패턴 모음입니다.

---

## 1. 박스 생성 → STL 내보내기

```rust
use cadkernel::prelude::*;

fn main() -> KernelResult<()> {
    let mut model = BRepModel::new();
    let b = make_box(&mut model, 20.0, 10.0, 5.0)?;

    let mesh = tessellate_solid(&model, b.solid);
    export_stl_binary(&mesh, "box.stl")?;

    println!("Volume: {:.1}", solid_mass_properties(&model, b.solid).volume);
    Ok(())
}
```

## 2. 스케치 → 돌출 → 질량 특성

```rust
use cadkernel::prelude::*;

fn main() -> KernelResult<()> {
    let mut sketch = Sketch::new();
    let p0 = sketch.add_point(0.0, 0.0);
    let p1 = sketch.add_point(10.0, 0.0);
    let p2 = sketch.add_point(10.0, 5.0);
    let p3 = sketch.add_point(0.0, 5.0);
    sketch.add_line(p0, p1);
    sketch.add_line(p1, p2);
    sketch.add_line(p2, p3);
    sketch.add_line(p3, p0);

    let result = solve(&mut sketch, 100, 1e-10);
    assert!(result.converged);

    let profile = extract_profile(&sketch, &WorkPlane::xy());

    let mut model = BRepModel::new();
    let ext = extrude(&mut model, &profile, Vec3::Z, 3.0)?;

    let props = solid_mass_properties(&model, ext.solid);
    println!("Volume:  {:.2}", props.volume);  // 150.00
    println!("Area:    {:.2}", props.surface_area);
    println!("Center:  {}", props.centroid);

    Ok(())
}
```

## 3. 실린더 생성 → OBJ 내보내기

```rust
use cadkernel::prelude::*;

fn main() -> KernelResult<()> {
    let mut model = BRepModel::new();
    let cyl = make_cylinder(&mut model, 2.0, 8.0, 32)?;

    let mesh = tessellate_solid(&model, cyl.solid);
    export_obj(&mesh, "cylinder.obj")?;

    let props = solid_mass_properties(&model, cyl.solid);
    println!("V={:.2}, A={:.2}", props.volume, props.surface_area);

    Ok(())
}
```

## 4. Revolve로 회전체 생성

```rust
use cadkernel::prelude::*;
use std::f64::consts::TAU;

fn main() -> KernelResult<()> {
    // 반원 프로파일 (단순화된 와인잔 형태)
    let profile = vec![
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(2.0, 0.0, 0.0),
        Point3::new(2.0, 0.0, 0.5),
        Point3::new(0.5, 0.0, 0.5),
        Point3::new(0.5, 0.0, 5.0),
        Point3::new(3.0, 0.0, 5.0),
        Point3::new(3.0, 0.0, 6.0),
        Point3::new(0.0, 0.0, 6.0),
    ];

    let mut model = BRepModel::new();
    let rev = revolve(
        &mut model,
        &profile,
        Point3::ORIGIN,
        Vec3::Z,
        TAU,
        24,
    )?;

    let mesh = tessellate_solid(&model, rev.solid);
    export_stl_binary(&mesh, "wineglass.stl")?;
    
    Ok(())
}
```

## 5. Sweep으로 파이프 생성

```rust
use cadkernel::prelude::*;

fn main() -> KernelResult<()> {
    // 원형 프로파일 (8각형 근사)
    let n = 8;
    let r = 0.5;
    let profile: Vec<Point3> = (0..n)
        .map(|i| {
            let angle = 2.0 * std::f64::consts::PI * (i as f64) / (n as f64);
            Point3::new(r * angle.cos(), r * angle.sin(), 0.0)
        })
        .collect();

    // L자 경로
    let path = vec![
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(0.0, 0.0, 5.0),
        Point3::new(5.0, 0.0, 5.0),
    ];

    let mut model = BRepModel::new();
    let sw = sweep(&mut model, &profile, &path)?;

    let mesh = tessellate_solid(&model, sw.solid);
    export_stl_ascii(&mesh, "pipe.stl", "pipe")?;

    Ok(())
}
```

## 6. Boolean 연산

```rust
use cadkernel::prelude::*;

fn main() -> KernelResult<()> {
    // 박스 A
    let mut model_a = BRepModel::new();
    let a = make_box(&mut model_a, 10.0, 10.0, 10.0)?;

    // 박스 B (겹치는 위치)
    let mut model_b = BRepModel::new();
    let b = make_box(&mut model_b, 10.0, 10.0, 10.0)?;
    model_b.transform(&Transform::translation(5.0, 5.0, 0.0));

    // Union
    let union = boolean_op(&model_a, a.solid, &model_b, b.solid, BooleanOp::Union)?;

    // Subtract
    let diff = boolean_op(&model_a, a.solid, &model_b, b.solid, BooleanOp::Subtract)?;

    // Intersect
    let inter = boolean_op(&model_a, a.solid, &model_b, b.solid, BooleanOp::Intersect)?;

    Ok(())
}
```

## 7. Persistent Naming으로 면 추적

```rust
use cadkernel::prelude::*;

fn main() -> KernelResult<()> {
    let mut model = BRepModel::new();
    let ext = extrude(
        &mut model,
        &[
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(5.0, 0.0, 0.0),
            Point3::new(5.0, 5.0, 0.0),
            Point3::new(0.0, 5.0, 0.0),
        ],
        Vec3::Z,
        3.0,
    )?;

    // 이력에서 연산 ID 추출
    let op = model.history.records().last().unwrap().operation;

    // 태그로 특정 면 찾기
    let top_tag = Tag::generated(EntityKind::Face, op, 1);
    let top_face = model.find_face_by_tag(&top_tag).unwrap();
    assert_eq!(top_face, ext.top_face);

    // 향후 이 태그로 Fillet/Chamfer 대상 지정 가능
    println!("Top face found via tag: {:?}", top_tag);

    Ok(())
}
```

## 8. 모델 검증 + 에러 처리

```rust
use cadkernel::prelude::*;

fn process() -> KernelResult<()> {
    let mut model = BRepModel::new();
    let b = make_box(&mut model, 5.0, 5.0, 5.0)?;

    // B-Rep 구조 검증 (twin 대칭, 루프 순환, 오일러 특성)
    model.validate()?;

    // 순회
    let faces = model.faces_of_edge(b.faces[0])?;

    Ok(())
}

fn main() {
    match process() {
        Ok(()) => println!("Success"),
        Err(KernelError::InvalidArgument(msg)) => {
            eprintln!("Bad input: {msg}");
        }
        Err(KernelError::ValidationFailed(msg)) => {
            eprintln!("Model corrupt: {msg}");
        }
        Err(e) => eprintln!("Error: {e}"),
    }
}
```

## 9. BoundingBox 활용

```rust
use cadkernel::prelude::*;

let mut bbox = BoundingBox::empty();
bbox.include_point(Point3::new(0.0, 0.0, 0.0));
bbox.include_point(Point3::new(10.0, 5.0, 3.0));

println!("Center: {}", bbox.center());      // (5, 2.5, 1.5)
println!("Diagonal: {}", bbox.diagonal());   // Vec3(10, 5, 3)
println!("Contains origin: {}", bbox.contains(Point3::ORIGIN));  // true

// 슬라이스에서 바로 생성
let bbox = BoundingBox::from(&points[..]);
```

## 10. Loft로 테이퍼 형상 생성

```rust
use cadkernel::prelude::*;

fn main() -> KernelResult<()> {
    // 아래 넓은 사각형 → 위로 갈수록 좁아지는 피라미드 형태
    let bottom = vec![
        Point3::new(-5.0, -5.0, 0.0),
        Point3::new( 5.0, -5.0, 0.0),
        Point3::new( 5.0,  5.0, 0.0),
        Point3::new(-5.0,  5.0, 0.0),
    ];
    let top = vec![
        Point3::new(-1.0, -1.0, 10.0),
        Point3::new( 1.0, -1.0, 10.0),
        Point3::new( 1.0,  1.0, 10.0),
        Point3::new(-1.0,  1.0, 10.0),
    ];

    let mut model = BRepModel::new();
    let lt = loft(&mut model, &[bottom, top], true, true)?;

    let mesh = tessellate_solid(&model, lt.solid);
    export_stl_binary(&mesh, "taper.stl")?;

    Ok(())
}
```

## 11. Linear Pattern으로 볼트 구멍 배열

```rust
use cadkernel::prelude::*;

fn main() -> KernelResult<()> {
    let mut model = BRepModel::new();

    // 원형 기둥 하나 생성
    let cyl = make_cylinder(&mut model, 0.5, 2.0, 16)?;

    // X 방향으로 5mm 간격, 총 6개 (원본 + 5 복사본)
    let pat = linear_pattern(&mut model, cyl.solid, Vec3::X, 5.0, 6)?;

    println!("Original + {} copies", pat.solids.len());

    Ok(())
}
```

## 12. Circular Pattern으로 기어 이빨 배열

```rust
use cadkernel::prelude::*;
use std::f64::consts::TAU;

fn main() -> KernelResult<()> {
    let mut model = BRepModel::new();

    // 이빨 하나 (박스로 근사)
    let tooth = make_box(&mut model, 1.0, 0.5, 2.0)?;

    // Z축 기준 360°에 24개 이빨 배치
    let pat = circular_pattern(
        &mut model, tooth.solid,
        Point3::ORIGIN, Vec3::Z,
        TAU, 24,
    )?;

    println!("{} gear teeth created", pat.solids.len() + 1);

    Ok(())
}
```

## 13. 변환 합성

```rust
use cadkernel::prelude::*;
use std::f64::consts::PI;

let t = Transform::translation(5.0, 0.0, 0.0)
    .then(Transform::rotation_z(PI / 4.0))
    .then(Transform::uniform_scale(2.0));

let p = t.apply_point(Point3::ORIGIN);
let v = t.apply_vec(Vec3::X);

// 역변환
if let Some(inv) = t.try_inverse() {
    let back = inv.apply_point(p);
    assert!(back.approx_eq(Point3::ORIGIN));
}
```

## 14. 솔리드 미러 (평면 반사)

```rust
use cadkernel::prelude::*;

fn main() -> KernelResult<()> {
    let mut model = BRepModel::new();
    let b = make_box(&mut model, 10.0, 5.0, 3.0)?;

    // XY 평면(Z=0) 기준 미러 → 아래쪽에 반사 복사본 생성
    let mirrored = mirror_solid(
        &mut model,
        b.solid,
        Point3::ORIGIN,
        Vec3::Z,
    )?;

    // 원본과 미러 모두 검증
    model.validate()?;

    let mesh = tessellate_solid(&model, mirrored.solid);
    export_stl_binary(&mesh, "mirrored.stl")?;

    Ok(())
}
```

## 15. 박스를 Shell로 중공 형상 만들기

```rust
use cadkernel::prelude::*;

fn main() -> KernelResult<()> {
    let mut model = BRepModel::new();
    let b = make_box(&mut model, 20.0, 20.0, 10.0)?;

    // 윗면을 제거하고 벽 두께 2.0mm로 중공화
    let shelled = shell_solid(
        &mut model,
        b.solid,
        b.faces[5],  // top face
        2.0,
    )?;

    let mesh = tessellate_solid(&model, shelled.solid);
    export_obj(&mesh, "box_shell.obj")?;

    Ok(())
}
```

## 16. 비균일 스케일

```rust
use cadkernel::prelude::*;

fn main() -> KernelResult<()> {
    let mut model = BRepModel::new();
    let sph = make_sphere(&mut model, 5.0, 16, 8)?;

    // 구를 X방향 2배, Y방향 1배, Z방향 0.5배로 → 타원체
    let ellipsoid = scale_solid(
        &mut model,
        sph.solid,
        Point3::ORIGIN,
        2.0, 1.0, 0.5,
    )?;

    let mesh = tessellate_solid(&model, ellipsoid.solid);
    export_stl_binary(&mesh, "ellipsoid.stl")?;

    Ok(())
}
```

## 17. SVG로 2D 프로파일 내보내기

```rust
use cadkernel::prelude::*;

fn main() -> KernelResult<()> {
    // L자 프로파일
    let profile = vec![
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(10.0, 0.0, 0.0),
        Point3::new(10.0, 3.0, 0.0),
        Point3::new(3.0, 3.0, 0.0),
        Point3::new(3.0, 8.0, 0.0),
        Point3::new(0.0, 8.0, 0.0),
    ];

    let svg = profile_to_svg(&profile);
    std::fs::write("l_profile.svg", svg.to_string())?;

    Ok(())
}
```

## 18. Fillet — 모서리 둥글게 처리

```rust
use cadkernel::prelude::*;

fn main() -> KernelResult<()> {
    let mut model = BRepModel::new();
    let b = make_box(&mut model, Point3::ORIGIN, 4.0, 4.0, 4.0)?;

    // 첫 번째 모서리(v0→v1)를 반지름 0.5, 4세그먼트로 필렛
    let v0 = b.vertices[0];
    let v1 = b.vertices[1];
    let result = fillet_edge(&mut model, b.solid, v0, v1, 0.5, 4)?;

    println!("원본 면 + 필렛 면 = {} faces", result.faces.len());
    println!("필렛 스트립: {} faces", result.fillet_faces.len());

    Ok(())
}
```

## 19. Split Body — 평면으로 솔리드 절단

```rust
use cadkernel::prelude::*;

fn main() -> KernelResult<()> {
    let mut model = BRepModel::new();
    let b = make_box(&mut model, Point3::ORIGIN, 4.0, 4.0, 4.0)?;

    // Z=2 평면으로 절단 → 윗부분(above) + 아랫부분(below)
    let result = split_solid(
        &mut model,
        b.solid,
        Point3::new(0.0, 0.0, 2.0),
        Vec3::Z,
    )?;

    assert!(model.solids.is_alive(result.above));
    assert!(model.solids.is_alive(result.below));
    println!("Split into two solids");

    Ok(())
}
```

## 20. Point-in-Solid — 점 내외부 판정

```rust
use cadkernel::prelude::*;

fn main() -> KernelResult<()> {
    let mut model = BRepModel::new();
    let b = make_box(&mut model, Point3::ORIGIN, 4.0, 4.0, 4.0)?;

    let inside  = point_in_solid(&model, b.solid, Point3::new(1.0, 1.0, 1.0))?;
    let outside = point_in_solid(&model, b.solid, Point3::new(10.0, 10.0, 10.0))?;
    let on_face = point_in_solid(&model, b.solid, Point3::new(0.0, 2.0, 2.0))?;

    assert_eq!(inside,  Containment::Inside);
    assert_eq!(outside, Containment::Outside);
    assert_eq!(on_face, Containment::OnBoundary);

    Ok(())
}
```

## 21. Closest Point Query — 솔리드 표면 최근접점

```rust
use cadkernel::prelude::*;

fn main() -> KernelResult<()> {
    let mut model = BRepModel::new();
    let b = make_box(&mut model, Point3::ORIGIN, 4.0, 4.0, 4.0)?;

    let query = Point3::new(2.0, 2.0, 7.0);
    let result = closest_point_on_solid(&model, b.solid, query)?;

    println!("최근접점: ({:.2}, {:.2}, {:.2})", result.point.x, result.point.y, result.point.z);
    println!("거리: {:.2}", result.distance);   // 3.00
    println!("면 핸들: {:?}", result.face);

    Ok(())
}
```

## 22. 2D 커브 오프셋 — CNC/스케치용 평행 이동

```rust
use cadkernel::prelude::*;

// 열린 폴리라인 오프셋
let polyline = vec![
    Point2::new(0.0, 0.0),
    Point2::new(5.0, 0.0),
    Point2::new(5.0, 3.0),
];
let offset = offset_polyline_2d(&polyline, 1.0);  // 왼쪽(CCW)으로 1.0mm

// 닫힌 폴리곤 오프셋 (사각형 바깥으로 2.0mm 확장)
let square = vec![
    Point2::new(0.0, 0.0),
    Point2::new(10.0, 0.0),
    Point2::new(10.0, 10.0),
    Point2::new(0.0, 10.0),
];
let enlarged = offset_polygon_2d(&square, 2.0);   // 바깥으로
let shrunk  = offset_polygon_2d(&square, -1.0);   // 안으로
```

## 23. Draft Angle — 금형 빼기 기울기

```rust
use cadkernel::prelude::*;

fn main() -> KernelResult<()> {
    let mut model = BRepModel::new();
    let b = make_box(&mut model, Point3::ORIGIN, 4.0, 4.0, 4.0)?;

    let angle = 5.0_f64.to_radians();
    let side_faces = vec![b.faces[2], b.faces[3], b.faces[4], b.faces[5]];

    let result = draft_faces(
        &mut model,
        b.solid,
        Vec3::Z,              // 금형 빼기 방향
        Point3::ORIGIN,       // 중립면 점
        angle,                // 5° 기울기
        &side_faces,          // 적용할 면들
    )?;

    assert!(model.solids.is_alive(result.solid));
    println!("Draft applied to {} faces", result.faces.len());

    Ok(())
}
```

## 24. 적응형 테셀레이션 — 곡률 기반 메시 분할

```rust
use cadkernel::prelude::*;

// 커브 테셀레이션 (원호를 높은 정밀도로 분할)
let opts = TessellationOptions {
    chord_tolerance: 0.001,
    angle_tolerance: 0.05,
    min_segments: 4,
    max_depth: 10,
};

let circle = Circle::new(Point3::ORIGIN, 5.0, Vec3::Z);
let points = adaptive_tessellate_curve(
    |t| circle.evaluate(t),
    |t| circle.tangent(t),
    0.0,
    std::f64::consts::TAU,
    &opts,
);
println!("원호 → {} 점 (곡률 적응)", points.len());

// 서피스 테셀레이션
let sphere = Sphere::new(Point3::ORIGIN, 3.0);
let mesh: TessMesh = adaptive_tessellate_surface(
    |u, v| sphere.evaluate(u, v),
    |u, v| sphere.normal(u, v),
    0.0, 1.0,
    0.0, 1.0,
    &opts,
);
println!("구 → {} 삼각형", mesh.triangles.len());
```

## 25. STEP 내보내기 / 읽기

```rust
use cadkernel::prelude::*;

fn main() -> KernelResult<()> {
    let mut model = BRepModel::new();
    let b = make_box(&mut model, Point3::ORIGIN, 10.0, 5.0, 3.0)?;
    let mesh = tessellate_solid(&model, b.solid);

    // STEP 파일로 내보내기
    let step_string = export_step_mesh(&mesh.vertices, &mesh.triangles)?;
    std::fs::write("output.step", &step_string)?;

    // StepWriter로 직접 제어
    let mut writer = StepWriter::new();
    writer.add_cartesian_point("origin", Point3::ORIGIN);
    writer.add_direction("z", 0.0, 0.0, 1.0);
    writer.add_axis2_placement_3d("axis", 1, 2, None);
    let step = writer.to_step_string()?;

    // STEP 파일 읽기
    let points = read_step_points(&step_string)?;
    println!("읽어온 점: {} 개", points.len());

    Ok(())
}
```

## 26. IGES 내보내기 / 읽기

```rust
use cadkernel::prelude::*;

fn main() -> KernelResult<()> {
    // IGES 내보내기
    let mut writer = IgesWriter::new();
    writer.add_point(Point3::new(1.0, 2.0, 3.0));
    writer.add_line(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(10.0, 5.0, 0.0),
    );

    let iges_str = writer.to_iges_string()?;
    std::fs::write("output.igs", &iges_str)?;

    // IGES 파일 읽기
    let points = read_iges_points(&iges_str)?;
    let lines  = read_iges_lines(&iges_str)?;
    println!("점: {} 개, 선: {} 개", points.len(), lines.len());

    Ok(())
}
```

## 27. Undo/Redo — 모델 이력 관리

```rust
use cadkernel::prelude::*;

fn main() -> KernelResult<()> {
    let model = BRepModel::new();
    let mut history = ModelHistory::new(model, 50);

    // 1단계: 정점 추가
    let mut m = history.current_model().clone();
    m.add_vertex(Point3::new(1.0, 0.0, 0.0));
    history.record(m, "add vertex");

    // 2단계: 정점 하나 더 추가
    let mut m = history.current_model().clone();
    m.add_vertex(Point3::new(2.0, 0.0, 0.0));
    history.record(m, "add second vertex");

    // Undo → 1단계로
    let restored = history.undo().unwrap();
    assert_eq!(restored.vertices.len(), 1);

    // Redo → 2단계로
    let restored = history.redo().unwrap();
    assert_eq!(restored.vertices.len(), 2);

    println!("Undo 가능: {}, Redo 가능: {}", history.can_undo(), history.can_redo());

    Ok(())
}
```

## 28. 속성 시스템 — 재질과 메타데이터

```rust
use cadkernel::prelude::*;

fn main() -> KernelResult<()> {
    let mut model = BRepModel::new();
    let b = make_box(&mut model, Point3::ORIGIN, 10.0, 10.0, 5.0)?;

    // 재질 지정
    model.properties.set_material(0, Material::steel());

    // 색상 지정
    model.properties.set_color(0, Color::rgb(0.8, 0.2, 0.1));

    // 메타데이터
    model.properties.set_metadata(0, "part_number", PropertyValue::String("A-100".into()));
    model.properties.set_metadata(0, "weight_kg", PropertyValue::Float(12.5));

    // Material 프리셋: steel(), aluminum(), plastic_abs(), wood()
    let alu = Material::aluminum();
    println!("알루미늄 밀도: {} kg/m³", alu.density.unwrap_or(0.0));

    Ok(())
}
```

## 29. Fillet — 모서리 라운딩

```rust
use cadkernel::prelude::*;

fn main() -> KernelResult<()> {
    let mut model = BRepModel::new();
    let b = make_box(&mut model, Point3::ORIGIN, 10.0, 10.0, 10.0)?;

    // 모서리 양 끝 꼭짓점을 지정하여 Fillet 적용
    let verts = model.vertices_of_face(b.faces[0])?;
    let filleted = fillet_edge(
        &mut model,
        b.solid,
        verts[0],   // 모서리 시작 꼭짓점
        verts[1],   // 모서리 끝 꼭짓점
        1.5,        // 반경 (mm)
        8,          // 세그먼트 수 (호 근사 해상도)
    )?;

    let mesh = tessellate_solid(&model, filleted.solid);
    export_stl_binary(&mesh, "filleted.stl")?;

    Ok(())
}
```
