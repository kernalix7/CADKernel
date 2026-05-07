//! Integration tests for the public API surface.
//!
//! These tests exercise the same surface an AI agent or external test
//! harness would see. They are the canonical "does the API work?" check.

use cadkernel_api::{ApiError, Command, Outcome, OutcomeKind, Session, command_schemas};

#[test]
fn empty_session_has_empty_document_and_log() {
    let session = Session::new();
    assert!(session.document().is_empty());
    assert!(session.log().is_empty());
}

#[test]
fn create_box_produces_solid_created_outcome() {
    let mut session = Session::new();
    let outcome = session
        .execute(Command::CreateBox {
            dx: 10.0,
            dy: 5.0,
            dz: 2.0,
        })
        .unwrap();
    assert_eq!(outcome.kind(), OutcomeKind::SolidCreated);
    assert_eq!(session.document().solid_count(), 1);
    assert_eq!(session.log().len(), 1);
}

#[test]
fn five_primitives_each_get_unique_solid_id() {
    let mut session = Session::new();
    let cmds = [
        Command::CreateBox {
            dx: 1.0,
            dy: 1.0,
            dz: 1.0,
        },
        Command::CreateCylinder {
            radius: 1.0,
            height: 2.0,
        },
        Command::CreateSphere { radius: 1.0 },
        Command::CreateCone {
            radius: 1.0,
            height: 2.0,
        },
        Command::CreateTorus {
            major_radius: 2.0,
            minor_radius: 0.5,
        },
    ];
    let mut ids = Vec::new();
    for c in cmds {
        let outcome = session.execute(c).unwrap();
        ids.push(outcome.primary_id().unwrap());
    }
    let unique: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(unique.len(), 5, "every primitive must get a unique SolidId");
    assert_eq!(session.document().solid_count(), 5);
}

#[test]
fn delete_solid_decrements_count_and_id_becomes_unknown() {
    let mut session = Session::new();
    let id = session
        .execute(Command::CreateBox {
            dx: 1.0,
            dy: 1.0,
            dz: 1.0,
        })
        .unwrap()
        .primary_id()
        .unwrap();
    session.execute(Command::DeleteSolid { id }).unwrap();
    assert_eq!(session.document().solid_count(), 0);
    let err = session.execute(Command::DeleteSolid { id }).unwrap_err();
    assert!(matches!(err, ApiError::UnknownSolid(_)));
}

#[test]
fn boolean_union_consumes_both_inputs_and_produces_new_solid() {
    let mut session = Session::new();
    let a = session
        .execute(Command::CreateBox {
            dx: 4.0,
            dy: 4.0,
            dz: 4.0,
        })
        .unwrap()
        .primary_id()
        .unwrap();
    let b = session
        .execute(Command::CreateSphere { radius: 2.5 })
        .unwrap()
        .primary_id()
        .unwrap();
    let outcome = session
        .execute(Command::BooleanUnion { lhs: a, rhs: b })
        .unwrap();
    let Outcome::Booleaned { result, consumed } = outcome else {
        panic!("expected Booleaned outcome, got {outcome:?}");
    };
    assert_eq!(consumed, vec![a, b]);
    assert_ne!(result, a);
    assert_ne!(result, b);
    assert_eq!(session.document().solid_count(), 1);
    // Both consumed IDs are now unknown.
    assert!(session.document().solid_label(a).is_none());
    assert!(session.document().solid_label(b).is_none());
    // Result ID resolves and has measurable mass.
    let measure = session.document().measure_solid(result).unwrap();
    assert!(measure.volume > 0.0);
}

#[test]
fn boolean_with_unknown_id_returns_error() {
    let mut session = Session::new();
    let a = session
        .execute(Command::CreateBox {
            dx: 1.0,
            dy: 1.0,
            dz: 1.0,
        })
        .unwrap()
        .primary_id()
        .unwrap();
    let phantom = cadkernel_api::SolidId(9_999);
    let err = session
        .execute(Command::BooleanUnion {
            lhs: a,
            rhs: phantom,
        })
        .unwrap_err();
    assert!(matches!(err, ApiError::UnknownSolid(_)));
    // Document state unchanged on failure.
    assert_eq!(session.document().solid_count(), 1);
}

