//! Surface workbench operations: ruled surface, surface from curves,
//! extend surface, and pipe surface.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_geometry::{Curve, NurbsCurve, NurbsSurface};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{
    BRepModel, EntityKind, FaceData, Handle, SolidData, Tag, VertexData,
};

/// Result of a ruled surface operation.
#[derive(Debug)]
pub struct RuledSurfaceResult {
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Result of a surface-from-curves operation.
#[derive(Debug)]
pub struct SurfaceFromCurvesResult {
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Result of an extend-surface operation.
#[derive(Debug)]
pub struct ExtendResult {
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Result of a pipe surface operation.
#[derive(Debug)]
pub struct PipeSurfaceResult {
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Creates a ruled (linear interpolation) surface between two NURBS curves.
///
/// Samples both curves at `segments_u + 1` points and linearly interpolates
/// between corresponding points with `segments_v + 1` rows.
pub fn ruled_surface(
    model: &mut BRepModel,
    curve1: &NurbsCurve,
    curve2: &NurbsCurve,
    segments_u: usize,
    segments_v: usize,
) -> KernelResult<RuledSurfaceResult> {
    if segments_u < 1 {
        return Err(KernelError::InvalidArgument(
            "ruled_surface requires segments_u >= 1".into(),
        ));
    }
    if segments_v < 1 {
        return Err(KernelError::InvalidArgument(
            "ruled_surface requires segments_v >= 1".into(),
        ));
    }

    let op = model.history.next_operation("ruled_surface");

    let (t0_1, t1_1) = curve1.domain();
    let (t0_2, t1_2) = curve2.domain();

    let nu = segments_u + 1;
    let nv = segments_v + 1;

    // Build vertex grid: rows along v (0=curve1, 1=curve2), columns along u.
    let mut verts: Vec<Vec<Handle<VertexData>>> = Vec::with_capacity(nv);
    let mut vert_idx = 0u32;

    for vi in 0..nv {
        let v_frac = vi as f64 / segments_v as f64;
        let mut row = Vec::with_capacity(nu);
        for ui in 0..nu {
            let u_frac = ui as f64 / segments_u as f64;
            let p1 = curve1.point_at(t0_1 + u_frac * (t1_1 - t0_1));
            let p2 = curve2.point_at(t0_2 + u_frac * (t1_2 - t0_2));
            let pt = Point3::new(
                p1.x * (1.0 - v_frac) + p2.x * v_frac,
                p1.y * (1.0 - v_frac) + p2.y * v_frac,
                p1.z * (1.0 - v_frac) + p2.z * v_frac,
            );
            let tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
            row.push(model.add_vertex_tagged(pt, tag));
            vert_idx += 1;
        }
        verts.push(row);
    }

    let mut all_faces = Vec::new();
    let mut edge_idx = 0u32;
    let mut face_idx = 0u32;

    for vi in 0..segments_v {
        for ui in 0..segments_u {
            let quad = [
                verts[vi][ui],
                verts[vi][ui + 1],
                verts[vi + 1][ui + 1],
                verts[vi + 1][ui],
            ];
            let f = make_quad_face(model, &quad, op, &mut edge_idx, face_idx)?;
            all_faces.push(f);
            face_idx += 1;
        }
    }

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&all_faces, shell_tag);
    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let solid = model.make_solid_tagged(&[shell], solid_tag);

    Ok(RuledSurfaceResult {
        solid,
        faces: all_faces,
    })
}

/// Creates a surface from a network of profile curves (Gordon-like construction).
///
/// Tessellates each profile curve into `segments + 1` points and connects
/// consecutive profiles with quad faces.
pub fn surface_from_curves(
    model: &mut BRepModel,
    profiles: &[&NurbsCurve],
    segments: usize,
) -> KernelResult<SurfaceFromCurvesResult> {
    if profiles.len() < 2 {
        return Err(KernelError::InvalidArgument(
            "surface_from_curves requires at least 2 profiles".into(),
        ));
    }
    if segments < 1 {
        return Err(KernelError::InvalidArgument(
            "surface_from_curves requires segments >= 1".into(),
        ));
    }

    let op = model.history.next_operation("surface_from_curves");
    let n_pts = segments + 1;

    // Build vertex grid: one row per profile, n_pts columns.
    let mut verts: Vec<Vec<Handle<VertexData>>> = Vec::with_capacity(profiles.len());
    let mut vert_idx = 0u32;

    for profile in profiles {
        let (t0, t1) = profile.domain();
        let mut row = Vec::with_capacity(n_pts);
        for ui in 0..n_pts {
            let u_frac = ui as f64 / segments as f64;
            let pt = profile.point_at(t0 + u_frac * (t1 - t0));
            let tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
            row.push(model.add_vertex_tagged(pt, tag));
            vert_idx += 1;
        }
        verts.push(row);
    }

    let mut all_faces = Vec::new();
    let mut edge_idx = 0u32;
    let mut face_idx = 0u32;

    for pi in 0..profiles.len() - 1 {
        for ui in 0..segments {
            let quad = [
                verts[pi][ui],
                verts[pi][ui + 1],
                verts[pi + 1][ui + 1],
                verts[pi + 1][ui],
            ];
            let f = make_quad_face(model, &quad, op, &mut edge_idx, face_idx)?;
            all_faces.push(f);
            face_idx += 1;
        }
    }

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&all_faces, shell_tag);
    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let solid = model.make_solid_tagged(&[shell], solid_tag);

