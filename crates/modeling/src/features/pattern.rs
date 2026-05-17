//! Linear and circular pattern (array) operations.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Quaternion, Vec3};
use cadkernel_topology::{BRepModel, FaceData, Handle, SolidData};

use super::copy_utils::copy_solid_transformed;

/// Result of a pattern operation.
#[derive(Debug)]
pub struct PatternResult {
    pub solids: Vec<Handle<SolidData>>,
    pub faces: Vec<Handle<FaceData>>,
}

/// One table-driven pattern row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TablePatternRow {
    pub position: Vec3,
    pub rotation: Vec3,
    pub scale: f64,
}

/// Creates a linear pattern of a solid along a direction.
///
/// `count` copies are placed at `spacing` intervals along `direction`.
/// The original solid is included as the first entry.
pub fn linear_pattern(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    direction: Vec3,
    spacing: f64,
    count: usize,
) -> KernelResult<PatternResult> {
    if count < 2 {
        return Err(KernelError::InvalidArgument(
            "pattern count must be at least 2".into(),
        ));
    }
    let dir = direction.normalized().ok_or(KernelError::InvalidArgument(
        "pattern direction must be non-zero".into(),
    ))?;

    let mut solids = vec![solid];
    let mut faces = Vec::new();

    for i in 1..count {
        let offset_x = dir.x * spacing * i as f64;
        let offset_y = dir.y * spacing * i as f64;
        let offset_z = dir.z * spacing * i as f64;

        let op = model.history.next_operation("linear_pattern");
        let result = copy_solid_transformed(
            model,
            solid,
            op,
            |pt| Point3::new(pt.x + offset_x, pt.y + offset_y, pt.z + offset_z),
            false,
        )?;
        solids.push(result.solid);
        faces.extend(result.faces);
    }

    Ok(PatternResult { solids, faces })
}

/// Creates a circular pattern of a solid around an axis.
///
/// `count` copies are placed at equal angular intervals (full 360°) around
/// the axis defined by `axis_origin` and `axis_dir`.
/// The original solid is included as the first entry.
pub fn circular_pattern(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    axis_origin: Point3,
    axis_dir: Vec3,
    count: usize,
) -> KernelResult<PatternResult> {
    if count < 2 {
        return Err(KernelError::InvalidArgument(
            "pattern count must be at least 2".into(),
        ));
    }
    let axis = axis_dir.normalized().ok_or(KernelError::InvalidArgument(
        "pattern axis must be non-zero".into(),
    ))?;

    let angle_step = std::f64::consts::TAU / count as f64;

    let mut solids = vec![solid];
    let mut faces = Vec::new();

    for i in 1..count {
        let angle = angle_step * i as f64;
        let q = Quaternion::from_axis_angle(axis, angle);

        let origin = axis_origin;
        let op = model.history.next_operation("circular_pattern");
        let result = copy_solid_transformed(
            model,
            solid,
            op,
            move |pt| {
                // Translate to origin, rotate, translate back.
                let rel = Vec3::new(pt.x - origin.x, pt.y - origin.y, pt.z - origin.z);
                let rotated = q.rotate_vec(rel);
                Point3::new(
                    origin.x + rotated.x,
                    origin.y + rotated.y,
                    origin.z + rotated.z,
                )
            },
            false,
        )?;
        solids.push(result.solid);
        faces.extend(result.faces);
    }

    Ok(PatternResult { solids, faces })
}

/// Creates a circular feature pattern around an axis with an explicit angular span.
///
/// `count` includes the original. `angle_rad` is divided by `count`, matching
/// full-circle pattern behavior without duplicating the original at 360°.
pub fn circular_pattern_features(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    axis_origin: Point3,
    axis_dir: Vec3,
    count: usize,
    angle_rad: f64,
) -> KernelResult<PatternResult> {
    if count < 2 {
        return Err(KernelError::InvalidArgument(
            "pattern count must be at least 2".into(),
        ));
    }
    if !angle_rad.is_finite() || angle_rad.abs() < 1e-12 {
        return Err(KernelError::InvalidArgument(
            "pattern angle must be finite and non-zero".into(),
        ));
    }
    let axis = axis_dir.normalized().ok_or(KernelError::InvalidArgument(
        "pattern axis must be non-zero".into(),
    ))?;

    let angle_step = angle_rad / count as f64;
    let mut solids = vec![solid];
    let mut faces = Vec::new();

    for i in 1..count {
        let angle = angle_step * i as f64;
        let q = Quaternion::from_axis_angle(axis, angle);
        let origin = axis_origin;
        let op = model.history.next_operation("circular_pattern_features");
        let result = copy_solid_transformed(
            model,
            solid,
            op,
            move |pt| {
                let rel = Vec3::new(pt.x - origin.x, pt.y - origin.y, pt.z - origin.z);
                let rotated = q.rotate_vec(rel);
                Point3::new(
                    origin.x + rotated.x,
                    origin.y + rotated.y,
                    origin.z + rotated.z,
                )
            },
            false,
        )?;
        solids.push(result.solid);
        faces.extend(result.faces);
    }

    Ok(PatternResult { solids, faces })
}

