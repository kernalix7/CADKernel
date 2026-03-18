//! Loft operation: blends between two or more cross-section profiles.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::Point3;
use cadkernel_topology::{BRepModel, EntityKind, FaceData, Handle, SolidData, Tag, VertexData};

/// Result of a loft operation.
#[derive(Debug)]
pub struct LoftResult {
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Lofts between two or more cross-section profiles to produce a solid.
///
/// Each profile is a closed polyline given as a slice of 3D points.
/// Profiles must all have the same number of points.
/// Adjacent profiles are connected with quad faces; first and last profiles
/// are capped.
pub fn loft(model: &mut BRepModel, profiles: &[&[Point3]]) -> KernelResult<LoftResult> {
    if profiles.len() < 2 {
        return Err(KernelError::InvalidArgument(
            "loft needs at least 2 profiles".into(),
        ));
    }
    let np = profiles[0].len();
    if np < 3 {
        return Err(KernelError::InvalidArgument(
            "loft profiles need at least 3 points".into(),
        ));
    }
    for (i, p) in profiles.iter().enumerate() {
        if p.len() != np {
            return Err(KernelError::InvalidArgument(format!(
                "profile {} has {} points, expected {}",
                i,
                p.len(),
                np
            )));
        }
    }

    let ns = profiles.len();
    let op = model.history.next_operation("loft");

    // Create vertices for each profile ring.
    let mut rings: Vec<Vec<Handle<VertexData>>> = Vec::with_capacity(ns);
    let mut vert_idx = 0u32;

    for profile in profiles {
        let mut ring = Vec::with_capacity(np);
        for &pt in *profile {
            let tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
            ring.push(model.add_vertex_tagged(pt, tag));
            vert_idx += 1;
        }
        rings.push(ring);
    }

    let mut all_faces = Vec::new();
    let mut edge_idx = 0u32;
    let mut face_idx = 0u32;

    // Bottom cap (reversed winding so normal points outward)
    let bottom = make_ring_face(model, &rings[0], op, &mut edge_idx, face_idx, true)?;
    all_faces.push(bottom);
    face_idx += 1;

    // Top cap
    let top = make_ring_face(model, &rings[ns - 1], op, &mut edge_idx, face_idx, false)?;
    all_faces.push(top);
    face_idx += 1;

    // Side quads between consecutive rings
    for si in 0..ns - 1 {
        for pi in 0..np {
            let next_pi = (pi + 1) % np;
            let verts = [
                rings[si][pi],
                rings[si][next_pi],
                rings[si + 1][next_pi],
                rings[si + 1][pi],
            ];
            let f = make_quad_face(model, &verts, op, &mut edge_idx, face_idx)?;
            all_faces.push(f);
            face_idx += 1;
        }
    }

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&all_faces, shell_tag);
    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let solid = model.make_solid_tagged(&[shell], solid_tag);

    Ok(LoftResult {
        solid,
        faces: all_faces,
    })
}

fn make_ring_face(
    model: &mut BRepModel,
    ring: &[Handle<VertexData>],
    op: cadkernel_topology::OperationId,
    edge_idx: &mut u32,
    face_idx: u32,
    reverse: bool,
) -> KernelResult<Handle<FaceData>> {
    let n = ring.len();
    let ordered: Vec<Handle<VertexData>> = if reverse {
        ring.iter().rev().copied().collect()
    } else {
        ring.to_vec()
    };
    let mut hes = Vec::with_capacity(n);
    for i in 0..n {
        let j = (i + 1) % n;
        let etag = Tag::generated(EntityKind::Edge, op, *edge_idx);
        let (_, he, _) = model.add_edge_tagged(ordered[i], ordered[j], etag);
        hes.push(he);
        *edge_idx += 1;
    }
    let lp = model.make_loop(&hes)?;
    let ft = Tag::generated(EntityKind::Face, op, face_idx);
    Ok(model.make_face_tagged(lp, ft))
}

fn make_quad_face(
    model: &mut BRepModel,
    verts: &[Handle<VertexData>; 4],
    op: cadkernel_topology::OperationId,
    edge_idx: &mut u32,
    face_idx: u32,
) -> KernelResult<Handle<FaceData>> {
    let mut hes = Vec::with_capacity(4);
    for i in 0..4 {
        let j = (i + 1) % 4;
        let etag = Tag::generated(EntityKind::Edge, op, *edge_idx);
        let (_, he, _) = model.add_edge_tagged(verts[i], verts[j], etag);
        hes.push(he);
        *edge_idx += 1;
    }
    let lp = model.make_loop(&hes)?;
    let ft = Tag::generated(EntityKind::Face, op, face_idx);
    Ok(model.make_face_tagged(lp, ft))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loft_two_squares() {
        let mut model = BRepModel::new();
        let bottom = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let top = vec![
            Point3::new(0.5, 0.5, 5.0),
            Point3::new(1.5, 0.5, 5.0),
            Point3::new(1.5, 1.5, 5.0),
            Point3::new(0.5, 1.5, 5.0),
        ];

        let r = loft(&mut model, &[&bottom, &top]).unwrap();
        // 2 rings * 4 verts = 8 vertices
        assert_eq!(model.vertices.len(), 8);
        // 1 bottom + 1 top + 1*4 side = 6 faces
        assert_eq!(r.faces.len(), 6);
        assert_eq!(model.solids.len(), 1);
    }

    #[test]
    fn test_loft_three_profiles() {
        let mut model = BRepModel::new();
        let p0: Vec<Point3> = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(2.0, 2.0, 0.0),
            Point3::new(0.0, 2.0, 0.0),
        ];
        let p1: Vec<Point3> = vec![
            Point3::new(0.5, 0.5, 3.0),
            Point3::new(1.5, 0.5, 3.0),
            Point3::new(1.5, 1.5, 3.0),
            Point3::new(0.5, 1.5, 3.0),
        ];
        let p2: Vec<Point3> = vec![
            Point3::new(0.0, 0.0, 6.0),
            Point3::new(2.0, 0.0, 6.0),
            Point3::new(2.0, 2.0, 6.0),
            Point3::new(0.0, 2.0, 6.0),
        ];

        let r = loft(&mut model, &[p0.as_slice(), p1.as_slice(), p2.as_slice()]).unwrap();
        // 3 rings * 4 = 12 verts
        assert_eq!(model.vertices.len(), 12);
        // 1 bottom + 1 top + 2*4 side = 10 faces
        assert_eq!(r.faces.len(), 10);
    }

    #[test]
    fn test_loft_validation() {
        let mut model = BRepModel::new();
        // Too few profiles
        let p = vec![Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0), Point3::new(0.5, 1.0, 0.0)];
        assert!(loft(&mut model, &[&p]).is_err());

        // Mismatched point counts
        let a = vec![Point3::ORIGIN, Point3::new(1.0, 0.0, 0.0), Point3::new(0.5, 1.0, 0.0)];
        let b = vec![
            Point3::new(0.0, 0.0, 5.0),
            Point3::new(1.0, 0.0, 5.0),
            Point3::new(1.0, 1.0, 5.0),
            Point3::new(0.0, 1.0, 5.0),
        ];
        assert!(loft(&mut model, &[&a, &b]).is_err());
    }
}
