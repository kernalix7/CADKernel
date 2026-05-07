//! Command palette — VS Code-style fuzzy command finder.
//!
//! Opens with **Ctrl + Shift + P** (Cmd + Shift + P on macOS). Type to
//! filter by command label; arrow keys navigate; Enter dispatches; Esc
//! closes. Results are scored by a simple fuzzy match (subsequence with
//! contiguous-bonus + start-of-word-bonus).
//!
//! Commands dispatch via the existing `gui.actions` queue — the palette
//! never executes work itself, it only enqueues `GuiAction` variants.
//!
//! Reference: `docs/COMMERCIAL_CAD_ROADMAP.md` Track B §B15 (search).

use super::theme;
use super::{GuiAction, GuiState};
use crate::render::{DisplayMode, StandardView};

/// Internal state for the command palette popup.
#[derive(Default)]
pub(crate) struct CommandPaletteState {
    /// Whether the palette is currently visible.
    pub open: bool,
    /// Current search query.
    pub query: String,
    /// Index of the highlighted result (within the filtered list).
    pub selected: usize,
    /// Set to `true` for one frame after opening so the input grabs focus.
    pub request_focus: bool,
}

impl CommandPaletteState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Toggle the palette open/closed and reset state.
    pub fn toggle(&mut self) {
        if self.open {
            self.close();
        } else {
            self.open = true;
            self.query.clear();
            self.selected = 0;
            self.request_focus = true;
        }
    }

    pub fn close(&mut self) {
        self.open = false;
        self.query.clear();
        self.selected = 0;
        self.request_focus = false;
    }
}

/// One palette entry. `run` is a function pointer that mutates `GuiState`
/// directly — most entries enqueue a `GuiAction`, but some (e.g. opening
/// a dialog) flip a UI flag instead.
struct Entry {
    label: &'static str,
    category: &'static str,
    shortcut: Option<&'static str>,
    run: fn(&mut GuiState),
}

