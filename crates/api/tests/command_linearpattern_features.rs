use cadkernel_api::{ApiError, Command, FeatureId, Outcome, Session, SolidId};

fn created_id(outcome: Outcome) -> SolidId {
    match outcome {
        Outcome::SolidCreated { id, .. } => id,
        other => panic!("expected SolidCreated, got {other:?}"),
    }
}

fn pattern_fields(outcome: Outcome) -> (SolidId, u32, u32, Vec<SolidId>) {
    match outcome {
        Outcome::PatternCreated {
            pattern_id,
            instance_count,
            total_features,
            ids,
        } => (pattern_id, instance_count, total_features, ids),
        other => panic!("expected PatternCreated, got {other:?}"),
    }
}

fn box_id(session: &mut Session) -> SolidId {
    created_id(
        session
            .execute(Command::CreateBox {
                dx: 1.0,
                dy: 1.0,
                dz: 1.0,
            })
            .unwrap(),
    )
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn legacy_empty_features_path_preserves_pattern_shape() {
    let mut session = Session::new();
    let source = box_id(&mut session);

    let out = session
        .execute(Command::LinearPattern {
            id: source,
            direction: [1.0, 0.0, 0.0],
            spacing: 2.0,
            count: 4,
            skip_instances: Vec::new(),
            features: Vec::new(),
            mirror_alternate: false,
            instance_overrides: Vec::new(),
        })
        .unwrap();

    let (pattern_id, instance_count, total_features, ids) = pattern_fields(out);
    assert_eq!(pattern_id, source);
    assert_eq!(instance_count, 4);
    assert_eq!(total_features, 3);
    assert_eq!(ids.len(), 4);
    assert_eq!(ids[0], source);
    assert_eq!(session.document().solid_count(), 4);
}

#[test]
fn mirror_alternate_reflects_odd_copies_about_instance_plane() {
    let mut session = Session::new();
    let source = box_id(&mut session);

    let out = session
        .execute(Command::LinearPattern {
            id: source,
            direction: [1.0, 0.0, 0.0],
            spacing: 3.0,
            count: 4,
            skip_instances: Vec::new(),
            features: Vec::new(),
            mirror_alternate: true,
            instance_overrides: Vec::new(),
        })
        .unwrap();

    let (_, instance_count, total_features, ids) = pattern_fields(out);
    assert_eq!(instance_count, 4);
    assert_eq!(total_features, 3);
    let odd_1 = session.document().bounding_box(ids[1]).unwrap();
    let even_2 = session.document().bounding_box(ids[2]).unwrap();
    let odd_3 = session.document().bounding_box(ids[3]).unwrap();
    assert_close(odd_1.min[0], 2.0);
    assert_close(odd_1.max[0], 3.0);
    assert_close(even_2.min[0], 6.0);
    assert_close(even_2.max[0], 7.0);
    assert_close(odd_3.min[0], 8.0);
    assert_close(odd_3.max[0], 9.0);
}

#[test]
fn instance_override_suppress_removes_target_index() {
    let mut session = Session::new();
    let source = box_id(&mut session);
    let command: Command = serde_json::from_value(serde_json::json!({
        "op": "linear_pattern",
        "id": source.0,
        "direction": [1.0, 0.0, 0.0],
        "spacing": 2.0,
        "count": 5,
        "instance_overrides": [{ "index": 2, "suppress": true }]
    }))
    .unwrap();

    let out = session.execute(command).unwrap();
    let (_, instance_count, total_features, ids) = pattern_fields(out);
    assert_eq!(instance_count, 4);
    assert_eq!(total_features, 3);
    assert_eq!(ids.len(), 4);
}

#[test]
fn instance_override_offset_adjust_shifts_target_bbox() {
    let mut session = Session::new();
    let source = box_id(&mut session);
    let command: Command = serde_json::from_value(serde_json::json!({
        "op": "linear_pattern",
        "id": source.0,
        "direction": [1.0, 0.0, 0.0],
        "spacing": 2.0,
        "count": 3,
        "instance_overrides": [{
            "index": 1,
            "offset_adjust": [0.25, 1.0, 0.0]
        }]
    }))
    .unwrap();

    let out = session.execute(command).unwrap();
    let (_, _, _, ids) = pattern_fields(out);
    let shifted = session.document().bounding_box(ids[1]).unwrap();
    assert_close(shifted.min[0], 2.25);
    assert_close(shifted.max[0], 3.25);
    assert_close(shifted.min[1], 1.0);
    assert_close(shifted.max[1], 2.0);
}

#[test]
fn out_of_range_instance_override_is_ignored() {
    let mut session = Session::new();
    let source = box_id(&mut session);
    let command: Command = serde_json::from_value(serde_json::json!({
        "op": "linear_pattern",
        "id": source.0,
        "direction": [1.0, 0.0, 0.0],
        "spacing": 2.0,
        "count": 3,
        "instance_overrides": [{ "index": 99, "suppress": true }]
    }))
    .unwrap();

    let out = session.execute(command).unwrap();
    let (_, instance_count, total_features, ids) = pattern_fields(out);
    assert_eq!(instance_count, 3);
    assert_eq!(total_features, 2);
    assert_eq!(ids.len(), 3);
}

#[test]
fn features_mode_patterns_each_resolved_primary_solid() {
    let mut session = Session::new();
    let first = box_id(&mut session);
    let second = box_id(&mut session);
    session
        .execute(Command::Translate {
            id: second,
            dx: 0.0,
            dy: 5.0,
            dz: 0.0,
        })
        .unwrap();

    let out = session
        .execute(Command::LinearPattern {
            id: SolidId(999),
            direction: [1.0, 0.0, 0.0],
            spacing: 10.0,
            count: 3,
            skip_instances: Vec::new(),
            features: vec![FeatureId(1), FeatureId(3)],
            mirror_alternate: false,
            instance_overrides: Vec::new(),
        })
        .unwrap();

    let (pattern_id, instance_count, total_features, ids) = pattern_fields(out);
    assert_eq!(pattern_id, first);
    assert_eq!(instance_count, 6);
    assert_eq!(total_features, 4);
    assert_eq!(ids.len(), 6);
    assert!(ids.contains(&first));
    assert!(ids.contains(&second));
    assert_eq!(session.document().solid_count(), 6);
}

#[test]
fn features_mode_unknown_feature_id_errors_with_id() {
    let mut session = Session::new();
    let err = session
        .execute(Command::LinearPattern {
            id: SolidId(0),
            direction: [1.0, 0.0, 0.0],
            spacing: 1.0,
            count: 2,
            skip_instances: Vec::new(),
            features: vec![FeatureId(999)],
            mirror_alternate: false,
            instance_overrides: Vec::new(),
        })
        .unwrap_err();

    match err {
        ApiError::InvalidArgument(msg) => assert!(msg.contains("FeatureId(999)")),
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

#[test]
fn features_mode_sentinel_feature_id_errors_with_id() {
    let mut session = Session::new();
    let err = session
        .execute(Command::LinearPattern {
            id: SolidId(0),
            direction: [1.0, 0.0, 0.0],
            spacing: 1.0,
            count: 2,
            skip_instances: Vec::new(),
            features: vec![FeatureId(0)],
            mirror_alternate: false,
            instance_overrides: Vec::new(),
        })
        .unwrap_err();

    match err {
        ApiError::InvalidArgument(msg) => assert!(msg.contains("FeatureId(0)")),
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

#[test]
fn linear_pattern_json_omits_empty_new_fields_and_round_trips_rich_shape() {
    let empty = Command::LinearPattern {
        id: SolidId(7),
        direction: [1.0, 0.0, 0.0],
        spacing: 1.0,
        count: 2,
        skip_instances: Vec::new(),
        features: Vec::new(),
        mirror_alternate: false,
        instance_overrides: Vec::new(),
    };
    let empty_json = serde_json::to_value(&empty).unwrap();
    assert!(empty_json.get("skip_instances").is_none());
    assert!(empty_json.get("features").is_none());
    assert!(empty_json.get("mirror_alternate").is_none());
    assert!(empty_json.get("instance_overrides").is_none());

    let rich_json = serde_json::json!({
        "op": "linear_pattern",
        "id": 7,
        "direction": [1.0, 0.0, 0.0],
        "spacing": 1.0,
        "count": 4,
        "features": [2],
        "mirror_alternate": true,
        "instance_overrides": [{
            "index": 2,
            "suppress": true,
            "offset_adjust": [0.1, 0.0, 0.0]
        }]
    });
    let command: Command = serde_json::from_value(rich_json).unwrap();
    let encoded = serde_json::to_value(&command).unwrap();
    assert_eq!(encoded["features"], serde_json::json!([2]));
    assert_eq!(encoded["mirror_alternate"], serde_json::json!(true));
    assert_eq!(
        encoded["instance_overrides"][0]["index"],
        serde_json::json!(2)
    );
    assert_eq!(
        encoded["instance_overrides"][0]["offset_adjust"],
        serde_json::json!([0.1, 0.0, 0.0])
    );
    let reparsed: Command = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(serde_json::to_value(reparsed).unwrap(), encoded);
}
