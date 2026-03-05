use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};

use super::Surface;

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

    fn find_span(knots: &[f64], n: usize, p: usize, t: f64) -> usize {
        if t >= knots[n] {
            return n - 1;
        }
        if t <= knots[p] {
            return p;
        }
        let mut lo = p;
        let mut hi = n;
        let mut mid = (lo + hi) / 2;
        while t < knots[mid] || t >= knots[mid + 1] {
            if t < knots[mid] {
                hi = mid;
            } else {
                lo = mid;
            }
            mid = (lo + hi) / 2;
        }
        mid
    }

    fn basis_funs(knots: &[f64], span: usize, t: f64, p: usize) -> Vec<f64> {
        let mut n = vec![0.0; p + 1];
        let mut left = vec![0.0; p + 1];
        let mut right = vec![0.0; p + 1];
        n[0] = 1.0;
        for j in 1..=p {
            left[j] = t - knots[span + 1 - j];
            right[j] = knots[span + j] - t;
            let mut saved = 0.0;
            for r in 0..j {
                let temp = n[r] / (right[r + 1] + left[j - r]);
                n[r] = saved + right[r + 1] * temp;
                saved = left[j - r] * temp;
            }
            n[j] = saved;
        }
        n
    }

    fn evaluate(&self, u: f64, v: f64) -> Point3 {
        let span_u = Self::find_span(&self.knots_u, self.count_u, self.degree_u, u);
        let span_v = Self::find_span(&self.knots_v, self.count_v, self.degree_v, v);
        let nu = Self::basis_funs(&self.knots_u, span_u, u, self.degree_u);
        let nv = Self::basis_funs(&self.knots_v, span_v, v, self.degree_v);

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
        let dt = 1e-6;
        let (u0, u1) = self.domain_u();
        let (v0, v1) = self.domain_v();

        let du_lo = (u - dt).max(u0);
        let du_hi = (u + dt).min(u1);
        let dv_lo = (v - dt).max(v0);
        let dv_hi = (v + dt).min(v1);

        let dpu = self.point_at(du_hi, v) - self.point_at(du_lo, v);
        let dpv = self.point_at(u, dv_hi) - self.point_at(u, dv_lo);

        let n = dpu.cross(dpv);
        n.normalized().unwrap_or(Vec3::Z)
    }

    fn domain_u(&self) -> (f64, f64) {
        let p = self.degree_u;
        (self.knots_u[p], self.knots_u[self.knots_u.len() - 1 - p])
    }

    fn domain_v(&self) -> (f64, f64) {
        let p = self.degree_v;
        (self.knots_v[p], self.knots_v[self.knots_v.len() - 1 - p])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
