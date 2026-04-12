//! STL (STereoLithography) file format import and export.
//!
//! Supports both ASCII and binary STL variants with parallel serialization.
//! Binary reader validates sizes and caps allocations at 50 million triangles.
//! Vertices are deduplicated on import using quantized keys (0.1 mm tolerance)
//! to enable cross-face smooth normal computation.

use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};
use rayon::prelude::*;

use crate::tessellate::Mesh;

/// Renders the mesh as an ASCII STL string.
///
/// Facets are formatted in parallel using rayon. The output follows the
/// standard `solid <name> / endsolid <name>` envelope.
pub fn write_stl_ascii(mesh: &Mesh, name: &str) -> String {
    let tris = mesh.to_triangles();

    let facet_strs: Vec<String> = tris
        .par_iter()
        .map(|tri| {
            let n = tri.normal;
            let mut s = format!("  facet normal {:.6e} {:.6e} {:.6e}\n", n.x, n.y, n.z);
            s.push_str("    outer loop\n");
            for v in &tri.vertices {
                s.push_str(&format!(
                    "      vertex {:.6e} {:.6e} {:.6e}\n",
                    v.x, v.y, v.z
                ));
            }
            s.push_str("    endloop\n");
            s.push_str("  endfacet\n");
            s
        })
        .collect();

    let body_len: usize = facet_strs.iter().map(|s| s.len()).sum();
    let mut out = String::with_capacity(name.len() * 2 + 30 + body_len);
    out.push_str(&format!("solid {name}\n"));
    for s in &facet_strs {
        out.push_str(s);
    }
    out.push_str(&format!("endsolid {name}\n"));
    out
}

/// Renders the mesh as a binary STL byte buffer.
///
/// Returns an 80-byte header, 4-byte triangle count (little-endian u32),
/// followed by 50 bytes per triangle. Fails if the triangle count exceeds `u32::MAX`.
pub fn write_stl_binary(mesh: &Mesh) -> KernelResult<Vec<u8>> {
    let tris = mesh.to_triangles();
    if tris.len() > u32::MAX as usize {
        return Err(KernelError::IoError(format!(
            "triangle count {} exceeds u32::MAX for binary STL",
            tris.len()
        )));
    }
    let n_tris = tris.len() as u32;
    let total = 84 + n_tris as usize * 50;

    let tri_bytes: Vec<[u8; 50]> = tris
        .par_iter()
        .map(|tri| {
            let mut chunk = [0u8; 50];
            let nx = (tri.normal.x as f32).to_le_bytes();
            let ny = (tri.normal.y as f32).to_le_bytes();
            let nz = (tri.normal.z as f32).to_le_bytes();
            chunk[0..4].copy_from_slice(&nx);
            chunk[4..8].copy_from_slice(&ny);
            chunk[8..12].copy_from_slice(&nz);

            for (vi, v) in tri.vertices.iter().enumerate() {
                let off = 12 + vi * 12;
                chunk[off..off + 4].copy_from_slice(&(v.x as f32).to_le_bytes());
                chunk[off + 4..off + 8].copy_from_slice(&(v.y as f32).to_le_bytes());
                chunk[off + 8..off + 12].copy_from_slice(&(v.z as f32).to_le_bytes());
            }
            chunk
        })
        .collect();

    let mut buf = Vec::with_capacity(total);
    let header = b"CADKernel binary STL";
    buf.extend_from_slice(header);
    buf.extend_from_slice(&[0u8; 80 - 20]);
    buf.extend_from_slice(&n_tris.to_le_bytes());
    for chunk in &tri_bytes {
        buf.extend_from_slice(chunk);
    }
    Ok(buf)
}

/// Writes an ASCII STL file to disk.
pub fn export_stl_ascii(mesh: &Mesh, path: &Path, name: &str) -> io::Result<()> {
    let content = write_stl_ascii(mesh, name);
    std::fs::write(path, content)
}

