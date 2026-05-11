//! Core error types for the CAD kernel.
//!
//! This crate defines [`KernelError`] and the [`KernelResult`] type alias used
//! throughout every other crate in the workspace. All public APIs in CADKernel
//! return `KernelResult<T>` rather than panicking, ensuring that callers can
//! handle failures gracefully.
//!
//! # Examples
//!
//! ```
//! use cadkernel_core::{KernelError, KernelResult};
//!
//! fn validate_radius(r: f64) -> KernelResult<()> {
//!     if r <= 0.0 {
//!         return Err(KernelError::InvalidArgument("radius must be positive".into()));
//!     }
//!     Ok(())
//! }
//!
//! assert!(validate_radius(1.0).is_ok());
//! assert!(validate_radius(-1.0).is_err());
//! ```

// Commercial CAD Roadmap v0.5 Gate 12: the core error crate is the root of the
// `KernelResult<T>`-first contract. Production code here must never panic.
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod error;

pub use error::{KernelError, KernelResult};

#[cfg(test)]
mod thread_safety_tests {
    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn core_types_are_send_sync() {
        assert_send_sync::<crate::KernelError>();
    }
}
