use crate::entity::{LineId, PointId};

/// Every constraint the sketch solver supports.
#[derive(Debug, Clone)]
pub enum Constraint {
    /// Two points share the same location.
    Coincident(PointId, PointId),

    /// A line is horizontal (dy = 0).
    Horizontal(LineId),

    /// A line is vertical (dx = 0).
    Vertical(LineId),

    /// Two lines are parallel.
    Parallel(LineId, LineId),

    /// Two lines are perpendicular.
    Perpendicular(LineId, LineId),

    /// A point lies on a line.
    PointOnLine(PointId, LineId),

    /// A point lies on a circle.
    PointOnCircle(PointId, PointId, f64),

    /// Two points are symmetric about a line.
    Symmetric(PointId, PointId, LineId),

    /// The distance between two points equals `d`.
    Distance(PointId, PointId, f64),

    /// The angle between two lines equals `theta` (radians).
    Angle(LineId, LineId, f64),

    /// A circle/arc has the given radius.
    Radius(PointId, PointId, f64),

    /// The length of a line segment equals `l`.
    Length(LineId, f64),

    /// Pin a point to a fixed (x, y) location.
    Fixed(PointId, f64, f64),

    /// A line is tangent to a circle (center + radius).
    Tangent(LineId, PointId, f64),

    /// Two lines have equal length.
    EqualLength(LineId, LineId),

    /// A point is the midpoint of a line segment.
    Midpoint(PointId, LineId),

    /// Two lines are collinear (lie on the same infinite line).
    Collinear(LineId, LineId),

    /// Two circles/arcs have equal radius.
    EqualRadius(PointId, PointId, PointId, PointId),

    /// Two circles/arcs share the same center point.
    Concentric(PointId, PointId),

    /// Circle/arc diameter equals d (diameter = 2 * radius).
    Diameter(PointId, PointId, f64),

    /// Lock all DOFs of a point (equivalent to Fixed but conceptually different).
    Block(PointId, f64, f64),

    /// Horizontal distance between two points equals d.
    HorizontalDistance(PointId, PointId, f64),

    /// Vertical distance between two points equals d.
    VerticalDistance(PointId, PointId, f64),

    /// A point lies on an edge/line (same as PointOnLine but conceptually for any curve).
    PointOnObject(PointId, LineId),
}

/// Trait implemented by each constraint variant so the solver can evaluate
/// the residual vector and the Jacobian contribution in a uniform way.
pub trait ConstraintEval {
    /// Number of scalar equations this constraint contributes.
    fn num_equations(&self) -> usize;

    /// Writes the constraint residual(s) into `out[0..num_equations()]`.
    fn residual(&self, vars: &[f64], out: &mut [f64]);

    /// Sparse Jacobian entries: `(row_offset + local_row, col, value)`.
    fn jacobian(&self, vars: &[f64], row_offset: usize, out: &mut Vec<(usize, usize, f64)>);
}

fn px(id: PointId) -> usize {
    id.0 * 2
}
fn py(id: PointId) -> usize {
    id.0 * 2 + 1
}

/// A constraint together with the line-endpoint lookup table needed for evaluation.
pub struct ConstraintWithCtx<'a> {
    pub constraint: &'a Constraint,
    pub lines: &'a [(PointId, PointId)],
}

