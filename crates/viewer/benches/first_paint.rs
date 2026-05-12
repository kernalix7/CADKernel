use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_first_paint_cpu(c: &mut Criterion) {
    c.bench_function("viewer_first_paint_cpu", |b| {
        b.iter(|| {
            let app = cadkernel_viewer::test_support::cold_init_cpu_only();
            black_box(app);
        })
    });
}

criterion_group!(first_paint, bench_first_paint_cpu);
criterion_main!(first_paint);
