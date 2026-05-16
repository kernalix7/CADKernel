use cadkernel_api::{
    AxisRef, Command, FeatureId, FeatureSpec, HelixSpec, Outcome, RecomputeCache, Session,
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
    FeatureSpec::Helix(HelixSpec {
        axis: AxisRef::Z,
        radius,
        pitch: 2.0,
        height: 10.0,
        turns,
        cone_angle: 0.0,
    })
}

fn input_hash(spec: &FeatureSpec, parents: &[(u32, u64)]) -> u64 {
    let bytes = serde_json::to_vec(spec).expect("spec bytes");
    RecomputeCache::input_hash(&bytes, parents)
}

fn session_with_two_helixes() -> (Session, cadkernel_api::BodyId, FeatureId, FeatureId) {
    let mut session = Session::new();
    session.execute(helix_command(3.0, 5.0)).expect("helix 1");
    session.execute(helix_command(4.0, 3.0)).expect("helix 2");
    let body_id = session.document().active_body().expect("active body");
    let body = session.document().body(body_id).expect("body");
    let first = FeatureId(body.features[0].feature_id);
    let second = FeatureId(body.features[1].feature_id);
    (session, body_id, first, second)
}

#[test]
fn new_cache_is_empty() {
    let cache = RecomputeCache::new(8);

    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
    assert_eq!(cache.max_entries(), 8);
}

#[test]
fn insert_then_get_returns_bytes() {
    let mut cache = RecomputeCache::new(8);

    cache.insert(0, 10, b"brep".to_vec());

    assert_eq!(cache.get(0, 10), Some(&b"brep"[..]));
    assert_eq!(cache.stats(), (1, 0));
}

#[test]
fn get_miss_returns_none_and_counts_miss() {
    let mut cache = RecomputeCache::new(8);

    assert!(cache.get(0, 10).is_none());
    assert_eq!(cache.stats(), (0, 1));
}

#[test]
fn insert_replaces_existing_entry() {
    let mut cache = RecomputeCache::new(8);

    cache.insert(0, 10, b"old".to_vec());
    cache.insert(0, 10, b"new".to_vec());

    assert_eq!(cache.len(), 1);
    assert_eq!(cache.get(0, 10), Some(&b"new"[..]));
}

#[test]
fn lru_evicts_oldest_entry() {
    let mut cache = RecomputeCache::new(2);

    cache.insert(0, 10, b"a".to_vec());
    cache.insert(1, 20, b"b".to_vec());
    cache.insert(2, 30, b"c".to_vec());

    assert!(cache.get(0, 10).is_none());
    assert_eq!(cache.get(1, 20), Some(&b"b"[..]));
    assert_eq!(cache.get(2, 30), Some(&b"c"[..]));
}

#[test]
fn get_refreshes_lru_position() {
    let mut cache = RecomputeCache::new(2);

    cache.insert(0, 10, b"a".to_vec());
    cache.insert(1, 20, b"b".to_vec());
    assert_eq!(cache.get(0, 10), Some(&b"a"[..]));
    cache.insert(2, 30, b"c".to_vec());

    assert_eq!(cache.get(0, 10), Some(&b"a"[..]));
    assert!(cache.get(1, 20).is_none());
    assert_eq!(cache.get(2, 30), Some(&b"c"[..]));
}

#[test]
fn invalidate_feature_removes_all_hashes_for_feature() {
    let mut cache = RecomputeCache::new(8);

    cache.insert(0, 10, b"a".to_vec());
    cache.insert(0, 11, b"b".to_vec());
    cache.insert(1, 20, b"c".to_vec());
    cache.invalidate_feature(0);

    assert!(cache.get(0, 10).is_none());
    assert!(cache.get(0, 11).is_none());
    assert_eq!(cache.get(1, 20), Some(&b"c"[..]));
}

#[test]
fn max_entries_zero_drops_inserts() {
    let mut cache = RecomputeCache::new(0);

    cache.insert(0, 10, b"a".to_vec());

    assert!(cache.is_empty());
}

#[test]
fn reset_stats_keeps_entries() {
    let mut cache = RecomputeCache::new(8);

    cache.insert(0, 10, b"a".to_vec());
    assert_eq!(cache.get(0, 10), Some(&b"a"[..]));
    cache.reset_stats();

    assert_eq!(cache.stats(), (0, 0));
    assert_eq!(cache.get(0, 10), Some(&b"a"[..]));
}

#[test]
fn input_hash_is_deterministic_for_same_spec_and_parents() {
    let spec = helix_spec(3.0, 5.0);
    let parents = [(0, 100), (1, 200)];

    assert_eq!(input_hash(&spec, &parents), input_hash(&spec, &parents));
}

#[test]
fn input_hash_changes_when_spec_changes() {
    let a = helix_spec(3.0, 5.0);
    let b = helix_spec(4.0, 5.0);

    assert_ne!(input_hash(&a, &[]), input_hash(&b, &[]));
}

#[test]
fn input_hash_changes_when_parent_hash_changes() {
    let spec = helix_spec(3.0, 5.0);

    assert_ne!(
        input_hash(&spec, &[(0, 100)]),
        input_hash(&spec, &[(0, 101)])
    );
}

#[test]
fn input_hash_canonicalizes_parent_order() {
    let spec = helix_spec(3.0, 5.0);

    assert_eq!(
        input_hash(&spec, &[(2, 200), (1, 100)]),
        input_hash(&spec, &[(1, 100), (2, 200)])
    );
}

#[test]
fn recompute_body_populates_cache_on_first_replay() {
    let (mut session, body_id, _, _) = session_with_two_helixes();

    session
        .execute(Command::RecomputeBody { body: body_id })
        .expect("recompute");
    let (hits, misses) = session.document().recompute_cache_stats();

    assert_eq!(session.document().recompute_cache_len(), 2);
    assert_eq!(hits, 0);
    assert!(misses >= 2);
}

#[test]
fn second_recompute_hits_cached_feature_outputs() {
    let (mut session, body_id, _, _) = session_with_two_helixes();

    session
        .execute(Command::RecomputeBody { body: body_id })
        .expect("first recompute");
    let before = session.document().recompute_cache_stats();
    session
        .execute(Command::RecomputeBody { body: body_id })
        .expect("second recompute");
    let after = session.document().recompute_cache_stats();

    assert!(after.0 >= before.0 + 2);
    assert_eq!(after.1, before.1);
}

#[test]
fn editing_downstream_feature_skips_upstream_from_cache() {
    let (mut session, body_id, _first, second) = session_with_two_helixes();
    session
        .execute(Command::RecomputeBody { body: body_id })
        .expect("populate cache");
    let before = session.document().recompute_cache_stats();

    let out = session
        .execute(Command::EditFeature {
            feature: second,
            new_spec: helix_spec(4.5, 3.0),
        })
        .expect("edit downstream");
    let after = session.document().recompute_cache_stats();

    assert!(matches!(out, Outcome::FeatureRecomputed { .. }));
    assert!(after.0 > before.0, "upstream feature should hit cache");
    assert!(after.1 > before.1, "edited downstream feature should miss");
}
