//! Bounding Volume Hierarchy for accelerating spatial queries.
//!
//! Provides an AABB-based BVH tree with support for box overlap, point containment,
//! and ray intersection queries.

use cadkernel_math::{Point3, Vec3};

/// Axis-aligned bounding box used by the BVH tree.
///
/// Differs from [`cadkernel_math::BoundingBox`] in that this type is
/// optimised for use within the geometry crate's BVH and spatial queries.
#[derive(Debug, Clone, Copy)]
pub struct Aabb {
    pub min: Point3,
    pub max: Point3,
}

impl Aabb {
    /// Creates a new AABB from explicit min/max corners.
    #[inline]
    pub fn new(min: Point3, max: Point3) -> Self {
        Self { min, max }
    }

    /// Computes the smallest AABB enclosing all given points.
    ///
    /// # Panics
    ///
    /// Panics if `points` is empty.
    pub fn from_points(points: &[Point3]) -> Self {
        assert!(!points.is_empty(), "Aabb::from_points requires at least one point");
        let mut min = points[0];
        let mut max = points[0];
        for &p in &points[1..] {
            min.x = min.x.min(p.x);
            min.y = min.y.min(p.y);
            min.z = min.z.min(p.z);
            max.x = max.x.max(p.x);
            max.y = max.y.max(p.y);
            max.z = max.z.max(p.z);
        }
        Self { min, max }
    }

    /// Returns the union of `self` and `other`.
    #[inline]
    pub fn merge(self, other: &Aabb) -> Aabb {
        Aabb {
            min: Point3::new(
                self.min.x.min(other.min.x),
                self.min.y.min(other.min.y),
                self.min.z.min(other.min.z),
            ),
            max: Point3::new(
                self.max.x.max(other.max.x),
                self.max.y.max(other.max.y),
                self.max.z.max(other.max.z),
            ),
        }
    }

    /// Returns `true` if this AABB overlaps `other`.
    #[inline]
    pub fn intersects(&self, other: &Aabb) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    /// Returns `true` if `p` lies inside (or on the boundary of) this AABB.
    #[inline]
    pub fn contains_point(&self, p: Point3) -> bool {
        p.x >= self.min.x
            && p.x <= self.max.x
            && p.y >= self.min.y
            && p.y <= self.max.y
            && p.z >= self.min.z
            && p.z <= self.max.z
    }

    /// Surface area of the box (used for SAH cost estimation).
    #[inline]
    pub fn surface_area(&self) -> f64 {
        let dx = self.max.x - self.min.x;
        let dy = self.max.y - self.min.y;
        let dz = self.max.z - self.min.z;
        2.0 * (dx * dy + dy * dz + dz * dx)
    }

    /// Center point of the AABB.
    #[inline]
    pub fn center(&self) -> Point3 {
        Point3::new(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
            (self.min.z + self.max.z) * 0.5,
        )
    }

    /// Returns a new AABB expanded by `margin` in every direction.
    #[inline]
    pub fn expand(&self, margin: f64) -> Aabb {
        Aabb {
            min: Point3::new(self.min.x - margin, self.min.y - margin, self.min.z - margin),
            max: Point3::new(self.max.x + margin, self.max.y + margin, self.max.z + margin),
        }
    }

    /// Minimum squared distance from a point to the closest point on the AABB.
    ///
    /// Returns 0.0 if the point is inside the box.
    #[inline]
    pub fn min_distance_sq(&self, p: Point3) -> f64 {
        let dx = if p.x < self.min.x {
            self.min.x - p.x
        } else if p.x > self.max.x {
            p.x - self.max.x
        } else {
            0.0
        };
        let dy = if p.y < self.min.y {
            self.min.y - p.y
        } else if p.y > self.max.y {
            p.y - self.max.y
        } else {
            0.0
        };
        let dz = if p.z < self.min.z {
            self.min.z - p.z
        } else if p.z > self.max.z {
            p.z - self.max.z
        } else {
            0.0
        };
        dx * dx + dy * dy + dz * dz
    }

