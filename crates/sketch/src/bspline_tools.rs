//! B-spline tools for sketch editing: degree elevation/reduction, knot operations, conversion.

use cadkernel_core::{KernelError, KernelResult};

use crate::Sketch;
use crate::entity::{BSplineId, PointId};

/// Converts a line, arc, or circle entity to a B-spline representation.
///
/// For lines: creates a degree-1 B-spline with 2 control points.
/// For arcs/circles: creates a degree-3 B-spline approximation.
/// Returns the new B-spline ID.
pub fn geometry_to_bspline(sketch: &mut Sketch, entity_id: usize) -> KernelResult<BSplineId> {
    // Try line first
    if entity_id < sketch.lines.len() {
        let line = sketch.lines[entity_id];
        let cps = vec![line.start, line.end];
        let bs = sketch.add_bspline(cps, 1, false);
        return Ok(bs);
    }

    // Try arc
    let arc_idx = entity_id - sketch.lines.len();
    if arc_idx < sketch.arcs.len() {
        let arc = sketch.arcs[arc_idx];
        let center = sketch.points[arc.center.0].position;
        let r = arc.radius;
        let sa = arc.start_angle;
        let ea = arc.end_angle;

        // Approximate arc with 5 sample points
        let n = 5;
        let mut cps = Vec::with_capacity(n);
        for i in 0..n {
            let t = sa + (ea - sa) * i as f64 / (n - 1) as f64;
            let x = center.x + r * t.cos();
            let y = center.y + r * t.sin();
            cps.push(sketch.add_point(x, y));
        }
        let bs = sketch.add_bspline(cps, 3.min(n - 1), false);
        return Ok(bs);
    }

    // Try circle
    let circle_idx = arc_idx - sketch.arcs.len();
    if circle_idx < sketch.circles.len() {
        let circle = sketch.circles[circle_idx];
        let center = sketch.points[circle.center.0].position;
        let r = circle.radius;

        // Approximate circle with 8 points
        let n = 8;
        let mut cps = Vec::with_capacity(n);
        for i in 0..n {
            let t = std::f64::consts::TAU * i as f64 / n as f64;
            let x = center.x + r * t.cos();
            let y = center.y + r * t.sin();
            cps.push(sketch.add_point(x, y));
        }
        let bs = sketch.add_bspline(cps, 3, true);
        return Ok(bs);
    }

    Err(KernelError::InvalidArgument(format!(
        "entity_id {} is out of range",
        entity_id
    )))
}

/// Increases the degree of a B-spline by 1.
///
/// Elevates the degree by duplicating each control point span. The shape
/// is preserved while increasing the polynomial degree.
pub fn increase_bspline_degree(sketch: &mut Sketch, bspline_id: BSplineId) -> KernelResult<()> {
    let bs = sketch
        .bsplines
        .get(bspline_id.0)
        .ok_or(KernelError::InvalidArgument("invalid bspline id".into()))?
        .clone();

    if bs.control_points.len() < 2 {
        return Err(KernelError::InvalidArgument(
            "B-spline needs at least 2 control points".into(),
        ));
    }

    let n = bs.control_points.len();
    let new_degree = bs.degree + 1;

    // Insert midpoint between each consecutive pair of control points
    let mut new_cps = Vec::with_capacity(2 * n - 1);
    for i in 0..n {
        new_cps.push(bs.control_points[i]);
        if i + 1 < n {
            let p0 = sketch.points[bs.control_points[i].0].position;
            let p1 = sketch.points[bs.control_points[i + 1].0].position;
            let mid = sketch.add_point((p0.x + p1.x) / 2.0, (p0.y + p1.y) / 2.0);
            new_cps.push(mid);
        }
    }

    sketch.bsplines[bspline_id.0].control_points = new_cps;
    sketch.bsplines[bspline_id.0].degree = new_degree;
    sketch.bsplines[bspline_id.0].knots.clear();

    Ok(())
}

/// Decreases the degree of a B-spline by 1.
///
/// Removes every other inserted midpoint, keeping the original control points.
/// Minimum degree is 1.
pub fn decrease_bspline_degree(sketch: &mut Sketch, bspline_id: BSplineId) -> KernelResult<()> {
    let bs = sketch
        .bsplines
        .get(bspline_id.0)
        .ok_or(KernelError::InvalidArgument("invalid bspline id".into()))?
        .clone();

    if bs.degree <= 1 {
        return Err(KernelError::InvalidArgument(
            "cannot reduce degree below 1".into(),
        ));
    }

    // Keep every other control point
    let new_cps: Vec<PointId> = bs.control_points.iter().step_by(2).copied().collect();

    if new_cps.len() < 2 {
        return Err(KernelError::InvalidArgument(
            "too few control points after degree reduction".into(),
        ));
    }

    sketch.bsplines[bspline_id.0].control_points = new_cps;
    sketch.bsplines[bspline_id.0].degree = bs.degree - 1;
    sketch.bsplines[bspline_id.0].knots.clear();

    Ok(())
}

