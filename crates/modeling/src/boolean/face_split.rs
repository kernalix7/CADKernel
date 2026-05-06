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
use cadkernel_geometry::NurbsCurve;
use cadkernel_geometry::curve::curve2d::{Curve2D, Line2D, NurbsCurve2D};
use cadkernel_geometry::curve::nurbs_fitting;
use cadkernel_geometry::intersect::surface_surface::{SsiCurve, intersect_surfaces};
use cadkernel_geometry::surface::Surface;
use cadkernel_math::{Point2, Point3, Vec3};
use cadkernel_topology::{
    BRepModel, EntityKind, FaceData, HalfEdgeData, Handle, OperationId, SolidData, Tag, VertexData,
};

const SPLIT_VERT_MERGE_TOL: f64 = 1e-6;

/// Position-based vertex and directed half-edge dedup for building a
/// single BRepModel with shared topology across adjacent faces.
struct SplitBuilder {
    vertex_positions: Vec<Point3>,
    vertex_handles: Vec<Handle<VertexData>>,
    edge_map: HashMap<(u32, u32), Handle<HalfEdgeData>>,
}

impl SplitBuilder {
    fn new() -> Self {
        Self {
            vertex_positions: Vec::new(),
            vertex_handles: Vec::new(),
            edge_map: HashMap::new(),
        }
    }

    fn vertex(&mut self, dst: &mut BRepModel, point: Point3) -> Handle<VertexData> {
        for (i, p) in self.vertex_positions.iter().enumerate() {
            if p.distance_to(point) < SPLIT_VERT_MERGE_TOL {
                return self.vertex_handles[i];
            }
        }
        let h = dst.add_vertex(point);
        self.vertex_positions.push(point);
        self.vertex_handles.push(h);
        h
    }

    fn directed_half_edge(
        &mut self,
        dst: &mut BRepModel,
        v_start: Handle<VertexData>,
        v_end: Handle<VertexData>,
    ) -> Handle<HalfEdgeData> {
        let key_fwd = (v_start.index(), v_end.index());
        // Reuse a cached half-edge only when it is not yet bound to a loop.
        // Every half-edge in a manifold B-Rep belongs to exactly one loop,
        // so sharing a half-edge across two loops would overwrite the
        // previous loop's next/prev pointers and corrupt topology.
        if let Some(&he) = self.edge_map.get(&key_fwd) {
            if dst.half_edges.get(he).is_some_and(|h| h.loop_ref.is_none()) {
                return he;
            }
        }
        let key_rev = (v_end.index(), v_start.index());
        if let Some(&he_rev) = self.edge_map.get(&key_rev) {
            if let Some(he_rev_data) = dst.half_edges.get(he_rev) {
                if let Some(twin) = he_rev_data.twin {
                    if dst
                        .half_edges
                        .get(twin)
                        .is_some_and(|h| h.loop_ref.is_none())
                    {
                        self.edge_map.insert(key_fwd, twin);
                        return twin;
                    }
                }
            }
        }
        let (_, he_a, he_b) = dst.add_edge(v_start, v_end);
        self.edge_map.insert(key_fwd, he_a);
        self.edge_map.insert(key_rev, he_b);
        he_a
    }
}

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
                let marched = intersect_surfaces(sa.as_ref(), sb.as_ref(), tolerance);
                if marched.is_empty() || marched.iter().all(|c| c.points.len() < 2) {
                    // Marching failed (common for unbounded surfaces like planes
                    // that have no useful sampling domain). Fall back to
                    // polygon-polygon intersection of the face outer loops.
                    compute_planar_intersection(model_a, face_a, model_b, face_b, tolerance)?
                } else {
                    marched
                }
            }
            _ => compute_planar_intersection(model_a, face_a, model_b, face_b, tolerance)?,
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
fn get_face_surface(
    model: &BRepModel,
    face: Handle<FaceData>,
) -> Option<Arc<dyn Surface + Send + Sync>> {
    let face_data = model.faces.get(face)?;
    face_data.surface.clone()
}

