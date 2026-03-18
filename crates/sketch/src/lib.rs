//! 2D parametric sketch solver.
//!
//! Build sketches from points, lines, arcs, and circles, apply geometric and
//! dimensional constraints, then call [`solve`] to find positions that satisfy
//! all constraints simultaneously.

pub mod constraint;
pub mod entity;
pub mod profile;
pub mod solver;
pub mod tools;
pub mod validate;

pub use constraint::Constraint;
pub use entity::{
    ArcId, BSplineId, CircleId, EllipseId, EllipticalArcId, HyperbolicArcId, LineId,
    ParabolicArcId, PointId, SketchArc, SketchBSpline, SketchCircle, SketchEllipse,
    SketchEllipticalArc, SketchHyperbolicArc, SketchLine, SketchParabolicArc, SketchPoint,
};
pub use profile::{WorkPlane, extract_profile};
pub use solver::{SolverResult, solve};
pub use tools::{
    FilletResult, SketchChamferResult, SplitResult, TrimResult, chamfer_sketch_corner,
    extend_edge, fillet_sketch_corner, split_edge, trim_edge,
};
pub use validate::{SketchValidation, SketchValidationIssue, validate_sketch};

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
    pub ellipses: Vec<SketchEllipse>,
    pub bsplines: Vec<SketchBSpline>,
    pub elliptical_arcs: Vec<SketchEllipticalArc>,
    pub hyperbolic_arcs: Vec<SketchHyperbolicArc>,
    pub parabolic_arcs: Vec<SketchParabolicArc>,
    pub constraints: Vec<Constraint>,
    /// If true, new geometry is created in construction mode.
    pub construction_mode: bool,
    /// Indices of points that are construction geometry (not used in profile).
    pub construction_points: Vec<PointId>,
    /// Indices of lines that are construction geometry.
    pub construction_lines: Vec<LineId>,
}

