use super::{DimensionKind, GuiState, SketchEntityRef, SketchTool};
use crate::render::{Camera, GridConfig, dot3, normalize3, sub3};
use cadkernel_sketch::{Constraint, ExternalReferenceGeometry, SketchDimensionKind};

// ---------------------------------------------------------------------------
// Auto-constraint detection thresholds
// ---------------------------------------------------------------------------
const ANGLE_SNAP_DEG: f64 = 5.0;
const POINT_SNAP_DIST: f64 = 0.5;
const MIDPOINT_SNAP_DIST: f64 = 0.5;
const GRID_SNAP_DIST: f64 = 0.15;
const DIMENSION_HIT_RADIUS: f32 = 18.0;
const POINT_HANDLE_RADIUS: f32 = 7.0;

// ---------------------------------------------------------------------------
// Auto-constraint indicator kinds
// ---------------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq, Eq)]
enum AutoConstraintKind {
    Coincident,
    Horizontal,
    Vertical,
    OnPoint,
    Midpoint,
    GridSnap,
    Intersection,
}

impl AutoConstraintKind {
    fn symbol(self) -> &'static str {
        match self {
            Self::Coincident => "\u{2295}",
            Self::Horizontal => "\u{2014}",
            Self::Vertical => "|",
            Self::OnPoint => "\u{25CF}",
            Self::Midpoint => "\u{25C6}",
            Self::GridSnap => "\u{229E}",
            Self::Intersection => "\u{2A2F}",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Coincident => "Coincident",
            Self::Horizontal => "Horizontal",
            Self::Vertical => "Vertical",
            Self::OnPoint => "On Point",
            Self::Midpoint => "Midpoint",
            Self::GridSnap => "Grid Snap",
            Self::Intersection => "Intersection",
        }
    }
}

// ---------------------------------------------------------------------------
// Screen-to-sketch-plane ray cast (local to this module)
// ---------------------------------------------------------------------------
fn screen_to_sketch(
    camera: &Camera,
    viewport: egui::Rect,
    plane: &cadkernel_sketch::WorkPlane,
    screen_pos: egui::Pos2,
) -> Option<(f64, f64)> {
    let w = viewport.width();
    let h = viewport.height();
    if w < 1.0 || h < 1.0 {
        return None;
    }
    let ndc_x = ((screen_pos.x - viewport.left()) / w) * 2.0 - 1.0;
    let ndc_y = 1.0 - ((screen_pos.y - viewport.top()) / h) * 2.0;

    let eye = camera.eye();
    let r = camera.screen_right();
    let u = camera.screen_up();
    let f = normalize3(sub3(camera.target, eye));

    let half_fov_tan = (camera.fovy * 0.5).tan();
    let dir = normalize3([
        f[0] + ndc_x * r[0] * half_fov_tan * camera.aspect + ndc_y * u[0] * half_fov_tan,
        f[1] + ndc_x * r[1] * half_fov_tan * camera.aspect + ndc_y * u[1] * half_fov_tan,
        f[2] + ndc_x * r[2] * half_fov_tan * camera.aspect + ndc_y * u[2] * half_fov_tan,
    ]);

    let pn = [
        plane.normal.x as f32,
        plane.normal.y as f32,
        plane.normal.z as f32,
    ];
    let po = [
        plane.origin.x as f32,
        plane.origin.y as f32,
        plane.origin.z as f32,
    ];
    let denom = dot3(dir, pn);
    if denom.abs() < 1e-6 {
        return None;
    }
    let t = dot3(sub3(po, eye), pn) / denom;
    if t < 0.0 {
        return None;
    }
    let hit = [
        eye[0] + t * dir[0],
        eye[1] + t * dir[1],
        eye[2] + t * dir[2],
    ];
    let rel = sub3(hit, po);
    let xa = [
        plane.x_axis.x as f32,
        plane.x_axis.y as f32,
        plane.x_axis.z as f32,
    ];
    let ya = [
        plane.y_axis.x as f32,
        plane.y_axis.y as f32,
        plane.y_axis.z as f32,
    ];
    Some((dot3(rel, xa) as f64, dot3(rel, ya) as f64))
}

#[derive(Clone, Copy)]
struct DimensionHit {
    constraint_index: usize,
    kind: DimensionKind,
    value: f64,
    pos: egui::Pos2,
}

fn ui_dimension_kind(kind: SketchDimensionKind) -> DimensionKind {
    match kind {
        SketchDimensionKind::Distance => DimensionKind::Distance,
        SketchDimensionKind::Angle => DimensionKind::Angle,
        SketchDimensionKind::Radius => DimensionKind::Radius,
        SketchDimensionKind::Length => DimensionKind::Length,
        SketchDimensionKind::Diameter => DimensionKind::Diameter,
        SketchDimensionKind::HorizontalDistance => DimensionKind::HDistance,
        SketchDimensionKind::VerticalDistance => DimensionKind::VDistance,
    }
}

fn format_dimension_edit_value(value: f64) -> String {
    let mut s = format!("{value:.4}");
    while s.contains('.') && s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    s
}

fn dim_line_label_pos(a: egui::Pos2, b: egui::Pos2, offset: f32) -> Option<egui::Pos2> {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 2.0 {
        return None;
    }
    let nx = -dy / len;
    let ny = dx / len;
    let off = egui::vec2(nx * offset, ny * offset);
    let sa = a + off;
    let sb = b + off;
    Some(egui::pos2((sa.x + sb.x) * 0.5, (sa.y + sb.y) * 0.5) + egui::vec2(0.0, -9.0))
}

fn collect_dimension_hits(
    sm: &super::SketchMode,
    project: &dyn Fn(f64, f64) -> Option<egui::Pos2>,
) -> Vec<DimensionHit> {
    let pt_pos = |pid: cadkernel_sketch::PointId| -> Option<(f64, f64)> {
        sm.sketch
            .points
            .get(pid.0)
            .map(|p| (p.position.x, p.position.y))
    };
    let line_ends = |lid: cadkernel_sketch::LineId| -> Option<((f64, f64), (f64, f64))> {
        let line = sm.sketch.lines.get(lid.0)?;
        let s = sm.sketch.points.get(line.start.0)?;
        let e = sm.sketch.points.get(line.end.0)?;
        Some(((s.position.x, s.position.y), (e.position.x, e.position.y)))
    };

    let mut hits = Vec::new();
    for (constraint_index, constraint) in sm.sketch.constraints.iter().enumerate() {
        let Ok(dim) = sm.sketch.dimension_constraint_value(constraint_index) else {
            continue;
        };
        let kind = ui_dimension_kind(dim.kind);
        let pos = match constraint {
            Constraint::Distance(p0, p1, _) => {
                let (Some(a), Some(b)) = (pt_pos(*p0), pt_pos(*p1)) else {
                    continue;
                };
                let (Some(sa), Some(sb)) = (project(a.0, a.1), project(b.0, b.1)) else {
                    continue;
                };
                dim_line_label_pos(sa, sb, 14.0)
            }
            Constraint::Length(lid, _) => {
                let Some((s, e)) = line_ends(*lid) else {
                    continue;
                };
                let (Some(sa), Some(sb)) = (project(s.0, s.1), project(e.0, e.1)) else {
                    continue;
                };
                dim_line_label_pos(sa, sb, 14.0)
            }
            Constraint::Radius(center, edge, _) => {
                let (Some(cp), Some(ep)) = (pt_pos(*center), pt_pos(*edge)) else {
                    continue;
                };
                let (Some(sc), Some(se)) = (project(cp.0, cp.1), project(ep.0, ep.1)) else {
                    continue;
                };
                Some(egui::pos2((sc.x + se.x) * 0.5, (sc.y + se.y) * 0.5) + egui::vec2(0.0, -9.0))
            }
            Constraint::Diameter(center, edge, _) => {
                let (Some(cp), Some(ep)) = (pt_pos(*center), pt_pos(*edge)) else {
                    continue;
                };
                let dx = ep.0 - cp.0;
                let dy = ep.1 - cp.1;
                let opp = (cp.0 - dx, cp.1 - dy);
                let (Some(se), Some(so)) = (project(ep.0, ep.1), project(opp.0, opp.1)) else {
                    continue;
                };
                Some(egui::pos2((so.x + se.x) * 0.5, (so.y + se.y) * 0.5) + egui::vec2(0.0, -9.0))
            }
            Constraint::Angle(l0, _l1, angle) => {
                let Some((s0, e0)) = line_ends(*l0) else {
                    continue;
                };
                let start = (e0.1 - s0.1).atan2(e0.0 - s0.0);
                let mid_t = start + *angle * 0.5;
                let label_r = 20.0_f64 * 1.4;
                project(s0.0 + label_r * mid_t.cos(), s0.1 + label_r * mid_t.sin())
            }
            Constraint::HorizontalDistance(p0, p1, _) => {
                let (Some(a), Some(b)) = (pt_pos(*p0), pt_pos(*p1)) else {
                    continue;
                };
                let y = (a.1 + b.1) * 0.5;
                let (Some(sa), Some(sb)) = (project(a.0, y), project(b.0, y)) else {
                    continue;
                };
                Some(egui::pos2((sa.x + sb.x) * 0.5, (sa.y + sb.y) * 0.5) + egui::vec2(0.0, -9.0))
            }
            Constraint::VerticalDistance(p0, p1, _) => {
                let (Some(a), Some(b)) = (pt_pos(*p0), pt_pos(*p1)) else {
                    continue;
                };
                let x = (a.0 + b.0) * 0.5;
                let (Some(sa), Some(sb)) = (project(x, a.1), project(x, b.1)) else {
                    continue;
                };
                Some(egui::pos2((sa.x + sb.x) * 0.5, (sa.y + sb.y) * 0.5) + egui::vec2(8.0, 0.0))
            }
            _ => None,
        };
        if let Some(pos) = pos {
            hits.push(DimensionHit {
                constraint_index,
                kind,
                value: dim.value,
                pos,
            });
        }
    }
    hits
}

fn detect_sketch_conflicts(sm: &super::SketchMode) -> Vec<cadkernel_sketch::ConstraintId> {
    if sm.sketch.constraints.len() < 2 {
        return Vec::new();
    }
    let ids: Vec<_> = (0..sm.sketch.constraints.len()).collect();
    cadkernel_sketch::detect_conflict(&sm.sketch, &ids)
}

// ---------------------------------------------------------------------------
// Detect auto-constraints near the cursor
// ---------------------------------------------------------------------------
fn detect_auto_constraints(
    sm: &super::SketchMode,
    cursor: (f64, f64),
    pending: Option<(f64, f64)>,
) -> Vec<AutoConstraintKind> {
    let mut result = Vec::new();

    let mut near_point = false;
    for pt in &sm.sketch.points {
        let dx = pt.position.x - cursor.0;
        let dy = pt.position.y - cursor.1;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist < POINT_SNAP_DIST {
            near_point = true;
            break;
        }
    }
    if near_point {
        if pending.is_some() {
            result.push(AutoConstraintKind::Coincident);
        } else {
            result.push(AutoConstraintKind::OnPoint);
        }
    }

    if let Some((px, py)) = pending {
        let dx = cursor.0 - px;
        let dy = cursor.1 - py;
        let len = (dx * dx + dy * dy).sqrt();
        if len > 1e-9 {
            let angle = dy.atan2(dx).to_degrees().abs();
            if angle < ANGLE_SNAP_DEG || (180.0 - angle).abs() < ANGLE_SNAP_DEG {
                result.push(AutoConstraintKind::Horizontal);
            }
            if (angle - 90.0).abs() < ANGLE_SNAP_DEG {
                result.push(AutoConstraintKind::Vertical);
            }
        }
    }

    for line in &sm.sketch.lines {
        if line.start.0 < sm.sketch.points.len() && line.end.0 < sm.sketch.points.len() {
            let s = &sm.sketch.points[line.start.0];
            let e = &sm.sketch.points[line.end.0];
            let mx = (s.position.x + e.position.x) * 0.5;
            let my = (s.position.y + e.position.y) * 0.5;
            let dx = mx - cursor.0;
            let dy = my - cursor.1;
            if (dx * dx + dy * dy).sqrt() < MIDPOINT_SNAP_DIST {
                result.push(AutoConstraintKind::Midpoint);
                break;
            }
        }
    }

    // Intersection snap (line-line)
    let n_lines = sm.sketch.lines.len();
    'outer: for i in 0..n_lines {
        for j in (i + 1)..n_lines {
            let li = &sm.sketch.lines[i];
            let lj = &sm.sketch.lines[j];
            if li.start.0 >= sm.sketch.points.len()
                || li.end.0 >= sm.sketch.points.len()
                || lj.start.0 >= sm.sketch.points.len()
                || lj.end.0 >= sm.sketch.points.len()
            {
                continue;
            }
            let (ax, ay) = (
                sm.sketch.points[li.start.0].position.x,
                sm.sketch.points[li.start.0].position.y,
            );
            let (bx, by) = (
                sm.sketch.points[li.end.0].position.x,
                sm.sketch.points[li.end.0].position.y,
            );
            let (cx, cy) = (
                sm.sketch.points[lj.start.0].position.x,
                sm.sketch.points[lj.start.0].position.y,
            );
            let (dx2, dy2) = (
                sm.sketch.points[lj.end.0].position.x,
                sm.sketch.points[lj.end.0].position.y,
            );
            let denom = (bx - ax) * (dy2 - cy) - (by - ay) * (dx2 - cx);
            if denom.abs() < 1e-12 {
                continue;
            }
            let t = ((cx - ax) * (dy2 - cy) - (cy - ay) * (dx2 - cx)) / denom;
            let u = ((cx - ax) * (by - ay) - (cy - ay) * (bx - ax)) / denom;
            if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
                let ix = ax + t * (bx - ax);
                let iy = ay + t * (by - ay);
                let d = ((cursor.0 - ix).powi(2) + (cursor.1 - iy).powi(2)).sqrt();
                if d < POINT_SNAP_DIST {
                    result.push(AutoConstraintKind::Intersection);
                    break 'outer;
                }
            }
        }
    }

    if sm.snap_enabled {
        let g = sm.grid_spacing.max(0.01);
        let gx = (cursor.0 / g).round() * g;
        let gy = (cursor.1 / g).round() * g;
        let dx = gx - cursor.0;
        let dy = gy - cursor.1;
        if (dx * dx + dy * dy).sqrt() < GRID_SNAP_DIST {
            result.push(AutoConstraintKind::GridSnap);
        }
    }

    result
}

// ===========================================================================
// Main overlay entry point
// ===========================================================================

