"""Example 2: Boolean operations (union, subtract, intersect, XOR).

Demonstrates combining solids with boolean operations to create
complex geometry from simple primitives.
"""

import cadkernel as ck

# --- Boolean Subtract: Box with cylindrical hole ---

model_box = ck.Model()
box_solid = ck.create_box(model_box, [0.0, 0.0, 0.0], 20.0, 20.0, 10.0)

model_cyl = ck.Model()
cyl_solid = ck.create_cylinder(model_cyl, [10.0, 10.0, -1.0], 4.0, 12.0)

result_sub = ck.boolean_subtract(model_box, box_solid, model_cyl, cyl_solid)
result_solid = result_sub.get_first_solid()
mesh_sub = ck.tessellate(result_sub, result_solid)
print(f"Subtract (box - cylinder): {mesh_sub}")
ck.export_stl("/tmp/bool_subtract.stl", mesh_sub)

# --- Boolean Union: Two overlapping boxes ---

model_a = ck.Model()
box_a = ck.create_box(model_a, [0.0, 0.0, 0.0], 15.0, 10.0, 10.0)

model_b = ck.Model()
box_b = ck.create_box(model_b, [10.0, 5.0, 0.0], 15.0, 10.0, 10.0)

result_union = ck.boolean_union(model_a, box_a, model_b, box_b)
solid_union = result_union.get_first_solid()
mesh_union = ck.tessellate(result_union, solid_union)
print(f"Union (box + box): {mesh_union}")
ck.export_stl("/tmp/bool_union.stl", mesh_union)

# --- Boolean Intersect: Box and sphere overlap ---

model_box2 = ck.Model()
box2 = ck.create_box(model_box2, [-5.0, -5.0, -5.0], 10.0, 10.0, 10.0)

model_sph = ck.Model()
sph = ck.create_sphere(model_sph, [0.0, 0.0, 0.0], 7.0)

result_int = ck.boolean_intersect(model_box2, box2, model_sph, sph)
solid_int = result_int.get_first_solid()
mesh_int = ck.tessellate(result_int, solid_int)
print(f"Intersect (box & sphere): {mesh_int}")
ck.export_stl("/tmp/bool_intersect.stl", mesh_int)

# --- Boolean XOR: Two boxes, keep non-overlapping regions ---

model_x = ck.Model()
box_x = ck.create_box(model_x, [0.0, 0.0, 0.0], 12.0, 12.0, 12.0)

model_y = ck.Model()
box_y = ck.create_box(model_y, [6.0, 6.0, 0.0], 12.0, 12.0, 12.0)

result_xor = ck.boolean_xor_op(model_x, box_x, model_y, box_y)
solid_xor = result_xor.get_first_solid()
mesh_xor = ck.tessellate(result_xor, solid_xor)
print(f"XOR (box ^ box): {mesh_xor}")
ck.export_stl("/tmp/bool_xor.stl", mesh_xor)

print("\nAll boolean results exported to /tmp/")
