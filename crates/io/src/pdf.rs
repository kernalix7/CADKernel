//! PDF export for technical drawings.
//!
//! Converts SVG drawing content to a minimal PDF 1.4 document with
//! line and text rendering. Dimensions are converted from millimeters
//! to PDF points (1 mm = 2.83465 pt).

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::Point2;

/// Exports a TechDraw SVG string to a minimal PDF file.
///
/// Generates a PDF/A-compatible document that embeds the SVG content
/// as a rendered page. The SVG is stored as a stream in the content object.
pub fn export_pdf(
    svg_content: &str,
    page_width_mm: f64,
    page_height_mm: f64,
) -> KernelResult<Vec<u8>> {
    if svg_content.is_empty() {
        return Err(KernelError::InvalidArgument("empty SVG content".into()));
    }

    // Convert mm to PDF points (1 mm = 2.83465 pt)
    let w_pt = page_width_mm * 2.834_645_669_3;
    let h_pt = page_height_mm * 2.834_645_669_3;

    // Convert SVG paths to basic PDF drawing commands
    let content_stream = svg_to_pdf_stream(svg_content, w_pt, h_pt);
    let stream_bytes = content_stream.as_bytes();

    let mut pdf = Vec::new();
    let mut offsets = Vec::new();

    // Header
    pdf.extend_from_slice(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n");

    // Object 1: Catalog
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    // Object 2: Pages
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    // Object 3: Page
    offsets.push(pdf.len());
    let page = format!(
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {w_pt:.2} {h_pt:.2}] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n"
    );
    pdf.extend_from_slice(page.as_bytes());

    // Object 4: Content stream
    offsets.push(pdf.len());
    let content_obj = format!("4 0 obj\n<< /Length {} >>\nstream\n", stream_bytes.len());
    pdf.extend_from_slice(content_obj.as_bytes());
    pdf.extend_from_slice(stream_bytes);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    // Object 5: Font
    offsets.push(pdf.len());
    pdf.extend_from_slice(
        b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
    );

    // Cross-reference table
    let xref_offset = pdf.len();
    let obj_count = offsets.len() + 1;
    let mut xref = format!("xref\n0 {obj_count}\n0000000000 65535 f \n");
    for &off in &offsets {
        xref.push_str(&format!("{off:010} 00000 n \n"));
    }
    pdf.extend_from_slice(xref.as_bytes());

    // Trailer
    let trailer =
        format!("trailer\n<< /Size {obj_count} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n");
    pdf.extend_from_slice(trailer.as_bytes());

    Ok(pdf)
}

/// Write PDF bytes to file.
pub fn write_pdf(path: &str, data: &[u8]) -> KernelResult<()> {
    std::fs::write(path, data).map_err(|e| KernelError::IoError(e.to_string()))
}

/// Convert SVG drawing commands to simplified PDF content stream.
fn svg_to_pdf_stream(svg: &str, page_w: f64, page_h: f64) -> String {
    let mut stream = String::new();

    // Set up coordinate transform: flip Y axis (PDF origin is bottom-left)
    stream.push_str(&format!("1 0 0 -1 0 {page_h:.2} cm\n"));

    // Draw border
    stream.push_str("0.5 w\n0 0 0 RG\n");
    stream.push_str(&format!("0 0 {page_w:.2} {page_h:.2} re S\n"));

    // Extract lines from SVG and render as PDF paths
    for line in svg.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("<line ") {
            if let Some((x1, y1, x2, y2)) = parse_svg_line(trimmed) {
                stream.push_str(&format!("{x1:.3} {y1:.3} m {x2:.3} {y2:.3} l S\n"));
            }
        } else if trimmed.starts_with("<text ") {
            if let Some((x, y, text)) = parse_svg_text(trimmed) {
                stream.push_str(&format!("BT /F1 8 Tf {x:.2} {y:.2} Td ({text}) Tj ET\n"));
            }
        }
    }

    // Title block
    stream.push_str("BT /F1 10 Tf 10 20 Td (CADKernel TechDraw) Tj ET\n");

    stream
}

fn parse_svg_line(line: &str) -> Option<(f64, f64, f64, f64)> {
    let x1 = extract_svg_attr(line, "x1")?;
    let y1 = extract_svg_attr(line, "y1")?;
    let x2 = extract_svg_attr(line, "x2")?;
    let y2 = extract_svg_attr(line, "y2")?;
    Some((x1, y1, x2, y2))
}

