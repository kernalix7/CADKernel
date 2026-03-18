//! Tube (hollow cylinder) primitive.

use std::f64::consts::TAU;
use std::sync::Arc;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_geometry::{Cylinder as CylSurface, Plane};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, EntityKind, FaceData, Handle, Orientation, SolidData, Tag, VertexData};

use super::{EdgeCache, bind_edge_line_segments, next_edge_tag};

#[derive(Debug)]
pub struct TubeResult {
    pub vertices: Vec<Handle<VertexData>>,
    pub faces: Vec<Handle<FaceData>>,
    pub solid: Handle<SolidData>,
}

/// Creates a tube (hollow cylinder) as a B-Rep solid.
///
/// `outer_radius` and `inner_radius` define the wall thickness.
/// The tube is aligned along Z with its base at `base_center`.
pub fn make_tube(
    model: &mut BRepModel,
    base_center: Point3,
    outer_radius: f64,
    inner_radius: f64,
    height: f64,
    segments: usize,
) -> KernelResult<TubeResult> {
    if segments < 3 {
        return Err(KernelError::InvalidArgument(
            "tube needs at least 3 segments".into(),
        ));
    }
    if outer_radius <= 0.0 || inner_radius <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "radii must be positive".into(),
        ));
    }
    if inner_radius >= outer_radius {
        return Err(KernelError::InvalidArgument(
            "inner_radius must be less than outer_radius".into(),
        ));
    }
    if height <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "height must be positive".into(),
        ));
    }

    let op = model.history.next_operation("make_tube");
    let n = segments;
    let mut ec = EdgeCache::new();
    let mut edge_idx = 0u32;
    let mut vert_idx = 0u32;
    let mut all_verts = Vec::new();

    // Create 4 rings of vertices: outer bottom, outer top, inner bottom, inner top
    let mut outer_bot = Vec::with_capacity(n);
    let mut outer_top = Vec::with_capacity(n);
    let mut inner_bot = Vec::with_capacity(n);
    let mut inner_top = Vec::with_capacity(n);

    for i in 0..n {
        let angle = TAU * i as f64 / n as f64;
        let (s, c) = angle.sin_cos();

        for (ring, r, dz) in [
            (&mut outer_bot, outer_radius, 0.0),
            (&mut outer_top, outer_radius, height),
            (&mut inner_bot, inner_radius, 0.0),
            (&mut inner_top, inner_radius, height),
        ] {
            let p = Point3::new(base_center.x + r * c, base_center.y + r * s, base_center.z + dz);
            let tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
            vert_idx += 1;
            let vh = model.add_vertex_tagged(p, tag);
            ring.push(vh);
            all_verts.push(vh);
        }
    }

    let mut faces = Vec::new();
    let mut face_idx = 0u32;

    // Outer lateral faces (quads)
    for i in 0..n {
        let j = (i + 1) % n;
        let hes = [
            ec.get_or_create(model, outer_bot[i], outer_bot[j], next_edge_tag(op, &mut edge_idx)),
            ec.get_or_create(model, outer_bot[j], outer_top[j], next_edge_tag(op, &mut edge_idx)),
            ec.get_or_create(model, outer_top[j], outer_top[i], next_edge_tag(op, &mut edge_idx)),
            ec.get_or_create(model, outer_top[i], outer_bot[i], next_edge_tag(op, &mut edge_idx)),
        ];
        let loop_h = model.make_loop(&hes)?;
        let tag = Tag::generated(EntityKind::Face, op, face_idx);
        faces.push(model.make_face_tagged(loop_h, tag));
        face_idx += 1;
    }

    // Inner lateral faces (quads, reversed winding for inward normal)
    for i in 0..n {
        let j = (i + 1) % n;
        let hes = [
            ec.get_or_create(model, inner_bot[j], inner_bot[i], next_edge_tag(op, &mut edge_idx)),
            ec.get_or_create(model, inner_bot[i], inner_top[i], next_edge_tag(op, &mut edge_idx)),
            ec.get_or_create(model, inner_top[i], inner_top[j], next_edge_tag(op, &mut edge_idx)),
            ec.get_or_create(model, inner_top[j], inner_bot[j], next_edge_tag(op, &mut edge_idx)),
        ];
        let loop_h = model.make_loop(&hes)?;
        let tag = Tag::generated(EntityKind::Face, op, face_idx);
        faces.push(model.make_face_tagged(loop_h, tag));
        face_idx += 1;
    }

    // Bottom annular face (outer CCW, inner CW → quads connecting outer and inner)
    for i in 0..n {
        let j = (i + 1) % n;
        let hes = [
            ec.get_or_create(model, outer_bot[j], outer_bot[i], next_edge_tag(op, &mut edge_idx)),
            ec.get_or_create(model, outer_bot[i], inner_bot[i], next_edge_tag(op, &mut edge_idx)),
            ec.get_or_create(model, inner_bot[i], inner_bot[j], next_edge_tag(op, &mut edge_idx)),
            ec.get_or_create(model, inner_bot[j], outer_bot[j], next_edge_tag(op, &mut edge_idx)),
        ];
        let loop_h = model.make_loop(&hes)?;
        let tag = Tag::generated(EntityKind::Face, op, face_idx);
        faces.push(model.make_face_tagged(loop_h, tag));
        face_idx += 1;
    }

    // Top annular face
    for i in 0..n {
        let j = (i + 1) % n;
        let hes = [
            ec.get_or_create(model, outer_top[i], outer_top[j], next_edge_tag(op, &mut edge_idx)),
            ec.get_or_create(model, outer_top[j], inner_top[j], next_edge_tag(op, &mut edge_idx)),
            ec.get_or_create(model, inner_top[j], inner_top[i], next_edge_tag(op, &mut edge_idx)),
            ec.get_or_create(model, inner_top[i], outer_top[i], next_edge_tag(op, &mut edge_idx)),
        ];
        let loop_h = model.make_loop(&hes)?;
        let tag = Tag::generated(EntityKind::Face, op, face_idx);
        faces.push(model.make_face_tagged(loop_h, tag));
        face_idx += 1;
    }
    let _ = face_idx;

    // Geometry binding
    let axis = Vec3::Z;
    if let Ok(outer_surf) = CylSurface::new(base_center, axis, outer_radius, height) {
        let surf = Arc::new(outer_surf);
        for face in &faces[..n] {
            model.bind_face_surface(*face, surf.clone(), Orientation::Forward);
        }
    }
    if let Ok(inner_surf) = CylSurface::new(base_center, axis, inner_radius, height) {
        let surf = Arc::new(inner_surf);
        for face in &faces[n..2 * n] {
            model.bind_face_surface(*face, surf.clone(), Orientation::Reversed);
        }
    }
    let bot_plane = Plane::new(base_center, Vec3::X, -Vec3::Y);
    if let Ok(bp) = bot_plane {
        let surf = Arc::new(bp);
        for face in &faces[2 * n..3 * n] {
            model.bind_face_surface(*face, surf.clone(), Orientation::Forward);
        }
    }
    let top_center = Point3::new(base_center.x, base_center.y, base_center.z + height);
    let top_plane = Plane::new(top_center, Vec3::X, Vec3::Y);
    if let Ok(tp) = top_plane {
        let surf = Arc::new(tp);
        for face in &faces[3 * n..] {
            model.bind_face_surface(*face, surf.clone(), Orientation::Forward);
        }
    }

    bind_edge_line_segments(model, &ec);

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&faces, shell_tag);
    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let solid = model.make_solid_tagged(&[shell], solid_tag);

    Ok(TubeResult {
        vertices: all_verts,
        faces,
        solid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tube_entity_counts() {
        let mut model = BRepModel::new();
        let r = make_tube(&mut model, Point3::ORIGIN, 2.0, 1.0, 3.0, 8).unwrap();
        // 4*N vertices, 4*N faces (outer lateral + inner lateral + bottom annulus + top annulus)
        assert_eq!(r.vertices.len(), 32);
        assert_eq!(r.faces.len(), 32);
        assert!(model.solids.is_alive(r.solid));
    }

    #[test]
    fn test_tube_validation() {
        let mut model = BRepModel::new();
        assert!(make_tube(&mut model, Point3::ORIGIN, 1.0, 1.5, 3.0, 8).is_err());
        assert!(make_tube(&mut model, Point3::ORIGIN, 2.0, 1.0, -1.0, 8).is_err());
        assert!(make_tube(&mut model, Point3::ORIGIN, 2.0, 1.0, 3.0, 2).is_err());
    }
}
