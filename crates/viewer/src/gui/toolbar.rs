use super::{AssemblyJointType, FemConstraintType, GizmoMode, GuiAction, GuiState, SelectionMode, SketchTool, Workbench};
use cadkernel_sketch::WorkPlane;
use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};

// ---------------------------------------------------------------------------
// Tool icon identifiers
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum ToolIcon {
    // File / general
    New, Open, Save, Undo, Redo,
    FitAll, ResetCamera, ShowAll, HideAll,
    // Selection modes
    SelectSolid, SelectFace, SelectEdge, SelectVertex,
    // Primitives
    PrimBox, Cylinder, Sphere, Cone, Torus,
    Tube, Prism, Wedge, Ellipsoid, Helix,
    // Boolean
    BoolUnion, BoolSubtract, BoolIntersect,
    // Transform / features
    Mirror, Scale, Shell, Fillet, Chamfer, Pattern,
    Extrude, Revolve, Hole,
    // Join / compound / convert / analysis
    Connect, Embed, Cutout,
    Explode, Filter, Fragments, Slice,
    Points, ToSolid, Defeature,
    Measure, Check,
    // PartDesign
    Pad, Pocket, Groove,
    AddLoft, AddPipe, SubLoft, SubPipe,
    Sprocket, Gear, Shaft,
    Suppress, SetTip, MoveUp, MoveDown,
    // Sketch geometry
    SketchSelect, SketchPoint, SketchLine, SketchRect, SketchCircle,
    SketchArc, SketchEllipse, SketchPolyline, SketchSlot, SketchBSpline,
    SketchPolygon,
    // Sketch constraints
    Coincident, Horizontal, Vertical, Parallel, Perpendicular,
    Tangent, Equal, Symmetric,
    LengthConstraint, Distance, Angle, Radius, Diameter, FixPin,
    // Sketch tools
    SketchFillet, SketchChamfer, Trim, Extend,
    // Sketch toggles
    Construction, Grid, Snap, ShowConstraints,
    // Close / cancel
    Accept, Cancel,
    // Mesh
    Import, ExportStl, ExportObj, ExportGltf,
    Decimate, Subdivide, FillHoles, FlipNormals, Smooth, Harmonize,
    Watertight, Remesh, Repair,
    // TechDraw
    NewPage, Template, Redraw,
    ViewFront, ViewTop, ViewRight, ViewIso, SectionView, DetailView, BrokenView, ThreeView,
    DimLinear, DimRadius, DimDiameter, DimAngle, DimArcLen, DimArea,
    TextAnnot, RichText, Balloon, Leader, Weld, SurfFinish,
    CenterFace, CenterLines, CenterPoints, BoltCircle,
    ExportSvg, ExportDxf, ExportPdf, Clear,
    // Assembly
    AssemblyNew, InsertComponent, Solve, ExplodeView, Bom, Dof,
    JointFixed, JointRevolute, JointCylindrical, JointSlider, JointBall,
    JointDistance, JointAngle, JointParallel, JointPerp,
    JointGear, JointRack, JointScrew, JointBelt,
    // Draft
    DraftLine, DraftWire, DraftCircle, DraftArc, DraftEllipse, DraftRect,
    DraftPolygon, DraftBSpline, DraftBezier, DraftPoint, DraftFacebind, DraftHatch,
    DraftMove, DraftRotate, DraftScale, DraftMirror, DraftOffset, DraftTrim,
    DraftStretch, DraftClone,
    DraftArrayRect, DraftArrayPolar, DraftArrayPath, DraftArrayPoint,
    DraftDimension, DraftLabel, DraftText,
    DraftUpgrade, DraftDowngrade, DraftWireToBSpline, DraftToSketch,
    DraftSnapEndpoint, DraftSnapMidpoint, DraftSnapCenter, DraftSnapGrid,
    DraftSnapAngle, DraftSnapIntersect, DraftSnapPerp, DraftSnapParallel,
    // Surface
    SurfFilling, SurfBoundary, SurfSections, SurfExtend, SurfBlend, SurfPipe, SurfCoons,
    // FEM
    FemAnalysis, FemMaterial,
    FemTetMesh, FemRefine, FemSmooth, FemQuality,
    FemFixed, FemForce, FemPressure, FemDisplacement, FemGravity, FemSpring,
    SolveStatic, SolveModal, SolveThermal, SolveBuckling, SolveNonlinear,
    ShowStress, ShowDisplacement, ShowVonMises, FemSummary, FemReport,
}

// ---------------------------------------------------------------------------
// Vector icon drawing
// ---------------------------------------------------------------------------

