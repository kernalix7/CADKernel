use super::{GuiState, ReportLevel};

/// Bottom panel tab state.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum BottomTab {
    Report,
    Console,
}

pub(crate) fn draw_report_panel(ctx: &egui::Context, gui: &mut GuiState) {
    if !gui.show_report_panel {
        return;
    }

    egui::TopBottomPanel::bottom("report_panel")
        .default_height(140.0)
        .resizable(true)
        .min_height(60.0)
        .show(ctx, |ui| {
            // Tab bar
            ui.horizontal(|ui| {
                let report_sel = gui.bottom_tab == BottomTab::Report;
                if ui.selectable_label(report_sel, "\u{1F4CB} Report View").clicked() {
                    gui.bottom_tab = BottomTab::Report;
                }
                let console_sel = gui.bottom_tab == BottomTab::Console;
                if ui.selectable_label(console_sel, "\u{1F4BB} Python Console").clicked() {
                    gui.bottom_tab = BottomTab::Console;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    match gui.bottom_tab {
                        BottomTab::Report => {
                            if ui.small_button("Clear").clicked() {
                                gui.report_lines.clear();
                            }
                            ui.weak(format!("{} entries", gui.report_lines.len()));
                        }
                        BottomTab::Console => {
                            if ui.small_button("Clear").clicked() {
                                gui.console_history.clear();
                            }
                        }
                    }
                });
            });
            ui.separator();

            match gui.bottom_tab {
                BottomTab::Report => draw_report_content(ui, gui),
                BottomTab::Console => draw_console_content(ui, gui),
            }
        });
}

fn draw_report_content(ui: &mut egui::Ui, gui: &GuiState) {
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
                    ReportLevel::Warning => "\u{26A0} ",
                    ReportLevel::Error => "\u{274C} ",
                };
                ui.label(egui::RichText::new(format!("{prefix}{msg}")).color(color));
            }
        });
}

fn draw_console_content(ui: &mut egui::Ui, gui: &mut GuiState) {
    // History display
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .max_height(ui.available_height() - 28.0)
        .show(ui, |ui| {
            if gui.console_history.is_empty() {
                ui.weak("CADKernel Python Console");
                ui.weak("Type Python commands below. (PyO3 backend not yet connected)");
            }
            for line in &gui.console_history {
                let (prefix, color) = if line.starts_with(">>>") {
                    ("", egui::Color32::from_rgb(100, 200, 100))
                } else if line.starts_with("Error") {
                    ("", egui::Color32::from_rgb(220, 60, 60))
                } else {
                    ("", ui.visuals().text_color())
                };
                ui.label(
                    egui::RichText::new(format!("{prefix}{line}"))
                        .color(color)
                        .monospace(),
                );
            }
        });

    // Input line
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(">>>")
                .color(egui::Color32::from_rgb(100, 200, 100))
                .monospace(),
        );
        let response = ui.text_edit_singleline(&mut gui.console_input);
        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            let cmd = gui.console_input.trim().to_string();
            if !cmd.is_empty() {
                gui.console_history.push(format!(">>> {cmd}"));
                // Placeholder: echo the command (PyO3 integration future)
                gui.console_history.push(format!("(not connected) {cmd}"));
                gui.console_input.clear();
            }
            response.request_focus();
        }
    });
}
