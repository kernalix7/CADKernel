use super::theme;
use super::{GuiAction, GuiState, SketchTool, ViewportInfo, Workbench};
use crate::render::Projection;
use crate::scene::{CreationParams, Scene};

/// Vertical divider (thin line) between status bar sections.
fn vert_divider(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(11.0, 14.0), egui::Sense::hover());
    ui.painter().line_segment(
        [
            egui::pos2(rect.center().x, rect.top() + 2.0),
            egui::pos2(rect.center().x, rect.bottom() - 2.0),
        ],
        egui::Stroke::new(0.5, egui::Color32::from_rgb(0x32, 0x38, 0x44)),
    );
}

/// Status-bar pill badge. Renders a rounded rect with a thin tinted background
/// behind the label; on hover the background brightens. Returns the egui
/// `Response` so call sites can layer click semantics + tooltips on top.
///
/// `accent` controls the tint: the badge fill is `accent.gamma_multiply(0.22)`,
/// the stroke is `accent.gamma_multiply(0.55)`, and the text is rendered in
/// `accent` itself. For purely decorative badges pass a dimmer color.
fn badge(
    ui: &mut egui::Ui,
    text: &str,
    accent: egui::Color32,
    interactive: bool,
) -> egui::Response {
    let font = egui::FontId::new(10.5, egui::FontFamily::Proportional);
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font, accent);
    let pad_x = 7.0_f32;
    let pad_y = 1.5_f32;
    let size = egui::vec2(galley.size().x + pad_x * 2.0, 16.0);
    let sense = if interactive {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(size, sense);

    let painter = ui.painter();
    let hovered = resp.hovered();
    let bg_alpha: f32 = if hovered { 0.32 } else { 0.18 };
    let stroke_alpha: f32 = if hovered { 0.85 } else { 0.55 };
    let bg = accent.gamma_multiply(bg_alpha);
    let stroke = egui::Stroke::new(0.8, accent.gamma_multiply(stroke_alpha));
    painter.rect_filled(rect, egui::CornerRadius::same(7), bg);
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(7),
        stroke,
        egui::StrokeKind::Inside,
    );
    let text_pos = egui::pos2(
        rect.left() + pad_x,
        rect.center().y - galley.size().y * 0.5 + pad_y * 0.5 - 0.5,
    );
    painter.galley(text_pos, galley, accent);
    resp
}

