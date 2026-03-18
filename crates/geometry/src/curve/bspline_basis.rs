//! B-spline basis function evaluation and derivatives.
//!
//! Shared between [`NurbsCurve`](super::nurbs::NurbsCurve) and
//! [`NurbsSurface`](crate::surface::nurbs::NurbsSurface).
//! Implements The NURBS Book Algorithms A2.1–A2.3.

/// Finds the knot span index for parameter `t` using binary search.
///
/// Returns `i` such that `knots[i] <= t < knots[i+1]` (or `n-1` at the
/// right boundary).
///
/// # Arguments
/// * `knots` — non-decreasing knot vector.
/// * `n` — number of control points.
/// * `p` — degree.
/// * `t` — parameter value.
pub fn find_span(knots: &[f64], n: usize, p: usize, t: f64) -> usize {
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

/// Computes the non-zero basis functions N_{span-p,p} .. N_{span,p} at `t`.
///
/// Returns a vector of length `p+1`.
/// The NURBS Book Algorithm A2.2.
pub fn basis_funs(knots: &[f64], span: usize, t: f64, p: usize) -> Vec<f64> {
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

/// Computes basis functions and their derivatives up to order `k` at `t`.
///
/// Returns a 2D array `ders[d][j]` where:
/// - `d` = derivative order (0..=k)
/// - `j` = basis function index (0..=p), corresponding to N_{span-p+j,p}
///
/// The NURBS Book Algorithm A2.3.
///
/// # Arguments
/// * `knots` — knot vector.
/// * `span` — knot span index from [`find_span`].
/// * `t` — parameter value.
/// * `p` — degree.
/// * `k` — maximum derivative order to compute.
pub fn ders_basis_funs(knots: &[f64], span: usize, t: f64, p: usize, k: usize) -> Vec<Vec<f64>> {
    let k = k.min(p); // can't take more derivatives than degree

    // ndu[j][r]: stores basis function values and knot differences
    let mut ndu = vec![vec![0.0; p + 1]; p + 1];
    ndu[0][0] = 1.0;

    let mut left = vec![0.0; p + 1];
    let mut right = vec![0.0; p + 1];

    for j in 1..=p {
        left[j] = t - knots[span + 1 - j];
        right[j] = knots[span + j] - t;
        let mut saved = 0.0;
        for r in 0..j {
            // Lower triangle: knot differences
            ndu[j][r] = right[r + 1] + left[j - r];
            let temp = ndu[r][j - 1] / ndu[j][r];
            // Upper triangle: basis function values
            ndu[r][j] = saved + right[r + 1] * temp;
            saved = left[j - r] * temp;
        }
        ndu[j][j] = saved;
    }

    // Load the basis functions (0th derivative)
    let mut ders = vec![vec![0.0; p + 1]; k + 1];
    for j in 0..=p {
        ders[0][j] = ndu[j][p];
    }

    // Compute derivatives: two rows a[0], a[1] alternated
    let mut a = vec![vec![0.0; p + 1]; 2];

    for r in 0..=p {
        // Alternate rows
        let mut s1 = 0usize;
        let mut s2 = 1usize;
        a[0][0] = 1.0;

        for kk in 1..=k {
            let mut d = 0.0;
            let rk = r as isize - kk as isize;
            let pk = (p as isize) - (kk as isize);

            if rk >= 0 {
                a[s2][0] = a[s1][0] / ndu[(pk + 1) as usize][rk as usize];
                d = a[s2][0] * ndu[rk as usize][pk as usize];
            }

            let j1 = if rk >= -1 { 1 } else { (-rk) as usize };
            let j2 = if (r as isize - 1) <= pk {
                kk - 1
            } else {
                p - r // The NURBS Book A2.3: j2 = p - r (NOT p - rk)
            };

            for j in j1..=j2 {
                a[s2][j] = (a[s1][j] - a[s1][j - 1]) / ndu[(pk + 1) as usize][(rk + j as isize) as usize];
                d += a[s2][j] * ndu[(rk + j as isize) as usize][pk as usize];
            }

            if (r as isize) <= pk {
                a[s2][kk] = -a[s1][kk - 1] / ndu[(pk + 1) as usize][r];
                d += a[s2][kk] * ndu[r][pk as usize];
            }

            ders[kk][r] = d;
            std::mem::swap(&mut s1, &mut s2);
        }
    }

    // Multiply by correct factors: k! * C(p,k) adjustment
    let mut factor = p as f64;
    for (kk, row) in ders.iter_mut().enumerate().skip(1) {
        for val in row.iter_mut() {
            *val *= factor;
        }
        factor *= p.saturating_sub(kk) as f64;
    }

    ders
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Linear basis (degree 1): N_{0,1}(0.5) = 0.5, N_{1,1}(0.5) = 0.5
    #[test]
    fn test_ders_linear() {
        let knots = [0.0, 0.0, 1.0, 1.0];
        let p = 1;
        let n = 2; // 2 control points
        let t = 0.5;
        let span = find_span(&knots, n, p, t);

        let ders = ders_basis_funs(&knots, span, t, p, 1);

        // 0th derivative: basis functions at t=0.5
        assert!((ders[0][0] - 0.5).abs() < 1e-12, "N0={}", ders[0][0]);
        assert!((ders[0][1] - 0.5).abs() < 1e-12, "N1={}", ders[0][1]);

        // 1st derivative: for clamped [0,0,1,1], dN0/dt = -1, dN1/dt = 1
        assert!((ders[1][0] - (-1.0)).abs() < 1e-12, "dN0={}", ders[1][0]);
        assert!((ders[1][1] - 1.0).abs() < 1e-12, "dN1={}", ders[1][1]);
    }

    /// Quadratic basis (degree 2), 3 control points, clamped knots [0,0,0,1,1,1]
    #[test]
    fn test_ders_quadratic() {
        let knots = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        let p = 2;
        let n = 3;
        let t = 0.5;
        let span = find_span(&knots, n, p, t);

        let ders = ders_basis_funs(&knots, span, t, p, 2);

        // Bernstein polynomials at t=0.5: (1-t)^2 = 0.25, 2t(1-t) = 0.5, t^2 = 0.25
        assert!((ders[0][0] - 0.25).abs() < 1e-12);
        assert!((ders[0][1] - 0.5).abs() < 1e-12);
        assert!((ders[0][2] - 0.25).abs() < 1e-12);

        // 1st derivatives: -2(1-t)=-1, 2-4t=0, 2t=1
        assert!((ders[1][0] - (-1.0)).abs() < 1e-12);
        assert!(ders[1][1].abs() < 1e-12);
        assert!((ders[1][2] - 1.0).abs() < 1e-12);

        // 2nd derivatives: 2, -4, 2
        assert!((ders[2][0] - 2.0).abs() < 1e-12, "d2N0={}", ders[2][0]);
        assert!((ders[2][1] - (-4.0)).abs() < 1e-12, "d2N1={}", ders[2][1]);
        assert!((ders[2][2] - 2.0).abs() < 1e-12, "d2N2={}", ders[2][2]);
    }

    /// Cubic basis (degree 3), 4 control points, clamped knots [0,0,0,0,1,1,1,1]
    #[test]
    fn test_ders_cubic() {
        let knots = [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0];
        let p = 3;
        let n = 4;
        let t = 0.5;
        let span = find_span(&knots, n, p, t);

        let ders = ders_basis_funs(&knots, span, t, p, 3);

        // Bernstein: (1-t)^3=0.125, 3t(1-t)^2=0.375, 3t^2(1-t)=0.375, t^3=0.125
        assert!((ders[0][0] - 0.125).abs() < 1e-12);
        assert!((ders[0][1] - 0.375).abs() < 1e-12);
        assert!((ders[0][2] - 0.375).abs() < 1e-12);
        assert!((ders[0][3] - 0.125).abs() < 1e-12);

        // Sum of basis functions = 1 (partition of unity)
        let sum: f64 = ders[0].iter().sum();
        assert!((sum - 1.0).abs() < 1e-12);

        // Sum of 1st derivatives = 0
        let sum_d1: f64 = ders[1].iter().sum();
        assert!(sum_d1.abs() < 1e-10);
    }

    /// Test at domain boundaries (t=0 and t=1)
    #[test]
    fn test_ders_boundary() {
        let knots = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        let p = 2;
        let n = 3;

        // At t=0
        let span0 = find_span(&knots, n, p, 0.0);
        let ders0 = ders_basis_funs(&knots, span0, 0.0, p, 1);
        assert!((ders0[0][0] - 1.0).abs() < 1e-12, "N0(0)={}", ders0[0][0]);
        assert!(ders0[0][1].abs() < 1e-12);
        assert!(ders0[0][2].abs() < 1e-12);

        // At t=1 (right boundary)
        let span1 = find_span(&knots, n, p, 1.0);
        let ders1 = ders_basis_funs(&knots, span1, 1.0, p, 1);
        assert!(ders1[0][0].abs() < 1e-12);
        assert!(ders1[0][1].abs() < 1e-12);
        assert!((ders1[0][2] - 1.0).abs() < 1e-12, "N2(1)={}", ders1[0][2]);
    }

    /// basis_funs should agree with ders_basis_funs[0]
    #[test]
    fn test_basis_funs_agree_with_ders() {
        let knots = [0.0, 0.0, 0.0, 0.5, 1.0, 1.0, 1.0];
        let p = 2;
        let n = 4;

        for i in 0..=10 {
            let t = i as f64 / 10.0;
            let span = find_span(&knots, n, p, t);
            let bf = basis_funs(&knots, span, t, p);
            let ders = ders_basis_funs(&knots, span, t, p, 0);

            for j in 0..=p {
                assert!(
                    (bf[j] - ders[0][j]).abs() < 1e-12,
                    "mismatch at t={t}, j={j}: bf={}, ders={}",
                    bf[j],
                    ders[0][j]
                );
            }
        }
    }

    /// Partition of unity: sum of basis functions = 1 for all t
    #[test]
    fn test_partition_of_unity() {
        let knots = [0.0, 0.0, 0.0, 0.0, 0.25, 0.5, 0.75, 1.0, 1.0, 1.0, 1.0];
        let p = 3;
        let n = 7;

        for i in 0..=40 {
            let t = i as f64 / 40.0;
            let span = find_span(&knots, n, p, t);
            let bf = basis_funs(&knots, span, t, p);
            let sum: f64 = bf.iter().sum();
            assert!(
                (sum - 1.0).abs() < 1e-12,
                "partition of unity violated at t={t}: sum={sum}"
            );
        }
    }
}
