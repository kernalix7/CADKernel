use super::{GuiAction, GuiState, SelectedEntity};
use super::theme;
use crate::scene::{CreationParams, Scene};

/// Property panel tab state.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PropertyTab {
    Data,
    View,
}

/// Standalone panel version (deprecated -- kept for compatibility).
#[allow(dead_code)]
pub(crate) fn draw_properties(
    _ctx: &egui::Context,
    _gui: &mut GuiState,
    _scene: &Scene,
) {
    // Now drawn inline inside ComboView
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Small dimmed unit label placed after a value.
fn unit_label(ui: &mut egui::Ui, unit: &str) {
    if !unit.is_empty() {
        ui.label(egui::RichText::new(unit).size(9.5).color(theme::COLOR_DIM));
    }
}

/// Read-only property row: label (dimmed) | value (dimmed) | unit.
fn readonly_row(ui: &mut egui::Ui, label: &str, value: &str, unit: &str) {
    ui.label(egui::RichText::new(format!("{label}:")).size(11.0).color(theme::COLOR_DIM));
    ui.label(egui::RichText::new(value).size(11.0).color(egui::Color32::from_rgb(155, 160, 170)));
    unit_label(ui, unit);
    ui.end_row();
}

/// Read-only property row with a numeric f64 formatted to `decimals` places.
fn readonly_f64_row(ui: &mut egui::Ui, label: &str, val: f64, decimals: usize, unit: &str) {
    readonly_row(ui, label, &format!("{val:.decimals$}"), unit);
}

/// Case-insensitive substring match for property filter.
fn matches_filter(text: &str, filter: &str) -> bool {
    if filter.is_empty() {
        return true;
    }
    text.to_ascii_lowercase().contains(&filter.to_ascii_lowercase())
}

/// Returns true if any of the given labels match the filter.
fn any_label_matches(labels: &[&str], filter: &str) -> bool {
    if filter.is_empty() {
        return true;
    }
    labels.iter().any(|l| matches_filter(l, filter))
}

// ---------------------------------------------------------------------------
// Search bar
// ---------------------------------------------------------------------------

fn draw_search_bar(ui: &mut egui::Ui) -> String {
    let filter_id = egui::Id::new("prop_search_filter");
    let mut filter: String = ui.data_mut(|d| d.get_temp(filter_id).unwrap_or_default());

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("\u{1F50D}").size(11.0).color(theme::COLOR_DIM));
        let resp = ui.add(
            egui::TextEdit::singleline(&mut filter)
                .desired_width(ui.available_width() - 22.0)
                .hint_text("Filter properties...")
                .font(egui::TextStyle::Small),
        );
        if !filter.is_empty()
            && ui
                .add(egui::Button::new(egui::RichText::new("\u{2715}").size(11.0)).small())
                .on_hover_text("Clear filter")
                .clicked()
        {
            filter.clear();
            resp.request_focus();
        }
    });
    ui.add_space(2.0);

    ui.data_mut(|d| d.insert_temp(filter_id, filter.clone()));
    filter
}

// ---------------------------------------------------------------------------
// Collapsible group helper
// ---------------------------------------------------------------------------

fn collapsible_group(
    ui: &mut egui::Ui,
    label: &str,
    default_open: bool,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    // FreeCAD-style section header with background bar
    if theme::draw_section_header(ui, label, label, default_open) {
        ui.add_space(2.0);
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(4, 2))
            .show(ui, |ui| {
                add_contents(ui);
            });
        ui.add_space(2.0);
    }
}

// ---------------------------------------------------------------------------
// Inline entry point
// ---------------------------------------------------------------------------

/// Inline version -- draws properties content into an existing Ui.
pub(crate) fn draw_properties_inline(
    ui: &mut egui::Ui,
    gui: &mut GuiState,
    scene: &Scene,
) {
    // Underline-style Data/View tabs
    {
        let tab_h = 22.0;
        let avail_w = ui.available_width();
        let (tab_rect, _) = ui.allocate_exact_size(egui::vec2(avail_w, tab_h), egui::Sense::hover());
        let painter = ui.painter();
        painter.rect_filled(tab_rect, 0.0, egui::Color32::from_rgb(32, 35, 40));

        let accent = egui::Color32::from_rgb(0, 122, 204);
        let tabs = [
            (PropertyTab::Data, "Data"),
            (PropertyTab::View, "View"),
        ];
        let font = egui::FontId::new(11.0, egui::FontFamily::Proportional);
        let mut x = tab_rect.left() + 4.0;
        for (tab_id, label) in &tabs {
            let is_active = gui.property_tab == *tab_id;
            let color = if is_active {
                egui::Color32::from_rgb(200, 205, 215)
            } else {
                egui::Color32::from_rgb(120, 125, 135)
            };
            let galley = painter.layout_no_wrap(label.to_string(), font.clone(), color);
            let tw = galley.size().x + 14.0;
            let area = egui::Rect::from_min_size(egui::pos2(x, tab_rect.top()), egui::vec2(tw, tab_h));
            let resp = ui.interact(area, ui.id().with(*label), egui::Sense::click());
            if resp.clicked() {
                gui.property_tab = *tab_id;
            }
            if resp.hovered() && !is_active {
                painter.rect_filled(area, 0.0, egui::Color32::from_rgb(40, 43, 50));
            }
            painter.galley(
                egui::pos2(x + 7.0, tab_rect.center().y - galley.size().y * 0.5),
                galley,
                color,
            );
            if is_active {
                painter.line_segment(
                    [egui::pos2(x, tab_rect.bottom() - 2.0), egui::pos2(x + tw, tab_rect.bottom() - 2.0)],
                    egui::Stroke::new(2.0, accent),
                );
            }
            x += tw + 2.0;
        }
        // Bottom line
        painter.line_segment(
            [egui::pos2(tab_rect.left(), tab_rect.bottom()), egui::pos2(tab_rect.right(), tab_rect.bottom())],
            egui::Stroke::new(0.5, egui::Color32::from_rgb(50, 54, 62)),
        );
    }
    ui.add_space(2.0);

    let filter = draw_search_bar(ui);

    if let Some(obj) = scene.selected_object() {
        match gui.property_tab {
            PropertyTab::Data => draw_data_tab(ui, gui, obj, &filter),
            PropertyTab::View => draw_view_tab(ui, gui, obj, &filter),
        }
    } else {
        draw_scene_overview(ui, scene);
    }
}

// ---------------------------------------------------------------------------
// Data tab
// ---------------------------------------------------------------------------

