use std::f64::consts::TAU;
use std::sync::Arc;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_geometry::{Cone as ConeSurface, Plane};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, EntityKind, Handle, Orientation, Tag};

use super::{EdgeCache, bind_edge_line_segments, next_edge_tag};

/// Creates a cone (or frustum) as a B-Rep solid.
///
/// The cone is aligned along the Z-axis with its base at `base_center`.
/// If `top_radius == 0.0`, produces a pointed cone (apex vertex).
/// If `top_radius > 0.0`, produces a truncated cone (frustum) with a top face.
///
/// Produces: bottom vertices + top vertex/vertices, lateral quad/tri faces,
/// bottom face, optional top face, 1 shell, 1 solid.
///
/// All faces are bound to their ideal surfaces (`Plane` for caps, `Cone` for
/// lateral faces); all edges to `LineSegment` curves.
pub fn make_cone(
    model: &mut BRepModel,
    base_center: Point3,
    base_radius: f64,
    top_radius: f64,
    height: f64,
    segments: usize,
) -> KernelResult<ConeResult> {
    if segments < 3 {
        return Err(KernelError::InvalidArgument(
            "cone needs at least 3 segments".into(),
        ));
    }
    if base_radius <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "cone base radius must be positive".into(),
        ));
    }
    if top_radius < 0.0 {
        return Err(KernelError::InvalidArgument(
            "cone top radius must be non-negative".into(),
        ));
    }
    let op = model.history.next_operation("make_cone");
    let top_center = base_center + Vec3::Z * height;
    let is_pointed = top_radius.abs() < 1e-14;

    // Bottom ring vertices
    let mut bottom_verts = Vec::new();
    for i in 0..segments {
        let angle = TAU * i as f64 / segments as f64;
        let (sin, cos) = angle.sin_cos();
        let p = Point3::new(
            base_center.x + base_radius * cos,
            base_center.y + base_radius * sin,
            base_center.z,
        );
        let tag = Tag::generated(EntityKind::Vertex, op, i as u32);
        bottom_verts.push(model.add_vertex_tagged(p, tag));
    }

    let mut edge_idx = 0u32;
    let mut face_idx = 0u32;
    let mut ec = EdgeCache::new();
    let mut all_faces = Vec::new();

    // Bottom face (CCW when viewed from -Z)
    let mut bottom_hes = Vec::new();
    for i in 0..segments {
        let next = (i + 1) % segments;
        let tag = next_edge_tag(op, &mut edge_idx);
        let he = ec.get_or_create(model, bottom_verts[next], bottom_verts[i], tag);
        bottom_hes.push(he);
    }
    let bottom_loop = model.make_loop(&bottom_hes)?;
    let bottom_face_tag = Tag::generated(EntityKind::Face, op, face_idx);
    let bottom_face = model.make_face_tagged(bottom_loop, bottom_face_tag);
    all_faces.push(bottom_face);
    face_idx += 1;

    let top_face;
    let mut lateral_faces = Vec::new();

    if is_pointed {
        // Pointed cone: single apex vertex + triangular lateral faces
        let apex_tag = Tag::generated(EntityKind::Vertex, op, segments as u32);
        let apex = model.add_vertex_tagged(top_center, apex_tag);

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
                apex,
                next_edge_tag(op, &mut edge_idx),
            );
            let he3 = ec.get_or_create(
                model,
                apex,
                bottom_verts[i],
                next_edge_tag(op, &mut edge_idx),
            );

            let lp = model.make_loop(&[he1, he2, he3])?;
            let ft = Tag::generated(EntityKind::Face, op, face_idx);
            lateral_faces.push(model.make_face_tagged(lp, ft));
            face_idx += 1;
        }
        top_face = None;
    } else {
        // Frustum: top ring vertices + top face + quad lateral faces
        let mut top_verts = Vec::new();
        for i in 0..segments {
            let angle = TAU * i as f64 / segments as f64;
            let (sin, cos) = angle.sin_cos();
            let p = Point3::new(
                top_center.x + top_radius * cos,
                top_center.y + top_radius * sin,
                top_center.z,
            );
            let tag = Tag::generated(EntityKind::Vertex, op, (segments + i) as u32);
            top_verts.push(model.add_vertex_tagged(p, tag));
        }

        // Top face (CCW when viewed from +Z)
        let mut top_hes = Vec::new();
        for i in 0..segments {
            let next = (i + 1) % segments;
            let tag = next_edge_tag(op, &mut edge_idx);
            let he = ec.get_or_create(model, top_verts[i], top_verts[next], tag);
            top_hes.push(he);
        }
        let top_loop = model.make_loop(&top_hes)?;
        let top_face_tag = Tag::generated(EntityKind::Face, op, face_idx);
        let tf = model.make_face_tagged(top_loop, top_face_tag);
        all_faces.push(tf);
        top_face = Some(tf);
        face_idx += 1;

        // Lateral quads
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
            let ft = Tag::generated(EntityKind::Face, op, face_idx);
            lateral_faces.push(model.make_face_tagged(lp, ft));
            face_idx += 1;
        }
    }

    all_faces.extend(&lateral_faces);

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&all_faces, shell_tag);
    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let solid = model.make_solid_tagged(&[shell], solid_tag);

    // --- Geometry binding ---
    // Bottom cap
    let bottom_plane = Plane::new(base_center, Vec3::X, -Vec3::Y)?;
    model.bind_face_surface(bottom_face, Arc::new(bottom_plane), Orientation::Forward);

    // Top cap (frustum only)
    if let Some(tf) = top_face {
        let top_plane = Plane::new(top_center, Vec3::X, Vec3::Y)?;
        model.bind_face_surface(tf, Arc::new(top_plane), Orientation::Forward);
    }

    // Cone surface for lateral faces
    if is_pointed {
        // Apex at top_center, axis toward base (-Z), half_angle = atan(R/h)
        let half_angle = (base_radius / height).atan();
        if let Ok(cone_surf) = ConeSurface::new(top_center, -Vec3::Z, half_angle) {
            let surf: Arc<dyn cadkernel_geometry::Surface + Send + Sync> = Arc::new(cone_surf);
            for &lat in &lateral_faces {
                model.bind_face_surface(lat, surf.clone(), Orientation::Forward);
            }
        }
    } else {
        let dr = base_radius - top_radius;
        if dr.abs() > 1e-10 {
            // Frustum: compute true apex above the top
            let d_apex = base_radius * height / dr;
            let apex_pt = base_center + Vec3::Z * d_apex;
            let half_angle = (dr / height).atan();
            if let Ok(cone_surf) = ConeSurface::new(apex_pt, -Vec3::Z, half_angle) {
                let surf: Arc<dyn cadkernel_geometry::Surface + Send + Sync> = Arc::new(cone_surf);
                for &lat in &lateral_faces {
                    model.bind_face_surface(lat, surf.clone(), Orientation::Forward);
                }
            }
        }
    }

    bind_edge_line_segments(model, &ec);

    Ok(ConeResult {
        bottom_face,
        top_face,
        lateral_faces,
        shell,
        solid,
    })
}

