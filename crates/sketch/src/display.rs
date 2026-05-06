//! Sketch display options, grid, snap, and UI helper operations.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::Point2;

use crate::{Constraint, PointId, Sketch, SketchArc, SketchCircle, SketchLine, SketchPoint};

// ---------------------------------------------------------------------------
// Sketch Grid
// ---------------------------------------------------------------------------

/// Grid configuration for a sketch view.
#[derive(Debug, Clone)]
pub struct SketchGrid {
    pub spacing: f64,
    pub subdivisions: u32,
    pub visible: bool,
    pub snap_to_grid: bool,
}

impl Default for SketchGrid {
    fn default() -> Self {
        Self {
            spacing: 10.0,
            subdivisions: 5,
            visible: true,
            snap_to_grid: false,
        }
    }
}

impl SketchGrid {
    pub fn new(spacing: f64, subdivisions: u32) -> Self {
        Self {
            spacing: spacing.max(0.001),
            subdivisions: subdivisions.max(1),
            ..Default::default()
        }
    }

    /// Snaps a point to the nearest grid intersection.
    pub fn snap_point(&self, p: Point2) -> Point2 {
        let sub_spacing = self.spacing / self.subdivisions as f64;
        let x = (p.x / sub_spacing).round() * sub_spacing;
        let y = (p.y / sub_spacing).round() * sub_spacing;
        Point2::new(x, y)
    }
}

// ---------------------------------------------------------------------------
// Snap System
// ---------------------------------------------------------------------------

/// What type of snap was performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapType {
    Endpoint,
    Midpoint,
    Center,
    Intersection,
    Perpendicular,
    Parallel,
    Grid,
}

/// Snap mode configuration.
#[derive(Debug, Clone)]
pub struct SketchSnap {
    pub snap_to_endpoint: bool,
    pub snap_to_midpoint: bool,
    pub snap_to_center: bool,
    pub snap_to_intersection: bool,
    pub snap_to_perp: bool,
    pub snap_to_parallel: bool,
    pub snap_to_grid: bool,
}

impl Default for SketchSnap {
    fn default() -> Self {
        Self {
            snap_to_endpoint: true,
            snap_to_midpoint: true,
            snap_to_center: true,
            snap_to_intersection: false,
            snap_to_perp: false,
            snap_to_parallel: false,
            snap_to_grid: true,
        }
    }
}

/// Finds the nearest snap point in the sketch geometry.
///
/// Checks endpoints, midpoints, centers, and grid intersections.
/// Returns the snapped point and what type of snap was performed.
pub fn snap_to_sketch_geometry(
    sketch: &Sketch,
    point: Point2,
    snap: &SketchSnap,
    grid: &SketchGrid,
) -> Option<(Point2, SnapType)> {
    let mut best: Option<(Point2, SnapType, f64)> = None;
    let mut consider = |p: Point2, st: SnapType| {
        let d = ((p.x - point.x).powi(2) + (p.y - point.y).powi(2)).sqrt();
        if best.as_ref().is_none_or(|(_, _, bd)| d < *bd) {
            best = Some((p, st, d));
        }
    };

    // Endpoints
    if snap.snap_to_endpoint {
        for pt in &sketch.points {
            consider(pt.position, SnapType::Endpoint);
        }
        for line in &sketch.lines {
            consider(sketch.points[line.start.0].position, SnapType::Endpoint);
            consider(sketch.points[line.end.0].position, SnapType::Endpoint);
        }
        for arc in &sketch.arcs {
            consider(
                sketch.points[arc.start_point.0].position,
                SnapType::Endpoint,
            );
            consider(sketch.points[arc.end_point.0].position, SnapType::Endpoint);
        }
    }

    // Midpoints
    if snap.snap_to_midpoint {
        for line in &sketch.lines {
            let s = sketch.points[line.start.0].position;
            let e = sketch.points[line.end.0].position;
            consider(
                Point2::new((s.x + e.x) * 0.5, (s.y + e.y) * 0.5),
                SnapType::Midpoint,
            );
        }
    }

    // Centers
    if snap.snap_to_center {
        for circle in &sketch.circles {
            consider(sketch.points[circle.center.0].position, SnapType::Center);
        }
        for arc in &sketch.arcs {
            consider(sketch.points[arc.center.0].position, SnapType::Center);
        }
    }

    // Grid
    if snap.snap_to_grid && grid.visible {
        let gp = grid.snap_point(point);
        consider(gp, SnapType::Grid);
    }

    best.map(|(p, st, _)| (p, st))
}

