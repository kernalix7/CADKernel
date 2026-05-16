use cadkernel_api::{
    AxisRef, BodyId, Command, EdgeRef, FaceRef, FeatureId, FeatureSpec, HelixSpec, Session, SolidId,
};
use cadkernel_topology::EntityKind;

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
    FeatureSpec::Helix(HelixSpec {
        axis: AxisRef::Z,
        radius,
        pitch: 2.0,
        height: 10.0,
        turns,
        cone_angle: 0.0,
    })
}

fn session_with_helix() -> (Session, BodyId, FeatureId, SolidId) {
    let mut session = Session::new();
    session
        .execute(helix_command(3.0, 5.0))
        .expect("create helix");
    let body_id = session.document().active_body().expect("active body");
    let body = session.document().body(body_id).expect("body");
    let feature_id = FeatureId(body.features[0].feature_id);
    let solid = SolidId(body.current_solid.expect("current solid"));
    (session, body_id, feature_id, solid)
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

fn first_edge_ref(session: &Session, solid: SolidId) -> EdgeRef {
    let (model, _) = session.document().solid_brep(solid).expect("brep");
    let tag = model
        .edges
        .iter()
        .next()
        .and_then(|(_, edge)| edge.tag.clone())
        .expect("edge tag");
    EdgeRef { solid, tag }
}

fn first_vertex_tag(session: &Session, solid: SolidId) -> cadkernel_api::Tag {
    let (model, _) = session.document().solid_brep(solid).expect("brep");
    model
        .vertices
        .iter()
        .next()
        .and_then(|(_, vertex)| vertex.tag.clone())
        .expect("vertex tag")
}

fn current_solid(session: &Session, body_id: BodyId) -> SolidId {
    let body = session.document().body(body_id).expect("body");
    SolidId(body.current_solid.expect("current solid"))
}

#[test]
fn feature_append_registers_persistent_names() {
    let (session, _body_id, _feature_id, solid) = session_with_helix();
    let face = first_face_ref(&session, solid);
    assert!(!session.document().persistent_names().is_empty());
    assert_eq!(
        session
            .document()
            .persistent_names()
            .resolve(&face.tag)
            .map(|k| k.2),
        Some(EntityKind::Face)
    );
}

#[test]
fn edge_ref_resolves_before_recompute() {
    let (session, _body_id, _feature_id, solid) = session_with_helix();
    let edge = first_edge_ref(&session, solid);
    assert!(session.document().resolve_edge_ref(&edge).is_ok());
}

#[test]
fn face_ref_survives_explicit_recompute_body() {
    let (mut session, body_id, _feature_id, solid) = session_with_helix();
    let face = first_face_ref(&session, solid);
    session
        .execute(Command::RecomputeBody { body: body_id })
        .expect("recompute");
    assert!(session.document().resolve_face_ref(&face).is_ok());
}

#[test]
fn edge_ref_survives_feature_edit() {
    let (mut session, _body_id, feature_id, solid) = session_with_helix();
    let edge = first_edge_ref(&session, solid);
    session
        .execute(Command::EditFeature {
            feature: feature_id,
            new_spec: helix_spec(4.0, 4.0),
        })
        .expect("edit feature");
    assert!(session.document().resolve_edge_ref(&edge).is_ok());
}

#[test]
fn vertex_tag_survives_feature_edit_in_name_map() {
    let (mut session, body_id, feature_id, solid) = session_with_helix();
    let tag = first_vertex_tag(&session, solid);
    session
        .execute(Command::EditFeature {
            feature: feature_id,
            new_spec: helix_spec(3.5, 5.0),
        })
        .expect("edit feature");
    let solid = current_solid(&session, body_id);
    let (model, _) = session.document().solid_brep(solid).expect("brep");
    assert!(model.name_map.get_vertex(&tag).is_some());
}

#[test]
fn save_load_json_preserves_persistent_name_entries() {
    let (session, _body_id, _feature_id, _solid) = session_with_helix();
    let before = session.document().persistent_names().entries();
    let json = session.save_to_json().expect("save");
    let loaded = Session::load_from_json(&json).expect("load");
    assert_eq!(loaded.document().persistent_names().entries(), before);
}

#[test]
fn save_load_json_then_recompute_keeps_face_ref_resolvable() {
    let (session, body_id, _feature_id, solid) = session_with_helix();
    let face = first_face_ref(&session, solid);
    let json = session.save_to_json().expect("save");
    let mut loaded = Session::load_from_json(&json).expect("load");
    loaded
        .execute(Command::RecomputeBody { body: body_id })
        .expect("recompute");
    assert!(loaded.document().resolve_face_ref(&face).is_ok());
}

#[test]
fn save_load_cadk_preserves_persistent_name_entries() {
    let (session, _body_id, _feature_id, _solid) = session_with_helix();
    let before = session.document().persistent_names().entries();
    let bytes = session.save_cadk().expect("save cadk");
    let loaded = Session::load_cadk(&bytes).expect("load cadk");
    assert_eq!(loaded.document().persistent_names().entries(), before);
}

#[test]
fn save_load_cadk_after_edit_keeps_edge_ref_resolvable() {
    let (mut session, _body_id, feature_id, solid) = session_with_helix();
    let edge = first_edge_ref(&session, solid);
    session
        .execute(Command::EditFeature {
            feature: feature_id,
            new_spec: helix_spec(4.0, 3.0),
        })
        .expect("edit feature");
    let bytes = session.save_cadk().expect("save cadk");
    let loaded = Session::load_cadk(&bytes).expect("load cadk");
    assert!(loaded.document().resolve_edge_ref(&edge).is_ok());
}

#[test]
fn v0_cadk_fixture_regenerates_persistent_names_on_load() {
    let fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/cadk-v0/r1_canonical.cadk");
    let session = Session::load_cadk_from_path(fixture).expect("load v0 fixture");
    assert_eq!(session.document().solid_count(), 1);
    assert!(!session.document().persistent_names().is_empty());
}

#[test]
fn multi_feature_recompute_keeps_first_feature_face_ref() {
    let (mut session, body_id, first_feature, solid) = session_with_helix();
    assert_eq!(current_solid(&session, body_id), solid);
    session
        .execute(helix_command(4.0, 2.0))
        .expect("second helix");
    let face = first_face_ref(&session, current_solid(&session, body_id));
    session
        .execute(Command::EditFeature {
            feature: first_feature,
            new_spec: helix_spec(3.2, 5.0),
        })
        .expect("edit first feature");
    let solid = current_solid(&session, body_id);
    assert_eq!(face.solid, solid);
    assert!(session.document().resolve_face_ref(&face).is_ok());
}
