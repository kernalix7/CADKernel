/// Default absolute tolerance for geometric comparisons.
///
/// CAD systems typically use tolerances in the 1e-7 to 1e-10 range.
/// We default to 1e-8 which balances precision with practical floating-point noise.
pub const EPSILON: f64 = 1e-8;

/// Returns the absolute difference between `a` and `b`.
///
/// Compare the result with [`EPSILON`] to check approximate equality:
/// `approx_eq(a, b) < EPSILON`.
#[inline]
pub fn approx_eq(a: f64, b: f64) -> f64 {
    (a - b).abs()
}

/// Returns `true` if `value` is effectively zero within [`EPSILON`].
#[inline]
pub fn is_zero(value: f64) -> bool {
    value.abs() < EPSILON
}

/// Returns `true` if `a` and `b` are within `tol` of each other.
#[inline]
pub fn approx_eq_tol(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() < tol
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approx_eq() {
        assert!(approx_eq(1.0, 1.0 + 1e-9) < EPSILON);
        assert!(approx_eq(1.0, 2.0) >= EPSILON);
    }

    #[test]
    fn test_is_zero() {
        assert!(is_zero(1e-9));
        assert!(!is_zero(1e-7));
    }
}