#[test]
fn translate_shifts_centroid_by_offset() {
    let mut session = Session::new();
    let id = session
        .execute(Command::CreateBox {
            dx: 2.0,
            dy: 2.0,
            dz: 2.0,
        })
        .unwrap()
        .primary_id()
        .unwrap();
    let before = session.document().measure_solid(id).unwrap();
    session
        .execute(Command::Translate {
            id,
            dx: 10.0,
            dy: -5.0,
            dz: 3.5,
        })
        .unwrap();
    let after = session.document().measure_solid(id).unwrap();
    assert!((after.centroid[0] - (before.centroid[0] + 10.0)).abs() < 1e-9);
    assert!((after.centroid[1] - (before.centroid[1] - 5.0)).abs() < 1e-9);
    assert!((after.centroid[2] - (before.centroid[2] + 3.5)).abs() < 1e-9);
    // Volume is preserved by translation.
    assert!((after.volume - before.volume).abs() < 1e-6);
}

#[test]
fn scale_about_centroid_changes_volume_by_factor_cubed() {
    let mut session = Session::new();
    let id = session
        .execute(Command::CreateBox {
            dx: 2.0,
            dy: 2.0,
            dz: 2.0,
        })
        .unwrap()
        .primary_id()
        .unwrap();
    let before = session.document().measure_solid(id).unwrap();
    session.execute(Command::Scale { id, factor: 2.0 }).unwrap();
    let after = session.document().measure_solid(id).unwrap();
    assert!((after.volume - before.volume * 8.0).abs() < 1e-3);
    // Centroid is preserved by uniform scale about itself.
    for axis in 0..3 {
        assert!((after.centroid[axis] - before.centroid[axis]).abs() < 1e-6);
    }
}

#[test]
fn scale_with_non_positive_factor_is_rejected() {
    let mut session = Session::new();
    let id = session
        .execute(Command::CreateBox {
            dx: 1.0,
            dy: 1.0,
            dz: 1.0,
        })
        .unwrap()
        .primary_id()
        .unwrap();
    let err = session
        .execute(Command::Scale { id, factor: -1.0 })
        .unwrap_err();
    assert!(matches!(err, ApiError::InvalidArgument(_)));
}

#[test]
fn rename_preserves_solid_id() {
    let mut session = Session::new();
    let id = session
        .execute(Command::CreateBox {
            dx: 1.0,
            dy: 1.0,
            dz: 1.0,
        })
        .unwrap()
        .primary_id()
        .unwrap();
    assert_eq!(session.document().solid_label(id), Some("Box"));
    session
        .execute(Command::Rename {
            id,
            label: "Housing".into(),
        })
        .unwrap();
    assert_eq!(session.document().solid_label(id), Some("Housing"));
}

#[test]
fn new_document_clears_state_and_log() {
    let mut session = Session::new();
    session
        .execute(Command::CreateBox {
            dx: 1.0,
            dy: 1.0,
            dz: 1.0,
        })
        .unwrap();
    session
        .execute(Command::CreateSphere { radius: 1.0 })
        .unwrap();
    assert_eq!(session.log().len(), 2);
    session.execute(Command::NewDocument).unwrap();
    // After NewDocument the log was cleared, then NewDocument itself was appended.
    assert_eq!(session.log().len(), 1);
    assert!(session.document().is_empty());
}

#[test]
fn invalid_primitive_dimensions_are_rejected_without_log_pollution() {
    let mut session = Session::new();
    let err = session
        .execute(Command::CreateBox {
            dx: -1.0,
            dy: 1.0,
            dz: 1.0,
        })
        .unwrap_err();
    assert!(matches!(err, ApiError::InvalidArgument(_)));
    assert!(session.log().is_empty());
    assert!(session.document().is_empty());
}

// -- JSON / replay -----------------------------------------------------------

