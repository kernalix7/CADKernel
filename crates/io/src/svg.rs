//! SVG (Scalable Vector Graphics) import/export for 2D geometry.

use std::fmt;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};

use crate::tessellate::Mesh;

/// Style attributes for an SVG element.
#[derive(Debug, Clone)]
pub struct SvgStyle {
    pub stroke: String,
    pub stroke_width: f64,
    pub fill: String,
    /// Optional SVG `stroke-dasharray` attribute (e.g. `"4,2"` for dashed lines).
    pub stroke_dasharray: Option<String>,
}

impl SvgStyle {
    /// Default stroke style.
    pub fn default_stroke() -> Self {
        Self {
            stroke: "black".into(),
            stroke_width: 1.0,
            fill: "none".into(),
            stroke_dasharray: None,
        }
    }
}

impl Default for SvgStyle {
    fn default() -> Self {
        Self::default_stroke()
    }
}

/// A single SVG drawing element.
#[derive(Debug, Clone)]
pub enum SvgElement {
    Line {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        style: SvgStyle,
    },
    Circle {
        cx: f64,
        cy: f64,
        r: f64,
        style: SvgStyle,
    },
    Polyline {
        points: Vec<(f64, f64)>,
        style: SvgStyle,
    },
    /// SVG `<text>` element.
    Text {
        x: f64,
        y: f64,
        text: String,
        font_size: f64,
        /// `text-anchor`: `"start"`, `"middle"`, or `"end"`.
        anchor: String,
        style: SvgStyle,
    },
}

/// An SVG document composed of elements.
#[derive(Debug, Clone)]
pub struct SvgDocument {
    pub width: f64,
    pub height: f64,
    pub elements: Vec<SvgElement>,
}

impl SvgDocument {
    /// Creates a new SVG document with the given dimensions.
    pub fn new(width: f64, height: f64) -> Self {
        Self {
            width,
            height,
            elements: Vec::new(),
        }
    }

    /// Adds an element to the document.
    pub fn add(&mut self, element: SvgElement) {
        self.elements.push(element);
    }

    /// Renders the document to an SVG string.
    pub fn render(&self) -> String {
        self.to_string()
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn write_dasharray(f: &mut fmt::Formatter<'_>, style: &SvgStyle) -> fmt::Result {
    if let Some(ref da) = style.stroke_dasharray {
        write!(f, r#" stroke-dasharray="{da}""#)?;
    }
    Ok(())
}

impl fmt::Display for SvgDocument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">"#,
            self.width, self.height, self.width, self.height
        )?;
        writeln!(f)?;
        for elem in &self.elements {
            match elem {
                SvgElement::Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    style,
                } => {
                    write!(
                        f,
                        r#"  <line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{}" stroke-width="{}" fill="{}""#,
                        xml_escape(&style.stroke),
                        style.stroke_width,
                        xml_escape(&style.fill)
                    )?;
                    write_dasharray(f, style)?;
                    writeln!(f, "/>")?;
                }
                SvgElement::Circle { cx, cy, r, style } => {
                    write!(
                        f,
                        r#"  <circle cx="{cx}" cy="{cy}" r="{r}" stroke="{}" stroke-width="{}" fill="{}""#,
                        xml_escape(&style.stroke),
                        style.stroke_width,
                        xml_escape(&style.fill)
                    )?;
                    write_dasharray(f, style)?;
                    writeln!(f, "/>")?;
                }
                SvgElement::Polyline { points, style } => {
                    let pts: Vec<String> = points.iter().map(|(x, y)| format!("{x},{y}")).collect();
                    write!(
                        f,
                        r#"  <polyline points="{}" stroke="{}" stroke-width="{}" fill="{}""#,
                        pts.join(" "),
                        xml_escape(&style.stroke),
                        style.stroke_width,
                        xml_escape(&style.fill)
                    )?;
                    write_dasharray(f, style)?;
                    writeln!(f, "/>")?;
                }
                SvgElement::Text {
                    x,
                    y,
                    text,
                    font_size,
                    anchor,
                    style,
                } => {
                    writeln!(
                        f,
                        r#"  <text x="{x}" y="{y}" font-size="{font_size}" text-anchor="{}" fill="{}">{}</text>"#,
                        xml_escape(anchor),
                        xml_escape(&style.fill),
                        xml_escape(text),
                    )?;
                }
            }
        }
        writeln!(f, "</svg>")?;
        Ok(())
    }
}