    /// Ray-AABB intersection returning the entry parameter `t`.
    ///
    /// Returns `Some(t_enter)` if the ray hits the box at non-negative `t`,
    /// or `None` if it misses. When the ray origin is inside the box,
    /// returns `Some(0.0)`.
    pub fn intersects_ray_t(&self, origin: Point3, direction: Vec3) -> Option<f64> {
        let mut t_min = f64::NEG_INFINITY;
        let mut t_max = f64::INFINITY;

        for axis in 0..3 {
            let (o, d, lo, hi) = match axis {
                0 => (origin.x, direction.x, self.min.x, self.max.x),
                1 => (origin.y, direction.y, self.min.y, self.max.y),
                _ => (origin.z, direction.z, self.min.z, self.max.z),
            };
            if d.abs() > f64::EPSILON {
                let inv = 1.0 / d;
                let mut t0 = (lo - o) * inv;
                let mut t1 = (hi - o) * inv;
                if t0 > t1 {
                    std::mem::swap(&mut t0, &mut t1);
                }
                t_min = t_min.max(t0);
                t_max = t_max.min(t1);
            } else if o < lo || o > hi {
                return None;
            }
        }

        if t_max >= t_min && t_max >= 0.0 {
            Some(t_min.max(0.0))
        } else {
            None
        }
    }

    /// Ray-AABB intersection using the slab method.
    /// Returns `true` if the ray `(origin, direction)` intersects this box
    /// at any non-negative parameter `t`.
    fn intersects_ray(&self, origin: Point3, direction: Vec3) -> bool {
        let mut t_min = f64::NEG_INFINITY;
        let mut t_max = f64::INFINITY;

        // X slab
        if direction.x.abs() > f64::EPSILON {
            let inv = 1.0 / direction.x;
            let mut t0 = (self.min.x - origin.x) * inv;
            let mut t1 = (self.max.x - origin.x) * inv;
            if t0 > t1 {
                std::mem::swap(&mut t0, &mut t1);
            }
            t_min = t_min.max(t0);
            t_max = t_max.min(t1);
        } else if origin.x < self.min.x || origin.x > self.max.x {
            return false;
        }

        // Y slab
        if direction.y.abs() > f64::EPSILON {
            let inv = 1.0 / direction.y;
            let mut t0 = (self.min.y - origin.y) * inv;
            let mut t1 = (self.max.y - origin.y) * inv;
            if t0 > t1 {
                std::mem::swap(&mut t0, &mut t1);
            }
            t_min = t_min.max(t0);
            t_max = t_max.min(t1);
        } else if origin.y < self.min.y || origin.y > self.max.y {
            return false;
        }

        // Z slab
        if direction.z.abs() > f64::EPSILON {
            let inv = 1.0 / direction.z;
            let mut t0 = (self.min.z - origin.z) * inv;
            let mut t1 = (self.max.z - origin.z) * inv;
            if t0 > t1 {
                std::mem::swap(&mut t0, &mut t1);
            }
            t_min = t_min.max(t0);
            t_max = t_max.min(t1);
        } else if origin.z < self.min.z || origin.z > self.max.z {
            return false;
        }

        t_max >= t_min && t_max >= 0.0
    }
}

// ---------------------------------------------------------------------------
// BVH tree
// ---------------------------------------------------------------------------

enum BvhNode {
    Leaf {
        aabb: Aabb,
        index: usize,
    },
    Internal {
        aabb: Aabb,
        left: Box<BvhNode>,
        right: Box<BvhNode>,
    },
}

impl BvhNode {
    fn aabb(&self) -> &Aabb {
        match self {
            BvhNode::Leaf { aabb, .. } | BvhNode::Internal { aabb, .. } => aabb,
        }
    }
}

/// Bounding Volume Hierarchy for spatial indexing.
///
/// Build from a set of `(Aabb, index)` pairs using midpoint split along the
/// longest axis. Supports AABB overlap, point containment, and ray queries.
pub struct Bvh {
    root: Option<BvhNode>,
    count: usize,
}

