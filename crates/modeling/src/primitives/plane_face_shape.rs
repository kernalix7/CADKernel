//! Bounded planar face (thin box) primitive.

use cadkernel_core::KernelResult;
use cadkernel_math::Point3;
use cadkernel_topology::{BRepModel, FaceData, Handle, SolidData};

use super::box_shape::make_box;

/// Handles returned from [`make_plane_face`].
pub struct PlaneFaceResult {
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Creates a flat rectangular face as a very thin box (thickness 0.001).
///
/// The face lies in the XY plane at the given origin, with extent
/// `width` along X and `height` along Y.
pub fn make_plane_face(
    model: &mut BRepModel,
    origin: Point3,
    width: f64,
    height: f64,
) -> KernelResult<PlaneFaceResult> {
    let thickness = 0.001;
    let result = make_box(model, origin, width, height, thickness)?;
    Ok(PlaneFaceResult {
        solid: result.solid,
        faces: result.faces,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plane_face_creation() {
        let mut model = BRepModel::new();
        let r = make_plane_face(&mut model, Point3::ORIGIN, 10.0, 5.0).unwrap();
        // A box has 6 faces
        assert_eq!(r.faces.len(), 6);
        assert!(model.solids.is_alive(r.solid));
    }

    #[test]
    fn test_plane_face_with_offset() {
        let mut model = BRepModel::new();
        let r = make_plane_face(&mut model, Point3::new(1.0, 2.0, 3.0), 4.0, 6.0).unwrap();
        assert_eq!(r.faces.len(), 6);
        assert!(model.solids.is_alive(r.solid));
    }

    #[test]
    fn test_plane_face_vertex_count() {
        let mut model = BRepModel::new();
        let _r = make_plane_face(&mut model, Point3::ORIGIN, 2.0, 3.0).unwrap();
        assert_eq!(model.vertices.len(), 8);
    }
}
