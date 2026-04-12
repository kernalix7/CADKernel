use super::{GizmoMode, GuiAction, GuiState, SelectionMode, SelectedEntity, SnapHighlight, ToastLevel};
use super::theme;
use crate::render::{Camera, cross3, dot3, normalize3};
use crate::scene::Scene;

pub(crate) fn draw_axes_overlay(ctx: &egui::Context, camera: &Camera, gui: &mut GuiState) {
    use crate::render::StandardView;

    let size = 45.0f32;
    let margin = 55.0f32;
    let center = egui::pos2(margin, ctx.screen_rect().bottom() - margin);

    let screen_right = camera.screen_right();
    let screen_up = camera.screen_up();
    let fwd = camera.forward();

    // Axis data: direction, color, label, positive-click view, negative-click view
    let axes: [([f32; 3], egui::Color32, &str, StandardView, StandardView); 3] = [
        ([1.0, 0.0, 0.0], egui::Color32::from_rgb(220, 60, 60), "X", StandardView::Right, StandardView::Left),
        ([0.0, 1.0, 0.0], egui::Color32::from_rgb(60, 200, 60), "Y", StandardView::Front, StandardView::Back),
        ([0.0, 0.0, 1.0], egui::Color32::from_rgb(70, 100, 240), "Z", StandardView::Top, StandardView::Bottom),
    ];

    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("axes_overlay"),
    ));

    // Background: subtle gradient ring
    painter.circle_filled(center, size + 10.0, egui::Color32::from_rgba_premultiplied(15, 15, 20, 100));
    painter.circle_filled(center, size + 6.0, egui::Color32::from_rgba_premultiplied(20, 22, 28, 140));
    painter.circle_stroke(center, size + 6.0, egui::Stroke::new(0.5, egui::Color32::from_rgba_premultiplied(100, 100, 120, 60)));

    // Center sphere (dark with highlight)
    painter.circle_filled(center, 4.0, egui::Color32::from_rgb(60, 62, 68));
    painter.circle_filled(egui::pos2(center.x - 1.0, center.y - 1.0), 1.5, egui::Color32::from_rgb(120, 125, 135));

    // Depth-sort axes (back-to-front)
    let mut depth_order: [(usize, f32); 3] = [
        (0, dot3(axes[0].0, fwd)),
        (1, dot3(axes[1].0, fwd)),
        (2, dot3(axes[2].0, fwd)),
    ];
    depth_order.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    // Hover detection
    let pointer_pos = ctx.input(|i| i.pointer.hover_pos());
    let click_radius = 14.0_f32;
    let mut hovered_axis: Option<(usize, bool)> = None; // (axis_idx, is_positive)

    // First pass: find hovered axis tip
    if let Some(pp) = pointer_pos {
        let mut best_dist = click_radius;
        for (idx, _) in &depth_order {
            let axis = axes[*idx].0;
            let sx = dot3(axis, screen_right) * size;
            let sy = -dot3(axis, screen_up) * size;
            let pos_end = egui::pos2(center.x + sx, center.y + sy);
            let neg_end = egui::pos2(center.x - sx * 0.35, center.y - sy * 0.35);

            let d_pos = pp.distance(pos_end);
            if d_pos < best_dist {
                best_dist = d_pos;
                hovered_axis = Some((*idx, true));
            }
            let d_neg = pp.distance(neg_end);
            if d_neg < best_dist {
                best_dist = d_neg;
                hovered_axis = Some((*idx, false));
            }
        }
    }

    // Handle click
    if let Some((idx, positive)) = hovered_axis {
        if ctx.input(|i| i.pointer.button_clicked(egui::PointerButton::Primary)) {
            let view = if positive { axes[idx].3 } else { axes[idx].4 };
            gui.actions.push(GuiAction::SetStandardView(view));
        }
        ctx.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
    }

    // Second pass: draw axes back-to-front
    for (idx, depth) in &depth_order {
        let (axis, color, label, _, _) = &axes[*idx];
        let sx = dot3(*axis, screen_right) * size;
        let sy = -dot3(*axis, screen_up) * size;
        let pos_end = egui::pos2(center.x + sx, center.y + sy);
        let neg_end = egui::pos2(center.x - sx * 0.35, center.y - sy * 0.35);

        // Depth-based opacity: axes pointing away are dimmer
        let facing = (1.0 - depth * 0.6).clamp(0.4, 1.0);
        let alpha = (255.0 * facing) as u8;
        let line_color = egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha);
        let faded_alpha = (80.0 * facing) as u8;
        let faded = egui::Color32::from_rgba_unmultiplied(
            color.r() / 3, color.g() / 3, color.b() / 3, faded_alpha,
        );

        // Negative axis stub (dashed feel — shorter, dimmer)
        painter.line_segment([center, neg_end], egui::Stroke::new(1.0, faded));

        // Positive axis — glow + core line for anti-aliased look
        let glow_color = egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), (40.0 * facing) as u8);
        painter.line_segment([center, pos_end], egui::Stroke::new(5.0, glow_color));
        painter.line_segment([center, pos_end], egui::Stroke::new(2.5, line_color));

        // Arrowhead (filled triangle)
        let dir_len = (sx * sx + sy * sy).sqrt().max(1e-6);
        let dx = sx / dir_len;
        let dy = sy / dir_len;
        let arrow_len = 9.0_f32;
        let arrow_half = 3.5_f32;
        let base = egui::pos2(pos_end.x - dx * arrow_len, pos_end.y - dy * arrow_len);
        let left = egui::pos2(base.x + dy * arrow_half, base.y - dx * arrow_half);
        let right_pt = egui::pos2(base.x - dy * arrow_half, base.y + dx * arrow_half);
        painter.add(egui::Shape::convex_polygon(
            vec![pos_end, left, right_pt],
            line_color,
            egui::Stroke::NONE,
        ));

        // Hover highlight: bright ring around tip
        let is_pos_hovered = matches!(hovered_axis, Some((hi, true)) if hi == *idx);
        let is_neg_hovered = matches!(hovered_axis, Some((hi, false)) if hi == *idx);

        if is_pos_hovered {
            painter.circle_filled(pos_end, 8.0, egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 40));
            painter.circle_stroke(pos_end, 8.0, egui::Stroke::new(1.5, *color));
        }
        if is_neg_hovered {
            painter.circle_filled(neg_end, 6.0, egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 40));
            painter.circle_stroke(neg_end, 6.0, egui::Stroke::new(1.0, faded));
        }

        // Label with shadow for readability
        let label_offset = 11.0;
        let label_pos = egui::pos2(pos_end.x + dx * label_offset, pos_end.y + dy * label_offset);
        let font = egui::FontId::proportional(if is_pos_hovered { 14.0 } else { 12.0 });

        // Shadow (offset dark text behind)
        painter.text(
            egui::pos2(label_pos.x + 1.0, label_pos.y + 1.0),
            egui::Align2::CENTER_CENTER,
            *label,
            font.clone(),
            egui::Color32::from_rgba_premultiplied(0, 0, 0, (180.0 * facing) as u8),
        );
        // Foreground label
        painter.text(
            label_pos,
            egui::Align2::CENTER_CENTER,
            *label,
            font,
            line_color,
        );
    }
}

