use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};

use super::Curve;

/// A NURBS (Non-Uniform Rational B-Spline) curve in 3D space.
#[derive(Debug, Clone)]
pub struct NurbsCurve {
    pub degree: usize,
    pub control_points: Vec<Point3>,
    pub weights: Vec<f64>,
    pub knots: Vec<f64>,
}

impl NurbsCurve {
    /// Creates a new NURBS curve.
    pub fn new(
        degree: usize,
        control_points: Vec<Point3>,
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

    /// Creates a simple Bezier curve (uniform weights, clamped knot vector).
    pub fn bezier(control_points: Vec<Point3>) -> KernelResult<Self> {
        let n = control_points.len();
        let degree = n - 1;
        let weights = vec![1.0; n];
        let mut knots = vec![0.0; degree + 1];
        knots.extend(vec![1.0; degree + 1]);
        Self::new(degree, control_points, weights, knots)
    }

    /// Finds the knot span index for parameter `t` using binary search.
    fn find_span(&self, t: f64) -> usize {
        let n = self.control_points.len();
        let p = self.degree;

        if t >= self.knots[n] {
            return n - 1;
        }
        if t <= self.knots[p] {
            return p;
        }

        let mut lo = p;
        let mut hi = n;
        let mut mid = (lo + hi) / 2;
        while t < self.knots[mid] || t >= self.knots[mid + 1] {
            if t < self.knots[mid] {
                hi = mid;
            } else {
                lo = mid;
            }
            mid = (lo + hi) / 2;
        }
        mid
    }

    /// Evaluates using the De Boor algorithm (rational).
    fn de_boor(&self, t: f64) -> Point3 {
        let p = self.degree;
        let span = self.find_span(t);

        let mut d: Vec<[f64; 4]> = (0..=p)
            .map(|j| {
                let idx = span - p + j;
                let w = self.weights[idx];
                let cp = self.control_points[idx];
                [cp.x * w, cp.y * w, cp.z * w, w]
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
                for (cur, &p) in d[j].iter_mut().zip(prev.iter()) {
                    *cur = (1.0 - alpha) * p + alpha * *cur;
                }
            }
        }

        let w = d[p][3];
        Point3::new(d[p][0] / w, d[p][1] / w, d[p][2] / w)
    }
}

impl NurbsCurve {
    /// Inserts a knot at parameter value `u` using the Boehm algorithm.
    /// The curve shape is unchanged but gains an additional control point.
    pub fn insert_knot(&self, u: f64) -> KernelResult<NurbsCurve> {
        let p = self.degree;
        let n = self.control_points.len();
        let (lo, hi) = self.domain();

        if u < lo || u > hi {
            return Err(KernelError::InvalidArgument(format!(
                "knot value {u} outside domain [{lo}, {hi}]"
            )));
        }

        let hw: Vec<[f64; 4]> = self
            .control_points
            .iter()
            .zip(self.weights.iter())
            .map(|(pt, &w)| [pt.x * w, pt.y * w, pt.z * w, w])
            .collect();

        let k = self.find_span(u);

        let mut new_knots = Vec::with_capacity(self.knots.len() + 1);
        new_knots.extend_from_slice(&self.knots[..=k]);
        new_knots.push(u);
        new_knots.extend_from_slice(&self.knots[k + 1..]);

        let mut new_hw: Vec<[f64; 4]> = Vec::with_capacity(n + 1);

        for i in 0..=n {
            if i <= k.saturating_sub(p) {
                new_hw.push(hw[i]);
            } else if i > k {
                new_hw.push(hw[i - 1]);
            } else {
                let denom = self.knots[i + p] - self.knots[i];
                let alpha = if denom.abs() < 1e-14 {
                    0.0
                } else {
                    (u - self.knots[i]) / denom
                };
                let mut blended = [0.0; 4];
                for c in 0..4 {
                    blended[c] = (1.0 - alpha) * hw[i - 1][c] + alpha * hw[i][c];
                }
                new_hw.push(blended);
            }
        }

        let new_weights: Vec<f64> = new_hw.iter().map(|h| h[3]).collect();
        let new_points: Vec<Point3> = new_hw
            .iter()
            .map(|h| {
                let w = h[3];
                Point3::new(h[0] / w, h[1] / w, h[2] / w)
            })
            .collect();

        NurbsCurve::new(p, new_points, new_weights, new_knots)
    }

