//! Involute spur gear primitive.

use std::f64::consts::TAU;
use std::sync::Arc;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_geometry::Plane;
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{
    BRepModel, EntityKind, FaceData, Handle, Orientation, SolidData, Tag, VertexData,
};

use crate::primitives::{EdgeCache, bind_edge_line_segments, next_edge_tag};

/// Handles returned from [`make_involute_gear`].
pub struct GearResult {
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Creates an involute spur gear solid.
///
/// `module_val` = gear module (m), tooth size parameter
/// `num_teeth` = number of teeth (z)
/// `pressure_angle` = pressure angle in radians (typically 20 degrees)
/// `face_width` = axial thickness of the gear
pub fn make_involute_gear(
    model: &mut BRepModel,
    module_val: f64,
    num_teeth: usize,
    pressure_angle: f64,
    face_width: f64,
) -> KernelResult<GearResult> {
    if module_val <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "module must be positive".into(),
        ));
    }
    if num_teeth < 3 {
        return Err(KernelError::InvalidArgument(
            "need at least 3 teeth".into(),
        ));
    }
    if pressure_angle <= 0.0 || pressure_angle >= std::f64::consts::FRAC_PI_2 {
        return Err(KernelError::InvalidArgument(
            "pressure_angle must be between 0 and pi/2".into(),
        ));
    }
    if face_width <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "face_width must be positive".into(),
        ));
    }

    let op = model.history.next_operation("make_involute_gear");
    let mut ec = EdgeCache::new();
    let mut edge_idx = 0u32;
    let mut vert_idx = 0u32;

    let z = num_teeth as f64;
    let pitch_radius = module_val * z / 2.0;
    let base_radius = pitch_radius * pressure_angle.cos();
    let addendum = module_val;
    let dedendum = 1.25 * module_val;
    let outer_radius = pitch_radius + addendum;
    let root_radius = (pitch_radius - dedendum).max(0.1 * module_val);

    // Number of points along one side of the involute tooth profile
    let involute_steps = 8;

    // Generate the 2D gear profile (one Z-layer of vertices)
    let tooth_angle = TAU / z;

    // Involute curve: parametric from base circle to outer circle
    // x(t) = r_b * (cos(t) + t * sin(t))
    // y(t) = r_b * (sin(t) - t * cos(t))
    // where t ranges from 0 to t_max such that the radius reaches outer_radius.
    let t_max = if base_radius > 1e-14 {
        ((outer_radius / base_radius) * (outer_radius / base_radius) - 1.0)
            .max(0.0)
            .sqrt()
    } else {
        1.0
    };

    // Involute tooth angle at pitch circle
    let t_pitch = if base_radius > 1e-14 {
        ((pitch_radius / base_radius) * (pitch_radius / base_radius) - 1.0)
            .max(0.0)
            .sqrt()
    } else {
        0.0
    };
    let inv_alpha = t_pitch - t_pitch.atan();

    // Half tooth thickness at pitch circle (in angular terms)
    let half_tooth = TAU / (4.0 * z) + inv_alpha;

    fn involute_point(base_r: f64, t: f64) -> (f64, f64) {
        let x = base_r * (t.cos() + t * t.sin());
        let y = base_r * (t.sin() - t * t.cos());
        (x, y)
    }

    // Build the profile for a single tooth, centered at angle 0
    // Left flank (mirrored involute)
    let mut profile_pts: Vec<(f64, f64)> = Vec::new();

    // Root arc start (at -half_tooth_root angle)
    let root_half_angle = tooth_angle / 2.0;

    // Root point before tooth
    {
        let a = -root_half_angle;
        profile_pts.push((root_radius * a.cos(), root_radius * a.sin()));
    }

    // Left involute flank (going outward)
    for i in 0..=involute_steps {
        let t = t_max * i as f64 / involute_steps as f64;
        let (ix, iy) = involute_point(base_radius, t);
        // Rotate involute so it's centered at -half_tooth
        let angle = -half_tooth;
        let rx = ix * angle.cos() - iy * angle.sin();
        let ry = ix * angle.sin() + iy * angle.cos();
        let r = (rx * rx + ry * ry).sqrt();
        if r >= root_radius && r <= outer_radius {
            profile_pts.push((rx, ry));
        }
    }

    // Right involute flank (mirrored, going inward from tip to root)
    for i in (0..=involute_steps).rev() {
        let t = t_max * i as f64 / involute_steps as f64;
        let (ix, iy) = involute_point(base_radius, t);
        // Mirror: negate y, then rotate by +half_tooth
        let mx = ix;
        let my = -iy;
        let angle = half_tooth;
        let rx = mx * angle.cos() - my * angle.sin();
        let ry = mx * angle.sin() + my * angle.cos();
        let r = (rx * rx + ry * ry).sqrt();
        if r >= root_radius && r <= outer_radius {
            profile_pts.push((rx, ry));
        }
    }

    // Root point after tooth
    {
        let a = root_half_angle;
        profile_pts.push((root_radius * a.cos(), root_radius * a.sin()));
    }

    // Replicate profile for all teeth around the gear
    let pts_per_tooth = profile_pts.len();
    let mut full_profile: Vec<(f64, f64)> = Vec::with_capacity(pts_per_tooth * num_teeth);

    for tooth in 0..num_teeth {
        let angle = tooth_angle * tooth as f64;
        let (sa, ca) = angle.sin_cos();
        for &(px, py) in &profile_pts {
            let rx = px * ca - py * sa;
            let ry = px * sa + py * ca;
            full_profile.push((rx, ry));
        }
    }

    let n = full_profile.len();

    // Create bottom and top vertices
    let mut bot_verts: Vec<Handle<VertexData>> = Vec::with_capacity(n);
    let mut top_verts: Vec<Handle<VertexData>> = Vec::with_capacity(n);

    for &(px, py) in &full_profile {
        let btag = Tag::generated(EntityKind::Vertex, op, vert_idx);
        vert_idx += 1;
        let bv = model.add_vertex_tagged(Point3::new(px, py, 0.0), btag);
        bot_verts.push(bv);

        let ttag = Tag::generated(EntityKind::Vertex, op, vert_idx);
        vert_idx += 1;
        let tv = model.add_vertex_tagged(Point3::new(px, py, face_width), ttag);
        top_verts.push(tv);
    }

    let mut faces = Vec::new();
    let mut face_idx = 0u32;

    // Bottom face (reversed winding for downward normal)
    {
        let mut hes = Vec::with_capacity(n);
        for i in (0..n).rev() {
            let j = if i == 0 { n - 1 } else { i - 1 };
            hes.push(ec.get_or_create(
                model,
                bot_verts[i],
                bot_verts[j],
                next_edge_tag(op, &mut edge_idx),
            ));
        }
        let loop_h = model.make_loop(&hes)?;
        let tag = Tag::generated(EntityKind::Face, op, face_idx);
        faces.push(model.make_face_tagged(loop_h, tag));
        face_idx += 1;
    }

    // Top face (forward winding for upward normal)
    {
        let mut hes = Vec::with_capacity(n);
        for i in 0..n {
            let j = (i + 1) % n;
            hes.push(ec.get_or_create(
                model,
                top_verts[i],
                top_verts[j],
                next_edge_tag(op, &mut edge_idx),
            ));
        }
        let loop_h = model.make_loop(&hes)?;
        let tag = Tag::generated(EntityKind::Face, op, face_idx);
        faces.push(model.make_face_tagged(loop_h, tag));
        face_idx += 1;
    }

    // Lateral faces (quads connecting bottom to top)
    for i in 0..n {
        let j = (i + 1) % n;
        let hes = [
            ec.get_or_create(
                model,
                bot_verts[i],
                bot_verts[j],
                next_edge_tag(op, &mut edge_idx),
            ),
            ec.get_or_create(
                model,
                bot_verts[j],
                top_verts[j],
                next_edge_tag(op, &mut edge_idx),
            ),
            ec.get_or_create(
                model,
                top_verts[j],
                top_verts[i],
                next_edge_tag(op, &mut edge_idx),
            ),
            ec.get_or_create(
                model,
                top_verts[i],
                bot_verts[i],
                next_edge_tag(op, &mut edge_idx),
            ),
        ];
        let loop_h = model.make_loop(&hes)?;
        let tag = Tag::generated(EntityKind::Face, op, face_idx);
        faces.push(model.make_face_tagged(loop_h, tag));
        face_idx += 1;
    }
    let _ = face_idx;

    // Geometry binding for top and bottom faces
    let bot_plane = Plane::new(Point3::ORIGIN, Vec3::X, -Vec3::Y);
    if let Ok(bp) = bot_plane {
        model.bind_face_surface(faces[0], Arc::new(bp), Orientation::Forward);
    }
    let top_plane = Plane::new(Point3::new(0.0, 0.0, face_width), Vec3::X, Vec3::Y);
    if let Ok(tp) = top_plane {
        model.bind_face_surface(faces[1], Arc::new(tp), Orientation::Forward);
    }

    bind_edge_line_segments(model, &ec);

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell = model.make_shell_tagged(&faces, shell_tag);
    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let solid = model.make_solid_tagged(&[shell], solid_tag);

    Ok(GearResult { solid, faces })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gear_20_teeth() {
        let mut model = BRepModel::new();
        let pressure_20deg = 20.0_f64.to_radians();
        let r = make_involute_gear(&mut model, 2.0, 20, pressure_20deg, 10.0).unwrap();
        assert!(model.solids.is_alive(r.solid));
        // At minimum: 2 caps + N lateral faces
        assert!(r.faces.len() > 2);
    }

    #[test]
    fn test_gear_3_teeth() {
        let mut model = BRepModel::new();
        let pressure_20deg = 20.0_f64.to_radians();
        let r = make_involute_gear(&mut model, 5.0, 3, pressure_20deg, 5.0).unwrap();
        assert!(model.solids.is_alive(r.solid));
    }

    #[test]
    fn test_gear_validation() {
        let mut model = BRepModel::new();
        let pa = 20.0_f64.to_radians();
        assert!(make_involute_gear(&mut model, -1.0, 20, pa, 10.0).is_err());
        assert!(make_involute_gear(&mut model, 2.0, 2, pa, 10.0).is_err());
        assert!(make_involute_gear(&mut model, 2.0, 20, 0.0, 10.0).is_err());
        assert!(make_involute_gear(&mut model, 2.0, 20, pa, -1.0).is_err());
    }
}
