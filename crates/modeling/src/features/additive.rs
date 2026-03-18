//! Additive and subtractive primitive operations for PartDesign.
//!
//! Each function creates a primitive in a temporary model and performs a
//! boolean operation (union or subtraction) against a base solid.

use cadkernel_core::KernelResult;
use cadkernel_math::Point3;
use cadkernel_topology::BRepModel;

use crate::boolean::{BooleanOp, boolean_op};
use crate::features::loft::loft;
use crate::features::sweep::sweep;
use crate::primitives::{
    make_box, make_cone, make_cylinder, make_ellipsoid, make_helix, make_prism, make_sphere,
    make_torus, make_wedge,
};

/// Additive box: creates a box and boolean-unions it with the base solid.
pub fn additive_box(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    origin: Point3,
    dx: f64,
    dy: f64,
    dz: f64,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_box(&mut tool, origin, dx, dy, dz)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Union)
}

/// Subtractive box: creates a box and boolean-subtracts it from the base solid.
pub fn subtractive_box(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    origin: Point3,
    dx: f64,
    dy: f64,
    dz: f64,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_box(&mut tool, origin, dx, dy, dz)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Difference)
}

/// Additive cylinder: creates a cylinder and boolean-unions it with the base solid.
pub fn additive_cylinder(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    base_center: Point3,
    radius: f64,
    height: f64,
    segments: usize,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_cylinder(&mut tool, base_center, radius, height, segments)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Union)
}

/// Subtractive cylinder: creates a cylinder and boolean-subtracts it from the base solid.
pub fn subtractive_cylinder(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    base_center: Point3,
    radius: f64,
    height: f64,
    segments: usize,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_cylinder(&mut tool, base_center, radius, height, segments)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Difference)
}

/// Additive sphere: creates a sphere and boolean-unions it with the base solid.
pub fn additive_sphere(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    center: Point3,
    radius: f64,
    segments: usize,
    rings: usize,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_sphere(&mut tool, center, radius, segments, rings)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Union)
}

/// Subtractive sphere: creates a sphere and boolean-subtracts it from the base solid.
pub fn subtractive_sphere(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    center: Point3,
    radius: f64,
    segments: usize,
    rings: usize,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_sphere(&mut tool, center, radius, segments, rings)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Difference)
}

/// Additive cone: creates a cone and boolean-unions it with the base solid.
pub fn additive_cone(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    base_center: Point3,
    base_radius: f64,
    top_radius: f64,
    height: f64,
    segments: usize,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_cone(&mut tool, base_center, base_radius, top_radius, height, segments)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Union)
}

/// Subtractive cone: creates a cone and boolean-subtracts it from the base solid.
pub fn subtractive_cone(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    base_center: Point3,
    base_radius: f64,
    top_radius: f64,
    height: f64,
    segments: usize,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_cone(&mut tool, base_center, base_radius, top_radius, height, segments)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Difference)
}

/// Additive torus: creates a torus and boolean-unions it with the base solid.
pub fn additive_torus(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    center: Point3,
    major_radius: f64,
    minor_radius: f64,
    major_segments: usize,
    minor_segments: usize,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_torus(&mut tool, center, major_radius, minor_radius, major_segments, minor_segments)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Union)
}

/// Subtractive torus: creates a torus and boolean-subtracts it from the base solid.
pub fn subtractive_torus(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    center: Point3,
    major_radius: f64,
    minor_radius: f64,
    major_segments: usize,
    minor_segments: usize,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_torus(&mut tool, center, major_radius, minor_radius, major_segments, minor_segments)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Difference)
}

/// Additive helix: creates a helix and boolean-unions it with the base solid.
#[allow(clippy::too_many_arguments)]
pub fn additive_helix(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    center: Point3,
    radius: f64,
    pitch: f64,
    turns: f64,
    tube_radius: f64,
    segments: usize,
    tube_segments: usize,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_helix(&mut tool, center, radius, pitch, turns, tube_radius, segments, tube_segments)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Union)
}

/// Subtractive helix: creates a helix and boolean-subtracts it from the base solid.
#[allow(clippy::too_many_arguments)]
pub fn subtractive_helix(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    center: Point3,
    radius: f64,
    pitch: f64,
    turns: f64,
    tube_radius: f64,
    segments: usize,
    tube_segments: usize,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_helix(&mut tool, center, radius, pitch, turns, tube_radius, segments, tube_segments)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Difference)
}

/// Additive ellipsoid: creates an ellipsoid and boolean-unions it with the base solid.
#[allow(clippy::too_many_arguments)]
pub fn additive_ellipsoid(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    center: Point3,
    rx: f64,
    ry: f64,
    rz: f64,
    segments: usize,
    rings: usize,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_ellipsoid(&mut tool, center, rx, ry, rz, segments, rings)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Union)
}

