//! Professional CAD application theme system.
//!
//! Provides dark/light themes with configurable UI density, inspired by
//! VS Code / Fusion 360 (dark) and VS Code / SolidWorks (light).

use egui::Color32;

/// Theme mode: dark, light, or resolved from the host environment.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum ThemeMode {
    Dark,
    Light,
    System,
}

impl ThemeMode {
    pub const ALL: &[ThemeMode] = &[ThemeMode::Dark, ThemeMode::Light, ThemeMode::System];

    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
            Self::System => "System",
        }
    }

    pub fn resolved(self) -> ThemeMode {
        match self {
            Self::System => {
                if detect_system_dark() {
                    Self::Dark
                } else {
                    Self::Light
                }
            }
            other => other,
        }
    }
}

/// UI density preset controlling spacing, button sizes, and font sizes.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum UiDensity {
    Compact,
    Normal,
    Spacious,
}

/// Complete theme definition for the CADKernel viewer.
#[derive(Clone)]
#[allow(dead_code)]
pub struct CadTheme {
    pub mode: ThemeMode,

    // Panel colors
    pub bg_primary: Color32,
    pub bg_secondary: Color32,
    pub bg_tertiary: Color32,
    pub bg_viewport: Color32,

    // Text colors
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_disabled: Color32,

    // Accent colors
    pub accent: Color32,
    pub accent_hover: Color32,
    pub accent_pressed: Color32,

    // Selection colors
    pub selection: Color32,
    pub selection_text: Color32,
    pub preselection: Color32,

    // Status colors
    pub error: Color32,
    pub warning: Color32,
    pub info: Color32,
    pub success: Color32,

    // Border colors
    pub border: Color32,
    pub border_strong: Color32,

    // Toolbar colors
    pub toolbar_bg: Color32,
    pub toolbar_button_hover: Color32,
    pub toolbar_button_active: Color32,

    // Tree colors
    pub tree_bg: Color32,
    pub tree_selected: Color32,
    pub tree_hover: Color32,

    // Panel chrome (FreeCAD-style titled header bars)
    pub panel_header_bg: Color32,
    pub panel_header_text: Color32,
    pub panel_separator: Color32,
    pub section_header_bg: Color32,
    pub section_header_text: Color32,

    // Spacing / sizing
    pub density: UiDensity,
    pub padding_outer: f32,
    pub padding_inner: f32,
    pub item_spacing: f32,
    pub button_size: f32,
    pub icon_size: f32,
    pub font_size_normal: f32,
    pub font_size_small: f32,
    pub font_size_header: f32,
    pub rounding: f32,

    // Panel chrome sizing
    pub panel_header_height: f32,
    pub section_header_height: f32,
    pub tree_row_height: f32,
}

