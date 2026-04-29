//! Additive and subtractive primitive operations for PartDesign.
//!
//! Each function creates a primitive in a temporary model and performs a
//! boolean operation (union or subtraction) against a base solid.

use cadkernel_core::KernelResult;
use cadkernel_math::Point3;
use cadkernel_topology::BRepModel;

use crate::boolean::{BooleanOp, boolean_op};
use crate::features::loft::loft;
use crate::features::sweep::sweep;
use crate::primitives::{
    make_box, make_cone, make_cylinder, make_ellipsoid, make_helix, make_prism, make_sphere,
    make_torus, make_wedge,
};

/// Additive box: creates a box and boolean-unions it with the base solid.
pub fn additive_box(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    origin: Point3,
    dx: f64,
    dy: f64,
    dz: f64,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_box(&mut tool, origin, dx, dy, dz)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Union)
}

/// Subtractive box: creates a box and boolean-subtracts it from the base solid.
pub fn subtractive_box(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    origin: Point3,
    dx: f64,
    dy: f64,
    dz: f64,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_box(&mut tool, origin, dx, dy, dz)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Difference)
}

/// Additive cylinder: creates a cylinder and boolean-unions it with the base solid.
pub fn additive_cylinder(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    base_center: Point3,
    radius: f64,
    height: f64,
    segments: usize,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_cylinder(&mut tool, base_center, radius, height, segments)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Union)
}

/// Subtractive cylinder: creates a cylinder and boolean-subtracts it from the base solid.
pub fn subtractive_cylinder(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    base_center: Point3,
    radius: f64,
    height: f64,
    segments: usize,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_cylinder(&mut tool, base_center, radius, height, segments)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Difference)
}

/// Additive sphere: creates a sphere and boolean-unions it with the base solid.
pub fn additive_sphere(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    center: Point3,
    radius: f64,
    segments: usize,
    rings: usize,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_sphere(&mut tool, center, radius, segments, rings)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Union)
}

/// Subtractive sphere: creates a sphere and boolean-subtracts it from the base solid.
pub fn subtractive_sphere(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    center: Point3,
    radius: f64,
    segments: usize,
    rings: usize,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_sphere(&mut tool, center, radius, segments, rings)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Difference)
}

/// Additive cone: creates a cone and boolean-unions it with the base solid.
pub fn additive_cone(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    base_center: Point3,
    base_radius: f64,
    top_radius: f64,
    height: f64,
    segments: usize,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_cone(&mut tool, base_center, base_radius, top_radius, height, segments)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Union)
}

/// Subtractive cone: creates a cone and boolean-subtracts it from the base solid.
pub fn subtractive_cone(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    base_center: Point3,
    base_radius: f64,
    top_radius: f64,
    height: f64,
    segments: usize,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_cone(&mut tool, base_center, base_radius, top_radius, height, segments)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Difference)
}

/// Additive torus: creates a torus and boolean-unions it with the base solid.
pub fn additive_torus(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    center: Point3,
    major_radius: f64,
    minor_radius: f64,
    major_segments: usize,
    minor_segments: usize,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_torus(&mut tool, center, major_radius, minor_radius, major_segments, minor_segments)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Union)
}

/// Subtractive torus: creates a torus and boolean-subtracts it from the base solid.
pub fn subtractive_torus(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    center: Point3,
    major_radius: f64,
    minor_radius: f64,
    major_segments: usize,
    minor_segments: usize,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_torus(&mut tool, center, major_radius, minor_radius, major_segments, minor_segments)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Difference)
}

/// Additive helix: creates a helix and boolean-unions it with the base solid.
#[allow(clippy::too_many_arguments)]
pub fn additive_helix(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    center: Point3,
    radius: f64,
    pitch: f64,
    turns: f64,
    tube_radius: f64,
    segments: usize,
    tube_segments: usize,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_helix(&mut tool, center, radius, pitch, turns, tube_radius, segments, tube_segments)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Union)
}

/// Subtractive helix: creates a helix and boolean-subtracts it from the base solid.
#[allow(clippy::too_many_arguments)]
pub fn subtractive_helix(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    center: Point3,
    radius: f64,
    pitch: f64,
    turns: f64,
    tube_radius: f64,
    segments: usize,
    tube_segments: usize,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_helix(&mut tool, center, radius, pitch, turns, tube_radius, segments, tube_segments)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Difference)
}

