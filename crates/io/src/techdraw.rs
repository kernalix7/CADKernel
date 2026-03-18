//! Technical drawing generation: orthographic projection, hidden-line removal,
//! dimension annotations, and SVG output.

#[allow(unused_imports)]
use cadkernel_math::{Point2, Point3, Vec3};
use cadkernel_topology::{BRepModel, Handle, SolidData};

use crate::svg::{SvgDocument, SvgElement, SvgStyle};
use crate::tessellate::tessellate_solid;

// ---------------------------------------------------------------------------
// Projection direction
// ---------------------------------------------------------------------------

/// Standard orthographic projection directions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionDir {
    Front,
    Back,
    Top,
    Bottom,
    Right,
    Left,
    Isometric,
}

impl ProjectionDir {
    /// Returns `(screen_right, screen_up, toward_camera)` basis vectors.
    ///
    /// * `screen_right` maps to +X on screen.
    /// * `screen_up` maps to +Y on screen.
    /// * `toward_camera` points from the scene toward the camera;
    ///   `dot(P, toward_camera)` gives depth (larger = closer).
    fn axes(self) -> (Vec3, Vec3, Vec3) {
        match self {
            Self::Front => (Vec3::X, Vec3::Z, Vec3::Y),
            Self::Back => (Vec3::new(-1.0, 0.0, 0.0), Vec3::Z, Vec3::new(0.0, -1.0, 0.0)),
            Self::Top => (Vec3::X, Vec3::new(0.0, -1.0, 0.0), Vec3::Z),
            Self::Bottom => (Vec3::X, Vec3::Y, Vec3::new(0.0, 0.0, -1.0)),
            Self::Right => (Vec3::new(0.0, -1.0, 0.0), Vec3::Z, Vec3::X),
            Self::Left => (Vec3::Y, Vec3::Z, Vec3::new(-1.0, 0.0, 0.0)),
            Self::Isometric => {
                let inv3 = 1.0 / 3.0_f64.sqrt();
                let inv2 = std::f64::consts::FRAC_1_SQRT_2;
                let inv6 = 1.0 / 6.0_f64.sqrt();
                (
                    Vec3::new(inv2, -inv2, 0.0),
                    Vec3::new(-inv6, -inv6, 2.0 * inv6),
                    Vec3::new(inv3, inv3, inv3),
                )
            }
        }
    }

    /// Human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Front => "Front",
            Self::Back => "Back",
            Self::Top => "Top",
            Self::Bottom => "Bottom",
            Self::Right => "Right",
            Self::Left => "Left",
            Self::Isometric => "Isometric",
        }
    }
}

// ---------------------------------------------------------------------------
// Projected geometry
// ---------------------------------------------------------------------------

/// A projected 2D edge segment.
#[derive(Debug, Clone)]
pub struct ProjectedEdge {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub visible: bool,
}

/// A 2D triangle used for hidden-line depth testing.
struct ProjectedTriangle {
    sx: [f64; 3],
    sy: [f64; 3],
    depth: [f64; 3],
}

// ---------------------------------------------------------------------------
// Drawing structures
// ---------------------------------------------------------------------------

/// A single drawing view.
#[derive(Debug, Clone)]
pub struct DrawingView {
    pub direction: ProjectionDir,
    pub edges: Vec<ProjectedEdge>,
    pub center_x: f64,
    pub center_y: f64,
    pub scale: f64,
}

/// Dimension annotation on a drawing.
#[derive(Debug, Clone)]
pub enum Dimension {
    /// Linear dimension between two points.
    Linear {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        offset: f64,
        text: String,
    },
    /// Radius dimension.
    Radius {
        cx: f64,
        cy: f64,
        r: f64,
        angle_deg: f64,
        text: String,
    },
}

/// Extended dimension annotation types for technical drawings.
#[derive(Debug, Clone)]
pub enum DimensionType {
    /// Linear dimension between two points.
    Length {
        start: Point2,
        end: Point2,
        value: f64,
    },
    /// Horizontal dimension (projected onto X axis).
    HorizontalDimension {
        start: Point2,
        end: Point2,
        value: f64,
    },
    /// Vertical dimension (projected onto Y axis).
    VerticalDimension {
        start: Point2,
        end: Point2,
        value: f64,
    },
    /// Radius dimension annotation.
    RadiusDimension { center: Point2, radius: f64 },
    /// Diameter dimension annotation.
    DiameterDimension { center: Point2, diameter: f64 },
    /// Angle dimension between two arms from a vertex.
    AngleDimension {
        vertex: Point2,
        arm1_end: Point2,
        arm2_end: Point2,
        angle: f64,
    },
}

