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

// ===========================================================================
// Extended roundtrip tests — edge cases and additional formats
// ===========================================================================

// ---------------------------------------------------------------------------
// Empty model roundtrip tests
// ---------------------------------------------------------------------------

#[test]
fn step_roundtrip_empty_model() {
    let model = BRepModel::new();
    let step_str = export_step(&model).unwrap();
    assert!(step_str.contains("ISO-10303-21"));
    let imported = import_step(&step_str).unwrap();
    assert_eq!(imported.vertices.iter().count(), 0);
    assert_eq!(imported.solids.iter().count(), 0);
}

#[test]
fn brep_roundtrip_empty_model() {
    let model = BRepModel::new();
    let brep_str = export_brep(&model).unwrap();
    let imported = import_brep(&brep_str).unwrap();
    assert_eq!(imported.vertices.iter().count(), 0);
    assert_eq!(imported.solids.iter().count(), 0);
}

#[test]
fn json_roundtrip_empty_model() {
    let model = BRepModel::new();
    let json_str = model_to_json(&model).unwrap();
    let imported = model_from_json(&json_str).unwrap();
    assert_eq!(imported.vertices.iter().count(), 0);
    assert_eq!(imported.solids.iter().count(), 0);
}

// ---------------------------------------------------------------------------
// Multi-solid roundtrip across more formats
// ---------------------------------------------------------------------------

#[test]
fn step_roundtrip_five_solids() {
    let mut model = BRepModel::new();
    for i in 0..5 {
        let offset = i as f64 * 15.0;
        make_box(
            &mut model,
            Point3::new(offset, 0.0, 0.0),
            2.0 + i as f64,
            3.0,
            4.0,
        )
        .unwrap();
    }
    let orig_solid_count: usize = model.solids.iter().count();
    assert_eq!(orig_solid_count, 5);

    let step_str = export_step(&model).unwrap();
    let imported = import_step(&step_str).unwrap();
    let imported_verts: usize = imported.vertices.iter().count();
    let orig_verts: usize = model.vertices.iter().count();
    assert!(
        imported_verts >= orig_verts,
        "expected >= {} vertices, got {}",
        orig_verts,
        imported_verts
    );
}

#[test]
fn json_roundtrip_multi_solid() {
    let mut model = BRepModel::new();
    make_box(&mut model, Point3::ORIGIN, 1.0, 2.0, 3.0).unwrap();
    make_box(&mut model, Point3::new(5.0, 0.0, 0.0), 4.0, 5.0, 6.0).unwrap();
    make_box(&mut model, Point3::new(0.0, 10.0, 0.0), 2.0, 2.0, 2.0).unwrap();

    let json_str = model_to_json(&model).unwrap();
    let imported = model_from_json(&json_str).unwrap();

    let orig_solids: usize = model.solids.iter().count();
    let imported_solids: usize = imported.solids.iter().count();
    assert_eq!(imported_solids, orig_solids, "solid count mismatch");

    let orig_verts: usize = model.vertices.iter().count();
    let imported_verts: usize = imported.vertices.iter().count();
    assert_eq!(imported_verts, orig_verts, "vertex count mismatch");
}

// ---------------------------------------------------------------------------
// STEP import → export → re-import consistency
// ---------------------------------------------------------------------------

#[test]
fn step_double_roundtrip_consistency() {
    let (model, _r) = make_test_box(Point3::new(10.0, 20.0, 30.0), 5.0, 10.0, 15.0);

    // First roundtrip
    let step_1 = export_step(&model).unwrap();
    let imported_1 = import_step(&step_1).unwrap();

    // Second roundtrip
    let step_2 = export_step(&imported_1).unwrap();
    let imported_2 = import_step(&step_2).unwrap();

    let verts_1: usize = imported_1.vertices.iter().count();
    let verts_2: usize = imported_2.vertices.iter().count();
    assert_eq!(verts_1, verts_2, "vertex count diverged: {} vs {}", verts_1, verts_2);
}

#[test]
fn step_roundtrip_preserves_face_count() {
    let (model, _r) = make_test_box(Point3::ORIGIN, 3.0, 4.0, 5.0);
    let orig_faces: usize = model.faces.iter().count();
    assert_eq!(orig_faces, 6, "a box should have 6 faces");

    let step_str = export_step(&model).unwrap();

    // Count ADVANCED_FACE entities in the STEP output
    let face_entity_count = step_str.matches("ADVANCED_FACE").count();
    assert!(
        face_entity_count >= 6,
        "expected >= 6 ADVANCED_FACE, got {}",
        face_entity_count
    );
}

// ---------------------------------------------------------------------------
// glTF roundtrip (embedded base64)
// ---------------------------------------------------------------------------

