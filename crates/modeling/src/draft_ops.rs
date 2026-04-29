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

/// Creates a wire consisting of a single straight-line edge between two points.
///
/// This is a convenience wrapper around [`make_wire`] for the common case
/// of constructing a simple two-point line in the Draft workbench.
pub fn make_line_draft(
    model: &mut BRepModel,
    p1: Point3,
    p2: Point3,
) -> KernelResult<WireResult> {
    make_wire(model, &[p1, p2])
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
    let u = n.cross(arbitrary).normalized().unwrap_or(Vec3::X);
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
    let u = n.cross(arbitrary).normalized().unwrap_or(Vec3::X);
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
    let u = n.cross(arbitrary).normalized().unwrap_or(Vec3::X);
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
        .min_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
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
        .min_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
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
        .min_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
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

/// Hatch pattern specification.
#[derive(Debug, Clone)]
pub enum HatchPattern {
    Lines { angle: f64, spacing: f64 },
    Cross { angle: f64, spacing: f64 },
    Dots { spacing: f64 },
}

/// Result of a hatch fill operation.
#[derive(Debug, Clone)]
pub struct HatchResult {
    pub lines: Vec<(Point3, Point3)>,
    pub pattern: HatchPattern,
}

/// Generate 2D hatch fill lines within a convex polygon boundary.
///
/// Lines are generated at the given `angle` and `spacing` from the pattern,
/// clipped to the bounding box of the boundary points.
pub fn draft_hatch(
    boundary_points: &[Point3],
    pattern: HatchPattern,
    scale: f64,
) -> KernelResult<HatchResult> {
    if boundary_points.len() < 3 {
        return Err(KernelError::InvalidArgument(
            "hatch requires at least 3 boundary points".into(),
        ));
    }
    if scale <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "scale must be positive".into(),
        ));
    }

    let (angle, spacing) = match &pattern {
        HatchPattern::Lines { angle, spacing } => (*angle, *spacing * scale),
        HatchPattern::Cross { angle, spacing } => (*angle, *spacing * scale),
        HatchPattern::Dots { spacing } => (0.0, *spacing * scale),
    };

    if spacing <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "hatch spacing must be positive".into(),
        ));
    }

    // Bounding box in XY
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for p in boundary_points {
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
    }

    let cos_a = angle.cos();
    let sin_a = angle.sin();
    let diag = ((max_x - min_x).powi(2) + (max_y - min_y).powi(2)).sqrt();
    let cx = (min_x + max_x) * 0.5;
    let cy = (min_y + max_y) * 0.5;

    let n_lines = (diag / spacing).ceil() as i32 + 1;
    let mut lines = Vec::new();

    for i in -n_lines..=n_lines {
        let offset = i as f64 * spacing;
        let ox = cx + offset * (-sin_a);
        let oy = cy + offset * cos_a;
        let p1 = Point3::new(ox - diag * cos_a, oy - diag * sin_a, 0.0);
        let p2 = Point3::new(ox + diag * cos_a, oy + diag * sin_a, 0.0);
        lines.push((p1, p2));
    }

    // For Cross pattern, add perpendicular lines
    if let HatchPattern::Cross { angle: a, .. } = &pattern {
        let a2 = a + std::f64::consts::FRAC_PI_2;
        let cos_a2 = a2.cos();
        let sin_a2 = a2.sin();
        for i in -n_lines..=n_lines {
            let offset = i as f64 * spacing;
            let ox = cx + offset * (-sin_a2);
            let oy = cy + offset * cos_a2;
            let p1 = Point3::new(ox - diag * cos_a2, oy - diag * sin_a2, 0.0);
            let p2 = Point3::new(ox + diag * cos_a2, oy + diag * sin_a2, 0.0);
            lines.push((p1, p2));
        }
    }

    Ok(HatchResult { lines, pattern })
}

/// Extract faces from a model by face handle indices.
///
/// Creates a new solid containing only the specified faces.
pub fn make_facebinder(
    model: &mut BRepModel,
    face_handles: &[Handle<FaceData>],
) -> KernelResult<Handle<SolidData>> {
    if face_handles.is_empty() {
        return Err(KernelError::InvalidArgument(
            "make_facebinder requires at least one face".into(),
        ));
    }

    let op = model.history.next_operation("facebinder");

    let mut new_faces = Vec::new();

    for (fi, &face_h) in face_handles.iter().enumerate() {
        let face = model
            .faces
            .get(face_h)
            .ok_or(KernelError::InvalidHandle("face"))?;
        let outer_loop_h = face.outer_loop;
        let hes = model.loop_half_edges(
            model
                .loops
                .get(outer_loop_h)
                .ok_or(KernelError::InvalidHandle("loop"))?
                .half_edge,
        );

        let mut face_points = Vec::new();
        for &he_h in &hes {
            if let Some(he) = model.half_edges.get(he_h) {
                if let Some(v) = model.vertices.get(he.origin) {
                    face_points.push(v.point);
                }
            }
        }

        if face_points.len() < 3 {
            continue;
        }

        let mut verts = Vec::new();
        for (vi, &pt) in face_points.iter().enumerate() {
            let tag = Tag::generated(EntityKind::Vertex, op, (fi * 100 + vi) as u32);
            verts.push(model.add_vertex_tagged(pt, tag));
        }

        let mut he_list = Vec::new();
        for i in 0..verts.len() {
            let j = (i + 1) % verts.len();
            let tag = Tag::generated(EntityKind::Edge, op, (fi * 100 + i) as u32);
            let (_, fwd, _) = model.add_edge_tagged(verts[i], verts[j], tag);
            he_list.push(fwd);
        }

        let loop_h = model.make_loop(&he_list)?;
        let face_tag = Tag::generated(EntityKind::Face, op, fi as u32);
        let new_face_h = model.make_face_tagged(loop_h, face_tag);
        new_faces.push(new_face_h);
    }

    if new_faces.is_empty() {
        return Err(KernelError::InvalidArgument(
            "no valid faces could be created from the input".into(),
        ));
    }

    let shell_tag = Tag::generated(EntityKind::Shell, op, 0);
    let shell_h = model.make_shell_tagged(&new_faces, shell_tag);
    let solid_tag = Tag::generated(EntityKind::Solid, op, 0);
    let solid_h = model.make_solid_tagged(&[shell_h], solid_tag);

    Ok(solid_h)
}

/// Dimension type for draft annotations.
#[derive(Debug, Clone)]
pub enum DraftDimensionType {
    Linear,
    Angular,
    Radial,
    Diameter,
    ArcLength,
}

/// Full draft dimension with type and offset.
#[derive(Debug, Clone)]
pub struct DraftDimensionFull {
    pub dim_type: DraftDimensionType,
    pub start: Point3,
    pub end: Point3,
    pub value: f64,
    pub midpoint: Point3,
    pub offset_point: Point3,
}

/// Create a full draft dimension with type.
pub fn make_draft_dimension_full(
    dim_type: DraftDimensionType,
    p1: Point3,
    p2: Point3,
    offset: f64,
) -> DraftDimensionFull {
    let diff = p2 - p1;
    let distance = diff.length();
    let value = match &dim_type {
        DraftDimensionType::Angular => {
            // Angle between p1 and p2 relative to origin
            let d1 = Vec3::new(p1.x, p1.y, p1.z);
            let d2 = Vec3::new(p2.x, p2.y, p2.z);
            let dot = d1.dot(d2);
            let l1 = d1.length();
            let l2 = d2.length();
            if l1 > 1e-15 && l2 > 1e-15 {
                (dot / (l1 * l2)).clamp(-1.0, 1.0).acos()
            } else {
                0.0
            }
        }
        DraftDimensionType::Radial | DraftDimensionType::Diameter => {
            let r = distance;
            match dim_type {
                DraftDimensionType::Diameter => r * 2.0,
                _ => r,
            }
        }
        _ => distance,
    };
    let midpoint = Point3::new(
        (p1.x + p2.x) * 0.5,
        (p1.y + p2.y) * 0.5,
        (p1.z + p2.z) * 0.5,
    );
    let normal = if distance > 1e-15 {
        let dir = Vec3::new(diff.x / distance, diff.y / distance, diff.z / distance);
        Vec3::new(-dir.y, dir.x, 0.0)
    } else {
        Vec3::Y
    };
    let offset_point = Point3::new(
        midpoint.x + normal.x * offset,
        midpoint.y + normal.y * offset,
        midpoint.z + normal.z * offset,
    );

    DraftDimensionFull {
        dim_type,
        start: p1,
        end: p2,
        value,
        midpoint,
        offset_point,
    }
}