pub(crate) fn draw_status_bar(
    ctx: &egui::Context,
    gui: &mut GuiState,
    vp: &ViewportInfo<'_>,
    scene: &Scene,
) {
    egui::TopBottomPanel::bottom("status_bar")
        .frame(egui::Frame {
            fill: egui::Color32::from_rgb(0x14, 0x17, 0x1D),
            inner_margin: egui::Margin::symmetric(10, 3),
            stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(0x0B, 0x0D, 0x12)),
            ..egui::Frame::NONE
        })
        .show(ctx, |ui| {
            // Top edge accent: subtle teal sliver (signature CADKernel color)
            let top_rect = ui.available_rect_before_wrap();
            ui.painter().line_segment(
                [
                    egui::pos2(top_rect.left(), top_rect.top() - 3.0),
                    egui::pos2(top_rect.right(), top_rect.top() - 3.0),
                ],
                egui::Stroke::new(1.0, theme::COLOR_ACCENT.gamma_multiply(0.55)),
            );

            ui.horizontal(|ui| {
                // -- Left section: Mouse coordinates with unit --
                if let Some(pos) = gui.mouse_world_pos {
                    ui.label(
                        egui::RichText::new(format!(
                            "X:{:.2}  Y:{:.2}  Z:{:.2} mm",
                            pos[0], pos[1], pos[2]
                        ))
                        .color(egui::Color32::from_rgb(130, 140, 155))
                        .size(10.5)
                        .monospace(),
                    );
                    vert_divider(ui);
                }

                // -- Center section: Active tool / sketch status --
                if let Some(sketch) = &gui.sketch_mode {
                    let tool_name = sketch_tool_label(sketch.tool);
                    let hint = sketch_tool_hint(sketch.tool);
                    ui.label(
                        egui::RichText::new(format!("{tool_name} -- {hint}"))
                            .size(11.0)
                            .color(theme::COLOR_INFO),
                    );
                    vert_divider(ui);

                    // DOF status (per-constraint type weighting)
                    let dof = sketch.degrees_of_freedom();
                    let n_pts = sketch.sketch.points.len();
                    let (status_text, status_color) = if n_pts == 0 {
                        ("Empty sketch", theme::COLOR_DIM)
                    } else if dof <= 0 {
                        ("Fully constrained", egui::Color32::from_rgb(100, 210, 120))
                    } else {
                        ("Under-constrained", egui::Color32::from_rgb(220, 180, 50))
                    };
                    ui.label(
                        egui::RichText::new(if dof > 0 {
                            format!("{status_text} ({dof} DOF)")
                        } else {
                            status_text.to_string()
                        })
                        .size(11.0)
                        .color(status_color),
                    );
                    if sketch.constraint_warning_count > 0 {
                        vert_divider(ui);
                        ui.label(
                            egui::RichText::new(sketch.constraint_status.clone())
                                .size(11.0)
                                .color(egui::Color32::from_rgb(255, 180, 70)),
                        );
                    }
                    if sketch.external_reference_count > 0 || sketch.reused_geometry_count > 0 {
                        vert_divider(ui);
                        ui.label(
                            egui::RichText::new(sketch.reference_status.clone())
                                .size(11.0)
                                .color(egui::Color32::from_rgb(150, 200, 255)),
                        );
                    }
                    // Show selection count in sketch mode
                    if !sketch.selected_entities.is_empty() {
                        vert_divider(ui);
                        ui.label(
                            egui::RichText::new(format!("Sel: {}", sketch.selected_entities.len()))
                                .size(11.0)
                                .color(egui::Color32::from_rgb(80, 160, 255)),
                        );
                    }

                    // Hovered entity info
                    if let Some(info) = sketch_hover_info(sketch) {
                        vert_divider(ui);
                        ui.label(
                            egui::RichText::new(info)
                                .size(11.0)
                                .color(egui::Color32::from_rgb(180, 200, 230)),
                        );
                    }
                    vert_divider(ui);

                    // Snap indicators
                    let snap_color = if sketch.snap_enabled {
                        egui::Color32::from_rgb(80, 180, 80)
                    } else {
                        theme::COLOR_DIM
                    };
                    ui.label(egui::RichText::new("Snap").size(10.0).color(snap_color));
                    let grid_color = if sketch.show_grid {
                        egui::Color32::from_rgb(80, 180, 80)
                    } else {
                        theme::COLOR_DIM
                    };
                    ui.label(egui::RichText::new("Grid").size(10.0).color(grid_color));
                    vert_divider(ui);
                } else {
                    // Workbench indicator badge (teal accent — matches activity rail)
                    let wb_label = workbench_short_label(gui.active_workbench);
                    badge(ui, wb_label, theme::COLOR_ACCENT, false);
                    vert_divider(ui);

                    // Selection mode indicator — "Auto" since auto-pick is always active
                    badge(ui, "Auto", theme::COLOR_DIM, false);
                    vert_divider(ui);

                    // Hover preview: show what entity is under cursor
                    let hover_text = build_hover_preview(gui);
                    if !hover_text.is_empty() {
                        ui.label(
                            egui::RichText::new(hover_text)
                                .color(theme::COLOR_PRESELECT)
                                .size(11.0),
                        );
                        vert_divider(ui);
                    }

                    // Preselection info / status message
                    let presel_text = build_preselection_text(gui, scene);
                    if !presel_text.is_empty() {
                        ui.label(
                            egui::RichText::new(presel_text)
                                .color(theme::COLOR_PRESELECT)
                                .size(11.0),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new(&gui.status_message)
                                .color(theme::COLOR_INFO)
                                .size(11.0),
                        );
                    }

                    // -- Quick Measure: dimensions of selected object(s) --
                    let measure_text = build_quick_measure_text(scene, gui);
                    if !measure_text.is_empty() {
                        vert_divider(ui);
                        ui.label(
                            egui::RichText::new(measure_text)
                                .size(11.0)
                                .color(egui::Color32::from_rgb(180, 220, 140)),
                        );
                    }
                }

                // -- Right section --
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Unit system indicator (far right, dimmed)
                    badge(ui, "mm", egui::Color32::from_rgb(110, 118, 130), false);

                    // Shortcuts hint (press F1 to open reference)
                    let hint_resp = badge(ui, "F1", egui::Color32::from_rgb(120, 130, 145), true);
                    if hint_resp.hovered() {
                        hint_resp.on_hover_text("Press F1 to open the Keyboard Shortcuts panel");
                    }

                    // Navigation mode indicator (clickable)
                    let nav_resp = badge(ui, "CAD", egui::Color32::from_rgb(120, 145, 180), true);
                    nav_resp.on_hover_text("Navigation style");

                    vert_divider(ui);

                    // Projection toggle (clickable)
                    let (proj_label, proj_color) = match vp.camera.projection {
                        Projection::Perspective => {
                            ("Persp", egui::Color32::from_rgb(110, 175, 235))
                        }
                        Projection::Orthographic => {
                            ("Ortho", egui::Color32::from_rgb(150, 215, 130))
                        }
                    };
                    let proj_resp = badge(ui, proj_label, proj_color, true);
                    if proj_resp.clicked() {
                        gui.actions.push(GuiAction::ToggleProjection);
                    }
                    proj_resp.on_hover_text("Toggle Perspective / Orthographic (5)");

                    vert_divider(ui);

                    // Display mode badge
                    badge(
                        ui,
                        vp.display_mode.label(),
                        egui::Color32::from_rgb(135, 145, 165),
                        false,
                    );

                    vert_divider(ui);

                    // Scene stats: objects + triangles
                    let n_obj = scene.len();
                    let n_vis = scene.visible_objects().count();
                    let total_tri: usize = scene
                        .visible_objects()
                        .map(|o| o.mesh.triangle_count())
                        .sum();
                    let tri_text = if total_tri >= 1_000_000 {
                        format!("{:.1}M", total_tri as f64 / 1_000_000.0)
                    } else if total_tri >= 1_000 {
                        format!("{:.1}K", total_tri as f64 / 1_000.0)
                    } else {
                        format!("{total_tri}")
                    };
                    badge(
                        ui,
                        &format!("{n_vis}/{n_obj} obj  \u{25B3} {tri_text}"),
                        theme::COLOR_DIM,
                        false,
                    );

                    // Selection info
                    let sel_count = scene.selected_objects().len();
                    if sel_count > 0 {
                        vert_divider(ui);
                        let sel_text = if sel_count > 1 {
                            format!("{sel_count} sel")
                        } else if let Some(obj) = scene.selected_object() {
                            format!("Sel: {}", obj.name)
                        } else {
                            String::new()
                        };
                        if !sel_text.is_empty() {
                            badge(ui, &sel_text, egui::Color32::from_rgb(100, 175, 255), false);
                        }
                    }

                    // Measure mode
                    if gui.measurement_mode {
                        vert_divider(ui);
                        badge(ui, "Measure", egui::Color32::from_rgb(230, 190, 80), false);
                    }

                    // FPS (far left of right section, so it renders last = leftmost)
                    if vp.show_fps {
                        vert_divider(ui);
                        badge(ui, &format!("{:.0} FPS", vp.fps), theme::COLOR_DIM, false);
                    }
                });
            });
        });
}