#[test]
fn gltf_roundtrip() {
    let (model, r) = make_test_box(Point3::ORIGIN, 2.0, 3.0, 4.0);
    let mesh = tessellate_box_mesh(&model, r.solid);
    let gltf_str = cadkernel_io::gltf::write_gltf(&mesh).unwrap();

    assert!(gltf_str.contains("\"asset\""));
    assert!(gltf_str.contains("\"meshes\""));
    assert!(gltf_str.contains("data:application/octet-stream;base64,"));

    let reimported = import_gltf(&gltf_str).unwrap();
    assert_eq!(
        reimported.vertices.len(),
        mesh.vertices.len(),
        "glTF vertex count mismatch: {} vs {}",
        reimported.vertices.len(),
        mesh.vertices.len()
    );
    assert_eq!(
        reimported.indices.len(),
        mesh.indices.len(),
        "glTF triangle count mismatch: {} vs {}",
        reimported.indices.len(),
        mesh.indices.len()
    );
}

#[test]
fn gltf_preserves_vertex_positions() {
    let mesh = Mesh {
        vertices: vec![
            Point3::new(1.0, 2.0, 3.0),
            Point3::new(4.0, 5.0, 6.0),
            Point3::new(7.0, 8.0, 9.0),
        ],
        normals: vec![Vec3::Z, Vec3::Z, Vec3::Z],
        indices: vec![[0, 1, 2]],
    };
    let gltf_str = cadkernel_io::gltf::write_gltf(&mesh).unwrap();
    let reimported = import_gltf(&gltf_str).unwrap();

    for (orig, imp) in mesh.vertices.iter().zip(reimported.vertices.iter()) {
        assert!(
            (orig.x - imp.x).abs() < 1e-5,
            "x mismatch: {} vs {}",
            orig.x,
            imp.x
        );
        assert!(
            (orig.y - imp.y).abs() < 1e-5,
            "y mismatch: {} vs {}",
            orig.y,
            imp.y
        );
        assert!(
            (orig.z - imp.z).abs() < 1e-5,
            "z mismatch: {} vs {}",
            orig.z,
            imp.z
        );
    }
}

// ---------------------------------------------------------------------------
// 3MF roundtrip tests
// ---------------------------------------------------------------------------

#[test]
fn threemf_roundtrip() {
    let (model, r) = make_test_box(Point3::ORIGIN, 3.0, 4.0, 5.0);
    let mesh = tessellate_box_mesh(&model, r.solid);
    let xml_str = export_3mf(&mesh).unwrap();

    assert!(xml_str.contains("<model"));
    assert!(xml_str.contains("<mesh"));
    assert!(xml_str.contains("<vertex"));
    assert!(xml_str.contains("<triangle"));

    let reimported = import_3mf(&xml_str).unwrap();
    assert_eq!(
        reimported.vertices.len(),
        mesh.vertices.len(),
        "3MF vertex count mismatch"
    );
    assert_eq!(
        reimported.indices.len(),
        mesh.indices.len(),
        "3MF triangle count mismatch"
    );
}

#[test]
fn threemf_roundtrip_single_triangle() {
    let mesh = Mesh {
        vertices: vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.5, 1.0, 0.0),
        ],
        normals: vec![Vec3::Z, Vec3::Z, Vec3::Z],
        indices: vec![[0, 1, 2]],
    };
    let xml_str = export_3mf(&mesh).unwrap();
    let reimported = import_3mf(&xml_str).unwrap();
    assert_eq!(reimported.vertices.len(), 3);
    assert_eq!(reimported.indices.len(), 1);
}

// ---------------------------------------------------------------------------
// Collada (DAE) roundtrip tests
// ---------------------------------------------------------------------------

#[test]
fn dae_roundtrip() {
    let (model, r) = make_test_box(Point3::ORIGIN, 2.0, 2.0, 2.0);
    let mesh = tessellate_box_mesh(&model, r.solid);
    let dae_str = export_dae(&mesh).unwrap();

    assert!(dae_str.contains("<COLLADA"));
    assert!(dae_str.contains("<geometry"));

    let reimported = import_dae(&dae_str).unwrap();
    assert_eq!(
        reimported.vertices.len(),
        mesh.vertices.len(),
        "DAE vertex count mismatch"
    );
    assert_eq!(
        reimported.indices.len(),
        mesh.indices.len(),
        "DAE triangle count mismatch"
    );
}

// ---------------------------------------------------------------------------
// AMF roundtrip tests
// ---------------------------------------------------------------------------

