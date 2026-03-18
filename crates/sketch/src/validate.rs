//! Sketch validation: checks for consistency issues.

use crate::Sketch;

/// Result of sketch validation.
#[derive(Debug, Clone)]
pub struct SketchValidation {
    pub valid: bool,
    pub issues: Vec<SketchValidationIssue>,
}

/// A specific issue found during sketch validation.
#[derive(Debug, Clone)]
pub enum SketchValidationIssue {
    /// A constraint references a non-existent point.
    InvalidPointReference { constraint_index: usize },
    /// A constraint references a non-existent line.
    InvalidLineReference { constraint_index: usize },
    /// Two points are nearly coincident but not constrained.
    NearlyCoincidentPoints {
        point_a: usize,
        point_b: usize,
        distance: f64,
    },
    /// A line has zero length.
    ZeroLengthLine { line_index: usize },
    /// Sketch has no geometry.
    EmptySketch,
    /// Sketch is over-constrained (more equations than DOFs).
    OverConstrained {
        dof: usize,
        equations: usize,
    },
    /// Sketch is under-constrained.
    UnderConstrained {
        dof: usize,
        equations: usize,
    },
}

/// Validates a sketch for common issues.
pub fn validate_sketch(sketch: &Sketch, tolerance: f64) -> SketchValidation {
    let mut issues = Vec::new();

    // Empty check
    if sketch.points.is_empty() && sketch.lines.is_empty() {
        issues.push(SketchValidationIssue::EmptySketch);
        return SketchValidation {
            valid: false,
            issues,
        };
    }

    let num_points = sketch.points.len();
    let num_lines = sketch.lines.len();

    // Check constraint references
    for (ci, c) in sketch.constraints.iter().enumerate() {
        let (point_refs, line_refs) = constraint_references(c);
        for pid in point_refs {
            if pid >= num_points {
                issues.push(SketchValidationIssue::InvalidPointReference {
                    constraint_index: ci,
                });
            }
        }
        for lid in line_refs {
            if lid >= num_lines {
                issues.push(SketchValidationIssue::InvalidLineReference {
                    constraint_index: ci,
                });
            }
        }
    }

    // Check for zero-length lines
    for (li, line) in sketch.lines.iter().enumerate() {
        let p1 = &sketch.points[line.start.0].position;
        let p2 = &sketch.points[line.end.0].position;
        let dx = p2.x - p1.x;
        let dy = p2.y - p1.y;
        if dx * dx + dy * dy < tolerance * tolerance {
            issues.push(SketchValidationIssue::ZeroLengthLine { line_index: li });
        }
    }

    // Check for nearly coincident (unconstrained) points
    for i in 0..num_points {
        for j in (i + 1)..num_points {
            let pi = &sketch.points[i].position;
            let pj = &sketch.points[j].position;
            let dist = ((pi.x - pj.x).powi(2) + (pi.y - pj.y).powi(2)).sqrt();
            if dist < tolerance && dist > 0.0 {
                issues.push(SketchValidationIssue::NearlyCoincidentPoints {
                    point_a: i,
                    point_b: j,
                    distance: dist,
                });
            }
        }
    }

    // DOF analysis
    let dof = num_points * 2;
    let equations = count_constraint_equations(sketch);
    if equations > dof {
        issues.push(SketchValidationIssue::OverConstrained { dof, equations });
    } else if equations < dof && dof > 0 {
        issues.push(SketchValidationIssue::UnderConstrained { dof, equations });
    }

    SketchValidation {
        valid: issues.is_empty(),
        issues,
    }
}

/// Extracts point and line index references from a constraint.
fn constraint_references(c: &crate::Constraint) -> (Vec<usize>, Vec<usize>) {
    use crate::Constraint::*;
    match c {
        Coincident(a, b) => (vec![a.0, b.0], vec![]),
        Horizontal(l) | Vertical(l) => (vec![], vec![l.0]),
        Parallel(a, b) | Perpendicular(a, b) => (vec![], vec![a.0, b.0]),
        PointOnLine(p, l) | PointOnObject(p, l) => (vec![p.0], vec![l.0]),
        PointOnCircle(p, c, _) => (vec![p.0, c.0], vec![]),
        Symmetric(a, b, l) => (vec![a.0, b.0], vec![l.0]),
        EqualRadius(a, b, c, d) => (vec![a.0, b.0, c.0, d.0], vec![]),
        Distance(a, b, _) | Radius(a, b, _) | Diameter(a, b, _) => (vec![a.0, b.0], vec![]),
        Angle(a, b, _) | EqualLength(a, b) | Collinear(a, b) => (vec![], vec![a.0, b.0]),
        Length(l, _) => (vec![], vec![l.0]),
        Fixed(p, _, _) | Block(p, _, _) => (vec![p.0], vec![]),
        Tangent(l, p, _) => (vec![p.0], vec![l.0]),
        Midpoint(p, l) => (vec![p.0], vec![l.0]),
        Concentric(a, b) => (vec![a.0, b.0], vec![]),
        HorizontalDistance(a, b, _) | VerticalDistance(a, b, _) => (vec![a.0, b.0], vec![]),
    }
}

/// Counts the total number of constraint equations.
fn count_constraint_equations(sketch: &Sketch) -> usize {
    use crate::Constraint::*;
    sketch
        .constraints
        .iter()
        .map(|c| match c {
            Coincident(_, _) | Symmetric(_, _, _) | Fixed(_, _, _) | Block(_, _, _)
            | Midpoint(_, _) | Collinear(_, _) | Concentric(_, _) => 2,
            _ => 1,
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Sketch;

    #[test]
    fn test_validate_empty_sketch() {
        let sketch = Sketch::new();
        let result = validate_sketch(&sketch, 0.01);
        assert!(!result.valid);
        assert!(result
            .issues
            .iter()
            .any(|i| matches!(i, SketchValidationIssue::EmptySketch)));
    }

    #[test]
    fn test_validate_zero_length_line() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(1.0, 1.0);
        let p1 = sketch.add_point(1.0, 1.0);
        sketch.add_line(p0, p1);
        let result = validate_sketch(&sketch, 0.01);
        assert!(result
            .issues
            .iter()
            .any(|i| matches!(i, SketchValidationIssue::ZeroLengthLine { .. })));
    }

    #[test]
    fn test_validate_nearly_coincident() {
        let mut sketch = Sketch::new();
        sketch.add_point(0.0, 0.0);
        sketch.add_point(0.001, 0.0);
        let result = validate_sketch(&sketch, 0.01);
        assert!(result.issues.iter().any(
            |i| matches!(i, SketchValidationIssue::NearlyCoincidentPoints { .. })
        ));
    }

    #[test]
    fn test_validate_good_sketch() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(5.0, 0.0);
        sketch.add_line(p0, p1);
        sketch.add_constraint(crate::Constraint::Fixed(p0, 0.0, 0.0));
        sketch.add_constraint(crate::Constraint::Fixed(p1, 5.0, 0.0));
        let result = validate_sketch(&sketch, 0.001);
        // Should be valid (fully constrained, no issues)
        assert!(result.valid, "{:?}", result.issues);
    }
}
