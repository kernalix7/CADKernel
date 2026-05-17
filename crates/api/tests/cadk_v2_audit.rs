use cadkernel_api::cadk::{
    self, BlobKind, BlobRecord, CadkFlags, Manifest, SaveOptions, SchemaVersion,
};
use cadkernel_api::{
    AxisRef, BodyId, ChamferMode, Command, DraftDirection, EdgeRef, ExtrudeKind, FaceRef,
    FeatureId, FeatureSpec, HelixSpec, HoleKind, InstanceOverride, LoftMode, PadDirection, PadSpec,
    PadType, PlaneRef, PocketType, RevolveSpec, ShellMode, SketchEdit, SketchId, SketchRef,
    SolidId, SweepMode, TableRow, Tag,
};
use cadkernel_topology::{EntityKind, OperationId};

const HEADER_SIZE: usize = 64;

fn crc32_ieee(bytes: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, slot) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
        *slot = c;
    }
    let mut c = 0xFFFF_FFFFu32;
    for &b in bytes {
        c = table[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

fn write_header(
    schema_version: u32,
    flags: u32,
    total_size: u64,
    manifest_offset: u64,
    manifest_length: u64,
    manifest_crc32: u32,
) -> [u8; HEADER_SIZE] {
    let mut out = [0u8; HEADER_SIZE];
    out[0..4].copy_from_slice(&schema_version.to_le_bytes());
    out[4..8].copy_from_slice(&flags.to_le_bytes());
    out[8..16].copy_from_slice(&total_size.to_le_bytes());
    out[16..24].copy_from_slice(&manifest_offset.to_le_bytes());
    out[24..32].copy_from_slice(&manifest_length.to_le_bytes());
    out[32..36].copy_from_slice(&manifest_crc32.to_le_bytes());
    out
}

fn encode_entries(
    schema_version: u32,
    flags: u32,
    entries: Vec<(BlobKind, &str, Vec<u8>)>,
) -> Vec<u8> {
    let manifest_offset = (cadk::MAGIC.len() + HEADER_SIZE) as u64;
    let mut content_offset = manifest_offset;
    let manifest_bytes = loop {
        let mut offset = content_offset;
        let records = entries
            .iter()
            .map(|(kind, name, body)| {
                let record = BlobRecord {
                    kind: *kind,
                    name: (*name).to_string(),
                    offset,
                    length: body.len() as u64,
                    crc32: crc32_ieee(body),
                };
                offset += body.len() as u64;
                record
            })
            .collect();
        let bytes = serde_json::to_vec(&Manifest { records }).expect("manifest json");
        let next_content_offset = manifest_offset + bytes.len() as u64;
        if next_content_offset == content_offset {
            break bytes;
        }
        content_offset = next_content_offset;
    };
    let total_size = content_offset + entries.iter().map(|(_, _, b)| b.len() as u64).sum::<u64>();
    let mut out = Vec::with_capacity(total_size as usize);
    out.extend_from_slice(&cadk::MAGIC);
    out.extend_from_slice(&write_header(
        schema_version,
        flags,
        total_size,
        manifest_offset,
        manifest_bytes.len() as u64,
        crc32_ieee(&manifest_bytes),
    ));
    out.extend_from_slice(&manifest_bytes);
    for (_, _, body) in entries {
        out.extend_from_slice(&body);
    }
    out
}

fn legacy_v1_container(
    commands: &[Command],
    compression_level: Option<i32>,
    thumbnail: Option<Vec<u8>>,
) -> Vec<u8> {
    let raw = serde_json::to_vec(commands).expect("legacy command json");
    let (doc, mut flags) = if let Some(level) = compression_level {
        (
            zstd::encode_all(raw.as_slice(), level).expect("zstd encode"),
            CadkFlags::DOCUMENT_COMPRESSED,
        )
    } else {
        (raw, 0)
    };
    let mut entries = vec![(BlobKind::Document, "document", doc)];
    if let Some(thumb) = thumbnail {
        flags |= CadkFlags::HAS_THUMBNAIL;
        entries.push((BlobKind::Thumbnail, "thumbnail", thumb));
    }
    encode_entries(1, flags, entries)
}

fn v2_document_container(document: serde_json::Value) -> Vec<u8> {
    encode_entries(
        2,
        0,
        vec![(
            BlobKind::Document,
            "document",
            serde_json::to_vec(&document).expect("document json"),
        )],
    )
}

fn face_ref() -> FaceRef {
    FaceRef {
        solid: SolidId(0),
        tag: Tag::generated(EntityKind::Face, OperationId(1), 0),
    }
}

fn sketch_ref(id: u64) -> SketchRef {
    SketchRef {
        sketch_id: SketchId(id),
    }
}

fn broad_command_log() -> Vec<Command> {
    vec![
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
            top_radius: 0.25,
        },
        Command::CreateTorus {
            major_radius: 2.0,
            minor_radius: 0.5,
        },
        Command::BooleanUnion {
            lhs: SolidId(0),
            rhs: SolidId(1),
        },
        Command::BooleanSubtract {
            lhs: SolidId(0),
            rhs: SolidId(1),
        },
        Command::BooleanIntersect {
            lhs: SolidId(0),
            rhs: SolidId(1),
        },
        Command::Translate {
            id: SolidId(0),
            dx: 0.0,
            dy: 0.0,
            dz: 0.0,
        },
        Command::Scale {
            id: SolidId(0),
            factor: 1.0,
        },
        Command::ScaleNonUniform {
            id: SolidId(0),
            factors: [1.0, 1.0, 1.0],
            point: [0.0, 0.0, 0.0],
        },
        Command::CenterOnOrigin { id: SolidId(0) },
        Command::AlignTo {
            id: SolidId(0),
            target_id: SolidId(1),
        },
        Command::ScaleToFit {
            id: SolidId(0),
            target_size: 1.0,
        },
        Command::TranslateTo {
            id: SolidId(0),
            point: [0.0, 0.0, 0.0],
        },
        Command::Rename {
            id: SolidId(0),
            label: "audit".into(),
        },
        Command::DeleteSolid { id: SolidId(0) },
        Command::Extrude {
            profile: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]],
            direction: [0.0, 0.0, 1.0],
            distance: 1.0,
            kind: ExtrudeKind::Blind,
        },
        Command::LinearPattern {
            id: SolidId(0),
            direction: [1.0, 0.0, 0.0],
            spacing: 1.0,
            count: 2,
            skip_instances: vec![1],
            features: vec![FeatureId(1)],
            mirror_alternate: true,
            instance_overrides: vec![InstanceOverride {
                index: 1,
                suppress: false,
                offset_adjust: [0.1, 0.0, 0.0],
            }],
        },
        Command::CircularPattern {
            features: vec![FeatureId(1)],
            axis: AxisRef::Z,
            count: 3,
            angle_rad: std::f64::consts::PI,
            instance_overrides: Vec::new(),
        },
        Command::SketchDrivenPattern {
            features: vec![FeatureId(1)],
            driver_sketch: SketchId(1),
        },
        Command::TableDrivenPattern {
            features: vec![FeatureId(1)],
            table: vec![TableRow {
                position: [0.0, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale: 1.0,
            }],
        },
        Command::FillPattern {
            features: vec![FeatureId(1)],
            target_face: face_ref(),
            density: 2.0,
        },
        Command::Mirror {
            id: SolidId(0),
            point: [0.0, 0.0, 0.0],
            normal: [1.0, 0.0, 0.0],
            merge: false,
            features: vec![FeatureId(1)],
        },
        Command::Pad {
            sketch: sketch_ref(0),
            distance: 1.0,
            direction: PadDirection::Normal,
            symmetric: false,
            type_: PadType::Blind,
        },
        Command::Pocket {
            sketch: sketch_ref(0),
            distance: 1.0,
            through_all: false,
            type_: PocketType::Blind,
        },
        Command::Revolve {
            sketch: sketch_ref(0),
            axis: AxisRef::Z,
            angle_rad: 1.0,
            symmetric: false,
        },
        Command::Groove {
            sketch: sketch_ref(0),
            axis: AxisRef::Z,
            angle_rad: 1.0,
        },
        Command::Hole {
            face: face_ref(),
            position: [0.0, 0.0],
            radius: 0.5,
            depth: 1.0,
            through_all: false,
            kind: HoleKind::Simple,
        },
        Command::Sweep {
            profile_sketch: sketch_ref(0),
            path_sketch: sketch_ref(1),
            mode: SweepMode::Standard,
        },
        Command::Loft {
            profiles: vec![sketch_ref(0), sketch_ref(1)],
            mode: LoftMode::Straight,
            ruled: false,
            closed: false,
        },
        Command::Helix {
            axis: AxisRef::Z,
            radius: 1.0,
            pitch: 1.0,
            height: 1.0,
            turns: 1.0,
            cone_angle: 0.0,
        },
        Command::Fillet {
            edges: Vec::<EdgeRef>::new(),
            radius: 0.5,
            variable: None,
        },
        Command::Chamfer {
            edges: Vec::<EdgeRef>::new(),
            distance: 0.5,
            mode: ChamferMode::Equal,
        },
        Command::Shell {
            solid: SolidId(0),
            removed_faces: Vec::new(),
            thickness: 0.5,
            mode: ShellMode::Inward,
        },
        Command::Draft {
            faces: Vec::new(),
            neutral_plane: face_ref(),
            angle_rad: 0.1,
            direction: DraftDirection::Pull,
        },
        Command::CreateSketch {
            plane: PlaneRef::XY,
            name: "Sketch".into(),
        },
        Command::EditSketch {
            sketch: SketchId(1),
            edits: Vec::<SketchEdit>::new(),
        },
        Command::DeleteSketch {
            sketch: SketchId(1),
        },
        Command::MapSketchToFace {
            sketch: SketchId(1),
            face: face_ref(),
        },
        Command::CreateBody {
            name: "Body".into(),
            base_plane: PlaneRef::XY,
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
            new_spec: FeatureSpec::Pad(PadSpec {
                sketch: sketch_ref(0),
                distance: 1.0,
                direction: PadDirection::Normal,
                symmetric: false,
                type_: PadType::Blind,
            }),
        },
        Command::NewDocument,
        Command::Measure { id: SolidId(0) },
        Command::Validate,
        Command::ListSolids,
        Command::FindByLabel {
            query: "box".into(),
        },
        Command::HistoryEvents,
        Command::Stats,
        Command::Bounds { id: SolidId(0) },
        Command::Distance {
            id_a: SolidId(0),
            id_b: SolidId(1),
        },
        Command::Volume { id: SolidId(0) },
        Command::SurfaceArea { id: SolidId(0) },
        Command::Centroid { id: SolidId(0) },
        Command::IntersectsAabb {
            id_a: SolidId(0),
            id_b: SolidId(1),
        },
        Command::Exists { id: SolidId(0) },
        Command::Diagonal { id: SolidId(0) },
        Command::AabbCenter { id: SolidId(0) },
        Command::AabbVolume { id: SolidId(0) },
        Command::ContainsAabb {
            id_outer: SolidId(0),
            id_inner: SolidId(1),
        },
        Command::AabbCorners { id: SolidId(0) },
        Command::SolidLabel { id: SolidId(0) },
        Command::IsEmpty,
        Command::AabbSurfaceArea { id: SolidId(0) },
        Command::SolidCount,
        Command::HistoryCount,
        Command::HasLabel {
            query: "box".into(),
        },
        Command::SolidIds,
        Command::AabbExtents { id: SolidId(0) },
        Command::AabbLongestAxis { id: SolidId(0) },
        Command::AabbShortestAxis { id: SolidId(0) },
        Command::AabbAspectRatio { id: SolidId(0) },
        Command::IsCubic { id: SolidId(0) },
        Command::IsSquareXy { id: SolidId(0) },
        Command::HistoryDescription { index: 0 },
        Command::IsSquareYz { id: SolidId(0) },
        Command::IsSquareXz { id: SolidId(0) },
        Command::OperationCount {
            op_name: "create_box".into(),
        },
        Command::LastOperation,
        Command::HasOperation {
            op_name: "create_box".into(),
        },
        Command::FirstOperation,
        Command::Duplicate { id: SolidId(0) },
        Command::Rotate {
            id: SolidId(0),
            axis: [0.0, 0.0, 1.0],
            angle_rad: 0.0,
            point: [0.0, 0.0, 0.0],
        },
        Command::Noop,
    ]
}

