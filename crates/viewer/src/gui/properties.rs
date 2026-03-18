use super::{GuiState, SelectedEntity};
use cadkernel_io::Mesh;
use cadkernel_modeling::compute_mass_properties;
use cadkernel_topology::BRepModel;

pub(crate) fn draw_properties(
    ctx: &egui::Context,
    gui: &mut GuiState,
    model: &BRepModel,
    mesh: &Option<Mesh>,
) {
    if !gui.show_properties {
        return;
    }
    egui::SidePanel::right("properties")
        .default_width(260.0)
        .show(ctx, |ui| {
            ui.heading("Properties");
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                match &gui.selected_entity {
                    Some(SelectedEntity::Solid(handle)) => {
                        draw_solid_props(ui, gui, model, mesh, *handle);
                    }
                    Some(SelectedEntity::Shell(handle)) => {
                        draw_shell_props(ui, model, *handle);
                    }
                    Some(SelectedEntity::Face(handle)) => {
                        draw_face_props(ui, model, *handle);
                    }
                    Some(SelectedEntity::Edge(handle)) => {
                        draw_edge_props(ui, model, *handle);
                    }
                    Some(SelectedEntity::Vertex(handle)) => {
                        draw_vertex_props(ui, model, *handle);
                    }
                    None => {
                        draw_model_overview(ui, gui, model, mesh);
                    }
                }

                if let Some(path) = &gui.current_file {
                    ui.separator();
                    ui.strong("File");
                    ui.label(path.as_str());
                }
            });
        });
}

fn draw_model_overview(
    ui: &mut egui::Ui,
    gui: &mut GuiState,
    model: &BRepModel,
    mesh: &Option<Mesh>,
) {
    ui.strong("Model Overview");
    egui::Grid::new("overview_grid")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            ui.label("Solids:");
            ui.label(format!("{}", model.solids.len()));
            ui.end_row();
            ui.label("Shells:");
            ui.label(format!("{}", model.shells.len()));
            ui.end_row();
            ui.label("Faces:");
            ui.label(format!("{}", model.faces.len()));
            ui.end_row();
            ui.label("Edges:");
            ui.label(format!("{}", model.edges.len()));
            ui.end_row();
            ui.label("Vertices:");
            ui.label(format!("{}", model.vertices.len()));
            ui.end_row();
        });

    if let Some(mesh) = mesh {
        ui.separator();
        ui.strong("Mesh");
        egui::Grid::new("mesh_grid")
            .num_columns(2)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                ui.label("Vertices:");
                ui.label(format!("{}", mesh.vertices.len()));
                ui.end_row();
                ui.label("Triangles:");
                ui.label(format!("{}", mesh.triangle_count()));
                ui.end_row();
            });

        ui.separator();
        ui.strong("Mass Properties");
        let tri_count = mesh.triangle_count();
        if gui.cached_props.is_none() || gui.cached_props_tri_count != tri_count {
            gui.cached_props = Some(compute_mass_properties(mesh));
            gui.cached_props_tri_count = tri_count;
        }
        if let Some(props) = &gui.cached_props {
            egui::Grid::new("mass_grid")
                .num_columns(2)
                .spacing([12.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Volume:");
                    ui.label(format!("{:.4}", props.volume));
                    ui.end_row();
                    ui.label("Surface area:");
                    ui.label(format!("{:.4}", props.surface_area));
                    ui.end_row();
                    ui.label("Centroid:");
                    ui.label(format!(
                        "({:.2}, {:.2}, {:.2})",
                        props.centroid.x, props.centroid.y, props.centroid.z
                    ));
                    ui.end_row();
                });
        }
    } else {
        ui.separator();
        ui.weak("No model loaded");
        ui.weak("Use File > Open or Create menu");
    }
}

