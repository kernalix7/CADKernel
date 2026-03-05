use std::io;
use std::path::Path;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};
use rayon::prelude::*;

use crate::tessellate::Mesh;

/// Renders the mesh as a Wavefront OBJ string.
pub fn write_obj(mesh: &Mesh) -> String {
    let vert_lines: Vec<String> = mesh
        .vertices
        .par_iter()
        .map(|v| format!("v {:.6} {:.6} {:.6}\n", v.x, v.y, v.z))
        .collect();

    let norm_lines: Vec<String> = mesh
        .normals
        .par_iter()
        .map(|n| format!("vn {:.6} {:.6} {:.6}\n", n.x, n.y, n.z))
        .collect();

    let face_lines: Vec<String> = mesh
        .indices
        .par_iter()
        .enumerate()
        .map(|(i, idx)| {
            let ni = i + 1;
            format!(
                "f {}//{} {}//{} {}//{}\n",
                idx[0] + 1,
                ni,
                idx[1] + 1,
                ni,
                idx[2] + 1,
                ni,
            )
        })
        .collect();

    let cap = 22
        + vert_lines.iter().map(|s| s.len()).sum::<usize>()
        + norm_lines.iter().map(|s| s.len()).sum::<usize>()
        + face_lines.iter().map(|s| s.len()).sum::<usize>();
    let mut out = String::with_capacity(cap);
    out.push_str("# CADKernel OBJ export\n");
    for s in &vert_lines {
        out.push_str(s);
    }
    for s in &norm_lines {
        out.push_str(s);
    }
    for s in &face_lines {
        out.push_str(s);
    }
    out
}

/// Writes a Wavefront OBJ file to disk.
pub fn export_obj(mesh: &Mesh, path: &Path) -> io::Result<()> {
    let content = write_obj(mesh);
    std::fs::write(path, content)
}

// ---------------------------------------------------------------------------
// Import (read / parse)
// ---------------------------------------------------------------------------

/// Extracts the vertex index from an OBJ face element token.
///
/// Handles `v`, `v/vt`, `v/vt/vn`, and `v//vn` formats.
/// Returns a 0-based vertex index.
fn parse_face_index(token: &str, vertex_count: usize) -> KernelResult<u32> {
    let idx_str = token.split('/').next().unwrap_or("");
    let idx: i64 = idx_str
        .parse()
        .map_err(|e| KernelError::IoError(format!("bad face index '{token}': {e}")))?;

    let zero_based = if idx > 0 {
        (idx - 1) as usize
    } else {
        // Negative indices count backwards from end
        vertex_count
            .checked_sub((-idx) as usize)
            .ok_or_else(|| KernelError::IoError(format!("face index out of range: {idx}")))?
    };

    if zero_based >= vertex_count {
        return Err(KernelError::IoError(format!(
            "face index {idx} out of range (vertex count: {vertex_count})"
        )));
    }
    Ok(zero_based as u32)
}

fn compute_normal(a: Point3, b: Point3, c: Point3) -> Vec3 {
    let u = b - a;
    let v = c - a;
    u.cross(v).normalized().unwrap_or(Vec3::Z)
}

/// Parses a Wavefront OBJ string into a [`Mesh`].
pub fn read_obj(input: &str) -> KernelResult<Mesh> {
    let lines: Vec<&str> = input.lines().collect();

    let v_lines: Vec<&str> = lines
        .iter()
        .filter(|l| l.trim().starts_with("v "))
        .copied()
        .collect();

    let vertices: Vec<Point3> = v_lines
        .par_iter()
        .map(|raw| {
            let parts: Vec<&str> = raw.split_whitespace().collect();
            if parts.len() < 4 {
                return Err(KernelError::IoError(format!(
                    "malformed vertex line: {raw}"
                )));
            }
            let x: f64 = parts[1]
                .parse()
                .map_err(|e| KernelError::IoError(format!("bad float in vertex: {e}")))?;
            let y: f64 = parts[2]
                .parse()
                .map_err(|e| KernelError::IoError(format!("bad float in vertex: {e}")))?;
            let z: f64 = parts[3]
                .parse()
                .map_err(|e| KernelError::IoError(format!("bad float in vertex: {e}")))?;
            Ok(Point3::new(x, y, z))
        })
        .collect::<KernelResult<Vec<_>>>()?;

    if vertices.is_empty() {
        return Err(KernelError::IoError("no vertices found in OBJ".into()));
    }

    let mut indices: Vec<[u32; 3]> = Vec::new();
    for raw_line in &lines {
        let line = raw_line.trim();
        if !line.starts_with("f ") {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            return Err(KernelError::IoError(format!(
                "face needs at least 3 vertices: {line}"
            )));
        }

        let face_indices: Vec<u32> = parts[1..]
            .iter()
            .map(|tok| parse_face_index(tok, vertices.len()))
            .collect::<KernelResult<Vec<_>>>()?;

        for i in 1..face_indices.len() - 1 {
            indices.push([face_indices[0], face_indices[i], face_indices[i + 1]]);
        }
    }

    let normals: Vec<Vec3> = indices
        .par_iter()
        .map(|idx| {
            compute_normal(
                vertices[idx[0] as usize],
                vertices[idx[1] as usize],
                vertices[idx[2] as usize],
            )
        })
        .collect();

    Ok(Mesh {
        vertices,
        normals,
        indices,
    })
}

