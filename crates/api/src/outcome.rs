//! Outcome — typed result of executing a [`Command`](crate::Command).

use serde::{Deserialize, Serialize};

use crate::document::SolidId;

/// Successful result of [`Session::execute`](crate::Session::execute).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Outcome {
    /// A new solid was created.
    SolidCreated { id: SolidId, label: String },
    /// A solid was deleted.
    SolidDeleted { id: SolidId },
    /// A boolean operation produced one new solid and consumed the inputs.
    Booleaned {
        result: SolidId,
        consumed: Vec<SolidId>,
    },
    /// A solid was modified in place.
    SolidModified { id: SolidId },
    /// A pattern (linear/mirror) produced one or more new solids; the
    /// original was preserved.
    PatternCreated { ids: Vec<SolidId> },
    /// The document was reset (`Command::NewDocument`).
    DocumentReset,
    /// Nothing happened (`Command::Noop`).
    Empty,
}

/// Coarse-grained tag, useful for AI/test branching without pattern matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeKind {
    SolidCreated,
    SolidDeleted,
    Booleaned,
    SolidModified,
    PatternCreated,
    DocumentReset,
    Empty,
}

impl Outcome {
    /// Returns the coarse-grained tag for this outcome.
    pub fn kind(&self) -> OutcomeKind {
        match self {
            Self::SolidCreated { .. } => OutcomeKind::SolidCreated,
            Self::SolidDeleted { .. } => OutcomeKind::SolidDeleted,
            Self::Booleaned { .. } => OutcomeKind::Booleaned,
            Self::SolidModified { .. } => OutcomeKind::SolidModified,
            Self::PatternCreated { .. } => OutcomeKind::PatternCreated,
            Self::DocumentReset => OutcomeKind::DocumentReset,
            Self::Empty => OutcomeKind::Empty,
        }
    }

    /// Returns the primary [`SolidId`] introduced by this outcome, if any.
    pub fn primary_id(&self) -> Option<SolidId> {
        match self {
            Self::SolidCreated { id, .. } => Some(*id),
            Self::SolidDeleted { id } => Some(*id),
            Self::Booleaned { result, .. } => Some(*result),
            Self::SolidModified { id } => Some(*id),
            Self::PatternCreated { ids } => ids.first().copied(),
            _ => None,
        }
    }
}
