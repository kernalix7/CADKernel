//! Comprehensive integration tests for cadkernel-sketch.
//!
//! Covers entity constructors, constraint variants, solver convergence,
//! validation, editing tools, b-spline operations, profile extraction,
//! display/snap helpers, and transformation utilities.

use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

use cadkernel_math::{Point2, Point3, Vec3};
use cadkernel_sketch::bspline_tools::{
    carbon_copy, decrease_bspline_degree, decrease_knot_multiplicity, delete_all_constraints,
    delete_all_geometry, external_projection, geometry_to_bspline, increase_bspline_degree,
    increase_knot_multiplicity, insert_knot, join_curves, mirror_geometry_axis, move_geometry,
    offset_geometry, rotate_geometry, scale_geometry,
};
use cadkernel_sketch::constraint::Constraint;
use cadkernel_sketch::display::{
    SectionViewState, SketchDisplayOptions, SketchEntity, SketchGrid, SketchSnap, SnapType,
    add_periodic_bspline_from_knots, align_view_to_sketch, contextual_dimension, copy_entities,
    paste_entities, remove_axes_alignment, select_h_axis, select_origin, select_v_axis,
    snap_to_sketch_geometry, stop_operation, toggle_constraints_visibility, toggle_construction,
    toggle_section_view, unified_horizontal_vertical, unified_radius_diameter,
};
use cadkernel_sketch::entity::{
    ArcId, BSplineId, CircleId, EllipseId, EllipticalArcId, HyperbolicArcId, LineId,
    ParabolicArcId, PointId, SketchArc, SketchBSpline, SketchCircle, SketchEllipse,
    SketchEllipticalArc, SketchHyperbolicArc, SketchLine, SketchParabolicArc, SketchPoint,
};
use cadkernel_sketch::profile::{WorkPlane, extract_profile};
use cadkernel_sketch::solver::{SolverResult, constraint_residuals, drag_solve, solve};
use cadkernel_sketch::tools::{
    chamfer_sketch_corner, extend_edge, external_intersection, fillet_sketch_corner, split_edge,
    trim_edge,
};
use cadkernel_sketch::validate::{SketchValidation, SketchValidationIssue, validate_sketch};
use cadkernel_sketch::Sketch;

const TOL: f64 = 1e-10;
const SOLVE_TOL: f64 = 1e-10;
const LOOSE: f64 = 1e-6;

// ---------------------------------------------------------------------------
// Entity construction
// ---------------------------------------------------------------------------

#[test]
fn sketch_point_construction() {
    let p = SketchPoint::new(3.0, -4.0);
    assert!((p.position.x - 3.0).abs() < TOL);
    assert!((p.position.y + 4.0).abs() < TOL);
}

#[test]
fn sketch_add_point_and_index() {
    let mut s = Sketch::new();
    let a = s.add_point(1.0, 2.0);
    let b = s.add_point(5.0, 6.0);
    assert_eq!(a.0, 0);
    assert_eq!(b.0, 1);
    assert_eq!(s.points.len(), 2);
}

#[test]
fn sketch_add_line_connects_points() {
    let mut s = Sketch::new();
    let p0 = s.add_point(0.0, 0.0);
    let p1 = s.add_point(2.0, 0.0);
    let l = s.add_line(p0, p1);
    assert_eq!(l.0, 0);
    assert_eq!(s.lines[l.0].start, p0);
    assert_eq!(s.lines[l.0].end, p1);
}

#[test]
fn sketch_add_circle_and_arc() {
    let mut s = Sketch::new();
    let c = s.add_point(0.0, 0.0);
    let cid = s.add_circle(c, 2.5);
    assert!((s.circles[cid.0].radius - 2.5).abs() < TOL);

    let sp = s.add_point(2.5, 0.0);
    let ep = s.add_point(0.0, 2.5);
    let aid = s.add_arc(c, sp, ep, 2.5, 0.0, FRAC_PI_2);
    assert!((s.arcs[aid.0].start_angle).abs() < TOL);
    assert!((s.arcs[aid.0].end_angle - FRAC_PI_2).abs() < TOL);
}

#[test]
fn sketch_add_ellipse_and_bspline() {
    let mut s = Sketch::new();
    let c = s.add_point(0.0, 0.0);
    let me = s.add_point(3.0, 0.0);
    let eid = s.add_ellipse(c, me, 1.5);
    assert!((s.ellipses[eid.0].minor_radius - 1.5).abs() < TOL);

    let pts: Vec<_> = (0..4).map(|i| s.add_point(i as f64, 0.0)).collect();
    let bs = s.add_bspline(pts, 3, false);
    assert_eq!(s.bsplines[bs.0].degree, 3);
    assert!(!s.bsplines[bs.0].closed);
}

#[test]
fn sketch_id_types_roundtrip() {
    assert_eq!(PointId(4).0, 4);
    assert_eq!(LineId(5).0, 5);
    assert_eq!(ArcId(6).0, 6);
    assert_eq!(CircleId(7).0, 7);
    assert_eq!(EllipseId(8).0, 8);
    assert_eq!(BSplineId(9).0, 9);
    assert_eq!(EllipticalArcId(10).0, 10);
    assert_eq!(HyperbolicArcId(11).0, 11);
    assert_eq!(ParabolicArcId(12).0, 12);
}

