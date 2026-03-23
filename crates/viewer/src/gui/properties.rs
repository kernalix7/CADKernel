use super::{GuiAction, GuiState};
use crate::scene::{CreationParams, Scene};

/// Property panel tab state.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PropertyTab {
    Data,
    View,
}

pub(crate) fn draw_properties(
    ctx: &egui::Context,
    gui: &mut GuiState,
    scene: &Scene,
) {
    if !gui.show_properties {
        return;
    }
    egui::SidePanel::right("properties")
        .default_width(280.0)
        .show(ctx, |ui| {
            // Tab bar
            ui.horizontal(|ui| {
                let data_selected = gui.property_tab == PropertyTab::Data;
                if ui.selectable_label(data_selected, "\u{1F4CA} Data").clicked() {
                    gui.property_tab = PropertyTab::Data;
                }
                let view_selected = gui.property_tab == PropertyTab::View;
                if ui.selectable_label(view_selected, "\u{1F3A8} View").clicked() {
                    gui.property_tab = PropertyTab::View;
                }
            });
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                if let Some(obj) = scene.selected_object() {
                    match gui.property_tab {
                        PropertyTab::Data => draw_data_tab(ui, gui, obj),
                        PropertyTab::View => draw_view_tab(ui, gui, obj),
                    }
                } else {
                    draw_scene_overview(ui, scene);
                }
            });
        });
}

fn draw_data_tab(
    ui: &mut egui::Ui,
    gui: &mut GuiState,
    obj: &crate::scene::SceneObject,
) {
    ui.strong(&obj.name);
    ui.separator();

    // Base properties
    egui::CollapsingHeader::new("Base")
        .default_open(true)
        .show(ui, |ui| {
            egui::Grid::new("base_grid").num_columns(2).show(ui, |ui| {
                ui.label("Label:");
                ui.label(&obj.name);
                ui.end_row();

                if let Some(params) = &obj.params {
                    ui.label("Type:");
                    ui.label(params_type_name(params));
                    ui.end_row();
                }

                ui.label("Visible:");
                ui.label(if obj.visible { "Yes" } else { "No" });
                ui.end_row();
            });
        });

    // Creation parameters (editable)
    if let Some(params) = &obj.params {
        egui::CollapsingHeader::new("Parameters")
            .default_open(true)
            .show(ui, |ui| {
                draw_params_editor(ui, gui, obj.id, params);
            });
    }

    // Topology info
    egui::CollapsingHeader::new("Topology")
        .default_open(false)
        .show(ui, |ui| {
            let m = &obj.model;
            egui::Grid::new("topo_grid").num_columns(2).show(ui, |ui| {
                ui.label("Solids:");
                ui.label(format!("{}", m.solids.len()));
                ui.end_row();
                ui.label("Shells:");
                ui.label(format!("{}", m.shells.len()));
                ui.end_row();
                ui.label("Faces:");
                ui.label(format!("{}", m.faces.len()));
                ui.end_row();
                ui.label("Edges:");
                ui.label(format!("{}", m.edges.len()));
                ui.end_row();
                ui.label("Vertices:");
                ui.label(format!("{}", m.vertices.len()));
                ui.end_row();
            });
        });

    // Mesh info
    egui::CollapsingHeader::new("Mesh")
        .default_open(false)
        .show(ui, |ui| {
            egui::Grid::new("mesh_grid").num_columns(2).show(ui, |ui| {
                ui.label("Triangles:");
                ui.label(format!("{}", obj.mesh.triangle_count()));
                ui.end_row();
                ui.label("Vertices:");
                ui.label(format!("{}", obj.mesh.vertices.len()));
                ui.end_row();
            });
        });

    // Mass properties
    egui::CollapsingHeader::new("Mass Properties")
        .default_open(false)
        .show(ui, |ui| {
            let props = cadkernel_modeling::compute_mass_properties(&obj.mesh);
            egui::Grid::new("mass_grid").num_columns(2).show(ui, |ui| {
                ui.label("Volume:");
                ui.label(format!("{:.4}", props.volume));
                ui.end_row();
                ui.label("Surface Area:");
                ui.label(format!("{:.4}", props.surface_area));
                ui.end_row();
                ui.label("Centroid X:");
                ui.label(format!("{:.4}", props.centroid.x));
                ui.end_row();
                ui.label("Centroid Y:");
                ui.label(format!("{:.4}", props.centroid.y));
                ui.end_row();
                ui.label("Centroid Z:");
                ui.label(format!("{:.4}", props.centroid.z));
                ui.end_row();
            });
        });
}

fn draw_view_tab(
    ui: &mut egui::Ui,
    gui: &mut GuiState,
    obj: &crate::scene::SceneObject,
) {
    ui.strong("Display Properties");
    ui.separator();

    egui::CollapsingHeader::new("Display")
        .default_open(true)
        .show(ui, |ui| {
            egui::Grid::new("display_grid").num_columns(2).show(ui, |ui| {
                // Color display
                ui.label("Shape Color:");
                let [r, g, b, a] = obj.color;
                let mut rgba = egui::Rgba::from_rgba_premultiplied(r, g, b, a);
                if egui::color_picker::color_edit_button_rgba(ui, &mut rgba, egui::color_picker::Alpha::Opaque).changed() {
                    gui.actions.push(GuiAction::SetObjectColor {
                        id: obj.id,
                        color: [rgba.r(), rgba.g(), rgba.b(), rgba.a()],
                    });
                }
                ui.end_row();

                ui.label("Visibility:");
                ui.label(if obj.visible { "\u{25C9} Visible" } else { "\u{25CB} Hidden" });
                ui.end_row();

                ui.label("Selection:");
                ui.label(if obj.selected { "\u{2714} Selected" } else { "\u{2717} Not selected" });
                ui.end_row();

                ui.label("Transparency:");
                let alpha = (1.0 - obj.color[3]) * 100.0;
                ui.label(format!("{alpha:.0}%"));
                ui.end_row();
            });
        });

    // Transparency slider
    egui::CollapsingHeader::new("Transparency")
        .default_open(false)
        .show(ui, |ui| {
            let mut alpha = obj.color[3];
            if ui.add(egui::Slider::new(&mut alpha, 0.1..=1.0).text("Opacity")).changed() {
                let mut c = obj.color;
                c[3] = alpha;
                gui.actions.push(GuiAction::SetObjectColor { id: obj.id, color: c });
            }
        });

    egui::CollapsingHeader::new("Object Name")
        .default_open(true)
        .show(ui, |ui| {
            ui.label(&obj.name);
        });
}

