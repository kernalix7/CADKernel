//! Integration tests for real-world import/export roundtrips.
//!
//! Tests complex multi-solid models, multi-face topology, and large vertex
//! counts across STEP, IGES, STL, OBJ, PLY, BREP, JSON, and native formats
//! to verify data integrity through the full export → import cycle.

use cadkernel_io::*;
use cadkernel_io::tessellate::{tessellate_solid, tessellate_solid_parallel, tessellate_solid_with_face_map};
use cadkernel_math::{Point3, Vec3};
use cadkernel_modeling::primitives::BoxResult;
use cadkernel_modeling::make_box;
use cadkernel_topology::BRepModel;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_test_box(origin: Point3, w: f64, h: f64, d: f64) -> (BRepModel, BoxResult) {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, origin, w, h, d).unwrap();
    (model, r)
}

fn tessellate_box_mesh(model: &BRepModel, solid: cadkernel_topology::Handle<cadkernel_topology::SolidData>) -> Mesh {
    tessellate_solid(model, solid)
}

// ---------------------------------------------------------------------------
// STEP roundtrip tests
// ---------------------------------------------------------------------------

#[test]
fn step_roundtrip_single_box() {
    let (model, _r) = make_test_box(Point3::ORIGIN, 10.0, 20.0, 30.0);
    let step_str = export_step(&model).unwrap();

    assert!(step_str.contains("ISO-10303-21"));
    assert!(step_str.contains("MANIFOLD_SOLID_BREP"));
    assert!(step_str.contains("CLOSED_SHELL"));
    assert!(step_str.contains("ADVANCED_FACE"));

    let imported = import_step(&step_str).unwrap();
    assert!(imported.vertices.iter().count() >= 8);
}

#[test]
fn step_roundtrip_multi_solid() {
    let mut model = BRepModel::new();
    let _r1 = make_box(&mut model, Point3::ORIGIN, 5.0, 5.0, 5.0).unwrap();
    let _r2 = make_box(&mut model, Point3::new(10.0, 0.0, 0.0), 3.0, 3.0, 3.0).unwrap();
    let _r3 = make_box(&mut model, Point3::new(0.0, 10.0, 0.0), 7.0, 2.0, 4.0).unwrap();

    let step_str = export_step(&model).unwrap();
    let imported = import_step(&step_str).unwrap();

    let orig_verts: usize = model.vertices.iter().count();
    let imported_verts: usize = imported.vertices.iter().count();
    assert!(imported_verts >= orig_verts, "vertex count: {} < {}", imported_verts, orig_verts);
}

#[test]
fn step_preserves_vertex_coordinates() {
    let (model, _r) = make_test_box(Point3::new(100.0, 200.0, 300.0), 1.0, 1.0, 1.0);
    let step_str = export_step(&model).unwrap();
    let points = read_step_points(&step_str).unwrap();

    assert!(points.len() >= 8);
    for p in &points {
        assert!(p.x >= 99.0 && p.x <= 102.0, "x out of range: {}", p.x);
        assert!(p.y >= 199.0 && p.y <= 202.0, "y out of range: {}", p.y);
        assert!(p.z >= 299.0 && p.z <= 302.0, "z out of range: {}", p.z);
    }
}

#[test]
fn step_typed_entity_parsing() {
    let (model, _r) = make_test_box(Point3::ORIGIN, 5.0, 5.0, 5.0);
    let step_str = export_step(&model).unwrap();
    let file = parse_step(&step_str).unwrap();

    let mut has_point = false;
    let mut has_direction = false;
    let mut has_vertex = false;
    let mut has_face = false;
    let mut has_shell = false;
    let mut has_solid = false;

    for entity in file.entities.values() {
        match entity {
            StepEntity::CartesianPoint(_) => has_point = true,
            StepEntity::Direction(_) => has_direction = true,
            StepEntity::VertexPoint(_) => has_vertex = true,
            StepEntity::AdvancedFace { .. } => has_face = true,
            StepEntity::ClosedShell { .. } => has_shell = true,
            StepEntity::ManifoldSolidBrep { .. } => has_solid = true,
            _ => {}
        }
    }

    assert!(has_point, "missing CartesianPoint");
    assert!(has_direction, "missing Direction");
    assert!(has_vertex, "missing VertexPoint");
    assert!(has_face, "missing AdvancedFace");
    assert!(has_shell, "missing ClosedShell");
    assert!(has_solid, "missing ManifoldSolidBrep");
}

