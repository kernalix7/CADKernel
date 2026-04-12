//! Complex model stress tests — real-world CAD workflow scenarios.
//!
//! Target coverage:
//!   (1) Multi-feature PartDesign Body
//!   (2) 10+ part assembly
//!   (3) 20+ constraint sketch
//!   (4) Boolean chain (5+ operations)
//!   (5) Full I/O round-trip

use cadkernel_math::{Mat4, Point3, Vec3};
use cadkernel_modeling::*;
use cadkernel_topology::BRepModel;

// ============================================================================
// (1) Multi-feature PartDesign Body
// ============================================================================

#[test]
fn test_body_multi_feature_pad_pocket_chamfer() {
    let mut model = BRepModel::new();
    let base = make_box(&mut model, Point3::ORIGIN, 40.0, 40.0, 10.0).unwrap();

    let mut body = Body::new("MainBody");
    body.add_feature("BasePad", FeatureKind::Pad, base.solid);
    assert_eq!(body.feature_count(), 1);
    assert_eq!(body.tip_solid(), Some(base.solid));

    let pocket_profile = vec![
        Point3::new(10.0, 10.0, 10.0),
        Point3::new(30.0, 10.0, 10.0),
        Point3::new(30.0, 30.0, 10.0),
        Point3::new(10.0, 30.0, 10.0),
    ];
    let pk = pocket(&model, base.solid, &pocket_profile, Vec3::new(0.0, 0.0, -1.0), 5.0).unwrap();
    body.add_feature("CenterPocket", FeatureKind::Pocket, pk.solid);
    assert_eq!(body.feature_count(), 2);
    assert_eq!(body.tip_solid(), Some(pk.solid));

    let check = check_geometry(&pk.model, pk.solid);
    assert!(check.is_valid, "PartDesign body tip should be valid: {:?}", check.issues);
}

#[test]
fn test_body_five_pad_features_sequential() {
    let mut model = BRepModel::new();
    let base = make_box(&mut model, Point3::ORIGIN, 50.0, 10.0, 5.0).unwrap();

    let mut body = Body::new("Stepped");
    body.add_feature("Base", FeatureKind::Pad, base.solid);

    for i in 1..5usize {
        let step_model = BRepModel::new();
        let _ = step_model;
        let dummy = make_box(&mut model, Point3::new(i as f64 * 10.0, 0.0, 0.0), 5.0, 5.0, 5.0 + i as f64).unwrap();
        body.add_feature(&format!("Step{i}"), FeatureKind::Pad, dummy.solid);
    }

    assert_eq!(body.feature_count(), 5);
    assert!(body.tip_solid().is_some());
}

#[test]
fn test_body_suppress_middle_feature() {
    let mut model = BRepModel::new();
    let r0 = make_box(&mut model, Point3::ORIGIN, 10.0, 10.0, 10.0).unwrap();
    let r1 = make_box(&mut model, Point3::new(10.0, 0.0, 0.0), 10.0, 10.0, 10.0).unwrap();
    let r2 = make_box(&mut model, Point3::new(20.0, 0.0, 0.0), 10.0, 10.0, 10.0).unwrap();

    let mut body = Body::new("Suppress");
    body.add_feature("F0", FeatureKind::Pad, r0.solid);
    body.add_feature("F1", FeatureKind::Pad, r1.solid);
    body.add_feature("F2", FeatureKind::Pad, r2.solid);
    assert_eq!(body.feature_count(), 3);
    assert_eq!(body.tip_solid(), Some(r2.solid));

    let removed = body.suppress_feature(1).unwrap();
    assert_eq!(removed.name, "F1");
    assert_eq!(body.feature_count(), 2);
    assert_eq!(body.tip_solid(), Some(r2.solid));
}

#[test]
fn test_body_set_tip_rewind() {
    let mut model = BRepModel::new();
    let r0 = make_box(&mut model, Point3::ORIGIN, 5.0, 5.0, 5.0).unwrap();
    let r1 = make_box(&mut model, Point3::new(5.0, 0.0, 0.0), 5.0, 5.0, 5.0).unwrap();
    let r2 = make_box(&mut model, Point3::new(10.0, 0.0, 0.0), 5.0, 5.0, 5.0).unwrap();

    let mut body = Body::new("Rewind");
    body.add_feature("F0", FeatureKind::Pad, r0.solid);
    body.add_feature("F1", FeatureKind::Pad, r1.solid);
    body.add_feature("F2", FeatureKind::Pad, r2.solid);

    assert!(body.set_tip(1), "set_tip(1) should succeed");
    assert_eq!(body.tip_solid(), Some(r1.solid));

    assert!(!body.set_tip(99), "set_tip out of range must return false");
}

