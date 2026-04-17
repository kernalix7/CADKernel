//! Comprehensive integration tests for the `cadkernel-geometry` crate.
//!
//! Covers the public API surface: curves (Line, LineSegment, Circle, Arc,
//! Ellipse, NurbsCurve), surfaces (Plane, Cylinder, Sphere, Cone, Torus,
//! NurbsSurface), tessellation, BVH/Aabb, intersection, and 2D offset.

use cadkernel_geometry::prelude::*;
use cadkernel_geometry::{
    Aabb, Bvh, Curve, LevelOfDetail, Surface, adaptive_tessellate_curve,
    adaptive_tessellate_surface, intersect_curves, offset_polygon_2d,
    offset_polygon_2d_checked, offset_polyline_2d,
};
use cadkernel_geometry::intersect::plane_plane::intersect_plane_plane;
use cadkernel_geometry::intersect::plane_sphere::intersect_plane_sphere;
use cadkernel_geometry::intersect::types::SsiResult;
use cadkernel_math::{BoundingBox, Point2, Point3, Vec3};
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

const TOL: f64 = 1e-10;
const LOOSE_TOL: f64 = 1e-6;

// ---------------------------------------------------------------------------
// Line
// ---------------------------------------------------------------------------

#[test]
fn line_point_at_origin() {
    let line = Line::new(Point3::ORIGIN, Vec3::X);
    assert!(line.point_at(0.0).approx_eq(Point3::ORIGIN));
    assert!(line.point_at(5.0).approx_eq(Point3::new(5.0, 0.0, 0.0)));
    assert!(line.point_at(-3.0).approx_eq(Point3::new(-3.0, 0.0, 0.0)));
}

#[test]
fn line_tangent_is_constant_direction() {
    let dir = Vec3::new(1.0, 2.0, 3.0);
    let line = Line::new(Point3::ORIGIN, dir);
    let t0 = line.tangent_at(0.0);
    let t1 = line.tangent_at(100.0);
    assert!((t0 - dir).length() < TOL);
    assert!((t1 - dir).length() < TOL);
}

#[test]
fn line_domain_is_infinite() {
    let line = Line::new(Point3::ORIGIN, Vec3::X);
    let (lo, hi) = line.domain();
    assert!(lo == f64::NEG_INFINITY);
    assert!(hi == f64::INFINITY);
}

#[test]
fn line_length_is_infinite() {
    let line = Line::new(Point3::ORIGIN, Vec3::X);
    assert!(line.length().is_infinite());
}

#[test]
fn line_not_closed() {
    assert!(!Line::new(Point3::ORIGIN, Vec3::X).is_closed());
}

#[test]
fn line_project_point_analytical() {
    let line = Line::new(Point3::ORIGIN, Vec3::X);
    let (t, closest) = line.project_point(Point3::new(3.0, 5.0, 0.0));
    assert!((t - 3.0).abs() < TOL);
    assert!(closest.approx_eq(Point3::new(3.0, 0.0, 0.0)));
}

#[test]
fn line_project_zero_direction_returns_origin() {
    let line = Line::new(Point3::new(1.0, 2.0, 3.0), Vec3::ZERO);
    let (t, closest) = line.project_point(Point3::new(10.0, 10.0, 10.0));
    assert_eq!(t, 0.0);
    assert!(closest.approx_eq(Point3::new(1.0, 2.0, 3.0)));
}

#[test]
fn line_bounding_box_finite_fallback() {
    let line = Line::new(Point3::ORIGIN, Vec3::X);
    let bb = line.bounding_box();
    assert!(bb.min.x <= 0.0);
    assert!(bb.max.x >= 0.0);
}

// ---------------------------------------------------------------------------
// LineSegment
// ---------------------------------------------------------------------------

#[test]
fn line_segment_midpoint() {
    let seg = LineSegment::new(Point3::ORIGIN, Point3::new(4.0, 0.0, 0.0));
    assert!(seg.point_at(0.5).approx_eq(Point3::new(2.0, 0.0, 0.0)));
}

#[test]
fn line_segment_length_3_4_5() {
    let seg = LineSegment::new(Point3::ORIGIN, Point3::new(3.0, 4.0, 0.0));
    assert!((seg.length() - 5.0).abs() < TOL);
}

#[test]
fn line_segment_domain_is_unit() {
    let seg = LineSegment::new(Point3::ORIGIN, Point3::new(1.0, 0.0, 0.0));
    assert_eq!(seg.domain(), (0.0, 1.0));
}

#[test]
fn line_segment_endpoints() {
    let seg = LineSegment::new(Point3::new(1.0, 2.0, 3.0), Point3::new(4.0, 5.0, 6.0));
    assert!(seg.point_at(0.0).approx_eq(Point3::new(1.0, 2.0, 3.0)));
    assert!(seg.point_at(1.0).approx_eq(Point3::new(4.0, 5.0, 6.0)));
}

#[test]
fn line_segment_tangent_is_end_minus_start() {
    let seg = LineSegment::new(Point3::ORIGIN, Point3::new(2.0, 0.0, 0.0));
    let t = seg.tangent_at(0.5);
    assert!(t.approx_eq(Vec3::new(2.0, 0.0, 0.0)));
}

#[test]
fn line_segment_not_closed() {
    let seg = LineSegment::new(Point3::ORIGIN, Point3::new(1.0, 0.0, 0.0));
    assert!(!seg.is_closed());
}

#[test]
fn line_segment_bounding_box() {
    let seg = LineSegment::new(Point3::new(-1.0, -2.0, 0.0), Point3::new(3.0, 4.0, 5.0));
    let bb = seg.bounding_box();
    assert!(bb.min.x <= -1.0);
    assert!(bb.max.x >= 3.0);
    assert!(bb.min.y <= -2.0);
    assert!(bb.max.y >= 4.0);
    assert!(bb.max.z >= 5.0);
}

