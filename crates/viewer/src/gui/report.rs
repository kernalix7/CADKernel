use super::{GuiState, ReportLevel};

pub(crate) fn draw_report_panel(ctx: &egui::Context, gui: &mut GuiState) {
    if !gui.show_report_panel {
        return;
    }

    egui::TopBottomPanel::bottom("report_panel")
        .default_height(120.0)
        .resizable(true)
        .min_height(60.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.strong("Report View");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("Clear").clicked() {
                        gui.report_lines.clear();
                    }
                    ui.weak(format!("{} entries", gui.report_lines.len()));
                });
            });
            ui.separator();

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for (level, msg) in &gui.report_lines {
                        let color = match level {
                            ReportLevel::Info => ui.visuals().text_color(),
                            ReportLevel::Warning => egui::Color32::from_rgb(220, 180, 50),
                            ReportLevel::Error => egui::Color32::from_rgb(220, 60, 60),
                        };
                        let prefix = match level {
                            ReportLevel::Info => "",
                            ReportLevel::Warning => "[Warning] ",
                            ReportLevel::Error => "[Error] ",
                        };
                        ui.label(egui::RichText::new(format!("{prefix}{msg}")).color(color));
                    }
                });
        });
}