/// Writes a binary STL file to disk.
pub fn export_stl_binary(mesh: &Mesh, path: &Path) -> KernelResult<()> {
    let data = write_stl_binary(mesh)?;
    let mut file = std::fs::File::create(path)?;
    file.write_all(&data)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Import (read / parse)
// ---------------------------------------------------------------------------

fn compute_normal(a: Point3, b: Point3, c: Point3) -> Vec3 {
    let u = b - a;
    let v = c - a;
    u.cross(v).normalized().unwrap_or(Vec3::Z)
}

fn quantize(v: f64) -> i64 {
    // 1e4 precision (0.1 mm tolerance) — matches float32 STL precision and
    // ensures coincident vertices are properly merged for smooth normals.
    (v * 1e4).round() as i64
}

fn vertex_key(pt: &Point3) -> (i64, i64, i64) {
    (quantize(pt.x), quantize(pt.y), quantize(pt.z))
}

struct VertexDedup {
    map: HashMap<(i64, i64, i64), u32>,
    vertices: Vec<Point3>,
}

impl VertexDedup {
    fn with_capacity(cap: usize) -> Self {
        Self {
            map: HashMap::with_capacity(cap),
            vertices: Vec::with_capacity(cap),
        }
    }

    fn insert(&mut self, pt: Point3) -> u32 {
        let key = vertex_key(&pt);
        if let Some(&idx) = self.map.get(&key) {
            idx
        } else {
            let idx = self.vertices.len() as u32;
            self.vertices.push(pt);
            self.map.insert(key, idx);
            idx
        }
    }
}

/// Parses an ASCII STL string into a [`Mesh`].
///
/// Vertices are deduplicated with 0.1 mm quantization tolerance. Normals are
/// recomputed from vertex positions (stored normals are ignored).
/// Returns an error if no triangles are found or vertex lines are malformed.
pub fn read_stl_ascii(input: &str) -> KernelResult<Mesh> {
    let raw_tris: Vec<[Point3; 3]> = parse_ascii_triangles(input)?;
    if raw_tris.is_empty() {
        return Err(KernelError::IoError(
            "no triangles found in ASCII STL".into(),
        ));
    }

    let normals: Vec<Vec3> = raw_tris
        .par_iter()
        .map(|t| compute_normal(t[0], t[1], t[2]))
        .collect();

    let mut dedup = VertexDedup::with_capacity(raw_tris.len());
    let indices: Vec<[u32; 3]> = raw_tris
        .iter()
        .map(|t| {
            let i0 = dedup.insert(t[0]);
            let i1 = dedup.insert(t[1]);
            let i2 = dedup.insert(t[2]);
            [i0, i1, i2]
        })
        .collect();

    Ok(Mesh {
        vertices: dedup.vertices,
        normals,
        indices,
    })
}

fn parse_ascii_triangles(input: &str) -> KernelResult<Vec<[Point3; 3]>> {
    let lines: Vec<&str> = input.lines().collect();
    let vertex_lines: Vec<(usize, &str)> = lines
        .par_iter()
        .enumerate()
        .filter(|(_, l)| l.trim().starts_with("vertex"))
        .map(|(i, l)| (i, *l))
        .collect();

    if vertex_lines.len() % 3 != 0 {
        return Err(KernelError::IoError(format!(
            "vertex count {} is not a multiple of 3",
            vertex_lines.len()
        )));
    }

    let parsed: Vec<KernelResult<Point3>> = vertex_lines
        .par_iter()
        .map(|(_, line)| {
            let trimmed = line.trim();
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() < 4 {
                return Err(KernelError::IoError(format!(
                    "malformed vertex line: {trimmed}"
                )));
            }
            let x: f64 = parts[1]
                .parse()
                .map_err(|e| KernelError::IoError(format!("bad float: {e}")))?;
            let y: f64 = parts[2]
                .parse()
                .map_err(|e| KernelError::IoError(format!("bad float: {e}")))?;
            let z: f64 = parts[3]
                .parse()
                .map_err(|e| KernelError::IoError(format!("bad float: {e}")))?;
            Ok(Point3::new(x, y, z))
        })
        .collect();

    let mut points = Vec::with_capacity(parsed.len());
    for r in parsed {
        points.push(r?);
    }

    let tris: Vec<[Point3; 3]> = points.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();

    Ok(tris)
}

fn read_f32_le(data: &[u8], off: usize) -> f64 {
    f32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]]) as f64
}

fn parse_binary_triangle(data: &[u8], base: usize) -> [Point3; 3] {
    let o = base + 12; // skip stored normal
    [
        Point3::new(
            read_f32_le(data, o),
            read_f32_le(data, o + 4),
            read_f32_le(data, o + 8),
        ),
        Point3::new(
            read_f32_le(data, o + 12),
            read_f32_le(data, o + 16),
            read_f32_le(data, o + 20),
        ),
        Point3::new(
            read_f32_le(data, o + 24),
            read_f32_le(data, o + 28),
            read_f32_le(data, o + 32),
        ),
    ]
}

