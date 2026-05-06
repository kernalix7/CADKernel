//! Integration tests for TechDraw DXF + PDF export.
//!
//! Round-trips a minimal sheet through both formats by writing to a tempfile
//! and re-reading it, asserting magic bytes / section markers in the output.

use std::path::PathBuf;

use cadkernel_io::techdraw::{DrawingSheet, DrawingView, ProjectedEdge, ProjectionDir};
use cadkernel_io::{drawing_to_dxf, drawing_to_pdf};

fn unit_square_sheet() -> DrawingSheet {
    let mut sheet = DrawingSheet::a4_landscape();
    sheet.title = "techdraw-export-test".into();
    sheet.views.push(DrawingView {
        direction: ProjectionDir::Front,
        edges: vec![
            ProjectedEdge {
                x1: 0.0,
                y1: 0.0,
                x2: 10.0,
                y2: 0.0,
                visible: true,
            },
            ProjectedEdge {
                x1: 10.0,
                y1: 0.0,
                x2: 10.0,
                y2: 10.0,
                visible: true,
            },
            ProjectedEdge {
                x1: 10.0,
                y1: 10.0,
                x2: 0.0,
                y2: 10.0,
                visible: true,
            },
            ProjectedEdge {
                x1: 0.0,
                y1: 10.0,
                x2: 0.0,
                y2: 0.0,
                visible: true,
            },
        ],
        center_x: 100.0,
        center_y: 80.0,
        scale: 2.0,
        sheet_x: None,
        sheet_y: None,
        sheet_scale: None,
    });
    sheet
}

fn tmp_path(stem: &str, ext: &str) -> PathBuf {
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("cadk_techdraw_{stem}_{pid}_{nanos}.{ext}"))
}

#[test]
fn dxf_roundtrip_writes_section_markers() {
    let sheet = unit_square_sheet();
    let path = tmp_path("dxf", "dxf");
    let doc = drawing_to_dxf(&sheet);
    doc.write_to(&path).expect("write_to dxf");

    let content = std::fs::read_to_string(&path).expect("read back dxf");
    assert!(content.contains("\nSECTION\n"), "missing SECTION marker");
    assert!(content.contains("\nENTITIES\n"), "missing ENTITIES section");
    assert!(
        content.contains("\nLWPOLYLINE\n"),
        "missing LWPOLYLINE entity"
    );
    assert!(
        content.contains("View_00_Front"),
        "missing per-view layer name"
    );
    assert!(content.ends_with("\nEOF\n"), "missing DXF EOF terminator");

    let _ = std::fs::remove_file(&path);
}

#[test]
fn pdf_roundtrip_writes_magic_and_eof() {
    let sheet = unit_square_sheet();
    let path = tmp_path("pdf", "pdf");
    let doc = drawing_to_pdf(&sheet).expect("drawing_to_pdf");
    doc.save_to(&path).expect("save_to pdf");

    let content = std::fs::read(&path).expect("read back pdf");
    assert!(content.starts_with(b"%PDF-"), "missing PDF magic header");
    assert!(
        content.windows(5).any(|w| w == b"%%EOF"),
        "missing PDF %%EOF trailer"
    );

    let _ = std::fs::remove_file(&path);
}
