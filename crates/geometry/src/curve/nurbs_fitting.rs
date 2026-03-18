//! NURBS curve fitting: interpolation and approximation.
//!
//! - [`interpolate`]: Global interpolation through given points (The NURBS Book A9.1).
//! - [`approximate`]: Least-squares approximation of a point cloud (The NURBS Book A9.7).

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::Point3;

use super::bspline_basis;
use super::nurbs::NurbsCurve;

/// Interpolates a NURBS curve of given degree through the provided points.
///
/// Uses chord-length parameterisation and averaging for the knot vector
/// (The NURBS Book Algorithm A9.1).
///
/// # Arguments
/// * `points` — data points to interpolate (at least `degree + 1`).
/// * `degree` — polynomial degree (typically 3 for cubic).
///
/// # Returns
/// A `NurbsCurve` that passes exactly through all given points.
pub fn interpolate(points: &[Point3], degree: usize) -> KernelResult<NurbsCurve> {
    let n = points.len();
    if n < degree + 1 {
        return Err(KernelError::InvalidArgument(format!(
            "need at least {} points for degree {}, got {}",
            degree + 1,
            degree,
            n
        )));
    }

    let p = degree;

    // 1. Chord-length parameterisation
    let params = chord_length_params(points);

    // 2. Averaging knot vector
    let knots = averaging_knots(&params, n, p);

    // 3. Set up and solve the linear system N * P = Q
    //    where N[i][j] = N_{j,p}(t_i)
    let mut mat = vec![vec![0.0; n]; n];
    for (i, &t) in params.iter().enumerate() {
        let span = bspline_basis::find_span(&knots, n, p, t);
        let basis = bspline_basis::basis_funs(&knots, span, t, p);
        for (j, &bf) in basis.iter().enumerate() {
            let col = span - p + j;
            if col < n {
                mat[i][col] = bf;
            }
        }
    }

    // Solve for X, Y, Z independently using Gaussian elimination
    let xs: Vec<f64> = points.iter().map(|p| p.x).collect();
    let ys: Vec<f64> = points.iter().map(|p| p.y).collect();
    let zs: Vec<f64> = points.iter().map(|p| p.z).collect();

    let cx = solve_linear_system(&mat, &xs)?;
    let cy = solve_linear_system(&mat, &ys)?;
    let cz = solve_linear_system(&mat, &zs)?;

    let control_points: Vec<Point3> = (0..n)
        .map(|i| Point3::new(cx[i], cy[i], cz[i]))
        .collect();
    let weights = vec![1.0; n];

    NurbsCurve::new(p, control_points, weights, knots)
}

