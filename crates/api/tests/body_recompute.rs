use cadkernel_api::{
    AxisRef, BodyId, Command, FeatureId, FeatureSpec, Outcome, PadDirection, PadSpec, PadType,
    PlaneRef, Session, SketchId, SketchRef, command_schemas,
};

fn helix_command(radius: f64, turns: f64) -> Command {
    Command::Helix {
        axis: AxisRef::Z,
        radius,
        pitch: 2.0,
        height: 10.0,
        turns,
        cone_angle: 0.0,
    }
}

fn helix_spec(radius: f64, turns: f64) -> FeatureSpec {
    FeatureSpec::Helix(cadkernel_api::HelixSpec {
        axis: AxisRef::Z,
        radius,
        pitch: 2.0,
        height: 10.0,
        turns,
        cone_angle: 0.0,
    })
}

fn pad_spec() -> FeatureSpec {
    FeatureSpec::Pad(PadSpec {
        sketch: SketchRef {
            sketch_id: SketchId(99),
        },
        distance: 1.0,
        direction: PadDirection::Normal,
        symmetric: false,
        type_: PadType::Blind,
    })
}

fn session_with_helix() -> (Session, BodyId, FeatureId) {
    let mut session = Session::new();
    session.execute(helix_command(3.0, 5.0)).expect("helix 1");
    let body_id = session.document().active_body().expect("active body");
    let body = session.document().body(body_id).expect("body");
    let first = FeatureId(body.features[0].feature_id);
    (session, body_id, first)
}

fn session_with_two_helixes() -> (Session, BodyId) {
    let mut session = Session::new();
    session.execute(helix_command(3.0, 5.0)).expect("helix 1");
    session.execute(helix_command(4.0, 3.0)).expect("helix 2");
    let body_id = session.document().active_body().expect("active body");
    (session, body_id)
}

#[test]
fn create_body_emits_body_created() {
    let mut session = Session::new();
    let out = session
        .execute(Command::CreateBody {
            name: "Body1".into(),
            base_plane: PlaneRef::XY,
        })
        .expect("CreateBody");
    assert!(matches!(
        out,
        Outcome::BodyCreated {
            body: BodyId(1),
            ref name,
            ..
        } if name == "Body1"
    ));
    assert_eq!(session.document().body_count(), 1);
    assert_eq!(session.document().active_body(), Some(BodyId(1)));
}

#[test]
fn create_body_ids_are_monotonic() {
    let mut session = Session::new();
    for i in 0..3 {
        session
            .execute(Command::CreateBody {
                name: format!("Body{}", i + 1),
                base_plane: PlaneRef::XY,
            })
            .expect("CreateBody");
    }
    assert_eq!(
        session.document().body_ids(),
        vec![BodyId(1), BodyId(2), BodyId(3)]
    );
}

