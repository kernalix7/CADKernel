//! 2D parametric curves for UV-space representation (trim boundaries, pcurves).
//!
//! - [`Curve2D`] trait — parametric curve in 2D.
//! - [`Line2D`], [`Circle2D`], [`NurbsCurve2D`] — concrete implementations.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point2, Vec2};

use super::bspline_basis;

/// A parametric curve in 2D space evaluated over parameter `t`.
pub trait Curve2D: Send + Sync {
    /// Evaluates the curve at parameter `t`.
    fn point_at(&self, t: f64) -> Point2;

    /// Evaluates the tangent at parameter `t`.
    fn tangent_at(&self, t: f64) -> Vec2;

    /// The valid parameter range `(t_min, t_max)`.
    fn domain(&self) -> (f64, f64);

    /// Whether the curve forms a closed loop.
    fn is_closed(&self) -> bool {
        false
    }
}

/// A 2D line segment from `start` to `end`, parameterized over [0, 1].
#[derive(Debug, Clone, Copy)]
pub struct Line2D {
    pub start: Point2,
    pub end: Point2,
}

impl Line2D {
    pub fn new(start: Point2, end: Point2) -> Self {
        Self { start, end }
    }
}

impl Curve2D for Line2D {
    fn point_at(&self, t: f64) -> Point2 {
        Point2::new(
            self.start.x + t * (self.end.x - self.start.x),
            self.start.y + t * (self.end.y - self.start.y),
        )
    }

    fn tangent_at(&self, _t: f64) -> Vec2 {
        Vec2::new(self.end.x - self.start.x, self.end.y - self.start.y)
    }

    fn domain(&self) -> (f64, f64) {
        (0.0, 1.0)
    }
}

/// A 2D circle (or circular arc) centered at `center` with `radius`.
/// Parameterized from `start_angle` to `end_angle`.
#[derive(Debug, Clone, Copy)]
pub struct Circle2D {
    pub center: Point2,
    pub radius: f64,
    pub start_angle: f64,
    pub end_angle: f64,
}

impl Circle2D {
    pub fn new(center: Point2, radius: f64, start_angle: f64, end_angle: f64) -> Self {
        Self {
            center,
            radius,
            start_angle,
            end_angle,
        }
    }

    /// Full circle.
    pub fn full(center: Point2, radius: f64) -> Self {
        Self::new(center, radius, 0.0, std::f64::consts::TAU)
    }

    fn angle_at(&self, t: f64) -> f64 {
        let (lo, hi) = self.domain();
        let frac = (t - lo) / (hi - lo);
        self.start_angle + frac * (self.end_angle - self.start_angle)
    }
}

impl Curve2D for Circle2D {
    fn point_at(&self, t: f64) -> Point2 {
        let a = self.angle_at(t);
        Point2::new(
            self.center.x + self.radius * a.cos(),
            self.center.y + self.radius * a.sin(),
        )
    }

    fn tangent_at(&self, t: f64) -> Vec2 {
        let a = self.angle_at(t);
        let span = self.end_angle - self.start_angle;
        Vec2::new(-self.radius * a.sin() * span, self.radius * a.cos() * span)
    }

    fn domain(&self) -> (f64, f64) {
        (0.0, 1.0)
    }

    fn is_closed(&self) -> bool {
        ((self.end_angle - self.start_angle).abs() - std::f64::consts::TAU).abs() < 1e-10
    }
}

/// A 2D NURBS curve (rational B-spline) in parameter space.
#[derive(Debug, Clone)]
pub struct NurbsCurve2D {
    pub degree: usize,
    pub control_points: Vec<Point2>,
    pub weights: Vec<f64>,
    pub knots: Vec<f64>,
}

impl NurbsCurve2D {
    pub fn new(
        degree: usize,
        control_points: Vec<Point2>,
        weights: Vec<f64>,
        knots: Vec<f64>,
    ) -> KernelResult<Self> {
        let n = control_points.len();
        if weights.len() != n {
            return Err(KernelError::InvalidArgument(format!(
                "weights.len() ({}) must equal control_points.len() ({})",
                weights.len(),
                n
            )));
        }
        if knots.len() != n + degree + 1 {
            return Err(KernelError::InvalidArgument(format!(
                "knots.len() ({}) must equal n + degree + 1 ({})",
                knots.len(),
                n + degree + 1
            )));
        }
        Ok(Self {
            degree,
            control_points,
            weights,
            knots,
        })
    }

    fn find_span(&self, t: f64) -> usize {
        bspline_basis::find_span(&self.knots, self.control_points.len(), self.degree, t)
    }