// ---------------------------------------------------------------------------
// Hover preview builder (shows what's under cursor)
// ---------------------------------------------------------------------------

fn build_hover_preview(gui: &GuiState) -> String {
    if let Some(entity) = &gui.preselected_entity {
        return match entity {
            super::SelectedEntity::Vertex(h) => format!("Vertex {}", h.index()),
            super::SelectedEntity::Edge(h) => format!("Edge {}", h.index()),
            super::SelectedEntity::Face(h) => format!("Face {}", h.index()),
            super::SelectedEntity::Solid(_) | super::SelectedEntity::Shell(_) => "Solid".into(),
        };
    }
    if gui.preselected_object_id.is_some() {
        return "Solid".into();
    }
    String::new()
}

// ---------------------------------------------------------------------------
// Preselection info builder
// ---------------------------------------------------------------------------

/// Build a description of what is currently selected.
fn build_preselection_text(gui: &GuiState, scene: &Scene) -> String {
    // When a sub-element entity is active and the selection mode targets
    // a sub-element type, show the entity kind relative to the selected object.
    if !gui.selected_entities.is_empty() {
        if let Some(obj) = scene.selected_object() {
            let n = gui.selected_entities.len();
            // Detect entity type from actual selection (auto-pick)
            let has_face = gui
                .selected_entities
                .iter()
                .any(|e| matches!(e, super::SelectedEntity::Face(_)));
            let has_edge = gui
                .selected_entities
                .iter()
                .any(|e| matches!(e, super::SelectedEntity::Edge(_)));
            let has_vertex = gui
                .selected_entities
                .iter()
                .any(|e| matches!(e, super::SelectedEntity::Vertex(_)));
            if has_vertex {
                return if n > 1 {
                    format!("{n} Vertices of {}", obj.name)
                } else {
                    format!("Vertex of {}", obj.name)
                };
            } else if has_edge {
                return if n > 1 {
                    format!("{n} Edges of {}", obj.name)
                } else {
                    format!("Edge of {}", obj.name)
                };
            } else if has_face {
                return if n > 1 {
                    format!("{n} Faces of {}", obj.name)
                } else {
                    format!("Face of {}", obj.name)
                };
            }
        }
    }
    String::new()
}

