//! VRML 2.0 (Virtual Reality Modeling Language) import/export.
//!
//! Supports reading and writing VRML 2.0 files with Shape/IndexedFaceSet nodes.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};

use crate::tessellate::Mesh;

/// Import a VRML 2.0 file content into a Mesh.
///
/// Parses `Shape { geometry IndexedFaceSet { ... } }` nodes.
pub fn import_vrml(content: &str) -> KernelResult<Mesh> {
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    // Parse coordinate points
    if let Some(coord_start) = content.find("Coordinate") {
        if let Some(point_start) = content[coord_start..].find("point") {
            let offset = coord_start + point_start;
            if let Some(bracket_start) = content[offset..].find('[') {
                let abs_start = offset + bracket_start + 1;
                if let Some(bracket_end) = content[abs_start..].find(']') {
                    let points_str = &content[abs_start..abs_start + bracket_end];
                    for chunk in points_str.split(',') {
                        let nums: Vec<f64> = chunk
                            .split_whitespace()
                            .filter_map(|s| s.parse::<f64>().ok())
                            .collect();
                        if nums.len() >= 3 {
                            vertices.push(Point3::new(nums[0], nums[1], nums[2]));
                            normals.push(Vec3::Z);
                        }
                    }
                }
            }
        }
    }

    if vertices.is_empty() {
        return Err(KernelError::IoError(
            "no coordinates found in VRML file".into(),
        ));
    }

    // Parse coordIndex
    if let Some(idx_start) = content.find("coordIndex") {
        if let Some(bracket_start) = content[idx_start..].find('[') {
            let abs_start = idx_start + bracket_start + 1;
            if let Some(bracket_end) = content[abs_start..].find(']') {
                let idx_str = &content[abs_start..abs_start + bracket_end];
                let mut face_verts: Vec<u32> = Vec::new();

                for token in idx_str.split(|c: char| c == ',' || c.is_whitespace()) {
                    let token = token.trim();
                    if token.is_empty() {
                        continue;
                    }
                    if let Ok(idx) = token.parse::<i32>() {
                        if idx == -1 {
                            // End of face — triangulate the polygon
                            if face_verts.len() >= 3 {
                                for i in 1..face_verts.len() - 1 {
                                    indices.push([face_verts[0], face_verts[i], face_verts[i + 1]]);
                                }
                            }
                            face_verts.clear();
                        } else if (idx as usize) < vertices.len() {
                            face_verts.push(idx as u32);
                        }
                    }
                }

                // Handle last face if no trailing -1
                if face_verts.len() >= 3 {
                    for i in 1..face_verts.len() - 1 {
                        indices.push([face_verts[0], face_verts[i], face_verts[i + 1]]);
                    }
                }
            }
        }
    }

    // Recompute normals from triangles
    for tri in &indices {
        if (tri[0] as usize) < vertices.len()
            && (tri[1] as usize) < vertices.len()
            && (tri[2] as usize) < vertices.len()
        {
            let v0 = vertices[tri[0] as usize];
            let v1 = vertices[tri[1] as usize];
            let v2 = vertices[tri[2] as usize];
            let e1 = v1 - v0;
            let e2 = v2 - v0;
            let n = e1.cross(e2);
            let n_norm = n.normalized().unwrap_or(Vec3::Z);
            for &idx in tri {
                normals[idx as usize] = n_norm;
            }
        }
    }

    Ok(Mesh {
        vertices,
        normals,
        indices,
    })
}

/// Export a Mesh to VRML 2.0 format string.
pub fn export_vrml(mesh: &Mesh) -> KernelResult<String> {
    if mesh.vertices.is_empty() {
        return Err(KernelError::IoError("empty mesh".into()));
    }

    let mut out = String::new();
    out.push_str("#VRML V2.0 utf8\n\n");
    out.push_str("Shape {\n");
    out.push_str("  appearance Appearance {\n");
    out.push_str("    material Material {\n");
    out.push_str("      diffuseColor 0.8 0.8 0.8\n");
    out.push_str("    }\n");
    out.push_str("  }\n");
    out.push_str("  geometry IndexedFaceSet {\n");
    out.push_str("    coord Coordinate {\n");
    out.push_str("      point [\n");

    for (i, v) in mesh.vertices.iter().enumerate() {
        if i > 0 {
            out.push_str(",\n");
        }
        out.push_str(&format!("        {:.6} {:.6} {:.6}", v.x, v.y, v.z));
    }

    out.push_str("\n      ]\n");
    out.push_str("    }\n");
    out.push_str("    coordIndex [\n");

    for (i, tri) in mesh.indices.iter().enumerate() {
        if i > 0 {
            out.push_str(",\n");
        }
        out.push_str(&format!("      {} {} {} -1", tri[0], tri[1], tri[2]));
    }

    out.push_str("\n    ]\n");
    out.push_str("  }\n");
    out.push_str("}\n");

    Ok(out)
}