fn draw_data_tab(
    ui: &mut egui::Ui,
    gui: &mut GuiState,
    obj: &crate::scene::SceneObject,
    filter: &str,
) {
    // Object name with type badge
    ui.horizontal(|ui| {
        if let Some(params) = &obj.params {
            let icon = theme::object_type_icon(Some(params));
            ui.label(egui::RichText::new(icon).size(13.0).color(theme::COLOR_ACCENT));
        }
        ui.strong(egui::RichText::new(&obj.name).size(12.5));
    });
    ui.add_space(3.0);

    // Base group
    if any_label_matches(&["Label", "Type", "ID"], filter) {
        collapsible_group(ui, "Base", true, |ui| {
            egui::Grid::new("base_grid").num_columns(3).spacing([10.0, 4.0]).show(ui, |ui| {
                if matches_filter("Label", filter) {
                    readonly_row(ui, "Label", &obj.name, "");
                }
                if let Some(params) = &obj.params {
                    if matches_filter("Type", filter) {
                        readonly_row(ui, "Type", params_type_name(params), "");
                    }
                }
                if matches_filter("ID", filter) {
                    readonly_row(ui, "ID", &format!("{}", obj.id), "");
                }
            });
        });
    }

    // Selected sub-element info (auto-pick: always show when entities are selected)
    if !gui.selected_entities.is_empty() {
        if gui.selected_entities.len() == 1 {
            draw_selected_element(ui, obj, &gui.selected_entities[0], filter);
        } else {
            draw_multi_selection_summary(ui, obj, &gui.selected_entities, filter);
        }
    }

    // Creation Parameters group
    if let Some(params) = &obj.params {
        let param_labels = creation_param_labels(params);
        if any_label_matches(&param_labels, filter) {
            collapsible_group(ui, "Creation Parameters", true, |ui| {
                draw_params_editor(ui, gui, obj.id, params, filter);
            });
        }
    }

    // Shape (Topology) group
    let shape_labels = ["Solids", "Shells", "Faces", "Edges", "Vertices", "Triangles", "Mesh Verts"];
    if any_label_matches(&shape_labels, filter) {
        collapsible_group(ui, "Shape", true, |ui| {
            let m = &obj.model;
            egui::Grid::new("topo_grid").num_columns(3).spacing([10.0, 4.0]).show(ui, |ui| {
                if matches_filter("Solids", filter) { readonly_row(ui, "Solids", &format!("{}", m.solids.len()), ""); }
                if matches_filter("Shells", filter) { readonly_row(ui, "Shells", &format!("{}", m.shells.len()), ""); }
                if matches_filter("Faces", filter) { readonly_row(ui, "Faces", &format!("{}", m.faces.len()), ""); }
                if matches_filter("Edges", filter) { readonly_row(ui, "Edges", &format!("{}", m.edges.len()), ""); }
                if matches_filter("Vertices", filter) { readonly_row(ui, "Vertices", &format!("{}", m.vertices.len()), ""); }
                if matches_filter("Triangles", filter) { readonly_row(ui, "Triangles", &format!("{}", obj.mesh.triangle_count()), ""); }
                if matches_filter("Mesh Verts", filter) { readonly_row(ui, "Mesh Verts", &format!("{}", obj.mesh.vertices.len()), ""); }
            });
        });
    }

    // Placement group
    let placement_labels = ["Position X", "Position Y", "Position Z", "Rotation X", "Rotation Y", "Rotation Z"];
    if any_label_matches(&placement_labels, filter) {
        collapsible_group(ui, "Placement", true, |ui| {
            let (cx, cy, cz) = mesh_center(&obj.mesh);
            ui.label(egui::RichText::new("Position").size(10.5).color(theme::COLOR_DIM).strong());
            egui::Grid::new("placement_pos_grid").num_columns(3).spacing([10.0, 4.0]).show(ui, |ui| {
                if matches_filter("Position X", filter) { readonly_f64_row(ui, "X", cx, 3, "mm"); }
                if matches_filter("Position Y", filter) { readonly_f64_row(ui, "Y", cy, 3, "mm"); }
                if matches_filter("Position Z", filter) { readonly_f64_row(ui, "Z", cz, 3, "mm"); }
            });
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Rotation").size(10.5).color(theme::COLOR_DIM).strong());
            egui::Grid::new("placement_rot_grid").num_columns(3).spacing([10.0, 4.0]).show(ui, |ui| {
                if matches_filter("Rotation X", filter) { readonly_f64_row(ui, "Angle X", 0.0, 1, "\u{00B0}"); }
                if matches_filter("Rotation Y", filter) { readonly_f64_row(ui, "Angle Y", 0.0, 1, "\u{00B0}"); }
                if matches_filter("Rotation Z", filter) { readonly_f64_row(ui, "Angle Z", 0.0, 1, "\u{00B0}"); }
            });
        });
    }

    // Transform group
    if any_label_matches(&["Move", "Rotate", "Scale"], filter) {
        collapsible_group(ui, "Transform", false, |ui| {
            draw_transform_actions(ui, gui, obj);
        });
    }

    // Computed Properties group
    let computed_labels = ["Volume", "Surface Area", "Center of Mass", "Centroid X", "Centroid Y", "Centroid Z"];
    if any_label_matches(&computed_labels, filter) {
        collapsible_group(ui, "Computed Properties", true, |ui| {
            let props = cadkernel_modeling::compute_mass_properties(&obj.mesh);
            egui::Grid::new("computed_grid").num_columns(3).spacing([10.0, 4.0]).show(ui, |ui| {
                if matches_filter("Volume", filter) { readonly_f64_row(ui, "Volume", props.volume, 4, "mm\u{00B3}"); }
                if matches_filter("Surface Area", filter) { readonly_f64_row(ui, "Surface Area", props.surface_area, 4, "mm\u{00B2}"); }
            });
            if any_label_matches(&["Center of Mass", "Centroid X", "Centroid Y", "Centroid Z"], filter) {
                ui.add_space(2.0);
                ui.label(egui::RichText::new("Center of Mass").size(10.5).color(theme::COLOR_DIM).strong());
                egui::Grid::new("com_grid").num_columns(3).spacing([10.0, 4.0]).show(ui, |ui| {
                    if matches_filter("Centroid X", filter) || matches_filter("Center of Mass", filter) { readonly_f64_row(ui, "X", props.centroid.x, 4, "mm"); }
                    if matches_filter("Centroid Y", filter) || matches_filter("Center of Mass", filter) { readonly_f64_row(ui, "Y", props.centroid.y, 4, "mm"); }
                    if matches_filter("Centroid Z", filter) || matches_filter("Center of Mass", filter) { readonly_f64_row(ui, "Z", props.centroid.z, 4, "mm"); }
                });
            }
        });
    }
}

// ---------------------------------------------------------------------------
// View tab
// ---------------------------------------------------------------------------