pub(crate) fn draw_techdraw_overlay(ctx: &egui::Context, gui: &GuiState) {
    let sheet = match &gui.techdraw_sheet {
        Some(s) => s,
        None => return,
    };
    if sheet.views.is_empty() {
        return;
    }

    let screen = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("techdraw_overlay"),
    ));

    painter.text(
        egui::pos2(screen.center().x, screen.min.y + 30.0),
        egui::Align2::CENTER_CENTER,
        format!("TechDraw: {} view(s)", sheet.views.len()),
        egui::FontId::proportional(16.0),
        egui::Color32::from_rgb(0, 120, 200),
    );

    let (mut gmin_x, mut gmin_y) = (f64::MAX, f64::MAX);
    let (mut gmax_x, mut gmax_y) = (f64::MIN, f64::MIN);
    for view in &sheet.views {
        for e in &view.edges {
            gmin_x = gmin_x.min(e.x1).min(e.x2);
            gmin_y = gmin_y.min(e.y1).min(e.y2);
            gmax_x = gmax_x.max(e.x1).max(e.x2);
            gmax_y = gmax_y.max(e.y1).max(e.y2);
        }
    }
    if gmin_x >= gmax_x || gmin_y >= gmax_y {
        return;
    }

    let model_w = gmax_x - gmin_x;
    let model_h = gmax_y - gmin_y;
    let margin = 80.0_f32;
    let avail_w = screen.width() - 2.0 * margin;
    let avail_h = screen.height() - 2.0 * margin - 40.0;
    let scale = (avail_w as f64 / model_w).min(avail_h as f64 / model_h) * 0.85;
    let cx_model = (gmin_x + gmax_x) / 2.0;
    let cy_model = (gmin_y + gmax_y) / 2.0;
    let screen_cx = screen.center().x;
    let screen_cy = screen.center().y + 20.0;

    let visible_color = egui::Color32::from_rgb(0, 0, 0);
    let hidden_color = egui::Color32::from_rgb(160, 160, 160);

    painter.rect_filled(screen, 0.0, egui::Color32::from_rgba_premultiplied(240, 240, 240, 220));

    for view in &sheet.views {
        for e in &view.edges {
            let sx1 = screen_cx + ((e.x1 - cx_model) * scale) as f32;
            let sy1 = screen_cy - ((e.y1 - cy_model) * scale) as f32;
            let sx2 = screen_cx + ((e.x2 - cx_model) * scale) as f32;
            let sy2 = screen_cy - ((e.y2 - cy_model) * scale) as f32;

            let color = if e.visible { visible_color } else { hidden_color };
            let width = if e.visible { 1.5 } else { 0.8 };

            if e.visible {
                painter.line_segment(
                    [egui::pos2(sx1, sy1), egui::pos2(sx2, sy2)],
                    egui::Stroke::new(width, color),
                );
            } else {
                draw_dashed_line(
                    &painter,
                    egui::pos2(sx1, sy1),
                    egui::pos2(sx2, sy2),
                    4.0,
                    2.0,
                    egui::Stroke::new(width, color),
                );
            }
        }

        let label_x = screen_cx + ((view.center_x - cx_model) * scale) as f32;
        let label_y = screen_cy
            - ((view.center_y - cy_model) * scale) as f32
            + td_view_radius(view, scale) as f32
            + 16.0;
        painter.text(
            egui::pos2(label_x, label_y),
            egui::Align2::CENTER_TOP,
            view.direction.label(),
            egui::FontId::proportional(13.0),
            egui::Color32::from_rgb(0, 80, 160),
        );
    }
}

fn td_view_radius(view: &cadkernel_io::DrawingView, scale: f64) -> f64 {
    let mut max_r = 0.0_f64;
    for e in &view.edges {
        let dx = (e.x1 - view.center_x) * scale;
        let dy = (e.y1 - view.center_y) * scale;
        max_r = max_r.max((dx * dx + dy * dy).sqrt());
        let dx2 = (e.x2 - view.center_x) * scale;
        let dy2 = (e.y2 - view.center_y) * scale;
        max_r = max_r.max((dx2 * dx2 + dy2 * dy2).sqrt());
    }
    max_r
}

fn draw_dashed_line(
    painter: &egui::Painter,
    from: egui::Pos2,
    to: egui::Pos2,
    dash_len: f32,
    gap_len: f32,
    stroke: egui::Stroke,
) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let total = (dx * dx + dy * dy).sqrt();
    if total < 1e-6 {
        return;
    }
    let ux = dx / total;
    let uy = dy / total;
    let mut t = 0.0_f32;
    while t < total {
        let end = (t + dash_len).min(total);
        painter.line_segment(
            [
                egui::pos2(from.x + ux * t, from.y + uy * t),
                egui::pos2(from.x + ux * end, from.y + uy * end),
            ],
            stroke,
        );
        t = end + gap_len;
    }
}

// ---------------------------------------------------------------------------
// World-to-screen projection helper
// ---------------------------------------------------------------------------

/// Project a 3D world point to 2D screen coordinates using the camera's
/// view-projection matrix. Returns `None` if the point is behind the camera.
fn world_to_screen(camera: &Camera, screen: egui::Rect, p: [f32; 3]) -> Option<egui::Pos2> {
    let vp = camera.view_proj();
    // Homogeneous clip coords
    let cx = vp[0][0] * p[0] + vp[1][0] * p[1] + vp[2][0] * p[2] + vp[3][0];
    let cy = vp[0][1] * p[0] + vp[1][1] * p[1] + vp[2][1] * p[2] + vp[3][1];
    let cw = vp[0][3] * p[0] + vp[1][3] * p[1] + vp[2][3] * p[2] + vp[3][3];
    if cw.abs() < 1e-7 {
        return None;
    }
    let ndc_x = cx / cw;
    let ndc_y = cy / cw;
    // NDC [-1,1] -> screen
    let sx = screen.min.x + (ndc_x + 1.0) * 0.5 * screen.width();
    let sy = screen.min.y + (1.0 - ndc_y) * 0.5 * screen.height();
    Some(egui::pos2(sx, sy))
}


// ---------------------------------------------------------------------------
// 3D ground grid (XZ plane)
// ---------------------------------------------------------------------------

pub(crate) fn draw_grid_3d_overlay(
    ctx: &egui::Context,
    camera: &Camera,
    spacing: f32,
    clip_rect: egui::Rect,
) {
    let screen = ctx.screen_rect();
    let mut painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Background,
        egui::Id::new("grid_3d_overlay"),
    ));
    painter.set_clip_rect(clip_rect);

    let center_x = (camera.target[0] / spacing).round() * spacing;
    let center_y = (camera.target[1] / spacing).round() * spacing;

    let half_count = 15_i32;
    let major_every = 5;

    for i in -half_count..=half_count {
        let off = i as f32 * spacing;
        let is_major = i % major_every == 0;

        let (alpha, width) = if is_major {
            (50u8, 0.6f32)
        } else {
            (20u8, 0.4f32)
        };

        let dist = off.abs();
        let max_dist = half_count as f32 * spacing;
        let fade = ((1.0 - dist / max_dist) * 1.5).clamp(0.0, 1.0);
        let a = (alpha as f32 * fade) as u8;
        if a < 2 {
            continue;
        }

        let color = egui::Color32::from_rgba_premultiplied(160, 160, 165, a);
        let stroke = egui::Stroke::new(width, color);

        let x_start = [center_x - half_count as f32 * spacing, center_y + off, 0.0];
        let x_end = [center_x + half_count as f32 * spacing, center_y + off, 0.0];
        if let (Some(sa), Some(sb)) = (
            world_to_screen(camera, screen, x_start),
            world_to_screen(camera, screen, x_end),
        ) {
            painter.line_segment([sa, sb], stroke);
        }

        let y_start = [center_x + off, center_y - half_count as f32 * spacing, 0.0];
        let y_end = [center_x + off, center_y + half_count as f32 * spacing, 0.0];
        if let (Some(sa), Some(sb)) = (
            world_to_screen(camera, screen, y_start),
            world_to_screen(camera, screen, y_end),
        ) {
            painter.line_segment([sa, sb], stroke);
        }
    }
}

// ---------------------------------------------------------------------------
// Measurement overlay
// ---------------------------------------------------------------------------