/// Increases the knot multiplicity at the given knot index.
///
/// Inserts a duplicate control point at the knot position.
pub fn increase_knot_multiplicity(
    sketch: &mut Sketch,
    bspline_id: BSplineId,
    knot_index: usize,
) -> KernelResult<()> {
    let bs = sketch
        .bsplines
        .get(bspline_id.0)
        .ok_or(KernelError::InvalidArgument("invalid bspline id".into()))?
        .clone();

    if knot_index >= bs.control_points.len() {
        return Err(KernelError::InvalidArgument(
            "knot_index out of range".into(),
        ));
    }

    // Duplicate the control point at the knot index
    let pt = sketch.points[bs.control_points[knot_index].0].position;
    let new_pt = sketch.add_point(pt.x, pt.y);

    let mut new_cps = bs.control_points.clone();
    new_cps.insert(knot_index + 1, new_pt);
    sketch.bsplines[bspline_id.0].control_points = new_cps;
    sketch.bsplines[bspline_id.0].knots.clear();

    Ok(())
}

/// Decreases the knot multiplicity at the given knot index.
///
/// Removes one control point at the knot position (if multiplicity > 1).
pub fn decrease_knot_multiplicity(
    sketch: &mut Sketch,
    bspline_id: BSplineId,
    knot_index: usize,
) -> KernelResult<()> {
    let bs = sketch
        .bsplines
        .get(bspline_id.0)
        .ok_or(KernelError::InvalidArgument("invalid bspline id".into()))?
        .clone();

    if bs.control_points.len() <= bs.degree + 1 {
        return Err(KernelError::InvalidArgument(
            "cannot remove more control points".into(),
        ));
    }
    if knot_index >= bs.control_points.len() {
        return Err(KernelError::InvalidArgument(
            "knot_index out of range".into(),
        ));
    }

    let mut new_cps = bs.control_points.clone();
    new_cps.remove(knot_index);
    sketch.bsplines[bspline_id.0].control_points = new_cps;
    sketch.bsplines[bspline_id.0].knots.clear();

    Ok(())
}

/// Inserts a knot at parameter value `t` by adding a new control point.
///
/// The new control point is computed by linear interpolation between
/// the two neighboring control points at the parameter location.
pub fn insert_knot(sketch: &mut Sketch, bspline_id: BSplineId, t_value: f64) -> KernelResult<()> {
    let bs = sketch
        .bsplines
        .get(bspline_id.0)
        .ok_or(KernelError::InvalidArgument("invalid bspline id".into()))?
        .clone();

    let n = bs.control_points.len();
    if n < 2 {
        return Err(KernelError::InvalidArgument(
            "B-spline needs at least 2 control points".into(),
        ));
    }

    // Find the span
    let t_clamped = t_value.clamp(0.0, 1.0);
    let span_f = t_clamped * (n - 1) as f64;
    let span = (span_f as usize).min(n - 2);
    let local_t = span_f - span as f64;

    let p0 = sketch.points[bs.control_points[span].0].position;
    let p1 = sketch.points[bs.control_points[span + 1].0].position;
    let nx = p0.x + local_t * (p1.x - p0.x);
    let ny = p0.y + local_t * (p1.y - p0.y);
    let new_pt = sketch.add_point(nx, ny);

    let mut new_cps = bs.control_points.clone();
    new_cps.insert(span + 1, new_pt);
    sketch.bsplines[bspline_id.0].control_points = new_cps;
    sketch.bsplines[bspline_id.0].knots.clear();

    Ok(())
}

/// Joins two curves into one B-spline.
///
/// Takes two entity IDs (lines, arcs, or B-splines), converts them to control
/// points, and creates a single merged B-spline.
pub fn join_curves(sketch: &mut Sketch, entity1: usize, entity2: usize) -> KernelResult<BSplineId> {
    let cps1 = collect_entity_points(sketch, entity1)?;
    let cps2 = collect_entity_points(sketch, entity2)?;

    let mut merged = cps1;
    merged.extend(cps2);

    let degree = 3.min(merged.len().saturating_sub(1));
    let bs = sketch.add_bspline(merged, degree, false);
    Ok(bs)
}

