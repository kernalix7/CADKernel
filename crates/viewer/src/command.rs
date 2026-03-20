//! Undo/redo command stack for the CAD application.

use cadkernel_io::Mesh;
use cadkernel_topology::{BRepModel, Handle, SolidData};

/// Snapshot of the application model state before a command.
#[derive(Clone)]
pub struct ModelSnapshot {
    pub model: BRepModel,
    pub current_solid: Option<Handle<SolidData>>,
    pub current_mesh: Option<Mesh>,
}

/// A recorded command with before/after state for undo/redo.
pub struct CommandRecord {
    /// Human-readable description of the operation.
    pub description: String,
    /// State before the command was executed.
    pub before: ModelSnapshot,
}

/// Undo/redo stack that manages command history.
pub struct CommandStack {
    /// Past commands (for undo). Most recent is last.
    history: Vec<CommandRecord>,
    /// Future commands (for redo). Most recent undo is last.
    future: Vec<CommandRecord>,
    /// Maximum history depth.
    max_depth: usize,
}

impl CommandStack {
    pub fn new(max_depth: usize) -> Self {
        Self {
            history: Vec::new(),
            future: Vec::new(),
            max_depth,
        }
    }

    /// Record a snapshot before executing a command.
    /// Call this BEFORE modifying the model.
    pub fn push(&mut self, description: impl Into<String>, snapshot: ModelSnapshot) {
        self.future.clear(); // new command invalidates redo stack
        self.history.push(CommandRecord {
            description: description.into(),
            before: snapshot,
        });
        if self.history.len() > self.max_depth {
            self.history.remove(0);
        }
    }

    /// Undo the last command. Returns the snapshot to restore.
    /// The caller must pass the CURRENT state so it can be pushed to redo.
    pub fn undo(&mut self, current: ModelSnapshot) -> Option<ModelSnapshot> {
        let record = self.history.pop()?;
        let restore = record.before.clone();
        self.future.push(CommandRecord {
            description: record.description,
            before: current,
        });
        Some(restore)
    }

    /// Redo a previously undone command. Returns the snapshot to restore.
    pub fn redo(&mut self, current: ModelSnapshot) -> Option<ModelSnapshot> {
        let record = self.future.pop()?;
        let restore = record.before.clone();
        self.history.push(CommandRecord {
            description: record.description,
            before: current,
        });
        Some(restore)
    }

    /// Whether undo is available.
    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    /// Whether redo is available.
    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    /// Description of the command that would be undone.
    pub fn undo_description(&self) -> Option<&str> {
        self.history.last().map(|r| r.description.as_str())
    }

    /// Description of the command that would be redone.
    pub fn redo_description(&self) -> Option<&str> {
        self.future.last().map(|r| r.description.as_str())
    }

    /// Number of commands in history.
    pub fn history_len(&self) -> usize {
        self.history.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_snapshot(id: u32) -> ModelSnapshot {
        let mut model = BRepModel::new();
        // Use vertex count as a marker to distinguish snapshots
        for _ in 0..id {
            model.add_vertex(cadkernel_math::Point3::ORIGIN);
        }
        ModelSnapshot {
            model,
            current_solid: None,
            current_mesh: None,
        }
    }

    #[test]
    fn test_undo_redo_basic() {
        let mut stack = CommandStack::new(100);

        let s0 = make_snapshot(0);
        stack.push("create box", s0.clone());

        let s1 = make_snapshot(1);
        let restored = stack.undo(s1).unwrap();
        assert_eq!(restored.model.vertices.len(), 0);
        assert!(stack.can_redo());
    }

    #[test]
    fn test_undo_empty() {
        let mut stack = CommandStack::new(100);
        assert!(stack.undo(make_snapshot(0)).is_none());
    }

    #[test]
    fn test_new_command_clears_redo() {
        let mut stack = CommandStack::new(100);
        stack.push("cmd1", make_snapshot(0));
        let _ = stack.undo(make_snapshot(1));
        assert!(stack.can_redo());

        stack.push("cmd2", make_snapshot(2));
        assert!(!stack.can_redo()); // redo cleared
    }
}