/// Full draft label with leader line target.
#[derive(Debug, Clone)]
pub struct DraftLabelFull {
    pub text: String,
    pub position: Point3,
    pub leader_target: Option<Point3>,
    pub style: AnnotationStyle,
}

/// Create a full label with leader and style.
pub fn make_label_full(
    text: &str,
    position: Point3,
    leader_target: Option<Point3>,
    style: AnnotationStyle,
) -> DraftLabelFull {
    DraftLabelFull {
        text: text.to_string(),
        position,
        leader_target,
        style,
    }
}

/// Annotation style settings.
#[derive(Debug, Clone)]
pub struct AnnotationStyle {
    pub font_size: f64,
    pub font_color: [f64; 3],
    pub line_width: f64,
    pub line_color: [f64; 3],
    pub arrow_size: f64,
}

impl Default for AnnotationStyle {
    fn default() -> Self {
        Self {
            font_size: 12.0,
            font_color: [0.0, 0.0, 0.0],
            line_width: 1.0,
            line_color: [0.0, 0.0, 0.0],
            arrow_size: 3.0,
        }
    }
}

/// Snap modes for draft editing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapMode {
    Endpoint,
    Midpoint,
    Center,
    Angle,
    Intersection,
    Perpendicular,
    Extension,
    Parallel,
    Special,
    Nearest,
    Grid,
    WorkingPlane,
    Dimensions,
}

/// Find the nearest snap target in a wire based on snap mode.
pub fn snap_to_point(
    wire: &[Point3],
    query: Point3,
    mode: SnapMode,
) -> KernelResult<SnapResult> {
    match mode {
        SnapMode::Endpoint => snap_to_endpoint(wire, query)
            .ok_or(KernelError::InvalidArgument("no snap target found".into())),
        SnapMode::Midpoint => snap_to_midpoint(wire, query)
            .ok_or(KernelError::InvalidArgument("no snap target found".into())),
        SnapMode::Nearest => snap_to_nearest(wire, query)
            .ok_or(KernelError::InvalidArgument("no snap target found".into())),
        SnapMode::Center => {
            // Center of bounding box
            if wire.is_empty() {
                return Err(KernelError::InvalidArgument("empty wire".into()));
            }
            let mut cx = 0.0;
            let mut cy = 0.0;
            let mut cz = 0.0;
            for p in wire {
                cx += p.x;
                cy += p.y;
                cz += p.z;
            }
            let n = wire.len() as f64;
            let center = Point3::new(cx / n, cy / n, cz / n);
            Ok(SnapResult {
                point: center,
                distance: center.distance_to(query),
            })
        }
        SnapMode::Grid => Ok(snap_to_grid(query, 1.0)),
        SnapMode::Angle => snap_to_angle(wire, query)
            .ok_or(KernelError::InvalidArgument("no snap target found".into())),
        SnapMode::Intersection => snap_to_intersection(wire, query)
            .ok_or(KernelError::InvalidArgument("no intersection found".into())),
        SnapMode::Perpendicular => snap_to_perpendicular(wire, query)
            .ok_or(KernelError::InvalidArgument("no snap target found".into())),
        SnapMode::Extension => snap_to_extension(wire, query)
            .ok_or(KernelError::InvalidArgument("no snap target found".into())),
        SnapMode::Parallel => snap_to_parallel(wire, query)
            .ok_or(KernelError::InvalidArgument("no snap target found".into())),
        SnapMode::Special => snap_to_special(wire, query)
            .ok_or(KernelError::InvalidArgument("no snap target found".into())),
        SnapMode::WorkingPlane => Ok(snap_to_working_plane(query, Point3::ORIGIN, Vec3::Z)),
        SnapMode::Dimensions => {
            // For generic snap_to_point, fall back to nearest
            snap_to_nearest(wire, query)
                .ok_or(KernelError::InvalidArgument("no snap target found".into()))
        }
    }
}

/// Lock a point's position along an axis constraint.
pub fn snap_lock(position: Point3, locked_axis: Vec3) -> Point3 {
    let n = locked_axis
        .normalized()
        .unwrap_or(Vec3::X);
    let proj = Vec3::new(position.x, position.y, position.z).dot(n);
    Point3::new(n.x * proj, n.y * proj, n.z * proj)
}

/// Trim or extend a wire to a target point.
///
/// If the target is beyond the wire, extends the last segment.
/// If the target is inside, trims the wire at the nearest edge point.
pub fn trimex_draft(points: &[Point3], target: Point3) -> KernelResult<Vec<Point3>> {
    if points.len() < 2 {
        return Err(KernelError::InvalidArgument(
            "trimex requires at least 2 points".into(),
        ));
    }

    // Find nearest point on wire
    let mut best_idx = 0;
    let mut best_t = 0.0;
    let mut best_dist = f64::INFINITY;

    for i in 0..points.len() - 1 {
        let a = points[i];
        let b = points[i + 1];
        let ab = b - a;
        let at = target - a;
        let ab_len_sq = ab.length_squared();
        let t = if ab_len_sq > 1e-30 {
            at.dot(ab) / ab_len_sq
        } else {
            0.0
        };
        let closest = Point3::new(a.x + t * ab.x, a.y + t * ab.y, a.z + t * ab.z);
        let d = closest.distance_to(target);
        if d < best_dist {
            best_dist = d;
            best_idx = i;
            best_t = t;
        }
    }

    if best_t <= 0.0 && best_idx == 0 {
        // Extend from the start
        let mut result = vec![target];
        result.extend_from_slice(points);
        Ok(result)
    } else if best_t >= 1.0 && best_idx == points.len() - 2 {
        // Extend from the end
        let mut result = points.to_vec();
        result.push(target);
        Ok(result)
    } else {
        // Trim at the nearest point
        let clamped_t = best_t.clamp(0.0, 1.0);
        let a = points[best_idx];
        let b = points[best_idx + 1];
        let trim_pt = Point3::new(
            a.x + clamped_t * (b.x - a.x),
            a.y + clamped_t * (b.y - a.y),
            a.z + clamped_t * (b.z - a.z),
        );
        let mut result: Vec<Point3> = points[..=best_idx].to_vec();
        result.push(trim_pt);
        Ok(result)
    }
}

/// Circular array with specified total angle (not necessarily full 360).
pub fn circular_array(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    center: Point3,
    axis: Vec3,
    count: usize,
    total_angle: f64,
) -> KernelResult<Vec<Handle<SolidData>>> {
    if count < 2 {
        return Err(KernelError::InvalidArgument(
            "circular_array count must be at least 2".into(),
        ));
    }
    if total_angle.abs() < 1e-15 {
        return Err(KernelError::InvalidArgument(
            "total_angle must be non-zero".into(),
        ));
    }
    let dir = axis.normalized().ok_or(KernelError::InvalidArgument(
        "axis must be non-zero".into(),
    ))?;

    let mut solids = vec![solid];
    let angle_step = total_angle / count as f64;

    for i in 1..count {
        let angle = angle_step * i as f64;
        let cos_a = angle.cos();
        let sin_a = angle.sin();

        let op = model.history.next_operation("circular_array");
        let result = copy_solid_transformed(
            model,
            solid,
            op,
            |pt| {
                let v = pt - center;
                let along = dir * v.dot(dir);
                let perp = v - along;
                let perp_len = perp.length();
                if perp_len < 1e-15 {
                    return pt;
                }
                let u = Vec3::new(perp.x / perp_len, perp.y / perp_len, perp.z / perp_len);
                let w = dir.cross(u);
                let rotated = u * (perp_len * cos_a) + w * (perp_len * sin_a) + along;
                Point3::new(center.x + rotated.x, center.y + rotated.y, center.z + rotated.z)
            },
            false,
        )?;
        solids.push(result.solid);
    }

    Ok(solids)
}

