//! A trimmed curve wrapper that restricts a base curve to a sub-domain.

use std::sync::Arc;

use cadkernel_math::{Point3, Vec3};

use super::Curve;

/// A curve that restricts a base curve to the parameter range `[t_start, t_end]`.
///
/// The trimmed curve is re-parameterized to `[0, 1]`:
/// - `point_at(0.0)` evaluates the base curve at `t_start`
/// - `point_at(1.0)` evaluates the base curve at `t_end`
#[derive(Debug, Clone)]
pub struct TrimmedCurve {
    pub base: Arc<dyn Curve>,
    pub t_start: f64,
    pub t_end: f64,
}

impl std::fmt::Debug for dyn Curve {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Curve(domain={:?})", self.domain())
    }
}

impl TrimmedCurve {
    /// Creates a new trimmed curve.
    ///
    /// `t_start` and `t_end` must be within the base curve's domain.
    pub fn new(base: Arc<dyn Curve>, t_start: f64, t_end: f64) -> Self {
        Self {
            base,
            t_start,
            t_end,
        }
    }

    /// Maps the trimmed parameter `t ∈ [0, 1]` to the base curve parameter.
    fn map_t(&self, t: f64) -> f64 {
        self.t_start + t * (self.t_end - self.t_start)
    }
}

impl Curve for TrimmedCurve {
    fn point_at(&self, t: f64) -> Point3 {
        self.base.point_at(self.map_t(t))
    }

    fn tangent_at(&self, t: f64) -> Vec3 {
        // Chain rule: d/dt_trimmed = (t_end - t_start) * d/dt_base
        self.base.tangent_at(self.map_t(t)) * (self.t_end - self.t_start)
    }

    fn domain(&self) -> (f64, f64) {
        (0.0, 1.0)
    }

    fn length(&self) -> f64 {
        // Numerical integration via sampling
        let steps = 64;
        let dt = 1.0 / steps as f64;
        let mut total = 0.0;
        let mut prev = self.point_at(0.0);
        for i in 1..=steps {
            let curr = self.point_at(dt * i as f64);
            total += prev.distance_to(curr);
            prev = curr;
        }
        total
    }

    fn is_closed(&self) -> bool {
        let start = self.point_at(0.0);
        let end = self.point_at(1.0);
        start.approx_eq(end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::line::LineSegment;

    #[test]
    fn test_trimmed_line() {
        let line = Arc::new(LineSegment::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(10.0, 0.0, 0.0),
        ));
        // Trim to [0.2, 0.7] of the base domain → x ∈ [2, 7]
        let trimmed = TrimmedCurve::new(line, 0.2, 0.7);
        assert!(trimmed.point_at(0.0).distance_to(Point3::new(2.0, 0.0, 0.0)) < 1e-10);
        assert!(trimmed.point_at(1.0).distance_to(Point3::new(7.0, 0.0, 0.0)) < 1e-10);
        assert!(trimmed.point_at(0.5).distance_to(Point3::new(4.5, 0.0, 0.0)) < 1e-10);
    }

    #[test]
    fn test_trimmed_circle_arc() {
        use crate::curve::circle::Circle;
        use std::f64::consts::FRAC_PI_2;

        let circle = Arc::new(Circle::xy(Point3::ORIGIN, 1.0));
        // Trim to first quadrant [0, π/2]
        let trimmed = TrimmedCurve::new(circle, 0.0, FRAC_PI_2);
        assert!(trimmed.point_at(0.0).distance_to(Point3::new(1.0, 0.0, 0.0)) < 1e-10);
        assert!(trimmed.point_at(1.0).distance_to(Point3::new(0.0, 1.0, 0.0)) < 1e-10);

        // All points should be on the unit circle
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            let p = trimmed.point_at(t);
            let r = (p.x * p.x + p.y * p.y).sqrt();
            assert!((r - 1.0).abs() < 1e-10);
        }
    }
}
