use std::fmt;

/// Errors that can occur during CADKernel operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelError {
    /// A handle does not refer to a live entity.
    InvalidHandle(&'static str),
    /// An operation received an invalid argument.
    InvalidArgument(String),
    /// A B-Rep validation check failed.
    ValidationFailed(String),
    /// The topology is not in the expected state.
    TopologyError(String),
    /// A geometric computation failed (e.g. degenerate input).
    GeometryError(String),
    /// An I/O operation failed.
    IoError(String),
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHandle(entity) => write!(f, "invalid handle: {entity}"),
            Self::InvalidArgument(msg) => write!(f, "invalid argument: {msg}"),
            Self::ValidationFailed(msg) => write!(f, "validation failed: {msg}"),
            Self::TopologyError(msg) => write!(f, "topology error: {msg}"),
            Self::GeometryError(msg) => write!(f, "geometry error: {msg}"),
            Self::IoError(msg) => write!(f, "io error: {msg}"),
        }
    }
}

impl std::error::Error for KernelError {}

impl From<std::io::Error> for KernelError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e.to_string())
    }
}

/// Alias for `Result<T, KernelError>`.
pub type KernelResult<T> = Result<T, KernelError>;