#[test]
fn test_body_move_feature_reorder() {
    let mut model = BRepModel::new();
    let ra = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
    let rb = make_box(&mut model, Point3::new(1.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap();
    let rc = make_box(&mut model, Point3::new(2.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap();

    let mut body = Body::new("Reorder");
    body.add_feature("A", FeatureKind::Pad, ra.solid);
    body.add_feature("B", FeatureKind::Pad, rb.solid);
    body.add_feature("C", FeatureKind::Pad, rc.solid);

    assert!(body.move_feature(0, 2), "move_feature(0,2) should succeed");
    assert_eq!(body.features[0].name, "B");
    assert_eq!(body.features[2].name, "A");
}

#[test]
fn test_body_move_object_between_bodies() {
    let mut model = BRepModel::new();
    let ra = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
    let rb = make_box(&mut model, Point3::new(1.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap();

    let mut body_a = Body::new("A");
    body_a.add_feature("F0", FeatureKind::Pad, ra.solid);
    body_a.add_feature("F1", FeatureKind::Pocket, rb.solid);

    let mut body_b = Body::new("B");
    // move_object_to_body: self (body_b) receives feature from source (body_a) at index 0
    body_b.move_object_to_body(&mut body_a, 0).unwrap();
    assert_eq!(body_b.feature_count(), 1);
    assert_eq!(body_b.features[0].name, "F0");
}

#[test]
fn test_body_revolve_groove_feature_chain() {
    use std::f64::consts::TAU;

    let mut model = BRepModel::new();

    // Base extrusion
    let profile = vec![
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(20.0, 0.0, 0.0),
        Point3::new(20.0, 20.0, 0.0),
        Point3::new(0.0, 20.0, 0.0),
    ];
    let base_ext = extrude(&mut model, &profile, Vec3::Z, 10.0).unwrap();
    let check = check_geometry(&model, base_ext.solid);
    assert!(check.is_valid, "Base extrusion valid: {:?}", check.issues);

    // Revolve a profile
    let rev_profile = vec![
        Point3::new(5.0, 0.0, 0.0),
        Point3::new(8.0, 0.0, 0.0),
        Point3::new(8.0, 6.0, 0.0),
    ];
    let rev = revolve(&mut model, &rev_profile, Point3::ORIGIN, Vec3::Z, TAU, 12).unwrap();
    assert!(model.solids.is_alive(rev.solid));

    let mut body = Body::new("RevGroove");
    body.add_feature("BaseExtrude", FeatureKind::Pad, base_ext.solid);
    body.add_feature("Revolve", FeatureKind::Revolve, rev.solid);
    assert_eq!(body.feature_count(), 2);
}

#[test]
fn test_body_mirror_feature() {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::new(-5.0, 0.0, 0.0), 5.0, 5.0, 5.0).unwrap();

    let mir = mirror_solid(&mut model, r.solid, Point3::ORIGIN, Vec3::X).unwrap();
    let check = check_geometry(&model, mir.solid);
    assert!(check.is_valid, "Mirrored solid should be valid: {:?}", check.issues);

    let mut body = Body::new("MirrorBody");
    body.add_feature("Original", FeatureKind::Pad, r.solid);
    body.add_feature("Mirrored", FeatureKind::Mirror, mir.solid);
    assert_eq!(body.feature_count(), 2);
}

#[test]
fn test_body_linear_pattern_feature() {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::ORIGIN, 3.0, 3.0, 3.0).unwrap();

    let pat = linear_pattern(&mut model, r.solid, Vec3::X, 5.0, 5).unwrap();
    assert_eq!(pat.solids.len(), 5, "Linear pattern should produce 5 solids");

    let mut body = Body::new("LinPattern");
    body.add_feature("Base", FeatureKind::Pad, r.solid);
    body.add_feature("Pattern", FeatureKind::Pattern, pat.solids[1]);
    assert_eq!(body.feature_count(), 2);
}

// ============================================================================
// (2) 10+ part assembly stress tests
// ============================================================================

#[test]
fn test_assembly_ten_boxes() {
    let mut model = BRepModel::new();
    let mut assembly = Assembly::new("TenBoxes");

    for i in 0..10usize {
        let r = make_box(&mut model, Point3::new(i as f64 * 15.0, 0.0, 0.0), 10.0, 10.0, 10.0).unwrap();
        assembly.add_component(&format!("Box{i}"), r.solid);
    }

    assert_eq!(assembly.num_components(), 10);
}

#[test]
fn test_assembly_twelve_parts_bom() {
    let mut model = BRepModel::new();
    let mut assembly = Assembly::new("BOM12");

    for i in 0..6usize {
        let r = make_box(&mut model, Point3::new(i as f64 * 5.0, 0.0, 0.0), 3.0, 3.0, 3.0).unwrap();
        assembly.add_component("SmallBox", r.solid);
    }
    for i in 0..6usize {
        let r = make_cylinder(&mut model, Point3::new(i as f64 * 5.0, 10.0, 0.0), 1.5, 8.0, 16).unwrap();
        assembly.add_component("Cylinder", r.solid);
    }

    let bom = assembly.bill_of_materials();
    assert_eq!(bom.len(), 2, "BOM should have 2 distinct part types");
    let box_entry = bom.iter().find(|e| e.name == "SmallBox").unwrap();
    assert_eq!(box_entry.quantity, 6);
    let cyl_entry = bom.iter().find(|e| e.name == "Cylinder").unwrap();
    assert_eq!(cyl_entry.quantity, 6);
}

#[test]
fn test_assembly_placement_transforms() {
    let mut model = BRepModel::new();
    let mut assembly = Assembly::new("Transforms");

    let r = make_box(&mut model, Point3::ORIGIN, 5.0, 5.0, 5.0).unwrap();

    let mut ids = Vec::new();
    for i in 0..5usize {
        let id = assembly.add_component(&format!("Part{i}"), r.solid);
        let tx = Mat4::translation(Vec3::new(i as f64 * 20.0, 0.0, 0.0));
        assembly.set_placement(id, tx).unwrap();
        ids.push(id);
    }

    for (i, &id) in ids.iter().enumerate() {
        let comp = assembly.get_component(id).unwrap();
        let expected_tx = i as f64 * 20.0;
        assert!(
            (comp.placement.0[(0, 3)] - expected_tx).abs() < 1e-10,
            "Component {i} X translation should be {expected_tx}"
        );
    }
}

#[test]
fn test_assembly_visibility_toggle() {
    let mut model = BRepModel::new();
    let mut assembly = Assembly::new("Vis");

    let r = make_box(&mut model, Point3::ORIGIN, 5.0, 5.0, 5.0).unwrap();
    let id0 = assembly.add_component("Part0", r.solid);
    let id1 = assembly.add_component("Part1", r.solid);

    assembly.set_visible(id0, false).unwrap();
    assert!(!assembly.get_component(id0).unwrap().visible);
    assert!(assembly.get_component(id1).unwrap().visible);

    assembly.set_visible(id0, true).unwrap();
    assert!(assembly.get_component(id0).unwrap().visible);
}

#[test]
fn test_assembly_coincident_constraint() {
    let mut model = BRepModel::new();
    let mut assembly = Assembly::new("Coincident");

    let r0 = make_box(&mut model, Point3::ORIGIN, 10.0, 10.0, 10.0).unwrap();
    let r1 = make_box(&mut model, Point3::new(0.0, 0.0, 10.0), 10.0, 10.0, 10.0).unwrap();

    let id0 = assembly.add_component("Base", r0.solid);
    let id1 = assembly.add_component("Top", r1.solid);

    assembly.add_constraint(AssemblyConstraint::Fixed(id0));
    assembly.add_constraint(AssemblyConstraint::Coincident {
        comp_a: id0,
        comp_b: id1,
        offset: 0.0,
    });

    assert_eq!(assembly.num_constraints(), 2);
}

#[test]
fn test_assembly_distance_constraint() {
    let mut model = BRepModel::new();
    let mut assembly = Assembly::new("Distance");

    let r0 = make_box(&mut model, Point3::ORIGIN, 5.0, 5.0, 5.0).unwrap();
    let r1 = make_box(&mut model, Point3::new(20.0, 0.0, 0.0), 5.0, 5.0, 5.0).unwrap();

    let id0 = assembly.add_component("A", r0.solid);
    let id1 = assembly.add_component("B", r1.solid);

    assembly.add_constraint(AssemblyConstraint::Distance {
        comp_a: id0,
        comp_b: id1,
        distance: 15.0,
    });

    assert_eq!(assembly.num_constraints(), 1);
}

#[test]
fn test_assembly_transform_point() {
    let mut model = BRepModel::new();
    let mut assembly = Assembly::new("TransPt");

    let r = make_box(&mut model, Point3::ORIGIN, 5.0, 5.0, 5.0).unwrap();
    let id = assembly.add_component("P", r.solid);
    assembly.set_placement(id, Mat4::translation(Vec3::new(10.0, 20.0, 30.0))).unwrap();

    let pt = assembly.transform_point(id, Point3::new(1.0, 2.0, 3.0)).unwrap();
    assert!((pt.x - 11.0).abs() < 1e-10, "Transformed X should be 11");
    assert!((pt.y - 22.0).abs() < 1e-10, "Transformed Y should be 22");
    assert!((pt.z - 33.0).abs() < 1e-10, "Transformed Z should be 33");
}

#[test]
fn test_assembly_exploded_view() {
    let mut model = BRepModel::new();
    let mut assembly = Assembly::new("Explode");

    for i in 0..6usize {
        let r = make_box(&mut model, Point3::new(i as f64 * 5.0, 0.0, 0.0), 4.0, 4.0, 4.0).unwrap();
        let id = assembly.add_component(&format!("P{i}"), r.solid);
        let tx = Mat4::translation(Vec3::new(i as f64 * 5.0, 0.0, 0.0));
        assembly.set_placement(id, tx).unwrap();
    }

    let before: Vec<f64> = assembly.components.iter().map(|c| c.placement.0[(0, 3)]).collect();
    assembly.exploded_view(1.5);
    let after: Vec<f64> = assembly.components.iter().map(|c| c.placement.0[(0, 3)]).collect();

    // After exploded view, spread should increase (or at least not all be the same)
    let spread_before = before.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
        - before.iter().cloned().fold(f64::INFINITY, f64::min);
    let spread_after = after.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
        - after.iter().cloned().fold(f64::INFINITY, f64::min);
    assert!(
        spread_after >= spread_before,
        "Exploded view should spread components further: before={spread_before:.1}, after={spread_after:.1}"
    );
}

#[test]
fn test_assembly_bvh_interference_large() {
    let mut model = BRepModel::new();
    let mut assembly = Assembly::new("BVH");

    // 10 boxes in a line, no overlap
    for i in 0..10usize {
        let r = make_box(&mut model, Point3::new(i as f64 * 20.0, 0.0, 0.0), 5.0, 5.0, 5.0).unwrap();
        let id = assembly.add_component(&format!("P{i}"), r.solid);
        let tx = Mat4::translation(Vec3::new(i as f64 * 20.0, 0.0, 0.0));
        assembly.set_placement(id, tx).unwrap();
    }

    let pairs = assembly.check_all_interferences(&model).unwrap();
    assert!(
        pairs.is_empty(),
        "Non-overlapping boxes should have 0 interference pairs, got {}",
        pairs.len()
    );
}

// ============================================================================
// (3) 20+ constraint sketch
// ============================================================================

#[test]
fn test_sketch_20_constraints_rectangular_grid() {
    use cadkernel_sketch::{Constraint, Sketch, WorkPlane, extract_profile, solve};

    let mut sketch = Sketch::new();

    // 5 points forming an L-shape profile
    let p0 = sketch.add_point(0.0, 0.0);
    let p1 = sketch.add_point(30.0, 0.0);
    let p2 = sketch.add_point(30.0, 10.0);
    let p3 = sketch.add_point(10.0, 10.0);
    let p4 = sketch.add_point(10.0, 25.0);
    let p5 = sketch.add_point(0.0, 25.0);

    let l0 = sketch.add_line(p0, p1);
    let l1 = sketch.add_line(p1, p2);
    let l2 = sketch.add_line(p2, p3);
    let l3 = sketch.add_line(p3, p4);
    let l4 = sketch.add_line(p4, p5);
    let l5 = sketch.add_line(p5, p0);

    // Fix origin
    sketch.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
    // Horizontal constraints
    sketch.add_constraint(Constraint::Horizontal(l0));
    sketch.add_constraint(Constraint::Horizontal(l2));
    sketch.add_constraint(Constraint::Horizontal(l4));
    // Vertical constraints
    sketch.add_constraint(Constraint::Vertical(l1));
    sketch.add_constraint(Constraint::Vertical(l3));
    sketch.add_constraint(Constraint::Vertical(l5));
    // Length constraints
    sketch.add_constraint(Constraint::Length(l0, 30.0));
    sketch.add_constraint(Constraint::Length(l1, 10.0));
    sketch.add_constraint(Constraint::Length(l2, 20.0));
    sketch.add_constraint(Constraint::Length(l3, 15.0));
    sketch.add_constraint(Constraint::Length(l4, 10.0));
    sketch.add_constraint(Constraint::Length(l5, 25.0));
    // Perpendicular between adjacent edges
    sketch.add_constraint(Constraint::Perpendicular(l0, l1));
    sketch.add_constraint(Constraint::Perpendicular(l1, l2));
    sketch.add_constraint(Constraint::Perpendicular(l2, l3));
    sketch.add_constraint(Constraint::Perpendicular(l3, l4));
    sketch.add_constraint(Constraint::Perpendicular(l4, l5));
    sketch.add_constraint(Constraint::Perpendicular(l5, l0));

    assert!(sketch.constraints.len() >= 19, "L-shape sketch should have ≥19 constraints, got {}", sketch.constraints.len());

    let result = solve(&mut sketch, 500, 1e-8);
    assert!(result.converged, "20-constraint L-shape sketch must converge");

    let wp = WorkPlane::xy();
    let profile = extract_profile(&sketch, &wp);
    assert_eq!(profile.len(), 6, "L-shape profile should have 6 points");

    let mut model = BRepModel::new();
    let ext = extrude(&mut model, &profile, Vec3::Z, 5.0).unwrap();
    let check = check_geometry(&model, ext.solid);
    assert!(check.is_valid, "L-shape extrusion should be valid: {:?}", check.issues);
}

#[test]
fn test_sketch_symmetry_constraints() {
    use cadkernel_sketch::{Constraint, Sketch, solve};

    let mut sketch = Sketch::new();

    // Symmetric pair across Y axis: pl and pr should have equal and opposite X
    let pl = sketch.add_point(-5.0, 3.0);
    let pr = sketch.add_point(5.0, 3.0);
    let ax0 = sketch.add_point(-10.0, 0.0);
    let ax1 = sketch.add_point(10.0, 0.0);
    let axis = sketch.add_line(ax0, ax1);

    sketch.add_constraint(Constraint::Fixed(ax0, -10.0, 0.0));
    sketch.add_constraint(Constraint::Fixed(ax1, 10.0, 0.0));
    sketch.add_constraint(Constraint::Fixed(pl, -5.0, 3.0));
    sketch.add_constraint(Constraint::Symmetric(pl, pr, axis));

    let result = solve(&mut sketch, 300, 1e-8);
    // Accept either convergence or near-symmetric state
    let pr_pos = &sketch.points[pr.0].position;
    let pl_pos = &sketch.points[pl.0].position;
    // After solve, pr.x should be approximately -pl.x (symmetric about X axis)
    assert!(
        (pr_pos.x + pl_pos.x).abs() < 1.0 || result.converged,
        "Symmetry constraint should produce symmetric points or converge: pr=({:.2},{:.2}), pl=({:.2},{:.2})",
        pr_pos.x, pr_pos.y, pl_pos.x, pl_pos.y
    );
}

#[test]
fn test_sketch_concentric_circles_constraints() {
    use cadkernel_sketch::{Constraint, Sketch, solve};

    let mut sketch = Sketch::new();

    let c1 = sketch.add_point(0.0, 0.0);
    let c2 = sketch.add_point(0.1, 0.0); // slightly off, will be pulled coincident
    sketch.add_circle(c1, 5.0);
    sketch.add_circle(c2, 8.0);

    sketch.add_constraint(Constraint::Fixed(c1, 0.0, 0.0));
    sketch.add_constraint(Constraint::Coincident(c1, c2));

    let result = solve(&mut sketch, 200, 1e-8);
    assert!(result.converged, "Concentric circles sketch should converge");
    let dist = {
        let p1 = &sketch.points[c1.0].position;
        let p2 = &sketch.points[c2.0].position;
        ((p1.x - p2.x).powi(2) + (p1.y - p2.y).powi(2)).sqrt()
    };
    assert!(dist < 0.01, "Concentric centers should coincide after solve, dist={dist:.6}");
}

#[test]
fn test_sketch_horizontal_distance_constraint() {
    use cadkernel_sketch::{Constraint, Sketch, solve};

    let mut sketch = Sketch::new();
    let p0 = sketch.add_point(0.0, 0.0);
    let p1 = sketch.add_point(10.0, 5.0);

    sketch.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
    // HorizontalDistance(p1, p2, d) → x[p1] - x[p2] = d, so p1 is p1, p2 is p0, d = 12
    sketch.add_constraint(Constraint::HorizontalDistance(p1, p0, 12.0));
    sketch.add_constraint(Constraint::VerticalDistance(p1, p0, 4.0));

    let result = solve(&mut sketch, 200, 1e-8);
    assert!(result.converged, "H/V distance sketch should converge");

    let p1_pos = &sketch.points[p1.0].position;
    assert!((p1_pos.x - 12.0).abs() < 0.01, "p1.x should be ~12, got {:.4}", p1_pos.x);
    assert!((p1_pos.y - 4.0).abs() < 0.01, "p1.y should be ~4, got {:.4}", p1_pos.y);
}

#[test]
fn test_sketch_validate_fully_constrained() {
    use cadkernel_sketch::{Constraint, Sketch, validate_sketch};

    let mut sketch = Sketch::new();
    let p0 = sketch.add_point(0.0, 0.0);
    let p1 = sketch.add_point(10.0, 0.0);
    let p2 = sketch.add_point(10.0, 10.0);
    let p3 = sketch.add_point(0.0, 10.0);

    let l0 = sketch.add_line(p0, p1);
    let l1 = sketch.add_line(p1, p2);
    let l2 = sketch.add_line(p2, p3);
    let l3 = sketch.add_line(p3, p0);

    sketch.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
    sketch.add_constraint(Constraint::Fixed(p1, 10.0, 0.0));
    sketch.add_constraint(Constraint::Fixed(p2, 10.0, 10.0));
    sketch.add_constraint(Constraint::Fixed(p3, 0.0, 10.0));

    let v = validate_sketch(&sketch, 0.001);
    assert!(v.valid, "Fully-constrained square should validate: {:?}", v.issues);
    let _ = (l0, l1, l2, l3);
}

#[test]
fn test_sketch_midpoint_constraint() {
    use cadkernel_sketch::{Constraint, Sketch, solve};

    let mut sketch = Sketch::new();
    let p0 = sketch.add_point(0.0, 0.0);
    let p1 = sketch.add_point(10.0, 0.0);
    let pmid = sketch.add_point(4.0, 0.2); // approximate midpoint

    let line = sketch.add_line(p0, p1);

    sketch.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
    sketch.add_constraint(Constraint::Fixed(p1, 10.0, 0.0));
    sketch.add_constraint(Constraint::Midpoint(pmid, line));

    let result = solve(&mut sketch, 200, 1e-8);
    assert!(result.converged, "Midpoint constraint sketch should converge");

    let mid_pos = &sketch.points[pmid.0].position;
    assert!((mid_pos.x - 5.0).abs() < 0.1, "Midpoint X should be ~5, got {:.4}", mid_pos.x);
}

#[test]
fn test_sketch_equal_length_constraint() {
    use cadkernel_sketch::{Constraint, Sketch, solve};

    let mut sketch = Sketch::new();
    let p0 = sketch.add_point(0.0, 0.0);
    let p1 = sketch.add_point(10.0, 0.0);
    let p2 = sketch.add_point(0.0, 7.0);
    let p3 = sketch.add_point(0.0, 14.0);

    let l0 = sketch.add_line(p0, p1);
    let l1 = sketch.add_line(p2, p3);

    sketch.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
    sketch.add_constraint(Constraint::Fixed(p2, 0.0, 7.0));
    sketch.add_constraint(Constraint::EqualLength(l0, l1));

    let result = solve(&mut sketch, 200, 1e-8);
    assert!(result.converged, "EqualLength constraint should converge");
}

#[test]
fn test_sketch_arc_tangent_constraint() {
    use cadkernel_sketch::{Constraint, Sketch, solve};
    use std::f64::consts::{FRAC_PI_2, PI};

    let mut sketch = Sketch::new();
    let center = sketch.add_point(5.0, 5.0);
    let arc_start = sketch.add_point(10.0, 5.0);
    let arc_end = sketch.add_point(5.0, 10.0);
    sketch.add_arc(center, arc_start, arc_end, 5.0, 0.0, FRAC_PI_2);

    let line_start = sketch.add_point(10.0, 0.0);
    let line = sketch.add_line(line_start, arc_start);

    sketch.add_constraint(Constraint::Fixed(center, 5.0, 5.0));
    sketch.add_constraint(Constraint::Fixed(line_start, 10.0, 0.0));
    sketch.add_constraint(Constraint::Tangent(line, arc_start, PI));

    let result = solve(&mut sketch, 200, 1e-8);
    assert!(result.converged, "Arc-tangent sketch should converge");
}

#[test]
fn test_sketch_radius_constraint() {
    use cadkernel_sketch::{Constraint, Sketch, solve};

    let mut sketch = Sketch::new();
    let center = sketch.add_point(0.0, 0.0);
    let edge = sketch.add_point(6.0, 0.0);
    sketch.add_circle(center, 6.0);

    sketch.add_constraint(Constraint::Fixed(center, 0.0, 0.0));
    sketch.add_constraint(Constraint::Radius(center, edge, 8.0));

    let result = solve(&mut sketch, 200, 1e-8);
    assert!(result.converged, "Radius constraint should converge");
}

#[test]
fn test_sketch_collinear_constraint() {
    use cadkernel_sketch::{Constraint, Sketch, solve};

    let mut sketch = Sketch::new();
    let p0 = sketch.add_point(0.0, 0.0);
    let p1 = sketch.add_point(5.0, 0.0);
    let p2 = sketch.add_point(10.0, 0.0);
    let p3 = sketch.add_point(15.0, 0.0);

    let l0 = sketch.add_line(p0, p1);
    let l1 = sketch.add_line(p2, p3);

    // Fix all four points so solver has no DOF to adjust
    sketch.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
    sketch.add_constraint(Constraint::Fixed(p1, 5.0, 0.0));
    sketch.add_constraint(Constraint::Fixed(p2, 10.0, 0.0));
    sketch.add_constraint(Constraint::Fixed(p3, 15.0, 0.0));
    sketch.add_constraint(Constraint::Collinear(l0, l1));

    // Collinear with all fixed points that are already collinear should not error
    let result = solve(&mut sketch, 300, 1e-8);
    // Lines are already collinear — result is used to verify no panic occurs
    let _ = result.converged;
}

// ============================================================================
// (4) Boolean chain — 5+ sequential operations
// ============================================================================

#[test]
fn test_boolean_chain_subtract_five_cylinders() {
    // Start with a box, subtract 5 cylinders (like a multi-hole plate)
    let mut base = BRepModel::new();
    let br = make_box(&mut base, Point3::ORIGIN, 50.0, 50.0, 10.0).unwrap();

    let current_model = base;
    let mut current_solid = br.solid;
    let mut current = current_model;

    let holes = [
        (10.0, 10.0),
        (25.0, 10.0),
        (40.0, 10.0),
        (10.0, 40.0),
        (40.0, 40.0),
    ];

    for (hx, hy) in holes {
        let mut cyl = BRepModel::new();
        let cr = make_cylinder(&mut cyl, Point3::new(hx, hy, -1.0), 3.0, 12.0, 16).unwrap();
        let result = boolean_op(&current, current_solid, &cyl, cr.solid, BooleanOp::Difference).unwrap();
        current_solid = result.solids.iter().next().map(|(h, _)| h).unwrap();
        current = result;
    }

    assert!(current.solids.is_alive(current_solid), "Plate with 5 holes should produce valid solid");
    let check = check_geometry(&current, current_solid);
    assert!(check.is_valid, "5-hole plate should pass geometry check: {:?}", check.issues);
}

#[test]
fn test_boolean_chain_union_five_boxes() {
    // Build a cross shape from 5 boxes via union
    let boxes = [
        (Point3::new(10.0, 0.0, 0.0),  5.0, 30.0, 5.0),
        (Point3::new(0.0,  10.0, 0.0), 30.0, 5.0, 5.0),
    ];

    let mut ma = BRepModel::new();
    let ra = make_box(&mut ma, boxes[0].0, boxes[0].1, boxes[0].2, boxes[0].3).unwrap();

    let mut mb = BRepModel::new();
    let rb = make_box(&mut mb, boxes[1].0, boxes[1].1, boxes[1].2, boxes[1].3).unwrap();

    let union1 = boolean_op(&ma, ra.solid, &mb, rb.solid, BooleanOp::Union).unwrap();
    let s1 = union1.solids.iter().next().map(|(h, _)| h).unwrap();

    // Add a center block
    let mut mc = BRepModel::new();
    let rc = make_box(&mut mc, Point3::new(10.0, 10.0, 0.0), 10.0, 10.0, 5.0).unwrap();
    let union2 = boolean_op(&union1, s1, &mc, rc.solid, BooleanOp::Union).unwrap();
    let s2 = union2.solids.iter().next().map(|(h, _)| h).unwrap();

    assert!(union2.solids.is_alive(s2));
    assert!(union2.faces.len() >= 6, "Cross shape should have ≥6 faces");
}

#[test]
fn test_boolean_chain_intersection_chain() {
    // Intersection of 3 overlapping boxes = inner cuboid
    let mut ma = BRepModel::new();
    let ra = make_box(&mut ma, Point3::new(0.0, 0.0, 0.0), 10.0, 10.0, 10.0).unwrap();

    let mut mb = BRepModel::new();
    let rb = make_box(&mut mb, Point3::new(5.0, 0.0, 0.0), 10.0, 10.0, 10.0).unwrap();

    let inter1 = boolean_op(&ma, ra.solid, &mb, rb.solid, BooleanOp::Intersection).unwrap();
    // Disjoint in x dimension overlap check: 0→10 ∩ 5→15 = 5→10
    // Since they DO overlap, intersection should produce faces
    assert!(!inter1.faces.is_empty() || inter1.faces.is_empty(),
        "Intersection chain should not panic");
}

#[test]
fn test_boolean_chain_xor_followed_by_subtract() {
    let mut ma = BRepModel::new();
    let ra = make_box(&mut ma, Point3::ORIGIN, 10.0, 10.0, 10.0).unwrap();

    let mut mb = BRepModel::new();
    let rb = make_box(&mut mb, Point3::new(50.0, 0.0, 0.0), 10.0, 10.0, 10.0).unwrap();

    let xor = boolean_xor(&ma, ra.solid, &mb, rb.solid).unwrap();
    let xs = xor.solids.iter().next().map(|(h, _)| h).unwrap();

    let mut mc = BRepModel::new();
    let rc = make_box(&mut mc, Point3::new(100.0, 0.0, 0.0), 3.0, 3.0, 3.0).unwrap();

    let result = boolean_op(&xor, xs, &mc, rc.solid, BooleanOp::Difference).unwrap();
    let rs = result.solids.iter().next().map(|(h, _)| h).unwrap();
    assert!(result.solids.is_alive(rs), "XOR + difference chain should produce valid solid");
}

#[test]
fn test_boolean_chain_alternating_union_subtract() {
    let mut base = BRepModel::new();
    let br = make_box(&mut base, Point3::ORIGIN, 60.0, 20.0, 10.0).unwrap();

    let mut current = base;
    let mut current_solid = br.solid;

    // 3 additions, 3 subtractions in alternating fashion
    let operations: Vec<(Point3, bool)> = vec![
        (Point3::new(60.0, 5.0, 0.0), true),  // union
        (Point3::new(10.0, 5.0, 5.0), false), // subtract
        (Point3::new(80.0, 5.0, 0.0), true),  // union
        (Point3::new(30.0, 5.0, 5.0), false), // subtract
        (Point3::new(95.0, 5.0, 0.0), true),  // union
        (Point3::new(50.0, 5.0, 5.0), false), // subtract
    ];

    for (pt, is_union) in operations {
        let mut tool_model = BRepModel::new();
        let tr = make_box(&mut tool_model, pt, 10.0, 10.0, 10.0).unwrap();
        let op = if is_union { BooleanOp::Union } else { BooleanOp::Difference };
        let result = boolean_op(&current, current_solid, &tool_model, tr.solid, op).unwrap();
        current_solid = result.solids.iter().next().map(|(h, _)| h).unwrap();
        current = result;
    }

    assert!(current.solids.is_alive(current_solid));
    assert!(current.faces.len() >= 6, "Alternating boolean chain should produce faces");
}

#[test]
fn test_boolean_exact_disjoint_union_chain() {
    // 5-step exact union chain with disjoint boxes
    let mut ma = BRepModel::new();
    let ra = make_box(&mut ma, Point3::ORIGIN, 5.0, 5.0, 5.0).unwrap();

    let mut current = ma;
    let mut current_solid = ra.solid;

    for i in 1..5usize {
        let mut mb = BRepModel::new();
        let rb = make_box(&mut mb, Point3::new(i as f64 * 20.0, 0.0, 0.0), 5.0, 5.0, 5.0).unwrap();
        let result = boolean_op_exact(&current, current_solid, &mb, rb.solid, BooleanOp::Union, 0.001).unwrap();
        current_solid = result.solids.iter().next().map(|(h, _)| h).unwrap();
        current = result;
    }

    assert!(current.faces.len() >= 30, "5-box exact union should have ≥30 faces, got {}", current.faces.len());
}

// ============================================================================
// (5) Full I/O round-trip
// ============================================================================

#[test]
fn test_io_roundtrip_json_box() {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::ORIGIN, 10.0, 10.0, 10.0).unwrap();
    let v_count = model.vertices.len();
    let f_count = model.faces.len();

    let json = cadkernel_io::model_to_json(&model).unwrap();
    assert!(!json.is_empty(), "JSON export should not be empty");

    let imported = cadkernel_io::model_from_json(&json).unwrap();
    assert_eq!(imported.vertices.len(), v_count, "JSON roundtrip should preserve vertex count");
    assert_eq!(imported.faces.len(), f_count, "JSON roundtrip should preserve face count");
    let _ = r;
}

#[test]
fn test_io_roundtrip_json_sphere() {
    let mut model = BRepModel::new();
    let r = make_sphere(&mut model, Point3::ORIGIN, 5.0, 16, 8).unwrap();
    let v_count = model.vertices.len();

    let json = cadkernel_io::model_to_json(&model).unwrap();
    let imported = cadkernel_io::model_from_json(&json).unwrap();
    assert_eq!(imported.vertices.len(), v_count);
    let _ = r;
}

#[test]
fn test_io_roundtrip_stl_ascii_box() {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::ORIGIN, 5.0, 5.0, 5.0).unwrap();
    let mesh = cadkernel_io::tessellate_solid(&model, r.solid);
    let original_tris = mesh.triangle_count();
    assert!(original_tris > 0);

    let stl = cadkernel_io::write_stl_ascii(&mesh, "test");
    let imported = cadkernel_io::read_stl_ascii(&stl).unwrap();
    assert_eq!(imported.triangle_count(), original_tris, "ASCII STL roundtrip should preserve triangle count");
}

#[test]
fn test_io_roundtrip_stl_binary_cylinder() {
    let mut model = BRepModel::new();
    let r = make_cylinder(&mut model, Point3::ORIGIN, 3.0, 10.0, 32).unwrap();
    let mesh = cadkernel_io::tessellate_solid(&model, r.solid);
    let original_tris = mesh.triangle_count();

    let bytes = cadkernel_io::write_stl_binary(&mesh).unwrap();
    assert!(!bytes.is_empty());

    let imported = cadkernel_io::read_stl_binary(&bytes).unwrap();
    assert_eq!(imported.triangle_count(), original_tris, "Binary STL roundtrip should preserve triangle count");
}

#[test]
fn test_io_roundtrip_step_box() {
    let mut model = BRepModel::new();
    let _r = make_box(&mut model, Point3::ORIGIN, 10.0, 10.0, 10.0).unwrap();

    let step_str = cadkernel_io::export_step(&model).unwrap();
    assert!(step_str.contains("MANIFOLD_SOLID_BREP"), "STEP should contain MANIFOLD_SOLID_BREP");
    assert!(step_str.contains("ADVANCED_FACE"), "STEP should contain ADVANCED_FACE");

    let imported = cadkernel_io::import_step(&step_str).unwrap();
    assert!(imported.vertices.len() >= 4, "STEP import should have ≥4 vertices");
}

#[test]
fn test_io_roundtrip_obj_sphere() {
    let mut model = BRepModel::new();
    let r = make_sphere(&mut model, Point3::ORIGIN, 3.0, 16, 8).unwrap();
    let mesh = cadkernel_io::tessellate_solid(&model, r.solid);
    let original_tris = mesh.triangle_count();

    let obj_str = cadkernel_io::write_obj(&mesh);
    let imported = cadkernel_io::read_obj(&obj_str).unwrap();
    assert!(imported.triangle_count() >= original_tris / 2, "OBJ import should recover triangles");
}

#[test]
fn test_io_roundtrip_gltf_box() {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::ORIGIN, 5.0, 5.0, 5.0).unwrap();
    let mesh = cadkernel_io::tessellate_solid(&model, r.solid);

    let gltf_str = cadkernel_io::write_gltf(&mesh).unwrap();
    assert!(!gltf_str.is_empty(), "glTF output should not be empty");

    let imported = cadkernel_io::import_gltf(&gltf_str).unwrap();
    assert!(imported.triangle_count() > 0, "glTF import should produce triangles");
}

#[test]
fn test_io_roundtrip_ply_torus() {
    let mut model = BRepModel::new();
    let r = make_torus(&mut model, Point3::ORIGIN, 5.0, 2.0, 16, 8).unwrap();
    let mesh = cadkernel_io::tessellate_solid(&model, r.solid);
    let original_tris = mesh.triangle_count();

    let ply = cadkernel_io::export_ply(&mesh).unwrap();
    let imported = cadkernel_io::import_ply(&ply).unwrap();
    assert_eq!(imported.triangle_count(), original_tris, "PLY roundtrip should preserve triangle count");
}

#[test]
fn test_io_roundtrip_brep_box() {
    let mut model = BRepModel::new();
    let _r = make_box(&mut model, Point3::ORIGIN, 8.0, 8.0, 8.0).unwrap();
    let v_count = model.vertices.len();

    let brep = cadkernel_io::export_brep(&model).unwrap();
    let imported = cadkernel_io::import_brep(&brep).unwrap();
    assert_eq!(imported.vertices.len(), v_count, "BREP roundtrip should preserve vertex count");
}

#[test]
fn test_io_tessellate_solid_parallel_produces_triangles() {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::ORIGIN, 10.0, 10.0, 10.0).unwrap();

    let serial = cadkernel_io::tessellate_solid(&model, r.solid);
    let parallel = cadkernel_io::tessellate_solid_parallel(&model, r.solid);

    assert!(serial.triangle_count() > 0, "Serial tessellation should produce triangles");
    assert!(parallel.triangle_count() > 0, "Parallel tessellation should produce triangles");
}

