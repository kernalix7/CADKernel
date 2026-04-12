"""Example 5: File format conversion.

Demonstrates converting between file formats: create geometry,
export to multiple formats, and re-import.
"""

import os
import cadkernel as ck

OUTPUT_DIR = "/tmp/cadkernel_formats"
os.makedirs(OUTPUT_DIR, exist_ok=True)

# Create a model with a torus (interesting geometry for format testing)
model = ck.Model()
torus = ck.create_torus(model, [0.0, 0.0, 0.0], 8.0, 3.0)
mesh = ck.tessellate(model, torus)
print(f"Source mesh: {mesh}")

# --- Export to all mesh-based formats ---

stl_path = os.path.join(OUTPUT_DIR, "torus.stl")
ck.export_stl(stl_path, mesh)
print(f"Exported STL: {stl_path}")

stl_bin_path = os.path.join(OUTPUT_DIR, "torus_binary.stl")
ck.export_stl_binary(stl_bin_path, mesh)
print(f"Exported binary STL: {stl_bin_path}")

obj_path = os.path.join(OUTPUT_DIR, "torus.obj")
ck.export_obj(obj_path, mesh)
print(f"Exported OBJ: {obj_path}")

gltf_path = os.path.join(OUTPUT_DIR, "torus.glb")
ck.export_gltf(gltf_path, mesh)
print(f"Exported glTF: {gltf_path}")

ply_path = os.path.join(OUTPUT_DIR, "torus.ply")
ck.export_ply(ply_path, mesh)
print(f"Exported PLY: {ply_path}")

dxf_path = os.path.join(OUTPUT_DIR, "torus.dxf")
ck.export_dxf(dxf_path, mesh)
print(f"Exported DXF: {dxf_path}")

threemf_path = os.path.join(OUTPUT_DIR, "torus.3mf")
ck.export_3mf(threemf_path, mesh)
print(f"Exported 3MF: {threemf_path}")

dae_path = os.path.join(OUTPUT_DIR, "torus.dae")
ck.export_dae(dae_path, mesh)
print(f"Exported DAE: {dae_path}")

amf_path = os.path.join(OUTPUT_DIR, "torus.amf")
ck.export_amf(amf_path, mesh)
print(f"Exported AMF: {amf_path}")

# --- Export B-Rep model formats ---

step_path = os.path.join(OUTPUT_DIR, "torus.step")
ck.export_step(step_path, model)
print(f"Exported STEP: {step_path}")

iges_path = os.path.join(OUTPUT_DIR, "torus.iges")
ck.export_iges(iges_path, model)
print(f"Exported IGES: {iges_path}")

brep_path = os.path.join(OUTPUT_DIR, "torus.brep")
ck.export_brep(brep_path, model)
print(f"Exported BREP: {brep_path}")

# --- Native format (.cadk) ---

cadk_path = os.path.join(OUTPUT_DIR, "torus.cadk")
ck.save_project(cadk_path, model)
print(f"Saved project: {cadk_path}")

# --- Re-import and verify roundtrip ---

print("\n--- Roundtrip verification ---")

# STL roundtrip
imported_stl = ck.import_stl(stl_path)
print(f"STL reimport: {imported_stl} (original: {mesh.triangle_count()} tri)")

# OBJ roundtrip
imported_obj = ck.import_obj(obj_path)
print(f"OBJ reimport: {imported_obj}")

# PLY roundtrip
imported_ply = ck.import_ply(ply_path)
print(f"PLY reimport: {imported_ply}")

# DXF roundtrip
imported_dxf = ck.import_dxf(dxf_path)
print(f"DXF reimport: {imported_dxf}")

# DAE roundtrip
imported_dae = ck.import_dae(dae_path)
print(f"DAE reimport: {imported_dae}")

# BREP roundtrip
imported_brep = ck.import_brep(brep_path)
print(f"BREP reimport: {imported_brep}")

# Native format roundtrip
loaded = ck.load_project(cadk_path)
print(f"Project reimport: {loaded}")

# Compare file sizes
print("\n--- File sizes ---")
for name in sorted(os.listdir(OUTPUT_DIR)):
    path = os.path.join(OUTPUT_DIR, name)
    size = os.path.getsize(path)
    if size > 1024:
        print(f"  {name}: {size / 1024:.1f} KB")
    else:
        print(f"  {name}: {size} B")

print("\nAll format conversions complete.")
