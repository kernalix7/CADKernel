//! Convenience wrappers that hide [`OperationId`] and [`BRepModel`] setup.
//!
//! Every `quick_*` function creates a fresh model internally, invokes the
//! real kernel operation, and returns the populated model. This is useful
//! for scripts, tests, and one-shot computations where the caller does not
//! need fine-grained control over the operation history.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::Point3;
use cadkernel_topology::{BRepModel, SolidData};

use crate::boolean::{BooleanOp, boolean_op};
use crate::measure::solid_mass_properties;
use crate::primitives::{make_box, make_cone, make_cylinder, make_sphere, make_torus};

// ---------------------------------------------------------------------------
// Primitive helpers
// ---------------------------------------------------------------------------

/// Creates a box at the origin with the given dimensions.
pub fn quick_box(w: f64, h: f64, d: f64) -> KernelResult<BRepModel> {
    if w <= 0.0 {
        return Err(KernelError::InvalidArgument(format!(
            "width must be > 0, got {w}"
        )));
    }
    if h <= 0.0 {
        return Err(KernelError::InvalidArgument(format!(
            "height must be > 0, got {h}"
        )));
    }
    if d <= 0.0 {
        return Err(KernelError::InvalidArgument(format!(
            "depth must be > 0, got {d}"
        )));
    }
    let mut model = BRepModel::new();
    make_box(&mut model, Point3::ORIGIN, w, h, d)?;
    Ok(model)
}

/// Creates a cylinder at the origin (base on XY plane, axis along Z).
pub fn quick_cylinder(radius: f64, height: f64) -> KernelResult<BRepModel> {
    if radius <= 0.0 {
        return Err(KernelError::InvalidArgument(format!(
            "radius must be > 0, got {radius}"
        )));
    }
    if height <= 0.0 {
        return Err(KernelError::InvalidArgument(format!(
            "height must be > 0, got {height}"
        )));
    }
    let mut model = BRepModel::new();
    make_cylinder(&mut model, Point3::ORIGIN, radius, height, 64)?;
    Ok(model)
}

/// Creates a UV-sphere at the origin.
pub fn quick_sphere(radius: f64) -> KernelResult<BRepModel> {
    if radius <= 0.0 {
        return Err(KernelError::InvalidArgument(format!(
            "radius must be > 0, got {radius}"
        )));
    }
    let mut model = BRepModel::new();
    make_sphere(&mut model, Point3::ORIGIN, radius, 64, 32)?;
    Ok(model)
}

/// Creates a cone (pointed, top radius = 0) at the origin.
pub fn quick_cone(radius: f64, height: f64) -> KernelResult<BRepModel> {
    if radius <= 0.0 {
        return Err(KernelError::InvalidArgument(format!(
            "radius must be > 0, got {radius}"
        )));
    }
    if height <= 0.0 {
        return Err(KernelError::InvalidArgument(format!(
            "height must be > 0, got {height}"
        )));
    }
    let mut model = BRepModel::new();
    make_cone(&mut model, Point3::ORIGIN, radius, 0.0, height, 64)?;
    Ok(model)
}

/// Creates a torus at the origin lying in the XY plane.
pub fn quick_torus(major: f64, minor: f64) -> KernelResult<BRepModel> {
    if major <= 0.0 {
        return Err(KernelError::InvalidArgument(format!(
            "major radius must be > 0, got {major}"
        )));
    }
    if minor <= 0.0 {
        return Err(KernelError::InvalidArgument(format!(
            "minor radius must be > 0, got {minor}"
        )));
    }
    if minor >= major {
        return Err(KernelError::InvalidArgument(format!(
            "minor radius ({minor}) must be < major radius ({major})"
        )));
    }
    let mut model = BRepModel::new();
    make_torus(&mut model, Point3::ORIGIN, major, minor, 64, 32)?;
    Ok(model)
}

// ---------------------------------------------------------------------------
// Boolean helpers
// ---------------------------------------------------------------------------

/// Returns the first solid handle from a model, or an error if no solids exist.
fn first_solid(model: &BRepModel) -> KernelResult<cadkernel_topology::Handle<SolidData>> {
    model
        .solids
        .iter()
        .next()
        .map(|(h, _)| h)
        .ok_or(KernelError::TopologyError(
            "model contains no solids".into(),
        ))
}

/// Boolean union of the first solid in each model.
pub fn quick_union(a: &BRepModel, b: &BRepModel) -> KernelResult<BRepModel> {
    let sa = first_solid(a)?;
    let sb = first_solid(b)?;
    boolean_op(a, sa, b, sb, BooleanOp::Union)
}

