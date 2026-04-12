"""CADKernel Python API -- Basic Usage Example.

Creates primitives, measures mass properties, and exports to STL.

Prerequisites:
    cd crates/python && maturin develop --release
"""

try:
    import cadkernel as ck

    # Create a model to hold geometry
    model = ck.Model()

    # Create a 20x30x40 box at the origin
    box_solid = ck.create_box(model, [0.0, 0.0, 0.0], 20.0, 30.0, 40.0)
    print(f"Box: {box_solid}")
    print(f"Model: {model}")

    # Create a cylinder offset to the right
    cyl_solid = ck.create_cylinder(model, [50.0, 0.0, 0.0], 10.0, 50.0)
    print(f"Cylinder: {cyl_solid}")

    # Tessellate the box and measure it
    box_mesh = ck.tessellate(model, box_solid)
    props = ck.mass_properties(box_mesh)
    print(f"\nBox mass properties:")
    print(f"  Volume: {props.volume:.2f}")
    print(f"  Surface area: {props.surface_area:.2f}")
    print(f"  Centroid: {props.centroid}")

    # Export to STL
    ck.export_stl("/tmp/basic_box.stl", box_mesh)
    print("\nExported box to /tmp/basic_box.stl")

    # Export cylinder to OBJ
    cyl_mesh = ck.tessellate(model, cyl_solid)
    ck.export_obj("/tmp/basic_cylinder.obj", cyl_mesh)
    print("Exported cylinder to /tmp/basic_cylinder.obj")

except ImportError:
    print("cadkernel not installed. Build with: cd crates/python && maturin develop")
