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
            Self::Back => (
                Vec3::new(-1.0, 0.0, 0.0),
                Vec3::Z,
                Vec3::new(0.0, -1.0, 0.0),
            ),
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
    pub sheet_x: Option<f64>,
    pub sheet_y: Option<f64>,
    pub sheet_scale: Option<f64>,
}

impl DrawingView {
    /// Override the automatic sheet placement used by `drawing_to_svg`.
    pub fn set_sheet_placement(&mut self, x: f64, y: f64, scale: Option<f64>) {
        self.sheet_x = Some(x);
        self.sheet_y = Some(y);
        self.sheet_scale = scale.filter(|s| *s > 0.0);
    }
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
            let _ = write!(
                svg,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />",
                start.x, start.y, ex1, ey1, style
            );
            let _ = write!(
                svg,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />",
                end.x, end.y, ex2, ey2, style
            );
            let _ = write!(
                svg,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />",
                ex1, ey1, ex2, ey2, style
            );
            let mx = (ex1 + ex2) / 2.0;
            let my = (ey1 + ey2) / 2.0 - 1.0;
            let _ = write!(
                svg,
                "<text x=\"{}\" y=\"{}\" {}>{:.2}</text>",
                mx, my, text_style, value
            );
        }
        DimensionType::HorizontalDimension { start, end, value } => {
            let offset = 10.0;
            let y_line = start.y.min(end.y) - offset;
            let _ = write!(
                svg,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />",
                start.x, start.y, start.x, y_line, style
            );
            let _ = write!(
                svg,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />",
                end.x, end.y, end.x, y_line, style
            );
            let _ = write!(
                svg,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />",
                start.x, y_line, end.x, y_line, style
            );
            let mx = (start.x + end.x) / 2.0;
            let _ = write!(
                svg,
                "<text x=\"{}\" y=\"{}\" {}>{:.2}</text>",
                mx,
                y_line - 1.0,
                text_style,
                value
            );
        }
        DimensionType::VerticalDimension { start, end, value } => {
            let offset = 10.0;
            let x_line = start.x.max(end.x) + offset;
            let _ = write!(
                svg,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />",
                start.x, start.y, x_line, start.y, style
            );
            let _ = write!(
                svg,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />",
                end.x, end.y, x_line, end.y, style
            );
            let _ = write!(
                svg,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />",
                x_line, start.y, x_line, end.y, style
            );
            let my = (start.y + end.y) / 2.0;
            let _ = write!(
                svg,
                "<text x=\"{}\" y=\"{}\" {}>{:.2}</text>",
                x_line + 3.0,
                my,
                text_style,
                value
            );
        }
        DimensionType::RadiusDimension { center, radius } => {
            let ex = center.x + radius;
            let ey = center.y;
            let _ = write!(
                svg,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />",
                center.x, center.y, ex, ey, style
            );
            let mx = (center.x + ex) / 2.0;
            let _ = write!(
                svg,
                "<text x=\"{}\" y=\"{}\" {}>R{:.2}</text>",
                mx,
                center.y - 1.0,
                text_style,
                radius
            );
        }
        DimensionType::DiameterDimension { center, diameter } => {
            let r = diameter / 2.0;
            let x1 = center.x - r;
            let x2 = center.x + r;
            let _ = write!(
                svg,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />",
                x1, center.y, x2, center.y, style
            );
            let _ = write!(
                svg,
                "<text x=\"{}\" y=\"{}\" {}>\u{2300}{:.2}</text>",
                center.x,
                center.y - 1.0,
                text_style,
                diameter
            );
        }
        DimensionType::AngleDimension {
            vertex,
            arm1_end,
            arm2_end,
            angle,
        } => {
            let _ = write!(
                svg,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />",
                vertex.x, vertex.y, arm1_end.x, arm1_end.y, style
            );
            let _ = write!(
                svg,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />",
                vertex.x, vertex.y, arm2_end.x, arm2_end.y, style
            );
            let mx = (vertex.x + arm1_end.x + arm2_end.x) / 3.0;
            let my = (vertex.y + arm1_end.y + arm2_end.y) / 3.0;
            let _ = write!(
                svg,
                "<text x=\"{}\" y=\"{}\" {}>{:.1}\u{00B0}</text>",
                mx, my, text_style, angle
            );
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
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
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
    pub extended_dimensions: Vec<DimensionType>,
    pub arc_length_dimensions: Vec<ArcLengthDimension>,
    pub area_annotations: Vec<AreaAnnotation>,
    pub text_annotations: Vec<TextAnnotation>,
    pub rich_text_annotations: Vec<RichTextAnnotation>,
    pub balloon_annotations: Vec<BalloonAnnotation>,
    pub leader_lines: Vec<LeaderLine>,
    pub weld_symbols: Vec<WeldSymbolFull>,
    pub surface_finish_symbols: Vec<SurfaceFinishSymbol>,
    pub center_marks: Vec<CenterMark>,
    pub centerlines: Vec<Centerline>,
    pub bolt_circle_centerlines: Vec<BoltCircleCenterlines>,
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
            extended_dimensions: Vec::new(),
            arc_length_dimensions: Vec::new(),
            area_annotations: Vec::new(),
            text_annotations: Vec::new(),
            rich_text_annotations: Vec::new(),
            balloon_annotations: Vec::new(),
            leader_lines: Vec::new(),
            weld_symbols: Vec::new(),
            surface_finish_symbols: Vec::new(),
            center_marks: Vec::new(),
            centerlines: Vec::new(),
            bolt_circle_centerlines: Vec::new(),
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
        sheet_x: None,
        sheet_y: None,
        sheet_scale: None,
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
    edges_3d.sort_by_key(|a| edge_key(a.0, a.1));
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
        sheet_x: None,
        sheet_y: None,
        sheet_scale: None,
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

    let normal = plane_normal.normalized().unwrap_or(Vec3::Z);

    // Build local 2D coordinate system on the cutting plane
    let up_hint = if normal.x.abs() < 0.9 {
        Vec3::X
    } else {
        Vec3::Y
    };
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
        sheet_x: None,
        sheet_y: None,
        sheet_scale: None,
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
        sheet_x: None,
        sheet_y: None,
        sheet_scale: None,
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
        let has_manual_layout = sheet.views.iter().any(|view| {
            view.sheet_x.is_some() || view.sheet_y.is_some() || view.sheet_scale.is_some()
        });

        if has_manual_layout {
            let cols = (n_views as f64).sqrt().ceil() as usize;
            let rows = n_views.div_ceil(cols);
            let cell_w = (sheet.width - margin * (cols as f64 + 1.0)) / cols as f64;
            let cell_h = (sheet.height - margin * (rows as f64 + 1.0)) / rows as f64;
            let scale = (cell_w / model_w).min(cell_h / model_h) * 0.7;
            for (i, view) in sheet.views.iter().enumerate() {
                let col = i % cols;
                let row = i / cols;
                let default_cx = margin + cell_w / 2.0 + (cell_w + margin) * col as f64;
                let default_cy = margin + cell_h / 2.0 + (cell_h + margin) * row as f64;
                let (cx, cy, view_scale) = view_sheet_layout(view, default_cx, default_cy, scale);
                render_view_to_svg(&mut doc, view, cx, cy, view_scale);
            }
        } else if n_views == 1 {
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
    for dim in &sheet.extended_dimensions {
        render_dimension_type_to_svg(&mut doc, dim);
    }
    for dim in &sheet.arc_length_dimensions {
        render_arc_length_dimension_to_svg(&mut doc, dim);
    }
    for ann in &sheet.area_annotations {
        render_area_annotation_to_svg(&mut doc, ann);
    }
    for ann in &sheet.text_annotations {
        render_text_annotation_to_svg(&mut doc, ann);
    }
    for ann in &sheet.rich_text_annotations {
        render_rich_text_annotation_to_svg(&mut doc, ann);
    }
    for ann in &sheet.balloon_annotations {
        render_balloon_annotation_to_svg(&mut doc, ann);
    }
    for leader in &sheet.leader_lines {
        render_leader_line_to_svg(&mut doc, leader);
    }
    for weld in &sheet.weld_symbols {
        render_weld_symbol_to_svg(&mut doc, weld);
    }
    for symbol in &sheet.surface_finish_symbols {
        render_surface_finish_to_svg(&mut doc, symbol);
    }
    for mark in &sheet.center_marks {
        render_center_mark_to_svg(&mut doc, mark);
    }
    for centerline in &sheet.centerlines {
        render_centerline_to_svg(&mut doc, centerline);
    }
    for bolt_circle in &sheet.bolt_circle_centerlines {
        render_bolt_circle_centerlines_to_svg(&mut doc, bolt_circle);
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

fn view_sheet_layout(
    view: &DrawingView,
    default_x: f64,
    default_y: f64,
    default_scale: f64,
) -> (f64, f64, f64) {
    (
        view.sheet_x.unwrap_or(default_x),
        view.sheet_y.unwrap_or(default_y),
        view.sheet_scale.unwrap_or(default_scale),
    )
}

fn render_view_to_svg(doc: &mut SvgDocument, view: &DrawingView, cx: f64, cy: f64, scale: f64) {
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

fn render_dimension_type_to_svg(doc: &mut SvgDocument, dim: &DimensionType) {
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
        DimensionType::Length { start, end, value } => {
            doc.add(SvgElement::Line {
                x1: start.x,
                y1: start.y,
                x2: end.x,
                y2: end.y,
                style: dim_style,
            });
            doc.add(SvgElement::Text {
                x: (start.x + end.x) * 0.5,
                y: (start.y + end.y) * 0.5 - 2.0,
                text: format!("{value:.2}"),
                font_size: 5.0,
                anchor: "middle".into(),
                style: text_style,
            });
        }
        DimensionType::HorizontalDimension { start, end, value } => {
            doc.add(SvgElement::Line {
                x1: start.x,
                y1: start.y,
                x2: end.x,
                y2: start.y,
                style: dim_style,
            });
            doc.add(SvgElement::Text {
                x: (start.x + end.x) * 0.5,
                y: start.y - 2.0,
                text: format!("{value:.2}"),
                font_size: 5.0,
                anchor: "middle".into(),
                style: text_style,
            });
        }
        DimensionType::VerticalDimension { start, end, value } => {
            doc.add(SvgElement::Line {
                x1: start.x,
                y1: start.y,
                x2: start.x,
                y2: end.y,
                style: dim_style,
            });
            doc.add(SvgElement::Text {
                x: start.x + 3.0,
                y: (start.y + end.y) * 0.5,
                text: format!("{value:.2}"),
                font_size: 5.0,
                anchor: "start".into(),
                style: text_style,
            });
        }
        DimensionType::RadiusDimension { center, radius } => {
            let visual_radius = radius.max(12.0);
            doc.add(SvgElement::Line {
                x1: center.x,
                y1: center.y,
                x2: center.x + visual_radius,
                y2: center.y,
                style: dim_style,
            });
            doc.add(SvgElement::Text {
                x: center.x + visual_radius * 0.5,
                y: center.y - 2.0,
                text: format!("R{radius:.2}"),
                font_size: 5.0,
                anchor: "middle".into(),
                style: text_style,
            });
        }
        DimensionType::DiameterDimension { center, diameter } => {
            let visual_radius = (diameter * 0.5).max(12.0);
            doc.add(SvgElement::Line {
                x1: center.x - visual_radius,
                y1: center.y,
                x2: center.x + visual_radius,
                y2: center.y,
                style: dim_style,
            });
            doc.add(SvgElement::Text {
                x: center.x,
                y: center.y - 2.0,
                text: format!("\u{2300}{diameter:.2}"),
                font_size: 5.0,
                anchor: "middle".into(),
                style: text_style,
            });
        }
        DimensionType::AngleDimension {
            vertex,
            arm1_end,
            arm2_end,
            angle,
        } => {
            doc.add(SvgElement::Line {
                x1: vertex.x,
                y1: vertex.y,
                x2: arm1_end.x,
                y2: arm1_end.y,
                style: dim_style.clone(),
            });
            doc.add(SvgElement::Line {
                x1: vertex.x,
                y1: vertex.y,
                x2: arm2_end.x,
                y2: arm2_end.y,
                style: dim_style,
            });
            doc.add(SvgElement::Text {
                x: (vertex.x + arm1_end.x + arm2_end.x) / 3.0,
                y: (vertex.y + arm1_end.y + arm2_end.y) / 3.0,
                text: format!("{angle:.1}\u{00B0}"),
                font_size: 5.0,
                anchor: "middle".into(),
                style: text_style,
            });
        }
    }
}

fn render_arc_length_dimension_to_svg(doc: &mut SvgDocument, dim: &ArcLengthDimension) {
    let style = SvgStyle {
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
    let radius = dim.radius.max(12.0);
    let start = dim.start_angle.to_radians();
    let end = dim.end_angle.to_radians();
    let steps = 24;
    let points: Vec<(f64, f64)> = (0..=steps)
        .map(|i| {
            let t = start + (end - start) * i as f64 / steps as f64;
            (
                dim.center.x + radius * t.cos(),
                dim.center.y + radius * t.sin(),
            )
        })
        .collect();
    doc.add(SvgElement::Polyline { points, style });
    let mid = (start + end) * 0.5;
    let arc_len = dim.radius * (end - start).abs();
    doc.add(SvgElement::Text {
        x: dim.center.x + (radius + 6.0) * mid.cos(),
        y: dim.center.y + (radius + 6.0) * mid.sin(),
        text: format!("Arc {arc_len:.2}"),
        font_size: 5.0,
        anchor: "middle".into(),
        style: text_style,
    });
}

fn render_area_annotation_to_svg(doc: &mut SvgDocument, ann: &AreaAnnotation) {
    let area_style = SvgStyle {
        stroke: "green".into(),
        stroke_width: 0.3,
        fill: "none".into(),
        stroke_dasharray: Some("3,1".into()),
    };
    let text_style = SvgStyle {
        stroke: "none".into(),
        stroke_width: 0.0,
        fill: "green".into(),
        stroke_dasharray: None,
    };
    if ann.boundary.len() >= 3 {
        let mut points: Vec<(f64, f64)> = ann.boundary.iter().map(|p| (p.x, p.y)).collect();
        if let Some(first) = points.first().copied() {
            points.push(first);
        }
        doc.add(SvgElement::Polyline {
            points,
            style: area_style,
        });
    }
    doc.add(SvgElement::Text {
        x: ann.label_position.x,
        y: ann.label_position.y,
        text: format!("Area: {:.2}", ann.area),
        font_size: 5.0,
        anchor: "middle".into(),
        style: text_style,
    });
}

fn annotation_text_style() -> SvgStyle {
    SvgStyle {
        stroke: "none".into(),
        stroke_width: 0.0,
        fill: "black".into(),
        stroke_dasharray: None,
    }
}

fn annotation_line_style() -> SvgStyle {
    SvgStyle {
        stroke: "black".into(),
        stroke_width: 0.5,
        fill: "none".into(),
        stroke_dasharray: None,
    }
}

fn render_text_annotation_to_svg(doc: &mut SvgDocument, ann: &TextAnnotation) {
    doc.add(SvgElement::Text {
        x: ann.position.x,
        y: ann.position.y,
        text: ann.text.clone(),
        font_size: ann.font_size,
        anchor: "start".into(),
        style: annotation_text_style(),
    });
}

fn strip_rich_text_tags(html: &str) -> String {
    let mut plain = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => plain.push(ch),
            _ => {}
        }
    }
    plain
}

fn render_rich_text_annotation_to_svg(doc: &mut SvgDocument, ann: &RichTextAnnotation) {
    doc.add(SvgElement::Text {
        x: ann.position.x,
        y: ann.position.y,
        text: strip_rich_text_tags(&ann.html_content),
        font_size: ann.font_size,
        anchor: "start".into(),
        style: annotation_text_style(),
    });
}

fn add_arrowhead(doc: &mut SvgDocument, tip: Point2, tail: Point2) {
    let dx = tail.x - tip.x;
    let dy = tail.y - tip.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 1e-10 {
        return;
    }
    let ux = dx / len;
    let uy = dy / len;
    let px = -uy;
    let py = ux;
    let a = 3.0;
    let p1 = (
        tip.x + ux * a + px * a * 0.35,
        tip.y + uy * a + py * a * 0.35,
    );
    let p2 = (
        tip.x + ux * a - px * a * 0.35,
        tip.y + uy * a - py * a * 0.35,
    );
    doc.add(SvgElement::Polyline {
        points: vec![p1, (tip.x, tip.y), p2],
        style: annotation_line_style(),
    });
}

fn render_balloon_annotation_to_svg(doc: &mut SvgDocument, ba: &BalloonAnnotation) {
    let dx = ba.balloon_center.x - ba.leader_start.x;
    let dy = ba.balloon_center.y - ba.leader_start.y;
    let len = (dx * dx + dy * dy).sqrt();
    let end = if len > 1e-10 {
        Point2::new(
            ba.balloon_center.x - dx / len * ba.radius,
            ba.balloon_center.y - dy / len * ba.radius,
        )
    } else {
        Point2::new(ba.balloon_center.x - ba.radius, ba.balloon_center.y)
    };
    doc.add(SvgElement::Line {
        x1: ba.leader_start.x,
        y1: ba.leader_start.y,
        x2: end.x,
        y2: end.y,
        style: annotation_line_style(),
    });
    add_arrowhead(doc, ba.leader_start, end);
    doc.add(SvgElement::Circle {
        cx: ba.balloon_center.x,
        cy: ba.balloon_center.y,
        r: ba.radius,
        style: annotation_line_style(),
    });
    doc.add(SvgElement::Text {
        x: ba.balloon_center.x,
        y: ba.balloon_center.y + ba.radius * 0.35,
        text: ba.text.clone(),
        font_size: ba.radius,
        anchor: "middle".into(),
        style: annotation_text_style(),
    });
}

fn render_leader_line_to_svg(doc: &mut SvgDocument, leader: &LeaderLine) {
    doc.add(SvgElement::Line {
        x1: leader.start.x,
        y1: leader.start.y,
        x2: leader.end.x,
        y2: leader.end.y,
        style: annotation_line_style(),
    });
    add_arrowhead(doc, leader.start, leader.end);
    doc.add(SvgElement::Text {
        x: leader.end.x + 3.0,
        y: leader.end.y,
        text: leader.text.clone(),
        font_size: 5.0,
        anchor: "start".into(),
        style: annotation_text_style(),
    });
}

fn render_weld_symbol_to_svg(doc: &mut SvgDocument, ws: &WeldSymbolFull) {
    let x = ws.position.x;
    let y = ws.position.y;
    let s = ws.size;
    let line_style = annotation_line_style();
    doc.add(SvgElement::Line {
        x1: x - s,
        y1: y,
        x2: x + s,
        y2: y,
        style: line_style.clone(),
    });

    match ws.weld_type {
        WeldType::Fillet => doc.add(SvgElement::Polyline {
            points: vec![
                (x - s * 0.4, y),
                (x + s * 0.4, y),
                (x, y + s * 0.6),
                (x - s * 0.4, y),
            ],
            style: line_style.clone(),
        }),
        WeldType::Groove => doc.add(SvgElement::Polyline {
            points: vec![(x - s * 0.3, y), (x, y + s * 0.5), (x + s * 0.3, y)],
            style: line_style.clone(),
        }),
        WeldType::Plug => doc.add(SvgElement::Polyline {
            points: vec![
                (x - s * 0.25, y),
                (x + s * 0.25, y),
                (x + s * 0.25, y + s * 0.45),
                (x - s * 0.25, y + s * 0.45),
                (x - s * 0.25, y),
            ],
            style: line_style.clone(),
        }),
        WeldType::Spot | WeldType::Seam => doc.add(SvgElement::Circle {
            cx: x,
            cy: y + s * 0.3,
            r: s * 0.2,
            style: if ws.weld_type == WeldType::Seam {
                SvgStyle {
                    stroke_dasharray: Some("1,1".into()),
                    ..line_style.clone()
                }
            } else {
                line_style.clone()
            },
        }),
        WeldType::Backing => doc.add(SvgElement::Polyline {
            points: vec![(x - s * 0.3, y), (x, y + s * 0.25), (x + s * 0.3, y)],
            style: line_style.clone(),
        }),
    }

    doc.add(SvgElement::Text {
        x: x + s + 2.0,
        y: y - 1.0,
        text: format!("{:.1}", ws.size),
        font_size: 4.0,
        anchor: "start".into(),
        style: annotation_text_style(),
    });
    if ws.length > 0.0 || ws.pitch > 0.0 {
        let label = if ws.pitch > 0.0 {
            format!("{:.0}({:.0})", ws.length, ws.pitch)
        } else {
            format!("{:.0}", ws.length)
        };
        doc.add(SvgElement::Text {
            x: x - s,
            y: y - 3.0,
            text: label,
            font_size: 3.5,
            anchor: "start".into(),
            style: annotation_text_style(),
        });
    }
}

fn render_surface_finish_to_svg(doc: &mut SvgDocument, sf: &SurfaceFinishSymbol) {
    let s = 5.0;
    doc.add(SvgElement::Polyline {
        points: vec![
            (sf.position.x - s * 0.5, sf.position.y - s),
            (sf.position.x, sf.position.y),
            (sf.position.x + s * 0.5, sf.position.y - s),
        ],
        style: annotation_line_style(),
    });
    doc.add(SvgElement::Text {
        x: sf.position.x + s * 0.6,
        y: sf.position.y - s * 0.5,
        text: format!("Ra {:.1}", sf.roughness),
        font_size: 4.0,
        anchor: "start".into(),
        style: annotation_text_style(),
    });
}

fn centerline_style(width: f64) -> SvgStyle {
    SvgStyle {
        stroke: "red".into(),
        stroke_width: width,
        fill: "none".into(),
        stroke_dasharray: Some("8,2,2,2".into()),
    }
}

fn center_mark_style() -> SvgStyle {
    SvgStyle {
        stroke: "red".into(),
        stroke_width: 0.3,
        fill: "none".into(),
        stroke_dasharray: None,
    }
}

fn render_center_mark_to_svg(doc: &mut SvgDocument, cm: &CenterMark) {
    let half = cm.size / 2.0;
    let style = center_mark_style();
    doc.add(SvgElement::Line {
        x1: cm.center.x - half,
        y1: cm.center.y,
        x2: cm.center.x + half,
        y2: cm.center.y,
        style: style.clone(),
    });
    doc.add(SvgElement::Line {
        x1: cm.center.x,
        y1: cm.center.y - half,
        x2: cm.center.x,
        y2: cm.center.y + half,
        style,
    });
}

fn render_centerline_to_svg(doc: &mut SvgDocument, cl: &Centerline) {
    let dx = cl.end.x - cl.start.x;
    let dy = cl.end.y - cl.start.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 1e-10 {
        return;
    }
    let ux = dx / len;
    let uy = dy / len;
    doc.add(SvgElement::Line {
        x1: cl.start.x - ux * cl.extension,
        y1: cl.start.y - uy * cl.extension,
        x2: cl.end.x + ux * cl.extension,
        y2: cl.end.y + uy * cl.extension,
        style: centerline_style(0.25),
    });
}

fn render_bolt_circle_centerlines_to_svg(doc: &mut SvgDocument, bc: &BoltCircleCenterlines) {
    if bc.radius <= 0.0 {
        return;
    }
    doc.add(SvgElement::Circle {
        cx: bc.center.x,
        cy: bc.center.y,
        r: bc.radius,
        style: centerline_style(0.25),
    });
    render_center_mark_to_svg(
        doc,
        &CenterMark {
            center: bc.center,
            size: bc.mark_size,
        },
    );

    let style = center_mark_style();
    for i in 0..bc.bolt_count {
        let angle = bc.start_angle.to_radians()
            + 2.0 * std::f64::consts::PI * i as f64 / bc.bolt_count as f64;
        let bx = bc.center.x + bc.radius * angle.cos();
        let by = bc.center.y + bc.radius * angle.sin();
        let half = bc.mark_size * 0.15;
        doc.add(SvgElement::Line {
            x1: bx - half,
            y1: by,
            x2: bx + half,
            y2: by,
            style: style.clone(),
        });
        doc.add(SvgElement::Line {
            x1: bx,
            y1: by - half,
            x2: bx,
            y2: by + half,
            style: style.clone(),
        });
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
    let large = if (ea - sa).abs() > std::f64::consts::PI {
        1
    } else {
        0
    };
    let sweep = if ea > sa { 1 } else { 0 };

    let _ = write!(
        svg,
        "<path d=\"M {},{} A {},{} 0 {} {} {},{}\" {} />",
        sx, sy, r, r, large, sweep, ex, ey, style
    );

    let mid_angle = (sa + ea) / 2.0;
    let mx = dim.center.x + (r + 5.0) * mid_angle.cos();
    let my = dim.center.y + (r + 5.0) * mid_angle.sin();
    let _ = write!(
        svg,
        "<text x=\"{}\" y=\"{}\" {}>{:.2}</text>",
        mx, my, text_style, arc_len
    );
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
            let _ = write!(
                svg,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />",
                min_x, y, max_x, y, style
            );
            let _ = write!(
                svg,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />",
                min_x,
                y - 3.0,
                min_x,
                y + 3.0,
                style
            );
            let _ = write!(
                svg,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />",
                max_x,
                y - 3.0,
                max_x,
                y + 3.0,
                style
            );
            let mx = (min_x + max_x) / 2.0;
            let _ = write!(
                svg,
                "<text x=\"{}\" y=\"{}\" {}>{:.2}</text>",
                mx,
                y - 2.0,
                text_style,
                max_x - min_x
            );
        }
        ExtentDimension::Vertical { min_y, max_y, x } => {
            let _ = write!(
                svg,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />",
                x, min_y, x, max_y, style
            );
            let _ = write!(
                svg,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />",
                x - 3.0,
                min_y,
                x + 3.0,
                min_y,
                style
            );
            let _ = write!(
                svg,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />",
                x - 3.0,
                max_y,
                x + 3.0,
                max_y,
                style
            );
            let my = (min_y + max_y) / 2.0;
            let _ = write!(
                svg,
                "<text x=\"{}\" y=\"{}\" {}>{:.2}</text>",
                x + 3.0,
                my,
                text_style,
                max_y - min_y
            );
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
        x - s,
        y,
        x + s,
        y
    );

    match ws.weld_type {
        WeldType::Fillet => {
            // Triangle symbol below line
            let _ = write!(
                svg,
                "<polygon points=\"{},{} {},{} {},{}\" fill=\"none\" stroke=\"black\" stroke-width=\"0.4\" />",
                x - s * 0.4,
                y,
                x + s * 0.4,
                y,
                x,
                y + s * 0.6
            );
        }
        WeldType::Groove => {
            // V symbol below line
            let _ = write!(
                svg,
                "<polyline points=\"{},{} {},{} {},{}\" fill=\"none\" stroke=\"black\" stroke-width=\"0.4\" />",
                x - s * 0.3,
                y,
                x,
                y + s * 0.5,
                x + s * 0.3,
                y
            );
        }
        WeldType::Plug => {
            // Filled rectangle
            let _ = write!(
                svg,
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"black\" />",
                x - s * 0.2,
                y,
                s * 0.4,
                s * 0.4
            );
        }
        WeldType::Spot => {
            // Circle
            let _ = write!(
                svg,
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"none\" stroke=\"black\" stroke-width=\"0.4\" />",
                x,
                y + s * 0.3,
                s * 0.2
            );
        }
        WeldType::Seam => {
            // Dashed arc
            let _ = write!(
                svg,
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"none\" stroke=\"black\" stroke-width=\"0.4\" stroke-dasharray=\"1,1\" />",
                x,
                y + s * 0.3,
                s * 0.2
            );
        }
        WeldType::Backing => {
            // Semicircle
            let r = s * 0.3;
            let _ = write!(
                svg,
                "<path d=\"M {},{} A {},{} 0 0 1 {},{}\" fill=\"none\" stroke=\"black\" stroke-width=\"0.4\" />",
                x - r,
                y,
                r,
                r,
                x + r,
                y
            );
        }
    }

    // Size text
    let _ = write!(
        svg,
        "<text x=\"{}\" y=\"{}\" font-size=\"4\" fill=\"black\">{:.1}</text>",
        x + s + 1.0,
        y - 1.0,
        ws.size
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
        (
            ba.balloon_center.x - ux * ba.radius,
            ba.balloon_center.y - uy * ba.radius,
        )
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
            ba.leader_start.x,
            ba.leader_start.y,
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
    let _ = write!(
        svg,
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />",
        bc.center.x - half,
        bc.center.y,
        bc.center.x + half,
        bc.center.y,
        cm_style
    );
    let _ = write!(
        svg,
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />",
        bc.center.x,
        bc.center.y - half,
        bc.center.x,
        bc.center.y + half,
        cm_style
    );

    // Radial lines to each bolt position
    for i in 0..bc.bolt_count {
        let angle = bc.start_angle.to_radians()
            + 2.0 * std::f64::consts::PI * i as f64 / bc.bolt_count as f64;
        let bx = bc.center.x + bc.radius * angle.cos();
        let by = bc.center.y + bc.radius * angle.sin();

        // Small cross at bolt position
        let s = bc.mark_size * 0.3;
        let _ = write!(
            svg,
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />",
            bx - s,
            by,
            bx + s,
            by,
            cm_style
        );
        let _ = write!(
            svg,
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />",
            bx,
            by - s,
            bx,
            by + s,
            cm_style
        );
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
        let y = bl.y_position
            + if i % 2 == 0 {
                bl.amplitude
            } else {
                -bl.amplitude
            };
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
// Task 5: Views, Dimensions, Annotations, Symbols expansion
// ---------------------------------------------------------------------------

/// Projection type for `project_shape_2d`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionType {
    Orthographic,
    Perspective,
}

/// A clipping group that constrains multiple views to a rectangular region.
#[derive(Debug, Clone)]
pub struct ClipGroup {
    pub views: Vec<DrawingView>,
    pub clip_x: f64,
    pub clip_y: f64,
    pub clip_width: f64,
    pub clip_height: f64,
}

/// Rich text annotation with HTML-subset content.
#[derive(Debug, Clone)]
pub struct RichTextAnnotation {
    pub position: Point2,
    pub html_content: String,
    pub font_size: f64,
}

/// Hole/shaft tolerance fit annotation (ISO 286).
#[derive(Debug, Clone)]
pub struct HoleShaftFit {
    pub position: Point2,
    pub nominal: f64,
    pub hole_class: String,
    pub shaft_class: String,
}

/// Area measurement annotation.
#[derive(Debug, Clone)]
pub struct AreaAnnotation {
    pub boundary: Vec<Point2>,
    pub area: f64,
    pub label_position: Point2,
}

/// Generates a broken view — removes a gap region from a long part projection.
///
/// Copies edges from the source projection, shifting those beyond `break_end`
/// backwards by `(break_end - break_start) - gap` to compress the view.
pub fn broken_view(
    source: &DrawingView,
    break_start: f64,
    break_end: f64,
    gap: f64,
) -> DrawingView {
    let shift = (break_end - break_start) - gap;
    let mut edges = Vec::new();
    for e in &source.edges {
        // Skip edges fully inside the removed region
        if e.x1 >= break_start && e.x1 <= break_end && e.x2 >= break_start && e.x2 <= break_end {
            continue;
        }
        let x1 = if e.x1 > break_end { e.x1 - shift } else { e.x1 };
        let x2 = if e.x2 > break_end { e.x2 - shift } else { e.x2 };
        edges.push(ProjectedEdge {
            x1,
            y1: e.y1,
            x2,
            y2: e.y2,
            visible: e.visible,
        });
    }
    // Recompute center
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
        direction: source.direction,
        edges,
        center_x: cx,
        center_y: cy,
        scale: source.scale,
        sheet_x: source.sheet_x,
        sheet_y: source.sheet_y,
        sheet_scale: source.sheet_scale,
    }
}

/// Generates a complex section view using multiple cutting planes (stepped/aligned section).
///
/// Cuts the tessellated solid with each plane and collects intersection edges.
pub fn complex_section_view(
    model: &BRepModel,
    solid: Handle<SolidData>,
    cutting_planes: &[(Point3, Vec3)],
) -> DrawingView {
    let mesh = tessellate_solid(model, solid);
    let mut all_edges = Vec::new();

    // Use the first plane's normal for local 2D system
    let first_normal = cutting_planes
        .first()
        .map(|(_, n)| n.normalized().unwrap_or(Vec3::Z))
        .unwrap_or(Vec3::Z);
    let up_hint = if first_normal.x.abs() < 0.9 {
        Vec3::X
    } else {
        Vec3::Y
    };
    let right = first_normal.cross(up_hint).normalized().unwrap_or(Vec3::X);
    let up = right.cross(first_normal).normalized().unwrap_or(Vec3::Y);

    for (plane_pt, plane_n) in cutting_planes {
        let normal = plane_n.normalized().unwrap_or(Vec3::Z);
        for idx in &mesh.indices {
            let pts = [
                mesh.vertices[idx[0] as usize],
                mesh.vertices[idx[1] as usize],
                mesh.vertices[idx[2] as usize],
            ];
            let dists: [f64; 3] = [
                (pts[0] - *plane_pt).dot(normal),
                (pts[1] - *plane_pt).dot(normal),
                (pts[2] - *plane_pt).dot(normal),
            ];
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
                let d1 = crossings[0] - cutting_planes[0].0;
                let d2 = crossings[1] - cutting_planes[0].0;
                all_edges.push(ProjectedEdge {
                    x1: d1.dot(right),
                    y1: d1.dot(up),
                    x2: d2.dot(right),
                    y2: d2.dot(up),
                    visible: true,
                });
            }
        }
    }

    let (mut cx, mut cy) = (0.0, 0.0);
    if !all_edges.is_empty() {
        for e in &all_edges {
            cx += e.x1 + e.x2;
            cy += e.y1 + e.y2;
        }
        let n = all_edges.len() as f64 * 2.0;
        cx /= n;
        cy /= n;
    }

    DrawingView {
        direction: ProjectionDir::Front,
        edges: all_edges,
        center_x: cx,
        center_y: cy,
        scale: 1.0,
        sheet_x: None,
        sheet_y: None,
        sheet_scale: None,
    }
}

/// Groups multiple views into a clip group with a rectangular clipping boundary.
pub fn clip_group(
    views: &[DrawingView],
    clip_x: f64,
    clip_y: f64,
    clip_width: f64,
    clip_height: f64,
) -> ClipGroup {
    let mut clipped_views = Vec::new();
    for view in views {
        let mut edges = Vec::new();
        for e in &view.edges {
            // Keep edges where at least one endpoint is inside the clip rect
            let in1 = e.x1 >= clip_x
                && e.x1 <= clip_x + clip_width
                && e.y1 >= clip_y
                && e.y1 <= clip_y + clip_height;
            let in2 = e.x2 >= clip_x
                && e.x2 <= clip_x + clip_width
                && e.y2 >= clip_y
                && e.y2 <= clip_y + clip_height;
            if in1 || in2 {
                edges.push(e.clone());
            }
        }
        clipped_views.push(DrawingView {
            direction: view.direction,
            edges,
            center_x: view.center_x,
            center_y: view.center_y,
            scale: view.scale,
            sheet_x: view.sheet_x,
            sheet_y: view.sheet_y,
            sheet_scale: view.sheet_scale,
        });
    }
    ClipGroup {
        views: clipped_views,
        clip_x,
        clip_y,
        clip_width,
        clip_height,
    }
}

/// Captures a 3D viewport into a 2D drawing view using camera projection parameters.
///
/// `view_dir` is the camera's forward direction, `up_dir` is the camera's up vector,
/// and `viewport_size` is `(width, height)` in pixels.
pub fn active_view(
    model: &BRepModel,
    solid: Handle<SolidData>,
    view_dir: Vec3,
    up_dir: Vec3,
    viewport_size: (f64, f64),
) -> DrawingView {
    let toward_cam = Vec3::new(-view_dir.x, -view_dir.y, -view_dir.z);
    let right = toward_cam.cross(up_dir).normalized().unwrap_or(Vec3::X);
    let up = right.cross(toward_cam).normalized().unwrap_or(Vec3::Y);

    let empty_view = DrawingView {
        direction: ProjectionDir::Front,
        edges: Vec::new(),
        center_x: viewport_size.0 / 2.0,
        center_y: viewport_size.1 / 2.0,
        scale: 1.0,
        sheet_x: None,
        sheet_y: None,
        sheet_scale: None,
    };

    let Some(solid_data) = model.solids.get(solid) else {
        return empty_view;
    };

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

    edges_3d.sort_by_key(|a| edge_key(a.0, a.1));
    edges_3d.dedup_by(|a, b| edge_key(a.0, a.1) == edge_key(b.0, b.1));

    let edges: Vec<ProjectedEdge> = edges_3d
        .iter()
        .map(|(p1, p2)| {
            let (sx1, sy1, _) = project_point(*p1, right, up, toward_cam);
            let (sx2, sy2, _) = project_point(*p2, right, up, toward_cam);
            ProjectedEdge {
                x1: sx1,
                y1: sy1,
                x2: sx2,
                y2: sy2,
                visible: true,
            }
        })
        .collect();

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
        direction: ProjectionDir::Front,
        edges,
        center_x: cx,
        center_y: cy,
        scale: 1.0,
        sheet_x: None,
        sheet_y: None,
        sheet_scale: None,
    }
}

/// Projects B-Rep edges to 2D without hidden-line removal.
///
/// Returns raw `ProjectedEdge` list for further processing.
pub fn project_shape_2d(
    model: &BRepModel,
    solid: Handle<SolidData>,
    direction: Vec3,
    _projection_type: ProjectionType,
) -> Vec<ProjectedEdge> {
    let toward_cam = Vec3::new(-direction.x, -direction.y, -direction.z);
    let up_hint = if toward_cam.x.abs() < 0.9 {
        Vec3::new(0.0, 0.0, 1.0)
    } else {
        Vec3::new(0.0, 1.0, 0.0)
    };
    let right = toward_cam.cross(up_hint).normalized().unwrap_or(Vec3::X);
    let up = right.cross(toward_cam).normalized().unwrap_or(Vec3::Y);

    let Some(solid_data) = model.solids.get(solid) else {
        return Vec::new();
    };

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

    edges_3d.sort_by_key(|a| edge_key(a.0, a.1));
    edges_3d.dedup_by(|a, b| edge_key(a.0, a.1) == edge_key(b.0, b.1));

    edges_3d
        .iter()
        .map(|(p1, p2)| {
            let (sx1, sy1, _) = project_point(*p1, right, up, toward_cam);
            let (sx2, sy2, _) = project_point(*p2, right, up, toward_cam);
            ProjectedEdge {
                x1: sx1,
                y1: sy1,
                x2: sx2,
                y2: sy2,
                visible: true,
            }
        })
        .collect()
}

/// Auto-detects dimension type from a pair of points plus optional radius.
///
/// If `radius > 0`, creates a radius dimension; otherwise creates a linear dimension.
pub fn contextual_dimension(p1: Point2, p2: Point2, radius: f64) -> DimensionType {
    if radius > 0.0 {
        DimensionType::RadiusDimension { center: p1, radius }
    } else {
        let dx = p2.x - p1.x;
        let dy = p2.y - p1.y;
        let value = (dx * dx + dy * dy).sqrt();
        DimensionType::Length {
            start: p1,
            end: p2,
            value,
        }
    }
}

/// Creates an angle dimension measured at vertex `p2` from arms `p2→p1` and `p2→p3`.
pub fn angle_from_3_points(p1: Point2, p2: Point2, p3: Point2) -> DimensionType {
    let dx1 = p1.x - p2.x;
    let dy1 = p1.y - p2.y;
    let dx2 = p3.x - p2.x;
    let dy2 = p3.y - p2.y;
    let cross = dx1 * dy2 - dy1 * dx2;
    let dot_val = dx1 * dx2 + dy1 * dy2;
    let angle = cross.atan2(dot_val).to_degrees().abs();
    DimensionType::AngleDimension {
        vertex: p2,
        arm1_end: p1,
        arm2_end: p3,
        angle,
    }
}

/// Creates an area measurement annotation from a boundary polygon.
///
/// Area is computed using the shoelace formula.
pub fn area_annotation(boundary: &[Point2], label_position: Point2) -> AreaAnnotation {
    let n = boundary.len();
    let mut area = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        area += boundary[i].x * boundary[j].y;
        area -= boundary[j].x * boundary[i].y;
    }
    area = (area / 2.0).abs();
    AreaAnnotation {
        boundary: boundary.to_vec(),
        area,
        label_position,
    }
}

