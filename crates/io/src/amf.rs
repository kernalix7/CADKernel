//! AMF (Additive Manufacturing File) import/export.
//!
//! Supports reading and writing AMF XML files with object/mesh/volume/triangle structure.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};

use crate::tessellate::Mesh;

/// Import an AMF XML content string into a Mesh.
///
/// Parses `<object><mesh><vertices>` and `<volume><triangle>` elements.
pub fn import_amf(content: &str) -> KernelResult<Mesh> {
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    // Parse vertices: <vertex><coordinates><x>...<y>...<z>...</coordinates></vertex>
    let mut search_pos = 0;
    while let Some(vtx_start) = content[search_pos..].find("<vertex>") {
        let abs_vtx = search_pos + vtx_start;
        let vtx_end = content[abs_vtx..]
            .find("</vertex>")
            .ok_or_else(|| KernelError::IoError("unclosed <vertex> tag".into()))?;
        let vtx_block = &content[abs_vtx..abs_vtx + vtx_end];

        let x = extract_tag_value(vtx_block, "x")?;
        let y = extract_tag_value(vtx_block, "y")?;
        let z = extract_tag_value(vtx_block, "z")?;

        vertices.push(Point3::new(x, y, z));
        normals.push(Vec3::Z);
        search_pos = abs_vtx + vtx_end + 9; // len("</vertex>")
    }

    if vertices.is_empty() {
        return Err(KernelError::IoError("no vertices found in AMF file".into()));
    }

    // Parse triangles: <triangle><v1>...<v2>...<v3>...</triangle>
    search_pos = 0;
    while let Some(tri_start) = content[search_pos..].find("<triangle>") {
        let abs_tri = search_pos + tri_start;
        let tri_end = content[abs_tri..]
            .find("</triangle>")
            .ok_or_else(|| KernelError::IoError("unclosed <triangle> tag".into()))?;
        let tri_block = &content[abs_tri..abs_tri + tri_end];

        let v1 = extract_tag_value_u32(tri_block, "v1")?;
        let v2 = extract_tag_value_u32(tri_block, "v2")?;
        let v3 = extract_tag_value_u32(tri_block, "v3")?;

        if (v1 as usize) < vertices.len()
            && (v2 as usize) < vertices.len()
            && (v3 as usize) < vertices.len()
        {
            indices.push([v1, v2, v3]);
        }
        search_pos = abs_tri + tri_end + 11; // len("</triangle>")
    }

    // Compute normals from triangles
    for tri in &indices {
        let v0 = vertices[tri[0] as usize];
        let v1 = vertices[tri[1] as usize];
        let v2 = vertices[tri[2] as usize];
        let e1 = v1 - v0;
        let e2 = v2 - v0;
        let n = e1.cross(e2).normalized().unwrap_or(Vec3::Z);
        for &idx in tri {
            normals[idx as usize] = n;
        }
    }

    Ok(Mesh {
        vertices,
        normals,
        indices,
    })
}

/// Export a Mesh to AMF XML format string.
pub fn export_amf(mesh: &Mesh) -> KernelResult<String> {
    if mesh.vertices.is_empty() {
        return Err(KernelError::IoError("empty mesh".into()));
    }

    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<amf unit=\"millimeter\" version=\"1.1\">\n");
    out.push_str("  <object id=\"0\">\n");
    out.push_str("    <mesh>\n");
    out.push_str("      <vertices>\n");

    for v in &mesh.vertices {
        out.push_str("        <vertex>\n");
        out.push_str("          <coordinates>\n");
        out.push_str(&format!("            <x>{:.6}</x>\n", v.x));
        out.push_str(&format!("            <y>{:.6}</y>\n", v.y));
        out.push_str(&format!("            <z>{:.6}</z>\n", v.z));
        out.push_str("          </coordinates>\n");
        out.push_str("        </vertex>\n");
    }

    out.push_str("      </vertices>\n");
    out.push_str("      <volume>\n");

    for tri in &mesh.indices {
        out.push_str("        <triangle>\n");
        out.push_str(&format!("          <v1>{}</v1>\n", tri[0]));
        out.push_str(&format!("          <v2>{}</v2>\n", tri[1]));
        out.push_str(&format!("          <v3>{}</v3>\n", tri[2]));
        out.push_str("        </triangle>\n");
    }

    out.push_str("      </volume>\n");
    out.push_str("    </mesh>\n");
    out.push_str("  </object>\n");
    out.push_str("</amf>\n");

    Ok(out)
}

/// Write a Mesh to an AMF file at the given path.
pub fn write_amf(path: &std::path::Path, mesh: &Mesh) -> KernelResult<()> {
    let content = export_amf(mesh)?;
    std::fs::write(path, content).map_err(|e| KernelError::IoError(e.to_string()))
}