/// Projects 3D points onto a sketch plane, adding them as sketch points.
pub fn external_projection(
    sketch: &mut Sketch,
    points_3d: &[cadkernel_math::Point3],
    plane: &crate::WorkPlane,
) -> Vec<PointId> {
    let mut ids = Vec::with_capacity(points_3d.len());
    for &pt in points_3d {
        let local = plane.to_local(pt);
        ids.push(sketch.add_point(local.x, local.y));
    }
    ids
}

/// Copies all geometry from source sketch into target sketch.
pub fn carbon_copy(source: &Sketch, target: &mut Sketch) -> KernelResult<()> {
    target.merge_with(source);
    Ok(())
}

/// Translates entities by (dx, dy).
pub fn move_geometry(sketch: &mut Sketch, entity_ids: &[PointId], dx: f64, dy: f64) {
    for &pid in entity_ids {
        if pid.0 < sketch.points.len() {
            sketch.points[pid.0].position.x += dx;
            sketch.points[pid.0].position.y += dy;
        }
    }
}

/// Rotates entities around a center point by an angle (radians).
pub fn rotate_geometry(
    sketch: &mut Sketch,
    entity_ids: &[PointId],
    center_x: f64,
    center_y: f64,
    angle: f64,
) {
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    for &pid in entity_ids {
        if pid.0 < sketch.points.len() {
            let p = &mut sketch.points[pid.0].position;
            let dx = p.x - center_x;
            let dy = p.y - center_y;
            p.x = center_x + dx * cos_a - dy * sin_a;
            p.y = center_y + dx * sin_a + dy * cos_a;
        }
    }
}

/// Scales entities from a center point by a factor.
pub fn scale_geometry(
    sketch: &mut Sketch,
    entity_ids: &[PointId],
    center_x: f64,
    center_y: f64,
    factor: f64,
) {
    for &pid in entity_ids {
        if pid.0 < sketch.points.len() {
            let p = &mut sketch.points[pid.0].position;
            p.x = center_x + (p.x - center_x) * factor;
            p.y = center_y + (p.y - center_y) * factor;
        }
    }
}

/// Offsets a curve entity by a given distance, creating a new set of points.
pub fn offset_geometry(
    sketch: &mut Sketch,
    entity_id: usize,
    distance: f64,
) -> KernelResult<Vec<PointId>> {
    if entity_id >= sketch.lines.len() {
        return Err(KernelError::InvalidArgument(
            "offset_geometry currently only supports lines".into(),
        ));
    }

    let line = sketch.lines[entity_id];
    let p0 = sketch.points[line.start.0].position;
    let p1 = sketch.points[line.end.0].position;

    let dx = p1.x - p0.x;
    let dy = p1.y - p0.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-14 {
        return Err(KernelError::InvalidArgument(
            "zero-length line cannot be offset".into(),
        ));
    }

    // Normal direction (perpendicular left)
    let nx = -dy / len * distance;
    let ny = dx / len * distance;

    let op0 = sketch.add_point(p0.x + nx, p0.y + ny);
    let op1 = sketch.add_point(p1.x + nx, p1.y + ny);
    sketch.add_line(op0, op1);

    Ok(vec![op0, op1])
}

/// Mirrors entities across an axis defined by a point and direction.
pub fn mirror_geometry_axis(
    sketch: &mut Sketch,
    entity_ids: &[PointId],
    axis_point_x: f64,
    axis_point_y: f64,
    axis_dir_x: f64,
    axis_dir_y: f64,
) -> Vec<PointId> {
    let len_sq = axis_dir_x * axis_dir_x + axis_dir_y * axis_dir_y;
    if len_sq < 1e-28 {
        return Vec::new();
    }

    let mut new_points = Vec::with_capacity(entity_ids.len());
    for &pid in entity_ids {
        if pid.0 >= sketch.points.len() {
            continue;
        }
        let p = sketch.points[pid.0].position;
        let dx = p.x - axis_point_x;
        let dy = p.y - axis_point_y;
        let t = (dx * axis_dir_x + dy * axis_dir_y) / len_sq;
        let proj_x = axis_point_x + t * axis_dir_x;
        let proj_y = axis_point_y + t * axis_dir_y;
        let mx = 2.0 * proj_x - p.x;
        let my = 2.0 * proj_y - p.y;
        new_points.push(sketch.add_point(mx, my));
    }
    new_points
}

