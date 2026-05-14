use cadkernel_api::{ApiError, Command, ExtrudeKind, Outcome, Session, SolidId};

fn created_id(outcome: Outcome) -> SolidId {
    match outcome {
        Outcome::SolidCreated { id, .. } => id,
        other => panic!("expected SolidCreated, got {other:?}"),
    }
}

fn box_id(session: &mut Session, size: f64) -> SolidId {
    created_id(
        session
            .execute(Command::CreateBox {
                dx: size,
                dy: size,
                dz: size,
            })
            .unwrap(),
    )
}

fn triangle_at_z(z: f64) -> Vec<[f64; 3]> {
    vec![[0.0, 0.0, z], [1.0, 0.0, z], [0.0, 1.0, z]]
}

fn assert_close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn through_all_spans_existing_document_bbox() {
    let mut session = Session::new();
    box_id(&mut session, 10.0);

    let extrude = created_id(
        session
            .execute(Command::Extrude {
                profile: triangle_at_z(-5.0),
                direction: [0.0, 0.0, 1.0],
                distance: 0.0,
                kind: ExtrudeKind::ThroughAll,
            })
            .unwrap(),
    );

    let bbox = session.document().bounding_box(extrude).unwrap();
    assert!(bbox.max[2] - bbox.min[2] >= 10.0);
}

#[test]
fn through_all_empty_document_errors() {
    let mut session = Session::new();
    let err = session
        .execute(Command::Extrude {
            profile: triangle_at_z(0.0),
            direction: [0.0, 0.0, 1.0],
            distance: 0.0,
            kind: ExtrudeKind::ThroughAll,
        })
        .unwrap_err();

    match err {
        ApiError::InvalidArgument(msg) => assert!(msg.contains("ThroughAll requires")),
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

#[test]
fn through_all_bbox_span_uses_global_span_plus_margin() {
    let mut session = Session::new();
    box_id(&mut session, 1.0);
    let second = box_id(&mut session, 1.0);
    session
        .execute(Command::Translate {
            id: second,
            dx: 0.0,
            dy: 0.0,
            dz: 20.0,
        })
        .unwrap();

    let extrude = created_id(
        session
            .execute(Command::Extrude {
                profile: triangle_at_z(-1.0),
                direction: [0.0, 0.0, 1.0],
                distance: -10.0,
                kind: ExtrudeKind::ThroughAll,
            })
            .unwrap(),
    );

    let bbox = session.document().bounding_box(extrude).unwrap();
    assert_close(bbox.max[2] - bbox.min[2], 21.21, 1e-8);
}

#[test]
fn up_to_face_extrudes_to_target_face_plane() {
    let mut session = Session::new();
    let target = box_id(&mut session, 10.0);

    let extrude = created_id(
        session
            .execute(Command::Extrude {
                profile: triangle_at_z(-5.0),
                direction: [0.0, 0.0, 1.0],
                distance: 0.0,
                kind: ExtrudeKind::UpToFace {
                    face_solid: target,
                    face_index: 0,
                },
            })
            .unwrap(),
    );

    let bbox = session.document().bounding_box(extrude).unwrap();
    assert_close(bbox.min[2], -5.0, 1e-9);
    assert!(bbox.max[2] <= 1e-9, "bbox passed target plane: {bbox:?}");
}

#[test]
fn up_to_face_missing_target_mentions_solid_id() {
    let mut session = Session::new();
    let err = session
        .execute(Command::Extrude {
            profile: triangle_at_z(-5.0),
            direction: [0.0, 0.0, 1.0],
            distance: 0.0,
            kind: ExtrudeKind::UpToFace {
                face_solid: SolidId(999),
                face_index: 0,
            },
        })
        .unwrap_err();

    match err {
        ApiError::InvalidArgument(msg) => assert!(msg.contains("solid#999")),
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

#[test]
fn up_to_face_out_of_range_face_index_errors() {
    let mut session = Session::new();
    let target = box_id(&mut session, 1.0);
    let err = session
        .execute(Command::Extrude {
            profile: triangle_at_z(-1.0),
            direction: [0.0, 0.0, 1.0],
            distance: 0.0,
            kind: ExtrudeKind::UpToFace {
                face_solid: target,
                face_index: 9999,
            },
        })
        .unwrap_err();

    match err {
        ApiError::InvalidArgument(msg) => assert!(msg.contains("face_index 9999")),
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

#[test]
fn up_to_face_parallel_target_plane_errors() {
    let mut session = Session::new();
    let target = box_id(&mut session, 1.0);
    let err = session
        .execute(Command::Extrude {
            profile: triangle_at_z(-1.0),
            direction: [0.0, 0.0, 1.0],
            distance: 0.0,
            kind: ExtrudeKind::UpToFace {
                face_solid: target,
                face_index: 2,
            },
        })
        .unwrap_err();

    match err {
        ApiError::InvalidArgument(msg) => assert!(msg.contains("parallel")),
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

#[test]
fn extrude_kind_new_variants_round_trip_through_json() {
    let through = serde_json::to_value(ExtrudeKind::ThroughAll).unwrap();
    assert_eq!(through, serde_json::json!({ "mode": "through_all" }));
    let parsed: ExtrudeKind = serde_json::from_value(through).unwrap();
    assert_eq!(parsed, ExtrudeKind::ThroughAll);

    let up_to = serde_json::to_value(ExtrudeKind::UpToFace {
        face_solid: SolidId(7),
        face_index: 3,
    })
    .unwrap();
    assert_eq!(
        up_to,
        serde_json::json!({
            "mode": "up_to_face",
            "face_solid": 7,
            "face_index": 3
        })
    );
    let parsed: ExtrudeKind = serde_json::from_value(up_to).unwrap();
    assert_eq!(
        parsed,
        ExtrudeKind::UpToFace {
            face_solid: SolidId(7),
            face_index: 3
        }
    );
}