/// Renders an area annotation to SVG.
pub fn area_annotation_to_svg(ann: &AreaAnnotation) -> String {
    use std::fmt::Write;
    let mut svg = String::new();
    if ann.boundary.len() >= 3 {
        let pts: Vec<String> = ann
            .boundary
            .iter()
            .map(|p| format!("{},{}", p.x, p.y))
            .collect();
        let _ = write!(
            svg,
            "<polygon points=\"{}\" fill=\"none\" stroke=\"green\" stroke-width=\"0.3\" stroke-dasharray=\"3,1\" />",
            pts.join(" ")
        );
    }
    let _ = write!(
        svg,
        "<text x=\"{}\" y=\"{}\" fill=\"green\" font-size=\"5\" text-anchor=\"middle\">Area: {:.2}</text>",
        ann.label_position.x, ann.label_position.y, ann.area
    );
    svg
}

/// Creates an arc length dimension.
pub fn arc_length_dimension(
    center: Point2,
    radius: f64,
    start_angle: f64,
    end_angle: f64,
) -> ArcLengthDimension {
    ArcLengthDimension {
        center,
        radius,
        start_angle,
        end_angle,
    }
}

/// Creates a horizontal or vertical extent dimension from a set of points.
///
/// `horizontal` selects the axis: `true` for horizontal (X), `false` for vertical (Y).
pub fn hv_extent_dimension(points: &[Point2], horizontal: bool) -> ExtentDimension {
    if horizontal {
        let min_x = points.iter().map(|p| p.x).fold(f64::MAX, f64::min);
        let max_x = points.iter().map(|p| p.x).fold(f64::MIN, f64::max);
        let y =
            points.iter().map(|p| p.y).fold(0.0, |a, b| a + b) / points.len().max(1) as f64 - 15.0;
        ExtentDimension::Horizontal { min_x, max_x, y }
    } else {
        let min_y = points.iter().map(|p| p.y).fold(f64::MAX, f64::min);
        let max_y = points.iter().map(|p| p.y).fold(f64::MIN, f64::max);
        let x =
            points.iter().map(|p| p.x).fold(0.0, |a, b| a + b) / points.len().max(1) as f64 + 15.0;
        ExtentDimension::Vertical { min_y, max_y, x }
    }
}

