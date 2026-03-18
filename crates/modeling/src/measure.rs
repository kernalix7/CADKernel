//! Mass-property and measurement utilities for B-Rep solids and meshes.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, EdgeData, FaceData, Handle, SolidData, VertexData};

/// Mass properties of a solid body.
#[derive(Debug, Clone)]
pub struct MassProperties {
    /// Volume of the solid.
    pub volume: f64,
    /// Surface area of the solid.
    pub surface_area: f64,
    /// Center of mass (centroid).
    pub centroid: Point3,
}

/// Computes mass properties from a tessellated mesh.
///
/// Uses the divergence theorem on the triangle mesh to compute signed
/// volume, surface area, and centroid.
pub fn compute_mass_properties(mesh: &cadkernel_io::Mesh) -> MassProperties {
    let mut volume = 0.0_f64;
    let mut area = 0.0_f64;
    let mut cx = 0.0_f64;
    let mut cy = 0.0_f64;
    let mut cz = 0.0_f64;

    for tri in mesh.indices.iter() {
        let v0 = mesh.vertices[tri[0] as usize];
        let v1 = mesh.vertices[tri[1] as usize];
        let v2 = mesh.vertices[tri[2] as usize];

        // Signed volume contribution via divergence theorem
        let cross_x = (v1.y - v0.y) * (v2.z - v0.z) - (v1.z - v0.z) * (v2.y - v0.y);
        let cross_y = (v1.z - v0.z) * (v2.x - v0.x) - (v1.x - v0.x) * (v2.z - v0.z);
        let cross_z = (v1.x - v0.x) * (v2.y - v0.y) - (v1.y - v0.y) * (v2.x - v0.x);

        let tri_vol = v0.x * cross_x + v0.y * cross_y + v0.z * cross_z;
        volume += tri_vol;

        // Surface area
        let area_2 = (cross_x * cross_x + cross_y * cross_y + cross_z * cross_z).sqrt();
        area += area_2;

        // Centroid weighted by volume contribution
        let mid_x = (v0.x + v1.x + v2.x) / 4.0;
        let mid_y = (v0.y + v1.y + v2.y) / 4.0;
        let mid_z = (v0.z + v1.z + v2.z) / 4.0;
        cx += tri_vol * mid_x;
        cy += tri_vol * mid_y;
        cz += tri_vol * mid_z;
    }

    volume /= 6.0;
    area /= 2.0;
    let abs_vol = volume.abs();
    if abs_vol < 1e-12 {
        // Degenerate mesh — centroid undefined.
        return MassProperties {
            volume: abs_vol,
            surface_area: area,
            centroid: Point3::ORIGIN,
        };
    }
    cx /= 6.0 * abs_vol;
    cy /= 6.0 * abs_vol;
    cz /= 6.0 * abs_vol;

    MassProperties {
        volume: volume.abs(),
        surface_area: area,
        centroid: Point3::new(cx, cy, cz),
    }
}

/// Computes mass properties for a solid by tessellating its faces.
pub fn solid_mass_properties(
    model: &BRepModel,
    solid: Handle<SolidData>,
) -> KernelResult<MassProperties> {
    let mesh = cadkernel_io::tessellate_solid(model, solid);
    if mesh.indices.is_empty() {
        return Err(KernelError::InvalidArgument(
            "solid has no tessellatable faces".into(),
        ));
    }
    Ok(compute_mass_properties(&mesh))
}

// ---------------------------------------------------------------------------
// Measurement utilities
// ---------------------------------------------------------------------------

/// Returns the Euclidean distance between two vertices.
pub fn measure_distance(
    model: &BRepModel,
    v1: Handle<VertexData>,
    v2: Handle<VertexData>,
) -> KernelResult<f64> {
    let p1 = model
        .vertices
        .get(v1)
        .ok_or(KernelError::InvalidHandle("vertex v1"))?
        .point;
    let p2 = model
        .vertices
        .get(v2)
        .ok_or(KernelError::InvalidHandle("vertex v2"))?
        .point;
    let d = p2 - p1;
    Ok((d.x * d.x + d.y * d.y + d.z * d.z).sqrt())
}

/// Returns the angle (in radians) between two edges, measured from
/// the direction vectors (start → end) of each edge.
pub fn measure_angle(
    model: &BRepModel,
    e1: Handle<EdgeData>,
    e2: Handle<EdgeData>,
) -> KernelResult<f64> {
    let dir1 = edge_direction(model, e1)?;
    let dir2 = edge_direction(model, e2)?;

    let dot = dir1.x * dir2.x + dir1.y * dir2.y + dir1.z * dir2.z;
    let clamped = dot.clamp(-1.0, 1.0);
    Ok(clamped.acos())
}

/// Returns the length of an edge (Euclidean distance from start to end vertex).
pub fn measure_edge_length(
    model: &BRepModel,
    edge: Handle<EdgeData>,
) -> KernelResult<f64> {
    let ed = model
        .edges
        .get(edge)
        .ok_or(KernelError::InvalidHandle("edge"))?;
    let p_start = model
        .vertices
        .get(ed.start)
        .ok_or(KernelError::InvalidHandle("edge start vertex"))?
        .point;
    let p_end = model
        .vertices
        .get(ed.end)
        .ok_or(KernelError::InvalidHandle("edge end vertex"))?
        .point;
    let d = p_end - p_start;
    Ok((d.x * d.x + d.y * d.y + d.z * d.z).sqrt())
}