/// Creates one translated instance for each driver sketch point.
pub fn sketch_driven_pattern_features(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    points: &[Point3],
) -> KernelResult<PatternResult> {
    if points.is_empty() {
        return Err(KernelError::InvalidArgument(
            "sketch-driven pattern requires at least one point".into(),
        ));
    }
    let mut solids = vec![solid];
    let mut faces = Vec::new();
    for point in points {
        let offset = Vec3::new(point.x, point.y, point.z);
        let op = model.history.next_operation("sketch_driven_pattern");
        let result = copy_solid_transformed(
            model,
            solid,
            op,
            move |pt| Point3::new(pt.x + offset.x, pt.y + offset.y, pt.z + offset.z),
            false,
        )?;
        solids.push(result.solid);
        faces.extend(result.faces);
    }
    Ok(PatternResult { solids, faces })
}

/// Creates one transformed instance per table row.
pub fn table_driven_pattern_features(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    rows: &[TablePatternRow],
) -> KernelResult<PatternResult> {
    if rows.is_empty() {
        return Err(KernelError::InvalidArgument(
            "table-driven pattern requires at least one row".into(),
        ));
    }
    for row in rows {
        if !row.position.x.is_finite()
            || !row.position.y.is_finite()
            || !row.position.z.is_finite()
            || !row.rotation.x.is_finite()
            || !row.rotation.y.is_finite()
            || !row.rotation.z.is_finite()
            || !row.scale.is_finite()
            || row.scale <= 0.0
        {
            return Err(KernelError::InvalidArgument(
                "table-driven pattern row must be finite and scale must be > 0".into(),
            ));
        }
    }

    let mut solids = vec![solid];
    let mut faces = Vec::new();
    for row in rows {
        let qx = Quaternion::from_axis_angle(Vec3::X, row.rotation.x);
        let qy = Quaternion::from_axis_angle(Vec3::Y, row.rotation.y);
        let qz = Quaternion::from_axis_angle(Vec3::Z, row.rotation.z);
        let offset = row.position;
        let scale = row.scale;
        let op = model.history.next_operation("table_driven_pattern");
        let result = copy_solid_transformed(
            model,
            solid,
            op,
            move |pt| {
                let scaled = Vec3::new(pt.x * scale, pt.y * scale, pt.z * scale);
                let rotated = qz.rotate_vec(qy.rotate_vec(qx.rotate_vec(scaled)));
                Point3::new(
                    rotated.x + offset.x,
                    rotated.y + offset.y,
                    rotated.z + offset.z,
                )
            },
            false,
        )?;
        solids.push(result.solid);
        faces.extend(result.faces);
    }
    Ok(PatternResult { solids, faces })
}

/// Distributes copied instances across a target face.
pub fn fill_pattern_features(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    target_face: Handle<FaceData>,
    density: f64,
) -> KernelResult<PatternResult> {
    if !density.is_finite() || density <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "fill pattern density must be > 0".into(),
        ));
    }
    let positions = fill_positions(model, target_face, density)?;
    let mut solids = vec![solid];
    let mut faces = Vec::new();
    for point in positions {
        let offset = Vec3::new(point.x, point.y, point.z);
        let op = model.history.next_operation("fill_pattern");
        let result = copy_solid_transformed(
            model,
            solid,
            op,
            move |pt| Point3::new(pt.x + offset.x, pt.y + offset.y, pt.z + offset.z),
            false,
        )?;
        solids.push(result.solid);
        faces.extend(result.faces);
    }
    Ok(PatternResult { solids, faces })
}