#[test]
fn step_large_model_roundtrip() {
    let mut model = BRepModel::new();
    for i in 0..20 {
        let x = (i % 5) as f64 * 10.0;
        let y = (i / 5) as f64 * 10.0;
        make_box(&mut model, Point3::new(x, y, 0.0), 4.0, 4.0, 4.0).unwrap();
    }

    let step_str = export_step(&model).unwrap();
    assert!(step_str.len() > 1000);

    let imported = import_step(&step_str).unwrap();
    let orig_count: usize = model.vertices.iter().count();
    let imported_count: usize = imported.vertices.iter().count();
    assert!(imported_count >= orig_count);
}

// ---------------------------------------------------------------------------
// IGES roundtrip tests
// ---------------------------------------------------------------------------

#[test]
fn iges_roundtrip_wireframe() {
    let mut model = BRepModel::new();
    let v0 = model.add_vertex(Point3::new(0.0, 0.0, 0.0));
    let v1 = model.add_vertex(Point3::new(10.0, 0.0, 0.0));
    let v2 = model.add_vertex(Point3::new(10.0, 10.0, 0.0));
    let v3 = model.add_vertex(Point3::new(0.0, 10.0, 0.0));
    model.add_edge(v0, v1);
    model.add_edge(v1, v2);
    model.add_edge(v2, v3);
    model.add_edge(v3, v0);

    let iges_str = export_iges(&model).unwrap();
    let entities = parse_iges(&iges_str).unwrap();

    let point_count = entities.iter().filter(|e| e.entity_type == IgesEntityType::Point).count();
    let line_count = entities.iter().filter(|e| e.entity_type == IgesEntityType::Line).count();
    assert_eq!(point_count, 4);
    assert_eq!(line_count, 4);

    let imported = import_iges(&iges_str).unwrap();
    assert!(imported.vertices.iter().count() >= 4);
}

#[test]
fn iges_preserves_point_coordinates() {
    let mut model = BRepModel::new();
    model.add_vertex(Point3::new(50.5, 100.25, -75.125));
    model.add_vertex(Point3::new(-30.0, 0.0, 200.0));

    let iges_str = export_iges(&model).unwrap();
    let points = read_iges_points(&iges_str).unwrap();
    assert_eq!(points.len(), 2);

    let p0 = &points[0];
    assert!((p0.x - 50.5).abs() < 1e-6, "x mismatch: {}", p0.x);
    assert!((p0.y - 100.25).abs() < 1e-6, "y mismatch: {}", p0.y);
    assert!((p0.z + 75.125).abs() < 1e-6, "z mismatch: {}", p0.z);
}

#[test]
fn iges_mesh_export_triangle_edges() {
    let mesh = Mesh {
        vertices: vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.5, 1.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        ],
        normals: vec![Vec3::Z, Vec3::Z, Vec3::Z, Vec3::Z],
        indices: vec![[0, 1, 2], [1, 3, 2]],
    };

    let output = export_iges_mesh(&mesh).unwrap();
    let entities = parse_iges(&output).unwrap();

    let point_count = entities.iter().filter(|e| e.entity_type == IgesEntityType::Point).count();
    assert_eq!(point_count, 4);
}

// ---------------------------------------------------------------------------
// STL roundtrip tests
// ---------------------------------------------------------------------------

#[test]
fn stl_ascii_roundtrip() {
    let (model, r) = make_test_box(Point3::ORIGIN, 5.0, 5.0, 5.0);
    let mesh = tessellate_box_mesh(&model, r.solid);
    let ascii = write_stl_ascii(&mesh, "test_box");

    assert!(ascii.contains("solid"));
    assert!(ascii.contains("facet normal"));

    let reimported = read_stl_ascii(&ascii).unwrap();
    assert_eq!(reimported.indices.len(), mesh.indices.len());
}

