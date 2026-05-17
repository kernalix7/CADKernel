use std::collections::BTreeSet;

use cadkernel_math::linalg::{DMatrix, DVector};

use crate::Sketch;
use crate::constraint::{Constraint, ConstraintEval, ConstraintWithCtx};
use crate::entity::PointId;

/// Outcome of a solver run.
///
/// Check `converged` to determine whether the constraints were satisfied
/// within tolerance. `remaining_dof` indicates how many degrees of freedom
/// remain after solving (0 = fully constrained).
#[derive(Debug, Clone)]
pub struct SolverResult {
    /// `true` if the residual dropped below the tolerance.
    pub converged: bool,
    /// Number of Newton-Raphson iterations performed.
    pub iterations: usize,
    /// Final L2 norm of the residual vector.
    pub residual: f64,
    /// Remaining degrees of freedom (`n_vars - rank(J)`). `None` if not computed.
    pub remaining_dof: Option<usize>,
    /// `true` if the system has more scalar equations than its Jacobian rank supports.
    pub over_constrained: bool,
}

/// Constraint block status after structural decomposition and rank analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockStatus {
    /// The block has the same number of scalar variables and equations, and
    /// the Jacobian is full rank.
    WellDetermined,
    /// The block leaves one or more independent degrees of freedom.
    UnderConstrained,
    /// The block has more scalar equations than its Jacobian rank supports.
    OverConstrained,
}

/// One strongly-connected constraint block in the sketch solver graph.
#[derive(Debug, Clone)]
pub struct SolverBlock {
    pub index: usize,
    pub point_ids: Vec<PointId>,
    pub variable_columns: Vec<usize>,
    pub constraint_indices: Vec<usize>,
    pub equation_count: usize,
    pub rank: usize,
    pub status: BlockStatus,
    pub remaining_dof: usize,
    pub redundant_equations: usize,
}

/// One deflation step performed by [`solve_with_analysis`].
#[derive(Debug, Clone)]
pub struct DeflationStep {
    pub block_index: usize,
    pub solved_variables: usize,
    pub solved_equations: usize,
    pub remaining_variables: usize,
    pub remaining_equations: usize,
    pub residual: f64,
    pub converged: bool,
}

/// Solver block decomposition and classification report.
#[derive(Debug, Clone)]
pub struct SolverAnalysis {
    pub blocks: Vec<SolverBlock>,
    pub deflation_steps: Vec<DeflationStep>,
    pub total_variables: usize,
    pub total_equations: usize,
    pub total_rank: usize,
}

/// Solver result together with the block analysis used to produce it.
#[derive(Debug, Clone)]
pub struct DeflatedSolverResult {
    pub result: SolverResult,
    pub analysis: SolverAnalysis,
}

/// Solves the constraint system attached to `sketch` using Newton-Raphson
/// with Armijo backtracking line search.
///
/// # Parameters
///
/// * `sketch` - The sketch whose point positions will be updated in place.
/// * `max_iter` - Maximum number of Newton iterations.
/// * `tol` - Convergence tolerance on the L2 residual norm.
///
/// # Returns
///
/// A [`SolverResult`] indicating convergence status, iteration count, and
/// remaining degrees of freedom.
pub fn solve(sketch: &mut Sketch, max_iter: usize, tol: f64) -> SolverResult {
    solve_with_analysis(sketch, max_iter, tol).result
}

