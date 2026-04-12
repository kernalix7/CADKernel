//! OCA/GCAD format import and export.
//!
//! OCA is a simple line-based CAD geometry format with POINT, LINE, ARC, and
//! FACE commands. This module converts between OCA text and triangle meshes.

use std::path::Path;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::Point3;

use crate::Mesh;

/// Import an OCA file from its text content.
///
/// Recognized commands:
/// - `POINT x y z` — defines a named vertex
/// - `LINE p1 p2` — line between point indices (stored but unused for mesh)
/// - `ARC cx cy cz r start_deg end_deg` — arc center + radius (stored but unused)
/// - `FACE p1 p2 p3 [p4 ...]` — triangulated polygon from point indices
///
/// All other lines are silently ignored.
pub fn import_oca(content: &str) -> KernelResult<Mesh> {
    let mut points: Vec<Point3> = Vec::new();
    let mut indices: Vec<[u32; 3]> = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0].to_uppercase().as_str() {
            "POINT" => {
                if parts.len() < 4 {
                    continue;
                }
                let x = parts[1].parse::<f64>().unwrap_or(0.0);
                let y = parts[2].parse::<f64>().unwrap_or(0.0);
                let z = parts[3].parse::<f64>().unwrap_or(0.0);
                points.push(Point3::new(x, y, z));
            }
            "FACE" => {
                if parts.len() < 4 {
                    continue;
                }
                // Parse vertex indices
                let face_indices: Vec<u32> = parts[1..]
                    .iter()
                    .filter_map(|s| s.parse::<u32>().ok())
                    .collect();
                // Fan-triangulate the polygon from the first vertex
                if face_indices.len() >= 3 {
                    for i in 1..face_indices.len() - 1 {
                        indices.push([face_indices[0], face_indices[i], face_indices[i + 1]]);
                    }
                }
            }
            // LINE and ARC are recognized but don't contribute to triangle mesh
            "LINE" | "ARC" => {}
            _ => {}
        }
    }

    if points.is_empty() {
        return Err(KernelError::IoError("OCA file contains no points".into()));
    }

    // Validate indices
    let max_idx = points.len() as u32;
    for tri in &indices {
        for &idx in tri {
            if idx >= max_idx {
                return Err(KernelError::IoError(format!(
                    "FACE index {} exceeds point count {}",
                    idx, max_idx
                )));
            }
        }
    }

    let mut mesh = Mesh {
        vertices: points,
        normals: Vec::new(),
        indices,
    };

    // Compute per-face normals
    for tri in &mesh.indices {
        let a = mesh.vertices[tri[0] as usize];
        let b = mesh.vertices[tri[1] as usize];
        let c = mesh.vertices[tri[2] as usize];
        let ab = b - a;
        let ac = c - a;
        let n = ab.cross(ac).normalized().unwrap_or(cadkernel_math::Vec3::ZERO);
        mesh.normals.push(n);
    }

    Ok(mesh)
}

/// Export a mesh to OCA format text.
pub fn export_oca(mesh: &Mesh) -> KernelResult<String> {
    if mesh.vertices.is_empty() {
        return Err(KernelError::IoError(
            "cannot export empty mesh to OCA".into(),
        ));
    }

    let mut out = String::new();
    out.push_str("# OCA/GCAD format\n");

    // Write points
    for (i, v) in mesh.vertices.iter().enumerate() {
        out.push_str(&format!("POINT {} {} {} # {}\n", v.x, v.y, v.z, i));
    }

    // Write faces (each triangle as a FACE command)
    for tri in &mesh.indices {
        out.push_str(&format!("FACE {} {} {}\n", tri[0], tri[1], tri[2]));
    }

    Ok(out)
}

