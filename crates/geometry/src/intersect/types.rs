use cadkernel_math::{Point3, Vec3};

/// An ellipse in 3D resulting from a surface-surface intersection.
#[derive(Debug, Clone)]
pub struct IntersectionEllipse {
    pub center: Point3,
    pub normal: Vec3,
    pub major_axis: Vec3,
    pub minor_axis: Vec3,
    pub semi_major: f64,
    pub semi_minor: f64,
}

impl IntersectionEllipse {
    /// Evaluates a point on the ellipse at angle `t` (radians).
    pub fn point_at(&self, t: f64) -> Point3 {
        let (sin, cos) = t.sin_cos();
        self.center
            + self.major_axis * (self.semi_major * cos)
            + self.minor_axis * (self.semi_minor * sin)
    }
}

/// Result of a surface-surface intersection.
#[derive(Debug, Clone)]
pub enum SsiResult {
    /// No intersection.
    Empty,
    /// Surfaces touch at a single point.
    Point(Point3),
    /// Intersection is a line (infinite or parameterised).
    Line { origin: Point3, direction: Vec3 },
    /// Intersection is a circle.
    Circle {
        center: Point3,
        normal: Vec3,
        radius: f64,
    },
    /// Intersection is an ellipse.
    Ellipse(IntersectionEllipse),
    /// Surfaces are coincident (infinite overlap).
    Coincident,
}

/// Result of a ray/line vs surface intersection.
#[derive(Debug, Clone)]
pub struct RayHit {
    /// Parameter along the ray.
    pub t: f64,
    /// Point of intersection.
    pub point: Point3,
}