/// Solve a sketch by decomposing the constraint graph into blocks, solving
/// each block, and deflating its variables and equations before continuing.
pub fn solve_with_analysis(sketch: &mut Sketch, max_iter: usize, tol: f64) -> DeflatedSolverResult {
    let n_vars = sketch.points.len() * 2;
    if n_vars == 0 {
        let result = SolverResult {
            converged: true,
            iterations: 0,
            residual: 0.0,
            remaining_dof: Some(0),
            over_constrained: false,
        };
        let analysis = analyze_blocks(sketch);
        return DeflatedSolverResult { result, analysis };
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
        let result = SolverResult {
            converged: true,
            iterations: 0,
            residual: 0.0,
            remaining_dof: Some(n_vars),
            over_constrained: false,
        };
        let analysis = analyze_blocks(sketch);
        return DeflatedSolverResult { result, analysis };
    }

    let mut vars = DVector::zeros(n_vars);
    sketch_to_vars(sketch, &mut vars);
    let blocks = decompose_blocks(sketch, &lines);
    let mut steps = Vec::new();
    let mut total_iterations = 0;
    let mut all_blocks_converged = true;
    let mut remaining_equations = n_eqs;
    let mut remaining_variables = n_vars;

    for block in &blocks {
        if block.equation_count == 0 {
            continue;
        }

        let block_result = solve_block(sketch, &lines, block, &mut vars, max_iter, tol);
        total_iterations += block_result.iterations;
        all_blocks_converged &= block_result.converged;
        remaining_equations = remaining_equations.saturating_sub(block.equation_count);
        remaining_variables = remaining_variables.saturating_sub(block.variable_columns.len());
        steps.push(DeflationStep {
            block_index: block.index,
            solved_variables: block.variable_columns.len(),
            solved_equations: block.equation_count,
            remaining_variables,
            remaining_equations,
            residual: block_result.residual,
            converged: block_result.converged,
        });
    }

    vars_to_sketch(&vars, sketch);

    let (final_residual, final_jac) = build_system(sketch, &lines, n_eqs, n_vars, vars.as_slice());
    let rank = jacobian_rank(&final_jac, 1e-8);
    let residual = final_residual.norm();
    let inf_norm = final_residual.iter().fold(0.0_f64, |a, &v| a.max(v.abs()));
    let result = SolverResult {
        converged: all_blocks_converged && inf_norm < tol,
        iterations: total_iterations,
        residual,
        remaining_dof: Some(n_vars.saturating_sub(rank)),
        over_constrained: n_eqs > rank,
    };

    let mut analysis = analyze_blocks(sketch);
    analysis.deflation_steps = steps;
    DeflatedSolverResult { result, analysis }
}

/// Analyze the current sketch into solver blocks without mutating geometry.
pub fn analyze_blocks(sketch: &Sketch) -> SolverAnalysis {
    let n_vars = sketch.points.len() * 2;
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

    let mut vars = DVector::zeros(n_vars);
    sketch_to_vars(sketch, &mut vars);

    let mut blocks = decompose_blocks(sketch, &lines);
    for block in &mut blocks {
        classify_block(sketch, &lines, block, vars.as_slice());
    }

    let total_rank = if n_eqs == 0 || n_vars == 0 {
        0
    } else {
        let (_, jac) = build_system(sketch, &lines, n_eqs, n_vars, vars.as_slice());
        jacobian_rank(&jac, 1e-8)
    };

    SolverAnalysis {
        blocks,
        deflation_steps: Vec::new(),
        total_variables: n_vars,
        total_equations: n_eqs,
        total_rank,
    }
}

