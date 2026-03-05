//! Conversion between cadkernel-math types and nalgebra / glam types.

use crate::point::{Point2, Point3};
use crate::vector::{Vec2, Vec3, Vec4};

// ---------------------------------------------------------------------------
// nalgebra conversions (already on the types; re-export convenience From impls)
// ---------------------------------------------------------------------------

impl From<nalgebra::Point2<f64>> for Point2 {
    #[inline]
    fn from(p: nalgebra::Point2<f64>) -> Self {
        Self::new(p.x, p.y)
    }
}

impl From<Point2> for nalgebra::Point2<f64> {
    #[inline]
    fn from(p: Point2) -> Self {
        Self::new(p.x, p.y)
    }
}

impl From<nalgebra::Point3<f64>> for Point3 {
    #[inline]
    fn from(p: nalgebra::Point3<f64>) -> Self {
        Self::new(p.x, p.y, p.z)
    }
}

impl From<Point3> for nalgebra::Point3<f64> {
    #[inline]
    fn from(p: Point3) -> Self {
        Self::new(p.x, p.y, p.z)
    }
}

impl From<nalgebra::Vector3<f64>> for Vec3 {
    #[inline]
    fn from(v: nalgebra::Vector3<f64>) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

impl From<Vec3> for nalgebra::Vector3<f64> {
    #[inline]
    fn from(v: Vec3) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

// ---------------------------------------------------------------------------
// glam conversions
// ---------------------------------------------------------------------------

impl From<glam::DVec2> for Point2 {
    #[inline]
    fn from(v: glam::DVec2) -> Self {
        Self::new(v.x, v.y)
    }
}

impl From<Point2> for glam::DVec2 {
    #[inline]
    fn from(p: Point2) -> Self {
        Self::new(p.x, p.y)
    }
}

impl From<glam::DVec3> for Point3 {
    #[inline]
    fn from(v: glam::DVec3) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

impl From<Point3> for glam::DVec3 {
    #[inline]
    fn from(p: Point3) -> Self {
        Self::new(p.x, p.y, p.z)
    }
}

impl From<glam::DVec2> for Vec2 {
    #[inline]
    fn from(v: glam::DVec2) -> Self {
        Self::new(v.x, v.y)
    }
}

impl From<Vec2> for glam::DVec2 {
    #[inline]
    fn from(v: Vec2) -> Self {
        Self::new(v.x, v.y)
    }
}

impl From<glam::DVec3> for Vec3 {
    #[inline]
    fn from(v: glam::DVec3) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

impl From<Vec3> for glam::DVec3 {
    #[inline]
    fn from(v: Vec3) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

impl From<glam::DVec4> for Vec4 {
    #[inline]
    fn from(v: glam::DVec4) -> Self {
        Self::new(v.x, v.y, v.z, v.w)
    }
}

impl From<Vec4> for glam::DVec4 {
    #[inline]
    fn from(v: Vec4) -> Self {
        Self::new(v.x, v.y, v.z, v.w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point3_roundtrip_nalgebra() {
        let p = Point3::new(1.0, 2.0, 3.0);
        let na: nalgebra::Point3<f64> = p.into();
        let back: Point3 = na.into();
        assert!(p.approx_eq(back));
    }

    #[test]
    fn test_point3_roundtrip_glam() {
        let p = Point3::new(4.0, 5.0, 6.0);
        let g: glam::DVec3 = p.into();
        let back: Point3 = g.into();
        assert!(p.approx_eq(back));
    }

    #[test]
    fn test_vec3_roundtrip_nalgebra() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        let na: nalgebra::Vector3<f64> = v.into();
        let back: Vec3 = na.into();
        assert!(v.approx_eq(back));
    }
}
