use super::{GuiAction, GuiState};
use crate::render::{Camera, Projection, StandardView, dot3, normalize3, sub3};

use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};
const FRAC_3PI_4: f32 = PI * 0.75;
const CORNER_PITCH: f32 = 0.6155;
const CHAMFER: f32 = 0.22;
const EDGE_BEVEL: f32 = 0.24;

fn build_chamfer_verts() -> [[f32; 3]; 24] {
    let s = 1.0f32;
    let c = s - 2.0 * CHAMFER;
    [
        [-c, -s, -s],
        [-s, -c, -s],
        [-s, -s, -c],
        [c, -s, -s],
        [s, -c, -s],
        [s, -s, -c],
        [s, c, -s],
        [c, s, -s],
        [s, s, -c],
        [-c, s, -s],
        [-s, c, -s],
        [-s, s, -c],
        [-c, -s, s],
        [-s, -c, s],
        [-s, -s, c],
        [c, -s, s],
        [s, -c, s],
        [s, -s, c],
        [s, c, s],
        [c, s, s],
        [s, s, c],
        [-c, s, s],
        [-s, c, s],
        [-s, s, c],
    ]
}

const FACE_OCTAGONS: [[usize; 8]; 6] = [
    [21, 19, 20, 8, 7, 9, 11, 23],
    [0, 3, 5, 17, 15, 12, 14, 2],
    [17, 5, 4, 6, 8, 20, 18, 16],
    [2, 14, 13, 22, 23, 11, 10, 1],
    [12, 15, 16, 18, 19, 21, 22, 13],
    [3, 0, 1, 10, 9, 7, 6, 4],
];
const FACE_NORMALS: [[f32; 3]; 6] = [
    [0.0, 1.0, 0.0],
    [0.0, -1.0, 0.0],
    [1.0, 0.0, 0.0],
    [-1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0],
    [0.0, 0.0, -1.0],
];
const FACE_LABELS: [&str; 6] = ["FRONT", "BACK", "RIGHT", "LEFT", "TOP", "BOTTOM"];
const FACE_TEXT_RIGHT: [[f32; 3]; 6] = [
    [-1.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, -1.0, 0.0],
    [-1.0, 0.0, 0.0],
    [-1.0, 0.0, 0.0],
];
const FACE_VIEWS: [StandardView; 6] = [
    StandardView::Front,
    StandardView::Back,
    StandardView::Right,
    StandardView::Left,
    StandardView::Top,
    StandardView::Bottom,
];

const CORNER_TRIS: [[usize; 3]; 8] = [
    [0, 2, 1],
    [3, 4, 5],
    [6, 7, 8],
    [9, 10, 11],
    [12, 13, 14],
    [15, 17, 16],
    [18, 20, 19],
    [21, 23, 22],
];
const CORNER_NORMALS: [[f32; 3]; 8] = [
    [-1.0, -1.0, -1.0],
    [1.0, -1.0, -1.0],
    [1.0, 1.0, -1.0],
    [-1.0, 1.0, -1.0],
    [-1.0, -1.0, 1.0],
    [1.0, -1.0, 1.0],
    [1.0, 1.0, 1.0],
    [-1.0, 1.0, 1.0],
];
const CORNER_YAW_PITCH: [[f32; 2]; 8] = [
    [-FRAC_3PI_4, -CORNER_PITCH],
    [-FRAC_PI_4, -CORNER_PITCH],
    [FRAC_PI_4, -CORNER_PITCH],
    [FRAC_3PI_4, -CORNER_PITCH],
    [-FRAC_3PI_4, CORNER_PITCH],
    [-FRAC_PI_4, CORNER_PITCH],
    [FRAC_PI_4, CORNER_PITCH],
    [FRAC_3PI_4, CORNER_PITCH],
];

