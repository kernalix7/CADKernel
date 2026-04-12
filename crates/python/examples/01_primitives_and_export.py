"""Example 1: Primitive creation and export to multiple formats.

Creates several primitive shapes, tessellates them, and exports to
STL, OBJ, and glTF formats.
"""

import cadkernel as ck

# Create a model to hold our shapes
model = ck.Model()

# Box: 20mm x 10mm x 15mm at origin
box_solid = ck.create_box(model, [0.0, 0.0, 0.0], 20.0, 10.0, 15.0)
print(f"Box created: {box_solid}")

# Cylinder: radius 5mm, height 20mm, offset to the right
cyl_solid = ck.create_cylinder(model, [30.0, 0.0, 0.0], 5.0, 20.0)
print(f"Cylinder created: {cyl_solid}")

# Sphere: radius 8mm
sphere_solid = ck.create_sphere(model, [0.0, 30.0, 0.0], 8.0)
print(f"Sphere created: {sphere_solid}")

# Cone: base radius 6, top radius 2, height 12
cone_solid = ck.create_cone(model, [30.0, 30.0, 0.0], 6.0, 2.0, 12.0)
print(f"Cone created: {cone_solid}")

# Torus: major radius 10, minor radius 3
torus_solid = ck.create_torus(model, [60.0, 0.0, 0.0], 10.0, 3.0)
print(f"Torus created: {torus_solid}")

# Tube (hollow cylinder): outer 8, inner 5, height 15
tube_solid = ck.create_tube(model, [60.0, 30.0, 0.0], 8.0, 5.0, 15.0)
print(f"Tube created: {tube_solid}")

# Hexagonal prism: radius 6, height 10, 6 sides
prism_solid = ck.create_prism(model, [0.0, 60.0, 0.0], 6.0, 10.0, 6)
print(f"Prism created: {prism_solid}")

print(f"\nModel summary: {model}")

# Tessellate each solid and export individually
shapes = [
    ("box", box_solid),
    ("cylinder", cyl_solid),
    ("sphere", sphere_solid),
    ("cone", cone_solid),
    ("torus", torus_solid),
    ("tube", tube_solid),
    ("prism", prism_solid),
]

for name, solid in shapes:
    mesh = ck.tessellate(model, solid)
    print(f"  {name}: {mesh}")

    # Export to STL (ASCII)
    ck.export_stl(f"/tmp/{name}.stl", mesh)

# Export the box mesh to multiple formats
box_mesh = ck.tessellate(model, box_solid)
ck.export_stl("/tmp/box.stl", box_mesh)
ck.export_obj("/tmp/box.obj", box_mesh)
ck.export_gltf("/tmp/box.glb", box_mesh)

# Measure the box
props = ck.mass_properties(box_mesh)
print(f"\nBox mass properties: {props}")
print(f"  Volume: {props.volume:.2f}")
print(f"  Surface area: {props.surface_area:.2f}")
print(f"  Centroid: {props.centroid}")

print("\nAll exports complete. Files saved to /tmp/")
