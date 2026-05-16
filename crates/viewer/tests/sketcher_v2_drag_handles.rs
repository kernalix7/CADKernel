use cadkernel_sketch::{
    Constraint, ExternalReferenceGeometry, ExternalReferenceSource, LineId, PointId, Sketch,
    analyze_profiles, drag_solve,
};

#[test]
fn drag_solve_free_point_moves_to_target() {
    let mut sketch = Sketch::new();
    let p = sketch.add_point(0.0, 0.0);

    let result = drag_solve(&mut sketch, p, 3.0, 4.0, 50, 1e-8);

    assert!(result.converged);
    assert!((sketch.points[p.0].position.x - 3.0).abs() < 1e-6);
    assert!((sketch.points[p.0].position.y - 4.0).abs() < 1e-6);
}

#[test]
fn drag_solve_invalid_point_is_rejected_without_constraint_leak() {
    let mut sketch = Sketch::new();
    sketch.add_point(0.0, 0.0);

    let result = drag_solve(&mut sketch, PointId(4), 1.0, 1.0, 50, 1e-8);

    assert!(!result.converged);
    assert!(result.residual.is_infinite());
    assert!(sketch.constraints.is_empty());
}

#[test]
fn drag_solve_non_finite_target_is_rejected_without_constraint_leak() {
    let mut sketch = Sketch::new();
    let p = sketch.add_point(0.0, 0.0);

    let result = drag_solve(&mut sketch, p, f64::NAN, 1.0, 50, 1e-8);

    assert!(!result.converged);
    assert!(result.residual.is_infinite());
    assert!(sketch.constraints.is_empty());
}

#[test]
fn drag_solve_removes_temporary_constraint_after_success() {
    let mut sketch = Sketch::new();
    let p = sketch.add_point(0.0, 0.0);
    sketch.add_constraint(Constraint::Fixed(p, 0.0, 0.0));
    let before = sketch.constraints.len();

    let _ = drag_solve(&mut sketch, p, 0.0, 0.0, 50, 1e-8);

    assert_eq!(sketch.constraints.len(), before);
}

#[test]
fn drag_solve_distance_constraint_moves_to_consistent_target() {
    let mut sketch = Sketch::new();
    let p0 = sketch.add_point(0.0, 0.0);
    let p1 = sketch.add_point(5.0, 0.0);
    sketch.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
    sketch.add_constraint(Constraint::Distance(p0, p1, 5.0));

    let result = drag_solve(&mut sketch, p1, 3.0, 4.0, 80, 1e-8);

    assert!(result.converged);
    let p = sketch.points[p1.0].position;
    assert!((p.x - 3.0).abs() < 1e-5);
    assert!((p.y - 4.0).abs() < 1e-5);
}

#[test]
fn drag_solve_horizontal_constraint_accepts_on_axis_target() {
    let mut sketch = Sketch::new();
    let p0 = sketch.add_point(0.0, 0.0);
    let p1 = sketch.add_point(5.0, 0.0);
    let line = sketch.add_line(p0, p1);
    sketch.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
    sketch.add_constraint(Constraint::Horizontal(line));

    let result = drag_solve(&mut sketch, p1, 8.0, 0.0, 50, 1e-8);

    assert!(result.converged);
    assert!((sketch.points[p1.0].position.y - 0.0).abs() < 1e-8);
    assert!((sketch.points[p1.0].position.x - 8.0).abs() < 1e-6);
}

#[test]
fn drag_solve_conflicting_fixed_constraint_reports_nonconvergence() {
    let mut sketch = Sketch::new();
    let p = sketch.add_point(0.0, 0.0);
    sketch.add_constraint(Constraint::Fixed(p, 0.0, 0.0));

    let result = drag_solve(&mut sketch, p, 4.0, 0.0, 12, 1e-10);

    assert!(!result.converged);
    assert!(result.residual > 1e-6);
}

#[test]
fn repeated_drag_solve_updates_same_point() {
    let mut sketch = Sketch::new();
    let p = sketch.add_point(0.0, 0.0);

    assert!(drag_solve(&mut sketch, p, 1.0, 2.0, 50, 1e-8).converged);
    assert!(drag_solve(&mut sketch, p, -2.0, 3.0, 50, 1e-8).converged);

    assert!((sketch.points[p.0].position.x + 2.0).abs() < 1e-6);
    assert!((sketch.points[p.0].position.y - 3.0).abs() < 1e-6);
}

#[test]
fn external_edge_reference_creates_construction_ghost_line() {
    let mut sketch = Sketch::new();

    let line = sketch.add_external_line_reference(
        ExternalReferenceSource::EdgeRef(7),
        (0.0, 0.0),
        (2.0, 0.0),
    );

    assert_eq!(line, LineId(0));
    assert_eq!(sketch.external_reference_count(), 1);
    assert_eq!(sketch.construction_points.len(), 2);
    assert_eq!(sketch.construction_lines, vec![LineId(0)]);
    assert!(matches!(
        sketch.external_references[0].geometry,
        ExternalReferenceGeometry::Line(LineId(0))
    ));
}

#[test]
fn external_point_reference_creates_construction_point() {
    let mut sketch = Sketch::new();

    let point = sketch.add_external_point_reference(ExternalReferenceSource::FaceRef(3), 1.0, 2.0);

    assert_eq!(point, PointId(0));
    assert_eq!(sketch.external_reference_count(), 1);
    assert_eq!(sketch.construction_points, vec![PointId(0)]);
    assert!(sketch.construction_lines.is_empty());
}

#[test]
fn external_reference_lines_are_ignored_by_profile_analysis() {
    let mut sketch = Sketch::new();
    let p0 = sketch.add_point(0.0, 0.0);
    let p1 = sketch.add_point(1.0, 0.0);
    let p2 = sketch.add_point(1.0, 1.0);
    let p3 = sketch.add_point(0.0, 1.0);
    sketch.add_line(p0, p1);
    sketch.add_line(p1, p2);
    sketch.add_line(p2, p3);
    sketch.add_line(p3, p0);
    sketch.add_external_line_reference(ExternalReferenceSource::EdgeRef(9), (0.0, 0.0), (1.0, 1.0));

    let profile = analyze_profiles(&sketch);

    assert!(profile.is_single_closed_profile());
    assert_eq!(profile.construction_line_count, 1);
}