impl CadTheme {
    /// Professional dark theme — CADKernel signature palette
    /// (deep teal accent, cool blue-tinted neutrals, soft elevation).
    /// Replaces the previous VS-Code-inspired flat blue with a more
    /// distinctive look while keeping the dock-friendly contrast levels.
    pub fn dark() -> Self {
        Self {
            mode: ThemeMode::Dark,
            // Cool blue-tinted neutrals (was pure greys)
            bg_primary: Color32::from_rgb(0x16, 0x19, 0x20),
            bg_secondary: Color32::from_rgb(0x1C, 0x20, 0x28),
            bg_tertiary: Color32::from_rgb(0x24, 0x29, 0x33),
            bg_viewport: Color32::from_rgb(0x2B, 0x30, 0x3B),
            text_primary: Color32::from_rgb(0xE4, 0xE7, 0xEC),
            text_secondary: Color32::from_rgb(0x9A, 0xA1, 0xAE),
            text_disabled: Color32::from_rgb(0x5C, 0x63, 0x70),
            // Signature accent: deep teal (was VS Code blue #007ACC)
            accent: Color32::from_rgb(0x14, 0xB8, 0xA6),
            accent_hover: Color32::from_rgb(0x2A, 0xD4, 0xC0),
            accent_pressed: Color32::from_rgb(0x0E, 0x8E, 0x80),
            selection: Color32::from_rgb(0x12, 0x55, 0x4F),
            selection_text: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            preselection: Color32::from_rgb(0x2E, 0x34, 0x40),
            error: Color32::from_rgb(0xF4, 0x6E, 0x6E),
            warning: Color32::from_rgb(0xF1, 0xB7, 0x3E),
            info: Color32::from_rgb(0x4D, 0xA8, 0xFF),
            success: Color32::from_rgb(0x6F, 0xD9, 0x89),
            border: Color32::from_rgb(0x32, 0x38, 0x44),
            border_strong: Color32::from_rgb(0x4A, 0x52, 0x60),
            toolbar_bg: Color32::from_rgb(0x1C, 0x20, 0x28),
            toolbar_button_hover: Color32::from_rgb(0x2E, 0x34, 0x40),
            toolbar_button_active: Color32::from_rgb(0x12, 0x55, 0x4F),
            tree_bg: Color32::from_rgb(0x1C, 0x20, 0x28),
            tree_selected: Color32::from_rgb(0x12, 0x55, 0x4F),
            tree_hover: Color32::from_rgb(0x2A, 0x30, 0x3C),
            // Dock chrome
            panel_header_bg: Color32::from_rgb(0x14, 0x17, 0x1D),
            panel_header_text: Color32::from_rgb(0xB0, 0xB7, 0xC4),
            panel_separator: Color32::from_rgb(0x2A, 0x30, 0x3C),
            section_header_bg: Color32::from_rgb(0x22, 0x27, 0x31),
            section_header_text: Color32::from_rgb(0xA0, 0xA8, 0xB6),
            ..Self::normal_density()
        }
    }

    /// Professional light theme (VS Code / SolidWorks light inspired).
    #[allow(dead_code)]
    pub fn light() -> Self {
        Self {
            mode: ThemeMode::Light,
            bg_primary: Color32::from_rgb(0xF3, 0xF3, 0xF3),
            bg_secondary: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            bg_tertiary: Color32::from_rgb(0xF0, 0xF0, 0xF0),
            bg_viewport: Color32::from_rgb(0xE8, 0xE8, 0xE8),
            text_primary: Color32::from_rgb(0x33, 0x33, 0x33),
            text_secondary: Color32::from_rgb(0x76, 0x76, 0x76),
            text_disabled: Color32::from_rgb(0xA0, 0xA0, 0xA0),
            accent: Color32::from_rgb(0x00, 0x78, 0xD4),
            accent_hover: Color32::from_rgb(0x10, 0x6E, 0xBE),
            accent_pressed: Color32::from_rgb(0x00, 0x5A, 0x9E),
            selection: Color32::from_rgb(0xCC, 0xE5, 0xFF),
            selection_text: Color32::from_rgb(0x00, 0x00, 0x00),
            preselection: Color32::from_rgb(0xE8, 0xE8, 0xE8),
            error: Color32::from_rgb(0xE5, 0x14, 0x00),
            warning: Color32::from_rgb(0xE8, 0x74, 0x00),
            info: Color32::from_rgb(0x00, 0x66, 0xB8),
            success: Color32::from_rgb(0x10, 0x7C, 0x10),
            border: Color32::from_rgb(0xD4, 0xD4, 0xD4),
            border_strong: Color32::from_rgb(0xB0, 0xB0, 0xB0),
            toolbar_bg: Color32::from_rgb(0xF3, 0xF3, 0xF3),
            toolbar_button_hover: Color32::from_rgb(0xE0, 0xE0, 0xE0),
            toolbar_button_active: Color32::from_rgb(0xCC, 0xE5, 0xFF),
            tree_bg: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            tree_selected: Color32::from_rgb(0xCC, 0xE5, 0xFF),
            tree_hover: Color32::from_rgb(0xE8, 0xE8, 0xE8),
            panel_header_bg: Color32::from_rgb(0xE0, 0xE2, 0xE6),
            panel_header_text: Color32::from_rgb(0x40, 0x44, 0x50),
            panel_separator: Color32::from_rgb(0xC8, 0xCC, 0xD4),
            section_header_bg: Color32::from_rgb(0xF0, 0xF1, 0xF3),
            section_header_text: Color32::from_rgb(0x50, 0x58, 0x65),
            ..Self::normal_density()
        }
    }

