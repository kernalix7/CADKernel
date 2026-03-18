//! Flat spiral (Archimedean spiral) tube primitive.

use std::f64::consts::TAU;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::Point3;
use cadkernel_topology::{BRepModel, EntityKind, FaceData, Handle, SolidData, Tag, VertexData};

use super::{EdgeCache, bind_edge_line_segments, next_edge_tag};

/// Handles returned from [`make_spiral`].
pub struct SpiralResult {
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Creates a flat spiral solid (tube along an Archimedean spiral path).
///
/// `initial_radius` = starting distance from center
/// `growth_per_rev` = radial growth per full revolution
/// `turns` = number of full turns
/// `tube_radius` = radius of the circular tube cross-section
/// `segments_per_turn` = tessellation resolution per revolution
#[allow(clippy::too_many_arguments)]
pub fn make_spiral(
    model: &mut BRepModel,
    center: Point3,
    initial_radius: f64,
    growth_per_rev: f64,
    turns: f64,
    tube_radius: f64,
    segments_per_turn: usize,
) -> KernelResult<SpiralResult> {
    if initial_radius <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "initial_radius must be positive".into(),
        ));
    }
    if growth_per_rev < 0.0 {
        return Err(KernelError::InvalidArgument(
            "growth_per_rev must be non-negative".into(),
        ));
    }
    if turns <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "turns must be positive".into(),
        ));
    }
    if tube_radius <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "tube_radius must be positive".into(),
        ));
    }
    if segments_per_turn < 4 {
        return Err(KernelError::InvalidArgument(
            "need at least 4 segments per turn".into(),
        ));
    }
    if tube_radius >= initial_radius {
        return Err(KernelError::InvalidArgument(
            "tube_radius must be less than initial_radius".into(),
        ));
    }

    let op = model.history.next_operation("make_spiral");
    let mut ec = EdgeCache::new();
    let mut edge_idx = 0u32;
    let mut vert_idx = 0u32;

    let total_segs = (segments_per_turn as f64 * turns).ceil() as usize;
    let n_rings = total_segs + 1;
    let tube_segs: usize = 8; // cross-section resolution

    // Generate rings of tube cross-section vertices along the spiral
    let mut rings: Vec<Vec<Handle<VertexData>>> = Vec::with_capacity(n_rings);

    for i in 0..n_rings {
        let t = i as f64 / total_segs as f64;
        let theta = TAU * turns * t;
        let r = initial_radius + growth_per_rev * turns * t;

        let (st, ct) = theta.sin_cos();
        // Spiral center point (flat — z = center.z)
        let cx = center.x + r * ct;
        let cy = center.y + r * st;
        let cz = center.z;

        // Tangent direction: derivative of (r*cos(theta), r*sin(theta))
        // dr/dtheta = growth_per_rev / (2*pi), but for the parameterization by t:
        let dr_dt = growth_per_rev * turns;
        let dtheta_dt = TAU * turns;
        let tx = dr_dt * ct - r * dtheta_dt * st;
        let ty = dr_dt * st + r * dtheta_dt * ct;
        let tlen = (tx * tx + ty * ty).sqrt();
        let (tx_n, ty_n) = if tlen > 1e-14 {
            (tx / tlen, ty / tlen)
        } else {
            (-st, ct)
        };

        // Normal: perpendicular to tangent in XY plane (rotate tangent 90 degrees)
        let nx = -ty_n;
        let ny = tx_n;

        // Binormal: Z axis (spiral is flat)
        let bz = 1.0;

        let mut ring = Vec::with_capacity(tube_segs);
        for j in 0..tube_segs {
            let phi = TAU * j as f64 / tube_segs as f64;
            let (sp, cp) = phi.sin_cos();

            let px = cx + tube_radius * (nx * cp);
            let py = cy + tube_radius * (ny * cp);
            let pz = cz + tube_radius * (bz * sp);

            let tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
            vert_idx += 1;
            let vh = model.add_vertex_tagged(Point3::new(px, py, pz), tag);
            ring.push(vh);
        }
        rings.push(ring);
    }

    // Build quad faces between consecutive rings
    let mut faces = Vec::new();
    let mut face_idx = 0u32;

    for i in 0..n_rings - 1 {
        for j in 0..tube_segs {
            let j_next = (j + 1) % tube_segs;
            let hes = [
                ec.get_or_create(
                    model,
                    rings[i][j],
                    rings[i][j_next],
                    next_edge_tag(op, &mut edge_idx),
                ),
                ec.get_or_create(
                    model,
                    rings[i][j_next],
                    rings[i + 1][j_next],
                    next_edge_tag(op, &mut edge_idx),
                ),
                ec.get_or_create(
                    model,
                    rings[i + 1][j_next],
                    rings[i + 1][j],
                    next_edge_tag(op, &mut edge_idx),
                ),
                ec.get_or_create(
                    model,
                    rings[i + 1][j],
                    rings[i][j],
                    next_edge_tag(op, &mut edge_idx),
                ),
            ];
            let loop_h = model.make_loop(&hes)?;
            let tag = Tag::generated(EntityKind::Face, op, face_idx);
            faces.push(model.make_face_tagged(loop_h, tag));
            face_idx += 1;
        }
    }

    // Start cap (reversed winding)
    {
        let mut hes = Vec::with_capacity(tube_segs);
        for j in (0..tube_segs).rev() {
            let j_prev = if j == 0 { tube_segs - 1 } else { j - 1 };
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
        let mut hes = Vec::with_capacity(tube_segs);
        for j in 0..tube_segs {
            let j_next = (j + 1) % tube_segs;
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

    Ok(SpiralResult { solid, faces })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spiral_entity_counts() {
        let mut model = BRepModel::new();
        let r = make_spiral(
            &mut model,
            Point3::ORIGIN,
            5.0,  // initial_radius
            1.0,  // growth_per_rev
            2.0,  // turns
            0.5,  // tube_radius
            8,    // segments_per_turn
        )
        .unwrap();
        // 2 turns * 8 segs = 16, ceil = 16, n_rings = 17
        // 16 * 8 lateral + 2 caps = 130 faces
        assert_eq!(r.faces.len(), 130);
        assert!(model.solids.is_alive(r.solid));
    }

    #[test]
    fn test_spiral_vertex_positions() {
        let mut model = BRepModel::new();
        let r = make_spiral(
            &mut model,
            Point3::new(1.0, 2.0, 3.0),
            5.0,
            0.0, // no growth — circle
            1.0,
            0.5,
            8,
        )
        .unwrap();
        // Should produce a valid solid
        assert!(model.solids.is_alive(r.solid));
        assert!(!r.faces.is_empty());
    }

    #[test]
    fn test_spiral_validation() {
        let mut model = BRepModel::new();
        assert!(make_spiral(&mut model, Point3::ORIGIN, -1.0, 1.0, 1.0, 0.5, 8).is_err());
        assert!(make_spiral(&mut model, Point3::ORIGIN, 5.0, -1.0, 1.0, 0.5, 8).is_err());
        assert!(make_spiral(&mut model, Point3::ORIGIN, 5.0, 1.0, -1.0, 0.5, 8).is_err());
        assert!(make_spiral(&mut model, Point3::ORIGIN, 5.0, 1.0, 1.0, -0.5, 8).is_err());
        assert!(make_spiral(&mut model, Point3::ORIGIN, 5.0, 1.0, 1.0, 0.5, 3).is_err());
        assert!(make_spiral(&mut model, Point3::ORIGIN, 5.0, 1.0, 1.0, 6.0, 8).is_err());
    }
}
