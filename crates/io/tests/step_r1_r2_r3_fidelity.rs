//! Wave 1 sub-phase 5a - STEP AP214 round-trip face-area fidelity
//! scaffolding for v1.0 Gate #16.

use cadkernel_io::tessellate::tessellate_solid;
use cadkernel_io::{export_step, import_step};
use cadkernel_math::Point3;
use cadkernel_modeling::primitives::{make_box, make_cylinder, make_torus};
use cadkernel_topology::BRepModel;

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

fn step_roundtrip_face_area_ratio(model: &BRepModel) -> f64 {
    let step = export_step(model).expect("export STEP");
    let imported = import_step(&step).expect("import STEP");
    face_area_overlap_ratio(model, &imported)
}

fn face_area_overlap_ratio(lhs: &BRepModel, rhs: &BRepModel) -> f64 {
    let a = total_face_area(lhs);
    let b = total_face_area(rhs);
    if a == 0.0 && b == 0.0 {
        return 1.0;
    }
    let (lo, hi) = if a < b { (a, b) } else { (b, a) };
    if hi == 0.0 {
        0.0
    } else {
        lo / hi
    }
}

#[test]
fn step_fidelity_helper_compute_face_area_overlap() {
    let mut m1 = BRepModel::new();
    make_box(&mut m1, Point3::ORIGIN, 10.0, 20.0, 30.0).expect("box");

    let id_ratio = face_area_overlap_ratio(&m1, &m1);
    assert!(
        (id_ratio - 1.0).abs() < 1e-12,
        "identity overlap must be exactly 1.0, got {id_ratio}"
    );

    let mut m2 = BRepModel::new();
    make_box(&mut m2, Point3::ORIGIN, 10.0, 20.0, 15.0).expect("smaller box");
    let different_ratio = face_area_overlap_ratio(&m1, &m2);
    assert!(
        different_ratio > 0.0 && different_ratio < 1.0,
        "different models must produce a strict partial overlap, got {different_ratio}"
    );
}

#[test]
fn r1_step_roundtrip_face_area_fidelity() {
    let mut model = BRepModel::new();
    make_box(&mut model, Point3::ORIGIN, 10.0, 20.0, 30.0).expect("R1 box");
    let ratio = step_roundtrip_face_area_ratio(&model);
    eprintln!("R1 STEP face-area fidelity = {ratio:.12}");
    assert!(ratio >= 0.999, "R1 STEP face-area fidelity = {ratio}");
}

#[test]
fn r2_step_roundtrip_face_area_fidelity() {
    let mut model = BRepModel::new();
    make_cylinder(&mut model, Point3::ORIGIN, 10.0, 20.0, 48).expect("R2 cylinder");
    let ratio = step_roundtrip_face_area_ratio(&model);
    eprintln!("R2 STEP face-area fidelity = {ratio:.12}");
    assert!(ratio >= 0.999, "R2 STEP face-area fidelity = {ratio}");
}

#[test]
fn r3_step_roundtrip_face_area_fidelity() {
    let mut model = BRepModel::new();
    make_torus(&mut model, Point3::ORIGIN, 18.0, 3.0, 64, 16).expect("R3 torus");
    let ratio = step_roundtrip_face_area_ratio(&model);
    eprintln!("R3 STEP face-area fidelity = {ratio:.12}");
    assert!(ratio >= 0.999, "R3 STEP face-area fidelity = {ratio}");
}
