use std::collections::HashMap;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};

use crate::Mesh;

/// Compute the triangle normal from three vertices (unnormalized cross product).
fn triangle_normal(a: Point3, b: Point3, c: Point3) -> Vec3 {
    let ab = b - a;
    let ac = c - a;
    ab.cross(ac)
}

/// Compute the unit normal for a triangle, falling back to zero if degenerate.
fn unit_triangle_normal(a: Point3, b: Point3, c: Point3) -> Vec3 {
    triangle_normal(a, b, c)
        .normalized()
        .unwrap_or(Vec3::ZERO)
}

/// Recompute per-triangle normals for all faces in the mesh.
fn recompute_normals(mesh: &mut Mesh) {
    mesh.normals.clear();
    for tri in &mesh.indices {
        let a = mesh.vertices[tri[0] as usize];
        let b = mesh.vertices[tri[1] as usize];
        let c = mesh.vertices[tri[2] as usize];
        mesh.normals.push(unit_triangle_normal(a, b, c));
    }
}

/// Canonical edge key: (min, max) of the two vertex indices.
fn edge_key(a: u32, b: u32) -> (u32, u32) {
    if a < b { (a, b) } else { (b, a) }
}

/// Reduce triangle count by iteratively collapsing the shortest edges.
///
/// `target_ratio` must be in `(0.0, 1.0)` and represents the fraction of
/// original triangles to keep. The algorithm repeatedly finds the shortest
/// edge, merges its two vertices to their midpoint, and removes degenerate
/// triangles.
pub fn decimate_mesh(mesh: &Mesh, target_ratio: f64) -> KernelResult<Mesh> {
    if target_ratio <= 0.0 || target_ratio >= 1.0 {
        return Err(KernelError::InvalidArgument(
            "target_ratio must be in (0.0, 1.0)".to_string(),
        ));
    }
    if mesh.indices.is_empty() {
        return Ok(mesh.clone());
    }

    let target_count = ((mesh.indices.len() as f64) * target_ratio).ceil() as usize;
    let target_count = target_count.max(1);

    let mut vertices = mesh.vertices.clone();
    let mut indices = mesh.indices.clone();

    // Mapping from old vertex index to its current representative.
    let mut remap: Vec<u32> = (0..vertices.len() as u32).collect();

    // Resolve chains: find the final representative for a vertex.
    let resolve = |remap: &[u32], mut v: u32| -> u32 {
        while remap[v as usize] != v {
            v = remap[v as usize];
        }
        v
    };

    while indices.len() > target_count {
        // Build edge set with lengths.
        let mut edges: HashMap<(u32, u32), f64> = HashMap::new();
        for tri in &indices {
            let v = [
                resolve(&remap, tri[0]),
                resolve(&remap, tri[1]),
                resolve(&remap, tri[2]),
            ];
            for &(a, b) in &[(v[0], v[1]), (v[1], v[2]), (v[2], v[0])] {
                if a == b {
                    continue;
                }
                let key = edge_key(a, b);
                edges.entry(key).or_insert_with(|| {
                    let d = vertices[a as usize] - vertices[b as usize];
                    d.length_squared()
                });
            }
        }

        if edges.is_empty() {
            break;
        }

        // Find shortest edge.
        let (&(va, vb), _) = edges
            .iter()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap();

        // Collapse: merge vb into va at midpoint.
        let mid = vertices[va as usize].midpoint(vertices[vb as usize]);
        vertices[va as usize] = mid;
        remap[vb as usize] = va;

        // Path-compress remap for vb chain.
        let mut compress_idx = vb;
        while remap[compress_idx as usize] != compress_idx {
            let next = remap[compress_idx as usize];
            remap[compress_idx as usize] = va;
            compress_idx = next;
        }

        // Update indices and remove degenerate triangles.
        indices.retain(|tri| {
            let v0 = resolve(&remap, tri[0]);
            let v1 = resolve(&remap, tri[1]);
            let v2 = resolve(&remap, tri[2]);
            v0 != v1 && v1 != v2 && v2 != v0
        });
    }

    // Compact vertices: only keep referenced ones.
    let mut used: HashMap<u32, u32> = HashMap::new();
    let mut new_verts = Vec::new();
    let mut new_indices = Vec::new();

    for tri in &indices {
        let v = [
            resolve(&remap, tri[0]),
            resolve(&remap, tri[1]),
            resolve(&remap, tri[2]),
        ];
        let mut new_tri = [0u32; 3];
        for (i, &vi) in v.iter().enumerate() {
            let next_id = used.len() as u32;
            let id = *used.entry(vi).or_insert_with(|| {
                new_verts.push(vertices[vi as usize]);
                next_id
            });
            new_tri[i] = id;
        }
        new_indices.push(new_tri);
    }

    let mut result = Mesh {
        vertices: new_verts,
        normals: Vec::new(),
        indices: new_indices,
    };
    recompute_normals(&mut result);
    Ok(result)
}