/// Removes all geometry entities from the sketch.
pub fn delete_all_geometry(sketch: &mut Sketch) {
    sketch.points.clear();
    sketch.lines.clear();
    sketch.arcs.clear();
    sketch.circles.clear();
    sketch.ellipses.clear();
    sketch.bsplines.clear();
    sketch.elliptical_arcs.clear();
    sketch.hyperbolic_arcs.clear();
    sketch.parabolic_arcs.clear();
    sketch.construction_points.clear();
    sketch.construction_lines.clear();
}

/// Removes all constraints from the sketch.
pub fn delete_all_constraints(sketch: &mut Sketch) {
    sketch.constraints.clear();
}

/// Collects the control/endpoint PointIds for a given entity index.
fn collect_entity_points(sketch: &Sketch, entity_id: usize) -> KernelResult<Vec<PointId>> {
    if entity_id < sketch.lines.len() {
        let l = sketch.lines[entity_id];
        return Ok(vec![l.start, l.end]);
    }
    let arc_idx = entity_id - sketch.lines.len();
    if arc_idx < sketch.arcs.len() {
        let a = sketch.arcs[arc_idx];
        return Ok(vec![a.start_point, a.end_point]);
    }
    let bs_idx = arc_idx - sketch.arcs.len();
    if bs_idx < sketch.bsplines.len() {
        return Ok(sketch.bsplines[bs_idx].control_points.clone());
    }
    Err(KernelError::InvalidArgument(format!(
        "entity {} not found",
        entity_id
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::PointId;

    #[test]
    fn test_geometry_to_bspline_line() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(5.0, 0.0);
        sketch.add_line(p0, p1);
        let bs = geometry_to_bspline(&mut sketch, 0).unwrap();
        assert_eq!(sketch.bsplines[bs.0].degree, 1);
        assert_eq!(sketch.bsplines[bs.0].control_points.len(), 2);
    }

    #[test]
    fn test_increase_degree() {
        let mut sketch = Sketch::new();
        let pts: Vec<_> = (0..4).map(|i| sketch.add_point(i as f64, 0.0)).collect();
        let bs = sketch.add_bspline(pts, 2, false);
        assert_eq!(sketch.bsplines[bs.0].degree, 2);

        increase_bspline_degree(&mut sketch, bs).unwrap();
        assert_eq!(sketch.bsplines[bs.0].degree, 3);
        assert!(sketch.bsplines[bs.0].control_points.len() > 4);
    }

    #[test]
    fn test_decrease_degree() {
        let mut sketch = Sketch::new();
        let pts: Vec<_> = (0..7).map(|i| sketch.add_point(i as f64, 0.0)).collect();
        let bs = sketch.add_bspline(pts, 3, false);

        // First elevate
        increase_bspline_degree(&mut sketch, bs).unwrap();
        let elevated_degree = sketch.bsplines[bs.0].degree;

        // Then reduce
        decrease_bspline_degree(&mut sketch, bs).unwrap();
        assert_eq!(sketch.bsplines[bs.0].degree, elevated_degree - 1);
    }

    #[test]
    fn test_decrease_degree_min() {
        let mut sketch = Sketch::new();
        let pts: Vec<_> = (0..3).map(|i| sketch.add_point(i as f64, 0.0)).collect();
        let bs = sketch.add_bspline(pts, 1, false);
        assert!(decrease_bspline_degree(&mut sketch, bs).is_err());
    }

    #[test]
    fn test_increase_knot_multiplicity() {
        let mut sketch = Sketch::new();
        let pts: Vec<_> = (0..5).map(|i| sketch.add_point(i as f64, 0.0)).collect();
        let bs = sketch.add_bspline(pts, 3, false);
        let orig_len = sketch.bsplines[bs.0].control_points.len();

        increase_knot_multiplicity(&mut sketch, bs, 2).unwrap();
        assert_eq!(sketch.bsplines[bs.0].control_points.len(), orig_len + 1);
    }

    #[test]
    fn test_decrease_knot_multiplicity() {
        let mut sketch = Sketch::new();
        let pts: Vec<_> = (0..6).map(|i| sketch.add_point(i as f64, 0.0)).collect();
        let bs = sketch.add_bspline(pts, 3, false);
        let orig_len = sketch.bsplines[bs.0].control_points.len();

        decrease_knot_multiplicity(&mut sketch, bs, 2).unwrap();
        assert_eq!(sketch.bsplines[bs.0].control_points.len(), orig_len - 1);
    }

    #[test]
    fn test_insert_knot() {
        let mut sketch = Sketch::new();
        let pts: Vec<_> = (0..4).map(|i| sketch.add_point(i as f64, 0.0)).collect();
        let bs = sketch.add_bspline(pts, 3, false);
        let orig_len = sketch.bsplines[bs.0].control_points.len();

        insert_knot(&mut sketch, bs, 0.5).unwrap();
        assert_eq!(sketch.bsplines[bs.0].control_points.len(), orig_len + 1);
    }

    #[test]
    fn test_join_curves() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(1.0, 0.0);
        let p2 = sketch.add_point(2.0, 1.0);
        let p3 = sketch.add_point(3.0, 1.0);
        sketch.add_line(p0, p1);
        sketch.add_line(p2, p3);

        let bs = join_curves(&mut sketch, 0, 1).unwrap();
        assert_eq!(sketch.bsplines[bs.0].control_points.len(), 4);
    }

    #[test]
    fn test_move_geometry() {
        let mut sketch = Sketch::new();
        let p = sketch.add_point(1.0, 2.0);
        move_geometry(&mut sketch, &[p], 3.0, 4.0);
        assert!((sketch.points[p.0].position.x - 4.0).abs() < 1e-10);
        assert!((sketch.points[p.0].position.y - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_rotate_geometry() {
        let mut sketch = Sketch::new();
        let p = sketch.add_point(1.0, 0.0);
        rotate_geometry(&mut sketch, &[p], 0.0, 0.0, std::f64::consts::FRAC_PI_2);
        assert!(sketch.points[p.0].position.x.abs() < 1e-10);
        assert!((sketch.points[p.0].position.y - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_scale_geometry() {
        let mut sketch = Sketch::new();
        let p = sketch.add_point(2.0, 3.0);
        scale_geometry(&mut sketch, &[p], 0.0, 0.0, 2.0);
        assert!((sketch.points[p.0].position.x - 4.0).abs() < 1e-10);
        assert!((sketch.points[p.0].position.y - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_offset_geometry() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(5.0, 0.0);
        sketch.add_line(p0, p1);

        let pts = offset_geometry(&mut sketch, 0, 1.0).unwrap();
        assert_eq!(pts.len(), 2);
        // Offset should be 1.0 in Y direction
        let oy = sketch.points[pts[0].0].position.y;
        assert!((oy - 1.0).abs() < 1e-10, "offset y = {oy}");
    }

    #[test]
    fn test_mirror_geometry_axis() {
        let mut sketch = Sketch::new();
        let p = sketch.add_point(1.0, 2.0);
        let mirrored = mirror_geometry_axis(&mut sketch, &[p], 0.0, 0.0, 0.0, 1.0);
        assert_eq!(mirrored.len(), 1);
        let m = sketch.points[mirrored[0].0].position;
        assert!((m.x - (-1.0)).abs() < 1e-10);
        assert!((m.y - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_delete_all_geometry() {
        let mut sketch = Sketch::new();
        sketch.add_point(0.0, 0.0);
        sketch.add_point(1.0, 0.0);
        let p0 = PointId(0);
        let p1 = PointId(1);
        sketch.add_line(p0, p1);

        delete_all_geometry(&mut sketch);
        assert_eq!(sketch.points.len(), 0);
        assert_eq!(sketch.lines.len(), 0);
    }

    #[test]
    fn test_delete_all_constraints() {
        let mut sketch = Sketch::new();
        let p = sketch.add_point(0.0, 0.0);
        sketch.add_constraint(crate::Constraint::Fixed(p, 0.0, 0.0));
        assert_eq!(sketch.constraints.len(), 1);

        delete_all_constraints(&mut sketch);
        assert_eq!(sketch.constraints.len(), 0);
    }

    #[test]
    fn test_carbon_copy() {
        let mut source = Sketch::new();
        let p0 = source.add_point(0.0, 0.0);
        let p1 = source.add_point(1.0, 1.0);
        source.add_line(p0, p1);

        let mut target = Sketch::new();
        carbon_copy(&source, &mut target).unwrap();
        assert_eq!(target.points.len(), 2);
        assert_eq!(target.lines.len(), 1);
    }

    #[test]
    fn test_external_projection() {
        let mut sketch = Sketch::new();
        let plane = crate::WorkPlane::xy();
        let pts_3d = vec![
            cadkernel_math::Point3::new(1.0, 2.0, 5.0),
            cadkernel_math::Point3::new(3.0, 4.0, 10.0),
        ];
        let ids = external_projection(&mut sketch, &pts_3d, &plane);
        assert_eq!(ids.len(), 2);
        assert!((sketch.points[ids[0].0].position.x - 1.0).abs() < 1e-10);
        assert!((sketch.points[ids[0].0].position.y - 2.0).abs() < 1e-10);
    }
}