#[test]
fn every_command_round_trips_through_json() {
    let cmds = vec![
        Command::CreateBox {
            dx: 1.0,
            dy: 2.0,
            dz: 3.0,
        },
        Command::CreateCylinder {
            radius: 1.5,
            height: 4.0,
        },
        Command::CreateSphere { radius: 2.0 },
        Command::CreateCone {
            radius: 1.0,
            height: 3.0,
        },
        Command::CreateTorus {
            major_radius: 5.0,
            minor_radius: 1.2,
        },
        Command::BooleanUnion {
            lhs: cadkernel_api::SolidId(0),
            rhs: cadkernel_api::SolidId(1),
        },
        Command::BooleanSubtract {
            lhs: cadkernel_api::SolidId(0),
            rhs: cadkernel_api::SolidId(1),
        },
        Command::BooleanIntersect {
            lhs: cadkernel_api::SolidId(0),
            rhs: cadkernel_api::SolidId(1),
        },
        Command::Translate {
            id: cadkernel_api::SolidId(0),
            dx: 1.0,
            dy: -2.0,
            dz: 0.5,
        },
        Command::Scale {
            id: cadkernel_api::SolidId(0),
            factor: 2.0,
        },
        Command::Rename {
            id: cadkernel_api::SolidId(0),
            label: "Part".into(),
        },
        Command::DeleteSolid {
            id: cadkernel_api::SolidId(0),
        },
        Command::NewDocument,
        Command::Noop,
    ];
    for c in cmds {
        let json = serde_json::to_string(&c).unwrap();
        let back: Command = serde_json::from_str(&json).unwrap();
        assert_eq!(c, back, "round-trip mismatch for {}", c.op_name());
    }
}

#[test]
fn replay_reproduces_a_five_command_session_deterministically() {
    let log = vec![
        Command::CreateBox {
            dx: 4.0,
            dy: 4.0,
            dz: 4.0,
        },
        Command::CreateSphere { radius: 2.5 },
        Command::BooleanSubtract {
            lhs: cadkernel_api::SolidId(0),
            rhs: cadkernel_api::SolidId(1),
        },
        Command::Translate {
            id: cadkernel_api::SolidId(2),
            dx: 1.0,
            dy: 0.0,
            dz: 0.0,
        },
        Command::Rename {
            id: cadkernel_api::SolidId(2),
            label: "Bracket".into(),
        },
    ];
    let session_a = Session::replay(&log).unwrap();
    let session_b = Session::replay(&log).unwrap();
    assert_eq!(session_a.document().solid_count(), 1);
    assert_eq!(session_b.document().solid_count(), 1);

    let id = cadkernel_api::SolidId(2);
    let m_a = session_a.document().measure_solid(id).unwrap();
    let m_b = session_b.document().measure_solid(id).unwrap();
    // Determinism: same inputs ⇒ byte-identical measurement summary.
    assert_eq!(m_a, m_b);
    assert_eq!(session_a.document().solid_label(id), Some("Bracket"));
}

#[test]
fn json_log_round_trip_preserves_outcome() {
    let mut session = Session::new();
    session
        .execute(Command::CreateBox {
            dx: 2.0,
            dy: 3.0,
            dz: 4.0,
        })
        .unwrap();
    session
        .execute(Command::CreateSphere { radius: 1.0 })
        .unwrap();
    let json = session.log_to_json().unwrap();
    let replay = Session::replay_from_json(&json).unwrap();
    assert_eq!(replay.document().solid_count(), 2);
    assert_eq!(replay.log().len(), 2);
}

// -- Schemas / discovery -----------------------------------------------------

