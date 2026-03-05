//! STEP (ISO 10303-21) reader/writer stubs.

use cadkernel_core::KernelResult;
use cadkernel_math::Point3;

/// A raw STEP entity parsed from text.
#[derive(Debug, Clone)]
pub struct ParsedStepEntity {
    pub id: u64,
    pub entity_type: String,
    pub params: Vec<String>,
}

/// A typed STEP entity.
#[derive(Debug, Clone)]
pub enum StepEntity {
    CartesianPoint(Point3),
    Direction([f64; 3]),
    VertexPoint(u64),
    EdgeCurve {
        start: u64,
        end: u64,
        curve: u64,
    },
    Other {
        entity_type: String,
        params: Vec<String>,
    },
}

/// STEP file writer.
#[derive(Debug)]
pub struct StepWriter {
    entities: Vec<(u64, StepEntity)>,
    next_id: u64,
}

impl StepWriter {
    /// Creates a new empty STEP writer.
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
            next_id: 1,
        }
    }

    /// Adds an entity and returns its STEP entity id.
    pub fn add_entity(&mut self, entity: StepEntity) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.entities.push((id, entity));
        id
    }

    /// Writes all entities to a STEP format string.
    pub fn write(&self) -> KernelResult<String> {
        todo!("STEP write not yet implemented")
    }

    /// Exports to a file.
    pub fn export(&self, _path: &str) -> KernelResult<()> {
        todo!("STEP export not yet implemented")
    }
}

impl Default for StepWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Parses raw STEP entities from text content.
pub fn parse_step_entities(_content: &str) -> KernelResult<Vec<ParsedStepEntity>> {
    todo!("STEP parsing not yet implemented")
}

/// Reads point entities from STEP content.
pub fn read_step_points(_content: &str) -> KernelResult<Vec<Point3>> {
    todo!("STEP point reading not yet implemented")
}

/// Exports a tessellated mesh to STEP format.
pub fn export_step_mesh(_mesh: &super::Mesh, _path: &str) -> KernelResult<()> {
    todo!("STEP mesh export not yet implemented")
}
