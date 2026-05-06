//! Scene overlay — accumulates 2D-paintable primitives (polylines, points,
//! labels) in world coordinates so prior-phase wire/curve-output features
//! become visible on screen without a wire-rendering wgpu pipeline.
//!
//! Each entry is stored in world-space and projected at draw time via the
//! current `Camera::view_proj`, so the overlay survives camera moves,
//! orbit, zoom, and projection-mode changes without re-running the
//! dispatcher.
//!
//! Owned by `GuiState` as `scene_overlay`. Dispatcher helpers in `app.rs`
//! call `gui.scene_overlay.add_polyline / add_point / add_label / clear`
//! to populate it; `paint()` is invoked from `gui::draw_ui` after the
//! existing axis / measurement / techdraw overlays.

use crate::render::Camera;
use cadkernel_math::Point3;

/// A polyline in world coordinates (each consecutive pair becomes a stroke).
#[derive(Debug, Clone)]
pub(crate) struct OverlayPolyline {
    pub points: Vec<Point3>,
    pub color: [u8; 4],
    pub width: f32,
}

/// A single 3D point rendered as a filled circle on screen.
#[derive(Debug, Clone)]
pub(crate) struct OverlayPoint {
    pub position: Point3,
    pub color: [u8; 4],
    pub radius: f32,
}

/// A 3D-anchored text label rendered at the projected screen position.
#[derive(Debug, Clone)]
pub(crate) struct OverlayLabel {
    pub anchor: Point3,
    pub text: String,
    pub font_size: f32,
    pub color: [u8; 4],
}

/// Accumulator owned by `GuiState`. Dispatcher arms call `add_*` to enqueue
/// primitives; `paint()` projects + draws them via `egui::Painter`.
#[derive(Debug, Clone, Default)]
pub(crate) struct SceneOverlay {
    pub polylines: Vec<OverlayPolyline>,
    pub points: Vec<OverlayPoint>,
    pub labels: Vec<OverlayLabel>,
}

impl SceneOverlay {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_polyline(&mut self, points: Vec<Point3>, color: [u8; 4], width: f32) {
        if points.len() >= 2 {
            self.polylines.push(OverlayPolyline {
                points,
                color,
                width,
            });
        }
    }

    pub fn add_point(&mut self, position: Point3, color: [u8; 4], radius: f32) {
        self.points.push(OverlayPoint {
            position,
            color,
            radius,
        });
    }

    pub fn add_label(
        &mut self,
        anchor: Point3,
        text: impl Into<String>,
        font_size: f32,
        color: [u8; 4],
    ) {
        self.labels.push(OverlayLabel {
            anchor,
            text: text.into(),
            font_size,
            color,
        });
    }

    pub fn clear(&mut self) {
        self.polylines.clear();
        self.points.clear();
        self.labels.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.polylines.is_empty() && self.points.is_empty() && self.labels.is_empty()
    }
}

/// Project a world-space point through the camera's view-projection matrix
/// to screen coordinates within `screen`. Returns `None` if behind camera.
fn world_to_screen(camera: &Camera, screen: egui::Rect, p: Point3) -> Option<egui::Pos2> {
    let vp = camera.view_proj();
    let pf = [p.x as f32, p.y as f32, p.z as f32];
    let cx = vp[0][0] * pf[0] + vp[1][0] * pf[1] + vp[2][0] * pf[2] + vp[3][0];
    let cy = vp[0][1] * pf[0] + vp[1][1] * pf[1] + vp[2][1] * pf[2] + vp[3][1];
    let cw = vp[0][3] * pf[0] + vp[1][3] * pf[1] + vp[2][3] * pf[2] + vp[3][3];
    if cw <= 1e-7 {
        return None;
    }
    let ndc_x = cx / cw;
    let ndc_y = cy / cw;
    let sx = screen.min.x + (ndc_x + 1.0) * 0.5 * screen.width();
    let sy = screen.min.y + (1.0 - ndc_y) * 0.5 * screen.height();
    Some(egui::pos2(sx, sy))
}

fn rgba(c: [u8; 4]) -> egui::Color32 {
    egui::Color32::from_rgba_premultiplied(c[0], c[1], c[2], c[3])
}

/// Paint all overlay entries via a foreground egui layer over the wgpu
/// viewport. Called from `gui::draw_ui` after the axis / techdraw overlays
/// so dispatched-curve output renders on top of the 3D scene.
pub(crate) fn paint(ctx: &egui::Context, camera: &Camera, overlay: &SceneOverlay) {
    if overlay.is_empty() {
        return;
    }
    let screen = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("scene_overlay"),
    ));

    for line in &overlay.polylines {
        let mut prev: Option<egui::Pos2> = None;
        for &p in &line.points {
            let cur = world_to_screen(camera, screen, p);
            if let (Some(a), Some(b)) = (prev, cur) {
                painter.line_segment([a, b], egui::Stroke::new(line.width, rgba(line.color)));
            }
            prev = cur;
        }
    }

    for pt in &overlay.points {
        if let Some(sp) = world_to_screen(camera, screen, pt.position) {
            painter.circle_filled(sp, pt.radius, rgba(pt.color));
        }
    }

    for lbl in &overlay.labels {
        if let Some(sp) = world_to_screen(camera, screen, lbl.anchor) {
            painter.text(
                sp,
                egui::Align2::LEFT_TOP,
                &lbl.text,
                egui::FontId::proportional(lbl.font_size),
                rgba(lbl.color),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_overlay_is_empty() {
        let o = SceneOverlay::new();
        assert!(o.is_empty());
    }

    #[test]
    fn add_polyline_with_two_points_pushes_entry() {
        let mut o = SceneOverlay::new();
        o.add_polyline(
            vec![Point3::ORIGIN, Point3::new(1.0, 0.0, 0.0)],
            [255, 255, 255, 255],
            1.5,
        );
        assert_eq!(o.polylines.len(), 1);
    }

    #[test]
    fn add_polyline_with_one_point_is_rejected() {
        let mut o = SceneOverlay::new();
        o.add_polyline(vec![Point3::ORIGIN], [255, 0, 0, 255], 1.0);
        assert!(
            o.polylines.is_empty(),
            "single-point polyline must be rejected"
        );
    }

    #[test]
    fn add_point_and_label_push_entries() {
        let mut o = SceneOverlay::new();
        o.add_point(Point3::ORIGIN, [200, 50, 50, 255], 4.0);
        o.add_label(Point3::new(1.0, 0.0, 0.0), "X", 14.0, [255, 255, 255, 255]);
        assert_eq!(o.points.len(), 1);
        assert_eq!(o.labels.len(), 1);
    }

    #[test]
    fn clear_removes_all_entries() {
        let mut o = SceneOverlay::new();
        o.add_polyline(
            vec![Point3::ORIGIN, Point3::new(1.0, 0.0, 0.0)],
            [255, 255, 255, 255],
            1.0,
        );
        o.add_point(Point3::ORIGIN, [255, 0, 0, 255], 3.0);
        o.add_label(Point3::ORIGIN, "P", 12.0, [200, 200, 200, 255]);
        o.clear();
        assert!(o.is_empty());
    }
}