fn draw_view_tab(
    ui: &mut egui::Ui,
    gui: &mut GuiState,
    obj: &crate::scene::SceneObject,
    filter: &str,
) {
    use crate::render::DisplayMode;

    // Display group
    if any_label_matches(&["Display Mode", "Transparency"], filter) {
        collapsible_group(ui, "Display", true, |ui| {
            egui::Grid::new("display_grid").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
                if matches_filter("Display Mode", filter) {
                    ui.label(egui::RichText::new("Display Mode:").size(11.0).color(theme::COLOR_DIM));
                    let current_label = gui.display_mode_override.unwrap_or(DisplayMode::Shading).label();
                    egui::ComboBox::from_id_salt("display_mode_combo")
                        .selected_text(egui::RichText::new(current_label).size(11.0))
                        .width(100.0)
                        .show_ui(ui, |ui| {
                            for &mode in DisplayMode::ALL {
                                if ui.selectable_label(gui.display_mode_override == Some(mode), mode.label()).clicked() {
                                    gui.display_mode_override = Some(mode);
                                    gui.actions.push(GuiAction::SetDisplayMode(mode));
                                }
                            }
                        });
                    ui.end_row();
                }
                if matches_filter("Transparency", filter) {
                    ui.label(egui::RichText::new("Transparency:").size(11.0).color(theme::COLOR_DIM));
                    let mut pct = ((1.0 - obj.color[3]) * 100.0).round() as i32;
                    if ui.add(egui::Slider::new(&mut pct, 0..=90).suffix("%").step_by(5.0)).changed() {
                        let mut c = obj.color;
                        c[3] = 1.0 - (pct as f32 / 100.0);
                        gui.actions.push(GuiAction::SetObjectColor { id: obj.id, color: c });
                    }
                    ui.end_row();
                }
            });
        });
    }

    // Material presets group
    if matches_filter("Material", filter) {
        collapsible_group(ui, "Material", true, |ui| {
            draw_material_presets(ui, gui, obj);
        });
    }

    // Colors group
    if any_label_matches(&["Shape Color", "Face Color", "Presets", "Hex Color", "Line Color", "Point Color"], filter) {
        collapsible_group(ui, "Colors", true, |ui| {
            if any_label_matches(&["Shape Color", "Face Color"], filter) {
                egui::Grid::new("color_picker_grid").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
                    ui.label(egui::RichText::new("Face Color:").size(11.0).color(theme::COLOR_DIM));
                    let [r, g, b, a] = obj.color;
                    let mut rgba = egui::Rgba::from_rgba_premultiplied(r, g, b, a);
                    if egui::color_picker::color_edit_button_rgba(ui, &mut rgba, egui::color_picker::Alpha::Opaque).changed() {
                        gui.actions.push(GuiAction::SetObjectColor { id: obj.id, color: [rgba.r(), rgba.g(), rgba.b(), rgba.a()] });
                    }
                    ui.end_row();
                });
            }
            if matches_filter("Presets", filter) {
                ui.add_space(2.0);
                draw_color_presets(ui, gui, obj);
            }
            if matches_filter("Hex Color", filter) {
                ui.add_space(2.0);
                draw_hex_color_input(ui, gui, obj);
            }
            if matches_filter("Line Color", filter) {
                ui.add_space(2.0);
                egui::Grid::new("line_color_grid").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
                    ui.label(egui::RichText::new("Line Color:").size(11.0).color(theme::COLOR_DIM));
                    ui.label(egui::RichText::new("(same as face)").size(10.0).color(theme::COLOR_DIM));
                    ui.end_row();
                });
            }
            if matches_filter("Point Color", filter) {
                egui::Grid::new("point_color_grid").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
                    ui.label(egui::RichText::new("Point Color:").size(11.0).color(theme::COLOR_DIM));
                    ui.label(egui::RichText::new("(same as face)").size(10.0).color(theme::COLOR_DIM));
                    ui.end_row();
                });
            }
        });
    }

    // Line Style group
    if any_label_matches(&["Line Width", "Point Size"], filter) {
        collapsible_group(ui, "Line Style", false, |ui| {
            egui::Grid::new("line_style_grid").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
                if matches_filter("Line Width", filter) {
                    ui.label(egui::RichText::new("Line Width:").size(11.0).color(theme::COLOR_DIM));
                    let lw_id = egui::Id::new("line_width_display");
                    let mut lw = ui.data_mut(|d| *d.get_temp_mut_or(lw_id, 1.0_f32));
                    if ui.add(egui::DragValue::new(&mut lw).range(1.0..=10.0).speed(0.5).suffix(" px")).changed() {
                        ui.data_mut(|d| d.insert_temp(lw_id, lw));
                    }
                    ui.end_row();
                }
                if matches_filter("Point Size", filter) {
                    ui.label(egui::RichText::new("Point Size:").size(11.0).color(theme::COLOR_DIM));
                    let ps_id = egui::Id::new("point_size_display");
                    let mut ps = ui.data_mut(|d| *d.get_temp_mut_or(ps_id, 3.0_f32));
                    if ui.add(egui::DragValue::new(&mut ps).range(1.0..=20.0).speed(0.5).suffix(" px")).changed() {
                        ui.data_mut(|d| d.insert_temp(ps_id, ps));
                    }
                    ui.end_row();
                }
            });
        });
    }

    // Visibility group
    if any_label_matches(&["Visibility", "Selectable"], filter) {
        collapsible_group(ui, "Visibility", true, |ui| {
            egui::Grid::new("visibility_grid").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
                if matches_filter("Visibility", filter) {
                    ui.label(egui::RichText::new("Show/Hide:").size(11.0).color(theme::COLOR_DIM));
                    let vis_text = if obj.visible { "\u{25C9} Visible" } else { "\u{25CB} Hidden" };
                    if ui.selectable_label(obj.visible, egui::RichText::new(vis_text).size(11.0)).clicked() {
                        gui.actions.push(GuiAction::ToggleVisibility(obj.id));
                    }
                    ui.end_row();
                }
                if matches_filter("Selectable", filter) {
                    ui.label(egui::RichText::new("Selectable:").size(11.0).color(theme::COLOR_DIM));
                    let sel_text = if obj.selected { "\u{2713} Selected" } else { "Not selected" };
                    ui.label(egui::RichText::new(sel_text).size(11.0));
                    ui.end_row();
                }
            });
        });
    }
}

// ---------------------------------------------------------------------------
// Color presets
// ---------------------------------------------------------------------------

fn draw_color_presets(ui: &mut egui::Ui, gui: &mut GuiState, obj: &crate::scene::SceneObject) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Presets:").size(10.0).color(theme::COLOR_DIM));
        let presets: &[(&str, [f32; 4])] = &[
            ("Red", [0.90, 0.20, 0.20, 1.0]),
            ("Green", [0.20, 0.80, 0.20, 1.0]),
            ("Blue", [0.30, 0.50, 0.90, 1.0]),
            ("Yellow", [0.90, 0.90, 0.20, 1.0]),
            ("Orange", [0.90, 0.60, 0.20, 1.0]),
            ("Cyan", [0.20, 0.80, 0.80, 1.0]),
            ("Gray", [0.60, 0.60, 0.60, 1.0]),
            ("White", [0.95, 0.95, 0.95, 1.0]),
        ];
        for &(name, color) in presets {
            let c32 = egui::Color32::from_rgba_unmultiplied(
                (color[0] * 255.0) as u8, (color[1] * 255.0) as u8, (color[2] * 255.0) as u8, 255,
            );
            let swatch_size = egui::vec2(14.0, 14.0);
            let (rect, resp) = ui.allocate_exact_size(swatch_size, egui::Sense::click());
            ui.painter().rect_filled(rect, 2.0, c32);
            if obj.color[0..3] == color[0..3] {
                ui.painter().rect_stroke(rect, 2.0, egui::Stroke::new(1.5, egui::Color32::WHITE), egui::StrokeKind::Middle);
            }
            if resp.clicked() {
                gui.actions.push(GuiAction::SetObjectColor { id: obj.id, color });
            }
            if resp.hovered() { resp.on_hover_text(name); }
        }
    });
}

// ---------------------------------------------------------------------------
// Hex color input
// ---------------------------------------------------------------------------

