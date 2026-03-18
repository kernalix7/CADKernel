//! Regular polygon prism primitive.

use cadkernel_core::KernelResult;
use cadkernel_math::Point3;
use cadkernel_topology::{BRepModel, FaceData, Handle, SolidData};

use super::prism_shape::make_prism;

/// Handles returned from [`make_polygon`].
pub struct PolygonResult {
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Creates a regular polygon prism (thin plate if height is small).
///
/// This delegates to [`make_prism`] with the given parameters.
pub fn make_polygon(
    model: &mut BRepModel,
    center: Point3,
    radius: f64,
    sides: usize,
    height: f64,
) -> KernelResult<PolygonResult> {
    let result = make_prism(model, center, radius, height, sides)?;
    Ok(PolygonResult {
        solid: result.solid,
        faces: result.faces,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hexagon() {
        let mut model = BRepModel::new();
        let r = make_polygon(&mut model, Point3::ORIGIN, 1.0, 6, 0.1).unwrap();
        // 2 caps + 6 lateral = 8 faces
        assert_eq!(r.faces.len(), 8);
        assert!(model.solids.is_alive(r.solid));
    }

    #[test]
    fn test_triangle() {
        let mut model = BRepModel::new();
        let r = make_polygon(&mut model, Point3::ORIGIN, 2.0, 3, 1.0).unwrap();
        // 2 caps + 3 lateral = 5 faces
        assert_eq!(r.faces.len(), 5);
        assert!(model.solids.is_alive(r.solid));
    }

    #[test]
    fn test_polygon_validation() {
        let mut model = BRepModel::new();
        assert!(make_polygon(&mut model, Point3::ORIGIN, 1.0, 2, 1.0).is_err());
        assert!(make_polygon(&mut model, Point3::ORIGIN, -1.0, 4, 1.0).is_err());
        assert!(make_polygon(&mut model, Point3::ORIGIN, 1.0, 4, -1.0).is_err());
    }
}
