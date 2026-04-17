//! Comprehensive integration tests for the `cadkernel-math` crate.
//!
//! Covers the full public API surface: Vec2/3/4, Point2/3, Mat3/4, Transform,
//! Quaternion, Ray3, BoundingBox, and tolerance helpers.

use cadkernel_math::prelude::*;
use cadkernel_math::tolerance::{approx_eq, approx_eq_tol, is_zero};
use cadkernel_math::vector::Vec4;
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};

const TOL: f64 = 1e-10;

// ---------------------------------------------------------------------------
// Vec2
// ---------------------------------------------------------------------------

#[test]
fn vec2_constants() {
    assert_eq!(Vec2::ZERO, Vec2::new(0.0, 0.0));
    assert_eq!(Vec2::X, Vec2::new(1.0, 0.0));
    assert_eq!(Vec2::Y, Vec2::new(0.0, 1.0));
}

#[test]
fn vec2_length_and_squared() {
    let v = Vec2::new(3.0, 4.0);
    assert!((v.length() - 5.0).abs() < TOL);
    assert!((v.length_squared() - 25.0).abs() < TOL);
}

#[test]
fn vec2_dot_product() {
    let a = Vec2::new(1.0, 2.0);
    let b = Vec2::new(3.0, 4.0);
    assert!((a.dot(b) - 11.0).abs() < TOL);
}

#[test]
fn vec2_cross_is_signed_area() {
    let a = Vec2::X;
    let b = Vec2::Y;
    assert!((a.cross(b) - 1.0).abs() < TOL);
    assert!((b.cross(a) + 1.0).abs() < TOL);
}

#[test]
fn vec2_normalized_zero_returns_none() {
    assert!(Vec2::ZERO.normalized().is_none());
}

#[test]
fn vec2_normalized_unit_length() {
    let v = Vec2::new(5.0, 12.0).normalized().unwrap();
    assert!((v.length() - 1.0).abs() < TOL);
}

#[test]
fn vec2_arithmetic_ops() {
    let a = Vec2::new(1.0, 2.0);
    let b = Vec2::new(3.0, 4.0);
    assert_eq!(a + b, Vec2::new(4.0, 6.0));
    assert_eq!(b - a, Vec2::new(2.0, 2.0));
    assert_eq!(-a, Vec2::new(-1.0, -2.0));
    assert_eq!(a * 2.0, Vec2::new(2.0, 4.0));
    assert_eq!(2.0 * a, Vec2::new(2.0, 4.0));
    assert_eq!(a / 2.0, Vec2::new(0.5, 1.0));
}

#[test]
fn vec2_assign_ops() {
    let mut v = Vec2::new(1.0, 2.0);
    v += Vec2::new(3.0, 4.0);
    assert_eq!(v, Vec2::new(4.0, 6.0));
    v -= Vec2::new(1.0, 1.0);
    assert_eq!(v, Vec2::new(3.0, 5.0));
    v *= 2.0;
    assert_eq!(v, Vec2::new(6.0, 10.0));
    v /= 2.0;
    assert_eq!(v, Vec2::new(3.0, 5.0));
}

#[test]
fn vec2_sum_iter() {
    let vs = [Vec2::new(1.0, 2.0), Vec2::new(3.0, 4.0), Vec2::new(5.0, 6.0)];
    let sum: Vec2 = vs.iter().copied().sum();
    assert_eq!(sum, Vec2::new(9.0, 12.0));
}

#[test]
fn vec2_from_array_and_tuple() {
    assert_eq!(Vec2::from([1.0, 2.0]), Vec2::new(1.0, 2.0));
    assert_eq!(Vec2::from((3.0, 4.0)), Vec2::new(3.0, 4.0));
}

// ---------------------------------------------------------------------------
// Vec3
// ---------------------------------------------------------------------------

#[test]
fn vec3_constants() {
    assert_eq!(Vec3::ZERO, Vec3::new(0.0, 0.0, 0.0));
    assert_eq!(Vec3::X, Vec3::new(1.0, 0.0, 0.0));
    assert_eq!(Vec3::Y, Vec3::new(0.0, 1.0, 0.0));
    assert_eq!(Vec3::Z, Vec3::new(0.0, 0.0, 1.0));
}