fn draw_hex_color_input(ui: &mut egui::Ui, gui: &mut GuiState, obj: &crate::scene::SceneObject) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Hex:").size(10.0).color(theme::COLOR_DIM));
        let hex_id = egui::Id::new("hex_color_input");
        let current_hex = format!(
            "#{:02X}{:02X}{:02X}",
            (obj.color[0] * 255.0) as u8, (obj.color[1] * 255.0) as u8, (obj.color[2] * 255.0) as u8,
        );
        let mut hex_str: String = ui.data_mut(|d| d.get_temp(hex_id).unwrap_or(current_hex));
        let resp = ui.add(egui::TextEdit::singleline(&mut hex_str).desired_width(70.0).font(egui::TextStyle::Small));
        ui.data_mut(|d| d.insert_temp(hex_id, hex_str.clone()));
        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            if let Some(color) = parse_hex_color(&hex_str) {
                gui.actions.push(GuiAction::SetObjectColor { id: obj.id, color });
            }
        }
    });
}

fn parse_hex_color(s: &str) -> Option<[f32; 4]> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 { return None; }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0])
}

// ---------------------------------------------------------------------------
// Transform actions
// ---------------------------------------------------------------------------

fn draw_transform_actions(ui: &mut egui::Ui, gui: &mut GuiState, obj: &crate::scene::SceneObject) {
    let id = obj.id;
    ui.label(egui::RichText::new("Move:").size(11.0).color(theme::COLOR_DIM));
    ui.horizontal(|ui| {
        let step_id = egui::Id::new("move_step");
        let mut step = ui.data_mut(|d| *d.get_temp_mut_or(step_id, 10.0_f64));
        ui.add(egui::DragValue::new(&mut step).range(0.1..=1000.0).speed(0.5).suffix(" mm"));
        ui.data_mut(|d| d.insert_temp(step_id, step));
        if ui.small_button("+X").clicked() { gui.actions.push(GuiAction::MoveObject { id, dx: step, dy: 0.0, dz: 0.0 }); }
        if ui.small_button("+Y").clicked() { gui.actions.push(GuiAction::MoveObject { id, dx: 0.0, dy: step, dz: 0.0 }); }
        if ui.small_button("+Z").clicked() { gui.actions.push(GuiAction::MoveObject { id, dx: 0.0, dy: 0.0, dz: step }); }
    });
    ui.label(egui::RichText::new("Rotate:").size(11.0).color(theme::COLOR_DIM));
    ui.horizontal(|ui| {
        let angle_id = egui::Id::new("rotate_angle");
        let mut angle = ui.data_mut(|d| *d.get_temp_mut_or(angle_id, 90.0_f64));
        ui.add(egui::DragValue::new(&mut angle).range(0.1..=360.0).speed(1.0).suffix("\u{00B0}"));
        ui.data_mut(|d| d.insert_temp(angle_id, angle));
        if ui.small_button("X").clicked() { gui.actions.push(GuiAction::RotateObject { id, axis: 0, angle_deg: angle }); }
        if ui.small_button("Y").clicked() { gui.actions.push(GuiAction::RotateObject { id, axis: 1, angle_deg: angle }); }
        if ui.small_button("Z").clicked() { gui.actions.push(GuiAction::RotateObject { id, axis: 2, angle_deg: angle }); }
    });
    ui.label(egui::RichText::new("Scale:").size(11.0).color(theme::COLOR_DIM));
    ui.horizontal(|ui| {
        let scale_id = egui::Id::new("scale_factor");
        let mut factor = ui.data_mut(|d| *d.get_temp_mut_or(scale_id, 2.0_f64));
        ui.add(egui::DragValue::new(&mut factor).range(0.01..=100.0).speed(0.05).suffix("x"));
        ui.data_mut(|d| d.insert_temp(scale_id, factor));
        if ui.small_button("Apply").clicked() { gui.actions.push(GuiAction::ScaleObjectUniform { id, factor }); }
    });
}

// ---------------------------------------------------------------------------
// Sub-element properties
// ---------------------------------------------------------------------------

fn draw_selected_element(
    ui: &mut egui::Ui,
    obj: &crate::scene::SceneObject,
    entity: &SelectedEntity,
    filter: &str,
) {
    match entity {
        SelectedEntity::Face(face_h) => {
            let labels = ["Face", "Triangles", "Area", "Loops"];
            if any_label_matches(&labels, filter) {
                collapsible_group(ui, "Selected Face", true, |ui| {
                    egui::Grid::new("sel_face_grid").num_columns(3).spacing([10.0, 4.0]).show(ui, |ui| {
                        if matches_filter("Face", filter) {
                            readonly_row(ui, "Face", &format!("{:?}", face_h), "");
                        }
                        // Triangle count and area from face_tri_map
                        if let Some((_fh, start, count)) = obj.face_tri_map.iter().find(|(fh, _, _)| fh == face_h) {
                            if matches_filter("Triangles", filter) {
                                readonly_row(ui, "Triangles", &format!("{count}"), "");
                            }
                            if matches_filter("Area", filter) {
                                let area = compute_face_area(&obj.vertices, *start, *count);
                                readonly_f64_row(ui, "Area", area, 4, "mm\u{00B2}");
                            }
                        }
                        // Loop count from B-Rep model
                        if let Some(face) = obj.model.faces.get(*face_h) {
                            if matches_filter("Loops", filter) {
                                let loop_count = 1 + face.inner_loops.len();
                                readonly_row(ui, "Loops", &format!("{loop_count}"), "");
                            }
                        }
                    });
                });
            }
        }
        SelectedEntity::Edge(edge_h) => {
            let labels = ["Edge", "Start", "End", "Length"];
            if any_label_matches(&labels, filter) {
                collapsible_group(ui, "Selected Edge", true, |ui| {
                    egui::Grid::new("sel_edge_grid").num_columns(3).spacing([10.0, 4.0]).show(ui, |ui| {
                        if matches_filter("Edge", filter) {
                            readonly_row(ui, "Edge", &format!("{:?}", edge_h), "");
                        }
                        // Find edge positions
                        if let Some(idx) = obj.edge_handles.iter().position(|eh| eh == edge_h) {
                            let (start, end) = obj.edge_positions[idx];
                            if matches_filter("Start", filter) {
                                readonly_row(ui, "Start", &format!("({:.3}, {:.3}, {:.3})", start[0], start[1], start[2]), "mm");
                            }
                            if matches_filter("End", filter) {
                                readonly_row(ui, "End", &format!("({:.3}, {:.3}, {:.3})", end[0], end[1], end[2]), "mm");
                            }
                            if matches_filter("Length", filter) {
                                let dx = (end[0] - start[0]) as f64;
                                let dy = (end[1] - start[1]) as f64;
                                let dz = (end[2] - start[2]) as f64;
                                let len = (dx * dx + dy * dy + dz * dz).sqrt();
                                readonly_f64_row(ui, "Length", len, 4, "mm");
                            }
                        }
                    });
                });
            }
        }
        SelectedEntity::Vertex(vert_h) => {
            let labels = ["Vertex", "X", "Y", "Z"];
            if any_label_matches(&labels, filter) {
                collapsible_group(ui, "Selected Vertex", true, |ui| {
                    egui::Grid::new("sel_vert_grid").num_columns(3).spacing([10.0, 4.0]).show(ui, |ui| {
                        if matches_filter("Vertex", filter) {
                            readonly_row(ui, "Vertex", &format!("{:?}", vert_h), "");
                        }
                        if let Some(idx) = obj.vertex_handles.iter().position(|vh| vh == vert_h) {
                            let pos = obj.vertex_positions[idx];
                            if matches_filter("X", filter) { readonly_f64_row(ui, "X", pos[0] as f64, 6, "mm"); }
                            if matches_filter("Y", filter) { readonly_f64_row(ui, "Y", pos[1] as f64, 6, "mm"); }
                            if matches_filter("Z", filter) { readonly_f64_row(ui, "Z", pos[2] as f64, 6, "mm"); }
                        }
                    });
                });
            }
        }
        _ => {}
    }
}

