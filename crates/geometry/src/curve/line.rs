use cadkernel_math::{Point3, Vec3};

use super::Curve;

/// An infinite line defined by origin + direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line {
    pub origin: Point3,
    pub direction: Vec3,
}

/// A bounded line segment from `start` to `end`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineSegment {
    pub start: Point3,
    pub end: Point3,
}

impl Line {
    /// Creates a line through `origin` in the given `direction`.
    pub fn new(origin: Point3, direction: Vec3) -> Self {
        Self { origin, direction }
    }
}

impl Curve for Line {
    fn point_at(&self, t: f64) -> Point3 {
        self.origin + self.direction * t
    }

    fn tangent_at(&self, _t: f64) -> Vec3 {
        self.direction
    }

    fn domain(&self) -> (f64, f64) {
        (f64::NEG_INFINITY, f64::INFINITY)
    }

    fn length(&self) -> f64 {
        f64::INFINITY
    }

    fn is_closed(&self) -> bool {
        false
    }
}

impl LineSegment {
    /// Creates a segment from `start` to `end`, parameterised over `[0, 1]`.
    pub fn new(start: Point3, end: Point3) -> Self {
        Self { start, end }
    }
}

impl Curve for LineSegment {
    fn point_at(&self, t: f64) -> Point3 {
        let d = self.end - self.start;
        self.start + d * t
    }

    fn tangent_at(&self, _t: f64) -> Vec3 {
        self.end - self.start
    }

    fn domain(&self) -> (f64, f64) {
        (0.0, 1.0)
    }

    fn length(&self) -> f64 {
        self.start.distance_to(self.end)
    }

    fn is_closed(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_math::EPSILON;

    #[test]
    fn test_line_segment_midpoint() {
        let seg = LineSegment::new(Point3::ORIGIN, Point3::new(2.0, 0.0, 0.0));
        let mid = seg.point_at(0.5);
        assert!(mid.approx_eq(Point3::new(1.0, 0.0, 0.0)));
    }

    #[test]
    fn test_line_segment_length() {
        let seg = LineSegment::new(Point3::ORIGIN, Point3::new(3.0, 4.0, 0.0));
        assert!((seg.length() - 5.0).abs() < EPSILON);
    }

    #[test]
    fn test_line_point_at() {
        let line = Line::new(Point3::ORIGIN, Vec3::X);
        assert!(line.point_at(5.0).approx_eq(Point3::new(5.0, 0.0, 0.0)));
    }
}
