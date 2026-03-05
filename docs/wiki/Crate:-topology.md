# Crate: cadkernel-topology

> **역할**: B-Rep (Boundary Representation) 반변 자료구조, Persistent Naming, Undo/Redo, 속성 시스템  
> **의존성**: `cadkernel-core`, `cadkernel-math`, `cadkernel-geometry` (feature flag)  
> **경로**: `crates/topology/`

## Entity 계층

```
SolidData ← ShellData ← FaceData ← LoopData ← HalfEdgeData
                                                    ↕
                                               EdgeData ← VertexData
                                                    ↕
                                               WireData (독립 체인)
```

| 엔티티 | 구조체 | 핵심 필드 |
|--------|--------|----------|
| Vertex | `VertexData` | `point: Point3`, `tag: Option<Tag>` |
| Edge | `EdgeData` | `start`, `end`, `half_edge_a/b`, `curve`* |
| HalfEdge | `HalfEdgeData` | `origin`, `twin`, `next`, `prev`, `edge`, `loop_ref` |
| Loop | `LoopData` | `half_edge`, `face` |
| Wire | `WireData` | `half_edges: Vec`, `is_closed` |
| Face | `FaceData` | `outer_loop`, `inner_loops`, `surface`*, `orientation` |
| Shell | `ShellData` | `faces: Vec` |
| Solid | `SolidData` | `shells: Vec` |

> *`curve`/`surface` 필드는 `geometry-binding` feature가 활성일 때만 존재

## EntityStore<T>

Arena 기반 O(1) 저장소. Generation 카운터로 use-after-free 방지.

```rust
let mut store = EntityStore::new();
let h = store.insert(data);         // O(1) insert → Handle<T>
let val = store.get(h);             // O(1) lookup (generation 체크)
let val = store.get_mut(h);         // O(1) mutable lookup
store.remove(h);                    // O(1) remove (generation 증가)
store.len();                        // O(1) (alive_count 캐시)
store.is_alive(h);                  // handle 유효성 확인
for (handle, value) in store.iter() { ... }
```

## BRepModel API

### 생성

```rust
let mut model = BRepModel::new();

let v = model.make_vertex(Point3::new(0.0, 0.0, 0.0));
let e = model.add_edge(v1, v2);
let l = model.make_loop(&[he1, he2, he3])?;  // KernelResult
let f = model.make_face(l);
let s = model.make_shell(&[f1, f2, f3]);
let solid = model.make_solid(&[s]);
let w = model.make_wire(half_edges, is_closed);
```

### Persistent Naming 생성

```rust
let f = model.make_face_tagged(loop_h, tag);
let w = model.make_wire_tagged(hes, true, tag);
let s = model.make_shell_tagged(faces, tag);
let solid = model.make_solid_tagged(shells, tag);
```

### Tag 조회

```rust
model.find_vertex_by_tag(&tag)   → Option<Handle<VertexData>>
model.find_edge_by_tag(&tag)     → Option<Handle<EdgeData>>
model.find_face_by_tag(&tag)     → Option<Handle<FaceData>>
model.find_wire_by_tag(&tag)     → Option<Handle<WireData>>
model.find_shell_by_tag(&tag)    → Option<Handle<ShellData>>
model.find_solid_by_tag(&tag)    → Option<Handle<SolidData>>
```

### 순회 (Traversal)

```rust
model.loop_half_edges(he)              → Vec<Handle<HalfEdgeData>>
model.vertices_of_face(face)?          → KernelResult<Vec<Handle<VertexData>>>
model.edges_of_face(face)?             → KernelResult<Vec<Handle<EdgeData>>>
model.faces_of_edge(edge)?             → KernelResult<Vec<Handle<FaceData>>>
model.faces_around_vertex(vertex)?     → KernelResult<Vec<Handle<FaceData>>>
```

### 검증 & 변환

```rust
model.validate()?;              // twin 대칭, 루프 순환, 오일러 특성
model.transform(&transform);    // 모든 정점에 어파인 변환
```

### 향상된 검증 API *(Phase 10)*

기본 `validate()` 외에 더 세밀한 검증 기능이 추가되었습니다.

```rust
// 매니폴드 검증 (모든 edge가 정확히 2개 face에 인접)
model.validate_manifold()?;

// 상세 검증 (모든 이슈를 수집하여 반환)
let issues: Vec<ValidationIssue> = model.validate_detailed();
for issue in &issues {
    match issue.severity {
        ValidationSeverity::Error => eprintln!("ERROR: {}", issue.message),
        ValidationSeverity::Warning => eprintln!("WARN:  {}", issue.message),
    }
}
```

#### ValidationIssue & ValidationSeverity