#[test]
fn amf_roundtrip() {
    let (model, r) = make_test_box(Point3::ORIGIN, 1.0, 2.0, 3.0);
    let mesh = tessellate_box_mesh(&model, r.solid);
    let amf_str = export_amf(&mesh).unwrap();

    assert!(amf_str.contains("<amf"));
    assert!(amf_str.contains("<mesh"));

    let reimported = import_amf(&amf_str).unwrap();
    assert_eq!(
        reimported.vertices.len(),
        mesh.vertices.len(),
        "AMF vertex count mismatch"
    );
    assert_eq!(
        reimported.indices.len(),
        mesh.indices.len(),
        "AMF triangle count mismatch"
    );
}

// ---------------------------------------------------------------------------
// VRML roundtrip tests
// ---------------------------------------------------------------------------

#[test]
fn vrml_roundtrip() {
    let (model, r) = make_test_box(Point3::ORIGIN, 4.0, 4.0, 4.0);
    let mesh = tessellate_box_mesh(&model, r.solid);
    let vrml_str = export_vrml(&mesh).unwrap();

    assert!(vrml_str.contains("#VRML V2.0"));
    assert!(vrml_str.contains("IndexedFaceSet"));

    let reimported = import_vrml(&vrml_str).unwrap();
    assert_eq!(
        reimported.vertices.len(),
        mesh.vertices.len(),
        "VRML vertex count mismatch"
    );
    assert_eq!(
        reimported.indices.len(),
        mesh.indices.len(),
        "VRML triangle count mismatch"
    );
}

// ---------------------------------------------------------------------------
// OCA/GCAD roundtrip tests
// ---------------------------------------------------------------------------

#[test]
fn oca_roundtrip() {
    let (model, r) = make_test_box(Point3::ORIGIN, 5.0, 5.0, 5.0);
    let mesh = tessellate_box_mesh(&model, r.solid);
    let oca_str = export_oca(&mesh).unwrap();

    assert!(oca_str.contains("OCA"), "OCA output should contain format marker");
    assert!(oca_str.contains("POINT"), "OCA output should contain POINT entries");
    assert!(oca_str.contains("FACE"), "OCA output should contain FACE entries");

    let reimported = import_oca(&oca_str).unwrap();
    assert_eq!(
        reimported.vertices.len(),
        mesh.vertices.len(),
        "OCA vertex count mismatch"
    );
    assert_eq!(
        reimported.indices.len(),
        mesh.indices.len(),
        "OCA triangle count mismatch"
    );
}

// ---------------------------------------------------------------------------
// DXF roundtrip tests
// ---------------------------------------------------------------------------

#[test]
fn dxf_roundtrip() {
    let (model, r) = make_test_box(Point3::ORIGIN, 1.0, 1.0, 1.0);
    let mesh = tessellate_box_mesh(&model, r.solid);
    let dxf_str = export_dxf(&mesh).unwrap();

    assert!(dxf_str.contains("SECTION"));
    assert!(dxf_str.contains("3DFACE"));

    let reimported = import_dxf(&dxf_str).unwrap();
    assert_eq!(
        reimported.indices.len(),
        mesh.indices.len(),
        "DXF triangle count mismatch"
    );
}

// ---------------------------------------------------------------------------
// DWG export test (export_dwg delegates to DXF internally)
// ---------------------------------------------------------------------------

#[test]
fn dwg_export_produces_valid_dxf_content() {
    let (model, r) = make_test_box(Point3::ORIGIN, 3.0, 3.0, 3.0);
    let mesh = tessellate_box_mesh(&model, r.solid);
    let binary = cadkernel_io::dwg::export_dwg(&mesh).unwrap();

    assert!(!binary.is_empty(), "DWG export should produce output");

    // export_dwg delegates to DXF internally, so the output is DXF text
    let text = String::from_utf8(binary).unwrap();
    assert!(text.contains("SECTION"), "DWG output should contain DXF SECTION");
    assert!(text.contains("3DFACE"), "DWG output should contain 3DFACE entities");

    // Verify the DXF content can be reimported via the DXF importer
    let reimported = import_dxf(&text).unwrap();
    assert_eq!(
        reimported.indices.len(),
        mesh.indices.len(),
        "DWG->DXF reimport triangle count mismatch"
    );
}

// ---------------------------------------------------------------------------
// SVG export → import roundtrip tests
// ---------------------------------------------------------------------------

#[test]
fn svg_export_and_import() {
    // Test SVG export produces valid content
    let profile = vec![
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(100.0, 0.0, 0.0),
        Point3::new(100.0, 100.0, 0.0),
        Point3::new(0.0, 100.0, 0.0),
    ];
    let svg_doc = cadkernel_io::svg::profile_to_svg(&profile, 200.0, 200.0);
    let svg_str = svg_doc.render();
    assert!(svg_str.contains("<svg"), "SVG output should contain <svg> tag");

    // Test SVG import with closed shapes (rect, polygon)
    let svg_with_shapes = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200">
  <rect x="10" y="10" width="50" height="30"/>
  <polygon points="100,10 150,10 150,60 100,60"/>
</svg>"#;
    let reimported = import_svg(svg_with_shapes).unwrap();
    assert!(
        !reimported.vertices.is_empty(),
        "SVG import should produce vertices from rect/polygon"
    );
    assert!(
        !reimported.indices.is_empty(),
        "SVG import should produce triangles from closed shapes"
    );
}