/// Place copies along a path, linking each copy to the path point.
pub fn path_link_array(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    path_points: &[Point3],
) -> KernelResult<Vec<Handle<SolidData>>> {
    if path_points.len() < 2 {
        return Err(KernelError::InvalidArgument(
            "path_link_array requires at least 2 path points".into(),
        ));
    }

    let origin = path_points[0];
    let mut solids = vec![solid];

    for &target in &path_points[1..] {
        let offset = target - origin;
        let op = model.history.next_operation("path_link_array");
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

/// Place copies at each specified point.
pub fn point_link_array(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    points: &[Point3],
) -> KernelResult<Vec<Handle<SolidData>>> {
    if points.len() < 2 {
        return Err(KernelError::InvalidArgument(
            "point_link_array requires at least 2 points".into(),
        ));
    }

    let origin = points[0];
    let mut solids = vec![solid];

    for &target in &points[1..] {
        let offset = target - origin;
        let op = model.history.next_operation("point_link_array");
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

/// Edit a vertex position in a wire.
pub fn edit_draft(points: &mut [Point3], index: usize, new_position: Point3) -> KernelResult<()> {
    if index >= points.len() {
        return Err(KernelError::InvalidArgument(format!(
            "index {} out of range ({})",
            index,
            points.len()
        )));
    }
    points[index] = new_position;
    Ok(())
}

/// Convert a draft wire (polyline) into a sketch representation.
///
/// Each wire segment becomes a `SketchLine`. Returns a `Sketch` object.
pub fn draft_to_sketch(points: &[Point3]) -> KernelResult<cadkernel_sketch::Sketch> {
    if points.len() < 2 {
        return Err(KernelError::InvalidArgument(
            "draft_to_sketch requires at least 2 points".into(),
        ));
    }

    let mut sketch = cadkernel_sketch::Sketch::new();

    // Add points as sketch entities
    let mut point_ids = Vec::with_capacity(points.len());
    for &pt in points {
        point_ids.push(sketch.add_point(pt.x, pt.y));
    }

    // Add line segments between consecutive points
    for i in 0..point_ids.len() - 1 {
        sketch.add_line(point_ids[i], point_ids[i + 1]);
    }

    Ok(sketch)
}

/// Creates a cubic Bezier wire (degree 3) from 4 control points.
///
/// This is a convenience wrapper around [`make_bezier_wire`] that specifically
/// takes start, control1, control2, and end points for a single cubic Bezier segment.
pub fn make_cubic_bezier_wire(
    p0: Point3,
    p1: Point3,
    p2: Point3,
    p3: Point3,
    segments: usize,
) -> KernelResult<Vec<Point3>> {
    make_bezier_wire(p0, p1, p2, p3, segments)
}

/// Creates a shape (polyline) from a text string.
///
/// Generates a set of point sequences representing each character as a simple
/// stroke font. Each character is a fixed-width block placed left-to-right.
/// Returns a vector of polyline segments (each segment is a list of points).
pub fn shape_from_text(
    text: &str,
    position: Point3,
    height: f64,
    normal: Vec3,
) -> KernelResult<Vec<Vec<Point3>>> {
    if text.is_empty() {
        return Err(KernelError::InvalidArgument(
            "text must not be empty".into(),
        ));
    }
    if height <= 0.0 {
        return Err(KernelError::InvalidArgument(
            "height must be positive".into(),
        ));
    }

    let n = normal.normalized().unwrap_or(Vec3::Z);
    // Build a local coordinate frame on the plane
    let arbitrary = if n.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
    let right = n.cross(arbitrary).normalized().unwrap_or(Vec3::X);
    let up = right.cross(n).normalized().unwrap_or(Vec3::Y);

    let char_width = height * 0.6;
    let mut result = Vec::new();

    for (ci, ch) in text.chars().enumerate() {
        let x_off = ci as f64 * char_width * 1.2;
        let strokes = char_strokes(ch);
        for stroke in strokes {
            let pts: Vec<Point3> = stroke
                .iter()
                .map(|&(sx, sy)| {
                    let x = x_off + sx * char_width;
                    let y = sy * height;
                    Point3::new(
                        position.x + right.x * x + up.x * y,
                        position.y + right.y * x + up.y * y,
                        position.z + right.z * x + up.z * y,
                    )
                })
                .collect();
            if pts.len() >= 2 {
                result.push(pts);
            }
        }
    }

    if result.is_empty() {
        return Err(KernelError::InvalidArgument(
            "no renderable characters in text".into(),
        ));
    }
    Ok(result)
}

/// Returns stroke data for a character as normalized (0..1, 0..1) coordinate pairs.
fn char_strokes(ch: char) -> Vec<Vec<(f64, f64)>> {
    match ch.to_ascii_uppercase() {
        'A' => vec![
            vec![(0.0, 0.0), (0.5, 1.0), (1.0, 0.0)],
            vec![(0.2, 0.4), (0.8, 0.4)],
        ],
        'B' => vec![
            vec![(0.0, 0.0), (0.0, 1.0), (0.7, 1.0), (0.8, 0.85), (0.7, 0.5), (0.0, 0.5)],
            vec![(0.0, 0.5), (0.7, 0.5), (0.8, 0.35), (0.8, 0.15), (0.7, 0.0), (0.0, 0.0)],
        ],
        'C' => vec![vec![(1.0, 0.0), (0.0, 0.0), (0.0, 1.0), (1.0, 1.0)]],
        'D' => vec![vec![(0.0, 0.0), (0.0, 1.0), (0.6, 1.0), (1.0, 0.5), (0.6, 0.0), (0.0, 0.0)]],
        'E' => vec![
            vec![(1.0, 0.0), (0.0, 0.0), (0.0, 1.0), (1.0, 1.0)],
            vec![(0.0, 0.5), (0.7, 0.5)],
        ],
        'F' => vec![
            vec![(0.0, 0.0), (0.0, 1.0), (1.0, 1.0)],
            vec![(0.0, 0.5), (0.7, 0.5)],
        ],
        'G' => vec![vec![(1.0, 1.0), (0.0, 1.0), (0.0, 0.0), (1.0, 0.0), (1.0, 0.5), (0.5, 0.5)]],
        'H' => vec![
            vec![(0.0, 0.0), (0.0, 1.0)],
            vec![(1.0, 0.0), (1.0, 1.0)],
            vec![(0.0, 0.5), (1.0, 0.5)],
        ],
        'I' => vec![
            vec![(0.3, 0.0), (0.7, 0.0)],
            vec![(0.5, 0.0), (0.5, 1.0)],
            vec![(0.3, 1.0), (0.7, 1.0)],
        ],
        'L' => vec![vec![(0.0, 1.0), (0.0, 0.0), (1.0, 0.0)]],
        'M' => vec![vec![(0.0, 0.0), (0.0, 1.0), (0.5, 0.5), (1.0, 1.0), (1.0, 0.0)]],
        'N' => vec![vec![(0.0, 0.0), (0.0, 1.0), (1.0, 0.0), (1.0, 1.0)]],
        'O' => vec![vec![(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0), (0.0, 0.0)]],
        'P' => vec![vec![(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.5), (0.0, 0.5)]],
        'R' => vec![
            vec![(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.5), (0.0, 0.5)],
            vec![(0.5, 0.5), (1.0, 0.0)],
        ],
        'S' => vec![vec![(1.0, 1.0), (0.0, 1.0), (0.0, 0.5), (1.0, 0.5), (1.0, 0.0), (0.0, 0.0)]],
        'T' => vec![
            vec![(0.0, 1.0), (1.0, 1.0)],
            vec![(0.5, 1.0), (0.5, 0.0)],
        ],
        'U' => vec![vec![(0.0, 1.0), (0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]],
        'V' => vec![vec![(0.0, 1.0), (0.5, 0.0), (1.0, 1.0)]],
        'W' => vec![vec![(0.0, 1.0), (0.25, 0.0), (0.5, 0.5), (0.75, 0.0), (1.0, 1.0)]],
        'X' => vec![
            vec![(0.0, 0.0), (1.0, 1.0)],
            vec![(0.0, 1.0), (1.0, 0.0)],
        ],
        'Y' => vec![
            vec![(0.0, 1.0), (0.5, 0.5)],
            vec![(1.0, 1.0), (0.5, 0.5)],
            vec![(0.5, 0.5), (0.5, 0.0)],
        ],
        'Z' => vec![vec![(0.0, 1.0), (1.0, 1.0), (0.0, 0.0), (1.0, 0.0)]],
        '0'..='9' => digit_strokes(ch),
        ' ' => vec![],
        '-' => vec![vec![(0.1, 0.5), (0.9, 0.5)]],
        '.' => vec![vec![(0.4, 0.0), (0.6, 0.0)]],
        _ => vec![vec![(0.0, 0.0), (1.0, 1.0), (0.0, 1.0), (1.0, 0.0)]],
    }
}

/// Returns stroke data for digit characters.
fn digit_strokes(ch: char) -> Vec<Vec<(f64, f64)>> {
    match ch {
        '0' => vec![vec![(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0), (0.0, 0.0)]],
        '1' => vec![vec![(0.3, 0.8), (0.5, 1.0), (0.5, 0.0)]],
        '2' => vec![vec![(0.0, 0.8), (0.0, 1.0), (1.0, 1.0), (1.0, 0.5), (0.0, 0.0), (1.0, 0.0)]],
        '3' => vec![
            vec![(0.0, 1.0), (1.0, 1.0), (1.0, 0.5), (0.2, 0.5)],
            vec![(1.0, 0.5), (1.0, 0.0), (0.0, 0.0)],
        ],
        '4' => vec![vec![(0.0, 1.0), (0.0, 0.5), (1.0, 0.5)], vec![(0.7, 1.0), (0.7, 0.0)]],
        '5' => vec![vec![(1.0, 1.0), (0.0, 1.0), (0.0, 0.5), (1.0, 0.5), (1.0, 0.0), (0.0, 0.0)]],
        '6' => vec![vec![(1.0, 1.0), (0.0, 0.5), (0.0, 0.0), (1.0, 0.0), (1.0, 0.5), (0.0, 0.5)]],
        '7' => vec![vec![(0.0, 1.0), (1.0, 1.0), (0.3, 0.0)]],
        '8' => vec![
            vec![(0.0, 0.5), (0.0, 1.0), (1.0, 1.0), (1.0, 0.5), (0.0, 0.5)],
            vec![(0.0, 0.5), (0.0, 0.0), (1.0, 0.0), (1.0, 0.5)],
        ],
        '9' => vec![vec![(1.0, 0.5), (0.0, 0.5), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0)]],
        _ => vec![],
    }
}

/// Snap to the centroid (center) of a wire.
pub fn snap_to_center(wire: &[Point3], query: Point3) -> Option<SnapResult> {
    if wire.is_empty() {
        return None;
    }
    let mut cx = 0.0;
    let mut cy = 0.0;
    let mut cz = 0.0;
    for p in wire {
        cx += p.x;
        cy += p.y;
        cz += p.z;
    }
    let n = wire.len() as f64;
    let center = Point3::new(cx / n, cy / n, cz / n);
    Some(SnapResult {
        point: center,
        distance: center.distance_to(query),
    })
}

/// Snap to the nearest angle-constrained direction from the first point.
///
/// Constrains movement to multiples of 15 degrees relative to the X axis.
pub fn snap_to_angle(wire: &[Point3], query: Point3) -> Option<SnapResult> {
    let origin = *wire.first()?;
    let dx = query.x - origin.x;
    let dy = query.y - origin.y;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist < 1e-15 {
        return Some(SnapResult {
            point: origin,
            distance: 0.0,
        });
    }
    let angle = dy.atan2(dx);
    let step = std::f64::consts::PI / 12.0; // 15 degrees
    let snapped_angle = (angle / step).round() * step;
    let p = Point3::new(
        origin.x + dist * snapped_angle.cos(),
        origin.y + dist * snapped_angle.sin(),
        query.z,
    );
    Some(SnapResult {
        point: p,
        distance: p.distance_to(query),
    })
}

/// Snap to the nearest intersection of two wire segments.
///
/// Checks all segment pairs for 2D intersections (projected on Z=0).
pub fn snap_to_intersection(wire: &[Point3], query: Point3) -> Option<SnapResult> {
    if wire.len() < 4 {
        return None;
    }
    let mut best: Option<SnapResult> = None;
    let n = wire.len();
    for i in 0..n - 1 {
        for j in (i + 2)..n - 1 {
            if let Some(pt) = segment_intersect_2d(wire[i], wire[i + 1], wire[j], wire[j + 1]) {
                let d = pt.distance_to(query);
                if best.as_ref().is_none_or(|b| d < b.distance) {
                    best = Some(SnapResult {
                        point: pt,
                        distance: d,
                    });
                }
            }
        }
    }
    best
}

/// 2D segment-segment intersection (ignoring z).
fn segment_intersect_2d(a0: Point3, a1: Point3, b0: Point3, b1: Point3) -> Option<Point3> {
    let d1x = a1.x - a0.x;
    let d1y = a1.y - a0.y;
    let d2x = b1.x - b0.x;
    let d2y = b1.y - b0.y;
    let denom = d1x * d2y - d1y * d2x;
    if denom.abs() < 1e-15 {
        return None;
    }
    let t = ((b0.x - a0.x) * d2y - (b0.y - a0.y) * d2x) / denom;
    let u = ((b0.x - a0.x) * d1y - (b0.y - a0.y) * d1x) / denom;
    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Some(Point3::new(a0.x + t * d1x, a0.y + t * d1y, a0.z))
    } else {
        None
    }
}

/// Snap to the nearest perpendicular foot on a wire segment.
pub fn snap_to_perpendicular(wire: &[Point3], query: Point3) -> Option<SnapResult> {
    snap_to_nearest(wire, query)
}

/// Snap along the extension direction of the nearest wire segment end.
pub fn snap_to_extension(wire: &[Point3], query: Point3) -> Option<SnapResult> {
    if wire.len() < 2 {
        return None;
    }
    let n = wire.len();
    // Extend from the last segment
    let dir = (wire[n - 1] - wire[n - 2]).normalized()?;
    let v = Vec3::new(
        query.x - wire[n - 1].x,
        query.y - wire[n - 1].y,
        query.z - wire[n - 1].z,
    );
    let proj = v.dot(dir);
    let p = Point3::new(
        wire[n - 1].x + dir.x * proj,
        wire[n - 1].y + dir.y * proj,
        wire[n - 1].z + dir.z * proj,
    );
    Some(SnapResult {
        point: p,
        distance: p.distance_to(query),
    })
}

/// Snap to the nearest point along a direction parallel to the nearest segment.
pub fn snap_to_parallel(wire: &[Point3], query: Point3) -> Option<SnapResult> {
    if wire.len() < 2 {
        return None;
    }
    // Find nearest segment
    let mut best_seg = 0;
    let mut best_dist = f64::MAX;
    for i in 0..wire.len() - 1 {
        let mid = Point3::new(
            (wire[i].x + wire[i + 1].x) * 0.5,
            (wire[i].y + wire[i + 1].y) * 0.5,
            (wire[i].z + wire[i + 1].z) * 0.5,
        );
        let d = mid.distance_to(query);
        if d < best_dist {
            best_dist = d;
            best_seg = i;
        }
    }
    let dir = (wire[best_seg + 1] - wire[best_seg]).normalized()?;
    let v = Vec3::new(
        query.x - wire[best_seg].x,
        query.y - wire[best_seg].y,
        query.z - wire[best_seg].z,
    );
    let proj = v.dot(dir);
    let p = Point3::new(
        wire[best_seg].x + dir.x * proj,
        wire[best_seg].y + dir.y * proj,
        wire[best_seg].z + dir.z * proj,
    );
    Some(SnapResult {
        point: p,
        distance: p.distance_to(query),
    })
}

/// Special snap: combines endpoint, midpoint, and center, picking the closest.
pub fn snap_to_special(wire: &[Point3], query: Point3) -> Option<SnapResult> {
    let candidates = [
        snap_to_endpoint(wire, query),
        snap_to_midpoint(wire, query),
        snap_to_center(wire, query),
    ];
    candidates
        .into_iter()
        .flatten()
        .min_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal))
}

/// Snap to the nearest grid point with configurable spacing.
pub fn snap_to_grid(query: Point3, spacing: f64) -> SnapResult {
    let s = if spacing > 0.0 { spacing } else { 1.0 };
    let snapped = Point3::new(
        (query.x / s).round() * s,
        (query.y / s).round() * s,
        (query.z / s).round() * s,
    );
    SnapResult {
        point: snapped,
        distance: snapped.distance_to(query),
    }
}

/// Snap a point to the nearest position on a working plane defined by origin and normal.
pub fn snap_to_working_plane(query: Point3, plane_origin: Point3, plane_normal: Vec3) -> SnapResult {
    let n = plane_normal.normalized().unwrap_or(Vec3::Z);
    let v = Vec3::new(
        query.x - plane_origin.x,
        query.y - plane_origin.y,
        query.z - plane_origin.z,
    );
    let dist = v.dot(n);
    let projected = Point3::new(
        query.x - n.x * dist,
        query.y - n.y * dist,
        query.z - n.z * dist,
    );
    SnapResult {
        point: projected,
        distance: projected.distance_to(query),
    }
}

/// Snap to the nearest dimension reference point (start, end, or midpoint of a dimension).
pub fn snap_to_dimensions(dim_start: Point3, dim_end: Point3, query: Point3) -> SnapResult {
    let mid = Point3::new(
        (dim_start.x + dim_end.x) * 0.5,
        (dim_start.y + dim_end.y) * 0.5,
        (dim_start.z + dim_end.z) * 0.5,
    );
    let candidates = [dim_start, dim_end, mid];
    let (point, distance) = candidates
        .iter()
        .map(|p| (*p, p.distance_to(query)))
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap();
    SnapResult { point, distance }
}

// ---------------------------------------------------------------------------
// Draft Layers
// ---------------------------------------------------------------------------

/// A draft layer groups objects with shared visual settings.
#[derive(Debug, Clone)]
pub struct DraftLayer {
    pub name: String,
    pub color: (f64, f64, f64),
    pub line_type: String,
    pub line_width: f64,
    pub visible: bool,
    pub locked: bool,
    pub objects: Vec<usize>,
}

impl DraftLayer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            color: (0.0, 0.0, 0.0),
            line_type: "Solid".into(),
            line_width: 1.0,
            visible: true,
            locked: false,
            objects: Vec::new(),
        }
    }
}

