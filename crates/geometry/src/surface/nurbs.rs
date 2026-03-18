use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};

use super::Surface;
use crate::curve::bspline_basis;
use crate::curve::nurbs::NurbsCurve;

/// A NURBS surface in 3D space.
///
/// Control points are stored in row-major order: `control_points[v_index * count_u + u_index]`.
#[derive(Debug, Clone)]
pub struct NurbsSurface {
    pub degree_u: usize,
    pub degree_v: usize,
    pub count_u: usize,
    pub count_v: usize,
    pub control_points: Vec<Point3>,
    pub weights: Vec<f64>,
    pub knots_u: Vec<f64>,
    pub knots_v: Vec<f64>,
}

impl NurbsSurface {
    /// Creates a new NURBS surface.
    ///
    /// Returns `KernelError::InvalidArgument` if the array lengths are inconsistent
    /// with the given degrees and control-point counts.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        degree_u: usize,
        degree_v: usize,
        count_u: usize,
        count_v: usize,
        control_points: Vec<Point3>,
        weights: Vec<f64>,
        knots_u: Vec<f64>,
        knots_v: Vec<f64>,
    ) -> KernelResult<Self> {
        let total = count_u * count_v;
        if control_points.len() != total {
            return Err(KernelError::InvalidArgument(format!(
                "control_points.len() ({}) must equal count_u * count_v ({})",
                control_points.len(),
                total
            )));
        }
        if weights.len() != total {
            return Err(KernelError::InvalidArgument(format!(
                "weights.len() ({}) must equal count_u * count_v ({})",
                weights.len(),
                total
            )));
        }
        if knots_u.len() != count_u + degree_u + 1 {
            return Err(KernelError::InvalidArgument(format!(
                "knots_u.len() ({}) must equal count_u + degree_u + 1 ({})",
                knots_u.len(),
                count_u + degree_u + 1
            )));
        }
        if knots_v.len() != count_v + degree_v + 1 {
            return Err(KernelError::InvalidArgument(format!(
                "knots_v.len() ({}) must equal count_v + degree_v + 1 ({})",
                knots_v.len(),
                count_v + degree_v + 1
            )));
        }
        Ok(Self {
            degree_u,
            degree_v,
            count_u,
            count_v,
            control_points,
            weights,
            knots_u,
            knots_v,
        })
    }

    /// Computes homogeneous surface derivatives SKL[ku][kv] = Σ N^(ku)_i * N^(kv)_j * w_ij * P_ij.
    ///
    /// Returns a 2D array indexed by [du_order][dv_order], each element is [x*w, y*w, z*w, w].
    fn homogeneous_derivatives(&self, u: f64, v: f64, ku: usize, kv: usize) -> Vec<Vec<[f64; 4]>> {
        let span_u = bspline_basis::find_span(&self.knots_u, self.count_u, self.degree_u, u);
        let span_v = bspline_basis::find_span(&self.knots_v, self.count_v, self.degree_v, v);
        let ders_u = bspline_basis::ders_basis_funs(&self.knots_u, span_u, u, self.degree_u, ku);
        let ders_v = bspline_basis::ders_basis_funs(&self.knots_v, span_v, v, self.degree_v, kv);

        let mut skl = vec![vec![[0.0f64; 4]; kv + 1]; ku + 1];

        for du in 0..=ku {
            for dv in 0..=kv {
                let mut sw = [0.0; 4];
                for (l, &nv_l) in ders_v[dv].iter().enumerate().take(self.degree_v + 1) {
                    let vi = span_v - self.degree_v + l;
                    let mut temp = [0.0; 4];
                    for (k, &nu_k) in ders_u[du].iter().enumerate().take(self.degree_u + 1) {
                        let ui = span_u - self.degree_u + k;
                        let idx = vi * self.count_u + ui;
                        let w = self.weights[idx];
                        let cp = &self.control_points[idx];
                        let nd = nu_k * nv_l;
                        temp[0] += nd * cp.x * w;
                        temp[1] += nd * cp.y * w;
                        temp[2] += nd * cp.z * w;
                        temp[3] += nd * w;
                    }
                    for (s, &t) in sw.iter_mut().zip(temp.iter()) {
                        *s += t;
                    }
                }
                skl[du][dv] = sw;
            }
        }

        skl
    }

    /// Computes analytical partial derivative ∂S/∂u at (u,v).
    fn analytical_du(&self, u: f64, v: f64) -> Vec3 {
        let skl = self.homogeneous_derivatives(u, v, 1, 0);
        let w = skl[0][0][3];
        if w.abs() < 1e-14 {
            return Vec3::ZERO;
        }
        let s = Vec3::new(skl[0][0][0] / w, skl[0][0][1] / w, skl[0][0][2] / w);
        let aw_u = Vec3::new(skl[1][0][0], skl[1][0][1], skl[1][0][2]);
        let w_u = skl[1][0][3];
        // Rational quotient rule: ∂S/∂u = (Aw_u - w_u * S) / w
        (aw_u - s * w_u) / w
    }

    /// Computes analytical partial derivative ∂S/∂v at (u,v).
    fn analytical_dv(&self, u: f64, v: f64) -> Vec3 {
        let skl = self.homogeneous_derivatives(u, v, 0, 1);
        let w = skl[0][0][3];
        if w.abs() < 1e-14 {
            return Vec3::ZERO;
        }
        let s = Vec3::new(skl[0][0][0] / w, skl[0][0][1] / w, skl[0][0][2] / w);
        let aw_v = Vec3::new(skl[0][1][0], skl[0][1][1], skl[0][1][2]);
        let w_v = skl[0][1][3];
        (aw_v - s * w_v) / w
    }

    // ── Helper: extract a row (constant v index) as a 1D NurbsCurve ──
    fn extract_row(&self, v_idx: usize) -> NurbsCurve {
        let start = v_idx * self.count_u;
        let pts: Vec<Point3> = self.control_points[start..start + self.count_u].to_vec();
        let wts: Vec<f64> = self.weights[start..start + self.count_u].to_vec();
        NurbsCurve {
            degree: self.degree_u,
            control_points: pts,
            weights: wts,
            knots: self.knots_u.clone(),
        }
    }

    // ── Helper: extract a column (constant u index) as a 1D NurbsCurve ──
    fn extract_column(&self, u_idx: usize) -> NurbsCurve {
        let pts: Vec<Point3> = (0..self.count_v)
            .map(|vi| self.control_points[vi * self.count_u + u_idx])
            .collect();
        let wts: Vec<f64> = (0..self.count_v)
            .map(|vi| self.weights[vi * self.count_u + u_idx])
            .collect();
        NurbsCurve {
            degree: self.degree_v,
            control_points: pts,
            weights: wts,
            knots: self.knots_v.clone(),
        }
    }

    /// Inserts a knot in the u-direction `times` times.
    ///
    /// Operates row-by-row: each row of the control net is treated as a 1D
    /// NurbsCurve and gets the same knot inserted.
    pub fn insert_knot_u(&self, knot: f64, times: usize) -> KernelResult<NurbsSurface> {
        if times == 0 {
            return Ok(self.clone());
        }
        // Apply knot insertions to each row
        let mut rows: Vec<NurbsCurve> = Vec::with_capacity(self.count_v);
        for vi in 0..self.count_v {
            let mut row = self.extract_row(vi);
            for _ in 0..times {
                row = row.insert_knot(knot)?;
            }
            rows.push(row);
        }

        let new_count_u = rows[0].control_points.len();
        let new_knots_u = rows[0].knots.clone();
        let total = new_count_u * self.count_v;
        let mut cps = Vec::with_capacity(total);
        let mut wts = Vec::with_capacity(total);
        for row in &rows {
            cps.extend_from_slice(&row.control_points);
            wts.extend_from_slice(&row.weights);
        }

        NurbsSurface::new(
            self.degree_u,
            self.degree_v,
            new_count_u,
            self.count_v,
            cps,
            wts,
            new_knots_u,
            self.knots_v.clone(),
        )
    }

    /// Inserts a knot in the v-direction `times` times.
    ///
    /// Operates column-by-column.
    pub fn insert_knot_v(&self, knot: f64, times: usize) -> KernelResult<NurbsSurface> {
        if times == 0 {
            return Ok(self.clone());
        }
        let mut cols: Vec<NurbsCurve> = Vec::with_capacity(self.count_u);
        for ui in 0..self.count_u {
            let mut col = self.extract_column(ui);
            for _ in 0..times {
                col = col.insert_knot(knot)?;
            }
            cols.push(col);
        }

        let new_count_v = cols[0].control_points.len();
        let new_knots_v = cols[0].knots.clone();
        let total = self.count_u * new_count_v;
        let mut cps = vec![Point3::ORIGIN; total];
        let mut wts = vec![0.0; total];
        for (ui, col) in cols.iter().enumerate() {
            for (vi, (pt, &w)) in col
                .control_points
                .iter()
                .zip(col.weights.iter())
                .enumerate()
            {
                let idx = vi * self.count_u + ui;
                cps[idx] = *pt;
                wts[idx] = w;
            }
        }

        NurbsSurface::new(
            self.degree_u,
            self.degree_v,
            self.count_u,
            new_count_v,
            cps,
            wts,
            self.knots_u.clone(),
            new_knots_v,
        )
    }

    /// Refines the u-direction knot vector by inserting multiple knots.
    pub fn refine_knots_u(&self, new_knots: &[f64]) -> KernelResult<NurbsSurface> {
        if new_knots.is_empty() {
            return Ok(self.clone());
        }
        let mut rows: Vec<NurbsCurve> = Vec::with_capacity(self.count_v);
        for vi in 0..self.count_v {
            let row = self.extract_row(vi);
            rows.push(row.refine_knots(new_knots)?);
        }

        let new_count_u = rows[0].control_points.len();
        let new_knots_u = rows[0].knots.clone();
        let total = new_count_u * self.count_v;
        let mut cps = Vec::with_capacity(total);
        let mut wts = Vec::with_capacity(total);
        for row in &rows {
            cps.extend_from_slice(&row.control_points);
            wts.extend_from_slice(&row.weights);
        }

        NurbsSurface::new(
            self.degree_u,
            self.degree_v,
            new_count_u,
            self.count_v,
            cps,
            wts,
            new_knots_u,
            self.knots_v.clone(),
        )
    }

    /// Refines the v-direction knot vector by inserting multiple knots.
    pub fn refine_knots_v(&self, new_knots: &[f64]) -> KernelResult<NurbsSurface> {
        if new_knots.is_empty() {
            return Ok(self.clone());
        }
        let mut cols: Vec<NurbsCurve> = Vec::with_capacity(self.count_u);
        for ui in 0..self.count_u {
            let col = self.extract_column(ui);
            cols.push(col.refine_knots(new_knots)?);
        }

        let new_count_v = cols[0].control_points.len();
        let new_knots_v = cols[0].knots.clone();
        let total = self.count_u * new_count_v;
        let mut cps = vec![Point3::ORIGIN; total];
        let mut wts = vec![0.0; total];
        for (ui, col) in cols.iter().enumerate() {
            for (vi, (pt, &w)) in col
                .control_points
                .iter()
                .zip(col.weights.iter())
                .enumerate()
            {
                let idx = vi * self.count_u + ui;
                cps[idx] = *pt;
                wts[idx] = w;
            }
        }

        NurbsSurface::new(
            self.degree_u,
            self.degree_v,
            self.count_u,
            new_count_v,
            cps,
            wts,
            self.knots_u.clone(),
            new_knots_v,
        )
    }

    /// Elevates the degree in the u-direction by 1.
    ///
    /// Each row is treated as a 1D NurbsCurve and degree-elevated.
    pub fn elevate_degree_u(&self) -> KernelResult<NurbsSurface> {
        let mut rows: Vec<NurbsCurve> = Vec::with_capacity(self.count_v);
        for vi in 0..self.count_v {
            let row = self.extract_row(vi);
            rows.push(row.elevate_degree()?);
        }

        let new_degree_u = self.degree_u + 1;
        let new_count_u = rows[0].control_points.len();
        let new_knots_u = rows[0].knots.clone();
        let total = new_count_u * self.count_v;
        let mut cps = Vec::with_capacity(total);
        let mut wts = Vec::with_capacity(total);
        for row in &rows {
            cps.extend_from_slice(&row.control_points);
            wts.extend_from_slice(&row.weights);
        }

        NurbsSurface::new(
            new_degree_u,
            self.degree_v,
            new_count_u,
            self.count_v,
            cps,
            wts,
            new_knots_u,
            self.knots_v.clone(),
        )
    }

    /// Elevates the degree in the v-direction by 1.
    ///
    /// Each column is treated as a 1D NurbsCurve and degree-elevated.
    pub fn elevate_degree_v(&self) -> KernelResult<NurbsSurface> {
        let mut cols: Vec<NurbsCurve> = Vec::with_capacity(self.count_u);
        for ui in 0..self.count_u {
            let col = self.extract_column(ui);
            cols.push(col.elevate_degree()?);
        }

        let new_degree_v = self.degree_v + 1;
        let new_count_v = cols[0].control_points.len();
        let new_knots_v = cols[0].knots.clone();
        let total = self.count_u * new_count_v;
        let mut cps = vec![Point3::ORIGIN; total];
        let mut wts = vec![0.0; total];
        for (ui, col) in cols.iter().enumerate() {
            for (vi, (pt, &w)) in col
                .control_points
                .iter()
                .zip(col.weights.iter())
                .enumerate()
            {
                let idx = vi * self.count_u + ui;
                cps[idx] = *pt;
                wts[idx] = w;
            }
        }

        NurbsSurface::new(
            self.degree_u,
            new_degree_v,
            self.count_u,
            new_count_v,
            cps,
            wts,
            self.knots_u.clone(),
            new_knots_v,
        )
    }

    /// Extracts an isocurve at constant `u` value.
    ///
    /// Evaluates all rows of the control net at parameter `u` using
    /// B-spline basis functions, producing a new NurbsCurve in the v direction.
    pub fn isocurve_u(&self, u: f64) -> NurbsCurve {
        let span = bspline_basis::find_span(&self.knots_u, self.count_u, self.degree_u, u);
        let basis = bspline_basis::basis_funs(&self.knots_u, span, u, self.degree_u);

        let mut pts = Vec::with_capacity(self.count_v);
        let mut wts = Vec::with_capacity(self.count_v);

        for vi in 0..self.count_v {
            let mut sw = [0.0f64; 4];
            for (k, &nu_k) in basis.iter().enumerate().take(self.degree_u + 1) {
                let ui = span - self.degree_u + k;
                let idx = vi * self.count_u + ui;
                let w = self.weights[idx];
                let cp = &self.control_points[idx];
                sw[0] += nu_k * cp.x * w;
                sw[1] += nu_k * cp.y * w;
                sw[2] += nu_k * cp.z * w;
                sw[3] += nu_k * w;
            }
            let w_sum = sw[3];
            if w_sum.abs() < 1e-14 {
                pts.push(Point3::new(sw[0], sw[1], sw[2]));
                wts.push(1.0);
            } else {
                pts.push(Point3::new(sw[0] / w_sum, sw[1] / w_sum, sw[2] / w_sum));
                wts.push(w_sum);
            }
        }

        NurbsCurve {
            degree: self.degree_v,
            control_points: pts,
            weights: wts,
            knots: self.knots_v.clone(),
        }
    }

    /// Extracts an isocurve at constant `v` value.
    ///
    /// Evaluates all columns of the control net at parameter `v` using
    /// B-spline basis functions, producing a new NurbsCurve in the u direction.
    pub fn isocurve_v(&self, v: f64) -> NurbsCurve {
        let span = bspline_basis::find_span(&self.knots_v, self.count_v, self.degree_v, v);
        let basis = bspline_basis::basis_funs(&self.knots_v, span, v, self.degree_v);

        let mut pts = Vec::with_capacity(self.count_u);
        let mut wts = Vec::with_capacity(self.count_u);

        for ui in 0..self.count_u {
            let mut sw = [0.0f64; 4];
            for (l, &nv_l) in basis.iter().enumerate().take(self.degree_v + 1) {
                let vi = span - self.degree_v + l;
                let idx = vi * self.count_u + ui;
                let w = self.weights[idx];
                let cp = &self.control_points[idx];
                sw[0] += nv_l * cp.x * w;
                sw[1] += nv_l * cp.y * w;
                sw[2] += nv_l * cp.z * w;
                sw[3] += nv_l * w;
            }
            let w_sum = sw[3];
            if w_sum.abs() < 1e-14 {
                pts.push(Point3::new(sw[0], sw[1], sw[2]));
                wts.push(1.0);
            } else {
                pts.push(Point3::new(sw[0] / w_sum, sw[1] / w_sum, sw[2] / w_sum));
                wts.push(w_sum);
            }
        }

        NurbsCurve {
            degree: self.degree_u,
            control_points: pts,
            weights: wts,
            knots: self.knots_u.clone(),
        }
    }

    fn evaluate(&self, u: f64, v: f64) -> Point3 {
        let span_u = bspline_basis::find_span(&self.knots_u, self.count_u, self.degree_u, u);
        let span_v = bspline_basis::find_span(&self.knots_v, self.count_v, self.degree_v, v);
        let nu = bspline_basis::basis_funs(&self.knots_u, span_u, u, self.degree_u);
        let nv = bspline_basis::basis_funs(&self.knots_v, span_v, v, self.degree_v);

        let mut sw = [0.0; 4];
        for (l, &nv_l) in nv.iter().enumerate().take(self.degree_v + 1) {
            let mut temp = [0.0; 4];
            let vi = span_v - self.degree_v + l;
            for (k, &nu_k) in nu.iter().enumerate().take(self.degree_u + 1) {
                let ui = span_u - self.degree_u + k;
                let idx = vi * self.count_u + ui;
                let w = self.weights[idx];
                let cp = self.control_points[idx];
                temp[0] += nu_k * cp.x * w;
                temp[1] += nu_k * cp.y * w;
                temp[2] += nu_k * cp.z * w;
                temp[3] += nu_k * w;
            }
            for (sw_i, &t_i) in sw.iter_mut().zip(temp.iter()) {
                *sw_i += nv_l * t_i;
            }
        }

        Point3::new(sw[0] / sw[3], sw[1] / sw[3], sw[2] / sw[3])
    }
}

