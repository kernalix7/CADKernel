use std::collections::BTreeSet;

use crate::Sketch;
use crate::solver::{SolverBlock, analyze_constraint_set};

/// Index of a constraint inside [`Sketch::constraints`].
pub type ConstraintId = usize;

/// Returns `true` when the selected constraints contain a rank conflict.
pub fn is_conflicting(sketch: &Sketch, constraint_ids: &[ConstraintId]) -> bool {
    let ids = normalized_ids(sketch, constraint_ids);
    if ids.is_empty() {
        return false;
    }
    let rank = analyze_constraint_set(sketch, &ids);
    rank.equation_count > rank.rank
}

/// Detect a deletion-minimal conflicting subset of the supplied constraints.
pub fn detect_conflict(sketch: &Sketch, constraint_ids: &[ConstraintId]) -> Vec<ConstraintId> {
    let ids = normalized_ids(sketch, constraint_ids);
    if !is_conflicting(sketch, &ids) {
        return Vec::new();
    }

    let mut conflict = quick_xplain(sketch, &[], &ids);
    if conflict.is_empty() {
        conflict = ids;
    }
    minimize_conflict(sketch, conflict)
}

/// Detect a minimal conflicting subset inside an over-constrained solver block.
pub fn detect_conflict_in_block(sketch: &Sketch, block: &SolverBlock) -> Vec<ConstraintId> {
    detect_conflict(sketch, &block.constraint_indices)
}

fn quick_xplain(
    sketch: &Sketch,
    background: &[ConstraintId],
    candidates: &[ConstraintId],
) -> Vec<ConstraintId> {
    if candidates.is_empty() {
        return Vec::new();
    }

    let mut combined = background.to_vec();
    combined.extend_from_slice(candidates);
    if !is_conflicting(sketch, &combined) {
        return Vec::new();
    }

    if candidates.len() == 1 {
        return candidates.to_vec();
    }

    let mid = candidates.len() / 2;
    let first = &candidates[..mid];
    let second = &candidates[mid..];

    let mut background_with_second = background.to_vec();
    background_with_second.extend_from_slice(second);
    let mut left = quick_xplain(sketch, &background_with_second, first);

    let mut background_with_left = background.to_vec();
    background_with_left.extend_from_slice(&left);
    let right = quick_xplain(sketch, &background_with_left, second);
    left.extend(right);
    normalized_slice(&left)
}

fn minimize_conflict(sketch: &Sketch, mut conflict: Vec<ConstraintId>) -> Vec<ConstraintId> {
    conflict = normalized_slice(&conflict);
    let mut index = 0;
    while index < conflict.len() {
        let mut candidate = conflict.clone();
        candidate.remove(index);
        if is_conflicting(sketch, &candidate) {
            conflict = candidate;
            index = 0;
        } else {
            index += 1;
        }
    }
    conflict
}

fn normalized_ids(sketch: &Sketch, ids: &[ConstraintId]) -> Vec<ConstraintId> {
    normalized_slice(ids)
        .into_iter()
        .filter(|id| *id < sketch.constraints.len())
        .collect()
}

fn normalized_slice(ids: &[ConstraintId]) -> Vec<ConstraintId> {
    let set: BTreeSet<_> = ids.iter().copied().collect();
    set.into_iter().collect()
}