fn draw_scene_overview(ui: &mut egui::Ui, scene: &Scene) {
    ui.strong("Scene Overview");
    ui.separator();

    egui::Grid::new("overview_grid").num_columns(2).show(ui, |ui| {
        ui.label("Objects:");
        ui.label(format!("{}", scene.len()));
        ui.end_row();
        ui.label("Visible:");
        ui.label(format!("{}", scene.visible_objects().count()));
        ui.end_row();
    });

    if scene.is_empty() {
        ui.separator();
        ui.weak("No objects in scene.\nCreate or import a model to start.");
    }
}

fn draw_params_editor(
    ui: &mut egui::Ui,
    gui: &mut GuiState,
    id: crate::scene::ObjectId,
    params: &CreationParams,
) {
    // Editable parameter fields — changes emit GuiAction to rebuild the object
    match params {
        CreationParams::Box { width, height, depth } => {
            let (mut w, mut h, mut d) = (*width, *height, *depth);
            let mut changed = false;
            egui::Grid::new("box_params").num_columns(2).show(ui, |ui| {
                ui.label("Width:");
                changed |= ui.add(egui::DragValue::new(&mut w).range(0.1..=1000.0).speed(0.1)).changed();
                ui.end_row();
                ui.label("Height:");
                changed |= ui.add(egui::DragValue::new(&mut h).range(0.1..=1000.0).speed(0.1)).changed();
                ui.end_row();
                ui.label("Depth:");
                changed |= ui.add(egui::DragValue::new(&mut d).range(0.1..=1000.0).speed(0.1)).changed();
                ui.end_row();
            });
            if changed {
                gui.actions.push(GuiAction::RebuildObject {
                    id,
                    params: CreationParams::Box { width: w, height: h, depth: d },
                });
            }
        }
        CreationParams::Cylinder { radius, height } => {
            let (mut r, mut h) = (*radius, *height);
            let mut changed = false;
            egui::Grid::new("cyl_params").num_columns(2).show(ui, |ui| {
                ui.label("Radius:");
                changed |= ui.add(egui::DragValue::new(&mut r).range(0.1..=500.0).speed(0.1)).changed();
                ui.end_row();
                ui.label("Height:");
                changed |= ui.add(egui::DragValue::new(&mut h).range(0.1..=1000.0).speed(0.1)).changed();
                ui.end_row();
            });
            if changed {
                gui.actions.push(GuiAction::RebuildObject {
                    id,
                    params: CreationParams::Cylinder { radius: r, height: h },
                });
            }
        }
        CreationParams::Sphere { radius } => {
            let mut r = *radius;
            let mut changed = false;
            egui::Grid::new("sph_params").num_columns(2).show(ui, |ui| {
                ui.label("Radius:");
                changed |= ui.add(egui::DragValue::new(&mut r).range(0.1..=500.0).speed(0.1)).changed();
                ui.end_row();
            });
            if changed {
                gui.actions.push(GuiAction::RebuildObject {
                    id,
                    params: CreationParams::Sphere { radius: r },
                });
            }
        }
        CreationParams::Cone { base_radius, top_radius, height } => {
            let (mut br, mut tr, mut h) = (*base_radius, *top_radius, *height);
            let mut changed = false;
            egui::Grid::new("cone_params").num_columns(2).show(ui, |ui| {
                ui.label("Base Radius:");
                changed |= ui.add(egui::DragValue::new(&mut br).range(0.01..=500.0).speed(0.1)).changed();
                ui.end_row();
                ui.label("Top Radius:");
                changed |= ui.add(egui::DragValue::new(&mut tr).range(0.0..=500.0).speed(0.1)).changed();
                ui.end_row();
                ui.label("Height:");
                changed |= ui.add(egui::DragValue::new(&mut h).range(0.1..=1000.0).speed(0.1)).changed();
                ui.end_row();
            });
            if changed {
                gui.actions.push(GuiAction::RebuildObject {
                    id,
                    params: CreationParams::Cone { base_radius: br, top_radius: tr, height: h },
                });
            }
        }
        CreationParams::Torus { major_radius, minor_radius } => {
            let (mut mr, mut mnr) = (*major_radius, *minor_radius);
            let mut changed = false;
            egui::Grid::new("tor_params").num_columns(2).show(ui, |ui| {
                ui.label("Major Radius:");
                changed |= ui.add(egui::DragValue::new(&mut mr).range(0.1..=500.0).speed(0.1)).changed();
                ui.end_row();
                ui.label("Minor Radius:");
                changed |= ui.add(egui::DragValue::new(&mut mnr).range(0.01..=200.0).speed(0.05)).changed();
                ui.end_row();
            });
            if changed {
                gui.actions.push(GuiAction::RebuildObject {
                    id,
                    params: CreationParams::Torus { major_radius: mr, minor_radius: mnr },
                });
            }
        }
        _ => {
            ui.weak("(parameters not editable for this type)");
        }
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
    }
}