impl Sketch {
    /// Creates an empty sketch with no entities or constraints.
    pub fn new() -> Self {
        Self {
            points: Vec::new(),
            lines: Vec::new(),
            arcs: Vec::new(),
            circles: Vec::new(),
            ellipses: Vec::new(),
            bsplines: Vec::new(),
            elliptical_arcs: Vec::new(),
            hyperbolic_arcs: Vec::new(),
            parabolic_arcs: Vec::new(),
            constraints: Vec::new(),
            construction_mode: false,
            construction_points: Vec::new(),
            construction_lines: Vec::new(),
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

    /// Adds a sketch ellipse.
    pub fn add_ellipse(
        &mut self,
        center: PointId,
        major_end: PointId,
        minor_radius: f64,
    ) -> EllipseId {
        let id = EllipseId(self.ellipses.len());
        self.ellipses.push(SketchEllipse {
            center,
            major_end,
            minor_radius,
        });
        id
    }

    /// Adds a B-spline from control points.
    pub fn add_bspline(
        &mut self,
        control_points: Vec<PointId>,
        degree: usize,
        closed: bool,
    ) -> BSplineId {
        let id = BSplineId(self.bsplines.len());
        self.bsplines.push(SketchBSpline {
            control_points,
            degree,
            closed,
        });
        id
    }

    /// Adds an elliptical arc.
    #[allow(clippy::too_many_arguments)]
    pub fn add_elliptical_arc(
        &mut self,
        center: PointId,
        major_end: PointId,
        minor_radius: f64,
        start_point: PointId,
        end_point: PointId,
        start_param: f64,
        end_param: f64,
    ) -> EllipticalArcId {
        let id = EllipticalArcId(self.elliptical_arcs.len());
        self.elliptical_arcs.push(SketchEllipticalArc {
            center,
            major_end,
            minor_radius,
            start_point,
            end_point,
            start_param,
            end_param,
        });
        id
    }

    /// Adds a hyperbolic arc.
    #[allow(clippy::too_many_arguments)]
    pub fn add_hyperbolic_arc(
        &mut self,
        center: PointId,
        vertex: PointId,
        semi_minor: f64,
        start_point: PointId,
        end_point: PointId,
        start_param: f64,
        end_param: f64,
    ) -> HyperbolicArcId {
        let id = HyperbolicArcId(self.hyperbolic_arcs.len());
        self.hyperbolic_arcs.push(SketchHyperbolicArc {
            center,
            vertex,
            semi_minor,
            start_point,
            end_point,
            start_param,
            end_param,
        });
        id
    }

    /// Adds a parabolic arc.
    #[allow(clippy::too_many_arguments)]
    pub fn add_parabolic_arc(
        &mut self,
        vertex: PointId,
        focal_length: f64,
        focus_angle: f64,
        start_point: PointId,
        end_point: PointId,
        start_param: f64,
        end_param: f64,
    ) -> ParabolicArcId {
        let id = ParabolicArcId(self.parabolic_arcs.len());
        self.parabolic_arcs.push(SketchParabolicArc {
            vertex,
            focal_length,
            focus_angle,
            start_point,
            end_point,
            start_param,
            end_param,
        });
        id
    }

    /// Toggles construction mode on/off.
    pub fn toggle_construction_mode(&mut self) {
        self.construction_mode = !self.construction_mode;
    }

    /// Marks a point as construction geometry.
    pub fn mark_construction_point(&mut self, id: PointId) {
        if !self.construction_points.contains(&id) {
            self.construction_points.push(id);
        }
    }

    /// Marks a line as construction geometry.
    pub fn mark_construction_line(&mut self, id: LineId) {
        if !self.construction_lines.contains(&id) {
            self.construction_lines.push(id);
        }
    }

    /// Creates a circle from 3 points.
    pub fn add_circle_3pt(&mut self, p0: PointId, p1: PointId, p2: PointId) -> CircleId {
        let s = self.points[p0.0].position;
        let m = self.points[p1.0].position;
        let e = self.points[p2.0].position;

        let ax = s.x - e.x;
        let ay = s.y - e.y;
        let bx = m.x - e.x;
        let by = m.y - e.y;
        let d = 2.0 * (ax * by - ay * bx);

        let (cx, cy, radius) = if d.abs() < 1e-14 {
            let cx = (s.x + e.x) / 2.0;
            let cy = (s.y + e.y) / 2.0;
            let r = ((s.x - cx).powi(2) + (s.y - cy).powi(2)).sqrt();
            (cx, cy, r)
        } else {
            let ux = (by * (ax * ax + ay * ay) - ay * (bx * bx + by * by)) / d;
            let uy = (ax * (bx * bx + by * by) - bx * (ax * ax + ay * ay)) / d;
            let cx = e.x + ux;
            let cy = e.y + uy;
            let r = (ux * ux + uy * uy).sqrt();
            (cx, cy, r)
        };

        let center = self.add_point(cx, cy);
        self.add_circle(center, radius)
    }

    /// Creates an ellipse from 3 points (endpoints of one axis + a point on the ellipse).
    pub fn add_ellipse_3pt(
        &mut self,
        axis_start: PointId,
        axis_end: PointId,
        point_on: PointId,
    ) -> EllipseId {
        let a0 = self.points[axis_start.0].position;
        let a1 = self.points[axis_end.0].position;
        let p = self.points[point_on.0].position;

        let cx = (a0.x + a1.x) / 2.0;
        let cy = (a0.y + a1.y) / 2.0;
        let center = self.add_point(cx, cy);

        // Semi-major axis length
        let dx = a1.x - a0.x;
        let dy = a1.y - a0.y;
        let major_half = (dx * dx + dy * dy).sqrt() / 2.0;
        let major_end = self.add_point(a1.x, a1.y);

        // Project point_on onto local coordinates
        let angle = dy.atan2(dx);
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        let lx = (p.x - cx) * cos_a + (p.y - cy) * sin_a;
        let ly = -(p.x - cx) * sin_a + (p.y - cy) * cos_a;

        // From ellipse equation: (lx/a)² + (ly/b)² = 1
        let ratio_sq = 1.0 - (lx / major_half).powi(2);
        let minor_radius = if ratio_sq > 1e-14 {
            ly.abs() / ratio_sq.sqrt()
        } else {
            ly.abs().max(0.1)
        };

        self.add_ellipse(center, major_end, minor_radius)
    }

    /// Creates a centered rectangle (from center point, half-width, half-height).
    pub fn add_centered_rectangle(
        &mut self,
        cx: f64,
        cy: f64,
        half_w: f64,
        half_h: f64,
    ) -> (Vec<PointId>, Vec<LineId>) {
        let p0 = self.add_point(cx - half_w, cy - half_h);
        let p1 = self.add_point(cx + half_w, cy - half_h);
        let p2 = self.add_point(cx + half_w, cy + half_h);
        let p3 = self.add_point(cx - half_w, cy + half_h);
        let l0 = self.add_line(p0, p1);
        let l1 = self.add_line(p1, p2);
        let l2 = self.add_line(p2, p3);
        let l3 = self.add_line(p3, p0);
        (vec![p0, p1, p2, p3], vec![l0, l1, l2, l3])
    }

    /// Creates a rounded rectangle with corner arcs.
    pub fn add_rounded_rectangle(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        r: f64,
    ) -> (Vec<PointId>, Vec<LineId>, Vec<ArcId>) {
        let r = r.min(w / 2.0).min(h / 2.0);

        // Corner arc centers
        let c0 = self.add_point(x + r, y + r);
        let c1 = self.add_point(x + w - r, y + r);
        let c2 = self.add_point(x + w - r, y + h - r);
        let c3 = self.add_point(x + r, y + h - r);

        // Tangent points (8 points: 2 per corner)
        let p0 = self.add_point(x + r, y);
        let p1 = self.add_point(x + w - r, y);
        let p2 = self.add_point(x + w, y + r);
        let p3 = self.add_point(x + w, y + h - r);
        let p4 = self.add_point(x + w - r, y + h);
        let p5 = self.add_point(x + r, y + h);
        let p6 = self.add_point(x, y + h - r);
        let p7 = self.add_point(x, y + r);

        use std::f64::consts::{FRAC_PI_2, PI};

        // Bottom side, right side, top side, left side
        let l0 = self.add_line(p0, p1);
        let l1 = self.add_line(p2, p3);
        let l2 = self.add_line(p4, p5);
        let l3 = self.add_line(p6, p7);

        // Corner arcs (bottom-right, top-right, top-left, bottom-left)
        let a0 = self.add_arc(c1, p1, p2, r, -FRAC_PI_2, 0.0);
        let a1 = self.add_arc(c2, p3, p4, r, 0.0, FRAC_PI_2);
        let a2 = self.add_arc(c3, p5, p6, r, FRAC_PI_2, PI);
        let a3 = self.add_arc(c0, p7, p0, r, PI, PI + FRAC_PI_2);

        (
            vec![p0, p1, p2, p3, p4, p5, p6, p7],
            vec![l0, l1, l2, l3],
            vec![a0, a1, a2, a3],
        )
    }

    /// Creates an arc slot (arc-shaped slot with two parallel arcs and connecting arcs).
    pub fn add_arc_slot(
        &mut self,
        center_x: f64,
        center_y: f64,
        radius: f64,
        start_angle: f64,
        end_angle: f64,
        width: f64,
    ) -> (Vec<PointId>, Vec<ArcId>) {
        let half_w = width / 2.0;
        let r_inner = radius - half_w;
        let r_outer = radius + half_w;
        let center = self.add_point(center_x, center_y);

        // Inner arc endpoints
        let pi0 = self.add_point(
            center_x + r_inner * start_angle.cos(),
            center_y + r_inner * start_angle.sin(),
        );
        let pi1 = self.add_point(
            center_x + r_inner * end_angle.cos(),
            center_y + r_inner * end_angle.sin(),
        );

        // Outer arc endpoints
        let po0 = self.add_point(
            center_x + r_outer * start_angle.cos(),
            center_y + r_outer * start_angle.sin(),
        );
        let po1 = self.add_point(
            center_x + r_outer * end_angle.cos(),
            center_y + r_outer * end_angle.sin(),
        );

        // End cap centers
        let cap_start_cx = center_x + radius * start_angle.cos();
        let cap_start_cy = center_y + radius * start_angle.sin();
        let cap_end_cx = center_x + radius * end_angle.cos();
        let cap_end_cy = center_y + radius * end_angle.sin();
        let cap_start = self.add_point(cap_start_cx, cap_start_cy);
        let cap_end = self.add_point(cap_end_cx, cap_end_cy);

        // Inner arc
        let arc_inner = self.add_arc(center, pi0, pi1, r_inner, start_angle, end_angle);
        // Outer arc
        let arc_outer = self.add_arc(center, po0, po1, r_outer, start_angle, end_angle);
        // End cap arcs (semicircles)
        let cap_start_angle = start_angle + std::f64::consts::PI;
        let arc_cap_start = self.add_arc(
            cap_start, po0, pi0, half_w,
            cap_start_angle, cap_start_angle + std::f64::consts::PI,
        );
        let arc_cap_end = self.add_arc(
            cap_end, pi1, po1, half_w,
            end_angle, end_angle + std::f64::consts::PI,
        );

        (
            vec![pi0, pi1, po0, po1],
            vec![arc_inner, arc_outer, arc_cap_start, arc_cap_end],
        )
    }

    /// Adds a polyline (multi-segment line). Returns the created line IDs.
    pub fn add_polyline(&mut self, points: &[PointId]) -> Vec<LineId> {
        let mut lines = Vec::new();
        for w in points.windows(2) {
            lines.push(self.add_line(w[0], w[1]));
        }
        lines
    }

    /// Adds a regular polygon. Returns created point and line IDs.
    pub fn add_regular_polygon(
        &mut self,
        center_x: f64,
        center_y: f64,
        radius: f64,
        sides: usize,
    ) -> (Vec<PointId>, Vec<LineId>) {
        let mut pts = Vec::new();
        for i in 0..sides {
            let angle = std::f64::consts::TAU * i as f64 / sides as f64;
            let x = center_x + radius * angle.cos();
            let y = center_y + radius * angle.sin();
            pts.push(self.add_point(x, y));
        }
        let mut lines = Vec::new();
        for i in 0..sides {
            let j = (i + 1) % sides;
            lines.push(self.add_line(pts[i], pts[j]));
        }
        (pts, lines)
    }

    /// Mirrors selected points about a mirror line, creating new points.
    ///
    /// Returns the newly created mirrored point IDs in the same order
    /// as the input `point_ids`.
    pub fn mirror_elements(&mut self, point_ids: &[PointId], mirror_line: LineId) -> Vec<PointId> {
        let (s, e) = (
            self.lines[mirror_line.0].start,
            self.lines[mirror_line.0].end,
        );
        let sx = self.points[s.0].position.x;
        let sy = self.points[s.0].position.y;
        let ex = self.points[e.0].position.x;
        let ey = self.points[e.0].position.y;

        let dx = ex - sx;
        let dy = ey - sy;
        let len_sq = dx * dx + dy * dy;

        let mut new_points = Vec::new();
        for &pid in point_ids {
            let px = self.points[pid.0].position.x;
            let py = self.points[pid.0].position.y;
            let dpx = px - sx;
            let dpy = py - sy;
            let t = (dpx * dx + dpy * dy) / len_sq;
            let proj_x = sx + t * dx;
            let proj_y = sy + t * dy;
            let mx = 2.0 * proj_x - px;
            let my = 2.0 * proj_y - py;
            new_points.push(self.add_point(mx, my));
        }
        new_points
    }

    /// Offsets a closed polyline by distance (positive = outward, negative = inward).
    ///
    /// Computes per-vertex bisector offsets from the ordered chain of line segments.
    /// Returns the newly created offset point IDs.
    pub fn offset_elements(&mut self, line_ids: &[LineId], distance: f64) -> Vec<PointId> {
        if line_ids.is_empty() {
            return Vec::new();
        }

        // Collect ordered points from lines
        let mut ordered_pts: Vec<PointId> = Vec::new();
        for &lid in line_ids {
            ordered_pts.push(self.lines[lid.0].start);
        }

        let n = ordered_pts.len();
        let mut new_points = Vec::new();

        for i in 0..n {
            let prev = (i + n - 1) % n;
            let next = (i + 1) % n;

            let pi = &self.points[ordered_pts[i].0].position;
            let pp = &self.points[ordered_pts[prev].0].position;
            let pn = &self.points[ordered_pts[next].0].position;

            // Edge vectors
            let e1x = pi.x - pp.x;
            let e1y = pi.y - pp.y;
            let e2x = pn.x - pi.x;
            let e2y = pn.y - pi.y;

            // Outward normals (left-hand normal for CCW polygon)
            let len1 = (e1x * e1x + e1y * e1y).sqrt();
            let len2 = (e2x * e2x + e2y * e2y).sqrt();

            if len1 < 1e-14 || len2 < 1e-14 {
                new_points.push(self.add_point(pi.x, pi.y));
                continue;
            }

            let n1x = -e1y / len1;
            let n1y = e1x / len1;
            let n2x = -e2y / len2;
            let n2y = e2x / len2;

            // Bisector
            let bx = n1x + n2x;
            let by = n1y + n2y;
            let b_len = (bx * bx + by * by).sqrt();

            if b_len < 1e-14 {
                let ox = pi.x + distance * n1x;
                let oy = pi.y + distance * n1y;
                new_points.push(self.add_point(ox, oy));
            } else {
                let cos_half = (n1x * bx + n1y * by) / b_len;
                let scale = if cos_half.abs() < 1e-14 {
                    distance
                } else {
                    distance / cos_half
                };
                let ox = pi.x + scale * bx / b_len;
                let oy = pi.y + scale * by / b_len;
                new_points.push(self.add_point(ox, oy));
            }
        }

        new_points
    }

    /// Scales selected points about a center, creating new points.
    pub fn scale_elements(
        &mut self,
        point_ids: &[PointId],
        center_x: f64,
        center_y: f64,
        factor: f64,
    ) -> Vec<PointId> {
        let mut new_points = Vec::new();
        for &pid in point_ids {
            let px = self.points[pid.0].position.x;
            let py = self.points[pid.0].position.y;
            let nx = center_x + (px - center_x) * factor;
            let ny = center_y + (py - center_y) * factor;
            new_points.push(self.add_point(nx, ny));
        }
        new_points
    }

    /// Rotates selected points about a center by angle (radians), creating new points.
    pub fn rotate_elements(
        &mut self,
        point_ids: &[PointId],
        center_x: f64,
        center_y: f64,
        angle: f64,
    ) -> Vec<PointId> {
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        let mut new_points = Vec::new();
        for &pid in point_ids {
            let px = self.points[pid.0].position.x;
            let py = self.points[pid.0].position.y;
            let dx = px - center_x;
            let dy = py - center_y;
            let nx = center_x + dx * cos_a - dy * sin_a;
            let ny = center_y + dx * sin_a + dy * cos_a;
            new_points.push(self.add_point(nx, ny));
        }
        new_points
    }

    /// Creates a slot (oblong) shape: two semicircles connected by two lines.
    ///
    /// Returns `(points, lines, arcs)` — the points around the slot perimeter,
    /// the two straight connecting lines, and the two semicircular arcs.
    pub fn add_slot(
        &mut self,
        center1_x: f64,
        center1_y: f64,
        center2_x: f64,
        center2_y: f64,
        radius: f64,
    ) -> (Vec<PointId>, Vec<LineId>, Vec<ArcId>) {
        let dx = center2_x - center1_x;
        let dy = center2_y - center1_y;
        let len = (dx * dx + dy * dy).sqrt();
        let (nx, ny) = if len < 1e-14 {
            (0.0, 1.0)
        } else {
            (-dy / len, dx / len)
        };

        // 4 corner points
        let p0 = self.add_point(center1_x + nx * radius, center1_y + ny * radius);
        let p1 = self.add_point(center2_x + nx * radius, center2_y + ny * radius);
        let p2 = self.add_point(center2_x - nx * radius, center2_y - ny * radius);
        let p3 = self.add_point(center1_x - nx * radius, center1_y - ny * radius);

        // Center points for arcs
        let c1 = self.add_point(center1_x, center1_y);
        let c2 = self.add_point(center2_x, center2_y);

        // Two straight lines
        let l0 = self.add_line(p0, p1);
        let l1 = self.add_line(p2, p3);

        // Semicircle arcs
        let angle_base = ny.atan2(nx);
        let arc0 = self.add_arc(
            c2,
            p1,
            p2,
            radius,
            angle_base,
            angle_base + std::f64::consts::PI,
        );
        let arc1 = self.add_arc(
            c1,
            p3,
            p0,
            radius,
            angle_base + std::f64::consts::PI,
            angle_base + std::f64::consts::TAU,
        );

        (
            vec![p0, p1, p2, p3],
            vec![l0, l1],
            vec![arc0, arc1],
        )
    }

    /// Adds an arc defined by 3 points (start, mid, end).
    pub fn add_arc_3pt(&mut self, start: PointId, mid: PointId, end: PointId) -> ArcId {
        let sx = self.points[start.0].position.x;
        let sy = self.points[start.0].position.y;
        let mx = self.points[mid.0].position.x;
        let my = self.points[mid.0].position.y;
        let ex = self.points[end.0].position.x;
        let ey = self.points[end.0].position.y;

        let ax = sx - ex;
        let ay = sy - ey;
        let bx = mx - ex;
        let by = my - ey;
        let d = 2.0 * (ax * by - ay * bx);
        let (cx, cy, radius) = if d.abs() < 1e-14 {
            let cx = (sx + ex) / 2.0;
            let cy = (sy + ey) / 2.0;
            let r = ((sx - cx) * (sx - cx) + (sy - cy) * (sy - cy)).sqrt();
            (cx, cy, r)
        } else {
            let ux = (by * (ax * ax + ay * ay) - ay * (bx * bx + by * by)) / d;
            let uy = (ax * (bx * bx + by * by) - bx * (ax * ax + ay * ay)) / d;
            let cx = ex + ux;
            let cy = ey + uy;
            let r = (ux * ux + uy * uy).sqrt();
            (cx, cy, r)
        };

        let center_pt = self.add_point(cx, cy);
        let start_angle = (sy - cy).atan2(sx - cx);
        let end_angle = (ey - cy).atan2(ex - cx);

        self.add_arc(center_pt, start, end, radius, start_angle, end_angle)
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
    fn test_polyline() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(1.0, 0.0);
        let p2 = sketch.add_point(1.0, 1.0);
        let p3 = sketch.add_point(0.0, 1.0);
        let lines = sketch.add_polyline(&[p0, p1, p2, p3]);
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_regular_polygon() {
        let mut sketch = Sketch::new();
        let (pts, lines) = sketch.add_regular_polygon(0.0, 0.0, 5.0, 6);
        assert_eq!(pts.len(), 6);
        assert_eq!(lines.len(), 6);
    }

    #[test]
    fn test_arc_3pt() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(1.0, 0.0);
        let p1 = sketch.add_point(0.0, 1.0);
        let p2 = sketch.add_point(-1.0, 0.0);
        let _arc = sketch.add_arc_3pt(p0, p1, p2);
        assert_eq!(sketch.arcs.len(), 1);
        let center_id = sketch.arcs[0].center;
        let cx = sketch.points[center_id.0].position.x;
        let cy = sketch.points[center_id.0].position.y;
        assert!(cx.abs() < 0.1, "center x = {cx}");
        assert!(cy.abs() < 0.1, "center y = {cy}");
    }

    #[test]
    fn test_ellipse() {
        let mut sketch = Sketch::new();
        let c = sketch.add_point(0.0, 0.0);
        let major = sketch.add_point(5.0, 0.0);
        let _e = sketch.add_ellipse(c, major, 3.0);
        assert_eq!(sketch.ellipses.len(), 1);
    }

    #[test]
    fn test_bspline() {
        let mut sketch = Sketch::new();
        let pts: Vec<_> = (0..5)
            .map(|i| sketch.add_point(i as f64, (i as f64).sin()))
            .collect();
        let _bs = sketch.add_bspline(pts, 3, false);
        assert_eq!(sketch.bsplines.len(), 1);
    }

    #[test]
    fn test_diameter_constraint() {
        let mut sketch = Sketch::new();
        let c = sketch.add_point(0.0, 0.0);
        let p = sketch.add_point(3.0, 0.0);
        sketch.add_constraint(Constraint::Fixed(c, 0.0, 0.0));
        sketch.add_constraint(Constraint::Diameter(p, c, 10.0));
        let result = solve(&mut sketch, 200, 1e-10);
        assert!(result.converged);
        let dx = sketch.points[p.0].position.x;
        let dy = sketch.points[p.0].position.y;
        let dist = (dx * dx + dy * dy).sqrt();
        assert!((dist - 5.0).abs() < 1e-6, "dist = {dist}");
    }

    #[test]
    fn test_horizontal_distance_constraint() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(5.0, 3.0);
        sketch.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
        sketch.add_constraint(Constraint::HorizontalDistance(p1, p0, 7.0));
        let result = solve(&mut sketch, 200, 1e-10);
        assert!(result.converged);
        let dx = sketch.points[p1.0].position.x - sketch.points[p0.0].position.x;
        assert!((dx - 7.0).abs() < 1e-6, "hdist = {dx}");
    }

    #[test]
    fn test_vertical_distance_constraint() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(3.0, 5.0);
        sketch.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
        sketch.add_constraint(Constraint::VerticalDistance(p1, p0, 4.0));
        let result = solve(&mut sketch, 200, 1e-10);
        assert!(result.converged);
        let dy = sketch.points[p1.0].position.y - sketch.points[p0.0].position.y;
        assert!((dy - 4.0).abs() < 1e-6, "vdist = {dy}");
    }

