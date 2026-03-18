//! A closed chain of 2D parametric curves forming a trim boundary in UV space.

use std::sync::Arc;

use cadkernel_math::Point2;

use crate::curve::curve2d::Curve2D;

const SAMPLES_PER_SEGMENT: usize = 32;

/// A closed wire (loop) of 2D parametric curves in UV parameter space.
///
/// Used to define trim boundaries on parametric surfaces. The outer boundary
/// is typically counter-clockwise, and hole boundaries are clockwise.
#[derive(Clone)]
pub struct ParametricWire2D {
    pub segments: Vec<Arc<dyn Curve2D>>,
    pub closed: bool,
}

impl ParametricWire2D {
    /// Creates a new parametric wire from curve segments.
    pub fn new(segments: Vec<Arc<dyn Curve2D>>, closed: bool) -> Self {
        Self { segments, closed }
    }

    /// Creates a closed wire from curve segments.
    pub fn closed(segments: Vec<Arc<dyn Curve2D>>) -> Self {
        Self::new(segments, true)
    }

    /// Tests whether a UV point `(u, v)` is inside this closed wire.
    ///
    /// Uses the winding number algorithm: the point is inside if the wire
    /// winds around it a non-zero number of times. More robust than the
    /// crossing number test for complex or self-intersecting loops.
    pub fn contains_point(&self, u: f64, v: f64) -> bool {
        if !self.closed || self.segments.is_empty() {
            return false;
        }
        let mut winding: i32 = 0;

        for curve in &self.segments {
            let (t0, t1) = curve.domain();
            for i in 0..SAMPLES_PER_SEGMENT {
                let ta = t0 + (t1 - t0) * i as f64 / SAMPLES_PER_SEGMENT as f64;
                let tb = t0 + (t1 - t0) * (i + 1) as f64 / SAMPLES_PER_SEGMENT as f64;
                let pa = curve.point_at(ta);
                let pb = curve.point_at(tb);

                if pa.y <= v {
                    if pb.y > v {
                        // Upward crossing — check if point is to the left of edge
                        let cross =
                            (pb.x - pa.x) * (v - pa.y) - (u - pa.x) * (pb.y - pa.y);
                        if cross > 0.0 {
                            winding += 1;
                        }
                    }
                } else if pb.y <= v {
                    // Downward crossing — check if point is to the right of edge
                    let cross =
                        (pb.x - pa.x) * (v - pa.y) - (u - pa.x) * (pb.y - pa.y);
                    if cross < 0.0 {
                        winding -= 1;
                    }
                }
            }
        }

        winding != 0
    }

    /// Samples uniformly spaced points along the wire.
    ///
    /// Returns `n` total points distributed proportionally across segments
    /// by approximate arc length.
    pub fn sample_points(&self, n: usize) -> Vec<Point2> {
        if self.segments.is_empty() || n == 0 {
            return Vec::new();
        }

        // Compute approximate arc length of each segment
        let lengths: Vec<f64> = self
            .segments
            .iter()
            .map(|seg| {
                let (t0, t1) = seg.domain();
                let steps = 32;
                let mut len = 0.0;
                let mut prev = seg.point_at(t0);
                for i in 1..=steps {
                    let t = t0 + (t1 - t0) * i as f64 / steps as f64;
                    let cur = seg.point_at(t);
                    len += prev.distance_to(cur);
                    prev = cur;
                }
                len
            })
            .collect();

        let total_length: f64 = lengths.iter().sum();
        if total_length < 1e-14 {
            return vec![self.segments[0].point_at(self.segments[0].domain().0); n];
        }

        let mut points = Vec::with_capacity(n);
        let mut remaining = n;

        for (i, seg) in self.segments.iter().enumerate() {
            let seg_n = if i == self.segments.len() - 1 {
                remaining
            } else {
                let frac = lengths[i] / total_length;
                let count = (frac * n as f64).round() as usize;
                count.min(remaining)
            };
            remaining = remaining.saturating_sub(seg_n);

            let (t0, t1) = seg.domain();
            for j in 0..seg_n {
                let t = t0 + (t1 - t0) * j as f64 / seg_n.max(1) as f64;
                points.push(seg.point_at(t));
            }
        }

        points
    }