fn draw_multi_selection_summary(
    ui: &mut egui::Ui,
    obj: &crate::scene::SceneObject,
    entities: &[SelectedEntity],
    filter: &str,
) {
    let mut face_count = 0usize;
    let mut edge_count = 0usize;
    let mut vert_count = 0usize;
    let mut total_face_area = 0.0_f64;
    let mut total_edge_length = 0.0_f64;

    for e in entities {
        match e {
            SelectedEntity::Face(fh) => {
                face_count += 1;
                if let Some((_fh, start, count)) = obj.face_tri_map.iter().find(|(f, _, _)| f == fh) {
                    total_face_area += compute_face_area(&obj.vertices, *start, *count);
                }
            }
            SelectedEntity::Edge(eh) => {
                edge_count += 1;
                if let Some(idx) = obj.edge_handles.iter().position(|e| e == eh) {
                    let (s, e) = obj.edge_positions[idx];
                    let dx = (e[0] - s[0]) as f64;
                    let dy = (e[1] - s[1]) as f64;
                    let dz = (e[2] - s[2]) as f64;
                    total_edge_length += (dx * dx + dy * dy + dz * dz).sqrt();
                }
            }
            SelectedEntity::Vertex(_) => { vert_count += 1; }
            _ => {}
        }
    }

    let labels = ["Selection", "Faces", "Edges", "Vertices", "Total Area", "Total Length"];
    if any_label_matches(&labels, filter) {
        collapsible_group(ui, &format!("Selection ({} items)", entities.len()), true, |ui| {
            egui::Grid::new("multi_sel_grid").num_columns(3).spacing([10.0, 4.0]).show(ui, |ui| {
                if face_count > 0 && matches_filter("Faces", filter) {
                    readonly_row(ui, "Faces", &format!("{face_count}"), "");
                }
                if edge_count > 0 && matches_filter("Edges", filter) {
                    readonly_row(ui, "Edges", &format!("{edge_count}"), "");
                }
                if vert_count > 0 && matches_filter("Vertices", filter) {
                    readonly_row(ui, "Vertices", &format!("{vert_count}"), "");
                }
                if face_count > 0 && matches_filter("Total Area", filter) {
                    readonly_f64_row(ui, "Total Area", total_face_area, 4, "mm\u{00B2}");
                }
                if edge_count > 0 && matches_filter("Total Length", filter) {
                    readonly_f64_row(ui, "Total Length", total_edge_length, 4, "mm");
                }
            });

            // Element listing with type badges
            ui.add_space(4.0);
            egui::CollapsingHeader::new(
                egui::RichText::new("Elements").size(11.0).color(theme::COLOR_DIM),
            )
            .default_open(false)
            .show(ui, |ui| {
                for (i, e) in entities.iter().enumerate() {
                    let (badge, desc) = match e {
                        SelectedEntity::Face(fh) => ("\u{25A0} F", format!("Face {:?}", fh)),
                        SelectedEntity::Edge(eh) => ("\u{2500} E", format!("Edge {:?}", eh)),
                        SelectedEntity::Vertex(vh) => ("\u{25CF} V", format!("Vertex {:?}", vh)),
                        _ => ("\u{25A1} S", format!("Entity {i}")),
                    };
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(badge).size(10.0).color(theme::COLOR_ACCENT).strong());
                        ui.label(egui::RichText::new(desc).size(10.0).color(theme::COLOR_DIM));
                    });
                }
            });
        });
    }

    // Measurement between exactly two sub-elements
    if entities.len() == 2 {
        let meas_labels = ["Measurement", "Distance"];
        if any_label_matches(&meas_labels, filter) {
            if let Some(dist) = measurement_distance(obj, &entities[0], &entities[1]) {
                collapsible_group(ui, "Measurement", true, |ui| {
                    egui::Grid::new("measurement_grid").num_columns(3).spacing([10.0, 4.0]).show(ui, |ui| {
                        readonly_f64_row(ui, "Distance", dist, 4, "mm");
                    });
                });
            }
        }
    }
}

/// Compute the 3D Euclidean distance between the representative points of two
/// selected sub-elements. Returns `None` if a representative point cannot be
/// determined (e.g. Solid/Shell selections).
fn measurement_distance(
    obj: &crate::scene::SceneObject,
    a: &SelectedEntity,
    b: &SelectedEntity,
) -> Option<f64> {
    let pa = entity_representative_point(obj, a)?;
    let pb = entity_representative_point(obj, b)?;
    let dx = (pb[0] - pa[0]) as f64;
    let dy = (pb[1] - pa[1]) as f64;
    let dz = (pb[2] - pa[2]) as f64;
    Some((dx * dx + dy * dy + dz * dz).sqrt())
}

/// Return the representative 3D point for a sub-element:
/// - Vertex: vertex position
/// - Edge: midpoint
/// - Face: centroid of triangle vertices
fn entity_representative_point(
    obj: &crate::scene::SceneObject,
    entity: &SelectedEntity,
) -> Option<[f32; 3]> {
    match entity {
        SelectedEntity::Vertex(vh) => {
            let idx = obj.vertex_handles.iter().position(|h| h == vh)?;
            Some(obj.vertex_positions[idx])
        }
        SelectedEntity::Edge(eh) => {
            let idx = obj.edge_handles.iter().position(|h| h == eh)?;
            let (s, e) = obj.edge_positions[idx];
            Some([
                (s[0] + e[0]) * 0.5,
                (s[1] + e[1]) * 0.5,
                (s[2] + e[2]) * 0.5,
            ])
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

fn compute_face_area(vertices: &[crate::render::Vertex], start_tri: usize, tri_count: usize) -> f64 {
    let mut area = 0.0_f64;
    let base = start_tri * 3;
    for i in (base..base + tri_count * 3).step_by(3) {
        if i + 2 >= vertices.len() { break; }
        let p0 = vertices[i].position;
        let p1 = vertices[i + 1].position;
        let p2 = vertices[i + 2].position;
        // Cross product magnitude / 2
        let ax = (p1[0] - p0[0]) as f64;
        let ay = (p1[1] - p0[1]) as f64;
        let az = (p1[2] - p0[2]) as f64;
        let bx = (p2[0] - p0[0]) as f64;
        let by = (p2[1] - p0[1]) as f64;
        let bz = (p2[2] - p0[2]) as f64;
        let cx = ay * bz - az * by;
        let cy = az * bx - ax * bz;
        let cz = ax * by - ay * bx;
        area += (cx * cx + cy * cy + cz * cz).sqrt() * 0.5;
    }
    area
}

// ---------------------------------------------------------------------------
// Scene overview
// ---------------------------------------------------------------------------

fn draw_scene_overview(ui: &mut egui::Ui, scene: &Scene) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("\u{1F4C4}").size(13.0).color(theme::COLOR_ACCENT));
        ui.strong(egui::RichText::new("Scene Overview").size(12.5));
    });
    ui.add_space(4.0);
    let n_total = scene.len();
    let n_visible = scene.visible_objects().count();
    let (total_tris, total_verts) = scene.visible_objects().fold((0, 0), |(t, v), obj| {
        (t + obj.mesh.triangle_count(), v + obj.mesh.vertices.len())
    });
    egui::Grid::new("overview_grid").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
        ui.label(egui::RichText::new("Objects:").size(11.0).color(theme::COLOR_DIM));
        ui.label(egui::RichText::new(format!("{n_total}")).size(11.0));
        ui.end_row();
        ui.label(egui::RichText::new("Visible:").size(11.0).color(theme::COLOR_DIM));
        ui.label(egui::RichText::new(format!("{n_visible}")).size(11.0));
        ui.end_row();
        ui.label(egui::RichText::new("Triangles:").size(11.0).color(theme::COLOR_DIM));
        ui.label(egui::RichText::new(format_count(total_tris)).size(11.0));
        ui.end_row();
        ui.label(egui::RichText::new("Vertices:").size(11.0).color(theme::COLOR_DIM));
        ui.label(egui::RichText::new(format_count(total_verts)).size(11.0));
        ui.end_row();
    });
    if scene.is_empty() {
        ui.add_space(8.0);
        ui.label(egui::RichText::new("No objects in scene.").size(11.0).color(theme::COLOR_DIM));
        ui.label(egui::RichText::new("Create or import a model to start.").size(10.5).color(theme::COLOR_DIM));
    }
}

