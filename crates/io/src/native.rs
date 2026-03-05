//! Native CADKernel project file format (`.cadk`).
//!
//! Human-readable JSON with format header for version tracking.
//! Designed for easy text editing and AI-assisted workflows.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_topology::BRepModel;
use serde::{Deserialize, Serialize};

/// File extension for the native CADKernel project format.
pub const CADK_EXTENSION: &str = "cadk";

/// Current format version string.
const FORMAT_VERSION: &str = "0.1.0";

/// Wrapper for the native `.cadk` file format.
#[derive(Serialize, Deserialize)]
struct CadkFile {
    /// Always `"CADKernel"`.
    format: String,
    /// Format version (semver).
    version: String,
    /// The B-Rep model data.
    model: BRepModel,
}

/// Saves a [`BRepModel`] to the native `.cadk` project format (pretty JSON).
pub fn save_project(model: &BRepModel, path: &str) -> KernelResult<()> {
    let file = CadkFile {
        format: "CADKernel".into(),
        version: FORMAT_VERSION.into(),
        model: model.clone(),
    };
    let json =
        serde_json::to_string_pretty(&file).map_err(|e| KernelError::IoError(e.to_string()))?;
    std::fs::write(path, json).map_err(|e| KernelError::IoError(e.to_string()))
}

/// Loads a [`BRepModel`] from a `.cadk` file, validating the format header.
pub fn load_project(path: &str) -> KernelResult<BRepModel> {
    let content = std::fs::read_to_string(path).map_err(|e| KernelError::IoError(e.to_string()))?;

    // Try parsing as CadkFile first (new format with header).
    if let Ok(file) = serde_json::from_str::<CadkFile>(&content) {
        if file.format != "CADKernel" {
            return Err(KernelError::IoError(format!(
                "unknown format: '{}' (expected 'CADKernel')",
                file.format
            )));
        }
        return Ok(file.model);
    }

    // Fall back to bare BRepModel JSON (legacy / plain json export).
    serde_json::from_str(&content).map_err(|e| KernelError::IoError(e.to_string()))
}
