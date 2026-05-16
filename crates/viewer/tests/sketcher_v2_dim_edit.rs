use cadkernel_sketch::{Constraint, Sketch, SketchDimensionKind};

fn two_point_sketch() -> Sketch {
    let mut sketch = Sketch::new();
    let p0 = sketch.add_point(0.0, 0.0);
    let p1 = sketch.add_point(10.0, 0.0);
    sketch.add_line(p0, p1);
    sketch
}

#[test]
fn dimension_value_reports_distance_constraint() {
    let mut sketch = two_point_sketch();
    sketch.add_constraint(Constraint::Distance(
        cadkernel_sketch::PointId(0),
        cadkernel_sketch::PointId(1),
        10.0,
    ));

    let value = sketch.dimension_constraint_value(0).unwrap();

    assert_eq!(value.kind, SketchDimensionKind::Distance);
    assert_eq!(value.value, 10.0);
}

#[test]
fn update_distance_constraint_changes_value() {
    let mut sketch = two_point_sketch();
    sketch.add_constraint(Constraint::Distance(
        cadkernel_sketch::PointId(0),
        cadkernel_sketch::PointId(1),
        10.0,
    ));

    let updated = sketch.update_dimension_constraint(0, 12.5).unwrap();

    assert_eq!(updated.kind, SketchDimensionKind::Distance);
    assert_eq!(updated.value, 12.5);
    assert!(matches!(sketch.constraints[0], Constraint::Distance(_, _, d) if d == 12.5));
}

#[test]
fn update_length_constraint_changes_value() {
    let mut sketch = two_point_sketch();
    sketch.add_constraint(Constraint::Length(cadkernel_sketch::LineId(0), 10.0));

    sketch.update_dimension_constraint(0, 8.0).unwrap();

    assert!(matches!(sketch.constraints[0], Constraint::Length(_, value) if value == 8.0));
}

#[test]
fn update_angle_constraint_accepts_degrees_and_stores_radians() {
    let mut sketch = Sketch::new();
    let p0 = sketch.add_point(0.0, 0.0);
    let p1 = sketch.add_point(1.0, 0.0);
    let p2 = sketch.add_point(0.0, 1.0);
    let l0 = sketch.add_line(p0, p1);
    let l1 = sketch.add_line(p0, p2);
    sketch.add_constraint(Constraint::Angle(l0, l1, 45.0_f64.to_radians()));

    let before = sketch.dimension_constraint_value(0).unwrap();
    let updated = sketch.update_dimension_constraint(0, 90.0).unwrap();

    assert_eq!(before.value, 45.0);
    assert_eq!(updated.kind, SketchDimensionKind::Angle);
    assert!((updated.value - 90.0).abs() < 1e-12);
    assert!(
        matches!(sketch.constraints[0], Constraint::Angle(_, _, a) if (a - std::f64::consts::FRAC_PI_2).abs() < 1e-12)
    );
}

#[test]
fn update_radius_constraint_changes_value() {
    let mut sketch = two_point_sketch();
    sketch.add_constraint(Constraint::Radius(
        cadkernel_sketch::PointId(0),
        cadkernel_sketch::PointId(1),
        5.0,
    ));

    sketch.update_dimension_constraint(0, 3.25).unwrap();

    assert!(matches!(sketch.constraints[0], Constraint::Radius(_, _, value) if value == 3.25));
}

#[test]
fn update_diameter_constraint_changes_value() {
    let mut sketch = two_point_sketch();
    sketch.add_constraint(Constraint::Diameter(
        cadkernel_sketch::PointId(0),
        cadkernel_sketch::PointId(1),
        10.0,
    ));

    sketch.update_dimension_constraint(0, 6.5).unwrap();

    assert!(matches!(sketch.constraints[0], Constraint::Diameter(_, _, value) if value == 6.5));
}

#[test]
fn update_horizontal_distance_constraint_changes_value() {
    let mut sketch = two_point_sketch();
    sketch.add_constraint(Constraint::HorizontalDistance(
        cadkernel_sketch::PointId(0),
        cadkernel_sketch::PointId(1),
        10.0,
    ));

    sketch.update_dimension_constraint(0, 11.0).unwrap();

    assert!(
        matches!(sketch.constraints[0], Constraint::HorizontalDistance(_, _, value) if value == 11.0)
    );
}

#[test]
fn update_vertical_distance_constraint_changes_value() {
    let mut sketch = two_point_sketch();
    sketch.add_constraint(Constraint::VerticalDistance(
        cadkernel_sketch::PointId(0),
        cadkernel_sketch::PointId(1),
        2.0,
    ));

    sketch.update_dimension_constraint(0, 4.0).unwrap();

    assert!(
        matches!(sketch.constraints[0], Constraint::VerticalDistance(_, _, value) if value == 4.0)
    );
}

#[test]
fn negative_dimension_value_is_rejected_without_mutation() {
    let mut sketch = two_point_sketch();
    sketch.add_constraint(Constraint::Length(cadkernel_sketch::LineId(0), 10.0));

    assert!(sketch.update_dimension_constraint(0, -1.0).is_err());

    assert!(matches!(sketch.constraints[0], Constraint::Length(_, value) if value == 10.0));
}

#[test]
fn zero_dimension_value_is_rejected_without_mutation() {
    let mut sketch = two_point_sketch();
    sketch.add_constraint(Constraint::Length(cadkernel_sketch::LineId(0), 10.0));

    assert!(sketch.update_dimension_constraint(0, 0.0).is_err());

    assert!(matches!(sketch.constraints[0], Constraint::Length(_, value) if value == 10.0));
}

#[test]
fn non_finite_dimension_value_is_rejected_without_mutation() {
    let mut sketch = two_point_sketch();
    sketch.add_constraint(Constraint::Length(cadkernel_sketch::LineId(0), 10.0));

    assert!(sketch.update_dimension_constraint(0, f64::NAN).is_err());

    assert!(matches!(sketch.constraints[0], Constraint::Length(_, value) if value == 10.0));
}

#[test]
fn non_dimensional_constraint_is_rejected() {
    let mut sketch = two_point_sketch();
    sketch.add_constraint(Constraint::Horizontal(cadkernel_sketch::LineId(0)));

    let err = sketch.update_dimension_constraint(0, 5.0).unwrap_err();

    assert!(err.to_string().contains("not dimensional"));
}

#[test]
fn invalid_dimension_constraint_index_is_rejected() {
    let mut sketch = two_point_sketch();

    let err = sketch.update_dimension_constraint(2, 5.0).unwrap_err();

    assert!(err.to_string().contains("out of range"));
}
