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

use super::tokens;
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
    let palette_width = 560.0_f32.min(screen.width() - 32.0);
    let palette_height = 420.0_f32.min(screen.height() - 64.0);

    egui::Area::new(egui::Id::new("__command_palette"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 80.0))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .inner_margin(egui::Margin::same(tokens::SPACE_MD as i8))
                .corner_radius(egui::CornerRadius::same(tokens::RADIUS_LG as u8))
                .show(ui, |ui| {
                    ui.set_width(palette_width);
                    ui.set_max_height(palette_height);

                    // Search input.
                    let edit = egui::TextEdit::singleline(&mut gui.command_palette.query)
                        .hint_text("Type a command…")
                        .font(egui::FontId::proportional(tokens::FONT_LG))
                        .desired_width(palette_width - 2.0 * tokens::SPACE_MD);
                    let resp = ui.add(edit);
                    if gui.command_palette.request_focus {
                        resp.request_focus();
                        gui.command_palette.request_focus = false;
                    }

                    ui.add_space(tokens::SPACE_XS);
                    ui.separator();
                    ui.add_space(tokens::SPACE_XS);

                    if scored.is_empty() {
                        ui.weak("No matching commands.");
                        return;
                    }

                    egui::ScrollArea::vertical()
                        .max_height(palette_height - 80.0)
                        .show(ui, |ui| {
                            for (visible_row, (_score, entry_idx)) in scored.iter().enumerate() {
                                let entry = &entries[*entry_idx];
                                let selected = visible_row == gui.command_palette.selected;
                                let row = ui.horizontal(|ui| {
                                    ui.set_min_width(palette_width - 2.0 * tokens::SPACE_MD);
                                    let bg = if selected {
                                        ui.visuals().selection.bg_fill
                                    } else {
                                        egui::Color32::TRANSPARENT
                                    };
                                    let text_color = if selected {
                                        ui.visuals().selection.stroke.color
                                    } else {
                                        ui.visuals().text_color()
                                    };
                                    let row_rect = ui.available_rect_before_wrap();
                                    ui.painter().rect_filled(
                                        egui::Rect::from_min_size(
                                            row_rect.min,
                                            egui::vec2(row_rect.width(), tokens::HIT_TARGET_MIN),
                                        ),
                                        egui::CornerRadius::same(tokens::RADIUS_SM as u8),
                                        bg,
                                    );

                                    ui.add_space(tokens::SPACE_SM);
                                    ui.vertical(|ui| {
                                        ui.add_space(tokens::SPACE_XS);
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(entry.label)
                                                    .color(text_color)
                                                    .size(tokens::FONT_MD),
                                            );
                                            ui.add_space(tokens::SPACE_SM);
                                            ui.weak(
                                                egui::RichText::new(entry.category)
                                                    .size(tokens::FONT_SM),
                                            );
                                        });
                                    });

                                    if let Some(sc) = entry.shortcut {
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.add_space(tokens::SPACE_SM);
                                                ui.weak(
                                                    egui::RichText::new(sc)
                                                        .size(tokens::FONT_SM)
                                                        .monospace(),
                                                );
                                            },
                                        );
                                    }
                                });

                                if row.response.interact(egui::Sense::click()).clicked() {
                                    to_dispatch = Some(entry.run);
                                }
                            }
                        });

                    ui.add_space(tokens::SPACE_XS);
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.weak(
                            egui::RichText::new("↑↓ navigate    ↵ run    Esc close")
                                .size(tokens::FONT_SM),
                        );
                    });
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