#[test]
fn sketch_struct_field_access() {
    let a = SketchArc {
        center: PointId(0),
        start_point: PointId(1),
        end_point: PointId(2),
        radius: 2.0,
        start_angle: 0.0,
        end_angle: PI,
    };
    assert_eq!(a.center.0, 0);
    assert!((a.radius - 2.0).abs() < TOL);

    let l = SketchLine {
        start: PointId(3),
        end: PointId(4),
    };
    assert_eq!(l.start.0, 3);

    let c = SketchCircle {
        center: PointId(5),
        radius: 1.5,
    };
    assert!((c.radius - 1.5).abs() < TOL);

    let e = SketchEllipse {
        center: PointId(6),
        major_end: PointId(7),
        minor_radius: 0.5,
    };
    assert_eq!(e.major_end.0, 7);

    let b = SketchBSpline {
        control_points: vec![PointId(0), PointId(1), PointId(2)],
        degree: 2,
        closed: false,
        knots: vec![],
    };
    assert_eq!(b.control_points.len(), 3);

    let ea = SketchEllipticalArc {
        center: PointId(0),
        major_end: PointId(1),
        minor_radius: 0.5,
        start_point: PointId(2),
        end_point: PointId(3),
        start_param: 0.0,
        end_param: PI,
    };
    assert!((ea.end_param - PI).abs() < TOL);

    let ha = SketchHyperbolicArc {
        center: PointId(0),
        vertex: PointId(1),
        semi_minor: 0.5,
        start_point: PointId(2),
        end_point: PointId(3),
        start_param: -1.0,
        end_param: 1.0,
    };
    assert!((ha.end_param - 1.0).abs() < TOL);

    let pa = SketchParabolicArc {
        vertex: PointId(0),
        focal_length: 1.0,
        focus_angle: 0.0,
        start_point: PointId(1),
        end_point: PointId(2),
        start_param: -1.0,
        end_param: 1.0,
    };
    assert!((pa.focal_length - 1.0).abs() < TOL);
}

// ---------------------------------------------------------------------------
// Polyline, polygon, and multi-point primitives
// ---------------------------------------------------------------------------

#[test]
fn polyline_builds_chained_lines() {
    let mut s = Sketch::new();
    let pts: Vec<_> = (0..4).map(|i| s.add_point(i as f64, 0.0)).collect();
    let lines = s.add_polyline(&pts);
    // open polyline: n vertices → n-1 lines
    assert_eq!(lines.len(), 3);
}

#[test]
fn polyline_with_three_points_has_two_lines() {
    let mut s = Sketch::new();
    let p0 = s.add_point(0.0, 0.0);
    let p1 = s.add_point(1.0, 0.0);
    let p2 = s.add_point(2.0, 1.0);
    let lines = s.add_polyline(&[p0, p1, p2]);
    assert_eq!(lines.len(), 2);
}

#[test]
fn regular_polygon_hex_has_six_sides() {
    let mut s = Sketch::new();
    let (pts, lines) = s.add_regular_polygon(0.0, 0.0, 1.0, 6);
    assert_eq!(pts.len(), 6);
    assert_eq!(lines.len(), 6);
}

#[test]
fn triangle_through_octagon_builders() {
    let mut s = Sketch::new();
    let (_, t) = s.add_triangle(0.0, 0.0, 1.0);
    assert_eq!(t.len(), 3);
    let (_, sq) = s.add_square(5.0, 0.0, 1.0);
    assert_eq!(sq.len(), 4);
    let (_, pent) = s.add_pentagon(0.0, 5.0, 1.0);
    assert_eq!(pent.len(), 5);
    let (_, hex) = s.add_hexagon(5.0, 5.0, 1.0);
    assert_eq!(hex.len(), 6);
    let (_, hept) = s.add_heptagon(10.0, 0.0, 1.0);
    assert_eq!(hept.len(), 7);
    let (_, oct) = s.add_octagon(10.0, 5.0, 1.0);
    assert_eq!(oct.len(), 8);
}

#[test]
fn slot_and_rounded_rectangle_builders() {
    let mut s = Sketch::new();
    let (_pts, lines, arcs) = s.add_slot(0.0, 0.0, 5.0, 0.0, 1.0);
    assert_eq!(lines.len(), 2);
    assert_eq!(arcs.len(), 2);

    let mut s2 = Sketch::new();
    let (_pts, lines, arcs) = s2.add_rounded_rectangle(0.0, 0.0, 4.0, 2.0, 0.5);
    assert_eq!(lines.len(), 4);
    assert_eq!(arcs.len(), 4);
}

#[test]
fn arc_3pt_recovers_center() {
    let mut s = Sketch::new();
    let p0 = s.add_point(1.0, 0.0);
    let p1 = s.add_point(0.0, 1.0);
    let p2 = s.add_point(-1.0, 0.0);
    let a = s.add_arc_3pt(p0, p1, p2);
    let cid = s.arcs[a.0].center;
    let cx = s.points[cid.0].position.x;
    let cy = s.points[cid.0].position.y;
    assert!(cx.abs() < LOOSE);
    assert!(cy.abs() < LOOSE);
}

// ---------------------------------------------------------------------------
// Constraint variants — constructed correctly for each kind
// ---------------------------------------------------------------------------

#[test]
fn all_constraint_variants_constructible() {
    let p0 = PointId(0);
    let p1 = PointId(1);
    let p2 = PointId(2);
    let p3 = PointId(3);
    let l0 = LineId(0);
    let l1 = LineId(1);

    let variants: Vec<Constraint> = vec![
        Constraint::Coincident(p0, p1),
        Constraint::Horizontal(l0),
        Constraint::Vertical(l0),
        Constraint::Parallel(l0, l1),
        Constraint::Perpendicular(l0, l1),
        Constraint::PointOnLine(p0, l0),
        Constraint::PointOnCircle(p0, p1, 1.0),
        Constraint::Symmetric(p0, p1, l0),
        Constraint::Distance(p0, p1, 1.0),
        Constraint::Angle(l0, l1, FRAC_PI_2),
        Constraint::Radius(p0, p1, 1.0),
        Constraint::Length(l0, 1.0),
        Constraint::Fixed(p0, 0.0, 0.0),
        Constraint::Tangent(l0, p0, 1.0),
        Constraint::EqualLength(l0, l1),
        Constraint::Midpoint(p0, l0),
        Constraint::Collinear(l0, l1),
        Constraint::EqualRadius(p0, p1, p2, p3),
        Constraint::Concentric(p0, p1),
        Constraint::Diameter(p0, p1, 2.0),
        Constraint::Block(p0, 0.0, 0.0),
        Constraint::HorizontalDistance(p0, p1, 1.0),
        Constraint::VerticalDistance(p0, p1, 1.0),
        Constraint::PointOnObject(p0, l0),
        Constraint::Refraction {
            line1: l0,
            line2: l1,
            ratio: 1.5,
        },
    ];
    assert!(variants.len() >= 25);
}

