//! glTF 2.0 file format import and export.
//!
//! Produces a single JSON file with embedded base64 buffer data (no separate
//! `.bin` file). Positions and normals are stored as `VEC3` float accessors;
//! indices are stored as unsigned 32-bit scalars.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};
use rayon::prelude::*;

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let full_chunks = data.len() / 3;
    let remainder = data.len() % 3;

    let full: Vec<[u8; 4]> = data[..full_chunks * 3]
        .par_chunks_exact(3)
        .map(|c| {
            let n = (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32;
            [
                CHARS[((n >> 18) & 63) as usize],
                CHARS[((n >> 12) & 63) as usize],
                CHARS[((n >> 6) & 63) as usize],
                CHARS[(n & 63) as usize],
            ]
        })
        .collect();

    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);
    for quad in &full {
        for &b in quad {
            result.push(b as char);
        }
    }

    if remainder > 0 {
        let tail = &data[full_chunks * 3..];
        let b = [
            tail[0] as u32,
            tail.get(1).copied().unwrap_or(0) as u32,
            0u32,
        ];
        let n = (b[0] << 16) | (b[1] << 8) | b[2];
        result.push(CHARS[((n >> 18) & 63) as usize] as char);
        result.push(CHARS[((n >> 12) & 63) as usize] as char);
        result.push(if remainder > 1 {
            CHARS[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        result.push('=');
    }

    result
}

/// Computes per-vertex normals by averaging face normals that share each vertex.
/// If normals are already per-vertex (same length as vertices), returns them directly.
fn compute_per_vertex_normals(
    vertices: &[Point3],
    normals: &[Vec3],
    indices: &[[u32; 3]],
) -> Vec<Vec3> {
    // If normals are already per-vertex, return them as-is
    if normals.len() == vertices.len() {
        return normals.to_vec();
    }
    let mut accum = vec![(0.0_f64, 0.0_f64, 0.0_f64); vertices.len()];
    for (tri_idx, tri) in indices.iter().enumerate() {
        let n = if tri_idx < normals.len() {
            normals[tri_idx]
        } else {
            Vec3::Z
        };
        for &vi in tri {
            let a = &mut accum[vi as usize];
            a.0 += n.x;
            a.1 += n.y;
            a.2 += n.z;
        }
    }
    accum
        .into_par_iter()
        .map(|(x, y, z)| Vec3::new(x, y, z).normalized().unwrap_or(Vec3::Z))
        .collect()
}

/// Writes a Mesh to glTF 2.0 JSON format with embedded base64 data.
///
/// Generates per-vertex normals by averaging face normals at shared vertices.
/// The buffer contains positions, normals, and indices as three buffer views.
/// Returns an error if the mesh is empty.
pub fn write_gltf(mesh: &super::Mesh) -> KernelResult<String> {
    if mesh.vertices.is_empty() || mesh.indices.is_empty() {
        return Err(KernelError::InvalidArgument(
            "cannot export empty mesh to glTF".into(),
        ));
    }

    let vertex_count = mesh.vertices.len();
    let index_count = mesh.indices.len() * 3;
    let per_vertex_normals =
        compute_per_vertex_normals(&mesh.vertices, &mesh.normals, &mesh.indices);

    let pos_f32: Vec<[f32; 3]> = mesh
        .vertices
        .par_iter()
        .map(|p| [p.x as f32, p.y as f32, p.z as f32])
        .collect();

    let (min_pos, max_pos) = pos_f32
        .par_iter()
        .fold(
            || ([f32::MAX; 3], [f32::MIN; 3]),
            |(mut mn, mut mx), c| {
                for i in 0..3 {
                    mn[i] = mn[i].min(c[i]);
                    mx[i] = mx[i].max(c[i]);
                }
                (mn, mx)
            },
        )
        .reduce(
            || ([f32::MAX; 3], [f32::MIN; 3]),
            |(mn1, mx1), (mn2, mx2)| {
                (
                    [mn1[0].min(mn2[0]), mn1[1].min(mn2[1]), mn1[2].min(mn2[2])],
                    [mx1[0].max(mx2[0]), mx1[1].max(mx2[1]), mx1[2].max(mx2[2])],
                )
            },
        );

    let mut pos_buf: Vec<u8> = Vec::with_capacity(vertex_count * 12);
    for c in &pos_f32 {
        for &v in c {
            pos_buf.extend_from_slice(&v.to_le_bytes());
        }
    }

    let norm_bytes: Vec<[u8; 12]> = per_vertex_normals
        .par_iter()
        .map(|n| {
            let mut b = [0u8; 12];
            b[0..4].copy_from_slice(&(n.x as f32).to_le_bytes());
            b[4..8].copy_from_slice(&(n.y as f32).to_le_bytes());
            b[8..12].copy_from_slice(&(n.z as f32).to_le_bytes());
            b
        })
        .collect();

    let mut norm_buf: Vec<u8> = Vec::with_capacity(vertex_count * 12);
    for b in &norm_bytes {
        norm_buf.extend_from_slice(b);
    }

    let idx_bytes: Vec<[u8; 12]> = mesh
        .indices
        .par_iter()
        .map(|tri| {
            let mut b = [0u8; 12];
            b[0..4].copy_from_slice(&tri[0].to_le_bytes());
            b[4..8].copy_from_slice(&tri[1].to_le_bytes());
            b[8..12].copy_from_slice(&tri[2].to_le_bytes());
            b
        })
        .collect();

    let mut idx_buf: Vec<u8> = Vec::with_capacity(index_count * 4);
    for b in &idx_bytes {
        idx_buf.extend_from_slice(b);
    }

    let pos_len = pos_buf.len();
    let norm_len = norm_buf.len();
    let idx_len = idx_buf.len();
    let total_len = pos_len + norm_len + idx_len;

    let mut buffer = Vec::with_capacity(total_len);
    buffer.extend_from_slice(&pos_buf);
    buffer.extend_from_slice(&norm_buf);
    buffer.extend_from_slice(&idx_buf);

    let b64 = base64_encode(&buffer);
    let uri = format!("data:application/octet-stream;base64,{b64}");

    let json = serde_json::json!({
        "asset": {
            "version": "2.0",
            "generator": "CADKernel"
        },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [{ "mesh": 0 }],
        "meshes": [{
            "primitives": [{
                "attributes": {
                    "POSITION": 0,
                    "NORMAL": 1
                },
                "indices": 2,
                "mode": 4
            }]
        }],
        "accessors": [
            {
                "bufferView": 0,
                "componentType": 5126,
                "count": vertex_count,
                "type": "VEC3",
                "min": min_pos,
                "max": max_pos
            },
            {
                "bufferView": 1,
                "componentType": 5126,
                "count": vertex_count,
                "type": "VEC3"
            },
            {
                "bufferView": 2,
                "componentType": 5125,
                "count": index_count,
                "type": "SCALAR"
            }
        ],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0, "byteLength": pos_len, "target": 34962 },
            { "buffer": 0, "byteOffset": pos_len, "byteLength": norm_len, "target": 34962 },
            { "buffer": 0, "byteOffset": pos_len + norm_len, "byteLength": idx_len, "target": 34963 }
        ],
        "buffers": [{
            "uri": uri,
            "byteLength": total_len
        }]
    });

    serde_json::to_string_pretty(&json).map_err(|e| KernelError::IoError(e.to_string()))
}