fn format_count(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        format!("{n}")
    }
}

// ---------------------------------------------------------------------------
// Parameter editor
// ---------------------------------------------------------------------------

fn draw_params_editor(
    ui: &mut egui::Ui, gui: &mut GuiState,
    id: crate::scene::ObjectId, params: &CreationParams, filter: &str,
) {
    match params {
        CreationParams::Box { width, height, depth } => {
            let (mut w, mut h, mut d) = (*width, *height, *depth);
            let mut changed = false;
            egui::Grid::new("box_params").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
                if matches_filter("Width", filter) { param_row(ui, "Width", &mut w, 0.1..=1000.0, 0.1, &mut changed); }
                if matches_filter("Height", filter) { param_row(ui, "Height", &mut h, 0.1..=1000.0, 0.1, &mut changed); }
                if matches_filter("Depth", filter) { param_row(ui, "Depth", &mut d, 0.1..=1000.0, 0.1, &mut changed); }
            });
            if changed { gui.actions.push(GuiAction::RebuildObject { id, params: CreationParams::Box { width: w, height: h, depth: d } }); }
        }
        CreationParams::Cylinder { radius, height } => {
            let (mut r, mut h) = (*radius, *height);
            let mut changed = false;
            egui::Grid::new("cyl_params").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
                if matches_filter("Radius", filter) { param_row(ui, "Radius", &mut r, 0.1..=500.0, 0.1, &mut changed); }
                if matches_filter("Height", filter) { param_row(ui, "Height", &mut h, 0.1..=1000.0, 0.1, &mut changed); }
            });
            if changed { gui.actions.push(GuiAction::RebuildObject { id, params: CreationParams::Cylinder { radius: r, height: h } }); }
        }
        CreationParams::Sphere { radius } => {
            let mut r = *radius;
            let mut changed = false;
            egui::Grid::new("sph_params").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
                if matches_filter("Radius", filter) { param_row(ui, "Radius", &mut r, 0.1..=500.0, 0.1, &mut changed); }
            });
            if changed { gui.actions.push(GuiAction::RebuildObject { id, params: CreationParams::Sphere { radius: r } }); }
        }
        CreationParams::Cone { base_radius, top_radius, height } => {
            let (mut br, mut tr, mut h) = (*base_radius, *top_radius, *height);
            let mut changed = false;
            egui::Grid::new("cone_params").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
                if matches_filter("Base Radius", filter) { param_row(ui, "Base Radius", &mut br, 0.01..=500.0, 0.1, &mut changed); }
                if matches_filter("Top Radius", filter) { param_row(ui, "Top Radius", &mut tr, 0.0..=500.0, 0.1, &mut changed); }
                if matches_filter("Height", filter) { param_row(ui, "Height", &mut h, 0.1..=1000.0, 0.1, &mut changed); }
            });
            if changed { gui.actions.push(GuiAction::RebuildObject { id, params: CreationParams::Cone { base_radius: br, top_radius: tr, height: h } }); }
        }
        CreationParams::Torus { major_radius, minor_radius } => {
            let (mut mr, mut mnr) = (*major_radius, *minor_radius);
            let mut changed = false;
            egui::Grid::new("tor_params").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
                if matches_filter("Major Radius", filter) { param_row(ui, "Major Radius", &mut mr, 0.1..=500.0, 0.1, &mut changed); }
                if matches_filter("Minor Radius", filter) { param_row(ui, "Minor Radius", &mut mnr, 0.01..=200.0, 0.05, &mut changed); }
            });
            if changed { gui.actions.push(GuiAction::RebuildObject { id, params: CreationParams::Torus { major_radius: mr, minor_radius: mnr } }); }
        }
        CreationParams::Tube { outer_radius, inner_radius, height } => {
            let (mut or, mut ir, mut h) = (*outer_radius, *inner_radius, *height);
            let mut changed = false;
            egui::Grid::new("tube_params").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
                if matches_filter("Outer Radius", filter) { param_row(ui, "Outer Radius", &mut or, 0.1..=500.0, 0.1, &mut changed); }
                if matches_filter("Inner Radius", filter) { param_row(ui, "Inner Radius", &mut ir, 0.01..=499.0, 0.1, &mut changed); }
                if matches_filter("Height", filter) { param_row(ui, "Height", &mut h, 0.1..=1000.0, 0.1, &mut changed); }
            });
            if changed { gui.actions.push(GuiAction::RebuildObject { id, params: CreationParams::Tube { outer_radius: or, inner_radius: ir, height: h } }); }
        }
        CreationParams::Prism { radius, height, sides } => {
            let (mut r, mut h, mut s) = (*radius, *height, *sides);
            let mut changed = false;
            egui::Grid::new("prism_params").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
                if matches_filter("Radius", filter) { param_row(ui, "Radius", &mut r, 0.1..=500.0, 0.1, &mut changed); }
                if matches_filter("Height", filter) { param_row(ui, "Height", &mut h, 0.1..=1000.0, 0.1, &mut changed); }
                if matches_filter("Sides", filter) {
                    ui.label(egui::RichText::new("Sides:").size(11.0).color(theme::COLOR_DIM));
                    changed |= ui.add(egui::DragValue::new(&mut s).range(3..=128)).changed();
                    ui.end_row();
                }
            });
            if changed { gui.actions.push(GuiAction::RebuildObject { id, params: CreationParams::Prism { radius: r, height: h, sides: s } }); }
        }
        CreationParams::Wedge { dx, dy, dz, dx2, dy2 } => {
            let (mut x, mut y, mut z, mut x2, mut y2) = (*dx, *dy, *dz, *dx2, *dy2);
            let mut changed = false;
            egui::Grid::new("wedge_params").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
                if matches_filter("DX", filter) { param_row(ui, "DX", &mut x, 0.1..=1000.0, 0.1, &mut changed); }
                if matches_filter("DY", filter) { param_row(ui, "DY", &mut y, 0.1..=1000.0, 0.1, &mut changed); }
                if matches_filter("DZ", filter) { param_row(ui, "DZ", &mut z, 0.1..=1000.0, 0.1, &mut changed); }
                if matches_filter("DX2", filter) { param_row(ui, "DX2", &mut x2, 0.0..=1000.0, 0.1, &mut changed); }
                if matches_filter("DY2", filter) { param_row(ui, "DY2", &mut y2, 0.0..=1000.0, 0.1, &mut changed); }
            });
            if changed { gui.actions.push(GuiAction::RebuildObject { id, params: CreationParams::Wedge { dx: x, dy: y, dz: z, dx2: x2, dy2: y2 } }); }
        }
        CreationParams::Ellipsoid { rx, ry, rz } => {
            let (mut x, mut y, mut z) = (*rx, *ry, *rz);
            let mut changed = false;
            egui::Grid::new("ell_params").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
                if matches_filter("Radius X", filter) { param_row(ui, "Radius X", &mut x, 0.1..=500.0, 0.1, &mut changed); }
                if matches_filter("Radius Y", filter) { param_row(ui, "Radius Y", &mut y, 0.1..=500.0, 0.1, &mut changed); }
                if matches_filter("Radius Z", filter) { param_row(ui, "Radius Z", &mut z, 0.1..=500.0, 0.1, &mut changed); }
            });
            if changed { gui.actions.push(GuiAction::RebuildObject { id, params: CreationParams::Ellipsoid { rx: x, ry: y, rz: z } }); }
        }
        CreationParams::Helix { radius, pitch, turns, tube_radius } => {
            let (mut r, mut p, mut t, mut tr) = (*radius, *pitch, *turns, *tube_radius);
            let mut changed = false;
            egui::Grid::new("helix_params").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
                if matches_filter("Radius", filter) { param_row(ui, "Radius", &mut r, 0.1..=500.0, 0.1, &mut changed); }
                if matches_filter("Pitch", filter) { param_row(ui, "Pitch", &mut p, 0.1..=100.0, 0.1, &mut changed); }
                if matches_filter("Turns", filter) { param_row(ui, "Turns", &mut t, 0.5..=100.0, 0.1, &mut changed); }
                if matches_filter("Tube Radius", filter) { param_row(ui, "Tube Radius", &mut tr, 0.01..=100.0, 0.05, &mut changed); }
            });
            if changed { gui.actions.push(GuiAction::RebuildObject { id, params: CreationParams::Helix { radius: r, pitch: p, turns: t, tube_radius: tr } }); }
        }
        CreationParams::Imported { path } => {
            ui.label(egui::RichText::new(format!("File: {path}")).size(11.0).color(theme::COLOR_DIM));
        }
        CreationParams::Extruded | CreationParams::Revolved => {
            ui.weak(egui::RichText::new("(feature operation)").size(11.0));
        }
        CreationParams::Boolean { op } => {
            ui.label(egui::RichText::new(format!("Operation: {op}")).size(11.0).color(theme::COLOR_DIM));
        }
        CreationParams::Fillet { radius } => {
            let mut r = *radius;
            let mut changed = false;
            egui::Grid::new("fillet_params").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
                if matches_filter("Radius", filter) { param_row(ui, "Radius", &mut r, 0.01..=100.0, 0.1, &mut changed); }
            });
            if changed { gui.actions.push(GuiAction::RebuildObject { id, params: CreationParams::Fillet { radius: r } }); }
        }
        CreationParams::Chamfer { distance } => {
            let mut d = *distance;
            let mut changed = false;
            egui::Grid::new("chamfer_params").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
                if matches_filter("Distance", filter) { param_row(ui, "Distance", &mut d, 0.01..=100.0, 0.1, &mut changed); }
            });
            if changed { gui.actions.push(GuiAction::RebuildObject { id, params: CreationParams::Chamfer { distance: d } }); }
        }
        CreationParams::Shell { thickness } => {
            let mut t = *thickness;
            let mut changed = false;
            egui::Grid::new("shell_params").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
                if matches_filter("Thickness", filter) { param_row(ui, "Thickness", &mut t, 0.01..=50.0, 0.1, &mut changed); }
            });
            if changed { gui.actions.push(GuiAction::RebuildObject { id, params: CreationParams::Shell { thickness: t } }); }
        }
        CreationParams::Mirror { .. }
        | CreationParams::Pattern { .. }
        | CreationParams::Groove { .. }
        | CreationParams::Sprocket { .. }
        | CreationParams::InvoluteGear { .. }
        | CreationParams::DraftLine { .. }
        | CreationParams::DraftCircle { .. }
        | CreationParams::DraftRectangle { .. }
        | CreationParams::DraftPolygon { .. }
        | CreationParams::DraftArc { .. }
        | CreationParams::DraftEllipse { .. }
        | CreationParams::SurfacePipe { .. }
        | CreationParams::SurfaceRuled { .. }
        | CreationParams::BooleanOp { .. }
        | CreationParams::ScaleOp { .. } => {
            ui.weak(egui::RichText::new("(parametric operation)").size(11.0));
        }
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn param_row(ui: &mut egui::Ui, label: &str, value: &mut f64, range: std::ops::RangeInclusive<f64>, speed: f64, changed: &mut bool) {
    ui.label(egui::RichText::new(format!("{label}:")).size(11.0).color(theme::COLOR_DIM));
    *changed |= ui.add(egui::DragValue::new(value).range(range).speed(speed).suffix(" mm")).changed();
    ui.end_row();
}