fn fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/cadk-v0/r1_canonical.cadk")
}

#[test]
fn empty_v1_document_migrates_to_empty_v2_sections() {
    let bytes = legacy_v1_container(&[], None, None);
    let migrated = cadk::migrate_to_current(&bytes).expect("migrate");
    let summary = cadk::inspect(&migrated).expect("inspect");
    let data = cadk::decode_document_data(&migrated).expect("decode");

    assert_eq!(summary.schema, SchemaVersion::V2);
    assert!(data.commands.is_empty());
    assert!(data.bodies.is_empty());
    assert!(data.sketches.is_empty());
}

#[test]
fn compressed_v1_document_with_thumbnail_migrates_cleanly() {
    let commands = vec![Command::CreateSphere { radius: 2.0 }];
    let thumbnail = vec![0x5Au8; 257];
    let bytes = legacy_v1_container(&commands, Some(3), Some(thumbnail.clone()));
    let migrated = cadk::migrate_to_current(&bytes).expect("migrate");
    let summary = cadk::inspect(&migrated).expect("inspect");

    assert_eq!(summary.schema, SchemaVersion::V2);
    assert!(summary.document_compressed());
    assert_eq!(cadk::decode(&migrated).expect("decode"), commands);
    assert_eq!(
        cadk::decode_thumbnail(&migrated).expect("thumbnail"),
        Some(thumbnail)
    );
}