/// Find boundary edges (used by exactly one triangle) and fill each hole with
/// fan triangulation from the loop centroid.
///
/// Boundary edge loops are chained together; for each loop a centroid vertex is
/// added and fan triangles connect consecutive boundary vertices to it.
pub fn fill_holes(mesh: &Mesh) -> KernelResult<Mesh> {
    // Count how many triangles reference each directed half-edge.
    let mut half_edge_count: HashMap<(u32, u32), u32> = HashMap::new();
    for tri in &mesh.indices {
        for &(a, b) in &[(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
            *half_edge_count.entry((a, b)).or_insert(0) += 1;
        }
    }

    // A boundary half-edge has no matching opposite half-edge.
    let mut boundary_next: HashMap<u32, u32> = HashMap::new();
    for &(a, b) in half_edge_count.keys() {
        if !half_edge_count.contains_key(&(b, a)) {
            // (a, b) is on the boundary — the hole boundary runs in the
            // opposite direction, so the hole edge is (b, a).
            boundary_next.insert(b, a);
        }
    }

    if boundary_next.is_empty() {
        return Ok(mesh.clone());
    }

    let mut result = mesh.clone();
    let mut visited: HashMap<u32, bool> = HashMap::new();

    // Chain boundary edges into loops and fill each one.
    for &start in boundary_next.keys() {
        if visited.contains_key(&start) {
            continue;
        }

        let mut loop_verts: Vec<u32> = Vec::new();
        let mut cur = start;
        loop {
            if visited.contains_key(&cur) {
                break;
            }
            visited.insert(cur, true);
            loop_verts.push(cur);
            match boundary_next.get(&cur) {
                Some(&next) => cur = next,
                None => break,
            }
        }

        if loop_verts.len() < 3 {
            continue;
        }

        // Compute centroid of the loop.
        let mut cx = 0.0;
        let mut cy = 0.0;
        let mut cz = 0.0;
        let n = loop_verts.len() as f64;
        for &vi in &loop_verts {
            let p = result.vertices[vi as usize];
            cx += p.x;
            cy += p.y;
            cz += p.z;
        }
        let centroid = Point3::new(cx / n, cy / n, cz / n);
        let centroid_idx = result.vertices.len() as u32;
        result.vertices.push(centroid);

        // Fan triangulation.
        for i in 0..loop_verts.len() {
            let a = loop_verts[i];
            let b = loop_verts[(i + 1) % loop_verts.len()];
            let tri = [a, b, centroid_idx];
            let normal = unit_triangle_normal(
                result.vertices[a as usize],
                result.vertices[b as usize],
                centroid,
            );
            result.indices.push(tri);
            result.normals.push(normal);
        }
    }

    Ok(result)
}

/// Compute per-vertex mean curvature using the cotangent-weighted discrete
/// Laplace-Beltrami operator.
///
/// Returns one curvature value per vertex (same length as `mesh.vertices`).
pub fn compute_curvature(mesh: &Mesh) -> KernelResult<Vec<f64>> {
    let n = mesh.vertices.len();
    if n == 0 {
        return Ok(Vec::new());
    }

    let mut laplacian = vec![Vec3::ZERO; n];
    let mut area = vec![0.0_f64; n];

    for tri in &mesh.indices {
        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;

        let p0 = mesh.vertices[i0];
        let p1 = mesh.vertices[i1];
        let p2 = mesh.vertices[i2];

        let e01 = p1 - p0;
        let e02 = p2 - p0;
        let e12 = p2 - p1;

        // Cotangent of angle at each vertex.
        // cot(angle) = dot(a, b) / |cross(a, b)|
        let cross0 = e01.cross(e02);
        let cross0_len = cross0.length();
        if cross0_len < 1e-30 {
            continue;
        }

        let cot0 = e01.dot(e02) / cross0_len;

        let e10 = p0 - p1;
        let cross1 = e10.cross(e12);
        let cross1_len = cross1.length();
        if cross1_len < 1e-30 {
            continue;
        }
        let cot1 = e10.dot(e12) / cross1_len;

        let e20 = p0 - p2;
        let e21 = p1 - p2;
        let cross2 = e20.cross(e21);
        let cross2_len = cross2.length();
        if cross2_len < 1e-30 {
            continue;
        }
        let cot2 = e20.dot(e21) / cross2_len;

        // Triangle area (Voronoi mixed area approximation: 1/3 per vertex).
        let tri_area = cross0_len * 0.5;
        let vertex_area = tri_area / 3.0;
        area[i0] += vertex_area;
        area[i1] += vertex_area;
        area[i2] += vertex_area;

        // Cotangent-weighted Laplacian contributions.
        // Edge (i1, i2): opposite angle is at i0, weight = cot0.
        let d12 = p2 - p1;
        laplacian[i1] += d12 * cot0;
        laplacian[i2] += (d12 * -1.0) * cot0;

        // Edge (i0, i2): opposite angle is at i1, weight = cot1.
        let d02 = p2 - p0;
        laplacian[i0] += d02 * cot1;
        laplacian[i2] += (d02 * -1.0) * cot1;

        // Edge (i0, i1): opposite angle is at i2, weight = cot2.
        let d01 = p1 - p0;
        laplacian[i0] += d01 * cot2;
        laplacian[i1] += (d01 * -1.0) * cot2;
    }

    let mut curvature = vec![0.0_f64; n];
    for i in 0..n {
        if area[i] > 1e-30 {
            // The cotangent Laplacian gives delta(x) = 2*H*n, so
            // mean curvature H = |delta(x)| / (2 * (2*area)) = |L| / (4*A).
            let hn = laplacian[i] * (1.0 / (4.0 * area[i]));
            curvature[i] = hn.length();
        }
    }

    Ok(curvature)
}

/// Subdivide each triangle into four by inserting edge midpoints (midpoint
/// subdivision).
///
/// Each original triangle is split into four sub-triangles. Edge midpoints are
/// shared across adjacent triangles. Per-triangle normals are recomputed.
pub fn subdivide_mesh(mesh: &Mesh) -> KernelResult<Mesh> {
    if mesh.indices.is_empty() {
        return Ok(mesh.clone());
    }

    let mut vertices = mesh.vertices.clone();
    let mut edge_midpoints: HashMap<(u32, u32), u32> = HashMap::new();

    let mut get_midpoint =
        |verts: &mut Vec<Point3>, a: u32, b: u32| -> u32 {
            let key = edge_key(a, b);
            if let Some(&idx) = edge_midpoints.get(&key) {
                return idx;
            }
            let mid = verts[a as usize].midpoint(verts[b as usize]);
            let idx = verts.len() as u32;
            verts.push(mid);
            edge_midpoints.insert(key, idx);
            idx
        };

    let mut new_indices = Vec::with_capacity(mesh.indices.len() * 4);

    for tri in &mesh.indices {
        let v0 = tri[0];
        let v1 = tri[1];
        let v2 = tri[2];

        let m01 = get_midpoint(&mut vertices, v0, v1);
        let m12 = get_midpoint(&mut vertices, v1, v2);
        let m20 = get_midpoint(&mut vertices, v2, v0);

        new_indices.push([v0, m01, m20]);
        new_indices.push([m01, v1, m12]);
        new_indices.push([m20, m12, v2]);
        new_indices.push([m01, m12, m20]);
    }

    let mut result = Mesh {
        vertices,
        normals: Vec::new(),
        indices: new_indices,
    };
    recompute_normals(&mut result);
    Ok(result)
}

/// Reverse the winding order of all triangles and negate their normals.
pub fn flip_normals(mesh: &Mesh) -> Mesh {
    let mut result = mesh.clone();
    for tri in &mut result.indices {
        tri.swap(1, 2);
    }
    for n in &mut result.normals {
        *n = -*n;
    }
    result
}

/// Laplacian smoothing of mesh vertices.
///
/// Iteratively moves each vertex toward the average of its neighbors.
/// `factor` controls how far each vertex moves (0.0 = no movement, 1.0 = full average).
pub fn smooth_mesh(mesh: &Mesh, iterations: usize, factor: f64) -> Mesh {
    let mut vertices = mesh.vertices.clone();
    let n = vertices.len();
    if n == 0 || iterations == 0 {
        return mesh.clone();
    }

    // Build adjacency: for each vertex, collect connected vertex indices.
    let mut adjacency: Vec<Vec<u32>> = vec![Vec::new(); n];
    for tri in &mesh.indices {
        for &(a, b) in &[(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
            if !adjacency[a as usize].contains(&b) {
                adjacency[a as usize].push(b);
            }
            if !adjacency[b as usize].contains(&a) {
                adjacency[b as usize].push(a);
            }
        }
    }

    for _ in 0..iterations {
        let prev = vertices.clone();
        for i in 0..n {
            let neighbors = &adjacency[i];
            if neighbors.is_empty() {
                continue;
            }
            let count = neighbors.len() as f64;
            let mut avg_x = 0.0;
            let mut avg_y = 0.0;
            let mut avg_z = 0.0;
            for &ni in neighbors {
                avg_x += prev[ni as usize].x;
                avg_y += prev[ni as usize].y;
                avg_z += prev[ni as usize].z;
            }
            avg_x /= count;
            avg_y /= count;
            avg_z /= count;
            vertices[i].x = prev[i].x + factor * (avg_x - prev[i].x);
            vertices[i].y = prev[i].y + factor * (avg_y - prev[i].y);
            vertices[i].z = prev[i].z + factor * (avg_z - prev[i].z);
        }
    }

    let mut result = Mesh {
        vertices,
        normals: Vec::new(),
        indices: mesh.indices.clone(),
    };
    recompute_normals(&mut result);
    result
}

/// Mesh boolean union (simple triangle-level merge, not exact CSG).
///
/// Combines vertices and indices from both meshes with appropriate index offset.
pub fn mesh_boolean_union(a: &Mesh, b: &Mesh) -> Mesh {
    let offset = a.vertices.len() as u32;
    let mut vertices = a.vertices.clone();
    vertices.extend_from_slice(&b.vertices);
    let mut normals = a.normals.clone();
    normals.extend_from_slice(&b.normals);
    let mut indices = a.indices.clone();
    for idx in &b.indices {
        indices.push([idx[0] + offset, idx[1] + offset, idx[2] + offset]);
    }
    Mesh {
        vertices,
        normals,
        indices,
    }
}

/// Cut mesh with a plane, keeping the side where the normal points.
///
/// Triangles fully on the positive side (where `dot(v - plane_point, plane_normal) >= 0`)
/// are kept. Triangles crossing the plane are clipped.
pub fn cut_mesh_with_plane(mesh: &Mesh, plane_point: Point3, plane_normal: Vec3) -> Mesh {
    let normal = plane_normal.normalized().unwrap_or(Vec3::Z);
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    for idx in &mesh.indices {
        let pts = [
            mesh.vertices[idx[0] as usize],
            mesh.vertices[idx[1] as usize],
            mesh.vertices[idx[2] as usize],
        ];
        let dists: [f64; 3] = [
            signed_distance(pts[0], plane_point, normal),
            signed_distance(pts[1], plane_point, normal),
            signed_distance(pts[2], plane_point, normal),
        ];

        let above = [dists[0] >= 0.0, dists[1] >= 0.0, dists[2] >= 0.0];
        let count_above = above.iter().filter(|&&x| x).count();

        if count_above == 3 {
            // Fully above: keep as-is
            let base = vertices.len() as u32;
            for &p in &pts {
                vertices.push(p);
            }
            let n = compute_triangle_normal(pts[0], pts[1], pts[2]);
            normals.push(n);
            indices.push([base, base + 1, base + 2]);
        } else if count_above == 0 {
            // Fully below: discard
        } else {
            // Partial: clip triangle
            clip_triangle_to_plane(
                &pts,
                &dists,
                &above,
                &mut vertices,
                &mut normals,
                &mut indices,
            );
        }
    }

    Mesh {
        vertices,
        normals,
        indices,
    }
}

fn signed_distance(p: Point3, plane_point: Point3, normal: Vec3) -> f64 {
    let d = p - plane_point;
    d.x * normal.x + d.y * normal.y + d.z * normal.z
}

fn compute_triangle_normal(a: Point3, b: Point3, c: Point3) -> Vec3 {
    let ab = b - a;
    let ac = c - a;
    ab.cross(ac).normalized().unwrap_or(Vec3::Z)
}

fn lerp_point(a: Point3, b: Point3, t: f64) -> Point3 {
    Point3::new(
        a.x + t * (b.x - a.x),
        a.y + t * (b.y - a.y),
        a.z + t * (b.z - a.z),
    )
}

fn clip_triangle_to_plane(
    pts: &[Point3; 3],
    dists: &[f64; 3],
    above: &[bool; 3],
    vertices: &mut Vec<Point3>,
    normals: &mut Vec<Vec3>,
    indices: &mut Vec<[u32; 3]>,
) {
    let count_above = above.iter().filter(|&&x| x).count();

    if count_above == 2 {
        // Two vertices above: produce a quad (2 triangles)
        let below_idx = above.iter().position(|&x| !x).unwrap();
        let a_idx = (below_idx + 1) % 3;
        let b_idx = (below_idx + 2) % 3;

        let t_a = dists[a_idx] / (dists[a_idx] - dists[below_idx]);
        let t_b = dists[b_idx] / (dists[b_idx] - dists[below_idx]);
        let clip_a = lerp_point(pts[a_idx], pts[below_idx], t_a);
        let clip_b = lerp_point(pts[b_idx], pts[below_idx], t_b);

        let base = vertices.len() as u32;
        vertices.push(pts[a_idx]);
        vertices.push(pts[b_idx]);
        vertices.push(clip_a);
        vertices.push(clip_b);

        let n = compute_triangle_normal(pts[a_idx], pts[b_idx], clip_a);
        normals.push(n);
        indices.push([base, base + 1, base + 2]);

        let n2 = compute_triangle_normal(pts[b_idx], clip_b, clip_a);
        normals.push(n2);
        indices.push([base + 1, base + 3, base + 2]);
    } else if count_above == 1 {
        // One vertex above: produce one triangle
        let above_idx = above.iter().position(|&x| x).unwrap();
        let b_idx = (above_idx + 1) % 3;
        let c_idx = (above_idx + 2) % 3;

        let t_b = dists[above_idx] / (dists[above_idx] - dists[b_idx]);
        let t_c = dists[above_idx] / (dists[above_idx] - dists[c_idx]);
        let clip_b = lerp_point(pts[above_idx], pts[b_idx], t_b);
        let clip_c = lerp_point(pts[above_idx], pts[c_idx], t_c);

        let base = vertices.len() as u32;
        vertices.push(pts[above_idx]);
        vertices.push(clip_b);
        vertices.push(clip_c);

        let n = compute_triangle_normal(pts[above_idx], clip_b, clip_c);
        normals.push(n);
        indices.push([base, base + 1, base + 2]);
    }
}

/// Extract cross-section contour from mesh intersection with plane.
///
/// Returns line segments forming the cross-section contour.
pub fn mesh_section_from_plane(
    mesh: &Mesh,
    plane_point: Point3,
    plane_normal: Vec3,
) -> Vec<[Point3; 2]> {
    let normal = plane_normal.normalized().unwrap_or(Vec3::Z);
    let mut segments = Vec::new();

    for idx in &mesh.indices {
        let pts = [
            mesh.vertices[idx[0] as usize],
            mesh.vertices[idx[1] as usize],
            mesh.vertices[idx[2] as usize],
        ];
        let dists: [f64; 3] = [
            signed_distance(pts[0], plane_point, normal),
            signed_distance(pts[1], plane_point, normal),
            signed_distance(pts[2], plane_point, normal),
        ];

        let mut crossings = Vec::new();
        for i in 0..3 {
            let j = (i + 1) % 3;
            if dists[i].abs() < 1e-12 {
                crossings.push(pts[i]);
            } else if dists[i] * dists[j] < 0.0 {
                let t = dists[i] / (dists[i] - dists[j]);
                crossings.push(lerp_point(pts[i], pts[j], t));
            }
        }

        // Deduplicate crossings that are too close
        crossings.dedup_by(|a, b| a.distance_to(*b) < 1e-12);

        if crossings.len() >= 2 {
            segments.push([crossings[0], crossings[1]]);
        }
    }

    segments
}

/// Split mesh into disconnected components using union-find.
pub fn split_mesh_by_components(mesh: &Mesh) -> Vec<Mesh> {
    let n = mesh.vertices.len();
    if n == 0 || mesh.indices.is_empty() {
        return vec![mesh.clone()];
    }

    // Union-Find
    let mut parent: Vec<usize> = (0..n).collect();
    let mut rank: Vec<usize> = vec![0; n];

    fn find(parent: &mut [usize], x: usize) -> usize {
        if parent[x] != x {
            parent[x] = find(parent, parent[x]);
        }
        parent[x]
    }

    fn union(parent: &mut [usize], rank: &mut [usize], a: usize, b: usize) {
        let ra = find(parent, a);
        let rb = find(parent, b);
        if ra == rb {
            return;
        }
        if rank[ra] < rank[rb] {
            parent[ra] = rb;
        } else if rank[ra] > rank[rb] {
            parent[rb] = ra;
        } else {
            parent[rb] = ra;
            rank[ra] += 1;
        }
    }

    // Union vertices connected by triangles
    for tri in &mesh.indices {
        union(&mut parent, &mut rank, tri[0] as usize, tri[1] as usize);
        union(&mut parent, &mut rank, tri[1] as usize, tri[2] as usize);
    }

    // Group triangles by component root
    let mut components: HashMap<usize, Vec<usize>> = HashMap::new();
    for (ti, tri) in mesh.indices.iter().enumerate() {
        let root = find(&mut parent, tri[0] as usize);
        components.entry(root).or_default().push(ti);
    }

    if components.len() <= 1 {
        return vec![mesh.clone()];
    }

    let mut result = Vec::with_capacity(components.len());
    for tri_indices in components.values() {
        let mut vert_map: HashMap<u32, u32> = HashMap::new();
        let mut new_verts = Vec::new();
        let mut new_normals = Vec::new();
        let mut new_indices = Vec::new();

        for &ti in tri_indices {
            let tri = mesh.indices[ti];
            let mut new_tri = [0u32; 3];
            for (k, &vi) in tri.iter().enumerate() {
                let next_id = new_verts.len() as u32;
                let id = *vert_map.entry(vi).or_insert_with(|| {
                    new_verts.push(mesh.vertices[vi as usize]);
                    next_id
                });
                new_tri[k] = id;
            }
            new_indices.push(new_tri);
            if ti < mesh.normals.len() {
                new_normals.push(mesh.normals[ti]);
            }
        }

        result.push(Mesh {
            vertices: new_verts,
            normals: new_normals,
            indices: new_indices,
        });
    }

    result
}

/// Make normals consistent (all outward-facing) via BFS winding propagation.
///
/// Starting from the first triangle, propagates consistent winding order to
/// all connected triangles using shared edges.
pub fn harmonize_normals(mesh: &Mesh) -> Mesh {
    if mesh.indices.is_empty() {
        return mesh.clone();
    }

    let n_tris = mesh.indices.len();
    let mut indices = mesh.indices.clone();

    // Build half-edge to triangle adjacency
    let mut he_to_tri: HashMap<(u32, u32), usize> = HashMap::new();
    for (ti, tri) in indices.iter().enumerate() {
        for k in 0..3 {
            let a = tri[k];
            let b = tri[(k + 1) % 3];
            he_to_tri.insert((a, b), ti);
        }
    }

    let mut visited = vec![false; n_tris];
    let mut queue = std::collections::VecDeque::new();
    visited[0] = true;
    queue.push_back(0);

    while let Some(ti) = queue.pop_front() {
        let tri = indices[ti];
        for k in 0..3 {
            let a = tri[k];
            let b = tri[(k + 1) % 3];
            // The opposite half-edge in a consistently wound neighbor is (b, a).
            // If neighbor has (a, b) as a half-edge, it has inconsistent winding.
            if let Some(&ni) = he_to_tri.get(&(b, a)) {
                if !visited[ni] {
                    visited[ni] = true;
                    queue.push_back(ni);
                    // Consistent: neighbor shares edge (b, a), which is correct.
                }
            }
            if let Some(&ni) = he_to_tri.get(&(a, b)) {
                if ni != ti && !visited[ni] {
                    visited[ni] = true;
                    // Inconsistent: neighbor has same half-edge direction -> flip it
                    indices[ni].swap(1, 2);
                    // Rebuild adjacency for the flipped triangle
                    let flipped = indices[ni];
                    for j in 0..3 {
                        let fa = flipped[j];
                        let fb = flipped[(j + 1) % 3];
                        he_to_tri.insert((fa, fb), ni);
                    }
                    queue.push_back(ni);
                }
            }
        }
    }

    let mut result = Mesh {
        vertices: mesh.vertices.clone(),
        normals: Vec::new(),
        indices,
    };
    recompute_normals(&mut result);
    result
}

/// Check if mesh is watertight (every edge shared by exactly 2 triangles).
pub fn check_mesh_watertight(mesh: &Mesh) -> bool {
    if mesh.indices.is_empty() {
        return false;
    }

    let mut edge_count: HashMap<(u32, u32), u32> = HashMap::new();
    for tri in &mesh.indices {
        for k in 0..3 {
            let a = tri[k];
            let b = tri[(k + 1) % 3];
            let key = edge_key(a, b);
            *edge_count.entry(key).or_insert(0) += 1;
        }
    }

    edge_count.values().all(|&c| c == 2)
}

/// Mesh boolean intersection: keep only triangles from both meshes that overlap
/// in bounding-box space. Approximate CSG via AABB overlap filtering.
pub fn mesh_boolean_intersection(a: &Mesh, b: &Mesh) -> Mesh {
    let a_bb = mesh_bounding_box(a);
    let b_bb = mesh_bounding_box(b);

    // Intersection AABB
    let min_x = a_bb.0.x.max(b_bb.0.x);
    let min_y = a_bb.0.y.max(b_bb.0.y);
    let min_z = a_bb.0.z.max(b_bb.0.z);
    let max_x = a_bb.1.x.min(b_bb.1.x);
    let max_y = a_bb.1.y.min(b_bb.1.y);
    let max_z = a_bb.1.z.min(b_bb.1.z);

    if min_x > max_x || min_y > max_y || min_z > max_z {
        return Mesh {
            vertices: Vec::new(),
            normals: Vec::new(),
            indices: Vec::new(),
        };
    }

    let intersect_min = Point3::new(min_x, min_y, min_z);
    let intersect_max = Point3::new(max_x, max_y, max_z);

    let filter_mesh = |mesh: &Mesh| -> Mesh {
        let mut vertices = Vec::new();
        let mut normals = Vec::new();
        let mut indices = Vec::new();
        for (ti, tri) in mesh.indices.iter().enumerate() {
            let centroid = triangle_centroid(
                mesh.vertices[tri[0] as usize],
                mesh.vertices[tri[1] as usize],
                mesh.vertices[tri[2] as usize],
            );
            if point_in_aabb(centroid, intersect_min, intersect_max) {
                let base = vertices.len() as u32;
                vertices.push(mesh.vertices[tri[0] as usize]);
                vertices.push(mesh.vertices[tri[1] as usize]);
                vertices.push(mesh.vertices[tri[2] as usize]);
                indices.push([base, base + 1, base + 2]);
                if ti < mesh.normals.len() {
                    normals.push(mesh.normals[ti]);
                }
            }
        }
        Mesh {
            vertices,
            normals,
            indices,
        }
    };

    let filtered_a = filter_mesh(a);
    let filtered_b = filter_mesh(b);
    mesh_boolean_union(&filtered_a, &filtered_b)
}

/// Mesh boolean difference: keep triangles from A that don't overlap with B's bounding box.
pub fn mesh_boolean_difference(a: &Mesh, b: &Mesh) -> Mesh {
    let b_bb = mesh_bounding_box(b);

    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    for (ti, tri) in a.indices.iter().enumerate() {
        let centroid = triangle_centroid(
            a.vertices[tri[0] as usize],
            a.vertices[tri[1] as usize],
            a.vertices[tri[2] as usize],
        );
        if !point_in_aabb(centroid, b_bb.0, b_bb.1) {
            let base = vertices.len() as u32;
            vertices.push(a.vertices[tri[0] as usize]);
            vertices.push(a.vertices[tri[1] as usize]);
            vertices.push(a.vertices[tri[2] as usize]);
            indices.push([base, base + 1, base + 2]);
            if ti < a.normals.len() {
                normals.push(a.normals[ti]);
            }
        }
    }

    Mesh {
        vertices,
        normals,
        indices,
    }
}

fn triangle_centroid(a: Point3, b: Point3, c: Point3) -> Point3 {
    Point3::new(
        (a.x + b.x + c.x) / 3.0,
        (a.y + b.y + c.y) / 3.0,
        (a.z + b.z + c.z) / 3.0,
    )
}

fn point_in_aabb(p: Point3, min: Point3, max: Point3) -> bool {
    p.x >= min.x && p.x <= max.x && p.y >= min.y && p.y <= max.y && p.z >= min.z && p.z <= max.z
}

/// Regular polyhedra type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegularSolidType {
    Tetrahedron,
    Cube,
    Octahedron,
    Dodecahedron,
    Icosahedron,
}

/// Create a regular solid (Platonic solid) mesh centered at origin with given size.
pub fn regular_solid(solid_type: RegularSolidType, size: f64) -> KernelResult<Mesh> {
    if size <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "size must be positive".to_string(),
        ));
    }

    let (raw_verts, raw_faces) = match solid_type {
        RegularSolidType::Tetrahedron => {
            let s = size;
            let verts = vec![
                [s, s, s],
                [-s, -s, s],
                [-s, s, -s],
                [s, -s, -s],
            ];
            let faces: Vec<Vec<usize>> =
                vec![vec![0, 1, 2], vec![0, 2, 3], vec![0, 3, 1], vec![1, 3, 2]];
            (verts, faces)
        }
        RegularSolidType::Cube => {
            let s = size * 0.5;
            let verts = vec![
                [-s, -s, -s], [s, -s, -s], [s, s, -s], [-s, s, -s],
                [-s, -s, s], [s, -s, s], [s, s, s], [-s, s, s],
            ];
            let faces: Vec<Vec<usize>> = vec![
                vec![0, 2, 1], vec![0, 3, 2],
                vec![4, 5, 6], vec![4, 6, 7],
                vec![0, 1, 5], vec![0, 5, 4],
                vec![2, 3, 7], vec![2, 7, 6],
                vec![0, 4, 7], vec![0, 7, 3],
                vec![1, 2, 6], vec![1, 6, 5],
            ];
            (verts, faces)
        }
        RegularSolidType::Octahedron => {
            let s = size;
            let verts = vec![
                [0.0, 0.0, s],
                [s, 0.0, 0.0],
                [0.0, s, 0.0],
                [-s, 0.0, 0.0],
                [0.0, -s, 0.0],
                [0.0, 0.0, -s],
            ];
            let faces: Vec<Vec<usize>> = vec![
                vec![0, 1, 2], vec![0, 2, 3], vec![0, 3, 4], vec![0, 4, 1],
                vec![5, 2, 1], vec![5, 3, 2], vec![5, 4, 3], vec![5, 1, 4],
            ];
            (verts, faces)
        }
        RegularSolidType::Dodecahedron => {
            let phi = (1.0 + 5.0_f64.sqrt()) / 2.0;
            let s = size * 0.5;
            let a = s;
            let b = s / phi;
            let c = s * phi;
            #[rustfmt::skip]
            let verts = vec![
                [a, a, a], [a, a, -a], [a, -a, a], [a, -a, -a],
                [-a, a, a], [-a, a, -a], [-a, -a, a], [-a, -a, -a],
                [0.0, b, c], [0.0, b, -c], [0.0, -b, c], [0.0, -b, -c],
                [b, c, 0.0], [b, -c, 0.0], [-b, c, 0.0], [-b, -c, 0.0],
                [c, 0.0, b], [c, 0.0, -b], [-c, 0.0, b], [-c, 0.0, -b],
            ];
            // Dodecahedron has 12 pentagonal faces; triangulate each into 3 triangles
            let pentagons: Vec<Vec<usize>> = vec![
                vec![0, 16, 2, 10, 8], vec![0, 8, 4, 14, 12],
                vec![16, 17, 1, 12, 0], vec![1, 9, 11, 3, 17],
                vec![1, 12, 14, 5, 9], vec![2, 13, 15, 6, 10],
                vec![13, 3, 17, 16, 2], vec![3, 11, 7, 15, 13],
                vec![4, 8, 10, 6, 18], vec![14, 4, 18, 19, 5],
                vec![5, 19, 7, 11, 9], vec![15, 7, 19, 18, 6],
            ];
            let mut faces: Vec<Vec<usize>> = Vec::new();
            for pent in &pentagons {
                for i in 1..pent.len() - 1 {
                    faces.push(vec![pent[0], pent[i], pent[i + 1]]);
                }
            }
            (verts, faces)
        }
        RegularSolidType::Icosahedron => {
            let phi = (1.0 + 5.0_f64.sqrt()) / 2.0;
            let s = size * 0.5;
            let a = s;
            let b = s * phi;
            let verts = vec![
                [-a, b, 0.0], [a, b, 0.0], [-a, -b, 0.0], [a, -b, 0.0],
                [0.0, -a, b], [0.0, a, b], [0.0, -a, -b], [0.0, a, -b],
                [b, 0.0, -a], [b, 0.0, a], [-b, 0.0, -a], [-b, 0.0, a],
            ];
            let faces: Vec<Vec<usize>> = vec![
                vec![0, 11, 5], vec![0, 5, 1], vec![0, 1, 7], vec![0, 7, 10], vec![0, 10, 11],
                vec![1, 5, 9], vec![5, 11, 4], vec![11, 10, 2], vec![10, 7, 6], vec![7, 1, 8],
                vec![3, 9, 4], vec![3, 4, 2], vec![3, 2, 6], vec![3, 6, 8], vec![3, 8, 9],
                vec![4, 9, 5], vec![2, 4, 11], vec![6, 2, 10], vec![8, 6, 7], vec![9, 8, 1],
            ];
            (verts, faces)
        }
    };

    let vertices: Vec<Point3> = raw_verts
        .iter()
        .map(|v| Point3::new(v[0], v[1], v[2]))
        .collect();
    let indices: Vec<[u32; 3]> = raw_faces
        .iter()
        .map(|f| [f[0] as u32, f[1] as u32, f[2] as u32])
        .collect();
    let mut mesh = Mesh {
        vertices,
        normals: Vec::new(),
        indices,
    };
    recompute_normals(&mut mesh);
    Ok(mesh)
}