/// Additive ellipsoid: creates an ellipsoid and boolean-unions it with the base solid.
#[allow(clippy::too_many_arguments)]
pub fn additive_ellipsoid(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    center: Point3,
    rx: f64,
    ry: f64,
    rz: f64,
    segments: usize,
    rings: usize,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_ellipsoid(&mut tool, center, rx, ry, rz, segments, rings)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Union)
}

/// Subtractive ellipsoid: creates an ellipsoid and boolean-subtracts it from the base solid.
#[allow(clippy::too_many_arguments)]
pub fn subtractive_ellipsoid(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    center: Point3,
    rx: f64,
    ry: f64,
    rz: f64,
    segments: usize,
    rings: usize,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_ellipsoid(&mut tool, center, rx, ry, rz, segments, rings)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Difference)
}

/// Additive prism: creates a regular polygon prism and boolean-unions it with the base solid.
pub fn additive_prism(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    base_center: Point3,
    radius: f64,
    height: f64,
    sides: usize,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_prism(&mut tool, base_center, radius, height, sides)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Union)
}

/// Subtractive prism: creates a regular polygon prism and boolean-subtracts it from the base solid.
pub fn subtractive_prism(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    base_center: Point3,
    radius: f64,
    height: f64,
    sides: usize,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_prism(&mut tool, base_center, radius, height, sides)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Difference)
}

/// Additive wedge: creates a tapered box and boolean-unions it with the base solid.
#[allow(clippy::too_many_arguments)]
pub fn additive_wedge(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    origin: Point3,
    dx: f64,
    dy: f64,
    dz: f64,
    dx2: f64,
    dy2: f64,
    xoff: f64,
    yoff: f64,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_wedge(&mut tool, origin, dx, dy, dz, dx2, dy2, xoff, yoff)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Union)
}

/// Subtractive wedge: creates a tapered box and boolean-subtracts it from the base solid.
#[allow(clippy::too_many_arguments)]
pub fn subtractive_wedge(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    origin: Point3,
    dx: f64,
    dy: f64,
    dz: f64,
    dx2: f64,
    dy2: f64,
    xoff: f64,
    yoff: f64,
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = make_wedge(&mut tool, origin, dx, dy, dz, dx2, dy2, xoff, yoff)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Difference)
}

/// Additive loft: lofts between profiles and boolean-unions with the base solid.
pub fn additive_loft(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    profiles: &[&[Point3]],
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = loft(&mut tool, profiles)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Union)
}

/// Additive pipe (sweep): sweeps a profile along a path and boolean-unions with the base solid.
pub fn additive_pipe(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    profile: &[Point3],
    path: &[Point3],
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = sweep(&mut tool, profile, path)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Union)
}