#[test]
fn vec3_length_and_squared() {
    let v = Vec3::new(2.0, 3.0, 6.0);
    assert!((v.length() - 7.0).abs() < TOL);
    assert!((v.length_squared() - 49.0).abs() < TOL);
}

#[test]
fn vec3_cross_right_hand_rule() {
    assert!(Vec3::X.cross(Vec3::Y).approx_eq(Vec3::Z));
    assert!(Vec3::Y.cross(Vec3::Z).approx_eq(Vec3::X));
    assert!(Vec3::Z.cross(Vec3::X).approx_eq(Vec3::Y));
    assert!(Vec3::Y.cross(Vec3::X).approx_eq(-Vec3::Z));
}

#[test]
fn vec3_cross_anticommutative() {
    let a = Vec3::new(1.0, 2.0, 3.0);
    let b = Vec3::new(4.0, 5.0, 6.0);
    assert!(a.cross(b).approx_eq(-b.cross(a)));
}

#[test]
fn vec3_dot_commutative() {
    let a = Vec3::new(1.0, 2.0, 3.0);
    let b = Vec3::new(4.0, 5.0, 6.0);
    assert!((a.dot(b) - b.dot(a)).abs() < TOL);
}

#[test]
fn vec3_approx_eq_threshold() {
    let a = Vec3::new(1.0, 0.0, 0.0);
    let b = Vec3::new(1.0 + 1e-9, 0.0, 0.0);
    assert!(a.approx_eq(b));
    let c = Vec3::new(1.0 + 1e-5, 0.0, 0.0);
    assert!(!a.approx_eq(c));
}

#[test]
fn vec3_sum_iter() {
    let vs = [Vec3::X, Vec3::Y, Vec3::Z];
    let sum: Vec3 = vs.iter().copied().sum();
    assert!(sum.approx_eq(Vec3::new(1.0, 1.0, 1.0)));
}

