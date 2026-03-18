//! Wedge (tapered box) primitive.

use std::sync::Arc;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_geometry::Plane;
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, EntityKind, FaceData, Handle, Orientation, SolidData, Tag, VertexData};

use super::{EdgeCache, bind_edge_line_segments, next_edge_tag};

#[derive(Debug)]
pub struct WedgeResult {
    pub vertices: Vec<Handle<VertexData>>,
    pub faces: Vec<Handle<FaceData>>,
    pub solid: Handle<SolidData>,
}

/// Parameters for wedge construction.
#[derive(Debug, Clone)]
pub struct WedgeParams {
    pub origin: Point3,
    pub dx: f64,
    pub dy: f64,
    pub dz: f64,
    pub dx2: f64,
    pub dy2: f64,
    pub xoff: f64,
    pub yoff: f64,
}

/// Creates a wedge (tapered box).
///
/// The bottom face is `dx × dy` at `origin`.
/// The top face is `dx2 × dy2` at `origin + (0,0,dz)`, offset by `(xoff, yoff)`.
/// When `dx2 = 0` and `dy2 = 0`, creates a pyramid (4 faces converge to a point).
#[allow(clippy::too_many_arguments)]
pub fn make_wedge(
    model: &mut BRepModel,
    origin: Point3,
    dx: f64,
    dy: f64,
    dz: f64,
    dx2: f64,
    dy2: f64,
    xoff: f64,
    yoff: f64,
) -> KernelResult<WedgeResult> {
    if dx <= 0.0 || dy <= 0.0 || dz <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "wedge base dimensions must be positive".into(),
        ));
    }
    if dx2 < 0.0 || dy2 < 0.0 {
        return Err(KernelError::InvalidArgument(
            "top dimensions must be non-negative".into(),
        ));
    }

    let op = model.history.next_operation("make_wedge");
    let mut ec = EdgeCache::new();
    let mut edge_idx = 0u32;

    let x0 = origin.x;
    let y0 = origin.y;
    let z0 = origin.z;

    // Bottom 4 vertices
    let bot = [
        Point3::new(x0, y0, z0),
        Point3::new(x0 + dx, y0, z0),
        Point3::new(x0 + dx, y0 + dy, z0),
        Point3::new(x0, y0 + dy, z0),
    ];

    let is_point = dx2 < 1e-14 && dy2 < 1e-14;

    let mut all_verts = Vec::new();
    let mut vert_idx = 0u32;

    let mut bot_v = Vec::with_capacity(4);
    for &p in &bot {
        let tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
        vert_idx += 1;
        let vh = model.add_vertex_tagged(p, tag);
        bot_v.push(vh);
        all_verts.push(vh);
    }

    let mut faces = Vec::new();
    let mut face_idx = 0u32;

    if is_point {
        // Pyramid: 4 bottom vertices + 1 apex
        let apex = Point3::new(x0 + dx * 0.5 + xoff, y0 + dy * 0.5 + yoff, z0 + dz);
        let tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
        let apex_v = model.add_vertex_tagged(apex, tag);
        all_verts.push(apex_v);

        // Bottom face (reversed)
        let hes = [
            ec.get_or_create(model, bot_v[3], bot_v[2], next_edge_tag(op, &mut edge_idx)),
            ec.get_or_create(model, bot_v[2], bot_v[1], next_edge_tag(op, &mut edge_idx)),
            ec.get_or_create(model, bot_v[1], bot_v[0], next_edge_tag(op, &mut edge_idx)),
            ec.get_or_create(model, bot_v[0], bot_v[3], next_edge_tag(op, &mut edge_idx)),
        ];
        let loop_h = model.make_loop(&hes)?;
        let tag = Tag::generated(EntityKind::Face, op, face_idx);
        faces.push(model.make_face_tagged(loop_h, tag));
        face_idx += 1;

        // 4 triangular faces
        for i in 0..4 {
            let j = (i + 1) % 4;
            let hes = [
                ec.get_or_create(model, bot_v[i], bot_v[j], next_edge_tag(op, &mut edge_idx)),
                ec.get_or_create(model, bot_v[j], apex_v, next_edge_tag(op, &mut edge_idx)),
                ec.get_or_create(model, apex_v, bot_v[i], next_edge_tag(op, &mut edge_idx)),
            ];
            let loop_h = model.make_loop(&hes)?;
            let tag = Tag::generated(EntityKind::Face, op, face_idx);
            faces.push(model.make_face_tagged(loop_h, tag));
            face_idx += 1;
        }
    } else {
        // Full wedge: 4 bottom + 4 top vertices
        let top = [
            Point3::new(x0 + xoff, y0 + yoff, z0 + dz),
            Point3::new(x0 + xoff + dx2, y0 + yoff, z0 + dz),
            Point3::new(x0 + xoff + dx2, y0 + yoff + dy2, z0 + dz),
            Point3::new(x0 + xoff, y0 + yoff + dy2, z0 + dz),
        ];

        let mut top_v = Vec::with_capacity(4);
        for &p in &top {
            let tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
            vert_idx += 1;
            let vh = model.add_vertex_tagged(p, tag);
            top_v.push(vh);
            all_verts.push(vh);
        }

        // Bottom face (reversed)
        let hes = [
            ec.get_or_create(model, bot_v[3], bot_v[2], next_edge_tag(op, &mut edge_idx)),
            ec.get_or_create(model, bot_v[2], bot_v[1], next_edge_tag(op, &mut edge_idx)),
            ec.get_or_create(model, bot_v[1], bot_v[0], next_edge_tag(op, &mut edge_idx)),
            ec.get_or_create(model, bot_v[0], bot_v[3], next_edge_tag(op, &mut edge_idx)),
        ];
        let loop_h = model.make_loop(&hes)?;
        let tag = Tag::generated(EntityKind::Face, op, face_idx);
        faces.push(model.make_face_tagged(loop_h, tag));
        face_idx += 1;

        // Top face (forward)
        let hes = [
            ec.get_or_create(model, top_v[0], top_v[1], next_edge_tag(op, &mut edge_idx)),
            ec.get_or_create(model, top_v[1], top_v[2], next_edge_tag(op, &mut edge_idx)),
            ec.get_or_create(model, top_v[2], top_v[3], next_edge_tag(op, &mut edge_idx)),
            ec.get_or_create(model, top_v[3], top_v[0], next_edge_tag(op, &mut edge_idx)),
        ];
        let loop_h = model.make_loop(&hes)?;
        let tag = Tag::generated(EntityKind::Face, op, face_idx);
        faces.push(model.make_face_tagged(loop_h, tag));
        face_idx += 1;

        // 4 lateral faces (quads)
        for i in 0..4 {
            let j = (i + 1) % 4;
            let hes = [
                ec.get_or_create(model, bot_v[i], bot_v[j], next_edge_tag(op, &mut edge_idx)),
                ec.get_or_create(model, bot_v[j], top_v[j], next_edge_tag(op, &mut edge_idx)),
                ec.get_or_create(model, top_v[j], top_v[i], next_edge_tag(op, &mut edge_idx)),
                ec.get_or_create(model, top_v[i], bot_v[i], next_edge_tag(op, &mut edge_idx)),
            ];
            let loop_h = model.make_loop(&hes)?;
            let tag = Tag::generated(EntityKind::Face, op, face_idx);
            faces.push(model.make_face_tagged(loop_h, tag));
            face_idx += 1;
        }
    }
    let _ = face_idx;
    let _ = vert_idx;

    // Geometry binding: bottom and top as planes
    if let Ok(bp) = Plane::new(origin, Vec3::X, -Vec3::Y) {
        model.bind_face_surface(faces[0], Arc::new(bp), Orientation::Forward);
    }
    if !is_point && faces.len() > 1 {
        let top_center = Point3::new(x0 + xoff + dx2 * 0.5, y0 + yoff + dy2 * 0.5, z0 + dz);
        if let Ok(tp) = Plane::new(top_center, Vec3::X, Vec3::Y) {
            model.bind_face_surface(faces[1], Arc::new(tp), Orientation::Forward);
        }
    }

    bind_edge_line_segments(model, &ec);

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&faces, shell_tag);
    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let solid = model.make_solid_tagged(&[shell], solid_tag);

    Ok(WedgeResult {
        vertices: all_verts,
        faces,
        solid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wedge_box_equivalent() {
        let mut model = BRepModel::new();
        let r = make_wedge(&mut model, Point3::ORIGIN, 2.0, 3.0, 4.0, 2.0, 3.0, 0.0, 0.0).unwrap();
        assert_eq!(r.vertices.len(), 8);
        assert_eq!(r.faces.len(), 6);
        assert!(model.solids.is_alive(r.solid));
    }

    #[test]
    fn test_wedge_pyramid() {
        let mut model = BRepModel::new();
        let r = make_wedge(&mut model, Point3::ORIGIN, 2.0, 2.0, 3.0, 0.0, 0.0, 0.0, 0.0).unwrap();
        assert_eq!(r.vertices.len(), 5); // 4 base + 1 apex
        assert_eq!(r.faces.len(), 5); // 1 base + 4 triangles
    }

    #[test]
    fn test_wedge_tapered() {
        let mut model = BRepModel::new();
        let r = make_wedge(&mut model, Point3::ORIGIN, 4.0, 4.0, 3.0, 2.0, 2.0, 1.0, 1.0).unwrap();
        assert_eq!(r.vertices.len(), 8);
        assert_eq!(r.faces.len(), 6);
    }

    #[test]
    fn test_wedge_validation() {
        let mut model = BRepModel::new();
        assert!(make_wedge(&mut model, Point3::ORIGIN, -1.0, 2.0, 3.0, 1.0, 1.0, 0.0, 0.0).is_err());
        assert!(make_wedge(&mut model, Point3::ORIGIN, 2.0, 2.0, 3.0, -1.0, 1.0, 0.0, 0.0).is_err());
    }
}
