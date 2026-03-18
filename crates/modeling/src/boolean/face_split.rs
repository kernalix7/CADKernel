//! Face splitting along surface-surface intersection curves.
//!
//! Given two solids whose faces partially overlap, this module:
//! 1. Finds overlapping face pairs (broad phase).
//! 2. Computes intersection curves via SSI marching.
//! 3. Fits intersection point clouds to NURBS curves.
//! 4. Splits affected faces along intersection curves.
//! 5. Returns modified models with split faces ready for boolean classification.

use std::collections::HashMap;
use std::sync::Arc;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_geometry::curve::curve2d::{Curve2D, Line2D, NurbsCurve2D};
use cadkernel_geometry::curve::nurbs_fitting;
use cadkernel_geometry::intersect::surface_surface::{SsiCurve, intersect_surfaces};
use cadkernel_geometry::surface::Surface;
use cadkernel_geometry::NurbsCurve;
use cadkernel_math::{Point2, Point3, Vec3};
use cadkernel_topology::{
    BRepModel, EntityKind, FaceData, Handle, OperationId, SolidData, Tag,
};

use super::broad_phase::{collect_solid_faces, find_overlapping_face_pairs};
use super::classify::face_normal_approx;

/// Tolerance for geometric proximity tests.
const SPLIT_TOL: f64 = 1e-6;

/// Maximum number of SSI points before we subsample.
const MAX_SSI_POINTS: usize = 500;

/// Result of splitting two solids along their intersection.
pub struct BooleanSplitResult {
    /// Model A with faces split along intersection curves.
    pub model_a: BRepModel,
    /// Solid handle in the split model A.
    pub solid_a: Handle<SolidData>,
    /// Model B with faces split along intersection curves.
    pub model_b: BRepModel,
    /// Solid handle in the split model B.
    pub solid_b: Handle<SolidData>,
    /// Whether any faces were actually split.
    pub had_splits: bool,
}

/// An intersection curve in both parameter spaces plus 3D.
#[derive(Debug, Clone)]
struct IntersectionEdge {
    /// 3D points along the intersection.
    points_3d: Vec<Point3>,
    /// UV parameters on surface A (stored for future pcurve generation).
    _params_a: Vec<(f64, f64)>,
    /// UV parameters on surface B (stored for future pcurve generation).
    _params_b: Vec<(f64, f64)>,
}

/// Splits two solids along their mutual intersection curves.
///
/// For each pair of overlapping faces, computes the SSI curve and splits
/// both faces. Returns new models with split topology.
pub fn split_solids_at_intersection(
    model_a: &BRepModel,
    solid_a: Handle<SolidData>,
    model_b: &BRepModel,
    solid_b: Handle<SolidData>,
    tolerance: f64,
) -> KernelResult<BooleanSplitResult> {
    let faces_a = collect_solid_faces(model_a, solid_a)?;
    let faces_b = collect_solid_faces(model_b, solid_b)?;

    // Broad phase: find overlapping face pairs
    let pairs = find_overlapping_face_pairs(model_a, &faces_a, model_b, &faces_b)?;

    if pairs.is_empty() {
        // No overlaps — return copies without splitting
        return no_split_result(model_a, solid_a, model_b, solid_b);
    }

    // For each overlapping pair, compute SSI curves
    let mut splits_a: HashMap<usize, Vec<IntersectionEdge>> = HashMap::new();
    let mut splits_b: HashMap<usize, Vec<IntersectionEdge>> = HashMap::new();

    for &(face_a, face_b) in &pairs {
        let idx_a = faces_a.iter().position(|&f| f == face_a);
        let idx_b = faces_b.iter().position(|&f| f == face_b);

        let (idx_a, idx_b) = match (idx_a, idx_b) {
            (Some(a), Some(b)) => (a, b),
            _ => continue,
        };

        // Get surfaces for SSI
        let surf_a = get_face_surface(model_a, face_a);
        let surf_b = get_face_surface(model_b, face_b);

        let ssi_curves = match (&surf_a, &surf_b) {
            (Some(sa), Some(sb)) => {
                intersect_surfaces(sa.as_ref(), sb.as_ref(), tolerance)
            }
            _ => {
                // No surface binding — use polygon-polygon intersection
                compute_planar_intersection(model_a, face_a, model_b, face_b, tolerance)?
            }
        };

        for curve in &ssi_curves {
            if curve.points.len() < 2 {
                continue;
            }
            let edge = IntersectionEdge {
                points_3d: curve.points.clone(),
                _params_a: curve.params_s1.clone(),
                _params_b: curve.params_s2.clone(),
            };
            splits_a.entry(idx_a).or_default().push(edge.clone());
            splits_b.entry(idx_b).or_default().push(edge);
        }
    }

    let had_splits = !splits_a.is_empty() || !splits_b.is_empty();

    // Build result models with split faces
    let (result_a, new_solid_a) =
        build_split_model(model_a, solid_a, &faces_a, &splits_a, tolerance, "split_a")?;
    let (result_b, new_solid_b) =
        build_split_model(model_b, solid_b, &faces_b, &splits_b, tolerance, "split_b")?;

    Ok(BooleanSplitResult {
        model_a: result_a,
        solid_a: new_solid_a,
        model_b: result_b,
        solid_b: new_solid_b,
        had_splits,
    })
}

