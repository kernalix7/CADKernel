use super::theme;
use super::{GuiAction, GuiState};
use crate::scene::{CreationParams, ObjectId, Scene, SceneObject};

// ---------------------------------------------------------------------------
// Entity icon types
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
enum EntityIcon {
    Solid,
    Face,
    Edge,
    Vertex,
    Sketch,
    Extrude,
    Revolve,
    Boolean,
    Pattern,
    Fillet,
    Chamfer,
    Assembly,
    Component,
    Imported,
    Body,
}

/// Draw a 14x14 procedural icon for an entity type.
fn draw_entity_icon(
    painter: &egui::Painter,
    rect: egui::Rect,
    entity_type: EntityIcon,
    color: egui::Color32,
) {
    let c = rect.center();
    let r = rect.width() * 0.4;
    let stroke = egui::Stroke::new(1.2, color);
    let thin = egui::Stroke::new(0.8, color);

    match entity_type {
        EntityIcon::Solid => {
            let d = r * 0.65;
            let off = r * 0.35;
            let front = egui::Rect::from_center_size(
                egui::pos2(c.x - off * 0.3, c.y + off * 0.3),
                egui::vec2(d * 2.0, d * 2.0),
            );
            painter.rect_stroke(front, 0.0, stroke, egui::StrokeKind::Outside);
            let tl = front.left_top();
            let tr = front.right_top();
            let br = front.right_bottom();
            let back_tl = egui::pos2(tl.x + off, tl.y - off);
            let back_tr = egui::pos2(tr.x + off, tr.y - off);
            let back_br = egui::pos2(br.x + off, br.y - off);
            painter.line_segment([tl, back_tl], thin);
            painter.line_segment([tr, back_tr], thin);
            painter.line_segment([br, back_br], thin);
            painter.line_segment([back_tl, back_tr], thin);
            painter.line_segment([back_tr, back_br], thin);
        }
        EntityIcon::Face => {
            let d = r * 0.7;
            let face_rect = egui::Rect::from_center_size(c, egui::vec2(d * 2.0, d * 1.6));
            painter.rect_filled(face_rect, 1.0, color);
        }
        EntityIcon::Edge => {
            painter.line_segment(
                [egui::pos2(c.x - r, c.y + r), egui::pos2(c.x + r, c.y - r)],
                egui::Stroke::new(1.8, color),
            );
        }
        EntityIcon::Vertex => {
            painter.circle_filled(c, r * 0.5, color);
        }
        EntityIcon::Sketch => {
            let p1 = egui::pos2(c.x - r, c.y + r);
            let p2 = egui::pos2(c.x + r * 0.6, c.y - r * 0.6);
            painter.line_segment([p1, p2], egui::Stroke::new(1.5, color));
            let tip_size = r * 0.35;
            let tip_dir_x = (p1.x - p2.x) / r;
            let tip_dir_y = (p1.y - p2.y) / r;
            let tip = egui::pos2(p1.x + tip_dir_x * tip_size, p1.y + tip_dir_y * tip_size);
            painter.circle_filled(tip, 1.5, color);
        }
        EntityIcon::Extrude => {
            let half_w = r * 0.5;
            let half_h = r * 0.35;
            let base = egui::pos2(c.x, c.y + r * 0.3);
            painter.rect_stroke(
                egui::Rect::from_center_size(base, egui::vec2(half_w * 2.0, half_h * 2.0)),
                0.0,
                thin,
                egui::StrokeKind::Outside,
            );
            let arrow_base = egui::pos2(c.x, c.y);
            let arrow_tip = egui::pos2(c.x, c.y - r * 0.9);
            painter.line_segment([arrow_base, arrow_tip], stroke);
            painter.line_segment(
                [
                    arrow_tip,
                    egui::pos2(arrow_tip.x - r * 0.25, arrow_tip.y + r * 0.25),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    arrow_tip,
                    egui::pos2(arrow_tip.x + r * 0.25, arrow_tip.y + r * 0.25),
                ],
                stroke,
            );
        }
        EntityIcon::Revolve => {
            let arc_r = r * 0.7;
            let n = 8;
            let start_angle = -std::f32::consts::FRAC_PI_4;
            let end_angle = std::f32::consts::PI + std::f32::consts::FRAC_PI_4;
            for i in 0..n {
                let t0 = start_angle + (end_angle - start_angle) * (i as f32 / n as f32);
                let t1 = start_angle + (end_angle - start_angle) * ((i + 1) as f32 / n as f32);
                painter.line_segment(
                    [
                        egui::pos2(c.x + arc_r * t0.cos(), c.y - arc_r * t0.sin()),
                        egui::pos2(c.x + arc_r * t1.cos(), c.y - arc_r * t1.sin()),
                    ],
                    stroke,
                );
            }
            let end_pt = egui::pos2(c.x + arc_r * end_angle.cos(), c.y - arc_r * end_angle.sin());
            painter.circle_filled(end_pt, 1.5, color);
        }
        EntityIcon::Boolean => {
            let off = r * 0.3;
            let cr = r * 0.5;
            painter.circle_stroke(egui::pos2(c.x - off, c.y), cr, stroke);
            painter.circle_stroke(egui::pos2(c.x + off, c.y), cr, stroke);
        }
        EntityIcon::Pattern => {
            let spacing = r * 0.5;
            let dot_r = 1.5;
            for dx in [-1.0_f32, 1.0] {
                for dy in [-1.0_f32, 1.0] {
                    painter.circle_filled(
                        egui::pos2(c.x + dx * spacing, c.y + dy * spacing),
                        dot_r,
                        color,
                    );
                }
            }
        }
        EntityIcon::Fillet => {
            let d = r * 0.7;
            let tl = egui::pos2(c.x - d, c.y - d);
            let bl = egui::pos2(c.x - d, c.y + d);
            let br = egui::pos2(c.x + d, c.y + d);
            let tr_x = egui::pos2(c.x + d, c.y);
            let tr_y = egui::pos2(c.x, c.y - d);
            painter.line_segment([tl, bl], thin);
            painter.line_segment([bl, br], thin);
            painter.line_segment([br, tr_x], thin);
            painter.line_segment([tr_y, tl], thin);
            let arc_n = 6;
            for i in 0..arc_n {
                let t0 = (i as f32 / arc_n as f32) * std::f32::consts::FRAC_PI_2;
                let t1 = ((i + 1) as f32 / arc_n as f32) * std::f32::consts::FRAC_PI_2;
                let p0 = egui::pos2(c.x + d * t0.cos(), c.y - d * t0.sin());
                let p1 = egui::pos2(c.x + d * t1.cos(), c.y - d * t1.sin());
                painter.line_segment([p0, p1], stroke);
            }
        }
        EntityIcon::Chamfer => {
            let d = r * 0.7;
            let tl = egui::pos2(c.x - d, c.y - d);
            let bl = egui::pos2(c.x - d, c.y + d);
            let br = egui::pos2(c.x + d, c.y + d);
            let cham1 = egui::pos2(c.x + d, c.y + d * 0.1);
            let cham2 = egui::pos2(c.x + d * 0.1, c.y - d);
            painter.line_segment([tl, bl], thin);
            painter.line_segment([bl, br], thin);
            painter.line_segment([br, cham1], thin);
            painter.line_segment([cham1, cham2], stroke);
            painter.line_segment([cham2, tl], thin);
        }
        EntityIcon::Assembly => {
            let d = r * 0.5;
            painter.rect_stroke(
                egui::Rect::from_center_size(
                    egui::pos2(c.x - d * 0.3, c.y - d * 0.3),
                    egui::vec2(d * 1.4, d * 1.4),
                ),
                0.0,
                stroke,
                egui::StrokeKind::Outside,
            );
            painter.rect_stroke(
                egui::Rect::from_center_size(
                    egui::pos2(c.x + d * 0.3, c.y + d * 0.3),
                    egui::vec2(d * 1.4, d * 1.4),
                ),
                0.0,
                thin,
                egui::StrokeKind::Outside,
            );
        }
        EntityIcon::Component => {
            let d = r * 0.65;
            painter.rect_stroke(
                egui::Rect::from_center_size(c, egui::vec2(d * 2.0, d * 2.0)),
                1.0,
                thin,
                egui::StrokeKind::Outside,
            );
            painter.line_segment(
                [
                    egui::pos2(c.x, c.y + d * 0.5),
                    egui::pos2(c.x, c.y - d * 0.5),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(c.x, c.y - d * 0.5),
                    egui::pos2(c.x - d * 0.3, c.y - d * 0.1),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(c.x, c.y - d * 0.5),
                    egui::pos2(c.x + d * 0.3, c.y - d * 0.1),
                ],
                stroke,
            );
        }
        EntityIcon::Imported => {
            let d = r * 0.65;
            let box_rect = egui::Rect::from_center_size(
                egui::pos2(c.x + d * 0.2, c.y),
                egui::vec2(d * 1.6, d * 2.0),
            );
            painter.rect_stroke(box_rect, 1.0, thin, egui::StrokeKind::Outside);
            let arrow_start = egui::pos2(c.x - r, c.y);
            let arrow_end = egui::pos2(c.x - d * 0.1, c.y);
            painter.line_segment([arrow_start, arrow_end], stroke);
            painter.line_segment(
                [
                    arrow_end,
                    egui::pos2(arrow_end.x - d * 0.3, arrow_end.y - d * 0.3),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    arrow_end,
                    egui::pos2(arrow_end.x - d * 0.3, arrow_end.y + d * 0.3),
                ],
                stroke,
            );
        }
        EntityIcon::Body => {
            let d = r * 0.6;
            painter.rect_stroke(
                egui::Rect::from_center_size(c, egui::vec2(d * 2.0, d * 2.0)),
                2.0,
                egui::Stroke::new(1.4, color),
                egui::StrokeKind::Outside,
            );
            let inner = r * 0.3;
            painter.rect_filled(
                egui::Rect::from_center_size(c, egui::vec2(inner * 2.0, inner * 2.0)),
                1.0,
                color,
            );
        }
    }
}

