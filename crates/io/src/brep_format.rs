//! CADKernel BREP text format import and export.
//!
//! A human-readable text format that serializes B-Rep topology: vertices,
//! edges, faces (with outer-loop edges), shells, and solids. Each section
//! lists entity counts followed by indexed data lines.

use std::fmt::Write;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::Point3;
use cadkernel_topology::BRepModel;

const MAX_BREP_SECTION_COUNT: usize = 5_000_000;

/// Export B-Rep model to text format.
///
/// Format:
/// ```text
/// CADKernel BREP v1
/// VERTICES <count>
/// <index> <x> <y> <z>
/// ...
/// EDGES <count>
/// <index> <start_vertex_idx> <end_vertex_idx>
/// ...
/// FACES <count>
/// <index> <outer_loop_edge_count> <edge_idx_0> <edge_idx_1> ...
/// ...
/// SHELLS <count>
/// <index> <face_count> <face_idx_0> <face_idx_1> ...
/// ...
/// SOLIDS <count>
/// <index> <shell_count> <shell_idx_0> <shell_idx_1> ...
/// ...
/// END
/// ```
pub fn export_brep(model: &BRepModel) -> KernelResult<String> {
    for (_, vd) in model.vertices.iter() {
        if !vd.point.x.is_finite() || !vd.point.y.is_finite() || !vd.point.z.is_finite() {
            return Err(KernelError::IoError(
                "model contains vertex with non-finite coordinates".into(),
            ));
        }
    }
    let mut out = String::new();
    out.push_str("CADKernel BREP v1\n");

    // Collect vertices with stable ordering
    let vert_entries: Vec<_> = model.vertices.iter().collect();
    let mut vert_index = std::collections::HashMap::new();
    let _ = writeln!(out, "VERTICES {}", vert_entries.len());
    for (i, &(vh, vd)) in vert_entries.iter().enumerate() {
        vert_index.insert(vh, i);
        let _ = writeln!(out, "{} {} {} {}", i, vd.point.x, vd.point.y, vd.point.z);
    }

    // Collect edges
    let edge_entries: Vec<_> = model.edges.iter().collect();
    let mut edge_index = std::collections::HashMap::new();
    let _ = writeln!(out, "EDGES {}", edge_entries.len());
    for (i, &(eh, ed)) in edge_entries.iter().enumerate() {
        edge_index.insert(eh, i);
        let si = vert_index.get(&ed.start).copied().ok_or_else(|| {
            KernelError::IoError(format!("edge {i} references missing start vertex"))
        })?;
        let ei = vert_index.get(&ed.end).copied().ok_or_else(|| {
            KernelError::IoError(format!("edge {i} references missing end vertex"))
        })?;
        let _ = writeln!(out, "{} {} {}", i, si, ei);
    }

    // Collect faces
    let face_entries: Vec<_> = model.faces.iter().collect();
    let mut face_index = std::collections::HashMap::new();
    let _ = writeln!(out, "FACES {}", face_entries.len());
    for (i, &(fh, fd)) in face_entries.iter().enumerate() {
        face_index.insert(fh, i);
        if let Some(ld) = model.loops.get(fd.outer_loop) {
            let loop_edges = model.loop_half_edges(ld.half_edge);
            let edge_indices: Vec<String> = loop_edges
                .iter()
                .filter_map(|&heh| {
                    let he = model.half_edges.get(heh)?;
                    let ei = edge_index.get(&he.edge?)?;
                    Some(ei.to_string())
                })
                .collect();
            let _ = writeln!(out, "{} {} {}", i, edge_indices.len(), edge_indices.join(" "));
        } else {
            let _ = writeln!(out, "{} 0", i);
        }
    }

    // Collect shells
    let shell_entries: Vec<_> = model.shells.iter().collect();
    let mut shell_index = std::collections::HashMap::new();
    let _ = writeln!(out, "SHELLS {}", shell_entries.len());
    for (i, &(sh, sd)) in shell_entries.iter().enumerate() {
        shell_index.insert(sh, i);
        let fi: Vec<String> = sd
            .faces
            .iter()
            .filter_map(|fh| face_index.get(fh).map(|x| x.to_string()))
            .collect();
        let _ = writeln!(out, "{} {} {}", i, fi.len(), fi.join(" "));
    }

    // Collect solids
    let solid_entries: Vec<_> = model.solids.iter().collect();
    let _ = writeln!(out, "SOLIDS {}", solid_entries.len());
    for (i, &(_sh, sd)) in solid_entries.iter().enumerate() {
        let si: Vec<String> = sd
            .shells
            .iter()
            .filter_map(|s| shell_index.get(s).map(|x| x.to_string()))
            .collect();
        let _ = writeln!(out, "{} {} {}", i, si.len(), si.join(" "));
    }

    out.push_str("END\n");
    Ok(out)
}

