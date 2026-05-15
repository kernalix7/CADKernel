use cadkernel_api::{
    ApiError, Command, EntityId, Outcome, PadDirection, PadType, PlaneRef, Session, SketchEdit,
    SketchId, SketchRef,
};
use cadkernel_topology::{EntityKind, OperationId, Tag};

fn square_edits(side: f64) -> Vec<SketchEdit> {
    vec![
        SketchEdit::AddPoint { x: 0.0, y: 0.0 },
        SketchEdit::AddPoint { x: side, y: 0.0 },
        SketchEdit::AddPoint { x: side, y: side },
        SketchEdit::AddPoint { x: 0.0, y: side },
        SketchEdit::AddLine { start: 0, end: 1 },
        SketchEdit::AddLine { start: 1, end: 2 },
        SketchEdit::AddLine { start: 2, end: 3 },
        SketchEdit::AddLine { start: 3, end: 0 },
    ]
}

fn create_square_sketch(session: &mut Session) -> SketchId {
    let out = session
        .execute(Command::CreateSketch {
            plane: PlaneRef::XY,
            name: "Sketch".into(),
        })
        .expect("CreateSketch");
    let sketch_id = match out {
        Outcome::SketchCreated { sketch_id, .. } => sketch_id,
        other => panic!("expected SketchCreated, got {other:?}"),
    };
    session
        .execute(Command::EditSketch {
            sketch: sketch_id,
            edits: square_edits(1.0),
        })
        .expect("EditSketch");
    sketch_id
}

#[test]
fn document_sketch_ids_are_monotonic_and_zero_is_sentinel() {
    let mut session = Session::new();
    for idx in 0..3 {
        session
            .execute(Command::CreateSketch {
                plane: PlaneRef::XY,
                name: format!("Sketch{}", idx + 1),
            })
            .expect("CreateSketch");
    }

    assert_eq!(
        session.document().sketch_ids(),
        vec![SketchId(1), SketchId(2), SketchId(3)]
    );
    assert_eq!(session.document().sketch_count(), 3);
    assert!(session.document().sketch(SketchId(0)).is_none());
    assert_eq!(session.document().active_sketch(), Some(SketchId(1)));
}

#[test]
fn create_sketch_persists_empty_sketch() {
    let mut session = Session::new();
    let out = session
        .execute(Command::CreateSketch {
            plane: PlaneRef::XZ,
            name: "Profile".into(),
        })
        .expect("CreateSketch");

    assert!(matches!(out, Outcome::SketchCreated { sketch_id: SketchId(1), .. }));
    let sketch = session.document().sketch(SketchId(1)).expect("sketch");
    assert_eq!(sketch.name, "Profile");
    assert!(sketch.entities.is_empty());
}

#[test]
fn edit_sketch_adds_profile_entities() {
    let mut session = Session::new();
    let sketch_id = create_square_sketch(&mut session);
    let sketch = session.document().sketch(sketch_id).expect("sketch");

    assert_eq!(sketch.entities.len(), 8);
    assert_eq!(session.document().active_sketch(), Some(sketch_id));
}

#[test]
fn edit_sketch_rejects_unknown_id() {
    let mut session = Session::new();
    let err = session
        .execute(Command::EditSketch {
            sketch: SketchId(99),
            edits: square_edits(1.0),
        })
        .unwrap_err();

    assert!(matches!(err, ApiError::InvalidArgument(msg) if msg.contains("Track 6")));
}

#[test]
fn delete_sketch_without_dependents_removes_slot() {
    let mut session = Session::new();
    let sketch_id = create_square_sketch(&mut session);

    session
        .execute(Command::DeleteSketch { sketch: sketch_id })
        .expect("DeleteSketch");

    assert!(session.document().sketch(sketch_id).is_none());
    assert_eq!(session.document().sketch_count(), 0);
}

#[test]
fn remove_missing_segment_reports_error() {
    let mut session = Session::new();
    let sketch_id = create_square_sketch(&mut session);
    let err = session
        .execute(Command::EditSketch {
            sketch: sketch_id,
            edits: vec![SketchEdit::RemoveSegment {
                entity: EntityId::Line { index: 9 },
            }],
        })
        .unwrap_err();

    assert!(matches!(err, ApiError::InvalidArgument(msg) if msg.contains("not found")));
}

