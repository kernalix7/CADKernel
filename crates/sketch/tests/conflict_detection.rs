use std::f64::consts::PI;

use cadkernel_sketch::{
    BlockStatus, Constraint, Sketch, analyze_blocks, detect_conflict, detect_conflict_in_block,
    is_conflicting,
};

fn assert_deletion_minimal(sketch: &Sketch, conflict: &[usize]) {
    assert!(is_conflicting(sketch, conflict));
    for index in 0..conflict.len() {
        let mut candidate = conflict.to_vec();
        candidate.remove(index);
        assert!(
            !is_conflicting(sketch, &candidate),
            "removing one constraint should clear conflict: {candidate:?}"
        );
    }
}

#[test]
fn empty_constraint_set_has_no_conflict() {
    let sketch = Sketch::new();
    assert!(!is_conflicting(&sketch, &[]));
    assert!(detect_conflict(&sketch, &[]).is_empty());
}

#[test]
fn single_fixed_constraint_has_no_conflict() {
    let mut sketch = Sketch::new();
    let point = sketch.add_point(0.0, 0.0);
    sketch.add_constraint(Constraint::Fixed(point, 0.0, 0.0));
    assert!(!is_conflicting(&sketch, &[0]));
    assert!(detect_conflict(&sketch, &[0]).is_empty());
}

#[test]
fn duplicate_fixed_constraints_are_conflicting() {
    let mut sketch = Sketch::new();
    let point = sketch.add_point(0.0, 0.0);
    sketch.add_constraint(Constraint::Fixed(point, 0.0, 0.0));
    sketch.add_constraint(Constraint::Fixed(point, 0.0, 0.0));

    assert!(is_conflicting(&sketch, &[0, 1]));
    let conflict = detect_conflict(&sketch, &[0, 1]);
    assert_eq!(conflict, vec![0, 1]);
    assert_deletion_minimal(&sketch, &conflict);
}

#[test]
fn contradictory_fixed_constraints_are_minimized_to_pair() {
    let mut sketch = Sketch::new();
    let point = sketch.add_point(0.0, 0.0);
    sketch.add_constraint(Constraint::Fixed(point, 0.0, 0.0));
    sketch.add_constraint(Constraint::Fixed(point, 5.0, 0.0));

    let conflict = detect_conflict(&sketch, &[0, 1]);
    assert_eq!(conflict, vec![0, 1]);
    assert_deletion_minimal(&sketch, &conflict);
}

#[test]
fn unrelated_constraints_are_excluded_from_conflict() {
    let mut sketch = Sketch::new();
    let pinned = sketch.add_point(0.0, 0.0);
    let other = sketch.add_point(10.0, 0.0);
    sketch.add_constraint(Constraint::Fixed(pinned, 0.0, 0.0));
    sketch.add_constraint(Constraint::Fixed(pinned, 1.0, 0.0));
    sketch.add_constraint(Constraint::Fixed(other, 10.0, 0.0));

    let conflict = detect_conflict(&sketch, &[0, 1, 2]);
    assert_eq!(conflict, vec![0, 1]);
    assert_deletion_minimal(&sketch, &conflict);
}

#[test]
fn detect_conflict_in_over_constrained_block_uses_block_constraints() {
    let mut sketch = Sketch::new();
    let point = sketch.add_point(0.0, 0.0);
    sketch.add_constraint(Constraint::Fixed(point, 0.0, 0.0));
    sketch.add_constraint(Constraint::Fixed(point, 1.0, 0.0));

    let analysis = analyze_blocks(&sketch);
    assert_eq!(analysis.blocks[0].status, BlockStatus::OverConstrained);
    let conflict = detect_conflict_in_block(&sketch, &analysis.blocks[0]);
    assert_eq!(conflict, vec![0, 1]);
}