#[test]
fn broad_command_log_migrates_without_schema_loss() {
    let commands = broad_command_log();
    assert!(commands.len() >= 22);
    let bytes = legacy_v1_container(&commands, None, None);
    let migrated = cadk::migrate_to_current(&bytes).expect("migrate");
    assert_eq!(cadk::decode(&migrated).expect("decode"), commands);
}

#[test]
fn truncated_v1_fixture_returns_codec_error() {
    let bytes = std::fs::read(fixture_path()).expect("fixture");
    let err = cadk::migrate_to_current(&bytes[..bytes.len() / 2]).unwrap_err();
    assert!(matches!(err, cadkernel_api::ApiError::Codec(_)));
}

#[test]
fn committed_cadk_v0_fixture_migrates_to_v2() {
    let bytes = std::fs::read(fixture_path()).expect("fixture");
    let migrated = cadk::migrate_to_current(&bytes).expect("migrate");
    let summary = cadk::inspect(&migrated).expect("inspect");
    let data = cadk::decode_document_data(&migrated).expect("decode");

    assert_eq!(summary.schema, SchemaVersion::V2);
    assert_eq!(data.commands.len(), 3);
    assert!(data.bodies.is_empty());
    assert!(data.sketches.is_empty());
}

#[test]
fn v2_payload_missing_bodies_defaults_empty() {
    let bytes = v2_document_container(serde_json::json!({
        "commands": [Command::Noop],
        "sketches": []
    }));
    let data = cadk::decode_document_data(&bytes).expect("decode");
    assert_eq!(data.commands, vec![Command::Noop]);
    assert!(data.bodies.is_empty());
}

