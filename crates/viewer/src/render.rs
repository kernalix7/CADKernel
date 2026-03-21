//! Low-level wgpu rendering primitives shared by both the simple viewer and the
//! full GUI application.

use cadkernel_io::Mesh;
use rayon::prelude::*;
use std::ops::Range;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::window::Window;

// ---------------------------------------------------------------------------
// Vertex
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    pub(crate) fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Projection {
    Perspective,
    Orthographic,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DisplayMode {
    AsIs,
    Points,
    Wireframe,
    HiddenLine,
    NoShading,
    Shading,
    FlatLines,
    Transparent,
}

impl DisplayMode {
    pub const ALL: &[DisplayMode] = &[
        DisplayMode::AsIs,
        DisplayMode::Points,
        DisplayMode::Wireframe,
        DisplayMode::HiddenLine,
        DisplayMode::NoShading,
        DisplayMode::Shading,
        DisplayMode::FlatLines,
        DisplayMode::Transparent,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::AsIs => "As Is",
            Self::Points => "Points",
            Self::Wireframe => "Wireframe",
            Self::HiddenLine => "Hidden Line",
            Self::NoShading => "No Shading",
            Self::Shading => "Shading",
            Self::FlatLines => "Flat Lines",
            Self::Transparent => "Transparent",
        }
    }

    pub fn shortcut(self) -> &'static str {
        match self {
            Self::AsIs => "V, 1",
            Self::Points => "V, 2",
            Self::Wireframe => "V, 3",
            Self::HiddenLine => "V, 4",
            Self::NoShading => "V, 5",
            Self::Shading => "V, 6",
            Self::FlatLines => "V, 7",
            Self::Transparent => "V, 8",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StandardView {
    Front,
    Back,
    Right,
    Left,
    Top,
    Bottom,
    Isometric,
}

impl StandardView {
    pub fn yaw_pitch(self) -> (f32, f32) {
        use std::f32::consts::*;
        match self {
            Self::Front => (FRAC_PI_2, 0.0),
            Self::Back => (-FRAC_PI_2, 0.0),
            Self::Right => (0.0, 0.0),
            Self::Left => (PI, 0.0),
            Self::Top => (FRAC_PI_2, FRAC_PI_2 - 0.001),
            Self::Bottom => (FRAC_PI_2, -(FRAC_PI_2 - 0.001)),
            Self::Isometric => (0.8, 0.4),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Front => "Front",
            Self::Back => "Back",
            Self::Right => "Right",
            Self::Left => "Left",
            Self::Top => "Top",
            Self::Bottom => "Bottom",
            Self::Isometric => "Isometric",
        }
    }
}

// ---------------------------------------------------------------------------
// Camera
// ---------------------------------------------------------------------------

pub struct Camera {
    pub target: [f32; 3],
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    /// In-plane roll angle (positive = CW as seen by user).
    pub roll: f32,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
    pub projection: Projection,
}

impl Camera {
    pub fn new(aspect: f32) -> Self {
        Self {
            target: [0.0, 0.0, 0.0],
            distance: 20.0,
            yaw: 0.8,
            pitch: 0.4,
            roll: 0.0,
            aspect,
            fovy: std::f32::consts::FRAC_PI_4,
            znear: 0.01,
            zfar: 10000.0,
            projection: Projection::Perspective,
        }
    }

    pub fn toggle_projection(&mut self) {
        self.projection = match self.projection {
            Projection::Perspective => Projection::Orthographic,
            Projection::Orthographic => Projection::Perspective,
        };
    }

    pub fn snap_to_view(&mut self, view: StandardView) {
        let (yaw, pitch) = view.yaw_pitch();
        self.yaw = yaw;
        self.pitch = pitch;
    }

    pub fn eye(&self) -> [f32; 3] {
        [
            self.target[0] + self.distance * self.yaw.cos() * self.pitch.cos(),
            self.target[1] + self.distance * self.yaw.sin() * self.pitch.cos(),
            self.target[2] + self.distance * self.pitch.sin(),
        ]
    }

    pub fn view_matrix(&self) -> [[f32; 4]; 4] {
        let up_z = if self.pitch.cos() >= 0.0 { 1.0 } else { -1.0 };
        let world_up = [0.0, 0.0, up_z];
        if self.roll.abs() < 1e-6 {
            return look_at(self.eye(), self.target, world_up);
        }
        // Compute base right/up, then rotate by roll around the forward axis.
        let f = normalize3(sub3(self.target, self.eye()));
        let r = normalize3(cross3(f, world_up));
        let u = cross3(r, f);
        let (sr, cr) = self.roll.sin_cos();
        let rolled_up = [
            -r[0] * sr + u[0] * cr,
            -r[1] * sr + u[1] * cr,
            -r[2] * sr + u[2] * cr,
        ];
        look_at(self.eye(), self.target, rolled_up)
    }

    pub fn projection_matrix(&self) -> [[f32; 4]; 4] {
        match self.projection {
            Projection::Perspective => perspective(self.fovy, self.aspect, self.znear, self.zfar),
            Projection::Orthographic => {
                let half_h = self.distance * (self.fovy * 0.5).tan();
                let half_w = half_h * self.aspect;
                orthographic(-half_w, half_w, -half_h, half_h, self.znear, self.zfar)
            }
        }
    }

    pub fn view_proj(&self) -> [[f32; 4]; 4] {
        mat4_mul(self.projection_matrix(), self.view_matrix())
    }

    pub fn inv_view_proj(&self) -> [[f32; 4]; 4] {
        mat4_inv(self.view_proj())
    }

    pub fn fit_to_bounds(&mut self, min: [f32; 3], max: [f32; 3]) {
        self.target = [
            (min[0] + max[0]) * 0.5,
            (min[1] + max[1]) * 0.5,
            (min[2] + max[2]) * 0.5,
        ];
        let dx = max[0] - min[0];
        let dy = max[1] - min[1];
        let dz = max[2] - min[2];
        let extent = (dx * dx + dy * dy + dz * dz).sqrt();
        self.distance = extent.max(0.1) * 1.5;
    }

    pub fn reset(&mut self) {
        self.target = [0.0; 3];
        self.distance = 20.0;
        self.yaw = 0.8;
        self.pitch = 0.4;
        self.roll = 0.0;
        self.projection = Projection::Perspective;
    }

    pub fn screen_right(&self) -> [f32; 3] {
        let f = normalize3(sub3(self.target, self.eye()));
        let up_z = if self.pitch.cos() >= 0.0 { 1.0 } else { -1.0 };
        let r = normalize3(cross3(f, [0.0, 0.0, up_z]));
        let u = cross3(r, f);
        if self.roll.abs() < 1e-6 {
            return r;
        }
        let (sr, cr) = self.roll.sin_cos();
        normalize3([
            r[0] * cr + u[0] * sr,
            r[1] * cr + u[1] * sr,
            r[2] * cr + u[2] * sr,
        ])
    }

    pub fn screen_up(&self) -> [f32; 3] {
        let f = normalize3(sub3(self.target, self.eye()));
        let up_z = if self.pitch.cos() >= 0.0 { 1.0 } else { -1.0 };
        let r = normalize3(cross3(f, [0.0, 0.0, up_z]));
        let u = cross3(r, f);
        if self.roll.abs() < 1e-6 {
            return u;
        }
        let (sr, cr) = self.roll.sin_cos();
        normalize3([
            -r[0] * sr + u[0] * cr,
            -r[1] * sr + u[1] * cr,
            -r[2] * sr + u[2] * cr,
        ])
    }
}

// ---------------------------------------------------------------------------
// Uniforms  (with dynamic-offset slot support)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub view_proj: [[f32; 4]; 4],
    pub light_dir: [f32; 4],
    pub base_color: [f32; 4],
    /// x = use_lighting, y = specular strength, z = shininess
    pub params: [f32; 4],
    /// Camera eye position in world space (xyz, w unused).
    pub eye_pos: [f32; 4],
}

const UNIFORM_ALIGN: u64 = 256;
const MAX_UNIFORM_SLOTS: u64 = 64;

fn uniform_stride() -> u64 {
    let raw = std::mem::size_of::<Uniforms>() as u64;
    raw.div_ceil(UNIFORM_ALIGN) * UNIFORM_ALIGN
}

// ---------------------------------------------------------------------------
// Linear-algebra helpers  (pub(crate) for use in gui axes overlay)
// ---------------------------------------------------------------------------

pub(crate) fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub(crate) fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub(crate) fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub(crate) fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let len = dot3(v, v).sqrt();
    if len < 1e-10 {
        return [0.0, 0.0, 0.0];
    }
    [v[0] / len, v[1] / len, v[2] / len]
}

