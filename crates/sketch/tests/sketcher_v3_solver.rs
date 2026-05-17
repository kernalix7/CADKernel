use cadkernel_sketch::{
    BlockStatus, Constraint, Sketch, analyze_blocks, solve, solve_with_analysis,
};

const TOL: f64 = 1e-9;

fn fixed_point_sketch(x: f64, y: f64, tx: f64, ty: f64) -> Sketch {
    let mut sketch = Sketch::new();
    let point = sketch.add_point(x, y);
    sketch.add_constraint(Constraint::Fixed(point, tx, ty));
    sketch
}

#[test]
fn empty_sketch_has_no_solver_blocks() {
    let sketch = Sketch::new();
    let analysis = analyze_blocks(&sketch);
    assert!(analysis.blocks.is_empty());
    assert_eq!(analysis.total_variables, 0);
    assert_eq!(analysis.total_equations, 0);
}

#[test]
fn free_point_block_is_under_constrained() {
    let mut sketch = Sketch::new();
    sketch.add_point(1.0, 2.0);
    let analysis = analyze_blocks(&sketch);
    assert_eq!(analysis.blocks.len(), 1);
    assert_eq!(analysis.blocks[0].status, BlockStatus::UnderConstrained);
    assert_eq!(analysis.blocks[0].remaining_dof, 2);
}

#[test]
fn fixed_point_block_is_well_determined() {
    let sketch = fixed_point_sketch(1.0, 2.0, 3.0, 4.0);
    let analysis = analyze_blocks(&sketch);
    assert_eq!(analysis.blocks.len(), 1);
    assert_eq!(analysis.blocks[0].status, BlockStatus::WellDetermined);
    assert_eq!(analysis.blocks[0].equation_count, 2);
    assert_eq!(analysis.blocks[0].rank, 2);
}

#[test]
fn single_distance_block_is_under_constrained() {
    let mut sketch = Sketch::new();
    let p0 = sketch.add_point(0.0, 0.0);
    let p1 = sketch.add_point(4.0, 0.0);
    sketch.add_constraint(Constraint::Distance(p0, p1, 5.0));
    let analysis = analyze_blocks(&sketch);
    assert_eq!(analysis.blocks.len(), 1);
    assert_eq!(analysis.blocks[0].status, BlockStatus::UnderConstrained);
    assert_eq!(analysis.blocks[0].remaining_dof, 3);
}

#[test]
fn duplicate_fixed_constraints_are_over_constrained() {
    let mut sketch = fixed_point_sketch(0.0, 0.0, 0.0, 0.0);
    sketch.add_constraint(Constraint::Fixed(cadkernel_sketch::PointId(0), 0.0, 0.0));
    let analysis = analyze_blocks(&sketch);
    assert_eq!(analysis.blocks.len(), 1);
    assert_eq!(analysis.blocks[0].status, BlockStatus::OverConstrained);
    assert_eq!(analysis.blocks[0].redundant_equations, 2);
}

#[test]
fn three_independent_fixed_points_form_three_blocks() {
    let mut sketch = Sketch::new();
    for i in 0..3 {
        let point = sketch.add_point(i as f64, 0.0);
        sketch.add_constraint(Constraint::Fixed(point, i as f64 + 10.0, i as f64 + 20.0));
    }

    let analysis = analyze_blocks(&sketch);
    assert_eq!(analysis.blocks.len(), 3);
    assert!(
        analysis
            .blocks
            .iter()
            .all(|block| block.status == BlockStatus::WellDetermined)
    );
}

#[test]
fn solve_with_analysis_records_three_deflation_steps() {
    let mut sketch = Sketch::new();
    for i in 0..3 {
        let point = sketch.add_point(i as f64, 0.0);
        sketch.add_constraint(Constraint::Fixed(point, i as f64, i as f64 + 1.0));
    }

    let result = solve_with_analysis(&mut sketch, 20, TOL);
    assert!(result.result.converged);
    assert_eq!(result.analysis.deflation_steps.len(), 3);
    assert_eq!(result.analysis.deflation_steps[0].remaining_equations, 4);
    assert_eq!(result.analysis.deflation_steps[1].remaining_equations, 2);
    assert_eq!(result.analysis.deflation_steps[2].remaining_equations, 0);
}

#[test]
fn deflation_steps_shrink_remaining_variable_count() {
    let mut sketch = Sketch::new();
    for i in 0..2 {
        let point = sketch.add_point(i as f64, 0.0);
        sketch.add_constraint(Constraint::Fixed(point, i as f64 + 2.0, i as f64 + 3.0));
    }

    let result = solve_with_analysis(&mut sketch, 20, TOL);
    let steps = &result.analysis.deflation_steps;
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].remaining_variables, 2);
    assert_eq!(steps[1].remaining_variables, 0);
}

