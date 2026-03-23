use cadkernel_geometry::Surface;
use cadkernel_geometry::surface::parametric_wire::ParametricWire2D;
use cadkernel_geometry::tessellate::{TessellationOptions, adaptive_tessellate_surface};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, FaceData, Handle, SolidData};
use rayon::prelude::*;
use std::collections::HashMap;

/// A single triangle with pre-computed normal.
#[derive(Debug, Clone, Copy)]
pub struct Triangle {
    pub vertices: [Point3; 3],
    pub normal: Vec3,
}

/// An indexed triangle mesh produced by tessellation.
#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<Point3>,
    pub normals: Vec<Vec3>,
    pub indices: Vec<[u32; 3]>,
}

impl Mesh {
    /// Creates an empty mesh with no vertices or triangles.
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            normals: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// Number of triangles in the mesh.
    pub fn triangle_count(&self) -> usize {
        self.indices.len()
    }

    /// Convert to a flat list of [`Triangle`]s.
    pub fn to_triangles(&self) -> Vec<Triangle> {
        self.indices
            .par_iter()
            .zip(self.normals.par_iter())
            .map(|(idx, &n)| Triangle {
                vertices: [
                    self.vertices[idx[0] as usize],
                    self.vertices[idx[1] as usize],
                    self.vertices[idx[2] as usize],
                ],
                normal: n,
            })
            .collect()
    }
}

impl Default for Mesh {
    fn default() -> Self {
        Self::new()
    }
}

/// Tessellates a single B-Rep face into triangles.
///
/// When the face has a bound surface geometry (via `geometry-binding` feature),
/// uses adaptive tessellation for higher-fidelity results on curved surfaces.
/// Otherwise falls back to simple fan triangulation of the outer-loop vertices.
pub fn tessellate_face(model: &BRepModel, face: Handle<FaceData>) -> Vec<Triangle> {
    if let Some(fd) = model.faces.get(face) {
        if let Some(ref surface) = fd.surface {
            let boundary = collect_face_points(model, face);
            if let Some(tess_mesh) = tessellate_surface_with_trim(
                surface.as_ref(),
                &boundary,
                fd.outer_trim.as_ref(),
                &fd.inner_trims,
            ) {
                return tess_mesh
                    .indices
                    .iter()
                    .map(|idx| {
                        let a = tess_mesh.vertices[idx[0] as usize];
                        let b = tess_mesh.vertices[idx[1] as usize];
                        let c = tess_mesh.vertices[idx[2] as usize];
                        Triangle {
                            vertices: [a, b, c],
                            normal: triangle_normal(a, b, c),
                        }
                    })
                    .collect();
            }
        }
    }
    let points = collect_face_points(model, face);
    fan_triangulate(&points)
}