/// Converts a 3D profile (closed polyline projected to XY) to an SVG document.
pub fn profile_to_svg(profile: &[Point3], width: f64, height: f64) -> SvgDocument {
    let mut doc = SvgDocument::new(width, height);
    if profile.len() < 2 {
        return doc;
    }
    let points: Vec<(f64, f64)> = profile.iter().map(|p| (p.x, p.y)).collect();
    let mut closed_points = points;
    if let Some(&first) = closed_points.first() {
        closed_points.push(first);
    }
    doc.add(SvgElement::Polyline {
        points: closed_points,
        style: SvgStyle::default_stroke(),
    });
    doc
}

// ---------------------------------------------------------------------------
// SVG Import
// ---------------------------------------------------------------------------

/// Import SVG content into a flat (z=0) triangulated mesh.
///
/// Parses `<rect>`, `<circle>`, `<ellipse>`, `<line>`, `<polyline>`,
/// `<polygon>`, and `<path>` elements. Closed shapes are triangulated
/// via ear-clipping. The `transform` attribute is supported for
/// translate, rotate, scale, and matrix forms.
pub fn import_svg(content: &str) -> KernelResult<Mesh> {
    if content.is_empty() {
        return Err(KernelError::InvalidArgument("empty SVG content".into()));
    }

    let mut all_polys: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut open_lines: Vec<((f64, f64), (f64, f64))> = Vec::new();

    // Simple tag-based parser: find self-closing or opening tags for shapes.
    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut pos = 0;

    while pos < len {
        if bytes[pos] == b'<' {
            let tag_start = pos;
            // Find end of tag (> or />)
            let mut tag_end = pos + 1;
            while tag_end < len && bytes[tag_end] != b'>' {
                tag_end += 1;
            }
            if tag_end >= len {
                break;
            }
            let tag = &content[tag_start..=tag_end];
            let xf = parse_transform(tag);

            if tag_starts_with(tag, "rect") {
                if let Some(poly) = parse_svg_rect(tag, &xf) {
                    all_polys.push(poly);
                }
            } else if tag_starts_with(tag, "circle") {
                if let Some(poly) = parse_svg_circle_tag(tag, &xf) {
                    all_polys.push(poly);
                }
            } else if tag_starts_with(tag, "ellipse") {
                if let Some(poly) = parse_svg_ellipse_tag(tag, &xf) {
                    all_polys.push(poly);
                }
            } else if tag_starts_with(tag, "line") && !tag_starts_with(tag, "linearGradient") {
                if let Some((a, b)) = parse_svg_line_tag(tag, &xf) {
                    open_lines.push((a, b));
                }
            } else if tag_starts_with(tag, "polygon") {
                if let Some(poly) = parse_svg_points_tag(tag, &xf) {
                    if poly.len() >= 3 {
                        all_polys.push(poly);
                    }
                }
            } else if tag_starts_with(tag, "polyline") {
                if let Some(poly) = parse_svg_points_tag(tag, &xf) {
                    // Polyline is closed if first==last
                    if poly.len() >= 3 {
                        let first = poly[0];
                        let last = poly[poly.len() - 1];
                        if (first.0 - last.0).abs() < 1e-9 && (first.1 - last.1).abs() < 1e-9 {
                            let mut p = poly;
                            p.pop();
                            all_polys.push(p);
                        }
                    }
                }
            } else if tag_starts_with(tag, "path") {
                let polys = parse_svg_path_tag(tag, &xf);
                for poly in polys {
                    if poly.len() >= 3 {
                        all_polys.push(poly);
                    }
                }
            }

            pos = tag_end + 1;
        } else {
            pos += 1;
        }
    }

    // Build mesh: triangulate each closed polygon, add open lines as degenerate tris
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    let up = Vec3::Z;

    for poly in &all_polys {
        let tris = ear_clip(poly);
        for tri in tris {
            let base = vertices.len() as u32;
            vertices.push(Point3::new(tri.0 .0, tri.0 .1, 0.0));
            vertices.push(Point3::new(tri.1 .0, tri.1 .1, 0.0));
            vertices.push(Point3::new(tri.2 .0, tri.2 .1, 0.0));
            normals.push(up);
            normals.push(up);
            normals.push(up);
            indices.push([base, base + 1, base + 2]);
        }
    }

    for (a, b) in &open_lines {
        let base = vertices.len() as u32;
        vertices.push(Point3::new(a.0, a.1, 0.0));
        vertices.push(Point3::new(b.0, b.1, 0.0));
        vertices.push(Point3::new(b.0, b.1, 0.0));
        normals.push(up);
        normals.push(up);
        normals.push(up);
        indices.push([base, base + 1, base + 2]);
    }

    Ok(Mesh {
        vertices,
        normals,
        indices,
    })
}