#[test]
fn invalid_constraint_ids_are_ignored() {
    let mut sketch = Sketch::new();
    let point = sketch.add_point(0.0, 0.0);
    sketch.add_constraint(Constraint::Fixed(point, 0.0, 0.0));
    sketch.add_constraint(Constraint::Fixed(point, 0.0, 0.0));

    let conflict = detect_conflict(&sketch, &[99, 1, 0, 42]);
    assert_eq!(conflict, vec![0, 1]);
}

#[test]
fn repeated_constraint_ids_are_normalized() {
    let mut sketch = Sketch::new();
    let point = sketch.add_point(0.0, 0.0);
    sketch.add_constraint(Constraint::Fixed(point, 0.0, 0.0));
    sketch.add_constraint(Constraint::Fixed(point, 0.0, 0.0));

    let conflict = detect_conflict(&sketch, &[0, 0, 1, 1]);
    assert_eq!(conflict, vec![0, 1]);
}

#[test]
fn duplicate_horizontal_constraints_are_minimal_conflict() {
    let mut sketch = Sketch::new();
    let p0 = sketch.add_point(0.0, 0.0);
    let p1 = sketch.add_point(5.0, 0.0);
    let line = sketch.add_line(p0, p1);
    sketch.add_constraint(Constraint::Horizontal(line));
    sketch.add_constraint(Constraint::Horizontal(line));

    let conflict = detect_conflict(&sketch, &[0, 1]);
    assert_eq!(conflict, vec![0, 1]);
    assert_deletion_minimal(&sketch, &conflict);
}

#[test]
fn already_pinned_point_reports_second_pin_conflict() {
    let mut sketch = Sketch::new();
    let point = sketch.add_point(2.0, 3.0);
    sketch.add_constraint(Constraint::Block(point, 2.0, 3.0));
    sketch.add_constraint(Constraint::Fixed(point, 2.0, 3.0));

    let conflict = detect_conflict(&sketch, &[0, 1]);
    assert_eq!(conflict, vec![0, 1]);
    assert_deletion_minimal(&sketch, &conflict);
}

#[test]
fn triangle_side_and_angle_set_has_minimal_conflict() {
    let mut sketch = Sketch::new();
    let p0 = sketch.add_point(0.0, 0.0);
    let p1 = sketch.add_point(1.0, 0.0);
    let p2 = sketch.add_point(0.5, 3.0_f64.sqrt() * 0.5);
    let l01 = sketch.add_line(p0, p1);
    let l12 = sketch.add_line(p1, p2);
    let l20 = sketch.add_line(p2, p0);
    let angle = 2.0 * PI / 3.0;
    sketch.add_constraint(Constraint::Distance(p0, p1, 1.0));
    sketch.add_constraint(Constraint::Distance(p1, p2, 1.0));
    sketch.add_constraint(Constraint::Distance(p2, p0, 1.0));
    sketch.add_constraint(Constraint::Angle(l01, l12, angle));
    sketch.add_constraint(Constraint::Angle(l12, l20, angle));
    sketch.add_constraint(Constraint::Angle(l20, l01, angle));

    assert!(is_conflicting(&sketch, &[0, 1, 2, 3, 4, 5]));
    let conflict = detect_conflict(&sketch, &[0, 1, 2, 3, 4, 5]);
    assert!((2..=6).contains(&conflict.len()), "{conflict:?}");
    assert_deletion_minimal(&sketch, &conflict);
}

#[test]
fn non_conflicting_mixed_constraints_return_empty_set() {
    let mut sketch = Sketch::new();
    let p0 = sketch.add_point(0.0, 0.0);
    let p1 = sketch.add_point(3.0, 0.0);
    let line = sketch.add_line(p0, p1);
    sketch.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
    sketch.add_constraint(Constraint::Horizontal(line));
    sketch.add_constraint(Constraint::Length(line, 3.0));

    assert!(!is_conflicting(&sketch, &[0, 1, 2]));
    assert!(detect_conflict(&sketch, &[0, 1, 2]).is_empty());
}