/// Fits an SSI point cloud to a NURBS curve.
///
/// If the points are nearly collinear, returns a degree-1 (linear) curve.
/// Otherwise fits a cubic NURBS through the points.
pub fn fit_ssi_to_nurbs(points: &[Point3], tolerance: f64) -> KernelResult<NurbsCurve> {
    if points.len() < 2 {
        return Err(KernelError::InvalidArgument(
            "need at least 2 points for SSI curve fitting".into(),
        ));
    }

    // Subsample if too many points
    let pts = if points.len() > MAX_SSI_POINTS {
        subsample(points, MAX_SSI_POINTS)
    } else {
        points.to_vec()
    };

    if pts.len() <= 4 {
        // Linear interpolation
        return nurbs_fitting::interpolate(&pts, 1.min(pts.len() - 1));
    }

    // Check if nearly collinear
    if is_nearly_collinear(&pts, tolerance * 10.0) {
        return nurbs_fitting::interpolate(&[pts[0], *pts.last().unwrap()], 1);
    }

    // Cubic NURBS interpolation
    let degree = 3.min(pts.len() - 1);
    nurbs_fitting::interpolate(&pts, degree)
}

/// Fits SSI UV parameters to a 2D NURBS pcurve in parameter space.
pub fn fit_ssi_to_pcurve(params: &[(f64, f64)]) -> KernelResult<Arc<dyn Curve2D>> {
    if params.len() < 2 {
        return Err(KernelError::InvalidArgument(
            "need at least 2 parameter pairs for pcurve fitting".into(),
        ));
    }

    if params.len() == 2 {
        let p0 = Point2::new(params[0].0, params[0].1);
        let p1 = Point2::new(params[1].0, params[1].1);
        return Ok(Arc::new(Line2D::new(p0, p1)));
    }

    // Convert to Point3 for 3D fitting, then extract XY as UV
    let pts_3d: Vec<Point3> = params
        .iter()
        .map(|&(u, v)| Point3::new(u, v, 0.0))
        .collect();

    let degree = 3.min(pts_3d.len() - 1);
    let curve = nurbs_fitting::interpolate(&pts_3d, degree)?;

    // Extract control points as 2D
    let cp2d: Vec<Point2> = curve
        .control_points
        .iter()
        .map(|p| Point2::new(p.x, p.y))
        .collect();

    Ok(Arc::new(NurbsCurve2D::new(
        curve.degree(),
        cp2d,
        curve.weights().to_vec(),
        curve.knots().to_vec(),
    )?))
}

/// Gets the bound surface from a face, if any.
fn get_face_surface(model: &BRepModel, face: Handle<FaceData>) -> Option<Arc<dyn Surface + Send + Sync>> {
    let face_data = model.faces.get(face)?;
    face_data.surface.clone()
}