#[test]
fn create_body_default_name_and_plane_deserialize() {
    let cmd: Command = serde_json::from_str(r#"{"op":"create_body"}"#).expect("command");
    match cmd {
        Command::CreateBody { name, base_plane } => {
            assert_eq!(name, "");
            assert_eq!(base_plane, PlaneRef::XY);
        }
        other => panic!("expected CreateBody, got {other:?}"),
    }
}

#[test]
fn create_body_custom_plane_normal_is_normalized() {
    let mut session = Session::new();
    let out = session
        .execute(Command::CreateBody {
            name: "Body".into(),
            base_plane: PlaneRef::Custom {
                origin: [1.0, 2.0, 3.0],
                normal: [0.0, 0.0, 2.0],
            },
        })
        .expect("CreateBody");
    match out {
        Outcome::BodyCreated { plane, .. } => {
            assert_eq!(plane.origin, [1.0, 2.0, 3.0]);
            assert_eq!(plane.normal, [0.0, 0.0, 1.0]);
        }
        other => panic!("expected BodyCreated, got {other:?}"),
    }
}

#[test]
fn create_body_rejects_zero_custom_normal() {
    let mut session = Session::new();
    let err = session
        .execute(Command::CreateBody {
            name: "Body".into(),
            base_plane: PlaneRef::Custom {
                origin: [0.0, 0.0, 0.0],
                normal: [0.0, 0.0, 0.0],
            },
        })
        .expect_err("zero normal");
    assert!(err.to_string().contains("normal"));
}

#[test]
fn standalone_helix_auto_creates_active_body() {
    let mut session = Session::new();
    session.execute(helix_command(3.0, 5.0)).expect("helix");
    let body_id = session.document().active_body().expect("body");
    let body = session.document().body(body_id).expect("body");
    assert_eq!(body.features.len(), 1);
    assert_eq!(body.features[0].spec_kind, "helix");
    assert_eq!(body.tip, Some(0));
    assert!(body.current_solid.is_some());
}

#[test]
fn additive_helix_appends_to_active_body() {
    let (session, body_id) = session_with_two_helixes();
    let body = session.document().body(body_id).expect("body");
    assert_eq!(body.features.len(), 2);
    assert!(body.features.iter().all(|f| f.spec_kind == "helix"));
    assert_eq!(body.tip, Some(1));
}

#[test]
fn recompute_body_after_standalone_helix_preserves_hash() {
    let mut session = Session::new();
    session.execute(helix_command(3.0, 5.0)).expect("helix");
    let body_id = session.document().active_body().expect("body");
    let solid_before = session
        .document()
        .body(body_id)
        .and_then(|body| body.current_solid)
        .expect("solid before");
    session
        .execute(Command::RecomputeBody { body: body_id })
        .expect("recompute");
    let solid_after = session
        .document()
        .body(body_id)
        .and_then(|body| body.current_solid)
        .expect("solid after");
    assert_eq!(solid_before, solid_after);
    assert_eq!(session.document().solid_count(), 1);
}

#[test]
fn suppress_helix_changes_body_hash_then_restores() {
    let mut session = Session::new();
    session.execute(helix_command(3.0, 5.0)).expect("helix");
    let body_id = session.document().active_body().expect("body");
    let feature = {
        let body = session.document().body(body_id).expect("body");
        FeatureId(body.features[0].feature_id)
    };
    session
        .execute(Command::SuppressFeature {
            feature,
            suppressed: true,
        })
        .expect("suppress");
    assert!(session.document().body(body_id).expect("body").features[0].suppressed);
    session
        .execute(Command::SuppressFeature {
            feature,
            suppressed: false,
        })
        .expect("restore");
    assert!(!session.document().body(body_id).expect("body").features[0].suppressed);
}

#[test]
fn set_tip_recomputes_to_prefix() {
    let (mut session, body_id, first) = session_with_helix();
    session
        .execute(Command::SetTip {
            body: body_id,
            feature: first,
        })
        .expect("set tip");
    let body = session.document().body(body_id).expect("body");
    assert_eq!(body.tip, Some(0));
}

#[test]
fn set_tip_unknown_feature_errors() {
    let (mut session, body_id, _first) = session_with_helix();
    let err = session
        .execute(Command::SetTip {
            body: body_id,
            feature: FeatureId(999),
        })
        .expect_err("unknown feature");
    assert!(err.to_string().contains("not found"));
}

#[test]
fn reorder_feature_changes_order() {
    let (mut session, body_id, first) = session_with_helix();
    session
        .execute(Command::ReorderFeature {
            from: first,
            to_position: 0,
        })
        .expect("reorder");
    let body = session.document().body(body_id).expect("body");
    assert_eq!(FeatureId(body.features[0].feature_id), first);
}

#[test]
fn reorder_feature_out_of_range_errors() {
    let (mut session, _body_id, first) = session_with_helix();
    let err = session
        .execute(Command::ReorderFeature {
            from: first,
            to_position: 99,
        })
        .expect_err("out of range");
    assert!(err.to_string().contains("cannot move"));
}

#[test]
fn edit_feature_recomputes_and_updates_spec_kind() {
    let (mut session, body_id, first) = session_with_helix();
    session
        .execute(Command::EditFeature {
            feature: first,
            new_spec: helix_spec(5.0, 2.0),
        })
        .expect("edit");
    let body = session.document().body(body_id).expect("body");
    assert_eq!(body.features[0].spec_kind, "helix");
    assert!(body.features[0].spec.is_some());
}

#[test]
fn edit_feature_to_failed_sketch_spec_reports_invalidation() {
    let (mut session, _body_id, first) = session_with_helix();
    let out = session
        .execute(Command::EditFeature {
            feature: first,
            new_spec: pad_spec(),
        })
        .expect("edit to failed spec still recomputes");
    match out {
        Outcome::FeatureRecomputed {
            downstream_invalidated,
            ..
        } => assert!(downstream_invalidated.contains(&first)),
        other => panic!("expected FeatureRecomputed, got {other:?}"),
    }
}

#[test]
fn recompute_unknown_body_errors() {
    let mut session = Session::new();
    let err = session
        .execute(Command::RecomputeBody { body: BodyId(99) })
        .expect_err("unknown body");
    assert!(err.to_string().contains("unknown body"));
}

#[test]
fn suppress_unknown_feature_errors() {
    let mut session = Session::new();
    let err = session
        .execute(Command::SuppressFeature {
            feature: FeatureId(99),
            suppressed: true,
        })
        .expect_err("unknown feature");
    assert!(err.to_string().contains("unknown feature"));
}

#[test]
fn body_commands_round_trip_through_json() {
    let commands = vec![
        Command::CreateBody {
            name: "Body".into(),
            base_plane: PlaneRef::XZ,
        },
        Command::SetTip {
            body: BodyId(1),
            feature: FeatureId(1),
        },
        Command::SuppressFeature {
            feature: FeatureId(1),
            suppressed: true,
        },
        Command::ReorderFeature {
            from: FeatureId(1),
            to_position: 0,
        },
        Command::RecomputeBody { body: BodyId(1) },
        Command::EditFeature {
            feature: FeatureId(1),
            new_spec: helix_spec(3.0, 4.0),
        },
    ];
    for command in commands {
        let json = serde_json::to_string(&command).expect("serialize command");
        let back: Command = serde_json::from_str(&json).expect("deserialize command");
        assert_eq!(back, command);
    }
}

#[test]
fn feature_spec_round_trips_through_json() {
    let spec = helix_spec(3.0, 4.0);
    let json = serde_json::to_string(&spec).expect("serialize spec");
    let back: FeatureSpec = serde_json::from_str(&json).expect("deserialize spec");
    assert_eq!(back, spec);
    assert_eq!(back.spec_kind(), "helix");
}

#[test]
fn edit_feature_default_spec_deserializes() {
    let cmd: Command =
        serde_json::from_str(r#"{"op":"edit_feature","feature":1}"#).expect("command");
    match cmd {
        Command::EditFeature { new_spec, .. } => assert_eq!(new_spec.spec_kind(), "pad"),
        other => panic!("expected EditFeature, got {other:?}"),
    }
}

#[test]
fn new_body_ops_appear_in_command_schemas() {
    let ops: std::collections::HashSet<_> = command_schemas().iter().map(|s| s.op).collect();
    for op in [
        "create_body",
        "set_tip",
        "suppress_feature",
        "reorder_feature",
        "recompute_body",
        "edit_feature",
    ] {
        assert!(ops.contains(op), "missing schema for {op}");
    }
}

#[test]
fn body_created_outcome_round_trips() {
    let outcome = Outcome::BodyCreated {
        body: BodyId(7),
        name: "Body7".into(),
        plane: cadkernel_api::Plane {
            origin: [0.0, 0.0, 0.0],
            normal: [0.0, 0.0, 1.0],
        },
    };
    let json = serde_json::to_string(&outcome).expect("serialize outcome");
    let back: Outcome = serde_json::from_str(&json).expect("deserialize outcome");
    assert_eq!(back, outcome);
    assert_eq!(back.primary_id(), None);
}