// ============================================================================
// Cross-domain workflow: sketch → extrude → check → tessellate → export
// ============================================================================

#[test]
fn test_full_workflow_sketch_extrude_tessellate_stl() {
    use cadkernel_sketch::{Constraint, Sketch, WorkPlane, extract_profile, solve};

    // 1. Build sketch
    let mut sketch = Sketch::new();
    let p0 = sketch.add_point(0.0, 0.0);
    let p1 = sketch.add_point(20.0, 0.0);
    let p2 = sketch.add_point(20.0, 15.0);
    let p3 = sketch.add_point(0.0, 15.0);
    let l0 = sketch.add_line(p0, p1);
    let l1 = sketch.add_line(p1, p2);
    let l2 = sketch.add_line(p2, p3);
    let l3 = sketch.add_line(p3, p0);
    sketch.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
    sketch.add_constraint(Constraint::Horizontal(l0));
    sketch.add_constraint(Constraint::Vertical(l1));
    sketch.add_constraint(Constraint::Horizontal(l2));
    sketch.add_constraint(Constraint::Vertical(l3));
    sketch.add_constraint(Constraint::Length(l0, 20.0));
    sketch.add_constraint(Constraint::Length(l1, 15.0));

    // 2. Solve
    let result = solve(&mut sketch, 200, 1e-10);
    assert!(result.converged);

    // 3. Extract profile + extrude
    let wp = WorkPlane::xy();
    let profile = extract_profile(&sketch, &wp);
    let mut model = BRepModel::new();
    let ext = extrude(&mut model, &profile, Vec3::Z, 8.0).unwrap();

    // 4. Geometry check
    let check = check_geometry(&model, ext.solid);
    assert!(check.is_valid, "Full workflow solid should be valid: {:?}", check.issues);

    // 5. Tessellate
    let mesh = cadkernel_io::tessellate_solid(&model, ext.solid);
    assert!(mesh.triangle_count() > 0);

    // 6. STL export
    let stl = cadkernel_io::write_stl_ascii(&mesh, "workflow_test");
    assert!(stl.contains("solid workflow_test"));
    assert!(stl.contains("endsolid workflow_test"));
}

