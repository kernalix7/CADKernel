//! Wave 1 sub-phase 5a - STEP AP214 round-trip face-area fidelity
//! scaffolding for v1.0 Gate #16.

use cadkernel_io::tessellate::tessellate_solid;
use cadkernel_io::{export_step, import_step};
use cadkernel_math::Point3;
use cadkernel_modeling::{quick_box, quick_cylinder, quick_subtract, quick_torus, quick_union};
use cadkernel_topology::BRepModel;

const MIN_FACE_AREA_JACCARD: f64 = 0.999;
const MAX_VERTEX_DELTA: f64 = 1.0e-6;

fn total_face_area(model: &BRepModel) -> f64 {
    let mut total = 0.0_f64;
    for (solid_h, _) in model.solids.iter() {
        let mesh = tessellate_solid(model, solid_h);
        for idx in &mesh.indices {
            let a = mesh.vertices[idx[0] as usize];
            let b = mesh.vertices[idx[1] as usize];
            let c = mesh.vertices[idx[2] as usize];
            let ab = b - a;
            let ac = c - a;
            total += 0.5 * ab.cross(ac).length();
        }
    }
    total
}

#[derive(Debug, Clone, Copy)]
struct StepRoundTripMetrics {
    face_area_jaccard: f64,
    source_face_area: f64,
    imported_face_area: f64,
    max_vertex_delta: f64,
    source_vertex_count: usize,
    imported_vertex_count: usize,
    source_face_count: usize,
    imported_face_count: usize,
}

fn step_roundtrip_metrics(model: &BRepModel) -> StepRoundTripMetrics {
    let step = export_step(model).expect("export STEP");
    let imported = import_step(&step).expect("import STEP");
    let source_face_area = total_face_area(model);
    let imported_face_area = total_face_area(&imported);
    StepRoundTripMetrics {
        face_area_jaccard: face_area_overlap_ratio_from_areas(source_face_area, imported_face_area),
        source_face_area,
        imported_face_area,
        max_vertex_delta: max_vertex_delta(model, &imported),
        source_vertex_count: model.vertices.len(),
        imported_vertex_count: imported.vertices.len(),
        source_face_count: model.faces.len(),
        imported_face_count: imported.faces.len(),
    }
}

fn face_area_overlap_ratio(lhs: &BRepModel, rhs: &BRepModel) -> f64 {
    face_area_overlap_ratio_from_areas(total_face_area(lhs), total_face_area(rhs))
}

fn face_area_overlap_ratio_from_areas(a: f64, b: f64) -> f64 {
    if a == 0.0 && b == 0.0 {
        return 1.0;
    }
    let (lo, hi) = if a < b { (a, b) } else { (b, a) };
    if hi == 0.0 { 0.0 } else { lo / hi }
}

fn max_vertex_delta(lhs: &BRepModel, rhs: &BRepModel) -> f64 {
    if lhs.vertices.is_empty() && rhs.vertices.is_empty() {
        return 0.0;
    }
    if lhs.vertices.is_empty() || rhs.vertices.is_empty() {
        return f64::INFINITY;
    }

    let lhs_points: Vec<_> = lhs.vertices.iter().map(|(_, v)| v.point).collect();
    let rhs_points: Vec<_> = rhs.vertices.iter().map(|(_, v)| v.point).collect();
    max_nearest_distance(&lhs_points, &rhs_points)
        .max(max_nearest_distance(&rhs_points, &lhs_points))
}

fn max_nearest_distance(source: &[Point3], target: &[Point3]) -> f64 {
    source
        .iter()
        .map(|p| {
            target
                .iter()
                .map(|q| p.distance_to(*q))
                .fold(f64::INFINITY, f64::min)
        })
        .fold(0.0, f64::max)
}

fn translate_model(model: &mut BRepModel, dx: f64, dy: f64, dz: f64) {
    for (_, v) in model.vertices.iter_mut() {
        v.point = Point3::new(v.point.x + dx, v.point.y + dy, v.point.z + dz);
    }
}