fn look_at(eye: [f32; 3], target: [f32; 3], up: [f32; 3]) -> [[f32; 4]; 4] {
    let f = normalize3(sub3(target, eye));
    let s = normalize3(cross3(f, up));
    let u = cross3(s, f);
    [
        [s[0], u[0], -f[0], 0.0],
        [s[1], u[1], -f[1], 0.0],
        [s[2], u[2], -f[2], 0.0],
        [-dot3(s, eye), -dot3(u, eye), dot3(f, eye), 1.0],
    ]
}

fn perspective(fovy: f32, aspect: f32, znear: f32, zfar: f32) -> [[f32; 4]; 4] {
    let f = 1.0 / (fovy * 0.5).tan();
    let range_inv = 1.0 / (znear - zfar);
    [
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, zfar * range_inv, -1.0],
        [0.0, 0.0, znear * zfar * range_inv, 0.0],
    ]
}

fn orthographic(
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    znear: f32,
    zfar: f32,
) -> [[f32; 4]; 4] {
    let rml = right - left;
    let tmb = top - bottom;
    let fmn = zfar - znear;
    [
        [2.0 / rml, 0.0, 0.0, 0.0],
        [0.0, 2.0 / tmb, 0.0, 0.0],
        [0.0, 0.0, -1.0 / fmn, 0.0],
        [
            -(right + left) / rml,
            -(top + bottom) / tmb,
            -znear / fmn,
            1.0,
        ],
    ]
}