/// Subtractive ellipsoid: creates an ellipsoid and boolean-subtracts it from the base solid.
#[allow(clippy::too_many_arguments)]
pub fn subtractive_ellipsoid(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    center: Point3,
    rx: f64,
    ry: f64,
    rz: f64,
    segments: usize,
    rings: usize,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_ellipsoid(&mut tool, center, rx, ry, rz, segments, rings)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Difference)
}

/// Additive prism: creates a regular polygon prism and boolean-unions it with the base solid.
pub fn additive_prism(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    base_center: Point3,
    radius: f64,
    height: f64,
    sides: usize,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_prism(&mut tool, base_center, radius, height, sides)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Union)
}

/// Subtractive prism: creates a regular polygon prism and boolean-subtracts it from the base solid.
pub fn subtractive_prism(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    base_center: Point3,
    radius: f64,
    height: f64,
    sides: usize,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_prism(&mut tool, base_center, radius, height, sides)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Difference)
}

/// Additive wedge: creates a tapered box and boolean-unions it with the base solid.
#[allow(clippy::too_many_arguments)]
pub fn additive_wedge(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    origin: Point3,
    dx: f64,
    dy: f64,
    dz: f64,
    dx2: f64,
    dy2: f64,
    xoff: f64,
    yoff: f64,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_wedge(&mut tool, origin, dx, dy, dz, dx2, dy2, xoff, yoff)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Union)
}

/// Subtractive wedge: creates a tapered box and boolean-subtracts it from the base solid.
#[allow(clippy::too_many_arguments)]
pub fn subtractive_wedge(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    origin: Point3,
    dx: f64,
    dy: f64,
    dz: f64,
    dx2: f64,
    dy2: f64,
    xoff: f64,
    yoff: f64,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_wedge(&mut tool, origin, dx, dy, dz, dx2, dy2, xoff, yoff)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Difference)
}

/// Subtractive loft: lofts between profiles and boolean-subtracts from the base solid.
pub fn subtractive_loft(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    profiles: &[&[Point3]],
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = loft(&mut tool, profiles)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Difference)
}