impl Surface for NurbsSurface {
    fn point_at(&self, u: f64, v: f64) -> Point3 {
        self.evaluate(u, v)
    }

    fn normal_at(&self, u: f64, v: f64) -> Vec3 {
        let du = self.analytical_du(u, v);
        let dv = self.analytical_dv(u, v);
        let n = du.cross(dv);
        n.normalized().unwrap_or(Vec3::Z)
    }

    fn du(&self, u: f64, v: f64) -> Vec3 {
        self.analytical_du(u, v)
    }

    fn dv(&self, u: f64, v: f64) -> Vec3 {
        self.analytical_dv(u, v)
    }

    fn domain_u(&self) -> (f64, f64) {
        let p = self.degree_u;
        (self.knots_u[p], self.knots_u[self.knots_u.len() - 1 - p])
    }

    fn domain_v(&self) -> (f64, f64) {
        let p = self.degree_v;
        (self.knots_v[p], self.knots_v[self.knots_v.len() - 1 - p])
    }

    /// Projects a point onto the surface using coarse grid + 2D Newton-Raphson.
    ///
    /// Newton system: `J^T * (S(u,v) - P) = 0`
    /// where `J = [∂S/∂u, ∂S/∂v]`.
    fn project_point(&self, point: Point3) -> (f64, f64, Point3) {
        let (u0, u1) = self.domain_u();
        let (v0, v1) = self.domain_v();

        // Coarse grid search for starting point
        let grid = 20;
        let mut best_u = u0;
        let mut best_v = v0;
        let mut best_dist = f64::MAX;

        for i in 0..=grid {
            let u = u0 + (u1 - u0) * i as f64 / grid as f64;
            for j in 0..=grid {
                let v = v0 + (v1 - v0) * j as f64 / grid as f64;
                let p = self.evaluate(u, v);
                let d = point.distance_to(p);
                if d < best_dist {
                    best_dist = d;
                    best_u = u;
                    best_v = v;
                }
            }
        }

        // 2D Newton-Raphson refinement
        let mut u = best_u;
        let mut v = best_v;

        for _ in 0..30 {
            let s = self.evaluate(u, v);
            let du = self.analytical_du(u, v);
            let dv = self.analytical_dv(u, v);
            let diff = s - point;
            let diff_v = Vec3::new(diff.x, diff.y, diff.z);

            // Residuals: r1 = diff · du, r2 = diff · dv
            let r1 = diff_v.dot(du);
            let r2 = diff_v.dot(dv);

            // Jacobian: [[du·du + diff·duu, du·dv + diff·duv],
            //            [du·dv + diff·duv, dv·dv + diff·dvv]]
            // Approximate (ignore second-order terms for stability):
            // J ≈ [[du·du, du·dv], [du·dv, dv·dv]]
            let j11 = du.dot(du);
            let j12 = du.dot(dv);
            let j22 = dv.dot(dv);

            let det = j11 * j22 - j12 * j12;
            if det.abs() < 1e-30 {
                break;
            }

            let delta_u = (j22 * r1 - j12 * r2) / det;
            let delta_v = (j11 * r2 - j12 * r1) / det;

            u = (u - delta_u).clamp(u0, u1);
            v = (v - delta_v).clamp(v0, v1);

            if delta_u.abs() < 1e-14 && delta_v.abs() < 1e-14 {
                break;
            }
        }

        (u, v, self.evaluate(u, v))
    }