/// 4x4 matrix inverse via cofactor expansion.
fn mat4_inv(m: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let m = |r: usize, c: usize| m[c][r]; // column-major access
    let cf = |r0: usize, r1: usize, c0: usize, c1: usize| -> f32 {
        m(r0, c0) * m(r1, c1) - m(r0, c1) * m(r1, c0)
    };
    let (s0, s1, s2, s3, s4, s5) = (
        cf(0, 1, 0, 1), cf(0, 1, 0, 2), cf(0, 1, 0, 3),
        cf(0, 1, 1, 2), cf(0, 1, 1, 3), cf(0, 1, 2, 3),
    );
    let (c5, c4, c3, c2, c1, c0) = (
        cf(2, 3, 0, 1), cf(2, 3, 0, 2), cf(2, 3, 0, 3),
        cf(2, 3, 1, 2), cf(2, 3, 1, 3), cf(2, 3, 2, 3),
    );
    let det = s0 * c5 - s1 * c4 + s2 * c3 + s3 * c2 - s4 * c1 + s5 * c0;
    if det.abs() < 1e-10 {
        return [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
    }
    let inv_det = 1.0 / det;
    let mut out = [[0.0f32; 4]; 4];
    out[0][0] = ( m(1,1) * c5 - m(1,2) * c4 + m(1,3) * c3) * inv_det;
    out[1][0] = (-m(1,0) * c5 + m(1,2) * c2 - m(1,3) * c1) * inv_det;
    out[2][0] = ( m(1,0) * c4 - m(1,1) * c2 + m(1,3) * c0) * inv_det;
    out[3][0] = (-m(1,0) * c3 + m(1,1) * c1 - m(1,2) * c0) * inv_det;
    out[0][1] = (-m(0,1) * c5 + m(0,2) * c4 - m(0,3) * c3) * inv_det;
    out[1][1] = ( m(0,0) * c5 - m(0,2) * c2 + m(0,3) * c1) * inv_det;
    out[2][1] = (-m(0,0) * c4 + m(0,1) * c2 - m(0,3) * c0) * inv_det;
    out[3][1] = ( m(0,0) * c3 - m(0,1) * c1 + m(0,2) * c0) * inv_det;
    out[0][2] = ( m(0,1) * s5 - m(0,2) * s4 + m(0,3) * s3) * inv_det;
    out[1][2] = (-m(0,0) * s5 + m(0,2) * s2 - m(0,3) * s1) * inv_det;
    out[2][2] = ( m(0,0) * s4 - m(0,1) * s2 + m(0,3) * s0) * inv_det;
    out[3][2] = (-m(0,0) * s3 + m(0,1) * s1 - m(0,2) * s0) * inv_det;
    out[0][3] = (-m(3,1) * s5 + m(3,2) * s4 - m(3,3) * s3) * inv_det;
    out[1][3] = ( m(3,0) * s5 - m(3,2) * s2 + m(3,3) * s1) * inv_det;
    out[2][3] = (-m(3,0) * s4 + m(3,1) * s2 - m(3,3) * s0) * inv_det;
    out[3][3] = ( m(3,0) * s3 - m(3,1) * s1 + m(3,2) * s0) * inv_det;
    out
}

fn mat4_mul(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0f32; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            out[i][j] = (0..4).map(|k| a[k][j] * b[i][k]).sum();
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Mesh → GPU vertex conversion
// ---------------------------------------------------------------------------

/// Auto-smooth crease angle (60°). Faces within this angle are smoothed;
/// faces beyond get sharp edges. Higher than Blender default (30°) to handle
/// coarse imported STL meshes where adjacent faces often exceed 30°.
const SMOOTH_ANGLE_DEG: f32 = 60.0;

pub fn mesh_to_vertices(mesh: &Mesh) -> Vec<Vertex> {
    let nv = mesh.vertices.len();
    let nf = mesh.indices.len();

    let positions: Vec<[f32; 3]> = mesh
        .vertices
        .par_iter()
        .map(|p| [p.x as f32, p.y as f32, p.z as f32])
        .collect();

    // Per-face: unit normals (for BFS angle comparison) and raw cross products
    // (for area-weighted accumulation). Raw cross product magnitude = 2×area,
    // so summing them gives area-weighted normals — large triangles contribute
    // proportionally more, eliminating artifacts from non-uniform mesh density.
    let face_data: Vec<([f32; 3], [f32; 3])> = mesh
        .indices
        .par_iter()
        .map(|idx| {
            let raw = face_cross_product(&positions, idx);
            let len = (raw[0] * raw[0] + raw[1] * raw[1] + raw[2] * raw[2]).sqrt();
            let unit = if len > 1e-8 {
                [raw[0] / len, raw[1] / len, raw[2] / len]
            } else {
                [0.0, 0.0, 1.0]
            };
            (unit, raw)
        })
        .collect();

    // Build vertex → face adjacency.
    let mut vert_faces: Vec<Vec<usize>> = vec![Vec::new(); nv];
    for (fi, idx) in mesh.indices.iter().enumerate() {
        for &vi in idx {
            let v = vi as usize;
            if v < nv {
                vert_faces[v].push(fi);
            }
        }
    }

    let cos_thresh = (SMOOTH_ANGLE_DEG * std::f32::consts::PI / 180.0).cos();

    // Smooth-group normals via BFS at each vertex.
    // Faces transitively connected within the crease angle share the EXACT
    // same area-weighted averaged normal → zero discontinuity on smooth surfaces
    // while preserving sharp edges (text engravings, chamfers, etc.).
    let mut corner_normals: Vec<[f32; 3]> = vec![[0.0; 3]; nf * 3];

    for (v, faces) in vert_faces.iter().enumerate() {
        let n = faces.len();
        if n == 0 {
            continue;
        }

        let mut visited = vec![false; n];

        for start in 0..n {
            if visited[start] {
                continue;
            }

            // BFS: group faces connected by smooth angle at this vertex.
            // Build local adjacency: two faces at this vertex are neighbors
            // if they share another vertex (i.e. share an edge through v).
            let mut local_adj: Vec<Vec<usize>> = vec![Vec::new(); n];
            for a in 0..n {
                let fa = &mesh.indices[faces[a]];
                for b in (a + 1)..n {
                    let fb = &mesh.indices[faces[b]];
                    let shared = fa.iter().any(|&va| va as usize != v && fb.contains(&va));
                    if shared {
                        local_adj[a].push(b);
                        local_adj[b].push(a);
                    }
                }
            }

            let mut group = vec![start];
            visited[start] = true;
            let mut qi = 0;
            while qi < group.len() {
                let fn_c = face_data[faces[group[qi]]].0;
                qi += 1;
                for &next in &local_adj[group[qi - 1]] {
                    if visited[next] {
                        continue;
                    }
                    let fn_n = face_data[faces[next]].0;
                    let d = fn_c[0] * fn_n[0] + fn_c[1] * fn_n[1] + fn_c[2] * fn_n[2];
                    if d >= cos_thresh {
                        visited[next] = true;
                        group.push(next);
                    }
                }
            }

            // Area-weighted average: sum raw cross products (magnitude ∝ area).
            let mut acc = [0.0f32; 3];
            for &gi in &group {
                let raw = face_data[faces[gi]].1;
                acc[0] += raw[0];
                acc[1] += raw[1];
                acc[2] += raw[2];
            }
            let len = (acc[0] * acc[0] + acc[1] * acc[1] + acc[2] * acc[2]).sqrt();
            let normal = if len > 1e-8 {
                [acc[0] / len, acc[1] / len, acc[2] / len]
            } else {
                face_data[faces[group[0]]].0
            };

            // Assign to all face corners in this group at vertex v.
            for &gi in &group {
                let fi = faces[gi];
                let idx_arr = mesh.indices[fi];
                for k in 0..3 {
                    if idx_arr[k] as usize == v {
                        corner_normals[fi * 3 + k] = normal;
                    }
                }
            }
        }
    }

    // Output: one Vertex per face corner.
    let cn_ref = &corner_normals;
    let pos_ref = &positions;
    mesh.indices
        .par_iter()
        .enumerate()
        .flat_map_iter(|(fi, idx)| {
            (0..3).map(move |k| Vertex {
                position: pos_ref[idx[k] as usize],
                normal: cn_ref[fi * 3 + k],
            })
        })
        .collect()
}

/// Returns the raw (unnormalized) cross product `(B−A) × (C−A)`.
/// Its magnitude equals 2× the triangle area — used for area-weighted
/// normal accumulation so large triangles contribute proportionally more.
fn face_cross_product(positions: &[[f32; 3]], idx: &[u32; 3]) -> [f32; 3] {
    let a = positions[idx[0] as usize];
    let b = positions[idx[1] as usize];
    let c = positions[idx[2] as usize];
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ]
}

pub fn compute_bounds(vertices: &[Vertex]) -> ([f32; 3], [f32; 3]) {
    vertices
        .par_iter()
        .fold(
            || ([f32::MAX; 3], [f32::MIN; 3]),
            |(mut mn, mut mx), v| {
                for i in 0..3 {
                    mn[i] = mn[i].min(v.position[i]);
                    mx[i] = mx[i].max(v.position[i]);
                }
                (mn, mx)
            },
        )
        .reduce(
            || ([f32::MAX; 3], [f32::MIN; 3]),
            |(mn1, mx1), (mn2, mx2)| {
                let mn = [mn1[0].min(mn2[0]), mn1[1].min(mn2[1]), mn1[2].min(mn2[2])];
                let mx = [mx1[0].max(mx2[0]), mx1[1].max(mx2[1]), mx1[2].max(mx2[2])];
                (mn, mx)
            },
        )
}

// ---------------------------------------------------------------------------
// WGSL shaders
// ---------------------------------------------------------------------------

pub(crate) const SHADER_SRC: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
    base_color: vec4<f32>,
    params: vec4<f32>,
    eye_pos: vec4<f32>,
}
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) world_pos: vec3<f32>,
}