    /// Create a theme from a mode selection.
    #[allow(dead_code)]
    pub fn from_mode(mode: ThemeMode) -> Self {
        match mode.resolved() {
            ThemeMode::Dark => Self::dark(),
            ThemeMode::Light => Self::light(),
            ThemeMode::System => Self::dark(),
        }
    }

    /// Set the UI density, adjusting spacing and font sizes.
    #[allow(dead_code)]
    pub fn with_density(mut self, density: UiDensity) -> Self {
        self.density = density;
        match density {
            UiDensity::Compact => {
                self.padding_outer = 4.0;
                self.padding_inner = 2.0;
                self.item_spacing = 2.0;
                self.button_size = 26.0;
                self.icon_size = 16.0;
                self.font_size_normal = 11.0;
                self.font_size_small = 10.0;
                self.font_size_header = 12.0;
                self.rounding = 2.0;
                self.panel_header_height = 20.0;
                self.section_header_height = 18.0;
                self.tree_row_height = 18.0;
            }
            UiDensity::Normal => {
                self.padding_outer = 8.0;
                self.padding_inner = 4.0;
                self.item_spacing = 4.0;
                self.button_size = 32.0;
                self.icon_size = 20.0;
                self.font_size_normal = 13.0;
                self.font_size_small = 11.0;
                self.font_size_header = 14.0;
                self.rounding = 4.0;
                self.panel_header_height = 24.0;
                self.section_header_height = 22.0;
                self.tree_row_height = 22.0;
            }
            UiDensity::Spacious => {
                self.padding_outer = 12.0;
                self.padding_inner = 6.0;
                self.item_spacing = 6.0;
                self.button_size = 38.0;
                self.icon_size = 24.0;
                self.font_size_normal = 14.0;
                self.font_size_small = 12.0;
                self.font_size_header = 16.0;
                self.rounding = 6.0;
                self.panel_header_height = 28.0;
                self.section_header_height = 24.0;
                self.tree_row_height = 24.0;
            }
        }
        self
    }

    /// Apply this theme to an egui context (visuals + style + text sizes).
    pub fn apply_to_egui(&self, ctx: &egui::Context) {
        let mut visuals = if self.mode == ThemeMode::Dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };

        visuals.override_text_color = Some(self.text_primary);
        visuals.panel_fill = self.bg_secondary;
        visuals.window_fill = self.bg_secondary;
        visuals.extreme_bg_color = self.bg_tertiary;
        visuals.faint_bg_color = self.bg_primary;

        visuals.selection.bg_fill = self.selection;
        visuals.selection.stroke = egui::Stroke::new(1.0, self.accent);