/// Computes intersection between two planar faces using polygon clipping.
fn compute_planar_intersection(
    model_a: &BRepModel,
    face_a: Handle<FaceData>,
    model_b: &BRepModel,
    face_b: Handle<FaceData>,
    tolerance: f64,
) -> KernelResult<Vec<SsiCurve>> {
    let poly_a = face_polygon_3d(model_a, face_a)?;
    let poly_b = face_polygon_3d(model_b, face_b)?;

    if poly_a.len() < 3 || poly_b.len() < 3 {
        return Ok(Vec::new());
    }

    let normal_a = face_normal_approx(model_a, face_a)?;
    let normal_b = face_normal_approx(model_b, face_b)?;

    // If faces are coplanar (normals parallel and on same plane), skip
    let cross = normal_a.cross(normal_b);
    if cross.length() < 1e-10 {
        return Ok(Vec::new());
    }

    // Find intersection line direction
    let line_dir = cross.normalized().unwrap_or(Vec3::Z);

    // Find intersection points: edges of A crossing plane of B, and vice versa
    let plane_b_d = normal_b.dot(Vec3::from(poly_b[0]));
    let plane_a_d = normal_a.dot(Vec3::from(poly_a[0]));

    let mut intersection_points = Vec::new();

    // Edges of A crossing plane of B
    for i in 0..poly_a.len() {
        let p0 = poly_a[i];
        let p1 = poly_a[(i + 1) % poly_a.len()];
        let d0 = normal_b.dot(Vec3::from(p0)) - plane_b_d;
        let d1 = normal_b.dot(Vec3::from(p1)) - plane_b_d;

        if d0.abs() < tolerance {
            add_unique_point(&mut intersection_points, p0, tolerance);
        }
        if (d0 > tolerance && d1 < -tolerance) || (d0 < -tolerance && d1 > tolerance) {
            let t = d0 / (d0 - d1);
            let ip = Point3::new(
                p0.x + t * (p1.x - p0.x),
                p0.y + t * (p1.y - p0.y),
                p0.z + t * (p1.z - p0.z),
            );
            add_unique_point(&mut intersection_points, ip, tolerance);
        }
    }

    // Edges of B crossing plane of A
    for i in 0..poly_b.len() {
        let p0 = poly_b[i];
        let p1 = poly_b[(i + 1) % poly_b.len()];
        let d0 = normal_a.dot(Vec3::from(p0)) - plane_a_d;
        let d1 = normal_a.dot(Vec3::from(p1)) - plane_a_d;

        if d0.abs() < tolerance {
            add_unique_point(&mut intersection_points, p0, tolerance);
        }
        if (d0 > tolerance && d1 < -tolerance) || (d0 < -tolerance && d1 > tolerance) {
            let t = d0 / (d0 - d1);
            let ip = Point3::new(
                p0.x + t * (p1.x - p0.x),
                p0.y + t * (p1.y - p0.y),
                p0.z + t * (p1.z - p0.z),
            );
            add_unique_point(&mut intersection_points, ip, tolerance);
        }
    }

    if intersection_points.len() < 2 {
        return Ok(Vec::new());
    }

    // Sort points along the intersection line direction
    let origin = intersection_points[0];
    intersection_points.sort_by(|a, b| {
        let da = line_dir.dot(Vec3::from(*a) - Vec3::from(origin));
        let db = line_dir.dot(Vec3::from(*b) - Vec3::from(origin));
        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Filter to points that are inside both polygons (approximately)
    let mut valid_points = Vec::new();
    for &p in &intersection_points {
        if point_near_polygon(&p, &poly_a, tolerance * 100.0)
            && point_near_polygon(&p, &poly_b, tolerance * 100.0)
        {
            valid_points.push(p);
        }
    }

    if valid_points.len() < 2 {
        return Ok(Vec::new());
    }

    // Create dummy params (no surface binding for planar faces)
    let n = valid_points.len();
    let params: Vec<(f64, f64)> = (0..n)
        .map(|i| (i as f64 / (n - 1).max(1) as f64, 0.0))
        .collect();

    Ok(vec![SsiCurve {
        points: valid_points,
        params_s1: params.clone(),
        params_s2: params,
    }])
}

/// Builds a new model with faces split along intersection edges.
fn build_split_model(
    src: &BRepModel,
    _src_solid: Handle<SolidData>,
    faces: &[Handle<FaceData>],
    splits: &HashMap<usize, Vec<IntersectionEdge>>,
    tolerance: f64,
    op_name: &str,
) -> KernelResult<(BRepModel, Handle<SolidData>)> {
    let mut dst = BRepModel::new();
    let op = dst.history.next_operation(op_name);
    let mut all_faces = Vec::new();
    let mut face_counter = 0u32;

    for (idx, &face_h) in faces.iter().enumerate() {
        if let Some(int_edges) = splits.get(&idx) {
            // This face needs splitting
            let sub_faces = split_face_along_curves(
                src, face_h, int_edges, &mut dst, op, &mut face_counter, tolerance,
            )?;
            all_faces.extend(sub_faces);
        } else {
            // Copy face unchanged
            let new_face = copy_face_with_geometry(src, face_h, &mut dst, op, face_counter)?;
            all_faces.push(new_face);
            face_counter += 1;
        }
    }

    if all_faces.is_empty() {
        return Err(KernelError::InvalidArgument(
            "split produced no faces".into(),
        ));
    }

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = dst.make_shell_tagged(&all_faces, shell_tag);
    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let solid = dst.make_solid_tagged(&[shell], solid_tag);

    Ok((dst, solid))
}

/// Splits a single face along one or more intersection curves.
///
/// The approach:
/// 1. Collect the face boundary polygon.
/// 2. For each intersection curve, find where it enters/exits the face boundary.
/// 3. Insert intersection vertices into the boundary.
/// 4. Split the boundary into sub-polygons.
/// 5. Create new faces for each sub-polygon.
fn split_face_along_curves(
    src: &BRepModel,
    face_h: Handle<FaceData>,
    int_edges: &[IntersectionEdge],
    dst: &mut BRepModel,
    op: OperationId,
    face_counter: &mut u32,
    tolerance: f64,
) -> KernelResult<Vec<Handle<FaceData>>> {
    let boundary = face_polygon_3d(src, face_h)?;
    if boundary.len() < 3 {
        // Degenerate face, just copy
        let f = copy_face_with_geometry(src, face_h, dst, op, *face_counter)?;
        *face_counter += 1;
        return Ok(vec![f]);
    }

    // Collect all intersection curves clipped to this face
    let mut all_clip_curves = Vec::new();
    for edge in int_edges {
        let clipped = clip_curve_to_polygon(&edge.points_3d, &boundary, tolerance);
        for segment in clipped {
            if segment.len() >= 2 {
                all_clip_curves.push(segment);
            }
        }
    }

    if all_clip_curves.is_empty() {
        // No intersection actually crosses this face
        let f = copy_face_with_geometry(src, face_h, dst, op, *face_counter)?;
        *face_counter += 1;
        return Ok(vec![f]);
    }

    // For each intersection curve segment, find entry/exit on boundary edges
    let split_polygons = split_polygon_with_curves(&boundary, &all_clip_curves, tolerance);

    if split_polygons.is_empty() {
        // Splitting failed — keep original
        let f = copy_face_with_geometry(src, face_h, dst, op, *face_counter)?;
        *face_counter += 1;
        return Ok(vec![f]);
    }

    // Create faces from sub-polygons
    let mut result_faces = Vec::new();
    let src_tag = src.faces.get(face_h).and_then(|f| f.tag.clone());
    let surface = get_face_surface(src, face_h);

    for poly in &split_polygons {
        if poly.len() < 3 {
            continue;
        }
        let new_face = create_face_from_polygon(dst, poly, op, *face_counter, &src_tag, &surface)?;
        result_faces.push(new_face);
        *face_counter += 1;
    }

    if result_faces.is_empty() {
        // Fallback: copy original
        let f = copy_face_with_geometry(src, face_h, dst, op, *face_counter)?;
        *face_counter += 1;
        return Ok(vec![f]);
    }

    Ok(result_faces)
}

/// Splits a polygon boundary using intersection curves that cross it.
///
/// For each curve that enters at edge i and exits at edge j:
/// - Creates two sub-polygons by inserting the curve points at the boundary.
fn split_polygon_with_curves(
    boundary: &[Point3],
    curves: &[Vec<Point3>],
    tolerance: f64,
) -> Vec<Vec<Point3>> {
    if curves.is_empty() {
        return vec![boundary.to_vec()];
    }

    let mut current_polygons = vec![boundary.to_vec()];

    for curve in curves {
        if curve.len() < 2 {
            continue;
        }

        let entry = curve[0];
        let exit = curve[curve.len() - 1];

        let mut next_polygons = Vec::new();

        for poly in &current_polygons {
            let split = split_single_polygon(poly, &entry, &exit, curve, tolerance);
            next_polygons.extend(split);
        }

        current_polygons = next_polygons;
    }

    current_polygons
}

/// Splits a single polygon using one intersection curve (entry→exit).
fn split_single_polygon(
    poly: &[Point3],
    entry: &Point3,
    exit: &Point3,
    curve: &[Point3],
    tolerance: f64,
) -> Vec<Vec<Point3>> {
    let n = poly.len();

    // Find which edge the entry point is on
    let entry_edge = find_edge_for_point(poly, entry, tolerance);
    let exit_edge = find_edge_for_point(poly, exit, tolerance);

    let (entry_edge, exit_edge) = match (entry_edge, exit_edge) {
        (Some(e), Some(x)) => (e, x),
        _ => return vec![poly.to_vec()], // can't split
    };

    if entry_edge == exit_edge {
        // Entry and exit on same edge — can't properly split
        return vec![poly.to_vec()];
    }

    // Build two sub-polygons:
    // Polygon 1: boundary[entry_edge+1 .. exit_edge] + reversed curve
    // Polygon 2: boundary[exit_edge+1 .. entry_edge] + curve
    let mut poly1 = Vec::new();
    let mut poly2 = Vec::new();

    // Polygon 1: entry → boundary → exit → reversed curve back to entry
    poly1.push(*entry);
    let mut i = (entry_edge + 1) % n;
    loop {
        poly1.push(poly[i]);
        if i == exit_edge {
            break;
        }
        i = (i + 1) % n;
        if poly1.len() > n + curve.len() {
            break; // safety
        }
    }
    poly1.push(*exit);
    // Add reversed interior curve points (skip first=entry, last=exit)
    for j in (1..curve.len() - 1).rev() {
        poly1.push(curve[j]);
    }

    // Polygon 2: exit → boundary → entry → curve
    poly2.push(*exit);
    let mut i = (exit_edge + 1) % n;
    loop {
        poly2.push(poly[i]);
        if i == entry_edge {
            break;
        }
        i = (i + 1) % n;
        if poly2.len() > n + curve.len() {
            break; // safety
        }
    }
    poly2.push(*entry);
    // Add interior curve points
    for pt in curve.iter().take(curve.len() - 1).skip(1) {
        poly2.push(*pt);
    }

    let mut result = Vec::new();
    if poly1.len() >= 3 {
        result.push(poly1);
    }
    if poly2.len() >= 3 {
        result.push(poly2);
    }

    if result.is_empty() {
        vec![poly.to_vec()]
    } else {
        result
    }
}

/// Finds which polygon edge a point lies on.
/// Returns the edge index (edge from poly[i] to poly[(i+1)%n]).
fn find_edge_for_point(poly: &[Point3], point: &Point3, tolerance: f64) -> Option<usize> {
    let n = poly.len();
    let mut best_edge = None;
    let mut best_dist = f64::MAX;

    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + 1) % n];
        let dist = point_to_segment_distance(point, &a, &b);
        if dist < tolerance * 10.0 && dist < best_dist {
            best_dist = dist;
            best_edge = Some(i);
        }
    }
    best_edge
}