/// Writes an SVG format string to a file at the given path.
pub fn write_svg(path: &str, content: &str) -> KernelResult<()> {
    std::fs::write(path, content).map_err(|e| KernelError::IoError(e.to_string()))
}

// ---------------------------------------------------------------------------
// Transform parsing
// ---------------------------------------------------------------------------

/// A 2D affine transform: [a b tx; c d ty; 0 0 1].
#[derive(Clone, Copy)]
struct Xf {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    tx: f64,
    ty: f64,
}

impl Xf {
    fn identity() -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            tx: 0.0,
            ty: 0.0,
        }
    }

    fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        (
            self.a * x + self.b * y + self.tx,
            self.c * x + self.d * y + self.ty,
        )
    }

    fn concat(&self, other: &Xf) -> Xf {
        Xf {
            a: self.a * other.a + self.b * other.c,
            b: self.a * other.b + self.b * other.d,
            c: self.c * other.a + self.d * other.c,
            d: self.c * other.b + self.d * other.d,
            tx: self.a * other.tx + self.b * other.ty + self.tx,
            ty: self.c * other.tx + self.d * other.ty + self.ty,
        }
    }
}

fn parse_transform(tag: &str) -> Xf {
    let Some(val) = extract_attr(tag, "transform") else {
        return Xf::identity();
    };
    parse_transform_value(&val)
}

fn parse_transform_value(val: &str) -> Xf {
    let mut result = Xf::identity();
    let mut rest = val;

    while !rest.is_empty() {
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }

        if let Some(inner) = extract_fn(rest, "matrix") {
            let nums = parse_nums(&inner);
            if nums.len() >= 6 {
                let m = Xf {
                    a: nums[0],
                    b: nums[2],
                    c: nums[1],
                    d: nums[3],
                    tx: nums[4],
                    ty: nums[5],
                };
                result = result.concat(&m);
            }
            rest = skip_past_paren(rest);
        } else if let Some(inner) = extract_fn(rest, "translate") {
            let nums = parse_nums(&inner);
            let tx = nums.first().copied().unwrap_or(0.0);
            let ty = nums.get(1).copied().unwrap_or(0.0);
            let m = Xf {
                a: 1.0,
                b: 0.0,
                c: 0.0,
                d: 1.0,
                tx,
                ty,
            };
            result = result.concat(&m);
            rest = skip_past_paren(rest);
        } else if let Some(inner) = extract_fn(rest, "scale") {
            let nums = parse_nums(&inner);
            let sx = nums.first().copied().unwrap_or(1.0);
            let sy = nums.get(1).copied().unwrap_or(sx);
            let m = Xf {
                a: sx,
                b: 0.0,
                c: 0.0,
                d: sy,
                tx: 0.0,
                ty: 0.0,
            };
            result = result.concat(&m);
            rest = skip_past_paren(rest);
        } else if let Some(inner) = extract_fn(rest, "rotate") {
            let nums = parse_nums(&inner);
            let angle = nums.first().copied().unwrap_or(0.0).to_radians();
            let cx = nums.get(1).copied().unwrap_or(0.0);
            let cy = nums.get(2).copied().unwrap_or(0.0);
            let cos_a = angle.cos();
            let sin_a = angle.sin();
            let m = Xf {
                a: cos_a,
                b: -sin_a,
                c: sin_a,
                d: cos_a,
                tx: cx - cos_a * cx + sin_a * cy,
                ty: cy - sin_a * cx - cos_a * cy,
            };
            result = result.concat(&m);
            rest = skip_past_paren(rest);
        } else {
            // Unknown function, skip a char
            rest = &rest[1..];
        }
    }

    result
}