/// Handles returned from [`make_cone`].
pub struct ConeResult {
    pub bottom_face: Handle<cadkernel_topology::FaceData>,
    /// `None` for a pointed cone, `Some` for a frustum.
    pub top_face: Option<Handle<cadkernel_topology::FaceData>>,
    pub lateral_faces: Vec<Handle<cadkernel_topology::FaceData>>,
    pub shell: Handle<cadkernel_topology::ShellData>,
    pub solid: Handle<cadkernel_topology::SolidData>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pointed_cone_entity_counts() {
        let mut model = BRepModel::new();
        let r = make_cone(&mut model, Point3::ORIGIN, 5.0, 0.0, 10.0, 8).unwrap();
        // 8 bottom ring + 1 apex = 9 vertices
        assert_eq!(model.vertices.len(), 9);
        // 1 bottom + 8 lateral = 9 faces
        assert_eq!(r.lateral_faces.len(), 8);
        assert!(r.top_face.is_none());
        assert_eq!(model.faces.len(), 9);
        assert_eq!(model.shells.len(), 1);
        assert_eq!(model.solids.len(), 1);
    }

    #[test]
    fn test_frustum_entity_counts() {
        let mut model = BRepModel::new();
        let r = make_cone(&mut model, Point3::ORIGIN, 5.0, 2.5, 10.0, 16).unwrap();
        // 16 bottom + 16 top = 32 vertices
        assert_eq!(model.vertices.len(), 32);
        // 1 bottom + 1 top + 16 lateral = 18 faces
        assert!(r.top_face.is_some());
        assert_eq!(r.lateral_faces.len(), 16);
        assert_eq!(model.faces.len(), 18);
        // 16 bottom ring + 16 top ring + 16 vertical = 48 edges
        assert_eq!(model.edges.len(), 48);
    }

    #[test]
    fn test_cone_apex_position() {
        let mut model = BRepModel::new();
        let _r = make_cone(&mut model, Point3::new(1.0, 2.0, 3.0), 5.0, 0.0, 7.0, 6).unwrap();
        // Apex should be at base_center + height along Z
        let expected = Point3::new(1.0, 2.0, 10.0);
        let found = model
            .vertices
            .iter()
            .any(|(_, v)| v.point.approx_eq(expected));
        assert!(found, "expected apex vertex at {expected:?}");
    }

    #[test]
    fn test_cone_validation() {
        let mut model = BRepModel::new();
        assert!(make_cone(&mut model, Point3::ORIGIN, 5.0, 0.0, 10.0, 2).is_err());
        assert!(make_cone(&mut model, Point3::ORIGIN, 0.0, 0.0, 10.0, 8).is_err());
        assert!(make_cone(&mut model, Point3::ORIGIN, 5.0, -1.0, 10.0, 8).is_err());
    }

    #[test]
    fn test_cone_geometry_binding() {
        let mut model = BRepModel::new();
        let r = make_cone(&mut model, Point3::ORIGIN, 5.0, 0.0, 10.0, 8).unwrap();

        assert!(model.face_has_surface(r.bottom_face));
        for &lat in &r.lateral_faces {
            assert!(model.face_has_surface(lat), "lateral face should have surface");
        }
        for (edge_h, _) in model.edges.iter() {
            assert!(model.edge_has_curve(edge_h), "edge should have curve");
        }
    }

    #[test]
    fn test_frustum_geometry_binding() {
        let mut model = BRepModel::new();
        let r = make_cone(&mut model, Point3::ORIGIN, 5.0, 2.5, 10.0, 8).unwrap();

        assert!(model.face_has_surface(r.bottom_face));
        assert!(model.face_has_surface(r.top_face.unwrap()));
        for &lat in &r.lateral_faces {
            assert!(model.face_has_surface(lat), "lateral face should have surface");
        }
    }
}