    fn de_boor(&self, t: f64) -> Point2 {
        let p = self.degree;
        let span = self.find_span(t);

        let mut d: Vec<[f64; 3]> = (0..=p)
            .map(|j| {
                let idx = span - p + j;
                let w = self.weights[idx];
                let cp = self.control_points[idx];
                [cp.x * w, cp.y * w, w]
            })
            .collect();

        for r in 1..=p {
            for j in (r..=p).rev() {
                let idx = span - p + j;
                let denom = self.knots[idx + p + 1 - r] - self.knots[idx];
                let alpha = if denom.abs() < 1e-14 {
                    0.0
                } else {
                    (t - self.knots[idx]) / denom
                };
                let prev = d[j - 1];
                for (k, &pv) in d[j].iter_mut().zip(prev.iter()) {
                    *k = (1.0 - alpha) * pv + alpha * *k;
                }
            }
        }

        let w = d[p][2];
        if w.abs() < 1e-14 {
            return Point2::new(d[p][0], d[p][1]);
        }
        Point2::new(d[p][0] / w, d[p][1] / w)
    }
}

impl Curve2D for NurbsCurve2D {
    fn point_at(&self, t: f64) -> Point2 {
        self.de_boor(t)
    }

    fn tangent_at(&self, t: f64) -> Vec2 {
        // Finite difference for simplicity
        let (lo, hi) = self.domain();
        let h = 1e-7;
        let ta = (t - h).max(lo);
        let tb = (t + h).min(hi);
        let dt = tb - ta;
        if dt.abs() < 1e-14 {
            return Vec2::ZERO;
        }
        let pa = self.point_at(ta);
        let pb = self.point_at(tb);
        Vec2::new((pb.x - pa.x) / dt, (pb.y - pa.y) / dt)
    }

    fn domain(&self) -> (f64, f64) {
        let p = self.degree;
        (self.knots[p], self.knots[self.knots.len() - 1 - p])
    }

    fn is_closed(&self) -> bool {
        if let (Some(first), Some(last)) = (
            self.control_points.first(),
            self.control_points.last(),
        ) {
            first.distance_to(*last) < 1e-10
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_2;

    #[test]
    fn test_line2d() {
        let line = Line2D::new(Point2::new(0.0, 0.0), Point2::new(3.0, 4.0));
        assert!(line.point_at(0.0).distance_to(Point2::new(0.0, 0.0)) < 1e-10);
        assert!(line.point_at(1.0).distance_to(Point2::new(3.0, 4.0)) < 1e-10);
        assert!(line.point_at(0.5).distance_to(Point2::new(1.5, 2.0)) < 1e-10);
    }

    #[test]
    fn test_circle2d() {
        let c = Circle2D::full(Point2::ORIGIN, 1.0);
        assert!(c.point_at(0.0).distance_to(Point2::new(1.0, 0.0)) < 1e-10);
        assert!(c.point_at(0.25).distance_to(Point2::new(0.0, 1.0)) < 1e-10);
        assert!(c.is_closed());
    }

    #[test]
    fn test_circle2d_arc() {
        let c = Circle2D::new(Point2::ORIGIN, 1.0, 0.0, FRAC_PI_2);
        assert!(c.point_at(0.0).distance_to(Point2::new(1.0, 0.0)) < 1e-10);
        assert!(c.point_at(1.0).distance_to(Point2::new(0.0, 1.0)) < 1e-10);
        assert!(!c.is_closed());
    }

    #[test]
    fn test_nurbs2d() {
        // Degree-1 line from (0,0) to (1,1)
        let c = NurbsCurve2D::new(
            1,
            vec![Point2::new(0.0, 0.0), Point2::new(1.0, 1.0)],
            vec![1.0, 1.0],
            vec![0.0, 0.0, 1.0, 1.0],
        )
        .unwrap();

        assert!(c.point_at(0.0).distance_to(Point2::new(0.0, 0.0)) < 1e-10);
        assert!(c.point_at(1.0).distance_to(Point2::new(1.0, 1.0)) < 1e-10);
        assert!(c.point_at(0.5).distance_to(Point2::new(0.5, 0.5)) < 1e-10);
    }

    #[test]
    fn test_nurbs2d_quadratic() {
        // Quadratic Bezier: (0,0), (0.5,1), (1,0) — parabolic arc
        let c = NurbsCurve2D::new(
            2,
            vec![
                Point2::new(0.0, 0.0),
                Point2::new(0.5, 1.0),
                Point2::new(1.0, 0.0),
            ],
            vec![1.0, 1.0, 1.0],
            vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        )
        .unwrap();

        let mid = c.point_at(0.5);
        assert!((mid.x - 0.5).abs() < 1e-10);
        assert!((mid.y - 0.5).abs() < 1e-10);
    }
}
