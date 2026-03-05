use crate::tolerance::is_zero;
use crate::vector::Vec3;

/// A unit quaternion for representing 3D rotations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quaternion {
    pub w: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Quaternion {
    /// The identity quaternion (no rotation).
    pub const IDENTITY: Self = Self {
        w: 1.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    /// Creates a quaternion from raw components `(w, x, y, z)`.
    #[inline]
    pub const fn new(w: f64, x: f64, y: f64, z: f64) -> Self {
        Self { w, x, y, z }
    }

    /// Creates a quaternion from an axis-angle rotation.
    /// `axis` will be normalised internally; `angle` is in radians.
    pub fn from_axis_angle(axis: Vec3, angle: f64) -> Self {
        let n = axis.normalized().unwrap_or(Vec3::Z);
        let half = angle * 0.5;
        let (s, c) = half.sin_cos();
        Self {
            w: c,
            x: n.x * s,
            y: n.y * s,
            z: n.z * s,
        }
    }

    /// Decomposes the quaternion back to axis and angle.
    pub fn to_axis_angle(self) -> (Vec3, f64) {
        let angle = 2.0 * self.w.clamp(-1.0, 1.0).acos();
        let s = (1.0 - self.w * self.w).sqrt();
        if is_zero(s) {
            (Vec3::Z, 0.0)
        } else {
            (Vec3::new(self.x / s, self.y / s, self.z / s), angle)
        }
    }

    /// Returns the L2 norm of the quaternion.
    #[inline]
    pub fn norm(self) -> f64 {
        (self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// Returns the normalised (unit) quaternion.
    pub fn normalized(self) -> Self {
        let n = self.norm();
        if is_zero(n) {
            Self::IDENTITY
        } else {
            Self {
                w: self.w / n,
                x: self.x / n,
                y: self.y / n,
                z: self.z / n,
            }
        }
    }

    /// The conjugate (inverse for unit quaternions).
    #[inline]
    pub fn conjugate(self) -> Self {
        Self {
            w: self.w,
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }

    /// Rotates a vector by this quaternion: q * v * q⁻¹.
    pub fn rotate_vec(self, v: Vec3) -> Vec3 {
        let qv = Self::new(0.0, v.x, v.y, v.z);
        let result = (self * qv) * self.conjugate();
        Vec3::new(result.x, result.y, result.z)
    }
}

impl Default for Quaternion {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl std::fmt::Display for Quaternion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({} + {}i + {}j + {}k)", self.w, self.x, self.y, self.z)
    }
}

impl std::ops::Mul for Quaternion {
    type Output = Self;

    /// Hamilton product of two quaternions.
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self {
            w: self.w * rhs.w - self.x * rhs.x - self.y * rhs.y - self.z * rhs.z,
            x: self.w * rhs.x + self.x * rhs.w + self.y * rhs.z - self.z * rhs.y,
            y: self.w * rhs.y - self.x * rhs.z + self.y * rhs.w + self.z * rhs.x,
            z: self.w * rhs.z + self.x * rhs.y - self.y * rhs.x + self.z * rhs.w,
        }
    }
}

impl Quaternion {
    /// Spherical linear interpolation from `self` to `other` at parameter `t`.
    pub fn slerp(self, other: Self, t: f64) -> Self {
        let mut dot = self.w * other.w + self.x * other.x + self.y * other.y + self.z * other.z;
        let mut b = other;
        if dot < 0.0 {
            dot = -dot;
            b = Self::new(-b.w, -b.x, -b.y, -b.z);
        }
        if dot > 0.9995 {
            // Lerp fallback for near-identical quaternions
            return Self::new(
                self.w + t * (b.w - self.w),
                self.x + t * (b.x - self.x),
                self.y + t * (b.y - self.y),
                self.z + t * (b.z - self.z),
            )
            .normalized();
        }
        let theta = dot.clamp(-1.0, 1.0).acos();
        let sin_theta = theta.sin();
        let wa = ((1.0 - t) * theta).sin() / sin_theta;
        let wb = (t * theta).sin() / sin_theta;
        Self::new(
            wa * self.w + wb * b.w,
            wa * self.x + wb * b.x,
            wa * self.y + wb * b.y,
            wa * self.z + wb * b.z,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tolerance::EPSILON;
    use std::f64::consts::FRAC_PI_2;

    #[test]
    fn test_axis_angle_roundtrip() {
        let q = Quaternion::from_axis_angle(Vec3::Z, FRAC_PI_2);
        let (axis, angle) = q.to_axis_angle();
        assert!((angle - FRAC_PI_2).abs() < 1e-10);
        assert!(axis.approx_eq(Vec3::Z));
    }

    #[test]
    fn test_rotate_vec_90deg_z() {
        let q = Quaternion::from_axis_angle(Vec3::Z, FRAC_PI_2);
        let v = q.rotate_vec(Vec3::X);
        assert!((v.x).abs() < EPSILON);
        assert!((v.y - 1.0).abs() < EPSILON);
        assert!((v.z).abs() < EPSILON);
    }

    #[test]
    fn test_identity_rotation() {
        let q = Quaternion::IDENTITY;
        let v = q.rotate_vec(Vec3::new(1.0, 2.0, 3.0));
        assert!(v.approx_eq(Vec3::new(1.0, 2.0, 3.0)));
    }

    #[test]
    fn test_slerp_endpoints() {
        let a = Quaternion::IDENTITY;
        let b = Quaternion::from_axis_angle(Vec3::Z, FRAC_PI_2);
        let s0 = a.slerp(b, 0.0);
        let s1 = a.slerp(b, 1.0);
        assert!((s0.w - a.w).abs() < 1e-10);
        assert!((s1.w - b.w).abs() < 1e-10);
    }

    #[test]
    fn test_conjugate_inverse() {
        let q = Quaternion::from_axis_angle(Vec3::new(1.0, 1.0, 0.0), 1.0);
        let prod = q * q.conjugate();
        assert!((prod.w - 1.0).abs() < 1e-10);
        assert!(prod.x.abs() < 1e-10);
    }
}
