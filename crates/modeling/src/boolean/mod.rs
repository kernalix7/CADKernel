pub mod broad_phase;
pub mod classify;
pub mod csg;
pub mod evaluate;

pub use csg::BooleanOp;
pub use evaluate::boolean_op;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::make_box;
    use cadkernel_math::Point3;
    use cadkernel_topology::BRepModel;

    #[test]
    fn test_union_produces_combined_faces() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(10.0, 10.0, 10.0), 2.0, 2.0, 2.0).unwrap();

        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Union).unwrap();
        assert!(
            result.faces.len() >= 12,
            "union of two disjoint boxes should keep all 12 faces, got {}",
            result.faces.len()
        );
        assert_eq!(result.solids.len(), 1);
    }

    #[test]
    fn test_subtract_removes_intersection() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(10.0, 10.0, 10.0), 2.0, 2.0, 2.0).unwrap();

        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Difference).unwrap();
        assert_eq!(
            result.faces.len(),
            6,
            "difference with disjoint box should keep A's 6 faces"
        );
    }

    #[test]
    fn test_intersect_keeps_only_overlap() {
        let mut a = BRepModel::new();
        let ra = make_box(&mut a, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut b = BRepModel::new();
        let rb = make_box(&mut b, Point3::new(10.0, 10.0, 10.0), 2.0, 2.0, 2.0).unwrap();

        let result = boolean_op(&a, ra.solid, &b, rb.solid, BooleanOp::Intersection).unwrap();
        assert_eq!(
            result.faces.len(),
            0,
            "intersection of disjoint boxes should produce no faces"
        );
    }
}