/// Generates split curves for two COPLANAR overlapping faces. Each edge
/// of `poly_b` is clipped against `poly_a` to produce the 3D segments
/// that lie on B's boundary and inside A. These segments, fed into the
/// standard split pipeline, let A's face be carved by B's boundary
/// (e.g. `top(A)` minus `top(B)` for a box-subtract-box L-shape).
fn coplanar_split_curves(poly_a: &[Point3], poly_b: &[Point3], tolerance: f64) -> Vec<SsiCurve> {
    if poly_a.len() < 3 || poly_b.len() < 3 {
        return Vec::new();
    }
    let normal_a = polygon_normal(poly_a);
    let (drop_axis, _) = largest_axis(&normal_a);
    let poly_a_2d: Vec<(f64, f64)> = poly_a.iter().map(|p| project_drop(p, drop_axis)).collect();

    let mut curves: Vec<SsiCurve> = Vec::new();
    let n_b = poly_b.len();
    for i in 0..n_b {
        let p0 = poly_b[i];
        let p1 = poly_b[(i + 1) % n_b];
        // A B-edge is an open 2-point "curve". Clip it against A's polygon.
        let clipped = clip_curve_to_polygon(&[p0, p1], poly_a, tolerance);
        for seg in clipped {
            if seg.len() < 2 {
                continue;
            }
            // Discard segments that lie entirely on A's boundary — they
            // are collinear with an A-edge and cannot split the polygon.
            let mid = Point3::new(
                (seg[0].x + seg[seg.len() - 1].x) * 0.5,
                (seg[0].y + seg[seg.len() - 1].y) * 0.5,
                (seg[0].z + seg[seg.len() - 1].z) * 0.5,
            );
            let mid_2d = project_drop(&mid, drop_axis);
            let boundary_tol = tolerance * 10.0;
            if point_on_polygon_boundary_2d(&mid_2d, &poly_a_2d, boundary_tol) {
                continue;
            }

            let n = seg.len();
            let params: Vec<(f64, f64)> = (0..n)
                .map(|i| (i as f64 / (n - 1).max(1) as f64, 0.0))
                .collect();
            curves.push(SsiCurve {
                points: seg,
                params_s1: params.clone(),
                params_s2: params,
            });
        }
    }
    curves
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

    // Coplanar face pair: produce split curves from B's boundary edges
    // projected into A's plane, clipped to A's polygon. This is what
    // non-coplanar SSI does implicitly via the intersection line; for
    // coplanar overlap we need the full boundary of B (restricted to A)
    // so downstream splitting can carve A into "inside B" and "outside B"
    // sub-polygons.
    let cross = normal_a.cross(normal_b);
    if cross.length() < 1e-10 {
        // Confirm same plane (B's first vertex lies on A's plane).
        let plane_a_d = normal_a.dot(Vec3::from(poly_a[0]));
        let offset = (normal_a.dot(Vec3::from(poly_b[0])) - plane_a_d).abs();
        if offset > tolerance * 100.0 {
            return Ok(Vec::new());
        }
        // Build coplanar split curves from B's polygon edges.
        let curves = coplanar_split_curves(&poly_a, &poly_b, tolerance);
        return Ok(curves);
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
    let mut builder = SplitBuilder::new();

    for (idx, &face_h) in faces.iter().enumerate() {
        if let Some(int_edges) = splits.get(&idx) {
            // This face needs splitting
            let sub_faces = split_face_along_curves(
                src,
                face_h,
                int_edges,
                &mut dst,
                op,
                &mut face_counter,
                tolerance,
                &mut builder,
            )?;
            all_faces.extend(sub_faces);
        } else if let Some(new_face) =
            copy_face_with_geometry(src, face_h, &mut dst, op, face_counter, &mut builder)?
        {
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
#[allow(clippy::too_many_arguments)]
fn split_face_along_curves(
    src: &BRepModel,
    face_h: Handle<FaceData>,
    int_edges: &[IntersectionEdge],
    dst: &mut BRepModel,
    op: OperationId,
    face_counter: &mut u32,
    tolerance: f64,
    builder: &mut SplitBuilder,
) -> KernelResult<Vec<Handle<FaceData>>> {
    let boundary = face_polygon_3d(src, face_h)?;
    if boundary.len() < 3 {
        let f = copy_face_with_geometry(src, face_h, dst, op, *face_counter, builder)?;
        if let Some(fh) = f {
            *face_counter += 1;
            return Ok(vec![fh]);
        }
        return Ok(Vec::new());
    }

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
        let f = copy_face_with_geometry(src, face_h, dst, op, *face_counter, builder)?;
        if let Some(fh) = f {
            *face_counter += 1;
            return Ok(vec![fh]);
        }
        return Ok(Vec::new());
    }

    // V36 R2c: merge chord-set into connected polylines before splitting.
    // Short SSI segments from adjacent tessellated surfaces often share endpoints
    // at interior points (e.g. 64 cylinder chord segments forming a closed
    // circle inside a box-top face). Stitching them into a single polyline
    // lets split_polygon_with_curves handle closed-loop and V-shaped curves
    // in one pass instead of failing each chord individually.
    let merged_curves =
        merge_chords_into_polylines_with_boundary(&all_clip_curves, Some(&boundary), tolerance);

    let raw_split_polygons = split_polygon_with_curves(&boundary, &merged_curves, tolerance);

    // V36 R2c: decompose any concave sub-polygons into convex pieces.
    // Downstream tessellation uses fan triangulation, which is only correct
    // for convex polygons; concave sub-polygons (e.g. an L-shape after
    // subtracting a corner box) otherwise inflate volume measurements.
    //
    // Skip decomposition for keyhole polygons (those containing coincident
    // non-adjacent vertices from a bridge-slit): decomposition at the
    // slit endpoint produces degenerate sub-polygons and breaks the
    // intentional closed-hole topology.
    let mut split_polygons: Vec<Vec<Point3>> = Vec::new();
    for poly in raw_split_polygons {
        if polygon_has_slit(&poly, tolerance) {
            split_polygons.push(poly);
            continue;
        }
        let decomposed = decompose_to_convex(&poly, tolerance);
        for sub in decomposed {
            if sub.len() >= 3 {
                split_polygons.push(sub);
            }
        }
    }

    if split_polygons.is_empty() {
        let f = copy_face_with_geometry(src, face_h, dst, op, *face_counter, builder)?;
        if let Some(fh) = f {
            *face_counter += 1;
            return Ok(vec![fh]);
        }
        return Ok(Vec::new());
    }

    let mut result_faces = Vec::new();
    let src_tag = src.faces.get(face_h).and_then(|f| f.tag.clone());
    let surface = get_face_surface(src, face_h);

    for poly in &split_polygons {
        if poly.len() < 3 {
            continue;
        }
        if let Some(new_face) =
            create_face_from_polygon(dst, poly, op, *face_counter, &src_tag, &surface, builder)?
        {
            result_faces.push(new_face);
            *face_counter += 1;
        }
    }

    if result_faces.is_empty() {
        let f = copy_face_with_geometry(src, face_h, dst, op, *face_counter, builder)?;
        if let Some(fh) = f {
            *face_counter += 1;
            return Ok(vec![fh]);
        }
        return Ok(Vec::new());
    }

    Ok(result_faces)
}

/// Splits a polygon boundary using intersection curves that cross it.
///
/// For each curve that enters at edge i and exits at edge j:
/// - Creates two sub-polygons by inserting the curve points at the boundary.
///
/// V36 R2c: also handles closed interior loops (both endpoints coincide and
/// interior to the polygon) via a "keyhole" insertion and handles curves with
/// one interior endpoint by extending along the tangent to the boundary.
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

        // Detect closed interior loop: first and last point coincide.
        let closed = entry.distance_to(exit) < tolerance * 10.0;

        let mut next_polygons = Vec::new();

        for poly in &current_polygons {
            let entry_on_boundary = find_edge_for_point(poly, &entry, tolerance).is_some();
            let exit_on_boundary = find_edge_for_point(poly, &exit, tolerance).is_some();

            let split = if closed && !entry_on_boundary {
                split_polygon_with_closed_loop(poly, curve, tolerance)
            } else if entry_on_boundary && exit_on_boundary {
                split_single_polygon(poly, &entry, &exit, curve, tolerance)
            } else if entry_on_boundary ^ exit_on_boundary {
                // One interior endpoint: extend along curve tangent to boundary.
                split_polygon_with_open_curve(poly, curve, tolerance)
            } else {
                vec![poly.to_vec()]
            };
            next_polygons.extend(split);
        }

        current_polygons = next_polygons;
    }

    current_polygons
}

/// Splits a polygon by an interior closed loop using a "keyhole" technique.
///
/// Finds the point on the loop closest to the polygon boundary, inserts a
/// bridge (two coincident edges in opposite directions), and walks the
/// polygon up to that bridge, into the loop, back to the bridge start, then
/// out along the reverse bridge. This yields a single simple polygon with a
/// degenerate slit — sufficient for the tessellation and volume measurement
/// path.
fn split_polygon_with_closed_loop(
    poly: &[Point3],
    loop_curve: &[Point3],
    tolerance: f64,
) -> Vec<Vec<Point3>> {
    if loop_curve.len() < 3 || poly.len() < 3 {
        return vec![poly.to_vec()];
    }

    // Drop the duplicate closing vertex if present.
    let loop_pts: Vec<Point3> = if loop_curve
        .first()
        .zip(loop_curve.last())
        .is_some_and(|(a, b)| a.distance_to(*b) < tolerance * 10.0)
    {
        loop_curve[..loop_curve.len() - 1].to_vec()
    } else {
        loop_curve.to_vec()
    };

    if loop_pts.len() < 3 {
        return vec![poly.to_vec()];
    }

    // Find the loop point closest to any polygon edge, along with the bridge
    // landing point on that edge.
    let mut best_loop_i = 0usize;
    let mut best_bridge = poly[0];
    let mut best_dist = f64::MAX;
    for (li, lp) in loop_pts.iter().enumerate() {
        let n = poly.len();
        for i in 0..n {
            let a = poly[i];
            let b = poly[(i + 1) % n];
            let ab = b - a;
            let ap = *lp - a;
            let len_sq = ab.dot(ab);
            if len_sq < 1e-20 {
                continue;
            }
            let t = (ap.dot(ab) / len_sq).clamp(0.0, 1.0);
            let closest = Point3::new(a.x + t * ab.x, a.y + t * ab.y, a.z + t * ab.z);
            let dist = lp.distance_to(closest);
            if dist < best_dist {
                best_dist = dist;
                best_loop_i = li;
                best_bridge = closest;
            }
        }
    }

    // Insert the bridge into the polygon at the closest edge (split the edge).
    let mut bridged_poly = Vec::with_capacity(poly.len() + 1);
    let n = poly.len();
    let mut bridge_edge = 0usize;
    let mut bridge_best = f64::MAX;
    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + 1) % n];
        let ab = b - a;
        let ap = best_bridge - a;
        let len_sq = ab.dot(ab);
        if len_sq < 1e-20 {
            continue;
        }
        let t = (ap.dot(ab) / len_sq).clamp(0.0, 1.0);
        let closest = Point3::new(a.x + t * ab.x, a.y + t * ab.y, a.z + t * ab.z);
        let dist = best_bridge.distance_to(closest);
        if dist < bridge_best {
            bridge_best = dist;
            bridge_edge = i;
        }
    }

    // Decide the loop orientation: reverse so that the keyhole traversal
    // encloses the outer region and excludes the loop interior.
    // We want to traverse the loop in the OPPOSITE orientation of the outer
    // polygon, so normals align (hole sense).
    let poly_normal = polygon_normal(poly);
    let loop_normal = polygon_normal(&loop_pts);
    let oriented_loop: Vec<Point3> = if poly_normal.dot(loop_normal) > 0.0 {
        loop_pts.iter().rev().copied().collect()
    } else {
        loop_pts.clone()
    };
    // Recompute best_loop_i if we reversed the loop.
    let loop_len = oriented_loop.len();
    let effective_start = if poly_normal.dot(loop_normal) > 0.0 {
        (loop_len - 1 - best_loop_i) % loop_len
    } else {
        best_loop_i
    };

    // Build the keyhole polygon: walk poly[0..=bridge_edge], then bridge point,
    // then loop starting at `effective_start`, then bridge point, then poly[bridge_edge+1..].
    for (i, &p) in poly.iter().enumerate() {
        bridged_poly.push(p);
        if i == bridge_edge {
            bridged_poly.push(best_bridge);
            // Walk the loop starting at effective_start.
            for k in 0..loop_len {
                bridged_poly.push(oriented_loop[(effective_start + k) % loop_len]);
            }
            // Close the loop back to its starting point.
            bridged_poly.push(oriented_loop[effective_start]);
            // Bridge back out.
            bridged_poly.push(best_bridge);
        }
    }

    if bridged_poly.len() < 3 {
        return vec![poly.to_vec()];
    }
    vec![bridged_poly]
}