/// Parses a binary STL byte slice into a [`Mesh`].
///
/// Validates header size, triangle count (capped at 50 million), and total
/// byte length. Vertices are deduplicated with 0.1 mm tolerance.
/// Normals are recomputed from vertex positions.
pub fn read_stl_binary(data: &[u8]) -> KernelResult<Mesh> {
    if data.len() < 84 {
        return Err(KernelError::IoError(
            "binary STL too short (< 84 bytes)".into(),
        ));
    }

    let tri_count = u32::from_le_bytes([data[80], data[81], data[82], data[83]]) as usize;
    const MAX_STL_TRIANGLES: usize = 50_000_000;
    if tri_count > MAX_STL_TRIANGLES {
        return Err(KernelError::IoError(format!(
            "binary STL triangle count {tri_count} exceeds limit {MAX_STL_TRIANGLES}"
        )));
    }
    let expected = 84 + tri_count * 50;
    if data.len() < expected {
        return Err(KernelError::IoError(format!(
            "binary STL truncated: expected {expected} bytes, got {}",
            data.len()
        )));
    }

    let offsets: Vec<usize> = (0..tri_count).map(|i| 84 + i * 50).collect();

    let raw_tris: Vec<[Point3; 3]> = offsets
        .par_iter()
        .map(|&off| parse_binary_triangle(data, off))
        .collect();

    let normals: Vec<Vec3> = raw_tris
        .par_iter()
        .map(|t| compute_normal(t[0], t[1], t[2]))
        .collect();

    let mut dedup = VertexDedup::with_capacity(tri_count);
    let indices: Vec<[u32; 3]> = raw_tris
        .iter()
        .map(|t| {
            let i0 = dedup.insert(t[0]);
            let i1 = dedup.insert(t[1]);
            let i2 = dedup.insert(t[2]);
            [i0, i1, i2]
        })
        .collect();

    Ok(Mesh {
        vertices: dedup.vertices,
        normals,
        indices,
    })
}