fn draw_solid_props(
    ui: &mut egui::Ui,
    gui: &mut GuiState,
    model: &BRepModel,
    mesh: &Option<Mesh>,
    handle: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
) {
    ui.strong("Solid Properties");
    if let Some(solid) = model.solids.get(handle) {
        egui::Grid::new("solid_grid")
            .num_columns(2)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                ui.label("Handle:");
                ui.label(format!("{:?}", handle));
                ui.end_row();
                if let Some(tag) = &solid.tag {
                    ui.label("Tag:");
                    ui.label(format!("{tag}"));
                    ui.end_row();
                }
                ui.label("Shells:");
                ui.label(format!("{}", solid.shells.len()));
                ui.end_row();

                // Count faces and edges
                let mut face_count = 0;
                let mut edge_set = std::collections::HashSet::new();
                let mut vertex_set = std::collections::HashSet::new();
                for &sh in &solid.shells {
                    if let Some(shell) = model.shells.get(sh) {
                        face_count += shell.faces.len();
                        for &fh in &shell.faces {
                            if let Ok(edges) = model.edges_of_face(fh) {
                                for eh in edges {
                                    edge_set.insert(format!("{:?}", eh));
                                    if let Some(ed) = model.edges.get(eh) {
                                        vertex_set.insert(format!("{:?}", ed.start));
                                        vertex_set.insert(format!("{:?}", ed.end));
                                    }
                                }
                            }
                        }
                    }
                }

                ui.label("Faces:");
                ui.label(format!("{face_count}"));
                ui.end_row();
                ui.label("Edges:");
                ui.label(format!("{}", edge_set.len()));
                ui.end_row();
                ui.label("Vertices:");
                ui.label(format!("{}", vertex_set.len()));
                ui.end_row();
            });

        // Mass properties
        if let Some(mesh) = mesh {
            ui.separator();
            ui.strong("Mass Properties");
            let tri_count = mesh.triangle_count();
            if gui.cached_props.is_none() || gui.cached_props_tri_count != tri_count {
                gui.cached_props = Some(compute_mass_properties(mesh));
                gui.cached_props_tri_count = tri_count;
            }
            if let Some(props) = &gui.cached_props {
                egui::Grid::new("solid_mass_grid")
                    .num_columns(2)
                    .spacing([12.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Volume:");
                        ui.label(format!("{:.4}", props.volume));
                        ui.end_row();
                        ui.label("Surface area:");
                        ui.label(format!("{:.4}", props.surface_area));
                        ui.end_row();
                        ui.label("Centroid:");
                        ui.label(format!(
                            "({:.2}, {:.2}, {:.2})",
                            props.centroid.x, props.centroid.y, props.centroid.z
                        ));
                        ui.end_row();
                    });
            }
        }
    } else {
        ui.weak("(solid not found)");
    }
}

fn draw_shell_props(
    ui: &mut egui::Ui,
    model: &BRepModel,
    handle: cadkernel_topology::Handle<cadkernel_topology::ShellData>,
) {
    ui.strong("Shell Properties");
    if let Some(shell) = model.shells.get(handle) {
        egui::Grid::new("shell_grid")
            .num_columns(2)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                ui.label("Handle:");
                ui.label(format!("{:?}", handle));
                ui.end_row();
                if let Some(tag) = &shell.tag {
                    ui.label("Tag:");
                    ui.label(format!("{tag}"));
                    ui.end_row();
                }
                ui.label("Faces:");
                ui.label(format!("{}", shell.faces.len()));
                ui.end_row();
                ui.label("Parent Solid:");
                ui.label(if shell.solid.is_some() {
                    "Yes"
                } else {
                    "None"
                });
                ui.end_row();
            });
    } else {
        ui.weak("(shell not found)");
    }
}

