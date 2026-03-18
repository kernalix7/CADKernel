use crate::surface::Surface;

/// Principal curvature information at a point on a surface.
#[derive(Debug, Clone, Copy)]
pub struct SurfaceCurvatures {
    pub gaussian: f64,
    pub mean: f64,
    pub k_min: f64,
    pub k_max: f64,
}

/// Compute Gaussian and mean curvature at `(u, v)` using the first and second
/// fundamental forms.  Second-order derivatives are approximated with central
/// finite differences of the analytical first derivatives.
pub fn surface_curvatures(surface: &dyn Surface, u: f64, v: f64) -> SurfaceCurvatures {
    let zero = SurfaceCurvatures {
        gaussian: 0.0,
        mean: 0.0,
        k_min: 0.0,
        k_max: 0.0,
    };

    // First fundamental form coefficients
    let du = surface.du(u, v);
    let dv = surface.dv(u, v);
    let e_coeff = du.dot(du);
    let f_coeff = du.dot(dv);
    let g_coeff = dv.dot(dv);

    // Unit normal
    let n = du.cross(dv);
    let n_len = n.length();
    if n_len < 1e-14 {
        return zero;
    }
    let n = n / n_len;

    // Second fundamental form via finite differences of first derivatives
    let h = 1e-6;
    let duu = (surface.du(u + h, v) - surface.du(u - h, v)) / (2.0 * h);
    let dvv = (surface.dv(u, v + h) - surface.dv(u, v - h)) / (2.0 * h);
    let duv = (surface.du(u, v + h) - surface.du(u, v - h)) / (2.0 * h);

    let l_coeff = duu.dot(n);
    let m_coeff = duv.dot(n);
    let n_coeff = dvv.dot(n);

    let det_i = e_coeff * g_coeff - f_coeff * f_coeff;
    if det_i.abs() < 1e-14 {
        return zero;
    }

    let gaussian = (l_coeff * n_coeff - m_coeff * m_coeff) / det_i;
    let mean = (e_coeff * n_coeff - 2.0 * f_coeff * m_coeff + g_coeff * l_coeff) / (2.0 * det_i);

    // Principal curvatures
    let disc = (mean * mean - gaussian).max(0.0);
    let sqrt_disc = disc.sqrt();
    let k_min = mean - sqrt_disc;
    let k_max = mean + sqrt_disc;

    SurfaceCurvatures {
        gaussian,
        mean,
        k_min,
        k_max,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::plane::Plane;
    use crate::surface::sphere::Sphere;
    use cadkernel_math::Point3;
    use cadkernel_math::Vec3;

    #[test]
    fn test_plane_curvature_is_zero() {
        let plane = Plane::new(Point3::ORIGIN, Vec3::X, Vec3::Y).unwrap();
        let c = surface_curvatures(&plane, 0.0, 0.0);
        assert!(c.gaussian.abs() < 1e-6);
        assert!(c.mean.abs() < 1e-6);
        assert!(c.k_min.abs() < 1e-6);
        assert!(c.k_max.abs() < 1e-6);
    }

    #[test]
    fn test_sphere_curvature() {
        let r = 3.0;
        let sphere = Sphere::new(Point3::ORIGIN, r).unwrap();
        let c = surface_curvatures(&sphere, 0.5, 0.3);
        let expected_gaussian = 1.0 / (r * r);
        let expected_mean_abs = 1.0 / r;
        assert!(
            (c.gaussian - expected_gaussian).abs() < 0.01,
            "gaussian: got {} expected {}",
            c.gaussian,
            expected_gaussian
        );
        // Mean curvature sign depends on normal orientation convention
        assert!(
            (c.mean.abs() - expected_mean_abs).abs() < 0.01,
            "mean: got {} expected +/-{}",
            c.mean,
            expected_mean_abs
        );
    }
}