@vertex
fn vs_main(vin: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.view_proj * vec4<f32>(vin.position, 1.0);
    out.world_normal = vin.normal;
    out.world_pos = vin.position;
    return out;
}

@fragment
fn fs_main(fin: VertexOutput) -> @location(0) vec4<f32> {
    var color = uniforms.base_color.rgb;
    if (uniforms.params.x > 0.5) {
        let light = normalize(uniforms.light_dir.xyz);
        let normal = normalize(fin.world_normal);

        // Diffuse (Lambert)
        let n_dot_l = max(dot(normal, light), 0.0);
        let ambient = 0.15;
        color = color * (ambient + n_dot_l * 0.85);

        // Specular (Blinn-Phong)
        let spec_strength = uniforms.params.y;
        if (spec_strength > 0.0) {
            let view_dir = normalize(uniforms.eye_pos.xyz - fin.world_pos);
            let half_dir = normalize(light + view_dir);
            let shininess = uniforms.params.z;
            let spec = pow(max(dot(normal, half_dir), 0.0), shininess);
            color = color + vec3<f32>(spec * spec_strength);
        }
    }
    return vec4<f32>(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), uniforms.base_color.a);
}
"#;

pub(crate) const GRADIENT_SHADER_SRC: &str = r#"
struct GradientOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_gradient(@builtin(vertex_index) idx: u32) -> GradientOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    var out: GradientOutput;
    out.pos = vec4<f32>(positions[idx], 0.999, 1.0);
    out.uv = positions[idx] * 0.5 + 0.5;
    return out;
}

@fragment
fn fs_gradient(fin: GradientOutput) -> @location(0) vec4<f32> {
    let top    = vec3<f32>(0.16, 0.17, 0.20);
    let bottom = vec3<f32>(0.08, 0.08, 0.10);
    let color  = mix(bottom, top, fin.uv.y);
    return vec4<f32>(color, 1.0);
}
"#;

// ---------------------------------------------------------------------------
// Grid overlay (dynamic – adapts to camera distance)
// ---------------------------------------------------------------------------

pub(crate) struct GridOverlay {
    pub buffer: wgpu::Buffer,
    pub minor_range: Range<u32>,
    pub major_range: Range<u32>,
    pub axis_x_range: Range<u32>,
    pub axis_y_range: Range<u32>,
    pub axis_z_range: Range<u32>,
}

pub(crate) struct GridConfig {
    pub minor_step: f32,
    pub major_step: f32,
    pub half_extent: f32,
    level: i32,
    /// Bounding extent of the loaded object (0 = no object).
    obj_extent: f32,
}

/// Pre-defined "nice" spacing levels (1-2-5 sequence across decades).
const GRID_LEVELS: &[f32] = &[
    0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0,
    2000.0, 5000.0,
];

impl GridConfig {
    pub fn new() -> Self {
        Self {
            minor_step: 1.0,
            major_step: 10.0,
            half_extent: 30.0,
            level: 0,
            obj_extent: 0.0,
        }
    }

    /// Set object bounding extent so the grid can grow to accommodate it.
    pub fn set_object_extent(&mut self, extent: f32) {
        self.obj_extent = extent;
    }

    /// Recompute grid spacing for the given camera distance.
    /// Fusion 360-style: conservative extent, discrete 1-2-5 levels,
    /// clear minor/major hierarchy.
    /// Returns `true` when the grid buffer must be rebuilt.
    pub fn update_for_camera(&mut self, distance: f32) -> bool {
        // Visible span estimate (slightly less than full viewport).
        let visible = distance * 2.0;

        // Target ~12-18 minor cells across the visible area — conservative.
        let target_cells = 15.0f32;
        let ideal_step = (visible / target_cells).max(1e-6);

        // Snap to nearest 1-2-5 level (with hysteresis: prefer current level).
        let new_level = snap_to_level(ideal_step);
        let new_minor = GRID_LEVELS[new_level];

        // --- Extent: conservative, object-aware ---
        // Base extent: enough for the object or a sensible default.
        let base = if self.obj_extent > 0.1 {
            self.obj_extent * 1.5
        } else {
            30.0
        };
        let major = new_minor * 10.0;
        // At least 5 major cells, but don't exceed what's useful.
        let min_half = major * 5.0;
        // Cap at 60% of visible area — grid should NOT fill the screen.
        let max_visible = visible * 0.6;
        let new_half_raw = base.max(min_half).min(max_visible).max(min_half);
        // Snap to major-step multiples for clean edges.
        let new_half = (new_half_raw / major).ceil() * major;

        let level_changed = new_level as i32 != self.level;
        let half_changed = (new_half - self.half_extent).abs() > self.half_extent * 0.3;

        if !level_changed && !half_changed {
            return false;
        }

        self.level = new_level as i32;
        self.minor_step = new_minor;
        self.major_step = major;
        self.half_extent = new_half;
        true
    }

    /// Force a rebuild on the next `update_for_camera` call.
    pub fn force_rebuild(&mut self) {
        self.level = i32::MIN;
    }