/// Per-face information: area, normal, center.
#[derive(Debug, Clone)]
pub struct FaceInfo {
    /// Face index.
    pub index: usize,
    /// Face area.
    pub area: f64,
    /// Face normal (unit vector).
    pub normal: Vec3,
    /// Face centroid.
    pub center: Point3,
}

/// Compute per-face information (area, normal, center) for all triangles.
pub fn face_info(mesh: &Mesh) -> Vec<FaceInfo> {
    mesh.indices
        .iter()
        .enumerate()
        .map(|(i, tri)| {
            let a = mesh.vertices[tri[0] as usize];
            let b = mesh.vertices[tri[1] as usize];
            let c = mesh.vertices[tri[2] as usize];
            let cross = triangle_normal(a, b, c);
            let area = cross.length() * 0.5;
            let normal = cross.normalized().unwrap_or(Vec3::Z);
            let center = triangle_centroid(a, b, c);
            FaceInfo {
                index: i,
                area,
                normal,
                center,
            }
        })
        .collect()
}

/// Mesh bounding box information.
#[derive(Debug, Clone)]
pub struct MeshBoundingBox {
    pub min: Point3,
    pub max: Point3,
    pub center: Point3,
    pub size: Vec3,
    pub diagonal: f64,
}

/// Compute bounding box of a mesh (min, max corners).
fn mesh_bounding_box(mesh: &Mesh) -> (Point3, Point3) {
    if mesh.vertices.is_empty() {
        return (Point3::ORIGIN, Point3::ORIGIN);
    }
    let mut min = mesh.vertices[0];
    let mut max = mesh.vertices[0];
    for v in &mesh.vertices {
        min.x = min.x.min(v.x);
        min.y = min.y.min(v.y);
        min.z = min.z.min(v.z);
        max.x = max.x.max(v.x);
        max.y = max.y.max(v.y);
        max.z = max.z.max(v.z);
    }
    (min, max)
}

