use serde::{Deserialize, Serialize};

use crate::face::FaceData;
use crate::handle::Handle;
use crate::naming::Tag;
use crate::solid::SolidData;

/// A connected set of faces forming a manifold (or non-manifold) sheet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellData {
    pub faces: Vec<Handle<FaceData>>,
    pub solid: Option<Handle<SolidData>>,
    pub tag: Option<Tag>,
}

impl ShellData {
    /// Creates an empty shell with no faces.
    pub fn new() -> Self {
        Self {
            faces: Vec::new(),
            solid: None,
            tag: None,
        }
    }
}

impl Default for ShellData {
    fn default() -> Self {
        Self::new()
    }
}