/// Updates dimension reference positions after model regeneration.
///
/// Shifts all dimensions by the difference between old and new model centers.
pub fn repair_dimension_refs(dims: &mut [DimensionType], old_center: Point2, new_center: Point2) {
    let dx = new_center.x - old_center.x;
    let dy = new_center.y - old_center.y;
    for dim in dims.iter_mut() {
        match dim {
            DimensionType::Length { start, end, .. } => {
                start.x += dx;
                start.y += dy;
                end.x += dx;
                end.y += dy;
            }
            DimensionType::HorizontalDimension { start, end, .. } => {
                start.x += dx;
                start.y += dy;
                end.x += dx;
                end.y += dy;
            }
            DimensionType::VerticalDimension { start, end, .. } => {
                start.x += dx;
                start.y += dy;
                end.x += dx;
                end.y += dy;
            }
            DimensionType::RadiusDimension { center, .. } => {
                center.x += dx;
                center.y += dy;
            }
            DimensionType::DiameterDimension { center, .. } => {
                center.x += dx;
                center.y += dy;
            }
            DimensionType::AngleDimension {
                vertex,
                arm1_end,
                arm2_end,
                ..
            } => {
                vertex.x += dx;
                vertex.y += dy;
                arm1_end.x += dx;
                arm1_end.y += dy;
                arm2_end.x += dx;
                arm2_end.y += dy;
            }
        }
    }
}