/// Map CreationParams to the appropriate EntityIcon.
fn icon_for_params(params: Option<&CreationParams>) -> EntityIcon {
    match params {
        Some(CreationParams::Box { .. })
        | Some(CreationParams::Cylinder { .. })
        | Some(CreationParams::Sphere { .. })
        | Some(CreationParams::Cone { .. })
        | Some(CreationParams::Torus { .. })
        | Some(CreationParams::Tube { .. })
        | Some(CreationParams::Prism { .. })
        | Some(CreationParams::Wedge { .. })
        | Some(CreationParams::Ellipsoid { .. })
        | Some(CreationParams::Helix { .. }) => EntityIcon::Solid,
        Some(CreationParams::Imported { .. }) => EntityIcon::Imported,
        Some(CreationParams::Extruded) => EntityIcon::Extrude,
        Some(CreationParams::Revolved) => EntityIcon::Revolve,
        Some(CreationParams::Boolean { .. }) | Some(CreationParams::BooleanOp { .. }) => {
            EntityIcon::Boolean
        }
        Some(CreationParams::Fillet { .. })
        | Some(CreationParams::Chamfer { .. })
        | Some(CreationParams::Shell { .. })
        | Some(CreationParams::Mirror { .. })
        | Some(CreationParams::Pattern { .. })
        | Some(CreationParams::Groove { .. })
        | Some(CreationParams::Sprocket { .. })
        | Some(CreationParams::InvoluteGear { .. })
        | Some(CreationParams::DraftLine { .. })
        | Some(CreationParams::DraftCircle { .. })
        | Some(CreationParams::DraftRectangle { .. })
        | Some(CreationParams::DraftPolygon { .. })
        | Some(CreationParams::DraftArc { .. })
        | Some(CreationParams::DraftEllipse { .. })
        | Some(CreationParams::SurfacePipe { .. })
        | Some(CreationParams::SurfaceRuled { .. })
        | Some(CreationParams::ScaleOp { .. }) => EntityIcon::Solid,
        None => EntityIcon::Solid,
    }
}

// ---------------------------------------------------------------------------
// Tree node structure
// ---------------------------------------------------------------------------

struct TreeNode {
    name: String,
    icon: EntityIcon,
    object_id: Option<ObjectId>,
    children: Vec<TreeNode>,
    is_body: bool,
    is_tip: bool,
    suppressed: bool,
    has_error: bool,
    needs_recompute: bool,
    is_active_body: bool,
}

fn active_body_id(ui: &egui::Ui) -> Option<ObjectId> {
    let key = egui::Id::new("tree_active_body");
    ui.data(|d| d.get_temp::<ObjectId>(key))
}

fn set_active_body_id(ui: &egui::Ui, id: Option<ObjectId>) {
    let key = egui::Id::new("tree_active_body");
    match id {
        Some(val) => ui.data_mut(|d| d.insert_temp(key, val)),
        None => ui.data_mut(|d| d.remove_by_type::<ObjectId>()),
    }
}

