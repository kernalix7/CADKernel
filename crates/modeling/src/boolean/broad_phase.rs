use cadkernel_core::{KernelError, KernelResult};
use cadkernel_geometry::bvh::{Aabb, Bvh};
use cadkernel_math::BoundingBox;
use cadkernel_topology::{BRepModel, FaceData, Handle};

/// Computes a tight AABB for a face by walking its outer loop vertices.
pub fn face_bbox(model: &BRepModel, face: Handle<FaceData>) -> KernelResult<BoundingBox> {
    let face_data = model
        .faces
        .get(face)
        .ok_or(KernelError::InvalidHandle("face"))?;
    let loop_data = model
        .loops
        .get(face_data.outer_loop)
        .ok_or(KernelError::InvalidHandle("loop"))?;
    let hes = model.loop_half_edges(loop_data.half_edge);

    let mut bb = BoundingBox::empty();
    for he_h in hes {
        let he = model
            .half_edges
            .get(he_h)
            .ok_or(KernelError::InvalidHandle("half_edge"))?;
        let v = model
            .vertices
            .get(he.origin)
            .ok_or(KernelError::InvalidHandle("vertex"))?;
        bb.include_point(v.point);
    }
    Ok(bb)
}

/// Returns pairs of face handles `(face_a, face_b)` from two different
/// solids whose bounding boxes overlap.
pub fn find_overlapping_face_pairs(
    model_a: &BRepModel,
    faces_a: &[Handle<FaceData>],
    model_b: &BRepModel,
    faces_b: &[Handle<FaceData>],
) -> KernelResult<Vec<(Handle<FaceData>, Handle<FaceData>)>> {
    let bboxes_a: Vec<_> = faces_a
        .iter()
        .map(|&f| face_bbox(model_a, f).map(|bb| (f, bb)))
        .collect::<KernelResult<Vec<_>>>()?;
    let bboxes_b: Vec<_> = faces_b
        .iter()
        .map(|&f| face_bbox(model_b, f).map(|bb| (f, bb)))
        .collect::<KernelResult<Vec<_>>>()?;

    // Build BVH over faces_b for O(n log n) overlap detection
    let bvh_items: Vec<(Aabb, usize)> = bboxes_b
        .iter()
        .enumerate()
        .map(|(i, (_, bb))| {
            (Aabb::new(bb.min, bb.max), i)
        })
        .collect();
    let bvh = Bvh::build(&bvh_items);

    let mut pairs = Vec::new();
    for &(fa, ref ba) in &bboxes_a {
        let query = Aabb::new(ba.min, ba.max);
        for idx in bvh.query_aabb(&query) {
            pairs.push((fa, bboxes_b[idx].0));
        }
    }
    Ok(pairs)
}

/// Collects all face handles from a solid in the model.
pub fn collect_solid_faces(
    model: &BRepModel,
    solid: Handle<cadkernel_topology::SolidData>,
) -> KernelResult<Vec<Handle<FaceData>>> {
    let solid_data = model
        .solids
        .get(solid)
        .ok_or(KernelError::InvalidHandle("solid"))?;
    let mut faces = Vec::new();
    for &shell_h in &solid_data.shells {
        let shell = model
            .shells
            .get(shell_h)
            .ok_or(KernelError::InvalidHandle("shell"))?;
        faces.extend_from_slice(&shell.faces);
    }
    Ok(faces)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::make_box;
    use cadkernel_math::Point3;

    #[test]
    fn test_overlapping_boxes() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(1.0, 1.0, 1.0), 2.0, 2.0, 2.0).unwrap();

        let faces_a = collect_solid_faces(&a, ra.solid).unwrap();
        let faces_b = collect_solid_faces(&b, rb.solid).unwrap();

        let pairs = find_overlapping_face_pairs(&a, &faces_a, &b, &faces_b).unwrap();
        assert!(
            !pairs.is_empty(),
            "overlapping boxes must produce face pairs"
        );
    }

    #[test]
    fn test_disjoint_boxes() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(10.0, 10.0, 10.0), 1.0, 1.0, 1.0).unwrap();

        let faces_a = collect_solid_faces(&a, ra.solid).unwrap();
        let faces_b = collect_solid_faces(&b, rb.solid).unwrap();

        let pairs = find_overlapping_face_pairs(&a, &faces_a, &b, &faces_b).unwrap();
        assert!(pairs.is_empty(), "disjoint boxes must produce no pairs");
    }
}
