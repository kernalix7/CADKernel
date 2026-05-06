//! Primitive solid constructors for the CAD kernel.
//!
//! Each function creates a solid in a [`BRepModel`](cadkernel_topology::BRepModel)
//! and returns a result struct containing the solid handle and handles to all
//! constituent faces, edges, and vertices. All primitives validate their
//! parameters (e.g. radius > 0, segments >= 3) and return
//! [`KernelResult`](cadkernel_core::KernelResult) on failure.

pub mod box_shape;
pub mod cone_shape;
pub mod cylinder_shape;
pub mod ellipsoid_shape;
pub mod helix_shape;
pub mod plane_face_shape;
pub mod polygon_shape;
pub mod prism_shape;
pub mod shape_primitives;
pub mod sphere_shape;
pub mod spiral_shape;
pub mod torus_shape;
pub mod tube_shape;
pub mod wedge_shape;

pub use box_shape::{BoxResult, make_box};
pub use cone_shape::{ConeResult, make_cone};
pub use cylinder_shape::{CylinderResult, make_cylinder};
pub use ellipsoid_shape::{EllipsoidResult, make_ellipsoid};
pub use helix_shape::{HelixResult, make_helix};
pub use plane_face_shape::{PlaneFaceResult, make_plane_face};
pub use polygon_shape::{PolygonResult, make_polygon};
pub use prism_shape::{PrismResult, make_prism};
pub use shape_primitives::{
    CircleShapeResult, ConvertToSolidResult, EllipseShapeResult, LineShapeResult, PointShapeResult,
    ShapeFromEdgesResult, convert_to_solid, make_circle_shape, make_ellipse_shape, make_line_shape,
    make_point_shape, shape_builder_from_edges,
};
pub use sphere_shape::{SphereResult, make_sphere};
pub use spiral_shape::{SpiralResult, make_spiral};
pub use torus_shape::{TorusResult, make_torus};
pub use tube_shape::{TubeResult, make_tube};
pub use wedge_shape::{WedgeResult, make_wedge};

use std::collections::HashMap;
use std::sync::Arc;

use cadkernel_core::KernelResult;
use cadkernel_geometry::LineSegment;
use cadkernel_math::Point3;
use cadkernel_topology::{
    BRepModel, EdgeData, EntityKind, HalfEdgeData, Handle, OperationId, SolidData, Tag, VertexData,
};

/// Parameters for the unified primitive constructor [`make_primitive`].
///
/// Dispatches to the appropriate `make_*` function based on the variant.
#[derive(Debug, Clone)]
pub enum PrimitiveParams {
    Box {
        width: f64,
        height: f64,
        depth: f64,
    },
    Cylinder {
        radius: f64,
        height: f64,
        segments: usize,
    },
    Sphere {
        radius: f64,
        segments: usize,
    },
    Cone {
        radius1: f64,
        radius2: f64,
        height: f64,
    },
    Torus {
        major_radius: f64,
        minor_radius: f64,
    },
}

/// Creates a primitive solid from unified parameters.
///
/// Dispatches to the underlying `make_box`, `make_cylinder`, `make_sphere`,
/// `make_cone`, or `make_torus` functions.
pub fn make_primitive(
    model: &mut BRepModel,
    params: &PrimitiveParams,
) -> KernelResult<Handle<SolidData>> {
    match params {
        PrimitiveParams::Box {
            width,
            height,
            depth,
        } => {
            let r = make_box(model, Point3::ORIGIN, *width, *height, *depth)?;
            Ok(r.solid)
        }
        PrimitiveParams::Cylinder {
            radius,
            height,
            segments,
        } => {
            let r = make_cylinder(model, Point3::ORIGIN, *radius, *height, *segments)?;
            Ok(r.solid)
        }
        PrimitiveParams::Sphere { radius, segments } => {
            let rings = (*segments / 2).max(2);
            let r = make_sphere(model, Point3::ORIGIN, *radius, *segments, rings)?;
            Ok(r.solid)
        }
        PrimitiveParams::Cone {
            radius1,
            radius2,
            height,
        } => {
            let r = make_cone(model, Point3::ORIGIN, *radius1, *radius2, *height, 64)?;
            Ok(r.solid)
        }
        PrimitiveParams::Torus {
            major_radius,
            minor_radius,
        } => {
            let r = make_torus(model, Point3::ORIGIN, *major_radius, *minor_radius, 64, 32)?;
            Ok(r.solid)
        }
    }
}

/// Creates a deep copy of a solid with an applied 4x4 transform matrix.
pub fn transformed_copy(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    transform: cadkernel_math::Mat4,
) -> KernelResult<Handle<SolidData>> {
    let op = model.history.next_operation("transformed_copy");
    let result = crate::features::copy_utils::copy_solid_transformed(
        model,
        solid,
        op,
        |p| transform.transform_point(p),
        false,
    )?;
    Ok(result.solid)
}

/// Edge deduplication cache for primitive construction.
///
/// Tracks edges by their vertex endpoint indices. When an edge between two
/// vertices already exists, returns the twin half-edge instead of creating a
/// duplicate topological edge.
type EdgeEntry = (Handle<EdgeData>, Handle<HalfEdgeData>, Handle<HalfEdgeData>);

