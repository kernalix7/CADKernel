//! Draft workbench operations: wire construction, clone, and array patterns.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_geometry::{Curve, NurbsCurve};
use cadkernel_math::{Point3, Vec3};
use cadkernel_topology::{BRepModel, EdgeData, EntityKind, FaceData, Handle, SolidData, Tag, VertexData};

use crate::features::copy_utils::copy_solid_transformed;

/// Result of [`make_wire`].
#[derive(Debug)]
pub struct WireResult {
    pub vertices: Vec<Handle<VertexData>>,
    pub edges: Vec<Handle<EdgeData>>,
}

/// Result of [`make_bspline_wire`].
#[derive(Debug)]
pub struct BSplineWireResult {
    pub vertices: Vec<Handle<VertexData>>,
    pub edges: Vec<Handle<EdgeData>>,
    pub curve: NurbsCurve,
}

/// Result of [`clone_solid`].
#[derive(Debug)]
pub struct CloneResult {
    pub solid: Handle<SolidData>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Result of [`rectangular_array`] and [`path_array`].
#[derive(Debug)]
pub struct ArrayResult {
    pub solids: Vec<Handle<SolidData>>,
    pub faces: Vec<Handle<FaceData>>,
}

/// Creates a 3D polyline wire from a sequence of points.
///
/// Each consecutive pair of points becomes a vertex and an edge.
/// If the first and last points coincide (within tolerance), the wire is
/// closed automatically and the last duplicate vertex is omitted.
///
/// Requires at least 2 points.
pub fn make_wire(model: &mut BRepModel, points: &[Point3]) -> KernelResult<WireResult> {
    if points.len() < 2 {
        return Err(KernelError::InvalidArgument(
            "make_wire requires at least 2 points".into(),
        ));
    }

    let op = model.history.next_operation("make_wire");

    let closed = points.len() > 2
        && (*points.first().unwrap() - *points.last().unwrap()).length() < 1e-10;

    let point_count = if closed { points.len() - 1 } else { points.len() };

    let mut vertices = Vec::with_capacity(point_count);
    for (i, &pt) in points.iter().take(point_count).enumerate() {
        let tag = Tag::generated(EntityKind::Vertex, op, i as u32);
        vertices.push(model.add_vertex_tagged(pt, tag));
    }

    let edge_count = if closed { point_count } else { point_count - 1 };
    let mut edges = Vec::with_capacity(edge_count);

    for i in 0..edge_count {
        let j = (i + 1) % point_count;
        let tag = Tag::generated(EntityKind::Edge, op, i as u32);
        let (edge_h, _, _) = model.add_edge_tagged(vertices[i], vertices[j], tag);
        edges.push(edge_h);
    }

    Ok(WireResult { vertices, edges })
}

/// Creates a B-spline wire from control points.
///
/// Builds a [`NurbsCurve`] of the given degree, tessellates it into `segments`
/// line-segment edges, and inserts the resulting vertices and edges into the
/// model.
pub fn make_bspline_wire(
    model: &mut BRepModel,
    control_points: Vec<Point3>,
    degree: usize,
    segments: usize,
) -> KernelResult<BSplineWireResult> {
    if control_points.len() <= degree {
        return Err(KernelError::InvalidArgument(format!(
            "need at least {} control points for degree {}",
            degree + 1,
            degree
        )));
    }
    if segments < 1 {
        return Err(KernelError::InvalidArgument(
            "segments must be at least 1".into(),
        ));
    }

    let n = control_points.len();
    let weights = vec![1.0; n];

    // Build a clamped uniform knot vector.
    let knot_count = n + degree + 1;
    let internal = knot_count - 2 * (degree + 1);
    let mut knots = vec![0.0; degree + 1];
    for i in 1..=internal {
        knots.push(i as f64 / (internal + 1) as f64);
    }
    knots.extend(vec![1.0; degree + 1]);

    let curve = NurbsCurve::new(degree, control_points, weights, knots)?;
    let (t_start, t_end) = curve.domain();

    let op = model.history.next_operation("make_bspline_wire");

    let mut vertices = Vec::with_capacity(segments + 1);
    for i in 0..=segments {
        let t = t_start + (t_end - t_start) * i as f64 / segments as f64;
        let pt = curve.point_at(t);
        let tag = Tag::generated(EntityKind::Vertex, op, i as u32);
        vertices.push(model.add_vertex_tagged(pt, tag));
    }

    let mut edges = Vec::with_capacity(segments);
    for i in 0..segments {
        let tag = Tag::generated(EntityKind::Edge, op, i as u32);
        let (edge_h, _, _) = model.add_edge_tagged(vertices[i], vertices[i + 1], tag);
        edges.push(edge_h);
    }

    Ok(BSplineWireResult {
        vertices,
        edges,
        curve,
    })
}

/// Deep-copies a solid at the same position (identity transform).
pub fn clone_solid(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
) -> KernelResult<CloneResult> {
    let op = model.history.next_operation("clone");
    let result = copy_solid_transformed(model, solid, op, |pt| pt, false)?;
    Ok(CloneResult {
        solid: result.solid,
        faces: result.faces,
    })
}

/// Creates a 2D rectangular grid of copies.
///
/// `count_x` copies along `dir1` with `spacing_x`, `count_y` copies along
/// `dir2` with `spacing_y`. The original solid is included as (0,0).
/// Total instances = `count_x * count_y` (must be >= 2).
#[allow(clippy::too_many_arguments)]
pub fn rectangular_array(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    dir1: Vec3,
    spacing_x: f64,
    count_x: usize,
    dir2: Vec3,
    spacing_y: f64,
    count_y: usize,
) -> KernelResult<ArrayResult> {
    if count_x < 1 {
        return Err(KernelError::InvalidArgument(
            "count_x must be at least 1".into(),
        ));
    }
    if count_y < 1 {
        return Err(KernelError::InvalidArgument(
            "count_y must be at least 1".into(),
        ));
    }
    let total = count_x * count_y;
    if total < 2 {
        return Err(KernelError::InvalidArgument(
            "rectangular_array total (count_x * count_y) must be at least 2".into(),
        ));
    }

    let d1 = dir1.normalized().ok_or(KernelError::InvalidArgument(
        "dir1 must be non-zero".into(),
    ))?;
    let d2 = dir2.normalized().ok_or(KernelError::InvalidArgument(
        "dir2 must be non-zero".into(),
    ))?;

    let mut solids = vec![solid];
    let mut faces = Vec::new();

    for ix in 0..count_x {
        for iy in 0..count_y {
            if ix == 0 && iy == 0 {
                continue; // original
            }
            let offset = d1 * (spacing_x * ix as f64) + d2 * (spacing_y * iy as f64);
            let op = model.history.next_operation("rectangular_array");
            let result = copy_solid_transformed(
                model,
                solid,
                op,
                |pt| Point3::new(pt.x + offset.x, pt.y + offset.y, pt.z + offset.z),
                false,
            )?;
            solids.push(result.solid);
            faces.extend(result.faces);
        }
    }

    Ok(ArrayResult { solids, faces })
}

/// Copies a solid along a series of path points.
///
/// At each path point a translated copy of the solid is placed. The
/// translation is the vector from the first path point to each subsequent
/// point. Requires at least 2 path points.
pub fn path_array(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    path_points: &[Point3],
) -> KernelResult<ArrayResult> {
    if path_points.len() < 2 {
        return Err(KernelError::InvalidArgument(
            "path_array requires at least 2 path points".into(),
        ));
    }

    let origin = path_points[0];
    let mut solids = vec![solid];
    let mut faces = Vec::new();

    for &target in &path_points[1..] {
        let offset = target - origin;
        let op = model.history.next_operation("path_array");
        let result = copy_solid_transformed(
            model,
            solid,
            op,
            |pt| Point3::new(pt.x + offset.x, pt.y + offset.y, pt.z + offset.z),
            false,
        )?;
        solids.push(result.solid);
        faces.extend(result.faces);
    }

    Ok(ArrayResult { solids, faces })
}

/// Returns the distance and midpoint between two points, for placing a
/// dimension annotation in a drawing.
pub fn make_dimension_text(start: Point3, end: Point3) -> (f64, Point3) {
    let diff = end - start;
    let distance = diff.length();
    let midpoint = Point3::new(
        (start.x + end.x) * 0.5,
        (start.y + end.y) * 0.5,
        (start.z + end.z) * 0.5,
    );
    (distance, midpoint)
}

/// Rounds corners of a polyline wire with circular arc approximations.
///
/// At each interior vertex, the sharp corner is replaced by an arc of the
/// given `radius`. The arc is approximated with 8 linear segments. If the
/// radius is too large for a given corner (exceeds half the shorter adjacent
/// edge), that corner is left sharp.
///
/// Returns the new polyline with fillets inserted.
pub fn make_fillet_wire(points: &[Point3], radius: f64) -> Vec<Point3> {
    if points.len() < 3 || radius <= 0.0 {
        return points.to_vec();
    }

    const ARC_SEGMENTS: usize = 8;
    let mut result = Vec::new();
    result.push(points[0]);

    for i in 1..points.len() - 1 {
        let prev = points[i - 1];
        let curr = points[i];
        let next = points[i + 1];

        let v_in = prev - curr;
        let v_out = next - curr;
        let len_in = v_in.length();
        let len_out = v_out.length();

        if len_in < 1e-12 || len_out < 1e-12 {
            result.push(curr);
            continue;
        }

        let d_in = Vec3::new(v_in.x / len_in, v_in.y / len_in, v_in.z / len_in);
        let d_out = Vec3::new(v_out.x / len_out, v_out.y / len_out, v_out.z / len_out);

        let cos_half = {
            let bisector = d_in + d_out;
            let bl = bisector.length();
            if bl < 1e-12 {
                result.push(curr);
                continue;
            }
            let dot_val = d_in.dot(d_out);
            let half_angle = (1.0 - dot_val).max(0.0).sqrt() / std::f64::consts::SQRT_2;
            (1.0 - half_angle * half_angle).max(0.0).sqrt()
        };

        if cos_half < 1e-12 {
            result.push(curr);
            continue;
        }

        let sin_half = (1.0 - cos_half * cos_half).max(0.0).sqrt();
        let tangent_len = radius * cos_half / sin_half.max(1e-12);

        if tangent_len > len_in * 0.5 || tangent_len > len_out * 0.5 {
            result.push(curr);
            continue;
        }

        let p_start = Point3::new(
            curr.x + d_in.x * tangent_len,
            curr.y + d_in.y * tangent_len,
            curr.z + d_in.z * tangent_len,
        );
        let p_end = Point3::new(
            curr.x + d_out.x * tangent_len,
            curr.y + d_out.y * tangent_len,
            curr.z + d_out.z * tangent_len,
        );

        for j in 0..=ARC_SEGMENTS {
            let t = j as f64 / ARC_SEGMENTS as f64;
            let pt = Point3::new(
                p_start.x * (1.0 - t) + p_end.x * t,
                p_start.y * (1.0 - t) + p_end.y * t,
                p_start.z * (1.0 - t) + p_end.z * t,
            );
            result.push(pt);
        }
    }

    result.push(*points.last().unwrap());
    result
}

/// Creates copies of a solid rotated around an axis (polar/circular array).
///
/// The copies are evenly distributed around the full circle (360 degrees).
/// The original solid is included as the first element. `count` must be >= 2.
pub fn polar_array(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    axis: Point3,
    axis_dir: Vec3,
    count: usize,
) -> KernelResult<Vec<Handle<SolidData>>> {
    if count < 2 {
        return Err(KernelError::InvalidArgument(
            "polar_array count must be at least 2".into(),
        ));
    }
    let dir = axis_dir.normalized().ok_or(KernelError::InvalidArgument(
        "axis_dir must be non-zero".into(),
    ))?;

    let mut solids = vec![solid];
    let angle_step = std::f64::consts::TAU / count as f64;

    for i in 1..count {
        let angle = angle_step * i as f64;
        let cos_a = angle.cos();
        let sin_a = angle.sin();

        let op = model.history.next_operation("polar_array");
        let result = copy_solid_transformed(
            model,
            solid,
            op,
            |pt| {
                let v = pt - axis;
                let along = dir * v.dot(dir);
                let perp = v - along;
                let perp_len = perp.length();
                if perp_len < 1e-15 {
                    return pt;
                }
                let u = Vec3::new(perp.x / perp_len, perp.y / perp_len, perp.z / perp_len);
                let w = dir.cross(u);
                let rotated = u * (perp_len * cos_a) + w * (perp_len * sin_a) + along;
                Point3::new(axis.x + rotated.x, axis.y + rotated.y, axis.z + rotated.z)
            },
            false,
        )?;
        solids.push(result.solid);
    }

    Ok(solids)
}

/// Creates copies of a solid at specified positions.
///
/// Each copy is translated by the offset from the first position to the
/// target position. The original solid is included as the first element.
/// Requires at least 2 positions.
pub fn point_array(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    positions: &[Point3],
) -> KernelResult<Vec<Handle<SolidData>>> {
    if positions.len() < 2 {
        return Err(KernelError::InvalidArgument(
            "point_array requires at least 2 positions".into(),
        ));
    }

    let origin = positions[0];
    let mut solids = vec![solid];

    for &target in &positions[1..] {
        let offset = target - origin;
        let op = model.history.next_operation("point_array");
        let result = copy_solid_transformed(
            model,
            solid,
            op,
            |pt| Point3::new(pt.x + offset.x, pt.y + offset.y, pt.z + offset.z),
            false,
        )?;
        solids.push(result.solid);
    }

    Ok(solids)
}

/// Returns a polyline approximation of a circle.
///
/// `segments` must be >= 3.
pub fn make_circle_wire(
    center: Point3,
    normal: Vec3,
    radius: f64,
    segments: usize,
) -> KernelResult<Vec<Point3>> {
    if radius <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "radius must be positive".into(),
        ));
    }
    if segments < 3 {
        return Err(KernelError::InvalidArgument(
            "segments must be at least 3".into(),
        ));
    }
    let n = normal.normalized().ok_or(KernelError::InvalidArgument(
        "normal must be non-zero".into(),
    ))?;

    let arbitrary = if n.x.abs() < 0.9 {
        Vec3::new(1.0, 0.0, 0.0)
    } else {
        Vec3::new(0.0, 1.0, 0.0)
    };
    let u = n.cross(arbitrary);
    let u = u.normalized().unwrap();
    let v = n.cross(u);

    let mut points = Vec::with_capacity(segments + 1);
    for i in 0..=segments {
        let angle = std::f64::consts::TAU * i as f64 / segments as f64;
        let pt = Point3::new(
            center.x + radius * (angle.cos() * u.x + angle.sin() * v.x),
            center.y + radius * (angle.cos() * u.y + angle.sin() * v.y),
            center.z + radius * (angle.cos() * u.z + angle.sin() * v.z),
        );
        points.push(pt);
    }

    Ok(points)
}