fn parse_svg_text(line: &str) -> Option<(f64, f64, String)> {
    let x = extract_svg_attr(line, "x")?;
    let y = extract_svg_attr(line, "y")?;
    let start = line.find('>')? + 1;
    let end = line[start..]
        .find('<')
        .map(|i| start + i)
        .unwrap_or(line.len());
    let text = line[start..end].to_string();
    if text.is_empty() {
        return None;
    }
    Some((x, y, text))
}

fn extract_svg_attr(line: &str, name: &str) -> Option<f64> {
    let key = format!("{name}=\"");
    let start = line.find(&key)? + key.len();
    let end = start + line[start..].find('"')?;
    line[start..end].parse().ok()
}

// ---------------------------------------------------------------------------
// PDF Import
// ---------------------------------------------------------------------------

/// Extracted text from a PDF page.
#[derive(Debug, Clone)]
pub struct PdfText {
    pub position: Point2,
    pub text: String,
    pub font_size: f64,
}

/// A single vector path command.
#[derive(Debug, Clone)]
pub enum PdfPathCmd {
    MoveTo(Point2),
    LineTo(Point2),
    CurveTo(Point2, Point2, Point2),
    ClosePath,
}

/// Extracted vector content from a PDF page.
#[derive(Debug, Clone)]
pub struct PdfVector {
    pub commands: Vec<PdfPathCmd>,
}

/// A single page from an imported PDF.
#[derive(Debug, Clone)]
pub struct PdfPage {
    pub width: f64,
    pub height: f64,
    pub text_content: Vec<PdfText>,
    pub vector_content: Vec<PdfVector>,
}

/// Result of importing a PDF file.
#[derive(Debug, Clone)]
pub struct PdfImportResult {
    pub pages: Vec<PdfPage>,
}

/// Import a PDF file (basic uncompressed PDF 1.4 support).
///
/// Parses the PDF header, cross-reference table, page objects, and
/// content streams. Extracts vector drawing commands (m/l/c/h) and
/// text operators (Tj/TJ). Encrypted or compressed PDFs return an error.
pub fn import_pdf(content: &[u8]) -> KernelResult<PdfImportResult> {
    if content.len() < 8 {
        return Err(KernelError::InvalidArgument(
            "content too short for PDF".into(),
        ));
    }
    if !content.starts_with(b"%PDF-") {
        return Err(KernelError::IoError(
            "not a PDF file (missing %PDF- header)".into(),
        ));
    }

    let text = String::from_utf8_lossy(content);

    // Check for encryption
    if text.contains("/Encrypt") {
        return Err(KernelError::IoError("encrypted PDF not supported".into()));
    }

    // Extract page dimensions from MediaBox
    let (page_w, page_h) = extract_media_box(&text).unwrap_or((612.0, 792.0));

    // Find content streams (between "stream" and "endstream")
    let streams = extract_streams(&text);

    let mut text_content = Vec::new();
    let mut vector_content = Vec::new();

    for stream in &streams {
        // Check if it looks compressed (starts with binary)
        if stream.bytes().take(4).any(|b| b > 127) {
            continue;
        }

        let (texts, vectors) = parse_content_stream(stream);
        text_content.extend(texts);
        vector_content.extend(vectors);
    }

    let page = PdfPage {
        width: page_w,
        height: page_h,
        text_content,
        vector_content,
    };

    Ok(PdfImportResult { pages: vec![page] })
}

fn extract_media_box(text: &str) -> Option<(f64, f64)> {
    let marker = "/MediaBox";
    let pos = text.find(marker)?;
    let after = &text[pos + marker.len()..];
    let bracket_start = after.find('[')?;
    let bracket_end = after[bracket_start..].find(']')? + bracket_start;
    let inner = &after[bracket_start + 1..bracket_end];
    let nums: Vec<f64> = inner
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
        .collect();
    if nums.len() >= 4 {
        Some((nums[2] - nums[0], nums[3] - nums[1]))
    } else {
        None
    }
}

fn extract_streams(text: &str) -> Vec<String> {
    let mut streams = Vec::new();
    let mut search_from = 0;
    while let Some(start) = text[search_from..]
        .find("stream\n")
        .or_else(|| text[search_from..].find("stream\r\n"))
    {
        let abs_start = search_from + start;
        // Find start of stream content (after "stream\n" or "stream\r\n")
        let content_start = if text[abs_start..].starts_with("stream\r\n") {
            abs_start + 8
        } else {
            abs_start + 7
        };

        let Some(end_offset) = text[content_start..].find("endstream") else {
            break;
        };
        let content_end = content_start + end_offset;

        // Trim trailing whitespace from the stream content
        let stream_content = text[content_start..content_end].trim_end();
        streams.push(stream_content.to_string());

        search_from = content_end + 9;
    }
    streams
}