#[test]
fn line_segment_project_uses_default_sampling() {
    let seg = LineSegment::new(Point3::ORIGIN, Point3::new(10.0, 0.0, 0.0));
    let (t, p) = seg.project_point(Point3::new(5.0, 3.0, 0.0));
    assert!((t - 0.5).abs() < LOOSE_TOL);
    assert!(p.approx_eq(Point3::new(5.0, 0.0, 0.0)));
}

// ---------------------------------------------------------------------------
// Circle
// ---------------------------------------------------------------------------

#[test]
fn circle_new_zero_normal_errors() {
    assert!(Circle::new(Point3::ORIGIN, Vec3::ZERO, 1.0).is_err());
}

#[test]
fn circle_xy_defaults() {
    let c = Circle::xy(Point3::ORIGIN, 2.0);
    assert!(c.point_at(0.0).approx_eq(Point3::new(2.0, 0.0, 0.0)));
    assert!(c.point_at(FRAC_PI_2).approx_eq(Point3::new(0.0, 2.0, 0.0)));
    assert!(c.x_axis().approx_eq(Vec3::X));
    assert!(c.y_axis().approx_eq(Vec3::Y));
}

#[test]
fn circle_length_is_circumference() {
    let c = Circle::xy(Point3::ORIGIN, 3.0);
    assert!((c.length() - TAU * 3.0).abs() < TOL);
}

#[test]
fn circle_is_closed() {
    assert!(Circle::xy(Point3::ORIGIN, 1.0).is_closed());
}

#[test]
fn circle_domain_is_tau() {
    let c = Circle::xy(Point3::ORIGIN, 1.0);
    assert_eq!(c.domain(), (0.0, TAU));
}

#[test]
fn circle_tangent_perpendicular_to_radius() {
    let c = Circle::xy(Point3::ORIGIN, 2.0);
    let tan = c.tangent_at(0.0);
    assert!((tan - Vec3::new(0.0, 2.0, 0.0)).length() < TOL);
}

#[test]
fn circle_new_custom_normal() {
    let c = Circle::new(Point3::ORIGIN, Vec3::Y, 1.0).unwrap();
    assert!((c.normal - Vec3::Y).length() < TOL);
    let p = c.point_at(0.0);
    let on_plane = (p - Point3::ORIGIN).dot(Vec3::Y);
    assert!(on_plane.abs() < TOL);
}

// ---------------------------------------------------------------------------
// Arc
// ---------------------------------------------------------------------------

#[test]
fn arc_xy_endpoints() {
    let arc = Arc::xy(Point3::ORIGIN, 1.0, 0.0, PI);
    assert!(arc.point_at(0.0).approx_eq(Point3::new(1.0, 0.0, 0.0)));
    assert!(arc.point_at(1.0).approx_eq(Point3::new(-1.0, 0.0, 0.0)));
}

#[test]
fn arc_quarter_length() {
    let arc = Arc::xy(Point3::ORIGIN, 1.0, 0.0, FRAC_PI_2);
    assert!((arc.length() - FRAC_PI_2).abs() < TOL);
}

#[test]
fn arc_not_closed() {
    let arc = Arc::xy(Point3::ORIGIN, 1.0, 0.0, FRAC_PI_2);
    assert!(!arc.is_closed());
}

#[test]
fn arc_domain_is_unit() {
    let arc = Arc::xy(Point3::ORIGIN, 1.0, 0.0, FRAC_PI_2);
    assert_eq!(arc.domain(), (0.0, 1.0));
}

#[test]
fn arc_axes() {
    let arc = Arc::xy(Point3::ORIGIN, 1.0, 0.0, FRAC_PI_2);
    assert!(arc.x_axis().approx_eq(Vec3::X));
    assert!(arc.y_axis().approx_eq(Vec3::Y));
}

// ---------------------------------------------------------------------------
// Ellipse
// ---------------------------------------------------------------------------

#[test]
fn ellipse_at_zero_is_major_axis_endpoint() {
    let e = Ellipse::new(Point3::ORIGIN, Vec3::Z, Vec3::X, 3.0, 2.0);
    assert!(e.point_at(0.0).approx_eq(Point3::new(3.0, 0.0, 0.0)));
}

#[test]
fn ellipse_at_half_pi_is_minor_axis_endpoint() {
    let e = Ellipse::new(Point3::ORIGIN, Vec3::Z, Vec3::X, 3.0, 2.0);
    let p = e.point_at(FRAC_PI_2);
    assert!(p.x.abs() < TOL);
    assert!((p.y - 2.0).abs() < TOL);
}

#[test]
fn ellipse_is_closed() {
    let e = Ellipse::new(Point3::ORIGIN, Vec3::Z, Vec3::X, 2.0, 1.0);
    assert!(e.is_closed());
    let a = e.point_at(0.0);
    let b = e.point_at(TAU);
    assert!(a.approx_eq(b));
}

#[test]
fn ellipse_circle_case_length_matches_circle() {
    let e = Ellipse::new(Point3::ORIGIN, Vec3::Z, Vec3::X, 1.0, 1.0);
    assert!((e.length() - TAU).abs() < 1e-3);
}

#[test]
fn ellipse_length_between_axes() {
    let e = Ellipse::new(Point3::ORIGIN, Vec3::Z, Vec3::X, 2.0, 1.0);
    let len = e.length();
    assert!(len > TAU);
    assert!(len < TAU * 2.0);
}

#[test]
fn ellipse_domain_is_tau() {
    let e = Ellipse::new(Point3::ORIGIN, Vec3::Z, Vec3::X, 1.0, 1.0);
    assert_eq!(e.domain(), (0.0, TAU));
}

// ---------------------------------------------------------------------------
// NurbsCurve
// ---------------------------------------------------------------------------

#[test]
fn nurbs_bezier_line_evaluates_linearly() {
    let c = NurbsCurve::bezier(vec![
        Point3::ORIGIN,
        Point3::new(1.0, 0.0, 0.0),
    ])
    .unwrap();
    assert!(c.point_at(0.5).approx_eq(Point3::new(0.5, 0.0, 0.0)));
}