fn solve_block(
    sketch: &Sketch,
    lines: &[(PointId, PointId)],
    block: &SolverBlock,
    vars: &mut DVector<f64>,
    max_iter: usize,
    tol: f64,
) -> SolverResult {
    let n_vars = block.variable_columns.len();
    if n_vars == 0 {
        let (residual_vec, _) = build_block_system(sketch, lines, block, vars.as_slice());
        return SolverResult {
            converged: residual_vec.iter().all(|r| r.abs() < tol),
            iterations: 0,
            residual: residual_vec.norm(),
            remaining_dof: Some(0),
            over_constrained: block.equation_count > 0,
        };
    }

    let mut local_vars = DVector::zeros(n_vars);
    for (local_col, &global_col) in block.variable_columns.iter().enumerate() {
        local_vars[local_col] = vars[global_col];
    }
    let initial_vars = local_vars.clone();
    let anchor = build_block_anchor_weights(sketch, block, n_vars);

    let mut result = SolverResult {
        converged: false,
        iterations: 0,
        residual: f64::MAX,
        remaining_dof: None,
        over_constrained: false,
    };

    let mut last_step_norm = f64::MAX;
    for iter in 0..max_iter {
        write_local_vars(block, &local_vars, vars);
        let (residual_vec, jacobian) = build_block_system(sketch, lines, block, vars.as_slice());

        let inf_norm = residual_vec.iter().fold(0.0_f64, |a, &v| a.max(v.abs()));
        let l2_norm = residual_vec.norm();
        result.residual = l2_norm;
        result.iterations = iter + 1;

        if inf_norm < tol {
            result.converged = true;
            write_local_vars(block, &local_vars, vars);
            break;
        }

        if last_step_norm < tol * 1e-3 {
            break;
        }

        let dx = solve_linear_system(
            &jacobian,
            &residual_vec,
            &anchor,
            &local_vars,
            &initial_vars,
        );

        let mut alpha = 1.0;
        let c = 1e-4;
        let base_cost = 0.5 * l2_norm * l2_norm;
        let gradient = jacobian.transpose() * &residual_vec;
        let directional = gradient.dot(&dx);

        let mut accepted = false;
        for _ in 0..20 {
            let candidate = &local_vars - &dx * alpha;
            write_local_vars(block, &candidate, vars);
            let (r_new, _) = build_block_system(sketch, lines, block, vars.as_slice());
            let new_cost = 0.5 * r_new.norm_squared();
            if new_cost <= base_cost - c * alpha * directional {
                local_vars = candidate;
                accepted = true;
                break;
            }
            alpha *= 0.5;
            if alpha < 1e-12 {
                break;
            }
        }

        last_step_norm = if accepted {
            (dx.norm() * alpha).abs()
        } else {
            0.0
        };
    }

    write_local_vars(block, &local_vars, vars);
    let (_, final_jac) = build_block_system(sketch, lines, block, vars.as_slice());
    let rank = jacobian_rank(&final_jac, 1e-8);
    result.remaining_dof = Some(n_vars.saturating_sub(rank));
    result.over_constrained = block.equation_count > rank;
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

fn write_local_vars(block: &SolverBlock, local_vars: &DVector<f64>, vars: &mut DVector<f64>) {
    for (local_col, &global_col) in block.variable_columns.iter().enumerate() {
        if global_col < vars.len() && local_col < local_vars.len() {
            vars[global_col] = local_vars[local_col];
        }
    }
}

fn build_block_system(
    sketch: &Sketch,
    lines: &[(PointId, PointId)],
    block: &SolverBlock,
    vars: &[f64],
) -> (DVector<f64>, DMatrix<f64>) {
    build_constraint_subset_system(
        sketch,
        lines,
        &block.constraint_indices,
        &block.variable_columns,
        block.equation_count,
        vars,
    )
}

fn build_constraint_subset_system(
    sketch: &Sketch,
    lines: &[(PointId, PointId)],
    constraint_indices: &[usize],
    variable_columns: &[usize],
    equation_count: usize,
    vars: &[f64],
) -> (DVector<f64>, DMatrix<f64>) {
    let mut residual = DVector::zeros(equation_count);
    let mut jac = DMatrix::zeros(equation_count, variable_columns.len());
    let mut global_to_local = vec![None; vars.len()];
    for (local_col, &global_col) in variable_columns.iter().enumerate() {
        if global_col < global_to_local.len() {
            global_to_local[global_col] = Some(local_col);
        }
    }

    let mut row = 0;
    let mut sparse_entries = Vec::new();
    for &constraint_index in constraint_indices {
        let Some(constraint) = sketch.constraints.get(constraint_index) else {
            continue;
        };
        let ctx = ConstraintWithCtx { constraint, lines };
        let neq = ctx.num_equations();
        let mut local_res = vec![0.0; neq];
        ctx.residual(vars, &mut local_res);
        for (i, &v) in local_res.iter().enumerate() {
            if row + i < residual.len() {
                residual[row + i] = v;
            }
        }

        sparse_entries.clear();
        ctx.jacobian(vars, row, &mut sparse_entries);
        for &(r, global_col, val) in &sparse_entries {
            if r < equation_count
                && let Some(Some(local_col)) = global_to_local.get(global_col)
            {
                jac[(r, *local_col)] += val;
            }
        }
        row += neq;
    }

    (residual, jac)
}

fn decompose_blocks(sketch: &Sketch, lines: &[(PointId, PointId)]) -> Vec<SolverBlock> {
    let point_count = sketch.points.len();
    let constraint_count = sketch.constraints.len();
    let mut adjacency = vec![Vec::new(); point_count + constraint_count];

    for (constraint_index, constraint) in sketch.constraints.iter().enumerate() {
        let constraint_node = point_count + constraint_index;
        for point in constraint_points(constraint, lines) {
            if point.0 < point_count {
                adjacency[constraint_node].push(point.0);
                adjacency[point.0].push(constraint_node);
            }
        }
    }

    let components = tarjan_scc(&adjacency);
    let mut blocks = Vec::new();
    for component in components {
        let mut point_ids = BTreeSet::new();
        let mut constraint_indices = BTreeSet::new();
        for node in component {
            if node < point_count {
                point_ids.insert(node);
            } else {
                constraint_indices.insert(node - point_count);
            }
        }

        if point_ids.is_empty() && constraint_indices.is_empty() {
            continue;
        }

        let point_ids: Vec<PointId> = point_ids.into_iter().map(PointId).collect();
        let variable_columns = variable_columns_for_points(&point_ids);
        let constraint_indices: Vec<usize> = constraint_indices.into_iter().collect();
        let equation_count = constraint_indices
            .iter()
            .filter_map(|&i| sketch.constraints.get(i))
            .map(|c| {
                ConstraintWithCtx {
                    constraint: c,
                    lines,
                }
                .num_equations()
            })
            .sum();

        blocks.push(SolverBlock {
            index: 0,
            point_ids,
            variable_columns,
            constraint_indices,
            equation_count,
            rank: 0,
            status: BlockStatus::UnderConstrained,
            remaining_dof: 0,
            redundant_equations: 0,
        });
    }

    blocks.sort_by_key(|block| {
        (
            block
                .constraint_indices
                .first()
                .copied()
                .unwrap_or(usize::MAX),
            block.point_ids.first().map(|p| p.0).unwrap_or(usize::MAX),
        )
    });
    for (index, block) in blocks.iter_mut().enumerate() {
        block.index = index;
    }
    blocks
}

fn classify_block(
    sketch: &Sketch,
    lines: &[(PointId, PointId)],
    block: &mut SolverBlock,
    vars: &[f64],
) {
    if block.equation_count == 0 {
        let variable_count = block.variable_columns.len();
        block.rank = 0;
        block.remaining_dof = variable_count;
        block.redundant_equations = 0;
        block.status = if variable_count == 0 {
            BlockStatus::WellDetermined
        } else {
            BlockStatus::UnderConstrained
        };
        return;
    }

    let (_, jac) = build_block_system(sketch, lines, block, vars);
    let variable_count = block.variable_columns.len();
    let rank = jacobian_rank(&jac, 1e-8);
    block.rank = rank;
    block.remaining_dof = variable_count.saturating_sub(rank);
    block.redundant_equations = block.equation_count.saturating_sub(rank);
    block.status = if block.equation_count > variable_count {
        BlockStatus::OverConstrained
    } else if block.equation_count == variable_count && rank == variable_count {
        BlockStatus::WellDetermined
    } else {
        BlockStatus::UnderConstrained
    };
}

fn tarjan_scc(adjacency: &[Vec<usize>]) -> Vec<Vec<usize>> {
    struct Tarjan<'a> {
        adjacency: &'a [Vec<usize>],
        next_index: usize,
        indices: Vec<Option<usize>>,
        lowlink: Vec<usize>,
        stack: Vec<usize>,
        on_stack: Vec<bool>,
        components: Vec<Vec<usize>>,
    }

    impl Tarjan<'_> {
        fn strong_connect(&mut self, node: usize) {
            self.indices[node] = Some(self.next_index);
            self.lowlink[node] = self.next_index;
            self.next_index += 1;
            self.stack.push(node);
            self.on_stack[node] = true;

            for &next in &self.adjacency[node] {
                if self.indices[next].is_none() {
                    self.strong_connect(next);
                    self.lowlink[node] = self.lowlink[node].min(self.lowlink[next]);
                } else if self.on_stack[next]
                    && let Some(next_index) = self.indices[next]
                {
                    self.lowlink[node] = self.lowlink[node].min(next_index);
                }
            }

            if self.indices[node] == Some(self.lowlink[node]) {
                let mut component = Vec::new();
                while let Some(member) = self.stack.pop() {
                    self.on_stack[member] = false;
                    component.push(member);
                    if member == node {
                        break;
                    }
                }
                self.components.push(component);
            }
        }
    }

    let mut tarjan = Tarjan {
        adjacency,
        next_index: 0,
        indices: vec![None; adjacency.len()],
        lowlink: vec![0; adjacency.len()],
        stack: Vec::new(),
        on_stack: vec![false; adjacency.len()],
        components: Vec::new(),
    };

    for node in 0..adjacency.len() {
        if tarjan.indices[node].is_none() {
            tarjan.strong_connect(node);
        }
    }
    tarjan.components
}

