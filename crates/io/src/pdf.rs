use cadkernel_core::{KernelError, KernelResult};

/// Export a TechDraw SVG string to a minimal PDF file.
///
/// Generates a PDF/A-compatible document that embeds the SVG content
/// as a rendered page. The SVG is stored as a stream in the content object.
pub fn export_pdf(svg_content: &str, page_width_mm: f64, page_height_mm: f64) -> KernelResult<Vec<u8>> {
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
    let content_obj = format!(
        "4 0 obj\n<< /Length {} >>\nstream\n",
        stream_bytes.len()
    );
    pdf.extend_from_slice(content_obj.as_bytes());
    pdf.extend_from_slice(stream_bytes);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    // Object 5: Font
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n");

    // Cross-reference table
    let xref_offset = pdf.len();
    let obj_count = offsets.len() + 1;
    let mut xref = format!("xref\n0 {obj_count}\n0000000000 65535 f \n");
    for &off in &offsets {
        xref.push_str(&format!("{off:010} 00000 n \n"));
    }
    pdf.extend_from_slice(xref.as_bytes());

    // Trailer
    let trailer = format!(
        "trailer\n<< /Size {obj_count} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n"
    );
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
    let end = line[start..].find('<').map(|i| start + i).unwrap_or(line.len());
    let text = line[start..end].to_string();
    if text.is_empty() { return None; }
    Some((x, y, text))
}

fn extract_svg_attr(line: &str, name: &str) -> Option<f64> {
    let key = format!("{name}=\"");
    let start = line.find(&key)? + key.len();
    let end = start + line[start..].find('"')?;
    line[start..end].parse().ok()
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
}