#[test]
fn nurbs_bezier_degree_matches_cp_minus_one() {
    let c = NurbsCurve::bezier(vec![
        Point3::ORIGIN,
        Point3::new(1.0, 1.0, 0.0),
        Point3::new(2.0, 0.0, 0.0),
    ])
    .unwrap();
    assert_eq!(c.degree(), 2);
}

#[test]
fn nurbs_control_point_count() {
    let c = NurbsCurve::bezier(vec![
        Point3::ORIGIN,
        Point3::new(1.0, 1.0, 0.0),
        Point3::new(2.0, 0.0, 0.0),
    ])
    .unwrap();
    assert_eq!(c.control_point_count(), 3);
}

#[test]
fn nurbs_knots_accessor() {
    let c = NurbsCurve::bezier(vec![
        Point3::ORIGIN,
        Point3::new(1.0, 0.0, 0.0),
    ])
    .unwrap();
    assert_eq!(c.knots().len(), 4);
}

#[test]
fn nurbs_weights_accessor() {
    let c = NurbsCurve::bezier(vec![
        Point3::ORIGIN,
        Point3::new(1.0, 0.0, 0.0),
    ])
    .unwrap();
    assert_eq!(c.weights().len(), 2);
    for &w in c.weights() {
        assert!((w - 1.0).abs() < TOL);
    }
}

#[test]
fn nurbs_insert_knot_preserves_shape() {
    let c = NurbsCurve::bezier(vec![
        Point3::ORIGIN,
        Point3::new(1.0, 1.0, 0.0),
        Point3::new(2.0, 0.0, 0.0),
    ])
    .unwrap();
    let inserted = c.insert_knot(0.5).unwrap();
    for i in 0..=10 {
        let t = i as f64 / 10.0;
        let a = c.point_at(t);
        let b = inserted.point_at(t);
        assert!(a.distance_to(b) < 1e-10);
    }
}

#[test]
fn nurbs_insert_knot_adds_cp() {
    let c = NurbsCurve::bezier(vec![
        Point3::ORIGIN,
        Point3::new(1.0, 1.0, 0.0),
        Point3::new(2.0, 0.0, 0.0),
    ])
    .unwrap();
    let inserted = c.insert_knot(0.5).unwrap();
    assert_eq!(inserted.control_point_count(), 4);
}

#[test]
fn nurbs_reverse_swaps_endpoints() {
    let c = NurbsCurve::bezier(vec![
        Point3::ORIGIN,
        Point3::new(1.0, 2.0, 0.0),
        Point3::new(3.0, 1.0, 0.0),
    ])
    .unwrap();
    let rev = c.reverse();
    let (a, b) = c.domain();
    for i in 0..=10 {
        let t = a + (b - a) * i as f64 / 10.0;
        let orig = c.point_at(t);
        let reversed = rev.point_at(a + b - t);
        assert!(orig.distance_to(reversed) < 1e-10);
    }
}

#[test]
fn nurbs_bounding_box_contains_control_points() {
    let c = NurbsCurve::bezier(vec![
        Point3::ORIGIN,
        Point3::new(1.0, 3.0, 0.0),
        Point3::new(2.0, 0.0, 0.0),
    ])
    .unwrap();
    let bb = Curve::bounding_box(&c);
    assert!(bb.min.x <= 0.0);
    assert!(bb.max.x >= 2.0);
    assert!(bb.max.y >= 2.0);
}