/// Distance from a point to a line segment.
fn point_to_segment_distance(p: &Point3, a: &Point3, b: &Point3) -> f64 {
    let ab = *b - *a;
    let ap = *p - *a;
    let len_sq = ab.dot(ab);
    if len_sq < 1e-20 {
        return ap.length();
    }
    let t = ap.dot(ab) / len_sq;
    let t = t.clamp(0.0, 1.0);
    let closest = Point3::new(a.x + t * ab.x, a.y + t * ab.y, a.z + t * ab.z);
    p.distance_to(closest)
}

/// Clips an intersection curve to the interior of a polygon.
///
/// Returns segments of the curve that are inside the polygon.
fn clip_curve_to_polygon(
    curve_points: &[Point3],
    polygon: &[Point3],
    tolerance: f64,
) -> Vec<Vec<Point3>> {
    if curve_points.len() < 2 || polygon.len() < 3 {
        return Vec::new();
    }

    // Compute polygon normal and project everything to 2D
    let normal = polygon_normal(polygon);
    let (drop_axis, _) = largest_axis(&normal);

    let poly2d: Vec<(f64, f64)> = polygon.iter().map(|p| project_drop(p, drop_axis)).collect();
    let curve2d: Vec<(f64, f64)> = curve_points
        .iter()
        .map(|p| project_drop(p, drop_axis))
        .collect();

    let mut segments = Vec::new();
    let mut current_segment = Vec::new();

    for (i, &cp) in curve_points.iter().enumerate() {
        let inside = point_in_polygon_2d_raw(&curve2d[i], &poly2d);

        if inside {
            current_segment.push(cp);
        } else if !current_segment.is_empty() {
            // Find the exit point on the polygon boundary
            if current_segment.len() >= 2 {
                segments.push(current_segment.clone());
            }
            current_segment.clear();
        }
    }

    if current_segment.len() >= 2 {
        segments.push(current_segment);
    }

    // For each segment, snap endpoints to polygon edges
    for seg in &mut segments {
        if let Some(first) = seg.first_mut() {
            if let Some(snapped) = snap_to_polygon_edge(first, polygon, tolerance * 10.0) {
                *first = snapped;
            }
        }
        if let Some(last) = seg.last_mut() {
            if let Some(snapped) = snap_to_polygon_edge(last, polygon, tolerance * 10.0) {
                *last = snapped;
            }
        }
    }

    segments
}