/// Returns a polyline approximation of a circular arc from `start` to `end`
/// around `center`.
///
/// The arc goes counter-clockwise when viewed from the direction of
/// `(start - center) × (end - center)`. `segments` must be >= 1.
pub fn make_arc_wire(
    center: Point3,
    start: Point3,
    end: Point3,
    segments: usize,
) -> KernelResult<Vec<Point3>> {
    if segments < 1 {
        return Err(KernelError::InvalidArgument(
            "segments must be at least 1".into(),
        ));
    }
    let v_start = start - center;
    let v_end = end - center;
    let radius = v_start.length();
    if radius < 1e-15 {
        return Err(KernelError::InvalidArgument(
            "start must not coincide with center".into(),
        ));
    }

    let normal = v_start.cross(v_end);
    let n_len = normal.length();
    let n = if n_len < 1e-15 {
        let arb = if v_start.x.abs() < 0.9 {
            Vec3::new(1.0, 0.0, 0.0)
        } else {
            Vec3::new(0.0, 1.0, 0.0)
        };
        let tmp = v_start.cross(arb);
        tmp.normalized().ok_or(KernelError::InvalidArgument(
            "degenerate arc configuration".into(),
        ))?
    } else {
        Vec3::new(normal.x / n_len, normal.y / n_len, normal.z / n_len)
    };

    let u = Vec3::new(v_start.x / radius, v_start.y / radius, v_start.z / radius);
    let v = n.cross(u);

    let dot_val = u.dot(v_end) / v_end.length().max(1e-15);
    let cross_val = v.dot(v_end) / v_end.length().max(1e-15);
    let mut sweep = cross_val.atan2(dot_val);
    if sweep <= 0.0 {
        sweep += std::f64::consts::TAU;
    }

    let mut points = Vec::with_capacity(segments + 1);
    for i in 0..=segments {
        let t = i as f64 / segments as f64;
        let angle = sweep * t;
        let pt = Point3::new(
            center.x + radius * (angle.cos() * u.x + angle.sin() * v.x),
            center.y + radius * (angle.cos() * u.y + angle.sin() * v.y),
            center.z + radius * (angle.cos() * u.z + angle.sin() * v.z),
        );
        points.push(pt);
    }

    Ok(points)
}