#[test]
fn deflated_solver_updates_each_block_solution() {
    let mut sketch = Sketch::new();
    let p0 = sketch.add_point(0.0, 0.0);
    let p1 = sketch.add_point(10.0, 10.0);
    sketch.add_constraint(Constraint::Fixed(p0, 2.0, 3.0));
    sketch.add_constraint(Constraint::Fixed(p1, -4.0, 5.0));

    let result = solve_with_analysis(&mut sketch, 20, TOL);
    assert!(result.result.converged);
    assert!((sketch.points[p0.0].position.x - 2.0).abs() < TOL);
    assert!((sketch.points[p0.0].position.y - 3.0).abs() < TOL);
    assert!((sketch.points[p1.0].position.x + 4.0).abs() < TOL);
    assert!((sketch.points[p1.0].position.y - 5.0).abs() < TOL);
}

#[test]
fn free_points_do_not_create_deflation_steps() {
    let mut sketch = fixed_point_sketch(0.0, 0.0, 1.0, 1.0);
    sketch.add_point(100.0, 200.0);

    let result = solve_with_analysis(&mut sketch, 20, TOL);
    assert_eq!(result.analysis.blocks.len(), 2);
    assert_eq!(result.analysis.deflation_steps.len(), 1);
    assert_eq!(sketch.points[1].position.x, 100.0);
    assert_eq!(sketch.points[1].position.y, 200.0);
}

#[test]
fn per_block_anchor_solves_unfixed_distance_even_when_other_block_is_fixed() {
    let mut sketch = fixed_point_sketch(0.0, 0.0, 0.0, 0.0);
    let a = sketch.add_point(10.0, 0.0);
    let b = sketch.add_point(13.0, 0.0);
    sketch.add_constraint(Constraint::Distance(a, b, 5.0));

    let result = solve_with_analysis(&mut sketch, 100, TOL);
    assert!(result.result.converged);
    let dx = sketch.points[a.0].position.x - sketch.points[b.0].position.x;
    let dy = sketch.points[a.0].position.y - sketch.points[b.0].position.y;
    assert!(((dx * dx + dy * dy).sqrt() - 5.0).abs() < 1e-6);
}

#[test]
fn well_determined_line_block_is_classified() {
    let mut sketch = Sketch::new();
    let p0 = sketch.add_point(0.0, 0.0);
    let p1 = sketch.add_point(4.0, 0.0);
    let line = sketch.add_line(p0, p1);
    sketch.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
    sketch.add_constraint(Constraint::Horizontal(line));
    sketch.add_constraint(Constraint::Length(line, 4.0));

    let analysis = analyze_blocks(&sketch);
    assert_eq!(analysis.blocks.len(), 1);
    assert_eq!(analysis.blocks[0].status, BlockStatus::WellDetermined);
}

#[test]
fn solve_reports_over_constrained_for_conflicting_fixed_point() {
    let mut sketch = fixed_point_sketch(0.0, 0.0, 0.0, 0.0);
    sketch.add_constraint(Constraint::Fixed(cadkernel_sketch::PointId(0), 1.0, 0.0));

    let result = solve(&mut sketch, 20, TOL);
    assert!(!result.converged);
    assert!(result.over_constrained);
}

#[test]
fn analysis_totals_match_block_counts() {
    let mut sketch = fixed_point_sketch(0.0, 0.0, 0.0, 0.0);
    let p = sketch.add_point(2.0, 0.0);
    sketch.add_constraint(Constraint::Fixed(p, 2.0, 0.0));

    let analysis = analyze_blocks(&sketch);
    let block_equations: usize = analysis
        .blocks
        .iter()
        .map(|block| block.equation_count)
        .sum();
    assert_eq!(analysis.total_variables, 4);
    assert_eq!(analysis.total_equations, block_equations);
    assert_eq!(analysis.total_rank, 4);
}

#[test]
fn solve_with_analysis_result_matches_plain_solve() {
    let mut with_analysis = fixed_point_sketch(5.0, -1.0, 8.0, 9.0);
    let mut plain = with_analysis.clone();

    let detailed = solve_with_analysis(&mut with_analysis, 20, TOL);
    let simple = solve(&mut plain, 20, TOL);
    assert_eq!(detailed.result.converged, simple.converged);
    assert_eq!(detailed.result.remaining_dof, simple.remaining_dof);
    assert_eq!(detailed.result.over_constrained, simple.over_constrained);
}

#[test]
fn deflation_step_records_block_convergence() {
    let mut sketch = fixed_point_sketch(5.0, 6.0, 1.0, 2.0);
    let result = solve_with_analysis(&mut sketch, 20, TOL);
    assert_eq!(result.analysis.deflation_steps.len(), 1);
    assert!(result.analysis.deflation_steps[0].converged);
    assert!(result.analysis.deflation_steps[0].residual < TOL);
}
