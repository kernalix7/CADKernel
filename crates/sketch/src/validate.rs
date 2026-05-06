//! Sketch validation: checks for consistency issues.

use std::collections::HashMap;

use crate::{Constraint, Sketch};

/// Result of sketch validation.
///
/// Contains a validity flag and a list of specific issues found.
#[derive(Debug, Clone)]
pub struct SketchValidation {
    /// `true` if no errors were found.
    pub valid: bool,
    /// All issues detected during validation.
    pub issues: Vec<SketchValidationIssue>,
}

impl SketchValidation {
    /// Number of actionable diagnostics to surface in the interactive UI.
    pub fn diagnostic_issue_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.should_show_warning())
            .count()
    }

    /// Compact English status text suitable for Sketcher banners/status bars.
    pub fn status_label(&self) -> String {
        self.issues
            .iter()
            .find(|issue| issue.should_show_warning())
            .or_else(|| self.issues.first())
            .map_or_else(
                || "Constraints: healthy".to_string(),
                |issue| issue.status_label(),
            )
    }
}

/// A specific issue found during sketch validation.
///
/// Categorises problems such as invalid references, degenerate geometry,
/// and under/over-constrained systems.
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
    /// Two constraints are identical and one can be removed.
    DuplicateConstraint {
        first_index: usize,
        duplicate_index: usize,
    },
    /// Two dimensional constraints target the same entity with incompatible values.
    ConflictingConstraintValue {
        first_index: usize,
        second_index: usize,
    },
    /// A dimensional constraint has a non-finite or non-positive value.
    InvalidConstraintValue { constraint_index: usize },
    /// Sketch has no geometry.
    EmptySketch,
    /// Sketch is over-constrained (more equations than DOFs).
    OverConstrained { dof: usize, equations: usize },
    /// Sketch is under-constrained.
    UnderConstrained { dof: usize, equations: usize },
}

impl SketchValidationIssue {
    /// Returns true when the issue should be highlighted beyond the normal DOF readout.
    pub fn should_show_warning(&self) -> bool {
        !matches!(self, Self::EmptySketch | Self::UnderConstrained { .. })
    }

    /// Compact English status text for UI banners and diagnostics.
    pub fn status_label(&self) -> String {
        match self {
            Self::InvalidPointReference { constraint_index } => {
                format!("Constraints: invalid point ref #{constraint_index}")
            }
            Self::InvalidLineReference { constraint_index } => {
                format!("Constraints: invalid line ref #{constraint_index}")
            }
            Self::NearlyCoincidentPoints { .. } => "Constraints: nearly coincident points".into(),
            Self::ZeroLengthLine { line_index } => {
                format!("Constraints: zero-length line #{line_index}")
            }
            Self::DuplicateConstraint {
                first_index,
                duplicate_index,
            } => format!("Constraints: duplicate #{first_index}/#{duplicate_index}"),
            Self::ConflictingConstraintValue {
                first_index,
                second_index,
            } => format!("Constraints: conflicting values #{first_index}/#{second_index}"),
            Self::InvalidConstraintValue { constraint_index } => {
                format!("Constraints: invalid value #{constraint_index}")
            }
            Self::EmptySketch => "Constraints: no geometry".into(),
            Self::OverConstrained { dof, equations } => {
                format!("Constraints: over-constrained ({equations}>{dof})")
            }
            Self::UnderConstrained { dof, equations } => {
                format!("Constraints: under-constrained ({equations}/{dof})")
            }
        }
    }
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

