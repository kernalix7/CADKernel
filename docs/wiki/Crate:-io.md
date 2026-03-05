# Crate: cadkernel-io

> **역할**: B-Rep 모델의 메시 테셀레이션, 파일 내보내기/가져오기, SVG 2D 출력, JSON 직렬화, STEP/IGES 산업 형식, glTF 내보내기  
> **의존성**: `cadkernel-core`, `cadkernel-math`, `cadkernel-topology`, `rayon`  
> **경로**: `crates/io/`

## 테셀레이션

B-Rep Face/Solid를 삼각형 메시로 변환합니다.

```rust
let mesh = tessellate_solid(&model, solid_handle);
let mesh = tessellate_face(&model, face_handle);
```

### Mesh 구조

```rust
pub struct Mesh {
    pub vertices: Vec<Point3>,
    pub indices: Vec<[u32; 3]>,  // 삼각형 인덱스
}

impl Mesh {
    pub fn triangle_count(&self) -> usize;
    pub fn to_triangles(&self) -> Vec<Triangle>;
}

pub struct Triangle {
    pub v0: Point3,
    pub v1: Point3,
    pub v2: Point3,
}
```

## STL 내보내기

### ASCII STL

```rust
// 문자열로 생성
let stl_text = write_stl_ascii(&mesh, "part_name");

// 파일로 직접 저장
export_stl_ascii(&mesh, "output.stl", "part_name")?;
```

출력 예시:
```
solid part_name
  facet normal 0.000000e0 0.000000e0 -1.000000e0
    outer loop
      vertex 0.000000e0 0.000000e0 0.000000e0
      vertex 1.000000e1 0.000000e0 0.000000e0
      vertex 1.000000e1 5.000000e0 0.000000e0
    endloop
  endfacet
  ...
endsolid part_name
```

### Binary STL

```rust
let stl_bytes: Vec<u8> = write_stl_binary(&mesh);
export_stl_binary(&mesh, "output.stl")?;
```

Binary STL 구조: 80 byte header + 4 byte triangle count + N × 50 byte triangles.

## OBJ 내보내기

```rust
let obj_text = write_obj(&mesh);
export_obj(&mesh, "output.obj")?;
```

출력 예시:
```
# CADKernel OBJ export
v 0.000000 0.000000 0.000000
v 10.000000 0.000000 0.000000
v 10.000000 5.000000 0.000000
...
f 1 2 3
f 1 3 4
...
```

## 전체 파이프라인 예제

```rust
use cadkernel::prelude::*;

fn main() -> KernelResult<()> {
    let mut model = BRepModel::new();
    let cyl = make_cylinder(&mut model, 2.0, 5.0, 32)?;
    
    let mesh = tessellate_solid(&model, cyl.solid);
    
    // 여러 형식으로 내보내기
    export_stl_ascii(&mesh, "cylinder.stl", "cylinder")?;
    export_stl_binary(&mesh, "cylinder_bin.stl")?;
    export_obj(&mesh, "cylinder.obj")?;
    
    println!("Exported {} triangles", mesh.triangle_count());
    Ok(())
}
```

## SVG 2D Export *(Phase 11)*

B-Rep 프로파일을 SVG 벡터 그래픽으로 내보냅니다.

### SvgDocument & SvgElement

```rust
use cadkernel_io::svg::*;

// 직접 구성
let mut doc = SvgDocument::new(200.0, 200.0);
doc.add(SvgElement::Line {
    x1: 10.0, y1: 10.0,
    x2: 190.0, y2: 10.0,
    style: SvgStyle::default(),
});
doc.add(SvgElement::Circle {
    cx: 100.0, cy: 100.0, r: 50.0,
    style: SvgStyle { stroke: "blue".into(), stroke_width: 1.5, fill: "none".into() },
});

let svg_text = doc.to_string();
std::fs::write("drawing.svg", &svg_text)?;
```

### 지원 요소

| SvgElement | 설명 |
|------------|------|
| `Line` | 직선 (x1, y1, x2, y2) |
| `Polyline` | 다중 점 연결선 |
| `Circle` | 원 (cx, cy, r) |
| `Arc` | 호 (SVG arc path) |
| `Polygon` | 다각형 (닫힌 경로) |

### 프로파일 변환

```rust
// 프로파일 점 리스트 → SVG (auto-fit viewBox)
let profile = vec![
    Point3::new(0.0, 0.0, 0.0),
    Point3::new(10.0, 0.0, 0.0),
    Point3::new(10.0, 5.0, 0.0),
    Point3::new(0.0, 5.0, 0.0),
];
let svg_doc = profile_to_svg(&profile);
std::fs::write("profile.svg", svg_doc.to_string())?;
```