pub(crate) fn draw_sketch_overlay(ctx: &egui::Context, gui: &mut GuiState, camera: &Camera) {
    let conflict_ids = if let Some(sm) = &mut gui.sketch_mode {
        sm.update_constraint_status();
        let ids = detect_sketch_conflicts(sm);
        if !ids.is_empty() {
            gui.status_message = format!("Over-constrained: {} conflicts", ids.len());
        }
        ids
    } else {
        Vec::new()
    };
    let sm = match &gui.sketch_mode {
        Some(sm) => sm,
        None => return,
    };

    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("sketch_overlay"),
    ));
    let viewport = ctx.available_rect();

    let project = |x: f64, y: f64| -> Option<egui::Pos2> {
        let wp = sm.plane.to_world(x, y);
        world_to_screen(camera, viewport, [wp.x as f32, wp.y as f32, wp.z as f32])
    };

    // Compute current mouse position in sketch-plane coordinates
    let mouse_screen = ctx.input(|i| i.pointer.hover_pos());
    let mouse_sketch =
        mouse_screen.and_then(|sp| screen_to_sketch(camera, viewport, &sm.plane, sp));

    // FreeCAD-style sketch colors
    let point_color = egui::Color32::from_rgb(240, 240, 240); // bright white points
    let line_color = egui::Color32::from_rgb(240, 240, 240); // white geometry
    let selected_color = egui::Color32::from_rgb(50, 220, 50); // green selected
    let hovered_color = egui::Color32::from_rgb(120, 255, 100); // bright green hover
    let construction_color = egui::Color32::from_rgb(60, 120, 220); // blue construction
    let external_ref_color = egui::Color32::from_rgb(120, 210, 255); // light-blue references
    let pending_color = egui::Color32::from_rgb(255, 200, 50); // golden pending
    let constraint_color = egui::Color32::from_rgb(220, 60, 60); // red constraints
    let grid_color = egui::Color32::from_rgba_premultiplied(50, 55, 70, 35);
    let grid_axis_color = egui::Color32::from_rgba_premultiplied(90, 95, 110, 70);
    let point_radius = 3.5;
    let sel = &sm.selected_entities;
    let hov = sm.hovered_entity;
    let entity_color = |entity: SketchEntityRef, base: egui::Color32| -> (egui::Color32, f32) {
        if sel.contains(&entity) {
            (selected_color, 3.0)
        } else if hov == Some(entity) {
            (hovered_color, 2.5)
        } else {
            (base, 2.0)
        }
    };

    // Draw sketch grid
    if sm.show_grid {
        let spacing = sm.grid_spacing.max(0.01);
        let grid_half = (20.0 / spacing).ceil() as i32;
        for i in -grid_half..=grid_half {
            let f = i as f64 * spacing;
            let color = if i == 0 { grid_axis_color } else { grid_color };
            let width = if i == 0 { 1.5 } else { 0.5 };
            if let (Some(a), Some(b)) = (
                project((-grid_half as f64) * spacing, f),
                project((grid_half as f64) * spacing, f),
            ) {
                painter.line_segment([a, b], egui::Stroke::new(width, color));
            }
            if let (Some(a), Some(b)) = (
                project(f, (-grid_half as f64) * spacing),
                project(f, (grid_half as f64) * spacing),
            ) {
                painter.line_segment([a, b], egui::Stroke::new(width, color));
            }
        }
    }

    // Draw sketch origin axis labels (X=red, Y=green)
    {
        let axis_len = 2.0; // units in sketch space
        let x_color = egui::Color32::from_rgb(220, 60, 60);
        let y_color = egui::Color32::from_rgb(60, 200, 60);
        if let (Some(o), Some(xt), Some(yt)) = (
            project(0.0, 0.0),
            project(axis_len, 0.0),
            project(0.0, axis_len),
        ) {
            // X axis arrow
            painter.line_segment([o, xt], egui::Stroke::new(2.0, x_color));
            let dx = xt.x - o.x;
            let dy = xt.y - o.y;
            let xlen = (dx * dx + dy * dy).sqrt();
            if xlen > 12.0 {
                let ux = dx / xlen;
                let uy = dy / xlen;
                let h = 6.0_f32;
                let w = 3.0_f32;
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        xt,
                        egui::pos2(xt.x - ux * h + uy * w, xt.y - uy * h - ux * w),
                        egui::pos2(xt.x - ux * h - uy * w, xt.y - uy * h + ux * w),
                    ],
                    x_color,
                    egui::Stroke::NONE,
                ));
                painter.text(
                    egui::pos2(xt.x + ux * 8.0, xt.y + uy * 8.0),
                    egui::Align2::CENTER_CENTER,
                    "X",
                    egui::FontId::proportional(12.0),
                    x_color,
                );
            }
            // Y axis arrow
            painter.line_segment([o, yt], egui::Stroke::new(2.0, y_color));
            let dx = yt.x - o.x;
            let dy = yt.y - o.y;
            let ylen = (dx * dx + dy * dy).sqrt();
            if ylen > 12.0 {
                let ux = dx / ylen;
                let uy = dy / ylen;
                let h = 6.0_f32;
                let w = 3.0_f32;
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        yt,
                        egui::pos2(yt.x - ux * h + uy * w, yt.y - uy * h - ux * w),
                        egui::pos2(yt.x - ux * h - uy * w, yt.y - uy * h + ux * w),
                    ],
                    y_color,
                    egui::Stroke::NONE,
                ));
                painter.text(
                    egui::pos2(yt.x + ux * 8.0, yt.y + uy * 8.0),
                    egui::Align2::CENTER_CENTER,
                    "Y",
                    egui::FontId::proportional(12.0),
                    y_color,
                );
            }
            // Origin dot
            painter.circle_filled(o, 3.0, egui::Color32::WHITE);
        }
    }

    // Draw explicit external references as ghost geometry.
    for reference in &sm.sketch.external_references {
        match reference.geometry {
            ExternalReferenceGeometry::Point(pid) if pid.0 < sm.sketch.points.len() => {
                let pt = &sm.sketch.points[pid.0];
                if let Some(sp) = project(pt.position.x, pt.position.y) {
                    painter.circle_stroke(sp, 6.0, egui::Stroke::new(1.5, external_ref_color));
                    painter.circle_filled(sp, 2.0, external_ref_color);
                }
            }
            ExternalReferenceGeometry::Line(lid) if lid.0 < sm.sketch.lines.len() => {
                let line = &sm.sketch.lines[lid.0];
                if line.start.0 < sm.sketch.points.len() && line.end.0 < sm.sketch.points.len() {
                    let s = &sm.sketch.points[line.start.0];
                    let e = &sm.sketch.points[line.end.0];
                    if let (Some(sp), Some(ep)) = (
                        project(s.position.x, s.position.y),
                        project(e.position.x, e.position.y),
                    ) {
                        draw_dashed_line(&painter, sp, ep, external_ref_color, 2.0, 5.0, 3.0);
                    }
                }
            }
            _ => {}
        }
    }

    // Draw construction lines (dashed)
    for &lid in &sm.sketch.construction_lines {
        if lid.0 < sm.sketch.lines.len() {
            let line = &sm.sketch.lines[lid.0];
            let s = &sm.sketch.points[line.start.0];
            let e = &sm.sketch.points[line.end.0];
            if let (Some(sp), Some(ep)) = (
                project(s.position.x, s.position.y),
                project(e.position.x, e.position.y),
            ) {
                draw_dashed_line(&painter, sp, ep, construction_color, 2.0, 6.0, 4.0);
            }
        }
    }

    // Draw construction points (blue X marker)
    for &pid in &sm.sketch.construction_points {
        if pid.0 < sm.sketch.points.len() {
            let pt = &sm.sketch.points[pid.0];
            if let Some(sp) = project(pt.position.x, pt.position.y) {
                let h = 4.0_f32;
                let c_stroke = egui::Stroke::new(1.5, construction_color);
                painter.line_segment(
                    [
                        egui::pos2(sp.x - h, sp.y - h),
                        egui::pos2(sp.x + h, sp.y + h),
                    ],
                    c_stroke,
                );
                painter.line_segment(
                    [
                        egui::pos2(sp.x + h, sp.y - h),
                        egui::pos2(sp.x - h, sp.y + h),
                    ],
                    c_stroke,
                );
            }
        }
    }

    // Draw DOF arrows on underconstrained points (orange direction indicators)
    if !sm.sketch.points.is_empty() && sm.show_constraints {
        let dof_color = egui::Color32::from_rgb(255, 160, 30);
        let dof_arrow_len = 14.0_f32;
        let n_pts = sm.sketch.points.len();
        // Determine which points are constrained in X/Y
        let mut x_constrained = vec![false; n_pts];
        let mut y_constrained = vec![false; n_pts];
        for c in &sm.sketch.constraints {
            match c {
                Constraint::Fixed(p, ..) if p.0 < n_pts => {
                    x_constrained[p.0] = true;
                    y_constrained[p.0] = true;
                }
                Constraint::Horizontal(lid) if lid.0 < sm.sketch.lines.len() => {
                    let ln = &sm.sketch.lines[lid.0];
                    if ln.start.0 < n_pts {
                        y_constrained[ln.start.0] = true;
                    }
                    if ln.end.0 < n_pts {
                        y_constrained[ln.end.0] = true;
                    }
                }
                Constraint::Vertical(lid) if lid.0 < sm.sketch.lines.len() => {
                    let ln = &sm.sketch.lines[lid.0];
                    if ln.start.0 < n_pts {
                        x_constrained[ln.start.0] = true;
                    }
                    if ln.end.0 < n_pts {
                        x_constrained[ln.end.0] = true;
                    }
                }
                Constraint::Coincident(p0, p1) if p0.0 < n_pts && p1.0 < n_pts => {
                    x_constrained[p0.0] = true;
                    y_constrained[p0.0] = true;
                    x_constrained[p1.0] = true;
                    y_constrained[p1.0] = true;
                }
                Constraint::Distance(p0, p1, _)
                | Constraint::HorizontalDistance(p0, p1, _)
                | Constraint::VerticalDistance(p0, p1, _) => {
                    if p0.0 < n_pts {
                        x_constrained[p0.0] = true;
                    }
                    if p1.0 < n_pts {
                        x_constrained[p1.0] = true;
                    }
                }
                Constraint::Block(p, ..) if p.0 < n_pts => {
                    x_constrained[p.0] = true;
                    y_constrained[p.0] = true;
                }
                _ => {}
            }
        }
        for (i, pt) in sm.sketch.points.iter().enumerate() {
            let xc = x_constrained[i];
            let yc = y_constrained[i];
            if xc && yc {
                continue;
            }
            if let Some(sp) = project(pt.position.x, pt.position.y) {
                if !xc {
                    // Show X-direction arrow
                    let tip = egui::pos2(sp.x + dof_arrow_len, sp.y);
                    painter.line_segment([sp, tip], egui::Stroke::new(1.5, dof_color));
                    painter.line_segment(
                        [tip, egui::pos2(tip.x - 3.0, tip.y - 2.0)],
                        egui::Stroke::new(1.5, dof_color),
                    );
                    painter.line_segment(
                        [tip, egui::pos2(tip.x - 3.0, tip.y + 2.0)],
                        egui::Stroke::new(1.5, dof_color),
                    );
                }
                if !yc {
                    // Show Y-direction arrow
                    let tip = egui::pos2(sp.x, sp.y - dof_arrow_len);
                    painter.line_segment([sp, tip], egui::Stroke::new(1.5, dof_color));
                    painter.line_segment(
                        [tip, egui::pos2(tip.x - 2.0, tip.y + 3.0)],
                        egui::Stroke::new(1.5, dof_color),
                    );
                    painter.line_segment(
                        [tip, egui::pos2(tip.x + 2.0, tip.y + 3.0)],
                        egui::Stroke::new(1.5, dof_color),
                    );
                }
            }
        }
    }

    // Draw points (FreeCAD-style: filled dot + ring on hover/select)
    for (i, pt) in sm.sketch.points.iter().enumerate() {
        if let Some(sp) = project(pt.position.x, pt.position.y) {
            let eref = SketchEntityRef::Point(i);
            let (color, _w) = entity_color(eref, point_color);
            let is_sel = sel.contains(&eref);
            let is_hov = hov == Some(eref);
            if is_sel {
                painter.circle_filled(sp, point_radius + 1.5, color);
                painter.circle_stroke(sp, point_radius + 4.0, egui::Stroke::new(1.5, color));
            } else if is_hov {
                painter.circle_filled(sp, point_radius + 1.0, color);
                painter.circle_stroke(sp, point_radius + 3.0, egui::Stroke::new(1.0, color));
            } else {
                painter.circle_filled(sp, point_radius, color);
            }
        }
    }

    // Draw drag handles for selected points.
    for entity in sel {
        let SketchEntityRef::Point(i) = *entity else {
            continue;
        };
        let Some(pt) = sm.sketch.points.get(i) else {
            continue;
        };
        let Some(sp) = project(pt.position.x, pt.position.y) else {
            continue;
        };
        let active = sm.drag_points.contains(&i);
        let hovered_handle = mouse_screen.is_some_and(|pointer| {
            super::overlays::sketch_drag_handle_hit(pointer, sp, POINT_HANDLE_RADIUS)
        });
        let handle_color = super::overlays::sketch_drag_handle_color(sm.solver_converged, active);
        painter.circle_filled(sp, POINT_HANDLE_RADIUS, handle_color.gamma_multiply(0.55));
        painter.circle_stroke(
            sp,
            if hovered_handle {
                POINT_HANDLE_RADIUS + 2.0
            } else {
                POINT_HANDLE_RADIUS
            },
            egui::Stroke::new(if hovered_handle { 2.4 } else { 2.0 }, handle_color),
        );
        painter.circle_filled(sp, 2.0, egui::Color32::WHITE);
    }

    // Draw lines
    for (i, line) in sm.sketch.lines.iter().enumerate() {
        let s = &sm.sketch.points[line.start.0];
        let e = &sm.sketch.points[line.end.0];
        if let (Some(sp), Some(ep)) = (
            project(s.position.x, s.position.y),
            project(e.position.x, e.position.y),
        ) {
            let (color, width) = entity_color(SketchEntityRef::Line(i), line_color);
            painter.line_segment([sp, ep], egui::Stroke::new(width, color));
        }
    }

    // Draw circles
    for (i, circle) in sm.sketch.circles.iter().enumerate() {
        let center = &sm.sketch.points[circle.center.0];
        if let Some(cp) = project(center.position.x, center.position.y) {
            if let Some(rp) = project(center.position.x + circle.radius, center.position.y) {
                let screen_r = cp.distance(rp);
                let (color, width) = entity_color(SketchEntityRef::Circle(i), line_color);
                painter.circle_stroke(cp, screen_r, egui::Stroke::new(width, color));
            }
        }
    }

    // Draw arcs (polyline approximation)
    for (i, arc) in sm.sketch.arcs.iter().enumerate() {
        let center = &sm.sketch.points[arc.center.0];
        let cx = center.position.x;
        let cy = center.position.y;
        let segments = 32;
        let angle_span = arc.end_angle - arc.start_angle;
        let mut pts = Vec::with_capacity(segments + 1);
        for i_seg in 0..=segments {
            let t = arc.start_angle + angle_span * (i_seg as f64 / segments as f64);
            let px = cx + arc.radius * t.cos();
            let py = cy + arc.radius * t.sin();
            if let Some(sp) = project(px, py) {
                pts.push(sp);
            }
        }
        if pts.len() >= 2 {
            let (color, width) = entity_color(SketchEntityRef::Arc(i), line_color);
            painter.add(egui::Shape::line(pts, egui::Stroke::new(width, color)));
        }
    }

    // Draw ellipses
    for (i, ell) in sm.sketch.ellipses.iter().enumerate() {
        if ell.center.0 < sm.sketch.points.len() && ell.major_end.0 < sm.sketch.points.len() {
            let center = &sm.sketch.points[ell.center.0];
            let major = &sm.sketch.points[ell.major_end.0];
            let cx = center.position.x;
            let cy = center.position.y;
            let dx = major.position.x - cx;
            let dy = major.position.y - cy;
            let semi_major = (dx * dx + dy * dy).sqrt();
            let angle = dy.atan2(dx);
            let semi_minor = ell.minor_radius;
            let segments = 48;
            let mut pts = Vec::with_capacity(segments + 1);
            let cos_a = angle.cos();
            let sin_a = angle.sin();
            for i_seg in 0..=segments {
                let t = 2.0 * std::f64::consts::PI * (i_seg as f64 / segments as f64);
                let ex = semi_major * t.cos();
                let ey = semi_minor * t.sin();
                let px = cx + ex * cos_a - ey * sin_a;
                let py = cy + ex * sin_a + ey * cos_a;
                if let Some(sp) = project(px, py) {
                    pts.push(sp);
                }
            }
            if pts.len() >= 2 {
                let (color, width) = entity_color(SketchEntityRef::Ellipse(i), line_color);
                painter.add(egui::Shape::line(pts, egui::Stroke::new(width, color)));
            }
        }
    }

    // Draw B-splines (smooth curve via de Boor + control polygon)
    let bspline_color = egui::Color32::from_rgb(240, 240, 240);
    let ctrl_poly_color = egui::Color32::from_rgba_premultiplied(100, 160, 240, 60);
    for (i_bsp, bsp) in sm.sketch.bsplines.iter().enumerate() {
        let is_sel = sel.contains(&SketchEntityRef::BSpline(i_bsp));
        let curve_color = if is_sel {
            selected_color
        } else {
            bspline_color
        };
        // Gather 2D control point coordinates
        let ctrl_2d: Vec<[f64; 2]> = bsp
            .control_points
            .iter()
            .filter_map(|&pid| {
                if pid.0 < sm.sketch.points.len() {
                    let pt = &sm.sketch.points[pid.0];
                    Some([pt.position.x, pt.position.y])
                } else {
                    None
                }
            })
            .collect();
        // Control polygon (dashed)
        let ctrl_screen: Vec<egui::Pos2> =
            ctrl_2d.iter().filter_map(|p| project(p[0], p[1])).collect();
        if ctrl_screen.len() >= 2 {
            for seg in ctrl_screen.windows(2) {
                draw_dashed_line(&painter, seg[0], seg[1], ctrl_poly_color, 1.0, 4.0, 3.0);
            }
        }
        // Control point diamonds
        for &sp in &ctrl_screen {
            let h = 3.5;
            let stroke = egui::Stroke::new(1.5, curve_color);
            painter.line_segment(
                [egui::pos2(sp.x, sp.y - h), egui::pos2(sp.x + h, sp.y)],
                stroke,
            );
            painter.line_segment(
                [egui::pos2(sp.x + h, sp.y), egui::pos2(sp.x, sp.y + h)],
                stroke,
            );
            painter.line_segment(
                [egui::pos2(sp.x, sp.y + h), egui::pos2(sp.x - h, sp.y)],
                stroke,
            );
            painter.line_segment(
                [egui::pos2(sp.x - h, sp.y), egui::pos2(sp.x, sp.y - h)],
                stroke,
            );
        }
        // Smooth curve via de Boor evaluation
        let n_ctrl = ctrl_2d.len();
        let deg = bsp.degree.min(n_ctrl.saturating_sub(1));
        if n_ctrl >= 2 && deg >= 1 {
            let knots = if bsp.knots.len() == n_ctrl + deg + 1 {
                bsp.knots.clone()
            } else {
                clamped_uniform_knots(n_ctrl, deg)
            };
            let n_samples = 4 * n_ctrl + 16;
            let t_min = knots[deg];
            let t_max = knots[n_ctrl];
            let curve_pts: Vec<egui::Pos2> = (0..=n_samples)
                .filter_map(|i| {
                    let t = t_min + (t_max - t_min) * (i as f64) / (n_samples as f64);
                    let p = de_boor_eval(&ctrl_2d, &knots, deg, t);
                    project(p[0], p[1])
                })
                .collect();
            if curve_pts.len() >= 2 {
                let width = if is_sel { 3.0 } else { 2.0 };
                painter.add(egui::Shape::line(
                    curve_pts,
                    egui::Stroke::new(width, curve_color),
                ));
            }
        }
    }

    // Draw closed profile highlights (translucent fill for extrudable regions)
    let has_closed_profile;
    {
        let loops = find_closed_loops(&sm.sketch);
        has_closed_profile = !loops.is_empty();
        let profile_fill = egui::Color32::from_rgba_premultiplied(50, 180, 100, 24);
        for lp in &loops {
            let screen_pts: Vec<egui::Pos2> = lp
                .iter()
                .filter_map(|&pid| {
                    if pid < sm.sketch.points.len() {
                        let pt = &sm.sketch.points[pid];
                        project(pt.position.x, pt.position.y)
                    } else {
                        None
                    }
                })
                .collect();
            if screen_pts.len() >= 3 {
                let shape =
                    egui::Shape::convex_polygon(screen_pts, profile_fill, egui::Stroke::NONE);
                painter.add(shape);
            }
        }
    }

    // Extrude direction preview arrow (when closed profile exists)
    if has_closed_profile {
        let plane = &sm.plane;
        let ext_dist = sm.extrude_distance as f32;
        // Centroid of all sketch points
        if !sm.sketch.points.is_empty() {
            let n = sm.sketch.points.len() as f64;
            let cx: f64 = sm.sketch.points.iter().map(|p| p.position.x).sum::<f64>() / n;
            let cy: f64 = sm.sketch.points.iter().map(|p| p.position.y).sum::<f64>() / n;
            // World-space base and tip
            let base_w = [
                (plane.origin.x + plane.x_axis.x * cx + plane.y_axis.x * cy) as f32,
                (plane.origin.y + plane.x_axis.y * cx + plane.y_axis.y * cy) as f32,
                (plane.origin.z + plane.x_axis.z * cx + plane.y_axis.z * cy) as f32,
            ];
            let tip_w = [
                base_w[0] + plane.normal.x as f32 * ext_dist,
                base_w[1] + plane.normal.y as f32 * ext_dist,
                base_w[2] + plane.normal.z as f32 * ext_dist,
            ];
            if let (Some(base_s), Some(tip_s)) = (
                world_to_screen(camera, viewport, base_w),
                world_to_screen(camera, viewport, tip_w),
            ) {
                let arrow_color = egui::Color32::from_rgb(100, 220, 140);
                // Shaft
                painter.line_segment([base_s, tip_s], egui::Stroke::new(2.0, arrow_color));
                // Arrowhead
                let dx = tip_s.x - base_s.x;
                let dy = tip_s.y - base_s.y;
                let len = (dx * dx + dy * dy).sqrt();
                if len > 10.0 {
                    let ux = dx / len;
                    let uy = dy / len;
                    let head_len = 10.0_f32;
                    let head_w = 5.0_f32;
                    let p1 = egui::pos2(
                        tip_s.x - ux * head_len + uy * head_w,
                        tip_s.y - uy * head_len - ux * head_w,
                    );
                    let p2 = egui::pos2(
                        tip_s.x - ux * head_len - uy * head_w,
                        tip_s.y - uy * head_len + ux * head_w,
                    );
                    painter.add(egui::Shape::convex_polygon(
                        vec![tip_s, p1, p2],
                        arrow_color,
                        egui::Stroke::NONE,
                    ));
                }
                // Distance label
                let mid = egui::pos2((base_s.x + tip_s.x) * 0.5 + 8.0, (base_s.y + tip_s.y) * 0.5);
                painter.text(
                    mid,
                    egui::Align2::LEFT_CENTER,
                    format!("{:.1} mm", sm.extrude_distance),
                    egui::FontId::proportional(11.0),
                    arrow_color,
                );
            }
        }
    }

    // Draw box selection rubber band (FreeCAD: blue=window, green=crossing)
    if let (Some((sx, sy)), Some((ex, ey))) = (sm.box_select_start, sm.box_select_end) {
        if let (Some(sp), Some(ep)) = (project(sx, sy), project(ex, ey)) {
            let window_mode = ex >= sx; // left→right = window, right→left = crossing
            let fill = if window_mode {
                egui::Color32::from_rgba_premultiplied(40, 100, 200, 20)
            } else {
                egui::Color32::from_rgba_premultiplied(40, 200, 100, 20)
            };
            let stroke_color = if window_mode {
                egui::Color32::from_rgb(60, 120, 220)
            } else {
                egui::Color32::from_rgb(60, 220, 120)
            };
            let rect = egui::Rect::from_two_pos(sp, ep);
            painter.rect_filled(rect, 0.0, fill);
            if window_mode {
                painter.rect_stroke(
                    rect,
                    0.0,
                    egui::Stroke::new(1.5, stroke_color),
                    egui::StrokeKind::Middle,
                );
            } else {
                // Crossing: dashed border
                let corners = [
                    rect.left_top(),
                    rect.right_top(),
                    rect.right_bottom(),
                    rect.left_bottom(),
                ];
                for i in 0..4 {
                    draw_dashed_line(
                        &painter,
                        corners[i],
                        corners[(i + 1) % 4],
                        stroke_color,
                        1.5,
                        6.0,
                        4.0,
                    );
                }
            }
        }
    }

    // Draw pending point (first click for line/rect)
    if let Some((px, py)) = sm.pending_point {
        if let Some(sp) = project(px, py) {
            painter.circle_filled(sp, point_radius + 2.0, pending_color);
        }
    }

    // -- Live preview (rubber-band from pending point to cursor) --
    let preview_color = egui::Color32::from_rgba_premultiplied(255, 200, 50, 180);
    let preview_stroke = egui::Stroke::new(1.5, preview_color);
    if let (Some((px, py)), Some((mx, my))) = (sm.pending_point, mouse_sketch) {
        match sm.tool {
            SketchTool::Line | SketchTool::Slot => {
                if let (Some(sp), Some(ep)) = (project(px, py), project(mx, my)) {
                    draw_dashed_line(&painter, sp, ep, preview_color, 1.5, 6.0, 4.0);
                }
            }
            SketchTool::Rectangle => {
                // 4-line preview rectangle
                let corners = [(px, py), (mx, py), (mx, my), (px, my)];
                for i in 0..4 {
                    let (ax, ay) = corners[i];
                    let (bx, by) = corners[(i + 1) % 4];
                    if let (Some(sp), Some(ep)) = (project(ax, ay), project(bx, by)) {
                        draw_dashed_line(&painter, sp, ep, preview_color, 1.5, 6.0, 4.0);
                    }
                }
            }
            SketchTool::Circle => {
                // Dashed circle preview
                let dx = mx - px;
                let dy = my - py;
                let radius = (dx * dx + dy * dy).sqrt();
                if let Some(cp) = project(px, py) {
                    let segments = 48;
                    let mut pts = Vec::with_capacity(segments + 1);
                    for i in 0..=segments {
                        let t = 2.0 * std::f64::consts::PI * (i as f64 / segments as f64);
                        if let Some(sp) = project(px + radius * t.cos(), py + radius * t.sin()) {
                            pts.push(sp);
                        }
                    }
                    if pts.len() >= 2 {
                        painter.add(egui::Shape::line(pts, preview_stroke));
                    }
                    // Radius line
                    if let Some(ep) = project(mx, my) {
                        draw_dashed_line(&painter, cp, ep, preview_color, 1.0, 4.0, 3.0);
                    }
                }
            }
            SketchTool::Arc => {
                // Arc preview: dashed line from center to cursor (radius indicator)
                if let (Some(sp), Some(ep)) = (project(px, py), project(mx, my)) {
                    draw_dashed_line(&painter, sp, ep, preview_color, 1.0, 4.0, 3.0);
                    // Preview arc (semicircle)
                    let dx = mx - px;
                    let dy = my - py;
                    let radius = (dx * dx + dy * dy).sqrt();
                    let segments = 32;
                    let mut pts = Vec::with_capacity(segments + 1);
                    for i in 0..=segments {
                        let t = std::f64::consts::PI * (i as f64 / segments as f64);
                        if let Some(sp) = project(px + radius * t.cos(), py + radius * t.sin()) {
                            pts.push(sp);
                        }
                    }
                    if pts.len() >= 2 {
                        painter.add(egui::Shape::line(pts, preview_stroke));
                    }
                }
            }
            SketchTool::Ellipse => {
                // Ellipse preview
                let rx = (mx - px).abs().max(0.1);
                let ry = (my - py).abs().max(0.1);
                let segments = 48;
                let mut pts = Vec::with_capacity(segments + 1);
                for i in 0..=segments {
                    let t = 2.0 * std::f64::consts::PI * (i as f64 / segments as f64);
                    if let Some(sp) = project(px + rx * t.cos(), py + ry * t.sin()) {
                        pts.push(sp);
                    }
                }
                if pts.len() >= 2 {
                    painter.add(egui::Shape::line(pts, preview_stroke));
                }
            }
            SketchTool::Polygon { sides } => {
                let dx = mx - px;
                let dy = my - py;
                let radius = (dx * dx + dy * dy).sqrt();
                let n = sides.max(3);
                let mut pts = Vec::with_capacity(n as usize + 1);
                for i in 0..=n {
                    let a = std::f64::consts::TAU * (i as f64 / n as f64);
                    if let Some(sp) = project(px + radius * a.cos(), py + radius * a.sin()) {
                        pts.push(sp);
                    }
                }
                if pts.len() >= 2 {
                    painter.add(egui::Shape::line(pts, preview_stroke));
                }
            }
            _ => {}
        }
    }
    // Polyline/BSpline: preview from last point to cursor
    if matches!(sm.tool, SketchTool::Polyline | SketchTool::BSpline) {
        if let Some(&(lx, ly)) = sm.polyline_points.last() {
            if let Some((mx, my)) = mouse_sketch {
                if let (Some(sp), Some(ep)) = (project(lx, ly), project(mx, my)) {
                    draw_dashed_line(&painter, sp, ep, preview_color, 1.5, 6.0, 4.0);
                }
            }
        }
    }

    // Draw polyline in-progress preview
    if matches!(sm.tool, SketchTool::Polyline) && !sm.polyline_points.is_empty() {
        let poly_color = egui::Color32::from_rgb(255, 200, 50);
        let pts: Vec<egui::Pos2> = sm
            .polyline_points
            .iter()
            .filter_map(|&(x, y)| project(x, y))
            .collect();
        if pts.len() >= 2 {
            painter.add(egui::Shape::line(pts, egui::Stroke::new(1.5, poly_color)));
        }
    }

    // Draw snap indicator (FreeCAD-style: orange crosshair + ring)
    if sm.snap_enabled {
        if let Some((px, py)) = sm.pending_point {
            if let Some(sp) = project(px, py) {
                let snap_color = egui::Color32::from_rgb(255, 160, 30);
                let half = 7.0;
                painter.circle_stroke(sp, half + 1.0, egui::Stroke::new(1.5, snap_color));
                painter.line_segment(
                    [egui::pos2(sp.x - half, sp.y), egui::pos2(sp.x + half, sp.y)],
                    egui::Stroke::new(1.5, snap_color),
                );
                painter.line_segment(
                    [egui::pos2(sp.x, sp.y - half), egui::pos2(sp.x, sp.y + half)],
                    egui::Stroke::new(1.5, snap_color),
                );
            }
        }
    }

    // Draw constraint indicators
    if sm.show_constraints {
        draw_constraint_indicators(&painter, sm, &project, constraint_color, &conflict_ids);
    }

    // -- Cursor crosshair on sketch plane (FreeCAD-style) --
    if !matches!(sm.tool, SketchTool::Select) {
        if let Some((cx, cy)) = mouse_sketch {
            if let Some(sp) = project(cx, cy) {
                let ch_inner = 4.0_f32;
                let ch_outer = 14.0_f32;
                let ch_color = egui::Color32::from_rgba_premultiplied(220, 220, 220, 160);
                let ch_stroke = egui::Stroke::new(1.0, ch_color);
                // Gap in center (cleaner crosshair)
                painter.line_segment(
                    [
                        egui::pos2(sp.x - ch_outer, sp.y),
                        egui::pos2(sp.x - ch_inner, sp.y),
                    ],
                    ch_stroke,
                );
                painter.line_segment(
                    [
                        egui::pos2(sp.x + ch_inner, sp.y),
                        egui::pos2(sp.x + ch_outer, sp.y),
                    ],
                    ch_stroke,
                );
                painter.line_segment(
                    [
                        egui::pos2(sp.x, sp.y - ch_outer),
                        egui::pos2(sp.x, sp.y - ch_inner),
                    ],
                    ch_stroke,
                );
                painter.line_segment(
                    [
                        egui::pos2(sp.x, sp.y + ch_inner),
                        egui::pos2(sp.x, sp.y + ch_outer),
                    ],
                    ch_stroke,
                );
            }
        }
    }

    // -- Dynamic dimension display while drawing --
    let dim_font = egui::FontId::proportional(11.0);
    draw_dynamic_dimensions(&painter, sm, &project, mouse_sketch, &dim_font);

    // -- Collect data needed by the OVP panel (before sm borrow ends) --
    let tool = sm.tool;
    let pending = sm.pending_point;
    let plane_snap = sm.plane;
    let poly_pts_last = sm.polyline_points.last().copied();
    let auto_constraints = mouse_sketch
        .map(|c| detect_auto_constraints(sm, c, pending))
        .unwrap_or_default();

    // -- Snap visual indicators near cursor (FreeCAD-style) --
    if !matches!(sm.tool, SketchTool::Select) {
        if let Some(mouse_screen_pos) = mouse_screen {
            let snap_green = egui::Color32::from_rgb(50, 220, 50);
            for ac in &auto_constraints {
                match *ac {
                    AutoConstraintKind::Coincident | AutoConstraintKind::OnPoint => {
                        // Circle + center dot (coincident snap)
                        let pos = mouse_screen_pos;
                        painter.circle_filled(pos, 3.0, snap_green);
                        painter.circle_stroke(pos, 7.0, egui::Stroke::new(2.0, snap_green));
                    }
                    AutoConstraintKind::Horizontal => {
                        // Horizontal guideline (subtle red dashed)
                        let pos = mouse_screen_pos;
                        let guide_color = egui::Color32::from_rgba_premultiplied(220, 60, 60, 80);
                        draw_dashed_line(
                            &painter,
                            egui::pos2(viewport.left(), pos.y),
                            egui::pos2(viewport.right(), pos.y),
                            guide_color,
                            0.8,
                            10.0,
                            6.0,
                        );
                        // H badge
                        painter.text(
                            egui::pos2(pos.x + 16.0, pos.y - 2.0),
                            egui::Align2::LEFT_CENTER,
                            "H",
                            egui::FontId::proportional(9.0),
                            egui::Color32::from_rgb(220, 100, 100),
                        );
                    }
                    AutoConstraintKind::Vertical => {
                        // Vertical guideline (subtle red dashed)
                        let pos = mouse_screen_pos;
                        let guide_color = egui::Color32::from_rgba_premultiplied(220, 60, 60, 80);
                        draw_dashed_line(
                            &painter,
                            egui::pos2(pos.x, viewport.top()),
                            egui::pos2(pos.x, viewport.bottom()),
                            guide_color,
                            0.8,
                            10.0,
                            6.0,
                        );
                        // V badge
                        painter.text(
                            egui::pos2(pos.x + 4.0, pos.y - 14.0),
                            egui::Align2::LEFT_CENTER,
                            "V",
                            egui::FontId::proportional(9.0),
                            egui::Color32::from_rgb(220, 100, 100),
                        );
                    }
                    AutoConstraintKind::Midpoint => {
                        // Diamond marker (filled) at midpoint
                        let pos = mouse_screen_pos;
                        let h = 5.0_f32;
                        painter.add(egui::Shape::convex_polygon(
                            vec![
                                egui::pos2(pos.x, pos.y - h),
                                egui::pos2(pos.x + h, pos.y),
                                egui::pos2(pos.x, pos.y + h),
                                egui::pos2(pos.x - h, pos.y),
                            ],
                            snap_green,
                            egui::Stroke::NONE,
                        ));
                    }
                    AutoConstraintKind::GridSnap => {
                        // Small cross at grid snap
                        let pos = mouse_screen_pos;
                        let h = 4.0_f32;
                        let gs_color = egui::Color32::from_rgb(180, 180, 200);
                        painter.line_segment(
                            [egui::pos2(pos.x - h, pos.y), egui::pos2(pos.x + h, pos.y)],
                            egui::Stroke::new(1.2, gs_color),
                        );
                        painter.line_segment(
                            [egui::pos2(pos.x, pos.y - h), egui::pos2(pos.x, pos.y + h)],
                            egui::Stroke::new(1.2, gs_color),
                        );
                    }
                    AutoConstraintKind::Intersection => {
                        // X cross + circle (intersection snap)
                        let pos = mouse_screen_pos;
                        let h = 6.0_f32;
                        let int_color = egui::Color32::from_rgb(255, 180, 40);
                        painter.line_segment(
                            [
                                egui::pos2(pos.x - h, pos.y - h),
                                egui::pos2(pos.x + h, pos.y + h),
                            ],
                            egui::Stroke::new(2.0, int_color),
                        );
                        painter.line_segment(
                            [
                                egui::pos2(pos.x + h, pos.y - h),
                                egui::pos2(pos.x - h, pos.y + h),
                            ],
                            egui::Stroke::new(2.0, int_color),
                        );
                        painter.circle_stroke(pos, h + 2.0, egui::Stroke::new(1.5, int_color));
                    }
                }
            }
        }
    }

    // -- FreeCAD-style sketch banner with background pill --
    let plane_label = if sm.plane.normal.z.abs() > 0.9 {
        "XY"
    } else if sm.plane.normal.y.abs() > 0.9 {
        "XZ"
    } else {
        "YZ"
    };
    let tool_label = match sm.tool {
        SketchTool::Select => "Select",
        SketchTool::Line => "Line",
        SketchTool::Rectangle => "Rectangle",
        SketchTool::Circle => "Circle",
        SketchTool::Arc => "Arc",
        SketchTool::Point => "Point",
        SketchTool::Ellipse => "Ellipse",
        SketchTool::Polyline => "Polyline",
        SketchTool::Slot => "Slot",
        SketchTool::BSpline => "B-Spline",
        SketchTool::Polygon { .. } => "Polygon",
    };
    let toggles = format!(
        "{}{}{}",
        if sm.construction_mode { " [C]" } else { "" },
        if sm.show_grid { " [G]" } else { "" },
        if sm.snap_enabled { " [S]" } else { "" },
    );
    let pt_count = sm.sketch.points.len();
    let ln_count = sm.sketch.lines.len();
    let c_count = sm.sketch.constraints.len();
    let dof = sm.degrees_of_freedom();
    let n_violated = sm
        .constraint_residuals
        .iter()
        .filter(|r| **r > 1e-6)
        .count();
    let n_conflicts = conflict_ids.len();
    let dof_tag = if pt_count == 0 {
        String::new()
    } else if n_conflicts > 0 {
        format!(" \u{2022} Over-constrained: {n_conflicts} conflicts")
    } else if n_violated > 0 {
        format!(" \u{2022} {n_violated} conflicting")
    } else if dof == 0 {
        " \u{2022} Fully constrained".to_string()
    } else {
        format!(" \u{2022} DOF: {dof}")
    };
    let sel_tag = if sm.selected_entities.is_empty() {
        String::new()
    } else {
        format!(" \u{2022} Sel: {}", sm.selected_entities.len())
    };
    let constraint_blocked = sm.constraint_warning_count > 0;
    let constraint_tag = if constraint_blocked {
        if sm.constraint_warning_count == 1 {
            format!(" \u{2022} {}", sm.constraint_status)
        } else {
            format!(
                " \u{2022} {} (+{} more)",
                sm.constraint_status,
                sm.constraint_warning_count - 1
            )
        }
    } else {
        String::new()
    };
    let reference_tag = if sm.external_reference_count > 0 || sm.reused_geometry_count > 0 {
        format!(" \u{2022} {}", sm.reference_status)
    } else {
        String::new()
    };
    let profile_blocked = ln_count > 0 && !sm.profile_ready;
    let profile_tag = if sm.profile_ready {
        " \u{2022} Profile ready".to_string()
    } else if profile_blocked {
        format!(" \u{2022} {}", sm.profile_status)
    } else {
        String::new()
    };
    let banner = format!(
        "Sketcher ({plane_label})  \u{2502}  {tool_label}  \u{2502}  P:{pt_count} L:{ln_count} C:{c_count}{toggles}{dof_tag}{constraint_tag}{reference_tag}{sel_tag}{profile_tag}{}",
        if pt_count == 0 {
            "  \u{2502}  Click to add"
        } else {
            ""
        },
    );
    let (banner_bg, banner_fg) = if n_conflicts > 0 || n_violated > 0 {
        (
            egui::Color32::from_rgba_premultiplied(180, 40, 40, 180),
            egui::Color32::from_rgb(255, 200, 200),
        )
    } else if constraint_blocked || profile_blocked {
        (
            egui::Color32::from_rgba_premultiplied(140, 90, 30, 190),
            egui::Color32::from_rgb(255, 225, 170),
        )
    } else if dof == 0 && pt_count > 0 {
        (
            egui::Color32::from_rgba_premultiplied(30, 120, 60, 180),
            egui::Color32::from_rgb(180, 255, 200),
        )
    } else {
        (
            egui::Color32::from_rgba_premultiplied(40, 50, 70, 200),
            egui::Color32::from_rgb(220, 220, 240),
        )
    };
    let banner_pos = egui::pos2(viewport.center().x, viewport.top() + 80.0);
    let banner_font = egui::FontId::proportional(12.0);
    let banner_galley = painter.layout_no_wrap(banner.clone(), banner_font.clone(), banner_fg);
    let banner_rect =
        egui::Rect::from_center_size(banner_pos, banner_galley.size() + egui::vec2(16.0, 6.0));
    painter.rect_filled(banner_rect, 4.0, banner_bg);
    painter.text(
        banner_pos,
        egui::Align2::CENTER_CENTER,
        banner,
        banner_font,
        banner_fg,
    );

    // -- Validation warnings in overlay --
    {
        use cadkernel_sketch::SketchValidationIssue;
        let warn_font = egui::FontId::proportional(10.0);
        let warn_color = egui::Color32::from_rgb(255, 180, 50);
        let mut top_warning_row = 0.0_f32;
        for issue in &sm.validation_issues {
            match issue {
                SketchValidationIssue::ZeroLengthLine { line_index } => {
                    let li = *line_index;
                    if li < sm.sketch.lines.len() {
                        let line = &sm.sketch.lines[li];
                        if line.start.0 < sm.sketch.points.len()
                            && line.end.0 < sm.sketch.points.len()
                        {
                            let sp = &sm.sketch.points[line.start.0];
                            let ep = &sm.sketch.points[line.end.0];
                            let mx = (sp.position.x + ep.position.x) * 0.5;
                            let my = (sp.position.y + ep.position.y) * 0.5;
                            if let Some(scr) = project(mx, my) {
                                painter.text(
                                    scr,
                                    egui::Align2::CENTER_CENTER,
                                    "\u{26A0} zero-len",
                                    warn_font.clone(),
                                    warn_color,
                                );
                            }
                        }
                    }
                }
                SketchValidationIssue::NearlyCoincidentPoints {
                    point_a, point_b, ..
                } if *point_a < sm.sketch.points.len() && *point_b < sm.sketch.points.len() => {
                    let pa = &sm.sketch.points[*point_a].position;
                    let pb = &sm.sketch.points[*point_b].position;
                    let mx = (pa.x + pb.x) * 0.5;
                    let my = (pa.y + pb.y) * 0.5;
                    if let Some(scr) = project(mx, my) {
                        painter.text(
                            scr + egui::vec2(0.0, 10.0),
                            egui::Align2::CENTER_TOP,
                            "\u{26A0} merge?",
                            warn_font.clone(),
                            warn_color,
                        );
                    }
                }
                SketchValidationIssue::OverConstrained { .. } => {
                    let oc_pos = egui::pos2(viewport.center().x, viewport.top() + 100.0);
                    let oc_text = if n_conflicts > 0 {
                        format!("\u{26A0} Over-constrained: {n_conflicts} conflicts")
                    } else {
                        "\u{26A0} Over-constrained".to_string()
                    };
                    let oc_font = warn_font.clone();
                    let oc_color = egui::Color32::from_rgb(255, 100, 100);
                    let oc_bg = egui::Color32::from_rgba_premultiplied(160, 30, 30, 180);
                    let oc_galley =
                        painter.layout_no_wrap(oc_text.clone(), oc_font.clone(), oc_color);
                    let oc_rect = egui::Rect::from_center_size(
                        oc_pos,
                        oc_galley.size() + egui::vec2(12.0, 4.0),
                    );
                    painter.rect_filled(oc_rect, 3.0, oc_bg);
                    painter.text(
                        oc_pos,
                        egui::Align2::CENTER_CENTER,
                        oc_text,
                        oc_font,
                        oc_color,
                    );
                }
                SketchValidationIssue::DuplicateConstraint { .. }
                | SketchValidationIssue::ConflictingConstraintValue { .. }
                | SketchValidationIssue::InvalidConstraintValue { .. } => {
                    let text = format!("\u{26A0} {}", issue.status_label());
                    let warn_pos = egui::pos2(
                        viewport.center().x,
                        viewport.top() + 118.0 + top_warning_row * 16.0,
                    );
                    top_warning_row += 1.0;
                    let warn_galley = painter.layout_no_wrap(
                        text.clone(),
                        warn_font.clone(),
                        egui::Color32::from_rgb(255, 220, 170),
                    );
                    let warn_rect = egui::Rect::from_center_size(
                        warn_pos,
                        warn_galley.size() + egui::vec2(12.0, 4.0),
                    );
                    painter.rect_filled(
                        warn_rect,
                        3.0,
                        egui::Color32::from_rgba_premultiplied(140, 80, 20, 180),
                    );
                    painter.text(
                        warn_pos,
                        egui::Align2::CENTER_CENTER,
                        text,
                        warn_font.clone(),
                        egui::Color32::from_rgb(255, 220, 170),
                    );
                }
                _ => {}
            }
        }
    }

    // -- On-View Parameters panel --
    if !matches!(tool, SketchTool::Select) {
        draw_ovp_panel(
            ctx,
            &plane_snap,
            tool,
            mouse_screen,
            mouse_sketch,
            pending,
            poly_pts_last,
            &auto_constraints,
        );
    }

    // -- Sketch cursor shape per tool --
    {
        let sm = gui.sketch_mode.as_ref().unwrap();
        let cursor = if !sm.drag_points.is_empty() {
            egui::CursorIcon::Grabbing
        } else if sm.box_select_start.is_some() {
            egui::CursorIcon::Crosshair
        } else {
            match sm.tool {
                SketchTool::Select => {
                    if sm.hovered_entity.is_some() {
                        egui::CursorIcon::PointingHand
                    } else {
                        egui::CursorIcon::Default
                    }
                }
                SketchTool::Line
                | SketchTool::Rectangle
                | SketchTool::Circle
                | SketchTool::Arc
                | SketchTool::Point
                | SketchTool::Ellipse
                | SketchTool::Polyline
                | SketchTool::Slot
                | SketchTool::BSpline
                | SketchTool::Polygon { .. } => egui::CursorIcon::Crosshair,
            }
        };
        ctx.set_cursor_icon(cursor);
    }

    // -- Dimension input popup --
    handle_dimension_label_double_click(ctx, gui, camera, viewport);
    draw_in_place_dimension_editor(ctx, gui);
    draw_dimension_popup(ctx, gui);

    // -- Right-click context menu --
    draw_sketch_context_menu(ctx, gui);
}