fn mesh_center(mesh: &cadkernel_io::Mesh) -> (f64, f64, f64) {
    if mesh.vertices.is_empty() { return (0.0, 0.0, 0.0); }
    let n = mesh.vertices.len() as f64;
    let sum = mesh.vertices.iter().fold((0.0_f64, 0.0_f64, 0.0_f64), |(sx, sy, sz), p| (sx + p.x, sy + p.y, sz + p.z));
    (sum.0 / n, sum.1 / n, sum.2 / n)
}

fn creation_param_labels(p: &CreationParams) -> Vec<&'static str> {
    match p {
        CreationParams::Box { .. } => vec!["Width", "Height", "Depth"],
        CreationParams::Cylinder { .. } => vec!["Radius", "Height"],
        CreationParams::Sphere { .. } => vec!["Radius"],
        CreationParams::Cone { .. } => vec!["Base Radius", "Top Radius", "Height"],
        CreationParams::Torus { .. } => vec!["Major Radius", "Minor Radius"],
        CreationParams::Tube { .. } => vec!["Outer Radius", "Inner Radius", "Height"],
        CreationParams::Prism { .. } => vec!["Radius", "Height", "Sides"],
        CreationParams::Wedge { .. } => vec!["DX", "DY", "DZ", "DX2", "DY2"],
        CreationParams::Ellipsoid { .. } => vec!["Radius X", "Radius Y", "Radius Z"],
        CreationParams::Helix { .. } => vec!["Radius", "Pitch", "Turns", "Tube Radius"],
        CreationParams::Fillet { .. } => vec!["Radius"],
        CreationParams::Chamfer { .. } => vec!["Distance"],
        CreationParams::Shell { .. } => vec!["Thickness"],
        _ => vec!["Parameters"],
    }
}