#[test]
fn command_schemas_cover_every_op_name() {
    let cmds = [
        Command::CreateBox {
            dx: 1.0,
            dy: 1.0,
            dz: 1.0,
        },
        Command::CreateCylinder {
            radius: 1.0,
            height: 1.0,
        },
        Command::CreateSphere { radius: 1.0 },
        Command::CreateCone {
            radius: 1.0,
            height: 1.0,
        },
        Command::CreateTorus {
            major_radius: 2.0,
            minor_radius: 0.5,
        },
        Command::BooleanUnion {
            lhs: cadkernel_api::SolidId(0),
            rhs: cadkernel_api::SolidId(1),
        },
        Command::BooleanSubtract {
            lhs: cadkernel_api::SolidId(0),
            rhs: cadkernel_api::SolidId(1),
        },
        Command::BooleanIntersect {
            lhs: cadkernel_api::SolidId(0),
            rhs: cadkernel_api::SolidId(1),
        },
        Command::Translate {
            id: cadkernel_api::SolidId(0),
            dx: 0.0,
            dy: 0.0,
            dz: 0.0,
        },
        Command::Scale {
            id: cadkernel_api::SolidId(0),
            factor: 1.0,
        },
        Command::Rename {
            id: cadkernel_api::SolidId(0),
            label: "x".into(),
        },
        Command::DeleteSolid {
            id: cadkernel_api::SolidId(0),
        },
        Command::Extrude {
            profile: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]],
            direction: [0.0, 0.0, 1.0],
            distance: 1.0,
            kind: cadkernel_api::ExtrudeKind::Blind,
        },
        Command::LinearPattern {
            id: cadkernel_api::SolidId(0),
            direction: [1.0, 0.0, 0.0],
            spacing: 1.0,
            count: 2,
        },
        Command::Mirror {
            id: cadkernel_api::SolidId(0),
            point: [0.0, 0.0, 0.0],
            normal: [1.0, 0.0, 0.0],
        },
        Command::NewDocument,
        Command::Noop,
    ];
    let schemas = command_schemas();
    let schema_names: std::collections::HashSet<_> = schemas.iter().map(|s| s.op).collect();
    for c in &cmds {
        assert!(
            schema_names.contains(c.op_name()),
            "command {} is missing from command_schemas()",
            c.op_name()
        );
    }
    assert_eq!(
        schemas.len(),
        cmds.len(),
        "every Command variant must have exactly one schema entry"
    );
}

#[test]
fn document_validate_is_clean_after_each_command_in_a_realistic_sequence() {
    let mut session = Session::new();
    let actions = vec![
        Command::CreateBox {
            dx: 4.0,
            dy: 4.0,
            dz: 4.0,
        },
        Command::CreateCylinder {
            radius: 1.0,
            height: 6.0,
        },
        Command::Translate {
            id: cadkernel_api::SolidId(1),
            dx: 0.0,
            dy: 0.0,
            dz: -1.0,
        },
        Command::BooleanSubtract {
            lhs: cadkernel_api::SolidId(0),
            rhs: cadkernel_api::SolidId(1),
        },
    ];
    for a in actions {
        session.execute(a).unwrap();
        assert!(
            session.document().validate().is_empty(),
            "document validation must remain clean throughout the sequence"
        );
    }
}

#[test]
fn snapshot_metadata_fields_populate_on_save() {
    use cadkernel_api::{Command, Session, SessionSnapshot};

    let mut session = Session::new();
    session
        .execute(Command::CreateBox {
            dx: 1.0,
            dy: 1.0,
            dz: 1.0,
        })
        .unwrap();
    session
        .execute(Command::CreateSphere { radius: 0.5 })
        .unwrap();

    let json = session
        .save_to_json_with_label(Some("test-label".into()))
        .unwrap();
    let snap: SessionSnapshot = serde_json::from_str(&json).unwrap();

    assert_eq!(snap.schema_version, SessionSnapshot::CURRENT_SCHEMA);
    assert_eq!(snap.cursor, 2);
    assert_eq!(snap.log_position, 2);
    assert_eq!(snap.label.as_deref(), Some("test-label"));
    assert!(!snap.document_hash.is_empty());
    assert_eq!(snap.document_hash.len(), 16);
    assert!(snap.timestamp >= 0);

    // Round-trip: load the snapshot back.
    let restored = Session::load_from_json(&json).unwrap();
    assert_eq!(restored.document().solid_count(), 2);
}

#[test]
fn old_schema_v1_snapshot_without_metadata_still_loads() {
    use cadkernel_api::Session;

    // A schema-v1 snapshot saved before A2 metadata fields were added.
    let legacy = r#"{
        "schema_version": 1,
        "commands": [
            { "op": "create_box", "dx": 2.0, "dy": 2.0, "dz": 2.0 }
        ],
        "cursor": 1
    }"#;
    let restored = Session::load_from_json(legacy).unwrap();
    assert_eq!(restored.document().solid_count(), 1);
}