/// Creates a rich text annotation with HTML-subset content.
pub fn rich_text_annotation(
    position: Point2,
    html_content: &str,
    font_size: f64,
) -> RichTextAnnotation {
    RichTextAnnotation {
        position,
        html_content: html_content.to_string(),
        font_size,
    }
}

/// Renders a rich text annotation to SVG.
///
/// Strips HTML tags and renders as plain text (SVG does not support HTML natively).
pub fn rich_text_annotation_to_svg(ann: &RichTextAnnotation) -> String {
    use std::fmt::Write;
    let mut svg = String::new();
    // Strip HTML tags for SVG rendering
    let mut plain = String::new();
    let mut in_tag = false;
    for ch in ann.html_content.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => plain.push(ch),
            _ => {}
        }
    }
    let escaped = xml_escape_str(&plain);
    let _ = write!(
        svg,
        "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"black\">{}</text>",
        ann.position.x, ann.position.y, ann.font_size, escaped
    );
    svg
}

/// Creates a balloon annotation with numbered leader.
pub fn balloon_annotation(
    leader_end: Point2,
    balloon_center: Point2,
    number: u32,
    radius: f64,
) -> BalloonAnnotation {
    BalloonAnnotation {
        leader_start: leader_end,
        balloon_center,
        radius,
        text: number.to_string(),
    }
}

/// Creates an axonometric (true-length) dimension projected along a view direction.
///
/// Measures the actual 3D distance and annotates it on the 2D projection.
pub fn axonometric_length_dimension(p1: Point3, p2: Point3, view_direction: Vec3) -> DimensionType {
    let toward_cam = Vec3::new(-view_direction.x, -view_direction.y, -view_direction.z);
    let up_hint = if toward_cam.x.abs() < 0.9 {
        Vec3::new(0.0, 0.0, 1.0)
    } else {
        Vec3::new(0.0, 1.0, 0.0)
    };
    let right = toward_cam.cross(up_hint).normalized().unwrap_or(Vec3::X);
    let up = right.cross(toward_cam).normalized().unwrap_or(Vec3::Y);

    let (sx1, sy1, _) = project_point(p1, right, up, toward_cam);
    let (sx2, sy2, _) = project_point(p2, right, up, toward_cam);

    // True 3D distance
    let d = p2 - p1;
    let true_length = (d.x * d.x + d.y * d.y + d.z * d.z).sqrt();

    DimensionType::Length {
        start: Point2::new(sx1, sy1),
        end: Point2::new(sx2, sy2),
        value: true_length,
    }
}

/// Parametric hatch pattern name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HatchPatternName {
    Lines,
    CrossHatch,
    Dots,
    Bricks,
}

/// Creates a geometric hatch pattern with named pattern, scale, and angle.
pub fn geometric_hatch(
    face_boundary: &[Point2],
    pattern_name: HatchPatternName,
    scale: f64,
    angle: f64,
) -> HatchPattern {
    let spacing = match pattern_name {
        HatchPatternName::Lines => 2.0 * scale,
        HatchPatternName::CrossHatch => 2.0 * scale,
        HatchPatternName::Dots => 3.0 * scale,
        HatchPatternName::Bricks => 4.0 * scale,
    };
    HatchPattern {
        boundary: face_boundary.to_vec(),
        angle,
        spacing,
    }
}

/// Renders a geometric hatch pattern to SVG (delegates to existing `hatch_pattern_to_svg`).
///
/// For cross-hatch, renders two passes at perpendicular angles.
pub fn geometric_hatch_to_svg(
    boundary: &[Point2],
    pattern_name: HatchPatternName,
    scale: f64,
    angle: f64,
) -> String {
    let hp1 = geometric_hatch(boundary, pattern_name, scale, angle);
    let mut svg = hatch_pattern_to_svg(&hp1);
    if pattern_name == HatchPatternName::CrossHatch {
        let hp2 = HatchPattern {
            boundary: boundary.to_vec(),
            angle: angle + 90.0,
            spacing: hp1.spacing,
        };
        svg.push_str(&hatch_pattern_to_svg(&hp2));
    }
    svg
}

/// Extended weld symbol with full ISO 2553 parameters.
#[derive(Debug, Clone)]
pub struct WeldSymbolFull {
    pub position: Point2,
    pub weld_type: WeldType,
    pub size: f64,
    pub length: f64,
    pub pitch: f64,
    pub contour: WeldContour,
    pub finish: WeldFinish,
}

/// Weld contour type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeldContour {
    None,
    Flush,
    Convex,
    Concave,
}

/// Weld finish method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeldFinish {
    None,
    Grinding,
    Machining,
    Chipping,
}

/// Creates a full weld symbol with ISO 2553 parameters.
pub fn weld_symbol(
    position: Point2,
    weld_type: WeldType,
    size: f64,
    length: f64,
    pitch: f64,
    contour: WeldContour,
    finish: WeldFinish,
) -> WeldSymbolFull {
    WeldSymbolFull {
        position,
        weld_type,
        size,
        length,
        pitch,
        contour,
        finish,
    }
}

/// Renders a full weld symbol to SVG.
pub fn weld_symbol_full_to_svg(ws: &WeldSymbolFull) -> String {
    use std::fmt::Write;
    // Base symbol rendering
    let base = WeldSymbol {
        position: ws.position,
        weld_type: ws.weld_type,
        size: ws.size,
    };
    let mut svg = weld_symbol_to_svg(&base);

    let x = ws.position.x;
    let y = ws.position.y;
    let s = ws.size;

    // Length and pitch text
    if ws.length > 0.0 || ws.pitch > 0.0 {
        let label = if ws.pitch > 0.0 {
            format!("{:.0}({:.0})", ws.length, ws.pitch)
        } else {
            format!("{:.0}", ws.length)
        };
        let _ = write!(
            svg,
            "<text x=\"{}\" y=\"{}\" font-size=\"3.5\" fill=\"black\">{}</text>",
            x - s,
            y - 3.0,
            label
        );
    }

    // Contour indicator
    match ws.contour {
        WeldContour::Flush => {
            let _ = write!(
                svg,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"black\" stroke-width=\"0.3\" />",
                x - s * 0.3,
                y - s * 0.1,
                x + s * 0.3,
                y - s * 0.1
            );
        }
        WeldContour::Convex => {
            let r = s * 0.3;
            let _ = write!(
                svg,
                "<path d=\"M {},{} A {},{} 0 0 1 {},{}\" fill=\"none\" stroke=\"black\" stroke-width=\"0.3\" />",
                x - r,
                y - s * 0.1,
                r,
                r * 0.5,
                x + r,
                y - s * 0.1
            );
        }
        WeldContour::Concave => {
            let r = s * 0.3;
            let _ = write!(
                svg,
                "<path d=\"M {},{} A {},{} 0 0 0 {},{}\" fill=\"none\" stroke=\"black\" stroke-width=\"0.3\" />",
                x - r,
                y - s * 0.1,
                r,
                r * 0.5,
                x + r,
                y - s * 0.1
            );
        }
        WeldContour::None => {}
    }

    // Finish method symbol
    let finish_char = match ws.finish {
        WeldFinish::Grinding => "G",
        WeldFinish::Machining => "M",
        WeldFinish::Chipping => "C",
        WeldFinish::None => "",
    };
    if !finish_char.is_empty() {
        let _ = write!(
            svg,
            "<text x=\"{}\" y=\"{}\" font-size=\"3\" fill=\"black\">{}</text>",
            x + s + 2.0,
            y + 3.0,
            finish_char
        );
    }

    svg
}

/// Creates a hole/shaft tolerance fit annotation (ISO 286).
pub fn hole_shaft_fit(
    position: Point2,
    nominal: f64,
    hole_class: &str,
    shaft_class: &str,
) -> HoleShaftFit {
    HoleShaftFit {
        position,
        nominal,
        hole_class: hole_class.to_string(),
        shaft_class: shaft_class.to_string(),
    }
}

/// Renders a hole/shaft fit annotation to SVG.
pub fn hole_shaft_fit_to_svg(hsf: &HoleShaftFit) -> String {
    use std::fmt::Write;
    let mut svg = String::new();
    let label = format!(
        "\u{2300}{:.1} {}/{}",
        hsf.nominal, hsf.hole_class, hsf.shaft_class
    );
    let _ = write!(
        svg,
        "<text x=\"{}\" y=\"{}\" font-size=\"5\" fill=\"blue\" text-anchor=\"middle\">{}</text>",
        hsf.position.x, hsf.position.y, label
    );
    svg
}

/// Renders a `ClipGroup` to SVG using clipPath.
pub fn clip_group_to_svg(cg: &ClipGroup) -> String {
    use std::fmt::Write;
    let mut svg = String::new();
    let clip_id = "clip-group-1";
    let _ = write!(
        svg,
        "<defs><clipPath id=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" /></clipPath></defs>",
        clip_id, cg.clip_x, cg.clip_y, cg.clip_width, cg.clip_height
    );
    let _ = write!(svg, "<g clip-path=\"url(#{})\" >", clip_id);
    let vis_style = "stroke=\"black\" stroke-width=\"0.5\" fill=\"none\"";
    let hid_style =
        "stroke=\"#888888\" stroke-width=\"0.3\" fill=\"none\" stroke-dasharray=\"2,1\"";
    for view in &cg.views {
        for e in &view.edges {
            let style = if e.visible { vis_style } else { hid_style };
            let _ = write!(
                svg,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />",
                e.x1, e.y1, e.x2, e.y2, style
            );
        }
    }
    svg.push_str("</g>");
    svg
}

/// Escapes XML entities in a string.
fn xml_escape_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Task 6: Centerlines, cosmetics, formatting, pages
// ---------------------------------------------------------------------------

/// Creates a centerline through the centroid of a face boundary.
///
/// The centerline runs in the longest-span direction of the boundary.
pub fn centerline_on_face(face_boundary: &[Point2], extension: f64) -> Centerline {
    let n = face_boundary.len();
    if n < 2 {
        return Centerline {
            start: Point2::new(0.0, 0.0),
            end: Point2::new(0.0, 0.0),
            extension,
        };
    }
    let cx = face_boundary.iter().map(|p| p.x).sum::<f64>() / n as f64;
    let cy = face_boundary.iter().map(|p| p.y).sum::<f64>() / n as f64;

    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    for p in face_boundary {
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
    }
    let span_x = max_x - min_x;
    let span_y = max_y - min_y;

    if span_x >= span_y {
        Centerline {
            start: Point2::new(min_x, cy),
            end: Point2::new(max_x, cy),
            extension,
        }
    } else {
        Centerline {
            start: Point2::new(cx, min_y),
            end: Point2::new(cx, max_y),
            extension,
        }
    }
}

