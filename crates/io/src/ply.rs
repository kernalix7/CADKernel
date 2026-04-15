//! PLY (Polygon File Format / Stanford Triangle Format) import and export.
//!
//! Supports ASCII PLY with vertex positions, optional per-vertex normals,
//! and triangular face elements. Vertex colors are tolerated on import
//! but not exported.

use std::fmt::Write;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};

use crate::tessellate::Mesh;

const MAX_PLY_VERTICES: usize = 50_000_000;
const MAX_PLY_FACES: usize = 50_000_000;

/// Exports a mesh to PLY ASCII format string.
///
/// Writes vertex positions with per-vertex normals (if available, per-face
/// normals are used for vertices that lack individual normals) and
/// triangular face elements with `vertex_indices` property lists.
pub fn export_ply(mesh: &Mesh) -> KernelResult<String> {
    for (i, v) in mesh.vertices.iter().enumerate() {
        if !v.x.is_finite() || !v.y.is_finite() || !v.z.is_finite() {
            return Err(KernelError::IoError(format!(
                "vertex {i} contains non-finite coordinate"
            )));
        }
    }
    let mut out = String::new();
    out.push_str("ply\nformat ascii 1.0\n");
    let _ = writeln!(out, "element vertex {}", mesh.vertices.len());
    out.push_str("property float x\nproperty float y\nproperty float z\n");
    out.push_str("property float nx\nproperty float ny\nproperty float nz\n");
    let _ = writeln!(out, "element face {}", mesh.indices.len());
    out.push_str("property list uchar int vertex_indices\n");
    out.push_str("end_header\n");

    for (i, v) in mesh.vertices.iter().enumerate() {
        let n = if i < mesh.normals.len() {
            mesh.normals[i]
        } else {
            Vec3::Z
        };
        let _ = writeln!(out, "{} {} {} {} {} {}", v.x, v.y, v.z, n.x, n.y, n.z);
    }
    for idx in &mesh.indices {
        let _ = writeln!(out, "3 {} {} {}", idx[0], idx[1], idx[2]);
    }
    Ok(out)
}

/// Imports a PLY ASCII string into a [`Mesh`].
///
/// Parses the PLY header for vertex/face counts and property declarations.
/// If per-vertex normals (`nx`, `ny`, `nz`) are declared, they are read;
/// otherwise per-face normals are computed from vertex positions.
pub fn import_ply(content: &str) -> KernelResult<Mesh> {
    let mut lines = content.lines();
    let mut vertex_count: usize = 0;
    let mut face_count: usize = 0;
    let mut in_header = true;
    let mut has_normals = false;

    // Parse header
    while in_header {
        let line = lines.next().ok_or_else(|| {
            KernelError::IoError("unexpected end of PLY header".into())
        })?;
        let trimmed = line.trim();
        if trimmed == "end_header" {
            in_header = false;
        } else if trimmed.starts_with("element vertex") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 3 {
                vertex_count = parts[2]
                    .parse()
                    .map_err(|e| KernelError::IoError(format!("bad vertex count: {e}")))?;
                if vertex_count > MAX_PLY_VERTICES {
                    return Err(KernelError::IoError(format!(
                        "vertex count {vertex_count} exceeds limit {MAX_PLY_VERTICES}"
                    )));
                }
            }
        } else if trimmed.starts_with("element face") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 3 {
                face_count = parts[2]
                    .parse()
                    .map_err(|e| KernelError::IoError(format!("bad face count: {e}")))?;
                if face_count > MAX_PLY_FACES {
                    return Err(KernelError::IoError(format!(
                        "face count {face_count} exceeds limit {MAX_PLY_FACES}"
                    )));
                }
            }
        } else if trimmed == "property float nx" {
            has_normals = true;
        }
    }

    let mut vertices = Vec::with_capacity(vertex_count);
    let mut normals = Vec::with_capacity(if has_normals { vertex_count } else { 0 });

    // Parse vertices
    for _ in 0..vertex_count {
        let line = lines
            .next()
            .ok_or_else(|| KernelError::IoError("unexpected end of PLY vertex data".into()))?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            return Err(KernelError::IoError(format!(
                "malformed PLY vertex line: {line}"
            )));
        }
        let x: f64 = parts[0]
            .parse()
            .map_err(|e| KernelError::IoError(format!("bad float: {e}")))?;
        let y: f64 = parts[1]
            .parse()
            .map_err(|e| KernelError::IoError(format!("bad float: {e}")))?;
        let z: f64 = parts[2]
            .parse()
            .map_err(|e| KernelError::IoError(format!("bad float: {e}")))?;
        vertices.push(Point3::new(x, y, z));

        if has_normals && parts.len() >= 6 {
            let nx: f64 = parts[3]
                .parse()
                .map_err(|e| KernelError::IoError(format!("bad float: {e}")))?;
            let ny: f64 = parts[4]
                .parse()
                .map_err(|e| KernelError::IoError(format!("bad float: {e}")))?;
            let nz: f64 = parts[5]
                .parse()
                .map_err(|e| KernelError::IoError(format!("bad float: {e}")))?;
            normals.push(Vec3::new(nx, ny, nz));
        }
    }

    let mut indices = Vec::with_capacity(face_count);

    // Parse faces
    for _ in 0..face_count {
        let line = lines
            .next()
            .ok_or_else(|| KernelError::IoError("unexpected end of PLY face data".into()))?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            return Err(KernelError::IoError(format!(
                "malformed PLY face line: {line}"
            )));
        }
        let arity: usize = parts[0]
            .parse()
            .map_err(|e| KernelError::IoError(format!("bad face vertex count: {e}")))?;
        if arity != 3 || parts.len() < 4 {
            return Err(KernelError::IoError(format!(
                "only triangular PLY faces are supported: {line}"
            )));
        }
        let i0: u32 = parts[1]
            .parse()
            .map_err(|e| KernelError::IoError(format!("bad index: {e}")))?;
        let i1: u32 = parts[2]
            .parse()
            .map_err(|e| KernelError::IoError(format!("bad index: {e}")))?;
        let i2: u32 = parts[3]
            .parse()
            .map_err(|e| KernelError::IoError(format!("bad index: {e}")))?;
        for idx in [i0, i1, i2] {
            if idx as usize >= vertex_count {
                return Err(KernelError::IoError(format!(
                    "PLY face index {idx} out of bounds for {vertex_count} vertices"
                )));
            }
        }
        indices.push([i0, i1, i2]);
    }

    // If no per-vertex normals, compute per-face normals
    if normals.is_empty() {
        for idx in &indices {
            let a = vertices[idx[0] as usize];
            let b = vertices[idx[1] as usize];
            let c = vertices[idx[2] as usize];
            let ab = b - a;
            let ac = c - a;
            let n = ab.cross(ac);
            let len = n.length();
            normals.push(if len > 1e-14 {
                n * (1.0 / len)
            } else {
                Vec3::Z
            });
        }
    }

    Ok(Mesh {
        vertices,
        normals,
        indices,
    })
}

