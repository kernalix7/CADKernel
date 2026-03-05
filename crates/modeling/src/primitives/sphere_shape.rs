use std::f64::consts::{FRAC_PI_2, TAU};

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::Point3;
use cadkernel_topology::{BRepModel, EntityKind, Handle, Tag};

/// Creates a UV-sphere as a B-Rep solid.
///
/// `rings` controls the number of latitude divisions (min 2),
/// `segments` controls the number of longitude divisions (min 3).
///
/// Produces: 2 pole vertices + `(rings-1)*segments` ring vertices,
/// `rings*segments` quad/tri faces, 1 shell, 1 solid.
pub fn make_sphere(
    model: &mut BRepModel,
    center: Point3,
    radius: f64,
    segments: usize,
    rings: usize,
) -> KernelResult<SphereResult> {
    if segments < 3 {
        return Err(KernelError::InvalidArgument(
            "sphere needs at least 3 segments".into(),
        ));
    }
    if rings < 2 {
        return Err(KernelError::InvalidArgument(
            "sphere needs at least 2 rings".into(),
        ));
    }
    let op = model.history.next_operation("make_sphere");

    let mut vert_idx = 0u32;

    // South pole
    let south_tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
    let south_pole = model.add_vertex_tagged(
        Point3::new(center.x, center.y, center.z - radius),
        south_tag,
    );
    vert_idx += 1;

    // Ring vertices (from south to north, excluding poles)
    let mut ring_verts: Vec<Vec<Handle<cadkernel_topology::VertexData>>> = Vec::new();
    for r in 1..rings {
        let phi = -FRAC_PI_2 + std::f64::consts::PI * r as f64 / rings as f64;
        let (sin_phi, cos_phi) = phi.sin_cos();
        let mut ring = Vec::new();
        for s in 0..segments {
            let theta = TAU * s as f64 / segments as f64;
            let (sin_t, cos_t) = theta.sin_cos();
            let p = Point3::new(
                center.x + radius * cos_phi * cos_t,
                center.y + radius * cos_phi * sin_t,
                center.z + radius * sin_phi,
            );
            let tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
            ring.push(model.add_vertex_tagged(p, tag));
            vert_idx += 1;
        }
        ring_verts.push(ring);
    }

    // North pole
    let north_tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
    let north_pole = model.add_vertex_tagged(
        Point3::new(center.x, center.y, center.z + radius),
        north_tag,
    );

    let mut edge_idx = 0u32;
    let mut face_idx = 0u32;
    let mut all_faces = Vec::new();

    // South cap triangles
    for s in 0..segments {
        let next = (s + 1) % segments;
        let e1_tag = Tag::generated(EntityKind::Edge, op, edge_idx);
        let (_, he1, _) = model.add_edge_tagged(south_pole, ring_verts[0][s], e1_tag);
        edge_idx += 1;
        let e2_tag = Tag::generated(EntityKind::Edge, op, edge_idx);
        let (_, he2, _) = model.add_edge_tagged(ring_verts[0][s], ring_verts[0][next], e2_tag);
        edge_idx += 1;
        let e3_tag = Tag::generated(EntityKind::Edge, op, edge_idx);
        let (_, he3, _) = model.add_edge_tagged(ring_verts[0][next], south_pole, e3_tag);
        edge_idx += 1;

        let lp = model.make_loop(&[he1, he2, he3])?;
        let ft = Tag::generated(EntityKind::Face, op, face_idx);
        all_faces.push(model.make_face_tagged(lp, ft));
        face_idx += 1;
    }

    // Middle ring quads
    for r in 0..ring_verts.len() - 1 {
        for s in 0..segments {
            let next = (s + 1) % segments;
            let e1t = Tag::generated(EntityKind::Edge, op, edge_idx);
            let (_, he1, _) = model.add_edge_tagged(ring_verts[r][s], ring_verts[r][next], e1t);
            edge_idx += 1;
            let e2t = Tag::generated(EntityKind::Edge, op, edge_idx);
            let (_, he2, _) =
                model.add_edge_tagged(ring_verts[r][next], ring_verts[r + 1][next], e2t);
            edge_idx += 1;
            let e3t = Tag::generated(EntityKind::Edge, op, edge_idx);
            let (_, he3, _) =
                model.add_edge_tagged(ring_verts[r + 1][next], ring_verts[r + 1][s], e3t);
            edge_idx += 1;
            let e4t = Tag::generated(EntityKind::Edge, op, edge_idx);
            let (_, he4, _) = model.add_edge_tagged(ring_verts[r + 1][s], ring_verts[r][s], e4t);
            edge_idx += 1;

            let lp = model.make_loop(&[he1, he2, he3, he4])?;
            let ft = Tag::generated(EntityKind::Face, op, face_idx);
            all_faces.push(model.make_face_tagged(lp, ft));
            face_idx += 1;
        }
    }

    // North cap triangles
    let last_ring = ring_verts.len() - 1;
    for s in 0..segments {
        let next = (s + 1) % segments;
        let e1t = Tag::generated(EntityKind::Edge, op, edge_idx);
        let (_, he1, _) =
            model.add_edge_tagged(ring_verts[last_ring][s], ring_verts[last_ring][next], e1t);
        edge_idx += 1;
        let e2t = Tag::generated(EntityKind::Edge, op, edge_idx);
        let (_, he2, _) = model.add_edge_tagged(ring_verts[last_ring][next], north_pole, e2t);
        edge_idx += 1;
        let e3t = Tag::generated(EntityKind::Edge, op, edge_idx);
        let (_, he3, _) = model.add_edge_tagged(north_pole, ring_verts[last_ring][s], e3t);
        edge_idx += 1;

        let lp = model.make_loop(&[he1, he2, he3])?;
        let ft = Tag::generated(EntityKind::Face, op, face_idx);
        all_faces.push(model.make_face_tagged(lp, ft));
        face_idx += 1;
    }

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&all_faces, shell_tag);
    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let solid = model.make_solid_tagged(&[shell], solid_tag);

    Ok(SphereResult {
        south_pole,
        north_pole,
        faces: all_faces,
        shell,
        solid,
    })
}

/// Handles returned from [`make_sphere`].
pub struct SphereResult {
    pub south_pole: Handle<cadkernel_topology::VertexData>,
    pub north_pole: Handle<cadkernel_topology::VertexData>,
    pub faces: Vec<Handle<cadkernel_topology::FaceData>>,
    pub shell: Handle<cadkernel_topology::ShellData>,
    pub solid: Handle<cadkernel_topology::SolidData>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sphere_entity_counts() {
        let mut model = BRepModel::new();
        let r = make_sphere(&mut model, Point3::ORIGIN, 1.0, 8, 4).unwrap();
        // 2 poles + 3 rings * 8 segments = 26 vertices
        assert_eq!(model.vertices.len(), 26);
        // 8 south tri + 2*8 middle quads + 8 north tri = 32 faces
        assert_eq!(r.faces.len(), 32);
        assert_eq!(model.shells.len(), 1);
        assert_eq!(model.solids.len(), 1);
    }

    #[test]
    fn test_sphere_poles_position() {
        let mut model = BRepModel::new();
        let r = make_sphere(&mut model, Point3::new(1.0, 2.0, 3.0), 5.0, 6, 3).unwrap();
        let south = model.vertices.get(r.south_pole).unwrap();
        assert!(south.point.approx_eq(Point3::new(1.0, 2.0, -2.0)));
        let north = model.vertices.get(r.north_pole).unwrap();
        assert!(north.point.approx_eq(Point3::new(1.0, 2.0, 8.0)));
    }
}
