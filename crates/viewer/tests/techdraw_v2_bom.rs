#[allow(dead_code)]
#[path = "../src/gui/techdraw.rs"]
mod techdraw_gui;

use cadkernel_io::{
    DrawingSheet, Mesh, drawing_to_dxf, drawing_to_svg,
    techdraw_dxf::{TechDrawBomCsvRow, techdraw_bom_to_csv},
};
use cadkernel_viewer::scene::Scene;
use techdraw_gui::{Bom, BomEntry};

#[test]
fn bom_entry_validates_item_quantity_and_description() {
    assert!(BomEntry::new(0, 1, "Bracket", "Steel").is_err());
    assert!(BomEntry::new(1, 0, "Bracket", "Steel").is_err());
    assert!(BomEntry::new(1, 1, " ", "Steel").is_err());

    let entry = BomEntry::new(1, 2, "Bracket", " ").expect("bom entry");
    assert_eq!(entry.material, "Unspecified");
}

#[test]
fn bom_from_parts_groups_duplicates_and_auto_numbers() {
    let bom = Bom::from_parts([
        ("Bracket", "Steel"),
        ("Bracket", "Steel"),
        ("Cover", "Aluminum"),
    ]);

    assert_eq!(bom.entries.len(), 2);
    assert_eq!(bom.entries[0].item_no, 1);
    assert_eq!(bom.entries[0].description, "Bracket");
    assert_eq!(bom.entries[0].qty, 2);
    assert_eq!(bom.entries[1].item_no, 2);
    assert_eq!(bom.entries[1].description, "Cover");
}

#[test]
fn bom_from_parts_keeps_material_variants_separate() {
    let bom = Bom::from_parts([
        ("Spacer", "Steel"),
        ("Spacer", "Aluminum"),
        ("Spacer", "Steel"),
    ]);

    assert_eq!(bom.entries.len(), 2);
    assert!(bom.entries.iter().any(|entry| entry.qty == 2));
    assert!(bom.entries.iter().any(|entry| entry.material == "Aluminum"));
}

#[test]
fn bom_new_renumbers_manual_entries_in_order() {
    let entries = vec![
        BomEntry::new(99, 4, "Washer", "Steel").expect("washer"),
        BomEntry::new(42, 1, "Shaft", "4140").expect("shaft"),
    ];
    let bom = Bom::new(entries).expect("bom");

    assert_eq!(bom.entries[0].item_no, 1);
    assert_eq!(bom.entries[1].item_no, 2);
}

#[test]
fn bom_push_manual_appends_next_item_number() {
    let mut bom = Bom::from_parts([("Base", "Cast Iron")]);
    bom.push_manual(6, "Screw M4", "Steel").expect("manual row");

    assert_eq!(bom.entries.len(), 2);
    assert_eq!(bom.entries[1].item_no, 2);
    assert_eq!(bom.entries[1].qty, 6);
}

#[test]
fn bom_layout_places_table_at_sheet_bottom_right() {
    let bom = Bom::from_parts([("Base", "Cast Iron"), ("Cover", "Aluminum")]);
    let layout = bom.layout_bottom_right(297.0, 210.0);

    assert!(layout.x > 190.0);
    assert!(layout.y > 175.0);
    assert_eq!(layout.width, 92.0);
    assert_eq!(layout.height, 21.0);
}

#[test]
fn bom_standalone_svg_renders_table_grid_and_rows() {
    let bom = Bom::from_parts([("Base <LH>", "Cast & Iron"), ("Cover", "Aluminum")]);
    let svg = bom.to_svg_bottom_right(297.0, 210.0);

    assert!(svg.contains("techdraw-bom"));
    assert!(svg.contains("<rect"));
    assert!(svg.contains("<line"));
    assert!(svg.contains("DESCRIPTION"));
    assert!(svg.contains("Base &lt;LH&gt;"));
    assert!(svg.contains("Cast &amp; Iron"));
}

#[test]
fn bom_apply_to_sheet_uses_existing_svg_export_path() {
    let mut sheet = DrawingSheet::a4_landscape();
    let bom = Bom::from_parts([("Base", "Cast Iron"), ("Cover", "Aluminum")]);
    bom.apply_to_sheet(&mut sheet, 3.5);

    let svg = drawing_to_svg(&sheet).render();
    assert!(svg.contains("ITEM QTY DESCRIPTION MATERIAL"));
    assert!(svg.contains("Base Cast Iron"));
    assert!(svg.contains("Cover Aluminum"));
}

#[test]
fn bom_apply_to_sheet_uses_existing_dxf_export_path() {
    let mut sheet = DrawingSheet::a4_landscape();
    let bom = Bom::from_parts([("Base", "Cast Iron")]);
    bom.apply_to_sheet(&mut sheet, 3.5);

    let dxf = drawing_to_dxf(&sheet).into_string();
    assert!(dxf.contains("Annotations"));
    assert!(dxf.contains("Base Cast Iron"));
}

#[test]
fn bom_csv_export_quotes_special_fields() {
    let rows = vec![
        TechDrawBomCsvRow {
            item_no: 1,
            qty: 2,
            description: "Plate, left".into(),
            material: "A36 \"hot\"".into(),
        },
        TechDrawBomCsvRow {
            item_no: 2,
            qty: 1,
            description: "Pin".into(),
            material: "Steel".into(),
        },
    ];
    let csv = techdraw_bom_to_csv(&rows);

    assert!(csv.starts_with("item_no,qty,description,material\n"));
    assert!(csv.contains("1,2,\"Plate, left\",\"A36 \"\"hot\"\"\""));
    assert!(csv.contains("2,1,Pin,Steel"));
}

#[test]
fn bom_to_csv_uses_auto_numbered_entries() {
    let bom = Bom::from_parts([("Bracket", "Steel"), ("Bracket", "Steel")]);
    let csv = bom.to_csv();

    assert!(csv.contains("1,2,Bracket,Steel"));
}

#[test]
fn scene_exports_visible_unsuppressed_bom_parts() {
    let mut scene = Scene::new();
    let keep_a = scene.add_mesh_object("Bracket", Mesh::new(), None, None);
    let keep_b = scene.add_mesh_object("Bracket", Mesh::new(), None, None);
    let hide = scene.add_mesh_object("Hidden", Mesh::new(), None, None);
    let suppress = scene.add_mesh_object("", Mesh::new(), None, None);

    scene.get_mut(hide).expect("hidden object").visible = false;
    scene
        .get_mut(suppress)
        .expect("suppressed object")
        .suppressed = true;

    let parts = scene.techdraw_bom_parts();
    assert_eq!(parts.len(), 2);
    assert!(parts.iter().all(|part| part.description == "Bracket"));

    let bom = Bom::from_parts(
        parts
            .into_iter()
            .map(|part| (part.description, part.material)),
    );
    assert_eq!(bom.entries.len(), 1);
    assert_eq!(bom.entries[0].qty, 2);
    assert!(scene.get(keep_a).is_some());
    assert!(scene.get(keep_b).is_some());
}
