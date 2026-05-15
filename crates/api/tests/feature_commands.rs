use cadkernel_api::{
    ApiError, AxisRef, BodyId, ChamferMode, Command, DraftDirection, EdgeRef, FaceRef, FeatureId,
    HoleKind, LoftMode, Outcome, PadDirection, PadType, Plane, PocketType, Session, ShellMode,
    SketchId, SketchRef, SolidId, SweepMode, Tag, VariableRadius, command_schemas,
};
use cadkernel_topology::EntityKind;

fn sketch(id: u64) -> SketchRef {
    SketchRef {
        sketch_id: SketchId(id),
    }
}

fn tag(kind: EntityKind) -> Tag {
    Tag::new(kind, Vec::new())
}

fn face_ref() -> FaceRef {
    FaceRef {
        solid: SolidId(0),
        tag: tag(EntityKind::Face),
    }
}

fn edge_ref() -> EdgeRef {
    EdgeRef {
        solid: SolidId(0),
        tag: tag(EntityKind::Edge),
    }
}

fn assert_round_trip(cmd: Command, op: &str) {
    let json = serde_json::to_string(&cmd).expect("serialize command");
    let back: Command = serde_json::from_str(&json).expect("deserialize command");
    assert_eq!(cmd, back);
    assert!(json.contains(&format!("\"op\":\"{op}\"")), "{json}");
}