#[test]
fn stl_binary_roundtrip() {
    let (model, r) = make_test_box(Point3::ORIGIN, 5.0, 5.0, 5.0);
    let mesh = tessellate_box_mesh(&model, r.solid);
    let binary = write_stl_binary(&mesh).unwrap();

    assert!(binary.len() > 84);

    let reimported = read_stl_binary(&binary).unwrap();
    assert_eq!(reimported.indices.len(), mesh.indices.len());
}

// ---------------------------------------------------------------------------
// OBJ roundtrip tests
// ---------------------------------------------------------------------------

#[test]
fn obj_roundtrip() {
    let (model, r) = make_test_box(Point3::ORIGIN, 3.0, 4.0, 5.0);
    let mesh = tessellate_box_mesh(&model, r.solid);
    let obj_str = write_obj(&mesh);

    assert!(obj_str.contains("v "));
    assert!(obj_str.contains("f "));

    let reimported = read_obj(&obj_str).unwrap();
    assert_eq!(reimported.vertices.len(), mesh.vertices.len());
}

// ---------------------------------------------------------------------------
// PLY roundtrip tests
// ---------------------------------------------------------------------------

#[test]
fn ply_roundtrip() {
    let (model, r) = make_test_box(Point3::ORIGIN, 2.0, 2.0, 2.0);
    let mesh = tessellate_box_mesh(&model, r.solid);
    let ply_str = export_ply(&mesh).unwrap();

    assert!(ply_str.contains("ply"));
    assert!(ply_str.contains("element vertex"));

    let reimported = import_ply(&ply_str).unwrap();
    assert_eq!(reimported.vertices.len(), mesh.vertices.len());
}

// ---------------------------------------------------------------------------
// BREP roundtrip tests
// ---------------------------------------------------------------------------

#[test]
fn brep_roundtrip() {
    let (model, _r) = make_test_box(Point3::ORIGIN, 6.0, 7.0, 8.0);
    let brep_str = export_brep(&model).unwrap();
    let reimported = import_brep(&brep_str).unwrap();

    let orig_verts: usize = model.vertices.iter().count();
    let imported_verts: usize = reimported.vertices.iter().count();
    assert_eq!(imported_verts, orig_verts);
}

#[test]
fn brep_multi_solid_roundtrip() {
    let mut model = BRepModel::new();
    make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
    make_box(&mut model, Point3::new(5.0, 0.0, 0.0), 2.0, 2.0, 2.0).unwrap();

    let brep_str = export_brep(&model).unwrap();
    let reimported = import_brep(&brep_str).unwrap();

    let orig_solids: usize = model.solids.iter().count();
    let imported_solids: usize = reimported.solids.iter().count();
    assert_eq!(imported_solids, orig_solids, "solid count mismatch");
}

// ---------------------------------------------------------------------------
// JSON roundtrip tests
// ---------------------------------------------------------------------------

#[test]
fn json_roundtrip() {
    let (model, _r) = make_test_box(Point3::ORIGIN, 4.0, 4.0, 4.0);
    let json_str = model_to_json(&model).unwrap();
    let reimported = model_from_json(&json_str).unwrap();

    let orig_verts: usize = model.vertices.iter().count();
    let imported_verts: usize = reimported.vertices.iter().count();
    assert_eq!(imported_verts, orig_verts);
}

// ---------------------------------------------------------------------------
// Cross-format consistency tests
// ---------------------------------------------------------------------------

#[test]
fn mesh_vertex_count_consistent_across_formats() {
    let (model, r) = make_test_box(Point3::ORIGIN, 1.0, 1.0, 1.0);
    let mesh = tessellate_box_mesh(&model, r.solid);
    let tri_count = mesh.indices.len();

    let stl_ascii = write_stl_ascii(&mesh, "test");
    let stl_mesh = read_stl_ascii(&stl_ascii).unwrap();
    assert_eq!(stl_mesh.indices.len(), tri_count, "STL triangle count mismatch");

    let obj_str = write_obj(&mesh);
    let obj_mesh = read_obj(&obj_str).unwrap();
    assert_eq!(obj_mesh.vertices.len(), mesh.vertices.len(), "OBJ vertex count mismatch");
}

