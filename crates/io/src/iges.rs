//! IGES (Initial Graphics Exchange Specification) reader/writer.
//!
//! Supports reading and writing IGES files with fixed-format 80-column records.
//! Entity types: 116 (Point), 110 (Line), 100 (Circular Arc), 126 (NURBS Curve),
//! 128 (NURBS Surface), 124 (Transform).

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::Point3;
use cadkernel_topology::BRepModel;

use crate::Mesh;

// ---------------------------------------------------------------------------
// Entity types
// ---------------------------------------------------------------------------

/// Type of IGES entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IgesEntityType {
    Point,
    Line,
    CircularArc,
    ConicArc,
    RationalBSplineCurve,
    RationalBSplineSurface,
    TransformationMatrix,
    Unknown(u32),
}

impl IgesEntityType {
    fn from_code(code: u32) -> Self {
        match code {
            116 => Self::Point,
            110 => Self::Line,
            100 => Self::CircularArc,
            104 => Self::ConicArc,
            126 => Self::RationalBSplineCurve,
            128 => Self::RationalBSplineSurface,
            124 => Self::TransformationMatrix,
            _ => Self::Unknown(code),
        }
    }

    fn to_code(self) -> u32 {
        match self {
            Self::Point => 116,
            Self::Line => 110,
            Self::CircularArc => 100,
            Self::ConicArc => 104,
            Self::RationalBSplineCurve => 126,
            Self::RationalBSplineSurface => 128,
            Self::TransformationMatrix => 124,
            Self::Unknown(c) => c,
        }
    }
}

/// A parsed IGES entity.
#[derive(Debug, Clone)]
pub struct IgesEntity {
    pub entity_type: IgesEntityType,
    pub params: Vec<f64>,
}

// ---------------------------------------------------------------------------
// IGES Writer
// ---------------------------------------------------------------------------

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
        let mut out = String::new();

        // Start section
        let start_line = format_record("CADKernel IGES Export", 'S', 1);
        out.push_str(&start_line);

        // Global section
        let global_params = "1H,,1H;,7HUnknown,11HCADKernel,44,308,15,308,15,,1.0,1,2HMM,1,0.01,15H20260101.000000,,0.001,100.0,,7HUnknown,7HUnknown,11,0,15H20260101.000000;";
        let global_lines = split_to_records(global_params, 'G');
        for line in &global_lines {
            out.push_str(line);
        }

        // Build Parameter Data and Directory Entry sections
        let mut de_lines: Vec<String> = Vec::new();
        let mut pd_lines: Vec<String> = Vec::new();
        let mut pd_seq = 1u32;

        for entity in &self.entities {
            let param_str = build_param_string(entity);
            let param_records = split_to_records(&param_str, 'P');
            let pd_start = pd_seq;
            let pd_count = param_records.len() as u32;

            for rec in &param_records {
                pd_lines.push(rec.clone());
                pd_seq += 1;
            }

            // Directory Entry: 2 lines per entity
            let etype = entity.entity_type.to_code();
            let de_seq = (de_lines.len() as u32) + 1;
            let line1 = format_de_line1(etype, pd_start, 0, 0, 0, de_seq);
            let line2 = format_de_line2(etype, 0, pd_count, 0, de_seq + 1);
            de_lines.push(line1);
            de_lines.push(line2);
        }

        // Append Directory Entry section
        for line in &de_lines {
            out.push_str(line);
        }

        // Append Parameter Data section (re-number with correct DE pointers)
        let mut pd_seq_out = 1u32;
        let mut entity_idx = 0usize;
        let mut lines_in_entity = 0u32;
        let mut current_de = 1u32;

        // Recompute: walk through pd_lines and assign DE pointers
        if !self.entities.is_empty() {
            let mut entity_pd_counts: Vec<u32> = Vec::new();
            for entity in &self.entities {
                let param_str = build_param_string(entity);
                let recs = split_to_records(&param_str, 'P');
                entity_pd_counts.push(recs.len() as u32);
            }

            for pd_line in &pd_lines {
                // Replace the DE pointer field (columns 65-72) with current entity's DE seq
                let content = if pd_line.len() >= 72 {
                    &pd_line[..64]
                } else {
                    pd_line.trim_end()
                };
                let formatted = format!(
                    "{:<64}{:>8}{:>1}{:>7}\n",
                    content,
                    current_de,
                    'P',
                    pd_seq_out
                );
                out.push_str(&formatted);
                pd_seq_out += 1;
                lines_in_entity += 1;

                if lines_in_entity >= entity_pd_counts[entity_idx] {
                    entity_idx += 1;
                    current_de += 2;
                    lines_in_entity = 0;
                }
            }
        }

        // Terminate section
        let s_count = 1u32;
        let g_count = global_lines.len() as u32;
        let d_count = de_lines.len() as u32;
        let p_count = pd_seq_out - 1;
        let term = format!(
            "{:>8}{:>8}{:>8}{:>8}{:>40}{:>8}{:>1}{:>7}\n",
            format!("S{s_count:>7}"),
            format!("G{g_count:>7}"),
            format!("D{d_count:>7}"),
            format!("P{p_count:>7}"),
            " ",
            " ",
            'T',
            1
        );
        out.push_str(&term);

        Ok(out)
    }

    /// Exports to a file.
    pub fn export(&self, path: &str) -> KernelResult<()> {
        let content = self.write()?;
        std::fs::write(path, content)
            .map_err(|e| KernelError::IoError(format!("write error: {e}")))?;
        Ok(())
    }
}

