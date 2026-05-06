use std::collections::{HashMap, HashSet};

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};

use crate::{LineId, PointId, Sketch, SketchLine};

/// A work plane in 3D space on which sketches are drawn.
///
/// Defines an origin, normal, and orthonormal X/Y axes. Used to map 2D
/// sketch coordinates to 3D world coordinates via
/// [`to_world`](Self::to_world) and back via [`to_local`](Self::to_local).
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
        let mut x = x_axis.normalized().unwrap_or(Vec3::X);
        // Gram-Schmidt: ensure x_axis is perpendicular to normal.
        x = (x - n * n.dot(x)).normalized().unwrap_or(Vec3::X);
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

    /// Projects a 3D world point onto this plane, returning the 2D local coordinates.
    pub fn to_local(&self, pt: Point3) -> cadkernel_math::Point2 {
        let d = pt - self.origin;
        cadkernel_math::Point2::new(d.dot(self.x_axis), d.dot(self.y_axis))
    }
}

/// Connectivity analysis for non-construction sketch profile lines.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SketchProfileAnalysis {
    /// Closed profile loops, each represented by ordered point ids.
    pub loops: Vec<Vec<PointId>>,
    /// Points that terminate an open profile chain.
    pub open_endpoints: Vec<PointId>,
    /// Points where more than two regular profile lines meet.
    pub branch_points: Vec<PointId>,
    /// Regular profile lines skipped because they reference missing points.
    pub invalid_line_indices: Vec<usize>,
    /// Number of non-construction lines considered for profile extraction.
    pub regular_line_count: usize,
    /// Number of construction lines ignored by the profile analyzer.
    pub construction_line_count: usize,
}

impl SketchProfileAnalysis {
    /// Returns true when the sketch contains exactly one usable closed profile.
    pub fn is_single_closed_profile(&self) -> bool {
        self.invalid_line_indices.is_empty()
            && self.open_endpoints.is_empty()
            && self.branch_points.is_empty()
            && self.loops.len() == 1
            && self.regular_line_count == self.loops[0].len()
            && self.loops[0].len() >= 3
    }

    /// Compact English status text suitable for UI banners and diagnostics.
    pub fn status_label(&self) -> String {
        if self.regular_line_count == 0 {
            return "Profile: no regular lines".to_string();
        }
        if !self.invalid_line_indices.is_empty() {
            return format!(
                "Profile: {} invalid line refs",
                self.invalid_line_indices.len()
            );
        }
        if !self.branch_points.is_empty() {
            return format!("Profile: {} branch points", self.branch_points.len());
        }
        if !self.open_endpoints.is_empty() {
            return format!("Profile: open ({} endpoints)", self.open_endpoints.len());
        }
        if self.loops.len() == 1 && self.regular_line_count == self.loops[0].len() {
            return format!("Profile: ready ({} edges)", self.loops[0].len());
        }
        if self.loops.is_empty() {
            "Profile: no closed loop".to_string()
        } else {
            format!("Profile: {} closed loops", self.loops.len())
        }
    }
}

/// Analyze regular (non-construction) sketch lines for closed profile loops.
///
/// Construction lines are ignored. The analyzer reports open endpoints and
/// branch points so the viewer can explain why Pad/Pocket/Groove cannot use a
/// sketch, instead of passing a partial open chain into the modeling kernel.
pub fn analyze_profiles(sketch: &Sketch) -> SketchProfileAnalysis {
    let construction_lines: HashSet<usize> = sketch
        .construction_lines
        .iter()
        .map(|LineId(i)| *i)
        .collect();
    let construction_line_count = sketch
        .lines
        .iter()
        .enumerate()
        .filter(|(i, _)| construction_lines.contains(i))
        .count();

    let mut invalid_line_indices = Vec::new();
    let mut regular_edges: Vec<(usize, SketchLine)> = Vec::new();
    for (idx, line) in sketch.lines.iter().copied().enumerate() {
        if construction_lines.contains(&idx) {
            continue;
        }
        if line.start.0 >= sketch.points.len()
            || line.end.0 >= sketch.points.len()
            || line.start == line.end
        {
            invalid_line_indices.push(idx);
            continue;
        }
        regular_edges.push((idx, line));
    }

    let regular_line_count = regular_edges.len();
    if regular_edges.is_empty() {
        return SketchProfileAnalysis {
            invalid_line_indices,
            regular_line_count,
            construction_line_count,
            ..SketchProfileAnalysis::default()
        };
    }

    let mut adj: HashMap<usize, Vec<(usize, usize)>> = HashMap::new();
    for (li, line) in &regular_edges {
        adj.entry(line.start.0).or_default().push((*li, line.end.0));
        adj.entry(line.end.0).or_default().push((*li, line.start.0));
    }

    let mut open_endpoints: Vec<PointId> = adj
        .iter()
        .filter_map(|(&pid, edges)| (edges.len() == 1).then_some(PointId(pid)))
        .collect();
    let mut branch_points: Vec<PointId> = adj
        .iter()
        .filter_map(|(&pid, edges)| (edges.len() > 2).then_some(PointId(pid)))
        .collect();
    open_endpoints.sort_by_key(|p| p.0);
    branch_points.sort_by_key(|p| p.0);

    let mut edge_by_index: HashMap<usize, SketchLine> = HashMap::new();
    for (idx, line) in &regular_edges {
        edge_by_index.insert(*idx, *line);
    }
    let mut visited: HashSet<usize> = HashSet::new();
    let mut loops = Vec::new();

    for (start_li, start_line) in &regular_edges {
        if visited.contains(start_li) {
            continue;
        }

        let component = collect_component(*start_li, &edge_by_index, &adj);
        for li in &component {
            visited.insert(*li);
        }

        let all_degree_two = component.iter().all(|li| {
            let line = edge_by_index[li];
            adj.get(&line.start.0).is_some_and(|v| v.len() == 2)
                && adj.get(&line.end.0).is_some_and(|v| v.len() == 2)
        });
        if !all_degree_two {
            continue;
        }

        if let Some(loop_pts) = trace_loop(*start_li, *start_line, &adj, component.len()) {
            if loop_pts.len() >= 3 {
                loops.push(loop_pts);
            }
        }
    }

    SketchProfileAnalysis {
        loops,
        open_endpoints,
        branch_points,
        invalid_line_indices,
        regular_line_count,
        construction_line_count,
    }
}

