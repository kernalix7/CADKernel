# CADKernel Python Bindings

Python bindings for [CADKernel](https://github.com/kernalix7/CADKernel), an open-source B-Rep CAD kernel built with Rust.

## Features

- **13 Primitive Shapes**: Box, Cylinder, Sphere, Cone, Torus, Tube, Prism, Wedge, Ellipsoid, Helix, Spiral, Polygon, Plane Face
- **Feature Operations**: Extrude, Revolve, Sweep, Loft, Chamfer, Fillet, Shell, Draft, Mirror, Pattern, Boolean (Union/Subtract/Intersect/XOR)
- **PartDesign**: 22 additive/subtractive operations (box, cylinder, sphere, cone, torus, helix, ellipsoid, prism, wedge, loft, pipe)
- **Parametric Sketch**: Points, Lines, Arcs, Circles, Ellipses, B-Splines with 20 constraint types
- **Mesh Operations**: Decimate, Subdivide, Smooth, Boolean, Repair, Remesh, Curvature analysis
- **FEM**: Tetrahedral mesh generation and static analysis
- **I/O Formats**: STL, OBJ, glTF, STEP, IGES, DXF, PLY, 3MF, BREP, DAE, AMF
- **Native Format**: `.cadk` project save/load

## Installation

```bash
pip install cadkernel
```

### Build from source

Requires Rust toolchain (1.85+) and maturin:

```bash
pip install maturin
cd crates/python
maturin develop --release
```

## Quick Start

```python
import cadkernel as ck

# Create a model and add a box
model = ck.Model()
box_solid = ck.create_box(model, [0, 0, 0], 10, 20, 30)

# Tessellate and export
mesh = ck.tessellate(model, box_solid)
ck.export_stl("box.stl", mesh)

# Boolean operations
model_a = ck.Model()
model_b = ck.Model()
box_a = ck.create_box(model_a, [0, 0, 0], 10, 10, 10)
cyl_b = ck.create_cylinder(model_b, [5, 5, -1], 3.0, 12.0)
result = ck.boolean_subtract(model_a, box_a, model_b, cyl_b)

# Parametric sketch
sketch = ck.Sketch()
p0 = sketch.add_point(0, 0)
p1 = sketch.add_point(10, 0)
p2 = sketch.add_point(10, 5)
p3 = sketch.add_point(0, 5)
sketch.add_line(p0, p1)
sketch.add_line(p1, p2)
sketch.add_line(p2, p3)
sketch.add_line(p3, p0)
sketch.solve(100, 1e-10)

model = ck.Model()
solid = ck.extrude_profile(model, sketch, "XY", 15.0)
```

## License

Apache-2.0
