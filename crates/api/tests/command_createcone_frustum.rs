//! A3.3 task #1 — `Command::CreateCone` frustum extension acceptance tests.
//!
//! Adds `top_radius: f64` with `#[serde(default)]` so that:
//! 1. Legacy serialized commands (no `top_radius` field) deserialize as a
//!    pure cone (`top_radius = 0.0`) — round-trip behavior unchanged.
//! 2. New callers can request a truncated cone (frustum) by setting
//!    `top_radius > 0.0`.
//! 3. The canonical hash distinguishes a pure cone from a frustum with the
//!    same base radius / height — otherwise autosave dedup would alias the
//!    two states and silently corrupt history.

use cadkernel_api::{Command, Outcome, Session, SolidId};

const RADIUS: f64 = 2.0;
const HEIGHT: f64 = 5.0;
const TOP_RADIUS: f64 = 1.0;

/// 1. Legacy JSON without `top_radius` deserializes as a pure cone and
///    produces a single solid whose AABB matches a pre-A3.3 cone.
#[test]
fn legacy_json_without_top_radius_deserializes_as_pure_cone() {
    let legacy = r#"{"op":"create_cone","radius":2.0,"height":5.0}"#;
    let cmd: Command = serde_json::from_str(legacy).expect("legacy JSON must parse");
    assert_eq!(
        cmd,
        Command::CreateCone {
            radius: RADIUS,
            height: HEIGHT,
            top_radius: 0.0,
        },
        "missing top_radius must default to 0.0"
    );

    let mut session = Session::new();
    let outcome = session.execute(cmd).expect("legacy cone must execute");
    let id = match outcome {
        Outcome::SolidCreated { id, .. } => id,
        other => panic!("expected SolidCreated, got {other:?}"),
    };
    let aabb = session
        .document()
        .bounding_box(id)
        .expect("cone must have a bbox");
    let size = aabb.size();
    // Pure cone of radius=2 height=5 should span ~[-2,2] in X and Y, [0,5] in Z.
    assert!((size[0] - 2.0 * RADIUS).abs() < 1e-6, "x extent");
    assert!((size[1] - 2.0 * RADIUS).abs() < 1e-6, "y extent");
    assert!((size[2] - HEIGHT).abs() < 1e-6, "z extent");
}

/// 2. A frustum with `top_radius > 0` builds successfully and is
///    distinguishable from a pure cone (vertex count strictly larger, since
///    the top vertex becomes a full ring + a top face).
#[test]
fn frustum_top_radius_positive_produces_truncated_cone() {
    let mut session = Session::new();
    let outcome = session
        .execute(Command::CreateCone {
            radius: RADIUS,
            height: HEIGHT,
            top_radius: TOP_RADIUS,
        })
        .expect("frustum must execute");
    let id = match outcome {
        Outcome::SolidCreated { id, label } => {
            assert_eq!(label, "Frustum", "frustum label distinguishes from cone");
            id
        }
        other => panic!("expected SolidCreated, got {other:?}"),
    };
    let aabb = session.document().bounding_box(id).expect("frustum bbox");
    let size = aabb.size();
    // Frustum widest at base, so X/Y extents still match base diameter.
    assert!((size[0] - 2.0 * RADIUS).abs() < 1e-6, "x extent");
    assert!((size[1] - 2.0 * RADIUS).abs() < 1e-6, "y extent");
    assert!((size[2] - HEIGHT).abs() < 1e-6, "z extent");

    // Frustum volume = (pi*h/3) * (r1^2 + r1*r2 + r2^2) = pi*5/3*(4+2+1) = 35pi/3.
    let mp = session
        .document()
        .measure_solid(id)
        .expect("frustum measurable");
    let expected_volume = std::f64::consts::PI * HEIGHT / 3.0
        * (RADIUS * RADIUS + RADIUS * TOP_RADIUS + TOP_RADIUS * TOP_RADIUS);
    // Tessellated approximation — 1% tolerance is generous but safe across
    // segment-count tweaks.
    assert!(
        (mp.volume - expected_volume).abs() / expected_volume < 0.01,
        "frustum volume {:.4} vs expected {:.4}",
        mp.volume,
        expected_volume,
    );
}

/// 3. Canonical hash MUST distinguish a pure cone from a frustum that
///    differs only in `top_radius`. Otherwise autosave dedup would alias
///    them and either skip writing a real state change or replay the wrong
///    state on recovery.
#[test]
fn canonical_hash_separates_pure_cone_from_frustum() {
    let pure_log = vec![Command::CreateCone {
        radius: RADIUS,
        height: HEIGHT,
        top_radius: 0.0,
    }];
    let frustum_log = vec![Command::CreateCone {
        radius: RADIUS,
        height: HEIGHT,
        top_radius: TOP_RADIUS,
    }];
    let pure = Session::replay(&pure_log).expect("replay pure cone");
    let frustum = Session::replay(&frustum_log).expect("replay frustum");
    assert_ne!(
        pure.canonical_hash(),
        frustum.canonical_hash(),
        "pure cone and frustum must hash differently — otherwise dedup aliases them",
    );
}

/// 4. Encode → decode → execute round-trip preserves the resulting AABB.
///    Catches drift between the in-memory struct and the JSON contract.
#[test]
fn frustum_encode_decode_execute_roundtrip_matches_aabb() {
    let original = Command::CreateCone {
        radius: RADIUS,
        height: HEIGHT,
        top_radius: TOP_RADIUS,
    };
    let json = serde_json::to_string(&original).expect("encode");
    let decoded: Command = serde_json::from_str(&json).expect("decode");
    assert_eq!(decoded, original);

    let mut a = Session::new();
    let outcome_a = a.execute(original).expect("execute original");
    let id_a = match outcome_a {
        Outcome::SolidCreated { id, .. } => id,
        other => panic!("expected SolidCreated, got {other:?}"),
    };
    let mut b = Session::new();
    let outcome_b = b.execute(decoded).expect("execute decoded");
    let id_b = match outcome_b {
        Outcome::SolidCreated { id, .. } => id,
        other => panic!("expected SolidCreated, got {other:?}"),
    };
    let aabb_a = a.document().bounding_box(id_a).expect("aabb a");
    let aabb_b = b.document().bounding_box(id_b).expect("aabb b");
    for axis in 0..3 {
        assert!((aabb_a.min[axis] - aabb_b.min[axis]).abs() < 1e-12);
        assert!((aabb_a.max[axis] - aabb_b.max[axis]).abs() < 1e-12);
    }
    // Both sessions started empty + applied one command → SolidId(0).
    assert_eq!(id_a, SolidId(0));
    assert_eq!(id_b, SolidId(0));
}

/// 5. Defensive: a frustum with invalid base radius is rejected (not panic).
#[test]
fn frustum_with_zero_base_radius_is_rejected() {
    let mut session = Session::new();
    let err = session
        .execute(Command::CreateCone {
            radius: 0.0,
            height: HEIGHT,
            top_radius: TOP_RADIUS,
        })
        .expect_err("zero base radius must error, not panic");
    let msg = format!("{err}");
    assert!(
        msg.contains("radius") || msg.contains("invalid"),
        "error must mention radius / invalid, got: {msg}"
    );
}