/// Build hierarchical tree from scene objects.
fn build_tree(scene: &Scene, filter: &str, active_body: Option<ObjectId>) -> Vec<TreeNode> {
    let filter_lower = filter.to_lowercase();
    let roots = scene.root_objects();
    let mut nodes: Vec<TreeNode> = Vec::new();

    for obj in &roots {
        if !filter_lower.is_empty() && !name_matches_filter(obj, scene, &filter_lower) {
            continue;
        }

        let is_active = active_body == Some(obj.id);

        if obj.is_body {
            let children = build_body_children(scene, obj.id, &filter_lower);
            nodes.push(TreeNode {
                name: obj.name.clone(),
                icon: EntityIcon::Body,
                object_id: Some(obj.id),
                children,
                is_body: true,
                is_tip: obj.is_tip,
                suppressed: obj.suppressed,
                has_error: obj.has_error,
                needs_recompute: obj.needs_recompute,
                is_active_body: is_active,
            });
        } else {
            let history_children = build_history_children(obj);
            nodes.push(TreeNode {
                name: obj.name.clone(),
                icon: icon_for_object(obj),
                object_id: Some(obj.id),
                children: history_children,
                is_body: false,
                is_tip: obj.is_tip,
                suppressed: obj.suppressed,
                has_error: obj.has_error,
                needs_recompute: obj.needs_recompute,
                is_active_body: false,
            });
        }
    }
    nodes
}

fn name_matches_filter(obj: &SceneObject, scene: &Scene, filter: &str) -> bool {
    if obj.name.to_lowercase().contains(filter) {
        return true;
    }
    if obj.is_body {
        for child in scene.children_of(obj.id) {
            if child.name.to_lowercase().contains(filter) {
                return true;
            }
        }
    }
    false
}

fn build_body_children(scene: &Scene, body_id: ObjectId, filter: &str) -> Vec<TreeNode> {
    let children = scene.children_of(body_id);
    children
        .iter()
        .filter(|c| filter.is_empty() || c.name.to_lowercase().contains(filter))
        .map(|child| {
            let history = build_history_children(child);
            TreeNode {
                name: child.name.clone(),
                icon: icon_for_object(child),
                object_id: Some(child.id),
                children: history,
                is_body: false,
                is_tip: child.is_tip,
                suppressed: child.suppressed,
                has_error: child.has_error,
                needs_recompute: child.needs_recompute,
                is_active_body: false,
            }
        })
        .collect()
}

fn build_history_children(obj: &SceneObject) -> Vec<TreeNode> {
    let records = obj.model.history.records();
    records
        .iter()
        .enumerate()
        .map(|(i, record)| {
            let child_icon = history_label_to_icon(&record.label);
            TreeNode {
                name: format!("{}. {}", i + 1, record.label),
                icon: child_icon,
                object_id: None,
                children: Vec::new(),
                is_body: false,
                is_tip: i + 1 == records.len(),
                suppressed: false,
                has_error: false,
                needs_recompute: false,
                is_active_body: false,
            }
        })
        .collect()
}

fn icon_for_object(obj: &SceneObject) -> EntityIcon {
    if obj.is_body {
        EntityIcon::Body
    } else {
        icon_for_params(obj.params.as_ref())
    }
}

/// Map history record labels to icons.
fn history_label_to_icon(label: &str) -> EntityIcon {
    let lower = label.to_lowercase();
    if lower.contains("extrude") || lower.contains("pad") || lower.contains("pocket") {
        EntityIcon::Extrude
    } else if lower.contains("revolve") || lower.contains("groove") {
        EntityIcon::Revolve
    } else if lower.contains("boolean")
        || lower.contains("union")
        || lower.contains("subtract")
        || lower.contains("intersect")
    {
        EntityIcon::Boolean
    } else if lower.contains("fillet") {
        EntityIcon::Fillet
    } else if lower.contains("chamfer") {
        EntityIcon::Chamfer
    } else if lower.contains("pattern") || lower.contains("array") {
        EntityIcon::Pattern
    } else if lower.contains("sketch") {
        EntityIcon::Sketch
    } else if lower.contains("assembly") {
        EntityIcon::Assembly
    } else {
        EntityIcon::Solid
    }
}

// ---------------------------------------------------------------------------
// Status overlay icons
// ---------------------------------------------------------------------------

const STATUS_ICON_SIZE: f32 = 8.0;

fn draw_status_overlay(painter: &egui::Painter, pos: egui::Pos2, node: &TreeNode) {
    if node.has_error {
        painter.text(
            pos,
            egui::Align2::CENTER_CENTER,
            "\u{26A0}",
            egui::FontId::proportional(STATUS_ICON_SIZE),
            egui::Color32::from_rgb(230, 80, 70),
        );
    } else if node.needs_recompute {
        painter.circle_filled(pos, 3.0, egui::Color32::from_rgb(70, 140, 230));
    } else if node.suppressed {
        painter.text(
            pos,
            egui::Align2::CENTER_CENTER,
            "\u{2013}",
            egui::FontId::proportional(STATUS_ICON_SIZE + 2.0),
            egui::Color32::from_rgb(120, 120, 130),
        );
    }
}

// ---------------------------------------------------------------------------
// Drag-drop visual feedback state
// ---------------------------------------------------------------------------

struct DragState {
    dragging_id: Option<ObjectId>,
    drop_target_y: Option<f32>,
    drop_target_x: f32,
    drop_target_width: f32,
}

impl DragState {
    fn new() -> Self {
        Self {
            dragging_id: None,
            drop_target_y: None,
            drop_target_x: 0.0,
            drop_target_width: 100.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Standalone panel version (deprecated -- kept for compatibility).
#[allow(dead_code)]
pub(crate) fn draw_model_tree(_ctx: &egui::Context, _gui: &mut GuiState, _scene: &Scene) {}

/// Inline version -- draws tree content into an existing Ui.
pub(crate) fn draw_model_tree_inline(ui: &mut egui::Ui, gui: &mut GuiState, scene: &Scene) {
    draw_search_box(ui, gui);

    handle_keyboard_shortcuts(ui, gui);

    if scene.is_empty() {
        ui.add_space(12.0);
        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new("No objects in scene")
                    .size(11.0)
                    .color(egui::Color32::from_rgb(90, 95, 105)),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Use Part workbench to create primitives")
                    .size(10.0)
                    .color(egui::Color32::from_rgb(70, 75, 85)),
            );
        });
        return;
    }

    let active_body = active_body_id(ui);
    let tree = build_tree(scene, &gui.tree_filter, active_body);

