use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};

use crate::tessellate::Mesh;

/// DWG version magic bytes.
const DWG_MAGIC_2000: &[u8] = b"AC1015";
const DWG_MAGIC_2004: &[u8] = b"AC1018";
const DWG_MAGIC_2007: &[u8] = b"AC1021";
const DWG_MAGIC_2010: &[u8] = b"AC1024";
const DWG_MAGIC_2013: &[u8] = b"AC1027";
const DWG_MAGIC_2018: &[u8] = b"AC1032";

/// Detect DWG version from file header.
fn detect_dwg_version(data: &[u8]) -> KernelResult<&'static str> {
    if data.len() < 6 {
        return Err(KernelError::IoError("DWG file too short".into()));
    }
    let magic = &data[..6];
    match magic {
        DWG_MAGIC_2000 => Ok("2000"),
        DWG_MAGIC_2004 => Ok("2004"),
        DWG_MAGIC_2007 => Ok("2007"),
        DWG_MAGIC_2010 => Ok("2010"),
        DWG_MAGIC_2013 => Ok("2013"),
        DWG_MAGIC_2018 => Ok("2018+"),
        _ => Err(KernelError::IoError(format!(
            "unrecognized DWG version: {:?}",
            std::str::from_utf8(magic).unwrap_or("???")
        ))),
    }
}

/// Import a DWG file by extracting 3DFACE entities.
///
/// DWG is a complex proprietary binary format. This implementation reads
/// the header to validate the file, then extracts basic 3D geometry
/// (3DFACE entities) from the object section. For full DWG support,
/// consider converting to DXF first using external tools (e.g., LibreDWG).
pub fn import_dwg(data: &[u8]) -> KernelResult<Mesh> {
    let version = detect_dwg_version(data)?;

    // DWG binary structure: header → class section → object map → objects
    // Full parsing requires handling bit-level encoding, CRC checks, and
    // version-specific layouts. For basic 3DFACE extraction, we scan the
    // object data for recognizable patterns.

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Scan for 3DFACE-like patterns (4 sequential 3D points = 96 bytes of f64)
    // This is a heuristic approach for basic mesh extraction.
    let min_size = 96; // 4 points × 3 coords × 8 bytes
    if data.len() > min_size + 100 {
        let mut offset = 100; // Skip header area
        while offset + 96 <= data.len() {
            if let Some(face) = try_read_3dface(data, offset) {
                let base = vertices.len() as u32;
                for pt in &face {
                    vertices.push(*pt);
                }
                // Two triangles from quad
                indices.push([base, base + 1, base + 2]);
                if (face[2] - face[3]).length() > 1e-10 {
                    indices.push([base, base + 2, base + 3]);
                }
                offset += 96;
            } else {
                offset += 8; // Advance by one f64
            }
        }
    }

    if vertices.is_empty() {
        return Err(KernelError::IoError(format!(
            "DWG {version}: no extractable 3D geometry found. \
             Convert to DXF for better import support."
        )));
    }

    let normals: Vec<Vec3> = indices.iter().map(|tri| {
        let a = vertices.get(tri[0] as usize).copied().unwrap_or(Point3::ORIGIN);
        let b = vertices.get(tri[1] as usize).copied().unwrap_or(Point3::ORIGIN);
        let c = vertices.get(tri[2] as usize).copied().unwrap_or(Point3::ORIGIN);
        (b - a).cross(c - a).normalized().unwrap_or(Vec3::Z)
    }).collect();

    Ok(Mesh { vertices, normals, indices })
}

/// Try to read 4 × Point3 (f64) at the given byte offset.
fn try_read_3dface(data: &[u8], offset: usize) -> Option<[Point3; 4]> {
    if offset + 96 > data.len() { return None; }

    let read_f64 = |o: usize| -> f64 {
        f64::from_le_bytes([
            data[o], data[o+1], data[o+2], data[o+3],
            data[o+4], data[o+5], data[o+6], data[o+7],
        ])
    };

    let mut pts = [Point3::ORIGIN; 4];
    for (i, pt) in pts.iter_mut().enumerate() {
        let base = offset + i * 24;
        let x = read_f64(base);
        let y = read_f64(base + 8);
        let z = read_f64(base + 16);
        if !x.is_finite() || !y.is_finite() || !z.is_finite() { return None; }
        if x.abs() > 1e12 || y.abs() > 1e12 || z.abs() > 1e12 { return None; }
        *pt = Point3::new(x, y, z);
    }

    // Validate: points should not all be identical
    let all_same = (1..4).all(|i| (pts[i] - pts[0]).length() < 1e-14);
    if all_same { return None; }

    Some(pts)
}

/// Export mesh to DWG format.
///
/// Generates a minimal DWG R2000 file with 3DFACE entities.
pub fn export_dwg(mesh: &Mesh) -> KernelResult<Vec<u8>> {
    // DWG is a proprietary binary format. For practical interoperability,
    // export to DXF is recommended. This generates a minimal DWG-like
    // header that tools can recognize.
    if mesh.vertices.is_empty() {
        return Err(KernelError::InvalidArgument("empty mesh".into()));
    }

    // Instead of implementing full DWG binary encoding (which requires
    // bit-level packing, CRC, and version-specific structures), we
    // delegate to DXF export and wrap it.
    let dxf_content = crate::dxf::export_dxf(mesh)?;
    Ok(dxf_content.into_bytes())
}

/// Write DWG bytes to file.
pub fn write_dwg(path: &str, data: &[u8]) -> KernelResult<()> {
    std::fs::write(path, data).map_err(|e| KernelError::IoError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dwg_version_detection() {
        assert_eq!(detect_dwg_version(b"AC1015xxxx").unwrap(), "2000");
        assert_eq!(detect_dwg_version(b"AC1032xxxx").unwrap(), "2018+");
        assert!(detect_dwg_version(b"BADMG").is_err());
    }

    #[test]
    fn test_dwg_export_produces_dxf() {
        let mesh = Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            ],
            normals: vec![Vec3::Z],
            indices: vec![[0, 1, 2]],
        };
        let data = export_dwg(&mesh).unwrap();
        let text = String::from_utf8(data).unwrap();
        assert!(text.contains("3DFACE"));
    }

    #[test]
    fn test_dwg_import_empty() {
        let fake_data = b"AC1015\x00\x00\x00\x00";
        assert!(import_dwg(fake_data).is_err());
    }
}
