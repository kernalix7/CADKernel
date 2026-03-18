//! Regular polygon prism primitive.

use std::f64::consts::TAU;
use std::sync::Arc;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_geometry::Plane;
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, EntityKind, FaceData, Handle, Orientation, SolidData, Tag, VertexData};

use super::{EdgeCache, bind_edge_line_segments, next_edge_tag};

#[derive(Debug)]
pub struct PrismResult {
    pub vertices: Vec<Handle<VertexData>>,
    pub faces: Vec<Handle<FaceData>>,
    pub solid: Handle<SolidData>,
}

/// Creates a regular polygon prism (e.g., triangular, pentagonal, hexagonal).
///
/// `sides` = number of polygon sides (3 = triangular prism, 5 = pentagonal, etc.)
/// `radius` = circumscribed circle radius (center to vertex distance)
/// `height` = extrusion height along Z
pub fn make_prism(
    model: &mut BRepModel,
    base_center: Point3,
    radius: f64,
    height: f64,
    sides: usize,
) -> KernelResult<PrismResult> {
    if sides < 3 {
        return Err(KernelError::InvalidArgument(
            "prism needs at least 3 sides".into(),
        ));
    }
    if radius <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "radius must be positive".into(),
        ));
    }
    if height <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "height must be positive".into(),
        ));
    }

    let op = model.history.next_operation("make_prism");
    let n = sides;
    let mut ec = EdgeCache::new();
    let mut edge_idx = 0u32;
    let mut all_verts = Vec::new();

    let mut bot_verts = Vec::with_capacity(n);
    let mut top_verts = Vec::with_capacity(n);

    for i in 0..n {
        let angle = TAU * i as f64 / n as f64;
        let (s, c) = angle.sin_cos();

        let bp = Point3::new(
            base_center.x + radius * c,
            base_center.y + radius * s,
            base_center.z,
        );
        let tp = Point3::new(
            base_center.x + radius * c,
            base_center.y + radius * s,
            base_center.z + height,
        );

        let btag = Tag::generated(EntityKind::Vertex, op, (2 * i) as u32);
        let ttag = Tag::generated(EntityKind::Vertex, op, (2 * i + 1) as u32);
        let bv = model.add_vertex_tagged(bp, btag);
        let tv = model.add_vertex_tagged(tp, ttag);
        bot_verts.push(bv);
        top_verts.push(tv);
        all_verts.push(bv);
        all_verts.push(tv);
    }

    let mut faces = Vec::new();
    let mut face_idx = 0u32;

    // Bottom face (polygon, reversed winding for downward normal)
    {
        let mut hes = Vec::with_capacity(n);
        for i in (0..n).rev() {
            let j = if i == 0 { n - 1 } else { i - 1 };
            hes.push(ec.get_or_create(
                model,
                bot_verts[i],
                bot_verts[j],
                next_edge_tag(op, &mut edge_idx),
            ));
        }
        let loop_h = model.make_loop(&hes)?;
        let tag = Tag::generated(EntityKind::Face, op, face_idx);
        faces.push(model.make_face_tagged(loop_h, tag));
        face_idx += 1;
    }

    // Top face (polygon, forward winding for upward normal)
    {
        let mut hes = Vec::with_capacity(n);
        for i in 0..n {
            let j = (i + 1) % n;
            hes.push(ec.get_or_create(
                model,
                top_verts[i],
                top_verts[j],
                next_edge_tag(op, &mut edge_idx),
            ));
        }
        let loop_h = model.make_loop(&hes)?;
        let tag = Tag::generated(EntityKind::Face, op, face_idx);
        faces.push(model.make_face_tagged(loop_h, tag));
        face_idx += 1;
    }

    // Lateral faces (quads)
    for i in 0..n {
        let j = (i + 1) % n;
        let hes = [
            ec.get_or_create(model, bot_verts[i], bot_verts[j], next_edge_tag(op, &mut edge_idx)),
            ec.get_or_create(model, bot_verts[j], top_verts[j], next_edge_tag(op, &mut edge_idx)),
            ec.get_or_create(model, top_verts[j], top_verts[i], next_edge_tag(op, &mut edge_idx)),
            ec.get_or_create(model, top_verts[i], bot_verts[i], next_edge_tag(op, &mut edge_idx)),
        ];
        let loop_h = model.make_loop(&hes)?;
        let tag = Tag::generated(EntityKind::Face, op, face_idx);
        faces.push(model.make_face_tagged(loop_h, tag));
        face_idx += 1;
    }
    let _ = face_idx;

    // Geometry binding
    let bot_plane = Plane::new(base_center, Vec3::X, -Vec3::Y);
    if let Ok(bp) = bot_plane {
        model.bind_face_surface(faces[0], Arc::new(bp), Orientation::Forward);
    }
    let top_center = Point3::new(base_center.x, base_center.y, base_center.z + height);
    let top_plane = Plane::new(top_center, Vec3::X, Vec3::Y);
    if let Ok(tp) = top_plane {
        model.bind_face_surface(faces[1], Arc::new(tp), Orientation::Forward);
    }
    // Lateral faces bound to planes
    for i in 0..n {
        let j = (i + 1) % n;
        let p0 = model.vertices.get(bot_verts[i]).unwrap().point;
        let p1 = model.vertices.get(bot_verts[j]).unwrap().point;
        let edge_dir = (p1 - p0).normalized().unwrap_or(Vec3::X);
        let v_axis = Vec3::Z;
        let u_axis = v_axis.cross(edge_dir).normalized().unwrap_or(Vec3::X);
        if let Ok(pl) = Plane::new(p0, u_axis, v_axis) {
            model.bind_face_surface(faces[2 + i], Arc::new(pl), Orientation::Forward);
        }
    }

    bind_edge_line_segments(model, &ec);

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&faces, shell_tag);
    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let solid = model.make_solid_tagged(&[shell], solid_tag);

    Ok(PrismResult {
        vertices: all_verts,
        faces,
        solid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_triangular_prism() {
        let mut model = BRepModel::new();
        let r = make_prism(&mut model, Point3::ORIGIN, 1.0, 2.0, 3).unwrap();
        assert_eq!(r.vertices.len(), 6);
        assert_eq!(r.faces.len(), 5); // 2 caps + 3 lateral
        assert!(model.solids.is_alive(r.solid));
    }

    #[test]
    fn test_hexagonal_prism() {
        let mut model = BRepModel::new();
        let r = make_prism(&mut model, Point3::ORIGIN, 1.0, 3.0, 6).unwrap();
        assert_eq!(r.vertices.len(), 12);
        assert_eq!(r.faces.len(), 8); // 2 caps + 6 lateral
    }

    #[test]
    fn test_prism_validation() {
        let mut model = BRepModel::new();
        assert!(make_prism(&mut model, Point3::ORIGIN, 1.0, 2.0, 2).is_err());
        assert!(make_prism(&mut model, Point3::ORIGIN, -1.0, 2.0, 4).is_err());
        assert!(make_prism(&mut model, Point3::ORIGIN, 1.0, -2.0, 4).is_err());
    }
}