pub(crate) fn draw_measurement_overlay(
    ctx: &egui::Context,
    camera: &Camera,
    gui: &GuiState,
    nav: &crate::nav::NavConfig,
) {
    if !gui.measurement_mode {
        return;
    }

    let screen = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("measurement_overlay"),
    ));

    let unit = nav.unit_system.label();
    let dp = nav.decimal_places as usize;
    let fmt = |v: f32| -> String {
        format!("{:.prec$}", v, prec = dp)
    };
    let accent = egui::Color32::from_rgb(255, 200, 50);
    let accent_dim = egui::Color32::from_rgba_premultiplied(255, 200, 50, 140);
    let text_color = egui::Color32::WHITE;
    let bg_color = egui::Color32::from_rgba_premultiplied(25, 25, 30, 220);
    let outline_color = egui::Color32::from_rgba_premultiplied(255, 200, 50, 80);

    // Mode indicator
    let pts = &gui.measurement_points;
    let mode_label = match pts.len() {
        0 => "Measure: click on surface to place point".to_string(),
        1 => "Measure: click 2nd point for distance | Esc to exit".to_string(),
        2 => "Measure: click 3rd for angle | C clear | Esc exit".to_string(),
        n => format!("Measure: {n} points | C clear | Esc exit"),
    };
    let mode_galley = painter.layout_no_wrap(
        mode_label,
        egui::FontId::proportional(11.0),
        accent_dim,
    );
    let mode_rect = egui::Rect::from_min_size(
        egui::pos2(
            screen.max.x - mode_galley.size().x - 16.0,
            screen.min.y + 40.0,
        ),
        egui::vec2(mode_galley.size().x + 12.0, mode_galley.size().y + 6.0),
    );
    painter.rect_filled(mode_rect, 4.0, bg_color);
    painter.rect_stroke(mode_rect, 4.0, egui::Stroke::new(0.5, outline_color), egui::StrokeKind::Middle);
    painter.galley(
        egui::pos2(
            screen.max.x - mode_galley.size().x - 10.0,
            screen.min.y + 43.0,
        ),
        mode_galley,
        accent_dim,
    );

    if pts.is_empty() {
        return;
    }

    // Draw numbered measurement points
    for (i, p) in pts.iter().enumerate() {
        if let Some(sp) = world_to_screen(camera, screen, *p) {
            // Outer glow ring
            painter.circle_stroke(sp, 8.0, egui::Stroke::new(1.0, outline_color));
            // Filled circle
            painter.circle_filled(sp, 5.0, accent);
            // Dark outline
            painter.circle_stroke(sp, 5.0, egui::Stroke::new(1.5, egui::Color32::BLACK));
            // Bright center dot
            painter.circle_filled(sp, 1.5, egui::Color32::WHITE);

            // Point number label
            let num_label = format!("P{}", i + 1);
            painter.text(
                egui::pos2(sp.x + 9.0, sp.y - 9.0),
                egui::Align2::LEFT_BOTTOM,
                &num_label,
                egui::FontId::proportional(10.0),
                accent,
            );

            // Coordinate tooltip near first point
            if i == 0 && pts.len() == 1 {
                let coord = format!(
                    "({}, {}, {}) {unit}",
                    fmt(p[0]), fmt(p[1]), fmt(p[2]),
                );
                draw_label_with_bg(
                    &painter, &coord,
                    egui::pos2(sp.x, sp.y + 16.0),
                    egui::FontId::proportional(10.0),
                    egui::Color32::from_rgb(180, 190, 200),
                    bg_color, outline_color,
                );
            }
        }
    }

    // Draw connecting lines and distances between consecutive pairs
    let mut total_path = 0.0_f32;
    for i in 0..pts.len().saturating_sub(1) {
        let a = pts[i];
        let b = pts[i + 1];
        if let (Some(sa), Some(sb)) = (
            world_to_screen(camera, screen, a),
            world_to_screen(camera, screen, b),
        ) {
            draw_dashed_line(&painter, sa, sb, 6.0, 3.0, egui::Stroke::new(2.0, accent));

            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            let dz = b[2] - a[2];
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            total_path += dist;

            let mid = egui::pos2((sa.x + sb.x) * 0.5, (sa.y + sb.y) * 0.5);
            let dist_label = format!("{} {unit}", fmt(dist));
            draw_label_with_bg(
                &painter, &dist_label,
                egui::pos2(mid.x, mid.y - 10.0),
                egui::FontId::proportional(13.0),
                text_color, bg_color, outline_color,
            );

            // Component distances (only for first segment to avoid clutter)
            if i == 0 {
                let comp_label = format!(
                    "\u{0394}X:{}  \u{0394}Y:{}  \u{0394}Z:{}",
                    fmt(dx.abs()), fmt(dy.abs()), fmt(dz.abs()),
                );
                draw_label_with_bg(
                    &painter, &comp_label,
                    egui::pos2(mid.x, mid.y + 6.0),
                    egui::FontId::proportional(10.0),
                    egui::Color32::from_rgb(180, 190, 200),
                    bg_color, outline_color,
                );
            }
        }
    }

    // Total path length for 3+ points
    if pts.len() >= 3 {
        let last_sp = world_to_screen(camera, screen, *pts.last().unwrap());
        if let Some(sp) = last_sp {
            let total_label = format!("Total: {} {unit}", fmt(total_path));
            draw_label_with_bg(
                &painter, &total_label,
                egui::pos2(sp.x, sp.y + 18.0),
                egui::FontId::proportional(11.0),
                accent, bg_color, outline_color,
            );
        }
    }

    // Angle between three points
    if pts.len() >= 3 {
        let a = pts[0];
        let b = pts[1]; // vertex of angle
        let c = pts[2];
        if let (Some(sa), Some(sb), Some(sc)) = (
            world_to_screen(camera, screen, a),
            world_to_screen(camera, screen, b),
            world_to_screen(camera, screen, c),
        ) {
            let ba = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
            let bc = [c[0] - b[0], c[1] - b[1], c[2] - b[2]];
            let dot = ba[0] * bc[0] + ba[1] * bc[1] + ba[2] * bc[2];
            let len_ba = (ba[0] * ba[0] + ba[1] * ba[1] + ba[2] * ba[2]).sqrt();
            let len_bc = (bc[0] * bc[0] + bc[1] * bc[1] + bc[2] * bc[2]).sqrt();
            if len_ba > 1e-7 && len_bc > 1e-7 {
                let cos_angle = (dot / (len_ba * len_bc)).clamp(-1.0, 1.0);
                let angle_deg = cos_angle.acos().to_degrees();

                // Arc at vertex
                let arc_r = 24.0_f32;
                let dir_a = egui::vec2(sa.x - sb.x, sa.y - sb.y);
                let dir_c = egui::vec2(sc.x - sb.x, sc.y - sb.y);
                let start_angle = dir_a.y.atan2(dir_a.x);
                let end_angle = dir_c.y.atan2(dir_c.x);
                let mut sweep = end_angle - start_angle;
                if sweep > std::f32::consts::PI {
                    sweep -= std::f32::consts::TAU;
                }
                if sweep < -std::f32::consts::PI {
                    sweep += std::f32::consts::TAU;
                }

                let segments = 24;
                let step = sweep / segments as f32;
                for seg in 0..segments {
                    let a1 = start_angle + seg as f32 * step;
                    let a2 = start_angle + (seg + 1) as f32 * step;
                    let p1 = egui::pos2(sb.x + arc_r * a1.cos(), sb.y + arc_r * a1.sin());
                    let p2 = egui::pos2(sb.x + arc_r * a2.cos(), sb.y + arc_r * a2.sin());
                    painter.line_segment([p1, p2], egui::Stroke::new(1.5, accent));
                }

                let mid_angle = start_angle + sweep * 0.5;
                let text_pos = egui::pos2(
                    sb.x + (arc_r + 16.0) * mid_angle.cos(),
                    sb.y + (arc_r + 16.0) * mid_angle.sin(),
                );
                let label = format!("{angle_deg:.1}\u{00B0}");
                draw_label_with_bg(
                    &painter, &label, text_pos,
                    egui::FontId::proportional(12.0),
                    text_color, bg_color, outline_color,
                );
            }
        }
    }
}

/// Draw a text label centred at `pos` with a semi-transparent background and
/// thin outline for readability against any scene content.
fn draw_label_with_bg(
    painter: &egui::Painter,
    text: &str,
    pos: egui::Pos2,
    font: egui::FontId,
    text_color: egui::Color32,
    bg_color: egui::Color32,
    outline_color: egui::Color32,
) {
    let galley = painter.layout_no_wrap(text.to_string(), font, text_color);
    let gw = galley.size().x;
    let gh = galley.size().y;
    let pad_x = 5.0_f32;
    let pad_y = 2.0_f32;
    let text_x = pos.x - gw * 0.5;
    let text_y = pos.y - gh * 0.5;
    let rect = egui::Rect::from_min_size(
        egui::pos2(text_x - pad_x, text_y - pad_y),
        egui::vec2(gw + pad_x * 2.0, gh + pad_y * 2.0),
    );
    painter.rect_filled(rect, 3.0, bg_color);
    painter.rect_stroke(rect, 3.0, egui::Stroke::new(0.5, outline_color), egui::StrokeKind::Middle);
    painter.galley(egui::pos2(text_x, text_y), galley, text_color);
}