fn variable_columns_for_points(points: &[PointId]) -> Vec<usize> {
    let mut columns = Vec::with_capacity(points.len() * 2);
    for point in points {
        columns.push(point.0 * 2);
        columns.push(point.0 * 2 + 1);
    }
    columns
}

pub(crate) struct ConstraintSetRank {
    pub equation_count: usize,
    pub rank: usize,
}

pub(crate) fn analyze_constraint_set(
    sketch: &Sketch,
    constraint_indices: &[usize],
) -> ConstraintSetRank {
    let lines: Vec<(PointId, PointId)> = sketch.lines.iter().map(|l| (l.start, l.end)).collect();
    let n_vars = sketch.points.len() * 2;
    let mut vars = DVector::zeros(n_vars);
    sketch_to_vars(sketch, &mut vars);

    let mut point_ids = BTreeSet::new();
    let mut equation_count = 0;
    for &constraint_index in constraint_indices {
        let Some(constraint) = sketch.constraints.get(constraint_index) else {
            continue;
        };
        equation_count += ConstraintWithCtx {
            constraint,
            lines: &lines,
        }
        .num_equations();
        for point in constraint_points(constraint, &lines) {
            if point.0 < sketch.points.len() {
                point_ids.insert(point.0);
            }
        }
    }

    let points: Vec<PointId> = point_ids.into_iter().map(PointId).collect();
    let variable_columns = variable_columns_for_points(&points);
    let (_, jac) = build_constraint_subset_system(
        sketch,
        &lines,
        constraint_indices,
        &variable_columns,
        equation_count,
        vars.as_slice(),
    );
    ConstraintSetRank {
        equation_count,
        rank: jacobian_rank(&jac, 1e-8),
    }
}