/// Splits a polygon using a curve with one interior endpoint by extending the
/// tangent at that endpoint to the nearest polygon edge. Falls back to
/// unchanged polygon if extension fails.
fn split_polygon_with_open_curve(
    poly: &[Point3],
    curve: &[Point3],
    tolerance: f64,
) -> Vec<Vec<Point3>> {
    if curve.len() < 2 || poly.len() < 3 {
        return vec![poly.to_vec()];
    }

    let first = curve[0];
    let last = curve[curve.len() - 1];
    let first_on = find_edge_for_point(poly, &first, tolerance).is_some();
    let last_on = find_edge_for_point(poly, &last, tolerance).is_some();

    // Build an extended curve with both endpoints on the boundary.
    let extended: Vec<Point3> = if !first_on && last_on {
        // Extend from curve[0] opposite to the tangent (curve[1] - curve[0]).
        let tangent = (curve[0] - curve[1]).normalized().unwrap_or(Vec3::X);
        let hit = ray_polygon_boundary(poly, curve[0], tangent, tolerance);
        if let Some(hit_pt) = hit {
            let mut new_curve = vec![hit_pt];
            new_curve.extend_from_slice(curve);
            new_curve
        } else {
            return vec![poly.to_vec()];
        }
    } else if first_on && !last_on {
        let n = curve.len();
        let tangent = (curve[n - 1] - curve[n - 2])
            .normalized()
            .unwrap_or(Vec3::X);
        let hit = ray_polygon_boundary(poly, curve[n - 1], tangent, tolerance);
        if let Some(hit_pt) = hit {
            let mut new_curve = curve.to_vec();
            new_curve.push(hit_pt);
            new_curve
        } else {
            return vec![poly.to_vec()];
        }
    } else {
        return vec![poly.to_vec()];
    };

    let entry = extended[0];
    let exit = extended[extended.len() - 1];
    split_single_polygon(poly, &entry, &exit, &extended, tolerance)
}