/// Imports an STL file from disk, auto-detecting ASCII vs binary format.
///
/// Heuristic: if the file starts with `"solid "` and contains `"facet"`, it
/// is tried as ASCII first, falling back to binary on parse failure.
pub fn import_stl(path: &str) -> KernelResult<Mesh> {
    let data = std::fs::read(path)?;

    let looks_ascii = data.starts_with(b"solid ") && data.windows(5).any(|w| w == b"facet");

    if looks_ascii {
        match String::from_utf8(data) {
            Ok(text) => {
                if let Ok(mesh) = read_stl_ascii(&text) {
                    return Ok(mesh);
                }
                return read_stl_binary(text.as_bytes());
            }
            Err(e) => {
                return read_stl_binary(&e.into_bytes());
            }
        }
    }

    read_stl_binary(&data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tessellate::Mesh;
    use cadkernel_math::{Point3, Vec3};

    fn make_single_triangle_mesh() -> Mesh {
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
    fn test_ascii_stl_format() {
        let mesh = make_single_triangle_mesh();
        let stl = write_stl_ascii(&mesh, "test");
        assert!(stl.starts_with("solid test\n"));
        assert!(stl.ends_with("endsolid test\n"));
        assert!(stl.contains("facet normal"));
        assert!(stl.contains("vertex"));
    }

    #[test]
    fn test_binary_stl_size() {
        let mesh = make_single_triangle_mesh();
        let data = write_stl_binary(&mesh).unwrap();
        // 80 header + 4 count + 1 * 50 = 134
        assert_eq!(data.len(), 134);
    }

    #[test]
    fn test_binary_stl_triangle_count() {
        let mesh = make_single_triangle_mesh();
        let data = write_stl_binary(&mesh).unwrap();
        let count = u32::from_le_bytes([data[80], data[81], data[82], data[83]]);
        assert_eq!(count, 1);
    }

    #[test]
    fn test_ascii_stl_multiple_triangles() {
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
        let stl = write_stl_ascii(&mesh, "quad");
        let facet_count = stl.matches("facet normal").count();
        assert_eq!(facet_count, 2);
    }

    // -----------------------------------------------------------------------
    // Import tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_roundtrip_ascii_stl() {
        let original = make_single_triangle_mesh();
        let stl_text = write_stl_ascii(&original, "roundtrip");
        let parsed = read_stl_ascii(&stl_text).unwrap();
        assert_eq!(parsed.triangle_count(), original.triangle_count());
    }

    #[test]
    fn test_roundtrip_binary_stl() {
        let original = make_single_triangle_mesh();
        let stl_bin = write_stl_binary(&original).unwrap();
        let parsed = read_stl_binary(&stl_bin).unwrap();
        assert_eq!(parsed.triangle_count(), original.triangle_count());
    }

    #[test]
    fn test_roundtrip_ascii_stl_multi() {
        let original = Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(1.0, 1.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            ],
            normals: vec![Vec3::Z, Vec3::Z],
            indices: vec![[0, 1, 2], [0, 2, 3]],
        };
        let stl_text = write_stl_ascii(&original, "quad");
        let parsed = read_stl_ascii(&stl_text).unwrap();
        assert_eq!(parsed.triangle_count(), 2);
        // Vertices should be deduplicated: 4 unique vertices
        assert_eq!(parsed.vertices.len(), 4);
    }

    #[test]
    fn test_roundtrip_binary_stl_multi() {
        let original = Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(1.0, 1.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            ],
            normals: vec![Vec3::Z, Vec3::Z],
            indices: vec![[0, 1, 2], [0, 2, 3]],
        };
        let stl_bin = write_stl_binary(&original).unwrap();
        let parsed = read_stl_binary(&stl_bin).unwrap();
        assert_eq!(parsed.triangle_count(), 2);
    }

    #[test]
    fn test_import_stl_ascii_auto() {
        let mesh = make_single_triangle_mesh();
        let stl_text = write_stl_ascii(&mesh, "auto");
        let dir = std::env::temp_dir();
        let path = dir.join("cadkernel_test_import_ascii.stl");
        std::fs::write(&path, &stl_text).unwrap();
        let parsed = import_stl(path.to_str().unwrap()).unwrap();
        assert_eq!(parsed.triangle_count(), 1);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_import_stl_binary_auto() {
        let mesh = make_single_triangle_mesh();
        let stl_bin = write_stl_binary(&mesh).unwrap();
        let dir = std::env::temp_dir();
        let path = dir.join("cadkernel_test_import_binary.stl");
        std::fs::write(&path, &stl_bin).unwrap();
        let parsed = import_stl(path.to_str().unwrap()).unwrap();
        assert_eq!(parsed.triangle_count(), 1);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_read_stl_ascii_error_empty() {
        let result = read_stl_ascii("");
        assert!(result.is_err());
    }

    #[test]
    fn test_read_stl_binary_error_too_short() {
        let result = read_stl_binary(&[0u8; 10]);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_stl_binary_error_truncated() {
        let mut data = vec![0u8; 84];
        // Claim 1 triangle but provide no triangle data
        data[80] = 1;
        let result = read_stl_binary(&data);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Edge case: empty mesh
    // -----------------------------------------------------------------------

    #[test]
    fn test_stl_empty_mesh_ascii() {
        let mesh = Mesh::new();
        let stl = write_stl_ascii(&mesh, "empty");
        assert!(stl.contains("solid empty"));
        assert!(stl.contains("endsolid empty"));
        // Parser returns error on empty STL (no triangles)
        assert!(read_stl_ascii(&stl).is_err());
    }

    #[test]
    fn test_stl_empty_mesh_binary() {
        let mesh = Mesh::new();
        let data = write_stl_binary(&mesh).unwrap();
        assert_eq!(data.len(), 84);
        let count = u32::from_le_bytes([data[80], data[81], data[82], data[83]]);
        assert_eq!(count, 0);
        let parsed = read_stl_binary(&data).unwrap();
        assert_eq!(parsed.triangle_count(), 0);
    }

    // -----------------------------------------------------------------------
    // Edge case: large mesh (sphere tessellation, > 10000 triangles)
    // -----------------------------------------------------------------------

    #[test]
    fn test_stl_large_mesh_roundtrip() {
        let segments = 128;
        let stacks = 80;
        let mut vertices = Vec::new();
        let mut normals = Vec::new();
        let mut indices = Vec::new();
        let mut idx = 0u32;

        for i in 0..stacks {
            let theta1 = std::f64::consts::PI * i as f64 / stacks as f64;
            let theta2 = std::f64::consts::PI * (i + 1) as f64 / stacks as f64;
            for j in 0..segments {
                let phi1 = 2.0 * std::f64::consts::PI * j as f64 / segments as f64;
                let phi2 = 2.0 * std::f64::consts::PI * (j + 1) as f64 / segments as f64;

                let p00 = Point3::new(
                    theta1.sin() * phi1.cos(),
                    theta1.sin() * phi1.sin(),
                    theta1.cos(),
                );
                let p10 = Point3::new(
                    theta2.sin() * phi1.cos(),
                    theta2.sin() * phi1.sin(),
                    theta2.cos(),
                );
                let p11 = Point3::new(
                    theta2.sin() * phi2.cos(),
                    theta2.sin() * phi2.sin(),
                    theta2.cos(),
                );
                let p01 = Point3::new(
                    theta1.sin() * phi2.cos(),
                    theta1.sin() * phi2.sin(),
                    theta1.cos(),
                );

                let n = Vec3::new(
                    (p00.x + p10.x + p11.x) / 3.0,
                    (p00.y + p10.y + p11.y) / 3.0,
                    (p00.z + p10.z + p11.z) / 3.0,
                );

                vertices.push(p00);
                vertices.push(p10);
                vertices.push(p11);
                normals.push(n);
                indices.push([idx, idx + 1, idx + 2]);
                idx += 3;

                vertices.push(p00);
                vertices.push(p11);
                vertices.push(p01);
                normals.push(n);
                indices.push([idx, idx + 1, idx + 2]);
                idx += 3;
            }
        }

        let mesh = Mesh { vertices, normals, indices };
        assert!(mesh.triangle_count() > 10000);

        // Binary round-trip
        let bin_data = write_stl_binary(&mesh).unwrap();
        let parsed_bin = read_stl_binary(&bin_data).unwrap();
        assert_eq!(parsed_bin.triangle_count(), mesh.triangle_count());

        // ASCII round-trip
        let ascii_data = write_stl_ascii(&mesh, "large_sphere");
        let parsed_ascii = read_stl_ascii(&ascii_data).unwrap();
        assert_eq!(parsed_ascii.triangle_count(), mesh.triangle_count());
    }

    // -----------------------------------------------------------------------
    // Edge case: binary vs ASCII cross-round-trip
    // -----------------------------------------------------------------------

    #[test]
    fn test_stl_binary_ascii_cross_roundtrip() {
        let mesh = Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.5, 1.0, 0.0),
                Point3::new(0.5, 0.5, 1.0),
            ],
            normals: vec![Vec3::Z, Vec3::new(0.0, -1.0, 0.0)],
            indices: vec![[0, 1, 2], [0, 1, 3]],
        };

        // Export binary, import, export ASCII, import, compare
        let bin = write_stl_binary(&mesh).unwrap();
        let from_bin = read_stl_binary(&bin).unwrap();
        let ascii = write_stl_ascii(&from_bin, "cross");
        let from_ascii = read_stl_ascii(&ascii).unwrap();

        assert_eq!(from_ascii.triangle_count(), mesh.triangle_count());
    }

    // -----------------------------------------------------------------------
    // Edge case: malformed ASCII
    // -----------------------------------------------------------------------

    #[test]
    fn test_stl_ascii_malformed_no_endsolid() {
        let bad = "solid test\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\n";
        let result = read_stl_ascii(bad);
        // Should still parse the triangle even without endsolid
        assert!(result.is_ok());
        assert_eq!(result.unwrap().triangle_count(), 1);
    }

    #[test]
    fn test_stl_ascii_malformed_garbage() {
        let bad = "not a valid stl file at all";
        let result = read_stl_ascii(bad);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Export-import-export consistency
    // -----------------------------------------------------------------------

    #[test]
    fn test_stl_export_import_export_consistency() {
        let mesh = make_single_triangle_mesh();
        let ascii1 = write_stl_ascii(&mesh, "consistency");
        let reimported = read_stl_ascii(&ascii1).unwrap();
        let ascii2 = write_stl_ascii(&reimported, "consistency");
        assert_eq!(ascii1, ascii2);
    }
}
