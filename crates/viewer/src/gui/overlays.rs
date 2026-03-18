use super::GuiState;
use crate::render::{Camera, dot3};

pub(crate) fn draw_axes_overlay(ctx: &egui::Context, camera: &Camera) {
    let size = 45.0f32;
    let margin = 55.0f32;
    let center = egui::pos2(margin, ctx.screen_rect().bottom() - margin);

    let right = camera.screen_right();
    let up = camera.screen_up();

    let axes: [([f32; 3], egui::Color32, &str); 3] = [
        ([1.0, 0.0, 0.0], egui::Color32::from_rgb(200, 60, 60), "X"),
        ([0.0, 1.0, 0.0], egui::Color32::from_rgb(60, 200, 60), "Y"),
        ([0.0, 0.0, 1.0], egui::Color32::from_rgb(80, 80, 230), "Z"),
    ];

    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("axes_overlay"),
    ));

    painter.circle_filled(center, 3.0, egui::Color32::from_gray(120));

    for (axis, color, label) in &axes {
        let sx = dot3(*axis, right) * size;
        let sy = -dot3(*axis, up) * size;
        let end = egui::pos2(center.x + sx, center.y + sy);
        let neg_end = egui::pos2(center.x - sx, center.y - sy);
        let faded = egui::Color32::from_rgba_premultiplied(
            color.r() / 3,
            color.g() / 3,
            color.b() / 3,
            100,
        );
        painter.line_segment([center, neg_end], egui::Stroke::new(1.0, faded));
        painter.line_segment([center, end], egui::Stroke::new(2.0, *color));
        painter.text(
            end,
            egui::Align2::CENTER_CENTER,
            *label,
            egui::FontId::proportional(11.0),
            *color,
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