/// Tessellates all faces of a solid into a single indexed [`Mesh`].
///
/// Vertices are shared across faces via bit-exact position deduplication,
/// enabling cross-face smooth normal computation in the viewer.
/// Returns an empty mesh if the solid or any of its sub-entities are invalid.
pub fn tessellate_solid(model: &BRepModel, solid: Handle<SolidData>) -> Mesh {
    let mut mesh = Mesh::new();
    let mut vmap: HashMap<(u64, u64, u64), u32> = HashMap::new();

    let Some(solid_data) = model.solids.get(solid) else {
        return mesh;
    };
    for &shell_h in &solid_data.shells {
        let Some(shell_data) = model.shells.get(shell_h) else {
            continue;
        };
        for &face_h in &shell_data.faces {
            // Use surface tessellation ONLY for non-planar surfaces.
            // Planar faces use simple polygon fan from boundary vertices,
            // which guarantees faces stay within their boundary.
            let face_data = model.faces.get(face_h);
            let use_surface_tess = face_data.is_some_and(|fd| {
                fd.surface.as_ref().is_some_and(|s| {
                    // Check if the surface produces non-coplanar points.
                    // Sample 4 corners of [0,1]x[0,1] domain. If all on same plane, it's planar.
                    let p00 = s.point_at(0.0, 0.0);
                    let p10 = s.point_at(1.0, 0.0);
                    let p01 = s.point_at(0.0, 1.0);
                    let p11 = s.point_at(1.0, 1.0);
                    let n1 = (p10 - p00).cross(p01 - p00);
                    let d = (p11 - p00).dot(n1);
                    // Non-planar if deviation > threshold
                    d.abs() > 1e-6
                })
            });

            if use_surface_tess {
                let fd = face_data.unwrap();
                let surface = fd.surface.as_ref().unwrap();
                let boundary = collect_face_points(model, face_h);
                let tess_result = tessellate_surface_with_trim(
                    surface.as_ref(),
                    &boundary,
                    fd.outer_trim.as_ref(),
                    &fd.inner_trims,
                );
                // Validate: all tessellated vertices must be within reasonable
                // distance of the boundary bounding box + margin.
                let tess_ok = tess_result.as_ref().is_some_and(|tm| {
                    if boundary.is_empty() || tm.vertices.is_empty() { return false; }
                    let (mut mn, mut mx) = (boundary[0], boundary[0]);
                    for p in &boundary {
                        mn = Point3::new(mn.x.min(p.x), mn.y.min(p.y), mn.z.min(p.z));
                        mx = Point3::new(mx.x.max(p.x), mx.y.max(p.y), mx.z.max(p.z));
                    }
                    let diag = ((mx.x-mn.x).powi(2) + (mx.y-mn.y).powi(2) + (mx.z-mn.z).powi(2)).sqrt();
                    let margin = diag * 0.5 + 1.0;
                    tm.vertices.iter().all(|v| {
                        v.x >= mn.x - margin && v.x <= mx.x + margin &&
                        v.y >= mn.y - margin && v.y <= mx.y + margin &&
                        v.z >= mn.z - margin && v.z <= mx.z + margin
                    })
                });
                if let Some(tess_mesh) = tess_result.filter(|_| tess_ok) {
                    // Remap tessellation vertices into the shared mesh.
                    let remap: Vec<u32> = tess_mesh
                        .vertices
                        .iter()
                        .map(|p| {
                            let key = (p.x.to_bits(), p.y.to_bits(), p.z.to_bits());
                            *vmap.entry(key).or_insert_with(|| {
                                let i = mesh.vertices.len() as u32;
                                mesh.vertices.push(*p);
                                i
                            })
                        })
                        .collect();
                    for idx in &tess_mesh.indices {
                        let a = tess_mesh.vertices[idx[0] as usize];
                        let b = tess_mesh.vertices[idx[1] as usize];
                        let c = tess_mesh.vertices[idx[2] as usize];
                        let n = triangle_normal(a, b, c);
                        mesh.normals.push(n);
                        mesh.indices
                            .push([remap[idx[0] as usize], remap[idx[1] as usize], remap[idx[2] as usize]]);
                    }
                    continue;
                }
            }

            // Fallback: fan triangulation from outer-loop vertices.
            let points = collect_face_points(model, face_h);
            if points.len() < 3 {
                continue;
            }
            let face_idx: Vec<u32> = points
                .iter()
                .map(|p| {
                    let key = (p.x.to_bits(), p.y.to_bits(), p.z.to_bits());
                    *vmap.entry(key).or_insert_with(|| {
                        let i = mesh.vertices.len() as u32;
                        mesh.vertices.push(*p);
                        i
                    })
                })
                .collect();
            for i in 1..(face_idx.len() - 1) {
                let n = triangle_normal(points[0], points[i], points[i + 1]);
                mesh.normals.push(n);
                mesh.indices
                    .push([face_idx[0], face_idx[i], face_idx[i + 1]]);
            }
        }
    }

    mesh
}

