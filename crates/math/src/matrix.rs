use nalgebra::{Matrix3 as NaMat3, Matrix4 as NaMat4};

/// A 3x3 matrix (rotations, 2D transforms).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat3(pub NaMat3<f64>);

/// A 4x4 matrix (full 3D affine/projective transforms).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4(pub NaMat4<f64>);

impl Mat3 {
    /// The 3x3 identity matrix.
    pub const IDENTITY: Self = Self(NaMat3::new(
        1.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, //
        0.0, 0.0, 1.0,
    ));

    /// Wraps an existing `nalgebra::Matrix3<f64>`.
    #[inline]
    pub fn from_nalgebra(m: NaMat3<f64>) -> Self {
        Self(m)
    }

    /// Returns the determinant of the matrix.
    #[inline]
    pub fn determinant(self) -> f64 {
        self.0.determinant()
    }

    /// Returns the inverse, or `None` if the matrix is singular.
    #[inline]
    pub fn try_inverse(self) -> Option<Self> {
        self.0.try_inverse().map(Self)
    }
}

impl std::ops::Mul for Mat3 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self(self.0 * rhs.0)
    }
}

impl Mat4 {
    /// The 4x4 identity matrix.
    pub const IDENTITY: Self = Self(NaMat4::new(
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ));

    /// Wraps an existing `nalgebra::Matrix4<f64>`.
    #[inline]
    pub fn from_nalgebra(m: NaMat4<f64>) -> Self {
        Self(m)
    }

    /// Returns the determinant of the matrix.
    #[inline]
    pub fn determinant(self) -> f64 {
        self.0.determinant()
    }

    /// Returns the inverse, or `None` if the matrix is singular.
    #[inline]
    pub fn try_inverse(self) -> Option<Self> {
        self.0.try_inverse().map(Self)
    }

    /// Creates a 4x4 matrix from four row arrays.
    #[inline]
    pub fn from_rows(r0: [f64; 4], r1: [f64; 4], r2: [f64; 4], r3: [f64; 4]) -> Self {
        Self(NaMat4::new(
            r0[0], r0[1], r0[2], r0[3],
            r1[0], r1[1], r1[2], r1[3],
            r2[0], r2[1], r2[2], r2[3],
            r3[0], r3[1], r3[2], r3[3],
        ))
    }

    /// Creates a translation matrix from a vector.
    #[inline]
    pub fn translation(v: crate::Vec3) -> Self {
        Self(NaMat4::new(
            1.0, 0.0, 0.0, v.x, //
            0.0, 1.0, 0.0, v.y, //
            0.0, 0.0, 1.0, v.z, //
            0.0, 0.0, 0.0, 1.0,
        ))
    }

    /// Transforms a 3D point by this matrix (w=1 homogeneous coordinate).
    #[inline]
    pub fn transform_point(self, p: crate::Point3) -> crate::Point3 {
        let m = &self.0;
        let x = m[(0, 0)] * p.x + m[(0, 1)] * p.y + m[(0, 2)] * p.z + m[(0, 3)];
        let y = m[(1, 0)] * p.x + m[(1, 1)] * p.y + m[(1, 2)] * p.z + m[(1, 3)];
        let z = m[(2, 0)] * p.x + m[(2, 1)] * p.y + m[(2, 2)] * p.z + m[(2, 3)];
        let w = m[(3, 0)] * p.x + m[(3, 1)] * p.y + m[(3, 2)] * p.z + m[(3, 3)];
        if (w - 1.0).abs() < 1e-14 {
            crate::Point3::new(x, y, z)
        } else {
            crate::Point3::new(x / w, y / w, z / w)
        }
    }
}

impl std::ops::Mul for Mat4 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self(self.0 * rhs.0)
    }
}

impl Default for Mat3 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Default for Mat4 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tolerance::EPSILON;

    #[test]
    fn test_mat4_identity_inverse() {
        let inv = Mat4::IDENTITY.try_inverse().unwrap();
        assert!((inv.determinant() - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_mat3_multiply_identity() {
        let m = Mat3::IDENTITY * Mat3::IDENTITY;
        assert!((m.determinant() - 1.0).abs() < EPSILON);
    }
}
