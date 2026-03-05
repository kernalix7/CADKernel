//! Core error types for the CAD kernel.
//!
//! This crate defines [`KernelError`] and the [`KernelResult`] type alias used
//! throughout every other crate in the workspace.

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
