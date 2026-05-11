//! Commercial CAD Roadmap v0.5 Gate #11 Phase A — measure
//! `Session::load_cadk` mean wall-clock time for the R1 and R2
//! reference parts. Gate target: R1 mean < 250 ms.
//!
//! Run locally:
//! ```bash
//! cargo bench -p cadkernel-api --bench reference_parts_open
//! ```
//!
//! CI parses `target/criterion/r1_open/new/estimates.json` to enforce
//! the threshold (see `scripts/bench_threshold.sh`).

use cadkernel_api::Session;
use cadkernel_api::reference_parts::{r1_bytes, r2_bytes};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_r1_open(c: &mut Criterion) {
    let bytes = r1_bytes().expect("build R1 fixture");
    c.bench_function("r1_open", |b| {
        b.iter(|| {
            let session = Session::load_cadk(black_box(&bytes)).expect("load R1");
            black_box(session);
        });
    });
}

fn bench_r2_open(c: &mut Criterion) {
    let bytes = r2_bytes().expect("build R2 fixture");
    c.bench_function("r2_open", |b| {
        b.iter(|| {
            let session = Session::load_cadk(black_box(&bytes)).expect("load R2");
            black_box(session);
        });
    });
}

criterion_group!(benches, bench_r1_open, bench_r2_open);
criterion_main!(benches);