fn extract_fn(s: &str, name: &str) -> Option<String> {
    let trimmed = s.trim_start();
    if !trimmed.starts_with(name) {
        return None;
    }
    let after = &trimmed[name.len()..].trim_start();
    if !after.starts_with('(') {
        return None;
    }
    let start = 1;
    let end = after.find(')')?;
    Some(after[start..end].to_string())
}

fn skip_past_paren(s: &str) -> &str {
    match s.find(')') {
        Some(i) => &s[i + 1..],
        None => "",
    }
}

fn parse_nums(s: &str) -> Vec<f64> {
    let cleaned = s.replace(',', " ");
    cleaned
        .split_whitespace()
        .filter_map(|tok| tok.parse::<f64>().ok())
        .collect()
}

// ---------------------------------------------------------------------------
// Tag helpers
// ---------------------------------------------------------------------------

fn tag_starts_with(tag: &str, name: &str) -> bool {
    let trimmed = tag.trim_start_matches('<');
    trimmed.starts_with(name)
        && trimmed
            .as_bytes()
            .get(name.len())
            .is_none_or(|b| b.is_ascii_whitespace() || *b == b'/' || *b == b'>')
}

fn extract_attr(tag: &str, name: &str) -> Option<String> {
    let key = format!("{name}=\"");
    let start = tag.find(&key)? + key.len();
    let end = start + tag[start..].find('"')?;
    Some(tag[start..end].to_string())
}

fn attr_f64(tag: &str, name: &str) -> Option<f64> {
    extract_attr(tag, name)?.parse().ok()
}

// ---------------------------------------------------------------------------
// Shape parsers
// ---------------------------------------------------------------------------

fn parse_svg_rect(tag: &str, xf: &Xf) -> Option<Vec<(f64, f64)>> {
    let x = attr_f64(tag, "x").unwrap_or(0.0);
    let y = attr_f64(tag, "y").unwrap_or(0.0);
    let w = attr_f64(tag, "width")?;
    let h = attr_f64(tag, "height")?;
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    let corners = [(x, y), (x + w, y), (x + w, y + h), (x, y + h)];
    Some(corners.iter().map(|&(px, py)| xf.apply(px, py)).collect())
}

fn parse_svg_circle_tag(tag: &str, xf: &Xf) -> Option<Vec<(f64, f64)>> {
    let cx = attr_f64(tag, "cx").unwrap_or(0.0);
    let cy = attr_f64(tag, "cy").unwrap_or(0.0);
    let r = attr_f64(tag, "r")?;
    if r <= 0.0 {
        return None;
    }
    Some(circle_poly(cx, cy, r, r, xf))
}

fn parse_svg_ellipse_tag(tag: &str, xf: &Xf) -> Option<Vec<(f64, f64)>> {
    let cx = attr_f64(tag, "cx").unwrap_or(0.0);
    let cy = attr_f64(tag, "cy").unwrap_or(0.0);
    let rx = attr_f64(tag, "rx")?;
    let ry = attr_f64(tag, "ry")?;
    if rx <= 0.0 || ry <= 0.0 {
        return None;
    }
    Some(circle_poly(cx, cy, rx, ry, xf))
}

fn circle_poly(cx: f64, cy: f64, rx: f64, ry: f64, xf: &Xf) -> Vec<(f64, f64)> {
    let n = 32;
    (0..n)
        .map(|i| {
            let angle = 2.0 * std::f64::consts::PI * (i as f64) / (n as f64);
            let px = cx + rx * angle.cos();
            let py = cy + ry * angle.sin();
            xf.apply(px, py)
        })
        .collect()
}

fn parse_svg_line_tag(tag: &str, xf: &Xf) -> Option<((f64, f64), (f64, f64))> {
    let x1 = attr_f64(tag, "x1")?;
    let y1 = attr_f64(tag, "y1")?;
    let x2 = attr_f64(tag, "x2")?;
    let y2 = attr_f64(tag, "y2")?;
    Some((xf.apply(x1, y1), xf.apply(x2, y2)))
}