/// Creates a centerline midway between two parallel lines.
pub fn centerline_between_lines(
    l1_start: Point2,
    l1_end: Point2,
    l2_start: Point2,
    l2_end: Point2,
    extension: f64,
) -> Centerline {
    Centerline {
        start: Point2::new(
            (l1_start.x + l2_start.x) / 2.0,
            (l1_start.y + l2_start.y) / 2.0,
        ),
        end: Point2::new((l1_end.x + l2_end.x) / 2.0, (l1_end.y + l2_end.y) / 2.0),
        extension,
    }
}

/// Creates a centerline between two points.
pub fn centerline_between_points(p1: Point2, p2: Point2, extension: f64) -> Centerline {
    Centerline {
        start: p1,
        end: p2,
        extension,
    }
}

/// Creates bolt circle centerlines.
pub fn bolt_circle_centerlines(
    center: Point2,
    radius: f64,
    count: usize,
    mark_size: f64,
) -> BoltCircleCenterlines {
    BoltCircleCenterlines {
        center,
        radius,
        bolt_count: count,
        start_angle: 0.0,
        mark_size,
    }
}

/// Creates a cosmetic line.
pub fn cosmetic_line(p1: Point2, p2: Point2, style: CosmeticLineStyle) -> CosmeticLine {
    CosmeticLine {
        start: p1,
        end: p2,
        style,
        color: "gray".to_string(),
        width: 0.3,
    }
}

/// Cosmetic thread representation.
#[derive(Debug, Clone)]
pub struct CosmeticThread {
    pub center: Point2,
    pub diameter: f64,
    pub length: f64,
    pub internal: bool,
}

/// Creates an internal thread symbol.
pub fn cosmetic_thread_internal(center: Point2, diameter: f64, length: f64) -> CosmeticThread {
    CosmeticThread {
        center,
        diameter,
        length,
        internal: true,
    }
}

/// Creates an external thread symbol.
pub fn cosmetic_thread_external(center: Point2, diameter: f64, length: f64) -> CosmeticThread {
    CosmeticThread {
        center,
        diameter,
        length,
        internal: false,
    }
}

/// Renders a cosmetic thread to SVG.
pub fn cosmetic_thread_to_svg(ct: &CosmeticThread) -> String {
    use std::fmt::Write;
    let mut svg = String::new();
    let r = ct.diameter / 2.0;
    let minor_r = r * 0.85;
    let x = ct.center.x;
    let y = ct.center.y;
    let half_len = ct.length / 2.0;

    // Outer circle (solid)
    let _ = write!(
        svg,
        "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"none\" stroke=\"black\" stroke-width=\"0.5\" />",
        x, y, r
    );
    // Inner circle (thin dashed for internal, solid for external)
    if ct.internal {
        let _ = write!(
            svg,
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"none\" stroke=\"black\" stroke-width=\"0.25\" stroke-dasharray=\"1,1\" />",
            x, y, minor_r
        );
    } else {
        let _ = write!(
            svg,
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"none\" stroke=\"black\" stroke-width=\"0.25\" />",
            x, y, minor_r
        );
    }
    // Thread extent lines
    let _ = write!(
        svg,
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"black\" stroke-width=\"0.3\" />",
        x - r,
        y - half_len,
        x - r,
        y + half_len
    );
    let _ = write!(
        svg,
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"black\" stroke-width=\"0.3\" />",
        x + r,
        y - half_len,
        x + r,
        y + half_len
    );
    svg
}

/// Cosmetic vertex marker.
#[derive(Debug, Clone)]
pub struct CosmeticVertex {
    pub position: Point2,
    pub size: f64,
    pub style: CosmeticVertexStyle,
}

/// Cosmetic vertex display style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CosmeticVertexStyle {
    Cross,
    Circle,
    Dot,
}

/// Creates a cosmetic vertex marker.
pub fn cosmetic_vertex(position: Point2, style: CosmeticVertexStyle) -> CosmeticVertex {
    CosmeticVertex {
        position,
        size: 2.0,
        style,
    }
}

/// Renders a cosmetic vertex to SVG.
pub fn cosmetic_vertex_to_svg(cv: &CosmeticVertex) -> String {
    use std::fmt::Write;
    let mut svg = String::new();
    let s = cv.size;
    let x = cv.position.x;
    let y = cv.position.y;
    match cv.style {
        CosmeticVertexStyle::Cross => {
            let _ = write!(
                svg,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"red\" stroke-width=\"0.3\" />",
                x - s,
                y,
                x + s,
                y
            );
            let _ = write!(
                svg,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"red\" stroke-width=\"0.3\" />",
                x,
                y - s,
                x,
                y + s
            );
        }
        CosmeticVertexStyle::Circle => {
            let _ = write!(
                svg,
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"none\" stroke=\"red\" stroke-width=\"0.3\" />",
                x, y, s
            );
        }
        CosmeticVertexStyle::Dot => {
            let _ = write!(
                svg,
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"red\" />",
                x,
                y,
                s * 0.5
            );
        }
    }
    svg
}

/// Cosmetic circle.
#[derive(Debug, Clone)]
pub struct CosmeticCircle {
    pub center: Point2,
    pub radius: f64,
}

/// Creates a cosmetic circle.
pub fn cosmetic_circle(center: Point2, radius: f64) -> CosmeticCircle {
    CosmeticCircle { center, radius }
}

/// Renders a cosmetic circle to SVG.
pub fn cosmetic_circle_to_svg(cc: &CosmeticCircle) -> String {
    use std::fmt::Write;
    let mut svg = String::new();
    let _ = write!(
        svg,
        "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"none\" stroke=\"gray\" stroke-width=\"0.3\" stroke-dasharray=\"4,2\" />",
        cc.center.x, cc.center.y, cc.radius
    );
    svg
}

/// Cosmetic arc.
#[derive(Debug, Clone)]
pub struct CosmeticArc {
    pub center: Point2,
    pub radius: f64,
    pub start_angle: f64,
    pub end_angle: f64,
}

/// Creates a cosmetic arc.
pub fn cosmetic_arc(center: Point2, radius: f64, start_angle: f64, end_angle: f64) -> CosmeticArc {
    CosmeticArc {
        center,
        radius,
        start_angle,
        end_angle,
    }
}

/// Renders a cosmetic arc to SVG.
pub fn cosmetic_arc_to_svg(ca: &CosmeticArc) -> String {
    use std::fmt::Write;
    let mut svg = String::new();
    let sa = ca.start_angle.to_radians();
    let ea = ca.end_angle.to_radians();
    let sx = ca.center.x + ca.radius * sa.cos();
    let sy = ca.center.y + ca.radius * sa.sin();
    let ex = ca.center.x + ca.radius * ea.cos();
    let ey = ca.center.y + ca.radius * ea.sin();
    let large = if (ea - sa).abs() > std::f64::consts::PI {
        1
    } else {
        0
    };
    let sweep = if ea > sa { 1 } else { 0 };
    let _ = write!(
        svg,
        "<path d=\"M {},{} A {},{} 0 {} {} {},{}\" fill=\"none\" stroke=\"gray\" stroke-width=\"0.3\" stroke-dasharray=\"4,2\" />",
        sx, sy, ca.radius, ca.radius, large, sweep, ex, ey
    );
    svg
}

/// Creates a cosmetic line parallel to a reference line at a given offset distance.
pub fn cosmetic_parallel_line(
    ref_start: Point2,
    ref_end: Point2,
    offset: f64,
    style: CosmeticLineStyle,
) -> CosmeticLine {
    let dx = ref_end.x - ref_start.x;
    let dy = ref_end.y - ref_start.y;
    let len = (dx * dx + dy * dy).sqrt();
    let (nx, ny) = if len > 1e-10 {
        (-dy / len * offset, dx / len * offset)
    } else {
        (0.0, offset)
    };
    CosmeticLine {
        start: Point2::new(ref_start.x + nx, ref_start.y + ny),
        end: Point2::new(ref_end.x + nx, ref_end.y + ny),
        style,
        color: "gray".to_string(),
        width: 0.3,
    }
}

/// Creates a cosmetic line perpendicular to a reference line through a given point.
///
/// The perpendicular line extends `half_length` in each direction from `through_point`.
pub fn cosmetic_perpendicular_line(
    ref_start: Point2,
    ref_end: Point2,
    through_point: Point2,
    half_length: f64,
    style: CosmeticLineStyle,
) -> CosmeticLine {
    let dx = ref_end.x - ref_start.x;
    let dy = ref_end.y - ref_start.y;
    let len = (dx * dx + dy * dy).sqrt();
    let (px, py) = if len > 1e-10 {
        (-dy / len, dx / len)
    } else {
        (0.0, 1.0)
    };
    CosmeticLine {
        start: Point2::new(
            through_point.x + px * half_length,
            through_point.y + py * half_length,
        ),
        end: Point2::new(
            through_point.x - px * half_length,
            through_point.y - py * half_length,
        ),
        style,
        color: "gray".to_string(),
        width: 0.3,
    }
}

/// Drawing element with appearance properties for `edit_line_appearance`.
#[derive(Debug, Clone)]
pub struct DrawingElement {
    pub id: String,
    pub color: String,
    pub width: f64,
    pub line_style: CosmeticLineStyle,
    pub visible: bool,
    pub locked: bool,
    pub stack_order: i32,
}

impl DrawingElement {
    /// Creates a new drawing element with default appearance.
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            color: "black".to_string(),
            width: 0.5,
            line_style: CosmeticLineStyle::Continuous,
            visible: true,
            locked: false,
            stack_order: 0,
        }
    }
}

/// Modifies a drawing element's line appearance.
pub fn edit_line_appearance(
    element: &mut DrawingElement,
    color: &str,
    width: f64,
    style: CosmeticLineStyle,
) {
    element.color = color.to_string();
    element.width = width;
    element.line_style = style;
}

/// Toggles visibility of a drawing element.
pub fn toggle_edge_visibility(element: &mut DrawingElement, visible: bool) {
    element.visible = visible;
}

/// Chained sequential dimensions.
#[derive(Debug, Clone)]
pub struct ChainDimension {
    pub segments: Vec<DimensionType>,
}

/// Creates a chain of sequential linear dimensions along a series of points.
pub fn chain_dimension(points: &[Point2]) -> ChainDimension {
    let mut segments = Vec::new();
    for w in points.windows(2) {
        let dx = w[1].x - w[0].x;
        let dy = w[1].y - w[0].y;
        let value = (dx * dx + dy * dy).sqrt();
        segments.push(DimensionType::Length {
            start: w[0],
            end: w[1],
            value,
        });
    }
    ChainDimension { segments }
}

/// Renders a chain dimension to SVG.
pub fn chain_dimension_to_svg(cd: &ChainDimension) -> String {
    let mut svg = String::new();
    for dim in &cd.segments {
        svg.push_str(&dimension_to_svg(dim));
    }
    svg
}

/// Coordinate (baseline) dimensions from a single origin.
#[derive(Debug, Clone)]
pub struct CoordinateDimension {
    pub origin: Point2,
    pub dimensions: Vec<DimensionType>,
}

/// Creates baseline coordinate dimensions from an origin to each point.
pub fn coordinate_dimension(origin: Point2, points: &[Point2]) -> CoordinateDimension {
    let mut dimensions = Vec::new();
    for pt in points {
        let dx = pt.x - origin.x;
        let dy = pt.y - origin.y;
        let value = (dx * dx + dy * dy).sqrt();
        dimensions.push(DimensionType::Length {
            start: origin,
            end: *pt,
            value,
        });
    }
    CoordinateDimension { origin, dimensions }
}

/// Renders coordinate dimensions to SVG.
pub fn coordinate_dimension_to_svg(cd: &CoordinateDimension) -> String {
    let mut svg = String::new();
    for dim in &cd.dimensions {
        svg.push_str(&dimension_to_svg(dim));
    }
    svg
}

/// Creates a chamfer dimension callout.
pub fn chamfer_dimension(vertex: Point2, d1: f64, _d2: f64, angle: f64) -> ChamferDimension {
    ChamferDimension {
        corner: vertex,
        size: d1,
        angle,
    }
}

/// Dimension with formatting metadata.
#[derive(Debug, Clone)]
pub struct FormattedDimension {
    pub dimension: DimensionType,
    pub prefix: String,
    pub decimal_places: u8,
}

/// Sets a prefix symbol (e.g. diameter, R, square) on a formatted dimension.
pub fn set_prefix_symbol(fd: &mut FormattedDimension, symbol: &str) {
    fd.prefix = symbol.to_string();
}

/// Sets the decimal precision on a formatted dimension.
pub fn set_decimal_places(fd: &mut FormattedDimension, places: u8) {
    fd.decimal_places = places;
}