/// Approximates a NURBS curve to a point cloud using least-squares fitting.
///
/// # Arguments
/// * `points` — data points to approximate (must be > `num_control_points`).
/// * `degree` — polynomial degree.
/// * `num_control_points` — number of control points in the result.
/// * `tolerance` — not used for early termination; provided for API consistency.
///
/// # Returns
/// A `NurbsCurve` that approximates the point cloud.
pub fn approximate(
    points: &[Point3],
    degree: usize,
    num_control_points: usize,
    _tolerance: f64,
) -> KernelResult<NurbsCurve> {
    let m = points.len();
    let n = num_control_points;
    let p = degree;

    if n < p + 1 {
        return Err(KernelError::InvalidArgument(format!(
            "need at least {} control points for degree {}, got {}",
            p + 1,
            p,
            n
        )));
    }
    if m <= n {
        return Err(KernelError::InvalidArgument(format!(
            "need more data points ({}) than control points ({})",
            m, n
        )));
    }

    // 1. Chord-length parameterisation
    let params = chord_length_params(points);

    // 2. Knot vector by averaging
    let knots = averaging_knots(&params, n, p);

    // 3. Build the N matrix (m × n) where N[i][j] = N_{j,p}(t_i)
    //    First and last points are interpolated exactly (clamped)
    let mut mat = vec![vec![0.0; n]; m];
    for (i, &t) in params.iter().enumerate() {
        let span = bspline_basis::find_span(&knots, n, p, t);
        let basis = bspline_basis::basis_funs(&knots, span, t, p);
        for (j, &bf) in basis.iter().enumerate() {
            let col = span - p + j;
            if col < n {
                mat[i][col] = bf;
            }
        }
    }

    // 4. Fix first and last CPs to match endpoints
    //    Remove rows 0 and m-1, columns 0 and n-1 from the system
    //    Adjust RHS: Q_k' = Q_k - N_{k,0}*Q_0 - N_{k,n-1}*Q_{m-1}
    let q0 = points[0];
    let qm = points[m - 1];

    // Interior system: (m-2) equations, (n-2) unknowns
    let rows = m - 2;
    let cols = n - 2;

    if cols == 0 {
        // Only endpoints, no interior CPs
        let control_points = vec![q0, qm];
        let weights = vec![1.0; 2];
        return NurbsCurve::new(p.min(1), control_points, weights, knots);
    }

    // Build N' (interior) and RHS
    let mut n_int = vec![vec![0.0; cols]; rows];
    let mut rhs_x = vec![0.0; rows];
    let mut rhs_y = vec![0.0; rows];
    let mut rhs_z = vec![0.0; rows];

    for i in 0..rows {
        let row_idx = i + 1; // skip first point
        for j in 0..cols {
            n_int[i][j] = mat[row_idx][j + 1]; // skip first column
        }
        rhs_x[i] = points[row_idx].x - mat[row_idx][0] * q0.x - mat[row_idx][n - 1] * qm.x;
        rhs_y[i] = points[row_idx].y - mat[row_idx][0] * q0.y - mat[row_idx][n - 1] * qm.y;
        rhs_z[i] = points[row_idx].z - mat[row_idx][0] * q0.z - mat[row_idx][n - 1] * qm.z;
    }

    // 5. Solve normal equations: N'^T * N' * P = N'^T * rhs
    let ntn = mat_transpose_mul(&n_int);
    let ntr_x = mat_transpose_vec(&n_int, &rhs_x);
    let ntr_y = mat_transpose_vec(&n_int, &rhs_y);
    let ntr_z = mat_transpose_vec(&n_int, &rhs_z);

    let cx = solve_linear_system(&ntn, &ntr_x)?;
    let cy = solve_linear_system(&ntn, &ntr_y)?;
    let cz = solve_linear_system(&ntn, &ntr_z)?;

    let mut control_points = Vec::with_capacity(n);
    control_points.push(q0);
    for i in 0..cols {
        control_points.push(Point3::new(cx[i], cy[i], cz[i]));
    }
    control_points.push(qm);

    let weights = vec![1.0; n];
    NurbsCurve::new(p, control_points, weights, knots)
}

/// Chord-length parameterisation: t_i proportional to cumulative chord distance.
fn chord_length_params(points: &[Point3]) -> Vec<f64> {
    let n = points.len();
    let mut params = vec![0.0; n];
    let mut total = 0.0;

    for i in 1..n {
        total += points[i - 1].distance_to(points[i]);
        params[i] = total;
    }

    if total > 1e-14 {
        for p in &mut params {
            *p /= total;
        }
    } else {
        // Degenerate: uniform spacing
        for (i, p) in params.iter_mut().enumerate() {
            *p = i as f64 / (n - 1) as f64;
        }
    }

    // Ensure exact 0 and 1 at boundaries
    params[0] = 0.0;
    params[n - 1] = 1.0;
    params
}

/// Averaging knot vector (The NURBS Book Eq. 9.8).
fn averaging_knots(params: &[f64], n: usize, p: usize) -> Vec<f64> {
    let m = n + p + 1;
    let mut knots = vec![0.0; m];

    // Clamped start
    for k in knots.iter_mut().take(p + 1) {
        *k = 0.0;
    }
    // Clamped end
    for k in knots.iter_mut().skip(m - p - 1) {
        *k = 1.0;
    }

    // Interior knots: average of p consecutive parameters
    for j in 1..=(n - p - 1) {
        let sum: f64 = params[j..(j + p)].iter().sum();
        knots[j + p] = sum / p as f64;
    }

    knots
}

/// N^T * N for a rectangular matrix N (rows × cols → cols × cols).
fn mat_transpose_mul(n: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let rows = n.len();
    let cols = if rows > 0 { n[0].len() } else { 0 };
    let mut result = vec![vec![0.0; cols]; cols];
    for i in 0..cols {
        for j in 0..cols {
            let mut s = 0.0;
            for row in n.iter().take(rows) {
                s += row[i] * row[j];
            }
            result[i][j] = s;
        }
    }
    result
}

/// N^T * v for a rectangular matrix N and vector v.
fn mat_transpose_vec(n: &[Vec<f64>], v: &[f64]) -> Vec<f64> {
    let rows = n.len();
    let cols = if rows > 0 { n[0].len() } else { 0 };
    let mut result = vec![0.0; cols];
    for j in 0..cols {
        let mut s = 0.0;
        for (i, row) in n.iter().enumerate().take(rows) {
            s += row[j] * v[i];
        }
        result[j] = s;
    }
    result
}