// ─── Phase V9: Draft Workbench Expansion ───

/// Returns a polyline approximation of an ellipse.
///
/// `rx` and `ry` are the semi-axes in the local U and V directions.
/// `segments` must be >= 3.
pub fn make_ellipse_wire(
    center: Point3,
    normal: Vec3,
    rx: f64,
    ry: f64,
    segments: usize,
) -> KernelResult<Vec<Point3>> {
    if rx <= 0.0 || ry <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "semi-axes must be positive".into(),
        ));
    }
    if segments < 3 {
        return Err(KernelError::InvalidArgument(
            "segments must be at least 3".into(),
        ));
    }
    let n = normal.normalized().ok_or(KernelError::InvalidArgument(
        "normal must be non-zero".into(),
    ))?;
    let arbitrary = if n.x.abs() < 0.9 {
        Vec3::new(1.0, 0.0, 0.0)
    } else {
        Vec3::new(0.0, 1.0, 0.0)
    };
    let u = n.cross(arbitrary).normalized().unwrap();
    let v = n.cross(u);

    let mut points = Vec::with_capacity(segments + 1);
    for i in 0..=segments {
        let angle = std::f64::consts::TAU * i as f64 / segments as f64;
        let pt = Point3::new(
            center.x + rx * angle.cos() * u.x + ry * angle.sin() * v.x,
            center.y + rx * angle.cos() * u.y + ry * angle.sin() * v.y,
            center.z + rx * angle.cos() * u.z + ry * angle.sin() * v.z,
        );
        points.push(pt);
    }
    Ok(points)
}

/// Returns a rectangle as a closed polyline (5 points, first == last).
pub fn make_rectangle_wire(
    origin: Point3,
    width: f64,
    height: f64,
    normal: Vec3,
) -> KernelResult<Vec<Point3>> {
    if width <= 0.0 || height <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "width and height must be positive".into(),
        ));
    }
    let n = normal.normalized().ok_or(KernelError::InvalidArgument(
        "normal must be non-zero".into(),
    ))?;
    let arbitrary = if n.x.abs() < 0.9 {
        Vec3::new(1.0, 0.0, 0.0)
    } else {
        Vec3::new(0.0, 1.0, 0.0)
    };
    let u = n.cross(arbitrary).normalized().unwrap();
    let v = n.cross(u);

    Ok(vec![
        origin,
        Point3::new(
            origin.x + width * u.x,
            origin.y + width * u.y,
            origin.z + width * u.z,
        ),
        Point3::new(
            origin.x + width * u.x + height * v.x,
            origin.y + width * u.y + height * v.y,
            origin.z + width * u.z + height * v.z,
        ),
        Point3::new(
            origin.x + height * v.x,
            origin.y + height * v.y,
            origin.z + height * v.z,
        ),
        origin,
    ])
}

/// Returns a regular polygon as a closed polyline.
///
/// `sides` must be >= 3.
pub fn make_polygon_wire(
    center: Point3,
    normal: Vec3,
    radius: f64,
    sides: usize,
) -> KernelResult<Vec<Point3>> {
    if radius <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "radius must be positive".into(),
        ));
    }
    if sides < 3 {
        return Err(KernelError::InvalidArgument(
            "sides must be at least 3".into(),
        ));
    }
    make_circle_wire(center, normal, radius, sides)
}

/// Creates a single point entity in the model.
pub fn make_point(model: &mut BRepModel, pt: Point3) -> Handle<VertexData> {
    let op = model.history.next_operation("make_point");
    let tag = Tag::generated(EntityKind::Vertex, op, 0);
    model.add_vertex_tagged(pt, tag)
}