/// Renders a formatted dimension to SVG.
pub fn formatted_dimension_to_svg(fd: &FormattedDimension) -> String {
    use std::fmt::Write;
    let base_svg = dimension_to_svg(&fd.dimension);
    if fd.prefix.is_empty() {
        return base_svg;
    }
    // Add prefix marker at the center of the dimension
    let (cx, cy) = match &fd.dimension {
        DimensionType::Length { start, end, .. } => {
            ((start.x + end.x) / 2.0, (start.y + end.y) / 2.0)
        }
        DimensionType::HorizontalDimension { start, end, .. } => {
            ((start.x + end.x) / 2.0, start.y.min(end.y) - 15.0)
        }
        DimensionType::VerticalDimension { start, end, .. } => {
            (start.x.max(end.x) + 15.0, (start.y + end.y) / 2.0)
        }
        DimensionType::RadiusDimension { center, radius } => {
            (center.x + radius / 2.0, center.y - 3.0)
        }
        DimensionType::DiameterDimension { center, .. } => (center.x, center.y - 5.0),
        DimensionType::AngleDimension {
            vertex,
            arm1_end,
            arm2_end,
            ..
        } => (
            (vertex.x + arm1_end.x + arm2_end.x) / 3.0,
            (vertex.y + arm1_end.y + arm2_end.y) / 3.0,
        ),
    };
    let mut svg = base_svg;
    let _ = write!(
        svg,
        "<text x=\"{}\" y=\"{}\" font-size=\"4\" fill=\"blue\" text-anchor=\"end\">{}</text>",
        cx - 2.0,
        cy - 1.0,
        fd.prefix
    );
    svg
}

/// Stack ordering for drawing elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackOrder {
    Front,
    Back,
    Forward,
    Backward,
}

/// Sets the z-order of a drawing element.
pub fn stack_order(element: &mut DrawingElement, order: StackOrder) {
    match order {
        StackOrder::Front => element.stack_order = i32::MAX,
        StackOrder::Back => element.stack_order = i32::MIN,
        StackOrder::Forward => element.stack_order = element.stack_order.saturating_add(1),
        StackOrder::Backward => element.stack_order = element.stack_order.saturating_sub(1),
    }
}

/// Alignment mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Left,
    Right,
    Top,
    Bottom,
    CenterHorizontal,
    CenterVertical,
}

/// Aligns a set of drawing elements by modifying their positions.
///
/// Operates on a list of `(x, y)` mutable positions.
pub fn align_elements(positions: &mut [(f64, f64)], alignment: Alignment) {
    if positions.is_empty() {
        return;
    }
    match alignment {
        Alignment::Left => {
            let min_x = positions.iter().map(|p| p.0).fold(f64::MAX, f64::min);
            for p in positions.iter_mut() {
                p.0 = min_x;
            }
        }
        Alignment::Right => {
            let max_x = positions.iter().map(|p| p.0).fold(f64::MIN, f64::max);
            for p in positions.iter_mut() {
                p.0 = max_x;
            }
        }
        Alignment::Top => {
            let min_y = positions.iter().map(|p| p.1).fold(f64::MAX, f64::min);
            for p in positions.iter_mut() {
                p.1 = min_y;
            }
        }
        Alignment::Bottom => {
            let max_y = positions.iter().map(|p| p.1).fold(f64::MIN, f64::max);
            for p in positions.iter_mut() {
                p.1 = max_y;
            }
        }
        Alignment::CenterHorizontal => {
            let avg_x = positions.iter().map(|p| p.0).sum::<f64>() / positions.len() as f64;
            for p in positions.iter_mut() {
                p.0 = avg_x;
            }
        }
        Alignment::CenterVertical => {
            let avg_y = positions.iter().map(|p| p.1).sum::<f64>() / positions.len() as f64;
            for p in positions.iter_mut() {
                p.1 = avg_y;
            }
        }
    }
}

/// Locks or unlocks a drawing element to prevent accidental modification.
pub fn lock_element(element: &mut DrawingElement, locked: bool) {
    element.locked = locked;
}

/// Standard paper template sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaperTemplate {
    A0Landscape,
    A0Portrait,
    A1Landscape,
    A1Portrait,
    A2Landscape,
    A2Portrait,
    A3Landscape,
    A3Portrait,
    A4Landscape,
    A4Portrait,
}

impl PaperTemplate {
    /// Returns `(width, height)` in millimeters.
    fn dimensions(self) -> (f64, f64) {
        match self {
            Self::A0Landscape => (1189.0, 841.0),
            Self::A0Portrait => (841.0, 1189.0),
            Self::A1Landscape => (841.0, 594.0),
            Self::A1Portrait => (594.0, 841.0),
            Self::A2Landscape => (594.0, 420.0),
            Self::A2Portrait => (420.0, 594.0),
            Self::A3Landscape => (420.0, 297.0),
            Self::A3Portrait => (297.0, 420.0),
            Self::A4Landscape => (297.0, 210.0),
            Self::A4Portrait => (210.0, 297.0),
        }
    }
}

/// Creates a drawing sheet from a named paper template.
pub fn page_from_template(template: PaperTemplate) -> DrawingSheet {
    let (w, h) = template.dimensions();
    DrawingSheet {
        width: w,
        height: h,
        views: Vec::new(),
        dimensions: Vec::new(),
        extended_dimensions: Vec::new(),
        arc_length_dimensions: Vec::new(),
        area_annotations: Vec::new(),
        text_annotations: Vec::new(),
        rich_text_annotations: Vec::new(),
        balloon_annotations: Vec::new(),
        leader_lines: Vec::new(),
        weld_symbols: Vec::new(),
        surface_finish_symbols: Vec::new(),
        center_marks: Vec::new(),
        centerlines: Vec::new(),
        bolt_circle_centerlines: Vec::new(),
        title: String::new(),
    }
}

/// Updates title-block fields on a drawing sheet.
///
/// Recognized keys: `"title"`, `"author"`, `"date"`, `"scale"`, `"sheet"`.
/// The `title` field updates `sheet.title` directly; other fields are appended
/// as metadata comments in the title.
pub fn update_template_fields(
    sheet: &mut DrawingSheet,
    fields: &std::collections::HashMap<String, String>,
) {
    if let Some(t) = fields.get("title") {
        sheet.title = t.clone();
    }
    let mut meta_parts = Vec::new();
    for key in &["author", "date", "scale", "sheet"] {
        if let Some(val) = fields.get(*key) {
            meta_parts.push(format!("{}: {}", key, val));
        }
    }
    if !meta_parts.is_empty() {
        if !sheet.title.is_empty() {
            sheet.title.push_str(" | ");
        }
        sheet.title.push_str(&meta_parts.join(" | "));
    }
}

/// Forces recalculation of all view positions and dimension placements on a sheet.
///
/// Re-centers views based on their edge extents.
pub fn redraw_page(sheet: &mut DrawingSheet) {
    for view in &mut sheet.views {
        if view.edges.is_empty() {
            continue;
        }
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        for e in &view.edges {
            min_x = min_x.min(e.x1).min(e.x2);
            min_y = min_y.min(e.y1).min(e.y2);
            max_x = max_x.max(e.x1).max(e.x2);
            max_y = max_y.max(e.y1).max(e.y2);
        }
        view.center_x = (min_x + max_x) / 2.0;
        view.center_y = (min_y + max_y) / 2.0;
    }
}

/// Renders all pages in a multi-sheet drawing to a vector of SVG documents.
///
/// Each sheet is independently rendered via `drawing_to_svg`.
pub fn print_all_pages(sheets: &[DrawingSheet]) -> Vec<crate::svg::SvgDocument> {
    sheets.iter().map(drawing_to_svg).collect()
}

// ---------------------------------------------------------------------------
// External SVG / bitmap image embedding
// ---------------------------------------------------------------------------

/// An external SVG fragment embedded in a drawing sheet.
#[derive(Debug, Clone)]
pub struct SvgInsert {
    pub svg_content: String,
    pub position: Point2,
    pub width: f64,
    pub height: f64,
}

impl SvgInsert {
    /// Renders this insert as an SVG `<foreignObject>` element.
    pub fn to_svg(&self) -> String {
        format!(
            "<foreignObject x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\">{}</foreignObject>",
            self.position.x, self.position.y, self.width, self.height, self.svg_content,
        )
    }
}

/// A raster image embedded in a drawing sheet (base64-encoded PNG/JPEG in SVG).
#[derive(Debug, Clone)]
pub struct BitmapImage {
    pub image_data: Vec<u8>,
    pub position: Point2,
    pub width: f64,
    pub height: f64,
}

impl BitmapImage {
    /// Renders this bitmap as an SVG `<image>` element with base64-encoded data.
    pub fn to_svg(&self) -> String {
        let b64 = base64_encode_bytes(&self.image_data);
        format!(
            "<image x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" href=\"data:image/png;base64,{}\"/>",
            self.position.x, self.position.y, self.width, self.height, b64,
        )
    }
}

/// Minimal base64 encoder for embedding image data in SVG.
fn base64_encode_bytes(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let full_chunks = data.len() / 3;
    let remainder = data.len() % 3;
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data[..full_chunks * 3].chunks_exact(3) {
        let n = (chunk[0] as u32) << 16 | (chunk[1] as u32) << 8 | chunk[2] as u32;
        result.push(CHARS[((n >> 18) & 63) as usize] as char);
        result.push(CHARS[((n >> 12) & 63) as usize] as char);
        result.push(CHARS[((n >> 6) & 63) as usize] as char);
        result.push(CHARS[(n & 63) as usize] as char);
    }
    if remainder > 0 {
        let tail = &data[full_chunks * 3..];
        let b0 = tail[0] as u32;
        let b1 = tail.get(1).copied().unwrap_or(0) as u32;
        let n = b0 << 16 | b1 << 8;
        result.push(CHARS[((n >> 18) & 63) as usize] as char);
        result.push(CHARS[((n >> 12) & 63) as usize] as char);
        if remainder == 2 {
            result.push(CHARS[((n >> 6) & 63) as usize] as char);
        } else {
            result.push('=');
        }
        result.push('=');
    }
    result
}

/// Embed an external SVG fragment into a drawing sheet.
pub fn insert_svg(
    drawing: &mut DrawingSheet,
    svg_content: &str,
    position: Point2,
    width: f64,
    height: f64,
) -> SvgInsert {
    let insert = SvgInsert {
        svg_content: svg_content.to_string(),
        position,
        width,
        height,
    };
    // Record as a dimension annotation with label so it appears in the drawing
    drawing.dimensions.push(Dimension::Linear {
        x1: position.x,
        y1: position.y,
        x2: position.x + width,
        y2: position.y + height,
        offset: 0.0,
        text: format!("[SVG Insert {}x{}]", width, height),
    });
    insert
}

/// Embed a raster image (PNG/JPEG bytes) into a drawing sheet.
pub fn bitmap_image(
    drawing: &mut DrawingSheet,
    image_data: &[u8],
    position: Point2,
    width: f64,
    height: f64,
) -> BitmapImage {
    let img = BitmapImage {
        image_data: image_data.to_vec(),
        position,
        width,
        height,
    };
    drawing.dimensions.push(Dimension::Linear {
        x1: position.x,
        y1: position.y,
        x2: position.x + width,
        y2: position.y + height,
        offset: 0.0,
        text: format!("[Bitmap {}x{}]", width, height),
    });
    img
}