#[test]
fn translate_coalesces_within_window() {
    use cadkernel_api::{Command, Session, SolidId};

    let mut session = Session::new();
    session
        .execute(Command::CreateBox {
            dx: 1.0,
            dy: 1.0,
            dz: 1.0,
        })
        .unwrap();
    let id = SolidId(0);

    // Three rapid translates of the same id within 1 s — should coalesce
    // into a single Translate(sum) log entry.
    session
        .execute(Command::Translate {
            id,
            dx: 1.0,
            dy: 0.0,
            dz: 0.0,
        })
        .unwrap();
    session
        .execute(Command::Translate {
            id,
            dx: 2.0,
            dy: 0.0,
            dz: 0.0,
        })
        .unwrap();
    session
        .execute(Command::Translate {
            id,
            dx: 0.0,
            dy: 4.0,
            dz: 0.0,
        })
        .unwrap();

    // Log = [CreateBox, Translate(merged 3.0, 4.0, 0.0)]
    assert_eq!(session.log().len(), 2);
    match &session.log()[1] {
        Command::Translate { id: t, dx, dy, dz } => {
            assert_eq!(*t, id);
            assert!((dx - 3.0).abs() < 1e-9);
            assert!((dy - 4.0).abs() < 1e-9);
            assert!((dz - 0.0).abs() < 1e-9);
        }
        other => panic!("expected coalesced Translate, got {other:?}"),
    }
    // Only 2 history events (CreateBox + 1 coalesced Translate, not 4).
    assert_eq!(session.document().history().len(), 2);
}

#[test]
fn coalescing_disabled_when_window_is_zero() {
    use cadkernel_api::{Command, Session, SolidId};

    let mut session = Session::new();
    session.set_coalesce_window_ms(0);
    session
        .execute(Command::CreateBox {
            dx: 1.0,
            dy: 1.0,
            dz: 1.0,
        })
        .unwrap();
    let id = SolidId(0);
    session
        .execute(Command::Translate {
            id,
            dx: 1.0,
            dy: 0.0,
            dz: 0.0,
        })
        .unwrap();
    session
        .execute(Command::Translate {
            id,
            dx: 1.0,
            dy: 0.0,
            dz: 0.0,
        })
        .unwrap();
    // Both translates should be separate log entries.
    assert_eq!(session.log().len(), 3);
}

#[test]
fn rename_coalesces_keeps_only_last_label() {
    use cadkernel_api::{Command, Session, SolidId};

    let mut session = Session::new();
    session
        .execute(Command::CreateBox {
            dx: 1.0,
            dy: 1.0,
            dz: 1.0,
        })
        .unwrap();
    let id = SolidId(0);
    for label in ["A", "AB", "ABC", "ABCD"] {
        session
            .execute(Command::Rename {
                id,
                label: label.to_string(),
            })
            .unwrap();
    }
    assert_eq!(session.log().len(), 2);
    match &session.log()[1] {
        Command::Rename { label, .. } => assert_eq!(label, "ABCD"),
        other => panic!("expected coalesced Rename, got {other:?}"),
    }
}

#[test]
fn session_save_cadk_round_trip_preserves_solid_count() {
    use cadkernel_api::{Command, Session};

    let mut session = Session::new();
    session
        .execute(Command::CreateBox {
            dx: 10.0,
            dy: 5.0,
            dz: 2.0,
        })
        .unwrap();
    session
        .execute(Command::CreateSphere { radius: 1.5 })
        .unwrap();
    session
        .execute(Command::CreateCylinder {
            radius: 0.5,
            height: 4.0,
        })
        .unwrap();

    let bytes = session.save_cadk().unwrap();
    assert_eq!(&bytes[..4], b"CADK");

    let restored = Session::load_cadk(&bytes).unwrap();
    assert_eq!(restored.document().solid_count(), 3);
    assert_eq!(restored.log().len(), 3);
}

