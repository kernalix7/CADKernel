//! Mass-property computation for B-Rep solids and triangle meshes.

use cadkernel_core::KernelResult;
use cadkernel_math::Point3;
use cadkernel_topology::{BRepModel, Handle, SolidData};

/// Mass properties of a solid body.
#[derive(Debug, Clone)]
pub struct MassProperties {
    /// Volume of the solid.
    pub volume: f64,
    /// Surface area of the solid.
    pub surface_area: f64,
    /// Center of mass (centroid).
    pub centroid: Point3,
}

/// Computes mass properties from a tessellated mesh.
///
/// Uses the divergence theorem on the triangle mesh to compute signed
/// volume, surface area, and centroid.
pub fn compute_mass_properties(mesh: &cadkernel_io::Mesh) -> MassProperties {
    let mut volume = 0.0_f64;
    let mut area = 0.0_f64;
    let mut cx = 0.0_f64;
    let mut cy = 0.0_f64;
    let mut cz = 0.0_f64;

    for tri in mesh.indices.iter() {
        let v0 = mesh.vertices[tri[0] as usize];
        let v1 = mesh.vertices[tri[1] as usize];
        let v2 = mesh.vertices[tri[2] as usize];

        // Signed volume contribution via divergence theorem
        let cross_x = (v1.y - v0.y) * (v2.z - v0.z) - (v1.z - v0.z) * (v2.y - v0.y);
        let cross_y = (v1.z - v0.z) * (v2.x - v0.x) - (v1.x - v0.x) * (v2.z - v0.z);
        let cross_z = (v1.x - v0.x) * (v2.y - v0.y) - (v1.y - v0.y) * (v2.x - v0.x);

        let tri_vol = v0.x * cross_x + v0.y * cross_y + v0.z * cross_z;
        volume += tri_vol;

        // Surface area
        let area_2 = (cross_x * cross_x + cross_y * cross_y + cross_z * cross_z).sqrt();
        area += area_2;

        // Centroid weighted by volume contribution
        let mid_x = (v0.x + v1.x + v2.x) / 4.0;
        let mid_y = (v0.y + v1.y + v2.y) / 4.0;
        let mid_z = (v0.z + v1.z + v2.z) / 4.0;
        cx += tri_vol * mid_x;
        cy += tri_vol * mid_y;
        cz += tri_vol * mid_z;
    }

    volume /= 6.0;
    area /= 2.0;
    let abs_vol = volume.abs().max(1e-30);
    cx /= 6.0 * abs_vol;
    cy /= 6.0 * abs_vol;
    cz /= 6.0 * abs_vol;

    MassProperties {
        volume: volume.abs(),
        surface_area: area,
        centroid: Point3::new(cx, cy, cz),
    }
}

/// Computes mass properties for a solid by tessellating its faces.
pub fn solid_mass_properties(
    _model: &BRepModel,
    _solid: Handle<SolidData>,
) -> KernelResult<MassProperties> {
    todo!("solid_mass_properties not yet implemented")
}