#[test]
fn v2_payload_missing_sketches_defaults_empty() {
    let bytes = v2_document_container(serde_json::json!({
        "commands": [Command::Noop],
        "bodies": []
    }));
    let data = cadk::decode_document_data(&bytes).expect("decode");
    assert_eq!(data.commands, vec![Command::Noop]);
    assert!(data.sketches.is_empty());
}

#[test]
fn corrupted_v2_document_crc_errors_without_panic() {
    let mut bytes = cadk::encode(&[Command::CreateBox {
        dx: 1.0,
        dy: 2.0,
        dz: 3.0,
    }])
    .expect("encode");
    let summary = cadk::inspect(&bytes).expect("inspect");
    let doc = summary
        .manifest
        .find_first(BlobKind::Document)
        .expect("document");
    bytes[doc.offset as usize] ^= 0xA5;

    let err = cadk::decode_document_data(&bytes).unwrap_err();
    assert!(matches!(err, cadkernel_api::ApiError::Codec(_)));
}

#[test]
fn v2_truncated_half_document_errors_without_panic() {
    let bytes = cadk::encode(&[Command::CreateCylinder {
        radius: 1.0,
        height: 2.0,
    }])
    .expect("encode");
    let summary = cadk::inspect(&bytes).expect("inspect");
    let doc = summary
        .manifest
        .find_first(BlobKind::Document)
        .expect("document");
    let truncated_at = doc.offset as usize + (doc.length as usize / 2);
    let err = cadk::decode_document_data(&bytes[..truncated_at]).unwrap_err();
    assert!(matches!(err, cadkernel_api::ApiError::Codec(_)));
}

#[test]
fn roundtrip_with_100_feature_body_preserves_snapshot_width() {
    let features: Vec<_> = (0..100)
        .map(|i| cadk::CadkBodyFeatureSnapshot {
            feature_id: i,
            name: format!("Feature{i}"),
            kind: "helix".into(),
            spec: None,
            spec_kind: "helix".into(),
            suppressed: false,
            solid: cadk::CadkHandleSnapshot {
                index: i as u32,
                generation: 1,
            },
            cached_solid: None,
        })
        .collect();
    let body = cadk::CadkBodySnapshot {
        id: 1,
        name: "Body".into(),
        features,
        tip: Some(99),
        base_plane_origin: [0.0, 0.0, 0.0],
        base_plane_normal: [0.0, 0.0, 1.0],
        current_solid: Some(99),
    };
    let document = cadk::CadkDocumentData {
        commands: vec![Command::Noop],
        bodies: vec![body],
        sketches: Vec::new(),
    };
    let bytes = encode_entries(
        2,
        0,
        vec![(
            BlobKind::Document,
            "document",
            serde_json::to_vec(&document).expect("document"),
        )],
    );
    let data = cadk::decode_document_data(&bytes).expect("decode");

    assert_eq!(data.commands, vec![Command::Noop]);
    assert_eq!(data.bodies.len(), 1);
    assert_eq!(data.bodies[0].features.len(), 100);
}

