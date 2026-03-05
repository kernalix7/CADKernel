use cadkernel_math::{Point3, Vec3};

/// A work plane in 3D space on which sketches are drawn.
#[derive(Debug, Clone, Copy)]
pub struct WorkPlane {
    pub origin: Point3,
    pub normal: Vec3,
    pub x_axis: Vec3,
    pub y_axis: Vec3,
}

impl WorkPlane {
    /// Creates a work plane from an origin, normal, and X-axis hint.
    /// The Y-axis is computed to form a right-handed frame.
    pub fn new(origin: Point3, normal: Vec3, x_axis: Vec3) -> Self {
        let n = normal.normalized().unwrap_or(Vec3::Z);
        let x = x_axis.normalized().unwrap_or(Vec3::X);
        let y = n.cross(x);
        Self {
            origin,
            normal: n,
            x_axis: x,
            y_axis: y,
        }
    }

    /// The standard XY plane at the world origin.
    pub fn xy() -> Self {
        Self {
            origin: Point3::ORIGIN,
            normal: Vec3::Z,
            x_axis: Vec3::X,
            y_axis: Vec3::Y,
        }
    }

    /// The XZ plane at the world origin (front view).
    pub fn xz() -> Self {
        Self {
            origin: Point3::ORIGIN,
            normal: Vec3::Y,
            x_axis: Vec3::X,
            y_axis: Vec3::Z,
        }
    }

    /// Maps a 2D sketch point to a 3D world point on this plane.
    pub fn to_world(&self, x: f64, y: f64) -> Point3 {
        self.origin + self.x_axis * x + self.y_axis * y
    }
}

use crate::Sketch;

/// Extracts the solved sketch point positions as a 3D polygon on the given
/// work plane. Returns the ordered list of points forming a closed profile.
///
/// Points are emitted in their storage order; the caller is responsible for
/// ensuring the sketch represents a single closed loop of lines.
pub fn extract_profile(sketch: &Sketch, plane: &WorkPlane) -> Vec<Point3> {
    if sketch.lines.is_empty() {
        return sketch
            .points
            .iter()
            .map(|p| plane.to_world(p.position.x, p.position.y))
            .collect();
    }

    let mut visited = vec![false; sketch.lines.len()];
    let mut ordered_points = Vec::new();

    // Start from the first line
    let first = &sketch.lines[0];
    visited[0] = true;
    ordered_points.push(first.start);
    ordered_points.push(first.end);

    // Chain lines by matching endpoints
    loop {
        let last = *ordered_points.last().unwrap();
        let mut found = false;
        for (i, line) in sketch.lines.iter().enumerate() {
            if visited[i] {
                continue;
            }
            if line.start == last {
                visited[i] = true;
                ordered_points.push(line.end);
                found = true;
                break;
            } else if line.end == last {
                visited[i] = true;
                ordered_points.push(line.start);
                found = true;
                break;
            }
        }
        if !found {
            break;
        }
    }

    // If the profile is closed, remove the duplicated last point
    if ordered_points.len() > 2 && ordered_points.first() == ordered_points.last() {
        ordered_points.pop();
    }

    ordered_points
        .iter()
        .map(|pid| {
            let p = &sketch.points[pid.0];
            plane.to_world(p.position.x, p.position.y)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Sketch;

    #[test]
    fn test_work_plane_xy() {
        let wp = WorkPlane::xy();
        let p = wp.to_world(1.0, 2.0);
        assert!((p.x - 1.0).abs() < 1e-10);
        assert!((p.y - 2.0).abs() < 1e-10);
        assert!(p.z.abs() < 1e-10);
    }

    #[test]
    fn test_extract_profile_square() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(1.0, 0.0);
        let p2 = sketch.add_point(1.0, 1.0);
        let p3 = sketch.add_point(0.0, 1.0);

        sketch.add_line(p0, p1);
        sketch.add_line(p1, p2);
        sketch.add_line(p2, p3);
        sketch.add_line(p3, p0);

        let wp = WorkPlane::xy();
        let profile = extract_profile(&sketch, &wp);
        assert_eq!(profile.len(), 4);
        assert!((profile[0].x - 0.0).abs() < 1e-10);
        assert!((profile[1].x - 1.0).abs() < 1e-10);
        assert!((profile[2].x - 1.0).abs() < 1e-10);
        assert!((profile[3].x - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_extract_profile_on_xz_plane() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(1.0, 0.0);
        let p2 = sketch.add_point(1.0, 1.0);
        sketch.add_line(p0, p1);
        sketch.add_line(p1, p2);

        let wp = WorkPlane::xz();
        let profile = extract_profile(&sketch, &wp);
        assert_eq!(profile.len(), 3);
        assert!(profile[0].y.abs() < 1e-10);
        assert!((profile[2].z - 1.0).abs() < 1e-10);
    }
}