// ---------------------------------------------------------------------------
// Display Options
// ---------------------------------------------------------------------------

/// Visual display toggles for the sketch editor.
#[derive(Debug, Clone)]
pub struct SketchDisplayOptions {
    pub show_constraints: bool,
    pub show_construction: bool,
    pub show_internal_geometry: bool,
    pub show_dof: bool,
    pub show_knot_multiplicity: bool,
    pub show_control_polygons: bool,
    pub show_weight: bool,
    pub show_bspline_degree: bool,
    pub show_bspline_comb: bool,
    pub auto_constraints: bool,
    pub auto_remove_redundant: bool,
    pub rendering_order: u32,
    pub show_grid: bool,
}

impl Default for SketchDisplayOptions {
    fn default() -> Self {
        Self {
            show_constraints: true,
            show_construction: true,
            show_internal_geometry: false,
            show_dof: false,
            show_knot_multiplicity: false,
            show_control_polygons: false,
            show_weight: false,
            show_bspline_degree: false,
            show_bspline_comb: false,
            auto_constraints: true,
            auto_remove_redundant: true,
            rendering_order: 0,
            show_grid: true,
        }
    }
}

/// Toggles constraint visibility in display options.
pub fn toggle_constraints_visibility(options: &mut SketchDisplayOptions) {
    options.show_constraints = !options.show_constraints;
}

// ---------------------------------------------------------------------------
// Sketch utility operations
// ---------------------------------------------------------------------------

/// Toggles construction mode for a specific entity.
///
/// If the entity is currently regular, it becomes construction geometry;
/// if it is already construction, it reverts to regular geometry.
/// Currently supports points and lines (by index).
pub fn toggle_construction(sketch: &mut Sketch, entity_id: usize) -> KernelResult<()> {
    // Try toggling as point
    let pid = PointId(entity_id);
    if entity_id < sketch.points.len() {
        if let Some(pos) = sketch.construction_points.iter().position(|&id| id == pid) {
            sketch.construction_points.remove(pos);
        } else {
            sketch.construction_points.push(pid);
        }
        return Ok(());
    }

    // Try toggling as line
    let lid = crate::LineId(entity_id);
    if entity_id < sketch.lines.len() {
        if let Some(pos) = sketch.construction_lines.iter().position(|&id| id == lid) {
            sketch.construction_lines.remove(pos);
        } else {
            sketch.construction_lines.push(lid);
        }
        return Ok(());
    }

    Err(KernelError::InvalidArgument(
        "entity_id does not refer to a point or line".into(),
    ))
}

/// Returns the sketch origin point (0, 0).
pub fn select_origin(_sketch: &Sketch) -> Point2 {
    Point2::new(0.0, 0.0)
}

/// Returns the horizontal axis endpoints (large range).
pub fn select_h_axis(_sketch: &Sketch) -> (Point2, Point2) {
    (Point2::new(-1e6, 0.0), Point2::new(1e6, 0.0))
}

/// Returns the vertical axis endpoints.
pub fn select_v_axis(_sketch: &Sketch) -> (Point2, Point2) {
    (Point2::new(0.0, -1e6), Point2::new(0.0, 1e6))
}

/// Removes horizontal and vertical constraints from the given entity IDs.
pub fn remove_axes_alignment(sketch: &mut Sketch, entity_ids: &[usize]) -> KernelResult<()> {
    sketch.constraints.retain(|c| match c {
        Constraint::Horizontal(lid) => !entity_ids.contains(&lid.0),
        Constraint::Vertical(lid) => !entity_ids.contains(&lid.0),
        _ => true,
    });
    Ok(())
}

