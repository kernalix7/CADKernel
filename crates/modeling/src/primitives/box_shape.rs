use std::sync::Arc;

use cadkernel_core::KernelResult;
use cadkernel_geometry::Plane;
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, EntityKind, Handle, Orientation, Tag, VertexData};

use super::{EdgeCache, bind_edge_line_segments, next_edge_tag};

/// Creates a box (rectangular cuboid) as a fully connected B-Rep solid.
///
/// The box is axis-aligned with one corner at `origin` and extends by
/// `(dx, dy, dz)` in the positive direction.
///
/// Produces: 8 vertices, 12 edges, 6 quad faces, 1 shell, 1 solid.
/// All entities are tagged for persistent naming.
/// All faces are bound to `Plane` surfaces; all edges to `LineSegment` curves.
pub fn make_box(
    model: &mut BRepModel,
    origin: Point3,
    dx: f64,
    dy: f64,
    dz: f64,
) -> KernelResult<BoxResult> {
    let op = model.history.next_operation("make_box");

    let o = origin;
    let pts = [
        Point3::new(o.x, o.y, o.z),                // 0: ---
        Point3::new(o.x + dx, o.y, o.z),           // 1: +--
        Point3::new(o.x + dx, o.y + dy, o.z),      // 2: ++-
        Point3::new(o.x, o.y + dy, o.z),           // 3: -+-
        Point3::new(o.x, o.y, o.z + dz),           // 4: --+
        Point3::new(o.x + dx, o.y, o.z + dz),      // 5: +-+
        Point3::new(o.x + dx, o.y + dy, o.z + dz), // 6: +++
        Point3::new(o.x, o.y + dy, o.z + dz),      // 7: -++
    ];

    let mut v = Vec::new();
    for (i, &pt) in pts.iter().enumerate() {
        let tag = Tag::generated(EntityKind::Vertex, op, i as u32);
        v.push(model.add_vertex_tagged(pt, tag));
    }

    // 6 faces defined by their 4 vertex indices (CCW from outside)
    // Face order: bottom(0), top(1), front(2), back(3), left(4), right(5)
    let face_defs: [(u32, [usize; 4]); 6] = [
        (0, [0, 3, 2, 1]), // bottom (z=0), normal -Z
        (1, [4, 5, 6, 7]), // top (z=dz), normal +Z
        (2, [0, 1, 5, 4]), // front (y=0), normal -Y
        (3, [2, 3, 7, 6]), // back (y=dy), normal +Y
        (4, [0, 4, 7, 3]), // left (x=0), normal -X
        (5, [1, 2, 6, 5]), // right (x=dx), normal +X
    ];

    let mut face_handles = Vec::new();
    let mut edge_idx = 0u32;
    let mut edge_cache = EdgeCache::new();

    for &(face_local_idx, ref verts) in &face_defs {
        let n = verts.len();
        let mut half_edges = Vec::new();

        for i in 0..n {
            let vs = v[verts[i]];
            let ve = v[verts[(i + 1) % n]];
            let tag = next_edge_tag(op, &mut edge_idx);
            let he = edge_cache.get_or_create(model, vs, ve, tag);
            half_edges.push(he);
        }

        let loop_h = model.make_loop(&half_edges)?;
        let face_tag = Tag::generated(EntityKind::Face, op, face_local_idx);
        let face_h = model.make_face_tagged(loop_h, face_tag);
        face_handles.push(face_h);
    }

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&face_handles, shell_tag);

    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let solid = model.make_solid_tagged(&[shell], solid_tag);

    // --- Geometry binding ---
    // Plane u_axis × v_axis = outward face normal
    let face_plane_defs: [(Point3, Vec3, Vec3); 6] = [
        (o, Vec3::X, -Vec3::Y),                       // bottom: normal -Z
        (o + Vec3::Z * dz, Vec3::X, Vec3::Y),         // top:    normal +Z
        (o, Vec3::X, Vec3::Z),                         // front:  normal -Y
        (o + Vec3::Y * dy, Vec3::X, -Vec3::Z),        // back:   normal +Y
        (o, Vec3::Y, -Vec3::Z),                        // left:   normal -X
        (o + Vec3::X * dx, Vec3::Y, Vec3::Z),         // right:  normal +X
    ];
    for (face_h, &(fo, u_ax, v_ax)) in face_handles.iter().zip(face_plane_defs.iter()) {
        let plane = Plane::new(fo, u_ax, v_ax)?;
        model.bind_face_surface(*face_h, Arc::new(plane), Orientation::Forward);
    }
    bind_edge_line_segments(model, &edge_cache);

    Ok(BoxResult {
        vertices: v,
        faces: face_handles,
        shell,
        solid,
    })
}

/// Handles returned from [`make_box`].
pub struct BoxResult {
    pub vertices: Vec<Handle<VertexData>>,
    pub faces: Vec<Handle<cadkernel_topology::FaceData>>,
    pub shell: Handle<cadkernel_topology::ShellData>,
    pub solid: Handle<cadkernel_topology::SolidData>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box_entity_counts() {
        let mut model = BRepModel::new();
        let _r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        assert_eq!(model.vertices.len(), 8);
        assert_eq!(model.edges.len(), 12); // 12 unique edges (shared between faces)
        assert_eq!(model.faces.len(), 6);
        assert_eq!(model.shells.len(), 1);
        assert_eq!(model.solids.len(), 1);
    }

    #[test]
    fn test_box_faces_tagged() {
        let mut model = BRepModel::new();
        let _r = make_box(&mut model, Point3::ORIGIN, 2.0, 3.0, 4.0).unwrap();

        let op = model.history.records()[0].operation;
        for i in 0..6 {
            let tag = Tag::generated(EntityKind::Face, op, i);
            assert!(
                model.find_face_by_tag(&tag).is_some(),
                "face tag {i} not found"
            );
        }
    }

    #[test]
    fn test_box_vertex_positions() {
        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::new(1.0, 2.0, 3.0), 4.0, 5.0, 6.0).unwrap();
        let first = model.vertices.get(r.vertices[0]).unwrap();
        assert!(first.point.approx_eq(Point3::new(1.0, 2.0, 3.0)));
        let last = model.vertices.get(r.vertices[7]).unwrap();
        assert!(last.point.approx_eq(Point3::new(1.0, 7.0, 9.0)));
    }

    #[test]
    fn test_box_geometry_binding() {
        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 2.0, 3.0, 4.0).unwrap();

        // All 6 faces should have bound surfaces
        for &face_h in &r.faces {
            assert!(model.face_has_surface(face_h), "face should have surface");
        }
        // All 12 edges should have bound curves
        for (edge_h, _) in model.edges.iter() {
            assert!(model.edge_has_curve(edge_h), "edge should have curve");
        }
    }
}
