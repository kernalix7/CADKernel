use crate::matrix::Mat4;
use crate::point::Point3;
use crate::quaternion::Quaternion;
use crate::vector::Vec3;

/// An affine transformation in 3D space, stored as a 4x4 matrix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    mat: Mat4,
}

impl Transform {
    /// The identity (no-op) transform.
    pub const IDENTITY: Self = Self {
        mat: Mat4::IDENTITY,
    };

    /// Creates a transform from a raw 4x4 matrix.
    #[inline]
    pub fn from_mat4(mat: Mat4) -> Self {
        Self { mat }
    }

    /// Returns the underlying 4x4 matrix.
    #[inline]
    pub fn matrix(&self) -> Mat4 {
        self.mat
    }

    /// Translation by `(tx, ty, tz)`.
    pub fn translation(tx: f64, ty: f64, tz: f64) -> Self {
        let m = nalgebra::Matrix4::new(
            1.0, 0.0, 0.0, tx, //
            0.0, 1.0, 0.0, ty, //
            0.0, 0.0, 1.0, tz, //
            0.0, 0.0, 0.0, 1.0,
        );
        Self {
            mat: Mat4::from_nalgebra(m),
        }
    }

    /// Uniform scale by `s`.
    pub fn uniform_scale(s: f64) -> Self {
        Self::scale(s, s, s)
    }

    /// Non-uniform scale.
    pub fn scale(sx: f64, sy: f64, sz: f64) -> Self {
        let m = nalgebra::Matrix4::new(
            sx, 0.0, 0.0, 0.0, //
            0.0, sy, 0.0, 0.0, //
            0.0, 0.0, sz, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        );
        Self {
            mat: Mat4::from_nalgebra(m),
        }
    }

    /// Rotation about the X axis by `angle` radians.
    pub fn rotation_x(angle: f64) -> Self {
        let (s, c) = angle.sin_cos();
        let m = nalgebra::Matrix4::new(
            1.0, 0.0, 0.0, 0.0, //
            0.0, c, -s, 0.0, //
            0.0, s, c, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        );
        Self {
            mat: Mat4::from_nalgebra(m),
        }
    }

    /// Rotation about the Y axis by `angle` radians.
    pub fn rotation_y(angle: f64) -> Self {
        let (s, c) = angle.sin_cos();
        let m = nalgebra::Matrix4::new(
            c, 0.0, s, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            -s, 0.0, c, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        );
        Self {
            mat: Mat4::from_nalgebra(m),
        }
    }

    /// Rotation about the Z axis by `angle` radians.
    pub fn rotation_z(angle: f64) -> Self {
        let (s, c) = angle.sin_cos();
        let m = nalgebra::Matrix4::new(
            c, -s, 0.0, 0.0, //
            s, c, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        );
        Self {
            mat: Mat4::from_nalgebra(m),
        }
    }

    /// Rotation about an arbitrary axis by `angle` radians (Rodrigues).
    pub fn rotation_axis_angle(axis: Vec3, angle: f64) -> Self {
        let q = Quaternion::from_axis_angle(axis, angle);
        Self::from_quaternion(q)
    }

    /// Builds a rotation transform from a unit quaternion.
    pub fn from_quaternion(q: Quaternion) -> Self {
        let Quaternion { w, x, y, z } = q.normalized();
        let m = nalgebra::Matrix4::new(
            1.0 - 2.0 * (y * y + z * z),
            2.0 * (x * y - z * w),
            2.0 * (x * z + y * w),
            0.0,
            2.0 * (x * y + z * w),
            1.0 - 2.0 * (x * x + z * z),
            2.0 * (y * z - x * w),
            0.0,
            2.0 * (x * z - y * w),
            2.0 * (y * z + x * w),
            1.0 - 2.0 * (x * x + y * y),
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
        );
        Self {
            mat: Mat4::from_nalgebra(m),
        }
    }

    /// Rotation about an arbitrary `axis` passing through `center` by `angle` radians.
    pub fn rotation_around_point(center: Point3, axis: Vec3, angle: f64) -> Self {
        let to_origin = Self::translation(-center.x, -center.y, -center.z);
        let rot = Self::rotation_axis_angle(axis, angle);
        let back = Self::translation(center.x, center.y, center.z);
        to_origin.then(rot).then(back)
    }