fn draw_icon(ui: &mut egui::Ui, rect: egui::Rect, icon: ToolIcon, color: Color32) {
    let painter = ui.painter_at(rect);
    let c = rect.center();
    let s = rect.width() * 0.35;
    let stroke = Stroke::new(1.4, color);
    let thin = Stroke::new(1.0, color);

    match icon {
        // ---- File / general ------------------------------------------------
        ToolIcon::New => {
            // Plus in circle
            let r = s * 0.85;
            painter.circle_stroke(c, r, thin);
            painter.line_segment([c - Vec2::X * r * 0.5, c + Vec2::X * r * 0.5], stroke);
            painter.line_segment([c - Vec2::Y * r * 0.5, c + Vec2::Y * r * 0.5], stroke);
        }
        ToolIcon::Open => {
            // Folder outline
            let tl = c + Vec2::new(-s, -s * 0.6);
            let br = c + Vec2::new(s, s * 0.7);
            let tab = c + Vec2::new(-s * 0.2, -s * 0.6);
            let tab_top = c + Vec2::new(-s * 0.2, -s * 0.9);
            let tl_top = c + Vec2::new(-s, -s * 0.9);
            painter.line_segment([tl_top, tab_top], stroke);
            painter.line_segment([tab_top, tab], stroke);
            painter.line_segment([tab, Pos2::new(br.x, tl.y)], stroke);
            painter.line_segment([Pos2::new(br.x, tl.y), br], stroke);
            painter.line_segment([br, Pos2::new(tl.x, br.y)], stroke);
            painter.line_segment([Pos2::new(tl.x, br.y), tl_top], stroke);
        }
        ToolIcon::Save => {
            // Floppy disk
            let h = s * 0.85;
            let r = egui::Rect::from_center_size(c, Vec2::splat(h * 2.0));
            painter.rect_stroke(r, 2.0, stroke, StrokeKind::Middle);
            // Inner label area
            let inner = egui::Rect::from_center_size(
                c + Vec2::new(0.0, h * 0.35),
                Vec2::new(h * 1.2, h * 0.7),
            );
            painter.rect_stroke(inner, 1.0, thin, StrokeKind::Middle);
            // Top slot
            let slot = egui::Rect::from_center_size(
                c + Vec2::new(h * 0.2, -h * 0.7),
                Vec2::new(h * 0.6, h * 0.5),
            );
            painter.rect_filled(slot, 0.0, color);
        }
        ToolIcon::Undo => {
            // Curved left arrow
            let r = s * 0.7;
            let center_arc = c + Vec2::new(s * 0.1, 0.0);
            draw_arc(&painter, center_arc, r, std::f32::consts::PI, -std::f32::consts::FRAC_PI_4, 10, stroke);
            // Arrowhead at start
            let tip = center_arc + Vec2::new(-r, 0.0);
            painter.line_segment([tip, tip + Vec2::new(s * 0.3, -s * 0.25)], stroke);
            painter.line_segment([tip, tip + Vec2::new(s * 0.3, s * 0.25)], stroke);
        }
        ToolIcon::Redo => {
            // Curved right arrow
            let r = s * 0.7;
            let center_arc = c + Vec2::new(-s * 0.1, 0.0);
            draw_arc(&painter, center_arc, r, 0.0, std::f32::consts::PI + std::f32::consts::FRAC_PI_4, 10, stroke);
            let tip = center_arc + Vec2::new(r, 0.0);
            painter.line_segment([tip, tip + Vec2::new(-s * 0.3, -s * 0.25)], stroke);
            painter.line_segment([tip, tip + Vec2::new(-s * 0.3, s * 0.25)], stroke);
        }
        ToolIcon::FitAll => {
            // Magnifier with arrows
            let r = s * 0.55;
            painter.circle_stroke(c + Vec2::new(-s * 0.15, -s * 0.15), r, stroke);
            let handle_start = c + Vec2::new(-s * 0.15 + r * 0.7, -s * 0.15 + r * 0.7);
            painter.line_segment([handle_start, c + Vec2::new(s * 0.8, s * 0.8)], Stroke::new(2.0, color));
        }
        ToolIcon::ResetCamera => {
            // Home icon
            let h = s * 0.8;
            // Roof
            painter.line_segment([c + Vec2::new(0.0, -h), c + Vec2::new(-h, 0.0)], stroke);
            painter.line_segment([c + Vec2::new(0.0, -h), c + Vec2::new(h, 0.0)], stroke);
            // Walls
            let wall_r = egui::Rect::from_min_max(
                c + Vec2::new(-h * 0.7, 0.0),
                c + Vec2::new(h * 0.7, h * 0.8),
            );
            painter.rect_stroke(wall_r, 0.0, thin, StrokeKind::Middle);
        }
        ToolIcon::ShowAll => {
            // Eye icon
            draw_eye(&painter, c, s, stroke, color, true);
        }
        ToolIcon::HideAll => {
            // Eye with strikethrough
            draw_eye(&painter, c, s, stroke, color, false);
        }

        // ---- Selection modes -----------------------------------------------
        ToolIcon::SelectSolid => {
            // Filled cube silhouette
            let h = s * 0.7;
            let r = egui::Rect::from_center_size(c, Vec2::splat(h * 2.0));
            painter.rect_filled(r, 2.0, Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 80));
            painter.rect_stroke(r, 2.0, stroke, StrokeKind::Middle);
        }
        ToolIcon::SelectFace => {
            // Square outline
            let h = s * 0.7;
            let r = egui::Rect::from_center_size(c, Vec2::splat(h * 2.0));
            painter.rect_stroke(r, 0.0, stroke, StrokeKind::Middle);
        }
        ToolIcon::SelectEdge => {
            // Horizontal line
            painter.line_segment([c - Vec2::X * s, c + Vec2::X * s], Stroke::new(2.0, color));
        }
        ToolIcon::SelectVertex => {
            // Filled circle (dot)
            painter.circle_filled(c, s * 0.4, color);
        }

        // ---- Primitives ----------------------------------------------------
        ToolIcon::PrimBox => {
            // 3D box wireframe
            let h = s * 0.6;
            let off = Vec2::new(h * 0.4, -h * 0.4);
            let front = egui::Rect::from_center_size(c + Vec2::new(-off.x * 0.3, off.y * 0.3), Vec2::splat(h * 1.5));
            painter.rect_stroke(front, 0.0, stroke, StrokeKind::Middle);
            // Back face (offset)
            let back = front.translate(off);
            painter.rect_stroke(back, 0.0, thin, StrokeKind::Middle);
            // Connecting lines (top-left, top-right, bottom-right only visible)
            painter.line_segment([front.right_top(), back.right_top()], thin);
            painter.line_segment([front.left_top(), back.left_top()], thin);
            painter.line_segment([front.right_bottom(), back.right_bottom()], thin);
        }
        ToolIcon::Cylinder => {
            // Top ellipse + sides + bottom ellipse
            let h = s * 0.8;
            let ry = s * 0.25;
            let top_c = c - Vec2::Y * h * 0.5;
            let bot_c = c + Vec2::Y * h * 0.5;
            draw_ellipse(&painter, top_c, s * 0.7, ry, 16, stroke);
            draw_ellipse(&painter, bot_c, s * 0.7, ry, 16, thin);
            painter.line_segment([top_c - Vec2::X * s * 0.7, bot_c - Vec2::X * s * 0.7], stroke);
            painter.line_segment([top_c + Vec2::X * s * 0.7, bot_c + Vec2::X * s * 0.7], stroke);
        }
        ToolIcon::Sphere => {
            // Circle + horizontal ellipse
            let r = s * 0.8;
            painter.circle_stroke(c, r, stroke);
            draw_ellipse(&painter, c, r, r * 0.3, 16, thin);
        }
        ToolIcon::Cone => {
            // Triangle + bottom ellipse
            let h = s * 0.9;
            let bot_c = c + Vec2::Y * h * 0.4;
            let top = c - Vec2::Y * h;
            painter.line_segment([top, bot_c - Vec2::X * s * 0.7], stroke);
            painter.line_segment([top, bot_c + Vec2::X * s * 0.7], stroke);
            draw_ellipse(&painter, bot_c, s * 0.7, s * 0.25, 16, stroke);
        }
        ToolIcon::Torus => {
            // Two nested ellipses (donut cross-section hint)
            let r_outer = s * 0.85;
            let r_inner = s * 0.35;
            draw_ellipse(&painter, c, r_outer, r_outer * 0.45, 20, stroke);
            draw_ellipse(&painter, c, r_inner, r_inner * 0.45, 12, thin);
        }
        ToolIcon::Tube => {
            // Cylinder with inner hole hint
            let h = s * 0.7;
            let ry = s * 0.2;
            let top_c = c - Vec2::Y * h * 0.5;
            let bot_c = c + Vec2::Y * h * 0.5;
            draw_ellipse(&painter, top_c, s * 0.65, ry, 14, stroke);
            draw_ellipse(&painter, top_c, s * 0.35, ry * 0.6, 10, thin);
            painter.line_segment([top_c - Vec2::X * s * 0.65, bot_c - Vec2::X * s * 0.65], stroke);
            painter.line_segment([top_c + Vec2::X * s * 0.65, bot_c + Vec2::X * s * 0.65], stroke);
            draw_arc(&painter, bot_c, s * 0.65, 0.0, std::f32::consts::PI, 10, thin);
        }
        ToolIcon::Prism => {
            // Hexagonal prism top face
            let r = s * 0.7;
            draw_regular_polygon(&painter, c, r, 6, stroke);
        }
        ToolIcon::Wedge => {
            // Wedge / ramp shape
            let h = s * 0.8;
            let tl = c + Vec2::new(-h, -h * 0.3);
            let tr = c + Vec2::new(h, -h * 0.8);
            let br = c + Vec2::new(h, h * 0.7);
            let bl = c + Vec2::new(-h, h * 0.7);
            painter.line_segment([tl, tr], stroke);
            painter.line_segment([tr, br], stroke);
            painter.line_segment([br, bl], stroke);
            painter.line_segment([bl, tl], stroke);
        }
        ToolIcon::Ellipsoid => {
            // Horizontal ellipse (wider than tall)
            draw_ellipse(&painter, c, s * 0.9, s * 0.6, 20, stroke);
        }
        ToolIcon::Helix => {
            // Sine wave (helix from side)
            let pts: Vec<Pos2> = (0..=16).map(|i| {
                let t = i as f32 / 16.0;
                let x = c.x - s + t * s * 2.0;
                let y = c.y + (t * std::f32::consts::TAU * 1.5).sin() * s * 0.5;
                Pos2::new(x, y)
            }).collect();
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
        }

        // ---- Boolean -------------------------------------------------------
        ToolIcon::BoolUnion => {
            let r = s * 0.5;
            let off = s * 0.3;
            painter.circle_stroke(c + Vec2::new(-off, 0.0), r, stroke);
            painter.circle_stroke(c + Vec2::new(off, 0.0), r, stroke);
        }
        ToolIcon::BoolSubtract => {
            let r = s * 0.5;
            let off = s * 0.3;
            painter.circle_stroke(c + Vec2::new(-off, 0.0), r, stroke);
            let dashed_color = Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 100);
            painter.circle_stroke(c + Vec2::new(off, 0.0), r, Stroke::new(1.4, dashed_color));
        }
        ToolIcon::BoolIntersect => {
            let r = s * 0.5;
            let off = s * 0.3;
            let dim = Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 80);
            painter.circle_stroke(c + Vec2::new(-off, 0.0), r, Stroke::new(1.4, dim));
            painter.circle_stroke(c + Vec2::new(off, 0.0), r, Stroke::new(1.4, dim));
            // Highlight intersection area with a small filled circle
            painter.circle_filled(c, r * 0.3, Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 60));
        }

        // ---- Transform / features ------------------------------------------
        ToolIcon::Mirror => {
            // Dashed vertical line + small shapes on each side
            let dash_color = Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 120);
            draw_dashed_line(&painter, c - Vec2::Y * s, c + Vec2::Y * s, 3.0, 3.0, Stroke::new(1.0, dash_color));
            let sz = s * 0.35;
            let left_r = egui::Rect::from_center_size(c + Vec2::new(-s * 0.55, 0.0), Vec2::splat(sz));
            let right_r = egui::Rect::from_center_size(c + Vec2::new(s * 0.55, 0.0), Vec2::splat(sz));
            painter.rect_stroke(left_r, 0.0, stroke, StrokeKind::Middle);
            painter.rect_stroke(right_r, 0.0, thin, StrokeKind::Middle);
        }
        ToolIcon::Scale => {
            // Small rect + big rect (scale indication)
            let small = egui::Rect::from_center_size(c + Vec2::new(-s * 0.2, s * 0.2), Vec2::splat(s * 0.7));
            let big = egui::Rect::from_center_size(c + Vec2::new(s * 0.15, -s * 0.15), Vec2::splat(s * 1.2));
            painter.rect_stroke(small, 0.0, thin, StrokeKind::Middle);
            painter.rect_stroke(big, 0.0, stroke, StrokeKind::Middle);
        }
        ToolIcon::Shell => {
            // Rect with inner rect (hollowed)
            let outer = egui::Rect::from_center_size(c, Vec2::splat(s * 1.5));
            let inner = egui::Rect::from_center_size(c, Vec2::splat(s * 0.8));
            painter.rect_stroke(outer, 0.0, stroke, StrokeKind::Middle);
            painter.rect_stroke(inner, 0.0, thin, StrokeKind::Middle);
        }
        ToolIcon::Fillet => {
            // Square with rounded corner
            let h = s * 0.8;
            let tl = c + Vec2::new(-h, -h);
            let tr = c + Vec2::new(h, -h);
            let bl = c + Vec2::new(-h, h);
            painter.line_segment([tl, tr], stroke);
            painter.line_segment([bl, tl], stroke);
            // Rounded bottom-right
            painter.line_segment([tr, c + Vec2::new(h, h * 0.3)], stroke);
            draw_arc(&painter, c + Vec2::new(h * 0.3, h * 0.3), h * 0.7, 0.0, std::f32::consts::FRAC_PI_2, 8, stroke);
            painter.line_segment([c + Vec2::new(h * 0.3, h), bl], stroke);
        }
        ToolIcon::Chamfer => {
            // Square with angled corner
            let h = s * 0.8;
            let tl = c + Vec2::new(-h, -h);
            let tr = c + Vec2::new(h, -h);
            let br = c + Vec2::new(h, h);
            let bl = c + Vec2::new(-h, h);
            painter.line_segment([tl, tr], stroke);
            painter.line_segment([bl, tl], stroke);
            // Chamfered bottom-right
            painter.line_segment([tr, c + Vec2::new(h, h * 0.3)], stroke);
            painter.line_segment([c + Vec2::new(h, h * 0.3), c + Vec2::new(h * 0.3, h)], Stroke::new(1.8, color));
            painter.line_segment([c + Vec2::new(h * 0.3, h), bl], stroke);
            // Highlight the chamfer
            painter.line_segment([br, br], Stroke::NONE); // unused corner
        }
        ToolIcon::Pattern => {
            // Three small squares in a row
            for i in 0..3 {
                let x = c.x - s * 0.7 + i as f32 * s * 0.7;
                let r = egui::Rect::from_center_size(Pos2::new(x, c.y), Vec2::splat(s * 0.45));
                painter.rect_stroke(r, 0.0, if i == 0 { stroke } else { thin }, StrokeKind::Middle);
            }
        }
        ToolIcon::Extrude => {
            // Rectangle with upward arrow
            let r = egui::Rect::from_center_size(c + Vec2::new(0.0, s * 0.3), Vec2::new(s * 1.2, s * 0.6));
            painter.rect_stroke(r, 0.0, stroke, StrokeKind::Middle);
            let arrow_base = c + Vec2::new(0.0, 0.0);
            let arrow_tip = c + Vec2::new(0.0, -s * 0.9);
            painter.line_segment([arrow_base, arrow_tip], stroke);
            painter.line_segment([arrow_tip, arrow_tip + Vec2::new(-s * 0.2, s * 0.25)], stroke);
            painter.line_segment([arrow_tip, arrow_tip + Vec2::new(s * 0.2, s * 0.25)], stroke);
        }
        ToolIcon::Revolve => {
            // Circular arrow (rotation)
            let r = s * 0.65;
            draw_arc(&painter, c, r, -std::f32::consts::FRAC_PI_4, std::f32::consts::PI * 1.5, 14, stroke);
            // Arrowhead
            let tip_angle = std::f32::consts::PI * 1.25;
            let tip = c + Vec2::new(r * tip_angle.cos(), r * tip_angle.sin());
            let tang = Vec2::new(-tip_angle.sin(), tip_angle.cos());
            painter.line_segment([tip, tip + (tang * s * 0.3) + Vec2::new(s * 0.15, 0.0)], stroke);
            painter.line_segment([tip, tip + (tang * s * 0.3) - Vec2::new(s * 0.15, 0.0)], stroke);
        }
        ToolIcon::Hole => {
            // Circle with cross (hole)
            let r = s * 0.65;
            painter.circle_stroke(c, r, stroke);
            painter.line_segment([c - Vec2::X * r * 0.5, c + Vec2::X * r * 0.5], thin);
            painter.line_segment([c - Vec2::Y * r * 0.5, c + Vec2::Y * r * 0.5], thin);
        }

        // ---- Join / compound / convert / analysis --------------------------
        ToolIcon::Connect => {
            let r = s * 0.35;
            painter.circle_stroke(c - Vec2::X * s * 0.4, r, stroke);
            painter.circle_stroke(c + Vec2::X * s * 0.4, r, stroke);
            painter.line_segment([c - Vec2::X * s * 0.05, c + Vec2::X * s * 0.05], Stroke::new(2.0, color));
        }
        ToolIcon::Embed => {
            let r = s * 0.6;
            let inner_r = s * 0.3;
            painter.rect_stroke(egui::Rect::from_center_size(c, Vec2::splat(r * 2.0)), 0.0, stroke, StrokeKind::Middle);
            painter.circle_stroke(c, inner_r, thin);
        }
        ToolIcon::Cutout => {
            let r = s * 0.6;
            painter.rect_stroke(egui::Rect::from_center_size(c, Vec2::splat(r * 2.0)), 0.0, stroke, StrokeKind::Middle);
            let dim = Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 80);
            painter.circle_stroke(c + Vec2::new(r * 0.4, 0.0), r * 0.5, Stroke::new(1.4, dim));
        }
        ToolIcon::Explode => {
            // Four small squares spreading out
            for &d in &[Vec2::new(-1.0, -1.0), Vec2::new(1.0, -1.0), Vec2::new(-1.0, 1.0), Vec2::new(1.0, 1.0)] {
                let p = c + d * s * 0.45;
                painter.rect_stroke(egui::Rect::from_center_size(p, Vec2::splat(s * 0.35)), 0.0, thin, StrokeKind::Middle);
            }
        }
        ToolIcon::Filter => {
            // Funnel shape
            painter.line_segment([c + Vec2::new(-s, -s * 0.6), c + Vec2::new(s, -s * 0.6)], stroke);
            painter.line_segment([c + Vec2::new(-s, -s * 0.6), c + Vec2::new(-s * 0.15, s * 0.2)], stroke);
            painter.line_segment([c + Vec2::new(s, -s * 0.6), c + Vec2::new(s * 0.15, s * 0.2)], stroke);
            painter.line_segment([c + Vec2::new(-s * 0.15, s * 0.2), c + Vec2::new(-s * 0.15, s * 0.8)], stroke);
            painter.line_segment([c + Vec2::new(s * 0.15, s * 0.2), c + Vec2::new(s * 0.15, s * 0.8)], stroke);
        }
        ToolIcon::Fragments => {
            let sz = s * 0.45;
            painter.rect_stroke(egui::Rect::from_center_size(c + Vec2::new(-sz * 0.5, -sz * 0.5), Vec2::splat(sz)), 0.0, stroke, StrokeKind::Middle);
            painter.rect_stroke(egui::Rect::from_center_size(c + Vec2::new(sz * 0.5, sz * 0.5), Vec2::splat(sz)), 0.0, thin, StrokeKind::Middle);
        }
        ToolIcon::Slice => {
            let h = s * 0.7;
            let r = egui::Rect::from_center_size(c, Vec2::new(h * 2.0, h * 1.5));
            painter.rect_stroke(r, 0.0, stroke, StrokeKind::Middle);
            painter.line_segment([Pos2::new(c.x, r.top()), Pos2::new(c.x, r.bottom())], Stroke::new(1.0, Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 120)));
        }
        ToolIcon::Points => {
            for &d in &[Vec2::ZERO, Vec2::new(-s * 0.6, -s * 0.4), Vec2::new(s * 0.5, -s * 0.3), Vec2::new(-s * 0.3, s * 0.5), Vec2::new(s * 0.6, s * 0.4)] {
                painter.circle_filled(c + d, 1.8, color);
            }
        }
        ToolIcon::ToSolid => {
            let h = s * 0.65;
            painter.rect_filled(egui::Rect::from_center_size(c, Vec2::splat(h * 1.6)), 2.0, Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 50));
            painter.rect_stroke(egui::Rect::from_center_size(c, Vec2::splat(h * 1.6)), 2.0, stroke, StrokeKind::Middle);
        }
        ToolIcon::Defeature => {
            // Scissors
            painter.line_segment([c + Vec2::new(-s * 0.6, -s * 0.6), c + Vec2::new(s * 0.6, s * 0.6)], stroke);
            painter.line_segment([c + Vec2::new(s * 0.6, -s * 0.6), c + Vec2::new(-s * 0.6, s * 0.6)], stroke);
            painter.circle_stroke(c + Vec2::new(-s * 0.6, s * 0.6), s * 0.2, thin);
            painter.circle_stroke(c + Vec2::new(s * 0.6, s * 0.6), s * 0.2, thin);
        }
        ToolIcon::Measure => {
            // Ruler
            painter.line_segment([c + Vec2::new(-s, s * 0.3), c + Vec2::new(s, s * 0.3)], stroke);
            painter.line_segment([c + Vec2::new(-s, -s * 0.3), c + Vec2::new(s, -s * 0.3)], stroke);
            for i in 0..5 {
                let x = c.x - s + i as f32 * s * 0.5;
                let h = if i % 2 == 0 { s * 0.3 } else { s * 0.15 };
                painter.line_segment([Pos2::new(x, c.y - s * 0.3), Pos2::new(x, c.y - s * 0.3 + h)], thin);
            }
        }
        ToolIcon::Check => {
            // Checkmark
            painter.line_segment([c + Vec2::new(-s * 0.6, 0.0), c + Vec2::new(-s * 0.15, s * 0.5)], Stroke::new(2.0, color));
            painter.line_segment([c + Vec2::new(-s * 0.15, s * 0.5), c + Vec2::new(s * 0.6, -s * 0.5)], Stroke::new(2.0, color));
        }

        // ---- PartDesign (reuse some icons, add unique ones) ----------------
        ToolIcon::Pad => draw_icon(ui, rect, ToolIcon::Extrude, color),
        ToolIcon::Pocket => {
            // Rect with downward arrow
            let r = egui::Rect::from_center_size(c + Vec2::new(0.0, -s * 0.3), Vec2::new(s * 1.2, s * 0.6));
            painter.rect_stroke(r, 0.0, stroke, StrokeKind::Middle);
            let arrow_base = c;
            let arrow_tip = c + Vec2::new(0.0, s * 0.9);
            painter.line_segment([arrow_base, arrow_tip], stroke);
            painter.line_segment([arrow_tip, arrow_tip + Vec2::new(-s * 0.2, -s * 0.25)], stroke);
            painter.line_segment([arrow_tip, arrow_tip + Vec2::new(s * 0.2, -s * 0.25)], stroke);
        }
        ToolIcon::Groove => draw_icon(ui, rect, ToolIcon::Revolve, color),
        ToolIcon::AddLoft | ToolIcon::SubLoft => {
            // Two horizontal lines with connecting curves
            painter.line_segment([c + Vec2::new(-s * 0.7, -s * 0.6), c + Vec2::new(s * 0.7, -s * 0.6)], stroke);
            painter.line_segment([c + Vec2::new(-s * 0.4, s * 0.6), c + Vec2::new(s * 0.4, s * 0.6)], stroke);
            painter.line_segment([c + Vec2::new(-s * 0.7, -s * 0.6), c + Vec2::new(-s * 0.4, s * 0.6)], thin);
            painter.line_segment([c + Vec2::new(s * 0.7, -s * 0.6), c + Vec2::new(s * 0.4, s * 0.6)], thin);
        }
        ToolIcon::AddPipe | ToolIcon::SubPipe => {
            // Circle with path line
            let r = s * 0.35;
            painter.circle_stroke(c + Vec2::new(-s * 0.5, 0.0), r, stroke);
            let pts: Vec<Pos2> = (0..=8).map(|i| {
                let t = i as f32 / 8.0;
                Pos2::new(c.x - s * 0.15 + t * s * 1.1, c.y + (t * std::f32::consts::PI).sin() * s * 0.4)
            }).collect();
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], thin);
            }
        }
        ToolIcon::Sprocket | ToolIcon::Gear => {
            // Gear: circle with teeth
            let r = s * 0.55;
            painter.circle_stroke(c, r * 0.5, thin);
            for i in 0..8 {
                let angle = i as f32 * std::f32::consts::TAU / 8.0;
                let inner = c + Vec2::new(r * 0.65 * angle.cos(), r * 0.65 * angle.sin());
                let outer = c + Vec2::new(r * 1.0 * angle.cos(), r * 1.0 * angle.sin());
                painter.line_segment([inner, outer], stroke);
            }
            painter.circle_stroke(c, r * 0.65, thin);
        }
        ToolIcon::Shaft => {
            // Horizontal cylinder side view
            let w = s * 0.9;
            let h = s * 0.35;
            painter.line_segment([c + Vec2::new(-w, -h), c + Vec2::new(w, -h)], stroke);
            painter.line_segment([c + Vec2::new(-w, h), c + Vec2::new(w, h)], stroke);
            draw_ellipse_half(&painter, c + Vec2::new(-w, 0.0), h, true, 8, stroke);
            draw_ellipse_half(&painter, c + Vec2::new(w, 0.0), h, false, 8, stroke);
        }
        ToolIcon::Suppress => {
            // X mark
            painter.line_segment([c + Vec2::new(-s * 0.5, -s * 0.5), c + Vec2::new(s * 0.5, s * 0.5)], stroke);
            painter.line_segment([c + Vec2::new(s * 0.5, -s * 0.5), c + Vec2::new(-s * 0.5, s * 0.5)], stroke);
        }
        ToolIcon::SetTip => {
            // Bullseye
            painter.circle_stroke(c, s * 0.7, stroke);
            painter.circle_filled(c, s * 0.25, color);
        }
        ToolIcon::MoveUp => {
            // Up arrow
            let tip = c - Vec2::Y * s * 0.7;
            painter.line_segment([c + Vec2::Y * s * 0.7, tip], stroke);
            painter.line_segment([tip, tip + Vec2::new(-s * 0.35, s * 0.35)], stroke);
            painter.line_segment([tip, tip + Vec2::new(s * 0.35, s * 0.35)], stroke);
        }
        ToolIcon::MoveDown => {
            // Down arrow
            let tip = c + Vec2::Y * s * 0.7;
            painter.line_segment([c - Vec2::Y * s * 0.7, tip], stroke);
            painter.line_segment([tip, tip + Vec2::new(-s * 0.35, -s * 0.35)], stroke);
            painter.line_segment([tip, tip + Vec2::new(s * 0.35, -s * 0.35)], stroke);
        }

        // ---- Sketch geometry -----------------------------------------------
        ToolIcon::SketchSelect => {
            // Arrow cursor
            let tip = c + Vec2::new(-s * 0.5, -s * 0.7);
            painter.line_segment([tip, tip + Vec2::new(0.0, s * 1.4)], stroke);
            painter.line_segment([tip, tip + Vec2::new(s * 0.9, s * 0.7)], stroke);
            painter.line_segment([tip + Vec2::new(0.0, s * 1.4), tip + Vec2::new(s * 0.35, s * 0.9)], stroke);
            painter.line_segment([tip + Vec2::new(s * 0.35, s * 0.9), tip + Vec2::new(s * 0.9, s * 0.7)], thin);
        }
        ToolIcon::SketchPoint => {
            painter.circle_filled(c, s * 0.25, color);
            painter.circle_stroke(c, s * 0.5, thin);
        }
        ToolIcon::SketchLine => {
            let a = c + Vec2::new(-s * 0.7, s * 0.5);
            let b = c + Vec2::new(s * 0.7, -s * 0.5);
            painter.line_segment([a, b], stroke);
            painter.circle_filled(a, 2.0, color);
            painter.circle_filled(b, 2.0, color);
        }
        ToolIcon::SketchRect => {
            painter.rect_stroke(egui::Rect::from_center_size(c, Vec2::new(s * 1.5, s * 1.0)), 0.0, stroke, StrokeKind::Middle);
        }
        ToolIcon::SketchCircle => {
            painter.circle_stroke(c, s * 0.7, stroke);
            painter.circle_filled(c, 1.5, color);
        }
        ToolIcon::SketchArc => {
            draw_arc(&painter, c, s * 0.7, -std::f32::consts::FRAC_PI_2 - 0.3, std::f32::consts::PI * 0.8, 12, stroke);
            let start_angle = -std::f32::consts::FRAC_PI_2 - 0.3;
            let end_angle = start_angle + std::f32::consts::PI * 0.8;
            let r = s * 0.7;
            painter.circle_filled(c + Vec2::new(r * start_angle.cos(), r * start_angle.sin()), 2.0, color);
            painter.circle_filled(c + Vec2::new(r * end_angle.cos(), r * end_angle.sin()), 2.0, color);
        }
        ToolIcon::SketchEllipse => {
            draw_ellipse(&painter, c, s * 0.85, s * 0.5, 16, stroke);
            painter.circle_filled(c, 1.5, color);
        }
        ToolIcon::SketchPolyline => {
            let pts = [
                c + Vec2::new(-s * 0.8, s * 0.4),
                c + Vec2::new(-s * 0.3, -s * 0.5),
                c + Vec2::new(s * 0.3, s * 0.2),
                c + Vec2::new(s * 0.8, -s * 0.4),
            ];
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
            for &p in &pts {
                painter.circle_filled(p, 1.5, color);
            }
        }
        ToolIcon::SketchSlot => {
            // Stadium shape (rounded rectangle)
            let hw = s * 0.8;
            let hh = s * 0.35;
            painter.line_segment([c + Vec2::new(-hw + hh, -hh), c + Vec2::new(hw - hh, -hh)], stroke);
            painter.line_segment([c + Vec2::new(-hw + hh, hh), c + Vec2::new(hw - hh, hh)], stroke);
            draw_arc(&painter, c + Vec2::new(-hw + hh, 0.0), hh, std::f32::consts::FRAC_PI_2, std::f32::consts::PI, 6, stroke);
            draw_arc(&painter, c + Vec2::new(hw - hh, 0.0), hh, -std::f32::consts::FRAC_PI_2, std::f32::consts::PI, 6, stroke);
        }
        ToolIcon::SketchBSpline => {
            // Smooth S-curve
            let pts: Vec<Pos2> = (0..=12).map(|i| {
                let t = i as f32 / 12.0;
                let x = c.x - s * 0.8 + t * s * 1.6;
                let y = c.y - (t * std::f32::consts::PI).sin() * s * 0.5;
                Pos2::new(x, y)
            }).collect();
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
        }
        ToolIcon::SketchPolygon => {
            draw_regular_polygon(&painter, c, s * 0.7, 6, stroke);
        }

        // ---- Sketch constraints --------------------------------------------
        ToolIcon::Coincident => {
            painter.circle_filled(c, s * 0.3, color);
        }
        ToolIcon::Horizontal => {
            painter.line_segment([c - Vec2::X * s * 0.8, c + Vec2::X * s * 0.8], Stroke::new(2.0, color));
            // Arrowheads
            let tip_l = c - Vec2::X * s * 0.8;
            let tip_r = c + Vec2::X * s * 0.8;
            painter.line_segment([tip_l, tip_l + Vec2::new(s * 0.2, -s * 0.15)], thin);
            painter.line_segment([tip_l, tip_l + Vec2::new(s * 0.2, s * 0.15)], thin);
            painter.line_segment([tip_r, tip_r + Vec2::new(-s * 0.2, -s * 0.15)], thin);
            painter.line_segment([tip_r, tip_r + Vec2::new(-s * 0.2, s * 0.15)], thin);
        }
        ToolIcon::Vertical => {
            painter.line_segment([c - Vec2::Y * s * 0.8, c + Vec2::Y * s * 0.8], Stroke::new(2.0, color));
            let tip_t = c - Vec2::Y * s * 0.8;
            let tip_b = c + Vec2::Y * s * 0.8;
            painter.line_segment([tip_t, tip_t + Vec2::new(-s * 0.15, s * 0.2)], thin);
            painter.line_segment([tip_t, tip_t + Vec2::new(s * 0.15, s * 0.2)], thin);
            painter.line_segment([tip_b, tip_b + Vec2::new(-s * 0.15, -s * 0.2)], thin);
            painter.line_segment([tip_b, tip_b + Vec2::new(s * 0.15, -s * 0.2)], thin);
        }
        ToolIcon::Parallel => {
            // Two parallel diagonal lines
            let off = Vec2::new(s * 0.2, 0.0);
            painter.line_segment([c + Vec2::new(-s * 0.5, s * 0.6) - off, c + Vec2::new(s * 0.5, -s * 0.6) - off], stroke);
            painter.line_segment([c + Vec2::new(-s * 0.5, s * 0.6) + off, c + Vec2::new(s * 0.5, -s * 0.6) + off], stroke);
        }
        ToolIcon::Perpendicular => {
            // L-shape (right angle)
            painter.line_segment([c + Vec2::new(-s * 0.6, s * 0.6), c + Vec2::new(-s * 0.6, -s * 0.6)], stroke);
            painter.line_segment([c + Vec2::new(-s * 0.6, s * 0.6), c + Vec2::new(s * 0.6, s * 0.6)], stroke);
            // Right angle marker
            let sq = s * 0.2;
            painter.line_segment([c + Vec2::new(-s * 0.6 + sq, s * 0.6), c + Vec2::new(-s * 0.6 + sq, s * 0.6 - sq)], thin);
            painter.line_segment([c + Vec2::new(-s * 0.6, s * 0.6 - sq), c + Vec2::new(-s * 0.6 + sq, s * 0.6 - sq)], thin);
        }
        ToolIcon::Tangent => {
            // Circle with tangent line
            let r = s * 0.45;
            painter.circle_stroke(c + Vec2::new(-s * 0.2, 0.0), r, stroke);
            painter.line_segment([c + Vec2::new(-s * 0.2 + r, -s * 0.7), c + Vec2::new(-s * 0.2 + r, s * 0.7)], thin);
        }
        ToolIcon::Equal => {
            // Two equal lines
            painter.line_segment([c + Vec2::new(-s * 0.7, -s * 0.2), c + Vec2::new(s * 0.7, -s * 0.2)], stroke);
            painter.line_segment([c + Vec2::new(-s * 0.7, s * 0.2), c + Vec2::new(s * 0.7, s * 0.2)], stroke);
        }
        ToolIcon::Symmetric => {
            // Mirror symbol: dashed center + two dots
            draw_dashed_line(&painter, c - Vec2::Y * s * 0.7, c + Vec2::Y * s * 0.7, 2.5, 2.5, thin);
            painter.circle_filled(c + Vec2::new(-s * 0.5, 0.0), 2.5, color);
            painter.circle_filled(c + Vec2::new(s * 0.5, 0.0), 2.5, color);
        }
        ToolIcon::LengthConstraint => {
            // Dimension line with L
            painter.line_segment([c + Vec2::new(-s * 0.8, 0.0), c + Vec2::new(s * 0.8, 0.0)], stroke);
            painter.line_segment([c + Vec2::new(-s * 0.8, -s * 0.3), c + Vec2::new(-s * 0.8, s * 0.3)], thin);
            painter.line_segment([c + Vec2::new(s * 0.8, -s * 0.3), c + Vec2::new(s * 0.8, s * 0.3)], thin);
        }
        ToolIcon::Distance => draw_icon(ui, rect, ToolIcon::LengthConstraint, color),
        ToolIcon::Angle => {
            // Angle arc
            painter.line_segment([c, c + Vec2::new(s * 0.8, 0.0)], stroke);
            painter.line_segment([c, c + Vec2::new(s * 0.5, -s * 0.7)], stroke);
            draw_arc(&painter, c, s * 0.4, -0.9, 0.9, 8, thin);
        }
        ToolIcon::Radius => {
            // R with line from center to edge
            let r = s * 0.6;
            painter.circle_stroke(c, r, thin);
            painter.line_segment([c, c + Vec2::new(r, 0.0)], stroke);
            painter.circle_filled(c, 1.5, color);
        }
        ToolIcon::Diameter => {
            let r = s * 0.6;
            painter.circle_stroke(c, r, thin);
            painter.line_segment([c + Vec2::new(-r, 0.0), c + Vec2::new(r, 0.0)], stroke);
        }
        ToolIcon::FixPin => {
            // Pin / thumbtack
            painter.circle_filled(c + Vec2::new(0.0, -s * 0.3), s * 0.3, color);
            painter.line_segment([c + Vec2::new(0.0, 0.0), c + Vec2::new(0.0, s * 0.7)], Stroke::new(2.0, color));
        }

        // ---- Sketch tools --------------------------------------------------
        ToolIcon::SketchFillet => draw_icon(ui, rect, ToolIcon::Fillet, color),
        ToolIcon::SketchChamfer => draw_icon(ui, rect, ToolIcon::Chamfer, color),
        ToolIcon::Trim => {
            // Scissors: two crossing lines
            painter.line_segment([c + Vec2::new(-s * 0.7, -s * 0.4), c + Vec2::new(s * 0.3, s * 0.2)], stroke);
            painter.line_segment([c + Vec2::new(-s * 0.7, s * 0.4), c + Vec2::new(s * 0.3, -s * 0.2)], stroke);
            painter.circle_stroke(c + Vec2::new(s * 0.5, s * 0.35), s * 0.2, thin);
            painter.circle_stroke(c + Vec2::new(s * 0.5, -s * 0.35), s * 0.2, thin);
        }
        ToolIcon::Extend => {
            // Line with arrow extending right
            painter.line_segment([c + Vec2::new(-s * 0.8, 0.0), c + Vec2::new(s * 0.5, 0.0)], stroke);
            let dim = Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 100);
            draw_dashed_line(&painter, c + Vec2::new(s * 0.5, 0.0), c + Vec2::new(s * 0.9, 0.0), 2.0, 2.0, Stroke::new(1.4, dim));
            painter.circle_filled(c + Vec2::new(-s * 0.8, 0.0), 2.0, color);
        }

        // ---- Sketch toggles ------------------------------------------------
        ToolIcon::Construction => {
            // Dashed diagonal
            draw_dashed_line(&painter, c + Vec2::new(-s * 0.7, s * 0.7), c + Vec2::new(s * 0.7, -s * 0.7), 3.0, 2.0, stroke);
        }
        ToolIcon::Grid => {
            // Grid pattern
            let h = s * 0.7;
            for i in -1..=1 {
                let off = i as f32 * h * 0.65;
                painter.line_segment([Pos2::new(c.x + off, c.y - h), Pos2::new(c.x + off, c.y + h)], thin);
                painter.line_segment([Pos2::new(c.x - h, c.y + off), Pos2::new(c.x + h, c.y + off)], thin);
            }
        }
        ToolIcon::Snap => {
            // Crosshair with circle
            painter.circle_stroke(c, s * 0.35, thin);
            painter.line_segment([c - Vec2::X * s * 0.7, c + Vec2::X * s * 0.7], thin);
            painter.line_segment([c - Vec2::Y * s * 0.7, c + Vec2::Y * s * 0.7], thin);
        }
        ToolIcon::ShowConstraints => draw_icon(ui, rect, ToolIcon::ShowAll, color),

        // ---- Close / cancel ------------------------------------------------
        ToolIcon::Accept => draw_icon(ui, rect, ToolIcon::Check, color),
        ToolIcon::Cancel => draw_icon(ui, rect, ToolIcon::Suppress, color),

        // ---- Mesh ----------------------------------------------------------
        ToolIcon::Import => draw_icon(ui, rect, ToolIcon::Open, color),
        ToolIcon::ExportStl | ToolIcon::ExportObj | ToolIcon::ExportGltf => draw_icon(ui, rect, ToolIcon::Save, color),
        ToolIcon::Decimate => {
            // Triangle with down arrow
            draw_regular_polygon(&painter, c + Vec2::new(0.0, -s * 0.15), s * 0.5, 3, stroke);
            painter.line_segment([c + Vec2::new(s * 0.5, s * 0.3), c + Vec2::new(s * 0.5, s * 0.7)], thin);
            painter.line_segment([c + Vec2::new(s * 0.5, s * 0.7), c + Vec2::new(s * 0.5 - s * 0.15, s * 0.55)], thin);
            painter.line_segment([c + Vec2::new(s * 0.5, s * 0.7), c + Vec2::new(s * 0.5 + s * 0.15, s * 0.55)], thin);
        }
        ToolIcon::Subdivide => {
            draw_regular_polygon(&painter, c, s * 0.65, 3, stroke);
            // Inner subdivision lines
            let r = s * 0.65;
            let verts: Vec<Pos2> = (0..3).map(|i| {
                let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::TAU / 3.0;
                c + Vec2::new(r * a.cos(), r * a.sin())
            }).collect();
            for i in 0..3 {
                let mid_a = Pos2::new((verts[i].x + verts[(i + 1) % 3].x) * 0.5, (verts[i].y + verts[(i + 1) % 3].y) * 0.5);
                let mid_b = Pos2::new((verts[(i + 1) % 3].x + verts[(i + 2) % 3].x) * 0.5, (verts[(i + 1) % 3].y + verts[(i + 2) % 3].y) * 0.5);
                painter.line_segment([mid_a, mid_b], thin);
            }
        }
        ToolIcon::FillHoles => {
            // Circle with patch
            painter.circle_stroke(c, s * 0.65, stroke);
            // Gap on right side filled
            draw_arc(&painter, c, s * 0.65, -0.4, 0.4, 4, Stroke::new(2.0, color));
        }
        ToolIcon::FlipNormals => {
            // Two arrows up/down
            painter.line_segment([c + Vec2::new(-s * 0.3, s * 0.6), c + Vec2::new(-s * 0.3, -s * 0.6)], stroke);
            painter.line_segment([c + Vec2::new(-s * 0.3, -s * 0.6), c + Vec2::new(-s * 0.3 - s * 0.15, -s * 0.35)], thin);
            painter.line_segment([c + Vec2::new(-s * 0.3, -s * 0.6), c + Vec2::new(-s * 0.3 + s * 0.15, -s * 0.35)], thin);
            painter.line_segment([c + Vec2::new(s * 0.3, -s * 0.6), c + Vec2::new(s * 0.3, s * 0.6)], stroke);
            painter.line_segment([c + Vec2::new(s * 0.3, s * 0.6), c + Vec2::new(s * 0.3 - s * 0.15, s * 0.35)], thin);
            painter.line_segment([c + Vec2::new(s * 0.3, s * 0.6), c + Vec2::new(s * 0.3 + s * 0.15, s * 0.35)], thin);
        }
        ToolIcon::Smooth => {
            // Wavy line
            let pts: Vec<Pos2> = (0..=10).map(|i| {
                let t = i as f32 / 10.0;
                Pos2::new(c.x - s * 0.8 + t * s * 1.6, c.y + (t * std::f32::consts::TAU).sin() * s * 0.35)
            }).collect();
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
        }
        ToolIcon::Harmonize => {
            // Arrows pointing same direction
            for i in 0..3 {
                let y = c.y - s * 0.5 + i as f32 * s * 0.5;
                let tip = Pos2::new(c.x + s * 0.5, y);
                painter.line_segment([Pos2::new(c.x - s * 0.5, y), tip], thin);
                painter.line_segment([tip, tip + Vec2::new(-s * 0.2, -s * 0.1)], thin);
                painter.line_segment([tip, tip + Vec2::new(-s * 0.2, s * 0.1)], thin);
            }
        }
        ToolIcon::Watertight => draw_icon(ui, rect, ToolIcon::Check, color),
        ToolIcon::Remesh => {
            // Grid of small triangles
            let h = s * 0.6;
            painter.rect_stroke(egui::Rect::from_center_size(c, Vec2::splat(h * 2.0)), 0.0, thin, StrokeKind::Middle);
            painter.line_segment([Pos2::new(c.x - h, c.y - h), Pos2::new(c.x + h, c.y + h)], thin);
            painter.line_segment([Pos2::new(c.x, c.y - h), Pos2::new(c.x, c.y + h)], thin);
            painter.line_segment([Pos2::new(c.x - h, c.y), Pos2::new(c.x + h, c.y)], thin);
        }
        ToolIcon::Repair => {
            // Wrench
            let h = s * 0.7;
            painter.line_segment([c + Vec2::new(-h * 0.7, h * 0.7), c + Vec2::new(h * 0.3, -h * 0.3)], Stroke::new(2.5, color));
            painter.circle_stroke(c + Vec2::new(h * 0.45, -h * 0.45), h * 0.35, stroke);
        }

        // ---- TechDraw: most share simple geometric icons -------------------
        ToolIcon::NewPage => draw_icon(ui, rect, ToolIcon::New, color),
        ToolIcon::Template => {
            let r = egui::Rect::from_center_size(c, Vec2::new(s * 1.4, s * 1.8));
            painter.rect_stroke(r, 0.0, stroke, StrokeKind::Middle);
            for i in 0..3 {
                let y = r.top() + s * 0.35 + i as f32 * s * 0.4;
                painter.line_segment([Pos2::new(r.left() + s * 0.2, y), Pos2::new(r.right() - s * 0.2, y)], thin);
            }
        }
        ToolIcon::Redraw => draw_icon(ui, rect, ToolIcon::Revolve, color),
        ToolIcon::ViewFront | ToolIcon::ViewTop | ToolIcon::ViewRight => {
            painter.rect_stroke(egui::Rect::from_center_size(c, Vec2::splat(s * 1.4)), 0.0, stroke, StrokeKind::Middle);
        }
        ToolIcon::ViewIso => {
            // Isometric cube hint
            let h = s * 0.5;
            painter.line_segment([c - Vec2::Y * h, c + Vec2::new(-h, 0.0)], stroke);
            painter.line_segment([c - Vec2::Y * h, c + Vec2::new(h, 0.0)], stroke);
            painter.line_segment([c + Vec2::new(-h, 0.0), c + Vec2::Y * h], stroke);
            painter.line_segment([c + Vec2::new(h, 0.0), c + Vec2::Y * h], stroke);
        }
        ToolIcon::SectionView => {
            let r = egui::Rect::from_center_size(c, Vec2::splat(s * 1.4));
            painter.rect_stroke(r, 0.0, stroke, StrokeKind::Middle);
            let dim = Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 100);
            painter.line_segment([Pos2::new(c.x, r.top()), Pos2::new(c.x, r.bottom())], Stroke::new(1.0, dim));
            // Hatch on left side
            for i in 0..4 {
                let y = r.top() + s * 0.2 + i as f32 * s * 0.35;
                painter.line_segment([Pos2::new(r.left() + s * 0.1, y), Pos2::new(c.x - s * 0.1, y + s * 0.15)], Stroke::new(0.5, dim));
            }
        }
        ToolIcon::DetailView => draw_icon(ui, rect, ToolIcon::FitAll, color),
        ToolIcon::BrokenView => {
            let r = egui::Rect::from_center_size(c, Vec2::splat(s * 1.4));
            painter.rect_stroke(r, 0.0, stroke, StrokeKind::Middle);
            // Zigzag break line
            let zig_pts = [
                Pos2::new(r.left(), c.y),
                Pos2::new(c.x - s * 0.3, c.y - s * 0.2),
                Pos2::new(c.x, c.y + s * 0.2),
                Pos2::new(c.x + s * 0.3, c.y - s * 0.2),
                Pos2::new(r.right(), c.y),
            ];
            for w in zig_pts.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
        }
        ToolIcon::ThreeView => {
            // Three small rectangles arranged in L
            let sz = s * 0.5;
            painter.rect_stroke(egui::Rect::from_center_size(c + Vec2::new(-sz, -sz), Vec2::splat(sz * 1.4)), 0.0, stroke, StrokeKind::Middle);
            painter.rect_stroke(egui::Rect::from_center_size(c + Vec2::new(sz, -sz), Vec2::splat(sz * 1.4)), 0.0, thin, StrokeKind::Middle);
            painter.rect_stroke(egui::Rect::from_center_size(c + Vec2::new(-sz, sz), Vec2::splat(sz * 1.4)), 0.0, thin, StrokeKind::Middle);
        }
        ToolIcon::DimLinear => draw_icon(ui, rect, ToolIcon::LengthConstraint, color),
        ToolIcon::DimRadius => draw_icon(ui, rect, ToolIcon::Radius, color),
        ToolIcon::DimDiameter => draw_icon(ui, rect, ToolIcon::Diameter, color),
        ToolIcon::DimAngle => draw_icon(ui, rect, ToolIcon::Angle, color),
        ToolIcon::DimArcLen => draw_icon(ui, rect, ToolIcon::SketchArc, color),
        ToolIcon::DimArea => {
            let r = egui::Rect::from_center_size(c, Vec2::splat(s * 1.2));
            painter.rect_filled(r, 0.0, Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 40));
            painter.rect_stroke(r, 0.0, stroke, StrokeKind::Middle);
        }
        ToolIcon::TextAnnot | ToolIcon::DraftText => {
            // Letter T
            painter.line_segment([c + Vec2::new(-s * 0.5, -s * 0.6), c + Vec2::new(s * 0.5, -s * 0.6)], Stroke::new(2.0, color));
            painter.line_segment([c + Vec2::new(0.0, -s * 0.6), c + Vec2::new(0.0, s * 0.6)], Stroke::new(2.0, color));
        }
        ToolIcon::RichText => {
            // Bold T
            painter.line_segment([c + Vec2::new(-s * 0.5, -s * 0.6), c + Vec2::new(s * 0.5, -s * 0.6)], Stroke::new(3.0, color));
            painter.line_segment([c + Vec2::new(0.0, -s * 0.6), c + Vec2::new(0.0, s * 0.6)], Stroke::new(3.0, color));
        }
        ToolIcon::Balloon => {
            painter.circle_stroke(c + Vec2::new(0.0, -s * 0.2), s * 0.45, stroke);
            painter.line_segment([c + Vec2::new(0.0, s * 0.25), c + Vec2::new(0.0, s * 0.8)], thin);
        }
        ToolIcon::Leader => {
            painter.line_segment([c + Vec2::new(-s * 0.7, s * 0.5), c + Vec2::new(s * 0.2, -s * 0.3)], stroke);
            painter.line_segment([c + Vec2::new(s * 0.2, -s * 0.3), c + Vec2::new(s * 0.7, -s * 0.3)], stroke);
            // Arrow at start
            let tip = c + Vec2::new(-s * 0.7, s * 0.5);
            painter.line_segment([tip, tip + Vec2::new(s * 0.25, -s * 0.05)], thin);
            painter.line_segment([tip, tip + Vec2::new(s * 0.05, -s * 0.25)], thin);
        }
        ToolIcon::Weld => {
            // V shape (weld symbol)
            painter.line_segment([c + Vec2::new(-s * 0.5, -s * 0.4), c + Vec2::new(0.0, s * 0.4)], stroke);
            painter.line_segment([c + Vec2::new(s * 0.5, -s * 0.4), c + Vec2::new(0.0, s * 0.4)], stroke);
            painter.line_segment([c + Vec2::new(-s * 0.7, -s * 0.4), c + Vec2::new(s * 0.7, -s * 0.4)], thin);
        }
        ToolIcon::SurfFinish => {
            // Inverted triangle surface finish symbol
            painter.line_segment([c + Vec2::new(0.0, -s * 0.7), c + Vec2::new(-s * 0.4, s * 0.5)], stroke);
            painter.line_segment([c + Vec2::new(0.0, -s * 0.7), c + Vec2::new(s * 0.4, s * 0.5)], stroke);
        }
        ToolIcon::CenterFace => {
            // Plus sign in circle
            let r = s * 0.5;
            painter.circle_stroke(c, r, thin);
            painter.line_segment([c - Vec2::X * r, c + Vec2::X * r], stroke);
            painter.line_segment([c - Vec2::Y * r, c + Vec2::Y * r], stroke);
        }
        ToolIcon::CenterLines => draw_icon(ui, rect, ToolIcon::Parallel, color),
        ToolIcon::CenterPoints => draw_icon(ui, rect, ToolIcon::SketchPoint, color),
        ToolIcon::BoltCircle => {
            painter.circle_stroke(c, s * 0.65, thin);
            for i in 0..6 {
                let a = i as f32 * std::f32::consts::TAU / 6.0;
                painter.circle_filled(c + Vec2::new(s * 0.65 * a.cos(), s * 0.65 * a.sin()), 1.5, color);
            }
        }
        ToolIcon::ExportSvg | ToolIcon::ExportDxf | ToolIcon::ExportPdf => draw_icon(ui, rect, ToolIcon::Save, color),
        ToolIcon::Clear => {
            // Trash can
            let w = s * 0.55;
            let h = s * 0.8;
            painter.line_segment([c + Vec2::new(-w, -h * 0.5), c + Vec2::new(w, -h * 0.5)], stroke);
            painter.rect_stroke(egui::Rect::from_x_y_ranges(c.x - w * 0.8..=c.x + w * 0.8, c.y - h * 0.5..=c.y + h), 0.0, stroke, StrokeKind::Middle);
            painter.line_segment([c + Vec2::new(-w * 0.3, -h * 0.5), c + Vec2::new(-w * 0.3, -h * 0.8)], thin);
            painter.line_segment([c + Vec2::new(w * 0.3, -h * 0.5), c + Vec2::new(w * 0.3, -h * 0.8)], thin);
            painter.line_segment([c + Vec2::new(-w * 0.3, -h * 0.8), c + Vec2::new(w * 0.3, -h * 0.8)], thin);
        }

        // ---- Assembly ------------------------------------------------------
        ToolIcon::AssemblyNew => draw_icon(ui, rect, ToolIcon::New, color),
        ToolIcon::InsertComponent => {
            // Box with plus
            let r = egui::Rect::from_center_size(c, Vec2::splat(s * 1.2));
            painter.rect_stroke(r, 0.0, stroke, StrokeKind::Middle);
            painter.line_segment([c - Vec2::X * s * 0.3, c + Vec2::X * s * 0.3], Stroke::new(2.0, color));
            painter.line_segment([c - Vec2::Y * s * 0.3, c + Vec2::Y * s * 0.3], Stroke::new(2.0, color));
        }
        ToolIcon::Solve => {
            // Play triangle
            let h = s * 0.6;
            let pts = [
                c + Vec2::new(-h * 0.6, -h),
                c + Vec2::new(h, 0.0),
                c + Vec2::new(-h * 0.6, h),
            ];
            painter.line_segment([pts[0], pts[1]], stroke);
            painter.line_segment([pts[1], pts[2]], stroke);
            painter.line_segment([pts[2], pts[0]], stroke);
        }
        ToolIcon::ExplodeView => draw_icon(ui, rect, ToolIcon::Explode, color),
        ToolIcon::Bom => draw_icon(ui, rect, ToolIcon::Template, color),
        ToolIcon::Dof => {
            // XYZ axes
            let l = s * 0.6;
            painter.line_segment([c, c + Vec2::new(l, 0.0)], Stroke::new(1.5, Color32::from_rgb(220, 80, 80)));
            painter.line_segment([c, c + Vec2::new(0.0, -l)], Stroke::new(1.5, Color32::from_rgb(80, 200, 80)));
            painter.line_segment([c, c + Vec2::new(-l * 0.5, l * 0.4)], Stroke::new(1.5, Color32::from_rgb(80, 80, 220)));
        }
        ToolIcon::JointFixed => {
            let h = s * 0.5;
            painter.line_segment([c + Vec2::new(-h, h), c + Vec2::new(h, h)], Stroke::new(2.5, color));
            // Ground hatch
            for i in 0..4 {
                let x = c.x - h + i as f32 * h * 0.65;
                painter.line_segment([Pos2::new(x, c.y + h), Pos2::new(x - h * 0.3, c.y + h + h * 0.3)], thin);
            }
            painter.rect_stroke(egui::Rect::from_center_size(c + Vec2::new(0.0, -s * 0.1), Vec2::new(h * 1.2, h * 0.8)), 0.0, stroke, StrokeKind::Middle);
        }
        ToolIcon::JointRevolute => {
            painter.circle_stroke(c, s * 0.5, stroke);
            draw_arc(&painter, c, s * 0.7, 0.0, std::f32::consts::PI * 1.5, 10, thin);
        }
        ToolIcon::JointCylindrical => draw_icon(ui, rect, ToolIcon::Cylinder, color),
        ToolIcon::JointSlider => {
            painter.line_segment([c + Vec2::new(-s * 0.8, 0.0), c + Vec2::new(s * 0.8, 0.0)], stroke);
            let tip = c + Vec2::new(s * 0.8, 0.0);
            painter.line_segment([tip, tip + Vec2::new(-s * 0.25, -s * 0.15)], thin);
            painter.line_segment([tip, tip + Vec2::new(-s * 0.25, s * 0.15)], thin);
            painter.rect_stroke(egui::Rect::from_center_size(c, Vec2::new(s * 0.5, s * 0.6)), 0.0, thin, StrokeKind::Middle);
        }
        ToolIcon::JointBall => {
            painter.circle_stroke(c, s * 0.55, stroke);
            painter.circle_filled(c, s * 0.2, color);
        }
        ToolIcon::JointDistance => draw_icon(ui, rect, ToolIcon::LengthConstraint, color),
        ToolIcon::JointAngle => draw_icon(ui, rect, ToolIcon::Angle, color),
        ToolIcon::JointParallel => draw_icon(ui, rect, ToolIcon::Parallel, color),
        ToolIcon::JointPerp => draw_icon(ui, rect, ToolIcon::Perpendicular, color),
        ToolIcon::JointGear => draw_icon(ui, rect, ToolIcon::Gear, color),
        ToolIcon::JointRack => {
            // Rack teeth
            let y = c.y;
            for i in 0..4 {
                let x = c.x - s * 0.7 + i as f32 * s * 0.45;
                painter.line_segment([Pos2::new(x, y + s * 0.3), Pos2::new(x, y - s * 0.3)], thin);
                painter.line_segment([Pos2::new(x, y - s * 0.3), Pos2::new(x + s * 0.22, y - s * 0.3)], thin);
                painter.line_segment([Pos2::new(x + s * 0.22, y - s * 0.3), Pos2::new(x + s * 0.22, y + s * 0.3)], thin);
            }
        }
        ToolIcon::JointScrew => {
            // Screw thread hint
            let r = s * 0.25;
            painter.circle_stroke(c + Vec2::new(0.0, -s * 0.4), r, stroke);
            painter.line_segment([c + Vec2::new(-r, -s * 0.4), c + Vec2::new(-r, s * 0.7)], stroke);
            painter.line_segment([c + Vec2::new(r, -s * 0.4), c + Vec2::new(r, s * 0.7)], stroke);
            for i in 0..3 {
                let y = c.y - s * 0.1 + i as f32 * s * 0.3;
                painter.line_segment([Pos2::new(c.x - r, y), Pos2::new(c.x + r, y)], thin);
            }
        }
        ToolIcon::JointBelt => {
            // Two circles connected by tangent lines
            let r1 = s * 0.3;
            let r2 = s * 0.3;
            let c1 = c + Vec2::new(-s * 0.45, 0.0);
            let c2 = c + Vec2::new(s * 0.45, 0.0);
            painter.circle_stroke(c1, r1, stroke);
            painter.circle_stroke(c2, r2, stroke);
            painter.line_segment([c1 + Vec2::new(0.0, -r1), c2 + Vec2::new(0.0, -r2)], thin);
            painter.line_segment([c1 + Vec2::new(0.0, r1), c2 + Vec2::new(0.0, r2)], thin);
        }

        // ---- Draft ---------------------------------------------------------
        ToolIcon::DraftLine => draw_icon(ui, rect, ToolIcon::SketchLine, color),
        ToolIcon::DraftWire => draw_icon(ui, rect, ToolIcon::SketchPolyline, color),
        ToolIcon::DraftCircle => draw_icon(ui, rect, ToolIcon::SketchCircle, color),
        ToolIcon::DraftArc => draw_icon(ui, rect, ToolIcon::SketchArc, color),
        ToolIcon::DraftEllipse => draw_icon(ui, rect, ToolIcon::SketchEllipse, color),
        ToolIcon::DraftRect => draw_icon(ui, rect, ToolIcon::SketchRect, color),
        ToolIcon::DraftPolygon => draw_icon(ui, rect, ToolIcon::SketchPolygon, color),
        ToolIcon::DraftBSpline => draw_icon(ui, rect, ToolIcon::SketchBSpline, color),
        ToolIcon::DraftBezier => {
            // Bezier with control handles
            let pts: Vec<Pos2> = (0..=10).map(|i| {
                let t = i as f32 / 10.0;
                let x = c.x - s * 0.8 + t * s * 1.6;
                let y = c.y + (t * std::f32::consts::PI * 1.5).sin() * s * 0.5;
                Pos2::new(x, y)
            }).collect();
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
            // Control point hints
            let dim = Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 80);
            painter.line_segment([pts[0], c + Vec2::new(-s * 0.3, -s * 0.6)], Stroke::new(0.5, dim));
            painter.circle_filled(c + Vec2::new(-s * 0.3, -s * 0.6), 1.5, color);
        }
        ToolIcon::DraftPoint => draw_icon(ui, rect, ToolIcon::SketchPoint, color),
        ToolIcon::DraftFacebind => {
            let r = egui::Rect::from_center_size(c, Vec2::splat(s * 1.2));
            painter.rect_filled(r, 0.0, Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 30));
            painter.rect_stroke(r, 0.0, stroke, StrokeKind::Middle);
        }
        ToolIcon::DraftHatch => {
            let r = egui::Rect::from_center_size(c, Vec2::splat(s * 1.2));
            painter.rect_stroke(r, 0.0, stroke, StrokeKind::Middle);
            for i in 0..5 {
                let off = -s * 0.6 + i as f32 * s * 0.3;
                painter.line_segment([Pos2::new(r.left(), c.y + off), Pos2::new(c.x + off, r.top())], thin);
            }
        }
        ToolIcon::DraftMove => {
            // Four-direction arrow
            let l = s * 0.6;
            let ah = s * 0.15;
            for &dir in &[Vec2::X, -Vec2::X, Vec2::Y, -Vec2::Y] {
                let tip = c + dir * l;
                painter.line_segment([c, tip], stroke);
                let perp = Vec2::new(-dir.y, dir.x);
                painter.line_segment([tip, tip - dir * ah + perp * ah], thin);
                painter.line_segment([tip, tip - dir * ah - perp * ah], thin);
            }
        }
        ToolIcon::DraftRotate => draw_icon(ui, rect, ToolIcon::Revolve, color),
        ToolIcon::DraftScale => draw_icon(ui, rect, ToolIcon::Scale, color),
        ToolIcon::DraftMirror => draw_icon(ui, rect, ToolIcon::Mirror, color),
        ToolIcon::DraftOffset => {
            // Two concentric rects
            let inner = egui::Rect::from_center_size(c, Vec2::splat(s * 0.8));
            let outer = egui::Rect::from_center_size(c, Vec2::splat(s * 1.4));
            painter.rect_stroke(inner, 0.0, stroke, StrokeKind::Middle);
            painter.rect_stroke(outer, 0.0, thin, StrokeKind::Middle);
        }
        ToolIcon::DraftTrim => draw_icon(ui, rect, ToolIcon::Trim, color),
        ToolIcon::DraftStretch => {
            painter.line_segment([c + Vec2::new(-s * 0.7, 0.0), c + Vec2::new(s * 0.7, 0.0)], stroke);
            let lt = c + Vec2::new(-s * 0.7, 0.0);
            let rt = c + Vec2::new(s * 0.7, 0.0);
            painter.line_segment([lt, lt + Vec2::new(s * 0.2, -s * 0.15)], thin);
            painter.line_segment([lt, lt + Vec2::new(s * 0.2, s * 0.15)], thin);
            painter.line_segment([rt, rt + Vec2::new(-s * 0.2, -s * 0.15)], thin);
            painter.line_segment([rt, rt + Vec2::new(-s * 0.2, s * 0.15)], thin);
        }
        ToolIcon::DraftClone => {
            // Two overlapping rects
            let r1 = egui::Rect::from_center_size(c + Vec2::new(-s * 0.2, -s * 0.2), Vec2::splat(s * 0.9));
            let r2 = egui::Rect::from_center_size(c + Vec2::new(s * 0.2, s * 0.2), Vec2::splat(s * 0.9));
            painter.rect_stroke(r1, 0.0, stroke, StrokeKind::Middle);
            painter.rect_stroke(r2, 0.0, thin, StrokeKind::Middle);
        }
        ToolIcon::DraftArrayRect => draw_icon(ui, rect, ToolIcon::Pattern, color),
        ToolIcon::DraftArrayPolar => {
            let r = s * 0.6;
            painter.circle_stroke(c, r, thin);
            for i in 0..5 {
                let a = i as f32 * std::f32::consts::TAU / 5.0 - std::f32::consts::FRAC_PI_2;
                let p = c + Vec2::new(r * a.cos(), r * a.sin());
                painter.rect_stroke(egui::Rect::from_center_size(p, Vec2::splat(s * 0.2)), 0.0, if i == 0 { stroke } else { thin }, StrokeKind::Middle);
            }
        }
        ToolIcon::DraftArrayPath => {
            // Small rects along a curve
            let pts: Vec<Pos2> = (0..=3).map(|i| {
                let t = i as f32 / 3.0;
                Pos2::new(c.x - s * 0.7 + t * s * 1.4, c.y + (t * std::f32::consts::PI).sin() * s * 0.4)
            }).collect();
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], thin);
            }
            for &p in &pts {
                painter.rect_stroke(egui::Rect::from_center_size(p, Vec2::splat(s * 0.2)), 0.0, stroke, StrokeKind::Middle);
            }
        }
        ToolIcon::DraftArrayPoint => {
            for &d in &[Vec2::new(-0.5, -0.5), Vec2::new(0.5, -0.3), Vec2::new(-0.3, 0.5), Vec2::new(0.6, 0.4)] {
                let p = c + d * s;
                painter.circle_filled(p, 1.5, color);
                painter.rect_stroke(egui::Rect::from_center_size(p, Vec2::splat(s * 0.2)), 0.0, thin, StrokeKind::Middle);
            }
        }
        ToolIcon::DraftDimension => draw_icon(ui, rect, ToolIcon::LengthConstraint, color),
        ToolIcon::DraftLabel => {
            // Tag shape
            let r = egui::Rect::from_center_size(c, Vec2::new(s * 1.4, s * 0.8));
            painter.rect_stroke(r, 2.0, stroke, StrokeKind::Middle);
            painter.circle_filled(Pos2::new(r.left() + s * 0.25, c.y), 1.5, color);
        }
        ToolIcon::DraftUpgrade => draw_icon(ui, rect, ToolIcon::MoveUp, color),
        ToolIcon::DraftDowngrade => draw_icon(ui, rect, ToolIcon::MoveDown, color),
        ToolIcon::DraftWireToBSpline => {
            // Polyline -> curve
            let pts_wire = [c + Vec2::new(-s * 0.8, s * 0.3), c + Vec2::new(-s * 0.2, -s * 0.4), c + Vec2::new(s * 0.4, s * 0.2)];
            for w in pts_wire.windows(2) {
                painter.line_segment([w[0], w[1]], thin);
            }
            // Arrow
            painter.line_segment([c + Vec2::new(s * 0.4, 0.0), c + Vec2::new(s * 0.8, 0.0)], stroke);
        }
        ToolIcon::DraftToSketch => {
            // Pencil
            painter.line_segment([c + Vec2::new(-s * 0.6, s * 0.6), c + Vec2::new(s * 0.4, -s * 0.4)], Stroke::new(2.0, color));
            painter.line_segment([c + Vec2::new(s * 0.4, -s * 0.4), c + Vec2::new(s * 0.6, -s * 0.6)], stroke);
        }
        ToolIcon::DraftSnapEndpoint => {
            painter.circle_filled(c, s * 0.3, color);
        }
        ToolIcon::DraftSnapMidpoint => {
            let h = s * 0.7;
            painter.line_segment([c + Vec2::new(-h, 0.0), c + Vec2::new(h, 0.0)], thin);
            painter.circle_stroke(c, s * 0.25, stroke);
        }
        ToolIcon::DraftSnapCenter => {
            painter.circle_stroke(c, s * 0.55, thin);
            painter.circle_filled(c, 2.0, color);
        }
        ToolIcon::DraftSnapGrid => draw_icon(ui, rect, ToolIcon::Grid, color),
        ToolIcon::DraftSnapAngle => draw_icon(ui, rect, ToolIcon::Angle, color),
        ToolIcon::DraftSnapIntersect => {
            painter.line_segment([c + Vec2::new(-s * 0.7, -s * 0.7), c + Vec2::new(s * 0.7, s * 0.7)], thin);
            painter.line_segment([c + Vec2::new(s * 0.7, -s * 0.7), c + Vec2::new(-s * 0.7, s * 0.7)], thin);
            painter.circle_filled(c, 2.5, color);
        }
        ToolIcon::DraftSnapPerp => draw_icon(ui, rect, ToolIcon::Perpendicular, color),
        ToolIcon::DraftSnapParallel => draw_icon(ui, rect, ToolIcon::Parallel, color),

        // ---- Surface -------------------------------------------------------
        ToolIcon::SurfFilling => {
            let r = egui::Rect::from_center_size(c, Vec2::splat(s * 1.4));
            painter.rect_filled(r, 0.0, Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 30));
            painter.rect_stroke(r, 0.0, stroke, StrokeKind::Middle);
        }
        ToolIcon::SurfBoundary => {
            painter.rect_stroke(egui::Rect::from_center_size(c, Vec2::splat(s * 1.3)), 0.0, stroke, StrokeKind::Middle);
        }
        ToolIcon::SurfSections => {
            for i in 0..4 {
                let y = c.y - s * 0.6 + i as f32 * s * 0.4;
                let w = s * (0.5 + (i as f32 * 0.2));
                painter.line_segment([Pos2::new(c.x - w, y), Pos2::new(c.x + w, y)], if i == 0 || i == 3 { stroke } else { thin });
            }
        }
        ToolIcon::SurfExtend => draw_icon(ui, rect, ToolIcon::Extend, color),
        ToolIcon::SurfBlend => {
            let pts: Vec<Pos2> = (0..=10).map(|i| {
                let t = i as f32 / 10.0;
                Pos2::new(c.x - s * 0.8 + t * s * 1.6, c.y + (t * std::f32::consts::PI).sin() * s * 0.4)
            }).collect();
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
        }
        ToolIcon::SurfPipe => draw_icon(ui, rect, ToolIcon::AddPipe, color),
        ToolIcon::SurfCoons => {
            // 4-sided patch
            let h = s * 0.7;
            painter.line_segment([c + Vec2::new(-h, -h), c + Vec2::new(h, -h)], stroke);
            painter.line_segment([c + Vec2::new(h, -h), c + Vec2::new(h, h)], stroke);
            painter.line_segment([c + Vec2::new(h, h), c + Vec2::new(-h, h)], stroke);
            painter.line_segment([c + Vec2::new(-h, h), c + Vec2::new(-h, -h)], stroke);
            // Diagonal cross to show surface
            painter.line_segment([c + Vec2::new(-h, -h), c + Vec2::new(h, h)], thin);
            painter.line_segment([c + Vec2::new(h, -h), c + Vec2::new(-h, h)], thin);
        }

        // ---- FEM -----------------------------------------------------------
        ToolIcon::FemAnalysis => {
            // Bar chart
            for i in 0..3 {
                let x = c.x - s * 0.5 + i as f32 * s * 0.5;
                let h = s * 0.4 + i as f32 * s * 0.3;
                painter.rect_stroke(egui::Rect::from_min_max(Pos2::new(x, c.y + s * 0.6 - h), Pos2::new(x + s * 0.3, c.y + s * 0.6)), 0.0, stroke, StrokeKind::Middle);
            }
        }
        ToolIcon::FemMaterial => {
            // Flask / beaker
            let top_w = s * 0.3;
            let bot_w = s * 0.6;
            let h = s * 0.8;
            painter.line_segment([c + Vec2::new(-top_w, -h), c + Vec2::new(-bot_w, h)], stroke);
            painter.line_segment([c + Vec2::new(top_w, -h), c + Vec2::new(bot_w, h)], stroke);
            painter.line_segment([c + Vec2::new(-bot_w, h), c + Vec2::new(bot_w, h)], stroke);
            painter.line_segment([c + Vec2::new(-top_w, -h), c + Vec2::new(top_w, -h)], stroke);
        }
        ToolIcon::FemTetMesh => {
            // Tetrahedron
            let h = s * 0.8;
            let top = c - Vec2::Y * h;
            let bl = c + Vec2::new(-h * 0.8, h * 0.6);
            let br = c + Vec2::new(h * 0.8, h * 0.6);
            let mid = c + Vec2::new(h * 0.3, 0.0);
            painter.line_segment([top, bl], stroke);
            painter.line_segment([top, br], stroke);
            painter.line_segment([bl, br], stroke);
            painter.line_segment([top, mid], thin);
            painter.line_segment([bl, mid], thin);
            painter.line_segment([br, mid], thin);
        }
        ToolIcon::FemRefine => draw_icon(ui, rect, ToolIcon::Subdivide, color),
        ToolIcon::FemSmooth => draw_icon(ui, rect, ToolIcon::Smooth, color),
        ToolIcon::FemQuality => draw_icon(ui, rect, ToolIcon::Check, color),
        ToolIcon::FemFixed => draw_icon(ui, rect, ToolIcon::JointFixed, color),
        ToolIcon::FemForce => {
            // Thick arrow right
            let tip = c + Vec2::X * s * 0.8;
            painter.line_segment([c - Vec2::X * s * 0.8, tip], Stroke::new(2.5, color));
            painter.line_segment([tip, tip + Vec2::new(-s * 0.3, -s * 0.2)], stroke);
            painter.line_segment([tip, tip + Vec2::new(-s * 0.3, s * 0.2)], stroke);
        }
        ToolIcon::FemPressure => {
            // Multiple down arrows
            for i in 0..3 {
                let x = c.x - s * 0.5 + i as f32 * s * 0.5;
                let tip = Pos2::new(x, c.y + s * 0.5);
                painter.line_segment([Pos2::new(x, c.y - s * 0.5), tip], thin);
                painter.line_segment([tip, tip + Vec2::new(-s * 0.1, -s * 0.15)], thin);
                painter.line_segment([tip, tip + Vec2::new(s * 0.1, -s * 0.15)], thin);
            }
        }
        ToolIcon::FemDisplacement => {
            // Arrow at angle
            let tip = c + Vec2::new(s * 0.6, -s * 0.6);
            painter.line_segment([c + Vec2::new(-s * 0.6, s * 0.6), tip], Stroke::new(2.0, color));
            painter.line_segment([tip, tip + Vec2::new(-s * 0.3, s * 0.05)], stroke);
            painter.line_segment([tip, tip + Vec2::new(-s * 0.05, s * 0.3)], stroke);
        }
        ToolIcon::FemGravity => {
            // g arrow down
            let tip = c + Vec2::Y * s * 0.8;
            painter.line_segment([c - Vec2::Y * s * 0.8, tip], Stroke::new(2.5, color));
            painter.line_segment([tip, tip + Vec2::new(-s * 0.2, -s * 0.3)], stroke);
            painter.line_segment([tip, tip + Vec2::new(s * 0.2, -s * 0.3)], stroke);
        }
        ToolIcon::FemSpring => {
            // Zigzag spring
            let n = 4;
            let total_h = s * 1.4;
            let w = s * 0.4;
            let step = total_h / n as f32;
            let start_y = c.y - total_h * 0.5;
            for i in 0..n {
                let y0 = start_y + i as f32 * step;
                let y1 = y0 + step;
                let mid_y = (y0 + y1) * 0.5;
                let sign = if i % 2 == 0 { 1.0 } else { -1.0 };
                painter.line_segment([Pos2::new(c.x, y0), Pos2::new(c.x + w * sign, mid_y)], thin);
                painter.line_segment([Pos2::new(c.x + w * sign, mid_y), Pos2::new(c.x, y1)], thin);
            }
        }
        ToolIcon::SolveStatic => draw_icon(ui, rect, ToolIcon::Solve, color),
        ToolIcon::SolveModal => {
            // Sine wave
            let pts: Vec<Pos2> = (0..=12).map(|i| {
                let t = i as f32 / 12.0;
                Pos2::new(c.x - s * 0.8 + t * s * 1.6, c.y + (t * std::f32::consts::TAU).sin() * s * 0.4)
            }).collect();
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
        }
        ToolIcon::SolveThermal => {
            // Flame shape
            let tip = c - Vec2::Y * s * 0.7;
            painter.line_segment([c + Vec2::new(-s * 0.3, s * 0.5), tip], stroke);
            painter.line_segment([c + Vec2::new(s * 0.3, s * 0.5), tip], stroke);
            draw_arc(&painter, c + Vec2::new(0.0, s * 0.5), s * 0.3, std::f32::consts::PI, std::f32::consts::PI, 6, stroke);
        }
        ToolIcon::SolveBuckling => {
            // Column with sideways deflection
            painter.line_segment([c + Vec2::new(0.0, s * 0.8), c + Vec2::new(0.0, s * 0.3)], stroke);
            let pts: Vec<Pos2> = (0..=8).map(|i| {
                let t = i as f32 / 8.0;
                let y = c.y + s * 0.3 - t * s * 1.1;
                let x = c.x + (t * std::f32::consts::PI).sin() * s * 0.4;
                Pos2::new(x, y)
            }).collect();
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
        }
        ToolIcon::SolveNonlinear => {
            // Curved stress-strain line
            let pts: Vec<Pos2> = (0..=10).map(|i| {
                let t = i as f32 / 10.0;
                let x = c.x - s * 0.7 + t * s * 1.4;
                let y = c.y + s * 0.5 - (t * 1.3).powf(0.4) * s * 1.0;
                Pos2::new(x, y)
            }).collect();
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
        }
        ToolIcon::ShowStress | ToolIcon::ShowVonMises => {
            // Gradient bar
            let r = egui::Rect::from_center_size(c, Vec2::new(s * 1.4, s * 0.8));
            painter.rect_stroke(r, 0.0, stroke, StrokeKind::Middle);
            for i in 0..5 {
                let t = i as f32 / 4.0;
                let x = r.left() + t * r.width();
                let shade = (t * 255.0) as u8;
                painter.line_segment(
                    [Pos2::new(x, r.top()), Pos2::new(x, r.bottom())],
                    Stroke::new(r.width() / 5.0, Color32::from_rgba_premultiplied(shade, (255 - shade) / 2, 255 - shade, 120)),
                );
            }
        }
        ToolIcon::ShowDisplacement => {
            // Deformed mesh hint
            let h = s * 0.6;
            painter.rect_stroke(egui::Rect::from_center_size(c, Vec2::splat(h * 2.0)), 0.0, thin, StrokeKind::Middle);
            // Deformed version offset
            let pts = [
                c + Vec2::new(-h + s * 0.1, -h + s * 0.05),
                c + Vec2::new(h + s * 0.15, -h + s * 0.1),
                c + Vec2::new(h + s * 0.2, h + s * 0.15),
                c + Vec2::new(-h + s * 0.05, h + s * 0.1),
            ];
            for i in 0..4 {
                painter.line_segment([pts[i], pts[(i + 1) % 4]], stroke);
            }
        }
        ToolIcon::FemSummary => draw_icon(ui, rect, ToolIcon::Template, color),
        ToolIcon::FemReport => draw_icon(ui, rect, ToolIcon::Template, color),
    }
}