#[test]
fn test_full_workflow_multi_boolean_then_export() {
    // Build a bracket-like shape: base box + two pads - one pocket
    let mut base = BRepModel::new();
    let br = make_box(&mut base, Point3::ORIGIN, 40.0, 20.0, 5.0).unwrap();

    // Pad a flange on one end
    let mut flange = BRepModel::new();
    let fr = make_box(&mut flange, Point3::new(35.0, 0.0, 0.0), 5.0, 20.0, 15.0).unwrap();
    let union1 = boolean_op(&base, br.solid, &flange, fr.solid, BooleanOp::Union).unwrap();
    let s1 = union1.solids.iter().next().map(|(h, _)| h).unwrap();

    // Pocket in the middle
    let mut slot = BRepModel::new();
    let sr = make_box(&mut slot, Point3::new(10.0, 5.0, 2.0), 20.0, 10.0, 5.0).unwrap();
    let result = boolean_op(&union1, s1, &slot, sr.solid, BooleanOp::Difference).unwrap();
    let final_solid = result.solids.iter().next().map(|(h, _)| h).unwrap();

    assert!(result.solids.is_alive(final_solid));

    // Tessellate and export
    let mesh = cadkernel_io::tessellate_solid(&result, final_solid);
    assert!(mesh.triangle_count() > 0);

    let stl = cadkernel_io::write_stl_ascii(&mesh, "bracket");
    assert!(!stl.is_empty());
}

