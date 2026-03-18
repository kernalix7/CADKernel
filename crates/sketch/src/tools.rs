//! Sketch editing tools: fillet, chamfer, trim, split, extend.

use crate::entity::{ArcId, LineId, PointId};
use crate::Sketch;

/// Result of a sketch fillet operation.
#[derive(Debug)]
pub struct FilletResult {
    /// The new arc created at the corner.
    pub arc: ArcId,
    /// The new point IDs at the tangent locations.
    pub tangent_points: (PointId, PointId),
}

/// Result of a sketch chamfer operation.
#[derive(Debug)]
pub struct SketchChamferResult {
    /// The new chamfer line.
    pub line: LineId,
    /// The new corner points.
    pub corner_points: (PointId, PointId),
}

/// Result of a trim operation.
#[derive(Debug)]
pub struct TrimResult {
    /// Whether a trim was performed.
    pub trimmed: bool,
}

/// Result of a split operation.
#[derive(Debug)]
pub struct SplitResult {
    /// The midpoint created at the split location.
    pub mid_point: PointId,
    /// The two resulting line segments.
    pub lines: (LineId, LineId),
}

/// Creates a fillet (rounded corner) between two lines sharing a common vertex.
///
/// Shortens both lines and inserts a tangent arc of the given radius.
pub fn fillet_sketch_corner(
    sketch: &mut Sketch,
    line_a: LineId,
    line_b: LineId,
    radius: f64,
) -> Option<FilletResult> {
    // Find the shared vertex between the two lines
    let la = sketch.lines[line_a.0];
    let lb = sketch.lines[line_b.0];

    let (shared, other_a, other_b) = find_shared_vertex(la.start, la.end, lb.start, lb.end)?;

    let ps = sketch.points[shared.0].position;
    let pa = sketch.points[other_a.0].position;
    let pb = sketch.points[other_b.0].position;

    // Direction vectors from shared vertex
    let da_x = pa.x - ps.x;
    let da_y = pa.y - ps.y;
    let db_x = pb.x - ps.x;
    let db_y = pb.y - ps.y;

    let la_len = (da_x * da_x + da_y * da_y).sqrt();
    let lb_len = (db_x * db_x + db_y * db_y).sqrt();

    if la_len < 1e-14 || lb_len < 1e-14 {
        return None;
    }

    // Unit direction vectors
    let ua_x = da_x / la_len;
    let ua_y = da_y / la_len;
    let ub_x = db_x / lb_len;
    let ub_y = db_y / lb_len;

    // Half angle between lines
    let cos_half = ((1.0 + ua_x * ub_x + ua_y * ub_y) / 2.0).sqrt();
    if cos_half < 1e-10 {
        return None; // Lines nearly parallel
    }
    let sin_half = (1.0 - cos_half * cos_half).sqrt();
    let tan_half = sin_half / cos_half;

    // Distance along each line to tangent point
    let t = radius / tan_half;
    if t > la_len || t > lb_len {
        return None; // Radius too large
    }

    // Tangent points
    let ta_x = ps.x + ua_x * t;
    let ta_y = ps.y + ua_y * t;
    let tb_x = ps.x + ub_x * t;
    let tb_y = ps.y + ub_y * t;

    let tp_a = sketch.add_point(ta_x, ta_y);
    let tp_b = sketch.add_point(tb_x, tb_y);

    // Arc center: along bisector at distance radius/sin(half_angle)
    let bx = ua_x + ub_x;
    let by = ua_y + ub_y;
    let b_len = (bx * bx + by * by).sqrt();
    if b_len < 1e-14 {
        return None;
    }
    let center_dist = radius / sin_half;
    let cx = ps.x + bx / b_len * center_dist;
    let cy = ps.y + by / b_len * center_dist;
    let center = sketch.add_point(cx, cy);

    // Compute arc angles
    let start_angle = (ta_y - cy).atan2(ta_x - cx);
    let end_angle = (tb_y - cy).atan2(tb_x - cx);

    let arc = sketch.add_arc(center, tp_a, tp_b, radius, start_angle, end_angle);

    // Update original lines to end at tangent points
    if sketch.lines[line_a.0].start == shared {
        sketch.lines[line_a.0].start = tp_a;
    } else {
        sketch.lines[line_a.0].end = tp_a;
    }
    if sketch.lines[line_b.0].start == shared {
        sketch.lines[line_b.0].start = tp_b;
    } else {
        sketch.lines[line_b.0].end = tp_b;
    }

    Some(FilletResult {
        arc,
        tangent_points: (tp_a, tp_b),
    })
}

