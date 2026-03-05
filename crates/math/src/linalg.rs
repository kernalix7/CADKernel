//! Re-exports of dynamic linear algebra types from `nalgebra`.
//!
//! Downstream crates that need dense matrix operations (e.g. solvers) should
//! import from here instead of depending on `nalgebra` directly.

pub use nalgebra::DMatrix;
pub use nalgebra::DVector;
pub use nalgebra::LU;