/// Compute full bounding box information for a mesh.
pub fn bounding_box_info(mesh: &Mesh) -> MeshBoundingBox {
    let (min, max) = mesh_bounding_box(mesh);
    let size = Vec3::new(max.x - min.x, max.y - min.y, max.z - min.z);
    let center = Point3::new(
        (min.x + max.x) * 0.5,
        (min.y + max.y) * 0.5,
        (min.z + max.z) * 0.5,
    );
    let diagonal = size.length();
    MeshBoundingBox {
        min,
        max,
        center,
        size,
        diagonal,
    }
}

/// Map per-vertex curvature values to RGB colors for visualization.
///
/// Returns one `[f32; 3]` RGB triplet per vertex. Low curvature → blue, high → red.
pub fn curvature_plot(curvature: &[f64]) -> Vec<[f32; 3]> {
    if curvature.is_empty() {
        return Vec::new();
    }
    let min_c = curvature.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_c = curvature.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max_c - min_c;
    curvature
        .iter()
        .map(|&c| {
            let t = if range > 1e-30 {
                ((c - min_c) / range).clamp(0.0, 1.0)
            } else {
                0.5
            };
            // Blue (0,0,1) → Cyan (0,1,1) → Green (0,1,0) → Yellow (1,1,0) → Red (1,0,0)
            let (r, g, b) = if t < 0.25 {
                let s = t / 0.25;
                (0.0, s, 1.0)
            } else if t < 0.5 {
                let s = (t - 0.25) / 0.25;
                (0.0, 1.0, 1.0 - s)
            } else if t < 0.75 {
                let s = (t - 0.5) / 0.25;
                (s, 1.0, 0.0)
            } else {
                let s = (t - 0.75) / 0.25;
                (1.0, 1.0 - s, 0.0)
            };
            [r as f32, g as f32, b as f32]
        })
        .collect()
}

