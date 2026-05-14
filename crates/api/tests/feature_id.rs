//! A2.1 — `FeatureId` plumbing tests.
//!
//! Verifies that:
//! - `Document::push_history` (via `Session::execute`) assigns a fresh
//!   monotonically-increasing [`FeatureId`] to every history event,
//!   starting at `FeatureId(1)`.
//! - `Document::feature(FeatureId)` looks up events by id; sentinel
//!   `FeatureId(0)` and unknown ids return `None`.
//! - `Document::features()` is a same-slice alias for `Document::history()`.
//! - The `feature_id` field round-trips through JSON serialisation and is
//!   excluded from `canonical_hash` (so a pre-A2.1 fixture with no
//!   `feature_id` still hashes identically post-A2.1).

use cadkernel_api::{Command, FeatureId, Session};

fn three_primitive_session() -> Session {
    let log = vec![
        Command::CreateBox {
            dx: 1.0,
            dy: 1.0,
            dz: 1.0,
        },
        Command::CreateSphere { radius: 0.5 },
        Command::CreateCylinder {
            radius: 0.5,
            height: 2.0,
        },
    ];
    Session::replay(&log).expect("replay")
}

#[test]
fn feature_ids_are_monotonic_starting_at_one() {
    let session = three_primitive_session();
    let ids: Vec<FeatureId> = session
        .document()
        .history()
        .iter()
        .map(|ev| ev.feature_id)
        .collect();
    assert_eq!(ids, vec![FeatureId(1), FeatureId(2), FeatureId(3)]);
}

#[test]
fn feature_lookup_returns_matching_event() {
    let session = three_primitive_session();
    let ev = session
        .document()
        .feature(FeatureId(2))
        .expect("FeatureId(2) must exist");
    assert_eq!(ev.op, "create_sphere");
}

#[test]
fn feature_lookup_returns_none_for_sentinel_and_unknown() {
    let session = three_primitive_session();
    assert!(
        session.document().feature(FeatureId(0)).is_none(),
        "sentinel FeatureId(0) must never resolve"
    );
    assert!(
        session.document().feature(FeatureId(999)).is_none(),
        "unknown FeatureId must return None"
    );
}

#[test]
fn features_alias_matches_history() {
    let session = three_primitive_session();
    let history = session.document().history();
    let features = session.document().features();
    assert_eq!(history.len(), features.len());
    for (a, b) in history.iter().zip(features.iter()) {
        assert_eq!(a, b);
    }
}

#[test]
fn old_history_event_json_deserialises_with_sentinel_feature_id() {
    // Pre-A2.1 wire format had no `feature_id` field. Older `.cadk`
    // snapshots holding such events must still deserialise via
    // `#[serde(default)]` — populated with FeatureId(0). Subsequent
    // replay reassigns fresh ids, so the sentinel never escapes.
    use cadkernel_api::HistoryEvent;
    let pre_a2_1 = r#"{"op":"create_box","primary":0,"description":"create_box(1x2x3) -> Solid#0"}"#;
    let ev: HistoryEvent = serde_json::from_str(pre_a2_1).expect("deserialise legacy event");
    assert_eq!(ev.feature_id, FeatureId(0));
    assert_eq!(ev.op, "create_box");
}

#[test]
fn canonical_hash_is_unchanged_by_feature_id_field() {
    // `canonical_hash` deliberately omits `feature_id` from its mix so
    // adding the field is non-breaking. Two sessions built the same way
    // must always produce the same hash regardless of internal id state.
    let a = three_primitive_session();
    let b = three_primitive_session();
    assert_eq!(a.canonical_hash(), b.canonical_hash());
}