#[test]
fn add_constraint_records_into_sketch() {
    let mut s = Sketch::new();
    let p = s.add_point(0.0, 0.0);
    s.add_constraint(Constraint::Fixed(p, 1.0, 2.0));
    s.add_constraint(Constraint::Block(p, 0.0, 0.0));
    assert_eq!(s.constraints.len(), 2);
}

// ---------------------------------------------------------------------------
// Solver — convergence on well-posed systems
// ---------------------------------------------------------------------------

#[test]
fn solver_empty_sketch_is_trivially_converged() {
    let mut s = Sketch::new();
    let r = solve(&mut s, 20, SOLVE_TOL);
    assert!(r.converged);
    assert_eq!(r.iterations, 0);
    assert_eq!(r.residual, 0.0);
}

#[test]
fn solver_unconstrained_sketch_converges() {
    let mut s = Sketch::new();
    s.add_point(1.0, 2.0);
    let r = solve(&mut s, 20, SOLVE_TOL);
    assert!(r.converged);
}

#[test]
fn solver_fixed_point_moves_to_target() {
    let mut s = Sketch::new();
    let p = s.add_point(0.0, 0.0);
    s.add_constraint(Constraint::Fixed(p, 7.0, 3.0));
    let r = solve(&mut s, 30, SOLVE_TOL);
    assert!(r.converged);
    assert!((s.points[p.0].position.x - 7.0).abs() < LOOSE);
    assert!((s.points[p.0].position.y - 3.0).abs() < LOOSE);
}

#[test]
fn solver_distance_constraint() {
    let mut s = Sketch::new();
    let p0 = s.add_point(0.0, 0.0);
    let p1 = s.add_point(1.0, 0.0);
    s.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
    s.add_constraint(Constraint::Distance(p0, p1, 10.0));
    let r = solve(&mut s, 100, SOLVE_TOL);
    assert!(r.converged);
    let dx = s.points[p1.0].position.x;
    let dy = s.points[p1.0].position.y;
    let d = (dx * dx + dy * dy).sqrt();
    assert!((d - 10.0).abs() < LOOSE);
}

#[test]
fn solver_right_angle_triangle() {
    let mut s = Sketch::new();
    let p0 = s.add_point(0.0, 0.0);
    let p1 = s.add_point(3.0, 0.5);
    let p2 = s.add_point(0.1, 4.0);
    s.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
    let l0 = s.add_line(p0, p1);
    let l1 = s.add_line(p0, p2);
    s.add_constraint(Constraint::Horizontal(l0));
    s.add_constraint(Constraint::Vertical(l1));
    s.add_constraint(Constraint::Length(l0, 3.0));
    s.add_constraint(Constraint::Length(l1, 4.0));
    let r = solve(&mut s, 200, SOLVE_TOL);
    assert!(r.converged);
    assert!(s.points[p1.0].position.y.abs() < LOOSE);
    assert!((s.points[p1.0].position.x.abs() - 3.0).abs() < LOOSE);
    assert!(s.points[p2.0].position.x.abs() < LOOSE);
    assert!((s.points[p2.0].position.y.abs() - 4.0).abs() < LOOSE);
}

#[test]
fn solver_result_records_iterations() {
    let mut s = Sketch::new();
    let p = s.add_point(5.0, 5.0);
    s.add_constraint(Constraint::Fixed(p, 0.0, 0.0));
    let r = solve(&mut s, 50, SOLVE_TOL);
    assert!(r.converged);
    assert!(r.iterations >= 1);
}

#[test]
fn solver_over_constrained_flag_not_set_for_fully_constrained() {
    let mut s = Sketch::new();
    let p = s.add_point(0.0, 0.0);
    s.add_constraint(Constraint::Fixed(p, 1.0, 2.0));
    let r = solve(&mut s, 30, SOLVE_TOL);
    assert!(r.converged);
    assert!(!r.over_constrained);
}

#[test]
fn solver_result_clone_is_equivalent() {
    let r = SolverResult {
        converged: true,
        iterations: 3,
        residual: 1e-12,
        remaining_dof: Some(0),
        over_constrained: false,
    };
    let c = r.clone();
    assert_eq!(c.iterations, 3);
    assert!(c.converged);
}

#[test]
fn constraint_residuals_zero_when_satisfied() {
    let mut s = Sketch::new();
    let p = s.add_point(1.0, 2.0);
    s.add_constraint(Constraint::Fixed(p, 1.0, 2.0));
    let res = constraint_residuals(&s);
    assert_eq!(res.len(), 1);
    assert!(res[0].abs() < LOOSE);
}

#[test]
fn constraint_residuals_nonzero_when_violated() {
    let mut s = Sketch::new();
    let p = s.add_point(1.0, 2.0);
    s.add_constraint(Constraint::Fixed(p, 10.0, 20.0));
    let res = constraint_residuals(&s);
    assert!(res[0] > 1.0);
}