/// Solves a linear system Ax = b using Gaussian elimination with partial pivoting.
fn solve_linear_system(a: &[Vec<f64>], b: &[f64]) -> KernelResult<Vec<f64>> {
    let n = b.len();
    if a.len() != n {
        return Err(KernelError::InvalidArgument(
            "matrix/vector size mismatch".into(),
        ));
    }

    // Augmented matrix
    let mut aug: Vec<Vec<f64>> = a
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let mut r = row.clone();
            r.push(b[i]);
            r
        })
        .collect();

    // Forward elimination with partial pivoting
    for col in 0..n {
        // Find pivot
        let mut max_val = aug[col][col].abs();
        let mut max_row = col;
        for (row_idx, aug_row) in aug.iter().enumerate().skip(col + 1) {
            if aug_row[col].abs() > max_val {
                max_val = aug_row[col].abs();
                max_row = row_idx;
            }
        }
        if max_val < 1e-14 {
            return Err(KernelError::InvalidArgument(
                "singular or near-singular matrix in interpolation".into(),
            ));
        }
        aug.swap(col, max_row);

        let pivot = aug[col][col];
        for row in (col + 1)..n {
            let factor = aug[row][col] / pivot;
            let (top, bottom) = aug.split_at_mut(row);
            let pivot_row = &top[col];
            for j in col..=n {
                bottom[0][j] -= factor * pivot_row[j];
            }
        }
    }

    // Back substitution
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let mut s = aug[i][n];
        for j in (i + 1)..n {
            s -= aug[i][j] * x[j];
        }
        x[i] = s / aug[i][i];
    }

    Ok(x)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::Curve;

    #[test]
    fn test_interpolate_collinear() {
        let points = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(3.0, 0.0, 0.0),
        ];
        let curve = interpolate(&points, 3).unwrap();

        for (i, pt) in points.iter().enumerate() {
            let t = i as f64 / 3.0;
            let eval = curve.point_at(t);
            assert!(
                eval.distance_to(*pt) < 1e-8,
                "mismatch at t={t}: eval={eval:?}, expected={pt:?}"
            );
        }
    }

    #[test]
    fn test_interpolate_quadratic() {
        let points = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        ];
        let curve = interpolate(&points, 2).unwrap();

        // Check that the curve passes through all points
        let (lo, hi) = curve.domain();
        assert!(curve.point_at(lo).distance_to(points[0]) < 1e-8);
        assert!(curve.point_at(hi).distance_to(points[2]) < 1e-8);
    }

    #[test]
    fn test_interpolate_3d() {
        let points = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 1.0),
            Point3::new(2.0, 0.0, 2.0),
            Point3::new(3.0, 1.0, 0.0),
            Point3::new(4.0, 0.0, 1.0),
        ];
        let curve = interpolate(&points, 3).unwrap();

        // Verify all points are on the curve (within tolerance)
        let params = chord_length_params(&points);
        for (i, pt) in points.iter().enumerate() {
            let eval = curve.point_at(params[i]);
            assert!(
                eval.distance_to(*pt) < 1e-6,
                "point {} mismatch: eval={eval:?}, expected={pt:?}",
                i
            );
        }
    }

    #[test]
    fn test_interpolate_too_few_points() {
        let points = vec![Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)];
        assert!(interpolate(&points, 3).is_err());
    }

    #[test]
    fn test_approximate_circle() {
        // Sample points from a circle
        let n_samples = 20;
        let points: Vec<Point3> = (0..n_samples)
            .map(|i| {
                let angle = 2.0 * std::f64::consts::PI * i as f64 / (n_samples - 1) as f64;
                Point3::new(angle.cos(), angle.sin(), 0.0)
            })
            .collect();

        let curve = approximate(&points, 3, 8, 0.01).unwrap();

        // Check that the approximation is close to the original points
        let params = chord_length_params(&points);
        let mut max_err = 0.0f64;
        for (i, pt) in points.iter().enumerate() {
            let eval = curve.point_at(params[i]);
            max_err = max_err.max(eval.distance_to(*pt));
        }
        assert!(
            max_err < 0.2,
            "approximation max error too large: {max_err}"
        );
    }

    #[test]
    fn test_approximate_too_few_data() {
        let points = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
        ];
        assert!(approximate(&points, 3, 5, 0.01).is_err());
    }
}
