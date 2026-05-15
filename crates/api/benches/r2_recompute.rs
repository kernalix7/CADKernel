//! Gate #10 Phase A placeholder — R2 dimension-change recompute latency.
//!
//! Measures the wall-clock cost of changing one dimension on the R2 housing
//! and triggering a full recompute. The `Session::recompute` API and the real
//! R2 parametric fixture land in Wave 3; this harness uses `r1_bytes` as a
//! stand-in so the bench binary compiles and the CI step is wired now.
//!
//! Run locally:
//! ```bash
//! cargo bench -p cadkernel-api --bench r2_recompute
//! ```
//!
//! CI parses `target/criterion/r2_recompute/new/estimates.json` to enforce
//! the threshold (see `scripts/bench_threshold.sh`).
//!
//! TODO(Wave 3): replace r1_bytes() with r2_bytes() + a Command::SetDimension
//! call once Session::recompute_body is implemented.

use cadkernel_api::Session;
use cadkernel_api::reference_parts::r1_bytes;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_r2_recompute(c: &mut Criterion) {
    let bytes = r1_bytes().expect("build R1 stand-in for r2_recompute placeholder");
    c.bench_function("r2_recompute", |b| {
        b.iter(|| {
            let session = Session::load_cadk(black_box(&bytes)).expect("load stand-in");
            black_box(session);
        });
    });
}

criterion_group!(benches, bench_r2_recompute);
criterion_main!(benches);
