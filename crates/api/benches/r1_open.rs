//! Gate #11 Phase A — R1 open latency (dedicated single-bench harness).
//!
//! This is a focused harness for the R1 open gate, separate from the
//! combined `reference_parts_open` harness. It exists so CI can target
//! the `r1_open` Criterion ID in isolation without running the full
//! reference_parts_open suite.
//!
//! Run locally:
//! ```bash
//! cargo bench -p cadkernel-api --bench r1_open
//! ```
//!
//! CI parses `target/criterion/r1_open/new/estimates.json` — the same ID
//! produced by `reference_parts_open`; either harness satisfies the gate.

use cadkernel_api::Session;
use cadkernel_api::reference_parts::r1_bytes;
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

criterion_group!(benches, bench_r1_open);
criterion_main!(benches);
