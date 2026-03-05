//! Convenience re-exports of the most commonly used geometry types.
//!
//! ```
//! use cadkernel_geometry::prelude::*;
//! ```

pub use crate::curve::Curve;
pub use crate::curve::arc::Arc;
pub use crate::curve::circle::Circle;
pub use crate::curve::ellipse::Ellipse;
pub use crate::curve::line::{Line, LineSegment};
pub use crate::curve::nurbs::NurbsCurve;

pub use crate::offset::{offset_polygon_2d, offset_polyline_2d};

pub use crate::surface::Surface;
pub use crate::surface::cone::Cone;
pub use crate::surface::cylinder::Cylinder;
pub use crate::surface::nurbs::NurbsSurface;
pub use crate::surface::plane::Plane;
pub use crate::surface::sphere::Sphere;
pub use crate::surface::torus::Torus;

pub use crate::tessellate::{
    TessMesh, TessellateCurve, TessellateSurface, TessellationOptions, adaptive_tessellate_curve,
    adaptive_tessellate_surface,
};