fn fill_positions(
    model: &BRepModel,
    face: Handle<FaceData>,
    density: f64,
) -> KernelResult<Vec<Point3>> {
    let vertices = model.vertices_of_face(face)?;
    if vertices.len() < 3 {
        return Err(KernelError::InvalidArgument(
            "fill pattern target face has fewer than 3 vertices".into(),
        ));
    }
    let mut points = Vec::with_capacity(vertices.len());
    for vertex in vertices {
        points.push(
            model
                .vertices
                .get(vertex)
                .ok_or(KernelError::InvalidHandle("vertex"))?
                .point,
        );
    }
    let area = polygon_area(&points)?;
    let normal = polygon_normal(&points)?;
    let u_axis = face_u_axis(&points, normal)?;
    let v_axis = normal
        .cross(u_axis)
        .normalized()
        .ok_or(KernelError::InvalidArgument(
            "fill pattern target face basis is degenerate".into(),
        ))?;
    let origin = points[0];
    let mut min_u = f64::INFINITY;
    let mut max_u = f64::NEG_INFINITY;
    let mut min_v = f64::INFINITY;
    let mut max_v = f64::NEG_INFINITY;
    for point in &points {
        let rel = Vec3::new(point.x - origin.x, point.y - origin.y, point.z - origin.z);
        let u = rel.dot(u_axis);
        let v = rel.dot(v_axis);
        min_u = min_u.min(u);
        max_u = max_u.max(u);
        min_v = min_v.min(v);
        max_v = max_v.max(v);
    }
    let count = (area * density).ceil().max(1.0);
    if count > 10_000.0 {
        return Err(KernelError::InvalidArgument(
            "fill pattern instance count exceeds 10000".into(),
        ));
    }
    let count = count as usize;
    let cols = (count as f64).sqrt().ceil() as usize;
    let rows = count.div_ceil(cols);
    let du = (max_u - min_u) / (cols as f64 + 1.0);
    let dv = (max_v - min_v) / (rows as f64 + 1.0);
    let mut positions = Vec::with_capacity(count);
    for row in 0..rows {
        for col in 0..cols {
            if positions.len() == count {
                return Ok(positions);
            }
            let u = min_u + du * (col as f64 + 1.0);
            let v = min_v + dv * (row as f64 + 1.0);
            let rel = u_axis * u + v_axis * v;
            positions.push(Point3::new(
                origin.x + rel.x,
                origin.y + rel.y,
                origin.z + rel.z,
            ));
        }
    }
    Ok(positions)
}

fn polygon_area(points: &[Point3]) -> KernelResult<f64> {
    let origin = points[0];
    let mut area = 0.0;
    for i in 1..points.len() - 1 {
        let a = Vec3::new(
            points[i].x - origin.x,
            points[i].y - origin.y,
            points[i].z - origin.z,
        );
        let b = Vec3::new(
            points[i + 1].x - origin.x,
            points[i + 1].y - origin.y,
            points[i + 1].z - origin.z,
        );
        area += 0.5 * a.cross(b).length();
    }
    if area <= 1e-12 {
        return Err(KernelError::InvalidArgument(
            "fill pattern target face area is degenerate".into(),
        ));
    }
    Ok(area)
}

fn polygon_normal(points: &[Point3]) -> KernelResult<Vec3> {
    let mut normal = Vec3::ZERO;
    for i in 0..points.len() {
        let current = points[i];
        let next = points[(i + 1) % points.len()];
        normal.x += (current.y - next.y) * (current.z + next.z);
        normal.y += (current.z - next.z) * (current.x + next.x);
        normal.z += (current.x - next.x) * (current.y + next.y);
    }
    normal.normalized().ok_or(KernelError::InvalidArgument(
        "fill pattern target face normal is degenerate".into(),
    ))
}

fn face_u_axis(points: &[Point3], normal: Vec3) -> KernelResult<Vec3> {
    for i in 1..points.len() {
        let candidate = Vec3::new(
            points[i].x - points[0].x,
            points[i].y - points[0].y,
            points[i].z - points[0].z,
        );
        let in_plane = candidate - normal * candidate.dot(normal);
        if let Some(axis) = in_plane.normalized() {
            return Ok(axis);
        }
    }
    Err(KernelError::InvalidArgument(
        "fill pattern target face basis is degenerate".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::make_box;

    #[test]
    fn test_linear_pattern_3_copies() {
        let mut model = cadkernel_topology::BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let pr = linear_pattern(&mut model, r.solid, Vec3::X, 3.0, 3).unwrap();
        // 3 solids total (original + 2 copies)
        assert_eq!(pr.solids.len(), 3);
        // 2 copies * 6 faces = 12 new faces
        assert_eq!(pr.faces.len(), 12);
        assert_eq!(model.solids.len(), 3);
    }

    #[test]
    fn test_circular_pattern_4_copies() {
        let mut model = cadkernel_topology::BRepModel::new();
        let r = make_box(&mut model, Point3::new(3.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap();

        let pr = circular_pattern(&mut model, r.solid, Point3::ORIGIN, Vec3::Z, 4).unwrap();
        assert_eq!(pr.solids.len(), 4);
        // 3 copies * 6 faces = 18 new faces
        assert_eq!(pr.faces.len(), 18);

        // Check that a copy exists roughly at (0, 3, 0) — 90° rotation around Z
        let has_rotated = model
            .vertices
            .iter()
            .any(|(_, v)| (v.point.y - 3.0).abs() < 0.5 && v.point.x.abs() < 0.5);
        assert!(has_rotated, "expected rotated vertex near (0, 3, 0)");
    }

    #[test]
    fn test_pattern_validation() {
        let mut model = cadkernel_topology::BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        assert!(linear_pattern(&mut model, r.solid, Vec3::X, 2.0, 1).is_err());
        assert!(circular_pattern(&mut model, r.solid, Point3::ORIGIN, Vec3::Z, 1).is_err());
    }
}