/// Returns a cubic Bézier curve as a polyline.
///
/// Uses de Casteljau subdivision with `segments` linear segments.
pub fn make_bezier_wire(
    p0: Point3,
    p1: Point3,
    p2: Point3,
    p3: Point3,
    segments: usize,
) -> KernelResult<Vec<Point3>> {
    if segments < 1 {
        return Err(KernelError::InvalidArgument(
            "segments must be at least 1".into(),
        ));
    }
    let mut points = Vec::with_capacity(segments + 1);
    for i in 0..=segments {
        let t = i as f64 / segments as f64;
        let u = 1.0 - t;
        let pt = Point3::new(
            u * u * u * p0.x + 3.0 * u * u * t * p1.x + 3.0 * u * t * t * p2.x + t * t * t * p3.x,
            u * u * u * p0.y + 3.0 * u * u * t * p1.y + 3.0 * u * t * t * p2.y + t * t * t * p3.y,
            u * u * u * p0.z + 3.0 * u * u * t * p1.z + 3.0 * u * t * t * p2.z + t * t * t * p3.z,
        );
        points.push(pt);
    }
    Ok(points)
}

/// Translate a solid by a displacement vector.
pub fn move_solid(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    displacement: Vec3,
) -> KernelResult<Handle<SolidData>> {
    let op = model.history.next_operation("move");
    let result = copy_solid_transformed(
        model,
        solid,
        op,
        |pt| Point3::new(pt.x + displacement.x, pt.y + displacement.y, pt.z + displacement.z),
        false,
    )?;
    Ok(result.solid)
}

/// Rotate a solid around an axis by a given angle (radians).
pub fn rotate_solid(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    axis_point: Point3,
    axis_dir: Vec3,
    angle: f64,
) -> KernelResult<Handle<SolidData>> {
    let dir = axis_dir.normalized().ok_or(KernelError::InvalidArgument(
        "axis_dir must be non-zero".into(),
    ))?;
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    let op = model.history.next_operation("rotate");
    let result = copy_solid_transformed(
        model,
        solid,
        op,
        |pt| {
            let v = pt - axis_point;
            let along = dir * v.dot(dir);
            let perp = v - along;
            let perp_len = perp.length();
            if perp_len < 1e-15 {
                return pt;
            }
            let u = Vec3::new(perp.x / perp_len, perp.y / perp_len, perp.z / perp_len);
            let w = dir.cross(u);
            let rotated = u * (perp_len * cos_a) + w * (perp_len * sin_a) + along;
            Point3::new(
                axis_point.x + rotated.x,
                axis_point.y + rotated.y,
                axis_point.z + rotated.z,
            )
        },
        false,
    )?;
    Ok(result.solid)
}

/// Scale a solid from a reference point.
pub fn scale_solid_draft(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    center: Point3,
    factor: f64,
) -> KernelResult<Handle<SolidData>> {
    if factor.abs() < 1e-15 {
        return Err(KernelError::InvalidArgument(
            "scale factor must be non-zero".into(),
        ));
    }
    let op = model.history.next_operation("scale");
    let result = copy_solid_transformed(
        model,
        solid,
        op,
        |pt| {
            Point3::new(
                center.x + (pt.x - center.x) * factor,
                center.y + (pt.y - center.y) * factor,
                center.z + (pt.z - center.z) * factor,
            )
        },
        false,
    )?;
    Ok(result.solid)
}

/// Mirror a solid across a plane defined by point and normal.
pub fn mirror_solid_draft(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    plane_point: Point3,
    plane_normal: Vec3,
) -> KernelResult<Handle<SolidData>> {
    let n = plane_normal.normalized().ok_or(KernelError::InvalidArgument(
        "plane_normal must be non-zero".into(),
    ))?;
    let op = model.history.next_operation("mirror");
    let result = copy_solid_transformed(
        model,
        solid,
        op,
        |pt| {
            let v = pt - plane_point;
            let d = v.dot(n);
            Point3::new(pt.x - 2.0 * d * n.x, pt.y - 2.0 * d * n.y, pt.z - 2.0 * d * n.z)
        },
        true, // mirror flips winding
    )?;
    Ok(result.solid)
}

/// Offset a polyline wire by a distance in the plane defined by the normal.
///
/// Each segment is offset perpendicular to its direction by `distance`.
/// Positive distance offsets to the left (when viewed from normal direction).
pub fn offset_wire(points: &[Point3], distance: f64, normal: Vec3) -> KernelResult<Vec<Point3>> {
    if points.len() < 2 {
        return Err(KernelError::InvalidArgument(
            "offset_wire requires at least 2 points".into(),
        ));
    }
    let n = normal.normalized().ok_or(KernelError::InvalidArgument(
        "normal must be non-zero".into(),
    ))?;

    let mut result = Vec::with_capacity(points.len());
    for i in 0..points.len() {
        let prev = if i > 0 { i - 1 } else { 0 };
        let next = if i < points.len() - 1 { i + 1 } else { i };
        let dir = points[next] - points[prev];
        let dir_len = dir.length();
        if dir_len < 1e-15 {
            result.push(points[i]);
            continue;
        }
        let tangent = Vec3::new(dir.x / dir_len, dir.y / dir_len, dir.z / dir_len);
        let offset_dir = n.cross(tangent);
        let offset_len = offset_dir.length();
        if offset_len < 1e-15 {
            result.push(points[i]);
            continue;
        }
        let offset_unit = Vec3::new(
            offset_dir.x / offset_len,
            offset_dir.y / offset_len,
            offset_dir.z / offset_len,
        );
        result.push(Point3::new(
            points[i].x + offset_unit.x * distance,
            points[i].y + offset_unit.y * distance,
            points[i].z + offset_unit.z * distance,
        ));
    }
    Ok(result)
}

/// Join multiple polyline wires into a single continuous polyline.
///
/// Wires are appended in order, reversing direction if needed to maintain
/// continuity (within tolerance).
pub fn join_wires(wires: &[Vec<Point3>], tolerance: f64) -> Vec<Point3> {
    if wires.is_empty() {
        return Vec::new();
    }
    let mut result = wires[0].clone();
    for wire in &wires[1..] {
        if wire.is_empty() {
            continue;
        }
        let end = *result.last().unwrap();
        let d_start = end.distance_to(wire[0]);
        let d_end = end.distance_to(*wire.last().unwrap());
        if d_end < d_start && d_end < tolerance {
            // Reverse this wire
            let mut reversed = wire.clone();
            reversed.reverse();
            if end.distance_to(reversed[0]) < tolerance {
                result.extend_from_slice(&reversed[1..]);
            } else {
                result.extend(reversed);
            }
        } else if d_start < tolerance {
            result.extend_from_slice(&wire[1..]);
        } else {
            result.extend(wire.iter());
        }
    }
    result
}

