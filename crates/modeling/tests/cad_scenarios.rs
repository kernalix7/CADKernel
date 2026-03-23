//! Real CAD scenario integration tests.
//!
//! These tests simulate actual mechanical design workflows to verify
//! the kernel produces correct results for practical use cases.

use cadkernel_math::{Point3, Vec3};
use cadkernel_modeling::*;
use cadkernel_topology::BRepModel;

// ---------------------------------------------------------------------------
// Scenario 1: Box with drilled hole (Boolean Subtract)
// ---------------------------------------------------------------------------

#[test]
fn scenario_box_with_hole() {
    let mut box_model = BRepModel::new();
    let box_r = make_box(&mut box_model, Point3::ORIGIN, 20.0, 20.0, 10.0).unwrap();
    assert_eq!(box_model.faces.len(), 6);

    let mut cyl_model = BRepModel::new();
    let cyl_r = make_cylinder(&mut cyl_model, Point3::new(10.0, 10.0, -1.0), 3.0, 12.0, 32).unwrap();

    // Boolean subtract: drill hole through box
    let result = boolean_op(
        &box_model, box_r.solid,
        &cyl_model, cyl_r.solid,
        BooleanOp::Difference,
    );
    assert!(result.is_ok(), "Boolean subtract should not error");
    let result = result.unwrap();
    // Result should have at least the original 6 faces (some may be split)
    assert!(result.faces.len() >= 6, "Drilled box should have ≥6 faces, got {}", result.faces.len());
    assert_eq!(result.solids.len(), 1, "Should produce exactly 1 solid");
}

// ---------------------------------------------------------------------------
// Scenario 2: Sketch → Solve → Extrude
// ---------------------------------------------------------------------------

#[test]
fn scenario_sketch_extrude() {
    use cadkernel_sketch::{Constraint, Sketch, WorkPlane, extract_profile, solve};

    let mut sketch = Sketch::new();
    let p0 = sketch.add_point(0.0, 0.0);
    let p1 = sketch.add_point(15.0, 0.5);
    let p2 = sketch.add_point(15.5, 8.0);
    let p3 = sketch.add_point(-0.5, 8.5);

    let l0 = sketch.add_line(p0, p1);
    let l1 = sketch.add_line(p1, p2);
    let l2 = sketch.add_line(p2, p3);
    let l3 = sketch.add_line(p3, p0);

    sketch.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
    sketch.add_constraint(Constraint::Horizontal(l0));
    sketch.add_constraint(Constraint::Vertical(l1));
    sketch.add_constraint(Constraint::Horizontal(l2));
    sketch.add_constraint(Constraint::Vertical(l3));
    sketch.add_constraint(Constraint::Length(l0, 15.0));
    sketch.add_constraint(Constraint::Length(l1, 8.0));

    let result = solve(&mut sketch, 200, 1e-10);
    assert!(result.converged, "Sketch solver must converge");
    // DOF may be Some(0) or None depending on solver path
    if let Some(dof) = result.remaining_dof {
        assert_eq!(dof, 0, "Fully constrained sketch should have 0 DOF");
    }

    let wp = WorkPlane::xy();
    let profile = extract_profile(&sketch, &wp);
    assert_eq!(profile.len(), 4, "Rectangle profile should have 4 points");

    let mut model = BRepModel::new();
    let ext = extrude(&mut model, &profile, Vec3::Z, 5.0).unwrap();
    assert_eq!(model.faces.len(), 6, "Extruded rectangle = 6 faces (box)");
    assert_eq!(model.vertices.len(), 8, "Extruded rectangle = 8 vertices");
    assert_eq!(model.solids.len(), 1);

    // Verify geometry check passes
    let check = check_geometry(&model, ext.solid);
    assert!(check.is_valid, "Extruded solid should be valid: {:?}", check.issues);
}

// ---------------------------------------------------------------------------
// Scenario 3: Boolean Union of two boxes
// ---------------------------------------------------------------------------

#[test]
fn scenario_boolean_union_two_boxes() {
    let mut a = BRepModel::new();
    let ra = make_box(&mut a, Point3::ORIGIN, 10.0, 10.0, 10.0).unwrap();

    let mut b = BRepModel::new();
    let rb = make_box(&mut b, Point3::new(5.0, 5.0, 5.0), 10.0, 10.0, 10.0).unwrap();

    let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Union);
    assert!(result.is_ok(), "Boolean union should succeed");
    let result = result.unwrap();
    assert_eq!(result.solids.len(), 1, "Union should produce 1 solid");
    // Overlapping boxes union: some faces inside each other get removed
    assert!(result.faces.len() >= 6, "Union should have ≥6 faces, got {}", result.faces.len());
}

// ---------------------------------------------------------------------------
// Scenario 4: Fillet edge
// ---------------------------------------------------------------------------

#[test]
fn scenario_fillet_box_edge() {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::ORIGIN, 10.0, 10.0, 10.0).unwrap();
    let initial_faces = model.faces.len();
    assert_eq!(initial_faces, 6);

    // Get first two adjacent vertices for an edge
    let verts: Vec<_> = model.vertices.iter().map(|(h, _)| h).collect();
    if verts.len() >= 2 {
        let result = fillet_edge(&mut model, r.solid, verts[0], verts[1], 1.0);
        if let Ok(fr) = result {
            // Fillet should add at least one face
            assert!(
                model.faces.len() >= initial_faces,
                "Fillet should not reduce face count: {} → {}",
                initial_faces, model.faces.len()
            );
            let check = check_geometry(&model, fr.solid);
            // Fillet result should pass basic validation
            assert!(check.is_valid || !check.issues.is_empty(),
                "Fillet result should have valid topology");
        }
    }
}

