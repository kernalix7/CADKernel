use cadkernel_api::{ApiError, BranchId, Command, Session, SolidId};

fn box_cmd() -> Command {
    Command::CreateBox {
        dx: 1.0,
        dy: 1.0,
        dz: 1.0,
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

fn cone_cmd() -> Command {
    Command::CreateCone {
        radius: 0.5,
        height: 2.0,
        top_radius: 0.0,
    }
}

fn torus_cmd() -> Command {
    Command::CreateTorus {
        major_radius: 2.0,
        minor_radius: 0.25,
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
fn diverging_after_undo_preserves_redo_branch() {
    let mut session = session_with_three_primitives();
    session.undo().unwrap();

    session.execute(cone_cmd()).unwrap();

    let branches = session.branches();
    assert_eq!(branches.len(), 1);
    assert_eq!(branches[0].parent_cursor, 2);
    assert!(branches[0].name.starts_with("diverge-"));
    assert!(!session.can_redo());
    assert_eq!(session.document().solid_label(SolidId(2)), Some("Cone"));
}

#[test]
fn branch_metadata_has_stable_id_and_parent_cursor() {
    let mut session = session_with_three_primitives();
    session.undo().unwrap();
    session.execute(cone_cmd()).unwrap();

    let branch = session.branches().remove(0);
    assert_eq!(branch.id, BranchId(1));
    assert_eq!(branch.parent_cursor, 2);
}

#[test]
fn multiple_divergences_coexist_in_creation_order() {
    let mut session = session_with_three_primitives();
    session.undo().unwrap();
    session.execute(cone_cmd()).unwrap();
    session.undo().unwrap();
    session.execute(torus_cmd()).unwrap();

    let branches = session.branches();
    assert_eq!(branches.len(), 2);
    assert_eq!(branches[0].id, BranchId(1));
    assert_eq!(branches[1].id, BranchId(2));
    assert_eq!(branches[0].parent_cursor, 2);
    assert_eq!(branches[1].parent_cursor, 2);
}

#[test]
fn promote_branch_restores_preserved_timeline() {
    let mut session = session_with_three_primitives();
    session.undo().unwrap();
    session.execute(cone_cmd()).unwrap();
    let branch_id = session.branches()[0].id;

    session.promote_branch(branch_id).unwrap();

    assert_eq!(session.cursor(), 3);
    assert_eq!(session.document().solid_label(SolidId(2)), Some("Cylinder"));
    assert!(matches!(session.log()[2], Command::CreateCylinder { .. }));
}

#[test]
fn promote_branch_removes_promoted_branch_and_saves_previous_current_line() {
    let mut session = session_with_three_primitives();
    session.undo().unwrap();
    session.execute(cone_cmd()).unwrap();
    let branch_id = session.branches()[0].id;

    session.promote_branch(branch_id).unwrap();

    let branches = session.branches();
    assert_eq!(branches.len(), 1);
    assert_ne!(branches[0].id, branch_id);
    assert_eq!(branches[0].parent_cursor, 2);
}

#[test]
fn promote_unknown_branch_errors() {
    let mut session = Session::new();
    let err = session.promote_branch(BranchId(42)).unwrap_err();
    assert!(matches!(err, ApiError::InvalidArgument(_)));
}

#[test]
fn redo_stack_is_removed_from_active_line_after_divergence() {
    let mut session = session_with_three_primitives();
    session.undo().unwrap();
    assert!(session.can_redo());

    session.execute(cone_cmd()).unwrap();

    assert!(!session.can_redo());
    assert_eq!(session.full_log().len(), 3);
    assert_eq!(session.branches().len(), 1);
}

#[test]
fn observer_command_does_not_create_branch_or_discard_redo() {
    let mut session = session_with_three_primitives();
    session.undo().unwrap();

    session.execute(Command::SolidCount).unwrap();

    assert!(session.can_redo());
    assert!(session.branches().is_empty());
    assert_eq!(session.cursor(), 2);
    assert_eq!(session.full_log().len(), 3);
}

#[test]
fn executing_at_log_end_does_not_create_empty_branch() {
    let mut session = session_with_three_primitives();

    session.execute(cone_cmd()).unwrap();

    assert!(session.branches().is_empty());
    assert_eq!(session.cursor(), 4);
}

#[test]
fn scrub_replays_prefix_state() {
    let mut session = session_with_three_primitives();

    session.scrub(1).unwrap();

    assert_eq!(session.cursor(), 1);
    assert_eq!(session.full_log().len(), 1);
    assert_eq!(session.document().solid_count(), 1);
    assert_eq!(session.document().solid_label(SolidId(0)), Some("Box"));
}

#[test]
fn scrub_preserves_abandoned_tail_as_branch() {
    let mut session = session_with_three_primitives();

    session.scrub(2).unwrap();

    let branches = session.branches();
    assert_eq!(branches.len(), 1);
    assert_eq!(branches[0].id, BranchId(1));
    assert_eq!(branches[0].parent_cursor, 2);
    assert!(branches[0].name.starts_with("scrub-"));
    assert!(!session.can_redo());
}

#[test]
fn scrub_to_log_end_from_undo_replays_redo_commands() {
    let mut session = session_with_three_primitives();
    session.undo().unwrap();

    session.scrub(3).unwrap();

    assert_eq!(session.cursor(), 3);
    assert_eq!(session.document().solid_count(), 3);
    assert!(session.branches().is_empty());
}

#[test]
fn scrub_rejects_cursor_past_log_end() {
    let mut session = session_with_three_primitives();

    let err = session.scrub(4).unwrap_err();

    assert!(matches!(err, ApiError::InvalidArgument(_)));
    assert_eq!(session.cursor(), 3);
}

#[test]
fn repeated_scrub_at_current_cursor_does_not_duplicate_branch() {
    let mut session = session_with_three_primitives();

    session.scrub(2).unwrap();
    session.scrub(2).unwrap();

    assert_eq!(session.branches().len(), 1);
    assert_eq!(session.cursor(), 2);
}

#[test]
fn promoted_branch_can_be_scrubbed_again() {
    let mut session = session_with_three_primitives();
    session.undo().unwrap();
    session.execute(cone_cmd()).unwrap();
    let old_line = session.branches()[0].id;
    session.promote_branch(old_line).unwrap();

    session.scrub(2).unwrap();

    assert_eq!(session.cursor(), 2);
    assert_eq!(session.document().solid_count(), 2);
    assert!(
        session
            .branches()
            .iter()
            .any(|branch| branch.parent_cursor == 2)
    );
}