impl Default for IgesWriter {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Format helpers
// ---------------------------------------------------------------------------

fn format_record(content: &str, section: char, seq: u32) -> String {
    format!("{:<72}{:>1}{:>7}\n", content, section, seq)
}

fn split_to_records(data: &str, section: char) -> Vec<String> {
    let max_content = 64;
    let mut records = Vec::new();
    let mut remaining = data;
    let mut seq = 1u32;

    while !remaining.is_empty() {
        let chunk_len = remaining.len().min(max_content);
        let chunk = &remaining[..chunk_len];
        remaining = &remaining[chunk_len..];
        let line = format!("{:<64}{:>8}{:>1}{:>7}\n", chunk, " ", section, seq);
        records.push(line);
        seq += 1;
    }

    if records.is_empty() {
        records.push(format!("{:<64}{:>8}{:>1}{:>7}\n", " ", " ", section, 1));
    }

    records
}

fn format_de_line1(
    entity_type: u32,
    pd_pointer: u32,
    _structure: u32,
    _line_font: u32,
    _level: u32,
    seq: u32,
) -> String {
    format!(
        "{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>1}{:>7}\n",
        entity_type,
        pd_pointer,
        0,
        0,
        0,
        0,
        0,
        0,
        "00000000",
        'D',
        seq
    )
}

fn format_de_line2(
    entity_type: u32,
    _line_weight: u32,
    param_line_count: u32,
    _form: u32,
    seq: u32,
) -> String {
    format!(
        "{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>1}{:>7}\n",
        entity_type,
        0,
        0,
        param_line_count,
        0,
        " ",
        " ",
        " ",
        " ",
        'D',
        seq
    )
}

fn build_param_string(entity: &IgesEntity) -> String {
    let etype = entity.entity_type.to_code();
    let params_str: Vec<String> = entity.params.iter().map(|v| format_iges_float(*v)).collect();
    if params_str.is_empty() {
        format!("{etype};")
    } else {
        format!("{},{};", etype, params_str.join(","))
    }
}

fn format_iges_float(v: f64) -> String {
    if v == v.floor() && v.abs() < 1e15 {
        format!("{:.1}", v)
    } else {
        format!("{}", v)
    }
}

// ---------------------------------------------------------------------------
// IGES Parser
// ---------------------------------------------------------------------------

/// Parses IGES file content into a list of entities.
pub fn parse_iges(content: &str) -> KernelResult<Vec<IgesEntity>> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return Err(KernelError::IoError("empty IGES file".into()));
    }

    // Classify lines by section
    let mut de_lines: Vec<&str> = Vec::new();
    let mut pd_lines: Vec<&str> = Vec::new();

    for line in &lines {
        if line.len() < 73 {
            continue;
        }
        let section = line.as_bytes()[72] as char;
        match section {
            'D' => de_lines.push(line),
            'P' => pd_lines.push(line),
            _ => {}
        }
    }

    // Parse Directory Entry pairs
    let mut dir_entries: Vec<(u32, u32, u32)> = Vec::new(); // (entity_type, pd_start, pd_count)
    let mut i = 0;
    while i + 1 < de_lines.len() {
        let line1 = de_lines[i];
        let line2 = de_lines[i + 1];

        let entity_type = parse_de_field(line1, 0);
        let pd_pointer = parse_de_field(line1, 1);

        let pd_count = parse_de_field(line2, 3);

        dir_entries.push((entity_type, pd_pointer, pd_count));
        i += 2;
    }

    // Parse Parameter Data
    // Concatenate all P lines into segments indexed by sequence number
    let mut pd_map: std::collections::HashMap<u32, String> = std::collections::HashMap::new();
    for line in &pd_lines {
        if line.len() < 73 {
            continue;
        }
        let seq_str = line[73..].trim();
        if let Ok(seq) = seq_str.parse::<u32>() {
            let data = &line[..64];
            pd_map.insert(seq, data.to_string());
        }
    }

    // Build entities from directory entries
    let mut entities = Vec::new();
    for &(etype_code, pd_start, pd_count) in &dir_entries {
        // Concatenate parameter data lines
        let mut param_data = String::new();
        let count = if pd_count == 0 { 1 } else { pd_count };
        for seq in pd_start..(pd_start + count) {
            if let Some(data) = pd_map.get(&seq) {
                param_data.push_str(data.trim());
            }
        }

        // Remove trailing semicolon
        if param_data.ends_with(';') {
            param_data.pop();
        }

        // Split by comma
        let parts: Vec<&str> = param_data.split(',').collect();

        // First part is entity type number, rest are parameters
        let params: Vec<f64> = parts
            .iter()
            .skip(1)
            .filter_map(|s| s.trim().parse::<f64>().ok())
            .collect();

        let etype = IgesEntityType::from_code(etype_code);
        entities.push(IgesEntity {
            entity_type: etype,
            params,
        });
    }

    Ok(entities)
}