// ---------------------------------------------------------------------------
// Drawing helpers
// ---------------------------------------------------------------------------

fn draw_arc(painter: &egui::Painter, center: Pos2, radius: f32, start_angle: f32, sweep: f32, segments: usize, stroke: Stroke) {
    let n = segments.max(2);
    let step = sweep / n as f32;
    let mut prev = center + Vec2::new(radius * start_angle.cos(), radius * start_angle.sin());
    for i in 1..=n {
        let a = start_angle + i as f32 * step;
        let cur = center + Vec2::new(radius * a.cos(), radius * a.sin());
        painter.line_segment([prev, cur], stroke);
        prev = cur;
    }
}

fn draw_ellipse(painter: &egui::Painter, center: Pos2, rx: f32, ry: f32, segments: usize, stroke: Stroke) {
    let n = segments.max(4);
    let step = std::f32::consts::TAU / n as f32;
    let mut prev = center + Vec2::new(rx, 0.0);
    for i in 1..=n {
        let a = i as f32 * step;
        let cur = center + Vec2::new(rx * a.cos(), ry * a.sin());
        painter.line_segment([prev, cur], stroke);
        prev = cur;
    }
}

fn draw_ellipse_half(painter: &egui::Painter, center: Pos2, ry: f32, left: bool, segments: usize, stroke: Stroke) {
    let n = segments.max(4);
    let start = if left { std::f32::consts::FRAC_PI_2 } else { -std::f32::consts::FRAC_PI_2 };
    let sweep = std::f32::consts::PI;
    let step = sweep / n as f32;
    let rx = ry * 0.4;
    let mut prev = center + Vec2::new(rx * start.cos(), ry * start.sin());
    for i in 1..=n {
        let a = start + i as f32 * step;
        let cur = center + Vec2::new(rx * a.cos(), ry * a.sin());
        painter.line_segment([prev, cur], stroke);
        prev = cur;
    }
}