    /// Bounding box from the convex hull property of NURBS control points.
    fn bounding_box(&self) -> cadkernel_math::BoundingBox {
        let first = self.control_points[0];
        let mut bb = cadkernel_math::BoundingBox::new(first, first);
        for &cp in &self.control_points {
            bb.include_point(cp);
        }
        bb
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::Curve;
    use cadkernel_math::EPSILON;

    fn bilinear_patch() -> NurbsSurface {
        NurbsSurface::new(
            1,
            1,
            2,
            2,
            vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
                Point3::new(1.0, 1.0, 0.0),
            ],
            vec![1.0; 4],
            vec![0.0, 0.0, 1.0, 1.0],
            vec![0.0, 0.0, 1.0, 1.0],
        )
        .unwrap()
    }

    #[test]
    fn test_bilinear_corners() {
        let s = bilinear_patch();
        assert!(s.point_at(0.0, 0.0).approx_eq(Point3::new(0.0, 0.0, 0.0)));
        assert!(s.point_at(1.0, 1.0).approx_eq(Point3::new(1.0, 1.0, 0.0)));
    }

    #[test]
    fn test_bilinear_center() {
        let s = bilinear_patch();
        let mid = s.point_at(0.5, 0.5);
        assert!((mid.x - 0.5).abs() < EPSILON);
        assert!((mid.y - 0.5).abs() < EPSILON);
    }