#[test]
fn roundtrip_with_50_sketch_records_preserves_sketch_section() {
    let sketches: Vec<_> = (0..50)
        .map(|i| serde_json::json!({ "id": i, "name": format!("Sketch{i}") }))
        .collect();
    let document = cadk::CadkDocumentData {
        commands: (0..50)
            .map(|i| Command::CreateSketch {
                plane: PlaneRef::XY,
                name: format!("Sketch{i}"),
            })
            .collect(),
        bodies: Vec::new(),
        sketches: sketches.clone(),
    };
    let bytes = encode_entries(
        2,
        0,
        vec![(
            BlobKind::Document,
            "document",
            serde_json::to_vec(&document).expect("document"),
        )],
    );
    let data = cadk::decode_document_data(&bytes).expect("decode");

    assert_eq!(data.commands.len(), 50);
    assert_eq!(data.sketches, sketches);
}

#[test]
fn inspect_v3_reports_unknown_schema_and_manifest() {
    let bytes = encode_entries(
        3,
        0,
        vec![(
            BlobKind::Document,
            "document",
            serde_json::to_vec(&serde_json::json!({"future": true})).expect("json"),
        )],
    );
    let summary = cadk::inspect(&bytes).expect("inspect");

    assert_eq!(summary.schema, SchemaVersion::Unknown(3));
    assert_eq!(summary.schema_version, 3);
    assert_eq!(summary.blobs.len(), 1);
    assert_eq!(summary.manifest.records.len(), 1);
}

#[test]
fn decode_v3_rejects_document_log() {
    let bytes = encode_entries(3, 0, vec![(BlobKind::Document, "document", b"{}".to_vec())]);
    let err = cadk::decode_document_data(&bytes).unwrap_err();
    assert!(matches!(err, cadkernel_api::ApiError::Codec(_)));
}

#[test]
fn unknown_schema_without_document_still_exposes_blobs() {
    let bytes = encode_entries(
        3,
        0,
        vec![(BlobKind::Attachment, "future-attachment", vec![1, 2, 3])],
    );
    let summary = cadk::inspect(&bytes).expect("inspect");

    assert_eq!(summary.schema, SchemaVersion::Unknown(3));
    assert_eq!(summary.document_length, 0);
    assert_eq!(summary.blobs[0].kind, BlobKind::Attachment);
}

#[test]
fn unknown_schema_is_not_writable() {
    assert!(SchemaVersion::Unknown(3).writable_u32().is_err());
    assert_eq!(SchemaVersion::current().writable_u32().unwrap(), 2);
}

#[test]
fn save_options_still_write_current_v2_schema() {
    let opts = SaveOptions::default()
        .with_compression(3)
        .with_thumbnail(vec![9, 8, 7]);
    let bytes = cadk::encode_with_options(&[Command::Noop], &opts).expect("encode");
    let summary = cadk::inspect(&bytes).expect("inspect");

    assert_eq!(summary.schema, SchemaVersion::V2);
    assert!(summary.document_compressed());
    assert!(summary.has_thumbnail());
}

#[test]
fn feature_spec_payload_round_trips_through_v2() {
    let cmd = Command::EditFeature {
        feature: FeatureId(7),
        new_spec: FeatureSpec::Helix(HelixSpec {
            axis: AxisRef::Z,
            radius: 1.0,
            pitch: 0.5,
            height: 5.0,
            turns: 10.0,
            cone_angle: 0.0,
        }),
    };
    let bytes = cadk::encode(std::slice::from_ref(&cmd)).expect("encode");
    assert_eq!(cadk::decode(&bytes).expect("decode"), vec![cmd]);
}

#[test]
fn revolve_spec_payload_round_trips_through_v2() {
    let cmd = Command::EditFeature {
        feature: FeatureId(8),
        new_spec: FeatureSpec::Revolve(RevolveSpec {
            sketch: sketch_ref(3),
            axis: AxisRef::Z,
            angle_rad: 1.57,
            symmetric: true,
        }),
    };
    let bytes = cadk::encode(std::slice::from_ref(&cmd)).expect("encode");
    assert_eq!(cadk::decode(&bytes).expect("decode"), vec![cmd]);
}