/// A snapshot of a sketch entity for copy/paste.
#[derive(Debug, Clone)]
pub enum SketchEntity {
    Point(SketchPoint),
    Line(SketchLine),
    Arc(SketchArc),
    Circle(SketchCircle),
}

/// Copies sketch entities by their point indices.
pub fn copy_entities(sketch: &Sketch, entity_ids: &[usize]) -> Vec<SketchEntity> {
    let mut result = Vec::new();
    for &id in entity_ids {
        if id < sketch.points.len() {
            result.push(SketchEntity::Point(sketch.points[id]));
        }
    }
    result
}

/// Pastes sketch entities into the sketch with an offset, returns new point indices.
pub fn paste_entities(
    sketch: &mut Sketch,
    entities: &[SketchEntity],
    offset: Point2,
) -> Vec<usize> {
    let mut new_ids = Vec::new();
    for entity in entities {
        match entity {
            SketchEntity::Point(pt) => {
                let pid = sketch.add_point(pt.position.x + offset.x, pt.position.y + offset.y);
                new_ids.push(pid.0);
            }
            SketchEntity::Line(line) => {
                // Copy the referenced points with offset
                let s = sketch.points[line.start.0].position;
                let e = sketch.points[line.end.0].position;
                let p0 = sketch.add_point(s.x + offset.x, s.y + offset.y);
                let p1 = sketch.add_point(e.x + offset.x, e.y + offset.y);
                let lid = sketch.add_line(p0, p1);
                new_ids.push(lid.0);
            }
            SketchEntity::Arc(arc) => {
                let cp = sketch.points[arc.center.0].position;
                let sp = sketch.points[arc.start_point.0].position;
                let ep = sketch.points[arc.end_point.0].position;
                let c = sketch.add_point(cp.x + offset.x, cp.y + offset.y);
                let s = sketch.add_point(sp.x + offset.x, sp.y + offset.y);
                let e = sketch.add_point(ep.x + offset.x, ep.y + offset.y);
                let aid = sketch.add_arc(c, s, e, arc.radius, arc.start_angle, arc.end_angle);
                new_ids.push(aid.0);
            }
            SketchEntity::Circle(circle) => {
                let cp = sketch.points[circle.center.0].position;
                let c = sketch.add_point(cp.x + offset.x, cp.y + offset.y);
                let cid = sketch.add_circle(c, circle.radius);
                new_ids.push(cid.0);
            }
        }
    }
    new_ids
}

/// Adds a closed periodic B-spline with explicit knot vector.
pub fn add_periodic_bspline_from_knots(
    sketch: &mut Sketch,
    control_points: Vec<PointId>,
    knots: Vec<f64>,
    degree: usize,
) -> crate::BSplineId {
    let id = crate::BSplineId(sketch.bsplines.len());
    sketch.bsplines.push(crate::SketchBSpline {
        control_points,
        degree,
        closed: true,
        knots,
    });
    id
}

/// Auto-detects the appropriate dimension constraint for a selection.
///
/// Returns `Constraint::Length` for lines, `Constraint::Radius` for arcs,
/// `Constraint::Diameter` for circles, or `Constraint::Distance` for two points.
pub fn contextual_dimension(sketch: &Sketch, selection: &[usize]) -> Option<Constraint> {
    match selection.len() {
        1 => {
            let id = selection[0];
            // Check if it is a line
            if id < sketch.lines.len() {
                let line = &sketch.lines[id];
                let s = sketch.points[line.start.0].position;
                let e = sketch.points[line.end.0].position;
                let len = ((e.x - s.x).powi(2) + (e.y - s.y).powi(2)).sqrt();
                return Some(Constraint::Length(crate::LineId(id), len));
            }
            // Check if it is a circle
            if id < sketch.circles.len() {
                let circle = &sketch.circles[id];
                return Some(Constraint::Diameter(
                    circle.center,
                    PointId(0),
                    circle.radius * 2.0,
                ));
            }
            None
        }
        2 => {
            // Two points -> distance
            let a = selection[0];
            let b = selection[1];
            if a < sketch.points.len() && b < sketch.points.len() {
                let pa = sketch.points[a].position;
                let pb = sketch.points[b].position;
                let d = ((pb.x - pa.x).powi(2) + (pb.y - pa.y).powi(2)).sqrt();
                return Some(Constraint::Distance(PointId(a), PointId(b), d));
            }
            None
        }
        _ => None,
    }
}

