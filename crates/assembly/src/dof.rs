use serde::{Deserialize, Serialize};

use crate::{Assembly, ComponentId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DofAnalysis {
    pub total_dof: i32,
    pub constraint_count: i32,
    pub redundancy: i32,
    pub remaining_dof: i32,
    pub fully_constrained: bool,
    pub over_constrained: bool,
    pub free_components: Vec<ComponentId>,
}

pub fn dof_count(assembly: &Assembly) -> i32 {
    analyze(assembly).remaining_dof
}

pub fn analyze(assembly: &Assembly) -> DofAnalysis {
    let total_dof = assembly.component_count() as i32 * 6;
    let constraint_count = assembly
        .active_mates()
        .map(|(_, mate)| mate.constraint_count())
        .sum::<i32>();
    let redundancy = (constraint_count - total_dof).max(0);
    let remaining_dof = total_dof - constraint_count + redundancy;
    let locked_components = assembly
        .active_mates()
        .filter_map(|(_, mate)| match mate {
            crate::Mate::Lock { component } => Some(*component),
            _ => None,
        })
        .collect::<std::collections::BTreeSet<_>>();
    let free_components = assembly
        .component_ids()
        .into_iter()
        .filter(|id| !locked_components.contains(id))
        .collect();

    DofAnalysis {
        total_dof,
        constraint_count,
        redundancy,
        remaining_dof,
        fully_constrained: remaining_dof == 0 && constraint_count >= total_dof,
        over_constrained: constraint_count > total_dof,
        free_components,
    }
}