/// Snaps a point to the nearest polygon edge if within tolerance.
fn snap_to_polygon_edge(point: &Point3, polygon: &[Point3], tolerance: f64) -> Option<Point3> {
    let n = polygon.len();
    let mut best_dist = f64::MAX;
    let mut best_point = None;

    for i in 0..n {
        let a = polygon[i];
        let b = polygon[(i + 1) % n];
        let ab = b - a;
        let ap = *point - a;
        let len_sq = ab.dot(ab);
        if len_sq < 1e-20 {
            continue;
        }
        let t = (ap.dot(ab) / len_sq).clamp(0.0, 1.0);
        let closest = Point3::new(a.x + t * ab.x, a.y + t * ab.y, a.z + t * ab.z);
        let dist = point.distance_to(closest);
        if dist < tolerance && dist < best_dist {
            best_dist = dist;
            best_point = Some(closest);
        }
    }

    best_point
}

/// Creates a face from a polygon of 3D points.
fn create_face_from_polygon(
    model: &mut BRepModel,
    polygon: &[Point3],
    op: OperationId,
    index: u32,
    parent_tag: &Option<Tag>,
    surface: &Option<Arc<dyn Surface + Send + Sync>>,
) -> KernelResult<Handle<FaceData>> {
    let n = polygon.len();
    let mut verts = Vec::with_capacity(n);
    let mut positions = Vec::new();

    for &p in polygon {
        let existing = positions.iter().position(|q: &Point3| q.approx_eq(p));
        if let Some(idx) = existing {
            verts.push(verts[idx]);
        } else {
            let v = model.add_vertex(p);
            positions.push(p);
            verts.push(v);
        }
    }

    let mut hes = Vec::with_capacity(n);
    for i in 0..n {
        let vs = verts[i];
        let ve = verts[(i + 1) % n];
        if vs == ve {
            continue;
        }
        let (_, he, _) = model.add_edge(vs, ve);
        hes.push(he);
    }

    if hes.len() < 3 {
        return Err(KernelError::InvalidArgument(
            "degenerate polygon after vertex dedup".into(),
        ));
    }

    let new_loop = model.make_loop(&hes)?;
    let tag = parent_tag
        .clone()
        .unwrap_or_else(|| Tag::generated(EntityKind::Face, op, index));

    let face = model.make_face_tagged(new_loop, tag);

    // Bind surface if available
    if let Some(surf) = surface {
        model.bind_face_surface(
            face,
            surf.clone(),
            cadkernel_topology::Orientation::Forward,
        );
    }

    Ok(face)
}