#[test]
fn vec3_from_array_and_tuple() {
    assert_eq!(Vec3::from([1.0, 2.0, 3.0]), Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(Vec3::from((4.0, 5.0, 6.0)), Vec3::new(4.0, 5.0, 6.0));
}

#[test]
fn vec3_nalgebra_roundtrip() {
    let v = Vec3::new(1.5, 2.5, 3.5);
    let na = v.to_nalgebra();
    let back = Vec3::from_nalgebra(na);
    assert!(v.approx_eq(back));
}

#[test]
fn vec3_scalar_division() {
    let v = Vec3::new(4.0, 8.0, 12.0) / 4.0;
    assert!(v.approx_eq(Vec3::new(1.0, 2.0, 3.0)));
}

// ---------------------------------------------------------------------------
// Vec4
// ---------------------------------------------------------------------------

#[test]
fn vec4_dot_product() {
    let a = Vec4::new(1.0, 2.0, 3.0, 4.0);
    let b = Vec4::new(5.0, 6.0, 7.0, 8.0);
    // 5 + 12 + 21 + 32 = 70
    assert!((a.dot(b) - 70.0).abs() < TOL);
}

#[test]
fn vec4_truncate_discards_w() {
    let v = Vec4::new(1.0, 2.0, 3.0, 9.0);
    let t = v.truncate();
    assert!(t.approx_eq(Vec3::new(1.0, 2.0, 3.0)));
}

#[test]
fn vec4_scalar_left_multiply() {
    let v = 2.0 * Vec4::new(1.0, 2.0, 3.0, 4.0);
    assert_eq!(v, Vec4::new(2.0, 4.0, 6.0, 8.0));
}

#[test]
fn vec4_default_is_zero() {
    assert_eq!(Vec4::default(), Vec4::ZERO);
}

// ---------------------------------------------------------------------------
// Point2
// ---------------------------------------------------------------------------

#[test]
fn point2_origin_default() {
    assert_eq!(Point2::default(), Point2::ORIGIN);
}

#[test]
fn point2_distance_and_midpoint() {
    let a = Point2::new(0.0, 0.0);
    let b = Point2::new(3.0, 4.0);
    assert!((a.distance_to(b) - 5.0).abs() < TOL);
    let mid = a.midpoint(b);
    assert!(mid.approx_eq(Point2::new(1.5, 2.0)));
}

#[test]
fn point2_sub_yields_vec2() {
    let a = Point2::new(5.0, 7.0);
    let b = Point2::new(1.0, 3.0);
    let v: Vec2 = a - b;
    assert_eq!(v, Vec2::new(4.0, 4.0));
}

#[test]
fn point2_plus_vec2() {
    let p = Point2::ORIGIN + Vec2::new(2.0, 3.0);
    assert!(p.approx_eq(Point2::new(2.0, 3.0)));
}

#[test]
fn point2_assign_ops() {
    let mut p = Point2::new(1.0, 1.0);
    p += Vec2::new(2.0, 3.0);
    assert_eq!(p, Point2::new(3.0, 4.0));
    p -= Vec2::new(1.0, 1.0);
    assert_eq!(p, Point2::new(2.0, 3.0));
}

#[test]
fn point2_nalgebra_roundtrip() {
    let p = Point2::new(7.0, 8.0);
    let na = p.to_nalgebra();
    let back = Point2::from_nalgebra(na);
    assert!(p.approx_eq(back));
}

// ---------------------------------------------------------------------------
// Point3
// ---------------------------------------------------------------------------

#[test]
fn point3_origin_default() {
    assert_eq!(Point3::default(), Point3::ORIGIN);
}

#[test]
fn point3_distance_and_midpoint() {
    let a = Point3::new(0.0, 0.0, 0.0);
    let b = Point3::new(2.0, 3.0, 6.0);
    assert!((a.distance_to(b) - 7.0).abs() < TOL);
    let mid = a.midpoint(b);
    assert!(mid.approx_eq(Point3::new(1.0, 1.5, 3.0)));
}

#[test]
fn point3_plus_and_minus_vec3() {
    let p = Point3::ORIGIN + Vec3::new(1.0, 2.0, 3.0);
    let q = p - Vec3::new(1.0, 0.0, 0.0);
    assert!(q.approx_eq(Point3::new(0.0, 2.0, 3.0)));
}

#[test]
fn point3_conversions_to_vec3() {
    let p = Point3::new(1.0, 2.0, 3.0);
    let v: Vec3 = p.into();
    assert_eq!(v, Vec3::new(1.0, 2.0, 3.0));
    let back: Point3 = v.into();
    assert!(back.approx_eq(p));
}

#[test]
fn point3_nalgebra_roundtrip() {
    let p = Point3::new(1.0, 2.0, 3.0);
    let na = p.to_nalgebra();
    let back = Point3::from_nalgebra(na);
    assert!(p.approx_eq(back));
}

// ---------------------------------------------------------------------------
// Mat3 / Mat4
// ---------------------------------------------------------------------------

#[test]
fn mat3_identity_det_one() {
    assert!((Mat3::IDENTITY.determinant() - 1.0).abs() < TOL);
}

#[test]
fn mat3_identity_mul_preserves() {
    let a = Mat3::IDENTITY;
    let b = Mat3::IDENTITY;
    let c = a * b;
    assert!((c.determinant() - 1.0).abs() < TOL);
}

#[test]
fn mat3_identity_inverse() {
    let inv = Mat3::IDENTITY.try_inverse().unwrap();
    assert!((inv.determinant() - 1.0).abs() < TOL);
}

#[test]
fn mat4_identity_det_one() {
    assert!((Mat4::IDENTITY.determinant() - 1.0).abs() < TOL);
}

#[test]
fn mat4_translation_transform_point() {
    let m = Mat4::translation(Vec3::new(3.0, 4.0, 5.0));
    let p = m.transform_point(Point3::new(1.0, 1.0, 1.0));
    assert!(p.approx_eq(Point3::new(4.0, 5.0, 6.0)));
}

#[test]
fn mat4_from_rows_matches_identity() {
    let m = Mat4::from_rows(
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    );
    assert_eq!(m, Mat4::IDENTITY);
}

#[test]
fn mat4_inverse_of_translation() {
    let m = Mat4::translation(Vec3::new(5.0, 6.0, 7.0));
    let inv = m.try_inverse().unwrap();
    let id = m * inv;
    // Identity transforms origin back to origin
    let p = id.transform_point(Point3::ORIGIN);
    assert!(p.approx_eq(Point3::ORIGIN));
}

#[test]
fn mat4_default_is_identity() {
    assert_eq!(Mat4::default(), Mat4::IDENTITY);
}

// ---------------------------------------------------------------------------
// Transform
// ---------------------------------------------------------------------------

#[test]
fn transform_identity_preserves_point() {
    let t = Transform::IDENTITY;
    let p = Point3::new(1.0, 2.0, 3.0);
    assert!(t.apply_point(p).approx_eq(p));
}

#[test]
fn transform_translation_additive() {
    let t = Transform::translation(1.0, 2.0, 3.0);
    let p = t.apply_point(Point3::new(10.0, 20.0, 30.0));
    assert!(p.approx_eq(Point3::new(11.0, 22.0, 33.0)));
}

#[test]
fn transform_uniform_scale_on_unit_sphere() {
    let t = Transform::uniform_scale(3.0);
    let p = t.apply_point(Point3::new(1.0, 0.0, 0.0));
    assert!(p.approx_eq(Point3::new(3.0, 0.0, 0.0)));
}

#[test]
fn transform_non_uniform_scale() {
    let t = Transform::scale(2.0, 3.0, 4.0);
    let p = t.apply_point(Point3::new(1.0, 1.0, 1.0));
    assert!(p.approx_eq(Point3::new(2.0, 3.0, 4.0)));
}

#[test]
fn transform_rotation_x_moves_y_to_z() {
    let t = Transform::rotation_x(FRAC_PI_2);
    let p = t.apply_point(Point3::new(0.0, 1.0, 0.0));
    assert!(p.approx_eq(Point3::new(0.0, 0.0, 1.0)));
}

#[test]
fn transform_rotation_y_moves_z_to_x() {
    let t = Transform::rotation_y(FRAC_PI_2);
    let p = t.apply_point(Point3::new(0.0, 0.0, 1.0));
    assert!(p.approx_eq(Point3::new(1.0, 0.0, 0.0)));
}

#[test]
fn transform_rotation_z_moves_x_to_y() {
    let t = Transform::rotation_z(FRAC_PI_2);
    let p = t.apply_point(Point3::new(1.0, 0.0, 0.0));
    assert!(p.approx_eq(Point3::new(0.0, 1.0, 0.0)));
}

#[test]
fn transform_compose_order_matters() {
    // T then S: first translate (1,0,0), then scale by 2 => (2,0,0)
    let ts = Transform::translation(1.0, 0.0, 0.0).then(Transform::uniform_scale(2.0));
    let p = ts.apply_point(Point3::ORIGIN);
    assert!(p.approx_eq(Point3::new(2.0, 0.0, 0.0)));

    // S then T: first scale by 2 (origin stays), then translate => (1,0,0)
    let st = Transform::uniform_scale(2.0).then(Transform::translation(1.0, 0.0, 0.0));
    let p = st.apply_point(Point3::ORIGIN);
    assert!(p.approx_eq(Point3::new(1.0, 0.0, 0.0)));
}

#[test]
fn transform_apply_vec_ignores_translation() {
    let t = Transform::translation(100.0, 200.0, 300.0);
    let v = t.apply_vec(Vec3::X);
    assert!(v.approx_eq(Vec3::X));
}

#[test]
fn transform_inverse_composes_to_identity() {
    let t = Transform::translation(1.0, 2.0, 3.0).then(Transform::rotation_z(FRAC_PI_4));
    let inv = t.try_inverse().unwrap();
    let p = Point3::new(5.0, 6.0, 7.0);
    let back = inv.apply_point(t.apply_point(p));
    assert!(back.approx_eq(p));
}

#[test]
fn transform_rotation_axis_angle_matches_z() {
    let t = Transform::rotation_axis_angle(Vec3::Z, FRAC_PI_2);
    let p = t.apply_point(Point3::new(1.0, 0.0, 0.0));
    assert!(p.approx_eq(Point3::new(0.0, 1.0, 0.0)));
}

#[test]
fn transform_rotation_around_point_offset_center() {
    let center = Point3::new(1.0, 0.0, 0.0);
    let t = Transform::rotation_around_point(center, Vec3::Z, PI);
    // 180° around (1,0,0) moves (2,0,0) -> (0,0,0)
    let p = t.apply_point(Point3::new(2.0, 0.0, 0.0));
    assert!(p.approx_eq(Point3::ORIGIN));
}

#[test]
fn transform_mirror_xy_plane_flips_z() {
    let t = Transform::mirror(Point3::ORIGIN, Vec3::Z);
    let p = t.apply_point(Point3::new(1.0, 2.0, 3.0));
    assert!(p.approx_eq(Point3::new(1.0, 2.0, -3.0)));
}

#[test]
fn transform_mirror_offset_plane() {
    // Mirror plane z=5 flips (1,2,3) to (1,2,7)
    let t = Transform::mirror(Point3::new(0.0, 0.0, 5.0), Vec3::Z);
    let p = t.apply_point(Point3::new(1.0, 2.0, 3.0));
    assert!(p.approx_eq(Point3::new(1.0, 2.0, 7.0)));
}

#[test]
fn transform_from_quaternion_matches_rotation() {
    let q = Quaternion::from_axis_angle(Vec3::Z, FRAC_PI_2);
    let t = Transform::from_quaternion(q);
    let p = t.apply_point(Point3::new(1.0, 0.0, 0.0));
    assert!(p.approx_eq(Point3::new(0.0, 1.0, 0.0)));
}

#[test]
fn transform_matrix_accessor() {
    let t = Transform::translation(1.0, 2.0, 3.0);
    let m = t.matrix();
    let p = m.transform_point(Point3::ORIGIN);
    assert!(p.approx_eq(Point3::new(1.0, 2.0, 3.0)));
}

#[test]
fn transform_default_is_identity() {
    assert_eq!(Transform::default(), Transform::IDENTITY);
}

// ---------------------------------------------------------------------------
// Quaternion
// ---------------------------------------------------------------------------

#[test]
fn quaternion_identity_preserves_vector() {
    let v = Quaternion::IDENTITY.rotate_vec(Vec3::new(1.0, 2.0, 3.0));
    assert!(v.approx_eq(Vec3::new(1.0, 2.0, 3.0)));
}

#[test]
fn quaternion_from_axis_angle_normalizes_axis() {
    // Non-unit axis should still yield a normal rotation
    let q = Quaternion::from_axis_angle(Vec3::new(0.0, 0.0, 5.0), FRAC_PI_2);
    let v = q.rotate_vec(Vec3::X);
    assert!(v.approx_eq(Vec3::Y));
}

#[test]
fn quaternion_rotate_vec_90_each_axis() {
    let qx = Quaternion::from_axis_angle(Vec3::X, FRAC_PI_2);
    assert!(qx.rotate_vec(Vec3::Y).approx_eq(Vec3::Z));
    let qy = Quaternion::from_axis_angle(Vec3::Y, FRAC_PI_2);
    assert!(qy.rotate_vec(Vec3::Z).approx_eq(Vec3::X));
    let qz = Quaternion::from_axis_angle(Vec3::Z, FRAC_PI_2);
    assert!(qz.rotate_vec(Vec3::X).approx_eq(Vec3::Y));
}

#[test]
fn quaternion_conjugate_unit_is_inverse() {
    let q = Quaternion::from_axis_angle(Vec3::new(1.0, 1.0, 0.0), 0.7);
    let prod = q * q.conjugate();
    assert!((prod.w - 1.0).abs() < TOL);
    assert!(prod.x.abs() < TOL);
    assert!(prod.y.abs() < TOL);
    assert!(prod.z.abs() < TOL);
}

#[test]
fn quaternion_axis_angle_roundtrip() {
    let axis = Vec3::new(1.0, 1.0, 1.0).normalized().unwrap();
    let q = Quaternion::from_axis_angle(axis, 1.3);
    let (a, angle) = q.to_axis_angle();
    assert!((angle - 1.3).abs() < 1e-9);
    assert!(a.approx_eq(axis));
}

#[test]
fn quaternion_norm_of_unit_is_one() {
    let q = Quaternion::from_axis_angle(Vec3::Y, 1.0);
    assert!((q.norm() - 1.0).abs() < TOL);
}

#[test]
fn quaternion_slerp_endpoints() {
    let a = Quaternion::IDENTITY;
    let b = Quaternion::from_axis_angle(Vec3::Z, FRAC_PI_2);
    let s0 = a.slerp(b, 0.0);
    let s1 = a.slerp(b, 1.0);
    assert!((s0.w - a.w).abs() < TOL);
    assert!((s1.w - b.w).abs() < TOL);
}

#[test]
fn quaternion_slerp_midpoint_is_half_rotation() {
    let a = Quaternion::IDENTITY;
    let b = Quaternion::from_axis_angle(Vec3::Z, FRAC_PI_2);
    let mid = a.slerp(b, 0.5);
    let v = mid.rotate_vec(Vec3::X);
    // Midpoint is a 45° rotation of X
    let expected = Vec3::new(FRAC_PI_4.cos(), FRAC_PI_4.sin(), 0.0);
    assert!(v.approx_eq(expected));
}

#[test]
fn quaternion_multiplication_is_rotation_composition() {
    let q1 = Quaternion::from_axis_angle(Vec3::Z, FRAC_PI_4);
    let q2 = Quaternion::from_axis_angle(Vec3::Z, FRAC_PI_4);
    let q = q2 * q1; // Applying q1 then q2 = composite rotation
    let v = q.rotate_vec(Vec3::X);
    assert!(v.approx_eq(Vec3::Y));
}

#[test]
fn quaternion_identity_to_axis_angle_zero() {
    let (_axis, angle) = Quaternion::IDENTITY.to_axis_angle();
    assert!(angle.abs() < TOL);
}

#[test]
fn quaternion_default_is_identity() {
    assert_eq!(Quaternion::default(), Quaternion::IDENTITY);
}

#[test]
fn quaternion_slerp_handles_near_identical() {
    let a = Quaternion::from_axis_angle(Vec3::Z, 0.001);
    let b = Quaternion::from_axis_angle(Vec3::Z, 0.002);
    let s = a.slerp(b, 0.5);
    assert!((s.norm() - 1.0).abs() < TOL);
}

// ---------------------------------------------------------------------------
// Ray3
// ---------------------------------------------------------------------------

#[test]
fn ray_constructs_normalized_direction() {
    let r = Ray3::new(Point3::ORIGIN, Vec3::new(2.0, 0.0, 0.0));
    assert!((r.direction.length() - 1.0).abs() < TOL);
}

#[test]
fn ray_zero_direction_fallback() {
    let r = Ray3::new(Point3::ORIGIN, Vec3::ZERO);
    assert!(r.direction.approx_eq(Vec3::Z));
}

#[test]
fn ray_at_parameter() {
    let r = Ray3::new(Point3::new(1.0, 0.0, 0.0), Vec3::X);
    let p = r.at(3.0);
    assert!(p.approx_eq(Point3::new(4.0, 0.0, 0.0)));
}

#[test]
fn ray_project_point_onto_line() {
    let r = Ray3::new(Point3::ORIGIN, Vec3::Z);
    let t = r.project(Point3::new(1.0, 2.0, 5.0));
    assert!((t - 5.0).abs() < TOL);
}

#[test]
fn ray_project_negative_t_behind_origin() {
    let r = Ray3::new(Point3::ORIGIN, Vec3::X);
    let t = r.project(Point3::new(-5.0, 0.0, 0.0));
    assert!((t + 5.0).abs() < TOL);
}

#[test]
fn ray_closest_point_clamps_to_ray() {
    let r = Ray3::new(Point3::ORIGIN, Vec3::X);
    // Point behind ray origin → clamped to origin
    let cp = r.closest_point(Point3::new(-10.0, 5.0, 0.0));
    assert!(cp.approx_eq(Point3::ORIGIN));
}

#[test]
fn ray_distance_to_point_perpendicular() {
    let r = Ray3::new(Point3::ORIGIN, Vec3::X);
    let d = r.distance_to_point(Point3::new(7.0, 3.0, 4.0));
    assert!((d - 5.0).abs() < TOL);
}

#[test]
fn ray_distance_along_line_is_zero() {
    let r = Ray3::new(Point3::ORIGIN, Vec3::X);
    let d = r.distance_to_point(Point3::new(42.0, 0.0, 0.0));
    assert!(d.abs() < TOL);
}

// ---------------------------------------------------------------------------
// BoundingBox
// ---------------------------------------------------------------------------

#[test]
fn bbox_new_normalizes_corners() {
    let bb = BoundingBox::new(Point3::new(1.0, 2.0, 3.0), Point3::new(-1.0, -2.0, -3.0));
    assert!(bb.min.approx_eq(Point3::new(-1.0, -2.0, -3.0)));
    assert!(bb.max.approx_eq(Point3::new(1.0, 2.0, 3.0)));
}

#[test]
fn bbox_empty_is_empty() {
    let bb = BoundingBox::empty();
    assert!(bb.is_empty());
}

#[test]
fn bbox_include_point_expands() {
    let mut bb = BoundingBox::empty();
    bb.include_point(Point3::new(1.0, 0.0, 0.0));
    bb.include_point(Point3::new(-2.0, 3.0, 0.0));
    assert!(!bb.is_empty());
    assert!(bb.contains(Point3::new(0.0, 1.0, 0.0)));
}

#[test]
fn bbox_contains_boundary() {
    let bb = BoundingBox::new(Point3::ORIGIN, Point3::new(1.0, 1.0, 1.0));
    assert!(bb.contains(Point3::ORIGIN));
    assert!(bb.contains(Point3::new(1.0, 1.0, 1.0)));
}

#[test]
fn bbox_contains_outside() {
    let bb = BoundingBox::new(Point3::ORIGIN, Point3::new(1.0, 1.0, 1.0));
    assert!(!bb.contains(Point3::new(1.1, 0.0, 0.0)));
}

#[test]
fn bbox_union_covers_both() {
    let a = BoundingBox::new(Point3::new(-1.0, -1.0, -1.0), Point3::new(1.0, 1.0, 1.0));
    let b = BoundingBox::new(Point3::new(5.0, 5.0, 5.0), Point3::new(6.0, 6.0, 6.0));
    let u = a.union(&b);
    assert!(u.contains(Point3::new(0.0, 0.0, 0.0)));
    assert!(u.contains(Point3::new(5.5, 5.5, 5.5)));
}

#[test]
fn bbox_intersection_overlapping() {
    let a = BoundingBox::new(Point3::ORIGIN, Point3::new(2.0, 2.0, 2.0));
    let b = BoundingBox::new(Point3::new(1.0, 1.0, 1.0), Point3::new(3.0, 3.0, 3.0));
    let i = a.intersection(&b).unwrap();
    assert!(i.min.approx_eq(Point3::new(1.0, 1.0, 1.0)));
    assert!(i.max.approx_eq(Point3::new(2.0, 2.0, 2.0)));
}

#[test]
fn bbox_intersection_disjoint_returns_none() {
    let a = BoundingBox::new(Point3::ORIGIN, Point3::new(1.0, 1.0, 1.0));
    let b = BoundingBox::new(Point3::new(5.0, 5.0, 5.0), Point3::new(6.0, 6.0, 6.0));
    assert!(a.intersection(&b).is_none());
}

#[test]
fn bbox_center_is_halfway() {
    let bb = BoundingBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 4.0, 6.0));
    assert!(bb.center().approx_eq(Point3::new(1.0, 2.0, 3.0)));
}

