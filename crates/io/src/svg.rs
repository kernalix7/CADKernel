//! SVG (Scalable Vector Graphics) output for 2D sketch profiles.

use std::fmt;

use cadkernel_math::Point3;

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