/// Copies a face preserving geometry binding.
fn copy_face_with_geometry(
    src: &BRepModel,
    face_h: Handle<FaceData>,
    dst: &mut BRepModel,
    op: OperationId,
    index: u32,
) -> KernelResult<Handle<FaceData>> {
    let face_data = src
        .faces
        .get(face_h)
        .ok_or(KernelError::InvalidHandle("face"))?;
    let loop_data = src
        .loops
        .get(face_data.outer_loop)
        .ok_or(KernelError::InvalidHandle("loop"))?;
    let src_hes = src.loop_half_edges(loop_data.half_edge);

    let mut new_verts = Vec::new();
    let mut src_positions = Vec::new();

    for &he_h in &src_hes {
        let he = src
            .half_edges
            .get(he_h)
            .ok_or(KernelError::InvalidHandle("half_edge"))?;
        let v = src
            .vertices
            .get(he.origin)
            .ok_or(KernelError::InvalidHandle("vertex"))?;
        let existing = src_positions.iter().position(|p: &Point3| p.approx_eq(v.point));
        if let Some(idx) = existing {
            new_verts.push(new_verts[idx]);
        } else {
            let new_v = dst.add_vertex(v.point);
            src_positions.push(v.point);
            new_verts.push(new_v);
        }
    }

    let n = new_verts.len();
    let mut new_hes = Vec::new();
    for i in 0..n {
        let vs = new_verts[i];
        let ve = new_verts[(i + 1) % n];
        let (edge_h, he, _) = dst.add_edge(vs, ve);

        // Copy edge curve binding
        if let Some(src_he) = src.half_edges.get(src_hes[i]) {
            if let Some(edge_handle) = src_he.edge {
                if let Some(src_edge) = src.edges.get(edge_handle) {
                    if let Some(ref curve) = src_edge.curve {
                        let domain = src_edge.curve_domain.unwrap_or((0.0, 1.0));
                        dst.bind_edge_curve(edge_h, curve.clone(), domain);
                    }
                }
            }
        }

        new_hes.push(he);
    }

    let new_loop = dst.make_loop(&new_hes)?;

    let tag = face_data
        .tag
        .clone()
        .unwrap_or_else(|| Tag::generated(EntityKind::Face, op, index));

    let new_face = dst.make_face_tagged(new_loop, tag);

    // Copy surface binding
    if let Some(ref surface) = face_data.surface {
        dst.bind_face_surface(new_face, surface.clone(), face_data.orientation);
    }

    // Copy trim binding
    if let Some(ref outer_trim) = face_data.outer_trim {
        dst.bind_face_trim(new_face, outer_trim.clone(), face_data.inner_trims.clone());
    }

    Ok(new_face)
}