    /// Elevates the degree of the curve by 1.
    /// The curve shape is unchanged but the degree increases.
    pub fn elevate_degree(&self) -> KernelResult<NurbsCurve> {
        let p = self.degree;

        let hw: Vec<[f64; 4]> = self
            .control_points
            .iter()
            .zip(self.weights.iter())
            .map(|(pt, &w)| [pt.x * w, pt.y * w, pt.z * w, w])
            .collect();

        let distinct_knots = distinct_knot_values(&self.knots);
        let n_segments = distinct_knots.len() - 1;

        let target_mult = p + 1;
        let mut curve_knots = self.knots.clone();
        let mut curve_hw = hw;
        let curve_degree = p;

        let _ = n_segments;
        for &knot_val in &distinct_knots[1..distinct_knots.len() - 1] {
            let current_mult = curve_knots
                .iter()
                .filter(|&&k| (k - knot_val).abs() < 1e-14)
                .count();
            let insertions_needed = target_mult.saturating_sub(current_mult);
            for _ in 0..insertions_needed {
                let (new_knots, new_hw) =
                    insert_knot_homogeneous(&curve_knots, &curve_hw, curve_degree, knot_val);
                curve_knots = new_knots;
                curve_hw = new_hw;
            }
        }

        let num_bezier_pts = target_mult;
        let mut elevated_segments: Vec<Vec<[f64; 4]>> = Vec::new();

        let bez_knot_spans: Vec<(f64, f64)> =
            distinct_knots.windows(2).map(|w| (w[0], w[1])).collect();

        let mut offset = 0;
        for _ in &bez_knot_spans {
            let seg: Vec<[f64; 4]> = curve_hw[offset..offset + num_bezier_pts].to_vec();
            let elevated = elevate_bezier_segment(&seg);
            elevated_segments.push(elevated);
            offset += num_bezier_pts;
        }

        let new_degree = p + 1;
        let mut new_hw: Vec<[f64; 4]> = Vec::new();

        for (seg_idx, seg) in elevated_segments.iter().enumerate() {
            if seg_idx == 0 {
                new_hw.extend_from_slice(seg);
            } else {
                new_hw.extend_from_slice(&seg[1..]);
            }
        }

        let mut new_knots = Vec::new();
        for _ in 0..=new_degree {
            new_knots.push(distinct_knots[0]);
        }
        for &knot_val in &distinct_knots[1..distinct_knots.len() - 1] {
            let orig_mult = self
                .knots
                .iter()
                .filter(|&&k| (k - knot_val).abs() < 1e-14)
                .count();
            for _ in 0..orig_mult + 1 {
                new_knots.push(knot_val);
            }
        }
        for _ in 0..=new_degree {
            new_knots.push(*distinct_knots.last().unwrap());
        }

        let expected_knots = new_hw.len() + new_degree + 1;
        while new_knots.len() < expected_knots {
            let last = *new_knots.last().unwrap();
            new_knots.insert(new_knots.len() - new_degree - 1, last);
        }
        while new_knots.len() > expected_knots {
            let remove_pos = new_knots.len() - new_degree - 2;
            new_knots.remove(remove_pos);
        }

        let new_weights: Vec<f64> = new_hw.iter().map(|h| h[3]).collect();
        let new_points: Vec<Point3> = new_hw
            .iter()
            .map(|h| {
                let w = h[3];
                Point3::new(h[0] / w, h[1] / w, h[2] / w)
            })
            .collect();

        NurbsCurve::new(new_degree, new_points, new_weights, new_knots)
    }