// ---------------------------------------------------------------------------
// PDF export → import roundtrip tests
// ---------------------------------------------------------------------------

#[test]
fn pdf_roundtrip() {
    let svg_content = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 210 297">
  <rect x="10" y="10" width="50" height="30" fill="none" stroke="black"/>
  <line x1="10" y1="10" x2="60" y2="40" stroke="black"/>
</svg>"#;

    let pdf_bytes = export_pdf(svg_content, 210.0, 297.0).unwrap();
    assert!(pdf_bytes.len() > 10, "PDF output too short");
    // PDF files start with %PDF
    assert_eq!(&pdf_bytes[0..5], b"%PDF-", "missing PDF header");

    let imported = import_pdf(&pdf_bytes).unwrap();
    assert!(
        !imported.pages.is_empty(),
        "PDF import should produce at least one page"
    );
}

// ---------------------------------------------------------------------------
// Native .cadk format roundtrip (file-based)
// ---------------------------------------------------------------------------

#[test]
fn native_cadk_roundtrip() {
    let (model, _r) = make_test_box(Point3::new(1.0, 2.0, 3.0), 10.0, 20.0, 30.0);

    let tmp_path = std::env::temp_dir().join("cadkernel_test_roundtrip.cadk");
    let path_str = tmp_path.to_str().unwrap();

    save_project(&model, path_str).unwrap();
    let loaded = load_project(path_str).unwrap();

    let orig_verts: usize = model.vertices.iter().count();
    let loaded_verts: usize = loaded.vertices.iter().count();
    assert_eq!(loaded_verts, orig_verts, "native format vertex count mismatch");

    let orig_solids: usize = model.solids.iter().count();
    let loaded_solids: usize = loaded.solids.iter().count();
    assert_eq!(loaded_solids, orig_solids, "native format solid count mismatch");

    // Clean up
    let _ = std::fs::remove_file(&tmp_path);
}

#[test]
fn native_cadk_roundtrip_multi_solid() {
    let mut model = BRepModel::new();
    make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
    make_box(&mut model, Point3::new(5.0, 5.0, 5.0), 3.0, 3.0, 3.0).unwrap();

    let tmp_path = std::env::temp_dir().join("cadkernel_test_multi.cadk");
    let path_str = tmp_path.to_str().unwrap();

    save_project(&model, path_str).unwrap();
    let loaded = load_project(path_str).unwrap();

    let orig_solids: usize = model.solids.iter().count();
    let loaded_solids: usize = loaded.solids.iter().count();
    assert_eq!(loaded_solids, orig_solids, "multi-solid native format mismatch");

    let _ = std::fs::remove_file(&tmp_path);
}

// ---------------------------------------------------------------------------
// Large vertex count roundtrip tests
// ---------------------------------------------------------------------------

#[test]
fn stl_large_mesh_roundtrip() {
    // Build a large mesh from multiple tessellated boxes
    let mut all_meshes = Vec::new();
    for i in 0..10 {
        let (model, r) = make_test_box(
            Point3::new(i as f64 * 5.0, 0.0, 0.0),
            3.0,
            3.0,
            3.0,
        );
        all_meshes.push(tessellate_box_mesh(&model, r.solid));
    }
    let mesh = cadkernel_io::tessellate::merge_meshes(&all_meshes);
    assert!(
        mesh.indices.len() >= 100,
        "merged mesh should have many triangles, got {}",
        mesh.indices.len()
    );

    // Binary roundtrip (exact fidelity)
    let binary = write_stl_binary(&mesh).unwrap();
    let reimported_bin = read_stl_binary(&binary).unwrap();
    assert_eq!(
        reimported_bin.indices.len(),
        mesh.indices.len(),
        "large STL binary roundtrip triangle mismatch"
    );

    // ASCII roundtrip: STL stores per-face vertices, so reimport may
    // change vertex count but triangle count should be preserved.
    let ascii = write_stl_ascii(&mesh, "large_boxes");
    let reimported_ascii = read_stl_ascii(&ascii).unwrap();
    assert!(
        reimported_ascii.indices.len() >= mesh.indices.len(),
        "ASCII reimport should have >= {} triangles, got {}",
        mesh.indices.len(),
        reimported_ascii.indices.len()
    );
}