#[test]
fn nurbs_second_derivative_quadratic_bezier() {
    // B''(t) = 2*(P0 - 2*P1 + P2) = 2*(0 - (1,2,0) + (1,0,0)) = (0,-4,0)
    let c = NurbsCurve::bezier(vec![
        Point3::ORIGIN,
        Point3::new(0.5, 1.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
    ])
    .unwrap();
    let d2 = c.second_derivative_at(0.5);
    assert!(d2.x.abs() < 1e-10);
    assert!((d2.y + 4.0).abs() < 1e-10);
}

#[test]
fn nurbs_curvature_straight_quadratic_is_zero() {
    // Use a quadratic Bezier that is still geometrically straight so the
    // analytical second derivative is well-defined without hitting a degree-1
    // boundary index in rational_derivatives.
    let c = NurbsCurve::bezier(vec![
        Point3::ORIGIN,
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(2.0, 0.0, 0.0),
    ])
    .unwrap();
    assert!(c.curvature_at(0.5).abs() < 1e-9);
}

// ---------------------------------------------------------------------------
// Plane
// ---------------------------------------------------------------------------

#[test]
fn plane_xy_normal() {
    let p = Plane::xy().unwrap();
    assert!(p.normal().approx_eq(Vec3::Z));
}

#[test]
fn plane_xz_normal_is_minus_y() {
    let p = Plane::xz().unwrap();
    assert!((p.normal() - (-Vec3::Y)).length() < TOL);
}

#[test]
fn plane_yz_normal_is_x() {
    let p = Plane::yz().unwrap();
    assert!(p.normal().approx_eq(Vec3::X));
}

#[test]
fn plane_new_parallel_axes_errors() {
    let err = Plane::new(Point3::ORIGIN, Vec3::X, Vec3::X);
    assert!(err.is_err());
}

#[test]
fn plane_from_three_points_builds_xy() {
    let p = Plane::from_three_points(
        Point3::ORIGIN,
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
    )
    .unwrap();
    assert!(p.normal().approx_eq(Vec3::Z));
}

#[test]
fn plane_signed_distance_sign() {
    let p = Plane::xy().unwrap();
    assert!((p.signed_distance(Point3::new(0.0, 0.0, 2.0)) - 2.0).abs() < TOL);
    assert!((p.signed_distance(Point3::new(0.0, 0.0, -3.0)) + 3.0).abs() < TOL);
    assert!(p.signed_distance(Point3::new(7.0, -4.0, 0.0)).abs() < TOL);
}

#[test]
fn plane_distance_is_absolute() {
    let p = Plane::xy().unwrap();
    assert!((p.distance(Point3::new(0.0, 0.0, -3.0)) - 3.0).abs() < TOL);
}

#[test]
fn plane_project_point_drops_normal_component() {
    let p = Plane::xy().unwrap();
    let proj = Plane::project_point(&p, Point3::new(2.0, 3.0, 7.0));
    assert!(proj.approx_eq(Point3::new(2.0, 3.0, 0.0)));
}

#[test]
fn plane_is_above() {
    let p = Plane::xy().unwrap();
    assert!(p.is_above(Point3::new(0.0, 0.0, 1.0)));
    assert!(!p.is_above(Point3::new(0.0, 0.0, -1.0)));
    assert!(!p.is_above(Point3::new(0.0, 0.0, 0.0)));
}

#[test]
fn plane_contains_point() {
    let p = Plane::xy().unwrap();
    assert!(p.contains_point(Point3::new(3.0, 4.0, 0.0)));
    assert!(!p.contains_point(Point3::new(3.0, 4.0, 0.5)));
}

#[test]
fn plane_surface_point_at_and_normal() {
    let p = Plane::xy().unwrap();
    assert!(Surface::point_at(&p, 2.0, 3.0).approx_eq(Point3::new(2.0, 3.0, 0.0)));
    assert!(p.normal_at(0.0, 0.0).approx_eq(Vec3::Z));
}

#[test]
fn plane_surface_domain_infinite() {
    let p = Plane::xy().unwrap();
    let (u_lo, u_hi) = p.domain_u();
    let (v_lo, v_hi) = p.domain_v();
    assert!(u_lo.is_infinite() && u_hi.is_infinite());
    assert!(v_lo.is_infinite() && v_hi.is_infinite());
}

#[test]
fn plane_surface_du_dv() {
    let p = Plane::xy().unwrap();
    let du = p.du(0.0, 0.0);
    let dv = p.dv(0.0, 0.0);
    assert!((du - Vec3::X).length() < 1e-4);
    assert!((dv - Vec3::Y).length() < 1e-4);
}

#[test]
fn plane_bounding_box_finite_fallback() {
    let p = Plane::xy().unwrap();
    let bb = Surface::bounding_box(&p);
    assert!(bb.min.x < 0.0);
    assert!(bb.max.x > 0.0);
}

// ---------------------------------------------------------------------------
// Cylinder
// ---------------------------------------------------------------------------

#[test]
fn cylinder_z_axis_defaults() {
    let c = Cylinder::z_axis(1.0, 5.0);
    assert!(c.base_center.approx_eq(Point3::ORIGIN));
    assert!(c.axis.approx_eq(Vec3::Z));
}

#[test]
fn cylinder_point_at_base() {
    let c = Cylinder::z_axis(1.0, 5.0);
    assert!(c.point_at(0.0, 0.0).approx_eq(Point3::new(1.0, 0.0, 0.0)));
}

#[test]
fn cylinder_point_at_top() {
    let c = Cylinder::z_axis(1.0, 5.0);
    assert!(c.point_at(FRAC_PI_2, 5.0).approx_eq(Point3::new(0.0, 1.0, 5.0)));
}

#[test]
fn cylinder_normal_is_unit() {
    let c = Cylinder::z_axis(1.0, 5.0);
    let n = c.normal_at(0.7, 2.0);
    assert!((n.length() - 1.0).abs() < TOL);
}

#[test]
fn cylinder_domain_u_is_tau_v_is_height() {
    let c = Cylinder::z_axis(1.0, 4.0);
    assert_eq!(c.domain_u(), (0.0, TAU));
    assert_eq!(c.domain_v(), (0.0, 4.0));
}

#[test]
fn cylinder_new_validates_non_zero_axis() {
    assert!(Cylinder::new(Point3::ORIGIN, Vec3::ZERO, 1.0, 5.0).is_err());
}

// ---------------------------------------------------------------------------
// Sphere
// ---------------------------------------------------------------------------

#[test]
fn sphere_new_rejects_zero_radius() {
    assert!(Sphere::new(Point3::ORIGIN, 0.0).is_err());
}

#[test]
fn sphere_new_rejects_negative_radius() {
    assert!(Sphere::new(Point3::ORIGIN, -1.0).is_err());
}

#[test]
fn sphere_equator_point() {
    let s = Sphere::new(Point3::ORIGIN, 2.0).unwrap();
    assert!(s.point_at(0.0, 0.0).approx_eq(Point3::new(2.0, 0.0, 0.0)));
}

#[test]
fn sphere_north_pole() {
    let s = Sphere::new(Point3::ORIGIN, 1.0).unwrap();
    assert!(s.point_at(0.0, FRAC_PI_2).approx_eq(Point3::new(0.0, 0.0, 1.0)));
}

#[test]
fn sphere_south_pole() {
    let s = Sphere::new(Point3::ORIGIN, 1.0).unwrap();
    assert!(s.point_at(0.0, -FRAC_PI_2).approx_eq(Point3::new(0.0, 0.0, -1.0)));
}

#[test]
fn sphere_normal_is_unit() {
    let s = Sphere::new(Point3::ORIGIN, 3.0).unwrap();
    let n = s.normal_at(1.2, 0.4);
    assert!((n.length() - 1.0).abs() < TOL);
}

#[test]
fn sphere_domains() {
    let s = Sphere::new(Point3::ORIGIN, 1.0).unwrap();
    assert_eq!(s.domain_u(), (0.0, TAU));
    let (v_lo, v_hi) = s.domain_v();
    assert!((v_lo + FRAC_PI_2).abs() < TOL);
    assert!((v_hi - FRAC_PI_2).abs() < TOL);
}

// ---------------------------------------------------------------------------
// Cone
// ---------------------------------------------------------------------------

#[test]
fn cone_apex_is_v_zero() {
    let c = Cone::new(Point3::ORIGIN, Vec3::Z, FRAC_PI_4).unwrap();
    assert!(c.point_at(0.0, 0.0).approx_eq(Point3::ORIGIN));
}

#[test]
fn cone_radius_at_v1() {
    let c = Cone::new(Point3::ORIGIN, Vec3::Z, FRAC_PI_4).unwrap();
    let p = c.point_at(0.0, 1.0);
    assert!((p.z - 1.0).abs() < TOL);
    let r = (p.x * p.x + p.y * p.y).sqrt();
    assert!((r - 1.0).abs() < LOOSE_TOL);
}

#[test]
fn cone_invalid_half_angle() {
    assert!(Cone::new(Point3::ORIGIN, Vec3::Z, 0.0).is_err());
    assert!(Cone::new(Point3::ORIGIN, Vec3::Z, FRAC_PI_2).is_err());
    assert!(Cone::new(Point3::ORIGIN, Vec3::Z, -0.1).is_err());
}

#[test]
fn cone_domains() {
    let c = Cone::new(Point3::ORIGIN, Vec3::Z, FRAC_PI_4).unwrap();
    assert_eq!(c.domain_u(), (0.0, TAU));
    assert_eq!(c.domain_v(), (0.0, 1.0));
}

// ---------------------------------------------------------------------------
// Torus
// ---------------------------------------------------------------------------

#[test]
fn torus_outer_equator() {
    let t = Torus::new(Point3::ORIGIN, Vec3::Z, 3.0, 1.0).unwrap();
    let p = t.point_at(0.0, 0.0);
    let r = (p.x * p.x + p.y * p.y).sqrt();
    assert!((r - 4.0).abs() < TOL);
    assert!(p.z.abs() < TOL);
}

#[test]
fn torus_inner_equator() {
    let t = Torus::new(Point3::ORIGIN, Vec3::Z, 3.0, 1.0).unwrap();
    let p = t.point_at(0.0, PI);
    let r = (p.x * p.x + p.y * p.y).sqrt();
    assert!((r - 2.0).abs() < TOL);
}

#[test]
fn torus_top_of_tube() {
    let t = Torus::new(Point3::ORIGIN, Vec3::Z, 3.0, 1.0).unwrap();
    let p = t.point_at(0.0, FRAC_PI_2);
    assert!((p.z - 1.0).abs() < TOL);
}

#[test]
fn torus_invalid_radii() {
    assert!(Torus::new(Point3::ORIGIN, Vec3::Z, 0.0, 1.0).is_err());
    assert!(Torus::new(Point3::ORIGIN, Vec3::Z, 1.0, 0.0).is_err());
    assert!(Torus::new(Point3::ORIGIN, Vec3::Z, -1.0, 1.0).is_err());
}

#[test]
fn torus_domains() {
    let t = Torus::new(Point3::ORIGIN, Vec3::Z, 3.0, 1.0).unwrap();
    assert_eq!(t.domain_u(), (0.0, TAU));
    assert_eq!(t.domain_v(), (0.0, TAU));
}

#[test]
fn torus_periodic() {
    let t = Torus::new(Point3::ORIGIN, Vec3::Z, 5.0, 2.0).unwrap();
    assert!(t.point_at(0.0, 0.0).approx_eq(t.point_at(TAU, TAU)));
}

// ---------------------------------------------------------------------------
// NurbsSurface (basic smoke tests)
// ---------------------------------------------------------------------------

#[test]
fn nurbs_surface_construct_bilinear_patch() {
    let s = NurbsSurface::new(
        1,
        1,
        2,
        2,
        vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
        ],
        vec![1.0, 1.0, 1.0, 1.0],
        vec![0.0, 0.0, 1.0, 1.0],
        vec![0.0, 0.0, 1.0, 1.0],
    );
    assert!(s.is_ok());
}