/// Add a single triangle to a mesh, returning a new mesh.
pub fn add_triangle(mesh: &Mesh, a: Point3, b: Point3, c: Point3) -> Mesh {
    let mut result = mesh.clone();
    let base = result.vertices.len() as u32;
    result.vertices.push(a);
    result.vertices.push(b);
    result.vertices.push(c);
    result.indices.push([base, base + 1, base + 2]);
    result.normals.push(unit_triangle_normal(a, b, c));
    result
}

/// UV coordinate for mesh unwrapping.
#[derive(Debug, Clone, Copy)]
pub struct UvCoord {
    pub u: f64,
    pub v: f64,
}

/// Result of mesh unwrapping: per-vertex UV coordinates.
#[derive(Debug, Clone)]
pub struct UnwrapResult {
    pub uvs: Vec<UvCoord>,
}

/// Unwrap mesh to UV coordinates using angle-based flattening (projection).
///
/// Projects vertices onto the best-fit plane determined by mesh bounding box
/// principal axes, then normalizes to [0, 1] range.
pub fn unwrap_mesh(mesh: &Mesh) -> UnwrapResult {
    if mesh.vertices.is_empty() {
        return UnwrapResult { uvs: Vec::new() };
    }

    // Find principal axis via bounding box
    let (min, max) = mesh_bounding_box(mesh);
    let size = Vec3::new(max.x - min.x, max.y - min.y, max.z - min.z);

    // Project along the smallest axis
    let (u_axis, v_axis, u_offset, v_offset, u_range, v_range) = if size.x <= size.y && size.x <= size.z {
        // X is smallest → project to YZ plane
        (1usize, 2usize, min.y, min.z, size.y, size.z)
    } else if size.y <= size.z {
        // Y is smallest → project to XZ plane
        (0usize, 2usize, min.x, min.z, size.x, size.z)
    } else {
        // Z is smallest → project to XY plane
        (0usize, 1usize, min.x, min.y, size.x, size.y)
    };

    let uvs = mesh
        .vertices
        .iter()
        .map(|v| {
            let coords = [v.x, v.y, v.z];
            let u = if u_range > 1e-30 {
                (coords[u_axis] - u_offset) / u_range
            } else {
                0.5
            };
            let vv = if v_range > 1e-30 {
                (coords[v_axis] - v_offset) / v_range
            } else {
                0.5
            };
            UvCoord { u, v: vv }
        })
        .collect();

    UnwrapResult { uvs }
}

/// Unwrap a single face (triangle) to UV coordinates.
///
/// Maps the triangle to a 2D coordinate system preserving edge lengths.
pub fn unwrap_face(a: Point3, b: Point3, c: Point3) -> [UvCoord; 3] {
    let ab = b - a;
    let ac = c - a;
    let ab_len = ab.length();
    let ac_len = ac.length();
    let cos_angle = if ab_len > 1e-30 && ac_len > 1e-30 {
        ab.dot(ac) / (ab_len * ac_len)
    } else {
        1.0
    };
    let sin_angle = (1.0 - cos_angle * cos_angle).max(0.0).sqrt();

    [
        UvCoord { u: 0.0, v: 0.0 },
        UvCoord { u: ab_len, v: 0.0 },
        UvCoord {
            u: ac_len * cos_angle,
            v: ac_len * sin_angle,
        },
    ]
}

/// Remove mesh components smaller than `min_triangles`.
///
/// Splits mesh into connected components and removes those with fewer triangles
/// than the threshold.
pub fn remove_components_by_size(mesh: &Mesh, min_triangles: usize) -> Mesh {
    let components = split_mesh_by_components(mesh);
    let kept: Vec<&Mesh> = components
        .iter()
        .filter(|c| c.indices.len() >= min_triangles)
        .collect();

    if kept.is_empty() {
        return Mesh {
            vertices: Vec::new(),
            normals: Vec::new(),
            indices: Vec::new(),
        };
    }

    let mut result = kept[0].clone();
    for m in &kept[1..] {
        result = mesh_boolean_union(&result, m);
    }
    result
}

/// Remove a specific component by index (0-based) from the mesh.
///
/// Components are ordered by their first triangle's vertex index.
pub fn remove_component(mesh: &Mesh, component_index: usize) -> KernelResult<Mesh> {
    let components = split_mesh_by_components(mesh);
    if component_index >= components.len() {
        return Err(KernelError::InvalidArgument(format!(
            "component index {} out of range (0..{})",
            component_index,
            components.len()
        )));
    }

    let kept: Vec<&Mesh> = components
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != component_index)
        .map(|(_, m)| m)
        .collect();

    if kept.is_empty() {
        return Ok(Mesh {
            vertices: Vec::new(),
            normals: Vec::new(),
            indices: Vec::new(),
        });
    }

    let mut result = kept[0].clone();
    for m in &kept[1..] {
        result = mesh_boolean_union(&result, m);
    }
    Ok(result)
}

/// Trim mesh by removing triangles whose centroids are inside another mesh's bounding box.
pub fn trim_mesh(mesh: &Mesh, tool: &Mesh) -> Mesh {
    mesh_boolean_difference(mesh, tool)
}

/// Generate multiple parallel cross-sections through a mesh.
///
/// Creates `count` evenly-spaced cross-sections along the given axis direction.
pub fn mesh_cross_sections(
    mesh: &Mesh,
    axis: Vec3,
    count: usize,
) -> Vec<Vec<[Point3; 2]>> {
    if count == 0 || mesh.vertices.is_empty() {
        return Vec::new();
    }

    let normal = axis.normalized().unwrap_or(Vec3::Z);

    // Find extent along axis
    let mut min_d = f64::INFINITY;
    let mut max_d = f64::NEG_INFINITY;
    for v in &mesh.vertices {
        let d = Vec3::new(v.x, v.y, v.z).dot(normal);
        min_d = min_d.min(d);
        max_d = max_d.max(d);
    }

    let range = max_d - min_d;
    if range < 1e-30 {
        return Vec::new();
    }

    let mut sections = Vec::with_capacity(count);
    for i in 0..count {
        let t = (i as f64 + 1.0) / (count as f64 + 1.0);
        let d = min_d + t * range;
        let plane_point = Point3::new(normal.x * d, normal.y * d, normal.z * d);
        sections.push(mesh_section_from_plane(mesh, plane_point, normal));
    }

    sections
}

/// Mesh segment: a group of triangles with similar normals.
#[derive(Debug, Clone)]
pub struct MeshSegment {
    /// Triangle indices in the original mesh.
    pub triangle_indices: Vec<usize>,
    /// Average normal of the segment.
    pub average_normal: Vec3,
}

