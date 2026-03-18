//! Sweep operation: sweeps a profile along a path curve.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, EntityKind, FaceData, Handle, SolidData, Tag, VertexData};

/// Result of a sweep operation.
#[derive(Debug)]
pub struct SweepResult {
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Sweeps a planar profile along a 3D path to produce a solid.
///
/// `profile` -- a closed polyline (the cross section, in local XY coordinates).
/// `path`    -- an open polyline defining the sweep trajectory (at least 2 points).
///
/// The profile is oriented perpendicular to the path using a Frenet-like frame.
pub fn sweep(
    model: &mut BRepModel,
    profile: &[Point3],
    path: &[Point3],
) -> KernelResult<SweepResult> {
    let np = profile.len();
    let ns = path.len();
    if np < 3 {
        return Err(KernelError::InvalidArgument(
            "sweep profile needs at least 3 points".into(),
        ));
    }
    if ns < 2 {
        return Err(KernelError::InvalidArgument(
            "sweep path needs at least 2 points".into(),
        ));
    }

    let op = model.history.next_operation("sweep");

    // Build a frame at each path point and place the profile.
    let mut rings: Vec<Vec<Handle<VertexData>>> = Vec::with_capacity(ns);
    let mut vert_idx = 0u32;

    for si in 0..ns {
        let tangent = if si == 0 {
            (path[1] - path[0]).normalized().unwrap_or(Vec3::Z)
        } else if si == ns - 1 {
            (path[ns - 1] - path[ns - 2])
                .normalized()
                .unwrap_or(Vec3::Z)
        } else {
            let t1 = (path[si + 1] - path[si]).normalized().unwrap_or(Vec3::Z);
            let t0 = (path[si] - path[si - 1]).normalized().unwrap_or(Vec3::Z);
            (t1 + t0).normalized().unwrap_or(Vec3::Z)
        };

        // Local coordinate frame perpendicular to tangent
        let up_hint = if tangent.cross(Vec3::Z).length() > 1e-6 {
            Vec3::Z
        } else {
            Vec3::X
        };
        let binormal = tangent.cross(up_hint).normalized().unwrap_or(Vec3::X);
        let normal = binormal.cross(tangent).normalized().unwrap_or(Vec3::Y);

        let center = path[si];
        let mut ring = Vec::with_capacity(np);
        for &pp in profile {
            let pt = Point3::new(
                center.x + pp.x * binormal.x + pp.y * normal.x,
                center.y + pp.x * binormal.y + pp.y * normal.y,
                center.z + pp.x * binormal.z + pp.y * normal.z,
            );
            let tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
            ring.push(model.add_vertex_tagged(pt, tag));
            vert_idx += 1;
        }
        rings.push(ring);
    }

    let mut all_faces = Vec::new();
    let mut edge_idx = 0u32;
    let mut face_idx = 0u32;

    // Bottom cap (reversed winding)
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

    Ok(SweepResult {
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
    fn test_sweep_square_along_line() {
        let mut model = BRepModel::new();
        let profile = vec![
            Point3::new(-1.0, -1.0, 0.0),
            Point3::new(1.0, -1.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(-1.0, 1.0, 0.0),
        ];
        let path = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.0, 0.0, 5.0),
            Point3::new(0.0, 0.0, 10.0),
        ];

        let r = sweep(&mut model, &profile, &path).unwrap();
        // 3 rings * 4 verts = 12 vertices
        assert_eq!(model.vertices.len(), 12);
        // 1 bottom + 1 top + 2*4 side = 10 faces
        assert_eq!(r.faces.len(), 10);
        assert_eq!(model.solids.len(), 1);
    }

    #[test]
    fn test_sweep_validation() {
        let mut model = BRepModel::new();
        assert!(sweep(
            &mut model,
            &[Point3::ORIGIN, Point3::new(1.0, 0.0, 0.0)],
            &[Point3::ORIGIN, Point3::new(0.0, 0.0, 5.0)]
        )
        .is_err());
    }
}