// ---------------------------------------------------------------------------
// Snap visualization
// ---------------------------------------------------------------------------

pub(crate) fn draw_snap_overlay(ctx: &egui::Context, camera: &Camera, gui: &GuiState) {
    let highlight = match &gui.snap_highlight {
        Some(h) => h,
        None => return,
    };

    let screen = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("snap_overlay"),
    ));

    let orange = egui::Color32::from_rgb(255, 160, 30);

    match highlight {
        SnapHighlight::Vertex(p) => {
            if let Some(sp) = world_to_screen(camera, screen, *p) {
                painter.circle_stroke(sp, 6.0, egui::Stroke::new(2.0, orange));
                painter.circle_filled(sp, 2.5, orange);
            }
        }
        SnapHighlight::GridPoint(p) => {
            if let Some(sp) = world_to_screen(camera, screen, *p) {
                painter.circle_filled(sp, 4.0, orange);
                // Crosshair
                let arm = 8.0;
                painter.line_segment(
                    [egui::pos2(sp.x - arm, sp.y), egui::pos2(sp.x + arm, sp.y)],
                    egui::Stroke::new(1.0, orange),
                );
                painter.line_segment(
                    [egui::pos2(sp.x, sp.y - arm), egui::pos2(sp.x, sp.y + arm)],
                    egui::Stroke::new(1.0, orange),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Sketch plane preview
// ---------------------------------------------------------------------------

pub(crate) fn draw_sketch_plane_preview(ctx: &egui::Context, camera: &Camera, gui: &GuiState) {
    let sketch = match &gui.sketch_mode {
        Some(s) => s,
        None => return,
    };

    let screen = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("sketch_plane_preview"),
    ));

    // Draw a semi-transparent rectangle at the sketch work plane
    let plane = &sketch.plane;
    let o = [plane.origin.x as f32, plane.origin.y as f32, plane.origin.z as f32];
    let u = [plane.x_axis.x as f32, plane.x_axis.y as f32, plane.x_axis.z as f32];
    let v = [plane.y_axis.x as f32, plane.y_axis.y as f32, plane.y_axis.z as f32];

    let extent = camera.distance * 0.5;

    // Four corners of the plane quad
    let corners = [
        [o[0] - u[0] * extent - v[0] * extent, o[1] - u[1] * extent - v[1] * extent, o[2] - u[2] * extent - v[2] * extent],
        [o[0] + u[0] * extent - v[0] * extent, o[1] + u[1] * extent - v[1] * extent, o[2] + u[2] * extent - v[2] * extent],
        [o[0] + u[0] * extent + v[0] * extent, o[1] + u[1] * extent + v[1] * extent, o[2] + u[2] * extent + v[2] * extent],
        [o[0] - u[0] * extent + v[0] * extent, o[1] - u[1] * extent + v[1] * extent, o[2] - u[2] * extent + v[2] * extent],
    ];

    let screen_corners: Vec<egui::Pos2> = corners
        .iter()
        .filter_map(|c| world_to_screen(camera, screen, *c))
        .collect();

    if screen_corners.len() == 4 {
        // Semi-transparent fill
        let fill = egui::Color32::from_rgba_premultiplied(80, 140, 220, 20);
        painter.add(egui::Shape::convex_polygon(
            screen_corners.clone(),
            fill,
            egui::Stroke::NONE,
        ));

        // Border
        let border = egui::Color32::from_rgba_premultiplied(80, 140, 220, 60);
        for i in 0..4 {
            painter.line_segment(
                [screen_corners[i], screen_corners[(i + 1) % 4]],
                egui::Stroke::new(1.0, border),
            );
        }
    }

    // U axis (red) and V axis (green) on the plane
    let axis_len = extent * 0.3;
    let u_tip = [o[0] + u[0] * axis_len, o[1] + u[1] * axis_len, o[2] + u[2] * axis_len];
    let v_tip = [o[0] + v[0] * axis_len, o[1] + v[1] * axis_len, o[2] + v[2] * axis_len];

    if let Some(so) = world_to_screen(camera, screen, o) {
        if let Some(su) = world_to_screen(camera, screen, u_tip) {
            painter.line_segment([so, su], egui::Stroke::new(2.0, egui::Color32::from_rgb(220, 60, 60)));
        }
        if let Some(sv) = world_to_screen(camera, screen, v_tip) {
            painter.line_segment([so, sv], egui::Stroke::new(2.0, egui::Color32::from_rgb(60, 200, 60)));
        }
    }
}

// ---------------------------------------------------------------------------
// Box selection rubber band
// ---------------------------------------------------------------------------

pub(crate) fn draw_rubber_band(ctx: &egui::Context, gui: &GuiState) {
    let (start, end) = match (gui.box_select_start, gui.box_select_end) {
        (Some(s), Some(e)) => (s, e),
        _ => return,
    };

    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("rubber_band"),
    ));

    // Left-to-right = window selection (blue), right-to-left = crossing (green)
    let is_window = end.0 >= start.0;
    let (fill, stroke_color) = if is_window {
        (
            egui::Color32::from_rgba_premultiplied(40, 90, 200, 25),
            egui::Color32::from_rgba_premultiplied(80, 140, 255, 180),
        )
    } else {
        (
            egui::Color32::from_rgba_premultiplied(40, 200, 90, 25),
            egui::Color32::from_rgba_premultiplied(80, 255, 140, 180),
        )
    };

    let rect = egui::Rect::from_two_pos(
        egui::pos2(start.0, start.1),
        egui::pos2(end.0, end.1),
    );
    painter.rect_filled(rect, 0.0, fill);

    if is_window {
        painter.rect_stroke(
            rect, 0.0,
            egui::Stroke::new(1.0, stroke_color),
            egui::StrokeKind::Middle,
        );
    } else {
        // Dashed border for crossing selection
        let corners = [rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom()];
        for i in 0..4 {
            let a = corners[i];
            let b = corners[(i + 1) % 4];
            draw_dashed_line_2d(&painter, a, b, 6.0, 3.0, egui::Stroke::new(1.0, stroke_color));
        }
    }
}

fn draw_dashed_line_2d(
    painter: &egui::Painter,
    a: egui::Pos2,
    b: egui::Pos2,
    dash: f32,
    gap: f32,
    stroke: egui::Stroke,
) {
    let d = b - a;
    let len = d.length();
    if len < 1.0 { return; }
    let dir = d / len;
    let mut t = 0.0;
    while t < len {
        let t_end = (t + dash).min(len);
        let p0 = a + dir * t;
        let p1 = a + dir * t_end;
        painter.line_segment([p0, p1], stroke);
        t = t_end + gap;
    }
}

// ---------------------------------------------------------------------------
// Transform gizmo overlay
// ---------------------------------------------------------------------------

/// Axis color palette: X = red, Y = green, Z = blue.
const GIZMO_AXIS_COLORS: [[u8; 3]; 3] = [
    [220, 60, 60],   // X
    [60, 200, 60],   // Y
    [60, 100, 220],  // Z
];

/// Brighter version of axis colors for hover highlight.
const GIZMO_AXIS_HOVER: [[u8; 3]; 3] = [
    [255, 100, 100], // X hovered
    [100, 255, 100], // Y hovered
    [100, 160, 255], // Z hovered
];

/// Compute the average position (centroid) of a scene object's mesh vertices.
fn mesh_centroid(obj: &crate::scene::SceneObject) -> [f32; 3] {
    let verts = &obj.mesh.vertices;
    if verts.is_empty() {
        return [0.0, 0.0, 0.0];
    }
    let mut sx = 0.0_f64;
    let mut sy = 0.0_f64;
    let mut sz = 0.0_f64;
    for v in verts {
        sx += v.x;
        sy += v.y;
        sz += v.z;
    }
    let n = verts.len() as f64;
    [(sx / n) as f32, (sy / n) as f32, (sz / n) as f32]
}

/// Return the axis color, choosing the brighter variant when hovered.
fn axis_color(axis: usize, hover_axis: Option<u8>) -> egui::Color32 {
    if hover_axis == Some(axis as u8) {
        let c = GIZMO_AXIS_HOVER[axis];
        egui::Color32::from_rgb(c[0], c[1], c[2])
    } else {
        let c = GIZMO_AXIS_COLORS[axis];
        egui::Color32::from_rgb(c[0], c[1], c[2])
    }
}