/// Segment mesh by grouping triangles with similar normals.
///
/// `angle_threshold` is the maximum angle (radians) between normals for triangles
/// to belong to the same segment.
pub fn segment_mesh(mesh: &Mesh, angle_threshold: f64) -> Vec<MeshSegment> {
    if mesh.indices.is_empty() {
        return Vec::new();
    }

    let cos_threshold = angle_threshold.cos();

    // Compute face normals
    let face_normals: Vec<Vec3> = mesh
        .indices
        .iter()
        .map(|tri| {
            unit_triangle_normal(
                mesh.vertices[tri[0] as usize],
                mesh.vertices[tri[1] as usize],
                mesh.vertices[tri[2] as usize],
            )
        })
        .collect();

    let mut assigned = vec![false; mesh.indices.len()];
    let mut segments = Vec::new();

    for seed in 0..mesh.indices.len() {
        if assigned[seed] {
            continue;
        }

        let seed_normal = face_normals[seed];
        if seed_normal.length_squared() < 1e-30 {
            assigned[seed] = true;
            continue;
        }

        let mut group = vec![seed];
        assigned[seed] = true;
        let mut sum_normal = seed_normal;

        // Region-growing: add unassigned triangles with similar normal
        let mut i = 0;
        while i < group.len() {
            let avg = sum_normal
                .normalized()
                .unwrap_or(Vec3::Z);
            // Check all remaining unassigned triangles
            for j in 0..mesh.indices.len() {
                if assigned[j] {
                    continue;
                }
                let n = face_normals[j];
                if n.dot(avg) >= cos_threshold {
                    assigned[j] = true;
                    group.push(j);
                    sum_normal += n;
                }
            }
            i += 1;
        }

        let average_normal = sum_normal
            .normalized()
            .unwrap_or(Vec3::Z);
        segments.push(MeshSegment {
            triangle_indices: group,
            average_normal,
        });
    }

    segments
}

/// Remesh (refinement) — subdivide long edges while keeping short edges intact.
///
/// Splits edges longer than `max_edge_length`, producing a more uniform mesh.
pub fn remesh(mesh: &Mesh, max_edge_length: f64) -> KernelResult<Mesh> {
    if max_edge_length <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "max_edge_length must be positive".to_string(),
        ));
    }
    if mesh.indices.is_empty() {
        return Ok(mesh.clone());
    }

    let max_len_sq = max_edge_length * max_edge_length;
    let mut vertices = mesh.vertices.clone();
    let mut indices = mesh.indices.clone();

    // Iterate a few times to refine progressively
    for _ in 0..4 {
        let mut new_indices = Vec::new();
        let mut edge_midpoints: HashMap<(u32, u32), u32> = HashMap::new();
        let mut any_split = false;

        for tri in &indices {
            let v0 = tri[0];
            let v1 = tri[1];
            let v2 = tri[2];

            let p0 = vertices[v0 as usize];
            let p1 = vertices[v1 as usize];
            let p2 = vertices[v2 as usize];

            let l01 = p0.distance_to(p1);
            let l12 = p1.distance_to(p2);
            let l20 = p2.distance_to(p0);

            let split01 = l01 * l01 > max_len_sq;
            let split12 = l12 * l12 > max_len_sq;
            let split20 = l20 * l20 > max_len_sq;

            if !split01 && !split12 && !split20 {
                new_indices.push(*tri);
                continue;
            }

            any_split = true;

            let mut get_mid = |a: u32, b: u32| -> u32 {
                let key = edge_key(a, b);
                if let Some(&idx) = edge_midpoints.get(&key) {
                    return idx;
                }
                let mid = vertices[a as usize].midpoint(vertices[b as usize]);
                let idx = vertices.len() as u32;
                vertices.push(mid);
                edge_midpoints.insert(key, idx);
                idx
            };

            if split01 && split12 && split20 {
                let m01 = get_mid(v0, v1);
                let m12 = get_mid(v1, v2);
                let m20 = get_mid(v2, v0);
                new_indices.push([v0, m01, m20]);
                new_indices.push([m01, v1, m12]);
                new_indices.push([m20, m12, v2]);
                new_indices.push([m01, m12, m20]);
            } else if split01 && split12 {
                let m01 = get_mid(v0, v1);
                let m12 = get_mid(v1, v2);
                new_indices.push([v0, m01, v2]);
                new_indices.push([m01, v1, m12]);
                new_indices.push([m01, m12, v2]);
            } else if split12 && split20 {
                let m12 = get_mid(v1, v2);
                let m20 = get_mid(v2, v0);
                new_indices.push([v0, v1, m12]);
                new_indices.push([v0, m12, m20]);
                new_indices.push([m20, m12, v2]);
            } else if split01 && split20 {
                let m01 = get_mid(v0, v1);
                let m20 = get_mid(v2, v0);
                new_indices.push([v0, m01, m20]);
                new_indices.push([m01, v1, v2]);
                new_indices.push([m01, v2, m20]);
            } else if split01 {
                let m01 = get_mid(v0, v1);
                new_indices.push([v0, m01, v2]);
                new_indices.push([m01, v1, v2]);
            } else if split12 {
                let m12 = get_mid(v1, v2);
                new_indices.push([v0, v1, m12]);
                new_indices.push([v0, m12, v2]);
            } else {
                let m20 = get_mid(v2, v0);
                new_indices.push([v0, v1, m20]);
                new_indices.push([v1, v2, m20]);
            }
        }

        indices = new_indices;
        if !any_split {
            break;
        }
    }

    let mut result = Mesh {
        vertices,
        normals: Vec::new(),
        indices,
    };
    recompute_normals(&mut result);
    Ok(result)
}

/// Evaluate and repair mesh: fix degenerate triangles, duplicate vertices, and inconsistent normals.
///
/// Returns the repaired mesh and a report of issues found.
#[derive(Debug, Clone)]
pub struct MeshRepairReport {
    pub degenerate_removed: usize,
    pub duplicate_vertices_merged: usize,
    pub normals_harmonized: bool,
}

/// Evaluate and repair a mesh.
pub fn evaluate_and_repair(mesh: &Mesh) -> (Mesh, MeshRepairReport) {
    let mut vertices = mesh.vertices.clone();
    let mut indices = mesh.indices.clone();

    // Step 1: Remove degenerate triangles (zero area)
    let orig_count = indices.len();
    indices.retain(|tri| {
        let a = vertices[tri[0] as usize];
        let b = vertices[tri[1] as usize];
        let c = vertices[tri[2] as usize];
        let cross = triangle_normal(a, b, c);
        cross.length_squared() > 1e-24
    });
    let degenerate_removed = orig_count - indices.len();

    // Step 2: Merge duplicate vertices (within tolerance)
    let mut vert_map: HashMap<[i64; 3], u32> = HashMap::new();
    let mut remap: Vec<u32> = vec![0; vertices.len()];
    let mut new_verts: Vec<Point3> = Vec::new();
    let quantize = 1e6;

    for (i, v) in vertices.iter().enumerate() {
        let key = [
            (v.x * quantize) as i64,
            (v.y * quantize) as i64,
            (v.z * quantize) as i64,
        ];
        let next_id = new_verts.len() as u32;
        let id = *vert_map.entry(key).or_insert_with(|| {
            new_verts.push(*v);
            next_id
        });
        remap[i] = id;
    }

    let duplicate_vertices_merged = vertices.len() - new_verts.len();
    vertices = new_verts;

    // Remap indices
    for tri in &mut indices {
        tri[0] = remap[tri[0] as usize];
        tri[1] = remap[tri[1] as usize];
        tri[2] = remap[tri[2] as usize];
    }

    // Remove degenerate after remap
    indices.retain(|tri| tri[0] != tri[1] && tri[1] != tri[2] && tri[2] != tri[0]);

    // Step 3: Harmonize normals
    let mut repaired = Mesh {
        vertices,
        normals: Vec::new(),
        indices,
    };
    recompute_normals(&mut repaired);
    repaired = harmonize_normals(&repaired);

    let report = MeshRepairReport {
        degenerate_removed,
        duplicate_vertices_merged,
        normals_harmonized: true,
    };

    (repaired, report)
}

