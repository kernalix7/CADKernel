use std::fmt::Write;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};

use crate::tessellate::Mesh;

/// Export mesh to 3MF format (XML).
pub fn export_3mf(mesh: &Mesh) -> KernelResult<String> {
    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<model unit=\"millimeter\" xmlns=\"http://schemas.microsoft.com/3dmanufacturing/core/2015/02\">\n");
    xml.push_str("  <resources>\n    <object id=\"1\" type=\"model\">\n      <mesh>\n");

    xml.push_str("        <vertices>\n");
    for v in &mesh.vertices {
        let _ = writeln!(
            xml,
            "          <vertex x=\"{}\" y=\"{}\" z=\"{}\" />",
            v.x, v.y, v.z
        );
    }
    xml.push_str("        </vertices>\n");

    xml.push_str("        <triangles>\n");
    for idx in &mesh.indices {
        let _ = writeln!(
            xml,
            "          <triangle v1=\"{}\" v2=\"{}\" v3=\"{}\" />",
            idx[0], idx[1], idx[2]
        );
    }
    xml.push_str("        </triangles>\n");

    xml.push_str("      </mesh>\n    </object>\n  </resources>\n");
    xml.push_str("  <build>\n    <item objectid=\"1\" />\n  </build>\n");
    xml.push_str("</model>\n");
    Ok(xml)
}

/// Import mesh from 3MF XML content.
pub fn import_3mf(content: &str) -> KernelResult<Mesh> {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("<vertex ") {
            let x = extract_attr(trimmed, "x").ok_or_else(|| KernelError::IoError("3MF: missing vertex x".into()))?;
            let y = extract_attr(trimmed, "y").ok_or_else(|| KernelError::IoError("3MF: missing vertex y".into()))?;
            let z = extract_attr(trimmed, "z").ok_or_else(|| KernelError::IoError("3MF: missing vertex z".into()))?;
            vertices.push(Point3::new(x, y, z));
        } else if trimmed.starts_with("<triangle ") {
            let v1 = extract_attr_u32(trimmed, "v1").ok_or_else(|| KernelError::IoError("3MF: missing triangle v1".into()))?;
            let v2 = extract_attr_u32(trimmed, "v2").ok_or_else(|| KernelError::IoError("3MF: missing triangle v2".into()))?;
            let v3 = extract_attr_u32(trimmed, "v3").ok_or_else(|| KernelError::IoError("3MF: missing triangle v3".into()))?;
            indices.push([v1, v2, v3]);
        }
    }

    let normals: Vec<Vec3> = indices.iter().map(|tri| {
        let a = vertices.get(tri[0] as usize).copied().unwrap_or(Point3::ORIGIN);
        let b = vertices.get(tri[1] as usize).copied().unwrap_or(Point3::ORIGIN);
        let c = vertices.get(tri[2] as usize).copied().unwrap_or(Point3::ORIGIN);
        let ab = b - a;
        let ac = c - a;
        ab.cross(ac).normalized().unwrap_or(Vec3::Z)
    }).collect();

    Ok(Mesh { vertices, normals, indices })
}

fn extract_attr(line: &str, name: &str) -> Option<f64> {
    let key = format!("{name}=\"");
    let start = line.find(&key)? + key.len();
    let end = start + line[start..].find('"')?;
    line[start..end].parse().ok()
}

fn extract_attr_u32(line: &str, name: &str) -> Option<u32> {
    let key = format!("{name}=\"");
    let start = line.find(&key)? + key.len();
    let end = start + line[start..].find('"')?;
    line[start..end].parse().ok()
}

/// Write 3MF XML string to file.
pub fn write_3mf(path: &str, content: &str) -> KernelResult<()> {
    std::fs::write(path, content)
        .map_err(|e| cadkernel_core::KernelError::IoError(e.to_string()))
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
            normals: vec![Vec3::Z],
            indices: vec![[0, 1, 2]],
        }
    }

    #[test]
    fn test_3mf_xml_structure() {
        let mesh = make_triangle_mesh();
        let xml = export_3mf(&mesh).unwrap();
        assert!(xml.contains("<?xml version"));
        assert!(xml.contains("<model"));
        assert!(xml.contains("</model>"));
        assert!(xml.contains("<vertices>"));
        assert!(xml.contains("<triangles>"));
        assert!(xml.contains("<vertex"));
        assert!(xml.contains("<triangle"));
        assert!(xml.contains("<build>"));
        assert!(xml.contains("<item objectid=\"1\""));
    }

    #[test]
    fn test_3mf_vertex_count() {
        let mesh = Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(1.0, 1.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            ],
            normals: vec![Vec3::Z, Vec3::Z],
            indices: vec![[0, 1, 2], [0, 2, 3]],
        };
        let xml = export_3mf(&mesh).unwrap();
        let vertex_count = xml.matches("<vertex x=").count();
        assert_eq!(vertex_count, 4);
        let tri_count = xml.matches("<triangle v1=").count();
        assert_eq!(tri_count, 2);
    }

    #[test]
    fn test_3mf_roundtrip() {
        let mesh = make_triangle_mesh();
        let xml = export_3mf(&mesh).unwrap();
        let imported = import_3mf(&xml).unwrap();
        assert_eq!(imported.vertices.len(), 3);
        assert_eq!(imported.indices.len(), 1);
        assert!((imported.vertices[0].x - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_3mf_empty_mesh() {
        let mesh = Mesh::new();
        let xml = export_3mf(&mesh).unwrap();
        assert!(xml.contains("<vertices>"));
        assert!(xml.contains("</vertices>"));
        assert!(!xml.contains("<vertex x="));
    }
}