#[test]
fn pad_resolves_persisted_sketch_ref() {
    let mut session = Session::new();
    session
        .execute(Command::CreateBox {
            dx: 2.0,
            dy: 2.0,
            dz: 1.0,
        })
        .expect("base");
    let sketch_id = create_square_sketch(&mut session);

    let out = session
        .execute(Command::Pad {
            sketch: SketchRef { sketch_id },
            distance: 1.0,
            direction: PadDirection::Normal,
            symmetric: false,
            type_: PadType::Blind,
        })
        .expect("Pad through SketchRef");

    assert!(matches!(out, Outcome::FeatureAdded { .. }));
    assert_eq!(session.document().sketch_count(), 1);
}

#[test]
fn delete_sketch_with_dependents_is_rejected() {
    let mut session = Session::new();
    session
        .execute(Command::CreateBox {
            dx: 2.0,
            dy: 2.0,
            dz: 1.0,
        })
        .expect("base");
    let sketch_id = create_square_sketch(&mut session);
    session
        .execute(Command::Pad {
            sketch: SketchRef { sketch_id },
            distance: 1.0,
            direction: PadDirection::Normal,
            symmetric: false,
            type_: PadType::Blind,
        })
        .expect("Pad");

    let err = session
        .execute(Command::DeleteSketch { sketch: sketch_id })
        .unwrap_err();

    assert!(matches!(err, ApiError::InvalidArgument(msg) if msg.contains("dependent features")));
}

#[test]
fn edit_sketch_recomputes_dependent_feature() {
    let mut session = Session::new();
    session
        .execute(Command::CreateBox {
            dx: 2.0,
            dy: 2.0,
            dz: 1.0,
        })
        .expect("base");
    let sketch_id = create_square_sketch(&mut session);
    session
        .execute(Command::Pad {
            sketch: SketchRef { sketch_id },
            distance: 1.0,
            direction: PadDirection::Normal,
            symmetric: false,
            type_: PadType::Blind,
        })
        .expect("Pad");

    let out = session
        .execute(Command::EditSketch {
            sketch: sketch_id,
            edits: vec![SketchEdit::UpdateParameter {
                name: "origin_x".into(),
                value: 0.25,
            }],
        })
        .expect("EditSketch recompute");

    assert!(matches!(
        out,
        Outcome::FeatureRecomputed {
            downstream_invalidated,
            ..
        } if !downstream_invalidated.is_empty()
    ));
}

#[test]
fn map_sketch_to_face_updates_plane_when_face_index_resolves() {
    let mut session = Session::new();
    session
        .execute(Command::CreateBox {
            dx: 2.0,
            dy: 2.0,
            dz: 1.0,
        })
        .expect("base");
    let sketch_id = create_square_sketch(&mut session);

    session
        .execute(Command::MapSketchToFace {
            sketch: sketch_id,
            face: cadkernel_api::FaceRef {
                solid: cadkernel_api::SolidId(0),
                tag: Tag::generated(EntityKind::Face, OperationId(1), 0),
            },
        })
        .expect("MapSketchToFace");

    let sketch = session.document().sketch(sketch_id).expect("sketch");
    let n = sketch.plane.normal;
    assert!(n.iter().any(|v| v.abs() > 0.5), "normal should be non-zero: {n:?}");
}

#[test]
fn tier4_commands_round_trip_json() {
    let commands = vec![
        Command::CreateSketch {
            plane: PlaneRef::XY,
            name: "Sketch".into(),
        },
        Command::EditSketch {
            sketch: SketchId(1),
            edits: square_edits(1.0),
        },
        Command::DeleteSketch {
            sketch: SketchId(1),
        },
        Command::MapSketchToFace {
            sketch: SketchId(1),
            face: cadkernel_api::FaceRef {
                solid: cadkernel_api::SolidId(0),
                tag: Tag::generated(EntityKind::Face, OperationId(1), 0),
            },
        },
    ];

    for cmd in commands {
        let json = serde_json::to_string(&cmd).expect("serialize");
        let decoded: Command = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded, cmd);
    }
}

#[test]
fn old_command_logs_still_replay_without_sketch_fields() {
    let json = r#"[{"op":"create_box","dx":1.0,"dy":1.0,"dz":1.0}]"#;
    let session = Session::replay_from_json(json).expect("old log");

    assert_eq!(session.document().solid_count(), 1);
    assert_eq!(session.document().sketch_count(), 0);
}
