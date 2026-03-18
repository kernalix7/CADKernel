//! A trimmed surface that restricts a base surface to regions defined by UV trim loops.

use std::sync::Arc;

use cadkernel_math::{Point3, Vec3};

use super::Surface;
use super::parametric_wire::ParametricWire2D;
use crate::curve::curve2d::Curve2D;

/// A surface with trim boundaries defined in UV parameter space.
///
/// The outer trim loop defines the exterior boundary.
/// Inner trim loops (holes) define excluded regions.
pub struct TrimmedSurface {
    /// The underlying parametric surface.
    pub base: Arc<dyn Surface>,
    /// Outer boundary loop in UV space (counter-clockwise).
    pub outer: ParametricWire2D,
    /// Inner boundary loops (holes) in UV space (clockwise).
    pub holes: Vec<ParametricWire2D>,
}

impl TrimmedSurface {
    /// Creates a new trimmed surface from a base surface and ParametricWire2D loops.
    pub fn new(
        base: Arc<dyn Surface>,
        outer: ParametricWire2D,
        holes: Vec<ParametricWire2D>,
    ) -> Self {
        Self { base, outer, holes }
    }

    /// Creates a trimmed surface from raw curve vectors (convenience constructor).
    pub fn from_curves(
        base: Arc<dyn Surface>,
        outer: Vec<Arc<dyn Curve2D>>,
        holes: Vec<Vec<Arc<dyn Curve2D>>>,
    ) -> Self {
        let outer_wire = ParametricWire2D::closed(outer);
        let hole_wires = holes
            .into_iter()
            .map(ParametricWire2D::closed)
            .collect();
        Self::new(base, outer_wire, hole_wires)
    }

    /// Tests whether a UV point is inside the trimmed region.
    pub fn contains_uv(&self, u: f64, v: f64) -> bool {
        if !self.outer.contains_point(u, v) {
            return false;
        }
        for hole in &self.holes {
            if hole.contains_point(u, v) {
                return false;
            }
        }
        true
    }
}

impl Surface for TrimmedSurface {
    fn point_at(&self, u: f64, v: f64) -> Point3 {
        self.base.point_at(u, v)
    }

    fn normal_at(&self, u: f64, v: f64) -> Vec3 {
        self.base.normal_at(u, v)
    }

    fn domain_u(&self) -> (f64, f64) {
        self.base.domain_u()
    }

    fn domain_v(&self) -> (f64, f64) {
        self.base.domain_v()
    }

    fn du(&self, u: f64, v: f64) -> Vec3 {
        self.base.du(u, v)
    }

    fn dv(&self, u: f64, v: f64) -> Vec3 {
        self.base.dv(u, v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_math::Point2;
    use crate::curve::curve2d::Line2D;
    use crate::surface::plane::Plane;

    fn square_loop() -> Vec<Arc<dyn Curve2D>> {
        vec![
            Arc::new(Line2D::new(Point2::new(0.0, 0.0), Point2::new(1.0, 0.0))),
            Arc::new(Line2D::new(Point2::new(1.0, 0.0), Point2::new(1.0, 1.0))),
            Arc::new(Line2D::new(Point2::new(1.0, 1.0), Point2::new(0.0, 1.0))),
            Arc::new(Line2D::new(Point2::new(0.0, 1.0), Point2::new(0.0, 0.0))),
        ]
    }

    #[test]
    fn test_trimmed_plane() {
        let plane = Arc::new(Plane::xy().unwrap());
        let trimmed = TrimmedSurface::from_curves(plane, square_loop(), vec![]);

        assert!(trimmed.contains_uv(0.5, 0.5));
        assert!(trimmed.contains_uv(0.1, 0.1));
        assert!(trimmed.contains_uv(0.9, 0.9));

        assert!(!trimmed.contains_uv(-0.1, 0.5));
        assert!(!trimmed.contains_uv(0.5, 1.5));
        assert!(!trimmed.contains_uv(2.0, 2.0));

        let p = trimmed.point_at(0.5, 0.5);
        assert!(p.distance_to(Point3::new(0.5, 0.5, 0.0)) < 1e-10);
    }

    #[test]
    fn test_trimmed_with_hole() {
        let plane = Arc::new(Plane::xy().unwrap());

        let hole = vec![
            Arc::new(Line2D::new(Point2::new(0.3, 0.3), Point2::new(0.7, 0.3))) as Arc<dyn Curve2D>,
            Arc::new(Line2D::new(Point2::new(0.7, 0.3), Point2::new(0.7, 0.7))),
            Arc::new(Line2D::new(Point2::new(0.7, 0.7), Point2::new(0.3, 0.7))),
            Arc::new(Line2D::new(Point2::new(0.3, 0.7), Point2::new(0.3, 0.3))),
        ];

        let trimmed = TrimmedSurface::from_curves(plane, square_loop(), vec![hole]);

        assert!(trimmed.contains_uv(0.1, 0.1));
        assert!(trimmed.contains_uv(0.9, 0.9));

        assert!(!trimmed.contains_uv(0.5, 0.5));
        assert!(!trimmed.contains_uv(0.4, 0.4));

        assert!(!trimmed.contains_uv(-0.1, 0.5));
    }
}
