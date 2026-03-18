use std::fmt::Write;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};

use crate::tessellate::Mesh;

/// Export mesh to DXF format (3DFACE entities).
pub fn export_dxf(mesh: &Mesh) -> KernelResult<String> {
    let mut out = String::new();
    out.push_str("0\nSECTION\n2\nHEADER\n0\nENDSEC\n");
    out.push_str("0\nSECTION\n2\nENTITIES\n");

    for idx in &mesh.indices {
        let p0 = mesh.vertices[idx[0] as usize];
        let p1 = mesh.vertices[idx[1] as usize];
        let p2 = mesh.vertices[idx[2] as usize];
        out.push_str("0\n3DFACE\n8\n0\n");
        write_dxf_point(&mut out, 10, p0);
        write_dxf_point(&mut out, 11, p1);
        write_dxf_point(&mut out, 12, p2);
        write_dxf_point(&mut out, 13, p2);
    }

    out.push_str("0\nENDSEC\n0\nEOF\n");
    Ok(out)
}

fn write_dxf_point(out: &mut String, group: i32, p: Point3) {
    let _ = write!(
        out,
        "{}\n{}\n{}\n{}\n{}\n{}\n",
        group,
        p.x,
        group + 10,
        p.y,
        group + 20,
        p.z
    );
}

/// Import DXF file, extracting 3DFACE entities into a mesh.
pub fn import_dxf(content: &str) -> KernelResult<Mesh> {
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    while i + 1 < lines.len() {
        let code = lines[i].trim();
        let value = lines[i + 1].trim();

        if code == "0" && value == "3DFACE" {
            let mut pts = [Point3::ORIGIN; 4];
            let mut j = i + 2;
            while j + 1 < lines.len() {
                let gc = lines[j].trim().parse::<i32>().unwrap_or(0);
                let gv = lines[j + 1].trim().parse::<f64>().unwrap_or(0.0);
                match gc {
                    10 => pts[0].x = gv,
                    20 => pts[0].y = gv,
                    30 => pts[0].z = gv,
                    11 => pts[1].x = gv,
                    21 => pts[1].y = gv,
                    31 => pts[1].z = gv,
                    12 => pts[2].x = gv,
                    22 => pts[2].y = gv,
                    32 => pts[2].z = gv,
                    13 => pts[3].x = gv,
                    23 => pts[3].y = gv,
                    33 => {
                        pts[3].z = gv;
                        break;
                    }
                    0 => break,
                    _ => {}
                }
                j += 2;
            }

            let base_idx = vertices.len() as u32;
            let v01 = pts[1] - pts[0];
            let v02 = pts[2] - pts[0];
            let n = v01.cross(v02);
            let n_len = n.length();
            let normal = if n_len > 1e-14 { n * (1.0 / n_len) } else { Vec3::Z };

            for pt in &pts[..3] {
                vertices.push(*pt);
                normals.push(normal);
            }
            indices.push([base_idx, base_idx + 1, base_idx + 2]);

            i = j + 2;
            continue;
        }
        i += 2;
    }

    Ok(Mesh {
        vertices,
        normals,
        indices,
    })
}

/// Write DXF string to file.
pub fn write_dxf(path: &str, content: &str) -> KernelResult<()> {
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
            normals: vec![Vec3::Z],
            indices: vec![[0, 1, 2]],
        }
    }

    #[test]
    fn test_dxf_export_contains_3dface() {
        let mesh = make_triangle_mesh();
        let dxf = export_dxf(&mesh).unwrap();
        assert!(dxf.contains("3DFACE"));
        assert!(dxf.contains("EOF"));
        assert!(dxf.contains("SECTION"));
    }

    #[test]
    fn test_dxf_roundtrip() {
        let mesh = make_triangle_mesh();
        let dxf = export_dxf(&mesh).unwrap();
        let imported = import_dxf(&dxf).unwrap();
        assert_eq!(imported.triangle_count(), 1);
        assert_eq!(imported.vertices.len(), 3);
    }

    #[test]
    fn test_dxf_roundtrip_multi() {
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
        let dxf = export_dxf(&mesh).unwrap();
        let imported = import_dxf(&dxf).unwrap();
        assert_eq!(imported.triangle_count(), 2);
    }

    #[test]
    fn test_import_empty_dxf() {
        let dxf = "0\nSECTION\n2\nHEADER\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nENDSEC\n0\nEOF\n";
        let mesh = import_dxf(dxf).unwrap();
        assert_eq!(mesh.triangle_count(), 0);
    }
}
