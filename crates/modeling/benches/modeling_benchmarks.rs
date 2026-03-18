use criterion::{Criterion, criterion_group, criterion_main};

use cadkernel_math::{Point3, Vec3};
use cadkernel_modeling::{
    BooleanOp, boolean_op, check_geometry, check_watertight, compute_mass_properties, extrude,
    fillet_edge, make_box, make_cone, make_cylinder, make_sphere, make_torus, mirror_solid,
    scale_solid,
};
use cadkernel_topology::BRepModel;

// ---------------------------------------------------------------------------
// Primitive creation
// ---------------------------------------------------------------------------

fn bench_make_box(c: &mut Criterion) {
    c.bench_function("make_box", |b| {
        b.iter(|| {
            let mut m = BRepModel::new();
            make_box(&mut m, Point3::ORIGIN, 10.0, 10.0, 10.0).unwrap();
        });
    });
}

fn bench_make_cylinder_32(c: &mut Criterion) {
    c.bench_function("make_cylinder (32 seg)", |b| {
        b.iter(|| {
            let mut m = BRepModel::new();
            make_cylinder(&mut m, Point3::ORIGIN, 5.0, 10.0, 32).unwrap();
        });
    });
}

fn bench_make_cylinder_64(c: &mut Criterion) {
    c.bench_function("make_cylinder (64 seg)", |b| {
        b.iter(|| {
            let mut m = BRepModel::new();
            make_cylinder(&mut m, Point3::ORIGIN, 5.0, 10.0, 64).unwrap();
        });
    });
}

fn bench_make_sphere_16x8(c: &mut Criterion) {
    c.bench_function("make_sphere (16x8)", |b| {
        b.iter(|| {
            let mut m = BRepModel::new();
            make_sphere(&mut m, Point3::ORIGIN, 5.0, 16, 8).unwrap();
        });
    });
}

fn bench_make_sphere_32x16(c: &mut Criterion) {
    c.bench_function("make_sphere (32x16)", |b| {
        b.iter(|| {
            let mut m = BRepModel::new();
            make_sphere(&mut m, Point3::ORIGIN, 5.0, 32, 16).unwrap();
        });
    });
}

// ---------------------------------------------------------------------------
// Tessellation
// ---------------------------------------------------------------------------

fn bench_tessellate_box(c: &mut Criterion) {
    let mut m = BRepModel::new();
    let r = make_box(&mut m, Point3::ORIGIN, 10.0, 10.0, 10.0).unwrap();
    c.bench_function("tessellate_box", |b| {
        b.iter(|| cadkernel_io::tessellate_solid(&m, r.solid));
    });
}

fn bench_tessellate_sphere_32x16(c: &mut Criterion) {
    let mut m = BRepModel::new();
    let r = make_sphere(&mut m, Point3::ORIGIN, 5.0, 32, 16).unwrap();
    c.bench_function("tessellate_sphere (32x16)", |b| {
        b.iter(|| cadkernel_io::tessellate_solid(&m, r.solid));
    });
}

// ---------------------------------------------------------------------------
// Mass properties
// ---------------------------------------------------------------------------

fn bench_mass_properties_box(c: &mut Criterion) {
    let mut m = BRepModel::new();
    let r = make_box(&mut m, Point3::ORIGIN, 10.0, 10.0, 10.0).unwrap();
    let mesh = cadkernel_io::tessellate_solid(&m, r.solid);
    c.bench_function("mass_properties (box mesh)", |b| {
        b.iter(|| compute_mass_properties(&mesh));
    });
}

fn bench_mass_properties_sphere(c: &mut Criterion) {
    let mut m = BRepModel::new();
    let r = make_sphere(&mut m, Point3::ORIGIN, 5.0, 32, 16).unwrap();
    let mesh = cadkernel_io::tessellate_solid(&m, r.solid);
    c.bench_function("mass_properties (sphere mesh)", |b| {
        b.iter(|| compute_mass_properties(&mesh));
    });
}

// ---------------------------------------------------------------------------
// STL export
// ---------------------------------------------------------------------------