    /// Human-readable label for the current minor grid spacing.
    pub fn scale_label(&self) -> String {
        let s = self.minor_step;
        if s >= 1000.0 {
            format!("{:.0} m", s / 1000.0)
        } else if s >= 1.0 {
            format!("{:.0} mm", s)
        } else if s >= 0.1 {
            format!("{:.1} mm", s)
        } else {
            format!("{:.2} mm", s)
        }
    }
}

/// Find the closest 1-2-5 level index for a given ideal spacing.
fn snap_to_level(ideal: f32) -> usize {
    let mut best = 0;
    let mut best_ratio = f32::MAX;
    for (i, &lvl) in GRID_LEVELS.iter().enumerate() {
        let ratio = (ideal.ln() - lvl.ln()).abs();
        if ratio < best_ratio {
            best_ratio = ratio;
            best = i;
        }
    }
    best
}

type GridRanges = (
    Vec<Vertex>,
    Range<u32>,
    Range<u32>,
    Range<u32>,
    Range<u32>,
    Range<u32>,
);

fn build_dynamic_grid(cfg: &GridConfig) -> GridRanges {
    let half = cfg.half_extent;
    let n = [0.0f32, 0.0, 1.0]; // Z-up normal
    let mut v: Vec<Vertex> = Vec::new();

    let minor_step = cfg.minor_step;
    let major_step = cfg.major_step;
    let minor_count = (half / minor_step).ceil() as i32;
    let major_i = if minor_step > 0.0 {
        (major_step / minor_step).round() as i32
    } else {
        0
    };

    // Grid on XY plane (Z=0)
    let minor_start = v.len() as u32;
    for i in -minor_count..=minor_count {
        if major_i > 0 && i % major_i == 0 {
            continue;
        }
        let p = i as f32 * minor_step;
        v.push(Vertex {
            position: [p, -half, 0.0],
            normal: n,
        });
        v.push(Vertex {
            position: [p, half, 0.0],
            normal: n,
        });
        v.push(Vertex {
            position: [-half, p, 0.0],
            normal: n,
        });
        v.push(Vertex {
            position: [half, p, 0.0],
            normal: n,
        });
    }
    let minor_end = v.len() as u32;

    let major_count = (half / major_step).ceil() as i32;
    let major_start = v.len() as u32;
    for i in -major_count..=major_count {
        if i == 0 {
            continue;
        }
        let p = i as f32 * major_step;
        v.push(Vertex {
            position: [p, -half, 0.0],
            normal: n,
        });
        v.push(Vertex {
            position: [p, half, 0.0],
            normal: n,
        });
        v.push(Vertex {
            position: [-half, p, 0.0],
            normal: n,
        });
        v.push(Vertex {
            position: [half, p, 0.0],
            normal: n,
        });
    }
    let major_end = v.len() as u32;

    // Axis X (red)
    let ax = v.len() as u32;
    v.push(Vertex {
        position: [-half, 0.0, 0.0],
        normal: n,
    });
    v.push(Vertex {
        position: [half, 0.0, 0.0],
        normal: n,
    });
    // Axis Y (green)
    let ay = v.len() as u32;
    v.push(Vertex {
        position: [0.0, -half, 0.0],
        normal: n,
    });
    v.push(Vertex {
        position: [0.0, half, 0.0],
        normal: n,
    });
    // Axis Z (blue, vertical)
    let az = v.len() as u32;
    v.push(Vertex {
        position: [0.0, 0.0, 0.0],
        normal: n,
    });
    v.push(Vertex {
        position: [0.0, 0.0, half * 0.3],
        normal: n,
    });
    let end = v.len() as u32;

    (
        v,
        minor_start..minor_end,
        major_start..major_end,
        ax..ay,
        ay..az,
        az..end,
    )
}

// ---------------------------------------------------------------------------
// GPU state
// ---------------------------------------------------------------------------

pub(crate) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub(crate) const MSAA_SAMPLES: u32 = 4;

pub(crate) struct GpuState {
    pub window: Arc<Window>,
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub solid_pipeline: wgpu::RenderPipeline,
    pub wire_pipeline: wgpu::RenderPipeline,
    pub transparent_pipeline: wgpu::RenderPipeline,
    pub gradient_pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub edge_index_buffer: wgpu::Buffer,
    pub num_edge_indices: u32,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub num_vertices: u32,
    pub depth_view: wgpu::TextureView,
    pub msaa_view: wgpu::TextureView,
    pub grid: GridOverlay,
}

fn build_edge_indices(num_vertices: u32) -> Vec<u32> {
    let num_tris = num_vertices / 3;
    let mut indices = Vec::with_capacity(num_tris as usize * 6);
    for t in 0..num_tris {
        let b = t * 3;
        indices.extend_from_slice(&[b, b + 1, b + 1, b + 2, b + 2, b]);
    }
    indices
}

