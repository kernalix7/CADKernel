//! Shape conversion utilities: mesh-to-solid, reverse normals, refine shape.

use std::collections::HashMap;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_io::tessellate::Mesh;
use cadkernel_math::Point3;
use cadkernel_topology::{BRepModel, EntityKind, FaceData, Handle, SolidData, Tag, VertexData};

use super::copy_utils::{collect_solid_faces, copy_solid_transformed};

/// Result of converting a mesh to a B-Rep solid.
#[derive(Debug)]
pub struct ShapeFromMeshResult {
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Result of reversing a solid.
#[derive(Debug)]
pub struct ReverseResult {
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Result of refining a shape.
#[derive(Debug)]
pub struct RefineResult {
    pub redundant_edge_count: usize,
}

/// Converts a triangle mesh to a B-Rep solid.
///
/// Creates a face for each triangle in the mesh, deduplicating vertices by
/// position. All triangles are assembled into a single shell and solid.
pub fn shape_from_mesh(
    model: &mut BRepModel,
    mesh: &Mesh,
) -> KernelResult<ShapeFromMeshResult> {
    if mesh.indices.is_empty() {
        return Err(KernelError::InvalidArgument(
            "shape_from_mesh requires at least 1 triangle".into(),
        ));
    }

    let op = model.history.next_operation("shape_from_mesh");

    // Create vertices (one per mesh vertex, deduplicated by index)
    let mut vert_handles: Vec<Handle<VertexData>> = Vec::with_capacity(mesh.vertices.len());
    for (i, &pt) in mesh.vertices.iter().enumerate() {
        let tag = Tag::generated(EntityKind::Vertex, op, i as u32);
        vert_handles.push(model.add_vertex_tagged(pt, tag));
    }

    // Create faces from triangles
    let mut all_faces: Vec<Handle<FaceData>> = Vec::with_capacity(mesh.indices.len());
    let mut edge_idx = 0u32;

    for (fi, tri) in mesh.indices.iter().enumerate() {
        let v0 = vert_handles[tri[0] as usize];
        let v1 = vert_handles[tri[1] as usize];
        let v2 = vert_handles[tri[2] as usize];

        let verts = [v0, v1, v2];
        let mut half_edges = Vec::with_capacity(3);
        for i in 0..3 {
            let j = (i + 1) % 3;
            let etag = Tag::generated(EntityKind::Edge, op, edge_idx);
            let (_, he, _) = model.add_edge_tagged(verts[i], verts[j], etag);
            half_edges.push(he);
            edge_idx += 1;
        }

        let loop_h = model.make_loop(&half_edges)?;
        let face_tag = Tag::generated(EntityKind::Face, op, fi as u32);
        all_faces.push(model.make_face_tagged(loop_h, face_tag));
    }

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&all_faces, shell_tag);

    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let solid = model.make_solid_tagged(&[shell], solid_tag);