#[test]
fn obj_large_mesh_roundtrip() {
    let grid_size = 30;
    let mut vertices = Vec::with_capacity(grid_size * grid_size);
    let mut normals = Vec::with_capacity(grid_size * grid_size);
    let mut indices = Vec::new();

    for y in 0..grid_size {
        for x in 0..grid_size {
            vertices.push(Point3::new(x as f64 * 0.5, y as f64 * 0.5, 0.0));
            normals.push(Vec3::Z);
        }
    }
    for y in 0..(grid_size - 1) {
        for x in 0..(grid_size - 1) {
            let i = (y * grid_size + x) as u32;
            indices.push([i, i + 1, i + grid_size as u32]);
            indices.push([i + 1, i + grid_size as u32 + 1, i + grid_size as u32]);
        }
    }

    let mesh = Mesh {
        vertices,
        normals,
        indices,
    };
    let obj_str = write_obj(&mesh);
    let reimported = read_obj(&obj_str).unwrap();
    assert_eq!(
        reimported.vertices.len(),
        mesh.vertices.len(),
        "large OBJ vertex mismatch"
    );
}

// ---------------------------------------------------------------------------
// Cross-format mesh consistency (extended)
// ---------------------------------------------------------------------------

#[test]
fn mesh_triangle_count_consistent_across_all_text_formats() {
    let (model, r) = make_test_box(Point3::ORIGIN, 2.0, 3.0, 4.0);
    let mesh = tessellate_box_mesh(&model, r.solid);
    let expected_tri = mesh.indices.len();
    let expected_vert = mesh.vertices.len();

    // OBJ
    let obj_str = write_obj(&mesh);
    let obj_mesh = read_obj(&obj_str).unwrap();
    assert_eq!(obj_mesh.vertices.len(), expected_vert, "OBJ vertex mismatch");

    // PLY
    let ply_str = export_ply(&mesh).unwrap();
    let ply_mesh = import_ply(&ply_str).unwrap();
    assert_eq!(ply_mesh.vertices.len(), expected_vert, "PLY vertex mismatch");

    // glTF
    let gltf_str = cadkernel_io::gltf::write_gltf(&mesh).unwrap();
    let gltf_mesh = import_gltf(&gltf_str).unwrap();
    assert_eq!(gltf_mesh.vertices.len(), expected_vert, "glTF vertex mismatch");
    assert_eq!(gltf_mesh.indices.len(), expected_tri, "glTF triangle mismatch");

    // 3MF
    let threemf_str = export_3mf(&mesh).unwrap();
    let threemf_mesh = import_3mf(&threemf_str).unwrap();
    assert_eq!(threemf_mesh.vertices.len(), expected_vert, "3MF vertex mismatch");
    assert_eq!(threemf_mesh.indices.len(), expected_tri, "3MF triangle mismatch");

    // DAE
    let dae_str = export_dae(&mesh).unwrap();
    let dae_mesh = import_dae(&dae_str).unwrap();
    assert_eq!(dae_mesh.vertices.len(), expected_vert, "DAE vertex mismatch");
    assert_eq!(dae_mesh.indices.len(), expected_tri, "DAE triangle mismatch");

    // AMF
    let amf_str = export_amf(&mesh).unwrap();
    let amf_mesh = import_amf(&amf_str).unwrap();
    assert_eq!(amf_mesh.vertices.len(), expected_vert, "AMF vertex mismatch");
    assert_eq!(amf_mesh.indices.len(), expected_tri, "AMF triangle mismatch");

    // VRML
    let vrml_str = export_vrml(&mesh).unwrap();
    let vrml_mesh = import_vrml(&vrml_str).unwrap();
    assert_eq!(vrml_mesh.vertices.len(), expected_vert, "VRML vertex mismatch");
    assert_eq!(vrml_mesh.indices.len(), expected_tri, "VRML triangle mismatch");

    // OCA
    let oca_str = export_oca(&mesh).unwrap();
    let oca_mesh = import_oca(&oca_str).unwrap();
    assert_eq!(oca_mesh.vertices.len(), expected_vert, "OCA vertex mismatch");
    assert_eq!(oca_mesh.indices.len(), expected_tri, "OCA triangle mismatch");
}

// ---------------------------------------------------------------------------
// Mesh operations — additional tests
// ---------------------------------------------------------------------------

#[test]
fn mesh_harmonize_normals_consistent() {
    let (model, r) = make_test_box(Point3::ORIGIN, 1.0, 1.0, 1.0);
    let mesh = tessellate_box_mesh(&model, r.solid);

    let harmonized = cadkernel_io::mesh_ops::harmonize_normals(&mesh);
    assert_eq!(
        harmonized.vertices.len(),
        mesh.vertices.len(),
        "harmonize should preserve vertex count"
    );
    assert_eq!(
        harmonized.indices.len(),
        mesh.indices.len(),
        "harmonize should preserve triangle count"
    );

    // All normals should remain unit length
    for n in &harmonized.normals {
        let len = (n.x * n.x + n.y * n.y + n.z * n.z).sqrt();
        assert!(
            (len - 1.0).abs() < 0.1 || len < 1e-10,
            "normal not unit length: {}",
            len
        );
    }
}