impl ConstraintEval for ConstraintWithCtx<'_> {
    fn num_equations(&self) -> usize {
        match self.constraint {
            Constraint::Coincident(..) => 2,
            Constraint::Horizontal(_) => 1,
            Constraint::Vertical(_) => 1,
            Constraint::Parallel(..) => 1,
            Constraint::Perpendicular(..) => 1,
            Constraint::PointOnLine(..) => 1,
            Constraint::PointOnCircle(..) => 1,
            Constraint::Symmetric(..) => 2,
            Constraint::Distance(..) => 1,
            Constraint::Angle(..) => 1,
            Constraint::Radius(..) => 1,
            Constraint::Length(..) => 1,
            Constraint::Fixed(..) => 2,
            Constraint::Tangent(..) => 1,
            Constraint::EqualLength(..) => 1,
            Constraint::Midpoint(..) => 2,
            Constraint::Collinear(..) => 2,
            Constraint::EqualRadius(..) => 1,
            Constraint::Concentric(..) => 2,
            Constraint::Diameter(..) => 1,
            Constraint::Block(..) => 2,
            Constraint::HorizontalDistance(..) => 1,
            Constraint::VerticalDistance(..) => 1,
            Constraint::PointOnObject(..) => 1,
        }
    }

    fn residual(&self, vars: &[f64], out: &mut [f64]) {
        match *self.constraint {
            Constraint::Coincident(p1, p2) => {
                out[0] = vars[px(p1)] - vars[px(p2)];
                out[1] = vars[py(p1)] - vars[py(p2)];
            }
            Constraint::Horizontal(lid) => {
                let (s, e) = self.lines[lid.0];
                out[0] = vars[py(s)] - vars[py(e)];
            }
            Constraint::Vertical(lid) => {
                let (s, e) = self.lines[lid.0];
                out[0] = vars[px(s)] - vars[px(e)];
            }
            Constraint::Parallel(l1, l2) => {
                let (s1, e1) = self.lines[l1.0];
                let (s2, e2) = self.lines[l2.0];
                let dx1 = vars[px(e1)] - vars[px(s1)];
                let dy1 = vars[py(e1)] - vars[py(s1)];
                let dx2 = vars[px(e2)] - vars[px(s2)];
                let dy2 = vars[py(e2)] - vars[py(s2)];
                out[0] = dx1 * dy2 - dy1 * dx2;
            }
            Constraint::Perpendicular(l1, l2) => {
                let (s1, e1) = self.lines[l1.0];
                let (s2, e2) = self.lines[l2.0];
                let dx1 = vars[px(e1)] - vars[px(s1)];
                let dy1 = vars[py(e1)] - vars[py(s1)];
                let dx2 = vars[px(e2)] - vars[px(s2)];
                let dy2 = vars[py(e2)] - vars[py(s2)];
                out[0] = dx1 * dx2 + dy1 * dy2;
            }
            Constraint::PointOnLine(p, lid) => {
                let (s, e) = self.lines[lid.0];
                let dx = vars[px(e)] - vars[px(s)];
                let dy = vars[py(e)] - vars[py(s)];
                let dpx = vars[px(p)] - vars[px(s)];
                let dpy = vars[py(p)] - vars[py(s)];
                out[0] = dpx * dy - dpy * dx;
            }
            Constraint::PointOnCircle(p, center, radius) => {
                let dx = vars[px(p)] - vars[px(center)];
                let dy = vars[py(p)] - vars[py(center)];
                out[0] = dx * dx + dy * dy - radius * radius;
            }
            Constraint::Symmetric(p1, p2, lid) => {
                let (s, e) = self.lines[lid.0];
                let mx = (vars[px(p1)] + vars[px(p2)]) * 0.5;
                let my = (vars[py(p1)] + vars[py(p2)]) * 0.5;
                let ldx = vars[px(e)] - vars[px(s)];
                let ldy = vars[py(e)] - vars[py(s)];
                let pmx = vars[px(p2)] - vars[px(p1)];
                let pmy = vars[py(p2)] - vars[py(p1)];
                // midpoint lies on line
                let dsx = mx - vars[px(s)];
                let dsy = my - vars[py(s)];
                out[0] = dsx * ldy - dsy * ldx;
                // p1-p2 direction perpendicular to line
                out[1] = pmx * ldx + pmy * ldy;
            }
            Constraint::Distance(p1, p2, d) => {
                let dx = vars[px(p1)] - vars[px(p2)];
                let dy = vars[py(p1)] - vars[py(p2)];
                out[0] = dx * dx + dy * dy - d * d;
            }
            Constraint::Angle(l1, l2, theta) => {
                let (s1, e1) = self.lines[l1.0];
                let (s2, e2) = self.lines[l2.0];
                let dx1 = vars[px(e1)] - vars[px(s1)];
                let dy1 = vars[py(e1)] - vars[py(s1)];
                let dx2 = vars[px(e2)] - vars[px(s2)];
                let dy2 = vars[py(e2)] - vars[py(s2)];
                let dot = dx1 * dx2 + dy1 * dy2;
                let cross = dx1 * dy2 - dy1 * dx2;
                out[0] = cross.atan2(dot) - theta;
            }
            Constraint::Radius(p, center, r) => {
                let dx = vars[px(p)] - vars[px(center)];
                let dy = vars[py(p)] - vars[py(center)];
                out[0] = dx * dx + dy * dy - r * r;
            }
            Constraint::Length(lid, l) => {
                let (s, e) = self.lines[lid.0];
                let dx = vars[px(e)] - vars[px(s)];
                let dy = vars[py(e)] - vars[py(s)];
                out[0] = dx * dx + dy * dy - l * l;
            }
            Constraint::Fixed(p, fx, fy) => {
                out[0] = vars[px(p)] - fx;
                out[1] = vars[py(p)] - fy;
            }
            Constraint::Tangent(lid, center, radius) => {
                let (s, e) = self.lines[lid.0];
                let ldx = vars[px(e)] - vars[px(s)];
                let ldy = vars[py(e)] - vars[py(s)];
                let cpx = vars[px(center)] - vars[px(s)];
                let cpy = vars[py(center)] - vars[py(s)];
                let cross = cpx * ldy - cpy * ldx;
                let len_sq = ldx * ldx + ldy * ldy;
                out[0] = cross * cross - radius * radius * len_sq;
            }
            Constraint::EqualLength(l1, l2) => {
                let (s1, e1) = self.lines[l1.0];
                let (s2, e2) = self.lines[l2.0];
                let dx1 = vars[px(e1)] - vars[px(s1)];
                let dy1 = vars[py(e1)] - vars[py(s1)];
                let dx2 = vars[px(e2)] - vars[px(s2)];
                let dy2 = vars[py(e2)] - vars[py(s2)];
                out[0] = (dx1 * dx1 + dy1 * dy1) - (dx2 * dx2 + dy2 * dy2);
            }
            Constraint::Midpoint(p, lid) => {
                let (s, e) = self.lines[lid.0];
                out[0] = vars[px(p)] - (vars[px(s)] + vars[px(e)]) * 0.5;
                out[1] = vars[py(p)] - (vars[py(s)] + vars[py(e)]) * 0.5;
            }
            Constraint::Collinear(l1, l2) => {
                let (s1, e1) = self.lines[l1.0];
                let (s2, _e2) = self.lines[l2.0];
                let dx1 = vars[px(e1)] - vars[px(s1)];
                let dy1 = vars[py(e1)] - vars[py(s1)];
                // s2 on line through s1-e1
                let dpx = vars[px(s2)] - vars[px(s1)];
                let dpy = vars[py(s2)] - vars[py(s1)];
                out[0] = dpx * dy1 - dpy * dx1;
                // l1 parallel to l2
                let (s2b, e2b) = self.lines[l2.0];
                let dx2 = vars[px(e2b)] - vars[px(s2b)];
                let dy2 = vars[py(e2b)] - vars[py(s2b)];
                out[1] = dx1 * dy2 - dy1 * dx2;
            }
            Constraint::EqualRadius(p1, c1, p2, c2) => {
                let dx1 = vars[px(p1)] - vars[px(c1)];
                let dy1 = vars[py(p1)] - vars[py(c1)];
                let dx2 = vars[px(p2)] - vars[px(c2)];
                let dy2 = vars[py(p2)] - vars[py(c2)];
                out[0] = (dx1 * dx1 + dy1 * dy1) - (dx2 * dx2 + dy2 * dy2);
            }
            Constraint::Concentric(c1, c2) => {
                out[0] = vars[px(c1)] - vars[px(c2)];
                out[1] = vars[py(c1)] - vars[py(c2)];
            }
            Constraint::Diameter(p, center, d) => {
                let dx = vars[px(p)] - vars[px(center)];
                let dy = vars[py(p)] - vars[py(center)];
                let r = d / 2.0;
                out[0] = dx * dx + dy * dy - r * r;
            }
            Constraint::Block(p, fx, fy) => {
                out[0] = vars[px(p)] - fx;
                out[1] = vars[py(p)] - fy;
            }
            Constraint::HorizontalDistance(p1, p2, d) => {
                out[0] = vars[px(p1)] - vars[px(p2)] - d;
            }
            Constraint::VerticalDistance(p1, p2, d) => {
                out[0] = vars[py(p1)] - vars[py(p2)] - d;
            }
            Constraint::PointOnObject(p, lid) => {
                let (s, e) = self.lines[lid.0];
                let dx = vars[px(e)] - vars[px(s)];
                let dy = vars[py(e)] - vars[py(s)];
                let dpx = vars[px(p)] - vars[px(s)];
                let dpy = vars[py(p)] - vars[py(s)];
                out[0] = dpx * dy - dpy * dx;
            }
        }
    }

    fn jacobian(&self, vars: &[f64], row: usize, out: &mut Vec<(usize, usize, f64)>) {
        match *self.constraint {
            Constraint::Coincident(p1, p2) => {
                out.push((row, px(p1), 1.0));
                out.push((row, px(p2), -1.0));
                out.push((row + 1, py(p1), 1.0));
                out.push((row + 1, py(p2), -1.0));
            }
            Constraint::Horizontal(lid) => {
                let (s, e) = self.lines[lid.0];
                out.push((row, py(s), 1.0));
                out.push((row, py(e), -1.0));
            }
            Constraint::Vertical(lid) => {
                let (s, e) = self.lines[lid.0];
                out.push((row, px(s), 1.0));
                out.push((row, px(e), -1.0));
            }
            Constraint::Parallel(l1, l2) => {
                let (s1, e1) = self.lines[l1.0];
                let (s2, e2) = self.lines[l2.0];
                let dx1 = vars[px(e1)] - vars[px(s1)];
                let dy1 = vars[py(e1)] - vars[py(s1)];
                let dx2 = vars[px(e2)] - vars[px(s2)];
                let dy2 = vars[py(e2)] - vars[py(s2)];
                // d/d(s1.x) = -dy2, d/d(e1.x) = dy2
                out.push((row, px(s1), -dy2));
                out.push((row, px(e1), dy2));
                out.push((row, py(s1), dx2));
                out.push((row, py(e1), -dx2));
                out.push((row, px(s2), dy1));
                out.push((row, px(e2), -dy1));
                out.push((row, py(s2), -dx1));
                out.push((row, py(e2), dx1));
            }
            Constraint::Perpendicular(l1, l2) => {
                let (s1, e1) = self.lines[l1.0];
                let (s2, e2) = self.lines[l2.0];
                let dx1 = vars[px(e1)] - vars[px(s1)];
                let dy1 = vars[py(e1)] - vars[py(s1)];
                let dx2 = vars[px(e2)] - vars[px(s2)];
                let dy2 = vars[py(e2)] - vars[py(s2)];
                out.push((row, px(s1), -dx2));
                out.push((row, px(e1), dx2));
                out.push((row, py(s1), -dy2));
                out.push((row, py(e1), dy2));
                out.push((row, px(s2), -dx1));
                out.push((row, px(e2), dx1));
                out.push((row, py(s2), -dy1));
                out.push((row, py(e2), dy1));
            }
            Constraint::PointOnLine(p, lid) => {
                let (s, e) = self.lines[lid.0];
                let dx = vars[px(e)] - vars[px(s)];
                let dy = vars[py(e)] - vars[py(s)];
                let dpx = vars[px(p)] - vars[px(s)];
                let dpy = vars[py(p)] - vars[py(s)];
                out.push((row, px(p), dy));
                out.push((row, py(p), -dx));
                out.push((row, px(s), -dy + dpy));
                out.push((row, py(s), dx - dpx));
                out.push((row, px(e), -dpy));
                out.push((row, py(e), dpx));
            }
            Constraint::PointOnCircle(p, center, _) => {
                let dx = vars[px(p)] - vars[px(center)];
                let dy = vars[py(p)] - vars[py(center)];
                out.push((row, px(p), 2.0 * dx));
                out.push((row, py(p), 2.0 * dy));
                out.push((row, px(center), -2.0 * dx));
                out.push((row, py(center), -2.0 * dy));
            }
            Constraint::Symmetric(p1, p2, lid) => {
                let (s, e) = self.lines[lid.0];
                let ldx = vars[px(e)] - vars[px(s)];
                let ldy = vars[py(e)] - vars[py(s)];
                // Row 0: midpoint on line
                out.push((row, px(p1), 0.5 * ldy));
                out.push((row, px(p2), 0.5 * ldy));
                out.push((row, py(p1), -0.5 * ldx));
                out.push((row, py(p2), -0.5 * ldx));
                let mx = (vars[px(p1)] + vars[px(p2)]) * 0.5;
                let my = (vars[py(p1)] + vars[py(p2)]) * 0.5;
                let dsx = mx - vars[px(s)];
                let dsy = my - vars[py(s)];
                out.push((row, px(s), -ldy + dsy));
                out.push((row, py(s), ldx - dsx));
                out.push((row, px(e), -dsy));
                out.push((row, py(e), dsx));
                // Row 1: p1-p2 perpendicular to line
                out.push((row + 1, px(p2), ldx));
                out.push((row + 1, px(p1), -ldx));
                out.push((row + 1, py(p2), ldy));
                out.push((row + 1, py(p1), -ldy));
                let pmx = vars[px(p2)] - vars[px(p1)];
                let pmy = vars[py(p2)] - vars[py(p1)];
                out.push((row + 1, px(s), -pmx));
                out.push((row + 1, py(s), -pmy));
                out.push((row + 1, px(e), pmx));
                out.push((row + 1, py(e), pmy));
            }
            Constraint::Distance(p1, p2, _) => {
                let dx = vars[px(p1)] - vars[px(p2)];
                let dy = vars[py(p1)] - vars[py(p2)];
                out.push((row, px(p1), 2.0 * dx));
                out.push((row, py(p1), 2.0 * dy));
                out.push((row, px(p2), -2.0 * dx));
                out.push((row, py(p2), -2.0 * dy));
            }
            Constraint::Angle(l1, l2, theta) => {
                let (s1, e1) = self.lines[l1.0];
                let (s2, e2) = self.lines[l2.0];
                let dx1 = vars[px(e1)] - vars[px(s1)];
                let dy1 = vars[py(e1)] - vars[py(s1)];
                let dx2 = vars[px(e2)] - vars[px(s2)];
                let dy2 = vars[py(e2)] - vars[py(s2)];
                let t = theta.tan();
                // f = cross - dot * tan(theta)
                // cross = dx1*dy2 - dy1*dx2
                // dot   = dx1*dx2 + dy1*dy2
                out.push((row, px(s1), -(dy2 - dx2 * t)));
                out.push((row, px(e1), dy2 - dx2 * t));
                out.push((row, py(s1), dx2 + dy2 * t));
                out.push((row, py(e1), -(dx2 + dy2 * t)));
                out.push((row, px(s2), dy1 + dx1 * t));
                out.push((row, px(e2), -(dy1 + dx1 * t)));
                out.push((row, py(s2), -(dx1 - dy1 * t)));
                out.push((row, py(e2), dx1 - dy1 * t));
            }
            Constraint::Radius(p, center, _) => {
                let dx = vars[px(p)] - vars[px(center)];
                let dy = vars[py(p)] - vars[py(center)];
                out.push((row, px(p), 2.0 * dx));
                out.push((row, py(p), 2.0 * dy));
                out.push((row, px(center), -2.0 * dx));
                out.push((row, py(center), -2.0 * dy));
            }
            Constraint::Length(lid, _) => {
                let (s, e) = self.lines[lid.0];
                let dx = vars[px(e)] - vars[px(s)];
                let dy = vars[py(e)] - vars[py(s)];
                out.push((row, px(s), -2.0 * dx));
                out.push((row, py(s), -2.0 * dy));
                out.push((row, px(e), 2.0 * dx));
                out.push((row, py(e), 2.0 * dy));
            }
            Constraint::Fixed(p, _, _) => {
                out.push((row, px(p), 1.0));
                out.push((row + 1, py(p), 1.0));
            }
            Constraint::Tangent(lid, center, radius) => {
                let (s, e) = self.lines[lid.0];
                let ldx = vars[px(e)] - vars[px(s)];
                let ldy = vars[py(e)] - vars[py(s)];
                let cpx = vars[px(center)] - vars[px(s)];
                let cpy = vars[py(center)] - vars[py(s)];
                let cross = cpx * ldy - cpy * ldx;
                let r2 = radius * radius;
                out.push((row, px(center), 2.0 * cross * ldy));
                out.push((row, py(center), -2.0 * cross * ldx));
                out.push((
                    row,
                    px(s),
                    -2.0 * cross * ldy + 2.0 * cross * cpy + 2.0 * r2 * ldx,
                ));
                out.push((
                    row,
                    py(s),
                    2.0 * cross * ldx - 2.0 * cross * cpx + 2.0 * r2 * ldy,
                ));
                out.push((row, px(e), -2.0 * cross * cpy - 2.0 * r2 * ldx));
                out.push((row, py(e), 2.0 * cross * cpx - 2.0 * r2 * ldy));
            }
            Constraint::EqualLength(l1, l2) => {
                let (s1, e1) = self.lines[l1.0];
                let (s2, e2) = self.lines[l2.0];
                let dx1 = vars[px(e1)] - vars[px(s1)];
                let dy1 = vars[py(e1)] - vars[py(s1)];
                let dx2 = vars[px(e2)] - vars[px(s2)];
                let dy2 = vars[py(e2)] - vars[py(s2)];
                out.push((row, px(s1), -2.0 * dx1));
                out.push((row, py(s1), -2.0 * dy1));
                out.push((row, px(e1), 2.0 * dx1));
                out.push((row, py(e1), 2.0 * dy1));
                out.push((row, px(s2), 2.0 * dx2));
                out.push((row, py(s2), 2.0 * dy2));
                out.push((row, px(e2), -2.0 * dx2));
                out.push((row, py(e2), -2.0 * dy2));
            }
            Constraint::Midpoint(p, lid) => {
                let (s, e) = self.lines[lid.0];
                out.push((row, px(p), 1.0));
                out.push((row, px(s), -0.5));
                out.push((row, px(e), -0.5));
                out.push((row + 1, py(p), 1.0));
                out.push((row + 1, py(s), -0.5));
                out.push((row + 1, py(e), -0.5));
            }
            Constraint::Collinear(l1, l2) => {
                let (s1, e1) = self.lines[l1.0];
                let (s2, e2) = self.lines[l2.0];
                let dx1 = vars[px(e1)] - vars[px(s1)];
                let dy1 = vars[py(e1)] - vars[py(s1)];
                let dpx = vars[px(s2)] - vars[px(s1)];
                let dpy = vars[py(s2)] - vars[py(s1)];
                // Row 0: s2 on line s1-e1
                out.push((row, px(s2), dy1));
                out.push((row, py(s2), -dx1));
                out.push((row, px(s1), -dy1 + dpy));
                out.push((row, py(s1), dx1 - dpx));
                out.push((row, px(e1), -dpy));
                out.push((row, py(e1), dpx));
                // Row 1: parallel (same as Parallel)
                let dx2 = vars[px(e2)] - vars[px(s2)];
                let dy2 = vars[py(e2)] - vars[py(s2)];
                out.push((row + 1, px(s1), -dy2));
                out.push((row + 1, px(e1), dy2));
                out.push((row + 1, py(s1), dx2));
                out.push((row + 1, py(e1), -dx2));
                out.push((row + 1, px(s2), dy1));
                out.push((row + 1, px(e2), -dy1));
                out.push((row + 1, py(s2), -dx1));
                out.push((row + 1, py(e2), dx1));
            }
            Constraint::EqualRadius(p1, c1, p2, c2) => {
                let dx1 = vars[px(p1)] - vars[px(c1)];
                let dy1 = vars[py(p1)] - vars[py(c1)];
                let dx2 = vars[px(p2)] - vars[px(c2)];
                let dy2 = vars[py(p2)] - vars[py(c2)];
                out.push((row, px(p1), 2.0 * dx1));
                out.push((row, py(p1), 2.0 * dy1));
                out.push((row, px(c1), -2.0 * dx1));
                out.push((row, py(c1), -2.0 * dy1));
                out.push((row, px(p2), -2.0 * dx2));
                out.push((row, py(p2), -2.0 * dy2));
                out.push((row, px(c2), 2.0 * dx2));
                out.push((row, py(c2), 2.0 * dy2));
            }
            Constraint::Concentric(c1, c2) => {
                out.push((row, px(c1), 1.0));
                out.push((row, px(c2), -1.0));
                out.push((row + 1, py(c1), 1.0));
                out.push((row + 1, py(c2), -1.0));
            }
            Constraint::Diameter(p, center, _) => {
                let dx = vars[px(p)] - vars[px(center)];
                let dy = vars[py(p)] - vars[py(center)];
                out.push((row, px(p), 2.0 * dx));
                out.push((row, py(p), 2.0 * dy));
                out.push((row, px(center), -2.0 * dx));
                out.push((row, py(center), -2.0 * dy));
            }
            Constraint::Block(p, _, _) => {
                out.push((row, px(p), 1.0));
                out.push((row + 1, py(p), 1.0));
            }
            Constraint::HorizontalDistance(p1, p2, _) => {
                out.push((row, px(p1), 1.0));
                out.push((row, px(p2), -1.0));
            }
            Constraint::VerticalDistance(p1, p2, _) => {
                out.push((row, py(p1), 1.0));
                out.push((row, py(p2), -1.0));
            }
            Constraint::PointOnObject(p, lid) => {
                let (s, e) = self.lines[lid.0];
                let dx = vars[px(e)] - vars[px(s)];
                let dy = vars[py(e)] - vars[py(s)];
                let dpx = vars[px(p)] - vars[px(s)];
                let dpy = vars[py(p)] - vars[py(s)];
                out.push((row, px(p), dy));
                out.push((row, py(p), -dx));
                out.push((row, px(s), -dy + dpy));
                out.push((row, py(s), dx - dpx));
                out.push((row, px(e), -dpy));
                out.push((row, py(e), dpx));
            }
        }
    }
}