const EDGE_SEGS: [[usize; 2]; 12] = [
    [0, 3],
    [4, 6],
    [7, 9],
    [10, 1],
    [12, 15],
    [16, 18],
    [19, 21],
    [22, 13],
    [2, 14],
    [5, 17],
    [8, 20],
    [11, 23],
];
const EDGE_YAW_PITCH: [[f32; 2]; 12] = [
    [-FRAC_PI_2, -FRAC_PI_4],
    [0.0, -FRAC_PI_4],
    [FRAC_PI_2, -FRAC_PI_4],
    [PI, -FRAC_PI_4],
    [-FRAC_PI_2, FRAC_PI_4],
    [0.0, FRAC_PI_4],
    [FRAC_PI_2, FRAC_PI_4],
    [PI, FRAC_PI_4],
    [-FRAC_3PI_4, 0.0],
    [-FRAC_PI_4, 0.0],
    [FRAC_PI_4, 0.0],
    [FRAC_3PI_4, 0.0],
];
const EDGE_ADJ_FACES: [[usize; 2]; 12] = [
    [5, 1],
    [5, 2],
    [5, 0],
    [5, 3],
    [4, 1],
    [4, 2],
    [4, 0],
    [4, 3],
    [3, 1],
    [2, 1],
    [2, 0],
    [3, 0],
];
const CORNER_ADJ_FACES: [[usize; 3]; 8] = [
    [5, 3, 1],
    [5, 2, 1],
    [5, 2, 0],
    [5, 3, 0],
    [4, 3, 1],
    [4, 2, 1],
    [4, 2, 0],
    [4, 3, 0],
];
#[allow(dead_code)]
const FACE_EDGE_ADJ: [[usize; 4]; 6] = [
    [4, 2, 5, 3],
    [5, 2, 4, 3],
    [1, 5, 0, 4],
    [1, 4, 0, 5],
    [1, 2, 0, 3],
    [1, 3, 0, 2],
];
const INV_SQRT2: f32 = std::f32::consts::FRAC_1_SQRT_2;
const EDGE_NORMALS: [[f32; 3]; 12] = [
    [0.0, -INV_SQRT2, -INV_SQRT2],
    [INV_SQRT2, 0.0, -INV_SQRT2],
    [0.0, INV_SQRT2, -INV_SQRT2],
    [-INV_SQRT2, 0.0, -INV_SQRT2],
    [0.0, -INV_SQRT2, INV_SQRT2],
    [INV_SQRT2, 0.0, INV_SQRT2],
    [0.0, INV_SQRT2, INV_SQRT2],
    [-INV_SQRT2, 0.0, INV_SQRT2],
    [-INV_SQRT2, -INV_SQRT2, 0.0],
    [INV_SQRT2, -INV_SQRT2, 0.0],
    [INV_SQRT2, INV_SQRT2, 0.0],
    [-INV_SQRT2, INV_SQRT2, 0.0],
];

