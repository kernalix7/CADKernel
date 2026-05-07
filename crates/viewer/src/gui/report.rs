use super::theme;
use super::{GuiState, ReportLevel};

/// Bottom panel tab state.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum BottomTab {
    Report,
    Console,
    History,
    Lua,
}

pub(crate) fn draw_report_panel(ctx: &egui::Context, gui: &mut GuiState) {
    if !gui.show_report_panel {
        return;
    }

    egui::TopBottomPanel::bottom("report_panel")
        .default_height(140.0)
        .resizable(true)
        .min_height(60.0)
        .frame(egui::Frame {
            fill: egui::Color32::from_rgb(0x1A, 0x1E, 0x26),
            inner_margin: egui::Margin::ZERO,
            stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(0x0F, 0x12, 0x18)),
            ..egui::Frame::NONE
        })
        .show(ctx, |ui| {
            // Tab bar with underline-style active indicator
            let tab_bar_rect = ui.available_rect_before_wrap();
            let tab_h = 26.0;
            let tab_bar = egui::Rect::from_min_size(
                tab_bar_rect.min,
                egui::vec2(tab_bar_rect.width(), tab_h),
            );
            ui.painter()
                .rect_filled(tab_bar, 0.0, egui::Color32::from_rgb(26, 28, 32));
            // Top edge line
            ui.painter().line_segment(
                [tab_bar.left_top(), tab_bar.right_top()],
                egui::Stroke::new(1.0, egui::Color32::from_rgb(56, 60, 70)),
            );

            let (tab_rect, _) = ui.allocate_exact_size(
                egui::vec2(tab_bar_rect.width(), tab_h),
                egui::Sense::hover(),
            );
            let tabs = [
                (BottomTab::Report, "Report"),
                (BottomTab::Console, "Python Console"),
                (BottomTab::Lua, "Lua"),
                (BottomTab::History, "History"),
            ];

            let tab_font = egui::FontId::new(11.0, egui::FontFamily::Proportional);
            let mut x = tab_rect.left() + 8.0;
            let accent = theme::COLOR_ACCENT;

            for (tab_id, label) in &tabs {
                let is_active = gui.bottom_tab == *tab_id;
                let text_color = if is_active {
                    egui::Color32::from_rgb(200, 205, 215)
                } else {
                    egui::Color32::from_rgb(120, 125, 135)
                };

                let galley =
                    ui.painter()
                        .layout_no_wrap(label.to_string(), tab_font.clone(), text_color);
                let tw = galley.size().x;
                let tab_w = tw + 16.0;
                let tab_area = egui::Rect::from_min_size(
                    egui::pos2(x, tab_rect.top()),
                    egui::vec2(tab_w, tab_h),
                );
                let resp = ui.interact(tab_area, ui.id().with(*label), egui::Sense::click());
                if resp.clicked() {
                    gui.bottom_tab = *tab_id;
                }

                // Hover bg
                if resp.hovered() && !is_active {
                    ui.painter()
                        .rect_filled(tab_area, 0.0, egui::Color32::from_rgb(40, 43, 50));
                }

                ui.painter().galley(
                    egui::pos2(x + 8.0, tab_rect.center().y - galley.size().y * 0.5),
                    galley,
                    text_color,
                );

                // Active underline
                if is_active {
                    ui.painter().line_segment(
                        [
                            egui::pos2(x, tab_rect.bottom() - 2.0),
                            egui::pos2(x + tab_w, tab_rect.bottom() - 2.0),
                        ],
                        egui::Stroke::new(2.0, accent),
                    );
                }

                x += tab_w + 2.0;
            }

            // Right-side controls
            let right_x = tab_rect.right() - 8.0;
            let cy = tab_rect.center().y;
            match gui.bottom_tab {
                BottomTab::Report => {
                    let info_count = gui
                        .report_lines
                        .iter()
                        .filter(|(l, _)| *l == ReportLevel::Info)
                        .count();
                    let warn_count = gui
                        .report_lines
                        .iter()
                        .filter(|(l, _)| *l == ReportLevel::Warning)
                        .count();
                    let err_count = gui
                        .report_lines
                        .iter()
                        .filter(|(l, _)| *l == ReportLevel::Error)
                        .count();

                    let summary = if err_count > 0 {
                        format!("{info_count}  \u{26A0}{warn_count}  \u{2716}{err_count}")
                    } else if warn_count > 0 {
                        format!("{info_count}  \u{26A0}{warn_count}")
                    } else {
                        format!("{info_count} entries")
                    };
                    ui.painter().text(
                        egui::pos2(right_x, cy),
                        egui::Align2::RIGHT_CENTER,
                        &summary,
                        egui::FontId::proportional(10.0),
                        theme::COLOR_DIM,
                    );
                }
                BottomTab::History => {
                    let total = gui.history_entries.len() + gui.future_entries.len();
                    ui.painter().text(
                        egui::pos2(right_x, cy),
                        egui::Align2::RIGHT_CENTER,
                        format!("{total} ops"),
                        egui::FontId::proportional(10.0),
                        theme::COLOR_DIM,
                    );
                }
                BottomTab::Lua => {
                    let count = gui.lua_history.len();
                    ui.painter().text(
                        egui::pos2(right_x, cy),
                        egui::Align2::RIGHT_CENTER,
                        format!("{count} cmds"),
                        egui::FontId::proportional(10.0),
                        theme::COLOR_DIM,
                    );
                }
                _ => {}
            }

            // Bottom line under tab bar
            ui.painter().line_segment(
                [
                    egui::pos2(tab_rect.left(), tab_rect.bottom()),
                    egui::pos2(tab_rect.right(), tab_rect.bottom()),
                ],
                egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 43, 50)),
            );

            // Content with padding
            ui.add_space(2.0);
            egui::Frame::NONE
                .inner_margin(egui::Margin::symmetric(6, 2))
                .show(ui, |ui| match gui.bottom_tab {
                    BottomTab::Report => draw_report_content(ui, gui),
                    BottomTab::Console => draw_console_content(ui, gui),
                    BottomTab::History => draw_history_content(ui, gui),
                    BottomTab::Lua => draw_lua_content(ui, gui),
                });
        });
}