impl GpuState {
    pub async fn new(window: Arc<Window>, vertices: &[Vertex]) -> Self {
        // Try all backends (Vulkan/Metal/DX12/GL) with HighPerformance first,
        // then fall back to LowPower, then software rendering (GL backend).
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = Self::pick_adapter(&instance, &surface).await;
        eprintln!(
            "[cadkernel-viewer] GPU adapter: {} ({:?})",
            adapter.get_info().name,
            adapter.get_info().backend,
        );

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .expect("failed to create GPU device");

        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .expect("surface not supported by adapter");
        config.present_mode = wgpu::PresentMode::AutoVsync;
        surface.configure(&device, &config);

        // -- shaders ------------------------------------------------------
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SRC.into()),
        });
        let grad_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gradient_shader"),
            source: wgpu::ShaderSource::Wgsl(GRADIENT_SHADER_SRC.into()),
        });

        // -- vertex / edge buffers ----------------------------------------
        let nv = vertices.len() as u32;
        let vertex_buffer = create_vertex_buffer(&device, vertices);
        let edge_ids = build_edge_indices(nv);
        let edge_index_buffer = create_edge_index_buffer(&device, &edge_ids);
        let num_edge_indices = edge_ids.len() as u32;

        // -- grid ---------------------------------------------------------
        let grid_cfg = GridConfig::new();
        let (grid_verts, g_minor, g_major, g_ax, g_ay, g_az) = build_dynamic_grid(&grid_cfg);
        let grid_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("grid_buffer"),
            contents: bytemuck::cast_slice(&grid_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let grid = GridOverlay {
            buffer: grid_buffer,
            minor_range: g_minor,
            major_range: g_major,
            axis_x_range: g_ax,
            axis_y_range: g_ay,
            axis_z_range: g_az,
        };

        // -- uniform buffer (dynamic offset) ------------------------------
        let stride = uniform_stride();
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniform_buffer"),
            size: stride * MAX_UNIFORM_SLOTS,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_size = std::mem::size_of::<Uniforms>() as u64;
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bind_group_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new(uniform_size),
                },
                count: None,
            }],
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("uniform_bind_group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &uniform_buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new(uniform_size),
                }),
            }],
        });

        // -- pipeline layouts ---------------------------------------------
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let grad_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gradient_pipeline_layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let depth_view = create_depth_texture(&device, &config, MSAA_SAMPLES);
        let msaa_view = create_msaa_texture(&device, &config, MSAA_SAMPLES);

        // -- pipelines ----------------------------------------------------
        let solid_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("solid_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: MSAA_SAMPLES,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let wire_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("wire_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: wgpu::DepthBiasState {
                    constant: -2,
                    slope_scale: -1.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState {
                count: MSAA_SAMPLES,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let transparent_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("transparent_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: MSAA_SAMPLES,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let gradient_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gradient_pipeline"),
            layout: Some(&grad_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &grad_shader,
                entry_point: Some("vs_gradient"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &grad_shader,
                entry_point: Some("fs_gradient"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: MSAA_SAMPLES,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Self {
            window,
            surface,
            device,
            queue,
            config,
            solid_pipeline,
            wire_pipeline,
            transparent_pipeline,
            gradient_pipeline,
            vertex_buffer,
            edge_index_buffer,
            num_edge_indices,
            uniform_buffer,
            uniform_bind_group,
            num_vertices: nv,
            depth_view,
            msaa_view,
            grid,
        }
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>, camera: &mut Camera) {
        let w = size.width.max(1);
        let h = size.height.max(1);
        self.config.width = w;
        self.config.height = h;
        self.surface.configure(&self.device, &self.config);
        self.depth_view = create_depth_texture(&self.device, &self.config, MSAA_SAMPLES);
        self.msaa_view = create_msaa_texture(&self.device, &self.config, MSAA_SAMPLES);
        camera.aspect = w as f32 / h as f32;
    }

    /// Pick the best available GPU adapter with fallback strategy:
    /// 1. HighPerformance (discrete GPU)
    /// 2. LowPower (integrated GPU)
    /// 3. Any available adapter (software / GL fallback)
    async fn pick_adapter(
        instance: &wgpu::Instance,
        surface: &wgpu::Surface<'static>,
    ) -> wgpu::Adapter {
        // Try high-performance first (discrete GPU).
        if let Some(a) = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(surface),
                force_fallback_adapter: false,
            })
            .await
        {
            return a;
        }
        // Try low-power (integrated GPU).
        if let Some(a) = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(surface),
                force_fallback_adapter: false,
            })
            .await
        {
            return a;
        }
        // Force software fallback (llvmpipe / swiftshader / warp).
        instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(surface),
                force_fallback_adapter: true,
            })
            .await
            .expect(
                "no suitable GPU or software adapter found — install Mesa/llvmpipe or swiftshader",
            )
    }

    // Write uniforms to a specific slot in the dynamic-offset buffer.
    pub(crate) fn write_slot(&self, slot: u32, u: &Uniforms) {
        let offset = slot as u64 * uniform_stride();
        self.queue
            .write_buffer(&self.uniform_buffer, offset, bytemuck::bytes_of(u));
    }

    // Dynamic offset for set_bind_group.
    pub(crate) fn slot_offset(slot: u32) -> u32 {
        (slot as u64 * uniform_stride()) as u32
    }

    /// Full render used by the simple `ViewerApp` (no egui).
    pub fn render(&self, camera: &Camera, mode: DisplayMode, show_grid: bool) {
        let vp = camera.view_proj();
        let eye = camera.eye();
        let eye_pos = [eye[0], eye[1], eye[2], 0.0];
        let cam_fwd = normalize3(sub3(camera.target, eye));
        let cam_r = camera.screen_right();
        let cam_u = camera.screen_up();
        let ld = normalize3([
            -cam_fwd[0] + cam_r[0] * 0.3 + cam_u[0] * 0.4,
            -cam_fwd[1] + cam_r[1] * 0.3 + cam_u[1] * 0.4,
            -cam_fwd[2] + cam_r[2] * 0.3 + cam_u[2] * 0.4,
        ]);
        let light = [ld[0], ld[1], ld[2], 0.0];
        let no_light = [0.0f32; 4];
        let lit_params = [1.0f32, 0.3, 64.0, 0.0];
        let unlit_params = [0.0f32; 4];

        // Pre-write all uniform slots before encoding commands.
        let mut slot: u32 = 0;

        // Slot 0..4: grid uniforms
        let grid_minor_slot = slot;
        slot += 1;
        let grid_major_slot = slot;
        slot += 1;
        let grid_ax_slot = slot;
        slot += 1;
        let grid_ay_slot = slot;
        slot += 1;
        let grid_az_slot = slot;
        slot += 1;

        if show_grid {
            self.write_slot(
                grid_minor_slot,
                &Uniforms {
                    view_proj: vp,
                    light_dir: no_light,
                    base_color: GRID_MINOR_COLOR,
                    params: unlit_params,
                    eye_pos,
                },
            );
            self.write_slot(
                grid_major_slot,
                &Uniforms {
                    view_proj: vp,
                    light_dir: no_light,
                    base_color: GRID_MAJOR_COLOR,
                    params: unlit_params,
                    eye_pos,
                },
            );
            self.write_slot(
                grid_ax_slot,
                &Uniforms {
                    view_proj: vp,
                    light_dir: no_light,
                    base_color: AXIS_X_COLOR,
                    params: unlit_params,
                    eye_pos,
                },
            );
            self.write_slot(
                grid_ay_slot,
                &Uniforms {
                    view_proj: vp,
                    light_dir: no_light,
                    base_color: AXIS_Y_COLOR,
                    params: unlit_params,
                    eye_pos,
                },
            );
            self.write_slot(
                grid_az_slot,
                &Uniforms {
                    view_proj: vp,
                    light_dir: no_light,
                    base_color: AXIS_Z_COLOR,
                    params: unlit_params,
                    eye_pos,
                },
            );
        }

        // Slot 5+: mesh uniforms
        let mesh_slot = slot;
        slot += 1;
        let wire_slot = slot;

        match mode {
            DisplayMode::AsIs | DisplayMode::Shading => {
                self.write_slot(
                    mesh_slot,
                    &Uniforms {
                        view_proj: vp,
                        light_dir: light,
                        base_color: SOLID_COLOR,
                        params: lit_params,
                        eye_pos,
                    },
                );
            }
            DisplayMode::Points => {
                self.write_slot(
                    mesh_slot,
                    &Uniforms {
                        view_proj: vp,
                        light_dir: no_light,
                        base_color: POINT_COLOR,
                        params: unlit_params,
                        eye_pos,
                    },
                );
            }
            DisplayMode::Wireframe => {
                self.write_slot(
                    mesh_slot,
                    &Uniforms {
                        view_proj: vp,
                        light_dir: no_light,
                        base_color: WIRE_COLOR,
                        params: unlit_params,
                        eye_pos,
                    },
                );
            }
            DisplayMode::HiddenLine => {
                self.write_slot(
                    mesh_slot,
                    &Uniforms {
                        view_proj: vp,
                        light_dir: no_light,
                        base_color: HIDDEN_LINE_COLOR,
                        params: lit_params,
                        eye_pos,
                    },
                );
                self.write_slot(
                    wire_slot,
                    &Uniforms {
                        view_proj: vp,
                        light_dir: no_light,
                        base_color: EDGE_OVERLAY_COLOR,
                        params: unlit_params,
                        eye_pos,
                    },
                );
            }
            DisplayMode::NoShading => {
                self.write_slot(
                    mesh_slot,
                    &Uniforms {
                        view_proj: vp,
                        light_dir: no_light,
                        base_color: NO_SHADE_COLOR,
                        params: lit_params,
                        eye_pos,
                    },
                );
            }
            DisplayMode::Transparent => {
                self.write_slot(
                    mesh_slot,
                    &Uniforms {
                        view_proj: vp,
                        light_dir: light,
                        base_color: TRANSPARENT_COLOR,
                        params: lit_params,
                        eye_pos,
                    },
                );
            }
            DisplayMode::FlatLines => {
                self.write_slot(
                    mesh_slot,
                    &Uniforms {
                        view_proj: vp,
                        light_dir: light,
                        base_color: SOLID_COLOR,
                        params: lit_params,
                        eye_pos,
                    },
                );
                self.write_slot(
                    wire_slot,
                    &Uniforms {
                        view_proj: vp,
                        light_dir: light,
                        base_color: EDGE_OVERLAY_COLOR,
                        params: unlit_params,
                        eye_pos,
                    },
                );
            }
        }

        // Acquire frame
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            Err(e) => {
                eprintln!("surface error: {e:?}");
                return;
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.msaa_view,
                    resolve_target: Some(&view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.12,
                            g: 0.12,
                            b: 0.16,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            // Background gradient
            pass.set_pipeline(&self.gradient_pipeline);
            pass.draw(0..3, 0..1);

            // Grid
            if show_grid {
                pass.set_pipeline(&self.wire_pipeline);
                pass.set_vertex_buffer(0, self.grid.buffer.slice(..));

                pass.set_bind_group(
                    0,
                    &self.uniform_bind_group,
                    &[Self::slot_offset(grid_minor_slot)],
                );
                if !self.grid.minor_range.is_empty() {
                    pass.draw(self.grid.minor_range.clone(), 0..1);
                }
                pass.set_bind_group(
                    0,
                    &self.uniform_bind_group,
                    &[Self::slot_offset(grid_major_slot)],
                );
                if !self.grid.major_range.is_empty() {
                    pass.draw(self.grid.major_range.clone(), 0..1);
                }
                pass.set_bind_group(
                    0,
                    &self.uniform_bind_group,
                    &[Self::slot_offset(grid_ax_slot)],
                );
                if !self.grid.axis_x_range.is_empty() {
                    pass.draw(self.grid.axis_x_range.clone(), 0..1);
                }
                pass.set_bind_group(
                    0,
                    &self.uniform_bind_group,
                    &[Self::slot_offset(grid_ay_slot)],
                );
                if !self.grid.axis_y_range.is_empty() {
                    pass.draw(self.grid.axis_y_range.clone(), 0..1);
                }
                pass.set_bind_group(
                    0,
                    &self.uniform_bind_group,
                    &[Self::slot_offset(grid_az_slot)],
                );
                if !self.grid.axis_z_range.is_empty() {
                    pass.draw(self.grid.axis_z_range.clone(), 0..1);
                }
            }

            // Mesh
            if self.num_vertices > 0 {
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                match mode {
                    DisplayMode::AsIs | DisplayMode::Shading => {
                        pass.set_bind_group(
                            0,
                            &self.uniform_bind_group,
                            &[Self::slot_offset(mesh_slot)],
                        );
                        pass.set_pipeline(&self.solid_pipeline);
                        pass.draw(0..self.num_vertices, 0..1);
                    }
                    DisplayMode::Points => {
                        pass.set_bind_group(
                            0,
                            &self.uniform_bind_group,
                            &[Self::slot_offset(mesh_slot)],
                        );
                        pass.set_pipeline(&self.wire_pipeline);
                        pass.draw(0..self.num_vertices, 0..1);
                    }
                    DisplayMode::Wireframe => {
                        pass.set_bind_group(
                            0,
                            &self.uniform_bind_group,
                            &[Self::slot_offset(mesh_slot)],
                        );
                        pass.set_pipeline(&self.wire_pipeline);
                        pass.set_index_buffer(
                            self.edge_index_buffer.slice(..),
                            wgpu::IndexFormat::Uint32,
                        );
                        pass.draw_indexed(0..self.num_edge_indices, 0, 0..1);
                    }
                    DisplayMode::HiddenLine => {
                        // Draw solid white first (fills depth buffer), then edges on top
                        pass.set_bind_group(
                            0,
                            &self.uniform_bind_group,
                            &[Self::slot_offset(mesh_slot)],
                        );
                        pass.set_pipeline(&self.solid_pipeline);
                        pass.draw(0..self.num_vertices, 0..1);
                        pass.set_bind_group(
                            0,
                            &self.uniform_bind_group,
                            &[Self::slot_offset(wire_slot)],
                        );
                        pass.set_pipeline(&self.wire_pipeline);
                        pass.set_index_buffer(
                            self.edge_index_buffer.slice(..),
                            wgpu::IndexFormat::Uint32,
                        );
                        pass.draw_indexed(0..self.num_edge_indices, 0, 0..1);
                    }
                    DisplayMode::NoShading => {
                        pass.set_bind_group(
                            0,
                            &self.uniform_bind_group,
                            &[Self::slot_offset(mesh_slot)],
                        );
                        pass.set_pipeline(&self.solid_pipeline);
                        pass.draw(0..self.num_vertices, 0..1);
                    }
                    DisplayMode::Transparent => {
                        pass.set_bind_group(
                            0,
                            &self.uniform_bind_group,
                            &[Self::slot_offset(mesh_slot)],
                        );
                        pass.set_pipeline(&self.transparent_pipeline);
                        pass.draw(0..self.num_vertices, 0..1);
                    }
                    DisplayMode::FlatLines => {
                        pass.set_bind_group(
                            0,
                            &self.uniform_bind_group,
                            &[Self::slot_offset(mesh_slot)],
                        );
                        pass.set_pipeline(&self.solid_pipeline);
                        pass.draw(0..self.num_vertices, 0..1);
                        pass.set_bind_group(
                            0,
                            &self.uniform_bind_group,
                            &[Self::slot_offset(wire_slot)],
                        );
                        pass.set_pipeline(&self.wire_pipeline);
                        pass.set_index_buffer(
                            self.edge_index_buffer.slice(..),
                            wgpu::IndexFormat::Uint32,
                        );
                        pass.draw_indexed(0..self.num_edge_indices, 0, 0..1);
                    }
                }
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }

    pub fn update_mesh(&mut self, vertices: &[Vertex]) {
        let nv = vertices.len() as u32;
        self.vertex_buffer = create_vertex_buffer(&self.device, vertices);
        let edge_ids = build_edge_indices(nv);
        self.edge_index_buffer = create_edge_index_buffer(&self.device, &edge_ids);
        self.num_edge_indices = edge_ids.len() as u32;
        self.num_vertices = nv;
    }

    pub fn rebuild_grid(&mut self, config: &GridConfig) {
        let (verts, minor, major, ax, ay, az) = build_dynamic_grid(config);
        let contents: &[u8] = if verts.is_empty() {
            bytemuck::bytes_of(&Vertex {
                position: [0.0; 3],
                normal: [0.0; 3],
            })
        } else {
            bytemuck::cast_slice(&verts)
        };
        self.grid = GridOverlay {
            buffer: self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("grid_buffer"),
                    contents,
                    usage: wgpu::BufferUsages::VERTEX,
                }),
            minor_range: minor,
            major_range: major,
            axis_x_range: ax,
            axis_y_range: ay,
            axis_z_range: az,
        };
    }
}

