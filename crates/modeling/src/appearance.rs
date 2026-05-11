//! Face appearance and attachment mode operations.

use std::collections::HashMap;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Mat4, Point3, Vec3};
use cadkernel_topology::{BRepModel, EntityKind, FaceData, Handle, SolidData, Tag};

use crate::features::copy_utils::collect_solid_faces;

// ---------------------------------------------------------------------------
// Face Appearance
// ---------------------------------------------------------------------------

/// Visual appearance for a single face.
#[derive(Debug, Clone)]
pub struct FaceAppearance {
    pub color: (f64, f64, f64),
    pub transparency: f64,
    pub material_name: Option<String>,
}

impl FaceAppearance {
    /// Creates a new face appearance with the given RGB color.
    pub fn new(r: f64, g: f64, b: f64) -> Self {
        Self {
            color: (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0)),
            transparency: 0.0,
            material_name: None,
        }
    }

    /// Sets transparency (0.0 = opaque, 1.0 = fully transparent).
    pub fn with_transparency(mut self, t: f64) -> Self {
        self.transparency = t.clamp(0.0, 1.0);
        self
    }

    /// Sets the material name.
    pub fn with_material(mut self, name: impl Into<String>) -> Self {
        self.material_name = Some(name.into());
        self
    }
}

/// Per-face appearance storage for a model.
#[derive(Debug, Clone, Default)]
pub struct FaceAppearanceMap {
    pub appearances: HashMap<usize, FaceAppearance>,
}

impl FaceAppearanceMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the appearance of a face at `face_idx`.
    pub fn set(&mut self, face_idx: usize, appearance: FaceAppearance) {
        self.appearances.insert(face_idx, appearance);
    }

    /// Returns the appearance of a face at `face_idx`, if any.
    pub fn get(&self, face_idx: usize) -> Option<&FaceAppearance> {
        self.appearances.get(&face_idx)
    }

    /// Removes the appearance override for a face.
    pub fn remove(&mut self, face_idx: usize) -> Option<FaceAppearance> {
        self.appearances.remove(&face_idx)
    }

    /// Returns the number of face appearance overrides.
    pub fn len(&self) -> usize {
        self.appearances.len()
    }

    /// Returns true if no face appearances are set.
    pub fn is_empty(&self) -> bool {
        self.appearances.is_empty()
    }
}

/// Sets the appearance of a face on a model.
pub fn set_face_appearance(
    map: &mut FaceAppearanceMap,
    face_idx: usize,
    appearance: FaceAppearance,
) -> KernelResult<()> {
    map.set(face_idx, appearance);
    Ok(())
}

/// Gets the appearance of a face from the map.
pub fn get_face_appearance(map: &FaceAppearanceMap, face_idx: usize) -> Option<&FaceAppearance> {
    map.get(face_idx)
}

// ---------------------------------------------------------------------------
// Attachment Mode
// ---------------------------------------------------------------------------

/// How an object attaches to a target face.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AttachmentMode {
    /// Attach flat to a planar face.
    FlatFace,
    /// Align normal to an edge direction.
    NormalToEdge,
    /// Align concentrically with a cylindrical face.
    Concentric,
    /// Tangent to a curved surface.
    Tangent,
    /// Free translation with no alignment.
    FreeTranslation,
}