#[test]
fn bbox_diagonal_max_minus_min() {
    let bb = BoundingBox::new(Point3::ORIGIN, Point3::new(3.0, 4.0, 12.0));
    assert!(bb.diagonal().approx_eq(Vec3::new(3.0, 4.0, 12.0)));
}

#[test]
fn bbox_overlaps_shared_edge() {
    let a = BoundingBox::new(Point3::ORIGIN, Point3::new(1.0, 1.0, 1.0));
    let b = BoundingBox::new(Point3::new(1.0, 0.0, 0.0), Point3::new(2.0, 1.0, 1.0));
    assert!(a.overlaps(&b));
}

#[test]
fn bbox_expand_grows_uniformly() {
    let bb = BoundingBox::new(Point3::ORIGIN, Point3::new(1.0, 1.0, 1.0));
    let e = bb.expand(0.5);
    assert!(e.min.approx_eq(Point3::new(-0.5, -0.5, -0.5)));
    assert!(e.max.approx_eq(Point3::new(1.5, 1.5, 1.5)));
}

#[test]
fn bbox_volume_and_area_unit_cube() {
    let bb = BoundingBox::new(Point3::ORIGIN, Point3::new(1.0, 1.0, 1.0));
    assert!((bb.volume() - 1.0).abs() < TOL);
    assert!((bb.surface_area() - 6.0).abs() < TOL);
}

