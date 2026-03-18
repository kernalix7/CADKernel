//! Thickness (thicken) operation — creates a solid shell from a surface/face set.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, EntityKind, FaceData, Handle, SolidData, Tag, VertexData};

use super::copy_utils::collect_solid_faces;

/// Result of a thickness operation.
pub struct ThicknessResult {
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Thickens a solid by creating inner and outer copies connected at boundaries.
///
/// `thickness` is the wall thickness. The original faces become the outer surface,
/// and offset copies become the inner surface. Rim faces connect them at open edges.
///
/// This is similar to shell but uses directional offset (along face normals)
/// rather than removing faces.
#[allow(clippy::too_many_arguments)]
pub fn thickness_solid(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    thickness: f64,
    join_type: ThicknessJoin,
) -> KernelResult<ThicknessResult> {
    if thickness <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "thickness must be positive".into(),
        ));
    }

    let op = model.history.next_operation("thickness");
    let face_handles = collect_solid_faces(model, solid)?;

    // Compute per-vertex normals (averaged from adjacent faces)
    let mut vertex_normals: std::collections::HashMap<u32, Vec3> =
        std::collections::HashMap::new();

    for &face_h in &face_handles {
        let verts = model.vertices_of_face(face_h)?;
        if verts.len() < 3 {
            continue;
        }

        let positions: Vec<Point3> = verts
            .iter()
            .map(|&vh| {
                model
                    .vertices
                    .get(vh)
                    .map(|v| v.point)
                    .unwrap_or(Point3::ORIGIN)
            })
            .collect();

        let v01 = positions[1] - positions[0];
        let v02 = positions[2] - positions[0];
        let face_normal = v01.cross(v02);

        for &vh in &verts {
            vertex_normals
                .entry(vh.index())
                .and_modify(|n| *n += face_normal)
                .or_insert(face_normal);
        }
    }

    for n in vertex_normals.values_mut() {
        if let Some(nn) = n.normalized() {
            *n = nn;
        }
    }

    let offset = match join_type {
        ThicknessJoin::Inward => -thickness,
        ThicknessJoin::Outward => thickness,
        ThicknessJoin::Centered => thickness * 0.5,
    };

    // Create inner (offset) copy of vertices
    let mut vert_map: std::collections::HashMap<u32, Handle<VertexData>> =
        std::collections::HashMap::new();
    let mut vert_idx = 0u32;

    for &face_h in &face_handles {
        let verts = model.vertices_of_face(face_h)?;
        for &vh in &verts {
            let key = vh.index();
            if vert_map.contains_key(&key) {
                continue;
            }
            let vd = model
                .vertices
                .get(vh)
                .ok_or(KernelError::InvalidHandle("vertex"))?;
            let n = vertex_normals.get(&key).copied().unwrap_or(Vec3::Z);
            let inner_pt = Point3::new(
                vd.point.x - n.x * offset,
                vd.point.y - n.y * offset,
                vd.point.z - n.z * offset,
            );
            let tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
            vert_map.insert(key, model.add_vertex_tagged(inner_pt, tag));
            vert_idx += 1;
        }
    }

    // Create inner faces (reversed winding)
    let mut all_faces = Vec::new();
    let mut edge_idx = 0u32;
    let mut face_idx = 0u32;

    // Outer faces: copy original
    for &face_h in &face_handles {
        let verts = model.vertices_of_face(face_h)?;
        let n = verts.len();
        let mut hes = Vec::with_capacity(n);
        for i in 0..n {
            let j = (i + 1) % n;
            let etag = Tag::generated(EntityKind::Edge, op, edge_idx);
            let (_, he, _) = model.add_edge_tagged(verts[i], verts[j], etag);
            hes.push(he);
            edge_idx += 1;
        }
        let loop_h = model.make_loop(&hes)?;
        let tag = Tag::generated(EntityKind::Face, op, face_idx);
        all_faces.push(model.make_face_tagged(loop_h, tag));
        face_idx += 1;
    }

    // Inner faces: reversed winding
    for &face_h in &face_handles {
        let verts = model.vertices_of_face(face_h)?;
        let n = verts.len();
        let mut inner_verts: Vec<Handle<VertexData>> =
            verts.iter().map(|vh| vert_map[&vh.index()]).collect();
        inner_verts.reverse();

        let mut hes = Vec::with_capacity(n);
        for i in 0..n {
            let j = (i + 1) % n;
            let etag = Tag::generated(EntityKind::Edge, op, edge_idx);
            let (_, he, _) = model.add_edge_tagged(inner_verts[i], inner_verts[j], etag);
            hes.push(he);
            edge_idx += 1;
        }
        let loop_h = model.make_loop(&hes)?;
        let tag = Tag::generated(EntityKind::Face, op, face_idx);
        all_faces.push(model.make_face_tagged(loop_h, tag));
        face_idx += 1;
    }

    // Rim faces connecting outer and inner boundaries
    // For each edge of each face, create a quad connecting outer→inner
    // This is simplified: connect each outer edge to its inner counterpart
    for &face_h in &face_handles {
        let verts = model.vertices_of_face(face_h)?;
        let n = verts.len();
        for i in 0..n {
            let j = (i + 1) % n;
            let o1 = verts[i];
            let o2 = verts[j];
            let i1 = vert_map[&o1.index()];
            let i2 = vert_map[&o2.index()];

            let hes = [
                {
                    let etag = Tag::generated(EntityKind::Edge, op, edge_idx);
                    edge_idx += 1;
                    let (_, he, _) = model.add_edge_tagged(o1, o2, etag);
                    he
                },
                {
                    let etag = Tag::generated(EntityKind::Edge, op, edge_idx);
                    edge_idx += 1;
                    let (_, he, _) = model.add_edge_tagged(o2, i2, etag);
                    he
                },
                {
                    let etag = Tag::generated(EntityKind::Edge, op, edge_idx);
                    edge_idx += 1;
                    let (_, he, _) = model.add_edge_tagged(i2, i1, etag);
                    he
                },
                {
                    let etag = Tag::generated(EntityKind::Edge, op, edge_idx);
                    edge_idx += 1;
                    let (_, he, _) = model.add_edge_tagged(i1, o1, etag);
                    he
                },
            ];
            let loop_h = model.make_loop(&hes)?;
            let tag = Tag::generated(EntityKind::Face, op, face_idx);
            all_faces.push(model.make_face_tagged(loop_h, tag));
            face_idx += 1;
        }
    }

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&all_faces, shell_tag);
    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let new_solid = model.make_solid_tagged(&[shell], solid_tag);

    Ok(ThicknessResult {
        solid: new_solid,
        faces: all_faces,
    })
}

/// Join type for the thickness operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThicknessJoin {
    /// Offset inward (shrink).
    Inward,
    /// Offset outward (grow).
    Outward,
    /// Center the thickness around the original surface.
    Centered,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thickness_box() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let result = thickness_solid(&mut model, b.solid, 0.5, ThicknessJoin::Inward).unwrap();
        assert!(model.solids.is_alive(result.solid));
        // outer(6) + inner(6) + rim(6*4=24) = 36 faces
        assert_eq!(result.faces.len(), 36);
    }

    #[test]
    fn test_thickness_validation() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        assert!(thickness_solid(&mut model, b.solid, 0.0, ThicknessJoin::Outward).is_err());
        assert!(thickness_solid(&mut model, b.solid, -1.0, ThicknessJoin::Outward).is_err());
    }

    #[test]
    fn test_thickness_centered() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let result = thickness_solid(&mut model, b.solid, 1.0, ThicknessJoin::Centered).unwrap();
        assert!(model.solids.is_alive(result.solid));
    }
}
