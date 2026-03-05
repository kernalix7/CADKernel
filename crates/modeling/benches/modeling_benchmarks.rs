use criterion::{Criterion, criterion_group, criterion_main};

use cadkernel_math::Point3;
use cadkernel_modeling::{
    BooleanOp, boolean_op, compute_mass_properties, extrude, make_box, make_cylinder, make_sphere,
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
        b.iter(|| cadkernel_io::write_stl_binary(&mesh));
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
    use cadkernel_math::Vec3;
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
);
criterion_main!(benches);
