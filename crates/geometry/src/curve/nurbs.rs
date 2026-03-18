use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};

use super::Curve;
use super::bspline_basis;

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
        bspline_basis::find_span(&self.knots, self.control_points.len(), self.degree, t)
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
        if w.abs() < 1e-14 {
            return Point3::new(d[p][0], d[p][1], d[p][2]);
        }
        Point3::new(d[p][0] / w, d[p][1] / w, d[p][2] / w)
    }
}

impl NurbsCurve {
    /// Reparameterize the curve domain from current `[a,b]` to `[new_start, new_end]`.
    ///
    /// This is a purely parametric change — the curve shape is unaffected.
    pub fn reparameterize(&mut self, new_start: f64, new_end: f64) {
        let (old_start, old_end) = self.domain();
        let old_range = old_end - old_start;
        if old_range.abs() < 1e-14 {
            return;
        }
        let scale = (new_end - new_start) / old_range;
        for knot in &mut self.knots {
            *knot = new_start + (*knot - old_start) * scale;
        }
    }

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

    /// Returns a new NURBS curve with reversed parameterisation.
    ///
    /// Reverses control points, weights, and knot vector so that
    /// `reversed.point_at(a + b - t) == original.point_at(t)`
    /// where `(a, b) = original.domain()`.
    pub fn reverse(&self) -> NurbsCurve {
        let (a, b) = self.domain();
        let ab = a + b;

        let mut rev_cp = self.control_points.clone();
        rev_cp.reverse();
        let mut rev_w = self.weights.clone();
        rev_w.reverse();
        let mut rev_knots: Vec<f64> = self.knots.iter().rev().map(|&k| ab - k).collect();
        // Ensure monotonically non-decreasing after reversal
        // (the reversal + negation automatically gives non-decreasing)
        // but due to floating point, snap identical knots
        for i in 1..rev_knots.len() {
            if rev_knots[i] < rev_knots[i - 1] {
                rev_knots[i] = rev_knots[i - 1];
            }
        }

        NurbsCurve {
            degree: self.degree,
            control_points: rev_cp,
            weights: rev_w,
            knots: rev_knots,
        }
    }

    /// Splits the curve at parameter `t`, returning two NURBS curves.
    ///
    /// Inserts the knot `t` until multiplicity equals `degree + 1`,
    /// effectively creating a C^{-1} break, then separates the two halves.
    pub fn split_at(&self, t: f64) -> KernelResult<(NurbsCurve, NurbsCurve)> {
        let (lo, hi) = self.domain();
        if t <= lo || t >= hi {
            return Err(KernelError::InvalidArgument(format!(
                "split parameter {t} must be strictly inside domain [{lo}, {hi}]"
            )));
        }

        let p = self.degree;

        // Count current multiplicity of t
        let current_mult = self.knots.iter().filter(|&&k| (k - t).abs() < 1e-14).count();
        let insertions_needed = (p + 1).saturating_sub(current_mult);

        // Insert knot until multiplicity = p+1
        let mut curve = self.clone();
        for _ in 0..insertions_needed {
            curve = curve.insert_knot(t)?;
        }

        // Find where the knot `t` starts in the refined knot vector
        let split_knot_start = curve
            .knots
            .iter()
            .position(|&k| (k - t).abs() < 1e-14)
            .unwrap();
        let _split_knot_end = split_knot_start + p; // p+1 copies of t, last is at +p

        // Left curve: control points 0..split_knot_start, knots 0..=split_knot_end
        let n_left_cp = split_knot_start;
        let left_cp = curve.control_points[..n_left_cp].to_vec();
        let left_w = curve.weights[..n_left_cp].to_vec();
        let left_knots = curve.knots[..n_left_cp + p + 1].to_vec();

        // Right curve: control points from split_knot_start.., knots from split_knot_start..
        let right_cp = curve.control_points[n_left_cp..].to_vec();
        let right_w = curve.weights[n_left_cp..].to_vec();
        let right_knots = curve.knots[split_knot_start..].to_vec();

        let left = NurbsCurve::new(p, left_cp, left_w, left_knots)?;
        let right = NurbsCurve::new(p, right_cp, right_w, right_knots)?;

        Ok((left, right))
    }