// ---------------------------------------------------------------------------
// Dimension editing
// ---------------------------------------------------------------------------
fn inline_dim_pos_id(dim_id: usize) -> egui::Id {
    egui::Id::new(("sketch_dimension_inline_pos", dim_id))
}

fn inline_dim_buffer_id(dim_id: usize) -> egui::Id {
    egui::Id::new(("sketch_dimension_inline_buffer", dim_id))
}

fn inline_dim_invalid_id(dim_id: usize) -> egui::Id {
    egui::Id::new(("sketch_dimension_inline_invalid", dim_id))
}

fn clear_inline_dimension_editor(ctx: &egui::Context, dim_id: usize) {
    ctx.data_mut(|data| {
        data.remove::<[f32; 2]>(inline_dim_pos_id(dim_id));
        data.remove::<String>(inline_dim_buffer_id(dim_id));
        data.remove::<String>(inline_dim_invalid_id(dim_id));
    });
}

fn inline_dimension_editor_active(ctx: &egui::Context, gui: &GuiState) -> bool {
    let Some(dim_id) = gui
        .dimension_popup
        .as_ref()
        .and_then(|popup| popup.edit_constraint_index)
    else {
        return false;
    };
    ctx.data_mut(|data| data.get_persisted::<[f32; 2]>(inline_dim_pos_id(dim_id)))
        .is_some()
}

