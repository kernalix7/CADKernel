//! Snapshot-based undo/redo system for [`BRepModel`](crate::BRepModel).

use serde::{Deserialize, Serialize};

use crate::BRepModel;

/// A snapshot entry in the undo/redo stack.
#[derive(Clone, Serialize, Deserialize)]
struct Snapshot {
    model: BRepModel,
    description: String,
}

/// Snapshot-based undo/redo manager for a [`BRepModel`].
///
/// Keeps up to `max_history` snapshots. Recording a new state after an undo
/// discards the redo stack (standard undo semantics).
#[derive(Clone, Serialize, Deserialize)]
pub struct ModelHistory {
    /// Undo stack (most recent at the back).
    undo_stack: Vec<Snapshot>,
    /// Redo stack (most recent at the back).
    redo_stack: Vec<Snapshot>,
    /// The current model state.
    current: BRepModel,
    /// Maximum number of undo snapshots to keep.
    max_history: usize,
}

impl ModelHistory {
    /// Creates a new history rooted at `model` with room for `max_history`
    /// undo snapshots.
    pub fn new(model: BRepModel, max_history: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            current: model,
            max_history,
        }
    }

    /// Returns a reference to the current model state.
    pub fn current_model(&self) -> &BRepModel {
        &self.current
    }

    /// Records a new model state with a human-readable description.
    ///
    /// The previous state is pushed onto the undo stack and the redo stack is
    /// cleared.
    pub fn record(&mut self, model: BRepModel, description: impl Into<String>) {
        let old = std::mem::replace(&mut self.current, model);
        self.undo_stack.push(Snapshot {
            model: old,
            description: description.into(),
        });
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    /// Returns `true` if there is at least one undo step available.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Returns `true` if there is at least one redo step available.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Undoes the last recorded operation, returning a reference to the
    /// restored model.
    ///
    /// Returns `None` if there is nothing to undo.
    pub fn undo(&mut self) -> Option<&BRepModel> {
        let snapshot = self.undo_stack.pop()?;
        let current = std::mem::replace(&mut self.current, snapshot.model);
        self.redo_stack.push(Snapshot {
            model: current,
            description: snapshot.description,
        });
        Some(&self.current)
    }

    /// Re-applies the last undone operation, returning a reference to the
    /// restored model.
    ///
    /// Returns `None` if there is nothing to redo.
    pub fn redo(&mut self) -> Option<&BRepModel> {
        let snapshot = self.redo_stack.pop()?;
        let current = std::mem::replace(&mut self.current, snapshot.model);
        self.undo_stack.push(Snapshot {
            model: current,
            description: snapshot.description,
        });
        Some(&self.current)
    }

    /// Returns descriptions of all undo steps (oldest first).
    pub fn history_descriptions(&self) -> Vec<&str> {
        self.undo_stack
            .iter()
            .map(|s| s.description.as_str())
            .collect()
    }

    /// Number of available undo steps.
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Number of available redo steps.
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }
}
