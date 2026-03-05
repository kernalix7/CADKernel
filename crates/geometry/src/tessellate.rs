//! Adaptive tessellation of curves and surfaces.
//!
//! Provides [`TessellationOptions`] for controlling chord-error and angle-based
//! subdivision, free functions [`adaptive_tessellate_curve`] and
//! [`adaptive_tessellate_surface`] that accept evaluation closures, and
//! convenience extension traits [`TessellateCurve`] / [`TessellateSurface`]
//! for direct use on [`Curve`](crate::curve::Curve) and
//! [`Surface`](crate::surface::Surface) implementors.

use cadkernel_math::Point3;

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
// Free functions (closure-based API)
// ---------------------------------------------------------------------------

/// Adaptively tessellates a parametric curve into a polyline.
///
/// # Arguments
///
/// * `eval` — evaluates the curve position at parameter `t`.
/// * `tangent` — evaluates the curve tangent at parameter `t`.
/// * `t_start` — start of the parameter domain.
/// * `t_end` — end of the parameter domain.
/// * `opts` — tessellation quality settings.
///
/// # Panics
///
/// Not yet implemented — currently calls `todo!()`.
pub fn adaptive_tessellate_curve<F, G>(
    eval: F,
    tangent: G,
    t_start: f64,
    t_end: f64,
    opts: &TessellationOptions,
) -> Vec<Point3>
where
    F: Fn(f64) -> Point3,
    G: Fn(f64) -> cadkernel_math::Vec3,
{
    let _ = (eval, tangent, t_start, t_end, opts);
    todo!("adaptive_tessellate_curve: not yet implemented")
}

/// Adaptively tessellates a parametric surface into a triangle mesh.
///
/// # Arguments
///
/// * `eval` — evaluates the surface position at parameters `(u, v)`.
/// * `normal` — evaluates the surface normal at parameters `(u, v)`.
/// * `u_domain` — `(u_min, u_max)` parameter range.
/// * `v_domain` — `(v_min, v_max)` parameter range.
/// * `opts` — tessellation quality settings.
///
/// # Panics
///
/// Not yet implemented — currently calls `todo!()`.
pub fn adaptive_tessellate_surface<F, G>(
    eval: F,
    normal: G,
    u_domain: (f64, f64),
    v_domain: (f64, f64),
    opts: &TessellationOptions,
) -> TessMesh
where
    F: Fn(f64, f64) -> Point3,
    G: Fn(f64, f64) -> cadkernel_math::Vec3,
{
    let _ = (eval, normal, u_domain, v_domain, opts);
    todo!("adaptive_tessellate_surface: not yet implemented")
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
