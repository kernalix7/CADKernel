//! Tests for `Document::clone_solid_brep` — the A3.2 bridge that lets the
//! viewer pull an owned `BRepModel` out of `Session::document()` after a
//! successful `Session::execute`, so it can hand the model to its existing
//! `add_to_scene(model, handle, ...)` call without holding a borrow on the
//! `Session`.

use cadkernel_api::{Command, Session, SolidId};

#[test]
fn clone_solid_brep_on_empty_document_returns_none() {
    let session = Session::new();
    assert!(session.document().clone_solid_brep(SolidId(0)).is_none());
    assert!(session.document().clone_solid_brep(SolidId(42)).is_none());
}

#[test]
fn clone_solid_brep_on_freshly_created_box_matches_source_vertex_count() {
    let mut session = Session::new();
    let outcome = session
        .execute(Command::CreateBox {
            dx: 10.0,
            dy: 5.0,
            dz: 2.0,
        })
        .unwrap();
    let id = outcome.primary_id().unwrap();

    let (source_model, source_handle) = session.document().solid_brep(id).unwrap();
    let source_vertex_count = source_model.vertices.len();
    assert!(source_vertex_count > 0, "box must have vertices");

    let (cloned_model, cloned_handle) = session.document().clone_solid_brep(id).unwrap();
    assert_eq!(cloned_model.vertices.len(), source_vertex_count);
    assert_eq!(cloned_handle, source_handle);
}

#[test]
fn cloning_then_dropping_clone_does_not_mutate_document() {
    let mut session = Session::new();
    let id = session
        .execute(Command::CreateBox {
            dx: 1.0,
            dy: 1.0,
            dz: 1.0,
        })
        .unwrap()
        .primary_id()
        .unwrap();

    let hash_before = session.document().canonical_hash();
    let vertex_count_before = session.document().solid_brep(id).unwrap().0.vertices.len();

    {
        let (mut cloned_model, _cloned_handle) = session.document().clone_solid_brep(id).unwrap();
        // Mutate the clone — drop every vertex. The source must remain intact.
        let handles: Vec<_> = cloned_model.vertices.iter().map(|(h, _)| h).collect();
        for h in handles {
            cloned_model.vertices.remove(h);
        }
        assert_eq!(cloned_model.vertices.len(), 0);
    }

    let hash_after = session.document().canonical_hash();
    let vertex_count_after = session.document().solid_brep(id).unwrap().0.vertices.len();
    assert_eq!(hash_before, hash_after);
    assert_eq!(vertex_count_before, vertex_count_after);
}

#[test]
fn session_execute_then_clone_solid_brep_round_trips() {
    // This mirrors what the A3.2 viewer GuiAction bridge does:
    //   1. session.execute(Command::CreateBox{...})
    //   2. id = outcome.primary_id()
    //   3. (model, handle) = session.document().clone_solid_brep(id)
    //   4. viewer.add_to_scene(model, handle, ...)
    let mut session = Session::new();
    let outcome = session
        .execute(Command::CreateBox {
            dx: 3.0,
            dy: 4.0,
            dz: 5.0,
        })
        .unwrap();
    let id = outcome
        .primary_id()
        .expect("CreateBox must yield a SolidId");

    let (model, handle) = session
        .document()
        .clone_solid_brep(id)
        .expect("clone_solid_brep must succeed for a just-created box");

    // The cloned handle must resolve inside the cloned model — i.e. this is a
    // self-contained owned pair the viewer can pass to add_to_scene.
    assert!(
        model.solids.get(handle).is_some(),
        "cloned handle must resolve inside the cloned BRepModel"
    );
    assert!(!model.vertices.is_empty(), "box clone must carry vertices");
}
