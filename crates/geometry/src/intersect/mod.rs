pub mod curve_curve;
pub mod curve_surface;
pub mod line_surface;
pub mod plane_cylinder;
pub mod plane_plane;
pub mod plane_sphere;
pub mod sphere_sphere;
pub mod surface_surface;
pub mod types;

pub use curve_curve::{CurveCurveHit, intersect_curves};
pub use curve_surface::{CurveSurfaceHit, intersect_curve_surface};
pub use surface_surface::{SsiCurve, SsiSeed, intersect_surfaces, ssi_starting_points};
pub use types::{IntersectionEllipse, RayHit, SsiResult};