/// Manages a stack of draft layers.
#[derive(Debug, Clone)]
pub struct LayerManager {
    pub layers: Vec<DraftLayer>,
    pub active_layer: usize,
}

impl LayerManager {
    pub fn new() -> Self {
        Self {
            layers: vec![DraftLayer::new("Default")],
            active_layer: 0,
        }
    }

    pub fn add_layer(&mut self, name: &str) -> usize {
        let idx = self.layers.len();
        self.layers.push(DraftLayer::new(name));
        idx
    }

    pub fn remove_layer(&mut self, index: usize) -> KernelResult<()> {
        if self.layers.len() <= 1 {
            return Err(KernelError::InvalidArgument(
                "cannot remove the last layer".into(),
            ));
        }
        if index >= self.layers.len() {
            return Err(KernelError::InvalidArgument(
                "layer index out of range".into(),
            ));
        }
        self.layers.remove(index);
        if self.active_layer >= self.layers.len() {
            self.active_layer = self.layers.len() - 1;
        }
        Ok(())
    }

    pub fn set_active(&mut self, index: usize) -> KernelResult<()> {
        if index >= self.layers.len() {
            return Err(KernelError::InvalidArgument(
                "layer index out of range".into(),
            ));
        }
        self.active_layer = index;
        Ok(())
    }

