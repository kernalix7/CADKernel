use crate::point::Point3;
use crate::vector::Vec3;

/// A ray in 3D space: origin + normalised direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray3 {
    pub origin: Point3,
    pub direction: Vec3,
}

impl Ray3 {
    /// Creates a new ray. `direction` is normalised internally.
    pub fn new(origin: Point3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalized().unwrap_or(Vec3::Z),
        }
    }

    /// Returns the point at parameter `t` along the ray: `origin + t * direction`.
    #[inline]
    pub fn at(&self, t: f64) -> Point3 {
        self.origin + self.direction * t
    }

    /// Projects a point onto the ray, returning the parameter `t`.
    /// Negative values indicate the point is behind the origin.
    pub fn project(&self, point: Point3) -> f64 {
        let v = point - self.origin;
        v.dot(self.direction)
    }

    /// Returns the closest point on the ray to the given point.
    /// Clamps `t` to `[0, ∞)` so the result lies on the ray (not the backing line).
    pub fn closest_point(&self, point: Point3) -> Point3 {
        let t = self.project(point).max(0.0);
        self.at(t)
    }

    /// Distance from a point to the infinite line defined by this ray.
    pub fn distance_to_point(&self, point: Point3) -> f64 {
        let v = point - self.origin;
        let projected = self.direction * v.dot(self.direction);
        (v - projected).length()
    }
}

impl std::fmt::Display for Ray3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ray3({} -> {})", self.origin, self.direction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tolerance::EPSILON;

    #[test]
    fn test_ray_at() {
        let r = Ray3::new(Point3::ORIGIN, Vec3::X);
        let p = r.at(5.0);
        assert!(p.approx_eq(Point3::new(5.0, 0.0, 0.0)));
    }

    #[test]
    fn test_ray_project() {
        let r = Ray3::new(Point3::ORIGIN, Vec3::X);
        let t = r.project(Point3::new(3.0, 4.0, 0.0));
        assert!((t - 3.0).abs() < EPSILON);
    }

    #[test]
    fn test_ray_closest_point() {
        let r = Ray3::new(Point3::ORIGIN, Vec3::X);
        let cp = r.closest_point(Point3::new(3.0, 4.0, 0.0));
        assert!(cp.approx_eq(Point3::new(3.0, 0.0, 0.0)));
    }

    #[test]
    fn test_ray_distance_to_point() {
        let r = Ray3::new(Point3::ORIGIN, Vec3::X);
        let d = r.distance_to_point(Point3::new(3.0, 4.0, 0.0));
        assert!((d - 4.0).abs() < EPSILON);
    }

    #[test]
    fn test_ray_behind_origin() {
        let r = Ray3::new(Point3::new(5.0, 0.0, 0.0), Vec3::X);
        let cp = r.closest_point(Point3::ORIGIN);
        assert!(cp.approx_eq(Point3::new(5.0, 0.0, 0.0)));
    }
}