    #[test]
    fn test_bilinear_normal() {
        let s = bilinear_patch();
        let n = s.normal_at(0.5, 0.5);
        assert!(n.approx_eq(Vec3::Z) || n.approx_eq(-Vec3::Z));
    }

    #[test]
    fn test_du_analytical_bilinear() {
        // For bilinear patch: S(u,v) = (u, v, 0)
        // ∂S/∂u = (1, 0, 0) everywhere
        let s = bilinear_patch();
        let du = s.du(0.5, 0.5);
        assert!((du.x - 1.0).abs() < 1e-10, "du.x={}", du.x);
        assert!(du.y.abs() < 1e-10, "du.y={}", du.y);
        assert!(du.z.abs() < 1e-10, "du.z={}", du.z);
    }

    #[test]
    fn test_dv_analytical_bilinear() {
        // ∂S/∂v = (0, 1, 0)
        let s = bilinear_patch();
        let dv = s.dv(0.5, 0.5);
        assert!(dv.x.abs() < 1e-10, "dv.x={}", dv.x);
        assert!((dv.y - 1.0).abs() < 1e-10, "dv.y={}", dv.y);
        assert!(dv.z.abs() < 1e-10, "dv.z={}", dv.z);
    }

    #[test]
    fn test_du_analytical_curved() {
        // Biquadratic patch: S(u,v) = (u, v, u*(1-u))
        // 3×3 control points, degree 2×2
        let s = NurbsSurface::new(
            2,
            2,
            3,
            3,
            vec![
                // v=0 row
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(0.5, 0.0, 0.25),
                Point3::new(1.0, 0.0, 0.0),
                // v=0.5 row
                Point3::new(0.0, 0.5, 0.0),
                Point3::new(0.5, 0.5, 0.25),
                Point3::new(1.0, 0.5, 0.0),
                // v=1 row
                Point3::new(0.0, 1.0, 0.0),
                Point3::new(0.5, 1.0, 0.25),
                Point3::new(1.0, 1.0, 0.0),
            ],
            vec![1.0; 9],
            vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
            vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        )
        .unwrap();

        let du = s.du(0.5, 0.5);
        // Finite difference for comparison
        let dt = 1e-7;
        let p0 = s.point_at(0.5 - dt, 0.5);
        let p1 = s.point_at(0.5 + dt, 0.5);
        let fd = (p1 - p0) / (2.0 * dt);

        assert!(
            (du.x - fd.x).abs() < 1e-4,
            "du.x={} vs fd.x={}",
            du.x,
            fd.x
        );
        assert!(
            (du.y - fd.y).abs() < 1e-4,
            "du.y={} vs fd.y={}",
            du.y,
            fd.y
        );
        assert!(
            (du.z - fd.z).abs() < 1e-4,
            "du.z={} vs fd.z={}",
            du.z,
            fd.z
        );
    }