    pub fn toggle_visibility(&mut self, index: usize) -> KernelResult<()> {
        let layer = self
            .layers
            .get_mut(index)
            .ok_or(KernelError::InvalidArgument("layer index out of range".into()))?;
        layer.visible = !layer.visible;
        Ok(())
    }

    pub fn get_active(&self) -> &DraftLayer {
        &self.layers[self.active_layer]
    }
}

impl Default for LayerManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Working Plane
// ---------------------------------------------------------------------------

/// A reference plane for drafting operations.
#[derive(Debug, Clone)]
pub struct WorkingPlane {
    pub origin: Point3,
    pub normal: Vec3,
    pub x_dir: Vec3,
}

impl WorkingPlane {
    pub fn new(origin: Point3, normal: Vec3, x_dir: Vec3) -> Self {
        Self { origin, normal, x_dir }
    }

    /// XY plane at origin.
    pub fn set_to_xy(&mut self) {
        self.origin = Point3::ORIGIN;
        self.normal = Vec3::Z;
        self.x_dir = Vec3::X;
    }

    /// XZ plane at origin.
    pub fn set_to_xz(&mut self) {
        self.origin = Point3::ORIGIN;
        self.normal = Vec3::Y;
        self.x_dir = Vec3::X;
    }

    /// YZ plane at origin.
    pub fn set_to_yz(&mut self) {
        self.origin = Point3::ORIGIN;
        self.normal = Vec3::X;
        self.x_dir = Vec3::Y;
    }

    /// Set to a face plane.
    pub fn set_to_face(&mut self, point: Point3, normal: Vec3) {
        self.origin = point;
        self.normal = normal.normalized().unwrap_or(Vec3::Z);
        let up = if self.normal.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
        self.x_dir = Vec3::new(
            up.y * self.normal.z - up.z * self.normal.y,
            up.z * self.normal.x - up.x * self.normal.z,
            up.x * self.normal.y - up.y * self.normal.x,
        ).normalized().unwrap_or(Vec3::X);
    }

    /// Align to the current camera view.
    pub fn align_to_view(&mut self, yaw: f64, pitch: f64) {
        let cy = yaw.cos();
        let sy = yaw.sin();
        let cp = pitch.cos();
        let sp = pitch.sin();
        self.normal = Vec3::new(sy * cp, -sp, cy * cp);
        self.x_dir = Vec3::new(cy, 0.0, -sy);
    }

    /// Projects a 3D point onto the working plane and returns 2D coordinates.
    pub fn project_point(&self, p: Point3) -> cadkernel_math::Point2 {
        let d = Vec3::new(
            p.x - self.origin.x,
            p.y - self.origin.y,
            p.z - self.origin.z,
        );
        let y_dir = Vec3::new(
            self.normal.y * self.x_dir.z - self.normal.z * self.x_dir.y,
            self.normal.z * self.x_dir.x - self.normal.x * self.x_dir.z,
            self.normal.x * self.x_dir.y - self.normal.y * self.x_dir.x,
        );
        let u = d.dot(self.x_dir);
        let v = d.dot(y_dir);
        cadkernel_math::Point2::new(u, v)
    }

    /// Unprojects a 2D point back into 3D world coordinates.
    pub fn unproject_point(&self, p: cadkernel_math::Point2) -> Point3 {
        let y_dir = Vec3::new(
            self.normal.y * self.x_dir.z - self.normal.z * self.x_dir.y,
            self.normal.z * self.x_dir.x - self.normal.x * self.x_dir.z,
            self.normal.x * self.x_dir.y - self.normal.y * self.x_dir.x,
        );
        Point3::new(
            self.origin.x + self.x_dir.x * p.x + y_dir.x * p.y,
            self.origin.y + self.x_dir.y * p.x + y_dir.y * p.y,
            self.origin.z + self.x_dir.z * p.x + y_dir.z * p.y,
        )
    }
}

impl Default for WorkingPlane {
    fn default() -> Self {
        Self {
            origin: Point3::ORIGIN,
            normal: Vec3::Z,
            x_dir: Vec3::X,
        }
    }
}

// ---------------------------------------------------------------------------
// Draft Styles
// ---------------------------------------------------------------------------

/// Arrow head style for dimension lines.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArrowStyle {
    Dot,
    Arrow,
    Tick,
    Circle,
    None,
}

/// Visual style for draft annotations.
#[derive(Debug, Clone)]
pub struct DraftStyle {
    pub line_color: (f64, f64, f64),
    pub line_width: f64,
    pub shape_color: (f64, f64, f64),
    pub font_size: f64,
    pub arrow_style: ArrowStyle,
    pub text_alignment: String,
}

impl Default for DraftStyle {
    fn default() -> Self {
        Self {
            line_color: (0.0, 0.0, 0.0),
            line_width: 1.0,
            shape_color: (0.8, 0.8, 0.8),
            font_size: 12.0,
            arrow_style: ArrowStyle::Arrow,
            text_alignment: "center".into(),
        }
    }
}

/// Manages named draft styles.
#[derive(Debug, Clone)]
pub struct DraftStyleManager {
    pub styles: std::collections::HashMap<String, DraftStyle>,
    pub active: String,
}

impl DraftStyleManager {
    pub fn new() -> Self {
        let mut styles = std::collections::HashMap::new();
        styles.insert("Standard".to_string(), DraftStyle::default());
        styles.insert(
            "Thin".to_string(),
            DraftStyle {
                line_width: 0.5,
                ..Default::default()
            },
        );
        styles.insert(
            "Thick".to_string(),
            DraftStyle {
                line_width: 2.0,
                ..Default::default()
            },
        );
        Self {
            styles,
            active: "Standard".into(),
        }
    }

