//! TechDraw DXF export — writes a [`DrawingSheet`] as a DXF document with
//! one `LWPOLYLINE` per drawing view edge, grouped onto a per-view layer.
//!
//! This is the API the viewer dispatcher's `TechDrawAction::ExportDxf` arm
//! calls. Returns a [`DxfDocument`] whose `write_to(path)` saves the file.
//!
//! Distinct from `dxf::export_drawing_dxf`, which emits `LINE` entities on
//! generic `Visible`/`Hidden` layers. The two are both useful: `LINE` is
//! easier for raw 2D readers; `LWPOLYLINE` per view preserves view identity
//! for round-tripping into AutoCAD-style multi-view sheets.
//!
//! Coordinate system: DXF model space, millimetres, origin at sheet
//! bottom-left (matching `DrawingSheet.width`/`.height`). Y is up.
//!
//! # Example
//!
//! ```no_run
//! use cadkernel_io::techdraw::{DrawingSheet, DrawingView, ProjectionDir, ProjectedEdge};
//! use cadkernel_io::techdraw_dxf::drawing_to_dxf;
//!
//! let mut sheet = DrawingSheet::a4_landscape();
//! sheet.title = "demo".into();
//! sheet.views.push(DrawingView {
//!     direction: ProjectionDir::Front,
//!     edges: vec![ProjectedEdge { x1: 0.0, y1: 0.0, x2: 10.0, y2: 0.0, visible: true }],
//!     center_x: 100.0,
//!     center_y: 100.0,
//!     scale: 1.0,
//!     sheet_x: None,
//!     sheet_y: None,
//!     sheet_scale: None,
//! });
//! drawing_to_dxf(&sheet).write_to("/tmp/sheet.dxf").unwrap();
//! ```

use std::fmt::Write;
use std::path::Path;

use cadkernel_core::{KernelError, KernelResult};

use crate::techdraw::{DrawingSheet, DrawingView, ProjectedEdge};

/// In-memory DXF document produced by [`drawing_to_dxf`].
#[derive(Debug, Clone)]
pub struct DxfDocument {
    body: String,
}

impl DxfDocument {
    /// Returns the DXF text content.
    pub fn as_str(&self) -> &str {
        &self.body
    }

    /// Consumes the document and returns the owned `String`.
    pub fn into_string(self) -> String {
        self.body
    }

    /// Writes the DXF content to `path`.
    pub fn write_to(&self, path: impl AsRef<Path>) -> KernelResult<()> {
        std::fs::write(path.as_ref(), &self.body).map_err(|e| KernelError::IoError(e.to_string()))
    }
}

/// Renders a [`DrawingSheet`] to a [`DxfDocument`] using `LWPOLYLINE` entities
/// grouped by view onto per-view layers.
pub fn drawing_to_dxf(sheet: &DrawingSheet) -> DxfDocument {
    let mut out = String::with_capacity(2048);

    // HEADER
    out.push_str("0\nSECTION\n2\nHEADER\n");
    out.push_str("9\n$ACADVER\n1\nAC1021\n");
    let _ = write!(
        out,
        "9\n$EXTMIN\n10\n0.0\n20\n0.0\n9\n$EXTMAX\n10\n{}\n20\n{}\n",
        sheet.width, sheet.height
    );
    out.push_str("0\nENDSEC\n");

    // TABLES — define one layer per view + a Border layer.
    out.push_str("0\nSECTION\n2\nTABLES\n");
    out.push_str("0\nTABLE\n2\nLAYER\n");
    write_layer(&mut out, "Border", 7);
    write_layer(&mut out, "Title", 7);
    for (idx, view) in sheet.views.iter().enumerate() {
        let layer = view_layer_name(idx, view);
        // ACI palette: cycle 1..=6 so each view gets a distinct colour.
        let colour = ((idx % 6) as i32) + 1;
        write_layer(&mut out, &layer, colour);
    }
    out.push_str("0\nENDTAB\n0\nENDSEC\n");

    // ENTITIES
    out.push_str("0\nSECTION\n2\nENTITIES\n");

    // Sheet border as a closed LWPOLYLINE on the Border layer.
    write_lwpolyline_closed(
        &mut out,
        "Border",
        &[
            (0.0, 0.0),
            (sheet.width, 0.0),
            (sheet.width, sheet.height),
            (0.0, sheet.height),
        ],
    );

    // Each view's edges become one LWPOLYLINE per edge on the view's layer.
    // (LWPOLYLINE accepts any vertex count; we use 2-vertex polylines so
    // disconnected edges remain disconnected — matching the input semantics.)
    for (idx, view) in sheet.views.iter().enumerate() {
        let layer = view_layer_name(idx, view);
        for edge in &view.edges {
            let (x1, y1, x2, y2) = transform_edge(view, edge);
            write_lwpolyline_open(&mut out, &layer, &[(x1, y1), (x2, y2)]);
        }
    }

    // Title block (optional).
    if !sheet.title.is_empty() {
        let _ = write!(
            out,
            "0\nTEXT\n8\nTitle\n10\n10.0\n20\n5.0\n40\n5.0\n1\n{}\n",
            sanitize_text(&sheet.title)
        );
    }

    out.push_str("0\nENDSEC\n0\nEOF\n");

    DxfDocument { body: out }
}