/// Creates a chamfer (straight bevel) between two lines sharing a common vertex.
pub fn chamfer_sketch_corner(
    sketch: &mut Sketch,
    line_a: LineId,
    line_b: LineId,
    distance: f64,
) -> Option<SketchChamferResult> {
    let la = sketch.lines[line_a.0];
    let lb = sketch.lines[line_b.0];

    let (shared, other_a, other_b) = find_shared_vertex(la.start, la.end, lb.start, lb.end)?;

    let ps = sketch.points[shared.0].position;
    let pa = sketch.points[other_a.0].position;
    let pb = sketch.points[other_b.0].position;

    // Direction vectors from shared vertex
    let da_x = pa.x - ps.x;
    let da_y = pa.y - ps.y;
    let db_x = pb.x - ps.x;
    let db_y = pb.y - ps.y;

    let la_len = (da_x * da_x + da_y * da_y).sqrt();
    let lb_len = (db_x * db_x + db_y * db_y).sqrt();

    if distance > la_len || distance > lb_len {
        return None;
    }

    // Points at distance along each line
    let ca_x = ps.x + da_x / la_len * distance;
    let ca_y = ps.y + da_y / la_len * distance;
    let cb_x = ps.x + db_x / lb_len * distance;
    let cb_y = ps.y + db_y / lb_len * distance;

    let cp_a = sketch.add_point(ca_x, ca_y);
    let cp_b = sketch.add_point(cb_x, cb_y);

    // Create chamfer line
    let chamfer_line = sketch.add_line(cp_a, cp_b);

    // Update original lines
    if sketch.lines[line_a.0].start == shared {
        sketch.lines[line_a.0].start = cp_a;
    } else {
        sketch.lines[line_a.0].end = cp_a;
    }
    if sketch.lines[line_b.0].start == shared {
        sketch.lines[line_b.0].start = cp_b;
    } else {
        sketch.lines[line_b.0].end = cp_b;
    }

    Some(SketchChamferResult {
        line: chamfer_line,
        corner_points: (cp_a, cp_b),
    })
}

/// Trims a line segment at its intersection with another line.
///
/// Removes the portion of `line_to_trim` that extends beyond the intersection
/// with `cutting_line`, on the side of `keep_point`.
pub fn trim_edge(
    sketch: &mut Sketch,
    line_to_trim: LineId,
    cutting_line: LineId,
    keep_point: PointId,
) -> TrimResult {
    let lt = sketch.lines[line_to_trim.0];
    let lc = sketch.lines[cutting_line.0];

    let p1 = sketch.points[lt.start.0].position;
    let p2 = sketch.points[lt.end.0].position;
    let p3 = sketch.points[lc.start.0].position;
    let p4 = sketch.points[lc.end.0].position;

    // Line-line intersection via parametric form
    let d1x = p2.x - p1.x;
    let d1y = p2.y - p1.y;
    let d2x = p4.x - p3.x;
    let d2y = p4.y - p3.y;

    let denom = d1x * d2y - d1y * d2x;
    if denom.abs() < 1e-14 {
        return TrimResult { trimmed: false }; // Parallel lines
    }

    let t = ((p3.x - p1.x) * d2y - (p3.y - p1.y) * d2x) / denom;

    // Intersection must be on the line segment (within [0,1])
    if !(-1e-10..=1.0 + 1e-10).contains(&t) {
        return TrimResult { trimmed: false };
    }

    let ix = p1.x + t * d1x;
    let iy = p1.y + t * d1y;
    let int_point = sketch.add_point(ix, iy);

    // Keep the side with keep_point
    if lt.start == keep_point {
        sketch.lines[line_to_trim.0].end = int_point;
    } else {
        sketch.lines[line_to_trim.0].start = int_point;
    }

    TrimResult { trimmed: true }
}