fn collect_component(
    start_li: usize,
    edge_by_index: &HashMap<usize, SketchLine>,
    adj: &HashMap<usize, Vec<(usize, usize)>>,
) -> Vec<usize> {
    let mut out = Vec::new();
    let mut seen: HashSet<usize> = HashSet::new();
    let mut stack = vec![start_li];
    while let Some(li) = stack.pop() {
        if !seen.insert(li) {
            continue;
        }
        out.push(li);
        let line = edge_by_index[&li];
        for pid in [line.start.0, line.end.0] {
            if let Some(edges) = adj.get(&pid) {
                for (next_li, _) in edges {
                    if !seen.contains(next_li) {
                        stack.push(*next_li);
                    }
                }
            }
        }
    }
    out
}

fn trace_loop(
    start_li: usize,
    start_line: SketchLine,
    adj: &HashMap<usize, Vec<(usize, usize)>>,
    expected_edges: usize,
) -> Option<Vec<PointId>> {
    let start = start_line.start.0;
    let mut current = start_line.end.0;
    let mut prev_line = start_li;
    let mut used = HashSet::from([start_li]);
    let mut points = vec![PointId(start)];

    loop {
        if current == start {
            return (used.len() == expected_edges).then_some(points);
        }
        if points.len() > expected_edges {
            return None;
        }
        points.push(PointId(current));
        let neighbors = adj.get(&current)?;
        let (next_line, next_point) = neighbors.iter().find(|(li, _)| *li != prev_line).copied()?;
        if !used.insert(next_line) && next_point != start {
            return None;
        }
        prev_line = next_line;
        current = next_point;
    }
}

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
            let p = sketch.points.get(pid.0).unwrap_or(&sketch.points[0]);
            plane.to_world(p.position.x, p.position.y)
        })
        .collect()
}

/// Extract a single closed, non-construction profile loop as world points.
///
/// Returns a validation error for open chains, branch points, invalid line
/// references, or sketches with zero/multiple closed loops. Use this for
/// feature commands such as Pad/Pocket that require an extrudable profile.
pub fn extract_profile_checked(sketch: &Sketch, plane: &WorkPlane) -> KernelResult<Vec<Point3>> {
    let analysis = analyze_profiles(sketch);
    if analysis.is_single_closed_profile() {
        return Ok(analysis.loops[0]
            .iter()
            .map(|pid| {
                let p = &sketch.points[pid.0];
                plane.to_world(p.position.x, p.position.y)
            })
            .collect());
    }

    Err(KernelError::InvalidArgument(analysis.status_label()))
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

    #[test]
    fn test_analyze_profiles_reports_single_closed_square() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(1.0, 0.0);
        let p2 = sketch.add_point(1.0, 1.0);
        let p3 = sketch.add_point(0.0, 1.0);
        sketch.add_line(p0, p1);
        sketch.add_line(p1, p2);
        sketch.add_line(p2, p3);
        sketch.add_line(p3, p0);

        let analysis = analyze_profiles(&sketch);
        assert!(analysis.is_single_closed_profile(), "{analysis:?}");
        assert_eq!(analysis.loops.len(), 1);
        assert_eq!(analysis.loops[0], vec![p0, p1, p2, p3]);
        assert!(analysis.open_endpoints.is_empty());
    }

    #[test]
    fn test_analyze_profiles_ignores_construction_diagonal() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(1.0, 0.0);
        let p2 = sketch.add_point(1.0, 1.0);
        let p3 = sketch.add_point(0.0, 1.0);
        sketch.add_line(p0, p1);
        sketch.add_line(p1, p2);
        sketch.add_line(p2, p3);
        sketch.add_line(p3, p0);
        let diag = sketch.add_line(p0, p2);
        sketch.construction_lines.push(diag);

        let analysis = analyze_profiles(&sketch);
        assert!(analysis.is_single_closed_profile(), "{analysis:?}");
        assert_eq!(analysis.regular_line_count, 4);
        assert_eq!(analysis.construction_line_count, 1);
    }

    #[test]
    fn test_extract_profile_checked_rejects_open_chain() {
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(1.0, 0.0);
        let p2 = sketch.add_point(1.0, 1.0);
        sketch.add_line(p0, p1);
        sketch.add_line(p1, p2);

        let err = extract_profile_checked(&sketch, &WorkPlane::xy()).unwrap_err();
        assert!(err.to_string().contains("open"), "{err}");
    }
}
