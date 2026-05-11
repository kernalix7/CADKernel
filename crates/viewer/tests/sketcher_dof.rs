//! Gate #8 — sketcher Degrees-of-Freedom readout.
//!
//! Locks in the DoF readout that the Sketcher status bar renders
//! (`gui/status_bar.rs:112` → `SketchMode::degrees_of_freedom()`). The
//! viewer's DoF formula is `2 * points − Σ constraint_dofs` with per-
//! constraint weights defined in `gui/sketch_state.rs::sketch_degrees_of_freedom`.
//!
//! The status bar uses the sign of this value to display:
//!   - "Empty sketch"        when `points == 0`
//!   - "Fully constrained"   when `dof <= 0`
//!   - "Under-constrained (N DOF)" when `dof > 0`
//!
//! Drag-respects-constraints is a separate sub-gate of Gate #8 and is
//! deferred — the live drag path runs through `sketch_ui.rs` and the
//! Newton-Raphson solver in `cadkernel_sketch::solver`. Adding a headless
//! drag harness needs a dedicated task.

use cadkernel_sketch::{Constraint, Sketch};
use cadkernel_viewer::test_support::sketch_degrees_of_freedom;

// ---------------------------------------------------------------------------
// Baseline cases — no constraints.
// ---------------------------------------------------------------------------

#[test]
fn empty_sketch_has_zero_dof() {
    let sk = Sketch::new();
    assert_eq!(sketch_degrees_of_freedom(&sk), 0);
}

#[test]
fn single_point_has_2_dof() {
    let mut sk = Sketch::new();
    sk.add_point(0.0, 0.0);
    assert_eq!(sketch_degrees_of_freedom(&sk), 2);
}

#[test]
fn single_line_has_4_dof() {
    let mut sk = Sketch::new();
    let p0 = sk.add_point(0.0, 0.0);
    let p1 = sk.add_point(1.0, 0.0);
    sk.add_line(p0, p1);
    // Two endpoints, each 2 DoF, no shared-point reduction in this formula.
    assert_eq!(sketch_degrees_of_freedom(&sk), 4);
}

// ---------------------------------------------------------------------------
// Single-constraint cases.
// ---------------------------------------------------------------------------

#[test]
fn line_with_horizontal_has_3_dof() {
    let mut sk = Sketch::new();
    let p0 = sk.add_point(0.0, 0.0);
    let p1 = sk.add_point(1.0, 0.0);
    let line = sk.add_line(p0, p1);
    sk.add_constraint(Constraint::Horizontal(line));
    // 4 - 1 = 3
    assert_eq!(sketch_degrees_of_freedom(&sk), 3);
}

#[test]
fn line_with_distance_has_3_dof() {
    let mut sk = Sketch::new();
    let p0 = sk.add_point(0.0, 0.0);
    let p1 = sk.add_point(1.0, 0.0);
    sk.add_line(p0, p1);
    sk.add_constraint(Constraint::Distance(p0, p1, 1.0));
    // 4 - 1 = 3
    assert_eq!(sketch_degrees_of_freedom(&sk), 3);
}

#[test]
fn line_with_length_has_3_dof() {
    let mut sk = Sketch::new();
    let p0 = sk.add_point(0.0, 0.0);
    let p1 = sk.add_point(1.0, 0.0);
    let line = sk.add_line(p0, p1);
    sk.add_constraint(Constraint::Length(line, 1.0));
    assert_eq!(sketch_degrees_of_freedom(&sk), 3);
}

#[test]
fn line_with_vertical_has_3_dof() {
    let mut sk = Sketch::new();
    let p0 = sk.add_point(0.0, 0.0);
    let p1 = sk.add_point(0.0, 1.0);
    let line = sk.add_line(p0, p1);
    sk.add_constraint(Constraint::Vertical(line));
    assert_eq!(sketch_degrees_of_freedom(&sk), 3);
}

// ---------------------------------------------------------------------------
// Compound constraints.
// ---------------------------------------------------------------------------

#[test]
fn line_with_horizontal_and_distance_has_2_dof() {
    let mut sk = Sketch::new();
    let p0 = sk.add_point(0.0, 0.0);
    let p1 = sk.add_point(1.0, 0.0);
    let line = sk.add_line(p0, p1);
    sk.add_constraint(Constraint::Horizontal(line));
    sk.add_constraint(Constraint::Distance(p0, p1, 1.0));
    // 4 - 1 (horizontal) - 1 (distance) = 2
    assert_eq!(sketch_degrees_of_freedom(&sk), 2);
}