fn draw_report_content(ui: &mut egui::Ui, gui: &GuiState) {
    // Severity filter toggles (using temp data for filter state)
    let filter_id = egui::Id::new("report_filters");
    let (show_info, show_warn, show_err) =
        ui.data_mut(|d| *d.get_temp_mut_or(filter_id, (true, true, true)));

    ui.horizontal(|ui| {
        let mut si = show_info;
        let mut sw = show_warn;
        let mut se = show_err;

        let info_color = if si {
            ui.visuals().text_color()
        } else {
            theme::COLOR_DIM
        };
        if ui
            .add(egui::Button::new(
                egui::RichText::new("Info").size(10.0).color(info_color),
            ))
            .clicked()
        {
            si = !si;
        }

        let warn_color = if sw {
            egui::Color32::from_rgb(220, 180, 50)
        } else {
            theme::COLOR_DIM
        };
        if ui
            .add(egui::Button::new(
                egui::RichText::new("Warn").size(10.0).color(warn_color),
            ))
            .clicked()
        {
            sw = !sw;
        }

        let err_color = if se {
            egui::Color32::from_rgb(220, 60, 60)
        } else {
            theme::COLOR_DIM
        };
        if ui
            .add(egui::Button::new(
                egui::RichText::new("Error").size(10.0).color(err_color),
            ))
            .clicked()
        {
            se = !se;
        }

        ui.data_mut(|d| {
            d.insert_temp(filter_id, (si, sw, se));
        });
    });

    let (show_info, show_warn, show_err) =
        ui.data_mut(|d| *d.get_temp_mut_or(filter_id, (true, true, true)));

    ui.add_space(2.0);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show(ui, |ui| {
            for (idx, (level, msg)) in gui.report_lines.iter().enumerate() {
                // Apply severity filter
                match level {
                    ReportLevel::Info if !show_info => continue,
                    ReportLevel::Warning if !show_warn => continue,
                    ReportLevel::Error if !show_err => continue,
                    _ => {}
                }

                let (icon, color, badge_bg) = match level {
                    ReportLevel::Info => (
                        "\u{2139}",
                        egui::Color32::from_rgb(170, 175, 190),
                        egui::Color32::from_rgba_premultiplied(60, 100, 180, 30),
                    ),
                    ReportLevel::Warning => (
                        "\u{26A0}",
                        egui::Color32::from_rgb(220, 180, 50),
                        egui::Color32::from_rgba_premultiplied(180, 140, 30, 30),
                    ),
                    ReportLevel::Error => (
                        "\u{2716}",
                        egui::Color32::from_rgb(220, 60, 60),
                        egui::Color32::from_rgba_premultiplied(180, 40, 40, 30),
                    ),
                };

                let timestamp = format!("{:04}", idx + 1);

                // Row with subtle background for warnings/errors
                let row_rect = ui.available_rect_before_wrap();
                let row_h = 16.0;
                let bg_rect = egui::Rect::from_min_size(
                    egui::pos2(row_rect.left(), row_rect.top()),
                    egui::vec2(row_rect.width(), row_h),
                );
                if !matches!(level, ReportLevel::Info) {
                    ui.painter().rect_filled(bg_rect, 0.0, badge_bg);
                }

                // Collapsible long messages (>80 chars)
                if msg.len() > 80 {
                    let short = &msg[..77];
                    let expand_id = egui::Id::new(("report_expand", idx));
                    let expanded = ui.data_mut(|d| *d.get_temp_mut_or(expand_id, false));
                    let display_msg = if expanded { msg.as_str() } else { short };
                    let suffix = if expanded { "" } else { "..." };

                    let resp = ui
                        .horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(&timestamp)
                                    .size(10.0)
                                    .color(theme::COLOR_DIM)
                                    .monospace(),
                            );
                            ui.label(egui::RichText::new(icon).size(10.0).color(color));
                            ui.label(
                                egui::RichText::new(format!("{display_msg}{suffix}"))
                                    .size(11.0)
                                    .color(color),
                            );
                        })
                        .response;
                    if resp.interact(egui::Sense::click()).clicked() {
                        ui.data_mut(|d| d.insert_temp(expand_id, !expanded));
                    }
                } else {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(&timestamp)
                                .size(10.0)
                                .color(theme::COLOR_DIM)
                                .monospace(),
                        );
                        ui.label(egui::RichText::new(icon).size(10.0).color(color));
                        ui.label(egui::RichText::new(msg).size(11.0).color(color));
                    });
                }
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
                let color = if line.starts_with(">>>") {
                    egui::Color32::from_rgb(100, 200, 100)
                } else if line.starts_with("Error") {
                    egui::Color32::from_rgb(220, 60, 60)
                } else {
                    ui.visuals().text_color()
                };
                ui.label(
                    egui::RichText::new(line)
                        .color(color)
                        .monospace()
                        .size(11.0),
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
                gui.console_history.push(format!("(not connected) {cmd}"));
                gui.console_input.clear();
            }
            response.request_focus();
        }
    });
}

