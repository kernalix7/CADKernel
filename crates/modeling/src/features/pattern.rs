//! Linear and circular pattern (array) operations.

use cadkernel_core::KernelResult;
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, FaceData, Handle, SolidData};

/// Result of a pattern operation.
#[derive(Debug)]
pub struct PatternResult {
    pub solids: Vec<Handle<SolidData>>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Creates a linear pattern of a solid along a direction.
///
/// `count` copies are placed at `spacing` intervals along `direction`.
pub fn linear_pattern(
    _model: &mut BRepModel,
    _solid: Handle<SolidData>,
    _direction: Vec3,
    _spacing: f64,
    _count: usize,
) -> KernelResult<PatternResult> {
    todo!("linear_pattern not yet implemented")
}

/// Creates a circular pattern of a solid around an axis.
///
/// `count` copies are placed at equal angular intervals around the axis.
pub fn circular_pattern(
    _model: &mut BRepModel,
    _solid: Handle<SolidData>,
    _axis_origin: Point3,
    _axis_dir: Vec3,
    _count: usize,
) -> KernelResult<PatternResult> {
    todo!("circular_pattern not yet implemented")
}