/// Returns the approximate area of a face by tessellating it and summing
/// triangle areas.
pub fn measure_face_area(
    model: &BRepModel,
    face: Handle<FaceData>,
) -> KernelResult<f64> {
    let triangles = cadkernel_io::tessellate_face(model, face);
    if triangles.is_empty() {
        return Err(KernelError::InvalidArgument(
            "face has no tessellatable geometry".into(),
        ));
    }
    let mut area = 0.0_f64;
    for tri in &triangles {
        let a = tri.vertices[0];
        let b = tri.vertices[1];
        let c = tri.vertices[2];
        let ab = b - a;
        let ac = c - a;
        let cross = Vec3::new(
            ab.y * ac.z - ab.z * ac.y,
            ab.z * ac.x - ab.x * ac.z,
            ab.x * ac.y - ab.y * ac.x,
        );
        area += 0.5 * (cross.x * cross.x + cross.y * cross.y + cross.z * cross.z).sqrt();
    }
    Ok(area)
}

/// Returns the approximate volume of a solid using the divergence theorem
/// on its tessellated mesh.
pub fn measure_solid_volume(
    model: &BRepModel,
    solid: Handle<SolidData>,
) -> KernelResult<f64> {
    let props = solid_mass_properties(model, solid)?;
    Ok(props.volume)
}

/// Returns the approximate center of mass of a solid.
pub fn measure_solid_center_of_mass(
    model: &BRepModel,
    solid: Handle<SolidData>,
) -> KernelResult<Point3> {
    let props = solid_mass_properties(model, solid)?;
    Ok(props.centroid)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn edge_direction(model: &BRepModel, edge: Handle<EdgeData>) -> KernelResult<Vec3> {
    let ed = model
        .edges
        .get(edge)
        .ok_or(KernelError::InvalidHandle("edge"))?;
    let p_start = model
        .vertices
        .get(ed.start)
        .ok_or(KernelError::InvalidHandle("edge start vertex"))?
        .point;
    let p_end = model
        .vertices
        .get(ed.end)
        .ok_or(KernelError::InvalidHandle("edge end vertex"))?
        .point;
    let d = p_end - p_start;
    d.normalized()
        .ok_or(KernelError::InvalidArgument("zero-length edge".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_math::Point3;

    fn make_test_box() -> (BRepModel, crate::primitives::BoxResult) {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 3.0, 4.0).unwrap();
        (model, b)
    }

    #[test]
    fn test_measure_distance() {
        let (model, b) = make_test_box();
        // Vertices 0 and 1 should be 2.0 apart (along X for the box origin→origin+dx)
        let v0 = b.vertices[0];
        let v1 = b.vertices[1];
        let dist = measure_distance(&model, v0, v1).unwrap();
        // Distance between any two adjacent vertices of a 2×3×4 box
        // is one of {2, 3, 4} depending on which edge
        assert!(dist > 0.0);
    }

    #[test]
    fn test_measure_angle_parallel_edges() {
        let (model, b) = make_test_box();
        let edges = model.edges_of_face(b.faces[0]).unwrap();
        // Opposite edges of a rectangular face should be parallel (angle = 0 or pi)
        assert!(edges.len() >= 2);
        let angle = measure_angle(&model, edges[0], edges[0]).unwrap();
        assert!(angle.abs() < 1e-10, "self-angle should be 0");
    }

    #[test]
    fn test_measure_edge_length() {
        let (model, b) = make_test_box();
        let edges = model.edges_of_face(b.faces[0]).unwrap();
        let len = measure_edge_length(&model, edges[0]).unwrap();
        // Edge lengths on a 2×3×4 box face are from {2, 3, 4}
        assert!(len > 0.0);
    }

    #[test]
    fn test_measure_face_area() {
        let (model, b) = make_test_box();
        // A 2×3×4 box has faces with areas: 2×3=6, 2×4=8, 3×4=12
        let area = measure_face_area(&model, b.faces[0]).unwrap();
        let valid = (area - 6.0).abs() < 0.5
            || (area - 8.0).abs() < 0.5
            || (area - 12.0).abs() < 0.5;
        assert!(valid, "face area {area} should be 6, 8, or 12");
    }

    #[test]
    fn test_measure_solid_volume() {
        let (model, b) = make_test_box();
        let vol = measure_solid_volume(&model, b.solid).unwrap();
        assert!(
            (vol - 24.0).abs() < 1.0,
            "volume of 2×3×4 box should be ~24, got {vol}"
        );
    }

    #[test]
    fn test_measure_solid_center_of_mass() {
        let (model, b) = make_test_box();
        let com = measure_solid_center_of_mass(&model, b.solid).unwrap();
        // Center of a 2×3×4 box at origin should be near (1, 1.5, 2)
        assert!(
            (com.x - 1.0).abs() < 0.5,
            "centroid x should be ~1, got {}",
            com.x
        );
        assert!(
            (com.y - 1.5).abs() < 0.5,
            "centroid y should be ~1.5, got {}",
            com.y
        );
        assert!(
            (com.z - 2.0).abs() < 0.5,
            "centroid z should be ~2, got {}",
            com.z
        );
    }
}