/// Computes an attachment transform for placing an object onto a face.
///
/// The transform aligns the local Z axis with the face normal at the centroid.
/// `offset` displaces along the normal direction.
pub fn compute_attachment(
    model: &BRepModel,
    face: Handle<FaceData>,
    mode: AttachmentMode,
    offset: f64,
) -> KernelResult<Mat4> {
    let verts = model.vertices_of_face(face)?;
    if verts.is_empty() {
        return Err(KernelError::InvalidArgument("face has no vertices".into()));
    }

    // Compute face centroid
    let mut cx = 0.0;
    let mut cy = 0.0;
    let mut cz = 0.0;
    let n = verts.len() as f64;
    for &vh in &verts {
        let pt = model
            .vertices
            .get(vh)
            .ok_or(KernelError::InvalidHandle("vertex"))?
            .point;
        cx += pt.x;
        cy += pt.y;
        cz += pt.z;
    }
    cx /= n;
    cy /= n;
    cz /= n;

    // Compute face normal from first 3 vertices
    if verts.len() < 3 {
        return Ok(Mat4::translation(Vec3::new(cx, cy, cz)));
    }
    let p0 = model
        .vertices
        .get(verts[0])
        .map(|v| v.point)
        .unwrap_or(Point3::ORIGIN);
    let p1 = model
        .vertices
        .get(verts[1])
        .map(|v| v.point)
        .unwrap_or(Point3::ORIGIN);
    let p2 = model
        .vertices
        .get(verts[2])
        .map(|v| v.point)
        .unwrap_or(Point3::ORIGIN);
    let edge1 = p1 - p0;
    let edge2 = p2 - p0;
    let normal_raw = Vec3::new(
        edge1.y * edge2.z - edge1.z * edge2.y,
        edge1.z * edge2.x - edge1.x * edge2.z,
        edge1.x * edge2.y - edge1.y * edge2.x,
    );
    let normal = normal_raw.normalized().unwrap_or(Vec3::Z);

    match mode {
        AttachmentMode::FlatFace | AttachmentMode::Concentric | AttachmentMode::Tangent => {
            // Build frame: Z = normal, X/Y = tangent plane
            let up = if normal.x.abs() < 0.9 {
                Vec3::X
            } else {
                Vec3::Y
            };
            let x_axis_raw = Vec3::new(
                up.y * normal.z - up.z * normal.y,
                up.z * normal.x - up.x * normal.z,
                up.x * normal.y - up.y * normal.x,
            );
            let x_axis = x_axis_raw.normalized().unwrap_or(Vec3::X);
            let y_axis = Vec3::new(
                normal.y * x_axis.z - normal.z * x_axis.y,
                normal.z * x_axis.x - normal.x * x_axis.z,
                normal.x * x_axis.y - normal.y * x_axis.x,
            );

            Ok(Mat4::from_rows(
                [x_axis.x, y_axis.x, normal.x, cx + normal.x * offset],
                [x_axis.y, y_axis.y, normal.y, cy + normal.y * offset],
                [x_axis.z, y_axis.z, normal.z, cz + normal.z * offset],
                [0.0, 0.0, 0.0, 1.0],
            ))
        }
        AttachmentMode::NormalToEdge => {
            // Use the first edge direction as alignment reference
            let edge_dir = (p1 - p0).normalized().unwrap_or(Vec3::X);
            let y_axis = Vec3::new(
                normal.y * edge_dir.z - normal.z * edge_dir.y,
                normal.z * edge_dir.x - normal.x * edge_dir.z,
                normal.x * edge_dir.y - normal.y * edge_dir.x,
            )
            .normalized()
            .unwrap_or(Vec3::Y);

            Ok(Mat4::from_rows(
                [edge_dir.x, y_axis.x, normal.x, cx + normal.x * offset],
                [edge_dir.y, y_axis.y, normal.y, cy + normal.y * offset],
                [edge_dir.z, y_axis.z, normal.z, cz + normal.z * offset],
                [0.0, 0.0, 0.0, 1.0],
            ))
        }
        AttachmentMode::FreeTranslation => Ok(Mat4::translation(Vec3::new(
            cx + normal.x * offset,
            cy + normal.y * offset,
            cz + normal.z * offset,
        ))),
    }
}

/// Improved defeaturing: removes specified faces and rebuilds the solid.
///
/// All faces except those in `face_indices` are retained. The face indices
/// refer to position in the shell's face list.
pub fn defeaturing_remove_faces(
    model: &mut BRepModel,
    solid: Handle<SolidData>,
    face_indices: &[usize],
) -> KernelResult<Handle<SolidData>> {
    let op = model.history.next_operation("defeaturing_remove_faces");
    let all_faces = collect_solid_faces(model, solid)?;

    if face_indices.is_empty() {
        return Err(KernelError::InvalidArgument(
            "no face indices provided for removal".into(),
        ));
    }

    let remove_set: std::collections::HashSet<usize> = face_indices.iter().copied().collect();

    let remaining: Vec<Handle<FaceData>> = all_faces
        .iter()
        .enumerate()
        .filter(|(i, _)| !remove_set.contains(i))
        .map(|(_, h)| *h)
        .collect();

    if remaining.is_empty() {
        return Err(KernelError::InvalidArgument(
            "cannot remove all faces from a solid".into(),
        ));
    }

    if remaining.len() == all_faces.len() {
        return Err(KernelError::InvalidArgument(
            "none of the specified face indices exist in the solid".into(),
        ));
    }

    rebuild_solid_from_faces(model, &remaining, op)
}