fn bench_stl_write_ascii(c: &mut Criterion) {
    let mut m = BRepModel::new();
    let r = make_sphere(&mut m, Point3::ORIGIN, 5.0, 32, 16).unwrap();
    let mesh = cadkernel_io::tessellate_solid(&m, r.solid);
    c.bench_function("stl_write_ascii (sphere)", |b| {
        b.iter(|| cadkernel_io::write_stl_ascii(&mesh, "bench"));
    });
}

fn bench_stl_write_binary(c: &mut Criterion) {
    let mut m = BRepModel::new();
    let r = make_sphere(&mut m, Point3::ORIGIN, 5.0, 32, 16).unwrap();
    let mesh = cadkernel_io::tessellate_solid(&m, r.solid);
    c.bench_function("stl_write_binary (sphere)", |b| {
        b.iter(|| cadkernel_io::write_stl_binary(&mesh).unwrap());
    });
}

// ---------------------------------------------------------------------------
// Boolean operations
// ---------------------------------------------------------------------------

fn bench_boolean_union(c: &mut Criterion) {
    c.bench_function("boolean_union (box+box)", |b| {
        b.iter(|| {
            let mut ma = BRepModel::new();
            let a = make_box(&mut ma, Point3::ORIGIN, 10.0, 10.0, 10.0).unwrap();
            let mut mb = BRepModel::new();
            let b_res = make_box(&mut mb, Point3::new(5.0, 5.0, 5.0), 10.0, 10.0, 10.0).unwrap();
            boolean_op(&ma, a.solid, &mb, b_res.solid, BooleanOp::Union).ok();
        });
    });
}

fn bench_boolean_difference(c: &mut Criterion) {
    c.bench_function("boolean_difference (box-box)", |b| {
        b.iter(|| {
            let mut ma = BRepModel::new();
            let a = make_box(&mut ma, Point3::ORIGIN, 10.0, 10.0, 10.0).unwrap();
            let mut mb = BRepModel::new();
            let b_res = make_box(&mut mb, Point3::new(5.0, 5.0, 5.0), 10.0, 10.0, 10.0).unwrap();
            boolean_op(&ma, a.solid, &mb, b_res.solid, BooleanOp::Difference).ok();
        });
    });
}

// ---------------------------------------------------------------------------
// Extrude
// ---------------------------------------------------------------------------

fn bench_extrude_square(c: &mut Criterion) {
    c.bench_function("extrude (square profile)", |b| {
        b.iter(|| {
            let mut m = BRepModel::new();
            let profile = vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(10.0, 0.0, 0.0),
                Point3::new(10.0, 10.0, 0.0),
                Point3::new(0.0, 10.0, 0.0),
            ];
            extrude(&mut m, &profile, Vec3::new(0.0, 0.0, 1.0), 10.0).ok();
        });
    });
}

// ---------------------------------------------------------------------------
// Advanced primitives
// ---------------------------------------------------------------------------

fn bench_make_cone(c: &mut Criterion) {
    c.bench_function("make_cone (64 seg)", |b| {
        b.iter(|| {
            let mut m = BRepModel::new();
            make_cone(&mut m, Point3::ORIGIN, 5.0, 2.0, 10.0, 64).unwrap();
        });
    });
}

fn bench_make_torus(c: &mut Criterion) {
    c.bench_function("make_torus (64x32)", |b| {
        b.iter(|| {
            let mut m = BRepModel::new();
            make_torus(&mut m, Point3::ORIGIN, 10.0, 3.0, 64, 32).unwrap();
        });
    });
}

// ---------------------------------------------------------------------------
// Feature operations
// ---------------------------------------------------------------------------

fn bench_mirror_solid(c: &mut Criterion) {
    c.bench_function("mirror_solid (box)", |b| {
        b.iter(|| {
            let mut m = BRepModel::new();
            let r = make_box(&mut m, Point3::ORIGIN, 10.0, 10.0, 10.0).unwrap();
            mirror_solid(&mut m, r.solid, Point3::ORIGIN, Vec3::X).ok();
        });
    });
}

