//! Helix (spring/thread) curve primitive.

use std::f64::consts::TAU;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::Point3;
use cadkernel_topology::{BRepModel, EntityKind, FaceData, Handle, SolidData, Tag, VertexData};

use super::{EdgeCache, bind_edge_line_segments, next_edge_tag};

#[derive(Debug)]
pub struct HelixResult {
    pub vertices: Vec<Handle<VertexData>>,
    pub faces: Vec<Handle<FaceData>>,
    pub solid: Handle<SolidData>,
}

/// Creates a helical tube (spring-like solid) as a B-Rep solid.
///
/// `radius` = helix center radius (distance from axis to tube center)
/// `pitch` = height per full revolution
/// `turns` = number of full revolutions
/// `tube_radius` = radius of the circular tube cross-section
/// `segments` = points per turn for helix curve
/// `tube_segments` = points around tube cross-section
#[allow(clippy::too_many_arguments)]
pub fn make_helix(
    model: &mut BRepModel,
    center: Point3,
    radius: f64,
    pitch: f64,
    turns: f64,
    tube_radius: f64,
    segments: usize,
    tube_segments: usize,
) -> KernelResult<HelixResult> {
    if radius <= 0.0 || tube_radius <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "radii must be positive".into(),
        ));
    }
    if pitch <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "pitch must be positive".into(),
        ));
    }
    if turns <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "turns must be positive".into(),
        ));
    }
    if segments < 4 {
        return Err(KernelError::InvalidArgument(
            "need at least 4 segments per turn".into(),
        ));
    }
    if tube_segments < 3 {
        return Err(KernelError::InvalidArgument(
            "need at least 3 tube segments".into(),
        ));
    }
    if tube_radius >= radius {
        return Err(KernelError::InvalidArgument(
            "tube_radius must be less than radius".into(),
        ));
    }

    let op = model.history.next_operation("make_helix");
    let mut ec = EdgeCache::new();
    let mut edge_idx = 0u32;
    let mut vert_idx = 0u32;

    let total_segs = (segments as f64 * turns).ceil() as usize;
    let n_rings = total_segs + 1;
    let ts = tube_segments;

    // Generate rings of tube cross-section vertices along the helix
    let mut rings: Vec<Vec<Handle<VertexData>>> = Vec::with_capacity(n_rings);
    let mut all_verts = Vec::new();

    for i in 0..n_rings {
        let t = i as f64 / total_segs as f64;
        let theta = TAU * turns * t; // angle around helix
        let z = pitch * turns * t;

        let (st, ct) = theta.sin_cos();
        // Helix center point
        let cx = center.x + radius * ct;
        let cy = center.y + radius * st;
        let cz = center.z + z;

        // Local coordinate frame at this point on the helix
        // tangent direction
        let _tx = -radius * st;
        let _ty = radius * ct;
        let _tz = pitch;

        // Normal direction (radial outward)
        let nx = ct;
        let ny = st;

        // Binormal = tangent × normal (approx Z component)
        let bx = -st * pitch;
        let by = ct * pitch;
        let bz = -(radius);
        let bl = (bx * bx + by * by + bz * bz).sqrt();
        let (bx, by, bz) = if bl > 1e-14 {
            (bx / bl, by / bl, bz / bl)
        } else {
            (0.0, 0.0, 1.0)
        };

        let mut ring = Vec::with_capacity(ts);
        for j in 0..ts {
            let phi = TAU * j as f64 / ts as f64;
            let (sp, cp) = phi.sin_cos();

            let px = cx + tube_radius * (nx * cp + bx * sp);
            let py = cy + tube_radius * (ny * cp + by * sp);
            let pz = cz + tube_radius * (bz * sp);

            let tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
            vert_idx += 1;
            let vh = model.add_vertex_tagged(Point3::new(px, py, pz), tag);
            ring.push(vh);
            all_verts.push(vh);
        }
        rings.push(ring);
    }

    // Build quad faces between consecutive rings
    let mut faces = Vec::new();
    let mut face_idx = 0u32;

    for i in 0..n_rings - 1 {
        for j in 0..ts {
            let j_next = (j + 1) % ts;
            let hes = [
                ec.get_or_create(model, rings[i][j], rings[i][j_next], next_edge_tag(op, &mut edge_idx)),
                ec.get_or_create(model, rings[i][j_next], rings[i + 1][j_next], next_edge_tag(op, &mut edge_idx)),
                ec.get_or_create(model, rings[i + 1][j_next], rings[i + 1][j], next_edge_tag(op, &mut edge_idx)),
                ec.get_or_create(model, rings[i + 1][j], rings[i][j], next_edge_tag(op, &mut edge_idx)),
            ];
            let loop_h = model.make_loop(&hes)?;
            let tag = Tag::generated(EntityKind::Face, op, face_idx);
            faces.push(model.make_face_tagged(loop_h, tag));
            face_idx += 1;
        }
    }

    // Cap the ends with polygon faces
    // Start cap (reversed winding)
    {
        let mut hes = Vec::with_capacity(ts);
        for j in (0..ts).rev() {
            let j_prev = if j == 0 { ts - 1 } else { j - 1 };
            hes.push(ec.get_or_create(
                model,
                rings[0][j],
                rings[0][j_prev],
                next_edge_tag(op, &mut edge_idx),
            ));
        }
        let loop_h = model.make_loop(&hes)?;
        let tag = Tag::generated(EntityKind::Face, op, face_idx);
        faces.push(model.make_face_tagged(loop_h, tag));
        face_idx += 1;
    }

    // End cap (forward winding)
    {
        let last = n_rings - 1;
        let mut hes = Vec::with_capacity(ts);
        for j in 0..ts {
            let j_next = (j + 1) % ts;
            hes.push(ec.get_or_create(
                model,
                rings[last][j],
                rings[last][j_next],
                next_edge_tag(op, &mut edge_idx),
            ));
        }
        let loop_h = model.make_loop(&hes)?;
        let tag = Tag::generated(EntityKind::Face, op, face_idx);
        faces.push(model.make_face_tagged(loop_h, tag));
        face_idx += 1;
    }
    let _ = face_idx;

    bind_edge_line_segments(model, &ec);

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&faces, shell_tag);
    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let solid = model.make_solid_tagged(&[shell], solid_tag);

    Ok(HelixResult {
        vertices: all_verts,
        faces,
        solid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_helix_entity_counts() {
        let mut model = BRepModel::new();
        let r = make_helix(&mut model, Point3::ORIGIN, 5.0, 2.0, 1.0, 0.5, 8, 6).unwrap();
        // 9 rings * 6 tube_segments = 54 verts
        // (9-1) * 6 = 48 lateral faces + 2 caps = 50
        assert_eq!(r.vertices.len(), 54);
        assert_eq!(r.faces.len(), 50);
        assert!(model.solids.is_alive(r.solid));
    }

    #[test]
    fn test_helix_validation() {
        let mut model = BRepModel::new();
        assert!(make_helix(&mut model, Point3::ORIGIN, -1.0, 2.0, 1.0, 0.5, 8, 6).is_err());
        assert!(make_helix(&mut model, Point3::ORIGIN, 5.0, -1.0, 1.0, 0.5, 8, 6).is_err());
        assert!(make_helix(&mut model, Point3::ORIGIN, 5.0, 2.0, -1.0, 0.5, 8, 6).is_err());
        assert!(make_helix(&mut model, Point3::ORIGIN, 5.0, 2.0, 1.0, 6.0, 8, 6).is_err());
        assert!(make_helix(&mut model, Point3::ORIGIN, 5.0, 2.0, 1.0, 0.5, 3, 6).is_err());
    }

    #[test]
    fn test_helix_multiple_turns() {
        let mut model = BRepModel::new();
        let r = make_helix(&mut model, Point3::ORIGIN, 3.0, 1.0, 3.0, 0.3, 8, 4).unwrap();
        // 3 turns * 8 segs = 24 + 1 = 25 rings, 4 tube segs
        // 24 * 4 = 96 lateral + 2 caps = 98
        assert_eq!(r.faces.len(), 98);
        assert!(model.solids.is_alive(r.solid));
    }
}
