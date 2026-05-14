//! A2.2 — `Command::Mirror` feature-list mode integration tests.
//!
//! Covers the new `features: Vec<FeatureId>` field added in A2.2:
//! - Backward-compat: legacy single-solid path is unchanged when
//!   `features` is empty (default).
//! - Multi-feature dispatch produces one combined `PatternCreated`
//!   outcome that mirrors every listed feature's primary solid.
//! - Error cases: unknown / sentinel `FeatureId`s and empty resolution
//!   surface `ApiError::InvalidArgument` with the offending id in the
//!   message.
//! - JSON wire shape: `features: Vec::new()` round-trips equivalently
//!   to omitting the field entirely (skip_serializing_if), and pre-A2.2
//!   blobs deserialise with the field defaulting to empty.

use cadkernel_api::{ApiError, Command, FeatureId, Outcome, Session, SolidId};

fn unit_cube() -> Command {
    Command::CreateBox {
        dx: 1.0,
        dy: 1.0,
        dz: 1.0,
    }
}

#[test]
fn legacy_empty_features_path_matches_pre_a2_2_behaviour() {
    // A bare `Command::Mirror` with `features: Vec::new()` must take the
    // legacy single-solid codepath and emit `Outcome::SolidCreated`
    // (matching the pre-A2.2 contract bit-for-bit on the success path).
    let mut session = Session::new();
    let source_id = match session.execute(unit_cube()).unwrap() {
        Outcome::SolidCreated { id, .. } => id,
        other => panic!("expected SolidCreated, got {other:?}"),
    };
    let outcome = session
        .execute(Command::Mirror {
            id: source_id,
            point: [0.0, 0.0, 0.0],
            normal: [1.0, 0.0, 0.0],
            merge: false,
            features: Vec::new(),
        })
        .unwrap();
    match outcome {
        Outcome::SolidCreated { id, .. } => {
            assert_ne!(id, source_id);
            assert_eq!(session.document().solid_count(), 2);
        }
        other => panic!("expected SolidCreated from legacy path, got {other:?}"),
    }
}

#[test]
fn single_feature_mirror_emits_pattern_created() {
    // Building a single cube assigns FeatureId(1). Passing that id in
    // `features` must take the A2.2 codepath and produce a
    // `PatternCreated` outcome with `instance_count = 2`
    // (1 original + 1 mirror) and `total_features = 1` new solid.
    let mut session = Session::new();
    let source_id = match session.execute(unit_cube()).unwrap() {
        Outcome::SolidCreated { id, .. } => id,
        other => panic!("expected SolidCreated, got {other:?}"),
    };
    let outcome = session
        .execute(Command::Mirror {
            id: SolidId(999), // ignored — `features` is non-empty.
            point: [0.0, 0.0, 0.0],
            normal: [1.0, 0.0, 0.0],
            merge: false,
            features: vec![FeatureId(1)],
        })
        .unwrap();
    match outcome {
        Outcome::PatternCreated {
            pattern_id,
            instance_count,
            total_features,
            ids,
        } => {
            assert_eq!(pattern_id, source_id);
            assert_eq!(instance_count, 2);
            assert_eq!(total_features, 1);
            assert_eq!(ids.len(), 2);
            assert_eq!(ids[0], source_id);
        }
        other => panic!("expected PatternCreated, got {other:?}"),
    }
    // Document now has the original plus its mirror.
    assert_eq!(session.document().solid_count(), 2);
}

#[test]
fn multi_feature_mirror_creates_one_pattern_with_three_new_solids() {
    // Three primitives → FeatureId(1..=3). Mirror all three across the
    // YZ plane: the document should contain six solids afterwards and
    // the outcome should be a single PatternCreated bundling the batch.
    let mut session = Session::new();
    let a = match session
        .execute(Command::CreateBox {
            dx: 1.0,
            dy: 1.0,
            dz: 1.0,
        })
        .unwrap()
    {
        Outcome::SolidCreated { id, .. } => id,
        other => panic!("expected SolidCreated, got {other:?}"),
    };
    // Translate so the second cube sits clear of the mirror plane and
    // does not face-touch the first cube's mirror.
    session
        .execute(Command::Translate {
            id: a,
            dx: 5.0,
            dy: 0.0,
            dz: 0.0,
        })
        .unwrap();
    let _b = match session
        .execute(Command::CreateSphere { radius: 0.5 })
        .unwrap()
    {
        Outcome::SolidCreated { id, .. } => id,
        other => panic!("expected SolidCreated, got {other:?}"),
    };
    let _c = match session
        .execute(Command::CreateCylinder {
            radius: 0.5,
            height: 2.0,
        })
        .unwrap()
    {
        Outcome::SolidCreated { id, .. } => id,
        other => panic!("expected SolidCreated, got {other:?}"),
    };
    assert_eq!(session.document().solid_count(), 3);

    // The first three feature ids in this session are 1=create_box,
    // 2=translate (primary=Some(a)), 3=create_sphere. Translate has a
    // primary so the dispatcher will mirror the box twice (once via
    // FeatureId(1), once via FeatureId(2)). We use 1, 3, 4 instead so
    // each FeatureId resolves to a different solid.
    let outcome = session
        .execute(Command::Mirror {
            id: SolidId(0),
            point: [0.0, 0.0, 0.0],
            normal: [1.0, 0.0, 0.0],
            merge: false,
            features: vec![FeatureId(1), FeatureId(3), FeatureId(4)],
        })
        .unwrap();
    match outcome {
        Outcome::PatternCreated {
            pattern_id,
            instance_count,
            total_features,
            ids,
        } => {
            assert_eq!(pattern_id, SolidId(0));
            assert_eq!(total_features, 3);
            assert_eq!(instance_count, 4);
            assert_eq!(ids.len(), 4);
            assert_eq!(ids[0], SolidId(0));
        }
        other => panic!("expected PatternCreated, got {other:?}"),
    }
    assert_eq!(session.document().solid_count(), 6);
}