fn handle_dimension_label_double_click(
    ctx: &egui::Context,
    gui: &mut GuiState,
    camera: &Camera,
    viewport: egui::Rect,
) {
    let double_clicked = ctx.input(|i| {
        i.pointer
            .button_double_clicked(egui::PointerButton::Primary)
    });
    if !double_clicked {
        return;
    }
    let Some(pointer_pos) = ctx.input(|i| i.pointer.interact_pos().or(i.pointer.hover_pos()))
    else {
        return;
    };

    let hit = {
        let Some(sm) = gui.sketch_mode.as_ref() else {
            return;
        };
        let project = |x: f64, y: f64| -> Option<egui::Pos2> {
            let wp = sm.plane.to_world(x, y);
            world_to_screen(camera, viewport, [wp.x as f32, wp.y as f32, wp.z as f32])
        };
        collect_dimension_hits(sm, &project)
            .into_iter()
            .filter_map(|hit| {
                let dist = hit.pos.distance(pointer_pos);
                (dist <= DIMENSION_HIT_RADIUS).then_some((hit, dist))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(hit, _)| hit)
    };

    if let Some(hit) = hit {
        gui.dimension_popup = Some(super::DimensionPopup {
            kind: hit.kind,
            value: hit.value,
            just_opened: true,
            edit_constraint_index: Some(hit.constraint_index),
        });
        ctx.data_mut(|data| {
            data.insert_persisted(
                inline_dim_pos_id(hit.constraint_index),
                [hit.pos.x, hit.pos.y],
            );
            data.insert_persisted(
                inline_dim_buffer_id(hit.constraint_index),
                format_dimension_edit_value(hit.value),
            );
            data.insert_persisted(inline_dim_invalid_id(hit.constraint_index), String::new());
        });
        let label = match hit.kind {
            DimensionKind::Distance => "Distance",
            DimensionKind::Radius => "Radius",
            DimensionKind::Angle => "Angle",
            DimensionKind::Length => "Length",
            DimensionKind::HDistance => "H-Distance",
            DimensionKind::VDistance => "V-Distance",
            DimensionKind::Diameter => "Diameter",
        };
        gui.status_message = format!("Editing {label} {:.2}", hit.value);
    }
}

fn draw_in_place_dimension_editor(ctx: &egui::Context, gui: &mut GuiState) {
    let Some((dim_id, popup_value)) = gui.dimension_popup.as_ref().and_then(|popup| {
        popup
            .edit_constraint_index
            .map(|dim_id| (dim_id, popup.value))
    }) else {
        return;
    };
    let Some(pos) = ctx.data_mut(|data| data.get_persisted::<[f32; 2]>(inline_dim_pos_id(dim_id)))
    else {
        return;
    };
    let popup_pos = egui::pos2(pos[0], pos[1]);
    let mut buffer = ctx
        .data_mut(|data| data.get_persisted::<String>(inline_dim_buffer_id(dim_id)))
        .unwrap_or_else(|| format_dimension_edit_value(popup_value));
    let invalid_message = ctx
        .data_mut(|data| data.get_persisted::<String>(inline_dim_invalid_id(dim_id)))
        .filter(|message| !message.is_empty());

    let mut commit = false;
    let mut cancel = false;
    let mut changed_buffer = false;

    egui::Area::new(egui::Id::new(("sketch_dimension_inline_editor", dim_id)))
        .order(egui::Order::Foreground)
        .fixed_pos(popup_pos + egui::vec2(-46.0, -20.0))
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_rgba_unmultiplied(22, 26, 34, 245))
                .stroke(egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgb(80, 150, 220),
                ))
                .corner_radius(4.0)
                .inner_margin(egui::Margin::symmetric(6, 4))
                .show(ui, |ui| {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut buffer)
                            .desired_width(78.0)
                            .font(egui::TextStyle::Monospace),
                    );
                    response.request_focus();
                    changed_buffer = response.changed();
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        commit = true;
                    }
                    if let Some(message) = invalid_message.as_deref() {
                        ui.label(
                            egui::RichText::new(message)
                                .size(10.0)
                                .color(egui::Color32::from_rgb(255, 150, 120)),
                        );
                    }
                });
        });

    if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
        commit = true;
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        cancel = true;
    }

    if cancel {
        clear_inline_dimension_editor(ctx, dim_id);
        gui.dimension_popup = None;
        gui.status_message = "Dimension edit cancelled".into();
        return;
    }

    if changed_buffer {
        ctx.data_mut(|data| {
            data.insert_persisted(inline_dim_buffer_id(dim_id), buffer.clone());
            data.insert_persisted(inline_dim_invalid_id(dim_id), String::new());
        });
    }

    if commit {
        let parsed = buffer.trim().parse::<f64>();
        match parsed {
            Ok(value) => {
                if let Some(sm) = &mut gui.sketch_mode {
                    if !value.is_finite() || value <= 0.0 {
                        ctx.data_mut(|data| {
                            data.insert_persisted(
                                inline_dim_invalid_id(dim_id),
                                "Enter a number > 0".to_string(),
                            );
                        });
                        return;
                    }
                    if let Err(err) = sm.sketch.dimension_constraint_value(dim_id) {
                        ctx.data_mut(|data| {
                            data.insert_persisted(inline_dim_invalid_id(dim_id), err.to_string());
                        });
                        return;
                    }
                    sm.save_snapshot();
                    match sm.sketch.update_dimension_constraint(dim_id, value) {
                        Ok(updated) => {
                            sm.update_constraint_status();
                            clear_inline_dimension_editor(ctx, dim_id);
                            gui.dimension_popup = None;
                            gui.status_message =
                                format!("Dimension updated to {:.2}", updated.value);
                        }
                        Err(err) => {
                            ctx.data_mut(|data| {
                                data.insert_persisted(
                                    inline_dim_invalid_id(dim_id),
                                    err.to_string(),
                                );
                            });
                        }
                    }
                }
            }
            Err(_) => {
                ctx.data_mut(|data| {
                    data.insert_persisted(
                        inline_dim_invalid_id(dim_id),
                        "Enter a number > 0".to_string(),
                    );
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Dimension input popup
// ---------------------------------------------------------------------------
fn draw_dimension_popup(ctx: &egui::Context, gui: &mut GuiState) {
    use super::GuiAction;

    if inline_dimension_editor_active(ctx, gui) {
        return;
    }

    let Some(popup) = &mut gui.dimension_popup else {
        return;
    };

    let label = match popup.kind {
        DimensionKind::Distance => "Distance",
        DimensionKind::Radius => "Radius",
        DimensionKind::Angle => "Angle (°)",
        DimensionKind::Length => "Length",
        DimensionKind::HDistance => "H-Distance",
        DimensionKind::VDistance => "V-Distance",
        DimensionKind::Diameter => "Diameter",
    };

    let kind = popup.kind;
    let speed = if matches!(kind, DimensionKind::Angle) {
        1.0
    } else {
        0.1
    };
    let just_opened = popup.just_opened;

    let mut confirmed = false;
    let mut cancelled = false;

    let suffix = if matches!(kind, DimensionKind::Angle) {
        "\u{00B0}"
    } else {
        " mm"
    };

    egui::Window::new(format!("\u{1F4CF} {label}"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .fixed_size([220.0, 0.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("{label}:")).size(12.0));
                let p = gui.dimension_popup.as_mut().unwrap();
                let resp = ui.add(
                    egui::DragValue::new(&mut p.value)
                        .speed(speed)
                        .range(0.0..=f64::MAX)
                        .suffix(suffix),
                );
                if just_opened {
                    resp.request_focus();
                    gui.dimension_popup.as_mut().unwrap().just_opened = false;
                }
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    confirmed = true;
                }
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("\u{2714} OK")
                                .strong()
                                .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(0, 100, 180))
                        .min_size(egui::vec2(60.0, 24.0)),
                    )
                    .clicked()
                {
                    confirmed = true;
                }
                if ui
                    .add(egui::Button::new("\u{2716} Cancel").min_size(egui::vec2(60.0, 24.0)))
                    .clicked()
                {
                    cancelled = true;
                }
            });
        });

    // Handle Escape to cancel
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        cancelled = true;
    }

    if confirmed {
        let popup = gui.dimension_popup.take().unwrap();
        let v = popup.value;
        if let Some(ci) = popup.edit_constraint_index {
            // Edit existing constraint in-place
            if let Some(sm) = &mut gui.sketch_mode {
                if ci < sm.sketch.constraints.len() {
                    if v <= 0.0 || !v.is_finite() {
                        gui.status_message = "Dimension value must be > 0".into();
                        return;
                    }
                    sm.save_snapshot();
                    match sm.sketch.update_dimension_constraint(ci, v) {
                        Ok(updated) => {
                            sm.update_constraint_status();
                            gui.status_message =
                                format!("Constraint updated to {:.2}", updated.value);
                        }
                        Err(err) => {
                            let _ = sm.undo_stack.pop();
                            gui.status_message = format!("Dimension edit failed: {err}");
                        }
                    }
                }
            }
        } else {
            // New constraint
            use super::SketcherAction as S;
            match popup.kind {
                DimensionKind::Length => {
                    gui.constraint_length_value = v;
                    gui.actions.push(GuiAction::Sketcher(S::ConstrainLength(v)));
                }
                DimensionKind::Distance => {
                    gui.constraint_distance_value = v;
                    gui.actions
                        .push(GuiAction::Sketcher(S::ConstrainDistance(v)));
                }
                DimensionKind::Angle => {
                    gui.constraint_angle_value = v;
                    gui.actions.push(GuiAction::Sketcher(S::ConstrainAngle(v)));
                }
                DimensionKind::Radius => {
                    gui.constraint_radius_value = v;
                    gui.actions.push(GuiAction::Sketcher(S::ConstrainRadius(v)));
                }
                DimensionKind::Diameter => {
                    gui.constraint_radius_value = v / 2.0;
                    gui.actions
                        .push(GuiAction::Sketcher(S::ConstrainDiameter(v)));
                }
                DimensionKind::HDistance => {
                    gui.constraint_distance_value = v;
                    gui.actions
                        .push(GuiAction::Sketcher(S::ConstrainHDistance(v)));
                }
                DimensionKind::VDistance => {
                    gui.constraint_distance_value = v;
                    gui.actions
                        .push(GuiAction::Sketcher(S::ConstrainVDistance(v)));
                }
            }
        }
    } else if cancelled {
        gui.dimension_popup = None;
    }
}

