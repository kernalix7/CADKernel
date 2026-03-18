//! Adaptive tessellation of curves and surfaces.
//!
//! Provides [`TessellationOptions`] for controlling chord-error and angle-based
//! subdivision, free functions [`adaptive_tessellate_curve`] and
//! [`adaptive_tessellate_surface`] that accept evaluation closures, and
//! convenience extension traits [`TessellateCurve`] / [`TessellateSurface`]
//! for direct use on [`Curve`](crate::curve::Curve) and
//! [`Surface`](crate::surface::Surface) implementors.

use cadkernel_math::{Point3, Vec3};

use crate::curve::Curve;
use crate::surface::Surface;

// ---------------------------------------------------------------------------
// TessellationOptions
// ---------------------------------------------------------------------------

/// Options that control the adaptive tessellation algorithm.
///
/// This struct is `Send + Sync` so it can be shared across threads.
#[derive(Debug, Clone, Copy)]
pub struct TessellationOptions {
    /// Maximum allowable chord error (distance from the midpoint of a linear
    /// segment to the true curve/surface), in model units.
    pub chord_tolerance: f64,
    /// Maximum allowable angle deviation between consecutive tangent vectors
    /// (in radians).
    pub angle_tolerance: f64,
    /// Minimum number of segments for the initial uniform subdivision.
    pub min_segments: usize,
    /// Maximum recursion depth for adaptive refinement.
    pub max_depth: usize,
}

impl Default for TessellationOptions {
    fn default() -> Self {
        Self {
            chord_tolerance: 0.01,
            angle_tolerance: 15.0_f64.to_radians(),
            min_segments: 4,
            max_depth: 8,
        }
    }
}

// ---------------------------------------------------------------------------
// TessMesh
// ---------------------------------------------------------------------------

/// A triangle mesh produced by surface tessellation.
///
/// This struct is `Send + Sync` so it can be shared across threads.
#[derive(Debug, Clone)]
pub struct TessMesh {
    /// Vertex positions.
    pub vertices: Vec<Point3>,
    /// Triangle indices — each element is `[i0, i1, i2]` indexing into
    /// `vertices`.
    pub indices: Vec<[u32; 3]>,
}

// ---------------------------------------------------------------------------
// Adaptive curve tessellation
// ---------------------------------------------------------------------------

/// Adaptively tessellates a parametric curve into a polyline.
///
/// Uses recursive bisection: at each interval `[t0, t1]`, the midpoint on
/// the curve is compared to the midpoint of the linear chord.  If the
/// chord error exceeds `opts.chord_tolerance` **or** the tangent angle change
/// exceeds `opts.angle_tolerance`, the interval is subdivided.
pub fn adaptive_tessellate_curve<F, G>(
    eval: F,
    tangent: G,
    t_start: f64,
    t_end: f64,
    opts: &TessellationOptions,
) -> Vec<Point3>
where
    F: Fn(f64) -> Point3,
    G: Fn(f64) -> Vec3,
{
    let n = opts.min_segments.max(2);
    let dt = (t_end - t_start) / n as f64;

    // Build initial uniform samples.
    let mut params: Vec<f64> = (0..=n).map(|i| t_start + dt * i as f64).collect();
    // Clamp the last value exactly to t_end.
    *params.last_mut().unwrap() = t_end;

    // Adaptive refinement pass.
    let cos_angle_tol = opts.angle_tolerance.cos();
    let mut i = 0;
    while i + 1 < params.len() {
        let t0 = params[i];
        let t1 = params[i + 1];
        let depth = depth_from_spacing(t0, t1, t_start, t_end, opts);
        if depth >= opts.max_depth {
            i += 1;
            continue;
        }

        let t_mid = 0.5 * (t0 + t1);
        let p0 = eval(t0);
        let p1 = eval(t1);
        let p_mid = eval(t_mid);

        let chord_mid = Point3::new(
            0.5 * (p0.x + p1.x),
            0.5 * (p0.y + p1.y),
            0.5 * (p0.z + p1.z),
        );
        let chord_err = p_mid.distance_to(chord_mid);

        let needs_split = if chord_err > opts.chord_tolerance {
            true
        } else {
            let tan0 = tangent(t0);
            let tan1 = tangent(t1);
            let len0 = (tan0.x * tan0.x + tan0.y * tan0.y + tan0.z * tan0.z).sqrt();
            let len1 = (tan1.x * tan1.x + tan1.y * tan1.y + tan1.z * tan1.z).sqrt();
            if len0 < 1e-14 || len1 < 1e-14 {
                false
            } else {
                let dot = (tan0.x * tan1.x + tan0.y * tan1.y + tan0.z * tan1.z) / (len0 * len1);
                dot < cos_angle_tol
            }
        };

        if needs_split {
            params.insert(i + 1, t_mid);
            // Don't advance i — re-check the first half.
        } else {
            i += 1;
        }
    }

    params.iter().map(|&t| eval(t)).collect()
}

