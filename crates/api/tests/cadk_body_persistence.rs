use cadkernel_api::cadk::{self, migrate_to_current, BlobKind, SchemaVersion};
use cadkernel_api::{
    AxisRef, BodyId, Command, FeatureId, FeatureSpec, HelixSpec, PlaneRef, Session, SolidId,
};

fn helix_command(radius: f64) -> Command {
    Command::Helix {
        axis: AxisRef::Z,
        radius,
        pitch: 2.0,
        height: 10.0,
        turns: 4.0,
        cone_angle: 0.0,
    }
}

fn fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/cadk-v0/r1_canonical.cadk")
}

#[test]
fn schema_version_current_is_v2() {
    assert_eq!(SchemaVersion::current(), SchemaVersion::V2);
}

#[test]
fn fresh_encode_reports_v2_schema() {
    let bytes = cadk::encode(&[Command::CreateBox {
        dx: 1.0,
        dy: 2.0,
        dz: 3.0,
    }])
    .expect("encode");
    let summary = cadk::inspect(&bytes).expect("inspect");
    assert_eq!(summary.schema_version, 2);
}

#[test]
fn v2_manifest_keeps_document_blob_kind() {
    let bytes = cadk::encode(&[Command::CreateSphere { radius: 2.0 }]).expect("encode");
    let summary = cadk::inspect(&bytes).expect("inspect");
    assert_eq!(summary.blobs[0].kind, BlobKind::Document);
    assert_eq!(summary.blobs[0].name, "document");
}

#[test]
fn primitive_log_has_empty_body_and_sketch_sections() {
    let bytes = cadk::encode(&[Command::CreateCylinder {
        radius: 2.0,
        height: 5.0,
    }])
    .expect("encode");
    let data = cadk::decode_document_data(&bytes).expect("decode data");
    assert_eq!(data.commands.len(), 1);
    assert!(data.bodies.is_empty());
    assert!(data.sketches.is_empty());
}

#[test]
fn helix_body_snapshot_is_written() {
    let bytes = cadk::encode(&[helix_command(3.0)]).expect("encode");
    let data = cadk::decode_document_data(&bytes).expect("decode data");
    assert_eq!(data.bodies.len(), 1);
    assert_eq!(data.bodies[0].features.len(), 1);
}

#[test]
fn helix_feature_spec_is_written() {
    let bytes = cadk::encode(&[helix_command(3.0)]).expect("encode");
    let data = cadk::decode_document_data(&bytes).expect("decode data");
    let spec = data.bodies[0].features[0]
        .spec
        .as_ref()
        .expect("feature spec");
    assert_eq!(spec["kind"], "helix");
}

#[test]
fn body_snapshot_preserves_tip_and_current_solid() {
    let bytes = cadk::encode(&[helix_command(3.0), helix_command(4.0)]).expect("encode");
    let data = cadk::decode_document_data(&bytes).expect("decode data");
    let body = &data.bodies[0];
    assert_eq!(body.tip, Some(1));
    assert!(body.current_solid.is_some());
}

#[test]
fn compressed_v2_body_snapshot_round_trips() {
    let opts = cadk::SaveOptions::default().with_compression(3);
    let bytes = cadk::encode_with_options(&[helix_command(3.0)], &opts).expect("encode");
    let data = cadk::decode_document_data(&bytes).expect("decode data");
    assert_eq!(data.commands, vec![helix_command(3.0)]);
    assert_eq!(data.bodies.len(), 1);
}

#[test]
fn thumbnail_v2_body_snapshot_round_trips() {
    let opts = cadk::SaveOptions::default().with_thumbnail(vec![1, 2, 3, 4]);
    let bytes = cadk::encode_with_options(&[helix_command(3.0)], &opts).expect("encode");
    let data = cadk::decode_document_data(&bytes).expect("decode data");
    let thumb = cadk::decode_thumbnail(&bytes).expect("thumbnail");
    assert_eq!(data.bodies.len(), 1);
    assert_eq!(thumb, Some(vec![1, 2, 3, 4]));
}

#[test]
fn session_load_v2_replays_command_log() {
    let mut session = Session::new();
    session.execute(helix_command(3.0)).expect("helix");
    let hash = session.document().canonical_hash();
    let bytes = session.save_cadk().expect("save");
    let loaded = Session::load_cadk(&bytes).expect("load");
    assert_eq!(loaded.document().canonical_hash(), hash);
}

#[test]
fn body_can_recompute_after_v2_load() {
    let mut session = Session::new();
    session.execute(helix_command(3.0)).expect("helix");
    let bytes = session.save_cadk().expect("save");
    let mut loaded = Session::load_cadk(&bytes).expect("load");
    loaded
        .execute(Command::RecomputeBody { body: BodyId(1) })
        .expect("recompute");
    assert_eq!(loaded.document().body_count(), 1);
}

#[test]
fn v1_fixture_decodes_with_empty_sections() {
    let bytes = std::fs::read(fixture_path()).expect("fixture");
    let data = cadk::decode_document_data(&bytes).expect("decode data");
    assert_eq!(data.commands.len(), 3);
    assert!(data.bodies.is_empty());
    assert!(data.sketches.is_empty());
}

#[test]
fn v1_fixture_migrates_to_v2() {
    let bytes = std::fs::read(fixture_path()).expect("fixture");
    let migrated = migrate_to_current(&bytes).expect("migrate");
    let summary = cadk::inspect(&migrated).expect("inspect");
    assert_eq!(summary.schema_version, 2);
}

#[test]
fn migrated_v1_fixture_gets_empty_v2_sections() {
    let bytes = std::fs::read(fixture_path()).expect("fixture");
    let migrated = migrate_to_current(&bytes).expect("migrate");
    let data = cadk::decode_document_data(&migrated).expect("decode data");
    assert_eq!(data.commands.len(), 3);
    assert!(data.bodies.is_empty());
    assert!(data.sketches.is_empty());
}

#[test]
fn invalid_future_feature_log_still_encodes_commands() {
    let cmd = Command::EditFeature {
        feature: FeatureId(99),
        new_spec: FeatureSpec::Helix(HelixSpec {
            axis: AxisRef::Z,
            radius: 1.0,
            pitch: 1.0,
            height: 2.0,
            turns: 2.0,
            cone_angle: 0.0,
        }),
    };
    let bytes = cadk::encode(std::slice::from_ref(&cmd)).expect("encode");
    assert_eq!(cadk::decode(&bytes).expect("decode"), vec![cmd]);
}

#[test]
fn create_body_command_round_trips_through_v2_payload() {
    let cmd = Command::CreateBody {
        name: "BodyA".into(),
        base_plane: PlaneRef::XY,
    };
    let bytes = cadk::encode(std::slice::from_ref(&cmd)).expect("encode");
    assert_eq!(cadk::decode(&bytes).expect("decode"), vec![cmd]);
}

#[test]
fn observer_command_round_trips_through_v2_payload() {
    let cmd = Command::Bounds { id: SolidId(0) };
    let bytes = cadk::encode(std::slice::from_ref(&cmd)).expect("encode");
    assert_eq!(cadk::decode(&bytes).expect("decode"), vec![cmd]);
}