fn params_type_name(p: &CreationParams) -> &'static str {
    match p {
        CreationParams::Box { .. } => "Part::Box",
        CreationParams::Cylinder { .. } => "Part::Cylinder",
        CreationParams::Sphere { .. } => "Part::Sphere",
        CreationParams::Cone { .. } => "Part::Cone",
        CreationParams::Torus { .. } => "Part::Torus",
        CreationParams::Tube { .. } => "Part::Tube",
        CreationParams::Prism { .. } => "Part::Prism",
        CreationParams::Wedge { .. } => "Part::Wedge",
        CreationParams::Ellipsoid { .. } => "Part::Ellipsoid",
        CreationParams::Helix { .. } => "Part::Helix",
        CreationParams::Imported { .. } => "Mesh::Import",
        CreationParams::Extruded => "Part::Extrusion",
        CreationParams::Revolved => "Part::Revolution",
        CreationParams::Boolean { .. } => "Part::Boolean",
        CreationParams::Fillet { .. } => "PartDesign::Fillet",
        CreationParams::Chamfer { .. } => "PartDesign::Chamfer",
        CreationParams::Shell { .. } => "PartDesign::Shell",
        CreationParams::Mirror { .. } => "PartDesign::Mirror",
        CreationParams::Pattern { .. } => "PartDesign::Pattern",
        CreationParams::Groove { .. } => "PartDesign::Groove",
        CreationParams::Sprocket { .. } => "PartDesign::Sprocket",
        CreationParams::InvoluteGear { .. } => "PartDesign::Gear",
        CreationParams::DraftLine { .. } => "Draft::Line",
        CreationParams::DraftCircle { .. } => "Draft::Circle",
        CreationParams::DraftRectangle { .. } => "Draft::Rectangle",
        CreationParams::DraftPolygon { .. } => "Draft::Polygon",
        CreationParams::DraftArc { .. } => "Draft::Arc",
        CreationParams::DraftEllipse { .. } => "Draft::Ellipse",
        CreationParams::SurfacePipe { .. } => "Surface::Pipe",
        CreationParams::SurfaceRuled { .. } => "Surface::Ruled",
        CreationParams::BooleanOp { .. } => "Part::BooleanOp",
        CreationParams::ScaleOp { .. } => "Part::Scale",
    }
}

// ---------------------------------------------------------------------------
// Material presets
// ---------------------------------------------------------------------------

struct MaterialPreset {
    name: &'static str,
    color: [f32; 4],
    icon: &'static str,
}

const MATERIAL_PRESETS: &[MaterialPreset] = &[
    MaterialPreset { name: "Steel",          color: [0.68, 0.70, 0.72, 1.0], icon: "\u{2699}" },
    MaterialPreset { name: "Aluminum",       color: [0.78, 0.80, 0.82, 1.0], icon: "\u{2B22}" },
    MaterialPreset { name: "Brass",          color: [0.80, 0.68, 0.35, 1.0], icon: "\u{25C9}" },
    MaterialPreset { name: "Copper",         color: [0.72, 0.45, 0.20, 1.0], icon: "\u{25C9}" },
    MaterialPreset { name: "Gold",           color: [0.85, 0.75, 0.35, 1.0], icon: "\u{2726}" },
    MaterialPreset { name: "Titanium",       color: [0.62, 0.63, 0.65, 1.0], icon: "\u{2B22}" },
    MaterialPreset { name: "Cast Iron",      color: [0.40, 0.40, 0.42, 1.0], icon: "\u{2699}" },
    MaterialPreset { name: "Plastic White",  color: [0.92, 0.92, 0.90, 1.0], icon: "\u{25CB}" },
    MaterialPreset { name: "Plastic Black",  color: [0.15, 0.15, 0.17, 1.0], icon: "\u{25CF}" },
    MaterialPreset { name: "Plastic Red",    color: [0.85, 0.15, 0.12, 1.0], icon: "\u{25CF}" },
    MaterialPreset { name: "Plastic Blue",   color: [0.15, 0.35, 0.85, 1.0], icon: "\u{25CF}" },
    MaterialPreset { name: "Glass",          color: [0.75, 0.85, 0.90, 0.35], icon: "\u{25C7}" },
    MaterialPreset { name: "Wood (Light)",   color: [0.72, 0.56, 0.35, 1.0], icon: "\u{25A3}" },
    MaterialPreset { name: "Wood (Dark)",    color: [0.45, 0.30, 0.18, 1.0], icon: "\u{25A3}" },
    MaterialPreset { name: "Rubber",         color: [0.20, 0.20, 0.22, 1.0], icon: "\u{25CB}" },
    MaterialPreset { name: "Carbon Fiber",   color: [0.12, 0.12, 0.14, 1.0], icon: "\u{25A6}" },
];

fn draw_material_presets(
    ui: &mut egui::Ui,
    gui: &mut GuiState,
    obj: &crate::scene::SceneObject,
) {
    // Current material detection (find closest match)
    let current_mat = MATERIAL_PRESETS.iter().enumerate().find(|(_, m)| {
        let dc: f32 = (0..3).map(|i| (obj.color[i] - m.color[i]).powi(2)).sum();
        dc < 0.01
    }).map(|(i, _)| i);

    // Material grid: 2 columns of swatches
    let cols = 2;
    egui::Grid::new("material_grid").num_columns(cols).spacing([4.0, 3.0]).show(ui, |ui| {
        for (i, mat) in MATERIAL_PRESETS.iter().enumerate() {
            let is_current = current_mat == Some(i);
            let swatch_color = egui::Color32::from_rgba_unmultiplied(
                (mat.color[0] * 255.0) as u8,
                (mat.color[1] * 255.0) as u8,
                (mat.color[2] * 255.0) as u8,
                (mat.color[3] * 255.0) as u8,
            );

            let btn_text = format!("{} {}", mat.icon, mat.name);
            let text = if is_current {
                egui::RichText::new(&btn_text).size(10.0).strong().color(egui::Color32::WHITE)
            } else {
                egui::RichText::new(&btn_text).size(10.0).color(theme::COLOR_DIM)
            };

            let mut btn = egui::Button::new(text).min_size(egui::vec2(125.0, 20.0));
            if is_current {
                btn = btn.fill(egui::Color32::from_rgb(0, 80, 150));
            }
            let resp = ui.add(btn);

            // Swatch preview
            let swatch_rect = egui::Rect::from_min_size(
                egui::pos2(resp.rect.right() - 18.0, resp.rect.top() + 3.0),
                egui::vec2(14.0, 14.0),
            );
            ui.painter().rect_filled(swatch_rect, 2.0, swatch_color);
            if is_current {
                ui.painter().rect_stroke(swatch_rect, 2.0, egui::Stroke::new(1.0, egui::Color32::WHITE), egui::StrokeKind::Middle);
            }

            if resp.clicked() {
                gui.actions.push(GuiAction::SetObjectColor { id: obj.id, color: mat.color });
            }
            resp.on_hover_text(mat.name);

            if (i + 1) % cols == 0 {
                ui.end_row();
            }
        }
    });
}