/// Casts a ray from `origin` in `dir` and returns the first polygon-edge hit.
fn ray_polygon_boundary(
    poly: &[Point3],
    origin: Point3,
    dir: Vec3,
    tolerance: f64,
) -> Option<Point3> {
    let normal = polygon_normal(poly);
    let (drop_axis, _) = largest_axis(&normal);
    let proj = |p: Point3| -> (f64, f64) { project_drop(&p, drop_axis) };
    let proj_dir = match drop_axis {
        0 => (dir.y, dir.z),
        1 => (dir.x, dir.z),
        _ => (dir.x, dir.y),
    };
    let len_sq = proj_dir.0 * proj_dir.0 + proj_dir.1 * proj_dir.1;
    if len_sq < 1e-20 {
        return None;
    }

    let (ox, oy) = proj(origin);
    let (dx, dy) = proj_dir;

    let n = poly.len();
    let mut best_t = f64::MAX;
    let mut best_hit = None;
    for i in 0..n {
        let (ax, ay) = proj(poly[i]);
        let (bx, by) = proj(poly[(i + 1) % n]);
        // Solve origin + t*dir = a + s*(b - a), 0 <= s <= 1, t > 0
        let ex = bx - ax;
        let ey = by - ay;
        let det = dx * (-ey) - dy * (-ex);
        if det.abs() < 1e-14 {
            continue;
        }
        let dax = ax - ox;
        let day = ay - oy;
        let t = (dax * (-ey) - day * (-ex)) / det;
        let s = (dx * day - dy * dax) / det;
        if t <= tolerance || !(-1e-9..=1.0 + 1e-9).contains(&s) {
            continue;
        }
        if t < best_t {
            best_t = t;
            let s_c = s.clamp(0.0, 1.0);
            let edge_pt = Point3::new(
                poly[i].x + s_c * (poly[(i + 1) % n].x - poly[i].x),
                poly[i].y + s_c * (poly[(i + 1) % n].y - poly[i].y),
                poly[i].z + s_c * (poly[(i + 1) % n].z - poly[i].z),
            );
            best_hit = Some(edge_pt);
        }
    }
    best_hit
}

/// Merges a set of chord segments into connected polylines by joining segments
/// that share endpoints (within `tolerance`). Returns polylines (possibly
/// closed) representing the connected components of the chord graph.
///
/// V36 R2c: when `polygon` is provided, the walker prefers boundary-touching
/// vertices as polyline start/end points. This turns a chord-graph like
/// `(5,5)→(10,5)→(10,10)→(5,10)→(5,5)` (a closed square that touches the outer
/// polygon at 3 vertices) into a single open polyline whose endpoints are on
/// the polygon boundary and whose middle vertex is the interior corner —
/// suitable for the standard 2-way `split_single_polygon`.
fn merge_chords_into_polylines_with_boundary(
    chords: &[Vec<Point3>],
    polygon: Option<&[Point3]>,
    tolerance: f64,
) -> Vec<Vec<Point3>> {
    let tol = tolerance.max(1e-9) * 10.0;
    if chords.is_empty() {
        return Vec::new();
    }

    // Collect unique endpoint positions; map each chord endpoint to an index.
    let mut points: Vec<Point3> = Vec::new();
    let find_or_insert = |points: &mut Vec<Point3>, p: Point3| -> usize {
        for (i, q) in points.iter().enumerate() {
            if q.distance_to(p) < tol {
                return i;
            }
        }
        points.push(p);
        points.len() - 1
    };

    // For each chord, record (start_idx, end_idx, chord_index).
    // The chord itself may have intermediate points we want to preserve.
    struct ChordRec {
        start: usize,
        end: usize,
        pts: Vec<Point3>,
    }
    let mut recs: Vec<ChordRec> = Vec::with_capacity(chords.len());
    // V36 R2c: dedupe chord records by unordered (start, end) endpoint pair.
    // Multiple overlapping face pairs (e.g. a box-top coplanar with a
    // cylinder cap plus 32 cylinder walls each producing a 2-point segment
    // on the same circle) emit duplicate chords with the same endpoints;
    // the polyline stitcher otherwise walks the graph inconsistently.
    let mut seen_pairs: std::collections::HashSet<(usize, usize)> =
        std::collections::HashSet::new();
    for chord in chords {
        if chord.len() < 2 {
            continue;
        }
        let s = find_or_insert(&mut points, *chord.first().unwrap());
        let e = find_or_insert(&mut points, *chord.last().unwrap());
        if s == e {
            continue; // zero-length chord
        }
        let key = if s < e { (s, e) } else { (e, s) };
        if !seen_pairs.insert(key) {
            continue;
        }
        recs.push(ChordRec {
            start: s,
            end: e,
            pts: chord.clone(),
        });
    }

    if recs.is_empty() {
        return Vec::new();
    }

    // For each point, determine whether it lies on the supplied polygon
    // boundary. These points act as "break points" in the walk — the walker
    // stops polylines at boundary vertices so downstream code sees curves with
    // on-boundary endpoints, which the existing 2-way splitter can handle.
    let on_boundary: Vec<bool> = if let Some(poly) = polygon {
        points
            .iter()
            .map(|p| find_edge_for_point(poly, p, tolerance).is_some())
            .collect()
    } else {
        vec![false; points.len()]
    };

    // Build adjacency: for each point, list of (chord_idx, is_end_at_start).
    let mut adjacency: Vec<Vec<(usize, bool)>> = vec![Vec::new(); points.len()];
    for (ci, r) in recs.iter().enumerate() {
        adjacency[r.start].push((ci, true));
        adjacency[r.end].push((ci, false));
    }

    let mut used = vec![false; recs.len()];
    let mut polylines: Vec<Vec<Point3>> = Vec::new();

    // Walks a chain of chords starting at `start_pt_idx` and `start_chord`.
    // Stops when:
    //   1. the walker reaches a boundary vertex (if polygon given), or
    //   2. there is no unvisited neighbor chord, or
    //   3. the chain closes back to the origin (loop).
    let walk_chain = |start_pt_idx: usize,
                      start_chord: usize,
                      at_start: bool,
                      used: &mut [bool],
                      stop_at_boundary: bool|
     -> Vec<Point3> {
        let mut polyline: Vec<Point3> = Vec::new();
        let mut cur_chord = start_chord;
        let mut cur_dir_start = at_start;
        let start_tail = start_pt_idx;
        loop {
            used[cur_chord] = true;
            let rec = &recs[cur_chord];
            let pts_to_append: Vec<Point3> = if cur_dir_start {
                rec.pts.clone()
            } else {
                rec.pts.iter().rev().copied().collect()
            };
            if polyline.is_empty() {
                polyline.extend(pts_to_append);
            } else {
                polyline.extend(pts_to_append.into_iter().skip(1));
            }

            let tail_pt_idx = if cur_dir_start { rec.end } else { rec.start };
            if stop_at_boundary && on_boundary[tail_pt_idx] {
                break;
            }
            if tail_pt_idx == start_tail {
                // Closed back to origin.
                break;
            }
            let next = adjacency[tail_pt_idx]
                .iter()
                .find(|&&(nci, _)| nci != cur_chord && !used[nci])
                .copied();
            if let Some((next_chord, at_start_next)) = next {
                cur_chord = next_chord;
                cur_dir_start = at_start_next;
            } else {
                break;
            }
        }
        polyline
    };

    // Pass 1: emit polylines that start at a boundary vertex. Each iteration
    // picks one unused chord incident to that boundary vertex and walks until
    // we hit another boundary vertex. This decomposes a mixed graph into
    // boundary-to-boundary open curves plus any purely-interior residue.
    if polygon.is_some() {
        for start_pt in 0..points.len() {
            if !on_boundary[start_pt] {
                continue;
            }
            loop {
                let fresh = adjacency[start_pt]
                    .iter()
                    .find(|&&(ci, _)| !used[ci])
                    .copied();
                let (sc, ats) = match fresh {
                    Some(v) => v,
                    None => break,
                };
                let polyline = walk_chain(start_pt, sc, ats, &mut used, true);
                if polyline.len() >= 2 {
                    polylines.push(polyline);
                }
            }
        }
    }

    // Pass 2: residue walker. Picks any unused chord and walks until closure
    // or dead end. Catches purely-interior closed loops and isolated chains.
    for ci in 0..recs.len() {
        if used[ci] {
            continue;
        }
        let polyline = walk_chain(recs[ci].start, ci, true, &mut used, false);
        if polyline.len() >= 2 {
            polylines.push(polyline);
        }
    }

    polylines
}

