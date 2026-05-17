use cadkernel_api::{ApiError, CheckpointId, Command, Session, SolidId};

fn box_cmd() -> Command {
    Command::CreateBox {
        dx: 2.0,
        dy: 2.0,
        dz: 2.0,
    }
}

fn sphere_cmd() -> Command {
    Command::CreateSphere { radius: 1.0 }
}

fn cylinder_cmd() -> Command {
    Command::CreateCylinder {
        radius: 0.5,
        height: 2.0,
    }
}

fn session_with_three_primitives() -> Session {
    let mut session = Session::new();
    session.execute(box_cmd()).unwrap();
    session.execute(sphere_cmd()).unwrap();
    session.execute(cylinder_cmd()).unwrap();
    session
}

#[test]
fn checkpoint_returns_monotonic_ids() {
    let mut session = Session::new();

    let a = session.checkpoint("a".to_string());
    let b = session.checkpoint("b".to_string());

    assert_eq!(a, CheckpointId(1));
    assert_eq!(b, CheckpointId(2));
}

#[test]
fn list_checkpoints_returns_creation_order() {
    let mut session = Session::new();
    let a = session.checkpoint("start".to_string());
    let b = session.checkpoint("next".to_string());

    let list = session.list_checkpoints();

    assert_eq!(list.len(), 2);
    assert_eq!(list[0].0, a);
    assert_eq!(list[0].1, "start");
    assert_eq!(list[1].0, b);
    assert_eq!(list[1].1, "next");
}

#[test]
fn find_checkpoint_returns_id_by_exact_name() {
    let mut session = Session::new();
    let cp = session.checkpoint("before-cut".to_string());

    assert_eq!(session.find_checkpoint("before-cut"), Some(cp));
    assert_eq!(session.find_checkpoint("Before-Cut"), None);
}

#[test]
fn checkpoint_by_name_alias_matches_find_checkpoint() {
    let mut session = Session::new();
    let cp = session.checkpoint("release".to_string());

    assert_eq!(session.checkpoint_by_name("release"), Some(cp));
    assert_eq!(session.checkpoint_by_name("missing"), None);
}

#[test]
fn restore_checkpoint_replays_document_state() {
    let mut session = Session::new();
    session.execute(box_cmd()).unwrap();
    let cp = session.checkpoint("one-solid".to_string());
    session.execute(sphere_cmd()).unwrap();

    session.restore(cp).unwrap();

    assert_eq!(session.cursor(), 1);
    assert_eq!(session.document().solid_count(), 1);
    assert_eq!(session.document().solid_label(SolidId(0)), Some("Box"));
}

#[test]
fn restore_checkpoint_preserves_checkpoint_redo_stack() {
    let mut session = session_with_three_primitives();
    session.undo().unwrap();
    let cp = session.checkpoint("middle".to_string());
    session.redo().unwrap();

    session.restore(cp).unwrap();

    assert_eq!(session.cursor(), 2);
    assert!(session.can_redo());
    session.redo().unwrap();
    assert_eq!(session.cursor(), 3);
    assert_eq!(session.document().solid_count(), 3);
}

#[test]
fn restore_unknown_checkpoint_errors() {
    let mut session = Session::new();

    let err = session.restore(CheckpointId(99)).unwrap_err();

    assert!(matches!(err, ApiError::InvalidArgument(_)));
}

#[test]
fn restored_checkpoint_matches_replayed_prefix_hash() {
    let mut session = session_with_three_primitives();
    let cp = session.checkpoint("full".to_string());
    let expected = Session::replay(session.log()).unwrap().canonical_hash();
    session
        .execute(Command::CreateTorus {
            major_radius: 2.0,
            minor_radius: 0.25,
        })
        .unwrap();

    session.restore(cp).unwrap();

    assert_eq!(session.canonical_hash(), expected);
}

#[test]
fn restore_saves_current_work_as_branch() {
    let mut session = Session::new();
    session.execute(box_cmd()).unwrap();
    let cp = session.checkpoint("base".to_string());
    session.execute(sphere_cmd()).unwrap();
    session.execute(cylinder_cmd()).unwrap();

    session.restore(cp).unwrap();

    let branches = session.branches();
    assert_eq!(branches.len(), 1);
    assert_eq!(branches[0].parent_cursor, 1);
    assert!(branches[0].name.starts_with("restore-"));
}

#[test]
fn checkpoint_can_be_created_after_restore() {
    let mut session = Session::new();
    session.execute(box_cmd()).unwrap();
    let cp = session.checkpoint("base".to_string());
    session.execute(sphere_cmd()).unwrap();
    session.restore(cp).unwrap();

    let restored = session.checkpoint("restored".to_string());

    assert_eq!(restored, CheckpointId(2));
    assert_eq!(session.find_checkpoint("restored"), Some(restored));
}

#[test]
fn duplicate_checkpoint_names_lookup_first_created() {
    let mut session = Session::new();
    let first = session.checkpoint("same".to_string());
    let second = session.checkpoint("same".to_string());

    assert_eq!(session.find_checkpoint("same"), Some(first));
    assert_eq!(session.list_checkpoints()[1].0, second);
}

#[test]
fn list_checkpoints_exposes_recent_creation_instant() {
    let mut session = Session::new();
    session.checkpoint("now".to_string());

    let created_at = session.list_checkpoints()[0].2;

    assert!(created_at.elapsed().as_secs() < 60);
}

#[test]
fn checkpoint_does_not_mutate_log_or_cursor() {
    let mut session = session_with_three_primitives();
    session.undo().unwrap();
    let cursor = session.cursor();
    let full_log_len = session.full_log().len();

    session.checkpoint("middle".to_string());

    assert_eq!(session.cursor(), cursor);
    assert_eq!(session.full_log().len(), full_log_len);
}

#[test]
fn restore_keeps_existing_checkpoints_available() {
    let mut session = Session::new();
    session.execute(box_cmd()).unwrap();
    let first = session.checkpoint("first".to_string());
    session.execute(sphere_cmd()).unwrap();
    let second = session.checkpoint("second".to_string());

    session.restore(first).unwrap();

    assert_eq!(session.find_checkpoint("first"), Some(first));
    assert_eq!(session.find_checkpoint("second"), Some(second));
}

#[test]
fn restore_to_second_checkpoint_after_restoring_first_works() {
    let mut session = Session::new();
    session.execute(box_cmd()).unwrap();
    let first = session.checkpoint("first".to_string());
    session.execute(sphere_cmd()).unwrap();
    let second = session.checkpoint("second".to_string());

    session.restore(first).unwrap();
    session.restore(second).unwrap();

    assert_eq!(session.cursor(), 2);
    assert_eq!(session.document().solid_count(), 2);
}