        visuals.widgets.inactive.bg_fill = self.bg_tertiary;
        visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, self.text_secondary);

        visuals.widgets.hovered.bg_fill = self.accent_hover;
        visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, self.text_primary);

        visuals.widgets.active.bg_fill = self.accent_pressed;
        visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, self.selection_text);

        visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, self.text_primary);
        visuals.widgets.noninteractive.bg_fill = self.bg_secondary;
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, self.border);

        visuals.window_corner_radius = egui::CornerRadius::same(self.rounding as u8);
        visuals.window_shadow = egui::epaint::Shadow::NONE;

        visuals.hyperlink_color = self.info;
        visuals.warn_fg_color = self.warning;
        visuals.error_fg_color = self.error;

        ctx.set_visuals(visuals);

        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(self.item_spacing, self.item_spacing);
        style.spacing.button_padding = egui::vec2(self.padding_inner, self.padding_inner * 0.75);
        style.spacing.window_margin = egui::Margin::same(self.padding_outer as i8);
        style.spacing.indent = 18.0;
        style.spacing.scroll.bar_width = 8.0;

        style.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::new(self.font_size_normal, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::new(self.font_size_normal, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Heading,
            egui::FontId::new(self.font_size_header, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Small,
            egui::FontId::new(self.font_size_small, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Monospace,
            egui::FontId::new(self.font_size_normal - 0.5, egui::FontFamily::Monospace),
        );

        ctx.set_style(style);
    }

    /// Default Normal density values (used as base for dark/light constructors).
    fn normal_density() -> Self {
        Self {
            mode: ThemeMode::Dark,
            bg_primary: Color32::BLACK,
            bg_secondary: Color32::BLACK,
            bg_tertiary: Color32::BLACK,
            bg_viewport: Color32::BLACK,
            text_primary: Color32::WHITE,
            text_secondary: Color32::GRAY,
            text_disabled: Color32::DARK_GRAY,
            accent: Color32::BLUE,
            accent_hover: Color32::BLUE,
            accent_pressed: Color32::BLUE,
            selection: Color32::BLUE,
            selection_text: Color32::WHITE,
            preselection: Color32::GRAY,
            error: Color32::RED,
            warning: Color32::YELLOW,
            info: Color32::BLUE,
            success: Color32::GREEN,
            border: Color32::DARK_GRAY,
            border_strong: Color32::GRAY,
            toolbar_bg: Color32::BLACK,
            toolbar_button_hover: Color32::DARK_GRAY,
            toolbar_button_active: Color32::BLUE,
            tree_bg: Color32::BLACK,
            tree_selected: Color32::BLUE,
            tree_hover: Color32::DARK_GRAY,
            panel_header_bg: Color32::BLACK,
            panel_header_text: Color32::GRAY,
            panel_separator: Color32::DARK_GRAY,
            section_header_bg: Color32::BLACK,
            section_header_text: Color32::GRAY,
            density: UiDensity::Normal,
            padding_outer: 8.0,
            padding_inner: 4.0,
            item_spacing: 4.0,
            button_size: 32.0,
            icon_size: 20.0,
            font_size_normal: 13.0,
            font_size_small: 11.0,
            font_size_header: 14.0,
            rounding: 6.0,
            panel_header_height: 24.0,
            section_header_height: 22.0,
            tree_row_height: 22.0,
        }
    }
}

/// Backward-compatible wrapper: applies the dark theme with normal density.
#[allow(dead_code)]
pub fn apply_cad_theme(ctx: &egui::Context) {
    CadTheme::dark().apply_to_egui(ctx);
}

pub fn detect_system_dark_from_env(value: Option<&str>) -> bool {
    value
        .map(|raw| {
            let normalized = raw.trim().to_ascii_lowercase();
            matches!(
                normalized.as_str(),
                "1" | "true" | "dark" | "yes" | "y" | "on"
            )
        })
        .unwrap_or(false)
}

pub fn detect_system_dark() -> bool {
    if let Ok(value) = std::env::var("CADKERNEL_THEME") {
        return detect_system_dark_from_env(Some(&value));
    }
    if let Ok(value) = std::env::var("GTK_THEME") {
        if value.to_ascii_lowercase().contains("dark") {
            return true;
        }
    }
    if let Ok(value) = std::env::var("COLORFGBG") {
        if let Some(bg) = value.split(';').next_back()
            && let Ok(code) = bg.parse::<u8>()
        {
            return code < 8;
        }
    }
    false
}

/// Object type icon for the model tree.
#[allow(dead_code)]
pub fn object_type_icon(params: Option<&crate::scene::CreationParams>) -> &'static str {
    use crate::scene::CreationParams;
    match params {
        Some(CreationParams::Box { .. }) => "\u{25A3}", // filled square
        Some(CreationParams::Cylinder { .. }) => "\u{25CD}", // circle with stroke
        Some(CreationParams::Sphere { .. }) => "\u{25CF}", // filled circle
        Some(CreationParams::Cone { .. }) => "\u{25B2}", // up triangle
        Some(CreationParams::Torus { .. }) => "\u{25CE}", // bullseye
        Some(CreationParams::Tube { .. }) => "\u{25C9}", // fisheye
        Some(CreationParams::Prism { .. }) => "\u{2B23}", // hexagon
        Some(CreationParams::Wedge { .. }) => "\u{25C7}", // diamond
        Some(CreationParams::Ellipsoid { .. }) => "\u{2B2D}", // horizontal ellipse
        Some(CreationParams::Helix { .. }) => "\u{223F}", // sine wave
        Some(CreationParams::Imported { .. }) => "\u{1F4C2}", // folder
        Some(CreationParams::Extruded) => "\u{2B06}",   // up arrow
        Some(CreationParams::Revolved) => "\u{21BB}",   // clockwise arrow
        Some(CreationParams::Boolean { .. }) => "\u{222A}", // union
        Some(CreationParams::Fillet { .. }) => "\u{25D5}", // circle with right half
        Some(CreationParams::Chamfer { .. }) => "\u{25C8}", // diamond in diamond
        Some(CreationParams::Shell { .. }) => "\u{25A2}", // square with orthog
        Some(CreationParams::Mirror { .. }) => "\u{21C6}", // left right arrows
        Some(CreationParams::Pattern { .. }) => "\u{2237}", // proportion
        Some(CreationParams::Groove { .. }) => "\u{21BB}", // clockwise arrow
        Some(CreationParams::Sprocket { .. }) => "\u{2699}", // gear
        Some(CreationParams::InvoluteGear { .. }) => "\u{2699}", // gear
        Some(CreationParams::DraftLine { .. }) => "\u{2571}", // diagonal
        Some(CreationParams::DraftCircle { .. }) => "\u{25CB}", // white circle
        Some(CreationParams::DraftRectangle { .. }) => "\u{25A1}", // white square
        Some(CreationParams::DraftPolygon { .. }) => "\u{2B23}", // hexagon
        Some(CreationParams::DraftArc { .. }) => "\u{25DC}", // upper left quad arc
        Some(CreationParams::DraftEllipse { .. }) => "\u{2B2D}", // horizontal ellipse
        Some(CreationParams::SurfacePipe { .. }) => "\u{2234}", // therefore
        Some(CreationParams::SurfaceRuled { .. }) => "\u{2225}", // parallel
        Some(CreationParams::BooleanOp { .. }) => "\u{222A}", // union
        Some(CreationParams::ScaleOp { .. }) => "\u{2922}", // ne arrow
        None => "\u{25A1}",                             // empty square
    }
}