/// Back-compat wrapper without polygon context (used in tests).
#[cfg(test)]
fn merge_chords_into_polylines(chords: &[Vec<Point3>], tolerance: f64) -> Vec<Vec<Point3>> {
    merge_chords_into_polylines_with_boundary(chords, None, tolerance)
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
///
/// V36 R2c: treat points that lie ON the polygon boundary (within tolerance)
/// as inside. The previous implementation relied on `point_in_polygon_2d_raw`
/// which is numerically unreliable on-boundary, causing boundary-touching
/// SSI endpoints to be dropped.
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

    let boundary_tol = tolerance * 10.0;
    let mut segments = Vec::new();
    let mut current_segment = Vec::new();

    for (i, &cp) in curve_points.iter().enumerate() {
        let on_boundary = point_on_polygon_boundary_2d(&curve2d[i], &poly2d, boundary_tol);
        let inside = on_boundary || point_in_polygon_2d_raw(&curve2d[i], &poly2d);

        if inside {
            current_segment.push(cp);
        } else if !current_segment.is_empty() {
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
            if let Some(snapped) = snap_to_polygon_edge(first, polygon, boundary_tol) {
                *first = snapped;
            }
        }
        if let Some(last) = seg.last_mut() {
            if let Some(snapped) = snap_to_polygon_edge(last, polygon, boundary_tol) {
                *last = snapped;
            }
        }
    }

    segments
}