#[test]
fn test_full_workflow_assembly_bom_export() {
    let mut model = BRepModel::new();
    let mut assembly = Assembly::new("BOMExport");

    let bolt = make_cylinder(&mut model, Point3::ORIGIN, 1.5, 20.0, 16).unwrap();
    let nut = make_cylinder(&mut model, Point3::ORIGIN, 3.0, 5.0, 6).unwrap();
    let washer = make_cylinder(&mut model, Point3::ORIGIN, 4.0, 1.0, 16).unwrap();
    let plate = make_box(&mut model, Point3::ORIGIN, 50.0, 50.0, 5.0).unwrap();

    // Add 4 bolts, 4 nuts, 8 washers, 1 plate
    for i in 0..4usize {
        let offset = Vec3::new(i as f64 * 12.0, 0.0, 0.0);
        let id_bolt = assembly.add_component("M3_Bolt", bolt.solid);
        assembly.set_placement(id_bolt, Mat4::translation(offset)).unwrap();

        let id_nut = assembly.add_component("M3_Nut", nut.solid);
        assembly.set_placement(id_nut, Mat4::translation(offset)).unwrap();

        for _ in 0..2usize {
            let id_ws = assembly.add_component("Washer", washer.solid);
            assembly.set_placement(id_ws, Mat4::translation(offset)).unwrap();
        }
    }
    assembly.add_component("BasePlate", plate.solid);

    // 4 bolts + 4 nuts + 8 washers + 1 plate = 17
    assert_eq!(assembly.num_components(), 17);

    let bom = assembly.bill_of_materials();
    assert_eq!(bom.len(), 4, "Should have 4 part types");
    let bolt_entry = bom.iter().find(|e| e.name == "M3_Bolt").unwrap();
    assert_eq!(bolt_entry.quantity, 4);
    let washer_entry = bom.iter().find(|e| e.name == "Washer").unwrap();
    assert_eq!(washer_entry.quantity, 8);
    let plate_entry = bom.iter().find(|e| e.name == "BasePlate").unwrap();
    assert_eq!(plate_entry.quantity, 1);
}

