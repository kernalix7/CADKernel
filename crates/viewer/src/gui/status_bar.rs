use super::{GuiState, ViewportInfo};
use crate::render::Projection;
use crate::scene::Scene;

pub(crate) fn draw_status_bar(
    ctx: &egui::Context,
    gui: &GuiState,
    vp: &ViewportInfo<'_>,
    scene: &Scene,
) {
    egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            // Left: status message + mouse coords
            if let Some(pos) = gui.mouse_world_pos {
                ui.weak(format!(
                    "X:{:.2} Y:{:.2} Z:{:.2}",
                    pos[0], pos[1], pos[2]
                ));
                ui.separator();
            }
            ui.label(&gui.status_message);

            // Right: scene info + display mode + projection + FPS
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let proj_tag = match vp.camera.projection {
                    Projection::Perspective => "Persp",
                    Projection::Orthographic => "Ortho",
                };
                let dm_tag = vp.display_mode.label();
                let fps_tag = if vp.show_fps {
                    format!("{:.0} FPS | ", vp.fps)
                } else {
                    String::new()
                };

                // Scene stats
                let n_obj = scene.len();
                let n_vis = scene.visible_objects().count();
                let total_tri: usize = scene.visible_objects().map(|o| o.mesh.triangle_count()).sum();

                let sel_info = if let Some(obj) = scene.selected_object() {
                    format!(" | Sel: {}", obj.name)
                } else {
                    String::new()
                };

                ui.label(format!(
                    "{fps_tag}Obj: {n_obj} ({n_vis} vis) | \u{25B3} {total_tri}{sel_info} | {dm_tag} | {proj_tag}"
                ));
            });
        });
    });
}