/// Creates a parametric sprocket solid.
///
/// Builds a sprocket gear with the given number of teeth, roller diameter, pitch
/// (chain pitch), and bore radius. The sprocket has thickness = roller_diameter.
pub fn make_sprocket(
    model: &mut BRepModel,
    num_teeth: usize,
    roller_diameter: f64,
    pitch: f64,
    bore_radius: f64,
) -> KernelResult<BRepModel> {
    use std::f64::consts::TAU;

    if num_teeth < 4 {
        return Err(cadkernel_core::KernelError::InvalidArgument(
            "sprocket needs at least 4 teeth".into(),
        ));
    }
    if roller_diameter <= 0.0 || pitch <= 0.0 || bore_radius <= 0.0 {
        return Err(cadkernel_core::KernelError::InvalidArgument(
            "all dimensions must be positive".into(),
        ));
    }

    let z = num_teeth as f64;
    let pitch_radius = pitch / (2.0 * (std::f64::consts::PI / z).sin());
    let roller_radius = roller_diameter / 2.0;
    let outer_radius = pitch_radius + roller_radius;
    let root_radius = pitch_radius - roller_radius;

    if bore_radius >= root_radius {
        return Err(cadkernel_core::KernelError::InvalidArgument(
            "bore_radius must be smaller than root radius".into(),
        ));
    }

    let thickness = roller_diameter;
    let tooth_angle = TAU / z;
    let profile_steps = 6;
    let mut profile_pts = Vec::new();

    for i in 0..num_teeth {
        let base_angle = tooth_angle * i as f64;
        let half_tooth = tooth_angle * 0.3;

        // Root arc on left side
        let a0 = base_angle - half_tooth;
        profile_pts.push(Point3::new(
            root_radius * a0.cos(),
            root_radius * a0.sin(),
            0.0,
        ));

        // Rise to outer
        for s in 1..profile_steps {
            let frac = s as f64 / profile_steps as f64;
            let r = root_radius + (outer_radius - root_radius) * frac;
            let a = a0 + half_tooth * frac;
            profile_pts.push(Point3::new(r * a.cos(), r * a.sin(), 0.0));
        }

        // Tip
        profile_pts.push(Point3::new(
            outer_radius * base_angle.cos(),
            outer_radius * base_angle.sin(),
            0.0,
        ));

        // Descend from outer
        let a1 = base_angle + half_tooth;
        for s in 1..profile_steps {
            let frac = s as f64 / profile_steps as f64;
            let r = outer_radius - (outer_radius - root_radius) * frac;
            let a = base_angle + half_tooth * frac;
            profile_pts.push(Point3::new(r * a.cos(), r * a.sin(), 0.0));
        }

        // Root arc on right side
        profile_pts.push(Point3::new(
            root_radius * a1.cos(),
            root_radius * a1.sin(),
            0.0,
        ));
    }

    let top_profile: Vec<Point3> = profile_pts
        .iter()
        .map(|p| Point3::new(p.x, p.y, thickness))
        .collect();

    let profiles: Vec<&[Point3]> = vec![profile_pts.as_slice(), top_profile.as_slice()];
    let mut tool = BRepModel::new();
    loft(&mut tool, &profiles)?;

    let first_solid = tool
        .solids
        .iter()
        .next()
        .map(|(h, _)| h)
        .ok_or(cadkernel_core::KernelError::InvalidArgument(
            "loft produced no solid".into(),
        ))?;

    // Subtract bore hole
    let mut bore_model = BRepModel::new();
    let bore_r = crate::primitives::make_cylinder(
        &mut bore_model,
        Point3::new(0.0, 0.0, -0.1),
        bore_radius,
        thickness + 0.2,
        32,
    )?;

    let result =
        boolean_op(&tool, first_solid, &bore_model, bore_r.solid, BooleanOp::Difference)?;
    let _ = model;
    Ok(result)
}

/// Creates a stepped shaft from a sequence of (length, diameter) segments.
///
/// Each segment is a cylinder centered on the Z axis, stacked end-to-end.
/// The first segment starts at Z=0.
pub fn shaft_design(
    model: &mut BRepModel,
    segments: &[(f64, f64)],
) -> KernelResult<BRepModel> {
    if segments.is_empty() {
        return Err(cadkernel_core::KernelError::InvalidArgument(
            "shaft_design requires at least 1 segment".into(),
        ));
    }
    for (i, &(length, diameter)) in segments.iter().enumerate() {
        if length <= 0.0 || diameter <= 0.0 {
            return Err(cadkernel_core::KernelError::InvalidArgument(format!(
                "segment {}: length and diameter must be positive",
                i
            )));
        }
    }

    let mut z_offset = 0.0;
    let mut result_model: Option<BRepModel> = None;

    for &(length, diameter) in segments {
        let radius = diameter / 2.0;
        let mut seg_model = BRepModel::new();
        let r = crate::primitives::make_cylinder(
            &mut seg_model,
            Point3::new(0.0, 0.0, z_offset),
            radius,
            length,
            32,
        )?;

        result_model = Some(match result_model {
            None => seg_model,
            Some(prev) => {
                let prev_solid = prev.solids.iter().next().map(|(h, _)| h).ok_or_else(|| {
                    cadkernel_core::KernelError::ValidationFailed(
                        "shaft_design: prior boolean union produced an empty model".into(),
                    )
                })?;
                boolean_op(&prev, prev_solid, &seg_model, r.solid, BooleanOp::Union)?
            }
        });
        z_offset += length;
    }

    let _ = model;
    result_model.ok_or_else(|| {
        cadkernel_core::KernelError::ValidationFailed(
            "shaft_design: no result produced (unreachable when segments are validated non-empty)".into(),
        )
    })
}