fn view_layer_name(idx: usize, view: &DrawingView) -> String {
    format!("View_{:02}_{}", idx, view.direction.label())
}

fn transform_edge(view: &DrawingView, edge: &ProjectedEdge) -> (f64, f64, f64, f64) {
    (
        view.center_x + edge.x1 * view.scale,
        view.center_y + edge.y1 * view.scale,
        view.center_x + edge.x2 * view.scale,
        view.center_y + edge.y2 * view.scale,
    )
}

fn write_layer(out: &mut String, name: &str, colour: i32) {
    let _ = write!(
        out,
        "0\nLAYER\n2\n{}\n70\n0\n62\n{}\n6\nCONTINUOUS\n",
        name, colour
    );
}

fn write_lwpolyline_open(out: &mut String, layer: &str, points: &[(f64, f64)]) {
    write_lwpolyline(out, layer, points, false);
}

fn write_lwpolyline_closed(out: &mut String, layer: &str, points: &[(f64, f64)]) {
    write_lwpolyline(out, layer, points, true);
}

fn write_lwpolyline(out: &mut String, layer: &str, points: &[(f64, f64)], closed: bool) {
    let _ = write!(
        out,
        "0\nLWPOLYLINE\n8\n{}\n90\n{}\n70\n{}\n",
        layer,
        points.len(),
        if closed { 1 } else { 0 }
    );
    for (x, y) in points {
        let _ = write!(out, "10\n{}\n20\n{}\n", x, y);
    }
}

fn sanitize_text(s: &str) -> String {
    // DXF text fields cannot contain raw newlines; collapse any whitespace.
    s.replace(['\n', '\r'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::techdraw::{ProjectedEdge, ProjectionDir};

    fn unit_square_view() -> DrawingView {
        DrawingView {
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
        }
    }

    #[test]
    fn drawing_to_dxf_emits_section_markers() {
        let mut sheet = DrawingSheet::a4_landscape();
        sheet.title = "unit-square".into();
        sheet.views.push(unit_square_view());
        let doc = drawing_to_dxf(&sheet);
        let s = doc.as_str();
        assert!(s.starts_with("0\nSECTION\n"));
        assert!(s.contains("\nHEADER\n"));
        assert!(s.contains("\nENTITIES\n"));
        assert!(s.contains("\nLWPOLYLINE\n"));
        assert!(s.contains("View_00_Front"));
        assert!(s.ends_with("\nEOF\n"));
    }

    #[test]
    fn drawing_to_dxf_one_polyline_per_edge() {
        let mut sheet = DrawingSheet::a4_landscape();
        sheet.views.push(unit_square_view());
        let s = drawing_to_dxf(&sheet).into_string();
        // 4 edges → 4 LWPOLYLINE entries on the view layer, plus 1 for the border.
        let count = s.matches("\nLWPOLYLINE\n").count();
        assert_eq!(
            count, 5,
            "expected 5 LWPOLYLINE entries (1 border + 4 edges), got {count}"
        );
    }
}