    // -- Mini toolbar: filter result count + expand/collapse all --
    {
        let avail_w = ui.available_width();
        let bar_h = 18.0;
        let (bar_rect, _) =
            ui.allocate_exact_size(egui::vec2(avail_w, bar_h), egui::Sense::hover());
        let painter = ui.painter();
        let cy = bar_rect.center().y;

        // Filter result count (left side)
        if !gui.tree_filter.is_empty() {
            let count = tree.len();
            painter.text(
                egui::pos2(bar_rect.left() + 6.0, cy),
                egui::Align2::LEFT_CENTER,
                format!("{count} result{}", if count == 1 { "" } else { "s" }),
                egui::FontId::proportional(9.5),
                theme::COLOR_DIM,
            );
        }

        // Right side: expand/collapse all buttons
        let btn_w = 16.0;
        let expand_rect = egui::Rect::from_center_size(
            egui::pos2(bar_rect.right() - 12.0, cy),
            egui::vec2(btn_w, bar_h),
        );
        let collapse_rect = egui::Rect::from_center_size(
            egui::pos2(bar_rect.right() - 30.0, cy),
            egui::vec2(btn_w, bar_h),
        );

        let expand_resp = ui.interact(
            expand_rect,
            ui.id().with("expand_all"),
            egui::Sense::click(),
        );
        let collapse_resp = ui.interact(
            collapse_rect,
            ui.id().with("collapse_all"),
            egui::Sense::click(),
        );

        let expand_color = if expand_resp.hovered() {
            egui::Color32::from_rgb(180, 190, 205)
        } else {
            egui::Color32::from_rgb(90, 95, 105)
        };
        let collapse_color = if collapse_resp.hovered() {
            egui::Color32::from_rgb(180, 190, 205)
        } else {
            egui::Color32::from_rgb(90, 95, 105)
        };

        // Expand all icon: ▿ (pointing down)
        painter.text(
            expand_rect.center(),
            egui::Align2::CENTER_CENTER,
            "\u{25BF}",
            egui::FontId::proportional(10.0),
            expand_color,
        );
        // Collapse all icon: ▹ (pointing right)
        painter.text(
            collapse_rect.center(),
            egui::Align2::CENTER_CENTER,
            "\u{25B9}",
            egui::FontId::proportional(10.0),
            collapse_color,
        );

        let expand_clicked = expand_resp.clicked();
        let collapse_clicked = collapse_resp.clicked();
        expand_resp.on_hover_text("Expand all");
        collapse_resp.on_hover_text("Collapse all");

        // Handle clicks: set expand state for all tree nodes
        if expand_clicked {
            for node in &tree {
                if let Some(obj_id) = node.object_id {
                    let eid = egui::Id::new(("tree_expand", obj_id));
                    ui.data_mut(|d| d.insert_temp(eid, true));
                }
                for child in &node.children {
                    if let Some(cid) = child.object_id {
                        let eid = egui::Id::new(("tree_expand", cid));
                        ui.data_mut(|d| d.insert_temp(eid, true));
                    }
                }
            }
        }
        if collapse_clicked {
            for node in &tree {
                if let Some(obj_id) = node.object_id {
                    let eid = egui::Id::new(("tree_expand", obj_id));
                    ui.data_mut(|d| d.insert_temp(eid, false));
                }
                for child in &node.children {
                    if let Some(cid) = child.object_id {
                        let eid = egui::Id::new(("tree_expand", cid));
                        ui.data_mut(|d| d.insert_temp(eid, false));
                    }
                }
            }
        }
    }

    // -- Groups section (collapsible) --
    if !scene.groups.is_empty() {
        let group_expanded = theme::draw_section_header(
            ui,
            "tree_groups",
            &format!("Groups ({})", scene.groups.len()),
            true,
        );
        if group_expanded {
            for group in &scene.groups {
                let members = scene.group_members(group.id);
                let gid = group.id;
                let row_h = 20.0;
                let avail_w = ui.available_width();
                let (row_rect, row_resp) =
                    ui.allocate_exact_size(egui::vec2(avail_w, row_h), egui::Sense::click());
                let painter = ui.painter();

                if row_resp.hovered() {
                    painter.rect_filled(row_rect, 0.0, egui::Color32::from_rgb(42, 45, 52));
                }

                let cy = row_rect.center().y;
                let mut x = row_rect.left() + 8.0;

                // Eye icon
                let eye_char = if group.visible {
                    "\u{25C9}"
                } else {
                    "\u{25CB}"
                };
                let eye_color = if group.visible {
                    egui::Color32::from_rgb(90, 185, 110)
                } else {
                    egui::Color32::from_gray(70)
                };
                painter.text(
                    egui::pos2(x + 5.0, cy),
                    egui::Align2::CENTER_CENTER,
                    eye_char,
                    egui::FontId::proportional(11.0),
                    eye_color,
                );
                let clicked_eye = row_resp.clicked()
                    && row_resp
                        .interact_pointer_pos()
                        .is_some_and(|p| p.x < x + 14.0);
                if clicked_eye {
                    gui.actions.push(GuiAction::ToggleGroupVisibility(gid));
                }
                x += 18.0;

                // Folder icon
                painter.text(
                    egui::pos2(x + 5.0, cy),
                    egui::Align2::CENTER_CENTER,
                    "\u{1F4C1}",
                    egui::FontId::proportional(11.0),
                    theme::COLOR_ACCENT,
                );
                x += 18.0;

                // Group name + member count
                let label_color = if group.visible {
                    egui::Color32::from_rgb(175, 185, 200)
                } else {
                    egui::Color32::from_gray(90)
                };
                painter.text(
                    egui::pos2(x, cy),
                    egui::Align2::LEFT_CENTER,
                    format!("{} ({})", group.name, members.len()),
                    egui::FontId::proportional(11.5),
                    label_color,
                );

                // Delete button
                let del_x = row_rect.right() - 16.0;
                if row_resp.hovered() {
                    painter.text(
                        egui::pos2(del_x, cy),
                        egui::Align2::CENTER_CENTER,
                        "\u{2715}",
                        egui::FontId::proportional(9.0),
                        egui::Color32::from_rgb(150, 60, 60),
                    );
                    if row_resp.clicked()
                        && row_resp
                            .interact_pointer_pos()
                            .is_some_and(|p| p.x > del_x - 8.0)
                    {
                        gui.actions.push(GuiAction::DeleteGroup(gid));
                    }
                }
            }
        }
        theme::draw_separator(ui);
    }

    // -- Assembly section (collapsible) --
    draw_assembly_section(ui, gui, Some(scene));

    // -- Document root node --
    {
        let row_h = 22.0;
        let avail_w = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(egui::vec2(avail_w, row_h), egui::Sense::hover());
        let painter = ui.painter();
        let cy = rect.center().y;

        // Document icon (small file icon)
        painter.text(
            egui::pos2(rect.left() + 10.0, cy),
            egui::Align2::CENTER_CENTER,
            "\u{1F4C4}",
            egui::FontId::proportional(11.0),
            egui::Color32::from_rgb(180, 160, 100),
        );
        painter.text(
            egui::pos2(rect.left() + 24.0, cy),
            egui::Align2::LEFT_CENTER,
            "CADKernel",
            egui::FontId::new(11.5, egui::FontFamily::Proportional),
            egui::Color32::from_rgb(170, 175, 185),
        );
        let count_text = format!(
            "{} object{}",
            scene.len(),
            if scene.len() == 1 { "" } else { "s" }
        );
        painter.text(
            egui::pos2(rect.right() - 6.0, cy),
            egui::Align2::RIGHT_CENTER,
            &count_text,
            egui::FontId::proportional(9.5),
            egui::Color32::from_rgb(80, 85, 95),
        );
    }

