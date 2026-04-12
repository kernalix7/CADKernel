use serde::{Deserialize, Serialize};

use crate::handle::Handle;
use crate::naming::Tag;
use crate::shell::ShellData;

/// A topological solid bounded by one or more closed shells.
///
/// The first shell is typically the outer boundary; additional shells
/// represent internal voids (e.g. a hollow part). Solids are the top-level
/// entities produced by primitive constructors and feature operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolidData {
    pub shells: Vec<Handle<ShellData>>,
    pub tag: Option<Tag>,
}

impl SolidData {
    /// Creates an empty solid with no shells.
    pub fn new() -> Self {
        Self {
            shells: Vec::new(),
            tag: None,
        }
    }
}

impl Default for SolidData {
    fn default() -> Self {
        Self::new()
    }
}
