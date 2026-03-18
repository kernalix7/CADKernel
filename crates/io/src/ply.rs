use std::fmt::Write;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};

use crate::tessellate::Mesh;

/// Export mesh to PLY format (ASCII).
pub fn export_ply(mesh: &Mesh) -> KernelResult<String> {
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

/// Import PLY file (ASCII format).
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
            }
        } else if trimmed.starts_with("element face") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 3 {
                face_count = parts[2]
                    .parse()
                    .map_err(|e| KernelError::IoError(format!("bad face count: {e}")))?;
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
        let i0: u32 = parts[1]
            .parse()
            .map_err(|e| KernelError::IoError(format!("bad index: {e}")))?;
        let i1: u32 = parts[2]
            .parse()
            .map_err(|e| KernelError::IoError(format!("bad index: {e}")))?;
        let i2: u32 = parts[3]
            .parse()
            .map_err(|e| KernelError::IoError(format!("bad index: {e}")))?;
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

/// Write PLY string to file.
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
}