fn rebuild_solid_from_faces(
    model: &mut BRepModel,
    faces: &[Handle<FaceData>],
    op: cadkernel_topology::OperationId,
) -> KernelResult<Handle<SolidData>> {
    use std::collections::HashMap;

    let mut vert_map: HashMap<u32, Handle<cadkernel_topology::VertexData>> = HashMap::new();
    let mut vert_idx = 0u32;

    for &fh in faces {
        let verts = model.vertices_of_face(fh)?;
        for &vh in &verts {
            let key = vh.index();
            if vert_map.contains_key(&key) {
                continue;
            }
            let pt = model
                .vertices
                .get(vh)
                .ok_or(KernelError::InvalidHandle("vertex"))?
                .point;
            let tag = Tag::generated(EntityKind::Vertex, op, vert_idx);
            vert_map.insert(key, model.add_vertex_tagged(pt, tag));
            vert_idx += 1;
        }
    }

    let mut new_faces = Vec::with_capacity(faces.len());
    let mut edge_idx = 0u32;

    for (fi, &fh) in faces.iter().enumerate() {
        let verts = model.vertices_of_face(fh)?;
        let new_verts: Vec<Handle<cadkernel_topology::VertexData>> =
            verts.iter().map(|vh| vert_map[&vh.index()]).collect();

        let nv = new_verts.len();
        let mut half_edges = Vec::with_capacity(nv);

        for j in 0..nv {
            let v_start = new_verts[j];
            let v_end = new_verts[(j + 1) % nv];
            let tag_e = Tag::generated(EntityKind::Edge, op, edge_idx);
            edge_idx += 1;
            let (_edge_h, he_a, _he_b) = model.add_edge_tagged(v_start, v_end, tag_e);
            half_edges.push(he_a);
        }

        let loop_h = model.make_loop(&half_edges)?;
        let tag_f = Tag::generated(EntityKind::Face, op, fi as u32);
        new_faces.push(model.make_face_tagged(loop_h, tag_f));
    }

    let tag_sh = Tag::generated(EntityKind::Shell, op, 0);
    let shell_h = model.make_shell_tagged(&new_faces, tag_sh);
    let tag_so = Tag::generated(EntityKind::Solid, op, 0);
    Ok(model.make_solid_tagged(&[shell_h], tag_so))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_math::Point3;

    #[test]
    fn test_face_appearance_new() {
        let fa = FaceAppearance::new(0.5, 0.6, 0.7);
        assert!((fa.color.0 - 0.5).abs() < 1e-10);
        assert!((fa.color.1 - 0.6).abs() < 1e-10);
        assert!((fa.color.2 - 0.7).abs() < 1e-10);
        assert!((fa.transparency - 0.0).abs() < 1e-10);
        assert!(fa.material_name.is_none());
    }

    #[test]
    fn test_face_appearance_builder() {
        let fa = FaceAppearance::new(1.0, 0.0, 0.0)
            .with_transparency(0.5)
            .with_material("Steel");
        assert!((fa.transparency - 0.5).abs() < 1e-10);
        assert_eq!(fa.material_name, Some("Steel".to_string()));
    }

    #[test]
    fn test_face_appearance_clamp() {
        let fa = FaceAppearance::new(2.0, -1.0, 0.5).with_transparency(3.0);
        assert!((fa.color.0 - 1.0).abs() < 1e-10);
        assert!((fa.color.1 - 0.0).abs() < 1e-10);
        assert!((fa.transparency - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_face_appearance_map() {
        let mut map = FaceAppearanceMap::new();
        assert!(map.is_empty());

        set_face_appearance(&mut map, 0, FaceAppearance::new(1.0, 0.0, 0.0)).unwrap();
        set_face_appearance(&mut map, 2, FaceAppearance::new(0.0, 1.0, 0.0)).unwrap();

        assert_eq!(map.len(), 2);
        assert!(get_face_appearance(&map, 0).is_some());
        assert!(get_face_appearance(&map, 1).is_none());
        assert!(get_face_appearance(&map, 2).is_some());

        let removed = map.remove(0);
        assert!(removed.is_some());
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_attachment_flat_face() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 3.0, 4.0).unwrap();

        let face = b.faces[0];
        let result = compute_attachment(&model, face, AttachmentMode::FlatFace, 0.0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_attachment_with_offset() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 3.0, 4.0).unwrap();

        let face = b.faces[0];
        let m1 = compute_attachment(&model, face, AttachmentMode::FlatFace, 0.0).unwrap();
        let m2 = compute_attachment(&model, face, AttachmentMode::FlatFace, 5.0).unwrap();

        // Translation column should differ by 5 units along the normal
        let dx = m2.0[(0, 3)] - m1.0[(0, 3)];
        let dy = m2.0[(1, 3)] - m1.0[(1, 3)];
        let dz = m2.0[(2, 3)] - m1.0[(2, 3)];
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();
        assert!((dist - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_attachment_normal_to_edge() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 3.0, 4.0).unwrap();
        let result = compute_attachment(&model, b.faces[0], AttachmentMode::NormalToEdge, 0.0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_attachment_free_translation() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 3.0, 4.0).unwrap();
        let result = compute_attachment(&model, b.faces[0], AttachmentMode::FreeTranslation, 1.0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_defeaturing_remove_faces() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 3.0, 4.0).unwrap();

        let new_solid = defeaturing_remove_faces(&mut model, b.solid, &[0, 1]).unwrap();

        let sd = model.solids.get(new_solid).unwrap();
        let shell = model.shells.get(sd.shells[0]).unwrap();
        assert_eq!(shell.faces.len(), 4);
    }

    #[test]
    fn test_defeaturing_empty_indices() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 3.0, 4.0).unwrap();

        let result = defeaturing_remove_faces(&mut model, b.solid, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_defeaturing_all_faces() {
        let mut model = BRepModel::new();
        let b = crate::make_box(&mut model, Point3::ORIGIN, 2.0, 3.0, 4.0).unwrap();

        let result = defeaturing_remove_faces(&mut model, b.solid, &[0, 1, 2, 3, 4, 5]);
        assert!(result.is_err());
    }
}
