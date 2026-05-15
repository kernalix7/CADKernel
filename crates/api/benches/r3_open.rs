//! Gate #16 placeholder — R3 gearbox assembly open latency.
//!
//! R3 is a three-component gearbox assembly (~150 features total) with
//! section views and GD&T. The real fixture and the assembly `Command`
//! variants land in Wave 3; this harness uses `r1_bytes` as a stand-in so
//! the bench binary compiles and the CI step is wired now.
//!
//! Run locally:
//! ```bash
//! cargo bench -p cadkernel-api --bench r3_open
//! ```
//!
//! CI parses `target/criterion/r3_open/new/estimates.json` to enforce
//! the threshold (see `scripts/bench_threshold.sh`). Gate target: < 3 s.
//!
//! TODO(Wave 3): replace r1_bytes() with r3_bytes() once the assembly
//! Command variants and fixture builder are implemented.

use cadkernel_api::Session;
use cadkernel_api::reference_parts::r1_bytes;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_r3_open(c: &mut Criterion) {
    let bytes = r1_bytes().expect("build R1 stand-in for r3_open placeholder");
    c.bench_function("r3_open", |b| {
        b.iter(|| {
            let session = Session::load_cadk(black_box(&bytes)).expect("load stand-in");
            black_box(session);
        });
    });
}

criterion_group!(benches, bench_r3_open);
criterion_main!(benches);