// ---------------------------------------------------------------------------
// Quick Measure builder
// ---------------------------------------------------------------------------

/// Build a quick-measure summary string from the scene selection.
///
/// - Single object: creation-param dimensions, bounding-box size, mesh stats.
/// - Two objects: distance between bounding-box centres.
/// - Measurement mode: point-to-point distance from placed points.
fn build_quick_measure_text(scene: &Scene, gui: &GuiState) -> String {
    // Measurement points take priority when in measure mode
    if gui.measurement_mode && gui.measurement_points.len() >= 2 {
        let a = gui.measurement_points[0];
        let b = gui.measurement_points[1];
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let dz = b[2] - a[2];
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();
        return format!("Distance: {dist:.2} mm");
    }

    let selected = scene.selected_objects();
    if selected.is_empty() {
        return String::new();
    }

    if selected.len() == 1 {
        let obj = selected[0];
        let mut parts: Vec<String> = Vec::new();

        // Show creation-param dimensions if available
        if let Some(params) = &obj.params {
            if let Some(dim_str) = creation_params_summary(params) {
                parts.push(dim_str);
            }
        }

        // Mesh bounding-box dimensions
        if let Some((min, max)) = mesh_bbox(&obj.mesh) {
            let sx = max[0] - min[0];
            let sy = max[1] - min[1];
            let sz = max[2] - min[2];
            parts.push(format!("BBox: {sx:.1} x {sy:.1} x {sz:.1} mm"));
        }

        // Mesh stats
        let tri = obj.mesh.triangle_count();
        let verts = obj.mesh.vertices.len();
        parts.push(format!("{verts} verts, {tri} tris"));

        return parts.join("  |  ");
    }

    if selected.len() == 2 {
        // Distance between centres of two objects
        let c0 = mesh_center(&selected[0].mesh);
        let c1 = mesh_center(&selected[1].mesh);
        if let (Some(a), Some(b)) = (c0, c1) {
            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            let dz = b[2] - a[2];
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            return format!(
                "Distance ({} - {}): {dist:.2} mm",
                selected[0].name, selected[1].name
            );
        }
    }

    format!("{} objects selected", selected.len())
}

