//! Mathematical primitives for the CAD kernel.
//!
//! Provides points, vectors, matrices, transforms, bounding boxes, quaternions,
//! rays, and geometric utility functions used by all higher-level crates.

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