// ---------------------------------------------------------------------------
// Sketch right-click context menu
// ---------------------------------------------------------------------------
fn draw_sketch_context_menu(ctx: &egui::Context, gui: &mut GuiState) {
    let sm = match &gui.sketch_mode {
        Some(sm) => sm,
        None => return,
    };
    if !sm.show_context_menu {
        return;
    }
    let has_sel = !sm.selected_entities.is_empty();
    let has_lines = sm
        .selected_entities
        .iter()
        .any(|e| matches!(e, SketchEntityRef::Line(_)));

    let mut close = false;
    let mut action: Option<SketchCtxAction> = None;
    egui::Area::new(egui::Id::new("sketch_ctx_menu"))
        .order(egui::Order::Foreground)
        .current_pos(ctx.input(|i| i.pointer.hover_pos().unwrap_or(egui::pos2(200.0, 200.0))))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(160.0);
                if has_sel {
                    ui.label(
                        egui::RichText::new("Edit")
                            .size(10.0)
                            .color(super::theme::MENU_SECTION_COLOR)
                            .strong(),
                    );
                    if ui.button("\u{1F5D1} Delete  (Del)").clicked() {
                        action = Some(SketchCtxAction::Delete);
                        close = true;
                    }
                    ui.separator();
                    if has_lines {
                        ui.label(
                            egui::RichText::new("Constraints")
                                .size(10.0)
                                .color(super::theme::MENU_SECTION_COLOR)
                                .strong(),
                        );
                        if ui.button("\u{2014} Horizontal  (H)").clicked() {
                            action = Some(SketchCtxAction::Horizontal);
                            close = true;
                        }
                        if ui.button("| Vertical  (V)").clicked() {
                            action = Some(SketchCtxAction::Vertical);
                            close = true;
                        }
                    }
                    if ui.button("\u{1F4CC} Fixed").clicked() {
                        action = Some(SketchCtxAction::Fixed);
                        close = true;
                    }
                    ui.separator();
                }
                ui.label(
                    egui::RichText::new("Selection")
                        .size(10.0)
                        .color(super::theme::MENU_SECTION_COLOR)
                        .strong(),
                );
                if ui.button("\u{2610} Select All  (Ctrl+A)").clicked() {
                    action = Some(SketchCtxAction::SelectAll);
                    close = true;
                }
                if has_sel && ui.button("Clear Selection").clicked() {
                    action = Some(SketchCtxAction::ClearSelection);
                    close = true;
                }
                ui.separator();
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });

    // Close on click outside
    if ctx.input(|i| i.pointer.any_click()) {
        close = true;
    }

    if close {
        if let Some(sm) = &mut gui.sketch_mode {
            sm.show_context_menu = false;
        }
    }
    if let Some(act) = action {
        apply_sketch_ctx_action(gui, act);
    }
}

