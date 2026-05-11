//! Mathematical primitives for the CAD kernel.
//!
//! Provides points, vectors, matrices, transforms, bounding boxes, quaternions,
//! rays, and geometric utility functions used by all higher-level crates.
//!
//! All geometric computations use `f64` precision. The tolerance constant
//! [`EPSILON`] (1e-8) is used for approximate-equality comparisons throughout
//! the kernel.
//!
//! # Key Types
//!
//! | Type | Description |
//! |------|-------------|
//! | [`Vec2`], [`Vec3`], [`Vec4`] | Direction vectors in 2D/3D/4D |
//! | [`Point2`], [`Point3`] | Positions in 2D/3D Euclidean space |
//! | [`Mat3`], [`Mat4`] | 3x3 and 4x4 matrices (wrapping `nalgebra`) |
//! | [`Transform`] | Affine 3D transform (translate, rotate, scale, mirror) |
//! | [`Quaternion`] | Unit quaternion for 3D rotations with SLERP |
//! | [`Ray3`] | Ray origin + normalised direction for intersection tests |
//! | [`BoundingBox`] | Axis-aligned bounding box (AABB) |
//!
//! # Examples
//!
//! ```
//! use cadkernel_math::{Point3, Vec3, Transform};
//!
//! let p = Point3::new(1.0, 0.0, 0.0);
//! let t = Transform::rotation_z(std::f64::consts::FRAC_PI_2);
//! let rotated = t.apply_point(p);
//! assert!(rotated.approx_eq(Point3::new(0.0, 1.0, 0.0)));
//! ```

// Commercial CAD Roadmap v0.5 Gate 12: math primitives feed every geometric
// computation in the workspace. Production code here must never panic.
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod bbox;
pub mod interop;
pub mod linalg;
pub mod matrix;
pub mod point;
pub mod prelude;
pub mod quaternion;
pub mod ray;
pub mod tolerance;
pub mod transform;
pub mod utils;
pub mod vector;

pub use bbox::BoundingBox;
pub use matrix::{Mat3, Mat4};
pub use point::{Point2, Point3};
pub use quaternion::Quaternion;
pub use ray::Ray3;
pub use tolerance::EPSILON;
pub use transform::Transform;
pub use vector::{Vec2, Vec3, Vec4};

#[cfg(test)]
mod thread_safety_tests {
    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn math_types_are_send_sync() {
        assert_send_sync::<crate::Point2>();
        assert_send_sync::<crate::Point3>();
        assert_send_sync::<crate::Vec2>();
        assert_send_sync::<crate::Vec3>();
        assert_send_sync::<crate::Vec4>();
        assert_send_sync::<crate::Transform>();
        assert_send_sync::<crate::BoundingBox>();
        assert_send_sync::<crate::Quaternion>();
        assert_send_sync::<crate::Ray3>();
        assert_send_sync::<crate::Mat3>();
        assert_send_sync::<crate::Mat4>();
    }
}