/// Return stroke width: thicker when the axis is hovered.
fn axis_width(axis: usize, hover_axis: Option<u8>) -> f32 {
    if hover_axis == Some(axis as u8) { 3.0 } else { 1.5 }
}

/// Compute the three screen-space direction vectors for the gizmo axes.
/// X = camera right, Y = camera up, Z = cross(right, up) (into screen),
/// displayed as a shorter diagonal.
fn gizmo_screen_dirs(camera: &Camera) -> [[f32; 2]; 3] {
    let r = camera.screen_right();
    let u = camera.screen_up();
    // Z axis: cross(right, up) projected onto screen as a diagonal hint.
    // Since cross(right, up) points into the screen (toward camera), we
    // represent it as a diagonal between -right and -up for visual clarity.
    let z_world = normalize3(cross3(r, u));
    let z_sx = dot3(z_world, r);
    let z_sy = -dot3(z_world, u);
    // If the Z direction collapses to near-zero on screen, use a fallback diagonal.
    let z_len = (z_sx * z_sx + z_sy * z_sy).sqrt();
    let (zx, zy) = if z_len > 0.01 {
        (z_sx / z_len, z_sy / z_len)
    } else {
        // Fallback: 45-degree diagonal
        let inv = 1.0 / 2.0_f32.sqrt();
        (-inv, -inv)
    };
    [
        [1.0, 0.0],   // X: rightward on screen
        [0.0, -1.0],  // Y: upward on screen (egui Y is down)
        [zx, zy],     // Z: into-screen hint
    ]
}

/// Draw an arrowhead (filled triangle) at the tip of an axis line.
fn draw_arrowhead(
    painter: &egui::Painter,
    tip: egui::Pos2,
    dir: [f32; 2],
    size: f32,
    color: egui::Color32,
) {
    let dx = dir[0];
    let dy = dir[1];
    // Perpendicular
    let px = -dy;
    let py = dx;
    let half = size * 0.4;
    let base = egui::pos2(tip.x - dx * size, tip.y - dy * size);
    let left = egui::pos2(base.x + px * half, base.y + py * half);
    let right = egui::pos2(base.x - px * half, base.y - py * half);
    painter.add(egui::Shape::convex_polygon(
        vec![tip, left, right],
        color,
        egui::Stroke::NONE,
    ));
}

/// Draw the translate gizmo: three arrows from the origin.
fn draw_translate_gizmo(
    painter: &egui::Painter,
    origin: egui::Pos2,
    dirs: &[[f32; 2]; 3],
    hover_axis: Option<u8>,
) {
    let arrow_len = 60.0_f32;
    let head_size = 12.0_f32;
    let z_scale = 0.6; // Z arrow is shorter
    for (i, dir) in dirs.iter().enumerate() {
        let scale = if i == 2 { z_scale } else { 1.0 };
        let len = arrow_len * scale;
        let end = egui::pos2(
            origin.x + dir[0] * len,
            origin.y + dir[1] * len,
        );
        let color = axis_color(i, hover_axis);
        let width = axis_width(i, hover_axis);
        painter.line_segment([origin, end], egui::Stroke::new(width, color));
        draw_arrowhead(painter, end, *dir, head_size * scale, color);
    }
}

/// Draw the rotate gizmo: three quarter-circle arcs.
fn draw_rotate_gizmo(
    painter: &egui::Painter,
    origin: egui::Pos2,
    dirs: &[[f32; 2]; 3],
    hover_axis: Option<u8>,
) {
    let radius = 50.0_f32;
    let segments = 20_u32;
    let quarter = std::f32::consts::FRAC_PI_2;

    for i in 0..3 {
        let color = axis_color(i, hover_axis);
        let width = axis_width(i, hover_axis);

        // Each arc lies in the plane perpendicular to its axis.
        // For screen-space rendering:
        //   X rotation arc: spans between Y and Z directions
        //   Y rotation arc: spans between X and Z directions
        //   Z rotation arc: spans between X and Y directions (screen plane circle)
        let (d_a, d_b) = match i {
            0 => (dirs[1], dirs[2]), // YZ plane
            1 => (dirs[0], dirs[2]), // XZ plane
            _ => (dirs[0], dirs[1]), // XY plane (screen circle)
        };

        for s in 0..segments {
            let t0 = s as f32 / segments as f32 * quarter;
            let t1 = (s + 1) as f32 / segments as f32 * quarter;
            let (c0, s0) = (t0.cos(), t0.sin());
            let (c1, s1) = (t1.cos(), t1.sin());
            let p0 = egui::pos2(
                origin.x + (d_a[0] * c0 + d_b[0] * s0) * radius,
                origin.y + (d_a[1] * c0 + d_b[1] * s0) * radius,
            );
            let p1 = egui::pos2(
                origin.x + (d_a[0] * c1 + d_b[0] * s1) * radius,
                origin.y + (d_a[1] * c1 + d_b[1] * s1) * radius,
            );
            painter.line_segment([p0, p1], egui::Stroke::new(width, color));
        }
    }
}

/// Draw the scale gizmo: three lines ending in small filled squares.
fn draw_scale_gizmo(
    painter: &egui::Painter,
    origin: egui::Pos2,
    dirs: &[[f32; 2]; 3],
    hover_axis: Option<u8>,
) {
    let line_len = 50.0_f32;
    let square_half = 3.0_f32;
    let z_scale = 0.6;
    for (i, dir) in dirs.iter().enumerate() {
        let scale = if i == 2 { z_scale } else { 1.0 };
        let len = line_len * scale;
        let end = egui::pos2(
            origin.x + dir[0] * len,
            origin.y + dir[1] * len,
        );
        let color = axis_color(i, hover_axis);
        let width = axis_width(i, hover_axis);
        painter.line_segment([origin, end], egui::Stroke::new(width, color));
        // Small filled square at the tip
        let sq = egui::Rect::from_center_size(end, egui::vec2(square_half * 2.0, square_half * 2.0));
        painter.rect_filled(sq, 0.0, color);
    }
}