#[test]
fn bbox_volume_rectangular_box() {
    let bb = BoundingBox::new(Point3::ORIGIN, Point3::new(2.0, 3.0, 4.0));
    assert!((bb.volume() - 24.0).abs() < TOL);
    // Surface area = 2 * (6 + 12 + 8) = 52
    assert!((bb.surface_area() - 52.0).abs() < TOL);
}

#[test]
fn bbox_longest_axis_returns_dominant() {
    assert_eq!(
        BoundingBox::new(Point3::ORIGIN, Point3::new(10.0, 1.0, 1.0)).longest_axis(),
        0
    );
    assert_eq!(
        BoundingBox::new(Point3::ORIGIN, Point3::new(1.0, 10.0, 1.0)).longest_axis(),
        1
    );
    assert_eq!(
        BoundingBox::new(Point3::ORIGIN, Point3::new(1.0, 1.0, 10.0)).longest_axis(),
        2
    );
}

#[test]
fn bbox_from_point_slice() {
    let pts = [
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 2.0, 3.0),
        Point3::new(-1.0, -2.0, 0.5),
    ];
    let bb: BoundingBox = pts.as_slice().into();
    assert!(bb.min.approx_eq(Point3::new(-1.0, -2.0, 0.0)));
    assert!(bb.max.approx_eq(Point3::new(1.0, 2.0, 3.0)));
}