/// Return a short summary of the creation parameters.
fn creation_params_summary(params: &CreationParams) -> Option<String> {
    match params {
        CreationParams::Box {
            width,
            height,
            depth,
        } => Some(format!("W:{width:.1} H:{height:.1} D:{depth:.1}")),
        CreationParams::Cylinder { radius, height } => {
            Some(format!("R:{radius:.1} H:{height:.1} D:{:.1}", radius * 2.0))
        }
        CreationParams::Sphere { radius } => Some(format!("R:{radius:.1} D:{:.1}", radius * 2.0)),
        CreationParams::Cone {
            base_radius,
            top_radius,
            height,
        } => Some(format!(
            "Rb:{base_radius:.1} Rt:{top_radius:.1} H:{height:.1}"
        )),
        CreationParams::Torus {
            major_radius,
            minor_radius,
        } => Some(format!(
            "Major R:{major_radius:.1} Minor R:{minor_radius:.1}"
        )),
        CreationParams::Tube {
            outer_radius,
            inner_radius,
            height,
        } => Some(format!(
            "Ro:{outer_radius:.1} Ri:{inner_radius:.1} H:{height:.1}"
        )),
        CreationParams::Prism {
            radius,
            height,
            sides,
        } => Some(format!("R:{radius:.1} H:{height:.1} Sides:{sides}")),
        CreationParams::Ellipsoid { rx, ry, rz } => {
            Some(format!("Rx:{rx:.1} Ry:{ry:.1} Rz:{rz:.1}"))
        }
        CreationParams::Helix {
            radius,
            pitch,
            turns,
            tube_radius,
        } => Some(format!(
            "R:{radius:.1} P:{pitch:.1} T:{turns:.1} Tr:{tube_radius:.1}"
        )),
        CreationParams::Fillet { radius } => Some(format!("Fillet R:{radius:.1}")),
        CreationParams::Chamfer { distance } => Some(format!("Chamfer D:{distance:.1}")),
        CreationParams::Shell { thickness } => Some(format!("Shell T:{thickness:.1}")),
        CreationParams::InvoluteGear {
            teeth,
            module_val,
            pressure_angle,
        } => Some(format!(
            "Z:{teeth} M:{module_val:.1} PA:{pressure_angle:.0}"
        )),
        CreationParams::Sprocket { teeth, pitch, .. } => {
            Some(format!("Z:{teeth} Pitch:{pitch:.1}"))
        }
        _ => None,
    }
}

/// Compute the axis-aligned bounding box of a mesh as `(min, max)` in `[f64; 3]`.
fn mesh_bbox(mesh: &cadkernel_io::Mesh) -> Option<([f64; 3], [f64; 3])> {
    let first = mesh.vertices.first()?;
    let mut min = [first.x, first.y, first.z];
    let mut max = min;
    for v in &mesh.vertices {
        let p = [v.x, v.y, v.z];
        for i in 0..3 {
            if p[i] < min[i] {
                min[i] = p[i];
            }
            if p[i] > max[i] {
                max[i] = p[i];
            }
        }
    }
    Some((min, max))
}

/// Compute the centroid (average vertex position) of a mesh.
fn mesh_center(mesh: &cadkernel_io::Mesh) -> Option<[f64; 3]> {
    if mesh.vertices.is_empty() {
        return None;
    }
    let n = mesh.vertices.len() as f64;
    let mut sum = [0.0_f64; 3];
    for v in &mesh.vertices {
        sum[0] += v.x;
        sum[1] += v.y;
        sum[2] += v.z;
    }
    Some([sum[0] / n, sum[1] / n, sum[2] / n])
}

// ---------------------------------------------------------------------------
// Sketch tool helpers
// ---------------------------------------------------------------------------