    #[test]
    fn test_normal_analytical_matches_cross() {
        let s = bilinear_patch();
        let n = s.normal_at(0.3, 0.7);
        // For flat patch, normal should be ±Z
        assert!(
            (n.z.abs() - 1.0).abs() < 1e-10,
            "normal should be ±Z: {:?}",
            n
        );
    }

    #[test]
    fn test_surface_knot_insert_u() {
        let s = bilinear_patch();
        let s2 = s.insert_knot_u(0.5, 1).unwrap();
        assert_eq!(s2.count_u, 3); // was 2, now 3
        assert_eq!(s2.count_v, 2); // unchanged
        // Shape must be preserved
        for i in 0..=5 {
            for j in 0..=5 {
                let u = i as f64 / 5.0;
                let v = j as f64 / 5.0;
                let p1 = s.point_at(u, v);
                let p2 = s2.point_at(u, v);
                assert!(
                    p1.distance_to(p2) < 1e-10,
                    "mismatch at ({u},{v}): {p1:?} vs {p2:?}"
                );
            }
        }
    }

    #[test]
    fn test_surface_knot_insert_v() {
        let s = bilinear_patch();
        let s2 = s.insert_knot_v(0.5, 1).unwrap();
        assert_eq!(s2.count_u, 2); // unchanged
        assert_eq!(s2.count_v, 3); // was 2, now 3
        for i in 0..=5 {
            for j in 0..=5 {
                let u = i as f64 / 5.0;
                let v = j as f64 / 5.0;
                let p1 = s.point_at(u, v);
                let p2 = s2.point_at(u, v);
                assert!(
                    p1.distance_to(p2) < 1e-10,
                    "mismatch at ({u},{v}): {p1:?} vs {p2:?}"
                );
            }
        }
    }

