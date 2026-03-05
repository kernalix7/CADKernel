use cadkernel_math::Point2;

/// A 2D point in sketch space, the fundamental degree of freedom.
#[derive(Debug, Clone, Copy)]
pub struct SketchPoint {
    pub position: Point2,
}

impl SketchPoint {
    /// Creates a sketch point at `(x, y)`.
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            position: Point2::new(x, y),
        }
    }
}

/// Index into the sketch's point storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PointId(pub usize);

/// Index into the sketch's line storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LineId(pub usize);

/// Index into the sketch's arc storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArcId(pub usize);

/// Index into the sketch's circle storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CircleId(pub usize);

/// A line segment connecting two sketch points.
#[derive(Debug, Clone, Copy)]
pub struct SketchLine {
    pub start: PointId,
    pub end: PointId,
}

/// A circular arc defined by center, radius, and angular span.
///
/// `start_point` and `end_point` are constrained to lie on the arc;
/// the solver keeps them in sync with `radius` and the angles.
#[derive(Debug, Clone, Copy)]
pub struct SketchArc {
    pub center: PointId,
    pub start_point: PointId,
    pub end_point: PointId,
    pub radius: f64,
    pub start_angle: f64,
    pub end_angle: f64,
}

/// A full circle defined by its center point and radius.
#[derive(Debug, Clone, Copy)]
pub struct SketchCircle {
    pub center: PointId,
    pub radius: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sketch_point_creation() {
        let p = SketchPoint::new(3.0, 4.0);
        assert!((p.position.x - 3.0).abs() < 1e-10);
        assert!((p.position.y - 4.0).abs() < 1e-10);
    }
}