fn sketch_tool_label(tool: SketchTool) -> &'static str {
    match tool {
        SketchTool::Select => "Select",
        SketchTool::Line => "Line",
        SketchTool::Rectangle => "Rectangle",
        SketchTool::Circle => "Circle",
        SketchTool::Arc => "Arc",
        SketchTool::Point => "Point",
        SketchTool::Ellipse => "Ellipse",
        SketchTool::Polyline => "Polyline",
        SketchTool::Slot => "Slot",
        SketchTool::BSpline => "B-Spline",
        SketchTool::Polygon { .. } => "Polygon",
    }
}

fn sketch_tool_hint(tool: SketchTool) -> &'static str {
    match tool {
        SketchTool::Select => "Click to select, drag to move",
        SketchTool::Line => "Click start point, click end point",
        SketchTool::Rectangle => "Click corner, click opposite corner",
        SketchTool::Circle => "Click center, click radius",
        SketchTool::Arc => "Click start, middle, end",
        SketchTool::Point => "Click to place point",
        SketchTool::Ellipse => "Click center, click major axis, click minor",
        SketchTool::Polyline => "Click points, right-click to finish",
        SketchTool::Slot => "Click center 1, center 2, then width",
        SketchTool::BSpline => "Click control points, right-click to finish",
        SketchTool::Polygon { .. } => "Click center, click vertex",
    }
}

/// Build a short info string for the currently hovered sketch entity.
fn sketch_hover_info(sm: &super::SketchMode) -> Option<String> {
    use super::SketchEntityRef;
    let entity = sm.hovered_entity.as_ref()?;
    let sk = &sm.sketch;
    match *entity {
        SketchEntityRef::Point(idx) => {
            let pt = sk.points.get(idx)?;
            Some(format!("Point({:.2}, {:.2})", pt.position.x, pt.position.y))
        }
        SketchEntityRef::Line(idx) => {
            let ln = sk.lines.get(idx)?;
            let p0 = sk.points.get(ln.start.0)?;
            let p1 = sk.points.get(ln.end.0)?;
            let dx = p1.position.x - p0.position.x;
            let dy = p1.position.y - p0.position.y;
            let len = (dx * dx + dy * dy).sqrt();
            let angle = dy.atan2(dx).to_degrees();
            Some(format!("Line  L:{len:.2}  A:{angle:.1}°"))
        }
        SketchEntityRef::Circle(idx) => {
            let c = sk.circles.get(idx)?;
            let cp = sk.points.get(c.center.0)?;
            Some(format!(
                "Circle  C({:.2},{:.2})  R:{:.2}",
                cp.position.x, cp.position.y, c.radius
            ))
        }
        SketchEntityRef::Arc(idx) => {
            let a = sk.arcs.get(idx)?;
            let cp = sk.points.get(a.center.0)?;
            let span = (a.end_angle - a.start_angle).to_degrees().abs();
            Some(format!(
                "Arc  C({:.2},{:.2})  R:{:.2}  Span:{:.1}°",
                cp.position.x, cp.position.y, a.radius, span
            ))
        }
        SketchEntityRef::Ellipse(idx) => {
            let e = sk.ellipses.get(idx)?;
            let cp = sk.points.get(e.center.0)?;
            Some(format!(
                "Ellipse  C({:.2},{:.2})  Rmin:{:.2}",
                cp.position.x, cp.position.y, e.minor_radius
            ))
        }
        SketchEntityRef::BSpline(idx) => {
            let b = sk.bsplines.get(idx)?;
            Some(format!(
                "B-Spline  deg:{}  pts:{}",
                b.degree,
                b.control_points.len()
            ))
        }
    }
}

fn workbench_short_label(wb: Workbench) -> &'static str {
    match wb {
        Workbench::Part => "Part",
        Workbench::PartDesign => "PartDesign",
        Workbench::Sketcher => "Sketcher",
        Workbench::Mesh => "Mesh",
        Workbench::TechDraw => "TechDraw",
        Workbench::Assembly => "Assembly",
        Workbench::Draft => "Draft",
        Workbench::Surface => "Surface",
        Workbench::Fem => "FEM",
    }
}
