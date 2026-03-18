//! Bounding Volume Hierarchy for accelerating spatial queries.
//!
//! Provides an AABB-based BVH tree with support for box overlap, point containment,
//! and ray intersection queries.

use cadkernel_math::{Point3, Vec3};

/// Axis-aligned bounding box.
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

        // Find longest axis
        let dx = total.max.x - total.min.x;
        let dy = total.max.y - total.min.y;
        let dz = total.max.z - total.min.z;

        let axis: fn(&Aabb) -> f64 = if dx >= dy && dx >= dz {
            |aabb| aabb.center().x
        } else if dy >= dz {
            |aabb| aabb.center().y
        } else {
            |aabb| aabb.center().z
        };

        // Sort by center along chosen axis
        items.sort_by(|a, b| {
            axis(&a.0)
                .partial_cmp(&axis(&b.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Split at midpoint
        let mid = items.len() / 2;
        let (left_items, right_items) = items.split_at_mut(mid);

        let left = Box::new(Self::build_recursive(left_items));
        let right = Box::new(Self::build_recursive(right_items));

        let aabb = left.aabb().merge(right.aabb());

        BvhNode::Internal { aabb, left, right }
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
}