/// Imports a Wavefront OBJ file from disk.
pub fn import_obj(path: &str) -> KernelResult<Mesh> {
    let text = std::fs::read_to_string(path)?;
    read_obj(&text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tessellate::Mesh;
    use cadkernel_math::{Point3, Vec3};

    #[test]
    fn test_obj_format() {
        let mesh = Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            ],
            normals: vec![Vec3::Z],
            indices: vec![[0, 1, 2]],
        };
        let obj = write_obj(&mesh);
        assert!(obj.starts_with("# CADKernel OBJ export\n"));
        assert!(obj.contains("v 0.000000 0.000000 0.000000\n"));
        assert!(obj.contains("vn 0.000000 0.000000 1.000000\n"));
        assert!(obj.contains("f 1//1 2//1 3//1\n"));
    }

    #[test]
    fn test_obj_vertex_count() {
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
        let obj = write_obj(&mesh);
        let v_count = obj.lines().filter(|l| l.starts_with("v ")).count();
        assert_eq!(v_count, 4);
        let f_count = obj.lines().filter(|l| l.starts_with("f ")).count();
        assert_eq!(f_count, 2);
    }

    // -----------------------------------------------------------------------
    // Import tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_roundtrip_obj() {
        let original = Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            ],
            normals: vec![Vec3::Z],
            indices: vec![[0, 1, 2]],
        };
        let obj_text = write_obj(&original);
        let parsed = read_obj(&obj_text).unwrap();
        assert_eq!(parsed.vertices.len(), original.vertices.len());
        assert_eq!(parsed.triangle_count(), original.triangle_count());
    }

    #[test]
    fn test_read_obj_plain_indices() {
        let input = "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
        let mesh = read_obj(input).unwrap();
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.triangle_count(), 1);
    }

    #[test]
    fn test_read_obj_with_texture_indices() {
        let input = "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1/1 2/2 3/3\n";
        let mesh = read_obj(input).unwrap();
        assert_eq!(mesh.triangle_count(), 1);
    }

    #[test]
    fn test_read_obj_with_normal_indices() {
        let input = "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1//1 2//2 3//3\n";
        let mesh = read_obj(input).unwrap();
        assert_eq!(mesh.triangle_count(), 1);
    }

    #[test]
    fn test_read_obj_with_full_indices() {
        let input = "v 0 0 0\nv 1 0 0\nv 0 1 0\nvn 0 0 1\nf 1/1/1 2/1/1 3/1/1\n";
        let mesh = read_obj(input).unwrap();
        assert_eq!(mesh.triangle_count(), 1);
    }

    #[test]
    fn test_read_obj_quad_triangulation() {
        let input = "\
v 0 0 0
v 1 0 0
v 1 1 0
v 0 1 0
f 1 2 3 4
";
        let mesh = read_obj(input).unwrap();
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.triangle_count(), 2);
    }

    #[test]
    fn test_read_obj_pentagon_triangulation() {
        let input = "\
v 0 0 0
v 1 0 0
v 1.5 0.5 0
v 1 1 0
v 0 1 0
f 1 2 3 4 5
";
        let mesh = read_obj(input).unwrap();
        assert_eq!(mesh.triangle_count(), 3);
    }

    #[test]
    fn test_read_obj_error_empty() {
        let result = read_obj("");
        assert!(result.is_err());
    }

    #[test]
    fn test_read_obj_error_bad_face_index() {
        let input = "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 99\n";
        let result = read_obj(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_import_obj_file() {
        let mesh = Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            ],
            normals: vec![Vec3::Z],
            indices: vec![[0, 1, 2]],
        };
        let obj_text = write_obj(&mesh);
        let dir = std::env::temp_dir();
        let path = dir.join("cadkernel_test_import.obj");
        std::fs::write(&path, &obj_text).unwrap();
        let parsed = import_obj(path.to_str().unwrap()).unwrap();
        assert_eq!(parsed.triangle_count(), 1);
        std::fs::remove_file(&path).ok();
    }
}