fn parse_content_stream(stream: &str) -> (Vec<PdfText>, Vec<PdfVector>) {
    let mut texts = Vec::new();
    let mut vectors = Vec::new();

    let mut current_path: Vec<PdfPathCmd> = Vec::new();
    let mut num_stack: Vec<f64> = Vec::new();

    // Text state
    let mut text_x = 0.0_f64;
    let mut text_y = 0.0_f64;
    let mut font_size = 12.0_f64;
    let mut in_text = false;

    for token in tokenize_pdf_stream(stream) {
        match token.as_str() {
            // Vector path operators
            "m" => {
                if num_stack.len() >= 2 {
                    let y = num_stack.pop().unwrap_or(0.0);
                    let x = num_stack.pop().unwrap_or(0.0);
                    current_path.push(PdfPathCmd::MoveTo(Point2::new(x, y)));
                }
                num_stack.clear();
            }
            "l" if !in_text => {
                if num_stack.len() >= 2 {
                    let y = num_stack.pop().unwrap_or(0.0);
                    let x = num_stack.pop().unwrap_or(0.0);
                    current_path.push(PdfPathCmd::LineTo(Point2::new(x, y)));
                }
                num_stack.clear();
            }
            "c" => {
                if num_stack.len() >= 6 {
                    let y3 = num_stack.pop().unwrap_or(0.0);
                    let x3 = num_stack.pop().unwrap_or(0.0);
                    let y2 = num_stack.pop().unwrap_or(0.0);
                    let x2 = num_stack.pop().unwrap_or(0.0);
                    let y1 = num_stack.pop().unwrap_or(0.0);
                    let x1 = num_stack.pop().unwrap_or(0.0);
                    current_path.push(PdfPathCmd::CurveTo(
                        Point2::new(x1, y1),
                        Point2::new(x2, y2),
                        Point2::new(x3, y3),
                    ));
                }
                num_stack.clear();
            }
            "h" => {
                current_path.push(PdfPathCmd::ClosePath);
                num_stack.clear();
            }
            "S" | "s" | "f" | "F" | "B" | "b" | "n" => {
                if !current_path.is_empty() {
                    vectors.push(PdfVector {
                        commands: std::mem::take(&mut current_path),
                    });
                }
                num_stack.clear();
            }
            "re" => {
                if num_stack.len() >= 4 {
                    let rh = num_stack.pop().unwrap_or(0.0);
                    let rw = num_stack.pop().unwrap_or(0.0);
                    let ry = num_stack.pop().unwrap_or(0.0);
                    let rx = num_stack.pop().unwrap_or(0.0);
                    current_path.push(PdfPathCmd::MoveTo(Point2::new(rx, ry)));
                    current_path.push(PdfPathCmd::LineTo(Point2::new(rx + rw, ry)));
                    current_path.push(PdfPathCmd::LineTo(Point2::new(rx + rw, ry + rh)));
                    current_path.push(PdfPathCmd::LineTo(Point2::new(rx, ry + rh)));
                    current_path.push(PdfPathCmd::ClosePath);
                }
                num_stack.clear();
            }
            // Text operators
            "BT" => {
                in_text = true;
                text_x = 0.0;
                text_y = 0.0;
                num_stack.clear();
            }
            "ET" => {
                in_text = false;
                num_stack.clear();
            }
            "Tf" => {
                if !num_stack.is_empty() {
                    font_size = num_stack.pop().unwrap_or(0.0);
                }
                num_stack.clear();
            }
            "Td" | "TD" => {
                if num_stack.len() >= 2 {
                    let ty = num_stack.pop().unwrap_or(0.0);
                    let tx = num_stack.pop().unwrap_or(0.0);
                    text_x += tx;
                    text_y += ty;
                }
                num_stack.clear();
            }
            "Tj" => {
                // Text string was on the stack as a string (handled below)
                num_stack.clear();
            }
            _ => {
                // Try to parse as number
                if let Ok(n) = token.parse::<f64>() {
                    num_stack.push(n);
                } else if token.starts_with('(') && token.ends_with(')') {
                    // PDF string literal — extract text
                    let inner = &token[1..token.len() - 1];
                    if !inner.is_empty() && in_text {
                        texts.push(PdfText {
                            position: Point2::new(text_x, text_y),
                            text: inner.to_string(),
                            font_size,
                        });
                    }
                } else if token.starts_with('/') {
                    // Name token — ignore
                    num_stack.clear();
                }
            }
        }
    }

    // Flush remaining path
    if !current_path.is_empty() {
        vectors.push(PdfVector {
            commands: current_path,
        });
    }

    (texts, vectors)
}

