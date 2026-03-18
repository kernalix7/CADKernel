use cadkernel_math::linalg::{DMatrix, DVector};

use crate::Sketch;
use crate::constraint::{ConstraintEval, ConstraintWithCtx};
use crate::entity::PointId;

/// Outcome of a solver run.
#[derive(Debug, Clone)]
pub struct SolverResult {
    pub converged: bool,
    pub iterations: usize,
    pub residual: f64,
}

/// Solve the constraint system attached to `sketch` using Newton-Raphson
/// with Armijo backtracking line search.
pub fn solve(sketch: &mut Sketch, max_iter: usize, tol: f64) -> SolverResult {
    let n_vars = sketch.points.len() * 2;
    if n_vars == 0 {
        return SolverResult {
            converged: true,
            iterations: 0,
            residual: 0.0,
        };
    }

    let lines: Vec<(PointId, PointId)> = sketch.lines.iter().map(|l| (l.start, l.end)).collect();

    let n_eqs: usize = sketch
        .constraints
        .iter()
        .map(|c| {
            ConstraintWithCtx {
                constraint: c,
                lines: &lines,
            }
            .num_equations()
        })
        .sum();

    if n_eqs == 0 {
        return SolverResult {
            converged: true,
            iterations: 0,
            residual: 0.0,
        };
    }

    let mut vars = DVector::zeros(n_vars);
    sketch_to_vars(sketch, &mut vars);

    let mut result = SolverResult {
        converged: false,
        iterations: 0,
        residual: f64::MAX,
    };

    for iter in 0..max_iter {
        let (residual_vec, jacobian) = build_system(sketch, &lines, n_eqs, n_vars, vars.as_slice());

        let norm = residual_vec.norm();
        result.residual = norm;
        result.iterations = iter + 1;

        if norm < tol {
            result.converged = true;
            vars_to_sketch(&vars, sketch);
            return result;
        }

        let dx = solve_linear_system(&jacobian, &residual_vec);

        // Armijo backtracking
        let mut alpha = 1.0;
        let c = 1e-4;
        let base_cost = 0.5 * norm * norm;
        let gradient = jacobian.transpose() * &residual_vec;
        let directional = gradient.dot(&dx);

        for _ in 0..20 {
            let candidate = &vars - &dx * alpha;
            let (r_new, _) = build_system(sketch, &lines, n_eqs, n_vars, candidate.as_slice());
            let new_cost = 0.5 * r_new.norm_squared();
            if new_cost <= base_cost - c * alpha * directional {
                vars = candidate;
                break;
            }
            alpha *= 0.5;
            if alpha < 1e-12 {
                vars -= &dx * alpha;
                break;
            }
        }
    }

    vars_to_sketch(&vars, sketch);
    result
}

fn sketch_to_vars(sketch: &Sketch, vars: &mut DVector<f64>) {
    for (i, p) in sketch.points.iter().enumerate() {
        vars[i * 2] = p.position.x;
        vars[i * 2 + 1] = p.position.y;
    }
}

fn vars_to_sketch(vars: &DVector<f64>, sketch: &mut Sketch) {
    for (i, p) in sketch.points.iter_mut().enumerate() {
        p.position.x = vars[i * 2];
        p.position.y = vars[i * 2 + 1];
    }
}

fn build_system(
    sketch: &Sketch,
    lines: &[(PointId, PointId)],
    n_eqs: usize,
    n_vars: usize,
    vars: &[f64],
) -> (DVector<f64>, DMatrix<f64>) {
    let mut residual = DVector::zeros(n_eqs);
    let mut jac = DMatrix::zeros(n_eqs, n_vars);

    let mut row = 0;
    let mut sparse_entries = Vec::new();

    for c in &sketch.constraints {
        let ctx = ConstraintWithCtx {
            constraint: c,
            lines,
        };
        let neq = ctx.num_equations();
        let mut local_res = vec![0.0; neq];
        ctx.residual(vars, &mut local_res);
        for (i, &v) in local_res.iter().enumerate() {
            residual[row + i] = v;
        }

        sparse_entries.clear();
        ctx.jacobian(vars, row, &mut sparse_entries);
        for &(r, c_idx, val) in &sparse_entries {
            if r < n_eqs && c_idx < n_vars {
                jac[(r, c_idx)] += val;
            }
        }

        row += neq;
    }

    (residual, jac)
}