#[test]
fn unknown_feature_id_surfaces_invalid_argument_with_id_in_message() {
    let mut session = Session::new();
    session.execute(unit_cube()).unwrap();
    let err = session
        .execute(Command::Mirror {
            id: SolidId(0),
            point: [0.0, 0.0, 0.0],
            normal: [1.0, 0.0, 0.0],
            merge: false,
            features: vec![FeatureId(999)],
        })
        .unwrap_err();
    match err {
        ApiError::InvalidArgument(msg) => {
            assert!(
                msg.contains("999"),
                "error must include offending id: {msg}"
            );
        }
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

#[test]
fn sentinel_feature_id_surfaces_invalid_argument() {
    // FeatureId(0) is the reserved "not assigned" sentinel. It must
    // never resolve to a real history event, so the dispatcher must
    // bounce it with InvalidArgument rather than silently succeed.
    let mut session = Session::new();
    session.execute(unit_cube()).unwrap();
    let err = session
        .execute(Command::Mirror {
            id: SolidId(0),
            point: [0.0, 0.0, 0.0],
            normal: [1.0, 0.0, 0.0],
            merge: false,
            features: vec![FeatureId(0)],
        })
        .unwrap_err();
    assert!(matches!(err, ApiError::InvalidArgument(_)));
}

#[test]
fn empty_features_field_round_trips_through_json_unchanged() {
    // `features: Vec::new()` must serialise to the same JSON as a
    // command built without the field, because of
    // `#[serde(skip_serializing_if = "Vec::is_empty")]`.
    let with_field = Command::Mirror {
        id: SolidId(0),
        point: [0.0, 0.0, 0.0],
        normal: [1.0, 0.0, 0.0],
        merge: false,
        features: Vec::new(),
    };
    let json = serde_json::to_value(&with_field).unwrap();
    assert!(
        json.get("features").is_none(),
        "empty features must be omitted from JSON: {json}"
    );
    // The pre-A2.2 wire blob (no `features`) must deserialise back to
    // an empty Vec via `#[serde(default)]`.
    let pre_a2_2 = serde_json::json!({
        "op": "mirror",
        "id": 0,
        "point": [0.0, 0.0, 0.0],
        "normal": [1.0, 0.0, 0.0],
    });
    let parsed: Command = serde_json::from_value(pre_a2_2).unwrap();
    match parsed {
        Command::Mirror { features, .. } => {
            assert!(
                features.is_empty(),
                "pre-A2.2 deserialise must produce empty features"
            );
        }
        other => panic!("expected Mirror, got {other:?}"),
    }
}

#[test]
fn non_empty_features_serialise_and_round_trip() {
    // Non-empty `features` must appear in the JSON and round-trip
    // through serde without loss.
    let cmd = Command::Mirror {
        id: SolidId(7),
        point: [1.0, 2.0, 3.0],
        normal: [0.0, 0.0, 1.0],
        merge: true,
        features: vec![FeatureId(2), FeatureId(5)],
    };
    let json = serde_json::to_value(&cmd).unwrap();
    assert_eq!(
        json.get("features"),
        Some(&serde_json::json!([2, 5])),
        "features must serialise as a JSON array of integers: {json}"
    );
    let parsed: Command = serde_json::from_value(json).unwrap();
    match parsed {
        Command::Mirror {
            id,
            merge,
            features,
            ..
        } => {
            assert_eq!(id, SolidId(7));
            assert!(merge);
            assert_eq!(features, vec![FeatureId(2), FeatureId(5)]);
        }
        other => panic!("expected Mirror, got {other:?}"),
    }
}