pub(crate) fn draw_view_cube(
    ctx: &egui::Context,
    camera: &Camera,
    gui: &mut GuiState,
    nav: &crate::nav::NavConfig,
) {
    let actions = &mut gui.actions;
    let cube_half = nav.cube_size * 0.5;
    // Total radius: cube_half*1.6 (ring) + 14 (arrows) + 8 (label text)
    let total_radius = cube_half * 1.6 + 22.0;
    let margin = total_radius + 8.0;
    let viewport = ctx.screen_rect(); // use FULL screen rect, not available (which is reduced by panels)
    let center = match nav.cube_corner {
        1 => egui::pos2(viewport.left() + margin, viewport.top() + margin + 6.0),     // TopLeft
        2 => egui::pos2(viewport.left() + margin, viewport.bottom() - margin - 6.0),  // BottomLeft
        3 => egui::pos2(viewport.right() - margin, viewport.bottom() - margin - 6.0), // BottomRight
        _ => egui::pos2(viewport.right() - margin, viewport.top() + margin + 6.0),    // TopRight (default)
    };

    let eye = camera.eye();
    let target = camera.target;
    let fwd = normalize3(sub3(target, eye));
    let cam_right = camera.screen_right();
    let cam_up = camera.screen_up();
    let light_dir = normalize3([0.4, 0.5, 0.75]);

    let cverts = build_chamfer_verts();
    let project = |v: [f32; 3]| -> (egui::Pos2, f32) {
        let sx = dot3(v, cam_right) * cube_half;
        let sy = -dot3(v, cam_up) * cube_half;
        (egui::pos2(center.x + sx, center.y + sy), dot3(v, fwd))
    };

    let edge_quads_2d: Vec<[(egui::Pos2, f32); 4]> = (0..12)
        .map(|ei| {
            let [a, b] = EDGE_SEGS[ei];
            let [fi1, fi2] = EDGE_ADJ_FACES[ei];
            let n1 = FACE_NORMALS[fi1];
            let n2 = FACE_NORMALS[fi2];
            let va = cverts[a];
            let vb = cverts[b];
            let bv = EDGE_BEVEL;
            [
                project([va[0] - n2[0] * bv, va[1] - n2[1] * bv, va[2] - n2[2] * bv]),
                project([vb[0] - n2[0] * bv, vb[1] - n2[1] * bv, vb[2] - n2[2] * bv]),
                project([vb[0] - n1[0] * bv, vb[1] - n1[1] * bv, vb[2] - n1[2] * bv]),
                project([va[0] - n1[0] * bv, va[1] - n1[1] * bv, va[2] - n1[2] * bv]),
            ]
        })
        .collect();

    let face_2d: Vec<Vec<(egui::Pos2, f32)>> = (0..6)
        .map(|fi| {
            let fn_ = FACE_NORMALS[fi];
            FACE_OCTAGONS[fi]
                .iter()
                .map(|&vi| {
                    let mut v = cverts[vi];
                    for k in 0..3 {
                        if fn_[k].abs() < 0.5 && v[k].abs() > 0.9 {
                            v[k] -= v[k].signum() * EDGE_BEVEL;
                        }
                    }
                    project(v)
                })
                .collect()
        })
        .collect();

    let corner_hex_2d: Vec<Vec<(egui::Pos2, f32)>> = (0..8)
        .map(|ci| {
            let tri = CORNER_TRIS[ci];
            let [va, vb, vc] = [cverts[tri[0]], cverts[tri[1]], cverts[tri[2]]];
            let shared_n = |a: [f32; 3], b: [f32; 3]| -> [f32; 3] {
                for k in 0..3 {
                    if a[k].abs() > 0.9 && b[k].abs() > 0.9 {
                        let mut n = [0.0f32; 3];
                        n[k] = a[k].signum();
                        return n;
                    }
                }
                [0.0; 3]
            };
            let n_ab = shared_n(va, vb);
            let n_bc = shared_n(vb, vc);
            let n_ca = shared_n(vc, va);
            let bv = EDGE_BEVEL;
            let ins = |v: [f32; 3], n: [f32; 3]| {
                project([v[0] - n[0] * bv, v[1] - n[1] * bv, v[2] - n[2] * bv])
            };
            vec![
                ins(va, n_ab),
                ins(va, n_ca),
                ins(vb, n_bc),
                ins(vb, n_ab),
                ins(vc, n_ca),
                ins(vc, n_bc),
            ]
        })
        .collect();

    let mut poly_order: Vec<usize> = (0..26).collect();
    poly_order.sort_by(|&a, &b| {
        let avg_depth = |id: usize| -> f32 {
            if id < 6 {
                face_2d[id].iter().map(|q| q.1).sum::<f32>() / 8.0
            } else if id < 14 {
                corner_hex_2d[id - 6].iter().map(|q| q.1).sum::<f32>() / 6.0
            } else {
                edge_quads_2d[id - 14].iter().map(|q| q.1).sum::<f32>() / 4.0
            }
        };
        avg_depth(a)
            .partial_cmp(&avg_depth(b))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("view_cube"),
    ));
    let mouse_pos = ctx.input(|i| i.pointer.hover_pos());
    let clicked = ctx.input(|i| i.pointer.button_clicked(egui::PointerButton::Primary));

    // Drop shadow
    painter.circle_filled(
        egui::pos2(center.x + 2.0, center.y + 3.0),
        cube_half * 1.45,
        egui::Color32::from_rgba_premultiplied(0, 0, 0, 35),
    );

    // Orbit ring + compass
    let ring_r = cube_half * 1.6;
    painter.circle_stroke(
        center,
        ring_r,
        egui::Stroke::new(1.5, egui::Color32::from_rgba_premultiplied(70, 72, 82, 120)),
    );
    let compass: [([f32; 3], &str); 4] = [
        ([0.0, 1.0, 0.0], "F"),
        ([1.0, 0.0, 0.0], "R"),
        ([0.0, -1.0, 0.0], "B"),
        ([-1.0, 0.0, 0.0], "L"),
    ];
    for (dir, label) in &compass {
        let dx = dot3(*dir, cam_right);
        let dy = -dot3(*dir, cam_up);
        let len = (dx * dx + dy * dy).sqrt().max(0.001);
        let (nx, ny) = (dx / len, dy / len);
        let alpha = (len * 220.0).min(200.0) as u8;
        if alpha > 40 {
            painter.text(
                egui::pos2(
                    center.x + nx * (ring_r + 10.0),
                    center.y + ny * (ring_r + 10.0),
                ),
                egui::Align2::CENTER_CENTER,
                *label,
                egui::FontId::proportional(9.0),
                egui::Color32::from_rgba_premultiplied(150, 155, 165, alpha),
            );
        }
        let (ti, to) = (ring_r - 3.0, ring_r + 3.0);
        painter.line_segment(
            [
                egui::pos2(center.x + nx * ti, center.y + ny * ti),
                egui::pos2(center.x + nx * to, center.y + ny * to),
            ],
            egui::Stroke::new(
                1.0,
                egui::Color32::from_rgba_premultiplied(90, 92, 100, alpha),
            ),
        );
    }

    // Hover detection
    #[derive(Clone, Copy)]
    enum HoverTarget {
        Face(usize),
        Edge(usize),
        Corner(usize),
    }
    let mut hover: Option<HoverTarget> = None;

    let face_visible = |fi: usize| dot3(FACE_NORMALS[fi], fwd) < 0.0;

    if let Some(mpos) = mouse_pos {
        for ci in 0..8 {
            let [f1, f2, f3] = CORNER_ADJ_FACES[ci];
            if !face_visible(f1) && !face_visible(f2) && !face_visible(f3) {
                continue;
            }
            let pts: Vec<egui::Pos2> = corner_hex_2d[ci].iter().map(|q| q.0).collect();
            if point_in_convex_poly(mpos, &pts) {
                hover = Some(HoverTarget::Corner(ci));
                break;
            }
        }
        if hover.is_none() {
            for ei in 0..12 {
                let [f1, f2] = EDGE_ADJ_FACES[ei];
                if !face_visible(f1) && !face_visible(f2) {
                    continue;
                }
                let pts: Vec<egui::Pos2> = edge_quads_2d[ei].iter().map(|q| q.0).collect();
                if point_in_convex_poly(mpos, &pts) {
                    hover = Some(HoverTarget::Edge(ei));
                    break;
                }
            }
        }
        if hover.is_none() {
            for &pi in poly_order.iter().rev() {
                if pi >= 6 {
                    continue;
                }
                if !face_visible(pi) {
                    continue;
                }
                let pts: Vec<egui::Pos2> = face_2d[pi].iter().map(|q| q.0).collect();
                if point_in_convex_poly(mpos, &pts) {
                    hover = Some(HoverTarget::Face(pi));
                    break;
                }
            }
        }
    }

    // Draw all polygons as a single epaint::Mesh
    let mut cube_mesh = egui::epaint::Mesh::default();
    let mut hovered_poly: Option<(Vec<egui::Pos2>, egui::Color32)> = None;

    struct FaceLabelInfo {
        cx: f32,
        cy: f32,
        pi: usize,
        facing: f32,
        is_hovered: bool,
    }
    let mut face_labels: Vec<FaceLabelInfo> = Vec::new();

    for &pi in &poly_order {
        let (normal, pts) = if pi < 6 {
            let pts: Vec<egui::Pos2> = face_2d[pi].iter().map(|q| q.0).collect();
            (FACE_NORMALS[pi], pts)
        } else if pi < 14 {
            let ci = pi - 6;
            let pts: Vec<egui::Pos2> = corner_hex_2d[ci].iter().map(|q| q.0).collect();
            (normalize3(CORNER_NORMALS[ci]), pts)
        } else {
            let ei = pi - 14;
            let pts: Vec<egui::Pos2> = edge_quads_2d[ei].iter().map(|q| q.0).collect();
            (EDGE_NORMALS[ei], pts)
        };

        let facing = -dot3(normal, fwd);
        if facing <= 0.0 {
            continue;
        }

        let diffuse = dot3(normal, light_dir).max(0.0);
        let shade = (0.35 + diffuse * 0.65).min(1.0);

        let is_hovered = if pi < 6 {
            matches!(hover, Some(HoverTarget::Face(fi)) if fi == pi)
        } else if pi < 14 {
            matches!(hover, Some(HoverTarget::Corner(ci)) if ci == pi - 6)
        } else {
            matches!(hover, Some(HoverTarget::Edge(ei)) if ei == pi - 14)
        };

        let (br, bg, bb) = if is_hovered {
            (75.0, 88.0, 115.0)
        } else if pi < 6 {
            (45.0, 50.0, 65.0)
        } else if pi < 14 {
            (38.0, 42.0, 55.0)
        } else {
            (40.0, 45.0, 58.0)
        };
        let r = (br * shade + 22.0).min(255.0) as u8;
        let g = (bg * shade + 22.0).min(255.0) as u8;
        let b = (bb * shade + 24.0).min(255.0) as u8;
        let alpha = (255.0 * nav.cube_opacity) as u8;
        let fill = egui::Color32::from_rgba_premultiplied(
            (r as u16 * alpha as u16 / 255) as u8,
            (g as u16 * alpha as u16 / 255) as u8,
            (b as u16 * alpha as u16 / 255) as u8,
            alpha,
        );

        if is_hovered {
            hovered_poly = Some((pts.clone(), fill));
        } else {
            let base = cube_mesh.vertices.len() as u32;
            for &p in &pts {
                cube_mesh.vertices.push(egui::epaint::Vertex {
                    pos: p,
                    uv: egui::epaint::WHITE_UV,
                    color: fill,
                });
            }
            for i in 1..pts.len() as u32 - 1 {
                cube_mesh.indices.push(base);
                cube_mesh.indices.push(base + i);
                cube_mesh.indices.push(base + i + 1);
            }
        }

        if pi < 6 {
            let n_pts = pts.len() as f32;
            let cx = pts.iter().map(|p| p.x).sum::<f32>() / n_pts;
            let cy = pts.iter().map(|p| p.y).sum::<f32>() / n_pts;
            face_labels.push(FaceLabelInfo {
                cx,
                cy,
                pi,
                facing,
                is_hovered,
            });
        }
    }

    painter.add(egui::Shape::mesh(cube_mesh));

    if let Some((pts, fill)) = hovered_poly {
        painter.add(egui::Shape::convex_polygon(
            pts,
            fill,
            egui::Stroke::new(
                2.0,
                egui::Color32::from_rgba_premultiplied(130, 155, 210, 255),
            ),
        ));
    }

    // Face labels
    for lbl in &face_labels {
        let font_size = 9.0 + lbl.facing * 3.0;
        let alpha = ((lbl.facing * 255.0) as u8).max(80);
        let tc = if lbl.is_hovered {
            egui::Color32::from_rgba_premultiplied(255, 255, 255, alpha)
        } else {
            egui::Color32::from_rgba_premultiplied(190, 195, 210, alpha)
        };
        let tr = FACE_TEXT_RIGHT[lbl.pi];
        let sx = dot3(tr, cam_right);
        let sy = -dot3(tr, cam_up);
        let angle = sy.atan2(sx);
        let font_id = egui::FontId::proportional(font_size);
        let galley = painter.layout_no_wrap(FACE_LABELS[lbl.pi].to_string(), font_id, tc);
        let gw = galley.rect.width();
        let gh = galley.rect.height();
        let text_pos = egui::pos2(
            lbl.cx - (gw * angle.cos() - gh * angle.sin()) * 0.5,
            lbl.cy - (gw * angle.sin() + gh * angle.cos()) * 0.5,
        );
        let mut ts = egui::epaint::TextShape::new(text_pos, galley, tc);
        ts.angle = angle;
        painter.add(ts);
    }

    // XYZ axis indicator
    let axis_len = cube_half * 0.7;
    let cube_axes: [([f32; 3], egui::Color32, &str); 3] = [
        ([1.0, 0.0, 0.0], egui::Color32::from_rgb(220, 60, 60), "X"),
        ([0.0, 1.0, 0.0], egui::Color32::from_rgb(60, 200, 60), "Y"),
        ([0.0, 0.0, 1.0], egui::Color32::from_rgb(70, 90, 230), "Z"),
    ];
    for (axis, color, label) in &cube_axes {
        let sx = dot3(*axis, cam_right) * axis_len;
        let sy = -dot3(*axis, cam_up) * axis_len;
        let end = egui::pos2(center.x + sx, center.y + sy);
        let neg = egui::pos2(center.x - sx * 0.3, center.y - sy * 0.3);
        let faded =
            egui::Color32::from_rgba_premultiplied(color.r() / 3, color.g() / 3, color.b() / 3, 60);
        painter.line_segment([center, neg], egui::Stroke::new(1.0, faded));
        painter.line_segment([center, end], egui::Stroke::new(2.0, *color));
        painter.text(
            end,
            egui::Align2::CENTER_CENTER,
            *label,
            egui::FontId::proportional(8.0),
            *color,
        );
    }

    // Handle click
    if clicked {
        if let Some(target) = hover {
            match target {
                HoverTarget::Face(fi) => {
                    actions.push(GuiAction::SetStandardView(FACE_VIEWS[fi]));
                }
                HoverTarget::Edge(ei) => {
                    let [yaw, pitch] = EDGE_YAW_PITCH[ei];
                    actions.push(GuiAction::SetCameraYawPitch(yaw, pitch));
                }
                HoverTarget::Corner(ci) => {
                    let [yaw, pitch] = CORNER_YAW_PITCH[ci];
                    actions.push(GuiAction::SetCameraYawPitch(yaw, pitch));
                }
            }
        }
    }

    // Arrow buttons
    let arrow_r = 10.0;
    let btn_base = egui::Color32::from_rgba_premultiplied(50, 52, 62, 170);
    let btn_hover_c = egui::Color32::from_rgba_premultiplied(80, 90, 115, 230);
    let btn_border = egui::Stroke::new(
        0.8,
        egui::Color32::from_rgba_premultiplied(90, 92, 100, 140),
    );
    let orbit_step = std::f32::consts::TAU / nav.orbit_steps.max(1) as f32;

    let arrow_dist = ring_r + 14.0;
    struct ArrowBtn {
        dx: f32,
        dy: f32,
        symbol: &'static str,
        screen_right: f32,
        screen_up: f32,
    }
    let arrows = [
        ArrowBtn {
            dx: 0.0,
            dy: -arrow_dist,
            symbol: "\u{25B2}",
            screen_right: 0.0,
            screen_up: orbit_step,
        },
        ArrowBtn {
            dx: 0.0,
            dy: arrow_dist,
            symbol: "\u{25BC}",
            screen_right: 0.0,
            screen_up: -orbit_step,
        },
        ArrowBtn {
            dx: -arrow_dist,
            dy: 0.0,
            symbol: "\u{25C0}",
            screen_right: -orbit_step,
            screen_up: 0.0,
        },
        ArrowBtn {
            dx: arrow_dist,
            dy: 0.0,
            symbol: "\u{25B6}",
            screen_right: orbit_step,
            screen_up: 0.0,
        },
    ];
    for btn in &arrows {
        let bpos = egui::pos2(center.x + btn.dx, center.y + btn.dy);
        let is_hov = mouse_pos
            .map(|mp| (mp - bpos).length_sq() < arrow_r * arrow_r)
            .unwrap_or(false);
        painter.circle_filled(bpos, arrow_r, if is_hov { btn_hover_c } else { btn_base });
        painter.circle_stroke(bpos, arrow_r, btn_border);
        painter.text(
            bpos,
            egui::Align2::CENTER_CENTER,
            btn.symbol,
            egui::FontId::proportional(9.0),
            egui::Color32::from_gray(if is_hov { 240 } else { 190 }),
        );
        if is_hov && clicked {
            actions.push(GuiAction::ScreenOrbit(btn.screen_right, btn.screen_up));
        }
    }

    // CW/CCW roll buttons
    let rot_y = center.y + arrow_dist + 22.0;
    let rot_sp = 22.0;
    for (dx_sign, symbol, roll_delta) in [
        (-0.5f32, "\u{21BA}", -orbit_step),
        (0.5, "\u{21BB}", orbit_step),
    ] {
        let bpos = egui::pos2(center.x + rot_sp * dx_sign, rot_y);
        let is_hov = mouse_pos
            .map(|mp| (mp - bpos).length_sq() < arrow_r * arrow_r)
            .unwrap_or(false);
        painter.circle_filled(bpos, arrow_r, if is_hov { btn_hover_c } else { btn_base });
        painter.circle_stroke(bpos, arrow_r, btn_border);
        painter.text(
            bpos,
            egui::Align2::CENTER_CENTER,
            symbol,
            egui::FontId::proportional(13.0),
            egui::Color32::from_gray(if is_hov { 240 } else { 190 }),
        );
        if is_hov && clicked {
            actions.push(GuiAction::RollDelta(roll_delta));
        }
    }

    // Side buttons (Home / Projection / Menu)
    let side_r = 10.0;
    let side_x = center.x + arrow_dist + 18.0;
    struct SideBtn {
        y_off: f32,
        symbol: &'static str,
        font: f32,
    }
    let side_btns = [
        SideBtn {
            y_off: -24.0,
            symbol: "\u{2302}",
            font: 13.0,
        },
        SideBtn {
            y_off: 0.0,
            symbol: "",
            font: 11.0,
        },
        SideBtn {
            y_off: 24.0,
            symbol: "\u{2630}",
            font: 12.0,
        },
    ];
    for (si, sb) in side_btns.iter().enumerate() {
        let bpos = egui::pos2(side_x, center.y + sb.y_off);
        let is_hov = mouse_pos
            .map(|mp| (mp - bpos).length_sq() < side_r * side_r)
            .unwrap_or(false);
        painter.circle_filled(bpos, side_r, if is_hov { btn_hover_c } else { btn_base });
        painter.circle_stroke(bpos, side_r, btn_border);
        let sym = if si == 1 {
            match camera.projection {
                Projection::Perspective => "P",
                Projection::Orthographic => "O",
            }
        } else {
            sb.symbol
        };
        painter.text(
            bpos,
            egui::Align2::CENTER_CENTER,
            sym,
            egui::FontId::proportional(sb.font),
            egui::Color32::from_gray(if is_hov { 240 } else { 185 }),
        );
        if is_hov && clicked {
            match si {
                0 => actions.push(GuiAction::SetStandardView(StandardView::Isometric)),
                1 => actions.push(GuiAction::ToggleProjection),
                _ => gui.show_view_menu = !gui.show_view_menu,
            }
        }
    }

    // View dropdown menu
    if gui.show_view_menu {
        let menu_x = side_x - side_r;
        let menu_y = center.y + side_btns[2].y_off + side_r + 4.0;
        let menu_id = egui::Id::new("viewcube_menu");
        let mut close = false;
        egui::Area::new(menu_id)
            .fixed_pos(egui::pos2(menu_x, menu_y))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(140.0);
                    let proj_label = match camera.projection {
                        Projection::Orthographic => "Orthographic  \u{2714}",
                        Projection::Perspective => "Orthographic",
                    };
                    if ui.button(proj_label).clicked() {
                        if matches!(camera.projection, Projection::Perspective) {
                            actions.push(GuiAction::ToggleProjection);
                        }
                        close = true;
                    }
                    let persp_label = match camera.projection {
                        Projection::Perspective => "Perspective  \u{2714}",
                        Projection::Orthographic => "Perspective",
                    };
                    if ui.button(persp_label).clicked() {
                        if matches!(camera.projection, Projection::Orthographic) {
                            actions.push(GuiAction::ToggleProjection);
                        }
                        close = true;
                    }
                    ui.separator();
                    if ui.button("Isometric").clicked() {
                        actions.push(GuiAction::SetStandardView(StandardView::Isometric));
                        close = true;
                    }
                    ui.separator();
                    if ui.button("Fit All").clicked() {
                        actions.push(GuiAction::FitAll);
                        close = true;
                    }
                });
            });
        if close {
            gui.show_view_menu = false;
        }
        if clicked && !close {
            let menu_rect =
                egui::Rect::from_min_size(egui::pos2(menu_x, menu_y), egui::vec2(150.0, 100.0));
            if let Some(mp) = mouse_pos {
                if !menu_rect.contains(mp) {
                    gui.show_view_menu = false;
                }
            }
        }
    }
}

fn point_in_convex_poly(p: egui::Pos2, poly: &[egui::Pos2]) -> bool {
    if poly.len() < 3 {
        return false;
    }
    let mut sign = 0i32;
    let n = poly.len();
    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + 1) % n];
        let cross = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
        let s = if cross > 0.0 {
            1
        } else if cross < 0.0 {
            -1
        } else {
            0
        };
        if s != 0 {
            if sign == 0 {
                sign = s;
            } else if sign != s {
                return false;
            }
        }
    }
    true
}
