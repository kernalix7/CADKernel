//! TechDraw PDF export — renders a [`DrawingSheet`] to a single-page A4
//! landscape PDF document.
//!
//! This is the API the viewer dispatcher's `TechDrawAction::ExportPdf` arm
//! calls. Returns a [`PdfDocument`] whose `save_to(path)` writes the file.
//!
//! Implementation note: routes through the existing [`drawing_to_svg`]
//! renderer + the [`crate::pdf::export_pdf`] SVG-to-PDF backend, so the same
//! drawing layout (single-view centred, 2-views side-by-side, 3-views
//! third-angle) appears identically in DXF / SVG / PDF outputs.
//!
//! Page geometry: the sheet is exported at the [`DrawingSheet`]'s declared
//! `width`×`height` in millimetres (A4 landscape = 297×210 mm). A4 portrait
//! and other paper sizes carry through transparently because we honour the
//! sheet's intrinsic dimensions rather than hard-coding a page size.
//!
//! Multi-page support is NOT implemented; this is a single-page exporter.
//!
//! # Example
//!
//! ```no_run
//! use cadkernel_io::techdraw::{DrawingSheet, DrawingView, ProjectionDir, ProjectedEdge};
//! use cadkernel_io::techdraw_pdf::drawing_to_pdf;
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
//! drawing_to_pdf(&sheet).unwrap().save_to("/tmp/sheet.pdf").unwrap();
//! ```

use std::path::Path;

use cadkernel_core::{KernelError, KernelResult};

use crate::pdf::export_pdf;
use crate::svg::SvgDocument;
use crate::techdraw::{DrawingSheet, drawing_to_svg};

/// In-memory PDF document produced by [`drawing_to_pdf`].
#[derive(Debug, Clone)]
pub struct PdfDocument {
    bytes: Vec<u8>,
    page_width_mm: f64,
    page_height_mm: f64,
}

impl PdfDocument {
    /// Returns the raw PDF bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consumes the document and returns the owned `Vec<u8>`.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Page width in millimetres.
    pub fn page_width_mm(&self) -> f64 {
        self.page_width_mm
    }

    /// Page height in millimetres.
    pub fn page_height_mm(&self) -> f64 {
        self.page_height_mm
    }

    /// Writes the PDF bytes to `path`.
    pub fn save_to(&self, path: impl AsRef<Path>) -> KernelResult<()> {
        std::fs::write(path.as_ref(), &self.bytes).map_err(|e| KernelError::IoError(e.to_string()))
    }
}

/// Renders a [`DrawingSheet`] to a [`PdfDocument`].
///
/// Returns an error if the sheet has zero-area dimensions or if the
/// underlying PDF writer fails.
pub fn drawing_to_pdf(sheet: &DrawingSheet) -> KernelResult<PdfDocument> {
    if sheet.width <= 0.0 || sheet.height <= 0.0 {
        return Err(KernelError::InvalidArgument(format!(
            "sheet has non-positive dimensions: {} x {}",
            sheet.width, sheet.height
        )));
    }

    let svg: SvgDocument = drawing_to_svg(sheet);
    let svg_bytes = svg.to_string();
    let bytes = export_pdf(&svg_bytes, sheet.width, sheet.height)?;

    Ok(PdfDocument {
        bytes,
        page_width_mm: sheet.width,
        page_height_mm: sheet.height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::techdraw::{DrawingView, ProjectedEdge, ProjectionDir};

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
    fn drawing_to_pdf_emits_magic_and_eof() {
        let mut sheet = DrawingSheet::a4_landscape();
        sheet.title = "unit-square".into();
        sheet.views.push(unit_square_view());
        let doc = drawing_to_pdf(&sheet).expect("drawing_to_pdf");
        let bytes = doc.as_bytes();
        assert!(bytes.starts_with(b"%PDF-"), "missing PDF magic header");
        assert!(
            bytes.windows(5).any(|w| w == b"%%EOF"),
            "missing PDF %%EOF trailer"
        );
        // A4 landscape passthrough.
        assert!((doc.page_width_mm() - 297.0).abs() < 1e-6);
        assert!((doc.page_height_mm() - 210.0).abs() < 1e-6);
    }

    #[test]
    fn drawing_to_pdf_rejects_zero_area_sheet() {
        let mut sheet = DrawingSheet::a4_landscape();
        sheet.width = 0.0;
        let err = drawing_to_pdf(&sheet).unwrap_err();
        match err {
            KernelError::InvalidArgument(_) => {}
            other => panic!("expected InvalidArgument, got {other:?}"),
        }
    }
}