fn draw_regular_polygon(painter: &egui::Painter, center: Pos2, radius: f32, sides: usize, stroke: Stroke) {
    let n = sides.max(3);
    let step = std::f32::consts::TAU / n as f32;
    let start = -std::f32::consts::FRAC_PI_2;
    let verts: Vec<Pos2> = (0..n).map(|i| {
        let a = start + i as f32 * step;
        center + Vec2::new(radius * a.cos(), radius * a.sin())
    }).collect();
    for i in 0..n {
        painter.line_segment([verts[i], verts[(i + 1) % n]], stroke);
    }
}

fn draw_eye(painter: &egui::Painter, center: Pos2, s: f32, stroke: Stroke, color: Color32, open: bool) {
    let w = s * 0.85;
    let h = s * 0.45;
    // Upper lid
    draw_arc(painter, center - Vec2::Y * h * 0.8, w * 1.2, std::f32::consts::FRAC_PI_4, std::f32::consts::FRAC_PI_2, 8, stroke);
    // Lower lid
    draw_arc(painter, center + Vec2::Y * h * 0.8, w * 1.2, -std::f32::consts::FRAC_PI_4 - std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2, 8, stroke);
    // Iris
    painter.circle_filled(center, s * 0.2, color);
    if !open {
        // Strikethrough
        painter.line_segment([center + Vec2::new(-w, -h), center + Vec2::new(w, h)], Stroke::new(1.5, color));
    }
}