pub(crate) struct EdgeCache {
    /// Maps `(v_start_index, v_end_index)` → `(edge, he_forward, he_backward)`.
    map: HashMap<(u32, u32), EdgeEntry>,
}

impl EdgeCache {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Returns the appropriate half-edge for the directed edge from `v_start` to
    /// `v_end`. If the edge already exists (in either direction), returns the
    /// matching twin half-edge. Otherwise creates a new edge.
    pub fn get_or_create(
        &mut self,
        model: &mut BRepModel,
        v_start: Handle<VertexData>,
        v_end: Handle<VertexData>,
        tag: Tag,
    ) -> Handle<HalfEdgeData> {
        let key_fwd = (v_start.index(), v_end.index());
        let key_rev = (v_end.index(), v_start.index());

        // Reverse direction edge exists — return the twin half-edge.
        if let Some(&(_, _he_fwd, he_bwd)) = self.map.get(&key_rev) {
            return he_bwd;
        }

        // Create a new edge and cache it.
        let (edge_h, he_fwd, he_bwd) = model.add_edge_tagged(v_start, v_end, tag);
        self.map.insert(key_fwd, (edge_h, he_fwd, he_bwd));
        he_fwd
    }

    /// Returns all unique edge handles created by this cache.
    pub fn all_edges(&self) -> Vec<Handle<EdgeData>> {
        self.map.values().map(|&(e, _, _)| e).collect()
    }

    /// Returns the number of unique edges created.
    #[allow(dead_code)]
    pub fn edge_count(&self) -> usize {
        self.map.len()
    }
}

/// Helper to create an edge tag with the given operation and a mutable edge
/// index counter.
pub(crate) fn next_edge_tag(op: OperationId, edge_idx: &mut u32) -> Tag {
    let tag = Tag::generated(EntityKind::Edge, op, *edge_idx);
    *edge_idx += 1;
    tag
}

/// Binds `LineSegment` curves to all edges tracked by the cache.
pub(crate) fn bind_edge_line_segments(model: &mut BRepModel, ec: &EdgeCache) {
    let bindings: Vec<_> = ec
        .all_edges()
        .into_iter()
        .filter_map(|edge_h| {
            let ed = model.edges.get(edge_h)?;
            let he_a = model.half_edges.get(ed.half_edge_a?)?;
            let he_b = model.half_edges.get(ed.half_edge_b?)?;
            let p0 = model.vertices.get(he_a.origin)?.point;
            let p1 = model.vertices.get(he_b.origin)?.point;
            Some((edge_h, p0, p1))
        })
        .collect();
    for (eh, p0, p1) in bindings {
        model.bind_edge_curve(eh, Arc::new(LineSegment::new(p0, p1)), (0.0, 1.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_math::Mat4;
    use cadkernel_topology::BRepModel;

    #[test]
    fn test_make_primitive_box() {
        let mut model = BRepModel::new();
        let params = PrimitiveParams::Box {
            width: 2.0,
            height: 3.0,
            depth: 4.0,
        };
        let solid = make_primitive(&mut model, &params).unwrap();
        assert!(model.solids.is_alive(solid));
    }

    #[test]
    fn test_make_primitive_cylinder() {
        let mut model = BRepModel::new();
        let params = PrimitiveParams::Cylinder {
            radius: 1.0,
            height: 5.0,
            segments: 16,
        };
        let solid = make_primitive(&mut model, &params).unwrap();
        assert!(model.solids.is_alive(solid));
    }

    #[test]
    fn test_make_primitive_sphere() {
        let mut model = BRepModel::new();
        let params = PrimitiveParams::Sphere {
            radius: 3.0,
            segments: 16,
        };
        let solid = make_primitive(&mut model, &params).unwrap();
        assert!(model.solids.is_alive(solid));
    }

    #[test]
    fn test_make_primitive_cone() {
        let mut model = BRepModel::new();
        let params = PrimitiveParams::Cone {
            radius1: 2.0,
            radius2: 0.5,
            height: 4.0,
        };
        let solid = make_primitive(&mut model, &params).unwrap();
        assert!(model.solids.is_alive(solid));
    }

    #[test]
    fn test_make_primitive_torus() {
        let mut model = BRepModel::new();
        let params = PrimitiveParams::Torus {
            major_radius: 5.0,
            minor_radius: 1.0,
        };
        let solid = make_primitive(&mut model, &params).unwrap();
        assert!(model.solids.is_alive(solid));
    }

    #[test]
    fn test_transformed_copy_identity() {
        let mut model = BRepModel::new();
        let b = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let copy = transformed_copy(&mut model, b.solid, Mat4::IDENTITY).unwrap();
        assert!(model.solids.is_alive(copy));
        assert_ne!(copy, b.solid);
    }

    #[test]
    fn test_transformed_copy_translation() {
        let mut model = BRepModel::new();
        let b = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let t = Mat4::from_rows(
            [1.0, 0.0, 0.0, 10.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        );
        let copy = transformed_copy(&mut model, b.solid, t).unwrap();
        assert!(model.solids.is_alive(copy));
    }
}
