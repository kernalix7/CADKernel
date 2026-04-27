use std::collections::HashMap;

use rayon::prelude::*;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::Point3;
use cadkernel_topology::{
    BRepModel, EntityKind, FaceData, HalfEdgeData, Handle, OperationId, SolidData, Tag, VertexData,
};

use super::broad_phase::collect_solid_faces;
use super::classify::FacePosition;
use super::csg::BooleanOp;

const VERT_MERGE_TOL: f64 = 1e-6;

struct SharedBuilder {
    vertex_positions: Vec<Point3>,
    vertex_handles: Vec<Handle<VertexData>>,
    edge_map: HashMap<(u32, u32), Handle<HalfEdgeData>>,
}

impl SharedBuilder {
    fn new() -> Self {
        Self {
            vertex_positions: Vec::new(),
            vertex_handles: Vec::new(),
            edge_map: HashMap::new(),
        }
    }

    fn vertex(&mut self, dst: &mut BRepModel, point: Point3) -> Handle<VertexData> {
        for (i, p) in self.vertex_positions.iter().enumerate() {
            if p.distance_to(point) < VERT_MERGE_TOL {
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
        // Only return a cached half-edge if it is not yet bound to a loop —
        // every half-edge in a manifold B-Rep belongs to exactly one loop,
        // so sharing a half-edge across two loops would overwrite the
        // previous loop's next/prev pointers and produce a corrupt topology.
        if let Some(&he) = self.edge_map.get(&key_fwd) {
            if dst
                .half_edges
                .get(he)
                .is_some_and(|h| h.loop_ref.is_none())
            {
                return he;
            }
            // Already bound — fall through and allocate a fresh edge pair.
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
        // Overwrite any stale mapping; subsequent faces look up the fresh pair.
        self.edge_map.insert(key_fwd, he_a);
        self.edge_map.insert(key_rev, he_b);
        he_a
    }
}

/// Copies a face from `src` into `dst`, sharing vertices and edges with
/// any previously-copied face via `builder`. When `flip` is true the
/// vertex order is reversed so the face normal inverts (used for
/// Difference on B-faces).
/// Inserts any builder-known vertex that lies strictly on the interior of
/// a segment into the vertex sequence, producing a polyline that matches
/// neighbouring faces' splits (T-junction repair).
fn insert_colinear_vertices(
    dst: &BRepModel,
    builder: &SharedBuilder,
    verts: &[Handle<VertexData>],
) -> Vec<Handle<VertexData>> {
    const T_TOL: f64 = 1e-6;
    let n = verts.len();
    let mut out: Vec<Handle<VertexData>> = Vec::with_capacity(n);
    for i in 0..n {
        let vs = verts[i];
        let ve = verts[(i + 1) % n];
        out.push(vs);
        let (ps, pe) = match (dst.vertices.get(vs), dst.vertices.get(ve)) {
            (Some(a), Some(b)) => (a.point, b.point),
            _ => continue,
        };
        let seg = pe - ps;
        let seg_len_sq = seg.dot(seg);
        if seg_len_sq < T_TOL * T_TOL {
            continue;
        }
        // Collect all builder vertices strictly between vs and ve.
        let mut inserts: Vec<(f64, Handle<VertexData>)> = Vec::new();
        for (j, &vh) in builder.vertex_handles.iter().enumerate() {
            if vh == vs || vh == ve {
                continue;
            }
            let vp = builder.vertex_positions[j];
            let vec = vp - ps;
            let t = vec.dot(seg) / seg_len_sq;
            if !(T_TOL..=(1.0 - T_TOL)).contains(&t) {
                continue;
            }
            // Perpendicular distance from the segment.
            let proj = Point3::new(
                ps.x + t * seg.x,
                ps.y + t * seg.y,
                ps.z + t * seg.z,
            );
            if proj.distance_to(vp) < T_TOL * 10.0 {
                inserts.push((t, vh));
            }
        }
        inserts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        for (_, vh) in inserts {
            if out.last().copied() != Some(vh) {
                out.push(vh);
            }
        }
    }
    out
}

fn copy_face_shared(
    src: &BRepModel,
    face_h: Handle<FaceData>,
    dst: &mut BRepModel,
    op: OperationId,
    face_index: u32,
    builder: &mut SharedBuilder,
    flip: bool,
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

    let mut positions: Vec<Point3> = Vec::with_capacity(src_hes.len());
    for &he_h in &src_hes {
        let he = src
            .half_edges
            .get(he_h)
            .ok_or(KernelError::InvalidHandle("half_edge"))?;
        let v = src
            .vertices
            .get(he.origin)
            .ok_or(KernelError::InvalidHandle("vertex"))?;
        positions.push(v.point);
    }

    if flip {
        positions.reverse();
    }

    let mut verts: Vec<Handle<VertexData>> = Vec::with_capacity(positions.len());
    for p in &positions {
        let v = builder.vertex(dst, *p);
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

    // V36 R2c: T-junction repair. A neighbouring face may have been split
    // during boolean evaluation so that one of its edges runs along the
    // interior of an edge of this face; insert any builder-known vertex
    // that lies strictly on segment (verts[i], verts[i+1]) so adjacent
    // faces share half-edges correctly and the shell stays watertight.
    let verts = insert_colinear_vertices(dst, builder, &verts);

    let n = verts.len();
    let mut hes: Vec<Handle<HalfEdgeData>> = Vec::with_capacity(n);
    for i in 0..n {
        let vs = verts[i];
        let ve = verts[(i + 1) % n];
        if vs == ve {
            continue;
        }
        hes.push(builder.directed_half_edge(dst, vs, ve));
    }
    if hes.len() < 3 {
        return Ok(None);
    }

    let new_loop = dst.make_loop(&hes)?;

    let tag = face_data
        .tag
        .clone()
        .unwrap_or_else(|| Tag::generated(EntityKind::Face, op, face_index));
    let new_face = dst.make_face_tagged(new_loop, tag);

    if let Some(ref surface) = face_data.surface {
        let orientation = if flip {
            match face_data.orientation {
                cadkernel_topology::Orientation::Forward => {
                    cadkernel_topology::Orientation::Reversed
                }
                cadkernel_topology::Orientation::Reversed => {
                    cadkernel_topology::Orientation::Forward
                }
            }
        } else {
            face_data.orientation
        };
        dst.bind_face_surface(new_face, surface.clone(), orientation);
    }

    Ok(Some(new_face))
}

/// Performs a boolean operation between two solids.
///
/// Automatically detects overlapping faces and applies face-splitting
/// when needed for correct results. Falls back to simple classification
/// when solids are disjoint or fully contained.
///
/// Returns `Err` for degenerate inputs (empty solids, invalid handles)
/// instead of panicking.
pub fn boolean_op(
    model_a: &BRepModel,
    solid_a: Handle<SolidData>,
    model_b: &BRepModel,
    solid_b: Handle<SolidData>,
    op: BooleanOp,
) -> KernelResult<BRepModel> {
    // Validate inputs — never panic on bad handles
    if model_a.solids.get(solid_a).is_none() {
        return Err(KernelError::InvalidHandle("solid_a"));
    }
    if model_b.solids.get(solid_b).is_none() {
        return Err(KernelError::InvalidHandle("solid_b"));
    }

    // Check for degenerate solids (no shells or no faces)
    let has_faces_a = model_a
        .solids
        .get(solid_a)
        .is_some_and(|s| s.shells.iter().any(|&sh| {
            model_a.shells.get(sh).is_some_and(|shell| !shell.faces.is_empty())
        }));
    let has_faces_b = model_b
        .solids
        .get(solid_b)
        .is_some_and(|s| s.shells.iter().any(|&sh| {
            model_b.shells.get(sh).is_some_and(|shell| !shell.faces.is_empty())
        }));

    if !has_faces_a && !has_faces_b {
        return Ok(BRepModel::new());
    }
    // For union with one empty operand, return the non-empty one
    if !has_faces_a {
        return match op {
            BooleanOp::Union => copy_solid_to_new_model(model_b, solid_b),
            BooleanOp::Intersection => Ok(BRepModel::new()),
            BooleanOp::Difference => Ok(BRepModel::new()),
        };
    }
    if !has_faces_b {
        return match op {
            BooleanOp::Union | BooleanOp::Difference => copy_solid_to_new_model(model_a, solid_a),
            BooleanOp::Intersection => Ok(BRepModel::new()),
        };
    }

    // Try split path first — if faces overlap, split them for precision
    let split_result = super::face_split::split_solids_at_intersection(
        model_a, solid_a, model_b, solid_b, 1e-6,
    );

    // Use split models if splitting succeeded and produced splits
    let (eff_a, eff_solid_a, eff_b, eff_solid_b) = match &split_result {
        Ok(sr) if sr.had_splits => (&sr.model_a, sr.solid_a, &sr.model_b, sr.solid_b),
        _ => (model_a, solid_a, model_b, solid_b),
    };

    let faces_a = collect_solid_faces(eff_a, eff_solid_a)?;
    let faces_b = collect_solid_faces(eff_b, eff_solid_b)?;

    let mut result = BRepModel::new();
    let result_op = result.history.next_operation(match op {
        BooleanOp::Union => "boolean_union",
        BooleanOp::Intersection => "boolean_intersection",
        BooleanOp::Difference => "boolean_difference",
    });

    // Classify faces from A in parallel (coplanar-aware).
    let classified_a: Vec<(Handle<FaceData>, FacePosition)> = faces_a
        .par_iter()
        .map(|&face_h| {
            super::classify::classify_face_with_coplanar(eff_a, face_h, eff_b, eff_solid_b)
                .map(|pos| (face_h, pos))
        })
        .collect::<KernelResult<Vec<_>>>()?;

    // Classify faces from B in parallel (coplanar-aware).
    let classified_b: Vec<(Handle<FaceData>, FacePosition)> = faces_b
        .par_iter()
        .map(|&face_h| {
            super::classify::classify_face_with_coplanar(eff_b, face_h, eff_a, eff_solid_a)
                .map(|pos| (face_h, pos))
        })
        .collect::<KernelResult<Vec<_>>>()?;

    let mut result_faces: Vec<Handle<FaceData>> = Vec::new();
    let mut face_counter = 0u32;
    let mut builder = SharedBuilder::new();

    // Face-kept rules (coplanar-aware).
    //
    // After coplanar-aware face splitting, each face is one of:
    //   - Outside: strictly outside the other solid.
    //   - Inside:  strictly inside the other solid.
    //   - OnBoundarySame: coincident with other's boundary, interiors on
    //     the SAME side (e.g. two identical solids).
    //   - OnBoundaryOpposite: coincident with other's boundary, interiors
    //     on OPPOSITE sides (e.g. two boxes mating along a shared face).
    //
    // Regular Boolean rules (A op B), derived from material-preservation:
    //   Union: result boundary = (A outside B) ∪ (B outside A) ∪
    //          (one copy of the co-facing overlap). Mating overlap is
    //          interior to the union and dropped.
    //   Intersection: boundary = (A inside B) ∪ (B inside A) ∪
    //          (one copy of the co-facing overlap).
    //   Difference (A − B): boundary = (A outside B) ∪ (A mating-overlap,
    //          where B doesn't carve through A) ∪ (B inside A, flipped).
    //          Co-facing overlap drops from A because B fully occupies
    //          A's interior side of that face.
    for (face_h, pos) in &classified_a {
        let keep = match op {
            BooleanOp::Union => {
                matches!(pos, FacePosition::Outside | FacePosition::OnBoundarySame)
            }
            BooleanOp::Intersection => {
                matches!(pos, FacePosition::Inside | FacePosition::OnBoundarySame)
            }
            BooleanOp::Difference => {
                matches!(pos, FacePosition::Outside | FacePosition::OnBoundaryOpposite)
            }
        };
        if keep {
            if let Some(new_face) =
                copy_face_shared(eff_a, *face_h, &mut result, result_op, face_counter, &mut builder, false)?
            {
                result_faces.push(new_face);
                face_counter += 1;
            }
        }
    }

    for (face_h, pos) in &classified_b {
        let keep = match op {
            // Union/Intersection: A's OnBoundarySame copy was already kept,
            // drop B's duplicate. OnBoundaryOpposite is interior to the
            // result for Union and is not part of the boundary for
            // Intersection (zero-volume overlap only touches along a face),
            // so drop as well.
            BooleanOp::Union => *pos == FacePosition::Outside,
            BooleanOp::Intersection => *pos == FacePosition::Inside,
            // Difference: keep B-face only if it is strictly inside A.
            // OnBoundarySame = A fully occupies B-interior-side already
            // dropped from A, so B duplicate not needed.
            // OnBoundaryOpposite = mating, no material carved, drop.
            BooleanOp::Difference => *pos == FacePosition::Inside,
        };
        if keep {
            // For Difference (A - B), B-faces classified Inside become
            // interior walls of A's new cavity. B's original outward normals
            // point AWAY from B's interior, i.e. INTO A's material after
            // subtraction. We flip them so the surviving shell has
            // consistent outward orientation (normals point away from the
            // remaining material, into the removed cavity). Union and
            // Intersection keep B's winding as-is because B's kept faces
            // in those ops already face the outside of the result solid.
            let flip = matches!(op, BooleanOp::Difference);
            if let Some(new_face) =
                copy_face_shared(eff_b, *face_h, &mut result, result_op, face_counter, &mut builder, flip)?
            {
                result_faces.push(new_face);
                face_counter += 1;
            }
        }
    }

    if !result_faces.is_empty() {
        let shell_tag = Tag::generated(EntityKind::Shell, result_op, 0);
        let shell = result.make_shell_tagged(&result_faces, shell_tag);
        let solid_tag = Tag::generated(EntityKind::Solid, result_op, 0);
        result.make_solid_tagged(&[shell], solid_tag);
    }

    Ok(result)
}

/// Copies a solid into a fresh BRepModel (used for degenerate-operand cases).
fn copy_solid_to_new_model(
    src: &BRepModel,
    solid: Handle<SolidData>,
) -> KernelResult<BRepModel> {
    let mut dst = BRepModel::new();
    let op = dst.history.next_operation("boolean_copy");

    let src_solid = src.solids.get(solid).ok_or(KernelError::InvalidHandle("solid"))?;
    let mut all_faces = Vec::new();
    let mut builder = SharedBuilder::new();
    let mut face_counter = 0u32;

    for &shell_h in &src_solid.shells {
        let shell_data = src.shells.get(shell_h).ok_or(KernelError::InvalidHandle("shell"))?;
        for &face_h in &shell_data.faces {
            if let Some(new_face) =
                copy_face_shared(src, face_h, &mut dst, op, face_counter, &mut builder, false)?
            {
                all_faces.push(new_face);
                face_counter += 1;
            }
        }
    }

    if !all_faces.is_empty() {
        let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
        let shell = dst.make_shell_tagged(&all_faces, shell_tag);
        let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
        dst.make_solid_tagged(&[shell], solid_tag);
    }

    Ok(dst)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::make_box;

    #[test]
    fn test_union_disjoint_boxes() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(5.0, 5.0, 5.0), 1.0, 1.0, 1.0).unwrap();

        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Union).unwrap();
        // Disjoint: all 12 faces should be outside each other → all kept
        assert_eq!(result.faces.len(), 12);
        assert_eq!(result.solids.len(), 1);
    }

    #[test]
    fn test_intersection_disjoint_boxes() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(5.0, 5.0, 5.0), 1.0, 1.0, 1.0).unwrap();

        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Intersection).unwrap();
        // Disjoint: no faces inside each other → empty result
        assert_eq!(result.faces.len(), 0);
    }

    #[test]
    fn test_difference_disjoint_boxes() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(5.0, 5.0, 5.0), 1.0, 1.0, 1.0).unwrap();

        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Difference).unwrap();
        // Disjoint: A is entirely outside B, B is entirely outside A
        // Keep A's outside faces (6) + B's inside faces (0) = 6
        assert_eq!(result.faces.len(), 6);
    }

    #[test]
    fn test_parallel_boolean_many_boxes() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(20.0, 20.0, 20.0), 3.0, 3.0, 3.0).unwrap();

        let union = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Union).unwrap();
        assert_eq!(union.faces.len(), 12);
        assert_eq!(union.solids.len(), 1);

        let inter = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Intersection).unwrap();
        assert_eq!(inter.faces.len(), 0);

        let diff = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Difference).unwrap();
        assert_eq!(diff.faces.len(), 6);
    }

    // -----------------------------------------------------------------------
    // Edge cases and robustness tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_boolean_invalid_handle_a() {
        let a = BRepModel::new();
        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        // Invalid solid handle for A
        let bad_handle = cadkernel_topology::Handle::from_raw_parts(999, 0);
        let result = boolean_op(&a, bad_handle, &b, rb.solid, BooleanOp::Union);
        assert!(result.is_err(), "should fail with invalid handle A");
    }

    #[test]
    fn test_boolean_invalid_handle_b() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let b = BRepModel::new();

        let bad_handle = cadkernel_topology::Handle::from_raw_parts(999, 0);
        let result = boolean_op(&a, ra.solid, &b, bad_handle, BooleanOp::Union);
        assert!(result.is_err(), "should fail with invalid handle B");
    }

    #[test]
    fn test_boolean_coplanar_faces() {
        // Two boxes sharing a face (touching at z=1)
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(0.0, 0.0, 1.0), 1.0, 1.0, 1.0).unwrap();

        // Union of touching boxes should not panic
        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Union);
        assert!(result.is_ok(), "coplanar face union should not panic");

        // Difference should not panic
        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Difference);
        assert!(result.is_ok(), "coplanar face difference should not panic");
    }

    #[test]
    fn test_boolean_identical_solids() {
        // Same box at same position — fully overlapping
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        // Union of identical solids should not panic
        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Union);
        assert!(result.is_ok(), "identical solid union should not panic");

        // Intersection should produce a result
        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Intersection);
        assert!(result.is_ok(), "identical solid intersection should not panic");
    }

    #[test]
    fn test_boolean_contained_solid() {
        // Small box fully inside larger box
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 10.0, 10.0, 10.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(3.0, 3.0, 3.0), 2.0, 2.0, 2.0).unwrap();

        // Difference: should keep A faces (outer), add B faces (inner)
        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Difference);
        assert!(result.is_ok(), "contained solid difference should not panic");

        // Intersection: should produce the inner box
        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Intersection);
        assert!(result.is_ok(), "contained solid intersection should not panic");
        let model = result.unwrap();
        // Inner box faces should be classified as Inside, so kept
        assert!(!model.faces.is_empty(), "intersection of containment should produce faces");
    }

    #[test]
    fn test_boolean_thin_wall() {
        // Very thin box (almost 2D) — tests near-degenerate geometry
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 10.0, 10.0, 0.001).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(5.0, 5.0, -0.5), 10.0, 10.0, 1.0).unwrap();

        // Should not panic even with very thin geometry
        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Union);
        assert!(result.is_ok(), "thin wall union should not panic");

        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Difference);
        assert!(result.is_ok(), "thin wall difference should not panic");
    }

    #[test]
    fn test_boolean_tangent_contact() {
        // Two boxes touching at a single edge
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(1.0, 1.0, 0.0), 1.0, 1.0, 1.0).unwrap();

        // Touching at edge — should not panic
        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Union);
        assert!(result.is_ok(), "edge-touching union should not panic");
    }

    #[test]
    fn test_boolean_vertex_contact() {
        // Two boxes touching at a single vertex
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(1.0, 1.0, 1.0), 1.0, 1.0, 1.0).unwrap();

        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Union);
        assert!(result.is_ok(), "vertex-touching union should not panic");
        let model = result.unwrap();
        assert_eq!(model.faces.len(), 12, "disjoint-touching union should keep all 12 faces");
    }

    #[test]
    fn test_boolean_very_small_solid() {
        // Extremely small solid — near machine epsilon
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 1e-10, 1e-10, 1e-10).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Union);
        assert!(result.is_ok(), "very small solid union should not panic");
    }

    #[test]
    fn test_boolean_empty_model_union() {
        // Model with solid but no faces
        let mut a = BRepModel::new();
        let shell = a.make_shell(&[]);
        let solid_a = a.make_solid(&[shell]);

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        // Union with empty solid should return the non-empty one
        let result = boolean_op(&a, solid_a, &b, rb.solid, BooleanOp::Union);
        assert!(result.is_ok());
        let model = result.unwrap();
        assert_eq!(model.faces.len(), 6, "union with empty should return other solid's faces");
    }

    #[test]
    fn test_boolean_empty_model_intersection() {
        let mut a = BRepModel::new();
        let shell = a.make_shell(&[]);
        let solid_a = a.make_solid(&[shell]);

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        // Intersection with empty solid should be empty
        let result = boolean_op(&a, solid_a, &b, rb.solid, BooleanOp::Intersection);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().faces.len(), 0);
    }

    #[test]
    fn test_boolean_all_ops_stress() {
        // Run all 3 operations on overlapping boxes — ensure none panic
        let offsets = [
            Point3::new(0.5, 0.0, 0.0),
            Point3::new(0.0, 0.5, 0.0),
            Point3::new(0.0, 0.0, 0.5),
            Point3::new(0.5, 0.5, 0.5),
            Point3::new(-0.5, -0.5, -0.5),
        ];

        for offset in &offsets {
            let mut a = BRepModel::new();
            let ra = make_box(&mut a, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

            let mut b = BRepModel::new();
            let rb = make_box(&mut b, *offset, 1.0, 1.0, 1.0).unwrap();

            for op in [BooleanOp::Union, BooleanOp::Intersection, BooleanOp::Difference] {
                let result = boolean_op(&a, ra.solid, &b, rb.solid, op);
                assert!(
                    result.is_ok(),
                    "boolean {:?} at offset {:?} should not panic/fail",
                    op, offset
                );
            }
        }
    }
}
