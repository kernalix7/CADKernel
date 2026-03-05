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