/// Static catalogue of palette-reachable commands.
///
/// New commands are added by appending an `Entry` here. Categories are
/// free-form strings shown in dim text after the label.
fn catalogue() -> Vec<Entry> {
    vec![
        // ---- File ----
        Entry { label: "New Model",          category: "File",    shortcut: Some("Ctrl+N"),       run: |g| g.actions.push(GuiAction::NewModel) },
        Entry { label: "Clear Recent Files", category: "File",    shortcut: None,                  run: |g| g.actions.push(GuiAction::ClearRecentFiles) },
        // ---- View / Camera ----
        Entry { label: "Reset Camera",       category: "View",    shortcut: Some("Home"),          run: |g| g.actions.push(GuiAction::ResetCamera) },
        Entry { label: "Fit All",            category: "View",    shortcut: Some("F"),             run: |g| g.actions.push(GuiAction::FitAll) },
        Entry { label: "Toggle Projection",  category: "View",    shortcut: Some("5"),             run: |g| g.actions.push(GuiAction::ToggleProjection) },
        Entry { label: "Toggle Grid",        category: "View",    shortcut: Some("G"),             run: |g| g.actions.push(GuiAction::ToggleGrid) },
        Entry { label: "View: Front",        category: "View",    shortcut: Some("1"),             run: |g| g.actions.push(GuiAction::SetStandardView(StandardView::Front)) },
        Entry { label: "View: Back",         category: "View",    shortcut: Some("Shift+1"),       run: |g| g.actions.push(GuiAction::SetStandardView(StandardView::Back)) },
        Entry { label: "View: Right",        category: "View",    shortcut: Some("3"),             run: |g| g.actions.push(GuiAction::SetStandardView(StandardView::Right)) },
        Entry { label: "View: Left",         category: "View",    shortcut: Some("Shift+3"),       run: |g| g.actions.push(GuiAction::SetStandardView(StandardView::Left)) },
        Entry { label: "View: Top",          category: "View",    shortcut: Some("7"),             run: |g| g.actions.push(GuiAction::SetStandardView(StandardView::Top)) },
        Entry { label: "View: Bottom",       category: "View",    shortcut: Some("Shift+7"),       run: |g| g.actions.push(GuiAction::SetStandardView(StandardView::Bottom)) },
        Entry { label: "View: Isometric",    category: "View",    shortcut: Some("0"),             run: |g| g.actions.push(GuiAction::SetStandardView(StandardView::Isometric)) },
        // ---- Display modes ----
        Entry { label: "Display: Shading",      category: "Display", shortcut: Some(DisplayMode::Shading.shortcut()),    run: |g| g.actions.push(GuiAction::SetDisplayMode(DisplayMode::Shading)) },
        Entry { label: "Display: Wireframe",    category: "Display", shortcut: Some(DisplayMode::Wireframe.shortcut()),  run: |g| g.actions.push(GuiAction::SetDisplayMode(DisplayMode::Wireframe)) },
        Entry { label: "Display: Hidden Line",  category: "Display", shortcut: Some(DisplayMode::HiddenLine.shortcut()), run: |g| g.actions.push(GuiAction::SetDisplayMode(DisplayMode::HiddenLine)) },
        Entry { label: "Display: Flat Lines",   category: "Display", shortcut: Some(DisplayMode::FlatLines.shortcut()),  run: |g| g.actions.push(GuiAction::SetDisplayMode(DisplayMode::FlatLines)) },
        Entry { label: "Display: Points",       category: "Display", shortcut: Some(DisplayMode::Points.shortcut()),     run: |g| g.actions.push(GuiAction::SetDisplayMode(DisplayMode::Points)) },
        Entry { label: "Display: Transparent",  category: "Display", shortcut: Some(DisplayMode::Transparent.shortcut()), run: |g| g.actions.push(GuiAction::SetDisplayMode(DisplayMode::Transparent)) },
        // ---- Edit ----
        Entry { label: "Undo", category: "Edit", shortcut: Some("Ctrl+Z"),       run: |g| g.actions.push(GuiAction::Undo) },
        Entry { label: "Redo", category: "Edit", shortcut: Some("Ctrl+Shift+Z"), run: |g| g.actions.push(GuiAction::Redo) },
        Entry { label: "Select All",      category: "Edit", shortcut: Some("Ctrl+A"),    run: |g| g.actions.push(GuiAction::SelectAll) },
        Entry { label: "Deselect All",    category: "Edit", shortcut: Some("Esc"),       run: |g| g.actions.push(GuiAction::DeselectAll) },
        Entry { label: "Delete Selected", category: "Edit", shortcut: Some("Delete"),    run: |g| g.actions.push(GuiAction::DeleteSelected) },
        // ---- Visibility ----
        Entry { label: "Show All", category: "Visibility", shortcut: None, run: |g| g.actions.push(GuiAction::ShowAll) },
        Entry { label: "Hide All", category: "Visibility", shortcut: None, run: |g| g.actions.push(GuiAction::HideAll) },
        // ---- Boolean (scene) ----
        Entry { label: "Boolean: Union (Selected)",     category: "Boolean", shortcut: None, run: |g| g.actions.push(GuiAction::BooleanSceneUnion) },
        Entry { label: "Boolean: Subtract (Selected)",  category: "Boolean", shortcut: None, run: |g| g.actions.push(GuiAction::BooleanSceneSubtract) },
        Entry { label: "Boolean: Intersect (Selected)", category: "Boolean", shortcut: None, run: |g| g.actions.push(GuiAction::BooleanSceneIntersect) },
        // ---- Create ----
        Entry { label: "Create: Box (10×10×10)",        category: "Create", shortcut: None, run: |g| g.actions.push(GuiAction::CreateBox      { width: 10.0, height: 10.0, depth: 10.0 }) },
        Entry { label: "Create: Cylinder (r=5, h=10)",  category: "Create", shortcut: None, run: |g| g.actions.push(GuiAction::CreateCylinder { radius: 5.0, height: 10.0 }) },
        Entry { label: "Create: Sphere (r=5)",          category: "Create", shortcut: None, run: |g| g.actions.push(GuiAction::CreateSphere   { radius: 5.0 }) },
        Entry { label: "Create: Cone (5→0, h=10)",      category: "Create", shortcut: None, run: |g| g.actions.push(GuiAction::CreateCone     { base_radius: 5.0, top_radius: 0.0, height: 10.0 }) },
        Entry { label: "Create: Torus (R=10, r=2)",     category: "Create", shortcut: None, run: |g| g.actions.push(GuiAction::CreateTorus    { major_radius: 10.0, minor_radius: 2.0 }) },
        Entry { label: "Create: Tube (R=10, r=8, h=10)",category: "Create", shortcut: None, run: |g| g.actions.push(GuiAction::CreateTube     { outer_radius: 10.0, inner_radius: 8.0, height: 10.0 }) },
        Entry { label: "Create: Wedge",                 category: "Create", shortcut: None, run: |g| g.actions.push(GuiAction::CreateWedge    { dx: 10.0, dy: 10.0, dz: 5.0, dx2: 6.0, dy2: 6.0 }) },
        Entry { label: "Create: Ellipsoid (5,3,2)",     category: "Create", shortcut: None, run: |g| g.actions.push(GuiAction::CreateEllipsoid{ rx: 5.0, ry: 3.0, rz: 2.0 }) },
        // ---- Help ----
        Entry { label: "Help: Keyboard Shortcuts", category: "Help", shortcut: Some("F1"), run: |g| { g.show_shortcuts = true; } },
    ]
}