/// No-split result: copies both models without modification.
fn no_split_result(
    model_a: &BRepModel,
    solid_a: Handle<SolidData>,
    model_b: &BRepModel,
    solid_b: Handle<SolidData>,
) -> KernelResult<BooleanSplitResult> {
    let faces_a = collect_solid_faces(model_a, solid_a)?;
    let faces_b = collect_solid_faces(model_b, solid_b)?;

    let empty_splits = HashMap::new();
    let (result_a, new_solid_a) =
        build_split_model(model_a, solid_a, &faces_a, &empty_splits, SPLIT_TOL, "copy_a")?;
    let (result_b, new_solid_b) =
        build_split_model(model_b, solid_b, &faces_b, &empty_splits, SPLIT_TOL, "copy_b")?;

    Ok(BooleanSplitResult {
        model_a: result_a,
        solid_a: new_solid_a,
        model_b: result_b,
        solid_b: new_solid_b,
        had_splits: false,
    })
}

// ─── Utility functions ───────────────────────────────────────────────────

fn face_polygon_3d(model: &BRepModel, face: Handle<FaceData>) -> KernelResult<Vec<Point3>> {
    let face_data = model
        .faces
        .get(face)
        .ok_or(KernelError::InvalidHandle("face"))?;
    let loop_data = model
        .loops
        .get(face_data.outer_loop)
        .ok_or(KernelError::InvalidHandle("loop"))?;
    let hes = model.loop_half_edges(loop_data.half_edge);

    let mut pts = Vec::with_capacity(hes.len());
    for he_h in hes {
        let he = model
            .half_edges
            .get(he_h)
            .ok_or(KernelError::InvalidHandle("half_edge"))?;
        let v = model
            .vertices
            .get(he.origin)
            .ok_or(KernelError::InvalidHandle("vertex"))?;
        pts.push(v.point);
    }
    Ok(pts)
}

fn polygon_normal(polygon: &[Point3]) -> Vec3 {
    if polygon.len() < 3 {
        return Vec3::Z;
    }
    let e1 = polygon[1] - polygon[0];
    let e2 = polygon[2] - polygon[0];
    e1.cross(e2).normalized().unwrap_or(Vec3::Z)
}

fn largest_axis(v: &Vec3) -> (usize, f64) {
    let ax = v.x.abs();
    let ay = v.y.abs();
    let az = v.z.abs();
    if ax >= ay && ax >= az {
        (0, ax)
    } else if ay >= az {
        (1, ay)
    } else {
        (2, az)
    }
}

fn project_drop(p: &Point3, axis: usize) -> (f64, f64) {
    match axis {
        0 => (p.y, p.z),
        1 => (p.x, p.z),
        _ => (p.x, p.y),
    }
}

fn point_in_polygon_2d_raw(point: &(f64, f64), polygon: &[(f64, f64)]) -> bool {
    let (px, py) = *point;
    let n = polygon.len();
    let mut crossings = 0u32;
    for i in 0..n {
        let (ax, ay) = polygon[i];
        let (bx, by) = polygon[(i + 1) % n];
        if (ay <= py && by > py) || (by <= py && ay > py) {
            let t = (py - ay) / (by - ay);
            let ix = ax + t * (bx - ax);
            if px < ix {
                crossings += 1;
            }
        }
    }
    crossings % 2 == 1
}

fn point_near_polygon(point: &Point3, polygon: &[Point3], tolerance: f64) -> bool {
    let normal = polygon_normal(polygon);
    let (drop_axis, _) = largest_axis(&normal);
    let p2d = project_drop(point, drop_axis);
    let poly2d: Vec<(f64, f64)> = polygon.iter().map(|p| project_drop(p, drop_axis)).collect();

    // Check if inside or on boundary
    if point_in_polygon_2d_raw(&p2d, &poly2d) {
        return true;
    }

    // Check if near any edge
    for i in 0..polygon.len() {
        let dist = point_to_segment_distance(point, &polygon[i], &polygon[(i + 1) % polygon.len()]);
        if dist < tolerance {
            return true;
        }
    }
    false
}

fn add_unique_point(points: &mut Vec<Point3>, p: Point3, tolerance: f64) {
    if !points.iter().any(|q| q.distance_to(p) < tolerance) {
        points.push(p);
    }
}

fn is_nearly_collinear(points: &[Point3], tolerance: f64) -> bool {
    if points.len() < 3 {
        return true;
    }
    let dir = (points[points.len() - 1] - points[0])
        .normalized()
        .unwrap_or(Vec3::X);
    for p in &points[1..points.len() - 1] {
        let v = *p - points[0];
        let proj = dir * v.dot(dir);
        let perp = v - proj;
        if perp.length() > tolerance {
            return false;
        }
    }
    true
}

