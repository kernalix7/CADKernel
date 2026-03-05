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

/// Tessellates a single B-Rep face into triangles using fan triangulation.
///
/// Collects the face's outer-loop vertices and builds a triangle fan from
/// the first vertex.  This is correct for convex planar polygons and a
/// reasonable approximation for mildly non-convex ones.
pub fn tessellate_face(model: &BRepModel, face: Handle<FaceData>) -> Vec<Triangle> {
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
            let points = collect_face_points(model, face_h);
            if points.len() < 3 {
                continue;
            }
            // Map face points to shared vertex indices.
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
            // Fan triangulation using shared indices.
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
    use cadkernel_topology::BRepModel;

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
}
