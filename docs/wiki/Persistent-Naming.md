# Persistent Naming

## 문제: Topological Naming Problem (TNP)

파라메트릭 CAD에서 모델은 이력(history)에 기반해 재구축됩니다. 예를 들어:

```
1. Box 생성 → 6개 면
2. 상단 면에 Fillet 적용
```

여기서 "상단 면"은 어떤 것일까요?

인덱스 기반 참조(`faces[2]`)는 위상 변경 시 깨집니다. 이전 단계에 파라미터를 변경하면 면 인덱스가 바뀔 수 있기 때문입니다. 이것이 **TNP(Topological Naming Problem)**입니다.

## 해결: Tag 기반 Persistent Naming

CADKernel은 인덱스 대신 **의미론적 태그(Tag)**로 엔티티를 참조합니다.

### Tag 구조

```rust
pub struct Tag {
    pub entity_kind: EntityKind,     // Vertex, Edge, Face, Shell, Solid, Wire
    pub operation: OperationId,       // 생성 연산 ID
    pub index: u32,                   // 연산 내 순번
    pub segment_kind: SegmentKind,    // Generated | Split | Modified
    pub parent: Option<Box<Tag>>,     // 파생 원본
}
```

### 생성 패턴

| 패턴 | 메서드 | 용도 |
|------|--------|------|
| 최초 생성 | `Tag::generated(kind, op, idx)` | Extrude/Revolve/Sweep이 면 생성 |
| 분할 | `tag.split(op, idx)` | Boolean이 기존 면을 분할 |
| 수정 | `tag.modified(op)` | Fillet이 기존 면을 변형 |
| 결합 | `tag.merged(other, op)` | Boolean Union에서 면 병합 |

### EntityKind

```rust
pub enum EntityKind {
    Vertex,
    Edge,
    Face,
    Shell,
    Solid,
    Wire,
}
```

## NameMap

Tag → Handle 매핑을 관리합니다.

```rust
pub struct NameMap {
    entries: HashMap<Tag, EntityRef>,
}

pub enum EntityRef {
    Vertex(Handle<VertexData>),
    Edge(Handle<EdgeData>),
    Face(Handle<FaceData>),
    Wire(Handle<WireData>),
    Shell(Handle<ShellData>),
    Solid(Handle<SolidData>),
}
```

### API

```rust
let mut map = NameMap::new();

// 삽입
map.insert(tag.clone(), EntityRef::Face(face_h));

// 조회
map.get(&tag)          → Option<&EntityRef>
map.get_vertex(&tag)   → Option<Handle<VertexData>>
map.get_edge(&tag)     → Option<Handle<EdgeData>>
map.get_face(&tag)     → Option<Handle<FaceData>>
map.get_wire(&tag)     → Option<Handle<WireData>>
map.get_shell(&tag)    → Option<Handle<ShellData>>
map.get_solid(&tag)    → Option<Handle<SolidData>>
```

## ShapeHistory

모델의 연산 이력을 추적합니다.

```rust
pub struct ShapeHistory {
    records: Vec<EvolutionRecord>,
    next_op: OperationId,
}

pub struct EvolutionRecord {
    pub operation: OperationId,
    pub description: String,
    pub created: Vec<Tag>,
    pub deleted: Vec<Tag>,
    pub modified: Vec<(Tag, Tag)>,  // (old, new)
}
```

### API

```rust
let op = model.history.new_operation("extrude Z=5.0".into());
model.history.record_created(op, &tag);
model.history.record_deleted(op, &tag);
model.history.record_modified(op, &old_tag, &new_tag);

let records = model.history.records();
```

## 실전 예제: Extrude + 면 참조

```rust
let mut model = BRepModel::new();

// Extrude는 내부적으로 태그를 자동 부여
let ext = extrude(&mut model, &profile, Vec3::Z, 5.0)?;
let op = model.history.records().last().unwrap().operation;

// 바닥 면 참조 (인덱스가 아닌 태그로)
let bottom_tag = Tag::generated(EntityKind::Face, op, 0);
let bottom_face = model.find_face_by_tag(&bottom_tag).unwrap();

// 나중에 모델이 변경되어도 태그로 안전하게 찾을 수 있음
assert_eq!(bottom_face, ext.bottom_face);
```

## 설계 원칙

1. **불변 식별자**: Tag는 모델 재구축 후에도 동일한 논리적 엔티티를 가리킴
2. **계보 추적**: `parent` 필드로 원본 엔티티까지의 계보(lineage) 추적 가능
3. **연산 캡슐화**: `OperationId`로 어떤 연산이 언제 어떤 엔티티를 생성/수정했는지 기록
4. **일관성**: 모든 Feature Operation (Extrude, Revolve, Sweep, Boolean, Primitive)이 동일한 태깅 프로토콜 사용