fn subsample(points: &[Point3], max: usize) -> Vec<Point3> {
    if points.len() <= max {
        return points.to_vec();
    }
    let step = points.len() as f64 / max as f64;
    let mut result = Vec::with_capacity(max);
    for i in 0..max {
        let idx = (i as f64 * step) as usize;
        result.push(points[idx.min(points.len() - 1)]);
    }
    // Ensure last point is included
    if let Some(&last) = points.last() {
        if let Some(r_last) = result.last_mut() {
            *r_last = last;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_geometry::Curve;
    use crate::primitives::make_box;

    #[test]
    fn test_fit_ssi_line() {
        let points = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        ];
        let curve = fit_ssi_to_nurbs(&points, 0.01).unwrap();
        let mid = curve.point_at(0.5);
        assert!(mid.distance_to(Point3::new(1.0, 0.0, 0.0)) < 0.1);
    }

    #[test]
    fn test_fit_ssi_arc() {
        let n = 20;
        let points: Vec<Point3> = (0..n)
            .map(|i| {
                let t = std::f64::consts::PI * i as f64 / (n - 1) as f64;
                Point3::new(t.cos(), t.sin(), 0.0)
            })
            .collect();
        let curve = fit_ssi_to_nurbs(&points, 0.01).unwrap();

        // Check a few points
        let start = curve.point_at(0.0);
        assert!(start.distance_to(points[0]) < 0.1);
    }

    #[test]
    fn test_fit_pcurve() {
        let params = vec![(0.0, 0.0), (0.5, 0.5), (1.0, 0.0)];
        let pcurve = fit_ssi_to_pcurve(&params).unwrap();
        let mid = pcurve.point_at(0.5);
        // Should be somewhere near (0.5, 0.5)
        assert!(mid.x > 0.1 && mid.x < 0.9);
    }

    #[test]
    fn test_split_disjoint_boxes() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(5.0, 5.0, 5.0), 1.0, 1.0, 1.0).unwrap();

        let result =
            split_solids_at_intersection(&a, ra.solid, &b, rb.solid, 0.001).unwrap();
        assert!(!result.had_splits);
        // Both models should have 6 faces
        assert_eq!(collect_solid_faces(&result.model_a, result.solid_a).unwrap().len(), 6);
        assert_eq!(collect_solid_faces(&result.model_b, result.solid_b).unwrap().len(), 6);
    }

    #[test]
    fn test_split_overlapping_boxes() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(1.0, 1.0, 1.0), 2.0, 2.0, 2.0).unwrap();

        let result =
            split_solids_at_intersection(&a, ra.solid, &b, rb.solid, 0.001).unwrap();
        // Overlapping boxes should produce split faces
        let faces_a = collect_solid_faces(&result.model_a, result.solid_a).unwrap();
        let faces_b = collect_solid_faces(&result.model_b, result.solid_b).unwrap();
        // At minimum, should have all original faces (some may be split)
        assert!(faces_a.len() >= 6);
        assert!(faces_b.len() >= 6);
    }

    #[test]
    fn test_point_to_segment() {
        let a = Point3::ORIGIN;
        let b = Point3::new(1.0, 0.0, 0.0);
        let p = Point3::new(0.5, 0.0, 0.0);
        assert!(point_to_segment_distance(&p, &a, &b) < 1e-10);

        let p2 = Point3::new(0.5, 1.0, 0.0);
        assert!((point_to_segment_distance(&p2, &a, &b) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_clip_curve_to_polygon() {
        let polygon = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(2.0, 2.0, 0.0),
            Point3::new(0.0, 2.0, 0.0),
        ];
        // Curve that passes through the polygon
        let curve = vec![
            Point3::new(-1.0, 1.0, 0.0),
            Point3::new(0.5, 1.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(1.5, 1.0, 0.0),
            Point3::new(3.0, 1.0, 0.0),
        ];
        let clipped = clip_curve_to_polygon(&curve, &polygon, 0.01);
        assert!(!clipped.is_empty(), "should have at least one segment inside");
        // The clipped segment should be in the interior
        for seg in &clipped {
            for p in seg {
                assert!(p.x >= -0.1 && p.x <= 2.1);
            }
        }
    }

    #[test]
    fn test_polygon_split() {
        let poly = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(2.0, 2.0, 0.0),
            Point3::new(0.0, 2.0, 0.0),
        ];
        // Curve from bottom edge to top edge
        let curve = vec![
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(1.0, 2.0, 0.0),
        ];
        let result = split_polygon_with_curves(&poly, &[curve], 0.01);
        assert_eq!(result.len(), 2, "square should split into 2 polygons");
        for sub in &result {
            assert!(sub.len() >= 3, "each sub-polygon should have ≥3 vertices");
        }
    }
}