/// Theme color constants used across the UI.
pub const COLOR_INFO: Color32 = Color32::from_rgb(160, 200, 240);
#[allow(dead_code)]
pub const COLOR_WARN: Color32 = Color32::from_rgb(240, 190, 60);
#[allow(dead_code)]
pub const COLOR_ERROR: Color32 = Color32::from_rgb(230, 80, 70);
#[allow(dead_code)]
pub const COLOR_SUCCESS: Color32 = Color32::from_rgb(100, 210, 120);
pub const COLOR_ACCENT: Color32 = Color32::from_rgb(0x14, 0xB8, 0xA6);
pub const COLOR_DIM: Color32 = Color32::from_rgb(120, 125, 135);
#[allow(dead_code)]
pub const COLOR_SELECTED: Color32 = Color32::from_rgb(0x2A, 0xD4, 0xC0);
#[allow(dead_code)]
pub const COLOR_PRESELECT: Color32 = Color32::from_rgb(240, 200, 80);

// ---------------------------------------------------------------------------
// Sketch overlay colors (FreeCAD-style)
// ---------------------------------------------------------------------------
/// Normal sketch geometry — white.
#[allow(dead_code)]
pub const SKETCH_GEOMETRY: Color32 = Color32::from_rgb(240, 240, 240);
/// Construction geometry — blue.
#[allow(dead_code)]
pub const SKETCH_CONSTRUCTION: Color32 = Color32::from_rgb(60, 120, 220);
/// Selected entity — green.
#[allow(dead_code)]
pub const SKETCH_SELECTED: Color32 = Color32::from_rgb(50, 220, 50);
/// Hovered entity — bright green.
#[allow(dead_code)]
pub const SKETCH_HOVERED: Color32 = Color32::from_rgb(120, 255, 100);
/// Pending / preview — golden.
#[allow(dead_code)]
pub const SKETCH_PENDING: Color32 = Color32::from_rgb(255, 200, 50);
/// Constraint satisfied — red.
#[allow(dead_code)]
pub const SKETCH_CONSTRAINT: Color32 = Color32::from_rgb(220, 60, 60);
/// Constraint violated — orange.
#[allow(dead_code)]
pub const SKETCH_VIOLATED: Color32 = Color32::from_rgb(255, 100, 40);
/// DOF arrow — orange.
#[allow(dead_code)]
pub const SKETCH_DOF: Color32 = Color32::from_rgb(255, 160, 30);

