use std::fmt;

/// Errors that can occur during CADKernel operations.
///
/// Every public API in the kernel returns [`KernelResult<T>`] instead of
/// panicking. The variants cover all major subsystems: topology handles,
/// parameter validation, geometry computation, B-Rep validation, and I/O.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelError {
    /// A handle does not refer to a live entity (e.g. the entity was deleted
    /// or belongs to a different model).
    InvalidHandle(&'static str),
    /// An operation received an invalid argument such as a negative radius,
    /// zero-length dimension, or out-of-range parameter.
    InvalidArgument(String),
    /// A B-Rep validation check failed (non-manifold topology, open shells,
    /// mismatched winding, etc.).
    ValidationFailed(String),
    /// The topology is not in the expected state (missing shell, dangling
    /// half-edge, loop with fewer than 3 edges, etc.).
    TopologyError(String),
    /// A geometric computation failed due to degenerate input (zero-length
    /// vector, singular matrix, coincident control points, etc.).
    GeometryError(String),
    /// An I/O operation failed (file not found, parse error, unsupported
    /// format version, write permission denied, etc.).
    IoError(String),
}

impl KernelError {
    /// Returns `true` if this error is due to an invalid or dead handle.
    pub fn is_invalid_handle(&self) -> bool {
        matches!(self, Self::InvalidHandle(_))
    }

    /// Returns `true` if this error is due to an invalid argument value.
    pub fn is_invalid_argument(&self) -> bool {
        matches!(self, Self::InvalidArgument(_))
    }

    /// Returns `true` if this error originated from the I/O subsystem.
    pub fn is_io_error(&self) -> bool {
        matches!(self, Self::IoError(_))
    }

    /// Wraps this error with additional context, prepending `context` to the
    /// existing message. Useful for adding caller-specific information.
    ///
    /// # Examples
    ///
    /// ```
    /// use cadkernel_core::KernelError;
    ///
    /// let err = KernelError::InvalidArgument("radius must be > 0".into());
    /// let wrapped = err.with_context("make_cylinder");
    /// assert!(wrapped.to_string().contains("make_cylinder"));
    /// ```
    pub fn with_context(self, context: &str) -> Self {
        match self {
            Self::InvalidHandle(entity) => Self::InvalidHandle(entity),
            Self::InvalidArgument(msg) => {
                Self::InvalidArgument(format!("{context}: {msg}"))
            }
            Self::ValidationFailed(msg) => {
                Self::ValidationFailed(format!("{context}: {msg}"))
            }
            Self::TopologyError(msg) => {
                Self::TopologyError(format!("{context}: {msg}"))
            }
            Self::GeometryError(msg) => {
                Self::GeometryError(format!("{context}: {msg}"))
            }
            Self::IoError(msg) => {
                Self::IoError(format!("{context}: {msg}"))
            }
        }
    }
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHandle(entity) => {
                write!(f, "invalid handle: {entity} (entity may have been deleted or belongs to another model)")
            }
            Self::InvalidArgument(msg) => write!(f, "invalid argument: {msg}"),
            Self::ValidationFailed(msg) => {
                write!(f, "validation failed: {msg}")
            }
            Self::TopologyError(msg) => write!(f, "topology error: {msg}"),
            Self::GeometryError(msg) => write!(f, "geometry error: {msg}"),
            Self::IoError(msg) => write!(f, "I/O error: {msg}"),
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
///
/// This is the standard return type for all fallible operations in the
/// CADKernel workspace.
pub type KernelResult<T> = Result<T, KernelError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_invalid_handle() {
        let err = KernelError::InvalidHandle("face");
        let msg = err.to_string();
        assert!(msg.contains("invalid handle"));
        assert!(msg.contains("face"));
        assert!(msg.contains("deleted"), "should mention possible cause");
    }

    #[test]
    fn test_display_invalid_argument() {
        let err = KernelError::InvalidArgument("radius must be > 0, got -5.0".into());
        let msg = err.to_string();
        assert!(msg.contains("invalid argument"));
        assert!(msg.contains("radius"));
        assert!(msg.contains("-5.0"));
    }

    #[test]
    fn test_display_io_error() {
        let err = KernelError::IoError("file not found".into());
        let msg = err.to_string();
        assert!(msg.contains("I/O error"), "should say I/O, got: {msg}");
    }

    #[test]
    fn test_with_context() {
        let err = KernelError::InvalidArgument("radius must be > 0".into());
        let wrapped = err.with_context("make_cylinder");
        let msg = wrapped.to_string();
        assert!(msg.contains("make_cylinder"), "context missing: {msg}");
        assert!(msg.contains("radius must be > 0"), "original missing: {msg}");
    }

    #[test]
    fn test_with_context_all_variants() {
        let cases: Vec<KernelError> = vec![
            KernelError::InvalidArgument("bad arg".into()),
            KernelError::ValidationFailed("not valid".into()),
            KernelError::TopologyError("broken topo".into()),
            KernelError::GeometryError("degenerate".into()),
            KernelError::IoError("disk full".into()),
        ];
        for err in cases {
            let wrapped = err.with_context("test_op");
            let msg = wrapped.to_string();
            assert!(msg.contains("test_op"), "context missing in: {msg}");
        }
    }

    #[test]
    fn test_with_context_invalid_handle_passthrough() {
        let err = KernelError::InvalidHandle("edge");
        let wrapped = err.with_context("some_op");
        // InvalidHandle uses &'static str, context is not prepended
        assert!(matches!(wrapped, KernelError::InvalidHandle("edge")));
    }

    #[test]
    fn test_is_predicates() {
        assert!(KernelError::InvalidHandle("v").is_invalid_handle());
        assert!(!KernelError::InvalidHandle("v").is_invalid_argument());
        assert!(!KernelError::InvalidHandle("v").is_io_error());

        assert!(KernelError::InvalidArgument("x".into()).is_invalid_argument());
        assert!(!KernelError::InvalidArgument("x".into()).is_invalid_handle());

        assert!(KernelError::IoError("x".into()).is_io_error());
        assert!(!KernelError::IoError("x".into()).is_invalid_handle());
    }

    #[test]
    fn test_from_std_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing file");
        let kernel_err: KernelError = io_err.into();
        assert!(kernel_err.is_io_error());
        assert!(kernel_err.to_string().contains("missing file"));
    }

    #[test]
    fn test_error_trait_impl() {
        let err = KernelError::GeometryError("zero vector".into());
        let std_err: &dyn std::error::Error = &err;
        assert!(std_err.to_string().contains("zero vector"));
    }
}
