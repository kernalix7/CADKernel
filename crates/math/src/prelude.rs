//! Convenience re-exports of the most commonly used math types.
//!
//! ```
//! use cadkernel_math::prelude::*;
//! ```

pub use crate::bbox::BoundingBox;
pub use crate::matrix::{Mat3, Mat4};
pub use crate::point::{Point2, Point3};
pub use crate::quaternion::Quaternion;
pub use crate::ray::Ray3;
pub use crate::tolerance::EPSILON;
pub use crate::transform::Transform;
pub use crate::vector::{Vec2, Vec3, Vec4};