/// Returns `Constraint::Radius` for arcs, `Constraint::Diameter` for circles.
pub fn unified_radius_diameter(sketch: &Sketch, entity_id: usize) -> Option<Constraint> {
    // Check arcs first
    if entity_id < sketch.arcs.len() {
        let arc = &sketch.arcs[entity_id];
        return Some(Constraint::Radius(arc.center, arc.start_point, arc.radius));
    }
    // Then circles
    if entity_id < sketch.circles.len() {
        let circle = &sketch.circles[entity_id];
        return Some(Constraint::Diameter(
            circle.center,
            PointId(0),
            circle.radius * 2.0,
        ));
    }
    None
}

/// Returns `Constraint::Horizontal` or `Constraint::Vertical` based on line orientation.
pub fn unified_horizontal_vertical(sketch: &Sketch, entity_id: usize) -> Option<Constraint> {
    if entity_id >= sketch.lines.len() {
        return None;
    }
    let line = &sketch.lines[entity_id];
    let s = sketch.points[line.start.0].position;
    let e = sketch.points[line.end.0].position;
    let dx = (e.x - s.x).abs();
    let dy = (e.y - s.y).abs();

    if dx > dy {
        Some(Constraint::Horizontal(crate::LineId(entity_id)))
    } else {
        Some(Constraint::Vertical(crate::LineId(entity_id)))
    }
}

/// Computes (yaw, pitch, roll) camera angles to align view with a sketch plane.
///
/// The plane is defined by its origin and normal in world space.
pub fn align_view_to_sketch(
    _plane_origin: cadkernel_math::Point3,
    plane_normal: cadkernel_math::Vec3,
) -> (f64, f64, f64) {
    let n = plane_normal.normalized().unwrap_or(cadkernel_math::Vec3::Z);
    let yaw = n.x.atan2(n.z);
    let pitch = (-n.y).asin();
    let roll = 0.0;
    (yaw, pitch, roll)
}

/// State for a section view (clipping plane).
#[derive(Debug, Clone)]
pub struct SectionViewState {
    pub plane_normal: cadkernel_math::Vec3,
    pub plane_point: cadkernel_math::Point3,
    pub enabled: bool,
}

/// Creates or toggles a section view state with the given plane and enabled flag.
pub fn toggle_section_view(
    plane_normal: cadkernel_math::Vec3,
    plane_point: cadkernel_math::Point3,
    enabled: bool,
) -> SectionViewState {
    SectionViewState {
        plane_normal,
        plane_point,
        enabled,
    }
}