enum SketchCtxAction {
    Delete,
    Horizontal,
    Vertical,
    Fixed,
    SelectAll,
    ClearSelection,
}

fn apply_sketch_ctx_action(gui: &mut GuiState, action: SketchCtxAction) {
    let sm = match &mut gui.sketch_mode {
        Some(sm) => sm,
        None => return,
    };
    match action {
        SketchCtxAction::Delete => {
            if !sm.selected_entities.is_empty() {
                sm.save_snapshot();
                crate::app::CadApp::delete_sketch_entities(sm);
                gui.status_message = "Sketch entities deleted".into();
            }
        }
        SketchCtxAction::Horizontal => {
            gui.actions.push(super::GuiAction::Sketcher(
                super::SketcherAction::ConstrainHorizontal,
            ));
        }
        SketchCtxAction::Vertical => {
            gui.actions.push(super::GuiAction::Sketcher(
                super::SketcherAction::ConstrainVertical,
            ));
        }
        SketchCtxAction::Fixed => {
            gui.actions.push(super::GuiAction::Sketcher(
                super::SketcherAction::ConstrainFixed,
            ));
        }
        SketchCtxAction::SelectAll => {
            let n_pts = sm.sketch.points.len();
            let n_lines = sm.sketch.lines.len();
            let n_arcs = sm.sketch.arcs.len();
            let n_circles = sm.sketch.circles.len();
            let n_ellipses = sm.sketch.ellipses.len();
            let n_bsplines = sm.sketch.bsplines.len();
            sm.selected_entities.clear();
            for i in 0..n_pts {
                sm.selected_entities.push(SketchEntityRef::Point(i));
            }
            for i in 0..n_lines {
                sm.selected_entities.push(SketchEntityRef::Line(i));
            }
            for i in 0..n_arcs {
                sm.selected_entities.push(SketchEntityRef::Arc(i));
            }
            for i in 0..n_circles {
                sm.selected_entities.push(SketchEntityRef::Circle(i));
            }
            for i in 0..n_ellipses {
                sm.selected_entities.push(SketchEntityRef::Ellipse(i));
            }
            for i in 0..n_bsplines {
                sm.selected_entities.push(SketchEntityRef::BSpline(i));
            }
            gui.status_message = format!("Selected {} entities", sm.selected_entities.len());
        }
        SketchCtxAction::ClearSelection => {
            sm.selected_entities.clear();
            gui.status_message = "Selection cleared".into();
        }
    }
}

// ---------------------------------------------------------------------------
// Dynamic dimension display while drawing
// ---------------------------------------------------------------------------
fn draw_dynamic_dimensions(
    painter: &egui::Painter,
    sm: &super::SketchMode,
    project: &dyn Fn(f64, f64) -> Option<egui::Pos2>,
    mouse_sketch: Option<(f64, f64)>,
    font: &egui::FontId,
) {
    let Some((mx, my)) = mouse_sketch else { return };
    let dim_color = egui::Color32::from_rgba_premultiplied(200, 200, 120, 200);

    match sm.tool {
        SketchTool::Line | SketchTool::Polyline => {
            let start = if matches!(sm.tool, SketchTool::Polyline) && !sm.polyline_points.is_empty()
            {
                sm.polyline_points.last().copied()
            } else {
                sm.pending_point
            };
            if let Some((sx, sy)) = start {
                let (Some(sp), Some(ep)) = (project(sx, sy), project(mx, my)) else {
                    return;
                };
                let dx = mx - sx;
                let dy = my - sy;
                let len = (dx * dx + dy * dy).sqrt();
                let angle_deg = dy.atan2(dx).to_degrees();

                draw_dynamic_dim_line(painter, sp, ep, 12.0, &format!("{len:.2}"), font, dim_color);

                if len > 0.5 {
                    let arc_r = 18.0_f32;
                    let segs = 16;
                    let start_rad = 0.0_f64;
                    let end_rad = angle_deg.to_radians();
                    let mut arc_pts = Vec::with_capacity(segs + 1);
                    for i in 0..=segs {
                        let t = start_rad + (end_rad - start_rad) * (i as f64 / segs as f64);
                        let ax = sp.x + arc_r * t.cos() as f32;
                        let ay = sp.y - arc_r * t.sin() as f32;
                        arc_pts.push(egui::pos2(ax, ay));
                    }
                    if arc_pts.len() >= 2 {
                        let arc_color = egui::Color32::from_rgba_premultiplied(180, 180, 120, 150);
                        painter.add(egui::Shape::line(
                            arc_pts,
                            egui::Stroke::new(0.8, arc_color),
                        ));
                    }
                    let label_t = (start_rad + end_rad) * 0.5;
                    let label_r = arc_r + 10.0;
                    let label_pos = egui::pos2(
                        sp.x + label_r * label_t.cos() as f32,
                        sp.y - label_r * label_t.sin() as f32,
                    );
                    painter.text(
                        label_pos,
                        egui::Align2::CENTER_CENTER,
                        format!("{angle_deg:.1}\u{00B0}"),
                        font.clone(),
                        dim_color,
                    );
                }
            }
        }
        SketchTool::Rectangle => {
            if let Some((sx, sy)) = sm.pending_point {
                let w = mx - sx;
                let h = my - sy;
                let (Some(p00), Some(p10), Some(p01), Some(p11)) = (
                    project(sx, sy),
                    project(mx, sy),
                    project(sx, my),
                    project(mx, my),
                ) else {
                    return;
                };
                draw_dynamic_dim_line(
                    painter,
                    p00,
                    p10,
                    -14.0,
                    &format!("{:.2}", w.abs()),
                    font,
                    dim_color,
                );
                draw_dynamic_dim_line(
                    painter,
                    p00,
                    p01,
                    -14.0,
                    &format!("{:.2}", h.abs()),
                    font,
                    dim_color,
                );
                let preview_color = egui::Color32::from_rgba_premultiplied(200, 200, 80, 120);
                let stroke = egui::Stroke::new(1.0, preview_color);
                painter.line_segment([p00, p10], stroke);
                painter.line_segment([p10, p11], stroke);
                painter.line_segment([p11, p01], stroke);
                painter.line_segment([p01, p00], stroke);
            }
        }
        SketchTool::Circle => {
            if let Some((cx, cy)) = sm.pending_point {
                let dx = mx - cx;
                let dy = my - cy;
                let radius = (dx * dx + dy * dy).sqrt();
                let (Some(cp), Some(ep)) = (project(cx, cy), project(mx, my)) else {
                    return;
                };
                draw_dynamic_dim_line(
                    painter,
                    cp,
                    ep,
                    12.0,
                    &format!("R {radius:.2}"),
                    font,
                    dim_color,
                );
                if let Some(rp) = project(cx + radius, cy) {
                    let screen_r = cp.distance(rp);
                    let preview_stroke = egui::Stroke::new(
                        1.0,
                        egui::Color32::from_rgba_premultiplied(200, 200, 80, 120),
                    );
                    painter.circle_stroke(cp, screen_r, preview_stroke);
                }
            }
        }
        SketchTool::Arc => {
            if let Some((cx, cy)) = sm.pending_point {
                let dx = mx - cx;
                let dy = my - cy;
                let radius = (dx * dx + dy * dy).sqrt();
                let angle_deg = dy.atan2(dx).to_degrees();
                let (Some(cp), Some(ep)) = (project(cx, cy), project(mx, my)) else {
                    return;
                };
                draw_dynamic_dim_line(
                    painter,
                    cp,
                    ep,
                    12.0,
                    &format!("R {radius:.2}"),
                    font,
                    dim_color,
                );
                let label_pos = egui::pos2((cp.x + ep.x) * 0.5, (cp.y + ep.y) * 0.5 - 18.0);
                painter.text(
                    label_pos,
                    egui::Align2::CENTER_CENTER,
                    format!("{angle_deg:.1}\u{00B0}"),
                    font.clone(),
                    dim_color,
                );
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Dynamic dimension line (lightweight, for in-progress drawing)
// ---------------------------------------------------------------------------
fn draw_dynamic_dim_line(
    painter: &egui::Painter,
    a: egui::Pos2,
    b: egui::Pos2,
    offset: f32,
    label: &str,
    font: &egui::FontId,
    color: egui::Color32,
) {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 2.0 {
        return;
    }
    let nx = -dy / len;
    let ny = dx / len;
    let off = egui::vec2(nx * offset, ny * offset);
    let sa = a + off;
    let sb = b + off;
    let stroke = egui::Stroke::new(0.8, color);
    painter.line_segment([a, sa + off * 0.15], stroke);
    painter.line_segment([b, sb + off * 0.15], stroke);
    painter.line_segment([sa, sb], stroke);
    let dir_ab = egui::vec2(dx / len, dy / len);
    draw_arrowhead(painter, sa, dir_ab, color);
    draw_arrowhead(painter, sb, -dir_ab, color);
    let mid = egui::pos2((sa.x + sb.x) * 0.5, (sa.y + sb.y) * 0.5);
    painter.text(
        mid + egui::vec2(0.0, -9.0),
        egui::Align2::CENTER_BOTTOM,
        label,
        font.clone(),
        color,
    );
}

// ---------------------------------------------------------------------------
// OVP row helper
// ---------------------------------------------------------------------------
fn ovp_row(
    ui: &mut egui::Ui,
    lbl: &str,
    value: &str,
    unit: &str,
    lc: egui::Color32,
    vc: egui::Color32,
    uc: egui::Color32,
) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(lbl).color(lc).size(11.0));
        ui.label(egui::RichText::new(value).color(vc).size(11.0).monospace());
        if !unit.is_empty() {
            ui.label(egui::RichText::new(unit).color(uc).size(10.0));
        }
    });
}