fn assert_invalid_contains(result: Result<Outcome, ApiError>, needle: &str) {
    match result.expect_err("command must fail") {
        ApiError::InvalidArgument(msg) => assert!(msg.contains(needle), "{msg}"),
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

#[test]
fn pad_round_trips() {
    assert_round_trip(
        Command::Pad {
            sketch: sketch(7),
            distance: 10.0,
            direction: PadDirection::Normal,
            symmetric: false,
            type_: PadType::Blind,
        },
        "pad",
    );
}

#[test]
fn pocket_round_trips() {
    assert_round_trip(
        Command::Pocket {
            sketch: sketch(8),
            distance: 4.0,
            through_all: true,
            type_: PocketType::ThroughAll,
        },
        "pocket",
    );
}

#[test]
fn revolve_round_trips() {
    assert_round_trip(
        Command::Revolve {
            sketch: sketch(9),
            axis: AxisRef::Z,
            angle_rad: std::f64::consts::PI,
            symmetric: true,
        },
        "revolve",
    );
}

#[test]
fn groove_round_trips() {
    assert_round_trip(
        Command::Groove {
            sketch: sketch(10),
            axis: AxisRef::X,
            angle_rad: 1.0,
        },
        "groove",
    );
}

#[test]
fn hole_round_trips() {
    assert_round_trip(
        Command::Hole {
            face: face_ref(),
            position: [1.0, 2.0],
            radius: 0.5,
            depth: 3.0,
            through_all: false,
            kind: HoleKind::Counterbore,
        },
        "hole",
    );
}

#[test]
fn sweep_round_trips() {
    assert_round_trip(
        Command::Sweep {
            profile_sketch: sketch(1),
            path_sketch: sketch(2),
            mode: SweepMode::Frenet,
        },
        "sweep",
    );
}

#[test]
fn loft_round_trips() {
    assert_round_trip(
        Command::Loft {
            profiles: vec![sketch(1), sketch(2), sketch(3)],
            mode: LoftMode::Smooth,
            ruled: true,
            closed: false,
        },
        "loft",
    );
}

#[test]
fn helix_round_trips() {
    assert_round_trip(
        Command::Helix {
            axis: AxisRef::Custom {
                position: [0.0, 0.0, 1.0],
                direction: [0.0, 0.0, 1.0],
            },
            radius: 3.0,
            pitch: 2.0,
            height: 10.0,
            turns: 5.0,
            cone_angle: 0.1,
        },
        "helix",
    );
}

#[test]
fn fillet_round_trips() {
    assert_round_trip(
        Command::Fillet {
            edges: vec![edge_ref()],
            radius: 1.0,
            variable: Some(VariableRadius {
                samples: vec![(0.0, 1.0), (1.0, 1.5)],
            }),
        },
        "fillet",
    );
}

#[test]
fn chamfer_round_trips() {
    assert_round_trip(
        Command::Chamfer {
            edges: vec![edge_ref()],
            distance: 1.0,
            mode: ChamferMode::DistanceAngle,
        },
        "chamfer",
    );
}

#[test]
fn shell_round_trips() {
    assert_round_trip(
        Command::Shell {
            solid: SolidId(0),
            removed_faces: vec![face_ref()],
            thickness: 0.5,
            mode: ShellMode::Outward,
        },
        "shell",
    );
}

#[test]
fn draft_round_trips() {
    assert_round_trip(
        Command::Draft {
            faces: vec![face_ref()],
            neutral_plane: face_ref(),
            angle_rad: 0.2,
            direction: DraftDirection::Push,
        },
        "draft",
    );
}

#[test]
fn pad_omits_defaults() {
    let cmd: Command =
        serde_json::from_str(r#"{"op":"pad","sketch":{"sketch_id":1},"distance":5.0}"#)
            .expect("deserialize minimal pad");
    match cmd {
        Command::Pad {
            direction,
            symmetric,
            type_,
            ..
        } => {
            assert_eq!(direction, PadDirection::Normal);
            assert!(!symmetric);
            assert_eq!(type_, PadType::Blind);
        }
        other => panic!("expected Pad, got {other:?}"),
    }
}

#[test]
fn pocket_omits_defaults() {
    let cmd: Command =
        serde_json::from_str(r#"{"op":"pocket","sketch":{"sketch_id":1},"distance":5.0}"#)
            .expect("deserialize minimal pocket");
    match cmd {
        Command::Pocket {
            through_all, type_, ..
        } => {
            assert!(!through_all);
            assert_eq!(type_, PocketType::Blind);
        }
        other => panic!("expected Pocket, got {other:?}"),
    }
}

#[test]
fn revolve_omits_defaults() {
    let cmd: Command = serde_json::from_str(
        r#"{"op":"revolve","sketch":{"sketch_id":1},"axis":{"axis":"z"},"angle_rad":1.0}"#,
    )
    .expect("deserialize minimal revolve");
    match cmd {
        Command::Revolve { symmetric, .. } => assert!(!symmetric),
        other => panic!("expected Revolve, got {other:?}"),
    }
}

#[test]
fn groove_minimal_deserializes() {
    let cmd: Command = serde_json::from_str(
        r#"{"op":"groove","sketch":{"sketch_id":1},"axis":{"axis":"x"},"angle_rad":1.0}"#,
    )
    .expect("deserialize minimal groove");
    assert!(matches!(cmd, Command::Groove { .. }));
}

#[test]
fn hole_omits_defaults() {
    let face = serde_json::to_string(&face_ref()).expect("face json");
    let cmd: Command = serde_json::from_str(&format!(
        r#"{{"op":"hole","face":{face},"position":[0.0,0.0],"radius":1.0,"depth":2.0}}"#
    ))
    .expect("deserialize minimal hole");
    match cmd {
        Command::Hole {
            through_all, kind, ..
        } => {
            assert!(!through_all);
            assert_eq!(kind, HoleKind::Simple);
        }
        other => panic!("expected Hole, got {other:?}"),
    }
}

#[test]
fn sweep_omits_defaults() {
    let cmd: Command = serde_json::from_str(
        r#"{"op":"sweep","profile_sketch":{"sketch_id":1},"path_sketch":{"sketch_id":2}}"#,
    )
    .expect("deserialize minimal sweep");
    match cmd {
        Command::Sweep { mode, .. } => assert_eq!(mode, SweepMode::Standard),
        other => panic!("expected Sweep, got {other:?}"),
    }
}

#[test]
fn loft_omits_defaults() {
    let cmd: Command =
        serde_json::from_str(r#"{"op":"loft","profiles":[{"sketch_id":1},{"sketch_id":2}]}"#)
            .expect("deserialize minimal loft");
    match cmd {
        Command::Loft {
            mode,
            ruled,
            closed,
            ..
        } => {
            assert_eq!(mode, LoftMode::Straight);
            assert!(!ruled);
            assert!(!closed);
        }
        other => panic!("expected Loft, got {other:?}"),
    }
}

#[test]
fn helix_omits_defaults() {
    let cmd: Command = serde_json::from_str(
        r#"{"op":"helix","axis":{"axis":"z"},"radius":1.0,"pitch":1.0,"height":5.0,"turns":5.0}"#,
    )
    .expect("deserialize minimal helix");
    match cmd {
        Command::Helix { cone_angle, .. } => assert_eq!(cone_angle, 0.0),
        other => panic!("expected Helix, got {other:?}"),
    }
}

#[test]
fn fillet_omits_defaults() {
    let edge = serde_json::to_string(&edge_ref()).expect("edge json");
    let cmd: Command = serde_json::from_str(&format!(
        r#"{{"op":"fillet","edges":[{edge}],"radius":1.0}}"#
    ))
    .expect("deserialize minimal fillet");
    match cmd {
        Command::Fillet { variable, .. } => assert_eq!(variable, None),
        other => panic!("expected Fillet, got {other:?}"),
    }
}

#[test]
fn chamfer_omits_defaults() {
    let edge = serde_json::to_string(&edge_ref()).expect("edge json");
    let cmd: Command = serde_json::from_str(&format!(
        r#"{{"op":"chamfer","edges":[{edge}],"distance":1.0}}"#
    ))
    .expect("deserialize minimal chamfer");
    match cmd {
        Command::Chamfer { mode, .. } => assert_eq!(mode, ChamferMode::Equal),
        other => panic!("expected Chamfer, got {other:?}"),
    }
}

#[test]
fn shell_omits_defaults() {
    let face = serde_json::to_string(&face_ref()).expect("face json");
    let cmd: Command = serde_json::from_str(&format!(
        r#"{{"op":"shell","solid":0,"removed_faces":[{face}],"thickness":1.0}}"#
    ))
    .expect("deserialize minimal shell");
    match cmd {
        Command::Shell { mode, .. } => assert_eq!(mode, ShellMode::Inward),
        other => panic!("expected Shell, got {other:?}"),
    }
}

#[test]
fn draft_omits_defaults() {
    let face = serde_json::to_string(&face_ref()).expect("face json");
    let cmd: Command = serde_json::from_str(&format!(
        r#"{{"op":"draft","faces":[{face}],"neutral_plane":{face},"angle_rad":0.2}}"#
    ))
    .expect("deserialize minimal draft");
    match cmd {
        Command::Draft { direction, .. } => assert_eq!(direction, DraftDirection::Pull),
        other => panic!("expected Draft, got {other:?}"),
    }
}

#[test]
fn helix_dispatch_creates_feature_when_body_exists() {
    let mut session = Session::new();
    session
        .execute(Command::CreateBox {
            dx: 20.0,
            dy: 20.0,
            dz: 20.0,
        })
        .expect("base body");
    let outcome = session.execute(Command::Helix {
        axis: AxisRef::Z,
        radius: 3.0,
        pitch: 2.0,
        height: 10.0,
        turns: 5.0,
        cone_angle: 0.0,
    });
    assert!(outcome.is_ok(), "got: {outcome:?}");
}

#[test]
fn pad_blocks_without_body() {
    let mut session = Session::new();
    assert_invalid_contains(
        session.execute(Command::Pad {
            sketch: sketch(1),
            distance: 5.0,
            direction: PadDirection::default(),
            symmetric: false,
            type_: PadType::default(),
        }),
        "no active body",
    );
}

#[test]
fn pad_blocks_pending_sketch_resolution() {
    let mut session = session_with_box();
    assert_invalid_contains(
        session.execute(Command::Pad {
            sketch: sketch(1),
            distance: 5.0,
            direction: PadDirection::default(),
            symmetric: false,
            type_: PadType::default(),
        }),
        "Track 6",
    );
}

#[test]
fn pocket_blocks_pending_sketch_resolution() {
    let mut session = session_with_box();
    assert_invalid_contains(
        session.execute(Command::Pocket {
            sketch: sketch(1),
            distance: 5.0,
            through_all: false,
            type_: PocketType::default(),
        }),
        "Track 6",
    );
}

#[test]
fn revolve_blocks_pending_sketch_resolution() {
    let mut session = session_with_box();
    assert_invalid_contains(
        session.execute(Command::Revolve {
            sketch: sketch(1),
            axis: AxisRef::Z,
            angle_rad: 1.0,
            symmetric: false,
        }),
        "Track 6",
    );
}

#[test]
fn groove_blocks_pending_sketch_resolution() {
    let mut session = session_with_box();
    assert_invalid_contains(
        session.execute(Command::Groove {
            sketch: sketch(1),
            axis: AxisRef::Z,
            angle_rad: 1.0,
        }),
        "Track 6",
    );
}

#[test]
fn sweep_blocks_pending_sketch_resolution() {
    let mut session = session_with_box();
    assert_invalid_contains(
        session.execute(Command::Sweep {
            profile_sketch: sketch(1),
            path_sketch: sketch(2),
            mode: SweepMode::default(),
        }),
        "Track 6",
    );
}

#[test]
fn loft_blocks_pending_sketch_resolution() {
    let mut session = session_with_box();
    assert_invalid_contains(
        session.execute(Command::Loft {
            profiles: vec![sketch(1), sketch(2)],
            mode: LoftMode::default(),
            ruled: false,
            closed: false,
        }),
        "Track 6",
    );
}

#[test]
fn hole_blocks_pending_face_resolution() {
    let mut session = session_with_box();
    assert_invalid_contains(
        session.execute(Command::Hole {
            face: face_ref(),
            position: [0.0, 0.0],
            radius: 1.0,
            depth: 2.0,
            through_all: false,
            kind: HoleKind::default(),
        }),
        "Track 4",
    );
}

#[test]
fn fillet_blocks_pending_edge_resolution() {
    let mut session = session_with_box();
    assert_invalid_contains(
        session.execute(Command::Fillet {
            edges: vec![edge_ref()],
            radius: 1.0,
            variable: None,
        }),
        "Track 4",
    );
}

#[test]
fn chamfer_blocks_pending_edge_resolution() {
    let mut session = session_with_box();
    assert_invalid_contains(
        session.execute(Command::Chamfer {
            edges: vec![edge_ref()],
            distance: 1.0,
            mode: ChamferMode::default(),
        }),
        "Track 4",
    );
}

#[test]
fn shell_blocks_pending_face_resolution() {
    let mut session = session_with_box();
    assert_invalid_contains(
        session.execute(Command::Shell {
            solid: SolidId(0),
            removed_faces: vec![face_ref()],
            thickness: 1.0,
            mode: ShellMode::default(),
        }),
        "Track 4",
    );
}

#[test]
fn draft_blocks_pending_face_resolution() {
    let mut session = session_with_box();
    assert_invalid_contains(
        session.execute(Command::Draft {
            faces: vec![face_ref()],
            neutral_plane: face_ref(),
            angle_rad: 0.2,
            direction: DraftDirection::default(),
        }),
        "Track 4",
    );
}

#[test]
fn feature_added_outcome_round_trips() {
    let outcome = Outcome::FeatureAdded {
        feature_id: FeatureId(9),
        body: BodyId(0),
        solid: SolidId(2),
    };
    let json = serde_json::to_string(&outcome).expect("serialize outcome");
    let back: Outcome = serde_json::from_str(&json).expect("deserialize outcome");
    assert_eq!(outcome, back);
    assert_eq!(outcome.primary_id(), Some(SolidId(2)));
}

#[test]
fn feature_recomputed_outcome_round_trips() {
    let outcome = Outcome::FeatureRecomputed {
        feature_id: FeatureId(9),
        solid: SolidId(2),
        downstream_invalidated: vec![FeatureId(10)],
    };
    let json = serde_json::to_string(&outcome).expect("serialize outcome");
    let back: Outcome = serde_json::from_str(&json).expect("deserialize outcome");
    assert_eq!(outcome, back);
    assert_eq!(outcome.primary_id(), Some(SolidId(2)));
}

#[test]
fn sketch_created_outcome_round_trips() {
    let outcome = Outcome::SketchCreated {
        sketch_id: SketchId(4),
        plane: Plane {
            origin: [0.0, 0.0, 0.0],
            normal: [0.0, 0.0, 1.0],
        },
    };
    let json = serde_json::to_string(&outcome).expect("serialize outcome");
    let back: Outcome = serde_json::from_str(&json).expect("deserialize outcome");
    assert_eq!(outcome, back);
    assert_eq!(outcome.primary_id(), None);
}

#[test]
fn save_to_json_load_from_json_still_round_trips_existing_commands() {
    let mut session = session_with_box();
    session
        .execute(Command::Translate {
            id: SolidId(0),
            dx: 1.0,
            dy: 2.0,
            dz: 3.0,
        })
        .expect("translate");
    let json = session.save_to_json().expect("save json");
    let loaded = Session::load_from_json(&json).expect("load json");
    assert_eq!(loaded.document().solid_count(), 1);
}

#[test]
fn save_cadk_load_cadk_still_round_trips_existing_commands() {
    let session = session_with_box();
    let bytes = session.save_cadk().expect("save cadk");
    let loaded = Session::load_cadk(&bytes).expect("load cadk");
    assert_eq!(loaded.document().solid_count(), 1);
}

#[test]
fn log_json_replay_still_round_trips_existing_commands() {
    let mut session = session_with_box();
    session
        .execute(Command::Rename {
            id: SolidId(0),
            label: "Renamed".into(),
        })
        .expect("rename");
    let json = session.log_to_json().expect("log json");
    let replayed = Session::replay_from_json(&json).expect("replay json");
    assert_eq!(replayed.document().solid_label(SolidId(0)), Some("Renamed"));
}

#[test]
fn load_from_json_preserves_redo_stack_for_existing_commands() {
    let mut session = session_with_box();
    session
        .execute(Command::CreateSphere { radius: 1.0 })
        .expect("sphere");
    let _undone = session.undo().expect("undo");
    let json = session.save_to_json().expect("save json");
    let mut loaded = Session::load_from_json(&json).expect("load json");
    assert_eq!(loaded.document().solid_count(), 1);
    let _outcome = loaded.redo().expect("redo");
    assert_eq!(loaded.document().solid_count(), 2);
}

#[test]
fn save_cadk_after_transform_still_round_trips_existing_commands() {
    let mut session = session_with_box();
    session
        .execute(Command::Scale {
            id: SolidId(0),
            factor: 2.0,
        })
        .expect("scale");
    let bytes = session.save_cadk().expect("save cadk");
    let loaded = Session::load_cadk(&bytes).expect("load cadk");
    assert_eq!(loaded.document().solid_count(), 1);
}

#[test]
fn command_schemas_includes_new_variants() {
    let names: Vec<&str> = command_schemas().iter().map(|schema| schema.op).collect();
    for op in [
        "pad", "pocket", "revolve", "groove", "hole", "sweep", "loft", "helix", "fillet",
        "chamfer", "shell", "draft",
    ] {
        assert!(names.contains(&op), "missing schema for {op}");
    }
}

fn session_with_box() -> Session {
    let mut session = Session::new();
    session
        .execute(Command::CreateBox {
            dx: 10.0,
            dy: 10.0,
            dz: 10.0,
        })
        .expect("create box");
    session
}
