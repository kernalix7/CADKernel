//! Real-world STEP/IGES corpus tests.
//!
//! Gated behind the `real-corpus` Cargo feature so the default test suite stays
//! lean. Run with:
//!
//! ```bash
//! cargo test -p cadkernel-io --features real-corpus
//! ```
//!
//! Fixture files live in `crates/io/tests/fixtures/{step,iges}/` and are
//! tracked individually in `crates/io/tests/fixtures/LICENSES.md`. Sources used
//! in Phase 1 are limited to license-unambiguous origins: NIST Engineering
//! Design Model Repository (US public domain) and STEPcode (BSD-3-Clause).
//!
//! The fixture list is hardcoded rather than discovered via `walkdir` so that:
//! 1. Each test case has a stable, named function visible in test output.
//! 2. New fixtures must be added intentionally with a paired `LICENSES.md`
//!    entry — preventing accidental commits of unlicensed files.

#![cfg(feature = "real-corpus")]

use std::path::PathBuf;

use cadkernel_io::{import_iges, import_step};

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

fn read_step(name: &str) -> String {
    let path = fixture_root().join("step").join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {e}", path.display()))
}

fn read_iges(name: &str) -> String {
    let path = fixture_root().join("iges").join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {e}", path.display()))
}

fn parse_step_with_solids(name: &str) {
    let content = read_step(name);
    let model = import_step(&content)
        .unwrap_or_else(|e| panic!("import_step {name}: {e:?}"));
    assert!(
        !model.solids.is_empty(),
        "{name}: expected at least one solid, got 0"
    );
}

fn parse_iges_nonempty(name: &str) {
    let content = read_iges(name);
    let entities = cadkernel_io::parse_iges(&content)
        .unwrap_or_else(|e| panic!("parse_iges {name}: {e:?}"));
    assert!(
        !entities.is_empty(),
        "{name}: expected at least one IGES entity, got 0"
    );
    let _ = import_iges(&content)
        .unwrap_or_else(|e| panic!("import_iges {name}: {e:?}"));
}

// -----------------------------------------------------------------------------
// STEP — NIST Engineering Design Model Repository (US public domain, AP203)
// -----------------------------------------------------------------------------

#[test]
fn nist_design2() {
    parse_step_with_solids("nist_design2.step");
}

#[test]
fn nist_part() {
    parse_step_with_solids("nist_part.step");
}

#[test]
fn nist_clevis21() {
    parse_step_with_solids("nist_clevis21.stp");
}

#[test]
fn nist_doghouse() {
    parse_step_with_solids("nist_doghouse.stp");
}

#[test]
fn nist_as1_pe() {
    parse_step_with_solids("nist_as1_pe.stp");
}

#[test]
fn nist_bracket1() {
    parse_step_with_solids("nist_bracket1.stp");
}

#[test]
fn nist_interacting_pockets() {
    parse_step_with_solids("nist_interacting_pockets.stp");
}

// -----------------------------------------------------------------------------
// STEP — STEPcode `data/` directory (BSD-3-Clause, AP214e3)
// -----------------------------------------------------------------------------

#[test]
fn stepcode_as1_oc_214() {
    parse_step_with_solids("stepcode_as1_oc_214.stp");
}

#[test]
fn stepcode_dm1_id_214() {
    parse_step_with_solids("stepcode_dm1_id_214.stp");
}

#[test]
fn stepcode_io1_cm_214() {
    parse_step_with_solids("stepcode_io1_cm_214.stp");
}

#[test]
fn stepcode_sg1_c5_214() {
    parse_step_with_solids("stepcode_sg1_c5_214.stp");
}

// -----------------------------------------------------------------------------
// IGES — NIST Engineering Design Model Repository (US public domain)
// -----------------------------------------------------------------------------

#[test]
fn nist_flange_blank6() {
    parse_iges_nonempty("nist_flange_blank6.igs");
}

#[test]
fn nist_d_part2() {
    parse_iges_nonempty("nist_d_part2.igs");
}

#[test]
fn nist_d_bearing3() {
    parse_iges_nonempty("nist_d_bearing3.igs");
}

#[test]
fn nist_cadds_part_level1() {
    parse_iges_nonempty("nist_cadds_part_level1.igs");
}