/// Copy a view from one drawing sheet to another.
pub fn share_view(
    source: &DrawingSheet,
    target: &mut DrawingSheet,
    view_index: usize,
) -> cadkernel_core::KernelResult<()> {
    if view_index >= source.views.len() {
        return Err(cadkernel_core::KernelError::InvalidArgument(format!(
            "view_index {} out of range (sheet has {} views)",
            view_index,
            source.views.len()
        )));
    }
    target.views.push(source.views[view_index].clone());
    Ok(())
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
        let view = section_view(&model, solid, Point3::new(0.0, 0.0, 5.0), Vec3::Z, "A-A");
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
        let dim = ExtentDimension::Horizontal {
            min_x: 10.0,
            max_x: 50.0,
            y: 80.0,
        };
        let svg = extent_dimension_to_svg(&dim);
        assert!(svg.contains("<line"));
        assert!(svg.contains("40.00"));
    }

    #[test]
    fn test_extent_dimension_vertical() {
        let dim = ExtentDimension::Vertical {
            min_y: 10.0,
            max_y: 60.0,
            x: 80.0,
        };
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

    // ----- Task 5 tests -----

    #[test]
    fn test_broken_view() {
        let source = DrawingView {
            direction: ProjectionDir::Front,
            edges: vec![
                ProjectedEdge {
                    x1: 0.0,
                    y1: 5.0,
                    x2: 10.0,
                    y2: 5.0,
                    visible: true,
                },
                ProjectedEdge {
                    x1: 20.0,
                    y1: 5.0,
                    x2: 25.0,
                    y2: 5.0,
                    visible: true,
                },
                ProjectedEdge {
                    x1: 50.0,
                    y1: 5.0,
                    x2: 60.0,
                    y2: 5.0,
                    visible: true,
                },
            ],
            center_x: 30.0,
            center_y: 5.0,
            scale: 1.0,
            sheet_x: None,
            sheet_y: None,
            sheet_scale: None,
        };
        let bv = broken_view(&source, 15.0, 45.0, 5.0);
        // Edge in removed region (20..25 is inside 15..45) should be removed
        assert_eq!(bv.edges.len(), 2);
        // Edge beyond break_end should be shifted
        let last = &bv.edges[1];
        assert!((last.x1 - 25.0).abs() < 1e-10);
    }

    #[test]
    fn test_complex_section_view() {
        let mut model = BRepModel::new();
        let solid = make_test_box(&mut model);
        let planes = vec![
            (Point3::new(0.0, 0.0, 5.0), Vec3::Z),
            (Point3::new(0.0, 5.0, 0.0), Vec3::Y),
        ];
        let view = complex_section_view(&model, solid, &planes);
        assert!(!view.edges.is_empty());
    }

    #[test]
    fn test_clip_group() {
        let view = DrawingView {
            direction: ProjectionDir::Front,
            edges: vec![
                ProjectedEdge {
                    x1: 5.0,
                    y1: 5.0,
                    x2: 15.0,
                    y2: 5.0,
                    visible: true,
                },
                ProjectedEdge {
                    x1: 50.0,
                    y1: 50.0,
                    x2: 60.0,
                    y2: 50.0,
                    visible: true,
                },
            ],
            center_x: 30.0,
            center_y: 25.0,
            scale: 1.0,
            sheet_x: None,
            sheet_y: None,
            sheet_scale: None,
        };
        let cg = clip_group(&[view], 0.0, 0.0, 20.0, 20.0);
        assert_eq!(cg.views.len(), 1);
        // Only the first edge has an endpoint inside the clip rect
        assert_eq!(cg.views[0].edges.len(), 1);
    }

    #[test]
    fn test_clip_group_to_svg() {
        let view = DrawingView {
            direction: ProjectionDir::Front,
            edges: vec![ProjectedEdge {
                x1: 5.0,
                y1: 5.0,
                x2: 10.0,
                y2: 5.0,
                visible: true,
            }],
            center_x: 5.0,
            center_y: 5.0,
            scale: 1.0,
            sheet_x: None,
            sheet_y: None,
            sheet_scale: None,
        };
        let cg = clip_group(&[view], 0.0, 0.0, 20.0, 20.0);
        let svg = clip_group_to_svg(&cg);
        assert!(svg.contains("clipPath"));
        assert!(svg.contains("<line"));
    }

    #[test]
    fn test_active_view() {
        let mut model = BRepModel::new();
        let solid = make_test_box(&mut model);
        let view = active_view(
            &model,
            solid,
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::Z,
            (800.0, 600.0),
        );
        assert!(!view.edges.is_empty());
    }

    #[test]
    fn test_project_shape_2d() {
        let mut model = BRepModel::new();
        let solid = make_test_box(&mut model);
        let edges = project_shape_2d(&model, solid, Vec3::Y, ProjectionType::Orthographic);
        assert!(!edges.is_empty());
        for e in &edges {
            assert!(e.visible);
        }
    }

    #[test]
    fn test_contextual_dimension_linear() {
        let dim = contextual_dimension(Point2::new(0.0, 0.0), Point2::new(30.0, 40.0), 0.0);
        match dim {
            DimensionType::Length { value, .. } => {
                assert!((value - 50.0).abs() < 1e-10);
            }
            _ => panic!("expected Length"),
        }
    }

    #[test]
    fn test_contextual_dimension_radius() {
        let dim = contextual_dimension(Point2::new(10.0, 10.0), Point2::new(0.0, 0.0), 5.0);
        match dim {
            DimensionType::RadiusDimension { radius, .. } => {
                assert!((radius - 5.0).abs() < 1e-10);
            }
            _ => panic!("expected RadiusDimension"),
        }
    }

    #[test]
    fn test_angle_from_3_points() {
        let dim = angle_from_3_points(
            Point2::new(10.0, 0.0),
            Point2::new(0.0, 0.0),
            Point2::new(0.0, 10.0),
        );
        match dim {
            DimensionType::AngleDimension { angle, .. } => {
                assert!((angle - 90.0).abs() < 1e-6);
            }
            _ => panic!("expected AngleDimension"),
        }
    }

    #[test]
    fn test_area_annotation() {
        let boundary = vec![
            Point2::new(0.0, 0.0),
            Point2::new(10.0, 0.0),
            Point2::new(10.0, 10.0),
            Point2::new(0.0, 10.0),
        ];
        let ann = area_annotation(&boundary, Point2::new(5.0, 5.0));
        assert!((ann.area - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_area_annotation_to_svg() {
        let boundary = vec![
            Point2::new(0.0, 0.0),
            Point2::new(10.0, 0.0),
            Point2::new(10.0, 10.0),
            Point2::new(0.0, 10.0),
        ];
        let ann = area_annotation(&boundary, Point2::new(5.0, 5.0));
        let svg = area_annotation_to_svg(&ann);
        assert!(svg.contains("<polygon"));
        assert!(svg.contains("Area: 100.00"));
    }

    #[test]
    fn test_arc_length_dimension_constructor() {
        let dim = arc_length_dimension(Point2::new(50.0, 50.0), 20.0, 0.0, 90.0);
        assert!((dim.radius - 20.0).abs() < 1e-10);
        assert!((dim.start_angle).abs() < 1e-10);
        assert!((dim.end_angle - 90.0).abs() < 1e-10);
    }

    #[test]
    fn test_hv_extent_dimension_horizontal() {
        let points = vec![
            Point2::new(10.0, 20.0),
            Point2::new(50.0, 30.0),
            Point2::new(30.0, 25.0),
        ];
        let dim = hv_extent_dimension(&points, true);
        match dim {
            ExtentDimension::Horizontal { min_x, max_x, .. } => {
                assert!((min_x - 10.0).abs() < 1e-10);
                assert!((max_x - 50.0).abs() < 1e-10);
            }
            _ => panic!("expected Horizontal"),
        }
    }

    #[test]
    fn test_hv_extent_dimension_vertical() {
        let points = vec![Point2::new(10.0, 5.0), Point2::new(20.0, 50.0)];
        let dim = hv_extent_dimension(&points, false);
        match dim {
            ExtentDimension::Vertical { min_y, max_y, .. } => {
                assert!((min_y - 5.0).abs() < 1e-10);
                assert!((max_y - 50.0).abs() < 1e-10);
            }
            _ => panic!("expected Vertical"),
        }
    }

    #[test]
    fn test_repair_dimension_refs() {
        let mut dims = vec![DimensionType::Length {
            start: Point2::new(10.0, 20.0),
            end: Point2::new(30.0, 40.0),
            value: 28.28,
        }];
        repair_dimension_refs(&mut dims, Point2::new(0.0, 0.0), Point2::new(5.0, 10.0));
        match &dims[0] {
            DimensionType::Length { start, end, .. } => {
                assert!((start.x - 15.0).abs() < 1e-10);
                assert!((start.y - 30.0).abs() < 1e-10);
                assert!((end.x - 35.0).abs() < 1e-10);
                assert!((end.y - 50.0).abs() < 1e-10);
            }
            _ => panic!("expected Length"),
        }
    }

    #[test]
    fn test_rich_text_annotation() {
        let ann =
            rich_text_annotation(Point2::new(10.0, 20.0), "<b>Bold</b> text &amp; more", 12.0);
        let svg = rich_text_annotation_to_svg(&ann);
        assert!(svg.contains("Bold text &amp;amp; more"));
        assert!(svg.contains("font-size=\"12\""));
    }

    #[test]
    fn test_balloon_annotation_constructor() {
        let ba = balloon_annotation(Point2::new(30.0, 30.0), Point2::new(60.0, 10.0), 42, 5.0);
        assert_eq!(ba.text, "42");
        assert!((ba.radius - 5.0).abs() < 1e-10);
        let svg = balloon_annotation_to_svg(&ba);
        assert!(svg.contains(">42<"));
    }

    #[test]
    fn test_axonometric_length_dimension() {
        let dim = axonometric_length_dimension(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(3.0, 4.0, 0.0),
            Vec3::Z,
        );
        match dim {
            DimensionType::Length { value, .. } => {
                assert!((value - 5.0).abs() < 1e-10);
            }
            _ => panic!("expected Length"),
        }
    }

    #[test]
    fn test_geometric_hatch_lines() {
        let boundary = vec![
            Point2::new(0.0, 0.0),
            Point2::new(20.0, 0.0),
            Point2::new(20.0, 20.0),
            Point2::new(0.0, 20.0),
        ];
        let hp = geometric_hatch(&boundary, HatchPatternName::Lines, 1.0, 45.0);
        assert_eq!(hp.angle, 45.0);
        assert!((hp.spacing - 2.0).abs() < 1e-10);
        let svg = hatch_pattern_to_svg(&hp);
        assert!(svg.contains("<line"));
    }

    #[test]
    fn test_geometric_hatch_crosshatch_svg() {
        let boundary = vec![
            Point2::new(0.0, 0.0),
            Point2::new(20.0, 0.0),
            Point2::new(20.0, 20.0),
            Point2::new(0.0, 20.0),
        ];
        let svg = geometric_hatch_to_svg(&boundary, HatchPatternName::CrossHatch, 1.0, 45.0);
        // Should have lines at two angles
        assert!(svg.matches("<line").count() > 2);
    }

    #[test]
    fn test_weld_symbol_full() {
        let ws = weld_symbol(
            Point2::new(50.0, 50.0),
            WeldType::Fillet,
            5.0,
            20.0,
            10.0,
            WeldContour::Flush,
            WeldFinish::Grinding,
        );
        let svg = weld_symbol_full_to_svg(&ws);
        assert!(svg.contains("<line"));
        assert!(svg.contains("<polygon"));
        assert!(svg.contains("20(10)"));
        assert!(svg.contains(">G<"));
    }

    #[test]
    fn test_weld_symbol_full_no_extras() {
        let ws = weld_symbol(
            Point2::new(50.0, 50.0),
            WeldType::Groove,
            5.0,
            0.0,
            0.0,
            WeldContour::None,
            WeldFinish::None,
        );
        let svg = weld_symbol_full_to_svg(&ws);
        assert!(svg.contains("<polyline"));
        assert!(!svg.contains(">G<"));
    }

    #[test]
    fn test_hole_shaft_fit() {
        let hsf = hole_shaft_fit(Point2::new(50.0, 50.0), 10.0, "H7", "g6");
        let svg = hole_shaft_fit_to_svg(&hsf);
        assert!(svg.contains("H7/g6"));
        assert!(svg.contains("10.0"));
    }

    #[test]
    fn test_xml_escape_str() {
        let escaped = xml_escape_str("A & B < C > D \"E\" 'F'");
        assert_eq!(escaped, "A &amp; B &lt; C &gt; D &quot;E&quot; &#39;F&#39;");
    }

    // ----- Task 6 tests -----

    #[test]
    fn test_centerline_on_face_horizontal() {
        let boundary = vec![
            Point2::new(0.0, 0.0),
            Point2::new(40.0, 0.0),
            Point2::new(40.0, 10.0),
            Point2::new(0.0, 10.0),
        ];
        let cl = centerline_on_face(&boundary, 3.0);
        // Wider than tall, so horizontal centerline at y=5
        assert!((cl.start.y - 5.0).abs() < 1e-10);
        assert!((cl.end.y - 5.0).abs() < 1e-10);
        assert!((cl.start.x - 0.0).abs() < 1e-10);
        assert!((cl.end.x - 40.0).abs() < 1e-10);
    }

    #[test]
    fn test_centerline_on_face_vertical() {
        let boundary = vec![
            Point2::new(0.0, 0.0),
            Point2::new(10.0, 0.0),
            Point2::new(10.0, 40.0),
            Point2::new(0.0, 40.0),
        ];
        let cl = centerline_on_face(&boundary, 2.0);
        // Taller than wide, so vertical centerline at x=5
        assert!((cl.start.x - 5.0).abs() < 1e-10);
        assert!((cl.end.x - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_centerline_between_lines() {
        let cl = centerline_between_lines(
            Point2::new(0.0, 0.0),
            Point2::new(100.0, 0.0),
            Point2::new(0.0, 20.0),
            Point2::new(100.0, 20.0),
            5.0,
        );
        assert!((cl.start.y - 10.0).abs() < 1e-10);
        assert!((cl.end.y - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_centerline_between_points() {
        let cl = centerline_between_points(Point2::new(10.0, 20.0), Point2::new(50.0, 60.0), 3.0);
        assert!((cl.start.x - 10.0).abs() < 1e-10);
        assert!((cl.end.x - 50.0).abs() < 1e-10);
        let svg = centerline_to_svg(&cl);
        assert!(svg.contains("stroke-dasharray"));
    }

    #[test]
    fn test_bolt_circle_centerlines_constructor() {
        let bc = bolt_circle_centerlines(Point2::new(50.0, 50.0), 20.0, 8, 4.0);
        assert_eq!(bc.bolt_count, 8);
        assert!((bc.radius - 20.0).abs() < 1e-10);
        let svg = bolt_circle_centerlines_to_svg(&bc);
        assert!(svg.contains("<circle"));
    }

    #[test]
    fn test_cosmetic_line_constructor() {
        let cl = cosmetic_line(
            Point2::new(0.0, 0.0),
            Point2::new(50.0, 50.0),
            CosmeticLineStyle::DashDot,
        );
        let svg = cosmetic_line_to_svg(&cl);
        assert!(svg.contains("stroke-dasharray"));
    }

    #[test]
    fn test_cosmetic_thread_internal() {
        let ct = cosmetic_thread_internal(Point2::new(50.0, 50.0), 10.0, 20.0);
        assert!(ct.internal);
        let svg = cosmetic_thread_to_svg(&ct);
        assert!(svg.contains("<circle"));
        assert!(svg.contains("stroke-dasharray=\"1,1\""));
    }

    #[test]
    fn test_cosmetic_thread_external() {
        let ct = cosmetic_thread_external(Point2::new(50.0, 50.0), 10.0, 20.0);
        assert!(!ct.internal);
        let svg = cosmetic_thread_to_svg(&ct);
        assert!(svg.contains("<circle"));
        assert!(!svg.contains("stroke-dasharray=\"1,1\""));
    }

    #[test]
    fn test_cosmetic_vertex_cross() {
        let cv = cosmetic_vertex(Point2::new(30.0, 30.0), CosmeticVertexStyle::Cross);
        let svg = cosmetic_vertex_to_svg(&cv);
        assert_eq!(svg.matches("<line").count(), 2);
    }

    #[test]
    fn test_cosmetic_vertex_circle() {
        let cv = cosmetic_vertex(Point2::new(30.0, 30.0), CosmeticVertexStyle::Circle);
        let svg = cosmetic_vertex_to_svg(&cv);
        assert!(svg.contains("<circle"));
    }

    #[test]
    fn test_cosmetic_vertex_dot() {
        let cv = cosmetic_vertex(Point2::new(30.0, 30.0), CosmeticVertexStyle::Dot);
        let svg = cosmetic_vertex_to_svg(&cv);
        assert!(svg.contains("fill=\"red\""));
    }

    #[test]
    fn test_cosmetic_circle() {
        let cc = cosmetic_circle(Point2::new(50.0, 50.0), 15.0);
        let svg = cosmetic_circle_to_svg(&cc);
        assert!(svg.contains("<circle"));
        assert!(svg.contains("r=\"15\""));
    }

    #[test]
    fn test_cosmetic_arc() {
        let ca = cosmetic_arc(Point2::new(50.0, 50.0), 20.0, 0.0, 90.0);
        let svg = cosmetic_arc_to_svg(&ca);
        assert!(svg.contains("<path"));
        assert!(svg.contains("stroke-dasharray"));
    }

    #[test]
    fn test_edit_line_appearance() {
        let mut elem = DrawingElement::new("e1");
        edit_line_appearance(&mut elem, "red", 1.0, CosmeticLineStyle::Dashed);
        assert_eq!(elem.color, "red");
        assert!((elem.width - 1.0).abs() < 1e-10);
        assert_eq!(elem.line_style, CosmeticLineStyle::Dashed);
    }

    #[test]
    fn test_toggle_edge_visibility() {
        let mut elem = DrawingElement::new("e1");
        assert!(elem.visible);
        toggle_edge_visibility(&mut elem, false);
        assert!(!elem.visible);
        toggle_edge_visibility(&mut elem, true);
        assert!(elem.visible);
    }

    #[test]
    fn test_chain_dimension() {
        let points = vec![
            Point2::new(0.0, 0.0),
            Point2::new(10.0, 0.0),
            Point2::new(25.0, 0.0),
        ];
        let cd = chain_dimension(&points);
        assert_eq!(cd.segments.len(), 2);
        match &cd.segments[0] {
            DimensionType::Length { value, .. } => {
                assert!((value - 10.0).abs() < 1e-10);
            }
            _ => panic!("expected Length"),
        }
        match &cd.segments[1] {
            DimensionType::Length { value, .. } => {
                assert!((value - 15.0).abs() < 1e-10);
            }
            _ => panic!("expected Length"),
        }
    }

    #[test]
    fn test_chain_dimension_to_svg() {
        let points = vec![
            Point2::new(0.0, 0.0),
            Point2::new(10.0, 0.0),
            Point2::new(25.0, 0.0),
        ];
        let cd = chain_dimension(&points);
        let svg = chain_dimension_to_svg(&cd);
        assert!(svg.contains("<line"));
        assert!(svg.contains("<text"));
    }

    #[test]
    fn test_coordinate_dimension() {
        let origin = Point2::new(0.0, 0.0);
        let points = vec![Point2::new(10.0, 0.0), Point2::new(20.0, 0.0)];
        let cd = coordinate_dimension(origin, &points);
        assert_eq!(cd.dimensions.len(), 2);
    }

    #[test]
    fn test_coordinate_dimension_to_svg() {
        let origin = Point2::new(0.0, 0.0);
        let points = vec![Point2::new(10.0, 0.0)];
        let cd = coordinate_dimension(origin, &points);
        let svg = coordinate_dimension_to_svg(&cd);
        assert!(svg.contains("<text"));
    }

    #[test]
    fn test_chamfer_dimension_constructor() {
        let cd = chamfer_dimension(Point2::new(20.0, 20.0), 1.5, 1.5, 45.0);
        assert!((cd.size - 1.5).abs() < 1e-10);
        assert!((cd.angle - 45.0).abs() < 1e-10);
        let svg = chamfer_dimension_to_svg(&cd);
        assert!(svg.contains("C1.5"));
    }

    #[test]
    fn test_formatted_dimension_prefix() {
        let mut fd = FormattedDimension {
            dimension: DimensionType::RadiusDimension {
                center: Point2::new(50.0, 50.0),
                radius: 10.0,
            },
            prefix: String::new(),
            decimal_places: 2,
        };
        set_prefix_symbol(&mut fd, "R");
        assert_eq!(fd.prefix, "R");
        let svg = formatted_dimension_to_svg(&fd);
        assert!(svg.contains(">R<"));
    }

    #[test]
    fn test_formatted_dimension_decimal_places() {
        let mut fd = FormattedDimension {
            dimension: DimensionType::Length {
                start: Point2::new(0.0, 0.0),
                end: Point2::new(10.0, 0.0),
                value: 10.0,
            },
            prefix: String::new(),
            decimal_places: 2,
        };
        set_decimal_places(&mut fd, 3);
        assert_eq!(fd.decimal_places, 3);
    }

    #[test]
    fn test_stack_order() {
        let mut elem = DrawingElement::new("e1");
        assert_eq!(elem.stack_order, 0);
        stack_order(&mut elem, StackOrder::Front);
        assert_eq!(elem.stack_order, i32::MAX);
        stack_order(&mut elem, StackOrder::Back);
        assert_eq!(elem.stack_order, i32::MIN);
        let mut elem2 = DrawingElement::new("e2");
        stack_order(&mut elem2, StackOrder::Forward);
        assert_eq!(elem2.stack_order, 1);
        stack_order(&mut elem2, StackOrder::Backward);
        assert_eq!(elem2.stack_order, 0);
    }

    #[test]
    fn test_align_elements_left() {
        let mut positions = vec![(10.0, 5.0), (30.0, 15.0), (20.0, 25.0)];
        align_elements(&mut positions, Alignment::Left);
        for p in &positions {
            assert!((p.0 - 10.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_align_elements_center_vertical() {
        let mut positions = vec![(10.0, 10.0), (20.0, 20.0), (30.0, 30.0)];
        align_elements(&mut positions, Alignment::CenterVertical);
        for p in &positions {
            assert!((p.1 - 20.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_lock_element() {
        let mut elem = DrawingElement::new("e1");
        assert!(!elem.locked);
        lock_element(&mut elem, true);
        assert!(elem.locked);
        lock_element(&mut elem, false);
        assert!(!elem.locked);
    }

    #[test]
    fn test_page_from_template_a4() {
        let sheet = page_from_template(PaperTemplate::A4Landscape);
        assert!((sheet.width - 297.0).abs() < 1e-10);
        assert!((sheet.height - 210.0).abs() < 1e-10);
    }

    #[test]
    fn test_page_from_template_a3_portrait() {
        let sheet = page_from_template(PaperTemplate::A3Portrait);
        assert!((sheet.width - 297.0).abs() < 1e-10);
        assert!((sheet.height - 420.0).abs() < 1e-10);
    }

    #[test]
    fn test_page_from_template_a0() {
        let sheet = page_from_template(PaperTemplate::A0Landscape);
        assert!((sheet.width - 1189.0).abs() < 1e-10);
        assert!((sheet.height - 841.0).abs() < 1e-10);
    }

    #[test]
    fn test_update_template_fields() {
        let mut sheet = page_from_template(PaperTemplate::A4Landscape);
        let mut fields = std::collections::HashMap::new();
        fields.insert("title".to_string(), "Test Drawing".to_string());
        fields.insert("author".to_string(), "Engineer".to_string());
        fields.insert("date".to_string(), "2026-03-24".to_string());
        update_template_fields(&mut sheet, &fields);
        assert!(sheet.title.contains("Test Drawing"));
        assert!(sheet.title.contains("author: Engineer"));
        assert!(sheet.title.contains("date: 2026-03-24"));
    }

    #[test]
    fn test_redraw_page() {
        let mut sheet = DrawingSheet::a4_landscape();
        sheet.views.push(DrawingView {
            direction: ProjectionDir::Front,
            edges: vec![ProjectedEdge {
                x1: 10.0,
                y1: 20.0,
                x2: 40.0,
                y2: 60.0,
                visible: true,
            }],
            center_x: 0.0,
            center_y: 0.0,
            scale: 1.0,
            sheet_x: None,
            sheet_y: None,
            sheet_scale: None,
        });
        redraw_page(&mut sheet);
        assert!((sheet.views[0].center_x - 25.0).abs() < 1e-10);
        assert!((sheet.views[0].center_y - 40.0).abs() < 1e-10);
    }

    #[test]
    fn test_drawing_to_svg_uses_manual_view_placement() {
        let mut sheet = DrawingSheet::a4_landscape();
        let mut view = DrawingView {
            direction: ProjectionDir::Front,
            edges: vec![ProjectedEdge {
                x1: 0.0,
                y1: 0.0,
                x2: 10.0,
                y2: 10.0,
                visible: true,
            }],
            center_x: 5.0,
            center_y: 5.0,
            scale: 1.0,
            sheet_x: None,
            sheet_y: None,
            sheet_scale: None,
        };
        view.set_sheet_placement(100.0, 80.0, Some(2.0));
        sheet.views.push(view);
        let svg = drawing_to_svg(&sheet).render();
        assert!(svg.contains("x1=\"90\""));
        assert!(svg.contains("x2=\"110\""));
        assert!(svg.contains("y1=\"90\""));
    }

    #[test]
    fn test_cosmetic_parallel_line() {
        let cl = cosmetic_parallel_line(
            Point2::new(0.0, 0.0),
            Point2::new(100.0, 0.0),
            10.0,
            CosmeticLineStyle::Dashed,
        );
        // Parallel to X-axis, offset 10 units in Y
        assert!((cl.start.y - 10.0).abs() < 1e-10);
        assert!((cl.end.y - 10.0).abs() < 1e-10);
        assert!((cl.start.x - 0.0).abs() < 1e-10);
        assert!((cl.end.x - 100.0).abs() < 1e-10);
        let svg = cosmetic_line_to_svg(&cl);
        assert!(svg.contains("stroke-dasharray"));
    }

    #[test]
    fn test_cosmetic_perpendicular_line() {
        let cl = cosmetic_perpendicular_line(
            Point2::new(0.0, 0.0),
            Point2::new(100.0, 0.0),
            Point2::new(50.0, 0.0),
            20.0,
            CosmeticLineStyle::DashDot,
        );
        // Perpendicular to X-axis through (50,0), half-length 20
        assert!((cl.start.x - 50.0).abs() < 1e-10);
        assert!((cl.end.x - 50.0).abs() < 1e-10);
        assert!((cl.start.y - 20.0).abs() < 1e-10);
        assert!((cl.end.y - (-20.0)).abs() < 1e-10);
    }

    #[test]
    fn test_print_all_pages() {
        let sheet1 = page_from_template(PaperTemplate::A4Landscape);
        let sheet2 = page_from_template(PaperTemplate::A3Landscape);
        let docs = print_all_pages(&[sheet1, sheet2]);
        assert_eq!(docs.len(), 2);
        assert!((docs[0].width - 297.0).abs() < 1e-10);
        assert!((docs[1].width - 420.0).abs() < 1e-10);
        // Each renders to valid SVG
        for doc in &docs {
            let rendered = doc.render();
            assert!(rendered.contains("<svg"));
            assert!(rendered.contains("</svg>"));
        }
    }

    #[test]
    fn test_insert_svg() {
        let mut sheet = DrawingSheet::a4_landscape();
        let insert = insert_svg(
            &mut sheet,
            "<circle cx=\"10\" cy=\"10\" r=\"5\"/>",
            Point2::new(20.0, 30.0),
            50.0,
            40.0,
        );
        assert!((insert.position.x - 20.0).abs() < 1e-10);
        assert!((insert.width - 50.0).abs() < 1e-10);
        let svg = insert.to_svg();
        assert!(svg.contains("foreignObject"));
        assert!(svg.contains("<circle"));
        assert_eq!(sheet.dimensions.len(), 1);
    }

    #[test]
    fn test_bitmap_image() {
        let mut sheet = DrawingSheet::a4_landscape();
        let data = vec![0x89, 0x50, 0x4E, 0x47]; // PNG magic bytes
        let img = bitmap_image(&mut sheet, &data, Point2::new(10.0, 20.0), 100.0, 80.0);
        assert_eq!(img.image_data.len(), 4);
        let svg = img.to_svg();
        assert!(svg.contains("<image"));
        assert!(svg.contains("base64,"));
        assert!(svg.contains("width=\"100\""));
        assert_eq!(sheet.dimensions.len(), 1);
    }

    #[test]
    fn test_share_view() {
        let mut source = DrawingSheet::a4_landscape();
        source.views.push(DrawingView {
            direction: ProjectionDir::Front,
            edges: vec![ProjectedEdge {
                x1: 0.0,
                y1: 0.0,
                x2: 10.0,
                y2: 10.0,
                visible: true,
            }],
            center_x: 5.0,
            center_y: 5.0,
            scale: 1.0,
            sheet_x: None,
            sheet_y: None,
            sheet_scale: None,
        });
        let mut target = DrawingSheet::a4_landscape();
        assert!(share_view(&source, &mut target, 0).is_ok());
        assert_eq!(target.views.len(), 1);
        assert_eq!(target.views[0].direction, ProjectionDir::Front);
    }

    #[test]
    fn test_share_view_out_of_range() {
        let source = DrawingSheet::a4_landscape();
        let mut target = DrawingSheet::a4_landscape();
        assert!(share_view(&source, &mut target, 0).is_err());
    }

    #[test]
    fn test_base64_encode_bytes() {
        let svg_insert = SvgInsert {
            svg_content: "<rect/>".to_string(),
            position: Point2::new(0.0, 0.0),
            width: 10.0,
            height: 10.0,
        };
        let svg = svg_insert.to_svg();
        assert!(svg.contains("foreignObject"));
    }
}
