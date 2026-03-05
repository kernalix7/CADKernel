# Error Handling

## 원칙

> **공개 API에서는 절대 panic하지 않는다.**

| 항목 | 규칙 |
|------|------|
| 공개 API | 반드시 `KernelResult<T>` 반환 |
| 내부 불변식 | `debug_assert!` (디버그 빌드에서만) |
| 테스트 코드 | `.unwrap()` / `.expect()` 허용 |
| `unwrap()` in src | **금지** (clippy 수준에서 차단) |

## KernelError 변형

```rust
pub enum KernelError {
    InvalidHandle(&'static str),    // 삭제된/잘못된 Handle 접근
    InvalidArgument(String),         // 생성자 입력 검증 실패
    ValidationFailed(String),        // B-Rep 구조 검증 실패
    TopologyError(String),           // 위상 연산 오류
    GeometryError(String),           // 기하 연산 오류
    IoError(String),                 // 파일 I/O 오류
}
```

## 가이드라인

### 생성자에서 검증

```rust
// GOOD: Result 반환
pub fn new(center: Point3, normal: Vec3, radius: f64) -> KernelResult<Self> {
    if normal.length_squared() < 1e-20 {
        return Err(KernelError::InvalidArgument(
            "circle normal must be non-zero".into()
        ));
    }
    if radius <= 0.0 {
        return Err(KernelError::InvalidArgument(
            format!("radius must be positive, got {radius}")
        ));
    }
    Ok(Self { center, normal: normal.normalized(), radius })
}

// BAD: panic
pub fn new(center: Point3, normal: Vec3, radius: f64) -> Self {
    assert!(radius > 0.0);  // ← 런타임 panic!
    Self { center, normal: normal.normalized(), radius }
}
```

### `?` 연산자로 전파

```rust
pub fn make_box(model: &mut BRepModel, dx: f64, dy: f64, dz: f64) -> KernelResult<BoxResult> {
    if dx <= 0.0 || dy <= 0.0 || dz <= 0.0 {
        return Err(KernelError::InvalidArgument(
            format!("dimensions must be positive: ({dx}, {dy}, {dz})")
        ));
    }
    // ... 생성 로직 ...
    model.validate()?;  // 검증 에러 자동 전파
    Ok(BoxResult { solid, faces })
}
```

### IO 에러 변환

```rust
impl From<std::io::Error> for KernelError {
    fn from(e: std::io::Error) -> Self {
        KernelError::IoError(e.to_string())
    }
}

// 사용: std::io::Error가 자동으로 KernelError로 변환
pub fn export_stl_ascii(mesh: &Mesh, path: &str, name: &str) -> KernelResult<()> {
    let content = write_stl_ascii(mesh, name);
    std::fs::write(path, content)?;  // io::Error → KernelError::IoError
    Ok(())
}
```

### Display 메시지

```rust
match err {
    KernelError::InvalidHandle(msg) =>
        "Invalid entity handle: {msg}",
    KernelError::InvalidArgument(msg) =>
        "Invalid argument: {msg}",
    KernelError::ValidationFailed(msg) =>
        "B-Rep validation failed: {msg}",
    KernelError::TopologyError(msg) =>
        "Topology error: {msg}",
    KernelError::GeometryError(msg) =>
        "Geometry error: {msg}",
    KernelError::IoError(msg) =>
        "I/O error: {msg}",
}
```

## 테스트에서의 에러 처리

```rust
#[test]
fn test_valid_box() {
    let mut model = BRepModel::new();
    let b = make_box(&mut model, 1.0, 1.0, 1.0).unwrap();
    assert_eq!(model.faces.len(), 6);
}

#[test]
fn test_invalid_box() {
    let mut model = BRepModel::new();
    let err = make_box(&mut model, -1.0, 1.0, 1.0).unwrap_err();
    assert!(matches!(err, KernelError::InvalidArgument(_)));
}
```

## 안전성 가드 사례

### Division by zero

```rust
// NurbsCurve::tangent_at
let (t0, t1) = (t - h, t + h);
if (t1 - t0).abs() < 1e-14 {
    return Vec3::ZERO;
}

// Ellipse::length (Ramanujan 근사)
let sum = a + b;
if sum.abs() < NUMERIC_ZERO {
    return 0.0;
}
```

### Empty collection

```rust
// NurbsCurve::is_closed
fn is_closed(&self) -> bool {
    let (Some(first), Some(last)) = (self.control_points.first(), self.control_points.last())
    else {
        return false;
    };
    first.approx_eq(*last)
}
```
