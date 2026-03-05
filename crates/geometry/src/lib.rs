//! Geometric primitives for the CAD kernel.
//!
//! Provides parametric curves ([`Curve`] trait), parametric surfaces ([`Surface`] trait),
//! and surface-surface / line-surface intersection routines.

pub mod curve;
pub mod intersect;
pub mod offset;
pub mod prelude;
pub mod surface;
pub mod tessellate;

pub use curve::Curve;
pub use curve::arc::Arc;
pub use curve::circle::Circle;
pub use curve::ellipse::Ellipse;
pub use curve::line::{Line, LineSegment};
pub use curve::nurbs::NurbsCurve;

pub use offset::{offset_polygon_2d, offset_polyline_2d};

pub use surface::Surface;
pub use surface::cone::Cone;
pub use surface::cylinder::Cylinder;
pub use surface::nurbs::NurbsSurface;
pub use surface::plane::Plane;
pub use surface::sphere::Sphere;
pub use surface::torus::Torus;

pub use tessellate::{
    TessMesh, TessellateCurve, TessellateSurface, TessellationOptions, adaptive_tessellate_curve,
    adaptive_tessellate_surface,
};

#[cfg(test)]
mod thread_safety_tests {
    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn geometry_types_are_send_sync() {
        assert_send_sync::<crate::Line>();
        assert_send_sync::<crate::LineSegment>();
        assert_send_sync::<crate::Arc>();
        assert_send_sync::<crate::Circle>();
        assert_send_sync::<crate::Ellipse>();
        assert_send_sync::<crate::NurbsCurve>();
        assert_send_sync::<crate::Plane>();
        assert_send_sync::<crate::Cylinder>();
        assert_send_sync::<crate::Sphere>();
        assert_send_sync::<crate::Cone>();
        assert_send_sync::<crate::Torus>();
        assert_send_sync::<crate::NurbsSurface>();
        assert_send_sync::<crate::TessellationOptions>();
        assert_send_sync::<crate::TessMesh>();
    }
}