/// Solve J * dx = r using the normal equations (J^T J) dx = J^T r.
/// Falls back to damped least-squares when singular.
fn solve_linear_system(jac: &DMatrix<f64>, residual: &DVector<f64>) -> DVector<f64> {
    let jt = jac.transpose();
    let jtj = &jt * jac;
    let jtr = &jt * residual;

    if let Some(lu) = jtj.clone().lu().try_inverse() {
        lu * jtr
    } else {
        // Levenberg-Marquardt damping
        let lambda = 1e-6;
        let eye = DMatrix::identity(jtj.nrows(), jtj.ncols());
        let damped = jtj + eye * lambda;
        damped
            .lu()
            .solve(&jtr)
            .unwrap_or_else(|| DVector::zeros(jac.ncols()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Sketch;
    use crate::constraint::Constraint;

    #[test]
    fn test_fixed_point_convergence() {
        let mut sketch = Sketch::new();
        let p = sketch.add_point(1.0, 1.0);
        sketch.add_constraint(Constraint::Fixed(p, 3.0, 4.0));

        let result = solve(&mut sketch, 50, 1e-10);
        assert!(result.converged);
        let pt = &sketch.points[p.0];
        assert!((pt.position.x - 3.0).abs() < 1e-8);
        assert!((pt.position.y - 4.0).abs() < 1e-8);
    }

    #[test]
    fn test_distance_constraint() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(3.0, 0.0);
        sketch.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
        sketch.add_constraint(Constraint::Distance(p0, p1, 5.0));

        let result = solve(&mut sketch, 100, 1e-10);
        assert!(result.converged);
        let dx = sketch.points[p1.0].position.x - sketch.points[p0.0].position.x;
        let dy = sketch.points[p1.0].position.y - sketch.points[p0.0].position.y;
        let dist = (dx * dx + dy * dy).sqrt();
        assert!((dist - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_horizontal_constraint() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(5.0, 3.0);
        sketch.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
        let l = sketch.add_line(p0, p1);
        sketch.add_constraint(Constraint::Horizontal(l));
        sketch.add_constraint(Constraint::Length(l, 5.0));

        let result = solve(&mut sketch, 100, 1e-10);
        assert!(result.converged);
        let dy = (sketch.points[p1.0].position.y - sketch.points[p0.0].position.y).abs();
        assert!(dy < 1e-6, "dy = {dy}");
    }

    #[test]
    fn test_equal_length_constraint() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(3.0, 0.0);
        let p2 = sketch.add_point(0.0, 1.0);
        let p3 = sketch.add_point(0.0, 5.0);
        sketch.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
        sketch.add_constraint(Constraint::Fixed(p2, 0.0, 1.0));
        let l1 = sketch.add_line(p0, p1);
        let l2 = sketch.add_line(p2, p3);
        sketch.add_constraint(Constraint::Horizontal(l1));
        sketch.add_constraint(Constraint::Vertical(l2));
        sketch.add_constraint(Constraint::Length(l1, 4.0));
        sketch.add_constraint(Constraint::EqualLength(l1, l2));

        let result = solve(&mut sketch, 100, 1e-10);
        assert!(result.converged);
        let dx1 = sketch.points[p1.0].position.x - sketch.points[p0.0].position.x;
        let dy2 = sketch.points[p3.0].position.y - sketch.points[p2.0].position.y;
        assert!((dx1.abs() - 4.0).abs() < 1e-6);
        assert!((dy2.abs() - 4.0).abs() < 1e-6);
    }

    #[test]
    fn test_midpoint_constraint() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(6.0, 4.0);
        let pm = sketch.add_point(1.0, 1.0);
        sketch.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
        sketch.add_constraint(Constraint::Fixed(p1, 6.0, 4.0));
        let l = sketch.add_line(p0, p1);
        sketch.add_constraint(Constraint::Midpoint(pm, l));

        let result = solve(&mut sketch, 100, 1e-10);
        assert!(result.converged);
        assert!((sketch.points[pm.0].position.x - 3.0).abs() < 1e-6);
        assert!((sketch.points[pm.0].position.y - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_collinear_constraint() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(2.0, 2.0);
        let p2 = sketch.add_point(3.0, 3.5);
        let p3 = sketch.add_point(5.0, 4.5);
        sketch.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
        sketch.add_constraint(Constraint::Fixed(p1, 2.0, 2.0));
        // Fix p2.x to fully constrain the system (collinear=2eq + length=1eq + fixed_x=1eq = 4eq for 4 vars)
        sketch.add_constraint(Constraint::Fixed(p2, 3.0, 3.0));
        let l1 = sketch.add_line(p0, p1);
        let l2 = sketch.add_line(p2, p3);
        sketch.add_constraint(Constraint::Collinear(l1, l2));
        sketch.add_constraint(Constraint::Length(l2, 2.0));

        let result = solve(&mut sketch, 100, 1e-10);
        assert!(result.converged);
        // l2 should be parallel to l1 (slope = 1) and s2 on y=x
        let e2 = &sketch.points[p3.0].position;
        let s2 = &sketch.points[p2.0].position;
        let dx = e2.x - s2.x;
        let dy = e2.y - s2.y;
        assert!((dy - dx).abs() < 1e-6, "l2 not parallel to y=x: dx={dx}, dy={dy}");
    }

    #[test]
    fn test_equal_radius_constraint() {
        let mut sketch = Sketch::new();
        // Circle 1: center c1, point on circle p1
        let c1 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(3.0, 0.0);
        // Circle 2: center c2, point on circle p2
        let c2 = sketch.add_point(5.0, 5.0);
        let p2 = sketch.add_point(5.0, 7.0);
        sketch.add_constraint(Constraint::Fixed(c1, 0.0, 0.0));
        sketch.add_constraint(Constraint::Fixed(c2, 5.0, 5.0));
        sketch.add_constraint(Constraint::Radius(p1, c1, 3.0));
        sketch.add_constraint(Constraint::EqualRadius(p1, c1, p2, c2));

        let result = solve(&mut sketch, 100, 1e-10);
        assert!(result.converged);
        let r1 = {
            let dx = sketch.points[p1.0].position.x - sketch.points[c1.0].position.x;
            let dy = sketch.points[p1.0].position.y - sketch.points[c1.0].position.y;
            (dx * dx + dy * dy).sqrt()
        };
        let r2 = {
            let dx = sketch.points[p2.0].position.x - sketch.points[c2.0].position.x;
            let dy = sketch.points[p2.0].position.y - sketch.points[c2.0].position.y;
            (dx * dx + dy * dy).sqrt()
        };
        assert!((r1 - r2).abs() < 1e-6, "r1={r1}, r2={r2}");
        assert!((r1 - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_concentric_constraint() {
        let mut sketch = Sketch::new();
        let c1 = sketch.add_point(1.0, 2.0);
        let c2 = sketch.add_point(3.0, 4.0);
        sketch.add_constraint(Constraint::Fixed(c1, 1.0, 2.0));
        sketch.add_constraint(Constraint::Concentric(c1, c2));

        let result = solve(&mut sketch, 100, 1e-10);
        assert!(result.converged);
        assert!((sketch.points[c2.0].position.x - 1.0).abs() < 1e-6);
        assert!((sketch.points[c2.0].position.y - 2.0).abs() < 1e-6);
    }
}