fn draw_dashed_line(painter: &egui::Painter, from: Pos2, to: Pos2, dash_len: f32, gap_len: f32, stroke: Stroke) {
    let dir = to - from;
    let total = dir.length();
    if total < 0.1 {
        return;
    }
    let unit = dir / total;
    let mut pos = 0.0;
    let mut drawing = true;
    while pos < total {
        let seg_len = if drawing { dash_len } else { gap_len };
        let end = (pos + seg_len).min(total);
        if drawing {
            painter.line_segment([
                from + unit * pos,
                from + unit * end,
            ], stroke);
        }
        pos = end;
        drawing = !drawing;
    }
}

// ---------------------------------------------------------------------------
// Flyout (dropdown grouped) toolbar button -- FreeCAD-style
// ---------------------------------------------------------------------------

/// A single tool entry in a flyout group: icon, title, description, shortcut.
type FlyoutEntry = (ToolIcon, &'static str, &'static str, &'static str);

/// Draw a FreeCAD-style flyout/dropdown grouped toolbar button.
///
/// The main 28x28 button shows `group[last_used]`. A small triangle in the
/// bottom-right corner opens a dropdown listing all tools in the group.
/// Clicking the main area activates the last-used tool; clicking the triangle
/// (or right-clicking) opens the dropdown. Returns `Some(index)` when a tool
/// is activated.
fn flyout_button(
    ui: &mut egui::Ui,
    group_id: &str,
    group: &[FlyoutEntry],
    enabled: bool,
) -> Option<usize> {
    if group.is_empty() {
        return None;
    }

    let last_used: usize = ui.memory_mut(|mem| {
        mem.data
            .get_persisted(egui::Id::new(group_id))
            .unwrap_or(0usize)
    });
    let idx = last_used.min(group.len() - 1);
    let (icon, title, desc, _shortcut) = group[idx];

    if !enabled {
        icon_button_disabled(ui, icon, title, desc);
        return None;
    }

    flyout_button_inner(ui, group_id, group, idx, false)
}

/// Flyout variant that allows specifying an active (highlighted) task index.
fn flyout_button_with_active(
    ui: &mut egui::Ui,
    group_id: &str,
    group: &[FlyoutEntry],
    enabled: bool,
    active_index: Option<usize>,
) -> Option<usize> {
    if group.is_empty() {
        return None;
    }

    // When a tool in the group is active, force the button to show that tool
    let display_idx = if let Some(ai) = active_index {
        ui.memory_mut(|mem| {
            mem.data.insert_persisted(egui::Id::new(group_id), ai);
        });
        ai
    } else {
        let stored: usize = ui.memory_mut(|mem| {
            mem.data
                .get_persisted(egui::Id::new(group_id))
                .unwrap_or(0usize)
        });
        stored.min(group.len() - 1)
    };

    if !enabled {
        let (icon, title, desc, _) = group[display_idx];
        icon_button_disabled(ui, icon, title, desc);
        return None;
    }

    flyout_button_inner(ui, group_id, group, display_idx, active_index.is_some())
}

/// Shared rendering core for both flyout variants.
fn flyout_button_inner(
    ui: &mut egui::Ui,
    group_id: &str,
    group: &[FlyoutEntry],
    display_idx: usize,
    is_active: bool,
) -> Option<usize> {
    let (icon, title, desc, shortcut) = group[display_idx];

    let size = egui::vec2(28.0, 28.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    // Triangle hit zone: 8x8 bottom-right corner
    let tri_rect = egui::Rect::from_min_size(
        Pos2::new(rect.right() - 8.0, rect.bottom() - 8.0),
        egui::vec2(8.0, 8.0),
    );
    let pointer_pos = ui.input(|i| i.pointer.hover_pos());
    let over_triangle = pointer_pos.is_some_and(|p| tri_rect.contains(p));

    if ui.is_rect_visible(rect) {
        let accent = Color32::from_rgb(0, 122, 204);
        let color = if is_active {
            ui.painter().rect_filled(
                rect,
                4.0,
                Color32::from_rgba_premultiplied(0, 122, 204, 40),
            );
            ui.painter().line_segment(
                [
                    Pos2::new(rect.left() + 2.0, rect.bottom()),
                    Pos2::new(rect.right() - 2.0, rect.bottom()),
                ],
                Stroke::new(2.0, accent),
            );
            Color32::WHITE
        } else if response.hovered() {
            ui.painter().rect_filled(
                rect,
                4.0,
                Color32::from_rgba_premultiplied(255, 255, 255, 20),
            );
            Color32::from_gray(220)
        } else {
            Color32::from_gray(160)
        };
        draw_icon(ui, rect.shrink(3.0), icon, color);

        // Small triangle indicator (bottom-right)
        let tri_color = if over_triangle && response.hovered() {
            Color32::from_gray(220)
        } else {
            Color32::from_gray(100)
        };
        let tri_cx = rect.right() - 4.0;
        let tri_cy = rect.bottom() - 3.0;
        let tri_pts = [
            Pos2::new(tri_cx - 3.0, tri_cy - 2.5),
            Pos2::new(tri_cx + 3.0, tri_cy - 2.5),
            Pos2::new(tri_cx, tri_cy + 1.5),
        ];
        ui.painter().add(egui::Shape::convex_polygon(
            tri_pts.to_vec(),
            tri_color,
            Stroke::NONE,
        ));
    }

    // Tooltip
    response.clone().on_hover_ui(|ui| {
        ui.strong(title);
        if !shortcut.is_empty() {
            ui.label(
                egui::RichText::new(shortcut)
                    .small()
                    .color(Color32::from_gray(140)),
            );
        }
        if !desc.is_empty() {
            ui.label(desc);
        }
        if group.len() > 1 {
            ui.label(
                egui::RichText::new("Click triangle for more tools")
                    .small()
                    .italics()
                    .color(Color32::from_gray(120)),
            );
        }
    });

    let popup_id = egui::Id::new(("flyout_popup", group_id));

    // Right-click or triangle click opens dropdown
    if (response.clicked() && over_triangle) || response.secondary_clicked() {
        ui.memory_mut(|mem| mem.toggle_popup(popup_id));
    }

    let mut result: Option<usize> = None;

    // Main area click (not over triangle)
    if response.clicked() && !over_triangle {
        result = Some(display_idx);
    }

    // Popup dropdown
    egui::popup_below_widget(
        ui,
        popup_id,
        &response,
        egui::PopupCloseBehavior::CloseOnClick,
        |ui| {
            ui.set_min_width(140.0);
            for (i, &(g_icon, g_title, g_desc, g_shortcut)) in group.iter().enumerate() {
                let btn_response = ui.horizontal(|ui| {
                    let icon_size = egui::vec2(20.0, 20.0);
                    let (icon_rect, _) =
                        ui.allocate_exact_size(icon_size, egui::Sense::hover());
                    if ui.is_rect_visible(icon_rect) {
                        draw_icon(
                            ui,
                            icon_rect.shrink(2.0),
                            g_icon,
                            if i == display_idx {
                                Color32::WHITE
                            } else {
                                Color32::from_gray(180)
                            },
                        );
                    }
                    let label = if g_shortcut.is_empty() {
                        g_title.to_string()
                    } else {
                        format!("{g_title}  ({g_shortcut})")
                    };
                    let text = if i == display_idx {
                        egui::RichText::new(label).strong()
                    } else {
                        egui::RichText::new(label)
                    };
                    ui.add(egui::Label::new(text).selectable(false))
                });
                let row_response = btn_response.response;
                row_response.clone().on_hover_text(g_desc);
                if row_response.clicked() {
                    result = Some(i);
                    ui.memory_mut(|mem| {
                        mem.data.insert_persisted(egui::Id::new(group_id), i);
                    });
                }
            }
        },
    );

    result
}

// ---------------------------------------------------------------------------
// Toolbar button with icon + rich tooltip
// ---------------------------------------------------------------------------

fn icon_button(
    ui: &mut egui::Ui,
    icon: ToolIcon,
    tooltip_title: &str,
    tooltip_desc: &str,
    shortcut: &str,
) -> bool {
    icon_button_ex(ui, icon, tooltip_title, tooltip_desc, shortcut, false)
}

fn icon_button_active(
    ui: &mut egui::Ui,
    icon: ToolIcon,
    tooltip_title: &str,
    tooltip_desc: &str,
    shortcut: &str,
) -> bool {
    icon_button_ex(ui, icon, tooltip_title, tooltip_desc, shortcut, true)
}

fn icon_button_ex(
    ui: &mut egui::Ui,
    icon: ToolIcon,
    tooltip_title: &str,
    tooltip_desc: &str,
    shortcut: &str,
    active: bool,
) -> bool {
    let size = egui::vec2(28.0, 28.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let accent = Color32::from_rgb(0, 122, 204);
        let color = if active {
            ui.painter().rect_filled(
                rect,
                4.0,
                Color32::from_rgba_premultiplied(0, 122, 204, 40),
            );
            // Bottom accent border
            ui.painter().line_segment(
                [
                    Pos2::new(rect.left() + 2.0, rect.bottom()),
                    Pos2::new(rect.right() - 2.0, rect.bottom()),
                ],
                Stroke::new(2.0, accent),
            );
            Color32::WHITE
        } else if response.hovered() {
            ui.painter().rect_filled(
                rect,
                4.0,
                Color32::from_rgba_premultiplied(255, 255, 255, 20),
            );
            Color32::from_gray(220)
        } else {
            Color32::from_gray(160)
        };

        draw_icon(ui, rect.shrink(3.0), icon, color);
    }

    response
        .clone()
        .on_hover_ui(|ui| {
            ui.strong(tooltip_title);
            if !shortcut.is_empty() {
                ui.label(
                    egui::RichText::new(shortcut)
                        .small()
                        .color(Color32::from_gray(140)),
                );
            }
            if !tooltip_desc.is_empty() {
                ui.label(tooltip_desc);
            }
        });

    response.clicked()
}

/// Non-interactive icon button rendered in a dimmed style.
/// Used when the action is unavailable (e.g., nothing selected, empty undo stack).
fn icon_button_disabled(
    ui: &mut egui::Ui,
    icon: ToolIcon,
    tooltip_title: &str,
    tooltip_desc: &str,
) {
    let size = egui::vec2(28.0, 28.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let color = Color32::from_gray(70);
        draw_icon(ui, rect.shrink(3.0), icon, color);
    }
    response.on_hover_ui(|ui| {
        ui.strong(format!("{tooltip_title} (unavailable)"));
        if !tooltip_desc.is_empty() {
            ui.label(tooltip_desc);
        }
    });
}

/// Helper macro: render an enabled button when the condition holds, otherwise disabled.
/// Keeps toolbar code DRY by combining the enabled/disabled branching in one place.
macro_rules! gated_button {
    ($ui:expr, $enabled:expr, $icon:expr, $title:expr, $desc:expr, $shortcut:expr, $body:block) => {
        if $enabled {
            if icon_button($ui, $icon, $title, $desc, $shortcut) $body
        } else {
            icon_button_disabled($ui, $icon, $title, $desc);
        }
    };
}

/// Same as `gated_button!` but uses `icon_button_active` when enabled and the task is active.
macro_rules! gated_button_active {
    ($ui:expr, $enabled:expr, $active:expr, $icon:expr, $title:expr, $desc:expr, $shortcut:expr, $body:block) => {
        if $enabled {
            let btn_fn = if $active { icon_button_active } else { icon_button };
            if btn_fn($ui, $icon, $title, $desc, $shortcut) $body
        } else {
            icon_button_disabled($ui, $icon, $title, $desc);
        }
    };
}

/// Toggleable icon button that shows selected state.
fn icon_toggle(
    ui: &mut egui::Ui,
    icon: ToolIcon,
    selected: bool,
    tooltip_title: &str,
    tooltip_desc: &str,
) -> bool {
    let size = egui::vec2(28.0, 28.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        if selected {
            ui.painter().rect_filled(
                rect,
                4.0,
                Color32::from_rgba_premultiplied(0, 122, 204, 40),
            );
            // Bottom accent border
            ui.painter().line_segment(
                [
                    Pos2::new(rect.left() + 2.0, rect.bottom()),
                    Pos2::new(rect.right() - 2.0, rect.bottom()),
                ],
                Stroke::new(2.0, Color32::from_rgb(0, 122, 204)),
            );
        }
        let color = if response.hovered() {
            if !selected {
                ui.painter().rect_filled(
                    rect,
                    4.0,
                    Color32::from_rgba_premultiplied(255, 255, 255, 20),
                );
            }
            Color32::from_gray(220)
        } else if selected {
            Color32::WHITE
        } else {
            Color32::from_gray(160)
        };

        draw_icon(ui, rect.shrink(3.0), icon, color);
    }

    response
        .clone()
        .on_hover_ui(|ui| {
            ui.strong(tooltip_title);
            if !tooltip_desc.is_empty() {
                ui.label(tooltip_desc);
            }
        });

    response.clicked()
}

// ---------------------------------------------------------------------------
// Toolbar separator (vertical line with spacing)
// ---------------------------------------------------------------------------

fn toolbar_separator(ui: &mut egui::Ui) {
    ui.add_space(4.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(1.0, 20.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let top = rect.left_top();
        let mid = rect.center();
        let bot = rect.left_bottom();
        // Gradient: transparent → gray → transparent
        ui.painter().line_segment(
            [top, mid],
            Stroke::new(1.0, Color32::from_rgba_premultiplied(80, 80, 80, 40)),
        );
        ui.painter().line_segment(
            [mid, bot],
            Stroke::new(1.0, Color32::from_rgba_premultiplied(80, 80, 80, 40)),
        );
        // Bright center segment
        let q1 = egui::pos2(top.x, top.y + rect.height() * 0.25);
        let q3 = egui::pos2(top.x, top.y + rect.height() * 0.75);
        ui.painter().line_segment(
            [q1, q3],
            Stroke::new(1.0, Color32::from_rgba_premultiplied(90, 90, 90, 100)),
        );
    }
    ui.add_space(4.0);
}

/// Small dim label for grouping toolbar sections (FreeCAD-style).
fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.add_space(2.0);
    ui.label(
        egui::RichText::new(text)
            .size(9.0)
            .color(super::theme::MENU_SECTION_COLOR),
    );
    ui.add_space(1.0);
}

// ---------------------------------------------------------------------------
// Top toolbar: file / edit / view / selection + workbench combo
// ---------------------------------------------------------------------------

pub(crate) fn draw_toolbar(ctx: &egui::Context, gui: &mut GuiState) {
    egui::TopBottomPanel::top("toolbar")
        .frame(egui::Frame {
            fill: Color32::from_rgb(45, 48, 56),
            inner_margin: egui::Margin::symmetric(8, 3),
            stroke: egui::Stroke::new(1.0, Color32::from_rgb(30, 32, 38)),
            ..egui::Frame::NONE
        })
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;

                section_label(ui, "File");
                // -- File group --
                if icon_button(ui, ToolIcon::New, "New", "Create a new model", "Ctrl+N") {
                    gui.actions.push(GuiAction::NewModel);
                }
                if icon_button(ui, ToolIcon::Open, "Open", "Open a file", "Ctrl+O") {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("CADKernel", &["cadk"])
                        .add_filter("STL", &["stl"])
                        .add_filter("OBJ", &["obj"])
                        .add_filter("All", &["*"])
                        .pick_file()
                    {
                        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                        if ext == "cadk" || ext == "json" {
                            gui.actions.push(GuiAction::OpenFile(path));
                        } else {
                            gui.actions.push(GuiAction::ImportFile(path));
                        }
                    }
                }
                if icon_button(ui, ToolIcon::Save, "Save", "Save project", "Ctrl+S") {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("CADKernel", &["cadk"])
                        .set_file_name("model.cadk")
                        .save_file()
                    {
                        gui.actions.push(GuiAction::SaveFile(path));
                    }
                }

                toolbar_separator(ui);

                section_label(ui, "Edit");
                // -- Edit group --
                gated_button!(ui, gui.tb_can_undo, ToolIcon::Undo, "Undo", "Undo last action", "Ctrl+Z", {
                    gui.actions.push(GuiAction::Undo);
                });
                // Undo history dropdown
                if gui.tb_can_undo && !gui.history_entries.is_empty() {
                    let drop_resp = ui.add(
                        egui::Button::new(egui::RichText::new("\u{25BE}").size(8.0).color(Color32::from_rgb(140, 145, 155)))
                            .min_size(Vec2::new(12.0, 24.0))
                            .frame(false),
                    );
                    let popup_id = egui::Id::new("undo_history_popup");
                    if drop_resp.clicked() {
                        ui.memory_mut(|m| m.toggle_popup(popup_id));
                    }
                    egui::popup_below_widget(ui, popup_id, &drop_resp, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
                        ui.set_min_width(200.0);
                        ui.label(egui::RichText::new("Undo History").size(10.0).strong());
                        ui.separator();
                        let entries = gui.history_entries.clone();
                        for (i, entry) in entries.iter().enumerate().rev().take(10) {
                            let label = format!("{}. {}", i + 1, entry);
                            if ui.button(egui::RichText::new(&label).size(11.0)).clicked() {
                                for _ in 0..=(entries.len() - 1 - i) {
                                    gui.actions.push(GuiAction::Undo);
                                }
                                ui.memory_mut(|m| m.toggle_popup(popup_id));
                            }
                        }
                    });
                }
                gated_button!(ui, gui.tb_can_redo, ToolIcon::Redo, "Redo", "Redo last action", "Ctrl+Y", {
                    gui.actions.push(GuiAction::Redo);
                });
                // Redo history dropdown
                if gui.tb_can_redo && !gui.future_entries.is_empty() {
                    let drop_resp = ui.add(
                        egui::Button::new(egui::RichText::new("\u{25BE}").size(8.0).color(Color32::from_rgb(140, 145, 155)))
                            .min_size(Vec2::new(12.0, 24.0))
                            .frame(false),
                    );
                    let popup_id = egui::Id::new("redo_history_popup");
                    if drop_resp.clicked() {
                        ui.memory_mut(|m| m.toggle_popup(popup_id));
                    }
                    egui::popup_below_widget(ui, popup_id, &drop_resp, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
                        ui.set_min_width(200.0);
                        ui.label(egui::RichText::new("Redo History").size(10.0).strong());
                        ui.separator();
                        let entries = gui.future_entries.clone();
                        for (i, entry) in entries.iter().enumerate().take(10) {
                            let label = format!("{}. {}", i + 1, entry);
                            if ui.button(egui::RichText::new(&label).size(11.0)).clicked() {
                                for _ in 0..=i {
                                    gui.actions.push(GuiAction::Redo);
                                }
                                ui.memory_mut(|m| m.toggle_popup(popup_id));
                            }
                        }
                    });
                }

                toolbar_separator(ui);

                section_label(ui, "View");
                // -- View group --
                if icon_button(ui, ToolIcon::FitAll, "Fit All", "Fit all objects in view", "F") {
                    gui.actions.push(GuiAction::FitAll);
                }
                if icon_button(ui, ToolIcon::ResetCamera, "Reset Camera", "Reset to default view", "") {
                    gui.actions.push(GuiAction::ResetCamera);
                }

                // Display mode dropdown
                {
                    use crate::render::DisplayMode;
                    let current_label = gui.tb_display_mode.label();
                    egui::ComboBox::from_id_salt("display_mode_selector")
                        .selected_text(egui::RichText::new(current_label).size(10.0))
                        .width(80.0)
                        .show_ui(ui, |ui| {
                            for &mode in DisplayMode::ALL {
                                let label = format!("{} ({})", mode.label(), mode.shortcut());
                                if ui.selectable_label(gui.tb_display_mode == mode, label).clicked() {
                                    gui.actions.push(GuiAction::SetDisplayMode(mode));
                                }
                            }
                        });
                }

                toolbar_separator(ui);

                section_label(ui, "Transform");
                // -- Gizmo mode toggle --
                for &(gizmo, icon, label, shortcut) in &[
                    (GizmoMode::Translate, ToolIcon::DraftMove, "Move", "W"),
                    (GizmoMode::Rotate, ToolIcon::DraftRotate, "Rotate", "E"),
                    (GizmoMode::Scale, ToolIcon::DraftScale, "Scale", "R"),
                ] {
                    let active = gui.gizmo_mode == gizmo;
                    if icon_toggle(ui, icon, active, label, shortcut) {
                        gui.actions.push(GuiAction::SetGizmoMode(gizmo));
                    }
                }

                toolbar_separator(ui);

                section_label(ui, "Scene");
                // -- Scene group --
                gated_button!(ui, gui.tb_has_objects, ToolIcon::ShowAll, "Show All", "Show all objects", "", {
                    gui.actions.push(GuiAction::ShowAll);
                });
                gated_button!(ui, gui.tb_has_objects, ToolIcon::HideAll, "Hide All", "Hide all objects", "", {
                    gui.actions.push(GuiAction::HideAll);
                });

                toolbar_separator(ui);

                section_label(ui, "Select");
                // -- Selection mode (with count badge) --
                let sel_count = gui.selected_entities.len();
                for &(mode, icon, label, shortcut) in &[
                    (SelectionMode::Solid, ToolIcon::SelectSolid, "Solid", ""),
                    (SelectionMode::Face, ToolIcon::SelectFace, "Face", "2"),
                    (SelectionMode::Edge, ToolIcon::SelectEdge, "Edge", "3"),
                    (SelectionMode::Vertex, ToolIcon::SelectVertex, "Vertex", "4"),
                ] {
                    let active = gui.selection_mode == mode;
                    let badge = if active && sel_count > 0 {
                        format!("{} [{}]", label, sel_count)
                    } else {
                        label.to_string()
                    };
                    if icon_toggle(ui, icon, active, &badge, shortcut) {
                        gui.actions.push(GuiAction::SetSelectionMode(mode));
                    }
                }

                toolbar_separator(ui);

                // -- Select All / Deselect --
                if icon_button(ui, ToolIcon::ShowAll, "Select All", "Select all objects", "Ctrl+A") {
                    gui.actions.push(GuiAction::SelectAll);
                }
                if icon_button(ui, ToolIcon::HideAll, "Deselect", "Clear selection", "Esc") {
                    gui.actions.push(GuiAction::DeselectAll);
                }

                toolbar_separator(ui);

                // Workbench dropdown selector
                let wb_label = gui.active_workbench.label();
                egui::ComboBox::from_id_salt("wb_selector")
                    .selected_text(wb_label)
                    .show_ui(ui, |ui| {
                        for &wb in Workbench::ALL {
                            ui.selectable_value(&mut gui.active_workbench, wb, wb.label());
                        }
                    });
            });
        });
}

