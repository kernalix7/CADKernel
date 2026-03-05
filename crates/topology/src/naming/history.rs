use serde::{Deserialize, Serialize};

use super::tag::{EntityKind, OperationId, Tag};

/// Records a single entity evolution within an operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Evolution {
    /// A new entity was created.
    Generated { tag: Tag, kind: EntityKind },
    /// An existing entity was modified (same topological role).
    Modified { old_tag: Tag, new_tag: Tag },
    /// An existing entity was split into parts.
    Split {
        parent_tag: Tag,
        child_tags: Vec<Tag>,
    },
    /// An entity was deleted.
    Deleted { tag: Tag },
}

/// A record of all entity changes from a single modeling operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionRecord {
    pub operation: OperationId,
    pub label: String,
    pub evolutions: Vec<Evolution>,
}

/// Tracks the complete construction history of a [`BRepModel`](super::super::BRepModel).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeHistory {
    next_op_id: u64,
    records: Vec<EvolutionRecord>,
}

impl ShapeHistory {
    /// Creates an empty history with the operation counter starting at 1.
    pub fn new() -> Self {
        Self {
            next_op_id: 1,
            records: Vec::new(),
        }
    }

    /// Allocates the next unique operation id.
    pub fn next_operation(&mut self, label: impl Into<String>) -> OperationId {
        let id = OperationId(self.next_op_id);
        self.next_op_id += 1;
        self.records.push(EvolutionRecord {
            operation: id,
            label: label.into(),
            evolutions: Vec::new(),
        });
        id
    }

    /// Returns the current operation id without advancing.
    pub fn current_op_id(&self) -> Option<OperationId> {
        self.records.last().map(|r| r.operation)
    }

    /// Appends an evolution to the current (latest) operation record.
    pub fn record(&mut self, evolution: Evolution) {
        if let Some(rec) = self.records.last_mut() {
            rec.evolutions.push(evolution);
        }
    }

    /// Returns all recorded operations.
    pub fn records(&self) -> &[EvolutionRecord] {
        &self.records
    }

    /// Finds all evolution records for a given operation.
    pub fn get_record(&self, op: OperationId) -> Option<&EvolutionRecord> {
        self.records.iter().find(|r| r.operation == op)
    }
}

impl Default for ShapeHistory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_ids_increment() {
        let mut h = ShapeHistory::new();
        let op1 = h.next_operation("create box");
        let op2 = h.next_operation("boolean union");
        assert_eq!(op1.0, 1);
        assert_eq!(op2.0, 2);
    }

    #[test]
    fn test_record_evolution() {
        let mut h = ShapeHistory::new();
        let op = h.next_operation("test");
        let tag = Tag::generated(EntityKind::Face, op, 0);
        h.record(Evolution::Generated {
            tag: tag.clone(),
            kind: EntityKind::Face,
        });
        assert_eq!(h.records().len(), 1);
        assert_eq!(h.records()[0].evolutions.len(), 1);
    }
}
