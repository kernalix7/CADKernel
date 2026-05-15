use crate::{Sketch, WorkPlane};

/// Stored sketch data that can be restored into an editing session.
#[derive(Debug, Clone)]
pub struct PersistedSketch {
    pub id: u64,
    pub name: String,
    pub plane: WorkPlane,
    pub sketch: Sketch,
}

impl PersistedSketch {
    pub fn new(id: u64, name: impl Into<String>, plane: WorkPlane, sketch: Sketch) -> Self {
        Self {
            id,
            name: name.into(),
            plane,
            sketch,
        }
    }
}