/// Boolean subtract: first solid of `a` minus first solid of `b`.
pub fn quick_subtract(a: &BRepModel, b: &BRepModel) -> KernelResult<BRepModel> {
    let sa = first_solid(a)?;
    let sb = first_solid(b)?;
    boolean_op(a, sa, b, sb, BooleanOp::Difference)
}

/// Boolean intersection of the first solid in each model.
pub fn quick_intersect(a: &BRepModel, b: &BRepModel) -> KernelResult<BRepModel> {
    let sa = first_solid(a)?;
    let sb = first_solid(b)?;
    boolean_op(a, sa, b, sb, BooleanOp::Intersection)
}

// ---------------------------------------------------------------------------
// Measurement helpers
// ---------------------------------------------------------------------------

/// Approximate volume of the first solid via divergence theorem on tessellated mesh.
pub fn quick_volume(model: &BRepModel) -> KernelResult<f64> {
    let solid = first_solid(model)?;
    let props = solid_mass_properties(model, solid)?;
    Ok(props.volume)
}

/// Approximate surface area of the first solid via tessellation.
pub fn quick_area(model: &BRepModel) -> KernelResult<f64> {
    let solid = first_solid(model)?;
    let props = solid_mass_properties(model, solid)?;
    Ok(props.surface_area)
}

/// Approximate centroid of the first solid as `(x, y, z)`.
pub fn quick_centroid(model: &BRepModel) -> KernelResult<(f64, f64, f64)> {
    let solid = first_solid(model)?;
    let props = solid_mass_properties(model, solid)?;
    Ok((props.centroid.x, props.centroid.y, props.centroid.z))
}

/// Min and max corners of an axis-aligned bounding box.
pub type BBoxResult = ((f64, f64, f64), (f64, f64, f64));