/// Draw the transform gizmo at the centroid of the currently selected object.
///
/// The gizmo mode (Translate/Rotate/Scale) is read from `gui.gizmo_mode`.
/// Keyboard toggling between modes (W/E/R) should be handled in app.rs.
pub(crate) fn draw_transform_gizmo(
    ctx: &egui::Context,
    camera: &Camera,
    gui: &mut GuiState,
    scene: &crate::scene::Scene,
) {
    if gui.gizmo_mode == GizmoMode::None {
        return;
    }

    // Only show gizmo when exactly one object is selected.
    let selected: Vec<&crate::scene::SceneObject> = scene.selected_objects();
    if selected.len() != 1 {
        return;
    }
    let obj = selected[0];
    let obj_id = obj.id;

    let center_3d = mesh_centroid(obj);
    let screen = ctx.screen_rect();
    let origin = match world_to_screen(camera, screen, center_3d) {
        Some(p) => p,
        None => return, // Behind camera
    };

    let dirs = gizmo_screen_dirs(camera);
    let arrow_len = 60.0_f32;
    let z_scale = 0.6;

    // Hover detection: find closest axis to cursor
    let mouse_pos = ctx.input(|i| i.pointer.hover_pos());
    let drag_delta = ctx.input(|i| i.pointer.delta());
    let primary_down = ctx.input(|i| i.pointer.button_down(egui::PointerButton::Primary));
    let primary_released = ctx.input(|i| i.pointer.button_released(egui::PointerButton::Primary));

    gui.gizmo_hover_axis = None;
    if let Some(mp) = mouse_pos {
        let threshold = 12.0_f32;
        for (i, dir) in dirs.iter().enumerate() {
            let scale = if i == 2 { z_scale } else { 1.0 };
            let len = arrow_len * scale;
            let end = egui::pos2(origin.x + dir[0] * len, origin.y + dir[1] * len);
            let dist = point_to_segment_dist(mp, origin, end);
            if dist < threshold {
                gui.gizmo_hover_axis = Some(i as u8);
                break;
            }
        }
    }

    // Drag interaction: when clicking on a hovered axis and dragging
    if let Some(hovered_axis) = gui.gizmo_hover_axis {
        if primary_down {
        let axis = hovered_axis as usize;
        let dx = drag_delta.x;
        let dy = drag_delta.y;
        if dx.abs() > 0.5 || dy.abs() > 0.5 {
            let dir = dirs[axis];
            let proj = dx * dir[0] + dy * dir[1];
            let speed = camera.distance * 0.003;
            let amount = proj as f64 * speed as f64;
            if amount.abs() > 1e-6 {
                use super::GuiAction;
                match gui.gizmo_mode {
                    GizmoMode::Translate => {
                        let (mx, my, mz) = match axis {
                            0 => (amount, 0.0, 0.0),
                            1 => (0.0, amount, 0.0),
                            _ => (0.0, 0.0, amount),
                        };
                        gui.actions.push(GuiAction::MoveObject { id: obj_id, dx: mx, dy: my, dz: mz });
                    }
                    GizmoMode::Rotate => {
                        let angle = proj as f64 * 0.5;
                        gui.actions.push(GuiAction::RotateObject { id: obj_id, axis: axis as u8, angle_deg: angle });
                    }
                    GizmoMode::Scale => {
                        let factor = 1.0 + proj as f64 * 0.005;
                        gui.actions.push(GuiAction::ScaleObjectUniform { id: obj_id, factor });
                    }
                    GizmoMode::None => {}
                }
            }
        }
        } // if primary_down
    } // if let Some(hovered_axis)
    if primary_released {
        // Drag ended — no special cleanup needed
    }

    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("transform_gizmo"),
    ));

    let hover = gui.gizmo_hover_axis;

    match gui.gizmo_mode {
        GizmoMode::Translate => draw_translate_gizmo(&painter, origin, &dirs, hover),
        GizmoMode::Rotate => draw_rotate_gizmo(&painter, origin, &dirs, hover),
        GizmoMode::Scale => draw_scale_gizmo(&painter, origin, &dirs, hover),
        GizmoMode::None => {} // Already handled above
    }

    // Center dot
    painter.circle_filled(origin, 5.0, egui::Color32::WHITE);
    painter.circle_stroke(origin, 5.0, egui::Stroke::new(1.0, egui::Color32::from_gray(60)));

    // Mode label below the gizmo center
    let label = match gui.gizmo_mode {
        GizmoMode::Translate => "Move (W)",
        GizmoMode::Rotate => "Rotate (E)",
        GizmoMode::Scale => "Scale (R)",
        GizmoMode::None => "",
    };
    if !label.is_empty() {
        painter.text(
            egui::pos2(origin.x, origin.y + 10.0),
            egui::Align2::CENTER_TOP,
            label,
            egui::FontId::proportional(10.0),
            egui::Color32::from_gray(160),
        );
    }
}

/// Distance from point to line segment.
fn point_to_segment_dist(p: egui::Pos2, a: egui::Pos2, b: egui::Pos2) -> f32 {
    let ab = egui::vec2(b.x - a.x, b.y - a.y);
    let ap = egui::vec2(p.x - a.x, p.y - a.y);
    let ab_sq = ab.x * ab.x + ab.y * ab.y;
    if ab_sq < 1e-6 {
        return ap.length();
    }
    let t = ((ap.x * ab.x + ap.y * ab.y) / ab_sq).clamp(0.0, 1.0);
    let proj = egui::pos2(a.x + ab.x * t, a.y + ab.y * t);
    (p - proj).length()
}

// ---------------------------------------------------------------------------
// Sub-element selection highlight overlay
// ---------------------------------------------------------------------------

pub(crate) fn draw_selection_overlay(
    ctx: &egui::Context,
    camera: &Camera,
    gui: &GuiState,
    scene: &Scene,
    nav: &crate::nav::NavConfig,
) {
    let has_selection = !gui.selected_entities.is_empty();
    let has_preselection = gui.preselected_entity.is_some();
    if !has_selection && !has_preselection {
        return;
    }

    let screen = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("selection_overlay"),
    ));

    // Preselection (hover) — use configurable preselection_color from NavConfig
    if let Some(pre) = &gui.preselected_entity {
        let skip = gui.selected_entities.contains(pre);
        if !skip {
            let pre_obj = gui.preselected_object_id
                .and_then(|id| scene.get_object(id));
            if let Some(obj) = pre_obj {
                let [pr, pg, pb] = nav.preselection_color;
                let pre_color = egui::Color32::from_rgba_unmultiplied(pr, pg, pb, 180);
                let pre_fill = egui::Color32::from_rgba_unmultiplied(pr, pg, pb, 35);
                draw_entity_highlight(&painter, camera, screen, obj, pre, pre_color, pre_fill);

                // Show entity type label near the cursor
                if let Some(pos) = gui.pointer_physical {
                    let entity_label = match pre {
                        SelectedEntity::Vertex(_) => "Vertex",
                        SelectedEntity::Edge(_) => "Edge",
                        SelectedEntity::Face(_) => "Face",
                        SelectedEntity::Solid(_) => "Solid",
                        _ => "",
                    };
                    if !entity_label.is_empty() {
                        let label_pos = egui::pos2(pos.0 + 16.0, pos.1 - 8.0);
                        let bg = egui::Color32::from_rgba_premultiplied(25, 25, 30, 200);
                        let text_col = egui::Color32::from_rgba_unmultiplied(pr, pg, pb, 220);
                        draw_label_with_bg(
                            &painter,
                            entity_label,
                            label_pos,
                            egui::FontId::proportional(10.0),
                            text_col,
                            bg,
                            egui::Color32::from_rgba_unmultiplied(pr, pg, pb, 60),
                        );
                    }
                }
            }
        }
    }

    // Selection — use configurable selection_color from NavConfig
    if let Some(obj) = scene.selected_object() {
        let [sr, sg, sb] = nav.selection_color;
        let sel_color = egui::Color32::from_rgba_unmultiplied(sr, sg, sb, 200);
        let sel_fill = egui::Color32::from_rgba_unmultiplied(sr, sg, sb, 50);
        for entity in &gui.selected_entities {
            draw_entity_highlight(&painter, camera, screen, obj, entity, sel_color, sel_fill);
        }

        // Measurement between exactly two selected sub-elements
        draw_measurement_overlay_between(&painter, camera, screen, obj, &gui.selected_entities);
    }
}

