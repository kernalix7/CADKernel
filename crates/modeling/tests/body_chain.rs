use cadkernel_modeling::{Body, FeatureKind};
use cadkernel_topology::{Handle, SolidData};
use serde_json::json;

fn solid(index: u32) -> Handle<SolidData> {
    Handle::from_raw_parts(index, 0)
}

fn body_with_three_features() -> Body {
    let mut body = Body::new_with_plane(7, "Body7", [1.0, 2.0, 3.0], [0.0, 0.0, 1.0]);
    body.add_feature("Pad", FeatureKind::Pad, solid(1));
    body.features[0].feature_id = 11;
    body.add_feature("Pocket", FeatureKind::Pocket, solid(2));
    body.features[1].feature_id = 12;
    body.add_feature("Helix", FeatureKind::Helix, solid(3));
    body.features[2].feature_id = 13;
    body
}

#[test]
fn body_new_with_id_and_plane() {
    let body = Body::new_with_plane(42, "Body42", [1.0, 2.0, 3.0], [0.0, 1.0, 0.0]);
    assert_eq!(body.id, 42);
    assert_eq!(body.name, "Body42");
    assert_eq!(body.base_plane_origin, [1.0, 2.0, 3.0]);
    assert_eq!(body.base_plane_normal, [0.0, 1.0, 0.0]);
    assert_eq!(body.tip, None);
    assert_eq!(body.current_solid, None);
}

#[test]
fn body_new_legacy_constructor_uses_default_plane() {
    let body = Body::new("Legacy");
    assert_eq!(body.id, 0);
    assert_eq!(body.name, "Legacy");
    assert_eq!(body.base_plane_origin, [0.0, 0.0, 0.0]);
    assert_eq!(body.base_plane_normal, [0.0, 0.0, 1.0]);
}

#[test]
fn body_add_feature_with_spec_moves_tip() {
    let mut body = Body::new("Body");
    body.add_feature_with_spec(
        21,
        "Helix",
        FeatureKind::Helix,
        "helix",
        json!({"kind":"helix"}),
    );
    assert_eq!(body.feature_count(), 1);
    assert_eq!(body.tip, Some(0));
    assert_eq!(body.features[0].feature_id, 21);
    assert_eq!(body.features[0].spec_kind, "helix");
    assert!(body.features[0].spec.is_some());
    assert_eq!(body.features[0].cached_solid, None);
}

#[test]
fn body_add_feature_legacy_sets_cached_solid() {
    let mut body = Body::new("Body");
    body.add_feature("Pad", FeatureKind::Pad, solid(4));
    assert_eq!(body.tip, Some(0));
    assert_eq!(body.tip_solid(), Some(solid(4)));
    assert_eq!(body.features[0].cached_solid, Some(solid(4)));
}

#[test]
fn body_suppress_by_id_toggles_in_place() {
    let mut body = body_with_three_features();
    assert!(body.suppress_feature_by_id(12, true));
    assert_eq!(body.features.len(), 3);
    assert!(body.features[1].suppressed);
    assert!(body.suppress_feature_by_id(12, false));
    assert!(!body.features[1].suppressed);
}

#[test]
fn body_suppress_unknown_id_returns_false() {
    let mut body = body_with_three_features();
    assert!(!body.suppress_feature_by_id(99, true));
}

#[test]
fn body_set_tip_by_feature_id() {
    let mut body = body_with_three_features();
    assert!(body.set_tip_by_feature_id(12));
    assert_eq!(body.tip, Some(1));
    assert_eq!(body.tip_solid(), Some(solid(2)));
}

#[test]
fn body_set_tip_unknown_id_returns_false() {
    let mut body = body_with_three_features();
    assert!(!body.set_tip_by_feature_id(99));
    assert_eq!(body.tip, Some(2));
}

