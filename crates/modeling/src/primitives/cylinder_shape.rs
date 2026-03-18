use std::f64::consts::TAU;
use std::sync::Arc;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_geometry::{Cylinder as CylSurface, Plane};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, EntityKind, Handle, Orientation, Tag};

use super::{EdgeCache, bind_edge_line_segments, next_edge_tag};

/// Creates a cylinder as a B-Rep solid.
///
/// The cylinder is aligned along the Z-axis with its base at `base_center`.
/// Circular edges are approximated as a polygon with `segments` sides.
///
/// Produces: `2*segments` vertices, `3*segments` edges, N+2 faces
/// (top, bottom, N lateral), 1 shell, 1 solid.
///
/// All faces are bound to their ideal surfaces (`Plane` for caps, `Cylinder` for
/// lateral faces); all edges to `LineSegment` curves.
pub fn make_cylinder(
    model: &mut BRepModel,
    base_center: Point3,
    radius: f64,
    height: f64,
    segments: usize,
) -> KernelResult<CylinderResult> {
    if segments < 3 {
        return Err(KernelError::InvalidArgument(
            "cylinder needs at least 3 segments".into(),
        ));
    }
    let op = model.history.next_operation("make_cylinder");
    let top_center = base_center + Vec3::Z * height;

    // Create vertices around the bottom and top circles
    let mut bottom_verts = Vec::new();
    let mut top_verts = Vec::new();
    for i in 0..segments {
        let angle = TAU * i as f64 / segments as f64;
        let (sin, cos) = angle.sin_cos();
        let dx = radius * cos;
        let dy = radius * sin;

        let bp = Point3::new(base_center.x + dx, base_center.y + dy, base_center.z);
        let tp = Point3::new(top_center.x + dx, top_center.y + dy, top_center.z);

        let bt = Tag::generated(EntityKind::Vertex, op, i as u32);
        let tt = Tag::generated(EntityKind::Vertex, op, (segments + i) as u32);
        bottom_verts.push(model.add_vertex_tagged(bp, bt));
        top_verts.push(model.add_vertex_tagged(tp, tt));
    }

    let mut edge_idx = 0u32;
    let mut ec = EdgeCache::new();

    // Bottom face (CCW when viewed from -Z)
    let mut bottom_hes = Vec::new();
    for i in 0..segments {
        let next = (i + 1) % segments;
        let tag = next_edge_tag(op, &mut edge_idx);
        let he = ec.get_or_create(model, bottom_verts[next], bottom_verts[i], tag);
        bottom_hes.push(he);
    }
    let bottom_loop = model.make_loop(&bottom_hes)?;
    let bottom_face_tag = Tag::generated(EntityKind::Face, op, 0);
    let bottom_face = model.make_face_tagged(bottom_loop, bottom_face_tag);

    // Top face (CCW when viewed from +Z)
    let mut top_hes = Vec::new();
    for i in 0..segments {
        let next = (i + 1) % segments;
        let tag = next_edge_tag(op, &mut edge_idx);
        let he = ec.get_or_create(model, top_verts[i], top_verts[next], tag);
        top_hes.push(he);
    }
    let top_loop = model.make_loop(&top_hes)?;
    let top_face_tag = Tag::generated(EntityKind::Face, op, 1);
    let top_face = model.make_face_tagged(top_loop, top_face_tag);

    // Lateral faces (one quad per segment)
    let mut lateral_faces = Vec::new();
    for i in 0..segments {
        let next = (i + 1) % segments;

        let he1 = ec.get_or_create(
            model,
            bottom_verts[i],
            bottom_verts[next],
            next_edge_tag(op, &mut edge_idx),
        );
        let he2 = ec.get_or_create(
            model,
            bottom_verts[next],
            top_verts[next],
            next_edge_tag(op, &mut edge_idx),
        );
        let he3 = ec.get_or_create(
            model,
            top_verts[next],
            top_verts[i],
            next_edge_tag(op, &mut edge_idx),
        );
        let he4 = ec.get_or_create(
            model,
            top_verts[i],
            bottom_verts[i],
            next_edge_tag(op, &mut edge_idx),
        );

        let lp = model.make_loop(&[he1, he2, he3, he4])?;
        let face_tag = Tag::generated(EntityKind::Face, op, (2 + i) as u32);
        lateral_faces.push(model.make_face_tagged(lp, face_tag));
    }

    let mut all_faces = vec![bottom_face, top_face];
    all_faces.extend(&lateral_faces);

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&all_faces, shell_tag);

    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let solid = model.make_solid_tagged(&[shell], solid_tag);

    // --- Geometry binding ---
    let bottom_plane = Plane::new(base_center, Vec3::X, -Vec3::Y)?;
    let top_plane = Plane::new(top_center, Vec3::X, Vec3::Y)?;
    model.bind_face_surface(bottom_face, Arc::new(bottom_plane), Orientation::Forward);
    model.bind_face_surface(top_face, Arc::new(top_plane), Orientation::Forward);

    let cyl_surf: Arc<dyn cadkernel_geometry::Surface + Send + Sync> =
        Arc::new(CylSurface::new(base_center, Vec3::Z, radius, height)?);
    for &lat_face in &lateral_faces {
        model.bind_face_surface(lat_face, cyl_surf.clone(), Orientation::Forward);
    }
    bind_edge_line_segments(model, &ec);

    Ok(CylinderResult {
        bottom_face,
        top_face,
        lateral_faces,
        shell,
        solid,
    })
}

/// Handles returned from [`make_cylinder`].
pub struct CylinderResult {
    pub bottom_face: Handle<cadkernel_topology::FaceData>,
    pub top_face: Handle<cadkernel_topology::FaceData>,
    pub lateral_faces: Vec<Handle<cadkernel_topology::FaceData>>,
    pub shell: Handle<cadkernel_topology::ShellData>,
    pub solid: Handle<cadkernel_topology::SolidData>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cylinder_entity_counts() {
        let mut model = BRepModel::new();
        let _r = make_cylinder(&mut model, Point3::ORIGIN, 1.0, 2.0, 16).unwrap();
        assert_eq!(model.vertices.len(), 32); // 16 bottom + 16 top
        assert_eq!(model.edges.len(), 48); // 16 bottom ring + 16 top ring + 16 vertical
        assert_eq!(model.faces.len(), 18); // 1 bottom + 1 top + 16 lateral
        assert_eq!(model.shells.len(), 1);
        assert_eq!(model.solids.len(), 1);
    }

    #[test]
    fn test_cylinder_tagged_faces() {
        let mut model = BRepModel::new();
        let _r = make_cylinder(&mut model, Point3::ORIGIN, 1.0, 2.0, 8).unwrap();
        let op = model.history.records()[0].operation;
        // bottom=0, top=1, lateral=2..9
        for i in 0..10 {
            let tag = Tag::generated(EntityKind::Face, op, i);
            assert!(model.find_face_by_tag(&tag).is_some());
        }
    }

    #[test]
    fn test_cylinder_geometry_binding() {
        let mut model = BRepModel::new();
        let r = make_cylinder(&mut model, Point3::ORIGIN, 1.0, 2.0, 8).unwrap();

        assert!(model.face_has_surface(r.bottom_face));
        assert!(model.face_has_surface(r.top_face));
        for &lat in &r.lateral_faces {
            assert!(model.face_has_surface(lat), "lateral face should have surface");
        }
        for (edge_h, _) in model.edges.iter() {
            assert!(model.edge_has_curve(edge_h), "edge should have curve");
        }
    }
}
