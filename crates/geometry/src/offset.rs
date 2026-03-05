//! 2D curve offset operations for polylines and polygons.
//!
//! Provides parallel offset of open polylines ([`offset_polyline_2d`]) and
//! closed polygons ([`offset_polygon_2d`]).  Positive distance offsets to the
//! left of the path (CCW exterior for a CCW polygon), negative to the right.

use cadkernel_math::Point2;

/// Offsets a closed 2D polygon by `distance`.
///
/// Positive `distance` expands the polygon outward (for a CCW-wound polygon),
/// negative `distance` shrinks it inward.
///
/// # Panics
///
/// Not yet implemented -- currently calls `todo!()`.
pub fn offset_polygon_2d(polygon: &[Point2], distance: f64) -> Vec<Point2> {
    let _ = (polygon, distance);
    todo!("offset_polygon_2d: not yet implemented")
}

/// Offsets an open 2D polyline by `distance`.
///
/// Positive `distance` offsets to the left of the polyline direction (CCW
/// convention), negative to the right.
///
/// # Panics
///
/// Not yet implemented -- currently calls `todo!()`.
pub fn offset_polyline_2d(polyline: &[Point2], distance: f64) -> Vec<Point2> {
    let _ = (polyline, distance);
    todo!("offset_polyline_2d: not yet implemented")
}