// ---------------------------------------------------------------------------
// Color constants
// ---------------------------------------------------------------------------

pub(crate) const SOLID_COLOR: [f32; 4] = [0.7, 0.75, 0.8, 1.0];
pub(crate) const WIRE_COLOR: [f32; 4] = [0.1, 0.9, 0.1, 1.0];
pub(crate) const TRANSPARENT_COLOR: [f32; 4] = [0.5, 0.6, 0.8, 0.35];
pub(crate) const EDGE_OVERLAY_COLOR: [f32; 4] = [0.05, 0.05, 0.05, 1.0];
pub(crate) const POINT_COLOR: [f32; 4] = [1.0, 0.85, 0.2, 1.0];
pub(crate) const NO_SHADE_COLOR: [f32; 4] = [0.75, 0.78, 0.82, 1.0];
pub(crate) const HIDDEN_LINE_COLOR: [f32; 4] = [0.9, 0.9, 0.9, 1.0];

pub(crate) const GRID_MINOR_COLOR: [f32; 4] = [0.22, 0.22, 0.24, 0.25];
pub(crate) const GRID_MAJOR_COLOR: [f32; 4] = [0.42, 0.42, 0.44, 0.75];
pub(crate) const AXIS_X_COLOR: [f32; 4] = [0.8, 0.2, 0.2, 1.0];
pub(crate) const AXIS_Y_COLOR: [f32; 4] = [0.2, 0.8, 0.2, 1.0];
pub(crate) const AXIS_Z_COLOR: [f32; 4] = [0.3, 0.3, 0.9, 1.0];