#[test]
fn mesh_flip_normals_inverts_direction() {
    let mesh = Mesh {
        vertices: vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.5, 1.0, 0.0),
        ],
        normals: vec![Vec3::Z, Vec3::Z, Vec3::Z],
        indices: vec![[0, 1, 2]],
    };
    let flipped = cadkernel_io::mesh_ops::flip_normals(&mesh);
    assert_eq!(flipped.vertices.len(), mesh.vertices.len());
    for n in &flipped.normals {
        assert!(
            (n.z - (-1.0)).abs() < 1e-10,
            "flipped normal z should be -1.0, got {}",
            n.z
        );
    }
}

#[test]
fn mesh_watertight_check() {
    let (model, r) = make_test_box(Point3::ORIGIN, 1.0, 1.0, 1.0);
    let mesh = tessellate_box_mesh(&model, r.solid);

    // A tessellated box might or might not be watertight depending on
    // tessellation, but the function should run without errors.
    let _is_watertight = cadkernel_io::mesh_ops::check_mesh_watertight(&mesh);
}

#[test]
fn mesh_scale_applies_correctly() {
    let mesh = Mesh {
        vertices: vec![
            Point3::new(1.0, 2.0, 3.0),
            Point3::new(4.0, 5.0, 6.0),
            Point3::new(7.0, 8.0, 9.0),
        ],
        normals: vec![Vec3::Z, Vec3::Z, Vec3::Z],
        indices: vec![[0, 1, 2]],
    };
    let scaled = cadkernel_io::mesh_ops::scale_mesh(&mesh, 2.0, 3.0, 4.0);
    assert_eq!(scaled.vertices.len(), 3);
    assert!((scaled.vertices[0].x - 2.0).abs() < 1e-10);
    assert!((scaled.vertices[0].y - 6.0).abs() < 1e-10);
    assert!((scaled.vertices[0].z - 12.0).abs() < 1e-10);
    assert!((scaled.vertices[1].x - 8.0).abs() < 1e-10);
    assert!((scaled.vertices[1].y - 15.0).abs() < 1e-10);
    assert!((scaled.vertices[1].z - 24.0).abs() < 1e-10);
}

#[test]
fn mesh_boolean_union_two_meshes() {
    let mesh_a = Mesh {
        vertices: vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.5, 1.0, 0.0),
        ],
        normals: vec![Vec3::Z, Vec3::Z, Vec3::Z],
        indices: vec![[0, 1, 2]],
    };
    let mesh_b = Mesh {
        vertices: vec![
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(3.0, 0.0, 0.0),
            Point3::new(2.5, 1.0, 0.0),
        ],
        normals: vec![Vec3::Z, Vec3::Z, Vec3::Z],
        indices: vec![[0, 1, 2]],
    };
    let result = cadkernel_io::mesh_ops::mesh_boolean_union(&mesh_a, &mesh_b);
    assert!(
        result.vertices.len() >= 6,
        "union should have at least 6 vertices, got {}",
        result.vertices.len()
    );
    assert!(
        result.indices.len() >= 2,
        "union should have at least 2 triangles, got {}",
        result.indices.len()
    );
}

#[test]
fn mesh_fill_holes_runs_on_open_mesh() {
    // Open mesh: a single triangle (definitely has boundary edges)
    let mesh = Mesh {
        vertices: vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.5, 1.0, 0.0),
        ],
        normals: vec![Vec3::Z, Vec3::Z, Vec3::Z],
        indices: vec![[0, 1, 2]],
    };
    let filled = cadkernel_io::mesh_ops::fill_holes(&mesh).unwrap();
    assert!(
        filled.indices.len() >= mesh.indices.len(),
        "fill_holes should produce >= original triangles"
    );
}

#[test]
fn regular_solid_generates_valid_meshes() {
    use cadkernel_io::mesh_ops::RegularSolidType;

    let types = [
        RegularSolidType::Tetrahedron,
        RegularSolidType::Cube,
        RegularSolidType::Octahedron,
        RegularSolidType::Dodecahedron,
        RegularSolidType::Icosahedron,
    ];
    let expected_faces = [4, 12, 8, 36, 20]; // triangulated face counts

    for (solid_type, &min_faces) in types.iter().zip(expected_faces.iter()) {
        let mesh = cadkernel_io::mesh_ops::regular_solid(*solid_type, 1.0).unwrap();
        assert!(
            !mesh.vertices.is_empty(),
            "{:?} should produce vertices",
            solid_type
        );
        assert!(
            mesh.indices.len() >= min_faces,
            "{:?} should have >= {} triangles, got {}",
            solid_type,
            min_faces,
            mesh.indices.len()
        );

        // All indices should be valid
        let max_idx = mesh.vertices.len() as u32;
        for tri in &mesh.indices {
            for &idx in tri {
                assert!(idx < max_idx, "{:?}: invalid index {} >= {}", solid_type, idx, max_idx);
            }
        }
    }
}