// ---------------------------------------------------------------------------
// Tessellation integration tests
// ---------------------------------------------------------------------------

#[test]
fn tessellation_produces_valid_mesh() {
    let (model, r) = make_test_box(Point3::ORIGIN, 1.0, 1.0, 1.0);
    let mesh = tessellate_box_mesh(&model, r.solid);

    assert!(!mesh.vertices.is_empty());
    assert!(!mesh.indices.is_empty());
    assert!(!mesh.normals.is_empty());

    let max_idx = mesh.vertices.len() as u32;
    for tri in &mesh.indices {
        for &idx in tri {
            assert!(idx < max_idx, "invalid index {} >= {}", idx, max_idx);
        }
    }
}

#[test]
fn parallel_tessellation_produces_valid_mesh() {
    let (model, r) = make_test_box(Point3::ORIGIN, 2.0, 3.0, 4.0);
    let serial = tessellate_solid(&model, r.solid);
    let parallel = tessellate_solid_parallel(&model, r.solid);

    // Both must produce non-empty valid meshes
    assert!(!serial.indices.is_empty(), "serial mesh empty");
    assert!(!parallel.indices.is_empty(), "parallel mesh empty");

    // Index bounds check
    for mesh in [&serial, &parallel] {
        let max_idx = mesh.vertices.len() as u32;
        for tri in &mesh.indices {
            for &idx in tri {
                assert!(idx < max_idx, "invalid index {} >= {}", idx, max_idx);
            }
        }
    }
}

#[test]
fn face_map_tessellation() {
    let (model, r) = make_test_box(Point3::ORIGIN, 1.0, 1.0, 1.0);
    let (mesh, face_map) = tessellate_solid_with_face_map(&model, r.solid);

    assert!(!face_map.is_empty(), "face_map should not be empty");
    assert!(!mesh.indices.is_empty());

    let mapped_total: usize = face_map.iter().map(|(_, _, count)| count).sum();
    assert_eq!(mapped_total, mesh.indices.len(),
        "face_map total {} != mesh triangles {}", mapped_total, mesh.indices.len());
}

// ---------------------------------------------------------------------------
// Mesh operations integration tests
// ---------------------------------------------------------------------------

#[test]
fn mesh_decimation_reduces_triangles() {
    let (model, r) = make_test_box(Point3::ORIGIN, 1.0, 1.0, 1.0);
    let mesh = tessellate_box_mesh(&model, r.solid);
    let original_count = mesh.indices.len();

    let decimated = cadkernel_io::mesh_ops::decimate_mesh(&mesh, 0.5).unwrap();
    assert!(decimated.indices.len() <= original_count,
        "decimated ({}) should have <= original ({})", decimated.indices.len(), original_count);
}

#[test]
fn mesh_subdivision_increases_triangles() {
    let (model, r) = make_test_box(Point3::ORIGIN, 1.0, 1.0, 1.0);
    let mesh = tessellate_box_mesh(&model, r.solid);
    let original_count = mesh.indices.len();

    let subdivided = cadkernel_io::mesh_ops::subdivide_mesh(&mesh).unwrap();
    assert!(subdivided.indices.len() >= original_count,
        "subdivided ({}) should have >= original ({})", subdivided.indices.len(), original_count);
}

#[test]
fn mesh_smooth_preserves_vertex_count() {
    let (model, r) = make_test_box(Point3::ORIGIN, 1.0, 1.0, 1.0);
    let mesh = tessellate_box_mesh(&model, r.solid);

    let smoothed = cadkernel_io::mesh_ops::smooth_mesh(&mesh, 3, 0.5);
    assert_eq!(smoothed.vertices.len(), mesh.vertices.len(),
        "smoothing should preserve vertex count");
    assert_eq!(smoothed.indices.len(), mesh.indices.len(),
        "smoothing should preserve triangle count");
}