/// Merges multiple [`Mesh`] objects into a single mesh.
///
/// Vertices and normals are concatenated. Triangle indices are offset so
/// they refer to the correct positions in the combined vertex array.
pub fn merge_meshes(meshes: &[Mesh]) -> Mesh {
    let total_verts: usize = meshes.iter().map(|m| m.vertices.len()).sum();
    let total_tris: usize = meshes.iter().map(|m| m.indices.len()).sum();

    let mut vertices = Vec::with_capacity(total_verts);
    let mut normals = Vec::with_capacity(total_tris);
    let mut indices = Vec::with_capacity(total_tris);

    for m in meshes {
        let offset = vertices.len() as u32;
        vertices.extend_from_slice(&m.vertices);
        normals.extend_from_slice(&m.normals);
        for idx in &m.indices {
            indices.push([idx[0] + offset, idx[1] + offset, idx[2] + offset]);
        }
    }

    Mesh { vertices, normals, indices }
}

/// Collects all face handles from a solid's shells.
fn collect_solid_face_handles(
    model: &BRepModel,
    solid: Handle<SolidData>,
) -> Vec<Handle<FaceData>> {
    let Some(solid_data) = model.solids.get(solid) else {
        return Vec::new();
    };
    let mut faces = Vec::new();
    for &shell_h in &solid_data.shells {
        if let Some(shell_data) = model.shells.get(shell_h) {
            faces.extend_from_slice(&shell_data.faces);
        }
    }
    faces
}

/// Tessellates a single face into an indexed [`Mesh`] (not shared-vertex).
///
/// This is a per-face helper used by [`tessellate_solid_parallel`].
fn tessellate_face_to_mesh(model: &BRepModel, face_h: Handle<FaceData>) -> Mesh {
    let face_data = model.faces.get(face_h);
    let has_surface = face_data.is_some_and(|fd| fd.surface.is_some());

    if has_surface {
        let fd = face_data.unwrap();
        let surface = fd.surface.as_ref().unwrap();
        let boundary = collect_face_points(model, face_h);
        if let Some(tess_mesh) = tessellate_surface_with_trim(
            surface.as_ref(),
            &boundary,
            fd.outer_trim.as_ref(),
            &fd.inner_trims,
        ) {
            let normals = tess_mesh
                .indices
                .iter()
                .map(|idx| {
                    let a = tess_mesh.vertices[idx[0] as usize];
                    let b = tess_mesh.vertices[idx[1] as usize];
                    let c = tess_mesh.vertices[idx[2] as usize];
                    triangle_normal(a, b, c)
                })
                .collect();
            return Mesh {
                vertices: tess_mesh.vertices,
                normals,
                indices: tess_mesh.indices,
            };
        }
    }

    let points = collect_face_points(model, face_h);
    if points.len() < 3 {
        return Mesh::new();
    }
    let mut mesh = Mesh::new();
    mesh.vertices = points;
    for i in 1..(mesh.vertices.len() - 1) {
        let n = triangle_normal(mesh.vertices[0], mesh.vertices[i], mesh.vertices[i + 1]);
        mesh.normals.push(n);
        mesh.indices.push([0, i as u32, (i + 1) as u32]);
    }
    mesh
}

/// Parallelized version of [`tessellate_solid`].
///
/// Tessellates each face in parallel using rayon, then merges the sub-meshes
/// into a single indexed [`Mesh`]. Falls back to serial processing when the
/// solid contains only one face.
///
/// Unlike [`tessellate_solid`], vertices are **not** deduplicated across faces.
/// This makes the function suitable for scenarios where per-face independence
/// is acceptable (e.g., preview rendering). For shared-vertex output with
/// cross-face smooth normals, use [`tessellate_solid`] instead.
pub fn tessellate_solid_parallel(model: &BRepModel, solid: Handle<SolidData>) -> Mesh {
    let face_handles = collect_solid_face_handles(model, solid);

    if face_handles.is_empty() {
        return Mesh::new();
    }

    // Serial fallback for single face.
    if face_handles.len() == 1 {
        return tessellate_face_to_mesh(model, face_handles[0]);
    }

    let sub_meshes: Vec<Mesh> = face_handles
        .par_iter()
        .map(|&fh| tessellate_face_to_mesh(model, fh))
        .collect();

    merge_meshes(&sub_meshes)
}