// ---------------------------------------------------------------------------
// Menu section label color
// ---------------------------------------------------------------------------
/// Dim label color for section headers in menus / context menus.
pub const MENU_SECTION_COLOR: Color32 = Color32::from_rgb(100, 105, 115);

// ---------------------------------------------------------------------------
// Panel chrome helpers — FreeCAD-style titled headers & section dividers
// ---------------------------------------------------------------------------

/// Draw a dock-style panel header bar (dark bg, title text, optional close button).
/// Returns true if close button was clicked.
pub fn draw_panel_header(ui: &mut egui::Ui, title: &str, closeable: bool) -> bool {
    let theme = CadTheme::dark();
    let avail_w = ui.available_width();
    let h = theme.panel_header_height;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(avail_w, h), egui::Sense::hover());

    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, theme.panel_header_bg);
    // Bottom edge highlight
    painter.line_segment(
        [
            egui::pos2(rect.left(), rect.bottom()),
            egui::pos2(rect.right(), rect.bottom()),
        ],
        egui::Stroke::new(1.0, theme.panel_separator),
    );

    painter.text(
        egui::pos2(rect.left() + 8.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::new(11.5, egui::FontFamily::Proportional),
        theme.panel_header_text,
    );

    let mut closed = false;
    if closeable {
        let btn_center = egui::pos2(rect.right() - 14.0, rect.center().y);
        let btn_rect = egui::Rect::from_center_size(btn_center, egui::vec2(16.0, 16.0));
        let btn_resp = ui.interact(btn_rect, ui.id().with("close_btn"), egui::Sense::click());
        let btn_color = if btn_resp.hovered() {
            theme.text_primary
        } else {
            theme.text_disabled
        };
        painter.text(
            btn_center,
            egui::Align2::CENTER_CENTER,
            "\u{2715}",
            egui::FontId::proportional(10.0),
            btn_color,
        );
        closed = btn_resp.clicked();
    }
    closed
}

