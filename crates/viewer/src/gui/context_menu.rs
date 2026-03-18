use super::{GuiAction, GuiState, SelectedEntity};
use cadkernel_topology::{Handle, SolidData};

pub(crate) fn solid_context_menu(
    ui: &mut egui::Ui,
    gui: &mut GuiState,
    _handle: Handle<SolidData>,
) {
    if ui.button("Select").clicked() {
        gui.selected_entity = Some(SelectedEntity::Solid(_handle));
        ui.close_menu();
    }
    if ui.button("Delete").clicked() {
        gui.actions.push(GuiAction::DeleteSelected);
        gui.selected_entity = Some(SelectedEntity::Solid(_handle));
        ui.close_menu();
    }
    ui.separator();
    if ui.button("Measure").clicked() {
        gui.actions.push(GuiAction::MeasureSolid);
        ui.close_menu();
    }
    if ui.button("Check Geometry").clicked() {
        gui.actions.push(GuiAction::CheckGeometry);
        ui.close_menu();
    }
    ui.separator();
    if ui.button("Export STL…").clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("STL", &["stl"])
            .set_file_name("model.stl")
            .save_file()
        {
            gui.actions.push(GuiAction::ExportStl(path));
        }
        ui.close_menu();
    }
}

#[allow(dead_code)]
pub(crate) fn viewport_context_menu(
    ui: &mut egui::Ui,
    gui: &mut GuiState,
) {
    use crate::render::{DisplayMode, StandardView};

    if ui.button("Fit All  (V)").clicked() {
        gui.actions.push(GuiAction::FitAll);
        ui.close_menu();
    }
    if ui.button("Reset Camera").clicked() {
        gui.actions.push(GuiAction::ResetCamera);
        ui.close_menu();
    }
    ui.separator();

    ui.menu_button("Standard Views", |ui| {
        for &(view, label) in &[
            (StandardView::Front, "Front"),
            (StandardView::Back, "Back"),
            (StandardView::Right, "Right"),
            (StandardView::Left, "Left"),
            (StandardView::Top, "Top"),
            (StandardView::Bottom, "Bottom"),
            (StandardView::Isometric, "Isometric"),
        ] {
            if ui.button(label).clicked() {
                gui.actions.push(GuiAction::SetStandardView(view));
                ui.close_menu();
            }
        }
    });

    ui.menu_button("Display Mode", |ui| {
        for &mode in DisplayMode::ALL {
            if ui.button(mode.label()).clicked() {
                gui.actions.push(GuiAction::SetDisplayMode(mode));
                ui.close_menu();
            }
        }
    });

    ui.separator();
    if ui.button("Select All").clicked() {
        gui.actions.push(GuiAction::SelectAll);
        ui.close_menu();
    }
    if ui.button("Deselect All").clicked() {
        gui.actions.push(GuiAction::DeselectAll);
        ui.close_menu();
    }
}
