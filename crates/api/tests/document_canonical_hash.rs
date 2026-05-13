//! A3.1 task #1 — `Document::canonical_hash()` acceptance tests.
//!
//! The autosave subsystem keys snapshots on the canonical hash so we need
//! the hash to be:
//! 1. Deterministic for a given build (same state → same `u64` every call).
//! 2. Stable across `save_cadk` / `load_cadk` roundtrips.
//! 3. Sensitive to meaningful state changes (different command prefix →
//!    different hash with overwhelming probability).

use cadkernel_api::{Command, Session, SolidId};

fn r1_log() -> Vec<Command> {
    vec![
        Command::CreateBox {
            dx: 50.0,
            dy: 30.0,
            dz: 10.0,
        },
        Command::CreateCylinder {
            radius: 3.0,
            height: 12.0,
        },
        Command::BooleanSubtract {
            lhs: SolidId(0),
            rhs: SolidId(1),
        },
    ]
}

#[test]
fn empty_document_hash_is_stable_under_repeated_calls() {
    let session = Session::new();
    let h1 = session.canonical_hash();
    let h2 = session.canonical_hash();
    let h3 = session.document().canonical_hash();
    assert_eq!(h1, h2);
    assert_eq!(h2, h3);
}

#[test]
fn two_empty_documents_hash_equal() {
    let a = Session::new();
    let b = Session::new();
    assert_eq!(a.canonical_hash(), b.canonical_hash());
}

#[test]
fn rebuilding_r1_twice_yields_identical_hashes() {
    let a = Session::replay(&r1_log()).expect("replay a");
    let b = Session::replay(&r1_log()).expect("replay b");
    assert_eq!(
        a.canonical_hash(),
        b.canonical_hash(),
        "deterministic build must produce the same canonical hash"
    );
}

#[test]
fn r1_full_vs_prefix_have_different_hashes() {
    let full = Session::replay(&r1_log()).expect("replay full");
    let prefix: Vec<Command> = r1_log().into_iter().take(2).collect();
    let partial = Session::replay(&prefix).expect("replay prefix");
    assert_ne!(
        full.canonical_hash(),
        partial.canonical_hash(),
        "prefix and full log must hash differently"
    );
}

#[test]
fn save_cadk_load_cadk_roundtrip_preserves_canonical_hash() {
    let session = Session::replay(&r1_log()).expect("replay");
    let before = session.canonical_hash();
    let bytes = session.save_cadk().expect("save_cadk");
    let reloaded = Session::load_cadk(&bytes).expect("load_cadk");
    let after = reloaded.canonical_hash();
    assert_eq!(
        before, after,
        "canonical_hash must survive .cadk save → load roundtrip"
    );
}

#[test]
fn empty_and_nonempty_hashes_differ() {
    let empty = Session::new();
    let r1 = Session::replay(&r1_log()).expect("replay r1");
    assert_ne!(empty.canonical_hash(), r1.canonical_hash());
}