#[test]
fn nurbs_surface_mismatched_cp_len_errors() {
    let s = NurbsSurface::new(
        1,
        1,
        2,
        2,
        vec![Point3::ORIGIN, Point3::new(1.0, 0.0, 0.0)],
        vec![1.0, 1.0, 1.0, 1.0],
        vec![0.0, 0.0, 1.0, 1.0],
        vec![0.0, 0.0, 1.0, 1.0],
    );
    assert!(s.is_err());
}

#[test]
fn nurbs_surface_corner_points_match_cps() {
    let s = NurbsSurface::new(
        1,
        1,
        2,
        2,
        vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
        ],
        vec![1.0, 1.0, 1.0, 1.0],
        vec![0.0, 0.0, 1.0, 1.0],
        vec![0.0, 0.0, 1.0, 1.0],
    )
    .unwrap();
    assert!(s.point_at(0.0, 0.0).approx_eq(Point3::new(0.0, 0.0, 0.0)));
    assert!(s.point_at(1.0, 0.0).approx_eq(Point3::new(1.0, 0.0, 0.0)));
    assert!(s.point_at(0.0, 1.0).approx_eq(Point3::new(0.0, 1.0, 0.0)));
    assert!(s.point_at(1.0, 1.0).approx_eq(Point3::new(1.0, 1.0, 0.0)));
}

// ---------------------------------------------------------------------------
// TessellationOptions
// ---------------------------------------------------------------------------

#[test]
fn tess_options_default_values() {
    let o = TessellationOptions::default();
    assert!(o.chord_tolerance > 0.0);
    assert!(o.angle_tolerance > 0.0);
    assert!(o.min_segments >= 2);
    assert!(o.max_depth >= 1);
}