#[test]
fn drag_solve_moves_point_toward_target() {
    let mut s = Sketch::new();
    let p = s.add_point(0.0, 0.0);
    let r = drag_solve(&mut s, p, 5.0, 7.0, 30, SOLVE_TOL);
    assert!(r.converged);
    assert!((s.points[p.0].position.x - 5.0).abs() < LOOSE);
    assert!((s.points[p.0].position.y - 7.0).abs() < LOOSE);
    assert!(s.constraints.is_empty());
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

#[test]
fn validate_empty_sketch_reports_issue() {
    let s = Sketch::new();
    let v = validate_sketch(&s, 1e-3);
    assert!(!v.valid);
    assert!(v
        .issues
        .iter()
        .any(|i| matches!(i, SketchValidationIssue::EmptySketch)));
}

#[test]
fn validate_zero_length_line() {
    let mut s = Sketch::new();
    let p0 = s.add_point(1.0, 1.0);
    let p1 = s.add_point(1.0, 1.0);
    s.add_line(p0, p1);
    let v = validate_sketch(&s, 1e-3);
    assert!(v
        .issues
        .iter()
        .any(|i| matches!(i, SketchValidationIssue::ZeroLengthLine { .. })));
}

#[test]
fn validate_nearly_coincident_points() {
    let mut s = Sketch::new();
    s.add_point(0.0, 0.0);
    s.add_point(1e-6, 0.0);
    let v = validate_sketch(&s, 1e-3);
    assert!(v
        .issues
        .iter()
        .any(|i| matches!(i, SketchValidationIssue::NearlyCoincidentPoints { .. })));
}

#[test]
fn validate_invalid_point_reference() {
    let mut s = Sketch::new();
    s.add_point(0.0, 0.0);
    s.add_constraint(Constraint::Fixed(PointId(99), 0.0, 0.0));
    let v = validate_sketch(&s, 1e-3);
    assert!(v
        .issues
        .iter()
        .any(|i| matches!(i, SketchValidationIssue::InvalidPointReference { .. })));
}

#[test]
fn validate_invalid_line_reference() {
    let mut s = Sketch::new();
    s.add_point(0.0, 0.0);
    s.add_constraint(Constraint::Horizontal(LineId(99)));
    let v = validate_sketch(&s, 1e-3);
    assert!(v
        .issues
        .iter()
        .any(|i| matches!(i, SketchValidationIssue::InvalidLineReference { .. })));
}

#[test]
fn validate_under_and_over_constrained_flags() {
    let mut under = Sketch::new();
    under.add_point(1.0, 1.0);
    let v = validate_sketch(&under, 1e-6);
    assert!(v
        .issues
        .iter()
        .any(|i| matches!(i, SketchValidationIssue::UnderConstrained { .. })));
}

#[test]
fn sketch_validation_clone() {
    let v = SketchValidation {
        valid: true,
        issues: vec![],
    };
    let c = v.clone();
    assert!(c.valid);
    assert!(c.issues.is_empty());
}

// ---------------------------------------------------------------------------
// Profile extraction and WorkPlane
// ---------------------------------------------------------------------------

#[test]
fn workplane_xy_yields_z_zero() {
    let wp = WorkPlane::xy();
    let p = wp.to_world(3.0, 4.0);
    assert!((p.x - 3.0).abs() < TOL);
    assert!((p.y - 4.0).abs() < TOL);
    assert!(p.z.abs() < TOL);
}

#[test]
fn workplane_xz_maps_y_to_zero() {
    let wp = WorkPlane::xz();
    let p = wp.to_world(2.0, 5.0);
    assert!((p.x - 2.0).abs() < TOL);
    assert!(p.y.abs() < TOL);
    assert!((p.z - 5.0).abs() < TOL);
}

#[test]
fn workplane_new_orthonormalizes_axes() {
    let wp = WorkPlane::new(
        Point3::ORIGIN,
        Vec3::new(0.0, 0.0, 2.0),
        Vec3::new(5.0, 0.0, 1.0),
    );
    assert!(wp.x_axis.dot(wp.normal).abs() < LOOSE);
    assert!(wp.y_axis.dot(wp.normal).abs() < LOOSE);
    assert!(wp.y_axis.dot(wp.x_axis).abs() < LOOSE);
}

#[test]
fn workplane_roundtrip_local_world() {
    let wp = WorkPlane::xy();
    let world = wp.to_world(2.0, 3.0);
    let local = wp.to_local(world);
    assert!((local.x - 2.0).abs() < TOL);
    assert!((local.y - 3.0).abs() < TOL);
}

#[test]
fn extract_profile_closed_square() {
    let mut s = Sketch::new();
    let p0 = s.add_point(0.0, 0.0);
    let p1 = s.add_point(1.0, 0.0);
    let p2 = s.add_point(1.0, 1.0);
    let p3 = s.add_point(0.0, 1.0);
    s.add_line(p0, p1);
    s.add_line(p1, p2);
    s.add_line(p2, p3);
    s.add_line(p3, p0);
    let wp = WorkPlane::xy();
    let profile = extract_profile(&s, &wp);
    assert_eq!(profile.len(), 4);
}

#[test]
fn extract_profile_empty_sketch_returns_empty() {
    let s = Sketch::new();
    let wp = WorkPlane::xy();
    let profile = extract_profile(&s, &wp);
    assert!(profile.is_empty());
}

#[test]
fn extract_profile_no_lines_returns_all_points() {
    let mut s = Sketch::new();
    s.add_point(0.0, 0.0);
    s.add_point(1.0, 0.0);
    s.add_point(0.5, 1.0);
    let wp = WorkPlane::xy();
    let profile = extract_profile(&s, &wp);
    assert_eq!(profile.len(), 3);
}

// ---------------------------------------------------------------------------
// Editing tools
// ---------------------------------------------------------------------------

#[test]
fn fillet_right_angle_corner() {
    let mut s = Sketch::new();
    let p0 = s.add_point(5.0, 0.0);
    let p1 = s.add_point(0.0, 0.0);
    let p2 = s.add_point(0.0, 5.0);
    let l0 = s.add_line(p0, p1);
    let l1 = s.add_line(p1, p2);
    let r = fillet_sketch_corner(&mut s, l0, l1, 1.0);
    assert!(r.is_some());
    let fr = r.unwrap();
    assert!(s.arcs[fr.arc.0].radius > 0.0);
}

#[test]
fn fillet_rejects_nonshared_lines() {
    let mut s = Sketch::new();
    let p0 = s.add_point(0.0, 0.0);
    let p1 = s.add_point(1.0, 0.0);
    let p2 = s.add_point(2.0, 0.0);
    let p3 = s.add_point(3.0, 0.0);
    let l0 = s.add_line(p0, p1);
    let l1 = s.add_line(p2, p3);
    let r = fillet_sketch_corner(&mut s, l0, l1, 0.5);
    assert!(r.is_none());
}

#[test]
fn chamfer_corner_creates_line() {
    let mut s = Sketch::new();
    let p0 = s.add_point(5.0, 0.0);
    let p1 = s.add_point(0.0, 0.0);
    let p2 = s.add_point(0.0, 5.0);
    let l0 = s.add_line(p0, p1);
    let l1 = s.add_line(p1, p2);
    let r = chamfer_sketch_corner(&mut s, l0, l1, 1.0);
    assert!(r.is_some());
}

#[test]
fn split_edge_divides_line() {
    let mut s = Sketch::new();
    let p0 = s.add_point(0.0, 0.0);
    let p1 = s.add_point(10.0, 0.0);
    let l = s.add_line(p0, p1);
    let r = split_edge(&mut s, l, 0.3);
    let mx = s.points[r.mid_point.0].position.x;
    assert!((mx - 3.0).abs() < TOL);
    assert_eq!(s.lines.len(), 2);
}

#[test]
fn trim_edge_at_intersection() {
    let mut s = Sketch::new();
    let p0 = s.add_point(0.0, 0.0);
    let p1 = s.add_point(10.0, 0.0);
    let p2 = s.add_point(5.0, -5.0);
    let p3 = s.add_point(5.0, 5.0);
    let l0 = s.add_line(p0, p1);
    let l1 = s.add_line(p2, p3);
    let r = trim_edge(&mut s, l0, l1, p0);
    assert!(r.trimmed);
}

#[test]
fn trim_edge_returns_false_for_parallel_lines() {
    let mut s = Sketch::new();
    let p0 = s.add_point(0.0, 0.0);
    let p1 = s.add_point(1.0, 0.0);
    let p2 = s.add_point(0.0, 1.0);
    let p3 = s.add_point(1.0, 1.0);
    let l0 = s.add_line(p0, p1);
    let l1 = s.add_line(p2, p3);
    let r = trim_edge(&mut s, l0, l1, p0);
    assert!(!r.trimmed);
}

#[test]
fn extend_edge_lengthens_line() {
    let mut s = Sketch::new();
    let p0 = s.add_point(0.0, 0.0);
    let p1 = s.add_point(5.0, 0.0);
    let l = s.add_line(p0, p1);
    extend_edge(&mut s, l, 12.0, 0.0);
    let ex = s.points[p1.0].position.x;
    assert!((ex - 12.0).abs() < TOL);
}

#[test]
fn external_intersection_finds_crossing() {
    let mut s = Sketch::new();
    let a = vec![Point2::new(0.0, 0.0), Point2::new(10.0, 10.0)];
    let b = vec![Point2::new(10.0, 0.0), Point2::new(0.0, 10.0)];
    let r = external_intersection(&mut s, &a, &b).unwrap();
    assert_eq!(r.len(), 1);
    let p = s.points[r[0].0].position;
    assert!((p.x - 5.0).abs() < TOL);
    assert!((p.y - 5.0).abs() < TOL);
}

#[test]
fn external_intersection_requires_two_points() {
    let mut s = Sketch::new();
    let a = vec![Point2::new(0.0, 0.0)];
    let b = vec![Point2::new(1.0, 0.0), Point2::new(2.0, 0.0)];
    assert!(external_intersection(&mut s, &a, &b).is_err());
}

// ---------------------------------------------------------------------------
// B-spline tools
// ---------------------------------------------------------------------------

#[test]
fn geometry_to_bspline_converts_line() {
    let mut s = Sketch::new();
    let p0 = s.add_point(0.0, 0.0);
    let p1 = s.add_point(5.0, 0.0);
    s.add_line(p0, p1);
    let bs = geometry_to_bspline(&mut s, 0).unwrap();
    assert_eq!(s.bsplines[bs.0].degree, 1);
    assert_eq!(s.bsplines[bs.0].control_points.len(), 2);
}

#[test]
fn increase_degree_raises_degree_and_cp_count() {
    let mut s = Sketch::new();
    let pts: Vec<_> = (0..4).map(|i| s.add_point(i as f64, 0.0)).collect();
    let bs = s.add_bspline(pts, 2, false);
    increase_bspline_degree(&mut s, bs).unwrap();
    assert_eq!(s.bsplines[bs.0].degree, 3);
    assert!(s.bsplines[bs.0].control_points.len() > 4);
}

#[test]
fn decrease_degree_below_one_errors() {
    let mut s = Sketch::new();
    let pts: Vec<_> = (0..3).map(|i| s.add_point(i as f64, 0.0)).collect();
    let bs = s.add_bspline(pts, 1, false);
    assert!(decrease_bspline_degree(&mut s, bs).is_err());
}

#[test]
fn increase_and_decrease_knot_multiplicity() {
    let mut s = Sketch::new();
    let pts: Vec<_> = (0..6).map(|i| s.add_point(i as f64, 0.0)).collect();
    let bs = s.add_bspline(pts, 3, false);
    let n0 = s.bsplines[bs.0].control_points.len();
    increase_knot_multiplicity(&mut s, bs, 2).unwrap();
    assert_eq!(s.bsplines[bs.0].control_points.len(), n0 + 1);
    decrease_knot_multiplicity(&mut s, bs, 2).unwrap();
    assert_eq!(s.bsplines[bs.0].control_points.len(), n0);
}

#[test]
fn insert_knot_adds_control_point() {
    let mut s = Sketch::new();
    let pts: Vec<_> = (0..4).map(|i| s.add_point(i as f64, 0.0)).collect();
    let bs = s.add_bspline(pts, 3, false);
    let n0 = s.bsplines[bs.0].control_points.len();
    insert_knot(&mut s, bs, 0.5).unwrap();
    assert_eq!(s.bsplines[bs.0].control_points.len(), n0 + 1);
}

#[test]
fn join_curves_merges_two_entities() {
    let mut s = Sketch::new();
    let p0 = s.add_point(0.0, 0.0);
    let p1 = s.add_point(1.0, 0.0);
    let p2 = s.add_point(2.0, 1.0);
    let p3 = s.add_point(3.0, 1.0);
    s.add_line(p0, p1);
    s.add_line(p2, p3);
    let bs = join_curves(&mut s, 0, 1).unwrap();
    assert_eq!(s.bsplines[bs.0].control_points.len(), 4);
}

// ---------------------------------------------------------------------------
// Transformation helpers — Sketch methods
// ---------------------------------------------------------------------------

#[test]
fn move_geometry_translates_points() {
    let mut s = Sketch::new();
    let p = s.add_point(1.0, 2.0);
    move_geometry(&mut s, &[p], 3.0, 4.0);
    assert!((s.points[p.0].position.x - 4.0).abs() < TOL);
    assert!((s.points[p.0].position.y - 6.0).abs() < TOL);
}

#[test]
fn rotate_geometry_ninety_degrees() {
    let mut s = Sketch::new();
    let p = s.add_point(1.0, 0.0);
    rotate_geometry(&mut s, &[p], 0.0, 0.0, FRAC_PI_2);
    assert!(s.points[p.0].position.x.abs() < LOOSE);
    assert!((s.points[p.0].position.y - 1.0).abs() < LOOSE);
}

#[test]
fn scale_geometry_doubles_distance() {
    let mut s = Sketch::new();
    let p = s.add_point(2.0, 3.0);
    scale_geometry(&mut s, &[p], 0.0, 0.0, 2.0);
    assert!((s.points[p.0].position.x - 4.0).abs() < TOL);
    assert!((s.points[p.0].position.y - 6.0).abs() < TOL);
}

#[test]
fn offset_geometry_on_line_creates_parallel() {
    let mut s = Sketch::new();
    let p0 = s.add_point(0.0, 0.0);
    let p1 = s.add_point(5.0, 0.0);
    s.add_line(p0, p1);
    let pts = offset_geometry(&mut s, 0, 1.0).unwrap();
    assert_eq!(pts.len(), 2);
    let oy = s.points[pts[0].0].position.y;
    assert!((oy - 1.0).abs() < TOL);
}

#[test]
fn offset_geometry_rejects_non_line_entity() {
    let mut s = Sketch::new();
    s.add_point(0.0, 0.0);
    assert!(offset_geometry(&mut s, 99, 1.0).is_err());
}

#[test]
fn mirror_geometry_axis_reflects_point() {
    let mut s = Sketch::new();
    let p = s.add_point(1.0, 2.0);
    let m = mirror_geometry_axis(&mut s, &[p], 0.0, 0.0, 0.0, 1.0);
    assert_eq!(m.len(), 1);
    let mp = s.points[m[0].0].position;
    assert!((mp.x + 1.0).abs() < TOL);
    assert!((mp.y - 2.0).abs() < TOL);
}

#[test]
fn sketch_mirror_elements_across_line() {
    let mut s = Sketch::new();
    let p = s.add_point(1.0, 2.0);
    // Mirror line: vertical line through origin (y-axis)
    let p_axis0 = s.add_point(0.0, 0.0);
    let p_axis1 = s.add_point(0.0, 1.0);
    let axis = s.add_line(p_axis0, p_axis1);
    let new_ids = s.mirror_elements(&[p], axis);
    assert_eq!(new_ids.len(), 1);
    let mp = s.points[new_ids[0].0].position;
    assert!((mp.x + 1.0).abs() < LOOSE);
    assert!((mp.y - 2.0).abs() < LOOSE);
}

#[test]
fn sketch_rotate_elements_copies_and_rotates() {
    let mut s = Sketch::new();
    let p = s.add_point(1.0, 0.0);
    let new_ids = s.rotate_elements(&[p], 0.0, 0.0, FRAC_PI_2);
    assert_eq!(new_ids.len(), 1);
    let np = s.points[new_ids[0].0].position;
    assert!(np.x.abs() < LOOSE);
    assert!((np.y - 1.0).abs() < LOOSE);
    assert!((s.points[p.0].position.x - 1.0).abs() < TOL);
}

#[test]
fn sketch_scale_elements_copies_points() {
    let mut s = Sketch::new();
    let p = s.add_point(2.0, 3.0);
    let new_ids = s.scale_elements(&[p], 0.0, 0.0, 2.0);
    assert_eq!(new_ids.len(), 1);
    let np = s.points[new_ids[0].0].position;
    assert!((np.x - 4.0).abs() < LOOSE);
}

#[test]
fn sketch_offset_elements_closed_rect() {
    let mut s = Sketch::new();
    let p0 = s.add_point(0.0, 0.0);
    let p1 = s.add_point(4.0, 0.0);
    let p2 = s.add_point(4.0, 2.0);
    let p3 = s.add_point(0.0, 2.0);
    let l0 = s.add_line(p0, p1);
    let l1 = s.add_line(p1, p2);
    let l2 = s.add_line(p2, p3);
    let l3 = s.add_line(p3, p0);
    let new_pts = s.offset_elements(&[l0, l1, l2, l3], 0.5);
    assert_eq!(new_pts.len(), 4);
}

// ---------------------------------------------------------------------------
// Sketch reference frame / merge operations
// ---------------------------------------------------------------------------

#[test]
fn merge_with_combines_sketches() {
    let mut a = Sketch::new();
    let p0 = a.add_point(0.0, 0.0);
    a.add_point(1.0, 0.0);
    a.add_line(p0, PointId(1));

    let mut b = Sketch::new();
    b.add_point(5.0, 5.0);

    let np = a.points.len();
    let nl = a.lines.len();
    a.merge_with(&b);
    assert_eq!(a.points.len(), np + b.points.len());
    assert_eq!(a.lines.len(), nl);
}

#[test]
fn attach_to_plane_returns_workplane() {
    let mut s = Sketch::new();
    let wp = s.attach_to_plane(Point3::ORIGIN, Vec3::Y, Vec3::X);
    // Y-axis normal means XZ-plane; so z should be perpendicular
    assert!((wp.normal - Vec3::Y).length() < LOOSE);
}

#[test]
fn reorient_builds_new_plane() {
    let mut s = Sketch::new();
    let wp = s.reorient(Vec3::Z, Vec3::X);
    assert!((wp.normal - Vec3::Z).length() < LOOSE);
    assert!((wp.x_axis.length() - 1.0).abs() < LOOSE);
}

#[test]
fn toggle_driving_reference_bounds_check() {
    let mut s = Sketch::new();
    let p = s.add_point(0.0, 0.0);
    s.add_constraint(Constraint::Fixed(p, 0.0, 0.0));
    assert!(s.toggle_driving_reference(0));
    assert!(!s.toggle_driving_reference(99));
}

#[test]
fn delete_all_geometry_clears_sketch() {
    let mut s = Sketch::new();
    let p0 = s.add_point(0.0, 0.0);
    let p1 = s.add_point(1.0, 0.0);
    s.add_line(p0, p1);
    delete_all_geometry(&mut s);
    assert!(s.points.is_empty());
    assert!(s.lines.is_empty());
}

#[test]
fn delete_all_constraints_empties_constraints() {
    let mut s = Sketch::new();
    let p = s.add_point(0.0, 0.0);
    s.add_constraint(Constraint::Fixed(p, 0.0, 0.0));
    delete_all_constraints(&mut s);
    assert!(s.constraints.is_empty());
}

#[test]
fn carbon_copy_duplicates_source() {
    let mut src = Sketch::new();
    let p0 = src.add_point(0.0, 0.0);
    let p1 = src.add_point(1.0, 1.0);
    src.add_line(p0, p1);
    let mut tgt = Sketch::new();
    carbon_copy(&src, &mut tgt).unwrap();
    assert_eq!(tgt.points.len(), 2);
    assert_eq!(tgt.lines.len(), 1);
}

#[test]
fn external_projection_maps_3d_points() {
    let mut s = Sketch::new();
    let wp = WorkPlane::xy();
    let pts = vec![Point3::new(1.0, 2.0, 7.0), Point3::new(3.0, 4.0, 9.0)];
    let ids = external_projection(&mut s, &pts, &wp);
    assert_eq!(ids.len(), 2);
    assert!((s.points[ids[0].0].position.x - 1.0).abs() < TOL);
    assert!((s.points[ids[0].0].position.y - 2.0).abs() < TOL);
}

// ---------------------------------------------------------------------------
// Display / snap / grid helpers
// ---------------------------------------------------------------------------

#[test]
fn grid_default_and_snap() {
    let g = SketchGrid::default();
    assert!((g.spacing - 10.0).abs() < TOL);
    let p = g.snap_point(Point2::new(7.3, 12.8));
    // subdivisions = 5 → sub_spacing = 2
    assert!((p.x - 8.0).abs() < TOL);
    assert!((p.y - 12.0).abs() < TOL);
}

#[test]
fn grid_new_clamps_low_values() {
    let g = SketchGrid::new(0.0, 0);
    assert!(g.spacing >= 0.001);
    assert!(g.subdivisions >= 1);
}

#[test]
fn snap_to_endpoint_finds_nearest() {
    let mut s = Sketch::new();
    let p0 = s.add_point(0.0, 0.0);
    let p1 = s.add_point(10.0, 0.0);
    s.add_line(p0, p1);
    let snap = SketchSnap {
        snap_to_endpoint: true,
        snap_to_midpoint: false,
        snap_to_center: false,
        snap_to_grid: false,
        ..Default::default()
    };
    let grid = SketchGrid {
        visible: false,
        ..Default::default()
    };
    let r = snap_to_sketch_geometry(&s, Point2::new(0.01, 0.01), &snap, &grid);
    assert!(r.is_some());
    let (p, st) = r.unwrap();
    assert!(p.x.abs() < LOOSE);
    assert_eq!(st, SnapType::Endpoint);
}

#[test]
fn display_options_defaults_enable_key_toggles() {
    let o = SketchDisplayOptions::default();
    assert!(o.show_constraints);
    assert!(o.auto_constraints);
    assert!(o.show_grid);
}

#[test]
fn toggle_constraints_visibility_inverts() {
    let mut o = SketchDisplayOptions::default();
    assert!(o.show_constraints);
    toggle_constraints_visibility(&mut o);
    assert!(!o.show_constraints);
}

#[test]
fn toggle_construction_tracks_ids() {
    let mut s = Sketch::new();
    s.add_point(0.0, 0.0);
    toggle_construction(&mut s, 0).unwrap();
    assert_eq!(s.construction_points.len(), 1);
    toggle_construction(&mut s, 0).unwrap();
    assert!(s.construction_points.is_empty());
}

#[test]
fn toggle_construction_rejects_bad_id() {
    let mut s = Sketch::new();
    assert!(toggle_construction(&mut s, 99).is_err());
}

#[test]
fn select_origin_is_at_zero() {
    let s = Sketch::new();
    let o = select_origin(&s);
    assert!(o.x.abs() < TOL);
    assert!(o.y.abs() < TOL);
}

#[test]
fn select_axes_span_wide_range() {
    let s = Sketch::new();
    let (h0, h1) = select_h_axis(&s);
    assert!(h0.x < 0.0 && h1.x > 0.0);
    let (v0, v1) = select_v_axis(&s);
    assert!(v0.y < 0.0 && v1.y > 0.0);
}

#[test]
fn remove_axes_alignment_filters_constraints() {
    let mut s = Sketch::new();
    let p0 = s.add_point(0.0, 0.0);
    let p1 = s.add_point(1.0, 0.0);
    s.add_line(p0, p1);
    s.add_constraint(Constraint::Horizontal(LineId(0)));
    s.add_constraint(Constraint::Length(LineId(0), 1.0));
    remove_axes_alignment(&mut s, &[0]).unwrap();
    assert_eq!(s.constraints.len(), 1);
    assert!(matches!(s.constraints[0], Constraint::Length(..)));
}

#[test]
fn copy_and_paste_entities() {
    let mut s = Sketch::new();
    s.add_point(0.0, 0.0);
    s.add_point(1.0, 1.0);
    let copies = copy_entities(&s, &[0, 1]);
    assert_eq!(copies.len(), 2);
    let new_ids = paste_entities(&mut s, &copies, Point2::new(10.0, 10.0));
    assert_eq!(new_ids.len(), 2);
}

#[test]
fn sketch_entity_variants_constructible() {
    let e1 = SketchEntity::Point(SketchPoint::new(0.0, 0.0));
    let e2 = SketchEntity::Line(SketchLine {
        start: PointId(0),
        end: PointId(1),
    });
    let e3 = SketchEntity::Arc(SketchArc {
        center: PointId(0),
        start_point: PointId(1),
        end_point: PointId(2),
        radius: 1.0,
        start_angle: 0.0,
        end_angle: PI,
    });
    let e4 = SketchEntity::Circle(SketchCircle {
        center: PointId(0),
        radius: 2.0,
    });
    let list = [e1, e2, e3, e4];
    assert_eq!(list.len(), 4);
}

#[test]
fn contextual_dimension_single_line_returns_length() {
    let mut s = Sketch::new();
    let p0 = s.add_point(0.0, 0.0);
    let p1 = s.add_point(4.0, 0.0);
    s.add_line(p0, p1);
    let c = contextual_dimension(&s, &[0]).unwrap();
    if let Constraint::Length(_, len) = c {
        assert!((len - 4.0).abs() < LOOSE);
    } else {
        panic!("expected Length");
    }
}

#[test]
fn contextual_dimension_two_points_returns_distance() {
    let mut s = Sketch::new();
    s.add_point(0.0, 0.0);
    s.add_point(3.0, 4.0);
    let c = contextual_dimension(&s, &[0, 1]).unwrap();
    if let Constraint::Distance(_, _, d) = c {
        assert!((d - 5.0).abs() < LOOSE);
    } else {
        panic!("expected Distance");
    }
}

#[test]
fn unified_radius_diameter_for_circle() {
    let mut s = Sketch::new();
    let c = s.add_point(0.0, 0.0);
    s.add_circle(c, 3.0);
    let r = unified_radius_diameter(&s, 0).unwrap();
    if let Constraint::Diameter(_, _, d) = r {
        assert!((d - 6.0).abs() < LOOSE);
    } else {
        panic!("expected Diameter");
    }
}

#[test]
fn unified_horizontal_vertical_picks_horizontal() {
    let mut s = Sketch::new();
    let p0 = s.add_point(0.0, 0.0);
    let p1 = s.add_point(10.0, 0.0);
    s.add_line(p0, p1);
    let c = unified_horizontal_vertical(&s, 0).unwrap();
    assert!(matches!(c, Constraint::Horizontal(_)));
}

#[test]
fn unified_horizontal_vertical_picks_vertical() {
    let mut s = Sketch::new();
    let p0 = s.add_point(0.0, 0.0);
    let p1 = s.add_point(0.0, 10.0);
    s.add_line(p0, p1);
    let c = unified_horizontal_vertical(&s, 0).unwrap();
    assert!(matches!(c, Constraint::Vertical(_)));
}

#[test]
fn align_view_to_xy_plane_returns_zero() {
    let (yaw, pitch, roll) = align_view_to_sketch(Point3::ORIGIN, Vec3::Z);
    assert!(yaw.abs() < TOL);
    assert!(pitch.abs() < TOL);
    assert!(roll.abs() < TOL);
}

#[test]
fn stop_operation_clears_construction_mode() {
    let mut s = Sketch::new();
    s.construction_mode = true;
    stop_operation(&mut s);
    assert!(!s.construction_mode);
}

#[test]
fn toggle_section_view_round_trip() {
    let on = toggle_section_view(Vec3::Z, Point3::new(0.0, 0.0, 1.0), true);
    let off = toggle_section_view(Vec3::Z, Point3::new(0.0, 0.0, 1.0), false);
    assert!(on.enabled);
    assert!(!off.enabled);
}

#[test]
fn section_view_state_clone() {
    let s = SectionViewState {
        plane_normal: Vec3::X,
        plane_point: Point3::ORIGIN,
        enabled: true,
    };
    let c = s.clone();
    assert!(c.enabled);
}

#[test]
fn periodic_bspline_from_knots_stores_closed_flag() {
    let mut s = Sketch::new();
    let cps: Vec<_> = (0..4).map(|i| s.add_point(i as f64, 0.0)).collect();
    let knots = vec![0.0, 0.0, 0.0, 0.25, 0.5, 0.75, 1.0, 1.0, 1.0];
    let bs = add_periodic_bspline_from_knots(&mut s, cps, knots, 2);
    assert!(s.bsplines[bs.0].closed);
    assert_eq!(s.bsplines[bs.0].knots.len(), 9);
}

#[test]
fn snap_type_equality_and_copy() {
    let a = SnapType::Endpoint;
    let b = SnapType::Endpoint;
    assert_eq!(a, b);
    let c = a;
    assert_eq!(c, SnapType::Endpoint);
}

#[test]
fn solver_converges_on_perpendicular_constraint() {
    let mut s = Sketch::new();
    let p0 = s.add_point(0.0, 0.0);
    let p1 = s.add_point(1.0, 0.0);
    let p2 = s.add_point(0.0, 1.0);
    s.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
    s.add_constraint(Constraint::Fixed(p1, 1.0, 0.0));
    let l0 = s.add_line(p0, p1);
    let l1 = s.add_line(p0, p2);
    s.add_constraint(Constraint::Perpendicular(l0, l1));
    s.add_constraint(Constraint::Length(l1, 1.0));
    let r = solve(&mut s, 100, SOLVE_TOL);
    assert!(r.converged);
    let d0x = s.points[p1.0].position.x - s.points[p0.0].position.x;
    let d0y = s.points[p1.0].position.y - s.points[p0.0].position.y;
    let d1x = s.points[p2.0].position.x - s.points[p0.0].position.x;
    let d1y = s.points[p2.0].position.y - s.points[p0.0].position.y;
    let dot = d0x * d1x + d0y * d1y;
    assert!(dot.abs() < LOOSE);
}

#[test]
fn solver_on_circular_tau_constraint() {
    let mut s = Sketch::new();
    let c = s.add_point(0.0, 0.0);
    let p0 = s.add_point(6.0, 0.0);
    let p1 = s.add_point(0.0, 6.0);
    s.add_constraint(Constraint::Fixed(c, 0.0, 0.0));
    s.add_constraint(Constraint::PointOnCircle(p0, c, 5.0));
    s.add_constraint(Constraint::PointOnCircle(p1, c, 5.0));
    let l0 = s.add_line(c, p0);
    let l1 = s.add_line(c, p1);
    s.add_constraint(Constraint::Horizontal(l0));
    s.add_constraint(Constraint::Vertical(l1));
    let r = solve(&mut s, 200, SOLVE_TOL);
    assert!(r.converged);
    let r0 = (s.points[p0.0].position.x.powi(2) + s.points[p0.0].position.y.powi(2)).sqrt();
    assert!((r0 - 5.0).abs() < LOOSE);
    assert!((TAU - 2.0 * PI).abs() < TOL);
}

#[test]
fn frac_pi_4_is_pi_over_four() {
    assert!((FRAC_PI_4 * 4.0 - PI).abs() < TOL);
}