pub(crate) fn constraint_points(
    constraint: &Constraint,
    lines: &[(PointId, PointId)],
) -> Vec<PointId> {
    let mut points = BTreeSet::new();

    match *constraint {
        Constraint::Coincident(p1, p2) | Constraint::Distance(p1, p2, _) => {
            points.insert(p1.0);
            points.insert(p2.0);
        }
        Constraint::Horizontal(line)
        | Constraint::Vertical(line)
        | Constraint::Length(line, _)
        | Constraint::PointOnObject(_, line) => {
            insert_line_points(&mut points, lines, line.0);
            if let Constraint::PointOnObject(point, _) = *constraint {
                points.insert(point.0);
            }
        }
        Constraint::Parallel(l1, l2)
        | Constraint::Perpendicular(l1, l2)
        | Constraint::Angle(l1, l2, _)
        | Constraint::EqualLength(l1, l2)
        | Constraint::Collinear(l1, l2) => {
            insert_line_points(&mut points, lines, l1.0);
            insert_line_points(&mut points, lines, l2.0);
        }
        Constraint::PointOnLine(point, line) => {
            points.insert(point.0);
            insert_line_points(&mut points, lines, line.0);
        }
        Constraint::PointOnCircle(point, center, _)
        | Constraint::Radius(point, center, _)
        | Constraint::Diameter(point, center, _)
        | Constraint::HorizontalDistance(point, center, _)
        | Constraint::VerticalDistance(point, center, _)
        | Constraint::Concentric(point, center) => {
            points.insert(point.0);
            points.insert(center.0);
        }
        Constraint::Symmetric(p1, p2, line) => {
            points.insert(p1.0);
            points.insert(p2.0);
            insert_line_points(&mut points, lines, line.0);
        }
        Constraint::Fixed(point, _, _) | Constraint::Block(point, _, _) => {
            points.insert(point.0);
        }
        Constraint::Tangent(line, center, _) => {
            points.insert(center.0);
            insert_line_points(&mut points, lines, line.0);
        }
        Constraint::Midpoint(point, line) => {
            points.insert(point.0);
            insert_line_points(&mut points, lines, line.0);
        }
        Constraint::EqualRadius(p1, c1, p2, c2) => {
            points.insert(p1.0);
            points.insert(c1.0);
            points.insert(p2.0);
            points.insert(c2.0);
        }
        Constraint::Refraction {
            line1,
            line2,
            ratio: _,
        } => {
            insert_line_points(&mut points, lines, line1.0);
            insert_line_points(&mut points, lines, line2.0);
        }
    }

    points.into_iter().map(PointId).collect()
}

