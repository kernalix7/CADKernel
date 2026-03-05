use std::fmt;
use std::ops::{Add, AddAssign, Sub, SubAssign};

use serde::{Deserialize, Serialize};

use crate::tolerance::EPSILON;
use crate::vector::{Vec2, Vec3};

/// A 2D point in Euclidean space.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point2 {
    pub x: f64,
    pub y: f64,
}

/// A 3D point in Euclidean space.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

// ---------------------------------------------------------------------------
// Point2
// ---------------------------------------------------------------------------

impl Point2 {
    /// The origin `(0, 0)`.
    pub const ORIGIN: Self = Self { x: 0.0, y: 0.0 };

    /// Creates a new 2D point.
    #[inline]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Euclidean distance to `other`.
    #[inline]
    pub fn distance_to(self, other: Self) -> f64 {
        (other - self).length()
    }

    /// Returns the point halfway between `self` and `other`.
    #[inline]
    pub fn midpoint(self, other: Self) -> Self {
        Self::new((self.x + other.x) * 0.5, (self.y + other.y) * 0.5)
    }

    /// Returns `true` if the two points coincide within [`EPSILON`] (Euclidean distance).
    #[inline]
    pub fn approx_eq(self, other: Self) -> bool {
        (other - self).length() < EPSILON
    }

    /// Converts to a `nalgebra::Point2<f64>`.
    #[inline]
    pub fn to_nalgebra(self) -> nalgebra::Point2<f64> {
        nalgebra::Point2::new(self.x, self.y)
    }

    /// Creates a [`Point2`] from a `nalgebra::Point2<f64>`.
    #[inline]
    pub fn from_nalgebra(p: nalgebra::Point2<f64>) -> Self {
        Self::new(p.x, p.y)
    }
}

impl Add<Vec2> for Point2 {
    type Output = Self;
    #[inline]
    fn add(self, v: Vec2) -> Self {
        Self::new(self.x + v.x, self.y + v.y)
    }
}

impl Sub for Point2 {
    type Output = Vec2;
    #[inline]
    fn sub(self, rhs: Self) -> Vec2 {
        Vec2::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Sub<Vec2> for Point2 {
    type Output = Self;
    #[inline]
    fn sub(self, v: Vec2) -> Self {
        Self::new(self.x - v.x, self.y - v.y)
    }
}

impl AddAssign<Vec2> for Point2 {
    #[inline]
    fn add_assign(&mut self, v: Vec2) {
        self.x += v.x;
        self.y += v.y;
    }
}

impl SubAssign<Vec2> for Point2 {
    #[inline]
    fn sub_assign(&mut self, v: Vec2) {
        self.x -= v.x;
        self.y -= v.y;
    }
}

impl Default for Point2 {
    fn default() -> Self {
        Self::ORIGIN
    }
}

impl fmt::Display for Point2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl From<Vec2> for Point2 {
    #[inline]
    fn from(v: Vec2) -> Self {
        Self::new(v.x, v.y)
    }
}

impl From<[f64; 2]> for Point2 {
    #[inline]
    fn from([x, y]: [f64; 2]) -> Self {
        Self::new(x, y)
    }
}

impl From<(f64, f64)> for Point2 {
    #[inline]
    fn from((x, y): (f64, f64)) -> Self {
        Self::new(x, y)
    }
}

// ---------------------------------------------------------------------------
// Point3
// ---------------------------------------------------------------------------

impl Point3 {
    /// The origin `(0, 0, 0)`.
    pub const ORIGIN: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    /// Creates a new 3D point.
    #[inline]
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Euclidean distance to `other`.
    #[inline]
    pub fn distance_to(self, other: Self) -> f64 {
        (other - self).length()
    }

    /// Returns the point halfway between `self` and `other`.
    #[inline]
    pub fn midpoint(self, other: Self) -> Self {
        Self::new(
            (self.x + other.x) * 0.5,
            (self.y + other.y) * 0.5,
            (self.z + other.z) * 0.5,
        )
    }

    /// Returns `true` if the two points coincide within [`EPSILON`] (Euclidean distance).
    #[inline]
    pub fn approx_eq(self, other: Self) -> bool {
        (other - self).length() < EPSILON
    }

    /// Converts to a `nalgebra::Point3<f64>`.
    #[inline]
    pub fn to_nalgebra(self) -> nalgebra::Point3<f64> {
        nalgebra::Point3::new(self.x, self.y, self.z)
    }

    /// Creates a [`Point3`] from a `nalgebra::Point3<f64>`.
    #[inline]
    pub fn from_nalgebra(p: nalgebra::Point3<f64>) -> Self {
        Self::new(p.x, p.y, p.z)
    }
}

impl Add<Vec3> for Point3 {
    type Output = Self;
    #[inline]
    fn add(self, v: Vec3) -> Self {
        Self::new(self.x + v.x, self.y + v.y, self.z + v.z)
    }
}

impl Sub for Point3 {
    type Output = Vec3;
    #[inline]
    fn sub(self, rhs: Self) -> Vec3 {
        Vec3::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Sub<Vec3> for Point3 {
    type Output = Self;
    #[inline]
    fn sub(self, v: Vec3) -> Self {
        Self::new(self.x - v.x, self.y - v.y, self.z - v.z)
    }
}

impl AddAssign<Vec3> for Point3 {
    #[inline]
    fn add_assign(&mut self, v: Vec3) {
        self.x += v.x;
        self.y += v.y;
        self.z += v.z;
    }
}

impl SubAssign<Vec3> for Point3 {
    #[inline]
    fn sub_assign(&mut self, v: Vec3) {
        self.x -= v.x;
        self.y -= v.y;
        self.z -= v.z;
    }
}

impl Default for Point3 {
    fn default() -> Self {
        Self::ORIGIN
    }
}

impl fmt::Display for Point3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

impl From<Vec3> for Point3 {
    #[inline]
    fn from(v: Vec3) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

impl From<Point3> for Vec3 {
    #[inline]
    fn from(p: Point3) -> Self {
        Self::new(p.x, p.y, p.z)
    }
}

impl From<Point2> for Vec2 {
    #[inline]
    fn from(p: Point2) -> Self {
        Self::new(p.x, p.y)
    }
}

impl From<[f64; 3]> for Point3 {
    #[inline]
    fn from([x, y, z]: [f64; 3]) -> Self {
        Self::new(x, y, z)
    }
}

impl From<(f64, f64, f64)> for Point3 {
    #[inline]
    fn from((x, y, z): (f64, f64, f64)) -> Self {
        Self::new(x, y, z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point3_distance() {
        let a = Point3::new(1.0, 0.0, 0.0);
        let b = Point3::new(4.0, 0.0, 0.0);
        assert!((a.distance_to(b) - 3.0).abs() < EPSILON);
    }

    #[test]
    fn test_point3_midpoint() {
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(2.0, 4.0, 6.0);
        let m = a.midpoint(b);
        assert!(m.approx_eq(Point3::new(1.0, 2.0, 3.0)));
    }

    #[test]
    fn test_point3_sub_gives_vec3() {
        let a = Point3::new(3.0, 2.0, 1.0);
        let b = Point3::new(1.0, 1.0, 1.0);
        let v = a - b;
        assert!((v.x - 2.0).abs() < EPSILON);
        assert!((v.y - 1.0).abs() < EPSILON);
        assert!((v.z - 0.0).abs() < EPSILON);
    }
}