// ---------------------------------------------------------------------------
// Fuzzy match — subsequence scoring with start-of-word and contiguous bonus
// ---------------------------------------------------------------------------

/// Score `query` against `target`. Returns `Some(score)` if every character
/// in `query` appears in `target` in order (case-insensitive), else `None`.
/// Higher score = better match. Empty query scores 0 (match all).
fn fuzzy_score(query: &str, target: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }
    let query_lower: Vec<char> = query.to_lowercase().chars().collect();
    let target_lower: Vec<char> = target.to_lowercase().chars().collect();

    let mut score: i32 = 0;
    let mut qi: usize = 0;
    let mut last_match_idx: Option<usize> = None;

    for (ti, tc) in target_lower.iter().enumerate() {
        if qi >= query_lower.len() {
            break;
        }
        if query_lower[qi] == *tc {
            // Base point for a match.
            score += 1;
            // Contiguous bonus: previous query char matched the previous target char.
            if let Some(prev) = last_match_idx
                && prev + 1 == ti
            {
                score += 5;
            }
            // Start-of-word bonus: matched at index 0, or after a space / colon.
            if ti == 0
                || matches!(
                    target_lower[ti - 1],
                    ' ' | ':' | '/' | '-' | '_' | '(' | ','
                )
            {
                score += 8;
            }
            last_match_idx = Some(ti);
            qi += 1;
        }
    }

    if qi == query_lower.len() {
        // Bonus for short targets (less noise).
        score += (50 - target.len() as i32).max(0);
        Some(score)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Render
// ---------------------------------------------------------------------------

/// Detect the global Ctrl+Shift+P shortcut and toggle the palette. Call
/// once per frame before `draw_command_palette`.
pub(crate) fn handle_global_shortcut(ctx: &egui::Context, state: &mut CommandPaletteState) {
    let triggered = ctx.input_mut(|i| {
        i.consume_shortcut(&egui::KeyboardShortcut::new(
            egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
            egui::Key::P,
        ))
    });
    if triggered {
        state.toggle();
    }
}

/// Render the palette modal. Pushes the chosen `GuiAction` (if any) into
/// `gui.actions` and closes the palette.
pub(crate) fn draw_command_palette(ctx: &egui::Context, gui: &mut GuiState) {
    if !gui.command_palette.open {
        return;
    }

    // Close on Escape.
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        gui.command_palette.close();
        return;
    }

    // Score and sort the catalogue once per frame.
    let entries = catalogue();
    let query = gui.command_palette.query.clone();
    let mut scored: Vec<(i32, usize)> = entries
        .iter()
        .enumerate()
        .filter_map(|(idx, e)| fuzzy_score(&query, e.label).map(|s| (s, idx)))
        .collect();
    scored.sort_by_key(|s| std::cmp::Reverse(s.0));
    let max_results = 12usize;
    scored.truncate(max_results);

    // Clamp selected index.
    if scored.is_empty() {
        gui.command_palette.selected = 0;
    } else if gui.command_palette.selected >= scored.len() {
        gui.command_palette.selected = scored.len() - 1;
    }

    // Arrow-key navigation. Read inputs once for the frame.
    let (down_pressed, up_pressed, enter_pressed) = ctx.input(|i| {
        (
            i.key_pressed(egui::Key::ArrowDown),
            i.key_pressed(egui::Key::ArrowUp),
            i.key_pressed(egui::Key::Enter),
        )
    });
    if !scored.is_empty() {
        if down_pressed {
            gui.command_palette.selected = (gui.command_palette.selected + 1) % scored.len();
        }
        if up_pressed {
            gui.command_palette.selected = if gui.command_palette.selected == 0 {
                scored.len() - 1
            } else {
                gui.command_palette.selected - 1
            };
        }
    }

    // Decide whether to dispatch _after_ rendering (so the closed state
    // isn't inconsistent during the same frame).
    let mut to_dispatch: Option<fn(&mut GuiState)> = None;

    let screen = ctx.screen_rect();
    let palette_width = 600.0_f32.min(screen.width() - 32.0);
    let palette_height = 460.0_f32.min(screen.height() - 64.0);

    egui::Area::new(egui::Id::new("__command_palette"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 92.0))
        .show(ctx, |ui| {
            // Themed popup frame: dark panel with thin teal accent stroke +
            // soft drop shadow, matches the rest of the CADKernel chrome.
            let frame = egui::Frame {
                fill: egui::Color32::from_rgb(0x1C, 0x20, 0x28),
                stroke: egui::Stroke::new(1.0, theme::COLOR_ACCENT.gamma_multiply(0.55)),
                inner_margin: egui::Margin::same(0),
                corner_radius: egui::CornerRadius::same(10),
                shadow: egui::epaint::Shadow {
                    offset: [0, 8],
                    blur: 24,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(110),
                },
                ..egui::Frame::NONE
            };
            frame.show(ui, |ui| {
                ui.set_width(palette_width);
                ui.set_max_height(palette_height);

                // -- Search header --
                let header_h = 44.0;
                let header_rect = ui.allocate_space(egui::vec2(palette_width, header_h)).1;
                let painter = ui.painter();
                painter.rect_filled(
                    header_rect,
                    egui::CornerRadius {
                        nw: 10,
                        ne: 10,
                        sw: 0,
                        se: 0,
                    },
                    egui::Color32::from_rgb(0x16, 0x19, 0x20),
                );
                painter.line_segment(
                    [
                        egui::pos2(header_rect.left() + 8.0, header_rect.bottom() - 0.5),
                        egui::pos2(header_rect.right() - 8.0, header_rect.bottom() - 0.5),
                    ],
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(0x10, 0x13, 0x19)),
                );
                // Magnifier glyph
                painter.text(
                    egui::pos2(header_rect.left() + 16.0, header_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    "\u{1F50D}",
                    egui::FontId::proportional(15.0),
                    theme::COLOR_ACCENT,
                );

                // Place the text edit inside the header rect.
                let edit_rect = egui::Rect::from_min_size(
                    egui::pos2(header_rect.left() + 38.0, header_rect.top() + 8.0),
                    egui::vec2(palette_width - 76.0, header_h - 16.0),
                );
                let edit_resp = ui.put(
                    edit_rect,
                    egui::TextEdit::singleline(&mut gui.command_palette.query)
                        .hint_text("Type a command, view, or tool…")
                        .font(egui::FontId::proportional(14.0))
                        .frame(false)
                        .text_color(egui::Color32::from_rgb(220, 226, 235)),
                );
                if gui.command_palette.request_focus {
                    edit_resp.request_focus();
                    gui.command_palette.request_focus = false;
                }
                // Match-count chip on the right edge of the header.
                let chip_text = if scored.is_empty() {
                    "no matches".to_string()
                } else {
                    format!("{} match{}", scored.len(), if scored.len() == 1 { "" } else { "es" })
                };
                let chip_color = if scored.is_empty() {
                    egui::Color32::from_rgb(180, 110, 110)
                } else {
                    theme::COLOR_ACCENT
                };
                let painter = ui.painter();
                painter.text(
                    egui::pos2(header_rect.right() - 14.0, header_rect.center().y),
                    egui::Align2::RIGHT_CENTER,
                    chip_text,
                    egui::FontId::proportional(10.5),
                    chip_color,
                );

                // -- Results list --
                if scored.is_empty() {
                    ui.add_space(28.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new("No commands match \u{201C}")
                                .color(theme::COLOR_DIM)
                                .size(12.0),
                        );
                        ui.label(
                            egui::RichText::new(format!("\u{201C}{}\u{201D}", gui.command_palette.query))
                                .color(egui::Color32::from_rgb(200, 210, 225))
                                .size(13.0)
                                .italics(),
                        );
                    });
                    ui.add_space(20.0);
                } else {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .max_height(palette_height - header_h - 32.0)
                        .show(ui, |ui| {
                            ui.add_space(4.0);
                            for (visible_row, (_score, entry_idx)) in scored.iter().enumerate() {
                                let entry = &entries[*entry_idx];
                                let selected = visible_row == gui.command_palette.selected;
                                let row_h = 34.0;
                                let (row_rect, row_resp) = ui.allocate_exact_size(
                                    egui::vec2(palette_width - 8.0, row_h),
                                    egui::Sense::click(),
                                );
                                // Inset the row so we get a 4px margin on each side.
                                let inset = row_rect.shrink2(egui::vec2(4.0, 1.0));
                                let painter = ui.painter();
                                let bg = if selected {
                                    theme::COLOR_ACCENT.gamma_multiply(0.18)
                                } else if row_resp.hovered() {
                                    egui::Color32::from_rgb(0x24, 0x29, 0x33)
                                } else {
                                    egui::Color32::TRANSPARENT
                                };
                                painter.rect_filled(inset, egui::CornerRadius::same(6), bg);
                                if selected {
                                    // Teal accent left bar.
                                    painter.rect_filled(
                                        egui::Rect::from_min_size(
                                            inset.min,
                                            egui::vec2(3.0, inset.height()),
                                        ),
                                        egui::CornerRadius {
                                            nw: 6,
                                            sw: 6,
                                            ne: 0,
                                            se: 0,
                                        },
                                        theme::COLOR_ACCENT,
                                    );
                                }

                                // Label
                                let label_color = if selected {
                                    egui::Color32::from_rgb(232, 238, 246)
                                } else {
                                    egui::Color32::from_rgb(210, 216, 226)
                                };
                                painter.text(
                                    egui::pos2(inset.left() + 14.0, inset.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    entry.label,
                                    egui::FontId::proportional(13.0),
                                    label_color,
                                );

                                // Category chip on the right side, before the shortcut.
                                let mut right_x = inset.right() - 8.0;
                                if let Some(sc) = entry.shortcut {
                                    let font = egui::FontId::monospace(10.5);
                                    let g = painter.layout_no_wrap(
                                        sc.to_string(),
                                        font.clone(),
                                        theme::COLOR_DIM,
                                    );
                                    let pad = 6.0;
                                    let chip_w = g.size().x + pad * 2.0;
                                    let chip_rect = egui::Rect::from_min_size(
                                        egui::pos2(right_x - chip_w, inset.center().y - 8.0),
                                        egui::vec2(chip_w, 16.0),
                                    );
                                    painter.rect_filled(
                                        chip_rect,
                                        egui::CornerRadius::same(4),
                                        egui::Color32::from_rgb(0x14, 0x17, 0x1D),
                                    );
                                    painter.rect_stroke(
                                        chip_rect,
                                        egui::CornerRadius::same(4),
                                        egui::Stroke::new(
                                            0.6,
                                            egui::Color32::from_rgb(0x35, 0x3C, 0x48),
                                        ),
                                        egui::StrokeKind::Inside,
                                    );
                                    painter.galley(
                                        egui::pos2(
                                            chip_rect.left() + pad,
                                            chip_rect.center().y - g.size().y * 0.5,
                                        ),
                                        g,
                                        theme::COLOR_DIM,
                                    );
                                    right_x = chip_rect.left() - 6.0;
                                }
                                // Category text right-aligned.
                                painter.text(
                                    egui::pos2(right_x, inset.center().y),
                                    egui::Align2::RIGHT_CENTER,
                                    entry.category,
                                    egui::FontId::proportional(10.5),
                                    theme::COLOR_DIM,
                                );

                                if row_resp.clicked() {
                                    to_dispatch = Some(entry.run);
                                }
                            }
                            ui.add_space(4.0);
                        });
                }

                // -- Footer hints --
                let footer_h = 26.0;
                let footer_rect = ui
                    .allocate_space(egui::vec2(palette_width, footer_h))
                    .1;
                let painter = ui.painter();
                painter.rect_filled(
                    footer_rect,
                    egui::CornerRadius {
                        nw: 0,
                        ne: 0,
                        sw: 10,
                        se: 10,
                    },
                    egui::Color32::from_rgb(0x16, 0x19, 0x20),
                );
                painter.line_segment(
                    [
                        egui::pos2(footer_rect.left() + 8.0, footer_rect.top() + 0.5),
                        egui::pos2(footer_rect.right() - 8.0, footer_rect.top() + 0.5),
                    ],
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(0x10, 0x13, 0x19)),
                );
                painter.text(
                    egui::pos2(footer_rect.left() + 14.0, footer_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    "\u{2191}\u{2193} navigate    \u{21B5} run    Esc close",
                    egui::FontId::proportional(10.5),
                    theme::COLOR_DIM,
                );
                painter.text(
                    egui::pos2(footer_rect.right() - 14.0, footer_rect.center().y),
                    egui::Align2::RIGHT_CENTER,
                    "Ctrl+P",
                    egui::FontId::monospace(10.0),
                    egui::Color32::from_rgb(120, 130, 145),
                );
            });
        });

    // Enter dispatches the highlighted entry.
    if enter_pressed && !scored.is_empty() {
        let (_, entry_idx) = scored[gui.command_palette.selected];
        to_dispatch = Some(entries[entry_idx].run);
    }

    if let Some(run) = to_dispatch {
        run(gui);
        gui.command_palette.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_matches_all() {
        assert_eq!(fuzzy_score("", "anything"), Some(0));
    }

    #[test]
    fn out_of_order_does_not_match() {
        assert!(fuzzy_score("xy", "yx").is_none());
    }

    #[test]
    fn subsequence_matches() {
        assert!(fuzzy_score("vfr", "View: Front").is_some());
    }

    #[test]
    fn contiguous_beats_scattered() {
        let contiguous = fuzzy_score("box", "box").unwrap();
        let scattered = fuzzy_score("box", "back of xenon").unwrap();
        assert!(
            contiguous > scattered,
            "contiguous score {contiguous} should beat scattered {scattered}"
        );
    }

    #[test]
    fn catalogue_is_nonempty() {
        assert!(!catalogue().is_empty());
    }

    #[test]
    fn toggle_open_close() {
        let mut s = CommandPaletteState::new();
        assert!(!s.open);
        s.toggle();
        assert!(s.open);
        assert!(s.request_focus);
        s.toggle();
        assert!(!s.open);
    }
}
