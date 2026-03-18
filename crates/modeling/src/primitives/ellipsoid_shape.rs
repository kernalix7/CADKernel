//! Ellipsoid primitive.

use std::f64::consts::{FRAC_PI_2, TAU};

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::Point3;
use cadkernel_topology::{BRepModel, EntityKind, FaceData, Handle, SolidData, Tag, VertexData};

use super::{EdgeCache, bind_edge_line_segments, next_edge_tag};

#[derive(Debug)]
pub struct EllipsoidResult {
    pub south_pole: Handle<VertexData>,
    pub north_pole: Handle<VertexData>,
    pub faces: Vec<Handle<FaceData>>,
    pub solid: Handle<SolidData>,
}

/// Creates an ellipsoid as a B-Rep solid.
///
/// `rx`, `ry`, `rz` are the semi-axis lengths along X, Y, Z.
/// When all three are equal, produces a sphere.
pub fn make_ellipsoid(
    model: &mut BRepModel,
    center: Point3,
    rx: f64,
    ry: f64,
    rz: f64,
    segments: usize,
    rings: usize,
) -> KernelResult<EllipsoidResult> {
    if segments < 3 {
        return Err(KernelError::InvalidArgument(
            "ellipsoid needs at least 3 segments".into(),
        ));
    }
    if rings < 2 {
        return Err(KernelError::InvalidArgument(
            "ellipsoid needs at least 2 rings".into(),
        ));
    }
    if rx <= 0.0 || ry <= 0.0 || rz <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "semi-axes must be positive".into(),
        ));
    }

    let op = model.history.next_operation("make_ellipsoid");
    let mut ec = EdgeCache::new();
    let mut edge_idx = 0u32;
    let mut vert_idx = 0u32;

    // South pole
    let sp_tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
    vert_idx += 1;
    let south_pole = model.add_vertex_tagged(
        Point3::new(center.x, center.y, center.z - rz),
        sp_tag,
    );

    // North pole
    let np_tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
    vert_idx += 1;
    let north_pole = model.add_vertex_tagged(
        Point3::new(center.x, center.y, center.z + rz),
        np_tag,
    );

    // Ring vertices (rings-1 rings)
    let mut ring_verts: Vec<Vec<Handle<VertexData>>> = Vec::with_capacity(rings - 1);
    for r in 1..rings {
        let phi = std::f64::consts::PI * r as f64 / rings as f64 - FRAC_PI_2;
        let cp = phi.cos();
        let sp_val = phi.sin();
        let mut ring = Vec::with_capacity(segments);
        for s in 0..segments {
            let theta = TAU * s as f64 / segments as f64;
            let (st, ct) = theta.sin_cos();
            let p = Point3::new(
                center.x + rx * cp * ct,
                center.y + ry * cp * st,
                center.z + rz * sp_val,
            );
            let tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
            vert_idx += 1;
            ring.push(model.add_vertex_tagged(p, tag));
        }
        ring_verts.push(ring);
    }

    let mut faces = Vec::new();
    let mut face_idx = 0u32;

    // South cap (triangles)
    for s in 0..segments {
        let s_next = (s + 1) % segments;
        let hes = [
            ec.get_or_create(model, ring_verts[0][s_next], ring_verts[0][s], next_edge_tag(op, &mut edge_idx)),
            ec.get_or_create(model, ring_verts[0][s], south_pole, next_edge_tag(op, &mut edge_idx)),
            ec.get_or_create(model, south_pole, ring_verts[0][s_next], next_edge_tag(op, &mut edge_idx)),
        ];
        let loop_h = model.make_loop(&hes)?;
        let tag = Tag::generated(EntityKind::Face, op, face_idx);
        faces.push(model.make_face_tagged(loop_h, tag));
        face_idx += 1;
    }

    // Middle bands (quads)
    for r in 0..ring_verts.len() - 1 {
        for s in 0..segments {
            let s_next = (s + 1) % segments;
            let hes = [
                ec.get_or_create(model, ring_verts[r][s], ring_verts[r][s_next], next_edge_tag(op, &mut edge_idx)),
                ec.get_or_create(model, ring_verts[r][s_next], ring_verts[r + 1][s_next], next_edge_tag(op, &mut edge_idx)),
                ec.get_or_create(model, ring_verts[r + 1][s_next], ring_verts[r + 1][s], next_edge_tag(op, &mut edge_idx)),
                ec.get_or_create(model, ring_verts[r + 1][s], ring_verts[r][s], next_edge_tag(op, &mut edge_idx)),
            ];
            let loop_h = model.make_loop(&hes)?;
            let tag = Tag::generated(EntityKind::Face, op, face_idx);
            faces.push(model.make_face_tagged(loop_h, tag));
            face_idx += 1;
        }
    }

    // North cap (triangles)
    let last_ring = ring_verts.len() - 1;
    for s in 0..segments {
        let s_next = (s + 1) % segments;
        let hes = [
            ec.get_or_create(model, ring_verts[last_ring][s], ring_verts[last_ring][s_next], next_edge_tag(op, &mut edge_idx)),
            ec.get_or_create(model, ring_verts[last_ring][s_next], north_pole, next_edge_tag(op, &mut edge_idx)),
            ec.get_or_create(model, north_pole, ring_verts[last_ring][s], next_edge_tag(op, &mut edge_idx)),
        ];
        let loop_h = model.make_loop(&hes)?;
        let tag = Tag::generated(EntityKind::Face, op, face_idx);
        faces.push(model.make_face_tagged(loop_h, tag));
        face_idx += 1;
    }
    let _ = face_idx;

    bind_edge_line_segments(model, &ec);

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&faces, shell_tag);
    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let solid = model.make_solid_tagged(&[shell], solid_tag);

    Ok(EllipsoidResult {
        south_pole,
        north_pole,
        faces,
        solid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ellipsoid_entity_counts() {
        let mut model = BRepModel::new();
        let r = make_ellipsoid(&mut model, Point3::ORIGIN, 2.0, 1.5, 1.0, 8, 4).unwrap();
        // 2 poles + (rings-1)*segments ring verts = 2 + 3*8 = 26
        // faces: 2*segments(caps) + (rings-2)*segments(middle) = 16 + 16 = 32
        assert_eq!(r.faces.len(), 32);
        assert!(model.solids.is_alive(r.solid));
    }

    #[test]
    fn test_ellipsoid_sphere_case() {
        let mut model = BRepModel::new();
        let r = make_ellipsoid(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0, 6, 3).unwrap();
        // Same topology as sphere
        assert_eq!(r.faces.len(), 18); // 6 + 6 + 6
        assert!(model.solids.is_alive(r.solid));
    }

    #[test]
    fn test_ellipsoid_validation() {
        let mut model = BRepModel::new();
        assert!(make_ellipsoid(&mut model, Point3::ORIGIN, -1.0, 1.0, 1.0, 6, 3).is_err());
        assert!(make_ellipsoid(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0, 2, 3).is_err());
        assert!(make_ellipsoid(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0, 6, 1).is_err());
    }
}