/// Subtractive pipe (sweep): sweeps a profile along a path and boolean-subtracts from the base solid.
pub fn subtractive_pipe(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    profile: &[Point3],
    path: &[Point3],
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = sweep(&mut tool, profile, path)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Difference)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_additive_box() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let result = additive_box(
            &base_model,
            base.solid,
            Point3::new(10.0, 10.0, 10.0),
            1.0,
            1.0,
            1.0,
        )
        .unwrap();

        // Union of two disjoint boxes should produce 1 solid with all 12 faces
        assert_eq!(result.solids.len(), 1);
        assert!(result.faces.len() >= 12);
    }

    #[test]
    fn test_subtractive_box() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let result = subtractive_box(
            &base_model,
            base.solid,
            Point3::new(10.0, 10.0, 10.0),
            1.0,
            1.0,
            1.0,
        )
        .unwrap();

        // Subtracting a disjoint box should preserve the original 6 faces
        assert_eq!(result.faces.len(), 6);
    }

    #[test]
    fn test_additive_cylinder() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let result = additive_cylinder(
            &base_model,
            base.solid,
            Point3::new(10.0, 10.0, 0.0),
            1.0,
            3.0,
            8,
        )
        .unwrap();

        assert_eq!(result.solids.len(), 1);
    }

    #[test]
    fn test_subtractive_cylinder() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let result = subtractive_cylinder(
            &base_model,
            base.solid,
            Point3::new(10.0, 10.0, 0.0),
            0.5,
            4.0,
            8,
        )
        .unwrap();

        // Disjoint subtraction preserves original
        assert_eq!(result.faces.len(), 6);
    }

    #[test]
    fn test_additive_sphere() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let result = additive_sphere(
            &base_model,
            base.solid,
            Point3::new(10.0, 10.0, 10.0),
            1.0,
            8,
            4,
        )
        .unwrap();

        assert_eq!(result.solids.len(), 1);
    }

    #[test]
    fn test_subtractive_sphere() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let result = subtractive_sphere(
            &base_model,
            base.solid,
            Point3::new(10.0, 10.0, 10.0),
            0.5,
            8,
            4,
        )
        .unwrap();

        assert_eq!(result.faces.len(), 6);
    }

    #[test]
    fn test_additive_cone() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let result = additive_cone(
            &base_model,
            base.solid,
            Point3::new(10.0, 10.0, 0.0),
            1.0,
            0.5,
            3.0,
            8,
        )
        .unwrap();

        assert_eq!(result.solids.len(), 1);
    }

    #[test]
    fn test_subtractive_cone() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let result = subtractive_cone(
            &base_model,
            base.solid,
            Point3::new(10.0, 10.0, 0.0),
            0.5,
            0.2,
            3.0,
            8,
        )
        .unwrap();

        assert_eq!(result.faces.len(), 6);
    }

    #[test]
    fn test_additive_torus() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let result = additive_torus(
            &base_model,
            base.solid,
            Point3::new(20.0, 20.0, 20.0),
            3.0,
            0.5,
            8,
            6,
        )
        .unwrap();

        assert_eq!(result.solids.len(), 1);
    }

    #[test]
    fn test_subtractive_torus() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let result = subtractive_torus(
            &base_model,
            base.solid,
            Point3::new(20.0, 20.0, 20.0),
            3.0,
            0.5,
            8,
            6,
        )
        .unwrap();

        assert_eq!(result.faces.len(), 6);
    }

    #[test]
    fn test_additive_helix() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let result = additive_helix(
            &base_model,
            base.solid,
            Point3::new(20.0, 20.0, 0.0),
            3.0, 2.0, 1.0, 0.3, 8, 4,
        ).unwrap();

        assert_eq!(result.solids.len(), 1);
    }

    #[test]
    fn test_subtractive_helix() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let result = subtractive_helix(
            &base_model,
            base.solid,
            Point3::new(20.0, 20.0, 0.0),
            3.0, 2.0, 1.0, 0.3, 8, 4,
        ).unwrap();

        assert_eq!(result.faces.len(), 6);
    }

    #[test]
    fn test_additive_ellipsoid() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let result = additive_ellipsoid(
            &base_model,
            base.solid,
            Point3::new(20.0, 20.0, 20.0),
            1.0, 1.5, 2.0, 8, 4,
        ).unwrap();

        assert_eq!(result.solids.len(), 1);
    }

    #[test]
    fn test_subtractive_ellipsoid() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let result = subtractive_ellipsoid(
            &base_model,
            base.solid,
            Point3::new(20.0, 20.0, 20.0),
            0.5, 0.5, 0.5, 8, 4,
        ).unwrap();

        assert_eq!(result.faces.len(), 6);
    }

    #[test]
    fn test_additive_prism() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let result = additive_prism(
            &base_model,
            base.solid,
            Point3::new(20.0, 20.0, 0.0),
            1.0, 3.0, 6,
        ).unwrap();

        assert_eq!(result.solids.len(), 1);
    }

    #[test]
    fn test_subtractive_prism() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let result = subtractive_prism(
            &base_model,
            base.solid,
            Point3::new(20.0, 20.0, 0.0),
            0.5, 3.0, 6,
        ).unwrap();

        assert_eq!(result.faces.len(), 6);
    }

    #[test]
    fn test_additive_wedge() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let result = additive_wedge(
            &base_model,
            base.solid,
            Point3::new(20.0, 20.0, 0.0),
            2.0, 2.0, 3.0, 1.0, 1.0, 0.5, 0.5,
        ).unwrap();

        assert_eq!(result.solids.len(), 1);
    }

    #[test]
    fn test_subtractive_wedge() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let result = subtractive_wedge(
            &base_model,
            base.solid,
            Point3::new(20.0, 20.0, 0.0),
            1.0, 1.0, 2.0, 0.5, 0.5, 0.25, 0.25,
        ).unwrap();

        assert_eq!(result.faces.len(), 6);
    }

    #[test]
    fn test_subtractive_loft() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let p0 = vec![
            Point3::new(20.0, 20.0, 0.0),
            Point3::new(21.0, 20.0, 0.0),
            Point3::new(21.0, 21.0, 0.0),
            Point3::new(20.0, 21.0, 0.0),
        ];
        let p1 = vec![
            Point3::new(20.0, 20.0, 3.0),
            Point3::new(21.0, 20.0, 3.0),
            Point3::new(21.0, 21.0, 3.0),
            Point3::new(20.0, 21.0, 3.0),
        ];

        let result = subtractive_loft(
            &base_model,
            base.solid,
            &[p0.as_slice(), p1.as_slice()],
        ).unwrap();

        assert_eq!(result.faces.len(), 6);
    }

    #[test]
    fn test_subtractive_pipe() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let profile = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.3, 0.0, 0.0),
            Point3::new(0.3, 0.3, 0.0),
            Point3::new(0.0, 0.3, 0.0),
        ];
        let path = vec![
            Point3::new(20.0, 20.0, 0.0),
            Point3::new(20.0, 20.0, 3.0),
            Point3::new(20.0, 20.0, 6.0),
        ];

        let result = subtractive_pipe(
            &base_model,
            base.solid,
            &profile,
            &path,
        ).unwrap();

        assert_eq!(result.faces.len(), 6);
    }
}