/// Split a wire at a given index, returning two sub-wires.
///
/// The split point is included in both resulting wires.
pub fn split_wire(points: &[Point3], split_index: usize) -> KernelResult<(Vec<Point3>, Vec<Point3>)> {
    if split_index == 0 || split_index >= points.len() {
        return Err(KernelError::InvalidArgument(
            "split_index must be between 1 and points.len()-1".into(),
        ));
    }
    let first = points[..=split_index].to_vec();
    let second = points[split_index..].to_vec();
    Ok((first, second))
}

/// Close an open wire by connecting the last point to the first.
///
/// If already closed (within tolerance), returns a copy unchanged.
pub fn upgrade_wire(points: &[Point3]) -> Vec<Point3> {
    if points.len() < 2 {
        return points.to_vec();
    }
    let mut result = points.to_vec();
    let first = result[0];
    let last = *result.last().unwrap();
    if first.distance_to(last) > 1e-10 {
        result.push(first);
    }
    result
}

/// Extract all vertex positions from a solid (downgrade to point cloud).
pub fn downgrade_solid(model: &BRepModel, solid: Handle<SolidData>) -> KernelResult<Vec<Point3>> {
    let solid_data = model
        .solids
        .get(solid)
        .ok_or(KernelError::InvalidArgument("invalid solid handle".into()))?;
    let mut points = Vec::new();
    for &shell_h in &solid_data.shells {
        let shell = model
            .shells
            .get(shell_h)
            .ok_or(KernelError::InvalidArgument("invalid shell".into()))?;
        for &face_h in &shell.faces {
            let face = model
                .faces
                .get(face_h)
                .ok_or(KernelError::InvalidArgument("invalid face".into()))?;
            // Traverse outer loop
            let loop_data = model
                .loops
                .get(face.outer_loop)
                .ok_or(KernelError::InvalidArgument("invalid loop".into()))?;
            let hes = model.loop_half_edges(loop_data.half_edge);
            for &he_h in &hes {
                if let Some(he) = model.half_edges.get(he_h) {
                    if let Some(v) = model.vertices.get(he.origin) {
                        points.push(v.point);
                    }
                }
            }
        }
    }
    points.dedup_by(|a, b| a.distance_to(*b) < 1e-10);
    Ok(points)
}

/// Convert a polyline wire to a B-spline curve approximation.
pub fn wire_to_bspline(points: &[Point3], degree: usize) -> KernelResult<NurbsCurve> {
    if points.len() <= degree {
        return Err(KernelError::InvalidArgument(format!(
            "need at least {} points for degree {}",
            degree + 1,
            degree
        )));
    }
    let n = points.len();
    let weights = vec![1.0; n];
    let knot_count = n + degree + 1;
    let internal = knot_count - 2 * (degree + 1);
    let mut knots = vec![0.0; degree + 1];
    for i in 1..=internal {
        knots.push(i as f64 / (internal + 1) as f64);
    }
    knots.extend(vec![1.0; degree + 1]);
    NurbsCurve::new(degree, points.to_vec(), weights, knots)
}

/// Convert a B-spline curve to a polyline by tessellation.
pub fn bspline_to_wire(curve: &NurbsCurve, segments: usize) -> KernelResult<Vec<Point3>> {
    if segments < 1 {
        return Err(KernelError::InvalidArgument(
            "segments must be at least 1".into(),
        ));
    }
    let (t_start, t_end) = curve.domain();
    let mut points = Vec::with_capacity(segments + 1);
    for i in 0..=segments {
        let t = t_start + (t_end - t_start) * i as f64 / segments as f64;
        points.push(curve.point_at(t));
    }
    Ok(points)
}

/// Draft label: text annotation at a position with optional leader line.
#[derive(Debug, Clone)]
pub struct DraftLabel {
    pub text: String,
    pub position: Point3,
    pub leader_target: Option<Point3>,
}

/// Create a draft label annotation.
pub fn make_label(text: &str, position: Point3, leader_target: Option<Point3>) -> DraftLabel {
    DraftLabel {
        text: text.to_string(),
        position,
        leader_target,
    }
}

/// Draft dimension: distance measurement between two points.
#[derive(Debug, Clone)]
pub struct DraftDimension {
    pub start: Point3,
    pub end: Point3,
    pub distance: f64,
    pub midpoint: Point3,
    pub offset_point: Point3,
}

/// Create a draft dimension annotation.
///
/// `offset` controls how far the dimension line is from the measured points.
pub fn make_draft_dimension(start: Point3, end: Point3, offset: f64) -> DraftDimension {
    let diff = end - start;
    let distance = diff.length();
    let midpoint = Point3::new(
        (start.x + end.x) * 0.5,
        (start.y + end.y) * 0.5,
        (start.z + end.z) * 0.5,
    );
    let normal = if distance > 1e-15 {
        let dir = Vec3::new(diff.x / distance, diff.y / distance, diff.z / distance);
        // Perpendicular in XY plane
        Vec3::new(-dir.y, dir.x, 0.0)
    } else {
        Vec3::Y
    };
    let offset_point = Point3::new(
        midpoint.x + normal.x * offset,
        midpoint.y + normal.y * offset,
        midpoint.z + normal.z * offset,
    );
    DraftDimension {
        start,
        end,
        distance,
        midpoint,
        offset_point,
    }
}

/// Snap result: nearest geometry point and its distance.
#[derive(Debug, Clone)]
pub struct SnapResult {
    pub point: Point3,
    pub distance: f64,
}

/// Find the nearest endpoint in a wire to the query point.
pub fn snap_to_endpoint(wire: &[Point3], query: Point3) -> Option<SnapResult> {
    wire.iter()
        .map(|&p| SnapResult {
            point: p,
            distance: p.distance_to(query),
        })
        .min_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap())
}

/// Find the nearest edge midpoint in a wire to the query point.
pub fn snap_to_midpoint(wire: &[Point3], query: Point3) -> Option<SnapResult> {
    if wire.len() < 2 {
        return None;
    }
    (0..wire.len() - 1)
        .map(|i| {
            let mid = wire[i].midpoint(wire[i + 1]);
            SnapResult {
                point: mid,
                distance: mid.distance_to(query),
            }
        })
        .min_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap())
}

/// Find the nearest point on any wire edge to the query point.
pub fn snap_to_nearest(wire: &[Point3], query: Point3) -> Option<SnapResult> {
    if wire.len() < 2 {
        return snap_to_endpoint(wire, query);
    }
    (0..wire.len() - 1)
        .map(|i| {
            let a = wire[i];
            let b = wire[i + 1];
            let ab = b - a;
            let aq = query - a;
            let ab_len_sq = ab.length_squared();
            let t = if ab_len_sq > 1e-30 {
                aq.dot(ab) / ab_len_sq
            } else {
                0.0
            }
            .clamp(0.0, 1.0);
            let closest = Point3::new(a.x + t * ab.x, a.y + t * ab.y, a.z + t * ab.z);
            SnapResult {
                point: closest,
                distance: closest.distance_to(query),
            }
        })
        .min_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap())
}

/// Compute wire length (sum of segment lengths).
pub fn wire_length(points: &[Point3]) -> f64 {
    if points.len() < 2 {
        return 0.0;
    }
    (0..points.len() - 1)
        .map(|i| points[i].distance_to(points[i + 1]))
        .sum()
}