/// Creates a reference copy of selected faces from a source model.
///
/// Copies the specified face indices (and their underlying geometry) into a
/// new solid. Used for PartDesign ShapeBinder.
pub fn shape_binder(
    source_model: &BRepModel,
    face_indices: &[usize],
) -> KernelResult<BRepModel> {
    if face_indices.is_empty() {
        return Err(cadkernel_core::KernelError::InvalidArgument(
            "shape_binder requires at least 1 face index".into(),
        ));
    }

    let all_faces: Vec<_> = source_model.faces.iter().map(|(h, _)| h).collect();
    let mut selected = Vec::new();
    for &idx in face_indices {
        if idx >= all_faces.len() {
            return Err(cadkernel_core::KernelError::InvalidArgument(format!(
                "face index {} out of range (model has {} faces)",
                idx,
                all_faces.len()
            )));
        }
        selected.push(all_faces[idx]);
    }

    let mut result = BRepModel::new();
    let op = result.history.next_operation("shape_binder");

    let mut new_faces = Vec::new();
    for (fi, &face_h) in selected.iter().enumerate() {
        let verts = source_model.vertices_of_face(face_h)?;
        let mut new_verts = Vec::new();
        for (vi, &vh) in verts.iter().enumerate() {
            let pt = source_model.vertices.get(vh).map(|v| v.point).unwrap_or(Point3::ORIGIN);
            let tag = cadkernel_topology::Tag::generated(
                cadkernel_topology::EntityKind::Vertex, op, (fi * 100 + vi) as u32,
            );
            new_verts.push(result.add_vertex_tagged(pt, tag));
        }

        let mut he_list = Vec::new();
        for i in 0..new_verts.len() {
            let j = (i + 1) % new_verts.len();
            let tag = cadkernel_topology::Tag::generated(
                cadkernel_topology::EntityKind::Edge, op, (fi * 100 + i) as u32,
            );
            let (_, fwd, _) = result.add_edge_tagged(new_verts[i], new_verts[j], tag);
            he_list.push(fwd);
        }

        if he_list.len() >= 2 {
            let loop_h = result.make_loop(&he_list)?;
            let face_tag = cadkernel_topology::Tag::generated(
                cadkernel_topology::EntityKind::Face, op, fi as u32,
            );
            new_faces.push(result.make_face_tagged(loop_h, face_tag));
        }
    }

    if !new_faces.is_empty() {
        let shell_tag = cadkernel_topology::Tag::generated(
            cadkernel_topology::EntityKind::Shell, op, 0,
        );
        let shell = result.make_shell_tagged(&new_faces, shell_tag);
        let solid_tag = cadkernel_topology::Tag::generated(
            cadkernel_topology::EntityKind::Solid, op, 0,
        );
        result.make_solid_tagged(&[shell], solid_tag);
    }

    Ok(result)
}

/// Creates a reference copy of selected edges from a source model.
///
/// Copies the specified edge indices into a new model as wire edges.
/// Used for PartDesign SubShapeBinder.
pub fn sub_shape_binder(
    source_model: &BRepModel,
    edge_indices: &[usize],
) -> KernelResult<BRepModel> {
    if edge_indices.is_empty() {
        return Err(cadkernel_core::KernelError::InvalidArgument(
            "sub_shape_binder requires at least 1 edge index".into(),
        ));
    }

    let all_edges: Vec<_> = source_model.edges.iter().map(|(h, _)| h).collect();

    let mut result = BRepModel::new();
    let op = result.history.next_operation("sub_shape_binder");
    let mut vert_idx = 0u32;

    for (edge_idx, &idx) in edge_indices.iter().enumerate() {
        if idx >= all_edges.len() {
            return Err(cadkernel_core::KernelError::InvalidArgument(format!(
                "edge index {} out of range (model has {} edges)",
                idx,
                all_edges.len()
            )));
        }

        let edge_h = all_edges[idx];
        let ed = source_model.edges.get(edge_h)
            .ok_or(cadkernel_core::KernelError::InvalidHandle("edge"))?;

        let p0 = ed.half_edge_a
            .and_then(|h| source_model.half_edges.get(h))
            .and_then(|he| source_model.vertices.get(he.origin))
            .map(|v| v.point)
            .unwrap_or(Point3::ORIGIN);
        let p1 = ed.half_edge_b
            .and_then(|h| source_model.half_edges.get(h))
            .and_then(|he| source_model.vertices.get(he.origin))
            .map(|v| v.point)
            .unwrap_or(Point3::ORIGIN);

        let tag_s = cadkernel_topology::Tag::generated(
            cadkernel_topology::EntityKind::Vertex, op, vert_idx,
        );
        vert_idx += 1;
        let tag_e = cadkernel_topology::Tag::generated(
            cadkernel_topology::EntityKind::Vertex, op, vert_idx,
        );
        vert_idx += 1;

        let v0 = result.add_vertex_tagged(p0, tag_s);
        let v1 = result.add_vertex_tagged(p1, tag_e);

        let edge_tag = cadkernel_topology::Tag::generated(
            cadkernel_topology::EntityKind::Edge, op, edge_idx as u32,
        );
        result.add_edge_tagged(v0, v1, edge_tag);
    }

    Ok(result)
}

