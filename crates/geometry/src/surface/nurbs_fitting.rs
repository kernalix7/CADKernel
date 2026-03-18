//! NURBS surface fitting: interpolation via tensor-product two-pass method.
//!
//! - [`interpolate`]: Global interpolation of a grid of points.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::Point3;

use super::nurbs::NurbsSurface;
use crate::curve::nurbs_fitting as curve_fit;

/// Interpolates a NURBS surface through a rectangular grid of points.
///
/// Uses a two-pass tensor-product approach:
/// 1. Interpolate each row (constant v) as a NurbsCurve in u.
/// 2. For each u-index column of the resulting control points, interpolate in v.
///
/// # Arguments
/// * `grid` — rectangular grid of points, `grid[v_idx][u_idx]`. All rows must have the same length.
/// * `degree_u` — polynomial degree in u-direction.
/// * `degree_v` — polynomial degree in v-direction.
///
/// # Returns
/// A `NurbsSurface` that passes through all grid points.
pub fn interpolate(
    grid: &[Vec<Point3>],
    degree_u: usize,
    degree_v: usize,
) -> KernelResult<NurbsSurface> {
    let nv = grid.len();
    if nv < degree_v + 1 {
        return Err(KernelError::InvalidArgument(format!(
            "need at least {} rows for degree_v={}, got {}",
            degree_v + 1,
            degree_v,
            nv
        )));
    }
    let nu = grid[0].len();
    if nu < degree_u + 1 {
        return Err(KernelError::InvalidArgument(format!(
            "need at least {} columns for degree_u={}, got {}",
            degree_u + 1,
            degree_u,
            nu
        )));
    }
    for (i, row) in grid.iter().enumerate() {
        if row.len() != nu {
            return Err(KernelError::InvalidArgument(format!(
                "row {} has {} points, expected {}",
                i,
                row.len(),
                nu
            )));
        }
    }

    // Pass 1: Interpolate each row in the u-direction.
    // Each row gives a NurbsCurve with `nu` control points and knots_u.
    let mut row_curves = Vec::with_capacity(nv);
    for row in grid {
        row_curves.push(curve_fit::interpolate(row, degree_u)?);
    }

    // All row curves share the same knots_u (same parameterization & degree).
    let knots_u = row_curves[0].knots.clone();
    let count_u = row_curves[0].control_points.len();

    // Pass 2: For each u-column of the row-curve control points,
    // interpolate in the v-direction.
    let mut final_cps = Vec::with_capacity(count_u * nv);
    let mut final_wts = Vec::with_capacity(count_u * nv);
    let mut knots_v = Vec::new();
    let mut count_v = 0;

    // Gather column points from row curves' control points
    for ui in 0..count_u {
        let col_pts: Vec<Point3> = row_curves
            .iter()
            .map(|c| c.control_points[ui])
            .collect();
        let col_curve = curve_fit::interpolate(&col_pts, degree_v)?;

        if ui == 0 {
            knots_v = col_curve.knots.clone();
            count_v = col_curve.control_points.len();
        }

        // Store control points in column order (will reassemble row-major later)
        for (vi, (pt, &w)) in col_curve
            .control_points
            .iter()
            .zip(col_curve.weights.iter())
            .enumerate()
        {
            // We need row-major: idx = vi * count_u + ui
            // But we're iterating ui first, so store temporarily
            let idx = vi * count_u + ui;
            if final_cps.len() <= idx {
                final_cps.resize(count_u * count_v, Point3::ORIGIN);
                final_wts.resize(count_u * count_v, 1.0);
            }
            final_cps[idx] = *pt;
            final_wts[idx] = w;
        }
    }

    NurbsSurface::new(
        degree_u,
        degree_v,
        count_u,
        count_v,
        final_cps,
        final_wts,
        knots_u,
        knots_v,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::Surface;

    #[test]
    fn test_surface_interpolate_flat() {
        // 4×4 grid on z=0 plane
        let grid: Vec<Vec<Point3>> = (0..4)
            .map(|vi| {
                (0..4)
                    .map(|ui| {
                        Point3::new(ui as f64 / 3.0, vi as f64 / 3.0, 0.0)
                    })
                    .collect()
            })
            .collect();

        let s = interpolate(&grid, 3, 3).unwrap();

        // Surface should pass through all grid points (within tolerance)
        for (vi, row) in grid.iter().enumerate() {
            for (ui, &expected) in row.iter().enumerate() {
                let u = ui as f64 / 3.0;
                let v = vi as f64 / 3.0;
                let eval = s.point_at(u, v);
                assert!(
                    eval.distance_to(expected) < 1e-6,
                    "flat grid mismatch at ({ui},{vi}): eval={eval:?}, expected={expected:?}"
                );
            }
        }
    }

    #[test]
    fn test_surface_interpolate_dome() {
        // Dome-like shape: z = 1 - (x^2 + y^2), sampled on [-1,1]×[-1,1]
        let n = 5;
        let grid: Vec<Vec<Point3>> = (0..n)
            .map(|vi| {
                let v = -1.0 + 2.0 * vi as f64 / (n - 1) as f64;
                (0..n)
                    .map(|ui| {
                        let u = -1.0 + 2.0 * ui as f64 / (n - 1) as f64;
                        Point3::new(u, v, 1.0 - u * u - v * v)
                    })
                    .collect()
            })
            .collect();

        let s = interpolate(&grid, 3, 3).unwrap();

        // Check corners pass through exactly
        let (u0, u1) = s.domain_u();
        let (v0, v1) = s.domain_v();
        let corner00 = s.point_at(u0, v0);
        let corner11 = s.point_at(u1, v1);
        assert!(
            corner00.distance_to(grid[0][0]) < 1e-6,
            "corner (0,0) mismatch"
        );
        assert!(
            corner11.distance_to(grid[n - 1][n - 1]) < 1e-6,
            "corner (n,n) mismatch"
        );
    }

    #[test]
    fn test_surface_interpolate_too_few_rows() {
        let grid = vec![
            vec![Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)],
        ];
        assert!(interpolate(&grid, 1, 3).is_err());
    }

    #[test]
    fn test_surface_interpolate_too_few_cols() {
        let grid: Vec<Vec<Point3>> = (0..4)
            .map(|vi| vec![Point3::new(0.0, vi as f64, 0.0)])
            .collect();
        assert!(interpolate(&grid, 3, 1).is_err());
    }
}