    #[test]
    fn test_surface_refine_u() {
        let s = bilinear_patch();
        let s2 = s.refine_knots_u(&[0.25, 0.5, 0.75]).unwrap();
        assert_eq!(s2.count_u, 5); // 2 + 3 insertions
        for i in 0..=5 {
            for j in 0..=5 {
                let u = i as f64 / 5.0;
                let v = j as f64 / 5.0;
                let p1 = s.point_at(u, v);
                let p2 = s2.point_at(u, v);
                assert!(
                    p1.distance_to(p2) < 1e-10,
                    "mismatch at ({u},{v}): {p1:?} vs {p2:?}"
                );
            }
        }
    }

    #[test]
    fn test_surface_refine_v() {
        let s = bilinear_patch();
        let s2 = s.refine_knots_v(&[0.3, 0.7]).unwrap();
        assert_eq!(s2.count_v, 4); // 2 + 2 insertions
        for i in 0..=5 {
            for j in 0..=5 {
                let u = i as f64 / 5.0;
                let v = j as f64 / 5.0;
                let p1 = s.point_at(u, v);
                let p2 = s2.point_at(u, v);
                assert!(
                    p1.distance_to(p2) < 1e-10,
                    "mismatch at ({u},{v}): {p1:?} vs {p2:?}"
                );
            }
        }
    }