    Ok(SurfaceFromCurvesResult {
        solid,
        faces: all_faces,
    })
}

/// Extends a solid by offsetting all vertices along their averaged face normals.
///
/// Uses the same vertex-normal offset technique as `offset_solid`.
pub fn extend_surface(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    distance: f64,
) -> KernelResult<ExtendResult> {
    if distance.abs() < 1e-14 {
        return Err(KernelError::InvalidArgument(
            "extend_surface distance must be non-zero".into(),
        ));
    }

    let solid_data = model
        .solids
        .get(solid)
        .ok_or_else(|| KernelError::InvalidArgument("solid not found".into()))?;
    let shell_handles: Vec<_> = solid_data.shells.clone();

    // Collect all face handles from the solid.
    let mut face_handles = Vec::new();
    for &sh in &shell_handles {
        if let Some(shell_data) = model.shells.get(sh) {
            face_handles.extend_from_slice(&shell_data.faces);
        }
    }

    // Compute per-vertex averaged face normal.
    let mut vertex_normals: std::collections::HashMap<u32, Vec3> =
        std::collections::HashMap::new();

    for &face_h in &face_handles {
        let verts = model.vertices_of_face(face_h)?;
        if verts.len() < 3 {
            continue;
        }
        let positions: Vec<Point3> = verts
            .iter()
            .map(|&vh| {
                model
                    .vertices
                    .get(vh)
                    .map(|v| v.point)
                    .unwrap_or(Point3::ORIGIN)
            })
            .collect();

        let v01 = positions[1] - positions[0];
        let v02 = positions[2] - positions[0];
        let face_normal = v01.cross(v02);

        for &vh in &verts {
            vertex_normals
                .entry(vh.index())
                .and_modify(|n| *n += face_normal)
                .or_insert(face_normal);
        }
    }

    // Normalize.
    for n in vertex_normals.values_mut() {
        if let Some(nn) = n.normalized() {
            *n = nn;
        }
    }

    // Offset each vertex.
    let mut moved = std::collections::HashSet::new();
    for &face_h in &face_handles {
        let verts = model.vertices_of_face(face_h)?;
        for &vh in &verts {
            if moved.contains(&vh.index()) {
                continue;
            }
            moved.insert(vh.index());
            if let Some(&normal) = vertex_normals.get(&vh.index()) {
                if let Some(vd) = model.vertices.get_mut(vh) {
                    vd.point = Point3::new(
                        vd.point.x + normal.x * distance,
                        vd.point.y + normal.y * distance,
                        vd.point.z + normal.z * distance,
                    );
                }
            }
        }
    }

    Ok(ExtendResult {
        solid,
        faces: face_handles,
    })
}

/// Creates a pipe (tubular surface) along a path with a given radius.
///
/// At each path point a local frame (tangent, normal, binormal) is computed.
/// Circle vertices are placed around each frame, and consecutive circles are
/// connected with quad faces. Both ends are capped.
pub fn pipe_surface(
    model: &mut BRepModel,
    path_points: &[Point3],
    radius: f64,
    segments: usize,
) -> KernelResult<PipeSurfaceResult> {
    if path_points.len() < 2 {
        return Err(KernelError::InvalidArgument(
            "pipe_surface path needs at least 2 points".into(),
        ));
    }
    if radius <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "pipe_surface radius must be positive".into(),
        ));
    }
    if segments < 3 {
        return Err(KernelError::InvalidArgument(
            "pipe_surface requires at least 3 segments".into(),
        ));
    }

    let op = model.history.next_operation("pipe_surface");
    let ns = path_points.len();
    let mut rings: Vec<Vec<Handle<VertexData>>> = Vec::with_capacity(ns);
    let mut vert_idx = 0u32;

    for si in 0..ns {
        let tangent = if si == 0 {
            (path_points[1] - path_points[0])
                .normalized()
                .unwrap_or(Vec3::Z)
        } else if si == ns - 1 {
            (path_points[ns - 1] - path_points[ns - 2])
                .normalized()
                .unwrap_or(Vec3::Z)
        } else {
            let t1 = (path_points[si + 1] - path_points[si])
                .normalized()
                .unwrap_or(Vec3::Z);
            let t0 = (path_points[si] - path_points[si - 1])
                .normalized()
                .unwrap_or(Vec3::Z);
            (t1 + t0).normalized().unwrap_or(Vec3::Z)
        };

        let up_hint = if tangent.cross(Vec3::Z).length() > 1e-6 {
            Vec3::Z
        } else {
            Vec3::X
        };
        let binormal = tangent.cross(up_hint).normalized().unwrap_or(Vec3::X);
        let normal = binormal.cross(tangent).normalized().unwrap_or(Vec3::Y);

        let center = path_points[si];
        let mut ring = Vec::with_capacity(segments);
        for ci in 0..segments {
            let angle = 2.0 * std::f64::consts::PI * ci as f64 / segments as f64;
            let (sin_a, cos_a) = angle.sin_cos();
            let pt = Point3::new(
                center.x + radius * (cos_a * binormal.x + sin_a * normal.x),
                center.y + radius * (cos_a * binormal.y + sin_a * normal.y),
                center.z + radius * (cos_a * binormal.z + sin_a * normal.z),
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

    // Bottom cap (reversed winding).
    let bottom = make_ring_face(model, &rings[0], op, &mut edge_idx, face_idx, true)?;
    all_faces.push(bottom);
    face_idx += 1;

    // Top cap.
    let top = make_ring_face(model, &rings[ns - 1], op, &mut edge_idx, face_idx, false)?;
    all_faces.push(top);
    face_idx += 1;

    // Side quads between consecutive rings.
    for si in 0..ns - 1 {
        for ci in 0..segments {
            let next_ci = (ci + 1) % segments;
            let quad = [
                rings[si][ci],
                rings[si][next_ci],
                rings[si + 1][next_ci],
                rings[si + 1][ci],
            ];
            let f = make_quad_face(model, &quad, op, &mut edge_idx, face_idx)?;
            all_faces.push(f);
            face_idx += 1;
        }
    }

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&all_faces, shell_tag);
    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let solid = model.make_solid_tagged(&[shell], solid_tag);

    Ok(PipeSurfaceResult {
        solid,
        faces: all_faces,
    })
}

// ---- helpers ----------------------------------------------------------------

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

/// Result of a Coons patch operation.
#[derive(Debug)]
pub struct CoonsPatchResult {
    pub surface: NurbsSurface,
}

/// Creates a Coons bilinear blending patch from four boundary curves.
///
/// The four curves form a closed boundary: `u0` (bottom), `u1` (top),
/// `v0` (left), `v1` (right). The resulting surface is a bilinear
/// interpolation (Coons patch) represented as a NurbsSurface.
///
/// Each boundary curve is sampled at `n+1` points (where `n` is a
/// resolution parameter derived from the curve degrees). The Coons
/// formula is: `S(u,v) = L_u + L_v - B` where `L_u` interpolates the
/// u-direction curves, `L_v` interpolates the v-direction curves, and
/// `B` is the bilinear correction from the four corner points.
pub fn coons_patch(
    curve_u0: &dyn Curve,
    curve_u1: &dyn Curve,
    curve_v0: &dyn Curve,
    curve_v1: &dyn Curve,
) -> KernelResult<CoonsPatchResult> {
    let n = 16_usize;
    let count_u = n + 1;
    let count_v = n + 1;

    let (u0_a, u0_b) = curve_u0.domain();
    let (u1_a, u1_b) = curve_u1.domain();
    let (v0_a, v0_b) = curve_v0.domain();
    let (v1_a, v1_b) = curve_v1.domain();

    // Corner points: P(0,0), P(1,0), P(0,1), P(1,1)
    let p00 = curve_u0.point_at(u0_a);
    let p10 = curve_u0.point_at(u0_b);
    let p01 = curve_u1.point_at(u1_a);
    let p11 = curve_u1.point_at(u1_b);

    let total = count_u * count_v;
    let mut control_points = Vec::with_capacity(total);

    for vi in 0..count_v {
        let v = vi as f64 / n as f64;
        for ui in 0..count_u {
            let u = ui as f64 / n as f64;

            // Ruled surface in u-direction
            let cu0 = curve_u0.point_at(u0_a + u * (u0_b - u0_a));
            let cu1 = curve_u1.point_at(u1_a + u * (u1_b - u1_a));
            let lu = Point3::new(
                (1.0 - v) * cu0.x + v * cu1.x,
                (1.0 - v) * cu0.y + v * cu1.y,
                (1.0 - v) * cu0.z + v * cu1.z,
            );

            // Ruled surface in v-direction
            let cv0 = curve_v0.point_at(v0_a + v * (v0_b - v0_a));
            let cv1 = curve_v1.point_at(v1_a + v * (v1_b - v1_a));
            let lv = Point3::new(
                (1.0 - u) * cv0.x + u * cv1.x,
                (1.0 - u) * cv0.y + u * cv1.y,
                (1.0 - u) * cv0.z + u * cv1.z,
            );

            // Bilinear correction
            let b = Point3::new(
                (1.0 - u) * (1.0 - v) * p00.x
                    + u * (1.0 - v) * p10.x
                    + (1.0 - u) * v * p01.x
                    + u * v * p11.x,
                (1.0 - u) * (1.0 - v) * p00.y
                    + u * (1.0 - v) * p10.y
                    + (1.0 - u) * v * p01.y
                    + u * v * p11.y,
                (1.0 - u) * (1.0 - v) * p00.z
                    + u * (1.0 - v) * p10.z
                    + (1.0 - u) * v * p01.z
                    + u * v * p11.z,
            );

            // Coons: S = Lu + Lv - B
            control_points.push(Point3::new(
                lu.x + lv.x - b.x,
                lu.y + lv.y - b.y,
                lu.z + lv.z - b.z,
            ));
        }
    }

    let weights = vec![1.0; total];

    // Uniform clamped knot vector of degree 1 for the interpolation grid
    let degree = 1;
    let make_knots = |count: usize| -> Vec<f64> {
        let mut knots = vec![0.0; degree + 1];
        for i in 1..count - degree {
            knots.push(i as f64 / (count - degree) as f64);
        }
        knots.extend(std::iter::repeat_n(1.0, degree + 1));
        knots
    };

    let knots_u = make_knots(count_u);
    let knots_v = make_knots(count_v);

    let surface = NurbsSurface::new(
        degree,
        degree,
        count_u,
        count_v,
        control_points,
        weights,
        knots_u,
        knots_v,
    )?;

    Ok(CoonsPatchResult { surface })
}

/// Result of a surface filling operation.
#[derive(Debug)]
pub struct SurfaceFillingResult {
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Result of a surface sections (skinning) operation.
#[derive(Debug)]
pub struct SurfaceSectionsResult {
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Result of a curve-on-mesh projection.
#[derive(Debug)]
pub struct CurveOnMeshResult {
    pub points: Vec<Point3>,
}

/// Fills an N-sided boundary patch by triangulating a centroid fan.
///
/// Given a closed boundary ring of 3D points, creates a surface patch
/// by connecting all boundary points to a centroid point via triangular fans.
pub fn filling(
    model: &mut BRepModel,
    boundary: &[Point3],
    subdivisions: usize,
) -> KernelResult<SurfaceFillingResult> {
    if boundary.len() < 3 {
        return Err(KernelError::InvalidArgument(
            "filling needs at least 3 boundary points".into(),
        ));
    }
    let n = boundary.len();
    let op = model.history.next_operation("filling");

    // Compute centroid
    let mut cx = 0.0;
    let mut cy = 0.0;
    let mut cz = 0.0;
    for p in boundary {
        cx += p.x;
        cy += p.y;
        cz += p.z;
    }
    let nf = n as f64;
    let centroid = Point3::new(cx / nf, cy / nf, cz / nf);

    // Subdivide: for each boundary edge, create subdivision points
    let subs = subdivisions.max(1);
    let mut all_points = Vec::new();
    let mut all_faces = Vec::new();
    let mut vert_idx = 0u32;
    let mut edge_idx = 0u32;
    let mut face_idx = 0u32;

    // Center vertex
    let center_tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
    let center_v = model.add_vertex_tagged(centroid, center_tag);
    vert_idx += 1;

    // Boundary vertices
    let mut boundary_verts = Vec::with_capacity(n);
    for &p in boundary {
        let tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
        boundary_verts.push(model.add_vertex_tagged(p, tag));
        vert_idx += 1;
    }

    if subs == 1 {
        // Simple fan triangulation
        for i in 0..n {
            let j = (i + 1) % n;
            let verts = [boundary_verts[i], boundary_verts[j], center_v];
            let mut hes = Vec::with_capacity(3);
            for k in 0..3 {
                let l = (k + 1) % 3;
                let etag = Tag::generated(EntityKind::Edge, op, edge_idx);
                let (_, he, _) = model.add_edge_tagged(verts[k], verts[l], etag);
                hes.push(he);
                edge_idx += 1;
            }
            let lp = model.make_loop(&hes)?;
            let ft = Tag::generated(EntityKind::Face, op, face_idx);
            all_faces.push(model.make_face_tagged(lp, ft));
            face_idx += 1;
        }
    } else {
        // Subdivided fan: create intermediate rings
        for i in 0..n {
            let j = (i + 1) % n;
            let p0 = boundary[i];
            let p1 = boundary[j];

            // Create sub-triangles from boundary edge to center
            let mut prev_a = boundary_verts[i];
            let mut prev_b = boundary_verts[j];

            for s in 1..=subs {
                let t = s as f64 / subs as f64;
                let a = Point3::new(
                    p0.x + t * (centroid.x - p0.x),
                    p0.y + t * (centroid.y - p0.y),
                    p0.z + t * (centroid.z - p0.z),
                );
                let b = Point3::new(
                    p1.x + t * (centroid.x - p1.x),
                    p1.y + t * (centroid.y - p1.y),
                    p1.z + t * (centroid.z - p1.z),
                );

                let va = if s == subs {
                    center_v
                } else {
                    let tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
                    vert_idx += 1;
                    model.add_vertex_tagged(a, tag)
                };
                let vb = if s == subs {
                    center_v
                } else {
                    let tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
                    vert_idx += 1;
                    model.add_vertex_tagged(b, tag)
                };

                if s == subs {
                    // Triangle to center
                    let tri = [prev_a, prev_b, center_v];
                    let mut hes = Vec::with_capacity(3);
                    for k in 0..3 {
                        let l = (k + 1) % 3;
                        let etag = Tag::generated(EntityKind::Edge, op, edge_idx);
                        let (_, he, _) = model.add_edge_tagged(tri[k], tri[l], etag);
                        hes.push(he);
                        edge_idx += 1;
                    }
                    let lp = model.make_loop(&hes)?;
                    let ft = Tag::generated(EntityKind::Face, op, face_idx);
                    all_faces.push(model.make_face_tagged(lp, ft));
                    face_idx += 1;
                } else {
                    // Quad strip
                    let quad = [prev_a, prev_b, vb, va];
                    let fh = make_quad_face(model, &quad, op, &mut edge_idx, face_idx)?;
                    all_faces.push(fh);
                    face_idx += 1;
                }

                prev_a = va;
                prev_b = vb;
            }
        }
    }

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&all_faces, shell_tag);
    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let solid = model.make_solid_tagged(&[shell], solid_tag);

    all_points.extend(boundary.iter().copied());

    Ok(SurfaceFillingResult {
        solid,
        faces: all_faces,
    })
}

/// Creates a surface by skinning through cross-section curves.
///
/// Interpolates between successive NURBS curve profiles to create
/// a smooth surface sheet.
pub fn sections(
    model: &mut BRepModel,
    profiles: &[&NurbsCurve],
    segments_u: usize,
) -> KernelResult<SurfaceSectionsResult> {
    // Skinning is essentially surface_from_curves
    surface_from_curves(model, profiles, segments_u).map(|r| SurfaceSectionsResult {
        solid: r.solid,
        faces: r.faces,
    })
}

/// Projects a polyline onto a mesh surface.
///
/// For each input point, finds the closest mesh vertex position,
/// effectively snapping the curve to the mesh.
pub fn curve_on_mesh(
    mesh: &cadkernel_io::tessellate::Mesh,
    curve_points: &[Point3],
) -> CurveOnMeshResult {
    let mut projected = Vec::with_capacity(curve_points.len());

    for &cp in curve_points {
        let mut best_dist = f64::MAX;
        let mut best_pt = cp;

        // Project onto closest mesh vertex
        for &mv in &mesh.vertices {
            let dx = mv.x - cp.x;
            let dy = mv.y - cp.y;
            let dz = mv.z - cp.z;
            let d2 = dx * dx + dy * dy + dz * dz;
            if d2 < best_dist {
                best_dist = d2;
                best_pt = mv;
            }
        }

        projected.push(best_pt);
    }

    CurveOnMeshResult { points: projected }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_geometry::Surface;

    fn make_linear_curve(p0: Point3, p1: Point3) -> NurbsCurve {
        NurbsCurve::new(
            1,
            vec![p0, p1],
            vec![1.0, 1.0],
            vec![0.0, 0.0, 1.0, 1.0],
        )
        .unwrap()
    }

    #[test]
    fn test_ruled_surface_two_lines() {
        let mut model = BRepModel::new();
        let c1 = make_linear_curve(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(10.0, 0.0, 0.0),
        );
        let c2 = make_linear_curve(
            Point3::new(0.0, 5.0, 0.0),
            Point3::new(10.0, 5.0, 0.0),
        );

        let r = ruled_surface(&mut model, &c1, &c2, 4, 2).unwrap();
        // 5 columns * 3 rows = 15 vertices
        assert_eq!(model.vertices.len(), 15);
        // 4 * 2 = 8 quad faces
        assert_eq!(r.faces.len(), 8);
        assert!(model.solids.is_alive(r.solid));
    }

    #[test]
    fn test_surface_from_curves() {
        let mut model = BRepModel::new();
        let c1 = make_linear_curve(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(10.0, 0.0, 0.0),
        );
        let c2 = make_linear_curve(
            Point3::new(0.0, 5.0, 0.0),
            Point3::new(10.0, 5.0, 0.0),
        );
        let c3 = make_linear_curve(
            Point3::new(0.0, 10.0, 3.0),
            Point3::new(10.0, 10.0, 3.0),
        );

        let profiles: Vec<&NurbsCurve> = vec![&c1, &c2, &c3];
        let r = surface_from_curves(&mut model, &profiles, 5).unwrap();
        // 3 profiles * 6 points = 18 vertices
        assert_eq!(model.vertices.len(), 18);
        // 2 strips * 5 quads = 10 faces
        assert_eq!(r.faces.len(), 10);
        assert!(model.solids.is_alive(r.solid));
    }

    #[test]
    fn test_pipe_surface_straight() {
        let mut model = BRepModel::new();
        let path = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.0, 0.0, 5.0),
            Point3::new(0.0, 0.0, 10.0),
        ];

        let r = pipe_surface(&mut model, &path, 1.0, 8).unwrap();
        // 3 stations * 8 segments = 24 vertices
        assert_eq!(model.vertices.len(), 24);
        // 2 caps + 2 * 8 side quads = 18 faces
        assert_eq!(r.faces.len(), 18);
        assert!(model.solids.is_alive(r.solid));
    }

    #[test]
    fn test_pipe_surface_validation() {
        let mut model = BRepModel::new();

        // Too few path points.
        assert!(pipe_surface(
            &mut model,
            &[Point3::ORIGIN],
            1.0,
            8
        )
        .is_err());

        // Non-positive radius.
        assert!(pipe_surface(
            &mut model,
            &[Point3::ORIGIN, Point3::new(0.0, 0.0, 5.0)],
            0.0,
            8
        )
        .is_err());

        // Too few segments.
        assert!(pipe_surface(
            &mut model,
            &[Point3::ORIGIN, Point3::new(0.0, 0.0, 5.0)],
            1.0,
            2
        )
        .is_err());
    }

    #[test]
    fn test_filling_triangle() {
        let mut model = BRepModel::new();
        let boundary = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(10.0, 0.0, 0.0),
            Point3::new(5.0, 10.0, 0.0),
        ];

        let r = filling(&mut model, &boundary, 1).unwrap();
        assert_eq!(r.faces.len(), 3);
        assert!(model.solids.is_alive(r.solid));
    }

    #[test]
    fn test_filling_pentagon() {
        let mut model = BRepModel::new();
        let mut boundary = Vec::new();
        for i in 0..5 {
            let angle = 2.0 * std::f64::consts::PI * i as f64 / 5.0;
            boundary.push(Point3::new(angle.cos() * 5.0, angle.sin() * 5.0, 0.0));
        }

        let r = filling(&mut model, &boundary, 1).unwrap();
        assert_eq!(r.faces.len(), 5);
    }

    #[test]
    fn test_filling_too_few_points() {
        let mut model = BRepModel::new();
        assert!(filling(&mut model, &[Point3::ORIGIN, Point3::new(1.0, 0.0, 0.0)], 1).is_err());
    }

    #[test]
    fn test_sections_skinning() {
        let mut model = BRepModel::new();
        let c1 = make_linear_curve(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(10.0, 0.0, 0.0),
        );
        let c2 = make_linear_curve(
            Point3::new(0.0, 5.0, 2.0),
            Point3::new(10.0, 5.0, 2.0),
        );
        let c3 = make_linear_curve(
            Point3::new(0.0, 10.0, 0.0),
            Point3::new(10.0, 10.0, 0.0),
        );

        let profiles: Vec<&NurbsCurve> = vec![&c1, &c2, &c3];
        let r = sections(&mut model, &profiles, 4).unwrap();
        assert!(!r.faces.is_empty());
        assert!(model.solids.is_alive(r.solid));
    }

    #[test]
    fn test_curve_on_mesh() {
        let mesh = cadkernel_io::tessellate::Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(5.0, 0.0, 0.0),
                Point3::new(10.0, 0.0, 0.0),
                Point3::new(0.0, 5.0, 0.0),
                Point3::new(5.0, 5.0, 1.0),
                Point3::new(10.0, 5.0, 0.0),
            ],
            normals: vec![Vec3::Z; 6],
            indices: vec![[0, 1, 4], [0, 4, 3], [1, 2, 5], [1, 5, 4]],
        };

        let curve_pts = vec![
            Point3::new(2.5, 2.5, 0.5),
            Point3::new(7.5, 2.5, 0.5),
        ];

        let r = curve_on_mesh(&mesh, &curve_pts);
        assert_eq!(r.points.len(), 2);
        // Points should snap to mesh vertices
        for p in &r.points {
            let on_mesh = mesh.vertices.iter().any(|v| {
                (v.x - p.x).abs() < 1e-10 && (v.y - p.y).abs() < 1e-10 && (v.z - p.z).abs() < 1e-10
            });
            assert!(on_mesh, "projected point should be on mesh vertex: {:?}", p);
        }
    }

    #[test]
    fn test_coons_patch_flat() {
        use cadkernel_geometry::LineSegment;
        let u0 = LineSegment::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(10.0, 0.0, 0.0),
        );
        let u1 = LineSegment::new(
            Point3::new(0.0, 10.0, 0.0),
            Point3::new(10.0, 10.0, 0.0),
        );
        let v0 = LineSegment::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.0, 10.0, 0.0),
        );
        let v1 = LineSegment::new(
            Point3::new(10.0, 0.0, 0.0),
            Point3::new(10.0, 10.0, 0.0),
        );

        let result = coons_patch(&u0, &u1, &v0, &v1).unwrap();
        let mid = result.surface.point_at(0.5, 0.5);
        assert!((mid.x - 5.0).abs() < 0.5, "mid.x = {}", mid.x);
        assert!((mid.y - 5.0).abs() < 0.5, "mid.y = {}", mid.y);
        assert!((mid.z).abs() < 0.5, "mid.z = {}", mid.z);
    }

    #[test]
    fn test_coons_patch_unit_square_corners() {
        use cadkernel_geometry::LineSegment;
        let u0 = LineSegment::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0));
        let u1 = LineSegment::new(Point3::new(0.0, 1.0, 0.0), Point3::new(1.0, 1.0, 0.0));
        let v0 = LineSegment::new(Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0));
        let v1 = LineSegment::new(Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0));
        let result = coons_patch(&u0, &u1, &v0, &v1).unwrap();
        let p00 = result.surface.point_at(0.0, 0.0);
        let p11 = result.surface.point_at(1.0, 1.0);
        assert!(p00.x.abs() < 0.1 && p00.y.abs() < 0.1);
        assert!((p11.x - 1.0).abs() < 0.1 && (p11.y - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_coons_patch_elevated_surface() {
        use cadkernel_geometry::LineSegment;
        let u0 = LineSegment::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0));
        let u1 = LineSegment::new(Point3::new(0.0, 1.0, 1.0), Point3::new(1.0, 1.0, 1.0));
        let v0 = LineSegment::new(Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 1.0, 1.0));
        let v1 = LineSegment::new(Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        let result = coons_patch(&u0, &u1, &v0, &v1).unwrap();
        let mid = result.surface.point_at(0.5, 0.5);
        assert!(mid.z > 0.0, "mid z should be elevated");
    }

    #[test]
    fn test_ruled_surface_validation_segments_u() {
        let mut model = BRepModel::new();
        let c1 = make_linear_curve(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0));
        let c2 = make_linear_curve(Point3::new(0.0, 1.0, 0.0), Point3::new(1.0, 1.0, 0.0));
        assert!(ruled_surface(&mut model, &c1, &c2, 0, 4).is_err());
    }

    #[test]
    fn test_ruled_surface_validation_segments_v() {
        let mut model = BRepModel::new();
        let c1 = make_linear_curve(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0));
        let c2 = make_linear_curve(Point3::new(0.0, 1.0, 0.0), Point3::new(1.0, 1.0, 0.0));
        assert!(ruled_surface(&mut model, &c1, &c2, 4, 0).is_err());
    }

    #[test]
    fn test_ruled_surface_single_segment() {
        let mut model = BRepModel::new();
        let c1 = make_linear_curve(Point3::new(0.0, 0.0, 0.0), Point3::new(4.0, 0.0, 0.0));
        let c2 = make_linear_curve(Point3::new(0.0, 2.0, 0.0), Point3::new(4.0, 2.0, 0.0));
        let r = ruled_surface(&mut model, &c1, &c2, 1, 1).unwrap();
        assert_eq!(r.faces.len(), 1);
    }

    #[test]
    fn test_pipe_surface_too_few_points() {
        let mut model = BRepModel::new();
        let path = vec![Point3::new(0.0, 0.0, 0.0)];
        assert!(pipe_surface(&mut model, &path, 1.0, 8).is_err());
    }

    #[test]
    fn test_pipe_surface_invalid_radius() {
        let mut model = BRepModel::new();
        let path = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.0, 0.0, 5.0),
        ];
        assert!(pipe_surface(&mut model, &path, 0.0, 8).is_err());
    }

    #[test]
    fn test_pipe_surface_invalid_segments() {
        let mut model = BRepModel::new();
        let path = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.0, 0.0, 5.0),
        ];
        assert!(pipe_surface(&mut model, &path, 1.0, 2).is_err());
    }

    #[test]
    fn test_pipe_surface_curved_path() {
        let mut model = BRepModel::new();
        let path = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(3.0, 1.0, 0.0),
        ];
        let r = pipe_surface(&mut model, &path, 0.2, 6).unwrap();
        assert!(!r.faces.is_empty());
    }

    #[test]
    fn test_filling_square() {
        let mut model = BRepModel::new();
        let boundary = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let result = filling(&mut model, &boundary, 1).unwrap();
        assert_eq!(result.faces.len(), 4);
    }

    #[test]
    fn test_filling_two_points_fails() {
        let mut model = BRepModel::new();
        let boundary = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
        ];
        assert!(filling(&mut model, &boundary, 1).is_err());
    }

    #[test]
    fn test_filling_with_subdivisions() {
        let mut model = BRepModel::new();
        let boundary = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(2.0, 2.0, 0.0),
            Point3::new(0.0, 2.0, 0.0),
        ];
        let result = filling(&mut model, &boundary, 2).unwrap();
        assert!(!result.faces.is_empty());
    }

    #[test]
    fn test_extend_surface_zero_distance_fails() {
        let mut model = BRepModel::new();
        let b = crate::primitives::make_box(&mut model, Point3::new(0.0, 0.0, 0.0), 2.0, 2.0, 2.0).unwrap();
        assert!(extend_surface(&mut model, b.solid, 0.0).is_err());
    }

    #[test]
    fn test_extend_surface_positive() {
        let mut model = BRepModel::new();
        let b = crate::primitives::make_box(&mut model, Point3::new(0.0, 0.0, 0.0), 2.0, 2.0, 2.0).unwrap();
        let result = extend_surface(&mut model, b.solid, 0.5).unwrap();
        assert_eq!(result.solid, b.solid);
    }

    #[test]
    fn test_extend_surface_negative_shrink() {
        let mut model = BRepModel::new();
        let b = crate::primitives::make_box(&mut model, Point3::new(0.0, 0.0, 0.0), 4.0, 4.0, 4.0).unwrap();
        let result = extend_surface(&mut model, b.solid, -0.5).unwrap();
        assert_eq!(result.solid, b.solid);
    }

    #[test]
    fn test_surface_from_curves_two_profiles() {
        let mut model = BRepModel::new();
        let c1 = make_linear_curve(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 0.0, 0.0));
        let c2 = make_linear_curve(Point3::new(0.0, 1.0, 1.0), Point3::new(2.0, 1.0, 1.0));
        let result = surface_from_curves(&mut model, &[&c1, &c2], 4).unwrap();
        assert!(!result.faces.is_empty());
    }

    #[test]
    fn test_surface_from_curves_single_profile_fails() {
        let mut model = BRepModel::new();
        let c1 = make_linear_curve(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 0.0, 0.0));
        assert!(surface_from_curves(&mut model, &[&c1], 4).is_err());
    }

    #[test]
    fn test_sections_two_profiles() {
        let mut model = BRepModel::new();
        let c1 = make_linear_curve(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 0.0, 0.0));
        let c2 = make_linear_curve(Point3::new(0.0, 0.0, 2.0), Point3::new(2.0, 0.0, 2.0));
        let result = sections(&mut model, &[&c1, &c2], 4).unwrap();
        assert!(!result.faces.is_empty());
    }

    #[test]
    fn test_sections_single_profile_fails() {
        let mut model = BRepModel::new();
        let c1 = make_linear_curve(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 0.0, 0.0));
        assert!(sections(&mut model, &[&c1], 4).is_err());
    }
}