fn draw_entity_highlight(
    painter: &egui::Painter,
    camera: &Camera,
    screen: egui::Rect,
    obj: &crate::scene::SceneObject,
    entity: &SelectedEntity,
    color: egui::Color32,
    fill: egui::Color32,
) {
    match entity {
        SelectedEntity::Face(face_h) => {
            if let Some((_fh, start, count)) = obj
                .face_tri_map
                .iter()
                .find(|(fh, _, _)| fh == face_h)
            {
                let base = start * 3;
                let end = base + count * 3;
                if end <= obj.vertices.len() {
                    for i in (base..end).step_by(3) {
                        let p0 = obj.vertices[i].position;
                        let p1 = obj.vertices[i + 1].position;
                        let p2 = obj.vertices[i + 2].position;
                        let s0 = world_to_screen(camera, screen, p0);
                        let s1 = world_to_screen(camera, screen, p1);
                        let s2 = world_to_screen(camera, screen, p2);
                        if let (Some(a), Some(b), Some(c)) = (s0, s1, s2) {
                            painter.add(egui::Shape::convex_polygon(
                                vec![a, b, c],
                                fill,
                                egui::Stroke::NONE,
                            ));
                            let stroke = egui::Stroke::new(1.5, color);
                            painter.line_segment([a, b], stroke);
                            painter.line_segment([b, c], stroke);
                            painter.line_segment([c, a], stroke);
                        }
                    }
                }
            }
        }
        SelectedEntity::Edge(edge_h) => {
            for (idx, eh) in obj.edge_handles.iter().enumerate() {
                if eh == edge_h {
                    let (start, end) = obj.edge_positions[idx];
                    let s0 = world_to_screen(camera, screen, start);
                    let s1 = world_to_screen(camera, screen, end);
                    if let (Some(a), Some(b)) = (s0, s1) {
                        // Glow (wider, semi-transparent)
                        let glow = egui::Color32::from_rgba_unmultiplied(
                            color.r(), color.g(), color.b(), 50,
                        );
                        painter.line_segment([a, b], egui::Stroke::new(8.0, glow));
                        // Core line
                        painter.line_segment([a, b], egui::Stroke::new(3.5, color));
                        // Endpoint dots
                        painter.circle_filled(a, 4.0, color);
                        painter.circle_filled(b, 4.0, color);
                        painter.circle_stroke(a, 4.0, egui::Stroke::new(1.0, egui::Color32::BLACK));
                        painter.circle_stroke(b, 4.0, egui::Stroke::new(1.0, egui::Color32::BLACK));
                    }
                    break;
                }
            }
        }
        SelectedEntity::Vertex(vert_h) => {
            for (idx, vh) in obj.vertex_handles.iter().enumerate() {
                if vh == vert_h {
                    let pos = obj.vertex_positions[idx];
                    if let Some(sp) = world_to_screen(camera, screen, pos) {
                        // Glow ring
                        let glow = egui::Color32::from_rgba_unmultiplied(
                            color.r(), color.g(), color.b(), 60,
                        );
                        painter.circle_filled(sp, 10.0, glow);
                        // Filled marker
                        painter.circle_filled(sp, 6.0, color);
                        painter.circle_stroke(sp, 6.0, egui::Stroke::new(1.5, egui::Color32::BLACK));
                        // Bright center
                        painter.circle_filled(sp, 2.0, egui::Color32::WHITE);
                    }
                    break;
                }
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Measurement overlay between two selected sub-elements
// ---------------------------------------------------------------------------

/// Compute the representative 3D point for a selected sub-element.
/// - Vertex: the vertex position
/// - Edge: midpoint of the edge segment
/// - Face: centroid of the face triangles (average of all triangle vertex positions)
fn representative_point(
    obj: &crate::scene::SceneObject,
    entity: &SelectedEntity,
) -> Option<[f32; 3]> {
    match entity {
        SelectedEntity::Vertex(vh) => {
            obj.vertex_handles
                .iter()
                .position(|h| h == vh)
                .map(|idx| obj.vertex_positions[idx])
        }
        SelectedEntity::Edge(eh) => {
            obj.edge_handles.iter().position(|h| h == eh).map(|idx| {
                let (s, e) = obj.edge_positions[idx];
                [
                    (s[0] + e[0]) * 0.5,
                    (s[1] + e[1]) * 0.5,
                    (s[2] + e[2]) * 0.5,
                ]
            })
        }
        SelectedEntity::Face(fh) => {
            let (_, start, count) = obj.face_tri_map.iter().find(|(f, _, _)| f == fh)?;
            let base = start * 3;
            let end = base + count * 3;
            if end > obj.vertices.len() || *count == 0 {
                return None;
            }
            let mut sx = 0.0_f64;
            let mut sy = 0.0_f64;
            let mut sz = 0.0_f64;
            let n = (end - base) as f64;
            for i in base..end {
                let p = obj.vertices[i].position;
                sx += p[0] as f64;
                sy += p[1] as f64;
                sz += p[2] as f64;
            }
            Some([(sx / n) as f32, (sy / n) as f32, (sz / n) as f32])
        }
        _ => None,
    }
}

/// Draw a measurement overlay (dashed line + distance label) between exactly
/// two selected sub-elements in the viewport.
fn draw_measurement_overlay_between(
    painter: &egui::Painter,
    camera: &Camera,
    screen: egui::Rect,
    obj: &crate::scene::SceneObject,
    entities: &[SelectedEntity],
) {
    if entities.len() != 2 {
        return;
    }
    let p0 = match representative_point(obj, &entities[0]) {
        Some(p) => p,
        None => return,
    };
    let p1 = match representative_point(obj, &entities[1]) {
        Some(p) => p,
        None => return,
    };

    let s0 = match world_to_screen(camera, screen, p0) {
        Some(p) => p,
        None => return,
    };
    let s1 = match world_to_screen(camera, screen, p1) {
        Some(p) => p,
        None => return,
    };

    let cyan = egui::Color32::from_rgb(0, 220, 220);
    draw_dashed_line(painter, s0, s1, 6.0, 3.0, egui::Stroke::new(1.5, cyan));

    // Small endpoint markers
    painter.circle_filled(s0, 3.0, cyan);
    painter.circle_filled(s1, 3.0, cyan);

    // 3D Euclidean distance
    let dx = (p1[0] - p0[0]) as f64;
    let dy = (p1[1] - p0[1]) as f64;
    let dz = (p1[2] - p0[2]) as f64;
    let dist = (dx * dx + dy * dy + dz * dz).sqrt();

    let mid = egui::pos2((s0.x + s1.x) * 0.5, (s0.y + s1.y) * 0.5);
    let bg = egui::Color32::from_rgba_premultiplied(20, 20, 20, 210);
    let outline = egui::Color32::from_rgba_premultiplied(0, 220, 220, 100);

    draw_label_with_bg(
        painter,
        &format!("{dist:.3} mm"),
        egui::pos2(mid.x, mid.y - 10.0),
        egui::FontId::proportional(12.0),
        egui::Color32::WHITE,
        bg,
        outline,
    );
}

// ---------------------------------------------------------------------------
// Welcome screen for empty scene
// ---------------------------------------------------------------------------

pub(crate) fn draw_welcome_screen(ctx: &egui::Context, gui: &mut GuiState) {
    let screen = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Middle,
        egui::Id::new("welcome_screen"),
    ));

    let cx = screen.center().x;
    let cy = screen.center().y - 30.0;

    // Title
    painter.text(
        egui::pos2(cx, cy - 60.0),
        egui::Align2::CENTER_CENTER,
        "\u{2B22}  CADKernel",
        egui::FontId::new(24.0, egui::FontFamily::Proportional),
        egui::Color32::from_rgb(160, 170, 185),
    );

    // Subtitle
    painter.text(
        egui::pos2(cx, cy - 30.0),
        egui::Align2::CENTER_CENTER,
        "Open-Source B-Rep CAD Kernel",
        egui::FontId::proportional(12.0),
        egui::Color32::from_rgb(90, 95, 110),
    );

    // Quick action buttons
    let btn_w = 150.0;
    let btn_h = 32.0;
    let btn_gap = 8.0;
    let total_h = 3.0 * btn_h + 2.0 * btn_gap;
    let start_y = cy + 10.0;

    let actions: &[(&str, &str, &str)] = &[
        ("\u{2795}  Create Box", "Create a box primitive", "box"),
        ("\u{1F4C2}  Import File", "Import STL/OBJ/glTF", "import"),
        ("\u{1F4C4}  Open Project", "Open .cadk file", "open"),
    ];

    let pointer_pos = ctx.input(|i| i.pointer.hover_pos());
    let clicked = ctx.input(|i| i.pointer.button_clicked(egui::PointerButton::Primary));

    for (i, &(label, hint, action_id)) in actions.iter().enumerate() {
        let btn_y = start_y + i as f32 * (btn_h + btn_gap);
        let btn_rect = egui::Rect::from_center_size(
            egui::pos2(cx, btn_y + btn_h * 0.5),
            egui::vec2(btn_w, btn_h),
        );

        let hovered = pointer_pos.is_some_and(|p| btn_rect.contains(p));

        let (bg, text_color) = if hovered {
            (
                egui::Color32::from_rgba_premultiplied(0, 122, 204, 50),
                egui::Color32::from_rgb(220, 225, 235),
            )
        } else {
            (
                egui::Color32::from_rgba_premultiplied(40, 42, 50, 180),
                egui::Color32::from_rgb(160, 168, 180),
            )
        };

        painter.rect_filled(btn_rect, 6.0, bg);
        painter.rect_stroke(
            btn_rect,
            6.0,
            egui::Stroke::new(0.5, egui::Color32::from_rgb(65, 70, 80)),
            egui::StrokeKind::Outside,
        );

        painter.text(
            btn_rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(12.5),
            text_color,
        );

        if hovered && clicked {
            match action_id {
                "box" => {
                    gui.active_task = Some(super::task_panel::ActiveTask::Box {
                        width: 10.0, height: 10.0, depth: 10.0, preview_id: None,
                    });
                }
                "import" => {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Mesh", &["stl", "obj", "gltf", "glb"])
                        .pick_file()
                    {
                        gui.actions.push(GuiAction::ImportFile(path));
                    }
                }
                "open" => {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("CADKernel", &["cadk"])
                        .pick_file()
                    {
                        gui.actions.push(GuiAction::OpenFile(path));
                    }
                }
                _ => {}
            }
        }

        if hovered {
            ctx.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
        }

        // Hint text below button
        painter.text(
            egui::pos2(cx, btn_y + btn_h + 1.0),
            egui::Align2::CENTER_TOP,
            hint,
            egui::FontId::proportional(9.5),
            egui::Color32::from_rgb(70, 75, 85),
        );
    }

    // Footer: keyboard shortcuts hint
    painter.text(
        egui::pos2(cx, start_y + total_h + 40.0),
        egui::Align2::CENTER_CENTER,
        "Press F1 for keyboard shortcuts",
        egui::FontId::proportional(10.0),
        egui::Color32::from_rgb(70, 75, 85),
    );
}

// ---------------------------------------------------------------------------
// Breadcrumb navigation bar
// ---------------------------------------------------------------------------

pub(crate) fn draw_breadcrumb_bar(
    ctx: &egui::Context,
    gui: &mut GuiState,
    scene: &Scene,
) {
    egui::TopBottomPanel::top("breadcrumb_bar")
        .exact_height(20.0)
        .frame(egui::Frame {
            fill: egui::Color32::from_rgb(30, 32, 38),
            inner_margin: egui::Margin::symmetric(8, 0),
            stroke: egui::Stroke::new(0.5, egui::Color32::from_rgb(50, 54, 62)),
            ..egui::Frame::NONE
        })
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;

                let dim_color = theme::COLOR_DIM;
                let active_color = egui::Color32::from_rgb(200, 205, 215);
                let sep_color = egui::Color32::from_rgb(80, 84, 92);
                let font = egui::FontId::new(10.5, egui::FontFamily::Proportional);

                // Determine segments
                let has_selection = scene.selected_object().is_some();
                let has_entities = !gui.selected_entities.is_empty();
                let in_sketch = gui.sketch_mode.is_some();

                // Scene root
                let scene_active = !has_selection && !in_sketch;
                let scene_color = if scene_active { active_color } else { dim_color };
                if ui.add(egui::Label::new(egui::RichText::new("Scene").font(font.clone()).color(scene_color)).selectable(false).sense(egui::Sense::click())).clicked() && !scene_active {
                    gui.actions.push(GuiAction::DeselectAll);
                }

                if let Some(obj) = scene.selected_object() {
                    ui.label(egui::RichText::new("\u{203A}").size(12.0).color(sep_color));
                    let obj_active = !has_entities && !in_sketch;
                    let obj_color = if obj_active { active_color } else { dim_color };
                    ui.add(egui::Label::new(egui::RichText::new(&obj.name).font(font.clone()).color(obj_color)).selectable(false));

                    if has_entities {
                        ui.label(egui::RichText::new("\u{203A}").size(12.0).color(sep_color));
                        let mode_str = match gui.selection_mode {
                            SelectionMode::Face => "Face",
                            SelectionMode::Edge => "Edge",
                            SelectionMode::Vertex => "Vertex",
                            SelectionMode::Solid => "Solid",
                        };
                        ui.add(egui::Label::new(egui::RichText::new(mode_str).font(font.clone()).color(active_color)).selectable(false));

                        if gui.selected_entities.len() > 1 {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new(format!("{} selected", gui.selected_entities.len())).size(10.0).color(theme::COLOR_ACCENT));
                            });
                        }
                    }
                }

                if in_sketch {
                    ui.label(egui::RichText::new("\u{203A}").size(12.0).color(sep_color));
                    ui.add(egui::Label::new(egui::RichText::new("Sketch").font(font).color(active_color)).selectable(false));
                }
            });
        });
}