#[test]
fn session_save_cadk_with_thumbnail_round_trips_payload() {
    use cadkernel_api::{cadk, Command, Session};

    let mut session = Session::new();
    session
        .execute(Command::CreateBox {
            dx: 4.0,
            dy: 3.0,
            dz: 2.0,
        })
        .unwrap();

    let thumbnail: Vec<u8> = (0u8..200).collect();
    let bytes = session.save_cadk_with_thumbnail(&thumbnail).unwrap();

    let restored = Session::load_cadk(&bytes).unwrap();
    assert_eq!(restored.document().solid_count(), 1);

    let recovered_thumb = cadk::decode_thumbnail(&bytes).unwrap().unwrap();
    assert_eq!(recovered_thumb, thumbnail);
}

#[test]
fn linear_pattern_outcome_reports_pattern_id_instance_count_and_total_features() {
    use cadkernel_api::{Command, Outcome, Session, SolidId};

    let mut session = Session::new();
    let create = session
        .execute(Command::CreateBox {
            dx: 2.0,
            dy: 2.0,
            dz: 2.0,
        })
        .unwrap();
    let source_id = match create {
        Outcome::SolidCreated { id, .. } => id,
        other => panic!("expected SolidCreated, got {other:?}"),
    };

    let pattern = session
        .execute(Command::LinearPattern {
            id: source_id,
            direction: [1.0, 0.0, 0.0],
            spacing: 5.0,
            count: 4,
        })
        .unwrap();

    match pattern {
        Outcome::PatternCreated {
            pattern_id,
            instance_count,
            total_features,
            ids,
        } => {
            assert_eq!(pattern_id, source_id, "pattern_id must echo the source");
            assert_eq!(instance_count, 4, "instance_count must equal Command::count");
            assert_eq!(
                total_features, 3,
                "total_features must equal new instances (count - 1)"
            );
            assert_eq!(ids.len(), 4, "ids list must include original + new copies");
            assert_eq!(ids[0], source_id, "ids[0] must be the source/original");
            // every ID after [0] must be unique and not equal to the source
            assert!(ids[1..].iter().all(|i| *i != source_id));
            assert_eq!(
                ids.iter().copied().collect::<std::collections::HashSet<_>>().len(),
                4,
                "all pattern ids must be unique"
            );
        }
        other => panic!("expected PatternCreated, got {other:?}"),
    }

    // Verify the JSON wire format carries the new fields (AI-/test-friendly schema).
    let pattern2 = session
        .execute(Command::LinearPattern {
            id: source_id,
            direction: [0.0, 1.0, 0.0],
            spacing: 3.0,
            count: 2,
        })
        .unwrap();
    let json = serde_json::to_value(&pattern2).unwrap();
    assert_eq!(json["kind"], "pattern_created");
    assert_eq!(json["pattern_id"], serde_json::json!(source_id.0));
    assert_eq!(json["instance_count"], serde_json::json!(2));
    assert_eq!(json["total_features"], serde_json::json!(1));
    assert!(json["ids"].is_array());

    // Sanity: count = 1 is rejected (existing API contract).
    let bad = session.execute(Command::LinearPattern {
        id: source_id,
        direction: [1.0, 0.0, 0.0],
        spacing: 1.0,
        count: 1,
    });
    assert!(bad.is_err(), "count < 2 must be rejected");

    // Drop unused warning for SolidId.
    let _: SolidId = source_id;
}

#[test]
fn extrude_kind_blind_is_default_and_matches_legacy_json_shape() {
    use cadkernel_api::{Command, ExtrudeKind, Outcome, Session};

    // ExtrudeKind defaults to Blind so a JSON document missing the `kind`
    // field round-trips into a Blind extrusion identical to pre-A2.1
    // behavior.
    let legacy_json = r#"{
        "op": "extrude",
        "profile": [[0,0,0],[2,0,0],[2,2,0],[0,2,0]],
        "direction": [0,0,1],
        "distance": 5
    }"#;
    let cmd: Command = serde_json::from_str(legacy_json).unwrap();
    match &cmd {
        Command::Extrude { kind, .. } => assert_eq!(*kind, ExtrudeKind::Blind),
        other => panic!("expected Extrude, got {other:?}"),
    }
    let mut session = Session::new();
    let out = session.execute(cmd).unwrap();
    assert!(matches!(out, Outcome::SolidCreated { .. }));
}