    /// Returns a flat list of sampled polygon vertices (one polyline for the wire).
    pub fn to_polyline(&self, samples_per_segment: usize) -> Vec<Point2> {
        let mut pts = Vec::new();
        for seg in &self.segments {
            let (t0, t1) = seg.domain();
            let n = samples_per_segment.max(2);
            for i in 0..n {
                let t = t0 + (t1 - t0) * i as f64 / n as f64;
                pts.push(seg.point_at(t));
            }
        }
        // Close the polyline by adding the first point
        if self.closed && !pts.is_empty() {
            pts.push(pts[0]);
        }
        pts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::curve2d::{Circle2D, Line2D};

    fn square_wire() -> ParametricWire2D {
        let segs: Vec<Arc<dyn Curve2D>> = vec![
            Arc::new(Line2D::new(Point2::new(0.0, 0.0), Point2::new(1.0, 0.0))),
            Arc::new(Line2D::new(Point2::new(1.0, 0.0), Point2::new(1.0, 1.0))),
            Arc::new(Line2D::new(Point2::new(1.0, 1.0), Point2::new(0.0, 1.0))),
            Arc::new(Line2D::new(Point2::new(0.0, 1.0), Point2::new(0.0, 0.0))),
        ];
        ParametricWire2D::closed(segs)
    }

    #[test]
    fn test_square_contains() {
        let wire = square_wire();
        assert!(wire.contains_point(0.5, 0.5));
        assert!(wire.contains_point(0.1, 0.1));
        assert!(wire.contains_point(0.9, 0.9));
        assert!(!wire.contains_point(-0.1, 0.5));
        assert!(!wire.contains_point(0.5, 1.5));
        assert!(!wire.contains_point(2.0, 2.0));
    }

    #[test]
    fn test_triangle_contains() {
        let segs: Vec<Arc<dyn Curve2D>> = vec![
            Arc::new(Line2D::new(Point2::new(0.0, 0.0), Point2::new(1.0, 0.0))),
            Arc::new(Line2D::new(Point2::new(1.0, 0.0), Point2::new(0.5, 1.0))),
            Arc::new(Line2D::new(Point2::new(0.5, 1.0), Point2::new(0.0, 0.0))),
        ];
        let wire = ParametricWire2D::closed(segs);

        assert!(wire.contains_point(0.5, 0.3));
        assert!(!wire.contains_point(0.95, 0.9));
        assert!(!wire.contains_point(-0.1, 0.0));
    }

    #[test]
    fn test_circle_contains() {
        let segs: Vec<Arc<dyn Curve2D>> = vec![
            Arc::new(Circle2D::full(Point2::new(0.5, 0.5), 0.4)),
        ];
        let wire = ParametricWire2D::closed(segs);

        assert!(wire.contains_point(0.5, 0.5));
        assert!(wire.contains_point(0.5, 0.8));
        assert!(!wire.contains_point(0.0, 0.0));
        assert!(!wire.contains_point(1.0, 1.0));
    }

    #[test]
    fn test_open_wire_never_contains() {
        let segs: Vec<Arc<dyn Curve2D>> = vec![
            Arc::new(Line2D::new(Point2::new(0.0, 0.0), Point2::new(1.0, 0.0))),
            Arc::new(Line2D::new(Point2::new(1.0, 0.0), Point2::new(1.0, 1.0))),
        ];
        let wire = ParametricWire2D::new(segs, false);
        assert!(!wire.contains_point(0.5, 0.5));
    }

    #[test]
    fn test_sample_points_count() {
        let wire = square_wire();
        let pts = wire.sample_points(40);
        assert_eq!(pts.len(), 40);
    }

    #[test]
    fn test_to_polyline() {
        let wire = square_wire();
        let poly = wire.to_polyline(4);
        // 4 segments × 4 samples + 1 closing = 17
        assert_eq!(poly.len(), 17);
        assert!(poly.first().unwrap().distance_to(*poly.last().unwrap()) < 1e-10);
    }
}