/// Exports a Mesh to a `.gltf` file at the given path.
pub fn export_gltf(mesh: &super::Mesh, path: &str) -> KernelResult<()> {
    let content = write_gltf(mesh)?;
    std::fs::write(path, content).map_err(|e| KernelError::IoError(e.to_string()))
}

// ---------------------------------------------------------------------------
// glTF import
// ---------------------------------------------------------------------------

fn base64_decode(input: &str) -> KernelResult<Vec<u8>> {
    const DECODE: [u8; 128] = {
        let mut t = [255u8; 128];
        let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut i = 0;
        while i < 64 {
            t[chars[i] as usize] = i as u8;
            i += 1;
        }
        t
    };
    let filtered: Vec<u8> = input
        .bytes()
        .filter(|&b| b != b'=' && b != b'\n' && b != b'\r' && b != b' ')
        .collect();
    let mut out = Vec::with_capacity(filtered.len() * 3 / 4);
    let chunks = filtered.chunks(4);
    for chunk in chunks {
        let a = *DECODE.get(chunk[0] as usize).unwrap_or(&255);
        let b = *DECODE
            .get(*chunk.get(1).unwrap_or(&0) as usize)
            .unwrap_or(&255);
        let c = *DECODE
            .get(*chunk.get(2).unwrap_or(&0) as usize)
            .unwrap_or(&0);
        let d = *DECODE
            .get(*chunk.get(3).unwrap_or(&0) as usize)
            .unwrap_or(&0);
        if a == 255 || b == 255 {
            return Err(KernelError::IoError("invalid base64".into()));
        }
        out.push((a << 2) | (b >> 4));
        if chunk.len() > 2 {
            out.push((b << 4) | (c >> 2));
        }
        if chunk.len() > 3 {
            out.push((c << 6) | d);
        }
    }
    Ok(out)
}

