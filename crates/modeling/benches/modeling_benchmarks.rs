use criterion::{Criterion, criterion_group, criterion_main};

use cadkernel_math::{Point3, Vec3};
use cadkernel_modeling::{
    Assembly, BooleanOp, boolean_op, check_geometry, check_watertight, compute_mass_properties,
    extrude, fillet_edge, make_box, make_cone, make_cylinder, make_sphere, make_torus,
    mirror_solid, scale_solid,
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

// ---------------------------------------------------------------------------
// Assembly (large, BVH-accelerated)
// ---------------------------------------------------------------------------

fn bench_assembly_500_parts_bvh_interference(c: &mut Criterion) {
    let mut model = BRepModel::new();
    let b = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

    let mut asm = Assembly::new("Bench 500");
    for i in 0..500 {
        let id = asm.add_component(&format!("P{i}"), b.solid);
        let x = (i % 50) as f64 * 5.0;
        let y = (i / 50) as f64 * 5.0;
        asm.set_placement(id, cadkernel_modeling::translation(x, y, 0.0))
            .unwrap();
    }

    c.bench_function("assembly_500_bvh_interference", |bench| {
        bench.iter(|| asm.check_all_interferences(&model).unwrap());
    });
}

fn bench_assembly_500_parts_bom(c: &mut Criterion) {
    let mut model = BRepModel::new();
    let b = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

    let mut asm = Assembly::new("BOM Bench");
    for i in 0..500 {
        // Reuse some names so BOM groups them
        let name = format!("Type_{}", i % 20);
        asm.add_component(&name, b.solid);
    }

    c.bench_function("assembly_500_bom", |bench| {
        bench.iter(|| asm.bill_of_materials());
    });
}

fn bench_assembly_1000_parts_bvh_interference(c: &mut Criterion) {
    let mut model = BRepModel::new();
    let b = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

    let mut asm = Assembly::new("Bench 1000");
    for i in 0..1000 {
        let id = asm.add_component(&format!("P{i}"), b.solid);
        let x = (i % 50) as f64 * 5.0;
        let y = (i / 50) as f64 * 5.0;
        asm.set_placement(id, cadkernel_modeling::translation(x, y, 0.0))
            .unwrap();
    }

    c.bench_function("assembly_1000_bvh_interference", |bench| {
        bench.iter(|| asm.check_all_interferences(&model).unwrap());
    });
}

// ---------------------------------------------------------------------------
// Boolean chain (5 solids)
// ---------------------------------------------------------------------------

fn bench_boolean_chain_5(c: &mut Criterion) {
    c.bench_function("boolean_chain (5 boxes union)", |b| {
        b.iter(|| {
            let mut current_model = BRepModel::new();
            let first = make_box(&mut current_model, Point3::ORIGIN, 10.0, 10.0, 10.0).unwrap();
            let mut cur_solid = first.solid;
            let mut cur_model = current_model;

            for i in 1..5 {
                let mut next = BRepModel::new();
                let nb = make_box(
                    &mut next,
                    Point3::new(i as f64 * 8.0, 0.0, 0.0),
                    10.0,
                    10.0,
                    10.0,
                )
                .unwrap();
                let result =
                    boolean_op(&cur_model, cur_solid, &next, nb.solid, BooleanOp::Union).unwrap();
                let s = result.solids.iter().next().map(|(h, _)| h).unwrap();
                cur_solid = s;
                cur_model = result;
            }
        });
    });
}

// ---------------------------------------------------------------------------
// Large pattern (50 copies)
// ---------------------------------------------------------------------------

fn bench_linear_pattern_50(c: &mut Criterion) {
    c.bench_function("linear_pattern (50 copies)", |b| {
        b.iter(|| {
            let mut m = BRepModel::new();
            let r = make_box(&mut m, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();
            cadkernel_modeling::linear_pattern(&mut m, r.solid, Vec3::X, 5.0, 50).unwrap();
        });
    });
}

fn bench_circular_pattern_36(c: &mut Criterion) {
    c.bench_function("circular_pattern (36 copies)", |b| {
        b.iter(|| {
            let mut m = BRepModel::new();
            let r = make_box(&mut m, Point3::new(10.0, 0.0, 0.0), 2.0, 2.0, 2.0).unwrap();
            cadkernel_modeling::circular_pattern(&mut m, r.solid, Point3::ORIGIN, Vec3::Z, 36)
                .unwrap();
        });
    });
}

// ---------------------------------------------------------------------------
// Large assembly creation (100 parts)
// ---------------------------------------------------------------------------

fn bench_assembly_100_parts_creation(c: &mut Criterion) {
    let mut model = BRepModel::new();
    let b = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

    c.bench_function("assembly_100_parts_creation", |bench| {
        bench.iter(|| {
            let mut asm = Assembly::new("Bench100");
            for i in 0..100 {
                let id = asm.add_component(&format!("P{i}"), b.solid);
                let x = (i % 10) as f64 * 5.0;
                let y = (i / 10) as f64 * 5.0;
                asm.set_placement(id, cadkernel_modeling::translation(x, y, 0.0))
                    .unwrap();
            }
            asm
        });
    });
}

// ---------------------------------------------------------------------------
// BVH nearest query
// ---------------------------------------------------------------------------

fn bench_bvh_nearest_1000(c: &mut Criterion) {
    use cadkernel_geometry::bvh::{Aabb, Bvh};

    let items: Vec<(Aabb, usize)> = (0..1000)
        .map(|i| {
            let x = (i % 50) as f64 * 3.0;
            let y = (i / 50) as f64 * 3.0;
            (
                Aabb::new(Point3::new(x, y, 0.0), Point3::new(x + 1.0, y + 1.0, 1.0)),
                i,
            )
        })
        .collect();
    let bvh = Bvh::build(&items);

    c.bench_function("bvh_nearest (1000 items)", |bench| {
        bench.iter(|| bvh.query_nearest(Point3::new(75.0, 30.0, 0.5)));
    });
}

// ---------------------------------------------------------------------------
// Multi-transform chain
// ---------------------------------------------------------------------------

fn bench_multi_transform_5(c: &mut Criterion) {
    c.bench_function("multi_transform (5 chained)", |b| {
        b.iter(|| {
            let mut m = BRepModel::new();
            let r = make_box(&mut m, Point3::ORIGIN, 5.0, 5.0, 5.0).unwrap();
            cadkernel_modeling::multi_transform(
                &mut m,
                r.solid,
                &[
                    cadkernel_modeling::Transform::Translation(Vec3::new(10.0, 0.0, 0.0)),
                    cadkernel_modeling::Transform::Rotation {
                        axis_origin: Point3::ORIGIN,
                        axis_dir: Vec3::Z,
                        angle: std::f64::consts::FRAC_PI_4,
                    },
                    cadkernel_modeling::Transform::Scale {
                        center: Point3::ORIGIN,
                        factor: 1.5,
                    },
                    cadkernel_modeling::Transform::Mirror {
                        plane_point: Point3::ORIGIN,
                        plane_normal: Vec3::X,
                    },
                    cadkernel_modeling::Transform::Translation(Vec3::new(0.0, 5.0, 0.0)),
                ],
            )
            .unwrap();
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
    bench_assembly_500_parts_bvh_interference,
    bench_assembly_500_parts_bom,
    bench_assembly_1000_parts_bvh_interference,
    bench_boolean_chain_5,
    bench_linear_pattern_50,
    bench_circular_pattern_36,
    bench_assembly_100_parts_creation,
    bench_bvh_nearest_1000,
    bench_multi_transform_5,
);
criterion_main!(benches);
