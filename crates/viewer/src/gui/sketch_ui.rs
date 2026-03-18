use super::{GuiState, SketchTool};
use crate::render::{Camera, GridConfig};
use cadkernel_sketch::Constraint;

pub(crate) fn draw_sketch_overlay(ctx: &egui::Context, gui: &mut GuiState, camera: &Camera) {
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

    let point_color = egui::Color32::from_rgb(50, 200, 50);
    let line_color = egui::Color32::from_rgb(80, 200, 80);
    let pending_color = egui::Color32::from_rgb(200, 200, 80);
    let constraint_color = egui::Color32::from_rgb(200, 100, 100);
    let point_radius = 4.0;

    // Draw points
    for pt in &sm.sketch.points {
        if let Some(sp) = project(pt.position.x, pt.position.y) {
            painter.circle_filled(sp, point_radius, point_color);
        }
    }

    // Draw lines
    for line in &sm.sketch.lines {
        let s = &sm.sketch.points[line.start.0];
        let e = &sm.sketch.points[line.end.0];
        if let (Some(sp), Some(ep)) = (
            project(s.position.x, s.position.y),
            project(e.position.x, e.position.y),
        ) {
            painter.line_segment([sp, ep], egui::Stroke::new(2.0, line_color));
        }
    }

    // Draw circles
    for circle in &sm.sketch.circles {
        let center = &sm.sketch.points[circle.center.0];
        if let Some(cp) = project(center.position.x, center.position.y) {
            if let Some(rp) =
                project(center.position.x + circle.radius, center.position.y)
            {
                let screen_r = cp.distance(rp);
                painter.circle_stroke(cp, screen_r, egui::Stroke::new(2.0, line_color));
            }
        }
    }

    // Draw arcs (polyline approximation)
    for arc in &sm.sketch.arcs {
        let center = &sm.sketch.points[arc.center.0];
        let cx = center.position.x;
        let cy = center.position.y;
        let segments = 32;
        let angle_span = arc.end_angle - arc.start_angle;
        let mut pts = Vec::with_capacity(segments + 1);
        for i in 0..=segments {
            let t = arc.start_angle + angle_span * (i as f64 / segments as f64);
            let px = cx + arc.radius * t.cos();
            let py = cy + arc.radius * t.sin();
            if let Some(sp) = project(px, py) {
                pts.push(sp);
            }
        }
        if pts.len() >= 2 {
            painter.add(egui::Shape::line(
                pts,
                egui::Stroke::new(2.0, line_color),
            ));
        }
    }

    // Draw pending point (first click for line/rect)
    if let Some((px, py)) = sm.pending_point {
        if let Some(sp) = project(px, py) {
            painter.circle_filled(sp, point_radius + 2.0, pending_color);
        }
    }

    // Draw constraint indicators
    for (ci, c) in sm.sketch.constraints.iter().enumerate() {
        let label = match c {
            Constraint::Horizontal(_) => "H",
            Constraint::Vertical(_) => "V",
            Constraint::Length(lid, len) => {
                let line = &sm.sketch.lines[lid.0];
                let s = &sm.sketch.points[line.start.0];
                let e = &sm.sketch.points[line.end.0];
                let mx = (s.position.x + e.position.x) / 2.0;
                let my = (s.position.y + e.position.y) / 2.0;
                if let Some(mp) = project(mx, my) {
                    let offset = egui::pos2(mp.x, mp.y - 14.0);
                    painter.text(
                        offset,
                        egui::Align2::CENTER_CENTER,
                        format!("{len:.1}"),
                        egui::FontId::proportional(11.0),
                        constraint_color,
                    );
                }
                continue;
            }
            Constraint::Fixed(..) => "Fix",
            Constraint::Parallel(..) => "//",
            Constraint::Perpendicular(..) => "\u{22A5}",
            Constraint::Coincident(..) => "\u{2261}",
            _ => continue,
        };
        match c {
            Constraint::Horizontal(lid) | Constraint::Vertical(lid) => {
                if lid.0 < sm.sketch.lines.len() {
                    let line = &sm.sketch.lines[lid.0];
                    let s = &sm.sketch.points[line.start.0];
                    let e = &sm.sketch.points[line.end.0];
                    let mx = (s.position.x + e.position.x) / 2.0;
                    let my = (s.position.y + e.position.y) / 2.0;
                    if let Some(mp) = project(mx, my) {
                        let offset =
                            egui::pos2(mp.x + 10.0, mp.y - 10.0 - (ci as f32 * 0.1));
                        painter.text(
                            offset,
                            egui::Align2::LEFT_CENTER,
                            label,
                            egui::FontId::proportional(11.0),
                            constraint_color,
                        );
                    }
                }
            }
            _ => {}
        }
    }

    // Sketch mode banner
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
    };
    let banner = format!(
        "SKETCH MODE ({} plane) | Tool: {} | Points: {} Lines: {} | Click to add",
        plane_label,
        tool_label,
        sm.sketch.points.len(),
        sm.sketch.lines.len()
    );
    painter.text(
        egui::pos2(viewport.center().x, viewport.top() + 80.0),
        egui::Align2::CENTER_CENTER,
        banner,
        egui::FontId::proportional(13.0),
        egui::Color32::from_rgb(255, 200, 50),
    );
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
