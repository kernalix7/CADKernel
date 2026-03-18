use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{BoundingBox, EPSILON, Point3, Vec3};

use super::Surface;

/// Fallback domain bound used when an infinite domain would produce NaN
/// in sampling-based algorithms (bounding_box, project_point).
const FINITE_FALLBACK: f64 = 1e6;

/// An infinite plane defined by origin, u-axis, and v-axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane {
    pub origin: Point3,
    pub u_axis: Vec3,
    pub v_axis: Vec3,
    normal: Vec3,
}

impl Plane {
    /// Creates a plane from an origin point and two tangent vectors.
    ///
    /// Returns `KernelError::InvalidArgument` if `u_axis` and `v_axis` are parallel.
    pub fn new(origin: Point3, u_axis: Vec3, v_axis: Vec3) -> KernelResult<Self> {
        let normal = u_axis.cross(v_axis).normalized().ok_or_else(|| {
            KernelError::InvalidArgument("plane axes must not be parallel".into())
        })?;
        Ok(Self {
            origin,
            u_axis,
            v_axis,
            normal,
        })
    }

    /// XY-plane at the origin.
    pub fn xy() -> KernelResult<Self> {
        Self::new(Point3::ORIGIN, Vec3::X, Vec3::Y)
    }

    /// XZ-plane at the origin.
    pub fn xz() -> KernelResult<Self> {
        Self::new(Point3::ORIGIN, Vec3::X, Vec3::Z)
    }

    /// YZ-plane at the origin.
    pub fn yz() -> KernelResult<Self> {
        Self::new(Point3::ORIGIN, Vec3::Y, Vec3::Z)
    }

    /// Returns the unit normal of the plane.
    pub fn normal(&self) -> Vec3 {
        self.normal
    }

    /// Creates a plane from three non-collinear points.
    /// `u_axis = p1 - p0`, `v_axis = p2 - p0`.
    pub fn from_three_points(p0: Point3, p1: Point3, p2: Point3) -> KernelResult<Self> {
        Self::new(p0, p1 - p0, p2 - p0)
    }

    /// Signed distance from a point to this plane.
    /// Positive = same side as normal, negative = opposite side.
    pub fn signed_distance(&self, point: Point3) -> f64 {
        (point - self.origin).dot(self.normal)
    }

    /// Absolute distance from a point to this plane.
    pub fn distance(&self, point: Point3) -> f64 {
        self.signed_distance(point).abs()
    }

    /// Projects a point onto this plane (closest point).
    pub fn project_point(&self, point: Point3) -> Point3 {
        point - self.normal * self.signed_distance(point)
    }

    /// Returns true if the point is on the positive side of the plane.
    pub fn is_above(&self, point: Point3) -> bool {
        self.signed_distance(point) > 0.0
    }

    /// Returns true if the point lies on the plane (within tolerance).
    pub fn contains_point(&self, point: Point3) -> bool {
        self.distance(point) < EPSILON
    }
}

impl Surface for Plane {
    fn point_at(&self, u: f64, v: f64) -> Point3 {
        self.origin + self.u_axis * u + self.v_axis * v
    }

    fn normal_at(&self, _u: f64, _v: f64) -> Vec3 {
        self.normal
    }

    fn domain_u(&self) -> (f64, f64) {
        (f64::NEG_INFINITY, f64::INFINITY)
    }

    fn domain_v(&self) -> (f64, f64) {
        (f64::NEG_INFINITY, f64::INFINITY)
    }

    /// Analytical projection onto an infinite plane.
    fn project_point(&self, point: Point3) -> (f64, f64, Point3) {
        let closest = Plane::project_point(self, point);
        let d = closest - self.origin;
        let u = d.dot(self.u_axis) / self.u_axis.dot(self.u_axis).max(1e-14);
        let v = d.dot(self.v_axis) / self.v_axis.dot(self.v_axis).max(1e-14);
        (u, v, closest)
    }

    /// Bounding box for an infinite plane — uses a large finite fallback domain.
    fn bounding_box(&self) -> BoundingBox {
        let corners = [
            self.point_at(-FINITE_FALLBACK, -FINITE_FALLBACK),
            self.point_at(FINITE_FALLBACK, -FINITE_FALLBACK),
            self.point_at(-FINITE_FALLBACK, FINITE_FALLBACK),
            self.point_at(FINITE_FALLBACK, FINITE_FALLBACK),
        ];
        let mut bb = BoundingBox::new(corners[0], corners[0]);
        for &c in &corners[1..] {
            bb.include_point(c);
        }
        bb
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xy_plane_normal() {
        let p = Plane::xy().unwrap();
        assert!(p.normal().approx_eq(Vec3::Z));
    }

    #[test]
    fn test_plane_point_at() {
        let p = Plane::xy().unwrap();
        assert!(p.point_at(1.0, 2.0).approx_eq(Point3::new(1.0, 2.0, 0.0)));
    }

    #[test]
    fn test_from_three_points() {
        let p = Plane::from_three_points(
            Point3::ORIGIN,
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        )
        .unwrap();
        assert!(p.normal().approx_eq(Vec3::Z));
    }

    #[test]
    fn test_signed_distance() {
        let p = Plane::xy().unwrap();
        let above = Point3::new(0.0, 0.0, 3.0);
        let below = Point3::new(0.0, 0.0, -2.0);
        assert!((p.signed_distance(above) - 3.0).abs() < EPSILON);
        assert!((p.signed_distance(below) - (-2.0)).abs() < EPSILON);
    }

    #[test]
    fn test_project_point() {
        let p = Plane::xy().unwrap();
        let proj = p.project_point(Point3::new(1.0, 2.0, 5.0));
        assert!(proj.approx_eq(Point3::new(1.0, 2.0, 0.0)));
    }

    #[test]
    fn test_contains_point() {
        let p = Plane::xy().unwrap();
        assert!(p.contains_point(Point3::new(5.0, 3.0, 0.0)));
        assert!(!p.contains_point(Point3::new(5.0, 3.0, 1.0)));
    }
}