fn draw_face_props(
    ui: &mut egui::Ui,
    model: &BRepModel,
    handle: cadkernel_topology::Handle<cadkernel_topology::FaceData>,
) {
    ui.strong("Face Properties");
    if let Some(face) = model.faces.get(handle) {
        egui::Grid::new("face_grid")
            .num_columns(2)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                ui.label("Handle:");
                ui.label(format!("{:?}", handle));
                ui.end_row();
                if let Some(tag) = &face.tag {
                    ui.label("Tag:");
                    ui.label(format!("{tag}"));
                    ui.end_row();
                }
                ui.label("Orientation:");
                ui.label(format!("{:?}", face.orientation));
                ui.end_row();
                ui.label("Inner loops:");
                ui.label(format!("{}", face.inner_loops.len()));
                ui.end_row();
                ui.label("Has surface:");
                ui.label(if face.surface.is_some() {
                    "Yes"
                } else {
                    "No"
                });
                ui.end_row();

                // Edge count via edges_of_face
                if let Ok(edges) = model.edges_of_face(handle) {
                    ui.label("Edges:");
                    ui.label(format!("{}", edges.len()));
                    ui.end_row();
                }
            });
    } else {
        ui.weak("(face not found)");
    }
}

fn draw_edge_props(
    ui: &mut egui::Ui,
    model: &BRepModel,
    handle: cadkernel_topology::Handle<cadkernel_topology::EdgeData>,
) {
    ui.strong("Edge Properties");
    if let Some(edge) = model.edges.get(handle) {
        egui::Grid::new("edge_grid")
            .num_columns(2)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                ui.label("Handle:");
                ui.label(format!("{:?}", handle));
                ui.end_row();
                if let Some(tag) = &edge.tag {
                    ui.label("Tag:");
                    ui.label(format!("{tag}"));
                    ui.end_row();
                }
                ui.label("Has curve:");
                ui.label(if edge.curve.is_some() { "Yes" } else { "No" });
                ui.end_row();

                if let Some(sv) = model.vertices.get(edge.start) {
                    ui.label("Start:");
                    ui.label(format!(
                        "({:.3}, {:.3}, {:.3})",
                        sv.point.x, sv.point.y, sv.point.z
                    ));
                    ui.end_row();
                }
                if let Some(ev) = model.vertices.get(edge.end) {
                    ui.label("End:");
                    ui.label(format!(
                        "({:.3}, {:.3}, {:.3})",
                        ev.point.x, ev.point.y, ev.point.z
                    ));
                    ui.end_row();
                }

                // Compute length
                if let (Some(sv), Some(ev)) = (
                    model.vertices.get(edge.start),
                    model.vertices.get(edge.end),
                ) {
                    let dx = ev.point.x - sv.point.x;
                    let dy = ev.point.y - sv.point.y;
                    let dz = ev.point.z - sv.point.z;
                    let length = (dx * dx + dy * dy + dz * dz).sqrt();
                    ui.label("Length:");
                    ui.label(format!("{:.4}", length));
                    ui.end_row();
                }
            });
    } else {
        ui.weak("(edge not found)");
    }
}

fn draw_vertex_props(
    ui: &mut egui::Ui,
    model: &BRepModel,
    handle: cadkernel_topology::Handle<cadkernel_topology::VertexData>,
) {
    ui.strong("Vertex Properties");
    if let Some(vertex) = model.vertices.get(handle) {
        egui::Grid::new("vertex_grid")
            .num_columns(2)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                ui.label("Handle:");
                ui.label(format!("{:?}", handle));
                ui.end_row();
                if let Some(tag) = &vertex.tag {
                    ui.label("Tag:");
                    ui.label(format!("{tag}"));
                    ui.end_row();
                }
                ui.label("X:");
                ui.label(format!("{:.6}", vertex.point.x));
                ui.end_row();
                ui.label("Y:");
                ui.label(format!("{:.6}", vertex.point.y));
                ui.end_row();
                ui.label("Z:");
                ui.label(format!("{:.6}", vertex.point.z));
                ui.end_row();
            });
    } else {
        ui.weak("(vertex not found)");
    }
}