/// Write a mesh to an OCA file on disk.
pub fn write_oca(path: &Path, mesh: &Mesh) -> KernelResult<()> {
    let content = export_oca(mesh)?;
    std::fs::write(path, content).map_err(|e| KernelError::IoError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_math::Point3;

    fn make_triangle_oca() -> String {
        "# Test triangle\nPOINT 0.0 0.0 0.0\nPOINT 1.0 0.0 0.0\nPOINT 0.0 1.0 0.0\nFACE 0 1 2\n"
            .to_string()
    }

    #[test]
    fn test_import_oca_triangle() {
        let mesh = import_oca(&make_triangle_oca()).unwrap();
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.indices.len(), 1);
        assert_eq!(mesh.indices[0], [0, 1, 2]);
    }

    #[test]
    fn test_import_oca_quad_face() {
        let content = "POINT 0 0 0\nPOINT 1 0 0\nPOINT 1 1 0\nPOINT 0 1 0\nFACE 0 1 2 3\n";
        let mesh = import_oca(content).unwrap();
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices.len(), 2); // quad split into 2 triangles
    }

    #[test]
    fn test_import_oca_ignores_comments() {
        let content = "# comment\nPOINT 0 0 0\n# another comment\nPOINT 1 0 0\nPOINT 0 1 0\nFACE 0 1 2\n";
        let mesh = import_oca(content).unwrap();
        assert_eq!(mesh.vertices.len(), 3);
    }

    #[test]
    fn test_import_oca_empty() {
        assert!(import_oca("").is_err());
        assert!(import_oca("# only comments").is_err());
    }

    #[test]
    fn test_import_oca_invalid_index() {
        let content = "POINT 0 0 0\nPOINT 1 0 0\nPOINT 0 1 0\nFACE 0 1 99\n";
        assert!(import_oca(content).is_err());
    }

    #[test]
    fn test_export_oca() {
        let mesh = Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            ],
            normals: Vec::new(),
            indices: vec![[0, 1, 2]],
        };
        let text = export_oca(&mesh).unwrap();
        assert!(text.contains("POINT 0 0 0"));
        assert!(text.contains("FACE 0 1 2"));
    }

    #[test]
    fn test_oca_roundtrip() {
        let mesh = Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
                Point3::new(1.0, 1.0, 0.0),
            ],
            normals: Vec::new(),
            indices: vec![[0, 1, 2], [1, 3, 2]],
        };
        let text = export_oca(&mesh).unwrap();
        let imported = import_oca(&text).unwrap();
        assert_eq!(imported.vertices.len(), mesh.vertices.len());
        assert_eq!(imported.indices.len(), mesh.indices.len());
    }

    #[test]
    fn test_export_oca_empty() {
        let mesh = Mesh {
            vertices: Vec::new(),
            normals: Vec::new(),
            indices: Vec::new(),
        };
        assert!(export_oca(&mesh).is_err());
    }

    #[test]
    fn test_import_oca_line_command_ignored() {
        let content = "POINT 0 0 0\nPOINT 1 0 0\nPOINT 0 1 0\nLINE 0 1\nFACE 0 1 2\n";
        let mesh = import_oca(content).unwrap();
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.indices.len(), 1);
    }

    #[test]
    fn test_import_oca_arc_command_ignored() {
        let content = "POINT 0 0 0\nPOINT 1 0 0\nPOINT 0 1 0\nARC 0.5 0.5 0 0.5 0 90\nFACE 0 1 2\n";
        let mesh = import_oca(content).unwrap();
        assert_eq!(mesh.vertices.len(), 3);
    }

    #[test]
    fn test_export_oca_header_comment() {
        let mesh = Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            ],
            normals: Vec::new(),
            indices: vec![[0, 1, 2]],
        };
        let text = export_oca(&mesh).unwrap();
        assert!(text.starts_with("# OCA/GCAD format\n"));
    }

    #[test]
    fn test_import_oca_normals_computed() {
        let mesh = import_oca(&make_triangle_oca()).unwrap();
        assert_eq!(mesh.normals.len(), 1);
        assert!((mesh.normals[0].z - 1.0).abs() < 1e-6 || mesh.normals[0].z.abs() > 0.0);
    }

    #[test]
    fn test_import_oca_no_faces() {
        let content = "POINT 0 0 0\nPOINT 1 0 0\nPOINT 0 1 0\n";
        let mesh = import_oca(content).unwrap();
        assert_eq!(mesh.indices.len(), 0);
    }

    #[test]
    fn test_oca_roundtrip_multiple_faces() {
        let mesh = Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
                Point3::new(1.0, 1.0, 0.0),
                Point3::new(2.0, 0.0, 0.0),
            ],
            normals: Vec::new(),
            indices: vec![[0, 1, 2], [1, 3, 2], [1, 4, 3]],
        };
        let text = export_oca(&mesh).unwrap();
        let imported = import_oca(&text).unwrap();
        assert_eq!(imported.indices.len(), 3);
    }

    #[test]
    fn test_import_oca_case_insensitive() {
        let content = "point 0 0 0\npoint 1 0 0\npoint 0 1 0\nface 0 1 2\n";
        let mesh = import_oca(content).unwrap();
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.indices.len(), 1);
    }

    #[test]
    fn test_oca_vertex_coordinates() {
        let content = "POINT 3.14 2.71 1.41\nPOINT 0 0 0\nPOINT 1 0 0\nFACE 0 1 2\n";
        let mesh = import_oca(content).unwrap();
        assert!(mesh.vertices[0].x > 3.0 && mesh.vertices[0].x < 3.2);
        assert!(mesh.vertices[0].y > 2.5 && mesh.vertices[0].y < 3.0);
    }
}