/// Tessellates a bound surface using the face boundary to determine the
/// parameter domain.  When trim wires are present, filters triangles
/// whose UV centroids fall outside the trimmed region.
fn tessellate_surface_with_trim(
    surface: &(dyn Surface + Send + Sync),
    boundary: &[Point3],
    outer_trim: Option<&ParametricWire2D>,
    inner_trims: &[ParametricWire2D],
) -> Option<cadkernel_geometry::tessellate::TessMesh> {
    if boundary.is_empty() {
        return None;
    }

    let (u_lo, u_hi) = surface.domain_u();
    let (v_lo, v_hi) = surface.domain_v();

    let (u_domain, v_domain) = if u_lo.is_finite() && u_hi.is_finite() && v_lo.is_finite() && v_hi.is_finite() {
        ((u_lo, u_hi), (v_lo, v_hi))
    } else {
        let mut u_min = f64::INFINITY;
        let mut u_max = f64::NEG_INFINITY;
        let mut v_min = f64::INFINITY;
        let mut v_max = f64::NEG_INFINITY;
        for pt in boundary {
            let (u, v, _) = surface.project_point(*pt);
            if u.is_finite() && v.is_finite() {
                u_min = u_min.min(u);
                u_max = u_max.max(u);
                v_min = v_min.min(v);
                v_max = v_max.max(v);
            }
        }
        if !u_min.is_finite() || !u_max.is_finite() || !v_min.is_finite() || !v_max.is_finite() {
            return None;
        }
        let u_margin = (u_max - u_min) * 0.01;
        let v_margin = (v_max - v_min) * 0.01;
        ((u_min - u_margin, u_max + u_margin), (v_min - v_margin, v_max + v_margin))
    };

    let opts = TessellationOptions::default();
    let tess = adaptive_tessellate_surface(
        |u, v| surface.point_at(u, v),
        |u, v| surface.normal_at(u, v),
        u_domain,
        v_domain,
        &opts,
    );

    // If trim wires are present, filter triangles by UV containment.
    if outer_trim.is_none() && inner_trims.is_empty() {
        return Some(tess);
    }

    let has_outer = outer_trim.is_some();
    let mut filtered_indices = Vec::new();

    for idx in &tess.indices {
        let a = tess.vertices[idx[0] as usize];
        let b = tess.vertices[idx[1] as usize];
        let c = tess.vertices[idx[2] as usize];

        let centroid = Point3::new(
            (a.x + b.x + c.x) / 3.0,
            (a.y + b.y + c.y) / 3.0,
            (a.z + b.z + c.z) / 3.0,
        );
        let (u, v, _) = surface.project_point(centroid);

        if has_outer && !outer_trim.unwrap().contains_point(u, v) {
            continue;
        }
        if inner_trims.iter().any(|hole| hole.contains_point(u, v)) {
            continue;
        }

        filtered_indices.push(*idx);
    }

    Some(cadkernel_geometry::tessellate::TessMesh {
        vertices: tess.vertices,
        indices: filtered_indices,
    })
}

fn collect_face_points(model: &BRepModel, face: Handle<FaceData>) -> Vec<Point3> {
    let Some(face_data) = model.faces.get(face) else {
        return Vec::new();
    };
    let Some(loop_data) = model.loops.get(face_data.outer_loop) else {
        return Vec::new();
    };

    let half_edges = model.loop_half_edges(loop_data.half_edge);
    half_edges
        .iter()
        .filter_map(|&he_h| {
            let he = model.half_edges.get(he_h)?;
            let v = model.vertices.get(he.origin)?;
            Some(v.point)
        })
        .collect()
}

fn triangle_normal(a: Point3, b: Point3, c: Point3) -> Vec3 {
    let u = b - a;
    let v = c - a;
    u.cross(v).normalized().unwrap_or(Vec3::Z)
}

