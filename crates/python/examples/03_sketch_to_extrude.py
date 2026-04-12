"""Example 3: Parametric sketch to extruded solid.

Demonstrates the sketch-based workflow: create 2D geometry with
constraints, solve the constraint system, then extrude into 3D.
"""

import cadkernel as ck

# --- L-shaped profile ---

sketch = ck.Sketch()

# Define points for an L-shape (counterclockwise)
#   p5 --- p4
#   |       |
#   |   p2--p3
#   |   |
#   p0--p1
p0 = sketch.add_point(0.0, 0.0)
p1 = sketch.add_point(20.0, 0.0)
p2 = sketch.add_point(20.0, 10.0)
p3 = sketch.add_point(10.0, 10.0)
p4 = sketch.add_point(10.0, 25.0)
p5 = sketch.add_point(0.0, 25.0)

# Connect points with lines
l0 = sketch.add_line(p0, p1)   # bottom
l1 = sketch.add_line(p1, p2)   # right-bottom
l2 = sketch.add_line(p2, p3)   # step
l3 = sketch.add_line(p3, p4)   # right-top
l4 = sketch.add_line(p4, p5)   # top
l5 = sketch.add_line(p5, p0)   # left

# Add constraints
sketch.constrain_fixed(p0, 0.0, 0.0)       # anchor the origin point
sketch.constrain_horizontal(l0)             # bottom edge horizontal
sketch.constrain_horizontal(l2)             # step horizontal
sketch.constrain_horizontal(l4)             # top edge horizontal
sketch.constrain_vertical(l1)               # right-bottom vertical
sketch.constrain_vertical(l3)               # right-top vertical
sketch.constrain_vertical(l5)               # left side vertical
sketch.constrain_length(l0, 20.0)           # bottom 20mm
sketch.constrain_length(l5, 25.0)           # left side 25mm

# Solve the constraint system
converged = sketch.solve(200, 1e-10)
print(f"Sketch solved: converged={converged}")
print(f"Sketch: {sketch}")

# Extrude the L-shape 15mm along Z (XY plane normal)
model = ck.Model()
solid = ck.extrude_profile(model, sketch, "XY", 15.0)
print(f"Extruded solid: {solid}")
print(f"Model: {model}")

# Tessellate and export
mesh = ck.tessellate(model, solid)
ck.export_stl("/tmp/l_shape.stl", mesh)
print(f"Mesh: {mesh}")

# Measure the result
props = ck.mass_properties(mesh)
print(f"Volume: {props.volume:.2f} mm^3")
print(f"Surface area: {props.surface_area:.2f} mm^2")

# --- Simple rectangle extruded on XZ plane ---

sketch2 = ck.Sketch()
a = sketch2.add_point(0.0, 0.0)
b = sketch2.add_point(30.0, 0.0)
c = sketch2.add_point(30.0, 15.0)
d = sketch2.add_point(0.0, 15.0)
sketch2.add_line(a, b)
sketch2.add_line(b, c)
sketch2.add_line(c, d)
sketch2.add_line(d, a)
sketch2.constrain_fixed(a, 0.0, 0.0)
sketch2.solve(100, 1e-10)

model2 = ck.Model()
xz_solid = ck.extrude_profile(model2, sketch2, "XZ", 10.0)
print(f"\nXZ-plane extrude: {xz_solid}")

mesh2 = ck.tessellate(model2, xz_solid)
ck.export_stl("/tmp/xz_extrude.stl", mesh2)

# --- Revolve a profile ---

sketch3 = ck.Sketch()
r0 = sketch3.add_point(5.0, 0.0)
r1 = sketch3.add_point(10.0, 0.0)
r2 = sketch3.add_point(10.0, 8.0)
r3 = sketch3.add_point(5.0, 8.0)
sketch3.add_line(r0, r1)
sketch3.add_line(r1, r2)
sketch3.add_line(r2, r3)
sketch3.add_line(r3, r0)
sketch3.constrain_fixed(r0, 5.0, 0.0)
sketch3.solve(100, 1e-10)

model3 = ck.Model()
rev_solid = ck.revolve_profile(
    model3, sketch3, "XY",
    [0.0, 0.0, 0.0],     # axis origin
    [0.0, 1.0, 0.0],     # axis direction (Y-axis)
    6.283185,             # full revolution (2*pi)
    32,                   # segments
)
print(f"\nRevolve solid: {rev_solid}")

mesh3 = ck.tessellate(model3, rev_solid)
ck.export_stl("/tmp/revolved.stl", mesh3)

print("\nAll sketch-based exports complete.")