    pub fn get_active(&self) -> &DraftStyle {
        self.styles.get(&self.active).unwrap_or_else(|| {
            self.styles.values().next().unwrap()
        })
    }

    pub fn set_active(&mut self, name: &str) -> KernelResult<()> {
        if !self.styles.contains_key(name) {
            return Err(KernelError::InvalidArgument(
                format!("style '{}' not found", name),
            ));
        }
        self.active = name.into();
        Ok(())
    }

    pub fn add_style(&mut self, name: &str, style: DraftStyle) {
        self.styles.insert(name.into(), style);
    }
}

impl Default for DraftStyleManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Wire/BSpline conversion (model-level)
// ---------------------------------------------------------------------------

/// Creates a face from a closed wire by closing the polyline if needed.
///
/// Returns a solid containing a single face whose outer loop follows the wire.
pub fn upgrade_wire_model(model: &mut BRepModel, points: &[Point3]) -> KernelResult<Handle<SolidData>> {
    use cadkernel_topology::{EntityKind, Tag};

    if points.len() < 3 {
        return Err(KernelError::InvalidArgument(
            "upgrade_wire requires at least 3 points".into(),
        ));
    }

    let op = model.history.next_operation("upgrade_wire");
    let closed = upgrade_wire(points);
    let n = closed.len() - 1; // remove closing point for vertex count

    let mut vertices = Vec::with_capacity(n);
    for (i, &pt) in closed.iter().take(n).enumerate() {
        let tag = Tag::generated(EntityKind::Vertex, op, i as u32);
        vertices.push(model.add_vertex_tagged(pt, tag));
    }

    let mut half_edges = Vec::with_capacity(n);
    for i in 0..n {
        let j = (i + 1) % n;
        let tag = Tag::generated(EntityKind::Edge, op, i as u32);
        let (_e, he, _) = model.add_edge_tagged(vertices[i], vertices[j], tag);
        half_edges.push(he);
    }

    let loop_h = model.make_loop(&half_edges)?;
    let tag_f = Tag::generated(EntityKind::Face, op, 0);
    let face_h = model.make_face_tagged(loop_h, tag_f);
    let tag_sh = Tag::generated(EntityKind::Shell, op, 0);
    let shell_h = model.make_shell_tagged(&[face_h], tag_sh);
    let tag_so = Tag::generated(EntityKind::Solid, op, 0);
    Ok(model.make_solid_tagged(&[shell_h], tag_so))
}

/// Decomposes a solid into individual face solids.
///
/// Returns one single-face solid per face in the original solid.
pub fn downgrade_solid_faces(model: &mut BRepModel, solid: Handle<SolidData>) -> KernelResult<Vec<Handle<SolidData>>> {
    use crate::features::copy_utils::collect_solid_faces;

    let faces = collect_solid_faces(model, solid)?;
    let mut result = Vec::with_capacity(faces.len());

    for (fi, &face_h) in faces.iter().enumerate() {
        let op = model.history.next_operation("downgrade_solid_face");
        let verts = model.vertices_of_face(face_h)?;

        let mut new_verts = Vec::with_capacity(verts.len());
        for (vi, &vh) in verts.iter().enumerate() {
            let pt = model.vertices.get(vh)
                .ok_or(KernelError::InvalidHandle("vertex"))?.point;
            let tag = cadkernel_topology::Tag::generated(
                cadkernel_topology::EntityKind::Vertex, op, vi as u32,
            );
            new_verts.push(model.add_vertex_tagged(pt, tag));
        }

        let n = new_verts.len();
        let mut half_edges = Vec::with_capacity(n);
        for j in 0..n {
            let tag = cadkernel_topology::Tag::generated(
                cadkernel_topology::EntityKind::Edge, op, j as u32,
            );
            let (_e, he, _) = model.add_edge_tagged(
                new_verts[j],
                new_verts[(j + 1) % n],
                tag,
            );
            half_edges.push(he);
        }

        let loop_h = model.make_loop(&half_edges)?;
        let tag_f = cadkernel_topology::Tag::generated(
            cadkernel_topology::EntityKind::Face, op, fi as u32,
        );
        let new_face = model.make_face_tagged(loop_h, tag_f);
        let tag_sh = cadkernel_topology::Tag::generated(
            cadkernel_topology::EntityKind::Shell, op, 0,
        );
        let shell_h = model.make_shell_tagged(&[new_face], tag_sh);
        let tag_so = cadkernel_topology::Tag::generated(
            cadkernel_topology::EntityKind::Solid, op, 0,
        );
        result.push(model.make_solid_tagged(&[shell_h], tag_so));
    }

    Ok(result)
}

/// Converts a polyline wire (in a BRepModel) to a B-spline curve and stores it.
pub fn wire_to_bspline_convert(_model: &mut BRepModel, points: &[Point3], degree: usize) -> KernelResult<NurbsCurve> {
    wire_to_bspline(points, degree)
}