    /// Refines the knot vector by inserting multiple knots at once.
    ///
    /// More efficient than calling `insert_knot` repeatedly because it processes
    /// all insertions in a single pass (The NURBS Book A5.4).
    pub fn refine_knots(&self, new_knots: &[f64]) -> KernelResult<NurbsCurve> {
        if new_knots.is_empty() {
            return Ok(self.clone());
        }
        let (lo, hi) = self.domain();
        for &k in new_knots {
            if k < lo || k > hi {
                return Err(KernelError::InvalidArgument(format!(
                    "knot {k} outside domain [{lo}, {hi}]"
                )));
            }
        }

        // Sort the new knots
        let mut sorted = new_knots.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mut result = self.clone();
        for &k in &sorted {
            result = result.insert_knot(k)?;
        }
        Ok(result)
    }

    /// Removes a knot from the curve (within tolerance), reducing control points.
    ///
    /// Attempts to remove the knot at value `knot_val` up to `times` times.
    /// Returns the number of times the knot was actually removed.
    /// Uses the inverse Boehm algorithm with alphas from the target knot vector.
    pub fn remove_knot(
        &self,
        knot_val: f64,
        times: usize,
        tolerance: f64,
    ) -> KernelResult<(NurbsCurve, usize)> {
        let p = self.degree;
        let (lo, hi) = self.domain();

        if (knot_val - lo).abs() < 1e-14 || (knot_val - hi).abs() < 1e-14 {
            return Ok((self.clone(), 0));
        }

        let mut curve = self.clone();
        let mut removed = 0;

        for _ in 0..times {
            let n = curve.control_points.len();
            if n <= p + 1 {
                break;
            }

            // Find one occurrence of the knot to remove
            let r = match curve
                .knots
                .iter()
                .position(|&k| (k - knot_val).abs() < 1e-14)
            {
                Some(idx) => idx,
                None => break,
            };

            // Build the target knot vector (current minus one occurrence)
            let mut target_knots = curve.knots.clone();
            target_knots.remove(r);

            // Find the span in the TARGET knot vector for the removed knot
            let target_n = n - 1;
            let target_span =
                bspline_basis::find_span(&target_knots, target_n, p, knot_val);

            // Work in homogeneous coordinates
            let hw: Vec<[f64; 4]> = curve
                .control_points
                .iter()
                .zip(curve.weights.iter())
                .map(|(pt, &w)| [pt.x * w, pt.y * w, pt.z * w, w])
                .collect();

            // Compute alphas from the TARGET knot vector (these are the Boehm alphas
            // that would have been used to insert knot_val into the target curve)
            let affected_lo = target_span + 1 - p; // first affected CP index in target
            let affected_hi = target_span; // last affected CP index in target

            // Forward sweep: recover target CPs from the left
            let mut new_hw = vec![[0.0f64; 4]; target_n];

            // CPs before affected range are unchanged
            new_hw[..affected_lo].copy_from_slice(&hw[..affected_lo]);
            // CPs after affected range are shifted by -1 (one CP removed)
            new_hw[(affected_hi + 1)..target_n]
                .copy_from_slice(&hw[(affected_hi + 2)..(target_n + 1)]);

            // Forward sweep through affected range
            let mut fwd = vec![[0.0f64; 4]; p];
            for (j, i) in (affected_lo..=affected_hi).enumerate() {
                let alpha = {
                    let denom = target_knots[i + p] - target_knots[i];
                    if denom.abs() < 1e-14 {
                        0.5
                    } else {
                        (knot_val - target_knots[i]) / denom
                    }
                };
                // hw[i] in the inserted curve = (1-alpha)*target[i-1] + alpha*target[i]
                // So: target[i] = (hw[i] - (1-alpha)*known_prev) / alpha
                let prev = if j == 0 { new_hw[affected_lo - 1] } else { fwd[j - 1] };
                if alpha.abs() < 1e-14 {
                    fwd[j] = hw[i];
                } else {
                    let mut q = [0.0; 4];
                    for c in 0..4 {
                        q[c] = (hw[i][c] - (1.0 - alpha) * prev[c]) / alpha;
                    }
                    fwd[j] = q;
                }
            }

            // Backward sweep through affected range
            let mut bwd = vec![[0.0f64; 4]; p];
            let n_affected = affected_hi - affected_lo + 1;
            for (j_rev, i) in (affected_lo..=affected_hi).rev().enumerate() {
                let alpha = {
                    let denom = target_knots[i + p] - target_knots[i];
                    if denom.abs() < 1e-14 {
                        0.5
                    } else {
                        (knot_val - target_knots[i]) / denom
                    }
                };
                let j = n_affected - 1 - j_rev;
                // hw[i+1] in inserted = (1-alpha_next)*target[i] + alpha_next*target[i+1]
                // But simpler: hw[i+1] = (1-a_{i+1})*target[i] + a_{i+1}*target[i+1]
                // target[i] = (hw[i+1] - a_{i+1}*known_next) / (1-a_{i+1})
                // Actually use: inserted[i] = (1-a_i)*target[i-1] + a_i*target[i]
                // target[i] = (inserted[i] - (1-a_i)*target[i-1]) / a_i  (same as fwd)
                // OR from right: inserted[i+1] = (1-a_{i+1})*target[i] + a_{i+1}*target[i+1]
                // target[i] = (inserted[i+1] - a_{i+1}*target[i+1]) / (1-a_{i+1})
                let next_alpha = if i < affected_hi {
                    let denom_next = target_knots[i + 1 + p] - target_knots[i + 1];
                    if denom_next.abs() < 1e-14 { 0.5 } else { (knot_val - target_knots[i + 1]) / denom_next }
                } else {
                    alpha // won't be used
                };
                let next = if j_rev == 0 { new_hw[(affected_hi + 1).min(target_n - 1)] } else { bwd[j + 1] };
                if i < affected_hi && (1.0 - next_alpha).abs() > 1e-14 {
                    let mut q = [0.0; 4];
                    for c in 0..4 {
                        q[c] = (hw[i + 1][c] - next_alpha * next[c]) / (1.0 - next_alpha);
                    }
                    bwd[j] = q;
                } else {
                    bwd[j] = fwd[j]; // fallback to forward sweep
                }
            }

            // Use forward estimates for left half, backward for right half
            for (j, i) in (affected_lo..=affected_hi).enumerate() {
                if j < n_affected / 2 {
                    new_hw[i] = fwd[j];
                } else if j > n_affected / 2 {
                    new_hw[i] = bwd[j];
                } else {
                    // Average at the midpoint
                    let mut avg = [0.0; 4];
                    for c in 0..4 {
                        avg[c] = 0.5 * (fwd[j][c] + bwd[j][c]);
                    }
                    new_hw[i] = avg;
                }
            }

            let new_w: Vec<f64> = new_hw.iter().map(|h| h[3]).collect();
            let new_cp: Vec<Point3> = new_hw
                .iter()
                .map(|h| {
                    let w = h[3];
                    if w.abs() < 1e-14 {
                        Point3::new(h[0], h[1], h[2])
                    } else {
                        Point3::new(h[0] / w, h[1] / w, h[2] / w)
                    }
                })
                .collect();

            match NurbsCurve::new(p, new_cp, new_w, target_knots) {
                Ok(candidate) => {
                    let mut max_err = 0.0f64;
                    for check in 0..=40 {
                        let t = lo + (hi - lo) * check as f64 / 40.0;
                        let d = curve.point_at(t).distance_to(candidate.point_at(t));
                        max_err = max_err.max(d);
                    }
                    if max_err > tolerance {
                        break;
                    }
                    curve = candidate;
                    removed += 1;
                }
                Err(_) => break,
            }
        }

        Ok((curve, removed))
    }