fn parse_de_field(line: &str, field_index: usize) -> u32 {
    let start = field_index * 8;
    let end = (start + 8).min(line.len());
    if start >= line.len() {
        return 0;
    }
    line[start..end].trim().parse::<u32>().unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Import
// ---------------------------------------------------------------------------

/// Imports an IGES file into a BRepModel.
pub fn import_iges(content: &str) -> KernelResult<BRepModel> {
    let entities = parse_iges(content)?;
    let mut model = BRepModel::new();

    // First pass: collect point entities as vertices
    let mut point_vertices = Vec::new();
    for entity in &entities {
        if entity.entity_type == IgesEntityType::Point && entity.params.len() >= 3 {
            let p = Point3::new(entity.params[0], entity.params[1], entity.params[2]);
            let vh = model.add_vertex(p);
            point_vertices.push(vh);
        }
    }

    // Second pass: create edges from line entities
    let mut point_idx = 0usize;
    for entity in &entities {
        match entity.entity_type {
            IgesEntityType::Line => {
                if entity.params.len() >= 6 {
                    let p1 = Point3::new(entity.params[0], entity.params[1], entity.params[2]);
                    let p2 = Point3::new(entity.params[3], entity.params[4], entity.params[5]);
                    let v1 = model.add_vertex(p1);
                    let v2 = model.add_vertex(p2);
                    model.add_edge(v1, v2);
                }
            }
            IgesEntityType::Point => {
                point_idx += 1;
            }
            _ => {}
        }
    }
    let _ = point_idx;

    Ok(model)
}

// ---------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------

/// Exports a BRepModel to an IGES format string.
pub fn export_iges(model: &BRepModel) -> KernelResult<String> {
    let mut writer = IgesWriter::new();

    // Export vertices as Point entities (type 116)
    for (_vh, vd) in model.vertices.iter() {
        writer.add_entity(IgesEntity {
            entity_type: IgesEntityType::Point,
            params: vec![vd.point.x, vd.point.y, vd.point.z],
        });
    }

    // Export edges as Line entities (type 110)
    for (_eh, ed) in model.edges.iter() {
        let p1 = model
            .vertices
            .get(ed.start)
            .map(|v| v.point)
            .unwrap_or(Point3::ORIGIN);
        let p2 = model
            .vertices
            .get(ed.end)
            .map(|v| v.point)
            .unwrap_or(Point3::ORIGIN);
        writer.add_entity(IgesEntity {
            entity_type: IgesEntityType::Line,
            params: vec![p1.x, p1.y, p1.z, p2.x, p2.y, p2.z],
        });
    }

    writer.write()
}

/// Exports a mesh to an IGES format string (vertices as points, triangle edges as lines).
pub fn export_iges_mesh(mesh: &Mesh) -> KernelResult<String> {
    let mut writer = IgesWriter::new();

    // Export vertices as Point entities
    for v in &mesh.vertices {
        writer.add_entity(IgesEntity {
            entity_type: IgesEntityType::Point,
            params: vec![v.x, v.y, v.z],
        });
    }

    // Export triangle edges as Line entities
    let mut edge_set = std::collections::HashSet::new();
    for tri in &mesh.indices {
        let pairs = [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])];
        for (a, b) in pairs {
            let key = if a < b { (a, b) } else { (b, a) };
            if edge_set.insert(key) {
                let p1 = mesh.vertices[a as usize];
                let p2 = mesh.vertices[b as usize];
                writer.add_entity(IgesEntity {
                    entity_type: IgesEntityType::Line,
                    params: vec![p1.x, p1.y, p1.z, p2.x, p2.y, p2.z],
                });
            }
        }
    }

    writer.write()
}