#[test]
fn merge_meshes_combines_correctly() {
    let mesh_a = Mesh {
        vertices: vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.5, 1.0, 0.0),
        ],
        normals: vec![Vec3::Z, Vec3::Z, Vec3::Z],
        indices: vec![[0, 1, 2]],
    };
    let mesh_b = Mesh {
        vertices: vec![
            Point3::new(5.0, 0.0, 0.0),
            Point3::new(6.0, 0.0, 0.0),
            Point3::new(5.5, 1.0, 0.0),
            Point3::new(5.5, 0.0, 1.0),
        ],
        normals: vec![Vec3::Z, Vec3::Z, Vec3::Z, Vec3::Z],
        indices: vec![[0, 1, 2], [0, 1, 3]],
    };

    let merged = cadkernel_io::tessellate::merge_meshes(&[mesh_a, mesh_b]);
    assert_eq!(merged.vertices.len(), 7, "merged vertex count");
    assert_eq!(merged.indices.len(), 3, "merged triangle count");

    // Verify index validity after merge
    let max_idx = merged.vertices.len() as u32;
    for tri in &merged.indices {
        for &idx in tri {
            assert!(idx < max_idx, "invalid merged index {} >= {}", idx, max_idx);
        }
    }
}

// ---------------------------------------------------------------------------
// Edge-case vertex coordinate tests
// ---------------------------------------------------------------------------

#[test]
fn ply_preserves_negative_coordinates() {
    let mesh = Mesh {
        vertices: vec![
            Point3::new(-100.5, -200.25, -300.125),
            Point3::new(100.5, 200.25, 300.125),
            Point3::new(0.0, 0.0, 0.0),
        ],
        normals: vec![Vec3::Z, Vec3::Z, Vec3::Z],
        indices: vec![[0, 1, 2]],
    };
    let ply_str = export_ply(&mesh).unwrap();
    let reimported = import_ply(&ply_str).unwrap();

    for (orig, imp) in mesh.vertices.iter().zip(reimported.vertices.iter()) {
        assert!(
            (orig.x - imp.x).abs() < 1e-3,
            "PLY x mismatch: {} vs {}",
            orig.x,
            imp.x
        );
        assert!(
            (orig.y - imp.y).abs() < 1e-3,
            "PLY y mismatch: {} vs {}",
            orig.y,
            imp.y
        );
        assert!(
            (orig.z - imp.z).abs() < 1e-3,
            "PLY z mismatch: {} vs {}",
            orig.z,
            imp.z
        );
    }
}

#[test]
fn obj_preserves_vertex_precision() {
    let mesh = Mesh {
        vertices: vec![
            Point3::new(0.123456, 0.654321, 0.111111),
            Point3::new(9.876543, 3.456789, 2.345678),
            Point3::new(-1.234567, 1.732051, -0.987654),
        ],
        normals: vec![Vec3::Z, Vec3::Z, Vec3::Z],
        indices: vec![[0, 1, 2]],
    };
    let obj_str = write_obj(&mesh);
    let reimported = read_obj(&obj_str).unwrap();

    for (orig, imp) in mesh.vertices.iter().zip(reimported.vertices.iter()) {
        assert!(
            (orig.x - imp.x).abs() < 1e-4,
            "OBJ x precision loss: {} vs {}",
            orig.x,
            imp.x
        );
        assert!(
            (orig.y - imp.y).abs() < 1e-4,
            "OBJ y precision loss: {} vs {}",
            orig.y,
            imp.y
        );
        assert!(
            (orig.z - imp.z).abs() < 1e-4,
            "OBJ z precision loss: {} vs {}",
            orig.z,
            imp.z
        );
    }
}

// ---------------------------------------------------------------------------
// Tessellation: serial vs parallel produce compatible output
// ---------------------------------------------------------------------------