```rust
pub struct ValidationIssue {
    pub severity: ValidationSeverity,
    pub message: String,
}

pub enum ValidationSeverity {
    Error,    // 구조적 결함 (댕글링 참조, twin 불일치 등)
    Warning,  // 잠재적 문제 (방향 불일치 등)
}
```

**검증 항목**:
- 기존: twin 대칭, 루프 순환, 오일러 특성
- 신규: 댕글링 참조 감지 (삭제된 엔티티 참조), 면 방향 일관성 체크

## Persistent Naming 시스템

> 상세: [[Persistent Naming]] 페이지 참조

```rust
// Tag 생성
let tag = Tag::generated(EntityKind::Face, OperationId(1), 0);
let derived = tag.split(OperationId(2), 1);
let modified = tag.modified(OperationId(3));

// NameMap
let mut map = NameMap::new();
map.insert(tag.clone(), EntityRef::Face(face_h));
map.get_face(&tag) → Option<Handle<FaceData>>
```

## Undo/Redo *(Phase 15)*

스냅샷 기반 Undo/Redo 시스템입니다. 각 연산 전후에 모델 전체를 스냅샷으로 저장합니다.

```rust
use cadkernel_topology::{BRepModel, ModelHistory};

let model = BRepModel::new();
let mut history = ModelHistory::new(model, 50);  // 최대 50 스냅샷

// 연산 수행 후 기록
let mut m = history.current_model().clone();
m.add_vertex(Point3::new(1.0, 0.0, 0.0));
history.record(m, "add vertex");

// Undo
if history.can_undo() {
    let restored = history.undo().unwrap();
    assert_eq!(restored.vertices.len(), 0);
}

// Redo
if history.can_redo() {
    let restored = history.redo().unwrap();
    assert_eq!(restored.vertices.len(), 1);
}

// 이력 확인
let descs = history.history_descriptions();
println!("Undo steps: {}", history.undo_count());
println!("Redo steps: {}", history.redo_count());
```

## 속성 시스템 *(Phase 15)*

엔티티에 재질, 색상, 메타데이터를 부여합니다.

### Color

```rust
use cadkernel_topology::Color;

let c = Color::rgb(0.8, 0.2, 0.1);
let c_alpha = Color::rgba(0.8, 0.2, 0.1, 0.5);

// 상수: Color::RED, GREEN, BLUE, WHITE, BLACK, GRAY
```

### Material 프리셋

```rust
use cadkernel_topology::Material;

let steel = Material::steel();       // 밀도 7850 kg/m³, metallic
let alu   = Material::aluminum();    // 밀도 2700 kg/m³
let abs   = Material::plastic_abs(); // 밀도 1050 kg/m³
let wood  = Material::wood();        // 밀도 600 kg/m³

// 커스텀 재질
let custom = Material::new("Titanium")
    .with_density(4500.0)
    .with_color(Color::rgb(0.7, 0.7, 0.75))
    .with_metallic(1.0)
    .with_roughness(0.25);
```

### PropertyStore

```rust
use cadkernel_topology::{PropertyStore, PropertyValue};

// BRepModel에 이미 포함됨 (model.properties)
model.properties.set_material(solid_index, Material::steel());
model.properties.set_metadata(solid_index, "part_number", PropertyValue::String("A-100".into()));

let mat = model.properties.get_material(solid_index);
let val = model.properties.get_metadata(solid_index, "part_number");
```

## Feature Flag

```toml
[features]
default = ["geometry-binding"]
geometry-binding = ["dep:cadkernel-geometry"]
```

**활성 시**: `EdgeData`에 `pub curve: Option<Arc<dyn Curve + Send + Sync>>`, `FaceData`에 `pub surface: Option<Arc<dyn Surface + Send + Sync>>` 필드 포함.

## 파일 구조

```
crates/topology/src/
├── lib.rs           ← BRepModel + API
├── handle.rs        ← Handle<T>
├── store.rs         ← EntityStore<T>
├── vertex.rs        ← VertexData
├── edge.rs          ← EdgeData (+ geometry binding)
├── halfedge.rs      ← HalfEdgeData
├── loop_wire.rs     ← LoopData
├── wire.rs          ← WireData
├── face.rs          ← FaceData, Orientation
├── shell.rs         ← ShellData
├── solid.rs         ← SolidData
├── error.rs         ← re-export from core
├── prelude.rs
├── history.rs       ← ModelHistory (Undo/Redo)
├── properties.rs    ← Color, Material, PropertyValue, PropertyStore
├── validation.rs    ← Euler 수 검증, 매니폴드 체크
└── naming/
    ├── mod.rs
    ├── tag.rs       ← Tag, EntityKind, OperationId, SegmentKind
    ├── name_map.rs  ← NameMap, EntityRef
    └── history.rs   ← ShapeHistory, EvolutionRecord
```
