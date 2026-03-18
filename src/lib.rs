//! CADKernel — a modular B-Rep CAD kernel in Rust.
//!
//! Re-exports every sub-crate (core, math, geometry, topology, modeling, sketch, io)
//! and provides a unified [`prelude`] module for convenient glob imports.

/// Shared error types and foundational traits.
pub use cadkernel_core as core;

/// Core math primitives: points, vectors, matrices, transforms, bounding boxes.
pub use cadkernel_math as math;

/// Geometric primitives: curves (line, circle, arc, NURBS) and surfaces (plane, cylinder, sphere, NURBS).
pub use cadkernel_geometry as geometry;

/// B-Rep topology: half-edge data structure, entity stores, and topological elements.
pub use cadkernel_topology as topology;

/// Modeling operations: primitive builders, boolean operations, feature ops.
pub use cadkernel_modeling as modeling;

/// 2D parametric sketch with constraint solver.
pub use cadkernel_sketch as sketch;

/// File I/O: tessellation, STL/OBJ export.
pub use cadkernel_io as io;

/// Unified prelude: import everything you need with `use cadkernel::prelude::*`.
pub mod prelude {
    // Core
    pub use cadkernel_core::{KernelError, KernelResult};

    // Math
    pub use cadkernel_math::prelude::*;

    // Geometry (including adaptive tessellation)
    pub use cadkernel_geometry::prelude::*;

    // Topology
    pub use cadkernel_topology::prelude::*;

    // Modeling
    pub use cadkernel_modeling::{
        BooleanOp, ChamferResult, ClosestPointResult, Containment, DraftResult, ExtrudeResult,
        FilletResult, LoftResult, MassProperties, MirrorResult, PatternResult, RevolveResult,
        ScaleResult, ShellResult, SplitResult, SweepResult, boolean_op, chamfer_edge,
        circular_pattern, closest_point_on_solid, compute_mass_properties, draft_faces, extrude,
        fillet_edge, linear_pattern, loft, make_box, make_cylinder, make_sphere, mirror_solid,
        point_in_solid, revolve, scale_solid, shell_solid, solid_mass_properties, split_solid,
        sweep,
    };

    // Sketch
    pub use cadkernel_sketch::{
        Constraint, Sketch, SolverResult, WorkPlane, extract_profile, solve,
    };

    // IO
    pub use cadkernel_io::{
        IgesEntity, IgesEntityType, IgesWriter, Mesh, StepWriter, SvgDocument, SvgElement,
        SvgStyle, Triangle, export_gltf, export_json, export_obj, export_step_mesh,
        export_stl_ascii, export_stl_binary, import_json, import_obj, import_stl, load_project,
        model_from_json, model_to_json, parse_step_entities, profile_to_svg, read_iges_lines,
        read_iges_points, read_json, read_obj, read_step_points, read_stl_ascii, read_stl_binary,
        save_project, tessellate_face, tessellate_solid, write_gltf, write_json, write_obj,
        write_stl_ascii, write_stl_binary,
    };
}

/// Application name used in banners and user-facing output.
pub const APP_NAME: &str = "CADKernel";

/// Returns a formatted version banner string for CLI/startup display.
pub fn version_banner(version: &str) -> String {
    format!("{APP_NAME} v{version} - pre-alpha")
}

#[cfg(test)]
mod tests {
    use super::version_banner;

    #[test]
    fn version_banner_contains_version() {
        let text = version_banner("0.1.0");
        assert!(text.contains("0.1.0"));
    }

    /// Full pipeline: Sketch → Solve → Extrude → Tessellate → STL export.
    #[test]
    fn e2e_sketch_extrude_stl() {
        use super::io::{tessellate_solid, write_stl_ascii, write_stl_binary};
        use super::math::Vec3;
        use super::modeling::extrude;
        use super::sketch::{Constraint, Sketch, WorkPlane, extract_profile, solve};
        use super::topology::BRepModel;

        // 1. Build a constrained rectangle sketch
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(0.0, 0.0);
        let p1 = sketch.add_point(10.0, 0.5);
        let p2 = sketch.add_point(10.5, 5.0);
        let p3 = sketch.add_point(-0.5, 5.5);

        let l0 = sketch.add_line(p0, p1);
        let l1 = sketch.add_line(p1, p2);
        let l2 = sketch.add_line(p2, p3);
        let l3 = sketch.add_line(p3, p0);

        sketch.add_constraint(Constraint::Fixed(p0, 0.0, 0.0));
        sketch.add_constraint(Constraint::Horizontal(l0));
        sketch.add_constraint(Constraint::Vertical(l1));
        sketch.add_constraint(Constraint::Horizontal(l2));
        sketch.add_constraint(Constraint::Vertical(l3));
        sketch.add_constraint(Constraint::Length(l0, 10.0));
        sketch.add_constraint(Constraint::Length(l1, 5.0));

        // 2. Solve
        let result = solve(&mut sketch, 200, 1e-10);
        assert!(result.converged, "sketch solver failed: {:?}", result);

        // 3. Extract 3D profile on XY plane
        let wp = WorkPlane::xy();
        let profile = extract_profile(&sketch, &wp);
        assert_eq!(profile.len(), 4);

        // 4. Extrude along Z by 3 units
        let mut model = BRepModel::new();
        let ext = extrude(&mut model, &profile, Vec3::Z, 3.0).unwrap();
        assert_eq!(model.vertices.len(), 8);
        assert_eq!(model.faces.len(), 6);
        assert_eq!(model.solids.len(), 1);

        // 5. Tessellate
        let mesh = tessellate_solid(&model, ext.solid);
        // 6 quad faces → 2 triangles each = 12 triangles
        assert_eq!(mesh.triangle_count(), 12);
        assert!(mesh.vertices.len() >= 8);

        // 6. Export to ASCII STL
        let stl_ascii = write_stl_ascii(&mesh, "box_from_sketch");
        assert!(stl_ascii.contains("solid box_from_sketch"));
        assert!(stl_ascii.contains("endsolid box_from_sketch"));
        let facet_count = stl_ascii.matches("facet normal").count();
        assert_eq!(facet_count, 12);

        // 7. Export to binary STL
        let stl_bin = write_stl_binary(&mesh).unwrap();
        let tri_count = u32::from_le_bytes([stl_bin[80], stl_bin[81], stl_bin[82], stl_bin[83]]);
        assert_eq!(tri_count, 12);
        assert_eq!(stl_bin.len(), 84 + 12 * 50);
    }