/// Extract the text content of a simple XML tag as f64.
fn extract_tag_value(block: &str, tag: &str) -> KernelResult<f64> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = block
        .find(&open)
        .ok_or_else(|| KernelError::IoError(format!("missing <{}> tag", tag)))?;
    let after_open = start + open.len();
    let end = block[after_open..]
        .find(&close)
        .ok_or_else(|| KernelError::IoError(format!("missing </{}> tag", tag)))?;
    let val_str = block[after_open..after_open + end].trim();
    val_str
        .parse::<f64>()
        .map_err(|_| KernelError::IoError(format!("invalid number in <{}>: {}", tag, val_str)))
}

/// Extract the text content of a simple XML tag as u32.
fn extract_tag_value_u32(block: &str, tag: &str) -> KernelResult<u32> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = block
        .find(&open)
        .ok_or_else(|| KernelError::IoError(format!("missing <{}> tag", tag)))?;
    let after_open = start + open.len();
    let end = block[after_open..]
        .find(&close)
        .ok_or_else(|| KernelError::IoError(format!("missing </{}> tag", tag)))?;
    let val_str = block[after_open..after_open + end].trim();
    val_str
        .parse::<u32>()
        .map_err(|_| KernelError::IoError(format!("invalid index in <{}>: {}", tag, val_str)))
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
    fn test_export_amf() {
        let mesh = test_mesh();
        let amf = export_amf(&mesh).unwrap();
        assert!(amf.contains("<?xml"));
        assert!(amf.contains("<amf"));
        assert!(amf.contains("<vertex>"));
        assert!(amf.contains("<triangle>"));
        assert!(amf.contains("<v1>"));
    }

    #[test]
    fn test_import_amf() {
        let mesh = test_mesh();
        let amf = export_amf(&mesh).unwrap();
        let imported = import_amf(&amf).unwrap();
        assert_eq!(imported.vertices.len(), 4);
        assert_eq!(imported.indices.len(), 4);
    }

    #[test]
    fn test_roundtrip_amf() {
        let mesh = test_mesh();
        let amf = export_amf(&mesh).unwrap();
        let imported = import_amf(&amf).unwrap();
        for (orig, imp) in mesh.vertices.iter().zip(imported.vertices.iter()) {
            assert!((orig.x - imp.x).abs() < 1e-4);
            assert!((orig.y - imp.y).abs() < 1e-4);
            assert!((orig.z - imp.z).abs() < 1e-4);
        }
    }

    #[test]
    fn test_export_empty_amf() {
        let mesh = Mesh {
            vertices: vec![],
            normals: vec![],
            indices: vec![],
        };
        assert!(export_amf(&mesh).is_err());
    }

    #[test]
    fn test_import_invalid_amf() {
        assert!(import_amf("not an amf file").is_err());
    }

    #[test]
    fn test_amf_roundtrip_coordinates() {
        let mesh = Mesh {
            vertices: vec![
                Point3::new(1.5, 2.5, 3.5),
                Point3::new(4.0, 5.0, 6.0),
                Point3::new(7.0, 8.0, 9.0),
            ],
            normals: vec![Vec3::Z],
            indices: vec![[0, 1, 2]],
        };
        let amf = export_amf(&mesh).unwrap();
        let imported = import_amf(&amf).unwrap();
        assert!((imported.vertices[0].x - 1.5).abs() < 1e-4);
        assert!((imported.vertices[1].x - 4.0).abs() < 1e-4);
    }

    #[test]
    fn test_amf_contains_amf_unit() {
        let mesh = test_mesh();
        let amf = export_amf(&mesh).unwrap();
        assert!(amf.contains("unit="));
    }

    #[test]
    fn test_amf_roundtrip_indices() {
        let mesh = test_mesh();
        let amf = export_amf(&mesh).unwrap();
        let imported = import_amf(&amf).unwrap();
        assert_eq!(imported.indices.len(), 4);
        assert_eq!(imported.indices[0][0], 0);
    }

    #[test]
    fn test_amf_normals_computed_on_import() {
        let mesh = test_mesh();
        let amf = export_amf(&mesh).unwrap();
        let imported = import_amf(&amf).unwrap();
        assert!(!imported.normals.is_empty());
    }

    #[test]
    fn test_amf_single_triangle() {
        let mesh = Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            ],
            normals: vec![Vec3::Z],
            indices: vec![[0, 1, 2]],
        };
        let amf = export_amf(&mesh).unwrap();
        let imported = import_amf(&amf).unwrap();
        assert_eq!(imported.vertices.len(), 3);
        assert_eq!(imported.indices.len(), 1);
    }

    #[test]
    fn test_amf_object_tag() {
        let mesh = test_mesh();
        let amf = export_amf(&mesh).unwrap();
        assert!(amf.contains("<object>") || amf.contains("<object "));
    }

    #[test]
    fn test_amf_mesh_tag() {
        let mesh = test_mesh();
        let amf = export_amf(&mesh).unwrap();
        assert!(amf.contains("<mesh>") || amf.contains("<mesh "));
    }
}