    let mut drag = DragState::new();

    let total = tree.len();
    for (i, node) in tree.iter().enumerate() {
        let is_last = i + 1 == total;
        draw_tree_node(ui, gui, scene, node, 1, is_last, &mut drag);
    }

    if let Some(drop_y) = drag.drop_target_y {
        let painter = ui.painter();
        let x_start = drag.drop_target_x;
        let x_end = drag.drop_target_x + drag.drop_target_width;
        painter.line_segment(
            [egui::pos2(x_start, drop_y), egui::pos2(x_end, drop_y)],
            egui::Stroke::new(2.0, theme::COLOR_ACCENT),
        );
        painter.circle_filled(egui::pos2(x_start, drop_y), 3.0, theme::COLOR_ACCENT);
    }
}

/// Search/filter box with FreeCAD-style compact styling.
fn draw_search_box(ui: &mut egui::Ui, gui: &mut GuiState) {
    let search_id = egui::Id::new("tree_search_box");
    let avail_w = ui.available_width();
    let h = 22.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(avail_w, h), egui::Sense::hover());

    // Search field background
    let inner = rect.shrink2(egui::vec2(2.0, 1.0));
    ui.painter()
        .rect_filled(inner, 3.0, egui::Color32::from_rgb(30, 32, 38));
    ui.painter().rect_stroke(
        inner,
        3.0,
        egui::Stroke::new(0.5, egui::Color32::from_rgb(55, 58, 65)),
        egui::StrokeKind::Outside,
    );

    // Search icon
    ui.painter().text(
        egui::pos2(inner.left() + 12.0, inner.center().y),
        egui::Align2::CENTER_CENTER,
        "\u{1F50D}",
        egui::FontId::proportional(10.0),
        egui::Color32::from_rgb(80, 85, 95),
    );

    // Text edit area
    let edit_rect = egui::Rect::from_min_max(
        egui::pos2(inner.left() + 22.0, inner.top()),
        egui::pos2(
            if gui.tree_filter.is_empty() {
                inner.right()
            } else {
                inner.right() - 18.0
            },
            inner.bottom(),
        ),
    );
    let mut child_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(edit_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    let resp = child_ui.add(
        egui::TextEdit::singleline(&mut gui.tree_filter)
            .hint_text("Filter...")
            .desired_width(edit_rect.width())
            .id(search_id)
            .margin(egui::Margin::ZERO)
            .frame(false),
    );

    // Clear button
    if !gui.tree_filter.is_empty() {
        let clear_rect = egui::Rect::from_center_size(
            egui::pos2(inner.right() - 10.0, inner.center().y),
            egui::vec2(14.0, 14.0),
        );
        let clear_resp = ui.interact(
            clear_rect,
            ui.id().with("clear_filter"),
            egui::Sense::click(),
        );
        let clear_color = if clear_resp.hovered() {
            egui::Color32::from_rgb(180, 185, 195)
        } else {
            egui::Color32::from_rgb(100, 105, 115)
        };
        ui.painter().text(
            clear_rect.center(),
            egui::Align2::CENTER_CENTER,
            "\u{00D7}",
            egui::FontId::proportional(12.0),
            clear_color,
        );
        if clear_resp.clicked() {
            gui.tree_filter.clear();
            resp.request_focus();
        }
    }
}

fn handle_keyboard_shortcuts(ui: &mut egui::Ui, gui: &mut GuiState) {
    let search_id = egui::Id::new("tree_search_box");
    if ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::F)) {
        ui.memory_mut(|m| m.request_focus(search_id));
    }
    if ui.input(|i| i.key_pressed(egui::Key::F2)) && gui.rename_edit.is_none() {
        if let Some(sel_id) = gui.actions.iter().find_map(|a| {
            if let GuiAction::SelectObject(id) = a {
                Some(*id)
            } else {
                None
            }
        }) {
            gui.rename_edit = Some((sel_id, String::new()));
        }
    }
    if ui.input(|i| i.key_pressed(egui::Key::Delete)) && gui.rename_edit.is_none() {
        gui.actions.push(GuiAction::DeleteSelected);
    }
}

// ---------------------------------------------------------------------------
// Recursive tree node rendering
// ---------------------------------------------------------------------------

const INDENT_PX: f32 = 16.0;
const ICON_SIZE: f32 = 14.0;
const GUIDE_COLOR: egui::Color32 = egui::Color32::from_rgb(55, 60, 70);
const ACTIVE_BODY_BG: egui::Color32 = egui::Color32::from_rgba_premultiplied(15, 45, 75, 35);
const TIP_COLOR: egui::Color32 = egui::Color32::from_rgb(60, 200, 100);
const ROW_HEIGHT: f32 = 22.0;