#[test]
fn bbox_default_is_empty() {
    assert!(BoundingBox::default().is_empty());
}

// ---------------------------------------------------------------------------
// Tolerance helpers
// ---------------------------------------------------------------------------

#[test]
fn tolerance_epsilon_is_1e8() {
    const { assert!(EPSILON < 1.0) };
    const { assert!(EPSILON > 0.0) };
    assert!((EPSILON - 1e-8).abs() < 1e-16);
}

#[test]
fn tolerance_approx_eq_returns_abs_diff() {
    assert!((approx_eq(1.0, 1.5) - 0.5).abs() < 1e-15);
    assert!(approx_eq(2.0, 2.0) < 1e-15);
}

#[test]
fn tolerance_is_zero_threshold() {
    assert!(is_zero(1e-10));
    assert!(is_zero(-1e-10));
    assert!(!is_zero(1e-6));
}

#[test]
fn tolerance_approx_eq_tol_custom_tolerance() {
    assert!(approx_eq_tol(1.0, 1.001, 0.01));
    assert!(!approx_eq_tol(1.0, 1.1, 0.01));
}

// ---------------------------------------------------------------------------
// Display impls (smoke tests)
// ---------------------------------------------------------------------------

#[test]
fn display_vec_point_bbox_ray_quat_work() {
    let _ = format!("{}", Vec2::X);
    let _ = format!("{}", Vec3::Y);
    let _ = format!("{}", Vec4::new(1.0, 2.0, 3.0, 4.0));
    let _ = format!("{}", Point2::ORIGIN);
    let _ = format!("{}", Point3::ORIGIN);
    let _ = format!(
        "{}",
        BoundingBox::new(Point3::ORIGIN, Point3::new(1.0, 1.0, 1.0))
    );
    let _ = format!("{}", Ray3::new(Point3::ORIGIN, Vec3::X));
    let _ = format!("{}", Quaternion::IDENTITY);
    let _ = format!("{}", Transform::IDENTITY);
}
