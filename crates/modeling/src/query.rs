//! Spatial queries for B-Rep solids.

use cadkernel_core::KernelResult;
use cadkernel_math::Point3;
use cadkernel_topology::{BRepModel, Handle, SolidData};

/// Result of a closest-point query.
#[derive(Debug, Clone)]
pub struct ClosestPointResult {
    /// The closest point on the solid surface.
    pub point: Point3,
    /// Distance from the query point to the closest surface point.
    pub distance: f64,
}

/// Containment classification of a point with respect to a solid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Containment {
    Inside,
    Outside,
    OnBoundary,
}

/// Finds the closest point on a solid's surface to the given query point.
pub fn closest_point_on_solid(
    _model: &BRepModel,
    _solid: Handle<SolidData>,
    _point: Point3,
) -> KernelResult<ClosestPointResult> {
    todo!("closest_point_on_solid not yet implemented")
}

/// Classifies whether a point is inside, outside, or on the boundary of a solid.
pub fn point_in_solid(
    _model: &BRepModel,
    _solid: Handle<SolidData>,
    _point: Point3,
) -> KernelResult<Containment> {
    todo!("point_in_solid not yet implemented")
}
