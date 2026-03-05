use crate::point::Point3;
use crate::vector::Vec3;

/// Axis-Aligned Bounding Box in 3D.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    pub min: Point3,
    pub max: Point3,
}

impl BoundingBox {
    /// Creates a bounding box from two corner points.
    /// The min/max are computed component-wise.
    pub fn new(a: Point3, b: Point3) -> Self {
        Self {
            min: Point3::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z)),
            max: Point3::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z)),
        }
    }

    /// Creates an "empty" bounding box that will expand on the first `include_point`.
    pub fn empty() -> Self {
        Self {
            min: Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY),
            max: Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY),
        }
    }

    /// Returns `true` if no points have been added.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.min.x > self.max.x
    }

    /// Expands the box to include the given point. Returns `&mut Self` for chaining.
    pub fn include_point(&mut self, p: Point3) -> &mut Self {
        self.min.x = self.min.x.min(p.x);
        self.min.y = self.min.y.min(p.y);
        self.min.z = self.min.z.min(p.z);
        self.max.x = self.max.x.max(p.x);
        self.max.y = self.max.y.max(p.y);
        self.max.z = self.max.z.max(p.z);
        self
    }

    /// Returns `true` if `p` is inside or on the boundary.
    #[inline]
    pub fn contains(&self, p: Point3) -> bool {
        p.x >= self.min.x
            && p.x <= self.max.x
            && p.y >= self.min.y
            && p.y <= self.max.y
            && p.z >= self.min.z
            && p.z <= self.max.z
    }

    /// Returns the union of two bounding boxes.
    pub fn union(&self, other: &Self) -> Self {
        Self {
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

    /// Returns the intersection, or `None` if they don't overlap.
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let min = Point3::new(
            self.min.x.max(other.min.x),
            self.min.y.max(other.min.y),
            self.min.z.max(other.min.z),
        );
        let max = Point3::new(
            self.max.x.min(other.max.x),
            self.max.y.min(other.max.y),
            self.max.z.min(other.max.z),
        );
        if min.x <= max.x && min.y <= max.y && min.z <= max.z {
            Some(Self { min, max })
        } else {
            None
        }
    }

    /// Center point of the bounding box.
    #[inline]
    pub fn center(&self) -> Point3 {
        self.min.midpoint(self.max)
    }

    /// Diagonal vector from min to max.
    #[inline]
    pub fn diagonal(&self) -> Vec3 {
        self.max - self.min
    }

    /// Returns true if this box overlaps with another.
    #[inline]
    pub fn overlaps(&self, other: &Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    /// Expands the box by `delta` in all directions.
    pub fn expand(&self, delta: f64) -> Self {
        Self {
            min: Point3::new(self.min.x - delta, self.min.y - delta, self.min.z - delta),
            max: Point3::new(self.max.x + delta, self.max.y + delta, self.max.z + delta),
        }
    }

    /// Returns the volume of the box.
    pub fn volume(&self) -> f64 {
        let s = self.size();
        s.x * s.y * s.z
    }

    /// Returns the surface area of the box.
    pub fn surface_area(&self) -> f64 {
        let s = self.size();
        2.0 * (s.x * s.y + s.y * s.z + s.z * s.x)
    }

    /// Returns the longest axis dimension (0=X, 1=Y, 2=Z).
    pub fn longest_axis(&self) -> usize {
        let s = self.size();
        if s.x >= s.y && s.x >= s.z {
            0
        } else if s.y >= s.z {
            1
        } else {
            2
        }
    }

    /// Returns the size along each axis as Vec3.
    #[inline]
    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }
}

impl Default for BoundingBox {
    fn default() -> Self {
        Self::empty()
    }
}

impl std::fmt::Display for BoundingBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BBox[{} .. {}]", self.min, self.max)
    }
}