/// Imports a B-Rep model from the CADKernel BREP text format.
///
/// Validates the `"CADKernel BREP v1"` header, then reads VERTICES, EDGES,
/// FACES, SHELLS, and SOLIDS sections to reconstruct the full topology.
pub fn import_brep(content: &str) -> KernelResult<BRepModel> {
    let mut lines = content.lines();

    let header = lines
        .next()
        .ok_or_else(|| KernelError::IoError("empty BREP file".into()))?
        .trim();
    if header != "CADKernel BREP v1" {
        return Err(KernelError::IoError(format!(
            "unsupported BREP header: {header}"
        )));
    }

    let mut model = BRepModel::new();

    // Parse vertices
    let vert_header = lines
        .next()
        .ok_or_else(|| KernelError::IoError("missing VERTICES section".into()))?;
    let vert_count = parse_section_count(vert_header, "VERTICES")?;
    let mut vert_handles = Vec::with_capacity(vert_count);
    for _ in 0..vert_count {
        let line = lines
            .next()
            .ok_or_else(|| KernelError::IoError("truncated vertex data".into()))?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            return Err(KernelError::IoError(format!(
                "malformed vertex line: {line}"
            )));
        }
        let x: f64 = parse_f64(parts[1])?;
        let y: f64 = parse_f64(parts[2])?;
        let z: f64 = parse_f64(parts[3])?;
        vert_handles.push(model.add_vertex(Point3::new(x, y, z)));
    }

    // Parse edges
    let edge_header = lines
        .next()
        .ok_or_else(|| KernelError::IoError("missing EDGES section".into()))?;
    let edge_count = parse_section_count(edge_header, "EDGES")?;
    let mut edge_he_handles = Vec::with_capacity(edge_count);
    for _ in 0..edge_count {
        let line = lines
            .next()
            .ok_or_else(|| KernelError::IoError("truncated edge data".into()))?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            return Err(KernelError::IoError(format!(
                "malformed edge line: {line}"
            )));
        }
        let si: usize = parse_usize(parts[1])?;
        let ei: usize = parse_usize(parts[2])?;
        if si >= vert_handles.len() || ei >= vert_handles.len() {
            return Err(KernelError::IoError("edge vertex index out of range".into()));
        }
        let (edge_h, he_h, _) = model.add_edge(vert_handles[si], vert_handles[ei]);
        edge_he_handles.push((edge_h, he_h));
    }

    // Parse faces
    let face_header = lines
        .next()
        .ok_or_else(|| KernelError::IoError("missing FACES section".into()))?;
    let face_count = parse_section_count(face_header, "FACES")?;
    let mut face_handles = Vec::with_capacity(face_count);
    for _ in 0..face_count {
        let line = lines
            .next()
            .ok_or_else(|| KernelError::IoError("truncated face data".into()))?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(KernelError::IoError(format!(
                "malformed face line: {line}"
            )));
        }
        let edge_cnt: usize = parse_usize(parts[1])?;
        let mut hes = Vec::with_capacity(edge_cnt);
        for k in 0..edge_cnt {
            if k + 2 >= parts.len() {
                return Err(KernelError::IoError("face edge index missing".into()));
            }
            let ei: usize = parse_usize(parts[k + 2])?;
            if ei >= edge_he_handles.len() {
                return Err(KernelError::IoError("face edge index out of range".into()));
            }
            hes.push(edge_he_handles[ei].1);
        }
        if hes.is_empty() {
            return Err(KernelError::IoError("face with no edges".into()));
        }
        let loop_h = model.make_loop(&hes).map_err(|e| {
            KernelError::IoError(format!("failed to create loop: {e}"))
        })?;
        face_handles.push(model.make_face(loop_h));
    }

    // Parse shells
    let shell_header = lines
        .next()
        .ok_or_else(|| KernelError::IoError("missing SHELLS section".into()))?;
    let shell_count = parse_section_count(shell_header, "SHELLS")?;
    let mut shell_handles = Vec::with_capacity(shell_count);
    for _ in 0..shell_count {
        let line = lines
            .next()
            .ok_or_else(|| KernelError::IoError("truncated shell data".into()))?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(KernelError::IoError(format!(
                "malformed shell line: {line}"
            )));
        }
        let fc: usize = parse_usize(parts[1])?;
        let mut faces = Vec::with_capacity(fc);
        for k in 0..fc {
            if k + 2 >= parts.len() {
                return Err(KernelError::IoError("shell face index missing".into()));
            }
            let fi: usize = parse_usize(parts[k + 2])?;
            if fi >= face_handles.len() {
                return Err(KernelError::IoError("shell face index out of range".into()));
            }
            faces.push(face_handles[fi]);
        }
        shell_handles.push(model.make_shell(&faces));
    }

    // Parse solids
    let solid_header = lines
        .next()
        .ok_or_else(|| KernelError::IoError("missing SOLIDS section".into()))?;
    let solid_count = parse_section_count(solid_header, "SOLIDS")?;
    for _ in 0..solid_count {
        let line = lines
            .next()
            .ok_or_else(|| KernelError::IoError("truncated solid data".into()))?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(KernelError::IoError(format!(
                "malformed solid line: {line}"
            )));
        }
        let sc: usize = parse_usize(parts[1])?;
        let mut shells = Vec::with_capacity(sc);
        for k in 0..sc {
            if k + 2 >= parts.len() {
                return Err(KernelError::IoError("solid shell index missing".into()));
            }
            let si: usize = parse_usize(parts[k + 2])?;
            if si >= shell_handles.len() {
                return Err(KernelError::IoError("solid shell index out of range".into()));
            }
            shells.push(shell_handles[si]);
        }
        model.make_solid(&shells);
    }

    Ok(model)
}