/// Render a dimension annotation to SVG string.
pub fn dimension_to_svg(dim: &DimensionType) -> String {
    use std::fmt::Write;
    let mut svg = String::new();
    let style = "stroke=\"blue\" stroke-width=\"0.3\" fill=\"none\"";
    let text_style = "fill=\"blue\" font-size=\"5\" text-anchor=\"middle\"";

    match dim {
        DimensionType::Length { start, end, value } => {
            let dx = end.x - start.x;
            let dy = end.y - start.y;
            let len = (dx * dx + dy * dy).sqrt();
            let offset = 10.0;
            let nx = if len > 1e-10 { -dy / len } else { 0.0 };
            let ny = if len > 1e-10 { dx / len } else { 1.0 };
            let ex1 = start.x + nx * offset;
            let ey1 = start.y + ny * offset;
            let ex2 = end.x + nx * offset;
            let ey2 = end.y + ny * offset;
            let _ = write!(svg, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />", start.x, start.y, ex1, ey1, style);
            let _ = write!(svg, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />", end.x, end.y, ex2, ey2, style);
            let _ = write!(svg, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />", ex1, ey1, ex2, ey2, style);
            let mx = (ex1 + ex2) / 2.0;
            let my = (ey1 + ey2) / 2.0 - 1.0;
            let _ = write!(svg, "<text x=\"{}\" y=\"{}\" {}>{:.2}</text>", mx, my, text_style, value);
        }
        DimensionType::HorizontalDimension { start, end, value } => {
            let offset = 10.0;
            let y_line = start.y.min(end.y) - offset;
            let _ = write!(svg, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />", start.x, start.y, start.x, y_line, style);
            let _ = write!(svg, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />", end.x, end.y, end.x, y_line, style);
            let _ = write!(svg, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />", start.x, y_line, end.x, y_line, style);
            let mx = (start.x + end.x) / 2.0;
            let _ = write!(svg, "<text x=\"{}\" y=\"{}\" {}>{:.2}</text>", mx, y_line - 1.0, text_style, value);
        }
        DimensionType::VerticalDimension { start, end, value } => {
            let offset = 10.0;
            let x_line = start.x.max(end.x) + offset;
            let _ = write!(svg, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />", start.x, start.y, x_line, start.y, style);
            let _ = write!(svg, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />", end.x, end.y, x_line, end.y, style);
            let _ = write!(svg, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />", x_line, start.y, x_line, end.y, style);
            let my = (start.y + end.y) / 2.0;
            let _ = write!(svg, "<text x=\"{}\" y=\"{}\" {}>{:.2}</text>", x_line + 3.0, my, text_style, value);
        }
        DimensionType::RadiusDimension { center, radius } => {
            let ex = center.x + radius;
            let ey = center.y;
            let _ = write!(svg, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />", center.x, center.y, ex, ey, style);
            let mx = (center.x + ex) / 2.0;
            let _ = write!(svg, "<text x=\"{}\" y=\"{}\" {}>R{:.2}</text>", mx, center.y - 1.0, text_style, radius);
        }
        DimensionType::DiameterDimension { center, diameter } => {
            let r = diameter / 2.0;
            let x1 = center.x - r;
            let x2 = center.x + r;
            let _ = write!(svg, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />", x1, center.y, x2, center.y, style);
            let _ = write!(svg, "<text x=\"{}\" y=\"{}\" {}>\u{2300}{:.2}</text>", center.x, center.y - 1.0, text_style, diameter);
        }
        DimensionType::AngleDimension {
            vertex,
            arm1_end,
            arm2_end,
            angle,
        } => {
            let _ = write!(svg, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />", vertex.x, vertex.y, arm1_end.x, arm1_end.y, style);
            let _ = write!(svg, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />", vertex.x, vertex.y, arm2_end.x, arm2_end.y, style);
            let mx = (vertex.x + arm1_end.x + arm2_end.x) / 3.0;
            let my = (vertex.y + arm1_end.y + arm2_end.y) / 3.0;
            let _ = write!(svg, "<text x=\"{}\" y=\"{}\" {}>{:.1}\u{00B0}</text>", mx, my, text_style, angle);
        }
    }
    svg
}

// ---------------------------------------------------------------------------
// Annotation types
// ---------------------------------------------------------------------------

/// Text annotation at a position.
#[derive(Debug, Clone)]
pub struct TextAnnotation {
    pub position: Point2,
    pub text: String,
    pub font_size: f64,
}

/// Leader line (arrow pointing to a feature with text label).
#[derive(Debug, Clone)]
pub struct LeaderLine {
    pub start: Point2,
    pub end: Point2,
    pub text: String,
}

/// Hatching pattern for a cross-section region.
#[derive(Debug, Clone)]
pub struct HatchPattern {
    pub boundary: Vec<Point2>,
    pub angle: f64,
    pub spacing: f64,
}

/// Center mark (crosshair at center of circle/arc).
#[derive(Debug, Clone)]
pub struct CenterMark {
    pub center: Point2,
    pub size: f64,
}

/// Surface finish symbol.
#[derive(Debug, Clone)]
pub struct SurfaceFinishSymbol {
    pub position: Point2,
    pub roughness: f64,
}

/// Renders a text annotation to an SVG string.
pub fn text_annotation_to_svg(ann: &TextAnnotation) -> String {
    use std::fmt::Write;
    let mut svg = String::new();
    let _ = write!(
        svg,
        "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"black\">{}</text>",
        ann.position.x, ann.position.y, ann.font_size, ann.text
    );
    svg
}

/// Renders a leader line to an SVG string.
pub fn leader_line_to_svg(ll: &LeaderLine) -> String {
    use std::fmt::Write;
    let mut svg = String::new();
    let dx = ll.end.x - ll.start.x;
    let dy = ll.end.y - ll.start.y;
    let len = (dx * dx + dy * dy).sqrt();
    let (ux, uy) = if len > 1e-10 {
        (dx / len, dy / len)
    } else {
        (1.0, 0.0)
    };
    // Arrowhead size
    let a = 3.0;
    let px = -uy;
    let py = ux;
    let ax1 = ll.start.x + ux * a + px * a * 0.3;
    let ay1 = ll.start.y + uy * a + py * a * 0.3;
    let ax2 = ll.start.x + ux * a - px * a * 0.3;
    let ay2 = ll.start.y + uy * a - py * a * 0.3;
    let _ = write!(
        svg,
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"black\" stroke-width=\"0.5\" />",
        ll.start.x, ll.start.y, ll.end.x, ll.end.y
    );
    let _ = write!(
        svg,
        "<polygon points=\"{},{} {},{} {},{}\" fill=\"black\" />",
        ll.start.x, ll.start.y, ax1, ay1, ax2, ay2
    );
    let _ = write!(
        svg,
        "<text x=\"{}\" y=\"{}\" font-size=\"5\" fill=\"black\">{}</text>",
        ll.end.x + 2.0,
        ll.end.y,
        ll.text
    );
    svg
}

/// Renders a hatch pattern to an SVG string.
pub fn hatch_pattern_to_svg(hp: &HatchPattern) -> String {
    use std::fmt::Write;
    let mut svg = String::new();
    if hp.boundary.len() < 3 || hp.spacing <= 0.0 {
        return svg;
    }
    // Compute bounding box of boundary
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    for p in &hp.boundary {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }
    let diag = ((max_x - min_x).powi(2) + (max_y - min_y).powi(2)).sqrt();
    let cx = (min_x + max_x) / 2.0;
    let cy = (min_y + max_y) / 2.0;
    let angle_rad = hp.angle.to_radians();
    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();
    let n_lines = (diag / hp.spacing).ceil() as i64 + 1;
    for i in (-n_lines)..=n_lines {
        let d = i as f64 * hp.spacing;
        let ox = cx + d * (-sin_a);
        let oy = cy + d * cos_a;
        let x1 = ox - diag * cos_a;
        let y1 = oy - diag * sin_a;
        let x2 = ox + diag * cos_a;
        let y2 = oy + diag * sin_a;
        // Clip line segment against polygon boundary edges to find intersections
        let clipped = clip_line_to_polygon(x1, y1, x2, y2, &hp.boundary);
        if let Some((cx1, cy1, cx2, cy2)) = clipped {
            let _ = write!(
                svg,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"black\" stroke-width=\"0.2\" />",
                cx1, cy1, cx2, cy2
            );
        }
    }
    svg
}

/// Clips a line segment to a convex polygon, returning the clipped segment if it intersects.
fn clip_line_to_polygon(
    x1: f64, y1: f64, x2: f64, y2: f64,
    polygon: &[Point2],
) -> Option<(f64, f64, f64, f64)> {
    // Collect all parameter t values where the line intersects polygon edges
    let dx = x2 - x1;
    let dy = y2 - y1;
    let n = polygon.len();
    let mut intersections = Vec::new();
    for i in 0..n {
        let j = (i + 1) % n;
        let ex = polygon[j].x - polygon[i].x;
        let ey = polygon[j].y - polygon[i].y;
        let denom = dx * ey - dy * ex;
        if denom.abs() < 1e-12 {
            continue;
        }
        let t = ((polygon[i].x - x1) * ey - (polygon[i].y - y1) * ex) / denom;
        let s = ((polygon[i].x - x1) * dy - (polygon[i].y - y1) * dx) / denom;
        if (0.0..=1.0).contains(&s) && (0.0..=1.0).contains(&t) {
            intersections.push(t);
        }
    }
    if intersections.len() < 2 {
        // Check if midpoint is inside (line fully within polygon)
        let mx = (x1 + x2) / 2.0;
        let my = (y1 + y2) / 2.0;
        if point_in_polygon_2d(mx, my, polygon) {
            return Some((x1, y1, x2, y2));
        }
        return None;
    }
    intersections.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let t_min = intersections[0];
    let t_max = intersections[intersections.len() - 1];
    Some((
        x1 + t_min * dx,
        y1 + t_min * dy,
        x1 + t_max * dx,
        y1 + t_max * dy,
    ))
}

/// Simple 2D point-in-polygon (crossing number).
fn point_in_polygon_2d(px: f64, py: f64, polygon: &[Point2]) -> bool {
    let n = polygon.len();
    if n < 3 {
        return false;
    }
    let mut crossings = 0;
    for i in 0..n {
        let j = (i + 1) % n;
        let yi = polygon[i].y;
        let yj = polygon[j].y;
        if (yi <= py && yj > py) || (yj <= py && yi > py) {
            let t = (py - yi) / (yj - yi);
            let xi = polygon[i].x + t * (polygon[j].x - polygon[i].x);
            if px < xi {
                crossings += 1;
            }
        }
    }
    crossings % 2 == 1
}

/// Renders a center mark to an SVG string.
pub fn center_mark_to_svg(cm: &CenterMark) -> String {
    use std::fmt::Write;
    let mut svg = String::new();
    let half = cm.size / 2.0;
    let _ = write!(
        svg,
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"red\" stroke-width=\"0.3\" />",
        cm.center.x - half,
        cm.center.y,
        cm.center.x + half,
        cm.center.y
    );
    let _ = write!(
        svg,
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"red\" stroke-width=\"0.3\" />",
        cm.center.x,
        cm.center.y - half,
        cm.center.x,
        cm.center.y + half
    );
    svg
}

/// Renders a surface finish symbol to an SVG string.
pub fn surface_finish_to_svg(sf: &SurfaceFinishSymbol) -> String {
    use std::fmt::Write;
    let mut svg = String::new();
    let s = 5.0;
    // V shape
    let _ = write!(
        svg,
        "<polyline points=\"{},{} {},{} {},{}\" fill=\"none\" stroke=\"black\" stroke-width=\"0.4\" />",
        sf.position.x - s * 0.5,
        sf.position.y - s,
        sf.position.x,
        sf.position.y,
        sf.position.x + s * 0.5,
        sf.position.y - s
    );
    // Roughness text
    let _ = write!(
        svg,
        "<text x=\"{}\" y=\"{}\" font-size=\"4\" fill=\"black\">Ra {:.1}</text>",
        sf.position.x + s * 0.6,
        sf.position.y - s * 0.5,
        sf.roughness
    );
    svg
}

/// A complete technical drawing sheet.
#[derive(Debug, Clone)]
pub struct DrawingSheet {
    pub width: f64,
    pub height: f64,
    pub views: Vec<DrawingView>,
    pub dimensions: Vec<Dimension>,
    pub title: String,
}

impl DrawingSheet {
    /// Creates an A4 landscape drawing sheet (297 x 210 mm).
    pub fn a4_landscape() -> Self {
        Self {
            width: 297.0,
            height: 210.0,
            views: Vec::new(),
            dimensions: Vec::new(),
            title: String::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Projection helpers
// ---------------------------------------------------------------------------

fn dot3(a: Vec3, b: Vec3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

/// Projects a 3D point → (screen_x, screen_y, depth).
fn project_point(p: Point3, right: Vec3, up: Vec3, toward_cam: Vec3) -> (f64, f64, f64) {
    let v = Vec3::new(p.x, p.y, p.z);
    (dot3(v, right), dot3(v, up), dot3(v, toward_cam))
}

/// Canonical edge key for deduplication (sorted by vertex bits).
fn edge_key(a: Point3, b: Point3) -> [u64; 6] {
    let ka = [a.x.to_bits(), a.y.to_bits(), a.z.to_bits()];
    let kb = [b.x.to_bits(), b.y.to_bits(), b.z.to_bits()];
    if ka <= kb {
        [ka[0], ka[1], ka[2], kb[0], kb[1], kb[2]]
    } else {
        [kb[0], kb[1], kb[2], ka[0], ka[1], ka[2]]
    }
}

// ---------------------------------------------------------------------------
// Hidden-line removal
// ---------------------------------------------------------------------------

/// Barycentric point-in-triangle test. Returns interpolated depth if inside.
fn point_in_triangle_depth(px: f64, py: f64, tri: &ProjectedTriangle) -> Option<f64> {
    let (x0, y0) = (tri.sx[0], tri.sy[0]);
    let (x1, y1) = (tri.sx[1], tri.sy[1]);
    let (x2, y2) = (tri.sx[2], tri.sy[2]);

    let denom = (y1 - y2) * (x0 - x2) + (x2 - x1) * (y0 - y2);
    if denom.abs() < 1e-12 {
        return None;
    }

    let a = ((y1 - y2) * (px - x2) + (x2 - x1) * (py - y2)) / denom;
    let b = ((y2 - y0) * (px - x2) + (x0 - x2) * (py - y2)) / denom;
    let c = 1.0 - a - b;

    if a >= -1e-8 && b >= -1e-8 && c >= -1e-8 {
        Some(a * tri.depth[0] + b * tri.depth[1] + c * tri.depth[2])
    } else {
        None
    }
}

/// Tests if an edge is visible by sampling points along it and checking
/// against projected triangles.
fn is_edge_visible(
    sx1: f64,
    sy1: f64,
    d1: f64,
    sx2: f64,
    sy2: f64,
    d2: f64,
    triangles: &[ProjectedTriangle],
) -> bool {
    const SAMPLES: usize = 5;
    let mut visible_count = 0;
    for i in 0..SAMPLES {
        let t = (i as f64 + 0.5) / SAMPLES as f64;
        let px = sx1 + t * (sx2 - sx1);
        let py = sy1 + t * (sy2 - sy1);
        let pd = d1 + t * (d2 - d1);
        let mut occluded = false;
        for tri in triangles {
            if let Some(tri_depth) = point_in_triangle_depth(px, py, tri) {
                // Triangle is closer to camera (larger depth) and in front of edge point.
                if tri_depth > pd + 1e-6 {
                    occluded = true;
                    break;
                }
            }
        }
        if !occluded {
            visible_count += 1;
        }
    }
    visible_count > SAMPLES / 2
}

// ---------------------------------------------------------------------------
// Projection API
// ---------------------------------------------------------------------------

/// Projects all edges of a solid to 2D with hidden-line removal.
pub fn project_solid(
    model: &BRepModel,
    solid: Handle<SolidData>,
    direction: ProjectionDir,
) -> DrawingView {
    let (right, up, toward_cam) = direction.axes();

    let empty_view = DrawingView {
        direction,
        edges: Vec::new(),
        center_x: 0.0,
        center_y: 0.0,
        scale: 1.0,
    };

    let Some(solid_data) = model.solids.get(solid) else {
        return empty_view;
    };

    // Collect all 3D edge segments.
    let mut edges_3d: Vec<(Point3, Point3)> = Vec::new();
    for &shell_h in &solid_data.shells {
        if let Some(shell_data) = model.shells.get(shell_h) {
            for &face_h in &shell_data.faces {
                if let Ok(face_edges) = model.edges_of_face(face_h) {
                    for edge_h in face_edges {
                        if let Some(edge_data) = model.edges.get(edge_h) {
                            if let (Some(v_start), Some(v_end)) = (
                                model.vertices.get(edge_data.start),
                                model.vertices.get(edge_data.end),
                            ) {
                                edges_3d.push((v_start.point, v_end.point));
                            }
                        }
                    }
                }
            }
        }
    }

    // Deduplicate edges.
    edges_3d.sort_by(|a, b| edge_key(a.0, a.1).cmp(&edge_key(b.0, b.1)));
    edges_3d.dedup_by(|a, b| edge_key(a.0, a.1) == edge_key(b.0, b.1));

    // Project edges to 2D.
    let projected: Vec<(f64, f64, f64, f64, f64, f64)> = edges_3d
        .iter()
        .map(|(p1, p2)| {
            let (sx1, sy1, d1) = project_point(*p1, right, up, toward_cam);
            let (sx2, sy2, d2) = project_point(*p2, right, up, toward_cam);
            (sx1, sy1, d1, sx2, sy2, d2)
        })
        .collect();

    // Tessellate for HLR.
    let mesh = tessellate_solid(model, solid);
    let proj_tris: Vec<ProjectedTriangle> = mesh
        .indices
        .iter()
        .map(|idx| {
            let pts: [(f64, f64, f64); 3] = [
                project_point(mesh.vertices[idx[0] as usize], right, up, toward_cam),
                project_point(mesh.vertices[idx[1] as usize], right, up, toward_cam),
                project_point(mesh.vertices[idx[2] as usize], right, up, toward_cam),
            ];
            ProjectedTriangle {
                sx: [pts[0].0, pts[1].0, pts[2].0],
                sy: [pts[0].1, pts[1].1, pts[2].1],
                depth: [pts[0].2, pts[1].2, pts[2].2],
            }
        })
        .collect();

    // Build edges with visibility.
    let edges: Vec<ProjectedEdge> = projected
        .iter()
        .map(|&(sx1, sy1, d1, sx2, sy2, d2)| {
            let visible = is_edge_visible(sx1, sy1, d1, sx2, sy2, d2, &proj_tris);
            ProjectedEdge {
                x1: sx1,
                y1: sy1,
                x2: sx2,
                y2: sy2,
                visible,
            }
        })
        .collect();

    // Compute bounding box center.
    let (mut min_x, mut min_y) = (f64::MAX, f64::MAX);
    let (mut max_x, mut max_y) = (f64::MIN, f64::MIN);
    for e in &edges {
        min_x = min_x.min(e.x1).min(e.x2);
        min_y = min_y.min(e.y1).min(e.y2);
        max_x = max_x.max(e.x1).max(e.x2);
        max_y = max_y.max(e.y1).max(e.y2);
    }
    let cx = if edges.is_empty() {
        0.0
    } else {
        (min_x + max_x) / 2.0
    };
    let cy = if edges.is_empty() {
        0.0
    } else {
        (min_y + max_y) / 2.0
    };

    DrawingView {
        direction,
        edges,
        center_x: cx,
        center_y: cy,
        scale: 1.0,
    }
}

// ---------------------------------------------------------------------------
// Multi-view drawing
// ---------------------------------------------------------------------------

/// Generates a standard third-angle 3-view drawing (Front + Top + Right).
pub fn three_view_drawing(model: &BRepModel, solid: Handle<SolidData>) -> DrawingSheet {
    let mut sheet = DrawingSheet::a4_landscape();
    sheet.title = "Three-View Drawing".into();

    sheet
        .views
        .push(project_solid(model, solid, ProjectionDir::Front));
    sheet
        .views
        .push(project_solid(model, solid, ProjectionDir::Top));
    sheet
        .views
        .push(project_solid(model, solid, ProjectionDir::Right));

    sheet
}

/// Generates a section view — a cross-section of the solid cut by a plane,
/// projected onto the cutting plane.
///
/// The cutting plane is defined by `plane_point` and `plane_normal`.
/// The resulting view shows the outline of the cut and optional hatching.
pub fn section_view(
    model: &BRepModel,
    solid: Handle<SolidData>,
    plane_point: Point3,
    plane_normal: Vec3,
    _label: &str,
) -> DrawingView {
    let mesh = tessellate_solid(model, solid);

    let normal = plane_normal
        .normalized()
        .unwrap_or(Vec3::Z);

    // Build local 2D coordinate system on the cutting plane
    let up_hint = if normal.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
    let right = normal.cross(up_hint).normalized().unwrap_or(Vec3::X);
    let up = right.cross(normal).normalized().unwrap_or(Vec3::Y);

    let mut edges = Vec::new();

    // Find intersection of each triangle with the cutting plane
    for idx in &mesh.indices {
        let pts = [
            mesh.vertices[idx[0] as usize],
            mesh.vertices[idx[1] as usize],
            mesh.vertices[idx[2] as usize],
        ];
        let dists: Vec<f64> = pts
            .iter()
            .map(|p| {
                let d = *p - plane_point;
                d.x * normal.x + d.y * normal.y + d.z * normal.z
            })
            .collect();

        let mut crossings = Vec::new();

        for i in 0..3 {
            let j = (i + 1) % 3;
            if dists[i].abs() < 1e-12 {
                crossings.push(pts[i]);
            } else if dists[i] * dists[j] < 0.0 {
                let t = dists[i] / (dists[i] - dists[j]);
                crossings.push(Point3::new(
                    pts[i].x + t * (pts[j].x - pts[i].x),
                    pts[i].y + t * (pts[j].y - pts[i].y),
                    pts[i].z + t * (pts[j].z - pts[i].z),
                ));
            }
        }

        if crossings.len() >= 2 {
            // Project onto cutting plane 2D coords
            let p1 = crossings[0] - plane_point;
            let p2 = crossings[1] - plane_point;
            edges.push(ProjectedEdge {
                x1: p1.x * right.x + p1.y * right.y + p1.z * right.z,
                y1: p1.x * up.x + p1.y * up.y + p1.z * up.z,
                x2: p2.x * right.x + p2.y * right.y + p2.z * right.z,
                y2: p2.x * up.x + p2.y * up.y + p2.z * up.z,
                visible: true,
            });
        }
    }

    // Compute center
    let (mut cx, mut cy) = (0.0, 0.0);
    if !edges.is_empty() {
        for e in &edges {
            cx += e.x1 + e.x2;
            cy += e.y1 + e.y2;
        }
        let n = edges.len() as f64 * 2.0;
        cx /= n;
        cy /= n;
    }

    DrawingView {
        direction: ProjectionDir::Front, // section views use custom direction
        edges,
        center_x: cx,
        center_y: cy,
        scale: 1.0,
    }
}

/// Generates a detail view — a magnified circular region of an existing view.
///
/// Copies edges from `source_view` that fall within `radius` of `(center_x, center_y)`
/// in the source view's coordinate system, and scales them by `magnification`.
pub fn detail_view(
    source_view: &DrawingView,
    center_x: f64,
    center_y: f64,
    radius: f64,
    magnification: f64,
) -> DrawingView {
    let r2 = radius * radius;
    let mut edges = Vec::new();

    for e in &source_view.edges {
        // Check if either endpoint is within the detail circle
        let dx1 = e.x1 - center_x;
        let dy1 = e.y1 - center_y;
        let dx2 = e.x2 - center_x;
        let dy2 = e.y2 - center_y;

        let in1 = dx1 * dx1 + dy1 * dy1 <= r2;
        let in2 = dx2 * dx2 + dy2 * dy2 <= r2;

        if in1 || in2 {
            edges.push(ProjectedEdge {
                x1: (e.x1 - center_x) * magnification,
                y1: (e.y1 - center_y) * magnification,
                x2: (e.x2 - center_x) * magnification,
                y2: (e.y2 - center_y) * magnification,
                visible: e.visible,
            });
        }
    }

    DrawingView {
        direction: source_view.direction,
        edges,
        center_x: 0.0,
        center_y: 0.0,
        scale: magnification,
    }
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

/// Renders a [`DrawingSheet`] to an SVG document.
pub fn drawing_to_svg(sheet: &DrawingSheet) -> SvgDocument {
    let mut doc = SvgDocument::new(sheet.width, sheet.height);

    // Background.
    doc.add(SvgElement::Line {
        x1: 0.0,
        y1: 0.0,
        x2: 0.0,
        y2: 0.0,
        style: SvgStyle {
            stroke: "none".into(),
            stroke_width: 0.0,
            fill: "white".into(),
            stroke_dasharray: None,
        },
    });

    // Compute global bounds across all views for auto-scaling.
    let (mut gmin_x, mut gmin_y) = (f64::MAX, f64::MAX);
    let (mut gmax_x, mut gmax_y) = (f64::MIN, f64::MIN);
    for view in &sheet.views {
        for e in &view.edges {
            gmin_x = gmin_x.min(e.x1).min(e.x2);
            gmin_y = gmin_y.min(e.y1).min(e.y2);
            gmax_x = gmax_x.max(e.x1).max(e.x2);
            gmax_y = gmax_y.max(e.y1).max(e.y2);
        }
    }
    let margin = 20.0;
    let has_views = gmin_x < gmax_x && gmin_y < gmax_y;

    if has_views {
    let model_w = gmax_x - gmin_x;
    let model_h = gmax_y - gmin_y;
    let n_views = sheet.views.len();

    if n_views == 1 {
        let view = &sheet.views[0];
        let scale = ((sheet.width - 2.0 * margin) / model_w)
            .min((sheet.height - 2.0 * margin) / model_h)
            * 0.8;
        let ox = sheet.width / 2.0;
        let oy = sheet.height / 2.0;
        render_view_to_svg(&mut doc, view, ox, oy, scale);
    } else if n_views >= 3 {
        // Third-angle projection layout:
        // Top view above front, right view beside front.
        let cell_w = (sheet.width - 3.0 * margin) / 2.0;
        let cell_h = (sheet.height - 3.0 * margin) / 2.0;
        let scale = (cell_w / model_w).min(cell_h / model_h) * 0.7;

        let front_cx = margin + cell_w / 2.0;
        let front_cy = margin + cell_h + margin / 2.0 + cell_h / 2.0;
        render_view_to_svg(&mut doc, &sheet.views[0], front_cx, front_cy, scale);

        let top_cx = front_cx;
        let top_cy = margin + cell_h / 2.0;
        render_view_to_svg(&mut doc, &sheet.views[1], top_cx, top_cy, scale);

        let right_cx = margin + cell_w + margin + cell_w / 2.0;
        let right_cy = front_cy;
        render_view_to_svg(&mut doc, &sheet.views[2], right_cx, right_cy, scale);
    } else {
        // 2 views: side by side.
        let cell_w = (sheet.width - 3.0 * margin) / 2.0;
        let cell_h = sheet.height - 2.0 * margin;
        let scale = (cell_w / model_w).min(cell_h / model_h) * 0.7;
        for (i, view) in sheet.views.iter().enumerate() {
            let cx = margin + cell_w / 2.0 + (cell_w + margin) * i as f64;
            let cy = sheet.height / 2.0;
            render_view_to_svg(&mut doc, view, cx, cy, scale);
        }
    }
    } // end if has_views

    // Render dimensions.
    for dim in &sheet.dimensions {
        render_dimension_to_svg(&mut doc, dim);
    }

    // Title block.
    if !sheet.title.is_empty() {
        doc.add(SvgElement::Text {
            x: sheet.width - margin,
            y: sheet.height - 5.0,
            text: sheet.title.clone(),
            font_size: 8.0,
            anchor: "end".into(),
            style: SvgStyle {
                stroke: "none".into(),
                stroke_width: 0.0,
                fill: "black".into(),
                stroke_dasharray: None,
            },
        });
    }

    // Border.
    let border = SvgStyle {
        stroke: "black".into(),
        stroke_width: 0.5,
        fill: "none".into(),
        stroke_dasharray: None,
    };
    doc.add(SvgElement::Polyline {
        points: vec![
            (margin / 2.0, margin / 2.0),
            (sheet.width - margin / 2.0, margin / 2.0),
            (sheet.width - margin / 2.0, sheet.height - margin / 2.0),
            (margin / 2.0, sheet.height - margin / 2.0),
            (margin / 2.0, margin / 2.0),
        ],
        style: border,
    });

    doc
}

fn render_view_to_svg(
    doc: &mut SvgDocument,
    view: &DrawingView,
    cx: f64,
    cy: f64,
    scale: f64,
) {
    let visible_style = SvgStyle {
        stroke: "black".into(),
        stroke_width: 0.5,
        fill: "none".into(),
        stroke_dasharray: None,
    };
    let hidden_style = SvgStyle {
        stroke: "#888888".into(),
        stroke_width: 0.3,
        fill: "none".into(),
        stroke_dasharray: Some("2,1".into()),
    };

    for e in &view.edges {
        let x1 = cx + (e.x1 - view.center_x) * scale;
        let y1 = cy - (e.y1 - view.center_y) * scale; // Flip Y for SVG.
        let x2 = cx + (e.x2 - view.center_x) * scale;
        let y2 = cy - (e.y2 - view.center_y) * scale;
        let style = if e.visible {
            visible_style.clone()
        } else {
            hidden_style.clone()
        };
        doc.add(SvgElement::Line {
            x1,
            y1,
            x2,
            y2,
            style,
        });
    }

    // View label below the view.
    let max_r = view_radius(view, scale);
    doc.add(SvgElement::Text {
        x: cx,
        y: cy + max_r + 12.0,
        text: view.direction.label().to_string(),
        font_size: 6.0,
        anchor: "middle".into(),
        style: SvgStyle {
            stroke: "none".into(),
            stroke_width: 0.0,
            fill: "black".into(),
            stroke_dasharray: None,
        },
    });
}

fn view_radius(view: &DrawingView, scale: f64) -> f64 {
    let mut max_r = 0.0_f64;
    for e in &view.edges {
        let dx1 = (e.x1 - view.center_x) * scale;
        let dy1 = (e.y1 - view.center_y) * scale;
        let dx2 = (e.x2 - view.center_x) * scale;
        let dy2 = (e.y2 - view.center_y) * scale;
        max_r = max_r.max((dx1 * dx1 + dy1 * dy1).sqrt());
        max_r = max_r.max((dx2 * dx2 + dy2 * dy2).sqrt());
    }
    max_r
}

fn render_dimension_to_svg(doc: &mut SvgDocument, dim: &Dimension) {
    let dim_style = SvgStyle {
        stroke: "blue".into(),
        stroke_width: 0.3,
        fill: "none".into(),
        stroke_dasharray: None,
    };
    let text_style = SvgStyle {
        stroke: "none".into(),
        stroke_width: 0.0,
        fill: "blue".into(),
        stroke_dasharray: None,
    };

    match dim {
        Dimension::Linear {
            x1,
            y1,
            x2,
            y2,
            offset,
            text,
        } => {
            let dx = x2 - x1;
            let dy = y2 - y1;
            let len = (dx * dx + dy * dy).sqrt();
            if len < 1e-10 {
                return;
            }
            let nx = -dy / len;
            let ny = dx / len;

            let ext_x1 = x1 + nx * offset;
            let ext_y1 = y1 + ny * offset;
            let ext_x2 = x2 + nx * offset;
            let ext_y2 = y2 + ny * offset;

            // Extension lines.
            doc.add(SvgElement::Line {
                x1: *x1,
                y1: *y1,
                x2: ext_x1,
                y2: ext_y1,
                style: dim_style.clone(),
            });
            doc.add(SvgElement::Line {
                x1: *x2,
                y1: *y2,
                x2: ext_x2,
                y2: ext_y2,
                style: dim_style.clone(),
            });
            // Dimension line.
            doc.add(SvgElement::Line {
                x1: ext_x1,
                y1: ext_y1,
                x2: ext_x2,
                y2: ext_y2,
                style: dim_style,
            });
            // Text at midpoint.
            doc.add(SvgElement::Text {
                x: (ext_x1 + ext_x2) / 2.0,
                y: (ext_y1 + ext_y2) / 2.0 - 1.0,
                text: text.clone(),
                font_size: 5.0,
                anchor: "middle".into(),
                style: text_style,
            });
        }
        Dimension::Radius {
            cx,
            cy,
            r,
            angle_deg,
            text,
        } => {
            let angle_rad = angle_deg.to_radians();
            let ex = cx + r * angle_rad.cos();
            let ey = cy + r * angle_rad.sin();
            doc.add(SvgElement::Line {
                x1: *cx,
                y1: *cy,
                x2: ex,
                y2: ey,
                style: dim_style,
            });
            doc.add(SvgElement::Text {
                x: (cx + ex) / 2.0,
                y: (cy + ey) / 2.0 - 1.0,
                text: text.clone(),
                font_size: 5.0,
                anchor: "middle".into(),
                style: text_style,
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Extended TechDraw annotations (Phase V4)
// ---------------------------------------------------------------------------

/// Arc length dimension.
#[derive(Debug, Clone)]
pub struct ArcLengthDimension {
    pub center: Point2,
    pub radius: f64,
    pub start_angle: f64,
    pub end_angle: f64,
}

/// Renders an arc length dimension to SVG.
pub fn arc_length_dimension_to_svg(dim: &ArcLengthDimension) -> String {
    use std::fmt::Write;
    let mut svg = String::new();
    let style = "stroke=\"blue\" stroke-width=\"0.3\" fill=\"none\"";
    let text_style = "fill=\"blue\" font-size=\"5\" text-anchor=\"middle\"";

    let r = dim.radius;
    let sa = dim.start_angle.to_radians();
    let ea = dim.end_angle.to_radians();
    let arc_len = r * (ea - sa).abs();

    let sx = dim.center.x + r * sa.cos();
    let sy = dim.center.y + r * sa.sin();
    let ex = dim.center.x + r * ea.cos();
    let ey = dim.center.y + r * ea.sin();
    let large = if (ea - sa).abs() > std::f64::consts::PI { 1 } else { 0 };
    let sweep = if ea > sa { 1 } else { 0 };

    let _ = write!(
        svg,
        "<path d=\"M {},{} A {},{} 0 {} {} {},{}\" {} />",
        sx, sy, r, r, large, sweep, ex, ey, style
    );

    let mid_angle = (sa + ea) / 2.0;
    let mx = dim.center.x + (r + 5.0) * mid_angle.cos();
    let my = dim.center.y + (r + 5.0) * mid_angle.sin();
    let _ = write!(svg, "<text x=\"{}\" y=\"{}\" {}>{:.2}</text>", mx, my, text_style, arc_len);
    svg
}

/// Horizontal or vertical extent dimension.
#[derive(Debug, Clone)]
pub enum ExtentDimension {
    Horizontal { min_x: f64, max_x: f64, y: f64 },
    Vertical { min_y: f64, max_y: f64, x: f64 },
}

/// Renders an extent dimension to SVG.
pub fn extent_dimension_to_svg(dim: &ExtentDimension) -> String {
    use std::fmt::Write;
    let mut svg = String::new();
    let style = "stroke=\"green\" stroke-width=\"0.3\" fill=\"none\"";
    let text_style = "fill=\"green\" font-size=\"5\" text-anchor=\"middle\"";

    match dim {
        ExtentDimension::Horizontal { min_x, max_x, y } => {
            let _ = write!(svg, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />", min_x, y, max_x, y, style);
            let _ = write!(svg, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />", min_x, y - 3.0, min_x, y + 3.0, style);
            let _ = write!(svg, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />", max_x, y - 3.0, max_x, y + 3.0, style);
            let mx = (min_x + max_x) / 2.0;
            let _ = write!(svg, "<text x=\"{}\" y=\"{}\" {}>{:.2}</text>", mx, y - 2.0, text_style, max_x - min_x);
        }
        ExtentDimension::Vertical { min_y, max_y, x } => {
            let _ = write!(svg, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />", x, min_y, x, max_y, style);
            let _ = write!(svg, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />", x - 3.0, min_y, x + 3.0, min_y, style);
            let _ = write!(svg, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />", x - 3.0, max_y, x + 3.0, max_y, style);
            let my = (min_y + max_y) / 2.0;
            let _ = write!(svg, "<text x=\"{}\" y=\"{}\" {}>{:.2}</text>", x + 3.0, my, text_style, max_y - min_y);
        }
    }
    svg
}

/// Chamfer dimension (e.g. "C1.5" or "1.5×45°").
#[derive(Debug, Clone)]
pub struct ChamferDimension {
    pub corner: Point2,
    pub size: f64,
    pub angle: f64,
}

/// Renders a chamfer dimension to SVG.
pub fn chamfer_dimension_to_svg(dim: &ChamferDimension) -> String {
    use std::fmt::Write;
    let mut svg = String::new();
    let text_style = "fill=\"blue\" font-size=\"5\" text-anchor=\"start\"";
    let label = if (dim.angle - 45.0).abs() < 0.01 {
        format!("C{:.1}", dim.size)
    } else {
        format!("{:.1}\u{00D7}{:.0}\u{00B0}", dim.size, dim.angle)
    };
    let _ = write!(
        svg,
        "<text x=\"{}\" y=\"{}\" {}>{}</text>",
        dim.corner.x + 3.0,
        dim.corner.y - 3.0,
        text_style,
        label
    );
    svg
}

/// Weld symbol annotation (ISO 2553).
#[derive(Debug, Clone)]
pub struct WeldSymbol {
    pub position: Point2,
    pub weld_type: WeldType,
    pub size: f64,
}

/// Standard weld types.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WeldType {
    Fillet,
    Groove,
    Plug,
    Spot,
    Seam,
    Backing,
}

/// Renders a weld symbol to SVG.
pub fn weld_symbol_to_svg(ws: &WeldSymbol) -> String {
    use std::fmt::Write;
    let mut svg = String::new();
    let s = ws.size;
    let x = ws.position.x;
    let y = ws.position.y;

    // Reference line
    let _ = write!(
        svg,
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"black\" stroke-width=\"0.5\" />",
        x - s, y, x + s, y
    );

    match ws.weld_type {
        WeldType::Fillet => {
            // Triangle symbol below line
            let _ = write!(
                svg,
                "<polygon points=\"{},{} {},{} {},{}\" fill=\"none\" stroke=\"black\" stroke-width=\"0.4\" />",
                x - s * 0.4, y,
                x + s * 0.4, y,
                x, y + s * 0.6
            );
        }
        WeldType::Groove => {
            // V symbol below line
            let _ = write!(
                svg,
                "<polyline points=\"{},{} {},{} {},{}\" fill=\"none\" stroke=\"black\" stroke-width=\"0.4\" />",
                x - s * 0.3, y,
                x, y + s * 0.5,
                x + s * 0.3, y
            );
        }
        WeldType::Plug => {
            // Filled rectangle
            let _ = write!(
                svg,
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"black\" />",
                x - s * 0.2, y, s * 0.4, s * 0.4
            );
        }
        WeldType::Spot => {
            // Circle
            let _ = write!(
                svg,
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"none\" stroke=\"black\" stroke-width=\"0.4\" />",
                x, y + s * 0.3, s * 0.2
            );
        }
        WeldType::Seam => {
            // Dashed arc
            let _ = write!(
                svg,
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"none\" stroke=\"black\" stroke-width=\"0.4\" stroke-dasharray=\"1,1\" />",
                x, y + s * 0.3, s * 0.2
            );
        }
        WeldType::Backing => {
            // Semicircle
            let r = s * 0.3;
            let _ = write!(
                svg,
                "<path d=\"M {},{} A {},{} 0 0 1 {},{}\" fill=\"none\" stroke=\"black\" stroke-width=\"0.4\" />",
                x - r, y, r, r, x + r, y
            );
        }
    }

    // Size text
    let _ = write!(
        svg,
        "<text x=\"{}\" y=\"{}\" font-size=\"4\" fill=\"black\">{:.1}</text>",
        x + s + 1.0, y - 1.0, ws.size
    );
    svg
}

/// Balloon annotation (numbered bubble for parts list).
#[derive(Debug, Clone)]
pub struct BalloonAnnotation {
    pub leader_start: Point2,
    pub balloon_center: Point2,
    pub radius: f64,
    pub text: String,
}

/// Renders a balloon annotation to SVG.
pub fn balloon_annotation_to_svg(ba: &BalloonAnnotation) -> String {
    use std::fmt::Write;
    let mut svg = String::new();

    // Leader line from start to balloon edge
    let dx = ba.balloon_center.x - ba.leader_start.x;
    let dy = ba.balloon_center.y - ba.leader_start.y;
    let len = (dx * dx + dy * dy).sqrt();
    let (ex, ey) = if len > 1e-10 {
        let ux = dx / len;
        let uy = dy / len;
        (ba.balloon_center.x - ux * ba.radius, ba.balloon_center.y - uy * ba.radius)
    } else {
        (ba.balloon_center.x - ba.radius, ba.balloon_center.y)
    };

    let _ = write!(
        svg,
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"black\" stroke-width=\"0.5\" />",
        ba.leader_start.x, ba.leader_start.y, ex, ey
    );

    // Arrowhead at leader start
    if len > ba.radius {
        let ux = dx / len;
        let uy = dy / len;
        let a = 2.5;
        let px = -uy;
        let py = ux;
        let _ = write!(
            svg,
            "<polygon points=\"{},{} {},{} {},{}\" fill=\"black\" />",
            ba.leader_start.x, ba.leader_start.y,
            ba.leader_start.x + ux * a + px * a * 0.3,
            ba.leader_start.y + uy * a + py * a * 0.3,
            ba.leader_start.x + ux * a - px * a * 0.3,
            ba.leader_start.y + uy * a - py * a * 0.3,
        );
    }

    // Balloon circle
    let _ = write!(
        svg,
        "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"white\" stroke=\"black\" stroke-width=\"0.5\" />",
        ba.balloon_center.x, ba.balloon_center.y, ba.radius
    );

    // Number text
    let _ = write!(
        svg,
        "<text x=\"{}\" y=\"{}\" font-size=\"5\" fill=\"black\" text-anchor=\"middle\" dominant-baseline=\"central\">{}</text>",
        ba.balloon_center.x, ba.balloon_center.y, ba.text
    );
    svg
}

/// Centerline between two points.
#[derive(Debug, Clone)]
pub struct Centerline {
    pub start: Point2,
    pub end: Point2,
    pub extension: f64,
}

/// Renders a centerline to SVG (chain-dash pattern).
pub fn centerline_to_svg(cl: &Centerline) -> String {
    use std::fmt::Write;
    let mut svg = String::new();
    let dx = cl.end.x - cl.start.x;
    let dy = cl.end.y - cl.start.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-10 {
        return svg;
    }
    let ux = dx / len;
    let uy = dy / len;
    let x1 = cl.start.x - ux * cl.extension;
    let y1 = cl.start.y - uy * cl.extension;
    let x2 = cl.end.x + ux * cl.extension;
    let y2 = cl.end.y + uy * cl.extension;

    let _ = write!(
        svg,
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"red\" stroke-width=\"0.25\" stroke-dasharray=\"8,2,2,2\" />",
        x1, y1, x2, y2
    );
    svg
}

/// Bolt circle centerlines.
#[derive(Debug, Clone)]
pub struct BoltCircleCenterlines {
    pub center: Point2,
    pub radius: f64,
    pub bolt_count: usize,
    pub start_angle: f64,
    pub mark_size: f64,
}

/// Renders bolt circle centerlines to SVG.
pub fn bolt_circle_centerlines_to_svg(bc: &BoltCircleCenterlines) -> String {
    use std::fmt::Write;
    let mut svg = String::new();
    let style = "stroke=\"red\" stroke-width=\"0.25\" stroke-dasharray=\"8,2,2,2\" fill=\"none\"";

    // Bolt circle (dashed)
    let _ = write!(
        svg,
        "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" {} />",
        bc.center.x, bc.center.y, bc.radius, style
    );

    // Center mark
    let half = bc.mark_size / 2.0;
    let cm_style = "stroke=\"red\" stroke-width=\"0.3\"";
    let _ = write!(svg, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />",
        bc.center.x - half, bc.center.y, bc.center.x + half, bc.center.y, cm_style);
    let _ = write!(svg, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />",
        bc.center.x, bc.center.y - half, bc.center.x, bc.center.y + half, cm_style);

    // Radial lines to each bolt position
    for i in 0..bc.bolt_count {
        let angle = bc.start_angle.to_radians() + 2.0 * std::f64::consts::PI * i as f64 / bc.bolt_count as f64;
        let bx = bc.center.x + bc.radius * angle.cos();
        let by = bc.center.y + bc.radius * angle.sin();

        // Small cross at bolt position
        let s = bc.mark_size * 0.3;
        let _ = write!(svg, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />",
            bx - s, by, bx + s, by, cm_style);
        let _ = write!(svg, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />",
            bx, by - s, bx, by + s, cm_style);
    }
    svg
}

/// Cosmetic line type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CosmeticLineStyle {
    Continuous,
    Dashed,
    DashDot,
    Dotted,
}

/// Cosmetic line (construction/reference line in drawing).
#[derive(Debug, Clone)]
pub struct CosmeticLine {
    pub start: Point2,
    pub end: Point2,
    pub style: CosmeticLineStyle,
    pub color: String,
    pub width: f64,
}

/// Renders a cosmetic line to SVG.
pub fn cosmetic_line_to_svg(cl: &CosmeticLine) -> String {
    use std::fmt::Write;
    let mut svg = String::new();
    let dash = match cl.style {
        CosmeticLineStyle::Continuous => String::new(),
        CosmeticLineStyle::Dashed => " stroke-dasharray=\"4,2\"".to_string(),
        CosmeticLineStyle::DashDot => " stroke-dasharray=\"8,2,2,2\"".to_string(),
        CosmeticLineStyle::Dotted => " stroke-dasharray=\"1,2\"".to_string(),
    };
    let _ = write!(
        svg,
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
        cl.start.x, cl.start.y, cl.end.x, cl.end.y, cl.color, cl.width, dash
    );
    svg
}

/// Broken view break lines.
#[derive(Debug, Clone)]
pub struct BreakLine {
    pub y_position: f64,
    pub x_min: f64,
    pub x_max: f64,
    pub amplitude: f64,
}

/// Renders a zig-zag break line to SVG.
pub fn break_line_to_svg(bl: &BreakLine) -> String {
    use std::fmt::Write;
    let mut svg = String::new();
    let segments = 8;
    let dx = (bl.x_max - bl.x_min) / segments as f64;
    let mut points = Vec::with_capacity(segments + 1);
    for i in 0..=segments {
        let x = bl.x_min + i as f64 * dx;
        let y = bl.y_position + if i % 2 == 0 { bl.amplitude } else { -bl.amplitude };
        points.push(format!("{},{}", x, y));
    }
    let _ = write!(
        svg,
        "<polyline points=\"{}\" fill=\"none\" stroke=\"black\" stroke-width=\"0.5\" />",
        points.join(" ")
    );
    svg
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_math::Point3;
    use cadkernel_topology::BRepModel;

    /// Builds a simple 10×10×10 box for testing (no dependency on modeling crate).
    fn make_test_box(model: &mut BRepModel) -> Handle<SolidData> {
        let pts = [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(10.0, 0.0, 0.0),
            Point3::new(10.0, 10.0, 0.0),
            Point3::new(0.0, 10.0, 0.0),
            Point3::new(0.0, 0.0, 10.0),
            Point3::new(10.0, 0.0, 10.0),
            Point3::new(10.0, 10.0, 10.0),
            Point3::new(0.0, 10.0, 10.0),
        ];
        let verts: Vec<_> = pts.iter().map(|&p| model.add_vertex(p)).collect();

        let face_indices = [
            [0, 3, 2, 1], // bottom
            [4, 5, 6, 7], // top
            [0, 1, 5, 4], // front
            [2, 3, 7, 6], // back
            [0, 4, 7, 3], // left
            [1, 2, 6, 5], // right
        ];

        let mut faces = Vec::new();
        for fi in &face_indices {
            let mut hes = Vec::new();
            for i in 0..4 {
                let j = (i + 1) % 4;
                let (_, he, _) = model.add_edge(verts[fi[i]], verts[fi[j]]);
                hes.push(he);
            }
            let lp = model.make_loop(&hes).unwrap();
            faces.push(model.make_face(lp));
        }
        let shell = model.make_shell(&faces);
        model.make_solid(&[shell])
    }

    #[test]
    fn test_project_front_view() {
        let mut model = BRepModel::new();
        let solid = make_test_box(&mut model);
        let view = project_solid(&model, solid, ProjectionDir::Front);
        assert!(!view.edges.is_empty(), "front view should have edges");
    }

    #[test]
    fn test_three_view_drawing() {
        let mut model = BRepModel::new();
        let solid = make_test_box(&mut model);
        let sheet = three_view_drawing(&model, solid);
        assert_eq!(sheet.views.len(), 3);
        for v in &sheet.views {
            assert!(!v.edges.is_empty());
        }
    }

    #[test]
    fn test_drawing_to_svg_output() {
        let mut model = BRepModel::new();
        let solid = make_test_box(&mut model);
        let sheet = three_view_drawing(&model, solid);
        let svg = drawing_to_svg(&sheet);
        let rendered = svg.render();
        assert!(rendered.contains("<svg"));
        assert!(rendered.contains("</svg>"));
        assert!(rendered.contains("<line"));
        assert!(rendered.contains("<text"));
    }

    #[test]
    fn test_projection_axes_orthogonal() {
        for dir in &[
            ProjectionDir::Front,
            ProjectionDir::Back,
            ProjectionDir::Top,
            ProjectionDir::Bottom,
            ProjectionDir::Right,
            ProjectionDir::Left,
            ProjectionDir::Isometric,
        ] {
            let (r, u, b) = dir.axes();
            assert!(
                dot3(r, u).abs() < 1e-10,
                "{:?}: right·up = {}",
                dir,
                dot3(r, u)
            );
            assert!(
                dot3(r, b).abs() < 1e-10,
                "{:?}: right·back = {}",
                dir,
                dot3(r, b)
            );
            assert!(
                dot3(u, b).abs() < 1e-10,
                "{:?}: up·back = {}",
                dir,
                dot3(u, b)
            );
        }
    }

    #[test]
    fn test_single_view_svg() {
        let mut model = BRepModel::new();
        let solid = make_test_box(&mut model);
        let view = project_solid(&model, solid, ProjectionDir::Isometric);
        let mut sheet = DrawingSheet::a4_landscape();
        sheet.views.push(view);
        let svg = drawing_to_svg(&sheet);
        let rendered = svg.render();
        assert!(rendered.contains("Isometric"));
    }

    #[test]
    fn test_section_view() {
        let mut model = BRepModel::new();
        let solid = make_test_box(&mut model);
        let view = section_view(
            &model,
            solid,
            Point3::new(0.0, 0.0, 5.0),
            Vec3::Z,
            "A-A",
        );
        assert!(!view.edges.is_empty(), "section view should have edges");
    }

    #[test]
    fn test_detail_view() {
        let mut model = BRepModel::new();
        let solid = make_test_box(&mut model);
        let front = project_solid(&model, solid, ProjectionDir::Front);
        let detail = detail_view(&front, 5.0, 5.0, 3.0, 2.0);
        // Detail view should have some edges from the front view
        assert!(detail.scale > 1.0);
    }

    #[test]
    fn test_text_annotation_to_svg() {
        let ann = TextAnnotation {
            position: Point2::new(10.0, 20.0),
            text: "Hello".into(),
            font_size: 12.0,
        };
        let svg = text_annotation_to_svg(&ann);
        assert!(svg.contains("<text"));
        assert!(svg.contains("Hello"));
        assert!(svg.contains("font-size=\"12\""));
        assert!(svg.contains("x=\"10\""));
        assert!(svg.contains("y=\"20\""));
    }

    #[test]
    fn test_leader_line_to_svg() {
        let ll = LeaderLine {
            start: Point2::new(50.0, 50.0),
            end: Point2::new(100.0, 30.0),
            text: "Feature A".into(),
        };
        let svg = leader_line_to_svg(&ll);
        assert!(svg.contains("<line"));
        assert!(svg.contains("<polygon"));
        assert!(svg.contains("<text"));
        assert!(svg.contains("Feature A"));
    }

    #[test]
    fn test_hatch_pattern_to_svg() {
        let hp = HatchPattern {
            boundary: vec![
                Point2::new(0.0, 0.0),
                Point2::new(20.0, 0.0),
                Point2::new(20.0, 20.0),
                Point2::new(0.0, 20.0),
            ],
            angle: 45.0,
            spacing: 2.0,
        };
        let svg = hatch_pattern_to_svg(&hp);
        assert!(svg.contains("<line"));
        assert!(svg.contains("stroke-width=\"0.2\""));
    }

    #[test]
    fn test_hatch_pattern_empty_boundary() {
        let hp = HatchPattern {
            boundary: vec![],
            angle: 45.0,
            spacing: 2.0,
        };
        let svg = hatch_pattern_to_svg(&hp);
        assert!(svg.is_empty());
    }

    #[test]
    fn test_center_mark_to_svg() {
        let cm = CenterMark {
            center: Point2::new(50.0, 50.0),
            size: 10.0,
        };
        let svg = center_mark_to_svg(&cm);
        assert!(svg.contains("<line"));
        // Should have two perpendicular lines
        assert_eq!(svg.matches("<line").count(), 2);
        assert!(svg.contains("stroke=\"red\""));
    }

    #[test]
    fn test_surface_finish_to_svg() {
        let sf = SurfaceFinishSymbol {
            position: Point2::new(80.0, 60.0),
            roughness: 1.6,
        };
        let svg = surface_finish_to_svg(&sf);
        assert!(svg.contains("<polyline"));
        assert!(svg.contains("<text"));
        assert!(svg.contains("Ra 1.6"));
    }

    #[test]
    fn test_dimension_svg() {
        let mut sheet = DrawingSheet::a4_landscape();
        sheet.dimensions.push(Dimension::Linear {
            x1: 50.0,
            y1: 100.0,
            x2: 150.0,
            y2: 100.0,
            offset: 15.0,
            text: "100.00".into(),
        });
        let svg = drawing_to_svg(&sheet);
        let rendered = svg.render();
        assert!(rendered.contains("100.00"));
    }

    #[test]
    fn test_arc_length_dimension_to_svg() {
        let dim = ArcLengthDimension {
            center: Point2::new(50.0, 50.0),
            radius: 20.0,
            start_angle: 0.0,
            end_angle: 90.0,
        };
        let svg = arc_length_dimension_to_svg(&dim);
        assert!(svg.contains("<path"));
        assert!(svg.contains("<text"));
    }

    #[test]
    fn test_extent_dimension_horizontal() {
        let dim = ExtentDimension::Horizontal { min_x: 10.0, max_x: 50.0, y: 80.0 };
        let svg = extent_dimension_to_svg(&dim);
        assert!(svg.contains("<line"));
        assert!(svg.contains("40.00"));
    }

    #[test]
    fn test_extent_dimension_vertical() {
        let dim = ExtentDimension::Vertical { min_y: 10.0, max_y: 60.0, x: 80.0 };
        let svg = extent_dimension_to_svg(&dim);
        assert!(svg.contains("<line"));
        assert!(svg.contains("50.00"));
    }

    #[test]
    fn test_chamfer_dimension_45() {
        let dim = ChamferDimension {
            corner: Point2::new(20.0, 20.0),
            size: 1.5,
            angle: 45.0,
        };
        let svg = chamfer_dimension_to_svg(&dim);
        assert!(svg.contains("C1.5"));
    }

    #[test]
    fn test_chamfer_dimension_non_45() {
        let dim = ChamferDimension {
            corner: Point2::new(20.0, 20.0),
            size: 2.0,
            angle: 30.0,
        };
        let svg = chamfer_dimension_to_svg(&dim);
        assert!(svg.contains("2.0"));
        assert!(svg.contains("30"));
    }

    #[test]
    fn test_weld_symbol_fillet() {
        let ws = WeldSymbol {
            position: Point2::new(50.0, 50.0),
            weld_type: WeldType::Fillet,
            size: 5.0,
        };
        let svg = weld_symbol_to_svg(&ws);
        assert!(svg.contains("<line"));
        assert!(svg.contains("<polygon"));
    }

    #[test]
    fn test_weld_symbol_groove() {
        let ws = WeldSymbol {
            position: Point2::new(50.0, 50.0),
            weld_type: WeldType::Groove,
            size: 5.0,
        };
        let svg = weld_symbol_to_svg(&ws);
        assert!(svg.contains("<polyline"));
    }

    #[test]
    fn test_balloon_annotation() {
        let ba = BalloonAnnotation {
            leader_start: Point2::new(30.0, 30.0),
            balloon_center: Point2::new(60.0, 10.0),
            radius: 5.0,
            text: "1".into(),
        };
        let svg = balloon_annotation_to_svg(&ba);
        assert!(svg.contains("<circle"));
        assert!(svg.contains("<line"));
        assert!(svg.contains(">1<"));
    }

    #[test]
    fn test_centerline_to_svg() {
        let cl = Centerline {
            start: Point2::new(10.0, 50.0),
            end: Point2::new(90.0, 50.0),
            extension: 5.0,
        };
        let svg = centerline_to_svg(&cl);
        assert!(svg.contains("<line"));
        assert!(svg.contains("stroke-dasharray"));
    }

    #[test]
    fn test_bolt_circle_centerlines() {
        let bc = BoltCircleCenterlines {
            center: Point2::new(50.0, 50.0),
            radius: 20.0,
            bolt_count: 6,
            start_angle: 0.0,
            mark_size: 4.0,
        };
        let svg = bolt_circle_centerlines_to_svg(&bc);
        assert!(svg.contains("<circle"));
        // Center mark + 6 bolt crosses (2 lines each) + 2 center lines = many lines
        assert!(svg.matches("<line").count() >= 14);
    }

    #[test]
    fn test_cosmetic_line_dashed() {
        let cl = CosmeticLine {
            start: Point2::new(0.0, 0.0),
            end: Point2::new(100.0, 100.0),
            style: CosmeticLineStyle::Dashed,
            color: "gray".into(),
            width: 0.3,
        };
        let svg = cosmetic_line_to_svg(&cl);
        assert!(svg.contains("stroke=\"gray\""));
        assert!(svg.contains("stroke-dasharray"));
    }

    #[test]
    fn test_break_line() {
        let bl = BreakLine {
            y_position: 50.0,
            x_min: 10.0,
            x_max: 90.0,
            amplitude: 3.0,
        };
        let svg = break_line_to_svg(&bl);
        assert!(svg.contains("<polyline"));
    }
}