impl Bvh {
    /// Builds a BVH from a slice of `(bounding_box, item_index)` pairs.
    pub fn build(items: &[(Aabb, usize)]) -> Self {
        if items.is_empty() {
            return Self {
                root: None,
                count: 0,
            };
        }
        let mut sorted: Vec<(Aabb, usize)> = items.to_vec();
        let root = Self::build_recursive(&mut sorted);
        Self {
            root: Some(root),
            count: items.len(),
        }
    }

    fn build_recursive(items: &mut [(Aabb, usize)]) -> BvhNode {
        if items.len() == 1 {
            return BvhNode::Leaf {
                aabb: items[0].0,
                index: items[0].1,
            };
        }

        // Compute total AABB
        let mut total = items[0].0;
        for item in items.iter().skip(1) {
            total = total.merge(&item.0);
        }

        // For small item counts, skip SAH and use simple midpoint along longest axis
        if items.len() <= 4 {
            let (best_axis, _) = Self::longest_axis(&total);
            Self::sort_by_axis(items, best_axis);
            let mid = items.len() / 2;
            let (left_items, right_items) = items.split_at_mut(mid);
            let left = Box::new(Self::build_recursive(left_items));
            let right = Box::new(Self::build_recursive(right_items));
            let aabb = left.aabb().merge(right.aabb());
            return BvhNode::Internal { aabb, left, right };
        }

        // SAH: evaluate all 3 axes, pick the best split
        let parent_sa = total.surface_area();
        let mut best_cost = f64::MAX;
        let mut best_axis = 0u8;
        let mut best_split = items.len() / 2;

        for axis in 0..3u8 {
            Self::sort_by_axis(items, axis);

            // Build prefix surface areas (left sweep)
            let n = items.len();
            let mut left_sa = vec![0.0f64; n];
            let mut left_box = items[0].0;
            left_sa[0] = left_box.surface_area();
            for i in 1..n {
                left_box = left_box.merge(&items[i].0);
                left_sa[i] = left_box.surface_area();
            }

            // Build suffix surface areas (right sweep)
            let mut right_sa = vec![0.0f64; n];
            let mut right_box = items[n - 1].0;
            right_sa[n - 1] = right_box.surface_area();
            for i in (0..n - 1).rev() {
                right_box = right_box.merge(&items[i].0);
                right_sa[i] = right_box.surface_area();
            }

            // Evaluate SAH cost at each split position
            // Cost(split=k) = C_trav + (left_sa/parent_sa * k + right_sa/parent_sa * (n-k))
            for k in 1..n {
                let cost = 1.0 + (left_sa[k - 1] * k as f64 + right_sa[k] * (n - k) as f64) / parent_sa;
                if cost < best_cost {
                    best_cost = cost;
                    best_axis = axis;
                    best_split = k;
                }
            }
        }

        // Re-sort along best axis and split
        Self::sort_by_axis(items, best_axis);
        let (left_items, right_items) = items.split_at_mut(best_split);

        let left = Box::new(Self::build_recursive(left_items));
        let right = Box::new(Self::build_recursive(right_items));
        let aabb = left.aabb().merge(right.aabb());

        BvhNode::Internal { aabb, left, right }
    }

    fn longest_axis(aabb: &Aabb) -> (u8, f64) {
        let dx = aabb.max.x - aabb.min.x;
        let dy = aabb.max.y - aabb.min.y;
        let dz = aabb.max.z - aabb.min.z;
        if dx >= dy && dx >= dz {
            (0, dx)
        } else if dy >= dz {
            (1, dy)
        } else {
            (2, dz)
        }
    }

