use cadkernel_api::{Command, Outcome, Session};
use cadkernel_viewer::test_support::CadApp;

fn first_object_id(app: &CadApp) -> u32 {
    app.scene_ref().objects.first().expect("scene object").id
}

fn app_with_box() -> CadApp {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(4.0, 4.0, 4.0);
    app
}

#[test]
fn empty_selection_fillet_is_graceful() {
    let mut app = CadApp::new_headless();

    app.dispatch_fillet_selected(0.1);

    assert!(app.status_message_for_test().contains("no selected edges"));
    assert_eq!(app.session_history_len_for_test(), 0);
}

#[test]
fn empty_selection_chamfer_is_graceful() {
    let mut app = CadApp::new_headless();

    app.dispatch_chamfer_selected(0.1);

    assert!(app.status_message_for_test().contains("no selected edges"));
    assert_eq!(app.session_history_len_for_test(), 0);
}

#[test]
fn empty_selection_shell_is_graceful() {
    let mut app = CadApp::new_headless();

    app.dispatch_shell_selected(0.1);

    assert!(app.status_message_for_test().contains("no selected faces"));
    assert_eq!(app.session_history_len_for_test(), 0);
}

#[test]
fn empty_selection_draft_is_graceful() {
    let mut app = CadApp::new_headless();

    app.dispatch_draft_selected(0.05);

    assert!(app.status_message_for_test().contains("no selected faces"));
    assert_eq!(app.session_history_len_for_test(), 0);
}

#[test]
fn selected_edge_fillet_dispatches_through_session() {
    let mut app = app_with_box();
    let object_id = first_object_id(&app);
    let before_history = app.session_history_len_for_test();
    assert!(app.select_edge_for_test(object_id, 0));

    app.dispatch_fillet_selected(0.1);

    assert!(app.status_message_for_test().contains("Fillet"));
    assert_eq!(app.selected_entity_count_for_test(), 0);
    assert!(app.session_history_len_for_test() > before_history);
    assert!(app.scene_ref().len() > 1);
}

#[test]
fn selected_edge_chamfer_dispatches_through_session() {
    let mut app = app_with_box();
    let object_id = first_object_id(&app);
    let before_history = app.session_history_len_for_test();
    assert!(app.select_edge_for_test(object_id, 1));

    app.dispatch_chamfer_selected(0.1);

    assert!(app.status_message_for_test().contains("Chamfer"));
    assert_eq!(app.selected_entity_count_for_test(), 0);
    assert!(app.session_history_len_for_test() > before_history);
    assert!(app.scene_ref().len() > 1);
}

#[test]
fn selected_face_shell_dispatches_through_session() {
    let mut app = app_with_box();
    let object_id = first_object_id(&app);
    let before_history = app.session_history_len_for_test();
    assert!(app.select_face_for_test(object_id, 0));

    app.dispatch_shell_selected(0.1);

    assert!(app.status_message_for_test().contains("Shell"));
    assert_eq!(app.selected_entity_count_for_test(), 0);
    assert!(app.session_history_len_for_test() > before_history);
    assert!(app.scene_ref().len() > 1);
}

#[test]
fn selected_face_draft_dispatches_through_session() {
    let mut app = app_with_box();
    let object_id = first_object_id(&app);
    let before_history = app.session_history_len_for_test();
    assert!(app.select_face_for_test(object_id, 0));

    app.dispatch_draft_selected(0.05);

    assert!(app.status_message_for_test().contains("Draft"));
    assert_eq!(app.selected_entity_count_for_test(), 0);
    assert!(app.session_history_len_for_test() > before_history);
    assert!(app.scene_ref().len() > 1);
}

#[test]
fn mixed_selection_fillet_uses_only_edges() {
    let mut app = app_with_box();
    let object_id = first_object_id(&app);
    let before_history = app.session_history_len_for_test();
    assert!(app.select_edge_and_face_for_test(object_id, 2, 0));

    app.dispatch_fillet_selected(0.1);

    assert!(app.status_message_for_test().contains("Fillet"));
    assert_eq!(app.selected_entity_count_for_test(), 0);
    assert!(app.session_history_len_for_test() > before_history);
}

#[test]
fn mixed_selection_shell_uses_only_faces() {
    let mut app = app_with_box();
    let object_id = first_object_id(&app);
    let before_history = app.session_history_len_for_test();
    assert!(app.select_edge_and_face_for_test(object_id, 3, 1));

    app.dispatch_shell_selected(0.1);

    assert!(app.status_message_for_test().contains("Shell"));
    assert_eq!(app.selected_entity_count_for_test(), 0);
    assert!(app.session_history_len_for_test() > before_history);
}

#[test]
fn document_resolves_scene_edge_ref_for_session_command() {
    let mut session = Session::new();
    let id = match session
        .execute(Command::CreateBox {
            dx: 3.0,
            dy: 3.0,
            dz: 3.0,
        })
        .expect("box")
    {
        Outcome::SolidCreated { id, .. } => id,
        other => panic!("unexpected outcome: {other:?}"),
    };
    let (model, solid) = session.document().clone_solid_brep(id).expect("solid");
    let mut scene = cadkernel_viewer::scene::Scene::new();
    let object_id = scene.add_object("Box", model, solid, None, Some(id));
    let edge = scene.get(object_id).expect("object").edge_handles[0];
    let refs = scene.selected_edges(object_id, &[edge]);

    let resolved = session.document().resolve_edge_ref(&refs[0]).expect("edge");

    assert_eq!(
        scene
            .get(object_id)
            .expect("object")
            .model
            .name_map
            .get_edge(&refs[0].tag),
        Some(resolved)
    );
}

#[test]
fn document_resolves_scene_face_ref_for_session_command() {
    let mut session = Session::new();
    let id = match session
        .execute(Command::CreateBox {
            dx: 3.0,
            dy: 3.0,
            dz: 3.0,
        })
        .expect("box")
    {
        Outcome::SolidCreated { id, .. } => id,
        other => panic!("unexpected outcome: {other:?}"),
    };
    let (model, solid) = session.document().clone_solid_brep(id).expect("solid");
    let mut scene = cadkernel_viewer::scene::Scene::new();
    let object_id = scene.add_object("Box", model, solid, None, Some(id));
    let face = scene
        .get(object_id)
        .expect("object")
        .model
        .faces
        .iter()
        .next()
        .map(|(handle, _)| handle)
        .expect("face");
    let refs = scene.selected_faces(object_id, &[face]);

    let resolved = session.document().resolve_face_ref(&refs[0]).expect("face");

    assert_eq!(
        scene
            .get(object_id)
            .expect("object")
            .model
            .name_map
            .get_face(&refs[0].tag),
        Some(resolved)
    );
}