#[test]
fn tess_options_from_lod_coarse_vs_fine() {
    let coarse = TessellationOptions::from_lod(LevelOfDetail::Coarse);
    let medium = TessellationOptions::from_lod(LevelOfDetail::Medium);
    let fine = TessellationOptions::from_lod(LevelOfDetail::Fine);
    assert!(coarse.chord_tolerance > medium.chord_tolerance);
    assert!(medium.chord_tolerance > fine.chord_tolerance);
    assert!(coarse.min_segments <= fine.min_segments);
    assert!(coarse.max_depth <= fine.max_depth);
}

#[test]
fn tess_options_from_lod_medium_is_default() {
    let medium = TessellationOptions::from_lod(LevelOfDetail::Medium);
    let default = TessellationOptions::default();
    assert!((medium.chord_tolerance - default.chord_tolerance).abs() < TOL);
    assert!((medium.angle_tolerance - default.angle_tolerance).abs() < TOL);
    assert_eq!(medium.min_segments, default.min_segments);
    assert_eq!(medium.max_depth, default.max_depth);
}

// ---------------------------------------------------------------------------
// Tessellation: curves
// ---------------------------------------------------------------------------

#[test]
fn tess_straight_line_min_segments() {
    let opts = TessellationOptions {
        chord_tolerance: 0.01,
        angle_tolerance: 0.1,
        min_segments: 5,
        max_depth: 4,
    };
    let pts = adaptive_tessellate_curve(
        |t| Point3::new(t, 0.0, 0.0),
        |_| Vec3::X,
        0.0,
        1.0,
        &opts,
    );
    assert!(pts.len() > opts.min_segments);
    assert!(pts.first().unwrap().approx_eq(Point3::ORIGIN));
    assert!(pts.last().unwrap().approx_eq(Point3::new(1.0, 0.0, 0.0)));
}

#[test]
fn tess_circle_via_extension_trait() {
    let c = Circle::xy(Point3::ORIGIN, 1.0);
    let pts = c.tessellate_adaptive(&TessellationOptions::default());
    assert!(pts.len() > 4);
}

// ---------------------------------------------------------------------------
// Tessellation: surfaces
// ---------------------------------------------------------------------------

#[test]
fn tess_surface_flat_returns_nonempty_mesh() {
    let mesh = adaptive_tessellate_surface(
        |u, v| Point3::new(u, v, 0.0),
        |_, _| Vec3::Z,
        (0.0, 1.0),
        (0.0, 1.0),
        &TessellationOptions::default(),
    );
    assert!(!mesh.vertices.is_empty());
    assert!(!mesh.indices.is_empty());
}

#[test]
fn tess_sphere_via_extension_trait() {
    let s = Sphere::new(Point3::ORIGIN, 1.0).unwrap();
    let mesh = s.tessellate_adaptive(&TessellationOptions::default());
    assert!(!mesh.vertices.is_empty());
    assert!(!mesh.indices.is_empty());
}

// ---------------------------------------------------------------------------
// Aabb
// ---------------------------------------------------------------------------

#[test]
fn aabb_new() {
    let bb = Aabb::new(Point3::new(-1.0, -2.0, -3.0), Point3::new(1.0, 2.0, 3.0));
    assert!(bb.min.approx_eq(Point3::new(-1.0, -2.0, -3.0)));
    assert!(bb.max.approx_eq(Point3::new(1.0, 2.0, 3.0)));
}

#[test]
fn aabb_from_points_single() {
    let p = Point3::new(1.0, 2.0, 3.0);
    let bb = Aabb::from_points(&[p]);
    assert!(bb.min.approx_eq(p));
    assert!(bb.max.approx_eq(p));
}

#[test]
fn aabb_from_points_multi() {
    let pts = [
        Point3::new(-1.0, 0.0, 0.0),
        Point3::new(2.0, 3.0, -5.0),
        Point3::new(0.0, -4.0, 1.0),
    ];
    let bb = Aabb::from_points(&pts);
    assert!((bb.min.x - (-1.0)).abs() < TOL);
    assert!((bb.min.y - (-4.0)).abs() < TOL);
    assert!((bb.min.z - (-5.0)).abs() < TOL);
    assert!((bb.max.x - 2.0).abs() < TOL);
    assert!((bb.max.y - 3.0).abs() < TOL);
    assert!((bb.max.z - 1.0).abs() < TOL);
}

#[test]
#[should_panic]
fn aabb_from_points_empty_panics() {
    let _ = Aabb::from_points(&[]);
}

#[test]
fn aabb_merge_grows_both() {
    let a = Aabb::new(Point3::ORIGIN, Point3::new(1.0, 1.0, 1.0));
    let b = Aabb::new(Point3::new(-1.0, -1.0, -1.0), Point3::new(0.5, 0.5, 0.5));
    let m = a.merge(&b);
    assert!((m.min.x - (-1.0)).abs() < TOL);
    assert!((m.min.y - (-1.0)).abs() < TOL);
    assert!((m.min.z - (-1.0)).abs() < TOL);
    assert!((m.max.x - 1.0).abs() < TOL);
}

#[test]
fn aabb_intersects_overlap_and_disjoint() {
    let a = Aabb::new(Point3::ORIGIN, Point3::new(2.0, 2.0, 2.0));
    let b = Aabb::new(Point3::new(1.0, 1.0, 1.0), Point3::new(3.0, 3.0, 3.0));
    let c = Aabb::new(Point3::new(5.0, 5.0, 5.0), Point3::new(6.0, 6.0, 6.0));
    assert!(a.intersects(&b));
    assert!(b.intersects(&a));
    assert!(!a.intersects(&c));
    assert!(!c.intersects(&b));
}

#[test]
fn aabb_contains_point_interior_boundary_exterior() {
    let bb = Aabb::new(Point3::ORIGIN, Point3::new(1.0, 1.0, 1.0));
    assert!(bb.contains_point(Point3::new(0.5, 0.5, 0.5)));
    assert!(bb.contains_point(Point3::ORIGIN));
    assert!(bb.contains_point(Point3::new(1.0, 1.0, 1.0)));
    assert!(!bb.contains_point(Point3::new(1.5, 0.5, 0.5)));
}

