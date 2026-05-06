# Sketch goldens manifest

Each row is a hand-crafted reference sketch with documented intent, constraint topology, and expected solver outcome. The TOML file in this directory is the input; the `.expected.toml` file (generated on demand) is the canonical solved state.

| ID | Title | Entities | Constraints | Expected outcome |
|---|---|---|---|---|
| 001 | Rectangle | 4 lines | 4 horizontal/vertical + 2 distance | WellDetermined |
| 002 | Concentric circles | 2 circles | 1 concentric + 2 radius | WellDetermined |
| 003 | Slot (oblong) | 2 lines + 2 arcs | tangent + equal radius + symmetric + width + length | WellDetermined |
| 004 | Hexagonal pattern | 6 lines | 6 equal length + 6 angle 120° + 1 horizontal | WellDetermined |
| 005 | Triangle inscribed in circle | 3 lines + 1 circle | 3 point-on-circle + 1 radius | WellDetermined (3 DoF: rotation about centre) — under-determined intentionally |
| 006 | Two parallel + redundant tangent | 2 lines + 1 circle | parallel + 2 tangent + 1 distance | OverDetermined (one tangent is implied) |
| 007 | Inconsistent square | 4 lines | 4 horizontal/vertical + 4 equal length + 1 distance contradiction | Inconsistent |
| 008 | Bell crank | 3 lines + 1 arc | mechanical linkage geometry | WellDetermined |
| 009 | Cam profile | NURBS curve | tangent + position constraints | WellDetermined |
| 010 | Gear tooth involute | 4 arcs + 2 lines | tangent at pitch circle | WellDetermined |
| 011 | Plate with bolt holes | rectangle + 4 circles | symmetric + concentric + radius + distances | WellDetermined |
| 012 | Construction lines | 2 construction lines + 1 line | construction-only points + tangent | WellDetermined |

Each `.toml` file in this directory follows this schema (see `001_rectangle.toml` for a worked example):

```toml
[sketch]
name = "Rectangle"
units = "mm"

[[entity]]
id = 1
kind = "Point"
x = 0.0
y = 0.0
anchor = true       # fixes 2 DoF

[[entity]]
id = 2
kind = "Line"
start = 1
end = 3

[[constraint]]
kind = "Distance"
between = [1, 3]
value = 100.0

[expected]
status = "WellDetermined"
positional_residual_max = 1e-12
dimensional_residual_max = 1e-9
solve_iterations_max = 5
```

Tests in `crates/sketch/tests/golden_corpus.rs` iterate this directory; each TOML file becomes one `#[test]`.