/// Write a Mesh to a VRML file at the given path.
pub fn write_vrml(path: &std::path::Path, mesh: &Mesh) -> KernelResult<()> {
    let content = export_vrml(mesh)?;
    std::fs::write(path, content).map_err(|e| KernelError::IoError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_mesh() -> Mesh {
        Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
                Point3::new(0.0, 0.0, 1.0),
            ],
            normals: vec![Vec3::Z; 4],
            indices: vec![[0, 1, 2], [0, 1, 3], [0, 2, 3], [1, 2, 3]],
        }
    }

    #[test]
    fn test_export_vrml() {
        let mesh = test_mesh();
        let vrml = export_vrml(&mesh).unwrap();
        assert!(vrml.contains("#VRML V2.0 utf8"));
        assert!(vrml.contains("IndexedFaceSet"));
        assert!(vrml.contains("Coordinate"));
        assert!(vrml.contains("coordIndex"));
    }

    #[test]
    fn test_import_vrml() {
        let mesh = test_mesh();
        let vrml = export_vrml(&mesh).unwrap();
        let imported = import_vrml(&vrml).unwrap();
        assert_eq!(imported.vertices.len(), 4);
        assert_eq!(imported.indices.len(), 4);
    }

    #[test]
    fn test_roundtrip() {
        let mesh = test_mesh();
        let vrml = export_vrml(&mesh).unwrap();
        let imported = import_vrml(&vrml).unwrap();
        for (orig, imp) in mesh.vertices.iter().zip(imported.vertices.iter()) {
            assert!((orig.x - imp.x).abs() < 1e-4);
            assert!((orig.y - imp.y).abs() < 1e-4);
            assert!((orig.z - imp.z).abs() < 1e-4);
        }
    }

    #[test]
    fn test_export_empty_mesh() {
        let mesh = Mesh {
            vertices: vec![],
            normals: vec![],
            indices: vec![],
        };
        assert!(export_vrml(&mesh).is_err());
    }

    #[test]
    fn test_import_invalid() {
        assert!(import_vrml("not a vrml file").is_err());
    }

    #[test]
    fn test_vrml_roundtrip_coordinates() {
        let mesh = test_mesh();
        let vrml = export_vrml(&mesh).unwrap();
        let imported = import_vrml(&vrml).unwrap();
        for (orig, imp) in mesh.vertices.iter().zip(imported.vertices.iter()) {
            assert!((orig.x - imp.x).abs() < 1e-4);
            assert!((orig.y - imp.y).abs() < 1e-4);
            assert!((orig.z - imp.z).abs() < 1e-4);
        }
    }

    #[test]
    fn test_vrml_index_count() {
        let mesh = test_mesh();
        let vrml = export_vrml(&mesh).unwrap();
        let imported = import_vrml(&vrml).unwrap();
        assert_eq!(imported.indices.len(), mesh.indices.len());
    }

    #[test]
    fn test_vrml_contains_shape() {
        let mesh = test_mesh();
        let vrml = export_vrml(&mesh).unwrap();
        assert!(vrml.contains("Shape") || vrml.contains("IndexedFaceSet"));
    }

    #[test]
    fn test_vrml_single_triangle() {
        let mesh = Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            ],
            normals: vec![Vec3::Z],
            indices: vec![[0, 1, 2]],
        };
        let vrml = export_vrml(&mesh).unwrap();
        let imported = import_vrml(&vrml).unwrap();
        assert_eq!(imported.vertices.len(), 3);
        assert_eq!(imported.indices.len(), 1);
    }

    #[test]
    fn test_vrml_normals_computed() {
        let mesh = test_mesh();
        let vrml = export_vrml(&mesh).unwrap();
        let imported = import_vrml(&vrml).unwrap();
        assert!(!imported.normals.is_empty());
    }

    #[test]
    fn test_vrml_two_quads() {
        let mesh = Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(1.0, 1.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
                Point3::new(2.0, 0.0, 0.0),
                Point3::new(3.0, 0.0, 0.0),
            ],
            normals: vec![Vec3::Z; 3],
            indices: vec![[0, 1, 2], [0, 2, 3], [1, 4, 5]],
        };
        let vrml = export_vrml(&mesh).unwrap();
        let imported = import_vrml(&vrml).unwrap();
        assert_eq!(imported.indices.len(), 3);
    }

    #[test]
    fn test_vrml_version_header() {
        let mesh = test_mesh();
        let vrml = export_vrml(&mesh).unwrap();
        assert!(vrml.starts_with("#VRML"));
    }
}