/// Draw a collapsible section header within a panel (lighter bg, icon + title).
/// Returns true if the section is expanded.
pub fn draw_section_header(ui: &mut egui::Ui, id: &str, title: &str, default_open: bool) -> bool {
    let theme = CadTheme::dark();
    let expand_id = ui.id().with(id);
    let expanded = ui.data_mut(|d| *d.get_temp_mut_or(expand_id, default_open));

    let avail_w = ui.available_width();
    let h = theme.section_header_height;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(avail_w, h), egui::Sense::click());

    let painter = ui.painter();
    let bg = if resp.hovered() {
        Color32::from_rgb(
            theme.section_header_bg.r().saturating_add(8),
            theme.section_header_bg.g().saturating_add(8),
            theme.section_header_bg.b().saturating_add(8),
        )
    } else {
        theme.section_header_bg
    };
    painter.rect_filled(rect, 0.0, bg);

    // Expand arrow
    let arrow = if expanded { "\u{25BC}" } else { "\u{25B6}" };
    painter.text(
        egui::pos2(rect.left() + 10.0, rect.center().y),
        egui::Align2::CENTER_CENTER,
        arrow,
        egui::FontId::proportional(8.0),
        theme.text_disabled,
    );

    painter.text(
        egui::pos2(rect.left() + 22.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::new(11.0, egui::FontFamily::Proportional),
        theme.section_header_text,
    );

    if resp.clicked() {
        ui.data_mut(|d| *d.get_temp_mut_or(expand_id, default_open) = !expanded);
        !expanded
    } else {
        expanded
    }
}

// ---------------------------------------------------------------------------
// Task panel helpers — FreeCAD TaskView style
// ---------------------------------------------------------------------------

/// Task panel header — accent gradient bar with icon and title.
pub fn draw_task_header(ui: &mut egui::Ui, icon: &str, title: &str) {
    let avail_w = ui.available_width();
    let h = 28.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(avail_w, h), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        // Gradient from accent-dark to bg
        painter.rect_filled(rect, 2.0, Color32::from_rgb(0x0A, 0x30, 0x58));
        // Left accent bar
        painter.rect_filled(
            egui::Rect::from_min_size(rect.left_top(), egui::vec2(3.0, h)),
            1.0,
            COLOR_ACCENT,
        );
        // Icon
        painter.text(
            egui::pos2(rect.left() + 12.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            icon,
            egui::FontId::proportional(14.0),
            Color32::from_rgb(100, 170, 240),
        );
        // Title
        painter.text(
            egui::pos2(rect.left() + 30.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            title,
            egui::FontId::new(12.5, egui::FontFamily::Proportional),
            Color32::WHITE,
        );
    }
    ui.add_space(4.0);
}

/// Task section label — small dim text with underline.
pub fn draw_task_section(ui: &mut egui::Ui, label: &str) {
    ui.add_space(6.0);
    let avail_w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(avail_w, 16.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.text(
            egui::pos2(rect.left() + 2.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::new(10.5, egui::FontFamily::Proportional),
            COLOR_ACCENT,
        );
        // Underline
        painter.line_segment(
            [
                egui::pos2(rect.left(), rect.bottom()),
                egui::pos2(rect.right(), rect.bottom()),
            ],
            egui::Stroke::new(1.0, Color32::from_rgba_premultiplied(80, 140, 220, 40)),
        );
    }
    ui.add_space(2.0);
}

/// Styled OK / Cancel button pair for task panel.
/// Returns (ok_clicked, cancel_clicked).
pub fn draw_task_buttons(ui: &mut egui::Ui) -> (bool, bool) {
    ui.add_space(6.0);
    // Separator
    let avail_w = ui.available_width();
    let (sep_rect, _) = ui.allocate_exact_size(egui::vec2(avail_w, 1.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(sep_rect, 0.0, Color32::from_gray(50));
    ui.add_space(6.0);

    let mut ok = false;
    let mut cancel = false;
    ui.horizontal(|ui| {
        ok = ui
            .add(
                egui::Button::new(
                    egui::RichText::new("\u{2714} OK")
                        .strong()
                        .color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(0, 100, 180))
                .min_size(egui::vec2(60.0, 24.0)),
            )
            .clicked();
        cancel = ui
            .add(egui::Button::new("\u{2716} Cancel").min_size(egui::vec2(60.0, 24.0)))
            .clicked();
    });
    (ok, cancel)
}

/// Subtle horizontal separator line.
pub fn draw_separator(ui: &mut egui::Ui) {
    let theme = CadTheme::dark();
    let avail_w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(avail_w, 1.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0.0, theme.panel_separator);
}