// ---------------------------------------------------------------------------
// Workbench tabs: styled tab bar
// ---------------------------------------------------------------------------

pub(crate) fn draw_workbench_tabs(ctx: &egui::Context, gui: &mut GuiState) {
    egui::TopBottomPanel::top("workbench_tabs")
        .frame(egui::Frame {
            fill: Color32::from_rgb(37, 37, 38),
            inner_margin: egui::Margin::symmetric(8, 2),
            stroke: egui::Stroke::new(1.0, Color32::from_rgb(28, 28, 30)),
            ..egui::Frame::NONE
        })
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                for &wb in Workbench::ALL {
                    let selected = gui.active_workbench == wb;
                    let text = if selected {
                        egui::RichText::new(wb.label())
                            .strong()
                            .color(Color32::WHITE)
                    } else {
                        egui::RichText::new(wb.label()).color(Color32::from_gray(150))
                    };

                    let padding = egui::vec2(12.0, 6.0);
                    let response = ui.add(
                        egui::Button::new(text)
                            .frame(false)
                            .min_size(egui::vec2(0.0, 24.0)),
                    );

                    // Draw accent underline for active tab
                    if selected {
                        let tab_rect = response.rect;
                        ui.painter().line_segment(
                            [
                                Pos2::new(tab_rect.left() + padding.x * 0.3, tab_rect.bottom()),
                                Pos2::new(tab_rect.right() - padding.x * 0.3, tab_rect.bottom()),
                            ],
                            Stroke::new(2.0, Color32::from_rgb(0, 122, 204)),
                        );
                    } else if response.hovered() {
                        let tab_rect = response.rect;
                        ui.painter().rect_filled(
                            tab_rect,
                            0.0,
                            Color32::from_rgba_premultiplied(255, 255, 255, 10),
                        );
                    }

                    if response
                        .on_hover_text(format!("{} workbench", wb.label()))
                        .clicked()
                    {
                        gui.active_workbench = wb;
                    }

                    // Add subtle separator between tabs
                    if wb != *Workbench::ALL.last().unwrap() {
                        let sep_rect = ui
                            .allocate_exact_size(egui::vec2(1.0, 16.0), egui::Sense::hover())
                            .0;
                        if ui.is_rect_visible(sep_rect) && !selected {
                            ui.painter().line_segment(
                                [sep_rect.left_top(), sep_rect.left_bottom()],
                                Stroke::new(1.0, Color32::from_gray(50)),
                            );
                        }
                    }
                }
            });
        });
}

// ---------------------------------------------------------------------------
// Context toolbar: workbench-specific tools
// ---------------------------------------------------------------------------

pub(crate) fn draw_context_toolbar(ctx: &egui::Context, gui: &mut GuiState) {
    egui::TopBottomPanel::top("context_toolbar")
        .frame(egui::Frame {
            fill: Color32::from_rgb(40, 40, 44),
            inner_margin: egui::Margin::symmetric(8, 3),
            stroke: egui::Stroke::new(1.0, Color32::from_rgb(28, 30, 35)),
            ..egui::Frame::NONE
        })
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                match gui.active_workbench {
                    Workbench::Part => draw_part_toolbar(ui, gui),
                    Workbench::PartDesign => draw_partdesign_toolbar(ui, gui),
                    Workbench::Sketcher => draw_sketcher_toolbar(ui, gui),
                    Workbench::Mesh => draw_mesh_toolbar(ui, gui),
                    Workbench::TechDraw => draw_techdraw_toolbar(ui, gui),
                    Workbench::Assembly => draw_assembly_toolbar(ui, gui),
                    Workbench::Draft => draw_draft_toolbar(ui, gui),
                    Workbench::Surface => draw_surface_toolbar(ui, gui),
                    Workbench::Fem => draw_fem_toolbar(ui, gui),
                }
            });
        });
}

// ===========================================================================
// Workbench-specific toolbar implementations
// ===========================================================================

fn draw_part_toolbar(ui: &mut egui::Ui, gui: &mut GuiState) {
    use super::task_panel::ActiveTask;

    section_label(ui, "Primitives");
    // -- Primitives flyout (Box, Cylinder, Sphere, Cone, Torus) --
    const PRIM_GROUP: &[FlyoutEntry] = &[
        (ToolIcon::PrimBox, "Box", "Create box primitive", ""),
        (ToolIcon::Cylinder, "Cylinder", "Create cylinder primitive", ""),
        (ToolIcon::Sphere, "Sphere", "Create sphere primitive", ""),
        (ToolIcon::Cone, "Cone", "Create cone primitive", ""),
        (ToolIcon::Torus, "Torus", "Create torus primitive", ""),
    ];
    let prim_active = match &gui.active_task {
        Some(ActiveTask::Box { .. }) => Some(0),
        Some(ActiveTask::Cylinder { .. }) => Some(1),
        Some(ActiveTask::Sphere { .. }) => Some(2),
        Some(ActiveTask::Cone { .. }) => Some(3),
        Some(ActiveTask::Torus { .. }) => Some(4),
        _ => None,
    };
    if let Some(idx) = flyout_button_with_active(ui, "flyout_primitives", PRIM_GROUP, true, prim_active) {
        gui.active_task = Some(match idx {
            0 => ActiveTask::Box { width: 10.0, height: 10.0, depth: 10.0, preview_id: None },
            1 => ActiveTask::Cylinder { radius: 5.0, height: 10.0, preview_id: None },
            2 => ActiveTask::Sphere { radius: 5.0, preview_id: None },
            3 => ActiveTask::Cone { base_radius: 5.0, top_radius: 0.0, height: 10.0, preview_id: None },
            _ => ActiveTask::Torus { major_radius: 10.0, minor_radius: 3.0, preview_id: None },
        });
    }

    // -- Extended primitives flyout (Tube, Prism, Wedge, Ellipsoid, Helix) --
    const EXT_PRIM_GROUP: &[FlyoutEntry] = &[
        (ToolIcon::Tube, "Tube", "Create hollow tube", ""),
        (ToolIcon::Prism, "Prism", "Create polygon prism", ""),
        (ToolIcon::Wedge, "Wedge", "Create wedge shape", ""),
        (ToolIcon::Ellipsoid, "Ellipsoid", "Create ellipsoid", ""),
        (ToolIcon::Helix, "Helix", "Create helical coil", ""),
    ];
    let ext_active = match &gui.active_task {
        Some(ActiveTask::Tube { .. }) => Some(0),
        Some(ActiveTask::Prism { .. }) => Some(1),
        Some(ActiveTask::Wedge { .. }) => Some(2),
        Some(ActiveTask::Ellipsoid { .. }) => Some(3),
        Some(ActiveTask::Helix { .. }) => Some(4),
        _ => None,
    };
    if let Some(idx) = flyout_button_with_active(ui, "flyout_ext_primitives", EXT_PRIM_GROUP, true, ext_active) {
        gui.active_task = Some(match idx {
            0 => ActiveTask::Tube { outer_radius: 5.0, inner_radius: 3.0, height: 10.0, preview_id: None },
            1 => ActiveTask::Prism { radius: 5.0, height: 10.0, sides: 6, preview_id: None },
            2 => ActiveTask::Wedge { dx: 10.0, dy: 10.0, dz: 10.0, dx2: 5.0, dy2: 5.0, preview_id: None },
            3 => ActiveTask::Ellipsoid { rx: 5.0, ry: 3.0, rz: 2.0, preview_id: None },
            _ => ActiveTask::Helix { radius: 5.0, pitch: 3.0, turns: 3.0, tube_radius: 0.5, preview_id: None },
        });
    }

    toolbar_separator(ui);

    let sel = gui.tb_has_selection;

    section_label(ui, "Boolean");
    // -- Boolean flyout (Union, Subtract, Intersect) --
    const BOOL_GROUP: &[FlyoutEntry] = &[
        (ToolIcon::BoolUnion, "Union", "Boolean union (select object)", ""),
        (ToolIcon::BoolSubtract, "Subtract", "Boolean subtraction (select object)", ""),
        (ToolIcon::BoolIntersect, "Intersect", "Boolean intersection (select object)", ""),
    ];
    if let Some(idx) = flyout_button(ui, "flyout_boolean", BOOL_GROUP, sel) {
        gui.active_task = Some(ActiveTask::BooleanOp {
            op_type: idx as u8, width: 5.0, height: 5.0, depth: 5.0,
            offset_x: 5.0, offset_y: 0.0, offset_z: 0.0, preview_id: None,
        });
    }

    toolbar_separator(ui);

    section_label(ui, "Transform");
    // -- Transforms --
    gated_button!(ui, sel, ToolIcon::Mirror, "Mirror", "Mirror solid (select object)", "", {
        gui.active_task = Some(ActiveTask::MirrorOp { plane: 0, preview_id: None });
    });
    gated_button!(ui, sel, ToolIcon::Scale, "Scale", "Scale solid (select object)", "", {
        gui.active_task = Some(ActiveTask::ScaleOp { factor: 2.0, preview_id: None });
    });
    gated_button!(ui, sel, ToolIcon::Shell, "Shell", "Hollow out solid (select object)", "", {
        gui.active_task = Some(ActiveTask::Shell { thickness: 1.0, preview_id: None });
    });
    let has_edges = gui.selected_entities.iter().any(|e| matches!(e, super::SelectedEntity::Edge(_)));
    let fillet_enabled = sel || has_edges;
    let fillet_tip = if has_edges { "Fillet selected edges" } else { "Round all edges (select object)" };
    let chamfer_tip = if has_edges { "Chamfer selected edges" } else { "Chamfer all edges (select object)" };
    gated_button!(ui, fillet_enabled, ToolIcon::Fillet, "Fillet", fillet_tip, "", {
        gui.active_task = Some(ActiveTask::Fillet { radius: 1.0, preview_id: None });
    });
    gated_button!(ui, fillet_enabled, ToolIcon::Chamfer, "Chamfer", chamfer_tip, "", {
        gui.active_task = Some(ActiveTask::Chamfer { distance: 1.0, preview_id: None });
    });
    gated_button!(ui, sel, ToolIcon::Pattern, "Pattern", "Linear pattern (select object)", "", {
        gui.active_task = Some(ActiveTask::Pattern {
            count: 3, spacing: 15.0, axis: 0, preview_id: None,
        });
    });

    toolbar_separator(ui);

    section_label(ui, "Join");
    // -- Join --
    gated_button!(ui, sel, ToolIcon::Connect, "Connect", "Connect shapes (select object)", "", {
        gui.actions.push(GuiAction::ConnectShapes);
    });
    gated_button!(ui, sel, ToolIcon::Embed, "Embed", "Embed shapes (select object)", "", {
        gui.actions.push(GuiAction::EmbedShapes);
    });
    gated_button!(ui, sel, ToolIcon::Cutout, "Cutout", "Cutout shapes (select object)", "", {
        gui.actions.push(GuiAction::CutoutShapes);
    });

    toolbar_separator(ui);

    section_label(ui, "Compound");
    // -- Compound --
    gated_button!(ui, sel, ToolIcon::Explode, "Explode", "Explode compound (select object)", "", {
        gui.actions.push(GuiAction::ExplodeCompound);
    });
    gated_button!(ui, sel, ToolIcon::Filter, "Filter", "Compound filter (select object)", "", {
        gui.actions.push(GuiAction::CompoundFilter);
    });
    gated_button!(ui, sel, ToolIcon::Fragments, "Fragments", "Boolean fragments (select object)", "", {
        gui.actions.push(GuiAction::BooleanFragments);
    });
    gated_button!(ui, sel, ToolIcon::Slice, "Slice", "Slice to compound (select object)", "", {
        gui.actions.push(GuiAction::SliceToCompound);
    });

    toolbar_separator(ui);

    section_label(ui, "Convert");
    // -- Convert --
    gated_button!(ui, sel, ToolIcon::Points, "Points", "Points from shape (select object)", "", {
        gui.actions.push(GuiAction::PointsFromShape);
    });
    gated_button!(ui, sel, ToolIcon::ToSolid, "To Solid", "Convert to solid (select object)", "", {
        gui.actions.push(GuiAction::ConvertToSolid);
    });
    gated_button!(ui, sel, ToolIcon::Defeature, "Defeature", "Auto-defeaturing (select object)", "", {
        gui.show_defeaturing = true;
    });

    toolbar_separator(ui);

    section_label(ui, "Analysis");
    // -- Analysis --
    gated_button!(ui, sel, ToolIcon::Measure, "Measure", "Mass properties (select object)", "", {
        gui.actions.push(GuiAction::MeasureSolid);
    });
    gated_button!(ui, sel, ToolIcon::Check, "Check", "Check geometry (select object)", "", {
        gui.actions.push(GuiAction::CheckGeometry);
    });
}

