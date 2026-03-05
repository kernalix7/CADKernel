//! IGES (Initial Graphics Exchange Specification) reader/writer stubs.

use cadkernel_core::KernelResult;
use cadkernel_math::Point3;

/// Type of IGES entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IgesEntityType {
    Point,
    Line,
    CircularArc,
    ConicArc,
    RationalBSplineCurve,
    RationalBSplineSurface,
}

/// A parsed IGES entity.
#[derive(Debug, Clone)]
pub struct IgesEntity {
    pub entity_type: IgesEntityType,
    pub params: Vec<f64>,
}

/// IGES file writer.
#[derive(Debug)]
pub struct IgesWriter {
    entities: Vec<IgesEntity>,
}

impl IgesWriter {
    /// Creates a new empty IGES writer.
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }

    /// Adds an entity to the writer.
    pub fn add_entity(&mut self, entity: IgesEntity) {
        self.entities.push(entity);
    }

    /// Writes all entities to an IGES format string.
    pub fn write(&self) -> KernelResult<String> {
        todo!("IGES write not yet implemented")
    }

    /// Exports to a file.
    pub fn export(&self, _path: &str) -> KernelResult<()> {
        todo!("IGES export not yet implemented")
    }
}

impl Default for IgesWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Reads IGES lines from text content and returns parsed entities.
pub fn read_iges_lines(_content: &str) -> KernelResult<Vec<IgesEntity>> {
    todo!("IGES line reading not yet implemented")
}

/// Reads point entities from IGES content.
pub fn read_iges_points(_content: &str) -> KernelResult<Vec<Point3>> {
    todo!("IGES point reading not yet implemented")
}