/// Subtractive loft: lofts between profiles and boolean-subtracts from the base solid.
pub fn subtractive_loft(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    profiles: &[&[Point3]],
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = loft(&mut tool, profiles)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Difference)
}

/// Subtractive pipe (sweep): sweeps a profile along a path and boolean-subtracts from the base solid.
pub fn subtractive_pipe(
    base_model: &BRepModel,
    base_solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>,
    profile: &[Point3],
    path: &[Point3],
) -> KernelResult<BRepModel> {
    let mut tool = BRepModel::new();
    let r = sweep(&mut tool, profile, path)?;
    boolean_op(base_model, base_solid, &tool, r.solid, BooleanOp::Difference)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_additive_box() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let result = additive_box(
            &base_model,
            base.solid,
            Point3::new(10.0, 10.0, 10.0),
            1.0,
            1.0,
            1.0,
        )
        .unwrap();

        // Union of two disjoint boxes should produce 1 solid with all 12 faces
        assert_eq!(result.solids.len(), 1);
        assert!(result.faces.len() >= 12);
    }

    #[test]
    fn test_subtractive_box() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let result = subtractive_box(
            &base_model,
            base.solid,
            Point3::new(10.0, 10.0, 10.0),
            1.0,
            1.0,
            1.0,
        )
        .unwrap();

        // Subtracting a disjoint box should preserve the original 6 faces
        assert_eq!(result.faces.len(), 6);
    }

    #[test]
    fn test_additive_cylinder() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let result = additive_cylinder(
            &base_model,
            base.solid,
            Point3::new(10.0, 10.0, 0.0),
            1.0,
            3.0,
            8,
        )
        .unwrap();

        assert_eq!(result.solids.len(), 1);
    }

    #[test]
    fn test_subtractive_cylinder() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let result = subtractive_cylinder(
            &base_model,
            base.solid,
            Point3::new(10.0, 10.0, 0.0),
            0.5,
            4.0,
            8,
        )
        .unwrap();

        // Disjoint subtraction preserves original
        assert_eq!(result.faces.len(), 6);
    }

    #[test]
    fn test_additive_sphere() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let result = additive_sphere(
            &base_model,
            base.solid,
            Point3::new(10.0, 10.0, 10.0),
            1.0,
            8,
            4,
        )
        .unwrap();

        assert_eq!(result.solids.len(), 1);
    }

    #[test]
    fn test_subtractive_sphere() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let result = subtractive_sphere(
            &base_model,
            base.solid,
            Point3::new(10.0, 10.0, 10.0),
            0.5,
            8,
            4,
        )
        .unwrap();

        assert_eq!(result.faces.len(), 6);
    }

    #[test]
    fn test_additive_cone() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let result = additive_cone(
            &base_model,
            base.solid,
            Point3::new(10.0, 10.0, 0.0),
            1.0,
            0.5,
            3.0,
            8,
        )
        .unwrap();

        assert_eq!(result.solids.len(), 1);
    }

    #[test]
    fn test_subtractive_cone() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let result = subtractive_cone(
            &base_model,
            base.solid,
            Point3::new(10.0, 10.0, 0.0),
            0.5,
            0.2,
            3.0,
            8,
        )
        .unwrap();

        assert_eq!(result.faces.len(), 6);
    }

    #[test]
    fn test_additive_torus() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let result = additive_torus(
            &base_model,
            base.solid,
            Point3::new(20.0, 20.0, 20.0),
            3.0,
            0.5,
            8,
            6,
        )
        .unwrap();

        assert_eq!(result.solids.len(), 1);
    }

    #[test]
    fn test_subtractive_torus() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let result = subtractive_torus(
            &base_model,
            base.solid,
            Point3::new(20.0, 20.0, 20.0),
            3.0,
            0.5,
            8,
            6,
        )
        .unwrap();

        assert_eq!(result.faces.len(), 6);
    }

    #[test]
    fn test_additive_helix() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let result = additive_helix(
            &base_model,
            base.solid,
            Point3::new(20.0, 20.0, 0.0),
            3.0, 2.0, 1.0, 0.3, 8, 4,
        ).unwrap();

        assert_eq!(result.solids.len(), 1);
    }

    #[test]
    fn test_subtractive_helix() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let result = subtractive_helix(
            &base_model,
            base.solid,
            Point3::new(20.0, 20.0, 0.0),
            3.0, 2.0, 1.0, 0.3, 8, 4,
        ).unwrap();

        assert_eq!(result.faces.len(), 6);
    }

    #[test]
    fn test_additive_ellipsoid() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let result = additive_ellipsoid(
            &base_model,
            base.solid,
            Point3::new(20.0, 20.0, 20.0),
            1.0, 1.5, 2.0, 8, 4,
        ).unwrap();

        assert_eq!(result.solids.len(), 1);
    }

    #[test]
    fn test_subtractive_ellipsoid() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let result = subtractive_ellipsoid(
            &base_model,
            base.solid,
            Point3::new(20.0, 20.0, 20.0),
            0.5, 0.5, 0.5, 8, 4,
        ).unwrap();

        assert_eq!(result.faces.len(), 6);
    }

    #[test]
    fn test_additive_prism() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let result = additive_prism(
            &base_model,
            base.solid,
            Point3::new(20.0, 20.0, 0.0),
            1.0, 3.0, 6,
        ).unwrap();

        assert_eq!(result.solids.len(), 1);
    }

    #[test]
    fn test_subtractive_prism() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let result = subtractive_prism(
            &base_model,
            base.solid,
            Point3::new(20.0, 20.0, 0.0),
            0.5, 3.0, 6,
        ).unwrap();

        assert_eq!(result.faces.len(), 6);
    }

    #[test]
    fn test_additive_wedge() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let result = additive_wedge(
            &base_model,
            base.solid,
            Point3::new(20.0, 20.0, 0.0),
            2.0, 2.0, 3.0, 1.0, 1.0, 0.5, 0.5,
        ).unwrap();

        assert_eq!(result.solids.len(), 1);
    }

    #[test]
    fn test_subtractive_wedge() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let result = subtractive_wedge(
            &base_model,
            base.solid,
            Point3::new(20.0, 20.0, 0.0),
            1.0, 1.0, 2.0, 0.5, 0.5, 0.25, 0.25,
        ).unwrap();

        assert_eq!(result.faces.len(), 6);
    }

    #[test]
    fn test_subtractive_loft() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let p0 = vec![
            Point3::new(20.0, 20.0, 0.0),
            Point3::new(21.0, 20.0, 0.0),
            Point3::new(21.0, 21.0, 0.0),
            Point3::new(20.0, 21.0, 0.0),
        ];
        let p1 = vec![
            Point3::new(20.0, 20.0, 3.0),
            Point3::new(21.0, 20.0, 3.0),
            Point3::new(21.0, 21.0, 3.0),
            Point3::new(20.0, 21.0, 3.0),
        ];

        let result = subtractive_loft(
            &base_model,
            base.solid,
            &[p0.as_slice(), p1.as_slice()],
        ).unwrap();

        assert_eq!(result.faces.len(), 6);
    }

    #[test]
    fn test_subtractive_pipe() {
        let mut base_model = BRepModel::new();
        let base = make_box(&mut base_model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();

        let profile = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.3, 0.0, 0.0),
            Point3::new(0.3, 0.3, 0.0),
            Point3::new(0.0, 0.3, 0.0),
        ];
        let path = vec![
            Point3::new(20.0, 20.0, 0.0),
            Point3::new(20.0, 20.0, 3.0),
            Point3::new(20.0, 20.0, 6.0),
        ];

        let result = subtractive_pipe(
            &base_model,
            base.solid,
            &profile,
            &path,
        ).unwrap();

        assert_eq!(result.faces.len(), 6);
    }
}
