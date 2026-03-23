//! Professional CAD application theme — FreeCAD-inspired dark theme.
//!
//! Sets up egui visuals, spacing, and colors for a professional CAD look.

/// Apply the CADKernel dark theme to an egui context.
pub fn apply_cad_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();

    // Background colors (darker, more professional)
    visuals.window_fill = egui::Color32::from_rgb(35, 38, 45);
    visuals.panel_fill = egui::Color32::from_rgb(40, 43, 50);
    visuals.faint_bg_color = egui::Color32::from_rgb(48, 52, 60);
    visuals.extreme_bg_color = egui::Color32::from_rgb(25, 27, 32);

    // Widget colors
    visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(50, 55, 65);
    visuals.widgets.noninteractive.fg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(180, 185, 195));

    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(55, 60, 72);
    visuals.widgets.inactive.fg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(190, 195, 205));

    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(70, 80, 100);
    visuals.widgets.hovered.fg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(220, 225, 235));

    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(60, 100, 160);
    visuals.widgets.active.fg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(240, 245, 255));

    // Selection colors (FreeCAD-like blue)
    visuals.selection.bg_fill = egui::Color32::from_rgba_premultiplied(60, 100, 170, 200);
    visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 160, 230));

    // Separator
    visuals.widgets.noninteractive.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 65, 75));

    // Window shadow
    visuals.window_shadow = egui::epaint::Shadow {
        offset: [2, 4],
        blur: 8,
        spread: 0,
        color: egui::Color32::from_black_alpha(80),
    };

    // Window/menu rounding set via widget visuals (egui 0.31+)

    // Hyperlink color
    visuals.hyperlink_color = egui::Color32::from_rgb(100, 160, 240);

    // Warn/error text
    visuals.warn_fg_color = egui::Color32::from_rgb(240, 190, 60);
    visuals.error_fg_color = egui::Color32::from_rgb(230, 80, 70);

    ctx.set_visuals(visuals);

    // Style: spacing and sizing
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(6.0, 4.0);
    style.spacing.button_padding = egui::vec2(6.0, 3.0);
    style.spacing.indent = 18.0;
    style.spacing.scroll.bar_width = 8.0;
    // Slightly larger text for readability
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::new(13.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::new(13.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::new(16.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        egui::FontId::new(11.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        egui::FontId::new(12.5, egui::FontFamily::Monospace),
    );
    ctx.set_style(style);
}

/// Object type icon for the model tree.
pub fn object_type_icon(params: Option<&crate::scene::CreationParams>) -> &'static str {
    use crate::scene::CreationParams;
    match params {
        Some(CreationParams::Box { .. }) => "\u{25A3}",       // ▣ filled square
        Some(CreationParams::Cylinder { .. }) => "\u{25CD}",  // ◍
        Some(CreationParams::Sphere { .. }) => "\u{25CF}",    // ●
        Some(CreationParams::Cone { .. }) => "\u{25B2}",      // ▲
        Some(CreationParams::Torus { .. }) => "\u{25CE}",     // ◎
        Some(CreationParams::Tube { .. }) => "\u{25C9}",      // ◉
        Some(CreationParams::Prism { .. }) => "\u{2B23}",     // ⬣
        Some(CreationParams::Wedge { .. }) => "\u{25C7}",     // ◇
        Some(CreationParams::Ellipsoid { .. }) => "\u{2B2D}", // ⬭
        Some(CreationParams::Helix { .. }) => "\u{223F}",     // ∿
        Some(CreationParams::Imported { .. }) => "\u{1F4C2}", // 📂
        Some(CreationParams::Extruded) => "\u{2B06}",         // ⬆
        Some(CreationParams::Revolved) => "\u{21BB}",         // ↻
        Some(CreationParams::Boolean { .. }) => "\u{222A}",   // ∪
        None => "\u{25A1}",                                    // □
    }
}

/// Theme color constants used across the UI.
pub const COLOR_INFO: egui::Color32 = egui::Color32::from_rgb(160, 200, 240);
#[allow(dead_code)]
pub const COLOR_WARN: egui::Color32 = egui::Color32::from_rgb(240, 190, 60);
#[allow(dead_code)]
pub const COLOR_ERROR: egui::Color32 = egui::Color32::from_rgb(230, 80, 70);
#[allow(dead_code)]
pub const COLOR_SUCCESS: egui::Color32 = egui::Color32::from_rgb(100, 210, 120);
pub const COLOR_ACCENT: egui::Color32 = egui::Color32::from_rgb(80, 140, 220);
pub const COLOR_DIM: egui::Color32 = egui::Color32::from_rgb(120, 125, 135);
pub const COLOR_SELECTED: egui::Color32 = egui::Color32::from_rgb(80, 160, 240);
#[allow(dead_code)]
pub const COLOR_PRESELECT: egui::Color32 = egui::Color32::from_rgb(240, 200, 80);
