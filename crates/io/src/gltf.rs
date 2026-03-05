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
fn compute_per_vertex_normals(
    vertices: &[Point3],
    normals: &[Vec3],
    indices: &[[u32; 3]],
) -> Vec<Vec3> {
    let mut accum = vec![(0.0_f64, 0.0_f64, 0.0_f64); vertices.len()];
    for (tri_idx, tri) in indices.iter().enumerate() {
        let n = normals[tri_idx];
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

/// Exports a Mesh to a .gltf file.
pub fn export_gltf(mesh: &super::Mesh, path: &str) -> KernelResult<()> {
    let content = write_gltf(mesh)?;
    std::fs::write(path, content).map_err(|e| KernelError::IoError(e.to_string()))
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
}