/// Estimates recursion depth from the interval's relative size.
fn depth_from_spacing(t0: f64, t1: f64, t_start: f64, t_end: f64, opts: &TessellationOptions) -> usize {
    let full = t_end - t_start;
    if full < 1e-14 {
        return opts.max_depth;
    }
    let frac = (t1 - t0) / full;
    let initial_frac = 1.0 / opts.min_segments.max(2) as f64;
    if frac >= initial_frac {
        return 0;
    }
    // Each bisection halves the interval.
    (initial_frac / frac).log2().ceil() as usize
}

// ---------------------------------------------------------------------------
// Adaptive surface tessellation
// ---------------------------------------------------------------------------

/// Adaptively tessellates a parametric surface into a triangle mesh.
///
/// Generates an initial uniform grid of `min_segments × min_segments` quads,
/// then adaptively subdivides cells where the chord error exceeds tolerance.
pub fn adaptive_tessellate_surface<F, G>(
    eval: F,
    normal: G,
    u_domain: (f64, f64),
    v_domain: (f64, f64),
    opts: &TessellationOptions,
) -> TessMesh
where
    F: Fn(f64, f64) -> Point3,
    G: Fn(f64, f64) -> Vec3,
{
    let _ = &normal; // Reserved for future normal-based refinement.

    let nu = opts.min_segments.max(2);
    let nv = opts.min_segments.max(2);
    let du = (u_domain.1 - u_domain.0) / nu as f64;
    let dv = (v_domain.1 - v_domain.0) / nv as f64;

    // Build initial grid of parameter values.
    let u_params: Vec<f64> = (0..=nu).map(|i| u_domain.0 + du * i as f64).collect();
    let v_params: Vec<f64> = (0..=nv).map(|j| v_domain.0 + dv * j as f64).collect();

    // Recursive quad tessellation.
    let mut mesh = TessMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
    };
    let mut vert_cache = std::collections::HashMap::<(u64, u64), u32>::new();

    for i in 0..nu {
        for j in 0..nv {
            tessellate_quad_adaptive(
                &eval,
                u_params[i],
                u_params[i + 1],
                v_params[j],
                v_params[j + 1],
                opts,
                0,
                &mut mesh,
                &mut vert_cache,
            );
        }
    }

    mesh
}

#[allow(clippy::too_many_arguments)]
fn tessellate_quad_adaptive<F>(
    eval: &F,
    u0: f64,
    u1: f64,
    v0: f64,
    v1: f64,
    opts: &TessellationOptions,
    depth: usize,
    mesh: &mut TessMesh,
    cache: &mut std::collections::HashMap<(u64, u64), u32>,
) where
    F: Fn(f64, f64) -> Point3,
{
    let p00 = eval(u0, v0);
    let p10 = eval(u1, v0);
    let p01 = eval(u0, v1);
    let p11 = eval(u1, v1);

    if depth < opts.max_depth {
        let u_mid = 0.5 * (u0 + u1);
        let v_mid = 0.5 * (v0 + v1);
        let p_center = eval(u_mid, v_mid);

        // Chord error: distance from true center to bilinear center.
        let bilinear = Point3::new(
            0.25 * (p00.x + p10.x + p01.x + p11.x),
            0.25 * (p00.y + p10.y + p01.y + p11.y),
            0.25 * (p00.z + p10.z + p01.z + p11.z),
        );
        let err = p_center.distance_to(bilinear);

        if err > opts.chord_tolerance {
            // Subdivide into 4 quads.
            tessellate_quad_adaptive(eval, u0, u_mid, v0, v_mid, opts, depth + 1, mesh, cache);
            tessellate_quad_adaptive(eval, u_mid, u1, v0, v_mid, opts, depth + 1, mesh, cache);
            tessellate_quad_adaptive(eval, u0, u_mid, v_mid, v1, opts, depth + 1, mesh, cache);
            tessellate_quad_adaptive(eval, u_mid, u1, v_mid, v1, opts, depth + 1, mesh, cache);
            return;
        }
    }

    // Emit two triangles for this quad.
    let i00 = get_or_insert(cache, mesh, u0, v0, eval);
    let i10 = get_or_insert(cache, mesh, u1, v0, eval);
    let i01 = get_or_insert(cache, mesh, u0, v1, eval);
    let i11 = get_or_insert(cache, mesh, u1, v1, eval);

    mesh.indices.push([i00, i10, i11]);
    mesh.indices.push([i00, i11, i01]);
}

fn get_or_insert<F>(
    cache: &mut std::collections::HashMap<(u64, u64), u32>,
    mesh: &mut TessMesh,
    u: f64,
    v: f64,
    eval: &F,
) -> u32
where
    F: Fn(f64, f64) -> Point3,
{
    let key = (u.to_bits(), v.to_bits());
    *cache.entry(key).or_insert_with(|| {
        let idx = mesh.vertices.len() as u32;
        mesh.vertices.push(eval(u, v));
        idx
    })
}

// ---------------------------------------------------------------------------
// Extension traits
// ---------------------------------------------------------------------------

/// Convenience extension trait for tessellating any [`Curve`] implementor.
pub trait TessellateCurve: Curve {
    /// Adaptively tessellates this curve using the given options.
    fn tessellate_adaptive(&self, opts: &TessellationOptions) -> Vec<Point3> {
        let (t0, t1) = self.domain();
        adaptive_tessellate_curve(|t| self.point_at(t), |t| self.tangent_at(t), t0, t1, opts)
    }
}

