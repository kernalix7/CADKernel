"""CADKernel Python API -- Parametric Modeling Example.

Demonstrates creating parts with variable dimensions and boolean operations.

Prerequisites:
    cd crates/python && maturin develop --release
"""

try:
    import cadkernel as ck

    def make_plate_with_holes(width, height, thickness, hole_radius, nx, ny):
        """Create a rectangular plate with a grid of through-holes."""
        # Base plate
        model = ck.Model()
        plate = ck.create_box(model, [0.0, 0.0, 0.0], width, height, thickness)

        # Punch holes in a grid pattern
        spacing_x = width / (nx + 1)
        spacing_y = height / (ny + 1)

        for ix in range(1, nx + 1):
            for iy in range(1, ny + 1):
                cx = spacing_x * ix
                cy = spacing_y * iy
                hole_model = ck.Model()
                hole = ck.create_cylinder(
                    hole_model,
                    [cx, cy, -0.5],
                    hole_radius,
                    thickness + 1.0,
                )
                result = ck.boolean_subtract(model, plate, hole_model, hole)
                # Continue with the result model for subsequent holes
                model = result
                plate = result.get_first_solid()

        return model, plate

    # Small plate: 50x30x5 with 2x1 grid of 3mm holes
    model, solid = make_plate_with_holes(50.0, 30.0, 5.0, 3.0, 2, 1)
    mesh = ck.tessellate(model, solid)
    props = ck.mass_properties(mesh)
    print(f"Plate with holes:")
    print(f"  Volume: {props.volume:.2f}")
    print(f"  Surface area: {props.surface_area:.2f}")

    ck.export_stl("/tmp/plate_with_holes.stl", mesh)
    print("Exported to /tmp/plate_with_holes.stl")

    # Flanged cylinder: a cylinder with a wider base
    base_model = ck.Model()
    flange = ck.create_cylinder(base_model, [0.0, 0.0, 0.0], 15.0, 3.0)

    shaft_model = ck.Model()
    shaft = ck.create_cylinder(shaft_model, [0.0, 0.0, 3.0], 8.0, 20.0)

    flanged = ck.boolean_union(base_model, flange, shaft_model, shaft)
    flanged_solid = flanged.get_first_solid()
    flanged_mesh = ck.tessellate(flanged, flanged_solid)
    ck.export_stl("/tmp/flanged_cylinder.stl", flanged_mesh)
    print("\nExported flanged cylinder to /tmp/flanged_cylinder.stl")

except ImportError:
    print("cadkernel not installed. Build with: cd crates/python && maturin develop")