/// Converts a B-spline curve to a polyline wire stored in the model.
pub fn bspline_to_wire_convert(
    model: &mut BRepModel,
    curve: &NurbsCurve,
    segments: usize,
) -> KernelResult<WireResult> {
    let points = bspline_to_wire(curve, segments)?;
    make_wire(model, &points)
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
    fn test_make_circle_wire_x_axis_normal_no_panic() {
        // Regression: with normal=X axis, the internal axis-selection branch
        // (n.x.abs() >= 0.9) is taken; ensure no panic on the cross-product
        // normalization fallback path.
        let pts = make_circle_wire(Point3::ORIGIN, Vec3::new(1.0, 0.0, 0.0), 1.0, 8).unwrap();
        assert_eq!(pts.len(), 9);
        let zero_normal = make_circle_wire(Point3::ORIGIN, Vec3::ZERO, 1.0, 8);
        assert!(zero_normal.is_err());
    }

    #[test]
    fn test_snap_helpers_nan_input_no_panic() {
        // Regression: NaN in query coords previously hit partial_cmp().unwrap().
        let wire = vec![Point3::ORIGIN, Point3::new(1.0, 0.0, 0.0)];
        let nan_query = Point3::new(f64::NAN, 0.0, 0.0);
        // All three snap helpers must return Some without panicking.
        assert!(snap_to_endpoint(&wire, nan_query).is_some());
        assert!(snap_to_midpoint(&wire, nan_query).is_some());
        assert!(snap_to_nearest(&wire, nan_query).is_some());
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

    #[test]
    fn test_draft_hatch_lines() {
        let boundary = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(10.0, 0.0, 0.0),
            Point3::new(10.0, 10.0, 0.0),
            Point3::new(0.0, 10.0, 0.0),
        ];
        let result = draft_hatch(
            &boundary,
            HatchPattern::Lines {
                angle: 0.0,
                spacing: 2.0,
            },
            1.0,
        )
        .unwrap();
        assert!(!result.lines.is_empty());
    }

    #[test]
    fn test_draft_hatch_cross() {
        let boundary = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(5.0, 0.0, 0.0),
            Point3::new(5.0, 5.0, 0.0),
        ];
        let result = draft_hatch(
            &boundary,
            HatchPattern::Cross {
                angle: 0.785,
                spacing: 1.0,
            },
            1.0,
        )
        .unwrap();
        assert!(!result.lines.is_empty());
    }

    #[test]
    fn test_draft_hatch_invalid() {
        let boundary = vec![Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)];
        assert!(draft_hatch(
            &boundary,
            HatchPattern::Lines {
                angle: 0.0,
                spacing: 1.0,
            },
            1.0,
        )
        .is_err());
    }

    #[test]
    fn test_make_facebinder() {
        let mut model = BRepModel::new();
        let bx = make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();
        let face_handles: Vec<_> = bx.faces.iter().take(2).cloned().collect();
        let solid_h = make_facebinder(&mut model, &face_handles).unwrap();
        let solid = model.solids.get(solid_h).unwrap();
        assert!(!solid.shells.is_empty());
    }

    #[test]
    fn test_make_facebinder_empty() {
        let mut model = BRepModel::new();
        assert!(make_facebinder(&mut model, &[]).is_err());
    }

    #[test]
    fn test_draft_dimension_full_linear() {
        let dim = make_draft_dimension_full(
            DraftDimensionType::Linear,
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(3.0, 4.0, 0.0),
            1.0,
        );
        assert!((dim.value - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_draft_dimension_full_diameter() {
        let dim = make_draft_dimension_full(
            DraftDimensionType::Diameter,
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(5.0, 0.0, 0.0),
            1.0,
        );
        assert!((dim.value - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_make_label_full() {
        let label = make_label_full(
            "Test Label",
            Point3::new(1.0, 2.0, 0.0),
            Some(Point3::ORIGIN),
            AnnotationStyle::default(),
        );
        assert_eq!(label.text, "Test Label");
        assert!(label.leader_target.is_some());
        assert!((label.style.font_size - 12.0).abs() < 1e-10);
    }

    #[test]
    fn test_annotation_style_default() {
        let style = AnnotationStyle::default();
        assert!((style.font_size - 12.0).abs() < 1e-10);
        assert!((style.line_width - 1.0).abs() < 1e-10);
        assert!((style.arrow_size - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_snap_to_point_endpoint() {
        let wire = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(5.0, 0.0, 0.0),
        ];
        let result = snap_to_point(&wire, Point3::new(4.9, 0.1, 0.0), SnapMode::Endpoint).unwrap();
        assert!((result.point.x - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_snap_to_point_grid() {
        let wire = vec![Point3::new(0.0, 0.0, 0.0)];
        let result = snap_to_point(&wire, Point3::new(1.3, 2.7, 0.0), SnapMode::Grid).unwrap();
        assert!((result.point.x - 1.0).abs() < 1e-10);
        assert!((result.point.y - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_snap_to_point_center() {
        let wire = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(10.0, 0.0, 0.0),
        ];
        let result = snap_to_point(&wire, Point3::ORIGIN, SnapMode::Center).unwrap();
        assert!((result.point.x - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_snap_lock() {
        let p = snap_lock(Point3::new(3.0, 4.0, 5.0), Vec3::X);
        assert!((p.x - 3.0).abs() < 1e-10);
        assert!((p.y).abs() < 1e-10);
        assert!((p.z).abs() < 1e-10);
    }

    #[test]
    fn test_trimex_draft_extend() {
        let pts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(5.0, 0.0, 0.0),
        ];
        let result = trimex_draft(&pts, Point3::new(10.0, 0.0, 0.0)).unwrap();
        assert_eq!(result.len(), 3);
        assert!((result[2].x - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_trimex_draft_trim() {
        let pts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(5.0, 0.0, 0.0),
            Point3::new(10.0, 0.0, 0.0),
        ];
        let result = trimex_draft(&pts, Point3::new(3.0, 0.5, 0.0)).unwrap();
        assert!(result.len() <= 3);
    }

    #[test]
    fn test_trimex_draft_invalid() {
        assert!(trimex_draft(&[Point3::ORIGIN], Point3::new(1.0, 0.0, 0.0)).is_err());
    }

    #[test]
    fn test_circular_array() {
        let mut model = BRepModel::new();
        let bx = make_box(&mut model, Point3::new(3.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap();
        let solids = circular_array(
            &mut model,
            bx.solid,
            Point3::ORIGIN,
            Vec3::Z,
            4,
            std::f64::consts::TAU,
        )
        .unwrap();
        assert_eq!(solids.len(), 4);
    }

    #[test]
    fn test_circular_array_half() {
        let mut model = BRepModel::new();
        let bx = make_box(&mut model, Point3::new(3.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap();
        let solids = circular_array(
            &mut model,
            bx.solid,
            Point3::ORIGIN,
            Vec3::Z,
            3,
            std::f64::consts::PI,
        )
        .unwrap();
        assert_eq!(solids.len(), 3);
    }

    #[test]
    fn test_circular_array_invalid() {
        let mut model = BRepModel::new();
        let bx = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        assert!(circular_array(&mut model, bx.solid, Point3::ORIGIN, Vec3::Z, 1, 1.0).is_err());
        assert!(circular_array(&mut model, bx.solid, Point3::ORIGIN, Vec3::Z, 2, 0.0).is_err());
    }

    #[test]
    fn test_path_link_array() {
        let mut model = BRepModel::new();
        let bx = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let path = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(3.0, 0.0, 0.0),
            Point3::new(6.0, 0.0, 0.0),
        ];
        let solids = path_link_array(&mut model, bx.solid, &path).unwrap();
        assert_eq!(solids.len(), 3);
    }

    #[test]
    fn test_path_link_array_invalid() {
        let mut model = BRepModel::new();
        let bx = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        assert!(path_link_array(&mut model, bx.solid, &[Point3::ORIGIN]).is_err());
    }

    #[test]
    fn test_point_link_array() {
        let mut model = BRepModel::new();
        let bx = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let pts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(5.0, 5.0, 0.0),
            Point3::new(-5.0, 5.0, 0.0),
        ];
        let solids = point_link_array(&mut model, bx.solid, &pts).unwrap();
        assert_eq!(solids.len(), 3);
    }

    #[test]
    fn test_edit_draft() {
        let mut pts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        ];
        edit_draft(&mut pts, 1, Point3::new(1.0, 5.0, 0.0)).unwrap();
        assert!((pts[1].y - 5.0).abs() < 1e-12);
        assert!(edit_draft(&mut pts, 10, Point3::ORIGIN).is_err());
    }

    #[test]
    fn test_draft_to_sketch() {
        let pts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
        ];
        let sketch = draft_to_sketch(&pts).unwrap();
        assert_eq!(sketch.points.len(), 3);
        assert_eq!(sketch.lines.len(), 2);
    }

    #[test]
    fn test_draft_to_sketch_invalid() {
        assert!(draft_to_sketch(&[Point3::ORIGIN]).is_err());
    }

    #[test]
    fn test_make_cubic_bezier_wire() {
        let pts = make_cubic_bezier_wire(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 3.0, 0.0),
            Point3::new(3.0, 3.0, 0.0),
            Point3::new(4.0, 0.0, 0.0),
            16,
        )
        .unwrap();
        assert_eq!(pts.len(), 17);
        assert!((pts[0].x).abs() < 1e-12);
        assert!((pts[16].x - 4.0).abs() < 1e-12);
    }

    #[test]
    fn test_shape_from_text() {
        let strokes = shape_from_text("AB", Point3::ORIGIN, 10.0, Vec3::Z).unwrap();
        assert!(!strokes.is_empty());
        // Each stroke should have at least 2 points
        for s in &strokes {
            assert!(s.len() >= 2);
        }
    }

    #[test]
    fn test_shape_from_text_empty() {
        assert!(shape_from_text("", Point3::ORIGIN, 10.0, Vec3::Z).is_err());
    }

    #[test]
    fn test_shape_from_text_invalid_height() {
        assert!(shape_from_text("A", Point3::ORIGIN, 0.0, Vec3::Z).is_err());
    }

    #[test]
    fn test_shape_from_text_digits() {
        let strokes = shape_from_text("123", Point3::ORIGIN, 5.0, Vec3::Z).unwrap();
        assert!(!strokes.is_empty());
    }

    #[test]
    fn test_snap_to_center() {
        let wire = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(10.0, 0.0, 0.0),
        ];
        let result = snap_to_center(&wire, Point3::ORIGIN).unwrap();
        assert!((result.point.x - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_snap_to_angle() {
        let wire = vec![Point3::new(0.0, 0.0, 0.0)];
        let result = snap_to_angle(&wire, Point3::new(5.0, 0.1, 0.0)).unwrap();
        // Should snap to 0 degrees (along X axis)
        assert!(result.point.y.abs() < 1e-10);
    }

    #[test]
    fn test_snap_to_intersection() {
        // X-shaped wire: (0,0)→(10,10)→(10,0)→(0,10)
        let wire = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(10.0, 10.0, 0.0),
            Point3::new(10.0, 0.0, 0.0),
            Point3::new(0.0, 10.0, 0.0),
        ];
        let result = snap_to_intersection(&wire, Point3::new(5.0, 5.0, 0.0));
        assert!(result.is_some());
        let r = result.unwrap();
        assert!((r.point.x - 5.0).abs() < 1e-10);
        assert!((r.point.y - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_snap_to_perpendicular() {
        let wire = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(10.0, 0.0, 0.0),
        ];
        let result = snap_to_perpendicular(&wire, Point3::new(5.0, 3.0, 0.0)).unwrap();
        assert!((result.point.x - 5.0).abs() < 1e-10);
        assert!((result.point.y).abs() < 1e-10);
    }

    #[test]
    fn test_snap_to_extension() {
        let wire = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(5.0, 0.0, 0.0),
        ];
        let result = snap_to_extension(&wire, Point3::new(10.0, 1.0, 0.0)).unwrap();
        assert!((result.point.x - 10.0).abs() < 1e-10);
        assert!((result.point.y).abs() < 1e-10);
    }

    #[test]
    fn test_snap_to_parallel() {
        let wire = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(10.0, 0.0, 0.0),
        ];
        let result = snap_to_parallel(&wire, Point3::new(3.0, 5.0, 0.0)).unwrap();
        assert!((result.point.x - 3.0).abs() < 1e-10);
        assert!((result.point.y).abs() < 1e-10);
    }

    #[test]
    fn test_snap_to_special() {
        let wire = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(10.0, 0.0, 0.0),
        ];
        let result = snap_to_special(&wire, Point3::new(0.1, 0.0, 0.0)).unwrap();
        // Should pick endpoint (0,0,0) as closest
        assert!((result.point.x).abs() < 1e-10);
    }

    #[test]
    fn test_snap_to_grid() {
        let result = snap_to_grid(Point3::new(1.3, 2.7, 0.4), 1.0);
        assert!((result.point.x - 1.0).abs() < 1e-10);
        assert!((result.point.y - 3.0).abs() < 1e-10);
        assert!((result.point.z).abs() < 1e-10);
    }

    #[test]
    fn test_snap_to_grid_custom_spacing() {
        let result = snap_to_grid(Point3::new(1.3, 2.7, 0.0), 0.5);
        assert!((result.point.x - 1.5).abs() < 1e-10);
        assert!((result.point.y - 2.5).abs() < 1e-10);
    }

    #[test]
    fn test_snap_to_working_plane() {
        let result = snap_to_working_plane(
            Point3::new(1.0, 2.0, 5.0),
            Point3::ORIGIN,
            Vec3::Z,
        );
        assert!((result.point.x - 1.0).abs() < 1e-10);
        assert!((result.point.y - 2.0).abs() < 1e-10);
        assert!((result.point.z).abs() < 1e-10);
    }

    #[test]
    fn test_snap_to_dimensions() {
        let result = snap_to_dimensions(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(10.0, 0.0, 0.0),
            Point3::new(4.8, 0.1, 0.0),
        );
        // Should snap to midpoint (5,0,0) as closest
        assert!((result.point.x - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_snap_to_point_angle_mode() {
        let wire = vec![Point3::new(0.0, 0.0, 0.0)];
        let result = snap_to_point(&wire, Point3::new(5.0, 0.01, 0.0), SnapMode::Angle).unwrap();
        assert!(result.point.y.abs() < 1e-10);
    }

    #[test]
    fn test_snap_to_point_extension_mode() {
        let wire = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(5.0, 0.0, 0.0),
        ];
        let result = snap_to_point(&wire, Point3::new(10.0, 1.0, 0.0), SnapMode::Extension).unwrap();
        assert!((result.point.x - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_snap_to_point_working_plane_mode() {
        let wire = vec![Point3::ORIGIN];
        let result = snap_to_point(&wire, Point3::new(1.0, 2.0, 5.0), SnapMode::WorkingPlane).unwrap();
        assert!((result.point.z).abs() < 1e-10);
    }

    #[test]
    fn test_layer_manager_new() {
        let lm = LayerManager::new();
        assert_eq!(lm.layers.len(), 1);
        assert_eq!(lm.active_layer, 0);
        assert_eq!(lm.get_active().name, "Default");
    }

    #[test]
    fn test_layer_manager_add_remove() {
        let mut lm = LayerManager::new();
        let idx = lm.add_layer("Layer 1");
        assert_eq!(idx, 1);
        assert_eq!(lm.layers.len(), 2);

        lm.set_active(1).unwrap();
        assert_eq!(lm.active_layer, 1);

        lm.remove_layer(0).unwrap();
        assert_eq!(lm.layers.len(), 1);
        assert_eq!(lm.active_layer, 0);
    }

    #[test]
    fn test_layer_manager_cannot_remove_last() {
        let mut lm = LayerManager::new();
        assert!(lm.remove_layer(0).is_err());
    }

    #[test]
    fn test_layer_toggle_visibility() {
        let mut lm = LayerManager::new();
        assert!(lm.layers[0].visible);
        lm.toggle_visibility(0).unwrap();
        assert!(!lm.layers[0].visible);
    }

    #[test]
    fn test_working_plane_xy() {
        let mut wp = WorkingPlane::default();
        wp.set_to_xy();
        assert!((wp.normal.z - 1.0).abs() < 1e-10);
        assert!((wp.x_dir.x - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_working_plane_project_unproject() {
        let wp = WorkingPlane::default();
        let p3 = Point3::new(3.0, 5.0, 0.0);
        let p2 = wp.project_point(p3);
        assert!((p2.x - 3.0).abs() < 1e-10);
        assert!((p2.y - 5.0).abs() < 1e-10);

        let back = wp.unproject_point(p2);
        assert!((back.x - 3.0).abs() < 1e-10);
        assert!((back.y - 5.0).abs() < 1e-10);
        assert!((back.z).abs() < 1e-10);
    }

    #[test]
    fn test_working_plane_xz() {
        let mut wp = WorkingPlane::default();
        wp.set_to_xz();
        assert!((wp.normal.y - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_working_plane_yz() {
        let mut wp = WorkingPlane::default();
        wp.set_to_yz();
        assert!((wp.normal.x - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_working_plane_set_to_face() {
        let mut wp = WorkingPlane::default();
        wp.set_to_face(Point3::new(1.0, 2.0, 3.0), Vec3::Y);
        assert!((wp.normal.y - 1.0).abs() < 1e-10);
        assert!((wp.origin.x - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_working_plane_align_to_view() {
        let mut wp = WorkingPlane::default();
        wp.align_to_view(0.0, 0.0);
        assert!((wp.normal.z - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_draft_style_manager() {
        let mut sm = DraftStyleManager::new();
        assert_eq!(sm.styles.len(), 3);
        assert_eq!(sm.active, "Standard");

        sm.set_active("Thin").unwrap();
        assert!((sm.get_active().line_width - 0.5).abs() < 1e-10);

        assert!(sm.set_active("NonExistent").is_err());
    }

    #[test]
    fn test_draft_style_add() {
        let mut sm = DraftStyleManager::new();
        sm.add_style("Custom", DraftStyle {
            line_width: 3.0,
            arrow_style: ArrowStyle::Tick,
            ..Default::default()
        });
        sm.set_active("Custom").unwrap();
        assert!((sm.get_active().line_width - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_arrow_style_variants() {
        assert_ne!(ArrowStyle::Dot, ArrowStyle::Arrow);
        assert_ne!(ArrowStyle::Tick, ArrowStyle::Circle);
        assert_ne!(ArrowStyle::None, ArrowStyle::Dot);
    }

    #[test]
    fn test_upgrade_wire_model() {
        let mut model = BRepModel::new();
        let pts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
        ];
        let solid = upgrade_wire_model(&mut model, &pts).unwrap();
        let sd = model.solids.get(solid).unwrap();
        assert_eq!(sd.shells.len(), 1);
    }

    #[test]
    fn test_downgrade_solid_faces() {
        let mut model = BRepModel::new();
        let b = make_box(&mut model, Point3::ORIGIN, 2.0, 3.0, 4.0).unwrap();
        let face_solids = downgrade_solid_faces(&mut model, b.solid).unwrap();
        assert_eq!(face_solids.len(), 6);
    }

    #[test]
    fn test_wire_to_bspline_convert() {
        let mut model = BRepModel::new();
        let pts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(2.0, 1.0, 0.0),
            Point3::new(3.0, 0.0, 0.0),
        ];
        let curve = wire_to_bspline_convert(&mut model, &pts, 2).unwrap();
        let wire = bspline_to_wire_convert(&mut model, &curve, 10).unwrap();
        assert!(wire.vertices.len() > 3);
    }

    #[test]
    fn test_make_line_draft() {
        let mut model = BRepModel::new();
        let p1 = Point3::new(0.0, 0.0, 0.0);
        let p2 = Point3::new(5.0, 3.0, 1.0);
        let result = make_line_draft(&mut model, p1, p2).unwrap();
        assert_eq!(result.vertices.len(), 2);
        assert_eq!(result.edges.len(), 1);
    }
}