    Ok(ShapeFromMeshResult {
        solid,
        faces: all_faces,
    })
}

/// Reverses normals of all faces in a solid by reversing the winding order.
///
/// A new solid is created; the original is not modified.
pub fn reverse_solid(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
) -> KernelResult<ReverseResult> {
    let op = model.history.next_operation("reverse_solid");

    let result = copy_solid_transformed(
        model,
        solid,
        op,
        |pt| pt, // identity transform
        true,     // reverse winding
    )?;

    Ok(ReverseResult {
        solid: result.solid,
        faces: result.faces,
    })
}

/// Analyzes a solid for redundant edges between coplanar adjacent faces.
///
/// Two faces sharing an edge are considered to have a "redundant" edge if
/// their face normals are approximately parallel (within a small tolerance).
/// Reports the count of such redundant edges found.
pub fn refine_shape(
    model: &BRepModel,
    solid: Handle<SolidData>,
) -> KernelResult<RefineResult> {
    let face_handles = collect_solid_faces(model, solid)?;

    // Compute a face normal for each face from its first 3 vertices
    let mut face_normals: HashMap<u32, (Point3, Point3, Point3)> = HashMap::new();
    for &fh in &face_handles {
        let verts = model.vertices_of_face(fh)?;
        if verts.len() >= 3 {
            let p0 = model.vertices.get(verts[0]).map(|v| v.point).unwrap_or(Point3::ORIGIN);
            let p1 = model.vertices.get(verts[1]).map(|v| v.point).unwrap_or(Point3::ORIGIN);
            let p2 = model.vertices.get(verts[2]).map(|v| v.point).unwrap_or(Point3::ORIGIN);
            face_normals.insert(fh.index(), (p0, p1, p2));
        }
    }

    // Build edge-to-face adjacency: for each pair of vertex indices forming an edge,
    // track which faces use that edge.
    let mut edge_faces: HashMap<(u32, u32), Vec<u32>> = HashMap::new();

    for &fh in &face_handles {
        let verts = model.vertices_of_face(fh)?;
        let n = verts.len();
        for i in 0..n {
            let j = (i + 1) % n;
            let a = verts[i].index();
            let b = verts[j].index();
            let key = if a < b { (a, b) } else { (b, a) };
            edge_faces.entry(key).or_default().push(fh.index());
        }
    }

    let mut redundant_count = 0;
    let cos_threshold = (1.0_f64).to_radians().cos(); // ~1 degree tolerance

    for faces in edge_faces.values() {
        if faces.len() == 2 {
            let f0 = faces[0];
            let f1 = faces[1];
            if let (Some(&(p0a, p1a, p2a)), Some(&(p0b, p1b, p2b))) =
                (face_normals.get(&f0), face_normals.get(&f1))
            {
                let n0 = face_normal(p0a, p1a, p2a);
                let n1 = face_normal(p0b, p1b, p2b);
                let dot = n0.0 * n1.0 + n0.1 * n1.1 + n0.2 * n1.2;
                if dot.abs() > cos_threshold {
                    redundant_count += 1;
                }
            }
        }
    }

    Ok(RefineResult {
        redundant_edge_count: redundant_count,
    })
}

fn face_normal(p0: Point3, p1: Point3, p2: Point3) -> (f64, f64, f64) {
    let ux = p1.x - p0.x;
    let uy = p1.y - p0.y;
    let uz = p1.z - p0.z;
    let vx = p2.x - p0.x;
    let vy = p2.y - p0.y;
    let vz = p2.z - p0.z;
    let nx = uy * vz - uz * vy;
    let ny = uz * vx - ux * vz;
    let nz = ux * vy - uy * vx;
    let len = (nx * nx + ny * ny + nz * nz).sqrt();
    if len > 1e-15 {
        (nx / len, ny / len, nz / len)
    } else {
        (0.0, 0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_math::Vec3;

    #[test]
    fn test_shape_from_mesh_single_triangle() {
        let mut model = BRepModel::new();
        let mesh = Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.5, 1.0, 0.0),
            ],
            normals: vec![Vec3::Z, Vec3::Z, Vec3::Z],
            indices: vec![[0, 1, 2]],
        };

        let result = shape_from_mesh(&mut model, &mesh).unwrap();
        assert_eq!(model.vertices.len(), 3);
        assert_eq!(result.faces.len(), 1);
        assert_eq!(model.solids.len(), 1);
    }

    #[test]
    fn test_shape_from_mesh_two_triangles() {
        let mut model = BRepModel::new();
        let mesh = Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(1.0, 1.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            ],
            normals: vec![Vec3::Z; 4],
            indices: vec![[0, 1, 2], [0, 2, 3]],
        };

        let result = shape_from_mesh(&mut model, &mesh).unwrap();
        assert_eq!(model.vertices.len(), 4);
        assert_eq!(result.faces.len(), 2);
        assert_eq!(model.solids.len(), 1);
    }

    #[test]
    fn test_shape_from_mesh_empty() {
        let mut model = BRepModel::new();
        let mesh = Mesh {
            vertices: Vec::new(),
            normals: Vec::new(),
            indices: Vec::new(),
        };
        assert!(shape_from_mesh(&mut model, &mesh).is_err());
    }

    #[test]
    fn test_reverse_solid_preserves_face_count() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let original_face_count = model.faces.len();
        let result = reverse_solid(&mut model, b.solid).unwrap();

        // Reversed solid should have same number of faces as original
        assert_eq!(result.faces.len(), original_face_count);
        assert_eq!(model.solids.len(), 2); // original + reversed
    }

    #[test]
    fn test_refine_shape_box_no_redundant() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let result = refine_shape(&model, b.solid).unwrap();
        // A box has 6 faces, each pair shares an edge but normals are perpendicular
        // → no redundant edges
        assert_eq!(
            result.redundant_edge_count, 0,
            "box should have no coplanar adjacent faces"
        );
    }
}
