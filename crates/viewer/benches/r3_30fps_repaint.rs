//! v1.0 Gate #33 — R3 1080p repaint budget.
//!
//! The benchmark writes the deterministic R3 `.cadk` fixture to `target/`
//! unless `CADKERNEL_R3_PATH` points at an existing fixture. The session is
//! then loaded through `Session::load_cadk_from_path`, staged in the viewer
//! scene, and repainted through the headless egui + wgpu offscreen path.

use cadkernel_api::Session;
use cadkernel_api::reference_parts::r3_bytes;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::path::PathBuf;

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const SAMPLE_FRAMES: usize = 100;
const MEAN_BUDGET_MS: f64 = 33.3;

fn r3_fixture_path() -> PathBuf {
    if let Some(path) = std::env::var_os("CADKERNEL_R3_PATH") {
        return PathBuf::from(path);
    }

    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target"));
    let dir = target_dir.join("cadkernel-benches");
    std::fs::create_dir_all(&dir).expect("create benchmark fixture directory");
    let path = dir.join("R3.cadk");
    let bytes = r3_bytes().expect("build R3 reference part");
    std::fs::write(&path, bytes).expect("write R3 reference part fixture");
    path
}

fn load_r3_session() -> Session {
    let path = r3_fixture_path();
    Session::load_cadk_from_path(&path).unwrap_or_else(|err| {
        panic!(
            "failed to load R3 fixture via Session::load_cadk_from_path({}): {err}",
            path.display()
        )
    })
}

fn prepared_app() -> cadkernel_viewer::test_support::CadApp {
    let session = load_r3_session();
    assert!(
        session.document().solid_count() > 0,
        "R3 fixture must contain at least one solid"
    );
    let mut app = cadkernel_viewer::test_support::CadApp::new_headless();
    app.load_session_for_test(session);
    assert!(
        !app.display_vertices_for_test().is_empty(),
        "R3 fixture must stage visible vertices"
    );
    app
}

fn assert_r3_repaint_budget() {
    let mut app = prepared_app();
    let (mean_ms, p95_ms) = app
        .measure_repaint_1080p_for_test(SAMPLE_FRAMES)
        .expect("measure R3 repaint");
    assert!(
        mean_ms <= MEAN_BUDGET_MS,
        "R3 repaint mean {mean_ms:.3} ms exceeds {MEAN_BUDGET_MS:.1} ms; p95={p95_ms:.3} ms"
    );
    black_box((mean_ms, p95_ms));
}

fn bench_r3_repaint_1080p(c: &mut Criterion) {
    assert_r3_repaint_budget();

    let mut app = prepared_app();
    app.repaint_one_offscreen_for_test(WIDTH, HEIGHT)
        .expect("warm up R3 repaint");
    c.bench_function("r3_repaint_1080p", |b| {
        b.iter(|| {
            app.repaint_one_offscreen_for_test(WIDTH, HEIGHT)
                .expect("R3 offscreen repaint");
            black_box(app.display_vertices_for_test().len());
        });
    });
}

criterion_group!(benches, bench_r3_repaint_1080p);
criterion_main!(benches);