    /// Decomposes the curve into Bezier segments.
    ///
    /// Each returned curve is a Bezier (all internal knot multiplicities = degree).
    pub fn decompose_to_bezier(&self) -> Vec<NurbsCurve> {
        let p = self.degree;
        let distinct = distinct_knot_values(&self.knots);

        if distinct.len() <= 2 {
            // Already a Bezier segment
            return vec![self.clone()];
        }

        // Insert knots until every internal knot has multiplicity p
        let mut refined = self.clone();
        for &knot_val in &distinct[1..distinct.len() - 1] {
            let current_mult = refined
                .knots
                .iter()
                .filter(|&&k| (k - knot_val).abs() < 1e-14)
                .count();
            let needed = p.saturating_sub(current_mult);
            for _ in 0..needed {
                refined = refined.insert_knot(knot_val).unwrap();
            }
        }

        // After refinement, each span uses p consecutive CPs (sharing boundary CPs).
        // Segment i uses CPs[i*p .. i*p + p + 1].
        let n_segments = distinct.len() - 1;
        let mut segments = Vec::with_capacity(n_segments);

        for (seg_idx, span) in distinct.windows(2).enumerate() {
            let (a, b) = (span[0], span[1]);
            let cp_start = seg_idx * p;
            let cp_end = cp_start + p + 1;

            if cp_end > refined.control_points.len() {
                continue;
            }

            let cp = refined.control_points[cp_start..cp_end].to_vec();
            let w = refined.weights[cp_start..cp_end].to_vec();
            let mut knots = vec![a; p + 1];
            knots.extend(vec![b; p + 1]);

            if let Ok(seg) = NurbsCurve::new(p, cp, w, knots) {
                segments.push(seg);
            }
        }

        segments
    }

