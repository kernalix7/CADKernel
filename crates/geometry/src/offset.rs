//! 2D curve offset operations for polylines and polygons.
//!
//! Provides parallel offset of open polylines ([`offset_polyline_2d`]) and
//! closed polygons ([`offset_polygon_2d`]).

use cadkernel_math::Point2;

/// Offsets a closed 2D polygon by `distance`.
///
/// Positive `distance` expands the polygon outward (for a CCW-wound polygon),
/// negative `distance` shrinks it inward.
///
/// Uses edge-normal offset with miter-join at corners. Self-intersections
/// from large offsets are **not** clipped — caller should validate.
pub fn offset_polygon_2d(polygon: &[Point2], distance: f64) -> Vec<Point2> {
    if polygon.len() < 3 || distance.abs() < 1e-14 {
        return polygon.to_vec();
    }
    let n = polygon.len();
    let mut result = Vec::with_capacity(n);

    for i in 0..n {
        let prev = polygon[(i + n - 1) % n];
        let curr = polygon[i];
        let next = polygon[(i + 1) % n];
        // For CCW polygon: outward = right normal of edge direction.
        result.push(miter_offset_with(prev, curr, next, distance, edge_right_normal));
    }

    result
}

/// Offsets an open 2D polyline by `distance`.
///
/// Positive `distance` offsets to the left of the polyline direction,
/// negative to the right.
///
/// End caps use simple normal offset (no rounding).
pub fn offset_polyline_2d(polyline: &[Point2], distance: f64) -> Vec<Point2> {
    if polyline.len() < 2 || distance.abs() < 1e-14 {
        return polyline.to_vec();
    }
    let n = polyline.len();
    let mut result = Vec::with_capacity(n);

    for i in 0..n {
        if i == 0 {
            let normal = edge_left_normal(polyline[0], polyline[1]);
            result.push(Point2::new(
                polyline[0].x + normal.0 * distance,
                polyline[0].y + normal.1 * distance,
            ));
        } else if i == n - 1 {
            let normal = edge_left_normal(polyline[n - 2], polyline[n - 1]);
            result.push(Point2::new(
                polyline[n - 1].x + normal.0 * distance,
                polyline[n - 1].y + normal.1 * distance,
            ));
        } else {
            let prev = polyline[i - 1];
            let curr = polyline[i];
            let next = polyline[i + 1];
            result.push(miter_offset_with(prev, curr, next, distance, edge_left_normal));
        }
    }

    result
}

/// Computes the miter offset point at a vertex using a given normal function.
fn miter_offset_with(
    prev: Point2,
    curr: Point2,
    next: Point2,
    distance: f64,
    normal_fn: fn(Point2, Point2) -> (f64, f64),
) -> Point2 {
    let n1 = normal_fn(prev, curr);
    let n2 = normal_fn(curr, next);

    let nx = n1.0 + n2.0;
    let ny = n1.1 + n2.1;
    let len = (nx * nx + ny * ny).sqrt();

    if len < 1e-14 {
        return Point2::new(curr.x + n1.0 * distance, curr.y + n1.1 * distance);
    }

    let nx = nx / len;
    let ny = ny / len;

    let cos_half = nx * n1.0 + ny * n1.1;
    let miter_len = if cos_half.abs() < 1e-10 {
        distance
    } else {
        distance / cos_half
    };

    let miter_len = miter_len.clamp(-4.0 * distance.abs(), 4.0 * distance.abs());
    Point2::new(curr.x + nx * miter_len, curr.y + ny * miter_len)
}

/// Left normal of edge `a→b`: `(-dy, dx)` normalised.
fn edge_left_normal(a: Point2, b: Point2) -> (f64, f64) {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-14 {
        return (0.0, 0.0);
    }
    (-dy / len, dx / len)
}

/// Right normal of edge `a→b`: `(dy, -dx)` normalised.
/// This is the outward normal for a CCW polygon.
fn edge_right_normal(a: Point2, b: Point2) -> (f64, f64) {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-14 {
        return (0.0, 0.0);
    }
    (dy / len, -dx / len)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq_2d(a: Point2, b: Point2) -> bool {
        (a.x - b.x).abs() < 1e-8 && (a.y - b.y).abs() < 1e-8
    }

    #[test]
    fn test_offset_square_outward() {
        // CCW unit square.
        let square = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ];
        let expanded = offset_polygon_2d(&square, 1.0);
        assert_eq!(expanded.len(), 4);
        assert!(approx_eq_2d(expanded[0], Point2::new(-1.0, -1.0)));
        assert!(approx_eq_2d(expanded[1], Point2::new(2.0, -1.0)));
        assert!(approx_eq_2d(expanded[2], Point2::new(2.0, 2.0)));
        assert!(approx_eq_2d(expanded[3], Point2::new(-1.0, 2.0)));
    }

    #[test]
    fn test_offset_square_inward() {
        let square = vec![
            Point2::new(0.0, 0.0),
            Point2::new(4.0, 0.0),
            Point2::new(4.0, 4.0),
            Point2::new(0.0, 4.0),
        ];
        let shrunk = offset_polygon_2d(&square, -1.0);
        assert_eq!(shrunk.len(), 4);
        assert!(approx_eq_2d(shrunk[0], Point2::new(1.0, 1.0)));
        assert!(approx_eq_2d(shrunk[2], Point2::new(3.0, 3.0)));
    }

    #[test]
    fn test_offset_polyline() {
        let line = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(2.0, 0.0),
        ];
        let offset = offset_polyline_2d(&line, 1.0);
        assert_eq!(offset.len(), 3);
        // Left offset of a horizontal rightward line → y += 1.
        for pt in &offset {
            assert!((pt.y - 1.0).abs() < 1e-8);
        }
    }

    #[test]
    fn test_zero_distance_noop() {
        let square = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
        ];
        let result = offset_polygon_2d(&square, 0.0);
        assert_eq!(result.len(), 3);
        for (a, b) in result.iter().zip(square.iter()) {
            assert!(approx_eq_2d(*a, *b));
        }
    }
}