    /// Full pipeline: Sketch → Solve → Revolve → Tessellate → OBJ export.
    #[test]
    fn e2e_sketch_revolve_obj() {
        use super::io::{tessellate_solid, write_obj};
        use super::math::{Point3, Vec3};
        use super::modeling::revolve;
        use super::sketch::{Constraint, Sketch, WorkPlane, extract_profile, solve};
        use super::topology::BRepModel;
        use std::f64::consts::TAU;

        // 1. L-shaped profile sketch on XZ plane
        let mut sketch = Sketch::new();
        let p0 = sketch.add_point(1.0, 0.0);
        let p1 = sketch.add_point(2.0, 0.1);
        let p2 = sketch.add_point(2.0, 3.0);

        let l0 = sketch.add_line(p0, p1);
        let l1 = sketch.add_line(p1, p2);

        sketch.add_constraint(Constraint::Fixed(p0, 1.0, 0.0));
        sketch.add_constraint(Constraint::Horizontal(l0));
        sketch.add_constraint(Constraint::Vertical(l1));
        sketch.add_constraint(Constraint::Length(l0, 1.0));
        sketch.add_constraint(Constraint::Length(l1, 3.0));

        // 2. Solve
        let result = solve(&mut sketch, 200, 1e-10);
        assert!(result.converged);

        // 3. Extract profile on XZ plane (Y is the revolve axis)
        let wp = WorkPlane::xz();
        let profile = extract_profile(&sketch, &wp);
        assert_eq!(profile.len(), 3);

        // 4. Revolve 360° around Y axis
        let mut model = BRepModel::new();
        let rev = revolve(&mut model, &profile, Point3::ORIGIN, Vec3::Y, TAU, 12).unwrap();
        assert_eq!(model.solids.len(), 1);

        // 5. Tessellate
        let mesh = tessellate_solid(&model, rev.solid);
        assert!(mesh.triangle_count() > 0);

        // 6. Export OBJ
        let obj = write_obj(&mesh);
        assert!(obj.starts_with("# CADKernel OBJ export\n"));
        let v_count = obj.lines().filter(|l| l.starts_with("v ")).count();
        assert!(v_count > 0);
        let f_count = obj.lines().filter(|l| l.starts_with("f ")).count();
        assert_eq!(f_count, mesh.triangle_count());
    }

    /// Verify that persistent tags survive the full pipeline.
    #[test]
    fn e2e_persistent_naming_through_extrude() {
        use super::math::{Point3, Vec3};
        use super::modeling::extrude;
        use super::topology::{BRepModel, EntityKind, Tag};

        let profile = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
        ];

        let mut model = BRepModel::new();
        let ext = extrude(&mut model, &profile, Vec3::Z, 2.0).unwrap();

        let op = model.history.records()[0].operation;

        // Bottom face tag = 0, top face tag = 1
        let bottom_tag = Tag::generated(EntityKind::Face, op, 0);
        let top_tag = Tag::generated(EntityKind::Face, op, 1);
        assert_eq!(model.find_face_by_tag(&bottom_tag), Some(ext.bottom_face));
        assert_eq!(model.find_face_by_tag(&top_tag), Some(ext.top_face));

        // All 6 vertices on the bottom should be at z=0
        for i in 0..3u32 {
            let vtag = Tag::generated(EntityKind::Vertex, op, i);
            let vh = model.find_vertex_by_tag(&vtag).unwrap();
            let v = model.vertices.get(vh).unwrap();
            assert!(v.point.z.abs() < 1e-8);
        }

        // All 3 vertices on the top should be at z=2
        for i in 0..3u32 {
            let vtag = Tag::generated(EntityKind::Vertex, op, 3 + i);
            let vh = model.find_vertex_by_tag(&vtag).unwrap();
            let v = model.vertices.get(vh).unwrap();
            assert!((v.point.z - 2.0).abs() < 1e-8);
        }
    }
}