fn parse_section_count(line: &str, expected_name: &str) -> KernelResult<usize> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 || parts[0] != expected_name {
        return Err(KernelError::IoError(format!(
            "expected '{expected_name}' section, got: {line}"
        )));
    }
    let count = parse_usize(parts[1])?;
    if count > MAX_BREP_SECTION_COUNT {
        return Err(KernelError::IoError(format!(
            "{expected_name} count {count} exceeds limit {MAX_BREP_SECTION_COUNT}"
        )));
    }
    Ok(count)
}

fn parse_f64(s: &str) -> KernelResult<f64> {
    s.parse()
        .map_err(|e| KernelError::IoError(format!("bad float: {e}")))
}

fn parse_usize(s: &str) -> KernelResult<usize> {
    s.parse()
        .map_err(|e| KernelError::IoError(format!("bad integer: {e}")))
}

/// Writes a BREP format string to a file at the given path.
pub fn write_brep(path: &str, content: &str) -> KernelResult<()> {
    std::fs::write(path, content).map_err(|e| KernelError::IoError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_math::Point3;
    use cadkernel_topology::BRepModel;

    fn make_triangle_solid() -> BRepModel {
        let mut model = BRepModel::new();
        let v0 = model.add_vertex(Point3::new(0.0, 0.0, 0.0));
        let v1 = model.add_vertex(Point3::new(1.0, 0.0, 0.0));
        let v2 = model.add_vertex(Point3::new(0.5, 1.0, 0.0));

        let (_, he01, _) = model.add_edge(v0, v1);
        let (_, he12, _) = model.add_edge(v1, v2);
        let (_, he20, _) = model.add_edge(v2, v0);

        let loop_h = model.make_loop(&[he01, he12, he20]).unwrap();
        let face = model.make_face(loop_h);
        let shell = model.make_shell(&[face]);
        model.make_solid(&[shell]);
        model
    }

    #[test]
    fn test_brep_export_format() {
        let model = make_triangle_solid();
        let brep = export_brep(&model).unwrap();
        assert!(brep.starts_with("CADKernel BREP v1\n"));
        assert!(brep.contains("VERTICES 3"));
        assert!(brep.contains("EDGES 3"));
        assert!(brep.contains("FACES 1"));
        assert!(brep.contains("SHELLS 1"));
        assert!(brep.contains("SOLIDS 1"));
        assert!(brep.ends_with("END\n"));
    }

    #[test]
    fn test_brep_roundtrip() {
        let model = make_triangle_solid();
        let brep = export_brep(&model).unwrap();
        let imported = import_brep(&brep).unwrap();

        assert_eq!(imported.vertices.len(), model.vertices.len());
        assert_eq!(imported.edges.len(), model.edges.len());
    }

    #[test]
    fn test_brep_import_bad_header() {
        let result = import_brep("INVALID HEADER\n");
        assert!(result.is_err());
    }

    #[test]
    fn test_brep_import_empty() {
        let result = import_brep("");
        assert!(result.is_err());
    }

    #[test]
    fn test_brep_roundtrip_vertex_count() {
        let model = make_triangle_solid();
        let brep = export_brep(&model).unwrap();
        let imported = import_brep(&brep).unwrap();
        assert_eq!(imported.vertices.len(), 3);
    }

    #[test]
    fn test_brep_roundtrip_face_count() {
        let model = make_triangle_solid();
        let brep = export_brep(&model).unwrap();
        let imported = import_brep(&brep).unwrap();
        assert_eq!(imported.faces.len(), 1);
    }

    #[test]
    fn test_brep_roundtrip_shell_count() {
        let model = make_triangle_solid();
        let brep = export_brep(&model).unwrap();
        let imported = import_brep(&brep).unwrap();
        assert_eq!(imported.shells.len(), 1);
    }

    #[test]
    fn test_brep_roundtrip_solid_count() {
        let model = make_triangle_solid();
        let brep = export_brep(&model).unwrap();
        let imported = import_brep(&brep).unwrap();
        assert_eq!(imported.solids.len(), 1);
    }

    #[test]
    fn test_brep_export_end_marker() {
        let model = make_triangle_solid();
        let brep = export_brep(&model).unwrap();
        assert!(brep.ends_with("END\n"));
    }

    #[test]
    fn test_brep_export_empty_model() {
        let model = BRepModel::new();
        let brep = export_brep(&model).unwrap();
        assert!(brep.contains("VERTICES 0"));
        assert!(brep.contains("EDGES 0"));
        assert!(brep.contains("FACES 0"));
        assert!(brep.contains("SOLIDS 0"));
    }

    #[test]
    fn test_brep_import_missing_vertices_section() {
        let result = import_brep("CADKernel BREP v1\n");
        assert!(result.is_err());
    }

    #[test]
    fn test_brep_import_excessive_section_count() {
        let result = import_brep("CADKernel BREP v1\nVERTICES 99999999\n");
        assert!(result.is_err());
    }

    #[test]
    fn test_brep_roundtrip_vertex_coordinates() {
        let model = make_triangle_solid();
        let brep = export_brep(&model).unwrap();
        assert!(brep.contains("0 0 0"));
        assert!(brep.contains("1 0 0"));
    }

    #[test]
    fn test_brep_import_bad_vertex_data() {
        let content = "CADKernel BREP v1\nVERTICES 1\n0 notanumber 0 0\nEDGES 0\nFACES 0\nSHELLS 0\nSOLIDS 0\n";
        assert!(import_brep(content).is_err());
    }

    #[test]
    fn test_brep_roundtrip_edge_count() {
        let model = make_triangle_solid();
        let brep = export_brep(&model).unwrap();
        let imported = import_brep(&brep).unwrap();
        assert_eq!(imported.edges.len(), 3);
    }

    #[test]
    fn test_brep_import_truncated_vertex() {
        let content = "CADKernel BREP v1\nVERTICES 2\n0 0.0 0.0 0.0\n";
        assert!(import_brep(content).is_err());
    }

    #[test]
    fn test_brep_contains_sections_in_order() {
        let model = make_triangle_solid();
        let brep = export_brep(&model).unwrap();
        let vi = brep.find("VERTICES").unwrap();
        let ei = brep.find("EDGES").unwrap();
        let fi = brep.find("FACES").unwrap();
        let si = brep.find("SHELLS").unwrap();
        let soi = brep.find("SOLIDS").unwrap();
        assert!(vi < ei && ei < fi && fi < si && si < soi);
    }

    #[test]
    fn test_brep_export_two_faces() {
        let mut model = BRepModel::new();
        let v0 = model.add_vertex(Point3::new(0.0, 0.0, 0.0));
        let v1 = model.add_vertex(Point3::new(1.0, 0.0, 0.0));
        let v2 = model.add_vertex(Point3::new(0.5, 1.0, 0.0));
        let v3 = model.add_vertex(Point3::new(2.0, 0.0, 0.0));

        let (_, he01, _) = model.add_edge(v0, v1);
        let (_, he12, _) = model.add_edge(v1, v2);
        let (_, he20, _) = model.add_edge(v2, v0);
        let (_, he13, _) = model.add_edge(v1, v3);
        let (_, he32, _) = model.add_edge(v3, v2);

        let lh1 = model.make_loop(&[he01, he12, he20]).unwrap();
        let lh2 = model.make_loop(&[he13, he32, he12]).unwrap();
        let f1 = model.make_face(lh1);
        let f2 = model.make_face(lh2);
        let shell = model.make_shell(&[f1, f2]);
        model.make_solid(&[shell]);

        let brep = export_brep(&model).unwrap();
        assert!(brep.contains("FACES 2"));
    }

    // -----------------------------------------------------------------------
    // Edge case: topology preservation (full round-trip)
    // -----------------------------------------------------------------------

    #[test]
    fn test_brep_topology_preservation() {
        let model = make_triangle_solid();
        let brep = export_brep(&model).unwrap();
        let imported = import_brep(&brep).unwrap();

        // All topology counts must match exactly
        assert_eq!(imported.vertices.len(), model.vertices.len());
        assert_eq!(imported.edges.len(), model.edges.len());
        assert_eq!(imported.faces.len(), model.faces.len());
        assert_eq!(imported.shells.len(), model.shells.len());
        assert_eq!(imported.solids.len(), model.solids.len());
    }

    // -----------------------------------------------------------------------
    // Edge case: empty model round-trip
    // -----------------------------------------------------------------------

    #[test]
    fn test_brep_empty_model_roundtrip() {
        let model = BRepModel::new();
        let brep = export_brep(&model).unwrap();
        let imported = import_brep(&brep).unwrap();
        assert_eq!(imported.vertices.len(), 0);
        assert_eq!(imported.edges.len(), 0);
        assert_eq!(imported.faces.len(), 0);
        assert_eq!(imported.shells.len(), 0);
        assert_eq!(imported.solids.len(), 0);
    }

    // -----------------------------------------------------------------------
    // Edge case: export-import-export consistency
    // -----------------------------------------------------------------------

    #[test]
    fn test_brep_export_import_export_consistency() {
        let model = make_triangle_solid();
        let brep1 = export_brep(&model).unwrap();
        let imported = import_brep(&brep1).unwrap();
        let brep2 = export_brep(&imported).unwrap();
        assert_eq!(brep1, brep2);
    }
}