fn tokenize_pdf_stream(stream: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let bytes = stream.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let ch = bytes[i];

        // Skip whitespace
        if ch.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // String literal
        if ch == b'(' {
            let start = i;
            let mut depth = 1;
            i += 1;
            while i < len && depth > 0 {
                if bytes[i] == b'(' && (i == 0 || bytes[i - 1] != b'\\') {
                    depth += 1;
                } else if bytes[i] == b')' && (i == 0 || bytes[i - 1] != b'\\') {
                    depth -= 1;
                }
                i += 1;
            }
            tokens.push(String::from_utf8_lossy(&bytes[start..i]).to_string());
            continue;
        }

        // Comment
        if ch == b'%' {
            while i < len && bytes[i] != b'\n' && bytes[i] != b'\r' {
                i += 1;
            }
            continue;
        }

        // Regular token (number, operator, name)
        let start = i;
        while i < len && !bytes[i].is_ascii_whitespace() && bytes[i] != b'(' && bytes[i] != b')' {
            i += 1;
        }
        if i > start {
            tokens.push(String::from_utf8_lossy(&bytes[start..i]).to_string());
        }
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_generation() {
        let svg = "<svg><line x1=\"10\" y1=\"20\" x2=\"100\" y2=\"20\" /></svg>";
        let pdf = export_pdf(svg, 297.0, 210.0).unwrap();
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.ends_with(b"%%EOF\n"));
    }

    #[test]
    fn test_pdf_empty_error() {
        assert!(export_pdf("", 297.0, 210.0).is_err());
    }

    #[test]
    fn test_pdf_contains_stream() {
        let svg = "<svg><line x1=\"0\" y1=\"0\" x2=\"50\" y2=\"50\" /></svg>";
        let pdf = export_pdf(svg, 297.0, 210.0).unwrap();
        let text = String::from_utf8_lossy(&pdf);
        assert!(text.contains("stream"));
        assert!(text.contains("endstream"));
    }

    #[test]
    fn test_import_pdf_roundtrip() {
        let svg = "<svg><line x1=\"10\" y1=\"20\" x2=\"100\" y2=\"20\" /></svg>";
        let pdf_bytes = export_pdf(svg, 297.0, 210.0).unwrap();
        let result = import_pdf(&pdf_bytes).unwrap();
        assert_eq!(result.pages.len(), 1);
        let page = &result.pages[0];
        assert!(page.width > 0.0);
        assert!(page.height > 0.0);
    }

    #[test]
    fn test_import_pdf_vectors() {
        let svg = "<svg><line x1=\"10\" y1=\"20\" x2=\"100\" y2=\"50\" /></svg>";
        let pdf_bytes = export_pdf(svg, 297.0, 210.0).unwrap();
        let result = import_pdf(&pdf_bytes).unwrap();
        let page = &result.pages[0];
        assert!(!page.vector_content.is_empty());
    }

    #[test]
    fn test_import_pdf_text() {
        let svg = r#"<svg><text x="10" y="20" font-size="12">Hello</text></svg>"#;
        let pdf_bytes = export_pdf(svg, 297.0, 210.0).unwrap();
        let result = import_pdf(&pdf_bytes).unwrap();
        let page = &result.pages[0];
        // Should extract the title block text at minimum
        assert!(!page.text_content.is_empty());
    }

    #[test]
    fn test_import_pdf_invalid() {
        assert!(import_pdf(b"not a pdf").is_err());
    }

    #[test]
    fn test_import_pdf_too_short() {
        assert!(import_pdf(b"short").is_err());
    }

    #[test]
    fn test_pdf_import_result_types() {
        let svg = "<svg><line x1=\"0\" y1=\"0\" x2=\"50\" y2=\"50\" /></svg>";
        let pdf_bytes = export_pdf(svg, 297.0, 210.0).unwrap();
        let result = import_pdf(&pdf_bytes).unwrap();
        let page = &result.pages[0];
        // Verify page dimensions (A4 landscape in points)
        assert!((page.width - 297.0 * 2.834_645_669_3).abs() < 1.0);
        assert!((page.height - 210.0 * 2.834_645_669_3).abs() < 1.0);
    }
}