    fn sort_by_axis(items: &mut [(Aabb, usize)], axis: u8) {
        let center_val: fn(&Aabb) -> f64 = match axis {
            0 => |aabb| aabb.center().x,
            1 => |aabb| aabb.center().y,
            _ => |aabb| aabb.center().z,
        };
        items.sort_by(|a, b| {
            center_val(&a.0)
                .partial_cmp(&center_val(&b.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// Returns indices of all items whose AABB overlaps the query box.
    pub fn query_aabb(&self, query: &Aabb) -> Vec<usize> {
        let mut results = Vec::new();
        if let Some(root) = &self.root {
            Self::query_aabb_recursive(root, query, &mut results);
        }
        results
    }

    fn query_aabb_recursive(node: &BvhNode, query: &Aabb, results: &mut Vec<usize>) {
        match node {
            BvhNode::Leaf { aabb, index } => {
                if aabb.intersects(query) {
                    results.push(*index);
                }
            }
            BvhNode::Internal { aabb, left, right } => {
                if aabb.intersects(query) {
                    Self::query_aabb_recursive(left, query, results);
                    Self::query_aabb_recursive(right, query, results);
                }
            }
        }
    }

    /// Returns indices of all items whose AABB contains the given point.
    pub fn query_point(&self, point: Point3) -> Vec<usize> {
        let mut results = Vec::new();
        if let Some(root) = &self.root {
            Self::query_point_recursive(root, point, &mut results);
        }
        results
    }

    fn query_point_recursive(node: &BvhNode, point: Point3, results: &mut Vec<usize>) {
        match node {
            BvhNode::Leaf { aabb, index } => {
                if aabb.contains_point(point) {
                    results.push(*index);
                }
            }
            BvhNode::Internal { aabb, left, right } => {
                if aabb.contains_point(point) {
                    Self::query_point_recursive(left, point, results);
                    Self::query_point_recursive(right, point, results);
                }
            }
        }
    }

    /// Returns indices of all items whose AABB is intersected by the ray.
    pub fn query_ray(&self, origin: Point3, direction: Vec3) -> Vec<usize> {
        let mut results = Vec::new();
        if let Some(root) = &self.root {
            Self::query_ray_recursive(root, origin, direction, &mut results);
        }
        results
    }

    fn query_ray_recursive(
        node: &BvhNode,
        origin: Point3,
        direction: Vec3,
        results: &mut Vec<usize>,
    ) {
        match node {
            BvhNode::Leaf { aabb, index } => {
                if aabb.intersects_ray(origin, direction) {
                    results.push(*index);
                }
            }
            BvhNode::Internal { aabb, left, right } => {
                if aabb.intersects_ray(origin, direction) {
                    Self::query_ray_recursive(left, origin, direction, results);
                    Self::query_ray_recursive(right, origin, direction, results);
                }
            }
        }
    }

    /// Returns the index of the item whose AABB center is closest to `point`.
    ///
    /// Uses branch-and-bound pruning: at each internal node the minimum
    /// possible squared distance from `point` to the child AABB is compared
    /// against the current best. Children that cannot improve the result are
    /// skipped entirely.
    ///
    /// Returns `None` if the BVH is empty.
    pub fn query_nearest(&self, point: Point3) -> Option<(usize, f64)> {
        let root = self.root.as_ref()?;
        let mut best_idx = usize::MAX;
        let mut best_dist_sq = f64::INFINITY;
        Self::query_nearest_recursive(root, point, &mut best_idx, &mut best_dist_sq);
        if best_idx == usize::MAX {
            None
        } else {
            Some((best_idx, best_dist_sq.sqrt()))
        }
    }

    fn query_nearest_recursive(
        node: &BvhNode,
        point: Point3,
        best_idx: &mut usize,
        best_dist_sq: &mut f64,
    ) {
        match node {
            BvhNode::Leaf { aabb, index } => {
                let center = aabb.center();
                let dsq = (center.x - point.x).powi(2)
                    + (center.y - point.y).powi(2)
                    + (center.z - point.z).powi(2);
                if dsq < *best_dist_sq {
                    *best_dist_sq = dsq;
                    *best_idx = *index;
                }
            }
            BvhNode::Internal { aabb, left, right } => {
                // Quick reject: if minimum distance to this AABB exceeds best, skip.
                let min_dsq = aabb.min_distance_sq(point);
                if min_dsq >= *best_dist_sq {
                    return;
                }

                // Visit the child whose AABB center is closer first (better pruning).
                let dl = left.aabb().min_distance_sq(point);
                let dr = right.aabb().min_distance_sq(point);
                if dl <= dr {
                    Self::query_nearest_recursive(left, point, best_idx, best_dist_sq);
                    Self::query_nearest_recursive(right, point, best_idx, best_dist_sq);
                } else {
                    Self::query_nearest_recursive(right, point, best_idx, best_dist_sq);
                    Self::query_nearest_recursive(left, point, best_idx, best_dist_sq);
                }
            }
        }
    }

    /// Returns indices of all items whose AABB is intersected by the ray,
    /// sorted by intersection distance (nearest first).
    ///
    /// Each result is `(item_index, t_enter)` where `t_enter` is the ray
    /// parameter at which the ray enters the AABB.
    pub fn query_ray_sorted(&self, origin: Point3, direction: Vec3) -> Vec<(usize, f64)> {
        let mut results = Vec::new();
        if let Some(root) = &self.root {
            Self::query_ray_sorted_recursive(root, origin, direction, &mut results);
        }
        results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    fn query_ray_sorted_recursive(
        node: &BvhNode,
        origin: Point3,
        direction: Vec3,
        results: &mut Vec<(usize, f64)>,
    ) {
        match node {
            BvhNode::Leaf { aabb, index } => {
                if let Some(t) = aabb.intersects_ray_t(origin, direction) {
                    results.push((*index, t));
                }
            }
            BvhNode::Internal { aabb, left, right } => {
                if aabb.intersects_ray(origin, direction) {
                    Self::query_ray_sorted_recursive(left, origin, direction, results);
                    Self::query_ray_sorted_recursive(right, origin, direction, results);
                }
            }
        }
    }

    /// Number of items stored in the BVH.
    #[inline]
    pub fn len(&self) -> usize {
        self.count
    }

    /// Returns `true` if the BVH contains no items.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn aabb(x0: f64, y0: f64, z0: f64, x1: f64, y1: f64, z1: f64) -> Aabb {
        Aabb::new(Point3::new(x0, y0, z0), Point3::new(x1, y1, z1))
    }

    #[test]
    fn test_aabb_from_points() {
        let pts = vec![
            Point3::new(1.0, 5.0, -2.0),
            Point3::new(-3.0, 0.0, 4.0),
            Point3::new(2.0, 3.0, 1.0),
        ];
        let bb = Aabb::from_points(&pts);
        assert_eq!(bb.min.x, -3.0);
        assert_eq!(bb.min.y, 0.0);
        assert_eq!(bb.min.z, -2.0);
        assert_eq!(bb.max.x, 2.0);
        assert_eq!(bb.max.y, 5.0);
        assert_eq!(bb.max.z, 4.0);
    }

    #[test]
    fn test_aabb_intersects() {
        let a = aabb(0.0, 0.0, 0.0, 2.0, 2.0, 2.0);
        let b = aabb(1.0, 1.0, 1.0, 3.0, 3.0, 3.0);
        let c = aabb(5.0, 5.0, 5.0, 6.0, 6.0, 6.0);

        assert!(a.intersects(&b));
        assert!(b.intersects(&a));
        assert!(!a.intersects(&c));
        assert!(!c.intersects(&a));
    }

    #[test]
    fn test_aabb_contains_point() {
        let bb = aabb(0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
        assert!(bb.contains_point(Point3::new(0.5, 0.5, 0.5)));
        assert!(bb.contains_point(Point3::new(0.0, 0.0, 0.0)));
        assert!(bb.contains_point(Point3::new(1.0, 1.0, 1.0)));
        assert!(!bb.contains_point(Point3::new(1.5, 0.5, 0.5)));
        assert!(!bb.contains_point(Point3::new(-0.1, 0.5, 0.5)));
    }

    #[test]
    fn test_bvh_build_empty() {
        let bvh = Bvh::build(&[]);
        assert!(bvh.is_empty());
        assert_eq!(bvh.len(), 0);
        assert!(bvh.query_aabb(&aabb(0.0, 0.0, 0.0, 1.0, 1.0, 1.0)).is_empty());
        assert!(bvh.query_point(Point3::new(0.0, 0.0, 0.0)).is_empty());
        assert!(bvh.query_ray(Point3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)).is_empty());
    }

    #[test]
    fn test_bvh_single_item() {
        let items = vec![(aabb(0.0, 0.0, 0.0, 1.0, 1.0, 1.0), 42)];
        let bvh = Bvh::build(&items);
        assert_eq!(bvh.len(), 1);

        let hits = bvh.query_aabb(&aabb(0.5, 0.5, 0.5, 2.0, 2.0, 2.0));
        assert_eq!(hits, vec![42]);

        let misses = bvh.query_aabb(&aabb(5.0, 5.0, 5.0, 6.0, 6.0, 6.0));
        assert!(misses.is_empty());
    }

    #[test]
    fn test_bvh_query_aabb() {
        let items: Vec<(Aabb, usize)> = (0..10)
            .map(|i| {
                let f = i as f64;
                (aabb(f, 0.0, 0.0, f + 1.0, 1.0, 1.0), i)
            })
            .collect();

        let bvh = Bvh::build(&items);
        assert_eq!(bvh.len(), 10);

        // Query box overlapping items 2, 3, 4
        let hits = bvh.query_aabb(&aabb(2.5, 0.0, 0.0, 4.5, 1.0, 1.0));
        assert!(hits.contains(&2));
        assert!(hits.contains(&3));
        assert!(hits.contains(&4));
        assert!(!hits.contains(&0));
        assert!(!hits.contains(&1));
        assert!(!hits.contains(&6));

        // Query box far away
        let far = bvh.query_aabb(&aabb(100.0, 100.0, 100.0, 200.0, 200.0, 200.0));
        assert!(far.is_empty());
    }

    #[test]
    fn test_bvh_query_point() {
        let items: Vec<(Aabb, usize)> = (0..10)
            .map(|i| {
                let f = i as f64;
                (aabb(f, 0.0, 0.0, f + 1.0, 1.0, 1.0), i)
            })
            .collect();

        let bvh = Bvh::build(&items);

        // Point at (3.5, 0.5, 0.5) should be inside item 3
        let hits = bvh.query_point(Point3::new(3.5, 0.5, 0.5));
        assert_eq!(hits, vec![3]);

        // Point at boundary (3.0, 0.5, 0.5) — inside items 2 and 3
        let boundary = bvh.query_point(Point3::new(3.0, 0.5, 0.5));
        assert!(boundary.contains(&2));
        assert!(boundary.contains(&3));

        // Point outside
        let outside = bvh.query_point(Point3::new(50.0, 50.0, 50.0));
        assert!(outside.is_empty());
    }

    #[test]
    fn test_bvh_query_ray() {
        // Three boxes along X axis
        let items = vec![
            (aabb(0.0, 0.0, 0.0, 1.0, 1.0, 1.0), 0),
            (aabb(3.0, 0.0, 0.0, 4.0, 1.0, 1.0), 1),
            (aabb(6.0, 0.0, 0.0, 7.0, 1.0, 1.0), 2),
        ];
        let bvh = Bvh::build(&items);

        // Ray along +X through center of all boxes
        let hits = bvh.query_ray(Point3::new(-1.0, 0.5, 0.5), Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(hits.len(), 3);
        assert!(hits.contains(&0));
        assert!(hits.contains(&1));
        assert!(hits.contains(&2));

        // Ray along +Y, only hits box 0
        let hits_y = bvh.query_ray(Point3::new(0.5, -1.0, 0.5), Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(hits_y.len(), 1);
        assert!(hits_y.contains(&0));

        // Ray that misses everything
        let misses = bvh.query_ray(Point3::new(0.5, 5.0, 0.5), Vec3::new(1.0, 0.0, 0.0));
        assert!(misses.is_empty());
    }

    #[test]
    fn test_aabb_min_distance_sq() {
        let bb = aabb(0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
        // Point inside: distance 0
        assert_eq!(bb.min_distance_sq(Point3::new(0.5, 0.5, 0.5)), 0.0);
        // Point outside on X axis
        let dsq = bb.min_distance_sq(Point3::new(3.0, 0.5, 0.5));
        assert!((dsq - 4.0).abs() < 1e-10, "Expected 4.0, got {dsq}");
        // Point at corner offset
        let dsq2 = bb.min_distance_sq(Point3::new(2.0, 2.0, 2.0));
        assert!((dsq2 - 3.0).abs() < 1e-10, "Expected 3.0, got {dsq2}");
    }

    #[test]
    fn test_aabb_intersects_ray_t() {
        let bb = aabb(0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
        // Ray from outside along +X
        let t = bb.intersects_ray_t(Point3::new(-2.0, 0.5, 0.5), Vec3::new(1.0, 0.0, 0.0));
        assert!(t.is_some());
        assert!((t.unwrap() - 2.0).abs() < 1e-10);
        // Ray from inside: t = 0
        let t2 = bb.intersects_ray_t(Point3::new(0.5, 0.5, 0.5), Vec3::new(1.0, 0.0, 0.0));
        assert!(t2.is_some());
        assert!((t2.unwrap()).abs() < 1e-10);
        // Ray that misses
        let t3 = bb.intersects_ray_t(Point3::new(-2.0, 5.0, 0.5), Vec3::new(1.0, 0.0, 0.0));
        assert!(t3.is_none());
    }

    #[test]
    fn test_bvh_query_nearest() {
        let items: Vec<(Aabb, usize)> = (0..10)
            .map(|i| {
                let f = i as f64 * 3.0;
                (aabb(f, 0.0, 0.0, f + 1.0, 1.0, 1.0), i)
            })
            .collect();

        let bvh = Bvh::build(&items);

        // Query point near item 3 (center at 9.5, 0.5, 0.5)
        let result = bvh.query_nearest(Point3::new(9.5, 0.5, 0.5));
        assert!(result.is_some());
        let (idx, dist) = result.unwrap();
        assert_eq!(idx, 3);
        assert!(dist < 0.01);

        // Query point far away — should still find the nearest
        let result2 = bvh.query_nearest(Point3::new(100.0, 0.5, 0.5));
        assert!(result2.is_some());
        assert_eq!(result2.unwrap().0, 9); // last item is closest
    }

    #[test]
    fn test_bvh_query_nearest_empty() {
        let bvh = Bvh::build(&[]);
        assert!(bvh.query_nearest(Point3::new(0.0, 0.0, 0.0)).is_none());
    }

    #[test]
    fn test_bvh_query_ray_sorted() {
        // Three boxes along X axis
        let items = vec![
            (aabb(0.0, 0.0, 0.0, 1.0, 1.0, 1.0), 0),
            (aabb(5.0, 0.0, 0.0, 6.0, 1.0, 1.0), 1),
            (aabb(10.0, 0.0, 0.0, 11.0, 1.0, 1.0), 2),
        ];
        let bvh = Bvh::build(&items);

        let hits = bvh.query_ray_sorted(Point3::new(-1.0, 0.5, 0.5), Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(hits.len(), 3);
        // Should be sorted by entry t: item 0 first, then 1, then 2
        assert_eq!(hits[0].0, 0);
        assert_eq!(hits[1].0, 1);
        assert_eq!(hits[2].0, 2);
        assert!(hits[0].1 < hits[1].1);
        assert!(hits[1].1 < hits[2].1);
    }

    #[test]
    fn test_bvh_sah_build_correctness() {
        // 20 non-uniform boxes spread across 3D space — triggers SAH
        let items: Vec<(Aabb, usize)> = (0..20)
            .map(|i| {
                let f = i as f64;
                let size = 0.5 + (i % 3) as f64;
                (aabb(f * 2.0, f * 0.5, f * 1.5, f * 2.0 + size, f * 0.5 + size, f * 1.5 + size), i)
            })
            .collect();

        let bvh = Bvh::build(&items);
        assert_eq!(bvh.len(), 20);

        // Every item should be findable via point query at its center
        for &(ref bb, idx) in &items {
            let center = bb.center();
            let hits = bvh.query_point(center);
            assert!(hits.contains(&idx), "item {} not found at its center", idx);
        }

        // AABB query should find subset
        let hits = bvh.query_aabb(&aabb(3.0, 0.0, 0.0, 8.0, 5.0, 10.0));
        assert!(!hits.is_empty());
    }
}