// ---------------------------------------------------------------------------
// On-View Parameters (OVP) panel
// ---------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
fn draw_ovp_panel(
    ctx: &egui::Context,
    _plane: &cadkernel_sketch::WorkPlane,
    tool: SketchTool,
    mouse_screen: Option<egui::Pos2>,
    mouse_sketch: Option<(f64, f64)>,
    pending: Option<(f64, f64)>,
    poly_last: Option<(f64, f64)>,
    auto_constraints: &[AutoConstraintKind],
) {
    let Some(screen_pos) = mouse_screen else {
        return;
    };
    let Some((mx, my)) = mouse_sketch else { return };

    let start_pt = match tool {
        SketchTool::Polyline => poly_last.or(pending),
        _ => pending,
    };
    let (seg_len, seg_angle_deg) = if let Some((sx, sy)) = start_pt {
        let dx = mx - sx;
        let dy = my - sy;
        ((dx * dx + dy * dy).sqrt(), dy.atan2(dx).to_degrees())
    } else {
        (0.0, 0.0)
    };
    let (rect_w, rect_h) = if let Some((sx, sy)) = pending {
        ((mx - sx).abs(), (my - sy).abs())
    } else {
        (0.0, 0.0)
    };
    let circle_r = if let Some((cx, cy)) = pending {
        let dx = mx - cx;
        let dy = my - cy;
        (dx * dx + dy * dy).sqrt()
    } else {
        0.0
    };

    let panel_pos = screen_pos + egui::vec2(22.0, 22.0);

    let tool_icon = match tool {
        SketchTool::Line | SketchTool::Polyline => "\u{2571}",
        SketchTool::Rectangle => "\u{25AD}",
        SketchTool::Circle => "\u{25CB}",
        SketchTool::Arc => "\u{25DC}",
        SketchTool::Point => "\u{2022}",
        SketchTool::Ellipse => "\u{2B2D}",
        SketchTool::Slot => "\u{2B2D}",
        SketchTool::BSpline => "\u{223F}",
        SketchTool::Polygon { .. } => "\u{2B23}",
        _ => "",
    };
    let tool_name = match tool {
        SketchTool::Line => "Line",
        SketchTool::Rectangle => "Rectangle",
        SketchTool::Circle => "Circle",
        SketchTool::Arc => "Arc",
        SketchTool::Point => "Point",
        SketchTool::Ellipse => "Ellipse",
        SketchTool::Polyline => "Polyline",
        SketchTool::Slot => "Slot",
        SketchTool::BSpline => "B-Spline",
        SketchTool::Polygon { .. } => "Polygon",
        _ => "",
    };

    egui::Area::new(egui::Id::new("sketch_ovp"))
        .fixed_pos(panel_pos)
        .order(egui::Order::Tooltip)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_rgba_unmultiplied(24, 28, 38, 235))
                .stroke(egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgba_unmultiplied(60, 80, 120, 160),
                ))
                .corner_radius(5.0)
                .inner_margin(egui::Margin::symmetric(8, 5))
                .show(ui, |ui| {
                    ui.set_min_width(130.0);
                    // Tool header
                    if !tool_name.is_empty() {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(tool_icon)
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(100, 180, 255)),
                            );
                            ui.label(
                                egui::RichText::new(tool_name)
                                    .size(10.5)
                                    .color(egui::Color32::from_rgb(140, 170, 210))
                                    .strong(),
                            );
                        });
                        ui.add_space(2.0);
                    }
                    let lc = egui::Color32::from_rgb(140, 155, 180);
                    let vc = egui::Color32::from_rgb(220, 225, 240);
                    let uc = egui::Color32::from_rgb(100, 110, 130);

                    ovp_row(ui, "X:", &format!("{mx:.2}"), "mm", lc, vc, uc);
                    ovp_row(ui, "Y:", &format!("{my:.2}"), "mm", lc, vc, uc);

                    match tool {
                        SketchTool::Line | SketchTool::Polyline
                            if (pending.is_some() || poly_last.is_some()) =>
                        {
                            ui.separator();
                            ovp_row(ui, "L:", &format!("{seg_len:.2}"), "mm", lc, vc, uc);
                            ovp_row(
                                ui,
                                "A:",
                                &format!("{seg_angle_deg:.1}\u{00B0}"),
                                "",
                                lc,
                                vc,
                                uc,
                            );
                        }
                        SketchTool::Rectangle if pending.is_some() => {
                            ui.separator();
                            ovp_row(ui, "W:", &format!("{rect_w:.2}"), "mm", lc, vc, uc);
                            ovp_row(ui, "H:", &format!("{rect_h:.2}"), "mm", lc, vc, uc);
                        }
                        SketchTool::Circle if pending.is_some() => {
                            ui.separator();
                            ovp_row(ui, "R:", &format!("{circle_r:.2}"), "mm", lc, vc, uc);
                        }
                        SketchTool::Arc if pending.is_some() => {
                            ui.separator();
                            ovp_row(ui, "R:", &format!("{circle_r:.2}"), "mm", lc, vc, uc);
                            ovp_row(
                                ui,
                                "A:",
                                &format!("{seg_angle_deg:.1}\u{00B0}"),
                                "",
                                lc,
                                vc,
                                uc,
                            );
                        }
                        SketchTool::Polygon { sides } => {
                            if pending.is_some() {
                                ui.separator();
                                ovp_row(ui, "R:", &format!("{circle_r:.2}"), "mm", lc, vc, uc);
                            }
                            ovp_row(ui, "N:", &format!("{sides}"), "", lc, vc, uc);
                        }
                        SketchTool::Point => {}
                        _ => {}
                    }

                    if !auto_constraints.is_empty() {
                        ui.separator();
                        for ac in auto_constraints {
                            let ac_color = egui::Color32::from_rgb(80, 200, 80);
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(ac.symbol()).color(ac_color).size(12.0),
                                );
                                ui.label(
                                    egui::RichText::new(ac.label()).color(ac_color).size(11.0),
                                );
                            });
                        }
                    }
                });
        });
}

// ---------------------------------------------------------------------------
// Constraint visualization colors
// ---------------------------------------------------------------------------
const GEO_COLOR: egui::Color32 = egui::Color32::from_rgb(220, 60, 60); // FreeCAD red constraints
const CONFLICT_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 40, 40);
const ERR_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 100, 40); // orange-red violated
const WARN_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 200, 50); // yellow warning

struct ConstraintCtx<'a> {
    painter: &'a egui::Painter,
    sm: &'a super::SketchMode,
    project: &'a dyn Fn(f64, f64) -> Option<egui::Pos2>,
    conflict_ids: &'a [cadkernel_sketch::ConstraintId],
    sym_font: egui::FontId,
    dim_font: egui::FontId,
}

