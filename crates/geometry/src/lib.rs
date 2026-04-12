//! Geometric primitives for the CAD kernel.
//!
//! Provides parametric curves ([`Curve`] trait), parametric surfaces
//! ([`Surface`] trait), NURBS evaluation, tessellation, intersection routines,
//! and a BVH for spatial indexing.
//!
//! # Curves
//!
//! All curve types implement the [`Curve`] trait, which requires `point_at`,
//! `tangent_at`, `domain`, `length`, and `is_closed`. Analytic types
//! ([`Line`], [`LineSegment`], [`Circle`], [`Arc`], [`Ellipse`]) provide
//! exact evaluation. [`NurbsCurve`] uses the De Boor algorithm for
//! rational B-spline evaluation.
//!
//! # Surfaces
//!
//! All surface types implement the [`Surface`] trait with `point_at`,
//! `normal_at`, `domain_u`, and `domain_v`. Analytic surfaces ([`Plane`],
//! [`Cylinder`], [`Sphere`], [`Cone`], [`Torus`]) and [`NurbsSurface`]
//! are available.
//!
//! # Tessellation
//!
//! Use [`adaptive_tessellate_curve`] and [`adaptive_tessellate_surface`] (or
//! the [`TessellateCurve`] / [`TessellateSurface`] extension traits) to
//! convert geometry into triangle meshes for rendering.
//!
//! # Intersection
//!
//! - [`intersect_curves`]: curve-curve intersection
//! - [`intersect_curve_surface`]: curve-surface intersection (subdivision + Newton)
//! - [`intersect_surfaces`]: surface-surface intersection (marching algorithm)

pub mod bvh;
pub mod curve;
pub mod intersect;
pub mod offset;
pub mod prelude;
pub mod surface;
pub mod tessellate;

pub use bvh::{Aabb, Bvh};
pub use curve::Curve;
pub use curve::arc::Arc;
pub use curve::circle::Circle;
pub use curve::curve2d::{Circle2D, Curve2D, Line2D, NurbsCurve2D};
pub use curve::ellipse::Ellipse;
pub use curve::line::{Line, LineSegment};
pub use curve::nurbs::NurbsCurve;
pub use curve::trimmed::TrimmedCurve;

pub use intersect::{CurveCurveHit, CurveSurfaceHit, intersect_curve_surface, intersect_curves};
pub use intersect::{SsiCurve, SsiSeed, intersect_surfaces, ssi_starting_points};
pub use offset::{offset_polygon_2d, offset_polygon_2d_checked, offset_polyline_2d};

pub use surface::Surface;
pub use surface::cone::Cone;
pub use surface::cylinder::Cylinder;
pub use surface::nurbs::NurbsSurface;
pub use surface::plane::Plane;
pub use surface::sphere::Sphere;
pub use surface::torus::Torus;
pub use surface::parametric_wire::ParametricWire2D;
pub use surface::trimmed::TrimmedSurface;

pub use curve::blend::blend_curve;
pub use curve::bspline_basis::BasisCache;
pub use curve::offset_curve::OffsetCurve;
pub use surface::continuity::{ContinuityLevel, check_surface_continuity};
pub use surface::curvature::{SurfaceCurvatures, surface_curvatures};
pub use surface::extrusion::ExtrusionSurface;
pub use surface::filling::CoonsPatch;
pub use surface::isocurve::{IsocurveU, IsocurveV};
pub use surface::pipe::PipeSurface;
pub use surface::revolution::RevolutionSurface;

pub use tessellate::{
    LevelOfDetail, TessMesh, TessellateCurve, TessellateSurface, TessellationOptions,
    adaptive_tessellate_curve, adaptive_tessellate_surface,
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
        assert_send_sync::<crate::IsocurveU>();
        assert_send_sync::<crate::IsocurveV>();
        assert_send_sync::<crate::OffsetCurve>();
        assert_send_sync::<crate::ExtrusionSurface>();
        assert_send_sync::<crate::RevolutionSurface>();
        assert_send_sync::<crate::SurfaceCurvatures>();
        assert_send_sync::<crate::ContinuityLevel>();
    }
}