/// Writes a PLY format string to a file at the given path.
pub fn write_ply(path: &str, content: &str) -> KernelResult<()> {
    std::fs::write(path, content).map_err(|e| KernelError::IoError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_math::{Point3, Vec3};

    fn make_triangle_mesh() -> Mesh {
        Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            ],
            normals: vec![Vec3::Z, Vec3::Z, Vec3::Z],
            indices: vec![[0, 1, 2]],
        }
    }

    #[test]
    fn test_ply_export_header() {
        let mesh = make_triangle_mesh();
        let ply = export_ply(&mesh).unwrap();
        assert!(ply.starts_with("ply\n"));
        assert!(ply.contains("element vertex 3"));
        assert!(ply.contains("element face 1"));
        assert!(ply.contains("end_header"));
    }

    #[test]
    fn test_ply_roundtrip() {
        let mesh = make_triangle_mesh();
        let ply = export_ply(&mesh).unwrap();
        let imported = import_ply(&ply).unwrap();
        assert_eq!(imported.vertices.len(), 3);
        assert_eq!(imported.triangle_count(), 1);

        for (orig, imp) in mesh.vertices.iter().zip(imported.vertices.iter()) {
            assert!((orig.x - imp.x).abs() < 1e-10);
            assert!((orig.y - imp.y).abs() < 1e-10);
            assert!((orig.z - imp.z).abs() < 1e-10);
        }
    }

    #[test]
    fn test_ply_roundtrip_multi() {
        let mesh = Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(1.0, 1.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            ],
            normals: vec![Vec3::Z, Vec3::Z, Vec3::Z, Vec3::Z],
            indices: vec![[0, 1, 2], [0, 2, 3]],
        };
        let ply = export_ply(&mesh).unwrap();
        let imported = import_ply(&ply).unwrap();
        assert_eq!(imported.vertices.len(), 4);
        assert_eq!(imported.triangle_count(), 2);
    }

    #[test]
    fn test_import_ply_error_truncated() {
        let result = import_ply("ply\nformat ascii 1.0\n");
        assert!(result.is_err());
    }

    #[test]
    fn test_import_ply_error_invalid_face_index() {
        let content = "ply\nformat ascii 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n0 1 0\n3 0 1 99\n";
        assert!(import_ply(content).is_err());
    }

    #[test]
    fn test_ply_header_contains_format() {
        let mesh = make_triangle_mesh();
        let ply = export_ply(&mesh).unwrap();
        assert!(ply.contains("format ascii 1.0"));
    }

    #[test]
    fn test_ply_normals_in_export() {
        let mesh = make_triangle_mesh();
        let ply = export_ply(&mesh).unwrap();
        assert!(ply.contains("property float nx"));
        assert!(ply.contains("property float nz"));
    }

    #[test]
    fn test_ply_roundtrip_normals() {
        let mesh = make_triangle_mesh();
        let ply = export_ply(&mesh).unwrap();
        let imported = import_ply(&ply).unwrap();
        assert!(!imported.normals.is_empty());
    }

    #[test]
    fn test_ply_vertex_property_order() {
        let mesh = make_triangle_mesh();
        let ply = export_ply(&mesh).unwrap();
        let x_pos = ply.find("property float x").unwrap();
        let y_pos = ply.find("property float y").unwrap();
        let z_pos = ply.find("property float z").unwrap();
        assert!(x_pos < y_pos && y_pos < z_pos);
    }

    #[test]
    fn test_ply_face_list_property() {
        let mesh = make_triangle_mesh();
        let ply = export_ply(&mesh).unwrap();
        assert!(ply.contains("property list uchar int vertex_indices"));
    }

    #[test]
    fn test_ply_roundtrip_large_coordinates() {
        let mesh = Mesh {
            vertices: vec![
                Point3::new(100.0, 200.0, 300.0),
                Point3::new(400.0, 500.0, 600.0),
                Point3::new(700.0, 800.0, 900.0),
            ],
            normals: vec![Vec3::Z, Vec3::Z, Vec3::Z],
            indices: vec![[0, 1, 2]],
        };
        let ply = export_ply(&mesh).unwrap();
        let imported = import_ply(&ply).unwrap();
        assert!((imported.vertices[0].x - 100.0).abs() < 1e-6);
        assert!((imported.vertices[2].z - 900.0).abs() < 1e-6);
    }

    #[test]
    fn test_ply_export_face_indices() {
        let mesh = make_triangle_mesh();
        let ply = export_ply(&mesh).unwrap();
        assert!(ply.contains("3 0 1 2"));
    }

    #[test]
    fn test_ply_import_no_normals_computes_face_normals() {
        let ply = "ply\nformat ascii 1.0\n\
            element vertex 3\nproperty float x\nproperty float y\nproperty float z\n\
            element face 1\nproperty list uchar int vertex_indices\n\
            end_header\n\
            0.0 0.0 0.0\n1.0 0.0 0.0\n0.0 1.0 0.0\n\
            3 0 1 2\n";
        let imported = import_ply(ply).unwrap();
        assert_eq!(imported.normals.len(), 1);
        assert!(imported.normals[0].z.abs() > 0.5);
    }

    #[test]
    fn test_ply_roundtrip_negative_coordinates() {
        let mesh = Mesh {
            vertices: vec![
                Point3::new(-1.0, -2.0, -3.0),
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 2.0, 3.0),
            ],
            normals: vec![Vec3::Z, Vec3::Z, Vec3::Z],
            indices: vec![[0, 1, 2]],
        };
        let ply = export_ply(&mesh).unwrap();
        let imported = import_ply(&ply).unwrap();
        assert!((imported.vertices[0].x - (-1.0)).abs() < 1e-6);
    }

    // -----------------------------------------------------------------------
    // Edge case: PLY with vertex colors
    // -----------------------------------------------------------------------

    #[test]
    fn test_ply_import_with_vertex_colors() {
        let ply = "ply\nformat ascii 1.0\n\
            element vertex 3\nproperty float x\nproperty float y\nproperty float z\n\
            property uchar red\nproperty uchar green\nproperty uchar blue\n\
            element face 1\nproperty list uchar int vertex_indices\n\
            end_header\n\
            0.0 0.0 0.0 255 0 0\n\
            1.0 0.0 0.0 0 255 0\n\
            0.0 1.0 0.0 0 0 255\n\
            3 0 1 2\n";
        let imported = import_ply(ply).unwrap();
        assert_eq!(imported.vertices.len(), 3);
        assert_eq!(imported.triangle_count(), 1);
    }

    // -----------------------------------------------------------------------
    // Edge case: empty mesh
    // -----------------------------------------------------------------------

    #[test]
    fn test_ply_empty_mesh() {
        let mesh = Mesh::new();
        let ply = export_ply(&mesh).unwrap();
        assert!(ply.contains("element vertex 0"));
        assert!(ply.contains("element face 0"));
    }

    // -----------------------------------------------------------------------
    // Edge case: export-import-export consistency
    // -----------------------------------------------------------------------

    #[test]
    fn test_ply_export_import_export_consistency() {
        let mesh = make_triangle_mesh();
        let ply1 = export_ply(&mesh).unwrap();
        let reimported = import_ply(&ply1).unwrap();
        let ply2 = export_ply(&reimported).unwrap();

        let v1_count = ply1.lines().filter(|l| l.contains("element vertex")).count();
        let v2_count = ply2.lines().filter(|l| l.contains("element vertex")).count();
        assert_eq!(v1_count, v2_count);
    }

    // -----------------------------------------------------------------------
    // Edge case: malformed header
    // -----------------------------------------------------------------------

    #[test]
    fn test_ply_malformed_missing_end_header() {
        let ply = "ply\nformat ascii 1.0\nelement vertex 3\n";
        let result = import_ply(ply);
        assert!(result.is_err());
    }
}
