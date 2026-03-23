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

/// Wrapper for the native `.cadk` file format (single model, legacy).
#[derive(Serialize, Deserialize)]
struct CadkFile {
    /// Always `"CADKernel"`.
    format: String,
    /// Format version (semver).
    version: String,
    /// The B-Rep model data.
    model: BRepModel,
}

/// A single scene object as stored on disk.
#[derive(Clone, Serialize, Deserialize)]
pub struct SceneObjectData {
    pub name: String,
    pub model: BRepModel,
    pub solid_index: u32,
    pub solid_generation: u64,
    pub color: [f32; 4],
    pub visible: bool,
    pub params_json: Option<String>,
}

/// Multi-object scene file format.
#[derive(Serialize, Deserialize)]
struct CadkSceneFile {
    format: String,
    version: String,
    scene: Vec<SceneObjectData>,
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

/// Saves a multi-object scene to `.cadk` format (pretty JSON).
pub fn save_scene(objects: &[SceneObjectData], path: &str) -> KernelResult<()> {
    let file = CadkSceneFile {
        format: "CADKernel".into(),
        version: FORMAT_VERSION.into(),
        scene: objects.to_vec(),
    };
    let json =
        serde_json::to_string_pretty(&file).map_err(|e| KernelError::IoError(e.to_string()))?;
    std::fs::write(path, json).map_err(|e| KernelError::IoError(e.to_string()))
}

/// Loads a multi-object scene from a `.cadk` file.
///
/// Returns `Ok(objects)` for scene files, or wraps a single-model legacy file
/// into a one-element vec.
pub fn load_scene(path: &str) -> KernelResult<Vec<SceneObjectData>> {
    let content = std::fs::read_to_string(path).map_err(|e| KernelError::IoError(e.to_string()))?;

    // Try multi-object scene format first.
    if let Ok(file) = serde_json::from_str::<CadkSceneFile>(&content) {
        if file.format == "CADKernel" {
            return Ok(file.scene);
        }
    }

    // Fall back: single-model file → wrap in a one-element scene.
    let model = load_project(path)?;
    let solid = model.solids.iter().next().map(|(h, _)| h);
    let (idx, generation) = solid.map_or((0, 0), |h| (h.index(), h.generation()));
    Ok(vec![SceneObjectData {
        name: "Imported".into(),
        model,
        solid_index: idx,
        solid_generation: generation,
        color: [0.70, 0.75, 0.80, 1.0],
        visible: true,
        params_json: None,
    }])
}
