#[allow(dead_code)]
#[path = "../src/gui/techdraw.rs"]
mod techdraw_gui;

use std::collections::BTreeSet;

use cadkernel_io::{DrawingSheet, drawing_to_dxf, drawing_to_svg};
use techdraw_gui::{DatumFeature, DatumLabel, GdtFrame, GdtSymbol};

#[test]
fn gdt_symbol_set_matches_2018_characteristics() {
    assert_eq!(GdtSymbol::ALL.len(), 14);
    assert_eq!(GdtSymbol::Straightness.label(), "Straightness");
    assert_eq!(GdtSymbol::TotalRunout.label(), "Total Runout");
}

#[test]
fn gdt_symbols_have_distinct_control_glyphs() {
    let glyphs: BTreeSet<_> = GdtSymbol::ALL.into_iter().map(GdtSymbol::glyph).collect();
    assert_eq!(glyphs.len(), GdtSymbol::ALL.len());
    assert!(glyphs.contains("⌖"));
    assert!(glyphs.contains("⌰"));
}

#[test]
fn datum_label_accepts_uppercase_letters() {
    let label = DatumLabel::new("AB").expect("datum label");
    assert_eq!(label.as_str(), "AB");
}

#[test]
fn datum_label_rejects_non_standard_labels() {
    assert!(DatumLabel::new("").is_err());
    assert!(DatumLabel::new("a").is_err());
    assert!(DatumLabel::new("ABCD").is_err());
    assert!(DatumLabel::new("A1").is_err());
}

#[test]
fn gdt_frame_validates_tolerance_and_datum_count() {
    assert!(GdtFrame::new(GdtSymbol::Flatness, -0.01, Vec::new()).is_err());
    assert!(GdtFrame::new(GdtSymbol::Flatness, f64::NAN, Vec::new()).is_err());

    let datums = ["A", "B", "C", "D"]
        .into_iter()
        .map(|label| DatumLabel::new(label).expect("datum"))
        .collect();
    assert!(GdtFrame::new(GdtSymbol::Position, 0.05, datums).is_err());
}

#[test]
fn gdt_frame_control_text_matches_feature_control_frame_style() {
    let frame = GdtFrame::new(
        GdtSymbol::Position,
        0.05,
        vec![
            DatumLabel::new("A").expect("datum A"),
            DatumLabel::new("B").expect("datum B"),
        ],
    )
    .expect("gdt frame");

    assert_eq!(frame.control_text(), "⌖ 0.050 [A] [B]");
}

#[test]
fn gdt_frame_can_be_written_as_sheet_text_annotation() {
    let frame = GdtFrame::new(
        GdtSymbol::Perpendicularity,
        0.02,
        vec![DatumLabel::new("A").expect("datum A")],
    )
    .expect("gdt frame");
    let ann = frame.to_text_annotation(42.0, 84.0, 5.5);

    assert_eq!(ann.position.x, 42.0);
    assert_eq!(ann.position.y, 84.0);
    assert_eq!(ann.font_size, 5.5);
    assert_eq!(ann.text, "⟂ 0.020 [A]");
}

#[test]
fn gdt_frame_applies_to_existing_svg_export_path() {
    let mut sheet = DrawingSheet::a4_landscape();
    let frame = GdtFrame::new(
        GdtSymbol::Flatness,
        0.01,
        vec![DatumLabel::new("C").expect("datum C")],
    )
    .expect("gdt frame");
    frame.apply_to_sheet(&mut sheet, 32.0, 48.0, 5.0);

    let svg = drawing_to_svg(&sheet).render();
    assert!(svg.contains("▱ 0.010 [C]"));
}

#[test]
fn gdt_frame_applies_to_techdraw_dxf_annotations() {
    let mut sheet = DrawingSheet::a4_landscape();
    let frame = GdtFrame::new(
        GdtSymbol::Runout,
        0.08,
        vec![DatumLabel::new("A").expect("datum A")],
    )
    .expect("gdt frame");
    frame.apply_to_sheet(&mut sheet, 50.0, 60.0, 4.0);

    let dxf = drawing_to_dxf(&sheet).into_string();
    assert!(dxf.contains("Annotations"));
    assert!(dxf.contains("↗ 0.080 [A]"));
}

#[test]
fn gdt_frame_standalone_svg_renders_control_boxes() {
    let frame = GdtFrame::new(
        GdtSymbol::TotalRunout,
        0.125,
        vec![
            DatumLabel::new("A").expect("datum A"),
            DatumLabel::new("B").expect("datum B"),
            DatumLabel::new("C").expect("datum C"),
        ],
    )
    .expect("gdt frame");

    let svg = frame.to_svg_at(10.0, 20.0);
    assert!(svg.contains("techdraw-gdt-frame"));
    assert!(svg.contains("data-characteristic=\"Total Runout\""));
    assert!(svg.contains(">⌰<"));
    assert!(svg.contains(">0.125<"));
    assert!(svg.contains(">A<"));
    assert!(svg.contains("<line"));
}

#[test]
fn datum_feature_svg_renders_datum_box() {
    let feature = DatumFeature::new(DatumLabel::new("D").expect("datum D"), 24.0, 36.0)
        .expect("datum feature");
    let svg = feature.to_svg();

    assert!(svg.contains("techdraw-datum"));
    assert!(svg.contains("<rect"));
    assert!(svg.contains(">D<"));
}