## JSON Serialization *(Phase 11)*

BRepModel을 JSON으로 직렬화/역직렬화합니다. 모든 토폴로지 및 수학 타입에 serde `Serialize`/`Deserialize`가 derive되어 있습니다.

### 메모리 내 변환

```rust
// Model → JSON 문자열
let json_str = model_to_json(&model)?;

// JSON 문자열 → Model
let restored: BRepModel = model_from_json(&json_str)?;
```

### 파일 I/O

```rust
// 파일로 쓰기
write_json(&model, "model.json")?;

// 파일에서 읽기
let loaded: BRepModel = read_json("model.json")?;
```

### 편의 함수

```rust
// export/import (내부적으로 write_json/read_json 호출)
export_json(&model, "backup.json")?;
let model = import_json("backup.json")?;
```

### Roundtrip 예제

```rust
use cadkernel::prelude::*;

fn main() -> KernelResult<()> {
    let mut model = BRepModel::new();
    let b = make_box(&mut model, 10.0, 5.0, 3.0)?;

    // JSON으로 저장
    export_json(&model, "box.json")?;

    // JSON에서 복원
    let restored = import_json("box.json")?;
    restored.validate()?;

    Ok(())
}
```

## STEP I/O *(Phase 16)*

ISO 10303-21 (AP214) 형식으로 기하 데이터를 교환합니다.

### STEP 쓰기

```rust
use cadkernel_io::step::*;

let mut writer = StepWriter::new();
let pt_id = writer.add_cartesian_point("origin", Point3::ORIGIN);
let dir_id = writer.add_direction("z_axis", 0.0, 0.0, 1.0);
writer.write(&mut file)?;
```

### STEP 읽기

```rust
// 점 데이터 읽기
let points = read_step_points("model.step")?;

// 전체 엔티티 파싱
let entities = parse_step_entities("model.step")?;
for entity in &entities {
    println!("#{}: {} -> {}", entity.id, entity.entity_type, entity.data);
}
```

### 메시 → STEP 내보내기

```rust
export_step_mesh(&mesh, "output.step")?;
```

## IGES I/O *(Phase 16)*

IGES 5.3 고정폭 80열 포맷으로 기본 기하를 교환합니다.

### IGES 쓰기

```rust
use cadkernel_io::iges::*;

let mut writer = IgesWriter::new();
writer.add_point(Point3::new(1.0, 2.0, 3.0));
writer.add_line(Point3::ORIGIN, Point3::new(10.0, 0.0, 0.0));
writer.write(&mut file)?;
```

### IGES 읽기

```rust
let points = read_iges_points("model.iges")?;
let lines = read_iges_lines("model.iges")?;
```

### 지원 엔티티 타입

| IGES 타입 | 코드 | 설명 |
|-----------|:----:|------|
| `CircularArc` | 100 | 원호 |
| `CompositeCurve` | 102 | 복합 곡선 |
| `Line` | 110 | 직선 |
| `Point` | 116 | 점 |

## 파일 구조

```
crates/io/src/
├── lib.rs          ← re-exports
├── tessellate.rs   ← Mesh, Triangle, tessellate_solid/face
├── stl.rs          ← write_stl_ascii/binary, export_stl_*, import_stl
├── obj.rs          ← write_obj, export_obj, import_obj
├── svg.rs          ← SvgDocument, SvgElement, SvgStyle, profile_to_svg
├── json.rs         ← model_to_json, model_from_json, write/read/export/import_json
├── gltf.rs         ← export_gltf (glTF 2.0, embedded base64, per-vertex normals)
├── step.rs         ← StepWriter, StepEntity, read_step_points, parse_step_entities, export_step_mesh
└── iges.rs         ← IgesWriter, IgesEntity, IgesEntityType, read_iges_points, read_iges_lines
```

## 성능 최적화

`rayon` 기반 멀티스레드 처리로 대용량 메시 I/O 성능을 최적화합니다.

- **STL/OBJ 파싱**: `par_iter`를 활용한 병렬 정점/삼각형 파싱
- **정점 중복 제거**: `HashMap` 기반 O(N) 알고리즘 (기존 O(N²) 선형 검색 대비 대폭 개선)
- **glTF 내보내기**: 버퍼 생성, 법선 계산, base64 인코딩 병렬화
- **테셀레이션**: 삼각형 변환 병렬 처리
- **바운딩 박스**: `par_iter` + fold/reduce 병렬 계산
```