fn r1_reference_model() -> BRepModel {
    let base = quick_box(80.0, 40.0, 10.0).expect("R1 base box");
    let mut upright = quick_box(10.0, 40.0, 40.0).expect("R1 upright box");
    translate_model(&mut upright, 0.0, 0.0, 8.0);
    let mut model = quick_union(&base, &upright).expect("R1 union");

    for (x, y) in [(20.0, 10.0), (60.0, 10.0), (20.0, 30.0), (60.0, 30.0)] {
        let mut hole = quick_cylinder(3.0, 14.0).expect("R1 screw hole");
        translate_model(&mut hole, x, y, -2.0);
        model = quick_subtract(&model, &hole).expect("R1 subtract hole");
    }

    model
}

fn r2_reference_model() -> BRepModel {
    let outer = quick_cylinder(24.0, 20.0).expect("R2 outer cylinder");
    let mut bore = quick_cylinder(12.0, 24.0).expect("R2 bore");
    translate_model(&mut bore, 0.0, 0.0, -2.0);
    let mut model = quick_subtract(&outer, &bore).expect("R2 bore subtract");

    for x in [16.0, -16.0] {
        let mut boss = quick_cylinder(5.0, 8.0).expect("R2 top boss");
        translate_model(&mut boss, x, 0.0, 18.0);
        model = quick_union(&model, &boss).expect("R2 boss union");
    }

    model
}

fn r3_reference_model() -> BRepModel {
    let torus = quick_torus(18.0, 3.0).expect("R3 torus");
    let mut hub = quick_cylinder(8.0, 6.0).expect("R3 hub");
    translate_model(&mut hub, 0.0, 0.0, -3.0);
    quick_union(&torus, &hub).expect("R3 hub union")
}

fn assert_step_fidelity(part: &str, model: &BRepModel) {
    let metrics = step_roundtrip_metrics(model);
    eprintln!(
        "{part} STEP face-area fidelity = {:.12}, area = {:.12} -> {:.12}, max vertex delta = {:.12e}, vertices = {} -> {}, faces = {} -> {}",
        metrics.face_area_jaccard,
        metrics.source_face_area,
        metrics.imported_face_area,
        metrics.max_vertex_delta,
        metrics.source_vertex_count,
        metrics.imported_vertex_count,
        metrics.source_face_count,
        metrics.imported_face_count
    );
    assert_eq!(
        metrics.source_vertex_count, metrics.imported_vertex_count,
        "{part} STEP vertex count changed"
    );
    assert!(
        metrics.face_area_jaccard >= MIN_FACE_AREA_JACCARD,
        "{part} STEP face-area fidelity = {}",
        metrics.face_area_jaccard
    );
    assert!(
        metrics.max_vertex_delta <= MAX_VERTEX_DELTA,
        "{part} STEP max vertex delta = {}",
        metrics.max_vertex_delta
    );
}

#[test]
fn step_fidelity_helper_compute_face_area_overlap() {
    let mut m1 = BRepModel::new();
    cadkernel_modeling::make_box(&mut m1, Point3::ORIGIN, 10.0, 20.0, 30.0).expect("box");

    let id_ratio = face_area_overlap_ratio(&m1, &m1);
    assert!(
        (id_ratio - 1.0).abs() < 1e-12,
        "identity overlap must be exactly 1.0, got {id_ratio}"
    );

    let mut m2 = BRepModel::new();
    cadkernel_modeling::make_box(&mut m2, Point3::ORIGIN, 10.0, 20.0, 15.0).expect("smaller box");
    let different_ratio = face_area_overlap_ratio(&m1, &m2);
    assert!(
        different_ratio > 0.0 && different_ratio < 1.0,
        "different models must produce a strict partial overlap, got {different_ratio}"
    );
}

#[test]
fn r1_step_roundtrip_face_area_fidelity() {
    assert_step_fidelity("R1", &r1_reference_model());
}

#[test]
fn r2_step_roundtrip_face_area_fidelity() {
    assert_step_fidelity("R2", &r2_reference_model());
}

#[test]
fn r3_step_roundtrip_face_area_fidelity() {
    assert_step_fidelity("R3", &r3_reference_model());
}