    /// Mirrors about a plane defined by a point and normal.
    pub fn mirror(plane_point: Point3, plane_normal: Vec3) -> Self {
        let n = plane_normal.normalized().unwrap_or(Vec3::Z);
        let d = -(n.x * plane_point.x + n.y * plane_point.y + n.z * plane_point.z);
        let m = nalgebra::Matrix4::new(
            1.0 - 2.0 * n.x * n.x,
            -2.0 * n.x * n.y,
            -2.0 * n.x * n.z,
            -2.0 * n.x * d,
            -2.0 * n.y * n.x,
            1.0 - 2.0 * n.y * n.y,
            -2.0 * n.y * n.z,
            -2.0 * n.y * d,
            -2.0 * n.z * n.x,
            -2.0 * n.z * n.y,
            1.0 - 2.0 * n.z * n.z,
            -2.0 * n.z * d,
            0.0,
            0.0,
            0.0,
            1.0,
        );
        Self {
            mat: Mat4::from_nalgebra(m),
        }
    }

    /// Compose `self` followed by `other` (i.e. `other * self`).
    #[inline]
    pub fn then(self, other: Self) -> Self {
        Self {
            mat: other.mat * self.mat,
        }
    }

    /// Inverse transform, if it exists.
    #[inline]
    pub fn try_inverse(self) -> Option<Self> {
        self.mat.try_inverse().map(|m| Self { mat: m })
    }

    /// Apply the transform to a point.
    pub fn apply_point(&self, p: Point3) -> Point3 {
        let v = self.mat.0 * nalgebra::Vector4::new(p.x, p.y, p.z, 1.0);
        Point3::new(v.x, v.y, v.z)
    }

    /// Apply the transform to a direction vector (ignores translation).
    pub fn apply_vec(&self, v: Vec3) -> Vec3 {
        let r = self.mat.0 * nalgebra::Vector4::new(v.x, v.y, v.z, 0.0);
        Vec3::new(r.x, r.y, r.z)
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl std::fmt::Display for Transform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Transform({:?})", self.mat.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tolerance::EPSILON;
    use std::f64::consts::FRAC_PI_2;

    #[test]
    fn test_translation() {
        let t = Transform::translation(1.0, 2.0, 3.0);
        let p = t.apply_point(Point3::ORIGIN);
        assert!(p.approx_eq(Point3::new(1.0, 2.0, 3.0)));
    }

    #[test]
    fn test_rotation_z_90() {
        let t = Transform::rotation_z(FRAC_PI_2);
        let p = t.apply_point(Point3::new(1.0, 0.0, 0.0));
        assert!(p.approx_eq(Point3::new(0.0, 1.0, 0.0)));
    }

    #[test]
    fn test_compose_translate_then_scale() {
        let t = Transform::translation(1.0, 0.0, 0.0).then(Transform::uniform_scale(2.0));
        let p = t.apply_point(Point3::ORIGIN);
        assert!(p.approx_eq(Point3::new(2.0, 0.0, 0.0)));
    }

    #[test]
    fn test_inverse() {
        let t = Transform::translation(3.0, 4.0, 5.0);
        let inv = t.try_inverse().unwrap();
        let p = t.then(inv).apply_point(Point3::new(7.0, 8.0, 9.0));
        assert!(p.approx_eq(Point3::new(7.0, 8.0, 9.0)));
    }

    #[test]
    fn test_apply_vec_ignores_translation() {
        let t = Transform::translation(100.0, 200.0, 300.0);
        let v = t.apply_vec(Vec3::X);
        assert!((v.length() - 1.0).abs() < EPSILON);
        assert!(v.approx_eq(Vec3::X));
    }

    #[test]
    fn test_rotation_axis_angle() {
        let t = Transform::rotation_axis_angle(Vec3::Z, FRAC_PI_2);
        let p = t.apply_point(Point3::new(1.0, 0.0, 0.0));
        assert!(p.approx_eq(Point3::new(0.0, 1.0, 0.0)));
    }

    #[test]
    fn test_rotation_around_point() {
        let center = Point3::new(1.0, 0.0, 0.0);
        let t = Transform::rotation_around_point(center, Vec3::Z, FRAC_PI_2);
        let p = t.apply_point(Point3::new(2.0, 0.0, 0.0));
        assert!(p.approx_eq(Point3::new(1.0, 1.0, 0.0)));
    }

    #[test]
    fn test_mirror_xy_plane() {
        let t = Transform::mirror(Point3::ORIGIN, Vec3::Z);
        let p = t.apply_point(Point3::new(1.0, 2.0, 3.0));
        assert!(p.approx_eq(Point3::new(1.0, 2.0, -3.0)));
    }
}