fn draw_constraint_indicators(
    painter: &egui::Painter,
    sm: &super::SketchMode,
    project: &dyn Fn(f64, f64) -> Option<egui::Pos2>,
    _color: egui::Color32,
    conflict_ids: &[cadkernel_sketch::ConstraintId],
) {
    let cx = ConstraintCtx {
        painter,
        sm,
        project,
        conflict_ids,
        sym_font: egui::FontId::proportional(10.0),
        dim_font: egui::FontId::proportional(11.0),
    };
    for (ci, c) in sm.sketch.constraints.iter().enumerate() {
        let color_override = cx.constraint_color(ci);
        match c {
            Constraint::Distance(p0, p1, d) => cx.draw_distance_c(*p0, *p1, *d, color_override),
            Constraint::Length(lid, val) => cx.draw_length_c(*lid, *val, color_override),
            Constraint::Radius(center, edge, r) => {
                cx.draw_radius(*center, *edge, *r, color_override)
            }
            Constraint::Diameter(center, edge, d) => {
                cx.draw_diameter(*center, *edge, *d, color_override)
            }
            Constraint::Angle(l0, l1, a) => cx.draw_angle(*l0, *l1, *a, color_override),
            Constraint::HorizontalDistance(p0, p1, d) => {
                cx.draw_h_distance(*p0, *p1, *d, color_override)
            }
            Constraint::VerticalDistance(p0, p1, d) => {
                cx.draw_v_distance(*p0, *p1, *d, color_override)
            }
            Constraint::Horizontal(lid) => cx.draw_geo_line_sym_c(*lid, "H", color_override),
            Constraint::Vertical(lid) => cx.draw_geo_line_sym_c(*lid, "V", color_override),
            Constraint::Parallel(l0, l1) => {
                cx.draw_geo_line_sym_c(*l0, "\u{2225}", color_override);
                cx.draw_geo_line_sym_c(*l1, "\u{2225}", color_override);
            }
            Constraint::Perpendicular(l0, l1) => cx.draw_perp(*l0, *l1, color_override),
            Constraint::Coincident(p0, p1) => cx.draw_coincident_c(*p0, *p1, color_override),
            Constraint::Tangent(lid, center, _r) => cx.draw_tangent(*lid, *center, color_override),
            Constraint::EqualLength(l0, l1) => {
                cx.draw_geo_line_sym_c(*l0, "=", color_override);
                cx.draw_geo_line_sym_c(*l1, "=", color_override);
            }
            Constraint::EqualRadius(c0, _, c1, _) => {
                cx.draw_geo_point_sym_c(*c0, "=R", color_override);
                cx.draw_geo_point_sym_c(*c1, "=R", color_override);
            }
            Constraint::Symmetric(p0, p1, lid) => cx.draw_symmetric(*p0, *p1, *lid, color_override),
            Constraint::Fixed(pid, _, _) => cx.draw_fixed_c(*pid, color_override),
            Constraint::Block(pid, _, _) => {
                cx.draw_geo_point_sym_c(*pid, "\u{229E}", color_override)
            }
            Constraint::Midpoint(pid, lid) => cx.draw_midpoint(*pid, *lid, color_override),
            Constraint::Collinear(l0, l1) => cx.draw_collinear(*l0, *l1, color_override),
            Constraint::Concentric(p0, p1) => cx.draw_concentric(*p0, *p1, color_override),
            Constraint::PointOnLine(pid, _) => {
                cx.draw_geo_point_sym_c(*pid, "\u{00D7}", color_override)
            }
            Constraint::PointOnCircle(pid, _, _) => {
                cx.draw_geo_point_sym_c(*pid, "\u{00D7}", color_override)
            }
            Constraint::PointOnObject(pid, _) => {
                cx.draw_geo_point_sym_c(*pid, "\u{00D7}", color_override)
            }
            Constraint::Refraction { line1, .. } => {
                cx.draw_geo_line_sym_c(*line1, "Ref", color_override)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Drawing helpers
// ---------------------------------------------------------------------------

fn draw_arrowhead(painter: &egui::Painter, tip: egui::Pos2, dir: egui::Vec2, color: egui::Color32) {
    let len = 6.0_f32;
    let half_w = 2.5_f32;
    let d = dir.normalized();
    let perp = egui::vec2(-d.y, d.x);
    let base = tip - d * len;
    let left = base + perp * half_w;
    let right = base - perp * half_w;
    painter.add(egui::Shape::convex_polygon(
        vec![tip, left, right],
        color,
        egui::Stroke::NONE,
    ));
}

fn draw_dim_line_c(
    painter: &egui::Painter,
    a: egui::Pos2,
    b: egui::Pos2,
    offset: f32,
    label: &str,
    font: &egui::FontId,
    color: egui::Color32,
) {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 2.0 {
        return;
    }
    let nx = -dy / len;
    let ny = dx / len;
    let off = egui::vec2(nx * offset, ny * offset);
    let sa = a + off;
    let sb = b + off;
    let stroke = egui::Stroke::new(1.0, color);
    painter.line_segment([a, sa + off * 0.2], stroke);
    painter.line_segment([b, sb + off * 0.2], stroke);
    painter.line_segment([sa, sb], stroke);
    let dir_ab = egui::vec2(dx / len, dy / len);
    draw_arrowhead(painter, sa, dir_ab, color);
    draw_arrowhead(painter, sb, -dir_ab, color);
    let mid = egui::pos2((sa.x + sb.x) * 0.5, (sa.y + sb.y) * 0.5);
    painter.text(
        mid + egui::vec2(0.0, -9.0),
        egui::Align2::CENTER_BOTTOM,
        label,
        font.clone(),
        color,
    );
}

impl ConstraintCtx<'_> {
    fn pt_pos(&self, pid: cadkernel_sketch::PointId) -> Option<(f64, f64)> {
        if pid.0 < self.sm.sketch.points.len() {
            let p = &self.sm.sketch.points[pid.0];
            Some((p.position.x, p.position.y))
        } else {
            None
        }
    }

    fn line_ends(&self, lid: cadkernel_sketch::LineId) -> Option<((f64, f64), (f64, f64))> {
        if lid.0 < self.sm.sketch.lines.len() {
            let line = &self.sm.sketch.lines[lid.0];
            let s = &self.sm.sketch.points[line.start.0];
            let e = &self.sm.sketch.points[line.end.0];
            Some(((s.position.x, s.position.y), (e.position.x, e.position.y)))
        } else {
            None
        }
    }

    fn proj(&self, x: f64, y: f64) -> Option<egui::Pos2> {
        (self.project)(x, y)
    }

    /// Color for constraint index: green=satisfied, red=violated, yellow=warning.
    fn constraint_color(&self, idx: usize) -> egui::Color32 {
        if self.conflict_ids.contains(&idx) {
            return CONFLICT_COLOR;
        }
        if let Some(&res) = self.sm.constraint_residuals.get(idx) {
            if res < 1e-6 {
                GEO_COLOR
            } else if res < 0.1 {
                WARN_COLOR
            } else {
                ERR_COLOR
            }
        } else {
            GEO_COLOR
        }
    }

    // --- Color-override variants for residual feedback ---

    fn draw_distance_c(
        &self,
        p0: cadkernel_sketch::PointId,
        p1: cadkernel_sketch::PointId,
        d: f64,
        color: egui::Color32,
    ) {
        let (Some(a), Some(b)) = (self.pt_pos(p0), self.pt_pos(p1)) else {
            return;
        };
        let (Some(sa), Some(sb)) = (self.proj(a.0, a.1), self.proj(b.0, b.1)) else {
            return;
        };
        draw_dim_line_c(
            self.painter,
            sa,
            sb,
            14.0,
            &format!("{d:.1}"),
            &self.dim_font,
            color,
        );
    }

    fn draw_length_c(&self, lid: cadkernel_sketch::LineId, val: f64, color: egui::Color32) {
        let Some((s, e)) = self.line_ends(lid) else {
            return;
        };
        let (Some(sa), Some(sb)) = (self.proj(s.0, s.1), self.proj(e.0, e.1)) else {
            return;
        };
        draw_dim_line_c(
            self.painter,
            sa,
            sb,
            14.0,
            &format!("{val:.1}"),
            &self.dim_font,
            color,
        );
    }

    fn draw_geo_line_sym_c(&self, lid: cadkernel_sketch::LineId, sym: &str, color: egui::Color32) {
        let Some((s, e)) = self.line_ends(lid) else {
            return;
        };
        let mx = (s.0 + e.0) * 0.5;
        let my = (s.1 + e.1) * 0.5;
        if let Some(mp) = self.proj(mx, my) {
            let dx = e.0 - s.0;
            let dy = e.1 - s.1;
            let len = (dx * dx + dy * dy).sqrt();
            let off = if len > 1e-9 {
                egui::vec2((-dy / len) as f32 * 12.0, (dx / len) as f32 * 12.0)
            } else {
                egui::vec2(0.0, -12.0)
            };
            self.painter.text(
                mp + off,
                egui::Align2::CENTER_CENTER,
                sym,
                self.sym_font.clone(),
                color,
            );
        }
    }

    fn draw_geo_point_sym_c(
        &self,
        pid: cadkernel_sketch::PointId,
        sym: &str,
        color: egui::Color32,
    ) {
        if let Some(p) = self.pt_pos(pid) {
            if let Some(sp) = self.proj(p.0, p.1) {
                self.painter.text(
                    sp + egui::vec2(8.0, -8.0),
                    egui::Align2::LEFT_CENTER,
                    sym,
                    self.sym_font.clone(),
                    color,
                );
            }
        }
    }

    fn draw_coincident_c(
        &self,
        p0: cadkernel_sketch::PointId,
        _p1: cadkernel_sketch::PointId,
        color: egui::Color32,
    ) {
        if let Some(p) = self.pt_pos(p0) {
            if let Some(sp) = self.proj(p.0, p.1) {
                self.painter.circle_filled(sp, 6.0, color);
            }
        }
    }

    fn draw_fixed_c(&self, pid: cadkernel_sketch::PointId, color: egui::Color32) {
        if let Some(p) = self.pt_pos(pid) {
            if let Some(sp) = self.proj(p.0, p.1) {
                let h = 5.0_f32;
                let stroke = egui::Stroke::new(1.5, color);
                self.painter.rect_stroke(
                    egui::Rect::from_center_size(sp, egui::vec2(h * 2.0, h * 2.0)),
                    0.0,
                    stroke,
                    egui::StrokeKind::Middle,
                );
            }
        }
    }

    fn draw_radius(
        &self,
        center: cadkernel_sketch::PointId,
        edge: cadkernel_sketch::PointId,
        r: f64,
        color: egui::Color32,
    ) {
        let (Some(cp), Some(ep)) = (self.pt_pos(center), self.pt_pos(edge)) else {
            return;
        };
        let (Some(sc), Some(se)) = (self.proj(cp.0, cp.1), self.proj(ep.0, ep.1)) else {
            return;
        };
        let stroke = egui::Stroke::new(1.0, color);
        self.painter.line_segment([sc, se], stroke);
        let dir = (se - sc).normalized();
        draw_arrowhead(self.painter, se, dir, color);
        let mid = egui::pos2((sc.x + se.x) * 0.5, (sc.y + se.y) * 0.5);
        self.painter.text(
            mid + egui::vec2(0.0, -9.0),
            egui::Align2::CENTER_BOTTOM,
            format!("R {r:.1}"),
            self.dim_font.clone(),
            color,
        );
    }

    fn draw_diameter(
        &self,
        center: cadkernel_sketch::PointId,
        edge: cadkernel_sketch::PointId,
        d: f64,
        color: egui::Color32,
    ) {
        let (Some(cp), Some(ep)) = (self.pt_pos(center), self.pt_pos(edge)) else {
            return;
        };
        let dx = ep.0 - cp.0;
        let dy = ep.1 - cp.1;
        let opp = (cp.0 - dx, cp.1 - dy);
        let (Some(se), Some(so)) = (self.proj(ep.0, ep.1), self.proj(opp.0, opp.1)) else {
            return;
        };
        let stroke = egui::Stroke::new(1.0, color);
        self.painter.line_segment([so, se], stroke);
        let dir = (se - so).normalized();
        draw_arrowhead(self.painter, se, dir, color);
        draw_arrowhead(self.painter, so, -dir, color);
        let mid = egui::pos2((so.x + se.x) * 0.5, (so.y + se.y) * 0.5);
        self.painter.text(
            mid + egui::vec2(0.0, -9.0),
            egui::Align2::CENTER_BOTTOM,
            format!("\u{2300} {d:.1}"),
            self.dim_font.clone(),
            color,
        );
    }

    fn draw_angle(
        &self,
        l0: cadkernel_sketch::LineId,
        l1: cadkernel_sketch::LineId,
        a: f64,
        color: egui::Color32,
    ) {
        let (Some((s0, e0)), Some((s1, e1))) = (self.line_ends(l0), self.line_ends(l1)) else {
            return;
        };
        let cx = s0.0;
        let cy = s0.1;
        let ang0 = (e0.1 - s0.1).atan2(e0.0 - s0.0);
        let ang1 = (e1.1 - s1.1).atan2(e1.0 - s1.0);
        let arc_r = 20.0_f64;
        let segs = 16;
        let span = a;
        let start = ang0;
        let mut pts = Vec::with_capacity(segs + 1);
        for i in 0..=segs {
            let t = start + span * (i as f64 / segs as f64);
            let px = cx + arc_r * t.cos();
            let py = cy + arc_r * t.sin();
            if let Some(sp) = self.proj(px, py) {
                pts.push(sp);
            }
        }
        if pts.len() >= 2 {
            self.painter
                .add(egui::Shape::line(pts, egui::Stroke::new(1.0, color)));
        }
        let mid_t = start + span * 0.5;
        let label_r = arc_r * 1.4;
        if let Some(mp) = self.proj(cx + label_r * mid_t.cos(), cy + label_r * mid_t.sin()) {
            let _ = ang1;
            self.painter.text(
                mp,
                egui::Align2::CENTER_CENTER,
                format!("{:.0}\u{00B0}", a.to_degrees()),
                self.dim_font.clone(),
                color,
            );
        }
    }

    fn draw_h_distance(
        &self,
        p0: cadkernel_sketch::PointId,
        p1: cadkernel_sketch::PointId,
        d: f64,
        color: egui::Color32,
    ) {
        let (Some(a), Some(b)) = (self.pt_pos(p0), self.pt_pos(p1)) else {
            return;
        };
        let y = (a.1 + b.1) * 0.5;
        let (Some(sa), Some(sb)) = (self.proj(a.0, y), self.proj(b.0, y)) else {
            return;
        };
        let (Some(pa), Some(pb)) = (self.proj(a.0, a.1), self.proj(b.0, b.1)) else {
            return;
        };
        let stroke = egui::Stroke::new(1.0, color);
        self.painter.line_segment([pa, sa], stroke);
        self.painter.line_segment([pb, sb], stroke);
        self.painter.line_segment([sa, sb], stroke);
        let dir = (sb - sa).normalized();
        draw_arrowhead(self.painter, sa, dir, color);
        draw_arrowhead(self.painter, sb, -dir, color);
        let mid = egui::pos2((sa.x + sb.x) * 0.5, (sa.y + sb.y) * 0.5);
        self.painter.text(
            mid + egui::vec2(0.0, -9.0),
            egui::Align2::CENTER_BOTTOM,
            format!("{d:.1}"),
            self.dim_font.clone(),
            color,
        );
    }

    fn draw_v_distance(
        &self,
        p0: cadkernel_sketch::PointId,
        p1: cadkernel_sketch::PointId,
        d: f64,
        color: egui::Color32,
    ) {
        let (Some(a), Some(b)) = (self.pt_pos(p0), self.pt_pos(p1)) else {
            return;
        };
        let x = (a.0 + b.0) * 0.5;
        let (Some(sa), Some(sb)) = (self.proj(x, a.1), self.proj(x, b.1)) else {
            return;
        };
        let (Some(pa), Some(pb)) = (self.proj(a.0, a.1), self.proj(b.0, b.1)) else {
            return;
        };
        let stroke = egui::Stroke::new(1.0, color);
        self.painter.line_segment([pa, sa], stroke);
        self.painter.line_segment([pb, sb], stroke);
        self.painter.line_segment([sa, sb], stroke);
        let dir = (sb - sa).normalized();
        draw_arrowhead(self.painter, sa, dir, color);
        draw_arrowhead(self.painter, sb, -dir, color);
        let mid = egui::pos2((sa.x + sb.x) * 0.5, (sa.y + sb.y) * 0.5);
        self.painter.text(
            mid + egui::vec2(8.0, 0.0),
            egui::Align2::LEFT_CENTER,
            format!("{d:.1}"),
            self.dim_font.clone(),
            color,
        );
    }

    fn draw_perp(
        &self,
        l0: cadkernel_sketch::LineId,
        _l1: cadkernel_sketch::LineId,
        color: egui::Color32,
    ) {
        let Some((s0, e0)) = self.line_ends(l0) else {
            return;
        };
        let mx = (s0.0 + e0.0) * 0.5;
        let my = (s0.1 + e0.1) * 0.5;
        if let Some(mp) = self.proj(mx, my) {
            let h = 6.0_f32;
            let stroke = egui::Stroke::new(1.2, color);
            self.painter
                .line_segment([egui::pos2(mp.x, mp.y), egui::pos2(mp.x + h, mp.y)], stroke);
            self.painter.line_segment(
                [egui::pos2(mp.x + h, mp.y), egui::pos2(mp.x + h, mp.y - h)],
                stroke,
            );
        }
    }

    fn draw_tangent(
        &self,
        lid: cadkernel_sketch::LineId,
        _center: cadkernel_sketch::PointId,
        color: egui::Color32,
    ) {
        let Some((s, e)) = self.line_ends(lid) else {
            return;
        };
        let mx = (s.0 + e.0) * 0.5;
        let my = (s.1 + e.1) * 0.5;
        if let Some(mp) = self.proj(mx, my) {
            self.painter.text(
                mp + egui::vec2(0.0, -14.0),
                egui::Align2::CENTER_CENTER,
                "T",
                self.sym_font.clone(),
                color,
            );
        }
    }

    fn draw_midpoint(
        &self,
        pid: cadkernel_sketch::PointId,
        _lid: cadkernel_sketch::LineId,
        color: egui::Color32,
    ) {
        if let Some(p) = self.pt_pos(pid) {
            if let Some(sp) = self.proj(p.0, p.1) {
                let h = 5.0_f32;
                self.painter.add(egui::Shape::convex_polygon(
                    vec![
                        egui::pos2(sp.x, sp.y - h),
                        egui::pos2(sp.x + h, sp.y),
                        egui::pos2(sp.x, sp.y + h),
                        egui::pos2(sp.x - h, sp.y),
                    ],
                    color,
                    egui::Stroke::NONE,
                ));
            }
        }
    }

    fn draw_collinear(
        &self,
        l0: cadkernel_sketch::LineId,
        l1: cadkernel_sketch::LineId,
        color: egui::Color32,
    ) {
        let (Some((s0, e0)), Some((s1, e1))) = (self.line_ends(l0), self.line_ends(l1)) else {
            return;
        };
        let m0 = ((s0.0 + e0.0) * 0.5, (s0.1 + e0.1) * 0.5);
        let m1 = ((s1.0 + e1.0) * 0.5, (s1.1 + e1.1) * 0.5);
        if let (Some(sp0), Some(sp1)) = (self.proj(m0.0, m0.1), self.proj(m1.0, m1.1)) {
            draw_dashed_line(self.painter, sp0, sp1, color, 1.0, 4.0, 3.0);
        }
    }

    fn draw_concentric(
        &self,
        p0: cadkernel_sketch::PointId,
        _p1: cadkernel_sketch::PointId,
        color: egui::Color32,
    ) {
        if let Some(p) = self.pt_pos(p0) {
            if let Some(sp) = self.proj(p.0, p.1) {
                let stroke = egui::Stroke::new(1.2, color);
                self.painter.circle_stroke(sp, 4.0, stroke);
                self.painter.circle_stroke(sp, 7.0, stroke);
            }
        }
    }

    fn draw_symmetric(
        &self,
        p0: cadkernel_sketch::PointId,
        p1: cadkernel_sketch::PointId,
        _lid: cadkernel_sketch::LineId,
        color: egui::Color32,
    ) {
        let (Some(a), Some(b)) = (self.pt_pos(p0), self.pt_pos(p1)) else {
            return;
        };
        let mid = ((a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5);
        if let Some(sp) = self.proj(mid.0, mid.1) {
            self.painter.text(
                sp + egui::vec2(0.0, -10.0),
                egui::Align2::CENTER_CENTER,
                "S",
                self.sym_font.clone(),
                color,
            );
            if let (Some(sa), Some(sb)) = (self.proj(a.0, a.1), self.proj(b.0, b.1)) {
                draw_dashed_line(self.painter, sa, sb, color, 0.8, 3.0, 3.0);
            }
        }
    }
}

fn draw_dashed_line(
    painter: &egui::Painter,
    from: egui::Pos2,
    to: egui::Pos2,
    color: egui::Color32,
    width: f32,
    dash_len: f32,
    gap_len: f32,
) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let total = (dx * dx + dy * dy).sqrt();
    if total < 1.0 {
        return;
    }
    let nx = dx / total;
    let ny = dy / total;
    let step = dash_len + gap_len;
    let mut d = 0.0;
    while d < total {
        let end = (d + dash_len).min(total);
        painter.line_segment(
            [
                egui::pos2(from.x + nx * d, from.y + ny * d),
                egui::pos2(from.x + nx * end, from.y + ny * end),
            ],
            egui::Stroke::new(width, color),
        );
        d += step;
    }
}

// ---------------------------------------------------------------------------
// Closed loop detection for profile highlighting
// ---------------------------------------------------------------------------

/// Find closed loops of lines in a sketch. Returns list of loops,
/// each loop being a Vec of PointId indices in order.
fn find_closed_loops(sketch: &cadkernel_sketch::Sketch) -> Vec<Vec<usize>> {
    cadkernel_sketch::analyze_profiles(sketch)
        .loops
        .into_iter()
        .map(|lp| lp.into_iter().map(|pid| pid.0).collect())
        .collect()
}

// ---------------------------------------------------------------------------
// B-spline evaluation (de Boor's algorithm)
// ---------------------------------------------------------------------------

fn clamped_uniform_knots(n_ctrl: usize, degree: usize) -> Vec<f64> {
    let m = n_ctrl + degree + 1;
    let mut knots = vec![0.0; m];
    let interior = m - 2 * (degree + 1);
    for i in 0..=degree {
        knots[i] = 0.0;
        knots[m - 1 - i] = 1.0;
    }
    for j in 1..=interior {
        knots[degree + j] = j as f64 / (interior + 1) as f64;
    }
    knots
}

fn de_boor_eval(ctrl: &[[f64; 2]], knots: &[f64], p: usize, t: f64) -> [f64; 2] {
    let n = ctrl.len();
    // Find knot span
    let mut k = p;
    for i in p..n {
        if t < knots[i + 1] || i + 1 == n {
            k = i;
            break;
        }
    }
    // de Boor column
    let mut d: Vec<[f64; 2]> = (0..=p)
        .map(|j| {
            let idx = (k as isize - p as isize + j as isize).clamp(0, n as isize - 1) as usize;
            ctrl[idx]
        })
        .collect();
    for r in 1..=p {
        for j in (r..=p).rev() {
            let i = k as isize - p as isize + j as isize;
            let left = knots[i as usize];
            let right = knots[(i as usize) + p + 1 - r];
            let denom = right - left;
            let alpha = if denom.abs() < 1e-12 {
                0.0
            } else {
                (t - left) / denom
            };
            d[j][0] = (1.0 - alpha) * d[j - 1][0] + alpha * d[j][0];
            d[j][1] = (1.0 - alpha) * d[j - 1][1] + alpha * d[j][1];
        }
    }
    d[p]
}

fn world_to_screen(camera: &Camera, viewport: egui::Rect, p: [f32; 3]) -> Option<egui::Pos2> {
    let vp = camera.view_proj();
    let clip = [
        vp[0][0] * p[0] + vp[1][0] * p[1] + vp[2][0] * p[2] + vp[3][0],
        vp[0][1] * p[0] + vp[1][1] * p[1] + vp[2][1] * p[2] + vp[3][1],
        vp[0][2] * p[0] + vp[1][2] * p[1] + vp[2][2] * p[2] + vp[3][2],
        vp[0][3] * p[0] + vp[1][3] * p[1] + vp[2][3] * p[2] + vp[3][3],
    ];
    if clip[3].abs() < 1e-6 {
        return None;
    }
    let ndc_x = clip[0] / clip[3];
    let ndc_y = clip[1] / clip[3];
    let sx = viewport.left() + (ndc_x + 1.0) * 0.5 * viewport.width();
    let sy = viewport.top() + (1.0 - ndc_y) * 0.5 * viewport.height();
    Some(egui::pos2(sx, sy))
}

pub(crate) fn draw_grid_scale_label(ctx: &egui::Context, grid_config: &GridConfig) {
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("grid_scale_label"),
    ));
    let viewport = ctx.available_rect();
    let label = grid_config.scale_label();
    let pos = egui::pos2(viewport.center().x, viewport.bottom() - 12.0);
    painter.text(
        pos,
        egui::Align2::CENTER_CENTER,
        format!("Grid: {label}"),
        egui::FontId::proportional(11.0),
        egui::Color32::from_rgba_premultiplied(140, 140, 140, 180),
    );
}