    #[test]
    fn test_block_constraint() {
        let mut sketch = Sketch::new();
        let p = sketch.add_point(3.0, 4.0);
        sketch.add_constraint(Constraint::Block(p, 3.0, 4.0));
        let result = solve(&mut sketch, 200, 1e-10);
        assert!(result.converged);
        assert!((sketch.points[p.0].position.x - 3.0).abs() < 1e-6);
        assert!((sketch.points[p.0].position.y - 4.0).abs() < 1e-6);
    }

    #[test]
    fn test_mirror_elements() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(1.0, 1.0);
        let p1 = sketch.add_point(2.0, 3.0);
        // Mirror axis: vertical line x=0
        let ax0 = sketch.add_point(0.0, 0.0);
        let ax1 = sketch.add_point(0.0, 5.0);
        let axis = sketch.add_line(ax0, ax1);

        let mirrored = sketch.mirror_elements(&[p0, p1], axis);
        assert_eq!(mirrored.len(), 2);
        let m0 = &sketch.points[mirrored[0].0].position;
        let m1 = &sketch.points[mirrored[1].0].position;
        assert!((m0.x - (-1.0)).abs() < 1e-10, "m0.x = {}", m0.x);
        assert!((m0.y - 1.0).abs() < 1e-10, "m0.y = {}", m0.y);
        assert!((m1.x - (-2.0)).abs() < 1e-10, "m1.x = {}", m1.x);
        assert!((m1.y - 3.0).abs() < 1e-10, "m1.y = {}", m1.y);
    }

    #[test]
    fn test_rotate_elements() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(1.0, 0.0);
        let angle = std::f64::consts::FRAC_PI_2; // 90 degrees

        let rotated = sketch.rotate_elements(&[p0], 0.0, 0.0, angle);
        assert_eq!(rotated.len(), 1);
        let r = &sketch.points[rotated[0].0].position;
        assert!(r.x.abs() < 1e-10, "r.x = {}", r.x);
        assert!((r.y - 1.0).abs() < 1e-10, "r.y = {}", r.y);
    }

    #[test]
    fn test_scale_elements() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(2.0, 3.0);

        let scaled = sketch.scale_elements(&[p0], 0.0, 0.0, 2.0);
        assert_eq!(scaled.len(), 1);
        let s = &sketch.points[scaled[0].0].position;
        assert!((s.x - 4.0).abs() < 1e-10, "s.x = {}", s.x);
        assert!((s.y - 6.0).abs() < 1e-10, "s.y = {}", s.y);
    }

    #[test]
    fn test_offset_elements() {
        let mut sketch = Sketch::new();
        // Create a unit square (CCW)
        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(1.0, 0.0);
        let p2 = sketch.add_point(1.0, 1.0);
        let p3 = sketch.add_point(0.0, 1.0);
        let l0 = sketch.add_line(p0, p1);
        let l1 = sketch.add_line(p1, p2);
        let l2 = sketch.add_line(p2, p3);
        let l3 = sketch.add_line(p3, p0);

        let offset_pts = sketch.offset_elements(&[l0, l1, l2, l3], 1.0);
        assert_eq!(offset_pts.len(), 4);
        // Each offset point should be further from center than original
        for pid in &offset_pts {
            let op = &sketch.points[pid.0].position;
            let dist = ((op.x - 0.5) * (op.x - 0.5) + (op.y - 0.5) * (op.y - 0.5)).sqrt();
            assert!(dist > 0.7, "offset point too close to center: ({}, {})", op.x, op.y);
        }
    }

    #[test]
    fn test_add_slot() {
        let mut sketch = Sketch::new();
        let (pts, lines, arcs) = sketch.add_slot(0.0, 0.0, 4.0, 0.0, 1.0);
        assert_eq!(pts.len(), 4);
        assert_eq!(lines.len(), 2);
        assert_eq!(arcs.len(), 2);
        // Verify the slot width is 2*radius = 2.0
        let p0 = &sketch.points[pts[0].0].position;
        let p3 = &sketch.points[pts[3].0].position;
        let width = ((p0.x - p3.x).powi(2) + (p0.y - p3.y).powi(2)).sqrt();
        assert!((width - 2.0).abs() < 1e-10, "slot width = {width}");
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