fn draw_tree_node(
    ui: &mut egui::Ui,
    gui: &mut GuiState,
    scene: &Scene,
    node: &TreeNode,
    depth: usize,
    is_last: bool,
    drag: &mut DragState,
) {
    if let Some(obj_id) = node.object_id {
        if let Some(obj) = scene.get(obj_id) {
            let has_children = !node.children.is_empty() || node.is_body;

            let expand_id = egui::Id::new(("tree_expand", obj_id));
            let default_expanded = node.is_body && node.is_active_body;
            let is_expanded = ui.data_mut(|d| *d.get_temp_mut_or(expand_id, default_expanded));

            draw_object_row(
                ui,
                gui,
                obj,
                node,
                depth,
                is_last,
                has_children,
                is_expanded,
                drag,
            );

            if is_expanded && !node.children.is_empty() {
                let child_count = node.children.len();
                for (ci, child) in node.children.iter().enumerate() {
                    let child_last = ci + 1 == child_count;
                    if child.object_id.is_some() {
                        draw_tree_node(ui, gui, scene, child, depth + 1, child_last, drag);
                    } else {
                        draw_history_row(ui, child, depth + 1, child_last, obj.selected);
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_object_row(
    ui: &mut egui::Ui,
    gui: &mut GuiState,
    obj: &SceneObject,
    node: &TreeNode,
    depth: usize,
    is_last: bool,
    has_children: bool,
    is_expanded: bool,
    drag: &mut DragState,
) {
    let id = obj.id;
    let is_selected = obj.selected;
    let expand_id = egui::Id::new(("tree_expand", id));

    let row_rect = ui.available_rect_before_wrap();
    let full_row =
        egui::Rect::from_min_size(row_rect.min, egui::vec2(row_rect.width(), ROW_HEIGHT));

    let row_resp = ui.allocate_rect(full_row, egui::Sense::click_and_drag());
    let hovered = row_resp.hovered();

    // Background: active body gets a subtle blue tint, selection is stronger
    if node.is_active_body && !is_selected {
        ui.painter().rect_filled(full_row, 0.0, ACTIVE_BODY_BG);
    }
    if is_selected {
        // Selection with left accent bar (FreeCAD style)
        ui.painter()
            .rect_filled(full_row, 0.0, egui::Color32::from_rgb(9, 71, 113));
        ui.painter().rect_filled(
            egui::Rect::from_min_size(full_row.left_top(), egui::vec2(2.0, ROW_HEIGHT)),
            0.0,
            egui::Color32::from_rgb(0, 122, 204),
        );
    } else if hovered {
        ui.painter()
            .rect_filled(full_row, 0.0, egui::Color32::from_rgb(42, 45, 48));
    }

    if row_resp.dragged() {
        drag.dragging_id = Some(id);
    }
    if drag.dragging_id.is_some() && drag.dragging_id != Some(id) && hovered {
        if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
            let mid_y = full_row.center().y;
            let target_y = if pointer.y < mid_y {
                full_row.top()
            } else {
                full_row.bottom()
            };
            drag.drop_target_y = Some(target_y);
            drag.drop_target_x = full_row.left() + (depth as f32 * INDENT_PX);
            drag.drop_target_width = full_row.width() - (depth as f32 * INDENT_PX);
        }
    }

    let painter = ui.painter();
    let left_x = full_row.left() + (depth as f32 * INDENT_PX);
    let center_y = full_row.center().y;

    draw_guide_lines(painter, full_row, depth, is_last, center_y);

    let mut x = left_x;

    if has_children {
        let arrow = if is_expanded { "\u{25BC}" } else { "\u{25B6}" };
        painter.text(
            egui::pos2(x + 6.0, center_y),
            egui::Align2::CENTER_CENTER,
            arrow,
            egui::FontId::proportional(9.0),
            theme::COLOR_DIM,
        );
        if row_resp.clicked()
            && row_resp
                .interact_pointer_pos()
                .is_some_and(|p| p.x < x + 14.0)
        {
            ui.data_mut(|d| {
                let val: &mut bool = d.get_temp_mut_or(expand_id, false);
                *val = !*val;
            });
            return;
        }
        x += 14.0;
    } else {
        x += 14.0;
    }

    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(x + ICON_SIZE * 0.5, center_y),
        egui::vec2(ICON_SIZE, ICON_SIZE),
    );
    let icon_color = if node.suppressed {
        egui::Color32::from_rgb(80, 80, 85)
    } else {
        theme::COLOR_ACCENT
    };
    draw_entity_icon(painter, icon_rect, node.icon, icon_color);

    if node.has_error || node.needs_recompute || node.suppressed {
        let overlay_pos = egui::pos2(icon_rect.right() - 2.0, icon_rect.top() + 2.0);
        draw_status_overlay(painter, overlay_pos, node);
    }
    x += ICON_SIZE + 3.0;

    let is_renaming = gui.rename_edit.as_ref().is_some_and(|(rid, _)| *rid == id);

    if is_renaming {
        let text_rect = egui::Rect::from_min_size(
            egui::pos2(x, full_row.top()),
            egui::vec2(full_row.right() - x - 40.0, ROW_HEIGHT),
        );
        let mut child_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(text_rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        let (_, text) = gui.rename_edit.as_mut().unwrap();
        let resp = child_ui.text_edit_singleline(text);
        if resp.lost_focus() || child_ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            let new_name = gui.rename_edit.take().unwrap().1;
            if !new_name.is_empty() {
                gui.actions.push(GuiAction::RenameObject(id, new_name));
            }
        }
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            gui.rename_edit = None;
        }
    } else {
        let label_color = if node.suppressed {
            egui::Color32::from_rgb(80, 80, 85)
        } else if is_selected {
            egui::Color32::WHITE
        } else if !obj.visible {
            theme::COLOR_DIM
        } else {
            ui.visuals().text_color()
        };

        let use_bold = node.is_active_body || is_selected;
        let font = if use_bold {
            egui::FontId::new(12.0, egui::FontFamily::Proportional)
        } else {
            egui::FontId::proportional(12.0)
        };

        if node.suppressed {
            let galley = painter.layout_no_wrap(obj.name.clone(), font.clone(), label_color);
            let name_width = galley.rect.width();
            painter.galley(
                egui::pos2(x, center_y - galley.rect.height() * 0.5),
                galley,
                label_color,
            );
            painter.line_segment(
                [
                    egui::pos2(x, center_y),
                    egui::pos2(x + name_width, center_y),
                ],
                egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 105)),
            );
        } else {
            painter.text(
                egui::pos2(x, center_y),
                egui::Align2::LEFT_CENTER,
                &obj.name,
                font,
                label_color,
            );
        }

        if node.is_tip && !node.suppressed {
            let tip_galley = painter.layout_no_wrap(
                obj.name.clone(),
                egui::FontId::proportional(12.0),
                label_color,
            );
            let name_width = tip_galley.rect.width();
            painter.text(
                egui::pos2(x + name_width + 4.0, center_y),
                egui::Align2::LEFT_CENTER,
                "\u{25B6}",
                egui::FontId::proportional(10.0),
                TIP_COLOR,
            );
        }

        // Color swatch (right side)
        let swatch_x = full_row.right() - 12.0;
        let [cr, cg, cb, _] = obj.color;
        let swatch_color =
            egui::Color32::from_rgb((cr * 255.0) as u8, (cg * 255.0) as u8, (cb * 255.0) as u8);
        let swatch_rect =
            egui::Rect::from_center_size(egui::pos2(swatch_x, center_y), egui::vec2(8.0, 8.0));
        painter.rect_filled(swatch_rect, 2.0, swatch_color);
        painter.rect_stroke(
            swatch_rect,
            2.0,
            egui::Stroke::new(0.5, egui::Color32::from_rgb(60, 62, 68)),
            egui::StrokeKind::Outside,
        );

        // Eye icon (visibility toggle) — only show on hover or hidden
        let eye_x = full_row.right() - 30.0;
        let show_eye = hovered || !obj.visible;
        if show_eye {
            let eye_char = if obj.visible { "\u{25C9}" } else { "\u{25CB}" };
            let eye_color = if obj.visible {
                egui::Color32::from_rgb(100, 108, 120)
            } else {
                egui::Color32::from_rgb(60, 60, 65)
            };
            painter.text(
                egui::pos2(eye_x, center_y),
                egui::Align2::CENTER_CENTER,
                eye_char,
                egui::FontId::proportional(11.0),
                eye_color,
            );
        }

        if row_resp.clicked() {
            let clicked_eye = row_resp
                .interact_pointer_pos()
                .is_some_and(|p| p.x >= eye_x - 9.0 && p.x <= eye_x + 9.0);
            if clicked_eye {
                gui.actions.push(GuiAction::ToggleVisibility(id));
            } else if ui.input(|i| i.modifiers.ctrl) {
                gui.actions.push(GuiAction::ToggleSelect(id));
            } else {
                gui.actions.push(GuiAction::SelectObject(id));
            }
        }
        if row_resp.double_clicked() {
            if node.is_body {
                let current = active_body_id(ui);
                if current == Some(id) {
                    set_active_body_id(ui, None);
                } else {
                    set_active_body_id(ui, Some(id));
                    ui.data_mut(|d| {
                        let val: &mut bool = d.get_temp_mut_or(expand_id, false);
                        *val = true;
                    });
                }
            } else {
                // Frame the camera on this object (CAD-conventional double-click
                // action). Rename remains accessible via F2 and the context menu.
                gui.actions.push(GuiAction::SelectObject(id));
                gui.actions.push(GuiAction::FocusObject(id));
            }
        }

        row_resp.context_menu(|ui| {
            super::context_menu::tree_context_menu(ui, gui, id, &obj.name, obj.visible);
        });
    }
}

/// Draw a history child row (non-interactive feature entry).
fn draw_history_row(
    ui: &mut egui::Ui,
    node: &TreeNode,
    depth: usize,
    is_last: bool,
    parent_selected: bool,
) {
    let row_rect = ui.available_rect_before_wrap();
    let hist_row_h = 18.0;
    let full_row =
        egui::Rect::from_min_size(row_rect.min, egui::vec2(row_rect.width(), hist_row_h));

    let _resp = ui.allocate_rect(full_row, egui::Sense::hover());
    let painter = ui.painter();
    let left_x = full_row.left() + (depth as f32 * INDENT_PX);
    let center_y = full_row.center().y;

    draw_guide_lines(painter, full_row, depth, is_last, center_y);

    let mut x = left_x;

    let child_icon_size = 12.0;
    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(x + child_icon_size * 0.5, center_y),
        egui::vec2(child_icon_size, child_icon_size),
    );
    let icon_color = if parent_selected {
        theme::COLOR_ACCENT
    } else {
        theme::COLOR_DIM
    };
    draw_entity_icon(painter, icon_rect, node.icon, icon_color);
    x += child_icon_size + 3.0;

    let text_color = if parent_selected {
        ui.visuals().text_color()
    } else {
        theme::COLOR_DIM
    };

    let font = egui::FontId::proportional(11.0);
    painter.text(
        egui::pos2(x, center_y),
        egui::Align2::LEFT_CENTER,
        &node.name,
        font,
        text_color,
    );

    if node.is_tip {
        let name_width = painter
            .layout_no_wrap(
                node.name.clone(),
                egui::FontId::proportional(11.0),
                text_color,
            )
            .rect
            .width();
        painter.text(
            egui::pos2(x + name_width + 3.0, center_y),
            egui::Align2::LEFT_CENTER,
            "\u{25B8}",
            egui::FontId::proportional(9.0),
            TIP_COLOR,
        );
    }
}