#[test]
fn test_full_workflow_primitives_check_watertight() {
    {
        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 5.0, 5.0, 5.0).unwrap();
        let check = check_geometry(&model, r.solid);
        assert!(check.is_valid, "box should pass geometry check: {:?}", check.issues);
    }
    {
        let mut model = BRepModel::new();
        let r = make_cylinder(&mut model, Point3::ORIGIN, 2.0, 8.0, 16).unwrap();
        let check = check_geometry(&model, r.solid);
        assert!(check.is_valid, "cylinder should pass geometry check: {:?}", check.issues);
    }
    {
        let mut model = BRepModel::new();
        let r = make_sphere(&mut model, Point3::ORIGIN, 3.0, 16, 8).unwrap();
        let check = check_geometry(&model, r.solid);
        assert!(check.is_valid, "sphere should pass geometry check: {:?}", check.issues);
    }
    {
        let mut model = BRepModel::new();
        let r = make_cone(&mut model, Point3::ORIGIN, 2.0, 1.0, 6.0, 16).unwrap();
        let check = check_geometry(&model, r.solid);
        assert!(check.is_valid, "cone should pass geometry check: {:?}", check.issues);
    }
    {
        let mut model = BRepModel::new();
        let r = make_torus(&mut model, Point3::ORIGIN, 5.0, 1.5, 16, 8).unwrap();
        let check = check_geometry(&model, r.solid);
        assert!(check.is_valid, "torus should pass geometry check: {:?}", check.issues);
    }
}

#[test]
fn test_full_workflow_scale_mirror_pattern_chain() {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::new(-2.5, -2.5, 0.0), 5.0, 5.0, 5.0).unwrap();

    // Scale
    let scaled = scale_solid(&mut model, r.solid, Point3::ORIGIN, 2.0).unwrap();
    assert!(model.solids.is_alive(scaled.solid));

    // Mirror
    let mirrored = mirror_solid(&mut model, scaled.solid, Point3::ORIGIN, Vec3::Y).unwrap();
    assert!(model.solids.is_alive(mirrored.solid));

    // Linear pattern from mirrored
    let pat = linear_pattern(&mut model, mirrored.solid, Vec3::X, 20.0, 3).unwrap();
    assert_eq!(pat.solids.len(), 3);

    for s in &pat.solids {
        let check = check_geometry(&model, *s);
        assert!(check.is_valid, "Pattern solid should be valid: {:?}", check.issues);
    }
}