fn parse_svg_points_tag(tag: &str, xf: &Xf) -> Option<Vec<(f64, f64)>> {
    let val = extract_attr(tag, "points")?;
    let nums = parse_nums(&val);
    if nums.len() < 4 {
        return None;
    }
    Some(
        nums.chunks(2)
            .filter(|c| c.len() == 2)
            .map(|c| xf.apply(c[0], c[1]))
            .collect(),
    )
}

// ---------------------------------------------------------------------------
// Path parser (M/L/C/Z/H/V + lowercase relative variants)
// ---------------------------------------------------------------------------

fn parse_svg_path_tag(tag: &str, xf: &Xf) -> Vec<Vec<(f64, f64)>> {
    let Some(d) = extract_attr(tag, "d") else {
        return Vec::new();
    };
    parse_path_d(&d, xf)
}

fn parse_path_d(d: &str, xf: &Xf) -> Vec<Vec<(f64, f64)>> {
    let tokens = tokenize_path(d);
    let mut polys = Vec::new();
    let mut current: Vec<(f64, f64)> = Vec::new();
    let mut cx = 0.0_f64;
    let mut cy = 0.0_f64;
    let mut start_x = 0.0_f64;
    let mut start_y = 0.0_f64;
    let mut i = 0;

    while i < tokens.len() {
        match tokens[i].as_str() {
            "M" => {
                close_subpath(&mut current, &mut polys);
                if i + 2 < tokens.len() {
                    cx = tokens[i + 1].parse().unwrap_or(0.0);
                    cy = tokens[i + 2].parse().unwrap_or(0.0);
                    start_x = cx;
                    start_y = cy;
                    current.push(xf.apply(cx, cy));
                    i += 3;
                    // Subsequent coordinate pairs are implicit LineTo
                    while i + 1 < tokens.len() && is_number(&tokens[i]) {
                        cx = tokens[i].parse().unwrap_or(0.0);
                        cy = tokens[i + 1].parse().unwrap_or(0.0);
                        current.push(xf.apply(cx, cy));
                        i += 2;
                    }
                } else {
                    i += 1;
                }
            }
            "m" => {
                close_subpath(&mut current, &mut polys);
                if i + 2 < tokens.len() {
                    cx += tokens[i + 1].parse::<f64>().unwrap_or(0.0);
                    cy += tokens[i + 2].parse::<f64>().unwrap_or(0.0);
                    start_x = cx;
                    start_y = cy;
                    current.push(xf.apply(cx, cy));
                    i += 3;
                    while i + 1 < tokens.len() && is_number(&tokens[i]) {
                        cx += tokens[i].parse::<f64>().unwrap_or(0.0);
                        cy += tokens[i + 1].parse::<f64>().unwrap_or(0.0);
                        current.push(xf.apply(cx, cy));
                        i += 2;
                    }
                } else {
                    i += 1;
                }
            }
            "L" => {
                i += 1;
                while i + 1 < tokens.len() && is_number(&tokens[i]) {
                    cx = tokens[i].parse().unwrap_or(0.0);
                    cy = tokens[i + 1].parse().unwrap_or(0.0);
                    current.push(xf.apply(cx, cy));
                    i += 2;
                }
            }
            "l" => {
                i += 1;
                while i + 1 < tokens.len() && is_number(&tokens[i]) {
                    cx += tokens[i].parse::<f64>().unwrap_or(0.0);
                    cy += tokens[i + 1].parse::<f64>().unwrap_or(0.0);
                    current.push(xf.apply(cx, cy));
                    i += 2;
                }
            }
            "H" => {
                i += 1;
                while i < tokens.len() && is_number(&tokens[i]) {
                    cx = tokens[i].parse().unwrap_or(cx);
                    current.push(xf.apply(cx, cy));
                    i += 1;
                }
            }
            "h" => {
                i += 1;
                while i < tokens.len() && is_number(&tokens[i]) {
                    cx += tokens[i].parse::<f64>().unwrap_or(0.0);
                    current.push(xf.apply(cx, cy));
                    i += 1;
                }
            }
            "V" => {
                i += 1;
                while i < tokens.len() && is_number(&tokens[i]) {
                    cy = tokens[i].parse().unwrap_or(cy);
                    current.push(xf.apply(cx, cy));
                    i += 1;
                }
            }
            "v" => {
                i += 1;
                while i < tokens.len() && is_number(&tokens[i]) {
                    cy += tokens[i].parse::<f64>().unwrap_or(0.0);
                    current.push(xf.apply(cx, cy));
                    i += 1;
                }
            }
            "C" => {
                i += 1;
                while i + 5 < tokens.len() && is_number(&tokens[i]) {
                    let x1: f64 = tokens[i].parse().unwrap_or(0.0);
                    let y1: f64 = tokens[i + 1].parse().unwrap_or(0.0);
                    let x2: f64 = tokens[i + 2].parse().unwrap_or(0.0);
                    let y2: f64 = tokens[i + 3].parse().unwrap_or(0.0);
                    let x3: f64 = tokens[i + 4].parse().unwrap_or(0.0);
                    let y3: f64 = tokens[i + 5].parse().unwrap_or(0.0);
                    flatten_cubic(&mut current, [(cx, cy), (x1, y1), (x2, y2), (x3, y3)], xf);
                    cx = x3;
                    cy = y3;
                    i += 6;
                }
            }
            "c" => {
                i += 1;
                while i + 5 < tokens.len() && is_number(&tokens[i]) {
                    let dx1: f64 = tokens[i].parse().unwrap_or(0.0);
                    let dy1: f64 = tokens[i + 1].parse().unwrap_or(0.0);
                    let dx2: f64 = tokens[i + 2].parse().unwrap_or(0.0);
                    let dy2: f64 = tokens[i + 3].parse().unwrap_or(0.0);
                    let dx3: f64 = tokens[i + 4].parse().unwrap_or(0.0);
                    let dy3: f64 = tokens[i + 5].parse().unwrap_or(0.0);
                    flatten_cubic(
                        &mut current,
                        [(cx, cy), (cx + dx1, cy + dy1), (cx + dx2, cy + dy2), (cx + dx3, cy + dy3)],
                        xf,
                    );
                    cx += dx3;
                    cy += dy3;
                    i += 6;
                }
            }
            "Z" | "z" => {
                cx = start_x;
                cy = start_y;
                // Close the subpath
                if current.len() >= 3 {
                    polys.push(std::mem::take(&mut current));
                } else {
                    current.clear();
                }
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }
    close_subpath(&mut current, &mut polys);
    polys
}

fn close_subpath(current: &mut Vec<(f64, f64)>, polys: &mut Vec<Vec<(f64, f64)>>) {
    if current.len() >= 3 {
        // Check if implicitly closed
        let first = current[0];
        let last = current[current.len() - 1];
        if (first.0 - last.0).abs() < 1e-9 && (first.1 - last.1).abs() < 1e-9 {
            current.pop();
            if current.len() >= 3 {
                polys.push(std::mem::take(current));
            } else {
                current.clear();
            }
            return;
        }
    }
    current.clear();
}

fn tokenize_path(d: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    for ch in d.chars() {
        if ch.is_ascii_alphabetic() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            tokens.push(ch.to_string());
        } else if ch == ',' || ch.is_ascii_whitespace() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
        } else if ch == '-' && !current.is_empty() && !current.ends_with('e') && !current.ends_with('E') {
            tokens.push(std::mem::take(&mut current));
            current.push(ch);
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn is_number(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_digit() || c == '-' || c == '.')
}

fn flatten_cubic(
    out: &mut Vec<(f64, f64)>,
    pts: [(f64, f64); 4],
    xf: &Xf,
) {
    let steps = 8;
    for i in 1..=steps {
        let t = i as f64 / steps as f64;
        let u = 1.0 - t;
        let px = u * u * u * pts[0].0 + 3.0 * u * u * t * pts[1].0 + 3.0 * u * t * t * pts[2].0 + t * t * t * pts[3].0;
        let py = u * u * u * pts[0].1 + 3.0 * u * u * t * pts[1].1 + 3.0 * u * t * t * pts[2].1 + t * t * t * pts[3].1;
        out.push(xf.apply(px, py));
    }
}

// ---------------------------------------------------------------------------
// Ear-clipping triangulation for simple polygons
// ---------------------------------------------------------------------------

type Tri2d = ((f64, f64), (f64, f64), (f64, f64));

fn ear_clip(poly: &[(f64, f64)]) -> Vec<Tri2d> {
    let n = poly.len();
    if n < 3 {
        return Vec::new();
    }
    if n == 3 {
        return vec![(poly[0], poly[1], poly[2])];
    }

    let mut result = Vec::new();
    let mut idx: Vec<usize> = (0..n).collect();

    // Ensure CCW winding
    let area2 = signed_area_2(poly);
    if area2 < 0.0 {
        idx.reverse();
    }

    let mut attempts = 0;
    let max_attempts = idx.len() * idx.len();

    while idx.len() > 3 && attempts < max_attempts {
        let m = idx.len();
        let mut found = false;
        for i in 0..m {
            let prev = idx[(i + m - 1) % m];
            let curr = idx[i];
            let next = idx[(i + 1) % m];

            let a = poly[prev];
            let b = poly[curr];
            let c = poly[next];

            // Convex vertex?
            if cross_2d(a, b, c) <= 0.0 {
                continue;
            }

            // Any other vertex inside this triangle?
            let mut ear = true;
            for &k in &idx[..m] {
                if k == prev || k == curr || k == next {
                    continue;
                }
                if point_in_tri(poly[k], a, b, c) {
                    ear = false;
                    break;
                }
            }

            if ear {
                result.push((a, b, c));
                idx.remove(i);
                found = true;
                break;
            }
        }
        if !found {
            attempts += 1;
            // Rotate to try different starting vertex
            if !idx.is_empty() {
                let first = idx.remove(0);
                idx.push(first);
            }
        }
    }

    if idx.len() == 3 {
        result.push((poly[idx[0]], poly[idx[1]], poly[idx[2]]));
    }

    result
}

fn signed_area_2(poly: &[(f64, f64)]) -> f64 {
    let n = poly.len();
    let mut sum = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        sum += poly[i].0 * poly[j].1 - poly[j].0 * poly[i].1;
    }
    sum
}

fn cross_2d(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

fn point_in_tri(p: (f64, f64), a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> bool {
    let d1 = cross_2d(a, b, p);
    let d2 = cross_2d(b, c, p);
    let d3 = cross_2d(c, a, p);
    let has_neg = (d1 < 0.0) || (d2 < 0.0) || (d3 < 0.0);
    let has_pos = (d1 > 0.0) || (d2 > 0.0) || (d3 > 0.0);
    !(has_neg && has_pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_svg_rect() {
        let svg = r#"<svg width="100" height="100"><rect x="10" y="10" width="80" height="60"/></svg>"#;
        let mesh = import_svg(svg).unwrap();
        assert!(mesh.triangle_count() >= 2);
        for v in &mesh.vertices {
            assert!((v.z).abs() < 1e-10);
        }
    }

    #[test]
    fn test_import_svg_circle() {
        let svg = r#"<svg width="100" height="100"><circle cx="50" cy="50" r="30"/></svg>"#;
        let mesh = import_svg(svg).unwrap();
        assert!(mesh.triangle_count() >= 10);
    }

    #[test]
    fn test_import_svg_ellipse() {
        let svg = r#"<svg width="100" height="100"><ellipse cx="50" cy="50" rx="40" ry="20"/></svg>"#;
        let mesh = import_svg(svg).unwrap();
        assert!(mesh.triangle_count() >= 10);
    }

    #[test]
    fn test_import_svg_polygon() {
        let svg = r#"<svg width="100" height="100"><polygon points="50,0 100,100 0,100"/></svg>"#;
        let mesh = import_svg(svg).unwrap();
        assert_eq!(mesh.triangle_count(), 1);
    }

    #[test]
    fn test_import_svg_line() {
        let svg = r#"<svg width="100" height="100"><line x1="0" y1="0" x2="100" y2="100"/></svg>"#;
        let mesh = import_svg(svg).unwrap();
        assert_eq!(mesh.triangle_count(), 1);
    }

    #[test]
    fn test_import_svg_path_rect() {
        let svg = r#"<svg width="100" height="100"><path d="M10 10 L90 10 L90 90 L10 90 Z"/></svg>"#;
        let mesh = import_svg(svg).unwrap();
        assert!(mesh.triangle_count() >= 2);
    }

    #[test]
    fn test_import_svg_path_cubic() {
        let svg = r#"<svg width="200" height="200"><path d="M10 80 C40 10, 65 10, 95 80 L95 150 L10 150 Z"/></svg>"#;
        let mesh = import_svg(svg).unwrap();
        assert!(mesh.triangle_count() >= 2);
    }

    #[test]
    fn test_import_svg_with_transform() {
        let svg = r#"<svg width="200" height="200"><rect x="0" y="0" width="50" height="50" transform="translate(10,20)"/></svg>"#;
        let mesh = import_svg(svg).unwrap();
        assert!(mesh.triangle_count() >= 2);
        // Check that translation was applied
        let min_x = mesh.vertices.iter().map(|v| v.x).fold(f64::INFINITY, f64::min);
        assert!(min_x >= 9.0);
    }

    #[test]
    fn test_import_svg_empty() {
        assert!(import_svg("").is_err());
    }

    #[test]
    fn test_import_svg_path_hv() {
        let svg = r#"<svg width="100" height="100"><path d="M0 0 H100 V100 H0 Z"/></svg>"#;
        let mesh = import_svg(svg).unwrap();
        assert!(mesh.triangle_count() >= 2);
    }

    #[test]
    fn test_import_svg_relative_path() {
        let svg = r#"<svg width="100" height="100"><path d="m10 10 l80 0 l0 80 l-80 0 z"/></svg>"#;
        let mesh = import_svg(svg).unwrap();
        assert!(mesh.triangle_count() >= 2);
    }

    #[test]
    fn test_ear_clip_triangle() {
        let poly = vec![(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)];
        let tris = ear_clip(&poly);
        assert_eq!(tris.len(), 1);
    }

    #[test]
    fn test_ear_clip_square() {
        let poly = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        let tris = ear_clip(&poly);
        assert_eq!(tris.len(), 2);
    }

    #[test]
    fn test_transform_scale() {
        let xf = parse_transform_value("scale(2,3)");
        let (x, y) = xf.apply(1.0, 1.0);
        assert!((x - 2.0).abs() < 1e-10);
        assert!((y - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_transform_rotate() {
        let xf = parse_transform_value("rotate(90)");
        let (x, y) = xf.apply(1.0, 0.0);
        assert!(x.abs() < 1e-10);
        assert!((y - 1.0).abs() < 1e-10);
    }

    // -----------------------------------------------------------------------
    // Edge case: Unicode / special characters in SVG content
    // -----------------------------------------------------------------------

    #[test]
    fn test_svg_xml_escape_entities() {
        let escaped = xml_escape("hello & world < > \" '");
        assert!(escaped.contains("&amp;"));
        assert!(escaped.contains("&lt;"));
        assert!(escaped.contains("&gt;"));
        assert!(escaped.contains("&quot;"));
        assert!(escaped.contains("&apos;"));
        assert!(!escaped.contains("& "));
    }

    #[test]
    fn test_svg_import_unicode_content() {
        let svg = r#"<svg width="100" height="100"><rect x="10" y="10" width="80" height="60" id="日本語"/></svg>"#;
        let mesh = import_svg(svg).unwrap();
        assert!(mesh.triangle_count() >= 2);
    }

    #[test]
    fn test_svg_import_special_chars_in_style() {
        let svg = r#"<svg width="100" height="100"><rect x="10" y="10" width="80" height="60" style="fill:red;stroke:blue"/></svg>"#;
        let mesh = import_svg(svg).unwrap();
        assert!(mesh.triangle_count() >= 2);
    }

    // -----------------------------------------------------------------------
    // Edge case: malformed SVG
    // -----------------------------------------------------------------------

    #[test]
    fn test_svg_import_not_xml() {
        let result = import_svg("just plain text, not SVG at all");
        // Non-SVG input may either error or produce empty mesh
        if let Ok(mesh) = result {
            assert_eq!(mesh.triangle_count(), 0);
        }
    }

    #[test]
    fn test_svg_import_no_shapes() {
        let svg = r#"<svg width="100" height="100"></svg>"#;
        let result = import_svg(svg);
        // Empty SVG should either succeed with 0 triangles or error
        if let Ok(mesh) = result {
            assert_eq!(mesh.triangle_count(), 0);
        }
    }
}
