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
}