/// Imports a glTF 2.0 JSON string (with embedded base64 data) into a [`Mesh`].
///
/// Reads the first mesh primitive's POSITION accessor and optional NORMAL accessor.
/// Supports UNSIGNED_BYTE, UNSIGNED_SHORT, and UNSIGNED_INT index component types.
/// Only embedded base64 data URIs are supported (no external `.bin` files).
pub fn import_gltf(content: &str) -> KernelResult<super::Mesh> {
    let val: serde_json::Value = serde_json::from_str(content)
        .map_err(|e| KernelError::IoError(format!("glTF JSON parse error: {e}")))?;

    let accessors = val["accessors"]
        .as_array()
        .ok_or_else(|| KernelError::IoError("no accessors".into()))?;
    let buffer_views = val["bufferViews"]
        .as_array()
        .ok_or_else(|| KernelError::IoError("no bufferViews".into()))?;
    let buffers = val["buffers"]
        .as_array()
        .ok_or_else(|| KernelError::IoError("no buffers".into()))?;
    let meshes = val["meshes"]
        .as_array()
        .ok_or_else(|| KernelError::IoError("no meshes".into()))?;

    // Decode buffer
    let uri = buffers
        .first()
        .and_then(|b| b["uri"].as_str())
        .ok_or_else(|| KernelError::IoError("no buffer URI".into()))?;
    let b64_prefix = "data:application/octet-stream;base64,";
    let b64_data = if let Some(stripped) = uri.strip_prefix(b64_prefix) {
        stripped
    } else {
        return Err(KernelError::IoError(
            "only embedded base64 glTF supported".into(),
        ));
    };
    let buffer = base64_decode(b64_data)?;

    // Find the first mesh primitive
    let prim = meshes
        .first()
        .and_then(|m| m["primitives"].as_array())
        .and_then(|prims| prims.first())
        .ok_or_else(|| KernelError::IoError("no mesh primitive".into()))?;
    let pos_accessor_idx = prim["attributes"]["POSITION"]
        .as_u64()
        .ok_or_else(|| KernelError::IoError("no POSITION accessor".into()))?
        as usize;
    let idx_accessor_idx = prim["indices"].as_u64().map(|v| v as usize);

    let accessor = |idx: usize| -> KernelResult<&serde_json::Value> {
        accessors
            .get(idx)
            .ok_or_else(|| KernelError::IoError(format!("accessor index {idx} out of bounds")))
    };
    let buffer_view = |idx: usize| -> KernelResult<&serde_json::Value> {
        buffer_views
            .get(idx)
            .ok_or_else(|| KernelError::IoError(format!("bufferView index {idx} out of bounds")))
    };

    // Read positions
    let pos_acc = accessor(pos_accessor_idx)?;
    let pos_bv_idx = pos_acc["bufferView"]
        .as_u64()
        .ok_or_else(|| KernelError::IoError("POSITION accessor missing bufferView".into()))?
        as usize;
    let pos_count = pos_acc["count"]
        .as_u64()
        .ok_or_else(|| KernelError::IoError("POSITION accessor missing count".into()))?
        as usize;
    let pos_bv = buffer_view(pos_bv_idx)?;
    let pos_offset = pos_bv["byteOffset"].as_u64().unwrap_or(0) as usize
        + pos_acc["byteOffset"].as_u64().unwrap_or(0) as usize;
    let pos_component = pos_acc["componentType"].as_u64().unwrap_or(5126);

    let mut vertices = Vec::with_capacity(pos_count);
    if pos_component != 5126 {
        return Err(KernelError::IoError(format!(
            "unsupported POSITION component type {pos_component}"
        )));
    }
    let pos_len = pos_count
        .checked_mul(12)
        .ok_or_else(|| KernelError::IoError("POSITION accessor byte range overflow".into()))?;
    let pos_end = pos_offset
        .checked_add(pos_len)
        .ok_or_else(|| KernelError::IoError("POSITION accessor byte range overflow".into()))?;
    if pos_end > buffer.len() {
        return Err(KernelError::IoError(
            "POSITION accessor exceeds buffer length".into(),
        ));
    }
    for i in 0..pos_count {
        let base = pos_offset + i * 12;
        let x = f32::from_le_bytes([
            buffer[base],
            buffer[base + 1],
            buffer[base + 2],
            buffer[base + 3],
        ]);
        let y = f32::from_le_bytes([
            buffer[base + 4],
            buffer[base + 5],
            buffer[base + 6],
            buffer[base + 7],
        ]);
        let z = f32::from_le_bytes([
            buffer[base + 8],
            buffer[base + 9],
            buffer[base + 10],
            buffer[base + 11],
        ]);
        vertices.push(Point3::new(x as f64, y as f64, z as f64));
    }

    // Read normals (optional)
    let norm_acc_idx = prim["attributes"]["NORMAL"].as_u64().map(|v| v as usize);
    let mut normals = Vec::new();
    if let Some(ni) = norm_acc_idx {
        let norm_acc = accessor(ni)?;
        if let (Some(bv_idx), Some(count)) = (
            norm_acc["bufferView"].as_u64().map(|v| v as usize),
            norm_acc["count"].as_u64().map(|v| v as usize),
        ) {
            let norm_bv = buffer_view(bv_idx)?;
            let norm_offset = norm_bv["byteOffset"].as_u64().unwrap_or(0) as usize
                + norm_acc["byteOffset"].as_u64().unwrap_or(0) as usize;
            let norm_len = count.checked_mul(12).unwrap_or(0);
            let norm_end = norm_offset.checked_add(norm_len).unwrap_or(0);
            if norm_end <= buffer.len() {
                normals.reserve(count);
                for i in 0..count {
                    let base = norm_offset + i * 12;
                    let nx = f32::from_le_bytes([
                        buffer[base],
                        buffer[base + 1],
                        buffer[base + 2],
                        buffer[base + 3],
                    ]);
                    let ny = f32::from_le_bytes([
                        buffer[base + 4],
                        buffer[base + 5],
                        buffer[base + 6],
                        buffer[base + 7],
                    ]);
                    let nz = f32::from_le_bytes([
                        buffer[base + 8],
                        buffer[base + 9],
                        buffer[base + 10],
                        buffer[base + 11],
                    ]);
                    normals.push(Vec3::new(nx as f64, ny as f64, nz as f64));
                }
            }
        }
    }

    // Read indices
    let mut indices = Vec::new();
    if let Some(idx_ai) = idx_accessor_idx {
        let idx_acc = accessor(idx_ai)?;
        let idx_bv_idx = idx_acc["bufferView"]
            .as_u64()
            .ok_or_else(|| KernelError::IoError("index accessor missing bufferView".into()))?
            as usize;
        let idx_count = idx_acc["count"]
            .as_u64()
            .ok_or_else(|| KernelError::IoError("index accessor missing count".into()))?
            as usize;
        let idx_bv = buffer_view(idx_bv_idx)?;
        let idx_offset = idx_bv["byteOffset"].as_u64().unwrap_or(0) as usize
            + idx_acc["byteOffset"].as_u64().unwrap_or(0) as usize;
        let idx_component = idx_acc["componentType"].as_u64().unwrap_or(5125);

        let stride = match idx_component {
            5121 => 1usize, // UNSIGNED_BYTE
            5123 => 2,      // UNSIGNED_SHORT
            5125 => 4,      // UNSIGNED_INT
            _ => {
                return Err(KernelError::IoError(format!(
                    "unsupported index component type {idx_component}"
                )));
            }
        };

        if idx_count % 3 != 0 {
            return Err(KernelError::IoError(
                "index accessor count is not divisible by 3".into(),
            ));
        }

        let idx_len = idx_count
            .checked_mul(stride)
            .ok_or_else(|| KernelError::IoError("index accessor byte range overflow".into()))?;
        let idx_end = idx_offset
            .checked_add(idx_len)
            .ok_or_else(|| KernelError::IoError("index accessor byte range overflow".into()))?;
        if idx_end > buffer.len() {
            return Err(KernelError::IoError(
                "index accessor exceeds buffer length".into(),
            ));
        }

        let read_idx = |i: usize| -> u32 {
            let base = idx_offset + i * stride;
            match idx_component {
                5121 => buffer[base] as u32,
                5123 => u16::from_le_bytes([buffer[base], buffer[base + 1]]) as u32,
                _ => u32::from_le_bytes([
                    buffer[base],
                    buffer[base + 1],
                    buffer[base + 2],
                    buffer[base + 3],
                ]),
            }
        };

        for t in 0..idx_count / 3 {
            let tri = [read_idx(t * 3), read_idx(t * 3 + 1), read_idx(t * 3 + 2)];
            if tri.iter().any(|&idx| idx as usize >= vertices.len()) {
                return Err(KernelError::IoError(format!(
                    "index accessor references vertex outside POSITION range: {:?}",
                    tri
                )));
            }
            indices.push(tri);
        }
    } else {
        // Non-indexed: every 3 vertices form a triangle
        for t in 0..vertices.len() / 3 {
            indices.push([t as u32 * 3, t as u32 * 3 + 1, t as u32 * 3 + 2]);
        }
    }

    // Compute face normals if not provided
    if norm_acc_idx.is_none() || normals.is_empty() {
        normals = indices
            .iter()
            .map(|tri| {
                let a = vertices
                    .get(tri[0] as usize)
                    .copied()
                    .unwrap_or(Point3::ORIGIN);
                let b = vertices
                    .get(tri[1] as usize)
                    .copied()
                    .unwrap_or(Point3::ORIGIN);
                let c = vertices
                    .get(tri[2] as usize)
                    .copied()
                    .unwrap_or(Point3::ORIGIN);
                let ab = b - a;
                let ac = c - a;
                ab.cross(ac).normalized().unwrap_or(Vec3::Z)
            })
            .collect();
    }

    Ok(super::Mesh {
        vertices,
        normals,
        indices,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Mesh;

    fn make_cube_mesh() -> Mesh {
        let vertices = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
            Point3::new(1.0, 0.0, 1.0),
            Point3::new(1.0, 1.0, 1.0),
            Point3::new(0.0, 1.0, 1.0),
        ];
        let normals = vec![
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
        ];
        let indices = vec![
            [0, 2, 1],
            [0, 3, 2], // bottom
            [4, 5, 6],
            [4, 6, 7], // top
            [0, 1, 5],
            [0, 5, 4], // front
            [2, 3, 7],
            [2, 7, 6], // back
            [0, 4, 7],
            [0, 7, 3], // left
            [1, 2, 6],
            [1, 6, 5], // right
        ];
        Mesh {
            vertices,
            normals,
            indices,
        }
    }

    #[test]
    fn test_import_gltf_error_invalid_accessor_index() {
        let content = r#"{
            "buffers": [{"uri": "data:application/octet-stream;base64,AAAAAA=="}],
            "bufferViews": [{"buffer": 0, "byteOffset": 0, "byteLength": 4}],
            "accessors": [],
            "meshes": [{"primitives": [{"attributes": {"POSITION": 0}}]}]
        }"#;
        assert!(import_gltf(content).is_err());
    }

    #[test]
    fn test_gltf_basic() {
        let mesh = make_cube_mesh();
        let gltf = write_gltf(&mesh).unwrap();
        assert!(gltf.contains("asset"));
        assert!(gltf.contains("\"version\""));
        assert!(gltf.contains("2.0"));
        assert!(gltf.contains("generator"));
        assert!(gltf.contains("CADKernel"));
    }

    #[test]
    fn test_gltf_has_buffer() {
        let mesh = make_cube_mesh();
        let gltf = write_gltf(&mesh).unwrap();
        assert!(gltf.contains("data:application/octet-stream;base64,"));
    }

    #[test]
    fn test_gltf_counts() {
        let mesh = make_cube_mesh();
        let gltf = write_gltf(&mesh).unwrap();
        let val: serde_json::Value = serde_json::from_str(&gltf).unwrap();
        let accessors = val["accessors"].as_array().unwrap();

        let pos_count = accessors[0]["count"].as_u64().unwrap();
        assert_eq!(pos_count, mesh.vertices.len() as u64);

        let norm_count = accessors[1]["count"].as_u64().unwrap();
        assert_eq!(norm_count, mesh.vertices.len() as u64);

        let idx_count = accessors[2]["count"].as_u64().unwrap();
        assert_eq!(idx_count, (mesh.indices.len() * 3) as u64);
    }

    #[test]
    fn test_gltf_empty_error() {
        let mesh = Mesh::new();
        assert!(write_gltf(&mesh).is_err());
    }

    #[test]
    fn test_gltf_roundtrip() {
        let mesh = make_cube_mesh();
        let gltf = write_gltf(&mesh).unwrap();
        let imported = import_gltf(&gltf).unwrap();
        assert_eq!(imported.vertices.len(), mesh.vertices.len());
        assert_eq!(imported.indices.len(), mesh.indices.len());
    }

    // -----------------------------------------------------------------------
    // Edge case: JSON structure validation
    // -----------------------------------------------------------------------

    #[test]
    fn test_gltf_json_structure() {
        let mesh = make_cube_mesh();
        let gltf = write_gltf(&mesh).unwrap();
        let val: serde_json::Value = serde_json::from_str(&gltf).unwrap();

        // Required top-level fields
        assert!(val["asset"].is_object());
        assert!(val["meshes"].is_array());
        assert!(val["accessors"].is_array());
        assert!(val["bufferViews"].is_array());
        assert!(val["buffers"].is_array());
        assert!(val["nodes"].is_array());
        assert!(val["scenes"].is_array());

        // Asset version must be "2.0"
        assert_eq!(val["asset"]["version"].as_str().unwrap(), "2.0");

        // Buffer must have byteLength and URI
        let buf = &val["buffers"][0];
        assert!(buf["byteLength"].as_u64().unwrap() > 0);
        assert!(
            buf["uri"]
                .as_str()
                .unwrap()
                .starts_with("data:application/octet-stream;base64,")
        );
    }

    // -----------------------------------------------------------------------
    // Edge case: malformed input
    // -----------------------------------------------------------------------

    #[test]
    fn test_gltf_import_malformed_json() {
        let result = import_gltf("not valid json {{{");
        assert!(result.is_err());
    }

    #[test]
    fn test_gltf_import_empty_json() {
        let result = import_gltf("{}");
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Edge case: export-import-export consistency
    // -----------------------------------------------------------------------

    #[test]
    fn test_gltf_export_import_export_consistency() {
        let mesh = make_cube_mesh();
        let gltf1 = write_gltf(&mesh).unwrap();
        let reimported = import_gltf(&gltf1).unwrap();
        let gltf2 = write_gltf(&reimported).unwrap();

        let val1: serde_json::Value = serde_json::from_str(&gltf1).unwrap();
        let val2: serde_json::Value = serde_json::from_str(&gltf2).unwrap();

        let count1 = val1["accessors"][0]["count"].as_u64().unwrap();
        let count2 = val2["accessors"][0]["count"].as_u64().unwrap();
        assert_eq!(count1, count2);
    }
}