    validate_constraint_diagnostics(sketch, tolerance, &mut issues);

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ConstraintSignature {
    Coincident([usize; 2]),
    Horizontal(usize),
    Vertical(usize),
    Parallel([usize; 2]),
    Perpendicular([usize; 2]),
    PointOnLine(usize, usize),
    PointOnCircle(usize, usize, u64),
    Symmetric([usize; 2], usize),
    Distance([usize; 2], u64),
    Angle(usize, usize, u64),
    Radius(usize, usize, u64),
    Length(usize, u64),
    Fixed(usize, u64, u64),
    Tangent(usize, usize, u64),
    EqualLength([usize; 2]),
    Midpoint(usize, usize),
    Collinear([usize; 2]),
    EqualRadius([usize; 2], [usize; 2]),
    Concentric([usize; 2]),
    Diameter(usize, usize, u64),
    Block(usize, u64, u64),
    HorizontalDistance(usize, usize, u64),
    VerticalDistance(usize, usize, u64),
    PointOnObject(usize, usize),
    Refraction(usize, usize, u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ConstraintValueTarget {
    Distance([usize; 2]),
    Angle(usize, usize),
    Radius(usize, usize),
    Length(usize),
    FixedPoint(usize),
    Tangent(usize, usize),
    Diameter(usize, usize),
    HorizontalDistance(usize, usize),
    VerticalDistance(usize, usize),
    PointOnCircle(usize, usize),
    Refraction(usize, usize),
}

#[derive(Debug, Clone, Copy)]
enum ConstraintValue {
    Scalar(f64),
    Point(f64, f64),
}

fn validate_constraint_diagnostics(
    sketch: &Sketch,
    tolerance: f64,
    issues: &mut Vec<SketchValidationIssue>,
) {
    let mut signatures: HashMap<ConstraintSignature, usize> = HashMap::new();
    let mut value_targets: HashMap<ConstraintValueTarget, (usize, ConstraintValue)> =
        HashMap::new();
    let value_tolerance = tolerance.max(1e-9);

    for (ci, constraint) in sketch.constraints.iter().enumerate() {
        if has_invalid_constraint_value(constraint) {
            issues.push(SketchValidationIssue::InvalidConstraintValue {
                constraint_index: ci,
            });
        }

        let signature = constraint_signature(constraint);
        if let Some(first_index) = signatures.insert(signature, ci) {
            issues.push(SketchValidationIssue::DuplicateConstraint {
                first_index,
                duplicate_index: ci,
            });
        }

        if let Some((target, value)) = constraint_value_target(constraint) {
            if let Some((first_index, first_value)) = value_targets.get(&target).copied() {
                if constraint_values_conflict(first_value, value, value_tolerance) {
                    issues.push(SketchValidationIssue::ConflictingConstraintValue {
                        first_index,
                        second_index: ci,
                    });
                }
            } else {
                value_targets.insert(target, (ci, value));
            }
        }
    }
}

fn constraint_signature(c: &Constraint) -> ConstraintSignature {
    match c {
        Constraint::Coincident(a, b) => ConstraintSignature::Coincident(canon_pair(a.0, b.0)),
        Constraint::Horizontal(l) => ConstraintSignature::Horizontal(l.0),
        Constraint::Vertical(l) => ConstraintSignature::Vertical(l.0),
        Constraint::Parallel(a, b) => ConstraintSignature::Parallel(canon_pair(a.0, b.0)),
        Constraint::Perpendicular(a, b) => ConstraintSignature::Perpendicular(canon_pair(a.0, b.0)),
        Constraint::PointOnLine(p, l) => ConstraintSignature::PointOnLine(p.0, l.0),
        Constraint::PointOnCircle(p, center, radius) => {
            ConstraintSignature::PointOnCircle(p.0, center.0, value_key(*radius))
        }
        Constraint::Symmetric(a, b, l) => ConstraintSignature::Symmetric(canon_pair(a.0, b.0), l.0),
        Constraint::Distance(a, b, distance) => {
            ConstraintSignature::Distance(canon_pair(a.0, b.0), value_key(*distance))
        }
        Constraint::Angle(a, b, angle) => ConstraintSignature::Angle(a.0, b.0, value_key(*angle)),
        Constraint::Radius(p, center, radius) => {
            ConstraintSignature::Radius(p.0, center.0, value_key(*radius))
        }
        Constraint::Length(l, length) => ConstraintSignature::Length(l.0, value_key(*length)),
        Constraint::Fixed(p, x, y) => ConstraintSignature::Fixed(p.0, value_key(*x), value_key(*y)),
        Constraint::Tangent(l, center, radius) => {
            ConstraintSignature::Tangent(l.0, center.0, value_key(*radius))
        }
        Constraint::EqualLength(a, b) => ConstraintSignature::EqualLength(canon_pair(a.0, b.0)),
        Constraint::Midpoint(p, l) => ConstraintSignature::Midpoint(p.0, l.0),
        Constraint::Collinear(a, b) => ConstraintSignature::Collinear(canon_pair(a.0, b.0)),
        Constraint::EqualRadius(a, b, c, d) => {
            let mut pairs = [canon_pair(a.0, b.0), canon_pair(c.0, d.0)];
            pairs.sort();
            ConstraintSignature::EqualRadius(pairs[0], pairs[1])
        }
        Constraint::Concentric(a, b) => ConstraintSignature::Concentric(canon_pair(a.0, b.0)),
        Constraint::Diameter(p, center, diameter) => {
            ConstraintSignature::Diameter(p.0, center.0, value_key(*diameter))
        }
        Constraint::Block(p, x, y) => ConstraintSignature::Block(p.0, value_key(*x), value_key(*y)),
        Constraint::HorizontalDistance(a, b, distance) => {
            ConstraintSignature::HorizontalDistance(a.0, b.0, value_key(*distance))
        }
        Constraint::VerticalDistance(a, b, distance) => {
            ConstraintSignature::VerticalDistance(a.0, b.0, value_key(*distance))
        }
        Constraint::PointOnObject(p, l) => ConstraintSignature::PointOnObject(p.0, l.0),
        Constraint::Refraction {
            line1,
            line2,
            ratio,
        } => ConstraintSignature::Refraction(line1.0, line2.0, value_key(*ratio)),
    }
}

fn constraint_value_target(c: &Constraint) -> Option<(ConstraintValueTarget, ConstraintValue)> {
    match c {
        Constraint::Distance(a, b, distance) => Some((
            ConstraintValueTarget::Distance(canon_pair(a.0, b.0)),
            ConstraintValue::Scalar(*distance),
        )),
        Constraint::Angle(a, b, angle) => Some((
            ConstraintValueTarget::Angle(a.0, b.0),
            ConstraintValue::Scalar(*angle),
        )),
        Constraint::Radius(p, center, radius) => Some((
            ConstraintValueTarget::Radius(p.0, center.0),
            ConstraintValue::Scalar(*radius),
        )),
        Constraint::Length(l, length) => Some((
            ConstraintValueTarget::Length(l.0),
            ConstraintValue::Scalar(*length),
        )),
        Constraint::Fixed(p, x, y) | Constraint::Block(p, x, y) => Some((
            ConstraintValueTarget::FixedPoint(p.0),
            ConstraintValue::Point(*x, *y),
        )),
        Constraint::Tangent(l, center, radius) => Some((
            ConstraintValueTarget::Tangent(l.0, center.0),
            ConstraintValue::Scalar(*radius),
        )),
        Constraint::Diameter(p, center, diameter) => Some((
            ConstraintValueTarget::Diameter(p.0, center.0),
            ConstraintValue::Scalar(*diameter),
        )),
        Constraint::HorizontalDistance(a, b, distance) => Some((
            ConstraintValueTarget::HorizontalDistance(a.0, b.0),
            ConstraintValue::Scalar(*distance),
        )),
        Constraint::VerticalDistance(a, b, distance) => Some((
            ConstraintValueTarget::VerticalDistance(a.0, b.0),
            ConstraintValue::Scalar(*distance),
        )),
        Constraint::PointOnCircle(p, center, radius) => Some((
            ConstraintValueTarget::PointOnCircle(p.0, center.0),
            ConstraintValue::Scalar(*radius),
        )),
        Constraint::Refraction {
            line1,
            line2,
            ratio,
        } => Some((
            ConstraintValueTarget::Refraction(line1.0, line2.0),
            ConstraintValue::Scalar(*ratio),
        )),
        _ => None,
    }
}

fn has_invalid_constraint_value(c: &Constraint) -> bool {
    match c {
        Constraint::PointOnCircle(_, _, radius)
        | Constraint::Distance(_, _, radius)
        | Constraint::Radius(_, _, radius)
        | Constraint::Length(_, radius)
        | Constraint::Tangent(_, _, radius)
        | Constraint::Diameter(_, _, radius) => !is_positive_finite(*radius),
        Constraint::Angle(_, _, angle)
        | Constraint::HorizontalDistance(_, _, angle)
        | Constraint::VerticalDistance(_, _, angle) => !angle.is_finite(),
        Constraint::Fixed(_, x, y) | Constraint::Block(_, x, y) => !x.is_finite() || !y.is_finite(),
        Constraint::Refraction { ratio, .. } => !is_positive_finite(*ratio),
        _ => false,
    }
}

fn constraint_values_conflict(a: ConstraintValue, b: ConstraintValue, tolerance: f64) -> bool {
    match (a, b) {
        (ConstraintValue::Scalar(a), ConstraintValue::Scalar(b)) => (a - b).abs() > tolerance,
        (ConstraintValue::Point(ax, ay), ConstraintValue::Point(bx, by)) => {
            (ax - bx).abs().max((ay - by).abs()) > tolerance
        }
        _ => false,
    }
}

fn is_positive_finite(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

fn value_key(value: f64) -> u64 {
    if value == 0.0 {
        0.0_f64.to_bits()
    } else {
        value.to_bits()
    }
}

fn canon_pair(a: usize, b: usize) -> [usize; 2] {
    if a <= b { [a, b] } else { [b, a] }
}

/// Extracts point and line index references from a constraint.
fn constraint_references(c: &Constraint) -> (Vec<usize>, Vec<usize>) {
    use Constraint::*;
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
        crate::Constraint::Refraction { line1, line2, .. } => (vec![], vec![line1.0, line2.0]),
    }
}

/// Counts the total number of constraint equations.
fn count_constraint_equations(sketch: &Sketch) -> usize {
    use crate::Constraint::*;
    sketch
        .constraints
        .iter()
        .map(|c| match c {
            Coincident(_, _)
            | Symmetric(_, _, _)
            | Fixed(_, _, _)
            | Block(_, _, _)
            | Midpoint(_, _)
            | Collinear(_, _)
            | Concentric(_, _) => 2,
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
        assert!(
            result
                .issues
                .iter()
                .any(|i| matches!(i, SketchValidationIssue::EmptySketch))
        );
    }

    #[test]
    fn test_validate_zero_length_line() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(1.0, 1.0);
        let p1 = sketch.add_point(1.0, 1.0);
        sketch.add_line(p0, p1);
        let result = validate_sketch(&sketch, 0.01);
        assert!(
            result
                .issues
                .iter()
                .any(|i| matches!(i, SketchValidationIssue::ZeroLengthLine { .. }))
        );
    }

    #[test]
    fn test_validate_nearly_coincident() {
        let mut sketch = Sketch::new();
        sketch.add_point(0.0, 0.0);
        sketch.add_point(0.001, 0.0);
        let result = validate_sketch(&sketch, 0.01);
        assert!(
            result
                .issues
                .iter()
                .any(|i| matches!(i, SketchValidationIssue::NearlyCoincidentPoints { .. }))
        );
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

    #[test]
    fn test_validate_duplicate_constraint_reports_indices() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(1.0, 0.0);
        let line = sketch.add_line(p0, p1);
        sketch.add_constraint(crate::Constraint::Horizontal(line));
        sketch.add_constraint(crate::Constraint::Horizontal(line));

        let result = validate_sketch(&sketch, 0.001);
        assert!(result.issues.iter().any(|issue| matches!(
            issue,
            SketchValidationIssue::DuplicateConstraint {
                first_index: 0,
                duplicate_index: 1
            }
        )));
        assert_eq!(result.diagnostic_issue_count(), 1);
        assert!(result.status_label().contains("duplicate"));
    }

    #[test]
    fn test_validate_conflicting_distance_constraints() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(1.0, 0.0);
        sketch.add_line(p0, p1);
        sketch.add_constraint(crate::Constraint::Distance(p0, p1, 1.0));
        sketch.add_constraint(crate::Constraint::Distance(p1, p0, 2.0));

        let result = validate_sketch(&sketch, 0.001);
        assert!(result.issues.iter().any(|issue| matches!(
            issue,
            SketchValidationIssue::ConflictingConstraintValue {
                first_index: 0,
                second_index: 1
            }
        )));
        assert!(result.status_label().contains("conflicting"));
    }

    #[test]
    fn test_validate_invalid_dimensional_value() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(1.0, 0.0);
        let line = sketch.add_line(p0, p1);
        sketch.add_constraint(crate::Constraint::Length(line, 0.0));

        let result = validate_sketch(&sketch, 0.001);
        assert!(result.issues.iter().any(|issue| matches!(
            issue,
            SketchValidationIssue::InvalidConstraintValue {
                constraint_index: 0
            }
        )));
        assert!(result.status_label().contains("invalid value"));
    }
}