/// Cancels the current sketch operation and resets pending state.
pub fn stop_operation(sketch: &mut Sketch) {
    sketch.construction_mode = false;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_simple_sketch() -> Sketch {
        let mut s = Sketch::new();
        let p0 = s.add_point(0.0, 0.0);
        let p1 = s.add_point(10.0, 0.0);
        let p2 = s.add_point(10.0, 10.0);
        s.add_line(p0, p1);
        s.add_line(p1, p2);
        let pc = s.add_point(5.0, 5.0);
        s.add_circle(pc, 3.0);
        s
    }

    #[test]
    fn test_grid_default() {
        let g = SketchGrid::default();
        assert!((g.spacing - 10.0).abs() < 1e-10);
        assert_eq!(g.subdivisions, 5);
    }

    #[test]
    fn test_grid_snap() {
        let g = SketchGrid::new(10.0, 2);
        let p = g.snap_point(Point2::new(7.3, 12.8));
        assert!((p.x - 5.0).abs() < 1e-10);
        assert!((p.y - 15.0).abs() < 1e-10);
    }

    #[test]
    fn test_snap_to_endpoint() {
        let sketch = make_simple_sketch();
        let snap = SketchSnap {
            snap_to_endpoint: true,
            ..Default::default()
        };
        let grid = SketchGrid::default();
        let result = snap_to_sketch_geometry(&sketch, Point2::new(0.1, 0.1), &snap, &grid);
        assert!(result.is_some());
        let (p, st) = result.unwrap();
        assert!((p.x).abs() < 1e-6);
        assert!((p.y).abs() < 1e-6);
        assert_eq!(st, SnapType::Endpoint);
    }

    #[test]
    fn test_snap_to_midpoint() {
        let sketch = make_simple_sketch();
        let snap = SketchSnap {
            snap_to_endpoint: false,
            snap_to_midpoint: true,
            snap_to_center: false,
            snap_to_grid: false,
            ..Default::default()
        };
        let grid = SketchGrid {
            visible: false,
            ..Default::default()
        };
        let result = snap_to_sketch_geometry(&sketch, Point2::new(5.0, 0.1), &snap, &grid);
        assert!(result.is_some());
        let (p, st) = result.unwrap();
        assert!((p.x - 5.0).abs() < 1e-6);
        assert!((p.y).abs() < 1e-6);
        assert_eq!(st, SnapType::Midpoint);
    }

    #[test]
    fn test_snap_to_center() {
        let sketch = make_simple_sketch();
        let snap = SketchSnap {
            snap_to_endpoint: false,
            snap_to_midpoint: false,
            snap_to_center: true,
            snap_to_grid: false,
            ..Default::default()
        };
        let grid = SketchGrid {
            visible: false,
            ..Default::default()
        };
        let result = snap_to_sketch_geometry(&sketch, Point2::new(5.1, 5.1), &snap, &grid);
        assert!(result.is_some());
        let (_, st) = result.unwrap();
        assert_eq!(st, SnapType::Center);
    }

    #[test]
    fn test_display_options_default() {
        let opts = SketchDisplayOptions::default();
        assert!(opts.show_constraints);
        assert!(opts.auto_constraints);
        assert!(opts.show_grid);
    }

    #[test]
    fn test_toggle_constraints_visibility() {
        let mut opts = SketchDisplayOptions::default();
        assert!(opts.show_constraints);
        toggle_constraints_visibility(&mut opts);
        assert!(!opts.show_constraints);
        toggle_constraints_visibility(&mut opts);
        assert!(opts.show_constraints);
    }

    #[test]
    fn test_toggle_construction() {
        let mut sketch = make_simple_sketch();
        assert!(sketch.construction_points.is_empty());
        toggle_construction(&mut sketch, 0).unwrap();
        assert_eq!(sketch.construction_points.len(), 1);
        toggle_construction(&mut sketch, 0).unwrap();
        assert!(sketch.construction_points.is_empty());
    }

    #[test]
    fn test_select_origin() {
        let sketch = Sketch::new();
        let o = select_origin(&sketch);
        assert!((o.x).abs() < 1e-10);
        assert!((o.y).abs() < 1e-10);
    }

    #[test]
    fn test_select_axes() {
        let sketch = Sketch::new();
        let (h0, h1) = select_h_axis(&sketch);
        assert!(h0.x < 0.0);
        assert!(h1.x > 0.0);
        assert!((h0.y).abs() < 1e-10);

        let (v0, v1) = select_v_axis(&sketch);
        assert!((v0.x).abs() < 1e-10);
        assert!(v0.y < 0.0);
        assert!(v1.y > 0.0);
    }

    #[test]
    fn test_remove_axes_alignment() {
        let mut sketch = make_simple_sketch();
        sketch.add_constraint(Constraint::Horizontal(crate::LineId(0)));
        sketch.add_constraint(Constraint::Vertical(crate::LineId(1)));
        sketch.add_constraint(Constraint::Length(crate::LineId(0), 10.0));

        remove_axes_alignment(&mut sketch, &[0, 1]).unwrap();
        assert_eq!(sketch.constraints.len(), 1); // Only Length remains
    }

    #[test]
    fn test_copy_paste_entities() {
        let mut sketch = make_simple_sketch();
        let copies = copy_entities(&sketch, &[0, 1]);
        assert_eq!(copies.len(), 2);

        let offset = Point2::new(20.0, 0.0);
        let new_ids = paste_entities(&mut sketch, &copies, offset);
        assert_eq!(new_ids.len(), 2);

        // Check the pasted point is offset
        let new_pt = &sketch.points[*new_ids.first().unwrap()];
        assert!((new_pt.position.x - 20.0).abs() < 1e-10);
    }

    #[test]
    fn test_contextual_dimension_line() {
        let sketch = make_simple_sketch();
        let c = contextual_dimension(&sketch, &[0]);
        assert!(c.is_some());
        if let Some(Constraint::Length(_, len)) = c {
            assert!((len - 10.0).abs() < 1e-6);
        } else {
            panic!("expected Length constraint");
        }
    }

    #[test]
    fn test_contextual_dimension_two_points() {
        let sketch = make_simple_sketch();
        let c = contextual_dimension(&sketch, &[0, 1]);
        assert!(c.is_some());
        if let Some(Constraint::Distance(_, _, d)) = c {
            assert!((d - 10.0).abs() < 1e-6);
        } else {
            panic!("expected Distance constraint");
        }
    }

    #[test]
    fn test_unified_radius_diameter() {
        let mut sketch = Sketch::new();
        let c = sketch.add_point(0.0, 0.0);
        sketch.add_circle(c, 5.0);

        let r = unified_radius_diameter(&sketch, 0);
        assert!(r.is_some());
        if let Some(Constraint::Diameter(_, _, d)) = r {
            assert!((d - 10.0).abs() < 1e-6);
        } else {
            panic!("expected Diameter constraint");
        }
    }

    #[test]
    fn test_unified_horizontal_vertical() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(10.0, 0.0);
        sketch.add_line(p0, p1);

        let c = unified_horizontal_vertical(&sketch, 0);
        assert!(matches!(c, Some(Constraint::Horizontal(_))));
    }

    #[test]
    fn test_align_view_to_sketch_xy() {
        let (yaw, pitch, roll) =
            align_view_to_sketch(cadkernel_math::Point3::ORIGIN, cadkernel_math::Vec3::Z);
        assert!((roll).abs() < 1e-10);
        assert!((pitch).abs() < 1e-10);
        // yaw = atan2(0, 1) = 0
        assert!((yaw).abs() < 1e-10);
    }

    #[test]
    fn test_stop_operation() {
        let mut sketch = Sketch::new();
        sketch.construction_mode = true;
        stop_operation(&mut sketch);
        assert!(!sketch.construction_mode);
    }

    #[test]
    fn test_periodic_bspline_from_knots() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(1.0, 0.0);
        let p2 = sketch.add_point(1.0, 1.0);
        let p3 = sketch.add_point(0.0, 1.0);

        let knots = vec![0.0, 0.0, 0.0, 0.25, 0.5, 0.75, 1.0, 1.0, 1.0];
        let bsp = add_periodic_bspline_from_knots(&mut sketch, vec![p0, p1, p2, p3], knots, 2);
        assert!(sketch.bsplines[bsp.0].closed);
        assert_eq!(sketch.bsplines[bsp.0].knots.len(), 9);
    }

    #[test]
    fn test_toggle_section_view_enabled() {
        let state = toggle_section_view(
            cadkernel_math::Vec3::Z,
            cadkernel_math::Point3::new(0.0, 0.0, 5.0),
            true,
        );
        assert!(state.enabled);
        assert!((state.plane_normal.z - 1.0).abs() < 1e-10);
        assert!((state.plane_point.z - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_toggle_section_view_disabled() {
        let state = toggle_section_view(
            cadkernel_math::Vec3::X,
            cadkernel_math::Point3::ORIGIN,
            false,
        );
        assert!(!state.enabled);
        assert!((state.plane_normal.x - 1.0).abs() < 1e-10);
    }
}