fn fan_triangulate(points: &[Point3]) -> Vec<Triangle> {
    if points.len() < 3 {
        return Vec::new();
    }
    let mut tris = Vec::with_capacity(points.len() - 2);
    let a = points[0];
    for i in 1..(points.len() - 1) {
        let b = points[i];
        let c = points[i + 1];
        tris.push(Triangle {
            vertices: [a, b, c],
            normal: triangle_normal(a, b, c),
        });
    }
    tris
}

#[allow(dead_code)]
fn append_fan_to_mesh(mesh: &mut Mesh, points: &[Point3]) {
    if points.len() < 3 {
        return;
    }
    let base = mesh.vertices.len() as u32;
    mesh.vertices.extend_from_slice(points);

    for i in 1..(points.len() - 1) {
        let a = points[0];
        let b = points[i];
        let c = points[i + 1];
        let n = triangle_normal(a, b, c);
        mesh.normals.push(n);
        mesh.indices
            .push([base, base + i as u32, base + (i + 1) as u32]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_math::Point3;
    use cadkernel_topology::{BRepModel, Orientation};

    #[test]
    fn test_fan_triangulate_quad() {
        let pts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let tris = fan_triangulate(&pts);
        assert_eq!(tris.len(), 2);
    }

    #[test]
    fn test_fan_triangulate_triangle() {
        let pts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.5, 1.0, 0.0),
        ];
        let tris = fan_triangulate(&pts);
        assert_eq!(tris.len(), 1);
        assert!((tris[0].normal.z - 1.0).abs() < 1e-8 || (tris[0].normal.z + 1.0).abs() < 1e-8);
    }

    #[test]
    fn test_tessellate_face_from_brep() {
        let mut model = BRepModel::new();
        let v0 = model.add_vertex(Point3::new(0.0, 0.0, 0.0));
        let v1 = model.add_vertex(Point3::new(1.0, 0.0, 0.0));
        let v2 = model.add_vertex(Point3::new(1.0, 1.0, 0.0));
        let v3 = model.add_vertex(Point3::new(0.0, 1.0, 0.0));

        let (_, he01, _) = model.add_edge(v0, v1);
        let (_, he12, _) = model.add_edge(v1, v2);
        let (_, he23, _) = model.add_edge(v2, v3);
        let (_, he30, _) = model.add_edge(v3, v0);

        let loop_h = model.make_loop(&[he01, he12, he23, he30]).unwrap();
        let face = model.make_face(loop_h);

        let tris = tessellate_face(&model, face);
        assert_eq!(tris.len(), 2);
    }

    #[test]
    fn test_mesh_from_solid() {
        let mut model = BRepModel::new();
        let v0 = model.add_vertex(Point3::new(0.0, 0.0, 0.0));
        let v1 = model.add_vertex(Point3::new(1.0, 0.0, 0.0));
        let v2 = model.add_vertex(Point3::new(0.5, 1.0, 0.0));

        let (_, he01, _) = model.add_edge(v0, v1);
        let (_, he12, _) = model.add_edge(v1, v2);
        let (_, he20, _) = model.add_edge(v2, v0);

        let loop_h = model.make_loop(&[he01, he12, he20]).unwrap();
        let face = model.make_face(loop_h);
        let shell = model.make_shell(&[face]);
        let solid = model.make_solid(&[shell]);

        let mesh = tessellate_solid(&model, solid);
        assert_eq!(mesh.triangle_count(), 1);
        assert_eq!(mesh.vertices.len(), 3);
    }

    #[test]
    fn test_tessellate_face_with_surface_geometry() {
        use cadkernel_geometry::surface::plane::Plane;
        use cadkernel_math::Vec3;
        use std::sync::Arc;

        let mut model = BRepModel::new();
        let v0 = model.add_vertex(Point3::new(0.0, 0.0, 0.0));
        let v1 = model.add_vertex(Point3::new(1.0, 0.0, 0.0));
        let v2 = model.add_vertex(Point3::new(1.0, 1.0, 0.0));
        let v3 = model.add_vertex(Point3::new(0.0, 1.0, 0.0));

        let (_, he01, _) = model.add_edge(v0, v1);
        let (_, he12, _) = model.add_edge(v1, v2);
        let (_, he23, _) = model.add_edge(v2, v3);
        let (_, he30, _) = model.add_edge(v3, v0);

        let loop_h = model.make_loop(&[he01, he12, he23, he30]).unwrap();
        let face = model.make_face(loop_h);

        // Bind a planar surface to the face.
        let plane = Plane::new(Point3::ORIGIN, Vec3::X, Vec3::Y).unwrap();
        model.bind_face_surface(face, Arc::new(plane), Orientation::Forward);

        let tris = tessellate_face(&model, face);
        // Adaptive tessellation produces more triangles than simple fan (min_segments=4 → 4×4=16 quads → 32 tris).
        assert!(tris.len() >= 2, "expected at least 2 triangles, got {}", tris.len());
        // All vertices should be on z=0 plane.
        for tri in &tris {
            for v in &tri.vertices {
                assert!((v.z).abs() < 1e-10, "vertex not on z=0 plane: {:?}", v);
            }
        }
    }

    #[test]
    fn test_tessellate_solid_with_surface_geometry() {
        use cadkernel_geometry::surface::sphere::Sphere;
        use std::sync::Arc;

        let mut model = BRepModel::new();
        let v0 = model.add_vertex(Point3::new(1.0, 0.0, 0.0));
        let v1 = model.add_vertex(Point3::new(0.0, 1.0, 0.0));
        let v2 = model.add_vertex(Point3::new(0.0, 0.0, 1.0));

        let (_, he01, _) = model.add_edge(v0, v1);
        let (_, he12, _) = model.add_edge(v1, v2);
        let (_, he20, _) = model.add_edge(v2, v0);

        let loop_h = model.make_loop(&[he01, he12, he20]).unwrap();
        let face = model.make_face(loop_h);

        let sphere = Sphere::new(Point3::ORIGIN, 1.0).unwrap();
        model.bind_face_surface(face, Arc::new(sphere), Orientation::Forward);

        let shell = model.make_shell(&[face]);
        let solid = model.make_solid(&[shell]);

        let mesh = tessellate_solid(&model, solid);
        // With only 3 boundary vertices and no trim wires,
        // a sphere face falls back to fan tessellation (1 triangle).
        // Surface tessellation only activates when boundary validation passes.
        assert!(mesh.triangle_count() >= 1, "expected ≥1 triangle, got {}", mesh.triangle_count());
    }

    #[test]
    fn test_tessellate_face_with_trim() {
        use cadkernel_geometry::curve::curve2d::Line2D;
        use cadkernel_geometry::surface::parametric_wire::ParametricWire2D;
        use cadkernel_geometry::surface::plane::Plane;
        use cadkernel_math::{Point2, Vec3};
        use std::sync::Arc;

        let mut model = BRepModel::new();
        let v0 = model.add_vertex(Point3::new(0.0, 0.0, 0.0));
        let v1 = model.add_vertex(Point3::new(1.0, 0.0, 0.0));
        let v2 = model.add_vertex(Point3::new(1.0, 1.0, 0.0));
        let v3 = model.add_vertex(Point3::new(0.0, 1.0, 0.0));

        let (_, he01, _) = model.add_edge(v0, v1);
        let (_, he12, _) = model.add_edge(v1, v2);
        let (_, he23, _) = model.add_edge(v2, v3);
        let (_, he30, _) = model.add_edge(v3, v0);

        let loop_h = model.make_loop(&[he01, he12, he23, he30]).unwrap();
        let face = model.make_face(loop_h);

        let plane = Plane::new(Point3::ORIGIN, Vec3::X, Vec3::Y).unwrap();
        model.bind_face_surface(face, Arc::new(plane), Orientation::Forward);

        // Bind a circular trim loop covering only a portion of the surface
        let outer_trim = ParametricWire2D::closed(vec![
            Arc::new(Line2D::new(Point2::new(0.2, 0.2), Point2::new(0.8, 0.2))) as Arc<dyn cadkernel_geometry::curve::curve2d::Curve2D>,
            Arc::new(Line2D::new(Point2::new(0.8, 0.2), Point2::new(0.8, 0.8))),
            Arc::new(Line2D::new(Point2::new(0.8, 0.8), Point2::new(0.2, 0.8))),
            Arc::new(Line2D::new(Point2::new(0.2, 0.8), Point2::new(0.2, 0.2))),
        ]);
        model.bind_face_trim(face, outer_trim, vec![]);

        let tris_trimmed = tessellate_face(&model, face);
        // Trimmed face should have fewer triangles than untrimmed
        assert!(!tris_trimmed.is_empty(), "trimmed tessellation should produce some triangles");
        // All vertices should lie roughly inside the trim region [0.2, 0.8]
        for tri in &tris_trimmed {
            let cx = (tri.vertices[0].x + tri.vertices[1].x + tri.vertices[2].x) / 3.0;
            let cy = (tri.vertices[0].y + tri.vertices[1].y + tri.vertices[2].y) / 3.0;
            assert!((0.1..=0.9).contains(&cx), "triangle centroid x={} outside trim", cx);
            assert!((0.1..=0.9).contains(&cy), "triangle centroid y={} outside trim", cy);
        }
    }

    #[test]
    fn test_tessellate_parallel_matches_serial() {
        let mut model = BRepModel::new();

        // Build a box-like solid with 2 triangle faces (2 shells not needed, just 2 faces).
        let v0 = model.add_vertex(Point3::new(0.0, 0.0, 0.0));
        let v1 = model.add_vertex(Point3::new(1.0, 0.0, 0.0));
        let v2 = model.add_vertex(Point3::new(1.0, 1.0, 0.0));
        let v3 = model.add_vertex(Point3::new(0.0, 1.0, 0.0));
        let v4 = model.add_vertex(Point3::new(0.0, 0.0, 1.0));

        // Face 1: triangle v0-v1-v2
        let (_, he01, _) = model.add_edge(v0, v1);
        let (_, he12, _) = model.add_edge(v1, v2);
        let (_, he20, _) = model.add_edge(v2, v0);
        let loop1 = model.make_loop(&[he01, he12, he20]).unwrap();
        let face1 = model.make_face(loop1);

        // Face 2: triangle v0-v3-v4
        let (_, he03, _) = model.add_edge(v0, v3);
        let (_, he34, _) = model.add_edge(v3, v4);
        let (_, he40, _) = model.add_edge(v4, v0);
        let loop2 = model.make_loop(&[he03, he34, he40]).unwrap();
        let face2 = model.make_face(loop2);

        let shell = model.make_shell(&[face1, face2]);
        let solid = model.make_solid(&[shell]);

        let serial = tessellate_solid(&model, solid);
        let parallel = tessellate_solid_parallel(&model, solid);

        assert_eq!(serial.triangle_count(), parallel.triangle_count());
        // Both should have 2 triangles (one per face).
        assert_eq!(serial.triangle_count(), 2);
        assert_eq!(parallel.triangle_count(), 2);
    }

    #[test]
    fn test_merge_meshes() {
        let mesh_a = Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.5, 1.0, 0.0),
            ],
            normals: vec![Vec3::Z],
            indices: vec![[0, 1, 2]],
        };
        let mesh_b = Mesh {
            vertices: vec![
                Point3::new(2.0, 0.0, 0.0),
                Point3::new(3.0, 0.0, 0.0),
                Point3::new(2.5, 1.0, 0.0),
            ],
            normals: vec![Vec3::Z],
            indices: vec![[0, 1, 2]],
        };

        let merged = merge_meshes(&[mesh_a, mesh_b]);

        assert_eq!(merged.vertices.len(), 6);
        assert_eq!(merged.normals.len(), 2);
        assert_eq!(merged.indices.len(), 2);
        // First triangle indices unchanged.
        assert_eq!(merged.indices[0], [0, 1, 2]);
        // Second triangle indices offset by 3.
        assert_eq!(merged.indices[1], [3, 4, 5]);
    }
}