// ---------------------------------------------------------------------------
// Tree guide line drawing
// ---------------------------------------------------------------------------

fn draw_guide_lines(
    painter: &egui::Painter,
    full_row: egui::Rect,
    depth: usize,
    is_last: bool,
    center_y: f32,
) {
    if depth == 0 {
        return;
    }
    let guide_x = full_row.left() + ((depth - 1) as f32 * INDENT_PX) + INDENT_PX * 0.5;
    let guide_stroke = egui::Stroke::new(1.0, GUIDE_COLOR);

    if is_last {
        painter.line_segment(
            [
                egui::pos2(guide_x, full_row.top()),
                egui::pos2(guide_x, center_y),
            ],
            guide_stroke,
        );
    } else {
        painter.line_segment(
            [
                egui::pos2(guide_x, full_row.top()),
                egui::pos2(guide_x, full_row.bottom()),
            ],
            guide_stroke,
        );
    }
    painter.line_segment(
        [
            egui::pos2(guide_x, center_y),
            egui::pos2(guide_x + INDENT_PX * 0.4, center_y),
        ],
        guide_stroke,
    );
}

// ---------------------------------------------------------------------------
// Context menu helpers
// ---------------------------------------------------------------------------

fn params_label(p: &CreationParams) -> &'static str {
    match p {
        CreationParams::Box { .. } => "Box",
        CreationParams::Cylinder { .. } => "Cylinder",
        CreationParams::Sphere { .. } => "Sphere",
        CreationParams::Cone { .. } => "Cone",
        CreationParams::Torus { .. } => "Torus",
        CreationParams::Tube { .. } => "Tube",
        CreationParams::Prism { .. } => "Prism",
        CreationParams::Wedge { .. } => "Wedge",
        CreationParams::Ellipsoid { .. } => "Ellipsoid",
        CreationParams::Helix { .. } => "Helix",
        CreationParams::Imported { .. } => "Imported",
        CreationParams::Extruded => "Extruded",
        CreationParams::Revolved => "Revolved",
        CreationParams::Boolean { .. } => "Boolean",
        CreationParams::Fillet { .. } => "Fillet",
        CreationParams::Chamfer { .. } => "Chamfer",
        CreationParams::Shell { .. } => "Shell",
        CreationParams::Mirror { .. } => "Mirror",
        CreationParams::Pattern { .. } => "Pattern",
        CreationParams::Groove { .. } => "Groove",
        CreationParams::Sprocket { .. } => "Sprocket",
        CreationParams::InvoluteGear { .. } => "Gear",
        CreationParams::DraftLine { .. } => "Line",
        CreationParams::DraftCircle { .. } => "Circle",
        CreationParams::DraftRectangle { .. } => "Rectangle",
        CreationParams::DraftPolygon { .. } => "Polygon",
        CreationParams::DraftArc { .. } => "Arc",
        CreationParams::DraftEllipse { .. } => "Ellipse",
        CreationParams::SurfacePipe { .. } => "Pipe",
        CreationParams::SurfaceRuled { .. } => "Ruled",
        CreationParams::BooleanOp { .. } => "BooleanOp",
        CreationParams::ScaleOp { .. } => "Scale",
    }
}

#[allow(dead_code)]
pub(crate) fn params_label_pub(p: &CreationParams) -> &'static str {
    params_label(p)
}

// ---------------------------------------------------------------------------
// Assembly tree section
// ---------------------------------------------------------------------------