fn insert_line_points(points: &mut BTreeSet<usize>, lines: &[(PointId, PointId)], line: usize) {
    if let Some((start, end)) = lines.get(line) {
        points.insert(start.0);
        points.insert(end.0);
    }
}

/// Estimate the numerical rank of a matrix via rank-revealing QR with column pivoting.
fn jacobian_rank(jac: &DMatrix<f64>, tol: f64) -> usize {
    let rows = jac.nrows();
    let cols = jac.ncols();
    if rows == 0 || cols == 0 {
        return 0;
    }

    let mut columns: Vec<Vec<f64>> = (0..cols)
        .map(|col| (0..rows).map(|row| jac[(row, col)]).collect())
        .collect();
    let max_abs = columns
        .iter()
        .flat_map(|column| column.iter())
        .fold(0.0_f64, |acc, &value| acc.max(value.abs()));
    if max_abs == 0.0 {
        return 0;
    }

    let threshold = tol * max_abs * rows.max(cols) as f64;
    let mut rank = 0;
    while rank < cols && rank < rows {
        let mut pivot = rank;
        let mut pivot_norm = column_norm_squared(&columns[rank]);
        for (candidate, column) in columns.iter().enumerate().skip(rank + 1) {
            let norm = column_norm_squared(column);
            if norm > pivot_norm {
                pivot = candidate;
                pivot_norm = norm;
            }
        }

        let pivot_len = pivot_norm.sqrt();
        if pivot_len <= threshold {
            break;
        }

        if pivot != rank {
            columns.swap(rank, pivot);
        }
        for value in &mut columns[rank] {
            *value /= pivot_len;
        }

        let pivot_column = columns[rank].clone();
        for column in columns.iter_mut().skip(rank + 1) {
            let projection = dot_columns(&pivot_column, column);
            for (value, pivot_value) in column.iter_mut().zip(&pivot_column) {
                *value -= projection * pivot_value;
            }
        }
        rank += 1;
    }
    rank
}

fn column_norm_squared(column: &[f64]) -> f64 {
    column.iter().map(|value| value * value).sum()
}

fn dot_columns(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(lhs, rhs)| lhs * rhs).sum()
}