/// Compute the area enclosed by a closed planar wire (polygon area via shoelace).
///
/// Assumes the wire lies approximately in a plane. Uses the 3D cross-product
/// method to compute the signed area relative to the given normal.
pub fn wire_area(points: &[Point3], normal: Vec3) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let n = normal.normalized().unwrap_or(Vec3::Z);
    let mut sum = Vec3::ZERO;
    let o = points[0];
    for i in 1..points.len() - 1 {
        let a = points[i] - o;
        let b = points[i + 1] - o;
        sum += a.cross(b);
    }
    (sum.dot(n) * 0.5).abs()
}

/// Returns an arc from 3 points as a polyline.
///
/// Computes the circumscribed circle center from `p1`, `p2`, `p3`, then
/// generates the arc from `p1` through `p2` to `p3`.
pub fn make_arc_3pt_wire(
    p1: Point3,
    p2: Point3,
    p3: Point3,
    segments: usize,
) -> KernelResult<Vec<Point3>> {
    if segments < 2 {
        return Err(KernelError::InvalidArgument(
            "segments must be at least 2".into(),
        ));
    }
    let v12 = p2 - p1;
    let v13 = p3 - p1;
    let normal = v12.cross(v13);
    let n_len = normal.length();
    if n_len < 1e-12 {
        return Err(KernelError::InvalidArgument(
            "three points are collinear".into(),
        ));
    }

    // Circumcenter via perpendicular bisectors
    let mid12 = p1.midpoint(p2);
    let mid13 = p1.midpoint(p3);
    let n = Vec3::new(normal.x / n_len, normal.y / n_len, normal.z / n_len);
    let d12 = v12.cross(n);
    let d13 = v13.cross(n);

    // Solve: mid12 + t * d12 = mid13 + s * d13 (projected to plane)
    let diff = Point3::new(mid13.x - mid12.x, mid13.y - mid12.y, mid13.z - mid12.z);
    let denom = d12.x * d13.y - d12.y * d13.x;
    let t = if denom.abs() > 1e-15 {
        (Vec3::new(diff.x, diff.y, diff.z).x * d13.y
            - Vec3::new(diff.x, diff.y, diff.z).y * d13.x)
            / denom
    } else {
        let denom2 = d12.y * d13.z - d12.z * d13.y;
        if denom2.abs() > 1e-15 {
            (Vec3::new(diff.x, diff.y, diff.z).y * d13.z
                - Vec3::new(diff.x, diff.y, diff.z).z * d13.y)
                / denom2
        } else {
            let denom3 = d12.x * d13.z - d12.z * d13.x;
            if denom3.abs() > 1e-15 {
                (Vec3::new(diff.x, diff.y, diff.z).x * d13.z
                    - Vec3::new(diff.x, diff.y, diff.z).z * d13.x)
                    / denom3
            } else {
                return Err(KernelError::InvalidArgument(
                    "cannot compute circumcenter".into(),
                ));
            }
        }
    };

    let center = Point3::new(
        mid12.x + t * d12.x,
        mid12.y + t * d12.y,
        mid12.z + t * d12.z,
    );
    let radius = center.distance_to(p1);

    // Compute angles
    let v1 = p1 - center;
    let v3 = p3 - center;
    let u = Vec3::new(v1.x / radius, v1.y / radius, v1.z / radius);
    let w = n.cross(u);

    let angle_end = {
        let d = v3.dot(Vec3::new(u.x * radius, u.y * radius, u.z * radius)) / (radius * radius);
        let c = v3.dot(Vec3::new(w.x * radius, w.y * radius, w.z * radius)) / (radius * radius);
        let mut a = c.atan2(d);
        // Make sure p2 is between p1 and p3
        let v2 = p2 - center;
        let d2 = v2.dot(Vec3::new(u.x * radius, u.y * radius, u.z * radius)) / (radius * radius);
        let c2 = v2.dot(Vec3::new(w.x * radius, w.y * radius, w.z * radius)) / (radius * radius);
        let a2 = c2.atan2(d2);
        if a > 0.0 && (a2 < 0.0 || a2 > a) {
            a -= std::f64::consts::TAU;
        } else if a < 0.0 && (a2 > 0.0 || a2 < a) {
            a += std::f64::consts::TAU;
        }
        a
    };

    let mut points = Vec::with_capacity(segments + 1);
    for i in 0..=segments {
        let t_param = i as f64 / segments as f64;
        let angle = angle_end * t_param;
        let pt = Point3::new(
            center.x + radius * (angle.cos() * u.x + angle.sin() * w.x),
            center.y + radius * (angle.cos() * u.y + angle.sin() * w.y),
            center.z + radius * (angle.cos() * u.z + angle.sin() * w.z),
        );
        points.push(pt);
    }
    Ok(points)
}

/// Chamfer wire corners with straight cuts.
///
/// Similar to `make_fillet_wire` but replaces corners with straight line segments
/// instead of arcs. `size` controls how far the chamfer extends along each edge.
pub fn make_chamfer_wire(points: &[Point3], size: f64) -> Vec<Point3> {
    if points.len() < 3 || size <= 0.0 {
        return points.to_vec();
    }

    let mut result = Vec::new();
    result.push(points[0]);

    for i in 1..points.len() - 1 {
        let prev = points[i - 1];
        let curr = points[i];
        let next = points[i + 1];

        let v_in = prev - curr;
        let v_out = next - curr;
        let len_in = v_in.length();
        let len_out = v_out.length();

        if len_in < 1e-12 || len_out < 1e-12 || size > len_in * 0.5 || size > len_out * 0.5 {
            result.push(curr);
            continue;
        }

        let d_in = Vec3::new(v_in.x / len_in, v_in.y / len_in, v_in.z / len_in);
        let d_out = Vec3::new(v_out.x / len_out, v_out.y / len_out, v_out.z / len_out);

        result.push(Point3::new(
            curr.x + d_in.x * size,
            curr.y + d_in.y * size,
            curr.z + d_in.z * size,
        ));
        result.push(Point3::new(
            curr.x + d_out.x * size,
            curr.y + d_out.y * size,
            curr.z + d_out.z * size,
        ));
    }

    result.push(*points.last().unwrap());
    result
}

