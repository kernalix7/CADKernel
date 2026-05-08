//! # cadkernel-api
//!
//! Stable public API surface for CADKernel. This crate is the **single
//! sanctioned entry point** for every non-GUI consumer:
//!
//! - AI agents (via the MCP server which delegates to [`Session`]).
//! - Lua and Python scripting.
//! - Integration tests and CI harnesses.
//! - External tools and downstream embedders.
//!
//! See `docs/COMMERCIAL_CAD_ROADMAP.md` for the long-term plan that this
//! crate operationalises (Phase 1).
//!
//! ## Three primary types
//!
//! - [`Document`] — the model container (currently a list of independent
//!   B-Rep solids; will absorb sketches, drawings, assembly and FEM in
//!   Phase 2). Owns the canonical state.
//! - [`Command`] — a serializable enum representing every state-mutating
//!   action. Every variant carries the parameters needed for deterministic
//!   replay. JSON-schema'd via `serde_json`.
//! - [`Session`] — owns one [`Document`], applies [`Command`]s, records
//!   them to a log, and returns typed [`Outcome`]s.
//!
//! ## Minimal example
//!
//! ```
//! use cadkernel_api::{Command, Outcome, Session};
//!
//! let mut session = Session::new();
//! let outcome = session.execute(Command::CreateBox { dx: 10.0, dy: 5.0, dz: 2.0 }).unwrap();
//! assert!(matches!(outcome, Outcome::SolidCreated { .. }));
//! assert_eq!(session.document().solid_count(), 1);
//! ```
//!
//! ## Replay
//!
//! ```
//! use cadkernel_api::{Command, Session};
//!
//! let log = vec![
//!     Command::CreateBox { dx: 10.0, dy: 5.0, dz: 2.0 },
//!     Command::CreateSphere { radius: 1.5 },
//! ];
//! let session = Session::replay(&log).unwrap();
//! assert_eq!(session.document().solid_count(), 2);
//! ```

mod command;
mod document;
mod outcome;
mod session;

pub mod cadk;

pub use command::{Command, ExtrudeKind, command_schemas};
pub use document::{AabbSummary, Document, DocumentIssue, HistoryEvent, MeasureSummary, SolidId};
pub use outcome::{Outcome, OutcomeKind, SolidEntry};
pub use session::{Session, SessionSnapshot};

/// Result type returned by the API surface.
pub type ApiResult<T> = Result<T, ApiError>;

/// Errors surfaced through the public API.
///
/// `ApiError` deliberately wraps lower-level kernel errors into a flat
/// taxonomy that maps cleanly onto JSON-RPC error codes for AI consumers.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "message")]
pub enum ApiError {
    /// A `Command` referenced a [`SolidId`] that does not exist.
    UnknownSolid(String),
    /// `Command` parameters were syntactically valid but semantically wrong
    /// (e.g. negative radius).
    InvalidArgument(String),
    /// The kernel rejected the operation (geometry / topology failure).
    Kernel(String),
    /// JSON serialization or deserialization failed.
    Codec(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownSolid(msg) => write!(f, "unknown solid: {msg}"),
            Self::InvalidArgument(msg) => write!(f, "invalid argument: {msg}"),
            Self::Kernel(msg) => write!(f, "kernel error: {msg}"),
            Self::Codec(msg) => write!(f, "codec error: {msg}"),
        }
    }
}

impl std::error::Error for ApiError {}

impl From<cadkernel_core::KernelError> for ApiError {
    fn from(err: cadkernel_core::KernelError) -> Self {
        match err {
            cadkernel_core::KernelError::InvalidArgument(msg) => Self::InvalidArgument(msg),
            other => Self::Kernel(other.to_string()),
        }
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        Self::Codec(err.to_string())
    }
}
