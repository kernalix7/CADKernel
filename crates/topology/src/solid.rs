use serde::{Deserialize, Serialize};

use crate::handle::Handle;
use crate::naming::Tag;
use crate::shell::ShellData;

/// A topological solid bounded by one or more closed shells.
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