impl From<&[Point3]> for BoundingBox {
    fn from(points: &[Point3]) -> Self {
        let mut bb = Self::empty();
        for &p in points {
            bb.include_point(p);
        }
        bb
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains() {
        let bb = BoundingBox::new(Point3::ORIGIN, Point3::new(1.0, 1.0, 1.0));
        assert!(bb.contains(Point3::new(0.5, 0.5, 0.5)));
        assert!(!bb.contains(Point3::new(2.0, 0.5, 0.5)));
    }

    #[test]
    fn test_union() {
        let a = BoundingBox::new(Point3::ORIGIN, Point3::new(1.0, 1.0, 1.0));
        let b = BoundingBox::new(Point3::new(2.0, 2.0, 2.0), Point3::new(3.0, 3.0, 3.0));
        let u = a.union(&b);
        assert!(u.contains(Point3::new(0.0, 0.0, 0.0)));
        assert!(u.contains(Point3::new(3.0, 3.0, 3.0)));
    }

    #[test]
    fn test_intersection() {
        let a = BoundingBox::new(Point3::ORIGIN, Point3::new(2.0, 2.0, 2.0));
        let b = BoundingBox::new(Point3::new(1.0, 1.0, 1.0), Point3::new(3.0, 3.0, 3.0));
        let i = a.intersection(&b).unwrap();
        assert!(i.min.approx_eq(Point3::new(1.0, 1.0, 1.0)));
        assert!(i.max.approx_eq(Point3::new(2.0, 2.0, 2.0)));
    }

    #[test]
    fn test_no_intersection() {
        let a = BoundingBox::new(Point3::ORIGIN, Point3::new(1.0, 1.0, 1.0));
        let b = BoundingBox::new(Point3::new(5.0, 5.0, 5.0), Point3::new(6.0, 6.0, 6.0));
        assert!(a.intersection(&b).is_none());
    }

    #[test]
    fn test_include_point() {
        let mut bb = BoundingBox::empty();
        assert!(bb.is_empty());
        bb.include_point(Point3::new(1.0, 2.0, 3.0));
        bb.include_point(Point3::new(-1.0, -2.0, -3.0));
        assert!(!bb.is_empty());
        assert!(bb.contains(Point3::ORIGIN));
    }

    #[test]
    fn test_overlaps() {
        let a = BoundingBox::new(Point3::ORIGIN, Point3::new(2.0, 2.0, 2.0));
        let b = BoundingBox::new(Point3::new(1.0, 1.0, 1.0), Point3::new(3.0, 3.0, 3.0));
        assert!(a.overlaps(&b));

        let c = BoundingBox::new(Point3::new(5.0, 5.0, 5.0), Point3::new(6.0, 6.0, 6.0));
        assert!(!a.overlaps(&c));
    }

    #[test]
    fn test_expand() {
        let bb = BoundingBox::new(Point3::new(1.0, 1.0, 1.0), Point3::new(2.0, 2.0, 2.0));
        let e = bb.expand(0.5);
        assert!(e.min.approx_eq(Point3::new(0.5, 0.5, 0.5)));
        assert!(e.max.approx_eq(Point3::new(2.5, 2.5, 2.5)));
    }

    #[test]
    fn test_volume() {
        let bb = BoundingBox::new(Point3::ORIGIN, Point3::new(1.0, 1.0, 1.0));
        assert!((bb.volume() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_surface_area() {
        let bb = BoundingBox::new(Point3::ORIGIN, Point3::new(1.0, 1.0, 1.0));
        assert!((bb.surface_area() - 6.0).abs() < 1e-12);
    }

    #[test]
    fn test_longest_axis() {
        let bb = BoundingBox::new(Point3::ORIGIN, Point3::new(10.0, 5.0, 3.0));
        assert_eq!(bb.longest_axis(), 0);

        let bb2 = BoundingBox::new(Point3::ORIGIN, Point3::new(1.0, 9.0, 3.0));
        assert_eq!(bb2.longest_axis(), 1);

        let bb3 = BoundingBox::new(Point3::ORIGIN, Point3::new(1.0, 2.0, 8.0));
        assert_eq!(bb3.longest_axis(), 2);
    }

    #[test]
    fn test_size() {
        let bb = BoundingBox::new(Point3::ORIGIN, Point3::new(3.0, 4.0, 5.0));
        let s = bb.size();
        assert!(s.approx_eq(Vec3::new(3.0, 4.0, 5.0)));
    }
}