// ---------------------------------------------------------------------------
// Toast notification overlay
// ---------------------------------------------------------------------------

const TOAST_DURATION_SECS: f32 = 3.0;
const TOAST_FADE_SECS: f32 = 0.5;

pub(crate) fn draw_toast_overlay(ctx: &egui::Context, gui: &mut GuiState) {
    let now = std::time::Instant::now();

    // Remove expired toasts
    gui.toasts.retain(|t| {
        now.duration_since(t.created_at).as_secs_f32() < TOAST_DURATION_SECS
    });

    if gui.toasts.is_empty() {
        return;
    }

    let screen = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("toast_overlay"),
    ));

    let toast_w = 280.0_f32;
    let toast_h = 28.0_f32;
    let gap = 4.0_f32;
    let right_margin = 12.0_f32;
    let bottom_margin = 36.0_f32; // above status bar

    let max_toasts = gui.toasts.len().min(5);

    for (i, toast) in gui.toasts.iter().rev().take(max_toasts).enumerate() {
        let age = now.duration_since(toast.created_at).as_secs_f32();
        let fade_start = TOAST_DURATION_SECS - TOAST_FADE_SECS;
        let alpha = if age >= fade_start {
            let t = (TOAST_DURATION_SECS - age) / TOAST_FADE_SECS;
            (t.clamp(0.0, 1.0) * 255.0) as u8
        } else {
            let enter_t = (age / 0.15).min(1.0);
            (enter_t * 255.0) as u8
        };

        let y = screen.bottom() - bottom_margin - (i as f32 + 1.0) * (toast_h + gap);
        let rect = egui::Rect::from_min_size(
            egui::pos2(screen.right() - toast_w - right_margin, y),
            egui::vec2(toast_w, toast_h),
        );

        let (bg_color, accent, icon) = match toast.level {
            ToastLevel::Success => (
                egui::Color32::from_rgba_premultiplied(20, 45, 30, alpha),
                egui::Color32::from_rgba_premultiplied(60, 200, 80, alpha),
                "\u{2713}",
            ),
            ToastLevel::Info => (
                egui::Color32::from_rgba_premultiplied(20, 30, 50, alpha),
                egui::Color32::from_rgba_premultiplied(70, 140, 220, alpha),
                "\u{2139}",
            ),
            ToastLevel::Warning => (
                egui::Color32::from_rgba_premultiplied(50, 40, 15, alpha),
                egui::Color32::from_rgba_premultiplied(230, 180, 50, alpha),
                "\u{26A0}",
            ),
            ToastLevel::Error => (
                egui::Color32::from_rgba_premultiplied(50, 20, 20, alpha),
                egui::Color32::from_rgba_premultiplied(220, 70, 60, alpha),
                "\u{2716}",
            ),
        };

        // Background with rounded corners
        painter.rect_filled(rect, 4.0, bg_color);
        painter.rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(0.5, egui::Color32::from_rgba_premultiplied(accent.r(), accent.g(), accent.b(), alpha / 2)),
            egui::StrokeKind::Outside,
        );

        // Left accent bar
        let accent_bar = egui::Rect::from_min_size(
            rect.left_top(),
            egui::vec2(3.0, toast_h),
        );
        painter.rect_filled(accent_bar, egui::CornerRadius { nw: 4, sw: 4, ne: 0, se: 0 }, accent);

        // Icon
        let text_alpha = egui::Color32::from_rgba_premultiplied(accent.r(), accent.g(), accent.b(), alpha);
        painter.text(
            egui::pos2(rect.left() + 12.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            icon,
            egui::FontId::proportional(12.0),
            text_alpha,
        );

        // Message text
        let msg_color = egui::Color32::from_rgba_premultiplied(200, 205, 215, alpha);
        let msg_galley = painter.layout_no_wrap(
            toast.message.clone(),
            egui::FontId::proportional(11.0),
            msg_color,
        );
        let max_text_w = toast_w - 32.0;
        let clipped = if msg_galley.size().x > max_text_w {
            // Truncate with ellipsis
            let mut truncated = toast.message.clone();
            while truncated.len() > 3 {
                truncated.pop();
                let test = painter.layout_no_wrap(
                    format!("{truncated}\u{2026}"),
                    egui::FontId::proportional(11.0),
                    msg_color,
                );
                if test.size().x <= max_text_w {
                    painter.galley(
                        egui::pos2(rect.left() + 26.0, rect.center().y - test.size().y * 0.5),
                        test,
                        msg_color,
                    );
                    break;
                }
            }
            true
        } else {
            false
        };
        if !clipped {
            painter.galley(
                egui::pos2(rect.left() + 26.0, rect.center().y - msg_galley.size().y * 0.5),
                msg_galley,
                msg_color,
            );
        }
    }
}
