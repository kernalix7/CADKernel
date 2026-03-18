use std::f64::consts::TAU;
use std::sync::Arc;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_geometry::Torus as TorSurface;
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, EntityKind, Handle, Orientation, Tag};

use super::{EdgeCache, bind_edge_line_segments, next_edge_tag};

/// Creates a torus as a B-Rep solid.
///
/// The torus is centered at `center` and lies in the XY plane.
/// `major_radius` is the distance from the center to the tube center.
/// `minor_radius` is the radius of the tube cross-section.
///
/// `major_segments` controls the number of divisions around the main ring (min 3).
/// `minor_segments` controls the number of divisions around the tube (min 3).
///
/// Produces: `major_segments * minor_segments` vertices,
/// `major_segments * minor_segments` quad faces, 1 shell, 1 solid.
///
/// All faces are bound to a shared `Torus` surface; all edges to `LineSegment`
/// curves.
pub fn make_torus(
    model: &mut BRepModel,
    center: Point3,
    major_radius: f64,
    minor_radius: f64,
    major_segments: usize,
    minor_segments: usize,
) -> KernelResult<TorusResult> {
    if major_segments < 3 {
        return Err(KernelError::InvalidArgument(
            "torus needs at least 3 major segments".into(),
        ));
    }
    if minor_segments < 3 {
        return Err(KernelError::InvalidArgument(
            "torus needs at least 3 minor segments".into(),
        ));
    }
    if major_radius <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "torus major radius must be positive".into(),
        ));
    }
    if minor_radius <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "torus minor radius must be positive".into(),
        ));
    }
    let op = model.history.next_operation("make_torus");

    // Create vertices: ring[major_i][minor_j]
    let mut ring_verts: Vec<Vec<Handle<cadkernel_topology::VertexData>>> = Vec::new();
    let mut vert_idx = 0u32;

    for i in 0..major_segments {
        let theta = TAU * i as f64 / major_segments as f64;
        let (sin_t, cos_t) = theta.sin_cos();
        let mut ring = Vec::new();

        for j in 0..minor_segments {
            let phi = TAU * j as f64 / minor_segments as f64;
            let (sin_p, cos_p) = phi.sin_cos();

            // Point on torus surface
            let r = major_radius + minor_radius * cos_p;
            let p = Point3::new(
                center.x + r * cos_t,
                center.y + r * sin_t,
                center.z + minor_radius * sin_p,
            );

            let tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
            ring.push(model.add_vertex_tagged(p, tag));
            vert_idx += 1;
        }
        ring_verts.push(ring);
    }

    let mut edge_idx = 0u32;
    let mut face_idx = 0u32;
    let mut all_faces = Vec::new();
    let mut ec = EdgeCache::new();

    // Create quad faces connecting adjacent rings
    for i in 0..major_segments {
        let next_i = (i + 1) % major_segments;
        for j in 0..minor_segments {
            let next_j = (j + 1) % minor_segments;

            let he1 = ec.get_or_create(
                model,
                ring_verts[i][j],
                ring_verts[i][next_j],
                next_edge_tag(op, &mut edge_idx),
            );
            let he2 = ec.get_or_create(
                model,
                ring_verts[i][next_j],
                ring_verts[next_i][next_j],
                next_edge_tag(op, &mut edge_idx),
            );
            let he3 = ec.get_or_create(
                model,
                ring_verts[next_i][next_j],
                ring_verts[next_i][j],
                next_edge_tag(op, &mut edge_idx),
            );
            let he4 = ec.get_or_create(
                model,
                ring_verts[next_i][j],
                ring_verts[i][j],
                next_edge_tag(op, &mut edge_idx),
            );

            let lp = model.make_loop(&[he1, he2, he3, he4])?;
            let ft = Tag::generated(EntityKind::Face, op, face_idx);
            all_faces.push(model.make_face_tagged(lp, ft));
            face_idx += 1;
        }
    }

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&all_faces, shell_tag);
    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let solid = model.make_solid_tagged(&[shell], solid_tag);

    // --- Geometry binding ---
    let tor_surf: Arc<dyn cadkernel_geometry::Surface + Send + Sync> =
        Arc::new(TorSurface::new(center, Vec3::Z, major_radius, minor_radius)?);
    for &face_h in &all_faces {
        model.bind_face_surface(face_h, tor_surf.clone(), Orientation::Forward);
    }
    bind_edge_line_segments(model, &ec);

    Ok(TorusResult {
        faces: all_faces,
        shell,
        solid,
    })
}

/// Handles returned from [`make_torus`].
pub struct TorusResult {
    pub faces: Vec<Handle<cadkernel_topology::FaceData>>,
    pub shell: Handle<cadkernel_topology::ShellData>,
    pub solid: Handle<cadkernel_topology::SolidData>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_torus_entity_counts() {
        let mut model = BRepModel::new();
        let r = make_torus(&mut model, Point3::ORIGIN, 5.0, 1.0, 8, 6).unwrap();
        // 8 * 6 = 48 vertices
        assert_eq!(model.vertices.len(), 48);
        // 8 * 6 = 48 quad faces
        assert_eq!(r.faces.len(), 48);
        // Each quad has 4 edges, shared: 2 * major * minor unique edges
        // major_segments * minor_segments (minor rings) + major_segments * minor_segments (major rings)
        assert_eq!(model.edges.len(), 2 * 8 * 6);
        assert_eq!(model.shells.len(), 1);
        assert_eq!(model.solids.len(), 1);
    }

    #[test]
    fn test_torus_vertex_positions() {
        let mut model = BRepModel::new();
        let _r = make_torus(&mut model, Point3::new(1.0, 2.0, 3.0), 5.0, 1.0, 4, 4).unwrap();
        // First vertex: theta=0, phi=0 → (major+minor, 0, 0) + center
        // Check that at least one vertex is at (center.x + major + minor, center.y, center.z)
        let expected = Point3::new(7.0, 2.0, 3.0);
        let found = model
            .vertices
            .iter()
            .any(|(_, v)| v.point.approx_eq(expected));
        assert!(found, "expected vertex at {expected:?}");
    }

    #[test]
    fn test_torus_validation() {
        let mut model = BRepModel::new();
        assert!(make_torus(&mut model, Point3::ORIGIN, 5.0, 1.0, 2, 4).is_err());
        assert!(make_torus(&mut model, Point3::ORIGIN, 5.0, 1.0, 4, 2).is_err());
        assert!(make_torus(&mut model, Point3::ORIGIN, 0.0, 1.0, 4, 4).is_err());
        assert!(make_torus(&mut model, Point3::ORIGIN, 5.0, 0.0, 4, 4).is_err());
    }

    #[test]
    fn test_torus_geometry_binding() {
        let mut model = BRepModel::new();
        let r = make_torus(&mut model, Point3::ORIGIN, 5.0, 1.0, 8, 6).unwrap();

        for &face_h in &r.faces {
            assert!(model.face_has_surface(face_h), "face should have surface");
        }
        for (edge_h, _) in model.edges.iter() {
            assert!(model.edge_has_curve(edge_h), "edge should have curve");
        }
    }
}
