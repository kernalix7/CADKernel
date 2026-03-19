use std::fmt::Write;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};

use crate::tessellate::Mesh;

/// Export mesh to Collada DAE format (XML).
pub fn export_dae(mesh: &Mesh) -> KernelResult<String> {
    let vc = mesh.vertices.len();
    let tc = mesh.indices.len();
    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    xml.push_str("<COLLADA xmlns=\"http://www.collada.org/2005/11/COLLADASchema\" version=\"1.4.1\">\n");
    xml.push_str("  <asset><created>2026-01-01</created><up_axis>Z_UP</up_axis></asset>\n");

    // Geometry library
    xml.push_str("  <library_geometries>\n");
    xml.push_str("    <geometry id=\"mesh0\" name=\"mesh\">\n      <mesh>\n");

    // Positions
    let _ = writeln!(xml, "        <source id=\"pos\"><float_array id=\"pos-array\" count=\"{}\">", vc * 3);
    for v in &mesh.vertices {
        let _ = write!(xml, "{} {} {} ", v.x, v.y, v.z);
    }
    xml.push_str("</float_array>\n");
    let _ = writeln!(xml, "          <technique_common><accessor source=\"#pos-array\" count=\"{vc}\" stride=\"3\">");
    xml.push_str("            <param name=\"X\" type=\"float\"/><param name=\"Y\" type=\"float\"/><param name=\"Z\" type=\"float\"/>\n");
    xml.push_str("          </accessor></technique_common></source>\n");

    // Normals
    let _ = writeln!(xml, "        <source id=\"norm\"><float_array id=\"norm-array\" count=\"{}\">", tc * 3);
    for n in &mesh.normals {
        let _ = write!(xml, "{} {} {} ", n.x, n.y, n.z);
    }
    xml.push_str("</float_array>\n");
    let _ = writeln!(xml, "          <technique_common><accessor source=\"#norm-array\" count=\"{tc}\" stride=\"3\">");
    xml.push_str("            <param name=\"X\" type=\"float\"/><param name=\"Y\" type=\"float\"/><param name=\"Z\" type=\"float\"/>\n");
    xml.push_str("          </accessor></technique_common></source>\n");

    // Vertices
    xml.push_str("        <vertices id=\"verts\"><input semantic=\"POSITION\" source=\"#pos\"/></vertices>\n");

    // Triangles
    let _ = writeln!(xml, "        <triangles count=\"{tc}\">");
    xml.push_str("          <input semantic=\"VERTEX\" source=\"#verts\" offset=\"0\"/>\n");
    xml.push_str("          <p>");
    for tri in &mesh.indices {
        let _ = write!(xml, "{} {} {} ", tri[0], tri[1], tri[2]);
    }
    xml.push_str("</p>\n");
    xml.push_str("        </triangles>\n");

    xml.push_str("      </mesh>\n    </geometry>\n  </library_geometries>\n");

    // Visual scene
    xml.push_str("  <library_visual_scenes>\n    <visual_scene id=\"scene\">\n");
    xml.push_str("      <node><instance_geometry url=\"#mesh0\"/></node>\n");
    xml.push_str("    </visual_scene>\n  </library_visual_scenes>\n");
    xml.push_str("  <scene><instance_visual_scene url=\"#scene\"/></scene>\n");
    xml.push_str("</COLLADA>\n");

    Ok(xml)
}

/// Import mesh from Collada DAE content.
pub fn import_dae(content: &str) -> KernelResult<Mesh> {
    // Extract float_array for positions (first one)
    let pos_data = extract_float_array(content, "pos-array")
        .or_else(|| extract_first_float_array(content))
        .ok_or_else(|| KernelError::IoError("DAE: no position float_array found".into()))?;

    if pos_data.len() % 3 != 0 {
        return Err(KernelError::IoError("DAE: position count not multiple of 3".into()));
    }
    let vertices: Vec<Point3> = pos_data.chunks(3)
        .map(|c| Point3::new(c[0], c[1], c[2]))
        .collect();

    // Extract <p> indices
    let indices_flat = extract_p_indices(content)
        .ok_or_else(|| KernelError::IoError("DAE: no <p> element found".into()))?;
    if indices_flat.len() % 3 != 0 {
        return Err(KernelError::IoError("DAE: index count not multiple of 3".into()));
    }
    let indices: Vec<[u32; 3]> = indices_flat.chunks(3)
        .map(|c| [c[0], c[1], c[2]])
        .collect();

    let normals: Vec<Vec3> = indices.iter().map(|tri| {
        let a = vertices.get(tri[0] as usize).copied().unwrap_or(Point3::ORIGIN);
        let b = vertices.get(tri[1] as usize).copied().unwrap_or(Point3::ORIGIN);
        let c = vertices.get(tri[2] as usize).copied().unwrap_or(Point3::ORIGIN);
        (b - a).cross(c - a).normalized().unwrap_or(Vec3::Z)
    }).collect();

    Ok(Mesh { vertices, normals, indices })
}

/// Write DAE string to file.
pub fn write_dae(path: &str, content: &str) -> KernelResult<()> {
    std::fs::write(path, content).map_err(|e| KernelError::IoError(e.to_string()))
}

fn extract_float_array(content: &str, id: &str) -> Option<Vec<f64>> {
    let tag = format!("id=\"{id}\"");
    let pos = content.find(&tag)?;
    let after = &content[pos..];
    let start = after.find('>')? + 1;
    let end = after.find("</float_array>")?;
    let text = &after[start..end];
    Some(text.split_whitespace().filter_map(|s| s.parse().ok()).collect())
}

fn extract_first_float_array(content: &str) -> Option<Vec<f64>> {
    let tag = "<float_array";
    let pos = content.find(tag)?;
    let after = &content[pos..];
    let start = after.find('>')? + 1;
    let end = after.find("</float_array>")?;
    let text = &after[start..end];
    Some(text.split_whitespace().filter_map(|s| s.parse().ok()).collect())
}

fn extract_p_indices(content: &str) -> Option<Vec<u32>> {
    let tag = "<p>";
    let pos = content.find(tag)?;
    let start = pos + tag.len();
    let end = start + content[start..].find("</p>")?;
    let text = &content[start..end];
    Some(text.split_whitespace().filter_map(|s| s.parse().ok()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_dae_export_structure() {
        let mesh = make_triangle_mesh();
        let dae = export_dae(&mesh).unwrap();
        assert!(dae.contains("COLLADA"));
        assert!(dae.contains("float_array"));
        assert!(dae.contains("<triangles"));
        assert!(dae.contains("<p>"));
    }

    #[test]
    fn test_dae_roundtrip() {
        let mesh = make_triangle_mesh();
        let dae = export_dae(&mesh).unwrap();
        let imported = import_dae(&dae).unwrap();
        assert_eq!(imported.vertices.len(), 3);
        assert_eq!(imported.indices.len(), 1);
    }
}