/// Returns true if point lies within `tol` of any polygon edge (2D projected).
fn point_on_polygon_boundary_2d(pt: &(f64, f64), polygon: &[(f64, f64)], tol: f64) -> bool {
    let n = polygon.len();
    let (px, py) = *pt;
    for i in 0..n {
        let (ax, ay) = polygon[i];
        let (bx, by) = polygon[(i + 1) % n];
        let ex = bx - ax;
        let ey = by - ay;
        let len_sq = ex * ex + ey * ey;
        if len_sq < 1e-20 {
            continue;
        }
        let t = ((px - ax) * ex + (py - ay) * ey) / len_sq;
        let t_c = t.clamp(0.0, 1.0);
        let cx = ax + t_c * ex;
        let cy = ay + t_c * ey;
        let dx = px - cx;
        let dy = py - cy;
        if (dx * dx + dy * dy).sqrt() < tol {
            return true;
        }
    }
    false
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

/// Creates a face from a polygon of 3D points using shared vertices/edges.
fn create_face_from_polygon(
    model: &mut BRepModel,
    polygon: &[Point3],
    op: OperationId,
    index: u32,
    parent_tag: &Option<Tag>,
    surface: &Option<Arc<dyn Surface + Send + Sync>>,
    builder: &mut SplitBuilder,
) -> KernelResult<Option<Handle<FaceData>>> {
    let mut verts: Vec<Handle<VertexData>> = Vec::with_capacity(polygon.len());
    for &p in polygon {
        let v = builder.vertex(model, p);
        if verts.last().copied() != Some(v) {
            verts.push(v);
        }
    }
    if verts.len() >= 2 && verts.first() == verts.last() {
        verts.pop();
    }
    if verts.len() < 3 {
        return Ok(None);
    }

    let n = verts.len();
    let mut hes = Vec::with_capacity(n);
    for i in 0..n {
        let vs = verts[i];
        let ve = verts[(i + 1) % n];
        if vs == ve {
            continue;
        }
        hes.push(builder.directed_half_edge(model, vs, ve));
    }

    if hes.len() < 3 {
        return Ok(None);
    }

    let new_loop = model.make_loop(&hes)?;
    let tag = parent_tag
        .clone()
        .unwrap_or_else(|| Tag::generated(EntityKind::Face, op, index));

    let face = model.make_face_tagged(new_loop, tag);

    if let Some(surf) = surface {
        model.bind_face_surface(face, surf.clone(), cadkernel_topology::Orientation::Forward);
    }

    Ok(Some(face))
}

/// Copies a face preserving geometry binding, sharing vertices/edges via builder.
fn copy_face_with_geometry(
    src: &BRepModel,
    face_h: Handle<FaceData>,
    dst: &mut BRepModel,
    op: OperationId,
    index: u32,
    builder: &mut SplitBuilder,
) -> KernelResult<Option<Handle<FaceData>>> {
    let face_data = src
        .faces
        .get(face_h)
        .ok_or(KernelError::InvalidHandle("face"))?;
    let loop_data = src
        .loops
        .get(face_data.outer_loop)
        .ok_or(KernelError::InvalidHandle("loop"))?;
    let src_hes = src.loop_half_edges(loop_data.half_edge);

    let mut verts: Vec<Handle<VertexData>> = Vec::with_capacity(src_hes.len());
    let mut src_edge_per_vert: Vec<Option<Handle<cadkernel_topology::EdgeData>>> =
        Vec::with_capacity(src_hes.len());
    for &he_h in &src_hes {
        let he = src
            .half_edges
            .get(he_h)
            .ok_or(KernelError::InvalidHandle("half_edge"))?;
        let v_src = src
            .vertices
            .get(he.origin)
            .ok_or(KernelError::InvalidHandle("vertex"))?;
        let v = builder.vertex(dst, v_src.point);
        if verts.last().copied() != Some(v) {
            verts.push(v);
            src_edge_per_vert.push(he.edge);
        }
    }
    if verts.len() >= 2 && verts.first() == verts.last() {
        verts.pop();
        src_edge_per_vert.pop();
    }
    if verts.len() < 3 {
        return Ok(None);
    }

    let n = verts.len();
    let mut new_hes = Vec::with_capacity(n);
    for i in 0..n {
        let vs = verts[i];
        let ve = verts[(i + 1) % n];
        if vs == ve {
            continue;
        }
        let he = builder.directed_half_edge(dst, vs, ve);
        if let Some(src_edge_h) = src_edge_per_vert[i] {
            if let Some(src_edge) = src.edges.get(src_edge_h) {
                if let Some(ref curve) = src_edge.curve {
                    if let Some(he_data) = dst.half_edges.get(he) {
                        if let Some(dst_edge_h) = he_data.edge {
                            let domain = src_edge.curve_domain.unwrap_or((0.0, 1.0));
                            dst.bind_edge_curve(dst_edge_h, curve.clone(), domain);
                        }
                    }
                }
            }
        }
        new_hes.push(he);
    }

    if new_hes.len() < 3 {
        return Ok(None);
    }

    let new_loop = dst.make_loop(&new_hes)?;

    let tag = face_data
        .tag
        .clone()
        .unwrap_or_else(|| Tag::generated(EntityKind::Face, op, index));

    let new_face = dst.make_face_tagged(new_loop, tag);

    if let Some(ref surface) = face_data.surface {
        dst.bind_face_surface(new_face, surface.clone(), face_data.orientation);
    }

    if let Some(ref outer_trim) = face_data.outer_trim {
        dst.bind_face_trim(new_face, outer_trim.clone(), face_data.inner_trims.clone());
    }

    Ok(Some(new_face))
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
    let (result_a, new_solid_a) = build_split_model(
        model_a,
        solid_a,
        &faces_a,
        &empty_splits,
        SPLIT_TOL,
        "copy_a",
    )?;
    let (result_b, new_solid_b) = build_split_model(
        model_b,
        solid_b,
        &faces_b,
        &empty_splits,
        SPLIT_TOL,
        "copy_b",
    )?;

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
    // Newell's method: robust for non-planar/near-degenerate input.
    let mut nx = 0.0;
    let mut ny = 0.0;
    let mut nz = 0.0;
    let n = polygon.len();
    for i in 0..n {
        let a = polygon[i];
        let b = polygon[(i + 1) % n];
        nx += (a.y - b.y) * (a.z + b.z);
        ny += (a.z - b.z) * (a.x + b.x);
        nz += (a.x - b.x) * (a.y + b.y);
    }
    let v = Vec3::new(nx, ny, nz);
    v.normalized().unwrap_or_else(|| {
        // Fall back to triangle 0-1-2 if Newell degenerates.
        let e1 = polygon[1] - polygon[0];
        let e2 = polygon[2] - polygon[0];
        e1.cross(e2).normalized().unwrap_or(Vec3::Z)
    })
}

/// Detects keyhole / bridge-slit polygons — those containing two
/// non-adjacent vertices at the same spatial position. Such polygons
/// encode a hole via a degenerate slit and must not be split further.
fn polygon_has_slit(polygon: &[Point3], tolerance: f64) -> bool {
    let n = polygon.len();
    let tol = tolerance * 10.0;
    for i in 0..n {
        for j in (i + 2)..n {
            // Skip the wrap-around adjacency (last vertex vs first).
            if i == 0 && j == n - 1 {
                continue;
            }
            if polygon[i].distance_to(polygon[j]) < tol {
                return true;
            }
        }
    }
    false
}

/// Decomposes a (possibly concave) simple polygon into convex sub-polygons.
///
/// Algorithm: locate the first reflex vertex; cut along a diagonal to another
/// polygon vertex that is fully inside the polygon; recurse on both halves.
/// Returns the input polygon unchanged if it is already convex or the cut
/// cannot be made (degenerate / self-intersecting input).
///
/// This is required because downstream tessellation uses fan triangulation,
/// which is only correct for convex polygons. Concave pieces produced by
/// boolean splitting (e.g. an L-shape after corner subtraction) would
/// otherwise inflate volume measurements.
fn decompose_to_convex(polygon: &[Point3], tolerance: f64) -> Vec<Vec<Point3>> {
    if polygon.len() < 4 {
        return vec![polygon.to_vec()];
    }

    let normal = polygon_normal(polygon);
    let (drop_axis, _) = largest_axis(&normal);
    let signed_area_sign = polygon_signed_area_2d_sign(polygon, drop_axis);
    if signed_area_sign == 0.0 {
        return vec![polygon.to_vec()];
    }

    decompose_to_convex_recursive(polygon, drop_axis, signed_area_sign, tolerance, 0)
}

fn decompose_to_convex_recursive(
    polygon: &[Point3],
    drop_axis: usize,
    ccw_sign: f64,
    tolerance: f64,
    depth: usize,
) -> Vec<Vec<Point3>> {
    const MAX_DEPTH: usize = 32;
    if depth >= MAX_DEPTH || polygon.len() < 4 {
        return vec![polygon.to_vec()];
    }

    let n = polygon.len();
    let poly2d: Vec<(f64, f64)> = polygon.iter().map(|p| project_drop(p, drop_axis)).collect();

    // Find first reflex vertex. A vertex v_i is reflex when the signed
    // area of (v_{i-1}, v_i, v_{i+1}) has sign opposite to the polygon's
    // overall orientation.
    let reflex_idx = (0..n).find(|&i| {
        let prev = poly2d[(i + n - 1) % n];
        let cur = poly2d[i];
        let next = poly2d[(i + 1) % n];
        let cross = (cur.0 - prev.0) * (next.1 - cur.1) - (cur.1 - prev.1) * (next.0 - cur.0);
        cross * ccw_sign < -tolerance
    });

    let Some(r) = reflex_idx else {
        return vec![polygon.to_vec()];
    };

    // Find a suitable diagonal partner. Prefer the farthest vertex whose
    // diagonal stays strictly inside the polygon and does not cross any
    // polygon edge. Skip adjacent vertices (r-1, r, r+1).
    let r_prev = (r + n - 1) % n;
    let r_next = (r + 1) % n;
    let mut best: Option<usize> = None;
    let mut best_score = -1.0f64;
    for j in 0..n {
        if j == r || j == r_prev || j == r_next {
            continue;
        }
        if !diagonal_inside_polygon_2d(&poly2d, r, j, tolerance) {
            continue;
        }
        // Prefer diagonals whose midpoint is well inside and which make
        // both sub-polygons balanced in vertex count.
        let da = (j + n - r) % n;
        let db = (r + n - j) % n;
        let balance = (da.min(db)) as f64;
        // Also prefer diagonals that actually resolve the reflex angle
        // on the r-side (the new interior angle at r becomes convex).
        let prev_v = poly2d[r_prev];
        let cur_v = poly2d[r];
        let j_v = poly2d[j];
        let cross =
            (cur_v.0 - prev_v.0) * (j_v.1 - cur_v.1) - (cur_v.1 - prev_v.1) * (j_v.0 - cur_v.0);
        if cross * ccw_sign < -tolerance {
            // Diagonal keeps r reflex on this side — skip.
            continue;
        }
        let score = balance;
        if score > best_score {
            best_score = score;
            best = Some(j);
        }
    }

    let Some(j) = best else {
        return vec![polygon.to_vec()];
    };

    // Build two sub-polygons along the diagonal r → j.
    let mut poly_a: Vec<Point3> = Vec::new();
    let mut k = r;
    loop {
        poly_a.push(polygon[k]);
        if k == j {
            break;
        }
        k = (k + 1) % n;
    }

    let mut poly_b: Vec<Point3> = Vec::new();
    let mut k = j;
    loop {
        poly_b.push(polygon[k]);
        if k == r {
            break;
        }
        k = (k + 1) % n;
    }

    if poly_a.len() < 3 || poly_b.len() < 3 {
        return vec![polygon.to_vec()];
    }

    let mut result =
        decompose_to_convex_recursive(&poly_a, drop_axis, ccw_sign, tolerance, depth + 1);
    result.extend(decompose_to_convex_recursive(
        &poly_b,
        drop_axis,
        ccw_sign,
        tolerance,
        depth + 1,
    ));
    result
}

fn polygon_signed_area_2d_sign(polygon: &[Point3], drop_axis: usize) -> f64 {
    let mut area2 = 0.0f64;
    let n = polygon.len();
    for i in 0..n {
        let (ax, ay) = project_drop(&polygon[i], drop_axis);
        let (bx, by) = project_drop(&polygon[(i + 1) % n], drop_axis);
        area2 += ax * by - bx * ay;
    }
    if area2 > 1e-12 {
        1.0
    } else if area2 < -1e-12 {
        -1.0
    } else {
        0.0
    }
}

/// Returns true when the segment (poly[i], poly[j]) is a valid interior
/// diagonal of the simple polygon: midpoint is strictly inside, and the
/// segment does not cross any non-adjacent polygon edge.
fn diagonal_inside_polygon_2d(poly2d: &[(f64, f64)], i: usize, j: usize, tolerance: f64) -> bool {
    let n = poly2d.len();
    let a = poly2d[i];
    let b = poly2d[j];
    let mid = ((a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5);
    if !point_in_polygon_2d_raw(&mid, poly2d) {
        return false;
    }
    for k in 0..n {
        let k_next = (k + 1) % n;
        if k == i || k == j || k_next == i || k_next == j {
            continue;
        }
        if segments_intersect_2d(a, b, poly2d[k], poly2d[k_next], tolerance) {
            return false;
        }
    }
    true
}

fn segments_intersect_2d(
    p1: (f64, f64),
    p2: (f64, f64),
    p3: (f64, f64),
    p4: (f64, f64),
    tolerance: f64,
) -> bool {
    let d1 = cross2(p4.0 - p3.0, p4.1 - p3.1, p1.0 - p3.0, p1.1 - p3.1);
    let d2 = cross2(p4.0 - p3.0, p4.1 - p3.1, p2.0 - p3.0, p2.1 - p3.1);
    let d3 = cross2(p2.0 - p1.0, p2.1 - p1.1, p3.0 - p1.0, p3.1 - p1.1);
    let d4 = cross2(p2.0 - p1.0, p2.1 - p1.1, p4.0 - p1.0, p4.1 - p1.1);
    if ((d1 > tolerance && d2 < -tolerance) || (d1 < -tolerance && d2 > tolerance))
        && ((d3 > tolerance && d4 < -tolerance) || (d3 < -tolerance && d4 > tolerance))
    {
        return true;
    }
    false
}

fn cross2(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    ax * by - ay * bx
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
    use crate::primitives::make_box;
    use cadkernel_geometry::Curve;

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

        let result = split_solids_at_intersection(&a, ra.solid, &b, rb.solid, 0.001).unwrap();
        assert!(!result.had_splits);
        // Both models should have 6 faces
        assert_eq!(
            collect_solid_faces(&result.model_a, result.solid_a)
                .unwrap()
                .len(),
            6
        );
        assert_eq!(
            collect_solid_faces(&result.model_b, result.solid_b)
                .unwrap()
                .len(),
            6
        );
    }

    #[test]
    fn test_split_overlapping_boxes() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(1.0, 1.0, 1.0), 2.0, 2.0, 2.0).unwrap();

        let result = split_solids_at_intersection(&a, ra.solid, &b, rb.solid, 0.001).unwrap();
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
        assert!(
            !clipped.is_empty(),
            "should have at least one segment inside"
        );
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

    // V36 R2c tests for chord-merge + keyhole handling.
    #[test]
    fn test_merge_chords_closed_loop() {
        // Four chords forming a square loop interior to a larger polygon.
        let chords = vec![
            vec![Point3::new(1.0, 1.0, 0.0), Point3::new(2.0, 1.0, 0.0)],
            vec![Point3::new(2.0, 1.0, 0.0), Point3::new(2.0, 2.0, 0.0)],
            vec![Point3::new(2.0, 2.0, 0.0), Point3::new(1.0, 2.0, 0.0)],
            vec![Point3::new(1.0, 2.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
        ];
        let merged = merge_chords_into_polylines(&chords, 0.01);
        assert_eq!(
            merged.len(),
            1,
            "four connected chords merge into one polyline"
        );
        let poly = &merged[0];
        // Closed loop should have first ~= last.
        assert!(
            poly.first().unwrap().distance_to(*poly.last().unwrap()) < 0.05,
            "merged loop is closed"
        );
        // 4 edges → 5 points (last == first).
        assert!(poly.len() >= 4);
    }

    #[test]
    fn test_merge_chords_v_shape() {
        // Two chords meeting at an interior point — one endpoint is on-boundary.
        let chords = vec![
            vec![Point3::new(5.0, 10.0, 10.0), Point3::new(5.0, 5.0, 10.0)],
            vec![Point3::new(5.0, 5.0, 10.0), Point3::new(10.0, 5.0, 10.0)],
        ];
        let merged = merge_chords_into_polylines(&chords, 0.01);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].len(), 3);
        // First and last should be the boundary-touching endpoints.
        let first = merged[0][0];
        let last = merged[0][2];
        let on_5_10_10 = first.distance_to(Point3::new(5.0, 10.0, 10.0)) < 1e-6
            || last.distance_to(Point3::new(5.0, 10.0, 10.0)) < 1e-6;
        let on_10_5_10 = first.distance_to(Point3::new(10.0, 5.0, 10.0)) < 1e-6
            || last.distance_to(Point3::new(10.0, 5.0, 10.0)) < 1e-6;
        assert!(on_5_10_10 && on_10_5_10);
    }

    #[test]
    fn test_l_shape_split_has_split_faces() {
        // Reproduces the L-subtraction case. A.top (the 10x10 square at z=10)
        // should be split once two curves meeting at interior point (5,5,10)
        // are merged into a V-shaped polyline.
        let mut big = BRepModel::new();
        let rb_big = make_box(&mut big, Point3::ORIGIN, 10.0, 10.0, 10.0).unwrap();
        let mut corner = BRepModel::new();
        let rb_corner = make_box(&mut corner, Point3::new(5.0, 5.0, 0.0), 5.0, 5.0, 10.0).unwrap();
        // Direct test of compute_planar_intersection for A.top vs B.left
        let faces_a = collect_solid_faces(&big, rb_big.solid).unwrap();
        let faces_b = collect_solid_faces(&corner, rb_corner.solid).unwrap();
        for (i, &fa) in faces_a.iter().enumerate() {
            for (j, &fb) in faces_b.iter().enumerate() {
                let pi = compute_planar_intersection(&big, fa, &corner, fb, 1e-6).unwrap();
                if !pi.is_empty() {
                    for curve in &pi {
                        eprintln!(
                            "A face {} vs B face {}: {} points [{:?}..{:?}]",
                            i,
                            j,
                            curve.points.len(),
                            curve.points.first(),
                            curve.points.last()
                        );
                    }
                }
            }
        }
        let split =
            split_solids_at_intersection(&big, rb_big.solid, &corner, rb_corner.solid, 1e-6)
                .unwrap();
        let faces_a = collect_solid_faces(&split.model_a, split.solid_a).unwrap();
        eprintln!(
            "had_splits = {}, split A faces = {}",
            split.had_splits,
            faces_a.len()
        );
        assert!(
            faces_a.len() > 6,
            "L-shape big-box faces should be split: got {}",
            faces_a.len()
        );
    }

    #[test]
    fn test_closed_loop_keyhole_insertion() {
        let poly = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(10.0, 0.0, 0.0),
            Point3::new(10.0, 10.0, 0.0),
            Point3::new(0.0, 10.0, 0.0),
        ];
        // An interior square loop (closed polyline).
        let loop_curve = vec![
            Point3::new(4.0, 4.0, 0.0),
            Point3::new(6.0, 4.0, 0.0),
            Point3::new(6.0, 6.0, 0.0),
            Point3::new(4.0, 6.0, 0.0),
            Point3::new(4.0, 4.0, 0.0),
        ];
        let result = split_polygon_with_curves(&poly, &[loop_curve], 0.01);
        assert_eq!(result.len(), 1, "keyhole should yield one bridged polygon");
        // Bridged polygon has original 4 + bridge-point × 2 + loop 4 + repeat start = 11ish
        assert!(
            result[0].len() > 8,
            "bridged polygon must contain the loop: got {}",
            result[0].len()
        );
    }
}