#[test]
fn extrude_kind_mid_plane_centers_solid_and_total_span_matches_distance() {
    use cadkernel_api::{Command, ExtrudeKind, Outcome, Session};

    let mut session = Session::new();
    let blind = session
        .execute(Command::Extrude {
            profile: vec![[0.0, 0.0, 0.0], [4.0, 0.0, 0.0], [4.0, 4.0, 0.0], [0.0, 4.0, 0.0]],
            direction: [0.0, 0.0, 1.0],
            distance: 10.0,
            kind: ExtrudeKind::Blind,
        })
        .unwrap();
    let blind_id = match blind {
        Outcome::SolidCreated { id, .. } => id,
        other => panic!("expected SolidCreated, got {other:?}"),
    };
    let blind_m = session.document().measure_solid(blind_id).unwrap();

    let mid = session
        .execute(Command::Extrude {
            profile: vec![[0.0, 0.0, 0.0], [4.0, 0.0, 0.0], [4.0, 4.0, 0.0], [0.0, 4.0, 0.0]],
            direction: [0.0, 0.0, 1.0],
            distance: 10.0,
            kind: ExtrudeKind::MidPlane,
        })
        .unwrap();
    let mid_id = match mid {
        Outcome::SolidCreated { id, .. } => id,
        other => panic!("expected SolidCreated, got {other:?}"),
    };
    let mid_m = session.document().measure_solid(mid_id).unwrap();

    // 4x4 base, height 10 → volume 160 in both kinds.
    assert!((blind_m.volume - 160.0).abs() < 1e-6);
    assert!((mid_m.volume - 160.0).abs() < 1e-6);
    // Blind: profile at z=0, extrudes up → centroid z = 5.0.
    assert!((blind_m.centroid[2] - 5.0).abs() < 1e-6);
    // MidPlane: profile shifted by -d/2 = -5, extrudes 10 → centroid z = 0.
    assert!(mid_m.centroid[2].abs() < 1e-6);
}

#[test]
fn extrude_kind_two_sided_extends_in_both_directions_with_correct_total_span() {
    use cadkernel_api::{Command, ExtrudeKind, Outcome, Session};

    let mut session = Session::new();
    let two = session
        .execute(Command::Extrude {
            profile: vec![[0.0, 0.0, 0.0], [3.0, 0.0, 0.0], [3.0, 3.0, 0.0], [0.0, 3.0, 0.0]],
            direction: [0.0, 0.0, 1.0],
            distance: 4.0,
            kind: ExtrudeKind::TwoSided { back_distance: 2.0 },
        })
        .unwrap();
    let two_id = match two {
        Outcome::SolidCreated { id, .. } => id,
        other => panic!("expected SolidCreated, got {other:?}"),
    };
    let m = session.document().measure_solid(two_id).unwrap();
    // 3x3 base, total height 6 → volume 54.
    assert!((m.volume - 54.0).abs() < 1e-6, "expected volume 54, got {}", m.volume);
    // Centroid z = midpoint of [-2, +4] = 1.0.
    assert!((m.centroid[2] - 1.0).abs() < 1e-6, "expected centroid z=1, got {}", m.centroid[2]);

    // back_distance must be positive.
    let bad = session.execute(Command::Extrude {
        profile: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]],
        direction: [0.0, 0.0, 1.0],
        distance: 1.0,
        kind: ExtrudeKind::TwoSided { back_distance: 0.0 },
    });
    assert!(bad.is_err(), "back_distance = 0 must be rejected");

    // JSON wire format: TwoSided uses tagged union with mode = two_sided.
    let json = serde_json::to_value(&Command::Extrude {
        profile: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]],
        direction: [0.0, 0.0, 1.0],
        distance: 4.0,
        kind: ExtrudeKind::TwoSided { back_distance: 2.0 },
    })
    .unwrap();
    assert_eq!(json["op"], "extrude");
    assert_eq!(json["kind"]["mode"], "two_sided");
    assert_eq!(json["kind"]["back_distance"], serde_json::json!(2.0));
}