#[test]
fn body_move_feature_by_id_reorders_and_tip_lands_on_last() {
    let mut body = body_with_three_features();
    assert!(body.move_feature_by_feature_id(11, 2));
    assert_eq!(body.features[0].feature_id, 12);
    assert_eq!(body.features[1].feature_id, 13);
    assert_eq!(body.features[2].feature_id, 11);
    assert_eq!(body.tip, Some(2));
}

#[test]
fn body_move_feature_out_of_range_returns_false() {
    let mut body = body_with_three_features();
    assert!(!body.move_feature_by_feature_id(11, 3));
}

#[test]
fn body_move_feature_unknown_id_returns_false() {
    let mut body = body_with_three_features();
    assert!(!body.move_feature_by_feature_id(99, 0));
}

#[test]
fn body_active_features_skips_suppressed() {
    let mut body = body_with_three_features();
    assert!(body.suppress_feature_by_id(12, true));
    let ids: Vec<_> = body
        .active_features()
        .map(|feature| feature.feature_id)
        .collect();
    assert_eq!(ids, vec![11, 13]);
}

#[test]
fn body_active_features_honors_tip_prefix() {
    let mut body = body_with_three_features();
    assert!(body.set_tip_by_feature_id(12));
    let ids: Vec<_> = body
        .active_features()
        .map(|feature| feature.feature_id)
        .collect();
    assert_eq!(ids, vec![11, 12]);
}

#[test]
fn empty_body_has_no_active_features() {
    let body = Body::new("Empty");
    assert_eq!(body.active_features().count(), 0);
    assert_eq!(body.tip_solid(), None);
}

#[test]
fn feature_index_of_finds_ids() {
    let body = body_with_three_features();
    assert_eq!(body.feature_index_of(11), Some(0));
    assert_eq!(body.feature_index_of(13), Some(2));
    assert_eq!(body.feature_index_of(99), None);
}

#[test]
fn move_object_to_body_by_feature_id_moves_entry() {
    let mut source = body_with_three_features();
    let mut target = Body::new("Target");
    target
        .move_object_to_body_by_feature_id(&mut source, 12)
        .expect("move by id");
    assert_eq!(source.feature_count(), 2);
    assert_eq!(target.feature_count(), 1);
    assert_eq!(target.features[0].feature_id, 12);
}

#[test]
fn move_object_to_body_by_unknown_id_errors() {
    let mut source = body_with_three_features();
    let mut target = Body::new("Target");
    assert!(
        target
            .move_object_to_body_by_feature_id(&mut source, 99)
            .is_err()
    );
}

#[test]
fn feature_kind_as_str_is_snake_case() {
    assert_eq!(FeatureKind::Pad.as_str(), "pad");
    assert_eq!(FeatureKind::Pocket.as_str(), "pocket");
    assert_eq!(FeatureKind::Revolve.as_str(), "revolve");
    assert_eq!(FeatureKind::Groove.as_str(), "groove");
    assert_eq!(FeatureKind::Hole.as_str(), "hole");
    assert_eq!(FeatureKind::Sweep.as_str(), "sweep");
    assert_eq!(FeatureKind::Loft.as_str(), "loft");
    assert_eq!(FeatureKind::Helix.as_str(), "helix");
    assert_eq!(FeatureKind::Fillet.as_str(), "fillet");
    assert_eq!(FeatureKind::Chamfer.as_str(), "chamfer");
    assert_eq!(FeatureKind::Shell.as_str(), "shell");
    assert_eq!(FeatureKind::Draft.as_str(), "draft");
    assert_eq!(FeatureKind::Mirror.as_str(), "mirror");
    assert_eq!(FeatureKind::Pattern.as_str(), "pattern");
}

#[test]
fn spec_json_is_stored_opaque() {
    let mut body = Body::new("Body");
    let spec = json!({"kind":"pad","distance":5.0});
    body.add_feature_with_spec(31, "Pad", FeatureKind::Pad, "pad", spec.clone());
    assert_eq!(body.features[0].spec.as_ref(), Some(&spec));
}