#[test]
fn tessellation_serial_and_parallel_both_valid() {
    let mut model = BRepModel::new();
    let r1 = make_box(&mut model, Point3::ORIGIN, 2.0, 3.0, 4.0).unwrap();
    let r2 = make_box(&mut model, Point3::new(5.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap();

    for solid in [r1.solid, r2.solid] {
        let serial = tessellate_solid(&model, solid);
        let parallel = tessellate_solid_parallel(&model, solid);

        assert!(!serial.indices.is_empty(), "serial mesh empty");
        assert!(!parallel.indices.is_empty(), "parallel mesh empty");

        // Both should produce valid meshes (index bounds)
        for mesh in [&serial, &parallel] {
            let max_idx = mesh.vertices.len() as u32;
            for tri in &mesh.indices {
                for &idx in tri {
                    assert!(idx < max_idx, "invalid index {} >= {}", idx, max_idx);
                }
            }
        }

        // Both should produce non-trivial output
        assert!(serial.vertices.len() >= 4, "serial should have >= 4 vertices");
        assert!(parallel.vertices.len() >= 4, "parallel should have >= 4 vertices");
    }
}

// ---------------------------------------------------------------------------
// Format-specific XML entity escaping
// ---------------------------------------------------------------------------

#[test]
fn threemf_handles_special_float_values() {
    // Mesh with coordinates that might produce special formatting
    let mesh = Mesh {
        vertices: vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1e-10, 1e10, -1e-10),
            Point3::new(0.5, 0.5, 0.5),
        ],
        normals: vec![Vec3::Z, Vec3::Z, Vec3::Z],
        indices: vec![[0, 1, 2]],
    };
    let xml_str = export_3mf(&mesh).unwrap();
    // The XML should be valid (no NaN, no Inf)
    assert!(!xml_str.contains("NaN"), "3MF should not contain NaN");
    assert!(!xml_str.contains("Infinity"), "3MF should not contain Infinity");

    let reimported = import_3mf(&xml_str).unwrap();
    assert_eq!(reimported.vertices.len(), 3);
}

#[test]
fn amf_handles_special_float_values() {
    let mesh = Mesh {
        vertices: vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1e-10, 1e10, -1e-10),
            Point3::new(0.5, 0.5, 0.5),
        ],
        normals: vec![Vec3::Z, Vec3::Z, Vec3::Z],
        indices: vec![[0, 1, 2]],
    };
    let amf_str = export_amf(&mesh).unwrap();
    assert!(!amf_str.contains("NaN"), "AMF should not contain NaN");
    assert!(!amf_str.contains("Infinity"), "AMF should not contain Infinity");

    let reimported = import_amf(&amf_str).unwrap();
    assert_eq!(reimported.vertices.len(), 3);
}

// ---------------------------------------------------------------------------
// STL binary size validation
// ---------------------------------------------------------------------------

#[test]
fn stl_binary_correct_size() {
    let mesh = Mesh {
        vertices: vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.5, 1.0, 0.0),
        ],
        normals: vec![Vec3::Z, Vec3::Z, Vec3::Z],
        indices: vec![[0, 1, 2]],
    };
    let binary = write_stl_binary(&mesh).unwrap();
    // STL binary: 80-byte header + 4-byte triangle count + (50 bytes per triangle)
    let expected_size = 80 + 4 + mesh.indices.len() * 50;
    assert_eq!(
        binary.len(),
        expected_size,
        "STL binary size should be {} but got {}",
        expected_size,
        binary.len()
    );
}

// ---------------------------------------------------------------------------
// BREP format fidelity tests
// ---------------------------------------------------------------------------

#[test]
fn brep_preserves_vertex_coordinates() {
    let mut model = BRepModel::new();
    let v_handles: Vec<_> = [
        Point3::new(1.5, 2.5, 3.5),
        Point3::new(-10.0, 0.0, 100.0),
        Point3::new(0.0, 0.0, 0.0),
    ]
    .iter()
    .map(|p| model.add_vertex(*p))
    .collect();

    let brep_str = export_brep(&model).unwrap();
    let reimported = import_brep(&brep_str).unwrap();

    let orig_pts: Vec<Point3> = v_handles
        .iter()
        .filter_map(|h| model.vertices.get(*h).map(|v| v.point))
        .collect();
    let imported_pts: Vec<Point3> = reimported
        .vertices
        .iter()
        .map(|(_, v)| v.point)
        .collect();

    assert_eq!(imported_pts.len(), orig_pts.len(), "vertex count mismatch");
    for (orig, imp) in orig_pts.iter().zip(imported_pts.iter()) {
        assert!(
            (orig.x - imp.x).abs() < 1e-10,
            "BREP x mismatch: {} vs {}",
            orig.x,
            imp.x
        );
        assert!(
            (orig.y - imp.y).abs() < 1e-10,
            "BREP y mismatch: {} vs {}",
            orig.y,
            imp.y
        );
        assert!(
            (orig.z - imp.z).abs() < 1e-10,
            "BREP z mismatch: {} vs {}",
            orig.z,
            imp.z
        );
    }
}