    /// Returns the number of control points.
    pub fn control_point_count(&self) -> usize {
        self.control_points.len()
    }

    /// Returns the degree of the curve.
    pub fn degree(&self) -> usize {
        self.degree
    }

    /// Returns the knot vector.
    pub fn knots(&self) -> &[f64] {
        &self.knots
    }

    /// Returns the weights.
    pub fn weights(&self) -> &[f64] {
        &self.weights
    }
}

fn distinct_knot_values(knots: &[f64]) -> Vec<f64> {
    let mut result = Vec::new();
    for &k in knots {
        if result
            .last()
            .is_none_or(|&last: &f64| (k - last).abs() > 1e-14)
        {
            result.push(k);
        }
    }
    result
}

fn insert_knot_homogeneous(
    knots: &[f64],
    hw: &[[f64; 4]],
    degree: usize,
    u: f64,
) -> (Vec<f64>, Vec<[f64; 4]>) {
    let n = hw.len();
    let p = degree;

    let mut k = p;
    for i in p..knots.len() - 1 {
        if u >= knots[i] && u < knots[i + 1] {
            k = i;
            break;
        }
    }
    if u >= knots[n] {
        k = n - 1;
    }

    let mut new_knots = Vec::with_capacity(knots.len() + 1);
    new_knots.extend_from_slice(&knots[..=k]);
    new_knots.push(u);
    new_knots.extend_from_slice(&knots[k + 1..]);

    let mut new_hw = Vec::with_capacity(n + 1);
    for i in 0..=n {
        if i <= k.saturating_sub(p) {
            new_hw.push(hw[i]);
        } else if i > k {
            new_hw.push(hw[i - 1]);
        } else {
            let denom = knots[i + p] - knots[i];
            let alpha = if denom.abs() < 1e-14 {
                0.0
            } else {
                (u - knots[i]) / denom
            };
            let mut blended = [0.0; 4];
            for c in 0..4 {
                blended[c] = (1.0 - alpha) * hw[i - 1][c] + alpha * hw[i][c];
            }
            new_hw.push(blended);
        }
    }

    (new_knots, new_hw)
}

fn elevate_bezier_segment(hw: &[[f64; 4]]) -> Vec<[f64; 4]> {
    let p = hw.len() - 1;
    let new_p = p + 1;
    let mut result = vec![[0.0; 4]; new_p + 1];

    for i in 0..=new_p {
        let t = i as f64 / new_p as f64;
        if i == 0 {
            result[i] = hw[0];
        } else if i == new_p {
            result[i] = hw[p];
        } else {
            for c in 0..4 {
                result[i][c] = t * hw[i - 1][c] + (1.0 - t) * hw[i][c];
            }
        }
    }

    result
}

impl Curve for NurbsCurve {
    fn point_at(&self, t: f64) -> Point3 {
        self.de_boor(t)
    }

    fn tangent_at(&self, t: f64) -> Vec3 {
        let dt = 1e-6;
        let (lo, hi) = self.domain();
        let t0 = (t - dt).max(lo);
        let t1 = (t + dt).min(hi);
        if (t1 - t0).abs() < 1e-14 {
            return Vec3::ZERO;
        }
        let p0 = self.point_at(t0);
        let p1 = self.point_at(t1);
        let diff = p1 - p0;
        diff * (1.0 / (t1 - t0))
    }

    fn domain(&self) -> (f64, f64) {
        let p = self.degree;
        (self.knots[p], self.knots[self.knots.len() - 1 - p])
    }

    fn length(&self) -> f64 {
        let (lo, hi) = self.domain();
        let steps = 128;
        let dt = (hi - lo) / steps as f64;
        let mut total = 0.0;
        let mut prev = self.point_at(lo);
        for i in 1..=steps {
            let t = lo + dt * i as f64;
            let curr = self.point_at(t);
            total += prev.distance_to(curr);
            prev = curr;
        }
        total
    }