/// Scale a mesh by given factors along each axis.
pub fn scale_mesh(mesh: &Mesh, sx: f64, sy: f64, sz: f64) -> Mesh {
    let mut result = mesh.clone();
    for v in &mut result.vertices {
        v.x *= sx;
        v.y *= sy;
        v.z *= sz;
    }
    recompute_normals(&mut result);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tetrahedron with 4 vertices and 4 triangles.
    fn make_tetrahedron() -> Mesh {
        let vertices = vec![
            Point3::new(1.0, 1.0, 1.0),
            Point3::new(-1.0, -1.0, 1.0),
            Point3::new(-1.0, 1.0, -1.0),
            Point3::new(1.0, -1.0, -1.0),
        ];
        let indices = vec![[0, 1, 2], [0, 2, 3], [0, 3, 1], [1, 3, 2]];
        let mut mesh = Mesh {
            vertices,
            normals: Vec::new(),
            indices,
        };
        recompute_normals(&mut mesh);
        mesh
    }

    /// A simple open mesh (2 triangles forming a quad with a boundary).
    fn make_open_quad() -> Mesh {
        let vertices = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let indices = vec![[0, 1, 2], [0, 2, 3]];
        let mut mesh = Mesh {
            vertices,
            normals: Vec::new(),
            indices,
        };
        recompute_normals(&mut mesh);
        mesh
    }

    /// Build an icosphere-like approximation of a unit sphere for curvature
    /// testing. Uses an octahedron subdivided once.
    fn make_unit_sphere_approx() -> Mesh {
        // Start with an octahedron.
        let vertices = vec![
            Point3::new(0.0, 0.0, 1.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(-1.0, 0.0, 0.0),
            Point3::new(0.0, -1.0, 0.0),
            Point3::new(0.0, 0.0, -1.0),
        ];
        let indices = vec![
            [0, 1, 2],
            [0, 2, 3],
            [0, 3, 4],
            [0, 4, 1],
            [5, 2, 1],
            [5, 3, 2],
            [5, 4, 3],
            [5, 1, 4],
        ];
        let mut mesh = Mesh {
            vertices,
            normals: Vec::new(),
            indices,
        };
        recompute_normals(&mut mesh);

        // Subdivide twice to get a better sphere approximation.
        mesh = subdivide_mesh(&mesh).unwrap();
        mesh = subdivide_mesh(&mesh).unwrap();

        // Project all vertices onto the unit sphere.
        for v in &mut mesh.vertices {
            let r = Vec3::new(v.x, v.y, v.z);
            let len = r.length();
            if len > 1e-30 {
                v.x = r.x / len;
                v.y = r.y / len;
                v.z = r.z / len;
            }
        }
        recompute_normals(&mut mesh);
        mesh
    }

    #[test]
    fn test_decimate_reduces_triangles() {
        let mesh = make_tetrahedron();
        assert_eq!(mesh.triangle_count(), 4);

        let decimated = decimate_mesh(&mesh, 0.5).unwrap();
        assert!(decimated.triangle_count() < 4);
        assert!(decimated.triangle_count() > 0);
    }

    #[test]
    fn test_decimate_invalid_ratio() {
        let mesh = make_tetrahedron();
        assert!(decimate_mesh(&mesh, 0.0).is_err());
        assert!(decimate_mesh(&mesh, 1.0).is_err());
        assert!(decimate_mesh(&mesh, -0.5).is_err());
        assert!(decimate_mesh(&mesh, 1.5).is_err());
    }

    #[test]
    fn test_fill_holes_closes_boundary() {
        // The open quad has 4 boundary edges forming one loop.
        let mesh = make_open_quad();
        let original_count = mesh.triangle_count();

        let filled = fill_holes(&mesh).unwrap();
        // Filling adds fan triangles, so triangle count should increase.
        assert!(filled.triangle_count() > original_count);
    }

    #[test]
    fn test_fill_holes_closed_mesh_unchanged() {
        let mesh = make_tetrahedron();
        let filled = fill_holes(&mesh).unwrap();
        assert_eq!(filled.triangle_count(), mesh.triangle_count());
    }

    #[test]
    fn test_compute_curvature_sphere() {
        let mesh = make_unit_sphere_approx();
        let curvature = compute_curvature(&mesh).unwrap();

        assert_eq!(curvature.len(), mesh.vertices.len());

        // For a unit sphere, mean curvature should be approximately 1.0.
        // With a discretized mesh this is approximate, so use a loose tolerance.
        let avg: f64 = curvature.iter().sum::<f64>() / curvature.len() as f64;
        assert!(
            (avg - 1.0).abs() < 0.5,
            "average curvature {avg} should be near 1.0"
        );
    }

    #[test]
    fn test_subdivide_mesh_4x_triangles() {
        let mesh = make_tetrahedron();
        assert_eq!(mesh.triangle_count(), 4);

        let subdivided = subdivide_mesh(&mesh).unwrap();
        assert_eq!(subdivided.triangle_count(), 16);
        assert_eq!(subdivided.normals.len(), 16);

        // Subdivide again.
        let subdivided2 = subdivide_mesh(&subdivided).unwrap();
        assert_eq!(subdivided2.triangle_count(), 64);
    }

    #[test]
    fn test_flip_normals_reverses() {
        let mesh = make_tetrahedron();
        let flipped = flip_normals(&mesh);

        assert_eq!(flipped.triangle_count(), mesh.triangle_count());

        for (orig, flip) in mesh.indices.iter().zip(flipped.indices.iter()) {
            // Winding order reversed: indices 1 and 2 swapped.
            assert_eq!(orig[0], flip[0]);
            assert_eq!(orig[1], flip[2]);
            assert_eq!(orig[2], flip[1]);
        }

        for (orig_n, flip_n) in mesh.normals.iter().zip(flipped.normals.iter()) {
            // Normals negated.
            assert!((orig_n.x + flip_n.x).abs() < 1e-12);
            assert!((orig_n.y + flip_n.y).abs() < 1e-12);
            assert!((orig_n.z + flip_n.z).abs() < 1e-12);
        }
    }

    /// Builds a simple cube mesh (12 triangles, 8 vertices).
    fn make_cube() -> Mesh {
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
        let indices = vec![
            [0, 2, 1], [0, 3, 2],
            [4, 5, 6], [4, 6, 7],
            [0, 1, 5], [0, 5, 4],
            [2, 3, 7], [2, 7, 6],
            [0, 4, 7], [0, 7, 3],
            [1, 2, 6], [1, 6, 5],
        ];
        let mut mesh = Mesh { vertices, normals: Vec::new(), indices };
        recompute_normals(&mut mesh);
        mesh
    }

    #[test]
    fn test_smooth_mesh_moves_vertices() {
        let mesh = make_cube();
        let smoothed = smooth_mesh(&mesh, 3, 0.5);
        assert_eq!(smoothed.triangle_count(), mesh.triangle_count());
        let mut moved = 0;
        for (orig, sm) in mesh.vertices.iter().zip(smoothed.vertices.iter()) {
            if orig.distance_to(*sm) > 1e-10 {
                moved += 1;
            }
        }
        assert!(moved > 0, "smoothing should move some vertices");
    }

    #[test]
    fn test_smooth_mesh_zero_iterations() {
        let mesh = make_cube();
        let smoothed = smooth_mesh(&mesh, 0, 0.5);
        for (orig, sm) in mesh.vertices.iter().zip(smoothed.vertices.iter()) {
            assert!(orig.distance_to(*sm) < 1e-14);
        }
    }

    #[test]
    fn test_mesh_boolean_union() {
        let a = make_tetrahedron();
        let b = make_open_quad();
        let merged = mesh_boolean_union(&a, &b);
        assert_eq!(
            merged.triangle_count(),
            a.triangle_count() + b.triangle_count()
        );
        assert_eq!(
            merged.vertices.len(),
            a.vertices.len() + b.vertices.len()
        );
    }

    #[test]
    fn test_cut_mesh_with_plane_half_cube() {
        let mesh = make_cube();
        let cut = cut_mesh_with_plane(
            &mesh,
            Point3::new(0.0, 0.0, 0.5),
            Vec3::Z,
        );
        assert!(cut.triangle_count() > 0, "cut should retain some triangles");
        for v in &cut.vertices {
            assert!(v.z >= 0.5 - 1e-10, "vertex z={} below cut plane", v.z);
        }
    }

    #[test]
    fn test_cut_mesh_discard_all() {
        let mesh = make_cube();
        let cut = cut_mesh_with_plane(
            &mesh,
            Point3::new(0.0, 0.0, 2.0),
            Vec3::Z,
        );
        assert_eq!(cut.triangle_count(), 0);
    }

    #[test]
    fn test_mesh_section_from_plane_cube() {
        let mesh = make_cube();
        let segments = mesh_section_from_plane(
            &mesh,
            Point3::new(0.0, 0.0, 0.5),
            Vec3::Z,
        );
        assert!(
            segments.len() >= 4,
            "expected at least 4 segments, got {}",
            segments.len()
        );
        for seg in &segments {
            assert!((seg[0].z - 0.5).abs() < 1e-10, "segment z={}", seg[0].z);
            assert!((seg[1].z - 0.5).abs() < 1e-10, "segment z={}", seg[1].z);
        }
    }

    #[test]
    fn test_split_mesh_by_components_two_cubes() {
        let cube1 = make_cube();
        let mut cube2 = make_cube();
        for v in &mut cube2.vertices {
            v.x += 10.0;
        }
        let merged = mesh_boolean_union(&cube1, &cube2);
        let components = split_mesh_by_components(&merged);
        assert_eq!(components.len(), 2, "should split into 2 components");
        for comp in &components {
            assert_eq!(comp.triangle_count(), 12);
        }
    }

    #[test]
    fn test_split_single_component() {
        let mesh = make_cube();
        let components = split_mesh_by_components(&mesh);
        assert_eq!(components.len(), 1);
    }

    #[test]
    fn test_harmonize_normals() {
        let mut mesh = make_cube();
        mesh.indices[0].swap(1, 2);
        mesh.indices[2].swap(1, 2);
        mesh.indices[4].swap(1, 2);
        recompute_normals(&mut mesh);

        let harmonized = harmonize_normals(&mesh);
        assert_eq!(harmonized.triangle_count(), mesh.triangle_count());

        let mut he_set: HashMap<(u32, u32), usize> = HashMap::new();
        for (ti, tri) in harmonized.indices.iter().enumerate() {
            for k in 0..3 {
                he_set.insert((tri[k], tri[(k + 1) % 3]), ti);
            }
        }
        // For consistent winding, each edge (a,b) should have a matching (b,a)
        // from a different triangle, not another (a,b)
        let mut bad_edges = 0;
        for tri in &harmonized.indices {
            for k in 0..3 {
                let a = tri[k];
                let b = tri[(k + 1) % 3];
                if he_set.contains_key(&(b, a)) {
                    // Good: opposite direction exists
                } else {
                    bad_edges += 1;
                }
            }
        }
        // Some boundary edges won't have opposites, but internal edges should
        assert!(bad_edges < harmonized.indices.len(), "most edges should be consistent");
    }

    #[test]
    fn test_check_mesh_watertight_closed_cube() {
        let mesh = make_cube();
        assert!(check_mesh_watertight(&mesh), "closed cube should be watertight");
    }

    #[test]
    fn test_check_mesh_watertight_open_quad() {
        let mesh = make_open_quad();
        assert!(!check_mesh_watertight(&mesh), "open quad is not watertight");
    }

    #[test]
    fn test_mesh_boolean_intersection() {
        let a = make_cube();
        let mut b = make_cube();
        // Shift b so it partially overlaps a
        for v in &mut b.vertices {
            v.x += 0.5;
        }
        let result = mesh_boolean_intersection(&a, &b);
        assert!(result.triangle_count() > 0, "intersection should have triangles");
        assert!(
            result.triangle_count() < a.triangle_count() + b.triangle_count(),
            "intersection should be smaller than union"
        );
    }

    #[test]
    fn test_mesh_boolean_difference() {
        let a = make_cube();
        let mut b = make_cube();
        for v in &mut b.vertices {
            v.x += 0.5;
        }
        let result = mesh_boolean_difference(&a, &b);
        assert!(result.triangle_count() > 0);
        assert!(result.triangle_count() <= a.triangle_count());
    }

    #[test]
    fn test_regular_solid_all_types() {
        let types = [
            (RegularSolidType::Tetrahedron, 4),
            (RegularSolidType::Cube, 12),
            (RegularSolidType::Octahedron, 8),
            (RegularSolidType::Icosahedron, 20),
        ];
        for (st, expected_tris) in types {
            let mesh = regular_solid(st, 1.0).unwrap();
            assert_eq!(
                mesh.triangle_count(),
                expected_tris,
                "{st:?} should have {expected_tris} triangles"
            );
        }
        // Dodecahedron: 12 pentagons × 3 triangles = 36
        let dodeca = regular_solid(RegularSolidType::Dodecahedron, 1.0).unwrap();
        assert_eq!(dodeca.triangle_count(), 36);
    }

    #[test]
    fn test_regular_solid_invalid_size() {
        assert!(regular_solid(RegularSolidType::Cube, 0.0).is_err());
        assert!(regular_solid(RegularSolidType::Cube, -1.0).is_err());
    }

    #[test]
    fn test_face_info_cube() {
        let mesh = make_cube();
        let infos = face_info(&mesh);
        assert_eq!(infos.len(), 12);
        for fi in &infos {
            assert!(fi.area > 0.0, "face area should be positive");
            assert!(fi.normal.length() > 0.99, "face normal should be unit");
        }
    }

    #[test]
    fn test_bounding_box_info_cube() {
        let mesh = make_cube();
        let bb = bounding_box_info(&mesh);
        assert!((bb.min.x - 0.0).abs() < 1e-12);
        assert!((bb.max.x - 1.0).abs() < 1e-12);
        assert!((bb.center.x - 0.5).abs() < 1e-12);
        assert!((bb.size.x - 1.0).abs() < 1e-12);
        assert!(bb.diagonal > 0.0);
    }

    #[test]
    fn test_curvature_plot() {
        let curvature = vec![0.0, 0.5, 1.0, 0.25, 0.75];
        let colors = curvature_plot(&curvature);
        assert_eq!(colors.len(), 5);
        // Min curvature (0.0) should be blue-ish
        assert!(colors[0][2] > 0.5, "low curvature should have high blue");
        // Max curvature (1.0) should be red-ish
        assert!(colors[2][0] > 0.5, "high curvature should have high red");
    }

    #[test]
    fn test_add_triangle() {
        let mesh = make_tetrahedron();
        let added = add_triangle(
            &mesh,
            Point3::new(5.0, 0.0, 0.0),
            Point3::new(6.0, 0.0, 0.0),
            Point3::new(5.5, 1.0, 0.0),
        );
        assert_eq!(added.triangle_count(), mesh.triangle_count() + 1);
        assert_eq!(added.vertices.len(), mesh.vertices.len() + 3);
    }

    #[test]
    fn test_unwrap_mesh() {
        let mesh = make_open_quad();
        let result = unwrap_mesh(&mesh);
        assert_eq!(result.uvs.len(), mesh.vertices.len());
        for uv in &result.uvs {
            assert!(uv.u >= 0.0 && uv.u <= 1.0, "u={} out of range", uv.u);
            assert!(uv.v >= 0.0 && uv.v <= 1.0, "v={} out of range", uv.v);
        }
    }

    #[test]
    fn test_unwrap_face() {
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        let uvs = unwrap_face(a, b, c);
        assert!((uvs[0].u - 0.0).abs() < 1e-12);
        assert!((uvs[0].v - 0.0).abs() < 1e-12);
        assert!((uvs[1].u - 1.0).abs() < 1e-12);
        assert!((uvs[1].v - 0.0).abs() < 1e-12);
        assert!(uvs[2].v > 0.0, "third vertex should have positive v");
    }

    #[test]
    fn test_remove_components_by_size() {
        let cube1 = make_cube(); // 12 triangles
        let tet = make_tetrahedron(); // 4 triangles — shift away
        let mut shifted_tet = tet.clone();
        for v in &mut shifted_tet.vertices {
            v.x += 20.0;
        }
        let merged = mesh_boolean_union(&cube1, &shifted_tet);
        let result = remove_components_by_size(&merged, 10);
        // Only the cube (12 tris) should remain
        assert_eq!(result.triangle_count(), 12);
    }

    #[test]
    fn test_remove_component() {
        let cube1 = make_cube();
        let mut cube2 = make_cube();
        for v in &mut cube2.vertices {
            v.x += 10.0;
        }
        let merged = mesh_boolean_union(&cube1, &cube2);
        let result = remove_component(&merged, 0).unwrap();
        assert_eq!(result.triangle_count(), 12);
        assert!(remove_component(&merged, 5).is_err());
    }

    #[test]
    fn test_mesh_cross_sections() {
        let mesh = make_cube();
        let sections = mesh_cross_sections(&mesh, Vec3::Z, 3);
        assert_eq!(sections.len(), 3);
        for section in &sections {
            assert!(!section.is_empty(), "each cross-section should have segments");
        }
    }

    #[test]
    fn test_segment_mesh_cube() {
        let mesh = make_cube();
        let segments = segment_mesh(&mesh, 0.1); // tight angle → many segments
        // A cube has 6 distinct face normals, so expect 6 segments
        assert_eq!(segments.len(), 6, "cube should have 6 normal-based segments");
        let total: usize = segments.iter().map(|s| s.triangle_indices.len()).sum();
        assert_eq!(total, 12, "all 12 triangles should be assigned");
    }

    #[test]
    fn test_remesh_splits_long_edges() {
        let mesh = make_cube();
        let remeshed = remesh(&mesh, 0.5).unwrap();
        assert!(
            remeshed.triangle_count() > mesh.triangle_count(),
            "remeshing with small max edge should increase triangle count"
        );
    }

    #[test]
    fn test_evaluate_and_repair() {
        let mut mesh = make_cube();
        // Add a degenerate triangle
        let n = mesh.vertices.len() as u32;
        mesh.vertices.push(Point3::new(5.0, 0.0, 0.0));
        mesh.vertices.push(Point3::new(5.0, 0.0, 0.0)); // same point
        mesh.vertices.push(Point3::new(5.0, 0.0, 0.0)); // same point
        mesh.indices.push([n, n + 1, n + 2]);

        let (repaired, report) = evaluate_and_repair(&mesh);
        assert!(report.degenerate_removed >= 1);
        assert!(repaired.triangle_count() <= mesh.triangle_count());
    }

    #[test]
    fn test_scale_mesh() {
        let mesh = make_cube();
        let scaled = scale_mesh(&mesh, 2.0, 3.0, 4.0);
        assert_eq!(scaled.triangle_count(), mesh.triangle_count());
        let bb = bounding_box_info(&scaled);
        assert!((bb.size.x - 2.0).abs() < 1e-12);
        assert!((bb.size.y - 3.0).abs() < 1e-12);
        assert!((bb.size.z - 4.0).abs() < 1e-12);
    }

    #[test]
    fn test_trim_mesh() {
        let mesh = make_cube();
        let mut tool = make_cube();
        for v in &mut tool.vertices {
            v.x += 0.5;
        }
        let result = trim_mesh(&mesh, &tool);
        assert!(result.triangle_count() > 0);
        assert!(result.triangle_count() <= mesh.triangle_count());
    }
}
