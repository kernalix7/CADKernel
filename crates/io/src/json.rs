//! JSON serialization/deserialization for B-Rep models.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_topology::BRepModel;

/// Serializes a [`BRepModel`] to a JSON string.
pub fn model_to_json(model: &BRepModel) -> KernelResult<String> {
    serde_json::to_string_pretty(model).map_err(|e| KernelError::IoError(e.to_string()))
}

/// Deserializes a [`BRepModel`] from a JSON string.
pub fn model_from_json(json: &str) -> KernelResult<BRepModel> {
    serde_json::from_str(json).map_err(|e| KernelError::IoError(e.to_string()))
}

/// Writes a [`BRepModel`] as JSON to a file.
pub fn write_json(model: &BRepModel, path: &str) -> KernelResult<()> {
    let content = model_to_json(model)?;
    std::fs::write(path, content).map_err(|e| KernelError::IoError(e.to_string()))
}

/// Reads a [`BRepModel`] from a JSON file.
pub fn read_json(path: &str) -> KernelResult<BRepModel> {
    let content = std::fs::read_to_string(path).map_err(|e| KernelError::IoError(e.to_string()))?;
    model_from_json(&content)
}

/// Alias for [`write_json`].
pub fn export_json(model: &BRepModel, path: &str) -> KernelResult<()> {
    write_json(model, path)
}

/// Alias for [`read_json`].
pub fn import_json(path: &str) -> KernelResult<BRepModel> {
    read_json(path)
}