    #[test]
    fn test_surface_elevate_u() {
        let s = bilinear_patch();
        let s2 = s.elevate_degree_u().unwrap();
        assert_eq!(s2.degree_u, 2); // was 1
        assert_eq!(s2.degree_v, 1); // unchanged
        for i in 0..=5 {
            for j in 0..=5 {
                let u = i as f64 / 5.0;
                let v = j as f64 / 5.0;
                let p1 = s.point_at(u, v);
                let p2 = s2.point_at(u, v);
                assert!(
                    p1.distance_to(p2) < 1e-10,
                    "elevate_u mismatch at ({u},{v}): {p1:?} vs {p2:?}"
                );
            }
        }
    }

    #[test]
    fn test_surface_elevate_v() {
        let s = bilinear_patch();
        let s2 = s.elevate_degree_v().unwrap();
        assert_eq!(s2.degree_u, 1); // unchanged
        assert_eq!(s2.degree_v, 2); // was 1
        for i in 0..=5 {
            for j in 0..=5 {
                let u = i as f64 / 5.0;
                let v = j as f64 / 5.0;
                let p1 = s.point_at(u, v);
                let p2 = s2.point_at(u, v);
                assert!(
                    p1.distance_to(p2) < 1e-10,
                    "elevate_v mismatch at ({u},{v}): {p1:?} vs {p2:?}"
                );
            }
        }
    }

