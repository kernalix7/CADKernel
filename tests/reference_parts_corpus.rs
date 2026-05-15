//! Reference-parts corpus regression — Commercial CAD Roadmap v0.5
//! Gates #9 / #10.
//!
//! This suite verifies that the R1 / R2 reference parts produced by
//! `examples/build_reference_parts.rs` are:
//!
//!   1. Buildable end-to-end through the public `cadkernel-api` surface
//!      with no direct kernel calls (Gate #9 R1, Gate #10 R2).
//!   2. Byte-deterministic — two independent runs of the same command
//!      log produce byte-identical `.cadk` payloads (Trust gate from
//!      §11 of the roadmap; cross-architecture identity check is
//!      delegated to multi-OS CI).
//!   3. Round-trip through `Session::save_cadk` / `Session::load_cadk`
//!      with byte-identical re-encoding.
//!   4. Recorded in the canonical command log with the expected number
//!      of history events.

use cadkernel_api::{Command, Outcome, Session};

/// Reproduces the command log used by `examples::build_reference_parts::build_r1_box`.
/// Kept in sync by hand — the example is the single source of truth for
/// reference-part shape; this test is the determinism guard.
fn r1_session() -> Session {
    let mut s = Session::new();
    s.execute(Command::CreateBox {
        dx: 100.0,
        dy: 50.0,
        dz: 25.0,
    })
    .expect("R1 CreateBox");
    s
}

/// Reproduces the command log used by `examples::build_reference_parts::build_r2_extrude`.
fn r2_session() -> Session {
    let mut s = Session::new();
    let plate = match s
        .execute(Command::CreateBox {
            dx: 60.0,
            dy: 40.0,
            dz: 10.0,
        })
        .expect("R2 plate")
    {
        Outcome::SolidCreated { id, .. } => id,
        other => panic!("expected SolidCreated, got {other:?}"),
    };
    let hole = match s
        .execute(Command::CreateCylinder {
            radius: 5.0,
            height: 10.0,
        })
        .expect("R2 hole")
    {
        Outcome::SolidCreated { id, .. } => id,
        other => panic!("expected SolidCreated, got {other:?}"),
    };
    s.execute(Command::BooleanSubtract {
        lhs: plate,
        rhs: hole,
    })
    .expect("R2 subtract");
    s
}

#[test]
fn r1_box_builds_through_public_api() {
    let s = r1_session();
    assert_eq!(s.document().solid_count(), 1, "R1 must be a single solid");
    assert_eq!(s.log().len(), 1, "R1 log = 1 command");
}

#[test]
fn r2_plate_with_hole_builds_through_public_api() {
    let s = r2_session();
    // The boolean consumes plate + hole and produces one combined solid.
    assert_eq!(
        s.document().solid_count(),
        1,
        "R2 must collapse to a single drilled plate"
    );
    assert_eq!(s.log().len(), 3, "R2 log = box + cylinder + subtract");
}

#[test]
fn r1_cadk_byte_deterministic_across_two_runs() {
    let a = r1_session().save_cadk().expect("R1 cadk run 1");
    let b = r1_session().save_cadk().expect("R1 cadk run 2");
    assert_eq!(
        a,
        b,
        "R1 .cadk must be byte-identical across runs (got len {} vs {})",
        a.len(),
        b.len()
    );
}

#[test]
fn r2_cadk_byte_deterministic_across_two_runs() {
    let a = r2_session().save_cadk().expect("R2 cadk run 1");
    let b = r2_session().save_cadk().expect("R2 cadk run 2");
    assert_eq!(
        a,
        b,
        "R2 .cadk must be byte-identical across runs (got len {} vs {})",
        a.len(),
        b.len()
    );
}

#[test]
fn r1_cadk_roundtrip_preserves_command_log() {
    let bytes = r1_session().save_cadk().expect("R1 encode");
    let restored = Session::load_cadk(&bytes).expect("R1 decode");
    assert_eq!(restored.log().len(), 1, "R1 round-trip log length");
    let re_encoded = restored.save_cadk().expect("R1 re-encode");
    assert_eq!(
        bytes, re_encoded,
        "R1 .cadk must re-encode byte-identically after a load_cadk round-trip"
    );
}

#[test]
fn r2_cadk_roundtrip_preserves_command_log() {
    let bytes = r2_session().save_cadk().expect("R2 encode");
    let restored = Session::load_cadk(&bytes).expect("R2 decode");
    assert_eq!(restored.log().len(), 3, "R2 round-trip log length");
    assert_eq!(
        restored.document().solid_count(),
        1,
        "R2 round-trip preserves the drilled-plate shape"
    );
    let re_encoded = restored.save_cadk().expect("R2 re-encode");
    assert_eq!(
        bytes, re_encoded,
        "R2 .cadk must re-encode byte-identically after a load_cadk round-trip"
    );
}