#[test]
fn aabb_surface_area_unit_cube_is_six() {
    let bb = Aabb::new(Point3::ORIGIN, Point3::new(1.0, 1.0, 1.0));
    assert!((bb.surface_area() - 6.0).abs() < TOL);
}

#[test]
fn aabb_center_midpoint() {
    let bb = Aabb::new(Point3::new(-2.0, 0.0, 4.0), Point3::new(2.0, 4.0, 8.0));
    let c = bb.center();
    assert!(c.approx_eq(Point3::new(0.0, 2.0, 6.0)));
}

#[test]
fn aabb_expand_all_axes() {
    let bb = Aabb::new(Point3::ORIGIN, Point3::new(1.0, 1.0, 1.0));
    let ex = bb.expand(0.5);
    assert!((ex.min.x - (-0.5)).abs() < TOL);
    assert!((ex.max.z - 1.5).abs() < TOL);
}

#[test]
fn aabb_min_distance_sq_inside_zero() {
    let bb = Aabb::new(Point3::ORIGIN, Point3::new(1.0, 1.0, 1.0));
    assert_eq!(bb.min_distance_sq(Point3::new(0.5, 0.5, 0.5)), 0.0);
}

#[test]
fn aabb_min_distance_sq_outside() {
    let bb = Aabb::new(Point3::ORIGIN, Point3::new(1.0, 1.0, 1.0));
    let d = bb.min_distance_sq(Point3::new(3.0, 0.5, 0.5));
    assert!((d - 4.0).abs() < TOL);
}

#[test]
fn aabb_intersects_ray_t_hit_from_outside() {
    let bb = Aabb::new(Point3::ORIGIN, Point3::new(1.0, 1.0, 1.0));
    let t = bb
        .intersects_ray_t(Point3::new(-2.0, 0.5, 0.5), Vec3::X)
        .unwrap();
    assert!((t - 2.0).abs() < TOL);
}

#[test]
fn aabb_intersects_ray_t_origin_inside_is_zero() {
    let bb = Aabb::new(Point3::ORIGIN, Point3::new(1.0, 1.0, 1.0));
    let t = bb
        .intersects_ray_t(Point3::new(0.5, 0.5, 0.5), Vec3::X)
        .unwrap();
    assert!(t.abs() < TOL);
}

#[test]
fn aabb_intersects_ray_t_miss_returns_none() {
    let bb = Aabb::new(Point3::ORIGIN, Point3::new(1.0, 1.0, 1.0));
    assert!(bb
        .intersects_ray_t(Point3::new(-2.0, 5.0, 0.5), Vec3::X)
        .is_none());
}

// ---------------------------------------------------------------------------
// Bvh
// ---------------------------------------------------------------------------

fn aabb_xyz(x: f64, y: f64, z: f64, sx: f64, sy: f64, sz: f64) -> Aabb {
    Aabb::new(Point3::new(x, y, z), Point3::new(x + sx, y + sy, z + sz))
}

#[test]
fn bvh_build_empty() {
    let bvh = Bvh::build(&[]);
    assert!(bvh.is_empty());
    assert_eq!(bvh.len(), 0);
}

#[test]
fn bvh_len_after_build() {
    let items: Vec<(Aabb, usize)> = (0..5)
        .map(|i| (aabb_xyz(i as f64, 0.0, 0.0, 1.0, 1.0, 1.0), i))
        .collect();
    let bvh = Bvh::build(&items);
    assert_eq!(bvh.len(), 5);
    assert!(!bvh.is_empty());
}

#[test]
fn bvh_query_aabb_finds_overlapping() {
    let items: Vec<(Aabb, usize)> = (0..6)
        .map(|i| (aabb_xyz(i as f64, 0.0, 0.0, 1.0, 1.0, 1.0), i))
        .collect();
    let bvh = Bvh::build(&items);
    let hits = bvh.query_aabb(&Aabb::new(
        Point3::new(2.5, 0.0, 0.0),
        Point3::new(4.5, 1.0, 1.0),
    ));
    assert!(hits.contains(&2));
    assert!(hits.contains(&3));
    assert!(hits.contains(&4));
    assert!(!hits.contains(&0));
}

#[test]
fn bvh_query_point_finds_containing() {
    let items: Vec<(Aabb, usize)> = (0..5)
        .map(|i| (aabb_xyz(i as f64, 0.0, 0.0, 1.0, 1.0, 1.0), i))
        .collect();
    let bvh = Bvh::build(&items);
    let hits = bvh.query_point(Point3::new(3.5, 0.5, 0.5));
    assert_eq!(hits, vec![3]);
}

#[test]
fn bvh_query_ray_along_x() {
    let items: Vec<(Aabb, usize)> = (0..3)
        .map(|i| (aabb_xyz((i * 3) as f64, 0.0, 0.0, 1.0, 1.0, 1.0), i))
        .collect();
    let bvh = Bvh::build(&items);
    let hits = bvh.query_ray(Point3::new(-1.0, 0.5, 0.5), Vec3::X);
    assert_eq!(hits.len(), 3);
}

#[test]
fn bvh_query_nearest() {
    let items: Vec<(Aabb, usize)> = (0..5)
        .map(|i| (aabb_xyz(i as f64 * 3.0, 0.0, 0.0, 1.0, 1.0, 1.0), i))
        .collect();
    let bvh = Bvh::build(&items);
    let (idx, _d) = bvh.query_nearest(Point3::new(6.5, 0.5, 0.5)).unwrap();
    assert_eq!(idx, 2);
}

#[test]
fn bvh_query_nearest_empty() {
    let bvh = Bvh::build(&[]);
    assert!(bvh.query_nearest(Point3::ORIGIN).is_none());
}