/// Stretch wire points within a region along a direction.
///
/// Points within `radius` of `center` are displaced by `displacement`.
/// Points outside are unchanged.
pub fn stretch_wire(
    points: &[Point3],
    center: Point3,
    radius: f64,
    displacement: Vec3,
) -> Vec<Point3> {
    points
        .iter()
        .map(|&p| {
            let d = p.distance_to(center);
            if d <= radius {
                let factor = 1.0 - d / radius.max(1e-15);
                Point3::new(
                    p.x + displacement.x * factor,
                    p.y + displacement.y * factor,
                    p.z + displacement.z * factor,
                )
            } else {
                p
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::make_box;
    use cadkernel_math::Point3;
    use cadkernel_topology::BRepModel;

    #[test]
    fn test_make_wire_basic() {
        let mut model = BRepModel::new();
        let pts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
        ];
        let result = make_wire(&mut model, &pts).unwrap();
        assert_eq!(result.vertices.len(), 3);
        assert_eq!(result.edges.len(), 2);
    }

    #[test]
    fn test_make_wire_closed() {
        let mut model = BRepModel::new();
        let pts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 0.0), // closes the wire
        ];
        let result = make_wire(&mut model, &pts).unwrap();
        assert_eq!(result.vertices.len(), 3);
        assert_eq!(result.edges.len(), 3); // 3 edges forming a closed triangle
    }

    #[test]
    fn test_make_wire_too_few_points() {
        let mut model = BRepModel::new();
        let result = make_wire(&mut model, &[Point3::new(0.0, 0.0, 0.0)]);
        assert!(result.is_err());
    }

    #[test]
    fn test_make_bspline_wire() {
        let mut model = BRepModel::new();
        let cps = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 2.0, 0.0),
            Point3::new(3.0, 1.0, 0.0),
            Point3::new(4.0, 0.0, 0.0),
        ];
        let result = make_bspline_wire(&mut model, cps, 3, 10).unwrap();
        assert_eq!(result.vertices.len(), 11);
        assert_eq!(result.edges.len(), 10);
        // First vertex should be near the first control point
        let v0 = model.vertices.get(result.vertices[0]).unwrap();
        assert!((v0.point.x).abs() < 1e-10);
    }

    #[test]
    fn test_clone_solid() {
        let mut model = BRepModel::new();
        let bx = make_box(&mut model, Point3::new(0.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap();
        let result = clone_solid(&mut model, bx.solid).unwrap();
        assert_eq!(result.faces.len(), 6);
        assert_ne!(result.solid, bx.solid);
    }

    #[test]
    fn test_rectangular_array() {
        let mut model = BRepModel::new();
        let bx = make_box(&mut model, Point3::new(0.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap();
        let result = rectangular_array(
            &mut model,
            bx.solid,
            Vec3::new(1.0, 0.0, 0.0),
            2.0,
            3,
            Vec3::new(0.0, 1.0, 0.0),
            2.0,
            2,
        )
        .unwrap();
        // 3x2 = 6 total solids (1 original + 5 copies)
        assert_eq!(result.solids.len(), 6);
    }

    #[test]
    fn test_rectangular_array_invalid() {
        let mut model = BRepModel::new();
        let bx = make_box(&mut model, Point3::new(0.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap();
        // 1x1 = 1 total, must be >= 2
        let result = rectangular_array(
            &mut model,
            bx.solid,
            Vec3::new(1.0, 0.0, 0.0),
            2.0,
            1,
            Vec3::new(0.0, 1.0, 0.0),
            2.0,
            1,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_path_array() {
        let mut model = BRepModel::new();
        let bx = make_box(&mut model, Point3::new(0.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap();
        let path = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(3.0, 0.0, 0.0),
            Point3::new(6.0, 0.0, 0.0),
        ];
        let result = path_array(&mut model, bx.solid, &path).unwrap();
        // 3 path points = 1 original + 2 copies
        assert_eq!(result.solids.len(), 3);
    }

    #[test]
    fn test_path_array_too_few_points() {
        let mut model = BRepModel::new();
        let bx = make_box(&mut model, Point3::new(0.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap();
        let result = path_array(&mut model, bx.solid, &[Point3::new(0.0, 0.0, 0.0)]);
        assert!(result.is_err());
    }

    #[test]
    fn test_make_dimension_text() {
        let (dist, mid) = make_dimension_text(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(3.0, 4.0, 0.0),
        );
        assert!((dist - 5.0).abs() < 1e-10);
        assert!((mid.x - 1.5).abs() < 1e-10);
        assert!((mid.y - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_make_fillet_wire() {
        let pts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(5.0, 0.0, 0.0),
            Point3::new(5.0, 5.0, 0.0),
        ];
        let filleted = make_fillet_wire(&pts, 1.0);
        assert!(filleted.len() > 3);
        assert!((filleted[0].x).abs() < 1e-10);
        let last = filleted.last().unwrap();
        assert!((last.x - 5.0).abs() < 1e-10);
        assert!((last.y - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_polar_array() {
        let mut model = BRepModel::new();
        let bx = make_box(&mut model, Point3::new(2.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap();
        let solids = polar_array(
            &mut model,
            bx.solid,
            Point3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            4,
        )
        .unwrap();
        assert_eq!(solids.len(), 4);
    }

    #[test]
    fn test_point_array() {
        let mut model = BRepModel::new();
        let bx = make_box(&mut model, Point3::new(0.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap();
        let positions = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(5.0, 0.0, 0.0),
            Point3::new(0.0, 5.0, 0.0),
        ];
        let solids = point_array(&mut model, bx.solid, &positions).unwrap();
        assert_eq!(solids.len(), 3);
    }

    #[test]
    fn test_make_circle_wire() {
        let pts = make_circle_wire(
            Point3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            2.0,
            32,
        )
        .unwrap();
        assert_eq!(pts.len(), 33);
        // First and last point should coincide (closed circle)
        let diff = pts[0] - *pts.last().unwrap();
        assert!(diff.length() < 1e-10);
        // All points should be at distance 2.0 from center
        for pt in &pts {
            let r = (pt.x * pt.x + pt.y * pt.y).sqrt();
            assert!((r - 2.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_make_ellipse_wire() {
        let pts = make_ellipse_wire(
            Point3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            3.0,
            2.0,
            32,
        )
        .unwrap();
        assert_eq!(pts.len(), 33);
        // Check first and last coincide (closed)
        assert!(pts[0].distance_to(*pts.last().unwrap()) < 1e-10);
    }

    #[test]
    fn test_make_rectangle_wire() {
        let pts = make_rectangle_wire(
            Point3::new(0.0, 0.0, 0.0),
            2.0,
            3.0,
            Vec3::Z,
        )
        .unwrap();
        assert_eq!(pts.len(), 5);
        assert!(pts[0].distance_to(pts[4]) < 1e-10);
    }

    #[test]
    fn test_make_polygon_wire() {
        let pts = make_polygon_wire(Point3::ORIGIN, Vec3::Z, 1.0, 6).unwrap();
        assert_eq!(pts.len(), 7); // hexagon + closing point
        assert!(make_polygon_wire(Point3::ORIGIN, Vec3::Z, 1.0, 2).is_err());
    }

    #[test]
    fn test_make_point() {
        let mut model = BRepModel::new();
        let vh = make_point(&mut model, Point3::new(1.0, 2.0, 3.0));
        let v = model.vertices.get(vh).unwrap();
        assert!((v.point.x - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_make_bezier_wire() {
        let pts = make_bezier_wire(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 2.0, 0.0),
            Point3::new(3.0, 2.0, 0.0),
            Point3::new(4.0, 0.0, 0.0),
            20,
        )
        .unwrap();
        assert_eq!(pts.len(), 21);
        assert!((pts[0].x - 0.0).abs() < 1e-12);
        assert!((pts[20].x - 4.0).abs() < 1e-12);
    }

    #[test]
    fn test_move_solid() {
        let mut model = BRepModel::new();
        let bx = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let moved = move_solid(&mut model, bx.solid, Vec3::new(5.0, 0.0, 0.0)).unwrap();
        assert_ne!(moved, bx.solid);
    }

    #[test]
    fn test_rotate_solid() {
        let mut model = BRepModel::new();
        let bx = make_box(&mut model, Point3::new(2.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap();
        let rotated = rotate_solid(
            &mut model,
            bx.solid,
            Point3::ORIGIN,
            Vec3::Z,
            std::f64::consts::FRAC_PI_2,
        )
        .unwrap();
        assert_ne!(rotated, bx.solid);
    }

    #[test]
    fn test_scale_solid_draft() {
        let mut model = BRepModel::new();
        let bx = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let scaled = scale_solid_draft(&mut model, bx.solid, Point3::ORIGIN, 2.0).unwrap();
        assert_ne!(scaled, bx.solid);
        assert!(scale_solid_draft(&mut model, bx.solid, Point3::ORIGIN, 0.0).is_err());
    }

    #[test]
    fn test_mirror_solid_draft() {
        let mut model = BRepModel::new();
        let bx = make_box(&mut model, Point3::new(1.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap();
        let mirrored = mirror_solid_draft(
            &mut model,
            bx.solid,
            Point3::ORIGIN,
            Vec3::X,
        )
        .unwrap();
        assert_ne!(mirrored, bx.solid);
    }

    #[test]
    fn test_offset_wire() {
        let pts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(5.0, 0.0, 0.0),
        ];
        let offset = offset_wire(&pts, 1.0, Vec3::Z).unwrap();
        assert_eq!(offset.len(), 2);
        assert!((offset[0].y - 1.0).abs() < 1e-10 || (offset[0].y + 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_join_wires() {
        let w1 = vec![Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)];
        let w2 = vec![Point3::new(1.0, 0.0, 0.0), Point3::new(2.0, 0.0, 0.0)];
        let joined = join_wires(&[w1, w2], 1e-6);
        assert_eq!(joined.len(), 3);
    }

    #[test]
    fn test_split_wire() {
        let pts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(3.0, 0.0, 0.0),
        ];
        let (a, b) = split_wire(&pts, 2).unwrap();
        assert_eq!(a.len(), 3);
        assert_eq!(b.len(), 2);
        assert!(split_wire(&pts, 0).is_err());
    }

    #[test]
    fn test_upgrade_wire() {
        let pts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
        ];
        let closed = upgrade_wire(&pts);
        assert_eq!(closed.len(), 4);
        assert!(closed[0].distance_to(*closed.last().unwrap()) < 1e-10);
    }

    #[test]
    fn test_downgrade_solid() {
        let mut model = BRepModel::new();
        let bx = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let points = downgrade_solid(&model, bx.solid).unwrap();
        assert!(points.len() >= 8, "box should have at least 8 unique vertices");
    }

    #[test]
    fn test_wire_to_bspline_roundtrip() {
        let pts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(3.0, 1.0, 0.0),
        ];
        let curve = wire_to_bspline(&pts, 3).unwrap();
        let back = bspline_to_wire(&curve, 20).unwrap();
        assert_eq!(back.len(), 21);
        // Endpoints should match
        assert!(back[0].distance_to(pts[0]) < 1e-6);
        assert!(back[20].distance_to(pts[3]) < 1e-6);
    }

    #[test]
    fn test_make_label() {
        let label = make_label("Test", Point3::new(1.0, 2.0, 0.0), Some(Point3::ORIGIN));
        assert_eq!(label.text, "Test");
        assert!(label.leader_target.is_some());
    }

    #[test]
    fn test_make_draft_dimension() {
        let dim = make_draft_dimension(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(3.0, 4.0, 0.0),
            1.0,
        );
        assert!((dim.distance - 5.0).abs() < 1e-10);
        assert!((dim.midpoint.x - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_snap_to_endpoint() {
        let wire = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(5.0, 0.0, 0.0),
            Point3::new(5.0, 5.0, 0.0),
        ];
        let result = snap_to_endpoint(&wire, Point3::new(4.9, 0.1, 0.0)).unwrap();
        assert!((result.point.x - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_snap_to_midpoint() {
        let wire = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(10.0, 0.0, 0.0),
        ];
        let result = snap_to_midpoint(&wire, Point3::new(5.0, 1.0, 0.0)).unwrap();
        assert!((result.point.x - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_snap_to_nearest() {
        let wire = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(10.0, 0.0, 0.0),
        ];
        let result = snap_to_nearest(&wire, Point3::new(3.0, 2.0, 0.0)).unwrap();
        assert!((result.point.x - 3.0).abs() < 1e-10);
        assert!((result.point.y - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_wire_length() {
        let pts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(3.0, 0.0, 0.0),
            Point3::new(3.0, 4.0, 0.0),
        ];
        let len = wire_length(&pts);
        // 3.0 + 4.0 = 7.0
        assert!((len - 7.0).abs() < 1e-10);
    }

    #[test]
    fn test_wire_area() {
        // Unit square
        let pts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 0.0),
        ];
        let area = wire_area(&pts, Vec3::Z);
        assert!((area - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_make_arc_3pt_wire() {
        let pts = make_arc_3pt_wire(
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(-1.0, 0.0, 0.0),
            16,
        )
        .unwrap();
        assert_eq!(pts.len(), 17);
        assert!((pts[0].x - 1.0).abs() < 1e-6);
        assert!((pts[16].x + 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_make_chamfer_wire() {
        let pts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(5.0, 0.0, 0.0),
            Point3::new(5.0, 5.0, 0.0),
        ];
        let chamfered = make_chamfer_wire(&pts, 1.0);
        assert!(chamfered.len() > pts.len());
    }

    #[test]
    fn test_stretch_wire() {
        let pts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(5.0, 0.0, 0.0),
        ];
        let stretched = stretch_wire(&pts, Point3::new(0.5, 0.0, 0.0), 2.0, Vec3::new(0.0, 3.0, 0.0));
        assert_eq!(stretched.len(), 3);
        // Point at (0,0,0) is within radius 2 of center (0.5,0,0), so y should be offset
        assert!(stretched[0].y > 0.0);
        // Point at (5,0,0) is outside radius, so unchanged
        assert!((stretched[2].y - 0.0).abs() < 1e-12);
    }

    #[test]
    fn test_make_arc_wire() {
        let pts = make_arc_wire(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            8,
        )
        .unwrap();
        assert_eq!(pts.len(), 9);
        // First point should be start
        assert!((pts[0].x - 1.0).abs() < 1e-10);
        // Last point should be near end
        assert!((pts.last().unwrap().y - 1.0).abs() < 1e-10);
    }
}