    #[test]
    fn test_surface_project_point() {
        let s = bilinear_patch();
        // Point directly above (0.3, 0.7, 0)
        let test_pt = Point3::new(0.3, 0.7, 5.0);
        let (u, v, closest) = s.project_point(test_pt);
        assert!(
            (u - 0.3).abs() < 1e-6,
            "projected u={u}, expected 0.3"
        );
        assert!(
            (v - 0.7).abs() < 1e-6,
            "projected v={v}, expected 0.7"
        );
        assert!(
            closest.distance_to(Point3::new(0.3, 0.7, 0.0)) < 1e-6,
            "closest point mismatch: {closest:?}"
        );
    }

    #[test]
    fn test_isocurve_u() {
        let s = bilinear_patch();
        // Extract isocurve at u=0.3 — should match surface evaluation along v
        let iso = s.isocurve_u(0.3);
        for j in 0..=10 {
            let v = j as f64 / 10.0;
            let surf_pt = s.point_at(0.3, v);
            let iso_pt = iso.point_at(v);
            assert!(
                surf_pt.distance_to(iso_pt) < 1e-10,
                "isocurve_u mismatch at v={v}: surf={surf_pt:?} iso={iso_pt:?}"
            );
        }
    }

    #[test]
    fn test_isocurve_v() {
        let s = bilinear_patch();
        // Extract isocurve at v=0.7 — should match surface evaluation along u
        let iso = s.isocurve_v(0.7);
        for i in 0..=10 {
            let u = i as f64 / 10.0;
            let surf_pt = s.point_at(u, 0.7);
            let iso_pt = iso.point_at(u);
            assert!(
                surf_pt.distance_to(iso_pt) < 1e-10,
                "isocurve_v mismatch at u={u}: surf={surf_pt:?} iso={iso_pt:?}"
            );
        }
    }

    #[test]
    fn test_isocurve_u_boundary() {
        let s = bilinear_patch();
        // u=0 boundary isocurve
        let iso = s.isocurve_u(0.0);
        let p0 = iso.point_at(0.0);
        let p1 = iso.point_at(1.0);
        assert!(p0.distance_to(Point3::new(0.0, 0.0, 0.0)) < 1e-10);
        assert!(p1.distance_to(Point3::new(0.0, 1.0, 0.0)) < 1e-10);
    }

    #[test]
    fn test_isocurve_v_boundary() {
        let s = bilinear_patch();
        // v=1 boundary isocurve
        let iso = s.isocurve_v(1.0);
        let p0 = iso.point_at(0.0);
        let p1 = iso.point_at(1.0);
        assert!(p0.distance_to(Point3::new(0.0, 1.0, 0.0)) < 1e-10);
        assert!(p1.distance_to(Point3::new(1.0, 1.0, 0.0)) < 1e-10);
    }

    #[test]
    fn test_surface_elevate_bilinear_to_biquadratic() {
        // Elevate both directions: degree (1,1) → (2,2)
        let s = bilinear_patch();
        let s2 = s.elevate_degree_u().unwrap().elevate_degree_v().unwrap();
        assert_eq!(s2.degree_u, 2);
        assert_eq!(s2.degree_v, 2);
        for i in 0..=10 {
            for j in 0..=10 {
                let u = i as f64 / 10.0;
                let v = j as f64 / 10.0;
                let p1 = s.point_at(u, v);
                let p2 = s2.point_at(u, v);
                assert!(
                    p1.distance_to(p2) < 1e-10,
                    "biquadratic mismatch at ({u},{v}): {p1:?} vs {p2:?}"
                );
            }
        }
    }
}