#[test]
fn bvh_query_ray_sorted_order() {
    let items = vec![
        (aabb_xyz(10.0, 0.0, 0.0, 1.0, 1.0, 1.0), 2),
        (aabb_xyz(5.0, 0.0, 0.0, 1.0, 1.0, 1.0), 1),
        (aabb_xyz(0.0, 0.0, 0.0, 1.0, 1.0, 1.0), 0),
    ];
    let bvh = Bvh::build(&items);
    let hits = bvh.query_ray_sorted(Point3::new(-1.0, 0.5, 0.5), Vec3::X);
    assert_eq!(hits.len(), 3);
    assert_eq!(hits[0].0, 0);
    assert_eq!(hits[1].0, 1);
    assert_eq!(hits[2].0, 2);
    assert!(hits[0].1 <= hits[1].1);
    assert!(hits[1].1 <= hits[2].1);
}

// ---------------------------------------------------------------------------
// Intersection: curves
// ---------------------------------------------------------------------------

#[test]
fn intersect_two_perpendicular_line_segments() {
    let a = LineSegment::new(Point3::new(-1.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0));
    let b = LineSegment::new(Point3::new(0.0, -1.0, 0.0), Point3::new(0.0, 1.0, 0.0));
    let hits = intersect_curves(&a, &b, 1e-8);
    assert_eq!(hits.len(), 1);
    assert!(hits[0].point.approx_eq(Point3::ORIGIN));
}

// ---------------------------------------------------------------------------
// Intersection: plane-plane
// ---------------------------------------------------------------------------

#[test]
fn intersect_xy_xz_returns_line_along_x() {
    let xy = Plane::xy().unwrap();
    let xz = Plane::xz().unwrap();
    match intersect_plane_plane(&xy, &xz) {
        SsiResult::Line { direction, .. } => {
            assert!(direction.cross(Vec3::X).length() < LOOSE_TOL);
        }
        other => panic!("expected Line, got {other:?}"),
    }
}

#[test]
fn intersect_same_plane_coincident() {
    let a = Plane::xy().unwrap();
    let b = Plane::xy().unwrap();
    assert!(matches!(intersect_plane_plane(&a, &b), SsiResult::Coincident));
}

#[test]
fn intersect_parallel_separated_planes_empty() {
    let a = Plane::xy().unwrap();
    let b = Plane::new(Point3::new(0.0, 0.0, 3.0), Vec3::X, Vec3::Y).unwrap();
    assert!(matches!(intersect_plane_plane(&a, &b), SsiResult::Empty));
}

// ---------------------------------------------------------------------------
// Intersection: plane-sphere
// ---------------------------------------------------------------------------

#[test]
fn intersect_plane_sphere_equator_circle() {
    let plane = Plane::xy().unwrap();
    let sphere = Sphere::new(Point3::ORIGIN, 1.0).unwrap();
    match intersect_plane_sphere(&plane, &sphere) {
        SsiResult::Circle { center, radius, .. } => {
            assert!(center.approx_eq(Point3::ORIGIN));
            assert!((radius - 1.0).abs() < TOL);
        }
        other => panic!("expected Circle, got {other:?}"),
    }
}

#[test]
fn intersect_plane_sphere_tangent_point() {
    let plane = Plane::new(Point3::new(0.0, 0.0, 1.0), Vec3::X, Vec3::Y).unwrap();
    let sphere = Sphere::new(Point3::ORIGIN, 1.0).unwrap();
    assert!(matches!(
        intersect_plane_sphere(&plane, &sphere),
        SsiResult::Point(_)
    ));
}

#[test]
fn intersect_plane_sphere_far_empty() {
    let plane = Plane::new(Point3::new(0.0, 0.0, 10.0), Vec3::X, Vec3::Y).unwrap();
    let sphere = Sphere::new(Point3::ORIGIN, 1.0).unwrap();
    assert!(matches!(
        intersect_plane_sphere(&plane, &sphere),
        SsiResult::Empty
    ));
}

// ---------------------------------------------------------------------------
// Offset
// ---------------------------------------------------------------------------

#[test]
fn offset_polygon_zero_distance_preserves_input() {
    let sq = vec![
        Point2::new(0.0, 0.0),
        Point2::new(1.0, 0.0),
        Point2::new(1.0, 1.0),
        Point2::new(0.0, 1.0),
    ];
    let out = offset_polygon_2d(&sq, 0.0);
    assert_eq!(out.len(), 4);
    for (a, b) in out.iter().zip(sq.iter()) {
        assert!((a.x - b.x).abs() < TOL);
        assert!((a.y - b.y).abs() < TOL);
    }
}

#[test]
fn offset_polygon_checked_two_vertex_errors() {
    let line = vec![Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)];
    assert!(offset_polygon_2d_checked(&line, 0.5).is_err());
}

#[test]
fn offset_polyline_nonempty_result() {
    let line = vec![
        Point2::new(0.0, 0.0),
        Point2::new(1.0, 0.0),
        Point2::new(2.0, 0.0),
    ];
    let out = offset_polyline_2d(&line, 1.0);
    assert_eq!(out.len(), 3);
    for p in &out {
        assert!((p.y - 1.0).abs() < 1e-8);
    }
}

// ---------------------------------------------------------------------------
// Thread-safety
// ---------------------------------------------------------------------------

#[test]
fn geometry_types_are_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Line>();
    assert_send_sync::<LineSegment>();
    assert_send_sync::<Circle>();
    assert_send_sync::<Arc>();
    assert_send_sync::<Ellipse>();
    assert_send_sync::<NurbsCurve>();
    assert_send_sync::<Plane>();
    assert_send_sync::<Cylinder>();
    assert_send_sync::<Sphere>();
    assert_send_sync::<Cone>();
    assert_send_sync::<Torus>();
    assert_send_sync::<NurbsSurface>();
    assert_send_sync::<Aabb>();
    assert_send_sync::<Bvh>();
    assert_send_sync::<TessellationOptions>();
    assert_send_sync::<TessMesh>();
    assert_send_sync::<BoundingBox>();
}