/// Compute per-constraint residual norms (how "violated" each constraint is).
/// Returns a `Vec<f64>` parallel to `sketch.constraints`, each entry is the L2 norm
/// of that constraint's residual equations.
pub fn constraint_residuals(sketch: &Sketch) -> Vec<f64> {
    let n_vars = sketch.points.len() * 2;
    if n_vars == 0 {
        return vec![0.0; sketch.constraints.len()];
    }
    let mut vars = DVector::zeros(n_vars);
    sketch_to_vars(sketch, &mut vars);
    let lines: Vec<(PointId, PointId)> = sketch.lines.iter().map(|l| (l.start, l.end)).collect();

    let mut result = Vec::with_capacity(sketch.constraints.len());
    for c in &sketch.constraints {
        let ctx = ConstraintWithCtx {
            constraint: c,
            lines: &lines,
        };
        let neq = ctx.num_equations();
        let mut local_res = vec![0.0; neq];
        ctx.residual(vars.as_slice(), &mut local_res);
        let norm = local_res.iter().map(|r| r * r).sum::<f64>().sqrt();
        result.push(norm);
    }
    result
}

/// Drag a point while maintaining all existing constraints.
///
/// Temporarily adds a high-weight Fixed constraint on the dragged point,
/// solves the system, then removes the temporary constraint.
pub fn drag_solve(
    sketch: &mut Sketch,
    point: PointId,
    target_x: f64,
    target_y: f64,
    max_iter: usize,
    tol: f64,
) -> SolverResult {
    use crate::constraint::Constraint;

    if point.0 >= sketch.points.len() || !target_x.is_finite() || !target_y.is_finite() {
        return SolverResult {
            converged: false,
            iterations: 0,
            residual: f64::INFINITY,
            remaining_dof: None,
            over_constrained: false,
        };
    }

    // Add temporary fixed constraint with the drag target
    sketch.add_constraint(Constraint::Fixed(point, target_x, target_y));
    let result = solve(sketch, max_iter, tol);
    // Remove the temporary constraint (last one added)
    sketch.constraints.pop();
    result
}

/// Solve (J^T J + W) dx = J^T r - W (x - x0) using the normal equations, where
/// `W = diag(anchor)` is a Tikhonov regularization that pulls variables toward
/// their initial values. `W` breaks the symmetry in underdetermined systems so
/// that unfixed points prefer to remain near their starting positions.
///
/// Falls back to Levenberg-Marquardt damping when the augmented matrix is
/// still singular.
fn solve_linear_system(
    jac: &DMatrix<f64>,
    residual: &DVector<f64>,
    anchor: &DVector<f64>,
    vars: &DVector<f64>,
    initial_vars: &DVector<f64>,
) -> DVector<f64> {
    let jt = jac.transpose();
    let jtj = &jt * jac;
    let jtr = &jt * residual;

    let n = jtj.nrows();
    let mut augmented = jtj;
    for i in 0..n {
        augmented[(i, i)] += anchor[i];
    }

    let drift = vars - initial_vars;
    let mut rhs = jtr;
    for i in 0..n {
        rhs[i] -= anchor[i] * drift[i];
    }

    if let Some(lu) = augmented.clone().lu().try_inverse() {
        lu * rhs
    } else {
        let lambda = 1e-6;
        let eye = DMatrix::identity(n, n);
        let damped = augmented + eye * lambda;
        damped
            .lu()
            .solve(&rhs)
            .unwrap_or_else(|| DVector::zeros(jac.ncols()))
    }
}

/// Build per-variable anchor weights for one deflated block. When the block
/// has no explicit `Fixed` or `Block` constraint, its lowest local point is
/// weakly anchored so underdetermined independent blocks do not drift.
fn build_block_anchor_weights(sketch: &Sketch, block: &SolverBlock, n_vars: usize) -> DVector<f64> {
    let mut weights = DVector::zeros(n_vars);
    if block.point_ids.is_empty() || n_vars < 2 {
        return weights;
    }

    let has_fixed = block
        .constraint_indices
        .iter()
        .filter_map(|&index| sketch.constraints.get(index))
        .any(|c| matches!(c, Constraint::Fixed(..) | Constraint::Block(..)));
    if has_fixed {
        return weights;
    }

    weights[0] = 1.0;
    weights[1] = 1.0;
    weights
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
        assert!(
            (dy - dx).abs() < 1e-6,
            "l2 not parallel to y=x: dx={dx}, dy={dy}"
        );
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