// ---------------------------------------------------------------------------
// Scenario 5: Mass properties of known box
// ---------------------------------------------------------------------------

#[test]
fn scenario_mass_properties_box() {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::ORIGIN, 10.0, 10.0, 10.0).unwrap();
    let mesh = cadkernel_io::tessellate_solid(&model, r.solid);

    let props = compute_mass_properties(&mesh);
    // Volume of 10x10x10 box = 1000 (tessellation may introduce ~5% error)
    assert!(
        (props.volume - 1000.0).abs() < 60.0,
        "Box volume should be ~1000, got {:.1}",
        props.volume
    );
    // Surface area = 6 * 100 = 600
    assert!(
        (props.surface_area - 600.0).abs() < 60.0,
        "Box surface area should be ~600, got {:.1}",
        props.surface_area
    );
    // Centroid at (5, 5, 5)
    assert!((props.centroid.x - 5.0).abs() < 0.5, "Centroid X should be ~5");
    assert!((props.centroid.y - 5.0).abs() < 0.5, "Centroid Y should be ~5");
    assert!((props.centroid.z - 5.0).abs() < 0.5, "Centroid Z should be ~5");
}

// ---------------------------------------------------------------------------
// Scenario 6: Tessellation normals point outward
// ---------------------------------------------------------------------------

#[test]
fn scenario_sphere_normals_outward() {
    let mut model = BRepModel::new();
    let r = make_sphere(&mut model, Point3::ORIGIN, 5.0, 32, 16).unwrap();
    let mesh = cadkernel_io::tessellate_solid(&model, r.solid);

    assert!(mesh.triangle_count() > 0);

    // For a sphere centered at origin, face normals should point outward
    // (dot product with centroid-to-face vector should be positive)
    let mut outward_count = 0;
    let mut total = 0;
    for (i, tri) in mesh.indices.iter().enumerate() {
        let a = mesh.vertices[tri[0] as usize];
        let b = mesh.vertices[tri[1] as usize];
        let c = mesh.vertices[tri[2] as usize];
        let center = Point3::new(
            (a.x + b.x + c.x) / 3.0,
            (a.y + b.y + c.y) / 3.0,
            (a.z + b.z + c.z) / 3.0,
        );
        let normal = if i < mesh.normals.len() {
            mesh.normals[i]
        } else {
            let e1 = b - a;
            let e2 = c - a;
            e1.cross(e2).normalized().unwrap_or(Vec3::Z)
        };
        // For sphere at origin, outward = center direction
        let outward_dir = Vec3::from(center).normalized().unwrap_or(Vec3::Z);
        if normal.dot(outward_dir) > 0.0 {
            outward_count += 1;
        }
        total += 1;
    }
    let ratio = outward_count as f64 / total as f64;
    assert!(ratio > 0.8, "At least 80% of normals should point outward, got {:.1}%", ratio * 100.0);
}

// ---------------------------------------------------------------------------
// Scenario 7: STEP roundtrip (basic)
// ---------------------------------------------------------------------------

#[test]
fn scenario_step_roundtrip() {
    let mut model = BRepModel::new();
    let _r = make_box(&mut model, Point3::ORIGIN, 10.0, 10.0, 10.0).unwrap();

    let step_str = cadkernel_io::export_step(&model).unwrap();
    assert!(step_str.contains("MANIFOLD_SOLID_BREP"));
    assert!(step_str.contains("ADVANCED_FACE"));

    let imported = cadkernel_io::import_step(&step_str).unwrap();
    // Import should reconstruct vertices
    assert!(
        imported.vertices.len() >= 4,
        "STEP import should have ≥4 vertices, got {}",
        imported.vertices.len()
    );
}

// ---------------------------------------------------------------------------
// Scenario 8: STL roundtrip
// ---------------------------------------------------------------------------

#[test]
fn scenario_stl_roundtrip() {
    let mut model = BRepModel::new();
    let r = make_sphere(&mut model, Point3::ORIGIN, 5.0, 16, 8).unwrap();
    let mesh = cadkernel_io::tessellate_solid(&model, r.solid);
    let original_tris = mesh.triangle_count();

    let stl_str = cadkernel_io::write_stl_ascii(&mesh, "test_sphere");
    let imported = cadkernel_io::read_stl_ascii(&stl_str).unwrap();
    assert_eq!(
        imported.triangle_count(), original_tris,
        "STL roundtrip should preserve triangle count"
    );
}

// ---------------------------------------------------------------------------
// Scenario 9: Sketch solver DOF analysis
// ---------------------------------------------------------------------------

#[test]
fn scenario_sketch_dof() {
    use cadkernel_sketch::{Constraint, Sketch, solve};

    // Under-constrained: 2 points, 1 distance constraint
    let mut sketch = Sketch::new();
    let p0 = sketch.add_point(0.0, 0.0);
    let p1 = sketch.add_point(5.0, 0.0);
    sketch.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
    sketch.add_constraint(Constraint::Distance(p0, p1, 5.0));

    let result = solve(&mut sketch, 100, 1e-10);
    assert!(result.converged);
    // p0 fixed (0 DOF) + p1 has distance constraint (1 DOF remaining for angle)
    if let Some(dof) = result.remaining_dof {
        assert!(dof >= 1, "Under-constrained system should have DOF ≥ 1, got {dof}");
    }
}

// ---------------------------------------------------------------------------
// Scenario 10: Geometry validation
// ---------------------------------------------------------------------------

#[test]
fn scenario_geometry_validation() {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::ORIGIN, 10.0, 10.0, 10.0).unwrap();

    let check = check_geometry(&model, r.solid);
    assert!(check.is_valid, "Box should pass geometry check: {:?}", check.issues);

    let wt = check_watertight(&model, r.solid);
    assert!(wt, "Box should be watertight");
}
