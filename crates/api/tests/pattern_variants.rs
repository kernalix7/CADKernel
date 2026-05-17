use cadkernel_api::{
    ApiError, AxisRef, Command, FaceRef, FeatureId, InstanceOverride, Outcome, PlaneRef, Session,
    SketchEdit, SketchId, SolidId, TableRow,
};

fn created_id(outcome: Outcome) -> SolidId {
    match outcome {
        Outcome::SolidCreated { id, .. } => id,
        other => panic!("expected SolidCreated, got {other:?}"),
    }
}

fn sketch_id(outcome: Outcome) -> SketchId {
    match outcome {
        Outcome::SketchCreated { sketch_id, .. } => sketch_id,
        other => panic!("expected SketchCreated, got {other:?}"),
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

fn unit_box(session: &mut Session) -> SolidId {
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

fn point_sketch(session: &mut Session, points: &[(f64, f64)]) -> SketchId {
    let sketch = sketch_id(
        session
            .execute(Command::CreateSketch {
                plane: PlaneRef::XY,
                name: "points".to_string(),
            })
            .unwrap(),
    );
    let edits = points
        .iter()
        .map(|(x, y)| SketchEdit::AddPoint { x: *x, y: *y })
        .collect();
    session
        .execute(Command::EditSketch { sketch, edits })
        .unwrap();
    sketch
}

fn empty_sketch(session: &mut Session) -> SketchId {
    sketch_id(
        session
            .execute(Command::CreateSketch {
                plane: PlaneRef::XY,
                name: "empty".to_string(),
            })
            .unwrap(),
    )
}

fn first_face_ref(session: &Session, solid: SolidId) -> FaceRef {
    let (model, _) = session.document().solid_brep(solid).expect("brep");
    let tag = model
        .faces
        .iter()
        .next()
        .and_then(|(_, face)| face.tag.clone())
        .expect("face tag");
    FaceRef { solid, tag }
}

fn assert_invalid_argument(result: Result<Outcome, ApiError>, needle: &str) {
    match result.unwrap_err() {
        ApiError::InvalidArgument(msg) => {
            assert!(msg.contains(needle), "expected {needle:?} in {msg:?}");
        }
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

fn circular_command(features: Vec<FeatureId>) -> Command {
    Command::CircularPattern {
        features,
        axis: AxisRef::Z,
        count: 4,
        angle_rad: std::f64::consts::TAU,
        instance_overrides: Vec::new(),
    }
}

#[test]
fn circular_pattern_single_feature_creates_expected_instances() {
    let mut session = Session::new();
    let source = unit_box(&mut session);

    let outcome = session
        .execute(circular_command(vec![FeatureId(1)]))
        .unwrap();
    let (pattern_id, instance_count, total_features, ids) = pattern_fields(outcome);

    assert_eq!(pattern_id, source);
    assert_eq!(instance_count, 4);
    assert_eq!(total_features, 3);
    assert_eq!(ids[0], source);
    assert_eq!(session.document().solid_count(), 4);
}

#[test]
fn circular_pattern_multiple_features_batches_sources() {
    let mut session = Session::new();
    let first = unit_box(&mut session);
    let second = unit_box(&mut session);

    let outcome = session
        .execute(circular_command(vec![FeatureId(1), FeatureId(2)]))
        .unwrap();
    let (pattern_id, instance_count, total_features, ids) = pattern_fields(outcome);

    assert_eq!(pattern_id, first);
    assert_eq!(instance_count, 8);
    assert_eq!(total_features, 6);
    assert!(ids.contains(&first));
    assert!(ids.contains(&second));
    assert_eq!(session.document().solid_count(), 8);
}

#[test]
fn circular_pattern_empty_features_errors() {
    let mut session = Session::new();
    unit_box(&mut session);

    assert_invalid_argument(session.execute(circular_command(Vec::new())), "requires");
}

#[test]
fn circular_pattern_unknown_feature_errors_with_id() {
    let mut session = Session::new();
    unit_box(&mut session);

    assert_invalid_argument(
        session.execute(circular_command(vec![FeatureId(999)])),
        "FeatureId(999)",
    );
}

#[test]
fn circular_pattern_rejects_count_below_two() {
    let mut session = Session::new();
    unit_box(&mut session);

    assert_invalid_argument(
        session.execute(Command::CircularPattern {
            features: vec![FeatureId(1)],
            axis: AxisRef::Z,
            count: 1,
            angle_rad: std::f64::consts::TAU,
            instance_overrides: Vec::new(),
        }),
        "count",
    );
}

#[test]
fn circular_pattern_rejects_zero_axis() {
    let mut session = Session::new();
    unit_box(&mut session);

    assert_invalid_argument(
        session.execute(Command::CircularPattern {
            features: vec![FeatureId(1)],
            axis: AxisRef::Custom {
                position: [0.0, 0.0, 0.0],
                direction: [0.0, 0.0, 0.0],
            },
            count: 2,
            angle_rad: std::f64::consts::TAU,
            instance_overrides: Vec::new(),
        }),
        "axis",
    );
}

#[test]
fn circular_pattern_rejects_zero_angle() {
    let mut session = Session::new();
    unit_box(&mut session);

    assert_invalid_argument(
        session.execute(Command::CircularPattern {
            features: vec![FeatureId(1)],
            axis: AxisRef::Z,
            count: 2,
            angle_rad: 0.0,
            instance_overrides: Vec::new(),
        }),
        "angle_rad",
    );
}

#[test]
fn circular_pattern_override_suppresses_copy() {
    let mut session = Session::new();
    let source = unit_box(&mut session);

    let outcome = session
        .execute(Command::CircularPattern {
            features: vec![FeatureId(1)],
            axis: AxisRef::Z,
            count: 4,
            angle_rad: std::f64::consts::TAU,
            instance_overrides: vec![InstanceOverride {
                index: 2,
                suppress: true,
                offset_adjust: [0.0, 0.0, 0.0],
            }],
        })
        .unwrap();
    let (pattern_id, instance_count, total_features, ids) = pattern_fields(outcome);

    assert_eq!(pattern_id, source);
    assert_eq!(instance_count, 3);
    assert_eq!(total_features, 2);
    assert_eq!(ids.len(), 3);
}

#[test]
fn circular_pattern_override_offsets_copy() {
    let mut session = Session::new();
    unit_box(&mut session);

    let outcome = session
        .execute(Command::CircularPattern {
            features: vec![FeatureId(1)],
            axis: AxisRef::Z,
            count: 2,
            angle_rad: std::f64::consts::TAU,
            instance_overrides: vec![InstanceOverride {
                index: 1,
                suppress: false,
                offset_adjust: [5.0, 0.0, 0.0],
            }],
        })
        .unwrap();
    let (_, _, _, ids) = pattern_fields(outcome);
    let shifted = session.document().bounding_box(ids[1]).unwrap();

    assert!((shifted.min[0] - 4.0).abs() < 1e-9);
    assert!((shifted.max[0] - 5.0).abs() < 1e-9);
}

#[test]
fn circular_pattern_override_can_suppress_original() {
    let mut session = Session::new();
    let source = unit_box(&mut session);

    let outcome = session
        .execute(Command::CircularPattern {
            features: vec![FeatureId(1)],
            axis: AxisRef::Z,
            count: 3,
            angle_rad: std::f64::consts::TAU,
            instance_overrides: vec![InstanceOverride {
                index: 0,
                suppress: true,
                offset_adjust: [0.0, 0.0, 0.0],
            }],
        })
        .unwrap();
    let (_, instance_count, total_features, ids) = pattern_fields(outcome);

    assert_eq!(instance_count, 2);
    assert_eq!(total_features, 2);
    assert!(!ids.contains(&source));
    assert_eq!(session.document().solid_label(source), None);
}

#[test]
fn sketch_driven_pattern_uses_each_driver_point() {
    let mut session = Session::new();
    let source = unit_box(&mut session);
    let sketch = point_sketch(&mut session, &[(2.0, 0.0), (0.0, 3.0)]);

    let outcome = session
        .execute(Command::SketchDrivenPattern {
            features: vec![FeatureId(1)],
            driver_sketch: sketch,
        })
        .unwrap();
    let (pattern_id, instance_count, total_features, ids) = pattern_fields(outcome);

    assert_eq!(pattern_id, source);
    assert_eq!(instance_count, 3);
    assert_eq!(total_features, 2);
    assert_eq!(ids.len(), 3);
    assert_eq!(session.document().solid_count(), 3);
}

#[test]
fn sketch_driven_pattern_batches_multiple_features() {
    let mut session = Session::new();
    let first = unit_box(&mut session);
    let second = unit_box(&mut session);
    let sketch = point_sketch(&mut session, &[(2.0, 0.0)]);

    let outcome = session
        .execute(Command::SketchDrivenPattern {
            features: vec![FeatureId(1), FeatureId(2)],
            driver_sketch: sketch,
        })
        .unwrap();
    let (pattern_id, instance_count, total_features, ids) = pattern_fields(outcome);

    assert_eq!(pattern_id, first);
    assert_eq!(instance_count, 4);
    assert_eq!(total_features, 2);
    assert!(ids.contains(&second));
}

#[test]
fn sketch_driven_pattern_unknown_sketch_errors() {
    let mut session = Session::new();
    unit_box(&mut session);

    assert_invalid_argument(
        session.execute(Command::SketchDrivenPattern {
            features: vec![FeatureId(1)],
            driver_sketch: SketchId(99),
        }),
        "driver sketch",
    );
}

#[test]
fn sketch_driven_pattern_empty_features_errors() {
    let mut session = Session::new();
    let sketch = point_sketch(&mut session, &[(1.0, 1.0)]);

    assert_invalid_argument(
        session.execute(Command::SketchDrivenPattern {
            features: Vec::new(),
            driver_sketch: sketch,
        }),
        "requires",
    );
}

#[test]
fn sketch_driven_pattern_requires_point_entities() {
    let mut session = Session::new();
    unit_box(&mut session);
    let sketch = empty_sketch(&mut session);

    assert_invalid_argument(
        session.execute(Command::SketchDrivenPattern {
            features: vec![FeatureId(1)],
            driver_sketch: sketch,
        }),
        "no point",
    );
}

#[test]
fn table_driven_pattern_creates_one_copy_per_row() {
    let mut session = Session::new();
    let source = unit_box(&mut session);

    let outcome = session
        .execute(Command::TableDrivenPattern {
            features: vec![FeatureId(1)],
            table: vec![
                TableRow {
                    position: [2.0, 0.0, 0.0],
                    rotation: [0.0, 0.0, 0.0],
                    scale: 1.0,
                },
                TableRow {
                    position: [0.0, 2.0, 0.0],
                    rotation: [0.0, 0.0, 0.0],
                    scale: 2.0,
                },
            ],
        })
        .unwrap();
    let (pattern_id, instance_count, total_features, ids) = pattern_fields(outcome);

    assert_eq!(pattern_id, source);
    assert_eq!(instance_count, 3);
    assert_eq!(total_features, 2);
    assert_eq!(session.document().solid_count(), 3);
    let first_copy = session.document().bounding_box(ids[1]).unwrap();
    assert!((first_copy.min[0] - 2.0).abs() < 1e-9);
    assert!((first_copy.max[0] - 3.0).abs() < 1e-9);
}

#[test]
fn table_driven_pattern_batches_multiple_features() {
    let mut session = Session::new();
    let first = unit_box(&mut session);
    let second = unit_box(&mut session);

    let outcome = session
        .execute(Command::TableDrivenPattern {
            features: vec![FeatureId(1), FeatureId(2)],
            table: vec![TableRow {
                position: [2.0, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale: 1.0,
            }],
        })
        .unwrap();
    let (pattern_id, instance_count, total_features, ids) = pattern_fields(outcome);

    assert_eq!(pattern_id, first);
    assert_eq!(instance_count, 4);
    assert_eq!(total_features, 2);
    assert!(ids.contains(&second));
}

#[test]
fn table_driven_pattern_rejects_empty_table() {
    let mut session = Session::new();
    unit_box(&mut session);

    assert_invalid_argument(
        session.execute(Command::TableDrivenPattern {
            features: vec![FeatureId(1)],
            table: Vec::new(),
        }),
        "table",
    );
}

#[test]
fn table_driven_pattern_rejects_non_positive_scale() {
    let mut session = Session::new();
    unit_box(&mut session);

    assert_invalid_argument(
        session.execute(Command::TableDrivenPattern {
            features: vec![FeatureId(1)],
            table: vec![TableRow {
                position: [0.0, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale: 0.0,
            }],
        }),
        "scale",
    );
}

#[test]
fn table_driven_pattern_unknown_feature_errors() {
    let mut session = Session::new();
    unit_box(&mut session);

    assert_invalid_argument(
        session.execute(Command::TableDrivenPattern {
            features: vec![FeatureId(999)],
            table: vec![TableRow {
                position: [0.0, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale: 1.0,
            }],
        }),
        "FeatureId(999)",
    );
}

#[test]
fn table_driven_pattern_round_trips_json() {
    let command = Command::TableDrivenPattern {
        features: vec![FeatureId(1)],
        table: vec![TableRow {
            position: [1.0, 2.0, 3.0],
            rotation: [0.1, 0.2, 0.3],
            scale: 1.5,
        }],
    };

    let encoded = serde_json::to_value(&command).unwrap();
    let decoded: Command = serde_json::from_value(encoded.clone()).unwrap();

    assert_eq!(serde_json::to_value(decoded).unwrap(), encoded);
}

#[test]
fn fill_pattern_distributes_instances_on_face() {
    let mut session = Session::new();
    let source = unit_box(&mut session);
    let face = first_face_ref(&session, source);

    let outcome = session
        .execute(Command::FillPattern {
            features: vec![FeatureId(1)],
            target_face: face,
            density: 2.0,
        })
        .unwrap();
    let (pattern_id, instance_count, total_features, ids) = pattern_fields(outcome);

    assert_eq!(pattern_id, source);
    assert_eq!(instance_count, 3);
    assert_eq!(total_features, 2);
    assert_eq!(ids.len(), 3);
    assert_eq!(session.document().solid_count(), 3);
}

#[test]
fn fill_pattern_batches_multiple_features() {
    let mut session = Session::new();
    let first = unit_box(&mut session);
    let face = first_face_ref(&session, first);
    let second = unit_box(&mut session);

    let outcome = session
        .execute(Command::FillPattern {
            features: vec![FeatureId(1), FeatureId(2)],
            target_face: face,
            density: 1.0,
        })
        .unwrap();
    let (pattern_id, instance_count, total_features, ids) = pattern_fields(outcome);

    assert_eq!(pattern_id, first);
    assert_eq!(instance_count, 4);
    assert_eq!(total_features, 2);
    assert!(ids.contains(&second));
}

#[test]
fn fill_pattern_rejects_bad_density() {
    let mut session = Session::new();
    let source = unit_box(&mut session);
    let face = first_face_ref(&session, source);

    assert_invalid_argument(
        session.execute(Command::FillPattern {
            features: vec![FeatureId(1)],
            target_face: face,
            density: 0.0,
        }),
        "density",
    );
}

#[test]
fn fill_pattern_unknown_face_errors() {
    let mut session = Session::new();
    unit_box(&mut session);
    let face = FaceRef {
        solid: SolidId(999),
        tag: first_face_ref(&session, SolidId(0)).tag,
    };

    match session
        .execute(Command::FillPattern {
            features: vec![FeatureId(1)],
            target_face: face,
            density: 1.0,
        })
        .unwrap_err()
    {
        ApiError::UnknownSolid(msg) => assert!(msg.contains("solid#999")),
        other => panic!("expected UnknownSolid, got {other:?}"),
    }
}

#[test]
fn fill_pattern_empty_features_errors() {
    let mut session = Session::new();
    let source = unit_box(&mut session);
    let face = first_face_ref(&session, source);

    assert_invalid_argument(
        session.execute(Command::FillPattern {
            features: Vec::new(),
            target_face: face,
            density: 1.0,
        }),
        "requires",
    );
}

#[test]
fn new_pattern_variants_report_stable_op_names() {
    assert_eq!(
        circular_command(vec![FeatureId(1)]).op_name(),
        "circular_pattern"
    );
    assert_eq!(
        Command::SketchDrivenPattern {
            features: vec![FeatureId(1)],
            driver_sketch: SketchId(1),
        }
        .op_name(),
        "sketch_driven_pattern"
    );
    assert_eq!(
        Command::TableDrivenPattern {
            features: vec![FeatureId(1)],
            table: Vec::new(),
        }
        .op_name(),
        "table_driven_pattern"
    );
    assert_eq!(
        Command::FillPattern {
            features: vec![FeatureId(1)],
            target_face: FaceRef::default(),
            density: 1.0,
        }
        .op_name(),
        "fill_pattern"
    );
}