fn bench_scale_solid(c: &mut Criterion) {
    c.bench_function("scale_solid (box)", |b| {
        b.iter(|| {
            let mut m = BRepModel::new();
            let r = make_box(&mut m, Point3::ORIGIN, 10.0, 10.0, 10.0).unwrap();
            scale_solid(&mut m, r.solid, Point3::ORIGIN, 2.0).ok();
        });
    });
}

fn bench_fillet_edge(c: &mut Criterion) {
    c.bench_function("fillet_edge (box)", |b| {
        b.iter(|| {
            let mut m = BRepModel::new();
            let r = make_box(&mut m, Point3::ORIGIN, 10.0, 10.0, 10.0).unwrap();
            let verts: Vec<_> = m.vertices.iter().map(|(h, _)| h).collect();
            if verts.len() >= 2 {
                fillet_edge(&mut m, r.solid, verts[0], verts[1], 1.0).ok();
            }
        });
    });
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn bench_check_geometry(c: &mut Criterion) {
    let mut m = BRepModel::new();
    let r = make_box(&mut m, Point3::ORIGIN, 10.0, 10.0, 10.0).unwrap();
    c.bench_function("check_geometry (box)", |b| {
        b.iter(|| check_geometry(&m, r.solid));
    });
}

fn bench_check_watertight(c: &mut Criterion) {
    let mut m = BRepModel::new();
    let r = make_box(&mut m, Point3::ORIGIN, 10.0, 10.0, 10.0).unwrap();
    c.bench_function("check_watertight (box)", |b| {
        b.iter(|| check_watertight(&m, r.solid));
    });
}

// ---------------------------------------------------------------------------
// Stress tests
// ---------------------------------------------------------------------------

fn bench_tessellate_sphere_64x32(c: &mut Criterion) {
    let mut m = BRepModel::new();
    let r = make_sphere(&mut m, Point3::ORIGIN, 5.0, 64, 32).unwrap();
    c.bench_function("tessellate_sphere (64x32)", |b| {
        b.iter(|| cadkernel_io::tessellate_solid(&m, r.solid));
    });
}

fn bench_tessellate_torus_64x32(c: &mut Criterion) {
    let mut m = BRepModel::new();
    let r = make_torus(&mut m, Point3::ORIGIN, 10.0, 3.0, 64, 32).unwrap();
    c.bench_function("tessellate_torus (64x32)", |b| {
        b.iter(|| cadkernel_io::tessellate_solid(&m, r.solid));
    });
}

fn bench_boolean_intersection(c: &mut Criterion) {
    c.bench_function("boolean_intersection (box∩box)", |b| {
        b.iter(|| {
            let mut ma = BRepModel::new();
            let a = make_box(&mut ma, Point3::ORIGIN, 10.0, 10.0, 10.0).unwrap();
            let mut mb = BRepModel::new();
            let b_res = make_box(&mut mb, Point3::new(5.0, 5.0, 5.0), 10.0, 10.0, 10.0).unwrap();
            boolean_op(&ma, a.solid, &mb, b_res.solid, BooleanOp::Intersection).ok();
        });
    });
}

criterion_group!(
    benches,
    bench_make_box,
    bench_make_cylinder_32,
    bench_make_cylinder_64,
    bench_make_sphere_16x8,
    bench_make_sphere_32x16,
    bench_tessellate_box,
    bench_tessellate_sphere_32x16,
    bench_mass_properties_box,
    bench_mass_properties_sphere,
    bench_stl_write_ascii,
    bench_stl_write_binary,
    bench_boolean_union,
    bench_boolean_difference,
    bench_extrude_square,
    bench_make_cone,
    bench_make_torus,
    bench_mirror_solid,
    bench_scale_solid,
    bench_fillet_edge,
    bench_check_geometry,
    bench_check_watertight,
    bench_tessellate_sphere_64x32,
    bench_tessellate_torus_64x32,
    bench_boolean_intersection,
);
criterion_main!(benches);