    fn is_closed(&self) -> bool {
        let (Some(first), Some(last)) = (self.control_points.first(), self.control_points.last())
        else {
            return false;
        };
        first.approx_eq(*last)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_math::EPSILON;

    #[test]
    fn test_bezier_line() {
        let curve =
            NurbsCurve::bezier(vec![Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)])
                .unwrap();
        assert!(curve.point_at(0.0).approx_eq(Point3::ORIGIN));
        assert!(curve.point_at(1.0).approx_eq(Point3::new(1.0, 0.0, 0.0)));
        assert!(curve.point_at(0.5).approx_eq(Point3::new(0.5, 0.0, 0.0)));
    }

    #[test]
    fn test_bezier_quadratic() {
        let curve = NurbsCurve::bezier(vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.5, 1.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
        ])
        .unwrap();
        let mid = curve.point_at(0.5);
        assert!((mid.x - 0.5).abs() < EPSILON);
        assert!((mid.y - 0.5).abs() < EPSILON);
    }

    #[test]
    fn test_nurbs_length_straight() {
        let curve =
            NurbsCurve::bezier(vec![Point3::new(0.0, 0.0, 0.0), Point3::new(3.0, 4.0, 0.0)])
                .unwrap();
        assert!((curve.length() - 5.0).abs() < 1e-4);
    }

    #[test]
    fn test_knot_insertion_preserves_shape() {
        let curve = NurbsCurve::new(
            2,
            vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 2.0, 0.0),
                Point3::new(3.0, 2.0, 0.0),
                Point3::new(4.0, 0.0, 0.0),
            ],
            vec![1.0, 1.0, 1.0, 1.0],
            vec![0.0, 0.0, 0.0, 0.5, 1.0, 1.0, 1.0],
        )
        .unwrap();

        let inserted = curve.insert_knot(0.25).unwrap();
        let (lo, hi) = curve.domain();

        for i in 0..=20 {
            let t = lo + (hi - lo) * i as f64 / 20.0;
            let orig = curve.point_at(t);
            let after = inserted.point_at(t);
            assert!(
                orig.distance_to(after) < 1e-10,
                "mismatch at t={t}: orig={orig:?}, after={after:?}"
            );
        }
    }

    #[test]
    fn test_knot_insertion_adds_control_point() {
        let curve = NurbsCurve::bezier(vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        ])
        .unwrap();

        let before_count = curve.control_point_count();
        let inserted = curve.insert_knot(0.5).unwrap();
        assert_eq!(inserted.control_point_count(), before_count + 1);
    }

    #[test]
    fn test_degree_elevation_preserves_shape() {
        let curve = NurbsCurve::bezier(vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 2.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        ])
        .unwrap();

        let elevated = curve.elevate_degree().unwrap();
        let (lo, hi) = curve.domain();

        for i in 0..=20 {
            let t = lo + (hi - lo) * i as f64 / 20.0;
            let orig = curve.point_at(t);
            let after = elevated.point_at(t);
            assert!(
                orig.distance_to(after) < 1e-8,
                "mismatch at t={t}: orig={orig:?}, after={after:?}"
            );
        }
    }

    #[test]
    fn test_degree_elevation_increases_degree() {
        let curve = NurbsCurve::bezier(vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        ])
        .unwrap();

        let before_degree = curve.degree();
        let elevated = curve.elevate_degree().unwrap();
        assert_eq!(elevated.degree(), before_degree + 1);
    }

    #[test]
    fn test_knot_insertion_boundary_error() {
        let curve = NurbsCurve::bezier(vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        ])
        .unwrap();

        assert!(curve.insert_knot(-0.5).is_err());
        assert!(curve.insert_knot(1.5).is_err());
    }
}