fn draw_history_content(ui: &mut egui::Ui, gui: &mut GuiState) {
    use super::GuiAction;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show(ui, |ui| {
            if gui.history_entries.is_empty() && gui.future_entries.is_empty() {
                ui.weak("No operations recorded yet.");
                return;
            }

            // Show history (past operations, oldest first)
            for (i, desc) in gui.history_entries.iter().enumerate() {
                let idx = i + 1;
                let icon = history_icon(desc);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("{idx:>3}."))
                            .size(10.0)
                            .color(theme::COLOR_DIM)
                            .monospace(),
                    );
                    ui.label(
                        egui::RichText::new(icon)
                            .size(10.0)
                            .color(theme::COLOR_ACCENT),
                    );
                    ui.label(
                        egui::RichText::new(desc)
                            .size(11.0)
                            .color(egui::Color32::from_rgb(170, 175, 185)),
                    );
                });
            }

            // Current position marker
            let current_idx = gui.history_entries.len();
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("  \u{25B6}")
                        .size(11.0)
                        .color(egui::Color32::from_rgb(50, 200, 100)),
                );
                ui.label(
                    egui::RichText::new(format!("Current state ({current_idx} operations)"))
                        .size(11.0)
                        .strong()
                        .color(egui::Color32::from_rgb(80, 200, 120)),
                );
            });

            // Show future (redo stack, dimmed)
            for (i, desc) in gui.future_entries.iter().enumerate() {
                let idx = current_idx + i + 1;
                let icon = history_icon(desc);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("{idx:>3}."))
                            .size(10.0)
                            .color(egui::Color32::from_gray(60))
                            .monospace(),
                    );
                    ui.label(
                        egui::RichText::new(icon)
                            .size(10.0)
                            .color(egui::Color32::from_gray(70)),
                    );
                    ui.label(
                        egui::RichText::new(format!("{desc} (undone)"))
                            .size(11.0)
                            .color(theme::COLOR_DIM),
                    );
                });
            }
        });

    // Undo/Redo buttons at bottom
    ui.horizontal(|ui| {
        let can_undo = !gui.history_entries.is_empty();
        let can_redo = !gui.future_entries.is_empty();
        if ui
            .add_enabled(can_undo, egui::Button::new("Undo"))
            .clicked()
        {
            gui.actions.push(GuiAction::Undo);
        }
        if ui
            .add_enabled(can_redo, egui::Button::new("Redo"))
            .clicked()
        {
            gui.actions.push(GuiAction::Redo);
        }
    });
}