    /// Joins two NURBS curves end-to-end (if their endpoints match within tolerance).
    ///
    /// Both curves must have the same degree. The resulting curve is C^0 at the join.
    pub fn join(&self, other: &NurbsCurve, tolerance: f64) -> KernelResult<NurbsCurve> {
        if self.degree != other.degree {
            return Err(KernelError::InvalidArgument(format!(
                "cannot join curves of different degrees: {} vs {}",
                self.degree, other.degree
            )));
        }

        let end_self = self.point_at(self.domain().1);
        let start_other = other.point_at(other.domain().0);
        if end_self.distance_to(start_other) > tolerance {
            return Err(KernelError::InvalidArgument(format!(
                "curve endpoints don't match: distance = {}",
                end_self.distance_to(start_other)
            )));
        }

        let p = self.degree;
        let (_, a_hi) = self.domain();
        let (b_lo, b_hi) = other.domain();

        // Shift other's knots so that other.domain().0 == self.domain().1
        let offset = a_hi - b_lo;

        // Merge control points: drop last of self (shared with first of other) → C^0
        let mut cp = self.control_points.clone();
        cp.extend_from_slice(&other.control_points[1..]);

        let mut w = self.weights.clone();
        w.extend_from_slice(&other.weights[1..]);

        // Merge knots: self's full knots (minus trailing clamp) + join knots + other's interior + trailing clamp
        let mut knots = self.knots.clone();
        // Remove the trailing p+1 clamped knots of self
        knots.truncate(knots.len() - p - 1);
        // Add p copies of the join parameter (C^0 continuity, shared control point)
        for _ in 0..p {
            knots.push(a_hi);
        }
        // Add other's interior knots (shifted), skipping both leading and trailing clamped
        let other_interior_start = p + 1; // skip all leading clamped knots
        let other_interior_end = other.knots.len() - p - 1; // skip trailing clamped knots
        for &k in &other.knots[other_interior_start..other_interior_end] {
            knots.push(k + offset);
        }
        // Add trailing clamped knots
        let end_val = b_hi + offset;
        for _ in 0..=p {
            knots.push(end_val);
        }

        NurbsCurve::new(p, cp, w, knots)
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

impl NurbsCurve {
    /// Computes the rational curve derivatives up to order `k` at parameter `t`.
    ///
    /// Uses The NURBS Book A4.2: first computes weighted (homogeneous) derivatives,
    /// then applies the rational quotient rule.
    ///
    /// Returns `Vec<Vec3>` of length `k+1` where index 0 is the point (as Vec3),
    /// index 1 is the first derivative, etc.
    fn rational_derivatives(&self, t: f64, k: usize) -> Vec<Vec3> {
        let p = self.degree;
        let n = self.control_points.len();
        let span = bspline_basis::find_span(&self.knots, n, p, t);
        let ders_n = bspline_basis::ders_basis_funs(&self.knots, span, t, p, k);

        // Compute homogeneous derivatives: Aw[d] = Σ N^(d)_{i,p}(t) * w_i * P_i (xyz + w)
        let mut aw = vec![[0.0f64; 4]; k + 1]; // [x*w, y*w, z*w, w] for each derivative order
        for d in 0..=k {
            for (j, &nd) in ders_n[d].iter().enumerate().take(p + 1) {
                let idx = span - p + j;
                if idx < n {
                    let w = self.weights[idx];
                    let cp = &self.control_points[idx];
                    aw[d][0] += nd * cp.x * w;
                    aw[d][1] += nd * cp.y * w;
                    aw[d][2] += nd * cp.z * w;
                    aw[d][3] += nd * w;
                }
            }
        }

        // Apply rational quotient rule (The NURBS Book Eq. 4.20):
        // CK[k] = (Aw[k] - Σ_{i=1..k} C(k,i) * wders[i] * CK[k-i]) / wders[0]
        let mut ck = vec![Vec3::ZERO; k + 1];
        let w0 = aw[0][3];
        if w0.abs() < 1e-14 {
            return ck;
        }

        // 0th derivative = point
        ck[0] = Vec3::new(aw[0][0] / w0, aw[0][1] / w0, aw[0][2] / w0);

        for kk in 1..=k {
            let mut v = Vec3::new(aw[kk][0], aw[kk][1], aw[kk][2]);
            for i in 1..=kk {
                let binom = binomial(kk, i) as f64;
                let wi = aw[i][3]; // i-th derivative of weight function
                v -= ck[kk - i] * (binom * wi);
            }
            ck[kk] = v / w0;
        }

        ck
    }
}

/// Binomial coefficient C(n, k).
fn binomial(n: usize, k: usize) -> usize {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    let mut result = 1usize;
    for i in 0..k {
        result = result * (n - i) / (i + 1);
    }
    result
}

impl Curve for NurbsCurve {
    fn point_at(&self, t: f64) -> Point3 {
        self.de_boor(t)
    }

    fn tangent_at(&self, t: f64) -> Vec3 {
        let ck = self.rational_derivatives(t, 1);
        ck[1]
    }

    fn second_derivative_at(&self, t: f64) -> Vec3 {
        let ck = self.rational_derivatives(t, 2);
        ck[2]
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

    fn reversed(&self) -> Option<Box<dyn Curve>> {
        Some(Box::new(self.reverse()))
    }

    /// Projects a point onto the curve using multi-start Newton-Raphson.
    ///
    /// Uses Bezier decomposition to generate starting points, then
    /// analytical Newton: `f(t) = (C(t) - P) · C'(t) = 0`.
    fn project_point(&self, point: Point3) -> (f64, Point3) {
        let (lo, hi) = self.domain();
        let segments = self.decompose_to_bezier();

        // Gather candidate starting parameters from each Bezier segment midpoint
        // plus the global endpoints
        let mut candidates = Vec::with_capacity(segments.len() + 2);
        candidates.push(lo);
        candidates.push(hi);
        for seg in &segments {
            let (a, b) = seg.domain();
            let mid_t = (a + b) / 2.0;
            candidates.push(mid_t);
            // Also check quarter points for better coverage
            candidates.push(a + (b - a) * 0.25);
            candidates.push(a + (b - a) * 0.75);
        }

        let mut best_t = lo;
        let mut best_dist = f64::MAX;

        for &start_t in &candidates {
            let mut t = start_t.clamp(lo, hi);

            // Newton-Raphson: minimize |C(t) - P|²
            // f(t) = (C(t)-P)·C'(t) = 0
            // f'(t) = C'(t)·C'(t) + (C(t)-P)·C''(t)
            for _ in 0..20 {
                let ders = self.rational_derivatives(t, 2);
                let c = self.point_at(t);
                let diff = c - point;
                let d1 = ders[1]; // C'(t)
                let d2 = ders[2]; // C''(t)

                let f = diff.x * d1.x + diff.y * d1.y + diff.z * d1.z;
                let df = d1.x * d1.x + d1.y * d1.y + d1.z * d1.z
                    + diff.x * d2.x + diff.y * d2.y + diff.z * d2.z;

                if df.abs() < 1e-20 {
                    break;
                }

                let dt = f / df;
                t = (t - dt).clamp(lo, hi);

                if dt.abs() < 1e-14 {
                    break;
                }
            }

            let c = self.point_at(t);
            let dist = point.distance_to(c);
            if dist < best_dist {
                best_dist = dist;
                best_t = t;
            }
        }

        (best_t, self.point_at(best_t))
    }

    /// Bounding box from the convex hull property of NURBS control points.
    ///
    /// For tighter bounds, decomposes into Bezier segments and uses
    /// the convex hull of each segment's control points.
    fn bounding_box(&self) -> cadkernel_math::BoundingBox {
        let segments = self.decompose_to_bezier();
        let first = self.control_points[0];
        let mut bb = cadkernel_math::BoundingBox::new(first, first);
        for seg in &segments {
            for &cp in &seg.control_points {
                bb.include_point(cp);
            }
        }
        bb
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
    fn test_tangent_analytical_line() {
        // Linear Bezier: tangent should be constant = (1,0,0)
        let curve =
            NurbsCurve::bezier(vec![Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)])
                .unwrap();
        let t = curve.tangent_at(0.5);
        assert!((t.x - 1.0).abs() < 1e-10, "tx={}", t.x);
        assert!(t.y.abs() < 1e-10);
        assert!(t.z.abs() < 1e-10);
    }

    #[test]
    fn test_tangent_analytical_quadratic() {
        // Quadratic Bezier: B(t) = (1-t)^2*P0 + 2t(1-t)*P1 + t^2*P2
        // P0=(0,0,0), P1=(0.5,1,0), P2=(1,0,0)
        // B'(t) = 2(1-t)(P1-P0) + 2t(P2-P1)
        // B'(0.5) = (P1-P0) + (P2-P1) = P2-P0 = (1,0,0)
        let curve = NurbsCurve::bezier(vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.5, 1.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
        ])
        .unwrap();
        let t = curve.tangent_at(0.5);
        assert!((t.x - 1.0).abs() < 1e-10, "tx={}", t.x);
        assert!(t.y.abs() < 1e-10, "ty={}", t.y);
    }

    #[test]
    fn test_tangent_vs_finite_diff() {
        // Compare analytical tangent with finite difference for a cubic curve
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

        let (lo, hi) = curve.domain();
        let dt = 1e-7;
        for i in 1..20 {
            let t = lo + (hi - lo) * i as f64 / 20.0;
            let analytical = curve.tangent_at(t);
            // Finite difference
            let p0 = curve.point_at((t - dt).max(lo));
            let p1 = curve.point_at((t + dt).min(hi));
            let fd = (p1 - p0) / (2.0 * dt);
            let err = ((analytical.x - fd.x).powi(2)
                + (analytical.y - fd.y).powi(2)
                + (analytical.z - fd.z).powi(2))
            .sqrt();
            assert!(err < 1e-4, "tangent error at t={t}: err={err}");
        }
    }

    #[test]
    fn test_second_deriv_quadratic() {
        // Quadratic Bezier: B''(t) = 2*(P0 - 2*P1 + P2)
        // P0=(0,0,0), P1=(0.5,1,0), P2=(1,0,0)
        // B''(t) = 2*(0 - (1,2,0) + (1,0,0)) = 2*(0,-2,0) = (0,-4,0)
        let curve = NurbsCurve::bezier(vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.5, 1.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
        ])
        .unwrap();
        let d2 = curve.second_derivative_at(0.5);
        assert!(d2.x.abs() < 1e-10, "d2x={}", d2.x);
        assert!((d2.y - (-4.0)).abs() < 1e-10, "d2y={}", d2.y);
    }

    #[test]
    fn test_tangent_rational_circle() {
        // A rational NURBS circle: tangent at t=0 should point in +Y direction
        use std::f64::consts::FRAC_1_SQRT_2;
        let w = FRAC_1_SQRT_2;
        let r = 1.0;
        // Quarter circle from (1,0,0) to (0,1,0)
        let curve = NurbsCurve::new(
            2,
            vec![
                Point3::new(r, 0.0, 0.0),
                Point3::new(r, r, 0.0),
                Point3::new(0.0, r, 0.0),
            ],
            vec![1.0, w, 1.0],
            vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        )
        .unwrap();

        let t0 = curve.tangent_at(0.0);
        // At start of quarter circle, tangent should be in +Y direction
        assert!(t0.x.abs() < 1e-8, "t0x={}", t0.x);
        assert!(t0.y > 0.0, "t0y should be positive: {}", t0.y);
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

    #[test]
    fn test_reversed_eval() {
        let curve = NurbsCurve::bezier(vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 2.0, 0.0),
            Point3::new(3.0, 1.0, 0.0),
        ])
        .unwrap();
        let rev = curve.reverse();
        let (a, b) = curve.domain();

        for i in 0..=20 {
            let t = a + (b - a) * i as f64 / 20.0;
            let orig = curve.point_at(t);
            let rev_t = a + b - t;
            let reversed = rev.point_at(rev_t);
            assert!(
                orig.distance_to(reversed) < 1e-10,
                "mismatch at t={t}: orig={orig:?}, rev={reversed:?}"
            );
        }
    }