#[test]
fn fixed_point_removes_2_dof() {
    let mut sk = Sketch::new();
    let p = sk.add_point(3.0, 4.0);
    sk.add_constraint(Constraint::Fixed(p, 3.0, 4.0));
    // 2 - 2 = 0
    assert_eq!(sketch_degrees_of_freedom(&sk), 0);
}

#[test]
fn coincident_constraint_removes_2_dof() {
    let mut sk = Sketch::new();
    let p0 = sk.add_point(0.0, 0.0);
    let p1 = sk.add_point(0.0, 0.0);
    sk.add_constraint(Constraint::Coincident(p0, p1));
    // 4 - 2 = 2
    assert_eq!(sketch_degrees_of_freedom(&sk), 2);
}

// ---------------------------------------------------------------------------
// Triangle: 3 lines, 3 shared corners, no other constraints.
// ---------------------------------------------------------------------------

#[test]
fn triangle_of_3_lines_with_shared_endpoints_has_6_dof() {
    // Build a triangle by reusing the same point ids — the formula counts
    // points only, not endpoint occurrences, so a 3-point triangle has
    // 3 * 2 = 6 DoF with no constraints.
    let mut sk = Sketch::new();
    let p0 = sk.add_point(0.0, 0.0);
    let p1 = sk.add_point(1.0, 0.0);
    let p2 = sk.add_point(0.5, 1.0);
    sk.add_line(p0, p1);
    sk.add_line(p1, p2);
    sk.add_line(p2, p0);
    assert_eq!(sketch_degrees_of_freedom(&sk), 6);
}

#[test]
fn triangle_with_3_horizontals_loses_3_dof() {
    // (Geometrically over-constrained, but the DoF count is purely a tally.)
    let mut sk = Sketch::new();
    let p0 = sk.add_point(0.0, 0.0);
    let p1 = sk.add_point(1.0, 0.0);
    let p2 = sk.add_point(0.5, 1.0);
    let l0 = sk.add_line(p0, p1);
    let l1 = sk.add_line(p1, p2);
    let l2 = sk.add_line(p2, p0);
    sk.add_constraint(Constraint::Horizontal(l0));
    sk.add_constraint(Constraint::Horizontal(l1));
    sk.add_constraint(Constraint::Horizontal(l2));
    // 6 - 3 = 3
    assert_eq!(sketch_degrees_of_freedom(&sk), 3);
}

#[test]
fn fully_constrained_box_yields_zero_or_negative_dof() {
    // Square at origin: 4 points (8 DoF), pinned by Fixed on p0 (−2),
    // Horizontal on bottom (−1), Vertical on left (−1), Length 1 on each
    // of the 4 sides (−4). Total removed = 8. Expected DoF = 0.
    let mut sk = Sketch::new();
    let p0 = sk.add_point(0.0, 0.0);
    let p1 = sk.add_point(1.0, 0.0);
    let p2 = sk.add_point(1.0, 1.0);
    let p3 = sk.add_point(0.0, 1.0);
    let lbot = sk.add_line(p0, p1);
    let lright = sk.add_line(p1, p2);
    let ltop = sk.add_line(p2, p3);
    let lleft = sk.add_line(p3, p0);

    sk.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
    sk.add_constraint(Constraint::Horizontal(lbot));
    sk.add_constraint(Constraint::Vertical(lleft));
    sk.add_constraint(Constraint::Length(lbot, 1.0));
    sk.add_constraint(Constraint::Length(lright, 1.0));
    sk.add_constraint(Constraint::Length(ltop, 1.0));
    sk.add_constraint(Constraint::Length(lleft, 1.0));

    let dof = sketch_degrees_of_freedom(&sk);
    // 8 - 2 - 1 - 1 - 1 - 1 - 1 - 1 = 0
    assert_eq!(dof, 0, "fully-constrained square should report 0 DoF");
}

// ---------------------------------------------------------------------------
// Status-bar readout invariants — the *value* drives the rendered label.
// (Mirrors the if/else in `gui/status_bar.rs:114-120`.)
// ---------------------------------------------------------------------------

#[test]
fn under_constrained_dof_is_positive() {
    let mut sk = Sketch::new();
    let p0 = sk.add_point(0.0, 0.0);
    let p1 = sk.add_point(1.0, 0.0);
    sk.add_line(p0, p1);
    // No constraints → 4 DoF → status bar shows "Under-constrained (4 DOF)".
    assert!(sketch_degrees_of_freedom(&sk) > 0);
}

#[test]
fn fully_constrained_dof_is_non_positive() {
    let mut sk = Sketch::new();
    let p = sk.add_point(0.0, 0.0);
    sk.add_constraint(Constraint::Fixed(p, 0.0, 0.0));
    assert!(sketch_degrees_of_freedom(&sk) <= 0);
}
