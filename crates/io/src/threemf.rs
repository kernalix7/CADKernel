use std::fmt::Write;

use cadkernel_core::KernelResult;

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
    fn test_3mf_empty_mesh() {
        let mesh = Mesh::new();
        let xml = export_3mf(&mesh).unwrap();
        assert!(xml.contains("<vertices>"));
        assert!(xml.contains("</vertices>"));
        assert!(!xml.contains("<vertex x="));
    }
}