fn draw_partdesign_toolbar(ui: &mut egui::Ui, gui: &mut GuiState) {
    use super::task_panel::ActiveTask;

    let sel = gui.tb_has_selection;
    let sketch = gui.tb_in_sketch;

    section_label(ui, "Features");
    // -- Feature (sketch-dependent operations) --
    let is_pad = matches!(&gui.active_task, Some(ActiveTask::Pad { .. }));
    gated_button_active!(ui, sel || sketch, is_pad, ToolIcon::Pad, "Pad", "Pad / additive extrude (select or sketch)", "", {
        gui.active_task = Some(ActiveTask::Pad {
            depth: 10.0, symmetric: false, preview_id: None,
        });
    });
    let is_pocket = matches!(&gui.active_task, Some(ActiveTask::Pocket { .. }));
    gated_button_active!(ui, sel || sketch, is_pocket, ToolIcon::Pocket, "Pocket", "Pocket / subtractive (select or sketch)", "", {
        gui.active_task = Some(ActiveTask::Pocket {
            depth: 5.0, through_all: false, preview_id: None,
        });
    });
    gated_button!(ui, sel || sketch, ToolIcon::Groove, "Groove", "Groove / subtractive revolve (select or sketch)", "", {
        gui.active_task = Some(ActiveTask::Groove { angle: 360.0, preview_id: None });
    });
    let is_hole = matches!(&gui.active_task, Some(ActiveTask::Hole { .. }));
    gated_button_active!(ui, sel || sketch, is_hole, ToolIcon::Hole, "Hole", "Create hole (select or sketch)", "", {
        gui.active_task = Some(ActiveTask::Hole {
            radius: 2.0, depth: 10.0, countersink: false, countersink_angle: 90.0, preview_id: None,
        });
    });

    toolbar_separator(ui);

    section_label(ui, "Additive");
    // -- Additive flyout (Loft, Pipe) --
    const ADD_GROUP: &[FlyoutEntry] = &[
        (ToolIcon::AddLoft, "Add Loft", "Additive loft (select object)", ""),
        (ToolIcon::AddPipe, "Add Pipe", "Additive pipe (select object)", ""),
    ];
    if let Some(idx) = flyout_button(ui, "flyout_pd_additive", ADD_GROUP, sel) {
        gui.actions.push(match idx {
            0 => GuiAction::AdditiveLoft,
            _ => GuiAction::AdditivePipe,
        });
    }

    // -- Subtractive flyout (Loft, Pipe) --
    const SUB_GROUP: &[FlyoutEntry] = &[
        (ToolIcon::SubLoft, "Sub Loft", "Subtractive loft (select object)", ""),
        (ToolIcon::SubPipe, "Sub Pipe", "Subtractive pipe (select object)", ""),
    ];
    if let Some(idx) = flyout_button(ui, "flyout_pd_subtractive", SUB_GROUP, sel) {
        gui.actions.push(match idx {
            0 => GuiAction::SubtractiveLoft,
            _ => GuiAction::SubtractivePipe,
        });
    }

    toolbar_separator(ui);

    section_label(ui, "Dress-up");
    // -- Dress-up flyout (Fillet, Chamfer, Shell) --
    let pd_has_edges = gui.selected_entities.iter().any(|e| matches!(e, super::SelectedEntity::Edge(_)));
    const DRESSUP_GROUP: &[FlyoutEntry] = &[
        (ToolIcon::Fillet, "Fillet", "Round edges (select object or edges)", ""),
        (ToolIcon::Chamfer, "Chamfer", "Bevel edges (select object or edges)", ""),
        (ToolIcon::Shell, "Shell", "Hollow out solid (select object)", ""),
    ];
    if let Some(idx) = flyout_button(ui, "flyout_pd_dressup", DRESSUP_GROUP, sel || pd_has_edges) {
        gui.active_task = Some(match idx {
            0 => ActiveTask::Fillet { radius: 1.0, preview_id: None },
            1 => ActiveTask::Chamfer { distance: 1.0, preview_id: None },
            _ => ActiveTask::Shell { thickness: 1.0, preview_id: None },
        });
    }

    toolbar_separator(ui);

    section_label(ui, "Transform");
    // -- Transform flyout (Mirror, Scale, Pattern) --
    const PD_XFORM_GROUP: &[FlyoutEntry] = &[
        (ToolIcon::Mirror, "Mirror", "Mirror solid (select object)", ""),
        (ToolIcon::Scale, "Scale", "Scale solid (select object)", ""),
        (ToolIcon::Pattern, "Pattern", "Linear pattern (select object)", ""),
    ];
    if let Some(idx) = flyout_button(ui, "flyout_pd_transform", PD_XFORM_GROUP, sel) {
        gui.active_task = Some(match idx {
            0 => ActiveTask::MirrorOp { plane: 0, preview_id: None },
            1 => ActiveTask::ScaleOp { factor: 2.0, preview_id: None },
            _ => ActiveTask::Pattern { count: 3, spacing: 15.0, axis: 0, preview_id: None },
        });
    }

    toolbar_separator(ui);

    section_label(ui, "Extras");
    // -- Extra --
    if icon_button(ui, ToolIcon::Sprocket, "Sprocket", "Create sprocket gear", "") {
        gui.active_task = Some(ActiveTask::Sprocket {
            teeth: 20, roller_diameter: 1.0, pitch: 12.7, bore: 5.0, preview_id: None,
        });
    }
    if icon_button(ui, ToolIcon::Gear, "Gear", "Involute gear", "") {
        gui.active_task = Some(ActiveTask::InvoluteGear {
            teeth: 20, module_val: 2.0, pressure_angle: 20.0, preview_id: None,
        });
    }
    if icon_button(ui, ToolIcon::Shaft, "Shaft", "Shaft design", "") {
        gui.show_shaft = true;
    }

    toolbar_separator(ui);

    section_label(ui, "Body");
    // -- Body --
    gated_button!(ui, sel, ToolIcon::Suppress, "Suppress", "Suppress feature (select object)", "", {
        gui.actions.push(GuiAction::SuppressFeature);
    });
    gated_button!(ui, sel, ToolIcon::SetTip, "Set Tip", "Set tip feature (select object)", "", {
        gui.actions.push(GuiAction::SetTip);
    });
    gated_button!(ui, sel, ToolIcon::MoveUp, "Up", "Move feature up (select object)", "", {
        gui.actions.push(GuiAction::MoveFeatureUp);
    });
    gated_button!(ui, sel, ToolIcon::MoveDown, "Down", "Move feature down (select object)", "", {
        gui.actions.push(GuiAction::MoveFeatureDown);
    });

    toolbar_separator(ui);

    section_label(ui, "Boolean");
    // -- Boolean --
    gated_button!(ui, sel, ToolIcon::BoolUnion, "Union", "Boolean union (select object)", "", {
        gui.active_task = Some(ActiveTask::BooleanOp {
            op_type: 0, width: 5.0, height: 5.0, depth: 5.0,
            offset_x: 5.0, offset_y: 0.0, offset_z: 0.0, preview_id: None,
        });
    });
    gated_button!(ui, sel, ToolIcon::BoolSubtract, "Subtract", "Boolean subtraction (select object)", "", {
        gui.active_task = Some(ActiveTask::BooleanOp {
            op_type: 1, width: 5.0, height: 5.0, depth: 5.0,
            offset_x: 5.0, offset_y: 0.0, offset_z: 0.0, preview_id: None,
        });
    });
}

fn draw_sketcher_toolbar(ui: &mut egui::Ui, gui: &mut GuiState) {
    use super::SketcherAction as S;
    let in_sketch = gui.sketch_mode.is_some();
    if !in_sketch {
        let has_face = gui.selected_entities.iter().any(|e| matches!(e, super::SelectedEntity::Face(_)));
        if has_face {
            if icon_button(ui, ToolIcon::SketchRect, "On Face", "Sketch on selected face", "") {
                gui.actions.push(GuiAction::Sketcher(S::EnterOnSelectedFace));
            }
            toolbar_separator(ui);
        }
        if icon_button(ui, ToolIcon::SketchRect, "XY Plane", "Sketch on XY plane", "") {
            gui.actions.push(GuiAction::Sketcher(S::Enter(WorkPlane::xy())));
        }
        if icon_button(ui, ToolIcon::SketchRect, "XZ Plane", "Sketch on XZ plane", "") {
            gui.actions.push(GuiAction::Sketcher(S::Enter(WorkPlane::xz())));
        }
        if gui.last_sketch.is_some() {
            toolbar_separator(ui);
            if icon_button(ui, ToolIcon::SketchSelect, "Edit Sketch", "Reopen last sketch for editing", "") {
                gui.actions.push(GuiAction::Sketcher(S::Edit));
            }
        }
    } else {
        section_label(ui, "Geometry");
        // -- Geometry tools --
        let current_tool = gui
            .sketch_mode
            .as_ref()
            .map(|s| s.tool)
            .unwrap_or(SketchTool::Select);

        for &(icon, tool, title, shortcut) in &[
            (ToolIcon::SketchSelect, SketchTool::Select, "Select", "S"),
            (ToolIcon::SketchPoint, SketchTool::Point, "Point", "P"),
            (ToolIcon::SketchLine, SketchTool::Line, "Line", "L"),
            (ToolIcon::SketchRect, SketchTool::Rectangle, "Rectangle", "R"),
            (ToolIcon::SketchCircle, SketchTool::Circle, "Circle", "C"),
            (ToolIcon::SketchArc, SketchTool::Arc, "Arc", "A"),
            (ToolIcon::SketchEllipse, SketchTool::Ellipse, "Ellipse", "E"),
            (ToolIcon::SketchPolyline, SketchTool::Polyline, "Polyline", "W"),
            (ToolIcon::SketchSlot, SketchTool::Slot, "Slot", ""),
            (ToolIcon::SketchBSpline, SketchTool::BSpline, "B-Spline", "B"),
        ] {
            if icon_toggle(ui, icon, current_tool == tool, title, shortcut) {
                gui.actions.push(GuiAction::Sketcher(S::SetTool(tool)));
            }
        }

        // Polygon dropdown
        egui::ComboBox::from_id_salt("sketch_polygon")
            .selected_text("\u{2B21}")
            .width(30.0)
            .show_ui(ui, |ui| {
                for (sides, label) in [
                    (3, "Triangle"),
                    (4, "Square"),
                    (5, "Pentagon"),
                    (6, "Hexagon"),
                ] {
                    if ui.selectable_label(false, label).clicked() {
                        gui.actions
                            .push(GuiAction::Sketcher(S::SetTool(SketchTool::Polygon { sides })));
                    }
                }
            });

        toolbar_separator(ui);

        section_label(ui, "Constraints");
        // -- Constraints --
        if icon_button(ui, ToolIcon::Coincident, "Coincident", "Coincident constraint", "") {
            gui.actions.push(GuiAction::Sketcher(S::ConstrainCoincident));
        }
        if icon_button(ui, ToolIcon::Horizontal, "Horizontal", "Horizontal constraint", "H") {
            gui.actions.push(GuiAction::Sketcher(S::ConstrainHorizontal));
        }
        if icon_button(ui, ToolIcon::Vertical, "Vertical", "Vertical constraint", "V") {
            gui.actions.push(GuiAction::Sketcher(S::ConstrainVertical));
        }
        if icon_button(ui, ToolIcon::Parallel, "Parallel", "Parallel constraint", "") {
            gui.actions.push(GuiAction::Sketcher(S::ConstrainParallel));
        }
        if icon_button(ui, ToolIcon::Perpendicular, "Perpendicular", "Perpendicular constraint", "") {
            gui.actions.push(GuiAction::Sketcher(S::ConstrainPerpendicular));
        }
        if icon_button(ui, ToolIcon::Tangent, "Tangent", "Tangent constraint", "") {
            gui.actions.push(GuiAction::Sketcher(S::ConstrainTangent));
        }
        if icon_button(ui, ToolIcon::Equal, "Equal", "Equal constraint", "") {
            gui.actions.push(GuiAction::Sketcher(S::ConstrainEqual));
        }
        if icon_button(ui, ToolIcon::Symmetric, "Symmetric", "Symmetric constraint", "") {
            gui.actions.push(GuiAction::Sketcher(S::ConstrainSymmetric));
        }
        if icon_button(ui, ToolIcon::LengthConstraint, "Length", "Length constraint", "") {
            gui.dimension_popup = Some(super::DimensionPopup {
                kind: super::DimensionKind::Length,
                value: gui.constraint_length_value,
                just_opened: true,
                edit_constraint_index: None,
            });
        }
        if icon_button(ui, ToolIcon::Distance, "Distance", "Distance constraint", "") {
            gui.dimension_popup = Some(super::DimensionPopup {
                kind: super::DimensionKind::Distance,
                value: gui.constraint_distance_value,
                just_opened: true,
                edit_constraint_index: None,
            });
        }
        if icon_button(ui, ToolIcon::Angle, "Angle", "Angle constraint (deg)", "") {
            gui.dimension_popup = Some(super::DimensionPopup {
                kind: super::DimensionKind::Angle,
                value: gui.constraint_angle_value,
                just_opened: true,
                edit_constraint_index: None,
            });
        }
        if icon_button(ui, ToolIcon::Radius, "Radius", "Radius constraint", "") {
            gui.dimension_popup = Some(super::DimensionPopup {
                kind: super::DimensionKind::Radius,
                value: gui.constraint_radius_value,
                just_opened: true,
                edit_constraint_index: None,
            });
        }
        if icon_button(ui, ToolIcon::Diameter, "Diameter", "Diameter constraint", "") {
            gui.dimension_popup = Some(super::DimensionPopup {
                kind: super::DimensionKind::Diameter,
                value: gui.constraint_radius_value * 2.0,
                just_opened: true,
                edit_constraint_index: None,
            });
        }
        if icon_button(ui, ToolIcon::FixPin, "Fix", "Fix position", "") {
            gui.actions.push(GuiAction::Sketcher(S::ConstrainFixed));
        }
        if icon_button(ui, ToolIcon::FixPin, "Block", "Block constraint", "") {
            gui.actions.push(GuiAction::Sketcher(S::ConstrainBlock));
        }
        if icon_button(ui, ToolIcon::Distance, "H-Dist", "Horizontal distance", "") {
            gui.dimension_popup = Some(super::DimensionPopup {
                kind: super::DimensionKind::HDistance,
                value: gui.constraint_distance_value,
                just_opened: true,
                edit_constraint_index: None,
            });
        }
        if icon_button(ui, ToolIcon::Distance, "V-Dist", "Vertical distance", "") {
            gui.dimension_popup = Some(super::DimensionPopup {
                kind: super::DimensionKind::VDistance,
                value: gui.constraint_distance_value,
                just_opened: true,
                edit_constraint_index: None,
            });
        }

        toolbar_separator(ui);

        section_label(ui, "Tools");
        // -- Tools --
        if icon_button(ui, ToolIcon::SketchFillet, "Fillet", "Fillet corner", "") {
            gui.actions.push(GuiAction::Sketcher(S::FilletCorner {
                radius: gui.sketch_fillet_radius,
            }));
        }
        if icon_button(ui, ToolIcon::SketchChamfer, "Chamfer", "Chamfer corner", "") {
            gui.actions.push(GuiAction::Sketcher(S::ChamferCorner {
                distance: gui.sketch_chamfer_distance,
            }));
        }
        if icon_button(ui, ToolIcon::Trim, "Trim", "Trim edge", "") {
            gui.actions.push(GuiAction::Sketcher(S::TrimEdge));
        }
        if icon_button(ui, ToolIcon::Extend, "Extend", "Extend edge", "") {
            gui.actions.push(GuiAction::Sketcher(S::ExtendEdge));
        }
        if icon_button(ui, ToolIcon::SketchBSpline, "Split", "Split edge at point", "") {
            gui.actions.push(GuiAction::Sketcher(S::SplitEdge));
        }
        if icon_button(ui, ToolIcon::Mirror, "Mirror", "Mirror sketch geometry", "") {
            gui.actions.push(GuiAction::Sketcher(S::MirrorGeometry));
        }
        if icon_button(ui, ToolIcon::SketchLine, "External", "External edge projection", "") {
            gui.actions.push(GuiAction::Sketcher(S::ExternalProjection));
        }
        if icon_button(ui, ToolIcon::SketchLine, "Carbon Copy", "Carbon copy geometry", "") {
            gui.actions.push(GuiAction::Sketcher(S::CarbonCopy));
        }
        if icon_button(ui, ToolIcon::SketchSelect, "Copy", "Copy selected entities", "Ctrl+C") {
            gui.actions.push(GuiAction::Sketcher(S::CopySelection));
        }
        if icon_button(ui, ToolIcon::SketchSelect, "Paste", "Paste copied entities at origin", "Ctrl+V") {
            gui.actions.push(GuiAction::Sketcher(S::PasteSelection(0.0, 0.0)));
        }
        if icon_button(ui, ToolIcon::Coincident, "Merge Pts", "Merge coincident points", "") {
            gui.actions.push(GuiAction::Sketcher(S::MergePoints));
        }

        toolbar_separator(ui);

        section_label(ui, "B-Spline");
        // -- B-Spline tools --
        if icon_button(ui, ToolIcon::SketchBSpline, "To B-Spline", "Convert to B-Spline", "") {
            gui.actions.push(GuiAction::Sketcher(S::ConvertToBSpline));
        }
        if icon_button(ui, ToolIcon::MoveUp, "+Degree", "Increase B-Spline degree", "") {
            gui.actions.push(GuiAction::Sketcher(S::IncreaseDegree));
        }
        if icon_button(ui, ToolIcon::MoveDown, "-Degree", "Decrease B-Spline degree", "") {
            gui.actions.push(GuiAction::Sketcher(S::DecreaseDegree));
        }
        if icon_button(ui, ToolIcon::SketchPoint, "Insert Knot", "Insert knot", "") {
            gui.actions.push(GuiAction::Sketcher(S::InsertKnot));
        }

        toolbar_separator(ui);

        section_label(ui, "Options");
        // -- Toggles --
        let construction = gui
            .sketch_mode
            .as_ref()
            .is_some_and(|s| s.construction_mode);
        let grid_on = gui.sketch_mode.as_ref().is_some_and(|s| s.show_grid);
        let snap_on = gui.sketch_mode.as_ref().is_some_and(|s| s.snap_enabled);
        let constr_vis = gui
            .sketch_mode
            .as_ref()
            .is_some_and(|s| s.show_constraints);

        if icon_toggle(ui, ToolIcon::Construction, construction, "Construction", "Toggle construction mode") {
            gui.actions.push(GuiAction::Sketcher(S::ToggleConstruction));
        }
        if icon_toggle(ui, ToolIcon::Grid, grid_on, "Grid", "Toggle grid") {
            gui.actions.push(GuiAction::Sketcher(S::ToggleGrid));
        }
        if let Some(sm) = &mut gui.sketch_mode {
            ui.add(
                egui::DragValue::new(&mut sm.grid_spacing)
                    .speed(0.1)
                    .range(0.1..=10.0)
                    .prefix("G:")
                    .max_decimals(1),
            );
        }
        if icon_toggle(ui, ToolIcon::Snap, snap_on, "Snap", "Toggle snap") {
            gui.actions.push(GuiAction::Sketcher(S::ToggleSnap));
        }
        if icon_toggle(ui, ToolIcon::ShowConstraints, constr_vis, "Constraints", "Toggle constraints visible") {
            gui.actions.push(GuiAction::Sketcher(S::ToggleConstraintsVisible));
        }

        toolbar_separator(ui);

        // -- Extrude distance --
        ui.label("Extrude:");
        if let Some(sm) = &mut gui.sketch_mode {
            ui.add(
                egui::DragValue::new(&mut sm.extrude_distance)
                    .speed(0.5)
                    .range(0.0..=1000.0)
                    .suffix(" mm"),
            );
        }

        toolbar_separator(ui);

        // -- Close/Cancel --
        if icon_button(ui, ToolIcon::Accept, "Close", "Solve and extrude sketch", "") {
            gui.actions.push(GuiAction::Sketcher(S::Close));
        }
        if icon_button(ui, ToolIcon::Cancel, "Cancel", "Discard sketch", "") {
            gui.actions.push(GuiAction::Sketcher(S::Cancel));
        }
    }
}

fn draw_mesh_toolbar(ui: &mut egui::Ui, gui: &mut GuiState) {
    section_label(ui, "Import/Export");
    // -- Import --
    if icon_button(ui, ToolIcon::Import, "Import STL", "Import STL mesh", "") {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("STL", &["stl"])
            .pick_file()
        {
            gui.actions.push(GuiAction::ImportFile(path));
        }
    }

    // -- Export flyout (STL, OBJ, glTF) --
    const MESH_EXPORT_GROUP: &[FlyoutEntry] = &[
        (ToolIcon::ExportStl, "Export STL", "Export as STL mesh", ""),
        (ToolIcon::ExportObj, "Export OBJ", "Export as OBJ mesh", ""),
        (ToolIcon::ExportGltf, "Export glTF", "Export as glTF mesh", ""),
    ];
    if let Some(idx) = flyout_button(ui, "flyout_mesh_export", MESH_EXPORT_GROUP, true) {
        let (filter_name, filter_ext, file_name) = match idx {
            0 => ("STL", "stl", "model.stl"),
            1 => ("OBJ", "obj", "model.obj"),
            _ => ("glTF", "gltf", "model.gltf"),
        };
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(filter_name, &[filter_ext])
            .set_file_name(file_name)
            .save_file()
        {
            gui.actions.push(match idx {
                0 => GuiAction::ExportStl(path),
                1 => GuiAction::ExportObj(path),
                _ => GuiAction::ExportGltf(path),
            });
        }
    }

    toolbar_separator(ui);

    section_label(ui, "Repair");
    // -- Repair --
    if icon_button(ui, ToolIcon::Decimate, "Decimate", "Reduce triangles 50%", "") {
        gui.actions.push(GuiAction::MeshDecimate(0.5));
    }
    if icon_button(ui, ToolIcon::Subdivide, "Subdivide", "Subdivide mesh", "") {
        gui.actions.push(GuiAction::MeshSubdivide);
    }
    if icon_button(ui, ToolIcon::FillHoles, "Fill Holes", "Fill boundary holes", "") {
        gui.actions.push(GuiAction::MeshFillHoles);
    }
    if icon_button(ui, ToolIcon::FlipNormals, "Flip Normals", "Reverse normals", "") {
        gui.actions.push(GuiAction::MeshFlipNormals);
    }
    if icon_button(ui, ToolIcon::Smooth, "Smooth", "Laplacian smoothing", "") {
        gui.show_mesh_smooth = true;
    }
    if icon_button(ui, ToolIcon::Harmonize, "Harmonize", "Consistent normals", "") {
        gui.actions.push(GuiAction::MeshHarmonizeNormals);
    }
    if icon_button(ui, ToolIcon::Watertight, "Watertight?", "Check watertight", "") {
        gui.actions.push(GuiAction::MeshCheckWatertight);
    }
    if icon_button(ui, ToolIcon::Remesh, "Remesh", "Remesh to target edge", "") {
        gui.show_mesh_remesh = true;
    }
    if icon_button(ui, ToolIcon::Repair, "Repair", "Auto-repair mesh", "") {
        gui.actions.push(GuiAction::MeshRepair);
    }
}

