# Getting Started

## 요구사항

| 항목 | 최소 버전 |
|------|----------|
| Rust | 1.85 (Edition 2024) |
| Cargo | 최신 |
| OS | Linux, macOS, Windows |

## 설치 및 빌드

```bash
# 1. 저장소 클론
git clone https://github.com/kernalix7/CADKernel.git
cd CADKernel

# 2. 빌드
cargo build --release

# 3. 테스트
cargo test --workspace

# 4. 린트
cargo clippy --workspace --all-targets --all-features -- -D warnings

# 5. 포맷 검사
cargo fmt --all -- --check
```

## 프로젝트 구조

```
CADKernel/
├── Cargo.toml              ← 워크스페이스 루트
├── src/
│   ├── lib.rs              ← 크레이트 re-export + prelude + E2E 테스트
│   └── main.rs             ← CLI 진입점 (버전 배너)
├── crates/
│   ├── core/               ← cadkernel-core (에러 타입)
│   ├── math/               ← cadkernel-math (수학)
│   ├── geometry/           ← cadkernel-geometry (기하)
│   ├── topology/           ← cadkernel-topology (위상)
│   ├── sketch/             ← cadkernel-sketch (2D 스케치)
│   ├── modeling/           ← cadkernel-modeling (모델링)
│   └── io/                 ← cadkernel-io (파일 I/O)
├── docs/
│   └── wiki/               ← 이 Wiki 원본
├── .github/
│   └── workflows/ci.yml    ← CI 파이프라인
└── *.md                    ← 프로젝트 문서
```

## 첫 번째 프로그램

`Cargo.toml`:
```toml
[dependencies]
cadkernel = { path = "../CADKernel" }
```

`main.rs`:
```rust
use cadkernel::prelude::*;

fn main() -> KernelResult<()> {
    // 1. 박스 생성
    let mut model = BRepModel::new();
    let b = make_box(&mut model, 10.0, 5.0, 3.0)?;

    // 2. 질량 특성 계산
    let props = solid_mass_properties(&model, b.solid);
    println!("Volume: {:.2}", props.volume);
    println!("Area:   {:.2}", props.surface_area);
    println!("Center: {}", props.centroid);

    // 3. STL 내보내기
    let mesh = tessellate_solid(&model, b.solid);
    export_stl_ascii(&mesh, "box.stl", "my_box")?;
    println!("Exported {} triangles to box.stl", mesh.triangle_count());

    Ok(())
}
```

출력:
```
Volume: 150.00
Area:   190.00
Center: (5, 2.5, 1.5)
Exported 12 triangles to box.stl
```

## 2D 스케치 → 3D 솔리드 파이프라인

```rust
use cadkernel::prelude::*;

fn main() -> KernelResult<()> {
    // 1. 스케치 생성
    let mut sketch = Sketch::new();
    let p0 = sketch.add_point(0.0, 0.0);
    let p1 = sketch.add_point(10.0, 0.0);
    let p2 = sketch.add_point(10.0, 5.0);
    let p3 = sketch.add_point(0.0, 5.0);

    let l0 = sketch.add_line(p0, p1);
    let l1 = sketch.add_line(p1, p2);
    let l2 = sketch.add_line(p2, p3);
    let l3 = sketch.add_line(p3, p0);

    // 2. 제약 조건
    sketch.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
    sketch.add_constraint(Constraint::Horizontal(l0));
    sketch.add_constraint(Constraint::Vertical(l1));
    sketch.add_constraint(Constraint::Horizontal(l2));
    sketch.add_constraint(Constraint::Vertical(l3));
    sketch.add_constraint(Constraint::Length(l0, 10.0));
    sketch.add_constraint(Constraint::Length(l1, 5.0));

    // 3. 솔브
    let result = solve(&mut sketch, 200, 1e-10);
    assert!(result.converged);

    // 4. 3D 프로파일 추출
    let wp = WorkPlane::xy();
    let profile = extract_profile(&sketch, &wp);

    // 5. 돌출
    let mut model = BRepModel::new();
    let ext = extrude(&mut model, &profile, Vec3::Z, 3.0)?;

    // 6. 검증
    model.validate()?;
    println!("Solid created: {} faces", model.faces.len());

    Ok(())
}
```

## 개발 워크플로우

```bash
# 코드 수정 후 전체 검증 (하나의 커맨드)
cargo fmt --all && \
cargo clippy --workspace --all-targets --all-features -- -D warnings && \
cargo test --workspace
```

커밋 규칙: [Conventional Commits](https://www.conventionalcommits.org/)

```
feat: add sweep operation
fix: handle empty NURBS control points
refactor: extract cadkernel-core crate
test: add mass properties unit tests
docs: update developer wiki
```
