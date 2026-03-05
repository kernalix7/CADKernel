//! 2D parametric sketch solver.
//!
//! Build sketches from points, lines, arcs, and circles, apply geometric and
//! dimensional constraints, then call [`solve`] to find positions that satisfy
//! all constraints simultaneously.

pub mod constraint;
pub mod entity;
pub mod profile;
pub mod solver;

pub use constraint::Constraint;
pub use entity::{
    ArcId, CircleId, LineId, PointId, SketchArc, SketchCircle, SketchLine, SketchPoint,
};
pub use profile::{WorkPlane, extract_profile};
pub use solver::{SolverResult, solve};

/// A 2D parametric sketch containing points, lines, arcs, circles,
/// and geometric/dimensional constraints.
///
/// Usage:
/// 1. Add entities with `add_point`, `add_line`, etc.
/// 2. Add constraints with `add_constraint`.
/// 3. Call [`solve`] to find positions satisfying all constraints.
/// 4. Call [`extract_profile`] to convert the result to a 3D point loop.
#[derive(Debug, Clone)]
pub struct Sketch {
    pub points: Vec<SketchPoint>,
    pub lines: Vec<SketchLine>,
    pub arcs: Vec<SketchArc>,
    pub circles: Vec<SketchCircle>,
    pub constraints: Vec<Constraint>,
}

impl Sketch {
    /// Creates an empty sketch with no entities or constraints.
    pub fn new() -> Self {
        Self {
            points: Vec::new(),
            lines: Vec::new(),
            arcs: Vec::new(),
            circles: Vec::new(),
            constraints: Vec::new(),
        }
    }

    /// Adds a free point at `(x, y)` and returns its id.
    pub fn add_point(&mut self, x: f64, y: f64) -> PointId {
        let id = PointId(self.points.len());
        self.points.push(SketchPoint::new(x, y));
        id
    }

    /// Adds a line segment between two existing points.
    pub fn add_line(&mut self, start: PointId, end: PointId) -> LineId {
        let id = LineId(self.lines.len());
        self.lines.push(SketchLine { start, end });
        id
    }

    /// Adds a circular arc defined by center, start/end points, radius, and angular span.
    pub fn add_arc(
        &mut self,
        center: PointId,
        start_point: PointId,
        end_point: PointId,
        radius: f64,
        start_angle: f64,
        end_angle: f64,
    ) -> ArcId {
        let id = ArcId(self.arcs.len());
        self.arcs.push(SketchArc {
            center,
            start_point,
            end_point,
            radius,
            start_angle,
            end_angle,
        });
        id
    }

    /// Adds a full circle with the given center point and radius.
    pub fn add_circle(&mut self, center: PointId, radius: f64) -> CircleId {
        let id = CircleId(self.circles.len());
        self.circles.push(SketchCircle { center, radius });
        id
    }

    /// Adds a geometric or dimensional constraint to the sketch.
    pub fn add_constraint(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
    }
}

impl Default for Sketch {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solve_square_with_constraints() {
        let mut sketch = Sketch::new();

        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(10.0, 0.5);
        let p2 = sketch.add_point(10.5, 10.0);
        let p3 = sketch.add_point(-0.5, 10.5);

        let l0 = sketch.add_line(p0, p1);
        let l1 = sketch.add_line(p1, p2);
        let l2 = sketch.add_line(p2, p3);
        let l3 = sketch.add_line(p3, p0);

        sketch.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
        sketch.add_constraint(Constraint::Horizontal(l0));
        sketch.add_constraint(Constraint::Vertical(l1));
        sketch.add_constraint(Constraint::Horizontal(l2));
        sketch.add_constraint(Constraint::Vertical(l3));
        sketch.add_constraint(Constraint::Length(l0, 10.0));
        sketch.add_constraint(Constraint::Length(l1, 10.0));

        let result = solve(&mut sketch, 200, 1e-10);
        assert!(result.converged, "solver did not converge: {:?}", result);

        let pts: Vec<_> = sketch.points.iter().map(|p| p.position).collect();
        assert!((pts[0].x).abs() < 1e-6);
        assert!((pts[0].y).abs() < 1e-6);
        assert!((pts[1].x - 10.0).abs() < 1e-6);
        assert!((pts[1].y).abs() < 1e-6);
        assert!((pts[2].x - 10.0).abs() < 1e-6);
        assert!((pts[2].y - 10.0).abs() < 1e-6);
        assert!((pts[3].x).abs() < 1e-6);
        assert!((pts[3].y - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_solve_parallel_lines() {
        let mut sketch = Sketch::new();

        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(5.0, 0.0);
        let p2 = sketch.add_point(0.0, 3.0);
        let p3 = sketch.add_point(5.0, 3.5);

        let l0 = sketch.add_line(p0, p1);
        let l1 = sketch.add_line(p2, p3);

        sketch.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
        sketch.add_constraint(Constraint::Fixed(p1, 5.0, 0.0));
        sketch.add_constraint(Constraint::Fixed(p2, 0.0, 3.0));
        sketch.add_constraint(Constraint::Parallel(l0, l1));
        sketch.add_constraint(Constraint::Length(l1, 5.0));

        let result = solve(&mut sketch, 200, 1e-10);
        assert!(result.converged, "solver did not converge: {:?}", result);

        let dy = (sketch.points[p3.0].position.y - sketch.points[p2.0].position.y).abs();
        assert!(dy < 1e-6, "lines not parallel: dy = {dy}");
    }

    #[test]
    fn test_full_pipeline_sketch_to_profile() {
        let mut sketch = Sketch::new();

        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(4.0, 0.5);
        let p2 = sketch.add_point(4.5, 3.0);
        let p3 = sketch.add_point(-0.5, 3.5);

        let l0 = sketch.add_line(p0, p1);
        let l1 = sketch.add_line(p1, p2);
        let l2 = sketch.add_line(p2, p3);
        let l3 = sketch.add_line(p3, p0);

        sketch.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
        sketch.add_constraint(Constraint::Horizontal(l0));
        sketch.add_constraint(Constraint::Vertical(l1));
        sketch.add_constraint(Constraint::Horizontal(l2));
        sketch.add_constraint(Constraint::Vertical(l3));
        sketch.add_constraint(Constraint::Length(l0, 4.0));
        sketch.add_constraint(Constraint::Length(l1, 3.0));

        let result = solve(&mut sketch, 200, 1e-10);
        assert!(result.converged);

        let wp = WorkPlane::xy();
        let profile = extract_profile(&sketch, &wp);
        assert_eq!(profile.len(), 4);
    }
}