/// Splits a line at a given parameter t ∈ [0,1].
///
/// The original line is shortened to [start, mid], and a new line [mid, end] is created.
pub fn split_edge(sketch: &mut Sketch, line: LineId, t: f64) -> SplitResult {
    let l = sketch.lines[line.0];
    let p1 = sketch.points[l.start.0].position;
    let p2 = sketch.points[l.end.0].position;

    let mx = p1.x + t * (p2.x - p1.x);
    let my = p1.y + t * (p2.y - p1.y);
    let mid = sketch.add_point(mx, my);

    let original_end = l.end;
    sketch.lines[line.0].end = mid;
    let new_line = sketch.add_line(mid, original_end);

    SplitResult {
        mid_point: mid,
        lines: (line, new_line),
    }
}

/// Extends a line to a target point.
///
/// Projects `target` onto the line's direction and extends the nearest endpoint.
pub fn extend_edge(sketch: &mut Sketch, line: LineId, target_x: f64, target_y: f64) {
    let l = sketch.lines[line.0];
    let p1 = sketch.points[l.start.0].position;
    let p2 = sketch.points[l.end.0].position;

    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    let len_sq = dx * dx + dy * dy;

    if len_sq < 1e-28 {
        return;
    }

    // Project target onto line direction
    let t = ((target_x - p1.x) * dx + (target_y - p1.y) * dy) / len_sq;
    let proj_x = p1.x + t * dx;
    let proj_y = p1.y + t * dy;

    // Extend the nearest endpoint
    if t < 0.0 {
        sketch.points[l.start.0].position.x = proj_x;
        sketch.points[l.start.0].position.y = proj_y;
    } else if t > 1.0 {
        sketch.points[l.end.0].position.x = proj_x;
        sketch.points[l.end.0].position.y = proj_y;
    }
}

/// Finds the shared vertex between two line segments.
fn find_shared_vertex(
    a_start: PointId,
    a_end: PointId,
    b_start: PointId,
    b_end: PointId,
) -> Option<(PointId, PointId, PointId)> {
    if a_start == b_start {
        Some((a_start, a_end, b_end))
    } else if a_start == b_end {
        Some((a_start, a_end, b_start))
    } else if a_end == b_start {
        Some((a_end, a_start, b_end))
    } else if a_end == b_end {
        Some((a_end, a_start, b_start))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Sketch;

    #[test]
    fn test_fillet_right_angle() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(5.0, 0.0);
        let p1 = sketch.add_point(0.0, 0.0); // shared corner
        let p2 = sketch.add_point(0.0, 5.0);
        let l0 = sketch.add_line(p0, p1);
        let l1 = sketch.add_line(p1, p2);

        let result = fillet_sketch_corner(&mut sketch, l0, l1, 1.0);
        assert!(result.is_some(), "fillet should succeed");
        let r = result.unwrap();
        assert!(sketch.arcs[r.arc.0].radius > 0.0);
    }

    #[test]
    fn test_chamfer() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(5.0, 0.0);
        let p1 = sketch.add_point(0.0, 0.0);
        let p2 = sketch.add_point(0.0, 5.0);
        let l0 = sketch.add_line(p0, p1);
        let l1 = sketch.add_line(p1, p2);

        let result = chamfer_sketch_corner(&mut sketch, l0, l1, 1.0);
        assert!(result.is_some());
    }

    #[test]
    fn test_trim_edge() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(10.0, 0.0);
        let p2 = sketch.add_point(5.0, -5.0);
        let p3 = sketch.add_point(5.0, 5.0);
        let l0 = sketch.add_line(p0, p1);
        let l1 = sketch.add_line(p2, p3);

        let result = trim_edge(&mut sketch, l0, l1, p0);
        assert!(result.trimmed);
        // Line should end near x=5
        let end = sketch.lines[l0.0].end;
        let ex = sketch.points[end.0].position.x;
        assert!((ex - 5.0).abs() < 0.1, "trim end x = {ex}");
    }

    #[test]
    fn test_split_edge() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(10.0, 0.0);
        let l0 = sketch.add_line(p0, p1);

        let result = split_edge(&mut sketch, l0, 0.5);
        let mx = sketch.points[result.mid_point.0].position.x;
        assert!((mx - 5.0).abs() < 1e-10);
        assert_eq!(sketch.lines.len(), 2);
    }

    #[test]
    fn test_extend_edge() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(5.0, 0.0);
        let l0 = sketch.add_line(p0, p1);

        extend_edge(&mut sketch, l0, 10.0, 0.0);
        let ex = sketch.points[p1.0].position.x;
        assert!((ex - 10.0).abs() < 1e-10, "extended end = {ex}");
    }
}