fn history_icon(desc: &str) -> &'static str {
    let lower = desc.to_lowercase();
    if lower.contains("create") || lower.contains("add") || lower.contains("make") {
        "\u{2795}" // plus
    } else if lower.contains("delete") || lower.contains("remove") {
        "\u{2796}" // minus
    } else if lower.contains("move") || lower.contains("translate") {
        "\u{2192}" // right arrow
    } else if lower.contains("rotate") {
        "\u{21BB}" // clockwise arrow
    } else if lower.contains("scale") {
        "\u{2922}" // NE arrow
    } else if lower.contains("boolean") || lower.contains("union") || lower.contains("subtract") {
        "\u{222A}" // union
    } else if lower.contains("fillet") || lower.contains("chamfer") {
        "\u{25D5}" // circle segment
    } else if lower.contains("extrude") || lower.contains("pad") {
        "\u{2B06}" // up arrow
    } else if lower.contains("revolve") {
        "\u{21BB}" // clockwise arrow
    } else if lower.contains("color") || lower.contains("appearance") {
        "\u{25CF}" // filled circle
    } else if lower.contains("import") {
        "\u{1F4C2}" // folder
    } else if lower.contains("rename") {
        "\u{270E}" // pencil
    } else {
        "\u{25CB}" // empty circle
    }
}

fn draw_lua_content(ui: &mut egui::Ui, gui: &mut GuiState) {
    use super::GuiAction;

    // History display
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .max_height(ui.available_height() - 28.0)
        .show(ui, |ui| {
            if gui.lua_history.is_empty() {
                ui.weak("CADKernel Lua Console");
                ui.weak("Type Lua commands below. Use cad.* for modeling operations.");
            }
            for (input, output, is_error) in &gui.lua_history {
                // Input line in green
                ui.label(
                    egui::RichText::new(format!("> {input}"))
                        .color(egui::Color32::from_rgb(100, 200, 100))
                        .monospace()
                        .size(11.0),
                );
                // Output: red if error, normal otherwise
                if !output.is_empty() {
                    let color = if *is_error {
                        egui::Color32::from_rgb(220, 60, 60)
                    } else {
                        ui.visuals().text_color()
                    };
                    ui.label(
                        egui::RichText::new(output)
                            .color(color)
                            .monospace()
                            .size(11.0),
                    );
                }
            }
        });

    // Input line
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(">")
                .color(egui::Color32::from_rgb(100, 200, 100))
                .monospace(),
        );
        let response = ui.add(
            egui::TextEdit::singleline(&mut gui.lua_input)
                .font(egui::TextStyle::Monospace)
                .desired_width(ui.available_width() - 50.0),
        );
        let enter_pressed = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let run_clicked = ui.button("Run").clicked();
        if enter_pressed || run_clicked {
            let cmd = gui.lua_input.trim().to_string();
            if !cmd.is_empty() {
                gui.actions.push(GuiAction::ExecuteLuaCode(cmd));
                gui.lua_input.clear();
            }
            response.request_focus();
        }
    });
}
