pub mod arc;
pub mod blend;
pub mod bspline_basis;
pub mod circle;
pub mod curve2d;
pub mod ellipse;
pub mod line;
pub mod nurbs;
pub mod nurbs_fitting;
pub mod offset_curve;
pub mod to_nurbs;
pub mod trimmed;

use cadkernel_math::{BoundingBox, Point3, Vec3};

const FINITE_DIFF_H: f64 = 1e-6;
const NUMERIC_ZERO: f64 = 1e-14;
const PROJECT_SAMPLES: usize = 64;
const NEWTON_ITERS: usize = 10;
const BBOX_SAMPLES: usize = 32;

/// A parametric curve in 3D space evaluated over a parameter `t`.
///
/// All implementations must be `Send + Sync` to allow safe usage
/// inside `Arc<dyn Curve>` across threads.
pub trait Curve: Send + Sync {
    /// Evaluates the curve at parameter `t`.
    fn point_at(&self, t: f64) -> Point3;

    /// Evaluates the tangent (first derivative) at parameter `t`.
    fn tangent_at(&self, t: f64) -> Vec3;

    /// The valid parameter range `(t_min, t_max)`.
    fn domain(&self) -> (f64, f64);

    /// Approximate arc length of the curve.
    fn length(&self) -> f64;

    /// Whether the curve forms a closed loop.
    fn is_closed(&self) -> bool;

    /// Second derivative (acceleration) at parameter `t`.
    /// Default: central finite difference.
    fn second_derivative_at(&self, t: f64) -> Vec3 {
        let (t0, t1) = self.domain();
        let ta = (t - FINITE_DIFF_H).max(t0);
        let tb = (t + FINITE_DIFF_H).min(t1);
        let dt = tb - ta;
        if dt.abs() < NUMERIC_ZERO {
            return Vec3::ZERO;
        }
        let d1a = self.tangent_at(ta);
        let d1b = self.tangent_at(tb);
        (d1b - d1a) / dt
    }

    /// Curvature magnitude at parameter `t`: |T'(t)| / |r'(t)|.
    fn curvature_at(&self, t: f64) -> f64 {
        let d1 = self.tangent_at(t);
        let d2 = self.second_derivative_at(t);
        let cross = d1.cross(d2);
        let d1_len = d1.length();
        if d1_len < NUMERIC_ZERO {
            return 0.0;
        }
        cross.length() / (d1_len * d1_len * d1_len)
    }

    /// Returns a curve with reversed parameterisation.
    /// Default: not supported (returns `None`).
    fn reversed(&self) -> Option<Box<dyn Curve>> {
        None
    }

    /// Projects a 3D point onto the curve, returning `(t, closest_point)`.
    /// Default: brute-force sampling with Newton refinement.
    fn project_point(&self, point: Point3) -> (f64, Point3) {
        let (t0, t1) = self.domain();
        let mut best_t = t0;
        let mut best_dist = f64::MAX;
        for i in 0..=PROJECT_SAMPLES {
            let t = t0 + (t1 - t0) * i as f64 / PROJECT_SAMPLES as f64;
            let p = self.point_at(t);
            let d = point.distance_to(p);
            if d < best_dist {
                best_dist = d;
                best_t = t;
            }
        }
        // Newton refinement
        for _ in 0..NEWTON_ITERS {
            let p = self.point_at(best_t);
            let d1 = self.tangent_at(best_t);
            let diff = p - point;
            let denom = d1.dot(d1);
            if denom.abs() < NUMERIC_ZERO {
                break;
            }
            let dt = Vec3::new(diff.x, diff.y, diff.z).dot(d1) / denom;
            best_t = (best_t - dt).clamp(t0, t1);
        }
        (best_t, self.point_at(best_t))
    }

    /// Axis-aligned bounding box of the curve (sampled).
    fn bounding_box(&self) -> BoundingBox {
        let (t0, t1) = self.domain();
        let first = self.point_at(t0);
        let mut bb = BoundingBox::new(first, first);
        for i in 1..=BBOX_SAMPLES {
            let t = t0 + (t1 - t0) * i as f64 / BBOX_SAMPLES as f64;
            bb.include_point(self.point_at(t));
        }
        bb
    }
}