/// Render the Assembly section of the tree (Components / Constraints / Joints).
/// No-op when there is no active assembly.
pub(crate) fn draw_assembly_section(ui: &mut egui::Ui, gui: &mut GuiState, scene: Option<&Scene>) {
    let Some(asm) = gui.assembly.as_ref() else {
        return;
    };
    let title = format!(
        "\u{1F527} Assembly \u{2014} \"{}\"  ({} components, {} constraints, {} joints)",
        asm.name,
        asm.num_components(),
        asm.num_constraints(),
        asm.joint_count(),
    );
    // Snapshot component rows with a scene-resolved ObjectId (if the solid
    // handle matches a SceneObject). The lookup is done eagerly so that the
    // immutable borrow of `asm` doesn't clash with `gui.actions.push` below.
    let comps: Vec<(usize, String, bool, Option<ObjectId>)> = asm
        .components
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let oid = scene.and_then(|s| find_object_for_solid(s, c.solid));
            (i, c.name.clone(), c.visible, oid)
        })
        .collect();
    let constraint_rows: Vec<String> = asm.constraints.iter().map(constraint_label).collect();
    let joint_rows: Vec<String> = asm.joints.iter().map(joint_label).collect();

    egui::CollapsingHeader::new(
        egui::RichText::new(title)
            .size(11.5)
            .color(theme::COLOR_ACCENT),
    )
    .id_salt("tree_assembly_root")
    .default_open(true)
    .show(ui, |ui| {
        egui::CollapsingHeader::new(format!("Components ({})", comps.len()))
            .id_salt("tree_assembly_comps")
            .default_open(true)
            .show(ui, |ui| {
                for (i, name, visible, oid) in &comps {
                    ui.horizontal(|ui| {
                        let eye = if *visible { "\u{25C9}" } else { "\u{25CB}" };
                        if ui
                            .small_button(eye)
                            .on_hover_text("Toggle visibility")
                            .clicked()
                        {
                            gui.actions.push(GuiAction::Assembly(
                                super::AssemblyAction::ToggleComponentVisibility(*i),
                            ));
                        }
                        let resp = ui
                            .label(format!("\u{1F4E6} {name}"))
                            .on_hover_text("Double-click to focus");
                        if resp.double_clicked() {
                            if let Some(id) = oid {
                                gui.actions.push(GuiAction::FocusObject(*id));
                            }
                        }
                    });
                }
            });
        egui::CollapsingHeader::new(format!("Constraints ({})", constraint_rows.len()))
            .id_salt("tree_assembly_cons")
            .default_open(false)
            .show(ui, |ui| {
                for (i, label) in constraint_rows.iter().enumerate() {
                    ui.label(
                        egui::RichText::new(format!("  {i}. {label}"))
                            .size(10.5)
                            .color(theme::COLOR_DIM),
                    );
                }
            });
        egui::CollapsingHeader::new(format!("Joints ({})", joint_rows.len()))
            .id_salt("tree_assembly_joints")
            .default_open(false)
            .show(ui, |ui| {
                for (i, label) in joint_rows.iter().enumerate() {
                    ui.label(
                        egui::RichText::new(format!("  {i}. {label}"))
                            .size(10.5)
                            .color(theme::COLOR_DIM),
                    );
                }
            });
    });
    theme::draw_separator(ui);
}

/// Pure helper: format an [`AssemblyConstraint`] as a one-line label for the
/// tree panel. Matches the spec format: `Fixed(comp 0)`, `Coincident(a,b)`, etc.
pub(crate) fn constraint_label(c: &cadkernel_modeling::AssemblyConstraint) -> String {
    use cadkernel_modeling::AssemblyConstraint as C;
    match c {
        C::Fixed(id) => format!("Fixed(comp {})", id.0),
        C::Coincident { comp_a, comp_b, .. } => {
            format!("Coincident({},{})", comp_a.0, comp_b.0)
        }
        C::Concentric { comp_a, comp_b } => {
            format!("Concentric({},{})", comp_a.0, comp_b.0)
        }
        C::Distance { comp_a, comp_b, .. } => {
            format!("Distance({},{})", comp_a.0, comp_b.0)
        }
        C::Angle { comp_a, comp_b, .. } => {
            format!("Angle({},{})", comp_a.0, comp_b.0)
        }
    }
}

/// Pure helper: format a [`JointType`] as a tree-row label. Uses the
/// `Revolute(a↔b)` / `Grounded` / `FixedJoint(a↔b)` patterns.
pub(crate) fn joint_label(j: &cadkernel_modeling::JointType) -> String {
    use cadkernel_modeling::JointType as J;
    match j {
        J::Grounded => "Grounded".to_string(),
        J::FixedJoint {
            component_a,
            component_b,
        } => {
            format!("FixedJoint({component_a}\u{2194}{component_b})")
        }
        J::Revolute {
            component_a,
            component_b,
            ..
        } => {
            format!("Revolute({component_a}\u{2194}{component_b})")
        }
        J::Cylindrical {
            component_a,
            component_b,
            ..
        } => {
            format!("Cylindrical({component_a}\u{2194}{component_b})")
        }
        J::Slider {
            component_a,
            component_b,
            ..
        } => {
            format!("Slider({component_a}\u{2194}{component_b})")
        }
        J::BallJoint {
            component_a,
            component_b,
            ..
        } => {
            format!("Ball({component_a}\u{2194}{component_b})")
        }
        J::ParallelAxes {
            component_a,
            component_b,
            ..
        } => {
            format!("Parallel({component_a}\u{2194}{component_b})")
        }
        J::PerpendicularAxes {
            component_a,
            component_b,
            ..
        } => {
            format!("Perpendicular({component_a}\u{2194}{component_b})")
        }
        J::AngleJoint {
            component_a,
            component_b,
            ..
        } => {
            format!("Angle({component_a}\u{2194}{component_b})")
        }
        J::GearJoint {
            component_a,
            component_b,
            ..
        } => {
            format!("Gear({component_a}\u{2194}{component_b})")
        }
        J::RackAndPinion {
            component_a,
            component_b,
            ..
        } => {
            format!("Rack({component_a}\u{2194}{component_b})")
        }
        J::ScrewJoint {
            component_a,
            component_b,
            ..
        } => {
            format!("Screw({component_a}\u{2194}{component_b})")
        }
        J::BeltJoint {
            component_a,
            component_b,
            ..
        } => {
            format!("Belt({component_a}\u{2194}{component_b})")
        }
    }
}

/// Lookup the first [`SceneObject`] whose solid handle matches `solid`.
/// Returns `None` if the assembly component is not mirrored in the scene.
fn find_object_for_solid(
    scene: &Scene,
    solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
) -> Option<ObjectId> {
    scene
        .visible_objects()
        .find(|o| o.solid == solid)
        .map(|o| o.id)
}