/// Blanket implementation: every `Curve` automatically gets `TessellateCurve`.
impl<T: Curve + ?Sized> TessellateCurve for T {}

/// Convenience extension trait for tessellating any [`Surface`] implementor.
pub trait TessellateSurface: Surface {
    /// Adaptively tessellates this surface using the given options.
    fn tessellate_adaptive(&self, opts: &TessellationOptions) -> TessMesh {
        let u_domain = self.domain_u();
        let v_domain = self.domain_v();
        adaptive_tessellate_surface(
            |u, v| self.point_at(u, v),
            |u, v| self.normal_at(u, v),
            u_domain,
            v_domain,
            opts,
        )
    }
}

/// Blanket implementation: every `Surface` automatically gets `TessellateSurface`.
impl<T: Surface + ?Sized> TessellateSurface for T {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tessellate_straight_line() {
        let pts = adaptive_tessellate_curve(
            |t| Point3::new(t, 0.0, 0.0),
            |_| Vec3::X,
            0.0,
            1.0,
            &TessellationOptions::default(),
        );
        // Straight line needs minimal subdivision.
        assert!(pts.len() >= 3);
        assert!(pts.first().unwrap().approx_eq(Point3::new(0.0, 0.0, 0.0)));
        assert!(pts.last().unwrap().approx_eq(Point3::new(1.0, 0.0, 0.0)));
    }

    #[test]
    fn test_tessellate_circle() {
        // A quarter circle should need more points than a straight line.
        let pts = adaptive_tessellate_curve(
            |t| Point3::new(t.cos(), t.sin(), 0.0),
            |t| Vec3::new(-t.sin(), t.cos(), 0.0),
            0.0,
            std::f64::consts::FRAC_PI_2,
            &TessellationOptions {
                chord_tolerance: 0.001,
                angle_tolerance: 5.0_f64.to_radians(),
                min_segments: 4,
                max_depth: 10,
            },
        );
        assert!(pts.len() > 4, "circle needs adaptive refinement: got {} points", pts.len());
        // First point should be (1, 0), last should be (0, 1).
        assert!((pts[0].x - 1.0).abs() < 1e-10);
        let last = pts.last().unwrap();
        assert!((last.x).abs() < 1e-10);
        assert!((last.y - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_tessellate_nurbs_curve() {
        use crate::curve::nurbs::NurbsCurve;
        let curve = NurbsCurve::bezier(vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.5, 1.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
        ])
        .unwrap();
        let pts = curve.tessellate_adaptive(&TessellationOptions::default());
        assert!(pts.len() >= 3);
        assert!(pts.first().unwrap().approx_eq(Point3::new(0.0, 0.0, 0.0)));
        assert!(pts.last().unwrap().approx_eq(Point3::new(1.0, 0.0, 0.0)));
    }

    #[test]
    fn test_tessellate_flat_surface() {
        let mesh = adaptive_tessellate_surface(
            |u, v| Point3::new(u, v, 0.0),
            |_, _| Vec3::Z,
            (0.0, 1.0),
            (0.0, 1.0),
            &TessellationOptions::default(),
        );
        // Flat surface should have minimal subdivision: n*n quads → 2*n*n triangles.
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
    }

    #[test]
    fn test_tessellate_curved_surface_more_tris() {
        // A sphere quadrant should produce more triangles than a flat patch.
        let flat = adaptive_tessellate_surface(
            |u, v| Point3::new(u, v, 0.0),
            |_, _| Vec3::Z,
            (0.0, 1.0),
            (0.0, 1.0),
            &TessellationOptions {
                chord_tolerance: 0.01,
                ..Default::default()
            },
        );
        let curved = adaptive_tessellate_surface(
            |u, v| Point3::new(v.cos() * u.cos(), v.cos() * u.sin(), v.sin()),
            |u, v| Vec3::new(v.cos() * u.cos(), v.cos() * u.sin(), v.sin()),
            (0.0, std::f64::consts::FRAC_PI_2),
            (0.0, std::f64::consts::FRAC_PI_2),
            &TessellationOptions {
                chord_tolerance: 0.01,
                ..Default::default()
            },
        );
        assert!(
            curved.indices.len() >= flat.indices.len(),
            "curved surface should have at least as many tris as flat: {} vs {}",
            curved.indices.len(),
            flat.indices.len()
        );
    }

    #[test]
    fn test_tessellate_nurbs_surface() {
        use crate::surface::nurbs::NurbsSurface;
        let surface = NurbsSurface::new(
            1,
            1,
            2,
            2,
            vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
                Point3::new(1.0, 1.0, 0.0),
            ],
            vec![1.0, 1.0, 1.0, 1.0],
            vec![0.0, 0.0, 1.0, 1.0],
            vec![0.0, 0.0, 1.0, 1.0],
        )
        .unwrap();
        let mesh = surface.tessellate_adaptive(&TessellationOptions::default());
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
    }
}