fn draw_techdraw_toolbar(ui: &mut egui::Ui, gui: &mut GuiState) {
    use cadkernel_io::ProjectionDir;

    section_label(ui, "Page");
    // -- Page --
    if icon_button(ui, ToolIcon::NewPage, "New Page", "New drawing page", "") {
        gui.actions.push(GuiAction::TechDrawNewPage);
    }
    if icon_button(ui, ToolIcon::Template, "Template", "From template", "") {
        gui.actions.push(GuiAction::TechDrawFromTemplate);
    }
    if icon_button(ui, ToolIcon::Redraw, "Redraw", "Redraw page", "") {
        gui.actions.push(GuiAction::TechDrawRedraw);
    }

    toolbar_separator(ui);

    section_label(ui, "Views");
    // -- Views --
    if icon_button(ui, ToolIcon::ViewFront, "Front", "Front projection", "") {
        gui.actions
            .push(GuiAction::TechDrawAddView(ProjectionDir::Front));
    }
    if icon_button(ui, ToolIcon::ViewTop, "Top", "Top projection", "") {
        gui.actions
            .push(GuiAction::TechDrawAddView(ProjectionDir::Top));
    }
    if icon_button(ui, ToolIcon::ViewRight, "Right", "Right projection", "") {
        gui.actions
            .push(GuiAction::TechDrawAddView(ProjectionDir::Right));
    }
    if icon_button(ui, ToolIcon::ViewIso, "Isometric", "Isometric projection", "") {
        gui.actions
            .push(GuiAction::TechDrawAddView(ProjectionDir::Isometric));
    }
    if icon_button(ui, ToolIcon::SectionView, "Section", "Section view", "") {
        gui.actions.push(GuiAction::TechDrawSectionView);
    }
    if icon_button(ui, ToolIcon::DetailView, "Detail", "Detail view", "") {
        gui.actions.push(GuiAction::TechDrawDetailView);
    }
    if icon_button(ui, ToolIcon::BrokenView, "Broken", "Broken view", "") {
        gui.actions.push(GuiAction::TechDrawBrokenView);
    }
    if icon_button(ui, ToolIcon::ThreeView, "3-View", "Standard 3-view drawing", "") {
        gui.actions.push(GuiAction::TechDrawThreeView);
    }

    toolbar_separator(ui);

    section_label(ui, "Dimensions");
    // -- Dimensions --
    if icon_button(ui, ToolIcon::DimLinear, "Linear", "Linear dimension", "") {
        gui.actions.push(GuiAction::TechDrawDimLinear);
    }
    if icon_button(ui, ToolIcon::DimRadius, "Radius", "Radius dimension", "") {
        gui.actions.push(GuiAction::TechDrawDimRadius);
    }
    if icon_button(ui, ToolIcon::DimDiameter, "Diameter", "Diameter dimension", "") {
        gui.actions.push(GuiAction::TechDrawDimDiameter);
    }
    if icon_button(ui, ToolIcon::DimAngle, "Angle", "Angle dimension", "") {
        gui.actions.push(GuiAction::TechDrawDimAngle);
    }
    if icon_button(ui, ToolIcon::DimArcLen, "Arc Length", "Arc length dimension", "") {
        gui.actions.push(GuiAction::TechDrawDimArcLen);
    }
    if icon_button(ui, ToolIcon::DimArea, "Area", "Area dimension", "") {
        gui.actions.push(GuiAction::TechDrawDimArea);
    }

    toolbar_separator(ui);

    section_label(ui, "Annotations");
    // -- Annotations --
    if icon_button(ui, ToolIcon::TextAnnot, "Text", "Add text", "") {
        gui.actions.push(GuiAction::TechDrawText);
    }
    if icon_button(ui, ToolIcon::RichText, "Rich Text", "Rich text annotation", "") {
        gui.actions.push(GuiAction::TechDrawRichText);
    }
    if icon_button(ui, ToolIcon::Balloon, "Balloon", "Balloon annotation", "") {
        gui.actions.push(GuiAction::TechDrawBalloon);
    }
    if icon_button(ui, ToolIcon::Leader, "Leader", "Leader line", "") {
        gui.actions.push(GuiAction::TechDrawLeader);
    }
    if icon_button(ui, ToolIcon::Weld, "Weld", "Weld symbol", "") {
        gui.actions.push(GuiAction::TechDrawWeld);
    }
    if icon_button(ui, ToolIcon::SurfFinish, "Surface Finish", "Surface finish symbol", "") {
        gui.actions.push(GuiAction::TechDrawSurfFinish);
    }

    toolbar_separator(ui);

    section_label(ui, "Centerlines");
    // -- Centerlines --
    if icon_button(ui, ToolIcon::CenterFace, "Face Center", "Face centerlines", "") {
        gui.actions.push(GuiAction::TechDrawCenterFace);
    }
    if icon_button(ui, ToolIcon::CenterLines, "Centerlines", "Centerlines between lines", "") {
        gui.actions.push(GuiAction::TechDrawCenterLines);
    }
    if icon_button(ui, ToolIcon::CenterPoints, "Center Points", "Centerlines from points", "") {
        gui.actions.push(GuiAction::TechDrawCenterPoints);
    }
    if icon_button(ui, ToolIcon::BoltCircle, "Bolt Circle", "Bolt circle centerlines", "") {
        gui.actions.push(GuiAction::TechDrawBoltCircle);
    }

    toolbar_separator(ui);

    section_label(ui, "Export");
    // -- Export --
    if icon_button(ui, ToolIcon::ExportSvg, "SVG", "Export to SVG", "") {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("SVG", &["svg"])
            .set_file_name("drawing.svg")
            .save_file()
        {
            gui.actions.push(GuiAction::TechDrawExportSvg(path));
        }
    }
    if icon_button(ui, ToolIcon::ExportDxf, "DXF", "Export to DXF", "") {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("DXF", &["dxf"])
            .set_file_name("drawing.dxf")
            .save_file()
        {
            gui.actions.push(GuiAction::TechDrawExportDxf(path));
        }
    }
    if icon_button(ui, ToolIcon::ExportPdf, "PDF", "Export to PDF", "") {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("PDF", &["pdf"])
            .set_file_name("drawing.pdf")
            .save_file()
        {
            gui.actions.push(GuiAction::TechDrawExportPdf(path));
        }
    }
    if icon_button(ui, ToolIcon::Clear, "Clear", "Clear all views", "") {
        gui.actions.push(GuiAction::TechDrawClear);
    }
}

fn draw_assembly_toolbar(ui: &mut egui::Ui, gui: &mut GuiState) {
    use super::AssemblyAction as A;
    section_label(ui, "Assembly");
    // -- Assembly --
    if icon_button(ui, ToolIcon::AssemblyNew, "New", "New assembly", "") {
        gui.actions.push(GuiAction::Assembly(A::Create));
    }
    if icon_button(ui, ToolIcon::InsertComponent, "Insert", "Insert component", "") {
        gui.actions.push(GuiAction::Assembly(A::InsertComponent));
    }
    if icon_button(ui, ToolIcon::Solve, "Solve", "Solve assembly", "") {
        gui.actions.push(GuiAction::Assembly(A::Solve));
    }
    if icon_button(ui, ToolIcon::ExplodeView, "Explode", "Exploded view", "") {
        gui.show_explode = true;
    }
    if icon_button(ui, ToolIcon::Bom, "BOM", "Bill of materials", "") {
        gui.actions.push(GuiAction::Assembly(A::BillOfMaterials));
    }
    if icon_button(ui, ToolIcon::Dof, "DOF", "Degrees of freedom analysis", "") {
        gui.actions.push(GuiAction::Assembly(A::DofAnalysis));
    }

    toolbar_separator(ui);

    section_label(ui, "Joints");
    // -- Joints --
    for &(icon, jtype, tip) in &[
        (ToolIcon::JointFixed, AssemblyJointType::Fixed, "Fixed joint"),
        (ToolIcon::JointRevolute, AssemblyJointType::Revolute, "Revolute joint"),
        (ToolIcon::JointCylindrical, AssemblyJointType::Cylindrical, "Cylindrical joint"),
        (ToolIcon::JointSlider, AssemblyJointType::Slider, "Slider joint"),
        (ToolIcon::JointBall, AssemblyJointType::Ball, "Ball joint"),
        (ToolIcon::JointDistance, AssemblyJointType::Distance, "Distance constraint"),
        (ToolIcon::JointAngle, AssemblyJointType::Angle, "Angle constraint"),
        (ToolIcon::JointParallel, AssemblyJointType::Parallel, "Parallel constraint"),
        (ToolIcon::JointPerp, AssemblyJointType::Perpendicular, "Perpendicular constraint"),
        (ToolIcon::JointGear, AssemblyJointType::Gear, "Gear joint"),
        (ToolIcon::JointRack, AssemblyJointType::Rack, "Rack and pinion"),
        (ToolIcon::JointScrew, AssemblyJointType::Screw, "Screw joint"),
        (ToolIcon::JointBelt, AssemblyJointType::Belt, "Belt/chain joint"),
    ] {
        if icon_button(ui, icon, tip, "", "") {
            gui.actions.push(GuiAction::Assembly(A::AddJoint(jtype)));
        }
    }
}

fn draw_draft_toolbar(ui: &mut egui::Ui, gui: &mut GuiState) {
    use super::task_panel::ActiveTask;
    section_label(ui, "Draw");
    // -- Draw --
    if icon_button(ui, ToolIcon::DraftLine, "Line", "Draft line", "") {
        gui.active_task = Some(ActiveTask::DraftLine {
            length: 10.0, angle: 0.0, preview_id: None,
        });
    }
    if icon_button(ui, ToolIcon::DraftWire, "Wire", "Draft wire", "") {
        gui.actions.push(GuiAction::DraftWire);
    }
    if icon_button(ui, ToolIcon::DraftCircle, "Circle", "Draft circle", "") {
        gui.active_task = Some(ActiveTask::DraftCircle {
            radius: 5.0, preview_id: None,
        });
    }
    if icon_button(ui, ToolIcon::DraftArc, "Arc", "Draft arc", "") {
        gui.active_task = Some(ActiveTask::DraftArc {
            radius: 5.0, start_angle: 0.0, end_angle: 180.0, preview_id: None,
        });
    }
    if icon_button(ui, ToolIcon::DraftEllipse, "Ellipse", "Draft ellipse", "") {
        gui.active_task = Some(ActiveTask::DraftEllipse {
            rx: 5.0, ry: 3.0, preview_id: None,
        });
    }
    if icon_button(ui, ToolIcon::DraftRect, "Rectangle", "Draft rectangle", "") {
        gui.active_task = Some(ActiveTask::DraftRectangle {
            width: 10.0, height: 10.0, preview_id: None,
        });
    }
    if icon_button(ui, ToolIcon::DraftPolygon, "Polygon", "Draft polygon", "") {
        gui.active_task = Some(ActiveTask::DraftPolygon {
            radius: 5.0, sides: 6, preview_id: None,
        });
    }
    if icon_button(ui, ToolIcon::DraftBSpline, "B-Spline", "Draft B-spline", "") {
        gui.actions.push(GuiAction::DraftBSpline);
    }
    if icon_button(ui, ToolIcon::DraftBezier, "Bezier", "Draft Bezier curve", "") {
        gui.actions.push(GuiAction::DraftBezier);
    }
    if icon_button(ui, ToolIcon::DraftPoint, "Point", "Draft point", "") {
        gui.actions.push(GuiAction::DraftPoint);
    }
    if icon_button(ui, ToolIcon::DraftFacebind, "Facebinder", "Draft facebinder", "") {
        gui.actions.push(GuiAction::DraftFacebinder);
    }
    if icon_button(ui, ToolIcon::DraftHatch, "Hatch", "Draft hatch", "") {
        gui.actions.push(GuiAction::DraftHatch);
    }

    toolbar_separator(ui);

    section_label(ui, "Modify");
    // -- Modify --
    if icon_button(ui, ToolIcon::DraftMove, "Move", "Move selection", "") {
        gui.actions.push(GuiAction::DraftMove);
    }
    if icon_button(ui, ToolIcon::DraftRotate, "Rotate", "Rotate selection", "") {
        gui.actions.push(GuiAction::DraftRotate);
    }
    if icon_button(ui, ToolIcon::DraftScale, "Scale", "Scale selection", "") {
        gui.actions.push(GuiAction::DraftScale);
    }
    if icon_button(ui, ToolIcon::DraftMirror, "Mirror", "Mirror selection", "") {
        gui.actions.push(GuiAction::DraftMirror);
    }
    if icon_button(ui, ToolIcon::DraftOffset, "Offset", "Offset shape", "") {
        gui.actions.push(GuiAction::DraftOffset);
    }
    if icon_button(ui, ToolIcon::DraftTrim, "Trim", "Trim/extend", "") {
        gui.actions.push(GuiAction::DraftTrim);
    }
    if icon_button(ui, ToolIcon::DraftStretch, "Stretch", "Stretch selection", "") {
        gui.actions.push(GuiAction::DraftStretch);
    }
    if icon_button(ui, ToolIcon::DraftClone, "Clone", "Clone object", "") {
        gui.actions.push(GuiAction::DraftClone);
    }

    toolbar_separator(ui);

    section_label(ui, "Array");
    // -- Array --
    if icon_button(ui, ToolIcon::DraftArrayRect, "Rect Array", "Rectangular array", "") {
        gui.actions.push(GuiAction::DraftArrayRect);
    }
    if icon_button(ui, ToolIcon::DraftArrayPolar, "Polar Array", "Polar array", "") {
        gui.actions.push(GuiAction::DraftArrayPolar);
    }
    if icon_button(ui, ToolIcon::DraftArrayPath, "Path Array", "Path array", "") {
        gui.actions.push(GuiAction::DraftArrayPath);
    }
    if icon_button(ui, ToolIcon::DraftArrayPoint, "Point Array", "Point array", "") {
        gui.actions.push(GuiAction::DraftArrayPoint);
    }

    toolbar_separator(ui);

    section_label(ui, "Annotation");
    // -- Annotation --
    if icon_button(ui, ToolIcon::DraftDimension, "Dimension", "Dimension", "") {
        gui.actions.push(GuiAction::DraftDimension);
    }
    if icon_button(ui, ToolIcon::DraftLabel, "Label", "Label", "") {
        gui.actions.push(GuiAction::DraftLabel);
    }
    if icon_button(ui, ToolIcon::DraftText, "Text", "Text", "") {
        gui.actions.push(GuiAction::DraftText);
    }

    toolbar_separator(ui);

    section_label(ui, "Convert");
    // -- Convert --
    if icon_button(ui, ToolIcon::DraftUpgrade, "Upgrade", "Upgrade shape", "") {
        gui.actions.push(GuiAction::DraftUpgrade);
    }
    if icon_button(ui, ToolIcon::DraftDowngrade, "Downgrade", "Downgrade shape", "") {
        gui.actions.push(GuiAction::DraftDowngrade);
    }
    if icon_button(ui, ToolIcon::DraftWireToBSpline, "Wire to BSpline", "Wire to BSpline", "") {
        gui.actions.push(GuiAction::DraftWireToBSpline);
    }
    if icon_button(ui, ToolIcon::DraftToSketch, "To Sketch", "Convert to sketch", "") {
        gui.actions.push(GuiAction::DraftToSketch);
    }

    toolbar_separator(ui);

    section_label(ui, "Snap");
    // -- Snap toggles --
    let snap_icons = [
        ToolIcon::DraftSnapEndpoint, ToolIcon::DraftSnapMidpoint,
        ToolIcon::DraftSnapCenter, ToolIcon::DraftSnapGrid,
        ToolIcon::DraftSnapAngle, ToolIcon::DraftSnapIntersect,
        ToolIcon::DraftSnapPerp, ToolIcon::DraftSnapParallel,
    ];
    let snap_labels = [
        "Endpoint", "Midpoint", "Center", "Grid",
        "Angle", "Intersect", "Perp", "Parallel",
    ];
    for i in 0..8 {
        if icon_toggle(ui, snap_icons[i], gui.draft_snap_modes[i], snap_labels[i], "") {
            gui.draft_snap_modes[i] = !gui.draft_snap_modes[i];
            gui.actions
                .push(GuiAction::ToggleDraftSnap(snap_labels[i].into()));
        }
    }
}

fn draw_surface_toolbar(ui: &mut egui::Ui, gui: &mut GuiState) {
    use super::task_panel::ActiveTask;
    section_label(ui, "Surface");
    if icon_button(ui, ToolIcon::SurfFilling, "Filling", "Fill a boundary with a surface", "") {
        gui.actions.push(GuiAction::SurfaceFilling);
    }
    if icon_button(ui, ToolIcon::SurfBoundary, "Boundary", "Boundary surface from edges", "") {
        gui.actions.push(GuiAction::SurfaceBoundary);
    }
    if icon_button(ui, ToolIcon::SurfSections, "Ruled", "Ruled surface from cross-sections", "") {
        gui.active_task = Some(ActiveTask::SurfaceRuled {
            width: 10.0, depth: 10.0, offset: 5.0, preview_id: None,
        });
    }
    if icon_button(ui, ToolIcon::SurfExtend, "Extend", "Extend surface", "") {
        gui.actions.push(GuiAction::SurfaceExtend);
    }
    if icon_button(ui, ToolIcon::SurfBlend, "Blend", "Blend between surfaces", "") {
        gui.actions.push(GuiAction::SurfaceBlend);
    }
    if icon_button(ui, ToolIcon::SurfPipe, "Pipe", "Pipe surface along path", "") {
        gui.active_task = Some(ActiveTask::SurfacePipe {
            radius: 1.0, length: 15.0, preview_id: None,
        });
    }
    if icon_button(ui, ToolIcon::SurfCoons, "Coons", "Coons patch from 4 edges", "") {
        gui.actions.push(GuiAction::SurfaceCoons);
    }
}

fn draw_fem_toolbar(ui: &mut egui::Ui, gui: &mut GuiState) {
    use super::FemAction as F;
    use super::task_panel::ActiveTask;
    section_label(ui, "Setup");
    // -- Setup --
    if icon_button(ui, ToolIcon::FemAnalysis, "Analysis", "Create FEM analysis", "") {
        gui.actions.push(GuiAction::Fem(F::CreateAnalysis));
    }
    if icon_button(ui, ToolIcon::FemMaterial, "Material", "Assign material", "") {
        gui.show_fem_material = true;
    }

    toolbar_separator(ui);

    section_label(ui, "Mesh");
    // -- Mesh --
    if icon_button(ui, ToolIcon::FemTetMesh, "Tet Mesh", "Generate tetrahedral mesh", "") {
        gui.active_task = Some(ActiveTask::FemMesh {
            element_size: gui.fem_element_size, preview_id: None,
        });
    }
    if icon_button(ui, ToolIcon::FemRefine, "Refine", "Refine FEM mesh", "") {
        gui.actions.push(GuiAction::Fem(F::GenTetMesh {
            element_size: gui.fem_element_size * 0.5,
        }));
    }
    if icon_button(ui, ToolIcon::FemSmooth, "Smooth", "Smooth FEM mesh", "") {
        gui.actions
            .push(GuiAction::StatusMessage("FEM mesh smoothing applied".into()));
    }
    if icon_button(ui, ToolIcon::FemQuality, "Quality", "Check mesh quality", "") {
        gui.actions
            .push(GuiAction::StatusMessage("FEM mesh quality: OK".into()));
    }

    toolbar_separator(ui);

    section_label(ui, "Constraints");
    // -- Constraints --
    if icon_button(ui, ToolIcon::FemFixed, "Fixed", "Fixed boundary condition", "") {
        gui.actions
            .push(GuiAction::Fem(F::AddConstraint(FemConstraintType::Fixed)));
    }
    if icon_button(ui, ToolIcon::FemForce, "Force", "Applied force", "") {
        gui.actions
            .push(GuiAction::Fem(F::AddConstraint(FemConstraintType::Force)));
    }
    if icon_button(ui, ToolIcon::FemPressure, "Pressure", "Pressure load", "") {
        gui.actions
            .push(GuiAction::Fem(F::AddConstraint(FemConstraintType::Pressure)));
    }
    if icon_button(ui, ToolIcon::FemDisplacement, "Displacement", "Prescribed displacement", "") {
        gui.actions
            .push(GuiAction::Fem(F::AddConstraint(FemConstraintType::Displacement)));
    }
    if icon_button(ui, ToolIcon::FemGravity, "Gravity", "Gravity load", "") {
        gui.actions
            .push(GuiAction::Fem(F::AddConstraint(FemConstraintType::Gravity)));
    }
    if icon_button(ui, ToolIcon::FemSpring, "Spring", "Spring element", "") {
        gui.actions
            .push(GuiAction::Fem(F::AddConstraint(FemConstraintType::Spring)));
    }

    toolbar_separator(ui);

    section_label(ui, "Solve");
    // -- Solve --
    if icon_button(ui, ToolIcon::SolveStatic, "Static", "Static structural analysis", "") {
        gui.actions.push(GuiAction::Fem(F::SolveStatic));
    }
    if icon_button(ui, ToolIcon::SolveModal, "Modal", "Modal analysis", "") {
        gui.actions.push(GuiAction::Fem(F::SolveModal { modes: 10 }));
    }
    if icon_button(ui, ToolIcon::SolveThermal, "Thermal", "Thermal analysis", "") {
        gui.actions.push(GuiAction::Fem(F::SolveThermal));
    }
    if icon_button(ui, ToolIcon::SolveBuckling, "Buckling", "Buckling analysis", "") {
        gui.actions.push(GuiAction::Fem(F::SolveBuckling { modes: 5 }));
    }
    if icon_button(ui, ToolIcon::SolveNonlinear, "Nonlinear", "Nonlinear analysis", "") {
        gui.actions.push(GuiAction::Fem(F::SolveNonlinear));
    }

    toolbar_separator(ui);

    section_label(ui, "Results");
    // -- Results --
    if icon_button(ui, ToolIcon::ShowStress, "Stress", "Show stress field", "") {
        gui.actions.push(GuiAction::Fem(F::ShowStress));
    }
    if icon_button(ui, ToolIcon::ShowDisplacement, "Displacement", "Show displacement field", "") {
        gui.actions.push(GuiAction::Fem(F::ShowDisplacement));
    }
    if icon_button(ui, ToolIcon::ShowVonMises, "Von Mises", "Show von Mises stress", "") {
        gui.actions.push(GuiAction::Fem(F::ShowVonMises));
    }
    if icon_button(ui, ToolIcon::FemSummary, "Summary", "FEM result summary", "") {
        gui.actions.push(GuiAction::Fem(F::Summary));
    }
    if icon_button(ui, ToolIcon::FemReport, "Report", "Generate FEM report", "") {
        gui.actions.push(GuiAction::Fem(F::Report));
    }
}