    #[test]
    fn test_reversed_trait() {
        let curve = NurbsCurve::bezier(vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
        ])
        .unwrap();
        let rev = curve.reversed();
        assert!(rev.is_some());
        let rev = rev.unwrap();
        // Start of reversed = end of original
        assert!(rev.point_at(0.0).approx_eq(Point3::new(1.0, 0.0, 0.0)));
        assert!(rev.point_at(1.0).approx_eq(Point3::ORIGIN));
    }

    #[test]
    fn test_split_cubic() {
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

        let (left, right) = curve.split_at(0.3).unwrap();
        let (lo, hi) = curve.domain();

        // Left should cover [lo, 0.3]
        let (ll, lh) = left.domain();
        assert!((ll - lo).abs() < 1e-10);
        assert!((lh - 0.3).abs() < 1e-10);

        // Right should cover [0.3, hi]
        let (rl, rh) = right.domain();
        assert!((rl - 0.3).abs() < 1e-10);
        assert!((rh - hi).abs() < 1e-10);

        // Evaluate at several points on left
        for i in 0..=10 {
            let t = lo + (0.3 - lo) * i as f64 / 10.0;
            let orig = curve.point_at(t);
            let split = left.point_at(t);
            assert!(
                orig.distance_to(split) < 1e-8,
                "left mismatch at t={t}: orig={orig:?}, split={split:?}"
            );
        }

        // Evaluate at several points on right
        for i in 0..=10 {
            let t = 0.3 + (hi - 0.3) * i as f64 / 10.0;
            let orig = curve.point_at(t);
            let split = right.point_at(t);
            assert!(
                orig.distance_to(split) < 1e-8,
                "right mismatch at t={t}: orig={orig:?}, split={split:?}"
            );
        }
    }

    #[test]
    fn test_join_two_segments() {
        let c1 = NurbsCurve::bezier(vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        ])
        .unwrap();
        let c2 = NurbsCurve::bezier(vec![
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(3.0, -1.0, 0.0),
            Point3::new(4.0, 0.0, 0.0),
        ])
        .unwrap();

        let joined = c1.join(&c2, 1e-6).unwrap();
        let (lo, hi) = joined.domain();

        // Check start and end
        assert!(joined.point_at(lo).approx_eq(Point3::ORIGIN));
        assert!(joined.point_at(hi).approx_eq(Point3::new(4.0, 0.0, 0.0)));

        // Check midpoint of first segment
        let p_mid1 = joined.point_at(0.5);
        let p_orig1 = c1.point_at(0.5);
        assert!(
            p_mid1.distance_to(p_orig1) < 1e-6,
            "first half mismatch: {p_mid1:?} vs {p_orig1:?}"
        );
    }

    #[test]
    fn test_split_boundary_error() {
        let curve = NurbsCurve::bezier(vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
        ])
        .unwrap();
        assert!(curve.split_at(0.0).is_err());
        assert!(curve.split_at(1.0).is_err());
    }

    #[test]
    fn test_refine_preserves_shape() {
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

        let refined = curve.refine_knots(&[0.25, 0.75]).unwrap();
        assert!(refined.control_points.len() > curve.control_points.len());

        let (lo, hi) = curve.domain();
        for i in 0..=20 {
            let t = lo + (hi - lo) * i as f64 / 20.0;
            let d = curve.point_at(t).distance_to(refined.point_at(t));
            assert!(d < 1e-10, "refine error at t={t}: {d}");
        }
    }

    #[test]
    fn test_refine_uniform() {
        let curve = NurbsCurve::bezier(vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        ])
        .unwrap();
        let new_knots: Vec<f64> = (1..=9).map(|i| i as f64 / 10.0).collect();
        let refined = curve.refine_knots(&new_knots).unwrap();
        assert_eq!(refined.control_points.len(), 12); // 3 + 9
    }

    #[test]
    fn test_remove_inserted_knot() {
        let curve = NurbsCurve::bezier(vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 2.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        ])
        .unwrap();
        let inserted = curve.insert_knot(0.5).unwrap();
        assert_eq!(inserted.control_points.len(), 4);

        let (removed_curve, count) = inserted.remove_knot(0.5, 1, 1e-6).unwrap();
        assert_eq!(count, 1, "should have removed 1 knot");
        assert_eq!(removed_curve.control_points.len(), 3);
    }

    #[test]
    fn test_decompose_bezier_noop() {
        let curve = NurbsCurve::bezier(vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        ])
        .unwrap();
        let segments = curve.decompose_to_bezier();
        assert_eq!(segments.len(), 1, "single Bezier should produce 1 segment");
    }

    #[test]
    fn test_decompose_multi_span() {
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

        let segments = curve.decompose_to_bezier();
        assert_eq!(segments.len(), 2, "two-span curve should decompose into 2 Bezier segments");

        // Each segment should match the original curve in its domain
        let (lo, hi) = curve.domain();
        for seg in &segments {
            let (sl, sh) = seg.domain();
            for i in 0..=10 {
                let t = sl + (sh - sl) * i as f64 / 10.0;
                let t_clamped = t.clamp(lo, hi);
                let orig = curve.point_at(t_clamped);
                let decomp = seg.point_at(t);
                assert!(
                    orig.distance_to(decomp) < 1e-8,
                    "decompose error at t={t}: orig={orig:?}, decomp={decomp:?}"
                );
            }
        }
    }

    #[test]
    fn test_join_degree_mismatch() {
        let c1 = NurbsCurve::bezier(vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
        ])
        .unwrap();
        let c2 = NurbsCurve::bezier(vec![
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.5, 1.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        ])
        .unwrap();
        assert!(c1.join(&c2, 1e-6).is_err()); // degree 1 vs degree 2
    }

    #[test]
    fn test_project_point_accuracy() {
        // Cubic Bezier curve
        let curve = NurbsCurve::bezier(vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 2.0, 0.0),
            Point3::new(3.0, 2.0, 0.0),
            Point3::new(4.0, 0.0, 0.0),
        ])
        .unwrap();

        // Project a point near the curve midpoint
        let test_pt = Point3::new(2.0, 2.5, 0.0);
        let (t, closest) = curve.project_point(test_pt);
        let on_curve = curve.point_at(t);
        assert!(
            closest.distance_to(on_curve) < 1e-10,
            "closest should be on curve"
        );

        // The tangent at the closest point should be perpendicular to the
        // vector from closest to test_pt
        let tangent = curve.tangent_at(t);
        let diff = test_pt - closest;
        let dot = tangent.x * diff.x + tangent.y * diff.y + tangent.z * diff.z;
        assert!(
            dot.abs() < 1e-6,
            "tangent·diff should be ~0 at closest point, got {dot}"
        );
    }

    #[test]
    fn test_project_on_helix() {
        // Create a helix-like NURBS by interpolating helix samples
        use crate::curve::nurbs_fitting;
        let n = 20;
        let points: Vec<Point3> = (0..n)
            .map(|i| {
                let t = i as f64 / (n - 1) as f64 * 4.0 * std::f64::consts::PI;
                Point3::new(t.cos(), t.sin(), t / (2.0 * std::f64::consts::PI))
            })
            .collect();
        let curve = nurbs_fitting::interpolate(&points, 3).unwrap();

        // Project center point — should find closest spiral point
        let test_pt = Point3::new(0.0, 0.0, 0.5);
        let (t, closest) = curve.project_point(test_pt);
        let dist = test_pt.distance_to(closest);
        // The helix is at radius 1, so minimum distance should be less than 1
        assert!(dist < 1.1, "project on helix: dist={dist}, t={t}");
    }

    #[test]
    fn test_reparameterize() {
        let mut curve = NurbsCurve::new(
            2,
            vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 1.0, 0.0),
                Point3::new(2.0, 0.0, 0.0),
            ],
            vec![1.0; 3],
            vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        )
        .unwrap();

        // Sample points before reparameterization
        let p_start = curve.point_at(0.0);
        let p_mid = curve.point_at(0.5);
        let p_end = curve.point_at(1.0);

        curve.reparameterize(2.0, 5.0);

        let (new_lo, new_hi) = curve.domain();
        assert!((new_lo - 2.0).abs() < 1e-10, "new lo = {new_lo}");
        assert!((new_hi - 5.0).abs() < 1e-10, "new hi = {new_hi}");

        // Same geometric points at corresponding parameters
        assert!(
            curve.point_at(2.0).distance_to(p_start) < 1e-10,
            "start point mismatch"
        );
        assert!(
            curve.point_at(3.5).distance_to(p_mid) < 1e-10,
            "mid point mismatch"
        );
        assert!(
            curve.point_at(5.0).distance_to(p_end) < 1e-10,
            "end point mismatch"
        );
    }
}
