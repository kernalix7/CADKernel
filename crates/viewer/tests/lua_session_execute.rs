//! Integration tests verifying that the Lua scripting bridge routes every
//! mutating command through `cadkernel_api::Session::execute`.

use cadkernel_api::{Command, Outcome, Session, SolidId};
use cadkernel_viewer::scripting::ScriptEngine;

fn engine_with_session() -> (
    ScriptEngine,
    std::sync::Arc<std::sync::Mutex<cadkernel_api::Session>>,
) {
    let engine = ScriptEngine::new().unwrap();
    let session = engine.session_arc();
    (engine, session)
}

#[test]
fn box_create_logs_command() {
    let (mut engine, session) = engine_with_session();

    let out = engine.execute("return cad.box(10, 20, 30)").unwrap();

    assert_eq!(out, "0");
    let session = session.lock().unwrap();
    assert_eq!(
        session.log(),
        &[Command::CreateBox {
            dx: 10.0,
            dy: 20.0,
            dz: 30.0,
        }]
    );
    assert_eq!(session.document().history().len(), 1);
    assert_eq!(session.document().history()[0].op, "create_box");
}

#[test]
fn boolean_union_via_lua_produces_booleaned_outcome() {
    let (mut engine, session) = engine_with_session();

    let out = engine
        .execute(
            r#"
            cad.box(10, 10, 10)
            cad.sphere(5)
            local c = cad.union(0, 1)
            return c
        "#,
        )
        .unwrap();

    assert_eq!(out, "2");
    {
        let session = session.lock().unwrap();
        assert_eq!(session.log().len(), 3);
        assert_eq!(
            &session.log()[0],
            &Command::CreateBox {
                dx: 10.0,
                dy: 10.0,
                dz: 10.0,
            }
        );
        assert_eq!(&session.log()[1], &Command::CreateSphere { radius: 5.0 });
        assert_eq!(
            &session.log()[2],
            &Command::BooleanUnion {
                lhs: SolidId(0),
                rhs: SolidId(1),
            }
        );
        assert_eq!(session.document().solid_count(), 1);
    }

    let listed = {
        let mut session = session.lock().unwrap();
        session.execute(Command::ListSolids).unwrap()
    };
    assert!(matches!(listed, Outcome::SolidsListed { entries } if entries.len() == 1));
}

#[test]
fn mirror_via_lua_captures_command_mirror() {
    let (mut engine, session) = engine_with_session();

    let out = engine
        .execute(
            r#"
            cad.box(10, 10, 10)
            local m = cad.mirror(0, 0, 0, 0, 1, 0, 0)
            return m
        "#,
        )
        .unwrap();

    assert_eq!(out, "1");
    let session = session.lock().unwrap();
    let last = session.log().last().unwrap();
    match last {
        Command::Mirror {
            id,
            point,
            normal,
            merge,
            features,
        } => {
            assert_eq!(*id, SolidId(0));
            assert_eq!(*point, [0.0, 0.0, 0.0]);
            assert_eq!(*normal, [1.0, 0.0, 0.0]);
            assert!(!merge);
            assert!(features.is_empty());
        }
        other => panic!("expected Mirror command, got {other:?}"),
    }
    assert_eq!(session.document().solid_count(), 2);
}

#[test]
fn translate_is_in_place_not_clone() {
    let (mut engine, session) = engine_with_session();

    let out = engine
        .execute(
            r#"
            cad.box(10, 10, 10)
            local t = cad.translate(0, 5, 5, 5)
            return t
        "#,
        )
        .unwrap();

    assert_eq!(out, "0");
    let session = session.lock().unwrap();
    assert_eq!(session.document().solid_count(), 1);
    assert_eq!(
        session.log(),
        &[
            Command::CreateBox {
                dx: 10.0,
                dy: 10.0,
                dz: 10.0,
            },
            Command::Translate {
                id: SolidId(0),
                dx: 5.0,
                dy: 5.0,
                dz: 5.0,
            },
        ]
    );
}

#[test]
fn measure_via_lua_runs_command_measure() {
    let (mut engine, session) = engine_with_session();

    let out = engine
        .execute(
            r#"
            cad.box(10, 20, 30)
            local m = cad.measure(0)
            return string.format("%.0f", m.volume)
        "#,
        )
        .unwrap();

    assert_eq!(out, "6000");
    let session = session.lock().unwrap();
    assert_eq!(session.log().len(), 1);
    assert_eq!(session.document().history().len(), 1);
}

#[test]
fn error_propagation_negative_dimension() {
    let (mut engine, session) = engine_with_session();

    let err = engine
        .execute("return cad.box(-1, 1, 1)")
        .expect_err("negative dimensions must fail")
        .to_string();

    assert!(!err.is_empty());
    assert!(err.contains("must be > 0"), "unexpected error: {err}");
    let session = session.lock().unwrap();
    assert_eq!(session.log().len(), 0);
    assert_eq!(session.document().solid_count(), 0);
}

#[test]
fn canonical_hash_changes_after_lua_command() {
    let (mut engine, session) = engine_with_session();

    let h0 = session.lock().unwrap().canonical_hash();
    engine.execute("return cad.box(1, 1, 1)").unwrap();
    let h1 = session.lock().unwrap().canonical_hash();

    assert_ne!(h0, h1);
}

#[test]
fn autosave_snapshot_round_trip_after_lua_commands() {
    let (mut engine, session) = engine_with_session();

    engine
        .execute(
            r#"
            cad.box(1, 1, 1)
            cad.sphere(2)
            cad.translate(0, 5, 0, 0)
        "#,
        )
        .unwrap();

    let (h_before, bytes) = {
        let session = session.lock().unwrap();
        (session.canonical_hash(), session.save_cadk().unwrap())
    };
    let session2 = Session::load_cadk(&bytes).unwrap();

    assert_eq!(session2.canonical_hash(), h_before);
}
