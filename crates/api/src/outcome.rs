//! Outcome — typed result of executing a [`Command`](crate::Command).

use serde::{Deserialize, Serialize};

use crate::document::{DocumentIssue, SolidId};

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
    ///
    /// - `pattern_id`: the source solid the pattern was generated from.
    /// - `instance_count`: total instances including the original.
    /// - `total_features`: number of newly inserted solids
    ///   (`instance_count - 1`, i.e. excluding the original).
    /// - `ids`: every solid in the pattern, with the original at index 0
    ///   and new instances appended in creation order.
    PatternCreated {
        pattern_id: SolidId,
        instance_count: u32,
        total_features: u32,
        ids: Vec<SolidId>,
    },
    /// The document was reset (`Command::NewDocument`).
    DocumentReset,
    /// Read-only measurement of a solid (`Command::Measure`). Carries the
    /// volume / surface area / centroid / axis-aligned bounding box —
    /// callers branch on this when scripting AI/test workflows that need
    /// to assert on geometry without separately calling the Document API.
    Measured {
        id: SolidId,
        volume: f64,
        surface_area: f64,
        centroid: [f64; 3],
        bbox_min: [f64; 3],
        bbox_max: [f64; 3],
    },
    /// Read-only document health report (`Command::Validate`). `issues`
    /// is empty when the document is clean. AI / test consumers branch
    /// on `issues.is_empty()` to decide whether to surface a warning.
    Validated {
        issues: Vec<DocumentIssue>,
    },
    /// Read-only enumeration of every solid in the document
    /// (`Command::ListSolids`). Pairs each `SolidId` with its label so
    /// AI / test consumers can render a tree view or pick targets for
    /// follow-up commands without juggling `Document::solid_ids()` and
    /// `Document::solid_label()` separately.
    SolidsListed {
        entries: Vec<SolidEntry>,
    },
    /// Nothing happened (`Command::Noop`).
    Empty,
}

/// One row in [`Outcome::SolidsListed`]. Keeps the wire format flat for
/// JSON consumers (`{"id": 0, "label": "Box"}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolidEntry {
    pub id: SolidId,
    pub label: String,
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
    Measured,
    Validated,
    SolidsListed,
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
            Self::Measured { .. } => OutcomeKind::Measured,
            Self::Validated { .. } => OutcomeKind::Validated,
            Self::SolidsListed { .. } => OutcomeKind::SolidsListed,
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
            Self::PatternCreated { ids, .. } => ids.first().copied(),
            Self::Measured { id, .. } => Some(*id),
            _ => None,
        }
    }
}