// ---------------------------------------------------------------------------
// Buffer helpers
// ---------------------------------------------------------------------------

fn create_vertex_buffer(device: &wgpu::Device, vertices: &[Vertex]) -> wgpu::Buffer {
    if vertices.is_empty() {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertex_buffer"),
            contents: bytemuck::bytes_of(&Vertex {
                position: [0.0; 3],
                normal: [0.0; 3],
            }),
            usage: wgpu::BufferUsages::VERTEX,
        })
    } else {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertex_buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        })
    }
}

fn create_edge_index_buffer(device: &wgpu::Device, indices: &[u32]) -> wgpu::Buffer {
    if indices.is_empty() {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("edge_index_buffer"),
            contents: bytemuck::bytes_of(&0u32),
            usage: wgpu::BufferUsages::INDEX,
        })
    } else {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("edge_index_buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        })
    }
}

pub(crate) fn create_depth_texture(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    sample_count: u32,
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth_texture"),
        size: wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    texture.create_view(&Default::default())
}

pub(crate) fn create_msaa_texture(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    sample_count: u32,
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("msaa_texture"),
        size: wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: config.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    texture.create_view(&Default::default())
}

// ---------------------------------------------------------------------------
// Mouse state
// ---------------------------------------------------------------------------

pub(crate) struct MouseState {
    pub left_pressed: bool,
    pub middle_pressed: bool,
    pub right_pressed: bool,
    pub shift_held: bool,
    pub ctrl_held: bool,
    pub alt_held: bool,
    pub last_pos: Option<(f64, f64)>,
}

impl MouseState {
    pub fn new() -> Self {
        Self {
            left_pressed: false,
            middle_pressed: false,
            right_pressed: false,
            shift_held: false,
            ctrl_held: false,
            alt_held: false,
            last_pos: None,
        }
    }
}