/// Reads IGES lines from text content and returns parsed entities.
pub fn read_iges_lines(content: &str) -> KernelResult<Vec<IgesEntity>> {
    let entities = parse_iges(content)?;
    Ok(entities
        .into_iter()
        .filter(|e| e.entity_type == IgesEntityType::Line)
        .collect())
}

/// Reads point entities from IGES content.
pub fn read_iges_points(content: &str) -> KernelResult<Vec<Point3>> {
    let entities = parse_iges(content)?;
    let mut points = Vec::new();
    for e in &entities {
        if e.entity_type == IgesEntityType::Point && e.params.len() >= 3 {
            points.push(Point3::new(e.params[0], e.params[1], e.params[2]));
        }
    }
    Ok(points)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_iges_basic() {
        let mut model = BRepModel::new();
        let v0 = model.add_vertex(Point3::new(0.0, 0.0, 0.0));
        let v1 = model.add_vertex(Point3::new(1.0, 0.0, 0.0));
        let v2 = model.add_vertex(Point3::new(1.0, 1.0, 0.0));
        let v3 = model.add_vertex(Point3::new(0.0, 1.0, 0.0));
        model.add_edge(v0, v1);
        model.add_edge(v1, v2);
        model.add_edge(v2, v3);
        model.add_edge(v3, v0);

        let output = export_iges(&model).unwrap();

        // Verify sections exist
        assert!(output.contains('S'), "missing Start section");
        assert!(output.contains('G'), "missing Global section");
        assert!(output.contains('D'), "missing Directory section");
        assert!(output.contains('P'), "missing Parameter section");
        assert!(output.contains('T'), "missing Terminate section");
    }

    #[test]
    fn test_parse_iges_roundtrip() {
        let mut model = BRepModel::new();
        let v0 = model.add_vertex(Point3::new(0.0, 0.0, 0.0));
        let v1 = model.add_vertex(Point3::new(1.0, 0.0, 0.0));
        let v2 = model.add_vertex(Point3::new(0.0, 1.0, 0.0));
        model.add_edge(v0, v1);
        model.add_edge(v1, v2);
        model.add_edge(v2, v0);

        let iges_str = export_iges(&model).unwrap();
        let entities = parse_iges(&iges_str).unwrap();

        // 3 vertices (Point) + 3 edges (Line) = 6 entities
        assert_eq!(entities.len(), 6);

        let point_count = entities
            .iter()
            .filter(|e| e.entity_type == IgesEntityType::Point)
            .count();
        let line_count = entities
            .iter()
            .filter(|e| e.entity_type == IgesEntityType::Line)
            .count();
        assert_eq!(point_count, 3);
        assert_eq!(line_count, 3);
    }

    #[test]
    fn test_export_iges_mesh() {
        let mesh = Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            ],
            normals: vec![
                cadkernel_math::Vec3::Z,
                cadkernel_math::Vec3::Z,
                cadkernel_math::Vec3::Z,
            ],
            indices: vec![[0, 1, 2]],
        };

        let output = export_iges_mesh(&mesh).unwrap();
        assert!(output.contains('S'));
        assert!(output.contains('T'));

        let entities = parse_iges(&output).unwrap();
        let point_count = entities
            .iter()
            .filter(|e| e.entity_type == IgesEntityType::Point)
            .count();
        let line_count = entities
            .iter()
            .filter(|e| e.entity_type == IgesEntityType::Line)
            .count();
        assert_eq!(point_count, 3);
        assert_eq!(line_count, 3);
    }

    #[test]
    fn test_import_iges_lines() {
        let mut model = BRepModel::new();
        let v0 = model.add_vertex(Point3::new(0.0, 0.0, 0.0));
        let v1 = model.add_vertex(Point3::new(5.0, 0.0, 0.0));
        model.add_edge(v0, v1);

        let iges_str = export_iges(&model).unwrap();
        let imported = import_iges(&iges_str).unwrap();

        // Should have at least the line's 2 vertices
        assert!(imported.vertices.iter().count() >= 2);
    }

    #[test]
    fn test_read_iges_points() {
        let mut model = BRepModel::new();
        model.add_vertex(Point3::new(1.0, 2.0, 3.0));
        model.add_vertex(Point3::new(4.0, 5.0, 6.0));

        let iges_str = export_iges(&model).unwrap();
        let points = read_iges_points(&iges_str).unwrap();
        assert_eq!(points.len(), 2);
        assert!((points[0].x - 1.0).abs() < 1e-10);
        assert!((points[1].z - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_iges_entity_type_roundtrip() {
        let codes = [116, 110, 100, 104, 126, 128, 124, 999];
        for code in codes {
            let etype = IgesEntityType::from_code(code);
            assert_eq!(etype.to_code(), code);
        }
    }
}