/// Axis-aligned bounding box of the first solid as `((min_x, min_y, min_z), (max_x, max_y, max_z))`.
///
/// Computed from the tessellated vertex positions.
pub fn quick_bbox(model: &BRepModel) -> KernelResult<BBoxResult> {
    let solid = first_solid(model)?;
    let mesh = cadkernel_io::tessellate_solid(model, solid);
    if mesh.vertices.is_empty() {
        return Err(KernelError::InvalidArgument(
            "solid has no tessellatable geometry for bounding box".into(),
        ));
    }
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut min_z = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut max_z = f64::NEG_INFINITY;
    for v in &mesh.vertices {
        min_x = min_x.min(v.x);
        min_y = min_y.min(v.y);
        min_z = min_z.min(v.z);
        max_x = max_x.max(v.x);
        max_y = max_y.max(v.y);
        max_z = max_z.max(v.z);
    }
    Ok(((min_x, min_y, min_z), (max_x, max_y, max_z)))
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------
    // Primitive tests
    // -------------------------------------------------------------------

    #[test]
    fn test_quick_box() {
        let model = quick_box(2.0, 3.0, 4.0).unwrap();
        assert_eq!(model.solids.len(), 1);
        assert_eq!(model.faces.len(), 6);
    }

    #[test]
    fn test_quick_box_invalid() {
        assert!(quick_box(0.0, 1.0, 1.0).is_err());
        assert!(quick_box(1.0, -1.0, 1.0).is_err());
        assert!(quick_box(1.0, 1.0, -0.5).is_err());
    }

    #[test]
    fn test_quick_cylinder() {
        let model = quick_cylinder(1.5, 4.0).unwrap();
        assert_eq!(model.solids.len(), 1);
    }

    #[test]
    fn test_quick_cylinder_invalid() {
        assert!(quick_cylinder(-1.0, 5.0).is_err());
        assert!(quick_cylinder(1.0, 0.0).is_err());
    }

    #[test]
    fn test_quick_sphere() {
        let model = quick_sphere(3.0).unwrap();
        assert_eq!(model.solids.len(), 1);
    }

    #[test]
    fn test_quick_sphere_invalid() {
        assert!(quick_sphere(0.0).is_err());
        assert!(quick_sphere(-2.0).is_err());
    }

    #[test]
    fn test_quick_cone() {
        let model = quick_cone(2.0, 5.0).unwrap();
        assert_eq!(model.solids.len(), 1);
    }

    #[test]
    fn test_quick_cone_invalid() {
        assert!(quick_cone(0.0, 5.0).is_err());
        assert!(quick_cone(2.0, -1.0).is_err());
    }

    #[test]
    fn test_quick_torus() {
        let model = quick_torus(5.0, 1.0).unwrap();
        assert_eq!(model.solids.len(), 1);
    }

    #[test]
    fn test_quick_torus_invalid() {
        assert!(quick_torus(0.0, 1.0).is_err());
        assert!(quick_torus(5.0, 0.0).is_err());
        assert!(quick_torus(3.0, 5.0).is_err(), "minor >= major should fail");
    }

    // -------------------------------------------------------------------
    // Boolean tests
    // -------------------------------------------------------------------

    #[test]
    fn test_quick_union_disjoint() {
        let a = quick_box(1.0, 1.0, 1.0).unwrap();
        let b = quick_box(1.0, 1.0, 1.0).unwrap();
        let result = quick_union(&a, &b).unwrap();
        assert!(!result.solids.is_empty());
    }

    #[test]
    fn test_quick_subtract() {
        let a = quick_box(10.0, 10.0, 10.0).unwrap();
        let b = quick_box(1.0, 1.0, 1.0).unwrap();
        let result = quick_subtract(&a, &b).unwrap();
        assert!(!result.faces.is_empty());
    }

    #[test]
    fn test_quick_intersect_disjoint() {
        let a = quick_box(1.0, 1.0, 1.0).unwrap();
        // Second box is far away -- no intersection
        let mut b = BRepModel::new();
        make_box(&mut b, Point3::new(100.0, 100.0, 100.0), 1.0, 1.0, 1.0).unwrap();
        let result = quick_intersect(&a, &b).unwrap();
        assert_eq!(result.faces.len(), 0, "disjoint intersection should be empty");
    }

    #[test]
    fn test_quick_boolean_no_solid_error() {
        let empty = BRepModel::new();
        let b = quick_box(1.0, 1.0, 1.0).unwrap();
        assert!(quick_union(&empty, &b).is_err());
        assert!(quick_subtract(&empty, &b).is_err());
        assert!(quick_intersect(&empty, &b).is_err());
    }

    // -------------------------------------------------------------------
    // Measurement tests
    // -------------------------------------------------------------------

    #[test]
    fn test_quick_volume_unit_cube() {
        let model = quick_box(1.0, 1.0, 1.0).unwrap();
        let vol = quick_volume(&model).unwrap();
        assert!(
            (vol - 1.0).abs() < 0.05,
            "unit cube volume should be ~1.0, got {vol}"
        );
    }

    #[test]
    fn test_quick_volume_scaled_box() {
        let model = quick_box(2.0, 3.0, 4.0).unwrap();
        let vol = quick_volume(&model).unwrap();
        assert!(
            (vol - 24.0).abs() < 1.0,
            "2x3x4 box volume should be ~24, got {vol}"
        );
    }

    #[test]
    fn test_quick_area_unit_cube() {
        let model = quick_box(1.0, 1.0, 1.0).unwrap();
        let area = quick_area(&model).unwrap();
        assert!(
            (area - 6.0).abs() < 0.5,
            "unit cube area should be ~6.0, got {area}"
        );
    }

    #[test]
    fn test_quick_centroid_unit_cube() {
        let model = quick_box(1.0, 1.0, 1.0).unwrap();
        let (cx, cy, cz) = quick_centroid(&model).unwrap();
        assert!((cx - 0.5).abs() < 0.1, "centroid x ~0.5, got {cx}");
        assert!((cy - 0.5).abs() < 0.1, "centroid y ~0.5, got {cy}");
        assert!((cz - 0.5).abs() < 0.1, "centroid z ~0.5, got {cz}");
    }

    #[test]
    fn test_quick_bbox_unit_cube() {
        let model = quick_box(1.0, 1.0, 1.0).unwrap();
        let ((min_x, min_y, min_z), (max_x, max_y, max_z)) = quick_bbox(&model).unwrap();
        assert!(min_x.abs() < 0.01);
        assert!(min_y.abs() < 0.01);
        assert!(min_z.abs() < 0.01);
        assert!((max_x - 1.0).abs() < 0.01);
        assert!((max_y - 1.0).abs() < 0.01);
        assert!((max_z - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_quick_bbox_offset_box() {
        let mut model = BRepModel::new();
        make_box(&mut model, Point3::new(5.0, 6.0, 7.0), 2.0, 3.0, 4.0).unwrap();
        let ((min_x, min_y, min_z), (max_x, max_y, max_z)) = quick_bbox(&model).unwrap();
        assert!((min_x - 5.0).abs() < 0.01);
        assert!((min_y - 6.0).abs() < 0.01);
        assert!((min_z - 7.0).abs() < 0.01);
        assert!((max_x - 7.0).abs() < 0.01);
        assert!((max_y - 9.0).abs() < 0.01);
        assert!((max_z - 11.0).abs() < 0.01);
    }

    #[test]
    fn test_quick_measurement_no_solid_error() {
        let empty = BRepModel::new();
        assert!(quick_volume(&empty).is_err());
        assert!(quick_area(&empty).is_err());
        assert!(quick_centroid(&empty).is_err());
        assert!(quick_bbox(&empty).is_err());
    }
}
