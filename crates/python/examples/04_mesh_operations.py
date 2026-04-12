"""Example 4: Mesh operations.

Demonstrates mesh manipulation: decimation, subdivision, smoothing,
boolean operations on meshes, repair, and analysis.
"""

import cadkernel as ck

# Create a sphere mesh to work with
model = ck.Model()
sphere = ck.create_sphere(model, [0.0, 0.0, 0.0], 10.0)
mesh = ck.tessellate(model, sphere)
print(f"Original sphere mesh: {mesh}")

# --- Decimate (reduce triangle count) ---
decimated = ck.decimate(mesh, 0.5)  # target 50% of triangles
print(f"Decimated (50%%): {decimated}")
ck.export_stl("/tmp/mesh_decimated.stl", decimated)

# --- Subdivide (increase resolution) ---
subdivided = ck.mesh_subdivide(mesh)
print(f"Subdivided: {subdivided}")
ck.export_stl("/tmp/mesh_subdivided.stl", subdivided)

# --- Smooth ---
smoothed = ck.mesh_smooth(mesh, 3, 0.5)  # 3 iterations, factor 0.5
print(f"Smoothed: {smoothed}")

# --- Mesh boolean operations ---
model_a = ck.Model()
box_a = ck.create_box(model_a, [-6.0, -6.0, -6.0], 12.0, 12.0, 12.0)
mesh_box = ck.tessellate(model_a, box_a)

model_b = ck.Model()
sph_b = ck.create_sphere(model_b, [0.0, 0.0, 0.0], 8.0)
mesh_sphere = ck.tessellate(model_b, sph_b)

mesh_union = ck.mesh_union(mesh_box, mesh_sphere)
print(f"\nMesh union: {mesh_union}")
ck.export_stl("/tmp/mesh_bool_union.stl", mesh_union)

mesh_diff = ck.mesh_difference(mesh_box, mesh_sphere)
print(f"Mesh difference: {mesh_diff}")
ck.export_stl("/tmp/mesh_bool_diff.stl", mesh_diff)

mesh_isect = ck.mesh_intersection(mesh_box, mesh_sphere)
print(f"Mesh intersection: {mesh_isect}")
ck.export_stl("/tmp/mesh_bool_isect.stl", mesh_isect)

# --- Cut with plane ---
cut = ck.mesh_cut_with_plane(mesh, [0.0, 0.0, 0.0], [0.0, 0.0, 1.0])
print(f"\nCut with XY plane: {cut}")
ck.export_stl("/tmp/mesh_cut.stl", cut)

# --- Harmonize normals ---
harmonized = ck.mesh_harmonize_normals(mesh)
print(f"Harmonized normals: {harmonized}")

# --- Flip normals ---
flipped = ck.mesh_flip_normals(mesh)
print(f"Flipped normals: {flipped}")

# --- Watertight check ---
is_watertight = ck.mesh_is_watertight(mesh)
print(f"\nWatertight: {is_watertight}")

# --- Curvature analysis ---
curvatures = ck.mesh_curvature(mesh)
if curvatures:
    min_c = min(curvatures)
    max_c = max(curvatures)
    avg_c = sum(curvatures) / len(curvatures)
    print(f"Curvature: min={min_c:.4f}, max={max_c:.4f}, avg={avg_c:.4f}")

# --- Scale ---
scaled = ck.mesh_scale(mesh, 2.0, 1.0, 0.5)
print(f"\nScaled (2x, 1x, 0.5x): {scaled}")
ck.export_stl("/tmp/mesh_scaled.stl", scaled)

# --- Evaluate and repair ---
repaired_mesh, issues = ck.mesh_repair(mesh)
print(f"\nRepair report:")
for issue in issues:
    print(f"  {issue}")
print(f"Repaired mesh: {repaired_mesh}")

print("\nAll mesh operations complete.")
