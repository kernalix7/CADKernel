use super::{GuiState, ViewportInfo};
use crate::render::Projection;
use cadkernel_io::Mesh;

pub(crate) fn draw_status_bar(
    ctx: &egui::Context,
    gui: &GuiState,
    vp: &ViewportInfo<'_>,
    mesh: &Option<Mesh>,
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

            // Right: mesh info + display mode + projection
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
                if let Some(mesh) = mesh {
                    ui.label(format!(
                        "{}V: {} | T: {} | {} | {}",
                        fps_tag,
                        mesh.vertices.len(),
                        mesh.triangle_count(),
                        dm_tag,
                        proj_tag,
                    ));
                } else {
                    ui.label(format!("{fps_tag}{dm_tag} | {proj_tag}"));
                }
            });
        });
    });
}
